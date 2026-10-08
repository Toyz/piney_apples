//! `ccDtMenu` and its task `ccThDtMenu` (main executable 0x00169b00 -
//! 0x00171250): the desktop's menu layer (priority 242), which holds the event
//! scripts' message window (`ccMsg`), the dim behind it, and the START system
//! menu (`docs/engine/desktop.md`, "The menu task"). The task runs each frame
//! before the desktop task (`ccSetupDesktop` 0x001684f0 starts it first, both
//! at 33). Opening a menu sleeps every other task and freezes every other
//! layer's picture (`ccSleepAllThread`, `OffFlipExcept`) until it wakes them.

use piney_data::tables::sjis::encode;
use piney_data::volume::Volume;
use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::anm::Ctx;
use crate::fade;
use crate::kanji::{Fonts, Kanji, Kt, Names, expand, extract};
use crate::message::{MENU_LAYER, MessageKind, MsgWindow, WindowTexture, menu_view, render};
use crate::save::{SaveState, offset};
use crate::savesys::{INFO_COUNT, SaveDataInfo, SaveSys};
use crate::sprite::Sprite;
use crate::{Request, Se};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

mod save;

/// `CheckMenuType` while a menu opens or closes.
pub const MENU_CHANGING: i32 = 12;
/// Menu numbers (the jump table at 0x0034c010).
pub const MENU_SYSTEM: i16 = 0;
pub const MENU_RESET: i16 = 6;
pub const MENU_FADE: i16 = 7;
pub const MENU_SAVE_SELECT: i16 = 8;
pub const MENU_SAVE: i16 = 9;
/// Sound effects: open, cursor, decide, back.
pub const SE_OPEN: Se = Se(16);
pub const SE_MOVE: Se = Se(17);
pub const SE_OK: Se = Se(18);
pub const SE_BACK: Se = Se(19);
/// Decide and back on the title screen (`ccGame.status == 1`): the
/// title's own sounds.
pub const SE_TITLE_OK: Se = Se(4);
pub const SE_TITLE_BACK: Se = Se(7);
/// The chosen row's colour on the title screen, where no select cursor is
/// drawn (`ccSpriteColorTable[20]`).
pub const C_TITLE_CHOSEN: usize = 20;
/// The title's OPTION list (`disp` 5): its rows at (193, 152 + 12), 24
/// apart, with no window.
pub const TITLE_LIST_X: f32 = 193.0;
pub const TITLE_LIST_Y: f32 = 164.0;
pub const TITLE_ROW_PITCH: f32 = 24.0;
/// The lists `ccDtMenu` has; `InitMenuList` fills the first 13 from
/// `dtMenuElementData` (0x00306f70, `piney_data::tables::dtmenu`).
pub const LISTS: usize = 14;
/// The dim under a menu or message: `EntryFade(6, 0, 0x60000000, 0, 0,
/// 512, 448)`.
pub const DIM_FRAMES: u32 = 6;
pub const DIM_COLOUR: u32 = 0x6000_0000;
/// The menu window's alpha steps (`Disp`): 12 in, 14 out.
pub const ALPHA_IN: i16 = 12;
pub const ALPHA_OUT: i16 = 14;
/// ccSys operations START is checked against (`CheckOperate(11)`, `(28)`).
pub const OPERATE_START: [i32; 2] = [11, 28];
/// The option menus (items of the OPTION list).
pub const MENU_CONTROLLER: i16 = 2;
pub const MENU_VIBRATE: i16 = 3;
pub const MENU_SCREEN: i16 = 4;
pub const MENU_SOUND: i16 = 5;
pub const MENU_VOICE: i16 = 10;
pub const MENU_STRWIN: i16 = 11;
/// The Controller's picture (`flContMenu`: `XCONTROL.CCS`).
pub const CONTROL_FILE: &str = "xcontrol";
pub const CONTROL_TEX: &str = "TEX_xcontrol";
/// Colours of `ccSpriteColorTable` the options use.
pub const C_WHITE: usize = 7;
pub const C_SET: usize = 6;
pub const C_CHOSEN: usize = 22;
pub const C_GREY: usize = 16;
pub const C_MARK: usize = 10;
/// Adjust Screen's range: x -48..48 by 3, y -16..16 by 1.
pub const SCREEN_X_RANGE: i16 = 96;
pub const SCREEN_Y_RANGE: i16 = 32;
/// Adjust Screen's twelve marks round the frame (x, y, w, h).
pub const SCREEN_MARKS: [(f32, f32, f32, f32); 12] = [
    (0.0, 0.0, 10.0, 80.0),
    (10.0, 0.0, 70.0, 10.0),
    (0.0, 368.0, 10.0, 80.0),
    (10.0, 438.0, 70.0, 10.0),
    (502.0, 0.0, 10.0, 80.0),
    (432.0, 0.0, 70.0, 10.0),
    (502.0, 368.0, 10.0, 80.0),
    (432.0, 438.0, 70.0, 10.0),
    (251.0, 112.0, 10.0, 48.0),
    (251.0, 288.0, 10.0, 48.0),
    (128.0, 219.0, 48.0, 10.0),
    (336.0, 219.0, 48.0, 10.0),
];

/// `ccMenuWindow::SetType` grids for the menu (wu, wv in 1/16 texels; cell
/// w, h; cells a row).
pub mod grid {
    pub const TYPE_0: (i32, i32, i32, i32, i32) = (0, 512, 9, 16, 28);
    pub const TYPE_1: (i32, i32, i32, i32, i32) = (0, 0, 14, 16, 18);
    pub const TYPE_2: (i32, i32, i32, i32, i32) = (0, 768, 16, 16, 16);
    /// 24 x 24: the Controller page's sticks.
    pub const TYPE_3: (i32, i32, i32, i32, i32) = (2048, 1024, 24, 24, 2);
    /// The Controller page's START and SELECT shapes (one cell each).
    pub const START: (i32, i32, i32, i32, i32) = (1792, 3776, 39, 12, 1);
    pub const SELECT: (i32, i32, i32, i32, i32) = (1024, 3776, 48, 12, 1);
    pub const BAR: (i32, i32, i32, i32, i32) = (1536, 1536, 16, 20, 3);
    /// One texel stretched: Adjust Screen's marks (`menuWindowA`).
    pub const DOT: (i32, i32, i32, i32, i32) = (1344, 2064, 1, 1, 1);
    /// The sliders' fill, 8 x 8.
    pub const FILL: (i32, i32, i32, i32, i32) = (1344, 2048, 8, 8, 1);
}

/// One `ccMenuList`: its items and cursor.
#[derive(Clone, Debug, Default)]
struct MenuList {
    /// The name (`str`, the item text other lists show for it).
    name: Vec<u8>,
    /// The window title (`strT`).
    title: Vec<u8>,
    /// `disp`: the layout (7 the option window, 11 a dialog, 5 the title's).
    disp: i16,
    /// `x`: the window's width in cells.
    width: i16,
    items: Vec<i16>,
    /// `y`: the rows `Select` moves over.
    count: i16,
    select: i16,
    prev: i16,
    /// The Controller's setting in force (`index`), and Adjust Screen's
    /// position (`sx`, `sy`).
    index: i16,
    sx: i16,
    sy: i16,
    /// +0x0c, +0x10: `SaveMenu`'s file and card slot cursors; +0x18 the
    /// last `ccSaveSys` result it saw (a halfword).
    file: i16,
    port: i16,
    last: i16,
}

/// A `ccScFade` element on the menu layer.
#[derive(Clone, Copy, Debug)]
struct Fade {
    c0: u32,
    c1: u32,
    cnt: u32,
    tcnt: u32,
}

impl Fade {
    fn colour(&self) -> u32 {
        u32::from_le_bytes(fade::colour(self.c0, self.c1, self.cnt, self.tcnt))
    }
}

/// Where the task picks up next frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resume {
    /// The top of the loop, after its `Breath`.
    Top,
    /// Inside a close, after `WakeAll; Disp; Breath`: `still = 0` and the
    /// flip back on, then the loop's own `Disp`.
    AfterWake,
    /// `SaveSelMenu`'s close: as [`Resume::AfterWake`], then its
    /// `DispInfo` once more.
    AfterWakeInfo,
    /// The Controller waiting on `XCONTROL.CCS` (`Disp; Breath` until the
    /// load task is done; the port's load is done the next frame).
    ControllerLoad,
    /// The Controller's close: two frames of `Disp; Breath`, then back.
    ControllerEnd(u8),
}

/// `ccMenuWindow::DispSelectCursol`'s state: the fading bars and the wing.
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

    /// `InitCursol(f)` (0x001b78b0).
    fn init(&mut self, f: bool) {
        self.sel = if f { -1 } else { -2 };
        self.idx = 0;
        self.a = [0; 12];
    }

    /// `DispSelectCursol(x, y, w, sn, a, 3)` (0x001b7910) at `ccSystem`'s
    /// frame `rate` (the bars' steps divided by 3 - rate, the wing's by
    /// (10 - k)(3 - rate) and 4.5 / rate): cells (grid, code, x, y, w, h,
    /// grey, alpha).
    fn disp(&mut self, x: f32, y: f32, w: i32, sn: i16, alpha: i16, rate: u32) -> Vec<Cell> {
        let mut out = Vec::new();
        let rate = rate.clamp(1, 2);
        let div = 3 - rate as i16;
        if self.sel == -2 {
            self.wx = [x - 16.0; 6];
            self.wy = [y - 18.0; 6];
        }
        if sn != self.sel {
            self.sel = sn;
            let mut m = 255;
            for (j, &a) in self.a.iter().enumerate() {
                if a < m {
                    m = a;
                    self.idx = j;
                }
            }
            self.a[self.idx] = 128;
            self.w[self.idx] = ((w - 1) * 14) as f32;
            self.x[self.idx] = x + 6.0;
            self.y[self.idx] = y - 2.0;
        }
        for s in 0..12 {
            if self.a[s] == 0 {
                continue;
            }
            if s == self.idx {
                if self.a[s] < 48 {
                    self.a[s] = 96;
                } else {
                    let step = ((32 - self.a[s]) / 16) / div;
                    self.a[s] += if step >= 0 { -1 } else { step };
                }
            } else {
                let step = ((self.a[s] >> 3) / div).max(4);
                self.a[s] -= step;
                if self.a[s] < 8 {
                    self.a[s] = 0;
                    continue;
                }
            }
            if self.a[s] > alpha {
                self.a[s] = alpha;
            }
            let a = self.a[s];
            let (bx, by, bw) = (self.x[s], self.y[s], self.w[s]);
            out.push(Cell { grid: grid::BAR, code: 0, x: bx, y: by, w: 16.0, h: 20.0, grey: 128, alpha: a, ctrl: 0 });
            out.push(Cell {
                grid: grid::BAR,
                code: 1,
                x: bx + 16.0,
                y: by,
                w: bw,
                h: 20.0,
                grey: 128,
                alpha: a,
                ctrl: 0,
            });
            out.push(Cell {
                grid: grid::BAR,
                code: 2,
                x: bx + 16.0 + bw,
                y: by,
                w: 16.0,
                h: 20.0,
                grey: 128,
                alpha: a,
                ctrl: 0,
            });
        }
        let cur = f32::from(self.a[self.idx]);
        let off = if cur > 96.0 { cur / 10.0 } else { cur / 12.0 - 4.0 };
        let (tx, ty) = (x + (off - 16.0), y + (off - 18.0));
        for k in (0..6).rev() {
            if k > 0 {
                let d = (10 - k) as f32 * (3.0 - rate as f32);
                self.wx[k] = self.wx[k - 1] + (tx - self.wx[k - 1]) / d;
                self.wy[k] = self.wy[k - 1] + (ty - self.wy[k - 1]) / d;
                let g = (128 - 8 * k) as u8;
                let a = ((48 - 8 * k) as i16).min(alpha);
                out.push(Cell {
                    grid: grid::TYPE_2,
                    code: 10,
                    x: self.wx[k],
                    y: self.wy[k],
                    w: 20.0,
                    h: 20.0,
                    grey: g,
                    alpha: a,
                    ctrl: 0,
                });
            } else {
                let d = 4.5 / rate as f32;
                self.wx[0] += (tx - self.wx[0]) / d;
                self.wy[0] += (ty - self.wy[0]) / d;
                out.push(Cell {
                    grid: grid::TYPE_2,
                    code: 10,
                    x: self.wx[0],
                    y: self.wy[0],
                    w: 20.0,
                    h: 20.0,
                    grey: 128,
                    alpha,
                    ctrl: 0,
                });
            }
        }
        out
    }
}

/// One window cell of the menu's drawing.
#[derive(Clone, Copy, Debug)]
struct Cell {
    grid: (i32, i32, i32, i32, i32),
    code: u8,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grey: u8,
    alpha: i16,
    /// `ctrl`'s 0x20 (mirrored) and 0x40 (upside down) bits.
    ctrl: u8,
}

impl Cell {
    /// A cell whose colour comes with it in the frame.
    fn at(grid: (i32, i32, i32, i32, i32), code: u8, (x, y, w, h): (f32, f32, f32, f32), alpha: i16) -> Self {
        Cell { grid, code, x, y, w, h, grey: 128, alpha, ctrl: 0 }
    }

    /// A cell of `grid` at its own size.
    fn plain(grid: (i32, i32, i32, i32, i32), code: u8, x: f32, y: f32, alpha: i16) -> Self {
        Cell::at(grid, code, (x, y, grid.2 as f32, grid.3 as f32), alpha)
    }
}

/// A frame of the menu's drawing, sent in `Disp`'s order.
#[derive(Default)]
struct MenuDraw {
    /// `menuKanji` rows: (row, x, y, colour index).
    rows: Vec<(u8, f32, f32, usize)>,
    /// `menuFont` texts: (text, x, y, colour index).
    font: Vec<(Vec<u8>, f32, f32, usize)>,
    /// The four name kanji (+0x30): (kanji, row, x, y, colour index), and
    /// what each has extracted.
    sets: Vec<(usize, u8, f32, f32, usize)>,
    set_rows: [Vec<u8>; 4],
    /// `menuWindowA` cells (the colour index with each).
    win_a: Vec<(Cell, usize)>,
    /// `menuWindow` cells (the colour index with each; the cursor's grey
    /// cells carry their own).
    win: Vec<(Cell, Option<usize>)>,
    /// `menuMask`: the Controller's picture at (x, y).
    mask: Option<(f32, f32)>,
}

/// The option menus' texts.
#[derive(Clone, Debug, Default)]
struct Texts {
    on_off: Vec<u8>,
    voice: Vec<u8>,
    sound: Vec<u8>,
    ctrl: Vec<u8>,
    /// The picture's labels: settingKanji 0, 1 and 2.
    ctrl_btn: Vec<u8>,
    ctrl_mov: Vec<u8>,
    ctrl_cam: Vec<u8>,
    /// Vibrate's, Voiceover's and Movie Text's information: for row 0 and
    /// row 1.
    info: [[Vec<u8>; 2]; 3],
    /// The Controller's help: two lines for each scheme.
    help: [[Vec<u8>; 2]; 4],
    screen: [Vec<u8>; 2],
}

/// What the menu needs from the desktop each frame.
pub struct MenuCtx<'a> {
    pub save: &'a mut SaveState,
    pub req: &'a mut Vec<Request>,
    pub pad: &'a Pad,
    /// `ccGame.enableReset`.
    pub enable_reset: bool,
    pub names: Names,
    /// `saveSys`, which the save menus after the staff roll (8, 9) drive;
    /// None where there is no desktop (the title).
    pub save_sys: Option<&'a mut SaveSys>,
}

/// `ccDtMenu`.
pub struct DtMenu {
    /// +0x00 `menu`, +0x02 `menuNext`: -1 none.
    pub menu: i16,
    pub menu_next: i16,
    /// +0x04: 0 hidden, 1 opening, 2 open, 3 closing.
    pub menu_status: i16,
    /// +0x06 `openReqNum`: a menu asked for, -1 none.
    pub open_req: i16,
    /// +0x08 `still`: the other layers' pictures are frozen.
    pub still: bool,
    /// Every other task sleeps (`ccSleepAllThread` until `ccWakeAllThread`).
    pub asleep: bool,
    /// `saveData.camType` (+0x8429) as the Controller page draws it: read
    /// when the page opens, set by its OK.
    cam_type: i8,
    /// +0x0a: the menu's step.
    pub proccess: i16,
    wait_count: i16,
    /// +0x0e `exceptionDisp`: the open option menu draws its own page
    /// (`SaveMenu`: 1 the card slots, 2 the files).
    exception_disp: i16,
    /// +0x10 `externFlag`: the event's "done" (a plain line shown).
    pub extern_flag: bool,
    /// +0x16 `forbid`: START does nothing (the staff roll).
    pub forbid: bool,
    /// +0x1c: the menu window's alpha.
    pub alpha: i16,
    /// `ccGame.status == 1`: the title screen's menu (`ccSetupDemo`), set
    /// by the title. Opening a menu then puts nothing to sleep, freezes
    /// nothing, plays no sound 16 and dims nothing; decide and back sound
    /// 4 and 7; the lists and option menus draw no select cursor but the
    /// chosen row in colour 20, and the lists' rows are 24 apart.
    pub title: bool,
    /// +0x214 `temp`: Sound's four settings while it is open.
    temp: [i16; 4],
    /// The row an on / off menu marks (the field's value), and Sound's
    /// output, as the save holds them.
    on_off_row: usize,
    sound_output: i16,
    fade: Option<Fade>,
    resume: Resume,
    lists: Vec<MenuList>,
    /// `menuKanji`'s rows: what was last extracted.
    rows: Option<Vec<u8>>,
    dialog: Vec<u8>,
    /// `resetMenuInfo`: the first question's line, the second's two.
    reset_info: [Vec<u8>; 3],
    /// The port's quit prompt ([`DtMenu::open_quit`]): the Reset window
    /// asks [`QUIT_INFO`] instead, once, and OK sets `quit_ok`.
    quit: bool,
    pub quit_ok: bool,
    texts: Texts,
    cursor: SelectCursor,
    /// `ccSystem`'s frame rate the last `disp` ran at (2 after the staff
    /// roll, for the save menus).
    frame_rate: u32,
    /// `ccMsg`.
    pub msg: MsgWindow,
    /// `ccMsg +0x24` as the save menu last set it (1 a message to
    /// acknowledge, 3 an operation, 0 acknowledged).
    pub msg_cursol: i16,
    /// The save menus' texts.
    save_texts: save::SaveTexts,
    /// `ccThSaveSys` as `SaveMenu`'s `StartReq` started it: 0 none, 1 made
    /// this frame (its first run only breathes), 2 running
    /// (`MainProccess` each frame before the menu).
    pub save_task: u8,
    /// `saveSys->info` as the last frame left it, for the file list.
    save_records: [SaveDataInfo; INFO_COUNT],
    /// The disc's (`volumeNum`): clear data of it or later shows as such.
    volume: Volume,
    window: Option<WindowTexture>,
    control: Option<WindowTexture>,
    /// What the menu did this frame, for `tools/test_desktop_menu_rs.py`.
    #[cfg(feature = "trace")]
    pub trace: Vec<String>,
}

/// The game's fixed-width menu text from its words: each word's Shift-JIS
/// in a 16-glyph slot, space-padded, as the game's strings hold them (its
/// code copies slot k with `ccKanjiStrcat(buf, str + 16 * k, 16)`).
pub fn slots(words: &[&str]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|w| {
            let mut b = encode(w);
            b.resize(SLOT, b' ');
            b
        })
        .collect()
}

/// A menu text slot's width.
const SLOT: usize = 16;

/// The port's quit prompt's lines, in the voice of the game's Reset
/// (`resetMenuInfo`): not the game's text.
pub const QUIT_INFO: [&str; 2] = ["#YQuit#W and close the game.", "#YData not saved will be lost.#W"];

impl DtMenu {
    /// `ccThDtMenu`'s `new ccDtMenu` (0x00169b60) with `InitMenuList`
    /// (0x0016a0b0): the lists from `dtMenuElementData` and the option
    /// texts (`piney_data::tables::dtmenu`); with the window texture and the
    /// Controller's picture.
    pub fn new(volume: Volume, window: Option<WindowTexture>, control: Option<WindowTexture>) -> Self {
        use piney_data::tables::dtmenu as t;
        let text = |s: Option<&str>| s.map(encode).unwrap_or_default();
        let mut lists: Vec<MenuList> = t::ELEMENTS
            .iter()
            .map(|e| {
                let (disp, width, items) = e.data.map_or((0, 0, Vec::new()), |d| (d.disp, d.width, d.items.to_vec()));
                MenuList {
                    name: text(e.name),
                    title: text(e.title),
                    disp,
                    width,
                    count: items.len() as i16,
                    items,
                    prev: -1,
                    ..MenuList::default()
                }
            })
            .collect();
        lists.resize(LISTS, MenuList { prev: -1, ..MenuList::default() });
        let texts = Texts {
            on_off: slots(*t::ON_OFF),
            voice: slots(*t::VOICE),
            sound: slots(*t::SOUND),
            ctrl: slots(*t::CTRL),
            ctrl_btn: slots(*t::CTRL_BTN),
            ctrl_mov: slots(*t::CTRL_MOV),
            ctrl_cam: slots(*t::CTRL_CAM),
            // The desktop's window shows each setting's first line.
            info: [*t::VIBRATION_INFO, *t::VOICE_INFO, *t::STRWIN_INFO].map(|i| [encode(i[0][0]), encode(i[1][0])]),
            help: [0, 1, 2, 3].map(|k| [encode(t::HELP[k][0]), encode(t::HELP[k][1])]),
            screen: [encode(*t::SCREEN_X), encode(*t::SCREEN_Y)],
        };
        let dialog = slots(*t::DIALOG);
        DtMenu {
            menu: -1,
            menu_next: -1,
            menu_status: 0,
            open_req: -1,
            still: false,
            asleep: false,
            cam_type: 0,
            proccess: 0,
            wait_count: 0,
            exception_disp: 0,
            extern_flag: false,
            forbid: false,
            alpha: 0,
            title: false,
            temp: [0; 4],
            on_off_row: 0,
            sound_output: 1,
            fade: None,
            resume: Resume::Top,
            lists,
            rows: None,
            dialog,
            reset_info: [encode(*t::RESET_INFO), encode(t::RESET_INFO2[0]), encode(t::RESET_INFO2[1])],
            quit: false,
            quit_ok: false,
            texts,
            cursor: SelectCursor::new(),
            msg: MsgWindow::of(volume),
            msg_cursol: 0,
            frame_rate: 1,
            save_texts: save::SaveTexts::of(volume),
            save_task: 0,
            save_records: [SaveDataInfo::default(); INFO_COUNT],
            volume,
            window,
            control,
            #[cfg(feature = "trace")]
            trace: Vec::new(),
        }
    }

    /// Notes a call for the trace (feature `trace`).
    #[allow(unused_variables)]
    fn note(&mut self, s: impl FnOnce() -> String) {
        #[cfg(feature = "trace")]
        self.trace.push(s());
    }

    fn msg_open_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        self.note(|| format!("[\"open_info\",{}]", json_lines(&lines)));
        self.msg.open_info(lines, names);
    }

    fn msg_change_info(&mut self, lines: [Option<&[u8]>; 4], names: &Names) {
        self.note(|| format!("[\"change_info\",{}]", json_lines(&lines)));
        self.msg.change_info(lines, names);
    }

    fn msg_close(&mut self) {
        self.note(|| "[\"close\"]".to_string());
        self.msg.close();
    }

    /// `CheckMenuType` (0x0016a8e0).
    pub fn check_menu_type(&self) -> i32 {
        if self.menu == self.menu_next { i32::from(self.menu) } else { MENU_CHANGING }
    }

    /// `OpenMenu(n)` (0x0016a7a0).
    fn open_menu(&mut self, n: i16, req: &mut Vec<Request>) {
        self.open_req = -1;
        self.menu = n;
        self.menu_next = n;
        if let Some(l) = self.lists.get_mut(n as usize) {
            l.prev = -1;
        }
        self.menu_status = 1;
        self.proccess = 0;
        self.wait_count = 0;
        // ccGame.status 1 (the title) skips the sleep, the freeze and the
        // sound.
        if !self.title {
            if !self.still {
                self.asleep = true;
                self.still = true;
            }
            if n != MENU_FADE {
                req.push(Request::Se(SE_OPEN));
            }
        }
        self.cursor.init(false);
        self.extern_flag = false;
    }

    /// One frame of `ccThDtMenu` (0x0016ac00), from where it last breathed.
    pub fn frame(&mut self, x: &mut MenuCtx) {
        match self.resume {
            Resume::Top => {}
            Resume::AfterWake => {
                self.resume = Resume::Top;
                self.still = false;
                return;
            }
            Resume::AfterWakeInfo => {
                self.resume = Resume::Top;
                self.still = false;
                self.clear_flag_info(&x.names);
                return;
            }
            Resume::ControllerLoad => {
                self.resume = Resume::Top;
                self.controller_loaded(x);
                return;
            }
            Resume::ControllerEnd(1) => {
                self.resume = Resume::ControllerEnd(2);
                return;
            }
            Resume::ControllerEnd(_) => {
                self.resume = Resume::Top;
                self.back(true);
                return;
            }
        }
        if self.open_req >= 0 {
            let n = self.open_req;
            self.open_menu(n, x.req);
        }
        if self.check_menu_type() == MENU_CHANGING {
            if self.menu_status == 0 {
                self.menu = self.menu_next;
                if self.menu != -1 {
                    self.menu_status = 1;
                }
            }
            self.cursor.init(true);
        }
        match self.check_menu_type() {
            -1 => self.start_check(x),
            0 | 1 => self.system_menu(x),
            2 => self.controller(x),
            3 | 10 | 11 => self.on_off(x),
            4 => self.screen(x),
            5 => self.sound(x),
            6 => self.reset_menu(x),
            7 => self.fade_menu(),
            8 => self.save_sel_menu(x),
            9 => self.save_menu(x),
            _ => {}
        }
    }

    /// The loop's START test (0x0016ad44).
    fn start_check(&mut self, x: &mut MenuCtx) {
        if !x.enable_reset || x.pad.push.bits() & Buttons::START.bits() == 0 || self.forbid {
            return;
        }
        if !x.save.check_operate(OPERATE_START[0]) || !x.save.check_operate(OPERATE_START[1]) {
            return;
        }
        self.open_menu(MENU_SYSTEM, x.req);
    }

    /// `Select(0, 0)` (0x0016a900): up and down (repeat bits) move the
    /// current list's cursor over its `y` rows, wrapping, with sound 17.
    fn select(&mut self, x: &mut MenuCtx) {
        let Some(l) = self.lists.get_mut(self.menu as usize) else { return };
        let n = l.count;
        if n <= 0 {
            return;
        }
        let rep = x.pad.repeat.bits();
        if rep & Buttons::UP.bits() != 0 {
            l.select -= 1;
            x.req.push(Request::Se(SE_MOVE));
        } else if rep & Buttons::DOWN.bits() != 0 {
            l.select += 1;
            x.req.push(Request::Se(SE_MOVE));
        }
        if l.select < 0 {
            l.select = n - 1;
        } else if l.select >= n {
            l.select = 0;
        }
    }

    /// The decide / back test the menus share: cancel first (sound 19,
    /// the title's 7), then OK (18, the title's 4).
    fn key(&self, x: &mut MenuCtx) -> Option<bool> {
        let push = x.pad.push.bits();
        if push & x.save.cancel() != 0 {
            x.req.push(Request::Se(self.se_back()));
            Some(false)
        } else if push & x.save.ok() != 0 {
            x.req.push(Request::Se(if self.title { SE_TITLE_OK } else { SE_OK }));
            Some(true)
        } else {
            None
        }
    }

    fn se_back(&self) -> Se {
        if self.title { SE_TITLE_BACK } else { SE_BACK }
    }

    fn entry_dim(&mut self) {
        if self.fade.is_none() {
            self.fade = Some(Fade { c0: 0, c1: DIM_COLOUR, cnt: 0, tcnt: DIM_FRAMES });
        }
    }

    /// `ContinueFade(fade, 6, 0)`: from the colour it has reached to none.
    fn undim(&mut self) {
        if let Some(f) = self.fade.as_mut() {
            *f = Fade { c0: f.colour(), c1: 0, cnt: 0, tcnt: DIM_FRAMES };
        }
    }

    fn fade_done(&self) -> bool {
        self.fade.is_none_or(|f| f.cnt >= f.tcnt)
    }

    /// The close every menu ends with once its fade is out: `menuNext`
    /// -1, the window closing, every task awake; the flip comes back after
    /// the next breath.
    fn close_all(&mut self) {
        self.fade = None;
        self.menu_next = -1;
        self.menu_status = 3;
        if self.still {
            self.asleep = false;
            self.resume = Resume::AfterWake;
        }
    }

    /// `FadeMenu` (0x0016ee20), menu 7: the event's message behind a dim,
    /// until the event says it is done (`externFlag`).
    fn fade_menu(&mut self) {
        match self.proccess {
            0 => {
                self.entry_dim();
                self.menu_status = 0;
                self.proccess = 1;
            }
            1 => {
                if self.extern_flag {
                    self.undim();
                    self.proccess = 2;
                }
            }
            2 if self.fade_done() => self.close_all(),
            _ => {}
        }
    }

    /// `SystemMenu` (0x0016aeb0), menu 0: the option list.
    fn system_menu(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        match self.proccess {
            0 => {
                let mut buf = Vec::new();
                for &it in &self.lists[m].items {
                    let name = self.lists.get(it as usize).map_or(&[][..], |l| &l.name[..]);
                    str_cat(&mut buf, name, 16);
                }
                self.rows = Some(buf);
                // The title's menu has no dim (its fade stays -1).
                if !self.title {
                    self.entry_dim();
                }
                let l = &mut self.lists[m];
                let n = l.count;
                if l.select >= n {
                    l.select = n - 1;
                }
                if l.select < 0 {
                    l.select = 0;
                }
                self.proccess = 1;
            }
            1 => {
                self.select(x);
                match self.key(x) {
                    Some(false) => {
                        self.cursor.init(true);
                        if self.fade.is_some() {
                            self.undim();
                            self.proccess = 2;
                        } else {
                            self.close_all();
                        }
                    }
                    Some(true) => {
                        self.cursor.init(true);
                        let l = &self.lists[m];
                        let next = l.items.get(l.select as usize).copied().unwrap_or(-1);
                        self.menu_next = next;
                        if let Some(t) = self.lists.get_mut(next as usize) {
                            t.prev = self.menu;
                        }
                        self.proccess = 0;
                        self.wait_count = 0;
                        self.menu_status = 3;
                    }
                    None => {}
                }
            }
            2 if self.fade_done() => self.close_all(),
            _ => {}
        }
    }

    /// The field an on / off menu sets, its rows and information texts, and
    /// the row that stands for the field's value.
    fn on_off_setting(&self, x: &MenuCtx) -> (usize, usize, Vec<u8>, usize) {
        let (field, rows, k) = match self.menu {
            MENU_VIBRATE => (offset::VIBRATION, self.texts.on_off.clone(), 0),
            MENU_VOICE => (offset::VOICE, self.texts.voice.clone(), 1),
            _ => (offset::STR_WIN_MODE, self.texts.on_off.clone(), 2),
        };
        let v = x.save.save.u8(field) as i8;
        let row = if self.menu == MENU_VOICE {
            match v {
                1 => 0,
                0 => 1,
                _ => 2,
            }
        } else if v != 0 {
            0
        } else {
            1
        };
        (field, k, rows, row)
    }

    /// `VibrationMenu` (0x0016cdf0), `VoiceMenu` (0x001708a0), `StrwinMenu`
    /// (0x00170f40): two rows, the one in force marked; OK sets the field
    /// and says so.
    fn on_off(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        let (field, k, rows, row) = self.on_off_setting(x);
        if self.proccess == 0 {
            self.rows = Some(rows);
            let l = &mut self.lists[m];
            l.select = if row == 0 { 0 } else { 1 };
            l.width = 6;
            l.count = 2;
            let info = self.texts.info[k][row.min(1)].clone();
            self.msg_open_info([Some(&info), None, None, None], &x.names);
            self.exception_disp = 1;
            self.proccess = 1;
        }
        self.select(x);
        match self.key(x) {
            Some(false) => {
                if self.menu == MENU_VIBRATE {
                    self.cursor.init(true);
                }
                self.msg_close();
                self.back(true);
            }
            Some(true) => {
                if self.menu == MENU_VIBRATE {
                    self.cursor.init(true);
                }
                let sel = self.lists[m].select;
                x.save.save.set_u8(field, if sel == 0 { 1 } else { 0 });
                if self.menu == MENU_VIBRATE {
                    x.req.push(Request::Vibration { on: sel == 0 });
                }
                let info = self.texts.info[k][sel.clamp(0, 1) as usize].clone();
                self.msg_change_info([Some(&info), None, None, None], &x.names);
                self.cursor.init(true);
            }
            None => {}
        }
        self.on_off_row = self.on_off_setting(x).3;
    }

    /// `ScreenMenu` (0x0016d4d0): the picture moved with the pad while it
    /// is open (`SetDisplayOffset` every frame); cancel leaves.
    fn screen(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        if self.proccess == 0 {
            let l = &mut self.lists[m];
            l.sx = x.save.save.i16(offset::SCREEN_X) + 48;
            l.sy = x.save.save.i16(offset::SCREEN_Y) + 16;
            self.exception_disp = 1;
            self.wait_count = 0;
            self.proccess = 1;
            return;
        }
        self.wait_count = (self.wait_count + 1) % 60;
        let rep = x.pad.repeat.bits();
        let l = &mut self.lists[m];
        if rep & Buttons::DOWN.bits() != 0 {
            x.req.push(Request::Se(SE_MOVE));
            l.sy = (l.sy + 1).min(SCREEN_Y_RANGE);
        } else if rep & Buttons::UP.bits() != 0 {
            x.req.push(Request::Se(SE_MOVE));
            l.sy = (l.sy - 1).max(0);
        }
        if rep & Buttons::RIGHT.bits() != 0 {
            x.req.push(Request::Se(SE_MOVE));
            l.sx = (l.sx + 3).min(SCREEN_X_RANGE);
        } else if rep & Buttons::LEFT.bits() != 0 {
            x.req.push(Request::Se(SE_MOVE));
            l.sx = (l.sx - 3).max(0);
        }
        let (ox, oy) = (l.sx - 48, l.sy - 16);
        x.req.push(Request::DisplayOffset { x: i32::from(ox), y: i32::from(oy) });
        x.save.save.set_i16(offset::SCREEN_X, ox);
        x.save.save.set_i16(offset::SCREEN_Y, oy);
        if x.pad.push.bits() & x.save.cancel() != 0 {
            x.req.push(Request::Se(self.se_back()));
            self.back(true);
        }
    }

    /// `SoundMenu` (0x0016dc80): three volumes in 32 steps and the output,
    /// set as they move (`ccSaveData::SetSoundEnv`).
    fn sound(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        let s = &x.save.save;
        if self.proccess == 0 {
            self.exception_disp = 1;
            self.lists[m].count = 4;
            self.temp = [
                s.i16(offset::MAIN_VOL) * 32 / 256,
                s.i16(offset::BGM_VOL) * 32 / 256,
                s.i16(offset::SE_VOL) * 32 / 256,
                s.i16(offset::OUTPUT),
            ];
            self.sound_output = s.i16(offset::OUTPUT);
            self.proccess = 1;
            return;
        }
        self.select(x);
        let i = self.lists[m].select.clamp(0, 3) as usize;
        let rep = x.pad.repeat.bits();
        let mut changed = false;
        if rep & Buttons::LEFT.bits() != 0 {
            if self.temp[i] > 0 {
                self.temp[i] -= 1;
                x.req.push(Request::Se(SE_MOVE));
                changed = true;
            } else {
                self.temp[i] = 0;
            }
        } else if rep & Buttons::RIGHT.bits() != 0 {
            let max = if i == 3 { 1 } else { 32 };
            if self.temp[i] < max {
                self.temp[i] += 1;
                x.req.push(Request::Se(SE_MOVE));
                changed = true;
            } else {
                self.temp[i] = max;
            }
        }
        if x.pad.push.bits() & x.save.cancel() != 0 {
            x.req.push(Request::Se(self.se_back()));
            self.cursor.init(true);
            self.back(true);
        }
        self.sound_output = x.save.save.i16(offset::OUTPUT);
        if changed {
            let t = self.temp;
            self.sound_output = t[3];
            let s = &mut x.save.save;
            s.set_i16(offset::MAIN_VOL, (t[0] << 8) / 32);
            s.set_i16(offset::BGM_VOL, (t[1] << 8) / 32);
            s.set_i16(offset::SE_VOL, (t[2] << 8) / 32);
            s.set_i16(offset::OUTPUT, t[3]);
            x.req.push(Request::SoundEnv {
                main: i32::from(s.i16(offset::MAIN_VOL)),
                bgm: i32::from(s.i16(offset::BGM_VOL)),
                se: i32::from(s.i16(offset::SE_VOL)),
                output: i32::from(t[3]),
            });
        }
    }

    /// `ControllerMenu` (0x0016b320): the camera scheme (`camType`).
    fn controller(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        match self.proccess {
            0 => {
                self.exception_disp = 0;
                self.menu_status = 3;
                self.rows = Some(self.texts.ctrl.clone());
                self.cam_type = x.save.save.u8(offset::CAM_TYPE) as i8;
                let t = i16::from(self.cam_type).clamp(0, 3);
                let l = &mut self.lists[m];
                l.select = t;
                l.index = t;
                l.width = 6;
                l.count = 4;
                self.proccess = 1;
            }
            // `ccThControllerMenu` loads XCONTROL.CCS while the menu draws.
            1 => self.resume = Resume::ControllerLoad,
            2 => {
                self.select(x);
                match self.key(x) {
                    Some(false) => {
                        self.cursor.init(true);
                        self.msg_close();
                        self.menu_status = 3;
                        self.proccess = 3;
                    }
                    Some(true) => {
                        self.cursor.init(true);
                        let l = &mut self.lists[m];
                        l.index = l.select;
                        let t = l.select;
                        x.save.save.set_u8(offset::CAM_TYPE, t as u8);
                        self.cam_type = t as i8;
                        x.req.push(Request::CameraType(i32::from(t)));
                        self.controller_help(x);
                    }
                    None => {}
                }
            }
            3 if self.menu_status == 0 => {
                self.exception_disp = 0;
                self.resume = Resume::ControllerEnd(1);
            }
            _ => {}
        }
    }

    /// The load done: the picture up, the window opening, the help.
    fn controller_loaded(&mut self, x: &mut MenuCtx) {
        self.menu_status = 1;
        self.exception_disp = 1;
        self.controller_help(x);
        self.proccess = 2;
    }

    /// `ChangeInfo(help[index])`, moved to (39, 444).
    fn controller_help(&mut self, x: &mut MenuCtx) {
        let i = self.lists[MENU_CONTROLLER as usize].index.clamp(0, 3) as usize;
        let [a, b] = self.texts.help[i].clone();
        self.msg_change_info([Some(&a), Some(&b), None, None], &x.names);
        self.msg.set_pos(39.0, 444.0);
    }

    /// `ResetMenu` (0x0016e930), menu 6, "Title Screen": two questions,
    /// then `ChangeRequest(1, 7)` (back to the title, the save reset).
    fn reset_menu(&mut self, x: &mut MenuCtx) {
        let m = self.menu as usize;
        match self.proccess {
            0 => {
                self.rows = Some(self.dialog.clone());
                self.lists[m].select = 1;
                self.wait_count = 0;
                if self.quit {
                    let [a, b] = QUIT_INFO.map(encode);
                    self.msg_open_info([Some(&a), Some(&b), None, None], &x.names);
                } else {
                    let info = self.reset_info[0].clone();
                    self.msg_open_info([Some(&info), None, None, None], &x.names);
                }
                self.proccess = 1;
            }
            1 | 3 => {
                self.select(x);
                match self.key(x) {
                    Some(true) => {
                        self.msg_close();
                        if self.lists[m].select == 1 {
                            self.back(false);
                        } else if self.quit {
                            self.quit_ok = true;
                            self.back(false);
                        } else if self.proccess == 1 {
                            self.menu_status = 3;
                            self.proccess = 2;
                        } else {
                            x.req.push(Request::ChangeMode { num: 1, sf: 7 });
                            self.proccess = 4;
                        }
                    }
                    Some(false) => {
                        self.msg_close();
                        self.back(true);
                    }
                    None => {}
                }
            }
            2 if self.menu_status == 0 => {
                self.menu_status = 1;
                let (a, b) = (self.reset_info[1].clone(), self.reset_info[2].clone());
                self.msg_open_info([Some(&a), Some(&b), None, None], &x.names);
                self.lists[m].select = 1;
                self.wait_count = 0;
                self.proccess = 3;
            }
            _ => {}
        }
    }

    /// The port's quit prompt (not the game's): the Reset window (menu 6)
    /// opened on its own, asking [`QUIT_INFO`] with OK / Cancel; OK sets
    /// [`DtMenu::quit_ok`] and closes it.
    pub fn open_quit(&mut self) {
        self.quit = true;
        self.quit_ok = false;
        self.open_req = MENU_RESET;
    }

    /// Whether no menu is open, opening or closing.
    pub fn closed(&self) -> bool {
        self.menu == -1 && self.menu_next == -1 && self.menu_status == 0 && self.open_req < 0
    }

    /// Back to the list this one came from.
    fn back(&mut self, cancel: bool) {
        let m = self.menu as usize;
        self.menu_next = self.lists[m].prev;
        self.menu_status = 3;
        self.proccess = 0;
        self.wait_count = 0;
        self.cursor.init(cancel);
    }

    /// `ccDtMenu::Disp` (0x0016a150): the window's alpha; the option menu's
    /// own page (`ExceptionDisp` 0x0016a680, by `menu`); the list; then
    /// onto the menu layer in its send order: the message, `menuKanji`,
    /// `menuFont`, `menuWindowA`, `menuWindow`, `menuMask`, the dim.
    pub fn disp(&mut self, ctx: &mut Ctx, fonts: &Fonts, names: &Names, frame_rate: u32) {
        self.frame_rate = frame_rate;
        match self.menu_status {
            0 => self.alpha = 0,
            1 => {
                self.alpha += ALPHA_IN;
                if self.alpha >= 128 {
                    self.alpha = 128;
                    self.menu_status = 2;
                }
            }
            2 => self.alpha = 128,
            3 => {
                self.alpha -= ALPHA_OUT;
                if self.alpha <= 0 {
                    self.alpha = 0;
                    self.menu_status = 0;
                }
            }
            _ => {}
        }
        let mut f = MenuDraw::default();
        if self.exception_disp != 0 {
            match self.menu {
                MENU_CONTROLLER => self.draw_controller(&mut f),
                MENU_VIBRATE | MENU_VOICE | MENU_STRWIN => self.draw_on_off(&mut f),
                MENU_SCREEN => self.draw_screen(&mut f),
                MENU_SOUND => self.draw_sound(&mut f),
                MENU_SAVE => self.draw_save(&mut f),
                _ => {}
            }
        }
        if self.alpha != 0 {
            self.draw_list(&mut f);
        }
        let draws = self.msg.disp(frame_rate);
        render(&draws, ctx, fonts, names, self.window.as_ref());
        self.send(ctx, fonts, names, &f);
        if let Some(fd) = self.fade.as_mut() {
            draw_dim(ctx, fd);
            if fd.cnt < fd.tcnt {
                fd.cnt += 1;
            }
        }
    }

    /// `DispSelectCursol(x, y, w, sn, alpha, 3)` into the window cells.
    fn cursor_cells(&mut self, f: &mut MenuDraw, x: f32, y: f32, w: i32, sn: i16) {
        let a = self.alpha;
        for c in self.cursor.disp(x, y, w, sn, a, self.frame_rate) {
            f.win.push((c, None));
        }
    }

    fn square(f: &mut MenuDraw, x: f32, y: f32, w: i32, h: i32, a: i16, colour: usize) {
        for c in square_cells(x, y, w, h, None, a) {
            f.win.push((c, Some(colour)));
        }
    }

    /// The OPTION list, the Title Screen dialog, and the title's OPTION
    /// list (`disp` 5: no window, no cursor).
    fn draw_list(&mut self, f: &mut MenuDraw) {
        let Some(l) = self.lists.get(self.menu.max(0) as usize).cloned() else { return };
        let (wx, wy, rx, ry, h) = match l.disp {
            7 => (179.0, 136.0, 193.0, 152.0, 7),
            11 => (200.0, 248.0, 214.0, 264.0, 2),
            5 => (0.0, 0.0, TITLE_LIST_X, if self.title { TITLE_LIST_Y } else { 152.0 }, 0),
            _ => return,
        };
        let a = self.alpha;
        if l.disp != 5 {
            let title = if l.title.is_empty() { None } else { Some(&l.title[..]) };
            for c in square_cells(wx, wy, i32::from(l.width), h, title, a) {
                f.win.push((c, Some(C_WHITE)));
            }
        }
        for i in 0..l.count.max(0) {
            let mut colour = C_WHITE;
            if i == l.select {
                if l.disp != 5 {
                    // The cursor keeps the 20 pitch.
                    self.cursor_cells(f, wx, ry + f32::from(20 * i), i32::from(l.width), l.select);
                }
                if self.title {
                    colour = C_TITLE_CHOSEN;
                }
            }
            let y = if self.title { ry + TITLE_ROW_PITCH * f32::from(i) } else { ry + f32::from(20 * i) };
            f.rows.push((i as u8, rx, y, colour));
        }
    }

    /// A row's colour on the title (the chosen one 20, the rest white), or
    /// `None` off the title.
    fn title_colour(&self, i: i16, select: i16) -> Option<usize> {
        self.title.then_some(if i == select { C_TITLE_CHOSEN } else { C_WHITE })
    }

    /// `VibrationMenuDisp` (0x0016d140) and its twins: the two rows, the
    /// one in force in yellow.
    fn draw_on_off(&mut self, f: &mut MenuDraw) {
        let m = self.menu as usize;
        let l = self.lists[m].clone();
        Self::square(f, 200.0, 248.0, i32::from(l.width), i32::from(l.count), self.alpha, C_WHITE);
        let cur = self.on_off_row;
        for i in 0..l.count.max(0) {
            let y = 264.0 + f32::from(20 * i);
            let colour = match self.title_colour(i, l.select) {
                Some(c) => c,
                None => {
                    if i == l.select {
                        self.cursor_cells(f, 200.0, y, i32::from(l.width), l.select);
                    }
                    if usize::from(i as u8) == cur { C_SET } else { C_WHITE }
                }
            };
            f.rows.push((i as u8, 214.0, y, colour));
        }
    }

    /// `ScreenMenuDisp` (0x0016d6d0): the values and the marks round the
    /// frame, pulsing.
    fn draw_screen(&mut self, f: &mut MenuDraw) {
        let l = self.lists[MENU_SCREEN as usize].clone();
        let a = self.alpha;
        Self::square(f, 200.0, 188.0, 6, 2, a, C_WHITE);
        let num = |v: i16| format!("{v:4}").into_bytes();
        f.font.push((self.texts.screen[0].clone(), 216.0, 204.0, C_WHITE));
        f.font.push((num(l.sx / 3 - 16), 252.0, 204.0, C_WHITE));
        f.font.push((self.texts.screen[1].clone(), 216.0, 224.0, C_WHITE));
        f.font.push((num(l.sy - 16), 252.0, 224.0, C_WHITE));
        let t = if self.wait_count < 31 { self.wait_count } else { 60 - self.wait_count };
        let wa = a.min(16 + (t << 7) / 15);
        for r in SCREEN_MARKS {
            f.win_a.push((Cell::at(grid::DOT, 0, r, wa), C_MARK));
        }
    }

    /// `DispSlideBar(8, now, 32)` (0x001b97f0) at (x, y).
    fn slider(f: &mut MenuDraw, x: f32, y: f32, now: i16, a: i16) {
        let cell = |code, r| (Cell::at(grid::TYPE_1, code, r, a), Some(C_WHITE));
        f.win.push(cell(11, (x, y, 14.0, 16.0)));
        f.win.push(cell(12, (x + 14.0, y, 112.0, 16.0)));
        f.win.push(cell(13, (x + 126.0, y, 14.0, 16.0)));
        let w = (14 * 8 + 18) * i32::from(now) / 32;
        f.win.push(cell(29, (x + 1.0 + w as f32, y + 6.0, 14.0, 16.0)));
        f.win.push((Cell::at(grid::FILL, 0, (x + 6.0, y + 4.0, w as f32, 8.0), a), Some(C_CHOSEN)));
    }

    /// `SoundMenuDisp` (0x0016e060): three sliders and Mono / Stereo.
    fn draw_sound(&mut self, f: &mut MenuDraw) {
        let l = self.lists[MENU_SOUND as usize].clone();
        let a = self.alpha;
        self.rows = Some(self.texts.sound.clone());
        Self::square(f, 152.0, 96.0, 12, 10, a, C_WHITE);
        for (r, y) in [116.0, 166.0, 216.0].into_iter().enumerate() {
            Self::slider(f, 178.0, y + 16.0, self.temp[r], a);
            let colour = match self.title_colour(r as i16, l.select) {
                Some(c) => c,
                None => {
                    if l.select == r as i16 {
                        self.cursor_cells(f, 152.0, y, 12, r as i16);
                    }
                    C_WHITE
                }
            };
            f.rows.push((r as u8, 166.0, y, colour));
        }
        f.rows.push((3, 166.0, 264.0, self.title_colour(3, l.select).unwrap_or(C_WHITE)));
        let out = self.sound_output;
        for (k, x) in [(0i16, 178.0), (1, 250.0)] {
            if out == k {
                if l.select == 3 && !self.title {
                    self.cursor_cells(f, x - 14.0, 280.0, 4, 3 + out);
                }
                f.rows.push((4 + k as u8, x, 280.0, C_CHOSEN));
            } else {
                f.rows.push((4 + k as u8, x, 280.0, C_GREY));
            }
        }
    }

    /// `ControllerMenuDisp` (0x0016b7f0): the list of schemes, the one in force
    /// in yellow, the pad picture and the labelled boxes round it, as the
    /// field's (piney-fieldui's `option/disp.rs`): the buttons (320, 128) and
    /// START / SELECT (179, 300) in colour 0 with settingKanji 0, the left stick
    /// (39, 300) in colour 0 with 1, the right stick (347, 300) and shoulders
    /// (165, 56) in colour 7 with 2, the camera's labels and arrows by `camType`.
    fn draw_controller(&mut self, f: &mut MenuDraw) {
        let l = self.lists[MENU_CONTROLLER as usize].clone();
        let a = self.alpha;
        Self::square(f, 39.0, 56.0, 6, 4, a, C_WHITE);
        if !self.title {
            self.cursor_cells(f, 39.0, 72.0 + f32::from(20 * l.select), 6, l.select);
        }
        for i in 0..4 {
            let colour = self.title_colour(i, l.select).unwrap_or(if i == l.index { C_SET } else { C_WHITE });
            f.rows.push((i as u8, 53.0, 72.0 + f32::from(20 * i), colour));
        }
        f.mask = Some((88.0, 168.0));
        f.set_rows[0] = self.texts.ctrl_btn.clone();
        f.set_rows[1] = self.texts.ctrl_mov.clone();
        f.set_rows[2] = self.texts.ctrl_cam.clone();
        let cam = self.cam_type;
        let cell =
            |f: &mut MenuDraw, g, code: u8, x: f32, y: f32| f.win.push((Cell::plain(g, code, x, y, a), Some(C_WHITE)));
        let label = |f: &mut MenuDraw, set: usize, row: u8, x: f32, y: f32| f.sets.push((set, row, x, y, C_WHITE));
        // The buttons: triangle, circle, cross, square.
        Self::square(f, 320.0, 128.0, 9, 4, a, 0);
        for (k, (button, row)) in [(17u8, 0u8), (18, 1), (16, 3), (19, 2)].into_iter().enumerate() {
            let y = 144.0 + (20 * k) as f32;
            cell(f, grid::TYPE_2, button, 326.0, y);
            label(f, 0, row, 342.0, y);
        }
        // START and SELECT: their buttons, labels and shapes.
        Self::square(f, 179.0, 300.0, 10, 2, a, 0);
        cell(f, grid::TYPE_2, 8, 187.0, 313.0);
        label(f, 0, 4, 209.0, 307.0);
        cell(f, grid::START, 0, 193.0, 323.0);
        cell(f, grid::TYPE_2, 9, 187.0, 340.0);
        label(f, 0, 5, 209.0, 334.0);
        cell(f, grid::SELECT, 0, 193.0, 350.0);
        // The left stick: move, run.
        Self::square(f, 39.0, 300.0, 8, 2, a, 0);
        cell(f, grid::TYPE_3, 0, 47.0, 316.0);
        label(f, 1, 0, 71.0, 316.0);
        label(f, 1, 1, 71.0, 336.0);
        // The right stick: zoom (A) or rotate (B).
        Self::square(f, 347.0, 300.0, 7, 2, a, C_WHITE);
        cell(f, grid::TYPE_3, 1, 359.0, 316.0);
        match cam {
            0 | 1 => {
                label(f, 2, 0, 383.0, 316.0);
                label(f, 2, 1, 383.0, 336.0);
            }
            2 | 3 => label(f, 2, 2, 383.0, 326.0),
            _ => {}
        }
        // The shoulder buttons: L1 and R1 at 72, L2 and R2 at 92.
        Self::square(f, 165.0, 56.0, 20, 2, a, C_WHITE);
        cell(f, grid::TYPE_2, 32, 179.0, 72.0);
        cell(f, grid::TYPE_2, 33, 195.0, 72.0);
        match cam {
            0 | 1 => label(f, 2, 2, 211.0, 72.0),
            2 | 3 => label(f, 2, 4, 211.0, 72.0),
            _ => {}
        }
        cell(f, grid::TYPE_2, 34, 179.0, 92.0);
        cell(f, grid::TYPE_2, 35, 195.0, 92.0);
        if (0..=3).contains(&cam) {
            label(f, 2, 3, 211.0, 92.0);
        }
        cell(f, grid::TYPE_2, 36, 323.0, 72.0);
        cell(f, grid::TYPE_2, 37, 339.0, 72.0);
        match cam {
            // Mutation moved this label from 369 to 375.
            0 | 1 => label(f, 2, 2, if self.volume == Volume::Inf { 369.0 } else { 375.0 }, 72.0),
            2 | 3 => label(f, 2, 0, 355.0, 72.0),
            _ => {}
        }
        cell(f, grid::TYPE_2, 38, 323.0, 92.0);
        cell(f, grid::TYPE_2, 39, 339.0, 92.0);
        match cam {
            0 | 1 => label(f, 2, 4, 355.0, 92.0),
            2 | 3 => label(f, 2, 1, 355.0, 92.0),
            _ => {}
        }
        // The rotation arrows: A-2 and B-2 upside down (0x40), the left
        // ones mirrored (0x20).
        let (lx, rx, y0, y1, upside) = match cam {
            0 => (165.0, 355.0, 60.0, 76.0, false),
            1 => (165.0, 355.0, 56.0, 72.0, true),
            2 => (345.0, 375.0, 290.0, 306.0, false),
            3 => (345.0, 375.0, 286.0, 302.0, true),
            _ => return,
        };
        let (top, bottom) = if upside { (32, 14) } else { (14, 32) };
        let v = if upside { 0x40 } else { 0 };
        for (x, flip) in [(lx, 0x20u8), (rx, 0)] {
            for (y, code) in [(y0, top), (y1, bottom)] {
                let mut c = Cell::plain(grid::TYPE_1, code, x, y, a);
                c.ctrl = flip | v;
                f.win.push((c, Some(C_WHITE)));
            }
        }
    }

    /// The frame's sprites in `Disp`'s send order (each prepended, so the
    /// last sent is drawn first).
    fn send(&mut self, ctx: &mut Ctx, fonts: &Fonts, names: &Names, f: &MenuDraw) {
        let view = menu_view();
        let a = self.alpha.clamp(0, 255) as u8;
        // menuKanji: Init(3, 24), kt 0, a 128 x 16 texture row an item.
        // The rows are drawn even before anything is extracted (START's
        // frame), from an empty texture.
        if !f.rows.is_empty() {
            let rows = self.rows.clone().unwrap_or_default();
            let mut k = Kanji::init(3, 24);
            let strn = expand(&rows, names);
            k.clm = extract(fonts, &strn, Kt::SmallProportional, k.th, &mut k.tex);
            let id = ctx.uploads.len() as u32;
            ctx.uploads.push(k.upload(id, fonts));
            let mut s = Sprite::mask(MENU_LAYER, TexRef::Upload(id), k.th as i32, 24);
            (s.wu, s.wv, s.wi, s.su, s.sv, s.sx, s.sy) = (0, 0, 1, 128, 16, 128.0, 16.0);
            for &(row, x, y, c) in &f.rows {
                let t = SPRITE_COLOR_TABLE[c];
                s.colour = [t[0], t[1], t[2], a];
                s.dx = x;
                s.dy = y;
                s.make_packet(row, &view);
                #[cfg(feature = "trace")]
                self.trace.push(pk("kanji", row, x, y, 128.0, 16.0, s.colour));
            }
            s.send(&mut ctx.layers);
        }
        // The four name kanji (+0x30, Init(3, 32)): SaveMenu's file list,
        // eight rows each.
        for (i, rows) in f.set_rows.iter().enumerate() {
            let mine: Vec<_> = f.sets.iter().filter(|p| p.0 == i).collect();
            if mine.is_empty() {
                continue;
            }
            let mut k = Kanji::init(3, 32);
            let strn = expand(rows, names);
            k.clm = extract(fonts, &strn, Kt::SmallProportional, k.th, &mut k.tex);
            let id = ctx.uploads.len() as u32;
            ctx.uploads.push(k.upload(id, fonts));
            let mut s = Sprite::mask(MENU_LAYER, TexRef::Upload(id), k.th as i32, 32);
            (s.wu, s.wv, s.wi, s.su, s.sv, s.sx, s.sy) = (0, 0, 1, 128, 16, 128.0, 16.0);
            for &&(_, row, x, y, c) in &mine {
                let t = SPRITE_COLOR_TABLE[c];
                s.colour = [t[0], t[1], t[2], a];
                s.dx = x;
                s.dy = y;
                s.make_packet(row, &view);
                #[cfg(feature = "trace")]
                self.trace.push(pk(&format!("set{i}"), row, x, y, 128.0, 16.0, s.colour));
            }
            s.send(&mut ctx.layers);
        }
        // menuFont: xasc00's fixed cells with a shadow (drawn here with the
        // large fixed font).
        #[cfg(feature = "trace")]
        for (text, x, y, c) in &f.font {
            let t = SPRITE_COLOR_TABLE[*c];
            self.trace.push(format!(
                "[\"str\",\"{}\",{x},{y},[{},{},{},{a}]]",
                String::from_utf8_lossy(text),
                t[0],
                t[1],
                t[2]
            ));
        }
        for (text, x, y, c) in &f.font {
            let mut k = Kanji::init(3, 16);
            k.kt = Kt::LargeFixed;
            let t = SPRITE_COLOR_TABLE[*c];
            k.colour = [t[0], t[1], t[2], a];
            k.dx = *x;
            k.dy = *y;
            ctx.disp(fonts, &mut k, MENU_LAYER, &view, text, names);
        }
        #[cfg(feature = "trace")]
        for (name, list) in
            [("winA", f.win_a.iter().map(|&(c, i)| (c, Some(i))).collect::<Vec<_>>()), ("win", f.win.clone())]
        {
            for (c, colour) in list {
                let rgb = colour.map_or([c.grey; 3], |i| {
                    [SPRITE_COLOR_TABLE[i][0], SPRITE_COLOR_TABLE[i][1], SPRITE_COLOR_TABLE[i][2]]
                });
                self.trace.push(pk(
                    name,
                    c.code,
                    c.x,
                    c.y,
                    c.w,
                    c.h,
                    [rgb[0], rgb[1], rgb[2], c.alpha.clamp(0, 255) as u8],
                ));
            }
        }
        #[cfg(feature = "trace")]
        if let Some((x, y)) = f.mask {
            self.trace.push(pk("mask", 0, x, y, 256.0, 128.0, [128, 128, 128, a]));
        }
        let Some(win) = &self.window else { return };
        let mut cells = |list: Vec<(Cell, Option<usize>)>, blend: usize| {
            let mut s = Sprite::mask(MENU_LAYER, win.tex.clone(), win.tex_h, 300);
            s.alpha_blend = blend;
            for (c, colour) in list {
                let (wu, wv, su, sv, wi) = c.grid;
                (s.wu, s.wv, s.su, s.sv, s.wi) = (wu, wv, su, sv, wi);
                (s.sx, s.sy, s.dx, s.dy) = (c.w, c.h, c.x, c.y);
                let rgb = colour.map_or([c.grey; 3], |i| {
                    let t = SPRITE_COLOR_TABLE[i];
                    [t[0], t[1], t[2]]
                });
                s.colour = [rgb[0], rgb[1], rgb[2], c.alpha.clamp(0, 255) as u8];
                s.flip_u = c.ctrl & 0x20 != 0;
                s.flip_v = c.ctrl & 0x40 != 0;
                s.make_packet(c.code, &view);
            }
            s.send(&mut ctx.layers);
        };
        // menuWindowA (alphaBlendType 1), then menuWindow.
        cells(f.win_a.iter().map(|&(c, i)| (c, Some(i))).collect(), 1);
        cells(f.win.clone(), 0);
        // menuMask: the Controller's picture, whole.
        if let (Some((x, y)), Some(pic)) = (f.mask, &self.control) {
            let mut s = Sprite::mask(MENU_LAYER, pic.tex.clone(), pic.tex_h, 2);
            (s.wu, s.wv, s.wi, s.su, s.sv, s.sx, s.sy, s.dx, s.dy) = (0, 0, 1, 256, 128, 256.0, 128.0, x, y);
            s.colour = [128, 128, 128, a];
            s.make_packet(0, &view);
            s.send(&mut ctx.layers);
        }
    }

    /// Whether the desktop's own input is live (`CheckMenuType() == -1`).
    pub fn idle(&self) -> bool {
        self.check_menu_type() == -1
    }
}

/// `ccMenuWindow::DispSquare(w, h, title)` at (x, y): grid 1's frame, and
/// the title in grid 0 cells over its top edge.
fn square_cells(x: f32, y: f32, w: i32, h: i32, title: Option<&[u8]>, alpha: i16) -> Vec<Cell> {
    let inner = (14 * w) as f32;
    let mid = (16 + 20 * (h - 1)) as f32;
    let mut out = Vec::new();
    let rows = [(y, 16.0, [2, 6, 3]), (y + 16.0, mid, [8, 1, 9]), (y + 16.0 + mid, 16.0, [4, 7, 5])];
    for (ry, rh, codes) in rows {
        let xs = [(x, 14.0), (x + 14.0, inner), (x + 14.0 + inner, 14.0)];
        for ((cx, cw), code) in xs.into_iter().zip(codes) {
            out.push(Cell { grid: grid::TYPE_1, code, x: cx, y: ry, w: cw, h: rh, grey: 128, alpha, ctrl: 0 });
        }
    }
    if let Some(t) = title {
        let mut codes = vec![26u8, 38];
        for &c in t {
            codes.push(match c {
                b'0'..=b'9' => c - 20,
                b'A'..=b'Z' => c - 65,
                _ => 38,
            });
        }
        codes.extend([38, 27]);
        for (i, code) in codes.into_iter().enumerate() {
            let cx = x - 6.0 + (9 * i) as f32;
            out.push(Cell { grid: grid::TYPE_0, code, x: cx, y: y - 8.0, w: 9.0, h: 16.0, grey: 128, alpha, ctrl: 0 });
        }
    }
    out
}

/// A trace record of one packet.
#[cfg(feature = "trace")]
fn pk(obj: &str, code: u8, x: f32, y: f32, w: f32, h: f32, rgba: [u8; 4]) -> String {
    format!("[\"pk\",\"{obj}\",{code},{x},{y},{w},{h},[{},{},{},{}]]", rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Up to four lines as a JSON list, null for none.
#[cfg_attr(not(feature = "trace"), allow(dead_code))]
fn json_lines(lines: &[Option<&[u8]>; 4]) -> String {
    let v: Vec<String> = lines
        .iter()
        .map(|l| l.map_or("null".to_string(), |b| format!("\"{}\"", String::from_utf8_lossy(b).replace('"', "\\\""))))
        .collect();
    format!("[{}]", v.join(","))
}

/// `ccKanjiStrcat(buf, s, n)`: `s` padded with spaces to `n` glyphs.
fn str_cat(buf: &mut Vec<u8>, s: &[u8], n: usize) {
    let mut glyphs = 0;
    let mut i = 0;
    while i < s.len() && s[i] != 0 && glyphs < n {
        let c = s[i];
        let w = if (0x20..0x80).contains(&c) { 1 } else { 2 };
        buf.extend_from_slice(&s[i..(i + w).min(s.len())]);
        i += w;
        glyphs += 1;
    }
    buf.extend(std::iter::repeat_n(b' ', n.saturating_sub(glyphs)));
}

/// The dim: `ccScFade` on the menu layer.
fn draw_dim(ctx: &mut Ctx, f: &Fade) {
    fade::draw_on(ctx, MENU_LAYER, f.c0, f.c1, f.cnt, f.tcnt);
}

/// What a message is shown with (`Host::message_open` on the desktop).
pub fn open_message(
    menu: &mut DtMenu,
    kind: MessageKind,
    emode: i32,
    name: Option<&[u8]>,
    lines: &[&[u8]],
    names: &Names,
) {
    // The event asks for the fade menu when no menu is open.
    if menu.menu == -1 {
        menu.open_req = MENU_FADE;
    }
    let line = |i: usize| lines.get(i).copied();
    match kind {
        MessageKind::Speech => {
            let l = |i| Some(line(i).unwrap_or(&[]));
            menu.msg.change(emode, name, [l(0), l(1), l(2)], names);
        }
        MessageKind::Info | MessageKind::InfoNow => {
            // Empty lines go to ChangeInfo as null pointers (0x001a9674).
            let l = |i| line(i).filter(|s: &&[u8]| !s.is_empty());
            menu.msg.change_info([l(0), l(1), l(2), None], names);
            if kind == MessageKind::InfoNow {
                menu.msg.hide_frame();
            }
        }
    }
}
