//! The enemies' race constructors (`enemy1.cpp` - `enemyZ.cpp`, gcmn
//! 0x0043ecf0-0x00452d00, with `ccEnemy::ccEnemy` 0x00432a90,
//! `ccEntryObj::ccEntryObj` 0x0042f7d0 and `ccEnemy::initEnemy` 0x00433260):
//! what an enemy is when it is made. [`construct`] is the dispatch of
//! `ccEntryRaceTbl` (gcmn 0x005f1d60); the row tables (`RaceCtor`) are read off
//! the constructors. Models, animation players, the weapon and dust
//! controllers and palettes are the runtime's ([`World`] calls and [`Out`]s).
//! The constructors are in docs/engine/battle.md ("The race constructors").

use std::cell::RefCell;

use crate::blocks::InfoRef;
use crate::chara::{AffectFunc, Char};
use crate::enemy_ai::{self, Enemy, EntryParam, Frame};
use crate::entry::{Cx, Out, rand_s, set_hit_sw};
use crate::geom::{self, F, V4};
use crate::world::{AnmSlot, CharHit, World};

/// `ccEnemyG`'s members (+0x340 on, 0x40 bytes): the gold goblins'
/// hoarding and escape (`checkGold`, `thinkGold`, `moveGold`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gold {
    /// `goldFlag`, `goldPreHold` (+0x340 bits 0, 1), `goldDisHold` (bits 2-3).
    pub flag: bool,
    pub pre_hold: bool,
    pub dis_hold: u8,
    /// `goldVolume` (+0x342) 1-4, `goldType` (+0x344) 1-5, `goldCnt` (+0x346).
    pub volume: i16,
    pub ty: i16,
    pub cnt: i16,
    /// `goldParam[4]` (+0x348).
    pub param: [i16; 4],
    /// `goldDirc`, `goldRotate`, `goldSpeed`, `goldAccel` (+0x350 - +0x35c).
    pub dirc: F,
    pub rotate: F,
    pub speed: F,
    pub accel: F,
    /// `goldEscPos` (+0x360), `goldEscCnt` (+0x370).
    pub esc_pos: V4,
    pub esc_cnt: i32,
}

/// A race's constructor as data, row by row from its first: `eneType`
/// (the race's type table), the `(info, n)` of the `ccEnemyWeaponCtrl` and
/// `ccEnemyDustCtrl` it makes for that row, then `anmFlag` 1 and
/// `frameNum` (or neither) and the clean-up hook (`destFunc`). Read off the
/// constructors (each row's type by the race's jump table, the controllers
/// by type, and by row where a type's rows differ).
/// A row's `eneType`, weapon controller and dust controller.
type CtorRow = (i16, Option<(&'static str, i32)>, Option<(&'static str, i32)>);

struct RaceCtor {
    /// The race's first row.
    first: i32,
    rows: &'static [CtorRow],
    anm: Option<u16>,
    dest: u32,
}

/// The gold goblins among G's rows from 129 (`ccEnemyG`'s table at
/// 0x006a89c0 says which).
const G_GOLD: [bool; 38] = {
    let mut g = [false; 38];
    let mut i = 2;
    while i < 38 {
        g[i] = matches!(i, 2..=9 | 11..=14 | 18..=21 | 25..=28);
        i += 1;
    }
    g
};

/// `ccEnemy1::ccEnemy1`'s rows.
const R_1: RaceCtor = RaceCtor {
    first: 0,
    rows: &[
        (0, Some(("e1x1WpInfo", 1)), None),
        (1, Some(("e1f1WpInfo", 1)), None),
        (1, Some(("e1faWpInfo", 2)), None),
        (2, Some(("e1s1WpInfo", 2)), None),
        (2, Some(("e1s1WpInfo", 2)), None),
        (3, Some(("e1k1WpInfo", 1)), Some(("e1k1DustInfo", 1))),
        (3, Some(("e1kaWpInfo", 2)), Some(("e1k1DustInfo", 1))),
        (4, Some(("e1z1WpInfo", 2)), Some(("e1z1DustInfo", 1))),
        (4, Some(("e1z1WpInfo", 2)), Some(("e1z1DustInfo", 1))),
        (4, Some(("e1z1WpInfo", 2)), Some(("e1z1DustInfo", 1))),
        (5, Some(("e1h1WpInfo", 2)), Some(("e1h1DustInfo", 5))),
        (5, Some(("e1h1WpInfo", 2)), Some(("e1h1DustInfo", 5))),
        (5, Some(("e1h1WpInfo", 2)), Some(("e1h1DustInfo", 5))),
    ],
    anm: Some(0x0),
    dest: 0x0043_ed40,
};

/// `ccEnemy2::ccEnemy2`'s rows.
const R_2: RaceCtor = RaceCtor {
    first: 13,
    rows: &[
        (0, Some(("e2x1WpInfo", 1)), None),
        (1, Some(("e2k1WpInfo", 2)), None),
        (1, Some(("e2k1WpInfo", 2)), None),
        (2, Some(("e2p1WpInfo", 3)), None),
        (2, Some(("e2p1WpInfo", 3)), None),
        (2, Some(("e2p1WpInfo", 3)), None),
        (3, Some(("e2v1WpInfo", 1)), None),
        (3, Some(("e2v1WpInfo", 1)), None),
        (3, Some(("e2v1WpInfo", 1)), None),
    ],
    anm: None,
    dest: 0x0043_f960,
};

/// `ccEnemy3::ccEnemy3`'s rows.
const R_3: RaceCtor = RaceCtor {
    first: 22,
    rows: &[
        (0, Some(("e3x1WpInfo", 2)), None),
        (1, Some(("e3s1WpInfo", 1)), None),
        (1, Some(("e3s1WpInfo", 1)), None),
        (1, Some(("e3s1WpInfo", 1)), None),
        (1, Some(("e3s1WpInfo", 1)), None),
        (1, Some(("e3s1WpInfo", 1)), None),
        (1, Some(("e3s1WpInfo", 1)), None),
        (2, Some(("e3m1WpInfo", 2)), None),
        (2, Some(("e3m1WpInfo", 2)), None),
        (2, Some(("e3m1WpInfo", 2)), None),
        (2, Some(("e3m1WpInfo", 2)), None),
        (2, Some(("e3m1WpInfo", 2)), None),
        (2, Some(("e3m1WpInfo", 2)), None),
    ],
    anm: None,
    dest: 0x0044_0320,
};

/// `ccEnemy4::ccEnemy4`'s rows.
const R_4: RaceCtor = RaceCtor {
    first: 35,
    rows: &[
        (0, Some(("e4x1WpInfo", 2)), None),
        (1, Some(("e4m1WpInfo", 1)), None),
        (1, Some(("e4m1WpInfo", 1)), None),
        (1, Some(("e4m1WpInfo", 1)), None),
        (1, Some(("e4m1WpInfo", 1)), None),
        (1, Some(("e4m1WpInfo", 1)), None),
        (1, Some(("e4m1WpInfo", 1)), None),
        (2, Some(("e4w1WpInfo", 1)), None),
        (2, Some(("e4w1WpInfo", 1)), None),
        (2, Some(("e4w1WpInfo", 1)), None),
        (2, Some(("e4w1WpInfo", 1)), None),
        (2, Some(("e4w1WpInfo", 1)), None),
        (2, Some(("e4w1WpInfo", 1)), None),
    ],
    anm: None,
    dest: 0x0044_0bb0,
};

/// `ccEnemyA::ccEnemyA`'s rows.
const R_A: RaceCtor = RaceCtor {
    first: 48,
    rows: &[
        (0, Some(("eax1WpInfo", 2)), None),
        (1, Some(("eag1WpInfo", 2)), Some(("eag1DustInfo", 3))),
        (1, Some(("eagaWpInfo", 2)), Some(("eag1DustInfo", 3))),
        (1, Some(("eagbWpInfo", 2)), Some(("eag1DustInfo", 3))),
        (2, Some(("eac1WpInfo", 2)), Some(("eac1DustInfo", 4))),
        (2, Some(("eac1WpInfo", 2)), Some(("eac1DustInfo", 4))),
        (2, Some(("eac1WpInfo", 2)), Some(("eac1DustInfo", 4))),
        (2, Some(("eac1WpInfo", 2)), Some(("eac1DustInfo", 4))),
        (3, Some(("ea11WpInfo", 2)), Some(("ea11DustInfo", 1))),
        (3, Some(("ea11WpInfo", 2)), Some(("ea11DustInfo", 1))),
        (3, Some(("ea11WpInfo", 2)), Some(("ea11DustInfo", 1))),
        (3, Some(("ea11WpInfo", 2)), Some(("ea11DustInfo", 1))),
        (3, Some(("ea11WpInfo", 2)), Some(("ea11DustInfo", 1))),
        (4, Some(("eak1WpInfo", 6)), Some(("eak1DustInfo", 4))),
        (4, Some(("eak1WpInfo", 6)), Some(("eak1DustInfo", 4))),
        (4, Some(("eak1WpInfo", 6)), Some(("eak1DustInfo", 4))),
        (4, Some(("eak1WpInfo", 6)), Some(("eak1DustInfo", 4))),
        (4, Some(("eak1WpInfo", 6)), Some(("eak1DustInfo", 4))),
    ],
    anm: Some(0xffff),
    dest: 0x0044_15a0,
};

/// `ccEnemyB::ccEnemyB`'s rows.
const R_B: RaceCtor = RaceCtor {
    first: 66,
    rows: &[
        (0, Some(("ebx1WpInfo", 1)), None),
        (1, Some(("ebl1WpInfo", 2)), Some(("ebl1DustInfo", 3))),
        (1, Some(("ebl1WpInfo", 2)), Some(("ebl1DustInfo", 3))),
        (1, Some(("ebl1WpInfo", 2)), Some(("ebl1DustInfo", 3))),
        (2, Some(("ebq1WpInfo", 5)), Some(("ebq1DustInfo", 3))),
        (2, Some(("ebq1WpInfo", 5)), Some(("ebq1DustInfo", 3))),
        (2, Some(("ebq1WpInfo", 5)), Some(("ebq1DustInfo", 3))),
        (3, Some(("ebk1WpInfo", 6)), Some(("ebk1DustInfo", 3))),
        (3, Some(("ebk1WpInfo", 6)), Some(("ebk1DustInfo", 3))),
        (3, Some(("ebk1WpInfo", 6)), Some(("ebk1DustInfo", 3))),
        (3, Some(("ebk1WpInfo", 6)), Some(("ebk1DustInfo", 3))),
        (3, Some(("ebk1WpInfo", 6)), Some(("ebk1DustInfo", 3))),
        (3, Some(("ebk1WpInfo", 6)), Some(("ebk1DustInfo", 3))),
    ],
    anm: None,
    dest: 0x0044_2130,
};

/// `ccEnemyC::ccEnemyC`'s rows.
const R_C: RaceCtor = RaceCtor {
    first: 79,
    rows: &[
        (0, Some(("ecx1WpInfo", 1)), None),
        (1, Some(("ecc1WpInfo", 2)), Some(("ecc1DustInfo", 1))),
        (1, Some(("ecc1WpInfo", 2)), Some(("ecc1DustInfo", 1))),
        (1, Some(("ecc1WpInfo", 2)), Some(("ecc1DustInfo", 1))),
        (2, Some(("ecm1WpInfo", 2)), None),
        (2, Some(("ecm1WpInfo", 2)), None),
        (2, Some(("ecm1WpInfo", 2)), None),
        (3, Some(("ecs1WpInfo", 2)), Some(("ecs1DustInfo", 4))),
        (3, Some(("ecs1WpInfo", 2)), Some(("ecs1DustInfo", 4))),
        (3, Some(("ecs1WpInfo", 2)), Some(("ecs1DustInfo", 4))),
    ],
    anm: None,
    dest: 0x0044_2ad0,
};

/// `ccEnemyD::ccEnemyD`'s rows.
const R_D: RaceCtor = RaceCtor {
    first: 89,
    rows: &[
        (0, Some(("edx1WpInfo", 1)), None),
        (1, Some(("edi1WpInfo", 2)), Some(("edi1DustInfo", 2))),
        (1, Some(("edi1WpInfo", 2)), Some(("edi1DustInfo", 2))),
        (1, Some(("edi1WpInfo", 2)), Some(("edi1DustInfo", 2))),
        (2, Some(("edd1WpInfo", 4)), Some(("edd1DustInfo", 3))),
        (2, Some(("edd1WpInfo", 4)), Some(("edd1DustInfo", 3))),
        (2, Some(("edd1WpInfo", 4)), Some(("edd1DustInfo", 3))),
        (2, Some(("edd1WpInfo", 4)), Some(("edd1DustInfo", 3))),
        (3, Some(("edv1WpInfo", 3)), Some(("edv1DustInfo", 1))),
        (3, Some(("edv1WpInfo", 3)), Some(("edv1DustInfo", 1))),
        (3, Some(("edv1WpInfo", 3)), Some(("edv1DustInfo", 1))),
        (4, Some(("ede1WpInfo", 6)), Some(("ede1DustInfo", 6))),
        (4, Some(("ede1WpInfo", 6)), Some(("ede1DustInfo", 6))),
        (4, Some(("ede1WpInfo", 6)), Some(("ede1DustInfo", 6))),
        (4, Some(("ede1WpInfo", 6)), Some(("ede1DustInfo", 6))),
    ],
    anm: None,
    dest: 0x0044_3930,
};

/// `ccEnemyE::ccEnemyE`'s rows.
const R_E: RaceCtor = RaceCtor {
    first: 104,
    rows: &[
        (0, Some(("eex1WpInfo", 1)), None),
        (1, Some(("eer1WpInfo", 5)), Some(("eer1DustInfo", 2))),
        (1, Some(("eer1WpInfo", 5)), Some(("eer1DustInfo", 2))),
        (1, Some(("eer1WpInfo", 5)), Some(("eer1DustInfo", 2))),
        (2, Some(("eee1WpInfo", 3)), None),
        (2, Some(("eee1WpInfo", 3)), None),
        (2, Some(("eee1WpInfo", 3)), None),
        (2, Some(("eee1WpInfo", 3)), None),
        (2, Some(("eee1WpInfo", 3)), None),
        (3, Some(("eet1WpInfo", 4)), Some(("eet1DustInfo", 8))),
        (3, Some(("eet1WpInfo", 4)), Some(("eet1DustInfo", 8))),
        (3, Some(("eet1WpInfo", 4)), Some(("eet1DustInfo", 8))),
    ],
    anm: Some(0xffff),
    dest: 0x0044_4860,
};

/// `ccEnemyF::ccEnemyF`'s rows.
const R_F: RaceCtor = RaceCtor {
    first: 116,
    rows: &[
        (0, Some(("efx1WpInfo", 1)), None),
        (1, Some(("eff1WpInfo", 3)), None),
        (1, Some(("eff1WpInfo", 3)), None),
        (1, Some(("eff1WpInfo", 3)), None),
        (2, Some(("efm1WpInfo", 1)), Some(("efm1DustInfo", 1))),
        (2, Some(("efm1WpInfo", 1)), Some(("efm1DustInfo", 1))),
        (3, Some(("efs1WpInfo", 1)), Some(("efs1DustInfo", 1))),
        (3, Some(("efs1WpInfo", 1)), Some(("efs1DustInfo", 1))),
        (4, Some(("efe1WpInfo", 5)), Some(("efe1DustInfo", 3))),
        (4, Some(("efe1WpInfo", 5)), Some(("efe1DustInfo", 3))),
        (4, Some(("efe1WpInfo", 5)), Some(("efe1DustInfo", 3))),
        (4, Some(("efe1WpInfo", 5)), Some(("efe1DustInfo", 3))),
        (4, Some(("efe1WpInfo", 5)), Some(("efe1DustInfo", 3))),
    ],
    anm: Some(0xffff),
    dest: 0x0044_54f0,
};

/// `ccEnemyG::ccEnemyG`'s rows.
const R_G: RaceCtor = RaceCtor {
    first: 129,
    rows: &[
        (0, Some(("egx1WpInfo", 2)), None),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (1, Some(("egn1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (2, Some(("egf1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (2, Some(("egf1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (2, Some(("egf1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (2, Some(("egf1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (2, Some(("egf1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (3, Some(("egk1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (4, Some(("egm1WpInfo", 1)), Some(("egn1DustInfo", 1))),
        (5, Some(("ego1WpInfo", 1)), Some(("ego1DustInfo", 3))),
        (5, Some(("ego1WpInfo", 1)), Some(("ego1DustInfo", 3))),
        (5, Some(("ego1WpInfo", 1)), Some(("ego1DustInfo", 3))),
        (6, Some(("egg1WpInfo", 3)), Some(("egg1DustInfo", 5))),
        (6, Some(("egg1WpInfo", 3)), Some(("egg1DustInfo", 5))),
        (6, Some(("egg1WpInfo", 3)), Some(("egg1DustInfo", 5))),
        (6, Some(("egg1WpInfo", 3)), Some(("egg1DustInfo", 5))),
        (6, Some(("egg1WpInfo", 3)), Some(("egg1DustInfo", 5))),
        (6, Some(("egg1WpInfo", 3)), Some(("egg1DustInfo", 5))),
    ],
    anm: Some(0x0),
    dest: 0x0044_6010,
};

/// `ccEnemyH::ccEnemyH`'s rows.
const R_H: RaceCtor = RaceCtor {
    first: 167,
    rows: &[
        (0, Some(("ehx1WpInfo", 1)), None),
        (1, Some(("ehs1WpInfo", 4)), None),
        (1, Some(("ehs1WpInfo", 4)), None),
        (1, Some(("ehs1WpInfo", 4)), None),
        (2, Some(("ehh1WpInfo", 3)), Some(("ehh1DustInfo", 1))),
        (2, Some(("ehh1WpInfo", 3)), Some(("ehh1DustInfo", 1))),
        (2, Some(("ehh1WpInfo", 3)), Some(("ehh1DustInfo", 1))),
        (3, Some(("ehk1WpInfo", 3)), Some(("ehk1DustInfo", 1))),
        (3, Some(("ehk1WpInfo", 3)), Some(("ehk1DustInfo", 1))),
        (4, Some(("ehk1WpInfo", 3)), Some(("ehk1DustInfo", 1))),
        (3, Some(("ehk1WpInfo", 3)), Some(("ehk1DustInfo", 1))),
    ],
    anm: None,
    dest: 0x0044_9320,
};

/// `ccEnemyI::ccEnemyI`'s rows.
const R_I: RaceCtor = RaceCtor {
    first: 178,
    rows: &[
        (0, Some(("eix1WpInfo", 1)), None),
        (1, Some(("eii1WpInfo", 1)), Some(("eii1DustInfo", 3))),
        (1, Some(("eii1WpInfo", 1)), Some(("eii1DustInfo", 3))),
        (1, Some(("eii1WpInfo", 1)), Some(("eii1DustInfo", 3))),
        (1, Some(("eii1WpInfo", 1)), Some(("eii1DustInfo", 3))),
        (1, Some(("eii1WpInfo", 1)), Some(("eii1DustInfo", 3))),
    ],
    anm: Some(0xffff),
    dest: 0x0044_9eb0,
};

/// `ccEnemyK::ccEnemyK`'s rows.
const R_K: RaceCtor = RaceCtor {
    first: 184,
    rows: &[
        (0, Some(("ekx1WpInfo", 1)), None),
        (1, Some(("ekd1WpInfo", 3)), None),
        (1, Some(("ekd1WpInfo", 3)), None),
        (2, Some(("eks1WpInfo", 1)), None),
        (2, Some(("eks1WpInfo", 1)), None),
        (2, Some(("eks1WpInfo", 1)), None),
        (3, Some(("eka1WpInfo", 2)), Some(("eka1DustInfo", 3))),
        (3, Some(("eka1WpInfo", 2)), Some(("eka1DustInfo", 3))),
        (3, Some(("eka1WpInfo", 2)), Some(("eka1DustInfo", 3))),
    ],
    anm: None,
    dest: 0x0044_a780,
};

/// `ccEnemyL::ccEnemyL`'s rows (203-206, type 3, are not made here).
const R_L: RaceCtor = RaceCtor {
    first: 193,
    rows: &[
        (0, Some(("elx1WpInfo", 1)), None),
        (1, Some(("ell1WpInfo", 4)), None),
        (1, Some(("ell1WpInfo", 4)), None),
        (1, Some(("ell1WpInfo", 4)), None),
        (2, Some(("elw1WpInfo", 5)), Some(("elw1DustInfo", 3))),
        (2, Some(("elw1WpInfo", 5)), Some(("elw1DustInfo", 3))),
        (2, Some(("elw1WpInfo", 5)), Some(("elw1DustInfo", 3))),
        (2, Some(("elw1WpInfo", 5)), Some(("elw1DustInfo", 3))),
        (2, Some(("elw1WpInfo", 5)), Some(("elw1DustInfo", 3))),
        (2, Some(("elw1WpInfo", 5)), Some(("elw1DustInfo", 3))),
        (3, None, None),
        (3, None, None),
        (3, None, None),
        (3, None, None),
        (4, Some(("eldfWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldaWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldwWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldeWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldlWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("elddWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldfWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldzWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldfWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldfWpInfo", 3)), Some(("eld1DustInfo", 7))),
        (4, Some(("eldfWpInfo", 3)), Some(("eld1DustInfo", 7))),
    ],
    anm: None,
    dest: 0x0044_b180,
};

/// `ccEnemyP::ccEnemyP`'s rows.
const R_P: RaceCtor = RaceCtor {
    first: 218,
    rows: &[
        (0, Some(("epx1WpInfo", 1)), None),
        (1, Some(("epg1WpInfo", 1)), None),
        (1, Some(("epg1WpInfo", 1)), None),
        (1, Some(("epg1WpInfo", 1)), None),
        (2, Some(("epw1WpInfo", 2)), Some(("epw1DustInfo", 1))),
        (2, Some(("epw1WpInfo", 2)), Some(("epw1DustInfo", 1))),
        (2, Some(("epw1WpInfo", 2)), Some(("epw1DustInfo", 1))),
        (2, Some(("epw1WpInfo", 2)), Some(("epw1DustInfo", 1))),
        (3, Some(("epm1WpInfo", 2)), Some(("epm1DustInfo", 1))),
        (3, Some(("epm1WpInfo", 2)), Some(("epm1DustInfo", 1))),
        (3, Some(("epm1WpInfo", 2)), Some(("epm1DustInfo", 1))),
    ],
    anm: Some(0xffff),
    dest: 0x0044_e4f0,
};

/// `ccEnemyS::ccEnemyS`'s rows.
const R_S: RaceCtor = RaceCtor {
    first: 229,
    rows: &[
        (0, Some(("esx1WpInfo", 1)), None),
        (1, Some(("esf1WpInfo", 3)), None),
        (1, Some(("esf1WpInfo", 3)), None),
        (1, Some(("esf1WpInfo", 3)), None),
        (2, Some(("ess1WpInfo", 3)), Some(("ess1DustInfo", 3))),
        (2, Some(("ess1WpInfo", 3)), Some(("ess1DustInfo", 3))),
        (2, Some(("ess1WpInfo", 3)), Some(("ess1DustInfo", 3))),
        (3, Some(("est1WpInfo", 4)), Some(("est1DustInfo", 5))),
        (3, Some(("est1WpInfo", 4)), Some(("est1DustInfo", 5))),
        (3, Some(("est1WpInfo", 4)), Some(("est1DustInfo", 5))),
    ],
    anm: None,
    dest: 0x0044_eec0,
};

/// `ccEnemyT::ccEnemyT`'s rows.
const R_T: RaceCtor = RaceCtor {
    first: 239,
    rows: &[
        (0, Some(("etx1WpInfo", 1)), None),
        (1, Some(("etn1WpInfo", 2)), Some(("etn1DustInfo", 1))),
        (1, Some(("etn1WpInfo", 2)), Some(("etn1DustInfo", 1))),
        (1, Some(("etn1WpInfo", 2)), Some(("etn1DustInfo", 1))),
        (1, Some(("etn1WpInfo", 2)), Some(("etn1DustInfo", 1))),
        (1, Some(("etn1WpInfo", 2)), Some(("etn1DustInfo", 1))),
    ],
    anm: None,
    dest: 0x0044_f990,
};

/// `ccEnemyU::ccEnemyU`'s rows.
const R_U: RaceCtor = RaceCtor {
    first: 245,
    rows: &[
        (0, Some(("eux1WpInfo", 1)), None),
        (1, Some(("eus1WpInfo", 1)), Some(("eus1DustInfo", 1))),
        (1, Some(("eus1WpInfo", 1)), Some(("eus1DustInfo", 1))),
        (1, Some(("eus1WpInfo", 1)), Some(("eus1DustInfo", 1))),
        (2, Some(("euz1WpInfo", 1)), Some(("euz1DustInfo", 2))),
        (2, Some(("euz1WpInfo", 1)), Some(("euz1DustInfo", 2))),
        (2, Some(("euz1WpInfo", 1)), Some(("euz1DustInfo", 2))),
        (3, Some(("eud1WpInfo", 3)), Some(("eud1DustInfo", 2))),
        (3, Some(("eud1WpInfo", 3)), Some(("eud1DustInfo", 2))),
        (3, Some(("eud1WpInfo", 3)), Some(("eud1DustInfo", 2))),
        (3, Some(("eud1WpInfo", 3)), Some(("eud1DustInfo", 2))),
        (3, Some(("eud1WpInfo", 3)), Some(("eud1DustInfo", 2))),
        (3, Some(("eud1WpInfo", 3)), Some(("eud1DustInfo", 2))),
        (4, Some(("eug1WpInfo", 3)), Some(("eug1DustInfo", 3))),
        (4, Some(("eugaWpInfo", 4)), Some(("eug1DustInfo", 3))),
        (4, Some(("eug1WpInfo", 3)), Some(("eug1DustInfo", 3))),
        (4, Some(("eug1WpInfo", 3)), Some(("eug1DustInfo", 3))),
        (5, Some(("eugzWpInfo", 6)), Some(("eugzDustInfo", 7))),
        (6, Some(("euc1WpInfo", 4)), Some(("euc1DustInfo", 3))),
        (6, Some(("euc1WpInfo", 4)), Some(("euc1DustInfo", 3))),
        (6, Some(("euc1WpInfo", 4)), Some(("euc1DustInfo", 3))),
        (6, Some(("euc1WpInfo", 4)), Some(("euc1DustInfo", 3))),
    ],
    anm: None,
    dest: 0x0045_0380,
};

/// `ccEnemyV::ccEnemyV`'s rows.
const R_V: RaceCtor = RaceCtor {
    first: 267,
    rows: &[
        (0, Some(("evx1WpInfo", 1)), None),
        (1, Some(("evm1WpInfo", 3)), Some(("evm1DustInfo", 5))),
        (1, Some(("evm1WpInfo", 3)), Some(("evm1DustInfo", 5))),
        (1, Some(("evm1WpInfo", 3)), Some(("evm1DustInfo", 5))),
        (2, Some(("evb1WpInfo", 4)), None),
        (2, Some(("evbaWpInfo", 4)), None),
        (2, Some(("evbbWpInfo", 4)), None),
        (3, Some(("evr1WpInfo", 3)), Some(("evr1DustInfo", 4))),
        (3, Some(("evr1WpInfo", 3)), Some(("evr1DustInfo", 4))),
        (4, Some(("evg1WpInfo", 5)), Some(("evg1DustInfo", 3))),
        (4, Some(("evg1WpInfo", 5)), Some(("evg1DustInfo", 3))),
        (4, Some(("evg1WpInfo", 5)), Some(("evg1DustInfo", 3))),
    ],
    anm: Some(0xffff),
    dest: 0x0045_1400,
};

/// `ccEnemyW::ccEnemyW`'s rows.
const R_W: RaceCtor = RaceCtor {
    first: 279,
    rows: &[
        (0, Some(("ewx1WpInfo", 1)), None),
        (1, Some(("eww1WpInfo", 1)), None),
        (1, Some(("eww1WpInfo", 1)), None),
        (2, Some(("ewp1WpInfo", 2)), Some(("ewp1DustInfo", 1))),
        (2, Some(("ewp1WpInfo", 2)), Some(("ewp1DustInfo", 1))),
        (2, Some(("ewp1WpInfo", 2)), Some(("ewp1DustInfo", 1))),
        (3, Some(("ewi1WpInfo", 2)), Some(("ewi1DustInfo", 2))),
        (3, Some(("ewi1WpInfo", 2)), Some(("ewi1DustInfo", 2))),
        (3, Some(("ewi1WpInfo", 2)), Some(("ewi1DustInfo", 2))),
    ],
    anm: None,
    dest: 0x0045_1fe0,
};

/// `ccEnemyZ::ccEnemyZ`'s rows.
const R_Z: RaceCtor = RaceCtor {
    first: 288,
    rows: &[
        (0, Some(("ezx1WpInfo", 1)), Some(("ezx1DustInfo", 1))),
        (1, Some(("eza1WpInfo", 5)), Some(("eza1DustInfo", 1))),
        (1, Some(("eza1WpInfo", 5)), Some(("eza1DustInfo", 1))),
        (1, Some(("eza1WpInfo", 5)), Some(("eza1DustInfo", 1))),
        (2, Some(("ezb1WpInfo", 4)), None),
        (2, Some(("ezb1WpInfo", 4)), None),
        (2, Some(("ezb1WpInfo", 4)), None),
        (2, Some(("ezb1WpInfo", 4)), None),
        (2, Some(("ezb1WpInfo", 4)), None),
        (3, Some(("ezc1WpInfo", 4)), Some(("ezc1DustInfo", 3))),
        (3, Some(("ezc1WpInfo", 4)), Some(("ezc1DustInfo", 3))),
        (3, Some(("ezc1WpInfo", 4)), Some(("ezc1DustInfo", 3))),
        (3, Some(("ezc1WpInfo", 4)), Some(("ezc1DustInfo", 3))),
        (3, Some(("ezc1WpInfo", 4)), Some(("ezc1DustInfo", 3))),
        (3, Some(("ezc1WpInfo", 4)), Some(("ezc1DustInfo", 3))),
    ],
    anm: None,
    dest: 0x0045_2a50,
};

/// The races [`construct`] makes, by `ccEntryRaceTbl` index: all of them,
/// but for `ccEnemyL` (14) not its type 3 (rows 203-206, whose constructor
/// sets up lights, scaling and more: `initELG`).
pub const RACES: [i32; 22] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21];

fn ctor_of(race: i32) -> Option<&'static RaceCtor> {
    Some(match race {
        0 => &R_1,
        1 => &R_2,
        2 => &R_3,
        3 => &R_4,
        4 => &R_A,
        5 => &R_B,
        6 => &R_C,
        7 => &R_D,
        8 => &R_E,
        9 => &R_F,
        10 => &R_G,
        11 => &R_H,
        12 => &R_I,
        13 => &R_K,
        14 => &R_L,
        15 => &R_P,
        16 => &R_S,
        17 => &R_T,
        18 => &R_U,
        19 => &R_V,
        20 => &R_W,
        21 => &R_Z,
        _ => return None,
    })
}

/// `ccCheckDustColor(ch)` (gcmn 0x0043a250): the ground's colour under the
/// enemy (`ccLandHitCheck(pos, 0x20000000)` then
/// `checkHitResultAttlibute()`, its type bits 0x00f0f0f0): 116 for the
/// grey and dark grounds, else 117.
pub fn check_dust_color(world: &mut dyn World, pos: V4) -> i16 {
    world.land(pos, 0x2000_0000);
    let a = world.hit_attribute() & 0x00f0_f0f0;
    const DARK: [u32; 11] = [
        0x0030_4050,
        0x0040_5060,
        0x0050_7080,
        0x0070_90a0,
        0x0090_b0c0,
        0x00c0_c0c0,
        0x00d0_d0d0,
        0x00e0_e0e0,
        0x0040_4040,
        0x0050_5050,
        0x0060_6060,
    ];
    if DARK.contains(&a) { 116 } else { 117 }
}

/// `ccGetNameBossAnm(name, out)` (gcmn 0x0042e580): the name with its
/// eighth character made `x` (the second model's animation of a middle
/// boss). A name of seven characters or fewer is left as it is (the game's
/// copy then runs on into its stack when it is exactly seven).
pub fn boss_anm_name(name: &str) -> String {
    let mut b = name.as_bytes().to_vec();
    if b.len() > 7 {
        b[7] = b'x';
    }
    String::from_utf8_lossy(&b).into_owned()
}

/// The player's frame through the world, for `initEnemy`'s `posP`.
struct WorldFrame<'w>(RefCell<&'w mut dyn World>);

impl Frame for WorldFrame<'_> {
    fn w2p(&self, v: [u32; 4]) -> [u32; 4] {
        self.0.borrow_mut().w2p(v)
    }
    fn p2w(&self, v: [u32; 4]) -> [u32; 4] {
        self.0.borrow_mut().p2w(v)
    }
}

/// `ccEnemy::ccEnemy` (0x00432a90) then `ccEnemy::initEnemy(entry)`
/// (0x00433260) for enemy `who` of entry `ent`: the model and animation
/// `initEnemyCCS` makes (a middle boss's second model through
/// `ccGetNameBossAnm`), the body into the collision list, `ccCheckDustColor`,
/// and the rules of [`enemy_ai::init_enemy`]. A middle boss's row has no
/// animation table: the game reads its names at EE address 0xb4 (not on the
/// disc; empty here, as in the checks).
pub fn init_enemy(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, Enemy) {
    let row = &cx.t.enemies[ent.id as usize];
    let name = row.anm.and_then(|n| n.get(6)).copied().unwrap_or_default().to_string();
    cx.world.anim_set(who, AnmSlot::Main, &name);
    let middle = enemy_ai::middle_boss(cx.t, ent.id) >= 0;
    if middle {
        cx.world.anim_set(who, AnmSlot::Second, &boss_anm_name(&name));
    }
    let base = row.param.base;
    let mut hit = CharHit {
        mask: u32::MAX,
        mask2: 0x4000_0002,
        pos: ent.pos,
        radius: base.width,
        height: geom::div(base.height, geom::k(2.0)),
        kind: base.ty as u32,
        ..CharHit::default()
    };
    set_hit_sw(cx.world, who, &mut hit, true);
    let dust = check_dust_color(cx.world, ent.pos);
    let slots = enemy_ai::anim_slots(row).unwrap_or([false; 6]);
    let (mut ch, mut e) = {
        let frame = WorldFrame(RefCell::new(&mut *cx.world));
        enemy_ai::init_enemy(cx.t, ent, slots, dust, &frame, &mut *cx.cc)
    };
    ch.affect.func = AffectFunc::Enemy;
    e.anm_tbl = if row.anm.is_some() { ent.id as u32 + 1 } else { 0 };
    e.hit = hit;
    if middle {
        e.transparency = geom::ONE;
        e.set_transparency = geom::ONE;
    }
    (ch, e)
}

/// `ccEnemyG::checkGold()` (gcmn 0x00447200), from the constructor of a
/// gold goblin: its hoard (`goldVolume` 1-4 by row), its kind
/// (`goldType`: type 1 rows alternate 1 and 2, types 2-4 give 3-5), and
/// `goldParam` by both, the larger hoards drawing `ccRandS`. Volumes 3
/// and 4 widen the row's `area`, `territory` and `viewRange` to 50000 in
/// the table ([`Out::GoldRanges`]).
pub fn check_gold(cx: &mut Cx, e: &mut Enemy, who: usize) {
    const VOLUME: [i16; 27] = [1, 1, 2, 2, 3, 3, 4, 4, 0, 1, 2, 3, 4, 0, 0, 0, 1, 2, 3, 4, 0, 0, 0, 1, 2, 3, 4];
    let idx = e.ene_id - 131;
    if let Some(&v) = usize::try_from(idx).ok().and_then(|i| VOLUME.get(i))
        && v != 0
    {
        e.gold.volume = v;
    }
    match e.ene_type {
        1 => {
            if (0..8).contains(&idx) {
                e.gold.ty = if idx & 1 == 0 { 1 } else { 2 };
            }
        }
        2 => e.gold.ty = 3,
        3 => e.gold.ty = 4,
        4 => e.gold.ty = 5,
        _ => {}
    }
    if e.gold.volume == 4 || e.gold.volume == 3 {
        cx.out.push(Out::GoldRanges { ene_id: e.ene_id });
    }
    let _ = who;
    let part = i32::from(e.ene_part);
    let rnds = &mut *cx.rnds;
    let mut rs = || i32::from(rand_s(rnds));
    let p: Option<[i32; 4]> = match (e.gold.volume, e.gold.ty) {
        (1, _) => Some([0, 0, 80, 80]),
        (2, 1) => Some([1, 1, 120, 120]),
        (2, 2) | (2, 3) => Some([2, 1, 150, 150]),
        (2, 4) => Some([2, 1, 160, 160]),
        (2, 5) => Some([2, 1, 140, 140]),
        (3, 1) => Some([part & 1, part & 1, rs() % 30 + 180, rs() % 20 + 180]),
        (3, 2) => Some([part % 3, part & 1, rs() % 30 + 200, rs() % 20 + 200]),
        (3, 3) => Some([(rs() & 1) + 1, (part & 1) + 1, rs() % 30 + 160, rs() % 20 + 160]),
        (3, 4) => Some([rs() % 3, (part & 1) + 1, rs() % 30 + 170, rs() % 20 + 170]),
        (3, 5) => Some([(rs() & 1) + 1, (part & 1) + 1, rs() % 30 + 170, rs() % 20 + 170]),
        (4, 1) => Some([part & 1, part & 1, rs() % 30 + 200, rs() % 20 + 210]),
        (4, 2) => Some([part % 3, part & 1, rs() % 30 + 220, rs() % 20 + 230]),
        (4, 3) => Some([(rs() & 1) + 1, (part & 1) + 1, rs() % 30 + 180, rs() % 20 + 190]),
        (4, 4) => Some([rs() % 3, (part & 1) + 1, rs() % 30 + 190, rs() % 20 + 200]),
        (4, 5) => Some([(rs() & 1) + 1, (part & 1) + 1, rs() % 30 + 190, rs() % 20 + 200]),
        _ => None,
    };
    if let Some(p) = p {
        e.gold.param = p.map(|x| x as i16);
    }
}

/// `ccEntryRaceTbl[race].func(entry)` for the races of [`RACES`]
/// (`ccEntryEnemyB` 0x004420e0, `G` 0x00445fc0, `K` 0x0044a730, `P`
/// 0x0044e4a0, `V` 0x004513b0: the constructors `ccEnemyB::ccEnemyB`
/// 0x00442190, `ccEnemyG::ccEnemyG` 0x00446070, `ccEnemyK::ccEnemyK`
/// 0x0044a7e0, `ccEnemyP::ccEnemyP` 0x0044e550, `ccEnemyV::ccEnemyV`
/// 0x00451460) for enemy `who`; None for another race.
pub fn construct(cx: &mut Cx, race: i32, ent: &EntryParam, who: usize) -> Option<(Char, Enemy)> {
    let r = ctor_of(race)?;
    if race == 14 && (203..=206).contains(&ent.id) {
        return None;
    }
    let (ch, mut e) = init_enemy(cx, ent, who);
    let id = e.ene_id;
    if race == 10 && (154..158).contains(&id) {
        cx.out.push(Out::Clut { who });
    }
    e.act_num = 0;
    e.act_cnt = 0;
    e.anm_num_old = -1;
    e.anm_num = 6;
    if let Some(frame) = r.anm {
        e.anm_flag = 1;
        e.frame_num = frame;
    }
    if race == 10 {
        e.gold = Gold { esc_pos: geom::VF0, ..Gold::default() };
    }
    let k = id - r.first;
    let row = usize::try_from(k).ok().and_then(|i| r.rows.get(i)).copied();
    if let Some((ty, _, _)) = row {
        e.ene_type = ty;
        if race == 10 && G_GOLD[k as usize] {
            e.gold.flag = true;
        }
    }
    if e.gold.flag {
        check_gold(cx, &mut e, who);
    }
    let ty = e.ene_type;
    if race == 11 && (ty == 3 || ty == 4) {
        for slot in 0..2 {
            // ehkBreathInfo: its two breaths (the node, the effect's file)
            cx.out.push(Out::Breath { who, slot, info: InfoRef::new("ehkBreathInfo", slot) });
        }
        e.breath = true;
    }
    let block = |b: Option<(&'static str, i32)>| b.map(|(name, n)| (InfoRef::new(name, 0), n));
    let (weapon, dust) = row.map_or((None, None), |(_, w, d)| (block(w), block(d)));
    e.weapon = weapon;
    if let Some((info, n)) = weapon {
        cx.out.push(Out::Weapon { who, info, n });
    }
    e.dust = dust;
    if let Some((info, n)) = dust {
        cx.out.push(Out::Dust { who, info, n });
    }
    if race == 14 {
        // ccEnemyL: types 2 and 4 breathe; then four ccRandS for the type
        // 3's colours (drawn for every type)
        let k = match ty {
            2 => Some(id - 197),
            4 => Some(id - 201),
            _ => None,
        };
        if let Some(k) = k {
            // elBrInfo: rows 197-202 (type 2) and 207-217 (type 4) in order
            cx.out.push(Out::Breath { who, slot: 0, info: InfoRef::new("elBrInfo", k as usize) });
            e.breath = true;
        }
        for v in e.l_rand.iter_mut() {
            *v = rand_s(cx.rnds);
        }
    }
    // the spellcasters' attacks made rarer: ccEnemyG type 4, ccEnemy3 and
    // ccEnemy4 types 1 and 2
    if (race == 10 && ty == 4) || ((race == 2 || race == 3) && (ty == 1 || ty == 2)) {
        for i in 0..e.skill_num.clamp(0, 6) as usize {
            match i {
                0 | 1 => e.skill_list[i].percentage = 0x3dcc_cccd,
                2 | 3 => e.skill_list[i].percentage = 0x3ecc_cccd,
                _ => {}
            }
        }
    }
    e.dest_func = r.dest;
    Some((ch, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gold_goblins() {
        let gold: Vec<i32> = (0..38).filter(|&i| G_GOLD[i as usize]).map(|i| i + 129).collect();
        assert_eq!(gold.len(), 20);
        assert!(gold.contains(&131) && gold.contains(&157) && !gold.contains(&130) && !gold.contains(&151));
    }

    #[test]
    fn boss_names() {
        assert_eq!(boss_anm_name("ANM_ebsnut0"), "ANM_ebsxut0");
        assert_eq!(boss_anm_name("short"), "short");
    }
}
