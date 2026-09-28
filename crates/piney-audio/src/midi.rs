//! MODMIDI.IRX, Sony's MIDI sequencer, as SNDBASE.IRX drives it: one sequencer
//! per `.sq` of the loaded bank, stepped every 4,167 us, its messages going to
//! a synthesizer port. Addresses are module offsets in MODMIDI.IRX. The Python
//! model this follows (`tools/midi.py`) matched the module run in
//! `tools/eemu.py` byte for byte on every tick of all 150 sequences on the
//! disc. The data (a set bit 7 means no delta next; loops are NRPN 99 / 6 /
//! 38) is in docs/engine/sound.md ("The sequencer").

use piney_data::sound::Sq;

/// `sceMidiEnv.status` bits.
pub const ST_LOADED: u32 = 0x1;
pub const ST_PLAY: u32 = 0x2;
pub const ST_END: u32 = 0x4;
pub const ST_NOLOOP: u32 = 0x8;

/// SNDBASE's stream buffers: u32 buffsize 1032 (8 of header).
const STREAM_BUFFSIZE: usize = 1032;
/// Output buffers there are (`midiGrp[1]`, SNDBASE 0x3564).
const STREAMS: usize = 3;

#[derive(Clone, Copy, Debug)]
struct Loop {
    id: u8,
    count: u8,
    rs: u8,
    nodelta: u8,
    time: u32,
    ptr: usize,
}

const FREE: Loop = Loop { id: 0xff, count: 0xff, rs: 0, nodelta: 0, time: 0, ptr: 0 };

/// Messages for the stream buffers: (buffer, bytes).
pub type Output = Vec<(usize, Vec<u8>)>;

/// One `sceMidiEnv` with its `system[]` state.
#[derive(Clone, Debug)]
pub struct Sequencer {
    sq: Vec<u8>,
    midi_chunk: usize,
    pub status: u32,
    selected: bool,
    pub position: u32,
    next: u32,
    acc: u32,
    ticklen: u32,
    division: u32,
    pub tempo: u32,
    reltempo: u32,
    ctable: usize,
    data: usize,
    ptr: usize,
    rs: u8,
    nodelta: u8,
    mastervol: u32,
    nrpn_msb: u8,
    dataentry: u8,
    chused: u16,
    vol: [u8; 16],
    chvol: [u8; 16],
    /// Per channel: program, CC 0, 1, 2, 5, 7, 10, 11, 64, 65, then bend.
    chstate: [[u16; 11]; 16],
    loops: [Loop; 8],
    out_port: [u16; 16],
    exc_port: u16,
    /// `sceMidi_Init`: the tick in 1/128 us.
    g_tick: u32,
    /// What sits in the stream buffers until the synthesizer reads them.
    out: Output,
    fill: [usize; STREAMS],
}

impl Default for Sequencer {
    fn default() -> Self {
        Sequencer::new()
    }
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer {
            sq: Vec::new(),
            midi_chunk: 0,
            status: 0,
            selected: false,
            position: 0,
            next: 0,
            acc: 0,
            ticklen: 0,
            division: 480,
            tempo: 500_000,
            reltempo: 256,
            ctable: 0,
            data: 0,
            ptr: 0,
            rs: 0,
            nodelta: 0,
            mastervol: 128,
            nrpn_msb: 0xff,
            dataentry: 0xff,
            chused: 0,
            vol: [127; 16],
            chvol: [128; 16],
            chstate: [[0xff; 11]; 16],
            loops: [FREE; 8],
            out_port: [1; 16],
            exc_port: 1,
            g_tick: (crate::TICK_US as u32) << 7,
            out: Vec::new(),
            fill: [0; STREAMS],
        }
    }

    fn u8(&self, p: usize) -> Option<u8> {
        self.sq.get(p).copied()
    }

    fn u16(&self, p: usize) -> u16 {
        self.sq.get(p..p + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&self, p: usize) -> u32 {
        self.sq.get(p..p + 4).map_or(0xffff_ffff, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn chunk_ok(&self, p: usize, kind: &[u8; 4]) -> bool {
        let mut want = *b"IECS\0\0\0\0";
        for i in 0..4 {
            want[4 + i] = kind[3 - i];
        }
        self.sq.get(p..p + 8) == Some(&want[..])
    }

    /// `ccSetSq(addr, i)` (SNDBASE 0x27d4): `sceMidi_Load`, every channel
    /// and sysex routed to stream buffer `buffer`, `sceMidi_SelectMidi(0)`
    /// and `sceMidi_MidiSetLocation(0)`. Not playing.
    pub fn load(&mut self, sq: &Sq, buffer: usize) -> bool {
        *self = Sequencer::new();
        self.sq = sq.data.clone();
        // sceMidi_Load (0x3a8)
        if !self.chunk_ok(0, b"Vers") {
            return false;
        }
        let sequ = self.u32(8) as usize;
        if !self.chunk_ok(sequ, b"Sequ") {
            return false;
        }
        let midi = self.u32(sequ + 20);
        if midi == 0xffff_ffff || !self.chunk_ok(midi as usize, b"Midi") {
            return false;
        }
        self.midi_chunk = midi as usize;
        self.status = ST_LOADED;
        self.out_port = [1 << buffer; 16];
        self.exc_port = 1 << buffer;
        self.select_midi(0) && self.set_location(0)
    }

    /// `sceMidi_SelectMidi` (0x1a5c).
    pub fn select_midi(&mut self, n: u32) -> bool {
        self.status &= !(ST_PLAY | ST_END);
        self.selected = false;
        let m = self.midi_chunk;
        if self.u32(m + 12) < n {
            return false;
        }
        let blk = m + self.u32(m + 16 + 4 * n as usize) as usize;
        self.ctable = blk + 6;
        if !(self.u16(blk + 6) == 1 && self.u16(blk + 8) != 0 && self.u32(blk) >= 12) {
            self.ctable = 0;
        }
        self.loops = [FREE; 8];
        self.data = blk + self.u32(blk) as usize;
        self.tempo = 500_000;
        self.reltempo = 256;
        self.division = self.u16(blk + 4) as u32;
        self.rewind();
        self.selected = true;
        true
    }

    /// 0x19a4: back to the first event.
    fn rewind(&mut self) {
        self.position = 0;
        self.next = 0;
        self.acc = 0;
        self.ticklen = 0;
        self.rs = 0;
        self.nodelta = 0;
        self.dataentry = 0xff;
        self.nrpn_msb = 0xff;
        self.ptr = self.data;
        self.vol = [127; 16];
        self.chstate = [[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xffff]; 16];
        self.calc_ticklen();
        self.read_delta();
    }

    /// `sceMidi_MidiSetLocation` (0x1c00): run events before `pos` without
    /// output (loop starts recorded, channel state stored).
    pub fn set_location(&mut self, pos: u32) -> bool {
        if pos < self.position {
            self.rewind();
        }
        while self.position < pos {
            if self.status & ST_END != 0 {
                return false;
            }
            if self.next >= pos {
                self.position = pos;
                break;
            }
            self.position = self.next;
            if !self.play_env(false) {
                break;
            }
        }
        self.position == pos
    }

    /// `sceMidi_MidiPlaySwitch` (0x18d4).
    pub fn play_switch(&mut self, on: bool) -> bool {
        if on {
            if self.status & (ST_LOADED | ST_END) != ST_LOADED || !self.selected {
                return false;
            }
            self.resend_state();
            self.status |= ST_PLAY;
        } else if self.status & ST_PLAY != 0 {
            self.all_notes_off();
            self.status &= !ST_PLAY;
        }
        true
    }

    /// SNDBASE command 0x110: seek to the start, then play.
    pub fn play(&mut self) {
        if self.set_location(0) {
            self.play_switch(true);
        }
    }

    /// SNDBASE command 0x20.
    pub fn stop(&mut self) {
        self.play_switch(false);
    }

    pub fn playing(&self) -> bool {
        self.status & ST_PLAY != 0
    }

    /// SNDBASE's ATick (0x3860) on a sequence that stopped by itself:
    /// `resetSqData` - select and locate to the start, not playing.
    pub fn reset(&mut self) {
        self.select_midi(0);
        self.set_location(0);
    }

    fn calc_ticklen(&mut self) {
        if self.division == 0 || self.reltempo == 0 {
            self.ticklen = 0;
            return;
        }
        let v = (self.tempo << 7) / self.division;
        self.ticklen = (v << 8) / self.reltempo;
    }

    fn varlen(&mut self) -> Option<u32> {
        let mut v = 0u32;
        loop {
            let b = self.u8(self.ptr)?;
            self.ptr += 1;
            v = (v << 7).wrapping_add((b & 0x7f) as u32);
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
    }

    fn read_delta(&mut self) {
        if self.nodelta != 0 {
            self.nodelta = 0;
            return;
        }
        let d = self.varlen().unwrap_or(0);
        self.next = self.next.wrapping_add(d);
    }

    /// `sceMidi_ATick` for this sequencer (0x560 -> 0x1580). Returns what
    /// is in the stream buffers when the synthesizer reads (and empties)
    /// them right after.
    pub fn tick(&mut self) -> Output {
        if self.status & ST_PLAY != 0 {
            let mut acc = self.acc.wrapping_add(self.g_tick);
            loop {
                if !self.play_env(true) {
                    self.status &= !ST_PLAY;
                    acc = 0;
                    self.all_notes_off();
                    break;
                }
                if acc < self.ticklen {
                    break;
                }
                self.position = self.position.wrapping_add(1);
                acc = acc.wrapping_sub(self.ticklen);
            }
            self.acc = acc;
        }
        self.fill = [0; STREAMS];
        std::mem::take(&mut self.out)
    }

    /// playEnv (0xf88): every event due at `position`. False stops it.
    fn play_env(&mut self, out: bool) -> bool {
        if self.position < self.next {
            return true;
        }
        loop {
            let p = self.ptr;
            let Some(b) = self.u8(p) else { return false };
            self.ptr = p + 1;
            let (status, d1) = if b & 0x80 != 0 {
                let Some(d1) = self.u8(p + 1) else { return false };
                self.ptr = p + 2;
                (b, d1)
            } else {
                if self.rs == 0 {
                    return false;
                }
                (self.rs, b)
            };
            if status < 0xf0 {
                if !self.channel_event(status, d1, out) {
                    return false;
                }
            } else {
                self.nodelta = 0;
                match status {
                    0xf7 => {
                        self.ptr -= 1;
                        let n = self.varlen().unwrap_or(0) as usize;
                        self.ptr += n;
                    }
                    0xf0 => self.sysex(out),
                    0xff => {
                        if !self.meta(d1) {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            self.read_delta();
            if self.position < self.next {
                return true;
            }
        }
    }

    fn next_byte(&mut self) -> u8 {
        let b = self.u8(self.ptr).unwrap_or(0);
        self.ptr += 1;
        b
    }

    /// 0x1040-0x13dc.
    fn channel_event(&mut self, status: u8, d1: u8, out: bool) -> bool {
        let mut msg = status as u32 | (d1 as u32) << 8;
        self.rs = status;
        let ch = (status & 15) as usize;
        let mut d2 = 0u8;
        let mut n = 0;
        match status & 0xf0 {
            0x80 => n = 2,
            0x90 | 0xe0 => {
                d2 = self.next_byte();
                msg |= (d2 as u32) << 16;
                n = 3;
                if status & 0xf0 == 0xe0 {
                    self.chstate[ch][10] = ((msg >> 8) & 0xffff) as u16;
                }
            }
            0xa0 => {
                if self.ctable != 0 {
                    let idx = ((ch as u8 | (d1 & 0xf0)) as usize) * 2;
                    n = 3;
                    if idx < self.u16(self.ctable + 2) as usize {
                        let lo = self.u8(self.ctable + 4 + idx).unwrap_or(0) as u32
                            | (self.u8(self.ctable + 5 + idx).unwrap_or(0) as u32) << 8;
                        msg = lo | ((((d1 & 15) as u32) << 3 | 7) << 16);
                    } else {
                        n = 0;
                    }
                } else {
                    d2 = self.next_byte();
                    msg |= (d2 as u32) << 16;
                    n = 3;
                }
            }
            0xb0 => {
                d2 = self.next_byte();
                msg |= (d2 as u32) << 16;
                n = 3;
                if d1 < 100 {
                    d2 = self.control(ch, d1, d2, out);
                    msg = (msg & 0xffff) | (d2 as u32) << 16;
                }
            }
            0xc0 => {
                self.chstate[ch][0] = (d1 & 0x7f) as u16;
                n = 2;
            }
            0xd0 => n = 2,
            _ => {}
        }
        if n != 0 {
            self.send_ch(msg, n, out);
        }
        self.nodelta = (d1 | d2) & 0x80;
        true
    }

    /// The CC jump table (0x36a8): stored for the resend, CC 7 scaled,
    /// CC 32 re-routes the channel, the NRPN loop markers.
    fn control(&mut self, ch: usize, cc: u8, d2: u8, out: bool) -> u8 {
        let v = d2 & 0x7f;
        let slot = match cc {
            0 => Some(1),
            1 => Some(2),
            2 => Some(3),
            5 => Some(4),
            10 => Some(6),
            11 => Some(7),
            64 => Some(8),
            65 => Some(9),
            _ => None,
        };
        if let Some(s) = slot {
            self.chstate[ch][s] = v as u16;
            return d2;
        }
        match cc {
            7 => {
                self.vol[ch] = v;
                self.chstate[ch][5] = v as u16;
                (d2 & 0x80) | self.scaled_vol(ch)
            }
            32 => {
                self.out_port[ch] = 1 << (d2 & 15);
                d2
            }
            6 | 38 | 98 | 99 => {
                self.loop_cc(cc, v, out);
                d2
            }
            _ => d2,
        }
    }

    fn scaled_vol(&self, ch: usize) -> u8 {
        let v = (self.vol[ch] as u32 * self.chvol[ch] as u32 * self.mastervol) >> 14;
        v.min(127) as u8
    }

    /// 0x664: the NRPN loop markers.
    fn loop_cc(&mut self, cc: u8, v: u8, out: bool) {
        match cc {
            99 => {
                self.nrpn_msb = v;
                self.dataentry = 0xff;
            }
            6 => {
                self.dataentry = v;
                if self.nrpn_msb != 0 {
                    return;
                }
                let k =
                    self.loops.iter().position(|e| e.id == v).or_else(|| self.loops.iter().position(|e| e.id == 0xff));
                // A ninth loop id writes past the table in the module; here
                // it is ignored.
                let Some(k) = k else { return };
                self.loops[k] =
                    Loop { id: v, count: 0xff, rs: self.rs, nodelta: self.nodelta, time: self.next, ptr: self.ptr };
            }
            38 => {
                if self.nrpn_msb != 1 || self.dataentry == 0xff || self.status & ST_NOLOOP != 0 || !out {
                    return;
                }
                let Some(k) = self.loops.iter().position(|e| e.id == self.dataentry) else { return };
                let e = &mut self.loops[k];
                if e.count == 0xff {
                    e.count = v;
                }
                if v != 0 && e.count == 0 {
                    e.count = 0xff;
                    return;
                }
                let e = *e;
                self.ptr = e.ptr;
                self.position = e.time;
                self.next = e.time;
                self.nodelta = e.nodelta;
                self.rs = e.rs;
                if v != 0 {
                    self.loops[k].count -= 1;
                }
            }
            _ => {}
        }
    }

    /// 0xc1c: F0, a skipped byte, varlen n, n bytes: sends F0 and them.
    fn sysex(&mut self, out: bool) {
        let n = self.varlen().unwrap_or(0) as usize;
        if out {
            let mut data = vec![0xf0];
            data.extend_from_slice(self.sq.get(self.ptr..self.ptr + n).unwrap_or(&[]));
            self.emit(self.exc_port, data);
        }
        self.ptr += n;
    }

    /// 0xa64.
    fn meta(&mut self, kind: u8) -> bool {
        let Some(n) = self.varlen() else { return false };
        match kind {
            0x2f => {
                self.status |= ST_END;
                false
            }
            0x51 => {
                if n != 3 {
                    return false;
                }
                let p = self.ptr;
                let b = |k| self.u8(p + k).unwrap_or(0) as u32;
                self.tempo = (b(0) << 16) + (b(1) << 8) + b(2);
                self.ptr += 3;
                self.calc_ticklen();
                true
            }
            _ => {
                self.ptr += n as usize;
                true
            }
        }
    }

    /// sendChMsg (0xdb4).
    fn send_ch(&mut self, msg: u32, n: usize, out: bool) {
        let msg = msg & 0x7f7fff;
        let ch = (msg & 15) as usize;
        self.chused |= 1 << ch;
        if !out {
            return;
        }
        let data = [msg as u8, (msg >> 8) as u8, (msg >> 16) as u8][..n].to_vec();
        self.emit(self.out_port[ch], data);
    }

    fn emit(&mut self, mut mask: u16, data: Vec<u8>) {
        let mut p = 0;
        while mask != 0 {
            if mask & 1 != 0 && p < STREAMS {
                let used = self.fill[p];
                if STREAM_BUFFSIZE >= used + data.len() + 8 {
                    self.fill[p] = used + data.len();
                    self.out.push((p, data.clone()));
                }
            }
            mask >>= 1;
            p += 1;
        }
    }

    /// 0x14cc: CC 64 0 and CC 123 0 on every channel used since the last
    /// time, in channel order.
    fn all_notes_off(&mut self) {
        for ch in 0..16u32 {
            if self.chused & (1 << ch) != 0 {
                self.send_ch(0x40b0 | ch, 3, true);
                self.send_ch(0x7bb0 | ch, 3, true);
            }
        }
        self.chused = 0;
    }

    /// 0x1630: the stored channel state, before ST_PLAY is set (so CC 7
    /// is not resent), and a bend stored with a no-delta bit is skipped.
    fn resend_state(&mut self) {
        for ch in 0..16usize {
            let st = self.chstate[ch];
            let c = ch as u32;
            let cc = |s: &mut Self, slot: usize, num: u32| {
                if st[slot] & 0x80 == 0 {
                    s.send_ch(0xb0 | c | num << 8 | (st[slot] as u32) << 16, 3, true);
                }
            };
            cc(self, 1, 0);
            if st[0] & 0x80 == 0 {
                self.send_ch(0xc0 | c | (st[0] as u32) << 8, 2, true);
            }
            cc(self, 2, 1);
            cc(self, 3, 2);
            cc(self, 4, 5);
            if st[5] & 0x80 == 0 {
                self.vol[ch] = st[5] as u8;
                if self.status & ST_PLAY != 0 {
                    let v = self.scaled_vol(ch) as u32;
                    self.send_ch(0x07b0 | c | v << 16, 3, true);
                }
            }
            cc(self, 6, 10);
            cc(self, 7, 11);
            cc(self, 8, 64);
            cc(self, 9, 65);
            if st[10] & 0x8080 == 0 {
                self.send_ch(0xe0 | c | (st[10] as u32) << 8, 3, true);
            }
        }
    }
}
