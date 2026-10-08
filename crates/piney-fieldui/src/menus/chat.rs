//! `ChatMenu` (gcmn 0x00528f20, menu 3) and `ChatMenuDisp` (0x00529fa0,
//! menus 3 and 83): CHAT, the party's orders (the square button): two
//! pages in town, three elsewhere (Skill Usage, Strategy, Members). The
//! first pass writes -1 at `equipSpcNum + 2 k` and the members' settings at
//! `chatMember + 12 k`; the strides differ, so the cancel tells no member
//! anything unless that pass was skipped or a member's Change Equipment
//! wrote its mark. The port keeps the same 36 bytes
//! ([`MenuCtrl::chat_mem`]). The steps are in docs/engine/field-ui.md.

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Draw, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::check_operate;
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::spr::{Obj, set_clm};

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// Whether the Sprite Ocarina's order shows: in a dungeon, not floors 8 or
/// 9, not in battle, operation 14 allowed.
fn ocarina(x: &mut Ctx) -> bool {
    let g = x.world.game;
    g.area == 2 && g.dungeon_type != 8 && g.dungeon_type != 9 && g.in_battle == 0 && check_operate(x, 14, 1)
}

/// The shut every order ends in (`menuNext = -1` ... `firstTime = 0`),
/// the help line after.
pub(crate) fn shut(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont { woke, cursors: false, after: After::ChatHelp })
}

/// Cancel: each member whose kept settings differ is told
/// (`ChangeEquipReport`).
fn report_changes(m: &MenuCtrl, x: &mut Ctx) {
    for k in 1..3 {
        let mark = m.chat_mem[3 + k];
        if mark < 0 {
            continue;
        }
        let Some(c) = x.world.party[k].clone() else { continue };
        if mark != 0 {
            x.req.push(Request::ChangeEquipReport { member: c.handle, n: mark });
            continue;
        }
        let rec = |i: usize| m.chat_mem.get(6 * k + i).copied().unwrap_or(0);
        if rec(4) != c.chat[4]
            || rec(0) != c.chat[0]
            || rec(1) != c.chat[1]
            || rec(2) != c.chat[2]
            || rec(3) != c.chat[3]
        {
            x.req.push(Request::ChangeEquipReport { member: c.handle, n: 0 });
        }
    }
}

/// Every member (slots 1 and 2) gets the order.
fn order_all(x: &mut Ctx, cmd: i32) {
    for k in 1..3 {
        if let Some(c) = &x.world.party[k] {
            x.req.push(Request::ChatCmd { member: c.handle, cmd });
        }
    }
}

/// OK on the members' page: the `select`th member's own orders (71).
fn to_member(m: &mut MenuCtrl, x: &Ctx) -> Flow {
    let sel = m.list().select;
    let mut n = 0;
    for k in 1..3 {
        if x.world.party[k].is_some() {
            if n == sel {
                m.chat_mem[0] = sel;
                m.lists[71].select = 0;
                m.change_menu_to(71);
                return Flow::Done;
            }
            n += 1;
        }
    }
    Flow::Done
}

/// The members' page's rows: 1 with one member, else 2.
fn members_rows(m: &mut MenuCtrl, x: &Ctx) {
    let i = idx(m);
    let l = &mut m.lists[i];
    l.my = if x.world.party_num() < 3 { 1 } else { 2 };
    if l.select >= l.my {
        l.select = l.my - 1;
    }
}

pub fn chat_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let flow = match m.proccess {
        0 => open(m, x),
        10 => {
            let old = m.lists[i].page;
            m.select(2, 0, true, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            match m.lists[i].page {
                1 => members_rows(m, x),
                0 => m.lists[i].my = m.lists[i].y,
                _ => {}
            }
            keys(m, x, true)
        }
        20 => {
            let old = m.lists[i].page;
            m.select_scr(3, 0, 0, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            match m.lists[i].page {
                2 => members_rows(m, x),
                1 => {
                    let l = &mut m.lists[i];
                    l.my = 4;
                    if l.select >= l.my {
                        l.select = l.my - 1;
                    }
                }
                0 => m.lists[i].my = m.lists[i].y,
                _ => {}
            }
            keys(m, x, false)
        }
        50 => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r != 0 {
                m.msg.close();
                m.menu_next = -1;
                m.menu_status = 3;
                m.bg_status = 3;
                let woke = m.breathe_close(x);
                return Flow::Breathed(Cont::close(woke));
            }
            Flow::Done
        }
        _ => Flow::Done,
    };
    if let Flow::Breathed(_) = flow {
        return flow;
    }
    help(m, x);
    flow
}

fn open(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    x.req.push(Request::TargetFix(true));
    x.change_target(None);
    m.bg_status = 1;
    let num = x.world.party_num();
    m.lists[i].y = num;
    if num < 2 {
        m.menu_status = 3;
        m.lists[i].disp = 4;
        let w = x.texts.party_warn.get(1).cloned().unwrap_or_default();
        let names = x.save.names();
        m.msg.open_info([Some(&w[0]), Some(&w[1]), None, None], &names);
        m.proccess = 50;
        return Flow::Done;
    }
    m.menu_status = 1;
    let l = &mut m.lists[i];
    l.disp = 9;
    l.x = 12;
    if x.world.game.area == 0 {
        l.page_num = 2;
        l.y = 2;
        l.my = 2;
        if l.select > l.my - 1 {
            l.select = l.my - 1;
        }
        if l.page > l.page_num - 1 {
            l.page = l.page_num - 1;
        }
        l.dy = 0;
        m.exception_disp = 1;
        m.proccess = 10;
    } else {
        l.page_num = 3;
        let rows = if ocarina(x) { 8 } else { 7 };
        let l = &mut m.lists[i];
        l.y = rows;
        l.my = rows;
        if l.select > l.my - 1 {
            l.select = l.my - 1;
        }
        if l.page > l.page_num - 1 {
            l.page = l.page_num - 1;
        }
        m.exception_disp = 2;
        m.proccess = 20;
    }
    if m.first_time != 0 {
        for k in 1..3 {
            m.chat_mem[3 + k] = -1;
            if let Some(c) = &x.world.party[k] {
                for (j, &v) in c.chat.iter().enumerate() {
                    if let Some(s) = m.chat_mem.get_mut(6 * k + j) {
                        *s = v;
                    }
                }
            }
        }
    }
    Flow::Done
}

fn keys(m: &mut MenuCtrl, x: &mut Ctx, town: bool) -> Flow {
    let i = idx(m);
    let cancel = x.pushed_cancel();
    let ok = !cancel && x.pushed_ok();
    if cancel {
        x.se(SE_BACK);
        report_changes(m, x);
        return shut(m, x);
    }
    if !ok {
        return Flow::Done;
    }
    x.se(SE_OK);
    let page = m.lists[i].page;
    let sel = m.lists[i].select.max(0) as usize;
    let t = x.texts;
    let piece = |v: &Vec<Vec<u8>>, k: usize| v.get(k).cloned().unwrap_or_default();
    match (town, page) {
        (true, 0) => {
            let cmd = t.town_chat_act.get(sel).copied().unwrap_or(0);
            order_all(x, cmd);
            crate::chat_msg::open_player(m, x, piece(&t.chat_str, sel + 16));
            shut(m, x)
        }
        (true, 1) | (false, 2) => to_member(m, x),
        (false, 0) => {
            let cmd = t.bt_chat_act.get(sel).copied().unwrap_or(0);
            order_all(x, cmd);
            let mut line = piece(&t.chat_action, 5);
            line.extend_from_slice(&piece(&t.chat_str, sel));
            crate::chat_msg::open_player(m, x, line);
            shut(m, x)
        }
        (false, 1) if sel < 4 => {
            let cmd = t.bt_chat_act.get(8 + sel).copied().unwrap_or(0);
            order_all(x, cmd);
            let mut line = piece(&t.chat_action, 5);
            line.extend_from_slice(&piece(&t.chat_str, sel + 8));
            line.extend_from_slice(&piece(&t.chat_action, 6));
            crate::chat_msg::open_player(m, x, line);
            x.save.save.set_u8(TACTICS, cmd as u8);
            shut(m, x)
        }
        _ => Flow::Done,
    }
}

/// `saveData.tactics` (+0x6773): the party's strategy.
pub const TACTICS: usize = 0x6773;

/// The order's help while `proccess` is below 50 (0x00529e58).
pub fn help(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.proccess >= 50 {
        return;
    }
    let l = m.list();
    let (page, sel) = (l.page, i32::from(l.select));
    let k = match (m.exception_disp, page) {
        (2, 0) => sel,
        (2, 1) => sel + 8,
        (2, 2) => 16,
        (1, 0) => sel + 14,
        (1, 1) => 16,
        _ => 0,
    };
    let Some(h) = x.texts.chat_help.get(k.max(0) as usize).cloned() else { return };
    let names = x.save.names();
    m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
}

/// `ccKanji::Disp(settingKanji[s], str, -1, 1, 1)`: the row drawn at once,
/// its queued packets not sent.
fn row_text(m: &mut MenuCtrl, s: usize, text: Vec<u8>, colour: usize) {
    let k = &mut m.setting[s];
    k.set_colour(colour);
    k.set_alpha(m.kanji_alpha);
    k.dx = 53.0;
    k.dy = piney_desktop::eef::from_int(s as i32 * 20 + 112);
    let rgba = [k.rgb[0], k.rgb[1], k.rgb[2], m.kanji_alpha as u8];
    let (dx, dy) = (k.dx, k.dy);
    m.draws.push(Draw::Text { obj: Obj::Setting(s as u8), text, dx, dy, rgba, count: -1, kt: 0 });
    m.setting_send[s] = false;
}

/// A member's name as a row of its own kanji.
fn member_row(m: &mut MenuCtrl, s: usize, name: &[u8]) {
    let mut buf = Vec::new();
    str_cat(&mut buf, name, 16);
    m.setting_text[s] = buf;
    let c = if m.reverse_head & (1 << s) != 0 { 0 } else { 7 };
    let k = &mut m.setting[s];
    k.set_colour(c);
    k.set_alpha(m.kanji_alpha);
    set_clm(k, 16, 0, 0, 1);
    k.dx = 53.0;
    k.dy = piney_desktop::eef::from_int(s as i32 * 20 + 112);
    k.make_packet(0);
}

/// `ChatMenuDisp`.
pub fn chat_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.kanji_alpha <= 0 {
        return;
    }
    let i = idx(m);
    let page = i32::from(m.lists[i].page);
    let t = x.texts;
    let piece = |v: &Vec<Vec<u8>>, k: i32| v.get(k.max(0) as usize).cloned().unwrap_or_default();
    let grey = |m: &MenuCtrl, bit: i32| if m.reverse_head & (1 << bit) != 0 { 0 } else { 7 };
    let members: Vec<Vec<u8>> = x.world.party[1..].iter().flatten().map(|c| c.name.clone()).collect();
    match m.exception_disp {
        1 => {
            extract_menu(m, piece(&t.chat_tags, page + 1));
            match page {
                0 => {
                    for s in 0..2 {
                        let c = grey(m, s);
                        row_text(m, s as usize, piece(&t.chat_str, s + 16), c);
                    }
                }
                1 => {
                    for (s, name) in members.iter().enumerate() {
                        member_row(m, s, name);
                    }
                }
                _ => {}
            }
        }
        2 => {
            extract_menu(m, piece(&t.chat_tags, page));
            match page {
                0 => {
                    let n = if ocarina(x) { 8 } else { 7 };
                    for s in 0..n {
                        let c = grey(m, s);
                        row_text(m, s as usize, piece(&t.chat_str, s), c);
                    }
                }
                1 => {
                    let tactics = i32::from(x.save.save.u8(TACTICS) as i8);
                    for (s, k) in (8..12).enumerate() {
                        let c = if m.reverse_head & (1 << k) != 0 {
                            0
                        } else if t.bt_chat_act.get(k as usize).copied() == Some(tactics) {
                            6
                        } else {
                            7
                        };
                        row_text(m, s, piece(&t.chat_str, k), c);
                    }
                }
                2 => {
                    for (s, name) in members.iter().enumerate() {
                        member_row(m, s, name);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}
