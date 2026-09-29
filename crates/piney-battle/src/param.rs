//! The game's battle structures as values: `ccCharParamElement` (the
//! sixteen stats), `ccCondition`, `ccCharBaseParam`, `ccSpcParam` (a party
//! member's record, as the save keeps it) and the table rows of skills,
//! items, equipment, enemies and bosses. Offsets are Infection's DWARF.

use piney_data::tables::sjis::encode;
use piney_data::tables::types as gt;

/// `ccCharParamElement`: `ccBattleAbility` (pAtk pDef pHit pEva mAtk mDef
/// mHit mEva), `ccAttribute` (soil water fire wind thunder dark) and
/// `ccTolerance` (spirit body), sixteen shorts.
pub type Elm = [i16; 16];

/// Indices into an [`Elm`].
pub mod elm {
    pub const P_ATK: usize = 0;
    pub const P_DEF: usize = 1;
    pub const P_HIT: usize = 2;
    pub const P_EVA: usize = 3;
    pub const M_ATK: usize = 4;
    pub const M_DEF: usize = 5;
    pub const M_HIT: usize = 6;
    pub const M_EVA: usize = 7;
    /// The six elements, soil water fire wind thunder dark.
    pub const ATTR: usize = 8;
    pub const SOIL: usize = 8;
    pub const WATER: usize = 9;
    pub const FIRE: usize = 10;
    pub const WIND: usize = 11;
    pub const THUNDER: usize = 12;
    pub const DARK: usize = 13;
    pub const SPIRIT: usize = 14;
    pub const BODY: usize = 15;
}

/// `ccCondition`'s sixteen shorts, by index.
pub mod cond {
    pub const DEAD: usize = 0;
    pub const HOLD: usize = 1;
    pub const DRAIN_HP: usize = 2;
    pub const DRAIN_SP: usize = 3;
    pub const CRITICAL: usize = 4;
    pub const DYING: usize = 5;
    pub const INVINCIBLE: usize = 6;
    pub const REGENE_SP: usize = 7;
    pub const CURSE: usize = 8;
    pub const SLEEP: usize = 9;
    pub const CONFUSION: usize = 10;
    pub const CHARM: usize = 11;
    pub const REGENE_HP: usize = 12;
    pub const POISON: usize = 13;
    pub const PARALYSIS: usize = 14;
    pub const SPEED: usize = 15;
}

/// 1.0f, the bits `speedValue` holds at rest.
pub const F_ONE: u32 = 0x3f80_0000;

/// `ccCondition` (0x24 bytes): the sixteen shorts, then `speedValue` as
/// its float bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Condition {
    pub v: [i16; 16],
    pub speed_value: u32,
}

impl Default for Condition {
    fn default() -> Self {
        Condition { v: [0; 16], speed_value: F_ONE }
    }
}

impl std::ops::Index<usize> for Condition {
    type Output = i16;
    fn index(&self, i: usize) -> &i16 {
        &self.v[i]
    }
}

impl std::ops::IndexMut<usize> for Condition {
    fn index_mut(&mut self, i: usize) -> &mut i16 {
        &mut self.v[i]
    }
}

/// `ccCharBaseParam.type` bits.
pub mod ty {
    /// A party member (Kite 0x1, the others 0x2, 0x4 on ccSpcParam).
    pub const PC: i32 = 0x07;
    pub const ENEMY: i32 = 0x60;
    /// An `enemyTbl` row's whole type for a middle boss (the Data Bugs):
    /// its base form's row in `gold` (`ccCheckMiddleBoss`).
    pub const MIDDLE_BOSS: i32 = 0x40;
    pub const BOSS: i32 = 0x80;
    pub const FOE: i32 = ENEMY | BOSS;
}

pub(crate) fn le16(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([b[at], b[at + 1]])
}

pub(crate) fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

pub(crate) fn put16(b: &mut [u8], at: usize, v: i16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

pub(crate) fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

pub(crate) fn read_elm(b: &[u8], at: usize) -> Elm {
    std::array::from_fn(|i| le16(b, at + 2 * i))
}

pub(crate) fn write_elm(b: &mut [u8], at: usize, e: &Elm) {
    for (i, v) in e.iter().enumerate() {
        put16(b, at + 2 * i, *v);
    }
}

/// `ccCharBaseParam` (0x24 bytes). Pointers are kept as the EE addresses
/// the game stores.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Base {
    pub name: u32,
    pub ccsname: u32,
    pub ty: i32,
    pub id: i16,
    pub level: i16,
    pub exp: i16,
    pub gold: i32,
    /// Float bits.
    pub height: u32,
    pub width: u32,
    pub msg: u32,
    /// The name's text where the base is a table's row (an enemy's, a
    /// gimmick's, an NPC's); the game keeps only `name`, its pointer.
    pub label: Option<&'static str>,
}

impl Base {
    pub fn parse(b: &[u8]) -> Base {
        Base {
            name: le32(b, 0),
            ccsname: le32(b, 4),
            ty: le32(b, 8) as i32,
            id: le16(b, 0xc),
            level: le16(b, 0xe),
            exp: le16(b, 0x10),
            gold: le32(b, 0x14) as i32,
            height: le32(b, 0x18),
            width: le32(b, 0x1c),
            msg: le32(b, 0x20),
            label: None,
        }
    }

    /// Every member; the padding at +0x12 is left as it is.
    pub fn write(&self, b: &mut [u8]) {
        put32(b, 0, self.name);
        put32(b, 4, self.ccsname);
        put32(b, 8, self.ty as u32);
        put16(b, 0xc, self.id);
        put16(b, 0xe, self.level);
        put16(b, 0x10, self.exp);
        put32(b, 0x14, self.gold as u32);
        put32(b, 0x18, self.height);
        put32(b, 0x1c, self.width);
        put32(b, 0x20, self.msg);
    }
}

/// `ccEquipment`: the row of each slot's table.
pub mod slot {
    pub const HEAD: usize = 0;
    pub const BODY: usize = 1;
    pub const ARM: usize = 2;
    pub const LEG: usize = 3;
    pub const WEAPON: usize = 4;
    pub const SHIELD: usize = 5;
}

/// `ccSpcParam` (0xdc bytes): a party member's record, one per character
/// in `saveData.spcParam[18]` (+0x7488), and what `charTbl` rows hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpcParam {
    pub base: Base,
    pub max_hp: i16,
    pub max_sp: i16,
    /// The character's own stats (level growth).
    pub elm: Elm,
    /// The effective stats `CalcReal` leaves.
    pub real: Elm,
    /// The equipment's sum.
    pub tune: Elm,
    /// Timed buffs and debuffs, with their frames left in `time`.
    pub temp: Elm,
    pub time: Elm,
    /// head body arm leg weapon shield.
    pub equipment: [i16; 6],
    /// Float bits.
    pub velocity: u32,
    pub job: i16,
    pub friendship: i16,
}

pub const SPC_PARAM_SIZE: usize = 0xdc;
/// `saveData.spcParam` (`ccSaveData` +0x7488).
pub const SAVE_SPC_PARAM: usize = 0x7488;

impl SpcParam {
    pub fn parse(b: &[u8]) -> SpcParam {
        SpcParam {
            base: Base::parse(b),
            max_hp: le16(b, 0x24),
            max_sp: le16(b, 0x26),
            elm: read_elm(b, 0x28),
            real: read_elm(b, 0x48),
            tune: read_elm(b, 0x68),
            temp: read_elm(b, 0x88),
            time: read_elm(b, 0xa8),
            equipment: std::array::from_fn(|i| le16(b, 0xc8 + 2 * i)),
            velocity: le32(b, 0xd4),
            job: le16(b, 0xd8),
            friendship: le16(b, 0xda),
        }
    }

    pub fn write(&self, b: &mut [u8]) {
        self.base.write(b);
        put16(b, 0x24, self.max_hp);
        put16(b, 0x26, self.max_sp);
        write_elm(b, 0x28, &self.elm);
        write_elm(b, 0x48, &self.real);
        write_elm(b, 0x68, &self.tune);
        write_elm(b, 0x88, &self.temp);
        write_elm(b, 0xa8, &self.time);
        for (i, v) in self.equipment.iter().enumerate() {
            put16(b, 0xc8 + 2 * i, *v);
        }
        put32(b, 0xd4, self.velocity);
        put16(b, 0xd8, self.job);
        put16(b, 0xda, self.friendship);
    }

    /// `saveData.spcParam[id]` (`ccGetCharParam(id)`; characters 18-20's
    /// in the save's extension).
    pub fn from_save(save: &piney_data::save::SaveData, id: usize) -> SpcParam {
        let at = piney_data::save::by_id::spc_param(id);
        SpcParam::parse(&save.record()[at..at + SPC_PARAM_SIZE])
    }

    pub fn store(&self, save: &mut piney_data::save::SaveData, id: usize) {
        let at = piney_data::save::by_id::spc_param(id);
        self.write(&mut save.record_mut()[at..at + SPC_PARAM_SIZE]);
    }
}

/// `ccSkillParam` (0x38 bytes): a row of `skillTbl` or `BossSkillTbl`,
/// and the four an enemy row carries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillParam {
    /// The name as the disc has it.
    pub name: Vec<u8>,
    /// The effect's file stem.
    pub filename: Vec<u8>,
    pub atk: i16,
    pub hit: i16,
    /// soil water fire wind thunder dark.
    pub attr: [i16; 6],
    pub condition: i32,
    /// Float bits: the distance the skill is used from, and its area's
    /// radius (0 for a single target).
    pub trigger_range: u32,
    pub target_range: u32,
    pub target_type: i32,
    /// SP.
    pub cost: i32,
    /// Type bits: see [`crate::skill::bits`].
    pub ty: i32,
    pub level: i16,
    pub dmg_rate: i16,
    /// The description's lines.
    pub str: Vec<Vec<u8>>,
}

pub const SKILL_PARAM_SIZE: u32 = 0x38;

/// A table's text, empty for none.
pub(crate) fn text(s: Option<&str>) -> Vec<u8> {
    s.map_or_else(Vec::new, encode)
}

fn attr(a: &gt::Attribute) -> [i16; 6] {
    [a.soil, a.water, a.fire, a.wind, a.thunder, a.dark]
}

/// `ccCharParamElement` as the sixteen stats in [`elm`]'s order.
pub fn elm_of(e: &gt::CharParamElement) -> Elm {
    let b = &e.battle_ability;
    let a = attr(&e.attribute);
    [
        b.p_atk,
        b.p_def,
        b.p_hit,
        b.p_eva,
        b.m_atk,
        b.m_def,
        b.m_hit,
        b.m_eva,
        a[0],
        a[1],
        a[2],
        a[3],
        a[4],
        a[5],
        e.tolerance.spirit,
        e.tolerance.body,
    ]
}

impl SkillParam {
    pub fn of(s: &gt::SkillParam) -> SkillParam {
        SkillParam {
            name: text(s.name),
            filename: text(s.filename),
            atk: s.atk,
            hit: s.hit,
            attr: attr(&s.attribute),
            condition: s.condition,
            trigger_range: s.trigger_range.to_bits(),
            target_range: s.target_range.to_bits(),
            target_type: s.target_type,
            cost: s.cost,
            ty: s.kind,
            level: s.level,
            dmg_rate: s.dmg_rate,
            str: s.str.unwrap_or_default().iter().map(|l| encode(l)).collect(),
        }
    }
}

/// `ccBattleEffect`: drainHP drainSP critical dying invincible.
pub type Beff = [i16; 5];

/// `ccEquipmentParam` (0x48 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipParam {
    pub name: Vec<u8>,
    pub ccsname: Vec<u8>,
    pub aura_sw: i16,
    pub elm: Elm,
    pub beff: Beff,
    pub code: i32,
    pub level: i16,
    pub skill_id: [i16; 3],
    pub price: i32,
    /// The description's lines.
    pub str: Vec<Vec<u8>>,
}

pub const EQUIP_PARAM_SIZE: u32 = 0x48;

impl EquipParam {
    pub fn of(e: &gt::EquipmentParam) -> EquipParam {
        let f = &e.beff;
        EquipParam {
            name: text(e.name),
            ccsname: text(e.ccsname),
            aura_sw: e.aura_sw,
            elm: elm_of(&e.elm),
            beff: [f.drain_hp, f.drain_sp, f.critical, f.dying, f.invincible],
            code: e.code,
            level: e.level,
            skill_id: e.skill_id,
            price: e.price,
            str: e.str.unwrap_or_default().iter().map(|l| encode(l)).collect(),
        }
    }
}

/// `ccItemParam` (0x14 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemParam {
    pub name: Vec<u8>,
    pub code: i32,
    /// The skill using the item casts.
    pub skill_id: i32,
    pub price: i32,
    /// The description's lines.
    pub str: Vec<Vec<u8>>,
}

pub const ITEM_PARAM_SIZE: u32 = 0x14;

impl ItemParam {
    pub fn of(i: &gt::ItemParam) -> ItemParam {
        ItemParam {
            name: text(i.name),
            code: i.code,
            skill_id: i.skill_id,
            price: i.price,
            str: i.str.unwrap_or_default().iter().map(|l| encode(l)).collect(),
        }
    }
}

/// `ccEnemyParamData` / `ccBossParamData` (0x68 bytes, one layout): an
/// enemy's or boss's fixed stats.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoeRow {
    pub base: Base,
    pub name: Vec<u8>,
    pub max_hp: i16,
    pub max_sp: i16,
    pub elm: Elm,
    pub beff: Beff,
    /// The protect gauge's size, and the defences it is filled against.
    pub max_pp: i16,
    pub p_def_pp: i16,
    pub m_def_pp: i16,
    /// Data Drain gives one of these item codes.
    pub item: [i32; 3],
    /// Immunities: bit 0x1 pDef, 0x2 mDef, 0x4-0x80 the elements.
    pub exdefense: i16,
}

pub const FOE_ROW_SIZE: u32 = 0x68;

/// A generated row's `ccCharBaseParam`: the name's text is its `label`
/// (the pointers are the running game's, 0 here).
pub(crate) fn base_of(b: &gt::CharBaseParam) -> Base {
    Base {
        name: 0,
        ccsname: 0,
        ty: b.kind,
        id: b.id,
        level: b.level,
        exp: b.exp,
        gold: b.gold,
        height: b.height.to_bits(),
        width: b.width.to_bits(),
        msg: b.msg,
        label: b.name,
    }
}

impl FoeRow {
    /// `ccBossParamData` and `ccEnemyParamData` share one layout.
    pub fn of(
        base: &gt::CharBaseParam,
        (max_hp, max_sp): (i16, i16),
        e: &gt::CharParamElement,
        f: &gt::BattleEffect,
        (max_pp, p_def_pp, m_def_pp): (i16, i16, i16),
        item: [i32; 3],
        exdefense: i16,
    ) -> FoeRow {
        FoeRow {
            base: base_of(base),
            name: text(base.name),
            max_hp,
            max_sp,
            elm: elm_of(e),
            beff: [f.drain_hp, f.drain_sp, f.critical, f.dying, f.invincible],
            max_pp,
            p_def_pp,
            m_def_pp,
            item,
            exdefense,
        }
    }

    pub fn of_boss(b: &gt::BossParamData) -> FoeRow {
        FoeRow::of(
            &b.base,
            (b.max_hp, b.max_sp),
            &b.elm,
            &b.beff,
            (b.max_pp, b.p_def_pp, b.m_def_pp),
            b.item,
            b.exdefense,
        )
    }

    pub fn of_enemy(b: &gt::EnemyParamData) -> FoeRow {
        FoeRow::of(
            &b.base,
            (b.max_hp, b.max_sp),
            &b.elm,
            &b.beff,
            (b.max_pp, b.p_def_pp, b.m_def_pp),
            b.item,
            b.exdefense,
        )
    }
}

/// `ccEnemyThinkParam` (0x28 bytes): ranges are float bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThinkParam {
    pub area: u32,
    pub territory: u32,
    pub view_range: u32,
    pub atk_range_a: u32,
    pub atk_range_b: u32,
    pub atk_range_c: u32,
    pub attack_delay: i32,
    pub max_spd: u32,
    pub anm_spd: u32,
    pub target_type: i32,
}

/// `ccEnemyTable` (0x1c0 bytes): a row of `enemyTbl`: the stats, how it
/// spawns (`ccEntry`: its `esize` and its six animation names), how it
/// thinks, and its skills: two attacks (`atc0`, `atc1`, with `mag0`,
/// `mag1`) and two skills (`ski0`, `ski1`) as whole `ccSkillParam`s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnemyTable {
    pub param: FoeRow,
    /// `entry.exist`, `entry.esize`.
    pub exist: i32,
    pub esize: i32,
    /// `entry.anm`: its animation names (attacks 0-5, wait 6, walk 7, the
    /// flinches, dying, ...). A middle boss's row has none on the disc; the
    /// area's loading gives it its base form's ([`crate::tables::Tables`]).
    pub anm: Option<&'static [&'static str]>,
    /// `entry.clut`: the CLUT its model takes (empty for its own), and
    /// `entry.fileList.name`: the model's file.
    pub clut: &'static str,
    pub file: &'static str,
    pub think: ThinkParam,
    pub atc: [SkillParam; 2],
    pub mag: [i32; 2],
    pub ski: [SkillParam; 2],
}

pub const ENEMY_TABLE_SIZE: u32 = 0x1c0;

impl EnemyTable {
    pub fn of(t: &gt::EnemyTable) -> EnemyTable {
        let k = &t.think;
        let s = &t.skill;
        EnemyTable {
            param: FoeRow::of_enemy(&t.param),
            exist: t.entry.exist,
            esize: t.entry.esize,
            anm: t.entry.anm,
            clut: t.entry.clut,
            file: t.entry.file_list.name.unwrap_or_default(),
            think: ThinkParam {
                area: k.area.to_bits(),
                territory: k.territory.to_bits(),
                view_range: k.view_range.to_bits(),
                atk_range_a: k.atk_range_a.to_bits(),
                atk_range_b: k.atk_range_b.to_bits(),
                atk_range_c: k.atk_range_c.to_bits(),
                attack_delay: k.attack_delay,
                max_spd: k.max_spd.to_bits(),
                anm_spd: k.anm_spd.to_bits(),
                target_type: k.target_type,
            },
            atc: [SkillParam::of(&s.atc0), SkillParam::of(&s.atc1)],
            mag: [s.mag0, s.mag1],
            ski: [SkillParam::of(&s.ski0), SkillParam::of(&s.ski1)],
        }
    }
}

/// `ccItemList` (4 bytes): an item stack in a list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemList {
    pub id: i16,
    pub category: i8,
    pub num: i8,
}

impl ItemList {
    pub fn parse(b: &[u8]) -> ItemList {
        ItemList { id: le16(b, 0), category: b[2] as i8, num: b[3] as i8 }
    }

    pub fn write(&self, b: &mut [u8]) {
        put16(b, 0, self.id);
        b[2] = self.category as u8;
        b[3] = self.num as u8;
    }

    /// The item code, `category << 16 | id`.
    pub fn code(&self) -> i32 {
        (i32::from(self.category) << 16) | i32::from(self.id as u16)
    }
}

/// `ccLevelUpTbl` (0x24 bytes): what one level adds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelUp {
    pub max_hp: i16,
    pub max_sp: i16,
    pub elm: Elm,
}

/// `ccAIParam` (0x24 bytes): a party member's AI settings
/// (`spcAIParam[18]`). Ranges are float bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AiParam {
    pub job: i32,
    pub caution_range: u32,
    pub territory: u32,
    pub arms_range: u32,
    pub attack_range: u32,
    pub stop_range: u32,
    pub no_turn_range: u32,
    /// Heal below this percentage of maxHP.
    pub heal_rate: i16,
    pub mercy: i16,
    /// SP kept back.
    pub saving_sp: i16,
    pub dummy: i16,
}

impl AiParam {
    pub fn parse(b: &[u8]) -> AiParam {
        AiParam {
            job: le32(b, 0) as i32,
            caution_range: le32(b, 4),
            territory: le32(b, 8),
            arms_range: le32(b, 0xc),
            attack_range: le32(b, 0x10),
            stop_range: le32(b, 0x14),
            no_turn_range: le32(b, 0x18),
            heal_rate: le16(b, 0x1c),
            mercy: le16(b, 0x1e),
            saving_sp: le16(b, 0x20),
            dummy: le16(b, 0x22),
        }
    }

    pub fn of(a: &gt::AIParam) -> AiParam {
        AiParam {
            job: a.job,
            caution_range: a.caution_range.to_bits(),
            territory: a.territory.to_bits(),
            arms_range: a.arms_range.to_bits(),
            attack_range: a.attack_range.to_bits(),
            stop_range: a.stop_range.to_bits(),
            no_turn_range: a.no_turn_range.to_bits(),
            heal_rate: a.heal_rate,
            mercy: a.mercy,
            saving_sp: a.saving_sp,
            dummy: a.dymmy_param3,
        }
    }
}

impl ItemList {
    pub fn of(l: &gt::ItemList) -> ItemList {
        ItemList { id: l.id, category: l.category, num: l.num }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spc_param_round_trips() {
        let mut b = [0u8; SPC_PARAM_SIZE];
        for (i, x) in b.iter_mut().enumerate() {
            *x = (i * 7 + 3) as u8;
        }
        let p = SpcParam::parse(&b);
        let mut c = b;
        p.write(&mut c);
        assert_eq!(b, c);
        assert_eq!(p.max_hp, le16(&b, 0x24));
        assert_eq!(p.equipment[4], le16(&b, 0xd0));
    }

    #[test]
    fn item_code() {
        let it = ItemList { id: 3, category: 12, num: 2 };
        assert_eq!(it.code(), 0xc0003);
    }
}
