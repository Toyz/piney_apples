//! The Recorder's page: `RecordMenu` (gcmn 0x00558260, menu 53: Save) and
//! `RecordMenuDisp` (0x00558db0), over `ccSaveSys` (main sdmng.cpp, the
//! desktop Data screen's own: [`piney_desktop::savesys::SaveSys`]).
//!
//! `StartReq(3)` starts the `ccThSaveSys` task (priority 20: it runs before
//! `ccThMenu` from the next frame on) whose `MainProccess` does the card
//! work; the page asks it for each step and follows its `result`: the low
//! 12 bits a `saveSysMsg` message, 0x1000 / 0x2000 one to acknowledge,
//! 0x8000 a YES / NO question, 0x10000 busy.
//!
//! ```text
//! proccess 0    StartReq(3); the dim; the list keeps the card slot
//!               (index), the file (page) and the last result (sx); then 1
//! proccess 1    the window gone and waited: the card slot list (x 11, y
//!               2), SlotSelectReq
//! proccess 2    Select; OK: LoadInfoReq(slot), 3; cancel: 100
//! proccess 3    until result 1 (the index read): 4
//! proccess 4    the window gone and waited: the file list (x 27, y 12),
//!               SaveSelectReq
//! proccess 5    at result 25 (choose a file): Select; OK: SaveDataReq(file),
//!               6; cancel: 1
//! proccess 6, 7 until result 1 (saved) and the wait: 4
//! proccess 100  EndReq; the window and the dim out
//! proccess 101  the window gone: the Recorder the target again, the tasks
//!               woken, back to its list
//! every frame   (proccess 1-99) result 0 outside 1: NextProccess(0), back
//!               to 1; 2: LoadInfoReq; a new result waits 10 frames (28,
//!               34 and 38 with sound 74); an acknowledgement: OK
//!               NextProccess(0); a question: OK / Cancel (disp 11) and
//!               NextProccess(1 - row) or (0); then the message: DispInfo
//!               for flagged results, else the page's help (DispMsg)
//! ```

use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_desktop::card::{MemoryCard, NoCard};
use piney_desktop::eef::from_int;
use piney_desktop::savesys::{INFO_SIZE, OPERATE_DESKTOP, SaveSys};

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::system::{extract_menu, str_cat};
use crate::spr::{font_type, make_num, set_clm};
use crate::talk;
use crate::window::{disp_square, set_type};

/// gcmn's ":" and "999:59:59".
pub const STR_COLON: &[u8] = b":";
pub const STR_MAX_TIME: &[u8] = b"999:59:59";
/// A play time at or past this shows "999:59:59".
pub const TIME_LIMIT: i32 = 0x0cdf_e5c4;

/// The page's texts.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    pub record_str: Vec<u8>,
    pub colon: Vec<u8>,
    pub max_time: Vec<u8>,
    /// `saveSysMsg` by number: four lines each (`ccKanjiStrSeparate`
    /// 0..3), or `None` where the pointer is null.
    pub messages: Vec<Option<[Vec<u8>; 4]>>,
}

impl Texts {
    /// The volume's: `recordMenuStr`'s 16-glyph rows, and `saveSysMsg`
    /// (`piney_data::tables::title`).
    pub fn of(volume: Volume) -> Texts {
        use crate::tables::piece;
        let record = piney_data::tables::fieldui::of(volume).record_str();
        Texts {
            record_str: piney_desktop::dtmenu::slots(record),
            colon: STR_COLON.to_vec(),
            max_time: STR_MAX_TIME.to_vec(),
            messages: piney_data::tables::title::of(volume)
                .save_sys_msg
                .iter()
                .map(|m| m.map(|l| [piece(l, 0), piece(l, 1), piece(l, 2), piece(l, 3)]))
                .collect(),
        }
    }
}

/// `saveSys` and the cards it writes to: the seam
/// ([`crate::FieldUi::set_card`], [`crate::FieldUi::set_card_position`]).
pub struct State {
    pub save_sys: SaveSys,
    pub card: Box<dyn MemoryCard>,
}

impl Default for State {
    /// Infection's until the field UI is given the disc's volume
    /// ([`crate::FieldUi::new`]).
    fn default() -> Self {
        State { save_sys: SaveSys::new(Volume::default()), card: Box::new(NoCard) }
    }
}

/// `ccThSaveSys`'s frame (priority 20, before `ccThMenu`): `MainProccess`
/// while `StartReq`'s task runs.
pub fn save_sys_task(st: &mut State, save: &SaveData) {
    if st.save_sys.running {
        st.save_sys.main_proccess(st.card.as_mut(), save);
    }
}

/// What the page does after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// 101's close: the flips back on, back to the list, then the common
    /// tail.
    Closed,
}

/// The page's tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::Closed => {
            m.still = 0;
            x.req.push(Request::Still(false));
            let i = idx(m);
            m.menu_next = m.lists[i].prev;
            m.menu_status = 3;
            m.proccess = 0;
            m.wait_count = 0;
            m.cursor.init(true);
            m.cursor_pr.init(true);
            common(m, x);
            None
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// The key the page reads: cancel (sound 19), else OK (18), else 0.
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

fn sys(m: &mut MenuCtrl) -> &mut SaveSys {
    &mut m.talk.record.save_sys
}

/// `RecordMenu`.
pub fn record_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 | 1 => {
            if m.proccess == 0 {
                sys(m).start_req(OPERATE_DESKTOP);
                m.menu_status = 0;
                m.bg_status = 1;
                let (port, file) = (sys(m).port, sys(m).file_num);
                let l = &mut m.lists[i];
                l.disp = 4;
                l.index = port as i16;
                l.page = file as i16;
                l.select = l.index;
                l.sx = 4;
                m.wait_count = 0;
                m.proccess += 1;
            }
            if m.menu_status == 0 && m.wait_count == 0 {
                m.exception_disp = 1;
                m.menu_status = 1;
                let l = &mut m.lists[i];
                l.disp = 4;
                l.x = 11;
                l.y = 2;
                sys(m).slot_select_req();
                m.cursor.init(true);
                m.proccess += 1;
            }
        }
        2 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            m.select(0, 0, false, x);
            let l = &mut m.lists[i];
            l.index = l.select;
            let key = pushed_key(x);
            if key == x.save.ok() {
                m.cursor.init(true);
                let port = i32::from(m.lists[i].index);
                sys(m).load_info_req(port);
                m.menu_status = 3;
                m.lists[i].select = 0;
                m.wait_count = 10;
                m.proccess += 1;
            } else if key == x.save.cancel() {
                m.cursor.init(true);
                m.proccess = 100;
            }
        }
        3 | 6 => {
            if sys(m).result == 1 {
                m.proccess += 1;
            }
        }
        4 => {
            if m.menu_status == 0 && m.wait_count == 0 {
                m.exception_disp = 0;
                let l = &mut m.lists[i];
                l.disp = 4;
                l.x = 27;
                l.y = 12;
                l.select = l.page;
                m.cursor.init(true);
                sys(m).save_select_req();
                m.proccess += 1;
            }
        }
        5 => {
            if sys(m).result == 25 {
                m.exception_disp = 2;
                m.menu_status = 1;
                let l = &mut m.lists[i];
                l.select = l.page;
                m.select(0, 0, false, x);
                let l = &mut m.lists[i];
                l.page = l.select;
                let key = pushed_key(x);
                if key == x.save.ok() {
                    m.cursor.init(true);
                    let file = i32::from(m.lists[i].page);
                    sys(m).save_data_req(file);
                    m.menu_status = 3;
                    m.lists[i].select = 0;
                    m.wait_count = 10;
                    m.proccess += 1;
                } else if key == x.save.cancel() {
                    m.cursor.init(true);
                    m.menu_status = 3;
                    m.wait_count = 10;
                    m.proccess = 1;
                }
            }
        }
        7 => {
            if m.wait_count == 0 && sys(m).result == 1 {
                m.proccess = 4;
            }
        }
        100 => {
            sys(m).end_req();
            m.menu_status = 3;
            m.bg_status = 3;
            m.proccess += 1;
        }
        101 if m.menu_status == 0 => {
            m.exception_disp = 0;
            let prev = x.target_prev.clone();
            x.change_target(prev.as_ref());
            x.req.push(Request::WakeAll);
            crate::disp::disp(m, x);
            return talk::breathed(talk::Tail::Record(Tail::Closed));
        }
        _ => {}
    }
    common(m, x);
    Flow::Done
}

/// The frame's common tail (0x00558838 on): the wait counted down, and for
/// proccess 1-99 `ccSaveSys`'s result followed.
fn common(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    if m.wait_count > 0 {
        m.wait_count -= 1;
    } else {
        m.wait_count = 0;
    }
    if m.proccess == 0 || m.proccess >= 100 {
        return;
    }
    let s0 = m.talk.record.save_sys.result;
    let n = s0 & 0xfff;
    let last = m.lists[i].sx as u16 as u32 | if m.lists[i].sx < 0 { 0xffff_0000 } else { 0 };
    let new = s0 != last;
    if n == 0 && m.proccess != 1 {
        m.talk.record.save_sys.next_proccess(0);
        m.menu_status = 3;
        m.wait_count = 10;
        m.proccess = 1;
    }
    if n == 2 {
        let port = i32::from(m.lists[i].index);
        m.talk.record.save_sys.load_info_req(port);
    }
    if matches!(n, 28 | 38 | 34) {
        if new {
            x.se(74);
            m.wait_count = 10;
        }
    } else if matches!(n, 0 | 29 | 24 | 39 | 35) && new {
        m.wait_count = 10;
    }
    let ack = s0 & 0x3000;
    if ack != 0 {
        if m.menu_status != 0 && m.menu_status != 3 {
            m.menu_status = 3;
            m.wait_count = 10;
        }
        if m.wait_count == 0 && x.pushed_ok() {
            x.se(SE_OK);
            m.talk.record.save_sys.next_proccess(0);
            m.wait_count = 10;
            m.msg.cursol = 0;
        }
    }
    let question = s0 & 0x8000;
    if question != 0 {
        if m.menu_status == 0 && m.wait_count == 0 {
            m.exception_disp = 0;
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.disp = 11;
            l.x = 6;
            l.y = 2;
            l.select = 1;
            m.cursor.init(true);
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
        }
        if m.wait_count == 0 {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.ok() {
                let sel = i32::from(m.lists[i].select);
                m.talk.record.save_sys.next_proccess(1 - sel);
                m.menu_status = 3;
                m.wait_count = 10;
            } else if key == x.save.cancel() {
                m.talk.record.save_sys.next_proccess(0);
                m.menu_status = 3;
                m.wait_count = 10;
            }
        }
    }
    m.lists[i].sx = s0 as i16;
    if m.wait_count != 0 {
        return;
    }
    let Some(Some(lines)) = x.texts.talk.record.messages.get((s0 & 0xfff) as usize).cloned() else { return };
    let names = x.save.names();
    if s0 & 0x1b000 != 0 {
        let l = |k: usize| Some(&lines[k][..]).filter(|s| !s.is_empty());
        m.msg.disp_info([l(0), l(1), l(2), l(3)], &names);
        if ack != 0 {
            m.msg.cursol = 1;
        } else if question == 0 {
            m.msg.cursol = 3;
        }
    } else if m.menu_status != 0 {
        m.msg.disp_msg(0x100, None, [Some(&lines[0]), Some(&lines[1]), Some(&lines[2])], &names);
    }
}

/// `RecordMenuDisp`: the card slots (exceptionDisp 1) or the twelve files
/// with each record's name, level and play time (2).
pub fn record_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let a = m.alpha;
    let rs = x.texts.talk.record.record_str.clone();
    extract_menu(m, rs);
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    m.kanji.set_colour(7);
    m.kanji.set_alpha(a);
    let fr = x.frame_rate;
    match m.exception_disp {
        1 => {
            set_type(&mut m.win, 1);
            m.win.set_colour(7);
            m.win.set_alpha(a);
            m.win.dx = 39.0;
            m.win.dy = 96.0;
            disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), None);
            for s0 in 0..i32::from(l.y) {
                if i32::from(l.index) == s0 {
                    m.cursor.disp(&mut m.win, 39.0, from_int(s0 * 20 + 112), 11, i32::from(l.index), a, 3, fr);
                }
                m.kanji.dx = 53.0;
                m.kanji.dy = from_int(s0 * 20 + 112);
                m.kanji.make_packet(4);
                m.kanji.dx = 149.0;
                m.kanji.dy = from_int(s0 * 20 + 112);
                m.kanji.make_packet(s0);
            }
        }
        2 => files(m, x, &l),
        _ => {}
    }
}

/// The file list (0x00559020 on).
fn files(m: &mut MenuCtrl, x: &mut Ctx, l: &crate::tables::MenuList) {
    let a = m.alpha;
    let fr = x.frame_rate;
    let s1 = 256 - ((i32::from(l.x) + 2) * 14) / 2;
    set_type(&mut m.win, 1);
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = from_int(s1);
    m.win.dy = 64.0;
    disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), None);
    font_type(&mut m.font, 1);
    m.font.shadow = false;
    let t = x.texts.talk.record.clone();
    let lv = x.texts.gt_lv.clone();
    let info = m.talk.record.save_sys.info;
    let mut s6 = 0usize;
    let mut buf = Vec::new();
    for s0 in 0..12i32 {
        let s5 = s0 & 7;
        if s5 == 0 {
            buf.clear();
            set_clm(&mut m.setting[s6], 16, 0, 0, 1);
        }
        if i32::from(l.page) == s0 {
            m.cursor.disp(
                &mut m.win,
                from_int(s1),
                from_int(s0 * 20 + 80),
                i32::from(l.x),
                i32::from(l.page),
                a,
                3,
                fr,
            );
        }
        let rec = &info[INFO_SIZE * s0 as usize..INFO_SIZE * (s0 as usize + 1)];
        let c = if i32::from(rec[2] as i8) >= m.talk.record.save_sys.volume.number() {
            6
        } else if rec[3] != 0 {
            18
        } else {
            7
        };
        m.kanji.set_colour(c);
        m.setting[s6].set_colour(c);
        m.font.set_colour(c);
        m.kanji.set_alpha(a);
        m.setting[s6].set_alpha(a);
        m.font.set_alpha(a);
        let y = s0 * 20 + 80;
        m.kanji.dx = from_int(s1 + 14);
        m.kanji.dy = from_int(y);
        m.kanji.make_packet(3);
        m.font.dx = from_int(s1 + 56);
        m.font.dy = from_int(y);
        if s0 < 9 {
            make_num(&mut m.font, 1, 0);
            make_num(&mut m.font, 1, s0 + 1);
        } else {
            make_num(&mut m.font, 2, s0 + 1);
        }
        m.font.make_str(&t.colon);
        if rec[0] != 0 {
            str_cat(&mut buf, &rec[4..], 16);
            m.setting[s6].dx = from_int(s1 + 95);
            m.setting[s6].dy = from_int(y);
            m.setting[s6].make_packet(s5);
            m.font.dx = from_int(s1 + 220);
            m.font.dy = from_int(y);
            m.font.make_str(&lv);
            m.font.dx = from_int(s1 + 250);
            m.font.dy = from_int(y);
            let level = i32::from(rec[1] as i8);
            if level < 10 {
                make_num(&mut m.font, 1, 0);
                make_num(&mut m.font, 1, level);
            } else {
                make_num(&mut m.font, 2, level);
            }
            m.font.dx = from_int(s1 + 282);
            m.font.dy = from_int(y);
            let time = i32::from_le_bytes([rec[24], rec[25], rec[26], rec[27]]);
            if time < TIME_LIMIT {
                let h = time / 216_000;
                let rem = time - h * 216_000;
                let mut mm = rem / 3600;
                let mut ss = (rem - mm * 3600) / 60;
                if mm < 10 {
                    mm += 100;
                }
                if ss < 10 {
                    ss += 100;
                }
                make_num(&mut m.font, 3, h);
                m.font.make_str(&t.colon);
                make_num(&mut m.font, 2, mm);
                m.font.make_str(&t.colon);
                make_num(&mut m.font, 2, ss);
            } else {
                m.font.make_str(&t.max_time);
            }
        } else {
            str_cat(&mut buf, t.record_str.get(32..).unwrap_or(&[]), 16);
            m.setting[s6].dx = from_int(s1 + 95);
            m.setting[s6].dy = from_int(y);
            m.setting[s6].make_packet(s5);
        }
        if s5 == 7 {
            m.setting_text[s6] = std::mem::take(&mut buf);
            s6 += 1;
        }
    }
    if 12 & 7 != 0 {
        m.setting_text[s6] = buf;
    }
}
