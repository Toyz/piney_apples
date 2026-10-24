//! Corbenik (`ccBoss08`, boss08.cpp and boss08_b, _c), Quarantine's last
//! phase boss: `bossFunc` code 7, fought twice in event 313 on field 8.
//! `ccThBoss08` plays the first form ([`first`]: `ccBoss08` and its eight
//! seeds) and, once it exits, the second (`ccBoss08_b` and its roots) in
//! the same task; the second `entry 7 7` (`eventStatus` 2) plays the third
//! (`ccBoss08_c`, its eye and killer eyes). Ported from Quarantine's
//! gcmn.prg (0x003ac9f0-0x003c18xx; names and layouts from Infection's
//! DWARF, whose gcmn carries the classes unused). docs/engine/boss-corbenik.md.

pub mod eff;
pub mod first;
pub mod second;
pub mod seed;
pub mod stardust;
#[cfg(test)]
mod tests;

use piney_data::volume::Volume;

use piney_data::field::ee;

use super::{Boss, Cx, EffKind, Out, Tbl, VF0};
use crate::geom::V4;

type F = u32;

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
/// `SelectTarget`'s radius: 999999.
const FAR: F = 0x4974_23f0;

/// `bossFunc`'s code for Corbenik, its first form's `bossTbl` row, a
/// seed's, and the file.
pub const CODE: i32 = 7;
pub const ROW: usize = 7;
pub const SEED_ROW: usize = 44;
pub const FILE: &str = "x81";
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
}

impl CorbenikData {
    /// The volume's.
    pub fn of(volume: Volume) -> CorbenikData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        CorbenikData {
            anims: s(t.corbenik_anims()),
            seed_anims: s(t.corbenik_seed_anims()),
            normal: t.corbenik_normal().to_vec(),
            b_anims: s(t.corbenik_b_anims()),
            b_normal: t.corbenik_b_normal().to_vec(),
            b_super: t.corbenik_b_super().to_vec(),
            root_anims_a: s(t.corbenik_root_anims_a()),
            root_anims_b: s(t.corbenik_root_anims_b()),
            dmg_barrier: s(t.corbenik_dmg_barrier()),
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
    /// `effExplode3(pos, spd, 1, kind)` grown 1 to 4 over 15: a seed
    /// bursts (kind 0), a laser lands (-1).
    Burst {
        pos: V4,
        spd: V4,
        kind: i32,
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
    /// laser's segment (`OBJ_ex31les1` and `les0` under fog 50 at 0xff0000)
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
    /// One of the transform's lays (`CMP_ex8efc2`) at `pos`, turned by
    /// `rot`, at `alpha`.
    TransformLay {
        pos: V4,
        rot: V4,
        alpha: F,
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
}

// --- shared by the forms ---------------------------------------------------------------

fn word(tbl: &[i32], i: i32) -> i32 {
    usize::try_from(i).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0)
}

/// `ccCheckTarget(t)` and not dead.
fn target_ok(cx: &Cx, t: Option<usize>) -> bool {
    cx.valid(t) && t.is_some_and(|t| !cx.dead(t))
}

/// `targetChar = SelectTarget(ty, 999999)`.
fn select(b: &mut Boss, cx: &mut Cx, ty: i32) -> Option<usize> {
    let t = b.select_target(cx, ty, FAR);
    cx.scene.chars[cx.me].target_char = t;
    t
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
fn manager_pass(b: &mut Boss, cx: &mut Cx) {
    let mut k = 0;
    while k < b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else {
            k += 1;
            continue;
        };
        if !e.enabled {
            b.effects.slots[k] = None;
            k += 1;
            continue;
        }
        let Some(mut fx) = e.corbenik.take() else {
            e.draw();
            k += 1;
            continue;
        };
        let alive = fx.draw(cx, b, k);
        if let Some(e) = b.effects.slots[k].as_mut() {
            e.corbenik = Some(fx);
            e.enabled = alive;
        }
        k += 1;
    }
}

/// An effect of Corbenik's own made in the first free slot.
fn make_fx(b: &mut Boss, cx: &mut Cx, kind: EffKind, fx: eff::Fx, pos: V4) -> i32 {
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.corbenik = Some(Box::new(fx));
    }
    cx.out(Out::Effect { id, kind, pos, dirc: VF0 });
    id
}
