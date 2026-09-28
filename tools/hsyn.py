#!/usr/bin/env python3
"""A model of MODHSYN.IRX (CSL hardware synthesizer "hsyn", version
string "PsIImodhsyn 2420" at .data 0xb5d0) as shipped on .hack//Infection:
how a MIDI note on becomes SPU2 voice register values.

All addresses are MODHSYN.IRX module offsets (linked at 0) unless marked
LIBSD. Integer arithmetic follows the R3000 code: C-style truncating division
where the code divides signed values (magic multiplies included), logical
shifts where it uses srl, 16-bit / 8-bit wraps where it stores halves/bytes.

Usage:
    import scei, hsyn as hsyn_ref
    hd = scei.Hd(hd_bytes)
    port = hsyn_ref.PortState(priority=0x40, max_polyphony=16, volume=256, spu_addr=0x5010)
    ch = hsyn_ref.ChannelState()
    hsyn_ref.program_change(hd, ch, 0)
    for v in hsyn_ref.note_on(hd, port, ch, None, 66, 112):
        print(v)          # dict: vag, spu_start, pitch, voll, volr, adsr1, adsr2, ...

(`program` may be None to use the channel's current program, as the IOP
does, or a program index to look it up directly.)

Layers:
  pure functions   sd_note2pitch, pitch_register, rate_ratio, bend_offset,
                   velocity_map, velocity_follow, crossfade, base_level,
                   channel_level, pan_base, pan_gains, output_volumes, adsr1,
                   adsr2, select, Lfo
  note_on()        the voices one note on starts and their key-on registers
  tick_voice()     one 0x4dc0 update of a sounding voice (LFO, portamento,
                   pitch / volume rewrites when they change)
  Synth            the whole module: 4 ports x 16 channels, 48 voices, MIDI
                   parsing (incl. 0xF9/0xFD), note off, sustain, all notes /
                   sound off, allocation and the four steal paths, the KOFF /
                   key-on / KON order of a tick. ENVX/ENDX come from callbacks;
                   every libsd write goes to Synth.log.

Checked against MODHSYN.IRX itself run in tools/iopemu.py: identical libsd
calls for every sound effect (and 3,792 more SE cases with controllers,
bend, port volumes, mono output and F9 overrides), 14,260 note ons over
banks 0, 4, 13, 34, 51, 52, 60 and 70 (every program, the note-range edges,
five velocities), the LFO waves and portamento tick by tick, and 750 ticks
of random four-port music and SE storms under a toy ENVX/ENDX model
(stealing, reclaim, sustain, all notes off, F9/FD). crates/piney-audio's
synthesizer is a port of this model, and its tests/hsyn.rs checks it
against the module's own writes (`tools/iopemu.py se-fixture` and
`song-fixture`).
"""

import struct

# --------------------------------------------------------------------------
# integer helpers

M32 = 0xFFFFFFFF


def s8(x):
    x &= 0xFF
    return x - 0x100 if x & 0x80 else x


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def s32(x):
    x &= M32
    return x - (1 << 32) if x & 0x80000000 else x


def tdiv(a, b):
    """C division: truncate toward zero."""
    q = abs(a) // abs(b)
    return q if (a >= 0) == (b >= 0) else -q


def sra(x, n):
    return s32(x) >> n


def sra_trunc(x, n):
    """`if (x < 0) x += (1 << n) - 1; x >>= n` - the compiler's x / 2**n."""
    x = s32(x)
    if x < 0:
        x += (1 << n) - 1
    return x >> n


def clamp(x, lo, hi):
    return lo if x < lo else hi if x > hi else x


# --------------------------------------------------------------------------
# LIBSD.IRX sceSdNote2Pitch (LIBSD 0x3050), tables at LIBSD .data 0x4860 and
# 0x4878 (12 semitone ratios, 128 fine ratios, both 0x8000 = 1.0).

N2P_SEMI = (0x8000, 0x879C, 0x8FAC, 0x9837, 0xA145, 0xAADC, 0xB504, 0xBFC8,
            0xCB2F, 0xD744, 0xE411, 0xF1A1)
N2P_FINE = (
    0x8000, 0x800E, 0x801D, 0x802C, 0x803B, 0x804A, 0x8058, 0x8067, 0x8076, 0x8085, 0x8094, 0x80A3,
    0x80B1, 0x80C0, 0x80CF, 0x80DE, 0x80ED, 0x80FC, 0x810B, 0x811A, 0x8129, 0x8138, 0x8146, 0x8155,
    0x8164, 0x8173, 0x8182, 0x8191, 0x81A0, 0x81AF, 0x81BE, 0x81CD, 0x81DC, 0x81EB, 0x81FA, 0x8209,
    0x8218, 0x8227, 0x8236, 0x8245, 0x8254, 0x8263, 0x8272, 0x8282, 0x8291, 0x82A0, 0x82AF, 0x82BE,
    0x82CD, 0x82DC, 0x82EB, 0x82FA, 0x830A, 0x8319, 0x8328, 0x8337, 0x8346, 0x8355, 0x8364, 0x8374,
    0x8383, 0x8392, 0x83A1, 0x83B0, 0x83C0, 0x83CF, 0x83DE, 0x83ED, 0x83FD, 0x840C, 0x841B, 0x842A,
    0x843A, 0x8449, 0x8458, 0x8468, 0x8477, 0x8486, 0x8495, 0x84A5, 0x84B4, 0x84C3, 0x84D3, 0x84E2,
    0x84F1, 0x8501, 0x8510, 0x8520, 0x852F, 0x853E, 0x854E, 0x855D, 0x856D, 0x857C, 0x858B, 0x859B,
    0x85AA, 0x85BA, 0x85C9, 0x85D9, 0x85E8, 0x85F8, 0x8607, 0x8617, 0x8626, 0x8636, 0x8645, 0x8655,
    0x8664, 0x8674, 0x8683, 0x8693, 0x86A2, 0x86B2, 0x86C1, 0x86D1, 0x86E0, 0x86F0, 0x8700, 0x870F,
    0x871F, 0x872E, 0x873E, 0x874E, 0x875D, 0x876D, 0x877D, 0x878C)


def sd_note2pitch(center_note, center_fine, note, fine):
    """LIBSD 0x3050 sceSdNote2Pitch(u16 center_note, u16 center_fine, u16 note,
    s16 fine) -> u16 SPU pitch (0x1000 = 1.0 when note == center, fine 0).
    fine is in 1/128 semitone."""
    total = s16(fine) + (center_fine & 0xFFFF)             # a3
    carry = tdiv(total, 128)                               # (a3 [+127]) >> 7
    semi = s16((note & 0xFFFF) + carry - (center_note & 0xFFFF))
    total -= carry << 7                                    # remainder, sign of total
    octs = tdiv(semi, 12)                                  # magic 0x2aaaaaab
    shift = octs - 2                                       # t0
    idx = semi - octs * 12
    if s16(idx) < 0 or (idx == 0 and total < 0):         # 0x30c8 / 0x30d8 -> 0x30e0
        idx += 12
        shift = octs - 3
    if total < 0:                                          # 0x30e8: one semitone down
        idx = idx - 1 + carry                              # (carry != 0 only for
        total += (carry + 1) << 7                          #  fine <= -128: libsd bug)
    p = (N2P_SEMI[s16(idx)] * N2P_FINE[total]) >> 16       # mult, sra 16
    shift = s16(shift)
    if shift < 0:                                          # 0x3144: rounded >> -shift
        n = -shift
        p = (p + (1 << (n - 1))) >> n
    return p & 0xFFFF                                      # shift >= 0: no scaling up


# --------------------------------------------------------------------------
# constants

TICK_US = 4167
# sceHSyn_Init 0x50: 0xb890 = ((tick_us << 8) * 0x10624dd3 >> 32) >> 6
#   = floor(tick_us * 256 / 1000): the tick in 1/256 ms (1066 for 4167 us)
TICK_256MS = (((TICK_US << 8) * 0x10624DD3) >> 32) >> 6

# LFO sine table, .data 0xb690: 200 s16, one cycle, peak 32767


def _sine_table():
    return (0, 1029, 2057, 3083, 4106, 5125, 6139, 7147, 8148, 9141, 10125, 11099, 12062, 13013,
            13951, 14875, 15785, 16679, 17557, 18417, 19259, 20083, 20886, 21669, 22430, 23169,
            23886, 24578, 25247, 25891, 26509, 27100, 27666, 28203, 28713, 29195, 29648, 30072,
            30465, 30829, 31163, 31465, 31737, 31977, 32186, 32363, 32508, 32621, 32702, 32750,
            32767, 32750, 32702, 32621, 32508, 32363, 32186, 31977, 31737, 31465, 31163, 30829,
            30465, 30072, 29648, 29195, 28713, 28203, 27666, 27100, 26509, 25891, 25247, 24578,
            23886, 23169, 22430, 21669, 20886, 20083, 19259, 18417, 17557, 16679, 15785, 14875,
            13951, 13013, 12062, 11099, 10125, 9141, 8148, 7147, 6139, 5125, 4106, 3083, 2057,
            1029) + tuple(-x for x in (0, 1029, 2057, 3083, 4106, 5125, 6139, 7147, 8148, 9141,
            10125, 11099, 12062, 13013, 13951, 14875, 15785, 16679, 17557, 18417, 19259, 20083,
            20886, 21669, 22430, 23169, 23886, 24578, 25247, 25891, 26509, 27100, 27666, 28203,
            28713, 29195, 29648, 30072, 30465, 30829, 31163, 31465, 31737, 31977, 32186, 32363,
            32508, 32621, 32702, 32750, 32767, 32750, 32702, 32621, 32508, 32363, 32186, 31977,
            31737, 31465, 31163, 30829, 30465, 30072, 29648, 29195, 28713, 28203, 27666, 27100,
            26509, 25891, 25247, 24578, 23886, 23169, 22430, 21669, 20886, 20083, 19259, 18417,
            17557, 16679, 15785, 14875, 13951, 13013, 12062, 11099, 10125, 9141, 8148, 7147,
            6139, 5125, 4106, 3083, 2057, 1029))


SINE200 = _sine_table()

# libsd register entries (SD_VP_*, SD_VA_*, SD_S_*): voice = core | voice << 1
VP_VOLL, VP_VOLR, VP_PITCH, VP_ADSR1, VP_ADSR2, VP_ENVX = 0x000, 0x100, 0x200, 0x300, 0x400, 0x500
VA_SSA = 0x2040
S_NON, S_KON, S_KOFF, S_ENDX = 0x1400, 0x1500, 0x1600, 0x1700
S_VMIXL, S_VMIXEL, S_VMIXR, S_VMIXER = 0x1800, 0x1900, 0x1A00, 0x1B00
ADSR2_CUT = 0xC021          # written to ADSR2 by all-sound-off / steals / exclusive groups


# --------------------------------------------------------------------------
# bank access (the IOP walks the raw .hd; scei.Hd drops the empty slots, so the
# maximum indices come from the chunk headers)

class Bank:
    """A loaded bank as sceHSyn_Load (0x47c) records it: pointers to the Head,
    Prog, Sset, Smpl, Vagi (and Setb) chunks plus the SPU address of the .bd."""

    def __init__(self, hd, spu_addr=0):
        self.hd = hd
        self.spu_addr = spu_addr
        d = hd.data

        def top(name):
            o = hd.chunks[name]
            if o == 0xFFFFFFFF:
                return -1
            return struct.unpack_from("<I", d, o + 12)[0]

        self.prog_max = top("Prog")
        self.sset_max = top("Sset")
        self.smpl_max = top("Smpl")
        self.vagi_max = top("Vagi")
        self.programs = {p.index: p for p in hd.programs}
        self.ssets = {s.index: s for s in hd.samplesets}
        self.samples = {s.index: s for s in hd.samples}
        self.vags = {v.index: v for v in hd.vags}


def as_bank(hd_or_bank, spu_addr=0):
    return hd_or_bank if isinstance(hd_or_bank, Bank) else Bank(hd_or_bank, spu_addr)


# --------------------------------------------------------------------------
# port and channel state

class PortState:
    """sceHSynEnv + the 'system' words that matter.
    priority / max_polyphony: env+0 / env+1 (SNDBASE ccSetPortAttr: attr & 0xff,
    attr >> 8; port 0 = 0x40/16, ports 1-3 = 0x10/32).
    volume: system+0 u16 (sceHSyn_SetVolume 0x1bc4; init 256 in 0x50).
    voices: system+2 u8, voices this port holds (incremented at note on 0x69d8,
    decremented when a voice is freed 0x62f4/0x5e60).
    mono: global 0xf0c0 (sceHSyn_SetOutputMode 0x2044: mode 0 -> 1 = mono,
    mode 1 -> 0 = stereo; Init leaves 0 = stereo).
    velocity_maps: env.velocityMapTbl rows (126 bytes each), unused by the game."""

    def __init__(self, priority=0x40, max_polyphony=16, volume=256, spu_addr=0x5010,
                 mono=False, velocity_maps=(), wave_type=0):
        self.priority = priority
        self.max_polyphony = max_polyphony
        self.volume = volume
        self.spu_addr = spu_addr
        self.mono = mono
        self.velocity_maps = list(velocity_maps)
        self.wave_type = wave_type      # env+3; 0 in the game (1 = SE-timbre mode)
        self.voices = 0


def cc_scale127(v):
    """0x9f8 / 0xa54 / 0xab0 / 0xb78: ((v & 0xff) << 7) / 127 (magic 0x81020409)."""
    return tdiv((v & 0xFF) << 7, 127) & 0xFF


class ChannelState:
    """The 56-byte channel (system+4 + 56*ch), initialised by 0x760 and
    changed by the controller handlers (0x1130 jump table at .rodata 0xb280)."""

    def __init__(self):
        self.program = None     # +24 program record (program change 0x7dc)
        self.bank = 0           # +42 CC0
        self.porta_time = 0     # +36 CC5 * 20
        self.bend = 0           # +38 s16: (msb << 7 | lsb) - 8192 (0x878)
        self.note_bend = -1     # +40 s16 per-note bend (0xF9 msg), -1 = none
        self.volume = 128       # +43 CC7 scaled 0..128 (init 128)
        self.expression = 128   # +44 CC11 scaled 0..128 (init 128)
        self.breath = 0         # +45 CC2 scaled -> amp LFO midi depth
        self.modulation = 0     # +46 CC1 scaled -> pitch LFO midi depth
        self.note_expr = 255    # +47 per-note expression (0xF9 msg), 255 = none
        self.note_pan = 0xFF    # +48 per-note pan (0xF9 msg), bit7 = none
        self.pan = 0            # +49 s8: CC10 - 64 clamped to -63..63 (0xc98)
        self.sustain = 0        # +50 CC64 value (any nonzero holds)
        self.porta_note = 255   # +51 CC65: 255 off, else last note (<=128 when on)
        self.porta_ctrl = 0     # +52 CC84
        self.nrpn = [255, 255, 255]  # +53 +54 +55


def program_change(bank, ch, prog_no, port=None):
    """0x7dc: look the program up in the bank selected by CC0 (bank slots are
    per port; the game only loads slot 0)."""
    bank = as_bank(bank)
    p = None
    if 0 <= prog_no <= bank.prog_max:
        p = bank.programs.get(prog_no)
        if p is not None and p.n_split == 0:
            p = None
    ch.program = p
    return p


def control_change(ch, cc, value):
    """0x1130 (jump table .rodata 0xb280, cc < 124). Returns what the IOP then
    does to the channel's voices (the per-voice effect is applied by
    recompute_* below; register writes happen in the next 0x4dc0 pass)."""
    value &= 0xFF
    if cc == 0:
        ch.bank = value                                  # 0x1168
    elif cc == 1:
        ch.modulation = cc_scale127(value)               # 0xb78 -> 0x4990 per voice
    elif cc == 2:
        ch.breath = cc_scale127(value)                   # 0xab0 -> 0x48e8 per voice
    elif cc == 5:
        ch.porta_time = (value * 20) & 0xFFFF            # 0xc40
    elif cc == 6:                                        # 0x1270 data entry (NRPN only)
        msb, lsb = ch.nrpn[0], ch.nrpn[1]
        ch.nrpn[2] = value
        if lsb != 255 and msb in (2, 3):
            ch.nrpn_event = (msb, lsb, value)            # 0x1e60 reverb type / 0x1ed0 depth...
        ch.nrpn = [255, 255, 255]
    elif cc == 7:
        ch.volume = cc_scale127(value)                   # 0x9f8 -> 0x93c(ch, 0)
    elif cc == 10:
        p = s8(value - 64)                               # 0xc98
        ch.pan = -63 if p < -63 else p
    elif cc == 11:
        ch.expression = cc_scale127(value)               # 0xa54 -> 0x93c(ch, 1)
    elif cc == 64:
        ch.sustain = value                               # 0x5f94 (0 releases held voices)
    elif cc == 65:
        if value:                                        # 0xc58
            if ch.porta_note == 255:
                ch.porta_note = 128
        else:
            ch.porta_note = 255
    elif cc == 84:
        ch.porta_ctrl = value                            # 0xc90
    elif cc == 98:
        ch.nrpn[1] = value                               # 0x1268
    elif cc == 99:
        ch.nrpn = [value, 255, 255]                      # 0x1254
    elif cc == 121:
        reset_all_controllers(ch)                        # 0x10b0
    # 120 all sound off (0x6214), 123 all notes off (0x60d4): voice lists
    return ch


def pitch_bend(ch, lsb, msb):
    """0x878: E0 lsb msb -> ((msb & 0xff) << 7) + (lsb & 0xff) - 8192 (s16)."""
    ch.bend = s16(((msb & 0xFF) << 7) + (lsb & 0xFF) - 8192)
    return ch.bend


def reset_all_controllers(ch):
    """0x10b0 (CC121 and sceHSyn_ResetAllControler): sustain off, bend centre,
    CC2 0, CC1 0, CC7 127 (->128), pan 64 (->0), CC11 127 (->128), porta off."""
    ch.sustain = 0
    pitch_bend(ch, 0, 64)
    ch.breath = 0
    ch.modulation = 0
    ch.volume = cc_scale127(127)
    ch.pan = 0
    ch.expression = cc_scale127(127)
    ch.porta_note = 255


# --------------------------------------------------------------------------
# velocity curves (0x8fe4, function table .data 0xb668) and velocity-follow
# curves (0x8e5c, table .data 0xb640)

def _sq127(v):
    """0x8f1c: v*v / 127 (magic 0x2040811), 0 -> 1."""
    r = (v * v) // 127
    return (r & 0xFF) if r else 1


VEL_CURVES = {
    0: lambda v: v & 0xFF,                          # 0x8f04 linear
    1: lambda v: (128 - v) & 0xFF,                  # 0x8f0c inverted
    2: lambda v: _sq127(v),                         # 0x8f1c square
    3: lambda v: (128 - _sq127(v)) & 0xFF,          # 0x8f60
    4: lambda v: (128 - _sq127((128 - v) & 0xFF)) & 0xFF,   # 0x8f88
    5: lambda v: _sq127((128 - v) & 0xFF) & 0xFF,   # 0x8fbc
}
for _i in range(4):
    VEL_CURVES[6 + _i] = VEL_CURVES[_i]             # entries 6..9 repeat 0..3


def velocity_map(vel, curve, maps=()):
    """0x8fe4(vel, curve, env.velocityMapTbl, env.velocityMapNum): curve high
    nibble = 1 + index of a 126-byte velocity map (none in the game), low
    nibble = curve function (>= 11 -> 0; 10 would jump into the sine table)."""
    vel &= 0xFF
    if vel == 0:
        vel = 1
    m = (curve >> 4) - 1
    if m != -1 and m < len(maps):
        vel = maps[m][vel - 1]
    fn = curve & 0xF
    if fn >= 11:
        fn = 0
    return VEL_CURVES[fn](vel) & 0xFF


def _vf(x, d):
    return tdiv(x, d)


def _vf_sq(vel, center):
    a = tdiv((vel - 1) << 15, 126)
    b = tdiv((center - 1) << 15, 126)
    return a, b


VEL_FOLLOW = {
    0: lambda v, c: tdiv((v - c) << 16, 126),                                   # 0x8950
    1: lambda v, c: tdiv((c - v) << 16, 126),                                   # 0x8984
    2: lambda v, c: sra_trunc(_vf_sq(v, c)[0] ** 2 - _vf_sq(v, c)[1] ** 2, 14),  # 0x89b8
    3: lambda v, c: ((0x10000 - sra_trunc(_vf_sq(v, c)[0] ** 2, 14))
                     - (0x10000 - sra_trunc(_vf_sq(v, c)[1] ** 2, 14))),        # 0x8a30
    4: lambda v, c: ((0x10000 - sra_trunc((0x10000 - _vf_sq(v, c)[0]) ** 2, 14))
                     - (0x10000 - sra_trunc((0x10000 - _vf_sq(v, c)[1]) ** 2, 14))),  # 0x8abc
    5: lambda v, c: (sra_trunc((0x10000 - _vf_sq(v, c)[0]) ** 2, 14)
                     - sra_trunc((0x10000 - _vf_sq(v, c)[1]) ** 2, 14)),        # 0x8b50
}


def _vf6(v, c, neg):                                  # 0x8be0 / 0x8c4c
    if v == c:
        return 0
    d = 127 - c if c < v else c - 1
    return tdiv(((c - v) if neg else (v - c)) << 16, d)


def _vf8(v, c):                                       # 0x8cb8
    if v == c:
        return 0
    x = (v - c) << 15
    if c < v:
        q = tdiv(x, 127 - c)
        return sra_trunc(q * q, 14)
    q = tdiv(x, c - 1)
    return -sra_trunc(q * q, 14)


def _vf9(v, c):                                       # 0x8d84
    if v == c:
        return 0
    x = (v - c) << 15
    if c < v:
        r = 0x8000 - tdiv(x, 127 - c)
        return 0x10000 - sra_trunc(r * r, 14)
    r = tdiv(x, c - 1) + 0x8000
    return sra_trunc(r * r, 14) - 0x10000


VEL_FOLLOW[6] = lambda v, c: _vf6(v, c, False)
VEL_FOLLOW[7] = lambda v, c: _vf6(v, c, True)
VEL_FOLLOW[8] = _vf8
VEL_FOLLOW[9] = _vf9
VEL_FOLLOW[10] = lambda v, c: v & 0xFF               # 0x8f04 (table slot 10)


def velocity_follow(vel, amount, center, curve, maps=()):
    """0x4884(voice, amount s8, center, curve) -> s16-ish offset:
    (0x8e5c(vel, center, curve) * amount) / 65536 (truncating)."""
    vel &= 0xFF
    if vel == 0:
        vel = 1
    m = (curve >> 4) - 1
    if m != -1 and m < len(maps):
        vel = maps[m][vel - 1]
        center = maps[m][center - 1]
    fn = curve & 0xF
    if fn >= 11:
        fn = 0
    r = VEL_FOLLOW[fn](vel, center & 0xFF)
    return sra_trunc(s32(r * s8(amount)), 16)


# --------------------------------------------------------------------------
# key follow on the envelope (0x3a74, 0x3b18 via 0x39ec)

def _kf_field(value, amount, key_delta, mask, shift):
    """0x39ec: d = (key_delta * amount) / 12; field += d, clamped 0..mask."""
    d = tdiv(key_delta * s8(amount), 12)
    if d == 0:
        return value & 0xFFFF
    f = ((value & 0xFFFF) >> shift) & mask
    f = clamp(f + d, 0, mask)
    return ((value & ~(mask << shift)) | (f << shift)) & 0xFFFF


def adsr1(note, smp):
    """0x3a74: sample ADSR1 (+0x12) with key follow on Ar (bits 8-14),
    Dr (bits 4-7) and Sl (bits 0-3)."""
    v = smp.adsr1
    if smp.kf_ar:
        v = _kf_field(v, smp.kf_ar, note - smp.kf_ar_center, 0x7F, 8)
    if smp.kf_dr:
        v = _kf_field(v, smp.kf_dr, note - smp.kf_dr_center, 0x0F, 4)
    if smp.kf_sl:
        v = _kf_field(v, smp.kf_sl, note - smp.kf_sl_center, 0x0F, 0)
    return v


def adsr2(note, smp):
    """0x3b18: sample ADSR2 (+0x14) with key follow on Sr (bits 6-12) and Rr
    (bits 0-4)."""
    v = smp.adsr2
    if smp.kf_sr:
        v = _kf_field(v, smp.kf_sr, note - smp.kf_sr_center, 0x7F, 6)
    if smp.kf_rr:
        v = _kf_field(v, smp.kf_rr, note - smp.kf_rr_center, 0x1F, 0)
    return v


# --------------------------------------------------------------------------
# sample selection (note on, 0x65bc)

def select(bank, program, note, vel):
    """0x65bc's loops: every split whose key range holds the note, every sample
    of its sample set whose velocity range holds the velocity. Yields
    (split, sset, sample, vag_or_None). vag None = a noise voice (the VAG index
    is past the Vagi table). The range bytes' bit 7 is the crossfade flag and
    is masked off for the range test."""
    bank = as_bank(bank)
    for sp in program.splits[:program.n_split]:        # stride always 20 (0x6ad4)
        if note < (sp.key_low & 0x7F) or (sp.key_high & 0x7F) < note:
            continue
        if sp.sample_set > bank.sset_max:
            continue
        ss = bank.ssets.get(sp.sample_set)
        if ss is None:
            continue                                   # (IOP would read offset -1: garbage)
        if vel < ss.vel_low or ss.vel_high < vel:
            continue
        for si in ss.samples:
            if si > bank.smpl_max:
                continue
            smp = bank.samples.get(si)
            if smp is None:
                continue
            if vel < (smp.vel_low & 0x7F) or (smp.vel_high & 0x7F) < vel:
                continue
            vag = None
            if smp.vag <= bank.vagi_max:
                vag = bank.vags.get(smp.vag)
                if vag is None or vag.offset == 0xFFFFFFFF:
                    continue                           # 0x6824: VAG offset -1 -> skip
            yield sp, ss, smp, vag


# --------------------------------------------------------------------------
# crossfade (0x4c68), pan (0x4a40, 0x4b14), volume (0x3b94, 0x484c, 0x4644)

def crossfade(smp, sp, vel, note):
    """0x4c68 -> 16.16 gain (0x10000 = 1). Velocity: sample bytes +2 low,
    +3 crossfade point, +4 high; bit 7 of low/high enables the fade.
    Key: split +2/+3/+4 the same way."""
    g = 0x10000
    xf = smp.vel_crossfade
    if vel >= xf:
        if smp.vel_high & 0x80:
            h = smp.vel_high & 0x7F
            if h != xf:
                h += 1
                g = ((h - vel) << 16 & M32) // ((h - xf) & M32)
    else:
        if smp.vel_low & 0x80:
            lo = smp.vel_low & 0x7F
            if lo != xf:
                lo -= 1
                g = ((vel - lo) << 16 & M32) // ((xf - lo) & M32)
    xf = sp.key_crossfade
    if note >= xf:
        if sp.key_high & 0x80:
            h = sp.key_high & 0x7F
            if h != xf:
                h += 1
                g = (g * (h - note) & M32) // ((h - xf) & M32)
    else:
        if sp.key_low & 0x80:
            lo = sp.key_low & 0x7F
            if lo != xf:
                lo -= 1
                g = (g * (note - lo) & M32) // ((xf - lo) & M32)
    return g


def pan_base(prog, sp, smp, note):
    """0x3b94 @0x3dd4 + 0x4a40 -> (pan 'v+30', phase flag 'v+83' or None).
    Key follow: ((note - split.kfpan_c) * split.kfpan + (note - prog.kfpan_c) *
    prog.kfpan) / 12. Normal programs: |prog.pan| - 128 + |split.pan| +
    |sample.pan| + kf clamped 0..127 (64 = centre). prog.attr bit 0: signed sum
    wrapped by 255 into -128..127 (negative = phase-inverted side)."""
    kf = tdiv((note - sp.key_follow_pan_center) * sp.key_follow_pan
              + (note - prog.key_follow_pan_center) * prog.key_follow_pan, 12)
    if prog.attr & 1:
        a = prog.pan - 128 + sp.pan + smp.pan + kf
        while a >= 128:
            a -= 255
        while a < -128:
            a += 255
        return a, None
    a = abs(prog.pan) - 128 + abs(sp.pan) + abs(smp.pan) + kf
    return clamp(a, 0, 127), 1


def pan_gains(base, phase, chan_pan, prog_attr, mono=False):
    """0x4b14(voice, pan): -> (gain L, gain R, phase flag). Gains 0..128.
    chan_pan = channel +49 (CC10 - 64, -63..63) or the per-note pan override
    (v+86 - 64 clamped at -63). Linear law: the far side is cut to
    (63 - |p|) * 128 / 63, the near side stays 128."""
    gl = gr = 128
    if mono:                                   # 0xf0c0 != 0: no pan at all
        return gl, gr, phase
    p = clamp(s8(chan_pan), -63, 63)
    if prog_attr & 1:
        a = s16(base)
        ph = 1
        if a < 0:
            a = -a
            ph = 0
        a += p
        if a < 0:
            ph ^= 1
            a = -a
        elif a >= 128:
            ph ^= 1
            a = 256 - a
        phase = ph
        a -= 64
    else:
        a = p + s16(base) - 64
    if a > 0:
        gl = 0 if a >= 63 else tdiv((63 - a) << 7, 63)
    elif a < 0:
        gr = 0 if a <= -63 else tdiv((a + 63) << 7, 63)
    return gl, gr, phase


def base_level(prog, sp, smp, vel_mapped, xfade):
    """0x3b94 @0x3d80: ((prog.vol * split.vol * sample.vol * vel) / 16384) *
    xfade >> 16 -> voice +22 (u16)."""
    t = prog.volume * sp.volume * smp.volume * vel_mapped
    t = sra_trunc(t, 14)
    return ((t * xfade) & M32) >> 16 & 0xFFFF


def channel_level(level22, ch_volume, expression):
    """0x484c: (ch.vol * expression * level22) >> 14 (logical) -> voice +24."""
    return ((ch_volume * expression * level22) & M32) >> 14 & 0xFFFF


def output_volumes(level24, port_volume, gl, gr, phase, amp_lfo=0, amp_depth_pos=0,
                   amp_depth_neg=0):
    """0x4644: port volume, amp LFO, pan gains -> (VOLL, VOLR) register values."""
    a = sra_trunc(s32(level24 * port_volume), 8)
    d = amp_depth_pos if amp_lfo >= 0 else amp_depth_neg
    m = sra_trunc(amp_lfo * d, 7) + 16384
    a = sra_trunc(s32(a * m), 14)
    if a < 0:
        a = 0
    l = ((gl * a) & M32) >> 7
    r = ((gr * a) & M32) >> 7
    l = 0x3FFF if l >= 0x4000 else l
    r = 0x3FFF if r >= 0x4000 else r
    if phase:
        return l, r
    return (-l) & 0x7FFF, (-r) & 0x7FFF


# --------------------------------------------------------------------------
# pitch (0x3b94 @0x3ea8, 0x45f0, 0x43e8)

def rate_ratio(rate):
    """0x3b94 @0x3ec8: ((rate << 16) * 0x057619f1 >> 32) >> 10 =
    floor(rate * 65536 / 48000), 16.16 (voice +60)."""
    return ((((rate & 0xFFFF) << 16) * 0x057619F1) >> 32) >> 10


def bend_offset(sp, bend):
    """0x45f0: (bend >= 0 ? split.bend_high : split.bend_low) * bend / 8192
    (truncating) -> voice +52, in 1/128 semitone."""
    rng = sp.bend_high if bend >= 0 else sp.bend_low
    return sra_trunc(s32(rng * bend), 13)


def pitch_register(prog, sp, smp, vag, note, fine_offset=0):
    """0x43e8 for a hsyn-mode voice (v+89 == 0, no fixed pitch v+44):
    fine_offset = LFO part + bend part (+52) + portamento (+56), in 1/128
    semitone. Returns the PITCH register value (<= 0x3fff)."""
    key = note + prog.transpose + sp.transpose
    fine = fine_offset + prog.detune + smp.detune + sp.detune
    if fine < 0:                                  # 0x4554: floor-normalise
        q = (fine + 127) >> 7
        key += q
        fine -= q << 7
        if fine != 0:
            fine += 128
            key -= 1
    p = sd_note2pitch(smp.base_note, 0, key & 0xFFFF, s16(fine))
    p = ((p * rate_ratio(vag.rate)) & M32) >> 16
    return 0x3FFF if p >= 0x4000 else p


def lfo_pitch_part(lfo_out, depth_pos, depth_neg):
    """0x43e8 @0x4400: lfo * (lfo >= 0 ? v+32 : v+34) / 16384 (truncating)."""
    d = depth_pos if lfo_out >= 0 else depth_neg
    return sra_trunc(s32(d * lfo_out), 14)


# --------------------------------------------------------------------------
# LFOs (init 0x93b4 / 0x97b0, tick 0x91ac, key-off 0x96f8, start 0x936c)

_rng = [0x02CEE9D7]         # .data 0xb820


def hsyn_rand():
    """0x9b00: seed = seed * 0x41c64e6d + 12345; return (s32)seed >> 16."""
    _rng[0] = (_rng[0] * 0x41C64E6D + 12345) & M32
    return s32(_rng[0]) >> 16


class Lfo:
    """The 40-byte LFO at voice +120 (pitch) or +160 (amp).
    state: 0 off, 1 delay, 2 fade in, 3 run, 4 release delay, 5 fade out, 6 done."""

    def __init__(self, prog, attr, delay_ms, fade_ms, which, tick=TICK_256MS):
        self.state = 0
        self.keyoff = 0
        self.out = 0
        self.wave = None
        self.table = None
        self.count = 0
        self.fade = 0
        self.fade_inc = 0
        self.phase = 0
        self.inc = 0
        if attr & 0x77 == 0:
            return
        w = prog.lfo_wave2 if which else prog.lfo_wave
        if w & 0x80:
            return          # user wave from env.lfoWaveTbl: the game has none
        if w == 0 or w >= 7:
            return
        self.wave = w
        self.delay = ((delay_ms << 8) + (tick >> 1)) // tick
        f = ((fade_ms << 8) + (tick >> 1)) // tick
        if f:
            f = 0x10000 // f
            if f == 0:
                f = 1
        self.fade_inc = f
        rnd = prog.lfo_random2 if which else prog.lfo_random
        ph = prog.lfo_phase2 if which else prog.lfo_phase
        cyc = prog.lfo_cycle2 if which else prog.lfo_cycle
        r = rnd << 8
        r = sra_trunc(s32(r * hsyn_rand()), 16)
        r += ph << 8
        if r < 0:
            r += 0x10000
        elif r > 0xFFFF:
            r -= 0x10000
        self.phase = r
        self.inc = 0x10000 // (((cyc << 8) + (tick >> 1)) // tick)
        if attr & 0x66:
            self.keyoff = 1
        if attr & 0x55:
            self._start()

    def _start(self):                                  # 0x936c
        if self.delay:
            self.state, self.count = 1, self.delay
        elif self.fade_inc:
            self.state, self.fade = 2, 0
        else:
            self.state = 3

    def _wave(self, wrapped):
        p = self.phase
        w = self.wave
        if w == 1:                                     # 0x9070 saw
            return s16(((p - 32768) << 15) >> 16)
        if w == 2:                                     # 0x9088 inverted saw
            return s16(((0xFFFF - p - 32768) << 15) >> 16)
        if w == 3:                                     # 0x90a8 triangle
            if p > 0xBFFF:
                p -= 0x10000
            elif p > 0x4000:
                p = 0x8000 - p
            return s16(p)
        if w == 4:                                     # 0x90e4 square
            return 16384 if p > 32767 else -16384
        if w == 5:                                     # 0x9178 sample & hold noise
            return s16(hsyn_rand()) >> 2 if wrapped else self.out
        # 6: 0x9104 table (sine, 200 entries)
        t = SINE200
        x = p * 200
        i = x >> 16
        j = i + 1
        if j >= 200:
            j = 0
        a = t[i]
        v = a + (((t[j] - a) * (x & 0xFFFF)) >> 16)
        v = s32(v)
        return s16(((v + (1 if v < 0 else 0)) << 15) >> 16)

    def tick(self):
        """0x91ac -> output (s16, about +-16384)."""
        s = self.state
        v = 0
        if s == 0 or s >= 6:
            self.out = 0
            return 0
        if s >= 2:
            self.phase += self.inc
            wrapped = 0
            if self.phase > 0xFFFF:
                self.phase -= 0x10000
                wrapped = 1
            v = self._wave(wrapped)
        if s == 1:
            self.count -= 1
            if self.count & 0xFFFF:
                self.out = 0
                return 0
            if self.fade_inc:
                self.state, self.fade = 2, 0
            else:
                self.state = 3
            v = 0
        elif s == 2:
            v = s16((s32(v * self.fade) if v * self.fade >= 0 else v * self.fade + 0xFFFF) >> 16)
            self.fade += self.fade_inc
            if self.fade > 0xFFFF:
                self.state = 3
        elif s == 4:
            self.count -= 1
            if self.count & 0xFFFF == 0:
                if self.fade_inc:
                    self.fade, self.state = 0x10000, 5
                else:
                    self.state = 6
        elif s == 5:
            t = v * self.fade
            v = s16((t if t >= 0 else t + 0xFFFF) >> 16)
            self.fade -= self.fade_inc
            if self.fade <= 0:
                self.state = 6
        self.out = s16(v)
        return self.out

    def key_off(self):
        """0x96f8."""
        if not self.keyoff:
            return
        s = self.state
        if s == 1:
            self.state = 6
        elif s == 0:
            self._start()
        elif s in (2, 3):
            if self.delay:
                self.state, self.count = 4, self.delay
            elif self.fade_inc:
                self.state, self.fade = 5, 0x10000
            else:
                self.state = 6
        else:
            self.state = 6


# --------------------------------------------------------------------------
# a voice's parameters at note on (0x65bc + 0x3b94) and at key on (0x3f50)

class Voice:
    pass


def setup_voice(bank, port, ch, prog, sp, ss, smp, vag, note, vel):
    """0x3b94 (called from 0x65bc @0x6a34): everything a voice needs."""
    v = Voice()
    v.prog, v.split, v.sset, v.sample, v.vag = prog, sp, ss, smp, vag
    v.note, v.vel = note, vel
    v.priority = (smp.priority + port.priority) & 0xFFFF     # 0x6844 -> +20
    v.group = smp.group
    v.spu_attr = smp.spu_attr
    maps = port.velocity_maps
    # LFOs: pitch (+120) with sample.lfo_attr & 7, amp (+160) with & 0x70
    v.lfo_pitch = Lfo(prog, smp.lfo_attr & 0x07, smp.pitch_lfo_delay, smp.pitch_lfo_fade, 0)
    v.lfo_amp = Lfo(prog, smp.lfo_attr & 0x70, smp.amp_lfo_delay, smp.amp_lfo_fade, 1)
    # pitch LFO depth: program depth + split key follow + sample velocity follow
    kf = tdiv((note - sp.key_follow_pitch_center) * sp.key_follow_pitch, 12)
    vf = velocity_follow(vel, smp.vel_follow_pitch, smp.vel_follow_pitch_center,
                         smp.vel_follow_pitch_curve, maps)
    v.pdepth_base = (clamp(prog.lfo_pitch_depth + kf + vf, -32768, 32767),
                     clamp(prog.lfo_pitch_depth2 + kf + vf, -32768, 32767))   # +36 +38
    kf = tdiv((note - sp.key_follow_amp_center) * sp.key_follow_amp, 12)
    vf = velocity_follow(vel, smp.vel_follow_amp, smp.vel_follow_amp_center,
                         smp.vel_follow_amp_curve, maps)
    v.adepth_base = (clamp(prog.lfo_amp_depth + kf + vf, -128, 127),
                     clamp(prog.lfo_amp_depth2 + kf + vf, -128, 127))         # +81 +82
    set_breath(v, ch.breath)                                                   # 0x48e8
    set_modulation(v, ch.modulation)                                           # 0x4990
    # level
    vm = velocity_map(vel, ss.vel_curve, maps)                                 # 0x8fe4
    v.xfade = crossfade(smp, sp, vel, note)                                    # 0x4c68
    v.level22 = base_level(prog, sp, smp, vm, v.xfade)
    # pan
    v.pan_base, ph = pan_base(prog, sp, smp, note)
    # +83 phase flag: 1 from 0x4a40 for normal programs; attr programs get it
    # from 0x4b14, which returns before setting it in mono mode - then the
    # hardware voice's previous value stays (0 for a never-used voice, which
    # inverts both volumes). stale_phase models that.
    v.phase = getattr(port, "stale_phase", 0) if ph is None else ph
    v.mono = port.mono
    v.prog_attr = prog.attr & 1                                               # +78
    if ch.note_pan & 0x80:
        cp = ch.pan
    else:
        cp = s8(ch.note_pan - 64)
        cp = -63 if cp < -63 else cp
    v.gl, v.gr, v.phase = pan_gains(v.pan_base, v.phase, cp, v.prog_attr, port.mono)
    v.expr = ch.expression if ch.note_expr == 255 else ch.note_expr            # +84
    v.level24 = channel_level(v.level22, ch.volume, v.expr)
    # portamento (0x5228, called from 0x65bc @0x6a2c just before 0x3b94):
    # glide from the channel's last released note (+51, set by note off
    # 0x5cb4 while CC65 is on) when CC5 time (+36) is non-zero
    v.flags = 0x208                          # 0x6908: key held (0x200) + pending (8)
    v.porta = 0                              # +56, 1/128 semitone
    v.porta_step = 0                         # +42
    if not (ch.porta_note & 0x80) and ch.porta_time:
        d = (ch.porta_note - note) << 7
        v.porta = d
        if d < 0:
            v.flags |= 2
            d = -d
        else:
            v.flags |= 4
        q = ((d * TICK_256MS) & M32) // ch.porta_time
        v.porta_step = sra_trunc(q, 8) & 0xFFFF
    v.pitch_cache = None                     # +48 (0x7fffffff at 0x3ec4)
    v.vol_cache = None                       # +26/+28 (0xffff at 0x3eb0)
    # pitch
    if vag is not None:
        v.ratio = rate_ratio(vag.rate)
        b = ch.bend if ch.note_bend & 0x8000 or ch.note_bend < 0 else ch.note_bend - 8192
        v.bend = bend_offset(sp, b)
    else:
        v.ratio = 0
        v.bend = 0
    return v


def set_modulation(v, cc1):
    """0x4990: pitch LFO depth (+32/+34) = base + cc1 * prog.lfo_midi_pitch_depth[2] / 128."""
    p = v.prog
    v.pdepth = (clamp(sra_trunc(cc1 * p.lfo_midi_pitch_depth, 7) + v.pdepth_base[0], -32768, 32767),
                clamp(sra_trunc(cc1 * p.lfo_midi_pitch_depth2, 7) + v.pdepth_base[1], -32768, 32767))


def set_breath(v, cc2):
    """0x48e8: amp LFO depth (+79/+80) = base + cc2 * prog.lfo_midi_amp_depth[2] / 128."""
    p = v.prog
    v.adepth = (clamp(sra_trunc(cc2 * p.lfo_midi_amp_depth, 7) + v.adepth_base[0], -128, 127),
                clamp(sra_trunc(cc2 * p.lfo_midi_amp_depth2, 7) + v.adepth_base[1], -128, 127))


def update_pitch(v):
    """0x43e8 with its cache: returns the new PITCH value, or None when the
    fine sum (LFO part + bend + portamento) is unchanged (no register write)
    or the voice is a noise voice."""
    if v.vag is None:
        return None
    fine = lfo_pitch_part(v.lfo_pitch.out, *v.pdepth) + v.bend + v.porta
    if fine == v.pitch_cache:
        return None
    v.pitch_cache = fine
    return pitch_register(v.prog, v.split, v.sample, v.vag, v.note, fine)


def update_volume(v, port):
    """0x4644 with its cache (+26/+28): (VOLL, VOLR) or None if unchanged."""
    a = sra_trunc(s32(v.level24 * port.volume), 8)
    lfo = v.lfo_amp.out
    d = v.adepth[0] if lfo >= 0 else v.adepth[1]
    a = sra_trunc(s32(a * (sra_trunc(lfo * d, 7) + 16384)), 14)
    if a < 0:
        a = 0
    l = min(((v.gl * a) & M32) >> 7, 0x3FFF)
    r = min(((v.gr * a) & M32) >> 7, 0x3FFF)
    if (l, r) == v.vol_cache:
        return None
    v.vol_cache = (l, r)
    if v.phase:
        return l, r
    return (-l) & 0x7FFF, (-r) & 0x7FFF


def tick_voice(v, port):
    """One 0x4dc0 pass over an active voice (after the ENVX/ENDX free check):
    portamento step, pitch LFO, pitch (VAG voices only), amp LFO, volume.
    Returns {'pitch': p, 'vol': (l, r)} for the registers it writes."""
    out = {}
    if v.vag is not None:
        if v.flags & 6:
            if v.flags & 2:
                v.porta += v.porta_step
                if v.porta >= 0:
                    v.porta = 0
                    v.flags &= ~2
            else:
                v.porta -= v.porta_step
                if v.porta <= 0:
                    v.porta = 0
                    v.flags &= ~4
        v.lfo_pitch.tick()
        p = update_pitch(v)
        if p is not None:
            out["pitch"] = p
    v.lfo_amp.tick()
    vol = update_volume(v, port)
    if vol is not None:
        out["vol"] = vol
    return out


def channel_changed(v, ch, what):
    """What a controller does to a live voice of its channel (the per-voice
    calls in 0x93c, 0xab0, 0xb78, 0xc98, 0x878); registers follow in
    tick_voice."""
    if what == "volume":                     # CC7: 0x93c(ch, 0) -> 0x484c
        v.level24 = channel_level(v.level22, ch.volume, v.expr)
    elif what == "expression":               # CC11: 0x93c(ch, 1): +84 = ch+44, 0x484c
        v.expr = ch.expression
        v.level24 = channel_level(v.level22, ch.volume, v.expr)
    elif what == "pan":                      # CC10: 0x4b14(v, ch+49)
        v.gl, v.gr, v.phase = pan_gains(v.pan_base, v.phase, ch.pan, v.prog_attr, v.mono)
    elif what == "bend" and v.vag is not None:   # 0x45f0
        v.bend = bend_offset(v.split, ch.bend)
    elif what == "modulation":               # CC1: 0x4990
        set_modulation(v, ch.modulation)
    elif what == "breath":                   # CC2: 0x48e8
        set_breath(v, ch.breath)


def voice_registers(v, port):
    """What 0x3f50 writes at key on (in this order): VOLL, VOLR (0x4644), PITCH
    (0x43e8), SSA (0x9e4c), ADSR1, ADSR2, then the NON / VMIX switches and the
    KON bit (the KON switch itself is written once per tick per core, 0x390c)."""
    voll, volr = output_volumes(v.level24, port.volume, v.gl, v.gr, v.phase,
                                v.lfo_amp.out, v.adepth[0], v.adepth[1])
    out = {
        "sample": v.sample.index, "split_keys": (v.split.key_low, v.split.key_high),
        "vag": v.vag.index if v.vag else None,
        "noise": v.vag is None,
        "voll": voll, "volr": volr,
        "adsr1": adsr1(v.note, v.sample), "adsr2": adsr2(v.note, v.sample),
        "vmixl": bool(v.spu_attr & 1), "vmixr": bool(v.spu_attr & 2),
        "vmixel": bool(v.spu_attr & 4), "vmixer": bool(v.spu_attr & 8),
        "reverb": bool(v.spu_attr & 0xC),
        "core_pref": {0x10: 0, 0x20: 1}.get(v.spu_attr & 0x30),
        "priority": v.priority,
        "level22": v.level22, "level24": v.level24, "gl": v.gl, "gr": v.gr,
    }
    if v.vag is not None:
        fine = lfo_pitch_part(v.lfo_pitch.out, *v.pdepth) + v.bend + v.porta
        out["pitch"] = pitch_register(v.prog, v.split, v.sample, v.vag, v.note, fine)
        out["spu_start"] = (port.spu_addr + v.vag.offset) & M32
        out["loop"] = bool(v.vag.loop)
    else:
        out["pitch"] = None
        out["spu_start"] = None
        out["noise_clock"] = min(v.note, 63)            # 0x4048: SD_C_NOISE_CLK
    return out


def note_on(hd, port_state, channel_state, program, note, velocity, bank=None):
    """0x65bc for a hsyn-mode port: the voices a note on starts, with the
    register values 0x3f50 writes when they key on. `program` is a program
    index or record, or None for the channel's current program (the IOP only
    uses the channel's). Velocity 0 is a note off (0x138c) -> []."""
    bk = as_bank(bank if bank is not None else hd, port_state.spu_addr)
    ch = channel_state
    if velocity == 0:
        return []
    if program is None:
        prog = ch.program
    elif isinstance(program, int):
        prog = program_change(bk, ch, program)
    else:
        prog = program
    if prog is None:
        return []
    out = []
    for sp, ss, smp, vag in select(bk, prog, note, velocity):
        v = setup_voice(bk, port_state, ch, prog, sp, ss, smp, vag, note, velocity)
        r = voice_registers(v, port_state)
        r["voice"] = v
        out.append(r)
    return out


# --------------------------------------------------------------------------
# voice allocation (0x6b3c) - the free lists only; the steal paths (0x6438
# per-port limit, 0x507c reclaim, 0x6be0 active, 0x6de4 pending) are in the
# report

class FreeVoices:
    """Two free lists (.bss 0xb880 + 8*core), filled in voice order by 0x34f0
    (append), allocated from the tail (0x6b3c takes head->prev), returned to
    the head (0x62f4 / 0x5e60 insert after head). Free counts: 0xf0a8[core]."""

    def __init__(self):
        from collections import deque
        self.lists = [deque(range(24)), deque(range(24))]     # head .. tail

    def alloc(self, spu_attr):
        a = spu_attr & 0x30
        if a == 0x10:
            core = 0
        elif a == 0x20:
            core = 1
        else:                       # 0 or 0x30: the core with more free voices, ties -> core 0
            core = 1 if len(self.lists[0]) < len(self.lists[1]) else 0
        if not self.lists[core]:
            return None             # no fallback to the other core
        return core, self.lists[core].pop()

    def free(self, core, voice):
        self.lists[core].appendleft(voice)


def key_on_order(allocs):
    """0x37a4 keys the pending voices of core 0 first, then core 1, each in
    allocation order (the pending list, 0xf090 + 8*core, is walked from its
    tail and voices are inserted at its head)."""
    return sorted(range(len(allocs)), key=lambda i: (allocs[i][0], i))


# --------------------------------------------------------------------------
# The whole synthesizer: ports, channels, voice lists, stealing, the tick.
# Hardware reads (ENVX per voice, ENDX per core) come from callbacks, so a
# port can plug in its SPU2 emulation; everything written to the SPU2 is
# appended to self.log as (kind, entry, value) with libsd's entry numbers.

class HwVoice:
    """One of the 48 voice records (.bss 0xb910 + 296 * (core * 24 + n))."""

    def __init__(self, core, n):
        self.core, self.n = core, n
        self.p = None           # the Voice from setup_voice (parameters)
        self.flags = 0          # +16: 0x200 held, 0x100 sustained, 8 pending,
        #                              2/4 portamento up/down, 1 one-shot VAG
        self.prio = 0           # +20
        self.note = 0           # +67
        self.id = 255           # +68
        self.mode = 255         # +87: 0 hsyn note, 1 SE-timbre note, 255 free
        self.envx_last = 0      # +18 (only 0x507c updates it)
        self.countdown = 0      # +75: 3 at key on, ENDX ignored while > 0
        self.port = None
        self.chan = None        # (port, channel)

    @property
    def spec(self):
        return self.core | self.n << 1


class Synth:
    KON_ENVX = 1000             # 0x3880: a pending voice keys on once ENVX < 1000

    def __init__(self, envx=None, endx=None, mono=False):
        self.envx = envx or (lambda core, n: 0)
        self.endx = endx or (lambda core: 0)
        self.mono = mono
        self.voices = [[HwVoice(c, n) for n in range(24)] for c in (0, 1)]
        self.free = [list(self.voices[0]), list(self.voices[1])]   # head..tail
        self.pending = [[], []]                                     # head..tail
        self.active = [[], []]                                      # head..tail
        self.kon = [0, 0]       # 0xb8a0
        self.koff = [0, 0]      # 0xb8d8
        self.vmix = {(c, k): 0 for c in (0, 1) for k in ("l", "r", "el", "er")}
        self.noise = [0, 0]     # 0xb898
        self.reclaim_ok = True  # 0xb8d4
        self.ports = []
        for p in range(4):
            ps = PortState(0x40 if p == 0 else 0x10, 16 if p == 0 else 32)
            ps.mono = mono
            ps.bank = None
            ps.channels = [ChannelState() for _ in range(16)]
            ps.lists = [([], [], []) for _ in range(16)]   # held A, sustained B, released C
            self.ports.append(ps)
        self.log = []

    # -- bank / port setup (sceHSyn_Load 0x47c, SetVolume 0x1bc4) ------------------
    def load(self, port, hd, spu_addr):
        ps = self.ports[port]
        ps.bank = Bank(hd, spu_addr)
        ps.spu_addr = spu_addr

    # -- list helpers ----------------------------------------------------------------
    def _unlink_global(self, v):
        for lst in self.free + self.pending + self.active:
            if v in lst:
                lst.remove(v)
                return

    def _unlink_chan(self, v):
        if v.chan is None:
            return
        p, c = v.chan
        for lst in self.ports[p].lists[c]:
            if v in lst:
                lst.remove(v)

    def _release_counts(self, v):
        ps = self.ports[v.port] if v.port is not None else None
        if ps is not None and ps.voices:
            ps.voices -= 1

    def _free(self, v, unlink_chan=True):
        """0x62f4 (with the channel unlink) / 0x5e60 (without)."""
        if unlink_chan:
            self._unlink_chan(v)
        v.flags = 0
        self._release_counts(v)
        self._unlink_global(v)
        self.free[v.core].insert(0, v)
        v.id = 255
        v.mode = 255
        v.chan = None

    def _cut(self, v):
        """KOFF + ADSR2 0xc021 now + free: 0x6214, 0x6438, 0x6f44."""
        self.koff[v.core] |= 1 << v.n
        self.log.append(("param", VP_ADSR2 | v.spec, ADSR2_CUT))
        self._free(v)

    # -- MIDI (0x15cc / 0x12e8 / 0x1418) ---------------------------------------------
    def midi(self, port, data):
        i = 0
        n = len(data)
        while i < n:
            st = data[i]
            i += 1
            if not st & 0x80:
                continue                      # no running status
            if st < 0xF0:
                if i >= n:
                    return
                d1 = data[i]
                i += 1
                d2 = 0
                if (0, 1, 1, 1, 0, 0, 1)[(st - 0x80) >> 4]:
                    if i >= n:
                        return
                    d2 = data[i]
                    i += 1
                self._channel_msg(port, st, d1, d2)
            elif st == 0xF0:
                while i < n and data[i] != 0xF7:
                    i += 1
                i += 1
            elif st in (0xF9, 0xFD):
                ln = 4 if st == 0xF9 else 6
                if i + ln > n:
                    return
                self._hs_msg(port, st, data[i:i + ln])
                i += ln

    def _channel_msg(self, port, st, d1, d2):
        c = st & 0xF
        ps = self.ports[port]
        ch = ps.channels[c]
        k = st & 0xF0
        if k == 0x90 and d2:
            self.note_on(port, c, d1 & 0xFF, d2 & 0xFF, 255)
        elif k in (0x80, 0x90):
            self.note_off(port, c, d1 & 0xFF, 255)
        elif k == 0xB0:
            self.control(port, c, d1, d2)
        elif k == 0xC0:
            if ps.bank is not None:
                program_change(ps.bank, ch, d1)
        elif k == 0xE0:
            old = ch.bend
            pitch_bend(ch, d1, d2)                  # 0x878: d1 = lsb, d2 = msb
            if ch.bend != old:
                for v in self._chan_voices(port, c):
                    self._apply(v, ch, "bend")

    def _hs_msg(self, port, st, b):
        c = b[1] & 0xF
        ch = self.ports[port].channels[c]
        if st == 0xF9:
            if b[0] == 0:
                ch.note_expr = b[2]
            elif b[0] == 1:
                ch.note_pan = b[2]
            elif b[0] == 2:
                ch.note_bend = s16(b[2] + (b[3] << 7))
            return
        kind, note, vid, val, val2 = b[0], b[2], b[3], b[4], b[5]
        anynote = bool(b[1] & 0x10)
        if kind == 0x10:
            if val:
                self.note_on(port, c, note & 0x7F, val, vid)
            else:
                self.note_off(port, c, note & 0x7F, vid)
            return

        def match(v):
            return (vid == 127 or v.id == vid) and (anynote or v.note == note)
        for v in self._chan_voices(port, c):
            if v.p is None or not match(v):
                continue
            if kind == 0:                                   # 0xe80
                v.p.expr = cc_scale127(val)
                v.p.level24 = channel_level(v.p.level22, ch.volume, v.p.expr)
            elif kind == 1:                                 # 0xd64
                p = s8(val - 64)
                p = -63 if p < -63 else p
                v.p.gl, v.p.gr, v.p.phase = pan_gains(v.p.pan_base, v.p.phase, p,
                                                      v.p.prog_attr, self.mono)
            elif kind == 2 and v.p.vag is not None:          # 0xf9c
                v.p.bend = bend_offset(v.p.split, s16((val2 << 7) + val - 8192))

    def _chan_voices(self, port, c):
        a, b, cc = self.ports[port].lists[c]
        return list(a) + list(b) + list(cc)

    def control(self, port, c, num, val):
        ps = self.ports[port]
        ch = ps.channels[c]
        if num == 64:
            self._sustain(port, c, val)
            return
        if num == 120:
            for lst in ps.lists[c]:
                while lst:
                    self._cut(lst[-1])
            return
        if num == 123:
            self.all_notes_off(port, c)
            return
        if num == 121:
            self._sustain(port, c, 0)
            old = (ch.bend, ch.breath, ch.modulation, ch.volume, ch.pan, ch.expression)
            reset_all_controllers(ch)
            new = (ch.bend, ch.breath, ch.modulation, ch.volume, ch.pan, ch.expression)
            for what, o, n in zip(("bend", "breath", "modulation", "volume", "pan",
                                   "expression"), old, new):
                if o != n:
                    for v in self._chan_voices(port, c):
                        self._apply(v, ch, what)
            return
        old = (ch.volume, ch.expression, ch.pan, ch.modulation, ch.breath)
        control_change(ch, num, val)
        what = {7: "volume", 11: "expression", 10: "pan", 1: "modulation", 2: "breath"}.get(num)
        if what:
            new = (ch.volume, ch.expression, ch.pan, ch.modulation, ch.breath)
            if new != old:                    # the handlers only act on a change
                for v in self._chan_voices(port, c):
                    self._apply(v, ch, what)

    def _apply(self, v, ch, what):
        if v.p is None:
            return
        if what == "bend":
            if v.p.vag is not None:
                v.p.bend = bend_offset(v.p.split, ch.bend)
        else:
            channel_changed(v.p, ch, what)

    def _sustain(self, port, c, val):
        """0x5f94."""
        ch = self.ports[port].channels[c]
        if val:
            ch.sustain = val
            return
        if not ch.sustain:
            return
        ch.sustain = 0
        a, b, cl = self.ports[port].lists[c]
        while b:
            v = b[0]
            b.remove(v)
            if v.flags & 8:
                self._free(v, unlink_chan=False)
                continue
            v.p.lfo_pitch.key_off()
            v.p.lfo_amp.key_off()
            self.koff[v.core] |= 1 << v.n
            cl.insert(0, v)
            v.flags &= ~0x300

    def all_notes_off(self, port, c):
        """0x60d4 (CC123, sceHSyn_AllNoteOff): held and sustained voices get
        KOFF (no LFO key-off); pending ones are freed."""
        a, b, cl = self.ports[port].lists[c]
        for lst in (a, b):
            while lst:
                v = lst[-1]
                lst.remove(v)
                if v.flags & 8:
                    self._free(v, unlink_chan=False)
                    continue
                v.flags &= ~0x300
                self.koff[v.core] |= 1 << v.n
                cl.insert(0, v)

    def note_off(self, port, c, note, vid=255):
        """0x5cb4."""
        ps = self.ports[port]
        ch = ps.channels[c]
        if ch.porta_note <= 128:
            ch.porta_note = note
        a, b, cl = ps.lists[c]
        for v in list(reversed(a)):
            if v.note != note or not (v.id == vid or vid == 255):
                continue
            a.remove(v)
            v.flags &= ~0x300
            if ch.sustain:
                v.flags |= 0x100
                b.insert(0, v)
            elif v.flags & 8:
                self._free(v, unlink_chan=False)
            else:
                v.p.lfo_pitch.key_off()
                v.p.lfo_amp.key_off()
                self.koff[v.core] |= 1 << v.n
                cl.insert(0, v)

    # -- note on and allocation -----------------------------------------------------------
    def note_on(self, port, c, note, vel, vid=255):
        """0x65bc."""
        ps = self.ports[port]
        ch = ps.channels[c]
        prog = ch.program
        if prog is None or ps.bank is None:
            return
        started = False
        for sp, ss, smp, vag in select(ps.bank, prog, note, vel):
            if smp.group:
                self._group_cut(port, c, smp.group)            # 0x6f44
            prio = (smp.priority + ps.priority) & 0xFFFF
            if ps.voices >= ps.max_polyphony:
                if not self._steal_port(port, prio):           # 0x6438
                    return                                     # the whole note on ends
            v = self._alloc(smp.spu_attr)
            if v is None and self.reclaim_ok:
                self.reclaim_ok = False
                if self._reclaim():
                    v = self._alloc(smp.spu_attr)
            if v is None:
                v = self._steal_active(prio, smp.spu_attr)     # 0x6be0
            if v is None:
                v = self._steal_pending(prio, smp.spu_attr)    # 0x6de4
            if v is None:
                continue
            v.flags = 0x208
            v.prio = prio
            v.note = note
            v.id = vid
            v.mode = 0
            v.envx_last = 0
            v.port, v.chan = port, (port, c)
            ps.voices += 1
            ps.lists[c][0].insert(0, v)
            started = True
            v.p = setup_voice(ps.bank, ps, ch, prog, sp, ss, smp, vag, note, vel)
            v.flags |= v.p.flags & 6
            self.pending[v.core].insert(0, v)
        if started:
            ch.note_bend, ch.note_expr, ch.note_pan = -1, 255, 0xFF

    def _alloc(self, spu_attr):
        a = spu_attr & 0x30
        core = 0 if a == 0x10 else 1 if a == 0x20 else (
            1 if len(self.free[0]) < len(self.free[1]) else 0)
        if not self.free[core]:
            return None
        return self.free[core].pop()

    def _group_cut(self, port, c, group):
        for i, lst in enumerate(self.ports[port].lists[c]):
            for v in list(reversed(lst)):
                if v.p is None or v.p.group != group:
                    continue
                if v.flags & 8:
                    lst.remove(v)
                    self._free(v, unlink_chan=False)
                    continue
                if i < 2:
                    self.koff[v.core] |= 1 << v.n
                self.log.append(("param", VP_ADSR2 | v.spec, ADSR2_CUT))
                self._free(v)

    def _steal_port(self, port, prio):
        """0x6438: the lowest-priority released voice of the port (ties: the
        oldest), else the lowest-priority held/sustained one; only voices
        with priority <= the new note's. KOFF + ADSR2 0xc021 + free."""
        ps = self.ports[port]
        best, sel = prio, None
        for c in range(16):
            for v in ps.lists[c][2]:
                if v.prio <= best:
                    best, sel = v.prio, v
        if sel is None:
            for c in range(16):
                for lst in ps.lists[c][:2]:
                    for v in lst:
                        if v.prio <= best:
                            best, sel = v.prio, v
        if sel is None:
            return False
        self._cut(sel)
        return True

    def _reclaim(self):
        """0x507c: free every active voice whose envelope has ended."""
        freed = False
        for core in (0, 1):
            endx = self.endx(core)
            for v in list(reversed(self.active[core])):
                e = self.envx(core, v.n)
                v.envx_last = e
                if v.flags & 0x300:
                    if v.countdown:
                        v.countdown = v.countdown - 1 if endx >> v.n & 1 else 0
                        v.envx_last = 0x7FFF
                        continue
                    if not endx >> v.n & 1 or e:
                        continue
                elif e:
                    continue
                self._free(v)
                freed = True
        return freed

    def _steal_active(self, prio, spu_attr):
        """0x6be0: over the active lists (core 1 first, oldest first): prefer
        released (state 0) over sustained (0x100) over held (0x200), then the
        lowest priority, then the lowest remembered ENVX (+18); only voices
        with priority <= the new note's. KOFF (normal release); the new note
        waits in the pending list until ENVX < 1000."""
        mask = spu_attr & 0x30 or 0x30
        state, bprio, benv, sel = 0x300, prio, 0xFFFF, None
        for core in (1, 0):
            if not mask & (0x10 << core):
                continue
            for v in reversed(self.active[core]):
                if v.mode != 0:
                    continue
                s = v.flags & 0x300
                if s < state:
                    if v.prio <= prio:
                        bprio, state, benv, sel = v.prio, s, v.envx_last, v
                elif s == state:
                    if v.prio == bprio:
                        if v.envx_last < benv:
                            benv, sel = v.envx_last, v
                    elif v.prio < bprio:
                        bprio, benv, sel = v.prio, v.envx_last, v
        if sel is None:
            return None
        self.koff[sel.core] |= 1 << sel.n
        self._release_counts(sel)
        self._unlink_chan(sel)
        self._unlink_global(sel)
        sel.flags, sel.id, sel.mode, sel.chan = 0, 255, 255, None
        return sel

    def _steal_pending(self, prio, spu_attr):
        """0x6de4: the lowest-priority pending voice (priority <= new)."""
        mask = spu_attr & 0x30 or 0x30
        th, sel = prio + 1, None
        for core in (1, 0):
            if not mask & (0x10 << core):
                continue
            for v in reversed(self.pending[core]):
                if v.mode == 0 and v.prio < th:
                    th, sel = v.prio, v
        if sel is None:
            return None
        self._release_counts(sel)
        self._unlink_chan(sel)
        self._unlink_global(sel)
        sel.flags, sel.id, sel.mode, sel.chan = 0, 255, 255, None
        return sel

    # -- the tick after MIDI: 0x37a4 then 0x4dc0 -------------------------------------------
    def _key_on(self, v):
        """0x3f50."""
        p = v.p
        ps = self.ports[v.port]
        sp = v.spec
        p.pitch_cache = None
        p.vol_cache = None
        vol = update_volume(p, ps)
        self.log.append(("param", VP_VOLL | sp, vol[0]))
        self.log.append(("param", VP_VOLR | sp, vol[1]))
        bit = 1 << v.n
        if p.vag is not None:
            self.noise[v.core] &= ~bit
            self.log.append(("param", VP_PITCH | sp, update_pitch(p)))
            self.log.append(("addr", VA_SSA | sp, (ps.spu_addr + p.vag.offset) & M32))
            self.log.append(("param", VP_ADSR1 | sp, adsr1(p.note, p.sample)))
            self.log.append(("param", VP_ADSR2 | sp, adsr2(p.note, p.sample)))
            if not p.vag.loop & 1:
                v.flags |= 1
        else:
            self.log.append(("core", v.core | 8, min(p.note, 63)))     # SD_C_NOISE_CLK
            self.noise[v.core] |= bit
        self.log.append(("switch", S_NON | v.core, self.noise[v.core]))
        for k, b in (("l", 1), ("r", 2), ("el", 4), ("er", 8)):
            if p.spu_attr & b:
                self.vmix[(v.core, k)] |= bit
            else:
                self.vmix[(v.core, k)] &= ~bit
        for k, code in (("l", S_VMIXL), ("r", S_VMIXR), ("el", S_VMIXEL), ("er", S_VMIXER)):
            self.log.append(("switch", code | v.core, self.vmix[(v.core, k)]))
        self.kon[v.core] |= bit
        v.countdown = 3
        v.flags &= ~8

    def tick_voices(self):
        """0x37a4 + 0x4dc0 (the part of sceHSyn_ATick after the MIDI input)."""
        for core in (0, 1):
            if self.koff[core]:
                self.log.append(("switch", S_KOFF | core, self.koff[core]))
                self.koff[core] = 0
        for core in (0, 1):
            for v in list(reversed(self.pending[core])):
                if self.envx(core, v.n) < self.KON_ENVX:
                    self._key_on(v)
                    self.pending[core].remove(v)
                    self.active[core].insert(0, v)
            if self.kon[core]:
                self.log.append(("switch", S_KON | core, self.kon[core]))
                self.kon[core] = 0
        for core in (0, 1):
            endx = self.endx(core)
            for v in list(reversed(self.active[core])):
                if self.reclaim_ok:
                    e = self.envx(core, v.n)
                    if v.flags & 0x300:
                        if v.countdown:
                            v.countdown = v.countdown - 1 if endx >> v.n & 1 else 0
                        elif endx >> v.n & 1 and not e:
                            self._free(v)
                            continue
                    elif not e:
                        self._free(v)
                        continue
                ps = self.ports[v.port]
                o = tick_voice(v.p, ps)
                if "pitch" in o:
                    self.log.append(("param", VP_PITCH | v.spec, o["pitch"]))
                if "vol" in o:
                    self.log.append(("param", VP_VOLL | v.spec, o["vol"][0]))
                    self.log.append(("param", VP_VOLR | v.spec, o["vol"][1]))
        self.reclaim_ok = True

    def tick(self, inputs=None):
        """sceHSyn_ATick: `inputs` = {port: MIDI bytes}, then the voices."""
        for port in range(4):
            if inputs and inputs.get(port):
                self.midi(port, inputs[port])
        self.tick_voices()
