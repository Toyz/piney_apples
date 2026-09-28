//! The battle tables, read at run time from the executable
//! (`SLUS_202.67`), `DATA/GCMN.PRG` (the field game, where nearly all of
//! them live) and `DATA/DEMO.PRG` (`charTbl`) by the addresses Infection's
//! code uses. The overlay's static constructors do not touch any of them,
//! so the file bytes are the tables as the game reads them. Names and
//! descriptions are game text: they are read from the disc and never copied
//! into the port.
//!
//! Row counts are the DWARF sizes, which `tools/battle.py` also derives
//! from the code that reads each table (`tools/test_battle.py` checks
//! the two agree).

use piney_data::Result;
use piney_data::iso::Iso;
use piney_data::tables::types as gt;
use piney_data::tables::{battle, fieldui, newgame};
use piney_data::volume::Volume;

use crate::param::*;

/// Rows a table holds (the DWARF sizes).
pub const SKILLS: usize = 304;
pub const ENEMIES: usize = 303;
pub const BOSSES: usize = 49;
pub const RACES: usize = 22;
/// An area item list's codes.
pub const AREA_ITEM_LEN: usize = 130;

/// The two overlays the tables are read from.
pub const GCMN_PATH: &str = "DATA/GCMN.PRG";
pub const DEMO_PATH: &str = "DATA/DEMO.PRG";

/// A `charTbl` row: a party member's record at level 1, as `NewGame`
/// copies it (the equipment, velocity, job and friendship sit at +0x48 in
/// the row).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharRow {
    pub name: Vec<u8>,
    pub param: SpcParam,
}

/// Every table the rules read.
#[derive(Clone, Debug, Default)]
pub struct Tables {
    /// The disc's volume, whose tables these are (and whose rules differ
    /// where the battle code changed after Infection).
    pub volume: Volume,
    pub skills: Vec<SkillParam>,
    pub boss_skills: Vec<SkillParam>,
    pub enemies: Vec<EnemyTable>,
    pub bosses: Vec<FoeRow>,
    /// Categories 10-15.
    pub items: [Vec<ItemParam>; 6],
    /// Categories 6-9: head body arm leg.
    pub armor: [Vec<EquipParam>; 4],
    /// Categories 0-5, by job.
    pub weapons: [Vec<EquipParam>; 6],
    pub chars: Vec<CharRow>,
    pub level_up: Vec<LevelUp>,
    pub exp_calc: [i16; 21],
    /// `ccEntryRaceTbl[i].raceNum`.
    pub race_num: Vec<i32>,
    pub drain_erosion: [[i32; 16]; 5],
    pub erosion: [i32; 16],
    pub player_default_items: Vec<ItemList>,
    pub player_default_important: Vec<ItemList>,
    pub spc_default_items: Vec<Vec<ItemList>>,
    pub ai_params: Vec<AiParam>,
    pub debuff_priority: [[i16; 9]; 3],
    pub buff_priority: [[i16; 5]; 3],
    /// By character id (18 on Infection, 21 from Mutation on).
    pub debuff_table_index: &'static [i16],
    pub buff_table_index: &'static [i16],
    pub debuff_skill_ability: [i16; 6],
    pub debuff_skill_attribute: [i16; 6],
    pub buff_skill_ability: [i16; 6],
    pub buff_skill_attribute: [i16; 6],
    /// [`AREA_ITEM_LISTS`] in order: box, danger box, suka box, area,
    /// idol, idol sub; each a list of `int[130]` item codes.
    pub area_items: [Vec<Vec<i32>>; 6],
    /// The party's chat lines (`ccAI::ChatMessage*`'s tables).
    pub chat: crate::party_chat::ChatTexts,
}

impl Tables {
    /// The disc's volume's.
    pub fn read(iso: &mut Iso) -> Result<Tables> {
        Ok(Tables::of(iso.volume()?))
    }

    /// The volume's (`piney_data::tables::battle`, `party_chat`,
    /// `fieldui`'s item lists, `newgame`'s `charTbl`).
    pub fn of(volume: Volume) -> Tables {
        let b = battle::of(volume);
        let f = fieldui::of(volume);
        let skills = |t: &[gt::SkillParam]| t.iter().map(SkillParam::of).collect::<Vec<_>>();
        let items = |t: &[gt::ItemParam]| t.iter().map(ItemParam::of).collect::<Vec<_>>();
        let equip = |t: &[gt::EquipmentParam]| t.iter().map(EquipParam::of).collect::<Vec<_>>();
        let lists = |t: &[gt::ItemList]| t.iter().map(ItemList::of).collect::<Vec<_>>();
        let chars = newgame::of(volume)
            .chars()
            .iter()
            .map(|c| {
                // The row is ccSpcParam's first 0x48 bytes, then the
                // equipment, velocity, job and friendship; the rest is zero.
                let mut r = [0u8; SPC_PARAM_SIZE];
                c.base.write(&mut r);
                r[0x24..0x26].copy_from_slice(&c.max_hp.to_le_bytes());
                r[0x26..0x28].copy_from_slice(&c.max_sp.to_le_bytes());
                c.elm.write(&mut r[0x28..]);
                c.equipment.write(&mut r[0xc8..]);
                r[0xd4..0xd8].copy_from_slice(&c.velocity.to_le_bytes());
                r[0xd8..0xda].copy_from_slice(&c.job.to_le_bytes());
                r[0xda..0xdc].copy_from_slice(&c.friendship.to_le_bytes());
                CharRow { name: text(c.base.name), param: SpcParam::parse(&r) }
            })
            .collect();
        let level_up = b
            .level_up()
            .iter()
            .map(|l| {
                let e = gt::CharParamElement {
                    battle_ability: l.battle_ability,
                    attribute: l.attribute,
                    tolerance: l.tolerance,
                };
                LevelUp { max_hp: l.max_hp, max_sp: l.max_sp, elm: elm_of(&e) }
            })
            .collect();
        let area = |l: &[&[i32]]| l.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
        Tables {
            volume,
            skills: skills(b.skills()),
            boss_skills: skills(b.boss_skills()),
            enemies: b.enemies().iter().map(EnemyTable::of).collect(),
            bosses: b.bosses().iter().map(FoeRow::of_boss).collect(),
            items: [
                items(b.item_r()),
                items(b.item_d()),
                items(b.item_u()),
                items(b.item_x()),
                items(b.item_t()),
                items(b.item_e()),
            ],
            armor: [equip(b.head()), equip(b.body()), equip(b.arm()), equip(b.leg())],
            weapons: [
                equip(b.weapon1()),
                equip(b.weapon2()),
                equip(b.weapon3()),
                equip(b.weapon4()),
                equip(b.weapon5()),
                equip(b.weapon6()),
            ],
            chars,
            level_up,
            exp_calc: b.exp_calc().try_into().expect("exp_calc's size"),
            race_num: b.races().iter().map(|r| r.race_num).collect(),
            drain_erosion: b.drain_erosion().try_into().expect("drain_erosion's size"),
            erosion: b.erosion().try_into().expect("erosion's size"),
            player_default_items: lists(b.player_default_items()),
            player_default_important: lists(b.player_default_important()),
            spc_default_items: b.spc_default_items().iter().map(|l| lists(l)).collect(),
            ai_params: b.ai_params().iter().map(AiParam::of).collect(),
            debuff_priority: b.debuff_priority().try_into().expect("debuff_priority's size"),
            buff_priority: b.buff_priority().try_into().expect("buff_priority's size"),
            debuff_table_index: b.debuff_table_index(),
            buff_table_index: b.buff_table_index(),
            debuff_skill_ability: b.debuff_skill_ability().try_into().expect("debuff_skill_ability's size"),
            debuff_skill_attribute: b.debuff_skill_attribute().try_into().expect("debuff_skill_attribute's size"),
            buff_skill_ability: b.buff_skill_ability().try_into().expect("buff_skill_ability's size"),
            buff_skill_attribute: b.buff_skill_attribute().try_into().expect("buff_skill_attribute's size"),
            area_items: [
                area(f.item_box_list()),
                area(f.danger_item_box_list()),
                area(f.suka_item_box_list()),
                area(f.area_item_list()),
                area(f.idol_item_list()),
                area(f.idol_sub_item_list()),
            ],
            chat: crate::party_chat::ChatTexts::of(volume),
        }
    }

    /// `ccGetSkillParam(id)`.
    pub fn skill(&self, id: i32) -> Option<&SkillParam> {
        usize::try_from(id).ok().and_then(|i| self.skills.get(i))
    }

    /// `ccGetJobWeaponParam(job, i)`: the weapon of a job's table; none for
    /// a job outside 0-5 (the game returns a null pointer).
    pub fn job_weapon(&self, job: i32, i: i32) -> Option<&EquipParam> {
        let t = self.weapons.get(usize::try_from(job).ok()?)?;
        t.get(usize::try_from(i).ok()?)
    }

    /// `ccGetEquipParam(category, i)`: categories 0-5 the weapons, 6-9 head
    /// body arm leg.
    pub fn equip(&self, category: i32, i: i32) -> Option<&EquipParam> {
        let i = usize::try_from(i).ok()?;
        match category {
            0..=5 => self.weapons[category as usize].get(i),
            6..=9 => self.armor[category as usize - 6].get(i),
            _ => None,
        }
    }

    /// `ccGetItemParam(code)`: categories 10-15.
    pub fn item(&self, code: i32) -> Option<&ItemParam> {
        let cat = code >> 16;
        let i = (code & 0xffff) as usize;
        if (10..16).contains(&cat) { self.items[cat as usize - 10].get(i) } else { None }
    }

    /// An item code's name, whatever its category.
    pub fn item_name(&self, code: i32) -> Option<&[u8]> {
        let cat = code >> 16;
        let i = code & 0xffff;
        if (0..10).contains(&cat) {
            self.equip(cat, i).map(|e| &e.name[..])
        } else {
            self.item(code).map(|p| &p.name[..])
        }
    }

    /// `ccCheckDrainEnemy(eid)` (gcmn 0x0042e330): whether `enemyTbl` row
    /// `eid` is the first of its race's group, the form an enemy takes
    /// after a Data Drain.
    pub fn check_drain_enemy(&self, eid: i32) -> bool {
        let mut total = 0;
        let mut e = eid;
        for &n in &self.race_num {
            let first = total;
            total += n;
            if eid < total {
                e = eid - first;
                break;
            }
        }
        e == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_volume_loads() {
        for v in Volume::ALL {
            let t = Tables::of(v);
            assert_eq!(t.debuff_table_index.len(), t.chars.len());
            assert_eq!(t.buff_table_index.len(), t.chars.len());
        }
    }

    #[test]
    fn drain_forms_are_group_starts() {
        let t = Tables { race_num: vec![3, 2, 4], ..Tables::default() };
        let firsts: Vec<i32> = (0..10).filter(|&e| t.check_drain_enemy(e)).collect();
        // Past the last group the id is compared as it is.
        assert_eq!(firsts, [0, 3, 5]);
    }
}
