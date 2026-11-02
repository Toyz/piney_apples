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
mod pictures;
pub mod second;
pub mod seed;
pub mod stardust;
#[cfg(test)]
mod tests;
pub mod third;

use piney_data::field::ee;
use piney_data::tables::combat::{self, Combat};
use piney_data::volume::Volume;

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
/// The clumps of x81 its forms' and parts' clips pose (the first is the
/// look's body, the first form's): the first form, the second, the
/// third's pieces, the seeds, the eyes, the killer eyes, the roots, the
/// dome and the barrier.
pub const BODIES: [&str; 14] = [
    "CMP_trall1",
    "CMP_trall2",
    "CMP_trall0",
    "CMP_trall3",
    "CMP_ex8body30",
    "CMP_ex8lbod06",
    "CMP_ex8aseed1",
    "CMP_trall5",
    "CMP_trall8",
    "CMP_trall2b",
    "CMP_trall2c",
    "CMP_trall2d",
    "CMP_trall4",
    "CMP_trallb",
];
/// An eye's flash (`ccBoss08Eye::PreDrawAnm` 0x003bed70):
/// `SetFogBlend(blend, 0xffffff)`.
pub const EYE_FLASH: u32 = 0x00ff_ffff;
/// `SetScaleAnm(1, 4, 15)`: a burst grown 1 to 4 over 15 frames.
pub const GROW: [F; 3] = [0x3f80_0000, 0x4080_0000, 0x4170_0000];
/// The seeds `ccBoss08` carries (`m_seedTbl[8]`).
pub const SEEDS: usize = 8;

/// Corbenik's tables (`tables::combat`), the volume's.
fn tables(cx: &Cx) -> &'static Combat {
    combat::of(cx.volume())
}

/// The first form's table by [`Tbl`] (`boss08ActNormalTbl` for mode 0,
/// `SetPatternMode(0)`'s; any other mode a null table).
pub(crate) fn words(v: Volume, t: Tbl) -> &'static [i32] {
    match t {
        Tbl::Normal => combat::of(v).corbenik_normal,
        _ => &[],
    }
}

/// The second form's table by [`Tbl`] (`boss08BNormalActTbl`,
/// `boss08BSuperActTbl`).
pub(crate) fn words_b(v: Volume, t: Tbl) -> &'static [i32] {
    match t {
        Tbl::Normal => combat::of(v).corbenik_b_normal,
        Tbl::Super => combat::of(v).corbenik_b_super,
        _ => &[],
    }
}

/// The third form's table by [`Tbl`] (mode 0 its first, from half its HP
/// the normal).
pub(crate) fn words_c(v: Volume, t: Tbl) -> &'static [i32] {
    match t {
        Tbl::Normal => combat::of(v).corbenik_c_first,
        Tbl::Super => combat::of(v).corbenik_c_normal,
        _ => &[],
    }
}

/// A killer eye's pattern table by its slot (the killer, the downer and
/// the rips tables).
pub(crate) fn killer_acts(v: Volume, slot: usize) -> &'static [i32] {
    let t = combat::of(v);
    [t.corbenik_killer_acts, t.corbenik_downner_acts, t.corbenik_rips_acts].get(slot).copied().unwrap_or(&[])
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
    /// `EFF_x000` on `CLT_x000c5`, 0x80e0ffff, `size` each way: a seed's
    /// flush before it bursts.
    Flush {
        pos: V4,
        size: F,
    },
    /// An element's trail (`effSmoke(pos, vec / 10, 8, 60, 114, 512, 32)`
    /// every fourth draw).
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
    LaserRadiate {
        pos: V4,
        turn: F,
    },
    /// A falling laser's head's `CMP_ex31les3` (2, 2, 3) turned about x by
    /// its pitch and z by its heading, at `pos`.
    LaserHead {
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
    /// A killer eye (turned ZYX) and a killer eye's guard
    /// (`effResistantShield(eye, magic, -1)`, its offset's height 0).
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
        from: V4,
        to: V4,
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

/// The base's patterns as Corbenik's forms run them: both w lanes 1.
const BASE_ACTS: BaseActs = BaseActs { dash_w: 0x3f80_0000, still_w: 0x3f80_0000 };

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

/// `ccCoord::SetMatrix_PosRotZYXScale(pos, rot, (s, s, s))` (main
/// 0x00136340): the scale on the unit matrix's diagonal, turned, moved.
fn pos_rot_zyx_scale(pos: V4, rot: V4, s: F) -> crate::geom::M4 {
    use crate::geom;
    let mut m = geom::unit_matrix();
    (m[0][0], m[1][1], m[2][2]) = (s, s, s);
    geom::trans_matrix(&geom::rot_matrix(&m, rot), pos)
}

/// `P2W(W2P(at) + Rz(-pi/2) Rz(d) v)`: a place `v` out from `at` along
/// `d` (the matrix turned by `d`, then by -pi/2).
fn out_from(cx: &Cx, at: V4, d: F, v: V4) -> V4 {
    use crate::geom;
    let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), crate::boss::NEG_HALF_PI), d);
    let o = geom::apply_matrix(&m, v);
    crate::boss::p2w(cx, geom::vadd(crate::boss::w2p(cx, at), o))
}
