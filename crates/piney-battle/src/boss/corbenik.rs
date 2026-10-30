//! Corbenik (`ccBoss08`, boss08.cpp and boss08_b, _c), Quarantine's last
//! phase boss: `bossFunc` code 7, fought twice in event 313 on field 8.
//! `ccThBoss08` plays the first form ([`first`]: `ccBoss08` and its eight
//! seeds) and, once it exits, the second (`ccBoss08_b` and its roots) in
//! the same task; the second `entry 7 7` (`eventStatus` 2) plays the third
//! (`ccBoss08_c`, its eye and killer eyes). Ported from Quarantine's
//! gcmn.prg (0x003ac9f0-0x003c18xx; names and layouts from Infection's
//! DWARF, whose gcmn carries the classes unused). docs/engine/boss-corbenik.md.

pub mod eff;
pub mod eye;
pub mod first;
pub mod laser;
pub mod second;
pub mod seed;
pub mod stardust;
#[cfg(test)]
mod tests;
pub mod third;

use piney_data::volume::Volume;

use piney_data::field::ee;

use super::{BaseActs, BossBase, Cx, Fx as BossFx, PassMode, Tbl, word};
use crate::geom::V4;

type F = u32;

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;

/// `bossFunc`'s code for Corbenik, its first form's `bossTbl` row, a
/// seed's, and the file.
pub const CODE: i32 = 7;
pub const ROW: usize = 7;
pub const SEED_ROW: usize = 44;
pub const FILE: &str = "x81";
/// `SetScaleAnm(1, 4, 15)`: a burst grown 1 to 4 over 15 frames.
pub const GROW: [F; 3] = [0x3f80_0000, 0x4080_0000, 0x4170_0000];
/// The seeds `ccBoss08` carries (`m_seedTbl[8]`).
pub const SEEDS: usize = 8;

/// Corbenik's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CorbenikData {
    /// `boss08AnmTbl`, `boss08SeedAnmTbl`: the clip by act.
    pub anims: Vec<Option<String>>,
    pub seed_anims: Vec<Option<String>>,
    /// `boss08ActNormalTbl` through its closing -1 (`SetPatternMode(0)`'s;
    /// any other mode is a null table).
    pub normal: Vec<i32>,
    /// The second form's: `boss08bAnmTbl`, `boss08BNormalActTbl`,
    /// `boss08BSuperActTbl`, the roots' two clip tables and the barrier's
    /// hit clip (`boss08DmgBarriar`).
    pub b_anims: Vec<Option<String>>,
    pub b_normal: Vec<i32>,
    pub b_super: Vec<i32>,
    pub root_anims_a: Vec<Option<String>>,
    pub root_anims_b: Vec<Option<String>>,
    pub dmg_barrier: Vec<Option<String>>,
    /// The third form's: `boss08cAnmTbl`, its two pattern tables (mode 0
    /// and, from half its HP, mode 1), the eyes' and killer eyes' clips,
    /// the killer eyes' three pattern tables (by slot), their skills and
    /// compels, the slave attack's rows, the killer eyes' throws and the
    /// reflected laser's rings (radii, heights).
    pub c_anims: Vec<Option<String>>,
    pub c_first: Vec<i32>,
    pub c_normal: Vec<i32>,
    pub eye_anims: Vec<Option<String>>,
    pub killer_anims: Vec<Option<String>>,
    pub killer_acts: [Vec<i32>; 3],
    pub killer_skills: Vec<i32>,
    pub killer_compels: Vec<i32>,
    pub slave_ids: Vec<i32>,
    pub killer_turns: Vec<F>,
    pub refrect_radii: Vec<F>,
    pub refrect_heights: Vec<F>,
}

impl CorbenikData {
    /// The volume's.
    pub fn of(volume: Volume) -> CorbenikData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        CorbenikData {
            anims: s(t.corbenik_anims),
            seed_anims: s(t.corbenik_seed_anims),
            normal: t.corbenik_normal.to_vec(),
            b_anims: s(t.corbenik_b_anims),
            b_normal: t.corbenik_b_normal.to_vec(),
            b_super: t.corbenik_b_super.to_vec(),
            root_anims_a: s(t.corbenik_root_anims_a),
            root_anims_b: s(t.corbenik_root_anims_b),
            dmg_barrier: s(t.corbenik_dmg_barrier),
            c_anims: s(t.corbenik_c_anims),
            c_first: t.corbenik_c_first.to_vec(),
            c_normal: t.corbenik_c_normal.to_vec(),
            eye_anims: s(t.corbenik_eye_anims),
            killer_anims: s(t.corbenik_killer_anims),
            killer_acts: [
                t.corbenik_killer_acts.to_vec(),
                t.corbenik_downner_acts.to_vec(),
                t.corbenik_rips_acts.to_vec(),
            ],
            killer_skills: t.corbenik_killer_skills.to_vec(),
            killer_compels: t.corbenik_killer_compels.to_vec(),
            slave_ids: t.corbenik_slave_ids.to_vec(),
            killer_turns: bits(t.corbenik_killer_turns),
            refrect_radii: bits(t.corbenik_refrect_radii),
            refrect_heights: bits(t.corbenik_refrect_heights),
        }
    }

    /// The third form's table by [`Tbl`] (0 the first, 1 the normal).
    pub(crate) fn words_c(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::Normal => &self.c_first,
            Tbl::Super => &self.c_normal,
            _ => &[],
        }
    }

    /// The table by [`Tbl`] (mode 0 the normal; no other).
    pub(crate) fn words(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::Normal => &self.normal,
            _ => &[],
        }
    }

    /// The second form's table by [`Tbl`] (0 the normal, 1 the super).
    pub(crate) fn words_b(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::Normal => &self.b_normal,
            Tbl::Super => &self.b_super,
            _ => &[],
        }
    }
}

/// What Corbenik and its effects show beside its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pic {
    /// A particle generator started at `pos`, by its table and rows.
    Particles {
        which: Gen,
        pos: V4,
    },
    /// A seed's charge (`g_energyChargeGenerator`, ff rows 1 and 4, its
    /// `distSW` off) started at its place, or deleted: `seed` its
    /// character.
    Charge {
        seed: usize,
        pos: V4,
        on: bool,
    },
    /// A seed's count over its head (`ccCalcTagPos` of its place 100 up,
    /// `ccFont` type 1, `MakeSignedNum(2, n)`).
    CountDown {
        pos: V4,
        n: i32,
    },
    /// A seed's after-image (`EntryAfterImageAnm(anm, pos, dirc, eai)`:
    /// colour 0x005050ff at 100, fading 1/30 a draw, scale 3 to 9 over 30)
    /// at its clip's `frame`, 50 over it.
    SeedImage {
        seed: usize,
        frame: i32,
        pos: V4,
        dirc: V4,
    },
    /// `effExplode3(pos, spd, scale, kind)`, then `SetScaleAnm(grow)` when
    /// given: a seed bursts (kind 0), a laser lands (-1, a random kind the
    /// effect draws from `ccRand`), the tera laser strikes.
    Burst {
        pos: V4,
        spd: V4,
        scale: F,
        kind: i32,
        grow: Option<[F; 3]>,
    },
    /// `effResistantShield(seed, 1, -1)`, its scale's x a quarter and its
    /// offset 100 down: a spell on a seed.
    SeedShield {
        seed: usize,
    },
    /// `ANM_xx11wave` posed at `time` at `pos`: the wave's ring.
    Wave {
        time: u32,
        pos: V4,
    },
    /// `EFF_x000` on `CLT_x000c5`, 0x80e0ffff, `size` each way: a seed's
    /// flush before it bursts.
    Flush {
        pos: V4,
        size: F,
    },
    /// The second form's barrier (`OBJ_ex8bria` at its body's
    /// `OBJ_ex8body2`), or its hit clip (`boss08DmgBarriar`) posed at
    /// `time` while a blow flashes it.
    Barrier,
    BarrierHit {
        time: u32,
    },
    /// The body drawn at a press's image: its clip's pose, the place, the
    /// heading, its transparency.
    PressImage {
        time: u32,
        pos: V4,
        dirc: V4,
        alpha: F,
    },
    /// An element (`BODYELEMENT_T`'s clip, the fog 3x the body's height,
    /// layer 3) at `pos` turned XYZ by `dirc`, and its trail
    /// (`effSmoke(pos, vec / 10, 8, 60, 114, 512, 32)` every fourth draw).
    Element {
        pos: V4,
        dirc: V4,
        alpha: F,
    },
    ElementSmoke {
        pos: V4,
        vec: V4,
    },
    /// `effRadiateSomething(pos, (0, DEG2RAD(-16384), 0), 45, 4, 4)`: the
    /// ground broken by the root drain and each root.
    Radiate {
        pos: V4,
    },
    /// `effSmoke(pos, rot, 8, 60, 114, 512, 32)`: the root drain's four.
    Smoke {
        pos: V4,
        rot: V4,
    },
    /// The star dust's: a star (`EFF_x000` on `CLT_x000c7`, 0x80e0ffff,
    /// `size` each way, at `scale`), the core it gathers into, a bolt's
    /// drawn points (`CMP_x012` segments, `width` across, at `alpha`), a
    /// laser's segment (`OBJ_ex31les1`, then `les0` under fog 50 at
    /// 0xff0000, turned by its trail's pitch and heading, a twentieth long)
    /// and a laser's landing radiate
    /// (`effRadiateSomething2(end, (0, DEG2RAD(-16384), turn), 45, 4, 4)`).
    Star {
        pos: V4,
        scale: F,
        size: F,
    },
    StarCore {
        pos: V4,
        scale: F,
    },
    Thunder {
        pts: Vec<V4>,
        shown: i32,
        alpha: F,
        width: F,
    },
    Laser {
        pos: V4,
        pitch: F,
        yaw: F,
        alpha: F,
    },
    LaserRadiate {
        pos: V4,
        turn: F,
    },
    /// A falling laser's head: `ANM_ex31les3` (posed at `time`) and
    /// `CMP_ex31les3` (2, 2, 3) turned about x by its pitch and z by its
    /// heading, at `pos`.
    LaserHead {
        time: u32,
        pos: V4,
        pitch: F,
        yaw: F,
    },
    /// One of the transform's lays (`CMP_ex8efc2`) at `pos`, turned by
    /// `rot`, at `alpha`.
    TransformLay {
        pos: V4,
        rot: V4,
        alpha: F,
    },
    /// An eye (`ccBoss08Eye`'s `ANM_ex8nut6` at its clip's time, turned
    /// XYZ, its flash's fog `blend` toward white), a killer eye (turned
    /// ZYX) and a killer eye's guard (`effResistantShield(eye, magic,
    /// -1)`, its offset's height 0).
    Eye {
        eye: usize,
        time: u32,
        pos: V4,
        dirc: V4,
        blend: F,
        alpha: F,
    },
    Killer {
        eye: usize,
        time: u32,
        pos: V4,
        dirc: V4,
        alpha: F,
    },
    KillerShield {
        eye: usize,
        magic: i32,
    },
    /// The tera laser's beam (`CMP_ex8efc3` scaled by `scale`, turned
    /// `-pi/2` about x then XYZ by `rot`, at `pos`), its charge's sparks
    /// (`EFF_x011` on `CLT_x011c2`, each at its place, frame, alpha and turn) and
    /// a reflected laser's head (`ANM_ex31les3`'s `CMP_ex31les3` from `from`
    /// toward `to`) and segments (`OBJ_ex31les1` and `les0` under fog 50,
    /// `len` a thousandth of their length, at `alpha`).
    TeraBeam {
        pos: V4,
        rot: V4,
        scale: V4,
    },
    Spark {
        pos: V4,
        frame: i32,
        alpha: F,
        turn: F,
    },
    RefrectHead {
        time: u32,
        from: V4,
        to: V4,
    },
    RefrectSeg {
        from: V4,
        to: V4,
        alpha: F,
    },
    /// `ccBossEffExplode2Create(pos)` (QUA gcmn 0x0037d760): five
    /// `BurstSmokeGenerator` rows about `pos` and `ccEnemyEffDust(pos, 10,
    /// 5)`.
    Explode2 {
        pos: V4,
    },
    /// A kept particle generator moved to `pos` (it follows a place), or
    /// killed (`ParticleKill`).
    GenMove {
        key: u32,
        pos: V4,
    },
    GenKill {
        key: u32,
    },
    /// `worldman->eventArea` +0x2bc (QUA gcmn 0x0030aa40): the arena's
    /// extra clump drawn (on) or not; the lasers hide it.
    ArenaClump {
        on: bool,
    },
    /// The root drain's own root (`ANM_ex8natc4`) at the target.
    RootL {
        time: u32,
        pos: V4,
        alpha: F,
    },
    /// A particle generator the form keeps under `key` (its
    /// `ccCheckParticleGenerator` answered by [`crate::boss::Cx::gen_alive`]),
    /// started at `pos`.
    Gen {
        key: u32,
        which: Gen,
        pos: V4,
    },
}

/// The generators Corbenik's code starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gen {
    /// `OnThinkWave`'s two: `MagicSquareGenerator` row 7 (ff 10, 1) and
    /// row 0 (ff 0, 12), their `distSW` off.
    WaveRing,
    WaveSquare,
    /// `g_seedShotSmokeGen` (ff 0, 1), `distSW` off: the seeds thrown.
    SeedShot,
    /// `g_rootDrainGen` rows 0 (ff 0, 1), 1 (ff 6), 1 (ff 3, 2) and 2
    /// (ff 4, 2, 5), their pause bits off: the drain's four in turn.
    RootDrain0,
    RootDrain1a,
    RootDrain1b,
    RootDrain2,
    /// `g_transformGen` (`g_transformFF`): the barrier's transform.
    Transform,
    /// The third form's: `g_generateEyeGen` (`g_generateEyeFF`) where the
    /// killer eyes are born; `BossDeadGenerator` row 0 (ff 0, 1, texture
    /// 105) and row 1 (ff 6 or ff 3, 2, texture 31) at its death;
    /// `g_energyChargeGenerator` row 2 (ff 2, 5) as the laser charges and
    /// row 4 (ff 5, 7, texture 108) under the tera laser's charge; the
    /// charge's own (QUA 0x00515490, ff 0x005154d0) and the beam's fall
    /// (0x005154f0, ff 0x00515530, following the beam).
    GenerateEye,
    BossDead0,
    BossDead1a,
    BossDead1b,
    LaserCharge,
    TeraCharge,
    TeraChargeGlow,
    TeraFall,
}

/// The base's patterns as Corbenik's forms run them: the neutral let go,
/// both w lanes 1.
const BASE_ACTS: BaseActs = BaseActs { other_forbid: 1, dash_w: 0x3f80_0000, still_w: 0x3f80_0000 };

fn bits(v: &[f32]) -> Vec<F> {
    v.iter().map(|f| f.to_bits()).collect()
}

// --- shared by the forms ---------------------------------------------------------------

/// `anm->SetAnm(anm->chunk)`: the clip it plays from its start.
fn restart(anm: &mut crate::boss::Anm, cx: &Cx) {
    if let Some(c) = anm.clip.clone() {
        anm.set(&c, cx.clips);
    }
}

/// An angle past pi taken a turn back (once).
fn wrap_hi(v: F) -> F {
    if ee::le(v, PI) { v } else { ee::sub(v, TWO_PI) }
}

/// An angle below -pi taken a turn on (once).
fn wrap_lo(v: F) -> F {
    if ee::lt(v, 0xc049_0fdb) { ee::add(v, TWO_PI) } else { v }
}

/// `ccBossEffManager::Draw`'s pass: each enabled effect's `Draw` in slot
/// order, a disabled one freed. Corbenik's own run their rules here.
fn manager_pass(b: &mut BossBase, cx: &mut Cx) {
    manager_pass_x(b, cx, None);
}

/// The pass with the third form's members at hand (its eyes).
fn manager_pass_x(b: &mut BossBase, cx: &mut Cx, mut third: Option<&mut third::Third>) {
    b.effect_pass(PassMode::Set, |b, k, _, fx| {
        let BossFx::Corbenik(fx) = fx else { return true };
        fx.draw(cx, b, k, third.as_deref_mut())
    });
}
