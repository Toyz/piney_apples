//! The lists the action button opens on a walking PC and on the town's
//! merchants: `PcMenu` (gcmn 0x00541e90, menu 22: Talk, Trade),
//! `VenderMenu` (0x00542840, 24: Talk, Buy, Sell - the weapon, item and
//! magic shops), `RecorderMenu` (0x00542df0, 25: Talk, Save) and
//! `FairyshopMenu` (0x00543390, 26: Talk, Store Items, Withdraw Items).
//! Their windows are target lists (`disp` 6) that `Disp` draws; the three
//! merchants' handlers differ only in Vender's `list.index`. The steps are
//! in docs/engine/field-ui.md (the lists).

use piney_data::save::by_id;
use piney_data::volume::Volume;

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::system::{extract_menu, item_rows};
use crate::talk::{self, Base, TalkReq, Then};

/// `saveData.tpcTradeListSW[24][3]` (+0x1e7c): a trading PC's (66-89)
/// three trades, 1 while open.
pub const TPC_TRADE_LIST_SW: usize = 0x1e7c;
/// `saveData.pcTradeCount[77]` (+0x686a).
pub const PC_TRADE_COUNT: usize = 0x686a;

/// The pages' texts (none: the lines are the NPC's own, read through
/// `base->msg`).
#[derive(Clone, Debug, Default)]
pub struct Texts {}

/// What the lists do after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// A merchant's list after its close for a lost target: the keys, this
    /// frame's pad (Vender stores the shop's type when `store` is set).
    Keys { store: bool },
}

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// Where `ccSaveData`'s trade counts keep trader (`type`, `id`)'s: on
/// Infection a party member's (`type & 4`, +0x6869 + id) or a PC's (30-79,
/// +0x685d + id); from Mutation on also members 18-20 and six NPCs in the
/// extension ([`by_id::trade_count`], MUT 0x00179ec0 and 0x0017a150).
fn count_at(x: &Ctx, types: u32, id: i32) -> Option<usize> {
    if x.texts.volume != Volume::Inf {
        return by_id::trade_count(types as i32, id);
    }
    let k = if types & 4 != 0 {
        id - 1
    } else if types & 0x18 != 0 && (30..80).contains(&id) {
        id - 13
    } else {
        return None;
    };
    usize::try_from(PC_TRADE_COUNT as i32 + k).ok()
}

/// `ccSaveData::CheckTradeCount(type, id)` (main 0x00178680): the trader's
/// count ([`count_at`]), -1 for anyone else (a PC's starts at -1: never
/// spoken to).
pub fn check_trade_count(x: &Ctx, types: u32, id: i32) -> i32 {
    count_at(x, types, id).map_or(-1, |at| i32::from(x.save.save.u8(at) as i8))
}

/// `ccSaveData::AddTradeCount(type, id)` (main 0x00178700): one more,
/// capped at 99 (a signed byte). Anyone else counts on Infection's entry
/// 0 and nowhere from Mutation on.
pub fn add_trade_count(x: &mut Ctx, types: u32, id: i32) {
    let at = match count_at(x, types, id) {
        Some(at) => at,
        None if x.texts.volume == Volume::Inf => PC_TRADE_COUNT,
        None => return,
    };
    let s = &mut x.save.save;
    let v = (s.u8(at) as i8).wrapping_add(1);
    s.set_u8(at, v as u8);
    if v >= 100 {
        s.set_u8(at, 99);
    }
}

/// `EntryAffect(cmndTarget, plw, n)`.
fn affect(x: &mut Ctx, n: i16) {
    if let Some(t) = x.target.as_ref() {
        let target = t.handle;
        talk::affect(x, target, n);
    }
}

/// `ccMsg->Open(rec, base->name, -1, -1)`: a record as the greeting.
pub fn open_greeting(m: &mut MenuCtrl, x: &mut Ctx, rec: u32, name: &[u8]) {
    talk::open_record(m, x, rec, name, -1, -1);
}

/// The rows: each item's name, 16 glyphs a row, into menuKanji.
fn rows(m: &mut MenuCtrl) {
    let r = item_rows(m);
    extract_menu(m, r);
}

/// `VenderMenu`, `RecorderMenu`, `FairyshopMenu`: `store` for Vender.
pub fn merchant_menu(m: &mut MenuCtrl, x: &mut Ctx, store: bool) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if x.target.is_none() {
                return m.close_menu(x);
            }
            rows(m);
            if m.first_time != 0 {
                if let Some(base) = talk::target_base(m, x)
                    && base.msg != 0
                {
                    affect(x, 14);
                    let server = x.world.game.server;
                    let rec = x.texts.talk.word(base.msg.wrapping_add(4 * server as u32));
                    open_greeting(m, x, rec, &base.name);
                }
                m.lists[i].select = 0;
            }
            m.talk.talk_num = 1;
            m.map_status = 3;
            talk::req(x, TalkReq::MerchantCamera);
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() {
                talk::req(x, TalkReq::Camera(1));
                m.map_status = 1;
                return talk::close(m, x, Then::Merchant(Tail::Keys { store }));
            }
            merchant_keys(m, x, store)
        }
        _ => Flow::Done,
    }
}

/// The merchants' keys (0x00542b44 on).
fn merchant_keys(m: &mut MenuCtrl, x: &mut Ctx, store: bool) -> Flow {
    let i = idx(m);
    let key = pushed_key(x);
    if key == x.save.cancel() {
        if x.target.is_some() {
            affect(x, 0);
        }
        talk::req(x, TalkReq::Camera(1));
        m.map_status = 1;
        return talk::close(m, x, Then::MsgClose);
    }
    if key == x.save.ok() {
        if m.lists[i].select != 0 {
            if m.still == 0 {
                talk::sleep_all(m, x);
            }
            if store {
                let types = talk::target_base(m, x).map_or(0, |b| b.types);
                m.lists[i].index = types as i16;
            }
            x.change_target(None);
        }
        let flow = m.change_menu(x);
        m.msg.close();
        return flow;
    }
    Flow::Done
}

/// The key the lists read: cancel (sound 19), else OK (18), else 0.
fn pushed_key(x: &mut Ctx) -> u32 {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        x.save.cancel()
    } else if x.pushed_ok() {
        x.se(SE_OK);
        x.save.ok()
    } else {
        0
    }
}

/// Whether `PcMenu` gives row `id` Talk and a line at random: the walking
/// PCs 30-65, and from Mutation on (MUT gcmn 0x00560bf4) Sieg and Kaz (120,
/// 121) and the four sign PCs (180-183) too. Anyone else past the trading
/// PCs is greeted with its table read as a record and has no Talk.
fn talks(volume: Volume, id: i32) -> bool {
    (30..66).contains(&id) || (volume != Volume::Inf && matches!(id, 120 | 121 | 180..=183))
}

/// `PcMenu`.
pub fn pc_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if x.target.is_none() {
                return m.close_menu(x);
            }
            rows(m);
            let mut first = false;
            let base = talk::target_base(m, x).unwrap_or_default();
            let id = i32::from(base.id);
            if m.first_time != 0 {
                if base.msg != 0 {
                    affect(x, 14);
                    let rec = if (66..80).contains(&id) {
                        first = true;
                        m.talk.talk_trade_flag = 0;
                        x.texts.talk.word(base.msg)
                    } else if talks(x.texts.volume, id) {
                        m.talk.talk_num = (m.rng.rand() % 3) as i16;
                        m.talk.talk_loop_cnt = 0;
                        m.lists[i].y = 2;
                        x.texts.talk.word(base.msg)
                    } else {
                        m.talk.talk_num = 1;
                        m.lists[i].y = 1;
                        base.msg
                    };
                    open_greeting(m, x, rec, &base.name);
                }
                m.lists[i].select = 0;
                if check_trade_count(x, base.types, id) < 0 {
                    add_trade_count(x, base.types, id);
                }
            }
            if (66..80).contains(&id) {
                trading_line(m, x, &base, first);
                m.lists[i].y = 2;
            }
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() || x.world.game.in_battle != 0 {
                return talk::close(m, x, Then::MsgClose);
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                affect(x, 0);
                return talk::close(m, x, Then::MsgClose);
            }
            if key == x.save.ok() {
                if m.lists[i].select != 0 {
                    x.change_target(None);
                }
                let flow = m.change_menu(x);
                m.msg.close();
                return flow;
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// A trading PC's line (0x00542168 - 0x0054224c): from a random one the
/// first time, else the last (0-2), the first whose trade is still open
/// (`tpcTradeListSW[id - 66]`); -1 when none is.
fn trading_line(m: &mut MenuCtrl, x: &mut Ctx, base: &Base, first: bool) {
    let id = i32::from(base.id);
    let mut s = if first { m.rng.rand() % 3 } else { i32::from(m.talk.talk_num).clamp(0, 2) };
    m.talk.talk_num = -1;
    for _ in 0..3 {
        let at = (TPC_TRADE_LIST_SW as i32 + (id - 66) * 3 + s) as usize;
        if x.save.save.u8(at) != 0 {
            m.talk.talk_num = s as i16;
            break;
        }
        s += 1;
        if s >= 3 {
            s = 0;
        }
    }
}

/// The lists' tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::Keys { store } => {
            m.msg.close();
            talk::then(merchant_keys(m, x, store))
        }
    }
}
