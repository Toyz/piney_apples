//! The enemies' movement and animation, and `ccEnemy::main` as a whole frame
//! ([`Motion::enemy_main`], 0x00432cd0): `moveEnemy`, `animEnemy`,
//! `dispEnemy` and the act helpers of `enemy.cpp`, and each race's `action()`,
//! `exclusive()` and `note()` by [`Kind`] (the files' ranges are in
//! docs/engine/source-tree.md; [`Kind::Other`] does nothing here). The
//! decisions are [`crate::enemy_ai`]'s; the affects land where the game calls
//! `EntryAffect`, and the rest goes to the world through [`MotionWorld::call`].
//! The motion is in docs/engine/battle.md ("Enemy movement and animation").

use piney_data::Result;
use piney_data::iso::Iso;

use crate::affect::{self, AffectCtx};
use crate::blocks::InfoRef;
use crate::chara::{self, AffectFunc, Body, Env};
use crate::damage::{self, Roll};
use crate::enemy_ai::{self, Ai, Begin, DrainSpawn, Enemy, Out, SkillUse, act, object_size, rad_disperse, rand_f};
use crate::event::{Event, Events, Who};
use crate::geom::*;
use crate::param::{SkillParam, cond, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::{self, bits};
use crate::tables::Tables;
use crate::weapon::WeaponAt;
use crate::world::{AnmSlot, Note, World};
use piney_data::tables::sjis::encode;

const K_0_005: F = 0x3ba3_d70a;
const K_0_01: F = 0x3c23_d70a;
const K_0_02: F = 0x3ca3_d70a;
const K_0_03: F = 0x3cf5_c28f;
const K_0_04: F = 0x3d23_d70a;
const K_0_05: F = 0x3d4c_cccd;
const K_0_06: F = 0x3d75_c28f;
const K_0_0625: F = 0x3d80_0000;
const K_0_08: F = 0x3da3_d70a;
const K_0_1: F = 0x3dcc_cccd;
const K_0_125: F = 0x3e00_0000;
const K_0_2: F = 0x3e4c_cccd;
const K_0_3: F = 0x3e99_999a;
const K_M0_3: F = 0xbe99_999a;
const K_0_4: F = 0x3ecc_cccd;
const K_0_5: F = 0x3f00_0000;
const K_0_6: F = 0x3f19_999a;
const K_0_7: F = 0x3f33_3333;
const K_0_8: F = 0x3f4c_cccd;
const K_0_9: F = 0x3f66_6666;
const K_1_1: F = 0x3f8c_cccd;
const K_1_5: F = 0x3fc0_0000;
const K_2: F = 0x4000_0000;
const K_3: F = 0x4040_0000;
const K_4: F = 0x4080_0000;
const K_5: F = 0x40a0_0000;
const K_10: F = 0x4120_0000;
const K_30: F = 0x41f0_0000;
const K_40: F = 0x4220_0000;
const K_100: F = 0x42c8_0000;
const K_200: F = 0x4348_0000;
const K_256: F = 0x4380_0000;
const K_300: F = 0x4396_0000;
const K_400: F = 0x43c8_0000;
const K_512: F = 0x4400_0000;
const K_600: F = 0x4416_0000;
const K_7000: F = 0x45da_c000;
const K_0_09: F = 0x3db8_51ec;
const K_0_16: F = 0x3e23_d70a;
const K_0_24: F = 0x3e75_c28f;
const K_0_28: F = 0x3e8f_5c29;
const K_1_8: F = 0x3fe6_6666;
const K_80: F = 0x42a0_0000;
const K_250: F = 0x437a_0000;
const K_M300: F = 0xc396_0000;
/// 3pi/4 (0x4016cbe4): the most a Death Head's escape turns aside.
const K_3PI_4: F = 0x4016_cbe4;
const K_0_004: F = 0x3b83_126f;
const K_1_2: F = 0x3f99_999a;
const K_60: F = 0x4270_0000;
const K_M50: F = 0xc248_0000;
/// `eet1DustInfo`: rows 8-11 are the turtles' footsteps.
const EET1_DUST_INFO: &str = "eet1DustInfo";
/// `ehkBrParam`: `ccEnemyH`'s breath.
const EHK_BR_PARAM: InfoRef = InfoRef::new("ehkBrParam", 0);
/// `elBrParam`: `ccEnemyL`'s breaths, the dragons' attacks 4 and 5 (rows 0
/// and 1) and the wyrms' (row 2).
const EL_BR_PARAM: &str = "elBrParam";
/// 0.0005: the loss of speed per unit of width at each body in the way.
const K_WIDTH_DRAG: F = 0x3a03_126f;
/// pi/24 (0x3e060a92): the swerve per body in the way.
const K_SWERVE: F = 0x3e06_0a92;
/// pi/10 (0x3ea0d97c) and 2pi/5 (0x3fa0d97c): an escape's scatter.
const K_PI_10: F = 0x3ea0_d97c;
/// 0.3 pi (`0x3f71463b`), 1.6, 2.5, 500, 1200, 0.25 and -0.8.
const K_0_3PI: F = 0x3f71_463b;
const K_1_6: F = 0x3fcc_cccd;
const K_2_5: F = 0x4020_0000;
const K_500: F = 0x43fa_0000;
const K_1200: F = 0x4496_0000;
const K_0_25: F = 0x3e80_0000;
const K_M0_8: F = 0xbf4c_cccd;
const K_2PI_5: F = 0x3fa0_d97c;
/// pi/4 (0x3f490fdb) and 1.4137167 (0x3fb4f4ab, 0.45 pi): the turn the
/// walk catches up with.
const K_PI_4: F = 0x3f49_0fdb;
const K_0_45PI: F = 0x3fb4_f4ab;

/// The polygons `moveEnemy` stands the enemy on (`ccLandHitCheck`).
pub const GROUND_MASK: u32 = 0x2000_0002;
/// The polygons `moveEnemy` tests the walk's line against
/// (`ccHitCheckLM2`).
pub const WALL_MASK: u32 = 0x4000_0002;

/// A call into code that is not rules, at the point the game makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// An output of [`crate::enemy_ai`]'s rules (`Ai::out`) or of the
    /// affects the frame applies, handed on where the rule made it: the
    /// presentation (`Out::Rule`'s numbers, marks, particles), the runtime's
    /// work (experience, removal, the drained form, a party member's
    /// cancelled attack and messages). The affects themselves
    /// (`Event::Affect`, `Out::Hold`) and an enemy's retargeting
    /// (`Event::EnemyRetarget`) are applied by the frame and not handed on.
    Rule(Out),
    /// `ccSkillRequest(this, skillTarget, skillId)` (gcmn 0x00572700) from
    /// `startSkill` on note 0x8003: a spell starts
    /// ([`crate::flow::Skills::request`], stype 0; draws `rand()`).
    SkillRequest { target: Option<usize>, sid: i32 },
    /// `ccSeSetParamEnemy(param, this, category)` (main 0x0017abd0): the
    /// sound of notes 1 and 2, `category` the race's `seCategory`.
    Sound { param: u32, category: i32 },
    /// `cameraShake(kind, 2, 20, 2)` (main 0x00162cd0), note 0x8002 near
    /// the camera; draws `rand()`.
    CameraShake { kind: i32 },
    /// `ccEnemyWeaponCtrl::note(this, note)` (gcmn 0x0043eae0): the race's
    /// weapon trail on every note ([`crate::weapon::WeaponCtrl::note`]);
    /// `at` is what it reads of the enemy then.
    WeaponNote { note: Note, at: WeaponAt },
    /// `ccEnemyWeaponCtrl::ctrl(this)` (gcmn 0x0043e900), from
    /// `exclusive()` ([`crate::weapon::WeaponCtrl::ctrl`]; it draws nothing
    /// random).
    WeaponCtrl(WeaponAt),
    /// `ccEnemyDustCtrl::ctrl(this, flag)` (gcmn 0x0043aa90): 0 from
    /// `exclusive()`, 1 on notes 1 and 2; its dust rings draw `ccRand()`
    /// and `rand()`. `at` is what it reads of the enemy then.
    DustCtrl { flag: i32, at: DustAt },
    /// A middle boss's second model (`anm2`, `ccEnemy` +0x1bc) drawn:
    /// its matrix and alpha set, `WORLD_MAN::SetActiveLayer(5)`,
    /// `ccAnm::Draw()` (main), `SetActiveLayer(0)`.
    DrawSecond { matrix: M4, alpha: F },
    /// The model's matrix set (`ccAnm` +0x40, from `sceVu0TransMatrix`)
    /// and `ccChar::Draw()` (gcmn 0x0056b1c0): the fog, flashes and the
    /// camera's transparency, then `ccAnm::Draw()`.
    Draw { matrix: M4 },
    /// `ccSeOn3D(id, pos)` (main 0x00179d90): a gold goblin's clink.
    Sound3d { id: i32, pos: V4 },
    /// `ccTransPosFW2LW(p, pos)` (gcmn 0x0059b9c0) then `ccEnemyEffDust(p,
    /// 1, size, 6, eneSmoke)` (gcmn 0x0043a3e0): a gold goblin's dust;
    /// draws `ccRand()` and `rand()`.
    Dust { pos: V4, size: F, smoke: i16 },
    /// `ccEnemyEffDust(pos, n, size, life, smoke)` (gcmn 0x0043a3e0): a
    /// scorpion's wheels spinning (`ccEnemyC`, `n` 2, size 3, life 20);
    /// draws `ccRand()` and `rand()`.
    EffDust { pos: V4, n: i32, size: F, life: i32, smoke: i16 },
    /// `ccEnemyE::note` (0x004452b0), a turtle (type 3) stepping: a ring of
    /// dust at its foot `foot` (`footObjTbl[foot]`, gcmn 0x005e0640, looked
    /// up in the clump: `GetObjAdrsF`; none when the model lacks it), by
    /// `eet1DustInfo` row `info`: `ccEnemyEffDustRing(the
    /// node's place, info.f8, info.f12, info.b1, info.b2, eneSmoke)`.
    FootDust { foot: usize, info: InfoRef, smoke: i16 },
    /// `ccEnemyBreath::ctrlBreath()` (gcmn 0x0043bee0) of `ccEnemyH`'s
    /// breath `slot`: its flames move, burst on the ground or a wall
    /// (`ccRandF`, `effSmoke`) and are drawn on layer 5.
    BreathCtrl { slot: usize },
    /// `ccEnemyBreath::setBreath(param, this)` (gcmn 0x0043bfe0): breath
    /// `slot` breathes a flame from its node, `param` its
    /// `ccEnemyBrParam` (`ehkBrParam`, `elBrParam` rows); what it reads
    /// of the enemy: `dispSW`, `transparency`, `actCnt`
    /// ([`crate::breath::Breath::set`]).
    BreathSet { slot: usize, param: InfoRef, disp: bool, transparency: F, act_cnt: i16 },
    /// A box's trap 0 (`ccGimBox::invokeTrap`, gcmn 0x00453c30): `dmg =
    /// CalcBattleDamage(target, sid, 1.0, -1)` of the box (draws `rand()`),
    /// then `target->EntryAffect(box, 1, dmg, 0, 0)`.
    TrapDamage { target: Option<usize>, sid: i32 },
    /// A box's traps 1 and 2: `ccItemSkillRequest(box, target, sid, 0)`
    /// (gcmn 0x00572790; stype 1, draws `rand()`).
    TrapSkill { target: Option<usize>, sid: i32 },
}

/// What a [`MotionWorld::call`] may reach, as the game's code does at that
/// point: the tables, the scene and both generators (the presentation that
/// draws, a skill request run at once).
pub struct At<'x> {
    pub t: &'x Tables,
    pub scene: &'x mut Scene,
    /// newlib's `rand()`.
    pub rand: &'x mut dyn Rng,
    /// `ccRand()`.
    pub cc: &'x mut dyn Rng,
}

/// What the motion code asks of the world beyond [`World`].
pub trait MotionWorld: World {
    /// `checkCameraShakeRange(pos)` (main 0x00162f10): whether a shock at
    /// `pos` is near enough the active camera to shake it
    /// (`ccEnemyCheckNote`, note 0x8002).
    fn shake_range(&mut self, pos: V4) -> bool;
    /// A call into code that is not rules, or an output of the rules, for
    /// enemy `who`, at the point the game makes it (see [`Call`] for the
    /// function each stands for). A call the game makes with effects on the
    /// characters or the generators (a skill request, the weapon and dust
    /// controllers, the camera shake) is carried out here, with `at`, so
    /// that everything after it in the frame sees it.
    fn call(&mut self, who: usize, c: Call, at: &mut At);
}

/// What `ccEnemyDustCtrl::ctrl` reads of its enemy: the controller's
/// `ccEnemyDustInfo` rows (`info`, `n` of them), `dispSW` (+0xe0 bit 4),
/// `eneSmoke` (+0x256), `anmNum` (+0x2bc), `frameNum` (+0x2c0, read as a
/// signed short), `pos` and `dirc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DustAt {
    pub info: Option<InfoRef>,
    pub n: i32,
    pub disp_sw: bool,
    pub smoke: i16,
    pub anm_num: i16,
    pub frame_num: i16,
    pub pos: V4,
    pub dirc: V4,
}

/// The class an enemy's row builds: the constructor of its race group in
/// `ccEntryRaceTbl` (gcmn 0x005f1d60; the rows' own `ccEntry.func` is
/// filled from it at run time), which sets the vtable, so which `action()`,
/// `exclusive()` and `note()` it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `ccEnemy1` (`ccEntryEnemy1`, 0x0043ecf0), race 0: the knights.
    E1,
    /// `ccEnemy2` (`ccEntryEnemy2`, 0x0043f910), race 1: rows 13-21.
    E2,
    /// `ccEnemy3` (`ccEntryEnemy3`, 0x004402d0), race 2: rows 22-34.
    E3,
    /// `ccEnemy4` (`ccEntryEnemy4`, 0x00440b60), race 3: the witches, rows
    /// 35-47.
    E4,
    /// `ccEnemyA` (`ccEntryEnemyA`, 0x00441550), race 4: the golems, rows
    /// 48-65.
    A,
    /// `ccEnemyB` (`ccEntryEnemyB`, 0x004420e0), race 5: rows 66-78.
    B,
    /// `ccEnemyC` (`ccEntryEnemyC`, 0x00442a80), race 6: the crabs and
    /// scorpions, rows 79-88.
    C,
    /// `ccEnemyD` (`ccEntryEnemyD`, 0x004438e0), race 7: the bats and
    /// eyes, rows 89-103.
    D,
    /// `ccEnemyE` (`ccEntryEnemyE`, 0x00444810), race 8: the rocks and
    /// turtles, rows 104-115.
    E,
    /// `ccEnemyF` (`ccEntryEnemyF`, 0x004454a0), race 9: the fish, rows
    /// 116-128.
    F,
    /// `ccEnemyG` (`ccEntryEnemyG`, 0x00445fc0), race 10: goblins, rows
    /// 129-166.
    G,
    /// `ccEnemyH` (`ccEntryEnemyH`, 0x004492d0), race 11: the dogs, rows
    /// 167-177.
    H,
    /// `ccEnemyI` (`ccEntryEnemyI`, 0x00449e60), race 12: the menhirs,
    /// rows 178-183.
    I,
    /// `ccEnemyK` (`ccEntryEnemyK`, 0x0044a730), race 13: rows 184-192.
    K,
    /// `ccEnemyL` (`ccEntryEnemyL`, 0x0044b130), race 14: the snakoids,
    /// wyrms and dragons, rows 193-217 (not its type 3, rows 203-206).
    L,
    /// `ccEnemyP` (`ccEntryEnemyP`, 0x0044e4a0), race 15: rows 218-228.
    P,
    /// `ccEnemyS` (`ccEntryEnemyS`, 0x0044ee70), race 16: the snakes,
    /// rows 229-238.
    S,
    /// `ccEnemyT` (`ccEntryEnemyT`, 0x0044f940), race 17: the boxes, rows
    /// 239-244.
    T,
    /// `ccEnemyU` (`ccEntryEnemyU`, 0x00450330), race 18: the undead, rows
    /// 245-266.
    U,
    /// `ccEnemyV` (`ccEntryEnemyV`, 0x004513b0), race 19: rows 267-278.
    V,
    /// `ccEnemyW` (`ccEntryEnemyW`, 0x00451f90), race 20: the ghosts, rows
    /// 279-287.
    W,
    /// `ccEnemyZ` (`ccEntryEnemyZ`, 0x00452a00), race 21: rows 288-302.
    Z,
    /// A race whose `action()` is not ported (its group).
    Other(i32),
}

impl Kind {
    /// By the race group.
    pub fn of_race(race: i32) -> Kind {
        match race {
            0 => Kind::E1,
            1 => Kind::E2,
            2 => Kind::E3,
            3 => Kind::E4,
            4 => Kind::A,
            5 => Kind::B,
            6 => Kind::C,
            7 => Kind::D,
            8 => Kind::E,
            9 => Kind::F,
            10 => Kind::G,
            11 => Kind::H,
            12 => Kind::I,
            13 => Kind::K,
            14 => Kind::L,
            15 => Kind::P,
            16 => Kind::S,
            17 => Kind::T,
            18 => Kind::U,
            19 => Kind::V,
            20 => Kind::W,
            21 => Kind::Z,
            r => Kind::Other(r),
        }
    }

    /// The class `enemyTbl[id]` builds ([`crate::enemy_ai::enemy_race`]).
    pub fn of_row(t: &Tables, id: i32) -> Kind {
        crate::enemy_ai::enemy_race(t, id).map_or(Kind::Other(-1), |(race, _)| Kind::of_race(race))
    }

    /// Whether the race's `note()` runs the dust controller on notes 1 and
    /// 2 (`ccEnemyP`, `V`, `2`, `3`, `4`, `C`, `E`, `H`, `S`, `T` and `W`
    /// drive only the weapon trail).
    fn note_dust(self) -> bool {
        matches!(
            self,
            Kind::G
                | Kind::K
                | Kind::B
                | Kind::E1
                | Kind::F
                | Kind::I
                | Kind::U
                | Kind::A
                | Kind::D
                | Kind::Z
                | Kind::L
        )
    }

    /// Whether the race's `exclusive()` runs the dust controller
    /// (`ccEnemy2`, `3` and `4` only the weapon trail).
    fn excl_dust(self) -> bool {
        !matches!(self, Kind::E2 | Kind::E3 | Kind::E4)
    }
}

/// What the motion code reads from the game's image besides the rule
/// tables: each race's sound category and the animation names.
#[derive(Clone, Debug, Default)]
pub struct MotionData {
    /// `ccEntryRaceTbl[race].seCategory` (gcmn 0x005f1d60, +8 of each
    /// 12-byte row).
    pub se_category: Vec<i32>,
    /// Each `enemyTbl` row's `ccEntry.anm` names (a `char[][30]`), at most
    /// [`CLIP_SLOTS`].
    clips: Vec<Vec<Vec<u8>>>,
}

/// How many slots of each animation table [`MotionData`] keeps: the
/// attacks 0-5, wait 6, walk 7, the flinches 8 and 9, dying 10, and some.
pub const CLIP_SLOTS: u32 = 16;

impl MotionData {
    /// The volume's (`piney_data::tables::battle`): the races' sound
    /// categories and the rows' animation names.
    pub fn of(volume: piney_data::volume::Volume, t: &Tables) -> MotionData {
        let races = piney_data::tables::battle::of(volume).races();
        let se_category = races.iter().map(|r| r.se_category).collect();
        let clips = t
            .enemies
            .iter()
            .map(|row| {
                let names = row.anm.unwrap_or_default();
                names.iter().take(CLIP_SLOTS as usize).map(|s| encode(s)).collect()
            })
            .collect();
        MotionData { se_category, clips }
    }

    /// From the disc's volume.
    pub fn read_iso(iso: &mut Iso, t: &Tables) -> Result<MotionData> {
        Ok(MotionData::of(iso.volume()?, t))
    }

    /// The name `strcpy` takes from `anmTbl + 30 anmNum`: `anm_tbl` is the
    /// row + 1 (empty for none, or a slot past the table).
    pub fn clip(&self, anm_tbl: u32, anm_num: i16) -> Vec<u8> {
        let row = (anm_tbl as usize).checked_sub(1);
        let k = usize::try_from(anm_num).ok();
        row.zip(k).and_then(|(r, k)| self.clips.get(r)?.get(k).cloned()).unwrap_or_default()
    }

    fn category(&self, race: i32) -> i32 {
        usize::try_from(race).ok().and_then(|r| self.se_category.get(r)).copied().unwrap_or(0)
    }
}

/// `ccGetNameBossAnm(name, out)` (gcmn 0x0042e580): a middle boss's second
/// model plays the clip of the same name with its eighth letter `x`.
pub fn boss_clip(name: &[u8]) -> Vec<u8> {
    let mut v = name.to_vec();
    if v.len() > 7 {
        v[7] = b'x';
    }
    v
}

// The act helpers ----------------------------------------------------------------

/// `ccEnemy::actMove(targetMd, paramMd, targetDd, paramDd, targetSpd,
/// paramSpd, paramReduce)` (0x004371e0): with a reduction, the target
/// speed falls with how far the walk still has to turn (`mdirc.z` turned
/// 0.3 of the way to `targetMd`, the difference over `pi reduce`), not
/// below 0; then the speed moves `paramSpd` of the way to it, the walk's
/// heading `mdirc.z` `paramMd` of the way to `targetMd` and the facing
/// `dirc.z` `paramDd` of the way to `targetDd` (each only when its rate is
/// not 0; a negative rate turns away).
#[allow(clippy::too_many_arguments)]
pub fn act_move(e: &mut Enemy, t_md: F, p_md: F, t_dd: F, p_dd: F, t_spd: F, p_spd: F, p_red: F) {
    let mut t_spd = t_spd;
    if !eq(p_red, 0) {
        let old = e.mdirc[2];
        let mut r = old;
        set_rad(&mut r, t_md, if lt(p_md, 0) { K_M0_3 } else { K_0_3 });
        let mut d = pi_limit(sub(r, old));
        if lt(d, 0) {
            d = neg(d);
        }
        t_spd = sub(t_spd, mul(t_spd, div(d, mul(PI, p_red))));
        if lt(t_spd, 0) {
            t_spd = 0;
        }
    }
    if !eq(p_spd, 0) {
        set_dist(&mut e.speed, t_spd, p_spd);
    }
    if !eq(p_md, 0) {
        set_rad(&mut e.mdirc[2], t_md, p_md);
    }
    if !eq(p_dd, 0) {
        set_rad(&mut e.dirc[2], t_dd, p_dd);
    }
}

/// `ccEnemy::actFollow(tMd, pMd, tSpd, pSpd, pRed)` (0x00437420): walk and
/// face the same way, `actMove(tMd, pMd, tMd, pMd, tSpd, pSpd, pRed)`.
pub fn act_follow(e: &mut Enemy, t_md: F, p_md: F, t_spd: F, p_spd: F, p_red: F) {
    act_move(e, t_md, p_md, t_md, p_md, t_spd, p_spd, p_red);
}

/// `ccEnemy::actSlide(tMd, pMd, tSpd, pSpd, pRed)` (0x00437460): walk the
/// way it faces and turn to `tMd`, `actMove(dirc.z, pMd, tMd, pMd, tSpd,
/// pSpd, pRed)`.
pub fn act_slide(e: &mut Enemy, t_md: F, p_md: F, t_spd: F, p_spd: F, p_red: F) {
    let face = e.dirc[2];
    act_move(e, face, p_md, t_md, p_md, t_spd, p_spd, p_red);
}

/// The heading an act follows.
#[derive(Clone, Copy)]
enum Head {
    /// `targetDirc`.
    Target,
    /// `baseDirc`: home.
    Home,
}

/// `actFollow(heading, pMd, tSpd, pSpd, pRed)`, the heading and the target
/// speed read from the enemy as the race's code reads them.
fn follow(e: &mut Enemy, h: Head, p_md: F, t_spd: fn(&Enemy) -> F, p_spd: F, p_red: F) {
    let t_md = match h {
        Head::Target => e.target_dirc,
        Head::Home => e.base_dirc,
    };
    let s = t_spd(e);
    act_follow(e, t_md, p_md, s, p_spd, p_red);
}

/// `actFollow(targetDirc, pMd, tSpd, pSpd, 0)`.
fn follow_at(e: &mut Enemy, p_md: F, t_spd: F, p_spd: F) {
    let td = e.target_dirc;
    act_follow(e, td, p_md, t_spd, p_spd, 0);
}

/// Standing: speed 0.
fn stand(_: &Enemy) -> F {
    0
}

/// `maxSpd`.
fn full(e: &Enemy) -> F {
    e.max_spd
}

/// `0.5 maxSpd`.
fn half(e: &Enemy) -> F {
    mul(K_0_5, e.max_spd)
}

/// `0.6 maxSpd`.
fn six_tenths(e: &Enemy) -> F {
    mul(K_0_6, e.max_spd)
}

// The frame's context ------------------------------------------------------------

/// The motion code at work: the enemies' rules (the scene, every enemy's
/// state, both generators), the world, and the image's data.
pub struct Motion<'m, 'a, W: MotionWorld + ?Sized> {
    pub ai: &'m mut Ai<'a>,
    pub w: &'m mut W,
    pub data: &'m MotionData,
    /// What the affects the frame applies need: the party, the menu,
    /// `ccSkillCheck` (the runtime's [`crate::flow::Skills::check`]).
    pub affect: &'m AffectCtx<'m>,
    /// The environment `CalcReal` and the damage read.
    pub env: &'m Env,
}

impl<W: MotionWorld + ?Sized> Motion<'_, '_, W> {
    fn e(&mut self, me: usize) -> &mut Enemy {
        self.ai.foes[me].as_mut().expect("an enemy's state")
    }

    fn en(&self, me: usize) -> &Enemy {
        self.ai.foes[me].as_ref().expect("an enemy's state")
    }

    /// A call to the world, with the scene and generators at hand.
    fn call(&mut self, me: usize, c: Call) {
        let ai = &mut *self.ai;
        let mut at = At { t: ai.t, scene: &mut *ai.scene, rand: &mut *ai.rand, cc: &mut *ai.cc };
        self.w.call(me, c, &mut at);
    }

    /// The rules' outputs since the last flush, in order: affects applied
    /// where the game calls `EntryAffect` ([`Motion::entry_affect`]), the
    /// rest handed to the world. Every function here flushes after the
    /// rules it calls; a runtime calling [`Ai`]'s rules itself flushes
    /// after them.
    pub fn flush(&mut self, me: usize) {
        for o in std::mem::take(&mut self.ai.out) {
            match o {
                Out::Rule(ev) => self.event(me, ev),
                // targetPtr->EntryAffect(this, 5, 0, 0, 0): on a null
                // target the call finds nobody on the lists.
                Out::Hold { target } => {
                    if let Some(t) = target {
                        self.entry_affect(me, t, Some(me), 5, [0; 3]);
                    }
                }
                o => self.call(me, Call::Rule(o)),
            }
        }
    }

    /// One event of a rule, `Who::Me` being enemy `me`: an affect is applied
    /// at once, an enemy's retargeting (`selectTarget()` in `affectEnemy`)
    /// run at once, anything else handed on.
    fn event(&mut self, me: usize, ev: Event) {
        let idx = |w: Who| match w {
            Who::Me => Some(me),
            Who::Char(i) => Some(i),
            _ => None,
        };
        match ev {
            Event::Affect { on, by, kind, p } => {
                if let Some(on) = idx(on) {
                    self.entry_affect(me, on, idx(by), kind, p);
                }
            }
            Event::EnemyRetarget(w) => {
                if let Some(w) = idx(w).filter(|&w| self.ai.foes.get(w).is_some_and(Option::is_some)) {
                    self.ai.select_target(w);
                    self.flush(me);
                }
            }
            ev => self.call(me, Call::Rule(Out::Rule(ev))),
        }
    }

    /// `ccChar::EntryAffect(by, kind, p0, p1, p2)` (gcmn 0x0056b020) on
    /// `on` where the game calls it inside the frame
    /// ([`crate::affect::entry_affect`]), with the enemy's own copy of what
    /// it leaves kept as `ccEnemyInfluence` (0x00432840) would leave it:
    /// the kind and first parameter once stored, `affectFlag` (and a
    /// drain's `drainFlag`) once the enemy's influence ran. The events it
    /// makes follow in order.
    pub fn entry_affect(&mut self, me: usize, on: usize, by: Option<usize>, kind: i16, p: [i16; 3]) {
        let lands = affect_lands(self.ai.scene, on, kind);
        let mut ev = Events::new();
        affect::entry_affect(self.ai.t, self.ai.scene, self.affect, on, by, kind, p, self.ai.rand, &mut ev);
        if let Some(e) = self.ai.foes.get_mut(on).and_then(Option::as_mut) {
            e.note_landed(lands, kind, p[0]);
        }
        for x in ev {
            self.event(me, x);
        }
    }

    /// `ccSkillDamage(this, skillTarget, skillParam, 0)` (gcmn 0x00573dc0,
    /// the 5-argument `ccSkillDamage` 0x00573e60 with no attribute
    /// critical) as `affectSkill` calls it: the rules of
    /// [`crate::damage::skill_damage`], each character's `EntryAffect(1,
    /// dmg, 0)` applied right after its roll as the game applies it (so the
    /// rolls of an area skill interleave with the hurt acts' draws).
    pub fn skill_damage(&mut self, me: usize, target: usize, sk: &SkillParam) -> i32 {
        let scene = &*self.ai.scene;
        let ad = scene.chars[me].cond[cond::DEAD];
        if !scene.listed(me) || !(ad == 0 || ad == 1) || !scene.listed(target) || scene.chars[target].dead() {
            return 0;
        }
        let splash = sk.ty & bits::SPLASH_HALF != 0;
        let tty = scene.chars[target].ty();
        if le(sk.target_range, 0) {
            self.hit(me, target, sk, ONE);
            return 1;
        }
        let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { scene.chars[me].pos_p } else { scene.chars[target].pos_p };
        let list = if tty & 6 != 0 {
            scene.pc_list.clone()
        } else if tty & ty::FOE != 0 {
            scene.ene_list.clone()
        } else {
            return 0;
        };
        // Each is tested as the loop reaches it, after the affects before it.
        let mut n = 0;
        for c in list {
            let ch = &self.ai.scene.chars[c];
            if tty & ch.ty() == 0
                || ch.cond[cond::DEAD] != 0
                || !le(damage::ground_distance(ch, centre), sk.target_range)
            {
                continue;
            }
            let mag = if splash && c != target { damage::F_HALF } else { ONE };
            self.hit(me, c, sk, mag);
            n += 1;
        }
        n
    }

    /// One character of [`Motion::skill_damage`]: `CalcBattleDamage` (its
    /// events handed on) and `EntryAffect(1, dmg, 0)`.
    fn hit(&mut self, me: usize, c: usize, sk: &SkillParam, mag: F) {
        let mut ev = Events::new();
        let r = damage::calc_damage_in(
            self.ai.t,
            self.ai.scene,
            me,
            c,
            sk,
            mag,
            Roll::Draw,
            self.ai.rand,
            self.env,
            &mut ev,
        );
        for x in ev {
            self.event(me, x);
        }
        self.entry_affect(me, c, Some(me), 1, [r.dmg as i16, 0, 0]);
    }

    fn cc(&mut self) -> i32 {
        self.ai.cc.rand()
    }

    fn think(&self, me: usize) -> crate::param::ThinkParam {
        self.ai.t.enemies[self.en(me).ene_id as usize].think
    }

    fn size(&self, me: usize) -> i32 {
        object_size(self.ai.t, &self.en(me).ent)
    }

    /// `ccEnemy::actEscape()` (0x004374a0): the first row of a race (`raceId`
    /// 0) backs off with [`Motion::act_escape_x`]; the others, on the act's
    /// first frame, toss two coins (`optFlag0`, `optFlag1`: which way) and
    /// back off by them ([`Motion::act_escape_by`]: 0 on the first coin's
    /// tails, else 2 or 1 by the second's), the reduction by size (4: 1.0,
    /// 3: 0.8, else 0.4).
    pub fn act_escape(&mut self, me: usize) {
        if self.en(me).race_id == 0 {
            self.act_escape_x(me);
            return;
        }
        let red = match self.size(me) {
            4 => ONE,
            3 => K_0_8,
            _ => K_0_4,
        };
        if self.en(me).act_cnt == 0 {
            let a = self.cc() & 1 != 0;
            self.e(me).opt_flag0 = a;
            let b = self.cc() & 1 != 0;
            self.e(me).opt_flag1 = b;
        }
        let e = self.en(me);
        let ty = if !e.opt_flag0 {
            0
        } else if e.opt_flag1 {
            2
        } else {
            1
        };
        self.act_escape_by(me, ty, red);
    }

    /// The heading an escape runs on: straight away from the target
    /// (`targetDirc + pi`), scattered by how pressed it is (`crisisRate`):
    /// below 0.4 by `2pi/5 (1 - crisis) + ccRandF(pi/10)`, below 0.8 by
    /// `2pi/5 crisis`, the side by bit 1 of `eneRand`.
    fn escape_heading(&mut self, me: usize) -> F {
        let f = pi_limit(add(PI, self.en(me).target_dirc));
        let crisis = self.en(me).crisis_rate;
        let away = if lt(crisis, K_0_4) {
            let r = rand_f(self.ai.cc, K_PI_10);
            let e = self.en(me);
            let mut v = add(mul(K_2PI_5, sub(ONE, e.crisis_rate)), r);
            if (e.ene_rand >> 1) & 1 != 0 {
                v = mul(v, MINUS_ONE);
            }
            v
        } else if lt(crisis, K_0_8) {
            let e = self.en(me);
            let mut v = mul(K_2PI_5, crisis);
            if (e.ene_rand >> 1) & 1 == 0 {
                v = mul(v, MINUS_ONE);
            }
            v
        } else {
            0
        };
        pi_limit(add(f, away))
    }

    /// `ccEnemy::actEscape(escType, escReduce)` (0x00437610): back off from
    /// the target. A close range under 100 counts as pressed (`crisisRate`
    /// 0.8); the first row of a race uses [`Motion::act_escape_x`]; held,
    /// the reduction is divided by 1.5. The speed aims at `maxSpd (1 +
    /// (10 crisis)^2 / 100)` on the `Motion::escape_heading`: type 2
    /// slides facing on (turning to the heading), 1 walks the heading
    /// facing the target, else walks and faces the heading; the rates scale
    /// with the reduction.
    pub fn act_escape_by(&mut self, me: usize, ty: i32, red: F) {
        if lt(self.think(me).atk_range_a, K_100) {
            self.e(me).crisis_rate = K_0_8;
        }
        if self.en(me).race_id == 0 {
            self.act_escape_x(me);
            return;
        }
        let mut red = red;
        if self.ai.scene.chars[me].cond[cond::HOLD] != 0 {
            red = div(red, K_1_5);
        }
        let e = self.en(me);
        let c10 = mul(K_10, e.crisis_rate);
        let q = div(mul(ONE, mul(c10, c10)), K_100);
        let spd = add(e.max_spd, mul(e.max_spd, q));
        let h = self.escape_heading(me);
        let e = self.e(me);
        let crisis = e.crisis_rate;
        let slow = mul(div(crisis, K_10), red);
        match ty {
            2 => {
                let face = e.dirc[2];
                act_move(e, face, mul(K_0_05, red), h, mul(K_0_05, red), spd, slow, red)
            }
            1 => {
                let td = e.target_dirc;
                act_move(e, h, mul(div(crisis, K_5), red), td, mul(K_0_05, red), spd, slow, red)
            }
            _ => act_move(e, h, mul(div(crisis, K_5), red), h, mul(K_0_1, red), spd, slow, red),
        }
    }

    /// `ccEnemy::actEscapeX()` (0x00437960): after 60 frames of backing off,
    /// attack if an attack is ready (turning to the target) or stop; until
    /// then run on the `Motion::escape_heading` at `maxSpd (1 + (1.1 +
    /// ccRandF(0.2)) (10 crisis)^2 / 100)`.
    pub fn act_escape_x(&mut self, me: usize) {
        if self.en(me).act_cnt >= 61 {
            let ok = self.ai.select_attack(me);
            self.flush(me);
            if ok {
                self.ai.set_act(me, act::ATTACK);
                self.flush(me);
                let e = self.e(me);
                let td = e.target_dirc;
                act_move(e, td, K_0_3, td, K_0_3, 0, K_0_3, 0);
            } else {
                self.ai.set_act(me, act::STOP);
                self.flush(me);
            }
            return;
        }
        let r = rand_f(self.ai.cc, K_0_2);
        let f2 = add(K_1_1, r);
        let e = self.en(me);
        let c10 = mul(K_10, e.crisis_rate);
        let q = div(mul(f2, mul(c10, c10)), K_100);
        let spd = add(e.max_spd, mul(e.max_spd, q));
        let h = self.escape_heading(me);
        let e = self.e(me);
        let crisis = e.crisis_rate;
        act_move(e, h, K_0_2, h, K_0_5, spd, crisis, K_2);
    }

    /// `ccSetRadDisperse(&targetDirc, pi/2)` every 16th frame of the act:
    /// a wanderer's new heading.
    fn wander_heading(&mut self, me: usize) {
        self.wander_by(me, HALF_PI);
    }

    /// [`Motion::wander_heading`] by `x` (`ccEnemyA` pi/4, `ccEnemyW` pi).
    fn wander_by(&mut self, me: usize, x: F) {
        if self.en(me).act_cnt & 0xf == 0 {
            let v = rad_disperse(self.en(me).target_dirc, x, self.ai.cc);
            self.e(me).target_dirc = v;
        }
    }

    /// An attack's count (`atkCnt`, 0 on the act's first frame) and end
    /// (`actionFlag` once the animation ended).
    fn attack_count(&mut self, me: usize) {
        let e = self.e(me);
        if e.act_cnt == 0 {
            e.atk_cnt = 0;
        } else {
            e.atk_cnt = e.atk_cnt.wrapping_add(1);
            if e.anm_flag != 0 {
                e.action_flag = true;
            }
        }
    }

    /// A flinch's clip (8 or 9 by a coin, on its first frame) and end.
    fn flinch_clip(&mut self, me: usize) {
        if self.en(me).act_cnt == 0 {
            let r = self.cc() & 1;
            self.e(me).anm_num = (r + 8) as i16;
        }
        let e = self.e(me);
        if e.anm_flag != 0 {
            e.action_flag = true;
        }
    }

    /// An act's clip, set on its first frame.
    fn clip_on_start(&mut self, me: usize, clip: i16) {
        let e = self.e(me);
        if e.act_cnt == 0 {
            e.anm_num = clip;
        }
    }

    /// Closing in: two coins (`optFlag0`, `optFlag1`) for the way to back
    /// off.
    fn coins(&mut self, me: usize) {
        let a = self.cc() & 1 != 0;
        self.e(me).opt_flag0 = a;
        let b = self.cc() & 1 != 0;
        self.e(me).opt_flag1 = b;
    }

    // moveEnemy ----------------------------------------------------------------

    /// `ccEnemy::moveEnemy()` (0x00433e80): the step along `mdirc.z` at
    /// `speed`, through the player's frame and onto the ground, tried against
    /// the other bodies ([`World::collide`], up to six times) and then the
    /// walls ([`World::line`] kind 1): only with nothing in the way does it
    /// move. A blocked walk swerves and counts `hitCnt`; an unblocked one
    /// tosses a `ccRand` coin into `optFlag1`. The rules are in
    /// docs/engine/battle.md ("moveEnemy").
    pub fn move_enemy(&mut self, me: usize) {
        let (width, height) = {
            let b = self.ai.scene.chars[me].base();
            (b.width, b.height)
        };
        let pos = self.ai.scene.chars[me].pos;
        let mut s2: i32 = 128;
        let fast = div(self.en(me).speed, K_30);
        if !le(fast, ONE) {
            let e = self.e(me);
            e.hit.radius = mul(e.hit.radius, add(K_0_6, fast));
            if !le(e.hit.radius, K_600) {
                e.hit.radius = K_600;
            }
            s2 = 32;
        }
        let mut f20 = self.en(me).speed;
        let mut tmp = self.en(me).mdirc[2];
        let mut s1: i32 = 0;
        let mut s0: i32 = 0;
        let mut f21: F = 0;
        let mut np: V4 = [0; 4];
        let mut pp: V4;
        while s1 < 6 {
            let mut mv = VF0;
            mv[0] = mul(f20, sinf(tmp));
            mv[1] = neg(mul(f20, cosf(tmp)));
            pp = self.w.w2p(pos);
            pp = vadd(pp, mv);
            pp[3] = ONE;
            np = self.w.p2w(pp);
            let zoffs = self.en(me).zoffs;
            np[2] = sub(np[2], zoffs);
            let h = self.w.land(np, GROUND_MASK);
            let z = add(zoffs, h);
            np[2] = z;
            let e = self.ai.foes[me].as_mut().expect("an enemy's state");
            e.hit.pos = np;
            s0 = self.w.collide(me, &mut e.hit);
            if s0 == 0 {
                break;
            }
            np = vadd(mv, e.hit.offset);
            np = vadd(np, pos);
            np[3] = ONE;
            f21 = get_dirc(pos, np);
            tmp = set_dirc(tmp, f21, s2);
            f20 = mul(f20, sub(ONE, mul(K_WIDTH_DRAG, width)));
            if s1 == 4 && !le(e.hit.radius, K_200) {
                e.hit.radius = K_200;
                tmp = set_dirc(tmp, e.mdirc[2], s2 >> 1);
                f20 = mul(f20, K_2);
                s2 >>= 2;
            } else {
                e.hit.radius = mul(e.hit.radius, K_0_9);
            }
            if !lt(height, K_300) {
                e.hit.height = mul(e.hit.height, K_0_9);
            }
            s1 += 1;
        }
        let end = np;
        pp = self.w.w2p(pos);
        np = self.w.p2w(pp);
        let half = div(height, K_2);
        let mut a = np;
        let mut b = end;
        a[2] = add(a[2], half);
        b[2] = add(b[2], half);
        let r = self.w.line(a, b, WALL_MASK, 1);
        let mut mv = if eq(MINUS_ONE, r) { vsub(end, np) } else { [0; 4] };
        np = vadd(np, mv);
        pp = vadd(pp, mv);
        if s1 != 0 {
            if s0 >= 2 {
                set_dist(&mut self.e(me).speed, 0, K_0_1);
                mv = vscale(mv, K_0_5);
                np = vsub(np, mv);
                pp = vsub(pp, mv);
            }
            let chg = get_dirc_chg_f(tmp, f21, s2);
            let mut sw = mul(K_SWERVE, from_int(s1));
            if lt(chg, 0) {
                sw = mul(sw, MINUS_ONE);
            }
            let e = self.en(me);
            let mut share = div(e.speed, e.max_spd);
            if !le(share, ONE) {
                share = ONE;
            }
            sw = mul(sw, share);
            sw = mul(sw, add(ONE, mul(K_0_05, from_int(e.hit_cnt))));
            let (r0, r1) = match self.size(me) {
                4 => (s2 >> 2, s2),
                3 => (s2 >> 1, s2 << 1),
                _ => (s2 << 2, s2 << 4),
            };
            let e = self.e(me);
            let to = pi_limit(add(e.mdirc[2], sw));
            if !eq(e.speed, 0) {
                e.mdirc[2] = set_dirc(e.mdirc[2], tmp, r0);
                e.mdirc[2] = set_dirc(e.mdirc[2], to, r1);
            }
        }
        if s1 == 0 {
            self.e(me).hit_cnt = 0;
            let c = self.cc() & 1 != 0;
            self.e(me).opt_flag1 = c;
        } else {
            let e = self.e(me);
            e.hit_cnt = e.hit_cnt.wrapping_add(1);
        }
        let e = self.e(me);
        if s1 != 0 {
            for _ in 0..s1 {
                set_dist(&mut e.hit_spd, K_0_8, K_0_06);
            }
        } else {
            set_dist(&mut e.hit_spd, ONE, K_0_06);
        }
        e.hit.radius = width;
        e.hit.height = div(height, K_2);
        let ch = &mut self.ai.scene.chars[me];
        ch.pos = np;
        ch.pos_p = pp;
        self.e(me).hit.pos = np;
    }

    // animEnemy ----------------------------------------------------------------

    /// `ccEnemy::animEnemy()` (0x00434560): a new `anmNum` plays its clip
    /// (`anmTbl + 30 anmNum`; a middle boss's second model its
    /// [`boss_clip`]) at a frame speed by `moveFlag` ([`Ai::anim_retarget`]
    /// for 2), at least 64; `anmFlag` is whether a play-once clip ended, and
    /// the notes the step passed go to [`Motion::check_note`].
    pub fn anim_enemy(&mut self, me: usize) {
        let e = self.en(me);
        if e.anm_num_old != e.anm_num {
            let name = self.data.clip(e.anm_tbl, e.anm_num);
            let second = e.ccs2_flag;
            self.w.anim_set(me, AnmSlot::Main, &String::from_utf8_lossy(&name));
            if second {
                self.w.anim_set(me, AnmSlot::Second, &String::from_utf8_lossy(&boss_clip(&name)));
            }
        }
        let mut step: u32 = 256;
        let spd = self.think(me).anm_spd;
        if !eq(ONE, spd) {
            let e = self.en(me);
            match e.move_flag {
                2 => {
                    let d = self.ai.anim_retarget(me);
                    self.flush(me);
                    let mut f = div(d, K_400);
                    if !le(f, ONE) {
                        f = ONE;
                    }
                    if lt(f, 0) {
                        f = 0;
                    }
                    f = sub(ONE, f);
                    step = fptoui(add(K_256, mul(K_512, f))) & 0xffff;
                }
                1 => {
                    step = fptoui(mul(spd, mul(K_256, div(e.speed, e.max_spd)))) & 0xffff;
                }
                _ => {
                    if self.ai.scene.chars[me].cond[cond::DEAD] == 0 && e.act_num != act::ATTACK {
                        step = fptoui(mul(K_256, add(ONE, e.crisis_rate))) & 0xffff;
                    }
                }
            }
        }
        if step < 64 {
            step = 64;
        }
        let step = step as u16;
        let frame = self.w.anim_frame(me, AnmSlot::Main);
        self.e(me).frame_num = frame;
        let ended = self.w.anim_forward(me, AnmSlot::Main, step);
        self.e(me).anm_flag = ended;
        for n in self.w.anim_notes(me, AnmSlot::Main) {
            self.check_note(me, n);
        }
        let e = self.e(me);
        e.anm_num_old = e.anm_num;
        if e.ccs2_flag {
            self.w.anim_forward(me, AnmSlot::Second, step);
        }
    }

    /// `ccEnemyCheckNote(note)` (0x004328b0), each note of the enemy's
    /// animation (`ccEnemy::noteEnemy`, 0x00434880): 1 and 2 a sound of the
    /// race's category, 0x8005 the attack lands ([`Ai::affect_skill`]),
    /// 0x8003 it starts ([`Ai::start_skill`]), 0x8002 a camera shake of the
    /// note's strength (0-2) when near the camera; then the race's
    /// `note()`.
    pub fn check_note(&mut self, me: usize, n: Note) {
        let race = self.en(me).race;
        match n.event {
            1 | 2 => {
                let category = self.data.category(race);
                self.call(me, Call::Sound { param: n.param, category });
            }
            0x8005 => {
                let hit = self.ai.affect_skill(me);
                self.flush(me);
                if let Some((target, skill)) = hit {
                    let sk = skill.get(self.ai.t, self.en(me).ene_id).cloned().unwrap_or_default();
                    self.skill_damage(me, target, &sk);
                }
            }
            0x8003 => {
                let used = self.ai.start_skill(me);
                self.flush(me);
                if let SkillUse::Request { target, sid } = used {
                    self.call(me, Call::SkillRequest { target, sid });
                }
            }
            0x8002 => {
                let pos = self.ai.scene.chars[me].pos;
                if self.w.shake_range(pos) && n.param <= 2 {
                    self.call(me, Call::CameraShake { kind: n.param as i32 });
                }
            }
            _ => {}
        }
        self.race_note(me, n);
    }

    /// The race's `note(note)`: `ccEnemyG::note` (0x00449240),
    /// `ccEnemyK::note` (0x0044b0a0), `ccEnemyB::note` (0x004429f0),
    /// `ccEnemy1::note` (0x0043f880), `ccEnemyF::note` (0x00445f30),
    /// `ccEnemyI::note` (0x0044a6a0) and `ccEnemyU::note` (0x00451310)
    /// drive the weapon trail and, on notes 1 and 2, the dust;
    /// `ccEnemyP::note` (0x0044ee30), `ccEnemyV::note` (0x00451f50),
    /// `ccEnemy2::note` (0x00440290), `ccEnemyC::note` (0x004438a0) and
    /// `ccEnemyH::note` (0x00449e20) only the weapon trail.
    pub fn race_note(&mut self, me: usize, n: Note) {
        let kind = Kind::of_row(self.ai.t, self.en(me).ene_id);
        let (wc, dc) = (self.en(me).weapon.is_some(), self.en(me).dust.is_some());
        if matches!(kind, Kind::Other(_)) {
            return;
        }
        if wc {
            let at = self.weapon_at(me);
            self.call(me, Call::WeaponNote { note: n, at });
        }
        if kind.note_dust() && (n.event == 1 || n.event == 2) && dc {
            let at = self.dust_at(me);
            self.call(me, Call::DustCtrl { flag: 1, at });
        }
        let e = self.en(me);
        if kind == Kind::E && e.ene_type == 3 && e.anm_num == 7 && (n.event == 1 || n.event == 2) {
            let first = if e.frame_num < 46 { 0 } else { 2 };
            for foot in first..first + 2 {
                let smoke = self.en(me).ene_smoke;
                self.call(me, Call::FootDust { foot, info: InfoRef::new(EET1_DUST_INFO, 8 + foot), smoke });
            }
        }
    }

    // dispEnemy ----------------------------------------------------------------

    /// The model's matrix `dispEnemy` builds: the offset `(yoffs, 0, 0)`
    /// turned a quarter turn back about z, then by the facing (`dirc`,
    /// `sceVu0RotMatrix`), added to the position; the facing's rotation
    /// moved there (`sceVu0TransMatrix`).
    pub fn model_matrix(&self, me: usize) -> M4 {
        let e = self.en(me);
        let m1 = rot_matrix_z(&unit_matrix(), deg2rad(-16384));
        let v = apply_matrix(&m1, [e.yoffs, 0, 0, ONE]);
        let mat = rot_matrix(&unit_matrix(), e.dirc);
        let off = apply_matrix(&mat, v);
        let mut at = vadd(self.ai.scene.chars[me].pos, off);
        at[3] = ONE;
        trans_matrix(&mat, at)
    }

    /// `ccEnemy::dispEnemy()` (0x004348b0), when `dispSW` is on: the model's
    /// matrix ([`Motion::model_matrix`]); a middle boss's second model
    /// drawn at the camera's transparency (`ccGetCameraTransparency(pos,
    /// width, height, 7000, 600)`) times `setTransparency`; then
    /// `ccChar::Draw()`.
    pub fn disp_enemy(&mut self, me: usize) {
        let matrix = self.model_matrix(me);
        if self.en(me).ccs2_flag {
            let ch = &self.ai.scene.chars[me];
            let (pos, width, height) = (ch.pos, ch.base().width, ch.base().height);
            let t = self.w.camera_transparency(pos, width, height, K_7000, K_600);
            let alpha = mul(t, self.en(me).set_transparency);
            self.call(me, Call::DrawSecond { matrix, alpha });
        }
        self.call(me, Call::Draw { matrix });
        self.e(me).anm_lw = matrix;
    }

    // The races ----------------------------------------------------------------

    /// The race's `action()`: [`Motion::action_g`], [`Motion::action_p`],
    /// [`Motion::action_k`], [`Motion::action_v`], [`Motion::action_b`],
    /// [`Motion::action_1`], [`Motion::action_2`], [`Motion::action_c`],
    /// [`Motion::action_f`], [`Motion::action_h`], [`Motion::action_i`],
    /// [`Motion::action_u`], [`Motion::action_3`], [`Motion::action_4`],
    /// [`Motion::action_a`], [`Motion::action_d`], [`Motion::action_e`],
    /// [`Motion::action_s`], [`Motion::action_t`], [`Motion::action_w`] and
    /// `ccEnemyZ` as [`Motion::action_b`]; nothing for `ccEnemyL`.
    pub fn action(&mut self, me: usize) {
        match Kind::of_row(self.ai.t, self.en(me).ene_id) {
            Kind::G => self.action_g(me),
            Kind::P => self.action_p(me),
            Kind::K => self.action_k(me),
            Kind::V => self.action_v(me),
            Kind::B => self.action_b(me),
            Kind::E1 => self.action_1(me),
            Kind::E2 => self.action_2(me),
            Kind::C => self.action_c(me),
            Kind::F => self.action_f(me),
            Kind::H => self.action_h(me),
            Kind::I => self.action_i(me),
            Kind::U => self.action_u(me),
            Kind::E3 => self.action_3(me),
            Kind::E4 => self.action_4(me),
            Kind::A => self.action_a(me),
            Kind::D => self.action_d(me),
            Kind::E => self.action_e(me),
            Kind::S => self.action_s(me),
            Kind::T => self.action_t(me),
            Kind::W => self.action_w(me),
            Kind::Z => self.action_b(me),
            Kind::L => self.action_l(me),
            Kind::Other(_) => {}
        }
        self.flush(me);
    }

    fn walk_flag(&mut self, me: usize) {
        let e = self.e(me);
        e.move_flag = u8::from(e.anm_num == 7);
    }

    /// `ccEnemyG::action()` (0x004467e0): the clips of the acts (wait 6, the
    /// walks 7, a flinch 8 or 9, dying 10), the attack's count and end, then
    /// the movement: `moveEG` ([`Motion::move_eg`]) or, for `eneType` 5 and
    /// 6, `moveEGG` ([`Motion::move_egg`]); `moveFlag` 1 while walking. A
    /// gold goblin moves with `moveGold` ([`Motion::move_gold`]) and sets
    /// `moveFlag` 2 while attacking.
    pub fn action_g(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => self.clip_on_start(me, 6),
            1..=4 => self.clip_on_start(me, 7),
            6 => self.attack_count(me),
            7 => self.flinch_clip(me),
            8 => self.clip_on_start(me, 10),
            _ => {}
        }
        if self.en(me).gold.flag {
            self.move_gold(me);
            let e = self.e(me);
            e.move_flag = if e.anm_num == 7 {
                1
            } else if e.act_num == act::ATTACK {
                2
            } else {
                0
            };
            return;
        }
        let ty = self.en(me).ene_type;
        if ty == 6 || ty == 5 {
            self.move_egg(me);
        } else {
            self.move_eg(me);
        }
        self.walk_flag(me);
    }

    /// `ccEnemyG::moveEG()` (0x00446aa0): the target's heading kept to its
    /// top two fraction bits (`& 0xffe00000`), then by act: wait turns to
    /// the target (or stops), close backs off ([`Motion::act_escape`]),
    /// wander follows a heading changed every 16 frames at half speed,
    /// chase and return run, stop and attack turn to the target, a flinch
    /// or dying stops.
    pub fn move_eg(&mut self, me: usize) {
        let e = self.e(me);
        e.target_dirc &= 0xffe0_0000;
        let tf = e.target_flag;
        match e.act_num {
            0 => {
                if tf {
                    follow(e, Head::Target, K_0_125, stand, 0, 0);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            1 => self.act_escape(me),
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_05, half, K_0_02, ONE);
            }
            3 => {
                if tf {
                    follow(e, Head::Target, K_0_0625, full, K_0_03, ONE);
                }
            }
            4 => follow(e, Head::Home, K_0_04, half, K_0_03, ONE),
            5 => {
                if tf {
                    follow(e, Head::Target, K_0_125, stand, K_0_3, 0);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            6 => {
                if tf {
                    follow(e, Head::Target, K_0_125, stand, K_0_08, 0);
                }
            }
            7 | 8 => set_dist(&mut e.speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemyG::moveGold()` (gcmn 0x00448520), a gold goblin's movement by
    /// act (`goldParam[2]`, `[3]` in hundredths): fleeing through
    /// `actEscapeGold` at a speed its hoard and the target's nearness set. The
    /// table is in docs/engine/battle.md ("The gold goblins").
    pub fn move_gold(&mut self, me: usize) {
        let hundredths = |v: i16| div(from_int(i32::from(v)), K_100);
        let act = self.en(me).act_num;
        match act {
            0 | 5 => {
                let e = self.e(me);
                if e.target_flag {
                    let p_spd = if act == 0 { K_0_2 } else { K_0_05 };
                    let t = e.target_dirc;
                    act_follow(e, t, K_0_125, 0, p_spd, ONE);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            1 => {
                let hold = self.ai.scene.chars[me].cond[cond::HOLD] != 0;
                if !hold {
                    let z = hundredths(self.en(me).gold.param[2]);
                    self.act_escape_gold(me, z);
                    return;
                }
                let mut zoom = hundredths(self.en(me).gold.param[3]);
                let base = K_2_5;
                let base = match self.en(me).gold.volume {
                    3 | 4 => {
                        let e = self.en(me);
                        let d = div(e.target_dist, K_500);
                        let cnt = from_int(e.gold.esc_cnt);
                        let base = if lt(d, ONE) {
                            add(base, div(div(mul(d, d), K_30), cnt))
                        } else {
                            sub(base, mul(div(d, K_30), cnt))
                        };
                        let mut c = sub(ONE, d);
                        if lt(c, K_0_7) {
                            c = K_0_7;
                        }
                        let r = rand_f(self.ai.cc, K_0_1);
                        self.e(me).crisis_rate = add(c, r);
                        base
                    }
                    _ => {
                        if !le(zoom, base) {
                            zoom = base;
                        }
                        base
                    }
                };
                self.act_escape_gold(me, sub(zoom, base));
            }
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                let (t, s) = (e.target_dirc, mul(K_0_5, e.max_spd));
                act_follow(e, t, K_0_05, s, K_0_02, ONE);
            }
            3 => {
                let e = self.e(me);
                if e.target_flag {
                    let (t, s) = (e.target_dirc, e.max_spd);
                    act_follow(e, t, K_0_0625, s, K_0_03, ONE);
                }
            }
            4 => {
                let e = self.e(me);
                let (b, m, s, a) = (e.base_dirc, e.mdirc[2], e.gold.speed, e.gold.accel);
                act_move(e, b, K_0_05, m, K_0_2, s, a, ONE);
            }
            6 => {
                let mut z = hundredths(self.en(me).gold.param[2]);
                if !le(z, K_1_6) {
                    z = K_1_6;
                }
                self.act_escape_gold(me, sub(z, K_1_6));
            }
            7 => {
                let c = self.ai.scene.chars[me].cond;
                let still =
                    c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 || (c[cond::HOLD] != 0 && self.ai.world.puppet_show);
                if still {
                    set_dist(&mut self.e(me).speed, 0, K_0_5);
                    return;
                }
                match self.en(me).gold.volume {
                    3 | 4 => {
                        let mut z = hundredths(self.en(me).gold.param[3]);
                        if !le(z, K_3) {
                            z = K_3;
                        }
                        self.e(me).crisis_rate = K_0_8;
                        self.act_escape_gold(me, sub(z, K_1_5));
                        let e = self.e(me);
                        if e.act_cnt == 0 && e.gold.dis_hold == 0 {
                            e.gold.dis_hold = 1;
                        }
                    }
                    1 => set_dist(&mut self.e(me).speed, 0, K_0_5),
                    _ => self.act_escape_gold(me, K_M0_8),
                }
            }
            8 => set_dist(&mut self.e(me).speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemyG::actEscapeGold(zoom)` (gcmn 0x00448b10): running from the
    /// target along `pi + targetDirc`, dispersed by `goldParam[0]` (`goldDirc`
    /// on frames 2 mod 4), turning at a rate by `goldParam[1]` (`goldRotate` on
    /// frames 1 mod 4) and a speed by the volume and `crisisRate`, then
    /// `actMove(goldDirc, goldRotate, mdirc.z, 0.2, goldSpeed, goldAccel, 1)`.
    /// The formulas are in docs/engine/battle.md ("The gold goblins").
    fn act_escape_gold(&mut self, me: usize, zoom: F) {
        let e = self.en(me);
        let (p0, p1, c) = (e.gold.param[0], e.gold.param[1], e.crisis_rate);
        let away = pi_limit(add(PI, e.target_dirc));
        let dirc = match p0 {
            0 => add(away, rand_f(self.ai.cc, mul(K_0_3PI, sub(ONE, c)))),
            1 => {
                let k = sub(ONE, c);
                add(away, enemy_ai::rand_f2(self.ai.cc, mul(PI, k), mul(K_PI_10, k)))
            }
            _ => {
                if lt(c, K_0_3) {
                    let _ = rand_f(self.ai.cc, K_PI_10);
                }
                away
            }
        };
        if self.en(me).act_cnt & 3 == 2 {
            self.e(me).gold.dirc = dirc;
        }
        let mut rot = if p1 == 0 {
            let r = rand_f(self.ai.cc, mul(K_0_2, sub(ONE, c)));
            add(r, mul(K_0_5, r))
        } else {
            let a = add(K_0_1, rand_f(self.ai.cc, K_0_1));
            let base = div(c, K_5);
            let near = lt(self.en(me).target_dist, K_1200);
            let r = rand_f(self.ai.cc, mul(a, c));
            sub(base, if near { add(a, r) } else { add(mul(K_0_1, a), r) })
        };
        if self.ai.scene.chars[me].cond[cond::HOLD] != 0 {
            rot = mul(rot, K_0_25);
        }
        let e = self.e(me);
        if e.act_cnt & 3 == 1 {
            e.gold.rotate = rot;
        }
        let q = || div(mul(mul(mul(K_10, c), K_2), mul(mul(K_10, c), K_2)), K_400);
        let m = e.max_spd;
        if e.gold.volume == 1 {
            let f = if lt(zoom, 0) { sub(K_1_1, c) } else { c };
            e.gold.speed = add(m, mul(zoom, mul(m, f)));
            e.gold.accel = div(c, K_10);
        } else {
            e.gold.speed = add(m, mul(zoom, mul(m, q())));
            e.gold.accel = q();
        }
        let (d, r, md, s, a) = (e.gold.dirc, e.gold.rotate, e.mdirc[2], e.gold.speed, e.gold.accel);
        act_move(e, d, r, md, K_0_2, s, a, ONE);
    }

    /// `ccEnemyG::moveEGG()` (0x00446d50), `eneType` 5 and 6: as
    /// [`Motion::move_eg`] with their own rates; closing in backs off on one
    /// coin (type 6: 1 or 0 at 0.7; 5: 2 or 0 at 0.8); a walk that has to
    /// turn more than pi/4 turns its facing after it (`0.1 d / 0.45 pi`).
    pub fn move_egg(&mut self, me: usize) {
        let e = self.e(me);
        let tf = e.target_flag;
        match e.act_num {
            0 => {
                if tf {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                if e.ene_type == 6 {
                    follow(e, Head::Target, K_0_01, half, K_0_03, K_0_6);
                } else {
                    follow(e, Head::Target, K_0_02, half, K_0_03, K_0_8);
                }
            }
            3 => {
                if tf {
                    follow(e, Head::Target, K_0_0625, full, K_0_3, ONE);
                }
            }
            1 => {
                if e.act_cnt == 0 {
                    let c = self.cc() & 1 != 0;
                    self.e(me).opt_flag0 = c;
                }
                let e = self.en(me);
                // Types other than 5 and 6 never reach moveEGG.
                let (ty, red) = if e.ene_type == 6 {
                    (i32::from(e.opt_flag0), K_0_7)
                } else {
                    (if e.opt_flag0 { 2 } else { 0 }, K_0_8)
                };
                self.act_escape_by(me, ty, red);
            }
            4 => follow(e, Head::Home, K_0_04, six_tenths, K_0_3, ONE),
            5 => {
                if tf {
                    follow(e, Head::Target, K_0_05, stand, K_0_3, 0);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            6 => {
                if tf {
                    follow(e, Head::Target, K_0_05, stand, K_0_3, 0);
                }
            }
            7 | 8 => set_dist(&mut e.speed, 0, K_0_5),
            _ => {}
        }
        let e = self.e(me);
        if e.move_flag == 1 {
            let d = fabs_d(pi_limit(sub(e.mdirc[2], e.dirc[2])));
            if !le(d, K_PI_4) {
                let r = mul(K_0_1, div(d, K_0_45PI));
                set_rad(&mut e.dirc[2], e.mdirc[2], r);
            }
        }
    }

    /// `ccEnemyP::action()` (0x0044e860): wait turns to the target (or
    /// stops), close backs off on two coins, wander follows a heading
    /// changed every 16 frames at half speed (by `eneType`), chase runs,
    /// return goes home, stop turns to the target, the attack turns to it
    /// (row 218 lunges at 40 for 16 frames), a flinch and dying stop.
    pub fn action_p(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                self.act_escape(me);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                match e.ene_type {
                    2 => follow(e, Head::Target, K_0_005, half, K_0_03, ONE),
                    0 | 1 | 3 => follow(e, Head::Target, K_0_05, half, K_0_05, ONE),
                    _ => {}
                }
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, ONE);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_06, half, K_0_1, ONE);
            }
            5 => self.face_or_stop(me, K_0_3),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if e.ene_id == 218 {
                        let spd = if e.atk_cnt < 16 { K_40 } else { 0 };
                        let td = e.target_dirc;
                        act_follow(e, td, K_0_08, spd, K_0_3, 0);
                    } else {
                        follow(e, Head::Target, K_0_125, stand, K_0_3, 0);
                    }
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// Turn to the target at 0.08 standing (`actFollow(targetDirc, 0.08, 0,
    /// rate, 0)`), or slow to a stop without one.
    fn face_or_stop(&mut self, me: usize, rate: F) {
        let e = self.e(me);
        if e.target_flag {
            follow(e, Head::Target, K_0_08, stand, rate, 0);
        } else {
            set_dist(&mut e.speed, 0, rate);
        }
    }

    fn dying(&mut self, me: usize) {
        let e = self.e(me);
        if e.act_cnt == 0 {
            e.anm_num = 10;
            e.speed = 0;
        }
    }

    /// An attack that lunges (`ccEnemyK` row 184, `ccEnemyV` row 267,
    /// `ccEnemyF` row 116): turn to the target at `md` and, for its first
    /// 32 frames while the target is beyond 200, run at 40 (speed rate
    /// `rate`), else stand (0.3).
    fn lunge(e: &mut Enemy, md: F, rate: F) {
        Self::lunge_by(e, 32, K_200, md, rate);
    }

    /// [`Motion::lunge`] for `frames` frames beyond `range`.
    fn lunge_by(e: &mut Enemy, frames: i16, range: F, md: F, rate: F) {
        let spd = if e.atk_cnt < frames && !le(e.target_dist, range) { K_40 } else { 0 };
        let td = e.target_dirc;
        act_follow(e, td, md, spd, if spd == K_40 { rate } else { K_0_3 }, 0);
    }

    /// `ccEnemyK::action()` (0x0044aa90): as [`Motion::action_p`] with its
    /// own wander; row 184's attack snaps its walk and facing to the target
    /// at the start and lunges at 40 for 32 frames while beyond 200.
    pub fn action_k(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                self.act_escape(me);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                match e.ene_type {
                    3 => follow(e, Head::Target, K_0_01, half, K_0_06, ONE),
                    0..=2 => follow(e, Head::Target, K_0_1, half, K_0_06, ONE),
                    _ => {}
                }
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, ONE);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_06, half, K_0_1, ONE);
            }
            5 => self.face_or_stop(me, K_0_3),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if e.ene_id == 184 {
                        if e.atk_cnt == 0 {
                            e.mdirc[2] = e.target_dirc;
                            e.dirc[2] = e.target_dirc;
                        }
                        Self::lunge(e, K_0_08, K_0_3);
                    } else {
                        follow(e, Head::Target, K_0_125, stand, K_0_3, 0);
                    }
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyV::action()` (0x004518d0): as [`Motion::action_p`] with its
    /// own rates; row 267's attack lunges at 40 for 32 frames while beyond
    /// 200.
    pub fn action_v(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                self.act_escape(me);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                match e.ene_type {
                    3 => follow(e, Head::Target, K_0_005, half, K_0_03, K_0_8),
                    4 | 1 => follow(e, Head::Target, K_0_02, half, K_0_06, K_1_5),
                    2 => follow(e, Head::Target, K_0_005, half, K_0_06, K_2),
                    0 => follow(e, Head::Target, K_0_05, half, K_0_06, ONE),
                    _ => {}
                }
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, K_1_5);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_01, six_tenths, K_0_06, 0);
            }
            5 => self.face_or_stop(me, K_0_2),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if e.ene_id == 267 {
                        Self::lunge(e, K_0_125, K_0_1);
                    } else {
                        follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                    }
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyB::action()` (0x004424e0): the acts' clips and the attack's
    /// count and end (as [`Motion::action_g`]; dying also stops), then
    /// `moveEB` ([`Motion::move_eb`]); `moveFlag` 1 while walking.
    pub fn action_b(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => self.clip_on_start(me, 6),
            1..=4 => self.clip_on_start(me, 7),
            6 => self.attack_count(me),
            7 => self.flinch_clip(me),
            8 => self.dying(me),
            _ => {}
        }
        self.move_eb(me);
        self.walk_flag(me);
    }

    /// `ccEnemyB::moveEB()` (0x004426d0): by act, as [`Motion::move_eg`]
    /// with its own rates (a stop turning to the target reduces its speed
    /// by the turn). `ccEnemyH::moveEH()` (0x00449a00) is the same.
    pub fn move_eb(&mut self, me: usize) {
        self.move_eb_by(me, K_1_5);
    }

    /// [`Motion::move_eb`] with the chase's reduction `chase_red` (1.5;
    /// `ccEnemyC::moveEC()` 1).
    fn move_eb_by(&mut self, me: usize, chase_red: F) {
        let e = self.e(me);
        let tf = e.target_flag;
        match e.act_num {
            0 => {
                if tf {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            1 => self.act_escape(me),
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_01, half, K_0_06, ONE);
            }
            3 => {
                if tf {
                    follow(e, Head::Target, K_0_03, full, K_0_02, chase_red);
                }
            }
            4 => follow(e, Head::Home, K_0_06, half, K_0_1, ONE),
            5 => {
                if tf {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, ONE);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            6 => {
                if tf {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                }
            }
            7 | 8 => set_dist(&mut e.speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemy1::action()` (0x0043f200), race 0: a flinch of a later type
    /// keeps the target's heading to its top fraction bit; the acts as
    /// [`Motion::action_p`] (closing in backs off on one coin, 2 or 0, at
    /// 0.8 for types 0-2 and 0.5 for 3-5); `moveFlag` 1 walking, 2 for types
    /// 2 and 3 in their clips 2 and 3; walking with more than pi/4 to turn
    /// slows and turns the facing after the walk.
    pub fn action_1(&mut self, me: usize) {
        {
            let e = self.e(me);
            if e.anm_num == 7 && e.ene_type != 0 {
                e.target_dirc &= 0xffc0_0000;
            }
        }
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_05, half, K_0_06, ONE);
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, K_1_5);
                }
            }
            1 => {
                self.clip_on_start(me, 7);
                if self.en(me).act_cnt == 0 {
                    let c = self.cc() & 1 != 0;
                    self.e(me).opt_flag0 = c;
                }
                // Types above 5 read an uninitialised register: taken as 0.8.
                let red = match self.en(me).ene_type {
                    3..=5 => K_0_5,
                    _ => K_0_8,
                };
                let ty = if self.en(me).opt_flag0 { 2 } else { 0 };
                self.act_escape_by(me, ty, red);
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_06, six_tenths, K_0_1, ONE);
            }
            5 => self.face_or_stop(me, K_0_3),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        let e = self.e(me);
        if e.anm_num == 7 {
            e.move_flag = 1;
        } else {
            e.move_flag = 0;
            if (e.ene_type == 2 || e.ene_type == 3) && (e.anm_num == 2 || e.anm_num == 3) {
                e.move_flag = 2;
            }
        }
        if e.move_flag == 1 {
            let d = fabs_d(pi_limit(sub(e.mdirc[2], e.dirc[2])));
            if !le(d, K_PI_4) {
                let r = mul(K_0_1, div(d, K_0_45PI));
                set_dist(&mut e.speed, 0, r);
                set_rad(&mut e.dirc[2], e.mdirc[2], r);
            }
        }
    }

    /// The acts' clips as `ccEnemyB`, `ccEnemy2`, `ccEnemyC`, `ccEnemyH`
    /// and `ccEnemyU` set them: wait 6, the walks 7, the attack's count and
    /// end, a flinch 8 or 9, dying 10 (stopping).
    fn clips_b(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => self.clip_on_start(me, 6),
            1..=4 => self.clip_on_start(me, 7),
            6 => self.attack_count(me),
            7 => self.flinch_clip(me),
            8 => self.dying(me),
            _ => {}
        }
    }

    /// Walking with more than pi/4 still to turn: slow down and turn the
    /// facing after the walk, both by `0.1 d / 0.45 pi` (`ccEnemy1` and
    /// `ccEnemy2` after their acts).
    fn walk_turn(e: &mut Enemy) {
        if e.move_flag == 1 {
            let d = fabs_d(pi_limit(sub(e.mdirc[2], e.dirc[2])));
            if !le(d, K_PI_4) {
                let r = mul(K_0_1, div(d, K_0_45PI));
                set_dist(&mut e.speed, 0, r);
                set_rad(&mut e.dirc[2], e.mdirc[2], r);
            }
        }
    }

    /// `ccEnemy2::action()` (0x0043fbf0): the clips as [`Motion::action_b`],
    /// `moveE2` ([`Motion::move_e2`]), `moveFlag` 1 while walking, and for
    /// types 0, 1 and 3 the walk's turn (as `ccEnemy1`).
    pub fn action_2(&mut self, me: usize) {
        self.clips_b(me);
        self.move_e2(me);
        self.walk_flag(me);
        let e = self.e(me);
        if matches!(e.ene_type, 0 | 1 | 3) {
            Self::walk_turn(e);
        }
    }

    /// `ccEnemy2::moveE2()` (0x0043fed0): [`Motion::move_eb`] but for
    /// closing in: type 2 backs off as any enemy ([`Motion::act_escape`]);
    /// types 0, 1 and 3 toss a coin (`optFlag0`) on the act's first frame
    /// and back off by it, sliding (2) or straight (0), reduction 0.8.
    pub fn move_e2(&mut self, me: usize) {
        if self.en(me).act_num != 1 {
            self.move_eb(me);
            return;
        }
        match self.en(me).ene_type {
            2 => self.act_escape(me),
            0 | 1 | 3 => {
                if self.en(me).act_cnt == 0 {
                    let c = self.cc() & 1 != 0;
                    self.e(me).opt_flag0 = c;
                }
                let ty = if self.en(me).opt_flag0 { 2 } else { 0 };
                self.act_escape_by(me, ty, K_0_8);
            }
            _ => {}
        }
    }

    /// `ccEnemyC::action()` (0x00442e30): the clips as
    /// [`Motion::action_b`], then `moveEC` for types 0-2 (as
    /// [`Motion::move_eb`], chasing with reduction 1) or `moveECS`
    /// ([`Motion::move_ecs`]) for type 3; `moveFlag` 1 while walking.
    pub fn action_c(&mut self, me: usize) {
        self.clips_b(me);
        match self.en(me).ene_type {
            0..=2 => self.move_eb_by(me, ONE),
            3 => self.move_ecs(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyC::moveECS()` (0x00443330), a scorpion: it walks the way it
    /// faces and turns to its heading (`actSlide`). Closing in it clatters
    /// (sound 179) and slides off (escape 2, reduction 1.8); a stop spins it
    /// (+0x340) until it slows under 5 and waits. The spin is in
    /// docs/engine/battle.md ("The scorpion's spin").
    pub fn move_ecs(&mut self, me: usize) {
        let tf = self.en(me).target_flag;
        match self.en(me).act_num {
            0 => {
                if tf {
                    let e = self.e(me);
                    let td = e.target_dirc;
                    act_slide(e, td, K_0_03, 0, K_0_06, 0);
                }
            }
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                let (td, s) = (e.target_dirc, half(e));
                act_slide(e, td, K_0_03, s, K_0_06, ONE);
            }
            3 => {
                if tf {
                    let e = self.e(me);
                    let (td, s) = (e.target_dirc, e.max_spd);
                    act_slide(e, td, K_0_09, s, K_0_28, ONE);
                }
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    let pos = self.ai.scene.chars[me].pos;
                    self.call(me, Call::Sound3d { id: 179, pos });
                }
                self.act_escape_by(me, 2, K_1_8);
            }
            4 => {
                let e = self.e(me);
                let (b, s) = (e.base_dirc, half(e));
                act_slide(e, b, K_0_06, s, K_0_1, ONE);
            }
            5 => {
                if self.en(me).act_cnt == 0 {
                    let q = {
                        let e = self.en(me);
                        mul(HALF_PI, div(e.speed, e.max_spd))
                    };
                    let c = self.cc() & 1 != 0;
                    let e = self.e(me);
                    e.opt_flag0 = c;
                    let from = if e.target_flag { e.target_dirc } else { e.dirc[2] };
                    let mut v = if c { add(from, q) } else { sub(from, q) };
                    if !le(v, PI) {
                        v = sub(v, TWO_PI);
                    }
                    if lt(v, NEG_PI) {
                        v = add(v, TWO_PI);
                    }
                    e.spin = v;
                }
                let e = self.e(me);
                let (face, spin) = (e.dirc[2], e.spin);
                act_move(e, face, 0, spin, K_0_1, 0, K_0_1, 0);
                let pos = self.ai.scene.chars[me].pos;
                if self.en(me).act_cnt == 0 {
                    self.call(me, Call::Sound3d { id: 179, pos });
                }
                if self.en(me).act_cnt & 7 == 0 {
                    self.call(me, Call::Sound3d { id: 36, pos });
                }
                let e = self.e(me);
                if lt(e.speed, K_5) {
                    e.speed = 0;
                    e.act_num = 0;
                    e.act_cnt = 0;
                }
            }
            6 => {
                if tf {
                    let e = self.e(me);
                    let td = e.target_dirc;
                    act_slide(e, td, K_0_24, 0, K_0_3, 0);
                }
            }
            7 | 8 => set_dist(&mut self.e(me).speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemyF::action()` (0x00445910): as [`Motion::action_p`] with its
    /// own rates; row 116 (Minnow) lunges as K's row 184 does (0.125,
    /// 0.1).
    pub fn action_f(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                self.act_escape(me);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_01, six_tenths, K_0_06, ONE);
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, K_1_5);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_01, six_tenths, K_0_06, 0);
            }
            5 => self.face_or_stop(me, K_0_2),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if e.ene_id == 116 {
                        Self::lunge(e, K_0_125, K_0_1);
                    } else {
                        follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                    }
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyH::action()` (0x00449810): as [`Motion::action_b`] (its
    /// `moveEH` is `moveEB`).
    pub fn action_h(&mut self, me: usize) {
        self.action_b(me);
    }

    /// `ccEnemyI::action()` (0x0044a120): as [`Motion::action_p`] with its
    /// own rates; row 178 (Wiggle Snake) snaps to the target at the start
    /// and lunges for 24 frames while beyond 250 (0.16, 0.1).
    pub fn action_i(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                self.act_escape(me);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_02, half, K_0_06, ONE);
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_03, ONE);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_02, half, K_0_06, 0);
            }
            5 => self.face_or_stop(me, K_0_2),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if e.ene_id == 178 {
                        if e.atk_cnt == 0 {
                            e.mdirc[2] = e.target_dirc;
                            e.dirc[2] = e.target_dirc;
                        }
                        Self::lunge_by(e, 24, K_250, K_0_16, K_0_1);
                    } else {
                        follow(e, Head::Target, K_0_125, stand, K_0_3, 0);
                    }
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyU::action()` (0x00450940): the clips as
    /// [`Motion::action_b`], `moveEU` ([`Motion::move_eu`]), `moveFlag` 1
    /// while walking.
    pub fn action_u(&mut self, me: usize) {
        self.clips_b(me);
        self.move_eu(me);
        self.walk_flag(me);
    }

    /// `ccEnemyU::moveEU()` (0x00450b30): every frame first the target's
    /// heading loses its low 18-21 bits (`ccRand() & 3`); then by act as
    /// [`Motion::move_eb`] (chasing with reduction 1), with the wanderer's
    /// rates by type, the tails escape of types 3-6 turned aside first, and
    /// row 245 (Death Head) snapping, dashing and bobbing. The rates are in
    /// docs/engine/battle.md ("Enemy movement and animation").
    pub fn move_eu(&mut self, me: usize) {
        let r = (self.cc() & 3) + 18;
        {
            let e = self.e(me);
            e.target_dirc = (((e.target_dirc as i32) >> r) << r) as u32;
        }
        let tf = self.en(me).target_flag;
        match self.en(me).act_num {
            0 | 3 | 4 | 5 | 7 | 8 => self.move_eb_by(me, ONE),
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                match e.ene_type {
                    0 | 1 => follow(e, Head::Target, K_0_05, half, K_0_03, ONE),
                    2 | 3 => follow(e, Head::Target, K_0_02, half, K_0_03, K_0_8),
                    4..=6 => follow(e, Head::Target, K_0_005, half, K_0_03, K_0_6),
                    _ => {}
                }
                let m = e.mdirc[2];
                set_rad(&mut e.dirc[2], m, K_0_05);
            }
            1 => match self.en(me).ene_type {
                0..=2 => self.act_escape(me),
                3..=6 => {
                    if self.en(me).act_cnt == 0 {
                        self.coins(me);
                    }
                    if self.en(me).opt_flag0 {
                        self.act_escape_by(me, 1, K_0_8);
                    } else {
                        if self.en(me).act_cnt == 0 {
                            let (m, d, v) = {
                                let e = self.en(me);
                                (e.mdirc[2], e.dirc[2], e.speed)
                            };
                            self.act_escape_by(me, 2, K_0_8);
                            let sign = if le(self.en(me).mdirc[2], m) { MINUS_ONE } else { ONE };
                            {
                                let e = self.e(me);
                                e.mdirc[2] = m;
                                e.dirc[2] = d;
                                e.speed = v;
                            }
                            let mut t = enemy_ai::rand_f2(self.ai.cc, K_3PI_4, K_3PI_4);
                            if lt(t, 0) {
                                t = mul(t, MINUS_ONE);
                            }
                            let t = mul(t, sign);
                            let e = self.e(me);
                            e.mdirc[2] = pi_limit(add(e.mdirc[2], t));
                        }
                        self.act_escape_by(me, 2, K_0_8);
                    }
                }
                _ => {}
            },
            6 if tf => {
                let range = self.think(me).atk_range_a;
                let e = self.e(me);
                if e.ene_id == 245 {
                    if e.atk_cnt == 0 {
                        e.mdirc[2] = e.target_dirc;
                        e.dirc[2] = e.target_dirc;
                    }
                    let spd = if e.atk_cnt < 32 && !le(e.target_dist, range) { mul(K_1_5, e.max_spd) } else { 0 };
                    follow_at(e, K_0_08, spd, K_0_3);
                } else {
                    follow_at(e, K_0_08, 0, K_0_3);
                }
            }
            _ => {}
        }
        let e = self.e(me);
        if e.ene_id != 245 {
            return;
        }
        match e.act_num {
            8 | 9 => set_dist(&mut e.zoffs, 0, K_0_08),
            6 => {
                if e.atk_num == 0 || e.atk_num == 1 {
                    e.rad_cnt = deg2rad(rad2deg(e.rad_cnt).wrapping_add(291));
                }
            }
            _ => {
                e.rad_cnt = deg2rad(rad2deg(e.rad_cnt).wrapping_add(584));
                e.zoffs = add(K_80, mul(K_30, piney_data::libm::sinf(e.rad_cnt)));
            }
        }
    }

    /// `moveFlag` 1 while walking (clip 7), 2 in clips 2 and 3
    /// (`ccEnemy3`, `ccEnemy4`).
    fn walk_flag_23(e: &mut Enemy) {
        e.move_flag = match e.anm_num {
            7 => 1,
            2 | 3 => 2,
            _ => 0,
        };
    }

    /// `ccEnemy3::action()` (0x00440610): the clips as
    /// [`Motion::action_b`], `moveE3` (as `moveEB`) and `moveFlag` as
    /// [`Motion::walk_flag_23`].
    pub fn action_3(&mut self, me: usize) {
        self.clips_b(me);
        self.move_eb(me);
        Self::walk_flag_23(self.e(me));
    }

    /// `ccEnemy4::action()` (0x00440ea0): the clips as
    /// [`Motion::action_b`], `moveE4` ([`Motion::move_e4`]), `moveFlag` as
    /// `ccEnemy3`, then the walk's turn (as `ccEnemy1`).
    pub fn action_4(&mut self, me: usize) {
        self.clips_b(me);
        self.move_e4(me);
        let e = self.e(me);
        Self::walk_flag_23(e);
        Self::walk_turn(e);
    }

    /// `ccEnemy4::moveE4()` (0x00441190): [`Motion::move_eb`] but closing
    /// in, where a coin (`optFlag0`) on the act's first frame picks sliding
    /// (2) or straight (0) away, reduction 0.8.
    pub fn move_e4(&mut self, me: usize) {
        if self.en(me).act_num != 1 {
            self.move_eb(me);
            return;
        }
        if self.en(me).act_cnt == 0 {
            let c = self.cc() & 1 != 0;
            self.e(me).opt_flag0 = c;
        }
        let ty = if self.en(me).opt_flag0 { 2 } else { 0 };
        self.act_escape_by(me, ty, K_0_8);
    }

    /// `ccEnemyA::action()` (0x00441a70): as [`Motion::action_p`] with its
    /// own rates; a wanderer's heading scatters by pi/4; closing in goes by
    /// type: 0 `actEscape()`, 1 and 2 slide or run by the first coin
    /// (reduction 0.8, 0.6), 3 slide or face the target (0.8), 4 face the
    /// target (1.0).
    pub fn action_a(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                let o = self.en(me).opt_flag0;
                match self.en(me).ene_type {
                    0 => self.act_escape(me),
                    1 => self.act_escape_by(me, if o { 2 } else { 0 }, K_0_8),
                    2 => self.act_escape_by(me, if o { 2 } else { 0 }, K_0_6),
                    3 => self.act_escape_by(me, if o { 2 } else { 1 }, K_0_8),
                    // types above 4 read uninitialised registers; none exist
                    4 => self.act_escape_by(me, 1, ONE),
                    _ => {}
                }
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_by(me, K_PI_4);
                let e = self.e(me);
                follow(e, Head::Target, K_0_01, six_tenths, K_0_06, ONE);
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, K_1_5);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                follow(e, Head::Home, K_0_01, six_tenths, K_0_06, 0);
            }
            5 => self.face_or_stop(me, K_0_3),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// Row 89 (Bat)'s attack in `moveED` (and, never reached, `moveES`):
    /// for 48 frames while the target is beyond 200 it flies at 30.
    fn bat_attack(e: &mut Enemy) {
        let spd = if e.ene_id == 89 && e.atk_cnt < 48 && !le(e.target_dist, K_200) { K_30 } else { 0 };
        follow_at(e, K_0_08, spd, K_0_3);
    }

    /// `ccEnemyD::action()` (0x00443d80): the clips as
    /// [`Motion::action_b`], `moveED` for types but 2 and 4 (as `moveEB`,
    /// chasing with reduction 1; the Bat's attack [`Motion::bat_attack`])
    /// or `moveEDD` ([`Motion::move_edd`]) for types 2 and 4; `moveFlag` 1
    /// walking, 2 for type 0 in clip 0.
    pub fn action_d(&mut self, me: usize) {
        self.clips_b(me);
        match self.en(me).ene_type {
            2 | 4 => self.move_edd(me),
            // 0, 1, 3, and any other type (the switch falls into moveED)
            _ => {
                let tf = self.en(me).target_flag;
                if self.en(me).act_num == 6 {
                    if tf {
                        Self::bat_attack(self.e(me));
                    }
                } else {
                    self.move_eb_by(me, ONE);
                }
            }
        }
        let e = self.e(me);
        e.move_flag = if e.anm_num == 7 {
            1
        } else if e.ene_type == 0 && e.anm_num == 0 {
            2
        } else {
            0
        };
    }

    /// `ccEnemyD::moveEDD()` (0x00444360), the eyes (types 2 and 4): wait
    /// .08/0/.3/0, wander .004/.6 max/.2/1, chase .0625/max/.3/1, home
    /// .04/.6 max/.3/1, stop and attack .125/0/.3/0; closing in a coin on the
    /// first frame picks `actEscape(1 or 2, 0.8)`.
    pub fn move_edd(&mut self, me: usize) {
        let tf = self.en(me).target_flag;
        match self.en(me).act_num {
            0 => self.face_or_stop(me, K_0_3),
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_004, six_tenths, K_0_2, ONE);
            }
            3 => {
                if tf {
                    follow(self.e(me), Head::Target, K_0_0625, full, K_0_3, ONE);
                }
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    let c = self.cc() & 1 != 0;
                    self.e(me).opt_flag0 = c;
                }
                let ty = if self.en(me).opt_flag0 { 1 } else { 2 };
                self.act_escape_by(me, ty, K_0_8);
            }
            4 => follow(self.e(me), Head::Home, K_0_04, six_tenths, K_0_3, ONE),
            5 => {
                let e = self.e(me);
                if tf {
                    follow(e, Head::Target, K_0_125, stand, K_0_3, 0);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            6 => {
                if tf {
                    follow(self.e(me), Head::Target, K_0_125, stand, K_0_3, 0);
                }
            }
            7 | 8 => set_dist(&mut self.e(me).speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemyE::action()` (0x00444bd0): as [`Motion::action_p`] with its
    /// own rates (a wanderer's by type); closing in sets no clip: types 0,
    /// 1 and 3 `actEscape()`, type 2 a coin for `actEscape(2 or 1, 0.5)`;
    /// type 0 (Moai) lunges as [`Motion::action_f`]'s Minnow.
    pub fn action_e(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            1 => match self.en(me).ene_type {
                0 | 1 | 3 => self.act_escape(me),
                2 => {
                    if self.en(me).act_cnt == 0 {
                        let c = self.cc() & 1 != 0;
                        self.e(me).opt_flag0 = c;
                    }
                    let ty = if self.en(me).opt_flag0 { 2 } else { 1 };
                    self.act_escape_by(me, ty, K_0_5);
                }
                _ => {}
            },
            2 => {
                self.clip_on_start(me, 7);
                self.wander_heading(me);
                let e = self.e(me);
                match e.ene_type {
                    0 => follow(e, Head::Target, K_0_05, half, K_0_03, ONE),
                    1 => follow(e, Head::Target, K_0_02, half, K_0_03, ONE),
                    2 | 3 => follow(e, Head::Target, K_0_005, half, K_0_03, K_0_5),
                    _ => {}
                }
            }
            3 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, K_1_5);
                }
            }
            4 => {
                self.clip_on_start(me, 7);
                follow(self.e(me), Head::Home, K_0_01, six_tenths, K_0_06, 0);
            }
            5 => self.face_or_stop(me, K_0_2),
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if e.ene_type == 0 {
                        Self::lunge(e, K_0_125, K_0_1);
                    } else {
                        follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                    }
                }
            }
            7 => {
                self.flinch_clip(me);
                set_dist(&mut self.e(me).speed, 0, K_0_5);
            }
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyS::action()` (0x0044f220): the clips as
    /// [`Motion::action_b`], `moveES`: as `moveEB` (chasing with
    /// reduction 1) but a wanderer's rates by type (0-1 .05/half/.06/1, 2
    /// .02/half/.02/1.5, 3 .03/half/.03/1.2), type 2 closing in
    /// `actEscape(2, 0.3)`, and the attack [`Motion::bat_attack`]'s (its
    /// row 89 test never true here).
    pub fn action_s(&mut self, me: usize) {
        self.clips_b(me);
        let tf = self.en(me).target_flag;
        match self.en(me).act_num {
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                match e.ene_type {
                    0 | 1 => follow(e, Head::Target, K_0_05, half, K_0_06, ONE),
                    3 => follow(e, Head::Target, K_0_03, half, K_0_03, K_1_2),
                    2 => follow(e, Head::Target, K_0_02, half, K_0_02, K_1_5),
                    _ => {}
                }
            }
            1 => match self.en(me).ene_type {
                0 | 1 | 3 => self.act_escape(me),
                2 => self.act_escape_by(me, 2, K_0_3),
                _ => {}
            },
            6 => {
                if tf {
                    Self::bat_attack(self.e(me));
                }
            }
            _ => self.move_eb_by(me, ONE),
        }
        self.walk_flag(me);
    }

    /// A box's walking clip (`ccEnemyT`): type 0 plays 7; type 1 (a
    /// mimic) first opens (11) unless it was walking, and walks (7) once
    /// the opening ends.
    fn box_walk(&mut self, me: usize) {
        let e = self.e(me);
        match e.ene_type {
            0 => {
                if e.act_cnt == 0 {
                    e.anm_num = 7;
                }
            }
            1 => {
                if e.act_cnt == 0 && e.anm_num_old != 7 {
                    e.anm_num = 11;
                }
                if e.anm_num_old == 11 && e.anm_flag != 0 {
                    e.anm_num = 7;
                }
            }
            _ => {}
        }
    }

    /// A flinch that plays clip 8 and counts its first frame again
    /// (`ccEnemyT`, `ccEnemyW`).
    fn flinch_8(&mut self, me: usize) {
        let e = self.e(me);
        if e.act_cnt == 0 {
            e.anm_num = 8;
            e.act_cnt += 1;
        }
        if e.anm_flag != 0 {
            e.action_flag = true;
        }
        set_dist(&mut e.speed, 0, K_0_5);
    }

    /// `ccEnemyT::action()` (0x0044fbf0), the boxes: a mimic (type 1)
    /// closes its lid (12) when it stops or waits after walking and opens it
    /// (11) to move ([`Motion::box_walk`]); closing in for 49 frames or
    /// more it gives up and stops (act 5, count 0, `moveFlag` left as it
    /// was). Rates: wait and stop .08/0/.3/0, wander .1/half/.06/1, chase
    /// .03/max/.02/1, home .06/half/.1/1, attack .125/0/.3/0.
    pub fn action_t(&mut self, me: usize) {
        let ty = self.en(me).ene_type;
        match self.en(me).act_num {
            0 => {
                let e = self.e(me);
                if ty == 0 && e.act_cnt == 0 {
                    e.anm_num = 6;
                } else if ty == 1 {
                    if e.act_cnt == 0 {
                        e.anm_num = if e.anm_num_old == 7 { 12 } else { 6 };
                    }
                    if e.anm_num_old == 12 && e.anm_flag != 0 {
                        e.anm_num = 6;
                    }
                }
                self.face_or_stop(me, K_0_3);
            }
            2 => {
                self.box_walk(me);
                self.wander_heading(me);
                follow(self.e(me), Head::Target, K_0_1, half, K_0_06, ONE);
            }
            3 => {
                self.box_walk(me);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_03, full, K_0_02, ONE);
                }
            }
            1 => {
                self.box_walk(me);
                if ty == 1 && self.en(me).act_cnt >= 49 {
                    let e = self.e(me);
                    e.act_num = 5;
                    e.act_cnt = 0;
                    return;
                }
                self.act_escape(me);
            }
            4 => {
                self.box_walk(me);
                follow(self.e(me), Head::Home, K_0_06, half, K_0_1, ONE);
            }
            5 => {
                let e = self.e(me);
                if ty == 1 && e.act_cnt == 0 {
                    e.anm_num = 12;
                }
                self.face_or_stop(me, K_0_3);
            }
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_125, stand, K_0_3, 0);
                }
            }
            7 => self.flinch_8(me),
            8 => self.dying(me),
            _ => {}
        }
        self.walk_flag(me);
    }

    /// `ccEnemyW::action()` (0x00452340), the ghosts: as
    /// [`Motion::action_p`] with its own rates (a wanderer scatters by pi
    /// every 16 frames at .004/half/.06/1; home at .8 `maxSpd`); a chase
    /// counts its first frame again; types 0 and 1 rush in attack 0 (1.2
    /// `maxSpd`, .2, reduction 1) and bob: `zoffs = 100 + 60 sinf(radCnt)`,
    /// `radCnt` on by 584 a frame (291 in attack 0, no bob), sinking to 0
    /// (0.1) when dying; a flinch plays clip 8.
    pub fn action_w(&mut self, me: usize) {
        match self.en(me).act_num {
            0 => {
                self.clip_on_start(me, 6);
                self.face_or_stop(me, K_0_3);
            }
            2 => {
                self.clip_on_start(me, 7);
                self.wander_by(me, PI);
                follow(self.e(me), Head::Target, K_0_004, half, K_0_06, ONE);
            }
            3 => {
                let e = self.e(me);
                if e.act_cnt == 0 {
                    e.act_cnt += 1;
                    e.anm_num = 7;
                }
                if e.target_flag {
                    follow(e, Head::Target, K_0_02, full, K_0_02, ONE);
                }
            }
            1 => {
                if self.en(me).act_cnt == 0 {
                    self.e(me).anm_num = 7;
                    self.coins(me);
                }
                self.act_escape(me);
            }
            4 => {
                self.clip_on_start(me, 7);
                let e = self.e(me);
                let (b, v) = (e.base_dirc, mul(K_0_8, e.max_spd));
                act_follow(e, b, K_0_06, v, K_0_1, ONE);
            }
            5 => {
                let e = self.e(me);
                if e.target_flag {
                    follow(e, Head::Target, K_0_08, stand, K_0_3, ONE);
                } else {
                    set_dist(&mut e.speed, 0, K_0_3);
                }
            }
            6 => {
                self.attack_count(me);
                let e = self.e(me);
                if e.target_flag {
                    if (e.ene_type == 0 || e.ene_type == 1) && e.atk_num == 0 {
                        let (td, v) = (e.target_dirc, mul(K_1_2, e.max_spd));
                        act_follow(e, td, K_0_08, v, K_0_2, ONE);
                    } else {
                        follow(e, Head::Target, K_0_08, stand, K_0_3, 0);
                    }
                }
            }
            7 => self.flinch_8(me),
            8 => self.dying(me),
            _ => {}
        }
        let e = self.e(me);
        if e.ene_type == 0 || e.ene_type == 1 {
            match e.act_num {
                8 | 9 => set_dist(&mut e.zoffs, 0, K_0_1),
                6 => {
                    if e.atk_num == 0 {
                        e.rad_cnt = deg2rad(rad2deg(e.rad_cnt).wrapping_add(291));
                    }
                }
                _ => {
                    e.rad_cnt = deg2rad(rad2deg(e.rad_cnt).wrapping_add(584));
                    e.zoffs = add(K_100, mul(K_60, piney_data::libm::sinf(e.rad_cnt)));
                }
            }
        }
        self.walk_flag(me);
    }

    /// `ccEnemyD::exclusive()` (0x004446a0) after the controllers: a type
    /// 3 (Ark Prince and its kin) floats at `zoffs` 0 (0.2), sinking to -50
    /// (0.08) when dying.
    fn excl_d(&mut self, me: usize) {
        let e = self.e(me);
        if e.ene_type != 3 {
            return;
        }
        if e.act_num == 8 || e.act_num == 9 {
            set_dist(&mut e.zoffs, K_M50, K_0_08);
        } else {
            set_dist(&mut e.zoffs, 0, K_0_2);
        }
    }

    /// `ccEnemyL::action()` (0x0044bae0): the clips as
    /// [`Motion::action_b`], then `moveEL` ([`Motion::move_el`]) for types
    /// 0 and 1 (and any but 2-4), `moveEL2` ([`Motion::move_el2`]) for 2
    /// and 4; type 3's `moveELG` is not ported. `moveFlag` 1 while
    /// walking.
    pub fn action_l(&mut self, me: usize) {
        self.clips_b(me);
        match self.en(me).ene_type {
            2 | 4 => self.move_el2(me),
            3 => {}
            _ => self.move_el(me),
        }
        self.walk_flag(me);
    }

    /// `ccEnemyL::moveEL()` (0x0044bd60), the snakoids: wait and stop
    /// .08/0/.3/0 (stop .3 without a target), wander .02/half/.03/1 with
    /// the facing following the walk (.05), chase .03/max/.02/1, close in
    /// `actEscape()`, home .03/.6 max/.3/1, attack .08/0/.3/0, a flinch
    /// stops (.5), dying nothing.
    pub fn move_el(&mut self, me: usize) {
        let tf = self.en(me).target_flag;
        match self.en(me).act_num {
            0 | 5 => self.face_or_stop(me, K_0_3),
            2 => {
                self.wander_heading(me);
                let e = self.e(me);
                follow(e, Head::Target, K_0_02, half, K_0_03, ONE);
                let m = e.mdirc[2];
                set_rad(&mut e.dirc[2], m, K_0_05);
            }
            3 => {
                if tf {
                    follow(self.e(me), Head::Target, K_0_03, full, K_0_02, ONE);
                }
            }
            1 => self.act_escape(me),
            4 => follow(self.e(me), Head::Home, K_0_03, six_tenths, K_0_3, ONE),
            6 => {
                if tf {
                    follow(self.e(me), Head::Target, K_0_08, stand, K_0_3, 0);
                }
            }
            7 => set_dist(&mut self.e(me).speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemyL::moveEL2()` (0x0044c010), the wyrms and dragons: wait and
    /// stop as `moveEL`, wander (scattered by pi/4) .01/half/.06/1, chase
    /// .03/max/.02/1.5, close in `actEscape(1, 0.3)`, home .01/half/.06/0,
    /// attack .08/0/.3/0 with or without a target, a flinch stops (.5).
    pub fn move_el2(&mut self, me: usize) {
        let tf = self.en(me).target_flag;
        match self.en(me).act_num {
            0 | 5 => self.face_or_stop(me, K_0_3),
            2 => {
                self.wander_by(me, K_PI_4);
                follow(self.e(me), Head::Target, K_0_01, half, K_0_06, ONE);
            }
            3 => {
                if tf {
                    follow(self.e(me), Head::Target, K_0_03, full, K_0_02, K_1_5);
                }
            }
            1 => self.act_escape_by(me, 1, K_0_3),
            4 => follow(self.e(me), Head::Home, K_0_01, half, K_0_06, 0),
            6 => follow(self.e(me), Head::Target, K_0_08, stand, K_0_3, 0),
            7 => set_dist(&mut self.e(me).speed, 0, K_0_5),
            _ => {}
        }
    }

    /// `ccEnemyL::exclusive()` (0x0044c300) after the controllers: the
    /// wyrms (type 2, `exclELW` 0x0044c3e0) while displayed run their
    /// breath and breathe in attack 4 at a target (`elBrParam` +0x80); the
    /// dragons (type 4, `exclELD` 0x0044c480) while displayed rise in
    /// attack 4 (`yoffs` toward 300 at .04 for its first 190 frames, else
    /// back to 0 at .1), run their breath and breathe in attacks 4 and 5
    /// (`elBrParam`, +0x40).
    fn excl_l(&mut self, me: usize) {
        let e = self.e(me);
        if !e.disp_sw {
            return;
        }
        match e.ene_type {
            2 => {
                if !e.breath {
                    return;
                }
                self.call(me, Call::BreathCtrl { slot: 0 });
                let e = self.en(me);
                if e.target_flag && e.act_num == 6 && e.atk_num == 4 {
                    let (disp, transparency, act_cnt) = (e.disp_sw, e.transparency, e.act_cnt);
                    let param = InfoRef::new(EL_BR_PARAM, 2);
                    self.call(me, Call::BreathSet { slot: 0, param, disp, transparency, act_cnt });
                }
            }
            4 => {
                if e.act_num == 6 && e.atk_num == 4 && e.act_cnt < 190 {
                    set_dist(&mut e.yoffs, K_300, K_0_04);
                } else {
                    set_dist(&mut e.yoffs, 0, K_0_1);
                }
                if !e.breath {
                    return;
                }
                self.call(me, Call::BreathCtrl { slot: 0 });
                let e = self.en(me);
                if e.target_flag && e.act_num == 6 && (e.atk_num == 4 || e.atk_num == 5) {
                    let param = InfoRef::new(EL_BR_PARAM, if e.atk_num == 4 { 0 } else { 1 });
                    let (disp, transparency, act_cnt) = (e.disp_sw, e.transparency, e.act_cnt);
                    self.call(me, Call::BreathSet { slot: 0, param, disp, transparency, act_cnt });
                }
            }
            _ => {}
        }
    }

    /// What the dust controller reads of enemy `me` now.
    fn dust_at(&self, me: usize) -> DustAt {
        let e = self.en(me);
        let (info, n) = e.dust.map_or((None, 0), |(info, n)| (Some(info), n));
        DustAt {
            info,
            n,
            disp_sw: e.disp_sw,
            smoke: e.ene_smoke,
            anm_num: e.anm_num,
            frame_num: e.frame_num as i16,
            pos: self.ai.scene.chars[me].pos,
            dirc: e.dirc,
        }
    }

    /// What the weapon controller reads of enemy `me` now: `dispSW`,
    /// `actNum`, `actCnt`, `atkNum` and its skill's element
    /// (`ccSkillCheckTypeAttribute(skillParam->type)`, 0 without one).
    fn weapon_at(&self, me: usize) -> WeaponAt {
        let e = self.en(me);
        let attr =
            e.skill_param.and_then(|r| r.get(self.ai.t, e.ene_id)).map_or(0, |s| skill::check_type_attribute(s.ty));
        WeaponAt { disp_sw: e.disp_sw, act_num: e.act_num, act_cnt: e.act_cnt, atk_num: e.atk_num, attr }
    }

    /// The race's `exclusive()`: the weapon controller and the dust
    /// controller (flag 0) when the race made them (`ccEnemyP::exclusive`
    /// 0x0044edd0, `ccEnemyK` 0x0044b040, `ccEnemyV` 0x00451ef0, `ccEnemyB`
    /// 0x00442990, `ccEnemy1` 0x0043f820, `ccEnemyG` 0x00448fc0); a gold
    /// goblin walking also clinks (`ccRand`: 1 in 8 at walking pace, 1 in 4
    /// running, every 16 frames) and raises dust every 5, 3, 2 or 1
    /// frames by its speed's share of the row's `maxSpd` (below 0.5, 0.8,
    /// 1, above), bigger the faster.
    pub fn exclusive(&mut self, me: usize) {
        let kind = Kind::of_row(self.ai.t, self.en(me).ene_id);
        if matches!(kind, Kind::Other(_)) {
            return;
        }
        if self.en(me).weapon.is_some() {
            let at = self.weapon_at(me);
            self.call(me, Call::WeaponCtrl(at));
        }
        if self.en(me).dust.is_some() && kind.excl_dust() {
            let at = self.dust_at(me);
            self.call(me, Call::DustCtrl { flag: 0, at });
        }
        match kind {
            Kind::F => self.excl_f(me),
            Kind::C => self.excl_c(me),
            Kind::H => self.excl_h(me),
            Kind::D => self.excl_d(me),
            Kind::L => self.excl_l(me),
            _ => {}
        }
        if kind != Kind::G || !self.en(me).gold.flag || self.en(me).anm_num != 7 {
            return;
        }
        let pos = self.ai.scene.chars[me].pos;
        let r = div(self.en(me).speed, self.think(me).max_spd);
        let (n, size) = if lt(r, K_0_5) {
            (5, K_2)
        } else if lt(r, K_0_8) {
            (3, K_3)
        } else if le(r, ONE) {
            if self.en(me).act_cnt & 0xf == 0 && self.cc() & 7 == 0 {
                self.call(me, Call::Sound3d { id: 161, pos });
            }
            (2, K_4)
        } else {
            if self.en(me).act_cnt & 0xf == 0 && self.cc() & 3 == 0 {
                let id = match self.cc() & 3 {
                    0 => 161,
                    1 => 160,
                    2 => 60,
                    _ => 177,
                };
                self.call(me, Call::Sound3d { id, pos });
            }
            (1, K_5)
        };
        let e = self.en(me);
        if i32::from(e.act_cnt) % n == 0 {
            let smoke = e.ene_smoke;
            self.call(me, Call::Dust { pos, size, smoke });
        }
    }

    /// `ccEnemyF::exclusive()` (0x00445e20) after the controllers: a type 2
    /// (the rays) dives in its attacks 2 and 3, its model sinking to -300
    /// (0.04) for their first 50 frames, and otherwise rises back to 0
    /// (0.1).
    fn excl_f(&mut self, me: usize) {
        let e = self.e(me);
        if e.ene_type != 2 {
            return;
        }
        if e.act_num == 6 && (e.atk_num == 2 || e.atk_num == 3) && e.act_cnt < 50 {
            set_dist(&mut e.yoffs, K_M300, K_0_04);
        } else {
            set_dist(&mut e.yoffs, 0, K_0_1);
        }
    }

    /// `ccEnemyC::exclusive()` (0x00443780) after the controllers: a
    /// spinning scorpion (type 3, act 5) kicks up dust at its four wheels
    /// (`wheelPos`, gcmn 0x005dec10, through the model's world matrix)
    /// every other frame.
    fn excl_c(&mut self, me: usize) {
        const WHEELS: [V4; 4] = [
            [0x4348_0000, 0xc396_0000, 0, ONE],
            [0xc348_0000, 0xc396_0000, 0, ONE],
            [0x4302_0000, 0x4348_0000, 0, ONE],
            [0xc302_0000, 0x4348_0000, 0, ONE],
        ];
        let e = self.en(me);
        if e.ene_type != 3 || e.act_num != 5 || e.act_cnt & 1 == 0 {
            return;
        }
        let (m, smoke) = (e.anm_lw, e.ene_smoke);
        for w in WHEELS {
            let pos = apply_matrix(&m, w);
            self.call(me, Call::EffDust { pos, n: 2, size: K_3, life: 20, smoke });
        }
    }

    /// `ccEnemyH::exclusive()` (0x00449cc0) after the controllers, for the
    /// types with breath (3, 4; `exclEHK`, 0x00449d50): while displayed,
    /// both breaths run, and in attack 4 at a target breath `actCnt & 1`
    /// breathes.
    fn excl_h(&mut self, me: usize) {
        let e = self.en(me);
        if !(e.ene_type == 3 || e.ene_type == 4) || !e.disp_sw {
            return;
        }
        for slot in 0..2 {
            self.call(me, Call::BreathCtrl { slot });
        }
        let e = self.en(me);
        if e.target_flag && e.act_num == 6 && e.atk_num == 4 {
            let slot = (e.act_cnt & 1) as usize;
            let (disp, transparency, act_cnt) = (e.disp_sw, e.transparency, e.act_cnt);
            self.call(me, Call::BreathSet { slot, param: EHK_BR_PARAM, disp, transparency, act_cnt });
        }
    }

    // ccEnemy::main ------------------------------------------------------------

    /// `ccEnemy::routineEnemy()` (0x00433990) as [`Ai::routine_enemy`], with
    /// `CalcReal(0)` run by [`Motion::calc_real`] (its timers' affects
    /// landing inside it): the life rate, the top speed, the attack delay
    /// and the flinch count after it.
    pub fn routine(&mut self, me: usize) {
        self.calc_real(me);
        let (hp, max_hp, speed_value) = {
            let ch = &self.ai.scene.chars[me];
            (ch.hp, ch.max_hp, ch.cond.speed_value)
        };
        let think_max = self.think(me).max_spd;
        let puppet = self.ai.world.puppet_show;
        let e = self.e(me);
        e.life_rate = div(from_int(i32::from(hp)), from_int(i32::from(max_hp)));
        e.max_spd = mul(speed_value, mul(think_max, e.hit_spd));
        if e.atk_dellay > 0 {
            e.atk_dellay -= 1;
        }
        if puppet && e.atk_dellay > 0 {
            e.atk_dellay -= 1;
        }
        if e.act_num != act::DAMAGE && e.damage_cnt > 0 {
            e.damage_cnt -= 1;
        }
    }

    /// `ccChar::CalcReal(0)` (gcmn 0x0056ba30) for an enemy, as
    /// [`crate::chara::calc_real`], with `ConditionTimeCount` (0x0056bfd0)
    /// run by [`Motion::condition_time_count`] so that each timer's
    /// `EntryAffect` lands where the game calls it: `real` is the row's
    /// stats plus `temp` after the timers (a poison tick that retargets an
    /// enemy with nobody left to fight clears its buffs first), the
    /// protect break counts down, the condition marks show.
    pub fn calc_real(&mut self, me: usize) {
        let tyb = self.ai.scene.chars[me].ty();
        if tyb & ty::FOE == 0 {
            return;
        }
        let enemy = tyb & ty::ENEMY != 0;
        {
            let ch = &mut self.ai.scene.chars[me];
            let Body::Foe(f) = &mut ch.body else { return };
            f.real = f.row.elm;
            if enemy {
                let (hp, sp) = (f.row.max_hp, f.row.max_sp);
                ch.max_hp = hp;
                ch.max_sp = sp;
            }
        }
        self.condition_time_count(me);
        let mut ev = Events::new();
        {
            let t = self.ai.t;
            let ch = &mut self.ai.scene.chars[me];
            chara::condition_battle_effect(t, ch);
            let Body::Foe(f) = &mut ch.body else { return };
            let temp = f.temp;
            chara::add_elm(&mut f.real, &temp, 32767, 0);
            if f.pp_count != 0 && (enemy || f.pp >= 0) {
                f.pp_count = f.pp_count.wrapping_sub(1);
                if f.pp_count <= 0 {
                    f.pp_count = 0;
                    if enemy {
                        f.pp = (i32::from(f.row.max_pp) / 2) as i16;
                        ev.push(Event::Protect { on: Who::Me, broken: 1, kind: -1 });
                    } else {
                        f.pp_restore = f.pp_restore.wrapping_add(1);
                        if f.pp_restore >= 2 {
                            f.pp_restore = 2;
                            f.pp = (i32::from(f.row.max_pp) * 3).wrapping_div(4) as i16;
                        } else {
                            f.pp = (i32::from(f.row.max_pp) / 2) as i16;
                        }
                        let kind = if f.row.base.id != 0 { 2 } else { 1 };
                        ev.push(Event::Protect { on: Who::Me, broken: 1, kind });
                    }
                    ev.push(Event::SetProtect { state: 1, on: Who::Me });
                }
            }
        }
        if self.env.area != 0 {
            ev.push(Event::DispCondition(Who::Me));
        }
        for x in ev {
            self.event(me, x);
        }
    }

    /// `ccChar::ConditionTimeCount()` (gcmn 0x0056bfd0) for an enemy, as
    /// [`crate::chara::condition_time_count`] with each tick's
    /// `EntryAffect(this, kind, amount)` applied at once
    /// ([`Motion::entry_affect`]): buffs and debuffs expire; sleep,
    /// confusion, charm, paralysis and speed count down; HP regeneration
    /// (9, maxHP/50) and poison (3, maxHP/100, while no menu is open) tick,
    /// then SP comes back (maxSP/50 a second) unless cursed, then SP
    /// regeneration (10) and the curse (4) tick.
    pub fn condition_time_count(&mut self, me: usize) {
        const TIMED: [usize; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13];
        let at_least_1 = |v: i32| if v == 0 { 1 } else { v } as i16;
        let tick = |c: i16, n: i32| i32::from(c).wrapping_rem(n) == 0;
        {
            let ch = &mut self.ai.scene.chars[me];
            if let Some((temp, time)) = ch.temp_time_mut() {
                for i in TIMED {
                    let t = time[i];
                    if t > 0 {
                        time[i] = t - 1;
                        if t - 1 == 0 {
                            temp[i] = 0;
                        }
                    } else if t < 0 {
                        time[i] = 0;
                        temp[i] = 0;
                    }
                }
            }
            let c = &mut ch.cond;
            for i in [cond::SLEEP, cond::CONFUSION, cond::CHARM, cond::PARALYSIS] {
                if c[i] > 0 {
                    c[i] -= 1;
                } else if c[i] < 0 {
                    c[i] = 0;
                }
            }
            if c[cond::SPEED] > 0 {
                c[cond::SPEED] -= 1;
                if c[cond::SPEED] == 0 {
                    c.speed_value = ONE;
                }
            } else if c[cond::SPEED] < 0 {
                c[cond::SPEED] = 0;
                c.speed_value = ONE;
            }
        }
        let ch = &mut self.ai.scene.chars[me];
        if ch.cond[cond::REGENE_HP] != 0 {
            ch.cond[cond::REGENE_HP] = ch.cond[cond::REGENE_HP].wrapping_sub(1);
            if tick(ch.cond[cond::REGENE_HP], 60) {
                let v = at_least_1(i32::from(ch.max_hp) / 50);
                self.entry_affect(me, me, Some(me), 9, [v, 0, 0]);
            }
        }
        let ch = &mut self.ai.scene.chars[me];
        if ch.cond[cond::POISON] != 0 && self.env.menu_type == -1 {
            ch.cond[cond::POISON] = ch.cond[cond::POISON].wrapping_sub(1);
            if tick(ch.cond[cond::POISON], 90) && !(ch.is_pc() && ch.no_death) {
                let v = at_least_1(i32::from(ch.max_hp) / 100);
                self.entry_affect(me, me, Some(me), 3, [v, 0, 0]);
            }
        }
        let ch = &mut self.ai.scene.chars[me];
        if ch.cond[cond::CURSE] == 0 && self.env.count.is_multiple_of(60) {
            ch.sp = ch.sp.wrapping_add(at_least_1(i32::from(ch.max_sp) / 50));
            if ch.sp > ch.max_sp {
                ch.sp = ch.max_sp;
            }
        }
        if ch.cond[cond::REGENE_SP] != 0 {
            ch.cond[cond::REGENE_SP] = ch.cond[cond::REGENE_SP].wrapping_sub(1);
            if tick(ch.cond[cond::REGENE_SP], 60) {
                let v = at_least_1(i32::from(ch.max_sp) / 50);
                self.entry_affect(me, me, Some(me), 10, [v, 0, 0]);
            }
        }
        let ch = &mut self.ai.scene.chars[me];
        if ch.cond[cond::CURSE] != 0 {
            ch.cond[cond::CURSE] = ch.cond[cond::CURSE].wrapping_sub(1);
            if tick(ch.cond[cond::CURSE], 90) {
                let v = at_least_1(i32::from(ch.max_sp) / 100);
                self.entry_affect(me, me, Some(me), 4, [v, 0, 0]);
            }
        }
    }

    /// The start of `ccEnemy::main()` (0x00432cd0), up to `think()`: as
    /// [`Ai::begin_frame`], with the race's `freeze()` ([`freeze_g`] for a
    /// goblin) and the affects of `routineEnemy`'s timers (`CalcReal(0)`:
    /// poison, curse, regeneration) applied inside it, before `checkEnemy`. A
    /// Data Drain taken last frame ends it (the book's record, conditions
    /// cleared, off the lists, the drained form spawned).
    pub fn begin(&mut self, me: usize) -> Begin {
        {
            let e = self.ai.foes[me].as_mut().expect("an enemy's state");
            if e.drain_cnt != 0 {
                e.drain_cnt = e.drain_cnt.wrapping_sub(1);
                let ch = &mut self.ai.scene.chars[me];
                if e.drain_cnt != 0 {
                    ch.cond[cond::HOLD] = 1;
                } else {
                    ch.cond[cond::DEAD] = 0;
                }
            }
        }
        if self.ai.scene.chars[me].cond[cond::DEAD] == 3 {
            self.call(me, Call::Rule(Out::Remove));
            return Begin::Removed;
        }
        let frozen = if Kind::of_row(self.ai.t, self.en(me).ene_id) == Kind::G {
            let mut e = self.ai.foes[me].take().expect("an enemy's state");
            let f = freeze_g(self.ai.scene, me, &mut e);
            self.ai.foes[me] = Some(e);
            f
        } else {
            self.en(me).freeze_flag
        };
        if frozen {
            enemy_ai::clear_condition_enemy(&mut self.ai.scene.chars[me], &mut self.ai.out);
            self.flush(me);
            let d = self.ai.scene.chars[me].cond[cond::DEAD];
            if self.en(me).cmnd_flag || d == 2 || d == 3 {
                return Begin::Frozen { ret: 1 };
            }
            enemy_ai::clear_target(self.e(me));
            self.ai.set_act(me, act::WAIT);
            self.flush(me);
            let e = self.e(me);
            e.speed = 0;
            e.anm_num = 6;
            e.anm_num_old = 0;
            return Begin::Frozen { ret: 0 };
        }
        self.routine(me);
        self.ai.check_enemy(me);
        self.flush(me);
        if !self.en(me).drain_flag {
            return Begin::Continue;
        }
        let ene_id = self.en(me).ene_id;
        self.call(me, Call::Rule(Out::KillRecord { ene_id }));
        enemy_ai::clear_condition_enemy(&mut self.ai.scene.chars[me], &mut self.ai.out);
        self.flush(me);
        if !self.en(me).cmnd_flag {
            enemy_ai::delete_from_lists(self.ai.scene, me);
            self.call(me, Call::Rule(Out::DeleteCmnd));
            self.e(me).cmnd_flag = true;
        }
        // entryDrainEnemy (0x004359e0)
        let did = enemy_ai::drain_id(self.ai.t, ene_id);
        let pos = self.ai.scene.chars[me].pos;
        let e = self.e(me);
        let mut ent = e.ent;
        ent.pos = pos;
        ent.dirc = e.dirc;
        ent.ty = 0;
        ent.id = did;
        e.fade_flag = 0;
        let drain_cnt = if self.ai.t.check_drain_enemy(did) { 60 } else { 30 };
        self.call(me, Call::Rule(Out::DrainSpawn(DrainSpawn { ent, drain_cnt })));
        Begin::Drained
    }

    /// `ccEnemy::main()` (0x00432cd0), one frame of enemy `me` (see the
    /// module doc): returns 1 when the enemy was removed or drained (or is
    /// frozen off the lists or dying), else 0. The race's `think()` is
    /// `defaultThink`, or a gold goblin's `thinkGold` (`ccEnemyG::think`,
    /// 0x004467a0).
    pub fn enemy_main(&mut self, me: usize) -> u8 {
        match self.begin(me) {
            Begin::Removed | Begin::Drained => return 1,
            Begin::Frozen { ret } => return ret,
            Begin::Continue => {}
        }
        if self.en(me).gold.flag {
            self.ai.think_gold(me);
        } else {
            self.ai.default_think(me);
        }
        self.flush(me);
        self.ai.interrupt_think(me);
        self.flush(me);
        self.action(me);
        self.ai.end_action(me);
        self.move_enemy(me);
        self.anim_enemy(me);
        if self.en(me).disp_sw {
            self.disp_enemy(me);
        }
        self.exclusive(me);
        0
    }
}

/// `(float)fabs((double)v)` (`fptodp`, `fabs`, `dptofp`): the magnitude.
/// The values the races take it of are angles within a turn, which the
/// double round trip leaves as they are.
fn fabs_d(v: F) -> F {
    if v & 0x7f80_0000 == 0 { 0 } else { v & 0x7fff_ffff }
}

/// `ccEnemyG::freeze()` (0x00446710), the vtable's `freeze()` for a
/// goblin: frozen, a gold goblin thaws at once (`freezeFlag` cleared, back
/// on the command lists with `ccEntryCmnd` unless it was taken off for
/// good) and runs its frame; returns whether the frame stops. Other goblins
/// and the other races stop while frozen (`ccEnemy::freeze`, 0x00432fa0).
pub fn freeze_g(scene: &mut crate::scene::Scene, me: usize, e: &mut Enemy) -> bool {
    if !e.freeze_flag {
        return false;
    }
    if e.gold.flag {
        e.freeze_flag = false;
        if !e.cmnd_flag {
            entry_cmnd(scene, me);
        }
        return false;
    }
    true
}

/// `ccEntryCmnd(ch)` (gcmn 0x00519630): unless it is on the command lists,
/// the character goes on the end of its side's (party 0x7, foes 0xe0,
/// objects).
pub fn entry_cmnd(scene: &mut crate::scene::Scene, c: usize) {
    if scene.listed(c) {
        return;
    }
    let tyb = scene.chars[c].ty();
    if tyb & 7 != 0 {
        scene.pc_list.push(c);
    } else if tyb & crate::param::ty::FOE != 0 {
        scene.ene_list.push(c);
    } else {
        scene.obj_list.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_clips_change_the_eighth_letter() {
        assert_eq!(boss_clip(b"ANM_egn1_wait"), b"ANM_egnx_wait".to_vec());
        assert_eq!(boss_clip(b"ANM"), b"ANM".to_vec());
    }

    #[test]
    fn act_move_turns_and_speeds_up() {
        let mut e = Enemy::default();
        act_follow(&mut e, HALF_PI, K_0_5, K_10, K_0_5, 0);
        // halfway: speed 5, heading pi/4 for walk and facing
        assert_eq!(e.speed, K_5);
        assert_eq!(e.mdirc[2], e.dirc[2]);
        assert!((f32::from_bits(e.mdirc[2]) - std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        // a reduction slows the target speed by the turn left
        let mut e = Enemy::default();
        act_follow(&mut e, PI, 0, K_10, ONE, ONE);
        assert!(f32::from_bits(e.speed) < 10.0);
    }

    #[test]
    fn fabs_through_double_clears_the_sign() {
        assert_eq!(fabs_d(neg(PI)), PI);
        assert_eq!(fabs_d(0x8000_0000), 0);
    }
}

/// What `ccChar::EntryAffect(kind)` (gcmn 0x0056b020) will do on `on`,
/// decided before it stores the affect: `stored`, the kind and first
/// parameter kept (not on a character off the lists, not over a foe's
/// pending Data Drain, not for kinds 5 and 6, not without an `affectFunc`);
/// `influenced`, `ccEnemyInfluence` run on it (stored, the kind not masked,
/// an enemy's function). The enemy's own copy follows from these
/// ([`crate::enemy_ai::Enemy::note_landed`]), wherever the affect comes
/// from: an enemy's frame or the menus.
pub fn affect_lands(scene: &Scene, on: usize, kind: i16) -> Landed {
    let ch = &scene.chars[on];
    let stored = scene.listed(on)
        && !(ch.ty() & ty::FOE != 0 && ch.affect.ty == 13)
        && kind != 5
        && kind != 6
        && ch.affect.func != AffectFunc::None;
    let influenced = stored && ch.affect.mask & (1 << (kind & 31)) == 0 && ch.affect.func == AffectFunc::Enemy;
    Landed { stored, influenced }
}

/// [`affect_lands`]'s answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landed {
    pub stored: bool,
    pub influenced: bool,
}
