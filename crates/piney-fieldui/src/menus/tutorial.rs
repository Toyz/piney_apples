//! The tutorial menus the opening's events open (`menu num=75` ...): the
//! real menus' lists with every row but one greyed, and Orca's lines from
//! the event's own message table (`evMsgTblM1[2]`, or `evMsgTblM1p` in
//! Parody Mode) opened by the menu itself: `PersonalMenuT` (75,
//! 0x005670c0), `PartyMenuT` (76, 0x00567410), `PartyInMenuT` (77,
//! 0x00567660). The lines and rows are in docs/engine/field-ui.md.

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BUZZ, SE_OK, SE_OPEN};
use crate::menus::gate::GateAfter;
use crate::menus::system::{extract_menu, item_rows, str_cat};

/// An event record the tutorials show: event 2's message `n`.
pub fn record(x: &Ctx, event: u16, n: usize) -> Option<piney_event::ir::Message> {
    let tbl = if x.save.parody() { &x.texts.tutorial_parody } else { &x.texts.tutorial };
    tbl.get(&event).and_then(|v| v.get(n)).cloned()
}

/// `ccMsg->Open(record, record->name, grp, msg)` (the ccEvMsgData form).
pub fn open(m: &mut MenuCtrl, x: &mut Ctx, event: u16, n: usize) {
    let names = x.save.names();
    let r = record(x, event, n).unwrap_or_default();
    let lines: Vec<&[u8]> = r.lines.iter().map(|l| l.as_bytes()).collect();
    m.msg.open_record(r.mode, r.name.as_ref().map(|t| t.as_bytes()), &lines, &names);
    x.req.push(Request::VoiceRequest { grp: i32::from(event), msg: n as i32 });
    m.note(|| format!("[\"msg_open\",{event},{n}]"));
}

/// `ccMsg->Change(record, record->name, grp, msg)`.
pub fn change(m: &mut MenuCtrl, x: &mut Ctx, event: u16, n: usize) {
    let names = x.save.names();
    let r = record(x, event, n).unwrap_or_default();
    let lines: Vec<&[u8]> = r.lines.iter().map(|l| l.as_bytes()).collect();
    m.msg.change_record(r.mode, r.name.as_ref().map(|t| t.as_bytes()), &lines, &names);
    x.req.push(Request::VoiceRequest { grp: i32::from(event), msg: n as i32 });
    m.note(|| format!("[\"msg_change\",{event},{n}]"));
}

/// `PersonalMenuT` (menu 75).
pub fn personal_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            let rows = item_rows(m);
            extract_menu(m, rows);
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.menu_status = 0;
            open(m, x, 2, 12);
            m.proccess += 1;
        }
        1 => {
            if x.pad.push.bits() & x.assign(1) != 0 {
                x.se(SE_OPEN);
                m.bg_status = 1;
                m.menu_status = 1;
                still_on(m, x);
                m.reverse_head = 191;
                x.req.push(Request::VoiceStop);
                change(m, x, 2, 13);
                m.proccess += 1;
            }
        }
        2 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                if m.list().select == 6 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    m.msg.close();
                    return m.change_menu(x);
                }
                x.se(SE_BUZZ);
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `PartyMenuT` (menu 76).
pub fn party_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            let rows = item_rows(m);
            extract_menu(m, rows);
            m.reverse_head = 6;
            open(m, x, 2, 14);
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                m.proccess += 1;
            }
        }
        2 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                if m.list().select == 0 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    m.msg.close();
                    return m.change_menu(x);
                }
                x.se(SE_BUZZ);
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `PartyInMenuT` (gcmn 0x00567660, menu 77): Orca's lines 15 to 18 over
/// the members who can join, the OK / Cancel question (only OK goes on),
/// the call ("Calling party member."), the member added
/// ([`Request::AddMember`]), line 19, then the panels back in and out.
pub fn party_in_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = m.menu.clamp(0, 88) as usize;
    match m.proccess {
        0 => {
            let l = &mut m.lists[i];
            l.x = 9;
            l.my = 1;
            l.y = 1;
            l.dy = 0;
            l.index = 0;
            l.select = 0;
            l.disp = 7;
            m.menu_status = 1;
            m.exception_disp = 1;
            m.wait_count = 0;
            m.face_num = 2;
            open(m, x, 2, 15);
            m.proccess += 1;
        }
        1 => {
            if check(m, x) {
                let w = m.wait_count;
                m.wait_count += 1;
                change(m, x, 2, (m.wait_count + 15) as usize);
                if w >= 2 {
                    m.proccess += 1;
                }
            }
        }
        2 => {
            m.select_scr(0, 0, 0, x);
            if x.pushed_ok() {
                x.req.push(Request::VoiceStop);
                m.msg.close();
                x.se(SE_OK);
                m.menu_status = 3;
                m.panel_status = 3;
                m.lists[i].index = m.lists[i].select;
                m.proccess += 1;
            }
        }
        3 => {
            if m.menu_status == 0 {
                m.exception_disp = 0;
                m.proccess += 1;
            }
        }
        4 => {
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.disp = 11;
            l.x = 6;
            l.y = 2;
            l.select = 0;
            m.reverse_head = 2;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            let [a, b] = x.texts.party_in_info[0].clone();
            let mut info = a;
            info.extend_from_slice(&crate::disp::member_name(x, i32::from(m.face_num)));
            info.extend_from_slice(&b);
            let names = x.save.names();
            m.msg.open_info([Some(&info), None, None, None], &names);
            m.cursor.init(true);
            m.cursor_pr.init(true);
            m.proccess += 1;
        }
        5 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                if m.lists[i].select == 1 {
                    x.se(SE_BUZZ);
                } else {
                    m.msg.close();
                    x.se(SE_OK);
                    m.menu_status = 3;
                    m.proccess = 10;
                }
            }
        }
        10 => {
            if m.menu_status == 0 {
                m.lists[i].x = 9;
                let [a, b] = x.texts.party_in_info[1].clone();
                let names = x.save.names();
                m.msg.open_info([Some(&a), Some(&b), None, None], &names);
                m.wait_count = 0;
                // ccStartThread(ccThPartyAdd): the member joins as the
                // thread runs, before the menu's next step.
                x.req.push(Request::AddMember(m.face_num));
                m.proccess += 1;
            }
        }
        11 => {
            let face = i32::from(m.face_num);
            let slot = x.world.member_slot(face);
            if slot >= 0 {
                m.face_tex[slot as usize] = crate::menus::party::menu_face_row(x, face);
            }
            m.proccess += 1;
            if m.wait_count < 30 {
                m.wait_count += 1;
            }
        }
        12 => {
            if m.wait_count == 30 {
                m.msg.close();
            } else if m.wait_count >= 41 {
                open(m, x, 2, 19);
                m.proccess += 1;
            }
            m.wait_count += 1;
        }
        13 => {
            if check(m, x) {
                m.bg_status = 3;
                if m.still == 1 {
                    return crate::menus::gate::wake_breath(m, x, crate::menus::gate::GateAfter::ProccessWait(14));
                }
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        14 | 16 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                if m.proccess == 16 {
                    m.menu_next = -1;
                    m.menu_status = 3;
                    m.bg_status = 3;
                    let woke = m.breathe_close(x);
                    return Flow::Breathed(Cont::close(woke));
                }
                m.wait_count = 0;
                m.panel_status = 1;
                m.proccess += 1;
            }
        }
        15 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 61 {
                m.wait_count = 0;
                m.panel_status = 3;
                m.proccess += 1;
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `ccMsg->Check(1)`: the event line's page read (true when done).
pub fn check(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let ok = x.save.ok();
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
    crate::menus::system::push_msg_requests(x, req);
    r != 0
}

/// The tutorials' own sleep: every task but the menu's asleep, the other
/// layers kept.
fn still_on(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.still == 0 {
        x.req.push(Request::SleepAll);
        m.still = 1;
        x.req.push(Request::Still(true));
        x.req.push(Request::KeepLayers);
    }
}

/// The close of TargetMenuT and ChatMenuT: CloseMenu, then the panels out.
fn close_panels(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont { woke, cursors: false, after: After::Gate(GateAfter::TutorialDone) })
}

// --- The skill and chat lessons (80 - 83; event 3) ------------------------------

/// `PersonalMenuTS` (gcmn 0x00569480, menu 80): line 34 ("press
/// triangle"); the button opens the list; only Skills (row 0) goes on.
pub fn personal_menu_ts(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            let rows = item_rows(m);
            extract_menu(m, rows);
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.menu_status = 0;
            m.reverse_head = 254;
            open(m, x, 3, 34);
            m.proccess += 1;
        }
        1 => {
            if x.pad.push.bits() & x.assign(1) != 0 {
                x.se(SE_OPEN);
                m.bg_status = 1;
                m.menu_status = 1;
                still_on(m, x);
                m.proccess += 1;
            }
        }
        2 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                if m.list().select == 0 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    m.msg.close();
                    return m.change_menu(x);
                }
                x.se(SE_BUZZ);
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `SkillMenuT` (gcmn 0x005697b0, menu 81): Kite's skills with line 35;
/// only the third page's first skill goes on (to 82).
pub fn skill_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = m.menu.clamp(0, 88) as usize;
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.exception_disp = 1;
            let l = &mut m.lists[i];
            l.page_num = 5;
            l.select = 0;
            l.page = 0;
            m.bg_status = 1;
            open(m, x, 3, 35);
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                m.proccess += 1;
            }
        }
        2 => {
            let old = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            crate::menus::skill::set_skill_list_fit(m, x, 0);
            if x.pushed_ok() {
                if m.lists[i].page == 2 && m.lists[i].select == 0 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    m.msg.close();
                    m.change_menu_to(82);
                } else {
                    x.se(SE_BUZZ);
                }
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `TargetMenuT` (gcmn 0x00569a00, menu 82): the party as targets with
/// line 36; OK uses skill 150 on the one chosen.
pub fn target_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = m.menu.clamp(0, 88) as usize;
    let mut buf = Vec::new();
    let mut cands = Vec::new();
    for c in x.world.party.iter().flatten() {
        str_cat(&mut buf, &c.name, 16);
        cands.push(c.clone());
    }
    m.lists[i].y = cands.len() as i16;
    extract_menu(m, buf);
    let pick = |m: &MenuCtrl| cands.get(m.lists[i].select.max(0) as usize).cloned();
    match m.proccess {
        0 => {
            m.bg_status = 3;
            m.lists[i].disp = 6;
            m.lists[i].select = 0;
            x.req.push(Request::TargetFix(true));
            let c = pick(m);
            x.change_target(c.as_ref());
            m.target_forbid = 0;
            open(m, x, 3, 36);
            m.cursor.init(true);
            m.cursor_pr.init(true);
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                m.proccess += 1;
            }
        }
        2 => {
            m.select(0, 0, false, x);
            x.req.push(Request::TargetFix(true));
            let c = pick(m);
            x.change_target(c.as_ref());
            if x.pushed_ok() {
                x.req.push(Request::VoiceStop);
                m.msg.close();
                m.proccess += 1;
            }
        }
        3 => {
            let target = x.target.as_ref().map_or(0, |t| t.handle);
            x.req.push(Request::Skill { target, skill: 150 });
            return close_panels(m, x);
        }
        _ => {}
    }
    Flow::Done
}

/// `ChatMenuT` (gcmn 0x00569d30, menu 83): line 45 ("press square"); the
/// button opens CHAT with line 46; only the first page's second row (the
/// heal) goes on: the member's order, and the menu shuts.
pub fn chat_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = m.menu.clamp(0, 88) as usize;
    match m.proccess {
        0 => {
            open(m, x, 3, 45);
            m.proccess += 1;
        }
        1 => {
            if x.pad.push.bits() & x.assign(2) != 0 {
                x.req.push(Request::VoiceStop);
                x.se(SE_OPEN);
                still_on(m, x);
                x.req.push(Request::TargetFix(true));
                m.target_forbid = 0;
                x.change_target(None);
                m.bg_status = 1;
                m.menu_status = 1;
                let l = &mut m.lists[i];
                l.disp = 9;
                l.select = 0;
                l.page = 0;
                l.page_num = 3;
                l.x = 12;
                l.y = 7;
                l.my = 7;
                m.exception_disp = 2;
                m.reverse_head = 4093;
                change(m, x, 3, 46);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        2 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                m.proccess = 20;
            }
        }
        20 => {
            let old = m.lists[i].page;
            m.select_scr(3, 0, 0, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let num = x.world.party_num();
            let l = &mut m.lists[i];
            match l.page {
                0 => {
                    m.reverse_head = 4093;
                    l.my = 7;
                }
                1 | 2 => {
                    m.reverse_head = 4095;
                    l.my = if l.page == 1 {
                        4
                    } else if num < 3 {
                        1
                    } else {
                        2
                    };
                    if l.select >= l.my {
                        l.select = l.my - 1;
                    }
                }
                _ => {}
            }
            if x.pushed_ok() {
                if m.lists[i].page == 0 && m.lists[i].select == 1 {
                    x.req.push(Request::VoiceStop);
                    x.se(SE_OK);
                    if let Some(c) = &x.world.party[1] {
                        x.req.push(Request::ManualOff(c.handle));
                    }
                    let sel = m.lists[i].select.max(0) as usize;
                    let text = x.texts.chat_str.get(sel).cloned().unwrap_or_default();
                    crate::chat_msg::open_player(m, x, text);
                    x.change_target(None);
                    m.msg.close();
                    m.menu_status = 3;
                    m.bg_status = 3;
                    m.panel_status = 3;
                    let woke = m.still == 1;
                    if woke {
                        x.req.push(Request::WakeAll);
                    }
                    crate::disp::disp(m, x);
                    let n = if woke { 4 } else { 3 };
                    return Flow::Breathed(Cont { woke, cursors: false, after: After::ChatT(n) });
                }
                x.se(SE_BUZZ);
            }
        }
        21 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                if let Some(c) = &x.world.party[1] {
                    x.req.push(Request::ManualModeAi { member: c.handle, ev: 1 });
                    x.req.push(Request::RemoteCmd { member: c.handle, cmd: 0 });
                }
                return close_panels(m, x);
            }
        }
        _ => {}
    }
    Flow::Done
}

/// ChatMenuT's frames after its breath: the flips back (when it woke the
/// tasks), `Disp; Breath` until `n` is 0, then the order: member 1 to
/// cast skill 155 on the player (`RequestChatCmd(5)`).
pub fn chat_tail(m: &mut MenuCtrl, woke: bool, n: u8, x: &mut Ctx) -> Option<Cont> {
    if woke {
        m.still = 0;
        x.req.push(Request::Still(false));
    }
    if n > 0 {
        crate::disp::disp(m, x);
        return Some(Cont { woke: false, cursors: false, after: After::ChatT(n - 1) });
    }
    if let (Some(c), Some(p)) = (&x.world.party[1], &x.world.party[0]) {
        x.req.push(Request::ChatOrder { member: c.handle, cmd: 5, target: p.handle, skill: 155 });
    }
    m.wait_count = 0;
    m.proccess = 21;
    None
}
