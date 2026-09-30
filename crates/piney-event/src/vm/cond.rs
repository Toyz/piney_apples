//! `ccEvent::CheckOpen` (`INF 0x001a7400`) and `CheckCurrentOpen`
//! (`0x001a7120`).

use super::{Vm, sllv64};
use crate::host::{EntryList, Host};
use crate::ir::{Cmp, Cond};
use crate::state::{DONE, Item, ScriptSave, bit32};
use piney_data::save::offset as off;

/// A comparison as the game codes it: 0 `==`, 1 game `>=` operand, 2 game
/// `<=` operand, anything else fails.
fn cmp(c: i16, game: i32, operand: i32) -> bool {
    Cmp::from_short(c).test(game, operand).unwrap_or(false)
}

impl Vm {
    /// `CheckOpen(eot, n, ev, lv)`: every condition must pass; at level 0
    /// nothing is tested (and the answer is yes). For a block (`ev` >= 0) at
    /// level 1 or 2 the precondition settings are one more condition, tested
    /// first. After the first failure the rest are not evaluated.
    pub(crate) fn check_open<H: Host + ?Sized>(
        &mut self,
        host: &mut H,
        n: i32,
        ev: i32,
        lv: i32,
        conds: &[Cond],
    ) -> bool {
        let (mut total, mut pass) = (0, 0);
        if ev >= 0 && lv > 0 {
            total += 1;
            if self.check_current(host, n) {
                pass += 1;
            }
        }
        for c in conds {
            if lv <= 0 || total != pass {
                continue;
            }
            total += 1;
            if self.cond(host, n, c) {
                pass += 1;
            }
        }
        total == pass
    }

    /// `CheckCurrentOpen(n)`.
    pub(crate) fn check_current<H: Host + ?Sized>(&self, host: &mut H, n: i32) -> bool {
        let c = self.mng.current;
        if c.phase >= 0 && !cmp(c.phase_comp, self.mng.enable_phase, c.phase as i32) {
            return false;
        }
        let game = host.game();
        if c.status >= 0 && game.status != c.status as i32 {
            return false;
        }
        if c.scene[0] >= 0 {
            let g = [game.area, game.town, game.field, game.dungeon, game.floor, game.block];
            if g[0] != c.scene[0] as i32 {
                return false;
            }
            if g.iter().zip(c.scene).skip(1).any(|(&have, want)| want >= 0 && have != want as i32) {
                return false;
            }
        }
        if c.flag >= 0 && host.save().flags(n) & (1u64 << (c.flag & 63)) == 0 {
            return false;
        }
        if c.status_index >= 0 {
            let v = host.save().status(c.status_index as i32) as i32;
            let ok = if c.range {
                c.status_num as i32 <= v && v <= c.status_comp as i32
            } else {
                cmp(c.status_comp, v, c.status_num as i32)
            };
            if !ok {
                return false;
            }
        }
        true
    }

    /// One condition, at level 2.
    pub(crate) fn cond<H: Host + ?Sized>(&self, host: &mut H, n: i32, c: &Cond) -> bool {
        let m = &self.mng;
        let game = host.game();
        match *c {
            Cond::EventDone { event } => host.save().flags(event as i32) & DONE != 0,
            Cond::BlockDone { num } => host.save().flags(n) & (1u64 << (num & 63)) != 0,
            Cond::Phase { phase, comp } => comp.test(m.enable_phase, phase as i32).unwrap_or(false),
            Cond::GameStatus { status } => game.status == status as i32,
            Cond::InPoint { num } => match m.point(num as i32) {
                Some(p) => game.floor == p.floor as i32 && game.block == p.block as i32,
                None => false,
            },
            Cond::Scene { area, town, field, dungeon, floor, block } => {
                [game.area, game.town, game.field, game.dungeon, game.floor, game.block]
                    == [area, town, field, dungeon, floor, block].map(|v| v as i32)
            }
            Cond::InTown { town } => game.area == 0 && game.town == town as i32,
            Cond::InField { town, field } => game.area == 1 && game.town == town as i32 && game.field == field as i32,
            Cond::InDungeon { town, field, dungeon } => {
                game.area == 2
                    && game.town == town as i32
                    && game.field == field as i32
                    && game.dungeon == dungeon as i32
            }
            Cond::Status { index, num, comp } => {
                comp.test(host.save().status(index as i32) as i32, num as i32).unwrap_or(false)
            }
            Cond::StatusRange { index, lo, hi } => {
                let v = host.save().status(index as i32) as i32;
                lo as i32 <= v && v <= hi as i32
            }
            Cond::GateWords { area, except } => {
                let info = host.story_area(area).unwrap_or_default();
                let words = info.words.map(|w| w.unwrap_or(0));
                if m.area_code_set.iter().any(|&w| w < 0) {
                    return false;
                }
                let at_gate = game.server == info.server && (0..3).all(|k| m.area_code_set[k] as i32 == words[k]);
                if at_gate { except == 0 } else { except == 1 }
            }
            Cond::TalkedTo { ty, code } => {
                if m.operate_set != 9 {
                    return false;
                }
                match m.operate_target {
                    Some(t) => host.target_alive(&t) && t.types & bit32(ty as i32) != 0 && t.code == code,
                    None => false,
                }
            }
            Cond::Operate { num, except } => {
                let intercepted = m.operate & sllv64(m.operate_set as i32) != 0;
                match (except, num) {
                    (0, -1) => !intercepted,
                    (0, _) => m.operate_set == num,
                    (1, -1) => intercepted,
                    (1, _) => m.operate_set != -1 && m.operate_set != num,
                    _ => false,
                }
            }
            Cond::NearMarker { marker, bounds, comp } => {
                let pos = match game.area {
                    0 => host.marker(marker).map(|mk| mk.pos),
                    1 | 2 => m.position(marker as i32).map(|p| p.pos),
                    _ => None,
                };
                let d = host.player_distance(pos);
                let b = bounds as i32 as f32 * 10.0;
                match comp {
                    Cmp::Eq => b == d,
                    Cmp::Ge => b <= d,
                    // `c.lt.s b, d` then branch if true: passes unless b < d.
                    Cmp::Le => b.partial_cmp(&d) != Some(std::cmp::Ordering::Less),
                    Cmp::Other(_) => false,
                }
            }
            Cond::Answer { msg, select } => m.msg_num == msg as i32 && m.msg_select == select as i32,
            Cond::BbsRead { thread, post } => host.save().bbs_state(thread as i32, post as i32) == 3,
            Cond::MailGot { mail } => matches!(host.save().mail_state(mail as i32), 4..=6),
            Cond::Mail4 { mail } => host.save().mail_state(mail as i32) == 4,
            Cond::Mail5 { mail } => host.save().mail_state(mail as i32) == 5,
            Cond::Mail6 { mail } => host.save().mail_state(mail as i32) == 6,
            Cond::NewsRead { news } => host.save().news_state(news as i32) == 3,
            Cond::InParty { pc } => {
                let p = host.party();
                if pc == -1 { p.num >= 2 } else { p.slot_of(pc) != -1 }
            }
            Cond::NotInParty { pc } => {
                let p = host.party();
                if pc == -1 { p.num < 2 } else { p.slot_of(pc) == -1 }
            }
            Cond::InPartyOf2 { pc } => {
                let p = host.party();
                p.num == 2 && p.slot_of(pc) != -1
            }
            Cond::PartyOther { pc } => {
                let p = host.party();
                p.ids[1..].iter().any(|&id| id != -1 && id != pc as i32)
            }
            Cond::Callable { pc } => host.save().member_word(off::PARTY_MEMBER_CALL) & bit32(pc as i32) != 0,
            Cond::Present { ty, code } => match ty {
                7 => host.boss().is_some_and(|b| b.entry == code as i32 && b.task_param == Some(0)),
                20 => host.entry_present(EntryList::Gimmicks, ty, code),
                5 | 6 => host.entry_present(EntryList::Characters, ty, code),
                2 => host.spc_present(code),
                _ => false,
            },
            Cond::Absent { ty, code } => match ty {
                7 => host.boss().is_some_and(|b| b.entry == code as i32 && b.task_param.is_some_and(|p| p != 0)),
                20 => !host.entry_present(EntryList::Gimmicks, ty, code),
                5 | 6 => !host.entry_present(EntryList::Characters, ty, code),
                _ => true,
            },
            Cond::NoActive {} => host.no_active_object(),
            Cond::NoEntries {} => host.no_entries(),
            Cond::NoMenu {} => host.field_menu() == -1,
            Cond::HasItem { pc, category, id, num, comp } => has_item(host.save(), pc, category, id, num, comp),
            Cond::Friendship { pc, num, comp } => {
                comp.test(host.save().friendship(pc as i32) as i32, num as i32).unwrap_or(false)
            }
            Cond::Pad { mask } => host.pad_pushed() & (mask as i32 as u32) != 0,
            Cond::EnemyPp { enemy, .. } => host.enemy_pp(enemy),
            Cond::Member { pc } => host.save().member_word(off::PARTY_MEMBER_FLAG) & bit32(pc as i32) != 0,
            Cond::MemberSaved { pc } => host.save().member_word(off::PARTY_MEMBER_SAVE) & bit32(pc as i32) != 0,
            Cond::Volume { num, comp } => comp.test(self.volume(), num as i32).unwrap_or(false),
            Cond::InRoom { floor, block } => game.floor == floor as i32 && game.block == block as i32,
        }
    }
}

/// `has_item`: the count of (`category`, `id`) in `pc`'s list against
/// `num`; for Kite, important items (category 15) count from
/// `impItemList` when the list has none. With `num` 0 and `==` or `<=` the
/// test is "has none".
pub fn has_item(s: &crate::state::SaveData, pc: i16, category: i16, id: i16, num: i16, comp: Cmp) -> bool {
    let Some(p) = usize::try_from(pc).ok().filter(|&p| p < 18) else { return false };
    let items: Vec<Item> = (0..40).map(|k| s.item(p, k)).collect();
    let imp = |id: i16| {
        usize::try_from(id).ok().filter(|&i| i < 320).map_or(0, |i| s.u8(off::IMP_ITEM_LIST + i) as i8 as i32)
    };
    let raw = comp.to_short();
    if num == 0 && (raw == 0 || raw == 2) {
        let found = items.iter().any(|it| it.category as i16 == category && it.id == id);
        if pc == 0 && !found && category == 15 && imp(id) > 0 {
            return false;
        }
        return !found;
    }
    let mut ok = false;
    if let Some(it) = items.iter().find(|it| it.category as i16 == category && it.id == id) {
        ok = cmp(raw, it.count as i32, num as i32);
    }
    if pc == 0 && !ok && category == 15 {
        let a = imp(id);
        if a > 0 {
            ok = cmp(raw, a, num as i32);
        }
    }
    ok
}
