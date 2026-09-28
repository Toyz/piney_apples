//! Stream 15's effect tasks, the ending (event 31's `stream 15`):
//! `Func_str0580` (0x0018e340) over `str0580` and `Func_str0581`
//! (0x001904a0) over `str0581`, the scene that follows it
//! (`docs/engine/stream.md`, "Stream 15's effect tasks").
//!
//! Beside the `ccStrEffectCtrl` every task keeps ([`Ctrl`]), both hold an
//! event table walker (`ccEventObj`) and a part group (`ccStrPartGrp`): a
//! cue in the table names an object of the scene and what to start at it -
//! a burst of `EFF_x001` puffs of `str0580e` (`ccEffPart0580`), a creator
//! that throws rocks (`ccObjPartCreate0580`, each rock a `ccObjPart0580`
//! with one of the `OBJ_se1_6ro*` models), or both.
//!
//! ```text
//! Func_str0580   str0580e's EFF_x001 and its four OBJ_se1_6ro* (the game
//!                crashes unless there are four); the scene view's divZ
//!                2000; eventObjTbl_0580; its Ctrl (the noise bands' Init
//!                draws 216 numbers). Each pass while param[1] is 0 and the
//!                scene is up:
//!   frames       1500, 1575, .., 1741: the feedback on (1.0, alpha 0x4c,
//!                0.01 of the view down); 1565, 1592, .., 1744: fading
//!   cues         500-610: SetObj(cue) and its part(s); 999: a fade to
//!                white over 30; else by the last digit: 1 noise on, 2 off,
//!                3 inversion on, 4 off, 5 as the frames' feedback, 6 off,
//!                7 / 8 a one-frame flash to / from white
//!   parts        on sysLayer: the creators (newest first; one gone when
//!                its life is up), then the parts (newest first; a puff
//!                gone below transparency 0.01)
//!   Ctrl         feedback, fade, noise, inversion, shades
//!   then         eventExTscb2 = the task; the feedback off; one pass;
//!                the parts deleted; passes while param[1] is not 2 (the
//!                next scene's ccSetStreamDemoThread sets it); three more
//! Func_str0581   str0580e's EFF_x001; divZ 3000; the fog SetFog(30000,
//!                50000, 0) (again every pass) but on EXT_se1_6bac1,
//!                EXT_se1_6clo1_1, EXT_se1_6clo1_2; eventObjTbl_0581; its
//!                Ctrl. Each pass while param[1] is 0 and the scene is up:
//!   frames       1050: the feedback on (1.005, alpha 0x58); 1155: off
//!   cues         600-610: SetObj(cue), a puff burst (type 0 only); 65:
//!                feedback 1.005 alpha 0x40 with a 15-frame fade ready; 56:
//!                shades and feedback off; 55: two shades (2- and 4-texel
//!                blocks of the picture being drawn, alpha 0x40 and 0x30,
//!                offsets -0.0041 and -0.0082, at depth 1000) and the
//!                feedback (1.01, alpha 0x50, at depth 1000); 899: to black
//!                over 30; 901-904: from white over 5; 900: from white over
//!                45; 699: a transfer (210 high) at the note's object; else
//!                by the last digit as Func_str0580's but 5 (1.02, alpha
//!                0x40) and 6 (fading)
//!   then         the feedback off; no pass; gone
//! ```
//!
//! `ccEventObj::SetObj(cue)` (0x00184150) walks the table from the entry
//! it last found, one way or the other, with its tests the wrong way
//! round: it finds the cue only when it is the next entry (or the one
//! before), and a cue it misses leaves it where it was. Stream 15 queues
//! 539, 540 and 541 in one frame (frame 1456), taken last first: 541 and
//! 540 are missed, 539 found, and every cue after it misses too - the
//! parts stop there, 138 rocks falling from then on.
//!
//! A rock's model has no Bbox, so `ccObj::CheckBoundingBox` always says it
//! is in view and a rock is never deleted: each falls (6 a frame faster)
//! until the scene ends.
//!
//! The parts' draws come out as [`PartDraw`]s; the stream draws them (the
//! puffs through `piney_effect::eff::Eff`, the rocks as the scene's models).

use piney_world::ee::{self, F, ONE, V4};

use crate::effect::{Ctrl, Cue, Draws, EFFECT_LAYER, FADE_WHITE, MAX_CUES, Rand, Transfer};

/// `StreamDemoFuncTbl`'s entries.
pub const STR0580: &str = "str0580";
pub const STR0581: &str = "str0581";

/// The effect file both tasks read (`str0580e`, or `str0580ep` when the
/// stream has no `str0580e`), its puff and its rocks (`GetChunkAdrs("OBJ_se1_6ro*")`, in
/// index order).
pub const EFF_FILE: &str = "str0580e";
pub const EFF_NAME: &str = "EFF_x001";
pub const ROCKS: [&str; 4] = ["OBJ_se1_6roa", "OBJ_se1_6rob", "OBJ_se1_6roc", "OBJ_se1_6rod"];

/// `charHeightTbl[17]` (0x0034e194, 210.0): cue 699's transfer.
pub const TRANSFER_HEIGHT_0581: u32 = 0x4352_0000;
/// `fogOffObj` (0x0030bf60): `Func_str0581`'s objects out of its fog.
pub const FOG_OFF_0581: [&str; 3] = ["EXT_se1_6bac1", "EXT_se1_6clo1_1", "EXT_se1_6clo1_2"];
/// `SetFog(30000, 50000, 0)` (`str0581Fog`), black.
pub const FOG_0581: (f32, f32, u32) = (30000.0, 50000.0, 0);
/// The scene view's `divZ` (+0x25c) the tasks set: 2000 and 3000.
pub const DIV_Z_0580: u32 = 0x44fa_0000;
pub const DIV_Z_0581: u32 = 0x453b_8000;

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_TURN: F = 0x4334_0000;
const FULL_TURN: F = 0x43b4_0000;
const MINUS_HALF_TURN: F = 0xc334_0000;
/// 2^31: `rand()` over it, times a range.
const RAND_RANGE: F = 0x4f00_0000;
/// A puff below this transparency is gone.
const TP_MIN: F = 0x3c23_d70a;

/// `ccEffPartParam0580` (0x60 bytes): a puff burst.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffPartParam {
    pub vel: V4,
    pub acc: V4,
    pub scale: [F; 2],
    pub dscale: [F; 2],
    pub tp: F,
    pub dtp: F,
    /// The burst's spread: an angle range (degrees) and a radius about the
    /// object, then a height and a random height.
    pub angle: F,
    pub radius: F,
    pub z: F,
    pub z_rand: F,
    /// Puffs: `count` plus `rand() % count_rand` (when not 0).
    pub count: i32,
    pub count_rand: i32,
    pub colour: u32,
}

/// `ccObjPartCreate0580`'s parameters (0x34 bytes, `p0580_3`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RockParam {
    /// Speed across (turned), its acceleration across, the speed up and a
    /// random part, the acceleration up.
    pub vx: F,
    pub ax: F,
    pub vz: F,
    pub vz_rand: F,
    pub az: F,
    /// How far out from the object a rock starts (random).
    pub radius: F,
    /// The turns about y and z each frame (degrees, plus a random part).
    pub dry: F,
    pub dry_rand: F,
    pub drz: F,
    pub drz_rand: F,
    /// Rocks a frame (1/4096) and a random part, and the frames it makes
    /// them for.
    pub rate: i32,
    pub rate_rand: i32,
    pub life: i32,
}

/// What an event entry starts (its type).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// 0: a puff burst.
    Eff(EffPartParam),
    /// 1: a rock creator.
    Rock(RockParam),
    /// 2: both (`p0580_0`: the burst's and the creator's parameters).
    Both(EffPartParam, RockParam),
    Other,
}

/// One entry of the run of event tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEntry {
    pub cue: u32,
    pub name: Option<String>,
    pub kind: EventKind,
}

/// What the tasks read (`tables::stream`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tables {
    /// `eventObjTbl_0580` (INF main 0x0034eb60), read on through
    /// `eventObjTbl_0581` (0x0034f1f0, entry 105) to its end, 16 bytes an
    /// entry: cue, name, type, parameters. The walk only ever reads in this
    /// run; a stray string between the two tables reads as nothing.
    pub entries: Vec<EventEntry>,
    /// `rockScaleTbl` (0x0034e490): a rock's scale, by its model.
    pub rock_scale: [F; 3],
}

fn eff_param(p: &piney_data::tables::types::EffPartParam0580) -> EffPartParam {
    let b = f32::to_bits;
    EffPartParam {
        vel: p.spd.map(b),
        acc: p.acl.map(b),
        scale: [b(p.xscale), b(p.yscale)],
        dscale: [b(p.xscale_spd), b(p.yscale_spd)],
        tp: b(p.tr),
        dtp: b(p.tr_spd),
        angle: b(p.offset_pos_a),
        radius: b(p.offset_pos_r),
        z: b(p.offset_pos_z),
        z_rand: b(p.offset_pos_z2),
        count: p.birth_num,
        count_rand: p.birth_num2,
        colour: p.color,
    }
}

fn rock_param(p: &piney_data::tables::types::ObjPartParam0580) -> RockParam {
    let b = f32::to_bits;
    RockParam {
        vx: b(p.xyspd),
        ax: b(p.xyacl),
        vz: b(p.zspd),
        vz_rand: b(p.zspd2),
        az: b(p.zacl),
        radius: b(p.offset_pos_r),
        dry: b(p.yrot),
        dry_rand: b(p.yrot2),
        drz: b(p.zrot),
        drz_rand: b(p.zrot2),
        rate: p.create.birth_num1,
        rate_rand: p.create.birth_num2,
        life: p.create.time_end,
    }
}

impl Tables {
    /// The volume's.
    pub fn read(volume: piney_data::volume::Volume) -> Tables {
        let t = piney_data::tables::stream::of(volume);
        let entries = t
            .event_objs()
            .iter()
            .map(|o| EventEntry {
                cue: o.cue,
                name: o.name.map(str::to_string),
                kind: match (&o.eff, &o.rock) {
                    (Some(e), Some(r)) => EventKind::Both(eff_param(e), rock_param(r)),
                    (Some(e), None) => EventKind::Eff(eff_param(e)),
                    (None, Some(r)) => EventKind::Rock(rock_param(r)),
                    (None, None) => EventKind::Other,
                },
            })
            .collect();
        Tables { entries, rock_scale: t.rock_scale().map(f32::to_bits) }
    }

    /// Where `eventObjTbl_0581` starts in the run (entry 105 on Infection):
    /// its six entries end it.
    pub fn first_0581(&self) -> usize {
        self.entries.len().saturating_sub(EVENT_OBJ_0581_ENTRIES)
    }
}

/// `eventObjTbl_0581`'s entries.
const EVENT_OBJ_0581_ENTRIES: usize = 6;

/// The scene as the tasks see it: `GetSubstAdrsF(name)` and an object's
/// `lwMatrix` translation now.
pub trait World {
    fn find(&self, name: &str) -> Option<u32>;
    fn position(&self, obj: u32) -> V4;
}

/// A world with nothing in it.
pub struct NoWorld;

impl World for NoWorld {
    fn find(&self, _name: &str) -> Option<u32> {
        None
    }
    fn position(&self, _obj: u32) -> V4 {
        [0, 0, 0, ONE]
    }
}

/// `ccEventObj` (12 bytes): the entry it last found and its object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventObj {
    pub at: usize,
    pub obj: Option<u32>,
}

impl EventObj {
    /// The constructor's: the table's first entry and its object.
    fn new(tables: &Tables, at: usize, world: &dyn World) -> EventObj {
        let obj = tables.entries.get(at).and_then(|e| e.name.as_deref()).and_then(|n| world.find(n));
        EventObj { at, obj }
    }

    /// `SetObj(cue)` (0x00184150): the entry found (and its object, which
    /// the result is), or None and nothing changed.
    fn set_obj(&mut self, tables: &Tables, cue: u32, world: &dyn World) -> Option<u32> {
        let e = &tables.entries;
        let cur = e.get(self.at)?.cue;
        let mut i = self.at;
        let found = if cur < cue {
            // Forward, on while the cue is below the entry.
            loop {
                i += 1;
                let c = e.get(i)?.cue;
                if c == cue {
                    break i;
                }
                if cue >= c {
                    return None;
                }
            }
        } else if cue < cur {
            // Back, on while the entry is below the cue.
            loop {
                i = i.checked_sub(1)?;
                let c = e[i].cue;
                if c == cue {
                    break i;
                }
                if c >= cue {
                    return None;
                }
            }
        } else {
            return None;
        };
        if let Some(n) = e[found].name.as_deref() {
            self.obj = world.find(n);
        }
        self.at = found;
        self.obj
    }
}

/// A float of `rand()` (`cvt.s.w`).
fn randf(rand: &mut Rand) -> F {
    ee::from_int(rand.rand())
}

/// `range * rand() / 2^31`.
fn rand_in(range: F, rand: &mut Rand) -> F {
    ee::div(ee::mul(range, randf(rand)), RAND_RANGE)
}

/// `ccRotate(a, d)` (0x00102190): `a + d`, brought back by a turn once past
/// pi the way it went.
fn rotate(a: F, d: F) -> F {
    let a = ee::add(a, d);
    if ee::le(d, 0) {
        if ee::lt(a, ee::neg(PI)) { ee::add(a, TWO_PI) } else { a }
    } else if !ee::le(a, PI) {
        ee::sub(a, TWO_PI)
    } else {
        a
    }
}

/// `ccEffPart0580` (0xd0): one puff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Puff {
    pub pos: V4,
    pub vel: V4,
    pub acc: V4,
    pub scale: [F; 2],
    pub dscale: [F; 2],
    pub tp: F,
    pub dtp: F,
    pub pattern: u16,
    pub colour: u32,
}

/// `ccObjPart0580` (0x120): one rock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rock {
    /// Which of [`ROCKS`] (0-2).
    pub model: usize,
    pub pos: V4,
    pub vel: V4,
    pub acc: V4,
    pub rot: V4,
    pub drot: V4,
    pub scale: [F; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Puff(Puff),
    Rock(Rock),
}

/// `ccObjPartCreate0580` (0x44): throws rocks from its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Creator {
    obj: Option<u32>,
    param: RockParam,
    count: i32,
    acc: [i32; 2],
}

/// A part's draw of the pass, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartDraw {
    /// `ccEff::Draw(pos, pattern)` of `EFF_x001` at this scale,
    /// transparency and colour.
    Puff { pos: V4, pattern: u16, scale: [F; 2], tp: F, colour: u32 },
    /// `ccObj::Draw(1.0)` of rock `model` ([`ROCKS`]) at this matrix
    /// (stored columns).
    Rock { model: usize, matrix: [V4; 4] },
}

/// `ccStrPartGrp`: the creators and the parts, each in the order made.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Parts {
    creators: Vec<Creator>,
    parts: Vec<Part>,
}

impl Parts {
    /// `ccEffPart0580::Create(grp, chunk, coord, param)` (0x0018dfc0).
    fn puffs(&mut self, at: V4, p: &EffPartParam, rand: &mut Rand) {
        let extra = if p.count_rand != 0 { rand.rand() % p.count_rand } else { 0 };
        for _ in 0..p.count + extra {
            let a = ee::sub(rand_in(p.angle, rand), HALF_TURN);
            let m = piney_data::anim::rot_z_bits_of(unit(), ee::div(ee::mul(PI, a), HALF_TURN));
            let mut pos = ee::apply(&m, [rand_in(p.radius, rand), 0, 0, 0]);
            pos = ee::vadd(pos, at);
            pos[2] = ee::add(pos[2], ee::add(p.z, rand_in(p.z_rand, rand)));
            let pattern = (rand.rand() & 3) as u16;
            self.parts.push(Part::Puff(Puff {
                pos,
                vel: p.vel,
                acc: p.acc,
                scale: p.scale,
                dscale: p.dscale,
                tp: p.tp,
                dtp: p.dtp,
                pattern,
                colour: p.colour,
            }));
        }
    }

    /// A creator on its object (`new ccObjPartCreate0580`, from the task).
    fn creator(&mut self, obj: Option<u32>, param: RockParam) {
        self.creators.push(Creator { obj, param, count: 0, acc: [0; 2] });
    }

    /// `ccObjPartCreate0580::Ctrl` (0x0018d8a0): true when its life is up.
    fn create(&mut self, k: usize, world: &dyn World, tables: &Tables, rand: &mut Rand) -> bool {
        let c = &mut self.creators[k];
        let (obj, p) = (c.obj, c.param);
        if let Some(obj) = obj {
            c.acc[0] = (c.acc[0] & 0xfff) + p.rate;
            c.acc[1] = (c.acc[1] & 0xfff) + p.rate_rand;
            let extra = c.acc[1] >> 12;
            let n = (c.acc[0] >> 12) + if extra != 0 { rand.rand() % extra } else { 0 };
            for _ in 0..n {
                let at = world.position(obj);
                let rock = new_rock(at, &p, tables, rand);
                self.parts.push(Part::Rock(rock));
            }
        }
        let c = &mut self.creators[k];
        c.count += 1;
        c.count >= p.life
    }

    /// The creators' then the parts' `Ctrl`, newest first, each dropped
    /// when done; the parts' draws.
    fn step(&mut self, world: &dyn World, tables: &Tables, rand: &mut Rand, out: &mut Vec<PartDraw>) {
        let mut gone = Vec::new();
        for k in (0..self.creators.len()).rev() {
            if self.create(k, world, tables, rand) {
                gone.push(k);
            }
        }
        for k in gone {
            self.creators.remove(k);
        }
        let mut keep = vec![true; self.parts.len()];
        for k in (0..self.parts.len()).rev() {
            keep[k] = match &mut self.parts[k] {
                Part::Puff(p) => puff_step(p, out),
                Part::Rock(r) => {
                    rock_step(r, out);
                    true
                }
            };
        }
        let mut it = keep.into_iter();
        self.parts.retain(|_| it.next().unwrap_or(true));
    }

    fn clear(&mut self) {
        self.creators.clear();
        self.parts.clear();
    }
}

fn unit() -> [V4; 4] {
    [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]
}

/// One rock as `ccObjPartCreate0580::Ctrl` makes it: a turn about z, the
/// start out along it, the speeds across (turned) and up, the turns, the
/// model and its scale.
fn new_rock(at: V4, p: &RockParam, tables: &Tables, rand: &mut Rand) -> Rock {
    let a = ee::sub(rand_in(FULL_TURN, rand), HALF_TURN);
    let m = piney_data::anim::rot_z_bits_of(unit(), ee::div(ee::mul(PI, a), HALF_TURN));
    let mut pos = [rand_in(p.radius, rand), 0, 0, 0];
    let mut vel = [rand_in(p.vx, rand), 0, 0, 0];
    let mut acc = [rand_in(p.ax, rand), 0, 0, 0];
    pos = ee::apply(&m, pos);
    vel = ee::apply(&m, vel);
    acc = ee::apply(&m, acc);
    pos = ee::vadd(pos, at);
    vel[2] = ee::add(p.vz, rand_in(p.vz_rand, rand));
    acc[2] = p.az;
    let turn = |rand: &mut Rand| ee::div(ee::mul(PI, ee::add(MINUS_HALF_TURN, rand_in(FULL_TURN, rand))), HALF_TURN);
    let rx = turn(rand);
    let ry = turn(rand);
    let deg = |base: F, range: F, rand: &mut Rand| ee::div(ee::mul(PI, ee::add(base, rand_in(range, rand))), HALF_TURN);
    let dry = deg(p.dry, p.dry_rand, rand);
    let drz = deg(p.drz, p.drz_rand, rand);
    let model = (rand.rand() % 3) as usize;
    let s = tables.rock_scale[model];
    Rock { model, pos, vel, acc, rot: [rx, ry, 0, 0], drot: [0, dry, drz, 0], scale: [s, s, s] }
}

/// `ccEffPart0580::Ctrl` (0x0018dee0): moved, grown and faded; drawn, or
/// false (gone) once too faint.
fn puff_step(p: &mut Puff, out: &mut Vec<PartDraw>) -> bool {
    p.pos = ee::vadd(p.pos, p.vel);
    p.vel = ee::vadd(p.vel, p.acc);
    p.scale[0] = ee::add(p.scale[0], p.dscale[0]);
    p.scale[1] = ee::add(p.scale[1], p.dscale[1]);
    p.tp = ee::add(p.tp, p.dtp);
    if ee::lt(p.tp, TP_MIN) {
        return false;
    }
    out.push(PartDraw::Puff { pos: p.pos, pattern: p.pattern, scale: p.scale, tp: p.tp, colour: p.colour });
    true
}

/// `ccObjPart0580::Ctrl` (0x0018d7e0): moved and turned, placed
/// (`SetMatrix_PosRotXYZScale`) and drawn; its model has no Bbox, so it is
/// always in view and never gone.
fn rock_step(r: &mut Rock, out: &mut Vec<PartDraw>) {
    r.pos = ee::vadd(r.pos, r.vel);
    r.vel = ee::vadd(r.vel, r.acc);
    r.rot[1] = rotate(r.rot[1], r.drot[1]);
    r.rot[2] = rotate(r.rot[2], r.drot[2]);
    let matrix = piney_world::field_ambient::pos_rot_xyz_scale(r.pos, r.rot, r.scale);
    out.push(PartDraw::Rock { model: r.model, matrix });
}

/// Where a task is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Its loop, while `param[1]` is 0 and the scene is up.
    Main,
    /// `Func_str0580` after its loop, waiting for `param[1]` 2.
    Wait,
    /// Its last passes.
    Tail(u8),
    Done,
}

/// `Func_str0580`'s state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0580 {
    pub ctrl: Ctrl,
    pub event: EventObj,
    tables: Tables,
    parts: Parts,
    draws: Vec<PartDraw>,
    phase: Phase,
    /// `param[1]` of its thread: 1 when the next scene's
    /// `ccSetStreamDemoThread` ends it, 2 when it is `eventExTscb2`.
    param: u8,
    /// The scene's state bit 0x40 (cleared by `DelSceneObject`).
    alive: bool,
}

/// `Func_str0580`'s feedback (the frames' and cue x5): `SetReflex(1.0, 0,
/// 0x4c808080)`, then its offset's y 0.16 (sixteenths of the view: 0.01 of
/// the view down).
fn reflex_0580(c: &mut Ctrl) {
    c.feedback.reflex(ONE, 0x4c80_8080);
    c.feedback.offset_y = 0x3e23_d70a;
}

const FRAMES_ON: [u32; 10] = [1741, 1730, 1718, 1706, 1694, 1682, 1664, 1630, 1575, 1500];
const FRAMES_FADE: [u32; 10] = [1744, 1733, 1721, 1709, 1697, 1685, 1670, 1640, 1592, 1565];

impl Str0580 {
    /// `Func_str0580` started: its objects (the noise bands' `Init` drawing
    /// from `rand`), the table walker on its first entry.
    pub fn new(tables: &Tables, world: &dyn World, rand: &mut Rand) -> Str0580 {
        let event = EventObj::new(tables, 0, world);
        Str0580 {
            ctrl: Ctrl::new(Some(EFFECT_LAYER), rand),
            event,
            tables: tables.clone(),
            parts: Parts::default(),
            draws: Vec::new(),
            phase: Phase::Main,
            param: 0,
            alive: true,
        }
    }

    /// `param[1]` set (1 ends the loop; 2 lets the task finish too).
    pub fn set_param(&mut self, p: u8) {
        self.param = p;
    }

    /// The scene's `DelSceneObject`.
    pub fn scene_gone(&mut self) {
        self.alive = false;
    }

    /// Whether it has ended (`ccDeleteThread`).
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Whether it is still in its loop.
    pub fn in_loop(&self) -> bool {
        self.phase == Phase::Main
    }

    /// The parts' draws of the last pass.
    pub fn part_draws(&self) -> &[PartDraw] {
        &self.draws
    }

    /// One activation: a pass of its loop, or of what follows it.
    pub fn step(&mut self, world: &dyn World, cues: &[Cue], frame_now: u32, paused: bool, rand: &mut Rand) -> Draws {
        self.draws.clear();
        match self.phase {
            Phase::Main if self.param == 0 && self.alive => {
                let c = &mut self.ctrl;
                if FRAMES_FADE.contains(&frame_now) {
                    c.feedback.flag = 2;
                } else if FRAMES_ON.contains(&frame_now) {
                    reflex_0580(c);
                }
                for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
                    self.cue(cue, world, rand);
                }
                self.parts.step(world, &self.tables, rand, &mut self.draws);
                self.ctrl.pass(paused, rand)
            }
            Phase::Main => {
                self.ctrl.feedback.off();
                let d = self.ctrl.pass(paused, rand);
                self.parts.clear();
                self.phase = Phase::Wait;
                d
            }
            Phase::Wait => {
                let d = self.ctrl.pass(paused, rand);
                if self.param == 2 {
                    self.phase = Phase::Tail(2);
                }
                d
            }
            Phase::Tail(n) => {
                let d = self.ctrl.pass(paused, rand);
                self.phase = if n > 1 { Phase::Tail(n - 1) } else { Phase::Done };
                d
            }
            Phase::Done => Draws::new(),
        }
    }

    fn cue(&mut self, cue: u32, world: &dyn World, rand: &mut Rand) {
        let c = &mut self.ctrl;
        match cue {
            500..=610 => {
                let Some(obj) = self.event.set_obj(&self.tables, cue, world) else { return };
                let at = world.position(obj);
                match self.tables.entries[self.event.at].kind {
                    EventKind::Both(e, r) => {
                        self.parts.puffs(at, &e, rand);
                        self.parts.creator(Some(obj), r);
                    }
                    EventKind::Rock(r) => self.parts.creator(Some(obj), r),
                    EventKind::Eff(e) => self.parts.puffs(at, &e, rand),
                    EventKind::Other => {}
                }
            }
            999 => c.fade.entry(30, FADE_WHITE.0, FADE_WHITE.1),
            _ => match cue % 10 {
                1 => c.noise_start(rand),
                2 => c.noise_on = false,
                3 => c.reverse = true,
                4 => c.reverse = false,
                5 => reflex_0580(c),
                6 => c.feedback.off(),
                7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                _ => {}
            },
        }
    }
}

/// `Func_str0581`'s state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0581 {
    pub ctrl: Ctrl,
    pub event: EventObj,
    tables: Tables,
    parts: Parts,
    draws: Vec<PartDraw>,
    transfers: Vec<Transfer>,
    phase: Phase,
    param: u8,
    alive: bool,
}

/// Cue 55's shades: `SetShade(2 << i, 1000, (4 - i) << 28 | 0x808080)`,
/// offset -0.0041 doubling.
const SHADE_Z: u32 = 0x447a_0000;
const SHADE_OFFSET: u32 = 0xbb86_594b;

impl Str0581 {
    /// `Func_str0581` started: as [`Str0580::new`] on `eventObjTbl_0581`.
    pub fn new(tables: &Tables, world: &dyn World, rand: &mut Rand) -> Str0581 {
        let event = EventObj::new(tables, tables.first_0581(), world);
        Str0581 {
            ctrl: Ctrl::new(Some(EFFECT_LAYER), rand),
            event,
            tables: tables.clone(),
            parts: Parts::default(),
            draws: Vec::new(),
            transfers: Vec::new(),
            phase: Phase::Main,
            param: 0,
            alive: true,
        }
    }

    pub fn set_param(&mut self, p: u8) {
        self.param = p;
    }

    pub fn scene_gone(&mut self) {
        self.alive = false;
    }

    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    pub fn part_draws(&self) -> &[PartDraw] {
        &self.draws
    }

    /// The transfers the last pass started.
    pub fn take_transfers(&mut self) -> Vec<Transfer> {
        std::mem::take(&mut self.transfers)
    }

    /// One activation: a pass of its loop; once it leaves, nothing.
    pub fn step(&mut self, world: &dyn World, cues: &[Cue], frame_now: u32, paused: bool, rand: &mut Rand) -> Draws {
        self.draws.clear();
        if self.phase != Phase::Main {
            return Draws::new();
        }
        if self.param != 0 || !self.alive {
            self.ctrl.feedback.off();
            self.parts.clear();
            self.phase = Phase::Done;
            return Draws::new();
        }
        let c = &mut self.ctrl;
        match frame_now {
            1050 => c.feedback.reflex(0x3f80_a3d7, 0x5880_8080),
            1155 => c.feedback.off(),
            _ => {}
        }
        for &Cue { param: cue, obj } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            self.cue(cue, obj, world, rand);
        }
        self.parts.step(world, &self.tables, rand, &mut self.draws);
        self.ctrl.pass(paused, rand)
    }

    fn cue(&mut self, cue: u32, obj: u32, world: &dyn World, rand: &mut Rand) {
        let c = &mut self.ctrl;
        match cue {
            600..=610 => {
                let Some(o) = self.event.set_obj(&self.tables, cue, world) else { return };
                if let EventKind::Eff(e) = self.tables.entries[self.event.at].kind {
                    let at = world.position(o);
                    self.parts.puffs(at, &e, rand);
                }
            }
            65 => c.feedback.reflex_timed(0x3f80_a3d7, 0x4080_8080, 0, 15),
            56 => {
                c.shades.count = 0;
                c.feedback.off();
            }
            55 => {
                let mut off = SHADE_OFFSET;
                c.shades.list.clear();
                for i in 0..2i32 {
                    let mut s =
                        piney_desktop::noiz::Sampling::shade(2 << i, SHADE_Z, ((4 - i) as u32) << 28 | 0x0080_8080);
                    s.offset = [off, off];
                    c.shades.list.push(s);
                    off = ee::mul(off, 0x4000_0000);
                }
                c.shades.count = 2;
                c.feedback.reflex(0x3f81_47ae, 0x5080_8080);
                c.feedback.z = SHADE_Z;
            }
            899 => c.fade.entry(30, 0, 0x8000_0000),
            901..=904 => c.fade.entry(5, FADE_WHITE.1, FADE_WHITE.0),
            900 => c.fade.entry(45, FADE_WHITE.1, FADE_WHITE.0),
            699 => self.transfers.push(Transfer { obj, height: TRANSFER_HEIGHT_0581 }),
            _ => match cue % 10 {
                1 => c.noise_start(rand),
                2 => c.noise_on = false,
                3 => c.reverse = true,
                4 => c.reverse = false,
                5 => c.feedback.reflex(0x3f82_8f5c, 0x4080_8080),
                6 => c.feedback.flag = 2,
                7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                _ => {}
            },
        }
    }
}

/// Names of the objects a task's table and fog list look up, for a
/// world that stands objects in by name (the harness's).
pub fn names(tables: &Tables) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in tables.entries.iter().filter_map(|e| e.name.clone()).chain(FOG_OFF_0581.iter().map(|s| s.to_string())) {
        if !out.contains(&n) {
            out.push(n);
        }
    }
    out
}

/// A world of named stand-ins, each object an index into `names`, at
/// `pos(name)`.
pub struct StandIns<F2: Fn(&str) -> V4> {
    pub names: Vec<String>,
    pub pos: F2,
}

impl<F2: Fn(&str) -> V4> World for StandIns<F2> {
    fn find(&self, name: &str) -> Option<u32> {
        self.names.iter().position(|n| n == name).map(|i| i as u32)
    }
    fn position(&self, obj: u32) -> V4 {
        self.names.get(obj as usize).map_or([0, 0, 0, ONE], |n| (self.pos)(n))
    }
}
