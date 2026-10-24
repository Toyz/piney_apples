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
pub mod seed;
#[cfg(test)]
mod tests;

use piney_data::volume::Volume;

use super::Tbl;
use crate::geom::V4;

type F = u32;

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
        }
    }

    /// The table by [`Tbl`] (mode 0 the normal; no other).
    pub(crate) fn words(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::Normal => &self.normal,
            _ => &[],
        }
    }
}

/// What Corbenik and its effects show beside its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pic {
    /// A particle generator started at `pos`, by its table and rows.
    Particles { which: Gen, pos: V4 },
    /// A seed's charge (`g_energyChargeGenerator`, ff rows 1 and 4, its
    /// `distSW` off) started at its place, or deleted: `seed` its
    /// character.
    Charge { seed: usize, pos: V4, on: bool },
    /// A seed's count over its head (`ccCalcTagPos` of its place 100 up,
    /// `ccFont` type 1, `MakeSignedNum(2, n)`).
    CountDown { pos: V4, n: i32 },
    /// A seed's after-image (`EntryAfterImageAnm(anm, pos, dirc, eai)`:
    /// colour 0x005050ff at 100, fading 1/30 a draw, scale 3 to 9 over 30)
    /// at its clip's `frame`, 50 over it.
    SeedImage { seed: usize, frame: i32, pos: V4, dirc: V4 },
    /// `effExplode3(pos, spd, 1, 0)` grown 1 to 4 over 15: a seed bursts.
    Burst { pos: V4, spd: V4 },
    /// `effResistantShield(seed, 1, -1)`, its scale's x a quarter and its
    /// offset 100 down: a spell on a seed.
    SeedShield { seed: usize },
    /// `ANM_xx11wave` posed at `time` at `pos`: the wave's ring.
    Wave { time: u32, pos: V4 },
    /// `EFF_x000` on `CLT_x000c5`, 0x80e0ffff, `size` each way: a seed's
    /// flush before it bursts.
    Flush { pos: V4, size: F },
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
}
