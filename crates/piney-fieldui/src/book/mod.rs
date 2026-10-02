//! The Ryu Books (`BOOK`, gcmn book.cpp, 0x0040c570 - 0x0041ad60): eight
//! pages of the player's records, each a window of counters, and the
//! desktop rewards a counter earns (`bookItemList`, given in order).
//!
//! Reading one is `ccUseItemRequest`'s `ccStartThread(ccThBook, 35,
//! 0x1000)`; the use waits in [`crate::menus::useitem`] while the task's
//! state is not 0. [`Task`] is `ccThBook` (0x0041a990): a fade, the book's
//! stream (112 + its page), then the `BOOK` object's frames. The breaths
//! inside the reward code are [`Step`]s.

mod pages;

use std::collections::VecDeque;

use piney_data::save::SaveData;
use piney_data::tables::book::Book as Tables;
use piney_data::tables::sjis::encode;
use piney_desktop::eef::from_int;
use piney_desktop::kanji::{Kt, str_width};

use crate::Request;
use crate::ctrl::{Ctx, Draw, MenuCtrl};
use crate::menus::personal::{check_fade, delete_fade};
use crate::spr::{Obj, Spr};
use crate::window::{Cursor, disp_scroll_bar, disp_square, set_type};

/// `saveData` offsets the books read and write.
pub mod save {
    /// `hyProccess[8][4]` (+0x683d): each book's rewards given, by row.
    pub const HY_PROCCESS: usize = 0x683d;
    /// `hyItem` (+0x685d): the rewards given so far (`bookItemList`'s next).
    pub const HY_ITEM: usize = 0x685d;
    /// `areaCount` (+0x6862).
    pub const AREA_COUNT: usize = 0x6862;
    /// `enemyKillCount[313]` (+0x68b7).
    pub const ENEMY_KILL: usize = 0x68b7;
    /// `playTime` (+0x8400), in frames.
    pub const PLAY_TIME: usize = 0x8400;
    /// `partyMemberFlag` (+0x2220).
    pub const PARTY_MEMBERS: usize = 0x2220;
    /// `dtWallpaperList[3]`, `dtBgmList[3]`, `dtStrList[5]` (+0x2238,
    /// +0x2244, +0x2250): the desktop's rewards, a bit each.
    pub const DT_WALLPAPER: usize = 0x2238;
    pub const DT_BGM: usize = 0x2244;
    pub const DT_STR: usize = 0x2250;
}

const SE_MOVE: i32 = 17;
const SE_OPEN: i32 = 16;
const SE_BACK: i32 = 19;
const SE_REWARD: i32 = 94;
const SE_ADD: i32 = 74;
const SE_START: i32 = 98;
/// `ccPad` bits: up and down.
const PAD_UP: u32 = 0x1000;
const PAD_DOWN: u32 = 0x4000;
/// Where the reward windows stand (`ccMsg` +0x184, +0x188).
const INFO_POS: (f32, f32) = (39.0, 430.0);
/// `ccSpriteColorTable` rows: a counter (7) and one at its cap (18).
const COL_TEXT: usize = 7;
const COL_OVER: usize = 18;
/// `msg[140]`, the page's kanji rows.
pub const MSG_ROWS: usize = 140;
/// `buttonLayer`: `ccLayer::Init(243)` with the menu's view.
pub const BUTTON_LAYER: i16 = 243;
/// `hiddenName`'s rows: book III's 67 characters, book IV's 303 enemies.
const CHARS: usize = 67;
const ENEMIES: usize = 303;

/// `CHARINFO` (0x78): a character of book III's list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharInfo {
    /// `name[32]`.
    pub name: Vec<u8>,
    pub trade_num: i32,
    pub is_friend: i32,
    pub is_online: i32,
    /// `item[16]`: (id, category, num) of each trade item; unset rows
    /// (-1, -1, -1).
    pub item: [(i16, i8, i8); 16],
    pub friendship: i32,
    pub party_time: i32,
    pub present: i32,
}

/// `ENEMYINFO` (0x24): book IV's list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnemyInfo {
    pub name: Vec<u8>,
    pub kill_cnt: i32,
}

/// `SCROLL_INFO`: a list's page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scroll {
    pub page_index_num: i32,
    pub page_index: i32,
    pub all_index_num: i32,
    pub all_index: i32,
    pub page_top: i32,
}

/// A stretch of the reward code (`GetBookItem` and what it calls) between
/// its breaths, each `Draw; ccBreathThread(1)`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    /// `GetBookItem`'s row `idx` of `table`: its test, then its windows.
    Row {
        table: pages::Table,
        idx: usize,
        value: i32,
        sel: i32,
    },
    /// `helpDisp` set.
    Help(i32),
    Se(i32),
    /// `EntryFlash(menuFade, 5, 0x80ffffff, ...)`.
    Flash,
    /// This many frames of `Draw; breath`.
    Frames(u32),
    /// `OpenInfo(l0, l1, l2, 0, -1, -1)` at [`INFO_POS`], `msgDispFlag`
    /// set; then [`Step::Wait`].
    Info([Option<Vec<u8>>; 3]),
    /// `Check(0)` until it answers (`Draw; breath` meanwhile), `Close`,
    /// `msgDispFlag` cleared.
    Wait,
    /// `BookAddItem`'s reward taken: its desktop bit, `hyItem` + 1.
    Add,
    /// `hyProccess[type][sel] = num`.
    Progress {
        sel: i32,
        num: i32,
    },
}

/// `BOOK` (0x4d24).
pub struct Book {
    /// +0x000 `type`: the book, 0-7.
    pub ty: i32,
    /// +0x004 `bg`: the `ccMask` over the screen (`str8800e`'s
    /// `TEX_x800bac1`).
    pub bg: Spr,
    /// +0x008 `win`; its select cursor.
    pub win: Spr,
    pub win_cursor: Cursor,
    /// +0x00c `button`, on `buttonLayer` (243).
    pub button: Spr,
    /// +0x014 `title`, +0x018 `msg[140]`: `ccKanji` rows.
    pub title: Spr,
    pub msg: Vec<Spr>,
    pub sel: i32,
    /// +0x24c `charData[68]` (`SetCharInfo` fills 67).
    pub char_data: Vec<CharInfo>,
    /// +0x222c `enemyData[303]`.
    pub enemy_data: Vec<EnemyInfo>,
    pub scroll_top: i32,
    pub is_sub_win_open: i32,
    pub now: i32,
    pub wait: i32,
    pub msg_disp_flag: i32,
    pub count_over: i32,
    pub check_value: [i32; 5],
    pub help_disp: i32,
    pub scroll: Scroll,
    pub key_wait: i32,
    pub cur_x: i32,
    pub cur_y: i32,
    pub sel_max: i32,
    pub cancel_flag: i32,
    pub exit: i32,
    steps: VecDeque<Step>,
}

/// What the book reads of the volume.
pub struct Env {
    pub t: &'static Tables,
    pub world: &'static piney_data::tables::world::World,
    /// `volumeNum`: 1 Infection ... 4 Quarantine.
    pub volume_num: i32,
}

impl Env {
    pub fn of(x: &Ctx) -> Env {
        let v = x.texts.volume;
        Env { t: piney_data::tables::book::of(v), world: piney_data::tables::world::of(v), volume_num: v as i32 + 1 }
    }
}

impl Book {
    /// `BOOK::BOOK(t)` (0x0040c7b0): the sprites, `SetCharInfo`, the enemy
    /// list, and the list pages of books III, IV and V.
    pub fn new(ty: i32, x: &Ctx, env: &Env) -> Book {
        let mut b = Book {
            ty,
            bg: Spr::new(Obj::BookBg, 1),
            win: Spr::new(Obj::BookWin, 256),
            win_cursor: Cursor::default(),
            button: Spr::new(Obj::BookButton, 256),
            title: Spr::new(Obj::BookTitle, 32),
            msg: (0..MSG_ROWS).map(|i| Spr::new(Obj::BookMsg(i as u8), 16)).collect(),
            sel: 0,
            char_data: set_char_info(x, env),
            enemy_data: Vec::new(),
            scroll_top: 0,
            is_sub_win_open: 0,
            now: 0,
            wait: 15,
            msg_disp_flag: 0,
            count_over: 0,
            check_value: [0; 5],
            help_disp: 1,
            scroll: Scroll::default(),
            key_wait: 0,
            cur_x: 0,
            cur_y: 0,
            sel_max: 0,
            cancel_flag: 0,
            exit: 0,
            steps: VecDeque::new(),
        };
        set_type(&mut b.win, 1);
        set_type(&mut b.button, 1);
        let save = &x.save.save;
        let hidden = encode(env.t.hidden_name);
        b.enemy_data = (0..ENEMIES)
            .map(|i| {
                let kill_cnt = i32::from(save.u8(save::ENEMY_KILL + i) as i8);
                let name = if kill_cnt == 0 {
                    hidden.clone()
                } else {
                    x.texts.enemy_names.get(i).cloned().unwrap_or_default()
                };
                EnemyInfo { name, kill_cnt }
            })
            .collect();
        let page = |sel_max, page_index_num, all_index_num| {
            (sel_max, Scroll { page_index_num, all_index_num, ..Scroll::default() })
        };
        let list = match ty {
            2 => Some(page(2, 7, CHARS as i32)),
            3 => Some(page(1, 9, ENEMIES as i32)),
            4 => Some(page(1, 9, 17)),
            _ => None,
        };
        if let Some((sel_max, scroll)) = list {
            b.sel_max = sel_max;
            b.scroll = scroll;
        }
        b
    }

    /// `CheckBookLimit(idx, val)` (0x0040c570): `val` past this book's cap
    /// for counter `idx` on the volume comes back as the cap, with
    /// `countOver` set.
    pub fn check_book_limit(&mut self, env: &Env, idx: i32, val: i32) -> i32 {
        self.count_over = 0;
        let vol = if env.volume_num == 0 { 1 } else { env.volume_num } - 1;
        if vol == 4 {
            return val;
        }
        let t = env.t;
        let (table, per): (&[i32], i32) = match self.ty {
            0 => (t.limits_1, 2),
            1 => (t.limits_2, 3),
            2 => (t.limits_3, 2),
            3 => (t.limits_4, 1),
            4 => (t.limits_5, 1),
            5 => (t.limits_6, 3),
            6 => (t.limits_7, 3),
            7 => (t.limits_8, 1),
            _ => return val,
        };
        match usize::try_from(idx + per * vol).ok().and_then(|k| table.get(k)) {
            Some(&cap) if cap < val => {
                self.count_over = 1;
                cap
            }
            _ => val,
        }
    }

    /// `msg[i]`'s colour: a counter at its cap red.
    fn over_colour(&mut self, i: usize) {
        let c = if self.count_over != 0 { COL_OVER } else { COL_TEXT };
        self.msg[i].set_colour(c);
    }

    /// `ccKanji::Disp(k, text, -1, 1, 1)` where `k` stands.
    fn text_of(k: &Spr, draws: &mut Vec<Draw>, text: Vec<u8>, kt: i32) {
        let rgba = [k.rgb[0], k.rgb[1], k.rgb[2], k.alpha as u8];
        draws.push(Draw::Text { obj: k.obj, text, dx: k.dx, dy: k.dy, rgba, count: -1, kt });
    }

    /// `ccKanji::Disp(msg[i], text, -1, 1, 1)`.
    fn text(&self, draws: &mut Vec<Draw>, i: usize, text: Vec<u8>, kt: i32) {
        Book::text_of(&self.msg[i], draws, text, kt);
    }

    /// `msg[i]` placed at (x, y).
    fn at(&mut self, i: usize, x: i32, y: i32) {
        self.msg[i].dx = from_int(x);
        self.msg[i].dy = from_int(y);
    }

    /// `BookOfs[type]`: (y, width, height).
    fn ofs(m: &MenuCtrl, ty: i32) -> (i32, i32, i32) {
        let k = (ty * 3) as usize;
        let o = &m.book_ofs;
        (o[k], o[k + 1], o[k + 2])
    }

    /// The window's left edge: `(width + 2) * 14` wide, centred.
    fn left(width: i32) -> i32 {
        (512 - (width + 2) * 14) / 2
    }

    /// `BOOK::Draw` (0x0040cb80): the page, the window with its cursor or
    /// scroll bar, the button, the cover behind.
    fn draw(&mut self, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) {
        self.check_value[..4].fill(0);
        pages::disp(self, m, x, env, draws);
        self.win.set_alpha(128);
        let (y, w, h) = Book::ofs(m, self.ty);
        let left = Book::left(w);
        self.win.dx = from_int(left);
        self.win.dy = from_int(y);
        disp_square(&mut self.win, w, h, None);
        let rate = x.frame_rate;
        let cursor = |b: &mut Book, n: i32, sn: i32| {
            b.win.dx = from_int(left);
            b.win.dy = from_int(y);
            let (cx, cy) = (from_int(b.cur_x - 14), from_int(b.cur_y));
            b.win_cursor.disp(&mut b.win, cx, cy, n, sn, 128, 3, rate);
        };
        let bar = |b: &mut Book, dx: i32, dy: i32| {
            b.win.dx = from_int(left + dx);
            b.win.dy = from_int(y + dy);
            let s = b.scroll;
            disp_scroll_bar(&mut b.win, 0, s.page_index_num, s.page_top, s.all_index_num);
        };
        let shown = self.sel + self.scroll.all_index;
        match self.ty {
            0 => cursor(self, 16, self.sel),
            1 => cursor(self, 19, self.sel),
            2 if self.is_sub_win_open == 0 => {
                cursor(self, 16, shown);
                bar(self, 252, 96);
            }
            3 if self.is_sub_win_open == 0 => {
                cursor(self, 16, shown);
                bar(self, 252, 76);
            }
            4 => {
                cursor(self, 16, shown);
                bar(self, 252, 76);
            }
            5 => cursor(self, 21, self.sel),
            6 => cursor(self, 16, self.sel),
            7 => {
                if self.is_sub_win_open == 0 {
                    cursor(self, 10, self.sel);
                }
                if self.is_sub_win_open == 2 {
                    cursor(self, 14, shown);
                    bar(self, 224, 76);
                }
            }
            _ => {}
        }
        draws.push(Draw::Send(self.win.take()));
        draws.push(Draw::Send(self.button.take()));
        let bg = &mut self.bg;
        bg.set_grid(256, 256, 512.0, 512.0, 0, 0, 1);
        bg.dx = 0.0;
        bg.dy = -16.0;
        bg.set_alpha(128);
        bg.make_packet(0);
        draws.push(Draw::Send(bg.take()));
    }

    /// `BOOK::PadControl` (0x0040d870): the page's own, then the cancel
    /// button closes the book.
    fn pad_control(&mut self, m: &mut MenuCtrl, x: &mut Ctx) {
        pages::pad_control(self, m, x);
        if self.is_sub_win_open == 0
            && self.cancel_flag == 0
            && self.wait == 0
            && x.pad.push.bits() & x.save.cancel() != 0
        {
            self.exit = 1;
        }
    }

    /// `GetBookItem(itemtbl, value, type, sel)` (0x0040ffb0): each row of
    /// `table` in turn ([`Step::Row`]).
    fn get_book_item(&mut self, env: &Env, table: pages::Table, value: i32, sel: i32) {
        for idx in 0..table.rows(env.t).len() {
            self.steps.push_back(Step::Row { table, idx, value, sel });
        }
    }

    /// The loop's body (`Breath` above it): `Draw` and `CheckItemGet`, then
    /// the pad, or after a sub-window's cancel the button let go.
    fn frame(&mut self, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) {
        if self.steps.is_empty() {
            self.draw(m, x, env, draws);
            pages::check_item_get(self, env);
        }
        if !self.run_steps(m, x, env, draws) {
            return;
        }
        if self.cancel_flag == 0 {
            self.pad_control(m, x);
        } else {
            self.cancel_flag = i32::from(x.pad.push.bits() & x.save.cancel() != 0);
        }
    }

    /// The reward steps: true once none is left, false when one breathed.
    fn run_steps(&mut self, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) -> bool {
        while let Some(step) = self.steps.pop_front() {
            match step {
                Step::Row { table, idx, value, sel } => {
                    // GetBookItem: helpDisp 0 for a row whose count is
                    // reached, the book's GetBookNNItem, helpDisp 1.
                    self.steps.push_front(Step::Help(1));
                    let row = table.rows(env.t)[idx];
                    if value >= row.cnt {
                        self.help_disp = 0;
                        let num = idx as i32 + 1;
                        for s in pages::row_steps(self, env, &x.save.save, row, sel, num).into_iter().rev() {
                            self.steps.push_front(s);
                        }
                    }
                }
                Step::Help(v) => self.help_disp = v,
                Step::Se(n) => x.se(n),
                Step::Flash => {
                    m.menu_fade.entry_flash(5, 0x80ff_ffff);
                }
                Step::Frames(n) => {
                    if n > 1 {
                        self.steps.push_front(Step::Frames(n - 1));
                    }
                    self.draw(m, x, env, draws);
                    return false;
                }
                Step::Info(lines) => {
                    let names = x.save.names();
                    m.msg.open_info([lines[0].as_deref(), lines[1].as_deref(), lines[2].as_deref(), None], &names);
                    m.msg.set_pos(INFO_POS.0, INFO_POS.1);
                    self.msg_disp_flag = 1;
                    self.steps.push_front(Step::Wait);
                }
                Step::Wait => {
                    if !crate::menus::tutorial::check(m, x) {
                        self.steps.push_front(Step::Wait);
                        self.draw(m, x, env, draws);
                        return false;
                    }
                    m.msg.close();
                    self.msg_disp_flag = 0;
                }
                Step::Add => add_item_apply(&mut x.save.save, env),
                Step::Progress { sel, num } => {
                    let a = save::HY_PROCCESS + (self.ty * 4 + sel) as usize;
                    x.save.save.set_u8(a, num as u8);
                }
            }
        }
        true
    }

    /// `hyProccess[type][sel]`: the rewards of this row given.
    fn progress(&self, save: &SaveData, sel: i32) -> i32 {
        i32::from(save.u8(save::HY_PROCCESS + (self.ty * 4 + sel) as usize) as i8)
    }
}

/// `BookAddItem` (0x0040d990): SE 74 and the reward's window
/// (`bookItemList[hyItem]`: 2 a wallpaper, 1 a BGM, 4 a movie, its number
/// in two digits), then [`Step::Add`]; it always answers 1.
fn add_item_steps(save: &SaveData, env: &Env) -> Vec<Step> {
    let mut out = vec![Step::Se(SE_ADD)];
    let hy = usize::from(save.u8(save::HY_ITEM));
    if let Some(r) = env.t.item_list.get(hy) {
        let msg = |k: usize| env.world.book.get(k).map(|s| encode(s)).unwrap_or_default();
        let line = |k: usize| {
            let mut l = msg(k);
            l.extend_from_slice(format!("{:02}", r.item).as_bytes());
            Some(l)
        };
        match r.category {
            2 => out.push(Step::Info([line(1), Some(msg(0)), None])),
            1 => out.push(Step::Info([line(2), Some(msg(0)), None])),
            4 => out.push(Step::Info([line(3), Some(msg(0)), Some(encode(env.t.movie_notice))])),
            _ => {}
        }
    }
    out.push(Step::Add);
    out
}

/// `BookAddItem`'s rest: the reward's desktop bit, `hyItem` + 1.
fn add_item_apply(save: &mut SaveData, env: &Env) {
    let hy = usize::from(save.u8(save::HY_ITEM));
    if let Some(r) = env.t.item_list.get(hy) {
        let list = match r.category {
            2 => Some(save::DT_WALLPAPER),
            1 => Some(save::DT_BGM),
            4 => Some(save::DT_STR),
            _ => None,
        };
        if let Some(base) = list {
            let n = r.item - 1;
            let at = (base as i32 + 4 * (n / 32)) as usize;
            let bit = 1u32 << (n % 32).rem_euclid(32);
            save.set_i32(at, (save.i32(at) as u32 | bit) as i32);
        }
    }
    save.set_u8(save::HY_ITEM, save.u8(save::HY_ITEM).wrapping_add(1));
}

/// `ccKanjiStrWidth(text, 0)`.
fn width(x: &Ctx, text: &[u8]) -> i32 {
    str_width(x.fonts, text, Kt::SmallProportional, &x.save.names())
}

/// `itoa_ns(buf, n)` (gcmn 0x0040c410): `n`'s digits, the leading zeros
/// of its four places dropped.
pub fn itoa_ns(n: i32) -> Vec<u8> {
    let digit = |v: i32| (v + 48) as u8;
    let mut out = vec![digit(n / 1000)];
    let mut started = out[0] != b'0';
    let mut pos = usize::from(started);
    let mut place = |out: &mut Vec<u8>, d: u8| {
        out.truncate(pos);
        out.push(d);
        if started || d != b'0' {
            started = true;
            pos += 1;
        }
    };
    let r = n % 1000;
    place(&mut out, digit(r / 100));
    let r = r % 100;
    place(&mut out, digit(r / 10));
    out.truncate(pos);
    out.push(digit(r % 10));
    out
}

/// `SetCharInfo(charData)` (gcmn 0x0040bcd0): the members 1-17, the
/// people 30-65 and the other players 66-79, with their trades.
fn set_char_info(x: &Ctx, env: &Env) -> Vec<CharInfo> {
    use crate::menus::merchant::check_trade_count;
    let save = &x.save.save;
    let hidden = encode(env.t.hidden_name);
    let clamp = |v: i32| if v >= 10000 { 9999 } else { v };
    let items = |base: usize| {
        let mut it = [(-1i16, -1i8, -1i8); 16];
        let mut k = 0;
        for j in 0..16 {
            let a = base + 4 * j;
            let (id, cat, num) = (save.i16(a), save.u8(a + 2) as i8, save.u8(a + 3) as i8);
            if cat >= 0 && id >= 0 {
                it[k] = (id, cat, num);
                k += 1;
            }
        }
        it
    };
    let mut out = Vec::with_capacity(CHARS);
    for i in 1..18usize {
        let id = i as i32;
        let mut c = CharInfo {
            name: x.texts.char_names.get(i).cloned().unwrap_or_default(),
            trade_num: check_trade_count(x, 4, id),
            is_online: x.world.spc.iter().position(|&(v, _)| v == id).map_or(-1, |k| k as i32),
            is_friend: save.i32(save::PARTY_MEMBERS) & (1 << i),
            item: items(0xe3c + (i - 1) * 64),
            friendship: i32::from(save.i16(0x7488 + 0xdc * i + 0xda)),
            party_time: save.i32(0x73b4 + 4 * i) / 60,
            present: save.i32(0x73f8 + 4 * i),
        };
        if c.trade_num == -1 {
            c.trade_num = 0;
        }
        c.trade_num = clamp(c.trade_num);
        if c.is_friend == 0 || c.is_friend == -1 {
            c.trade_num = 0;
            c.is_friend = -1;
            c.name = hidden.clone();
        }
        out.push(c);
    }
    let person = |id: i32| {
        let mut c = CharInfo {
            name: x.texts.npc_names.get(id as usize).cloned().unwrap_or_default(),
            trade_num: check_trade_count(x, 8, id),
            is_online: i32::from(x.world.npcs.contains(&id)),
            ..CharInfo::default()
        };
        c.is_friend = c.trade_num;
        if c.is_friend == 0 {
            c.is_friend = 1;
        } else if c.is_friend == -1 {
            c.trade_num = 0;
            c.is_friend = -1;
            c.name = hidden.clone();
        }
        c.trade_num = clamp(c.trade_num);
        c
    };
    for k in 0..36usize {
        let mut c = person(30 + k as i32);
        c.item = items(0x127c + 64 * k);
        out.push(c);
    }
    for k in 0..14usize {
        let mut c = person(66 + k as i32);
        c.item = [(-1, -1, -1); 16];
        let mut n = 0;
        for j in 0..3 {
            if save.u8(0x1e7c + 3 * k + j) as i8 != 0 {
                c.item[n] = x.texts.tpc_trade.get(k).and_then(|r| r.get(j)).copied().unwrap_or((-1, -1, -1));
                n += 1;
            }
        }
        out.push(c);
    }
    out
}

/// `ccThBook` (0x0041a990): where the task is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// Its first frame: named, `Breath(1)`.
    Start,
    /// `EntryFade(12, 0, 0x80000000)` on the menu's fader, until it ends.
    FadeOut,
    /// `StillOff`, state 2, the screen black, `ccBreathThread(2)`: frames
    /// left before `DeleteFade` and the stream.
    Black(u8),
    /// The stream asked for ([`Request::BookStream`]): `ccBreathThread(2)`
    /// (frames left), then its end waited for.
    StreamStart(u8),
    Stream,
    /// The book's frames.
    Run,
    /// `exit`: the use takes it down.
    Done,
}

/// `ccThBook`'s task block: `param` +0x14 the book (the item less 273),
/// +0x18 its state (1, 2 during the stream, 0 once closed).
pub struct Task {
    pub page: i32,
    pub state: i32,
    stage: Stage,
    fade: i32,
    /// The stream has ended (the runtime's answer).
    pub stream_done: bool,
    pub book: Option<Book>,
}

impl Task {
    pub fn new(page: i32) -> Task {
        Task { page, state: 1, stage: Stage::Start, fade: -1, stream_done: false, book: None }
    }

    /// One frame of the task, after the menu's (priority 35).
    pub fn frame(&mut self, m: &mut MenuCtrl, x: &mut Ctx) -> Vec<Draw> {
        let mut draws = Vec::new();
        let env = Env::of(x);
        match self.stage {
            Stage::Start => self.stage = Stage::FadeOut,
            Stage::FadeOut => {
                if self.fade < 0 {
                    self.fade = m.menu_fade.entry_fade(12, 0, 0x8000_0000);
                }
                if !check_fade(m, self.fade) {
                    // StillOff, state 2, ccSys.bgColor black.
                    m.still = 0;
                    x.req.push(Request::Still(false));
                    self.state = 2;
                    x.req.push(Request::WorldHidden(true));
                    self.stage = Stage::Black(1);
                }
            }
            Stage::Black(n) if n > 0 => self.stage = Stage::Black(n - 1),
            Stage::Black(_) => {
                delete_fade(m, self.fade);
                x.req.push(Request::BookStream(Some(self.page)));
                self.stage = Stage::StreamStart(1);
            }
            Stage::StreamStart(n) if n > 0 => self.stage = Stage::StreamStart(n - 1),
            Stage::StreamStart(_) => {
                self.stage = Stage::Stream;
                return self.frame(m, x);
            }
            Stage::Stream => {
                if self.stream_done {
                    m.menu_fade.entry_flash(12, 0x8060_8080);
                    x.req.push(Request::BookStream(None));
                    x.req.push(Request::WorldHidden(false));
                    self.state = 1;
                    m.still = 1;
                    x.req.push(Request::Still(true));
                    x.req.push(Request::KeepLayers);
                    self.book = Some(Book::new(self.page, x, &env));
                    x.se(SE_START);
                    self.stage = Stage::Run;
                }
            }
            Stage::Run => {
                if let Some(b) = self.book.as_mut() {
                    b.frame(m, x, &env, &mut draws);
                    if b.steps.is_empty() && b.exit != 0 {
                        x.se(SE_BACK);
                        self.state = 0;
                        self.stage = Stage::Done;
                    }
                }
            }
            Stage::Done => {}
        }
        draws
    }
}
