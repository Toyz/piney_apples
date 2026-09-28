//! `Acces_control` (DESKTOP.PRG, acces.cpp): the Accessory screen, a list
//! of the wallpapers unlocked, their artist and caption, and choosing one
//! (`docs/engine/desktop.md`, "Accessory").
//!
//! Layout constants are the class's own (constructor 0x00403d50): the list
//! frame `laysize` (180, 130, 178 x 192) on the kanji layer, and the scroll
//! bar `scrsize`, drawn on a layer of its own (`ScrLayer`, priority 129)
//! with the same frame but the default aspect. Text is `ccKanji` with the
//! `Init` drop shadow in `ccSpriteColorTable[7]`; the cursor and the bars
//! are cells of `TEX_xddcurs1`.

use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::assets::Assets;
use crate::content::{self, WallData};
use crate::layers::{KANJI_LAYER, RESK_LAYER};
use crate::mail::{MailCtx, cursor_alpha, cursor_texture, frame};
use crate::save::{ORIGINAL_WALL_1, SaveState};
use crate::sprite::Sprite;
use crate::view::LayerView;
use crate::{Request, Se};

/// `Numremit`: rows the list shows.
pub const ROWS: i32 = 9;
/// `shiftRemit`, `UpRemit`: where the list starts scrolling down / up.
pub const SHIFT_REMIT: i32 = 6;
pub const UP_REMIT: i32 = 2;
/// Row pitch and first row.
pub const ROW_H: i32 = 17;
pub const ROW_Y: i32 = 3;
/// Where a row's text starts, and the two caption lines.
pub const TEXT_X: f32 = 32.0;
pub const CAPTION: [(f32, f32); 2] = [(18.0, 165.0), (18.0, 186.0)];
/// The cursor's x, and its y past its row's.
pub const CURSOR_X: i32 = 20;
pub const CURSOR_DY: i32 = 7;
/// `laysize` (x, y, hi, wid): the list's frame (`ControlDataSet` 0x004043c0:
/// `SetFrame(180, 130, 192, 178, 96, 89, 1, 6/7)`).
pub const LAYSIZE: [f32; 4] = [180.0, 130.0, 178.0, 192.0];
/// `scrsize` (x, y, hi, wid): the scroll bar; x is never read.
pub const SCRSIZE: [f32; 4] = [240.0, 12.0, 156.0, 5.0];
/// `ScrLayer`'s priority (`Init(129, 0)` in `SetData` 0x004040f0).
pub const SCR_LAYER: i16 = RESK_LAYER;
/// The rule between the list and the caption: `laysize.wid - 21` wide, 2
/// high, at (12, 135) on the scroll layer.
pub const RULE: (f32, f32, f32, f32) = (21.0, 2.0, 12.0, 135.0);
/// `CurRepeat`: frames of repeat input before the first repeat, then between.
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
/// `ORIGINAL_WALL_1/2/3`: ALTIMIT's own wallpapers, always listed last.
pub const ORIGINAL_WALLS: [usize; 3] =
    [ORIGINAL_WALL_1 as usize, ORIGINAL_WALL_1 as usize + 1, ORIGINAL_WALL_1 as usize + 2];

/// The cells of `TEX_xddcurs1` (wu, wv in 1/16 texels; w, h).
pub mod cell {
    pub const CURSOR: (i32, i32, i32, i32) = (208, 0, 14, 11);
    pub const SCROLL: (i32, i32, i32, i32) = (0, 368, 7, 7);
}

#[derive(Clone, Copy)]
struct Keys {
    push: u32,
    unpush: u32,
    repeat: u32,
    ok: u32,
    cancel: u32,
}

/// `Acces_control`.
pub struct Accessory {
    curs: (TexRef, i32),
    table: Vec<WallData>,
    /// `wallList`: `WallTbl` indices.
    pub list: Vec<usize>,
    /// +0x14 `listNO`: the cursor; +0x2c `MinMax`: the top row.
    pub list_no: i32,
    pub min_max: i32,
    pushflg: i32,
    pushcnt: i32,
    repeat_count: i16,
    /// +0x20: 0 enter, 1 the list, 3 leave.
    pub acces_mode: i32,
    wall_proccess: i32,
    change_flg: i32,
    kill_flg: i32,
    /// +0xd8 `OldWall`: the wallpaper up, as a `WallTbl` index.
    pub old_wall: usize,
    /// The wallpaper `ChangeWall` just put up, for the desktop's `ScrBack`.
    swap: Option<usize>,
    mask: Option<Sprite>,
    scrmask: Option<Sprite>,
    /// `ScrLayer`'s view.
    scr_view: LayerView,
}

impl Accessory {
    /// The constructor (0x00403d50) and `WallTbl` as the overlay loads it.
    pub fn new(assets: &Assets) -> piney_data::Result<Self> {
        let table = content::walls(assets.volume);
        Ok(Accessory {
            curs: cursor_texture(&assets.desk),
            table,
            list: Vec::new(),
            list_no: 0,
            min_max: 0,
            pushflg: 0,
            pushcnt: 0,
            repeat_count: REPEAT_FIRST,
            acces_mode: 0,
            wall_proccess: 0,
            change_flg: 0,
            kill_flg: 0,
            old_wall: 0,
            swap: None,
            mask: None,
            scrmask: None,
            scr_view: scr_frame(),
        })
    }

    /// `AddWallList` (0x004044f0), from `AddAllList`: each `WallTbl` row
    /// but the originals, in table order, whose bit of `dtWallpaperList`
    /// is set, then the three originals. The bits count only the other
    /// rows: Mutation's rows after the originals (52 on) take bits 49 on.
    pub fn add_wall_list(&mut self, save: &SaveState) {
        let mut bit = 0;
        for k in 0..self.table.len() {
            if ORIGINAL_WALLS.contains(&k) {
                continue;
            }
            if save.wallpaper_unlocked(bit) {
                self.list.push(k);
            }
            bit += 1;
        }
        self.list.extend(ORIGINAL_WALLS.iter().filter(|&&k| k < self.table.len()));
    }

    /// `ControlDataSet(kanjilayer, ScrBack, &WallTbl[dtWallpaper])`
    /// (0x004043c0), as ChooseMode enters: the list's frame on the kanji
    /// layer (kept after leaving) and the wallpaper up.
    pub fn control_data_set(&mut self, x: &mut MailCtx) {
        x.views.kanji = frame(LAYSIZE);
        self.old_wall = x.save.dt_wallpaper().min(self.table.len() - 1);
    }

    /// `MainAcces_control` (0x00403c40). Returns -1 to leave.
    pub fn main(&mut self, x: &mut MailCtx, pad: &Pad) -> i32 {
        let k = Keys {
            push: pad.push.bits(),
            unpush: pad.unpush.bits(),
            repeat: pad.repeat.bits(),
            ok: x.save.ok(),
            cancel: x.save.cancel(),
        };
        match self.acces_mode {
            0 => {
                if self.list.is_empty() {
                    if k.push & k.cancel != 0 {
                        self.acces_mode = 3;
                    }
                    return 0;
                }
                self.set_data();
                self.acces_mode = 1;
                1
            }
            1 => {
                self.wall_list(x, k);
                1
            }
            3 => {
                if self.kill_flg == 0 {
                    self.kill_flg = 1;
                    3
                } else {
                    // DelData (0x00404450), ResetData (0x004040d0).
                    self.mask = None;
                    self.scrmask = None;
                    self.pushflg = 0;
                    self.pushcnt = 0;
                    self.list_no = 0;
                    self.min_max = 0;
                    self.acces_mode = 0;
                    self.kill_flg = 0;
                    -1
                }
            }
            m => m,
        }
    }

    /// `SetData` (0x004040f0): the cursor's mask on the kanji layer, the
    /// bar's on `ScrLayer`.
    fn set_data(&mut self) {
        let (tex, h) = self.curs.clone();
        self.mask = Some(Sprite::mask(KANJI_LAYER, tex.clone(), h, 2));
        self.scrmask = Some(Sprite::mask(SCR_LAYER, tex, h, 3));
        self.scr_view = scr_frame();
    }

    /// `WallList` (0x00404710).
    fn wall_list(&mut self, x: &mut MailCtx, k: Keys) {
        if self.change_flg == 0 {
            self.list_move(x, k);
        }
        self.set_length();
        self.write_list(x);
        if k.push & k.ok != 0 {
            x.req.push(Request::Se(Se::OPEN));
            if self.change_flg == 0 {
                self.change_flg = 1;
                x.req.push(Request::EnableReset(false));
            }
        } else if self.change_flg == 0 && k.push & k.cancel != 0 {
            self.acces_mode = 3;
            self.change_flg = 0;
        }
        if self.change_flg == 1 {
            self.change_flg = self.change_wall(x);
            if self.change_flg == 0 {
                self.pushflg = 0;
                self.pushcnt = 0;
                x.req.push(Request::EnableReset(true));
            }
        }
    }

    /// `ChangeWall` (0x00404c70): 0 starts `ccThWallLoad` on the chosen
    /// wallpaper (or ends at once if it is up); 1, once loaded, puts it on
    /// `ScrBack` and writes `dtWallpaper`; 2 frees the old one.
    fn change_wall(&mut self, x: &mut MailCtx) -> i32 {
        let Some(&w) = self.list.get(self.list_no as usize) else { return 0 };
        match self.wall_proccess {
            0 => {
                if self.table[w].no == self.table[self.old_wall].no {
                    return 0;
                }
                self.pushflg = 0;
                self.pushcnt = 0;
                self.wall_proccess = 1;
                1
            }
            1 => {
                // The load task has finished (the port loads at once).
                self.swap = Some(w);
                x.save.set_dt_wallpaper(self.table[w].no as i8);
                self.wall_proccess = 2;
                1
            }
            _ => {
                self.old_wall = w;
                self.wall_proccess = 0;
                0
            }
        }
    }

    /// The wallpaper to put on `ScrBack` this frame, if any.
    pub fn take_swap(&mut self) -> Option<&WallData> {
        self.swap.take().map(|w| &self.table[w])
    }

    fn cur_repeat(&mut self, k: Keys, key: u32) -> bool {
        if k.push & key != 0 {
            self.pushflg = 1;
            return true;
        }
        if k.unpush & key != 0 {
            self.repeat_count = REPEAT_FIRST;
            self.pushflg = 0;
            return false;
        }
        if k.repeat & key != 0 {
            self.pushcnt += 1;
            if self.pushcnt == i32::from(self.repeat_count) {
                self.repeat_count = REPEAT_NEXT;
                self.pushcnt = 2;
                return true;
            }
        }
        false
    }

    /// `ListMove` (0x004053d0).
    fn list_move(&mut self, x: &mut MailCtx, k: Keys) {
        let n = self.list.len() as i32;
        if k.push & k.cancel != 0 {
            self.pushflg = 0;
            self.pushcnt = 0;
        }
        let mut d = 0;
        if self.cur_repeat(k, Buttons::UP.bits()) {
            x.req.push(Request::Se(Se(6)));
            self.list_no -= 1;
            d = 2;
            if self.list_no < 0 {
                self.list_no = n - 1;
                self.min_max = (n - ROWS).max(0);
            }
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            x.req.push(Request::Se(Se(6)));
            self.list_no += 1;
            d = 1;
            if self.list_no > n - 1 {
                self.list_no = 0;
                self.min_max = 0;
            }
        }
        if self.list_no - self.min_max > SHIFT_REMIT && d == 1 && n - (self.min_max + ROWS) > 0 {
            self.min_max += 1;
        }
        if d == 2 && self.min_max > 0 && self.list_no - UP_REMIT < self.min_max {
            self.min_max -= 1;
        }
    }

    /// `SetLength` (0x00404f20): the scroll bar and the rule, on `ScrLayer`.
    fn set_length(&mut self) {
        let view = self.scr_view;
        let packets = self.length_packets();
        let Some(m) = self.scrmask.as_mut() else { return };
        for [dx, dy, sx, sy] in packets {
            let (wu, wv, su, sv) = cell::SCROLL;
            m.wu = wu;
            m.wv = wv;
            m.wi = 1;
            m.su = su;
            m.sv = sv;
            m.sx = sx;
            m.sy = sy;
            m.dx = dx;
            m.dy = dy;
            m.make_packet(0, &view);
        }
    }

    /// SetLength's two packets, (dx, dy, sx, sy): the bar, `min(156, 156 *
    /// 9 / n)` long, then the rule.
    fn length_packets(&self) -> [[f32; 4]; 2] {
        use crate::eef::{add, div, from_int, le, mul, sub};
        let n = self.list.len() as i32;
        let travel = SCRSIZE[2];
        let mut len = mul(travel, div(from_int(ROWS << 4), from_int(n << 4)));
        if !le(len, travel) {
            len = travel;
        }
        let rest = sub(travel, len);
        let step = div(rest, from_int(n - ROWS));
        let dy = if self.min_max == n - ROWS {
            add(SCRSIZE[1], rest)
        } else {
            add(SCRSIZE[1], mul(step, from_int(self.min_max)))
        };
        [[sub(LAYSIZE[3], SCRSIZE[3]), dy, SCRSIZE[3], len], [RULE.2, RULE.3, sub(LAYSIZE[3], RULE.0), RULE.1]]
    }

    /// `WriteList` (0x004050c0): the rows, the selected one's caption, and
    /// the cursor.
    fn write_list(&mut self, x: &mut MailCtx) {
        let view = x.views.kanji;
        let mut cur_y = ROW_Y;
        let top = self.min_max.max(0) as usize;
        for (r, &w) in self.list.iter().skip(top).take(ROWS as usize).enumerate() {
            let r = r as i32;
            let y = (ROW_Y + ROW_H * r) as f32;
            let wall = &self.table[w];
            if self.list_no == self.min_max + r {
                let mut s = b"#G".to_vec();
                s.extend_from_slice(&wall.title);
                x.text(KANJI_LAYER, &view, TEXT_X, y, &s, true);
                x.text(KANJI_LAYER, &view, CAPTION[0].0, CAPTION[0].1, &wall.comment, true);
                x.text(KANJI_LAYER, &view, CAPTION[1].0, CAPTION[1].1, &wall.comment2, true);
                cur_y = ROW_Y + ROW_H * r + CURSOR_DY;
            } else {
                x.text(KANJI_LAYER, &view, TEXT_X, y, &wall.title, true);
            }
        }
        self.time_alpha_cur_draw(x, CURSOR_X, cur_y);
    }

    /// `TimeAlphaCurDraw(x, y, 0)` (0x00404860).
    fn time_alpha_cur_draw(&mut self, x: &mut MailCtx, px: i32, py: i32) {
        let a = cursor_alpha(x.count, x.frame_rate);
        let view = x.views.kanji;
        let Some(m) = self.mask.as_mut() else { return };
        m.colour[3] = a;
        let (wu, wv, w, h) = cell::CURSOR;
        m.cell(wu, wv, w, h);
        m.dx = px as f32;
        m.dy = py as f32;
        m.make_packet(0, &view);
        m.send(&mut x.ctx.layers);
        m.colour[3] = 0x80;
    }

    /// `DrawBack` (0x00404e40): both masks.
    pub fn draw_back(&mut self, x: &mut MailCtx) {
        if let Some(m) = self.mask.as_mut() {
            m.send(&mut x.ctx.layers);
        }
        if let Some(m) = self.scrmask.as_mut() {
            m.send(&mut x.ctx.layers);
        }
    }

    /// A wallpaper's `WallTbl` entry.
    pub fn wall(&self, i: usize) -> Option<&WallData> {
        self.table.get(i)
    }
}

/// `ScrLayer`'s `SetFrame(180, 130, 192, 178, 96, 89, 1, 1)`.
fn scr_frame() -> LayerView {
    LayerView::frame(LAYSIZE[0], LAYSIZE[1], LAYSIZE[3], LAYSIZE[2], 1.0, 1.0)
}

/// Hooks for `tools/test_desktop_rs.py`: the list cursor, the scroll bar
/// and the list build on given states, without drawing.
#[cfg(feature = "trace")]
pub mod probe {
    use super::*;

    impl Accessory {
        /// A list of `n` wallpapers and a fresh cursor.
        pub fn probe_list(&mut self, n: usize) {
            self.list = (0..n).map(|i| i % self.table.len()).collect();
            (self.list_no, self.min_max, self.pushflg, self.pushcnt, self.repeat_count) = (0, 0, 0, 0, REPEAT_FIRST);
        }

        /// ListMove with the pad (direct, push, unpush, repeat): (listNO,
        /// MinMax, pushflg, pushcnt, RepeatCount) after.
        pub fn probe_move(&mut self, x: &mut MailCtx, pad: [u32; 4]) -> (i32, i32, i32, i32, i16) {
            let k = Keys { push: pad[1], unpush: pad[2], repeat: pad[3], ok: x.save.ok(), cancel: x.save.cancel() };
            self.list_move(x, k);
            (self.list_no, self.min_max, self.pushflg, self.pushcnt, self.repeat_count)
        }

        /// SetLength at top row `min_max`: each packet's (dx, dy, sx, sy).
        pub fn probe_length(&mut self, min_max: i32) -> [[f32; 4]; 2] {
            self.min_max = min_max;
            self.length_packets()
        }

        /// AddWallList over a fresh list.
        pub fn probe_add(&mut self, save: &SaveState) -> Vec<usize> {
            self.list.clear();
            self.add_wall_list(save);
            self.list.clone()
        }
    }
}
