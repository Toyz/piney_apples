//! PARTY (PERSONAL in a town, menu 9, `PartyMenu` gcmn 0x0053a370) and its
//! pages: Add (68, `PartyInMenu` 0x0053a7a0, its page `PartyInMenuDisp` in
//! [`crate::menus::party`]), Remove (69, `PartyOutMenu` 0x0053ba90) and
//! Disband (70, `PartyDisbandMenu` 0x0053c340).
//!
//! ```text
//! 9    outside a town: "You can only form parties in towns." and back;
//!      else Add greyed with a full party, Remove and Disband alone
//!      (reverseHead), the rows into menuKanji, the target dropped; Select;
//!      cancel (19) back, OK (18) into the row; the row's help (DispMsg)
//! 68   refused with three in the party or none to call; the members of
//!      partyMemberFlag not in the party (at most 8 rows shown, the page
//!      PartyInMenuDisp); OK: "Add <name> to your party." OK / Cancel; OK:
//!      "Calling party member." then, when partyMemberCall has the member
//!      and there is room for its character (fewer than five loaded, or it
//!      is one of them), ccThPartyAdd (ccParty::AddMember), the face, 40
//!      frames, the message closed, at 51 the member's greeting
//!      (spcMsgPartyIn10 / 11, or 20 / 21 when its character was loaded
//!      and idle; by talkNum), then the menu shut; else 41 frames, "There
//!      is no response to the Flash Mail." and back to the list
//! 69   refused alone; the members but Kite; OK: "Remove <name> from your
//!      party." OK / Cancel; OK: the member's farewell (spcMsgPartyOut10 /
//!      11), ccParty::DelMember(slot), the menu shut
//! 70   refused alone; "Removing all members." OK / Cancel; OK: each
//!      member's farewell and DelMember in turn (slots 1, 2), the menu shut
//! ```
//!
//! The greetings and farewells open with the member's voice
//! (`ccEvVoiceRequest(-30, id * 3 + n)`: n 0 a greeting, 1 the idle
//! one's, 2 a farewell).

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::disp::member_name;
use crate::menus::party::{can_join, menu_face_row};
use crate::menus::personal::check;
use crate::menus::system::{extract_menu, item_rows, str_cat};

/// `spcMsgPartyIn10`, `11`, `20`, `21`, `spcMsgPartyOut10`, `11`: a
/// `ccEvMsgData*` per member.
pub const SPC_MSG: [u32; 6] = [0x0063_7f90, 0x0063_7ff0, 0x0063_8150, 0x0063_81b0, 0x0063_8310, 0x0063_8370];
/// `saveData.talkNum[18]` (+0x220c) and `partyMemberCall` (+0x2224).
pub const TALK_NUM: usize = 0x220c;
pub const PARTY_MEMBER_CALL: usize = 0x2224;

/// An event record: `emode` and its lines (trailing empty ones left out).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartyMsg {
    pub mode: i32,
    pub lines: Vec<Vec<u8>>,
}

/// PARTY's texts.
#[derive(Clone, Debug, Default)]
pub struct PartyTexts {
    pub help: Vec<[Vec<u8>; 3]>,
    pub warn: Vec<[Vec<u8>; 3]>,
    pub out_info: [[Vec<u8>; 2]; 2],
    /// `SPC_MSG`'s tables, 18 members each.
    pub msgs: Vec<Vec<Option<PartyMsg>>>,
}

impl PartyTexts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> PartyTexts {
        use crate::tables::piece;
        let f = piney_data::tables::fieldui::of(volume);
        let three = |l: &[&str]| [piece(l, 0), piece(l, 1), piece(l, 2)];
        let two = |l: &[&str]| [piece(l, 0), piece(l, 1)];
        let record = |r: &Option<piney_data::tables::types::EvMsgData>| {
            r.map(|r| {
                let mut lines: Vec<Vec<u8>> =
                    r.str.unwrap_or_default().iter().map(|l| piney_data::tables::sjis::encode(l)).collect();
                while lines.last().is_some_and(|l| l.is_empty()) {
                    lines.pop();
                }
                PartyMsg { mode: r.emode, lines }
            })
        };
        let out = f.party_out_info();
        PartyTexts {
            help: f.party_help().iter().map(|l| three(l)).collect(),
            warn: f.party_warn().iter().map(|l| three(l)).collect(),
            out_info: [two(out[0]), two(out[1])],
            msgs: [f.spc_msg0(), f.spc_msg1(), f.spc_msg2(), f.spc_msg3(), f.spc_msg4(), f.spc_msg5()]
                .iter()
                .map(|t| t.iter().map(record).collect())
                .collect(),
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

fn back(m: &mut MenuCtrl) {
    let prev = m.lists[idx(m)].prev;
    m.back_to_prev(prev);
}

/// The cancel or OK of a Select, its sound played (cancel first).
fn key(x: &mut Ctx) -> Option<bool> {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        Some(false)
    } else if x.pushed_ok() {
        x.se(SE_OK);
        Some(true)
    } else {
        None
    }
}

fn info(m: &mut MenuCtrl, x: &Ctx, lines: [Option<&[u8]>; 4]) {
    let names = x.save.names();
    m.msg.open_info(lines, &names);
}

/// A refusal: `partyMenuWarn[k]`'s first `n` lines.
fn warn(m: &mut MenuCtrl, x: &Ctx, k: usize, n: usize) {
    let w = x.texts.pers.party.warn.get(k).cloned().unwrap_or_default();
    let l = |i: usize| (i < n).then_some(&w[i][..]);
    info(m, x, [l(0), l(1), l(2), None]);
}

/// `ccMsg->Open(spcMsg..[id], name, -30, id * 3 + n)`.
fn open_msg(m: &mut MenuCtrl, x: &mut Ctx, table: usize, id: i32, name: &[u8], n: i32) {
    let r = x.texts.pers.party.msgs.get(table).and_then(|t| t.get(id.clamp(0, 17) as usize)).cloned().flatten();
    let r = r.unwrap_or_default();
    let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
    let names = x.save.names();
    m.msg.open_record(r.mode, Some(name), &lines, &names);
    x.req.push(Request::VoiceRequest { grp: -30, msg: id * 3 + n });
}

fn talk_num(x: &Ctx, id: i32) -> i8 {
    x.save.save.u8(TALK_NUM + id.clamp(0, 17) as usize) as i8
}

/// The menu shut (`menuNext` -1, the tasks woken, `Disp`, the breath).
fn shut(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont::close(woke))
}

/// `ccParty::DelMember(slot)`: the party's own side at once (this frame's
/// drawing sees it), the rest the runtime's.
fn del_member(x: &mut Ctx, slot: i32) {
    if (1..3).contains(&slot) && x.world.party_id[slot as usize] != -1 {
        x.world.party[slot as usize] = None;
        x.world.party_id[slot as usize] = -1;
        x.req.push(Request::DelMember(slot));
    }
}

/// `PartyMenu` (menu 9).
pub fn party_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let step = m.proccess;
    match step {
        0 => {
            if x.world.game.area != 0 {
                m.lists[i].disp = 4;
                let w = x.texts.pers.party.warn.first().map(|w| w[0].clone()).unwrap_or_default();
                info(m, x, [Some(&w), None, None, None]);
                m.proccess = 10;
            } else {
                let n = x.world.party_num();
                if n < 2 {
                    m.reverse_head = (m.reverse_head & !1) | 6;
                } else if n == 3 {
                    m.reverse_head = (m.reverse_head | 1) & !6;
                }
                m.lists[i].disp = 7;
                let rows = item_rows(m);
                extract_menu(m, rows);
                x.req.push(Request::TargetFix(true));
                x.change_target(None);
                m.proccess += 1;
            }
        }
        1 => {
            // Select(0, 0) with a3 left 1 from the switch: the help fades in
            // again on a move.
            m.select(0, 0, true, x);
            match key(x) {
                Some(false) => back(m),
                Some(true) => {
                    let _ = m.change_menu(x);
                }
                None => {}
            }
        }
        10 if check(m, x) => {
            m.msg.close();
            back(m);
        }
        _ => {}
    }
    if m.proccess < 2 {
        let sel = m.lists[i].select.max(0) as usize;
        let h = x.texts.pers.party.help.get(sel).cloned().unwrap_or_default();
        let names = x.save.names();
        m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
    }
    Flow::Done
}

/// `ccSPC::CheckSpc(id)`: the member's place among the characters loaded.
fn check_spc(x: &Ctx, id: i32) -> Option<usize> {
    x.world.spc.iter().position(|&(v, _)| v == id)
}

/// `PartyInMenu` (menu 68).
pub fn party_in_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let face = i32::from(m.face_num);
    let step = m.proccess;
    match step {
        0 => {
            m.lists[i].disp = 4;
            if x.world.party_num() >= 3 {
                warn(m, x, 2, 3);
                m.proccess = 10;
            } else {
                m.lists[i].my = (0..18).filter(|&k| can_join(x, k)).count() as i16;
                if m.lists[i].my == 0 {
                    warn(m, x, 3, 2);
                    m.proccess = 10;
                } else {
                    m.lists[i].index = 0;
                    m.lists[i].dy = 0;
                    m.proccess += 1;
                }
            }
        }
        1 | 2 => {
            if m.proccess == 1 {
                let l = &mut m.lists[i];
                l.select = l.index;
                l.y = l.my.min(8);
                l.disp = 7;
                m.panel_status = 1;
                m.menu_status = 1;
                m.exception_disp = 1;
                m.proccess += 1;
            }
            m.select_scr(0, 0, 0, x);
            let l = m.lists[i].clone();
            let (dy, y, sel) = (i32::from(l.dy), i32::from(l.y), i32::from(l.select));
            let mut n = 0;
            for k in 0..18 {
                if !can_join(x, k) {
                    continue;
                }
                if n >= dy {
                    if y < n - dy {
                        break;
                    }
                    if sel == n {
                        m.face_num = k as i16;
                        break;
                    }
                }
                n += 1;
            }
            match key(x) {
                Some(false) => {
                    m.panel_status = 3;
                    back(m);
                }
                Some(true) => {
                    m.menu_status = 3;
                    m.panel_status = 3;
                    m.lists[i].index = m.lists[i].select;
                    m.cursors_init();
                    m.proccess += 1;
                }
                None => {}
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
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            let [a, b] = x.texts.party_in_info[0].clone();
            let mut s = a;
            s.extend_from_slice(&member_name(x, face));
            s.extend_from_slice(&b);
            info(m, x, [Some(&s), None, None, None]);
            m.cursors_init();
            m.proccess += 1;
        }
        5 => {
            m.select(0, 0, false, x);
            if let Some(ok) = key(x) {
                m.menu_status = 3;
                m.cursors_init();
                m.msg.close();
                if !ok || m.lists[i].select == 1 {
                    m.proccess += 1;
                } else {
                    m.panel_status = 3;
                    let call = x.save.save.i32(PARTY_MEMBER_CALL) as u32 & (1u32 << face.clamp(0, 31)) != 0;
                    m.proccess = if !call {
                        20
                    } else if x.world.spc.len() < 5 || check_spc(x, face).is_some() {
                        30
                    } else {
                        20
                    };
                }
            }
        }
        6 => {
            if m.menu_status == 0 {
                m.lists[i].x = 9;
                m.cursors_init();
                m.proccess = 1;
            }
        }
        10 => {
            if check(m, x) {
                m.msg.close();
                back(m);
            }
        }
        20 | 30 => {
            if m.menu_status == 0 {
                m.lists[i].x = 9;
                if m.proccess == 30 {
                    m.temp[0] = 0;
                    if let Some(s) = check_spc(x, face)
                        && x.world.spc[s].1 == 0
                    {
                        m.temp[0] = -1;
                    }
                }
                let [a, b] = x.texts.party_in_info[1].clone();
                info(m, x, [Some(&a), Some(&b), None, None]);
                m.wait_count = 0;
                if m.proccess == 30 {
                    // ccStartThread(ccThPartyAdd): ccParty::AddMember(face)
                    // as the frame ends.
                    x.req.push(Request::AddMember(face as i16));
                }
                m.proccess += 1;
            }
        }
        21 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 41 {
                let t = x.texts.pers.party.warn.get(4).cloned().unwrap_or_default();
                let names = x.save.names();
                m.msg.change_info([Some(&t[0]), Some(&t[1]), Some(&t[2]), None], &names);
                m.proccess += 1;
            }
        }
        22 => {
            if check(m, x) {
                m.msg.close();
                m.cursors_init();
                m.proccess = 1;
            }
        }
        31 => {
            // The task is done a frame on: the member's face.
            let slot = x.world.member_slot(face);
            if slot >= 0 {
                m.face_tex[slot as usize] = menu_face_row(x, face);
            }
            m.proccess += 1;
            if m.wait_count < 40 {
                m.wait_count += 1;
            }
        }
        32 => {
            let w = m.wait_count;
            if w == 40 {
                m.msg.close();
            } else if w >= 51 {
                let idle = m.temp[0] != 0;
                let table = usize::from(talk_num(x, face) != 0) + if idle { 2 } else { 0 };
                let name = member_name(x, face);
                open_msg(m, x, table, face, &name, i32::from(idle));
                m.proccess += 1;
            }
            m.wait_count += 1;
        }
        33 if check(m, x) => {
            return shut(m, x);
        }
        _ => {}
    }
    Flow::Done
}

/// The party's members but Kite (slots 1, 2): (slot, character).
fn others(x: &Ctx) -> Vec<(usize, crate::world::CharInfo)> {
    (1..3).filter(|&k| x.world.party_id[k] != -1).filter_map(|k| x.world.party[k].clone().map(|c| (k, c))).collect()
}

/// `PartyOutMenu` (menu 69).
pub fn party_out_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let step = m.proccess;
    match step {
        0 => {
            m.lists[i].disp = 4;
            if x.world.party_num() < 2 {
                warn(m, x, 1, 2);
                m.proccess = 10;
            } else {
                m.lists[i].index = 0;
                m.proccess += 1;
            }
        }
        1 => {
            m.panel_status = 1;
            m.menu_status = 1;
            let mut buf = Vec::new();
            let o = others(x);
            for (_, c) in &o {
                str_cat(&mut buf, &c.name, 16);
            }
            let l = &mut m.lists[i];
            l.disp = 7;
            l.select = l.index;
            l.y = o.len() as i16;
            extract_menu(m, buf);
            m.proccess += 1;
        }
        2 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(false) => {
                    m.panel_status = 3;
                    back(m);
                }
                Some(true) => {
                    let sel = usize::try_from(m.lists[i].select).ok();
                    if let Some((k, _)) = sel.and_then(|s| others(x).get(s).cloned()) {
                        m.lists[i].page = k as i16;
                    }
                    m.lists[i].index = m.lists[i].select;
                    m.panel_status = 3;
                    m.menu_status = 3;
                    m.cursors_init();
                    m.proccess += 1;
                }
                None => {}
            }
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
            l.select = 0;
            let slot = l.page.clamp(0, 2) as usize;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            let [a, b] = x.texts.pers.party.out_info[0].clone();
            let mut s = a;
            s.extend_from_slice(&x.world.party[slot].as_ref().map(|c| c.name.clone()).unwrap_or_default());
            s.extend_from_slice(&b);
            info(m, x, [Some(&s), None, None, None]);
            m.cursors_init();
            m.proccess += 1;
        }
        5 => {
            m.select(0, 0, false, x);
            if let Some(ok) = key(x) {
                m.menu_status = 3;
                m.cursors_init();
                m.msg.close();
                if !ok || m.lists[i].select == 1 {
                    m.proccess += 1;
                } else {
                    m.panel_status = 3;
                    m.proccess = 20;
                }
            }
        }
        6 => {
            if m.menu_status == 0 {
                m.lists[i].x = 9;
                m.cursors_init();
                m.proccess = 1;
            }
        }
        10 => {
            if check(m, x) {
                m.msg.close();
                back(m);
            }
        }
        20 => {
            if m.menu_status == 0 {
                m.lists[i].x = 9;
                let slot = m.lists[i].page.clamp(0, 2) as usize;
                let id = x.world.party_id[slot];
                let table = 4 + usize::from(talk_num(x, id) != 0);
                let name = member_name(x, id);
                open_msg(m, x, table, id, &name, 2);
                m.proccess += 1;
            }
        }
        21 if check(m, x) => {
            let slot = i32::from(m.lists[i].page);
            del_member(x, slot);
            return shut(m, x);
        }
        _ => {}
    }
    Flow::Done
}

/// `PartyDisbandMenu` (menu 70).
pub fn party_disband_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let step = m.proccess;
    match step {
        0 => {
            m.lists[i].disp = 4;
            if x.world.party_num() < 2 {
                warn(m, x, 1, 2);
                m.proccess = 10;
            } else {
                m.panel_status = 1;
                m.lists[i].disp = 11;
                let s = x.texts.pers.party.out_info[1][0].clone();
                info(m, x, [Some(&s), None, None, None]);
                let d = x.texts.dialog_default.clone();
                extract_menu(m, d);
                m.lists[i].select = 0;
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        1 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(true) => {
                    m.msg.close();
                    if m.lists[i].select == 1 {
                        m.panel_status = 3;
                        back(m);
                    } else {
                        m.menu_status = 3;
                        m.panel_status = 3;
                        m.cursors_init();
                        m.proccess = 20;
                    }
                }
                Some(false) => {
                    m.msg.close();
                    m.panel_status = 3;
                    back(m);
                }
                None => {}
            }
        }
        10 => {
            if check(m, x) {
                m.msg.close();
                back(m);
            }
        }
        20 => {
            if m.menu_status == 0 {
                m.lists[i].x = 9;
                m.lists[i].index = 1;
                m.proccess += 1;
            }
        }
        21 => {
            let slot = m.lists[i].index;
            if slot >= 3 {
                return shut(m, x);
            }
            match x.world.party.get(slot.max(0) as usize).cloned().flatten() {
                Some(c) => {
                    let id = i32::from(c.id);
                    let table = 4 + usize::from(talk_num(x, id) != 0);
                    open_msg(m, x, table, id, &c.name, 2);
                    m.proccess += 1;
                }
                None => m.lists[i].index = slot + 1,
            }
        }
        22 if check(m, x) => {
            let slot = i32::from(m.lists[i].index);
            del_member(x, slot);
            m.lists[i].index += 1;
            m.proccess -= 1;
        }
        _ => {}
    }
    Flow::Done
}
