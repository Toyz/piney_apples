//! What a scene's set-up (`ccSetupGameCtrl`) puts back in the save:
//! `ccSaveData::SetSpcItemTown` (INF main 0x00176260), the members' kit
//! when a town is reached from a field or dungeon, and `SetTradeItemTown`
//! (INF main 0x001767a0), the trade lists. Mutation and Outbreak run the
//! same code over their longer tables (21 members, 20 trading members and
//! 54 NPCs, the last ones in the save's extension).

use piney_data::save::{SaveData, by_id, offset};
use piney_data::tables::types::ItemList;
use piney_data::tables::{battle, fieldui, newgame};
use piney_data::volume::Volume;

use crate::item::{add_item, category_order, del_item, get_item_num};

/// A member's list holds this many before `SetSpcItemTown` sells.
const KEPT: usize = 20;
/// `spcParam[i].base.gold`'s cap.
const GOLD_MAX: i32 = 9_999_999;
/// The trades an NPC's default list restocks; the last slot is the
/// town's own pick.
const NPC_TRADES: usize = 15;

/// `ccGetItemPrice(cat, id)` (main 0x00178eb0): equipment (0-9) and items
/// (10-15) by their rows' price; 0 past the categories or a table.
pub fn item_price(volume: Volume, cat: i32, id: i32) -> i32 {
    let b = battle::of(volume);
    let row = usize::try_from(id).ok();
    let equip = |t: &[piney_data::tables::types::EquipmentParam]| row.and_then(|r| t.get(r)).map_or(0, |e| e.price);
    let item = |t: &[piney_data::tables::types::ItemParam]| row.and_then(|r| t.get(r)).map_or(0, |e| e.price);
    match cat {
        0 => equip(b.weapon1()),
        1 => equip(b.weapon2()),
        2 => equip(b.weapon3()),
        3 => equip(b.weapon4()),
        4 => equip(b.weapon5()),
        5 => equip(b.weapon6()),
        6 => equip(b.head()),
        7 => equip(b.body()),
        8 => equip(b.arm()),
        9 => equip(b.leg()),
        10 => item(b.item_r()),
        11 => item(b.item_d()),
        12 => item(b.item_u()),
        13 => item(b.item_x()),
        14 => item(b.item_t()),
        15 => item(b.item_e()),
        _ => 0,
    }
}

/// An entry of a save list (`ccItemList`: id, category, count).
fn entry(save: &SaveData, at: usize) -> ItemList {
    ItemList { id: save.i16(at), category: save.u8(at + 2) as i8, num: save.u8(at + 3) as i8 }
}

fn put(save: &mut SaveData, at: usize, e: ItemList) {
    save.set_i16(at, e.id);
    save.set_u8(at + 2, e.category as u8);
    save.set_u8(at + 3, e.num as u8);
}

/// What `SetSpcItemTown` never sells: the Health Drink to Resurrect and
/// Mage's Soul to Recovery Drink (category 10, rows 0-5 and 18-23), the
/// Sprite Ocarina (13/1), and member 1's Aromatic Grass (14/10).
fn kept_for_good(member: i32, e: ItemList) -> bool {
    match (e.category, e.id) {
        (10, 0..6 | 18..24) | (13, 1) => true,
        (14, 10) => member == 1,
        _ => false,
    }
}

/// `SetSpcItemTown`: every member whose address Kite has (`partyMemberFlag`
/// and `partyMemberCall`) sells the cheapest of what he carries past 20
/// (half its price a piece, his gold capped at 9,999,999), then buys his
/// `spcDefaultItemList` row back up to its counts at full price (his gold
/// down to 0 at most).
pub fn set_spc_item_town(save: &mut SaveData, volume: Volume) {
    let order = category_order(volume);
    let known = save.i32(offset::PARTY_MEMBER_FLAG) & save.i32(offset::PARTY_MEMBER_CALL);
    let rows = battle::of(volume).spc_default_items();
    for (member, kit) in rows.iter().enumerate().skip(1) {
        let m = member as i32;
        if known & (1 << m) == 0 {
            continue;
        }
        let list = by_id::item_list(member);
        let gold = by_id::spc_param(member) + offset::SPC_GOLD;
        let slots = || (0..40).map(move |k| list + 4 * k);
        let carried = slots().filter(|&at| entry(save, at).category >= 0).count();
        for _ in KEPT..carried {
            // The cheapest of what may be sold (the last of equal prices).
            let pick = slots().map(|at| entry(save, at)).filter(|&e| e.category >= 0 && !kept_for_good(m, e)).fold(
                None,
                |best: Option<(i32, ItemList)>, e| {
                    let price = item_price(volume, i32::from(e.category), i32::from(e.id));
                    if price <= best.map_or(GOLD_MAX, |(p, _)| p) { Some((price, e)) } else { best }
                },
            );
            let Some((price, e)) = pick else { break };
            let got = save.i32(gold).wrapping_add(i32::from(e.num).wrapping_mul(price) / 2);
            save.set_i32(gold, got.min(GOLD_MAX));
            del_item(save, m, i32::from(e.category), i32::from(e.id), i32::from(e.num));
        }
        for d in kit.iter().filter(|d| d.category >= 0) {
            let (cat, id) = (i32::from(d.category), i32::from(d.id));
            let short = i32::from(d.num) - get_item_num(save, m, cat, id);
            let cost = if short > 0 {
                add_item(save, &order, m, cat, id, short);
                short.wrapping_mul(item_price(volume, cat, id))
            } else {
                0
            };
            save.set_i32(gold, save.i32(gold).wrapping_sub(cost).max(0));
        }
    }
}

/// `SetTradeItemTown`: each default trade whose switch is on back in its
/// slot, for the members (16 slots) and the NPCs (15); then each NPC, on
/// an odd `rand()`, offers in its last slot a piece of category
/// `rand() % 10` near Kite's own ([`town_pick`]) unless its list has it.
/// `server` is the scene's (`ccGame` +0x1c); `rand` is newlib's.
pub fn set_trade_item_town(save: &mut SaveData, volume: Volume, server: i32, rand: &mut impl FnMut() -> i32) {
    let t = newgame::of(volume);
    for (i, trades) in t.spc_trade().iter().enumerate() {
        restore(save, by_id::spc_trade_list(i), trades.iter().take(16));
    }
    for (k, trades) in t.npc_trade().iter().enumerate() {
        let list = by_id::npc_trade_list(k);
        restore(save, list, trades.iter().take(NPC_TRADES));
        if rand() & 1 == 0 {
            continue;
        }
        let cat = rand() % 10;
        let Some(id) = town_pick(save, volume, server, cat, rand) else { continue };
        let listed =
            (0..NPC_TRADES).map(|j| entry(save, list + 4 * j)).any(|e| i32::from(e.category) == cat && e.id == id);
        if !listed {
            put(save, list + 4 * NPC_TRADES, ItemList { id, category: cat as i8, num: 1 });
        }
    }
}

/// The trades of a default list whose switch is on, into their slots.
fn restore<'a>(
    save: &mut SaveData,
    list: usize,
    trades: impl Iterator<Item = &'a piney_data::tables::types::TradeList>,
) {
    for (j, tr) in trades.enumerate() {
        if tr.sw != 0 {
            put(save, list + 4 * j, tr.lst);
        }
    }
}

/// The piece of category `cat` an NPC offers: Kite's own (his weapon's
/// row in `feTbl[0]` for the weapons, his armour's in `feTbl[cat]`) among
/// the rows `f_limitTbl[10 + server]` counts (`[15 + server]` for
/// category 2), moved -1 to +2 by `rand() % 4` and held within the count;
/// `feTbl[cat]`'s row there. None when Kite's piece is not counted.
fn town_pick(save: &SaveData, volume: Volume, server: i32, cat: i32, rand: &mut impl FnMut() -> i32) -> Option<i16> {
    let f = fieldui::of(volume);
    let (fe, limits) = (f.fe_tbl(), f.f_limit());
    let kite = crate::param::SpcParam::from_save(save, 0).equipment;
    let (key, look) = match cat {
        0..=5 => (kite[4], 0),
        _ => (kite[cat as usize - 6], cat as usize),
    };
    let base = if cat == 2 { 15 } else { 10 };
    let limit = usize::try_from(base + server).ok().and_then(|k| limits.get(k)).map_or(0, |&n| i32::from(n));
    let row = (0..limit).find(|&r| fe.get(look).and_then(|l| l.get(r as usize)) == Some(&key))?;
    let r = (row + rand() % 4 - 1).clamp(0, limit);
    fe.get(cat as usize).and_then(|l| l.get(r as usize)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORCA: usize = 2;
    const EMPTY: ItemList = ItemList { id: -1, category: -1, num: 0 };

    /// A save with Orca's address known and his list empty but `items`.
    fn save_with(items: &[ItemList]) -> SaveData {
        let mut save = SaveData::default();
        save.set_i32(offset::PARTY_MEMBER_FLAG, 1 << ORCA);
        save.set_i32(offset::PARTY_MEMBER_CALL, 1 << ORCA);
        let list = by_id::item_list(ORCA);
        for k in 0..40 {
            put(&mut save, list + 4 * k, items.get(k).copied().unwrap_or(EMPTY));
        }
        save
    }

    /// Past 20 stacks Orca sells the cheapest (the last of equal prices,
    /// never his Healing Potions) for half, then buys his kit back.
    #[test]
    fn a_member_sells_past_twenty_then_buys_his_kit() {
        let v = Volume::Inf;
        let kit: Vec<ItemList> =
            battle::of(v).spc_default_items()[ORCA].iter().copied().filter(|e| e.category >= 0).collect();
        // 22 stacks: his kit at 1 each, then stacks of category 11.
        let mut items: Vec<ItemList> = kit.iter().map(|e| ItemList { num: 1, ..*e }).collect();
        let extra: Vec<ItemList> =
            (0..).map(|id| ItemList { id, category: 11, num: 2 }).take(22 - items.len()).collect();
        items.extend(&extra);
        let mut save = save_with(&items);
        let gold = by_id::spc_param(ORCA) + offset::SPC_GOLD;
        save.set_i32(gold, 100_000);
        set_spc_item_town(&mut save, v);
        let price = |e: &ItemList| item_price(v, i32::from(e.category), i32::from(e.id));
        let mut sold: Vec<&ItemList> = extra.iter().collect();
        sold.sort_by_key(|e| (price(e), std::cmp::Reverse(e.id)));
        sold.truncate(2);
        let n = |e: &ItemList| get_item_num(&save, ORCA as i32, i32::from(e.category), i32::from(e.id));
        for e in &extra {
            assert_eq!(n(e), if sold.contains(&e) { 0 } else { 2 }, "{e:?}");
        }
        let got: i32 = sold.iter().map(|e| 2 * price(e) / 2).sum();
        let cost: i32 = kit.iter().map(|e| i32::from(e.num - 1) * price(e)).sum();
        for e in &kit {
            assert_eq!(n(e), i32::from(e.num), "{e:?}");
        }
        assert_eq!(save.i32(gold), 100_000 + got - cost);
    }

    /// Without both address bits nothing changes.
    #[test]
    fn an_unknown_member_keeps_his_list() {
        let mut save = save_with(&[]);
        save.set_i32(offset::PARTY_MEMBER_CALL, 0);
        let before = save.clone();
        set_spc_item_town(&mut save, Volume::Inf);
        assert!(save == before);
    }

    /// An NPC's list: its restocking trades back, the others as they were;
    /// on an odd draw its last slot the town's pick near Kite's blades.
    #[test]
    fn an_npc_restocks_and_picks() {
        let v = Volume::Inf;
        let t = newgame::of(v);
        let mut save = SaveData::default();
        for k in 0..t.npc_trade().len() {
            for j in 0..16 {
                put(&mut save, by_id::npc_trade_list(k) + 4 * j, EMPTY);
            }
        }
        // Even draws: no picks.
        set_trade_item_town(&mut save, v, 0, &mut || 0);
        for (k, l) in t.npc_trade().iter().enumerate() {
            for j in 0..16 {
                let want = l.get(j).filter(|e| j < NPC_TRADES && e.sw != 0).map_or(EMPTY, |e| e.lst);
                assert_eq!(entry(&save, by_id::npc_trade_list(k) + 4 * j), want, "npc {k} slot {j}");
            }
        }
        // Kite's blades at row 3 of feTbl[0]; draws 1 (odd), 4 (category
        // 4), 2 (+1): feTbl[4]'s row 4 in the last slot.
        let fe = fieldui::of(v).fe_tbl();
        let blade = by_id::spc_param(0) + 0xc8 + 8;
        save.set_i16(blade, fe[0][3]);
        let mut draws = [1, 4, 2].into_iter().cycle();
        set_trade_item_town(&mut save, v, 0, &mut || draws.next().unwrap());
        let last = entry(&save, by_id::npc_trade_list(0) + 4 * NPC_TRADES);
        assert_eq!(last, ItemList { id: fe[4][4], category: 4, num: 1 });
    }

    /// Every volume's members (21 from Mutation on, the last three in the
    /// save's extension) take their kit; the trade lists run through.
    #[test]
    fn every_volume_restocks() {
        for v in Volume::ALL {
            let rows = battle::of(v).spc_default_items();
            let mut save = SaveData::default();
            save.set_i32(offset::PARTY_MEMBER_FLAG, -1);
            save.set_i32(offset::PARTY_MEMBER_CALL, -1);
            for member in 0..rows.len() {
                for k in 0..40 {
                    put(&mut save, by_id::item_list(member) + 4 * k, EMPTY);
                }
            }
            set_spc_item_town(&mut save, v);
            for (member, kit) in rows.iter().enumerate().skip(1) {
                for e in kit.iter().filter(|e| e.category >= 0) {
                    let n = get_item_num(&save, member as i32, i32::from(e.category), i32::from(e.id));
                    assert_eq!(n, i32::from(e.num), "{v:?} member {member} {e:?}");
                }
            }
            let mut r = 7i32;
            set_trade_item_town(&mut save, v, 1, &mut || {
                r = r.wrapping_mul(1_103_515_245).wrapping_add(12345) & 0x7fff_ffff;
                r
            });
        }
    }
}
