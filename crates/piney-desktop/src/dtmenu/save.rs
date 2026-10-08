//! The save menus after the staff roll (`docs/engine/desktop.md`, "The menu
//! task"): menu 8, `ccDtMenu::SaveSelMenu` (0x0016ef90), the clear data's
//! question over an OK / Cancel dialog, and menu 9, `ccDtMenu::SaveMenu`
//! (0x0016f2c0) with `SaveMenuDisp` (0x0016fe70), which drives `ccSaveSys` as
//! the Data screen does. The ending's `staff_roll` opens menu 8 unless parody
//! mode is on, and waits for the menus to close.

use piney_data::tables::sjis::encode;
use piney_data::volume::Volume;

use super::{C_WHITE, DtMenu, MENU_SAVE, MenuCtx, MenuDraw, Resume};
use crate::kanji::Names;
use crate::savesys::{SaveSys, code};
use crate::{Request, Se};

/// What `SaveMenu`'s `ccSaveSys::StartReq` is given: the `$a1`
/// `ccMenuWindow::InitCursol` left in the task (its loop's 12), not a mode
/// of its own. ccSaveSys reads `operate` only for 1 and 2 (the title's
/// loads), so it saves as the Data screen's 3 does.
pub const OPERATE_SAVE_MENU: i32 = 12;
/// "Data saved.", "Save data created.", "Formatted": sound 74 the first
/// frame one is up.
pub const SAVED_MESSAGES: [u32; 3] = [28, 38, 34];
pub const SE_SAVED: Se = Se(74);
/// The frames `SaveMenu` holds its input after a change (`waitCount`).
pub const WAIT: i16 = 10;
/// The file list's window: 24 cells wide, twelve rows, centred.
pub const FILE_WIDTH: i16 = 24;
/// The play time past which the list shows "999:59:59" (1000 hours).
pub const TIME_LIMIT: i32 = 0x0cdf_e5c4;
/// `menuFont`'s cell width (`ccFont::SetType(1)`).
pub const FONT_W: f32 = 12.0;
/// The colours of a file's row: clear data, parody mode, else white.
pub const C_CLEAR: usize = 6;
pub const C_PARODY: usize = 18;

/// The texts the save menus draw.
#[derive(Clone, Debug, Default)]
pub struct SaveTexts {
    /// The clear data's question, three lines.
    pub clear_flag: [Vec<u8>; 3],
    /// `recordMenuStr`, the rows `Extract` makes of it.
    pub record: Vec<u8>,
    /// Its third row ("Unused"), as `ccKanjiStrcat(buf, recordMenuStr + 32,
    /// 16)` copies it.
    pub unused: Vec<u8>,
    pub colon: Vec<u8>,
    pub lv: Vec<u8>,
    pub time_max: Vec<u8>,
    /// `saveSysMsg` by message number: its four lines
    /// (`ccKanjiStrSeparate` 0..3), or `None` where the entry is null.
    pub messages: Vec<Option<[Vec<u8>; 4]>>,
}

impl SaveTexts {
    /// The volume's (`piney_data::tables::dtmenu`, and `saveSysMsg` from
    /// `piney_data::tables::title`).
    pub fn of(volume: Volume) -> Self {
        use piney_data::tables::{dtmenu, title};
        let messages = title::of(volume)
            .save_sys_msg
            .iter()
            .map(|m| m.map(|lines| [0, 1, 2, 3].map(|k| lines.get(k).map_or(Vec::new(), |l| encode(l)))))
            .collect();
        let q = dtmenu::of(volume).clear_flag;
        SaveTexts {
            clear_flag: [0, 1, 2].map(|k| encode(q[k])),
            record: super::slots(*dtmenu::RECORD),
            unused: super::slots(&dtmenu::RECORD[2..]),
            colon: encode(*dtmenu::COLON),
            lv: encode(*dtmenu::LV),
            time_max: encode(*dtmenu::TIME_MAX),
            messages,
        }
    }
}

/// `dec2str(n, v, buf, 0)` (0x0015cec0) as `ccFont::MakeNum(0, n, v)` calls
/// it: the last `n` digits; when the number fits, its leading zeros but the
/// last become spaces.
pub fn dec2str(n: usize, v: u64) -> Vec<u8> {
    let mut out = vec![b'0'; n];
    let mut v = v;
    for k in (0..n).rev() {
        out[k] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    if v == 0 {
        for c in out.iter_mut().take(n.saturating_sub(1)) {
            if *c != b'0' {
                break;
            }
            *c = b' ';
        }
    }
    out
}

/// `ccKanjiStrcat(buf, s, 16)`: `s` padded to 16 glyphs.
fn strcat16(buf: &mut Vec<u8>, s: &[u8]) {
    super::str_cat(buf, s, 16);
}

impl DtMenu {
    /// `ccMsg->DispInfo(l0, l1, l2, l3)`, noted for the trace.
    fn msg_disp_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        self.note(|| format!("[\"disp_info\",{}]", super::json_lines(&lines)));
        self.msg.disp_info(lines, names);
    }

    /// `ccMsg->DispMsg({0x100, lines}, null)`, noted for the trace.
    fn msg_disp_msg(&mut self, lines: [&[u8]; 3], names: &Names) {
        let l = [Some(lines[0]), Some(lines[1]), Some(lines[2]), None];
        self.note(|| format!("[\"disp_msg\",{}]", super::json_lines(&l)));
        self.msg.disp_msg(0x100, None, [Some(lines[0]), Some(lines[1]), Some(lines[2])], names);
    }

    /// `ccMsg +0x24` set by the save menu.
    fn set_msg_cursol(&mut self, v: i16) {
        self.msg.cursol = v;
        self.msg_cursol = v;
    }

    /// `SaveSelMenu`'s question, every frame: `DispInfo` of
    /// `clearFlagStr[volumeNum - 1]`'s three lines (the fourth null).
    pub(super) fn clear_flag_info(&mut self, names: &Names) {
        let [a, b, c] = self.save_texts.clear_flag.clone();
        self.msg_disp_info([Some(&a), Some(&b), Some(&c), None], names);
    }

    /// The close both save menus end with: `menuNext` -1, the window
    /// closing, every task awake (the flip back after the breath, and for
    /// menu 8 its question once more).
    fn close_save(&mut self, info: bool) {
        self.menu_next = -1;
        self.menu_status = 3;
        if self.still {
            self.asleep = false;
            self.resume = if info { Resume::AfterWakeInfo } else { Resume::AfterWake };
        }
    }

    /// `SaveSelMenu` (0x0016ef90), menu 8: "You now have the Data Flag
    /// for .hack//INFECTION. Save?" over the OK / Cancel dialog (Cancel
    /// second). OK opens menu 9; Cancel or the cancel button closes.
    pub(super) fn save_sel_menu(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        match self.proccess {
            0 => {
                self.rows = Some(self.dialog.clone());
                self.lists[m].select = 0;
                self.proccess = 1;
            }
            1 => {
                self.select(x);
                match self.key(x) {
                    Some(true) if self.lists[m].select != 1 => {
                        self.menu_next = MENU_SAVE;
                        self.lists[MENU_SAVE as usize].prev = self.menu;
                        self.proccess = 0;
                        self.wait_count = 0;
                        self.menu_status = 3;
                        // `InitCursol` with the `$a1` left from the select
                        // test (1): the delay slot sets nothing.
                        self.cursor.init(true);
                    }
                    Some(_) => {
                        self.close_save(true);
                        if self.resume == Resume::AfterWakeInfo {
                            // The rest of the function runs after the
                            // breath.
                            return;
                        }
                    }
                    None => {}
                }
            }
            _ => {}
        }
        self.clear_flag_info(&x.names);
    }

    /// `SaveMenu` (0x0016f2c0), menu 9: `ccSaveSys` from `StartReq` to
    /// `EndReq`, the card slots and the files drawn by `SaveMenuDisp`, the
    /// task's messages (`DispInfo`, the button for one to acknowledge) and
    /// questions (the OK / Cancel dialog), and plain lines (`DispMsg`).
    pub(super) fn save_menu(&mut self, x: &mut MenuCtx) {
        let Some(sys) = x.save_sys.take() else {
            // No desktop: nothing to save with.
            self.close_save(false);
            return;
        };
        self.save_menu_with(x, sys);
        self.save_records = std::array::from_fn(|i| sys.record(i));
        x.save_sys = Some(sys);
    }

    fn save_menu_with(&mut self, x: &mut MenuCtx, sys: &mut SaveSys) {
        let m = self.menu as usize;
        if self.proccess == 0 {
            sys.start_req(OPERATE_SAVE_MENU);
            self.save_task = 1;
            self.menu_status = 0;
            let l = &mut self.lists[m];
            l.disp = 4;
            l.port = sys.port as i16;
            l.file = sys.file_num as i16;
            l.select = l.port;
            l.last = 4;
            self.wait_count = 0;
            self.proccess = 1;
        }
        match self.proccess {
            1 => {
                if self.menu_status == 0 && self.wait_count == 0 {
                    self.exception_disp = 1;
                    self.menu_status = 1;
                    let l = &mut self.lists[m];
                    (l.disp, l.width, l.count) = (4, 11, 2);
                    sys.slot_select_req();
                    self.cursor.init(true);
                    self.proccess = 2;
                }
            }
            2 => {
                self.lists[m].select = self.lists[m].port;
                self.select(x);
                self.lists[m].port = self.lists[m].select;
                match self.key(x) {
                    Some(true) => {
                        self.cursor.init(true);
                        sys.load_info_req(i32::from(self.lists[m].port));
                        self.menu_status = 3;
                        self.lists[m].select = 0;
                        self.wait_count = WAIT;
                        self.proccess = 3;
                    }
                    Some(false) => {
                        self.cursor.init(true);
                        self.proccess = 100;
                    }
                    None => {}
                }
            }
            3 | 6 => {
                if sys.result == code::DONE {
                    self.proccess += 1;
                }
            }
            4 => {
                if self.menu_status == 0 && self.wait_count == 0 {
                    self.exception_disp = 0;
                    let l = &mut self.lists[m];
                    (l.disp, l.width, l.count) = (4, FILE_WIDTH, 12);
                    l.select = l.file;
                    self.cursor.init(true);
                    sys.save_select_req();
                    self.proccess = 5;
                }
            }
            5 => {
                if sys.result == code::SAVE_SELECT {
                    self.exception_disp = 2;
                    self.menu_status = 1;
                    self.lists[m].select = self.lists[m].file;
                    self.select(x);
                    self.lists[m].file = self.lists[m].select;
                    match self.key(x) {
                        Some(true) => {
                            self.cursor.init(true);
                            sys.save_data_req(i32::from(self.lists[m].file));
                            self.menu_status = 3;
                            self.lists[m].select = 0;
                            self.wait_count = WAIT;
                            self.proccess = 6;
                        }
                        Some(false) => {
                            self.cursor.init(true);
                            self.menu_status = 3;
                            self.wait_count = WAIT;
                            self.proccess = 1;
                        }
                        None => {}
                    }
                }
            }
            7 => {
                if self.wait_count == 0 && sys.result == code::DONE {
                    self.proccess = 4;
                }
            }
            100 => {
                sys.end_req();
                self.save_task = 0;
                self.menu_status = 3;
                self.proccess = 101;
            }
            101 if self.menu_status == 0 => {
                self.exception_disp = 0;
                self.close_save(false);
            }
            _ => {}
        }
        if self.wait_count > 0 {
            self.wait_count -= 1;
        } else {
            self.wait_count = 0;
        }
        if self.proccess >= 100 {
            return;
        }
        self.save_messages(x, sys);
    }

    /// `SaveMenu`'s tail while it runs: the task's result read each frame.
    fn save_messages(&mut self, x: &mut MenuCtx, sys: &mut SaveSys) {
        let m = self.menu as usize;
        let r = sys.result;
        let n = r & code::MESSAGE;
        if n == code::BACK && self.proccess != 1 {
            sys.next_proccess(0);
            self.menu_status = 3;
            self.wait_count = WAIT;
            self.proccess = 1;
        }
        if n == code::RELOAD {
            sys.load_info_req(i32::from(self.lists[m].port));
        }
        if SAVED_MESSAGES.contains(&n) && r as i32 != i32::from(self.lists[m].last) {
            x.req.push(Request::Se(SE_SAVED));
            self.wait_count = crate::savesys::saved_hold(self.volume, x.pad);
        }
        if r & (code::ACK | code::ERROR) != 0 {
            if self.menu_status != 0 && self.menu_status != 3 {
                self.menu_status = 3;
                self.wait_count = WAIT;
            }
            if self.wait_count == 0 && x.pad.push.bits() & x.save.ok() != 0 {
                x.req.push(Request::Se(if self.title { super::SE_TITLE_OK } else { super::SE_OK }));
                sys.next_proccess(0);
                self.wait_count = WAIT;
                self.set_msg_cursol(0);
            }
        }
        if r & code::QUESTION != 0 {
            if self.menu_status == 0 && self.wait_count == 0 {
                self.exception_disp = 0;
                self.menu_status = 1;
                let l = &mut self.lists[m];
                (l.disp, l.width, l.count, l.select) = (11, 6, 2, 1);
                self.cursor.init(true);
                self.rows = Some(self.dialog.clone());
            }
            if self.wait_count == 0 {
                self.select(x);
                match self.key(x) {
                    Some(true) => {
                        sys.next_proccess(1 - i32::from(self.lists[m].select));
                        self.menu_status = 3;
                        self.wait_count = WAIT;
                    }
                    Some(false) => {
                        sys.next_proccess(0);
                        self.menu_status = 3;
                        self.wait_count = WAIT;
                    }
                    None => {}
                }
            }
        }
        self.lists[m].last = r as i16;
        if self.wait_count != 0 {
            return;
        }
        let Some(Some(lines)) = self.save_texts.messages.get(SaveSys::message_number(r)).cloned() else { return };
        if r & (code::ACK | code::ERROR | code::QUESTION | code::BUSY) != 0 {
            let l = |i: usize| (!lines[i].is_empty()).then_some(&lines[i][..]);
            self.msg_disp_info([l(0), l(1), l(2), l(3)], &x.names);
            if r & (code::ACK | code::ERROR) != 0 {
                self.set_msg_cursol(1);
            } else if r & code::QUESTION == 0 {
                self.set_msg_cursol(3);
            }
        } else if self.menu_status != 0 {
            self.msg_disp_msg([&lines[0], &lines[1], &lines[2]], &x.names);
        }
    }

    /// The save menus' words for the checks: `waitCount`, `exceptionDisp`,
    /// list 9's select, card slot, file, last result, disp, width and
    /// count, the task's state and `ccMsg +0x24` as the menu set it.
    pub fn save_state(&self) -> [i32; 12] {
        let l = &self.lists[MENU_SAVE as usize];
        [
            i32::from(self.wait_count),
            i32::from(self.exception_disp),
            i32::from(l.select),
            i32::from(l.port),
            i32::from(l.file),
            i32::from(l.last),
            i32::from(l.disp),
            i32::from(l.width),
            i32::from(l.count),
            i32::from(self.lists[super::MENU_SAVE_SELECT as usize].select),
            i32::from(self.msg_cursol),
            i32::from(self.save_task),
        ]
    }

    /// `SaveMenuDisp` (0x0016fe70): the card slots (exceptionDisp 1) or
    /// the files (2), with the menu window's alpha.
    pub(super) fn draw_save(&mut self, f: &mut MenuDraw) {
        let l = self.lists[MENU_SAVE as usize].clone();
        let a = self.alpha;
        match self.exception_disp {
            1 => {
                // menuKanji: recordMenuStr's rows; MEMORY CARD and the
                // slot's name on each row.
                self.rows = Some(self.save_texts.record.clone());
                Self::square(f, 179.0, 136.0, i32::from(l.width), i32::from(l.count), a, C_WHITE);
                for s in 0..l.count.max(0) {
                    let y = f32::from(152 + 20 * s);
                    if l.port == s {
                        self.cursor_cells(f, 179.0, y, 11, s);
                    }
                    f.rows.push((4, 193.0, y, C_WHITE));
                    f.rows.push((s as u8, 289.0, y, C_WHITE));
                }
            }
            2 => {
                let x0 = 256 - 7 * (i32::from(l.width) + 2);
                let xf = x0 as f32;
                Self::square(f, xf, 80.0, i32::from(l.width), i32::from(l.count), a, C_WHITE);
                let mut buf = Vec::new();
                let mut set = 0usize;
                for s in 0..12i16 {
                    let row = (s % 8) as u8;
                    if row == 0 {
                        buf.clear();
                    }
                    let y = f32::from(96 + 20 * s);
                    if l.file == s {
                        self.cursor_cells(f, xf, y, i32::from(l.width), s);
                    }
                    let rec = self.save_records[s as usize];
                    let colour = if i32::from(rec.clear_flag) >= self.volume.number() {
                        C_CLEAR
                    } else if rec.parody_flag != 0 {
                        C_PARODY
                    } else {
                        C_WHITE
                    };
                    // The file's number and ":".
                    let mut fx = xf + 14.0;
                    let put = |f: &mut MenuDraw, fx: &mut f32, t: Vec<u8>| {
                        let n = t.len() as f32;
                        f.font.push((t, *fx, y, colour));
                        *fx += FONT_W * n;
                    };
                    put(f, &mut fx, dec2str(2, (s + 1) as u64));
                    put(f, &mut fx, self.save_texts.colon.clone());
                    if rec.status != 0 {
                        strcat16(&mut buf, rec.name());
                        f.sets.push((set, row, xf + 56.0, y, colour));
                        let mut fx = xf + 182.0;
                        put(f, &mut fx, self.save_texts.lv.clone());
                        let lv = i32::from(rec.level);
                        put(f, &mut fx, dec2str(if lv < 10 { 1 } else { 2 }, lv as u32 as u64));
                        let mut fx = xf + 238.0;
                        let t = rec.playtime;
                        if t < TIME_LIMIT {
                            let h = t / 216_000;
                            let r = t - h * 216_000;
                            let mut mi = r / 3600;
                            let mut se = (r - mi * 3600) / 60;
                            if mi < 10 {
                                mi += 100;
                            }
                            if se < 10 {
                                se += 100;
                            }
                            put(f, &mut fx, dec2str(3, h as u32 as u64));
                            put(f, &mut fx, self.save_texts.colon.clone());
                            put(f, &mut fx, dec2str(2, mi as u32 as u64));
                            put(f, &mut fx, self.save_texts.colon.clone());
                            put(f, &mut fx, dec2str(2, se as u32 as u64));
                        } else {
                            put(f, &mut fx, self.save_texts.time_max.clone());
                        }
                    } else {
                        strcat16(&mut buf, &self.save_texts.unused);
                        f.sets.push((set, row, xf + 56.0, y, colour));
                    }
                    if row == 7 {
                        f.set_rows[set] = buf.clone();
                        set += 1;
                    }
                }
                if 12 % 8 != 0 {
                    f.set_rows[set] = buf;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::dec2str;

    #[test]
    fn dec2str_pads_as_make_num() {
        assert_eq!(dec2str(2, 1), b" 1");
        assert_eq!(dec2str(2, 12), b"12");
        assert_eq!(dec2str(3, 0), b"  0");
        assert_eq!(dec2str(2, 105), b"05");
        assert_eq!(dec2str(1, 7), b"7");
    }
}
