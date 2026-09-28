//! `NameEntry_Control` (desktop.prg, NameEntry.cpp 0x00414310 - 0x0041b5b0):
//! the new game's name entry, which event 1 opens while the desktop's setup
//! runs (`ccEvent::Execute` case 156, 0x001b2070). [`NameEntry::new`] is the
//! construction (`Init` 0x004195b0), [`NameEntry::step`] one `Main`
//! (0x00414310). The US build reaches the information pages, the English
//! grid for the user and character names, the confirmation and `SetSaveData`
//! (`plName`, `plRealName`), all on layer 131. Texts, the grid and the
//! forbidden names are read from the program image (docs/engine/desktop.md).

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_data::tables::nameentry as ne;
use piney_data::tables::sjis::encode;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_draw::{Frame, TexRef};
use piney_input::{Buttons, Pad};

use crate::anm::Ctx;
use crate::assets::{FONT_FILE, read_fonts};
use crate::eef;
use crate::kanji::{Fonts, Kanji, Kt, Names};
use crate::message::{MsgDraw, WindowTexture, square};
use crate::save::{SaveState, offset};
use crate::sprite::Sprite;
use crate::view::{LayerView, View};
use crate::{FRAME_RATE, Request, Se};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

/// `ccLayer::Init(131, NULL)`: the entry's layer, framed as the default.
pub const LAYER: i16 = 131;
/// `fontLayer` (`fontInit`: `ccLayer::Init(240, NULL)`), where `fontSend`
/// sends the system font's packets (`fontDef`) every frame.
pub const FONT_LAYER: i16 = 240;

/// The face panel's scene file (`ccFacePanel::ccFacePanel` 0x001bb830);
/// its picture is the volume's (`piney_data::tables::nameentry`: Kite
/// before the bracelet on Infection, with it after).
pub const FACE_FILE: &str = "xwin_f00";

/// The system font's texture (`fontSetup`).
pub const FONT_TEX: &str = "TEX_xasc00";

/// Bytes of a block row: seven two-byte cells (the copy loop's count).
pub const ROW_BYTES: usize = 14;
/// Grid rows drawn (`List[18]`), three blocks of five cells a row.
pub const LIST_ROWS: usize = 18;
/// `StrMaxNum` for each name: `MyName` (not reached), the user name, the
/// character name; and the default character name's length (`SelName`'s
/// Default puts `NameNum` at 4).
pub const MY_MAX: i16 = 7;
pub const USER_MAX: i16 = 18;
pub const GAME_MAX: i16 = 14;
pub const DEFAULT_GAME_LEN: i16 = 4;
/// `m_Max` on the grid.
pub const GRID_MAX: (i32, i32) = (14, 5);
/// `m_SubTrans`: the fades' step.
pub const TRANS_STEP: i32 = 10;
/// `CurRepeat`: frames of a held key before it repeats, then between.
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
/// `NameSelSet`: the `_` blinks every this many frames plus one.
pub const BLINK: i32 = 30;
/// `ccSpcParam` +0x24, +0x26: the face panel's HP and SP bars read them.
pub const SPC_MAX_HP: usize = 0x24;
pub const SPC_MAX_SP: usize = 0x26;
/// The bars' width.
pub const BAR_W: i32 = 56;

/// Sound effects.
pub const SE_MOVE: Se = Se(6);
pub const SE_OK: Se = Se(4);
pub const SE_BACK: Se = Se(7);
pub const SE_DIALOG_MOVE: Se = Se(17);
/// The dialog's YES / NO decided, an information page turned.
pub const SE_DIALOG_OK: Se = Se(18);
pub const SE_DIALOG_CANCEL: Se = Se(19);

/// `SetALLPos` (0x0041a350).
pub mod pos {
    pub const INFO: (i32, i32) = (120, 170);
    pub const LIST: (i32, i32) = (70, 145);
    pub const MSG_INFO: (i32, i32) = (70, 315);
    pub const ENTER_END: (i32, i32) = (124, 270);
    pub const F_AFF: (i32, i32) = (150, 200);
    pub const NAME_INFO: (i32, i32) = (150, 50);
    pub const NAME_PREV: (i32, i32) = (234, 24);
    pub const FACE: (i32, i32) = (50, 14);
    /// `m_hi`: a grid row's height.
    pub const HI: i32 = 20;
}

/// `ccMenuWindow::SetType` grids (wu, wv in 1/16 texels; cell w, h; cells
/// a row) and the others the entry's windows cut.
pub mod grid {
    pub type Grid = (i32, i32, i32, i32, i32);
    pub const TYPE_1: Grid = (0, 0, 14, 16, 18);
    pub const TYPE_2: Grid = (0, 768, 16, 16, 16);
    pub const BAR: Grid = (1536, 1536, 16, 20, 3);
    /// The name frame round the name being entered (`DrawWindow`).
    pub const WAKU: Grid = (0, 3456, 16, 16, 4);
    /// `DispTarget(8, 1)`: the panel, the tab and its end.
    pub const TARGET: Grid = (1024, 3072, 90, 44, 2);
    pub const TAB: Grid = (0, 1536, 16, 24, 6);
    pub const TAB_END: Grid = (640, 1536, 8, 24, 1);
    /// The face panel's HP / SP boxes, captions, and bar fill.
    pub const BOX: Grid = (1088, 2048, 8, 16, 2);
    pub const CAPTION: Grid = (0, 2048, 84, 16, 1);
    pub const FILL: Grid = (1344, 2048, 8, 8, 1);
    /// `ccFont::SetType(2)`: 10 x 12 cells from (128, 96), 12 a row.
    pub const FONT_2: Grid = (2048, 1536, 10, 12, 12);
}

/// C's `char *` view of a buffer: up to its first NUL.
fn cstr(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}

/// `strcpy` into a fixed buffer.
fn strcpy(dst: &mut [u8], src: &[u8]) {
    let s = cstr(src);
    let n = s.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&s[..n]);
    if n < dst.len() {
        dst[n] = 0;
    }
}

/// `StrMaxNum - strlen(strstr(name, "_"))`: where the first `_` is when the
/// name is padded to `max` (and `max` when there is none).
fn first_us(name: &[u8], max: i16) -> i16 {
    let s = cstr(name);
    let tail = s.iter().position(|&c| c == b'_').map_or(0, |i| s.len() - i);
    max - tail as i16
}

/// A name buffer (`char[20]`): `head`, then `_` up to `n`, then NUL.
fn padded(head: &[u8], n: usize) -> [u8; 20] {
    let mut b = [0u8; 20];
    strcpy(&mut b, head);
    for c in b.iter_mut().take(n).skip(cstr(head).len()) {
        *c = b'_';
    }
    b[n.min(19)] = 0;
    b
}

/// The entry's texts, alike on the four volumes
/// (`piney_data::tables::nameentry`), as the Shift-JIS it draws.
#[derive(Clone, Debug)]
struct Texts {
    /// `InfoMsg[0..5]`, lines 0 and 1 (`ccKanjiStrSeparate`).
    info: Vec<[Vec<u8>; 2]>,
    /// `ResetMsg[NameAct]`.
    reset: [Vec<u8>; 3],
    /// `StrMsg[0..4]`, lines 0..4 as `ccKanjiStrSeparate` finds them.
    str_msg: [[Vec<u8>; 4]; 4],
    all_reset: Vec<u8>,
    enter: Vec<u8>,
    affirmation: [Vec<u8>; 2],
    entry_name: Vec<u8>,
    entry_game_name: Vec<u8>,
    yes: Vec<u8>,
    no: Vec<u8>,
    /// `NameInfoBlock[NameAct]`, lines 1 and 2.
    labels: [[Vec<u8>; 2]; 3],
    /// `MenuBlockE[CurAct]`.
    menu: [Vec<u8>; 4],
    forbidden: Vec<Vec<u8>>,
    default_game: Vec<u8>,
    dashes: Vec<u8>,
}

impl Texts {
    fn new() -> Texts {
        let e = |s: &str| encode(s);
        let lines = |l: &[&str], k: usize| l.get(k).map_or_else(Vec::new, |s| encode(s));
        let lines4 = |l: &[&str]| [lines(l, 0), lines(l, 1), lines(l, 2), lines(l, 3)];
        Texts {
            info: ne::INFO.iter().map(|l| [lines(l, 0), lines(l, 1)]).collect(),
            reset: [e(ne::RESET[0]), e(ne::RESET[1]), e(ne::RESET[2])],
            str_msg: [lines4(ne::STR_MSG[0]), lines4(ne::STR_MSG[1]), lines4(ne::STR_MSG[2]), lines4(*ne::ERR_NAME)],
            all_reset: e(*ne::ALL_RESET),
            enter: e(*ne::ENTER),
            affirmation: [lines(*ne::AFFIRMATION, 0), lines(*ne::AFFIRMATION, 1)],
            entry_name: e(*ne::ENTRY_NAME),
            entry_game_name: e(*ne::ENTRY_GAME_NAME),
            yes: e(*ne::YES),
            no: e(*ne::NO),
            labels: [0, 1, 2].map(|i| [lines(ne::NAME_INFO[i], 1), lines(ne::NAME_INFO[i], 2)]),
            menu: [0, 1, 2, 3].map(|i| e(ne::MENU[i])),
            forbidden: ne::FORBIDDEN.iter().map(|s| e(s)).collect(),
            default_game: e(*ne::DEFAULT_GAME_NAME),
            dashes: e(*ne::FACE_DASHES),
        }
    }

    /// Row `no` of a grid (`char[][14]`): the 14 bytes `SetBufBlock`
    /// copies, its text then NULs. A row past the table reads as NULs:
    /// the game reads the tables' neighbours there, which the cursor never
    /// reaches with anything but NULs to show.
    fn row(grid: &[&str], no: i32) -> Vec<u8> {
        let mut r = usize::try_from(no).ok().and_then(|i| grid.get(i)).map_or_else(Vec::new, |s| encode(s));
        r.resize(ROW_BYTES, 0);
        r
    }

    /// The string at row `s` of a grid (`strcpy` from there).
    fn row_str(grid: &[&str], s: usize) -> Vec<u8> {
        grid.get(s).map_or_else(Vec::new, |s| encode(s))
    }
}

/// The pad as the entry reads it: `ccSys.pad[0]`'s push, unpush and repeat
/// bits, and the save's decide and cancel buttons.
#[derive(Clone, Copy, Debug)]
struct Keys {
    push: u32,
    unpush: u32,
    repeat: u32,
    ok: u32,
    cancel: u32,
}

/// The alphas `AllTlans` writes into the entry's sprites (`color[0].a`),
/// 128 as `ccSprite`'s constructor leaves them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alphas {
    pub info: i32,
    pub menu: i32,
    pub name_prev: i32,
    pub list: i32,
    pub name_info: i32,
    pub real_name: i32,
    pub windows: i32,
    pub waku: i32,
}

impl Default for Alphas {
    fn default() -> Self {
        Alphas {
            info: 128,
            menu: 128,
            name_prev: 128,
            list: 128,
            name_info: 128,
            real_name: 128,
            windows: 128,
            waku: 128,
        }
    }
}

/// `NameEntry_Control`'s members that decide what happens (names by the
/// DWARF), as `Init` (0x004195b0) leaves them.
#[derive(Clone, Debug)]
pub struct Logic {
    /// +0x230 `m_MainAct`, +0x232 `m_TempMainAct`, +0x1e0 `ReserveACT`.
    pub main_act: i16,
    pub temp_main_act: i16,
    pub reserve: i16,
    /// +0x000 `m_NowMode`: 2, the English grid, all the US build uses.
    pub now_mode: i16,
    pub tmp_now_mode: i16,
    /// +0x236 `m_CurAct`: 0 the grid, 1-3 the menu row (Default, One Back,
    /// Enter), 4 the dialog, 5 an information page.
    pub cur_act: i16,
    pub tmp_cur_act: i16,
    /// +0x14c `m_CurPos`, +0x154 `m_Max`, +0x15c `m_Min`, +0x1f0 `m_TmpX`.
    pub x: i32,
    pub y: i32,
    pub max: (i32, i32),
    pub min: (i32, i32),
    pub tmp_x: i32,
    /// +0x23a `m_NameAct`: 1 the user name, 2 the character name.
    pub name_act: i16,
    /// +0x240 `m_Q_Act`.
    pub q: i16,
    /// +0x22c `m_StrMaxNum`.
    pub str_max: i16,
    /// +0x23c `m_WinAct`: 0 a page, 1 the grid, 3 the dialog.
    pub win_act: i16,
    pub tmp_win_act: i16,
    /// +0x220 `m_winFlg`: the name frame pulsing up.
    pub win_flg: bool,
    /// +0x10e `m_NameNum`: characters entered.
    pub name_num: i16,
    /// +0x1f4 `m_dialog`: 1 YES, 0 NO.
    pub dialog: i16,
    /// +0x20c `m_InfoCount`: the page shown.
    pub info_count: i32,
    pub play_lock: i32,
    pub move_lock: i32,
    pub all_lock: i16,
    /// +0x21c `m_Err`: 1 no name, 2 no field back, 3 a forbidden name.
    pub err: i32,
    pub time_count: i32,
    pub switch: i32,
    /// +0x242 `m_Tlans_Act`: the fade's step.
    pub tlans: i16,
    /// +0x1f8 `m_Transparency`, +0x1fc `m_SubTrans`, +0x200 `m_winTrans`.
    pub trans: i32,
    pub sub_trans: i32,
    pub win_trans: i32,
    /// +0x21a `m_NameFlg`: the dialog's answer, 1 YES, -1 NO.
    pub name_flg: i16,
    pub push_flg: i16,
    pub push_cnt: i16,
    pub repeat_count: i16,
    /// +0x148 `m_SW`: 1 decide, -1 cancel this frame.
    pub sw: i16,
    /// +0x004 `m_NO`: the grid row under the cursor.
    pub no: i32,
    /// +0x1e2 `m_cbuf`: the cell under the cursor.
    pub cbuf: Vec<u8>,
    /// `*m_Blockbuf`: the row under the cursor, `#G` round the cell.
    pub blockbuf: Vec<u8>,
    pub my_name: [u8; 20],
    pub sur_name: [u8; 20],
    pub game_name: [u8; 20],
    pub temp_name: [u8; 20],
    pub true_sur_name: [u8; 20],
    pub entry_name: [u8; 38],
    pub entry_game_name: [u8; 38],
    /// +0x16c `m_DispCurPos`: where the cursor goes (set by `KanjiDraw`,
    /// read by the next frame's `DrawCur`).
    pub disp_cur: (i32, i32),
    /// `AllTlans`' writes.
    pub alphas: Alphas,
    /// `ccMenuWindow::InitCursol(0)` on `m_Windows` asked for this frame.
    cursor_reset: bool,
}

impl Logic {
    fn new(default_game: &[u8]) -> Logic {
        let sur = padded(b"", USER_MAX as usize);
        Logic {
            main_act: 0,
            temp_main_act: 0,
            reserve: -1,
            now_mode: 2,
            tmp_now_mode: 0,
            cur_act: 5,
            tmp_cur_act: 0,
            x: 0,
            y: 0,
            max: (0, 0),
            min: (0, 0),
            tmp_x: 0,
            name_act: 1,
            q: 1,
            str_max: USER_MAX,
            win_act: 0,
            tmp_win_act: 0,
            win_flg: true,
            name_num: first_us(&sur, USER_MAX),
            dialog: 0,
            info_count: 0,
            play_lock: 0,
            move_lock: 0,
            all_lock: 1,
            err: 0,
            time_count: 0,
            switch: 1,
            tlans: 2,
            trans: 128,
            sub_trans: TRANS_STEP,
            win_trans: 128,
            name_flg: 0,
            push_flg: 0,
            push_cnt: 0,
            repeat_count: REPEAT_FIRST,
            sw: 0,
            no: 0,
            cbuf: Vec::new(),
            blockbuf: vec![0; ROW_BYTES],
            my_name: padded(b"", MY_MAX as usize),
            sur_name: sur,
            game_name: padded(default_game, GAME_MAX as usize),
            temp_name: [0; 20],
            true_sur_name: [0; 20],
            entry_name: [0; 38],
            entry_game_name: [0; 38],
            disp_cur: (0, 0),
            alphas: Alphas::default(),
            cursor_reset: false,
        }
    }

    /// The name `NameAct` picks.
    fn name_mut(&mut self) -> &mut [u8; 20] {
        match self.name_act {
            0 => &mut self.my_name,
            1 => &mut self.sur_name,
            _ => &mut self.game_name,
        }
    }

    fn name(&self) -> &[u8; 20] {
        match self.name_act {
            0 => &self.my_name,
            1 => &self.sur_name,
            _ => &self.game_name,
        }
    }

    fn set_max(&mut self, mx: i32, my: i32) {
        self.max = (mx, my);
        self.min = (0, 0);
    }

    /// One `Main` (0x00414310) up to the draws: true when it returns 1
    /// from `SetSaveData`.
    fn main(&mut self, k: &Keys, t: &Texts, req: &mut Vec<Request>) -> bool {
        self.change_act();
        match self.main_act {
            // SetStartInfoMsg.
            0 => self.main_act = 1,
            1 => self.play_info(2),
            2 => self.set_hira(),
            3 => self.play_hira(),
            6 => self.set_eigo(),
            // PlayEigo: nothing while NowMode is 2.
            7 => {}
            14 => self.set_name_two(),
            16 => self.set_name_three(),
            17 => self.play_name_three(),
            // SetInfoMsg.
            18 => {
                self.cur_act = 5;
                self.main_act = 19;
                self.win_act = 0;
            }
            19 => self.play_info(5),
            20 => return true,
            // The kana, symbol and kanji grids and the first name (4, 5,
            // 8-13, 15) are not reached by the US build.
            _ => {}
        }
        self.move_cur(k, req);
        self.name_sel_set();
        if self.all_lock == 0 {
            self.judgment(t);
            self.set_buf_block();
            self.sel_name(t);
        }
        self.all_tlans();
        false
    }

    /// `ChangeACT` (0x0041a4a0).
    fn change_act(&mut self) {
        if self.reserve != -1 {
            self.main_act = self.reserve;
            self.reserve = -1;
            self.x = 0;
            self.y = 0;
            self.cur_act = 0;
            self.all_lock = 0;
        }
    }

    /// `PlayStartInfoMsg` (0x0041adf0, pages 0-1) and `PlayInfoMsg`
    /// (0x0041aeb0, pages 2-4): a decide fades the page out, and the next
    /// comes up as it fades in.
    fn play_info(&mut self, end: i32) {
        match self.sw {
            -1 => self.sw = 0,
            1 => self.tlans = 1,
            _ => {}
        }
        if self.tlans == 3 {
            self.info_count += 1;
        }
        if end == 2 {
            if self.info_count >= 2 {
                self.tlans = 2;
                self.main_act = 2;
                self.win_act = 1;
                self.cur_act = 0;
                self.set_max(GRID_MAX.0, GRID_MAX.1);
            }
        } else if self.info_count >= end {
            self.main_act = 20;
        }
    }

    /// `SetHira` (0x0041aff0).
    fn set_hira(&mut self) {
        self.main_act = 3;
        if self.all_lock != 0 {
            self.all_lock = 0;
        }
        self.set_max(GRID_MAX.0, GRID_MAX.1);
    }

    /// `PlayHira` (0x0041b030): straight on to the mode's own grid.
    fn play_hira(&mut self) {
        if self.now_mode != 0 {
            if let Some(&r) = [2, 4, 6, 8, 10].get(self.now_mode as usize) {
                self.reserve = r;
            }
            self.all_lock = 1;
        }
    }

    /// `SetEigo` (0x0041b0c0).
    fn set_eigo(&mut self) {
        if self.all_lock != 0 {
            self.all_lock = 0;
        }
        self.main_act = 7;
        self.set_max(GRID_MAX.0, GRID_MAX.1);
    }

    /// The mode's own `Set*` after a name changes (only `SetEigo` is
    /// reached).
    fn set_mode(&mut self) {
        match self.now_mode {
            0 => self.set_hira(),
            2 => self.set_eigo(),
            _ => {}
        }
    }

    /// `SetNameTwo` (0x0041a870): the user name taken, on to the
    /// character name.
    fn set_name_two(&mut self) {
        self.dialog = 0;
        let n = self.name_num.clamp(0, 20) as usize;
        self.sur_name[0] = 0;
        let temp = self.temp_name;
        // strncpy(SurName, TempName, NameNum): no NUL added.
        for (i, c) in self.sur_name.iter_mut().enumerate().take(n) {
            *c = if i < cstr(&temp).len() { temp[i] } else { 0 };
        }
        let sur = self.sur_name;
        strcpy(&mut self.true_sur_name, &sur);
        self.cursor_reset = true;
        self.cur_act = 0;
        self.x = 0;
        self.y = 0;
        self.info_count = 2;
        self.str_max = GAME_MAX;
        self.name_num = first_us(&self.game_name, self.str_max);
        self.name_act = 2;
        self.q = 2;
        self.main_act = self.temp_main_act;
        self.win_act = self.tmp_win_act;
        self.now_mode = self.tmp_now_mode;
        self.set_mode();
    }

    /// `SetNameThree` (0x0041ab60): both names taken, the dialog.
    fn set_name_three(&mut self) {
        self.tmp_win_act = self.win_act;
        self.main_act = 17;
        self.win_act = 3;
        self.cur_act = 4;
        self.dialog = 0;
        let sur = self.true_sur_name;
        strcpy(&mut self.entry_name, &sur);
        let n = first_us(&sur, USER_MAX).clamp(0, 37) as usize;
        self.entry_name[n] = 0;
        self.str_max = GAME_MAX;
        let game = self.game_name;
        strcpy(&mut self.entry_game_name, &game);
        let n = first_us(&game, GAME_MAX).clamp(0, 37) as usize;
        self.entry_game_name[n] = 0;
    }

    /// `PlayNameThree` (0x0041acc0): YES on to the last pages, NO (or
    /// cancel) back to Enter.
    fn play_name_three(&mut self) {
        match self.sw {
            1 => {
                self.name_flg = if self.dialog != 0 { 1 } else { -1 };
                self.tlans = 1;
            }
            -1 => {
                self.name_flg = -1;
                self.tlans = 1;
                self.sw = 0;
            }
            _ => {}
        }
        if self.name_flg != 0 && self.tlans == 3 {
            self.cursor_reset = true;
            self.cur_act = 0;
            self.x = 0;
            self.y = 0;
            match self.name_flg {
                1 => {
                    self.name_num = 0;
                    self.main_act = 18;
                }
                -1 => {
                    self.main_act = self.temp_main_act;
                    self.win_act = self.tmp_win_act;
                    self.cur_act = 3;
                }
                _ => {}
            }
        }
    }

    /// `CurRepeat(key)` (0x0041a4e0): a push, or a held key once its count
    /// (shared by every key) reaches 30, then every 9 frames.
    fn rep(&mut self, k: &Keys, key: u32) -> bool {
        if k.push & key != 0 {
            self.push_flg = 1;
            return true;
        }
        if k.unpush & key != 0 {
            self.repeat_count = REPEAT_FIRST;
            self.push_flg = 0;
            return false;
        }
        if k.repeat & key != 0 {
            self.push_cnt = self.push_cnt.wrapping_add(1);
            if self.push_cnt == self.repeat_count {
                self.repeat_count = REPEAT_NEXT;
                self.push_cnt = 2;
                return true;
            }
        }
        false
    }

    /// `MoveCur` (0x004145d0).
    fn move_cur(&mut self, k: &Keys, req: &mut Vec<Request>) {
        self.sw = 0;
        if self.play_lock == 1 || self.move_lock == 1 || self.tlans != 0 {
            return;
        }
        match self.cur_act {
            0 => self.normal_move(k, req),
            1..=3 => self.menu_move(k, req),
            // DiaMoveCur (0x00415040).
            4 => {
                if self.rep(k, Buttons::UP.bits()) {
                    self.dialog = 1;
                    req.push(Request::Se(SE_DIALOG_MOVE));
                } else if self.rep(k, Buttons::DOWN.bits()) {
                    self.dialog = 0;
                    req.push(Request::Se(SE_DIALOG_MOVE));
                } else if k.push & k.ok != 0 {
                    self.sw = 1;
                    req.push(Request::Se(SE_DIALOG_OK));
                } else if k.push & k.cancel != 0 {
                    self.sw = -1;
                    req.push(Request::Se(SE_DIALOG_CANCEL));
                }
            }
            // InfoMoveCur (0x00415130).
            5 if k.push & k.ok != 0 => {
                self.sw = 1;
                req.push(Request::Se(SE_DIALOG_OK));
            }
            _ => {}
        }
    }

    /// Off the grid's top or bottom onto the menu row, the item under the
    /// block the cursor was in.
    fn onto_menu(&mut self) {
        let third = (self.max.0 + 1) / 3;
        self.cur_act = if self.x < third {
            1
        } else if self.x < 2 * third {
            2
        } else {
            3
        };
        self.tmp_cur_act = self.cur_act;
        self.tmp_x = self.x;
    }

    /// `NormalMoveCur` (0x004146b0): the grid.
    fn normal_move(&mut self, k: &Keys, req: &mut Vec<Request>) {
        let mut moved = true;
        if self.rep(k, Buttons::UP.bits()) {
            self.y -= 1;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::DOWN.bits()) {
            self.y += 1;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::LEFT.bits()) {
            self.x -= 1;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::RIGHT.bits()) {
            self.x += 1;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, k.ok) {
            self.sw = 1;
            req.push(Request::Se(SE_OK));
        } else if self.rep(k, k.cancel) {
            self.sw = -1;
            req.push(Request::Se(SE_BACK));
        } else {
            moved = false;
        }
        if !moved && self.rep(k, Buttons::START.bits()) {
            self.y = self.max.1;
            self.cur_act = 3;
            req.push(Request::Se(SE_MOVE));
        }
        if self.y > self.max.1 || self.y < self.min.1 {
            self.onto_menu();
        }
        if self.x > self.max.0 {
            self.x = self.min.0;
        }
        if self.x < self.min.0 {
            self.x = self.max.0;
        }
    }

    /// Back onto the grid from the menu row: where the cursor left it, or
    /// the middle of the item's block.
    fn back_x(&mut self) {
        let block = self.max.0 / 3;
        let h = block / 2;
        if self.tmp_cur_act == self.cur_act {
            self.x = self.tmp_x;
        } else {
            match self.cur_act {
                1 => self.x = h,
                2 => self.x = block + h,
                3 => self.x = h + 2 * block,
                _ => {}
            }
        }
    }

    /// `MenuMoveCUr` (0x00414a70): the menu row.
    fn menu_move(&mut self, k: &Keys, req: &mut Vec<Request>) {
        if self.rep(k, Buttons::UP.bits()) {
            self.back_x();
            self.cur_act = 0;
            self.y = self.max.1;
            self.err = 0;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::DOWN.bits()) {
            self.back_x();
            self.cur_act = 0;
            self.y = self.min.1;
            self.err = 0;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::LEFT.bits()) {
            self.cur_act -= 1;
            self.err = 0;
            if self.cur_act <= 0 {
                self.cur_act = 3;
            }
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::RIGHT.bits()) {
            self.cur_act += 1;
            self.err = 0;
            if self.cur_act >= 4 {
                self.cur_act = 1;
            }
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, Buttons::START.bits()) {
            self.err = 0;
            self.cur_act = 3;
            req.push(Request::Se(SE_MOVE));
        } else if self.rep(k, k.ok) {
            self.sw = 1;
            req.push(Request::Se(SE_OK));
        } else if self.rep(k, k.cancel) {
            self.sw = -1;
            req.push(Request::Se(SE_BACK));
        }
    }

    /// `NameSelSet` (0x004152f0): `TempName`, the name being entered with
    /// the next place blinking `_` / space.
    fn name_sel_set(&mut self) {
        if self.play_lock != 0 || self.cur_act == 4 {
            return;
        }
        let old = self.time_count;
        self.time_count += 1;
        if old == BLINK {
            self.time_count = 0;
            self.switch = -self.switch;
        }
        let ch = if self.switch == 1 { b'_' } else { b' ' };
        let name = *self.name();
        strcpy(&mut self.temp_name, &name);
        if self.name_num < self.str_max && (0..20).contains(&self.name_num) {
            self.temp_name[self.name_num as usize] = ch;
        }
    }

    /// `JudgmentProcess` (0x00418960): Enter and One Back.
    fn judgment(&mut self, t: &Texts) {
        let mut refused = false;
        if self.cur_act == 3 && self.sw == 1 {
            if self.name_num <= 0 {
                self.err = 1;
            } else {
                let name = *self.name();
                let n = first_us(&name, self.str_max).clamp(0, 19) as usize;
                let mut tmp = [0u8; 20];
                tmp[..n].copy_from_slice(&name[..n]);
                match check_name(&tmp, self.name_act, &t.forbidden) {
                    2 => self.err = 1,
                    1 => refused = true,
                    _ => {
                        self.temp_main_act = self.main_act;
                        self.tmp_win_act = self.win_act;
                        self.tmp_now_mode = self.now_mode;
                        self.sw = 0;
                        if self.play_lock == 0 {
                            self.tlans = if self.name_act == 2 { 1 } else { 3 };
                            self.play_lock = 1;
                        }
                    }
                }
            }
        } else if self.cur_act == 2 && self.sw == 1 {
            match self.name_act {
                0 | 1 => self.err = 2,
                2 => {
                    self.temp_main_act = self.main_act;
                    self.tmp_win_act = self.win_act;
                    self.tmp_now_mode = self.now_mode;
                    self.sw = 0;
                    let sur = self.sur_name;
                    strcpy(&mut self.temp_name, &sur);
                    if self.play_lock == 0 {
                        self.tlans = 3;
                        self.play_lock = 2;
                    }
                }
                _ => {}
            }
        }
        if refused {
            self.err = 3;
        }
        if self.play_lock == 1 && self.tlans == 3 {
            match self.q {
                0 => self.main_act = 12,
                1 => self.main_act = 14,
                2 => self.main_act = 16,
                _ => {}
            }
            self.play_lock = 0;
        }
        if self.play_lock == 2 && self.tlans == 3 {
            if self.q == 2 {
                self.name_act = 1;
                self.q = 1;
                self.str_max = USER_MAX;
                self.cursor_reset = true;
                self.cur_act = 0;
                self.x = 0;
                self.y = 0;
                self.name_num = first_us(&self.sur_name, USER_MAX);
                self.main_act = self.temp_main_act;
                self.win_act = self.tmp_win_act;
                self.now_mode = self.tmp_now_mode;
            }
            self.set_mode();
            self.play_lock = 0;
        }
    }

    /// `SetBufBlock` (0x00418f10): the row under the cursor into
    /// `m_Blockbuf`, and on the grid `CutInBlock` round its cell.
    fn set_buf_block(&mut self) {
        let base = match self.main_act {
            3 => {
                self.no = self.x / 5 + self.y + 5 * (self.x / 5);
                *ne::HIRA_BLOCK
            }
            7 => {
                self.no = self.x / 5 + 3 * self.y;
                *ne::EIGO_BLOCK
            }
            // The kana, symbol and kanji grids are not reached.
            _ => return,
        };
        self.blockbuf = Texts::row(base, self.no);
        if self.cur_act == 0 {
            self.cut_in_block(self.x % 5);
        }
    }

    /// `CutInBlock(cblk, point)` (0x00419390): `prefix #G cell #W rest`,
    /// the cell also into `m_cbuf` (a full-width space shows as a square).
    fn cut_in_block(&mut self, point: i32) {
        let b = self.blockbuf.clone();
        let at = |i: usize| b.get(i).copied().unwrap_or(0);
        let p = (2 * point).max(0) as usize;
        let prefix = &cstr(&b)[..cstr(&b).len().min(p)];
        let mut sel = cstr(&[at(p), at(p + 1)]).to_vec();
        self.cbuf = sel.clone();
        if sel == b"\x81\x40" {
            sel = b"\x81\xa1".to_vec();
        }
        let rest: Vec<u8> = (p + 2..).map(at).take_while(|&c| c != 0).take(64).collect();
        let mut out = prefix.to_vec();
        out.extend_from_slice(b"#G");
        out.extend_from_slice(&sel);
        out.extend_from_slice(b"#W");
        out.extend_from_slice(&rest);
        self.blockbuf = out;
    }

    /// `SelName` (0x00415430): a cell entered, a character deleted, Default.
    fn sel_name(&mut self, t: &Texts) {
        if self.sw == 1 && self.cur_act == 0 {
            if self.name_num < self.str_max {
                let c = self.cbuf.first().copied().unwrap_or(0);
                let n = self.name_num;
                if (0..20).contains(&n) {
                    self.name_mut()[n as usize] = c;
                }
                self.name_num += 1;
            }
            if self.name_num == self.str_max {
                self.y = self.max.1;
                self.cur_act = 3;
            }
        } else if self.sw == -1 && self.cur_act != 4 {
            let n = self.name_num;
            if n > 0 && n <= 20 {
                self.name_mut()[n as usize - 1] = b'_';
            }
            self.name_num -= 1;
            if self.name_num < 0 {
                self.name_num = 0;
            }
        }
        if self.sw == 1 && self.cur_act == 1 {
            match self.name_act {
                0 => {
                    self.name_num = 0;
                    self.my_name = padded(b"", MY_MAX as usize);
                }
                1 => {
                    self.name_num = 0;
                    self.sur_name = padded(b"", USER_MAX as usize);
                }
                2 => {
                    self.name_num = DEFAULT_GAME_LEN;
                    self.game_name = padded(&t.default_game, GAME_MAX as usize);
                }
                _ => {}
            }
        }
    }

    /// `AllTlans` (0x00415830): the fade's step (`m_Tlans_Act` 2 in from
    /// 0, 1 out then 3, 3 in, 4-9 settle, 0 still), the name frame's pulse,
    /// and the alphas of what the screen draws.
    fn all_tlans(&mut self) {
        match self.tlans {
            1 => {
                self.tlans = 5;
                self.move_lock = 1;
            }
            2 => {
                self.trans = 0;
                self.tlans = 6;
                self.move_lock = 1;
            }
            3 => self.tlans = 6,
            4 => self.tlans = 9,
            5 => {
                if self.win_act == 0 {
                    self.trans = 128;
                    self.tlans = 3;
                } else {
                    self.trans -= self.sub_trans;
                }
                if self.trans < 0 {
                    self.trans = 0;
                    self.tlans = 3;
                }
            }
            6 => {
                self.trans += self.sub_trans;
                if self.trans > 128 {
                    self.trans = 128;
                    self.tlans = 4;
                }
            }
            9 => {
                self.move_lock = 0;
                self.tlans = 0;
            }
            _ => {}
        }
        let a = &mut self.alphas;
        let t = self.trans;
        match self.win_act {
            0 => {
                a.info = t;
                a.windows = t;
            }
            1 => {
                a.info = t;
                a.menu = t;
                a.name_prev = t;
                a.list = t;
                if self.win_flg {
                    self.win_trans += self.sub_trans >> 1;
                    if self.win_trans > 128 {
                        self.win_trans = 128;
                        self.win_flg = false;
                    }
                } else {
                    self.win_trans -= self.sub_trans >> 1;
                    if self.win_trans < 0 {
                        self.win_trans = 0;
                        self.win_flg = true;
                    }
                }
                if t < 128 {
                    a.waku = t;
                    self.win_trans = 128;
                } else {
                    a.waku = self.win_trans;
                }
                a.windows = t;
            }
            2 => {
                a.menu = t;
                a.info = t;
                a.list = t;
                a.windows = t;
            }
            3 => {
                a.menu = t;
                a.info = t;
                a.name_info = t;
                a.real_name = t;
                a.list = t;
                a.windows = t;
            }
            _ => {}
        }
    }
}

/// `CheckName` (0x00418db0): 2 when the name is only spaces, 1 when a
/// character name is one of the forbidden (lower case, spaces dropped, 19
/// characters looked at), else 0.
fn check_name(name: &[u8], name_act: i16, forbidden: &[Vec<u8>]) -> i32 {
    let s: Vec<u8> = cstr(name).iter().take(19).map(|c| c.to_ascii_lowercase()).filter(|&c| c != b' ').collect();
    if s.is_empty() {
        return 2;
    }
    if name_act == 2 && forbidden.iter().any(|f| cstr(f) == &s[..]) {
        return 1;
    }
    0
}

/// `ccMenuWindow::DispSelectCursol`'s state (+0xd0 on): the fading bars and
/// the wing.
#[derive(Clone, Debug)]
struct SelectCursor {
    sel: i16,
    idx: usize,
    a: [i16; 12],
    w: [f32; 12],
    x: [f32; 12],
    y: [f32; 12],
    wx: [f32; 6],
    wy: [f32; 6],
}

impl SelectCursor {
    /// `InitCursol(0)` (0x001b78b0).
    fn new() -> Self {
        SelectCursor {
            sel: -2,
            idx: 0,
            a: [0; 12],
            w: [0.0; 12],
            x: [0.0; 12],
            y: [0.0; 12],
            wx: [0.0; 6],
            wy: [0.0; 6],
        }
    }

    fn init(&mut self) {
        self.sel = -2;
        self.idx = 0;
        self.a = [0; 12];
    }
}

/// `ccMenuWindow` +0x1ac, +0x1ae: `DispPageCursol`'s alpha and its step.
#[derive(Clone, Copy, Debug, Default)]
struct PageCursor {
    a: i16,
    da: i16,
}

/// A trace event, built only when the `trace` feature records it.
fn event(f: impl FnOnce() -> String) {
    #[cfg(feature = "trace")]
    trace::push(f());
    #[cfg(not(feature = "trace"))]
    let _ = f;
}

fn hex(s: &[u8]) -> String {
    s.iter().map(|b| format!("{b:02x}")).collect()
}

/// An object's name for the trace: `List[3]`, `Windows`.
fn label(name: &str, i: Option<usize>) -> String {
    match i {
        Some(i) => format!("{name}[{i}]"),
        None => name.to_string(),
    }
}

/// A sprite of the entry's and its name for the trace.
struct Spr {
    s: Sprite,
    name: &'static str,
}

impl Spr {
    fn new(name: &'static str, layer: i16, tex: TexRef, tex_h: i32, packet_max: usize) -> Spr {
        Spr { s: Sprite::mask(layer, tex, tex_h, packet_max), name }
    }

    fn grid(&mut self, g: grid::Grid) {
        (self.s.wu, self.s.wv, self.s.su, self.s.sv, self.s.wi) = g;
    }

    fn at(&mut self, dx: f32, dy: f32) {
        self.s.dx = dx;
        self.s.dy = dy;
    }

    fn size(&mut self, sx: f32, sy: f32) {
        self.s.sx = sx;
        self.s.sy = sy;
    }

    fn rgb(&mut self, c: [u8; 4]) {
        self.s.colour[..3].copy_from_slice(&c[..3]);
    }

    fn alpha(&mut self, a: i32) {
        self.s.colour[3] = a as u8;
    }

    /// `ccSprite::MakePacket(code, 1)`.
    fn pkt(&mut self, code: u8, view: &LayerView) {
        let s = &self.s;
        let name = self.name;
        event(|| {
            let f = [s.dx, s.dy, s.sx, s.sy, s.cx, s.cy].map(f32::to_bits);
            format!(
                "[\"pkt\",\"{name}\",{code},[{},{},{},{},{},{}],{},{},{},{},{},[{},{},{},{}],{}]",
                f[0],
                f[1],
                f[2],
                f[3],
                f[4],
                f[5],
                s.su,
                s.sv,
                s.wu,
                s.wv,
                s.wi,
                s.colour[0],
                s.colour[1],
                s.colour[2],
                s.colour[3],
                s.flip_u as i32
            )
        });
        self.s.make_packet(code, view);
    }

    /// `ccSprite::SendPacket`.
    fn send(&mut self, ctx: &mut Ctx) {
        event(|| format!("[\"send\",\"{}\"]", self.name));
        self.s.send(&mut ctx.layers);
    }

    /// `ccMenuWindow::DispSquare(w, h, NULL)` at (dx, dy) (message.rs'
    /// port of it), in the sprite's colour.
    fn square(&mut self, w: i32, h: i32, view: &LayerView) {
        let mut cells = Vec::new();
        square(&mut cells, self.s.dx, self.s.dy, w, h, self.s.colour[3]);
        for c in cells {
            if let MsgDraw::Cell { code, dx, dy, sx, sy, grid, .. } = c {
                self.grid(grid);
                self.at(dx, dy);
                self.size(sx, sy);
                self.pkt(code, view);
            }
        }
    }
}

/// The system font's `MakePacketStr(str, 0)`: a cell for each byte in
/// 0x21-0x7f, every byte a cell's width on.
fn font_str(f: &mut Spr, text: &[u8], view: &LayerView) {
    let s = &f.s;
    let name = f.name;
    event(|| {
        let v = [s.dx, s.dy, s.sx, s.sy].map(f32::to_bits);
        format!(
            "[\"str\",\"{name}\",[{},{},{},{}],{},{},{},{},{},[{},{},{},{}],\"{}\"]",
            v[0],
            v[1],
            v[2],
            v[3],
            s.su,
            s.sv,
            s.wu,
            s.wv,
            s.wi,
            s.colour[0],
            s.colour[1],
            s.colour[2],
            s.colour[3],
            hex(text)
        )
    });
    for &c in cstr(text) {
        if (0x21..0x80).contains(&c) {
            f.s.make_packet(c - 0x20, view);
        } else {
            f.s.dx = eef::add(f.s.dx, f.s.sx);
        }
    }
}

/// The C remainder and quotient of a hardware `div` by zero as the EE gives
/// them (LO -1 for a dividend at or above zero, else 1).
fn div_ee(n: i32, d: i32) -> i32 {
    if d == 0 { if n < 0 { 1 } else { -1 } } else { n.wrapping_div(d) }
}

/// The new game's name entry.
pub struct NameEntry {
    fonts: Fonts,
    texts: Texts,
    logic: Logic,
    view: LayerView,
    /// `m_Windows`, `m_WinWaku`, `m_Face`'s two sprites (the picture, and
    /// the panel cut from the window texture), and the system font.
    windows: Spr,
    waku: Spr,
    face0: Spr,
    face1: Spr,
    font: Spr,
    /// `m_Windows`' cursor states.
    cursor: SelectCursor,
    page: PageCursor,
    done: bool,
    requests: Vec<Request>,
}

/// A texture of a scene file in `DATA.BIN` and its height, drawn through
/// its own palette.
fn texture(archive: &Archive, file: &str, name: &str) -> Result<(TexRef, i32)> {
    let ccs = Ccs::parse(archive.inflate_named(file)?)?;
    let object = ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{file}::{name}")))?;
    let (textures, _) = piney_data::texture::read(&ccs)?;
    let t = textures.iter().find(|t| t.object == object).ok_or_else(|| Error::NotFound(name.to_string()))?;
    Ok((TexRef::Ccs { file: file.to_string(), texture: object, clut: t.clut }, 1 << t.th))
}

impl NameEntry {
    /// `new NameEntry_Control; Init()`: the disc's volume for the fonts,
    /// and the textures (`xwindow`, `xwin_f00`, `xasc00`).
    pub fn new(iso: &mut Iso, archive: Arc<Archive>, state: &SaveState) -> Result<NameEntry> {
        NameEntry::of(iso.volume()?, &archive, state)
    }

    /// As [`NameEntry::new`] for a volume.
    pub fn of(volume: Volume, archive: &Archive, _state: &SaveState) -> Result<NameEntry> {
        let fonts = read_fonts(volume, archive)?;
        let texts = Texts::new();
        let win = WindowTexture::read(archive).ok_or_else(|| Error::NotFound("xwindow::TEX_xwindo00".into()))?;
        let (face_tex, face_h) = texture(archive, FACE_FILE, ne::of(volume).face_tex())?;
        let (font_tex, font_h) = texture(archive, FONT_FILE, FONT_TEX)?;
        let logic = Logic::new(&texts.default_game);
        let mut windows = Spr::new("Windows", LAYER, win.tex.clone(), win.tex_h, 300);
        windows.grid(grid::TYPE_1);
        let mut waku = Spr::new("WinWaku", LAYER, win.tex.clone(), win.tex_h, 300);
        waku.grid(grid::TYPE_1);
        let face0 = Spr::new("Face.spr0", LAYER, face_tex, face_h, 2);
        let face1 = Spr::new("Face.spr1", LAYER, win.tex, win.tex_h, 32);
        let font = Spr::new("font", FONT_LAYER, font_tex, font_h, 1024);
        Ok(NameEntry {
            fonts,
            texts,
            logic,
            view: LayerView::default_layer(),
            windows,
            waku,
            face0,
            face1,
            font,
            cursor: SelectCursor::new(),
            page: PageCursor::default(),
            done: false,
            requests: Vec::new(),
        })
    }

    /// One `Main`. On the frame it returns 1 the names are written into the
    /// save (`SetSaveData`: `plName` the character name, `plRealName` the
    /// user name) and nothing is drawn; after that it does nothing.
    pub fn step(&mut self, pad: &Pad, state: &mut SaveState) -> Frame {
        let mut ctx = Ctx::new(View::default());
        if self.done {
            return ctx.finish();
        }
        if state.save.u8(offset::NEW_GAME_FLAG) != 0 {
            self.done = true;
            return ctx.finish();
        }
        let keys = Keys {
            push: pad.push.bits(),
            unpush: pad.unpush.bits(),
            repeat: pad.repeat.bits(),
            ok: state.ok(),
            cancel: state.cancel(),
        };
        let finished = self.logic.main(&keys, &self.texts, &mut self.requests);
        if std::mem::take(&mut self.logic.cursor_reset) {
            self.cursor.init();
            self.page = PageCursor::default();
        }
        if finished {
            self.set_save_data(state);
            self.done = true;
            return ctx.finish();
        }
        let names = state.names();
        let spc = offset::SPC_PARAM;
        let bars = (state.save.i16(spc + SPC_MAX_HP), state.save.i16(spc + SPC_MAX_SP));
        self.draw_cur(&mut ctx);
        self.kanji_draw(&mut ctx, &names);
        self.draw_window(&mut ctx, bars);
        ctx.finish()
    }

    /// `Main` has returned 1.
    pub fn done(&self) -> bool {
        self.done
    }

    /// The sound effects the frames since the last call asked for.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// The logic's state, for inspection.
    pub fn logic(&self) -> &Logic {
        &self.logic
    }

    /// `SetSaveData` (0x004152a0): `strcpy(plName, EntryGameName)`,
    /// `strcpy(plRealName, EntryName)`.
    fn set_save_data(&self, state: &mut SaveState) {
        let b = state.save.bytes_mut();
        strcpy(&mut b[offset::PL_NAME..offset::PL_NAME + 24], &self.logic.entry_game_name);
        strcpy(&mut b[offset::PL_REAL_NAME..offset::PL_REAL_NAME + 24], &self.logic.entry_name);
    }

    /// A `ccKanji::Disp(text, -1)` on the layer, which sends at once.
    #[allow(clippy::too_many_arguments)]
    fn kanji(
        &self,
        ctx: &mut Ctx,
        names: &Names,
        name: &str,
        i: Option<usize>,
        kt: Kt,
        alpha: i32,
        (dx, dy): (i32, i32),
        text: &[u8],
    ) {
        let text = cstr(text);
        let c = SPRITE_COLOR_TABLE[7];
        let colour = [c[0], c[1], c[2], alpha as u8];
        let (fx, fy) = (eef::from_int(dx), eef::from_int(dy));
        event(|| {
            format!(
                "[\"kanji\",\"{}\",{},1,{},{},[{},{},{},{}],\"{}\"]",
                label(name, i),
                kt as i32,
                fx.to_bits(),
                fy.to_bits(),
                colour[0],
                colour[1],
                colour[2],
                colour[3],
                hex(text)
            )
        });
        let mut k = Kanji::init(3, 16);
        k.kt = kt;
        k.colour = colour;
        k.dx = fx;
        k.dy = fy;
        ctx.disp(&self.fonts, &mut k, LAYER, &self.view, text, names);
    }

    /// `DrawCur` (0x00418860): on the grid, the cursor 20 right and down of
    /// where the last `KanjiDraw` put it, the wing only.
    fn draw_cur(&mut self, ctx: &mut Ctx) {
        if self.logic.win_act != 1 {
            return;
        }
        let (x, y) = self.logic.disp_cur;
        // KanjiFlg is 0: both move.
        self.logic.disp_cur =
            (eef::to_int(eef::add(eef::from_int(x), 20.0)), eef::to_int(eef::add(eef::from_int(y), 20.0)));
        let t = self.logic.trans;
        if t != 0 {
            let (x, y) = self.logic.disp_cur;
            self.select_cursor(eef::from_int(x), eef::from_int(y), 1, 1, t, 1);
            self.windows.send(ctx);
        }
    }

    /// `m_Windows->DispSelectCursol(x, y, w, sn, a, bits)` (0x001b7910):
    /// bit 2 the bars (a new one on the item `sn` when it changes, the old
    /// fading), bit 1 the six-feather wing chasing the item.
    fn select_cursor(&mut self, x: f32, y: f32, w: i32, sn: i16, alpha: i32, bits: u32) {
        use eef::{add, div, from_int, mul, sub};
        let fr = FRAME_RATE as i32;
        let view = self.view;
        let c = &mut self.cursor;
        let win = &mut self.windows;
        if c.sel == -2 {
            c.wx = [sub(x, 16.0); 6];
            c.wy = [sub(y, 18.0); 6];
        }
        if sn != c.sel {
            c.sel = sn;
            let mut m = 255;
            for j in 0..12 {
                if c.a[j] < m {
                    m = c.a[j];
                    c.idx = j;
                }
            }
            c.a[c.idx] = 128;
            c.w[c.idx] = from_int((w - 1) * 14);
            c.x[c.idx] = add(6.0, x);
            c.y[c.idx] = sub(y, 2.0);
        }
        win.grid(grid::BAR);
        for s in 0..12 {
            if c.a[s] == 0 {
                continue;
            }
            let a = i32::from(c.a[s]);
            if s == c.idx {
                if a < 48 {
                    c.a[s] = 96;
                } else {
                    let mut step = (32 - a) / 16 / (3 - fr);
                    if step >= 0 {
                        step = -1;
                    }
                    c.a[s] = (a + step) as i16;
                }
            } else {
                let step = (a / 8 / (3 - fr)).max(4);
                c.a[s] = (a - step) as i16;
                if c.a[s] < 8 {
                    c.a[s] = 0;
                    continue;
                }
            }
            if alpha < i32::from(c.a[s]) {
                c.a[s] = alpha as i16;
            }
            if bits & 2 != 0 {
                win.alpha(i32::from(c.a[s]));
                win.size(16.0, 20.0);
                win.at(c.x[s], c.y[s]);
                win.pkt(0, &view);
                win.size(c.w[s], 20.0);
                win.at(add(16.0, c.x[s]), c.y[s]);
                win.pkt(1, &view);
                win.size(16.0, 20.0);
                win.at(add(add(16.0, c.x[s]), c.w[s]), c.y[s]);
                win.pkt(2, &view);
            }
        }
        if bits & 1 != 0 {
            win.alpha(alpha);
            win.grid(grid::TYPE_2);
            win.size(20.0, 20.0);
            let mut f = from_int(i32::from(c.a[c.idx]));
            f = if eef::lt(96.0, f) { div(f, 10.0) } else { sub(div(f, 12.0), 4.0) };
            let tx = add(x, sub(f, 16.0));
            let ty = add(y, sub(f, 18.0));
            for k in (0..6).rev() {
                if k > 0 {
                    let d = mul(sub(10.0, from_int(k as i32)), sub(3.0, from_int(fr)));
                    c.wx[k] = add(c.wx[k - 1], div(sub(tx, c.wx[k - 1]), d));
                    c.wy[k] = add(c.wy[k - 1], div(sub(ty, c.wy[k - 1]), d));
                    let g = (128 - 8 * k) as u8;
                    win.rgb([g, g, g, 0]);
                    win.alpha((48 - 8 * k as i32).min(alpha));
                } else {
                    let d = div(4.5, from_int(fr));
                    c.wx[0] = add(c.wx[0], div(sub(tx, c.wx[0]), d));
                    c.wy[0] = add(c.wy[0], div(sub(ty, c.wy[0]), d));
                    win.rgb(SPRITE_COLOR_TABLE[7]);
                    win.alpha(alpha);
                }
                win.at(c.wx[k], c.wy[k]);
                win.pkt(10, &view);
            }
        }
    }

    /// `KanjiDraw` (0x004166b0): the screen's text, and `m_DispCurPos`.
    fn kanji_draw(&mut self, ctx: &mut Ctx, names: &Names) {
        let l = self.logic.clone();
        let t = &self.texts;
        let a = l.alphas;
        let mut act = l.main_act;
        if matches!(act, 12 | 14 | 16) {
            act = l.temp_main_act;
        }
        let (mut grid, mut dialog) = (false, false);
        let (ix, iy) = pos::INFO;
        match act {
            1 | 19 => {
                let page = t.info.get(l.info_count.max(0) as usize);
                let at: &[(i32, i32)] = match l.info_count {
                    0 => &[(18, 0), (26, 20)],
                    1 => &[(4, 0), (56, 20)],
                    2 => &[(-10, 10)],
                    3 => &[(12, 0), (-18, 20)],
                    4 => &[(10, 10)],
                    _ => &[],
                };
                if let Some(page) = page {
                    for (i, &(dx, dy)) in at.iter().enumerate() {
                        self.kanji(
                            ctx,
                            names,
                            "Info",
                            Some(i),
                            Kt::LargeProportional,
                            a.info,
                            (ix + dx, iy + dy),
                            &page[i],
                        );
                    }
                }
            }
            17 => {
                dialog = true;
                let (nx, ny) = pos::NAME_INFO;
                let (fx, fy) = pos::F_AFF;
                let lp = Kt::LargeProportional;
                self.kanji(ctx, names, "NameInfo", Some(0), lp, a.name_info, (nx, ny), &t.entry_name);
                self.kanji(ctx, names, "NameInfo", Some(1), lp, a.name_info, (nx, ny + 50), &t.entry_game_name);
                self.kanji(ctx, names, "Info", Some(0), lp, a.info, (fx, fy), &t.affirmation[0]);
                self.kanji(ctx, names, "Info", Some(1), lp, a.info, (fx, fy + 20), &t.affirmation[1]);
                self.kanji(ctx, names, "Info", Some(2), lp, a.info, (fx + 80, fy + 70), &t.yes);
                self.kanji(ctx, names, "Info", Some(3), lp, a.info, (fx + 80, fy + 90), &t.no);
            }
            3 | 7 => {
                grid = true;
                let (name, alpha) = if l.reserve == -1 { ("List", a.list) } else { ("DamyList", 128) };
                let (lx, ly) = pos::LIST;
                for s in 0..LIST_ROWS {
                    let si = s as i32;
                    let (col, row) = if act == 3 { (si / 6, si % 6) } else { (si % 3, si / 3) };
                    let at = if act == 3 {
                        (lx + 20 + 120 * col, ly + pos::HI * row)
                    } else {
                        (lx + 18 + 120 * col, ly - 4 + pos::HI * row)
                    };
                    let text = if si == l.no {
                        self.logic.disp_cur = if act == 3 {
                            (lx + 120 * col + 20 * (l.x % 5), ly + pos::HI * row - 10)
                        } else {
                            (lx - 6 + 120 * col + 24 * (l.x % 5), ly - 14 + pos::HI * row)
                        };
                        l.blockbuf.clone()
                    } else {
                        Texts::row_str(if act == 3 { *ne::HIRA_BLOCK } else { *ne::EIGO_BLOCK }, s)
                    };
                    self.kanji(ctx, names, name, Some(s), Kt::LargeFixed, alpha, at, &text);
                }
            }
            // The kana, symbol and kanji grids (5, 9, 11) are not reached.
            _ => {}
        }
        let (mx, my) = pos::MSG_INFO;
        if grid {
            let (ex, ey) = pos::ENTER_END;
            let menu = t.menu.get(l.cur_act.max(0) as usize).cloned().unwrap_or_default();
            self.kanji(ctx, names, "EnterEnd", None, Kt::SmallProportional, a.menu, (ex, ey), &menu);
            match l.cur_act {
                1 => self.logic.disp_cur = (ex - 26, ey - 16),
                2 => self.logic.disp_cur = (ex + 82, ey - 16),
                3 => self.logic.disp_cur = (ex + 200, ey - 16),
                _ => {}
            }
            let lp = Kt::LargeProportional;
            let act = l.name_act.clamp(0, 2) as usize;
            if l.err != 0 {
                let e = &t.str_msg[3];
                match l.err {
                    1 => self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &e[0]),
                    2 => self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &e[1]),
                    3 => {
                        self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &e[2]);
                        self.kanji(ctx, names, "Info", Some(1), lp, a.info, (mx + 7, my + 20), &e[3]);
                    }
                    _ => {}
                }
            } else {
                match l.cur_act {
                    1 => self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &t.reset[act]),
                    2 => self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &t.all_reset),
                    3 => self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &t.enter),
                    _ => {
                        let m = &t.str_msg[act];
                        self.kanji(ctx, names, "Info", Some(0), lp, a.info, (mx + 7, my), &m[0]);
                        self.kanji(ctx, names, "Info", Some(1), lp, a.info, (mx + 7, my + 20), &m[1]);
                    }
                }
            }
        }
        if dialog && l.name_act == 2 {
            let (nx, ny) = pos::NAME_INFO;
            let lf = Kt::LargeFixed;
            self.kanji(ctx, names, "RealName", Some(0), lf, a.real_name, (nx, ny + 20), &l.entry_name);
            self.kanji(ctx, names, "RealName", Some(1), lf, a.real_name, (nx, ny + 70), &l.entry_game_name);
        }
        if grid {
            let (px, py) = pos::NAME_PREV;
            let act = l.name_act.clamp(0, 2) as usize;
            let sp = Kt::SmallProportional;
            let p = a.name_prev;
            self.kanji(ctx, names, "NamePrev", Some(1), sp, p, (px, py), &t.labels[act][0]);
            self.kanji(ctx, names, "NamePrev", Some(2), sp, p, (px, py + 45), &t.labels[act][1]);
            let (first, second): (&[u8], &[u8]) = match l.name_act {
                0 => (&l.sur_name, &l.game_name),
                1 => (&l.temp_name, &l.game_name),
                _ => (&l.sur_name, &l.temp_name),
            };
            self.kanji(ctx, names, "NamePrev", Some(4), Kt::LargeFixed, p, (px, py + 20), first);
            self.kanji(ctx, names, "NamePrev", Some(5), Kt::LargeFixed, p, (px, py + 60), second);
        }
    }

    /// `DrawWindow` (0x00415ed0): the windows of the screen, the frame round
    /// the name being entered and the face panel, then the two window
    /// sprites sent.
    fn draw_window(&mut self, ctx: &mut Ctx, bars: (i16, i16)) {
        let view = self.view;
        self.windows.grid(grid::TYPE_1);
        self.waku.grid(grid::TYPE_1);
        let l = &self.logic;
        let (px, py) = pos::NAME_PREV;
        let (s0, s1, f) = match l.name_act {
            1 => (px - 6, py - 4, 200.0),
            2 => (px - 6, py + 41, 160.0),
            _ => (240, 10, 70.0),
        };
        self.windows.alpha(l.alphas.windows);
        self.waku.alpha(l.alphas.waku);
        let fi = eef::from_int;
        match l.win_act {
            0 => {
                let (ix, iy) = pos::INFO;
                self.windows.at(fi(ix - 49), fi(iy - 14));
                self.windows.square(25, 2, &view);
                self.windows.at(fi(ix + 140), fi(iy + 40));
                self.page_cursor();
            }
            1 => {
                let (lx, ly) = pos::LIST;
                let (mx, my) = pos::MSG_INFO;
                let w = &mut self.windows;
                w.at(fi(px - 10), fi(py - 14));
                w.square(15, 4, &view);
                w.at(fi(lx - 14), fi(ly - 14));
                w.square(27, 7, &view);
                w.at(fi(mx - 14), fi(my - 14));
                w.square(27, 2, &view);
                let k = &mut self.waku;
                k.grid(grid::WAKU);
                k.at(fi(s0), fi(s1));
                k.size(16.0, 16.0);
                k.pkt(0, &view);
                k.size(f, 16.0);
                k.pkt(4, &view);
                k.size(16.0, 16.0);
                k.pkt(1, &view);
                k.at(fi(s0), fi(s1 + 16));
                k.size(16.0, 16.0);
                k.pkt(2, &view);
                k.at(fi(eef::to_int(eef::add(fi(s0 + 16), f))), fi(s1 + 16));
                k.size(16.0, 16.0);
                k.pkt(3, &view);
                k.at(fi(s0), fi(s1 + 32));
                k.size(16.0, 16.0);
                k.pkt(6, &view);
                k.size(f, 16.0);
                k.pkt(5, &view);
                k.size(16.0, 16.0);
                k.pkt(7, &view);
                let (fx, fy) = pos::FACE;
                let t = l.trans;
                self.face_panel(ctx, fi(fx), fi(fy), t, bars);
            }
            3 => {
                let (fx, fy) = pos::F_AFF;
                let (nx, ny) = pos::NAME_INFO;
                let w = &mut self.windows;
                w.at(fi(fx - 14), fi(fy - 14));
                w.square(17, 2, &view);
                w.at(fi(nx - 14), fi(ny - 14));
                w.square(17, 5, &view);
                w.at(fi(fx + 55), fi(fy + 60));
                w.square(6, 2, &view);
                let (dy, sn) = if l.dialog == 0 { (92, 0) } else { (72, 1) };
                let t = l.trans;
                self.select_cursor(fi(fx + 55), fi(fy + dy), 6, sn, t, 3);
            }
            // WinAct 2 (the kanji grid's) is not reached.
            _ => {}
        }
        self.waku.send(ctx);
        self.windows.send(ctx);
    }

    /// `m_Windows->DispPageCursol(0)` (0x001baa10) at the sprite's (dx,
    /// dy): cell 24, its width and drop following an alpha that climbs by
    /// 8 and falls by 4.
    fn page_cursor(&mut self) {
        use eef::{add, div, from_int, mul, sub};
        let view = self.view;
        let p = &mut self.page;
        p.a += p.da;
        if p.a <= 0 {
            p.da = if FRAME_RATE == 2 { 16 } else { 8 };
            p.a = 0;
        } else if p.a >= 128 {
            p.da = if FRAME_RATE == 2 { -8 } else { -4 };
            p.a = 128;
        }
        let af = from_int(i32::from(p.a));
        let w = div(mul(20.0, af), 128.0);
        let d = div(mul(6.0, af), 128.0);
        let win = &mut self.windows;
        win.grid(grid::TYPE_1);
        win.alpha(i32::from(p.a));
        win.size(w, 20.0);
        let (x, y) = (win.s.dx, win.s.dy);
        win.at(sub(x, div(w, 2.0)), add(y, d));
        win.pkt(24, &view);
    }

    /// `ccFacePanel::Draw(x, y, alpha)` (main 0x001bba30): the panel
    /// (`DispTarget(8, 1)`), the HP and SP boxes and bars, "---/---" in the
    /// system font, the face; then the panel and the face sent.
    fn face_panel(&mut self, ctx: &mut Ctx, x: f32, y: f32, alpha: i32, (hp, sp): (i16, i16)) {
        use eef::{add, from_int, sub};
        let view = self.view;
        let c7 = SPRITE_COLOR_TABLE[7];
        let p = &mut self.face1;
        p.rgb(c7);
        p.alpha(alpha);
        p.at(x, y);
        // DispTarget(8, 1) (0x001ba660).
        p.grid(grid::TARGET);
        p.size(90.0, 44.0);
        p.at(x, add(44.0, y));
        p.pkt(1, &view);
        p.at(x, y);
        p.pkt(0, &view);
        p.grid(grid::TAB);
        p.at(sub(add(90.0, x), 40.0), y);
        let mut rest = 100.0;
        let tab = sub(eef::mul(8.0, from_int(8 - 1)), 4.0);
        if !eef::le(tab, 0.0) {
            p.size(tab, 24.0);
            p.pkt(4, &view);
            rest = sub(rest, tab);
        }
        p.size(16.0, 24.0);
        p.pkt(5, &view);
        if !eef::le(rest, 0.0) {
            p.size(rest, 24.0);
            p.pkt(1, &view);
        }
        p.grid(grid::TAB_END);
        p.size(8.0, 24.0);
        p.pkt(0, &view);
        // The HP and SP boxes.
        let f20 = add(74.0, x);
        let f23 = add(24.0, y);
        let f21 = add(6.0, f23);
        let f24 = add(52.0, y);
        let f22 = add(6.0, f24);
        p.grid(grid::BOX);
        p.size(8.0, 25.0);
        p.s.flip_u = true;
        p.at(f20, f21);
        p.pkt(1, &view);
        let f27 = add(10.0, f20);
        p.at(f27, f22);
        p.pkt(1, &view);
        p.s.flip_u = false;
        p.size(80.0, 25.0);
        let f28 = add(8.0, f20);
        p.at(f28, f21);
        p.pkt(0, &view);
        p.size(60.0, 25.0);
        let f27 = add(8.0, f27);
        p.at(f27, f22);
        p.pkt(0, &view);
        p.size(8.0, 25.0);
        p.at(add(80.0, f28), f21);
        p.pkt(1, &view);
        p.at(add(60.0, f27), f22);
        p.pkt(1, &view);
        // "HP", "SP".
        p.grid(grid::CAPTION);
        p.size(84.0, 16.0);
        let f20 = add(6.0, f20);
        p.at(f20, f23);
        p.pkt(0, &view);
        p.at(f20, f24);
        p.pkt(1, &view);
        // The bars: maxHP * 56 / maxHP of them full, the rest empty.
        p.grid(grid::FILL);
        let bar_x = sub(sub(add(84.0, f20), 56.0), 2.0);
        for (v, dy, rgb) in [(hp, f23, [48, 128, 80, 0]), (sp, f24, [48, 80, 128, 0])] {
            let v = i32::from(v);
            let full = div_ee(v.wrapping_mul(BAR_W), v);
            p.size(from_int(full), 8.0);
            p.at(bar_x, add(4.0, dy));
            p.rgb(rgb);
            p.alpha(alpha);
            p.pkt(0, &view);
            if full < BAR_W {
                p.size(from_int(BAR_W - full), 8.0);
                p.rgb(SPRITE_COLOR_TABLE[18]);
                p.alpha(alpha);
                p.pkt(0, &view);
            }
        }
        p.rgb(c7);
        // font->SetType(2), in ccSpriteColorTable[22].
        let fnt = &mut self.font;
        fnt.grid(grid::FONT_2);
        fnt.size(10.0, 12.0);
        fnt.rgb(SPRITE_COLOR_TABLE[22]);
        fnt.alpha(alpha);
        let fx = sub(add(16.0, f20), 10.0);
        let dashes = self.texts.dashes.clone();
        fnt.at(fx, add(38.0, y));
        font_str(fnt, &dashes, &view);
        fnt.rgb(SPRITE_COLOR_TABLE[22]);
        fnt.alpha(alpha);
        fnt.at(fx, add(66.0, y));
        font_str(fnt, &dashes, &view);
        // The face, 96 x 96.
        let f = &mut self.face0;
        f.rgb(c7);
        f.alpha(alpha);
        f.at(x, y);
        f.s.su = 96;
        f.s.sv = 96;
        f.size(96.0, 96.0);
        f.pkt(0, &view);
        self.face1.send(ctx);
        self.face0.send(ctx);
        // fontSend, from ccSystem::Ctrl, sends the font's packets with the
        // frame's.
        self.font.s.send(&mut ctx.layers);
    }
}

/// A record of the entry's text, packets and sends, in call order, as JSON
/// arrays, for `tools/test_desktop_name_rs.py`.
#[cfg(feature = "trace")]
pub mod trace {
    use std::cell::RefCell;

    thread_local! {
        static LOG: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    }

    pub fn start() {
        LOG.with(|l| *l.borrow_mut() = Some(Vec::new()));
    }

    pub fn stop() -> Vec<String> {
        LOG.with(|l| l.borrow_mut().take().unwrap_or_default())
    }

    pub(super) fn push(e: String) {
        LOG.with(|l| {
            if let Some(v) = l.borrow_mut().as_mut() {
                v.push(e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_underscore() {
        assert_eq!(first_us(&padded(b"", 18), 18), 0);
        assert_eq!(first_us(&padded(b"Kite", 14), 14), 4);
        assert_eq!(first_us(b"Helba\0", 14), 14);
    }

    #[test]
    fn names_are_checked() {
        let f = vec![b"mia".to_vec(), b"balmung".to_vec()];
        assert_eq!(check_name(b"   \0", 2, &f), 2);
        assert_eq!(check_name(b"bal Mung\0", 2, &f), 1);
        assert_eq!(check_name(b"bal Mung\0", 1, &f), 0);
        assert_eq!(check_name(b"Mia\0", 2, &f), 1);
        assert_eq!(check_name(b"Kite\0", 2, &f), 0);
    }

    #[test]
    fn repeat_waits_then_steps() {
        let mut l = Logic::new(b"Kite");
        let held = Keys { push: 0, unpush: 0, repeat: 0x1000, ok: 0x40, cancel: 0x20 };
        let fired: Vec<usize> = (1..60).filter(|_| l.rep(&held, 0x1000)).collect();
        assert_eq!(fired, vec![30, 37, 44, 51, 58]);
    }
}
