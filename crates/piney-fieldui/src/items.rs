//! Skills and items as the battle menus read them: `skillTbl`, the item
//! and equipment tables behind `ccGetItemParam` / `ccGetEquipParam`, the
//! player's lists in the save (`SetSkillList`, `SetItemList`), and the
//! rules that grey a row (`ccCheckSkillUseful`, `ccCheckItemUseful`).
//!
//! The tables are read from the executable with `GCMN.PRG` loaded over it
//! at run time; nothing of them is copied into the sources.

use piney_desktop::SaveState;
use piney_desktop::eef::{add, le, mul};

use crate::world::{CharInfo, World};

/// The save's lists (`ccSaveData`).
pub const ITEM_LIST: usize = 0x0030;
pub const PL_ITEM_LIST: usize = 0x0b70;
pub const IMP_ITEM_LIST: usize = 0x0cfc;
pub const SKILL_LIST: usize = 0x1ec4;
pub const DRAIN_EVOLUTION: usize = 0x6860;
pub const PLCOL: usize = 0x6771;
pub const EVENT_STATUS: usize = 0x64f8;
/// Items a character carries, skills a character knows.
pub const ITEMS: usize = 40;
pub const SKILL_SLOTS: usize = 20;

/// `ccSkillParam` (0x38 bytes), what the menus read of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkillParam {
    /// +0x00 `name`.
    pub name: Vec<u8>,
    /// +0x1c `triggerRange`, +0x20 `targetRange`.
    pub trigger_range: f32,
    pub target_range: f32,
    /// +0x24 `targetType`: the type bits it can aim at (3 the party).
    pub target_type: u32,
    /// +0x28 `cost`: SP.
    pub cost: i32,
    /// +0x2c `type`: bit 0 attack, 1 magic, 0x2000 centred on the user,
    /// 0x4000 an area, 0x10000 strengthen, 0x20000 weaken, 0x40000
    /// recovery.
    pub kind: u32,
    /// +0x34 `str`: the help, three lines.
    pub help: [Vec<u8>; 3],
}

/// `ccItemParam` (0x14 bytes), or an equipment row's name, price (+0x40)
/// and comment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemParam {
    pub name: Vec<u8>,
    /// +0x08 `skillID`: the skill it casts, -1 none (items only).
    pub skill: i32,
    /// +0x0c `price`.
    pub price: i32,
    /// +0x10 `str` (equipment +0x44): three lines.
    pub comment: [Vec<u8>; 3],
}

/// The tables.
#[derive(Clone, Debug, Default)]
pub struct ItemTables {
    pub skills: Vec<SkillParam>,
    /// By category 0-15.
    pub items: Vec<Vec<ItemParam>>,
    /// `ccGetItemIcon` by category.
    pub icons: [i32; 16],
    pub pages: [i32; 5],
}

impl ItemTables {
    /// The volume's, from the battle's tables (`piney_battle::Tables`):
    /// the skills, the equipment (categories 0-9) and items (10-15), and
    /// `ccGetItemIcon`'s and `SetItemList`'s tables.
    pub fn of(volume: piney_data::volume::Volume, t: &piney_battle::Tables) -> ItemTables {
        let three = |l: &[Vec<u8>]| [0, 1, 2].map(|k| l.get(k).cloned().unwrap_or_default());
        let skills = t
            .skills
            .iter()
            .map(|s| SkillParam {
                name: s.name.clone(),
                trigger_range: f32::from_bits(s.trigger_range),
                target_range: f32::from_bits(s.target_range),
                target_type: s.target_type as u32,
                cost: s.cost,
                kind: s.ty as u32,
                help: three(&s.str),
            })
            .collect();
        let equip = |e: &piney_battle::param::EquipParam| ItemParam {
            name: e.name.clone(),
            skill: -1,
            price: e.price,
            comment: three(&e.str),
        };
        let item = |i: &piney_battle::param::ItemParam| ItemParam {
            name: i.name.clone(),
            skill: i.skill_id,
            price: i.price,
            comment: three(&i.str),
        };
        let mut items: Vec<Vec<ItemParam>> = t.weapons.iter().map(|w| w.iter().map(equip).collect()).collect();
        items.extend(t.armor.iter().map(|a| a.iter().map(equip).collect()));
        items.extend(t.items.iter().map(|l| l.iter().map(item).collect()));
        let b = piney_data::tables::battle::of(volume);
        ItemTables {
            skills,
            items,
            icons: b.item_icons().try_into().unwrap_or_default(),
            pages: b.item_pages().try_into().unwrap_or_default(),
        }
    }

    /// `ccGetSkillParam(i)`.
    pub fn skill(&self, i: i32) -> Option<&SkillParam> {
        usize::try_from(i).ok().and_then(|i| self.skills.get(i))
    }

    /// `ccGetItemParam` / `ccGetEquipParam` by category and row.
    pub fn item(&self, cat: i32, id: i32) -> Option<&ItemParam> {
        let c = usize::try_from(cat).ok()?;
        let i = usize::try_from(id & 0xffff).ok()?;
        self.items.get(c)?.get(i)
    }

    /// `ccGetItemName(cat, id)`: empty past the tables.
    pub fn item_name(&self, cat: i32, id: i32) -> Vec<u8> {
        self.item(cat, id).map(|p| p.name.clone()).unwrap_or_default()
    }

    /// `ccGetItemIcon(cat, id)`.
    pub fn item_icon(&self, cat: i32) -> i32 {
        usize::try_from(cat).ok().and_then(|c| self.icons.get(c)).copied().unwrap_or(0)
    }
}

/// One `ccItemList` entry: `id`, `category`, `num` (id -1, category -1
/// when empty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: i16,
    pub cat: i8,
    pub num: i8,
}

impl Item {
    pub const NONE: Item = Item { id: -1, cat: -1, num: 0 };

    /// `(category << 16) | (u16)id`, as `ccUseItemRequest` takes it.
    pub fn code(self) -> i32 {
        (i32::from(self.cat) << 16) | i32::from(self.id as u16)
    }
}

/// `saveData.itemList[pc][k]` (`GetItemList`, MUT main 0x0017a9d0:
/// characters 18-20 in the extension).
pub fn save_item(save: &SaveState, pc: usize, k: usize) -> Item {
    let at = piney_data::save::by_id::item_list(pc) + 4 * k;
    let s = &save.save;
    Item { id: s.i16(at), cat: s.u8(at + 2) as i8, num: s.u8(at + 3) as i8 }
}

/// `saveData.plItemList[k]` (the item box, 99 entries).
pub fn pl_item(save: &SaveState, k: usize) -> Item {
    let at = PL_ITEM_LIST + 4 * k;
    let b = save.save.bytes();
    Item { id: i16::from_le_bytes([b[at], b[at + 1]]), cat: b[at + 2] as i8, num: b[at + 3] as i8 }
}

/// `saveData.skillList[pc][k]`.
pub fn save_skill(save: &SaveState, pc: usize, k: usize) -> i16 {
    save.save.i16(SKILL_LIST + pc * 40 + 2 * k)
}

/// `ccSaveData::DelItem(pc, cat, id, n)` (main 0x00177af0): `n` fewer of
/// the item; the slot emptied at none left. Key items (15, member 0) count
/// down in `impItemList[id]`.
pub fn del_item(save: &mut SaveState, pc: usize, cat: i32, id: i32, n: i32) {
    if pc == 0 && cat == 15 {
        let at = IMP_ITEM_LIST + id as usize;
        let v = (i32::from(save.save.u8(at) as i8) - n).max(0);
        save.save.set_u8(at, v as u8);
        return;
    }
    for k in 0..ITEMS {
        let it = save_item(save, pc, k);
        if i32::from(it.id) == id && i32::from(it.cat) == cat {
            let at = piney_data::save::by_id::item_list(pc) + 4 * k;
            if n < i32::from(it.num) {
                save.save.set_u8(at + 3, (i32::from(it.num) - n) as u8);
            } else {
                save.save.set_i16(at, -1);
                save.save.set_u8(at + 2, 0xff);
                save.save.set_u8(at + 3, 0);
            }
            return;
        }
    }
}

/// `SetSkillList(pc, page, list, 0)` (0x00526ed0): member `pc`'s skills
/// on a page of the Skills menu (0 attack, 1 magic, 2 recovery, 3
/// strengthen, 4 weaken, 5 Data Drain), -1 past the end.
pub fn skill_list(t: &ItemTables, save: &SaveState, pc: usize, page: i32) -> [i16; SKILL_SLOTS] {
    let mut out = [-1i16; SKILL_SLOTS];
    let mut n = 0usize;
    if page == 5 {
        let mut drains = 0;
        for k in 0..SKILL_SLOTS {
            let s = save_skill(save, pc, k);
            if (2..6).contains(&s) {
                drains += 1;
            }
        }
        if drains != 0 {
            let evo = save.save.i16(DRAIN_EVOLUTION);
            out[0] = 2;
            if evo >= 9 {
                out[1] = 3;
            }
            if evo >= 10 {
                out[2] = 4;
            }
            if evo >= 11 {
                out[3] = 5;
            }
        }
        return out;
    }
    for k in 0..SKILL_SLOTS {
        let s = save_skill(save, pc, k);
        if s < 0 {
            continue;
        }
        let kind = t.skill(i32::from(s)).map(|p| p.kind).unwrap_or(0);
        let drain = (2..6).contains(&s);
        let revive = (178..181).contains(&s);
        let take = match page {
            0 => kind & 1 != 0,
            1 => kind & 2 != 0 && kind & 0x70000 == 0 && !drain,
            2 => kind & 0x40000 != 0 || revive,
            3 => kind & 0x10000 != 0 && !revive,
            4 => kind & 0x20000 != 0,
            _ => false,
        };
        if take && n < SKILL_SLOTS {
            out[n] = s;
            n += 1;
        }
    }
    out
}

/// How many entries a skill list holds, as `SetSkillList(.., 1)` counts
/// them into `my` (the Data Drain page counts only the first two).
pub fn skill_count(t: &ItemTables, save: &SaveState, pc: usize, page: i32) -> i16 {
    if page == 5 {
        let l = skill_list(t, save, pc, 5);
        if l[0] < 0 {
            return 0;
        }
        let evo = save.save.i16(DRAIN_EVOLUTION);
        return 1 + i16::from(evo >= 9) + i16::from(evo >= 10) + i16::from(evo >= 11);
    }
    skill_list(t, save, pc, page).iter().filter(|&&s| s >= 0).count() as i16
}

/// `SetItemList(pc, page, list, 0)` (0x005272d0): member `pc`'s items on
/// a page of the Items menu.
pub fn item_list(t: &ItemTables, save: &SaveState, pc: usize, page: i32) -> [Item; ITEMS] {
    let filter = t.pages.get(page.clamp(0, 4) as usize).copied().unwrap_or(-1);
    let mut out = [Item::NONE; ITEMS];
    let mut n = 0usize;
    for k in 0..ITEMS {
        let it = save_item(save, pc, k);
        let c = i32::from(it.cat);
        let take = match filter {
            -1 => c == 10 || c == 13,
            -2 => (0..10).contains(&c),
            f => c == f,
        };
        if take {
            out[n] = it;
            n += 1;
        }
    }
    out
}

/// The tail of `SetSkillList` / `SetItemList` with the count flag: the
/// list's `my` (at least 1) and its scroll and cursor kept in range.
pub fn fit_list(l: &mut crate::tables::MenuList, count: i16) {
    l.my = count.max(1);
    let d = l.my - l.y;
    let rel = l.select - l.dy;
    if d < l.dy {
        l.dy = d.max(0);
    }
    l.select = l.dy + rel;
    if l.my - 1 < l.select {
        l.select = l.my - 1;
    } else if l.select < 0 {
        l.select = 0;
    }
}

/// `ccCheckItemUseful(cat, id)` (0x0057a6d0): 0 no use in the field, 1 a
/// target to choose, 2 used at once.
pub fn check_item_useful(cat: i32, id: i32) -> i32 {
    match cat {
        10 | 11 => 1,
        12 => 2,
        13 => {
            if id == 0 {
                1
            } else {
                2
            }
        }
        15 if (273..=280).contains(&id)
            || (287..=290).contains(&id)
            || [42, 43, 44, 45, 46, 47, 48, 49, 60, 61, 68, 69].contains(&id) =>
        {
            2
        }
        _ => 0,
    }
}

/// Whether a Data Drain skill (2-5) cannot take `c`: an enemy or boss
/// whose protect is not broken.
pub fn drain_blocked(skill: i32, c: &CharInfo) -> bool {
    if !((2..5).contains(&skill) || skill == 5) {
        return false;
    }
    (c.is(0x60) || c.is(0x80)) && c.pp == 0
}

/// `cmndDist <= range + width`.
pub fn in_reach(c: &CharInfo, range: f32) -> bool {
    le(c.cmnd_dist, add(range, c.width))
}

/// `ccCheckSkillUseful(skill)` (0x0057a890): a party member it can take
/// (for party skills; 180, the revival, a fallen one), or a character on
/// `cmndSortRoot` of its target types in reach, standing, in front of the
/// camera and (Data Drain) with its protect broken; from Mutation on no
/// Data Drain target at all while `drain_off` ([`drain_off`]).
pub fn check_skill_useful(t: &ItemTables, w: &World, skill: i32, drain_off: bool) -> bool {
    if skill <= 0 {
        return false;
    }
    let Some(p) = t.skill(skill) else { return false };
    if p.target_type & 3 != 0 {
        return w.party.iter().flatten().any(|c| skill != 180 || (c.condition[0] != 0 && c.condition[0] != 5));
    }
    w.sorted.iter().any(|c| {
        p.target_type & c.types != 0
            && in_reach(c, p.trigger_range)
            && c.condition[0] == 0
            && c.in_view
            && !(drain_off && (2..=5).contains(&skill))
            && !drain_blocked(skill, c)
    })
}

/// From Mutation on, `saveData.eventStatus[40]` (the bracelet turned
/// off, which also hides its gauge): the Data Drain skills find no target.
pub fn drain_off(volume: piney_data::volume::Volume, save: &piney_data::save::SaveData) -> bool {
    volume != piney_data::volume::Volume::Inf && save.u8(EVENT_STATUS + 40) != 0
}

/// The horizontal distance `sqrtf(|a - b|)` with z dropped, as TargetMenu
/// measures a sub-target (`sceVu0InnerProduct` of the difference), by the
/// volume's `sqrtf` (`sqrt.s` from Outbreak on).
pub fn flat_dist(volume: piney_data::volume::Volume, a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = piney_desktop::eef::sub(a[0], b[0]);
    let dy = piney_desktop::eef::sub(a[1], b[1]);
    let sq = add(mul(dx, dx), mul(dy, dy));
    f32::from_bits(piney_data::libm::sqrtf_on(volume, sq.to_bits()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_codes() {
        assert_eq!(Item { id: 3, cat: 10, num: 1 }.code(), 0xa0003);
        assert_eq!(check_item_useful(10, 0), 1);
        assert_eq!(check_item_useful(13, 1), 2);
        assert_eq!(check_item_useful(15, 44), 2);
        assert_eq!(check_item_useful(15, 50), 0);
        assert_eq!(check_item_useful(14, 0), 0);
    }

    #[test]
    fn del_item_counts_down_and_empties() {
        let mut s = SaveState::fresh();
        let at = ITEM_LIST + 4 * 2;
        s.save.set_i16(at, 7);
        s.save.set_u8(at + 2, 10);
        s.save.set_u8(at + 3, 2);
        del_item(&mut s, 0, 10, 7, 1);
        assert_eq!(save_item(&s, 0, 2), Item { id: 7, cat: 10, num: 1 });
        del_item(&mut s, 0, 10, 7, 1);
        assert_eq!(save_item(&s, 0, 2), Item::NONE);
    }

    #[test]
    fn fit_list_keeps_the_cursor_in_range() {
        let mut l = crate::tables::MenuList { y: 8, select: 6, dy: 0, ..Default::default() };
        fit_list(&mut l, 3);
        assert_eq!((l.my, l.select, l.dy), (3, 2, 0));
        fit_list(&mut l, 0);
        assert_eq!((l.my, l.select), (1, 0));
    }
}
