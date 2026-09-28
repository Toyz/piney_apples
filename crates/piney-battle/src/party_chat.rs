//! The party members' chat lines: `ccAI::ChatMessage*` (`personal.cpp`,
//! gcmn 0x00586130-0x00592a98), the line each situation picks from the
//! tables in GCMN.PRG's data, the `#` codes `ChatMessageModify` fills in,
//! and `ChatMessageSender`, which opens the balloon ([`crate::party_ai`]'s
//! `Call::ChatMessageSender`).
//!
//! A line is not shown where it is picked. Each function writes its text
//! into the AI (`+0x24`, 80 bytes) and sets `chatRequest`; the member's
//! next `ChatMessageSender` (from `Brains`, once a frame) opens it over the
//! member with `ccChatMsg::OpenChat` and clears the request. A later line
//! the same frame overwrites an earlier one.
//!
//! ```text
//! every line   partyFlag (ccSpcChar +0xe0 bits 14-16, signed) 1, else
//!              nothing; the table's row MessageIndex(): the character id,
//!              18 for id 1 while saveData +0x220d is set (its lines
//!              garbled); nothing said under manual control
//!              (manualSW) or where the table has no line
//! a 19 table   one line a character
//! a 57 table   three a character: (rand() >> 3) % 100 below 60 the first,
//!              below 90 the second, else the third ("variant"), or
//!              (rand() >> 3) % 3 where noted
//! ChatMessageModify(line, a1, a2, a3)  0x00586190 into +0x24: #0 the
//!              leader's name (getPartyMenberChar(0)), #1 a1 or the
//!              leader's, #a a2 or " " (@3726), #b a3 or " "; stops once
//!              79 bytes are written (a name copied whole)
//! ```
//!
//! The chance a line has, and what else it does, are each function's own
//! ([`Ctx::chat_line`]); `docs/engine/battle.md` ("The chat lines") lists
//! them.

use std::collections::BTreeMap;

use piney_data::field::ee;
use piney_data::tables::sjis::encode;

use crate::chara::{Body, check_char_attribute};
use crate::party_ai::{Chat, Ctx};
use crate::scene::Scene;
use crate::tables::Tables;

/// The tables' names: Infection's addresses in GCMN.PRG, which the
/// tables are known by.
pub mod va {
    pub const STANDBY_ACCEPT: u32 = 0x0065_1ae0;
    pub const ACCEPT: u32 = 0x0065_1b30;
    pub const USE_ALL_ATTACK_SKILL_ACCEPT: u32 = 0x0065_1c20;
    pub const USE_PHYSICAL_SKILL_ACCEPT: u32 = 0x0065_1d10;
    pub const USE_MAGIC_SKILL_ACCEPT: u32 = 0x0065_1d60;
    pub const NOT_USE_SKILL_ACCEPT: u32 = 0x0065_1db0;
    pub const BUFF_ACCEPT: u32 = 0x0065_1e00;
    pub const DEBUFF_ACCEPT: u32 = 0x0065_1e50;
    pub const HEAL_START: u32 = 0x0065_1ea0;
    pub const CURE_START: u32 = 0x0065_1ef0;
    pub const RESURRECT_START: u32 = 0x0065_1f40;
    pub const SHORT_OF_SP: u32 = 0x0065_1f90;
    pub const NOTHING_DENY: u32 = 0x0065_1fe0;
    pub const NO_BATTLE_MODE_DENY: u32 = 0x0065_2030;
    pub const GHOST_DENY: u32 = 0x0065_2080;
    pub const CONDITION_RED: u32 = 0x0065_20d0;
    pub const THX_HEAL: u32 = 0x0065_2120;
    pub const THX_RESURRECT: u32 = 0x0065_2170;
    pub const THX_BUFF: u32 = 0x0065_21c0;
    pub const USE_OCARINA: u32 = 0x0065_2210;
    pub const NO_OCARINA_DENY: u32 = 0x0065_2260;
    pub const DISABLE_OCARINA_DENY: u32 = 0x0065_22b0;
    pub const ENTERED_FIELD: u32 = 0x0065_2300;
    pub const VICTORY: u32 = 0x0065_23f0;
    pub const NEUTRAL: u32 = 0x0065_24e0;
    pub const HEAL_PLZ: u32 = 0x0065_25d0;
    pub const TREATMENT_PLZ: u32 = 0x0065_2620;
    pub const RESURRECT_PLZ: u32 = 0x0065_2670;
    pub const BULL: u32 = 0x0065_26c0;
    pub const DASH_FOR_ENEMY: u32 = 0x0065_27b0;
    pub const TIMID: u32 = 0x0065_28a0;
    pub const SMALL_DAMAGED: u32 = 0x0065_2990;
    pub const DAMAGED: u32 = 0x0065_2a80;
    pub const FATAL_DAMAGED: u32 = 0x0065_2b70;
    pub const ATTACK_SMALL_HIT: u32 = 0x0065_2c60;
    pub const ATTACK_HIT: u32 = 0x0065_2d50;
    pub const ATTACK_FATAL_HIT: u32 = 0x0065_2e40;
    pub const REENCOUNTER: u32 = 0x0065_2f30;
    pub const GOTO_WEAPON_SHOP: u32 = 0x0065_3020;
    pub const GOTO_GOODS_SHOP: u32 = 0x0065_3070;
    pub const GOTO_FAIRY_SHOP: u32 = 0x0065_30c0;
    pub const GOTO_MAGIC_SHOP: u32 = 0x0065_3110;
    pub const GOTO_ETC: u32 = 0x0065_3160;
    pub const BYEBYE: u32 = 0x0065_31b0;
    pub const ARRIVE_DELTA: u32 = 0x0065_3200;
    pub const ARRIVE_LAMBDA: u32 = 0x0065_3250;
    pub const ARRIVE_SIGMA: u32 = 0x0065_32a0;
    pub const ARRIVE_OMEGA: u32 = 0x0065_32f0;
    pub const ARRIVE_THETA: u32 = 0x0065_3340;
    pub const GOTO_RECORD_SHOP: u32 = 0x0065_3390;
    pub const CHANGE_EQUIP_OK: u32 = 0x0065_33e0;
    pub const CHANGE_EQUIP_NOT_SKILL: u32 = 0x0065_3430;
    pub const CHANGE_EQUIP_NOT_ETC: u32 = 0x0065_3480;
    pub const ATTRIBUTE_FOLLOW: u32 = 0x0065_34d0;
    pub const ATTRIBUTE_GUARD: u32 = 0x0065_3520;
    pub const ATTRIBUTE_CRITICAL: u32 = 0x0065_3570;
    pub const HEAL_PLZ_ACCEPT: u32 = 0x0065_35c0;
    pub const SKILL_ACCEPT: u32 = 0x0065_3610;
    pub const ASSIGN_ACCEPT: u32 = 0x0065_3660;
    pub const POISON_CURSE: u32 = 0x0065_36b0;
    pub const PARALYSIS_SLEEP: u32 = 0x0065_3700;
    pub const CHARM_CONFUSION: u32 = 0x0065_3750;
    pub const WAIT_PLZ: u32 = 0x0065_37a0;
    pub const GHOST_CONDITION: u32 = 0x0065_37f0;
    pub const LEVEL_DOWN: u32 = 0x0065_3840;
    pub const LEVEL_UP: u32 = 0x0065_3890;
    pub const PRESENT_OTHER_FELLOW: u32 = 0x0065_38e0;
    pub const DEAD_OTHER_FELLOW: u32 = 0x0065_3930;
    pub const OPEN_TRAP_BOX: u32 = 0x0065_3980;
    pub const QUIT_PUCCIGUSO: u32 = 0x0065_39d0;
    pub const GRATS_LEVEL_UP: u32 = 0x0065_3a20;
    pub const CONDITION_MODIFY_ENEMY: u32 = 0x0065_3a70;
    pub const OPEN_TREASURE_BOX: u32 = 0x0065_3ac0;
    pub const A_LOT_OF_ENEMY: u32 = 0x0065_3b10;
    pub const BETTER_WEAPON: u32 = 0x0065_3b60;
    pub const WORSE_WEAPON: u32 = 0x0065_3b70;
    pub const FELLOW_IS_HIGH_LEVEL: u32 = 0x0065_3b80;
    pub const WALKING_TALK: u32 = 0x0065_3b90;
}

/// `@3726`: what `#a` and `#b` become with nothing given.
pub const NONE: &[u8] = b" ";

/// Every line table with its entries: one a character (19) or three
/// (57), and the three weapon and level lines.
pub const TABLES: [(u32, usize); 77] = [
    (va::STANDBY_ACCEPT, 19),
    (va::ACCEPT, 57),
    (va::USE_ALL_ATTACK_SKILL_ACCEPT, 57),
    (va::USE_PHYSICAL_SKILL_ACCEPT, 19),
    (va::USE_MAGIC_SKILL_ACCEPT, 19),
    (va::NOT_USE_SKILL_ACCEPT, 19),
    (va::BUFF_ACCEPT, 19),
    (va::DEBUFF_ACCEPT, 19),
    (va::HEAL_START, 19),
    (va::CURE_START, 19),
    (va::RESURRECT_START, 19),
    (va::SHORT_OF_SP, 19),
    (va::NOTHING_DENY, 19),
    (va::NO_BATTLE_MODE_DENY, 19),
    (va::GHOST_DENY, 19),
    (va::CONDITION_RED, 19),
    (va::THX_HEAL, 19),
    (va::THX_RESURRECT, 19),
    (va::THX_BUFF, 19),
    (va::USE_OCARINA, 19),
    (va::NO_OCARINA_DENY, 19),
    (va::DISABLE_OCARINA_DENY, 19),
    (va::ENTERED_FIELD, 57),
    (va::VICTORY, 57),
    (va::NEUTRAL, 57),
    (va::HEAL_PLZ, 19),
    (va::TREATMENT_PLZ, 19),
    (va::RESURRECT_PLZ, 19),
    (va::BULL, 57),
    (va::DASH_FOR_ENEMY, 57),
    (va::TIMID, 57),
    (va::SMALL_DAMAGED, 57),
    (va::DAMAGED, 57),
    (va::FATAL_DAMAGED, 57),
    (va::ATTACK_SMALL_HIT, 57),
    (va::ATTACK_HIT, 57),
    (va::ATTACK_FATAL_HIT, 57),
    (va::REENCOUNTER, 57),
    (va::GOTO_WEAPON_SHOP, 19),
    (va::GOTO_GOODS_SHOP, 19),
    (va::GOTO_FAIRY_SHOP, 19),
    (va::GOTO_MAGIC_SHOP, 19),
    (va::GOTO_ETC, 19),
    (va::BYEBYE, 19),
    (va::ARRIVE_DELTA, 19),
    (va::ARRIVE_LAMBDA, 19),
    (va::ARRIVE_SIGMA, 19),
    (va::ARRIVE_OMEGA, 19),
    (va::ARRIVE_THETA, 19),
    (va::GOTO_RECORD_SHOP, 19),
    (va::CHANGE_EQUIP_OK, 19),
    (va::CHANGE_EQUIP_NOT_SKILL, 19),
    (va::CHANGE_EQUIP_NOT_ETC, 19),
    (va::ATTRIBUTE_FOLLOW, 19),
    (va::ATTRIBUTE_GUARD, 19),
    (va::ATTRIBUTE_CRITICAL, 19),
    (va::HEAL_PLZ_ACCEPT, 19),
    (va::SKILL_ACCEPT, 19),
    (va::ASSIGN_ACCEPT, 19),
    (va::POISON_CURSE, 19),
    (va::PARALYSIS_SLEEP, 19),
    (va::CHARM_CONFUSION, 19),
    (va::WAIT_PLZ, 19),
    (va::GHOST_CONDITION, 19),
    (va::LEVEL_DOWN, 19),
    (va::LEVEL_UP, 19),
    (va::PRESENT_OTHER_FELLOW, 19),
    (va::DEAD_OTHER_FELLOW, 19),
    (va::OPEN_TRAP_BOX, 19),
    (va::QUIT_PUCCIGUSO, 19),
    (va::GRATS_LEVEL_UP, 19),
    (va::CONDITION_MODIFY_ENEMY, 19),
    (va::OPEN_TREASURE_BOX, 19),
    (va::A_LOT_OF_ENEMY, 19),
    (va::BETTER_WEAPON, 3),
    (va::WORSE_WEAPON, 3),
    (va::FELLOW_IS_HIGH_LEVEL, 3),
];

/// The remark tables Mutation adds (no Infection address to know them
/// by), one line a character; each said by a function of its own that
/// `ChatMessageWalkingTalk`'s neighbours hold (gcmn 0x005bb760 on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remark {
    /// From `ChatMessageAttack`: the foe is immune to physical attacks,
    /// or to magic.
    PhysicalTolerance,
    MagicTolerance,
    /// From `ccAI::UseItem`: an item used, or the last of it (`#a` its
    /// name).
    UseItem,
    UseLastItem,
    /// From `ChatCommandBuffPlz` and `ChatCommandDeBuffPlz`: the only one
    /// the member has.
    OnlyBuff,
    OnlyDebuff,
}

/// The chat lines' texts from GCMN.PRG.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatTexts {
    /// Each table by address: its lines, None for a null entry.
    pub lines: BTreeMap<u32, Vec<Option<Vec<u8>>>>,
    /// The [`Remark`] tables, in its order (empty on Infection).
    pub remarks: [Vec<Option<Vec<u8>>>; 6],
    /// `attributeStrTbl` and `attributeStrTbl2`.
    pub attribute: [Vec<Vec<u8>>; 2],
    /// `@3726`.
    pub none: Vec<u8>,
}

impl ChatTexts {
    /// The volume's (`piney_data::tables::party_chat`), each table under
    /// its [`va`] name.
    pub fn of(volume: piney_data::volume::Volume) -> ChatTexts {
        let t = piney_data::tables::party_chat::of(volume);
        let conv = |l: &[Option<&str>]| l.iter().map(|x| x.map(encode)).collect::<Vec<_>>();
        let mut lines = BTreeMap::new();
        lines.insert(va::STANDBY_ACCEPT, conv(t.standby_accept()));
        lines.insert(va::ACCEPT, conv(t.accept()));
        lines.insert(va::USE_ALL_ATTACK_SKILL_ACCEPT, conv(t.use_all_attack_skill_accept()));
        lines.insert(va::USE_PHYSICAL_SKILL_ACCEPT, conv(t.use_physical_skill_accept()));
        lines.insert(va::USE_MAGIC_SKILL_ACCEPT, conv(t.use_magic_skill_accept()));
        lines.insert(va::NOT_USE_SKILL_ACCEPT, conv(t.not_use_skill_accept()));
        lines.insert(va::BUFF_ACCEPT, conv(t.buff_accept()));
        lines.insert(va::DEBUFF_ACCEPT, conv(t.debuff_accept()));
        lines.insert(va::HEAL_START, conv(t.heal_start()));
        lines.insert(va::CURE_START, conv(t.cure_start()));
        lines.insert(va::RESURRECT_START, conv(t.resurrect_start()));
        lines.insert(va::SHORT_OF_SP, conv(t.short_of_sp()));
        lines.insert(va::NOTHING_DENY, conv(t.nothing_deny()));
        lines.insert(va::NO_BATTLE_MODE_DENY, conv(t.no_battle_mode_deny()));
        lines.insert(va::GHOST_DENY, conv(t.ghost_deny()));
        lines.insert(va::CONDITION_RED, conv(t.condition_red()));
        lines.insert(va::THX_HEAL, conv(t.thx_heal()));
        lines.insert(va::THX_RESURRECT, conv(t.thx_resurrect()));
        lines.insert(va::THX_BUFF, conv(t.thx_buff()));
        lines.insert(va::USE_OCARINA, conv(t.use_ocarina()));
        lines.insert(va::NO_OCARINA_DENY, conv(t.no_ocarina_deny()));
        lines.insert(va::DISABLE_OCARINA_DENY, conv(t.disable_ocarina_deny()));
        lines.insert(va::ENTERED_FIELD, conv(t.entered_field()));
        lines.insert(va::VICTORY, conv(t.victory()));
        lines.insert(va::NEUTRAL, conv(t.neutral()));
        lines.insert(va::HEAL_PLZ, conv(t.heal_plz()));
        lines.insert(va::TREATMENT_PLZ, conv(t.treatment_plz()));
        lines.insert(va::RESURRECT_PLZ, conv(t.resurrect_plz()));
        lines.insert(va::BULL, conv(t.bull()));
        lines.insert(va::DASH_FOR_ENEMY, conv(t.dash_for_enemy()));
        lines.insert(va::TIMID, conv(t.timid()));
        lines.insert(va::SMALL_DAMAGED, conv(t.small_damaged()));
        lines.insert(va::DAMAGED, conv(t.damaged()));
        lines.insert(va::FATAL_DAMAGED, conv(t.fatal_damaged()));
        lines.insert(va::ATTACK_SMALL_HIT, conv(t.attack_small_hit()));
        lines.insert(va::ATTACK_HIT, conv(t.attack_hit()));
        lines.insert(va::ATTACK_FATAL_HIT, conv(t.attack_fatal_hit()));
        lines.insert(va::REENCOUNTER, conv(t.reencounter()));
        lines.insert(va::GOTO_WEAPON_SHOP, conv(t.goto_weapon_shop()));
        lines.insert(va::GOTO_GOODS_SHOP, conv(t.goto_goods_shop()));
        lines.insert(va::GOTO_FAIRY_SHOP, conv(t.goto_fairy_shop()));
        lines.insert(va::GOTO_MAGIC_SHOP, conv(t.goto_magic_shop()));
        lines.insert(va::GOTO_ETC, conv(t.goto_etc()));
        lines.insert(va::BYEBYE, conv(t.byebye()));
        lines.insert(va::ARRIVE_DELTA, conv(t.arrive_delta()));
        lines.insert(va::ARRIVE_LAMBDA, conv(t.arrive_lambda()));
        lines.insert(va::ARRIVE_SIGMA, conv(t.arrive_sigma()));
        lines.insert(va::ARRIVE_OMEGA, conv(t.arrive_omega()));
        lines.insert(va::ARRIVE_THETA, conv(t.arrive_theta()));
        lines.insert(va::GOTO_RECORD_SHOP, conv(t.goto_record_shop()));
        lines.insert(va::CHANGE_EQUIP_OK, conv(t.change_equip_ok()));
        lines.insert(va::CHANGE_EQUIP_NOT_SKILL, conv(t.change_equip_not_skill()));
        lines.insert(va::CHANGE_EQUIP_NOT_ETC, conv(t.change_equip_not_etc()));
        lines.insert(va::ATTRIBUTE_FOLLOW, conv(t.attribute_follow()));
        lines.insert(va::ATTRIBUTE_GUARD, conv(t.attribute_guard()));
        lines.insert(va::ATTRIBUTE_CRITICAL, conv(t.attribute_critical()));
        lines.insert(va::HEAL_PLZ_ACCEPT, conv(t.heal_plz_accept()));
        lines.insert(va::SKILL_ACCEPT, conv(t.skill_accept()));
        lines.insert(va::ASSIGN_ACCEPT, conv(t.assign_accept()));
        lines.insert(va::POISON_CURSE, conv(t.poison_curse()));
        lines.insert(va::PARALYSIS_SLEEP, conv(t.paralysis_sleep()));
        lines.insert(va::CHARM_CONFUSION, conv(t.charm_confusion()));
        lines.insert(va::WAIT_PLZ, conv(t.wait_plz()));
        lines.insert(va::GHOST_CONDITION, conv(t.ghost_condition()));
        lines.insert(va::LEVEL_DOWN, conv(t.level_down()));
        lines.insert(va::LEVEL_UP, conv(t.level_up()));
        lines.insert(va::PRESENT_OTHER_FELLOW, conv(t.present_other_fellow()));
        lines.insert(va::DEAD_OTHER_FELLOW, conv(t.dead_other_fellow()));
        lines.insert(va::OPEN_TRAP_BOX, conv(t.open_trap_box()));
        lines.insert(va::QUIT_PUCCIGUSO, conv(t.quit_pucciguso()));
        lines.insert(va::GRATS_LEVEL_UP, conv(t.grats_level_up()));
        lines.insert(va::CONDITION_MODIFY_ENEMY, conv(t.condition_modify_enemy()));
        lines.insert(va::OPEN_TREASURE_BOX, conv(t.open_treasure_box()));
        lines.insert(va::A_LOT_OF_ENEMY, conv(t.a_lot_of_enemy()));
        lines.insert(va::BETTER_WEAPON, conv(t.better_weapon()));
        lines.insert(va::WORSE_WEAPON, conv(t.worse_weapon()));
        lines.insert(va::FELLOW_IS_HIGH_LEVEL, conv(t.fellow_is_high_level()));
        lines.insert(va::WALKING_TALK, conv(t.walking_talk()));
        let names = |l: &[&str]| l.iter().map(|x| encode(x)).collect::<Vec<_>>();
        let remarks = [
            conv(t.physical_tolerance()),
            conv(t.magic_tolerance()),
            conv(t.use_item()),
            conv(t.use_last_item()),
            conv(t.only_buff()),
            conv(t.only_debuff()),
        ];
        ChatTexts {
            lines,
            remarks,
            attribute: [names(t.attribute_str()), names(t.attribute_str2())],
            none: NONE.to_vec(),
        }
    }

    /// Entry `i` of remark table `r`.
    pub fn remark(&self, r: Remark, i: i32) -> Option<&[u8]> {
        self.remarks[r as usize].get(usize::try_from(i).ok()?)?.as_deref()
    }

    /// Entry `i` of table `at`.
    pub fn line(&self, at: u32, i: i32) -> Option<&[u8]> {
        let t = self.lines.get(&at)?;
        t.get(usize::try_from(i).ok()?)?.as_deref()
    }

    /// `getAttributeStr(attr, n)` (gcmn 0x00597760): below 2 entry 0 ("!"),
    /// 8 and over entry 1 ("?"), else the element's name; `n` non-zero the
    /// second table.
    pub fn attribute(&self, attr: i32, n: i32) -> Vec<u8> {
        let i = if attr < 2 {
            0
        } else if attr >= 8 {
            1
        } else {
            attr as usize
        };
        self.attribute[usize::from(n != 0)].get(i).cloned().unwrap_or_default()
    }
}

/// `ChatMessageModify`'s longest write before it stops.
const MODIFY_MAX: usize = 79;

/// `ccAI::ChatMessageModify(line, a1, a2, a3)` (gcmn 0x00586190): the
/// text the AI keeps, `leader` the name `#0` (and `#1` without `a1`)
/// gives, `none` what `#a` and `#b` give without theirs. A `#` with any
/// other code would never end (the game loops on it); the port stops.
pub fn modify(
    line: &[u8],
    leader: &[u8],
    a1: Option<&[u8]>,
    a2: Option<&[u8]>,
    a3: Option<&[u8]>,
    none: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while out.len() < MODIFY_MAX {
        let Some(&c) = line.get(i).filter(|&&c| c != 0) else { break };
        if c != b'#' {
            out.push(c);
            i += 1;
            continue;
        }
        let with = match line.get(i + 1) {
            Some(b'0') => leader,
            Some(b'1') => a1.unwrap_or(leader),
            Some(b'a') => a2.unwrap_or(none),
            Some(b'b') => a3.unwrap_or(none),
            _ => break,
        };
        out.extend(with.iter().copied().take_while(|&b| b != 0));
        i += 2;
    }
    out
}

/// The name a line gives a character (`ccChar.base->name`): a party
/// member's is the save's `plName` for Kite (id 0) and `charTbl`'s for the
/// others; an enemy's its row's.
pub fn char_name(t: &Tables, save: &piney_data::save::SaveData, scene: &Scene, ch: usize) -> Vec<u8> {
    let Some(c) = scene.chars.get(ch) else { return Vec::new() };
    match &c.body {
        Body::Spc(_) => {
            let id = c.id();
            if id == 0 {
                save.name().to_vec()
            } else {
                usize::try_from(id).ok().and_then(|i| t.chars.get(i)).map(|r| r.name.clone()).unwrap_or_default()
            }
        }
        _ => Vec::new(),
    }
}

/// `ccPartyManager.memberChar[k]`.
fn member(p: &crate::exp::Party, k: i32) -> Option<usize> {
    usize::try_from(k).ok().and_then(|k| p.members.get(k).copied().flatten())
}

/// A line's variant from `(rand() >> 3) % 100`: below 60 the first, below
/// 90 the second, else the third.
fn variant(r: i32) -> i32 {
    let v = (r >> 3) % 100;
    if v < 60 {
        0
    } else if v < 90 {
        1
    } else {
        2
    }
}

impl Ctx<'_> {
    fn chat_body(&self, me: usize) -> usize {
        self.crew.ais.get(&me).map_or(me, |a| a.body)
    }

    /// `partyFlag` 1: the member is in the party.
    fn chat_gate(&self, me: usize) -> bool {
        let b = self.chat_body(me);
        crate::ai_move::party_flag(self.scene.chars[b].party_flag) == 1
    }

    /// `ch->base->name`. The game reads it without a check: for no
    /// character it reads through address 0, where eemu (as the game's
    /// start of memory) holds zeros, so the pointer is null and `#1`
    /// gives the leader's name, `#0` nothing.
    fn chat_name(&mut self, ch: Option<usize>) -> Option<Vec<u8>> {
        ch.map(|c| self.rt.char_name(self.t, self.save, self.scene, c))
    }

    /// `(rand() >> 3)`.
    fn chat_draw(&mut self) -> i32 {
        self.rng.rand() >> 3
    }

    /// Table `at`'s line for the member: entry `MessageIndex()`, or with
    /// three a character entry `3 MessageIndex() + v`.
    fn chat_row(&self, me: usize, at: u32, v: Option<i32>) -> Option<Vec<u8>> {
        let i = self.message_index(me);
        let i = match v {
            Some(v) => 3 * i + v,
            None => i,
        };
        self.t.chat.line(at, i).map(<[u8]>::to_vec)
    }

    /// The end every line shares: unless under manual control and with a
    /// line, `ChatMessageModify(line, a1, a2, a3)` and `chatRequest` 1.
    fn chat_say(
        &mut self,
        me: usize,
        line: Option<Vec<u8>>,
        a1: Option<Vec<u8>>,
        a2: Option<Vec<u8>>,
        a3: Option<Vec<u8>>,
    ) {
        let Some(line) = line else { return };
        if self.crew.ais.get(&me).is_none_or(|a| a.manual_sw) {
            return;
        }
        let leader_ch = member(self.party, 0);
        let leader = self.chat_name(leader_ch).unwrap_or_default();
        let none = self.t.chat.none.clone();
        let text = modify(&line, &leader, a1.as_deref(), a2.as_deref(), a3.as_deref(), &none);
        if let Some(a) = self.crew.ais.get_mut(&me) {
            a.chat_text = text;
            a.chat_request = true;
        }
    }

    /// A line from a table of one a character, said.
    fn chat_say_row(&mut self, me: usize, at: u32) {
        let line = self.chat_row(me, at, None);
        self.chat_say(me, line, None, None, None);
    }

    /// The turn a line ends with where the member can turn
    /// (`CheckAction(1)`): to face `to`, else its AI's target; with
    /// neither it keeps its heading. `atan2f(d.x, -d.y)` of the target's
    /// `posP` less its own.
    fn chat_face(&mut self, me: usize, to: Option<usize>) {
        let body = self.chat_body(me);
        if !self.can_act(body, 1) {
            return;
        }
        let to = to.or_else(|| self.crew.ais.get(&me).and_then(|a| a.target));
        let Some(t) = to else { return };
        let (a, b) = (self.scene.chars[t].pos_p, self.scene.chars[body].pos_p);
        let dx = ee::sub(a[0], b[0]);
        let dy = ee::sub(a[1], b[1]);
        let h = piney_data::libm::atan2f(dx, piney_data::libm::neg(dy));
        self.crew.spc.entry(body).or_default().dirc[2] = h;
    }

    /// The `ccAISystem` "heal me" request (0x10016) a hurt member files
    /// with itself, as `ChatMessageDamage` and `ChatMessageNeutral` build
    /// it: nothing while a 0x1xxxx message to it waits; else from and to
    /// itself, parameter -1, pointing at its body, in `11 ((rand() >> 3) &
    /// 3)` frames.
    fn chat_heal_me(&mut self, me: usize) {
        let id = self.crew.ais.get(&me).map_or(0, |a| a.sys_msg.id) as u16;
        let waiting = self
            .crew
            .sys
            .msg_buff
            .iter()
            .any(|m| m.state != 0 && (m.name as u32) & 0xffff_0000 == 0x1_0000 && m.receiver == id);
        if waiting {
            return;
        }
        let body = self.chat_body(me);
        let k = (self.chat_draw() & 3) as u16;
        self.crew.send(0x10016, id, id, 0, 11 * k, -1, Some(body));
    }

    fn chat_drop_heal_me(&mut self, me: usize) {
        let id = self.crew.ais.get(&me).map_or(0, |a| a.sys_msg.id) as u16;
        self.crew.sys.delete_delay(0x10016, id, id);
    }

    /// A `ccAI::ChatMessage*` of `me`'s AI: the line picked and kept for
    /// the next `ChatMessageSender`, with what else the function does.
    /// Returns what the function returns (`ChatMessageAttackTarget`'s
    /// kind of line; 0 for the others).
    pub fn chat_line(&mut self, me: usize, msg: Chat) -> i32 {
        use va::*;
        match msg {
            Chat::Line(table, i) => {
                // ccAI::ChatMessage(table[index], 0, 0, 0): no party check.
                let line = self.t.chat.line(table.table(), i).map(<[u8]>::to_vec);
                self.chat_say(me, line, None, None, None);
            }
            Chat::ConditionMinus => self.chat_template(me, CONDITION_RED),
            Chat::CanNot => self.chat_template(me, NOTHING_DENY),
            Chat::Oom => self.chat_template(me, SHORT_OF_SP),
            Chat::NoBattleModeDeny => self.chat_template(me, NO_BATTLE_MODE_DENY),
            Chat::DisableOcarinaDeny => self.chat_template(me, DISABLE_OCARINA_DENY),
            Chat::NoOcarinaDeny => self.chat_template(me, NO_OCARINA_DENY),
            Chat::UseOcarina => self.chat_template(me, USE_OCARINA),
            Chat::LevelDown => self.chat_template(me, LEVEL_DOWN),
            Chat::LevelUp => self.chat_template(me, LEVEL_UP),
            Chat::GhostCondition => self.chat_template(me, GHOST_CONDITION),
            Chat::TreatmentPlz => self.chat_template(me, TREATMENT_PLZ),
            Chat::HealPlz => self.chat_template(me, HEAL_PLZ),
            Chat::EquipOk => self.chat_template(me, CHANGE_EQUIP_OK),
            Chat::AttributeCritical => self.chat_template(me, ATTRIBUTE_CRITICAL),
            Chat::Ghost => self.chat_chance(me, 7, GHOST_DENY),
            Chat::HealStart => self.chat_chance(me, 7, HEAL_START),
            Chat::CureStart => self.chat_chance(me, 7, CURE_START),
            Chat::ResurrectStart => self.chat_chance(me, 7, RESURRECT_START),
            Chat::WaitPlz => self.chat_chance(me, 1, WAIT_PLZ),
            Chat::QuitPucciguso => self.chat_chance(me, 1, QUIT_PUCCIGUSO),
            Chat::Accept => self.chat_chance_variant(me, true, ACCEPT),
            Chat::EnteredField => self.chat_chance_variant(me, true, ENTERED_FIELD),
            Chat::Reencounter => self.chat_chance_variant(me, false, REENCOUNTER),
            Chat::Victory => {
                // (rand() >> 3) & 3: a quarter of the time.
                if self.chat_gate(me) && self.chat_draw() & 3 == 0 {
                    let v = variant(self.rng.rand());
                    let line = self.chat_row(me, VICTORY, Some(v));
                    self.chat_say(me, line, None, None, None);
                }
            }
            Chat::EquipNot(v) => {
                if self.chat_gate(me) {
                    self.chat_say_row(me, if v == 1 { CHANGE_EQUIP_NOT_SKILL } else { CHANGE_EQUIP_NOT_ETC });
                }
            }
            Chat::EnteredTown => {
                if self.chat_gate(me) {
                    let at = match self.game.server {
                        0 => ARRIVE_DELTA,
                        1 => ARRIVE_THETA,
                        2 => ARRIVE_LAMBDA,
                        3 => ARRIVE_SIGMA,
                        4 => ARRIVE_OMEGA,
                        _ => return 0,
                    };
                    self.chat_say_row(me, at);
                }
            }
            Chat::DeadOtherFellow(ch) => {
                if self.chat_gate(me) {
                    let name = self.chat_name(ch);
                    let line = self.chat_row(me, DEAD_OTHER_FELLOW, None);
                    self.chat_say(me, line, name, None, None);
                }
            }
            Chat::GratsLevelUp(ch) => {
                if self.chat_gate(me) {
                    let name = self.chat_name(ch);
                    let line = self.chat_row(me, GRATS_LEVEL_UP, None);
                    self.chat_say(me, line, name, None, None);
                    self.chat_face(me, ch);
                }
            }
            Chat::OpenTrapBox => {
                if self.chat_gate(me) {
                    self.chat_say_row(me, OPEN_TRAP_BOX);
                    let leader = member(self.party, 0);
                    self.chat_face(me, leader);
                }
            }
            Chat::OpenTreasureBox => {
                if self.chat_gate(me) && self.chat_draw() % 3 == 0 {
                    self.chat_say_row(me, OPEN_TREASURE_BOX);
                    let leader = member(self.party, 0);
                    self.chat_face(me, leader);
                }
            }
            Chat::PresentOtherFellow(ch) => {
                let listed = ch.is_some_and(|c| self.scene.listed(c));
                if listed && self.chat_gate(me) && self.chat_draw() & 1 != 0 {
                    let name = self.chat_name(ch);
                    let line = self.chat_row(me, PRESENT_OTHER_FELLOW, None);
                    self.chat_say(me, line, name, None, None);
                    let leader = member(self.party, 0);
                    self.chat_face(me, leader);
                }
            }
            Chat::WalkingTalk => {
                if self.chat_gate(me) {
                    let who = self.chat_pick_other(me);
                    let name = self.chat_name(who);
                    let i = self.message_index(me);
                    let v = self.chat_draw() % 3;
                    let line = self.t.chat.line(WALKING_TALK, 3 * i + v).map(<[u8]>::to_vec);
                    self.chat_say(me, line, name, None, None);
                }
            }
            Chat::ResurrectPlz => {
                if self.chat_gate(me) {
                    self.chat_say_row(me, RESURRECT_PLZ);
                }
                self.chat_drop_heal_me(me);
            }
            Chat::Affect(kind, from, n, hp) => match kind {
                18 => return self.chat_line(me, Chat::ConditionModify(n)),
                17 => self.chat_thanks(me, THX_BUFF, from),
                20 => self.chat_thanks(me, THX_RESURRECT, from),
                16 | 8 | 7 => self.chat_thanks_heal(me, from, n, hp),
                _ => {}
            },
            Chat::ConditionModify(sid) => self.chat_condition_modify(me, sid),
            Chat::ChatCmdAccept(cmd) => self.chat_cmd_accept(me, cmd),
            Chat::AttackTarget(tp) => return self.chat_attack_target(me, tp),
            Chat::Attack(tp, dmg, sid) => self.chat_attack(me, tp, dmg, sid),
            Chat::Remark(r, code) => self.chat_remark(me, r, code),
            Chat::Damage(dmg, hp) => self.chat_damage(me, dmg, hp),
            Chat::Neutral => self.chat_neutral(me),
            Chat::AttributeGuard(attr) => {
                if self.chat_gate(me) {
                    let a = self.t.chat.attribute(attr, 0);
                    let line = self.chat_row(me, ATTRIBUTE_GUARD, None);
                    self.chat_say(me, line, None, Some(a), None);
                }
            }
            Chat::AttributeFollow(attr) => {
                if self.chat_gate(me) {
                    let b = self.t.chat.attribute(attr, 1);
                    let a = self.t.chat.attribute(attr, 0);
                    let line = self.chat_row(me, ATTRIBUTE_FOLLOW, None);
                    self.chat_say(me, line, None, Some(a), Some(b));
                }
            }
        }
        0
    }

    /// The plain line: in the party, its row.
    /// A [`Remark`] (gcmn 0x005bb760 on, from Mutation on): in the party,
    /// the member's row, `#a` the name of item `code` for an item's.
    fn chat_remark(&mut self, me: usize, r: Remark, code: i32) {
        if !self.chat_gate(me) {
            return;
        }
        let line = self.t.chat.remark(r, self.message_index(me)).map(<[u8]>::to_vec);
        let item = matches!(r, Remark::UseItem | Remark::UseLastItem)
            .then(|| self.t.item_name(code).unwrap_or_default().to_vec());
        self.chat_say(me, line, None, item, None);
    }

    fn chat_template(&mut self, me: usize, at: u32) {
        if self.chat_gate(me) {
            self.chat_say_row(me, at);
        }
    }

    /// In the party, `(rand() >> 3) & mask` non-zero, its row.
    fn chat_chance(&mut self, me: usize, mask: i32, at: u32) {
        if self.chat_gate(me) && self.chat_draw() & mask != 0 {
            self.chat_say_row(me, at);
        }
    }

    /// In the party, (with `chance`) `(rand() >> 3) & 7` non-zero, a
    /// variant of the three.
    fn chat_chance_variant(&mut self, me: usize, chance: bool, at: u32) {
        if !self.chat_gate(me) || (chance && self.chat_draw() & 7 == 0) {
            return;
        }
        let v = variant(self.rng.rand());
        let line = self.chat_row(me, at, Some(v));
        self.chat_say(me, line, None, None, None);
    }

    /// `ChatMessageThanksBuff(from)` (0x0058e680), `ThanksResurrect(from)`
    /// (0x0058e5b0): in the party, its row, `#1` the giver's name.
    fn chat_thanks(&mut self, me: usize, at: u32, from: Option<usize>) {
        if self.chat_gate(me) {
            let name = self.chat_name(from);
            let line = self.chat_row(me, at, None);
            self.chat_say(me, line, name, None, None);
        }
    }

    /// `ChatMessageThanksHeal(from, n)` (0x0058e450): in the party, 7 in 8
    /// thanks the healer; then, in the party or not, the "heal me" request
    /// is withdrawn when half the member's HP (as the affect found it,
    /// `hp`) is below `n` or it heals itself (`CheckSkillMask()` bit 4).
    fn chat_thanks_heal(&mut self, me: usize, from: Option<usize>, n: i32, hp: i16) {
        if self.chat_gate(me) && self.chat_draw() & 7 != 0 {
            let name = self.chat_name(from);
            let line = self.chat_row(me, va::THX_HEAL, None);
            self.chat_say(me, line, name, None, None);
        }
        if i32::from(hp) / 2 < n || self.check_skill_mask(me) & 4 != 0 {
            self.chat_drop_heal_me(me);
        }
    }

    /// `ChatMessageConditionModify(sid)` (0x00591380), in the party: the
    /// skill's line (156 and 162 poison and curse, 157 and 160 paralysis
    /// and sleep, 159 and 161 charm and confusion, none for the rest),
    /// then the "cured" request (0x10012) withdrawn and filed again to
    /// itself in 150 frames.
    fn chat_condition_modify(&mut self, me: usize, sid: i32) {
        if !self.chat_gate(me) {
            return;
        }
        let at = match sid {
            156 | 162 => Some(va::POISON_CURSE),
            157 | 160 => Some(va::PARALYSIS_SLEEP),
            159 | 161 => Some(va::CHARM_CONFUSION),
            _ => None,
        };
        if let Some(at) = at {
            self.chat_say_row(me, at);
        }
        let id = self.crew.ais.get(&me).map_or(0, |a| a.sys_msg.id) as u16;
        self.crew.sys.delete_delay(0x10012, id, id);
        let body = self.chat_body(me);
        self.crew.send(0x10012, id, id, 0, 150, -1, Some(body));
    }

    /// `ChatMessageChatCmdAccept(cmd)` (0x0058fb40): the answer to a
    /// command of the CHAT menu.
    fn chat_cmd_accept(&mut self, me: usize, cmd: i32) {
        use va::*;
        match cmd {
            0 | 13 | 14 => {
                if !self.chat_gate(me) {
                    return;
                }
                // By the command the AI holds (chatCmd), not the one given.
                match self.crew.ais.get(&me).map_or(-1, |a| a.chat_cmd) {
                    0 => {
                        let v = self.chat_draw() % 3;
                        let line = self.chat_row(me, USE_ALL_ATTACK_SKILL_ACCEPT, Some(v));
                        self.chat_say(me, line, None, None, None);
                    }
                    13 if self.chat_draw() & 7 != 0 => self.chat_say_row(me, USE_PHYSICAL_SKILL_ACCEPT),
                    14 if self.chat_draw() & 7 != 0 => self.chat_say_row(me, USE_MAGIC_SKILL_ACCEPT),
                    _ => {}
                }
            }
            4 | 10 => self.chat_template(me, STANDBY_ACCEPT),
            5 => self.chat_template(me, SKILL_ACCEPT),
            11 => self.chat_template(me, ASSIGN_ACCEPT),
            15 => self.chat_template(me, NOT_USE_SKILL_ACCEPT),
            16 => self.chat_template(me, HEAL_PLZ_ACCEPT),
            17 => self.chat_template(me, DEBUFF_ACCEPT),
            18 => self.chat_template(me, BUFF_ACCEPT),
            _ => self.chat_chance_variant(me, true, ACCEPT),
        }
    }

    /// `ChatMessageAttackTarget(tp)` (0x0058d7b0): what a member says as it
    /// takes on a foe. Returns 1-3 the level lines, 4 an element, 5 a
    /// crowd, 6 a foe that changes conditions, 0 nothing.
    fn chat_attack_target(&mut self, me: usize, tp: Option<usize>) -> i32 {
        use va::*;
        if !self.chat_gate(me) {
            return 0;
        }
        let body = self.chat_body(me);
        if let Some(t) = tp {
            let ty = self.scene.chars[t].ty();
            if ty & 0xe0 == 0 {
                return 0;
            }
            if ty & 0xc0 == 0 {
                // A plain enemy.
                let id = i32::from(self.scene.chars[body].id());
                let attr = check_char_attribute(&self.scene.chars[t], 0);
                if attr != -1 && self.chat_draw() & 1 != 0 {
                    self.chat_line(me, Chat::AttributeFollow(attr));
                    return 4;
                }
                let n = self.count_target_in_area(me, 0xe0, 0x447a_0000);
                if n >= 6 {
                    if id == 3 || (9..=12).contains(&id) || id == 15 {
                        self.chat_template(me, A_LOT_OF_ENEMY);
                        return 5;
                    }
                } else {
                    let tricks = self.rt.foe_condition_skills(t);
                    if tricks > 0 && matches!(id, 2 | 5 | 6 | 9 | 13 | 14 | 17) {
                        self.chat_template(me, CONDITION_MODIFY_ENEMY);
                        return 6;
                    }
                }
            }
        }
        let Some(t) = tp.or_else(|| self.crew.ais.get(&me).and_then(|a| a.target)) else { return 0 };
        if self.chat_draw() % 3 == 0 {
            return 0;
        }
        let diff = i32::from(self.scene.chars[t].level()) - i32::from(self.scene.chars[body].level());
        let (at, kind) = if diff >= 3 {
            (TIMID, 1)
        } else if diff < -2 {
            (BULL, 2)
        } else {
            (DASH_FOR_ENEMY, 3)
        };
        let v = self.chat_draw() % 3;
        let line = self.chat_row(me, at, Some(v));
        self.chat_say(me, line, None, None, None);
        kind
    }

    /// `ChatMessageAttack(tp, dmg, sid)` (0x00590790): a member's hit on
    /// a foe, every third one (`atkMsgCnt`) worth a line.
    fn chat_attack(&mut self, me: usize, tp: Option<usize>, dmg: i32, sid: i32) {
        use va::*;
        if dmg < 0 {
            return;
        }
        let cnt = {
            let Some(a) = self.crew.ais.get_mut(&me) else { return };
            a.atk_msg_cnt = a.atk_msg_cnt.wrapping_add(1);
            if a.atk_msg_cnt >= 3 {
                a.atk_msg_cnt = 0;
            }
            a.atk_msg_cnt
        };
        if !self.chat_gate(me) {
            return;
        }
        let Some(t) = tp else { return };
        let ty = self.scene.chars[t].ty();
        if self.t.volume != piney_data::volume::Volume::Inf {
            // From Mutation on a blocked hit tells which Exdefense held.
            if dmg == 0 {
                let held = crate::damage::exdefense_held(self.t, self.scene, t, sid);
                if held & 1 != 0 {
                    return self.chat_remark(me, Remark::PhysicalTolerance, 0);
                }
                if held & 2 != 0 {
                    return self.chat_remark(me, Remark::MagicTolerance, 0);
                }
                if held != 0 && cnt >= 0 {
                    let attr = self.t.skill(sid).map_or(-1, |s| crate::skill::skill_attribute(s.ty));
                    self.chat_line(me, Chat::AttributeGuard(attr));
                    if let Some(a) = self.crew.ais.get_mut(&me) {
                        a.atk_msg_cnt = -3;
                    }
                    return;
                }
            }
        } else if sid >= 2 && ty & 0xe0 != 0 {
            let attr = self.t.skill(sid).map_or(-1, |s| crate::skill::skill_attribute(s.ty));
            if attr != -1 && cnt >= 0 && dmg == 0 && check_char_attribute(&self.scene.chars[t], 0) == attr {
                self.chat_line(me, Chat::AttributeGuard(attr));
                if let Some(a) = self.crew.ais.get_mut(&me) {
                    a.atk_msg_cnt = -3;
                }
                return;
            }
        }
        if self.crew.ais.get(&me).is_none_or(|a| a.atk_msg_cnt != 0) {
            return;
        }
        let max = i32::from(self.scene.chars[t].max_hp);
        let third = max / 3;
        let pct = (max / 100).max(1);
        let at = if third < dmg {
            if self.chat_draw() & 1 == 0 {
                return;
            }
            ATTACK_FATAL_HIT
        } else if dmg < pct && max >= 31 {
            let mask = if ty & 0xc0 != 0 { 3 } else { 1 };
            let r = self.chat_draw() & mask;
            if (mask == 3 && r != 0) || (mask == 1 && r == 0) {
                return;
            }
            ATTACK_SMALL_HIT
        } else {
            if self.chat_draw() & 1 == 0 {
                return;
            }
            ATTACK_HIT
        };
        let v = self.chat_draw() % 3;
        let line = self.chat_row(me, at, Some(v));
        self.chat_say(me, line, None, None, None);
    }

    /// `ChatMessageDamage(dmg)` (0x005901d0): a member hurt by a foe, every
    /// third time (`dmgMsgCnt`) worth a line; then, with the HP as the
    /// affect found it (`hp`, not yet lowered), the "heal me" request
    /// filed below a third of the maximum or withdrawn.
    fn chat_damage(&mut self, me: usize, dmg: i32, hp: i16) {
        use va::*;
        let cnt = {
            let Some(a) = self.crew.ais.get_mut(&me) else { return };
            a.dmg_msg_cnt = a.dmg_msg_cnt.wrapping_add(1);
            if a.dmg_msg_cnt >= 3 {
                a.dmg_msg_cnt = 0;
            }
            a.dmg_msg_cnt
        };
        let body = self.chat_body(me);
        let max = i32::from(self.scene.chars[body].max_hp);
        if self.chat_gate(me) && cnt == 0 {
            let third = max / 3;
            let pct = (max / 100).max(1);
            let at = if third < dmg {
                Some(FATAL_DAMAGED)
            } else if dmg < pct && max >= 31 {
                Some(SMALL_DAMAGED)
            } else if dmg >= 10 {
                Some(DAMAGED)
            } else {
                None
            };
            if let Some(at) = at
                && self.chat_draw() & 1 != 0
            {
                let v = self.chat_draw() % 3;
                let line = self.chat_row(me, at, Some(v));
                self.chat_say(me, line, None, None, None);
            }
        }
        let hp = i32::from(hp);
        if hp - dmg < max / 3 && self.check_skill_mask(me) & 4 == 0 {
            self.chat_heal_me(me);
        } else if hp / 2 < hp - dmg || self.check_skill_mask(me) & 4 != 0 {
            self.chat_drop_heal_me(me);
        }
    }

    /// Another party member for a line (`WalkingTalk`, `Neutral`): with
    /// fewer than three the leader; else `(rand() >> 3) % num` again while
    /// it is the member itself, the leader after five draws.
    fn chat_pick_other(&mut self, me: usize) -> Option<usize> {
        let body = self.chat_body(me);
        let num = self.party.num;
        let own = self.party.slot_of(i32::from(self.scene.chars[body].id()));
        let k = if num < 3 {
            0
        } else {
            let mut tries = 0;
            loop {
                let r = self.chat_draw() % num;
                tries += 1;
                if tries >= 5 {
                    break 0;
                }
                if r != own {
                    break r;
                }
            }
        };
        member(self.party, k)
    }

    /// `ChatMessageNeutral()` (0x0058ec30): a member's idle line out of a
    /// fight. First its "heal me" request filed (below a third of its HP)
    /// or withdrawn; then 7 in 8 a line: Rachel (id 12) half the time
    /// compares her weapon with a member's (`BetterWeapon`,
    /// `WorseWeaponMessages`), Natsume (id 11) half the time notes a
    /// member two levels above her (`FellowIsHighLevel`), each turning to
    /// that member; else a variant of its own neutral lines.
    fn chat_neutral(&mut self, me: usize) {
        use va::*;
        if self.game.in_battle != 0 || self.crew.ais.get(&me).is_none_or(|a| a.manual_sw) || !self.chat_gate(me) {
            return;
        }
        let body = self.chat_body(me);
        let (hp, max) = (i32::from(self.scene.chars[body].hp), i32::from(self.scene.chars[body].max_hp));
        if hp < max / 3 && self.check_skill_mask(me) & 4 == 0 {
            self.chat_heal_me(me);
        } else if hp / 2 < hp || self.check_skill_mask(me) & 4 != 0 {
            self.chat_drop_heal_me(me);
        }
        if self.chat_draw() & 7 == 0 {
            return;
        }
        let id = self.scene.chars[body].id();
        if (id == 12 || id == 11) && self.chat_draw() & 1 != 0 {
            let who = self.chat_pick_other(me);
            let name = self.chat_name(who);
            if id == 12 {
                let price = |ctx: &Self, c: Option<usize>| -> i32 {
                    let p = c.and_then(|c| ctx.scene.chars[c].spc());
                    p.and_then(|p| ctx.t.equip(i32::from(p.job), i32::from(p.equipment[4]))).map_or(0, |e| e.price)
                };
                let (mine, theirs) = (price(self, Some(body)), price(self, who));
                if mine < theirs {
                    self.chat_weapon_line(me, BETTER_WEAPON, 12, name);
                    self.chat_face(me, who);
                    return;
                }
                if theirs < mine {
                    self.chat_weapon_line(me, WORSE_WEAPON, 12, name);
                    self.chat_face(me, who);
                    return;
                }
            } else {
                let lv = |c: Option<usize>| c.map_or(0, |c| i32::from(self.scene.chars[c].level()));
                if i32::from(self.scene.chars[body].level()) + 1 < lv(who) {
                    self.chat_weapon_line(me, FELLOW_IS_HIGH_LEVEL, 11, name);
                    self.chat_face(me, who);
                    return;
                }
            }
        }
        let v = variant(self.rng.rand());
        let line = self.chat_row(me, NEUTRAL, Some(v));
        self.chat_say(me, line, None, None, None);
    }

    /// `ChatMessageBetterWeapon(name)` (0x005924b0),
    /// `ChatMessageWorseWeaponMessages(name)` (0x00592590),
    /// `ChatMessageFellowIsHighLevel(name)` (0x00592670): for character
    /// `id` only, in the party, one of the table's three lines by
    /// `(rand() >> 3) % 3`, `#1` the name.
    fn chat_weapon_line(&mut self, me: usize, at: u32, id: i16, name: Option<Vec<u8>>) {
        let body = self.chat_body(me);
        if self.scene.chars[body].id() != id || !self.chat_gate(me) {
            return;
        }
        let v = self.chat_draw() % 3;
        let line = self.t.chat.line(at, v).map(<[u8]>::to_vec);
        self.chat_say(me, line, name, None, None);
    }

    /// `ccAI::ChatMessageSender()` (gcmn 0x00586420), from `Brains`: the
    /// text a line left opened over the member (returned: the runtime's
    /// `ccChatMsg::OpenChat(body, text)`) and `chatRequest` cleared; then
    /// the arrival's count (`arrivalChatCnt`) run down, its line said when
    /// it reaches 0 out of a fight: `ChatMessageEnteredTown` in the Root
    /// Town, `ChatMessageEnteredField` in a field, or a dungeon of field
    /// type 4 outside a story dungeon, come straight from the town
    /// (`areaPrev` 0). Nothing once the party is wiped out.
    pub fn chat_sender(&mut self, me: usize) -> Option<Vec<u8>> {
        if self.party.annihilated(self.scene) {
            return None;
        }
        let a = self.crew.ais.get_mut(&me)?;
        let opened = if a.chat_request {
            a.chat_request = false;
            Some(a.chat_text.clone())
        } else {
            None
        };
        let cnt = a.arrival_chat_cnt;
        if cnt < 0 {
            return opened;
        }
        if cnt == 0 && self.game.in_battle == 0 {
            let g = self.game;
            let field = match g.area {
                0 => Some(Chat::EnteredTown),
                1 => (g.area_prev == 0).then_some(Chat::EnteredField),
                2 => (g.field_type == 4 && g.field == 0 && g.area_prev == 0).then_some(Chat::EnteredField),
                _ => None,
            };
            if let Some(c) = field {
                self.chat_line(me, c);
            }
        }
        if let Some(a) = self.crew.ais.get_mut(&me) {
            a.arrival_chat_cnt -= 1;
        }
        opened
    }

    /// `ccAI::Greeting(from, n)` (gcmn 0x00583190), a greeting affect
    /// (kinds 14 and 15) on a member: its target the greeter (Infection
    /// only; from Mutation on the target stays), `talkFlag`
    /// set, `gDeg` the heading to the greeter (`RAD2DEG(atan2f(d.x,
    /// -d.y))`), and the body stopped (`moveFlag` and `runFlag` cleared).
    pub fn greeting(&mut self, me: usize, from: Option<usize>) {
        let body = self.chat_body(me);
        let Some(f) = from else { return };
        let (a, b) = (self.scene.chars[f].pos_p, self.scene.chars[body].pos_p);
        let dx = ee::sub(a[0], b[0]);
        let dy = ee::sub(a[1], b[1]);
        let h = piney_data::libm::atan2f(dx, piney_data::libm::neg(dy));
        let deg = crate::geom::rad2deg(h);
        let infection = self.t.volume == piney_data::volume::Volume::Inf;
        if let Some(ai) = self.crew.ais.get_mut(&me) {
            // From Mutation on the speaker is not kept as the target.
            if infection {
                ai.target = Some(f);
            }
            ai.talk_flag = true;
            ai.g_deg = deg as u16;
        }
        let s = self.crew.spc.entry(body).or_default();
        s.move_flag = false;
        s.run_flag = false;
    }
}

#[cfg(test)]
mod tests {
    use super::modify;

    #[test]
    fn modify_fills_the_codes() {
        let m = |l: &[u8], a1: Option<&[u8]>, a2: Option<&[u8]>| modify(l, b"Kite", a1, a2, None, b" ");
        assert_eq!(m(b"#0, #1!", None, None), b"Kite, Kite!");
        assert_eq!(m(b"#1 hit #a#b.", Some(b"Orca"), Some(b"Fire")), b"Orca hit Fire .");
        // A name is copied whole past the 79th byte; nothing after it.
        let long = [b'x'; 78];
        let mut line = long.to_vec();
        line.extend_from_slice(b"#0 more");
        let out = modify(&line, b"Kite", None, None, None, b" ");
        assert_eq!(out.len(), 82);
        assert!(out.ends_with(b"Kite"));
        // Any other code: the game never ends; the port stops there.
        assert_eq!(m(b"Hey #@ there", None, None), b"Hey ");
    }
}
