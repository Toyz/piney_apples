//! Rewards: experience per kill (`ccExpDistributor`, gcmn 0x00571290),
//! the infection a Data Drain adds (`ccSaveData::AddLvErosion`, main
//! 0x001787a0), the item a Data Drain gives (`ccMenuCtrl::DataDrainMenu`,
//! gcmn 0x00533100) and the item a box, trap or idol holds
//! (`ccMenuCtrl::AreaItem`, gcmn 0x00544c00).

use piney_data::field::ee;
use piney_data::save::SaveData;

use crate::chara::cdiv;
use crate::damage::fptosi;
use crate::param::{SpcParam, cond};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;

/// `saveData.erosion` (+0x676e), Kite's infection, 0-100.
pub const SAVE_EROSION: usize = 0x676e;
/// `saveData.partyMemberFlag` (+0x2220) and `partyMemberExp` (+0x222c):
/// a bit per character; members out of the party with both set share the
/// experience at 60%.
pub const SAVE_PARTY_MEMBER_FLAG: usize = 0x2220;
pub const SAVE_PARTY_MEMBER_EXP: usize = 0x222c;

/// `ccPartyManager` (gcmn 0x00730310): the party's characters (up to
/// three), their ids, and how many there are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Party {
    /// Scene indices of the members (`ccChar *member[3]`).
    pub members: [Option<usize>; 3],
    /// Their character ids, -1 for an empty slot.
    pub ids: [i32; 3],
    pub num: i32,
}

impl Default for Party {
    fn default() -> Self {
        Party { members: [None; 3], ids: [-1; 3], num: 0 }
    }
}

impl Party {
    /// `checkPartyMenberNum(id)` (gcmn 0x0059d100): the slot of a
    /// character id, or -1.
    pub fn slot_of(&self, id: i32) -> i32 {
        (0..3).find(|&n| self.ids[n] == (id & 0xffff)).map_or(-1, |n| n as i32)
    }

    /// `checkPartyAnnihilation()` (gcmn 0x0059d080): every member down
    /// (`dead` set and not 5).
    pub fn annihilated(&self, scene: &Scene) -> bool {
        let down = self
            .members
            .iter()
            .flatten()
            .filter(|&&i| {
                let d = scene.chars[i].cond[cond::DEAD];
                d != 0 && d != 5
            })
            .count() as i32;
        down == self.num
    }
}

/// Experience one kill gives a member: `expCalcTbl[clamp(enemy level -
/// member level, -10, 10) + 10]`; 60% for a member out of the party.
pub fn exp_for_kill(t: &Tables, enemy_level: i16, level: i16, in_party: bool) -> i32 {
    let d = (i32::from(enemy_level) - i32::from(level)).clamp(-10, 10);
    let e = i32::from(t.exp_calc[(d + 10) as usize]);
    if in_party { e } else { cdiv(e * 60, 100) }
}

/// One member's share of a kill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpGain {
    pub id: i32,
    pub exp: i32,
    /// In the party: the game shows the amount over the member
    /// (`ccEntryFlyFontNewExp(19, exp, pos, ch)`).
    pub in_party: bool,
}

/// `ccExpDistributor(level)` (gcmn 0x00571290), once per kill: the
/// infection falls by `count % 3 + 2` (not below 0), then each of the 18
/// characters (21 from Mutation on) gains [`exp_for_kill`]: a party member (from its `ccChar`'s
/// record) unless dead or level 99, one out of the party (from the save)
/// only when flagged in both `partyMemberFlag` and `partyMemberExp`. The
/// enemy table's own `exp` is never read; levels are checked later
/// (`ccSpcCheckLevelUp`). `count` is `ccSystem.count`.
pub fn exp_distributor(
    t: &Tables,
    scene: &mut Scene,
    party: &Party,
    save: &mut SaveData,
    level: i16,
    count: u32,
) -> Vec<ExpGain> {
    let e = save.i16(SAVE_EROSION).wrapping_sub((count % 3 + 2) as i16);
    save.set_i16(SAVE_EROSION, if e < 0 { 0 } else { e });
    let mut out = Vec::new();
    for i in 0..t.chars.len() as i32 {
        let n = party.slot_of(i);
        if n != -1 {
            let Some(ci) = party.members[n as usize] else { continue };
            let ch = &mut scene.chars[ci];
            if ch.dead() {
                continue;
            }
            let Some(p) = ch.spc_mut() else { continue };
            if p.base.level >= 99 {
                continue;
            }
            let g = exp_for_kill(t, level, p.base.level, true);
            p.base.exp = p.base.exp.wrapping_add(g as i16);
            out.push(ExpGain { id: i, exp: g, in_party: true });
        } else {
            let bit = 1u32 << i;
            if save.i32(SAVE_PARTY_MEMBER_FLAG) as u32 & bit == 0 || save.i32(SAVE_PARTY_MEMBER_EXP) as u32 & bit == 0 {
                continue;
            }
            let mut p = SpcParam::from_save(save, i as usize);
            if p.base.level >= 99 {
                continue;
            }
            let g = exp_for_kill(t, level, p.base.level, false);
            p.base.exp = p.base.exp.wrapping_add(g as i16);
            let at = piney_data::save::by_id::spc_param(i as usize) + 0x10;
            save.set_i16(at, p.base.exp);
            out.push(ExpGain { id: i, exp: g, in_party: false });
        }
    }
    out
}

/// `ccSaveData::AddLvErosion(n, factor)` (main 0x001787a0): the infection
/// a Data Drain adds, `erosionTbl[clamp(n, -5, 10) + 5] * factor`
/// (`fptosi` of the EE product), capped at 100; `n` is the target's level
/// less Kite's. Returns the new infection.
pub fn add_lv_erosion(t: &Tables, erosion: i16, n: i32, factor: u32) -> i16 {
    let n = n.clamp(-5, 10);
    let add = fptosi(ee::mul(factor, ee::from_int(t.erosion[(n + 5) as usize]))) as i16;
    let e = erosion.wrapping_add(add);
    if e >= 101 { 100 } else { e }
}

/// The Data Drain skills' infection factor (`DataDrainMenu`): Data Drain
/// (2) 1.0, Drain Arc (3) and 2128 Drain (4) 1.5, Drain Heart (5) 3.0.
pub fn drain_factor(sid: i32) -> Option<u32> {
    match sid {
        2 => Some(0x3f80_0000),
        3 | 4 => Some(0x3fc0_0000),
        5 => Some(0x4040_0000),
        _ => None,
    }
}

/// The item a Data Drain gives (`DataDrainMenu`, read from the code): a
/// boss always gives `item[0]`; for an enemy the roll is `rand() % 100 +
/// 60` for 2128 Drain and Drain Heart (sid 4 up), else `rand() % 100 +
/// erosion / 2` (the infection already raised by this drain): 96 and up
/// `item[2]`, 50 and up `item[1]`, else `item[0]`. Returns the item code
/// and the roll.
pub fn drain_drop(item: &[i32; 3], erosion: i16, sid: i32, boss: bool, rng: &mut dyn Rng) -> (i32, Option<i32>) {
    if boss {
        return (item[0], None);
    }
    let roll = if sid >= 4 { rng.rand() % 100 + 60 } else { rng.rand() % 100 + i32::from(erosion) / 2 };
    let k = if roll >= 96 {
        2
    } else if roll >= 50 {
        1
    } else {
        0
    };
    (item[k], Some(roll))
}

/// Which list `ccMenuCtrl::AreaItem` picks from, by its `kind` argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaList {
    /// 0: an item box (`ItemBoxList[server * 6 + field attribute]`).
    Box,
    /// 1: a danger box (`DangerItemBoxList`).
    DangerBox,
    /// 2: `SukaItemBoxList[server]`.
    SukaBox,
    /// 3 and anything else: a trap's (`areaItemList`).
    Area,
    /// 4: an idol (`idolItemList`).
    Idol,
    /// 5: `idolSubItemList[server]`.
    IdolSub,
}

impl AreaList {
    pub fn from_kind(kind: i32) -> AreaList {
        match kind {
            0 => AreaList::Box,
            1 => AreaList::DangerBox,
            2 => AreaList::SukaBox,
            4 => AreaList::Idol,
            5 => AreaList::IdolSub,
            _ => AreaList::Area,
        }
    }
}

/// Where the area's item level comes from (`AreaItem`'s inputs besides
/// the RNG): `ccGame.server`, `WORLD_MAN::GetFieldAttrb()`, `ccGame.floor`
/// and either the event area's level (`GetEventAreaInfo()->+0x1c`, when
/// `ccGame.field` is set) or the field's (`(level - 1) * 25 + 10 +
/// offset`, the world manager's field info +0x20 and +0x28).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AreaInfo {
    pub server: i32,
    pub field_attr: i32,
    pub floor: i32,
    /// The base index: the event area's level, or `(level - 1) * 25 + 10
    /// + offset`.
    pub base: i32,
}

/// `ccMenuCtrl::AreaItem(item, kind)` (gcmn 0x00544c00): a fixed item
/// (`item >= 0`) as it is; otherwise the list's entry at `base + floor +
/// 1 + rand() % 5`, at most 129.
pub fn area_item(t: &Tables, item: i32, kind: i32, area: &AreaInfo, rng: &mut dyn Rng) -> i32 {
    if item >= 0 {
        return item;
    }
    let (k, i) = match AreaList::from_kind(kind) {
        AreaList::Box => (0, area.server * 6 + area.field_attr),
        AreaList::DangerBox => (1, area.server * 6 + area.field_attr),
        AreaList::SukaBox => (2, area.server),
        AreaList::Area => (3, area.server * 6 + area.field_attr),
        AreaList::Idol => (4, area.server * 6 + area.field_attr),
        AreaList::IdolSub => (5, area.server),
    };
    let mut n = area.base + area.floor + 1 + rng.rand() % 5;
    if n >= 130 {
        n = 129;
    }
    usize::try_from(i)
        .ok()
        .and_then(|i| t.area_items[k].get(i))
        .and_then(|l| usize::try_from(n).ok().and_then(|n| l.get(n)))
        .copied()
        .unwrap_or(0)
}

/// `ccSpcCheckLevelUp()` (gcmn 0x005a1630): outside the Root Town
/// (`area` non-zero), each member 1-17 (1-20 from Mutation on) away from
/// the party but sharing
/// experience (both flags set) levels up in the save from its stored exp:
/// 1000 a level, the character's `LevelUpParamTbl` row each time (stats
/// clamped 0..999, maxHP and maxSP not clamped), level 99 at most with
/// exp 0. The party's own members level up through
/// [`crate::chara::check_level_up`].
pub fn level_up_absent(t: &Tables, save: &mut SaveData, party: &Party, area: i32) {
    if area == 0 {
        return;
    }
    let flags = save.i32(SAVE_PARTY_MEMBER_FLAG) as u32;
    let exp_flags = save.i32(SAVE_PARTY_MEMBER_EXP) as u32;
    for i in 1..t.chars.len() {
        let bit = 1u32 << i;
        if flags & bit == 0 || exp_flags & bit == 0 || party.slot_of(i as i32) != -1 {
            continue;
        }
        let mut p = SpcParam::from_save(save, i);
        let g = usize::try_from(p.base.id).ok().and_then(|k| t.level_up.get(k)).copied().unwrap_or_default();
        while i32::from(p.base.exp) - 999 > 0 {
            p.base.exp = p.base.exp.wrapping_sub(1000);
            p.base.level = p.base.level.wrapping_add(1);
            crate::chara::add_elm(&mut p.elm, &g.elm, 999, 0);
            p.max_hp = p.max_hp.wrapping_add(g.max_hp);
            p.max_sp = p.max_sp.wrapping_add(g.max_sp);
            if p.base.level >= 99 {
                p.base.level = 99;
                p.base.exp = 0;
                break;
            }
        }
        p.store(save, i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_table_clamps() {
        let t = Tables { exp_calc: std::array::from_fn(|i| i as i16 * 10), ..Tables::default() };
        assert_eq!(exp_for_kill(&t, 50, 1, true), 200);
        assert_eq!(exp_for_kill(&t, 1, 50, true), 0);
        assert_eq!(exp_for_kill(&t, 5, 5, true), 100);
        assert_eq!(exp_for_kill(&t, 5, 5, false), 60);
    }

    #[test]
    fn party_slots() {
        let p = Party { members: [Some(0), Some(1), None], ids: [0, 7, -1], num: 2 };
        assert_eq!(p.slot_of(7), 1);
        assert_eq!(p.slot_of(3), -1);
    }
}
