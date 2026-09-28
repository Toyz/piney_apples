#!/usr/bin/env python3
"""A model of MODMIDI.IRX (Sony CSL "midi" sequencer, IOP) playing one
Midi block of an SCEI .sq file, as .hack//Infection's SNDBASE.IRX drives it.

Addresses are MODMIDI.IRX module-relative (linked at 0). Everything that the
IOP code computes in u32 arithmetic is done here with explicit & 0xffffffff.

What SNDBASE does (ccSetSq 0x27d4 / ATick 0x3860, read from its code):
    sceMidi_Init(ctx, 4167)          -> g_tick = 4167 << 7 (0x140, stored 0x3bf0)
    sceMidi_Load(ctx, i)             (0x3a8)
    env.outPort[0..15] = 1 << i; env.excOutPort = 1 << i; callbacks all 0
    sceMidi_SelectMidi(ctx, i, 0)    (0x1a5c)
    sceMidi_MidiSetLocation(ctx, i, 0)  (0x1c00; no-op at position 0)
    sceMidi_MidiPlaySwitch(ctx, i, 1)   (0x18d4)
    then every 4167 us: sceMidi_ATick(ctx) (0x560) -> tick(env) (0x1580)

The output of a tick is the bytes MODMIDI appends to the stream buffers
(group 1 of the CSL context: streamBf/streamBf2/streamBf3, u32 buffsize 1032,
u32 validsize, then data) that sceHSyn_ATick reads right after, in the same
timer tick.  Messages are raw MIDI with running status expanded; port bit p
of env.outPort[ch] selects stream buffer p.  Note off is 2 bytes (8n kk):
MODHSYN's parser (0x15cc, data-byte table 0xb5e0) expects exactly that.

Midi block (Midi chunk + offsets[n]):
    u32 data offset (6 on every disc file), u16 ticks per quarter (480),
    and only if data offset >= 12: u16 1, u16 table bytes, table of
    (status, data1) pairs for the Ax compressed form.
Events: [varlen delta] event.  The delta is absent when the previous
channel event had bit 7 set in any of its data bytes (the "no-delta" flag,
masked off on output).  8n has no velocity byte.  Loops are NRPN:
    B0 63 00, B0 06 id            loop start (recorded, still sent)
    B0 63 01, B0 06 id, B0 26 n   loop end: n = 0 forever, else n repeats

Checked byte for byte and state for state against the module itself run in
tools/eemu.py (and again through tools/iopemu.py): all 150 disc files from
play through their first loop jump and 3,000 ticks more, hand-made files
for the paths the disc never takes (finite and nested loops, CC 32, F0 / F7
and text metas, every no-delta form, the An table, tempo changes, the error
stops), and 450 seeks, pauses and resends. crates/piney-audio's sequencer
is checked against this model by its tests/midi.rs (the `fixture` command
writes what it compares).

    tools/midi.py FILE.sq [--ticks N] [--offset N --size N]
    tools/midi.py --disc --bank B [--slot S] [--ticks N] [--port P] [--location TICK]
    tools/midi.py fixture [--ticks N]      per-sequence hashes of every .sq on
                                           the disc played for N ATicks
                                           (crates/piney-audio/tests/midi_fixture.txt)

--disc reads the bank's .sq files from work/infection/disc/DATA/SNDDATA.BIN
through tools/sound.py; slot counts the bank's non-empty .sq files.
"""

import struct
import sys

M32 = 0xFFFFFFFF

ST_LOADED = 0x1      # env.status bits (sceMidiEnv.status, env+0x0c)
ST_PLAY = 0x2        # midi playing (ATick only runs tick() while set)
ST_END = 0x4         # FF 2F reached
ST_NOLOOP = 0x8      # loop ends ignored (never set by MODMIDI or SNDBASE)

STREAM_BUFFSIZE = 1032   # SNDBASE streamBf*: u32 buffsize (header 8 + 1024 data)


class SeqError(Exception):
    pass


class Loop:
    """One of the 8 12-byte loop table entries at system+0x174 (env+0x1c8)."""
    __slots__ = ("id", "count", "rs", "nodelta", "time", "ptr")

    def __init__(self):
        self.id = 0xFF        # +0  loop id (0xff = free)
        self.count = 0xFF     # +1  remaining repeats (0xff = not counting)
        self.rs = 0           # +2  saved running status
        self.nodelta = 0      # +3  saved no-delta flag (always 0 in practice)
        self.time = 0         # +4  saved event time (u32)
        self.ptr = 0          # +8  saved read pointer (just past the CC 6 event)


class Midi:
    """sceMidiEnv plus its system[] area for one sequencer port.

    Field names/offsets (system = env + 0x54):
      env+0x08 position        current song time in ticks (u32)
      env+0x0c status
      env+0x10 outPort[16]     u16 port bitmask per MIDI channel
      env+0x30 excOutPort      u16 port bitmask for F0 sysex
      sys+0x0c ctable          compression table pointer (block+6) or 0
      sys+0x10 data            first event (block + u32 block[0])
      sys+0x14 ptr             read pointer
      sys+0x18 next            absolute time of the next event (u32)
      sys+0x1c acc             sub-tick accumulator, 1/128 us units (u32)
      sys+0x20 ticklen         length of one MIDI tick, 1/128 us (u32)
      sys+0x24 division        ticks per quarter note (u16 from block+4)
      sys+0x28 tempo           us per quarter note (default 500000)
      sys+0x2c reltempo        relative tempo, 256 = 1.0 (default 256)
      sys+0x30 rs              running status (u8)
      sys+0x31 nodelta         "next event has no delta time" flag (u8)
      sys+0x32 mastervol       128
      sys+0x33 nrpn_msb        last CC 99 value (0xff after rewind)
      sys+0x34 dataentry       last CC 6 value / loop id (0xff)
      sys+0x36 chused          u16 mask of channels that were sent a message
      sys+0x38+ch  vol[ch]     raw CC 7 value (127 after rewind)
      sys+0x48+ch  chvol[ch]   per-channel volume scale (128)
      sys+0x58+12*ch chstate   program, CC0,1,2,5,7,10,11,64,65, u16 bend
      sys+0x174 loops[8]
    """

    def __init__(self, sq, port=0, midi_index=0, location=0, tick_us=4167, verbose=False):
        """ccSetSq(port) + sceMidi_MidiSetLocation(location) +
        sceMidi_MidiPlaySwitch(1), as SNDBASE does it (location 0 in
        ccSetSq; the RPC play command and bgmChange seek elsewhere)."""
        self.sq = bytes(sq)
        self.port = port
        self.verbose = verbose
        self.g_tick = (tick_us << 7) & M32          # sceMidi_Init 0x14c: a1 << 7
        self.out = []                               # bytes in the stream buffers
        self.fill = {}                              # port -> validsize (bytes)
        self.stats = None                           # optional hook: dict of counters
        self._load()
        self._select_midi(midi_index)
        if self.set_location(location):
            raise SeqError(f"cannot seek to {location}")
        self._play_switch_on()

    # -- byte access -----------------------------------------------------
    def u8(self, p):
        if p >= len(self.sq):
            raise SeqError(f"read past end of .sq at 0x{p:x}")
        return self.sq[p]

    def u16(self, p):
        return struct.unpack_from("<H", self.sq, p)[0]

    def u32(self, p):
        return struct.unpack_from("<I", self.sq, p)[0]

    # -- setup ------------------------------------------------------------
    def _chunk_ok(self, p, kind):
        # 0x2ac: compares the 8 bytes at p with "SCEI"+kind stored reversed
        return self.sq[p:p + 8] == b"IECS" + kind[::-1]

    def _load(self):
        """sceMidi_Load 0x3a8."""
        self.song_num = 0xFFFFFFFF
        self.midi_num = 0xFFFFFFFF
        self.position = 0
        self.status = 0
        self.mastervol = 128                         # 0x43c sb 0x80, 50(sys)
        self.chvol = [128] * 16                      # 0x428 loop, sys+0x48..
        self.chused = 0                              # 0x424 sh 0, 54(sys)
        self.out_port = [1 << self.port] * 16        # SNDBASE ccSetSq
        self.exc_port = 1 << self.port
        if not self._chunk_ok(0, b"Vers"):
            raise SeqError("no SCEI/Vers")
        sequ = self.u32(8)                           # 0x494: Vers+8 = Sequ offset
        if not self._chunk_ok(sequ, b"Sequ"):
            raise SeqError("no SCEI/Sequ")
        song = self.u32(sequ + 16)
        midi = self.u32(sequ + 20)
        if midi == M32 or not self._chunk_ok(midi, b"Midi"):
            raise SeqError("no SCEI/Midi")
        self.song_chunk = None if song == M32 else song
        self.midi_chunk = midi
        self.status = ST_LOADED

    def _select_midi(self, n):
        """sceMidi_SelectMidi 0x1a5c (after the play/stop checks)."""
        self.status &= ~(ST_PLAY | ST_END) & M32    # 0x1ac8: and ~6
        self.midi_num = M32
        midi = self.midi_chunk
        if self.u32(midi + 12) < n:                  # Midi+0x0c: max index
            raise SeqError(f"midi {n} > max {self.u32(midi + 12)}")
        blk = midi + self.u32(midi + 16 + 4 * n)     # Midi+0x10: offsets[]
        # 0x1b18..0x1b5c: compression table only when u16 block[6] == 1,
        # u16 block[8] != 0 and u32 block[0] >= 12; else 0.
        self.ctable = blk + 6
        if not (self.u16(blk + 6) == 1 and self.u16(blk + 8) != 0 and self.u32(blk) >= 12):
            self.ctable = 0
        self.loops = [Loop() for _ in range(8)]      # 0x1b6c: id/count = 0xff
        self.data = blk + self.u32(blk)              # 0x1bb0: sys+0x10
        self.tempo = 500000                          # 0x1bbc: 0x7a120
        self.reltempo = 256                          # 0x1bc0
        self.division = self.u16(blk + 4)            # 0x1bc8
        self._rewind()                               # 0x19a4
        self.midi_num = n

    def _rewind(self):
        """0x19a4: back to the first event."""
        self.position = 0
        self.next = 0
        self.acc = 0
        self.ticklen = 0
        self.rs = 0
        self.nodelta = 0
        self.dataentry = 0xFF
        self.nrpn_msb = 0xFF
        self.ptr = self.data
        self.vol = [127] * 16                        # 0x19f8 sb 127, 56(sys+ch)
        # chstate: 10 bytes 0xff + u16 0xffff per channel (0x19fc..0x1a24)
        self.chstate = [[0xFF] * 10 + [0xFFFF] for _ in range(16)]
        self._calc_ticklen()                         # 0x958
        self._read_delta()                           # 0x9c8

    def set_location(self, pos):
        """sceMidi_MidiSetLocation 0x1c00: seek.  Events before `pos` are run
        with no output (out = 0, so loop ends are ignored too, but loop starts
        are recorded and channel state is stored); events at `pos` are left
        for the next ATick.  Does not touch acc.  Returns 0 or -1."""
        if pos < self.position:
            self._rewind()
        while self.position < pos:
            if self.status & ST_END:                 # 0x1c68
                return -1
            if self.next >= pos:                     # 0x1c84
                self.position = pos
                break
            self.position = self.next
            if not self._play_env(None):
                break
        return 0 if self.position == pos else -1

    _set_location = set_location

    def play_switch(self, on):
        """sceMidi_MidiPlaySwitch 0x18d4.  on: needs status & 5 == 1 and a
        selected midi; resends the stored channel state (0x1630) *before*
        setting ST_PLAY, then sets it.  off: if playing, all notes off
        (0x14cc) and clear ST_PLAY.  Output lands in the stream buffers and
        is returned by the next atick()."""
        if on:
            if (self.status & (ST_LOADED | ST_END)) != ST_LOADED or self.midi_num == M32:
                return -1
            self._resend_state()
            self.status |= ST_PLAY
        elif self.status & ST_PLAY:
            self._all_notes_off()
            self.status &= ~ST_PLAY & M32
        return 0

    def _play_switch_on(self):
        if self.play_switch(True):
            raise SeqError("cannot play")

    # -- timing -------------------------------------------------------------
    def _calc_ticklen(self):
        """0x958: ticklen = ((tempo << 7) / division << 8) / reltempo, all u32."""
        v = ((self.tempo << 7) & M32) // self.division
        v = ((v << 8) & M32) // self.reltempo
        self.ticklen = v

    def _varlen(self):
        """0x994: MIDI variable-length number (no length limit, u32 wrap)."""
        v = 0
        while True:
            b = self.u8(self.ptr)
            self.ptr += 1
            v = ((v << 7) + (b & 0x7F)) & M32
            if not (b & 0x80):
                return v

    def _read_delta(self):
        """0x9c8: skip the delta if the last event set the no-delta flag."""
        if self.nodelta:
            self.nodelta = 0
            return 0
        d = self._varlen()
        self.next = (self.next + d) & M32
        return d

    # -- one ATick ----------------------------------------------------------
    def atick(self):
        """sceMidi_ATick 0x560 for this env -> tick 0x1580.  Returns the list
        of (port, bytes) in the stream buffers when sceHSyn_ATick reads and
        empties them right after (MODHSYN 0x17b0): what this call appended,
        preceded by anything play_switch() put there since the last call."""
        if self.status & ST_PLAY:
            acc = (self.acc + self.g_tick) & M32     # 0x15b0
            while True:
                if not self._play_env(True):         # 0x15bc
                    self.status &= ~ST_PLAY & M32    # 0x15fc
                    acc = 0
                    self._all_notes_off()            # 0x14cc
                    break
                if acc < self.ticklen:               # 0x15d4 (ticklen re-read)
                    break
                self.position = (self.position + 1) & M32
                acc = (acc - self.ticklen) & M32
            self.acc = acc
        out, self.out, self.fill = self.out, [], {}
        return out

    # -- event interpreter ---------------------------------------------------
    def _play_env(self, out):
        """playEnv 0xf88.  `out` is truthy when output (and loops) are on;
        MidiSetLocation passes 0.  Returns False to stop the sequencer."""
        if self.position < self.next:                # 0xfc0
            return True
        while True:
            p = self.ptr
            b = self.u8(p)
            self.ptr = p + 1
            if b & 0x80:                             # 0xfe0
                status = b
                d1 = self.u8(p + 1)                  # data1 read with status
                self.ptr = p + 2
            else:
                if self.rs == 0:                     # 0x1000
                    self._log(f"playEnv running status error {b:02x} {self.rs:02x}")
                    return False
                status, d1 = self.rs, b
            if self.stats is not None:
                self._stat_event(status, d1, b & 0x80)
            if status < 0xF0:
                ok = self._channel_event(status, d1, out)
                if not ok:
                    return False
            else:
                self.nodelta = 0                     # 0x13e8
                if status == 0xF7:                   # 0xa20
                    self.ptr -= 1
                    n = self._varlen()
                    self.ptr += n
                elif status == 0xF0:                 # 0xc1c (d1 byte skipped!)
                    self._sysex(out)
                elif status == 0xFF:                 # 0xa64
                    if not self._meta(d1):
                        return False
                else:
                    self._log(f"playEnv unknown status {status:02x}")
                    return False
            self._read_delta()                       # 0x1480
            if self.position < self.next:            # 0x1494
                return True

    def _channel_event(self, status, d1, out):
        """0x1040..0x13dc.  Builds msg = status | d1 << 8 | d2 << 16, length n."""
        msg = status | (d1 << 8)
        self.rs = status                             # 0x1058
        ch = status & 0x0F
        hi = status & 0xF0
        d2 = 0                                       # s0
        n = 0                                        # s3
        if hi == 0x80:                               # 0x108c: 2 bytes, no velocity
            n = 2
        elif hi in (0x90, 0xE0):                     # 0x1340
            d2 = self._next_byte()
            msg |= d2 << 16
            n = 3
            if hi == 0xE0:
                self.chstate[ch][10] = (msg >> 8) & 0xFFFF
        elif hi == 0xA0:                             # 0x1108
            if self.ctable:
                idx = (ch | (d1 & 0xF0)) * 2
                n = 3
                if idx < self.u16(self.ctable + 2):
                    lo = self.u8(self.ctable + 4 + idx) | (self.u8(self.ctable + 5 + idx) << 8)
                    msg = lo | ((((d1 & 0x0F) << 3) | 7) << 16)
                else:
                    n = 0                            # 0x1160: nothing sent
                d2 = 0
            else:
                d2 = self._next_byte()
                msg |= d2 << 16
                n = 3
        elif hi == 0xB0:                             # 0x1190
            d2 = self._next_byte()
            msg |= d2 << 16
            n = 3
            if d1 < 100:                             # jump table 0x36a8
                d2 = self._control(ch, d1, d2, out)
                msg = (msg & 0xFFFF) | (d2 << 16)
        elif hi == 0xC0:                             # 0x10e8
            self.chstate[ch][0] = d1 & 0x7F
            n = 2
        elif hi == 0xD0:                             # 0x10fc
            n = 2
        if n:
            self._send_ch(msg, n, out)               # 0x13c8 -> 0xdb4
        self.nodelta = (d1 | d2) & 0x80              # 0x13d0
        return True

    def _next_byte(self):
        b = self.u8(self.ptr)
        self.ptr += 1
        return b

    def _control(self, ch, cc, d2, out):
        """Jump table 0x36a8 for CC 0..99; returns the (possibly rewritten)
        data2 byte.  Every one of these is still sent."""
        v = d2 & 0x7F
        st = self.chstate[ch]
        slot = {0: 1, 1: 2, 2: 3, 5: 4, 10: 6, 11: 7, 64: 8, 65: 9}.get(cc)
        if slot is not None:                         # 0x11d8.. store for resend
            st[slot] = v
        elif cc == 7:                                # 0x1248 volume, scaled
            self.vol[ch] = v
            st[5] = v
            d2 = (d2 & 0x80) | self._scaled_vol(ch)
        elif cc == 32:                               # 0x1304 port select!
            self.out_port[ch] = 1 << (d2 & 0x0F)
        elif cc in (6, 38, 98, 99):                  # 0x1320 -> 0x664
            self._loop_cc(cc, v, out)
        return d2

    def _scaled_vol(self, ch):
        """0xf40: vol * chvol * mastervol >> 14, max 127."""
        v = (self.vol[ch] * self.chvol[ch] * self.mastervol) >> 14
        return 127 if v >= 128 else v

    def _loop_cc(self, cc, v, out):
        """0x664: NRPN loop markers.
           CC 99 = v          -> nrpn_msb = v, dataentry = 0xff
           CC 98              -> ignored
           CC 6  = id         -> dataentry = id; if nrpn_msb == 0: loop start
           CC 38 = count      -> if nrpn_msb == 1: loop end of dataentry"""
        if cc == 99:
            self.nrpn_msb = v
            self.dataentry = 0xFF
        elif cc == 6:
            self.dataentry = v                       # 0x6ec (delay slot)
            if self.nrpn_msb != 0:
                return
            e = self._find_loop(v)                   # 0x630
            if e is None:
                e = self._find_loop(0xFF)            # 0x710: first free
                if e is None:
                    # 0x738 falls out with s0 = table + 96: writes past sys[]
                    raise SeqError("9th loop id: MODMIDI writes past the loop table")
            elif e.count != 0xFF:
                self._log(f"loop start duplext {v}")
            e.id = v
            e.count = 0xFF
            e.rs = self.rs
            e.nodelta = self.nodelta
            e.time = self.next
            e.ptr = self.ptr
            if self.stats is not None:
                self.stats["loop_start"] = self.stats.get("loop_start", 0) + 1
        elif cc == 38:
            if self.nrpn_msb != 1 or self.dataentry == 0xFF:
                return
            if (self.status & ST_NOLOOP) or not out:
                return
            e = self._find_loop(self.dataentry)
            if e is None:
                self._log(f"not found loop start {self.dataentry}")
                return
            if e.count == 0xFF:
                e.count = v
            if v != 0 and e.count == 0:              # 0x844..0x8b4: done
                e.count = 0xFF
                return
            # repeatCallBack (env+0x4c) is 0 in the game: no call
            self.ptr = e.ptr                         # 0x8bc
            self.position = e.time
            self.next = e.time
            self.nodelta = e.nodelta
            self.rs = e.rs
            if v != 0:
                e.count -= 1
            if self.stats is not None:
                k = "loop_jump_inf" if v == 0 else "loop_jump"
                self.stats[k] = self.stats.get(k, 0) + 1

    def _find_loop(self, lid):
        for e in self.loops:
            if e.id == lid:
                return e
        return None

    def _sysex(self, out):
        """0xc1c: F0 <skipped byte> <varlen n> <n bytes>: sends F0 + n bytes."""
        n = self._varlen()
        if out:
            data = bytes([0xF0]) + self.sq[self.ptr:self.ptr + n]
            self._emit_ports(self.exc_port, data)
        self.ptr += n

    def _meta(self, kind):
        """0xa64 (metaMsgCallBack is 0 in the game)."""
        n = self._varlen()
        if kind == 0x2F:                             # 0xbac
            self.status |= ST_END
            return False
        if kind == 0x51:                             # 0xaec
            if n != 3:
                self._log(f"parseMetaEvent tempo length error {n}")
                return False
            p = self.ptr
            self.tempo = (self.u8(p) << 16) + (self.u8(p + 1) << 8) + self.u8(p + 2)
            self.ptr += 3
            self._calc_ticklen()
            if self.stats is not None:
                self.stats.setdefault("tempos", set()).add(self.tempo)
            return True
        self.ptr += n                                # 0xbc0: anything else skipped
        return True

    # -- output -------------------------------------------------------------
    def _send_ch(self, msg, n, out):
        """sendChMsg 0xdb4: mask data bytes, mark channel, copy n bytes to
        every port in outPort[ch]."""
        msg &= 0x7F7FFF
        ch = msg & 0x0F
        self.chused |= 1 << ch                       # table 0x3bb0 = 1 << ch
        if not out:
            return
        data = bytes([msg & 0xFF, (msg >> 8) & 0xFF, (msg >> 16) & 0xFF][:n])
        self._emit_ports(self.out_port[ch], data)

    def _emit_ports(self, mask, data):
        # port p = stream buffer p of the output group (only 3 exist in SNDBASE;
        # 0xbf4 returns 0 for p >= buffNum and the message is skipped)
        p = 0
        while mask:
            if mask & 1:
                if p < 3:
                    used = self.fill.get(p, 0)
                    # 0xe8c: buffsize < validsize + n + 8 -> dropped;
                    # 0xcd8 (sysex): buffsize < validsize + n + 9 with n
                    # excluding the F0, i.e. the same test on len(data)
                    if STREAM_BUFFSIZE < used + len(data) + 8:
                        self._log("Buffer OverRun")
                        if self.stats is not None:
                            self.stats["overrun"] = self.stats.get("overrun", 0) + 1
                    else:
                        self.fill[p] = used + len(data)
                        self.out.append((p, data))
            mask >>= 1
            p += 1

    def _all_notes_off(self):
        """0x14cc: for every channel used since the last call: CC 64 = 0 and
        CC 123 = 0, then clear the mask."""
        for ch in range(16):
            if self.chused & (1 << ch):
                self._send_ch(0x40B0 | ch, 3, True)
                self._send_ch(0x7BB0 | ch, 3, True)
        self.chused = 0

    def _resend_state(self):
        """0x1630 (PlaySwitch 1): per channel 0..15, every stored value whose
        bit 7 is clear, in this order: CC 0, program, CC 1, CC 2, CC 5,
        (CC 7: only vol[ch] is updated; 0x1cf0 sends it only while ST_PLAY,
        which is still clear here), CC 10, CC 11, CC 64, CC 65, pitch bend
        (stored as d1 | d2 << 8 *with* the no-delta bits, resent only when
        (bend & 0x8080) == 0).  After a rewind all are 0xff: nothing."""
        for ch in range(16):
            st = self.chstate[ch]

            def cc(slot, num):
                if not st[slot] & 0x80:
                    self._send_ch(0xB0 | ch | (num << 8) | (st[slot] << 16), 3, True)
            cc(1, 0)                                         # 0x1668
            if not st[0] & 0x80:                             # 0x169c
                self._send_ch(0xC0 | ch | (st[0] << 8), 2, True)
            cc(2, 1)
            cc(3, 2)
            cc(4, 5)
            if not st[5] & 0x80:                             # 0x1770
                self.vol[ch] = st[5]
                if self.status & ST_PLAY:                    # 0x1cf0
                    self._send_ch(0x07B0 | ch | (self._scaled_vol(ch) << 16), 3, True)
            cc(6, 10)
            cc(7, 11)
            cc(8, 64)
            cc(9, 65)
            if not st[10] & 0x8080:                          # 0x186c
                self._send_ch(0xE0 | ch | (st[10] << 8), 3, True)

    def _log(self, s):
        if self.verbose:
            print("modmidi:", s, file=sys.stderr)

    def _stat_event(self, status, d1, explicit):
        s = self.stats
        s.setdefault("status", {})
        k = status & 0xF0 if status < 0xF0 else status
        s["status"][k] = s["status"].get(k, 0) + 1


def play(sq, ticks, port=0, midi_index=0, location=0, verbose=False):
    """Yield (atick_index, port, message bytes) for `ticks` ATick calls; the
    first ATick after MidiPlaySwitch(1) is index 0."""
    m = Midi(sq, port=port, midi_index=midi_index, location=location, verbose=verbose)
    for t in range(ticks):
        for p, data in m.atick():
            yield t, p, data
        if not (m.status & ST_PLAY):
            return


ROOT = __import__("os").path.dirname(__import__("os").path.dirname(__import__("os").path.abspath(__file__)))


def all_sq():
    """(bank, slot, offset, bytes) for every .sq on the disc."""
    import os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import sound
    elf = os.path.join(ROOT, "work", "infection", "disc", "SLUS_202.67")
    prog = sound.Program(elf)
    with open(prog.snddata_path(), "rb") as f:
        data = f.read()
    out = []
    for b in sound.load_banks(prog):
        for slot, (off, n) in enumerate(b.sq_offsets):
            out.append((b, slot, off, data[off:off + n]))
    return out


FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3


def fnv(data, h=FNV_OFFSET):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return h


def fixture(ticks):
    """Lines for crates/piney-audio/tests/midi_fixture.txt: every .sq
    played from its start for `ticks` ATicks on stream buffer 0."""
    print(f"# tools/midi.py fixture: every .sq played {ticks} ATicks from the start, as MODMIDI does.")
    print("# sq BANK SLOT OFFSET MESSAGES BYTES LAST_ATICK STATUS FNV(atick u32, port u8, len u8, bytes)")
    for b, slot, off, sq in all_sq():
        m = Midi(sq)
        h = FNV_OFFSET
        count = size = last = 0
        for t in range(ticks):
            for p, data in m.atick():
                h = fnv(t.to_bytes(4, "little") + bytes((p, len(data))) + data, h)
                count += 1
                size += len(data)
                last = t
            if not (m.status & ST_PLAY):
                break
        print(f"sq {b.index} {slot} {off} {count} {size} {last} {m.status} {h:016x}")
    return 0


def main(argv=None):
    if (argv or sys.argv[1:])[:1] == ["fixture"]:
        rest = (argv or sys.argv[1:])[1:]
        n = int(rest[rest.index("--ticks") + 1]) if "--ticks" in rest else 40000
        return fixture(n)
    import argparse
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("file", nargs="?")
    ap.add_argument("--offset", type=lambda s: int(s, 0), default=0)
    ap.add_argument("--size", type=lambda s: int(s, 0), default=None)
    ap.add_argument("--disc", action="store_true", help="read from SNDDATA.BIN via tools/sound.py")
    ap.add_argument("--bank", type=int)
    ap.add_argument("--slot", type=int, default=0)
    ap.add_argument("--ticks", type=int, default=2400)
    ap.add_argument("--port", type=int, default=0, help="sequencer 0-2 (stream buffer)")
    ap.add_argument("--location", type=int, default=0, help="start tick (MidiSetLocation)")
    ap.add_argument("-v", "--verbose", action="store_true")
    a = ap.parse_args(argv)
    if a.disc:
        sq = next(s for b, slot, off, s in all_sq() if b.index == a.bank and slot == a.slot)
    else:
        raw = open(a.file, "rb").read()
        sq = raw[a.offset:a.offset + a.size] if a.size else raw[a.offset:]
    for t, p, data in play(sq, a.ticks, port=a.port, location=a.location, verbose=a.verbose):
        print(f"{t:7d} p{p} {data.hex(' ')}")


if __name__ == "__main__":
    main()
