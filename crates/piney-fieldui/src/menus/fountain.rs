//! The Spring of Myst (`gimmickTbl` row 20, base type 0x400000, the lakes'
//! `ccGimEtc`): `FountainMenu` (gcmn 0x00548060, menu 40), `FountainMenu2`
//! (0x00548420, 41: the item thrown in), `FountainMenu3` (0x00548a00, 42:
//! Monsieur's golden axe game), their pages `FountainMenuDisp2`
//! (0x00549ed0) and `FountainMenuDisp3` (0x0054a260), and
//! `SetFountainCamera` (0x00526c60). The spring's own frames are
//! piney-battle's `gimetc`; each `EntryAffect(spring, plw, 11)` here moves
//! it on a state. The steps are in docs/engine/field-ui.md (the spring).

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::items::Item;
use crate::menus::getitem::sleep_others;
use crate::menus::system::{extract_menu, push_msg_requests};
use crate::spr::{font_type, make_num, set_clm};
use crate::talk::{self, TalkReq};
use crate::window::{disp_square, disp_square_sb, disp_target};

/// The two strings `FountainMenu3` builds its lines with (gcmn @13687,
/// @13904: "#G" and "#W?").
pub const THROWN_HEAD: &[u8] = b"#G";
pub const QUESTION: &[u8] = b"#W?";
/// `saveData.fountainCount[2]` (+0x7470, +0x7472): the items thrown in,
/// under area level 4 and from it.
pub const FOUNTAIN_COUNT: [usize; 2] = [0x7470, 0x7472];
/// The voices' group (`ccEvVoiceRequest(-20, talkNum ...)`).
pub const VOICE_GRP: i32 = -20;
/// The Golden and Silver Axes (`category << 16 | id`).
pub const GOLDEN_AXE: i32 = 0x3_003c;
pub const SILVER_AXE: i32 = 0x3_003d;

/// The spring's texts and tables.
#[derive(Clone, Debug, Default)]
pub struct FountainTexts {
    /// `fountainMenuHelp[k]`'s three pieces.
    pub help: Vec<[Vec<u8>; 3]>,
    /// "Neither", "Golden Axe", "Silver Axe" (16 bytes each).
    pub answers: Vec<u8>,
    /// `fountainNameStr`'s names by server (+ 5 from level 4).
    pub names: Vec<Vec<u8>>,
    pub thrown_head: Vec<u8>,
    pub question: Vec<u8>,
    /// `feTbl[k]`: 80 rows each (the limits are 73 at most; the game may
    /// read the row at the limit).
    pub fe: Vec<Vec<i16>>,
    pub f_limit: [i16; 20],
}

impl FountainTexts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> FountainTexts {
        use crate::tables::piece;
        use piney_data::tables::sjis::encode;
        let f = piney_data::tables::fieldui::of(volume);
        FountainTexts {
            help: f.fountain_help().iter().map(|l| [piece(l, 0), piece(l, 1), piece(l, 2)]).collect(),
            answers: encode(f.fountain_answers()),
            names: f.fountain_names().iter().map(|l| encode(l)).collect(),
            thrown_head: THROWN_HEAD.to_vec(),
            question: QUESTION.to_vec(),
            fe: f.fe_tbl().iter().map(|l| l.to_vec()).collect(),
            f_limit: f.f_limit().try_into().unwrap_or_default(),
        }
    }
}

/// What a fountain menu does after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// Menu 41's close: the item under the cursor's help after it.
    ItemHelp,
    /// Menu 42's step 2 after the tasks woke: the rest of the step.
    Hold,
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// The keys as the three menus read them: cancel first (sound 19), then
/// OK (18); `Some(true)` OK.
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

/// `InitCursol(1)` on both cursors.
fn cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// The message's Check.
fn checked(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let ok = x.save.ok();
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
    push_msg_requests(x, req);
    r != 0
}

/// `OpenInfo(help[k]'s pieces, 0, grp, msg)`: the voice asked when `voice`.
fn info(m: &mut MenuCtrl, x: &mut Ctx, k: usize, voice: bool, change: bool) {
    let h = x.texts.fountain.help.get(k).cloned().unwrap_or_default();
    let names = x.save.names();
    let lines = [Some(h[0].as_slice()), Some(h[1].as_slice()), Some(h[2].as_slice()), None];
    if change {
        m.msg.change_info(lines, &names);
    } else {
        m.msg.open_info(lines, &names);
    }
    if voice {
        x.req.push(Request::VoiceRequest { grp: VOICE_GRP, msg: k as i32 });
    }
}

/// `OpenInfo(line, 0, 0, 0, -1, -1)`: one line, no voice.
fn line(m: &mut MenuCtrl, x: &mut Ctx, s: &[u8]) {
    let names = x.save.names();
    m.msg.open_info([Some(s), None, None, None], &names);
}

/// `EntryAffect(cmndTargetPrev, plw, 11)`: the spring on a state.
fn spring_on(x: &mut Ctx) {
    if let Some(t) = &x.target_prev {
        x.req.push(Request::Affect { target: t.handle, kind: 11 });
    }
}

/// The spring's state (`cmndTargetPrev` +0x1d4), -1 with it gone.
fn spring_state(x: &Ctx) -> i32 {
    x.target_prev.as_ref().map_or(-1, |t| t.act_num)
}

/// The menu shut as the three menus shut it: the target free, then
/// `CloseMenu`'s steps.
fn shut(m: &mut MenuCtrl, x: &mut Ctx, after: After) -> Flow {
    x.req.push(Request::TargetFix(false));
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont { woke, cursors: false, after })
}

/// The bag's items of categories 0-9, in the bag's order (the save's
/// 40 slots at +0x30: id, category, count).
pub fn throwables(x: &Ctx) -> Vec<Item> {
    let b = x.save.save.bytes();
    (0..40)
        .map(|k| {
            let at = 0x30 + 4 * k;
            Item { id: i16::from_le_bytes([b[at], b[at + 1]]), cat: b[at + 2] as i8, num: b[at + 3] as i8 }
        })
        .filter(|it| (0..10).contains(&it.cat))
        .collect()
}

/// The item's code (`category << 16 | id`).
fn code(it: Item) -> i32 {
    (i32::from(it.cat) << 16) | i32::from(it.id as u16)
}

/// `FountainMenu` (menu 40).
pub fn fountain_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            m.bg_status = 1;
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            info(m, x, 0, true, false);
            let rows = x.texts.dialog_default.clone();
            extract_menu(m, rows);
            m.lists[i].select = 0;
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(true) => {
                    m.msg.close();
                    x.req.push(Request::VoiceStop);
                    cursors(m);
                    if m.lists[i].select == 1 {
                        return shut(m, x, After::Nothing);
                    }
                    m.change_menu_to(41);
                    Flow::Done
                }
                Some(false) => {
                    m.msg.close();
                    cursors(m);
                    x.req.push(Request::TargetFix(false));
                    x.req.push(Request::VoiceStop);
                    shut(m, x, After::Nothing)
                }
                None => Flow::Done,
            }
        }
        _ => Flow::Done,
    }
}

/// `FountainMenu2` (menu 41).
pub fn fountain_menu2(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let bag = throwables(x);
    m.lists[i].my = bag.len() as i16;
    match m.proccess {
        0 => {
            if bag.is_empty() {
                let s = x.texts.fountain.help[1][0].clone();
                line(m, x, &s);
                m.proccess += 1;
            } else {
                let l = &mut m.lists[i];
                l.y = l.my.min(8);
                l.dy = 0;
                m.exception_disp = 1;
                m.menu_status = 1;
                m.proccess += 2;
            }
            Flow::Done
        }
        1 => {
            if checked(m, x) {
                m.msg.close();
                return shut(m, x, After::Nothing);
            }
            Flow::Done
        }
        2 => {
            m.select_scr(0, 0, 0, x);
            let it = bag.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            if x.pad.push.bits() & 0x10 != 0 {
                x.se(SE_OK);
                m.item_num = code(it);
                m.change_menu_to(64);
                m.first_time = 0;
                return Flow::Done;
            } else {
                match key(x) {
                    Some(false) => {
                        cursors(m);
                        return shut(m, x, After::Fountain(Tail::ItemHelp));
                    }
                    Some(true) => {
                        m.item_num = code(it);
                        cursors(m);
                        m.change_menu_to(42);
                    }
                    None => {}
                }
            }
            item_help(m, x, it);
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// `DispMsg`: the item's name and comment in the window (none for an
/// empty slot).
fn item_help(m: &mut MenuCtrl, x: &Ctx, it: Item) {
    let names = x.save.names();
    let p = x.texts.items.item(i32::from(it.cat), i32::from(it.id)).filter(|_| it.cat >= 0 && it.id >= 0);
    match p {
        Some(p) => {
            let p = p.clone();
            m.msg.disp_msg(
                0x100,
                Some(&p.name),
                [Some(&p.comment[0]), Some(&p.comment[1]), Some(&p.comment[2])],
                &names,
            );
        }
        None => m.msg.disp_msg(0x100, None, [None, None, None], &names),
    }
}

/// A menu's breath tail ([`After::Fountain`]).
pub fn after(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) {
    match t {
        Tail::ItemHelp => {
            let bag = throwables(x);
            let i = idx(m);
            let it = bag.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            item_help(m, x, it);
        }
        Tail::Hold => {
            hold(m, x);
            camera(m, x);
        }
    }
}

/// The row of item `item` (`category << 16 | id`) among the spring's
/// known items (`feTbl[category]`'s first `f_limitTbl` rows) and that
/// limit.
fn known(x: &Ctx, item: i32) -> (Option<usize>, i16) {
    let g = x.world.game;
    let cat = item >> 16;
    let mut k = g.server;
    if g.word_level >= 4 {
        k += 10;
    }
    if cat == 2 {
        k += 5;
    }
    let f = &x.texts.fountain;
    let limit = usize::try_from(k).ok().and_then(|k| f.f_limit.get(k)).copied().unwrap_or(0);
    let list = usize::try_from(cat).ok().and_then(|c| f.fe.get(c));
    let id = (item & 0xffff) as i16;
    let row = list.and_then(|l| l.iter().take(limit.max(0) as usize).position(|&v| v == id));
    (row, limit)
}

/// The line `talkNum + k`.
fn talk_line(m: &MenuCtrl, k: i16) -> usize {
    (m.talk.talk_num + k).max(0) as usize
}

/// `FountainMenu3` (menu 42).
pub fn fountain_menu3(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let flow = menu3(m, x);
    // A breath puts the tail (and so the camera) on the next frame.
    if !matches!(flow, Flow::Breathed(_)) {
        camera(m, x);
    }
    flow
}

/// The tail: `SetFountainCamera` while 2 <= proccess < 15.
fn camera(m: &MenuCtrl, x: &mut Ctx) {
    if (2..15).contains(&m.proccess) {
        talk::req(x, TalkReq::FountainCamera);
    }
}

fn menu3(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let proccess = m.proccess;
    match proccess {
        0 => {
            m.lists[i].select = 0;
            m.exception_disp = 0;
            let rows = x.texts.dialog_default.clone();
            extract_menu(m, rows);
            let f = &x.texts.fountain;
            let mut s = f.help[2][0].clone();
            s.extend_from_slice(&x.texts.items.item_name(m.item_num >> 16, m.item_num & 0xffff));
            s.extend_from_slice(&f.question);
            line(m, x, &s);
            m.proccess += 1;
        }
        1 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(false) => {
                    m.msg.close();
                    cursors(m);
                    let prev = m.lists[i].prev;
                    back(m, prev);
                }
                Some(true) => {
                    m.msg.close();
                    cursors(m);
                    if m.lists[i].select != 0 {
                        let prev = m.lists[i].prev;
                        back(m, prev);
                    } else {
                        spring_on(x);
                        m.proccess += 1;
                    }
                }
                None => {}
            }
        }
        2 => {
            m.wait_count = 0;
            m.menu_status = 3;
            m.bg_status = 3;
            m.map_status = 3;
            if m.still != 0 {
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return Flow::Breathed(Cont { woke: true, cursors: false, after: After::Fountain(Tail::Hold) });
            }
            hold(m, x);
        }
        3 => {
            if m.wait_count == 60 {
                m.msg.close();
            } else {
                m.wait_count += 1;
            }
            if spring_state(x) == 3 {
                let item = m.item_num;
                crate::items::del_item(x.save, 0, item >> 16, item & 0xffff, 1);
                let high = x.world.game.word_level >= 4;
                let at = FOUNTAIN_COUNT[usize::from(high)];
                let c = (i32::from(x.save.save.i16(at)) + 1).min(10000);
                x.save.save.set_i16(at, c as i16);
                m.talk.talk_num = if high { 15 } else { 4 };
                cursors(m);
                m.wait_count = 0;
                let l = &mut m.lists[i];
                l.select = 0;
                l.x = 6;
                l.y = 3;
                l.disp = 11;
                m.menu_status = 1;
                m.exception_disp = 1;
                let rows = x.texts.fountain.answers.clone();
                extract_menu(m, rows);
                let k = talk_line(m, 0);
                info2(m, x, k);
                m.proccess += 1;
            }
        }
        4 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                x.se(SE_OK);
                m.menu_status = 3;
                x.req.push(Request::VoiceStop);
                m.msg.close();
                cursors(m);
                spring_on(x);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        5 => {
            let c = m.wait_count;
            m.wait_count += 1;
            if c >= 11 {
                m.lists[i].disp = 4;
                m.exception_disp = 1;
                m.menu_status = 1;
                if spring_state(x) == 5 {
                    match m.lists[i].select {
                        1 | 2 => {
                            let k = talk_line(m, m.lists[i].select);
                            info2(m, x, k);
                            m.proccess = 10;
                        }
                        0 => {
                            let k = talk_line(m, 3);
                            info2(m, x, k);
                            m.wait_count = 0;
                            m.proccess += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
        6 => {
            if checked(m, x) {
                let c = m.wait_count;
                m.wait_count += 1;
                if c >= 2 {
                    m.proccess += 1;
                } else {
                    let k = talk_line(m, 3 + m.wait_count);
                    info_change(m, x, k);
                }
            }
        }
        7 => {
            let (row, _) = known(x, m.item_num);
            m.menu_status = 3;
            x.req.push(Request::VoiceStop);
            m.msg.close();
            m.wait_count = 0;
            if row.is_some() {
                spring_on(x);
                m.proccess = 11;
            } else {
                m.proccess += 1;
            }
        }
        8 => {
            let c = m.wait_count;
            m.wait_count += 1;
            if c >= 11 {
                m.menu_status = 1;
                m.exception_disp = 1;
                let k = talk_line(m, 6);
                info_change(m, x, k);
                m.proccess += 1;
                m.wait_count = 0;
            }
        }
        9 => {
            if checked(m, x) {
                let k = talk_line(m, 7 + m.wait_count);
                info_change(m, x, k);
                let c = m.wait_count;
                m.wait_count += 1;
                if c >= 2 {
                    m.proccess += 1;
                }
            }
        }
        10 => {
            if checked(m, x) {
                m.menu_status = 3;
                m.msg.close();
                spring_on(x);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        11 => {
            if m.wait_count >= 11 {
                if spring_state(x) == 7 {
                    m.proccess += 1;
                }
            } else {
                m.wait_count += 1;
            }
        }
        12 => {
            spring_on(x);
            let k = talk_line(m, 10);
            info2(m, x, k);
            m.menu_status = 1;
            m.exception_disp = 1;
            m.proccess += 1;
        }
        13 => {
            if checked(m, x) {
                m.menu_status = 3;
                spring_on(x);
                m.proccess += 1;
            }
        }
        14 => {
            if x.target_prev.is_none() {
                x.req.push(Request::TargetFix(false));
                talk::req(x, TalkReq::Camera(1));
                m.wait_count = 0;
                talk::req(x, TalkReq::FountainParty(false));
                m.proccess += 1;
            }
        }
        15 => {
            let c = m.wait_count;
            m.wait_count += 1;
            if c >= 11 {
                if m.still == 0 {
                    m.bg_status = 1;
                    m.mode = 0;
                    sleep_others(m, x);
                }
                m.map_status = 1;
                result(m, x);
            }
        }
        16 => {
            if checked(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        17 => {
            let c = m.wait_count;
            m.wait_count += 1;
            if c >= 7 {
                m.change_menu_to(67);
                m.first_time = 0;
            }
        }
        _ => {}
    }
    Flow::Done
}

/// Back to the list (menu 41), as the game writes it.
fn back(m: &mut MenuCtrl, prev: i16) {
    m.menu_next = prev;
    m.menu_status = 3;
    m.proccess = 0;
    m.wait_count = 0;
    cursors(m);
}

/// `OpenInfo(help[k]'s first two pieces, 0, 0, -20, k)`.
fn info2(m: &mut MenuCtrl, x: &mut Ctx, k: usize) {
    let h = x.texts.fountain.help.get(k).cloned().unwrap_or_default();
    let names = x.save.names();
    m.msg.open_info([Some(h[0].as_slice()), Some(h[1].as_slice()), None, None], &names);
    x.req.push(Request::VoiceRequest { grp: VOICE_GRP, msg: k as i32 });
}

/// `ChangeInfo(help[k]'s first two pieces, 0, 0, -20, k)`.
fn info_change(m: &mut MenuCtrl, x: &mut Ctx, k: usize) {
    let h = x.texts.fountain.help.get(k).cloned().unwrap_or_default();
    let names = x.save.names();
    m.msg.change_info([Some(h[0].as_slice()), Some(h[1].as_slice()), None, None], &names);
    x.req.push(Request::VoiceRequest { grp: VOICE_GRP, msg: k as i32 });
}

/// Step 2's rest: the party held, the enemies' conditions cleared, "<item>
/// was thrown in.".
fn hold(m: &mut MenuCtrl, x: &mut Ctx) {
    talk::req(x, TalkReq::FountainParty(true));
    let f = &x.texts.fountain;
    let mut s = f.thrown_head.clone();
    s.extend_from_slice(&x.texts.items.item_name(m.item_num >> 16, m.item_num & 0xffff));
    s.extend_from_slice(&f.help[3][0]);
    line(m, x, &s);
    m.proccess += 1;
}

/// Step 15's answer: the item Monsieur gives, and his line.
fn result(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    match m.lists[i].select {
        1 | 2 => {
            m.drain_num = 1;
            m.talk.drain_item[0] = if m.lists[i].select == 1 { GOLDEN_AXE } else { SILVER_AXE };
            m.wait_count = 10;
            m.proccess = 17;
        }
        0 => {
            let item = m.item_num;
            let cat = item >> 16;
            let (row, limit) = known(x, item);
            let Some(row) = row else {
                m.drain_num = 3;
                m.talk.drain_item[0] = item;
                m.talk.drain_item[1] = GOLDEN_AXE;
                m.talk.drain_item[2] = SILVER_AXE;
                m.wait_count = 10;
                m.proccess = 17;
                return;
            };
            let bg = x.world.game.bgnum;
            let step: i32 = if cat < 6 {
                match bg {
                    0 | 3 => 2,
                    1 | 2 => 1,
                    _ => -1,
                }
            } else {
                match bg {
                    0 | 3 => -1,
                    1 | 2 => 1,
                    _ => 2,
                }
            };
            let mut r = row as i32 + step;
            if r < 0 {
                r = 0;
            } else if i32::from(limit) < r {
                r = i32::from(limit);
            }
            let f = &x.texts.fountain;
            let id = usize::try_from(cat).ok().and_then(|c| f.fe.get(c)).and_then(|l| l.get(r as usize)).copied();
            m.drain_num = 1;
            m.talk.drain_item[0] = (cat << 16) | i32::from(id.unwrap_or(0));
            let (k, se) = match step {
                2.. => (26, 92),
                1 => (27, 91),
                _ => (28, 93),
            };
            x.se(se);
            let h = f.help[k].clone();
            let names = x.save.names();
            // The name is saveData itself: the player's (+0).
            let pl = names.name.clone();
            m.msg.open(0x100, Some(&pl), [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
            x.req.push(Request::VoiceRequest { grp: VOICE_GRP, msg: k as i32 });
            m.proccess += 1;
        }
        _ => {}
    }
}

/// `FountainMenuDisp2`: the list of the bag's items (eight rows, a scroll
/// bar past eight), the cursor, the names and counts. The rows are drawn
/// before their names are extracted (the game's order).
pub fn fountain_menu_disp2(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let a = m.alpha;
    m.win.dx = 39.0;
    m.win.dy = 96.0;
    // The element's strT, NULL for this list: no tab.
    let title = (!l.title.is_empty()).then_some(&l.title[..]);
    if l.y < l.my {
        disp_square_sb(&mut m.win, 11, i32::from(l.y), i32::from(l.dy), i32::from(l.my), title);
    } else {
        disp_square(&mut m.win, 11, i32::from(l.y), title);
    }
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    m.kanji.set_colour(7);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    let mut rows = Vec::new();
    let mut shown: i16 = 0;
    let fr = x.frame_rate;
    for (s, it) in throwables(x).into_iter().enumerate() {
        if (s as i16) < l.dy {
            continue;
        }
        let name = x.texts.items.item_name(i32::from(it.cat), i32::from(it.id));
        crate::menus::system::str_cat(&mut rows, &name, 16);
        if shown == l.select - l.dy {
            m.cursor.disp(&mut m.win, 39.0, f32::from(shown * 20 + 112), 11, i32::from(l.select), a, 3, fr);
        }
        m.font.set_alpha(a);
        m.font.dx = 161.0;
        m.font.dy = f32::from(shown * 20 + 114);
        make_num(&mut m.font, 3, i32::from(it.num));
        m.kanji.set_alpha(a);
        m.kanji.dx = 53.0;
        m.kanji.dy = f32::from(shown * 20 + 112);
        m.kanji.make_packet(i32::from(shown));
        shown += 1;
        if shown >= 8 {
            break;
        }
    }
    extract_menu(m, rows);
}

/// `FountainMenuDisp3`: Monsieur's name plate ("Monsieur Lv. n" by server,
/// the second five from area level 4).
pub fn fountain_menu_disp3(m: &mut MenuCtrl, x: &mut Ctx) {
    let g = x.world.game;
    let mut k = g.server;
    if g.word_level >= 4 {
        k += 5;
    }
    let name = usize::try_from(k).ok().and_then(|k| x.texts.fountain.names.get(k)).cloned().unwrap_or_default();
    let a = m.alpha;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = 35.0;
    m.win.dy = 96.0;
    let names = x.save.names();
    disp_target(&mut m.win, piney_desktop::message::str_len(&name, &names), 0);
    let mut buf = Vec::new();
    crate::menus::system::str_cat(&mut buf, &name, 16);
    m.setting_text[0] = buf;
    let k = &mut m.setting[0];
    set_clm(k, 16, 0, 0, 1);
    k.set_colour(7);
    k.set_alpha(a);
    k.dx = 60.0;
    k.dy = 101.0;
    k.make_packet(0);
}
