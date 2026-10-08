//! The talk pages: `TalkMenu` (gcmn 0x0054de30, menu 47: what the
//! character says), `SpcMenu` (0x005418a0, 21, a party member), `NpcMenu`
//! (0x00542510, 23, an administrator) and `PresentMenu` (0x00553cd0, 50,
//! Gift) with `PresentMenuDisp` (0x00555020). TalkMenu picks the line by
//! the character's type (a member's by `talkNum[id]`, a trading PC's the
//! next open trade in words, a merchant's by server) and goes back to the
//! list when `Check(1)` answers. The steps are in docs/engine/field-ui.md
//! (Talk, SpcMenu, Gift).

use piney_battle::item as bitem;
use piney_data::save::by_id;
use piney_data::tables::fieldui;
use piney_data::volume::Volume;
use piney_desktop::eef::from_int;

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_MOVE, SE_OK};
use crate::items::{self, Item};
use crate::menus::system::{extract_menu, item_rows, push_msg_requests};
use crate::menus::{breeder, equip, merchant};
use crate::spr::{make_num, set_clm};
use crate::talk::{self, TalkReq, Then};
use crate::window::disp_square_w2;

/// gcmn's literals: " #G", "#W.", "#W, ", "#W,".
pub const STR_GREEN: &[u8] = b" #G";
pub const STR_END: &[u8] = b"#W.";
pub const STR_COMMA_SPACE: &[u8] = b"#W, ";
pub const STR_COMMA: &[u8] = b"#W,";
/// `saveData.talkNum[id]` (+0x220c), a signed byte: from Mutation on
/// read through the getter (MUT main 0x0017abf0), 18-20 in the extension.
pub fn talk_num(x: &Ctx, id: i32) -> i8 {
    usize::try_from(id).map_or(0, |id| x.save.save.u8(by_id::talk_num(id)) as i8)
}

/// The texts: `tpcTalkStr`'s two pieces, `tradeMenuHelp[6]`'s three lines,
/// the colour literals, and `ccCheckVoiceGrp`'s groups by id - 141.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    pub tpc_talk: [Vec<u8>; 2],
    pub trade_none: [Vec<u8>; 3],
    pub green: Vec<u8>,
    pub end: Vec<u8>,
    pub comma_space: Vec<u8>,
    pub comma: Vec<u8>,
    pub voice_grp: Vec<i32>,
}

impl Texts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> Texts {
        use crate::tables::piece;
        let f = piney_data::tables::fieldui::of(volume);
        let none = f.trade_help()[6];
        Texts {
            tpc_talk: [piece(f.tpc_talk(), 0), piece(f.tpc_talk(), 1)],
            trade_none: [piece(none, 0), piece(none, 1), piece(none, 2)],
            green: STR_GREEN.to_vec(),
            end: STR_END.to_vec(),
            comma_space: STR_COMMA_SPACE.to_vec(),
            comma: STR_COMMA.to_vec(),
            voice_grp: f.voice_groups().to_vec(),
        }
    }
}

/// `ccCheckVoiceGrp(id)`.
pub fn check_voice_grp(t: &Texts, id: i32) -> i32 {
    usize::try_from(id - 141).ok().and_then(|k| t.voice_grp.get(k)).copied().unwrap_or(-1)
}

/// What the pages do after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// TalkMenu's close for a lost target, before its `proccess++`.
    TalkClosed,
    /// PresentMenu's breaths.
    Present(PresentTail),
}

/// The pages' tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::TalkClosed => {
            m.proccess += 1;
            None
        }
        Tail::Present(t) => present_tail(m, t, x),
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// Back to the list the page came from (`menuNext = prev`, the window
/// closing, proccess and waitCount 0, both cursors reset).
fn back(m: &mut MenuCtrl) {
    let prev = m.lists[idx(m)].prev;
    m.back_to_prev(prev);
}

/// `TalkMenu`.
pub fn talk_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            if x.target.is_none() {
                return talk::close(m, x, Then::Talk(Tail::TalkClosed));
            }
            let base = talk::target_base(m, x).unwrap_or_default();
            if base.msg == 0 {
                back(m);
                m.proccess += 1;
                return Flow::Done;
            }
            if let Some(t) = x.target.as_ref() {
                let target = t.handle;
                talk::affect(x, target, 15);
            }
            let id = i32::from(base.id);
            let tt = &x.texts.talk;
            let tn = i32::from(m.talk.talk_num);
            if base.types & 4 != 0 {
                let n = i32::from(talk_num(x, id));
                let tbl = tt.word(base.msg.wrapping_add((4 * n) as u32));
                let rec = tbl.wrapping_add((12 * tn) as u32);
                talk::open_record(m, x, rec, &base.name, -31, 2 * id + 1);
            } else if base.types & 8 != 0 {
                if (66..80).contains(&id) {
                    trading_pc(m, x, &base);
                } else {
                    let mut a0 = 1;
                    if m.talk.talk_loop_cnt < 3 {
                        let n = i32::from(talk_num(x, 0));
                        a0 = (n + 1) * 3 + tn;
                        m.talk.talk_loop_cnt += 1;
                        m.talk.talk_num += 1;
                        if m.talk.talk_num >= 3 {
                            m.talk.talk_num = 0;
                        }
                    }
                    let rec = tt.word(base.msg.wrapping_add((4 * a0) as u32));
                    talk::open_record(m, x, rec, &base.name, -1, -1);
                }
            } else if base.types & talkers(x.texts.volume) != 0 {
                // From Mutation on (MUT gcmn 0x0056d244) a breeder whose
                // town has three grown Grunties and mail 324 at 3 or more
                // talks of the race.
                let race = x.texts.volume != Volume::Inf
                    && base.types & 0x0800_0000 != 0
                    && m.pg_adult_num >= 3
                    && x.save.save.mail(breeder::RACE_MAIL) as i8 >= 3;
                let rec = if race {
                    piney_data::tables::race::of(x.texts.volume).talk_va()
                } else {
                    let server = x.world.game.server;
                    let tbl = tt.word(base.msg.wrapping_add((4 * server) as u32));
                    tbl.wrapping_add((12 * tn) as u32)
                };
                talk::open_record(m, x, rec, &base.name, -1, -1);
            } else {
                let rec = base.msg.wrapping_add((12 * tn) as u32);
                let grp = check_voice_grp(&x.texts.talk.talk, id);
                talk::open_record(m, x, rec, &base.name, grp, tn);
            }
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            if x.target.is_none() {
                return talk::close(m, x, Then::Nothing);
            }
            if talk::msg_check(m, x) != 0 {
                back(m);
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// The kinds `TalkMenu` reads a table of lines by server for: from
/// Mutation on the Event NPC's 0x10000000 too (MUT gcmn 0x0056d230).
fn talkers(v: Volume) -> u32 {
    if v == Volume::Inf { 0x0800_1f00 } else { 0x1800_1f00 }
}

/// A trading PC's offer (0x0054df48 - 0x0054e3c0): the next of its three
/// trades still open, in words; `tradeMenuHelp[6]` when none is.
fn trading_pc(m: &mut MenuCtrl, x: &mut Ctx, base: &talk::Base) {
    let id = i32::from(base.id);
    if m.talk.talk_trade_flag == 0 {
        let mut v = i32::from(m.talk.talk_num);
        m.talk.talk_num = -1;
        for _ in 0..3 {
            v += 1;
            if v >= 3 {
                v = 0;
            }
            let at = (crate::menus::merchant::TPC_TRADE_LIST_SW as i32 + (id - 66) * 3 + v) as usize;
            if x.save.save.u8(at) != 0 {
                m.talk.talk_num = v as i16;
                break;
            }
        }
    }
    let t = x.texts.talk.talk.clone();
    if m.talk.talk_num < 0 {
        talk::open_data(m, x, 0, &base.name, [&t.trade_none[0], &t.trade_none[1], &t.trade_none[2]]);
        return;
    }
    m.talk.talk_trade_flag = 0;
    // `tpcTradeList[id - 66][talkNum]`: {id, category, num} x 4.
    let tpc = piney_data::tables::newgame::of(x.texts.volume).tpc_trade();
    let rec = usize::try_from(id - 66).ok().and_then(|i| tpc.get(i)).and_then(|t| t.get(m.talk.talk_num as usize));
    let entry = |k: u32| rec.map_or((0, 0, 0), |r| (r[k as usize].id, r[k as usize].category, r[k as usize].num));
    let offers = (1..4).filter(|&k| entry(k).1 >= 0).count();
    let items = &x.texts.items;
    let piece = |k: u32| {
        let (iid, cat, num) = entry(k);
        let mut s = piney_desktop::kanji::dec2sjis(i32::from(num), 16, 0);
        s.extend_from_slice(&t.green);
        s.extend(items.item_name(i32::from(cat), i32::from(iid)));
        s
    };
    let mut lines: [Vec<u8>; 3] = Default::default();
    lines[0] = t.tpc_talk[0].clone();
    lines[0].extend(piece(0));
    lines[0].extend_from_slice(&t.tpc_talk[1]);
    let mut n = 1;
    lines[1] = piece(1);
    lines[1].extend_from_slice(if offers == 1 { &t.end } else { &t.comma_space });
    if offers >= 2 {
        lines[1].extend(piece(2));
        lines[1].extend_from_slice(if offers == 2 { &t.end } else { &t.comma });
        n = 2;
    }
    if offers == 3 {
        lines[n] = piece(3);
        lines[n].extend_from_slice(&t.end);
    }
    talk::open_data(m, x, 0, &base.name, [&lines[0], &lines[1], &lines[2]]);
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

/// `EntryAffect(cmndTarget, plw, n)`.
fn affect_target(x: &mut Ctx, n: i16) {
    if let Some(t) = x.target.as_ref() {
        let target = t.handle;
        talk::affect(x, target, n);
    }
}

/// `ccSaveData::SetSpcBaseMsg()` (main 0x00176200), which `ccSetupNewGame`
/// calls as the field starts: `spcParam[k].base.msg = spcMsgTbl[k]` for
/// members 1-17, and from Mutation on 1-20 (MUT 0x001777d0, 18-20 in the
/// extension); `spcMsgTbl` has a row more than that. A save keeps whatever
/// pointer was there (`charTbl`'s, DEMO.PRG's). `SpcMenu`, `TalkMenu` and
/// `PresentMenu` read the save's.
pub fn set_spc_base_msg(save: &mut piney_desktop::SaveState, volume: Volume) {
    for (k, &v) in fieldui::of(volume).spc_msg_tbl().iter().enumerate().skip(1) {
        save.save.set_i32(by_id::spc_param(k) + 0x20, v as i32);
    }
}

/// `NpcMenu` (gcmn 0x00542510, menu 23): an administrator (`npcTbl` rows
/// 29 and 158, type 0x10, whose `msg` is `sysopeMsg`) says its line; there
/// is no list. `Check(1)` chains an emode-1 record to the next in its
/// table; the menu shuts when the line is done, or at once in a battle.
pub fn npc_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            if x.target.is_none() {
                return talk::close(m, x, Then::Talk(Tail::TalkClosed));
            }
            let base = talk::target_base(m, x).unwrap_or_default();
            if base.msg == 0 {
                return talk::close(m, x, Then::Talk(Tail::TalkClosed));
            }
            affect_target(x, 15);
            talk::open_record(m, x, base.msg, &base.name, -1, -1);
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            if x.target.is_none() || x.world.game.in_battle != 0 {
                return talk::close(m, x, Then::MsgClose);
            }
            if talk::msg_check(m, x) != 0 {
                return talk::close(m, x, Then::Nothing);
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// `SpcMenu` (gcmn 0x005418a0, menu 21): a party member spoken to (its
/// base is `saveData.spcParam[id].base`); the list is Talk (47), Trade
/// (48), Gift (50). The first time it greets (voice -31, `2 id`) and starts
/// the member's trade count. The steps are in docs/engine/field-ui.md.
pub fn spc_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if x.target.is_none() {
                return talk::close(m, x, Then::Nothing);
            }
            let rows = item_rows(m);
            extract_menu(m, rows);
            if m.first_time != 0 {
                let base = talk::target_base(m, x).unwrap_or_default();
                let id = i32::from(base.id);
                if base.msg != 0 {
                    affect_target(x, 14);
                    let n = i32::from(talk_num(x, id));
                    let rec = x.texts.talk.word(base.msg.wrapping_add((4 * n) as u32));
                    talk::open_record(m, x, rec, &base.name, -31, 2 * id);
                }
                m.lists[i].select = 0;
                if merchant::check_trade_count(x, base.types, id) < 0 {
                    merchant::add_trade_count(x, base.types, id);
                }
            }
            m.talk.talk_num = 1;
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() {
                return talk::close(m, x, Then::MsgClose);
            }
            if x.world.game.in_battle != 0 {
                affect_target(x, 0);
                return talk::close(m, x, Then::MsgClose);
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                x.req.push(Request::VoiceStop);
                affect_target(x, 0);
                return talk::close(m, x, Then::MsgClose);
            }
            if key == x.save.ok() {
                x.req.push(Request::VoiceStop);
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

/// Infection's worth bands for the thanks (`PresentMenu` 0x00554a68: 100,
/// 5000, 10000, 20000); the later volumes' are `fieldui::gift_bands`.
const INF_BANDS: [i32; 4] = [100, 5000, 10_000, 20_000];
/// The friendship each band of thanks adds.
const BAND_FRIENDSHIP: [i32; 5] = [1, 10, 20, 50, 100];
/// What a wanted gift is worth from Mutation on.
const WANTED: i32 = 50_000;

/// How `PresentMenu` weighs and gives a gift. Infection gives it at the
/// question (proccess 4) and weighs its worth alone (6). From Mutation on
/// the member weighs it against what it has (6), and takes it after the
/// thanks (8): MUT gcmn 0x00573660, 0x00573cfc; Outbreak and Quarantine
/// the same, with their own bands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GiftOrder {
    GiveThenThank,
    ThankThenGive,
}

impl GiftOrder {
    fn of(volume: Volume) -> GiftOrder {
        match volume {
            Volume::Inf => GiftOrder::GiveThenThank,
            _ => GiftOrder::ThankThenGive,
        }
    }
}

/// `ccSpcParam` fields: `base.gold` (+0x14), `equipment` (+0xc8: head,
/// body, arm, leg, weapon, shield), `job` (+0xd8), `friendship` (+0xda).
pub const SPC_GOLD: usize = 0x14;
pub const SPC_EQUIPMENT: usize = 0xc8;
pub const SPC_JOB: usize = 0xd8;
pub const SPC_FRIENDSHIP: usize = 0xda;
/// The most gold anyone holds.
const GOLD_MAX: i32 = 9_999_999;

/// What the pages do after a breath (PresentMenu's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentTail {
    /// `AddSpcItem`'s breath for `ccThEquipMenu`: the rest of the gift
    /// (item `cat`/`id`, `n` of it).
    Equip { gift: SpcGift, cat: i32, id: i32, n: i32 },
    /// The tasks woken in town after the gift: the flips back on,
    /// proccess 6.
    Town,
    /// proccess 9's first breath: the member's thanks done, then the
    /// window shuts.
    Given,
    /// proccess 10's wake: back to the member's list.
    Back,
    /// From Mutation on, proccess 8's `AddSpcItem` breath: the rest of the
    /// gift, then the line saying what the member did with it.
    Gave { gift: SpcGift, cat: i32, id: i32, n: i32 },
}

/// A party member's `spcParam` record (`GetSpcParam`, MUT main
/// 0x0017aec0: 18-20 in the extension).
fn spc_at(sid: i32) -> usize {
    by_id::spc_param(sid.max(0) as usize)
}

/// `presentMenuHelp[k]`'s three lines: the count's help, the question
/// ("Give "), "No items you can give.", then what the member did with the
/// gift, by `AddSpcItem`'s answer: [3] received, [4] equipped, [5] used.
fn present_help(x: &Ctx, k: u32) -> [Vec<u8>; 3] {
    use crate::tables::piece;
    let h = piney_data::tables::fieldui::of(x.texts.volume).present_help();
    let l = h.get(k as usize).copied().unwrap_or_default();
    [piece(l, 0), piece(l, 1), piece(l, 2)]
}

/// An equipment row's short: +0x38 the weight class or weapon level (with
/// nothing worn, -1, the row before the table, as the game reads it).
fn equip_level(x: &Ctx, cat: i32, id: i32) -> i16 {
    x.texts.pers.equip_raw.level(cat, id)
}

/// `ccGetItemPrice(cat, id)` (main 0x00178eb0) as the game reads it (an
/// equipment id of -1 too).
fn price_of(x: &Ctx, cat: i32, id: i32) -> i32 {
    match cat {
        0..=9 => x.texts.pers.equip_raw.price(cat, id),
        10..=15 => crate::menus::shop::price(x, cat, id),
        _ => 0,
    }
}

/// `ccGetItemTradeRate(rates[sid - 1], cat, id)` (main 0x00178fb0) with
/// the rates Gift weighs by: `spcTradeRateTbl` on Infection, and from
/// Mutation on a table of its own (`gift_rates`, MUT main 0x0035f620).
/// Items 10-14, armour by its weight class (1-3), the six weapons; 10
/// otherwise.
fn gift_rate(x: &Ctx, sid: i32, cat: i32, id: i32) -> i32 {
    let f = fieldui::of(x.texts.volume);
    let rates = match GiftOrder::of(x.texts.volume) {
        GiftOrder::GiveThenThank => f.spc_trade_rate(),
        GiftOrder::ThankThenGive => f.gift_rates(),
    };
    let row = usize::try_from(sid - 1).ok().and_then(|i| rates.get(i));
    let at = |k: i32| row.and_then(|r| r.get(k as usize)).map_or(10, |&v| i32::from(v));
    match cat {
        10..=14 => at(cat - 10),
        6..=9 => match equip_level(x, cat, id) {
            1 => at(5),
            2 => at(6),
            3 => at(7),
            _ => 10,
        },
        0..=5 => at(8 + cat),
        _ => 10,
    }
}

/// `ccSaveData::AddFriendship(id, n)` (main 0x00177eb0): at least 0, at
/// most the volume's cap.
fn add_friendship(x: &mut Ctx, sid: i32, n: i32) {
    let caps = piney_data::tables::fieldui::of(x.texts.volume).friendship_cap();
    let vol = x.texts.volume.number() as usize;
    let at = spc_at(sid) + SPC_FRIENDSHIP;
    let s = &mut x.save.save;
    let v = s.i16(at).wrapping_add(n as i16);
    s.set_i16(at, v);
    if v < 0 {
        s.set_i16(at, 0);
        return;
    }
    let cap = caps.get(vol).copied().unwrap_or(0);
    if cap < i32::from(v) {
        s.set_i16(at, cap as i16);
    }
}

/// `ccSaveData::ChangeEquipment(sid, cat, id)` (main 0x00177010), that is
/// `ccChangeEquipment(&spcParam[sid].equipment, skillList[sid], cat, id,
/// &spcParam[sid])` (main 0x00177070, [`equip::change_equipment`]) on the
/// member's record and skills in the save.
fn change_equipment(x: &mut Ctx, sid: i32, cat: i32, id: i32) {
    let at = spc_at(sid);
    let list = by_id::skill_list(sid.max(0) as usize);
    let s = &mut x.save.save;
    let mut eq: [i16; 6] = std::array::from_fn(|k| s.i16(at + SPC_EQUIPMENT + 2 * k));
    let mut sk: [i16; 20] = std::array::from_fn(|k| s.i16(list + 2 * k));
    let job = s.i16(at + SPC_JOB);
    equip::change_equipment(&x.texts.pers.equip_raw, &mut eq, &mut sk, cat, id, job);
    for (k, v) in eq.iter().enumerate() {
        s.set_i16(at + SPC_EQUIPMENT + 2 * k, *v);
    }
    for (k, v) in sk.iter().enumerate() {
        s.set_i16(list + 2 * k, *v);
    }
}

/// `ccChar::CalcReal(spc, 1)` on the member given something to wear: its
/// `real` and `tune` worked out again into its record
/// ([`equip::calc_real`], the member as its record has it), and the
/// runtime told ([`Request::CalcReal`]) for the character itself.
fn calc_real(x: &mut Ctx, member: u32, sid: i32) {
    let types = x.save.save.i32(spc_at(sid) + 0x08) as u32;
    let mut c = crate::world::CharInfo { handle: member, types, id: sid as i16, ..Default::default() };
    equip::calc_real(&x.texts.pers.battle, x.save, &mut c);
    x.req.push(Request::CalcReal { target: member });
}

/// A member's gold after `v` more (at most 9999999).
fn add_member_gold(x: &mut Ctx, sid: i32, v: i32) {
    let at = spc_at(sid) + SPC_GOLD;
    let mut g = x.save.save.i32(at).wrapping_add(v);
    if GOLD_MAX < g {
        g = GOLD_MAX;
    }
    x.save.save.set_i32(at, g);
}

/// `ccMenuCtrl::AddSpcItemSub(spc, cat, id, num)` (gcmn 0x00527d80): the
/// item into member `sid`'s bag. Past 99 of it, the rest is sold in town
/// (half price to the member's gold); with the bag full and none of it,
/// the cheapest thing carried goes (not the first six healing items, nor
/// Mia's 14/10), sold in town - or, nothing cheaper, the gift itself.
fn add_spc_item_sub(x: &mut Ctx, sid: i32, cat: i32, id: i32, num: i32) {
    let town = x.world.game.area == 0;
    let s1 = price_of(x, cat, id);
    let mut num = num;
    let n = bitem::get_item_num(&x.save.save, sid, cat, id);
    if n != 0 && n + num >= 100 {
        if town {
            add_member_gold(x, sid, s1.wrapping_mul(n + num - 99) / 2);
        }
    } else if bitem::get_item_slot(&x.save.save, sid) < 0 && bitem::get_item_num(&x.save.save, sid, cat, id) <= 0 {
        let mut s3 = if sid == 1 && cat == 14 && id == 10 { GOLD_MAX } else { s1 };
        let mut s5 = -1i32;
        for k in 0..crate::items::ITEMS {
            let it = items::save_item(x.save, sid.max(0) as usize, k);
            if it.cat == 10 && (0..6).contains(&it.id) {
                continue;
            }
            if sid == 1 && it.cat == 14 && it.id == 10 {
                continue;
            }
            let p = price_of(x, i32::from(it.cat), i32::from(it.id));
            if s3 >= p {
                s3 = p;
                s5 = k as i32;
            }
        }
        let count = if s5 < 0 {
            s3 = s1;
            let c = num;
            num = 0;
            c
        } else {
            let it = items::save_item(x.save, sid.max(0) as usize, s5 as usize);
            bitem::del_item(&mut x.save.save, sid, i32::from(it.cat), i32::from(it.id), i32::from(it.num));
            i32::from(it.num)
        };
        if town {
            add_member_gold(x, sid, s3.wrapping_mul(count) / 2);
        }
    }
    let order = x.texts.talk.shop.order;
    bitem::add_item(&mut x.save.save, &order, sid, cat, id, num);
}

/// `AddSpcItem` between its equipment change and its bag: what it still
/// has to add.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpcGift {
    pub sid: i32,
    pub cat: i32,
    pub id: i32,
    pub num: i32,
    /// The piece taken off (-1 none), added to the bag after.
    pub cao: i32,
    pub ido: i32,
    /// 0 put in the bag, 1 worn, 2 used.
    pub ret: i32,
}

/// What `AddSpcItem` did this frame.
pub enum Added {
    Done(i32),
    /// It breathed once for `ccThEquipMenu`; [`add_spc_item_after`] goes on.
    Breathed(SpcGift),
}

/// `ccMenuCtrl::AddSpcItem(spc, cat, id, num, tf)` (gcmn 0x00527950): an
/// item given to party member `sid` (the character `member`). A book (12)
/// is read at once, `num` times (2). Equipment the member can wear that is
/// dearer than what it wears is put on (1), with `tf` the thread
/// `ccThEquipMenu`, which the menu breathes once for; one fewer then goes
/// to the bag, with the piece taken off. Anything else goes to the bag (0).
#[allow(clippy::too_many_arguments)]
pub fn add_spc_item(
    m: &mut MenuCtrl,
    x: &mut Ctx,
    member: u32,
    sid: i32,
    cat: i32,
    id: i32,
    num: i32,
    tf: bool,
) -> Added {
    if cat == 12 {
        for _ in 0..num {
            talk::req(x, TalkReq::SpcUseItem { spc: member, code: (cat << 16) | (id & 0xffff) });
        }
        return Added::Done(2);
    }
    let mut g = SpcGift { sid, cat, id, num, cao: -1, ido: -1, ret: 0 };
    if cat < 10 {
        let spc = spc_at(sid);
        let job = i32::from(x.save.save.i16(spc + SPC_JOB));
        let most = if cat < 6 {
            if (0..6).contains(&job) && cat == job { 0 } else { -1 }
        } else {
            match job {
                0 | 4 => 2,
                1..=3 => 3,
                5 => 1,
                _ => -1,
            }
        };
        if i32::from(equip_level(x, cat, id)) <= most {
            let slot = if (6..10).contains(&cat) { (cat - 6) as usize } else { 4 };
            let worn = i32::from(x.save.save.i16(spc + SPC_EQUIPMENT + 2 * slot));
            g.cao = cat;
            g.ido = worn;
            if price_of(x, cat, worn) < price_of(x, cat, id) {
                g.ret = 1;
                change_equipment(x, sid, cat, id);
                calc_real(x, member, sid);
                if tf {
                    x.req.push(Request::ChangeEquip { target: member, cat, item: id });
                    crate::disp::disp(m, x);
                    return Added::Breathed(g);
                }
                return Added::Done(add_spc_item_after(x, g));
            }
            g.cao = -1;
            g.ido = -1;
        }
    }
    Added::Done(add_spc_item_end(x, g))
}

/// `AddSpcItem` after the piece is worn: one fewer to the bag.
pub fn add_spc_item_after(x: &mut Ctx, mut g: SpcGift) -> i32 {
    g.num -= 1;
    if g.num <= 0 {
        g.cat = -1;
        g.id = -1;
    }
    add_spc_item_end(x, g)
}

fn add_spc_item_end(x: &mut Ctx, g: SpcGift) -> i32 {
    if g.cat >= 0 && g.id >= 0 {
        add_spc_item_sub(x, g.sid, g.cat, g.id, g.num);
    }
    if g.cao >= 0 && g.ido >= 0 {
        add_spc_item_sub(x, g.sid, g.cao, g.ido, 1);
    }
    g.ret
}

fn cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// `ccMsg->Check(0)`.
fn check(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    push_msg_requests(x, req);
    r
}

/// `OpenInfo(s, 0, 0, 0, -1, -1)`.
fn open_info(m: &mut MenuCtrl, x: &Ctx, s: &[u8]) {
    let names = x.save.names();
    m.msg.open_info([Some(s), None, None, None], &names);
}

/// The count (`waitCount`, 1 .. `most`): up and down by one, left up and
/// right down by ten (as the shops').
fn count_keys(m: &mut MenuCtrl, x: &mut Ctx, most: i32) {
    let rep = x.pad.repeat.bits();
    let most = most as i16;
    if rep & 0x1000 != 0 {
        m.wait_count += 1;
        if most < m.wait_count {
            m.wait_count = most;
        } else {
            x.se(SE_MOVE);
        }
    } else if rep & 0x4000 != 0 {
        m.wait_count -= 1;
        if m.wait_count > 0 {
            x.se(SE_MOVE);
        } else {
            m.wait_count = 1;
        }
    } else if rep & 0x8000 != 0 {
        if m.wait_count < most {
            m.wait_count += 10;
            x.se(SE_MOVE);
            if most < m.wait_count {
                m.wait_count = most;
            }
        } else {
            m.wait_count = most;
        }
    } else if rep & 0x2000 != 0 {
        if m.wait_count >= 2 {
            m.wait_count -= 10;
            x.se(SE_MOVE);
            if m.wait_count <= 0 {
                m.wait_count = 1;
            }
        } else {
            m.wait_count = 1;
        }
    }
}

/// The bag's entry under the cursor (`SetItemList(0, page, list, 0)`).
fn bag_item(m: &MenuCtrl, x: &Ctx, k: i16) -> Item {
    let i = idx(m);
    let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
    list.get(k.max(0) as usize).copied().unwrap_or(Item::NONE)
}

/// `PresentMenu` (gcmn 0x00553cd0, menu 50): Gift, the bag's pages; what
/// is given goes to the member's bag (or is worn, or read), the gift's
/// worth to `spcPresent` and the member's friendship, and the member
/// thanks Kite. The member is `cmndTargetPrev` (SpcMenu dropped it).
/// Nothing sets proccess 20 ("No items you can give.") on menu 50. The
/// steps and the worth's thresholds are in docs/engine/field-ui.md (Gift).
pub fn present_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            if m.still == 0 {
                talk::sleep_all(m, x);
            }
            m.exception_disp = 1;
            m.bg_status = 1;
            let l = &mut m.lists[i];
            l.page_num = 5;
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            crate::menus::item::set_item_list_fit(m, x, 0);
            m.proccess += 1;
        }
        1 => {
            let old = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = crate::menus::item::set_item_list_fit(m, x, 0);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&it.cat) {
                x.se(SE_OK);
                m.item_num = (i32::from(it.cat) << 16) | i32::from(it.id);
                m.change_menu_to(64);
                return Flow::Done;
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                cursors(m);
                m.proccess = 10;
            } else if key == x.save.ok() {
                cursors(m);
                if it.cat >= 0 {
                    m.exception_disp = 2;
                    m.wait_count = 1;
                    m.proccess += 1;
                }
            }
            item_help(m, x, it);
        }
        2 => {
            let it = bag_item(m, x, m.lists[i].select);
            count_keys(m, x, i32::from(it.num));
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.exception_disp = 1;
                cursors(m);
                m.proccess = 1;
            } else if key == x.save.ok() {
                m.menu_status = 3;
                cursors(m);
                m.proccess += 1;
            }
            let h = present_help(x, 0);
            let names = x.save.names();
            m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
        }
        3 if m.menu_status == 0 => {
            let it = bag_item(m, x, m.lists[i].select);
            let mut s = present_help(x, 1)[0].clone();
            s.extend(piney_desktop::kanji::dec2sjis(i32::from(m.wait_count), 16, 0));
            s.extend_from_slice(&x.texts.talk.talk.green);
            s.extend(x.texts.items.item_name(i32::from(it.cat), i32::from(it.id)));
            s.extend_from_slice(&x.texts.talk.talk.end);
            open_info(m, x, &s);
            m.exception_disp = 0;
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.disp = 11;
            l.sx = l.x;
            l.sy = l.y;
            l.x = 6;
            l.y = 2;
            l.index = l.select;
            l.select = 0;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            cursors(m);
            m.proccess += 1;
        }
        4 => {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.menu_status = 3;
                m.msg.close();
                cursors(m);
                m.proccess += 1;
            } else if key == x.save.ok() {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                if m.lists[i].select == 1 {
                    m.proccess += 1;
                    return Flow::Done;
                }
                let it = bag_item(m, x, m.lists[i].index);
                let (cat, id, n) = (i32::from(it.cat), i32::from(it.id), i32::from(m.wait_count));
                if GiftOrder::of(x.texts.volume) == GiftOrder::ThankThenGive {
                    // The count kept for proccess 8 (temp[0]), the gift named.
                    m.talk.temp[0] = n;
                    m.item_num = (cat << 16) | (id & 0xffff);
                    return asked(m, x);
                }
                let (member, sid) = recipient(m, x);
                add_spc_present(x, sid, price_of(x, cat, id).wrapping_mul(n));
                return match add_spc_item(m, x, member, sid, cat, id, n, true) {
                    Added::Done(r) => given(m, x, r, cat, id, n),
                    Added::Breathed(gift) => {
                        talk::breathed(talk::Tail::Talk(Tail::Present(PresentTail::Equip { gift, cat, id, n })))
                    }
                };
            }
        }
        5 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            l.x = l.sx;
            l.y = l.sy;
            l.disp = 4;
            m.exception_disp = 1;
            m.menu_status = 1;
            cursors(m);
            m.proccess = 1;
        }
        6 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            l.x = l.sx;
            l.y = l.sy;
            l.disp = 4;
            let (cat, id) = (m.item_num >> 16, m.item_num & 0xffff);
            let base = talk::prev_base(m, x).unwrap_or_default();
            let sid = i32::from(base.id);
            let (worth, bands) = match GiftOrder::of(x.texts.volume) {
                GiftOrder::GiveThenThank => (inf_worth(x, sid, cat, id), INF_BANDS),
                GiftOrder::ThankThenGive => weighed_worth(x, sid, cat, id),
            };
            // The first band the worth is under, else the last record.
            let k = bands.iter().position(|&b| worth < b).unwrap_or(bands.len());
            let f = fieldui::of(x.texts.volume);
            let tbl = if talk_num(x, sid) == 0 { f.spc_msg_present10_va() } else { f.spc_msg_present11_va() };
            let rec = x.texts.talk.word(tbl.wrapping_add((4 * sid) as u32)).wrapping_add(12 * k as u32);
            add_friendship(x, sid, BAND_FRIENDSHIP[k]);
            talk::open_record(m, x, rec, &base.name, -32, 5 * sid + k as i32);
            m.proccess += 1;
        }
        7 => {
            if check(m, x) != 0 {
                m.msg.close();
                if m.still == 0 {
                    talk::sleep_all(m, x);
                }
                m.bg_status = 1;
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        8 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w < 9 {
                return Flow::Done;
            }
            if GiftOrder::of(x.texts.volume) == GiftOrder::ThankThenGive {
                // The gift given now, as Infection's proccess 4 gave it.
                let it = bag_item(m, x, m.lists[i].index);
                let (cat, id, n) = (i32::from(it.cat), i32::from(it.id), m.talk.temp[0]);
                let (member, sid) = recipient(m, x);
                add_spc_present(x, sid, price_of(x, cat, id).wrapping_mul(n));
                return match add_spc_item(m, x, member, sid, cat, id, n, true) {
                    Added::Done(r) => {
                        gave(m, x, r, cat, id, n);
                        Flow::Done
                    }
                    Added::Breathed(gift) => {
                        talk::breathed(talk::Tail::Talk(Tail::Present(PresentTail::Gave { gift, cat, id, n })))
                    }
                };
            }
            what_was_done(m, x);
        }
        9 => {
            if check(m, x) != 0 {
                m.msg.close();
                m.bg_status = 3;
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Talk(Tail::Present(PresentTail::Given)));
            }
        }
        10 => {
            let prev = x.target_prev.clone();
            x.change_target(prev.as_ref());
            if m.mode == 1 && m.still == 1 {
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Talk(Tail::Present(PresentTail::Back)));
            }
            present_back(m);
        }
        20 => {
            let h = present_help(x, 2);
            open_info(m, x, &h[0]);
            m.wait_count = 0;
            m.proccess += 1;
        }
        21 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 16 {
                m.proccess += 1;
            }
        }
        22 if x.pushed_cancel() => {
            x.se(SE_BACK);
            m.msg.close();
            m.proccess = 10;
        }
        _ => {}
    }
    Flow::Done
}

/// proccess 4 after `AddSpcItem`: its answer in `trapNum`, the item in
/// `itemNum`, the bag's N gone, then [`asked`].
fn given(m: &mut MenuCtrl, x: &mut Ctx, ret: i32, cat: i32, id: i32, n: i32) -> Flow {
    m.trap_num = ret;
    m.item_num = (cat << 16) | (id & 0xffff);
    bitem::del_item(&mut x.save.save, 0, cat, id, n);
    asked(m, x)
}

/// The end of proccess 4's OK: the dim out; in town the tasks woken, then
/// proccess 6.
fn asked(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    m.bg_status = 3;
    if x.world.game.area == 0 {
        x.req.push(Request::WakeAll);
        crate::disp::disp(m, x);
        return talk::breathed(talk::Tail::Talk(Tail::Present(PresentTail::Town)));
    }
    m.proccess += 2;
    Flow::Done
}

/// The member given to (`cmndTargetPrev`) and its id.
fn recipient(m: &MenuCtrl, x: &Ctx) -> (u32, i32) {
    let sid = i32::from(talk::prev_base(m, x).unwrap_or_default().id);
    (x.target_prev.as_ref().map_or(0, |t| t.handle), sid)
}

/// `spcPresent[sid] += v`, at most 9999999 (Infection's proccess 4; from
/// Mutation on the adder MUT main 0x0017ae30, 18-20 in the extension).
fn add_spc_present(x: &mut Ctx, sid: i32, v: i32) {
    let at = by_id::spc_present(sid.max(0) as usize);
    let s = &mut x.save.save;
    s.set_i32(at, s.i32(at).wrapping_add(v));
    if GOLD_MAX < s.i32(at) {
        s.set_i32(at, GOLD_MAX);
    }
}

/// Infection's worth of a gift (proccess 6, 0x00554920): its price at the
/// member's rate in tenths; Mia's Aromatic Grass (14/10) 50000.
fn inf_worth(x: &Ctx, sid: i32, cat: i32, id: i32) -> i32 {
    if sid == 1 && cat == 14 && id == 10 {
        return WANTED;
    }
    price_of(x, cat, id).wrapping_mul(gift_rate(x, sid, cat, id)) / 10
}

/// The piece member `sid` wears in equipment category `cat`'s slot (a
/// weapon's for 0-5, head to leg for 6-9).
fn worn(x: &Ctx, sid: i32, cat: i32) -> i32 {
    let slot = if (6..10).contains(&cat) { (cat - 6) as usize } else { 4 };
    i32::from(x.save.save.i16(spc_at(sid) + SPC_EQUIPMENT + 2 * slot))
}

/// Mutation's worth of a gift (proccess 6, MUT gcmn 0x00573660) and the
/// bands it falls in. A wanted gift (`gift_wants`) is worth 50000 unless
/// it is equipment the member carries or wears. Other equipment is worth
/// its value less that of the member's best of its kind (a weapon: the
/// member's job's), carried or worn; it uses the first bands.
fn weighed_worth(x: &Ctx, sid: i32, cat: i32, id: i32) -> (i32, [i32; 4]) {
    let f = fieldui::of(x.texts.volume);
    let mut worth = price_of(x, cat, id).wrapping_mul(gift_rate(x, sid, cat, id)) / 10;
    let bag = || (0..items::ITEMS).map(|k| items::save_item(x.save, sid.max(0) as usize, k));
    let wanted = usize::try_from(sid - 1).ok().and_then(|r| f.gift_wants().get(r));
    let mut kind = Some(cat).filter(|c| (0..10).contains(c));
    if wanted.is_some_and(|w| w.iter().any(|e| i32::from(e.category) == cat && i32::from(e.id) == id)) {
        let had = kind.is_some()
            && (bag().any(|e| i32::from(e.cat) == cat && i32::from(e.id) == id) || worn(x, sid, cat) == id);
        if !had {
            kind = None;
            worth = WANTED;
        }
    }
    let band = |k: usize| f.gift_bands().get(k).copied().unwrap_or_default();
    let Some(mut kind) = kind else { return (worth, band(1)) };
    let job = i32::from(x.save.save.i16(spc_at(sid) + SPC_JOB));
    if kind < 6 && (0..6).contains(&job) {
        kind = job;
    }
    // The member's dearest of the kind: carried first, then worn.
    let mut best = (0, id);
    for e in bag().filter(|e| i32::from(e.cat) == kind && e.id >= 0) {
        let p = price_of(x, kind, i32::from(e.id));
        if best.0 < p {
            best = (p, i32::from(e.id));
        }
    }
    let on = worn(x, sid, kind);
    let p = price_of(x, kind, on);
    if best.0 < p {
        best = (p, on);
    }
    worth = worth.wrapping_sub(best.0.wrapping_mul(gift_rate(x, sid, kind, best.1)) / 10);
    (worth, band(0))
}

/// Mutation's proccess 8 after `AddSpcItem`: its answer in `trapNum`, the
/// bag's N gone, then [`what_was_done`].
fn gave(m: &mut MenuCtrl, x: &mut Ctx, ret: i32, cat: i32, id: i32, n: i32) {
    m.trap_num = ret;
    bitem::del_item(&mut x.save.save, 0, cat, id, n);
    what_was_done(m, x);
}

/// proccess 8's line: "<name> received / equipped / used #G<item>#W."
/// (`presentMenuHelp[3 + trapNum]`), sound 92 for a book read.
fn what_was_done(m: &mut MenuCtrl, x: &mut Ctx) {
    let mut s = talk::prev_base(m, x).unwrap_or_default().name;
    let k = (3 + m.trap_num).clamp(0, 5) as u32;
    s.extend_from_slice(&present_help(x, k)[0]);
    s.extend_from_slice(&x.texts.gi_green);
    s.extend(x.texts.items.item_name(m.item_num >> 16, m.item_num & 0xffff));
    s.extend_from_slice(&x.texts.talk.talk.end);
    open_info(m, x, &s);
    if m.trap_num == 2 {
        x.se(92);
    }
    m.proccess += 1;
}

/// proccess 10's end: back to the member's list.
fn present_back(m: &mut MenuCtrl) {
    m.bg_status = 3;
    let prev = m.lists[idx(m)].prev;
    m.menu_next = prev;
    m.menu_status = 3;
    m.proccess = 0;
    m.wait_count = 0;
    cursors(m);
}

/// PresentMenu's tails.
fn present_tail(m: &mut MenuCtrl, t: PresentTail, x: &mut Ctx) -> Option<Cont> {
    match t {
        PresentTail::Equip { gift, cat, id, n } => {
            let r = add_spc_item_after(x, gift);
            talk::then(given(m, x, r, cat, id, n))
        }
        PresentTail::Town => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.proccess += 2;
            None
        }
        PresentTail::Given => {
            m.still = 0;
            x.req.push(Request::Still(false));
            x.req.push(Request::TargetFix(false));
            if let Some(p) = x.target_prev.as_ref() {
                let h = p.handle;
                talk::affect(x, h, 0);
                talk::req(x, TalkReq::PresentOther(h));
            }
            talk::then(talk::close(m, x, Then::Nothing))
        }
        PresentTail::Back => {
            m.still = 0;
            x.req.push(Request::Still(false));
            present_back(m);
            None
        }
        PresentTail::Gave { gift, cat, id, n } => {
            let r = add_spc_item_after(x, gift);
            gave(m, x, r, cat, id, n);
            None
        }
    }
}

/// `DispMsg({0x100, name, comment}, 0)` for the item under the cursor, or
/// an empty one.
fn item_help(m: &mut MenuCtrl, x: &mut Ctx, it: Item) {
    let names = x.save.names();
    if it.cat < 0 || it.id < 0 {
        m.msg.disp_msg(0x100, None, [None, None, None], &names);
        return;
    }
    let p = x.texts.items.item(i32::from(it.cat), i32::from(it.id)).cloned().unwrap_or_default();
    m.msg.disp_msg(0x100, Some(&p.name), [Some(&p.comment[0]), Some(&p.comment[1]), Some(&p.comment[2])], &names);
}

/// `PresentMenuDisp` (gcmn 0x00555020): the Items page (`ItemMenuDisp`),
/// and while counting the count's window beside the row: "Give" and the
/// count.
pub fn present_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    crate::menus::item::item_menu_disp(m, x);
    if m.exception_disp != 2 {
        return;
    }
    let l = m.lists[idx(m)].clone();
    let a = m.alpha;
    let s1 = i32::from(l.x) * 14 + 87;
    let s0 = (i32::from(l.select) - i32::from(l.dy)) * 20 + 112;
    m.win_pr.set_colour(7);
    m.win_pr.set_alpha(a);
    m.win_pr.dx = from_int(s1);
    m.win_pr.dy = from_int(s0 - 16);
    disp_square_w2(&mut m.win_pr, 5, 2, 1, None);
    let fr = x.frame_rate;
    m.cursor_pr.disp(&mut m.win_pr, from_int(s1), from_int(s0), 8, 0, a, 6, fr);
    m.kanji_pr_text = x.texts.talk.shop.shop_str.clone();
    set_clm(&mut m.kanji_pr, 16, 0, 0, 1);
    m.kanji_pr.set_colour(7);
    m.kanji_pr.set_alpha(a);
    m.kanji_pr.dx = from_int(s1 + 14);
    m.kanji_pr.dy = from_int(s0);
    m.kanji_pr.make_packet(5);
    m.font.set_colour(22);
    m.font.set_alpha(a);
    m.font.dx = from_int(s1 + 106);
    m.font.dy = from_int(s0);
    make_num(&mut m.font, 2, i32::from(m.wait_count));
}
