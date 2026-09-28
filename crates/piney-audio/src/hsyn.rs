//! MODHSYN.IRX, Sony's "hardware synthesizer" (CSL hsyn, "PsIImodhsyn
//! 2420"): MIDI messages on four ports become SPU2 voices. Addresses are
//! MODHSYN.IRX module offsets (LIBSD ones marked). This follows the Python
//! model `tools/hsyn.py` line for line; that model produced the same libsd
//! calls as the module run in `tools/iopemu.py` on every case tried (every
//! sound effect, 14,260 note ons over 8 banks, controllers, LFOs, stealing),
//! and `tests/hsyn.rs` checks this port against the module's own writes.
//!
//! Per tick (`sceHSyn_ATick`, 0x182c): each port's MIDI bytes are parsed
//! (0x15cc: no running status, 8n with one data byte, F9 / FD the SE
//! messages); then KOFF per core, the pending voices keyed on (VOLL, VOLR,
//! PITCH, SSA, ADSR1, ADSR2, the mix switches, then KON per core), then every
//! sounding voice updated (LFOs, portamento, pitch and volume when they
//! change) or freed when its envelope has ended.
//!
//! A note on (0x65bc) sounds every sample of every split of the channel's
//! program whose key and velocity ranges hold it. Pitch: `sceSdNote2Pitch`
//! from the sample's base note, times the VAG's rate over 48 kHz. Volume:
//! program x split x sample x velocity curve x crossfade, x CC7 x CC11, x the
//! port volume (0..256), split by a linear balance pan.

use std::sync::Arc;

use piney_data::sound::hd::{Program, Sample, Split, Vag};
use piney_data::sound::{Bank, Hd};

use crate::spu::Regs;

const M32: i64 = 0xffff_ffff;

fn s8(x: i64) -> i64 {
    (x as u8) as i8 as i64
}

fn s16(x: i64) -> i64 {
    (x as u16) as i16 as i64
}

fn s32(x: i64) -> i64 {
    (x as u32) as i32 as i64
}

fn tdiv(a: i64, b: i64) -> i64 {
    if b == 0 { 0 } else { a / b }
}

/// `if (x < 0) x += (1 << n) - 1; x >>= n`: the compiler's x / 2^n.
fn sra_trunc(x: i64, n: u32) -> i64 {
    let mut x = s32(x);
    if x < 0 {
        x += (1 << n) - 1;
    }
    x >> n
}

// LIBSD sceSdNote2Pitch (LIBSD 0x3050); tables at LIBSD .data 0x4860, 0x4878.
const N2P_SEMI: [i64; 12] =
    [0x8000, 0x879C, 0x8FAC, 0x9837, 0xA145, 0xAADC, 0xB504, 0xBFC8, 0xCB2F, 0xD744, 0xE411, 0xF1A1];
#[rustfmt::skip]
const N2P_FINE: [i64; 128] = [
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
    0x871F, 0x872E, 0x873E, 0x874E, 0x875D, 0x876D, 0x877D, 0x878C,
];

/// `sceSdNote2Pitch(center_note, center_fine, note, fine)` (LIBSD 0x3050):
/// the SPU pitch, 0x1000 when `note` is `center_note`; `fine` in 1/128
/// semitone.
pub fn sd_note2pitch(center_note: i64, center_fine: i64, note: i64, fine: i64) -> i64 {
    let mut total = s16(fine) + (center_fine & 0xffff);
    let carry = tdiv(total, 128);
    let semi = s16((note & 0xffff) + carry - (center_note & 0xffff));
    total -= carry << 7;
    let octs = tdiv(semi, 12);
    let mut shift = octs - 2;
    let mut idx = semi - octs * 12;
    if s16(idx) < 0 || (idx == 0 && total < 0) {
        idx += 12;
        shift = octs - 3;
    }
    if total < 0 {
        idx = idx - 1 + carry;
        total += (carry + 1) << 7;
    }
    let semi_ratio = N2P_SEMI.get(s16(idx).clamp(0, 11) as usize).copied().unwrap_or(0x8000);
    let fine_ratio = N2P_FINE.get(total.clamp(0, 127) as usize).copied().unwrap_or(0x8000);
    let mut p = (semi_ratio * fine_ratio) >> 16;
    let shift = s16(shift);
    if shift < 0 {
        let n = -shift;
        p = (p + (1 << (n - 1))) >> n;
    }
    p & 0xffff
}

/// sceHSyn_Init (0x50): the tick in 1/256 ms, `4167 * 256 / 1000`.
const TICK_256MS: i64 = (((4167i64 << 8) * 0x10624DD3) >> 32) >> 6;

/// The LFO sine table (.data 0xb690): one cycle in 200 steps.
const SINE_HALF: [i64; 100] = [
    0, 1029, 2057, 3083, 4106, 5125, 6139, 7147, 8148, 9141, 10125, 11099, 12062, 13013, 13951, 14875, 15785, 16679,
    17557, 18417, 19259, 20083, 20886, 21669, 22430, 23169, 23886, 24578, 25247, 25891, 26509, 27100, 27666, 28203,
    28713, 29195, 29648, 30072, 30465, 30829, 31163, 31465, 31737, 31977, 32186, 32363, 32508, 32621, 32702, 32750,
    32767, 32750, 32702, 32621, 32508, 32363, 32186, 31977, 31737, 31465, 31163, 30829, 30465, 30072, 29648, 29195,
    28713, 28203, 27666, 27100, 26509, 25891, 25247, 24578, 23886, 23169, 22430, 21669, 20886, 20083, 19259, 18417,
    17557, 16679, 15785, 14875, 13951, 13013, 12062, 11099, 10125, 9141, 8148, 7147, 6139, 5125, 4106, 3083, 2057,
    1029,
];

fn sine200(i: usize) -> i64 {
    if i < 100 { SINE_HALF[i] } else { -SINE_HALF[i - 100] }
}

// libsd entries: voice = core | voice << 1.
const VP_VOLL: u16 = 0x000;
const VP_VOLR: u16 = 0x100;
const VP_PITCH: u16 = 0x200;
const VP_ADSR1: u16 = 0x300;
const VP_ADSR2: u16 = 0x400;
const VP_ENVX: u16 = 0x500;
const VA_SSA: u16 = 0x2040;
const S_NON: u16 = 0x1400;
const S_KON: u16 = 0x1500;
const S_KOFF: u16 = 0x1600;
const S_ENDX: u16 = 0x1700;
const S_VMIX: [u16; 4] = [0x1800, 0x1a00, 0x1900, 0x1b00];
/// Written to ADSR2 by all-sound-off, the port steal and exclusive groups.
const ADSR2_CUT: u16 = 0xc021;
/// 0x3880: a pending voice keys on once ENVX < 1000.
const KON_ENVX: u16 = 1000;

/// A bank as sceHSyn_Load (0x47c) sees it, with the tables' highest slots.
pub struct HsynBank {
    pub bank: Arc<Bank>,
    pub spu_addr: u32,
    prog_max: i64,
    sset_max: i64,
    smpl_max: i64,
    vagi_max: i64,
}

impl HsynBank {
    fn new(bank: Arc<Bank>, spu_addr: u32) -> HsynBank {
        let hd: &Hd = &bank.hd;
        let top = |n: usize| n as i64 - 1;
        HsynBank {
            prog_max: top(hd.programs.len()),
            sset_max: top(hd.sample_sets.len()),
            smpl_max: top(hd.samples.len()),
            vagi_max: top(hd.vags.len()),
            bank,
            spu_addr,
        }
    }

    fn program(&self, p: usize) -> Option<&Program> {
        self.bank.hd.programs.get(p)?.as_ref()
    }
}

/// The 56-byte channel (system+4 + 56 * ch).
#[derive(Clone, Debug)]
struct Channel {
    program: Option<usize>,
    bank: u8,
    porta_time: i64,
    bend: i64,
    note_bend: i64,
    volume: i64,
    expression: i64,
    breath: i64,
    modulation: i64,
    note_expr: i64,
    note_pan: i64,
    pan: i64,
    sustain: i64,
    porta_note: i64,
    porta_ctrl: i64,
    nrpn: [i64; 3],
}

impl Channel {
    fn new() -> Channel {
        Channel {
            program: None,
            bank: 0,
            porta_time: 0,
            bend: 0,
            note_bend: -1,
            volume: 128,
            expression: 128,
            breath: 0,
            modulation: 0,
            note_expr: 255,
            note_pan: 0xff,
            pan: 0,
            sustain: 0,
            porta_note: 255,
            porta_ctrl: 0,
            nrpn: [255; 3],
        }
    }
}

/// 0x9f8 and friends: `((v & 0xff) << 7) / 127`.
fn cc_scale127(v: i64) -> i64 {
    tdiv((v & 0xff) << 7, 127) & 0xff
}

fn pitch_bend(ch: &mut Channel, lsb: i64, msb: i64) {
    ch.bend = s16(((msb & 0xff) << 7) + (lsb & 0xff) - 8192);
}

/// 0x10b0: the channel side of CC121 / sceHSyn_ResetAllControler.
fn reset_controllers(ch: &mut Channel) {
    ch.sustain = 0;
    pitch_bend(ch, 0, 64);
    ch.breath = 0;
    ch.modulation = 0;
    ch.volume = cc_scale127(127);
    ch.pan = 0;
    ch.expression = cc_scale127(127);
    ch.porta_note = 255;
}

fn sq127(v: i64) -> i64 {
    let r = (v * v) / 127;
    if r != 0 { r & 0xff } else { 1 }
}

/// 0x8fe4: the velocity curve (the game sets no velocity maps).
fn velocity_map(vel: i64, curve: u8) -> i64 {
    let mut vel = vel & 0xff;
    if vel == 0 {
        vel = 1;
    }
    let mut f = curve & 15;
    if f >= 11 {
        f = 0;
    }
    if (6..=9).contains(&f) {
        f -= 6;
    }
    (match f {
        0 => vel,
        1 => 128 - vel,
        2 => sq127(vel),
        3 => 128 - sq127(vel),
        4 => 128 - sq127((128 - vel) & 0xff),
        5 => sq127((128 - vel) & 0xff),
        _ => vel,
    }) & 0xff
}

fn vf_sq(v: i64, c: i64) -> (i64, i64) {
    (tdiv((v - 1) << 15, 126), tdiv((c - 1) << 15, 126))
}

/// 0x8e5c's curves.
fn vel_follow_curve(f: u8, v: i64, c: i64) -> i64 {
    match f {
        0 => tdiv((v - c) << 16, 126),
        1 => tdiv((c - v) << 16, 126),
        2 => {
            let (a, b) = vf_sq(v, c);
            sra_trunc(a * a - b * b, 14)
        }
        3 => {
            let (a, b) = vf_sq(v, c);
            (0x10000 - sra_trunc(a * a, 14)) - (0x10000 - sra_trunc(b * b, 14))
        }
        4 => {
            let (a, b) = vf_sq(v, c);
            (0x10000 - sra_trunc((0x10000 - a) * (0x10000 - a), 14))
                - (0x10000 - sra_trunc((0x10000 - b) * (0x10000 - b), 14))
        }
        5 => {
            let (a, b) = vf_sq(v, c);
            sra_trunc((0x10000 - a) * (0x10000 - a), 14) - sra_trunc((0x10000 - b) * (0x10000 - b), 14)
        }
        6 | 7 => {
            if v == c {
                return 0;
            }
            let d = if c < v { 127 - c } else { c - 1 };
            tdiv(if f == 7 { c - v } else { v - c } << 16, d)
        }
        8 => {
            if v == c {
                return 0;
            }
            let x = (v - c) << 15;
            if c < v {
                let q = tdiv(x, 127 - c);
                sra_trunc(q * q, 14)
            } else {
                let q = tdiv(x, c - 1);
                -sra_trunc(q * q, 14)
            }
        }
        9 => {
            if v == c {
                return 0;
            }
            let x = (v - c) << 15;
            if c < v {
                let r = 0x8000 - tdiv(x, 127 - c);
                0x10000 - sra_trunc(r * r, 14)
            } else {
                let r = tdiv(x, c - 1) + 0x8000;
                sra_trunc(r * r, 14) - 0x10000
            }
        }
        _ => v & 0xff,
    }
}

/// 0x4884: a velocity-follow offset.
fn velocity_follow(vel: i64, amount: i8, center: u8, curve: u8) -> i64 {
    let mut vel = vel & 0xff;
    if vel == 0 {
        vel = 1;
    }
    let mut f = curve & 15;
    if f >= 11 {
        f = 0;
    }
    let r = vel_follow_curve(f, vel, center as i64);
    sra_trunc(s32(r * amount as i64), 16)
}

/// 0x39ec: a key-follow step on one ADSR field.
fn kf_field(value: u16, amount: i8, key_delta: i64, mask: i64, shift: u32) -> u16 {
    let d = tdiv(key_delta * amount as i64, 12);
    if d == 0 {
        return value;
    }
    let f = ((value as i64) >> shift) & mask;
    let f = (f + d).clamp(0, mask);
    (((value as i64) & !(mask << shift)) | (f << shift)) as u16
}

/// 0x3a74: ADSR1 with key follow on Ar, Dr and Sl.
pub fn adsr1(note: i64, s: &Sample) -> u16 {
    let mut v = s.adsr1;
    if s.kf_ar != 0 {
        v = kf_field(v, s.kf_ar, note - s.kf_ar_center as i64, 0x7f, 8);
    }
    if s.kf_dr != 0 {
        v = kf_field(v, s.kf_dr, note - s.kf_dr_center as i64, 0x0f, 4);
    }
    if s.kf_sl != 0 {
        v = kf_field(v, s.kf_sl, note - s.kf_sl_center as i64, 0x0f, 0);
    }
    v
}

/// 0x3b18: ADSR2 with key follow on Sr and Rr.
pub fn adsr2(note: i64, s: &Sample) -> u16 {
    let mut v = s.adsr2;
    if s.kf_sr != 0 {
        v = kf_field(v, s.kf_sr, note - s.kf_sr_center as i64, 0x7f, 6);
    }
    if s.kf_rr != 0 {
        v = kf_field(v, s.kf_rr, note - s.kf_rr_center as i64, 0x1f, 0);
    }
    v
}

/// 0x4c68: the velocity and key crossfades, 16.16.
fn crossfade(smp: &Sample, sp: &Split, vel: i64, note: i64) -> i64 {
    let mut g: i64 = 0x10000;
    let xf = smp.vel_crossfade as i64;
    if vel >= xf {
        if smp.vel_high & 0x80 != 0 {
            let mut h = (smp.vel_high & 0x7f) as i64;
            if h != xf {
                h += 1;
                let d = (h - xf) & M32;
                if d != 0 {
                    g = (((h - vel) << 16) & M32) / d;
                }
            }
        }
    } else if smp.vel_low & 0x80 != 0 {
        let mut lo = (smp.vel_low & 0x7f) as i64;
        if lo != xf {
            lo -= 1;
            let d = (xf - lo) & M32;
            if d != 0 {
                g = (((vel - lo) << 16) & M32) / d;
            }
        }
    }
    let xf = sp.key_crossfade as i64;
    if note >= xf {
        if sp.key_high & 0x80 != 0 {
            let mut h = (sp.key_high & 0x7f) as i64;
            if h != xf {
                h += 1;
                let d = (h - xf) & M32;
                if d != 0 {
                    g = ((g * (h - note)) & M32) / d;
                }
            }
        }
    } else if sp.key_low & 0x80 != 0 {
        let mut lo = (sp.key_low & 0x7f) as i64;
        if lo != xf {
            lo -= 1;
            let d = (xf - lo) & M32;
            if d != 0 {
                g = ((g * (note - lo)) & M32) / d;
            }
        }
    }
    g
}

/// 0x3b94 / 0x4a40: the voice's pan base and phase flag (None for an
/// attr-1 program, which takes it from 0x4b14).
fn pan_base(prog: &Program, sp: &Split, smp: &Sample, note: i64) -> (i64, Option<i64>) {
    let kf = tdiv(
        (note - sp.key_follow_pan_center as i64) * sp.key_follow_pan as i64
            + (note - prog.key_follow_pan_center as i64) * prog.key_follow_pan as i64,
        12,
    );
    if prog.attr & 1 != 0 {
        let mut a = prog.pan as i64 - 128 + sp.pan as i64 + smp.pan as i64 + kf;
        while a >= 128 {
            a -= 255;
        }
        while a < -128 {
            a += 255;
        }
        return (a, None);
    }
    let a = (prog.pan as i64).abs() - 128 + (sp.pan as i64).abs() + (smp.pan as i64).abs() + kf;
    (a.clamp(0, 127), Some(1))
}

/// 0x4b14: (gain L, gain R, phase), gains 0..128, a linear balance law.
fn pan_gains(base: i64, phase: i64, chan_pan: i64, prog_attr: i64, mono: bool) -> (i64, i64, i64) {
    let (mut gl, mut gr) = (128, 128);
    if mono {
        return (gl, gr, phase);
    }
    let p = s8(chan_pan).clamp(-63, 63);
    let mut phase = phase;
    let a;
    if prog_attr & 1 != 0 {
        let mut x = s16(base);
        let mut ph = 1;
        if x < 0 {
            x = -x;
            ph = 0;
        }
        x += p;
        if x < 0 {
            ph ^= 1;
            x = -x;
        } else if x >= 128 {
            ph ^= 1;
            x = 256 - x;
        }
        phase = ph;
        a = x - 64;
    } else {
        a = p + s16(base) - 64;
    }
    if a > 0 {
        gl = if a >= 63 { 0 } else { tdiv((63 - a) << 7, 63) };
    } else if a < 0 {
        gr = if a <= -63 { 0 } else { tdiv((a + 63) << 7, 63) };
    }
    (gl, gr, phase)
}

fn base_level(prog: &Program, sp: &Split, smp: &Sample, vel_mapped: i64, xfade: i64) -> i64 {
    let t = prog.volume as i64 * sp.volume as i64 * smp.volume as i64 * vel_mapped;
    let t = sra_trunc(t, 14);
    (((t * xfade) & M32) >> 16) & 0xffff
}

fn channel_level(level22: i64, ch_volume: i64, expression: i64) -> i64 {
    (((ch_volume * expression * level22) & M32) >> 14) & 0xffff
}

/// 0x3ec8: `rate * 65536 / 48000`, 16.16.
fn rate_ratio(rate: u16) -> i64 {
    (((((rate as u64) << 16) * 0x057619F1) >> 32) >> 10) as i64
}

/// 0x45f0: the bend in 1/128 semitone.
fn bend_offset(sp: &Split, bend: i64) -> i64 {
    let rng = if bend >= 0 { sp.bend_high } else { sp.bend_low } as i64;
    sra_trunc(s32(rng * bend), 13)
}

/// 0x43e8: the PITCH register.
pub fn pitch_register(prog: &Program, sp: &Split, smp: &Sample, vag: &Vag, note: i64, fine_offset: i64) -> u16 {
    let mut key = note + prog.transpose as i64 + sp.transpose as i64;
    let mut fine = fine_offset + prog.detune as i64 + smp.detune as i64 + sp.detune as i64;
    if fine < 0 {
        let q = (fine + 127) >> 7;
        key += q;
        fine -= q << 7;
        if fine != 0 {
            fine += 128;
            key -= 1;
        }
    }
    let p = sd_note2pitch(smp.base_note as i64, 0, key & 0xffff, s16(fine));
    let p = ((p * rate_ratio(vag.rate)) & M32) >> 16;
    p.min(0x3fff) as u16
}

/// 0x9b00, the LFOs' random numbers.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> i64 {
        self.0 = self.0.wrapping_mul(0x41C64E6D).wrapping_add(12345);
        (self.0 as i32 >> 16) as i64
    }
}

/// The 40-byte LFO (init 0x93b4, tick 0x91ac, key off 0x96f8). State: 0
/// off, 1 delay, 2 fade in, 3 run, 4 release delay, 5 fade out, 6 done.
#[derive(Clone, Debug, Default)]
struct Lfo {
    state: u8,
    keyoff: bool,
    out: i64,
    wave: u8,
    count: i64,
    delay: i64,
    fade: i64,
    fade_inc: i64,
    phase: i64,
    inc: i64,
}

impl Lfo {
    fn new(prog: &Program, attr: u8, delay_ms: u16, fade_ms: u16, second: bool, rng: &mut Rng) -> Lfo {
        let mut l = Lfo::default();
        if attr & 0x77 == 0 {
            return l;
        }
        let w = if second { prog.lfo_wave2 } else { prog.lfo_wave };
        if w & 0x80 != 0 || w == 0 || w >= 7 {
            return l;
        }
        l.wave = w;
        let tick = TICK_256MS;
        l.delay = (((delay_ms as i64) << 8) + (tick >> 1)) / tick;
        let mut f = (((fade_ms as i64) << 8) + (tick >> 1)) / tick;
        if f != 0 {
            f = 0x10000 / f;
            if f == 0 {
                f = 1;
            }
        }
        l.fade_inc = f;
        let (rnd, ph, cyc) = if second {
            (prog.lfo_random2, prog.lfo_phase2, prog.lfo_cycle2)
        } else {
            (prog.lfo_random, prog.lfo_phase, prog.lfo_cycle)
        };
        let r = (rnd as i64) << 8;
        let mut r = sra_trunc(s32(r * rng.next()), 16);
        r += (ph as i64) << 8;
        if r < 0 {
            r += 0x10000;
        } else if r > 0xffff {
            r -= 0x10000;
        }
        l.phase = r;
        let per = (((cyc as i64) << 8) + (tick >> 1)) / tick;
        l.inc = if per != 0 { 0x10000 / per } else { 0 };
        if attr & 0x66 != 0 {
            l.keyoff = true;
        }
        if attr & 0x55 != 0 {
            l.start();
        }
        l
    }

    fn start(&mut self) {
        if self.delay != 0 {
            self.state = 1;
            self.count = self.delay;
        } else if self.fade_inc != 0 {
            self.state = 2;
            self.fade = 0;
        } else {
            self.state = 3;
        }
    }

    fn wave_value(&self, wrapped: bool, rng: &mut Rng) -> i64 {
        let mut p = self.phase;
        match self.wave {
            1 => s16(((p - 32768) << 15) >> 16),
            2 => s16(((0xffff - p - 32768) << 15) >> 16),
            3 => {
                if p > 0xbfff {
                    p -= 0x10000;
                } else if p > 0x4000 {
                    p = 0x8000 - p;
                }
                s16(p)
            }
            4 => {
                if p > 32767 {
                    16384
                } else {
                    -16384
                }
            }
            5 => {
                if wrapped {
                    s16(rng.next()) >> 2
                } else {
                    self.out
                }
            }
            _ => {
                let x = p * 200;
                let i = (x >> 16) as usize;
                let j = if i + 1 >= 200 { 0 } else { i + 1 };
                let a = sine200(i.min(199));
                let v = s32(a + (((sine200(j) - a) * (x & 0xffff)) >> 16));
                s16(((v + if v < 0 { 1 } else { 0 }) << 15) >> 16)
            }
        }
    }

    fn tick(&mut self, rng: &mut Rng) -> i64 {
        let s = self.state;
        if s == 0 || s >= 6 {
            self.out = 0;
            return 0;
        }
        let mut v = 0;
        if s >= 2 {
            self.phase += self.inc;
            let mut wrapped = false;
            if self.phase > 0xffff {
                self.phase -= 0x10000;
                wrapped = true;
            }
            v = self.wave_value(wrapped, rng);
        }
        match s {
            1 => {
                self.count -= 1;
                if self.count & 0xffff != 0 {
                    self.out = 0;
                    return 0;
                }
                if self.fade_inc != 0 {
                    self.state = 2;
                    self.fade = 0;
                } else {
                    self.state = 3;
                }
                v = 0;
            }
            2 => {
                let t = v * self.fade;
                v = s16((if t >= 0 { s32(t) } else { t + 0xffff }) >> 16);
                self.fade += self.fade_inc;
                if self.fade > 0xffff {
                    self.state = 3;
                }
            }
            4 => {
                self.count -= 1;
                if self.count & 0xffff == 0 {
                    if self.fade_inc != 0 {
                        self.fade = 0x10000;
                        self.state = 5;
                    } else {
                        self.state = 6;
                    }
                }
            }
            5 => {
                let t = v * self.fade;
                v = s16((if t >= 0 { t } else { t + 0xffff }) >> 16);
                self.fade -= self.fade_inc;
                if self.fade <= 0 {
                    self.state = 6;
                }
            }
            _ => {}
        }
        self.out = s16(v);
        self.out
    }

    fn key_off(&mut self) {
        if !self.keyoff {
            return;
        }
        match self.state {
            1 => self.state = 6,
            0 => self.start(),
            2 | 3 => {
                if self.delay != 0 {
                    self.state = 4;
                    self.count = self.delay;
                } else if self.fade_inc != 0 {
                    self.state = 5;
                    self.fade = 0x10000;
                } else {
                    self.state = 6;
                }
            }
            _ => self.state = 6,
        }
    }
}

/// A sounding note's parameters (0x3b94).
#[derive(Clone)]
struct Params {
    bank: Arc<Bank>,
    prog: usize,
    split: Split,
    sample: Sample,
    vag: Option<Vag>,
    note: i64,
    group: u8,
    spu_attr: u8,
    lfo_pitch: Lfo,
    lfo_amp: Lfo,
    pdepth_base: (i64, i64),
    adepth_base: (i64, i64),
    pdepth: (i64, i64),
    adepth: (i64, i64),
    level22: i64,
    pan_base: i64,
    phase: i64,
    prog_attr: i64,
    gl: i64,
    gr: i64,
    expr: i64,
    level24: i64,
    flags: u32,
    porta: i64,
    porta_step: i64,
    pitch_cache: Option<i64>,
    vol_cache: Option<(i64, i64)>,
    bend: i64,
}

impl Params {
    fn program(&self) -> &Program {
        self.bank.hd.programs[self.prog].as_ref().expect("a voice's program")
    }
}

fn set_modulation(v: &mut Params, cc1: i64) {
    let p = v.program();
    let a = (sra_trunc(cc1 * p.lfo_midi_pitch_depth as i64, 7) + v.pdepth_base.0).clamp(-32768, 32767);
    let b = (sra_trunc(cc1 * p.lfo_midi_pitch_depth2 as i64, 7) + v.pdepth_base.1).clamp(-32768, 32767);
    v.pdepth = (a, b);
}

fn set_breath(v: &mut Params, cc2: i64) {
    let p = v.program();
    let a = (sra_trunc(cc2 * p.lfo_midi_amp_depth as i64, 7) + v.adepth_base.0).clamp(-128, 127);
    let b = (sra_trunc(cc2 * p.lfo_midi_amp_depth2 as i64, 7) + v.adepth_base.1).clamp(-128, 127);
    v.adepth = (a, b);
}

fn lfo_pitch_part(lfo: i64, pos: i64, neg: i64) -> i64 {
    let d = if lfo >= 0 { pos } else { neg };
    sra_trunc(s32(d * lfo), 14)
}

/// 0x43e8 with its cache: the new PITCH, or None when nothing changed.
fn update_pitch(v: &mut Params) -> Option<u16> {
    let vag = v.vag?;
    let fine = lfo_pitch_part(v.lfo_pitch.out, v.pdepth.0, v.pdepth.1) + v.bend + v.porta;
    if Some(fine) == v.pitch_cache {
        return None;
    }
    v.pitch_cache = Some(fine);
    Some(pitch_register(v.program(), &v.split, &v.sample, &vag, v.note, fine))
}

/// 0x4644 with its cache: (VOLL, VOLR), or None when unchanged.
fn update_volume(v: &mut Params, port_volume: i64) -> Option<(u16, u16)> {
    let a = sra_trunc(s32(v.level24 * port_volume), 8);
    let lfo = v.lfo_amp.out;
    let d = if lfo >= 0 { v.adepth.0 } else { v.adepth.1 };
    let mut a = sra_trunc(s32(a * (sra_trunc(lfo * d, 7) + 16384)), 14);
    if a < 0 {
        a = 0;
    }
    let l = (((v.gl * a) & M32) >> 7).min(0x3fff);
    let r = (((v.gr * a) & M32) >> 7).min(0x3fff);
    if Some((l, r)) == v.vol_cache {
        return None;
    }
    v.vol_cache = Some((l, r));
    if v.phase != 0 { Some((l as u16, r as u16)) } else { Some((((-l) & 0x7fff) as u16, ((-r) & 0x7fff) as u16)) }
}

/// One 0x4dc0 pass over a sounding voice: (PITCH, (VOLL, VOLR)) to write.
fn tick_params(v: &mut Params, port_volume: i64, rng: &mut Rng) -> (Option<u16>, Option<(u16, u16)>) {
    let mut pitch = None;
    if v.vag.is_some() {
        if v.flags & 6 != 0 {
            if v.flags & 2 != 0 {
                v.porta += v.porta_step;
                if v.porta >= 0 {
                    v.porta = 0;
                    v.flags &= !2;
                }
            } else {
                v.porta -= v.porta_step;
                if v.porta <= 0 {
                    v.porta = 0;
                    v.flags &= !4;
                }
            }
        }
        v.lfo_pitch.tick(rng);
        pitch = update_pitch(v);
    }
    v.lfo_amp.tick(rng);
    (pitch, update_volume(v, port_volume))
}

/// One of the 48 voice records (.bss 0xb910 + 296 * index).
#[derive(Clone, Default)]
struct HwVoice {
    p: Option<Box<Params>>,
    /// 0x200 held, 0x100 sustained, 8 pending, 2 / 4 portamento, 1 one-shot.
    flags: u32,
    prio: i64,
    note: i64,
    id: i64,
    /// 0 hsyn note, 255 free.
    mode: u8,
    envx_last: i64,
    countdown: i64,
    port: usize,
    chan: Option<(usize, usize)>,
}

fn spec(v: usize) -> u16 {
    ((v / 24) | (v % 24) << 1) as u16
}

struct Port {
    priority: i64,
    max_polyphony: i64,
    volume: i64,
    bank: Option<HsynBank>,
    channels: Vec<Channel>,
    /// Per channel: held, sustained, released (voice indices, head first).
    lists: Vec<[Vec<usize>; 3]>,
    voices: i64,
    input: Vec<u8>,
}

impl Port {
    fn new(priority: i64, max_polyphony: i64) -> Port {
        Port {
            priority,
            max_polyphony,
            volume: 256,
            bank: None,
            channels: vec![Channel::new(); 16],
            lists: vec![[Vec::new(), Vec::new(), Vec::new()]; 16],
            voices: 0,
            input: Vec::new(),
        }
    }
}

/// The synthesizer: four ports of 16 channels over 48 voices.
pub struct Synth {
    voices: Vec<HwVoice>,
    /// Per core, head first: free, pending, active.
    free: [Vec<usize>; 2],
    pending: [Vec<usize>; 2],
    active: [Vec<usize>; 2],
    kon: [u32; 2],
    koff: [u32; 2],
    /// VMIXL, VMIXR, VMIXEL, VMIXER per core.
    vmix: [[u32; 4]; 2],
    noise: [u32; 2],
    reclaim_ok: bool,
    ports: Vec<Port>,
    /// `sceHSyn_SetOutputMode(0)`: no pan.
    pub mono: bool,
    rng: Rng,
}

impl Default for Synth {
    fn default() -> Self {
        Synth::new()
    }
}

impl Synth {
    /// After sceHSyn_Init: every voice free, ports at volume 256.
    pub fn new() -> Synth {
        let mut voices = vec![HwVoice::default(); 48];
        for v in &mut voices {
            v.id = 255;
            v.mode = 255;
        }
        Synth {
            voices,
            free: [(0..24).collect(), (24..48).collect()],
            pending: [Vec::new(), Vec::new()],
            active: [Vec::new(), Vec::new()],
            kon: [0; 2],
            koff: [0; 2],
            vmix: [[0; 4]; 2],
            noise: [0; 2],
            reclaim_ok: true,
            ports: (0..4).map(|p| if p == 0 { Port::new(0x40, 16) } else { Port::new(0x10, 32) }).collect(),
            mono: false,
            rng: Rng(0x02CE_E9D7),
        }
    }

    /// `sceHSyn_Load(ctx, port, spuAddr, hd)`, as SNDBASE's ccSetHdSynth
    /// (0x257c) calls it: it clears the port's priority first.
    pub fn load(&mut self, port: usize, bank: Arc<Bank>, spu_addr: u32) {
        let p = &mut self.ports[port];
        p.priority = 0;
        p.bank = Some(HsynBank::new(bank, spu_addr));
    }

    /// SNDBASE ccSetPortAttr: priority `attr & 0xff`, voices `attr >> 8`.
    pub fn set_attr(&mut self, port: usize, attr: u16) {
        self.ports[port].priority = (attr & 0xff) as i64;
        self.ports[port].max_polyphony = (attr >> 8) as i64;
    }

    /// `sceHSyn_SetVolume` (0x1bc4): 0..256; voices follow on the next tick.
    pub fn set_volume(&mut self, port: usize, vol: u16) {
        self.ports[port].volume = vol as i64;
    }

    /// MIDI bytes for a port's input buffer, parsed at the next tick.
    pub fn input(&mut self, port: usize, msg: &[u8]) {
        self.ports[port].input.extend_from_slice(msg);
    }

    /// `sceHSyn_ResetAllControler` (0x1d08): CC121 on every channel.
    pub fn reset_all_controllers(&mut self, port: usize) {
        for c in 0..16 {
            self.control(port, c, 121, 0);
        }
    }

    /// `sceHSyn_AllNoteOff` (0x1c40): 0x60d4 on every channel.
    pub fn all_note_off(&mut self, port: usize, _regs: &mut dyn Regs) {
        for c in 0..16 {
            self.all_notes_off(port, c);
        }
    }

    /// `sceHSyn_AllSoundOff` (0x1ca4): 0x6214 (CC120) on every channel.
    pub fn all_sound_off(&mut self, port: usize, regs: &mut dyn Regs) {
        for c in 0..16 {
            self.sound_off(port, c, regs);
        }
    }

    /// Voices sounding or waiting to.
    pub fn busy_voices(&self) -> usize {
        self.pending.iter().chain(&self.active).map(|l| l.len()).sum()
    }

    // lists ---------------------------------------------------------------------

    fn unlink_global(&mut self, v: usize) {
        for l in self.free.iter_mut().chain(self.pending.iter_mut()).chain(self.active.iter_mut()) {
            if let Some(i) = l.iter().position(|&x| x == v) {
                l.remove(i);
                return;
            }
        }
    }

    fn unlink_chan(&mut self, v: usize) {
        let Some((p, c)) = self.voices[v].chan else { return };
        for l in self.ports[p].lists[c].iter_mut() {
            l.retain(|&x| x != v);
        }
    }

    fn release_count(&mut self, v: usize) {
        let p = self.voices[v].port;
        if self.voices[v].chan.is_some() || self.voices[v].p.is_some() {
            let ps = &mut self.ports[p];
            if ps.voices != 0 {
                ps.voices -= 1;
            }
        }
    }

    /// 0x62f4 (with the channel unlink) / 0x5e60 (without).
    fn free_voice(&mut self, v: usize, unlink_chan: bool) {
        if unlink_chan {
            self.unlink_chan(v);
        }
        self.voices[v].flags = 0;
        self.release_count(v);
        self.unlink_global(v);
        self.free[v / 24].insert(0, v);
        let hv = &mut self.voices[v];
        hv.id = 255;
        hv.mode = 255;
        hv.chan = None;
    }

    /// KOFF + ADSR2 0xc021 at once + free (0x6214, 0x6438, 0x6f44).
    fn cut(&mut self, v: usize, regs: &mut dyn Regs) {
        self.koff[v / 24] |= 1 << (v % 24);
        regs.set_param(VP_ADSR2 | spec(v), ADSR2_CUT);
        self.free_voice(v, true);
    }

    // MIDI (0x15cc, 0x12e8, 0x1418) ------------------------------------------------

    fn midi(&mut self, port: usize, data: &[u8], regs: &mut dyn Regs) {
        let n = data.len();
        let mut i = 0;
        while i < n {
            let st = data[i];
            i += 1;
            if st & 0x80 == 0 {
                continue;
            }
            if st < 0xf0 {
                if i >= n {
                    return;
                }
                let d1 = data[i];
                i += 1;
                let mut d2 = 0;
                if [false, true, true, true, false, false, true][((st - 0x80) >> 4) as usize] {
                    if i >= n {
                        return;
                    }
                    d2 = data[i];
                    i += 1;
                }
                self.channel_msg(port, st, d1 as i64, d2 as i64, regs);
            } else if st == 0xf0 {
                while i < n && data[i] != 0xf7 {
                    i += 1;
                }
                i += 1;
            } else if st == 0xf9 || st == 0xfd {
                let len = if st == 0xf9 { 4 } else { 6 };
                if i + len > n {
                    return;
                }
                let b: Vec<i64> = data[i..i + len].iter().map(|&x| x as i64).collect();
                self.hs_msg(port, st, &b, regs);
                i += len;
            }
        }
    }

    fn channel_msg(&mut self, port: usize, st: u8, d1: i64, d2: i64, regs: &mut dyn Regs) {
        let c = (st & 15) as usize;
        match st & 0xf0 {
            0x90 if d2 != 0 => self.note_on(port, c, d1 & 0xff, d2 & 0xff, 255, regs),
            0x80 | 0x90 => self.note_off(port, c, d1 & 0xff, 255),
            0xb0 => self.control_regs(port, c, d1, d2, regs),
            0xc0 => {
                let ps = &mut self.ports[port];
                let p = ps.bank.as_ref().and_then(|b| {
                    let n = d1;
                    (0..=b.prog_max)
                        .contains(&n)
                        .then_some(n as usize)
                        .filter(|&n| b.program(n).is_some_and(|p| p.n_split != 0))
                });
                if ps.bank.is_some() {
                    ps.channels[c].program = p;
                }
            }
            0xe0 => {
                let ch = &mut self.ports[port].channels[c];
                let old = ch.bend;
                pitch_bend(ch, d1, d2);
                if ch.bend != old {
                    let bend = ch.bend;
                    for v in self.chan_voices(port, c) {
                        if let Some(p) = self.voices[v].p.as_mut()
                            && p.vag.is_some()
                        {
                            p.bend = bend_offset(&p.split, bend);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn hs_msg(&mut self, port: usize, st: u8, b: &[i64], regs: &mut dyn Regs) {
        let c = (b[1] & 15) as usize;
        if st == 0xf9 {
            let ch = &mut self.ports[port].channels[c];
            match b[0] {
                0 => ch.note_expr = b[2],
                1 => ch.note_pan = b[2],
                2 => ch.note_bend = s16(b[2] + (b[3] << 7)),
                _ => {}
            }
            return;
        }
        let (kind, note, vid, val, val2) = (b[0], b[2], b[3], b[4], b[5]);
        let anynote = b[1] & 0x10 != 0;
        if kind == 0x10 {
            if val != 0 {
                self.note_on(port, c, note & 0x7f, val, vid, regs);
            } else {
                self.note_off(port, c, note & 0x7f, vid);
            }
            return;
        }
        let ch = self.ports[port].channels[c].clone();
        let mono = self.mono;
        for v in self.chan_voices(port, c) {
            let hv = &self.voices[v];
            if hv.p.is_none() || !((vid == 127 || hv.id == vid) && (anynote || hv.note == note)) {
                continue;
            }
            let p = self.voices[v].p.as_mut().unwrap();
            match kind {
                0 => {
                    p.expr = cc_scale127(val);
                    p.level24 = channel_level(p.level22, ch.volume, p.expr);
                }
                1 => {
                    let q = s8(val - 64).max(-63);
                    let (gl, gr, ph) = pan_gains(p.pan_base, p.phase, q, p.prog_attr, mono);
                    (p.gl, p.gr, p.phase) = (gl, gr, ph);
                }
                2 if p.vag.is_some() => p.bend = bend_offset(&p.split, s16((val2 << 7) + val - 8192)),
                _ => {}
            }
        }
    }

    fn chan_voices(&self, port: usize, c: usize) -> Vec<usize> {
        let l = &self.ports[port].lists[c];
        l[0].iter().chain(&l[1]).chain(&l[2]).copied().collect()
    }

    fn control(&mut self, port: usize, c: usize, num: i64, val: i64) {
        let mut dummy = crate::spu::Recorder::default();
        self.control_regs(port, c, num, val, &mut dummy);
    }

    /// 0x1130: the controllers.
    fn control_regs(&mut self, port: usize, c: usize, num: i64, val: i64, regs: &mut dyn Regs) {
        match num {
            64 => {
                self.sustain(port, c, val);
                return;
            }
            120 => {
                self.sound_off(port, c, regs);
                return;
            }
            123 => {
                self.all_notes_off(port, c);
                return;
            }
            121 => {
                self.sustain(port, c, 0);
                let ch = &mut self.ports[port].channels[c];
                let old = (ch.bend, ch.breath, ch.modulation, ch.volume, ch.pan, ch.expression);
                reset_controllers(ch);
                let new = (ch.bend, ch.breath, ch.modulation, ch.volume, ch.pan, ch.expression);
                let changed = [
                    ("bend", old.0 != new.0),
                    ("breath", old.1 != new.1),
                    ("modulation", old.2 != new.2),
                    ("volume", old.3 != new.3),
                    ("pan", old.4 != new.4),
                    ("expression", old.5 != new.5),
                ];
                for (what, ch) in changed {
                    if ch {
                        self.apply_all(port, c, what);
                    }
                }
                return;
            }
            _ => {}
        }
        let ch = &mut self.ports[port].channels[c];
        let old = (ch.volume, ch.expression, ch.pan, ch.modulation, ch.breath);
        let value = val & 0xff;
        match num {
            0 => ch.bank = value as u8,
            1 => ch.modulation = cc_scale127(value),
            2 => ch.breath = cc_scale127(value),
            5 => ch.porta_time = (value * 20) & 0xffff,
            6 => {
                ch.nrpn[2] = value;
                ch.nrpn = [255; 3];
            }
            7 => ch.volume = cc_scale127(value),
            10 => ch.pan = s8(value - 64).max(-63),
            11 => ch.expression = cc_scale127(value),
            65 => {
                if value != 0 {
                    if ch.porta_note == 255 {
                        ch.porta_note = 128;
                    }
                } else {
                    ch.porta_note = 255;
                }
            }
            84 => ch.porta_ctrl = value,
            98 => ch.nrpn[1] = value,
            99 => ch.nrpn = [value, 255, 255],
            _ => {}
        }
        let what = match num {
            7 => "volume",
            11 => "expression",
            10 => "pan",
            1 => "modulation",
            2 => "breath",
            _ => return,
        };
        let new = (ch.volume, ch.expression, ch.pan, ch.modulation, ch.breath);
        if new != old {
            self.apply_all(port, c, what);
        }
    }

    fn apply_all(&mut self, port: usize, c: usize, what: &str) {
        let ch = self.ports[port].channels[c].clone();
        let mono = self.mono;
        for v in self.chan_voices(port, c) {
            let Some(p) = self.voices[v].p.as_mut() else { continue };
            match what {
                "bend" => {
                    if p.vag.is_some() {
                        p.bend = bend_offset(&p.split, ch.bend);
                    }
                }
                "volume" => p.level24 = channel_level(p.level22, ch.volume, p.expr),
                "expression" => {
                    p.expr = ch.expression;
                    p.level24 = channel_level(p.level22, ch.volume, p.expr);
                }
                "pan" => {
                    let (gl, gr, ph) = pan_gains(p.pan_base, p.phase, ch.pan, p.prog_attr, mono);
                    (p.gl, p.gr, p.phase) = (gl, gr, ph);
                }
                "modulation" => set_modulation(p, ch.modulation),
                "breath" => set_breath(p, ch.breath),
                _ => {}
            }
        }
    }

    /// 0x5f94.
    fn sustain(&mut self, port: usize, c: usize, val: i64) {
        let ch = &mut self.ports[port].channels[c];
        if val != 0 {
            ch.sustain = val;
            return;
        }
        if ch.sustain == 0 {
            return;
        }
        ch.sustain = 0;
        while let Some(&v) = self.ports[port].lists[c][1].first() {
            self.ports[port].lists[c][1].remove(0);
            if self.voices[v].flags & 8 != 0 {
                self.free_voice(v, false);
                continue;
            }
            if let Some(p) = self.voices[v].p.as_mut() {
                p.lfo_pitch.key_off();
                p.lfo_amp.key_off();
            }
            self.koff[v / 24] |= 1 << (v % 24);
            self.ports[port].lists[c][2].insert(0, v);
            self.voices[v].flags &= !0x300;
        }
    }

    /// 0x6214 (CC120): cut every voice of the channel.
    fn sound_off(&mut self, port: usize, c: usize, regs: &mut dyn Regs) {
        for k in 0..3 {
            while let Some(&v) = self.ports[port].lists[c][k].last() {
                self.cut(v, regs);
            }
        }
    }

    /// 0x60d4 (CC123): KOFF on held and sustained voices, no LFO key off;
    /// pending ones freed.
    fn all_notes_off(&mut self, port: usize, c: usize) {
        for k in 0..2 {
            while let Some(&v) = self.ports[port].lists[c][k].last() {
                self.ports[port].lists[c][k].pop();
                if self.voices[v].flags & 8 != 0 {
                    self.free_voice(v, false);
                    continue;
                }
                self.voices[v].flags &= !0x300;
                self.koff[v / 24] |= 1 << (v % 24);
                self.ports[port].lists[c][2].insert(0, v);
            }
        }
    }

    /// 0x5cb4.
    fn note_off(&mut self, port: usize, c: usize, note: i64, vid: i64) {
        let ch = &mut self.ports[port].channels[c];
        if ch.porta_note <= 128 {
            ch.porta_note = note;
        }
        let sustain = ch.sustain;
        let held: Vec<usize> = self.ports[port].lists[c][0].iter().rev().copied().collect();
        for v in held {
            let hv = &self.voices[v];
            if hv.note != note || !(hv.id == vid || vid == 255) {
                continue;
            }
            self.ports[port].lists[c][0].retain(|&x| x != v);
            self.voices[v].flags &= !0x300;
            if sustain != 0 {
                self.voices[v].flags |= 0x100;
                self.ports[port].lists[c][1].insert(0, v);
            } else if self.voices[v].flags & 8 != 0 {
                self.free_voice(v, false);
            } else {
                if let Some(p) = self.voices[v].p.as_mut() {
                    p.lfo_pitch.key_off();
                    p.lfo_amp.key_off();
                }
                self.koff[v / 24] |= 1 << (v % 24);
                self.ports[port].lists[c][2].insert(0, v);
            }
        }
    }

    /// 0x65bc.
    fn note_on(&mut self, port: usize, c: usize, note: i64, vel: i64, vid: i64, regs: &mut dyn Regs) {
        let ps = &self.ports[port];
        let (Some(prog_i), Some(hb)) = (ps.channels[c].program, ps.bank.as_ref()) else { return };
        let bank = hb.bank.clone();
        let spu_addr = hb.spu_addr;
        let (sset_max, smpl_max, vagi_max) = (hb.sset_max, hb.smpl_max, hb.vagi_max);
        let Some(prog) = bank.hd.programs.get(prog_i).and_then(|p| p.as_ref()) else { return };
        let _ = spu_addr;
        let mut started = false;
        for sp in prog.splits.iter().take(prog.n_split as usize) {
            if note < (sp.key_low & 0x7f) as i64 || ((sp.key_high & 0x7f) as i64) < note {
                continue;
            }
            if sp.sample_set as i64 > sset_max {
                continue;
            }
            let Some(ss) = bank.hd.sample_sets.get(sp.sample_set as usize).and_then(|s| s.as_ref()) else { continue };
            if vel < ss.vel_low as i64 || (ss.vel_high as i64) < vel {
                continue;
            }
            for &si in &ss.samples {
                if si as i64 > smpl_max {
                    continue;
                }
                let Some(smp) = bank.hd.samples.get(si as usize).and_then(|s| s.as_ref()) else { continue };
                if vel < (smp.vel_low & 0x7f) as i64 || ((smp.vel_high & 0x7f) as i64) < vel {
                    continue;
                }
                let vag = if (smp.vag as i64) <= vagi_max {
                    match bank.hd.vags.get(smp.vag as usize).copied().flatten() {
                        Some(v) if v.offset != 0xffff_ffff => Some(v),
                        _ => continue,
                    }
                } else {
                    None
                };
                if smp.group != 0 {
                    self.group_cut(port, c, smp.group, regs);
                }
                let prio = (smp.priority as i64 + self.ports[port].priority) & 0xffff;
                if self.ports[port].voices >= self.ports[port].max_polyphony && !self.steal_port(port, prio, regs) {
                    return;
                }
                let mut v = self.alloc(smp.spu_attr);
                if v.is_none() && self.reclaim_ok {
                    self.reclaim_ok = false;
                    if self.reclaim(regs) {
                        v = self.alloc(smp.spu_attr);
                    }
                }
                if v.is_none() {
                    v = self.steal_active(prio, smp.spu_attr);
                }
                if v.is_none() {
                    v = self.steal_pending(prio, smp.spu_attr);
                }
                let Some(v) = v else { continue };
                let ch = self.ports[port].channels[c].clone();
                let mono = self.mono;
                let params = self.setup_voice(&bank, prog_i, prog, *sp, ss.vel_curve, *smp, vag, note, vel, &ch, mono);
                let hv = &mut self.voices[v];
                hv.flags = 0x208 | (params.flags & 6);
                hv.prio = prio;
                hv.note = note;
                hv.id = vid;
                hv.mode = 0;
                hv.envx_last = 0;
                hv.port = port;
                hv.chan = Some((port, c));
                hv.p = Some(Box::new(params));
                self.ports[port].voices += 1;
                self.ports[port].lists[c][0].insert(0, v);
                started = true;
                self.pending[v / 24].insert(0, v);
            }
        }
        if started {
            let ch = &mut self.ports[port].channels[c];
            ch.note_bend = -1;
            ch.note_expr = 255;
            ch.note_pan = 0xff;
        }
    }

    /// 0x3b94.
    #[allow(clippy::too_many_arguments)]
    fn setup_voice(
        &mut self,
        bank: &Arc<Bank>,
        prog_i: usize,
        prog: &Program,
        sp: Split,
        vel_curve: u8,
        smp: Sample,
        vag: Option<Vag>,
        note: i64,
        vel: i64,
        ch: &Channel,
        mono: bool,
    ) -> Params {
        let lfo_pitch =
            Lfo::new(prog, smp.lfo_attr & 0x07, smp.pitch_lfo_delay, smp.pitch_lfo_fade, false, &mut self.rng);
        let lfo_amp = Lfo::new(prog, smp.lfo_attr & 0x70, smp.amp_lfo_delay, smp.amp_lfo_fade, true, &mut self.rng);
        let kf = tdiv((note - sp.key_follow_pitch_center as i64) * sp.key_follow_pitch as i64, 12);
        let vf = velocity_follow(vel, smp.vel_follow_pitch, smp.vel_follow_pitch_center, smp.vel_follow_pitch_curve);
        let pdepth_base = (
            (prog.lfo_pitch_depth as i64 + kf + vf).clamp(-32768, 32767),
            (prog.lfo_pitch_depth2 as i64 + kf + vf).clamp(-32768, 32767),
        );
        let kf = tdiv((note - sp.key_follow_amp_center as i64) * sp.key_follow_amp as i64, 12);
        let vf = velocity_follow(vel, smp.vel_follow_amp, smp.vel_follow_amp_center, smp.vel_follow_amp_curve);
        let adepth_base = (
            (prog.lfo_amp_depth as i64 + kf + vf).clamp(-128, 127),
            (prog.lfo_amp_depth2 as i64 + kf + vf).clamp(-128, 127),
        );
        let vm = velocity_map(vel, vel_curve);
        let xfade = crossfade(&smp, &sp, vel, note);
        let level22 = base_level(prog, &sp, &smp, vm, xfade);
        let (pan_b, ph) = pan_base(prog, &sp, &smp, note);
        let phase = ph.unwrap_or(0);
        let prog_attr = (prog.attr & 1) as i64;
        let cp = if ch.note_pan & 0x80 != 0 { ch.pan } else { s8(ch.note_pan - 64).max(-63) };
        let (gl, gr, phase) = pan_gains(pan_b, phase, cp, prog_attr, mono);
        let expr = if ch.note_expr == 255 { ch.expression } else { ch.note_expr };
        let level24 = channel_level(level22, ch.volume, expr);
        let mut flags = 0x208;
        let mut porta = 0;
        let mut porta_step = 0;
        if ch.porta_note & 0x80 == 0 && ch.porta_time != 0 {
            let mut d = (ch.porta_note - note) << 7;
            porta = d;
            if d < 0 {
                flags |= 2;
                d = -d;
            } else {
                flags |= 4;
            }
            let q = ((d * TICK_256MS) & M32) / ch.porta_time;
            porta_step = sra_trunc(q, 8) & 0xffff;
        }
        let bend = if vag.is_some() {
            let b = if ch.note_bend & 0x8000 != 0 || ch.note_bend < 0 { ch.bend } else { ch.note_bend - 8192 };
            bend_offset(&sp, b)
        } else {
            0
        };
        let mut p = Params {
            bank: bank.clone(),
            prog: prog_i,
            split: sp,
            sample: smp,
            vag,
            note,
            group: smp.group,
            spu_attr: smp.spu_attr,
            lfo_pitch,
            lfo_amp,
            pdepth_base,
            adepth_base,
            pdepth: (0, 0),
            adepth: (0, 0),
            level22,
            pan_base: pan_b,
            phase,
            prog_attr,
            gl,
            gr,
            expr,
            level24,
            flags,
            porta,
            porta_step,
            pitch_cache: None,
            vol_cache: None,
            bend,
        };
        set_breath(&mut p, ch.breath);
        set_modulation(&mut p, ch.modulation);
        p
    }

    /// 0x6b3c: the free list's tail of the core the sample asks for, or of
    /// the core with more free voices (ties to core 0); no fallback.
    fn alloc(&mut self, spu_attr: u8) -> Option<usize> {
        let core = match spu_attr & 0x30 {
            0x10 => 0,
            0x20 => 1,
            _ => {
                if self.free[0].len() < self.free[1].len() {
                    1
                } else {
                    0
                }
            }
        };
        self.free[core].pop()
    }

    /// 0x6f44: exclusive groups.
    fn group_cut(&mut self, port: usize, c: usize, group: u8, regs: &mut dyn Regs) {
        for i in 0..3 {
            let list: Vec<usize> = self.ports[port].lists[c][i].iter().rev().copied().collect();
            for v in list {
                if self.voices[v].p.as_ref().is_none_or(|p| p.group != group) {
                    continue;
                }
                if self.voices[v].flags & 8 != 0 {
                    self.ports[port].lists[c][i].retain(|&x| x != v);
                    self.free_voice(v, false);
                    continue;
                }
                if i < 2 {
                    self.koff[v / 24] |= 1 << (v % 24);
                }
                regs.set_param(VP_ADSR2 | spec(v), ADSR2_CUT);
                self.free_voice(v, true);
            }
        }
    }

    /// 0x6438: the port's lowest-priority released voice (the oldest on a
    /// tie), else held or sustained; only at or below the new priority.
    fn steal_port(&mut self, port: usize, prio: i64, regs: &mut dyn Regs) -> bool {
        let mut best = prio;
        let mut sel = None;
        for c in 0..16 {
            for &v in &self.ports[port].lists[c][2] {
                if self.voices[v].prio <= best {
                    best = self.voices[v].prio;
                    sel = Some(v);
                }
            }
        }
        if sel.is_none() {
            for c in 0..16 {
                for k in 0..2 {
                    for &v in &self.ports[port].lists[c][k] {
                        if self.voices[v].prio <= best {
                            best = self.voices[v].prio;
                            sel = Some(v);
                        }
                    }
                }
            }
        }
        match sel {
            Some(v) => {
                self.cut(v, regs);
                true
            }
            None => false,
        }
    }

    /// 0x507c: free every voice whose envelope has ended.
    fn reclaim(&mut self, regs: &mut dyn Regs) -> bool {
        let mut freed = false;
        for core in 0..2 {
            let endx = regs.get_switch(S_ENDX | core as u16);
            let list: Vec<usize> = self.active[core].iter().rev().copied().collect();
            for v in list {
                let n = v % 24;
                let e = regs.get_param(VP_ENVX | spec(v)) as i64;
                self.voices[v].envx_last = e;
                if self.voices[v].flags & 0x300 != 0 {
                    if self.voices[v].countdown != 0 {
                        let hv = &mut self.voices[v];
                        hv.countdown = if endx >> n & 1 != 0 { hv.countdown - 1 } else { 0 };
                        hv.envx_last = 0x7fff;
                        continue;
                    }
                    if endx >> n & 1 == 0 || e != 0 {
                        continue;
                    }
                } else if e != 0 {
                    continue;
                }
                self.free_voice(v, true);
                freed = true;
            }
        }
        freed
    }

    /// 0x6be0: steal a sounding voice (core 1 first, oldest first): released
    /// before sustained before held, then the lowest priority, then the
    /// lowest remembered ENVX. It gets a normal KOFF; the new note waits.
    fn steal_active(&mut self, prio: i64, spu_attr: u8) -> Option<usize> {
        let mask = if spu_attr & 0x30 != 0 { spu_attr & 0x30 } else { 0x30 };
        let (mut state, mut bprio, mut benv, mut sel) = (0x300u32, prio, 0xffffi64, None);
        for core in [1usize, 0] {
            if mask & (0x10 << core) == 0 {
                continue;
            }
            for &v in self.active[core].iter().rev() {
                let hv = &self.voices[v];
                if hv.mode != 0 {
                    continue;
                }
                let s = hv.flags & 0x300;
                if s < state {
                    if hv.prio <= prio {
                        bprio = hv.prio;
                        state = s;
                        benv = hv.envx_last;
                        sel = Some(v);
                    }
                } else if s == state {
                    if hv.prio == bprio {
                        if hv.envx_last < benv {
                            benv = hv.envx_last;
                            sel = Some(v);
                        }
                    } else if hv.prio < bprio {
                        bprio = hv.prio;
                        benv = hv.envx_last;
                        sel = Some(v);
                    }
                }
            }
        }
        let v = sel?;
        self.koff[v / 24] |= 1 << (v % 24);
        self.release_count(v);
        self.unlink_chan(v);
        self.unlink_global(v);
        let hv = &mut self.voices[v];
        hv.flags = 0;
        hv.id = 255;
        hv.mode = 255;
        hv.chan = None;
        Some(v)
    }

    /// 0x6de4: the lowest-priority waiting voice.
    fn steal_pending(&mut self, prio: i64, spu_attr: u8) -> Option<usize> {
        let mask = if spu_attr & 0x30 != 0 { spu_attr & 0x30 } else { 0x30 };
        let (mut th, mut sel) = (prio + 1, None);
        for core in [1usize, 0] {
            if mask & (0x10 << core) == 0 {
                continue;
            }
            for &v in self.pending[core].iter().rev() {
                let hv = &self.voices[v];
                if hv.mode == 0 && hv.prio < th {
                    th = hv.prio;
                    sel = Some(v);
                }
            }
        }
        let v = sel?;
        self.release_count(v);
        self.unlink_chan(v);
        self.unlink_global(v);
        let hv = &mut self.voices[v];
        hv.flags = 0;
        hv.id = 255;
        hv.mode = 255;
        hv.chan = None;
        Some(v)
    }

    /// 0x3f50: a pending voice's registers.
    fn key_on(&mut self, v: usize, regs: &mut dyn Regs) {
        let core = v / 24;
        let bit = 1u32 << (v % 24);
        let sp = spec(v);
        let port = self.voices[v].port;
        let port_volume = self.ports[port].volume;
        let spu_addr = self.ports[port].bank.as_ref().map_or(0, |b| b.spu_addr);
        let p = self.voices[v].p.as_mut().expect("a pending voice's parameters");
        p.pitch_cache = None;
        p.vol_cache = None;
        let (voll, volr) = update_volume(p, port_volume).unwrap_or((0, 0));
        regs.set_param(VP_VOLL | sp, voll);
        regs.set_param(VP_VOLR | sp, volr);
        let spu_attr = p.spu_attr;
        let mut one_shot = false;
        if let Some(vag) = p.vag {
            self.noise[core] &= !bit;
            let pitch = update_pitch(p).unwrap_or(0);
            regs.set_param(VP_PITCH | sp, pitch);
            regs.set_addr(VA_SSA | sp, spu_addr.wrapping_add(vag.offset));
            regs.set_param(VP_ADSR1 | sp, adsr1(p.note, &p.sample));
            regs.set_param(VP_ADSR2 | sp, adsr2(p.note, &p.sample));
            one_shot = !vag.looped;
        } else {
            // SD_C_NOISE_CLK: a noise voice's clock is its note.
            regs.set_core_attr(core as u16 | 8, p.note.min(63) as u16);
            self.noise[core] |= bit;
        }
        if one_shot {
            self.voices[v].flags |= 1;
        }
        regs.set_switch(S_NON | core as u16, self.noise[core]);
        for (k, b) in [1u8, 2, 4, 8].into_iter().enumerate() {
            if spu_attr & b != 0 {
                self.vmix[core][k] |= bit;
            } else {
                self.vmix[core][k] &= !bit;
            }
        }
        // VMIXL, VMIXR, VMIXEL, VMIXER, in that order.
        for (k, entry) in [(0usize, S_VMIX[0]), (1, S_VMIX[1]), (2, S_VMIX[2]), (3, S_VMIX[3])] {
            regs.set_switch(entry | core as u16, self.vmix[core][k]);
        }
        self.kon[core] |= bit;
        self.voices[v].countdown = 3;
        self.voices[v].flags &= !8;
    }

    /// `sceHSyn_ATick` (0x182c): each port's MIDI, then 0x37a4 (KOFF, key
    /// ons, KON) and 0x4dc0 (the sounding voices).
    pub fn tick(&mut self, regs: &mut dyn Regs) {
        for port in 0..4 {
            let input = std::mem::take(&mut self.ports[port].input);
            if !input.is_empty() {
                self.midi(port, &input, regs);
            }
        }
        for core in 0..2 {
            if self.koff[core] != 0 {
                regs.set_switch(S_KOFF | core as u16, self.koff[core]);
                self.koff[core] = 0;
            }
        }
        for core in 0..2 {
            let list: Vec<usize> = self.pending[core].iter().rev().copied().collect();
            for v in list {
                if regs.get_param(VP_ENVX | spec(v)) < KON_ENVX {
                    self.key_on(v, regs);
                    self.pending[core].retain(|&x| x != v);
                    self.active[core].insert(0, v);
                }
            }
            if self.kon[core] != 0 {
                regs.set_switch(S_KON | core as u16, self.kon[core]);
                self.kon[core] = 0;
            }
        }
        for core in 0..2 {
            let endx = regs.get_switch(S_ENDX | core as u16);
            let list: Vec<usize> = self.active[core].iter().rev().copied().collect();
            for v in list {
                let n = v % 24;
                if self.reclaim_ok {
                    let e = regs.get_param(VP_ENVX | spec(v));
                    let hv = &mut self.voices[v];
                    if hv.flags & 0x300 != 0 {
                        if hv.countdown != 0 {
                            hv.countdown = if endx >> n & 1 != 0 { hv.countdown - 1 } else { 0 };
                        } else if endx >> n & 1 != 0 && e == 0 {
                            self.free_voice(v, true);
                            continue;
                        }
                    } else if e == 0 {
                        self.free_voice(v, true);
                        continue;
                    }
                }
                let port_volume = self.ports[self.voices[v].port].volume;
                let sp = spec(v);
                let Some(p) = self.voices[v].p.as_mut() else { continue };
                let (pitch, vol) = tick_params(p, port_volume, &mut self.rng);
                if let Some(pitch) = pitch {
                    regs.set_param(VP_PITCH | sp, pitch);
                }
                if let Some((l, r)) = vol {
                    regs.set_param(VP_VOLL | sp, l);
                    regs.set_param(VP_VOLR | sp, r);
                }
            }
        }
        self.reclaim_ok = true;
    }
}
