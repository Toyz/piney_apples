//! The Chaos Gate's menus (gcmn menu.cpp): `GateMenu` (28) and its pages
//! Random (57), New Keyword (58), Word List (59), Warp History (60) and
//! Other Servers (61), with `GtNewMenuDisp` (the keyword screen, also
//! drawn by 59, 60 and 87), `GtListMenuDisp`, `GtRecordMenuDisp` and
//! `GtTownMenuDisp`. Every page picks three words, shows the area they make
//! (`SimGenerateCode`), and on "Warp" runs the same code: the area check,
//! the gate hack (62) for a protected story area, else the party's leave
//! and the area change. The steps are in docs/engine/field-ui.md.

use piney_desktop::eef::{add, div, from_int, mul};

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Draw, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::items::EVENT_STATUS;
use crate::menus::check_operate;
use crate::menus::system::{extract_menu, item_rows, push_msg_requests, str_cat};
use crate::spr::{Obj, font_type, make_num, set_clm};
use crate::window::{disp_square, disp_square_sb, set_type};
use crate::words::{UNSET, Word, field_attr};

/// The save's gate lists (`ccSaveData`).
pub const TOWN_MOVE_FLAG: usize = 0x2234;
pub const GATE_LIST_MARK: usize = 0x5048;
pub const GATE_RECORD: usize = 0x50ac;
pub const WORD_LIST: usize = 0x523c;
pub const GATE_ORDER_LIST: usize = 0x5278;
pub const PROTECT_AREA: usize = 0x65c8;
pub const AREA_COUNT: usize = 0x6862;
/// `eventFlag` bit the story area 71 checks (`saveData + 0x5bc0`, bit 62).
pub const FLAG71: usize = 0x5bc0;

/// What happens after a gate menu's own breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateAfter {
    /// The tasks were woken: still 0, the flips back, then `proccess`.
    Proccess(i16),
    /// As `Proccess`, then `waitCount` 0 (PartyInMenuT's wake).
    ProccessWait(i16),
    /// Woken, the dim out, (the message closed,) the target back to the
    /// gate, and back to the gate's list; then (the lists' cancel) the
    /// page's help line.
    Back { close_msg: bool, help: bool },
    /// Other Servers' "Warp": woken, the dim and the window out, the wait
    /// from 0, `proccess` 10.
    TownGo,
    /// GateMenu after its close: the help line.
    GateHelp,
    /// GtNewMenuT's close: its tail (as CloseMenu), then the panels out.
    TutorialDone,
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

fn bit(x: &Ctx, base: usize, k: i32) -> bool {
    if k < 0 {
        return false;
    }
    let w = x.save.save.i32(base + 4 * (k as usize >> 5)) as u32;
    w & (1 << (k & 31)) != 0
}

fn server(x: &Ctx) -> i32 {
    x.world.game.server.clamp(0, 4)
}

/// `gateOrderList[server]`'s story areas (the Word List), in order.
pub fn order_list(x: &Ctx) -> Vec<i16> {
    let at = GATE_ORDER_LIST + 128 * server(x) as usize;
    (0..64).map(|k| x.save.save.i16(at + 2 * k)).filter(|&v| v >= 0).collect()
}

/// `gateRecord[server]`'s codes (the history).
pub fn records(x: &Ctx) -> Vec<i32> {
    let at = GATE_RECORD + 80 * server(x) as usize;
    (0..20).map(|k| x.save.save.i32(at + 4 * k)).filter(|&v| v >= 0).collect()
}

/// The towns of `townMoveFlag` but the current one.
pub fn towns(x: &Ctx) -> Vec<i32> {
    let f = x.save.save.i16(TOWN_MOVE_FLAG) as i32;
    (0..5).filter(|&t| f & (1 << t) != 0 && t != x.world.game.town).collect()
}

/// The words of the list the save holds for a part (A 0, B 1, C 2), by ID.
pub fn part_words(x: &Ctx, part: i32) -> Vec<i32> {
    (0..480)
        .filter(|&i| bit(x, WORD_LIST, i) && x.texts.words.part(i) == part && x.texts.words.word(i).is_some())
        .collect()
}

/// `ccSaveData::SetGateRecord(server, code)` (main 0x00178250): the code
/// to the front of the history unless it is there.
pub fn set_gate_record(x: &mut Ctx, code: i32) {
    let at = GATE_RECORD + 80 * server(x) as usize;
    let s = &mut x.save.save;
    if (0..20).any(|k| s.i32(at + 4 * k) == code) {
        return;
    }
    for k in (1..20).rev() {
        let v = s.i32(at + 4 * (k - 1));
        s.set_i32(at + 4 * k, v);
    }
    s.set_i32(at, code);
}

/// "#B" + the server's symbol + " A B C": the address for the window.
fn address(x: &Ctx, ids: [i32; 3]) -> Vec<u8> {
    let t = x.texts;
    let mut s = t.gt_hash_b.clone();
    s.extend_from_slice(&t.server_str.get(server(x) as usize).cloned().unwrap_or_default());
    for id in ids {
        s.extend_from_slice(&t.gt_space);
        if let Some(w) = t.words.word(id) {
            s.extend_from_slice(&w.text);
        }
    }
    s
}

fn generate(m: &mut MenuCtrl, x: &Ctx, ids: [i32; 3]) {
    let flag71 = (x.save.save.i32(FLAG71 + 4) as u32 >> 30) & 1 != 0;
    m.generated = x.texts.words.generate(ids, x.world.game.server, flag71);
}

fn temp3(m: &MenuCtrl) -> [i32; 3] {
    [m.temp[0], m.temp[1], m.temp[2]]
}

/// `ChangeInfo(l0, l1)` / `OpenInfo(...)` at (39, 444).
fn info(m: &mut MenuCtrl, x: &Ctx, l: [Option<&[u8]>; 2], open: bool) {
    let names = x.save.names();
    if open {
        m.msg.open_info([l[0], l[1], None, None], &names);
    } else {
        m.msg.change_info([l[0], l[1], None, None], &names);
    }
    m.msg.set_pos(39.0, 444.0);
}

/// The close when the events bar the address (or Other Servers is held):
/// the circle closes, target unfixed, the window shut.
fn barred(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if let Some(p) = x.target_prev.clone() {
        x.req.push(Request::Affect { target: p.handle, kind: 0 });
    }
    x.req.push(Request::TargetFix(false));
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont::close(woke))
}

/// `WakeAll; Disp; Breath`, then `after`.
pub(crate) fn wake_breath(m: &mut MenuCtrl, x: &mut Ctx, after: GateAfter) -> Flow {
    x.req.push(Request::WakeAll);
    crate::disp::disp(m, x);
    Flow::Breathed(Cont { woke: false, cursors: false, after: After::Gate(after) })
}

/// "Warp" (0x0055da60 and its copies): the events' check, the gate hack
/// for a protected story area, else the leave begins.
fn warp(m: &mut MenuCtrl, x: &mut Ctx, next: i16) -> Flow {
    let ids = temp3(m);
    x.req.push(Request::AreaCodeSet(ids.map(|v| v as i16)));
    if !x.texts.words.check_area_code(ids, x.world.game.server, &x.world.area_codes) {
        return barred(m, x);
    }
    let ev = m.generated.event;
    // From Mutation on, a story area the bracelet's state bars (MUT gcmn
    // 0x0057e394, the same in each warp menu): Kite's line instead.
    if ev != 0
        && let Some(r) = &x.texts.gate_refusal
        && x.save.save.u8(EVENT_STATUS + r.status) != 0
        && r.areas.contains(&ev)
    {
        m.proccess = 30;
        return Flow::Done;
    }
    if ev != 0
        && let Some(e) = x.texts.words.event(ev)
        && !bit(x, PROTECT_AREA, e.code)
        && e.server == x.world.game.server
        && (e.protect[1] != 0 || e.protect[3] != 0 || e.protect[5] != 0 || e.protect[7] != 0)
    {
        if let Some(p) = x.target_prev.clone() {
            x.req.push(Request::Affect { target: p.handle, kind: 0 });
        }
        m.change_menu_to(62);
        return Flow::Done;
    }
    m.menu_status = 3;
    m.bg_status = 3;
    m.wait_count = 0;
    wake_breath(m, x, GateAfter::Proccess(next))
}

/// From Mutation on, the refusal a warp menu turns to (proccess 30-32, MUT
/// gcmn 0x0057e7f8 and its copies): the target back on the gate, the dim
/// and the window out; after seven frames Kite's line; on its close the
/// menu shut.
fn refusal(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        30 => {
            if let Some(p) = x.target_prev.clone() {
                x.req.push(Request::Affect { target: p.handle, kind: 0 });
            }
            x.req.push(Request::TargetFix(false));
            m.wait_count = 0;
            m.menu_status = 3;
            m.bg_status = 3;
            wake_breath(m, x, GateAfter::Proccess(31))
        }
        31 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7
                && let Some(r) = x.texts.gate_refusal.clone()
            {
                // ccMsg->Open(record, record->name, -1, -1): no voice.
                let names = x.save.names();
                let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
                m.msg.open_record(r.mode, r.name.as_deref(), &lines, &names);
                m.proccess += 1;
            }
            Flow::Done
        }
        _ => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r == 0 {
                return Flow::Done;
            }
            m.menu_next = -1;
            m.menu_status = 3;
            m.bg_status = 3;
            let woke = m.breathe_close(x);
            Flow::Breathed(Cont::close(woke))
        }
    }
}

/// The leave (0x0055dcdc and its copies): each member transferred out at
/// frame 5, 35, 65; at 211 true for the area change.
fn leave(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    m.wait_count += 1;
    for k in 0..3 {
        if i32::from(m.wait_count) == 30 * k as i32 + 5
            && let Some(c) = &x.world.party[k]
        {
            x.req.push(Request::TransferOut(c.handle));
        }
    }
    let w = m.wait_count;
    m.wait_count += 1;
    if w < 211 {
        return false;
    }
    x.req.push(Request::DeleteNoPartyMember);
    true
}

fn count_area(x: &mut Ctx) {
    let v = (x.save.save.i16(AREA_COUNT) as i32 + 1).min(10000);
    x.save.save.set_i16(AREA_COUNT, v as i16);
}

/// Back to the gate's list with the target back on the gate.
fn back(m: &mut MenuCtrl, x: &mut Ctx) {
    m.exception_disp = 0;
    let p = x.target_prev.clone();
    x.change_target(p.as_ref());
    let prev = m.list().prev;
    m.back_to_prev(prev);
}

/// After a gate menu's own breath.
pub fn after(m: &mut MenuCtrl, g: GateAfter, x: &mut Ctx) {
    match g {
        GateAfter::GateHelp => gate_help(m, x),
        GateAfter::TutorialDone => m.panel_status = 3,
        GateAfter::Proccess(p) => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.proccess = p;
        }
        GateAfter::ProccessWait(p) => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.wait_count = 0;
            m.proccess = p;
        }
        GateAfter::TownGo => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.bg_status = 3;
            m.menu_status = 3;
            m.wait_count = 0;
            m.proccess = 10;
        }
        GateAfter::Back { close_msg, help } => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.bg_status = 3;
            if close_msg {
                m.msg.close();
            }
            let menu = m.menu;
            let p = x.target_prev.clone();
            x.change_target(p.as_ref());
            let prev = m.list().prev;
            m.back_to_prev(prev);
            match menu {
                59 if help => list_help(m, x),
                61 if help => town_help(m, x),
                _ => {}
            }
        }
    }
}

// --- GateMenu (28) ------------------------------------------------------------------

pub fn gate_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if m.first_time != 0
                && let Some(t) = x.target.clone()
            {
                x.req.push(Request::Affect { target: t.handle, kind: 11 });
                x.req.push(Request::Flash);
            }
            let rows = item_rows(m);
            extract_menu(m, rows);
            if order_list(x).is_empty() {
                m.reverse_head |= 4;
            }
            if records(x).is_empty() {
                m.reverse_head |= 8;
            }
            if towns(x).is_empty() {
                m.reverse_head |= 16;
            }
            m.proccess += 1;
        }
        1 => {
            m.select(0, 0, false, x);
            let cancel = x.pushed_cancel();
            let ok = !cancel && x.pushed_ok();
            if cancel {
                x.se(SE_BACK);
                if let Some(t) = x.target.clone() {
                    x.req.push(Request::Affect { target: t.handle, kind: 0 });
                    x.req.push(Request::Flash);
                }
                x.req.push(Request::TargetFix(false));
                m.menu_next = -1;
                m.menu_status = 3;
                m.bg_status = 3;
                let woke = m.breathe_close(x);
                return Flow::Breathed(Cont { woke, cursors: false, after: After::Gate(GateAfter::GateHelp) });
            }
            if ok {
                x.se(SE_OK);
                x.change_target(None);
                let blocked = m.lists[i].select == 4 && !check_operate(x, 16, 0);
                if !blocked && m.still == 0 {
                    x.req.push(Request::SleepAll);
                    m.still = 1;
                    x.req.push(Request::Still(true));
                    x.req.push(Request::KeepLayers);
                }
                if let Flow::Breathed(c) = m.change_menu(x) {
                    return Flow::Breathed(Cont { after: After::Gate(GateAfter::GateHelp), ..c });
                }
            }
        }
        _ => {}
    }
    gate_help(m, x);
    Flow::Done
}

fn gate_help(m: &mut MenuCtrl, x: &mut Ctx) {
    let sel = m.list().select.max(0) as usize;
    let Some(h) = x.texts.gate_help.get(sel).cloned() else { return };
    let names = x.save.names();
    m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
}

// --- Random (57) --------------------------------------------------------------------

/// A word of a part at random (`rand() % n`); none held: kept as it was.
fn random_word(m: &mut MenuCtrl, x: &Ctx, part: i32) -> i32 {
    let list = part_words(x, part);
    let r = m.rng.rand();
    if list.is_empty() {
        return m.temp[part as usize];
    }
    list[(r % list.len() as i32) as usize]
}

pub fn random_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 | 1 => {
            if m.proccess == 0 {
                m.menu_status = 0;
                m.bg_status = 1;
                m.exception_disp = 0;
                m.proccess += 1;
            }
            for part in 0..3 {
                m.temp[part as usize] = random_word(m, x, part);
            }
            generate(m, x, temp3(m));
            if m.generated.event == 0 {
                m.proccess += 1;
            }
        }
        2 => {
            m.menu_status = 1;
            m.exception_disp = 4;
            m.lists[i].y = 2;
            m.lists[i].select = 0;
            let text = address(x, temp3(m));
            let head = x.texts.gt_menu_info.clone();
            info(m, x, [Some(&head), Some(&text)], true);
            generate(m, x, temp3(m));
            m.proccess += 1;
        }
        3 => {
            m.select(0, 0, false, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.msg.close();
                m.cursors_init();
                m.proccess = 20;
            } else if x.pushed_ok() {
                x.se(SE_OK);
                m.msg.close();
                m.cursors_init();
                m.proccess = if m.lists[i].select == 0 { m.proccess + 1 } else { 20 };
            }
        }
        4 => return warp(m, x, 5),
        5 => {
            if leave(m, x) {
                let ids = temp3(m);
                if m.generated.event == 0 {
                    set_gate_record(x, ids[0] * 1_000_000 + ids[1] * 1000 + ids[2]);
                }
                count_area(x);
                x.req.push(Request::GoToArea(ids));
            }
        }
        20 => {
            m.menu_status = 3;
            m.bg_status = 3;
            return wake_breath(m, x, GateAfter::Proccess(21));
        }
        21 if m.menu_status == 0 => back(m, x),
        30..=32 => return refusal(m, x),
        _ => {}
    }
    Flow::Done
}

// --- New Keyword (58) ---------------------------------------------------------------

/// A part's list: the rows (`my`) and the cursor on the word kept.
fn open_part(m: &mut MenuCtrl, x: &Ctx, part: usize, excp: i16) {
    let i = idx(m);
    let h = x.texts.gt_new_info.get(part).cloned().unwrap_or_default();
    info(m, x, [Some(&h[0]), Some(&h[1])], false);
    m.menu_status = 1;
    m.exception_disp = excp;
    m.kanji_alpha = -24;
    let l = &mut m.lists[i];
    l.index = part as i16;
    l.y = 8;
    l.dy = m.temp[3 + part] as i16;
    l.my = 0;
    l.select = 0;
    for w in part_words(x, part as i32) {
        if m.temp[part] == w {
            m.lists[i].select = m.lists[i].my;
        }
        m.lists[i].my += 1;
    }
}

/// OK on a part's list: its word and scroll kept.
fn pick_part(m: &mut MenuCtrl, x: &Ctx) {
    let i = idx(m);
    let part = m.lists[i].index.clamp(0, 2) as usize;
    let list = part_words(x, part as i32);
    if let Some(&w) = list.get(m.lists[i].select.max(0) as usize) {
        m.temp[part] = w;
        m.temp[3 + part] = i32::from(m.lists[i].dy);
    }
}

pub fn new_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 | 1 => {
            if m.proccess == 0 {
                m.lists[i].disp = 4;
                m.exception_disp = 0;
                m.menu_status = 0;
                m.bg_status = 1;
                m.temp[..6].fill(0);
                m.reverse_head = 0;
                m.proccess += 1;
            }
            open_part(m, x, 0, 1);
            m.proccess += 1;
        }
        3 => {
            open_part(m, x, 1, 2);
            m.proccess += 1;
        }
        5 => {
            open_part(m, x, 2, 3);
            m.proccess += 1;
        }
        2 | 4 | 6 => {
            m.select_scr(0, 0, 0, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.kanji_alpha = -24;
                m.cursors_init();
                match m.proccess {
                    2 => {
                        m.msg.close();
                        m.proccess = 20;
                    }
                    4 => m.proccess = 1,
                    _ => m.proccess = 3,
                }
            } else if x.pushed_ok() {
                x.se(SE_OK);
                m.kanji_alpha = -24;
                m.cursors_init();
                pick_part(m, x);
                m.proccess += 1;
            }
        }
        7 => {
            let text = address(x, temp3(m));
            let head = x.texts.gt_menu_info.clone();
            info(m, x, [Some(&head), Some(&text)], false);
            m.menu_status = 1;
            m.exception_disp = 4;
            // From Mutation on the words fade in again (MUT gcmn 0x0057e0f8).
            if x.texts.words.volume > 1 {
                m.kanji_alpha = -24;
            }
            m.lists[i].y = 2;
            m.lists[i].select = 0;
            generate(m, x, temp3(m));
            m.proccess += 1;
        }
        8 => {
            m.select(0, 0, false, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.kanji_alpha = -24;
                m.cursors_init();
                m.proccess = 5;
            } else if x.pushed_ok() {
                x.se(SE_OK);
                // As on cancel, from Mutation on (MUT gcmn 0x0057e1ec).
                if x.texts.words.volume > 1 {
                    m.kanji_alpha = -24;
                }
                m.cursors_init();
                m.proccess = if m.lists[i].select == 0 { m.proccess + 1 } else { 5 };
            }
        }
        9 => {
            m.msg.close();
            return warp(m, x, 10);
        }
        10 => {
            if leave(m, x) {
                let ev = m.generated.event;
                let ids = temp3(m);
                if ev == 0
                    || (36..39).contains(&ev)
                    || (68..71).contains(&ev)
                    || (92..100).contains(&ev)
                    || ev == 118
                    || ev == 119
                {
                    set_gate_record(x, ids[0] * 1_000_000 + ids[1] * 1000 + ids[2]);
                }
                count_area(x);
                x.req.push(Request::GoToArea(ids));
            }
        }
        20 => {
            m.menu_status = 3;
            m.bg_status = 3;
            return wake_breath(m, x, GateAfter::Proccess(21));
        }
        21 if m.menu_status == 0 => back(m, x),
        30..=32 => return refusal(m, x),
        _ => {}
    }
    Flow::Done
}

// --- Word List (59) and Warp History (60) -------------------------------------------

fn list_help(m: &mut MenuCtrl, x: &mut Ctx) {
    let codes = order_list(x);
    let sel = m.list().select.max(0) as usize;
    let code = codes.get(sel).copied().unwrap_or(0).max(0) as usize;
    // DispMsg(ccEvMsgData): the note's lines.
    let t = x.texts.gate_word_list_msg.get(code).cloned().unwrap_or_default();
    let names = x.save.names();
    m.msg.disp_msg_record(0x100, None, &[&t[0], &t[1], &t[2]], &names);
}

/// The first three steps both lists share (proccess 0, 1).
fn list_open(m: &mut MenuCtrl, x: &Ctx, n: usize, cursors: bool) {
    let i = idx(m);
    match m.proccess {
        0 => {
            m.bg_status = 1;
            m.lists[i].disp = 4;
            if n == 0 {
                let w = x.texts.gt_list_warn.clone();
                let names = x.save.names();
                m.msg.open_info([Some(&w), None, None, None], &names);
                m.proccess = 10;
            } else {
                m.lists[i].index = 0;
                m.lists[i].dy = 0;
                m.proccess += 1;
            }
        }
        1 => {
            let l = &mut m.lists[i];
            l.x = 23;
            l.select = l.index;
            l.y = (n as i16).min(8);
            l.disp = 4;
            m.menu_status = 1;
            m.exception_disp = 1;
            if cursors {
                m.cursors_init();
            }
            m.proccess += 1;
        }
        _ => {}
    }
}

/// The confirmation for a listed address (proccess 3).
fn list_confirm(m: &mut MenuCtrl, x: &Ctx, ids: [i32; 3]) {
    let i = idx(m);
    m.temp[..3].copy_from_slice(&ids);
    m.menu_status = 1;
    m.exception_disp = 4;
    m.lists[i].y = 2;
    m.lists[i].select = 0;
    m.cursors_init();
    let text = address(x, ids);
    let head = x.texts.gt_menu_info.clone();
    info(m, x, [Some(&head), Some(&text)], true);
    generate(m, x, ids);
    m.proccess += 1;
}

/// The Warp / Cancel question (proccess 4) and its return (5).
fn list_question(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    match m.proccess {
        4 => {
            m.select(0, 0, false, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.msg.close();
                m.cursors_init();
                m.menu_status = 3;
                m.proccess += 1;
            } else if x.pushed_ok() {
                x.se(SE_OK);
                m.msg.close();
                m.cursors_init();
                if m.lists[i].select == 0 {
                    m.proccess += 2;
                } else {
                    m.menu_status = 3;
                    m.proccess += 1;
                }
            }
        }
        5 if m.menu_status == 0 => {
            if m.menu == 60 {
                m.cursors_init();
            }
            m.proccess = 1;
        }
        _ => {}
    }
}

pub fn list_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let codes = order_list(x);
    m.lists[i].my = codes.len() as i16;
    match m.proccess {
        0 | 1 => list_open(m, x, codes.len(), true),
        2 => {
            m.select_scr(0, 0, 0, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                return wake_breath(m, x, GateAfter::Back { close_msg: false, help: true });
            }
            if x.pushed_ok() {
                x.se(SE_OK);
                m.cursors_init();
                m.menu_status = 3;
                m.lists[i].index = m.lists[i].select;
                m.proccess += 1;
            }
            list_help(m, x);
        }
        3 => {
            if m.menu_status == 0 {
                let code = codes.get(m.lists[i].index.max(0) as usize).copied().unwrap_or(0);
                let w = &x.texts.words;
                let ids = match w.event(i32::from(code)).and_then(|e| e.words.clone()) {
                    Some(t) => [w.id_of(&t[0]), w.id_of(&t[1]), w.id_of(&t[2])],
                    None => [-1; 3],
                };
                list_confirm(m, x, ids);
            }
        }
        4 | 5 => list_question(m, x),
        6 => return warp(m, x, 7),
        7 => {
            if leave(m, x) {
                count_area(x);
                x.req.push(Request::GoToArea(temp3(m)));
            }
        }
        10 => return check_back(m, x),
        30..=32 => return refusal(m, x),
        _ => {}
    }
    Flow::Done
}

/// The warning's Check, then back to the gate.
fn check_back(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let ok = x.save.ok();
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
    push_msg_requests(x, req);
    if r == 0 {
        return Flow::Done;
    }
    wake_breath(m, x, GateAfter::Back { close_msg: m.menu == 61, help: false })
}

pub fn record_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let recs = records(x);
    match m.proccess {
        0 | 1 => {
            if m.proccess == 0 {
                m.lists[i].my = recs.len() as i16;
            }
            list_open(m, x, recs.len(), false);
        }
        2 => {
            m.select_scr(0, 0, 0, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                return wake_breath(m, x, GateAfter::Back { close_msg: false, help: true });
            }
            if x.pushed_ok() {
                x.se(SE_OK);
                m.cursors_init();
                m.menu_status = 3;
                m.lists[i].index = m.lists[i].select;
                m.proccess += 1;
            }
        }
        3 => {
            if m.menu_status == 0 {
                let v = recs.get(m.lists[i].index.max(0) as usize).copied().unwrap_or(0);
                list_confirm(m, x, [v / 1_000_000, (v % 1_000_000) / 1000, v % 1000]);
            }
        }
        4 | 5 => list_question(m, x),
        6 => return warp(m, x, 7),
        7 => {
            if leave(m, x) {
                count_area(x);
                x.req.push(Request::GoToArea(temp3(m)));
            }
        }
        10 => return check_back(m, x),
        30..=32 => return refusal(m, x),
        _ => {}
    }
    Flow::Done
}

// --- Other Servers (61) -------------------------------------------------------------

fn town_help(m: &mut MenuCtrl, x: &mut Ctx) {
    let ts = towns(x);
    let sel = m.list().select.max(0) as usize;
    let t = ts.get(sel).copied().unwrap_or(0).max(0) as usize;
    let Some(h) = x.texts.gt_town_help.get(t).cloned() else { return };
    let names = x.save.names();
    m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
}

pub fn town_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            if !check_operate(x, 16, 0) {
                return barred(m, x);
            }
            m.bg_status = 1;
            let ts = towns(x);
            let mut buf = Vec::new();
            for &t in &ts {
                let s = x.texts.gt_town_str.get(t as usize).cloned().unwrap_or_default();
                str_cat(&mut buf, &s, 16);
            }
            m.lists[i].y = ts.len() as i16;
            if ts.is_empty() {
                let w = x.texts.gt_town_warn.clone();
                let names = x.save.names();
                m.msg.open_info([Some(&w), None, None, None], &names);
                m.proccess = 1;
            } else {
                m.lists[i].disp = 8;
                m.menu_status = 1;
                m.exception_disp = 1;
                extract_menu(m, buf);
                m.proccess = 2;
            }
        }
        1 => return check_back(m, x),
        2 => {
            let ts = towns(x);
            m.select(0, 0, false, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                return wake_breath(m, x, GateAfter::Back { close_msg: false, help: true });
            }
            if x.pushed_ok() {
                x.se(SE_OK);
                m.cursors_init();
                m.menu_status = 3;
                m.lists[i].index = ts.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(0) as i16;
                m.proccess += 1;
            }
            town_help(m, x);
        }
        3 => {
            if m.menu_status == 0 {
                m.proccess += 1;
            }
        }
        4 => {
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.disp = 11;
            l.x = 6;
            l.y = 2;
            l.page = l.select;
            l.select = 0;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            let mut text = x.texts.gt_menu_info.clone();
            let t = m.lists[i].index.max(0) as usize;
            text.extend_from_slice(&x.texts.gt_town_str.get(t).cloned().unwrap_or_default());
            text.extend_from_slice(&x.texts.gt_dot);
            let names = x.save.names();
            m.msg.open_info([Some(&text), None, None, None], &names);
            m.cursors_init();
            m.proccess += 1;
        }
        5 => {
            m.select(0, 0, false, x);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.msg.close();
                m.cursors_init();
                m.menu_status = 3;
                m.proccess += 1;
            } else if x.pushed_ok() {
                x.se(SE_OK);
                m.msg.close();
                m.cursors_init();
                if m.lists[i].select == 1 {
                    m.menu_status = 3;
                    m.proccess += 1;
                } else {
                    return wake_breath(m, x, GateAfter::TownGo);
                }
            }
        }
        6 => {
            if m.menu_status == 0 {
                let l = &mut m.lists[i];
                l.x = 9;
                l.select = l.page;
                m.cursors_init();
                m.proccess = 0;
            }
        }
        10 => {
            let gone = leave(m, x);
            if gone {
                x.req.push(Request::ChangeArea { area: 0, n: i32::from(m.lists[i].index) });
            }
        }
        _ => {}
    }
    Flow::Done
}

// --- The drawing --------------------------------------------------------------------

/// `ccGetGtNewColor(priA, priB, priC, fA, fB, fC, s)` (gcmn 0x0055f310):
/// how many of the other words that set this attribute outrank word `s`,
/// as a colour (2, 4, 17 from gcmn 0x00651718).
pub fn gt_new_colour(pri: [i32; 3], f: [i32; 3], s: usize, colours: [i32; 3]) -> usize {
    let mut v = 0;
    for o in 0..3 {
        if o != s && f[o] != UNSET && pri[s] < pri[o] {
            v += 1;
        }
    }
    colours[v] as usize
}

fn word_text(x: &Ctx, id: i32) -> Option<Vec<u8>> {
    x.texts.words.word(id).map(|w| w.text.clone())
}

/// `ccKanji::Disp(settingKanji[s], text, count)`: drawn at once.
fn setting_text(m: &mut MenuCtrl, s: usize, text: Vec<u8>, count: i32, kt: i32, pos: (f32, f32), alpha: i32) {
    let k = &mut m.setting[s];
    k.set_colour(7);
    k.set_alpha(alpha);
    k.dx = pos.0;
    k.dy = pos.1;
    let rgba = [k.rgb[0], k.rgb[1], k.rgb[2], alpha as u8];
    m.draws.push(Draw::Text { obj: Obj::Setting(s as u8), text, dx: pos.0, dy: pos.1, rgba, count, kt });
    m.setting_send[s] = false;
}

/// The box around a part's name or an attribute column: eight cells of
/// the window's frame (wv 3456, 16x16 each, 4 a row).
#[allow(clippy::too_many_arguments)]
fn frame_box(m: &mut MenuCtrl, x0: i32, y0: i32, mid: f32, tall: f32, right: f32, alpha: i32) {
    let w = &mut m.win;
    w.set_grid(16, 16, 16.0, 16.0, 0, 3456, 4);
    w.set_alpha(alpha);
    w.dx = from_int(x0);
    w.dy = from_int(y0);
    w.make_packet(0);
    w.sx = mid;
    w.sy = 16.0;
    w.make_packet(4);
    w.sx = 16.0;
    w.sy = 16.0;
    w.make_packet(1);
    w.dx = from_int(x0);
    w.dy = from_int(y0 + 16);
    w.sx = 16.0;
    w.sy = tall;
    w.make_packet(2);
    w.dx = add(right, from_int(x0));
    w.dy = add(16.0, from_int(y0));
    w.sx = 16.0;
    w.sy = tall;
    w.make_packet(3);
    w.dx = from_int(x0);
    w.dy = from_int(y0 + 16 + tall as i32);
    w.sx = 16.0;
    w.sy = 16.0;
    w.make_packet(6);
    w.sx = mid;
    w.sy = 16.0;
    w.make_packet(5);
    w.sx = 16.0;
    w.sy = 16.0;
    w.make_packet(7);
}

/// `GtNewMenuDisp` (gcmn 0x0055f430): the keyword screen.
pub fn new_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let fp = if m.menu == 79 { 16 } else { 32 };
    let t = x.texts;
    let alpha = m.alpha;
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    m.win.dx = 39.0;
    m.win.dy = from_int(fp);
    disp_square(&mut m.win, 29, 3, None);
    set_type(&mut m.win, 1);
    let l = m.lists[i].clone();
    if m.exception_disp != 4 {
        let s3 = fp + 112;
        m.win.dx = 39.0;
        m.win.dy = from_int(s3);
        if l.y < l.my {
            disp_square_sb(&mut m.win, 8, 8, i32::from(l.dy), i32::from(l.my), Some(&t.gt_word));
        } else {
            disp_square(&mut m.win, 8, 8, Some(&t.gt_word));
        }
        set_clm(&mut m.kanji, 16, 0, 0, 1);
        let mut buf = Vec::new();
        let (mut n, mut row) = (0i32, 0i32);
        for id in part_words(x, i32::from(l.index)) {
            if n >= i32::from(l.dy) {
                if let Some(w) = t.words.word(id) {
                    str_cat(&mut buf, &w.text, 16);
                }
                let y = from_int(s3 + 16 + row * 20);
                if i32::from(l.select) == n {
                    m.gt_word = id;
                    let fr = x.frame_rate;
                    m.cursor.disp(&mut m.win, 39.0, y, 8, i32::from(l.select - l.dy), alpha, 3, fr);
                }
                let c = if m.reverse_head & (1 << row) != 0 { 0 } else { 7 };
                m.kanji.set_colour(c);
                m.kanji.set_alpha(m.kanji_alpha);
                m.kanji.dx = 53.0;
                m.kanji.dy = y;
                m.kanji.make_packet(row);
                row += 1;
            }
            n += 1;
            if row >= 8 {
                break;
            }
        }
        extract_menu(m, buf);
    } else {
        let g = m.generated.clone();
        let attr = field_attr(g.base.field_type(), g.base.weather());
        let piece = |k: i32| t.gt_new_str0.get(k.max(0) as usize).cloned().unwrap_or_default();
        let mut buf = piece(0);
        for k in 1..4 {
            buf.extend_from_slice(&piece(k));
        }
        buf.extend_from_slice(&piece(attr + 4));
        str_cat(&mut buf, &t.gt_area_info, 16);
        set_clm(&mut m.kanji, 16, 0, 0, 1);
        extract_menu(m, buf);
        let s3 = fp + 112;
        m.win.dx = 39.0;
        m.win.dy = from_int(s3);
        disp_square(&mut m.win, 8, 2, None);
        let ka = m.kanji_alpha;
        if l.y != 0 {
            for r in 0..i32::from(l.y) {
                let y = from_int(s3 + 16 + r * 20);
                if i32::from(l.select) == r {
                    let fr = x.frame_rate;
                    m.cursor.disp(&mut m.win, 39.0, y, 8, i32::from(l.select), alpha, 3, fr);
                }
                let c = if m.reverse_head & (1 << r) != 0 { 0 } else { 7 };
                m.kanji.set_colour(c);
                m.kanji.set_alpha(ka);
                m.kanji.dx = 53.0;
                m.kanji.dy = y;
                m.kanji.make_packet(r);
            }
        } else {
            m.kanji.set_colour(7);
            m.kanji.set_alpha(ka);
            m.kanji.dx = 53.0;
            m.kanji.dy = from_int(s3 + 16);
            m.kanji.make_packet(5);
        }
        let s3 = fp + 192;
        m.win.dx = 39.0;
        m.win.dy = from_int(s3);
        disp_square(&mut m.win, 8, 5, None);
        m.kanji.set_colour(7);
        m.kanji.set_alpha(ka);
        for (code, dx, dy) in [(2, 53.0, s3 + 16), (3, 53.0, s3 + 76), (4, 109.0, s3 + 96)] {
            m.kanji.dx = dx;
            m.kanji.dy = from_int(dy);
            m.kanji.make_packet(code);
        }
        font_type(&mut m.font, 1);
        m.font.shadow = true;
        m.font.set_colour(7);
        m.font.set_alpha(ka);
        m.font.dx = 95.0;
        m.font.dy = from_int(s3 + 36);
        m.font.make_str(&t.gt_lv);
        let gm = x.world.game;
        let mut lv = gm.area_level;
        if lv <= 0 || gm.area == 0 {
            let (kind, rank) = if g.event == 0 {
                (attr, g.base.enemy_ofs() + (g.base.area_level() - 1) * 25 + 10)
            } else {
                let e = t.words.event(g.event).map_or(UNSET, |e| e.enemy);
                let r = if e != UNSET { e } else { g.base.enemy_ofs() + (g.base.area_level() - 1) * 25 + 10 };
                (6, r.max(0))
            };
            lv = t.words.analyze_enemies(gm.server, kind, rank);
            x.world.game.area_level = lv;
            x.req.push(Request::AreaLevel(lv));
        }
        make_num(&mut m.font, 2, lv);
        let ic = &mut m.item_icon;
        ic.set_grid(14, 16, 14.0, 16.0, 0, 0x1400, 9);
        ic.set_colour(7);
        ic.set_alpha(ka);
        ic.dx = 92.0;
        ic.dy = from_int(s3 + 94);
        ic.make_packet(attr);
    }
    // The attributes' names and the parts' boxes.
    let s3 = fp + 112;
    m.win.dx = 206.0;
    m.win.dy = from_int(s3);
    disp_square(&mut m.win, 17, 9, Some(&t.gt_status));
    let mut buf = Vec::new();
    for k in 0..8 {
        str_cat(&mut buf, &t.gt_new_str1.get(k).cloned().unwrap_or_default(), 16);
    }
    m.setting_text[4] = buf;
    set_clm(&mut m.setting[4], 16, 0, 0, 1);
    let mut buf = Vec::new();
    for k in 8..12 {
        str_cat(&mut buf, &t.gt_new_str1.get(k).cloned().unwrap_or_default(), 16);
    }
    m.setting_text[5] = buf;
    set_clm(&mut m.setting[5], 16, 0, 0, 1);
    m.setting[4].set_colour(7);
    m.setting[4].set_alpha(alpha);
    for r in 0..8 {
        m.setting[4].dx = 220.0;
        m.setting[4].dy = from_int(s3 + 16 + r * 20);
        m.setting[4].make_packet(r);
    }
    let k5 = &mut m.setting[5];
    k5.set_colour(7);
    k5.set_alpha(alpha);
    k5.dx = 220.0;
    k5.dy = from_int(s3 + 176);
    k5.make_packet(0);
    k5.set_colour(17);
    k5.set_alpha(alpha);
    for (code, dx) in [(1, 104.0), (2, 216.0), (3, 328.0)] {
        k5.dx = dx;
        k5.dy = from_int(fp + 8);
        k5.make_packet(code);
    }
    let excp = m.exception_disp;
    let mut blink = alpha;
    if excp != 4 {
        let s2 = (i32::from(excp) - 1) * 112 + 92;
        let c = (x.count % 24) as i32;
        blink = if c < 12 { (c * 128) / 12 } else { ((24 - c) * 128) / 12 };
        if alpha < blink {
            blink = alpha;
        }
        frame_box(m, s2, fp + 27, 96.0, 16.0, 112.0, blink);
    }
    let server_sym = t.server_str.get(server(x) as usize).cloned().unwrap_or_default();
    setting_text(m, 3, server_sym, 1, 2, (64.0, from_int(fp + 44)), alpha);
    let s3 = fp + 44;
    let mut s2 = 64;
    let cur = m.gt_word;
    let temps = temp3(m);
    let part_id = |k: usize| if i32::from(excp) - 1 == k as i32 { cur } else { temps[k] };
    for k in 0..3usize {
        if i32::from(excp) <= k as i32 {
            break;
        }
        let Some(text) = word_text(x, part_id(k)) else { continue };
        let dx = if k == 0 { 104 } else { s2 + 112 };
        setting_text(m, k, text, -1, 0, (from_int(dx), from_int(s3 + 2)), alpha);
        s2 = dx;
    }
    for k in 0..3 {
        let a = if k == i32::from(excp) - 1 { blink } else { alpha };
        frame_box(m, k * 44 + 336, fp + 116, 10.0, 168.0, 26.0, a);
    }
    // The attribute marks: each word's set fields, coloured by rank. From
    // Mutation on the dungeon size (field 1) is set when not 0, and its
    // colour takes 0 as unset (MUT gcmn 0x00580668); Infection tests it
    // against 255 like the rest. The empty slots' word has it 0 there
    // (MUT gcmn 0x00580230), 255 on Infection.
    let later = t.words.volume > 1;
    let unset = |f: usize, v: i32| if later && f == 1 { v == 0 } else { v == UNSET };
    let mut fields = [UNSET; 9];
    if later {
        fields[1] = 0;
    }
    let dummy = Word { pri: 0, fields, ..Word::default() };
    let get = |id: i32| t.words.word(id).cloned().unwrap_or_else(|| dummy.clone());
    let ws: [Word; 3] = match excp {
        1 => [get(cur), dummy.clone(), dummy.clone()],
        2 => [get(m.temp[0]), get(cur), dummy.clone()],
        3 => [get(m.temp[0]), get(m.temp[1]), get(cur)],
        _ => [get(m.temp[0]), get(m.temp[1]), get(m.temp[2])],
    };
    let wa = &mut m.win_a;
    wa.set_grid(32, 24, 32.0, 24.0, 0, 3072, 2);
    let s3 = fp + 136;
    let s6 = alpha - (alpha >> 3);
    let s4 = alpha - (alpha >> 1);
    let pri = [ws[0].pri, ws[1].pri, ws[2].pri];
    for s in 0..3usize {
        let dx = from_int(s as i32 * 44 + 357);
        for f in 0..9usize {
            if unset(f, ws[s].fields[f]) {
                continue;
            }
            let dy = from_int(s3 + 20 * f as i32);
            let wa = &mut m.win_a;
            wa.set_colour(7);
            wa.set_alpha(s6);
            wa.sx = 32.0;
            wa.sy = 24.0;
            wa.cx = -16.0;
            wa.cy = -12.0;
            wa.dx = dx;
            wa.dy = dy;
            wa.sx = mul(wa.sx, 0.5);
            wa.sy = mul(wa.sy, 0.5);
            wa.cx = mul(wa.cx, 0.5);
            wa.cy = mul(wa.cy, 0.5);
            wa.make_packet(0);
            let fs = [ws[0].fields[f], ws[1].fields[f], ws[2].fields[f]].map(|v| if unset(f, v) { UNSET } else { v });
            let c = gt_new_colour(pri, fs, s, t.gt_new_colours);
            let r = m.rng.rand();
            let wa = &mut m.win_a;
            wa.set_colour(c);
            wa.set_alpha(s4);
            wa.sx = 32.0;
            wa.sy = 24.0;
            wa.cx = -16.0;
            wa.cy = -12.0;
            wa.dx = dx;
            wa.dy = dy;
            let k = add(1.2, div(from_int(r % 600), 1000.0));
            wa.sx = mul(wa.sx, k);
            wa.sy = mul(wa.sy, k);
            wa.cx = mul(wa.cx, k);
            wa.cy = mul(wa.cy, k);
            wa.make_packet(1);
        }
    }
}

/// The Word List's and history's rows, drawn at once at (x, 20 r + 112).
fn list_rows(m: &mut MenuCtrl, x: &mut Ctx, rows: Vec<(Vec<u8>, bool)>, text_x: f32) {
    let i = idx(m);
    let l = m.lists[i].clone();
    font_type(&mut m.font, 1);
    m.font.shadow = true;
    m.win.dx = 39.0;
    m.win.dy = 96.0;
    let title = x.texts.gt_word.clone();
    if l.y < l.my {
        disp_square_sb(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.dy), i32::from(l.my), Some(&title));
    } else {
        disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), Some(&title));
    }
    let alpha = m.alpha;
    for (n, (text, marked)) in rows.into_iter().enumerate() {
        let n = n as i32;
        if n < i32::from(l.dy) {
            continue;
        }
        let s2 = n - i32::from(l.dy);
        if s2 >= i32::from(l.y) {
            break;
        }
        let fp = s2 * 20 + 112;
        if i32::from(l.select) == n {
            let fr = x.frame_rate;
            m.cursor.disp(&mut m.win, 39.0, from_int(fp), i32::from(l.x), i32::from(l.select), alpha, 3, fr);
        }
        setting_text(m, s2 as usize, text, -1, 0, (text_x, from_int(fp)), alpha);
        if marked {
            set_type(&mut m.win, 1);
            m.win.set_colour(7);
            m.win.set_alpha(alpha);
            m.win.dx = 53.0;
            m.win.dy = from_int(fp);
            m.win.make_packet(28);
        }
    }
}

/// `GtListMenuDisp` (gcmn 0x00563010).
pub fn list_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.exception_disp == 4 {
        return new_menu_disp(m, x);
    }
    let t = x.texts;
    let sym = t.server_str.get(server(x) as usize).cloned().unwrap_or_default();
    let mark = GATE_LIST_MARK + 20 * server(x) as usize;
    let rows: Vec<(Vec<u8>, bool)> = order_list(x)
        .into_iter()
        .map(|code| {
            let mut s = t.gt_space.clone();
            s.extend_from_slice(&sym);
            s.extend_from_slice(&t.gt_space);
            if let Some(ws) = t.words.event(i32::from(code)).and_then(|e| e.words.clone()) {
                s.extend_from_slice(&ws[0]);
                s.extend_from_slice(&t.gt_space);
                s.extend_from_slice(&ws[1]);
                s.extend_from_slice(&t.gt_space);
                s.extend_from_slice(&ws[2]);
            }
            s.extend_from_slice(&t.gt_spaces16);
            (s, bit(x, mark, i32::from(code)))
        })
        .collect();
    list_rows(m, x, rows, 67.0);
}

/// `GtRecordMenuDisp` (gcmn 0x00564080).
pub fn record_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.exception_disp == 4 {
        return new_menu_disp(m, x);
    }
    let t = x.texts;
    let sym = t.server_str.get(server(x) as usize).cloned().unwrap_or_default();
    let rows: Vec<(Vec<u8>, bool)> = records(x)
        .into_iter()
        .map(|v| {
            let mut s = sym.clone();
            for id in [v / 1_000_000, (v % 1_000_000) / 1000, v % 1000] {
                s.extend_from_slice(&t.gt_space);
                if let Some(w) = t.words.word(id) {
                    s.extend_from_slice(&w.text);
                }
            }
            (s, false)
        })
        .collect();
    list_rows(m, x, rows, 53.0);
}

/// `GtTownMenuDisp` (gcmn 0x00564df0): a mark by each town whose server
/// holds a marked story area.
pub fn town_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let ts = towns(x);
    let alpha = m.alpha;
    for (r, &town) in ts.iter().enumerate() {
        let base = GATE_LIST_MARK + 20 * town.clamp(0, 4) as usize;
        if (0..160).any(|k| bit(x, base, k)) {
            let w = &mut m.win_pr;
            set_type(w, 1);
            w.set_colour(7);
            w.set_alpha(alpha);
            w.dx = 53.0;
            w.dy = from_int(r as i32 * 20 + 112);
            w.make_packet(28);
        }
    }
}

// --- The tutorial's gate (78, 79; event 2) ------------------------------------------

/// `ccCheckTargetTypeId(type, id)` (gcmn 0x005199e0) for an object: the
/// first on `cmndObjRoot`'s chain of that type and id.
fn object_of(x: &Ctx, types: u32, id: i16) -> Option<crate::world::CharInfo> {
    x.world.obj_chain.iter().find(|c| c.is(types) && c.id == id).cloned()
}

/// A tutorial record's text as `ChangeInfo(sep 0, sep 1, sep 2, 0)` (or
/// `OpenInfo`) at (39, 444), with its voice (event 2).
fn tutorial_info(m: &mut MenuCtrl, x: &mut Ctx, n: usize, open: bool) {
    let r = crate::menus::tutorial::record(x, 2, n).unwrap_or_default();
    let l: Vec<Vec<u8>> = (0..3).map(|k| r.lines.get(k).map(|t| t.as_bytes().to_vec()).unwrap_or_default()).collect();
    let names = x.save.names();
    let lines = [Some(&l[0][..]), Some(&l[1][..]), Some(&l[2][..]), None];
    if open {
        m.msg.open_info(lines, &names);
    } else {
        m.msg.change_info(lines, &names);
    }
    m.msg.set_pos(39.0, 444.0);
    x.req.push(Request::VoiceRequest { grp: 2, msg: n as i32 });
}

/// `GateMenuT` (gcmn 0x00567e50, menu 78): Orca at the gate, messages 30 to
/// 33; only New Keyword (row 1) goes on (to 79), the rest buzz.
pub fn gate_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    use crate::menus::tutorial::{change, open};
    match m.proccess {
        0 => {
            let Some(gate) = object_of(x, 0x2000, 16) else {
                m.menu_next = -1;
                m.menu_status = 3;
                m.bg_status = 3;
                let woke = m.breathe_close(x);
                return Flow::Breathed(Cont::close(woke));
            };
            x.req.push(Request::TargetFix(true));
            x.change_target(Some(&gate));
            m.menu_status = 0;
            open(m, x, 2, 30);
            m.proccess += 1;
        }
        1 | 3 => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r != 0 {
                let n = if m.proccess == 1 { 31 } else { 33 };
                change(m, x, 2, n);
                m.proccess += 1;
            }
        }
        2 => {
            if x.pushed_ok() {
                x.req.push(Request::VoiceStop);
                x.se(crate::ctrl::SE_OPEN);
                if let Some(t) = x.target.clone() {
                    x.req.push(Request::Affect { target: t.handle, kind: 11 });
                }
                x.req.push(Request::Flash);
                m.menu_status = 1;
                let rows = item_rows(m);
                extract_menu(m, rows);
                m.reverse_head |= 29;
                m.target_forbid = 0;
                change(m, x, 2, 32);
                m.proccess += 1;
            }
        }
        4 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                if m.list().select == 1 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    x.change_target(None);
                    if m.still == 0 {
                        x.req.push(Request::SleepAll);
                        m.still = 1;
                        x.req.push(Request::Still(true));
                        x.req.push(Request::KeepLayers);
                    }
                    m.msg.close();
                    return m.change_menu(x);
                }
                x.se(crate::ctrl::SE_BUZZ);
            }
        }
        _ => {}
    }
    Flow::Done
}

/// A part's list in the tutorial: its words counted, the cursor on the
/// word kept (`with_select`), all rows but the first greyed.
fn tutorial_part(m: &mut MenuCtrl, x: &Ctx, part: usize, with_select: bool) {
    let i = idx(m);
    m.lists[i].my = 0;
    for w in part_words(x, part as i32) {
        if with_select && m.temp[part] == w {
            m.lists[i].select = m.lists[i].my;
        }
        m.lists[i].my += 1;
    }
}

/// The tutorial's part list: OK only on the first row; the rows greyed
/// but the first (all while scrolled).
fn tutorial_pick(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    m.select_scr(0, 0, 0, x);
    if x.pushed_ok() {
        if m.lists[i].select == 0 {
            x.req.push(Request::VoiceStop);
            x.se(SE_OK);
            pick_part(m, x);
            m.proccess += 1;
        } else {
            x.se(crate::ctrl::SE_BUZZ);
        }
    }
    m.reverse_head = if m.lists[i].dy != 0 { 255 } else { 254 };
}

/// `GtNewMenuT` (gcmn 0x00568350, menu 79): the keyword screen with Orca's
/// lessons (messages 35 to 56), the first word of each part only, and
/// the party's leave; the event does the rest.
pub fn new_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            m.lists[i].disp = 4;
            m.temp[..6].fill(0);
            m.bg_status = 1;
            m.menu_status = 1;
            m.exception_disp = 1;
            let l = &mut m.lists[i];
            l.select = 0;
            l.index = 0;
            l.y = 8;
            l.dy = 0;
            tutorial_part(m, x, 0, false);
            m.reverse_head = 254;
            m.wait_count = 0;
            tutorial_info(m, x, 35, true);
            m.proccess += 1;
        }
        1 | 9 => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r != 0 {
                let (last, base) = if m.proccess == 1 { (4, 35) } else { (12, 43) };
                let w = m.wait_count;
                m.wait_count += 1;
                if w >= last {
                    m.proccess += 1;
                } else {
                    let n = base + m.wait_count as usize;
                    tutorial_info(m, x, n, false);
                }
            }
        }
        2 => {
            tutorial_info(m, x, 40, false);
            tutorial_part(m, x, 0, true);
            m.proccess += 1;
        }
        3 | 5 | 7 => tutorial_pick(m, x),
        4 | 6 => {
            let part = if m.proccess == 4 { 1 } else { 2 };
            tutorial_info(m, x, 40 + part, false);
            m.exception_disp = part as i16 + 1;
            let l = &mut m.lists[i];
            l.index = part as i16;
            l.y = 8;
            l.dy = m.temp[3 + part] as i16;
            l.my = 0;
            l.select = 0;
            m.reverse_head = 254;
            tutorial_part(m, x, part, true);
            m.proccess += 1;
        }
        8 => {
            m.exception_disp = 4;
            m.lists[i].y = 2;
            m.lists[i].select = 0;
            m.reverse_head = 2;
            generate(m, x, temp3(m));
            m.wait_count = 0;
            tutorial_info(m, x, 43, false);
            m.proccess += 1;
        }
        10 => {
            tutorial_info(m, x, 56, false);
            m.wait_count = 0;
            m.proccess += 1;
        }
        11 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                m.proccess += 1;
            }
        }
        12 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                if m.lists[i].select == 0 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    m.proccess += 1;
                } else {
                    x.se(crate::ctrl::SE_BUZZ);
                }
            }
        }
        13 => {
            m.msg.close();
            generate(m, x, temp3(m));
            m.wait_count = 0;
            m.menu_status = 3;
            m.bg_status = 3;
            return wake_breath(m, x, GateAfter::Proccess(14));
        }
        14 => {
            let gone = leave_members(m, x);
            if gone {
                m.menu_next = -1;
                m.menu_status = 3;
                m.bg_status = 3;
                let woke = m.breathe_close(x);
                return Flow::Breathed(Cont { woke, cursors: false, after: After::Gate(GateAfter::TutorialDone) });
            }
        }
        _ => {}
    }
    Flow::Done
}

/// The leave's transfers alone (the tutorial asks for no area change):
/// true at frame 211.
fn leave_members(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    m.wait_count += 1;
    for k in 0..3 {
        if i32::from(m.wait_count) == 30 * k as i32 + 5
            && let Some(c) = &x.world.party[k]
        {
            x.req.push(Request::TransferOut(c.handle));
        }
    }
    let w = m.wait_count;
    m.wait_count += 1;
    w >= 211
}
