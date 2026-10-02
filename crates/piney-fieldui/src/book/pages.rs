//! Each book's page: `DispNN`, `PadControlNN`, `CheckItemGetNN` and the
//! reward windows `GetBookNNItem`, dispatched by `type` as `BOOK::Draw`,
//! `PadControl` and `CheckItemGet` do.

use piney_data::save::SaveData;
use piney_data::tables::book::{Book as Tables, BookMsg, Bookitem};
use piney_data::tables::sjis::encode;

use super::{Book, Env, PAD_DOWN, PAD_UP, SE_MOVE, SE_REWARD, Step, itoa_ns, save, width};
use crate::ctrl::{Ctx, Draw, MenuCtrl};

/// A `BOOKITEM` table (`BookNNItem`): a counter's thresholds.
// Books 4-8's pages, which read T40 on, are not ported yet (worklog 317).
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    T10,
    T11,
    T20,
    T21,
    T22,
    T30,
    T31,
    T32,
    T33,
    T40,
    T41,
    T50,
    T60,
    T61,
    T62,
    T70,
    T71,
    T72,
    T80,
    T81,
    T82,
}

impl Table {
    /// Its rows, before the `cnt` -1 one.
    pub fn rows(self, t: &'static Tables) -> &'static [Bookitem] {
        match self {
            Table::T10 => t.items_10,
            Table::T11 => t.items_11,
            Table::T20 => t.items_20,
            Table::T21 => t.items_21,
            Table::T22 => t.items_22,
            Table::T30 => t.items_30,
            Table::T31 => t.items_31,
            Table::T32 => t.items_32,
            Table::T33 => t.items_33,
            Table::T40 => t.items_40,
            Table::T41 => t.items_41,
            Table::T50 => t.items_50,
            Table::T60 => t.items_60,
            Table::T61 => t.items_61,
            Table::T62 => t.items_62,
            Table::T70 => t.items_70,
            Table::T71 => t.items_71,
            Table::T72 => t.items_72,
            Table::T80 => t.items_80,
            Table::T81 => t.items_81,
            Table::T82 => t.items_82,
        }
    }
}

/// A `ccMsgData`'s string `k` (1-3 the window's lines), as bytes.
fn line(m: &BookMsg, k: usize) -> Vec<u8> {
    m.str[k].map(encode).unwrap_or_default()
}

/// `BOOK::DispNN` by `type`.
pub fn disp(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) {
    match b.ty {
        0 => disp01(b, m, x, env, draws),
        1 => disp02(b, m, x, env, draws),
        2 => disp03(b, m, x, env, draws),
        _ => {}
    }
}

/// `BOOK::PadControlNN` by `type`.
pub fn pad_control(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx) {
    match b.ty {
        0 => pad_rows(b, x, 1),
        1 | 5 | 6 => pad_rows(b, x, 2),
        2 => pad_control03(b, m, x),
        _ => {}
    }
}

/// `BOOK::CheckItemGetNN` by `type`: the page's counters against their
/// tables once its opening wait has run down (book I's every frame).
pub fn check_item_get(b: &mut Book, env: &Env) {
    let v = b.check_value;
    let get = |b: &mut Book, t: Table, value: i32, sel: i32| b.get_book_item(env, t, value, sel);
    match b.ty {
        0 if b.wait == 0 || b.wait == 1 => {
            get(b, Table::T10, v[0], 0);
            get(b, Table::T11, v[1], 1);
        }
        1 if b.wait == 1 => {
            get(b, Table::T20, v[0], 0);
            get(b, Table::T21, v[1], 1);
            get(b, Table::T22, v[2], 2);
        }
        // The first call's row is the 1 left in the register of the
        // wait's test; the trades' table then takes row 0.
        2 if b.is_sub_win_open == 0 && b.wait == 1 => {
            get(b, Table::T30, v[0], 1);
            get(b, Table::T32, v[2], 2);
            get(b, Table::T31, v[1], 0);
            get(b, Table::T33, v[3], 3);
        }
        _ => {}
    }
}

/// `PadControl01` (0x00410eb0), and the other pages of fixed rows: while
/// `wait` counts down nothing; with a reward window up nothing; down and up
/// (pushed) move the row between 0 and `last`.
fn pad_rows(b: &mut Book, x: &mut Ctx, last: i32) {
    if b.wait != 0 {
        b.wait -= 1;
        return;
    }
    if b.msg_disp_flag != 0 {
        return;
    }
    let push = x.pad.push.bits();
    if push & PAD_DOWN != 0 {
        if b.sel != last {
            b.key_wait = 2;
            x.se(SE_MOVE);
            b.sel += 1;
        }
    } else if push & PAD_UP != 0 && b.sel != 0 {
        b.key_wait = 2;
        x.se(SE_MOVE);
        b.sel -= 1;
    }
}

/// The page's title, centred over its window (`strlen / 2` glyphs 20
/// wide), 10 below the window's top, in colour 7.
fn title(b: &mut Book, m: &MenuCtrl, text: &str, draws: &mut Vec<Draw>) {
    let (y, w, _) = Book::ofs(m, b.ty);
    let raw = encode(text);
    let width = (w + 2) * 14;
    let glyphs = (raw.len() / 2) as i32;
    let x = (512 - width) / 2 + (((width - glyphs * 20) as u32) >> 1) as i32;
    b.title.dx = piney_desktop::eef::from_int(x);
    b.title.dy = piney_desktop::eef::from_int(y + 10);
    b.title.set_colour(super::COL_TEXT);
    Book::text_of(&b.title, draws, raw, 0);
}

/// The time `playTime` (frames) makes: hours, minutes, seconds, with a
/// carry where seconds or minutes come to 60.
fn hms(frames: i32) -> (i32, i32, i32) {
    let s = frames / 60;
    let mut h = s / 3600;
    let r = s % 3600;
    let mut mi = r / 60;
    let mut sec = r % 60;
    if sec == 60 {
        mi += 1;
        sec = 0;
    }
    if mi == 60 {
        h += 1;
        mi = 0;
    }
    (h, mi, sec)
}

/// A help line `DispMsg` shows with one of its strings replaced (gcmn's
/// `ccMsgData` patched in place), unless a reward window is up.
fn help(m: &mut MenuCtrl, x: &Ctx, b: &Book, msg: &BookMsg, k: usize, text: Vec<u8>) {
    if b.msg_disp_flag != 0 {
        return;
    }
    let mut lines: [Option<Vec<u8>>; 3] = [1, 2, 3].map(|i| msg.str[i].map(encode));
    lines[k - 1] = Some(text);
    let names = x.save.names();
    let name = msg.str[0].map(encode);
    m.msg.disp_msg(msg.emode, name.as_deref(), [lines[0].as_deref(), lines[1].as_deref(), lines[2].as_deref()], &names);
}

/// `countStopMsg[volumeNum - 1]` in `BookCounterStopHelpMsg`'s third
/// string, while `wait` is 0.
fn counter_stop(m: &mut MenuCtrl, x: &Ctx, b: &Book, env: &Env) {
    if b.wait != 0 {
        return;
    }
    let text = usize::try_from(env.volume_num - 1)
        .ok()
        .and_then(|k| env.t.count_stop.get(k).copied().flatten())
        .map(encode)
        .unwrap_or_default();
    help(m, x, b, &env.t.counter_stop_help, 3, text);
}

/// `Disp01` (0x00410410): book I, the areas visited and the play time.
fn disp01(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) {
    let t = env.t;
    title(b, m, t.title1, draws);
    let (y, w, _) = Book::ofs(m, b.ty);
    let x0 = Book::left(w) + 28;
    let y0 = y + 50;
    let save = &x.save.save;
    let areas = i32::from(save.i16(save::AREA_COUNT));
    b.check_value[0] = b.check_book_limit(env, 0, areas);
    b.at(1, x0, y0);
    if b.sel == 0 {
        b.cur_x = x0;
        b.cur_y = y0;
    }
    b.over_colour(1);
    b.text(draws, 1, encode(t.area_msg), 0);
    let area_w = width(x, &encode(t.area_msg));
    b.at(2, x0 + area_w + 14, y0);
    b.over_colour(2);
    b.text(draws, 2, format!("{areas:5}").into_bytes(), 1);
    if b.help_disp != 0 && b.sel == 0 {
        if b.count_over == 0 {
            let mut s256 = format!("{areas}").into_bytes();
            s256.extend_from_slice(&encode(t.cnt0));
            let mut s768 = encode(t.cnt1);
            s768.extend_from_slice(&s256);
            s768.extend_from_slice(&encode(t.cnt2));
            help(m, x, b, &t.help10, 3, s768);
        } else {
            counter_stop(m, x, b, env);
        }
    }
    if b.sel == 1 {
        b.cur_x = x0;
        b.cur_y = y0 + 20;
    }
    let (h, mi, s) = hms(x.save.save.i32(save::PLAY_TIME));
    let value = s + h * 10000 + mi * 100;
    b.check_value[1] = b.check_book_limit(env, 1, value);
    b.at(3, x0, y0 + 20);
    b.over_colour(3);
    b.text(draws, 3, encode(t.time_msg), 0);
    let long = format!("{h:3}:{mi:02}:{s:02}").into_bytes();
    let short = format!("{h}:{mi:02}:{s:02}").into_bytes();
    b.at(4, x0 + area_w - 18, y0 + 20);
    b.over_colour(4);
    b.text(draws, 4, long, 1);
    if b.help_disp != 0 && b.sel == 1 {
        if b.count_over == 0 {
            let mut s768 = encode(t.cnt3);
            s768.extend_from_slice(&short);
            s768.extend_from_slice(b".");
            help(m, x, b, &t.help11, 2, s768);
        } else {
            counter_stop(m, x, b, env);
        }
    }
}

/// A counter row of books II, VI and VII: the label, its count in fixed
/// digits (`%5d`), its help line with the count (`bookCnt1` count
/// `bookCnt0` `bookCnt2`) or the counter-stop line at a cap.
struct Row<'a> {
    label: &'a str,
    value: i32,
    /// `msg[k]`'s colour is set for the count; Disp02's first row sets
    /// the label's twice and the count's only at a cap.
    number_colour: bool,
    help: &'a BookMsg,
}

/// Row `k` of a page of counter rows at (x0, y0 + 20 k): `msg[1 + 2k]`
/// the label, `msg[2 + 2k]` the count `width` past x0 + 28.
#[allow(clippy::too_many_arguments)]
fn counter_row(
    b: &mut Book,
    m: &mut MenuCtrl,
    x: &mut Ctx,
    env: &Env,
    draws: &mut Vec<Draw>,
    k: i32,
    (x0, y0): (i32, i32),
    width: i32,
    row: Row,
) {
    let t = env.t;
    let y = y0 + 20 * k;
    if b.sel == k {
        b.cur_x = x0;
        b.cur_y = y;
    }
    b.check_value[k as usize] = b.check_book_limit(env, k, row.value);
    let (label, num) = (1 + 2 * k as usize, 2 + 2 * k as usize);
    b.at(label, x0, y);
    b.over_colour(label);
    b.text(draws, label, encode(row.label), 0);
    b.at(num, x0 + width + 28, y);
    if row.number_colour {
        b.over_colour(num);
    } else {
        b.over_colour(label);
        if b.count_over != 0 {
            b.msg[num].set_colour(super::COL_OVER);
        }
    }
    b.text(draws, num, format!("{:5}", row.value).into_bytes(), 1);
    if b.help_disp != 0 && b.sel == k {
        if b.count_over == 0 {
            let mut s208 = format!("{}", row.value).into_bytes();
            s208.extend_from_slice(&encode(t.cnt0));
            let mut s336 = encode(t.cnt1);
            s336.extend_from_slice(&s208);
            s336.extend_from_slice(&encode(t.cnt2));
            help(m, x, b, row.help, 3, s336);
        } else {
            counter_stop(m, x, b, env);
        }
    }
}

/// `Disp02` (0x00413590): book II, the magic portals opened and the
/// fields and dungeons whose portals are all opened.
fn disp02(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) {
    let t = env.t;
    title(b, m, t.title2, draws);
    let (y, w, _) = Book::ofs(m, b.ty);
    let at = (Book::left(w) + 28, y + 50);
    let width = super::width(x, &encode(t.magic_circle_all_open_msg2));
    let save = &x.save.save;
    let counts = [0x6864, 0x6866, 0x6868].map(|a| i32::from(save.i16(a)));
    let rows = [
        (t.magic_circle_msg, &t.help20, false),
        (t.magic_circle_all_open_msg, &t.help21, true),
        (t.magic_circle_all_open_msg2, &t.help22, true),
    ];
    for (k, (label, help, number_colour)) in rows.into_iter().enumerate() {
        let row = Row { label, value: counts[k], number_colour, help };
        counter_row(b, m, x, env, draws, k as i32, at, width, row);
    }
}

/// `Disp03` (0x00414250): book III, the characters met and their trades;
/// with its sub-window open, the character's trade items.
fn disp03(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>) {
    let t = env.t;
    title(b, m, t.title3, draws);
    let (y, w, _) = Book::ofs(m, b.ty);
    let left = Book::left(w);
    let x0 = left + 28;
    let y0 = y + 50;
    if b.is_sub_win_open != 0 {
        disp03_sub(b, m, x, env, draws, (x0, y0));
        return;
    }
    let names = x.save.names();
    let pages = b.scroll.page_index_num.max(0) as usize;
    let trades: i32 = b.char_data.iter().take(super::CHARS).map(|c| c.trade_num).sum();
    let mut yy = y0 + 60;
    for s in 0..pages {
        let idx = s as i32 + b.scroll.page_top;
        let c = b.char_data.get(idx as usize).cloned().unwrap_or_default();
        let hidden = c.is_friend == -1;
        b.at(s, x0, yy);
        b.msg[s].set_colour(super::COL_TEXT);
        if hidden {
            b.msg[s].set_colour(16);
        }
        if b.sel == 2 && b.scroll.all_index == idx {
            b.cur_x = x0;
            b.cur_y = yy;
        }
        let mut name = b"     ".to_vec();
        name.extend_from_slice(&c.name);
        b.text(draws, s, name, 0);
        let k = s + pages;
        b.at(k, x0, yy);
        b.msg[k].set_colour(super::COL_TEXT);
        if hidden {
            b.msg[k].set_colour(16);
        }
        let row = format!("{:02}:                       {:2}", idx + 1, c.trade_num).into_bytes();
        b.text(draws, k, row, 1);
        yy += 20;
    }
    let _ = names;
    b.now = b.scroll.all_index;
    let friends = b.char_data.iter().take(super::CHARS).filter(|c| c.is_friend != -1).count() as i32;
    b.check_value[0] = b.check_book_limit(env, 0, friends);
    let yy = y + 50;
    if b.sel == 0 {
        b.cur_x = x0;
        b.cur_y = yy;
    }
    let (l0, n0, l1, n1) = (pages * 2, pages * 2 + 1, pages * 2 + 2, pages * 2 + 3);
    b.at(l0, x0, yy);
    b.over_colour(l0);
    b.text(draws, l0, encode(t.char_list), 0);
    let total_w = super::width(x, &encode(t.trade_total));
    b.at(n0, x0 + total_w, yy);
    b.over_colour(n0);
    let mut s = itoa(friends);
    s.extend_from_slice(b"/67");
    b.text(draws, n0, s, 1);
    if b.help_disp != 0 && b.sel == 0 {
        if b.count_over == 0 {
            let mut line = encode(t.char_msg0);
            line.extend_from_slice(format!("{friends}").as_bytes());
            line.extend_from_slice(&encode(t.char_msg1));
            if b.msg_disp_flag == 0 {
                help30(m, x, b, env, [Some(line), None, None]);
            }
        } else {
            counter_stop(m, x, b, env);
        }
    }
    let quarantine = env.volume_num == 4;
    let all_met = || b.char_data.iter().take(super::CHARS).all(|c| c.is_friend != -1);
    if b.count_over == 0 && quarantine && all_met() {
        b.check_value[2] = 999_999_999;
    }
    if b.sel == 1 {
        b.cur_x = x0;
        b.cur_y = yy + 20;
    }
    b.check_value[1] = b.check_book_limit(env, 1, trades);
    if b.count_over == 0 && quarantine {
        let data = &b.char_data[..super::CHARS.min(b.char_data.len())];
        if data.iter().all(|c| c.is_friend != -1) && data.iter().all(|c| c.trade_num > 0) {
            b.check_value[3] = 999_999_999;
        }
    }
    b.at(l1, x0, yy + 20);
    b.over_colour(l1);
    b.text(draws, l1, encode(t.trade_total), 0);
    b.at(n1, x0 + total_w + 24, yy + 20);
    b.over_colour(n1);
    b.text(draws, n1, format!("{trades:4}").into_bytes(), 1);
    if b.help_disp != 0 && b.sel == 1 {
        if b.count_over == 0 {
            let mut line = encode(t.trade_msg0);
            line.extend_from_slice(format!("{trades}").as_bytes());
            line.extend_from_slice(&encode(t.trade_msg1));
            if b.msg_disp_flag == 0 {
                help30(m, x, b, env, [Some(line), None, None]);
            }
        } else {
            counter_stop(m, x, b, env);
        }
    }
    let now = b.char_data.get(b.now as usize).cloned().unwrap_or_default();
    if b.help_disp != 0 && b.sel == 2 {
        let mut line = encode(t.trade_msg2);
        line.extend_from_slice(&itoa_ns(now.trade_num));
        line.extend_from_slice(&encode(t.trade_msg3));
        line.extend_from_slice(&now.name);
        line.extend_from_slice(b".");
        if b.msg_disp_flag == 0 {
            help30(m, x, b, env, [Some(line), Some(encode(t.char_msg2)), None]);
            b.button.dx = 61.0;
            b.button.dy = 396.0;
            crate::window::disp_button(&mut b.button, 3, x.count, x.frame_rate);
        }
    }
    b.win.dx = piney_desktop::eef::from_int(left + 34);
    b.win.dy = piney_desktop::eef::from_int(y + 90);
    crate::window::disp_line_h(&mut b.win, 13);
}

/// Disp03 with its sub-window open: the character, "Online" while it is,
/// and its sixteen trade items in two columns.
fn disp03_sub(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx, env: &Env, draws: &mut Vec<Draw>, (x0, y0): (i32, i32)) {
    let t = env.t;
    let c = b.char_data.get(b.now as usize).cloned().unwrap_or_default();
    b.at(1, x0, y0);
    b.msg[1].set_colour(super::COL_TEXT);
    b.text(draws, 1, c.name.clone(), 0);
    if c.is_online > 0 {
        b.at(2, x0 + 196, y0);
        b.msg[2].set_colour(2);
        b.text(draws, 2, encode(t.on_line), 0);
    }
    let (_, w, _) = Book::ofs(m, b.ty);
    let width = (w + 2) * 14;
    let head = encode(t.trade_item);
    let glyphs = (head.len() / 2) as i32;
    let hx = (512 - width) / 2 + (((width - glyphs * 14) as u32) >> 1) as i32;
    b.at(3, hx, y0 + 20);
    b.msg[3].set_colour(17);
    b.text(draws, 3, head, 0);
    let mut y = y0 + 40;
    let x2 = Book::left(w) + 28;
    for k in 0..8usize {
        for (row, item, dx) in [(4 + k, c.item[k], 0), (12 + k, c.item[k + 8], 140)] {
            b.at(row, x2 + dx, y);
            b.msg[row].set_colour(super::COL_TEXT);
            if item.0 != -1 {
                let name = x.texts.items.item_name(i32::from(item.1), i32::from(item.0));
                b.text(draws, row, name, 0);
            }
        }
        y += 20;
    }
    help30(m, x, b, env, [Some(encode(t.back_msg)), None, None]);
    b.button.dx = 61.0;
    b.button.dy = 372.0;
    crate::window::disp_button(&mut b.button, 0, x.count, x.frame_rate);
}

/// `BookHelp30` with its three lines written (`DispMsg`, whatever
/// `msgDispFlag` says: the callers test it where the game does).
fn help30(m: &mut MenuCtrl, x: &Ctx, b: &Book, env: &Env, lines: [Option<Vec<u8>>; 3]) {
    let _ = b;
    let msg = &env.t.help30;
    let names = x.save.names();
    let name = msg.str[0].map(encode);
    m.msg.disp_msg(msg.emode, name.as_deref(), [lines[0].as_deref(), lines[1].as_deref(), lines[2].as_deref()], &names);
}

/// `itoa(buf, n)` (gcmn 0x0040c270): `n` in four places, the leading zeros
/// spaces.
fn itoa(n: i32) -> Vec<u8> {
    format!("{n:4}").into_bytes()
}

/// A list page's move to row `target` (`PadControl03`'s and the others'
/// loops): the cursor moves within the page, then the page scrolls. A
/// row it cannot reach would loop for ever in the game; here it stops.
fn scroll_to(b: &mut Book, target: i32) {
    let s = &mut b.scroll;
    while s.all_index != target {
        let before = (s.page_index, s.all_index, s.page_top);
        if target < s.all_index {
            if s.page_index != 0 {
                s.page_index -= 1;
                s.all_index -= 1;
            } else if s.all_index != 0 {
                s.all_index -= 1;
                s.page_top -= 1;
            }
        } else if s.page_index != s.page_index_num - 1 {
            s.page_index += 1;
            s.all_index += 1;
        } else if s.all_index != s.all_index_num - 1 {
            s.all_index += 1;
            s.page_top += 1;
        }
        if before == (s.page_index, s.all_index, s.page_top) {
            break;
        }
    }
}

/// The next row at or after `from` (below `end`, the search's own bound)
/// whose `known` holds; -1 for none (`from` at or past `limit` too).
fn next_row(from: i32, limit: i32, end: i32, known: impl Fn(i32) -> bool) -> i32 {
    if from >= limit {
        return -1;
    }
    (from..end).find(|&i| known(i)).unwrap_or(-1)
}

/// The previous row at or before `from` whose `known` holds; -1 for none.
fn prev_row(from: i32, known: impl Fn(i32) -> bool) -> i32 {
    if from < 0 {
        return -1;
    }
    (0..=from).rev().find(|&i| known(i)).unwrap_or(-1)
}

/// `PadControl03` (0x004111a0): book III's rows, its list of characters
/// met (the cursor skips the unknown), and their sub-window.
fn pad_control03(b: &mut Book, m: &mut MenuCtrl, x: &mut Ctx) {
    let cancel = x.save.cancel();
    let push = x.pad.push.bits();
    if b.is_sub_win_open != 0 {
        if push & cancel != 0 {
            m.book_ofs[(b.ty * 3 + 1) as usize] = 19;
            x.se(super::SE_BACK);
            b.cancel_flag = 1;
            b.is_sub_win_open = 0;
            b.key_wait = 0;
        }
        return;
    }
    if b.wait != 0 {
        b.wait -= 1;
        return;
    }
    if b.msg_disp_flag != 0 {
        return;
    }
    let chars = super::CHARS as i32;
    let data = b.char_data.clone();
    let known = |i: i32| data.get(i as usize).is_some_and(|c| c.is_friend > 0);
    if b.sel != b.sel_max {
        if push & PAD_DOWN != 0 {
            b.key_wait = 2;
            x.se(SE_MOVE);
            b.sel += 1;
            if b.sel == 2 {
                let target = next_row(b.scroll.all_index, chars, chars, known);
                if target != -1 && b.scroll.all_index != target {
                    scroll_to(b, target);
                }
            }
        } else if push & PAD_UP != 0 && b.sel != 0 {
            b.key_wait = 2;
            x.se(SE_MOVE);
            b.sel -= 1;
        }
        return;
    }
    if push & x.save.ok() != 0 {
        m.book_ofs[(b.ty * 3 + 1) as usize] = 19;
        x.se(super::SE_OPEN);
        b.is_sub_win_open = 1;
        b.key_wait = 0;
    }
    let rep = x.pad.repeat.bits();
    if rep & PAD_DOWN != 0 {
        let target = next_row(b.scroll.all_index + 1, chars, chars, known);
        b.key_wait = 2;
        x.se(SE_MOVE);
        if target != -1 && b.scroll.all_index != target {
            scroll_to(b, target);
        }
    } else if rep & PAD_UP != 0 {
        let target = prev_row(b.scroll.all_index - 1, known);
        x.se(SE_MOVE);
        if target == -1 {
            scroll_to(b, 0);
            b.sel -= 1;
        } else {
            if b.scroll.all_index != target {
                scroll_to(b, target);
            }
            b.key_wait = 2;
        }
    }
}

/// The reward flash: SE 94, then twice `EntryFlash` and five frames.
fn flash() -> Vec<Step> {
    vec![Step::Se(SE_REWARD), Step::Flash, Step::Frames(5), Step::Flash, Step::Frames(5)]
}

/// `OpenInfo(head + number + tail, "", 0, 0)`: a reward's first window.
fn count_info(msg: &BookMsg, number: Vec<u8>) -> Step {
    let mut l = line(msg, 1);
    l.extend_from_slice(&number);
    l.extend_from_slice(&line(msg, 2));
    Step::Info([Some(l), Some(Vec::new()), None])
}

/// `OpenInfo(str[1], str[2], 0, 0)`: a book's own reward window.
fn msg_info(msg: &BookMsg) -> Step {
    Step::Info([Some(line(msg, 1)), Some(line(msg, 2)), None])
}

/// `GetBookNNItem(row, value, type, sel, num)`: unless the row's reward
/// was given (`hyProccess`), the flash, the book's windows, `BookAddItem`,
/// and the row marked given.
pub fn row_steps(b: &Book, env: &Env, save: &SaveData, row: Bookitem, sel: i32, num: i32) -> Vec<Step> {
    if b.progress(save, sel) >= num {
        return Vec::new();
    }
    let t = env.t;
    let c = row.cnt;
    let n = itoa_ns(c);
    let all = c == 999_999_999;
    let comp = |m: &BookMsg| Step::Info([Some(line(m, 1)), Some(Vec::new()), None]);
    // A count line with the message's third string as the window's second.
    let count3 = |m: &BookMsg, n: Vec<u8>| {
        let Step::Info([l, _, _]) = count_info(m, n) else { unreachable!() };
        Step::Info([l, Some(line(m, 3)), None])
    };
    let mut out = flash();
    let (first, own): (Option<Step>, Option<&BookMsg>) = match b.ty {
        // GetBook01Item (0x0040ddd0): the areas, or the time as h:mm:ss.
        0 => {
            let f = match sel {
                0 => Some(count_info(&t.item_msg10, n)),
                1 => Some(count_info(
                    &t.item_msg11,
                    format!("{}:{:02}:{:02}", c / 10000, c % 10000 / 100, c % 100).into_bytes(),
                )),
                _ => None,
            };
            (f, Some(&t.item_msg1))
        }
        // GetBook02Item (0x0040e220).
        1 => {
            let f = match sel {
                0 => Some(count_info(&t.item_msg20, n)),
                1 => Some(count3(&t.item_msg21, n)),
                2 => Some(count3(&t.item_msg22, n)),
                _ => None,
            };
            (f, Some(&t.item_msg2))
        }
        // GetBook03Item (0x0040e6f0): every character met or traded with
        // (999999999) as its own line.
        2 => {
            let f = match sel {
                0 if all => Some(comp(&t.item_msg31_comp)),
                0 => Some(count_info(&t.item_msg31, n)),
                1 => Some(count_info(&t.item_msg30, n)),
                2 => Some(comp(&t.item_msg30_comp)),
                3 => Some(comp(&t.item_msg31_comp)),
                _ => None,
            };
            (f, Some(&t.item_msg3))
        }
        // GetBook04Item (0x0040eca0): another row has no window of its own.
        3 => match sel {
            0 if all => (Some(comp(&t.item_msg40_comp)), Some(&t.item_msg4)),
            0 => (Some(count_info(&t.item_msg40, n)), Some(&t.item_msg4)),
            1 => (Some(comp(&t.item_msg40_comp)), Some(&t.item_msg4)),
            _ => (None, None),
        },
        // GetBookItem's own (0x004100d8): the count with `%d`.
        4 => (Some(count_info(&t.item_msg50, format!("{c}").into_bytes())), Some(&t.item_msg5)),
        // GetBook06Item (0x0040f100).
        5 => {
            let f = match sel {
                0 => Some(count_info(&t.item_msg60, n)),
                1 => Some(count3(&t.item_msg61, n)),
                2 => Some(count_info(&t.item_msg62, n)),
                _ => None,
            };
            (f, Some(&t.item_msg6))
        }
        // GetBook07Item (0x0040f5d0).
        6 => {
            let f = match sel {
                0 => Some(count_info(&t.item_msg70, n)),
                1 => Some(count_info(&t.item_msg71, n)),
                2 => Some(count_info(&t.item_msg72, n)),
                _ => None,
            };
            (f, Some(&t.item_msg7))
        }
        // GetBook08Item (0x0040faa0).
        7 => {
            let f = match sel {
                0 => Some(Step::Info([1, 2, 3].map(|k| Some(line(&t.item_msg80, k))))),
                1 | 2 if all => {
                    Some(Step::Info([Some(line(&t.item_msg81_comp, 1)), Some(line(&t.item_msg81_comp, 2)), None]))
                }
                1 | 2 => Some(count_info(&t.item_msg81, n)),
                _ => None,
            };
            (f, Some(&t.item_msg8))
        }
        _ => (None, None),
    };
    out.extend(first);
    out.extend(own.map(msg_info));
    out.extend(super::add_item_steps(save, env));
    out.push(Step::Progress { sel, num });
    out
}
