//! What `DUNGEON::SetRoom` stands in the room it builds, after the room's anm
//! and its hits, and what `DUNGEON::Draw` does with it each frame: the water
//! and magma sparks (`SetWater` 0x005c3f30), the room lights and their glows
//! (`SetLight` 0x005c4560), the lakes' statues and flowers and types 3 and
//! 7's walls (`SetObject` 0x005c70c0), types 0-3's animated objects
//! (`SetAnmObject` 0x005c3ad0); `DrawWater`, `DrawEff` and `Draw` step and
//! draw them (docs/engine/dungeon.md, "The dressing").

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use piney_data::ccs::Ccs;
use piney_data::dungeon::Rng;
use piney_desktop::layers::Layers;

use super::{Anm, DungeonArea, Instance, Slot, invers};
use crate::draw::{self, Draw, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::field_ambient::{Op, Sprite};
use crate::field_area::REF_LAYER;
use crate::hit::{HitModel, UNIT};
use crate::pose::Play;
use crate::town::{Light, WaterZero, water_rows};

/// `object[10]`, `anmobj[10]`, `anmobj2[10]`: at most ten of each.
const PIECES: usize = 10;
/// `roomlight[32]`: `SetLight` checks the count before each dummy, and
/// `DrawEff` walks the 32.
const ROOM_LIGHTS: usize = 32;

/// The dressing's places in the hit list after the room's
/// (`DungeonArea::joined`): (CLUMP + k, node) for `object[k]`'s nodes,
/// (ANM_OBJ + k, object) for `anmobj[k]`'s, (ANM_OBJ2 + k, object) for
/// `anmobj2[k]`'s.
pub(super) const CLUMP: usize = 0x100;
pub(super) const ANM_OBJ: usize = 0x200;
pub(super) const ANM_OBJ2: usize = 0x300;

/// `SetWater`'s dummy (an exact name) and `waterAnmName[type]` (@5657,
/// 0x00696af0); the water's own object, whose model water 0 draws with
/// the picture behind it (@5659).
const WATER_DUMMY: &str = "OBJ_0paf0_";
const WATER_ANM: [&str; 10] = [
    "ANM_sd1af0_a",
    "ANM_sd2af0_a",
    "ANM_sd3af0_a",
    "ANM_sd5af0_a",
    "ANM_sd1af0_a",
    "ANM_sd2af0_a",
    "ANM_sd3af0_a",
    "ANM_sd5af0_a",
    "ANM_sd4af0_a",
    "ANM_sd4af0_a",
];
pub(super) const WATER_OBJ: &str = "OBJ_f_8s40_";
/// `DrawWater`'s scroll step, 0.005 (0x3ba3d70a), and `vftoi12`'s 4096.
const WATER_STEP: F = 0x3ba3_d70a;
const K4096: F = 0x4580_0000;

/// `SetWater`'s embers: the dummies, the Eff (@1150) and the height the
/// spark starts from (-100).
const MAGMA: &str = "OBJ_o_magma*";
const SPARK: &str = "EFF_o_spark_s0_";
const SPARK_Z: F = 0xc2c8_0000;
/// `SNOW(ccs, pos)` (gcmn 0x00503cf0): type 3, range 15, speed 1, the
/// Eff's scale 0.5.
const SPARK_RANGE: F = 0x4170_0000;
const SPARK_SPEED: F = ONE;
const HALF: F = 0x3f00_0000;
const TWO: F = 0x4000_0000;
const K10: F = 0x4120_0000;
const K100: F = 0x42c8_0000;
/// `DUNGEON::ChangeClut(eff, add)`'s palettes (gp 0x003783a0): type 0's
/// and type 1's, each with its "c1" and "c2" twins; the lakes and types
/// 2-7 change nothing.
const SPARK_CLUT: [[&str; 3]; 2] =
    [["CLT_sd100o11", "CLT_sd100o11c1", "CLT_sd100o11c2"], ["CLT_sd200o19", "CLT_sd200o19c1", "CLT_sd200o19c2"]];

/// `DUNGEON::SetObject`'s lake pieces: each dummy's clump, then
/// `OBJ_0ps4*`'s picked by `fieldrand(4)`.
const LAKE_PIECES: [(&str, &str); 4] = [
    ("OBJ_0ps0*", "CMP_o_statue_l2_"),
    ("OBJ_0ps1*", "CMP_o_statue_l3_"),
    ("OBJ_0ps2*", "CMP_o_flower_l2_"),
    ("OBJ_0ps3*", "CMP_o_flower_l3_"),
];
const LAKE_PICKED: &str = "OBJ_0ps4*";
const LAKE_PICKS: [&str; 4] = ["CMP_o_statue_l0_", "CMP_o_statue_l1_", "CMP_o_flower_l0_", "CMP_o_flower_l1_"];
/// Types 3 and 7's walls (@7087-7090).
const WALLS: [(&str, &str); 2] = [("OBJ_0paw0*", "ANM_sd5aw0_a"), ("OBJ_0paw1*", "ANM_sd5aw1_a")];
/// `SetAnmObject`'s dummies and anms by type (0-3; @5566 0x00696ad0,
/// @5571 0x00696ae0).
const ANM_OBJECTS: [(&str, &str); 4] = [
    ("OBJ_0pag0_*", "ANM_sd1ag0_a"),
    ("OBJ_0pac0_*", "ANM_sd2ag0_a"),
    ("OBJ_0par0_*", "ANM_sd3ag0_a"),
    ("OBJ_0pab0_*", "ANM_sd5ag0_a"),
];

/// One of `SetLight`'s `ROOMLIGHT`s as its case makes it: the glow at
/// `at` (a vector the dummy's world matrix turns, then its place added),
/// the light's colour (`ccSetColor` of the packed bytes), and the second
/// glow at the same place.
struct Glow {
    eff: &'static str,
    at: [F; 3],
    colour: u32,
    eff2: Option<&'static str>,
}

const fn glow(eff: &'static str, at: [F; 3], colour: u32, eff2: Option<&'static str>) -> Glow {
    Glow { eff, at, colour, eff2 }
}

const WHITE: u32 = 0x00e6_fafa;
const BLUE: u32 = 0x0096_b4fa;
const SKY: u32 = 0x006e_c3e1;
const LEL0: &str = "EFF_0lel0_";
const LEC0: &str = "EFF_0lec0_";
const LEC1: Option<&str> = Some("EFF_0lec1_");
const LEB0: &str = "EFF_0leb0_";
/// The searches in order, with each type's (by `type & 3`, jump tables
/// @6868 and @6869) glows for one dummy.
const LIGHT_M0: &str = "OBJ_o_light_m0*";
const LIGHT_S0: &str = "OBJ_o_light_s0*";
const ALTAR: &str = "OBJ_o_altar_l0*";
const M0: [&[Glow]; 4] = [
    &[glow(LEL0, [0, 0xc016_6666, 0x439b_0000], WHITE, None)],
    &[glow(LEL0, [0, 0xbeb3_3333, 0x439b_0000], WHITE, None)],
    &[glow(LEC0, [0, 0x432f_0000, 0x4357_0000], BLUE, LEC1)],
    &[glow(LEB0, [0, 0x4280_0000, 0x43d0_0000], SKY, None)],
];
const S0: [&[Glow]; 4] = [
    &[glow("EFF_0let0_", [0, 0, 0x42b4_0000], BLUE, Some("EFF_0let1_"))],
    &[glow(LEC0, [0x4234_0000, 0, 0x434d_0000], BLUE, LEC1), glow(LEC0, [0xc234_0000, 0, 0x434d_0000], BLUE, LEC1)],
    &[glow(LEC0, [0, 0x4120_0000, 0x4357_0000], BLUE, LEC1)],
    &[
        glow(LEB0, [0, 0, 0x43c8_0000], WHITE, None),
        glow(LEB0, [0x42c8_0000, 0, 0x4389_8000], WHITE, None),
        glow(LEB0, [0xc2c8_0000, 0, 0x4348_0000], WHITE, None),
    ],
];
const ALTAR_GLOWS: &[Glow] = &[
    glow(LEC0, [0x4348_0000, 0x439b_0000, 0x4357_0000], BLUE, LEC1),
    glow(LEC0, [0xc348_0000, 0x439b_0000, 0x4357_0000], BLUE, LEC1),
];
/// Every omni light's fall-off (`farDownStart` 100, `farDownEnd` 250) and
/// `ccSetColor`'s 1 / 255 (0x3b808081).
const FAR_START: f32 = 100.0;
const FAR_END: f32 = 250.0;
const BY_255: F = 0x3b80_8081;

/// `SetWater`'s water: `water[0..3]` (+0x364), three anms of the type's
/// `ANM_sdNaf0_a` (`SetFogSw(0)`, their `OBJ_f_8s40_` each its own copy
/// with `ccObj::Duplicate`, water 0's drawing the frame buffer:
/// `ccModel::ChangeTex` of `texChunk`), run one step, at the dummy's own
/// matrix moved by the room's centre (not turned). No hits.
#[derive(Clone, Debug)]
pub struct Water {
    pub name: &'static str,
    pub plays: [Play; 3],
    pub root: [V4; 4],
    /// Water 0's model and the texture coordinates `waterUVModifi` last
    /// wrote (None: the file has no `OBJ_f_8s40_`).
    pub water0: Option<WaterZero>,
    /// What the last `DrawWater` passed `ccAnm::SetUV` (`vftoi12` of the
    /// scroll): water 1's U, water 2's V.
    pub uv: u16,
}

/// A `SNOW` of type 3 (0x50 bytes, `fire[k]` +0x54): an ember rising from
/// a magma dummy and starting again there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spark {
    /// +0 frames to its restart, +8 frames to its next turn.
    pub life: i32,
    pub cnt: i32,
    /// +0x20 where it starts, +0x30 its place, +0x40 its step.
    pub base: V4,
    pub pos: V4,
    pub dir: V4,
    /// Its `ccEff`'s place and transparency as the last `Move` and `Draw`
    /// left them.
    pub eff_pos: V4,
    pub transparency: F,
}

/// `fieldrand(n)` (main 0x0019c460); the game divides by `n` with no
/// guard, so for 0 the port takes the step alone.
fn fieldrand(rng: &mut Rng, n: u32) -> u32 {
    if n == 0 { rng.advance() } else { rng.below(n) }
}

/// A `fieldrand` result as a float (the game converts it unsigned; the
/// results are small).
fn fr(rng: &mut Rng, n: u32) -> F {
    ee::from_int(fieldrand(rng, n) as i32)
}

impl Spark {
    /// `SNOW(ccs, base)`: `cnt = fieldrand(10) + 3`, `life = fieldrand(50) +
    /// 10`, then `calcPos3`.
    fn new(base: V4, rng: &mut Rng) -> Spark {
        let cnt = fieldrand(rng, 10) as i32 + 3;
        let life = fieldrand(rng, 50) as i32 + 10;
        let mut s = Spark { life, cnt, base, pos: [0; 4], dir: [0; 4], eff_pos: [0, 0, 0, ONE], transparency: ONE };
        s.calc_pos3(rng);
        s
    }

    /// `(fieldrand(range) - range / 2) / 10`, `fptosi` of the range.
    fn drift(rng: &mut Rng) -> F {
        let n = ee::to_int(SPARK_RANGE) as u32;
        let x = fr(rng, n);
        ee::div(ee::sub(x, ee::div(SPARK_RANGE, TWO)), K10)
    }

    /// `calcPos3(base, pos, dir, range, speed)` (gcmn 0x00502e20): two
    /// draws for a place about the base that `sceVu0CopyVector(pos, base)`
    /// then overwrites (the spark starts at its base, w 1); the step's x
    /// and y drift, z `(fieldrand(3) + 2) * speed`, w 1.
    fn calc_pos3(&mut self, rng: &mut Rng) {
        fieldrand(rng, 300);
        fieldrand(rng, 300);
        self.pos = self.base;
        self.pos[3] = ONE;
        self.dir[0] = Spark::drift(rng);
        self.dir[1] = Spark::drift(rng);
        self.dir[2] = ee::mul(ee::from_int(fieldrand(rng, 3) as i32 + 2), SPARK_SPEED);
        self.dir[3] = ONE;
    }

    /// `SNOW::Move` (0x00503e80) for type 3: no wrap about the player;
    /// a new drift every `cnt` frames, back to the base when `life` runs
    /// out.
    fn step(&mut self, rng: &mut Rng) {
        self.pos = ee::vadd(self.pos, self.dir);
        self.pos[3] = ONE;
        self.eff_pos = self.pos;
        self.life -= 1;
        self.cnt -= 1;
        if self.cnt == 0 {
            self.cnt = fieldrand(rng, 10) as i32 + 3;
            self.dir[0] = Spark::drift(rng);
            self.dir[1] = Spark::drift(rng);
        }
        if self.life == 0 {
            self.calc_pos3(rng);
            self.eff_pos = self.pos;
            self.life = fieldrand(rng, 60) as i32 + 20;
        }
    }

    /// `SNOW::Draw` (0x005042f0): fading over its last 32 frames.
    fn fade(&mut self) {
        self.transparency = if self.life < 32 { ee::div(ee::from_int(self.life), K100) } else { ONE };
    }
}

/// One `ROOMLIGHT` (0x40 bytes, `roomlight[k]` +0x374).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomLight {
    /// +0x30 the glow (`Init(chunk, 1)`, its fog bit cleared), +0x10 its
    /// place, +0 its pattern.
    pub eff: &'static str,
    pub pos: V4,
    pub pat: u16,
    /// +0x34 the second glow (fogged) at +0x20, pattern +4.
    pub eff2: Option<(&'static str, V4)>,
    pub pat2: u16,
    /// +0x38: its `ccOmniLight`'s place in the room's light group.
    pub light: usize,
    /// The patterns the last `DrawEff` drew.
    pub shown: (u16, u16),
}

/// `object[k]` (+0xd3588): a lake's statue or flower, `EntryObject`'s
/// `ccClump` of the dungeon file's `name`, its hits on (`HitEnable(1)`)
/// and every node's at its matrix `m`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clump {
    pub name: &'static str,
    pub nodes: Vec<u32>,
    pub m: [V4; 4],
}

/// What `SetRoom` stood in the room built.
#[derive(Clone, Debug, Default)]
pub struct Dressing {
    pub water: Option<Water>,
    /// `fire[0..60]`.
    pub sparks: Vec<Spark>,
    pub lights: Vec<RoomLight>,
    pub objects: Vec<Clump>,
    /// `anmobj[0..10]` (types 3 and 7), `anmobj2[0..10]` (types 0-3).
    pub anm_objects: Vec<Anm>,
    pub anm_objects2: Vec<Anm>,
}

/// Each Eff chunk's (0x0e00) pattern transparencies by its `EFF_` object:
/// `patNum` (u16 at +14) patterns of 8 bytes from +36, the transparency
/// (4096 = 1) at +4 of each (`crates/piney-effect/src/eff.rs`).
pub(super) fn eff_patterns(ccs: &Ccs) -> HashMap<u32, Vec<u16>> {
    let d: &[u8] = &ccs.data;
    let u16_at = |p: usize| u16::from_le_bytes([d[p], d[p + 1]]);
    let mut out = HashMap::new();
    for ch in ccs.walk().chunks {
        if ch.in_frames || ch.kind != 0x0e00 {
            continue;
        }
        let p = ch.payload();
        if p + 36 > d.len() {
            continue;
        }
        let n = usize::from(u16_at(p + 14));
        if p + 36 + 8 * n > d.len() {
            continue;
        }
        let object = u32::from_le_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]);
        out.insert(object, (0..n).map(|i| u16_at(p + 36 + 8 * i + 4)).collect());
    }
    out
}

/// `sceVu0RotMatrix(m, local, (0, 0, rotate))` then `sceVu0TransMatrix(m,
/// m, (x, y, 0))`: a dummy's own matrix turned and moved with the room.
fn placed(local: &[V4; 4], slot: &Slot, turn: bool) -> [V4; 4] {
    let mut m = if turn { piney_data::anim::rot_bits_of(*local, [0, 0, slot.rotate]) } else { *local };
    for (k, p) in [slot.pos[0], slot.pos[1], 0].into_iter().enumerate() {
        m[3][k] = ee::add(m[3][k], p);
    }
    m
}

impl DungeonArea {
    /// `ccAnm::GetSubstAdrs(pattern)` on the room's anm: its objects whose
    /// name (an ExtObj copy's target's) is the pattern, or with a trailing
    /// `*` starts with it, in the anm's order.
    fn subst(&self, pattern: &str) -> Vec<Instance> {
        let Some(room) = &self.room else { return Vec::new() };
        let (want, prefix) = match pattern.strip_suffix('*') {
            Some(p) => (p, true),
            None => (pattern, false),
        };
        room.objects
            .iter()
            .filter(|o| {
                self.name_in(room.special, o.target)
                    .is_some_and(|n| if prefix { n.starts_with(want) } else { n == want })
            })
            .cloned()
            .collect()
    }

    /// The dressing gone (`DeleteRoom`, `ClearRoom`).
    pub(super) fn clear_dress(&mut self) {
        self.dress = Dressing::default();
    }

    /// `SetWater(f, i)`: the water at the room's first `OBJ_0paf0_`, then a
    /// spark at each `OBJ_o_magma*` dummy's place as its world matrix
    /// (`lwMatrix`) holds it, z -100.
    pub(super) fn set_water(&mut self, slot: &Slot) {
        self.dress.water = None;
        if let Some(d) = self.subst(WATER_DUMMY).first() {
            let name = WATER_ANM[usize::from(self.dtype).min(9)];
            if let Some(mut play) = Play::new(&self.file, name) {
                play.forward(&self.file);
                let root = placed(&d.local, slot, false);
                let water0 = self.water_model.clone().map(|model| WaterZero { model, st: None, reach: 0x7f7f_ffff });
                self.dress.water = Some(Water { name, plays: [play.clone(), play.clone(), play], root, water0, uv: 0 });
            }
        }
        for d in self.subst(MAGMA) {
            let t = d.world[3];
            let s = Spark::new([t[0], t[1], SPARK_Z, t[3]], &mut self.rng);
            self.dress.sparks.push(s);
        }
    }

    /// The (new, old) palette `SetWater`'s `DUNGEON::ChangeClut(eff, "c1")`
    /// (clutType 3) or `"c2"` (clutType 4) asks of each spark: types 0
    /// and 1 only; `ccEff::ChangeClut` changes it only while the spark's
    /// palette is the old one.
    pub fn spark_clut(&self) -> Option<(&'static str, &'static str)> {
        let names = SPARK_CLUT.get(usize::from(self.dtype))?;
        match self.tex_clut.clut_type {
            3 => Some((names[1], names[0])),
            4 => Some((names[2], names[0])),
            _ => None,
        }
    }

    /// The number of patterns of the dungeon file's Eff `name`.
    fn pat_num(&self, name: &str) -> u32 {
        self.file.ccs.find_object(name).and_then(|o| self.eff_pats.get(&o)).map_or(0, |p| p.len() as u32)
    }

    /// `SetLight(f, i)`: the old room lights gone (`DelGrp`), then for
    /// each dummy of the three searches, while fewer than 32 are made, its
    /// case's `ROOMLIGHT`s (types 8 and 9 none; the altar's only in types 2
    /// and 6). A case of several fills `roomlight[n..]` and adds its count
    /// after (so a room could run past the 32 into `door[]`; none of the
    /// files' rooms does).
    pub(super) fn set_light(&mut self) {
        let ty = usize::from(self.dtype);
        let mut n = 0usize;
        let searches: [(&str, &[Glow]); 3] = [
            (LIGHT_M0, M0[ty & 3]),
            (LIGHT_S0, S0[ty & 3]),
            (ALTAR, if matches!(ty, 2 | 6) { ALTAR_GLOWS } else { &[] }),
        ];
        for (pattern, glows) in searches {
            for d in self.subst(pattern) {
                if n >= ROOM_LIGHTS {
                    return;
                }
                if ty >= 8 {
                    continue;
                }
                for g in glows {
                    self.room_light(g, &d.world);
                }
                n += glows.len();
            }
        }
    }

    /// One `ROOMLIGHT` at a dummy whose world matrix is `lw`: the glow at
    /// `lw` applied to the case's vector (w 0) plus its place, its pattern
    /// `fieldrand(patNum)`; the omni light (type 4, priority 1, the case's
    /// colour, falling off from 100 to 250) at the dummy's place, into the
    /// group (`AddGrp`); the second glow at the same place, its pattern
    /// drawn after.
    fn room_light(&mut self, g: &Glow, lw: &[V4; 4]) {
        let t = lw[3];
        let pos = ee::vadd(ee::apply(lw, [g.at[0], g.at[1], g.at[2], 0]), t);
        let n = self.pat_num(g.eff);
        let pat = fieldrand(&mut self.rng, n) as u16;
        let byte = |k: u32| ee::mul(ee::from_int(((g.colour >> (8 * k)) & 0xff) as i32), BY_255);
        let colour = Vec3::new(ee::f(byte(0)), ee::f(byte(1)), ee::f(byte(2)));
        self.lights.lights.push(Light {
            kind: 4,
            pos: Vec3::new(ee::f(t[0]), ee::f(t[1]), ee::f(t[2])),
            dir: Vec3::ZERO,
            colour,
            intensity: 1.0,
            far_start: FAR_START,
            far_end: FAR_END,
            radius: [0.0; 2],
            priority: 1,
        });
        let light = self.lights.lights.len() - 1;
        let (eff2, pat2) = match g.eff2 {
            Some(e) => {
                let n = self.pat_num(e);
                (Some((e, pos)), fieldrand(&mut self.rng, n) as u16)
            }
            None => (None, 0),
        };
        self.dress.lights.push(RoomLight { eff: g.eff, pos, pat, eff2, pat2, light, shown: (pat, pat2) });
    }

    /// `SetObject(f, i)`. The lake types: a clump at each `OBJ_0ps0*` ..
    /// `OBJ_0ps3*` dummy of `room[level][i]`, then `OBJ_0ps4*`'s each picked by
    /// `fieldrand(4)`. Types 3 and 7: a wall anm at each `OBJ_0paw0*`, then
    /// `OBJ_0paw1*` dummy (at most ten). The game stores each new anm's matrix by
    /// the dummy's index in its own search, so a paw1 wall moves the paw0 wall of
    /// its index and keeps its own unit matrix, unless the room has no paw0
    /// walls (docs/engine/dungeon.md).
    pub(super) fn set_object(&mut self, f: usize, slot: &Slot) {
        if matches!(self.dtype, 8 | 9) && self.level == f {
            let mut num = 0;
            for (pattern, name) in LAKE_PIECES {
                for d in self.subst(pattern) {
                    self.entry_object(&d, slot, name, num);
                    num += 1;
                }
            }
            for d in self.subst(LAKE_PICKED) {
                let k = fieldrand(&mut self.rng, 4) as usize;
                self.entry_object(&d, slot, LAKE_PICKS[k], num);
                num += 1;
            }
        }
        if !matches!(self.dtype, 3 | 7) {
            return;
        }
        let mut s1 = 0usize;
        for (pattern, name) in WALLS {
            for (s0, d) in self.subst(pattern).into_iter().enumerate() {
                let Some(anm) = self.make_anm_in(false, name, &UNIT, 1) else { return };
                self.dress.anm_objects.push(anm);
                let m = placed(&d.local, slot, true);
                if let Some(a) = self.dress.anm_objects.get(s0) {
                    let objects = self.pose(&a.play, &m);
                    let a = &mut self.dress.anm_objects[s0];
                    a.root = m;
                    a.objects = objects;
                }
                let a = &mut self.dress.anm_objects[s1];
                a.hit_world = a.objects.iter().map(|o| o.world).collect();
                for o in 0..a.objects.len() {
                    self.joined.push((ANM_OBJ + s1, o));
                }
                s1 += 1;
                if s1 == PIECES {
                    return;
                }
            }
        }
    }

    /// `EntryObject(dummy, f, i, name, num)`: for `num` under 10, the clump
    /// at the dummy's own matrix turned and moved with the room, its hits
    /// joining the list (the lakes' palette swaps by `GetBG`, taken as 0,
    /// change nothing).
    fn entry_object(&mut self, d: &Instance, slot: &Slot, name: &'static str, num: usize) {
        if num >= PIECES {
            return;
        }
        let nodes = self
            .file
            .ccs
            .find_object(name)
            .and_then(|c| self.file.scene.clumps.iter().find(|(o, _)| *o == c))
            .map(|(_, n)| n.clone())
            .unwrap_or_default();
        let k = self.dress.objects.len();
        for n in 0..nodes.len() {
            self.joined.push((CLUMP + k, n));
        }
        self.dress.objects.push(Clump { name, nodes, m: placed(&d.local, slot, true) });
    }

    /// `SetAnmObject(f, i)`: types 0-3, the type's anm at each of its
    /// dummies (at most ten), one step on, turned and moved with the room,
    /// its hits on.
    pub(super) fn set_anm_object(&mut self, slot: &Slot) {
        let Some(&(pattern, name)) = ANM_OBJECTS.get(usize::from(self.dtype)) else { return };
        for d in self.subst(pattern).into_iter().take(PIECES) {
            let m = placed(&d.local, slot, true);
            let Some(anm) = self.make_anm_in(false, name, &m, 1) else { return };
            let k = self.dress.anm_objects2.len();
            for o in 0..anm.objects.len() {
                self.joined.push((ANM_OBJ2 + k, o));
            }
            self.dress.anm_objects2.push(anm);
        }
    }

    /// The hit models of the dressing's entry (kind, index) in the list.
    pub(super) fn dress_hits(&self, kind: usize, o: usize) -> Vec<HitModel> {
        match kind {
            k if (CLUMP..CLUMP + PIECES).contains(&k) => {
                let Some(c) = self.dress.objects.get(k - CLUMP) else { return Vec::new() };
                self.clump_hits(&c.nodes, &c.m, 0x10 + (k - CLUMP) as u32, o)
            }
            k if (ANM_OBJ..ANM_OBJ + PIECES).contains(&k) => match self.dress.anm_objects.get(k - ANM_OBJ) {
                Some(a) => self.obj_hits(a, o, 0x20 + (k - ANM_OBJ) as u32),
                None => Vec::new(),
            },
            k if (ANM_OBJ2..ANM_OBJ2 + PIECES).contains(&k) => match self.dress.anm_objects2.get(k - ANM_OBJ2) {
                Some(a) => self.obj_hits(a, o, 0x30 + (k - ANM_OBJ2) as u32),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// The names of the dressing's entry's hit models (for the checks).
    pub(super) fn dress_hit_names(&self, kind: usize, o: usize) -> Vec<String> {
        if (CLUMP..CLUMP + PIECES).contains(&kind) {
            let n = self.dress_hits(kind, o).len();
            let name = self.dress.objects.get(kind - CLUMP).map_or("?", |c| c.name);
            return vec![name.to_string(); n];
        }
        let anm = if (ANM_OBJ..ANM_OBJ + PIECES).contains(&kind) {
            self.dress.anm_objects.get(kind - ANM_OBJ)
        } else {
            self.dress.anm_objects2.get(kind - ANM_OBJ2)
        };
        let Some(inst) = anm.and_then(|a| a.objects.get(o)) else { return Vec::new() };
        let n = self.dress_hits(kind, o).len();
        vec![self.name_in(false, inst.controller).unwrap_or("?").to_string(); n]
    }

    /// Node `k` of a clump's hit models at `m`, `type` 1.
    pub(super) fn clump_hits(&self, nodes: &[u32], m: &[V4; 4], tag: u32, k: usize) -> Vec<HitModel> {
        let mut out = Vec::new();
        let Some(models) = nodes.get(k).and_then(|n| self.obj_models.get(n)) else { return out };
        for model in models {
            for (j, h) in self.hit_models.get(model).map(Vec::as_slice).unwrap_or(&[]).iter().enumerate() {
                let mut h = h.clone();
                h.rm = *m;
                h.im = invers(m);
                h.kind = 1;
                h.id = (tag << 20) | ((k as u32) << 4) | j as u32;
                out.push(h);
            }
        }
        out
    }

    /// `DrawEff` for this frame: each spark moved (with `step`) and drawn
    /// (`SNOW::Move`, `SNOW::Draw`); each room light's glows drawn at their
    /// patterns and, with `step`, the light's intensity the glow's
    /// transparency times its pattern's (over 4096) and both patterns on
    /// by one (the second's first). All on `EFF_LAYER` (3). The lakes'
    /// fireflies, tree and leaves are not ported.
    pub fn draw_eff(&mut self, step: bool) -> Vec<Op> {
        let mut out = Vec::new();
        let clut = self.spark_clut();
        let file = self.file_name;
        let sprite = |name: &'static str, pos: V4, pattern: u16, scale: F, transparency: F, fog: bool| {
            Op::Sprite(Sprite {
                layer: 3,
                file,
                name,
                pos,
                pattern,
                scale: [scale; 2],
                rotate: 0,
                transparency,
                fog,
                ztest: true,
                colour: None,
                clut: None,
            })
        };
        for s in &mut self.dress.sparks {
            if step {
                s.step(&mut self.rng);
                s.fade();
            }
            let mut op = sprite(SPARK, s.eff_pos, 0, HALF, s.transparency, false);
            if let Op::Sprite(sp) = &mut op {
                sp.clut = clut;
            }
            out.push(op);
        }
        let file_ccs = &self.file.ccs;
        for rl in self.dress.lights.iter_mut().take(ROOM_LIGHTS) {
            if step {
                rl.shown = (rl.pat, rl.pat2);
            }
            out.push(sprite(rl.eff, rl.pos, rl.shown.0, ONE, ONE, false));
            if let Some((e2, pos2)) = rl.eff2 {
                out.push(sprite(e2, pos2, rl.shown.1, ONE, ONE, true));
            }
            if !step {
                continue;
            }
            let pats = |name: &str| file_ccs.find_object(name).and_then(|o| self.eff_pats.get(&o));
            if let Some((e2, _)) = rl.eff2 {
                let n = pats(e2).map_or(0, Vec::len);
                rl.pat2 += 1;
                if usize::from(rl.pat2) >= n {
                    rl.pat2 = 0;
                }
            }
            let p = pats(rl.eff);
            let t = p.and_then(|p| p.get(usize::from(rl.pat))).copied().unwrap_or(4096);
            let tr = ee::mul(ONE, ee::div(ee::from_int(i32::from(t)), K4096));
            if let Some(l) = self.lights.lights.get_mut(rl.light) {
                l.intensity = ee::f(tr);
            }
            rl.pat += 1;
            if usize::from(rl.pat) >= p.map_or(0, Vec::len) {
                rl.pat = 0;
            }
        }
        out
    }

    /// `DrawWater`: the three steps (with `step`); `ccAnm::SetUV` of
    /// `vftoi12(u)` into water 1's U and water 2's V, then the scroll on by
    /// 0.005, back to 0 at 1 (`u` lives as long as the dungeon here, the
    /// whole game in the game); `waterUVModifi` of water 0's
    /// `OBJ_f_8s40_`; waters 1 and 2 on `objLayer`, water 0 on `refLayer`
    /// with the picture behind it, none fogged.
    fn draw_water(&mut self, layers: &mut Layers, to_screen: Mat4, world_screen: &[V4; 4], step: bool) {
        let volume = self.hits.volume;
        let Some(w) = &mut self.dress.water else { return };
        if step {
            for p in &mut w.plays {
                p.forward(&self.file);
            }
            w.uv = (ee::to_int(ee::mul(self.water_u, K4096)) & 0xffff) as u16;
            self.water_u = ee::add(self.water_u, WATER_STEP);
            if !ee::lt(self.water_u, ONE) {
                self.water_u = 0;
            }
        }
        let file = &*self.file;
        let root = draw::mat(&w.root);
        if let Some(w0) = &mut w.water0 {
            let worlds = w.plays[0].worlds(file, root, &[]);
            let lw = worlds.iter().find(|(o, _)| {
                let target = file.scene.ext.get(o).copied().unwrap_or(**o);
                file.obj_model.get(&target) == Some(&w0.model.object)
            });
            if let Some((_, lw)) = lw {
                let lw: [V4; 4] = lw.to_cols_array_2d().map(|c| c.map(f32::to_bits));
                w0.modify(volume, &lw, lw[3], lw[3], world_screen);
            }
        }
        for k in [1usize, 2, 0] {
            let play = &w.plays[k];
            let rows = if k == 0 { play.uv_rows(file) } else { water_rows(file, k, w.uv) };
            let edits = if k == 0 { w.water0.as_ref().filter(|w0| w0.st.is_some()) } else { None };
            let e = edits.map(|w0| (w0.model.object, w0.edits()));
            let layer = if k == 0 { REF_LAYER } else { OBJ_LAYER };
            for (&obj, &world) in &play.worlds(file, root, &[]) {
                let target = file.scene.ext.get(&obj).copied().unwrap_or(obj);
                let Some(&model) = file.obj_model.get(&target) else { continue };
                let d = Draw {
                    file,
                    model,
                    world,
                    alpha: 1.0,
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: self.clut_swaps.clone(),
                };
                let edit = e.as_ref().filter(|(m, _)| *m == model).map(|(_, x)| x);
                draw::model_edited(layers, layer, to_screen, d, edit, draw::Fogging::None);
            }
        }
    }

    /// An anm's objects' models at their world matrices on `objLayer`,
    /// fogged by depth, with the file's palette swaps.
    fn draw_anm_models(&self, layers: &mut Layers, to_screen: Mat4, anm: &Anm, fog: piney_draw::DepthFog) {
        let file = &*self.file;
        let rows = anm.play.uv_rows(file);
        for o in &anm.objects {
            let Some(models) = self.obj_models.get(&o.target) else { continue };
            let world = draw::mat(&o.world);
            for &model in models {
                if file.models.get(&model).is_none_or(|i| i.mtype & 0x600 == 0x600) {
                    continue;
                }
                let d = Draw {
                    file,
                    model,
                    world,
                    alpha: 1.0,
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: self.clut_swaps.clone(),
                };
                draw::model_edited(layers, OBJ_LAYER, to_screen, d, None, draw::Fogging::Depth(fog));
            }
        }
    }

    /// A clump's nodes' models at `m` (`ccClump::Draw`), as the ban
    /// block's.
    pub(super) fn draw_clump(&self, layers: &mut Layers, to_screen: Mat4, nodes: &[u32], m: &[V4; 4]) {
        let fog = self.room_depth_fog();
        let world = draw::mat(m);
        let rows = HashMap::new();
        let mut models: Vec<u32> = nodes.iter().filter_map(|n| self.obj_models.get(n)).flatten().copied().collect();
        models.dedup();
        for model in models {
            if self.file.models.get(&model).is_none_or(|i| i.mtype & 0x600 == 0x600) {
                continue;
            }
            let d = Draw {
                file: &self.file,
                model,
                world,
                alpha: 1.0,
                rows: &rows,
                lights: None,
                nodes: &[],
                morph: Vec::new(),
                clut_swaps: self.clut_swaps.clone(),
            };
            draw::model_edited(layers, OBJ_LAYER, to_screen, d, None, draw::Fogging::Depth(fog));
        }
    }

    /// `DUNGEON::Draw`'s part before the room: `DrawWater`, `DrawEff` (its
    /// sprites handed back for the effects), then for k < 10 `object[k]`,
    /// `anmobj[k]` and `anmobj2[k]`, each anm one step on (with `step`)
    /// and drawn. Drawn whether or not the player's cell is the room's.
    pub(super) fn draw_dress(
        &mut self,
        layers: &mut Layers,
        to_screen: Mat4,
        world_screen: &[V4; 4],
        step: bool,
    ) -> Vec<Op> {
        self.draw_water(layers, to_screen, world_screen, step);
        let out = self.draw_eff(step);
        if step {
            let file = self.file.clone();
            for list in [&mut self.dress.anm_objects, &mut self.dress.anm_objects2] {
                for a in list.iter_mut() {
                    a.play.forward(&file);
                }
            }
            let posed: Vec<Vec<Instance>> = self
                .dress
                .anm_objects
                .iter()
                .chain(&self.dress.anm_objects2)
                .map(|a| self.pose(&a.play, &a.root))
                .collect();
            for (a, objects) in self.dress.anm_objects.iter_mut().chain(&mut self.dress.anm_objects2).zip(posed) {
                a.objects = objects;
            }
        }
        let fog = self.room_depth_fog();
        for k in 0..PIECES {
            if let Some(c) = self.dress.objects.get(k) {
                self.draw_clump(layers, to_screen, &c.nodes, &c.m);
            }
            if let Some(a) = self.dress.anm_objects.get(k) {
                self.draw_anm_models(layers, to_screen, a, fog);
            }
            if let Some(a) = self.dress.anm_objects2.get(k) {
                self.draw_anm_models(layers, to_screen, a, fog);
            }
        }
        out
    }
}
