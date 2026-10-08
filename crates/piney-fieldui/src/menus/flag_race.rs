//! The Flag Race's menu, from Mutation on (MUT gcmn 0x0058a4f0, menu 88),
//! opened by the breeder's list: the greeting, 100 GP, the Grunty picked
//! (its page [`flag_race_menu_disp`], 0x0058c310), the race started
//! (0x005ff820) and waited on, its pause, then the time, the rank blinking
//! on the Rankings page, the breeder's word, the wallpapers and the prize
//! through `GetItemMenu`. The steps are in docs/engine/flag-race.md.

use piney_data::tables::{fieldui, race, sjis::encode, world};
use piney_desktop::kanji::dec2sjis;

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::breeder::{self, MENU_FLAG_RACE};
use crate::menus::shop::{GOLD, GOLD_MAX};
use crate::menus::system::extract_menu;
use crate::spr::set_clm_large;
use crate::talk;
use crate::window::{disp_square, set_type};

/// The race costs 100 GP.
pub const COST: i32 = 100;
/// `GetItemMenu`, where the prize goes.
const MENU_GET_ITEM: i16 = 29;
/// `saveData +0x8462`: the prizes each town's ranks have given.
const PRIZES: usize = 0x8462;
/// `ccSeOn(74)`: a desktop item's sound.
const SE_DESKTOP: i32 = 74;

/// What the menu reads of the race (`0x0038bd44`) while it lives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaceView {
    /// +0xa3: 1 over, 2 and 3 the result shown.
    pub state: i8,
    /// +0xa2: 1-3 a rank, 4 near the third, 0 none.
    pub rank: i8,
    /// +0xa4: the timer runs (the race can be paused).
    pub running: bool,
    /// +0x90: the time in frames.
    pub time: i16,
}

/// What the menu does after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// Woken from the pause (Continue): `proccess` on.
    Resumed,
    /// Woken to quit: the race stopped, `proccess` on two.
    Quit,
    /// Woken after a wallpaper's window: the count cleared, on.
    Woken,
}

/// The pages' tails: `still` 0 and the layers flipping again, then the
/// page's own.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) {
    m.still = 0;
    x.req.push(Request::Still(false));
    match t {
        Tail::Resumed => m.proccess += 1,
        Tail::Quit => {
            x.req.push(Request::RaceQuit);
            m.proccess += 2;
        }
        Tail::Woken => {
            m.wait_count = 0;
            m.proccess += 1;
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

fn cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// The key the pages read: cancel (sound 19), else OK (18), else 0.
fn key(x: &mut Ctx) -> u32 {
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

/// `ccKanjiStrSeparate(s, k)` of a race text.
fn line(lines: &[&str], k: usize) -> Vec<u8> {
    crate::tables::piece(lines, k)
}

/// `OpenInfo(l0, 0, 0, 0, -1, -1)`.
fn info(m: &mut MenuCtrl, x: &Ctx, l0: &[u8], l1: Option<&[u8]>) {
    let names = x.save.names();
    m.msg.open_info([Some(l0), l1, None, None], &names);
}

/// `cmndTargetPrev->base->name`, the breeder's.
fn breeder_name(m: &MenuCtrl, x: &Ctx) -> Vec<u8> {
    talk::prev_base(m, x).map(|b| b.name).unwrap_or_default()
}

/// The prize count of the town's rank `rank` (1-3; `+0x845e + 3 server +
/// rank`).
fn prize_at(x: &Ctx, rank: i8) -> usize {
    (PRIZES as i32 - 4 + 3 * x.world.game.server + i32::from(rank)) as usize
}

fn prizes(x: &Ctx, rank: i8) -> i8 {
    x.save.save.u8(prize_at(x, rank)) as i8
}

/// `ccMsg->Change(results[i], prev->name, -1, -1)`: a word of the
/// results' table (9 records).
fn result(m: &mut MenuCtrl, x: &mut Ctx, i: i32) {
    let t = race::of(x.texts.volume);
    let rec = x.texts.talk.word(t.results_va().wrapping_add((4 * i) as u32));
    let r = x.texts.talk.ev_msg(rec);
    let name = breeder_name(m, x);
    let names = x.save.names();
    let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
    m.msg.change_record(r.emode, Some(&name), &lines, &names);
    m.talk.chain = Some(talk::Chain { rec, name, grp: -1, msg: -1 });
}

/// The tasks asleep for a window over the frozen screen (`ccSleepAllThread`,
/// `still`, the layers' flips), the dim at 1.
fn sleep(m: &mut MenuCtrl, x: &mut Ctx) {
    talk::sleep_all(m, x);
    m.bg_status = 1;
}

/// The tasks woken, `Disp` and the breath; `t` next frame.
fn wake(m: &mut MenuCtrl, x: &mut Ctx, t: Tail) -> Flow {
    m.bg_status = 3;
    x.req.push(Request::WakeAll);
    crate::disp::disp(m, x);
    talk::breathed(talk::Tail::Breeder(breeder::Tail::Race(t)))
}

/// A wallpaper given: its window (`bookWallPaperAdd` and the number in
/// three digits, then `bookItemAddMsg`), sound 74 and its desktop bit.
pub(crate) fn wallpaper(m: &mut MenuCtrl, x: &mut Ctx, wp: i16) {
    let book = world::of(x.texts.volume).book();
    let mut s = encode(book.get(1).copied().unwrap_or(""));
    s.extend(dec2sjis(i32::from(wp), 3, 0));
    let add = encode(book.first().copied().unwrap_or(""));
    info(m, x, &s, Some(&add));
    x.se(SE_DESKTOP);
    let n = i32::from(wp) - 1;
    let at = piney_data::save::offset::DT_WALLPAPER_LIST + 4 * n.div_euclid(32) as usize;
    let bit = 1u32 << n.rem_euclid(32);
    let v = x.save.save.i32(at) as u32 | bit;
    x.save.save.set_i32(at, v as i32);
}

/// `FlagRace` (MUT gcmn 0x0058a4f0, menu 88).
pub fn flag_race_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let rs = fieldui::of(x.texts.volume).race_str();
    let race = x.world.race;
    match m.proccess {
        // The breeder's greeting, then the target dropped.
        0 => {
            m.lists[i].disp = 4;
            m.menu_status = 0;
            let rec = race::of(x.texts.volume).greet_va();
            let name = talk::target_base(m, x).map(|b| b.name).unwrap_or_default();
            talk::open_record(m, x, rec, &name, -1, -1);
            m.proccess += 1;
        }
        1 => {
            if talk::msg_check(m, x) != 0 {
                x.change_target(None);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        // "It costs 100GP": yes or no.
        2 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 6 {
                let l = &mut m.lists[i];
                l.x = 6;
                l.y = 2;
                l.my = l.y;
                l.disp = 11;
                m.menu_status = 1;
                m.exception_disp = 0;
                m.lists[i].select = 0;
                cursors(m);
                let d = x.texts.dialog_default.clone();
                extract_menu(m, d);
                info(m, x, &line(rs.cost, 0), None);
                m.proccess += 1;
            }
        }
        3 => {
            m.select(0, 0, false, x);
            let k = key(x);
            let sel = m.lists[i].select;
            if (k == x.save.ok() && sel == 1) || k == x.save.cancel() {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                m.wait_count = 0;
                m.proccess += 2;
            } else if k == x.save.ok() && sel == 0 {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                let gold = x.save.save.i32(GOLD);
                if gold >= COST {
                    x.save.save.set_i32(GOLD, (gold - COST).min(GOLD_MAX));
                    m.proccess += 1;
                } else {
                    m.wait_count = 1;
                    m.proccess += 2;
                }
            }
        }
        4 if m.menu_status == 0 => {
            m.lists[i].select = 0;
            cursors(m);
            m.proccess = 10;
        }
        // Not raced: "I await your returning challenge" or "not enough
        // money", then back to the breeder.
        5 if m.menu_status == 0 => {
            let prev = x.target_prev.clone();
            x.change_target(prev.as_ref());
            let lines = if m.wait_count == 0 { rs.retry } else { rs.no_money };
            let (l0, l1, l2) = (line(lines, 0), line(lines, 1), line(lines, 2));
            // cmndTarget's name: the breeder again.
            let name = talk::target_base(m, x).map(|b| b.name).unwrap_or_default();
            talk::open_data(m, x, 0, &name, [&l0, &l1, &l2]);
            m.proccess += 1;
        }
        6 => {
            if breeder_check(m, x) != 0 {
                m.menu_next = m.lists[i].prev;
                m.menu_status = 3;
                m.proccess = 0;
                m.wait_count = 0;
                cursors(m);
            }
        }
        // The town's three Grunties ([`flag_race_menu_disp`]).
        10 => {
            let l = &mut m.lists[i];
            l.x = 24;
            l.y = 3;
            l.my = l.y;
            if l.select < 0 {
                l.select = 0;
            }
            if l.select >= l.y {
                l.select = l.y - 1;
            }
            m.menu_status = 1;
            m.lists[i].disp = 4;
            m.exception_disp = 1;
            x.change_target(None);
            m.proccess += 1;
        }
        11 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                x.se(SE_OK);
                cursors(m);
                m.menu_status = 3;
                m.proccess += 1;
            } else {
                let names = x.save.names();
                let name = breeder_name(m, x);
                m.msg.disp_msg(0x100, Some(&name), [Some(&line(rs.select, 0)), None, None], &names);
            }
        }
        // "Start the race with this Grunty?"
        12 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.page = l.select;
            l.select = 0;
            l.x = 6;
            l.y = 2;
            l.my = l.y;
            l.disp = 11;
            m.menu_status = 1;
            m.exception_disp = 0;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            info(m, x, &line(rs.start, 0), None);
            cursors(m);
            m.proccess += 1;
        }
        13 => {
            m.select(0, 0, false, x);
            let k = key(x);
            let sel = m.lists[i].select;
            if (k == x.save.ok() && sel == 1) || k == x.save.cancel() {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                m.proccess += 1;
            } else if k == x.save.ok() && sel == 0 {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                if let Some(t) = x.target_prev.as_ref() {
                    let h = t.handle;
                    talk::affect(x, h, 0);
                }
                m.proccess = 20;
            }
        }
        14 if m.menu_status == 0 => {
            m.lists[i].select = m.lists[i].page;
            cursors(m);
            m.proccess = 10;
        }
        // The race (0x005ff820): the kind is the Grunty's row less 145.
        20 => {
            let server = x.world.game.server;
            let rows = fieldui::of(x.texts.volume).race_grunties();
            let pick = m.lists[i].page;
            if let Some(g) =
                usize::try_from(server - 1).ok().and_then(|s| rows.get(s)).and_then(|r| r.get(pick as usize))
            {
                x.req.push(Request::RaceStart(i32::from(g.row) - race_row0()));
            }
            m.proccess += 1;
        }
        // The race runs: its result, or its pause (cancel while the timer
        // runs).
        21 => {
            if let Some(r) = race {
                if matches!(r.state, 2 | 3) {
                    m.proccess = 30;
                } else if r.running && x.pushed_cancel() {
                    x.se(SE_BACK);
                    sleep(m, x);
                    m.lists[i].select = 0;
                    m.proccess += 1;
                }
            }
        }
        22 => {
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.x = 8;
            l.y = 2;
            l.my = l.y;
            extract_menu(m, encode(fieldui::of(x.texts.volume).race_pause()));
            info(m, x, &line(rs.paused, 0), None);
            cursors(m);
            m.proccess += 1;
        }
        23 => {
            m.select(0, 0, false, x);
            let k = key(x);
            let sel = m.lists[i].select;
            if (k == x.save.ok() && sel == 0) || k == x.save.cancel() {
                // Continue.
                m.menu_status = 3;
                m.msg.close();
                return wake(m, x, Tail::Resumed);
            } else if k == x.save.ok() && sel == 1 {
                m.lists[i].page = 1;
                m.msg.close();
                m.menu_status = 3;
                m.proccess += 2;
            }
        }
        24 if m.menu_status == 0 => m.proccess = 21,
        // "Do you wish to quit?"
        25 if m.menu_status == 0 => {
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.x = 6;
            l.y = 2;
            l.my = l.y;
            l.select = 1;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            info(m, x, &line(rs.quit, 0), None);
            cursors(m);
            m.proccess += 1;
        }
        26 => {
            m.select(0, 0, false, x);
            let k = key(x);
            let sel = m.lists[i].select;
            if (k == x.save.ok() && sel == 1) || k == x.save.cancel() {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                m.proccess += 1;
            } else if k == x.save.ok() && sel == 0 {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                m.msg.close();
                return wake(m, x, Tail::Quit);
            }
        }
        27 if m.menu_status == 0 => {
            m.lists[i].select = m.lists[i].page;
            m.proccess = 22;
        }
        28 => {
            if race.is_some_and(|r| matches!(r.state, 2 | 3)) {
                m.proccess = 40;
            }
        }
        // The time: "The current record is" mm:ss:hh.
        30 => {
            let t = i32::from(race.map_or(0, |r| r.time));
            let min = t / 1800;
            let rem = t - 1800 * min;
            let sec = rem / 30;
            let hund = (rem - 30 * sec) * 100 / 30;
            let mut s = Vec::new();
            for (v, k) in [(min, 1), (sec, 2), (hund, 3)] {
                s.extend(dec2sjis(v, 2, 2));
                s.extend(line(rs.record, k));
            }
            let name = breeder_name(m, x);
            talk::open_data(m, x, 0, &name, [&line(rs.record, 0), &s, b""]);
            m.proccess += 1;
        }
        31 => {
            if breeder_check(m, x) != 0 {
                if race.is_some_and(|r| (1..4).contains(&r.rank)) {
                    m.menu_status = 1;
                    m.lists[i].disp = 4;
                    m.exception_disp = 2;
                    m.proccess += 1;
                } else {
                    m.proccess = 40;
                }
            }
        }
        // The Rankings page with the new rank blinking.
        32 => {
            if key(x) != 0 {
                m.menu_status = 3;
                m.proccess = 40;
            }
        }
        // The breeder's word on the result (8: a town's first win).
        40 => {
            let rank = race.map_or(0, |r| r.rank);
            let k = if rank == 1 && prizes(x, rank) == 0 { 8 } else { i32::from(rank) };
            result(m, x, k);
            m.proccess += 1;
        }
        41 | 43 | 54 => {
            if talk::msg_check(m, x) != 0 {
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        42 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                let rank = race.map_or(0, |r| r.rank);
                if rank == 1 && prizes(x, rank) == 0 {
                    m.proccess = 50;
                } else if (1..4).contains(&rank) && prizes(x, rank) >= 3 {
                    result(m, x, 5);
                    m.proccess += 1;
                } else {
                    m.proccess += 2;
                }
            }
        }
        // The prize (the rank's, or once its three are given another),
        // through GetItemMenu; the race told the menu is done.
        44 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                let rank = race.map_or(0, |r| r.rank);
                x.req.push(Request::RaceDone);
                m.forbid = 1;
                sleep(m, x);
                let t = race::of(x.texts.volume);
                let s = usize::try_from(x.world.game.server - 1).unwrap_or(0);
                let k = usize::try_from(rank).unwrap_or(0);
                let ranked = (1..4).contains(&rank);
                let p = if ranked && prizes(x, rank) >= 3 {
                    t.prizes_gone()[s][k]
                } else {
                    if ranked {
                        let at = prize_at(x, rank);
                        x.save.save.set_u8(at, x.save.save.u8(at).wrapping_add(1));
                    }
                    t.prizes()[s][k]
                };
                m.item_num = (i32::from(p.cat) << 16) | i32::from(p.id);
                m.menu_next = MENU_GET_ITEM;
                let n = m.list_at(MENU_GET_ITEM);
                m.lists[n].prev = m.menu;
                m.proccess = 0;
                m.wait_count = 0;
                m.menu_status = 3;
                m.first_time = 0;
                cursors(m);
            }
        }
        // A town's first win: its wallpaper.
        50 => {
            sleep(m, x);
            let wp = race::of(x.texts.volume).wallpapers()[x.world.game.server.max(0) as usize];
            wallpaper(m, x, wp);
            m.proccess += 1;
        }
        51 | 56 => {
            if breeder_check(m, x) != 0 {
                return wake(m, x, Tail::Woken);
            }
        }
        52 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.proccess += 1;
            }
        }
        // The other towns' first wins: with three, the Grand Slam.
        53 => {
            m.wait_count = 0;
            for s in 1..5 {
                let at = (PRIZES as i32 - 3 + 3 * s) as usize;
                if x.save.save.u8(at) as i8 > 0 {
                    m.wait_count += 1;
                }
            }
            if m.wait_count >= 3 {
                result(m, x, 7);
                m.wait_count = 0;
                m.proccess += 1;
            } else {
                m.proccess = 60;
            }
        }
        55 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                sleep(m, x);
                let wp = race::of(x.texts.volume).wallpapers()[0];
                wallpaper(m, x, wp);
                m.proccess += 1;
            }
        }
        57 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.proccess = 60;
            }
        }
        60 => {
            result(m, x, 6);
            m.wait_count = 0;
            m.proccess = 43;
        }
        _ => {}
    }
    Flow::Done
}

/// `ccMsg->Check(0)`.
fn breeder_check(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    crate::menus::system::push_msg_requests(x, req);
    r
}

fn race_row0() -> i32 {
    145
}

/// The Flag Race's page (MUT gcmn 0x0058c310): with `exceptionDisp` 1
/// the window of the town's three Grunties, each with its three stars
/// under Speed, Acceleration and Turning, and the cursor; with 2 the
/// Rankings page with the new rank blinking.
pub fn flag_race_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    match m.exception_disp {
        2 => breeder::rankings_menu_disp(m, x),
        1 => grunty_page(m, x),
        _ => {}
    }
}

fn grunty_page(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let (lx, ly, sel) = (m.lists[i].x, m.lists[i].y, m.lists[i].select);
    let a = m.alpha;
    m.win.set_colour(7);
    // 256 - (x + 2) 7: the window centred.
    let left = 256 - i32::from(lx + 2) * 7;
    set_type(&mut m.win, 1);
    (m.win.dx, m.win.dy) = (left as f32, 144.0);
    disp_square(&mut m.win, i32::from(lx), i32::from(ly) + 1, None);
    let names = &mut m.setting[0];
    names.set_colour(7);
    names.set_alpha(a);
    crate::spr::set_clm(names, 16, 0, 0, 1);
    for (k, (dx, dy)) in [(28, 180), (28, 200), (28, 220), (150, 160), (204, 160), (300, 160)].into_iter().enumerate() {
        (names.dx, names.dy) = ((left + dx) as f32, dy as f32);
        names.make_packet(k as i32);
    }
    let stars = &mut m.setting[1];
    stars.set_colour(7);
    stars.set_alpha(a);
    set_clm_large(stars, 2, 0, 0, 8);
    let rows = fieldui::of(x.texts.volume).race_grunties();
    let server = x.world.game.server;
    let mut text = Vec::new();
    if let Some(rows) = usize::try_from(server - 1).ok().and_then(|s| rows.get(s)) {
        for (k, g) in rows.iter().enumerate() {
            let name = usize::try_from(g.row).ok().and_then(|n| x.texts.npc_names.get(n)).cloned().unwrap_or_default();
            crate::menus::system::str_cat(&mut text, &name, 16);
            // Each column's x by its stars, the second's and third's test
            // for none reading the first's (the game's).
            let at = |v: i16, first: i16| {
                if v == 3 {
                    21
                } else if first == 0 {
                    24
                } else {
                    28
                }
            };
            let y = (20 * k + 178) as f32;
            for (v, first, dx) in [(g.speed, g.speed, 140), (g.accel, g.speed, 216), (g.turn, g.speed, 292)] {
                (stars.dx, stars.dy) = ((left + at(v, first) + dx) as f32, y);
                stars.make_packet(i32::from(v));
            }
        }
    }
    let rs = fieldui::of(x.texts.volume).race_str();
    text.extend(encode(rs.specs.first().copied().unwrap_or("")));
    m.setting_text[0] = text;
    m.setting_text[1] = encode(rs.stars.first().copied().unwrap_or(""));
    m.setting_kt[1] = 3;
    let cy = (20 * i32::from(sel) + 180) as f32;
    m.cursor.disp(&mut m.win, (left + 14) as f32, cy, i32::from(lx) - 2, i32::from(sel), a, 3, x.frame_rate);
}

/// Whether a menu number is the race's (88).
pub fn is_race_menu(menu: i16) -> bool {
    menu == MENU_FLAG_RACE
}
