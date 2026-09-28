//! Mutation's stream effect tasks after its opening ([`crate::opening`]):
//! the `StreamDemoFuncTbl` rows (MUT main 0x00367210) Infection lacks. Each
//! keeps the [`Ctrl`] and pass of Infection's tasks; they differ in the fog
//! they set, the buffer shades and the cues (`docs/engine/stream.md`,
//! "Mutation's other effect tasks").

use piney_desktop::noiz::Sampling;
use piney_world::ee::{self, F};

use crate::effect::{Ctrl, Cue, Draws, EFFECT_LAYER, FADE_WHITE, HitMark, MAX_CUES, Rand, Tables, Transfer, hit_mark};
use crate::scene::SceneFog;

/// `SetFog(0, 0, 0, 0, 0)`: no fog (the EE's 0 / 0 is the largest float).
pub fn no_fog() -> SceneFog {
    SceneFog::with_rates(0.0, 0.0, 0.0, 0.0, 0)
}

pub const STR0770: &str = "str0770";
pub const STR0780: &str = "str0780";
pub const STR0820: &str = "str0820";
pub const STR0821: &str = "str0821";
pub const STR0880: &str = "str0880";
pub const STR0885: &str = "str0885";
pub const STR0932: &str = "str0932";
pub const STR1040: &str = "str1040";
pub const STR1041: &str = "str1041";
pub const STR1050: &str = "str1050";
pub const STR1090: &str = "str1090";
pub const STR9204: &str = "str9204";
pub const STR9205: &str = "str9205";
pub const STR9206: &str = "str9206";
/// Mutation's rows that run `Func_str9101` ([`crate::effect::Str9001`]).
pub const STR9201: &str = "str9201";
pub const STR9301: &str = "str9301";

/// Mutation's `charHeightTbl` (main 0x00365b40) rows the transfers take:
/// 0 (160), 6 (180), 15 (165) and 17 (210).
pub const HEIGHT_0: u32 = 0x4320_0000;
pub const HEIGHT_6: u32 = 0x4334_0000;
pub const HEIGHT_15: u32 = 0x4325_0000;
pub const HEIGHT_17: u32 = 0x4352_0000;

/// `SetFog(0, 4000, 0, 50, 0x008c5000)` on the scene's draw environment:
/// streams 25 and 26.
pub const FOG_0770: (f32, f32, f32, f32, u32) = (0.0, 4000.0, 0.0, 50.0, 0x008c_5000);

/// `SetFog(0, 4000, 0, 60, 0x0078145a)`: `Func_str0880`'s.
pub const FOG_0880: (f32, f32, f32, f32, u32) = (0.0, 4000.0, 0.0, 60.0, 0x0078_145a);

/// `SetFog(200, 4000, 0, 60, 0x0019140a)`: `Func_str0932`'s.
pub const FOG_0932: (f32, f32, f32, f32, u32) = (200.0, 4000.0, 0.0, 60.0, 0x0019_140a);

const SHADE_COLOUR: u32 = 0x4080_8080;

/// Two shades, `SetShade(0, 896, 0, 8 - i, 8 - i, z_i, 0x40808080)`, the
/// depth `z` then `z * k`; both drawn.
fn shades(c: &mut Ctrl, z: F, k: F) {
    let sh = &mut c.shades;
    sh.list.clear();
    let mut z = z;
    for i in 0..2u8 {
        sh.list.push(Sampling::shade_own(8 - i, 8 - i, z, SHADE_COLOUR));
        z = ee::mul(z, k);
    }
    sh.count = 2;
}

/// Cue 899: a fade to black over the frames left (1 at the end).
fn fade_out(c: &mut Ctrl, frame_now: u32, frame_end: u32) {
    let left = frame_end.wrapping_sub(frame_now);
    c.fade.entry(if left != 0 { left } else { 1 }, 0, 0x8000_0000);
}

/// Cue 999: a fade to white over the frames left (1 at the end).
fn fade_white(c: &mut Ctrl, frame_now: u32, frame_end: u32) {
    let left = frame_end.wrapping_sub(frame_now);
    c.fade.entry(if left != 0 { left } else { 1 }, FADE_WHITE.0, FADE_WHITE.1);
}

/// `Func_str0770` (MUT main 0x0019df80), stream 25.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0770 {
    pub ctrl: Ctrl,
}

impl Str0770 {
    /// Started: the objects (the noise bands' `Init` drawing from `rand`)
    /// and the shades at depths 310 and 620; the fog is the caller's
    /// ([`FOG_0770`]).
    pub fn new(rand: &mut Rand) -> Str0770 {
        let mut ctrl = Ctrl::new(Some(EFFECT_LAYER), rand);
        shades(&mut ctrl, 0x439b_0000, 0x4000_0000);
        Str0770 { ctrl }
    }

    /// One pass: cue 899, then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        for &Cue { param, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            if param == 899 {
                fade_out(&mut self.ctrl, frame_now, frame_end);
            }
        }
        self.ctrl.pass(paused, rand)
    }
}

/// `Func_str0780` (MUT main 0x0019e9e0), stream 26.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0780 {
    pub ctrl: Ctrl,
}

impl Str0780 {
    /// Started as [`Str0770::new`], the shades at depths 1000 and 3000.
    pub fn new(rand: &mut Rand) -> Str0780 {
        let mut ctrl = Ctrl::new(Some(EFFECT_LAYER), rand);
        shades(&mut ctrl, 0x447a_0000, 0x4040_0000);
        Str0780 { ctrl }
    }

    /// One pass: at scene frames 287 and 2540 the shades at 310 and 620,
    /// at 2469 at 800 and 1600; cue 899; then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        match frame_now {
            287 | 2540 => shades(c, 0x439b_0000, 0x4000_0000),
            2469 => shades(c, 0x4448_0000, 0x4000_0000),
            _ => {}
        }
        for &Cue { param, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            if param == 899 {
                fade_out(c, frame_now, frame_end);
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0820` (MUT main 0x0019f5d0), stream 27's first scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0820 {
    pub ctrl: Ctrl,
    pub transfers: Vec<Transfer>,
}

impl Str0820 {
    /// Started: the objects, and white held for a frame.
    pub fn new(rand: &mut Rand) -> Str0820 {
        let mut ctrl = Ctrl::new(Some(EFFECT_LAYER), rand);
        ctrl.fade.entry(1, FADE_WHITE.1, FADE_WHITE.1);
        Str0820 { ctrl, transfers: Vec::new() }
    }

    /// One pass: the cues, last queued first - 500 and 510 a transfer at
    /// the note's object, 900 a white flash over 20 frames, 999 to white,
    /// 5 a grey flash over 10 and the feedback, 15 / 25 the feedback with
    /// a fade over 20 ready, 6 / 16 / 26 the feedback fading; then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                500 => self.transfers.push(Transfer { obj, height: HEIGHT_0 }),
                510 => self.transfers.push(Transfer { obj, height: HEIGHT_15 }),
                900 => c.fade.entry(20, FADE_WHITE.1, FADE_WHITE.0),
                999 => fade_white(c, frame_now, frame_end),
                5 => {
                    c.fade.entry(10, 0x80c0_c0c0, 0x00c0_c0c0);
                    c.feedback.reflex(0x3f81_47ae, 0x3480_8080);
                }
                15 | 25 => c.feedback.reflex_timed(0x3f81_47ae, 0x5080_8080, 0, 20),
                6 | 16 | 26 => c.feedback.flag = 2,
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0821` (MUT main 0x001a03a0), stream 27's second scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0821 {
    pub ctrl: Ctrl,
    pub transfers: Vec<Transfer>,
}

impl Str0821 {
    /// Started as [`Str0820::new`].
    pub fn new(rand: &mut Rand) -> Str0821 {
        let mut ctrl = Ctrl::new(Some(EFFECT_LAYER), rand);
        ctrl.fade.entry(1, FADE_WHITE.1, FADE_WHITE.1);
        Str0821 { ctrl, transfers: Vec::new() }
    }

    /// One pass: at scene frame 2 the white fading over 20 frames; the
    /// cues - 501 and 502 a transfer, 900 a white flash over 20, 899 to
    /// black; then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        if frame_now == 2 {
            c.fade.entry(20, FADE_WHITE.1, FADE_WHITE.0);
        }
        for &Cue { param, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                501 => self.transfers.push(Transfer { obj, height: HEIGHT_17 }),
                502 => self.transfers.push(Transfer { obj, height: HEIGHT_15 }),
                900 => c.fade.entry(20, FADE_WHITE.1, FADE_WHITE.0),
                899 => fade_out(c, frame_now, frame_end),
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0880` (MUT main 0x001a1050), for `str0880` and `str0885`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0880 {
    pub ctrl: Ctrl,
    rot: Vec<[u32; 3]>,
    marks_n: usize,
    pub marks: Vec<HitMark>,
}

impl Str0880 {
    /// Started: the objects; the fog is the caller's ([`FOG_0880`]).
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str0880 {
        Str0880 {
            ctrl: Ctrl::new(Some(EFFECT_LAYER), rand),
            rot: tables.hit_rot_0880.clone(),
            marks_n: 0,
            marks: Vec::new(),
        }
    }

    /// One pass: the cues, last queued first - 601-604 a hit mark at the
    /// note's object, each taking the next rotation; else the last digit
    /// (jump table MUT main 0x00366f10): 1 noise on, 2 off, 3 inversion on,
    /// 4 off, 5 the feedback, 6 off - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                601..=604 => {
                    self.marks.push(hit_mark(&self.rot, obj, self.marks_n));
                    self.marks_n += 1;
                }
                _ => match param % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(0x3f81_eb85, 0x5080_8080),
                    6 => c.feedback.off(),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0932` (MUT main 0x001a1cb0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0932 {
    pub ctrl: Ctrl,
    /// The last pass set [`no_fog`] on the scene, for the host.
    pub fog_off: bool,
}

impl Str0932 {
    /// Started: the objects; the fog is the caller's ([`FOG_0932`]).
    pub fn new(rand: &mut Rand) -> Str0932 {
        Str0932 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), fog_off: false }
    }

    /// One pass: at scene frame 1700 `SetFog(0, 0, 0, 0, 0)` (no fog);
    /// the cues, last queued first - 899 to black over the frames left;
    /// else the last digit: 1 noise on, 2 off, 5 the feedback, 6 off - then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        if frame_now == 1700 {
            self.fog_off = true;
        }
        for &Cue { param, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                899 => fade_out(c, frame_now, frame_end),
                _ => match param % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    5 => c.feedback.reflex(0x3f82_8f5c, 0x4880_8080),
                    6 => c.feedback.off(),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str1040`'s texts: stream 31's subtitle records 16-19 (the
/// cues 700-730), four lines each (the fourth empty in the table), none
/// where the volume has no such records.
pub fn text_1040(volume: piney_data::volume::Volume) -> [[Vec<Vec<u8>>; 2]; 4] {
    let t = piney_data::tables::stream::of(volume);
    let rows = |parody: bool, k: usize| {
        let table = if parody { t.subtitles_parody() } else { t.subtitles() };
        let r = table.get(31).copied().flatten().and_then(|r| r.get(16 + k));
        let l = r.and_then(|r| r.str).unwrap_or_default();
        (0..4).map(|i| l.get(i).map_or_else(Vec::new, |s| piney_data::tables::sjis::encode(s))).collect()
    };
    std::array::from_fn(|k| [rows(false, k), rows(true, k)])
}

/// `Func_str1040`'s shades: `SetShade(0, 896, 0, 8 - i, 8 - i, z_i,
/// colour)`, the depth `z` then `z * k`; a fade in progress goes on.
fn shades_1040(c: &mut Ctrl, z: F, k: F, colour: u32) {
    let sh = &mut c.shades;
    sh.list.clear();
    let mut z = z;
    for i in 0..2u8 {
        sh.list.push(Sampling::shade_own(8 - i, 8 - i, z, colour));
        z = ee::mul(z, k);
    }
    sh.count = 2;
}

/// `Func_str1040` (MUT main 0x001a2880), stream 31.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str1040 {
    pub ctrl: Ctrl,
    /// The two blocks of lines (0x298 bytes each, as the opening's) and
    /// the one the next text goes to (+0x12).
    pub text: [crate::opening::TextLines; 2],
    pub cur: usize,
    lines: [Vec<Vec<u8>>; 4],
    texts: Vec<crate::opening::TextDraw>,
}

impl Str1040 {
    /// Started: the objects, both blocks at alpha 0; the texts by
    /// `saveData.parodyFlag`.
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str1040 {
        let p = usize::from(tables.parody);
        Str1040 {
            ctrl: Ctrl::new(Some(EFFECT_LAYER), rand),
            text: Default::default(),
            cur: 0,
            lines: std::array::from_fn(|k| tables.text_1040[k][p].clone()),
            texts: Vec::new(),
        }
    }

    /// The text lines of the last pass, block 0's then block 1's.
    pub fn text_draws(&self) -> &[crate::opening::TextDraw] {
        &self.texts
    }

    /// One pass: the scene frame's shades (340 fading in over 20 frames;
    /// off at 621, 1586, 2106, 2961, 3411 and 4416); the cues, last queued
    /// first - 700-730 a text into the current block, 709-739 its fade out
    /// and the other block current, 800-829 fades; [`Ctrl::pass`]; then
    /// both blocks' fades.
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        const TWO: F = 0x4000_0000;
        const ONE_POINT_SIX: F = 0x3fcc_cccd;
        match frame_now {
            340 => {
                shades_1040(c, 0x439b_0000, TWO, 0x0080_8080);
                c.shades.fade_cnt = 20;
                c.shades.fade_rate = ((0 - 96) << 16) / 20;
                c.shades.fade_base = 96;
            }
            1331 | 1752 => shades_1040(c, 0x447a_0000, ONE_POINT_SIX, 0x4080_8080),
            1676 => shades_1040(c, 0x4316_0000, TWO, 0x5080_8080),
            2400 => shades_1040(c, 0x43a7_8000, TWO, 0x5080_8080),
            2566 => shades_1040(c, 0x443b_8000, TWO, 0x5080_8080),
            3170 => shades_1040(c, 0x44bb_8000, TWO, 0x4080_8080),
            4146 | 4306 | 4725 => shades_1040(c, 0x44bb_8000, ee::ONE, 0x5080_8080),
            621 | 1586 | 2106 | 2961 | 3411 | 4416 => c.shades.count = 0,
            _ => {}
        }
        for &Cue { param, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                700 | 710 | 720 | 730 => {
                    let t = &mut self.text[self.cur];
                    t.set_lines(&self.lines[(param as usize - 700) / 10]);
                    t.place();
                }
                709 | 719 | 729 | 739 => {
                    self.text[self.cur].hide();
                    self.cur ^= 1;
                }
                809 => c.fade.entry(20, 0, 0x8000_0000),
                800 => c.fade.entry(20, 0x8000_0000, 0),
                819 => c.fade.entry(20, 0, 0x4000_0000),
                801 => c.fade.entry(20, 0x4000_0000, 0),
                829 => fade_out(c, frame_now, frame_end),
                _ => {}
            }
        }
        let d = c.pass(paused, rand);
        self.texts.clear();
        for t in &mut self.text {
            t.fade();
            self.texts.extend(t.shown().iter().map(|l| crate::opening::TextDraw {
                text: l.text.clone(),
                x: l.x,
                y: l.y,
                alpha: l.alpha,
            }));
        }
        d
    }
}

/// `Func_str1041` (MUT main 0x001a4930), stream 32.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str1041 {
    pub ctrl: Ctrl,
}

impl Str1041 {
    /// Started: the objects, and black held for a frame.
    pub fn new(rand: &mut Rand) -> Str1041 {
        let mut ctrl = Ctrl::new(Some(EFFECT_LAYER), rand);
        ctrl.fade.entry(1, 0x8000_0000, 0x8000_0000);
        Str1041 { ctrl }
    }

    /// One pass: at scene frame 2 the black fading over 30 frames; the
    /// cues, last queued first - 899 to black over the frames left, 800
    /// nothing; else the last digit (jump table MUT main 0x00366f30): 1
    /// noise and the feedback on, 2 noise off, 3 inversion on, 4 off, 5 a
    /// fainter feedback, 6 off - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        if frame_now == 2 {
            c.fade.entry(30, 0x8000_0000, 0);
        }
        for &Cue { param, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                899 => fade_out(c, frame_now, frame_end),
                800 => {}
                _ => match param % 10 {
                    1 => {
                        c.noise_start(rand);
                        c.feedback.reflex(0x3f82_8f5c, 0x4080_8080);
                    }
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(0x3f81_eb85, 0x4880_8080),
                    6 => c.feedback.off(),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str1050` (MUT main 0x001a5640), stream 33.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str1050 {
    pub ctrl: Ctrl,
    pub transfers: Vec<Transfer>,
}

impl Str1050 {
    /// Started: the objects; nothing on.
    pub fn new(rand: &mut Rand) -> Str1050 {
        Str1050 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), transfers: Vec::new() }
    }

    /// One pass: the cues, last queued first - 7 and 9 a transfer at the
    /// note's object, 899 to black over the frames left; else the last
    /// digit (jump table MUT main 0x00366f50): 1 noise on, 2 off, 3
    /// inversion on, 4 off, 5 the feedback, 6 off - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                899 => fade_out(c, frame_now, frame_end),
                9 => self.transfers.push(Transfer { obj, height: HEIGHT_6 }),
                7 => self.transfers.push(Transfer { obj, height: HEIGHT_17 }),
                _ => match param % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(0x3f81_47ae, 0x4880_8080),
                    6 => c.feedback.off(),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str1090` (MUT main 0x001a8570), stream 35.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str1090 {
    pub ctrl: Ctrl,
    pub transfers: Vec<Transfer>,
}

impl Str1090 {
    /// Started: the objects; nothing on.
    pub fn new(rand: &mut Rand) -> Str1090 {
        Str1090 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), transfers: Vec::new() }
    }

    /// One pass: the cues, last queued first - 500 and 501 a transfer at
    /// the note's object, 900-950 a white flash over 6 frames, 809-859 a
    /// fade to 0x60 black over 40, 869 black at once; else the last digit:
    /// 5 a faint feedback, 6 off - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                869 => c.fade.entry(1, 0, 0x8000_0000),
                809 | 819 | 829 | 839 | 849 | 859 => c.fade.entry(40, 0, 0x6000_0000),
                900 | 910 | 920 | 930 | 940 | 950 => c.fade.entry(6, FADE_WHITE.1, FADE_WHITE.0),
                500 => self.transfers.push(Transfer { obj, height: HEIGHT_0 }),
                501 => self.transfers.push(Transfer { obj, height: HEIGHT_15 }),
                _ => match param % 10 {
                    5 => c.feedback.reflex(0x3f81_47ae, 0x3080_8080),
                    6 => c.feedback.off(),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str9204` (MUT main 0x001a92a0), for `str9204`-`str9206`: as
/// Infection's `Func_str9000` ([`crate::effect::Str9000`]) with no banner
/// and its own cues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str9204 {
    pub ctrl: Ctrl,
}

impl Str9204 {
    /// Started: the scene's view letterboxed ([`crate::effect::FRAME_9000`];
    /// the fog, [`crate::effect::FOG_9000`], is the caller's), then the
    /// objects through the letterbox.
    pub fn start(scene: &mut crate::scene::Scene, rand: &mut Rand) -> Str9204 {
        scene.frame = crate::effect::FRAME_9000;
        Str9204 { ctrl: Ctrl::framed(Some(EFFECT_LAYER), crate::effect::FRAME_9000, rand) }
    }

    /// One pass: the cues, last queued first, by their value (jump table
    /// MUT main 0x00367030): 1 noise on, 2 off, 3 inversion on, 4 off, 5
    /// the feedback with a 60-frame fade ready, 6 fading - then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match param {
                1 => c.noise_start(rand),
                2 => c.noise_on = false,
                3 => c.reverse = true,
                4 => c.reverse = false,
                5 => c.feedback.reflex_timed(0x3f81_47ae, 0x5880_8080, 0, 60),
                6 => c.feedback.flag = 2,
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shades_double_and_triple() {
        let mut r = Rand::default();
        let s = Str0780::new(&mut r);
        let z: Vec<u32> = s.ctrl.shades.list.iter().map(|s| s.z).collect();
        assert_eq!(z, [0x447a_0000, 0x453b_8000]);
        let s = Str0770::new(&mut r);
        let z: Vec<u32> = s.ctrl.shades.list.iter().map(|s| s.z).collect();
        assert_eq!(z, [0x439b_0000, 0x441b_0000]);
    }
}
