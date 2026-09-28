//! A stream's own effect task: `ccSetStreamDemoThread` (0x001979c0) starts
//! the task `StreamDemoFuncTbl` (0x0034f420) names for the scene, priority
//! 96, and the scene's 0x8010 notes cue it (`docs/engine/stream.md`,
//! "Effects"). Ported: stream 2's `Func_str0001` (0x001885d0), stream 5's
//! `Func_str0120` (0x001950b0, Skeith and Orca) and stream 10's
//! `Func_str0300` (0x00189e10, the first Data Drain), [`Task`].
//!
//! Each task keeps a `ccStrEffectCtrl` ([`Ctrl`]): eight raster-noise bands,
//! a screen fade, a feedback (the last frame drawn over this one, larger), a
//! colour inversion and buffer shades; what differs is the start and which
//! cue does what. `Func_str0300` starts with everything off, draws its
//! feedback on its own layer, and reads a cue by its last digit (1 noise
//! on, 2 off, 3 inversion on, 4 off, 5 feedback, 6 off, 7 and 8 a one-frame
//! white flash) but for 55, 75 (feedbacks of other scales and weights) and
//! 999 (to white). `Func_str0120` is the same with its own map: feedbacks
//! that hold, then fade out (5, 610-614), fades from and to white and to
//! black (800, 999, 899), the view's `divZ` to 500 (12), and hit marks
//! (`effHitMarkStr`, 601-603 and 610-614) at the note's object, which the
//! port reports ([`HitMark`]) but does not draw.
//!
//! The task runs once a game frame after the scene's own task (priority 24):
//! it takes the cues queued this frame (last first), updates its effects and
//! sends their packets, then breathes. Its first pass is in the frame before
//! the scene's first drawn one, and it stops when the scene's
//! `DelSceneObject` clears state 0x40: one pass into the scene's two gap
//! frames after a natural end, none after a skip.
//!
//! What `Func_str0001` does:
//! - **At start.** `OBJ_se1_6flo1`'s model stops writing Z
//!   (`ccObj::SetRenderState(CCRS_ZWRITEENABLE, 0)`: TEST NEVER / FB_ONLY);
//!   the scene's draw environment fogs to black from 7,500 to 20,000
//!   (`ccDrawEnv::SetFog`) except on the four [`FOG_OFF`] objects; eight
//!   `ccRasterNoize` bands are set up on its own layer ([`EFFECT_LAYER`]),
//!   drawing 8 x 27 numbers from `rand`; the feedback
//!   (`ccBufferSampling::SetReflex(1.015, 0, 0x58808080)`) goes on on
//!   `LYR_jm`; and a fade in from black over 30 frames starts (`ccScFade`,
//!   on the font layer).
//! - **Cues.** 5, 6, 15: the feedback again at alpha 0x58, 0x50, 0x60. 11:
//!   the raster noise starts, each band at a random row, height, drift and
//!   life; 12: it stops. 999: a fade to white over the frames left. Any
//!   other (800, 16 in `str0001`): nothing.
//! - **Each frame.** The feedback (the previous frame's picture 1.5% larger,
//!   blended over the current one); the fade; the noise when on: each band
//!   new offsets for its rows from `rand`, drifting a sixteenth of its speed
//!   a frame, a new band when its life runs out.
//!
//! Its `ccBufferReverce` (colour inversion) is never switched on and its
//! `ccBufferShade_sc` has no entries: neither draws. The fog is the
//! scene's ([`Scene::fog`]), taken at each model's centre.

use crate::file;
use crate::scene::{Loaded, Scene, SceneFog};
use piney_data::anim::ee;
use piney_desktop::fade;
use piney_desktop::layers::FONT_LAYER;
use piney_desktop::noiz::{Band, EffView, Sampling, reverse_prim_in};
use piney_desktop::view::Frame;
use piney_draw::{Cmd, Prim};

/// The C library's `rand` (newlib, 0x00133a38): `next = next *
/// 6364136223846793005 + 1` in 64 bits, returning bits 32-62. The state
/// (`_impure_ptr->_rand_next`) starts at 1 and is the whole game's: only the
/// staff roll calls `srand`, and every task that draws a random number
/// moves it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rand {
    pub next: u64,
}

impl Default for Rand {
    fn default() -> Self {
        Rand { next: 1 }
    }
}

impl Rand {
    pub fn rand(&mut self) -> i32 {
        self.next = self.next.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (self.next >> 32 & 0x7fff_ffff) as i32
    }
}

/// `StreamDemoFuncTbl`'s entries for the tasks ported.
pub const STR0001: &str = "str0001";
pub const STR0120: &str = "str0120";
pub const STR0300: &str = "str0300";
pub const STR0090: &str = "str0090";
pub const STR0110: &str = "str0110";
pub const STR0130: &str = "str0130";
pub const STR0150: &str = "str0150";
pub const STR0240: &str = "str0240";
pub const STR0250: &str = "str0250";
pub const STR0301: &str = "str0301";
pub const STR0305: &str = "str0305";
pub const STR0350: &str = "str0350";
pub const STR0570: &str = "str0570";
pub const STR0610: &str = "str0610";
pub const STR8000: &str = "str8000";
/// Skeith's drain movie (stream 18): `StreamDemoFuncTbl` gives it
/// `Func_str8000` too.
pub const STR9102: &str = "str9102";
/// A member drained (stream 20): `StreamDemoFuncTbl`'s `Func_str9000`.
pub const STR9104: &str = "str9104";
/// Stream 17: `StreamDemoFuncTbl`'s `Func_str9001`.
pub const STR9101: &str = "str9101";
/// The gate hack's town gate (`str71xx`).
pub const STR7100: &str = "str7100";
/// Streams 112-119 (`str88xx`).
pub const STR8800: &str = "str8800";

/// A cue as `ccSetStreamDemoNote` queues it: the note's param, and its
/// object (the index its first word names; the game keeps the object's
/// pointer, `ccAnmNote` +0x10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    pub param: u32,
    pub obj: u32,
}

/// What the tasks read from the executable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tables {
    /// `hitRot0120a` (INF main 0x0034f320): `Func_str0120`'s hit marks'
    /// rotations, in degrees, 16 bytes each (x, y, z, pad), as f32 bits. It
    /// has three; the cues 610-614 ask for up to five, reading on into the
    /// next data (the jump table, denormal floats the EE takes as 0): eight
    /// read.
    pub hit_rot_0120a: Vec<[u32; 3]>,
    /// `hitRot0240` (0x0034e3d0): `Func_str0240`'s, seven entries of the
    /// same form; its eighth mark reads the next data, the string
    /// "OBJ_trall".
    pub hit_rot_0240: Vec<[u32; 3]>,
    /// `hitRot0250` (0x0034e450): `Func_str0250`'s one rotation, (0, 0, 0).
    pub hit_rot_0250: [u32; 3],
    /// `hitRot0301` (0x0034f310): `Func_str0301`'s one rotation, (0, 30,
    /// 180).
    pub hit_rot_0301: [u32; 3],
    /// `bossSkillNameTbl[0]`'s texture (`xeffect`'s `TEX_detadrain`) and its
    /// height, which `Func_str9000`'s banner draws from; None: no banner
    /// (the game makes none while `game` is missing or its status is 2 or
    /// 7). Not in the executable: the player of the stream gives it
    /// ([`crate::Options::skill_names`]).
    pub skill_names: Option<(piney_draw::TexRef, i32)>,
    /// Stream 15's event tables and rock scales.
    pub ending: crate::ending::Tables,
}

impl Tables {
    /// The volume's (`tables::stream`).
    pub fn read(volume: piney_data::volume::Volume) -> Tables {
        let t = piney_data::tables::stream::of(volume);
        let bits = |r: [f32; 3]| r.map(f32::to_bits);
        Tables {
            hit_rot_0120a: t.hit_rot_0120a().iter().map(|&r| bits(r)).collect(),
            hit_rot_0240: t.hit_rot_0240().iter().map(|&r| bits(r)).collect(),
            hit_rot_0250: bits(t.hit_rot_0250()),
            hit_rot_0301: bits(t.hit_rot_0301()),
            skill_names: None,
            ending: crate::ending::Tables::read(volume),
        }
    }
}

/// A task's `SetFog` on the scene's draw environment, and the objects it
/// takes out of it (`fogSW` bit 3 cleared) by name.
fn set_fog(loaded: &Loaded, scene: &mut Scene, fog: SceneFog, off: &[&str]) {
    scene.fog = Some(fog);
    for name in off {
        if let Some(o) = find_entry(loaded, scene.file, name)
            && let Some(&i) = scene.by_obj.get(&o)
        {
            scene.fog_off.insert(i);
        }
    }
}

/// A hit mark at `obj` turned by the `n`th row of `table` (degrees; a row
/// past it is 0): `pi * deg / 180` in the EE's float.
fn hit_mark(table: &[[u32; 3]], obj: u32, n: usize) -> HitMark {
    const PI: u32 = 0x4049_0fdb;
    const HALF_TURN: u32 = 0x4334_0000;
    let deg = table.get(n).copied().unwrap_or_default();
    HitMark { obj, rot: deg.map(|d| ee::div(ee::mul(PI, d), HALF_TURN)) }
}

/// `effHitMarkStr(pos, rot, strEffLayer)` (0x001cc620): a hit mark the
/// task starts at the note's object - its `lwMatrix` translation - turned
/// by `rot` (radians, f32 bits, `pi * deg / 180` in the EE's float). The
/// mark is a `ccEffect` the stream's effect task (`ccThEffectStr`, priority
/// 80) draws on layer 20 from the next frame; not drawn by the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitMark {
    pub obj: u32,
    pub rot: [u32; 3],
}

/// `ccSetStreamDemoThread`'s key into `StreamDemoFuncTbl` for a scene file:
/// a parody file's trailing `p` dropped, then `str7Nxx` (N 0-4) read as
/// `str7N00`, `str88xx` as `str8800` and `str8Nxx` (N 1-3) as `str8000`.
pub fn task_name(stem: &str) -> String {
    let mut n: Vec<u8> = stem.strip_suffix('p').unwrap_or(stem).bytes().collect();
    if n.len() >= 7 {
        let tail: &[u8] = match (n[3], n[4]) {
            (b'7', b'0'..=b'4') | (b'8', b'8') => b"00",
            (b'8', b'1'..=b'3') => b"000",
            _ => b"",
        };
        let at = 7 - tail.len();
        n[at..7].copy_from_slice(tail);
    }
    String::from_utf8_lossy(&n).into_owned()
}

/// `ccStream::GetSubstAdrsF(name, 0)` (0x00144630) over the scene file's
/// own index: an entry of that name, or an ExtObj whose final target has
/// it (`ccMatchIndex` 0x00101ad0); `#` entries are skipped.
pub fn find_entry(loaded: &Loaded, f: usize, name: &str) -> Option<u32> {
    let file = &loaded.files[f];
    (1..file.sf.ccs.objects.len() as u32).find(|&o| {
        if file.external(o) {
            return false;
        }
        if file.setup.kind.get(&o) == Some(&file::EXT_OBJ) {
            loaded.follow(f, o).and_then(|(g, t, _)| loaded.files[g].name(t)) == Some(name)
        } else {
            file.name(o) == Some(name)
        }
    })
}

/// The task's own layer (`new ccLayer`, `Init(100, sysLayer's view)`),
/// where the raster noise draws.
pub const EFFECT_LAYER: i16 = 100;
/// `strEffLayer`: the effect objects' layer (`RequestStrPlay`'s, priority
/// 20), where the tasks' hit marks and transfers draw.
pub const STR_EFF_LAYER: i16 = 20;

/// `LYR_jm`, the scene layer the feedback draws on (the epitaph's text,
/// priority 8 in `str0001`).
pub const FEEDBACK_LAYER_NAME: &str = "LYR_jm";

/// `ccObj::SetRenderState(CCRS_ZWRITEENABLE, 0)` at start.
pub const NO_Z_WRITE: &str = "OBJ_se1_6flo1";

/// `SetFog(fogt[0], fogt[1], 0)` (0x0034e370): F = 255 (clear) at view
/// depth 7,500 falling to 0 (black) at 20,000, per vertex through VU1
/// (`clamp(fogB + fogA w, 0, 255)`, fogA = -255 / (far - near)), on every
/// object whose `fogSW` is set (`ccObj::Init` sets it) but these
/// (`fogOffObj` 0x0030bd80): [`Scene::fog`], at each model's centre.
pub const FOG_NEAR: f32 = 7500.0;
pub const FOG_FAR: f32 = 20000.0;
pub const FOG_COLOUR: [u8; 3] = [0, 0, 0];
pub const FOG_OFF: [&str; 4] = ["OBJ_se1_6bac1", "OBJ_se1_6clo1_1", "OBJ_se1_6clo1_2", "OBJ_se1_6moo1"];

/// `SetNoize(20)` each frame.
const NOISE_AMP: i32 = 20;
const NOISE_BANDS: usize = 8;

/// `SetReflex(1.015f, 0, colour)`: the scale (0x3f81eb85).
const REFLEX_SCALE: u32 = 0x3f81_eb85;
const REFLEX_START: u32 = 0x5880_8080;
/// `Func_str0300`'s cue 55: `SetReflex(1.125f, 0, 0x48808080)`.
const REFLEX_SCALE_WIDE: u32 = 0x3f90_0000;

/// The fades: in from black over 30 frames; out to white (cue 999).
const FADE_IN: (u32, u32, u32) = (30, 0x8000_0000, 0x0000_0000);
pub(crate) const FADE_WHITE: (u32, u32) = (0x00ff_ffff, 0x80ff_ffff);

/// The most cues `ccSetStreamDemoNote` queues in a frame (`eventExNoteTbl`).
pub const MAX_CUES: usize = 8;

/// One `ccRasterNoizeSD`: a `ccRasterNoize` band and its drift.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Noise {
    pub band: Band,
    pub move_y: i32,
    pub offs_y: i32,
    pub pos_y: i32,
    pub time: i32,
}

impl Noise {
    /// `ccRasterNoize::Init` (0x00106170), then `SetNoize(32)`.
    fn new(view: &EffView, rand: &mut Rand) -> Noise {
        Noise { band: Band::new_in(view, &mut || rand.rand()), move_y: 0, offs_y: 0, pos_y: 0, time: 0 }
    }

    /// Cue 11's, and a spent band's, new band `i`: `rand` for its row, its
    /// height, its drift and its life, in that order.
    fn restart(&mut self, i: usize, view: &EffView, rand: &mut Rand) {
        let y = rand.rand() % 50 + 48 * i as i32;
        let h = rand.rand() % 11 + 2;
        self.offs_y = 0;
        self.pos_y = y;
        self.move_y = rand.rand() % 32 - 16;
        self.time = rand.rand() % 15 + 5;
        self.band.set_yh_in(view, y, h);
    }
}

/// `ccBufferSampling_sc`: the feedback and its fade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feedback {
    /// `SetReflex`'s scale about the view's centre (f32 bits), both axes.
    pub scale: u32,
    /// Its turn (radians, f32 bits).
    pub roll: u32,
    /// `ccBufferSampling.color`: RGBA with R in the low byte; A is the
    /// blend's weight.
    pub colour: u32,
    /// 0 off, 1 on, 2 fading out.
    pub flag: u8,
    pub exec_time: i32,
    pub fade_time: i32,
    pub fade_alpha1: i32,
    pub fade_alpha0: i16,
    /// What stream 15's tasks set after `SetReflex`: the picture's offset
    /// in sixteenths of the view's height (+0x24) and the depth it is drawn
    /// at (+0x08), f32 bits.
    pub offset_y: u32,
    pub z: u32,
}

impl Feedback {
    /// `SetReflex(scale, 0, colour)`, on (the cues' form).
    pub(crate) fn reflex(&mut self, scale: u32, colour: u32) {
        self.reflex_turned(scale, 0, colour);
    }

    /// `SetReflex(scale, roll, colour)`, on.
    pub(crate) fn reflex_turned(&mut self, scale: u32, roll: u32, colour: u32) {
        *self = Feedback {
            scale,
            roll,
            colour,
            flag: 1,
            exec_time: 0,
            fade_time: 0,
            fade_alpha1: 0,
            fade_alpha0: 0,
            offset_y: 0,
            z: 0,
        };
    }

    /// `SetReflex(scale, 0, colour)` on for `exec` frames, then fading out
    /// over `fade` frames: `fadeAlpha0` 0, `fadeAlpha1` the colour's alpha
    /// (a signed byte) in 1/4096, divided by `fade` as C divides.
    pub(crate) fn reflex_timed(&mut self, scale: u32, colour: u32, exec: i32, fade: i32) {
        self.reflex(scale, colour);
        self.exec_time = exec;
        self.fade_alpha0 = 0;
        self.fade_alpha1 = (i32::from((colour >> 24) as u8 as i8) << 12) / fade;
        self.fade_time = fade;
    }

    /// Off: `flag`, `execTime` and `fadeTime` 0.
    pub(crate) fn off(&mut self) {
        self.flag = 0;
        self.exec_time = 0;
        self.fade_time = 0;
    }

    /// `ccBufferSampling::MakePacket` (0x00106dd0) at this scale and
    /// colour, on a layer with `view`.
    fn prim(&self, view: &EffView) -> Prim {
        let mut s = Sampling::reflex(self.scale, self.roll, self.colour);
        s.offset[1] = self.offset_y;
        s.z = self.z;
        s.prim_in(view)
    }
}

/// `ccScFade_sc` with its one element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fade {
    /// `flag`: `SendPacket` each frame.
    pub on: bool,
    pub cnt: u32,
    pub tcnt: u32,
    pub col0: u32,
    pub col1: u32,
}

impl Fade {
    /// `DeleteFade`, then `EntryFade(t, c0, c1, view rect)` into the freed
    /// element.
    pub(crate) fn entry(&mut self, t: u32, c0: u32, c1: u32) {
        *self = Fade { on: true, cnt: 0, tcnt: t, col0: c0, col1: c1 };
    }
}

/// A task's packets for one frame: (layer, commands), in the order sent;
/// each goes to the front of its layer, as `ccLayer`'s lists take them.
pub type Draws = Vec<(i16, Vec<Cmd>)>;

/// `ccStrEffectCtrl` (0x20): what every effect task keeps - the eight noise
/// bands on the task's own layer (`ccRasterNoizeSDM_sc`), the fade
/// (`ccScFade_sc`), the feedback (`ccBufferSampling_sc`), the inversion
/// (`ccBufferReverce_sc`); the buffer shades (`ccBufferShade_sc`) and +0x1c
/// (a `ccMask`) no ported task uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctrl {
    /// The feedback's layer; None: the scene has none (the game would crash).
    pub feedback_layer: Option<i16>,
    pub noise: Vec<Noise>,
    /// `ccRasterNoizeSDM_sc.flag`.
    pub noise_on: bool,
    pub feedback: Feedback,
    pub fade: Fade,
    /// `ccBufferReverce_sc.flag`.
    pub reverse: bool,
    /// `ccBufferShade_sc`: its shades, the first `count` drawn (last
    /// first) on the task's layer, and their fade.
    pub shades: Shades,
    /// The frame of the view the task's layers share (the scene's): the
    /// bands, the feedback and the inversion are drawn in it, and the fade
    /// over its rectangle.
    pub frame: Frame,
}

impl Ctrl {
    /// The task's objects made: the eight noise bands' `Init` (drawing their
    /// numbers from `rand`); everything off.
    pub fn new(feedback_layer: Option<i16>, rand: &mut Rand) -> Ctrl {
        Ctrl::framed(feedback_layer, Frame::DEFAULT, rand)
    }

    /// As [`Ctrl::new`] after the task's `SetFrame` gave the scene's view
    /// `frame`.
    pub fn framed(feedback_layer: Option<i16>, frame: Frame, rand: &mut Rand) -> Ctrl {
        let view = EffView::of(&frame);
        Ctrl {
            feedback_layer,
            noise: (0..NOISE_BANDS).map(|_| Noise::new(&view, rand)).collect(),
            noise_on: false,
            feedback: Feedback {
                scale: REFLEX_SCALE,
                roll: 0,
                colour: 0,
                flag: 0,
                exec_time: 0,
                fade_time: 0,
                fade_alpha1: 0,
                fade_alpha0: 0,
                offset_y: 0,
                z: 0,
            },
            fade: Fade { on: false, cnt: 0, tcnt: 0, col0: 0, col1: 0 },
            reverse: false,
            shades: Shades::default(),
            frame,
        }
    }

    /// The raster noise on: each band at a random row, height, drift and
    /// life.
    pub(crate) fn noise_start(&mut self, rand: &mut Rand) {
        let view = EffView::of(&self.frame);
        for (i, n) in self.noise.iter_mut().enumerate() {
            n.restart(i, &view, rand);
        }
        self.noise_on = true;
    }

    /// A fade to black over the frames left (`frameEnd` when none are):
    /// streams 6 and 7's cue 899.
    fn fade_to_black(&mut self, frame_now: u32, frame_end: u32) {
        self.fade_left(frame_now, frame_end, 0, 0x8000_0000);
    }

    /// A fade from `c0` to `c1` over the frames left (`frameEnd` when none
    /// are).
    fn fade_left(&mut self, frame_now: u32, frame_end: u32, c0: u32, c1: u32) {
        let left = frame_end.wrapping_sub(frame_now);
        self.fade.entry(if left != 0 { left } else { frame_end }, c0, c1);
    }

    /// Cue 999: a fade to white over the frames left (1 at the end).
    fn fade_to_white(&mut self, frame_now: u32, frame_end: u32) {
        let left = frame_end.wrapping_sub(frame_now) as i32;
        self.fade.entry(if left != 0 { left as u32 } else { 1 }, FADE_WHITE.0, FADE_WHITE.1);
    }

    /// The pass after the cues: the feedback, the fade, the noise (moved
    /// on unless the scene is held, `paused`), the inversion; the packets
    /// sent, in order.
    pub fn pass(&mut self, paused: bool, rand: &mut Rand) -> Draws {
        let view = EffView::of(&self.frame);
        let mut out: Draws = Vec::new();
        let fb = &mut self.feedback;
        match fb.flag {
            1 => {
                if let Some(l) = self.feedback_layer {
                    out.push((l, vec![Cmd::Prim(fb.prim(&view))]));
                }
                if !paused && fb.exec_time != 0 {
                    fb.exec_time -= 1;
                    if fb.exec_time == 0 {
                        fb.flag = 2;
                    }
                }
            }
            2 => {
                if !paused {
                    fb.fade_time -= 1;
                    if fb.fade_time == 0 {
                        fb.flag = 0;
                        fb.exec_time = 0;
                        fb.fade_time = 0;
                        if fb.fade_alpha0 != 0 {
                            fb.flag = 1;
                        }
                    }
                }
                let a = (i32::from(fb.fade_alpha0) + (fb.fade_time.wrapping_mul(fb.fade_alpha1) >> 12)).clamp(0, 255);
                fb.colour = fb.colour & 0x00ff_ffff | (a as u32) << 24;
                if let Some(l) = self.feedback_layer {
                    out.push((l, vec![Cmd::Prim(fb.prim(&view))]));
                }
            }
            _ => {}
        }
        if self.fade.on {
            // ccScFade::SendPacket: the element at its count, then one on,
            // held at the end.
            let fd = &mut self.fade;
            let mut ctx = piney_desktop::anm::Ctx::new(piney_desktop::view::View::default());
            let f = &self.frame;
            fade::draw_rect(&mut ctx, FONT_LAYER, [f.x, f.y, f.w, f.h], fd.col0, fd.col1, fd.cnt, fd.tcnt);
            out.push((FONT_LAYER, ctx.layers.flatten()));
            fd.cnt = (fd.cnt + 1).min(fd.tcnt);
        }
        if self.noise_on {
            if !paused {
                for (i, n) in self.noise.iter_mut().enumerate() {
                    n.band.set_noize_in(&view, NOISE_AMP, &mut || rand.rand());
                    n.offs_y += n.move_y;
                    n.band.set_y_in(&view, n.pos_y + (n.offs_y >> 4));
                    n.time -= 1;
                    if n.time <= 0 {
                        n.restart(i, &view, rand);
                    }
                }
            }
            for n in &self.noise {
                out.push((EFFECT_LAYER, vec![Cmd::Prim(n.band.prim_in(&view))]));
            }
        }
        if self.reverse {
            out.push((EFFECT_LAYER, vec![Cmd::Prim(reverse_prim_in(&view))]));
        }
        let sh = &mut self.shades;
        let n = sh.count.min(sh.list.len());
        if sh.fade_cnt != 0 {
            let a = sh.fade_base.wrapping_add(((sh.fade_rate.wrapping_mul(i32::from(sh.fade_cnt))) >> 16) as i16);
            for s in sh.list[..n].iter_mut().rev() {
                s.colour = (a as u32) << 24 | s.colour & 0x00ff_ffff;
            }
            sh.fade_cnt -= 1;
        }
        for s in sh.list[..n].iter().rev() {
            out.push((EFFECT_LAYER, vec![Cmd::Prim(s.prim_in(&view))]));
        }
        out
    }
}

/// `ccBufferShade_sc` (0xf8): four `ccBufferSampling`s, the number drawn
/// (+0xe8), and a fade of their alpha (+0xea it, +0xec frames left, +0xee
/// its base, +0xf0 its step in 1/65536).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shades {
    pub list: Vec<Sampling>,
    pub count: usize,
    pub fade_cnt: i16,
    pub fade_base: i16,
    pub fade_rate: i32,
}

/// `Func_str0001`'s state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0001 {
    pub ctrl: Ctrl,
}

impl Str0001 {
    /// `Func_str0001` started on `scene`: `LYR_jm` found, the floor's model
    /// set to write no Z, then [`Str0001::new`].
    pub fn start(loaded: &Loaded, scene: &mut Scene, rand: &mut Rand) -> Str0001 {
        let f = scene.file;
        if let Some(o) = find_entry(loaded, f, NO_Z_WRITE)
            && let Some(&i) = scene.by_obj.get(&o)
            && let Some(model) = scene.nodes[i].model
        {
            scene.z_write_off.insert(model);
        }
        let layer = find_entry(loaded, f, FEEDBACK_LAYER_NAME)
            .and_then(|o| loaded.files[f].setup.layers.as_ref()?.layers.get(&o).copied());
        set_fog(loaded, scene, SceneFog::new(FOG_NEAR, FOG_FAR, 0), &FOG_OFF);
        Str0001::new(layer, rand)
    }

    /// The task's start, up to its loop: the eight noise bands' `Init`
    /// (drawing their numbers from `rand`), the feedback on, the fade in.
    pub fn new(feedback_layer: Option<i16>, rand: &mut Rand) -> Str0001 {
        let mut ctrl = Ctrl::new(feedback_layer, rand);
        ctrl.feedback.reflex(REFLEX_SCALE, REFLEX_START);
        ctrl.fade.entry(FADE_IN.0, FADE_IN.1, FADE_IN.2);
        Str0001 { ctrl }
    }

    /// One pass of the task's loop: `cues` as `ccSetStreamDemoNote` queued
    /// them this frame (the first [`MAX_CUES`]), `frame_now` / `frame_end`
    /// the scene's when the task runs (after the scene read its next
    /// frame), `paused` its `ctrl` bit 0. Returns the packets sent.
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                999 => c.fade_to_white(frame_now, frame_end),
                12 => c.noise_on = false,
                11 => c.noise_start(rand),
                6 => c.feedback.reflex(REFLEX_SCALE, 0x5080_8080),
                15 => c.feedback.reflex(REFLEX_SCALE, 0x6080_8080),
                5 => c.feedback.reflex(REFLEX_SCALE, 0x5880_8080),
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0300`'s state: stream 10, the first Data Drain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0300 {
    pub ctrl: Ctrl,
}

impl Str0300 {
    /// `Func_str0300` (0x00189e10) started: its layer (100) and objects,
    /// the noise bands' `Init` drawing their numbers; nothing on. Its
    /// feedback draws on its own layer.
    pub fn new(rand: &mut Rand) -> Str0300 {
        Str0300 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: the cues (last queued first), then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                999 => c.fade_to_white(frame_now, frame_end),
                75 => c.feedback.reflex(REFLEX_SCALE, 0x4080_8080),
                55 => c.feedback.reflex(REFLEX_SCALE_WIDE, 0x4880_8080),
                // The rest by their last digit (`divu`: unsigned).
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(REFLEX_SCALE, 0x5080_8080),
                    6 => c.feedback.off(),
                    // A one-frame white flash: in, then out.
                    7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                    8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str8000`'s state: the drain movies (streams 108-111, and 18).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str8000 {
    pub ctrl: Ctrl,
}

impl Str8000 {
    /// `Func_str8000` (0x00184460) started as [`Str0300::new`] (it leaves
    /// `strEffLayer` alone).
    pub fn new(rand: &mut Rand) -> Str8000 {
        Str8000 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: the cues (last queued first), then [`Ctrl::pass`]. As
    /// [`Str0300::step`] but 999's fade falls back to `frameEnd` and 75
    /// has no case of its own (its last digit 5). By the last digit (jump
    /// table 0x0034e250): 1 the noise on, 2 off, 3 the inversion on, 4
    /// off, 5 the feedback on (1.015, alpha 0x50), 6 off, 7 and 8 a
    /// one-frame flash to and from white; 55 the feedback 1.125, alpha
    /// 0x48.
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                999 => c.fade_left(frame_now, frame_end, FADE_WHITE.0, FADE_WHITE.1),
                55 => c.feedback.reflex(REFLEX_SCALE_WIDE, 0x4880_8080),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(REFLEX_SCALE, 0x5080_8080),
                    6 => c.feedback.off(),
                    7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                    8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str9000`'s state: a member drained (stream 20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str9000 {
    pub ctrl: Ctrl,
    /// The banner's texture and height, and its row (`bossSkillNameTbl`'s
    /// third word, 0).
    banner: Option<(piney_draw::TexRef, i32)>,
}

/// `Func_str9000`'s `SetFrame(0, 30, 512, 324, 256, 162, 1, 0.84375)` on
/// the scene's view: a letterbox 324 lines high, the picture squeezed to
/// 27/32 of its height. Its end sets the default back.
pub const FRAME_9000: Frame =
    Frame { x: 0.0, y: 30.0, w: 512.0, h: 324.0, cx: 256.0, cy: 162.0, ax: 1.0, ay: 0.843_75 };

/// `Func_str9000`'s `SetFog(1500, 12000, 0, 70, 0x001e1e1e)`: [`Scene::fog`].
pub const FOG_9000: (f32, f32, f32, f32, u32) = (1500.0, 12000.0, 0.0, 70.0, 0x001e_1e1e);

/// The banner's layer: `ccLayer::Init(101, 0)`, a view of its own.
pub const BANNER_LAYER: i16 = 101;

impl Str9000 {
    /// `Func_str9000` (0x00186af0) started: the scene's view letterboxed
    /// ([`FRAME_9000`]) and fogged, then the same objects as
    /// [`Str0090::new`] (the bands through the letterbox), and the banner
    /// (`ccMask(128, 0)` on [`BANNER_LAYER`], `SetTex` of
    /// `bossSkillNameTbl[0]`) when [`Tables::skill_names`] gives it.
    pub fn start(scene: &mut Scene, tables: &Tables, rand: &mut Rand) -> Str9000 {
        scene.frame = FRAME_9000;
        Str9000::new(tables, rand)
    }

    /// As [`Str9000::start`] without a scene.
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str9000 {
        Str9000 { ctrl: Ctrl::framed(Some(EFFECT_LAYER), FRAME_9000, rand), banner: tables.skill_names.clone() }
    }

    /// One pass: the cues (last queued first) - 999 a fade to white over
    /// the frames left; 910 from white over 12 frames; 989 to white over 4;
    /// the rest by their last digit (jump table 0x0034e2e0): 1 the noise
    /// on, 2 off, 3 the inversion on, 4 off, 5 the feedback on (1.01, alpha
    /// 0x50) with a 15-frame fade ready, 6 fading, 7 and 8 a one-frame
    /// flash to and from white - then [`Ctrl::pass`], then the banner.
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                999 => c.fade_left(frame_now, frame_end, FADE_WHITE.0, FADE_WHITE.1),
                910 => c.fade.entry(12, FADE_WHITE.1, FADE_WHITE.0),
                989 => c.fade.entry(4, FADE_WHITE.0, FADE_WHITE.1),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex_timed(0x3f81_47ae, 0x5080_8080, 0, 15),
                    6 => c.feedback.flag = 2,
                    7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                    8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                    _ => {}
                },
            }
        }
        let mut out = c.pass(paused, rand);
        if let Some((tex, tex_h)) = &self.banner {
            out.push((BANNER_LAYER, banner(tex, *tex_h, 0)));
        }
        out
    }
}

/// The skill name's banner (`ccMask` +0x1c of `ccStrEffectCtrl`), each
/// pass: row `n` of the texture, 256 x 20 texels, at (128, 10) on a layer
/// of the default view, colour 0x80 at transparency 1.
fn banner(tex: &piney_draw::TexRef, tex_h: i32, n: i32) -> Vec<Cmd> {
    use piney_desktop::sprite::Sprite;
    let mut sp = Sprite::mask(BANNER_LAYER, tex.clone(), tex_h, 128);
    sp.transp = 1.0;
    sp.wu = 0;
    sp.wv = n * 5 * 64;
    sp.wi = 1;
    sp.sx = 256.0;
    sp.sy = 20.0;
    sp.su = 256;
    sp.sv = 20;
    sp.dx = 128.0;
    sp.dy = 10.0;
    sp.make_packet(0, &piney_desktop::view::LayerView::default_layer());
    let mut ctx = piney_desktop::anm::Ctx::new(piney_desktop::view::View::default());
    sp.send(&mut ctx.layers);
    ctx.layers.flatten()
}

/// `Func_str9001`'s state: stream 17.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str9001 {
    pub ctrl: Ctrl,
}

impl Str9001 {
    /// `Func_str9001` (0x00186210) started as [`Str8000::new`].
    pub fn new(rand: &mut Rand) -> Str9001 {
        Str9001 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: each cue (last queued first) through `Func_str9001sub`
    /// (0x00185e30) - 999 a fade to white over the frames left; by the
    /// last digit (jump table 0x0034e2b0): 1 the noise on, 2 off, 3 the
    /// inversion on, 4 off, 5 the feedback on (1.0, alpha 0x48) with a
    /// 60-frame fade ready, 6 fading, 7 and 8 a one-frame flash to and from
    /// white - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                999 => c.fade_left(frame_now, frame_end, FADE_WHITE.0, FADE_WHITE.1),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex_timed(0x3f80_0000, 0x4880_8080, 0, 60),
                    6 => c.feedback.flag = 2,
                    7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                    8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str7100`'s state: the gate hack's movie, a town's gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str7100 {
    pub ctrl: Ctrl,
}

impl Str7100 {
    /// `Func_str7100` (0x00187ad0) started as [`Str8000::new`].
    pub fn new(rand: &mut Rand) -> Str7100 {
        Str7100 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: each cue (last queued first) by its last digit only (jump
    /// table 0x0034e310, 0-6): 1 the noise on, 2 off, 3 the inversion on,
    /// 4 off, 5 the feedback on (1.015, alpha 0x50; the same on either side
    /// of its test of a state 2), 6 off - then [`Ctrl::pass`]. No fades.
    pub fn step(&mut self, cues: &[Cue], paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue % 10 {
                1 => c.noise_start(rand),
                2 => c.noise_on = false,
                3 => c.reverse = true,
                4 => c.reverse = false,
                5 => c.feedback.reflex(REFLEX_SCALE, 0x5080_8080),
                6 => c.feedback.off(),
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str8800`'s state: streams 112-119.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str8800 {
    pub ctrl: Ctrl,
}

impl Str8800 {
    /// `Func_str8800` (0x00185170) started as [`Str8000::new`].
    pub fn new(rand: &mut Rand) -> Str8800 {
        Str8800 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: the cues (last queued first) - 999 a fade to white over
    /// the frames left; by the last digit (jump table 0x0034e280): 1 the
    /// noise on, 2 off, 3 the inversion on, 4 off, 5 the feedback on
    /// (1.005, alpha 0x60), 6 off, 7 and 8 a one-frame flash to and from
    /// white - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                999 => c.fade_left(frame_now, frame_end, FADE_WHITE.0, FADE_WHITE.1),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(0x3f80_a3d7, 0x6080_8080),
                    6 => c.feedback.off(),
                    7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                    8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0090`'s state: stream 3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0090 {
    pub ctrl: Ctrl,
}

/// `Func_str0090`'s `SetFog(200, 7000, 0x00141e00)` on the scene's draw
/// environment at its start: to a dark green-blue from view depth 200 to
/// 7,000, every object with `fogSW`: [`Scene::fog`].
pub const FOG_0090: (f32, f32, u32) = (200.0, 7000.0, 0x0014_1e00);

impl Str0090 {
    /// `Func_str0090` (0x001919f0) started: its layer (100) and objects,
    /// the noise bands' `Init` drawing their numbers (their texture then
    /// `SetTex(896, 0)`; they never come on), the scene's fog; nothing on.
    /// Its feedback draws on its own layer.
    pub fn new(rand: &mut Rand) -> Str0090 {
        Str0090 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: the cues (last queued first) - 5 and 15 the feedback on
    /// (1.001 larger, alpha 0x6c), 6 and 16 into its fading state, which
    /// with no fade time left draws it at alpha 0 from then on - then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                5 | 15 => c.feedback.reflex(0x3f80_20c5, 0x6c80_8080),
                6 | 16 => c.feedback.flag = 2,
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0110`'s state: stream 4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0110 {
    pub ctrl: Ctrl,
    /// Cue x3's `SetFog(0, 0, 0, 0, 0)`: the scene's fog off from then on
    /// (the stream clears [`Scene::fog`]).
    pub fog_off: bool,
}

impl Str0110 {
    /// `Func_str0110` (0x00194410) started as [`Str0090::new`]: the same
    /// objects and the scene's fog ([`FOG_0090`]); nothing on.
    pub fn new(rand: &mut Rand) -> Str0110 {
        Str0110 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), fog_off: false }
    }

    /// One pass: the cues (last queued first), then [`Ctrl::pass`]. 25 the
    /// feedback on (1.02, alpha 0x40); 15 on for 44 frames at 1.04, alpha
    /// 0x66, fading over 32; 5 on for 10 at 1.015, alpha 0x40, fading over
    /// 15; 16 into its fading state. The rest by their last digit: 1 the
    /// noise on, 2 off, 3 the fog off and the inversion on, 4 the inversion
    /// off, 6 the feedback fading.
    pub fn step(&mut self, cues: &[Cue], paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                25 => c.feedback.reflex(0x3f82_8f5c, 0x4080_8080),
                16 => c.feedback.flag = 2,
                15 => c.feedback.reflex_timed(0x3f85_1eb8, 0x6680_8080, 44, 32),
                5 => c.feedback.reflex_timed(REFLEX_SCALE, 0x4080_8080, 10, 15),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => {
                        self.fog_off = true;
                        c.reverse = true;
                    }
                    4 => c.reverse = false,
                    6 => c.feedback.flag = 2,
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0130`'s state: stream 6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0130 {
    pub ctrl: Ctrl,
}

/// `Func_str0130`'s `SetFrame(0, 48, 512, 288, 256, 192, 0.75, 0.75)` on
/// the scene's view: a letterbox 288 lines high, the picture at 3/4 size
/// centred 48 lines below the frame's middle. Its end sets the default
/// back.
pub const FRAME_0130: Frame = Frame { x: 0.0, y: 48.0, w: 512.0, h: 288.0, cx: 256.0, cy: 192.0, ax: 0.75, ay: 0.75 };

impl Str0130 {
    /// `Func_str0130` (0x001961c0) started: the scene's view letterboxed
    /// ([`FRAME_0130`]) before its layer (100) and objects are made, so the
    /// noise bands' `Init` scales through it; nothing on.
    pub fn start(scene: &mut Scene, rand: &mut Rand) -> Str0130 {
        scene.frame = FRAME_0130;
        Str0130::new(rand)
    }

    /// As [`Str0130::start`] without a scene.
    pub fn new(rand: &mut Rand) -> Str0130 {
        Str0130 { ctrl: Ctrl::framed(Some(EFFECT_LAYER), FRAME_0130, rand) }
    }

    /// One pass: the cues (last queued first) - 899 a fade to black over
    /// the frames left (all the scene's at its end) across the letterbox -
    /// then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            if cue == 899 {
                c.fade_to_black(frame_now, frame_end);
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0150`'s state: stream 7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0150 {
    pub ctrl: Ctrl,
    /// The objects cue 500 started a transfer at since the last take.
    transfers: Vec<Transfer>,
}

/// `Func_str0150`'s `SetFog(1000, 7500, 0, 85, 0x00144870)` on the scene's
/// draw environment, and the object it takes out of it (`fogObj0150`,
/// `fogSW` bit 3 cleared): [`Scene::fog`].
pub const FOG_0150: (f32, f32, f32, f32, u32) = (1000.0, 7500.0, 0.0, 85.0, 0x0014_4870);
pub const FOG_OFF_0150: [&str; 1] = ["OBJ_sfp7bac2"];

/// [`FOG_0150`] as the scene's fog (streams 7, 13 and 16).
fn fog_0150() -> SceneFog {
    let (near, far, near_rate, far_rate, colour) = FOG_0150;
    SceneFog::with_rates(near, far, near_rate, far_rate, colour)
}

/// `charHeightTbl[0]` (0x0034e150, 160.0): the height cue 500's transfer is
/// given.
pub const TRANSFER_HEIGHT: u32 = 0x4320_0000;
/// `charHeightTbl[17]` (0x0034e194, 210.0): `Func_str0610`'s cue 510's.
pub const TRANSFER_HEIGHT_TALL: u32 = 0x4352_0000;

/// `effTransferStr(pos, height, strEffLayer)` (0x001ce220): the arrival's
/// rings (stream effect controller -2) the task starts at the note's
/// object for a character `height` high (f32 bits); not drawn by the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub obj: u32,
    pub height: u32,
}

impl Str0150 {
    /// `Func_str0150` (0x00189320) started: `strEffLayer` given the scene's
    /// view, the fog, then the same objects as [`Str0090::new`]; nothing on.
    pub fn new(rand: &mut Rand) -> Str0150 {
        Str0150 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), transfers: Vec::new() }
    }

    /// One pass: the cues (last queued first) - 899 a fade to black over
    /// the frames left, 500 `effTransferStr(pos, charHeightTbl[0],
    /// strEffLayer)` at the note's object - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                899 => self.ctrl.fade_to_black(frame_now, frame_end),
                500 => self.transfers.push(Transfer { obj, height: TRANSFER_HEIGHT }),
                _ => {}
            }
        }
        self.ctrl.pass(paused, rand)
    }

    /// The transfers the last pass started.
    pub fn take_transfers(&mut self) -> Vec<Transfer> {
        std::mem::take(&mut self.transfers)
    }
}

/// `Func_str0240`'s state: stream 8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0240 {
    pub ctrl: Ctrl,
    /// Hit marks placed so far: the row of [`HIT_ROT_0240`] the next takes.
    pub marks_n: usize,
    rot: Vec<[u32; 3]>,
    marks: Vec<HitMark>,
}

impl Str0240 {
    /// `Func_str0240` (0x0018abb0) started: `GetSubstAdrsF("OBJ_trall")`
    /// (its answer unused), `strEffLayer` given the scene's view, the same
    /// objects as [`Str0090::new`]; nothing on.
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str0240 {
        Str0240 {
            ctrl: Ctrl::new(Some(EFFECT_LAYER), rand),
            marks_n: 0,
            rot: tables.hit_rot_0240.clone(),
            marks: Vec::new(),
        }
    }

    /// The hit marks the last pass started.
    pub fn take_marks(&mut self) -> Vec<HitMark> {
        std::mem::take(&mut self.marks)
    }

    /// One pass: the cues (last queued first) - 899 a fade to black over
    /// the frames left; 600-605, 610 and 611 a hit mark at the note's
    /// object, each taking the next row of [`HIT_ROT_0240`] - then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                899 => self.ctrl.fade_to_black(frame_now, frame_end),
                600..=605 | 610 | 611 => {
                    self.marks.push(hit_mark(&self.rot, obj, self.marks_n));
                    self.marks_n += 1;
                }
                _ => {}
            }
        }
        self.ctrl.pass(paused, rand)
    }
}

/// `Func_str0250`'s state: stream 9.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0250 {
    pub ctrl: Ctrl,
    rot: [u32; 3],
    marks: Vec<HitMark>,
}

/// `Func_str0250`'s turned feedback (cues 910, 920, 930): 1.0, turned
/// 0.00697 radians (0x3be4c389), alpha 0x50.
const REFLEX_ROLL_0250: u32 = 0x3be4_c389;

impl Str0250 {
    /// `Func_str0250` (0x0018b700) started: `strEffLayer` given the scene's
    /// view, the same objects as [`Str0090::new`]; nothing on.
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str0250 {
        Str0250 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), rot: tables.hit_rot_0250, marks: Vec::new() }
    }

    /// The hit marks the last pass started.
    pub fn take_marks(&mut self) -> Vec<HitMark> {
        std::mem::take(&mut self.marks)
    }

    /// One pass: the cues (last queued first), then [`Ctrl::pass`]. 601 a
    /// hit mark ([`HIT_ROT_0250`]); 989 a fade to white over the frames
    /// left; 940 the feedback off and 910, 920, 930 a turned one on, each
    /// with a fade from white over 10 frames, as 900 alone; 25 the feedback
    /// on (1.015, alpha 0x40); 15 (1.03, alpha 0x40) and 5 (1.015, alpha
    /// 0x50) on with a 15-frame fade ready. The rest by their last digit: 1
    /// the noise on, 2 off, 3 the inversion on, 4 off, 6 the feedback
    /// fading.
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                601 => self.marks.push(hit_mark(&[self.rot], obj, 0)),
                989 => c.fade_left(frame_now, frame_end, FADE_WHITE.0, FADE_WHITE.1),
                940 => {
                    c.feedback.off();
                    c.fade.entry(10, FADE_WHITE.1, FADE_WHITE.0);
                }
                910 | 920 | 930 => {
                    c.feedback.reflex_turned(0x3f80_0000, REFLEX_ROLL_0250, 0x5080_8080);
                    c.fade.entry(10, FADE_WHITE.1, FADE_WHITE.0);
                }
                900 => c.fade.entry(10, FADE_WHITE.1, FADE_WHITE.0),
                25 => c.feedback.reflex(REFLEX_SCALE, 0x4080_8080),
                15 => c.feedback.reflex_timed(0x3f83_d70a, 0x4080_8080, 0, 15),
                5 => c.feedback.reflex_timed(REFLEX_SCALE, 0x5080_8080, 0, 15),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    6 => c.feedback.flag = 2,
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0301`'s state: stream 11.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0301 {
    pub ctrl: Ctrl,
    rot: [u32; 3],
    marks: Vec<HitMark>,
}

impl Str0301 {
    /// `Func_str0301` (0x001923b0) started: `strEffLayer` given the scene's
    /// view, the same objects as [`Str0090::new`]; nothing on.
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str0301 {
        Str0301 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), rot: tables.hit_rot_0301, marks: Vec::new() }
    }

    /// One pass: the cues (last queued first) - 899 a fade to black over
    /// the frames left, 601 a hit mark turned by [`HIT_ROT_0301`] - then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                899 => self.ctrl.fade_to_black(frame_now, frame_end),
                601 => self.marks.push(hit_mark(&[self.rot], obj, 0)),
                _ => {}
            }
        }
        self.ctrl.pass(paused, rand)
    }
}

/// `Func_str0305`'s state: stream 12.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0305 {
    pub ctrl: Ctrl,
    transfers: Vec<Transfer>,
}

impl Str0305 {
    /// `Func_str0305` (0x001938d0) started as [`Str0301::new`].
    pub fn new(rand: &mut Rand) -> Str0305 {
        Str0305 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), transfers: Vec::new() }
    }

    /// One pass: the cues (last queued first) - 899 a fade to black over
    /// the frames left, 500 a transfer at the note's object (as
    /// [`Str0150::step`]'s), 5 the feedback on (1.001, alpha 0x40) with a
    /// 20-frame fade ready, 6 into its fading state - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                899 => self.ctrl.fade_to_black(frame_now, frame_end),
                500 => self.transfers.push(Transfer { obj, height: TRANSFER_HEIGHT }),
                5 => self.ctrl.feedback.reflex_timed(0x3f80_20c5, 0x4080_8080, 0, 20),
                6 => self.ctrl.feedback.flag = 2,
                _ => {}
            }
        }
        self.ctrl.pass(paused, rand)
    }
}

/// `Func_str0350`'s state: stream 13.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0350 {
    pub ctrl: Ctrl,
}

impl Str0350 {
    /// `Func_str0350` (0x00192e80) started: `strEffLayer` given the scene's
    /// view, the same objects as [`Str0090::new`], then `Func_str0150`'s
    /// fog ([`FOG_0150`], `fogOffObj` the same one object); nothing on.
    pub fn new(rand: &mut Rand) -> Str0350 {
        Str0350 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand) }
    }

    /// One pass: cue 899 a fade to black over the frames left, then
    /// [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            if cue == 899 {
                self.ctrl.fade_to_black(frame_now, frame_end);
            }
        }
        self.ctrl.pass(paused, rand)
    }
}

/// `Func_str0570`'s state: stream 14.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0570 {
    pub ctrl: Ctrl,
    transfers: Vec<Transfer>,
}

impl Str0570 {
    /// `Func_str0570` (0x0018c780) started: `strEffLayer` given the scene's
    /// view, the same objects as [`Str0090::new`]; nothing on.
    pub fn new(rand: &mut Rand) -> Str0570 {
        Str0570 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), transfers: Vec::new() }
    }

    /// One pass. First the scene's frame: at 1455 a fade from white over 10
    /// frames, at 1470 the fade gone (`DeleteFade`, off). Then the cues
    /// (last queued first): 500 a transfer; 999 a fade to white over the
    /// frames left; 55 the feedback on (1.015, alpha 0x40) with a 32-frame
    /// fade ready, 45 the same without; 25 (1.001, alpha 0x61) with a
    /// 64-frame fade ready, 15 without. The rest by their last digit (jump
    /// table 0x0034e460): 1 the noise on, 2 off, 3 the inversion on, 4 off,
    /// 5 the feedback on (1.015, alpha 0x50), 6 fading, 7 and 8 a one-frame
    /// flash to and from white. Then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        if frame_now == 1455 {
            c.fade.entry(10, FADE_WHITE.1, FADE_WHITE.0);
        }
        if frame_now == 1470 {
            c.fade.on = false;
        }
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                500 => self.transfers.push(Transfer { obj, height: TRANSFER_HEIGHT }),
                999 => c.fade_left(frame_now, frame_end, FADE_WHITE.0, FADE_WHITE.1),
                55 => c.feedback.reflex_timed(REFLEX_SCALE, 0x4080_8080, 0, 32),
                45 => c.feedback.reflex(REFLEX_SCALE, 0x4080_8080),
                25 => c.feedback.reflex_timed(0x3f80_20c5, 0x6180_8080, 0, 64),
                15 => c.feedback.reflex(0x3f80_20c5, 0x6180_8080),
                _ => match cue % 10 {
                    1 => c.noise_start(rand),
                    2 => c.noise_on = false,
                    3 => c.reverse = true,
                    4 => c.reverse = false,
                    5 => c.feedback.reflex(REFLEX_SCALE, 0x5080_8080),
                    6 => c.feedback.flag = 2,
                    7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                    8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                    _ => {}
                },
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0610`'s state: stream 16.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0610 {
    pub ctrl: Ctrl,
    transfers: Vec<Transfer>,
}

impl Str0610 {
    /// `Func_str0610` (0x00196c00) started: `strEffLayer` given the scene's
    /// view, `Func_str0150`'s fog ([`FOG_0150`], `fogObj0610` the same one
    /// object), the same objects as [`Str0090::new`]; nothing on.
    pub fn new(rand: &mut Rand) -> Str0610 {
        Str0610 { ctrl: Ctrl::new(Some(EFFECT_LAYER), rand), transfers: Vec::new() }
    }

    /// One pass: the cues (last queued first) - 889 a fade to black over
    /// the frames left; 800 from black over 20 frames; 899 to black over
    /// 30; 6 the feedback fading; 5 the feedback on (1.01, alpha 0x40) with
    /// a 10-frame fade ready; 510 a transfer for a character 210 high, 500
    /// one 160 high - then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let c = &mut self.ctrl;
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                889 => c.fade_to_black(frame_now, frame_end),
                800 => c.fade.entry(20, 0x8000_0000, 0),
                899 => c.fade.entry(30, 0, 0x8000_0000),
                6 => c.feedback.flag = 2,
                5 => c.feedback.reflex_timed(0x3f81_47ae, 0x4080_8080, 0, 10),
                510 => self.transfers.push(Transfer { obj, height: TRANSFER_HEIGHT_TALL }),
                500 => self.transfers.push(Transfer { obj, height: TRANSFER_HEIGHT }),
                _ => {}
            }
        }
        c.pass(paused, rand)
    }
}

/// `Func_str0120`'s state: stream 5, Skeith and Orca.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0120 {
    pub ctrl: Ctrl,
    /// The hit marks cues 601-603 and 610-614 have placed, each counting
    /// its own way through `hitRot0120a`.
    pub marks_a: usize,
    pub marks_b: usize,
    /// Cue 12's `divZ` (f32 bits), once set.
    pub div_z: Option<u32>,
    rot: Vec<[u32; 3]>,
    /// This pass's marks.
    marks: Vec<HitMark>,
}

impl Str0120 {
    /// `Func_str0120` (0x001950b0) started: as `Func_str0300` (its layer,
    /// objects and noise bands; nothing on; the feedback on its own layer),
    /// and the effect objects' layer (`strEffLayer`) given the scene's view.
    pub fn new(tables: &Tables, rand: &mut Rand) -> Str0120 {
        Str0120 {
            ctrl: Ctrl::new(Some(EFFECT_LAYER), rand),
            marks_a: 0,
            marks_b: 0,
            div_z: None,
            rot: tables.hit_rot_0120a.clone(),
            marks: Vec::new(),
        }
    }

    /// A hit mark at `obj` with the `n`th rotation.
    fn mark(&mut self, obj: u32, n: usize) {
        self.marks.push(hit_mark(&self.rot, obj, n));
    }

    /// The hit marks the last pass started.
    pub fn take_marks(&mut self) -> Vec<HitMark> {
        std::mem::take(&mut self.marks)
    }

    /// One pass: the cues (last queued first), then [`Ctrl::pass`].
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        let _ = (frame_now, frame_end);
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            match cue {
                12 => {
                    // The scene's view: VU1 clips nearer, at 500.
                    self.div_z = Some(0x43fa_0000);
                    self.ctrl.noise_on = false;
                }
                5 => self.ctrl.feedback.reflex_timed(0x3f85_1eb8, 0x6680_8080, 24, 32),
                610..=614 => {
                    self.mark(obj, self.marks_b);
                    self.marks_b += 1;
                    self.ctrl.feedback.reflex_timed(0x3f82_8f5c, 0x6080_8080, 2, 10);
                }
                601..=603 => {
                    self.mark(obj, self.marks_a);
                    self.marks_a += 1;
                }
                800 => self.ctrl.fade.entry(10, FADE_WHITE.1, FADE_WHITE.0),
                899 => self.ctrl.fade.entry(1, 0x8000_0000, 0x8000_0000),
                999 => self.ctrl.fade.entry(9, FADE_WHITE.0, FADE_WHITE.1),
                _ => match cue % 10 {
                    1 => self.ctrl.noise_start(rand),
                    2 => self.ctrl.noise_on = false,
                    3 => self.ctrl.reverse = true,
                    4 => self.ctrl.reverse = false,
                    5 => self.ctrl.feedback.reflex(0x3f83_d70a, 0x3080_8080),
                    6 => self.ctrl.feedback.off(),
                    _ => {}
                },
            }
        }
        self.ctrl.pass(paused, rand)
    }
}

/// The effect task a scene runs, of those ported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Task {
    Str0001(Str0001),
    Str0090(Str0090),
    Str0110(Str0110),
    Str0120(Str0120),
    Str0130(Str0130),
    Str0150(Str0150),
    Str0240(Str0240),
    Str0250(Str0250),
    Str0300(Str0300),
    Str0301(Str0301),
    Str0305(Str0305),
    Str0350(Str0350),
    Str0570(Str0570),
    Str0610(Str0610),
    Str8000(Str8000),
    Str9000(Str9000),
    Str9001(Str9001),
    Str7100(Str7100),
    Str8800(Str8800),
    Str0580(Box<crate::ending::Str0580>),
    Str0581(Box<crate::ending::Str0581>),
}

/// The scene a task reads objects from: `GetSubstAdrsF` over the scene
/// file ([`find_entry`]) and the node's `lwMatrix` translation now.
pub struct SceneWorld<'a> {
    pub loaded: &'a Loaded,
    pub scene: &'a Scene,
}

impl crate::ending::World for SceneWorld<'_> {
    fn find(&self, name: &str) -> Option<u32> {
        find_entry(self.loaded, self.scene.file, name)
    }

    fn position(&self, obj: u32) -> [u32; 4] {
        let w = self.scene.by_obj.get(&obj).map_or([[0; 4]; 4], |&i| self.scene.world(i));
        [w[3][0], w[3][1], w[3][2], w[3][3]]
    }
}

impl Task {
    /// `ccSetStreamDemoThread`'s task for the scene file `stem`
    /// ([`task_name`]), started on `scene`; None when it has none or it is
    /// not ported.
    pub fn start(stem: &str, loaded: &Loaded, scene: &mut Scene, tables: &Tables, rand: &mut Rand) -> Option<Task> {
        match task_name(stem).as_str() {
            STR0001 => Some(Task::Str0001(Str0001::start(loaded, scene, rand))),
            STR0090 | STR0110 => {
                set_fog(loaded, scene, SceneFog::new(FOG_0090.0, FOG_0090.1, FOG_0090.2), &[]);
                Some(if task_name(stem) == STR0090 {
                    Task::Str0090(Str0090::new(rand))
                } else {
                    Task::Str0110(Str0110::new(rand))
                })
            }
            STR0120 => Some(Task::Str0120(Str0120::new(tables, rand))),
            STR0130 => Some(Task::Str0130(Str0130::start(scene, rand))),
            STR0150 => {
                set_fog(loaded, scene, fog_0150(), &FOG_OFF_0150);
                Some(Task::Str0150(Str0150::new(rand)))
            }
            STR0240 => Some(Task::Str0240(Str0240::new(tables, rand))),
            STR0250 => Some(Task::Str0250(Str0250::new(tables, rand))),
            STR0301 => Some(Task::Str0301(Str0301::new(tables, rand))),
            STR0305 => Some(Task::Str0305(Str0305::new(rand))),
            STR0350 => {
                set_fog(loaded, scene, fog_0150(), &FOG_OFF_0150);
                Some(Task::Str0350(Str0350::new(rand)))
            }
            STR0570 => Some(Task::Str0570(Str0570::new(rand))),
            STR0610 => {
                set_fog(loaded, scene, fog_0150(), &FOG_OFF_0150);
                Some(Task::Str0610(Str0610::new(rand)))
            }
            STR8000 | STR9102 => Some(Task::Str8000(Str8000::new(rand))),
            STR9104 => {
                let (near, far, near_rate, far_rate, colour) = FOG_9000;
                set_fog(loaded, scene, SceneFog::with_rates(near, far, near_rate, far_rate, colour), &[]);
                Some(Task::Str9000(Str9000::start(scene, tables, rand)))
            }
            STR9101 => Some(Task::Str9001(Str9001::new(rand))),
            STR7100 => Some(Task::Str7100(Str7100::new(rand))),
            STR8800 => Some(Task::Str8800(Str8800::new(rand))),
            STR0300 => Some(Task::Str0300(Str0300::new(rand))),
            crate::ending::STR0580 => {
                let world = SceneWorld { loaded, scene };
                Some(Task::Str0580(Box::new(crate::ending::Str0580::new(&tables.ending, &world, rand))))
            }
            crate::ending::STR0581 => {
                let (near, far, colour) = crate::ending::FOG_0581;
                set_fog(loaded, scene, SceneFog::new(near, far, colour), &crate::ending::FOG_OFF_0581);
                let world = SceneWorld { loaded, scene };
                Some(Task::Str0581(Box::new(crate::ending::Str0581::new(&tables.ending, &world, rand))))
            }
            _ => None,
        }
    }

    /// One pass of the task's loop (see [`Str0001::step`]).
    pub fn step(&mut self, cues: &[Cue], frame_now: u32, frame_end: u32, paused: bool, rand: &mut Rand) -> Draws {
        self.step_in(&crate::ending::NoWorld, cues, frame_now, frame_end, paused, rand)
    }

    /// [`Task::step`] with the scene's objects (stream 15's tasks read
    /// them).
    pub fn step_in(
        &mut self,
        world: &dyn crate::ending::World,
        cues: &[Cue],
        frame_now: u32,
        frame_end: u32,
        paused: bool,
        rand: &mut Rand,
    ) -> Draws {
        match self {
            Task::Str0580(t) => t.step(world, cues, frame_now, paused, rand),
            Task::Str0581(t) => t.step(world, cues, frame_now, paused, rand),
            Task::Str0001(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0090(t) => t.step(cues, paused, rand),
            Task::Str0110(t) => t.step(cues, paused, rand),
            Task::Str0120(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0130(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0150(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0240(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0250(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0300(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0301(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0305(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0350(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0570(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str0610(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str8000(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str9000(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str9001(t) => t.step(cues, frame_now, frame_end, paused, rand),
            Task::Str7100(t) => t.step(cues, paused, rand),
            Task::Str8800(t) => t.step(cues, frame_now, frame_end, paused, rand),
        }
    }

    /// The hit marks the last pass started.
    pub fn take_marks(&mut self) -> Vec<HitMark> {
        match self {
            Task::Str0120(t) => t.take_marks(),
            Task::Str0240(t) => t.take_marks(),
            Task::Str0250(t) => t.take_marks(),
            Task::Str0301(t) => std::mem::take(&mut t.marks),
            _ => Vec::new(),
        }
    }

    /// The transfers the last pass started.
    pub fn take_transfers(&mut self) -> Vec<Transfer> {
        match self {
            Task::Str0150(t) => t.take_transfers(),
            Task::Str0305(t) => std::mem::take(&mut t.transfers),
            Task::Str0570(t) => std::mem::take(&mut t.transfers),
            Task::Str0610(t) => std::mem::take(&mut t.transfers),
            Task::Str0581(t) => t.take_transfers(),
            _ => Vec::new(),
        }
    }

    /// Stream 15's parts drawn in the last pass.
    pub fn part_draws(&self) -> &[crate::ending::PartDraw] {
        match self {
            Task::Str0580(t) => t.part_draws(),
            Task::Str0581(t) => t.part_draws(),
            _ => &[],
        }
    }

    pub fn ctrl(&self) -> &Ctrl {
        match self {
            Task::Str0001(t) => &t.ctrl,
            Task::Str0090(t) => &t.ctrl,
            Task::Str0110(t) => &t.ctrl,
            Task::Str0120(t) => &t.ctrl,
            Task::Str0130(t) => &t.ctrl,
            Task::Str0150(t) => &t.ctrl,
            Task::Str0240(t) => &t.ctrl,
            Task::Str0250(t) => &t.ctrl,
            Task::Str0300(t) => &t.ctrl,
            Task::Str0301(t) => &t.ctrl,
            Task::Str0305(t) => &t.ctrl,
            Task::Str0350(t) => &t.ctrl,
            Task::Str0570(t) => &t.ctrl,
            Task::Str0610(t) => &t.ctrl,
            Task::Str8000(t) => &t.ctrl,
            Task::Str9000(t) => &t.ctrl,
            Task::Str9001(t) => &t.ctrl,
            Task::Str7100(t) => &t.ctrl,
            Task::Str8800(t) => &t.ctrl,
            Task::Str0580(t) => &t.ctrl,
            Task::Str0581(t) => &t.ctrl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// newlib's generator from its seed: the first numbers.
    #[test]
    fn rand_is_newlibs() {
        let mut r = Rand::default();
        let v: Vec<i32> = (0..3).map(|_| r.rand()).collect();
        assert_eq!(v, [1_481_765_933, 1_085_377_743, 1_270_216_262]);
    }

    #[test]
    fn task_names_fold() {
        assert_eq!(task_name("str0001"), "str0001");
        assert_eq!(task_name("str0001p"), "str0001");
        assert_eq!(task_name("str7123"), "str7100");
        assert_eq!(task_name("str8812"), "str8800");
        assert_eq!(task_name("str8234"), "str8000");
        assert_eq!(task_name("str6100"), "str6100");
    }

    /// `ccRasterNoize::Init` makes 27-row bands (24 lines of 384 truncated
    /// on the EE) and draws 8 x 27 numbers; the task's first state.
    #[test]
    fn start_draws_216_numbers() {
        let mut r = Rand::default();
        let s = Str0001::new(Some(8), &mut r);
        assert!(s.ctrl.noise.iter().all(|n| n.band.h == 27));
        let mut check = Rand::default();
        for _ in 0..216 {
            check.rand();
        }
        assert_eq!(r, check);
        assert_eq!(check.next, 5_194_877_864_407_658_025);
    }
}
