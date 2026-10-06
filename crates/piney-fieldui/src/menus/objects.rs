//! The field objects' menus the action button opens without the dim
//! (`ccThGameCtrl`'s table, by the target's base type): `ItemObjMenu`
//! (gcmn 0x00546fd0, menu 34, a breakable), `TrapObjMenu` (0x00547360, 35,
//! a trapped breakable), `SymbolMenu` (0x00546aa0, 36), `VirusMenu`
//! (0x00546dd0, 37), `TimeIdolMenu` (0x00547a50, 39, the Zeit statue) and
//! `FoodMenu` (0x0054a420, 43, a Grunty food). 32, 33 and 38 are
//! [`super::getitem`]'s. TrapObjMenu with no trap or another skill repeats
//! proccess 4 for good, as the game does. See docs/engine/field-ui.md.

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_OK};
use crate::menus::getitem::{area_item, sleep_others};
use crate::menus::system::push_msg_requests;

/// `saveData.breakCount` (+0x7442): the objects broken.
pub const BREAK_COUNT: usize = 0x7442;
/// `saveData.symbolCount` (+0x7444): the symbols used.
pub const SYMBOL_COUNT: usize = 0x7444;
/// `saveData` +0x7446: each Grunty food found (16 shorts, key items
/// 26-41).
pub const FOOD_COUNT: usize = 0x7446;
/// `saveData` +0x6774: the best time-idol rank reached (-1 none), then
/// the five ranks (+0x6775, 40 bytes each: the name, 20 bytes, and the
/// time as "hh:mm:ss", 20 bytes).
pub const TIME_IDOL_BEST: usize = 0x6774;
pub const TIME_IDOL_RANKS: usize = 0x6775;
/// gcmn's ":".
pub const COLON: &[u8] = b":";

/// The objects' texts.
#[derive(Clone, Debug, Default)]
pub struct ObjTexts {
    /// "Received effects of #Y".
    pub get_skill: Vec<u8>,
    /// `timeIdolMenuHelp[k]`'s two pieces: 0 out of the ranking, 1 ranked
    /// in, 2 "Renewed " / " #Wrecord.", 3 "Current time: #R".
    pub time_help: Vec<[Vec<u8>; 2]>,
    /// `timeIdolMenuStr`'s pieces: 0-4 the places, 5-9 their titles.
    pub time_str: Vec<Vec<u8>>,
    pub colon: Vec<u8>,
    pub time_items: [i32; 5],
}

impl ObjTexts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> ObjTexts {
        use crate::tables::piece;
        let f = piney_data::tables::fieldui::of(volume);
        ObjTexts {
            get_skill: piney_data::tables::sjis::encode(f.get_skill()),
            time_help: f.time_idol_help().iter().map(|l| [piece(l, 0), piece(l, 1)]).collect(),
            time_str: f.time_idol_str().iter().map(|l| piney_data::tables::sjis::encode(l)).collect(),
            colon: COLON.to_vec(),
            time_items: f.time_idol_item().try_into().unwrap_or_default(),
        }
    }
}

/// A save counter up by one, at most 10000.
fn count_up(x: &mut Ctx, at: usize) {
    let c = (i32::from(x.save.save.i16(at)) + 1).min(10000);
    x.save.save.set_i16(at, c as i16);
}

/// `wait_count` up; whether it was already `n` or more.
fn waited(m: &mut MenuCtrl, n: i16) -> bool {
    let w = m.wait_count;
    m.wait_count += 1;
    w >= n
}

/// The target opened (`EntryAffect(cmndTarget, plw, 11)`), then dropped.
fn open_target(x: &mut Ctx) -> Option<crate::world::CharInfo> {
    let t = x.target.clone();
    if let Some(t) = &t {
        x.req.push(Request::Affect { target: t.handle, kind: 11 });
    }
    x.change_target(None);
    t
}

/// The message's Check.
fn checked(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let ok = x.save.ok();
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
    push_msg_requests(x, req);
    r != 0
}

/// Proccesses 0-3 of 34 and 35: the target fixed; 4 frames; its item
/// (and for 35 its skill) kept, Kite breaking it; 12 frames, his attack
/// stopped; the target opened and dropped. True at the end of the last.
fn break_open(m: &mut MenuCtrl, x: &mut Ctx, trap: bool) -> bool {
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            if waited(m, 3) {
                m.item_num = -1;
                m.trap_num = -1;
                if let Some(t) = &x.target {
                    m.item_num = t.item;
                    if trap {
                        m.trap_num = i32::from(t.skill);
                    }
                    x.req.push(Request::BreakSomething);
                }
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        2 => {
            m.wait_count += 1;
            if m.wait_count >= 12 {
                x.req.push(Request::AttackCancel);
                m.proccess += 1;
            }
        }
        3 => {
            open_target(x);
            return true;
        }
        _ => {}
    }
    false
}

/// Into GetItemMenu (29) with `itemNum`.
fn to_get_item(m: &mut MenuCtrl) -> Flow {
    m.change_menu_to(29);
    Flow::Done
}

/// `ItemObjMenu` (menu 34).
pub fn item_obj_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if m.proccess < 4 {
        if break_open(m, x, false) {
            count_up(x, BREAK_COUNT);
            if m.rng.rand() % 3 == 0 {
                m.wait_count = 0;
                m.proccess += 1;
            } else {
                x.req.push(Request::TargetFix(false));
                return m.close_menu(x);
            }
        }
        return Flow::Done;
    }
    if m.proccess == 4 && waited(m, 16) {
        if m.still == 0 {
            sleep_others(m, x);
        }
        let n = m.item_num;
        m.item_num = area_item(m, x, n, 3);
        return to_get_item(m);
    }
    Flow::Done
}

/// `TrapObjMenu` (menu 35).
pub fn trap_obj_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if m.proccess < 4 {
        if break_open(m, x, true) {
            m.wait_count = 0;
            m.proccess += 1;
        }
        return Flow::Done;
    }
    match m.proccess {
        4 => {
            if waited(m, 26) {
                if m.still == 0 {
                    sleep_others(m, x);
                }
                // No trap: proccess 4 again (and again: the game's).
                if m.trap_num != -1 {
                    m.proccess += 1;
                }
            }
        }
        5 => {
            let line = match m.trap_num {
                162 => 3,
                156 => 2,
                1 => 1,
                _ => {
                    m.proccess = 4;
                    return Flow::Done;
                }
            };
            let names = x.save.names();
            let t = &x.texts.trap_menu;
            let l = |k: usize| t.get(k).map(Vec::as_slice);
            m.msg.open_info([l(0), l(line), None, None], &names);
            m.wait_count = 0;
            m.proccess += 1;
        }
        6 => {
            if waited(m, 11) {
                m.proccess += 1;
            }
        }
        7 => {
            if x.pushed_ok() {
                x.se(SE_OK);
                m.msg.close();
                m.proccess += 1;
            }
        }
        8 => {
            let n = m.item_num;
            m.item_num = area_item(m, x, n, 3);
            count_up(x, BREAK_COUNT);
            return to_get_item(m);
        }
        _ => {}
    }
    Flow::Done
}

/// `SymbolMenu` (menu 36).
pub fn symbol_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let proccess = m.proccess;
    match proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            m.trap_num = -1;
            if let Some(t) = open_target(x) {
                m.trap_num = i32::from(t.skill);
            }
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            if waited(m, 21) {
                if m.still == 0 {
                    sleep_others(m, x);
                }
                m.proccess += 1;
            }
        }
        2 => {
            let t = x.texts;
            let mut line = t.objects.get_skill.clone();
            line.extend_from_slice(&t.items.skill(m.trap_num).map(|p| p.name.clone()).unwrap_or_default());
            line.extend_from_slice(&t.gi_bang);
            let names = x.save.names();
            m.msg.open_info([Some(&line), None, None, None], &names);
            m.wait_count = 0;
            m.proccess += 1;
        }
        3 => {
            if waited(m, 11) {
                m.proccess += 1;
            }
        }
        4 if checked(m, x) => {
            count_up(x, SYMBOL_COUNT);
            m.msg.close();
            return m.close_menu(x);
        }
        _ => {}
    }
    Flow::Done
}

/// `VirusMenu` (menu 37).
pub fn virus_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            m.item_num = -1;
            if let Some(t) = open_target(x) {
                m.item_num = t.item;
            }
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            if waited(m, 21) {
                if m.still == 0 {
                    sleep_others(m, x);
                }
                m.proccess += 1;
            }
        }
        2 => {
            if m.item_num < 0 {
                m.item_num = 0xf_0000;
            }
            return to_get_item(m);
        }
        _ => {}
    }
    Flow::Done
}

/// `FoodMenu` (gcmn 0x0054a420, menu 43): a Grunty food picked up.
pub fn food_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            m.item_num = -1;
            if let Some(t) = open_target(x) {
                m.item_num = 0xf_0000 | ((i32::from(t.id) + 4) & 0xffff);
            }
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            if waited(m, 21) {
                if m.still == 0 {
                    sleep_others(m, x);
                }
                m.proccess += 1;
            }
        }
        2 => {
            if m.item_num < 0 {
                m.item_num = 0xf_001a;
            }
            let k = (m.item_num & 0xffff) - 26;
            if (0..16).contains(&k) {
                count_up(x, FOOD_COUNT + 2 * k as usize);
            }
            return to_get_item(m);
        }
        _ => {}
    }
    Flow::Done
}

/// A time's "hh:mm:ss" from sixtieths (`dec2sjis(n, buf, 2, 2)` each: two
/// digits, zero-padded; the hours at most 99).
pub fn time_text(t: i32, colon: &[u8]) -> Vec<u8> {
    let two = |n: i32| -> [u8; 2] {
        let n = n.rem_euclid(100);
        [b'0' + (n / 10) as u8, b'0' + (n % 10) as u8]
    };
    let h = (t / 216_000).min(99);
    let min = (t % 216_000) / 3600;
    let s = (t % 3600) / 60;
    let mut out = two(h).to_vec();
    out.extend_from_slice(colon);
    out.extend_from_slice(&two(min));
    out.extend_from_slice(colon);
    out.extend_from_slice(&two(s));
    out
}

/// `sjis2dec(s)` (main 0x0015fa30): two ASCII digits, else -1.
fn two_digits(b: &[u8]) -> i32 {
    match (b.first(), b.get(1)) {
        (Some(&a), Some(&c)) if a.is_ascii_digit() && c.is_ascii_digit() => {
            i32::from(a - b'0') * 10 + i32::from(c - b'0')
        }
        _ => -1,
    }
}

/// `ccSaveData::CheckTimeIdolRankIn(t)` (main 0x001789a0): the first rank
/// whose time (hh at +0, mm at +3, ss at +6) is longer than `t`, -1 none.
pub fn check_time_idol_rank_in(save: &crate::SaveState, t: i32) -> i32 {
    let b = save.save.bytes();
    for k in 0..5 {
        let at = TIME_IDOL_RANKS + 40 * k + 20;
        let s = &b[at..at + 8];
        let total = two_digits(&s[0..]) * 216_000 + two_digits(&s[3..]) * 3600 + two_digits(&s[6..]) * 60;
        if t < total {
            return k as i32;
        }
    }
    -1
}

/// `strcpy` into the save: the bytes up to the NUL, and the NUL.
fn put_str(save: &mut crate::SaveState, at: usize, s: &[u8]) {
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    for (i, &c) in s[..end].iter().chain(std::iter::once(&0)).enumerate() {
        save.save.set_u8(at + i, c);
    }
}

/// A NUL-ended string of the save.
fn get_str(save: &crate::SaveState, at: usize) -> Vec<u8> {
    let b = save.save.bytes();
    let end = b[at..].iter().position(|&c| c == 0).map_or(b.len(), |e| at + e);
    b[at..end].to_vec()
}

/// `ccSaveData::SetTimeIdolRank(rank, name, time)` (main 0x00178870): rank
/// 0-4; unless it is the best already, the best becomes it and the ranks
/// from the old best (4 when none) down to it move one place down; then
/// the name and the time into it.
pub fn set_time_idol_rank(save: &mut crate::SaveState, rank: i32, name: &[u8], time: &[u8]) {
    if !(0..5).contains(&rank) {
        return;
    }
    let best = i32::from(save.save.u8(TIME_IDOL_BEST) as i8);
    if rank != best {
        let mut k = if best < 0 { 4 } else { best };
        save.save.set_u8(TIME_IDOL_BEST, rank as u8);
        while rank < k {
            let to = TIME_IDOL_RANKS + 40 * k as usize;
            let from = to - 40;
            let n = get_str(save, from);
            put_str(save, to, &n);
            let t = get_str(save, from + 20);
            put_str(save, to + 20, &t);
            k -= 1;
        }
    }
    let at = TIME_IDOL_RANKS + 40 * rank as usize;
    put_str(save, at, name);
    put_str(save, at + 20, time);
}

/// `TimeIdolMenu` (menu 39).
pub fn time_idol_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let proccess = m.proccess;
    match proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            open_target(x);
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            if waited(m, 21) {
                if m.still == 0 {
                    sleep_others(m, x);
                }
                m.bg_status = 1;
                m.proccess += 1;
            }
        }
        2 => time_idol_result(m, x),
        3 if checked(m, x) => m.change_menu_to(67),
        _ => {}
    }
    Flow::Done
}

/// TimeIdolMenu's proccess 2: the time, the rank, the items and the
/// message.
fn time_idol_result(m: &mut MenuCtrl, x: &mut Ctx) {
    let t = x.world.game.game_cnt[2];
    let ot = &x.texts.objects;
    let time = time_text(t, &ot.colon);
    let mut best = i32::from(x.save.save.u8(TIME_IDOL_BEST) as i8);
    if best < 0 {
        best = 5;
    }
    let rank = check_time_idol_rank_in(x.save, t);
    let name = get_str(x.save, 0);
    let kind = if rank < 0 {
        m.drain_num = 1;
        m.talk.drain_item[0] = 0xb_0038;
        0
    } else if rank < best {
        m.drain_num = best - rank;
        for k in 0..m.drain_num.max(0) as usize {
            m.talk.drain_item[k] = ot.time_items.get(rank as usize + k).copied().unwrap_or(0);
        }
        set_time_idol_rank(x.save, rank, &name, &time);
        2
    } else {
        m.drain_num = 1;
        m.talk.drain_item[0] = 0xb_0036;
        if rank == best {
            set_time_idol_rank(x.save, rank, &name, &time);
        }
        1
    };
    m.proccess += 1;
    let piece = |v: &[Vec<u8>], k: i32| v.get(k.max(0) as usize).cloned().unwrap_or_default();
    let mut l0 = ot.time_help[3][0].clone();
    l0.extend_from_slice(&time);
    let (l1, l2) = if kind == 2 {
        let h = &ot.time_help[2];
        let mut l1 = h[0].clone();
        l1.extend_from_slice(&piece(&ot.time_str, rank));
        l1.extend_from_slice(&h[1]);
        // The second line: the piece's copy is overwritten by the title.
        (l1, piece(&ot.time_str, rank + 5))
    } else {
        let h = &ot.time_help[kind];
        (h[0].clone(), h[1].clone())
    };
    let names = x.save.names();
    m.msg.open(0x100, None, [Some(&l0), Some(&l1), Some(&l2)], &names);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_as_the_game_writes_them() {
        assert_eq!(time_text(0, b":"), b"00:00:00");
        assert_eq!(time_text(132 * 60, b":"), b"00:02:12");
        assert_eq!(time_text(3 * 216_000 + 4 * 3600 + 5 * 60 + 59, b":"), b"03:04:05");
        assert_eq!(time_text(120 * 216_000, b":")[..2], *b"99");
        assert_eq!(two_digits(b"07"), 7);
        assert_eq!(two_digits(b"\0\0"), -1);
    }

    #[test]
    fn a_rank_moves_the_others_down() {
        let mut s = crate::SaveState::fresh();
        for k in 0..5 {
            put_str(&mut s, TIME_IDOL_RANKS + 40 * k, format!("P{k}").as_bytes());
            put_str(&mut s, TIME_IDOL_RANKS + 40 * k + 20, format!("00:0{}:00", k + 1).as_bytes());
        }
        s.save.set_u8(TIME_IDOL_BEST, 0xff);
        // 90 seconds: second place (the first is 1:00).
        assert_eq!(check_time_idol_rank_in(&s, 90 * 60), 1);
        assert_eq!(check_time_idol_rank_in(&s, 10 * 60 * 60), -1);
        set_time_idol_rank(&mut s, 1, b"Kite", b"00:01:30");
        assert_eq!(s.save.u8(TIME_IDOL_BEST), 1);
        let name = |k: usize| get_str(&s, TIME_IDOL_RANKS + 40 * k);
        assert_eq!(
            [name(0), name(1), name(2), name(3), name(4)],
            [b"P0".to_vec(), b"Kite".to_vec(), b"P1".to_vec(), b"P2".to_vec(), b"P3".to_vec()]
        );
        assert_eq!(get_str(&s, TIME_IDOL_RANKS + 40 + 20), b"00:01:30");
    }
}
