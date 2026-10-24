//! Quarantine's field staff roll, `STFROLL_VOL4`: the thread event 314's
//! `ending_kanji` starts in field 126 (QUA gcmn 0x004f0950, priority 66)
//! and the object it runs (0x150 bytes, no symbol). Each frame its main
//! (0x004f1880) scrolls the credits, fades the logos, plays the ring and
//! the book (`ending`'s `ANM_ring_anm_2`, `ANM_book_anm2`), makes the
//! particle generators and moves the comets they follow. The thread ends
//! (its tscb's +0x14) once the music has stopped. docs/engine/stfroll.md.
//!
//! The object's own fields are named by their offsets; the vectors
//! (`{?, size, data}` at +0x00, +0x0c, +0x18) are `Vec`s.

use piney_data::tables::stfroll::{StfRoll as Table, StfRollLine};

use crate::ee::{self, F, ONE, V4};
use crate::effect::EffectCtrl;
use crate::particle::{FfParam, GenParam, GenRef, Generator};
use crate::{Cx, VecRef, vu};

/// `ending`: the scene file the roll's animations and textures come from.
pub const CCS: &str = "ending";
/// The kanji lines it keeps (+0x104).
pub const LINES: usize = 17;
/// `ccLayer::Init(129, sysLayer's view)`: the credits' layer.
pub const TEXT_LAYER: i16 = 129;
/// The anchors the comets' positions are published under.
const ANCHOR: u32 = 0x5354_0000;

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_PI: F = 0xc049_0fdb;

/// The two animations (+0x48, `new ccAnm[2]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anim {
    Ring,
    Book,
}

impl Anim {
    pub fn clip(self) -> &'static str {
        match self {
            Anim::Ring => "ANM_ring_anm_2",
            Anim::Book => "ANM_book_anm2",
        }
    }
}

/// A `ccAnm` as the roll uses one: its clip's time and step, whether
/// `SetAnm` found the clip (+0xac), and its +0x88 transparency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anm {
    /// `frameNow * 256 + frameCnt`.
    pub time: u32,
    /// +0x9c `frameSpd`.
    pub spd: u16,
    pub found: bool,
    pub transparency: F,
}

/// What the roll's animations are: their lengths and whether they loop,
/// for `_AnimateForward`; None where the scene file lacks the clip.
pub trait Clips {
    fn clip(&self, a: Anim) -> Option<(u32, bool)>;
}

impl Clips for piney_desktop::assets::SceneFile {
    fn clip(&self, a: Anim) -> Option<(u32, bool)> {
        let an = &self.anims[self.anim(a.clip())?];
        Some((an.frames, an.looping))
    }
}

/// A comet (0x80 bytes): four path points, the place along them, its
/// generators and where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Comet {
    /// The anchor its position is published under.
    pub key: u32,
    /// +0x00 .. +0x30 the path's points.
    pub points: [V4; 4],
    /// +0x40 `t` in the segment, +0x44 the segment (4: done), +0x48 the
    /// step a frame, +0x4c frames held before it moves.
    pub t: F,
    pub seg: i32,
    pub speed: F,
    pub delay: i32,
    /// +0x50 it has a main generator (+0x60); +0x54 it is done; +0x58
    /// how its generators end (0 killed, 1 deleted); +0x5c frames since
    /// it arrived.
    pub has_gen: i32,
    pub dead: i32,
    pub kill_mode: i32,
    pub age: i32,
    pub main_gen: Option<GenRef>,
    /// +0x64 the generators that follow it besides.
    pub subs: Vec<GenRef>,
    /// +0x70 where it is.
    pub pos: V4,
}

impl Comet {
    /// The constructor (0x004f3890).
    fn new(key: u32) -> Comet {
        Comet {
            key,
            points: [ee::VF0; 4],
            t: 0,
            seg: 0,
            speed: 0,
            delay: 0,
            has_gen: 0,
            dead: 0,
            kill_mode: 0,
            age: 0,
            main_gen: None,
            subs: Vec::new(),
            pos: ee::VF0,
        }
    }

    /// 0x004f37f0: point `i`, or (-1) the position.
    fn set(&mut self, i: i32, v: V4) {
        match usize::try_from(i) {
            Ok(i) => self.points[i] = v,
            Err(_) => self.pos = v,
        }
    }
}

/// One line of text the roll keeps: its table (the credits, or the last
/// page's) and row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineRef {
    pub end: bool,
    pub row: usize,
}

/// What a frame of the roll draws, in its order.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    /// `ccKanji::Disp(text, -1, 1, 1)` of kanji `k` at (x, y) on
    /// [`TEXT_LAYER`].
    Line { k: usize, x: F, y: F, text: &'static str },
    /// The `ccMask` (`TEX_hackrogo`, `TEX_hackrogo_2` or `TEX_hack_url`)
    /// over the screen on sysLayer: 512 x 384 of its 512 x 512 cell.
    Logo { tex: &'static str, transp: F },
    /// `ccAnm::Draw` of an animation at its time, its matrix as the
    /// `SetMatrix_*` left it.
    Anm { anim: Anim, time: u32, matrix: vu::M4, transparency: F },
}

/// What the roll asks of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// `ccBgmPlay(n)`.
    Bgm(i32),
    /// `ccBgmStop()`.
    BgmStop,
    /// `changeCamera(3)`, `cameraSetPos(pos, 3)`, `cameraSetView(view, 3)`.
    Camera { pos: V4, view: V4 },
}

/// The roll's object.
#[derive(Clone, Debug, PartialEq)]
pub struct StfRoll {
    /// The rows as the roll rewrites their `y` (the game writes into its
    /// tables): the credits and the last page.
    pub rows: [Vec<StfRollLine>; 2],
    /// +0x00 the lines kept, first on top.
    pub lines: Vec<LineRef>,
    /// +0x0c the comets.
    pub comets: Vec<Comet>,
    /// +0x18 the dead comets this frame, by their place in +0x0c when
    /// found (not moved as the earlier ones go).
    pub dead: Vec<usize>,
    /// +0x24 the last comet (the book follows it): its key.
    pub last: Option<u32>,
    /// Where it was when it was last there (the book reads the freed
    /// comet's +0x70, which nothing reuses).
    pub last_pos: V4,
    /// +0x28 the credits' scroll a frame; +0x2c the logo's hold.
    pub scroll: F,
    pub hold: i32,
    /// +0x30 the book's wait; +0x34 the book drawn once in phase 4.
    pub book_wait: i32,
    pub book_drawn: i32,
    /// +0x38's texture.
    pub logo: &'static str,
    /// +0x48 the ring and the book.
    pub anms: [Anm; 2],
    /// +0x50 the camera's view, +0x60 its eye, +0x70 the ring's place,
    /// +0x80 its turn.
    pub view: V4,
    pub eye: V4,
    pub pos: V4,
    pub rot: V4,
    /// +0xd0, +0xd4 the ring's two generators.
    pub glows: [Option<GenRef>; 2],
    /// +0x100 the next line's row, +0x108 done, +0x10c and +0x110 the
    /// lines' spacing, +0x114 the table's end reached.
    pub row: usize,
    pub done: bool,
    pub spacing: (i32, i32),
    pub at_end: bool,
    /// +0x118 the roll's state (0 the first logo, 1 the credits, 2 the
    /// second logos, 3 after), +0x11c the ring's and book's phase.
    pub state: i32,
    pub phase: i32,
    /// +0x120 the logo's fade a frame, +0x124 its transparency.
    pub fade: F,
    pub alpha: F,
    /// +0x128 the phase's frame count (to 31), +0x12c, +0x130, +0x134 the
    /// late comets' timer, count and limit, +0x138 the ring's animation
    /// over, +0x13c the logo's wait, +0x140 the music's last frames.
    pub count: i32,
    pub spawn: i32,
    pub spawned: i32,
    pub spawn_max: i32,
    pub ring_done: i32,
    pub logo_wait: i32,
    pub tail: i32,
    /// Comets made, for their anchors.
    pub serial: u32,
}

/// `ccRand()`: the Mersenne Twister's next word, as a signed int.
fn cc_rand(cx: &mut Cx) -> i32 {
    cx.host.genrand() as i32
}

/// `ccRandF(x)`.
fn rand_f(cx: &mut Cx, x: F) -> F {
    crate::portal::rand_f(cx.host, x)
}

/// `range[abs(ccRand() % 6)]`, then `ccRandF` of it.
fn pick(cx: &mut Cx, range: &[f32]) -> F {
    let i = (cc_rand(cx) % 6).unsigned_abs() as usize;
    let x = range.get(i).copied().unwrap_or(0.0).to_bits();
    rand_f(cx, x)
}

/// Generator `row` of the table with force fields `ffs` (rows; none for
/// NULL).
fn generator(cx: &mut Cx, t: &Table, row: usize, ffs: [Option<usize>; 4]) -> Generator {
    let p = GenParam::other(t.gens, t.gens_va, row);
    let ff = ffs.map(|f| f.map(|k| FfParam::row(t.ffs, t.ffs_va, k)));
    Generator::new(Some(p), ff, &cx.assets.particle.force_fields, &mut cx.particles.generator_serial)
}

/// Force field rows by address: 0x005cf000 + 0x20 k.
const FF_0: usize = 0;
const FF_3: usize = 3;
const FF_4: usize = 4;
const FF_5: usize = 5;
const FF_6: usize = 6;
const FF_8: usize = 8;
const FF_9: usize = 9;
const FF_10: usize = 10;

impl StfRoll {
    /// The thread's `new` (0x150 bytes, the first nine words cleared), the
    /// constructor (0x004f0aa0) and `Init(0)` (0x004f0d40).
    pub fn new(t: &Table, clips: &dyn Clips, asks: &mut Vec<Ask>) -> StfRoll {
        let anm = Anm { time: 0, spd: 256, found: false, transparency: ONE };
        let mut s = StfRoll {
            rows: [t.lines.to_vec(), t.end_lines.to_vec()],
            lines: Vec::new(),
            comets: Vec::new(),
            dead: Vec::new(),
            last: None,
            last_pos: ee::VF0,
            scroll: 0,
            hold: 0,
            book_wait: 0,
            book_drawn: 0,
            logo: "",
            anms: [anm; 2],
            view: ee::VF0,
            eye: ee::VF0,
            pos: ee::VF0,
            rot: ee::VF0,
            glows: [None; 2],
            row: 0,
            done: false,
            spacing: (0, 0),
            at_end: false,
            state: 0,
            phase: 0,
            fade: 0,
            alpha: 0,
            count: 0,
            spawn: 0,
            spawned: 0,
            spawn_max: 0,
            ring_done: 0,
            logo_wait: 0,
            tail: 0,
            serial: 0,
        };
        s.init_first(clips, asks);
        s
    }

    /// `Init(0)`: the layers, the 17 kanji, the two animations (each one
    /// step on; the ring then 25 a frame), the first 17 lines, the camera
    /// and the first logo.
    fn init_first(&mut self, clips: &dyn Clips, asks: &mut Vec<Ask>) {
        self.scroll = 0xbf0c_cccd;
        self.hold = 0;
        self.spacing = (20, 5);
        self.state = 0;
        self.phase = 0;
        self.row = 0;
        self.at_end = false;
        self.fade = 0x3ccc_cccd;
        self.alpha = 0;
        self.hold = 240;
        for (k, a) in [Anim::Ring, Anim::Book].into_iter().enumerate() {
            let anm = &mut self.anms[k];
            anm.found = clips.clip(a).is_some();
            if anm.found {
                Self::forward(anm, clips, a);
            }
            if a == Anim::Ring {
                anm.spd = 25;
            }
        }
        self.pos[2] = 0x4296_0000;
        self.pos[1] = 0xc248_0000;
        for k in 0..LINES {
            if k != 0 {
                self.row += 1;
            }
            self.push_line(LineRef { end: false, row: self.row });
        }
        self.eye[1] = 0xc3af_0000;
        asks.push(Ask::Camera { pos: self.eye, view: self.view });
        self.logo = "TEX_hackrogo";
    }

    /// `Init(n)` for n 1-3.
    fn init(&mut self, n: i32, asks: &mut Vec<Ask>) {
        match n {
            1 => {
                asks.push(Ask::Bgm(2));
                (self.state, self.phase, self.count) = (1, 1, 0);
                (self.fade, self.alpha, self.hold) = (0x3ccc_cccd, 0, 240);
                (self.tail, self.spawn_max, self.spawned) = (480, 1, 1);
                self.logo = "TEX_hackrogo_2";
            }
            2 => {
                (self.state, self.row, self.at_end) = (2, 0, false);
                (self.fade, self.alpha, self.hold) = (0x3ccc_cccd, 0, 240);
                self.lines.clear();
                self.push_line(LineRef { end: true, row: 0 });
            }
            3 => {
                self.state = 3;
                self.spawn = 300;
            }
            _ => {}
        }
    }

    /// `_AnimateForward(frameSpd)`: whether a play-once clip ended.
    fn forward(anm: &mut Anm, clips: &dyn Clips, a: Anim) -> bool {
        let Some((frames, looping)) = clips.clip(a) else { return false };
        let f = piney_data::anim::forward_clip(frames, looping, anm.time, u32::from(anm.spd));
        anm.time = f.time;
        f.ended
    }

    fn row_of(&self, l: LineRef) -> &StfRollLine {
        &self.rows[usize::from(l.end)][l.row]
    }

    fn row_mut(&mut self, l: LineRef) -> &mut StfRollLine {
        &mut self.rows[usize::from(l.end)][l.row]
    }

    /// 0x004f3710: a line onto the end, 25 below the last (400 for the
    /// first); none once the table's end is in. The end row is put in too.
    fn push_line(&mut self, l: LineRef) {
        if self.at_end {
            return;
        }
        if self.row_of(l).text.is_none() {
            self.at_end = true;
        }
        let y = match self.lines.last() {
            Some(&last) => {
                let gap = ee::add(ee::from_int(self.spacing.0), ee::from_int(self.spacing.1));
                ee::add(gap, self.row_of(last).y.to_bits())
            }
            None => 0x43c8_0000,
        };
        self.row_mut(l).y = f32::from_bits(y);
        self.lines.push(l);
    }

    /// 0x004f36b0: the first line out, unless none or the end is in.
    fn drop_first(&mut self) {
        if !self.lines.is_empty() && !self.at_end {
            self.lines.remove(0);
        }
    }

    /// One frame of main (0x004f1880).
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Table, clips: &dyn Clips) -> (Vec<Draw>, Vec<Ask>) {
        let (mut draws, mut asks) = (Vec::new(), Vec::new());
        self.scroll_lines();
        self.fade_logo();
        self.draw_text(&mut draws);
        self.next_state(&mut asks);
        self.anm_alpha();
        self.draw_anms(clips, &mut draws);
        self.make_generators(ctrl, cx, t);
        self.next_phase(cx, &mut asks);
        self.spawn_late(ctrl, cx, t);
        self.move_comets(ctrl, cx);
        (draws, asks)
    }

    /// 0x004f2f40: the credits move up while they show.
    fn scroll_lines(&mut self) {
        if self.state != 1 {
            return;
        }
        for k in 0..self.lines.len() {
            let l = self.lines[k];
            let y = ee::add(self.row_of(l).y.to_bits(), self.scroll);
            self.row_mut(l).y = f32::from_bits(y);
            if self.row_of(l).text.is_none() {
                break;
            }
        }
    }

    /// The logo's transparency: up (or down) by the fade, held to 0..1.
    fn step_alpha(&mut self) {
        if ee::le(self.alpha, ONE) {
            self.alpha = ee::add(self.alpha, self.fade);
        }
        if !ee::le(self.alpha, ONE) {
            self.alpha = ONE;
        }
        if ee::le(self.alpha, 0) {
            self.alpha = 0;
        }
    }

    /// 0x004f2fe0.
    fn fade_logo(&mut self) {
        match self.state {
            0 => self.step_alpha(),
            2 if self.logo_wait == 0 => self.step_alpha(),
            _ => {}
        }
    }

    /// 0x004f31b0: the credits' lines, or the logo.
    fn draw_text(&mut self, draws: &mut Vec<Draw>) {
        match self.state {
            1 => {
                for (k, &l) in self.lines.iter().enumerate() {
                    let r = self.row_of(l);
                    let Some(text) = r.text else { break };
                    draws.push(Draw::Line { k, x: r.x.to_bits(), y: r.y.to_bits(), text });
                }
            }
            0 | 2 => {
                if self.logo_wait != 0 {
                    self.logo_wait -= 1;
                } else {
                    draws.push(Draw::Logo { tex: self.logo, transp: self.alpha });
                }
            }
            _ => {}
        }
    }

    /// 0x004f3370: the logos' holds and the credits' turnover.
    fn next_state(&mut self, asks: &mut Vec<Ask>) {
        if self.lines.is_empty() {
            return;
        }
        match self.state {
            0 => {
                if !ee::lt(self.alpha, ONE) {
                    self.hold -= 1;
                }
                if !ee::lt(self.alpha, ONE) && self.hold == 0 {
                    self.fade = ee::mul(self.fade, 0xbf80_0000);
                } else if self.hold == 0 && ee::le(self.alpha, 0) {
                    self.init(1, asks);
                }
            }
            1 => {
                // Each line, from the first; a line that goes off the top
                // takes the first out, so the one after it is passed over.
                let mut k = 0;
                while k < self.lines.len() {
                    let l = self.lines[k];
                    let r = *self.row_of(l);
                    if r.text.is_none() {
                        if ee::lt(r.y.to_bits(), 0) {
                            self.init(2, asks);
                            return;
                        }
                    } else if ee::lt(r.y.to_bits(), ee::from_int(-self.spacing.0)) {
                        self.row += 1;
                        self.drop_first();
                        self.push_line(LineRef { end: false, row: self.row });
                    }
                    k += 1;
                }
            }
            2 => {
                if !ee::lt(self.alpha, ONE) {
                    self.hold -= 1;
                }
                if !ee::lt(self.alpha, ONE) && self.hold == 0 {
                    self.fade = ee::mul(self.fade, 0xbf80_0000);
                } else if self.hold == 0 && ee::le(self.alpha, 0) {
                    self.fade = ee::mul(self.fade, 0xbf80_0000);
                    self.row += 1;
                    self.alpha = 0;
                    self.hold = 240;
                    self.logo_wait = 180;
                    self.drop_first();
                    self.push_line(LineRef { end: true, row: self.row });
                    self.logo = "TEX_hack_url";
                    if self.at_end {
                        self.init(3, asks);
                    }
                }
            }
            _ => {}
        }
    }

    /// 0x004f1cb0: the ring fades once its clip is over; the book comes up.
    fn anm_alpha(&mut self) {
        match self.phase {
            1 if self.ring_done != 0 && self.count > 0 => {
                let a = &mut self.anms[0];
                a.transparency = ee::sub(a.transparency, 0x3c23_d70a);
            }
            3 => {
                if self.count == 0 {
                    self.anms[1].transparency = 0;
                }
                if self.count > 0 {
                    let a = &mut self.anms[1];
                    a.transparency = ee::add(a.transparency, 0x3b03_126f);
                }
            }
            _ => {}
        }
    }

    /// `x` back into -pi..pi.
    fn wrap(x: F) -> F {
        let x = if ee::lt(x, NEG_PI) { ee::add(x, TWO_PI) } else { x };
        if ee::le(x, PI) { x } else { ee::sub(x, TWO_PI) }
    }

    fn anm_draw(&self, k: usize, matrix: vu::M4, draws: &mut Vec<Draw>) {
        let anim = if k == 0 { Anim::Ring } else { Anim::Book };
        let a = &self.anms[k];
        draws.push(Draw::Anm { anim, time: a.time, matrix, transparency: a.transparency });
    }

    /// 0x004f2620: the ring and the book, each phase its way.
    fn draw_anms(&mut self, clips: &dyn Clips, draws: &mut Vec<Draw>) {
        let one = [ONE; 4];
        match self.phase {
            1 => {
                if self.ring_done == 0 {
                    if ee::lt(ee::fabsf(self.pos[1]), 0x4334_0000) {
                        self.pos[1] = ee::sub(self.pos[1], 0x3d4c_cccd);
                    }
                    self.pos[2] = ee::sub(self.pos[2], 0x3c54_fdf4);
                    self.rot[0] = Self::wrap(ee::add(self.rot[0], 0x3b03_126f));
                    if self.anms[0].time >> 8 == 720 {
                        self.anms[0].spd = 144;
                    }
                    self.rot[2] = self.rot[0];
                    let m = vu::pos_rot_xyz_scale(self.pos, self.rot, one);
                    let ended = self.anms[0].found && Self::forward(&mut self.anms[0], clips, Anim::Ring);
                    self.ring_done = i32::from(ended);
                    if ended {
                        self.count = 0;
                    }
                    self.anm_draw(0, m, draws);
                } else {
                    self.rot[0] = ee::add(self.rot[0], 0x3b03_126f);
                    self.rot[2] = self.rot[0];
                    let m = vu::pos_rot_xyz_scale(self.pos, self.rot, one);
                    self.anm_draw(0, m, draws);
                }
            }
            3 => {
                if !ee::lt(self.anms[1].transparency, 0x3f4c_cccd) {
                    self.rot[0] = ee::sub(self.rot[0], 0x3c13_74bc);
                }
                if self.book_wait == 300 {
                    if self.anms[1].time >> 8 < 341 && self.anms[1].found {
                        Self::forward(&mut self.anms[1], clips, Anim::Book);
                    }
                } else {
                    self.book_wait += 1;
                }
                if ee::lt(self.rot[0], 0) {
                    self.rot[0] = 0;
                }
                let m = vu::pos_rot_zyx_scale(self.pos, self.rot, one);
                self.anm_draw(1, m, draws);
            }
            4 => {
                let m = vu::pos_rot_zyx_scale(self.pos, self.rot, one);
                self.book_drawn = 1;
                self.anm_draw(1, m, draws);
            }
            5 => {
                self.rot[1] = Self::wrap(ee::sub(self.rot[1], 0x3c9b_a5e3));
                self.rot[0] = ee::add(self.rot[0], 0x3c13_74bc);
                if !ee::le(self.rot[0], HALF_PI) {
                    self.rot[0] = HALF_PI;
                }
                let m = vu::pos_rot_zyx_scale(self.last_pos, self.rot, one);
                if self.anms[1].found {
                    Self::forward(&mut self.anms[1], clips, Anim::Book);
                }
                self.anm_draw(1, m, draws);
            }
            _ => {}
        }
    }

    /// A comet of path `kind`, at `speed` (0x004f38f0's random one when
    /// None).
    fn new_comet(&mut self, cx: &mut Cx, t: &Table, kind: i32) -> Comet {
        self.serial += 1;
        let mut c = Comet::new(ANCHOR | self.serial);
        // 0x004f38f0: |ccRandF(0.0045)|, at least 0.003.
        let r = ee::fabsf(rand_f(cx, 0x3b93_74bc));
        c.speed = if ee::le(r, 0x3b44_9ba6) { 0x3b44_9ba6 } else { r };
        self.path(cx, t, &mut c, kind);
        c
    }

    /// 0x004f12a0: the comet's four points (and its place at the first).
    fn path(&self, cx: &mut Cx, t: &Table, c: &mut Comet, kind: i32) {
        let point = |x: F, y: F, z: F| [x, y, z, ONE];
        match kind {
            0 | 1 => {
                for i in 0..4 {
                    let mut v = ee::VF0;
                    let random = if kind == 0 { i != 3 } else { i != 3 && i != 0 };
                    if random {
                        v[0] = pick(cx, t.rand_x);
                        v[1] = pick(cx, if kind == 0 { t.rand_y } else { t.rand_y2 });
                    }
                    v[2] = ee::from_int(if kind == 0 { 130 - 90 * i } else { 90 * i - 60 });
                    c.set(i, v);
                    if i == 0 {
                        c.set(-1, v);
                    }
                }
            }
            2 => {
                for (i, z) in [(0, 0x4348_0000), (1, 0x42f0_0000), (2, 0x4270_0000)] {
                    let x = pick(cx, t.rand_x);
                    let y = pick(cx, t.rand_y);
                    c.set(i, point(x, y, z));
                    if i == 0 {
                        c.set(-1, point(x, y, z));
                    }
                }
                c.set(3, point(0, 0xc270_0000, 0xc170_0000));
            }
            3 => {
                c.set(0, self.pos);
                c.set(-1, self.pos);
                // sceVu0AddVector: all four lanes, so w is 2.
                let v = std::array::from_fn(|k| ee::add([0, 0, 0xc220_0000, ONE][k], self.pos[k]));
                c.set(1, v);
                c.set(2, point(0, 0, 0xc234_0000));
                c.set(3, point(0, 0, 0x43d2_0000));
            }
            _ => {}
        }
    }

    /// A generator following comet `c` (its +0x70), started.
    fn follow(ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Table, c: &Comet, row: usize, ffs: [Option<usize>; 4]) -> GenRef {
        ctrl.anchors.insert(c.key, c.pos);
        let mut g = generator(cx, t, row, ffs);
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::Anchor(c.key));
        cx.particles.start(g)
    }

    /// A generator standing at `pos`, started.
    fn stand(cx: &mut Cx, t: &Table, row: usize, ffs: [Option<usize>; 4], pos: V4) -> GenRef {
        let mut g = generator(cx, t, row, ffs);
        g.pos = pos;
        cx.particles.start(g)
    }

    /// 0x004f1d80: each phase's generators and comets, by its frame count.
    fn make_generators(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Table) {
        let up = |s: &mut Self| {
            if s.count < 31 {
                s.count += 1;
            }
        };
        match self.phase {
            1 if self.ring_done == 0 => {
                if self.count == 1 {
                    let at = [0, 0, 0xc348_0000, ONE];
                    self.glows[0] = Some(Self::stand(cx, t, 0, [None; 4], at));
                    self.glows[1] = Some(Self::stand(cx, t, 1, [None; 4], at));
                }
                up(self);
            }
            1 => {
                up(self);
                if self.count == 1 {
                    let mut c = self.new_comet(cx, t, 1);
                    c.speed = 0x3b03_126f;
                    c.main_gen = Some(Self::follow(ctrl, cx, t, &c, 5, [Some(FF_4), Some(FF_5), None, None]));
                    c.has_gen = 1;
                    let sub = Self::follow(ctrl, cx, t, &c, 4, [Some(FF_6), None, None, None]);
                    c.subs.push(sub);
                    c.age = -60;
                    // Two more that follow it, kept by no one.
                    Self::follow(ctrl, cx, t, &c, 2, [Some(FF_3), None, None, None]);
                    Self::follow(ctrl, cx, t, &c, 3, [Some(FF_3), None, None, None]);
                    self.comets.push(c);
                }
            }
            3 => {
                if self.count == 0 {
                    Self::stand(cx, t, 6, [Some(FF_8), None, None, None], self.pos);
                    let at = std::array::from_fn(|k| ee::add([0, 0xc1a0_0000, 0, ONE][k], self.pos[k]));
                    Self::stand(cx, t, 7, [Some(FF_9), None, None, None], at);
                } else if self.count == 2 {
                    let mut c = self.new_comet(cx, t, 2);
                    c.speed = 0x3ba3_d70a;
                    c.age = -60;
                    c.main_gen = Some(Self::follow(ctrl, cx, t, &c, 5, [Some(FF_4), Some(FF_5), None, None]));
                    c.has_gen = 1;
                    let sub = Self::follow(ctrl, cx, t, &c, 4, [Some(FF_6), None, None, None]);
                    c.subs.push(sub);
                    self.comets.push(c);
                }
                up(self);
            }
            4 => {
                up(self);
                if self.count == 1 {
                    let mut c = self.new_comet(cx, t, 3);
                    c.speed = 0x3b83_126f;
                    c.main_gen = Some(Self::follow(ctrl, cx, t, &c, 8, [Some(FF_0), Some(FF_10), None, None]));
                    c.has_gen = 1;
                    self.last = Some(c.key);
                    self.last_pos = c.pos;
                    self.comets.push(c);
                }
            }
            _ => {}
        }
    }

    /// Every comet done (+0x54 1).
    fn all_done(&self) -> bool {
        self.comets.iter().all(|c| c.dead == 1)
    }

    /// 0x004f3990: a comet's generators ended as its mode says (those
    /// still running), then freed.
    fn end_gens(cx: &mut Cx, c: &mut Comet) {
        let end = |cx: &mut Cx, g: GenRef, mode: i32| {
            if cx.particles.check(g) {
                match mode {
                    0 => cx.particles.particle_kill(g),
                    1 => cx.particles.particle_delete(g),
                    _ => {}
                }
            }
        };
        if let Some(g) = c.main_gen {
            end(cx, g, c.kill_mode);
        }
        for &g in &c.subs {
            end(cx, g, c.kill_mode);
        }
        c.subs.clear();
    }

    /// 0x004f2aa0: the ring's and book's phases end.
    fn next_phase(&mut self, cx: &mut Cx, asks: &mut Vec<Ask>) {
        match self.phase {
            1 => {
                if self.count == 0 || self.ring_done == 0 || self.spawned != self.spawn_max || !self.all_done() {
                    return;
                }
                (self.phase, self.count) = (2, 0);
                for mut c in std::mem::take(&mut self.comets) {
                    if c.dead == 1 {
                        Self::end_gens(cx, &mut c);
                    }
                }
            }
            2 => {
                if !self.all_done() {
                    return;
                }
                for g in self.glows.iter().flatten() {
                    if cx.particles.check(*g) {
                        cx.particles.particle_kill(*g);
                    }
                }
                self.pos[1] = 0;
                (self.phase, self.count) = (3, 0);
                self.rot = [ONE, 0, 0, ONE];
            }
            3 => {
                if !self.all_done() {
                    return;
                }
                if self.anms[1].time >> 8 < 339 || ee::le(self.anms[1].transparency, ONE) {
                    return;
                }
                (self.phase, self.count) = (4, 0);
                self.rot = ee::VF0;
            }
            4 if self.book_drawn != 0 => {
                (self.phase, self.count) = (5, 0);
                self.rot = ee::VF0;
            }
            5 => {
                if !self.all_done() {
                    return;
                }
                let left = self.tail;
                self.tail -= 1;
                if left <= 0 {
                    asks.push(Ask::BgmStop);
                    self.done = true;
                }
            }
            _ => {}
        }
    }

    /// 0x004f1920: in state 3 with the ring's phase 1, a comet every 250
    /// frames while the count is short of the limit.
    fn spawn_late(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Table) {
        if self.state != 3 || self.phase != 1 || self.spawned >= self.spawn_max {
            return;
        }
        self.spawn += 1;
        if self.spawn < 250 {
            return;
        }
        let mut c = self.new_comet(cx, t, 0);
        c.main_gen = Some(Self::follow(ctrl, cx, t, &c, 4, [None; 4]));
        c.has_gen = 1;
        c.kill_mode = 1;
        self.comets.push(c);
        self.spawn = 0;
        self.spawned += 1;
    }

    /// 0x004f3d50: the Catmull-Rom point at `t` of `a`..`d`, w 1.
    fn spline(t: F, p: [V4; 4]) -> V4 {
        let t2 = ee::mul(t, t);
        let t3 = ee::mul(t2, t);
        let half = 0x3f00_0000;
        let w0 = ee::sub(ee::add(ee::mul(0xbf00_0000, t3), t2), ee::mul(half, t));
        let w1 = ee::add(ONE, ee::add(ee::mul(0x3fc0_0000, t3), ee::mul(0xc020_0000, t2)));
        let w2 = ee::add(ee::mul(half, t), ee::add(ee::mul(0xbfc0_0000, t3), ee::mul(0x4000_0000, t2)));
        let w3 = ee::sub(ee::mul(half, t3), ee::mul(half, t2));
        let mut out = ee::apply(&p, [w0, w1, w2, w3]);
        out[3] = ONE;
        out
    }

    /// 0x004f3ad0: a comet's frame.
    fn step_comet(cx: &mut Cx, c: &mut Comet) {
        if c.delay != 0 {
            c.delay -= 1;
            return;
        }
        if c.seg != 4 && !ee::le(c.t, ONE) {
            c.t = 0;
            c.seg += 1;
        }
        let [p0, p1, p2, p3] = c.points;
        match c.seg {
            0 => c.pos = Self::spline(c.t, [p0, p0, p1, p2]),
            1 => c.pos = Self::spline(c.t, [p0, p1, p2, p3]),
            2 => c.pos = Self::spline(c.t, [p1, p2, p3, p3]),
            3 => {
                if c.has_gen == 1
                    && let Some(g) = c.main_gen
                    && cx.particles.check(g)
                    && c.kill_mode == 0
                {
                    cx.particles.particle_kill(g);
                }
                for &g in &c.subs {
                    if cx.particles.check(g) {
                        match c.kill_mode {
                            0 => cx.particles.particle_kill(g),
                            1 => cx.particles.particle_delete(g),
                            _ => {}
                        }
                    }
                }
                c.seg = 4;
            }
            4 => {
                let age = c.age;
                c.age += 1;
                if age >= 90 {
                    c.dead = 1;
                }
            }
            _ => {}
        }
        c.t = ee::add(c.t, c.speed);
    }

    /// 0x004f1ae0: the comets move; the done ones go (each by the place it
    /// was found at, the later ones not moved down as the earlier go).
    fn move_comets(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        if self.comets.is_empty() {
            return;
        }
        let mut found = 0;
        for k in 0..self.comets.len() {
            Self::step_comet(cx, &mut self.comets[k]);
            let c = &self.comets[k];
            ctrl.anchors.insert(c.key, c.pos);
            if self.last == Some(c.key) {
                self.last_pos = c.pos;
            }
            if c.dead == 1 {
                self.dead.push(k);
                found += 1;
            }
        }
        while found != 0 {
            let k = self.dead[0];
            if let Some(c) = self.comets.get_mut(k) {
                Self::end_gens(cx, c);
            }
            // Its generators that still run go on reading the freed
            // comet's +0x70, which nothing writes again: the anchor stays.
            if k < self.comets.len() {
                self.comets.remove(k);
            }
            self.dead.remove(0);
            found = self.dead.len();
        }
        self.dead.clear();
    }
}
