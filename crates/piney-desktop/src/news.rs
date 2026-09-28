//! `Web_control` (DESKTOP.PRG, webnews.cpp; constructor 0x0040d320): the News
//! screen, a list of headlines and the page image each opens
//! (`docs/engine/desktop.md`, "News"). The list frame `laysize` (116, 140, 180
//! x 364) is on the kanji layer with its scroll bar `scrsize`, the page's
//! `websize` with `scrweb`. Rows are `ccKanji` `Init(3, 16)` without the
//! shadow; the NEW tag, cursor and bars are cells of `TEX_xddcurs1`; a page is
//! one sprite cut from its 512 x 512 texture (`xddn_NNN`).

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::assets::Assets;
use crate::content::{self, HtmlData};
use crate::layers::KANJI_LAYER;
use crate::mail::{MailCtx, cursor_alpha, cursor_texture, frame};
use crate::save::SaveState;
use crate::sprite::Sprite;
use crate::{Request, Se};

/// `Numremit`: rows the list shows.
pub const ROWS: i32 = 12;
/// `shiftRemit`, `UpRemit`: where the list starts scrolling down / up.
pub const SHIFT_REMIT: i32 = 8;
pub const UP_REMIT: i32 = 3;
/// Row pitch and first row of the list (`WriteList`).
pub const ROW_H: f32 = 16.5;
pub const ROW_Y: f32 = 5.0;
/// Where a row's text starts.
pub const TEXT_X: f32 = 44.0;
/// `laysize` (x, y, hi, wid): the list's frame on the kanji layer
/// (`SetData` 0x0040d780: `SetFrame(116, 140, 364, 180, 182, 90, 1, 6/7)`).
pub const LAYSIZE: [f32; 4] = [116.0, 140.0, 180.0, 364.0];
/// `scrsize` (x, y, hi, wid): the list's scroll bar.
pub const SCRSIZE: [f32; 4] = [0.0, 12.0, 181.0, 7.0];
/// `scrweb` (x, y, hi, wid): the page's scroll bar.
pub const SCRWEB: [f32; 4] = [358.0, 12.0, 180.0, 5.0];
/// `websize` (x, y, hi, wid): the page window; only hi and wid are read.
pub const WEBSIZE: [f32; 4] = [118.0, 140.0, 206.0, 376.0];
/// The page sprite's width: `websize.wid - 23`.
pub const PAGE_W: i32 = 353;
/// `MovePoint`: pixels a page scrolls a step.
pub const MOVE_POINT: i32 = 6;
/// `CurRepeat`: frames of repeat input before the first repeat, then between.
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
/// `mask = ccMask(20, 0)`: the list's sprite.
pub const MASK_PACKETS: usize = 20;
/// `webnewsList[n]`: posted and read (`DeleteWeb`). 0 is not posted; any
/// other value (1 from event opcode 111) is posted and unread.
pub const NEWS_READ: u8 = 3;

/// The cells of `TEX_xddcurs1` (wu, wv in 1/16 texels; w, h).
pub mod cell {
    pub const NEW: (i32, i32, i32, i32) = (0, 880, 25, 9);
    pub const CURSOR: (i32, i32, i32, i32) = (208, 0, 14, 11);
    pub const SCROLL: (i32, i32, i32, i32) = (0, 368, 7, 7);
}

/// The pad as News tests it.
#[derive(Clone, Copy)]
struct Keys {
    direct: u32,
    push: u32,
    unpush: u32,
    repeat: u32,
    ok: u32,
    cancel: u32,
}

impl Keys {
    fn of(pad: &Pad, save: &SaveState) -> Keys {
        Keys {
            direct: pad.direct.bits(),
            push: pad.push.bits(),
            unpush: pad.unpush.bits(),
            repeat: pad.repeat.bits(),
            ok: save.ok(),
            cancel: save.cancel(),
        }
    }
}

/// A page's texture, found when its file loads.
#[derive(Clone, Debug)]
struct Page {
    tex: TexRef,
    tex_h: i32,
}

/// `Web_control`.
pub struct News {
    archive: Arc<Archive>,
    curs: (TexRef, i32),
    /// `HtmlTbl` in RAM: its `flg` is 0 in the file, 3 once `AddHtmlList`
    /// finds the headline read, 1 once a page closes.
    table: Vec<HtmlData>,
    flags: Vec<i32>,
    /// `htmllist`: `HtmlTbl` indices, in table order.
    pub list: Vec<usize>,
    /// +0x0c: 0 enter, 1 list, 2 loading, 3 page, 4 leave.
    pub web_mode: i32,
    /// +0x20 `listNO`: the cursor; +0x38 `MinMax`: the top row.
    pub list_no: i32,
    pub min_max: i32,
    /// +0x24 `Hi`: the page's first texel row shown.
    pub hi: i32,
    /// +0x28 `PreMode`: 0 opening, 1 shown, 3 closing.
    pub pre_mode: i32,
    pushflg: i32,
    pushcnt: i32,
    repeat_count: i16,
    /// +0x48 `NewWebNum`: headlines not yet read (the second NEW mark).
    pub new_web_num: i32,
    kill_flg: i8,
    subkill: i8,
    /// +0x60 `Html`: the headline opened.
    html: Option<usize>,
    html_proccess: i32,
    /// `LoadHtml`'s `param[0]`: the page file is in.
    loaded: Option<Option<Page>>,
    mask: Option<Sprite>,
    web_mask: Option<Sprite>,
    web_scrmask: Option<Sprite>,
}

impl News {
    /// Whether `HtmlTbl` row `row`'s headline is unread (its `flg` 0).
    pub fn unread(&self, row: usize) -> bool {
        self.flags.get(row) == Some(&0)
    }

    /// The constructor (0x0040d320) and the table as the overlay loads it.
    pub fn new(assets: &Assets) -> piney_data::Result<Self> {
        let table = content::news(assets.volume);
        let flags = table.iter().map(|h| h.flg).collect();
        Ok(News {
            archive: assets.archive.clone(),
            curs: cursor_texture(&assets.desk),
            table,
            flags,
            list: Vec::new(),
            web_mode: 0,
            list_no: 0,
            min_max: 0,
            hi: 0,
            pre_mode: 0,
            pushflg: 0,
            pushcnt: 0,
            repeat_count: REPEAT_FIRST,
            new_web_num: 0,
            kill_flg: 0,
            subkill: 0,
            html: None,
            html_proccess: 0,
            loaded: None,
            mask: None,
            web_mask: None,
            web_scrmask: None,
        })
    }

    /// `AddHtmlList` (0x0040d5c0), from `Desktop_control::AddAllList`: every
    /// posted headline in table order; a read one gets `flg` 3, any other
    /// counts as new.
    pub fn add_html_list(&mut self, save: &SaveState) {
        for i in 0..self.table.len() {
            let s = save.save.webnews(i) as i8;
            if s == 0 {
                continue;
            }
            if s == NEWS_READ as i8 {
                self.flags[i] = i32::from(NEWS_READ);
            } else {
                self.new_web_num += 1;
            }
            self.list.push(i);
        }
    }

    /// `SetData(ccsc, kanjilayer)` (0x0040d780), once as ChooseMode enters
    /// News: the list's frame on the kanji layer. (It also builds
    /// `PageBack`, `ANM_xddwebn0`, which nothing draws.)
    pub fn set_data(&mut self, x: &mut MailCtx) {
        x.views.kanji = frame(LAYSIZE);
    }

    /// `ResetWebNews` (0x0040d6e0).
    fn reset(&mut self) {
        self.web_mode = 0;
        self.list_no = 0;
        self.hi = 0;
        self.pre_mode = 0;
        self.min_max = 0;
        self.kill_flg = 0;
        self.mask = None;
    }

    /// `MainWebNews` (0x0040d1d0). Returns -1 to leave, else `Webmode`
    /// (3 while a page shows).
    pub fn main(&mut self, x: &mut MailCtx, pad: &Pad) -> i32 {
        let k = Keys::of(pad, x.save);
        match self.web_mode {
            0 => {
                if self.list.is_empty() {
                    if k.push & k.cancel != 0 {
                        self.web_mode = 4;
                    }
                    return 0;
                }
                self.web_data_set();
            }
            1 => self.web_list(x, k),
            2 | 3 => {
                if self.web_mode == 2 {
                    self.html_data_set(x);
                }
                if self.web_mode == 3 {
                    self.draw_page(x, k);
                }
            }
            4 => {
                if self.kill_flg == 0 {
                    if !self.list.is_empty() {
                        self.set_length(x);
                    }
                    self.kill_flg += 1;
                } else {
                    self.reset();
                    return -1;
                }
            }
            _ => {}
        }
        self.web_mode
    }

    /// `WebDataSet` (0x0040d860): the rows' kanji and the list's mask.
    fn web_data_set(&mut self) {
        self.web_mode = 1;
        self.subkill = 0;
        let (tex, h) = self.curs.clone();
        self.mask = Some(Sprite::mask(KANJI_LAYER, tex, h, MASK_PACKETS));
    }

    /// `WebList` (0x0040da30): the list's frame; OK opens the headline
    /// under the cursor.
    fn web_list(&mut self, x: &mut MailCtx, k: Keys) {
        self.move_cur(x, k);
        self.set_length(x);
        self.write_list(x);
        if k.push & k.cancel != 0 {
            self.web_mode = 4;
        } else if k.push & k.ok != 0 {
            x.req.push(Request::Se(Se::OPEN));
            self.html = self.list.get(self.list_no as usize).copied();
            self.web_mode = 2;
            x.req.push(Request::EnableReset(false));
        }
    }

    /// `HtmlDataSet` (0x0040db30): the list while the page loads.
    fn html_data_set(&mut self, x: &mut MailCtx) {
        self.set_length(x);
        self.write_list(x);
        if self.set_page() {
            self.web_mode = 3;
            self.pushflg = 0;
            self.pushcnt = 0;
            x.req.push(Request::EnableReset(true));
        }
    }

    /// `setPage` (0x0040ea50): the first call makes the page's two masks
    /// and starts `LoadHtml`; a later one, once the file is in, gives the
    /// page mask its texture.
    fn set_page(&mut self) -> bool {
        if self.html_proccess == 0 {
            let (tex, h) = self.curs.clone();
            self.web_scrmask = Some(Sprite::mask(KANJI_LAYER, tex, h, 1));
            // LoadHtml ("PAGE LOAD"): ccLoadFLAddOne(html), then param[0] = 1.
            self.loaded = Some(self.html.and_then(|h| page(&self.archive, &self.table[h])));
            self.html_proccess = 1;
            return false;
        }
        let Some(page) = self.loaded.take() else { return false };
        self.web_mask = page.map(|p| Sprite::mask(KANJI_LAYER, p.tex, p.tex_h, 1));
        self.html_proccess = 0;
        true
    }

    /// `DrawPage` (0x0040e700).
    fn draw_page(&mut self, x: &mut MailCtx, k: Keys) {
        match self.pre_mode {
            0 | 1 => {
                // SetWeb (0x0040e9e0).
                self.pre_mode = 1;
                if k.push & k.cancel != 0 {
                    self.pre_mode = 3;
                    x.req.push(Request::Se(Se::CLOSE));
                }
                self.web_move(k);
                self.previw_web(x);
                self.set_web_length(x);
            }
            3 => {
                if self.subkill == 0 {
                    self.subkill += 1;
                } else {
                    self.delete_web(x);
                }
            }
            _ => {}
        }
    }

    /// `DeleteWeb` (0x0040ec10): the page closes and its headline is read.
    fn delete_web(&mut self, x: &mut MailCtx) {
        if let Some(h) = self.html {
            if self.flags[h] == 0 {
                self.new_web_num -= 1;
            }
            self.flags[h] = 1;
            let no = self.table[h].no;
            if (0..piney_data::save::WEBNEWS_SLOTS as i32).contains(&no) {
                x.save.save.set_webnews(no as usize, NEWS_READ);
            }
        }
        self.web_mode = 1;
        self.pre_mode = 0;
        self.web_mask = None;
        self.web_scrmask = None;
        self.subkill = 0;
        self.hi = 0;
        x.views.kanji = frame(LAYSIZE);
    }

    /// `CurRepeat(key)`, inlined in MoveCur.
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

    /// `MoveCur` (0x0040dc10): the list cursor and scroll.
    fn move_cur(&mut self, x: &mut MailCtx, k: Keys) {
        let n = self.list.len() as i32;
        if k.push & k.cancel != 0 {
            self.pushflg = 0;
            self.pushcnt = 0;
            self.repeat_count = REPEAT_FIRST;
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
            self.list_no += 1;
            x.req.push(Request::Se(Se(6)));
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

    /// `SetLength` (0x0040e5d0): the list's scroll bar.
    fn set_length(&mut self, x: &mut MailCtx) {
        use crate::eef::{add, div, from_int, le, mul, sub};
        let n = self.list.len() as i32;
        let travel = SCRSIZE[2];
        let mut len = mul(travel, div(from_int(ROWS << 4), from_int(n << 4)));
        if !le(len, travel) {
            len = travel;
        }
        let rest = sub(travel, len);
        let step = div(rest, from_int(n - ROWS));
        let dy = if self.list_no == n - 1 {
            add(SCRSIZE[1], rest)
        } else {
            add(SCRSIZE[1], mul(step, from_int(self.min_max)))
        };
        let Some(m) = self.mask.as_mut() else { return };
        let (wu, wv, su, sv) = cell::SCROLL;
        m.wu = wu;
        m.wv = wv;
        m.wi = 1;
        m.su = su;
        m.sv = sv;
        m.sx = SCRSIZE[3];
        m.sy = len;
        m.dx = sub(sub(LAYSIZE[3], SCRSIZE[3]), 1.0);
        m.dy = dy;
        m.make_packet(0, &x.views.kanji);
    }

    /// `WriteList` (0x0040e0f0): the rows, the NEW tags and the cursor.
    fn write_list(&mut self, x: &mut MailCtx) {
        use crate::eef::{add, from_int, mul, to_int};
        let view = x.views.kanji;
        let mut cur_y = 0;
        let top = self.min_max.max(0) as usize;
        for (r, &h) in self.list.iter().skip(top).take(ROWS as usize).enumerate() {
            let f = mul(ROW_H, from_int(r as i32));
            let dy = add(ROW_Y, f);
            let mut s = Vec::new();
            if self.list_no == self.min_max + r as i32 {
                s.extend_from_slice(b"#G");
                cur_y = to_int(dy);
            }
            s.extend_from_slice(&self.table[h].title);
            x.text(KANJI_LAYER, &view, TEXT_X, dy, &s, false);
            if self.flags[h] == 0
                && let Some(m) = self.mask.as_mut()
            {
                let (wu, wv, w, hh) = cell::NEW;
                m.cell(wu, wv, w, hh);
                m.dx = 12.0;
                m.dy = add(7.0, f);
                m.make_packet(0, &view);
            }
        }
        self.time_alpha_cur_draw(x, 0, cur_y + 3);
    }

    /// `TimeAlphaCurDraw(x, y, 0, NULL)` (0x0040e3c0): the blinking cursor,
    /// sent with whatever the mask has queued.
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

    /// The page's height (`hight`).
    fn page_height(&self) -> i32 {
        self.html.map_or(0, |h| self.table[h].height)
    }

    /// `WebMove` (0x0040ded0): a page scrolls while up or down is held,
    /// `MovePoint` on held frames 1, 4, 6, 8, ...
    fn web_move(&mut self, k: Keys) {
        use crate::eef::{from_int, le, lt, sub, to_int};
        let height = from_int(self.page_height());
        let win = WEBSIZE[2];
        if k.push & k.cancel != 0 {
            self.pushflg = 0;
            self.pushcnt = 0;
        }
        if k.direct & Buttons::UP.bits() != 0 {
            self.pushflg = -1;
        } else if k.direct & Buttons::DOWN.bits() != 0 {
            self.pushflg = 1;
        }
        if (self.pushflg == -1 && k.direct & Buttons::UP.bits() == 0)
            || (self.pushflg == 1 && k.direct & Buttons::DOWN.bits() == 0)
        {
            self.pushflg = 0;
            self.pushcnt = 0;
        }
        if self.pushflg == -1 {
            self.pushcnt += 1;
            if self.pushcnt >= 4 {
                self.pushcnt = 2;
                self.hi -= MOVE_POINT;
            } else if self.pushcnt == 1 {
                self.hi -= MOVE_POINT;
            }
            if self.hi < 0 {
                self.hi = 0;
            }
        }
        if self.pushflg == 1 {
            self.pushcnt += 1;
            if self.pushcnt >= 4 {
                self.pushcnt = 2;
                self.hi += MOVE_POINT;
            } else if self.pushcnt == 1 {
                self.hi += MOVE_POINT;
            }
            let max = sub(height, win);
            if !lt(from_int(self.hi), max) {
                self.hi = to_int(max);
            }
        }
        if le(height, win) {
            self.hi = 0;
        }
    }

    /// `PreviwWeb` (0x0040ed10): rows `Hi..Hi + 206` of the page.
    fn previw_web(&mut self, x: &mut MailCtx) {
        let view = x.views.kanji;
        let hi = self.hi;
        let Some(m) = self.web_mask.as_mut() else { return };
        m.wu = 0;
        m.wv = hi << 4;
        m.wi = 1;
        m.sx = PAGE_W as f32;
        m.sy = WEBSIZE[2];
        m.su = PAGE_W;
        m.sv = WEBSIZE[2] as i32;
        m.dx = 0.0;
        m.dy = 0.0;
        m.make_packet(0, &view);
    }

    /// `SetWebLength` (0x0040e7e0): the page's scroll bar.
    fn set_web_length(&mut self, x: &mut MailCtx) {
        use crate::eef::{add, div, from_int, le, lt, mul, sub, to_int};
        let height = self.page_height();
        let q = from_int((height / MOVE_POINT) * MOVE_POINT);
        let win = WEBSIZE[2];
        let mp = from_int(MOVE_POINT);
        let travel = SCRWEB[2];
        let mut len = mul(travel, div(mul(mp, div(win, mp)), q));
        if le(from_int(height), win) {
            len = travel;
        }
        let dx = sub(SCRWEB[0], 1.0);
        let max = sub(from_int(height), win);
        let steps = to_int(div(max, mp));
        let step = div(sub(travel, len), from_int(steps));
        let k = if self.hi != 0 { from_int(self.hi / MOVE_POINT) } else { 0.0 };
        let dy = if self.hi != 0 && !lt(from_int(self.hi), max) {
            add(SCRWEB[1], sub(travel, len))
        } else {
            add(SCRWEB[1], mul(step, k))
        };
        let view = x.views.kanji;
        let Some(m) = self.web_scrmask.as_mut() else { return };
        let (wu, wv, su, sv) = cell::SCROLL;
        m.wu = wu;
        m.wv = wv;
        m.wi = 1;
        m.su = su;
        m.sv = sv;
        m.sx = SCRWEB[3];
        m.sy = len;
        m.dx = dx;
        m.dy = dy;
        m.make_packet(0, &view);
    }

    /// `DrawBack` (0x0040dba0): the page and its bar while a page shows,
    /// then the list's mask.
    pub fn draw_back(&mut self, x: &mut MailCtx) {
        if self.web_mode == 3
            && let Some(m) = self.web_mask.as_mut()
        {
            m.send(&mut x.ctx.layers);
            if let Some(s) = self.web_scrmask.as_mut() {
                s.send(&mut x.ctx.layers);
            }
        }
        if let Some(m) = self.mask.as_mut() {
            m.send(&mut x.ctx.layers);
        }
    }

    /// The headline open, as an `HtmlTbl` index, while its page shows.
    pub fn open_page(&self) -> Option<usize> {
        if self.web_mode == 3 { self.html } else { None }
    }

    /// Whether `DrawWebnews` draws the page animation twice (`PreMode` 3).
    pub fn closing(&self) -> bool {
        self.pre_mode == 3
    }

    /// A headline's `HtmlTbl` entry.
    pub fn headline(&self, i: usize) -> Option<&HtmlData> {
        self.table.get(i)
    }
}

/// `ccLoadFLAddOne(html)` and `SetTex(ccsName, chunk, 1)`: the page file
/// and its texture.
fn page(archive: &Archive, h: &HtmlData) -> Option<Page> {
    let ccs = Ccs::parse(archive.inflate_named(&h.ccs_name.to_ascii_lowercase()).ok()?).ok()?;
    let texture = ccs.find_object(&h.chunk)?;
    let (textures, _) = piney_data::texture::read(&ccs).ok()?;
    let t = textures.iter().find(|t| t.object == texture)?;
    Some(Page { tex: TexRef::Ccs { file: h.ccs_name.to_ascii_lowercase(), texture, clut: t.clut }, tex_h: 1 << t.th })
}

/// Hooks for `tools/test_desktop_rs.py`: the list cursor, the page scroll
/// and both scroll bars on given states, without drawing.
#[cfg(feature = "trace")]
pub mod probe {
    use super::*;

    impl News {
        /// A list of `n` headlines (table order, wrapping) and a fresh
        /// cursor: (listNO, MinMax, pushflg, pushcnt, RepeatCount) zero but
        /// the count at 30.
        pub fn probe_list(&mut self, n: usize) {
            self.list = (0..n).map(|i| i % self.table.len()).collect();
            (self.list_no, self.min_max, self.pushflg, self.pushcnt, self.repeat_count) = (0, 0, 0, 0, REPEAT_FIRST);
        }

        /// (listNO, MinMax, pushflg, pushcnt, RepeatCount).
        pub fn probe_state(&self) -> (i32, i32, i32, i32, i16) {
            (self.list_no, self.min_max, self.pushflg, self.pushcnt, self.repeat_count)
        }

        /// One MoveCur with the pad (direct, push, unpush, repeat).
        pub fn probe_move_cur(&mut self, x: &mut MailCtx, pad: [u32; 4]) {
            let k = probe_keys(x, pad);
            self.move_cur(x, k);
        }

        /// SetLength with the cursor at `list_no` and the top row at
        /// `min_max`: the bar's (dy, sy).
        pub fn probe_length(&mut self, x: &mut MailCtx, list_no: i32, min_max: i32) -> (f32, f32) {
            self.list_no = list_no;
            self.min_max = min_max;
            let (tex, h) = self.curs.clone();
            self.mask = Some(Sprite::mask(KANJI_LAYER, tex, h, MASK_PACKETS));
            self.set_length(x);
            let m = self.mask.as_ref().unwrap();
            (m.dy, m.sy)
        }

        /// A page `height` high open at row `hi`, as the probe's page.
        pub fn probe_page(&mut self, height: i32, hi: i32) {
            self.table[0].height = height;
            self.html = Some(0);
            self.hi = hi;
            (self.pushflg, self.pushcnt) = (0, 0);
        }

        /// SetWebLength: the bar's (dy, sy).
        pub fn probe_web_length(&mut self, x: &mut MailCtx) -> (f32, f32) {
            let (tex, h) = self.curs.clone();
            self.web_scrmask = Some(Sprite::mask(KANJI_LAYER, tex, h, 1));
            self.set_web_length(x);
            let m = self.web_scrmask.as_ref().unwrap();
            (m.dy, m.sy)
        }

        /// One WebMove: (Hi, pushflg, pushcnt) after.
        pub fn probe_web_move(&mut self, x: &mut MailCtx, pad: [u32; 4]) -> (i32, i32, i32) {
            let k = probe_keys(x, pad);
            self.web_move(k);
            (self.hi, self.pushflg, self.pushcnt)
        }
    }

    fn probe_keys(x: &MailCtx, pad: [u32; 4]) -> Keys {
        Keys { direct: pad[0], push: pad[1], unpush: pad[2], repeat: pad[3], ok: x.save.ok(), cancel: x.save.cancel() }
    }
}
