//! `ccMessage` (message.cpp, main executable): the event scripts' speech and
//! information windows as the desktop shows them (`docs/engine/desktop.md`,
//! "Messages"). The desktop's instance is `ccMsg`, made by `ccDtMenu` on the
//! menu layer (242, `SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)`). The event
//! task calls `Change` / `ChangeInfo`, then `Check` once a frame;
//! `ccDtMenu::Disp` calls `Disp`. The window is a `ccMenuWindow` cutting cells
//! from `xwindow::TEX_xwindo00`; text is `ccKanji` kt 2.

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_draw::TexRef;

use crate::anm::Ctx;
use crate::kanji::{Fonts, Kanji, Kt, Names, extended_code, str_width};
use crate::sprite::Sprite;
use crate::view::LayerView;
use crate::{Request, Se};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

/// `dtMenu->menuLayer`: `ccLayer::Init(242, 0)`.
pub const MENU_LAYER: i16 = 242;
/// The window texture's file and chunks (`ccInitMenuWindow`).
pub const WINDOW_FILE: &str = "xwindow";
pub const WINDOW_TEX: &str = "TEX_xwindo00";
pub const WINDOW_CLUT: &str = "CLT_xwindo00";
/// `type`: the speech window and the information window.
pub const TYPE_SPEECH: i16 = 5;
pub const TYPE_INFO: i16 = 6;
/// Where `Change` and `ChangeInfo` put the window (`dx`, `dy`).
pub const SPEECH_POS: (f32, f32) = (39.0, 328.0);
pub const INFO_POS: (f32, f32) = (39.0, 240.0);
/// `SetMsg`: a line of this many glyphs or more is dropped.
pub const MAX_GLYPHS: i32 = 73;
/// Alpha steps of the fades: in by 24, out by 28.
pub const FADE_IN: i16 = 24;
pub const FADE_OUT: i16 = 28;
/// Frames `Check` waits before it reads the pad, and after the confirm
/// button before it answers.
pub const WAIT: i16 = 10;
pub const WAIT_CLOSE: i16 = 2;
/// Sound effects: the window closing, an answer moving.
pub const SE_CLOSE: Se = Se(18);
pub const SE_SELECT: Se = Se(17);
/// Frames a glyph takes to type (`Disp`: the emode 0x200 test gives 1
/// either way).
pub const TYPE_FRAMES: i16 = 1;
/// Window cells a row wide and three rows high (`DispSquare(29, 3)`).
pub const SQUARE_W: i32 = 29;
pub const SQUARE_H: i32 = 3;

/// `ccMenuWindow::SetType` grids (wu, wv in 1/16 texels; cell w, h; cells a
/// row).
pub mod grid {
    pub const TYPE_1: (i32, i32, i32, i32, i32) = (0, 0, 14, 16, 18);
    pub const NAME: (i32, i32, i32, i32, i32) = (2816, 1344, 14, 16, 4);
    /// The "next" indicator's cell in grid type 1.
    pub const PAGE_CURSOR: u8 = 24;
}

/// What a message window is (the event's instruction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    /// `message`: the speech window, typed out.
    Speech,
    /// `info`: the information window, shown at once, centred.
    Info,
    /// `info_now`: as `Info`, with no frame drawn (the event zeroes the
    /// window's status after `ChangeInfo`).
    InfoNow,
}

/// One draw call of `Disp`, in order: a window cell, a line of text, or the
/// window's `SendPacket`.
#[derive(Clone, Debug, PartialEq)]
pub enum MsgDraw {
    Cell {
        code: u8,
        dx: f32,
        dy: f32,
        sx: f32,
        sy: f32,
        grid: (i32, i32, i32, i32, i32),
        alpha: u8,
    },
    /// A line of text at (dx, dy); a `centred` one is centred on dx by its
    /// width in the fonts it is drawn with.
    Text {
        line: usize,
        text: Vec<u8>,
        count: i32,
        dx: f32,
        dy: f32,
        rgba: [u8; 4],
        centred: bool,
    },
    Send,
}

/// `ccMessage` and its window's page cursor.
#[derive(Clone, Debug)]
pub struct MsgWindow {
    /// +0x1c, +0x1e: 0 hidden, 1 fading in, 2 up, 4 fading out.
    pub kanji_status: i16,
    pub window_status: i16,
    /// +0x20: 7 after `Close`; 8 and 9 are the save menus' immediate mode.
    pub mode: i16,
    /// +0x22: 5 speech, 6 information.
    pub ty: i16,
    /// +0x24: 0 typing, 1 or 2 waiting for the next line, 4 a question.
    pub cursol: i16,
    str_cnt: i16,
    /// +0x28: the line being typed; 4 when all are shown.
    pub str_line: i16,
    /// +0x2a: glyphs shown per line, -1 for all.
    pub str_clm: [i16; 4],
    /// +0x14: each line's glyph count.
    pub sf: [i16; 4],
    pub kanji_alpha: i16,
    pub window_alpha: i16,
    /// +0x36: the answer under the cursor, 0 or 1.
    pub select: i16,
    pub wait_cnt: i16,
    pub wait_close_cnt: i16,
    /// +0x48: the record's `emode`.
    pub emode: i32,
    /// The lines (0 the name), `None` where the game holds a null pointer.
    lines: [Option<Vec<u8>>; 4],
    dx: f32,
    dy: f32,
    /// +0x18c: the information window's line count.
    ln: i32,
    /// `ccMenuWindow` +0x1ac / +0x1ae: the page cursor's alpha and step.
    page_a: i16,
    page_da: i16,
}

impl Default for MsgWindow {
    /// The constructor (0x001a4580): mode 7, the rest 0.
    fn default() -> Self {
        MsgWindow {
            kanji_status: 0,
            window_status: 0,
            mode: 7,
            ty: 0,
            cursol: 0,
            str_cnt: 0,
            str_line: 0,
            str_clm: [0; 4],
            sf: [0; 4],
            kanji_alpha: 0,
            window_alpha: 0,
            select: 0,
            wait_cnt: 0,
            wait_close_cnt: 0,
            emode: 0,
            lines: [None, None, None, None],
            dx: 0.0,
            dy: 0.0,
            ln: 0,
            page_a: 0,
            page_da: 0,
        }
    }
}

/// Where a [`MsgDraw::Text`] line starts: at dx, or for an information
/// line, `256 - width / 2`.
pub fn text_dx(dx: f32, text: &[u8], centred: bool, fonts: &Fonts, names: &Names) -> f32 {
    if centred { (dx as i32 - str_width(fonts, text, Kt::LargeProportional, names) / 2) as f32 } else { dx }
}

/// `ccKanjiStrlen` (0x0015f120): glyphs in a line, `#0` and `#1` counted as
/// the names they stand for, other `#` escapes as none, `%%` and `%#` as
/// one, other `%` pairs as none.
pub fn str_len(s: &[u8], names: &Names) -> i32 {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut n = 0;
    let mut i = 0;
    while at(i) != 0 {
        let c = at(i);
        if c == b'#' {
            match at(i + 1) {
                b'0' => n += str_len(&names.name, names),
                b'1' => n += str_len(&names.real, names),
                _ => {}
            }
            i += 2;
        } else if c == b'%' {
            if at(i + 1) == b'%' || at(i + 1) == b'#' {
                n += 1;
            }
            i += 2;
        } else if (0x20..0x80).contains(&c) {
            n += 1;
            i += 1;
        } else {
            let e = extended_code(u16::from(c) << 8 | u16::from(at(i + 1)));
            i += if (e >> 8) & 0xff == 0x25 { 2 } else { 1 };
            n += 1;
        }
    }
    n
}

/// The cursor state an emode leaves once its text is out (`Check`).
fn cursol_for(emode: i32, old: i16) -> i16 {
    match emode & 0xff {
        0 => 2,
        1 | 2 => 1,
        3 => 4,
        _ => old,
    }
}

impl MsgWindow {
    /// `Change(data, name, grp, msg)` (0x001a54a0): a speech record. The
    /// window reopens unless a speech window is up; the text restarts.
    /// `lines` are the record's three lines (`ccKanjiStrSeparate` 0..2).
    pub fn change(&mut self, emode: i32, name: Option<&[u8]>, lines: [Option<&[u8]>; 3], names: &Names) {
        if !(self.ty == TYPE_SPEECH && self.window_status == 2) {
            self.ty = TYPE_SPEECH;
            self.window_status = 1;
            self.window_alpha = -FADE_IN;
        }
        self.kanji_status = 1;
        self.kanji_alpha = -FADE_IN;
        self.wait_cnt = WAIT;
        self.wait_close_cnt = 0;
        self.set_msg(emode, name, lines, names);
        (self.dx, self.dy) = SPEECH_POS;
    }

    /// `SetMsg(data, name)` (0x001a51b0): line 0 the name (shown whole),
    /// lines 1-3 the text, typed from nothing unless emode has 0x100.
    fn set_msg(&mut self, emode: i32, name: Option<&[u8]>, lines: [Option<&[u8]>; 3], names: &Names) {
        self.str_cnt = 0;
        let start = if emode & 0x100 != 0 {
            self.str_line = 4;
            -1
        } else {
            self.str_line = if name.is_some() { 1 } else { 0 };
            0
        };
        self.emode = emode;
        for i in 0..4 {
            let (src, clm) = match i {
                0 => (name, -1),
                _ => (lines[i - 1], start),
            };
            match src {
                Some(s) => {
                    let n = str_len(s, names);
                    self.str_clm[i] = clm;
                    if n < MAX_GLYPHS {
                        self.sf[i] = n as i16;
                        self.lines[i] = Some(s.to_vec());
                    } else {
                        self.sf[i] = 0;
                        self.lines[i] = None;
                    }
                }
                None => {
                    self.sf[i] = 0;
                    self.str_clm[i] = 0;
                    self.lines[i] = None;
                }
            }
        }
        self.cursol = 0;
    }

    /// `Open(ccMsgData, name, grp, msg)` (0x001a53e0): a speech window
    /// opened afresh: mode 7, both statuses 1 from alpha 0.
    pub fn open(&mut self, emode: i32, name: Option<&[u8]>, lines: [Option<&[u8]>; 3], names: &Names) {
        self.open_fresh();
        self.set_msg(emode, name, lines, names);
        (self.dx, self.dy) = SPEECH_POS;
    }

    fn open_fresh(&mut self) {
        self.mode = 7;
        self.ty = TYPE_SPEECH;
        self.window_status = 1;
        self.window_alpha = 0;
        self.kanji_status = 1;
        self.kanji_alpha = 0;
        self.wait_cnt = WAIT;
        self.wait_close_cnt = 0;
    }

    /// `Open(ccEvMsgData, name, grp, msg)` (0x001a5790): as [`Self::open`]
    /// with an event record (`text` its lines, `None` for a record without
    /// text, which the game swaps for `errorData`).
    pub fn open_record(&mut self, emode: i32, name: Option<&[u8]>, text: &[&[u8]], names: &Names) {
        self.open_fresh();
        self.set_msg_record(emode, name, text, names);
        (self.dx, self.dy) = SPEECH_POS;
    }

    /// `Change(ccEvMsgData, name, grp, msg)` (0x001a5860): as
    /// [`Self::change`] with an event record.
    pub fn change_record(&mut self, emode: i32, name: Option<&[u8]>, text: &[&[u8]], names: &Names) {
        if !(self.ty == TYPE_SPEECH && self.window_status == 2) {
            self.ty = TYPE_SPEECH;
            self.window_status = 1;
            self.window_alpha = -FADE_IN;
        }
        self.kanji_status = 1;
        self.kanji_alpha = -FADE_IN;
        self.wait_cnt = WAIT;
        self.wait_close_cnt = 0;
        self.set_msg_record(emode, name, text, names);
        (self.dx, self.dy) = SPEECH_POS;
    }

    /// `SetMsg(ccEvMsgData, name)` (0x001a5580): typing starts on line 1
    /// unless there is neither a name nor text; an empty line is a null
    /// line.
    fn set_msg_record(&mut self, emode: i32, name: Option<&[u8]>, text: &[&[u8]], names: &Names) {
        self.str_cnt = 0;
        let first_empty = text.first().is_none_or(|l| l.is_empty());
        let start = if emode & 0x100 != 0 {
            self.str_line = 4;
            -1
        } else {
            self.str_line = if name.is_some() || !first_empty { 1 } else { 0 };
            0
        };
        self.emode = emode;
        match name {
            Some(s) => {
                let n = str_len(s, names);
                self.str_clm[0] = -1;
                if n < MAX_GLYPHS {
                    self.sf[0] = n as i16;
                    self.lines[0] = Some(s.to_vec());
                } else {
                    self.sf[0] = 0;
                    self.lines[0] = None;
                }
            }
            None => {
                self.sf[0] = 0;
                self.str_clm[0] = 0;
                self.lines[0] = None;
            }
        }
        for i in 1..4 {
            match text.get(i - 1).filter(|l| !l.is_empty()) {
                Some(s) => {
                    let n = str_len(s, names);
                    self.str_clm[i] = start;
                    if n < MAX_GLYPHS {
                        self.sf[i] = n as i16;
                        self.lines[i] = Some(s.to_vec());
                    } else {
                        self.sf[i] = 0;
                        self.lines[i] = None;
                    }
                }
                None => {
                    self.sf[i] = 0;
                    self.str_clm[i] = 0;
                    self.lines[i] = None;
                }
            }
        }
        self.cursol = 0;
    }

    /// `DispMsg(ccMsgData, name)` (0x001a5d70): a help line a menu shows
    /// while it keeps calling this every frame (mode 8; `Disp` turns it to
    /// 9, and the frame after it closes).
    pub fn disp_msg(&mut self, emode: i32, name: Option<&[u8]>, lines: [Option<&[u8]>; 3], names: &Names) {
        if self.mode == 9 {
            self.set_msg(emode, name, lines, names);
        } else {
            self.open(emode, name, lines, names);
        }
        self.mode = 8;
        (self.dx, self.dy) = SPEECH_POS;
    }

    /// `DispMsg(ccEvMsgData, name)` (0x001a5de0).
    pub fn disp_msg_record(&mut self, emode: i32, name: Option<&[u8]>, text: &[&[u8]], names: &Names) {
        if self.mode == 9 {
            self.set_msg_record(emode, name, text, names);
        } else {
            self.open_record(emode, name, text, names);
        }
        self.mode = 8;
        (self.dx, self.dy) = SPEECH_POS;
    }

    /// `DispInfo(l0, l1, l2, l3)` (0x001a60e0): an information window kept
    /// up the same way.
    pub fn disp_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        if self.mode == 9 {
            self.set_info(lines, names);
        } else {
            self.open_info(lines, names);
        }
        self.mode = 8;
        (self.dx, self.dy) = INFO_POS;
    }

    /// `CloseInstant` (0x001a5190): gone this frame.
    pub fn close_instant(&mut self) {
        self.mode = 7;
        self.kanji_status = 0;
        self.window_status = 0;
    }

    /// `ccMsg->kanjiAlpha = a` (the menus' Select does -32 on a move, so
    /// the help text fades in again).
    pub fn set_kanji_alpha(&mut self, a: i16) {
        self.kanji_alpha = a;
    }

    /// `+0x20 mode`.
    pub fn mode(&self) -> i16 {
        self.mode
    }

    /// Line `k` as it stands (`None` for a null pointer).
    pub fn line(&self, k: usize) -> Option<&[u8]> {
        self.lines.get(k)?.as_deref()
    }

    /// `ChangeInfo(l0, l1, l2, l3, grp, msg)` (0x001a6000): an information
    /// window, its lines shown at once (a null line is `None`).
    pub fn change_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        if !(self.ty == TYPE_INFO && (self.window_status == 1 || self.window_status == 2)) {
            self.ty = TYPE_INFO;
            self.window_status = 1;
            self.window_alpha = -FADE_IN;
        }
        self.kanji_status = 1;
        self.kanji_alpha = -FADE_IN;
        self.wait_cnt = WAIT;
        self.wait_close_cnt = 0;
        self.set_info(lines, names);
        (self.dx, self.dy) = INFO_POS;
    }

    /// `SetInfo(lines)` (0x001a5e50).
    fn set_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        self.emode = 0;
        self.str_line = 4;
        self.ln = 0;
        for (i, l) in lines.iter().enumerate() {
            match l {
                Some(s) => {
                    self.sf[i] = str_len(s, names) as i16;
                    self.lines[i] = Some(s.to_vec());
                    self.ln = i as i32 + 1;
                }
                None => {
                    self.sf[i] = 0;
                    self.lines[i] = None;
                }
            }
        }
    }

    /// `OpenInfo(l0, l1, l2, l3, -1, -1)` (0x001a5f50): an information
    /// window opened afresh (the reset menu's questions): mode 7, both
    /// statuses 1 from alpha 0.
    pub fn open_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        self.change_info(lines, names);
        self.mode = 7;
        self.window_status = 1;
        self.kanji_status = 1;
        self.window_alpha = 0;
        self.kanji_alpha = 0;
    }

    /// Where the window sits (`dx`, `dy`), as the Controller menu moves its
    /// help to (39, 444).
    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.dx = x;
        self.dy = y;
    }

    /// `info_now`: the event zeroes the window after `ChangeInfo`, so only
    /// the text shows.
    pub fn hide_frame(&mut self) {
        self.window_status = 0;
        self.window_alpha = 0;
    }

    /// `Close` (0x001a5150).
    pub fn close(&mut self) {
        if self.window_status == 1 || self.window_status == 2 {
            self.mode = 7;
            self.window_status = 4;
        }
        self.kanji_status = 4;
    }

    /// `Check(0)` (0x001a5960), once a frame from the event: 0 while the
    /// window waits; 1 once it closes, or the answer (1 or 2) to a
    /// question. `push` and `repeat` are `ccSys.pad[0]`'s, `ok` the
    /// decide button.
    pub fn check(&mut self, push: u32, repeat: u32, ok: u32, req: &mut Vec<Request>) -> i32 {
        let mut r = 0;
        if self.wait_cnt != 0 {
            self.wait_cnt -= 1;
            if self.wait_cnt >= 11 {
                self.wait_cnt = 10;
            }
            if self.wait_cnt <= 0 {
                self.wait_cnt = 0;
            }
        }
        if self.wait_close_cnt != 0 {
            self.wait_close_cnt -= 1;
            if self.wait_close_cnt >= 3 {
                self.wait_close_cnt = 2;
            }
            if self.wait_close_cnt <= 0 {
                self.wait_close_cnt = -1;
                self.cursol = 0;
                match self.emode & 0xff {
                    0 => {
                        req.push(Request::Se(SE_CLOSE));
                        self.close();
                        r = 1;
                    }
                    // Chaining to the next record: no Infection record uses it.
                    1 => req.push(Request::Se(SE_CLOSE)),
                    2 => {
                        req.push(Request::Se(SE_CLOSE));
                        r = 1;
                    }
                    3 => {
                        req.push(Request::Se(SE_CLOSE));
                        self.close();
                        r = i32::from(self.select) + 1;
                        self.select = 0;
                    }
                    _ => {}
                }
            }
        }
        if self.wait_cnt != 0 || self.wait_close_cnt != 0 {
            return r;
        }
        if push & ok != 0 {
            req.push(Request::VoiceStop);
            if self.cursol == 0 {
                self.str_clm = [-1; 4];
                self.cursol = cursol_for(self.emode, self.cursol);
            } else {
                self.wait_close_cnt = WAIT_CLOSE;
            }
        } else if self.str_line >= 4 {
            self.cursol = cursol_for(self.emode, self.cursol);
            if self.emode & 0xff == 3 {
                if repeat & piney_input::Buttons::UP.bits() != 0 {
                    req.push(Request::Se(SE_SELECT));
                    self.select -= 1;
                    if self.select < 0 {
                        self.select = 1;
                    }
                }
                if repeat & piney_input::Buttons::DOWN.bits() != 0 {
                    req.push(Request::Se(SE_SELECT));
                    self.select += 1;
                    if self.select >= 2 {
                        self.select = 0;
                    }
                }
            }
        }
        r
    }

    /// A status's alpha step (`Disp`): whether to draw, and the new status.
    fn step(status: &mut i16, alpha: &mut i16, text: bool) -> bool {
        match *status {
            1 => {
                *alpha += FADE_IN;
                if *alpha >= 128 {
                    *alpha = 128;
                    *status = 2;
                }
                true
            }
            2 => {
                if text {
                    *alpha = (*alpha + FADE_IN).clamp(0, 128);
                }
                true
            }
            4 => {
                *alpha -= FADE_OUT;
                if *alpha <= 0 {
                    *alpha = 0;
                    *status = 0;
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// `Disp` (0x001a4730), every frame: the fades, one glyph typed, the
    /// window, the lines, the page cursor, then the window's `SendPacket`.
    pub fn disp(&mut self, frame_rate: u32) -> Vec<MsgDraw> {
        let mut out = Vec::new();
        match self.mode {
            8 => self.mode = 9,
            9 => self.close(),
            _ => {}
        }
        let draw_window = Self::step(&mut self.window_status, &mut self.window_alpha, false);
        let draw_text = Self::step(&mut self.kanji_status, &mut self.kanji_alpha, true);
        let (x, y) = (self.dx, self.dy);
        let wa = self.window_alpha.clamp(0, 255) as u8;
        if draw_window {
            if self.ty == TYPE_SPEECH {
                if self.lines[0].is_some() {
                    square_name(&mut out, x, y, SQUARE_W, SQUARE_H, wa);
                } else {
                    square(&mut out, x, 8.0 + (16.0 + y), SQUARE_W, SQUARE_H, wa);
                }
            } else {
                let top = y - 32.0 - (20 * self.ln) as f32;
                square(&mut out, x, top, SQUARE_W, self.ln, wa);
            }
        }
        if draw_text {
            if self.ty == TYPE_SPEECH {
                self.type_on();
                let last = self.str_line.min(3) as usize;
                for i in 0..=last {
                    if self.sf[i] == 0 || self.str_clm[i] == 0 || self.kanji_alpha == 0 {
                        continue;
                    }
                    let Some(text) = self.lines[i].clone() else { continue };
                    let dy = (if i == 0 { 6 } else { 14 }) as f32 + (y + (21 * i as i32) as f32);
                    let mut rgba = if self.cursol == 4 && i32::from(self.select) + 2 == i as i32 {
                        SPRITE_COLOR_TABLE[22]
                    } else {
                        SPRITE_COLOR_TABLE[7]
                    };
                    rgba[3] = self.kanji_alpha as u8;
                    out.push(MsgDraw::Text {
                        line: i,
                        text,
                        count: i32::from(self.str_clm[i]),
                        dx: 28.0 + x,
                        dy,
                        rgba,
                        centred: false,
                    });
                }
            } else {
                let top = crate::eef::to_int(y) - 32 - 20 * self.ln;
                for i in 0..self.ln.max(0) as usize {
                    if self.sf[i] == 0 || self.kanji_alpha == 0 {
                        continue;
                    }
                    let Some(text) = self.lines[i].clone() else { continue };
                    let mut rgba = SPRITE_COLOR_TABLE[7];
                    rgba[3] = self.kanji_alpha as u8;
                    out.push(MsgDraw::Text {
                        line: i,
                        dx: 256.0,
                        text,
                        count: -1,
                        dy: (top + 12 + 21 * i as i32) as f32,
                        rgba,
                        centred: true,
                    });
                }
            }
        }
        match self.cursol {
            1 | 2 => {
                let py = if self.ty == TYPE_SPEECH { 92.0 + y } else { y - 20.0 };
                self.page_cursor(&mut out, 256.0, py, frame_rate);
            }
            // 3 (DispPageCursol(1)) and 4 (DispSelectCursol) are never
            // reached by Infection's records: not ported.
            3 | 4 => {}
            _ => {
                self.page_a = 0;
                self.page_da = 0;
            }
        }
        out.push(MsgDraw::Send);
        out
    }

    /// One glyph more of the line being typed (speech, not emode 0x100).
    fn type_on(&mut self) {
        if self.emode & 0x100 != 0 {
            return;
        }
        if self.str_line >= 4 {
            self.str_line = 4;
            return;
        }
        let l = self.str_line as usize;
        if self.str_clm[l] < 0 {
            self.str_cnt = 0;
            self.str_line += 1;
            return;
        }
        self.str_cnt += 1;
        if self.str_cnt < TYPE_FRAMES {
            return;
        }
        self.str_cnt = 0;
        self.str_clm[l] += 1;
        if self.str_clm[l] >= self.sf[l] {
            self.str_clm[l] = self.sf[l];
            self.str_line += 1;
        }
    }

    /// `ccMenuWindow::DispPageCursol(0)` (0x001baa10): cell 24 of grid 1,
    /// its width and drop following an alpha that climbs by 8 and falls by
    /// 4 (16 and 8 at frame rate 2).
    fn page_cursor(&mut self, out: &mut Vec<MsgDraw>, x: f32, y: f32, frame_rate: u32) {
        use crate::eef::{div, from_int, mul, sub};
        self.page_a += self.page_da;
        if self.page_a <= 0 {
            self.page_da = if frame_rate == 2 { 16 } else { 8 };
            self.page_a = 0;
        } else if self.page_a >= 128 {
            self.page_da = if frame_rate == 2 { -8 } else { -4 };
            self.page_a = 128;
        }
        let a = from_int(i32::from(self.page_a));
        let w = div(mul(20.0, a), 128.0);
        let d = div(mul(6.0, a), 128.0);
        out.push(MsgDraw::Cell {
            code: grid::PAGE_CURSOR,
            dx: sub(x, div(w, 2.0)),
            dy: crate::eef::add(y, d),
            sx: w,
            sy: 20.0,
            grid: grid::TYPE_1,
            alpha: self.page_a as u8,
        });
    }

    /// Whether anything shows.
    pub fn is_up(&self) -> bool {
        self.window_status != 0 || self.kanji_status != 0
    }
}

/// `ccMenuWindow::DispSquare(w, h)` at (x, y): a frame of grid-1 cells,
/// `w` cells of 14 wide inside, the middle `16 + 20 (h - 1)` high.
pub(crate) fn square(out: &mut Vec<MsgDraw>, x: f32, y: f32, w: i32, h: i32, alpha: u8) {
    let inner = (14 * w) as f32;
    let mid = (16 + 20 * (h - 1)) as f32;
    let rows = [(y, 16.0, [2, 6, 3]), (y + 16.0, mid, [8, 1, 9]), (y + 16.0 + mid, 16.0, [4, 7, 5])];
    for (ry, rh, codes) in rows {
        let xs = [(x, 14.0), (x + 14.0, inner), (x + 14.0 + inner, 14.0)];
        for ((cx, cw), code) in xs.into_iter().zip(codes) {
            out.push(MsgDraw::Cell { code, dx: cx, dy: ry, sx: cw, sy: rh, grid: grid::TYPE_1, alpha });
        }
    }
}

/// `ccMenuWindow::DispSquareName(w, h)` at (x, y): the body `16 + 20 h`
/// high from y + 20 and its bottom row, then the name header's two rows.
fn square_name(out: &mut Vec<MsgDraw>, x: f32, y: f32, w: i32, h: i32, alpha: u8) {
    let inner = (14 * w) as f32;
    let body = (16 + 20 * h) as f32;
    let xs = [(x, 14.0), (x + 14.0, inner), (x + 14.0 + inner, 14.0)];
    let rows = [
        (y + 20.0, body, [8, 1, 9], grid::TYPE_1),
        (y + 20.0 + body, 16.0, [4, 7, 5], grid::TYPE_1),
        (y, 16.0, [1, 2, 3], grid::NAME),
        (y + 16.0, 16.0, [4, 5, 6], grid::NAME),
    ];
    for (ry, rh, codes, g) in rows {
        for ((cx, cw), code) in xs.into_iter().zip(codes) {
            out.push(MsgDraw::Cell { code, dx: cx, dy: ry, sx: cw, sy: rh, grid: g, alpha });
        }
    }
}

/// The window's texture, read from `xwindow` once.
#[derive(Clone, Debug)]
pub struct WindowTexture {
    pub tex: TexRef,
    pub tex_h: i32,
}

impl WindowTexture {
    pub fn read(archive: &Archive) -> Option<Self> {
        Self::read_named(archive, WINDOW_FILE, WINDOW_TEX, Some(WINDOW_CLUT))
    }

    /// A texture chunk of a resident file (the palette the named CLUT or
    /// the texture's own).
    pub fn read_named(archive: &Archive, file: &str, tex: &str, clut: Option<&str>) -> Option<Self> {
        let ccs = Ccs::parse(archive.inflate_named(file).ok()?).ok()?;
        let texture = ccs.find_object(tex)?;
        let (textures, _) = piney_data::texture::read(&ccs).ok()?;
        let t = textures.iter().find(|t| t.object == texture)?;
        let clut = clut.and_then(|c| ccs.find_object(c)).unwrap_or(t.clut);
        Some(WindowTexture { tex: TexRef::Ccs { file: file.to_string(), texture, clut }, tex_h: 1 << t.th })
    }
}

/// The menu layer's view: `SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)`.
pub fn menu_view() -> LayerView {
    LayerView::frame(0.0, 0.0, 512.0, 384.0, 1.0, 6.0 / 7.0)
}

/// Draws `Disp`'s calls on the menu layer: the text at once (each
/// `ccKanji::Disp` sends), the window's cells when it sends.
pub fn render(draws: &[MsgDraw], ctx: &mut Ctx, fonts: &Fonts, names: &Names, window: Option<&WindowTexture>) {
    let view = menu_view();
    let mut sprite = window.map(|w| Sprite::mask(MENU_LAYER, w.tex.clone(), w.tex_h, 64));
    for d in draws {
        match d {
            MsgDraw::Cell { code, dx, dy, sx, sy, grid, alpha } => {
                let Some(s) = sprite.as_mut() else { continue };
                let (wu, wv, su, sv, wi) = *grid;
                s.wu = wu;
                s.wv = wv;
                s.su = su;
                s.sv = sv;
                s.wi = wi;
                s.sx = *sx;
                s.sy = *sy;
                s.dx = *dx;
                s.dy = *dy;
                s.colour = [128, 128, 128, *alpha];
                s.make_packet(*code, &view);
            }
            MsgDraw::Text { text, count, dx, dy, rgba, centred, .. } => {
                let mut k = Kanji::init(3, 16);
                k.kt = Kt::LargeProportional;
                k.colour = *rgba;
                k.dx = text_dx(*dx, text, *centred, fonts, names);
                k.dy = *dy;
                ctx.disp_count(fonts, &mut k, MENU_LAYER, &view, text, names, *count);
            }
            MsgDraw::Send => {
                if let Some(s) = sprite.as_mut() {
                    s.send(&mut ctx.layers);
                }
            }
        }
    }
}
