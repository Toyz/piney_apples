//! The Flag Race, from Mutation on (MUT gcmn race.cpp, 0x005fca40 to
//! 0x005ffa40): the three flags (gimmick 45, [`Flag`]) `ccSetChibiGuso`
//! puts in a town whose three pens hold grown Grunties, and the race's
//! task (`PG_RACE`, 0x005ff680, priority 63) with its object (176 bytes,
//! 0x005fd120): the Grunty brought out, the countdown, the ride with its
//! timer, the finish or the quit, the result, and Kite set down by the
//! breeder. Menu 88 starts it (piney-fieldui); the ride is the town's
//! [`crate::town_ride`]. The steps are in docs/engine/flag-race.md.

use std::rc::Rc;

use glam::Mat4;
use piney_battle::ride::RaceRide;
use piney_data::archive::Archive;
use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;
use piney_desktop::message::WindowTexture;
use piney_desktop::sprite::Sprite;
use piney_desktop::view::LayerView;

use crate::ee::{self, F, ONE, V4};
use crate::foe::Model;
use crate::merchant::EntryObj;
use crate::pose::Play;

/// The flags' gimmick (`gimmickTbl[45]`, `PG_FLAG`, `XP_FLAG.CCS`).
pub const FLAG_GIMMICK: usize = 45;
/// A Grunty's kind is its `npcTbl` row less 145.
pub const ROW_KIND0: i32 = 145;
/// The timer stops at 18000 frames (ten minutes).
pub const TIME_LIMIT: i16 = 18000;
/// `saveData +0x8432`: each town's three ranks (servers 1-4).
pub const RACE_RECORDS: usize = 0x8432;
/// `saveData +0x8462`: the prizes each rank has given, 3 a town.
pub const RACE_PRIZES: usize = 0x8462;

/// The fades' colours: clear, then black.
const CLEAR: u32 = 0;
const BLACK: u32 = 0x8000_0000;
/// The race's layers: the 3D clips (`+0x50`, priority 200) and the timer
/// and flags (`+0x54`, 100).
pub const CLIP_LAYER: i16 = 200;
pub const HUD_LAYER: i16 = 100;
/// A flag is taken within 200 of Kite; drawn within 8500.
const TAKE_DIST: F = 0x4348_0000;
const DRAW_DIST: F = 0x4604_d000;
/// 1/15: the taken flag's map mark fades a frame.
const MARK_FADE: F = 0x3d88_8889;

/// The race's files: `xp_flag`, `xp_text`, `xp_cup` (`ccAddRequestFileListInu`
/// loads them with the town's Grunties).
pub const FLAG_FILE: &str = "xp_flag";
pub const TEXT_FILE: &str = "xp_text";
pub const CUP_FILE: &str = "xp_cup";

/// What the race does around the town the frame it runs, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RaceEvent {
    /// `ccSeOn(n)`: the countdown's.
    Se(i32),
    /// `ccSeOn3D(n, pos)`: a flag taken (240).
    Se3d(i32, V4),
    /// `effOpenBox(pos)`: the sparks of a flag taken.
    OpenBox(V4),
    /// `ccBgmPlay(n)`, `ccBgmStop()`: `VOICE/BGM.BIN`.
    Bgm(i32),
    BgmStop,
    /// `ccPgBgmInit()` in a town: the sequences out, the breeder's tune
    /// held off (`ccSnd +0x13a` 1).
    PgBgmInit,
    /// The end: `ccSnd +0x13a` 0, then sequence 1 in from nothing to 256
    /// over 30 frames (main 0x0017c1e0).
    PgBgmEnd,
    /// The riding Grunty's notes (`ccSeSetParamInu(param, pcgs)`, at its
    /// place) and its dust at a leg (`effSmoke`).
    RideSound {
        param: u32,
        attribute: u32,
        pos: V4,
    },
    RideSmoke {
        pos: V4,
        v: V4,
        s: F,
        life: i32,
        t: i32,
    },
    /// `ccMenu` +0xc (the panels), +0x10 (the map), +0xfe (the ban).
    Panel(i16),
    Map(i16),
    Forbid(i16),
}

/// The fader the race draws its fades with (`ccMenu.menuFade`, +0xb8).
pub trait RaceHost {
    /// `EntryFade(n, 0, col1)` from clear: the element, -1 when none is
    /// free.
    fn entry_fade(&mut self, n: i16, col1: u32) -> i32;
    /// `ContinueFade(i, n, col1)`.
    fn continue_fade(&mut self, i: i32, n: i16, col1: u32);
    /// `CheckFade(i)`: still counting.
    fn check_fade(&mut self, i: i32) -> bool;
    /// `DeleteFade(i)`.
    fn delete_fade(&mut self, i: i32);
}

/// No fader: every fade done at once (tests and probes).
pub struct NoFades;

impl RaceHost for NoFades {
    fn entry_fade(&mut self, _n: i16, _col1: u32) -> i32 {
        0
    }
    fn continue_fade(&mut self, _i: i32, _n: i16, _col1: u32) {}
    fn check_fade(&mut self, _i: i32) -> bool {
        false
    }
    fn delete_fade(&mut self, _i: i32) {}
}

// --- the save --------------------------------------------------------------------

/// Main 0x0017a860: a race's time into town `server`'s three ranks. The
/// first rank whose time (the town's next racer's, `ranks`, where the
/// save holds 0) it beats takes it, the lower ones moving down; that rank
/// (1-3) back. Otherwise 4 within 30 frames of the third, else 0.
pub fn enter_time(save: &mut SaveData, ranks: &[(i16, i16); 3], server: i32, time: i16, row: i16) -> i8 {
    let at = RACE_RECORDS + 12 * (server - 1) as usize;
    let mut racer = ranks.iter();
    let mut last = 0i16;
    for k in 0..3 {
        last = match save.i16(at + 4 * k) {
            0 => racer.next().map_or(0, |r| r.0),
            t => t,
        };
        if time < last {
            for j in (k + 1..3).rev() {
                let (t, r) = (save.i16(at + 4 * (j - 1)), save.i16(at + 4 * (j - 1) + 2));
                save.set_i16(at + 4 * j, t);
                save.set_i16(at + 4 * j + 2, r);
            }
            save.set_i16(at + 4 * k, time);
            save.set_i16(at + 4 * k + 2, row);
            return k as i8 + 1;
        }
    }
    if i32::from(time) - 30 < i32::from(last) { 4 } else { 0 }
}

/// 0x005ff3d0: a time in frames as hours, minutes, seconds and hundredths
/// (`(frames % 30) * 100 / 30`), each a byte, the remainders taken in
/// shorts.
pub fn split(t: i16) -> [i8; 4] {
    let h = (i32::from(t) / 108_000) as i8;
    let t = (i32::from(t) - 108_000 * i32::from(h)) as i16;
    let m = (i32::from(t) / 1800) as i8;
    let t = (i32::from(t) - 1800 * i32::from(m)) as i16;
    let s = (i32::from(t) / 30) as i8;
    let t = (i32::from(t) - 30 * i32::from(s)) as i16;
    let hh = (100 * i32::from(t) / 30) as i8;
    [h, m, s, hh]
}

// --- the flags -------------------------------------------------------------------

/// A flag's state (`actNum`, +0x1d4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagState {
    /// Out: taken when Kite comes within 200.
    Out,
    /// Taken: its map mark fading over 16 frames.
    Taken,
    /// Gone: neither drawn nor marked (the state it is made in).
    Hidden,
}

/// A flag (`ccGimmick` of gimmick 45, 512 bytes; vtable 0x0038a9d0): made
/// by 0x005fca40(n) at the town's marker `n`, kept in 0x00774a90[n].
pub struct Flag {
    /// +0x1e0: which of the three.
    pub n: usize,
    /// +0x40 `pos`, +0x60 `dirc` (0, 0, -pi).
    pub pos: V4,
    pub dirc: V4,
    pub entry: EntryObj,
    /// +0x88 `transparency`, +0x8c `setTransparency`.
    pub transparency: F,
    pub set_transparency: F,
    /// The base's +0x18 height (`gimmickTbl[45]`).
    pub height: F,
    pub state: FlagState,
    /// +0x1e8 the fade's count, +0x1ec the map mark's alpha (0 to 1, read
    /// by `DrawMap` through 0x005fd100), +0x1f0 taken.
    pub cnt: i32,
    pub alpha: F,
    pub taken: bool,
    /// `CMP_trall` of `xp_flag` with its palette (`CLT_xpflag01c1`, `c2`
    /// for the second and third), `ANM_xpflnut0` on it.
    model: Option<Rc<Model>>,
    play: Option<Play>,
    /// What the last frame drew: its alpha.
    drawn: Option<F>,
}

/// What a flag's frame asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlagFrame {
    pub taken: bool,
}

impl Flag {
    /// 0x005fca40(n) and the constructor (0x005fcba0): at the town's
    /// marker for flag `n` (`flag_markers[town][n]`), turned -pi, its
    /// clump's palette swapped for the second and third, `ANM_xpflnut0`
    /// set, fog off; off the command lists for good, hidden, its mark at 1.
    pub fn new(
        archive: &Archive,
        volume: Volume,
        town_file: &SceneFile,
        town: i32,
        n: usize,
    ) -> piney_data::Result<Option<Flag>> {
        let t = piney_data::tables::race::of(volume);
        let Some(Some(markers)) = usize::try_from(town).ok().and_then(|k| t.flag_markers().get(k)) else {
            return Ok(None);
        };
        let Some((pos, _)) = crate::grunty::dummy(town_file, markers[n]) else { return Ok(None) };
        let file = Rc::new(SceneFile::read(archive, FLAG_FILE)?);
        let mut model = Model::new(file.clone(), "CMP_trall")?;
        if let Some(Some(clut)) = t.flag_cluts().get(n) {
            model.change_clut("MAT_clut", clut)?;
        }
        let play = model.play("ANM_xpflnut0");
        let base = &piney_data::tables::battle::of(volume).gimmicks()[FLAG_GIMMICK].param.base;
        let mut entry = EntryObj::default();
        entry.delete_cmnd(true);
        Ok(Some(Flag {
            n,
            pos,
            dirc: [0, 0, ee::deg2rad(-32768), 0],
            entry,
            transparency: ONE,
            set_transparency: ONE,
            height: base.height.to_bits(),
            state: FlagState::Hidden,
            cnt: 0,
            alpha: ONE,
            taken: false,
            model: Some(Rc::new(model)),
            play,
            drawn: None,
        }))
    }

    /// One frame on the entry control's list: `ccEntryObj::routine`, then
    /// its main ([`Flag::main`]).
    pub fn step(&mut self, volume: Volume, player: V4, cam: &crate::camera::Cam) -> FlagFrame {
        let mut dirc = self.dirc;
        self.entry.routine_parts(
            volume,
            self.pos,
            &mut dirc,
            &mut self.set_transparency,
            &mut self.transparency,
            player,
        );
        self.dirc = dirc;
        let pos = self.pos;
        self.main(|| {
            let mut hide = false;
            crate::char::camera_transparency(
                volume,
                pos,
                player,
                cam.pos,
                cam.deg[1],
                0,
                0,
                0x45da_c000,
                0x4416_0000,
                &mut hide,
            )
        })
    }

    /// Its main (0x005fce60, carried as `ccBossEffLaserRain2::Draw`): out,
    /// Kite within 200 takes it (its fade out over 15, sound 240, the
    /// sparks, the race told); taken, its map mark fades 1/15 a frame for
    /// 17 frames, then it is gone. Within 8500 and not gone it is drawn at
    /// `fade` (`ccGetCameraTransparency(pos, 0, 0, 7000, 600)`) times its
    /// `setTransparency`.
    pub fn main(&mut self, fade: impl FnOnce() -> F) -> FlagFrame {
        let mut out = FlagFrame::default();
        match self.state {
            FlagState::Out => {
                if ee::lt(self.entry.pl_dist, TAKE_DIST) {
                    self.entry.fade_flag = 2;
                    self.entry.fade_cnt = 15;
                    self.state = FlagState::Taken;
                    self.taken = true;
                    self.cnt = 0;
                    out.taken = true;
                }
                self.alpha = ONE;
            }
            FlagState::Taken => {
                let old = self.cnt;
                self.cnt += 1;
                if old >= 16 {
                    self.state = FlagState::Hidden;
                    self.cnt = 0;
                }
                self.alpha = ee::sub(self.alpha, MARK_FADE);
                if ee::lt(self.alpha, 0) {
                    self.alpha = 0;
                }
            }
            FlagState::Hidden => self.alpha = 0,
        }
        self.drawn = None;
        if ee::lt(self.entry.pl_dist, DRAW_DIST) && self.state != FlagState::Hidden {
            if let (Some(p), Some(m)) = (self.play.as_mut(), self.model.as_ref()) {
                p.forward(m.file());
            }
            self.drawn = Some(ee::mul(fade(), self.set_transparency));
        }
        out
    }

    /// Not the game's: a flag of no clump, at `pos`, for the tests.
    pub fn bare(n: usize, pos: V4, height: F) -> Flag {
        Flag {
            n,
            pos,
            dirc: [0, 0, ee::deg2rad(-32768), 0],
            entry: EntryObj::default(),
            transparency: ONE,
            set_transparency: ONE,
            height,
            state: FlagState::Hidden,
            cnt: 0,
            alpha: ONE,
            taken: false,
            model: None,
            play: None,
            drawn: None,
        }
    }

    /// The alpha the last frame drew it at, if it did.
    pub fn drawn(&self) -> Option<F> {
        self.drawn
    }

    /// What the last frame drew: the clump at its place on the character
    /// layer (`SetActiveLayer(5)`), its shadow's alpha from the
    /// transparency and its length three times the height.
    pub fn draw(&self, layers: &mut Layers, to_screen: Mat4, lights: &crate::town::TownLights) {
        let (Some(t), Some(p), Some(m)) = (self.drawn, self.play.as_ref(), self.model.as_ref()) else { return };
        let root = crate::body::root(self.pos, self.dirc);
        crate::draw::char_shadow(layers, t, self.height, |layers| {
            m.draw(layers, crate::draw::CHAR_LAYER, to_screen, p, root, ee::f(t), Some((lights, false)), None)
        });
    }
}

// --- the race --------------------------------------------------------------------

/// Where the race's task stands: each a place it breathes (yields the
/// frame) and goes on from the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pc {
    /// Not yet run: the object is made, then the set-up (0x005fd820).
    Start,
    /// The set-up's fade out.
    SetupOut,
    /// The intro's loop (0x005fdaf0): at its top, inside step 1's wait for
    /// the fade, after the loop's breath.
    IntroTop,
    IntroFade,
    IntroEnd,
    /// The set-up after the intro's breath, then its fade in.
    SetupCam,
    SetupIn,
    /// The main loop (0x005fded0) after its breath.
    Main,
    /// The finish's (0x005fe290) and the quit's (0x005feab0) fades.
    FinishOut,
    FinishIn,
    QuitOut,
    QuitIn,
    /// The end: the fade out, the breath after Kite is placed, the fade in.
    EndOut,
    EndCam,
    EndIn,
    /// The task waits for the ride to be gone (`pcgs`), then ends.
    WaitRide,
    /// Ended: the object deleted.
    Gone,
}

/// The clips the race draws (`ccAnm`s +0x08, +0x10 .. +0x2c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    /// `ANM_xp_count_a` of `xp_text`: 3, 2, 1, GO.
    Count,
    /// The cup and the place by rank (`xp_cup`), and `ANM_xpchnut0`.
    Cup,
    Place,
    Chnut,
    /// `ANM_xp_out_a` (no rank), `ANM_xp_over_a` (time over).
    Out,
    Over,
}

/// A clip as the race drew it: its `ccAnm` (the objects' own records:
/// their scale, transparency, display) at the place in front of the
/// camera (0x005ff880).
#[derive(Clone)]
struct ClipDraw {
    anm: piney_desktop::anm::Anm,
    pos: V4,
    rot: V4,
}

/// One sprite of the HUD as `MakePacket` put it: the texture (0 the
/// timer's digits, 1-3 the flags), the cell, its place and size, its
/// texels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudCell {
    pub tex: usize,
    pub code: i32,
    pub dx: f32,
    pub dy: f32,
    pub sx: f32,
    pub sy: f32,
    pub su: i32,
    pub sv: i32,
}

/// The race's files.
pub struct RaceFiles {
    text: Rc<SceneFile>,
    cup: Rc<SceneFile>,
    /// `TEX_xp_tim_1` and `TEX_xpflag02` .. `04` of `xp_text`.
    tex: [Option<WindowTexture>; 4],
}

impl RaceFiles {
    pub fn read(archive: &Archive) -> piney_data::Result<RaceFiles> {
        let text = Rc::new(SceneFile::read(archive, TEXT_FILE)?);
        let cup = Rc::new(SceneFile::read(archive, CUP_FILE)?);
        let tex = ["TEX_xp_tim_1", "TEX_xpflag02", "TEX_xpflag03", "TEX_xpflag04"]
            .map(|t| WindowTexture::read_named(archive, TEXT_FILE, t, None));
        Ok(RaceFiles { text, cup, tex })
    }

    /// `ccAnm::SetAnm` of a clip of `xp_text` or `xp_cup`.
    fn anm(&self, c: Clip, name: &str) -> Option<piney_desktop::anm::Anm> {
        let file = match c {
            Clip::Count | Clip::Out | Clip::Over => &self.text,
            Clip::Cup | Clip::Place | Clip::Chnut => &self.cup,
        };
        let mut a = piney_desktop::anm::Anm::new();
        a.set(file, name);
        a.is_set().then_some(a)
    }
}

/// The race object (0x005fd120) and its task's place.
pub struct Race {
    /// +0x74 the Grunty's kind (its row less 145), +0x7c `game.server`.
    pub kind: i32,
    pub server: i32,
    /// +0x64-+0x70: the ride's handling (by kind, or by town for kind 0).
    pub ride: RaceRide,
    /// +0x78: the fade's element.
    fade: i32,
    /// +0x80 the phase (0 the countdown, 1 riding, 2 the end's result, 3
    /// and 5 done), +0x84 the result's step, +0x88 its count.
    pub phase: i32,
    pub sub: i32,
    pub cnt: i32,
    /// +0x8c the countdown's frames, +0x8e its number (4 down to 0, then
    /// counting the GO's frames up).
    pub tick: i16,
    pub count: i16,
    /// +0x90 the time in frames, +0x92 the time each flag was taken.
    pub time: i16,
    pub splits: [i16; 3],
    /// +0x98: the intro's step, which the Grunty raced steps on (2-5) and
    /// the end sets to 7.
    pub intro: i16,
    /// +0x9a: the time's hours, minutes, seconds, hundredths.
    pub digits: [i8; 4],
    /// +0x9e the flags counted, +0x9f the flags in the order taken.
    pub taken: i8,
    pub order: [i8; 3],
    /// +0xa2 the rank (1-3, 4 near, 0 none), +0xa3 the state (1 over, 2
    /// the result shown), +0xa4 the timer running (the race can be
    /// paused), +0xa5 the menu done, +0xa6 restart (never set), +0xa7 the
    /// end (the ride leaves).
    pub rank: i8,
    pub state: i8,
    pub running: bool,
    pub done: bool,
    pub end: bool,
    pc: Pc,
    files: Rc<RaceFiles>,
    /// `ccAnm`s: the countdown's (+0x08) and the chain's (+0x20), the
    /// result's (+0x10, +0x18, +0x28, +0x2c) once set.
    count_play: Option<piney_desktop::anm::Anm>,
    chnut_play: Option<piney_desktop::anm::Anm>,
    cup_play: Option<piney_desktop::anm::Anm>,
    place_play: Option<piney_desktop::anm::Anm>,
    out_play: Option<piney_desktop::anm::Anm>,
    over_play: Option<piney_desktop::anm::Anm>,
    /// What the last frame drew (kept while the tasks sleep).
    clips: Vec<ClipDraw>,
    cells: Vec<HudCell>,
    /// The events of the frames since the last take.
    pub events: Vec<RaceEvent>,
}

impl Race {
    /// 0x005ff820(kind)'s task, its object made on the first run.
    pub fn new(files: Rc<RaceFiles>, kind: i32, server: i32) -> Race {
        Race {
            kind,
            server,
            ride: RaceRide::default(),
            fade: -1,
            phase: 0,
            sub: 0,
            cnt: 0,
            tick: 0,
            count: 4,
            time: 0,
            splits: [0; 3],
            intro: 0,
            digits: [0; 4],
            taken: 0,
            order: [-1; 3],
            rank: 0,
            state: 0,
            running: false,
            done: false,
            end: false,
            pc: Pc::Start,
            count_play: files.anm(Clip::Count, "ANM_xp_count_a"),
            chnut_play: files.anm(Clip::Chnut, "ANM_xpchnut0"),
            cup_play: None,
            place_play: None,
            out_play: None,
            over_play: None,
            files,
            clips: Vec::new(),
            cells: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Whether the task has ended (the globals cleared).
    pub fn gone(&self) -> bool {
        self.pc == Pc::Gone
    }

    /// 0x005ff740(n): a flag taken goes in the order.
    pub fn flag_taken(&mut self, n: usize) {
        if let Some(o) = usize::try_from(self.taken).ok().and_then(|k| self.order.get_mut(k)) {
            *o = n as i8;
        }
    }

    /// What the last frame's HUD drew, each `MakePacket` in order.
    pub fn hud_cells(&self) -> &[HudCell] {
        &self.cells
    }

    /// Menu 88's quit: the timer stopped, the state over.
    pub fn quit(&mut self) {
        self.running = false;
        self.state = 1;
    }

    /// What the last frame drew: the clips on their layer (through the
    /// world's camera, `world_screen`), the timer and flags on theirs.
    pub fn draw(&self, ctx: &mut piney_desktop::anm::Ctx, world_screen: &[V4; 4]) {
        let view = piney_desktop::view::View::of_world_screen(*world_screen);
        for c in &self.clips {
            let mut a = c.anm.clone();
            a.set_pos_rot(ee::f3(c.pos), ee::f3(c.rot));
            a.draw_on(ctx, CLIP_LAYER, &view);
        }
        let layers = &mut ctx.layers;
        let view = LayerView::default_layer();
        let mut cur: Option<(usize, Sprite)> = None;
        for c in &self.cells {
            if cur.as_ref().is_none_or(|(t, _)| *t != c.tex) {
                if let Some((_, mut s)) = cur.take() {
                    s.send(layers);
                }
                let Some(Some(t)) = self.files.tex.get(c.tex) else { continue };
                cur = Some((c.tex, Sprite::mask(HUD_LAYER, t.tex.clone(), t.tex_h, 8)));
            }
            let Some((_, s)) = cur.as_mut() else { continue };
            (s.dx, s.dy, s.sx, s.sy, s.su, s.sv) = (c.dx, c.dy, c.sx, c.sy, c.su, c.sv);
            (s.wu, s.wv, s.wi) = (0, 0, if c.tex == 0 { 8 } else { 1 });
            s.make_packet(c.code as u8, &view);
        }
        if let Some((_, mut s)) = cur {
            s.send(layers);
        }
    }
    /// 0x005fdd10's count, phase 0: when the number is 0 the race starts
    /// (the timer runs; the map and music 0; true: Kite let go and the
    /// field camera, the world's); while it has not, a number every 30
    /// frames with its sound (236-239 for 3 down to 0).
    pub fn countdown(&mut self) -> bool {
        let start = self.count == 0;
        if start {
            self.phase += 1;
            self.running = true;
            self.events.push(RaceEvent::Map(1));
            self.events.push(RaceEvent::Bgm(0));
        }
        if !self.running {
            self.tick += 1;
            if self.tick >= 30 {
                self.count -= 1;
                self.tick = 0;
                let se = match self.count {
                    3 => Some(236),
                    2 => Some(237),
                    1 => Some(238),
                    0 => Some(239),
                    _ => None,
                };
                if let Some(n) = se {
                    self.events.push(RaceEvent::Se(n));
                }
            }
        }
        start
    }

    /// 0x005fee10: the timer (while it runs: the digits, a flag newly
    /// taken's split, the three taken or the limit stop it), then the
    /// time, each flag taken and its split drawn.
    pub fn hud(&mut self, cells: &[[i32; 2]]) {
        if self.running {
            self.digits = split(self.time);
            let n = self.taken;
            if n < 3 && self.order[n as usize] != -1 {
                self.splits[n as usize] = self.time;
                self.taken += 1;
            }
            if self.taken == 3 {
                self.running = false;
                self.state = 1;
            }
            if self.running {
                self.time = self.time.wrapping_add(1);
            }
            if self.time > TIME_LIMIT {
                self.running = false;
                self.state = 1;
                self.time = TIME_LIMIT;
            }
        }
        let [_, m, s, hh] = self.digits.map(i32::from);
        let digits = [m / 10, m % 10, 10, s / 10, s % 10, 10, hh / 10, hh % 10];
        for (k, d) in digits.iter().enumerate() {
            let [x, y] = cells[k];
            self.cells.push(HudCell {
                tex: 0,
                code: *d,
                dx: x as f32,
                dy: (y + 20) as f32,
                sx: 16.0,
                sy: 32.0,
                su: 32,
                sv: 64,
            });
        }
        for i in 0..self.taken.max(0) as usize {
            let f = self.order[i];
            let y = (24 * i) as i32;
            self.cells.push(HudCell {
                tex: 1 + f.max(0) as usize,
                code: 0,
                dx: 0.0,
                dy: (y + 60) as f32,
                sx: 24.0,
                sy: 24.0,
                su: 64,
                sv: 64,
            });
            let [_, m, s, hh] = split(self.splits[i]).map(i32::from);
            // The minutes' second digit as the game takes it: (m - m / 10)
            // % 10, not m % 10.
            let digits = [m / 10, (m - m / 10) % 10, 10, s / 10, s % 10, 10, hh / 10, hh % 10];
            for (k, d) in digits.iter().enumerate() {
                self.cells.push(HudCell {
                    tex: 0,
                    code: *d,
                    dx: (12 * k as i32 + 34) as f32,
                    dy: (y + 64) as f32,
                    sx: 12.0,
                    sy: 24.0,
                    su: 32,
                    sv: 64,
                });
            }
        }
    }
}

/// 0x005ff880(pos, rot, dist, height): a place `dist` along the active
/// camera's turn (its pitch and heading, no roll) from the eye, at `height`
/// unless that is 0, facing back at the camera.
fn before_camera(cam: &crate::camera::Camera, dist: F, height: F) -> (V4, V4) {
    before_camera_at(cam.rot(), cam.active().pos, dist, height)
}

/// 0x005ff880 on `cameraGetRot`'s `rot` and `cameraGetPos`'s `eye`.
pub fn before_camera_at(rot: V4, eye: V4, dist: F, height: F) -> (V4, V4) {
    let mut r = rot;
    r[1] = 0;
    r[3] = ONE;
    // sceVu0RotMatrix: about z, then y, then x.
    let m = crate::evcam::rot_x(&crate::evcam::rot_y(&crate::evcam::rot_z(&crate::evcam::UNIT, r[2]), r[1]), r[0]);
    let v = ee::apply(&m, [0, dist, 0, ONE]);
    let mut p = eye;
    p[0] = ee::add(p[0], v[0]);
    p[1] = ee::add(p[1], v[1]);
    p[2] = if ee::eq(0, height) { ee::add(p[2], v[2]) } else { height };
    p[3] = ONE;
    let z = (ee::rad2deg(r[2]) as u16).wrapping_add(0x8000) as i16;
    (p, [0, r[1], ee::deg2rad(z), ONE])
}

/// The race's run of a frame, on the town's world.
impl crate::World {
    /// 0x005ff820(kind) from menu 88: in a town with no race, the task
    /// started (it runs from the next frame's slot).
    pub fn race_start(&mut self, kind: i32) -> bool {
        if self.race.is_some() || self.volume == Volume::Inf {
            return false;
        }
        let files = match RaceFiles::read(&self.archive) {
            Ok(f) => Rc::new(f),
            Err(e) => {
                tracing::warn!("the flag race's files: {e}");
                return false;
            }
        };
        let server = crate::area::SERVER_OF_TOWN.get(self.town.base.no as usize).copied().unwrap_or(0);
        self.race = Some(Box::new(Race::new(files, kind, server)));
        true
    }

    /// The race, while its task lives (`0x0038bd44`).
    pub fn race(&self) -> Option<&Race> {
        self.race.as_deref()
    }

    pub fn race_mut(&mut self) -> Option<&mut Race> {
        self.race.as_deref_mut()
    }

    /// 0x005ff790: the race runs in this town.
    pub fn racing(&self) -> bool {
        self.race.is_some()
    }

    /// The flags `ccSetChibiGuso` put out.
    pub fn flags(&self) -> &[Flag] {
        &self.flags
    }

    /// What the town's `DrawMap` reads of the race: while it runs
    /// (0x005ff790), each flag's place and its mark's fade (0x005fd100).
    pub fn map_racers(&self) -> Option<[Option<crate::map::town::Racer>; 3]> {
        self.race.as_ref()?;
        let mut out = [None; 3];
        for f in &self.flags {
            if let Some(o) = out.get_mut(f.n) {
                *o = Some(crate::map::town::Racer { pos: f.pos, fade: f.alpha });
            }
        }
        Some(out)
    }

    /// The race's events since the last take.
    pub fn take_race_events(&mut self) -> Vec<RaceEvent> {
        let mut out = std::mem::take(&mut self.race_events);
        if let Some(r) = self.race.as_deref_mut() {
            out.append(&mut r.events);
        }
        out
    }

    /// The race's slot (priority 63): the task's code up to its next
    /// breath.
    pub(crate) fn race_slot(&mut self, host: &mut dyn RaceHost) {
        let Some(mut r) = self.race.take() else { return };
        r.clips.clear();
        r.cells.clear();
        self.race_run(&mut r, host);
        if r.gone() {
            self.race_events.append(&mut r.events);
        } else {
            self.race = Some(r);
        }
    }

    fn race_run(&mut self, r: &mut Race, host: &mut dyn RaceHost) {
        let t = piney_data::tables::race::of(self.volume);
        let s = usize::try_from(r.server).unwrap_or(0);
        loop {
            match r.pc {
                Pc::Start => {
                    // ccPgBgmInit; the fade out.
                    r.events.push(RaceEvent::PgBgmInit);
                    r.fade = host.entry_fade(30, BLACK);
                    r.pc = Pc::SetupOut;
                    return;
                }
                Pc::SetupOut => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    // Kite at the start, paused; the Grunty called.
                    self.player.body.pos = t.kite_pos()[s].map(f32::to_bits);
                    self.player.body.dirc = [0, 0, ee::deg2rad(t.kite_rot()[s]), ONE];
                    self.player.acts.pause = true;
                    self.ride_start_town(r.kind);
                    r.pc = Pc::IntroTop;
                }
                Pc::IntroTop => {
                    match r.intro {
                        0 => {
                            self.camera.change_camera(3);
                            let (pos, view) = if r.kind != 0 {
                                let k = r.kind as usize;
                                (t.intro_pos_kind()[k], t.intro_view_kind()[k])
                            } else {
                                (t.intro_pos_server()[s], t.intro_view_server()[s])
                            };
                            let n = self.camera.cam_id;
                            let c = self.camera.cam_mut(n);
                            c.pos = pos.map(f32::to_bits);
                            c.view = view.map(f32::to_bits);
                            r.intro += 1;
                            host.continue_fade(r.fade, 30, CLEAR);
                            r.events.push(RaceEvent::Bgm(3));
                        }
                        1 => {
                            r.pc = Pc::IntroFade;
                            return;
                        }
                        5 => {
                            if r.cnt == 30 {
                                r.fade = host.entry_fade(30, BLACK);
                            }
                            if r.cnt >= 61 {
                                r.cnt = 0;
                                r.intro += 1;
                            } else {
                                r.cnt += 1;
                            }
                        }
                        _ => {}
                    }
                    r.pc = Pc::IntroEnd;
                    return;
                }
                Pc::IntroFade => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    host.delete_fade(r.fade);
                    r.intro += 1;
                    r.cnt = 0;
                    r.pc = Pc::IntroEnd;
                    return;
                }
                Pc::IntroEnd => {
                    if r.intro != 6 {
                        r.pc = Pc::IntroTop;
                        continue;
                    }
                    r.events.push(RaceEvent::BgmStop);
                    self.camera.change_camera(1);
                    self.camera.soft_reset(self.player.body.dirc[2]);
                    r.pc = Pc::SetupCam;
                    return;
                }
                Pc::SetupCam => {
                    // The field camera's place held by the event camera.
                    let (pos, view) = (self.camera.active().pos, self.camera.active().view);
                    self.camera.change_camera(3);
                    let n = self.camera.cam_id;
                    let c = self.camera.cam_mut(n);
                    c.pos = pos;
                    c.view = view;
                    r.events.push(RaceEvent::Map(3));
                    for f in &mut self.flags {
                        f.entry.fade_flag = 1;
                        f.entry.fade_cnt = 5;
                        f.state = FlagState::Out;
                        f.taken = false;
                    }
                    let p = if r.kind != 0 { t.ride_kind()[r.kind as usize] } else { t.ride_server()[s] };
                    let p = p.map(f32::to_bits);
                    r.ride = RaceRide { max: p[0], accel: p[1], ease_on: p[2], ease_off: p[3] };
                    host.continue_fade(r.fade, 30, CLEAR);
                    r.pc = Pc::SetupIn;
                    return;
                }
                Pc::SetupIn => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    host.delete_fade(r.fade);
                    r.pc = Pc::Main;
                    return;
                }
                Pc::Main => {
                    match r.phase {
                        0 => self.race_countdown(r, s),
                        1 => {
                            let old = r.count;
                            r.count += 1;
                            if old < 30 {
                                self.race_clip(r, Clip::Count, 0xc448_0000, t.count_height()[s].to_bits());
                            }
                            if r.state == 1 {
                                r.phase += 1;
                            }
                        }
                        2 => {
                            // The finish with the three flags, else the
                            // quit; true: breathing in the fade.
                            let breathes =
                                if r.taken == 3 { self.race_finish(r, host, s) } else { self.race_quit(r, host, s) };
                            if breathes {
                                return;
                            }
                        }
                        _ => {}
                    }
                    self.race_after(r, host);
                    return;
                }
                Pc::FinishOut | Pc::QuitOut => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    let finish = r.pc == Pc::FinishOut;
                    let (pos, rot) = (t.ride_end_pos()[s].map(f32::to_bits), t.ride_end_rot()[s].map(f32::to_bits));
                    self.ride_place(pos, rot);
                    if finish {
                        self.player.body.pos = pos;
                        self.player.body.dirc = rot;
                    }
                    self.camera.change_camera(3);
                    let n = self.camera.cam_id;
                    let c = self.camera.cam_mut(n);
                    c.pos = t.finish_cam_pos()[s].map(f32::to_bits);
                    c.view = t.finish_cam_view()[s].map(f32::to_bits);
                    r.ride.ease_off = ONE;
                    if finish {
                        self.race_rank(r);
                        r.events.push(RaceEvent::BgmStop);
                        r.events.push(RaceEvent::Map(3));
                    } else {
                        r.events.push(RaceEvent::Map(3));
                        r.events.push(RaceEvent::BgmStop);
                    }
                    host.continue_fade(r.fade, 30, CLEAR);
                    r.pc = if finish { Pc::FinishIn } else { Pc::QuitIn };
                    return;
                }
                Pc::FinishIn => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    host.delete_fade(r.fade);
                    match r.rank {
                        0 => {
                            r.sub = 3;
                            r.events.push(RaceEvent::Bgm(6));
                        }
                        4 => {
                            r.sub = 4;
                            r.events.push(RaceEvent::Bgm(6));
                        }
                        // 1-3: music 4 (the 4 the test left in a0).
                        _ => {
                            r.sub = 2;
                            r.events.push(RaceEvent::Bgm(4));
                        }
                    }
                    self.race_after(r, host);
                    return;
                }
                Pc::QuitIn => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    host.delete_fade(r.fade);
                    if r.time < TIME_LIMIT {
                        r.sub = 3;
                        r.state = 2;
                    } else {
                        r.over_play = r.files.anm(Clip::Over, "ANM_xp_over_a");
                        r.sub += 1;
                    }
                    self.race_after(r, host);
                    return;
                }
                Pc::EndOut => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    r.events.push(RaceEvent::BgmStop);
                    r.end = true;
                    r.intro = 7;
                    self.camera.change_camera(1);
                    r.events.push(RaceEvent::Panel(1));
                    r.events.push(RaceEvent::Map(1));
                    r.events.push(RaceEvent::PgBgmEnd);
                    for f in &mut self.flags {
                        f.entry.fade_flag = 2;
                        f.entry.fade_cnt = 15;
                        f.taken = false;
                        f.cnt = 0;
                        f.state = FlagState::Taken;
                    }
                    // Kite by the breeder, facing him.
                    let at = t.end_pos()[s].map(f32::to_bits);
                    self.player.body.pos = at;
                    let breeder = crate::grunty::dummy(&self.town.base.file, "DMY_merchant6").map(|d| d.0);
                    let mut rot = self.player.body.dirc;
                    rot[2] = breeder.map_or(rot[2], |b| crate::rtownpc::get_dirc(at, b));
                    self.player.body.dirc = rot;
                    r.pc = Pc::EndCam;
                    return;
                }
                Pc::EndCam => {
                    self.camera.soft_reset(self.player.body.dirc[2]);
                    host.continue_fade(r.fade, 30, CLEAR);
                    r.pc = Pc::EndIn;
                    return;
                }
                Pc::EndIn => {
                    if host.check_fade(r.fade) {
                        return;
                    }
                    host.delete_fade(r.fade);
                    r.pc = Pc::WaitRide;
                    return;
                }
                Pc::WaitRide => {
                    if self.ride.is_some() {
                        return;
                    }
                    r.events.push(RaceEvent::Forbid(0));
                    r.pc = Pc::Gone;
                    return;
                }
                Pc::Gone => return,
            }
        }
    }

    /// The main loop's tail: the HUD, the (dead) restart, and the menu's
    /// done flag: the end's fade out begins, else the breath.
    fn race_after(&mut self, r: &mut Race, host: &mut dyn RaceHost) {
        self.race_hud(r);
        if r.done {
            r.fade = host.entry_fade(30, BLACK);
            r.pc = Pc::EndOut;
        } else {
            r.pc = Pc::Main;
        }
    }

    /// 0x005fdd10, phase 0: when the number is 0 the race starts (the
    /// timer, Kite free, the field camera, the map, music 0); while it has
    /// not, a number every 30 frames with its sound. The clip drawn each
    /// call.
    fn race_countdown(&mut self, r: &mut Race, s: usize) {
        if r.countdown() {
            self.player.acts.pause = false;
            self.camera.change_camera(1);
        }
        let h = piney_data::tables::race::of(self.volume).count_height()[s].to_bits();
        self.race_clip(r, Clip::Count, 0xc448_0000, h);
    }

    /// A clip stepped and drawn in front of the camera.
    fn race_clip(&mut self, r: &mut Race, clip: Clip, dist: F, height: F) {
        let (pos, rot) = before_camera(&self.camera, dist, height);
        let p = match clip {
            Clip::Count => &mut r.count_play,
            Clip::Cup => &mut r.cup_play,
            Clip::Place => &mut r.place_play,
            Clip::Chnut => &mut r.chnut_play,
            Clip::Out => &mut r.out_play,
            Clip::Over => &mut r.over_play,
        };
        let Some(p) = p.as_mut() else { return };
        p.forward();
        r.clips.push(ClipDraw { anm: p.clone(), pos, rot });
    }

    /// 0x005fe830: the time into the save's ranks; the result's clips by
    /// the rank.
    fn race_rank(&mut self, r: &mut Race) {
        let t = piney_data::tables::race::of(self.volume);
        let ranks: [(i16, i16); 3] = piney_data::tables::fieldui::of(self.volume)
            .race_ranks()
            .get((r.server - 1) as usize)
            .map_or([(0, 0); 3], |rs| rs.map(|x| (x.time, x.row)));
        let row = (r.kind + ROW_KIND0) as i16;
        r.rank = enter_time(&mut self.save.save, &ranks, r.server, r.time, row);
        match r.rank {
            1..=3 => {
                let k = r.rank as usize;
                r.cup_play = r.files.anm(Clip::Cup, t.cup_anms()[k]);
                r.place_play = r.files.anm(Clip::Place, t.place_anms()[k]);
            }
            _ => r.out_play = r.files.anm(Clip::Out, "ANM_xp_out_a"),
        }
    }

    /// 0x005fe290, phase 2 with the three flags: Kite held 32 frames, the
    /// fade's step (true: the task breathes inside it), then the result's
    /// clips until the music after.
    fn race_finish(&mut self, r: &mut Race, host: &mut dyn RaceHost, s: usize) -> bool {
        let h = piney_data::tables::race::of(self.volume).cup_height()[s].to_bits();
        match r.sub {
            0 => {
                self.player.acts.pause = true;
                let old = r.cnt;
                r.cnt += 1;
                if old >= 31 {
                    r.sub += 1;
                    r.cnt = 0;
                }
            }
            1 => {
                r.fade = host.entry_fade(30, BLACK);
                r.pc = Pc::FinishOut;
                return true;
            }
            2 => {
                self.race_clip(r, Clip::Cup, 0xc3fa_0000, h);
                self.race_clip(r, Clip::Place, 0xc3fa_0000, h);
                self.race_clip(r, Clip::Chnut, 0xc3fa_0000, h);
                let old = r.cnt;
                r.cnt += 1;
                if old >= 191 {
                    r.sub = 6;
                    r.cnt = 0;
                    r.state = 2;
                    r.events.push(RaceEvent::BgmStop);
                }
            }
            3 | 4 => {
                self.race_clip(r, Clip::Out, 0xc3fa_0000, h);
                let old = r.cnt;
                r.cnt += 1;
                if old >= 181 {
                    r.phase += 1;
                    r.sub = 5;
                    r.state = 2;
                }
            }
            6 => {
                let old = r.cnt;
                r.cnt += 1;
                if old >= 11 {
                    r.events.push(RaceEvent::Bgm(5));
                    r.phase = 5;
                }
            }
            _ => {}
        }
        false
    }

    /// 0x005feab0, phase 2 short of the flags (quit, or the time over):
    /// Kite held 32 frames, the fade's step, then a time over's clip.
    fn race_quit(&mut self, r: &mut Race, host: &mut dyn RaceHost, s: usize) -> bool {
        match r.sub {
            0 => {
                self.player.acts.pause = true;
                let old = r.cnt;
                r.cnt += 1;
                if old >= 31 {
                    r.sub += 1;
                    r.cnt = 0;
                }
            }
            1 => {
                r.fade = host.entry_fade(30, BLACK);
                r.pc = Pc::QuitOut;
                return true;
            }
            2 => {
                let h = piney_data::tables::race::of(self.volume).cup_height()[s].to_bits();
                self.race_clip(r, Clip::Over, 0xc3fa_0000, h);
                let old = r.cnt;
                r.cnt += 1;
                if old >= 181 {
                    r.phase += 1;
                    r.sub = 3;
                    r.state = 2;
                }
            }
            _ => {}
        }
        false
    }

    /// 0x005fee10 ([`Race::hud`]) with the volume's timer cells.
    fn race_hud(&mut self, r: &mut Race) {
        r.hud(piney_data::tables::race::of(self.volume).timer_cells());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_a_time() {
        assert_eq!(split(0), [0, 0, 0, 0]);
        // 1:02.50 = 1875 frames: the hundredths (15 * 100 / 30).
        assert_eq!(split(1875), [0, 1, 2, 50]);
        assert_eq!(split(TIME_LIMIT), [0, 10, 0, 0]);
        assert_eq!(split(29), [0, 0, 0, 96]);
    }

    #[test]
    fn a_time_takes_its_rank() {
        let mut save = SaveData::new();
        let ranks = [(2000, 150), (2500, 151), (3000, 152)];
        // Under the town's first racer: rank 1, the save's row filled.
        assert_eq!(enter_time(&mut save, &ranks, 1, 1900, 146), 1);
        assert_eq!((save.i16(RACE_RECORDS), save.i16(RACE_RECORDS + 2)), (1900, 146));
        // The racers after keep their places by the save's zeros: 2100
        // beats the first racer (2000 is the next unheld), not 1900.
        assert_eq!(enter_time(&mut save, &ranks, 1, 2100, 147), 3);
        assert_eq!(save.i16(RACE_RECORDS + 8), 2100);
        // Not within the three, near the third, and far behind.
        assert_eq!(enter_time(&mut save, &ranks, 1, 2110, 147), 4);
        assert_eq!(enter_time(&mut save, &ranks, 1, 9000, 147), 0);
    }
}
