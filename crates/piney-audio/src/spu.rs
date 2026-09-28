//! The SPU2 as the synthesizer drives it: 2 cores x 24 voices playing
//! PS-ADPCM from sound RAM at 48 kHz, each with a pitch counter, 4-point
//! interpolation, an ADSR envelope and a fixed left/right volume, and a
//! sound-data input for streamed PCM.
//!
//! Registers are addressed the way libsd addresses them (`sceSdSetParam`,
//! `sceSdSetSwitch`, `sceSdSetAddr` entries), so what MODHSYN.IRX writes -
//! recorded by `tools/iopemu.py` - can be replayed here unchanged:
//!
//! ```text
//! voice param   (param << 8) | (voice << 1) | core     0 VOLL, 1 VOLR, 2 PITCH, 3 ADSR1, 4 ADSR2
//! voice address 0x2040 | (voice << 1) | core            SSA (0x2140 LSAX)
//! switch        (switch << 8) | core, 24-bit mask       0x15 KON, 0x16 KOFF, 0x18 VMIXL,
//!                                                       0x19 VMIXEL, 0x1a VMIXR, 0x1b VMIXER
//! ```
//!
//! The pitch counter, the interpolation table, the envelope generator and
//! the voice volume follow the PlayStation SPU as documented by psx-spx
//! ("SPU ADPCM Pitch", "SPU Volume and ADSR Generator"); the SPU2 runs the
//! same voices at 48 kHz, so pitch 0x1000 plays a sample at 48,000 Hz.
//! The effect sends go to [`crate::reverb`].

use piney_data::sound::adpcm::{self, History, PER_FRAME};

use crate::reverb::Reverb;

pub const RATE: u32 = 48_000;
pub const VOICES: usize = 24;
pub const RAM: usize = 2 << 20;

pub mod param {
    pub const VOLL: u16 = 0;
    pub const VOLR: u16 = 1;
    pub const PITCH: u16 = 2;
    pub const ADSR1: u16 = 3;
    pub const ADSR2: u16 = 4;
    pub const ENVX: u16 = 5;
}

pub mod switch {
    pub const KON: u16 = 0x15;
    pub const KOFF: u16 = 0x16;
    pub const ENDX: u16 = 0x17;
    pub const VMIXL: u16 = 0x18;
    pub const VMIXEL: u16 = 0x19;
    pub const VMIXR: u16 = 0x1a;
    pub const VMIXER: u16 = 0x1b;
}

pub mod addr {
    pub const SSA: u16 = 0x20;
    pub const LSAX: u16 = 0x21;
    pub const NAX: u16 = 0x22;
}

/// The SPU's interpolation table (psx-spx, "4-Point Gaussian
/// Interpolation"): a four-term cosine window; each four entries
/// `[i], [0xff - i], [0x100 + i], [0x1ff - i]` sum to 0x7f7f..0x7f81.
#[rustfmt::skip]
static GAUSS: [i16; 512] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3,
    3, 4, 4, 5, 5, 6, 7, 7, 8, 9, 9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 21, 22, 24, 25, 27, 28, 30, 32, 33, 35, 37,
    39, 41, 44, 46, 48, 51, 53, 56, 58, 61, 64, 67, 70, 73, 77, 80,
    84, 87, 91, 95, 99, 103, 107, 111, 116, 120, 125, 130, 135, 140, 145, 150,
    156, 161, 167, 173, 179, 186, 192, 199, 205, 212, 219, 227, 234, 242, 250, 257,
    266, 274, 283, 291, 300, 309, 319, 328, 338, 348, 358, 369, 379, 390, 401, 412,
    424, 436, 448, 460, 473, 485, 498, 512, 525, 539, 553, 567, 582, 597, 612, 627,
    643, 659, 675, 692, 708, 726, 743, 761, 779, 797, 816, 835, 854, 874, 894, 914,
    935, 956, 977, 999, 1020, 1043, 1066, 1089, 1112, 1136, 1160, 1184, 1209, 1234, 1260, 1286,
    1312, 1339, 1366, 1394, 1422, 1450, 1479, 1508, 1537, 1567, 1598, 1628, 1660, 1691, 1723, 1756,
    1789, 1822, 1856, 1890, 1924, 1959, 1995, 2031, 2067, 2104, 2141, 2179, 2217, 2256, 2295, 2334,
    2374, 2415, 2456, 2497, 2539, 2582, 2624, 2668, 2712, 2756, 2801, 2846, 2892, 2938, 2985, 3032,
    3079, 3128, 3176, 3225, 3275, 3325, 3376, 3427, 3479, 3531, 3584, 3637, 3691, 3745, 3799, 3855,
    3910, 3967, 4023, 4081, 4138, 4197, 4255, 4315, 4374, 4435, 4495, 4557, 4619, 4681, 4744, 4807,
    4871, 4935, 5000, 5065, 5131, 5197, 5264, 5332, 5399, 5468, 5536, 5606, 5676, 5746, 5817, 5888,
    5959, 6032, 6104, 6177, 6251, 6325, 6400, 6475, 6550, 6626, 6702, 6779, 6856, 6934, 7012, 7091,
    7170, 7249, 7329, 7409, 7490, 7571, 7653, 7735, 7817, 7900, 7983, 8066, 8150, 8234, 8319, 8404,
    8489, 8575, 8661, 8748, 8834, 8922, 9009, 9097, 9185, 9273, 9362, 9451, 9541, 9630, 9720, 9811,
    9901, 9992, 10083, 10174, 10266, 10358, 10450, 10542, 10635, 10727, 10820, 10913, 11007, 11100, 11194, 11288,
    11382, 11476, 11571, 11665, 11760, 11855, 11950, 12045, 12140, 12236, 12331, 12427, 12522, 12618, 12714, 12809,
    12905, 13001, 13097, 13193, 13289, 13385, 13481, 13577, 13673, 13769, 13865, 13961, 14056, 14152, 14248, 14343,
    14439, 14534, 14630, 14725, 14820, 14915, 15010, 15104, 15199, 15293, 15387, 15481, 15575, 15669, 15762, 15855,
    15948, 16041, 16133, 16226, 16317, 16409, 16500, 16592, 16682, 16773, 16863, 16953, 17042, 17131, 17220, 17308,
    17396, 17484, 17571, 17658, 17744, 17830, 17916, 18001, 18086, 18170, 18254, 18337, 18420, 18502, 18584, 18665,
    18746, 18826, 18905, 18985, 19063, 19141, 19219, 19295, 19372, 19447, 19522, 19597, 19671, 19744, 19816, 19888,
    19959, 20030, 20100, 20169, 20238, 20306, 20373, 20439, 20505, 20570, 20634, 20698, 20760, 20822, 20884, 20944,
    21004, 21063, 21121, 21178, 21235, 21290, 21345, 21399, 21452, 21505, 21556, 21607, 21657, 21706, 21754, 21801,
    21848, 21893, 21938, 21982, 22025, 22066, 22107, 22148, 22187, 22225, 22262, 22299, 22334, 22369, 22402, 22435,
    22467, 22498, 22527, 22556, 22584, 22611, 22637, 22662, 22686, 22709, 22731, 22752, 22772, 22791, 22809, 22826,
    22842, 22857, 22872, 22885, 22897, 22908, 22918, 22927, 22935, 22942, 22948, 22953, 22957, 22960, 22962, 22963,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// The ADSR generator of one voice (psx-spx, "Envelope Operation depending
/// on Shift/Step/Mode/Direction"), stepped once per output sample.
#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    adsr1: u16,
    adsr2: u16,
    phase: Phase,
    level: i32,
    counter: u32,
}

impl Envelope {
    fn new() -> Envelope {
        Envelope { adsr1: 0, adsr2: 0, phase: Phase::Off, level: 0, counter: 0 }
    }

    fn key_on(&mut self) {
        self.phase = Phase::Attack;
        self.level = 0;
        self.counter = 0;
    }

    fn key_off(&mut self) {
        if self.phase != Phase::Off {
            self.phase = Phase::Release;
            self.counter = 0;
        }
    }

    pub fn level(&self) -> i32 {
        self.level
    }

    fn sustain_level(&self) -> i32 {
        (((self.adsr1 & 15) as i32) + 1) * 0x800
    }

    /// One step: (exponential, decreasing, shift, step).
    fn advance(&mut self, exp: bool, dec: bool, shift: u32, step_value: u32) {
        let rate = (shift << 2) | step_value;
        let mut step = 7 - step_value as i32;
        if dec {
            step = !step;
        }
        step <<= 11u32.saturating_sub(shift);
        let mut inc = 0x8000u32 >> shift.saturating_sub(11);
        if exp && !dec && self.level > 0x6000 {
            if shift < 10 {
                step >>= 2;
            } else if shift >= 11 {
                inc >>= 2;
            } else {
                step >>= 1;
                inc >>= 1;
            }
        } else if exp && dec {
            step = (step * self.level) >> 15;
        }
        if rate != 0x7f {
            inc = inc.max(1);
        }
        self.counter += inc;
        if self.counter & 0x8000 == 0 {
            return;
        }
        self.counter = 0;
        self.level += step;
        self.level = if dec { self.level.max(0) } else { self.level.clamp(-0x8000, 0x7fff) };
    }

    fn tick(&mut self) {
        let (a1, a2) = (self.adsr1 as u32, self.adsr2 as u32);
        match self.phase {
            Phase::Off => {}
            Phase::Attack => {
                self.advance(a1 & 0x8000 != 0, false, (a1 >> 10) & 0x1f, (a1 >> 8) & 3);
                if self.level >= 0x7fff {
                    self.phase = Phase::Decay;
                    self.counter = 0;
                }
            }
            Phase::Decay => {
                self.advance(true, true, (a1 >> 4) & 0xf, 0);
                if self.level <= self.sustain_level() {
                    self.phase = Phase::Sustain;
                    self.counter = 0;
                }
            }
            Phase::Sustain => {
                self.advance(a2 & 0x8000 != 0, a2 & 0x4000 != 0, (a2 >> 8) & 0x1f, (a2 >> 6) & 3);
            }
            Phase::Release => {
                self.advance(a2 & 0x20 != 0, true, a2 & 0x1f, 0);
                if self.level <= 0 {
                    self.level = 0;
                    self.phase = Phase::Off;
                }
            }
        }
    }
}

/// One voice's registers and state.
#[derive(Clone, Debug)]
pub struct Voice {
    pub vol_l: u16,
    pub vol_r: u16,
    pub pitch: u16,
    pub ssa: u32,
    pub lsa: u32,
    pub env: Envelope,
    /// The address of the frame being played.
    cur: u32,
    /// The frame's flags.
    flags: u8,
    /// Three samples of the previous frame, then this frame's 28.
    buf: [i16; 3 + PER_FRAME],
    hist: History,
    counter: u32,
    pub endx: bool,
    pub dry_l: bool,
    pub dry_r: bool,
    pub wet_l: bool,
    pub wet_r: bool,
}

impl Voice {
    fn new() -> Voice {
        Voice {
            vol_l: 0,
            vol_r: 0,
            pitch: 0,
            ssa: 0,
            lsa: 0,
            env: Envelope::new(),
            cur: 0,
            flags: 0,
            buf: [0; 3 + PER_FRAME],
            hist: History::default(),
            counter: 0,
            endx: false,
            dry_l: false,
            dry_r: false,
            wet_l: false,
            wet_r: false,
        }
    }

    /// Is the voice sounding (its envelope not finished)?
    pub fn active(&self) -> bool {
        self.env.phase != Phase::Off
    }

    fn decode(&mut self, ram: &[u8]) {
        let a = self.cur as usize & (RAM - 1) & !15;
        let frame = &ram[a..a + 16];
        self.flags = frame[1];
        if self.flags & adpcm::LOOP_START != 0 {
            self.lsa = self.cur;
        }
        let mut out = [0i16; PER_FRAME];
        adpcm::decode_frame(frame, &mut self.hist, &mut out);
        self.buf.copy_within(PER_FRAME..PER_FRAME + 3, 0);
        self.buf[3..].copy_from_slice(&out);
    }

    fn key_on(&mut self, ram: &[u8]) {
        self.cur = self.ssa;
        self.lsa = self.ssa;
        self.hist = History::default();
        self.buf = [0; 3 + PER_FRAME];
        self.counter = 0;
        self.endx = false;
        self.env.key_on();
        self.decode(ram);
    }

    /// The next output sample, before the left/right volumes.
    fn sample(&mut self, ram: &[u8]) -> i32 {
        let s = (self.counter >> 12) as usize;
        let i = ((self.counter >> 4) & 0xff) as usize;
        let g = |k: usize| GAUSS[k] as i32;
        let b = |k: usize| self.buf[k] as i32;
        let mut out = (g(0xff - i) * b(s)) >> 15;
        out += (g(0x1ff - i) * b(s + 1)) >> 15;
        out += (g(0x100 + i) * b(s + 2)) >> 15;
        out += (g(i) * b(s + 3)) >> 15;
        let out = (out * self.env.level) >> 15;

        let step = (self.pitch as u32).min(0x4000);
        self.counter += step;
        while (self.counter >> 12) as usize >= PER_FRAME {
            self.counter -= (PER_FRAME as u32) << 12;
            // Leaving the frame: its end flag jumps to the loop address; with
            // no repeat the voice is released and silenced.
            if self.flags & adpcm::END != 0 {
                self.endx = true;
                self.cur = self.lsa;
                if self.flags & adpcm::REPEAT == 0 {
                    self.env.phase = Phase::Off;
                    self.env.level = 0;
                }
            } else {
                self.cur = self.cur.wrapping_add(16);
            }
            self.decode(ram);
        }
        self.env.tick();
        out
    }
}

/// A fixed voice volume register: bits 0-14 are volume / 2.
fn fixed_volume(v: u16) -> i32 {
    if v & 0x8000 != 0 {
        // Sweep mode is not used by the synthesizer; hold the level.
        return 0x7fff;
    }
    (((v << 1) as i16) as i32).clamp(-0x8000, 0x7ffe)
}

/// The two cores.
pub struct Spu {
    pub ram: Vec<u8>,
    pub voices: Vec<Voice>,
    /// Core 1's master volume (the output), 0..0x3fff per side.
    pub master: (u16, u16),
    /// The sound-data input volume, 0..0x7fff per side.
    pub input_vol: (u16, u16),
    /// Core 0's master volume, 0x3fff from `spuInit`.
    pub master0: (u16, u16),
    /// The two cores' reverb units (hall, as `spuInit` sets them).
    pub reverb: [Reverb; 2],
    pub reverb_on: bool,
    /// When set, every register write, in order: for checking the
    /// synthesizer against what MODHSYN.IRX writes (`tools/iopemu.py`).
    pub log: Option<Vec<Write>>,
}

/// One libsd register write: `('p', entry, value)` for sceSdSetParam,
/// `('a', ...)` sceSdSetAddr, `('s', ...)` sceSdSetSwitch.
pub type Write = (char, u16, u32);

/// The libsd calls the synthesizer makes: the SPU2 itself, or a
/// [`Recorder`] for checking against the module.
pub trait Regs {
    fn set_param(&mut self, entry: u16, value: u16);
    fn get_param(&mut self, entry: u16) -> u16;
    fn set_switch(&mut self, entry: u16, value: u32);
    fn get_switch(&mut self, entry: u16) -> u32;
    fn set_addr(&mut self, entry: u16, value: u32);
    /// `sceSdSetCoreAttr`; only the noise clock reaches it.
    fn set_core_attr(&mut self, entry: u16, value: u16);
}

/// A register file that only remembers and logs, as `tools/iopemu.py`'s
/// libsd stand-ins do: a get returns the last value set (so ENVX and ENDX
/// read 0).
#[derive(Default)]
pub struct Recorder {
    pub log: Vec<Write>,
    values: std::collections::HashMap<(char, u16), u32>,
}

impl Regs for Recorder {
    fn set_param(&mut self, entry: u16, value: u16) {
        self.log.push(('p', entry, value as u32));
        self.values.insert(('p', entry), value as u32);
    }
    fn get_param(&mut self, entry: u16) -> u16 {
        self.values.get(&('p', entry)).copied().unwrap_or(0) as u16
    }
    fn set_switch(&mut self, entry: u16, value: u32) {
        self.log.push(('s', entry, value));
        self.values.insert(('s', entry), value);
    }
    fn get_switch(&mut self, entry: u16) -> u32 {
        self.values.get(&('s', entry)).copied().unwrap_or(0)
    }
    fn set_addr(&mut self, entry: u16, value: u32) {
        self.log.push(('a', entry, value));
        self.values.insert(('a', entry), value);
    }
    fn set_core_attr(&mut self, entry: u16, value: u16) {
        self.log.push(('c', entry, value as u32));
    }
}

impl Regs for Spu {
    fn set_param(&mut self, entry: u16, value: u16) {
        Spu::set_param(self, entry, value)
    }
    fn get_param(&mut self, entry: u16) -> u16 {
        Spu::get_param(self, entry)
    }
    fn set_switch(&mut self, entry: u16, value: u32) {
        Spu::set_switch(self, entry, value)
    }
    fn get_switch(&mut self, entry: u16) -> u32 {
        if entry >> 8 == switch::ENDX { self.endx((entry & 1) as usize) } else { 0 }
    }
    fn set_addr(&mut self, entry: u16, value: u32) {
        Spu::set_addr(self, entry, value)
    }
    fn set_core_attr(&mut self, _entry: u16, _value: u16) {
        // Noise voices are not modelled; no bank on the disc has one.
    }
}

impl Default for Spu {
    fn default() -> Self {
        Spu::new()
    }
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            ram: vec![0; RAM],
            voices: vec![Voice::new(); 2 * VOICES],
            master: (0x3fff, 0x3fff),
            input_vol: (0x7fff, 0x7fff),
            master0: (0x3fff, 0x3fff),
            reverb: [Reverb::hall(), Reverb::hall()],
            reverb_on: true,
            log: None,
        }
    }

    /// Copy sample data into sound RAM (`sceSdVoiceTrans`).
    pub fn write_ram(&mut self, at: usize, data: &[u8]) {
        let end = (at + data.len()).min(RAM);
        self.ram[at..end].copy_from_slice(&data[..end - at]);
    }

    fn voice_index(entry: u16) -> usize {
        let core = (entry & 1) as usize;
        let v = ((entry >> 1) & 31) as usize;
        core * VOICES + v.min(VOICES - 1)
    }

    /// `sceSdSetParam` for the voice parameters.
    pub fn set_param(&mut self, entry: u16, value: u16) {
        if let Some(l) = &mut self.log {
            l.push(('p', entry, value as u32));
        }
        if entry & 0x80 != 0 {
            // Master parameters: 0x09 MVOLL, 0x0a MVOLR; core 1 is the output.
            if entry & 1 == 1 {
                match entry >> 8 {
                    0x09 => self.master.0 = value,
                    0x0a => self.master.1 = value,
                    _ => {}
                }
            }
            return;
        }
        let v = &mut self.voices[Spu::voice_index(entry)];
        match entry >> 8 {
            param::VOLL => v.vol_l = value,
            param::VOLR => v.vol_r = value,
            param::PITCH => v.pitch = value,
            param::ADSR1 => v.env.adsr1 = value,
            param::ADSR2 => v.env.adsr2 = value,
            _ => {}
        }
    }

    pub fn get_param(&self, entry: u16) -> u16 {
        let v = &self.voices[Spu::voice_index(entry)];
        match entry >> 8 {
            param::VOLL => v.vol_l,
            param::VOLR => v.vol_r,
            param::PITCH => v.pitch,
            param::ADSR1 => v.env.adsr1,
            param::ADSR2 => v.env.adsr2,
            param::ENVX => v.env.level.clamp(0, 0x7fff) as u16,
            _ => 0,
        }
    }

    /// `sceSdSetAddr` for the voice addresses (bytes in sound RAM).
    pub fn set_addr(&mut self, entry: u16, value: u32) {
        if let Some(l) = &mut self.log {
            l.push(('a', entry, value));
        }
        let v = &mut self.voices[Spu::voice_index(entry)];
        match entry >> 8 {
            addr::SSA => v.ssa = value,
            addr::LSAX => v.lsa = value,
            _ => {}
        }
    }

    /// `sceSdSetSwitch`: a 24-bit mask of one core's voices.
    pub fn set_switch(&mut self, entry: u16, mask: u32) {
        if let Some(l) = &mut self.log {
            l.push(('s', entry, mask));
        }
        let core = (entry & 1) as usize;
        for k in 0..VOICES {
            let on = mask & (1 << k) != 0;
            let v = &mut self.voices[core * VOICES + k];
            match entry >> 8 {
                switch::KON if on => v.key_on(&self.ram),
                switch::KOFF if on => v.env.key_off(),
                switch::VMIXL => v.dry_l = on,
                switch::VMIXR => v.dry_r = on,
                switch::VMIXEL => v.wet_l = on,
                switch::VMIXER => v.wet_r = on,
                _ => {}
            }
        }
    }

    /// ENDX: which of a core's voices have passed an end flag since key on.
    pub fn endx(&self, core: usize) -> u32 {
        (0..VOICES).filter(|&k| self.voices[core * VOICES + k].endx).fold(0, |m, k| m | 1 << k)
    }

    /// One output sample: every voice, plus `input` (the sound-data input's
    /// stereo sample), through the master volume.
    /// One output sample. Per core, as `spuInit` sets the mixers (MMIX
    /// core 0 = 0x0fc0, core 1 = 0x0fcc): the voices dry and, through the
    /// core's reverb, wet; core 0 adds the sound-data input (dry) and goes
    /// through its master volume into core 1 (dry), whose master volume is
    /// the output.
    pub fn sample(&mut self, input: (i16, i16)) -> (i16, i16) {
        let mut dry = [[0i32; 2]; 2];
        let mut wet = [[0i32; 2]; 2];
        let ram = &self.ram;
        for (k, v) in self.voices.iter_mut().enumerate() {
            if !v.active() {
                continue;
            }
            let core = k / VOICES;
            let s = v.sample(ram);
            let l = (s * fixed_volume(v.vol_l)) >> 15;
            let r = (s * fixed_volume(v.vol_r)) >> 15;
            if v.dry_l {
                dry[core][0] += l;
            }
            if v.dry_r {
                dry[core][1] += r;
            }
            if v.wet_l {
                wet[core][0] += l;
            }
            if v.wet_r {
                wet[core][1] += r;
            }
        }
        let clamp = |x: i32| x.clamp(-0x8000, 0x7fff);
        let mut ext = (0, 0);
        for core in 0..2 {
            let (rl, rr) = if self.reverb_on {
                self.reverb[core].sample((clamp(wet[core][0]), clamp(wet[core][1])))
            } else {
                (0, 0)
            };
            let mut l = dry[core][0] + rl + ext.0;
            let mut r = dry[core][1] + rr + ext.1;
            if core == 0 {
                l += (input.0 as i32 * self.input_vol.0 as i32) >> 15;
                r += (input.1 as i32 * self.input_vol.1 as i32) >> 15;
            }
            let mv = if core == 0 { self.master0 } else { self.master };
            ext = ((clamp(l) * fixed_volume(mv.0)) >> 15, (clamp(r) * fixed_volume(mv.1)) >> 15);
        }
        (ext.0 as i16, ext.1 as i16)
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauss_rows_sum_to_255_256ths() {
        for i in 0..256 {
            let s: i32 = [i, 0xff - i, 0x100 + i, 0x1ff - i].iter().map(|&k| GAUSS[k] as i32).sum();
            assert!((0x7f7f..=0x7f81).contains(&s), "{i}: {s:x}");
        }
    }

    #[test]
    fn attack_decay_sustain_release() {
        let mut e = Envelope::new();
        // Linear attack at the fastest rate, decay shift 0 to sustain level 7.
        e.adsr1 = 0x0007;
        e.adsr2 = 0x1fc0 | 0x0a;
        e.key_on();
        let mut n = 0;
        while e.phase == Phase::Attack {
            e.tick();
            n += 1;
        }
        assert!(n < 64, "{n}");
        while e.phase == Phase::Decay {
            e.tick();
        }
        assert!(e.level <= 0x4000 && e.level > 0x3000, "{:x}", e.level);
        e.key_off();
        let mut n = 0;
        while e.phase != Phase::Off {
            e.tick();
            n += 1;
        }
        assert!(n > 100, "{n}");
        assert_eq!(e.level, 0);
    }

    #[test]
    fn one_frame_at_unit_pitch() {
        let mut spu = Spu::new();
        // A frame of 28 constant nibbles (shift 12, filter 0: 3 each), then
        // an end frame that loops on itself.
        let mut f = vec![12u8, 0x04];
        f.extend([0x33u8; 14]);
        let mut g = vec![12u8, 0x07];
        g.extend([0x33u8; 14]);
        f.extend(g);
        spu.write_ram(0x1000, &f);
        spu.set_addr(0x2040, 0x1000);
        spu.set_param(0x0000, 0x3fff);
        spu.set_param(0x0100, 0x3fff);
        spu.set_param(0x0200, 0x1000);
        spu.set_param(0x0300, 0x000f);
        spu.set_param(0x0400, 0x1fc0);
        spu.set_switch(0x1800, 1);
        spu.set_switch(0x1a00, 1);
        spu.set_switch(0x1500, 1);
        assert!(spu.voices[0].active());
        let out: Vec<(i16, i16)> = (0..400).map(|_| spu.sample((0, 0))).collect();
        let bad: Vec<_> =
            out.iter().enumerate().filter(|&(_, &(l, r))| !(l == r && (-1..=4).contains(&l))).take(5).collect();
        assert!(bad.is_empty(), "{bad:?}");
        assert!(spu.voices[0].active());
    }
}
