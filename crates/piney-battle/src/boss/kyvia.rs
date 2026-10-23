//! Kyvia's fights ([`Fight`]): `bossFunc` codes 12 (`ccBossKyvia01`, MUT
//! gcmn 0x004d2960-0x004d93e8, event 108 on field 9's disc), 13
//! (Outbreak's field 10), 14 (Quarantine's field 11) and 15 (field 12,
//! [`fourth`]). The body stands off the disc and strikes; its core
//! (`kyviaCore`, [`core`]) is the target, with five gomoras
//! (`kyviaGomora`, [`gomora`]) about it, each a `ccBoss` taken out of the
//! body's frame (a [`Tree`]). docs/engine/boss-kyvia.md and its fights'.

pub mod core;
pub mod fourth;
pub mod gomora;
pub mod hell;
pub mod magic_ball;
pub mod thunder;

use piney_data::field::ee;

use self::core::Core;
use self::gomora::Gomora;
use super::blur::Set;
use super::{AfterImageParams, Anm, Boss, Class, Cx, EffKind, Out, VF0};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, M4, V4};
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_PI: F = 0xc049_0fdb;

/// `bossFunc`'s codes: Kyvia's first fight, its second, third and fourth.
pub const CODE: i32 = 12;
pub const CODE_SECOND: i32 = 13;
pub const CODE_THIRD: i32 = 14;
pub const CODE_FOURTH: i32 = 15;
/// The body's `bodyHit` radius, 400 in both constructors (MUT gcmn
/// 0x004d2d10, OUT 0x004d5b60); its height and kind are its row's.
pub const BODY_RADIUS: F = 0x43c8_0000;

/// The stream the core and each gomora draw from at level `lv`:
/// `CoreInit(lv)` (MUT gcmn 0x004f82c0) and `GomoraInit(lv)` (0x004fe460)
/// both take `GetCCSAdrs("x01")` to `"x04"` by level (none past them). The
/// gomoras' clip, `ANM_ex0gnut0`, draws `CMP_trall3`'s objects, and its
/// morpher `MPH_ex01gom1` bends `MDL_ex01gom1` through `MDL_ex01gom2`-`4`
/// in turn: no gomora picks one of them.
pub fn stream(lv: i16) -> Option<&'static str> {
    ["x01", "x02", "x03", "x04"].get(usize::try_from(lv).ok()?.checked_sub(1)?).copied()
}

/// Which of Kyvia's fights the body is: its first (`ccBossKyvia01`,
/// Mutation's field 9), its second (`ccBossKyvia02`, Outbreak's field 10:
/// two stages, level 2, [`thunder`]), its third (`ccBossKyvia03`,
/// Quarantine's field 11: three stages, level 3, [`magic_ball`]) or its
/// fourth (`ccBossKyvia04`, field 12: four places, the EX turn and the
/// rebirth, [`fourth`]). Each later one is the one before's code with its
/// own numbers wherever they differ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fight {
    #[default]
    First,
    Second,
    Third,
    Fourth,
}

impl Fight {
    /// The fight of a `bossFunc` code.
    pub fn of_code(code: i32) -> Option<Fight> {
        match code {
            CODE => Some(Fight::First),
            CODE_SECOND => Some(Fight::Second),
            CODE_THIRD => Some(Fight::Third),
            CODE_FOURTH => Some(Fight::Fourth),
            _ => None,
        }
    }

    /// The body's `bossTbl` row: its code, but the third's constructor
    /// takes 13 as the second's does (QUA gcmn 0x003d0bec).
    pub fn row(self) -> usize {
        match self {
            Fight::First => 12,
            Fight::Second | Fight::Third => 13,
            Fight::Fourth => 15,
        }
    }

    /// The core's and the gomoras' level (`CoreInit(kLv)`).
    pub fn level(self) -> i16 {
        match self {
            Fight::First => 1,
            Fight::Second => 2,
            Fight::Third => 3,
            Fight::Fourth => 4,
        }
    }

    /// The disc's stages (`DiscMaxLV`).
    fn stages(self) -> i16 {
        self.level()
    }

    /// The fight's file: the body's, and the core's and the gomoras'
    /// stream at its level ([`stream`]).
    pub fn file(self) -> &'static str {
        stream(self.level()).unwrap_or("x01")
    }

    /// The boss skills of the arm, the beams and the meteors
    /// (`HandAtk`, `LightAtk`, `KyviaMagicDamage`).
    fn skills(self) -> [usize; 3] {
        match self {
            Fight::First => [33, 32, 34],
            Fight::Second => [36, 37, 38],
            Fight::Third => [76, 37, 38],
            Fight::Fourth => [44, 52, 38],
        }
    }

    /// From the second fight on: the stages, the sounds at the disc and
    /// the numbers the second fight changed.
    fn later(self) -> bool {
        self != Fight::First
    }
}

/// The body's acts (`actNum`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    /// Struck while the core is: its smoke and the camera (`CheckDmgSmokeEff`).
    pub const DAMAGE: i16 = 1;
    pub const DAMAGE2: i16 = 2;
    pub const HAND: i16 = 3;
    pub const LIGHT: i16 = 4;
    pub const MEGID: i16 = 5;
    /// The second and third fights' thunderbolts ([`super::thunder`]).
    pub const THUNDER: i16 = 6;
    /// The third fight's magical ball ([`super::magic_ball`]); 21 is idle
    /// as 11-13 are.
    pub const MAGIC_BALL: i16 = 20;
    pub const DEAD: i16 = 14;
    /// The fourth fight's ([`super::fourth`]): hell's express, the great
    /// meteor, the skills and the rebirth.
    pub const HELL: i16 = 21;
    pub const BIG_MEGID: i16 = 22;
    pub const SKILL: i16 = 23;
    pub const REBIRTH: i16 = 24;
}

/// Kyvia's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KyviaData {
    /// `Kyvia01AnmTbl` to `Kyvia03AnmTbl`, `kyviaCoreAnmTbl`,
    /// `kyviaGomoraAnmTbl`.
    pub anims: Vec<Option<String>>,
    pub anims_second: Vec<Option<String>>,
    pub anims_third: Vec<Option<String>>,
    /// `Kyvia04AnmTbl` and the EX turn's `Kyvia04AnmTbl_EX`.
    pub anims_fourth: Vec<Option<String>>,
    pub anims_fourth_ex: Vec<Option<String>>,
    pub core_anims: Vec<Option<String>>,
    pub gomora_anims: Vec<Option<String>>,
    /// `AllGomoraList_1[0]`: each list the gomoras' attributes by slave.
    pub gomora_lists: Vec<[i16; 5]>,
    /// `AllGomoraList_2`: level 2's lists by the core's deaths, and level
    /// 3's (`ListStepUp`'s third table, Mutation on).
    pub gomora_lists_second: Vec<Vec<[i16; 5]>>,
    pub gomora_lists_third: Vec<Vec<[i16; 5]>>,
    pub gomora_lists_fourth: Vec<Vec<[i16; 5]>>,
    /// `Skill_VARIOUS`, `Skill_DOWNER`, and the fourth fight's
    /// `Skill_KYVIA_VARIOUS` (`Skill_Atk`'s).
    pub various: Vec<i16>,
    pub downer: Vec<i16>,
    pub kyvia_various: Vec<i16>,
}

impl KyviaData {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> KyviaData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        let row = |l: &[i16]| std::array::from_fn(|k| l.get(k).copied().unwrap_or(4));
        KyviaData {
            anims: s(t.kyvia01_anims()),
            anims_second: s(t.kyvia02_anims()),
            anims_third: s(t.kyvia03_anims()),
            anims_fourth: s(t.kyvia04_anims()),
            anims_fourth_ex: s(t.kyvia04_anims_ex()),
            core_anims: s(t.kyvia_core_anims()),
            gomora_anims: s(t.kyvia_gomora_anims()),
            gomora_lists: t.kyvia_gomora_lists().iter().map(|l| row(l)).collect(),
            gomora_lists_second: t
                .kyvia_gomora_lists_2()
                .iter()
                .map(|step| step.iter().map(|l| row(l)).collect())
                .collect(),
            gomora_lists_third: t
                .kyvia_gomora_lists_3()
                .iter()
                .map(|step| step.iter().map(|l| row(l)).collect())
                .collect(),
            gomora_lists_fourth: t
                .kyvia_gomora_lists_4()
                .iter()
                .map(|step| step.iter().map(|l| row(l)).collect())
                .collect(),
            various: t.kyvia_various_skills().to_vec(),
            downer: t.kyvia_downer_skills().to_vec(),
            kyvia_various: t.kyvia_skills_various().to_vec(),
        }
    }

    /// `AllGomoraList_lv[step]`: the lists a gomora of level `lv` takes
    /// after the core's `step`th death; none past the table (a null).
    pub fn gomora_lists_of(&self, lv: i16, step: usize) -> Option<&[[i16; 5]]> {
        match (lv, step) {
            (1, 0) => Some(&self.gomora_lists),
            (2, _) => self.gomora_lists_second.get(step).map(Vec::as_slice),
            (3, _) => self.gomora_lists_third.get(step).map(Vec::as_slice),
            (4, _) => self.gomora_lists_fourth.get(step).map(Vec::as_slice),
            _ => None,
        }
    }
}

/// The fight's particle tables (gcmn, rows 0x38 bytes), each with its
/// force fields: `KyviaGenerator` (MUT 0x0061f650), `KyviaDmgGenerator`
/// (0x0061fce0), `CoreGenerator` (0x0061f3d0), `GomoraGenerator`
/// (0x0061eec0), `GomoraAuraGenerator` (0x0061f1b0), and the missiles'
/// `MissileSmokeGenerator` and `BurstSmokeGenerator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Kyvia,
    KyviaDmg,
    Core,
    Gomora,
    GomoraAura,
    MissileSmoke,
    BurstSmoke,
}

impl Table {
    /// Its name in the game's symbols, less `Generator`.
    pub fn name(self) -> &'static str {
        match self {
            Table::Kyvia => "Kyvia",
            Table::KyviaDmg => "KyviaDmg",
            Table::Core => "Core",
            Table::Gomora => "Gomora",
            Table::GomoraAura => "GomoraAura",
            Table::MissileSmoke => "MissileSmoke",
            Table::BurstSmoke => "BurstSmoke",
        }
    }
}

/// One `new ccParticleGenerator(row, ff...)`: the table, its row, the
/// rows of its force field table (`ff_table`'s, the table's own but for
/// one of the fourth fight's) in order, and `distSW` (+4 bit 0) as the
/// maker leaves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenRow {
    pub table: Table,
    pub row: u8,
    pub ff: &'static [u8],
    pub ff_table: Table,
    pub dist_sw: bool,
}

/// The fight's particle generators, by where the game makes them (MUT
/// gcmn; Outbreak's code is the same).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gen {
    /// `LightAtk`: the light gathering (Kyvia row 0, ff 0), the flash as
    /// the beams leave (row 7, ff 10 11), a beam's head and tail (row 5, ff
    /// 1 / 8) and its burst at a member (row 3, ff 12 4).
    Light,
    BeamsOut,
    BeamHead,
    BeamTail,
    BeamBurst,
    /// `CheckDmgSmokeEff`'s smoke (KyviaDmg row 0, ff 0 1) and the death's
    /// (row 1, ff 2 3; the second fight's with `distSW` off).
    Smoke,
    DeadSmoke(Fight),
    /// The core's rise (Core row 0, ff 0), its auras (rows 7 and 8, ff 3)
    /// and its death's (rows 1 and 2, ff 1).
    CoreRise,
    CoreAura(u8),
    CoreDeath(u8),
    /// A gomora out of the core (Gomora rows 5 and 6, ff 6 5 / 7 5), its
    /// dash's blow (row 0, ff 10 9) and its death's (row 1, ff 0 2 4; row
    /// 2, ff 3 4).
    GomoraOut(u8),
    GomoraBlow,
    GomoraDeath(u8),
    /// A gomora's aura by attribute and level (GomoraAura row attribute, ff
    /// level), and its attack's (row 4 + attribute, ff 0).
    GomoraAura(u8, u8),
    GomoraAttack(u8),
    /// The second fight's thunder: the smoke under a member's bolt
    /// (MissileSmoke row 17, ff 9), its life the thunder's time.
    ThunderSmoke(i16),
    /// The meteors' and the bolts' own (`MissileSmoke`, `BurstSmoke` rows),
    /// which piney-effect makes with their pictures.
    Trail(u8),
    Burst(u8),
    /// The third fight's magical ball's ([`magic_ball::BallGen`]).
    Ball(magic_ball::BallGen),
    /// The fourth fight's own ([`fourth::Make`]) and its hell's express's
    /// ([`hell::HellGen`]).
    Fourth(fourth::Make),
    Hell(hell::HellGen),
}

impl Gen {
    /// Its table, row and force fields.
    pub fn rows(self) -> GenRow {
        let g = |table, row, ff| GenRow { table, row, ff, ff_table: table, dist_sw: true };
        match self {
            Gen::Light => g(Table::Kyvia, 0, &[0]),
            Gen::BeamsOut => g(Table::Kyvia, 7, &[10, 11]),
            Gen::BeamHead => g(Table::Kyvia, 5, &[1]),
            Gen::BeamTail => g(Table::Kyvia, 5, &[8]),
            Gen::BeamBurst => g(Table::Kyvia, 3, &[12, 4]),
            Gen::Smoke => g(Table::KyviaDmg, 0, &[0, 1]),
            Gen::DeadSmoke(f) => GenRow { dist_sw: f == Fight::First, ..g(Table::KyviaDmg, 1, &[2, 3]) },
            Gen::CoreRise => g(Table::Core, 0, &[0]),
            Gen::CoreAura(k) => g(Table::Core, 7 + k, &[3]),
            Gen::CoreDeath(k) => g(Table::Core, 1 + k, &[1]),
            Gen::GomoraOut(0) => g(Table::Gomora, 5, &[6, 5]),
            Gen::GomoraOut(_) => g(Table::Gomora, 6, &[7, 5]),
            Gen::GomoraBlow => g(Table::Gomora, 0, &[10, 9]),
            Gen::GomoraDeath(0) => g(Table::Gomora, 1, &[0, 2, 4]),
            Gen::GomoraDeath(_) => g(Table::Gomora, 2, &[3, 4]),
            Gen::GomoraAura(attr, 0) => g(Table::GomoraAura, attr, &[0]),
            Gen::GomoraAura(attr, 1) => g(Table::GomoraAura, attr, &[1]),
            Gen::GomoraAura(attr, _) => g(Table::GomoraAura, attr, &[2]),
            Gen::GomoraAttack(attr) => g(Table::GomoraAura, 4 + attr, &[0]),
            Gen::ThunderSmoke(_) => GenRow { dist_sw: false, ..g(Table::MissileSmoke, 17, &[9]) },
            Gen::Trail(r) => GenRow { dist_sw: false, ..g(Table::MissileSmoke, r, &[]) },
            Gen::Burst(r) => GenRow { dist_sw: false, ..g(Table::BurstSmoke, r, &[]) },
            Gen::Ball(which) => {
                let (row, ff) = which.rows();
                GenRow { dist_sw: false, ..g(Table::MissileSmoke, row, ff) }
            }
            Gen::Fourth(m) => m.rows(),
            Gen::Hell(h) => {
                let (row, ff) = h.rows();
                GenRow { dist_sw: false, ..g(Table::MissileSmoke, row, ff) }
            }
        }
    }
}

/// What a generator follows (`syncPos`, `syncPosType` 0), each frame: a
/// character's place, or a member of the fight that moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    /// The character's `pos` (the core's and its auras').
    Char(usize),
    /// The body's `DedDmgVec` (+0x29540), the death's smoke.
    DeadSmoke,
    /// `LBpos[k]` (+0x294a0), `LBposSub[k]` (+0x294d0): a beam's ends.
    BeamHead(u8),
    BeamTail(u8),
    /// A gomora's `AuraPos` (+0x293f0), by its character.
    Aura(usize),
    /// The magical ball's `PlaPos[k]` and `PlaPos2[k]` (the boss's one
    /// ball).
    Plasma(u8),
    Plasma2(u8),
    /// Hell's express's `Pos[k]` (a soul's place; the boss's one).
    Soul(u8),
}

/// Where a generator stands: its `pos` (+0x60), or following a place with
/// its `offset` (+0x80).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    At(V4),
    Follows(Follow, V4),
    /// From `pos` to `pos2` (+0x70): the fourth fight's walls.
    Line(V4, V4),
}

/// A member the game keeps a generator's pointer in, to kill or stop it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Held {
    /// The body's +0x29500 (the light, then the last beam's burst).
    Light,
    /// +0x29504 and +0x29510: beam `k`'s head and tail.
    BeamHead(u8),
    BeamTail(u8),
    /// `DmgGp[k]` (+0x29550); 4 is `DeadGp` (+0x29560), which a fifth
    /// smoke reaches.
    Smoke(u8),
    /// The core's two auras (+0x29360, +0x29364), by its character.
    CoreAura(usize, u8),
    /// A gomora's aura (+0x293e0), by its character.
    GomoraAura(usize),
    /// The fourth fight's walls and the light at each place
    /// (`WallGp[k]`, `WallGp2[k]`, `CoreAppGp[k]`, `CoreAppGp2[k]`).
    Wall(u8),
    Wall2(u8),
    CoreApp(u8),
    CoreApp2(u8),
    /// Hell's express's souls' (`Sgp[k]`, `Kgp[k]`, `Pgp[k]`) and the
    /// last's (`Ogp`).
    Soul(u8, u8),
    LastSoul,
}

pub use super::GenOp;

/// `new ccParticleGenerator` and `startParticleGenerator`: one of the
/// fight's generators, where it stands, and the member that keeps it.
pub(crate) fn particles(cx: &mut Cx, which: Gen, at: Place, held: Option<Held>) {
    cx.out(Out::KyviaParticles { which, at, held });
}

/// What the fight does to a generator it keeps.
pub(crate) fn gen_op(cx: &mut Cx, held: Held, op: GenOp) {
    cx.out(Out::KyviaGen { held, op });
}

/// `ccBossKyvia01`'s own members (+0x29350 on), and `ccBossKyvia02`'s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kyvia {
    pub fight: Fight,
    /// `anmw`: `ANM_ex0batc0`, the arm `HandAtk` swings.
    pub anm_w: Anm,
    /// `DiscPos`: the disc (`discPrevPos`) as the last frame read it.
    pub disc_pos: V4,
    pub disc_lv: i16,
    pub disc_max_lv: i16,
    /// `dummypos`: `DMY_marker01`, where the body stands.
    pub dummy_pos: V4,
    /// `tesPOS`, `tesDIRC`: where the arm is drawn.
    pub tes_pos: V4,
    pub tes_dirc: V4,
    pub snd_pos_l: V4,
    pub snd_pos_r: V4,
    pub atk_pat_mode: i32,
    /// The third fight's +0x293f4: frames of the last stage's third
    /// attack turn (the player held at 30).
    pub pat_count: i32,
    pub l_offset: V4,
    pub l_point: [V4; 6],
    pub l_char: [V4; 3],
    pub l_bpos: [V4; 3],
    pub l_bpos_sub: [V4; 3],
    pub btime: F,
    pub add_time: F,
    pub sub_btime: F,
    pub max_hp: i32,
    pub switch_hp: i32,
    pub hp_proccess: i32,
    pub dmg_anm_flg: i32,
    pub ded_dmg_vec: V4,
    /// `DmgGp[k]` and `DeadGp` made.
    pub dmg_gp: [bool; 4],
    pub dead_gp: bool,
    pub cam_rot_x: F,
    pub move_transfer: V4,
    /// `Z`: the camera's turn about the body.
    pub z: F,
    pub atk_mat: M4,
    pub atk_vec: V4,
    pub quake_vector: V4,
    pub time_mode: i32,
    pub s_point: V4,
    /// `SlaveCore`: the core's character.
    pub core: usize,
    pub st_flg: u8,
    pub af_cou: i32,
    pub af_timing: i32,
    pub default_argb: u32,
    pub ex_argb: u32,
    pub wait_disc_count: i16,
    pub live_flg: u8,
    pub now_point: i16,
    pub max_point: i16,
    pub disc_ex_flg: u8,
    /// +0x295f8: the cinema is on (turned off with the player unlocked).
    pub cinema: u8,
    /// `bossCam` +0xe0, the pitch, as the body last set it.
    pub cam_pitch: F,
    /// The second fight's thunder ([`thunder::Thunder`]).
    pub thunder: Option<thunder::Thunder>,
    /// The fourth fight's own members ([`fourth::Fourth`]).
    pub fourth: Option<Box<fourth::Fourth>>,
    /// `bossTbl` row 15's `maxHP` as the fourth fight's rebirth wrote it.
    pub row15_hp: Option<i16>,
}

/// One of the fight's parts as the body's frame holds it: its `ccBoss`, its
/// own members and its character.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Part<T> {
    pub b: Box<Boss>,
    pub x: Box<T>,
    pub me: usize,
}

/// The core and its gomoras, out of their characters for the frame.
#[derive(Debug, Default)]
pub struct Tree {
    pub core: Option<Part<Core>>,
    pub gomoras: Vec<Part<Gomora>>,
}

/// `cx.me` set to `me` for `f`.
pub(crate) fn as_me<R>(cx: &mut Cx, me: usize, f: impl FnOnce(&mut Cx) -> R) -> R {
    let old = cx.me;
    cx.me = me;
    let r = f(cx);
    cx.me = old;
    r
}

pub(crate) fn dp(x: F) -> f64 {
    f64::from(f32::from_bits(x))
}

/// `dptofp`: a double rounded to the nearest float.
pub(crate) fn fp(d: f64) -> F {
    (d as f32).to_bits()
}

/// `fabs` of a float through `fptodp` and back: the float without its sign.
pub(crate) fn fabs(x: F) -> F {
    x & 0x7fff_ffff
}

/// `ccRandF(x)` without its sign.
pub(crate) fn rand_abs(cx: &mut Cx, x: F) -> F {
    fabs(rand_f(cx.cc, x))
}

/// An angle into -pi..pi as the game does it, once each way.
pub(crate) fn wrap(mut v: F) -> F {
    if !ee::le(v, PI) {
        v = ee::sub(v, TWO_PI);
    }
    if ee::lt(v, NEG_PI) {
        v = ee::add(v, TWO_PI);
    }
    v
}

fn kyvia_of(b: &mut Boss) -> Box<Kyvia> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Kyvia(k) => k,
        other => {
            b.class = other;
            Box::default()
        }
    }
}

impl Tree {
    /// The core and its gomoras taken out of their characters.
    pub fn take(cx: &mut Cx, core_me: usize) -> Tree {
        let mut t = Tree::default();
        let Some(mut cb) = cx.scene.chars.get_mut(core_me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())
        else {
            return t;
        };
        let Class::KyviaCore(cxs) = std::mem::replace(&mut cb.class, Class::Plain) else {
            if let Some(f) = cx.scene.chars[core_me].foe_state_mut() {
                f.boss = Some(cb);
            }
            return t;
        };
        for &g in &cxs.gomoras {
            let Some(mut gb) = cx.scene.chars.get_mut(g).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())
            else {
                continue;
            };
            match std::mem::replace(&mut gb.class, Class::Plain) {
                Class::Gomora(gx) => t.gomoras.push(Part { b: gb, x: gx, me: g }),
                other => {
                    gb.class = other;
                    if let Some(f) = cx.scene.chars[g].foe_state_mut() {
                        f.boss = Some(gb);
                    }
                }
            }
        }
        t.core = Some(Part { b: cb, x: cxs, me: core_me });
        t
    }

    /// Everything put back.
    pub fn put(self, cx: &mut Cx) {
        for mut g in self.gomoras {
            g.b.class = Class::Gomora(g.x);
            if let Some(f) = cx.scene.chars[g.me].foe_state_mut() {
                f.boss = Some(g.b);
            }
        }
        if let Some(mut c) = self.core {
            c.b.class = Class::KyviaCore(c.x);
            if let Some(f) = cx.scene.chars[c.me].foe_state_mut() {
                f.boss = Some(c.b);
            }
        }
    }
}

/// `ccChar::SetBaseParam(ccGetBossParam(row), &param)` (MUT gcmn
/// 0x00591090) with the character's own `param`.
pub(crate) fn set_base_param(cx: &mut Cx, who: usize, row: usize) {
    set_base_param_of(cx, who, row, who);
}

/// `SetBaseParam` with `owner`'s `param` (OUT gcmn 0x0058d160): `who`
/// points at that block from now on, and the row sets it (`real` the
/// row's stats, no buffs, the gauge empty), the base, HP and SP full, the
/// conditions cleared and alive.
pub(crate) fn set_base_param_of(cx: &mut Cx, who: usize, row: usize, owner: usize) {
    let Some(r) = cx.t.bosses.get(row).cloned() else { return };
    if owner == who {
        cx.scene.own_param(who);
    } else {
        cx.scene.share_param(who, owner);
    }
    if let Some(p) = cx.scene.param_mut(who) {
        *p = crate::chara::Param { real: r.elm, ..Default::default() };
    }
    let ch = &mut cx.scene.chars[who];
    let (hp, sp) = (r.max_hp, r.max_sp);
    if let Some(f) = ch.foe_state_mut() {
        f.row = r;
    }
    ch.max_hp = hp;
    ch.max_sp = sp;
    ch.hp = hp;
    ch.sp = sp;
    crate::affect::clear_conditions(ch);
    ch.cond[cond::DEAD] = 0;
}

/// A part's character: a boss of `row`, no condition, its affects its
/// class's, on no command list.
pub(crate) fn new_char(cx: &mut Cx, row: usize) -> usize {
    let r = cx.t.bosses.get(row).cloned().unwrap_or_default();
    let mut ch = crate::chara::Char::foe(r);
    ch.condition_num = -1;
    ch.affect.func = crate::chara::AffectFunc::Boss;
    cx.scene.add(ch, 3)
}

/// A `ccBoss` as its constructor (MUT gcmn 0x00471690) leaves it: exited,
/// not drawn, no body hit or cheat HP.
pub(crate) fn plain_boss() -> Boss {
    Boss {
        class: Class::Plain,
        wait_pat_num: -2,
        pat_num: 13,
        stage_eff_id: -1,
        exit: 1,
        transparency: ONE,
        set_transparency: ONE,
        anm: Anm::new(),
        anm_wave: Anm::new(),
        ..Boss::default()
    }
}

/// The head of `ccBoss::Main` as the core's and the gomoras' `Main`s have
/// it: `CalcReal`, the target and the centre, the party held while
/// `lockPlayer` (and the menu's forbid), then `stop`'s count.
pub(crate) fn head(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    b.calc_real(cx);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = super::dist(cx, pp, b.center_pos_p);
    b.center_dirc = get_dirc(pp, b.center_pos_p);
    if b.lock_player != 0 {
        for m in cx.party.members.into_iter().flatten() {
            if cx.valid(Some(m)) {
                cx.affect(m, 5, 32767);
            }
        }
        if b.reserve_forbid_menu == 1 && cx.env.menu_forbid == 0 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: true, chat_except: b.reserve_forbid_chat_except != 0 });
            b.reserve_forbid_menu = 0;
            b.reserve_forbid_chat_except = 0;
        }
    } else if b.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
        cx.out(Out::MenuForbid { on: false, chat_except: false });
        b.reserve_forbid_menu = 0;
        b.reserve_forbid_chat_except = 0;
    }
    if b.stop != 0 {
        b.stop_count += 1;
        if b.stop_count >= 61 {
            b.stop_count = 0;
            b.stop = 0;
        }
        b.move_spd = 0;
    } else {
        b.stop_count = 0;
    }
}

/// The affect the core and the gomoras share (MUT gcmn 0x004f99c0,
/// 0x004ff480): `ccBoss::Affect`'s kinds without its checks on the party.
/// True for a hit (kinds 1 and 3), whose resistance the core then shows.
pub(crate) fn part_affect(b: &mut Boss, cx: &mut Cx) -> bool {
    let me = cx.me;
    let (t, p0, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.person)
    };
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    match t {
        6 => b.stop = 0,
        5 => b.stop = 1,
        21 => {
            let v = mhp / 10;
            let ch = &mut cx.scene.chars[me];
            ch.hp = v;
            ch.max_hp = v;
            if let Some(p) = cx.scene.param_mut(me) {
                p.pp = -1;
                p.pp_count = 0;
            }
        }
        13 => {
            b.change_action(cx, 11, 2, true);
            cx.scene.chars[me].affect.ty = 0;
        }
        7 | 9 => {
            cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
            cx.scene.chars[me].hp = hp.wrapping_add(p0).min(mhp);
        }
        1 | 3 => {
            cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
            if p0 >= 0 {
                cx.out(Out::HitMark { by: person });
                let mut h = if b.cheat_hp != 0 { hp.wrapping_sub(p0 / 10).max(mhp / 2) } else { hp.wrapping_sub(p0) };
                if h <= 0 {
                    h = 0;
                    crate::affect::clear_conditions(&mut cx.scene.chars[me]);
                    b.change_action(cx, 14, 2, true);
                } else if t == 1 {
                    let n = if cx.env.count & 1 != 0 { 1 } else { 2 };
                    b.change_action(cx, n, 0, true);
                }
                cx.scene.chars[me].hp = h;
            }
            return true;
        }
        _ => {}
    }
    false
}

// --- the constructor ---------------------------------------------------------------------

/// `ccBossKyvia01::ccBossKyvia01(1)` (MUT gcmn 0x004d2a80) or
/// `ccBossKyvia02::ccBossKyvia02(2)` (OUT gcmn 0x004d5940) with its core
/// and the core's five gomoras (`kyviaCore::CoreInit(lv)`,
/// `GomoraInit(lv)`), whose characters it adds to the scene with their
/// bosses. The body is `scene.chars[cx.me]` (`bossTbl` row 12 or 13); it
/// stands at `DMY_marker01` of the disc's file (`cx.disc.marker`).
pub fn new(cx: &mut Cx, fight: Fight) -> Boss {
    let me = cx.me;
    let mut b = plain_boss();
    let mut x = Kyvia {
        fight,
        st_flg: 1,
        time_mode: 1,
        now_point: 1,
        max_point: 1,
        live_flg: 1,
        atk_pat_mode: 1,
        disc_lv: 1,
        disc_max_lv: fight.stages(),
        disc_ex_flg: 1,
        af_timing: 2,
        anm_w: Anm::new(),
        atk_mat: geom::unit_matrix(),
        ..Kyvia::default()
    };
    set_base_param(cx, me, fight.row());
    let (anims, clip) = match fight {
        Fight::First => (&cx.data.kyvia.anims, "ANM_ex01nut0"),
        Fight::Second => (&cx.data.kyvia.anims_second, "ANM_ex02nut0"),
        Fight::Third => (&cx.data.kyvia.anims_third, "ANM_ex03nut0"),
        Fight::Fourth => (&cx.data.kyvia.anims_fourth, "ANM_ex04nut0"),
    };
    b.anm_tbl = anims.clone();
    // OffExit, OnDraw, OnBodyHit (not the fourth's), OnCheatHP.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = i8::from(fight != Fight::Fourth);
    b.cheat_hp = 1;
    b.dirc = VF0;
    b.anm.set(clip, cx.clips);
    b.change_action(cx, act::NEUTRAL, 0, true);
    x.dummy_pos = cx.disc.marker;
    // The fourth fight's body stands at the field's origin.
    cx.scene.chars[me].pos = if fight == Fight::Fourth { VF0 } else { x.dummy_pos };
    // InitBossCamera(450, 1050) (the fourth's 250, 700); the pitch (0.26,
    // the third's 0.25, the fourth's 0.13), its limit 0.6 (none in the
    // fourth), the sway.
    x.cam_rot_x = match fight {
        Fight::Third => 0x3e80_0000,
        Fight::Fourth => 0x3e05_1eb8,
        _ => 0x3e85_1eb8,
    };
    x.cam_pitch = x.cam_rot_x;
    cx.out(Out::CamPitch { add: false, v: x.cam_pitch });
    if fight != Fight::Fourth {
        cx.out(Out::CamRotXLimit(0x3f19_999a));
    }
    cx.out(Out::CamSway(true));
    x.anm_w.set("ANM_ex0batc0", cx.clips);
    // InitStageEffect: the stage fader.
    b.use_stage_eff = true;
    x.default_argb = 0x3080_8080;
    x.ex_argb = match fight {
        Fight::First => 0x6080_8080,
        _ => 0x7080_8080,
    };
    for s in [Set::Enable, Set::Scale(ONE), Set::Abgr(x.default_argb)] {
        cx.out(Out::Blur(s));
    }
    // The core at the fight's level.
    let lv = fight.level();
    let core_me = new_char(cx, 32 + 2 * (lv as usize - 1));
    x.core = core_me;
    let cb = self::core::new(cx, core_me, me, lv);
    if let Some(f) = cx.scene.chars[core_me].foe_state_mut() {
        f.boss = Some(Box::new(cb));
    }
    x.max_hp = i32::from(cx.scene.chars[core_me].max_hp);
    x.switch_hp = match fight {
        Fight::First => x.max_hp / 4,
        _ => x.max_hp / 5,
    };
    x.hp_proccess = 0;
    x.thunder = fight.later().then(thunder::Thunder::default);
    x.fourth = (fight == Fight::Fourth).then(|| Box::new(fourth::Fourth::new()));
    x.quake_vector = [0x4120_0000, 0x4120_0000, 0x4120_0000, 0];
    b.class = Class::Kyvia(Box::new(x));
    b
}

// --- the frame ---------------------------------------------------------------------------

/// `ccThBossEffect`'s pass over the fight's effects ([`manager_pass`]),
/// which the caller runs before the blur's `Main` and [`main`].
pub(super) fn effects_pass(b: &mut Boss, cx: &mut Cx) {
    let mut x = kyvia_of(b);
    let mut t = Tree::take(cx, x.core);
    manager_pass(b, &mut x, &mut t, cx);
    t.put(cx);
    b.class = Class::Kyvia(x);
}

/// `ccBossKyvia01::Main` (MUT gcmn 0x004d4410).
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = kyvia_of(b);
    let mut t = Tree::take(cx, x.core);
    if b.exit == 0 {
        frame(b, &mut x, &mut t, cx);
    }
    t.put(cx);
    b.class = Class::Kyvia(x);
}

fn frame(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    b.calc_real(cx);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = super::dist(cx, pp, b.center_pos_p);
    b.center_dirc = get_dirc(pp, b.center_pos_p);
    if b.stop != 0 {
        b.stop_count += 1;
        if b.stop_count >= 61 {
            b.stop_count = 0;
            b.stop = 0;
        }
        b.move_spd = 0;
    } else {
        b.stop_count = 0;
    }
    think(b, x, t, cx);
    action(b, x, t, cx);
    if x.fight == Fight::Fourth {
        fourth::move_body(b, cx);
    } else {
        b.move_(cx);
    }
    if b.lock_player != 0 {
        for m in cx.party.members.into_iter().flatten() {
            if cx.valid(Some(m)) {
                cx.affect(m, 5, 32767);
            }
        }
        if b.reserve_forbid_menu == 1 && cx.env.menu_forbid == 0 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: true, chat_except: b.reserve_forbid_chat_except != 0 });
            b.reserve_forbid_menu = 0;
            b.reserve_forbid_chat_except = 0;
        }
    } else if b.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
        cx.out(Out::MenuForbid { on: false, chat_except: false });
        b.reserve_forbid_menu = 0;
        b.reserve_forbid_chat_except = 0;
        if x.cinema != 0 {
            cx.out(Out::Cinema(None));
            x.cinema = 0;
        }
    }
    if b.draw_sw != 0 {
        b.anm_status = i8::from(b.anm.forward());
    }
    // Slave(): the core's Main.
    if let Some(c) = t.core.as_mut() {
        self::core::main(c, &mut t.gomoras, cx);
    }
    // The fourth fight's `Main` clears the body's hold after.
    if x.fight == Fight::Fourth {
        cx.scene.chars[me].cond[cond::HOLD] = 0;
    }
}

/// `ccBossKyvia01::Think` (MUT gcmn 0x004d47c0).
fn think(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    if x.fight == Fight::Fourth {
        fourth::think_head(b, x, t, cx);
        fourth::think_turn(b, x, t, cx);
        return;
    }
    x.af_cou += 1;
    if x.af_timing < x.af_cou {
        x.af_cou = 0;
    }
    check_disc_move(b, x, t, cx);
    match b.act_num {
        act::NEUTRAL => switch_action_pattern(b, x, t, cx),
        1..=5 | 11..=14 => {}
        act::THUNDER if x.fight.later() => {}
        20 | 21 if x.fight == Fight::Third => {}
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `SwitchActionPattern` (MUT gcmn 0x004d48a0, OUT 0x004d7790, QUA
/// 0x003d29e0): the core's attack turn (`kAtkFlg` 2, in its state 0)
/// begins the body's next attack in turn. The second fight's first stage
/// has the thunder for the beams and the meteors; its second the four in
/// turn from the beams. The third's are by stage ([`third_pattern`]).
fn switch_action_pattern(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    switch_dead_check(b, x, t, cx);
    let Some(c) = t.core.as_mut() else { return };
    if c.x.k_atk_flg != 2 {
        return;
    }
    if x.fight == Fight::Third && x.disc_lv == x.disc_max_lv && x.atk_pat_mode == 3 {
        // The last stage's meteors wait for the menu, the player held
        // from its 30th frame (the core's flag kept).
        let n = x.pat_count;
        x.pat_count += 1;
        if n == 30 {
            b.lock_player(cx, false);
        }
        if cx.env.menu_type != -1 {
            return;
        }
    }
    if c.x.core_state != 0 {
        return;
    }
    x.cinema = 1;
    let first_stage = x.disc_lv != x.disc_max_lv;
    // (cinema, act, the next AtkPatMode, ThunderType)
    let next = match (x.fight, x.atk_pat_mode) {
        (Fight::First, 1) => Some((33, act::HAND, 2, None)),
        (Fight::First, 2) => Some((32, act::LIGHT, 3, None)),
        (Fight::First, 3) => Some((34, act::MEGID, 1, None)),
        (Fight::Second, 1) => Some((36, act::HAND, 2, None)),
        (Fight::Second, 2) if first_stage => Some((46, act::THUNDER, 3, Some(thunder::Kind::Strike))),
        (Fight::Second, 2) => Some((37, act::LIGHT, 3, None)),
        (Fight::Second, 3) if first_stage => Some((46, act::THUNDER, 1, Some(thunder::Kind::Wide))),
        (Fight::Second, 3) => Some((38, act::MEGID, 4, None)),
        (Fight::Second, 4) => Some((46, act::THUNDER, 2, Some(thunder::Kind::Storm))),
        (Fight::Third, mode) => third_pattern(x, cx, mode),
        _ => None,
    };
    if let Some((cinema, n, mode, kind)) = next {
        cx.out(Out::Cinema(Some(cinema)));
        b.change_action(cx, n, 0, true);
        if let (Some(k), Some(th)) = (kind, x.thunder.as_mut()) {
            th.kind = k;
        }
        x.atk_pat_mode = mode;
    }
    c.x.k_atk_flg = 0;
}

/// The third fight's turn (QUA gcmn 0x003d2af0): by `AtkPatMode` and the
/// stage, the arm (cinema 36), the thunder (48, of kind 1 or 2), the
/// beams (37) or the meteors (53), then the next mode; the last stage's
/// meteors lead to the magical ball (41) while the core is below 70% of
/// `MaxHP`, and the ball back to the second mode.
fn third_pattern(x: &mut Kyvia, cx: &Cx, mode: i32) -> Option<(i32, i16, i32, Option<thunder::Kind>)> {
    use thunder::Kind;
    let next = match mode {
        3 => 1,
        4 => 2,
        _ => mode + 1,
    };
    let (cinema, n, kind) = match (mode, x.disc_lv) {
        (1, 1 | 2) | (2, 1) => (36, act::HAND, None),
        (1, 3) | (2, 2) | (3, 1) => (48, act::THUNDER, Some(Kind::Wide)),
        (2, 3) => (37, act::LIGHT, None),
        (3, 2) => (48, act::THUNDER, Some(Kind::Storm)),
        (3, 3) => {
            x.pat_count = 0;
            // `(double)hp < MaxHP * 0.7`.
            let hp = f64::from(cx.scene.chars[x.core].hp);
            let low = hp < f64::from(x.max_hp) * 0.7;
            return Some((53, act::MEGID, if low { 4 } else { 1 }, None));
        }
        (4, _) => (41, act::MAGIC_BALL, None),
        _ => return None,
    };
    Some((cinema, n, next, kind))
}

/// `SwitchDeadCheck` (MUT gcmn 0x004d4a80): the core's flags read into the
/// body's acts, and the body's death once the core is gone.
pub(super) fn switch_dead_check(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let flg = t.core.as_ref().map_or(0, |c| c.x.k_atk_flg);
    match flg {
        2 => {
            check_dmg_smoke_eff(x, t, cx);
        }
        1 => {
            b.change_action(cx, act::DAMAGE, 0, true);
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 3;
            }
        }
        3 => {
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 2;
            }
        }
        _ => {
            if t.core.as_ref().is_some_and(|c| c.x.k_dmg_flg == 4) {
                // `actNum != 2 || actNum != 1`: always.
                b.change_action(cx, act::DAMAGE2, 0, true);
                if let Some(c) = t.core.as_mut() {
                    c.x.k_dmg_flg = 0;
                }
            }
        }
    }
    // The fourth fight's death is its `DeadCheck`'s.
    if x.fight != Fight::Fourth && x.live_flg == 0 && !cx.annihilated() && !cx.game_over {
        cx.out(Out::ClearSpcCondition);
        b.change_action(cx, act::DEAD, 2, true);
    }
}

/// `CheckDiscMove` (MUT gcmn 0x004d4c10, OUT 0x004d7ca0): the disc's ride
/// (the camera on the body), its stop and the core's entry; after the
/// core's death the next stage, or the body's end. The second fight's
/// sounds are at the disc, the first's at the body.
fn check_disc_move(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    x.disc_pos = cx.disc.prev_pos;
    let me = cx.me;
    let at = if x.fight.later() { x.disc_pos } else { cx.scene.chars[me].pos };
    if x.st_flg == 1 {
        if !cx.disc.moving {
            if x.wait_disc_count == 0 {
                for s in [Set::Enable, Set::Scale(0x3f86_6666), Set::Abgr(x.ex_argb)] {
                    cx.out(Out::Blur(s));
                }
                x.quake_vector = [0x41a0_0000, 0x41a0_0000, 0x41a0_0000, x.quake_vector[3]];
                cx.out(Out::Se3dNote { se: 56, pos: at, note: 60 });
                cx.out(Out::CamMode { mode: 6 });
                cx.out(Out::CamInitLock);
                x.disc_ex_flg = 1;
            }
            x.wait_disc_count += 1;
            if x.wait_disc_count < 21 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
                cx.out(Out::Blur(Set::Scale(0x3f83_d70a)));
            }
            if x.wait_disc_count == 21 {
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            }
            if x.wait_disc_count == 35 {
                cx.out(Out::Blur(Set::Scale(0x3f81_47ae)));
            }
            if x.wait_disc_count == 50 {
                x.st_flg = 0;
                x.wait_disc_count = 0;
                entry_slave(x, t, cx, 1);
                cx.out(Out::Blur(Set::Scale(ONE)));
                if x.fight.later() {
                    // The core's MaxHP again, a smoke for each seventh
                    // (the third fight's fifth).
                    let core = t.core.as_ref().map_or(me, |c| c.me);
                    x.max_hp = i32::from(cx.scene.chars[core].max_hp);
                    x.switch_hp = x.max_hp / if x.fight == Fight::Third { 5 } else { 7 };
                }
            }
            return;
        }
        ride_camera(x, cx);
        if b.act_num == act::NEUTRAL && matches!(b.anm.frame(), 20 | 60) {
            cx.out(Out::Se3d { se: 232, pos: at });
        }
        return;
    }
    if !t.core.as_ref().is_some_and(|c| c.x.dead_flg != 0) {
        return;
    }
    if x.disc_lv < x.disc_max_lv {
        next_stage(b, x, cx);
    } else if x.live_flg != 0 {
        x.live_flg = 0;
    }
}

/// The camera on the ride: mode 5, 1500 off the disc turned toward the
/// body and 500 up, looking 150 over the disc (the second fight's at the
/// disc itself: it passes the disc's place for the view; the third's as
/// the first's).
fn ride_camera(x: &mut Kyvia, cx: &mut Cx) {
    if x.disc_ex_flg != 0 {
        cx.out(Out::CamMode { mode: 5 });
        x.disc_ex_flg = 0;
    }
    let d = get_dirc(x.disc_pos, cx.scene.chars[cx.me].pos);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), d);
    let mut eye = geom::vadd(geom::apply_matrix(&m, [0, 0x44bb_8000, 0x43fa_0000, ONE]), x.disc_pos);
    let view = match x.fight {
        Fight::First | Fight::Third | Fight::Fourth => geom::vadd([0, 0, 0x4316_0000, ONE], x.disc_pos),
        Fight::Second => {
            eye[3] = ONE;
            x.disc_pos
        }
    };
    cx.out(Out::FreeCam { pos: eye, view });
}

/// `CheckDiscMove` after a core's death with a stage to come (OUT gcmn
/// 0x004d80cc; the first fight has none): the quake and the camera on the
/// ride, then the disc on (`EVENTAREAB8::Move`) at 60 with the attacks
/// from the first again.
fn next_stage(b: &mut Boss, x: &mut Kyvia, cx: &mut Cx) {
    x.wait_disc_count += 1;
    match x.wait_disc_count {
        1 => {
            x.quake_vector = [0x41a0_0000, 0x41a0_0000, 0x41a0_0000, x.quake_vector[3]];
            cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
            b.lock_player(cx, false);
        }
        20 => ride_camera(x, cx),
        40 => cx.out(Out::Blur(Set::Abgr(0x5080_8080))),
        60 => {
            b.unlock_player();
            x.st_flg = 1;
            x.disc_lv += 1;
            x.wait_disc_count = 0;
            cx.out(Out::DiscNextStage);
            cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            cx.out(Out::Se3dNote { se: 56, pos: x.disc_pos, note: 60 });
            x.atk_pat_mode = 1;
        }
        n if n >= 41 => {
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            if n % 6 == 0 {
                cx.out(Out::Se3dNote { se: 56, pos: x.disc_pos, note: 60 });
            }
        }
        _ => {}
    }
}

/// `CheckDmgSmokeEff` (MUT gcmn 0x004d5140, OUT 0x004d8410): each part of
/// the core's HP lost (`SwitchHp`) puts a smoke on the body, 500 up and up
/// to 300 off (the second fight's 1000 up and 300 back, and only on the
/// last stage); -1 when none is due, else the next part.
pub(super) fn check_dmg_smoke_eff(x: &mut Kyvia, t: &Tree, cx: &mut Cx) -> i32 {
    let second = x.fight.later();
    if second && x.disc_lv != x.disc_max_lv {
        return -1;
    }
    // The fourth fight's smokes come after the rebirth.
    let fourth = x.fight == Fight::Fourth;
    if fourth && x.fourth.as_ref().is_none_or(|f| f.rebirth_flg == 0) {
        return -1;
    }
    let core_hp = t.core.as_ref().map_or(0, |c| i32::from(cx.scene.chars[c.me].hp));
    let k = x.hp_proccess;
    if core_hp > x.max_hp - x.switch_hp * (k + 1) {
        return -1;
    }
    if x.smoke_made(k) {
        return -1;
    }
    let mut v = VF0;
    let r = rand_f(cx.cc, 0x4396_0000);
    v[1] = fabs(r) ^ 0x8000_0000;
    let turn = rand_abs(cx, PI);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
    v = geom::apply_matrix(&m, v);
    let up = match x.fight {
        Fight::First => 0x43fa_0000,
        Fight::Fourth => 0x4496_0000,
        _ => 0x447a_0000,
    };
    v[2] = ee::add(v[2], up);
    let mut pos = geom::vadd(v, cx.scene.chars[cx.me].pos);
    if second {
        let back = if fourth { 0xc489_8000 } else { 0xc396_0000 };
        pos = geom::vadd(pos, [0, back, 0, ONE]);
    }
    x.set_smoke(k);
    let which = if fourth { Gen::Fourth(fourth::Make::Smoke) } else { Gen::Smoke };
    particles(cx, which, Place::At(pos), Some(Held::Smoke(k as u8)));
    x.hp_proccess += 1;
    x.hp_proccess
}

impl Kyvia {
    /// `DmgGp[k]` set. Past the four it reads on: `DeadGp`, then `CamRotX`
    /// (0.26, never 0), so a fifth smoke is the last (the second fight's
    /// sevenths reach it).
    fn smoke_made(&self, k: i32) -> bool {
        match k {
            0..=3 => self.dmg_gp[k as usize],
            4 => self.dead_gp,
            _ => true,
        }
    }

    fn set_smoke(&mut self, k: i32) {
        match k {
            0..=3 => self.dmg_gp[k as usize] = true,
            4 => self.dead_gp = true,
            _ => {}
        }
    }
}

/// `EntrySlave(0, on)` (MUT gcmn 0x004d92b0): the first exited core, out,
/// ordered, at the disc: its slave number, or -1.
fn entry_slave(x: &mut Kyvia, t: &mut Tree, cx: &mut Cx, on: i32) -> i32 {
    let Some(c) = t.core.as_mut() else { return -1 };
    if c.b.exit == 0 {
        return -1;
    }
    c.b.exit = 0;
    c.x.order_num = on;
    c.x.disc_pos = x.disc_pos;
    let _ = cx;
    c.x.slave_id
}

// --- Action ------------------------------------------------------------------------------

/// `ccBossKyvia01::Action` (MUT gcmn 0x004d3330).
fn action(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    match b.act_num {
        act::NEUTRAL | 11..=13 => {}
        act::DAMAGE => {
            match b.act_proccess {
                0 => {
                    b.lock_player(cx, false);
                    if check_dmg_smoke_eff(x, t, cx) == -1 {
                        b.act_proccess = 50;
                    } else {
                        b.act_proccess = 1;
                        x.dmg_anm_flg = 1;
                        x.quake_vector = [0x4120_0000, 0x4120_0000, 0x4120_0000, x.quake_vector[3]];
                        cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
                        let (back, up, eye) = match x.fight {
                            Fight::First => (0x43af_0000, 0x4422_8000, 0xc448_0000),
                            Fight::Second | Fight::Third => (0x4422_8000, 0x4489_8000, 0xc461_0000),
                            Fight::Fourth => (0x4483_4000, 0x44c8_0000, 0xc461_0000),
                        };
                        damage_camera(x, cx, [back, up, eye], None);
                        set_pitch(x, cx, 0);
                    }
                    let pos = cx.scene.chars[me].pos;
                    cx.out(Out::Se3dNote { se: 191, pos, note: 72 });
                }
                1 => {
                    b.act_proccess = 2;
                    b.act_count = 0;
                }
                2 => {
                    if b.anm_status != 0 {
                        b.act_proccess = 50;
                        cx.out(Out::Blur(Set::Abgr(x.default_argb)));
                        cx.out(Out::CamMode { mode: 6 });
                        set_pitch(x, cx, x.cam_rot_x);
                    }
                }
                50 if b.anm_status != 0 => {
                    set_pitch(x, cx, x.cam_rot_x);
                    b.change_action(cx, act::NEUTRAL, 0, true);
                    x.dmg_anm_flg = 0;
                    b.unlock_player();
                }
                _ => {}
            }
            if x.dmg_anm_flg != 0 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            b.spd_down(0, 0x40a0_0000);
        }
        act::DAMAGE2 => {
            if b.act_proccess == 0 {
                let n = 65 + cx.cc.rand() % 5;
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 191, pos, note: n as u8 });
                b.act_proccess += 1;
                let ch = &mut cx.scene.chars[me];
                ch.affect.person = Some(me);
                ch.affect.ty = 1;
                ch.affect.color_cnt = -10;
                ch.affect.color_rate = 70;
                ch.affect.color = 255;
            }
            cx.scene.chars[me].affect.color_cnt = -10;
            if b.anm_status != 0 {
                let ch = &mut cx.scene.chars[me];
                ch.affect.color_fix = 0;
                ch.affect.color = 0;
                ch.affect.color_rate = 0;
                b.change_action(cx, act::NEUTRAL, 0, true);
                // The EX turn's pattern reads the core's hit itself.
                if !fourth::ex(x)
                    && let Some(c) = t.core.as_mut()
                    && c.x.k_dmg_flg == 4
                {
                    c.x.k_dmg_flg = 0;
                }
            }
            b.spd_down(0, 0x40a0_0000);
        }
        act::HAND => hand_atk(b, x, t, cx),
        act::LIGHT => {
            light_atk(b, x, t, cx);
            b.spd_down(0, 0x40a0_0000);
        }
        act::MEGID => {
            megid_flame(b, x, t, cx);
            b.spd_down(0, 0x40a0_0000);
        }
        act::THUNDER if x.fight.later() => thunder::thunderbolt_atk(b, x, t, cx),
        act::MAGIC_BALL if x.fight == Fight::Third => magic_ball::magic_ball_atk(b, x, t, cx),
        act::MAGIC_BALL if x.fight == Fight::Fourth => fourth::magic_ball_atk(b, x, t, cx),
        21 if x.fight == Fight::Third => {}
        act::HELL if x.fight == Fight::Fourth => fourth::hell_e_atk(b, x, t, cx),
        act::BIG_MEGID if x.fight == Fight::Fourth => fourth::big_megid(b, x, t, cx),
        act::SKILL if x.fight == Fight::Fourth => fourth::skill_atk(b, x, t, cx),
        act::REBIRTH if x.fight == Fight::Fourth => fourth::rebirth(b, x, t, cx),
        act::DEAD if x.fight == Fight::Fourth => fourth::dead(b, x, t, cx),
        act::DEAD => dead(b, x, cx),
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// The pitch (`bossCam` +0xe0) set.
pub(super) fn set_pitch(x: &mut Kyvia, cx: &mut Cx, v: F) {
    x.cam_pitch = v;
    cx.out(Out::CamPitch { add: false, v });
}

/// The free camera round the body the hit and death use: mode 5, the point
/// looked at `back` behind and `up` above the body's place (`LOffset`),
/// the eye `Atk_vec` (0, `eye`, 0) turned about z by `Z` (a fresh
/// `ccRandF(1)`, or `z` given) and added to it.
fn damage_camera(x: &mut Kyvia, cx: &mut Cx, [back, up, eye]: [F; 3], z: Option<F>) {
    let me = cx.me;
    x.l_offset = VF0;
    x.move_transfer = VF0;
    x.l_offset = cx.scene.chars[me].pos;
    x.atk_vec = VF0;
    x.atk_mat = geom::unit_matrix();
    x.l_offset[1] = ee::sub(x.l_offset[1], back);
    x.l_offset[2] = ee::add(x.l_offset[2], up);
    x.atk_vec[1] = eye;
    cx.out(Out::CamMode { mode: 5 });
    x.z = match z {
        Some(z) => z,
        None => rand_f(cx.cc, ONE),
    };
    x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
    x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
    x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
    cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
}

/// Act 14 (MUT gcmn 0x004d3898): the body's death, the voices and the
/// camera round it, then the task's exit.
fn dead(b: &mut Boss, x: &mut Kyvia, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            cx.out(Out::Cinema(Some(-1)));
            b.lock_player(cx, false);
            b.act_proccess += 1;
            let (up, back, x0) = match x.fight {
                Fight::First => (0x443b_8000, 0x4391_0000, 0x4120_0000),
                Fight::Second => (0x4496_0000, 0x441d_8000, 0xc248_0000),
                Fight::Third | Fight::Fourth => (0x449c_4000, 0x4434_0000, 0),
            };
            x.ded_dmg_vec = cx.scene.chars[me].pos;
            x.ded_dmg_vec[2] = ee::add(x.ded_dmg_vec[2], up);
            x.ded_dmg_vec[1] = ee::sub(x.ded_dmg_vec[1], back);
            x.ded_dmg_vec[0] = x0;
            x.dead_gp = true;
            particles(cx, Gen::DeadSmoke(x.fight), Place::Follows(Follow::DeadSmoke, VF0), Some(Held::Smoke(4)));
            b.act_count = 0;
            let pos = cx.scene.chars[me].pos;
            cx.out(Out::Se3dNote { se: 192, pos, note: 48 });
            cx.out(Out::MusicFade { t: 30 });
            if x.fight == Fight::Second {
                x.quake_vector = [0x41f0_0000, 0x41f0_0000, 0x41a0_0000, x.quake_vector[3]];
                cx.out(Out::Blur(Set::Scale(0x3f83_d70a)));
            }
        }
        1 => {
            // `actCount++ == 60`: the smokes killed (their pointers kept).
            if b.act_count == 60 {
                for k in (0..4u8).filter(|&k| x.dmg_gp[usize::from(k)]) {
                    gen_op(cx, Held::Smoke(k), GenOp::Kill);
                }
            }
            b.act_count += 1;
            let later = x.fight.later();
            if later && b.act_count < 11 {
                x.ded_dmg_vec[0] = ee::add(x.ded_dmg_vec[0], 0x40a0_0000);
            }
            if b.act_count >= 135 {
                x.ded_dmg_vec[0] = ee::add(x.ded_dmg_vec[0], ONE);
                if later {
                    x.ded_dmg_vec[1] = ee::sub(x.ded_dmg_vec[1], HALF);
                }
                match x.fight {
                    Fight::First => x.ded_dmg_vec[2] = ee::sub(x.ded_dmg_vec[2], 0x3e19_999a),
                    Fight::Second => {}
                    Fight::Third | Fight::Fourth => x.ded_dmg_vec[2] = ee::sub(x.ded_dmg_vec[2], 0x3f0c_cccd),
                }
            }
            if ee::lt(ONE, ee::mul(0x42c8_0000, x.cam_pitch)) {
                let v = ee::sub(x.cam_pitch, ee::div(x.cam_pitch, 0x41a0_0000));
                set_pitch(x, cx, v);
            }
            dead_voice(b, x.fight, cx);
            dead_cam(b, x, cx);
            x.ded_dmg_vec[2] = ee::sub(x.ded_dmg_vec[2], ONE);
            let rise = match x.fight {
                Fight::First => 0x4000_0000,
                Fight::Second => 0x4080_0000,
                Fight::Third | Fight::Fourth => 0x4040_0000,
            };
            x.ded_dmg_vec[1] = ee::add(x.ded_dmg_vec[1], rise);
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            if b.anm_status != 0 {
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        2 => {
            if x.dead_gp {
                gen_op(cx, Held::Smoke(4), GenOp::Kill);
                gen_op(cx, Held::Smoke(4), GenOp::Forget);
            }
            x.dead_gp = false;
            b.act_proccess += 1;
        }
        3 => {
            b.exit = 1;
            b.unlock_player();
            cx.out(Out::Cinema(None));
        }
        _ => {}
    }
}

/// `DeadKyviaVoice` (MUT gcmn 0x004d3cf0): the body's cries by `actCount`,
/// at the body (the third fight's not placed, QUA 0x003d1df0).
pub(super) fn dead_voice(b: &Boss, fight: Fight, cx: &mut Cx) {
    const CRIES: [(i16, i32, u8); 19] = [
        (30, 192, 60),
        (50, 192, 48),
        (70, 191, 72),
        (90, 191, 69),
        (100, 192, 69),
        (120, 191, 69),
        (130, 192, 58),
        (150, 192, 48),
        (170, 191, 72),
        (190, 191, 69),
        (200, 192, 65),
        (220, 191, 72),
        (230, 192, 61),
        (250, 192, 48),
        (270, 191, 72),
        (290, 191, 69),
        (300, 192, 72),
        (320, 191, 69),
        (330, 192, 72),
    ];
    // The fourth fight's (QUA gcmn 0x003dadb0), not placed either.
    const CRIES_FOURTH: [(i16, i32, u8); 20] = [
        (30, 192, 60),
        (50, 192, 48),
        (70, 191, 72),
        (90, 191, 69),
        (100, 192, 69),
        (100, 226, 63),
        (120, 191, 69),
        (120, 246, 49),
        (140, 246, 45),
        (140, 226, 63),
        (190, 246, 53),
        (190, 226, 63),
        (210, 246, 55),
        (210, 226, 63),
        (250, 246, 49),
        (250, 226, 63),
        (290, 246, 55),
        (290, 226, 63),
        (320, 246, 49),
        (320, 226, 63),
    ];
    let pos = cx.scene.chars[cx.me].pos;
    if fight == Fight::Fourth {
        for (at, se, note) in CRIES_FOURTH {
            if b.act_count == at {
                cx.out(Out::SeNote { se, note });
            }
        }
        return;
    }
    for (at, se, note) in CRIES {
        if b.act_count == at {
            cx.out(if fight == Fight::Third { Out::SeNote { se, note } } else { Out::Se3dNote { se, pos, note } });
        }
    }
}

/// `DeadKyviaCam` (MUT gcmn 0x004d3fc0, OUT 0x004d6eb0, QUA 0x003d2080):
/// the death's cameras at 70 (the third fight's 60) and 150, then the eye
/// rising 4 a frame.
pub(super) fn dead_cam(b: &Boss, x: &mut Kyvia, cx: &mut Cx) {
    let me = cx.me;
    let second = x.fight.later();
    // The third fight's first camera at 60, 650 back (the fourth's 1050
    // back, 1600 up, its eye 1200 off).
    let fourth = x.fight == Fight::Fourth;
    let (first_at, back) = match x.fight {
        Fight::Third => (60, 0x4422_8000),
        Fight::Fourth => (60, 0x4483_4000),
        _ => (70, 0x43af_0000),
    };
    if b.act_count == first_at {
        if second {
            cx.out(Out::Blur(Set::Abgr(0x5080_8080)));
            cx.out(Out::Blur(Set::Scale(0x3f82_8f5c)));
            let z = rand_abs(cx, ONE);
            let (up, eye) = if fourth { (0x44c8_0000, 0xc496_0000) } else { (0x447a_0000, 0xc461_0000) };
            damage_camera(x, cx, [back, up, eye], Some(z));
        } else {
            cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
            let z = fabs(rand_f(cx.cc, ONE)) ^ 0x8000_0000;
            damage_camera(x, cx, [0x43af_0000, 0x4409_8000, 0xc448_0000], Some(z));
        }
    }
    if b.act_count == 150 {
        let (up, back, rise) =
            if second { (0x43fa_0000, 0x452f_0000, 0x447a_0000) } else { (0x4316_0000, 0x44bb_8000, 0x4348_0000) };
        x.l_offset = VF0;
        x.move_transfer = VF0;
        x.l_offset = cx.scene.chars[me].pos;
        x.atk_vec = VF0;
        x.atk_mat = geom::unit_matrix();
        let (back2, up) = if fourth { (0x43fa_0000, 0x4448_0000) } else { (0x42c8_0000, up) };
        x.l_offset[1] = ee::sub(x.l_offset[1], back2);
        x.l_offset[2] = ee::add(x.l_offset[2], up);
        x.atk_vec[1] = ee::sub(x.atk_vec[1], back);
        x.atk_vec[2] = ee::add(x.atk_vec[2], rise);
        x.z = if second {
            fabs(rand_f(cx.cc, ONE)) ^ 0x8000_0000
        } else {
            let r = rand_abs(cx, HALF);
            fp(0.2 + dp(r))
        };
        x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
        x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
        x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
        cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
    }
    if b.act_count >= 161 {
        x.atk_vec[2] = ee::add(x.atk_vec[2], if fourth { 0x4100_0000 } else { 0x4080_0000 });
        cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
    }
}

const HALF: F = 0x3f00_0000;

// --- the attacks -------------------------------------------------------------------------

/// The core's `SetCoreState(n)` from the body.
fn core_state(t: &mut Tree, cx: &mut Cx, n: i32) {
    if let Some(c) = t.core.as_mut() {
        self::core::set_core_state(c, &mut t.gomoras, cx, n);
    }
}

/// The core ready for the body's attack: in its state 0 with it and its
/// gomoras at rest (`GetActNum`).
fn core_ready(t: &Tree) -> bool {
    t.core.as_ref().is_some_and(|c| c.x.core_state == 0 && self::core::get_act_num(c, &t.gomoras))
}

fn core_state_is(t: &Tree, n: i32) -> bool {
    t.core.as_ref().is_some_and(|c| c.x.core_state == n)
}

fn core_fade_check(t: &Tree) -> bool {
    t.core.as_ref().is_some_and(|c| self::core::fade_check(c, &t.gomoras))
}

/// `ccBossSkillDamage(CORE_THIS, m, i)` on each member alive.
fn core_damage_all(t: &mut Tree, cx: &mut Cx, i: usize) {
    let Some(c) = t.core.as_mut() else { return };
    for m in cx.party.members.into_iter().flatten() {
        if cx.valid(Some(m)) && cx.scene.chars[m].hp > 0 {
            let cm = c.me;
            as_me(cx, cm, |cx| c.b.skill_damage(cx, Some(m), None, i));
        }
    }
}

/// The arm's after-images (+0x29620 of the first fight's body, +0x296c0
/// of the second's, the constructors at MUT gcmn 0x004d2f70 and OUT
/// 0x004d5d68): colour 0x00525221 at 30, transparency 0.5 by -0.02, no
/// scale.
pub const ARM_AFTERIMAGE: AfterImageParams = AfterImageParams {
    color: 0x0052_5221,
    color_alpha: 0x41f0_0000,
    color_spd: 0,
    max_frame: 0x42a0_0000,
    transparency: 0x3f00_0000,
    trans_spd: 0xbca3_d70a,
    ss: VF0,
    es: VF0,
    scale_time: 0,
};

/// `HandAtk` (MUT gcmn 0x004d5930, OUT 0x004d8c70, QUA 0x003d4280): the
/// arm's sweep, boss skill 33 (36, 76) on every member at its frame 48.
/// The later fights' early stages look from further and shake harder.
fn hand_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    let ex = fourth::ex(x);
    match b.act_proccess {
        // The EX turn's attacks begin at once, the core beside the body.
        0 if ex => {
            b.act_proccess += 1;
            cx.out(Out::Blur(Set::Enable));
            cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            b.lock_player(cx, false);
        }
        1 if ex => b.act_proccess += 1,
        0 => {
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                cx.out(Out::Blur(Set::Enable));
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
                b.lock_player(cx, false);
            }
        }
        1 => {
            if core_state_is(t, 6) {
                b.act_proccess += 1;
            }
        }
        2 => {
            let range = match (x.fight, x.disc_lv) {
                (Fight::First, _) => 0x4541_c000,
                (Fight::Second, _) if x.disc_lv != x.disc_max_lv => 0x458c_a000,
                (Fight::Second, _) => 0x455a_c000,
                // The third's by stage: 6000, 3000, 4500.
                (Fight::Third, 1) => 0x45bb_8000,
                (Fight::Third, 2) => 0x453b_8000,
                (Fight::Third, _) => 0x458c_a000,
                // The fourth's 1800 past Kite (none at the last place).
                (Fight::Fourth, 4) => {
                    b.act_proccess += 1;
                    return;
                }
                (Fight::Fourth, _) => ee::add(0x44e1_0000, super::dist(cx, cx.scene.chars[me].pos, x.disc_pos)),
            };
            cx.out(Out::CamModeRange { mode: 3, range });
            b.act_proccess += 1;
        }
        3 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
            }
        }
        4 => {
            let m = geom::unit_matrix();
            x.tes_pos = cx.cam.active_pos;
            x.tes_dirc = cx.cam.rot;
            let arm = match x.fight {
                Fight::Fourth => [0, 0xc535_4000, 0xc396_0000, ONE],
                _ => [0xc2dc_0000, 0xc509_8000, 0, ONE],
            };
            let v = geom::apply_matrix(&geom::rot_matrix_z(&m, x.tes_dirc[2]), arm);
            let rm = geom::rot_matrix_z(&m, x.tes_dirc[2]);
            x.tes_dirc[0] = 0;
            x.tes_dirc[1] = 0;
            x.tes_dirc[2] = wrap(ee::sub(x.tes_dirc[2], PI));
            x.tes_pos = geom::vadd(x.tes_pos, v);
            b.act_proccess += 1;
            x.snd_pos_l = [0xc3fa_0000, 0, 0, ONE];
            x.snd_pos_r = [0x43fa_0000, 0, 0, ONE];
            x.snd_pos_l = geom::vadd(geom::apply_matrix(&rm, x.snd_pos_l), x.tes_pos);
            x.snd_pos_r = geom::vadd(geom::apply_matrix(&rm, x.snd_pos_r), x.tes_pos);
            let pos = cx.scene.chars[me].pos;
            cx.out(if x.fight == Fight::Fourth {
                Out::SeNote { se: 40, note: 60 }
            } else {
                Out::Se3dNote { se: 40, pos, note: 60 }
            });
        }
        5 => {
            if x.af_cou == x.af_timing {
                let (frame, pos, dirc) = (x.anm_w.frame(), x.tes_pos, x.tes_dirc);
                cx.out(Out::AfterImageAnm { frame, pos, dirc, params: ARM_AFTERIMAGE });
            }
            let q = if x.fight.later() && x.disc_lv != x.disc_max_lv {
                [0x41f0_0000, 0x4220_0000, 0x4270_0000]
            } else {
                [0x41a0_0000, 0x41f0_0000, 0x4220_0000]
            };
            x.quake_vector[..3].copy_from_slice(&q);
            if x.anm_w.frame() == 20 {
                cx.out(Out::Se3dNote { se: 206, pos: x.snd_pos_r, note: 68 });
            }
            if x.anm_w.frame() == 12 {
                cx.out(Out::Se3dNote { se: 206, pos: x.snd_pos_l, note: 68 });
            }
            let ended = x.anm_w.forward();
            cx.out(Out::DrawArm { time: x.anm_w.posed, pos: x.tes_pos, dirc: x.tes_dirc });
            if x.anm_w.frame() == 48 {
                core_damage_all(t, cx, x.fight.skills()[0]);
                cx.out(Out::Se3dNote { se: 56, pos: x.disc_pos, note: 72 });
                if x.fight == Fight::First {
                    cx.out(Out::Se3d { se: 40, pos: x.disc_pos });
                }
            }
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            if ended {
                cx.out(Out::CamMode { mode: 4 });
                core_state(t, cx, 7);
                x.anm_w.set("ANM_ex0batc0", cx.clips);
                b.act_proccess += 1;
            }
        }
        6 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            }
        }
        // The fourth fight's end waits for the core's fade only (the EX
        // turn's ends it).
        7 if ex => {
            core_end_fade_ex(t, cx);
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        7 if x.fight == Fight::Fourth && core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        7 if x.fight != Fight::Fourth && b.anm_status != 0 && core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

/// The core's `EndFadeEX()`: back from its fade at once in the EX turn.
pub(super) fn core_end_fade_ex(t: &mut Tree, cx: &mut Cx) {
    if let Some(c) = t.core.as_mut() {
        let cm = c.me;
        as_me(cx, cm, |cx| self::core::end_fade_ex(c, cx));
    }
}

/// `KyviaMagicDamage(pos)` (MUT gcmn 0x004d53b0, OUT 0x004d86e0), the
/// meteors' callback: boss skill 34 (38) by the core on each member alive;
/// 1.
fn magic_damage(fight: Fight, t: &mut Tree, cx: &mut Cx) -> i32 {
    core_damage_all(t, cx, fight.skills()[2]);
    1
}

/// `MegidFlame` (MUT gcmn 0x004d8bb0, OUT 0x004dc000): the meteors
/// (`ccBossEffMeteoriteMissile` of seven over the disc, 2500 up and 500
/// ahead), stream 48 (73) shown first.
fn megid_flame(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    let ex = fourth::ex(x);
    match b.act_proccess {
        0 if ex => {
            b.act_proccess += 1;
            b.lock_player(cx, false);
        }
        // The third fight holds the player from the first frame on.
        0 if x.fight == Fight::Third => {
            b.lock_player(cx, false);
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
            }
        }
        0 => {
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                b.lock_player(cx, false);
            }
        }
        1 => {
            if ex || core_state_is(t, 6) {
                b.act_proccess += 1;
                cx.out(Out::CamModeRange { mode: 3, range: 0x451c_4000 });
                cx.out(Out::Blur(Set::Enable));
                cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
            }
        }
        2 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
            }
        }
        3 => {
            // The later fights wait out an annihilation or a game over too.
            let over = x.fight.later() && (cx.annihilated() || cx.game_over);
            if cx.env.menu_type != -1 || over {
                return;
            }
            let (stream, back, up, eye) = match x.fight {
                Fight::First => (48, 0x43af_0000, 0x4422_8000, 0xc522_8000),
                Fight::Second => (73, 0x4409_8000, 0x4489_8000, 0xc52f_0000),
                Fight::Third => (108, 0x4409_8000, 0x4489_8000, 0xc52f_0000),
                Fight::Fourth => (110, 0x4409_8000, 0x4489_8000, 0xc52f_0000),
            };
            cx.out(Out::StreamMenu { stream, mask: 0 });
            cx.out(Out::CamMode { mode: 5 });
            x.l_offset = cx.scene.chars[me].pos;
            x.l_offset[1] = ee::sub(x.l_offset[1], back);
            x.l_offset[2] = ee::add(x.l_offset[2], up);
            x.atk_vec = VF0;
            x.atk_mat = geom::unit_matrix();
            x.atk_vec[1] = eye;
            let r = rand_f(cx.cc, 0x3ecc_cccd);
            x.atk_mat = geom::rot_matrix_z(&x.atk_mat, r);
            x.atk_mat = geom::rot_matrix_x(&x.atk_mat, 0x3e4c_cccd);
            let mut eye = geom::vadd(geom::apply_matrix(&x.atk_mat, x.atk_vec), x.l_offset);
            eye[3] = ONE;
            cx.out(Out::FreeCam { pos: eye, view: x.l_offset });
            b.act_proccess += 1;
        }
        4 => {
            b.act_proccess += 1;
            x.s_point = x.disc_pos;
            x.s_point[1] = ee::add(x.s_point[1], 0x43fa_0000);
            x.s_point[2] = ee::add(x.s_point[2], 0x451c_4000);
        }
        5 => {
            b.act_proccess += 1;
            meteorite(b, x, cx);
            cx.out(Out::Se3dNote { se: 57, pos: x.disc_pos, note: 70 });
            b.act_count = 0;
        }
        6 => {
            let at = x.disc_pos;
            match b.act_count {
                5 => cx.out(Out::Se3dNote { se: 57, pos: at, note: 60 }),
                20 => {
                    cx.out(Out::Se3dNote { se: 40, pos: at, note: 60 });
                    cx.out(Out::Se3dNote { se: 56, pos: at, note: 50 });
                }
                40 => cx.out(Out::Se3dNote { se: 40, pos: at, note: 60 }),
                60 => {
                    cx.out(Out::Se3dNote { se: 40, pos: at, note: 60 });
                    if x.fight == Fight::Second {
                        x.quake_vector = [0x41a0_0000, 0x4248_0000, 0x42a0_0000, x.quake_vector[3]];
                        let q = x.quake_vector;
                        cx.quake_vec([q[0], q[1], q[2]]);
                    }
                }
                _ => {}
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 100 {
                b.act_proccess += 1;
                cx.out(Out::CamMode { mode: 6 });
            }
        }
        7 => {
            if !ex {
                core_state(t, cx, 7);
            }
            cx.out(Out::CamMode { mode: 4 });
            b.act_proccess += 1;
        }
        8 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            }
        }
        9 if ex => {
            core_end_fade_ex(t, cx);
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        9 if core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

// --- LightAtk ----------------------------------------------------------------------------

/// A Catmull-Rom point of `p` at `t`, with the game's `mula`/`madd` chain.
pub(crate) fn catmull(t: F, p: [F; 4]) -> F {
    let t2 = ee::mul(t, t);
    let t3 = ee::mul(t2, t);
    let acc = ee::add(ee::mul(0xbf00_0000, t3), t2);
    let b0 = ee::sub(acc, ee::mul(HALF, t));
    let half_t = ee::mul(HALF, t);
    let b1 = ee::add(ONE, ee::add(ee::mul(0x3fc0_0000, t3), ee::mul(0xc020_0000, t2)));
    let b2 = ee::add(half_t, ee::add(ee::mul(0xbfc0_0000, t3), ee::mul(0x4000_0000, t2)));
    let b3 = ee::sub(ee::mul(HALF, t3), ee::mul(HALF, t2));
    let f1 = ee::add(ee::mul(p[0], b0), ee::mul(p[1], b1));
    let acc = ee::add(ee::mul(p[2], b2), f1);
    ee::add(acc, ee::mul(p[3], b3))
}

/// A beam's point: the spline through `p` at `t` in each of x, y, z, w 1.
fn beam(t: F, p: [V4; 4]) -> V4 {
    let mut v = [0, 0, 0, ONE];
    for (k, c) in v.iter_mut().take(3).enumerate() {
        *c = catmull(t, [p[0][k], p[1][k], p[2][k], p[3][k]]);
    }
    v
}

/// `SetBPos` (MUT gcmn 0x004d5450): the six beam points off the disc.
fn set_b_pos(x: &mut Kyvia, cx: &mut Cx) {
    x.l_point.fill(x.disc_pos);
    for (k, r) in [0x442f_0000, 0x4422_8000, 0x4402_0000, 0x43fa_0000, 0x443b_8000, 0x4422_8000].into_iter().enumerate()
    {
        let v = rand_f(cx.cc, r);
        x.l_point[k][0] = ee::add(x.l_point[k][0], v);
    }
    for (k, r) in [0x43c8_0000, 0x43e1_0000, 0x4413_8000, 0x4396_0000, 0x4409_8000, 0x43c8_0000].into_iter().enumerate()
    {
        let v = rand_f(cx.cc, r);
        x.l_point[k][1] = ee::add(x.l_point[k][1], v);
    }
    for (k, r) in [0x43aa_0000, 0x43af_0000, 0x43c3_0000].into_iter().enumerate() {
        let v = rand_f(cx.cc, r);
        x.l_point[k][2] = ee::add(x.l_point[k][2], ee::add(0x4396_0000, v));
    }
    for (k, r) in [0x4396_0000, 0x4409_8000, 0x43c8_0000].into_iter().enumerate() {
        let v = rand_abs(cx, r);
        x.l_point[3 + k][2] = ee::add(x.l_point[3 + k][2], fp(100.0 + dp(v)));
    }
}

/// `LightAtk` (MUT gcmn 0x004d60a0, OUT 0x004d94a0): three beams from the
/// body over the party, their paths splines through `SetBPos`'s points,
/// landing as boss skill 32 (37) on every member.
fn light_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    let ex = fourth::ex(x);
    let fourth = x.fight == Fight::Fourth;
    // The fourth fight's generators here keep `distSW` off.
    let pick = |g: Gen, f: fourth::Make| if fourth { Gen::Fourth(f) } else { g };
    match b.act_proccess {
        // The EX turn's: before the body at once, the party gathered.
        0 if ex => {
            x.l_offset = VF0;
            x.move_transfer = VF0;
            x.l_offset = cx.scene.chars[me].pos;
            x.l_offset[1] = ee::sub(x.l_offset[1], 0x4448_0000);
            x.l_offset[2] = ee::add(x.l_offset[2], 0x44ce_4000);
            b.act_proccess += 1;
            cx.out(Out::CamMode { mode: 5 });
            x.atk_vec = VF0;
            x.atk_mat = geom::unit_matrix();
            x.atk_vec[1] = 0xc409_8000;
            x.z = rand_f(cx.cc, 0x3e4c_cccd);
            x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
            x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
            x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
            cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
            cx.out(Out::Blur(Set::Enable));
            cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
            let mut at = [0x4188_0000, 0xc4c7_c000, 0x447a_0000, ONE];
            fourth::together_char_pos(cx, &mut at, 2);
            b.lock_player(cx, false);
        }
        1 if ex => {
            b.act_proccess += 1;
            x.atk_vec = VF0;
            x.atk_vec[1] = 0xc3af_0000;
            x.quake_vector = [0x4120_0000, 0x4120_0000, 0x41f0_0000, x.quake_vector[3]];
        }
        0 => {
            if !core_state_is(t, 0) {
                return;
            }
            x.l_offset = VF0;
            x.move_transfer = VF0;
            let (back, up) = match x.fight {
                Fight::First => (0x43af_0000, 0x4422_8000),
                Fight::Second => (0x4409_8000, 0x4496_0000),
                Fight::Third => (0x4409_8000, 0x44a2_8000),
                Fight::Fourth => (0x4409_8000, 0x44a8_c000),
            };
            x.l_offset = cx.scene.chars[me].pos;
            x.l_offset[1] = ee::sub(x.l_offset[1], back);
            x.l_offset[2] = ee::add(x.l_offset[2], up);
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                cx.out(Out::CamMode { mode: 5 });
                x.atk_vec = VF0;
                x.atk_mat = geom::unit_matrix();
                x.atk_vec[1] = back ^ 0x8000_0000;
                x.z = rand_f(cx.cc, if x.fight == Fight::Third { 0x3e4c_cccd } else { HALF });
                x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
                x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
                x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
                cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
                b.lock_player(cx, false);
            }
            cx.out(Out::Blur(Set::Enable));
            cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
        }
        1 => {
            if core_state_is(t, 6) {
                b.act_proccess += 1;
                x.atk_vec = VF0;
                x.atk_vec[1] = 0xc3af_0000;
                x.quake_vector = [0x4120_0000, 0x4120_0000, 0x4120_0000, x.quake_vector[3]];
            }
        }
        2 => {
            b.act_proccess += 1;
            particles(cx, pick(Gen::Light, fourth::Make::Light), Place::At(x.l_offset), Some(Held::Light));
            let mut n = 0;
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && cx.scene.chars[m].hp > 0 {
                    x.l_char[n] = cx.scene.chars[m].pos;
                    n += 1;
                }
            }
            set_b_pos(x, cx);
            match 3 - n as i32 {
                2 => {
                    x.l_char[1] = x.l_char[0];
                    x.l_char[2] = x.l_char[0];
                }
                1 => {
                    let r = cx.cc.rand();
                    if r % 2 != 0 {
                        x.l_char[2] = x.l_char[0];
                    } else {
                        x.l_char[2] = x.l_char[1];
                    }
                }
                _ => {}
            }
            x.add_time = 0x3d23_d70a;
            x.sub_btime = fp(1.5 * dp(0xbd23_d70a));
            x.btime = 0;
            cx.out(Out::Se3dNote { se: 191, pos: cx.scene.chars[me].pos, note: 60 });
        }
        3 => {
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            let c = b.act_count;
            b.act_count += 1;
            if c == 40 {
                b.act_count = 0;
                b.act_proccess += 1;
                let tilt = if x.fight.later() { 0x3eb3_3333 } else { 0x3e19_999a };
                x.atk_mat = geom::rot_matrix_x(&x.atk_mat, tilt);
                if fourth {
                    let at = [0x4188_0000, 0xc4c7_c000, 0x447a_0000, ONE];
                    particles(cx, Gen::Fourth(fourth::Make::Glow), Place::At(at), None);
                }
            }
        }
        4 => {
            let far = match x.fight {
                Fight::Second => 2300.0,
                Fight::Fourth => 2000.0,
                _ => 2500.0,
            };
            let left = fp(far - dp(fabs(x.move_transfer[1])));
            if !ee::le(left, 0x4100_0000) {
                x.move_transfer[1] = ee::add(x.move_transfer[1], ee::mul(0xbf80_0000, ee::div(left, 0x4080_0000)));
            }
            let mut eye = geom::apply_matrix(&x.atk_mat, x.move_transfer);
            eye = geom::vadd(eye, x.atk_vec);
            eye = geom::vadd(x.l_offset, eye);
            let third = x.fight == Fight::Third;
            if third {
                // The point looked at sinks 8 a frame, the eye rises 12.
                x.l_offset[2] = ee::sub(x.l_offset[2], 0x4100_0000);
                x.atk_vec[2] = ee::add(x.atk_vec[2], 0x4140_0000);
            }
            // The fourth's looks 450 under the beams' start.
            let mut view = x.l_offset;
            if fourth {
                view[2] = ee::sub(view[2], 0x43e1_0000);
            }
            cx.out(Out::FreeCam { pos: eye, view });
            // The third fight counts after the check (and raises the beams'
            // start 180): its beams leave a frame later in the count.
            let c = b.act_count;
            if !third {
                b.act_count += 1;
            }
            if c == 30 {
                if third {
                    x.l_offset = geom::vadd(x.l_offset, [0, 0, 0x4334_0000, ONE]);
                }
                b.act_proccess += 1;
                particles(cx, pick(Gen::BeamsOut, fourth::Make::BeamsOut), Place::At(x.l_offset), None);
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 192, pos, note: 60 });
                cx.out(Out::Se3dNote { se: 61, pos, note: 48 });
                for k in 0..3 {
                    x.l_bpos[k] = x.l_offset;
                    x.l_bpos_sub[k] = x.l_offset;
                    let n = k as u8;
                    let (head, tail) =
                        (pick(Gen::BeamHead, fourth::Make::BeamHead), pick(Gen::BeamTail, fourth::Make::BeamTail));
                    particles(cx, head, Place::Follows(Follow::BeamHead(n), VF0), Some(Held::BeamHead(n)));
                    particles(cx, tail, Place::Follows(Follow::BeamTail(n), VF0), Some(Held::BeamTail(n)));
                    b.act_count = 0;
                }
            }
            if third {
                b.act_count += 1;
            }
        }
        5 => {
            let c = b.act_count;
            b.act_count += 1;
            if c < 10 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            if !ee::le(x.btime, ONE) {
                x.time_mode += 1;
                if x.time_mode == 3 {
                    x.add_time = ee::mul(0x4000_0000, x.add_time);
                }
                x.btime = 0;
                x.sub_btime = fp(1.5 * dp(x.add_time ^ 0x8000_0000));
            }
            let (o, p, c) = (x.l_offset, x.l_point, x.l_char);
            let paths: Option<[[V4; 4]; 3]> = match x.time_mode {
                1 => Some(std::array::from_fn(|k| [o, o, p[k], p[k + 3]])),
                2 => Some(std::array::from_fn(|k| [o, p[k], p[k + 3], c[k]])),
                3 => Some(std::array::from_fn(|k| [p[k], p[k + 3], c[k], c[k]])),
                _ => None,
            };
            if let Some(paths) = paths {
                // Each beam's head, then (while SubBtime is past 0) its tail.
                for (b, p) in x.l_bpos.iter_mut().zip(&paths) {
                    *b = beam(x.btime, *p);
                }
                if !ee::le(x.sub_btime, 0) {
                    let tt = if geom::eq(0, x.sub_btime) { x.btime } else { x.sub_btime };
                    for (b, p) in x.l_bpos_sub.iter_mut().zip(&paths) {
                        *b = beam(tt, *p);
                    }
                }
            } else if x.time_mode == 4 {
                x.time_mode = 1;
                if x.fight == Fight::Second {
                    x.quake_vector = [0x4248_0000, 0x4248_0000, 0x42c8_0000, x.quake_vector[3]];
                }
                for k in 0..3 {
                    // The beam's ends killed, their pointers kept.
                    gen_op(cx, Held::BeamHead(k as u8), GenOp::Kill);
                    gen_op(cx, Held::BeamTail(k as u8), GenOp::Kill);
                    let m = cx.party.members[k];
                    if let Some(m) = m
                        && cx.valid(Some(m))
                        && cx.scene.chars[m].hp > 0
                    {
                        let burst = pick(Gen::BeamBurst, fourth::Make::BeamBurst);
                        particles(cx, burst, Place::At(x.l_bpos[k]), Some(Held::Light));
                    }
                }
                b.act_proccess += 1;
            }
            x.btime = ee::add(x.btime, x.add_time);
            x.sub_btime = ee::add(x.sub_btime, x.add_time);
        }
        6 => {
            core_damage_all(t, cx, x.fight.skills()[1]);
            b.act_proccess += 1;
            b.act_count = 0;
            let pos = cx.scene.chars[me].pos;
            cx.out(Out::Se3dNote { se: 35, pos, note: 60 });
            cx.out(Out::Se3dNote { se: 56, pos, note: 60 });
            match x.fight {
                Fight::Third => x.quake_vector = [0x41a0_0000, 0x41f0_0000, 0x4234_0000, x.quake_vector[3]],
                Fight::Fourth => x.quake_vector = [0x4120_0000, 0x4120_0000, 0x42c8_0000, x.quake_vector[3]],
                _ => {}
            }
        }
        7 => {
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                b.act_proccess += 1;
            }
        }
        8 => {
            cx.out(Out::CamMode { mode: 6 });
            cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            if !ex {
                core_state(t, cx, 7);
            }
            b.act_proccess += 1;
        }
        9 if ex => {
            core_end_fade_ex(t, cx);
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        9 if core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

// --- the meteors -------------------------------------------------------------------------

/// `ccBossEffMeteoriteMissile` (MUT gcmn 0x0047aa90, `Draw` 0x0047b330): each
/// meteor's fall, a Bezier from 500 back (turned by -2.5525 about x) down to
/// a random point round the missile's place; the first's landing calls the
/// body's `KyviaMagicDamage` at 20 and 60 frames on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meteorite {
    pub pos: V4,
    pub start: Vec<V4>,
    pub pos2: Vec<V4>,
    pub pos3: Vec<V4>,
    pub end: Vec<V4>,
    pub now: Vec<V4>,
    /// `t` (each meteor's step), `Time` (its place on the path).
    pub step: Vec<F>,
    pub time: Vec<F>,
    pub landed: Vec<u8>,
    pub count: Vec<i32>,
    pub quake: V4,
}

/// What a meteor shower's `Draw` asks beside its flights, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeteorCall {
    /// Meteor `k` landed: SE 35 note 60 at the shower's place, and for the
    /// first its burst (`BurstSmokeGenerator` rows 8, 9 and 11, and the
    /// rocks).
    Landed(usize),
    /// `bossCam->QuakeCam(quake)` while a landed meteor's count is under
    /// 60.
    Quake,
    /// The first meteor's count at 20 or 60: the callback
    /// (`KyviaMagicDamage`).
    Due,
}

impl Meteorite {
    /// The constructor's flights for `n` meteors round `pos` within
    /// `range`; `cc` is `ccRandF`'s source.
    pub fn make(cc: &mut dyn crate::rand::Rng, pos: V4, range: F, n: usize) -> Meteorite {
        let mut m = Meteorite { pos, ..Meteorite::default() };
        for _ in 0..n {
            let turn = rand_f(cc, 0x4048_f5c3);
            let rm = geom::rot_matrix_z(&geom::unit_matrix(), turn);
            let r = rand_f(cc, ee::mul(range, HALF));
            let mut v = VF0;
            v[1] = fabs(ee::add(ee::mul(range, ee::sub(ONE, HALF)), r));
            m.end.push(geom::apply_matrix(&rm, v));
        }
        let rx = geom::rot_matrix_x(&geom::unit_matrix(), 0xc023_5ce2);
        for k in 0..n {
            let back = geom::apply_matrix(&rx, [0, 0xc3fa_0000, 0, ONE]);
            let p3 = geom::vadd(m.end[k], back);
            let p2 = geom::vadd(p3, geom::vadd(p3, back));
            let p1 = geom::vadd(p2, back);
            m.pos3.push(p3);
            m.pos2.push(p2);
            m.start.push(p1);
        }
        for _ in 0..n {
            let r = fabs(rand_f(cc, 0x3cf5_c28f));
            m.step.push(fp(dp(0x3cf5_c28f) + dp(r)));
            m.time.push(0);
        }
        m.now = vec![VF0; n];
        m.landed = vec![0; n];
        m.count = vec![0; n];
        m.quake = [0x41f0_0000, 0x4220_0000, 0x4248_0000, 0];
        m
    }

    /// The constructor in the fight: the flights and each meteor's two
    /// trails (their pictures piney-effect's).
    fn new(cx: &mut Cx, pos: V4, range: F, n: usize) -> Meteorite {
        let m = Meteorite::make(cx.cc, pos, range, n);
        for k in 0..n {
            particles(cx, Gen::Trail(6), Place::At(m.now[k]), None);
            particles(cx, Gen::Trail(7), Place::At(m.now[k]), None);
        }
        m
    }

    /// `Spline` (MUT 0x0047b910): the cubic Bezier at `t`.
    fn spline(t: F, p: [V4; 4]) -> V4 {
        let u = ee::sub(ONE, t);
        let b0 = ee::mul(u, ee::mul(u, u));
        let b1 = ee::mul(t, ee::mul(ee::mul(0x4040_0000, u), u));
        let b2 = ee::mul(t, ee::mul(ee::mul(0x4040_0000, u), t));
        let b3 = ee::mul(t, ee::mul(t, t));
        let mut v = [0, 0, 0, ONE];
        for (i, c) in v.iter_mut().take(3).enumerate() {
            let acc = ee::add(ee::mul(p[0][i], b0), ee::mul(p[1][i], b1));
            let acc = ee::add(ee::mul(p[2][i], b2), acc);
            *c = ee::add(acc, ee::mul(p[3][i], b3));
        }
        v
    }

    /// One `Draw`'s flights, meteor by meteor, what it asks in `calls`:
    /// false once every meteor has lain 90 frames.
    pub fn fly(&mut self, calls: &mut Vec<MeteorCall>) -> bool {
        for k in 0..self.end.len() {
            if self.landed[k] == 0 {
                let p = Self::spline(self.time[k], [self.start[k], self.pos2[k], self.pos3[k], self.end[k]]);
                self.now[k] = geom::vadd(p, self.pos);
            }
            if dp(self.time[k]) > 0.984 && self.landed[k] == 0 {
                self.landed[k] = 1;
                calls.push(MeteorCall::Landed(k));
            }
            if self.landed[k] != 0 {
                self.count[k] += 1;
                if self.count[k] < 60 {
                    calls.push(MeteorCall::Quake);
                }
            }
            if k == 0 && matches!(self.count[0], 20 | 60) {
                calls.push(MeteorCall::Due);
            }
            self.time[k] = ee::add(self.time[k], self.step[k]);
        }
        !self.landed.iter().zip(&self.count).all(|(&l, &c)| l == 1 && c >= 90)
    }

    /// One `Draw` in the fight: the flights, then their calls in order.
    /// The first's landing calls `fight`'s `KyviaMagicDamage`.
    fn draw(&mut self, fight: Fight, t: &mut Tree, cx: &mut Cx) -> bool {
        let mut calls = Vec::new();
        let alive = self.fly(&mut calls);
        for c in calls {
            match c {
                MeteorCall::Landed(k) => {
                    cx.out(Out::Se3dNote { se: 35, pos: self.pos, note: 60 });
                    if k == 0 {
                        particles(cx, Gen::Burst(8), Place::At(self.pos), None);
                        particles(cx, Gen::Burst(9), Place::At(self.pos), None);
                        let mut up = VF0;
                        up[2] = 0x42f0_0000;
                        particles(cx, Gen::Burst(11), Place::At(geom::vadd(up, self.pos)), None);
                        let mut rot = cx.cam.rot;
                        rot[2] = wrap(ee::sub(rot[2], PI));
                        rot[0] = 0;
                        rot[1] = 0;
                        cx.out(Out::SmokeRock { pos: self.now[k], rot });
                    }
                }
                MeteorCall::Quake => {
                    let q = self.quake;
                    cx.quake_vec([q[0], q[1], q[2]]);
                }
                MeteorCall::Due => {
                    magic_damage(fight, t, cx);
                }
            }
        }
        alive
    }
}

/// `ccBossEffMeteoriteMissileCreate(DiscPos, 500, 7, 0, KyviaMagicDamage,
/// bossCam)`.
fn meteorite(b: &mut Boss, x: &mut Kyvia, cx: &mut Cx) {
    let m = Meteorite::new(cx, x.disc_pos, 0x43fa_0000, 7);
    let kind = EffKind::Meteorite { count: 7 };
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.meteorite = Some(Box::new(m));
    }
    cx.out(Out::Effect { id, kind, pos: x.disc_pos, dirc: VF0 });
}

/// `ccBossEffManager::Draw` over the fight's effects: a meteorite's `Draw`
/// may call the body's `KyviaMagicDamage`, the magical ball's its
/// `KyviaMagicalBallDamage`; a thunderbolt's draws from `ccRand`.
fn manager_pass(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    for k in 0..b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else { continue };
        if !e.enabled {
            b.effects.slots[k] = None;
            continue;
        }
        let alive = if let Some(mut ball) = e.ball.take() {
            let alive = ball.draw(&mut magic_ball::FightBall { b, x, t, cx });
            if let Some(e) = b.effects.slots[k].as_mut() {
                e.ball = Some(ball);
            }
            alive
        } else if let Some(mut hell) = e.hell.take() {
            let (abgr, scale) = (cx.disc.blur_abgr, cx.disc.blur_scale);
            let alive = hell.draw(&mut hell::FightHell { b, cx, slot: k, abgr, scale });
            if let Some(e) = b.effects.slots[k].as_mut() {
                e.hell = Some(hell);
            }
            alive
        } else {
            match (e.meteorite.as_mut(), e.bolt.as_mut()) {
                (Some(m), _) => m.draw(x.fight, t, cx),
                (None, Some(bolt)) => bolt.draw(cx),
                (None, None) => {
                    e.draw();
                    continue;
                }
            }
        };
        if !alive && let Some(e) = b.effects.slots[k].as_mut() {
            e.enabled = false;
        }
    }
}

#[cfg(test)]
mod tests;
