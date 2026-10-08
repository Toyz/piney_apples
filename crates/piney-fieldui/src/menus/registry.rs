//! The Event NPC's menus, from Mutation on: its list (MUT gcmn 0x0058d470,
//! menu 90: Talk and Item List) and Item List (0x0058d9f0, menu 91). Item
//! List puts what Kite holds, wears and keeps at Elf's Haven in the save's
//! item registry; once every item is in, the NPC gives the desktop's
//! wallpaper 56, BGM 51 and movies 90-96 and goes. Its page is
//! [`item_list_menu_disp`] (0x0058edc0). Steps in docs/engine/field-ui.md.

use piney_data::save::{by_id, offset};
use piney_data::tables::{fieldui, registry, sjis::encode, world};
use piney_desktop::eef::from_int;
use piney_desktop::kanji::dec2sjis;

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::flag_race::wallpaper;
use crate::menus::system::{extract_menu, item_rows, str_cat};
use crate::spr::{font_type, set_clm};
use crate::talk::{self, Then};
use crate::window::{disp_button, disp_scroll_bar, disp_square_tag, set_type};

/// `ccSeOn(74)`: a desktop item's sound.
const SE_DESKTOP: i32 = 74;
/// The page's rows.
const ROWS: i32 = 8;

/// An item category as Item List keeps it (MUT gcmn 0x0058f5d0, 0x0058f6c0
/// and 0x0058f7b0, the same on Outbreak and Quarantine): its ids, and how
/// many of them the list holds. The weapons, armour and treasure leave out
/// their last ids; the items (-2: category 10, and 13 from id 24) leave out
/// id 22; the Grunty food (-3, key items 26-41) and virus cores (-4, key
/// items 0-11) leave out none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shelf {
    ids: i32,
    listed: i32,
}

impl Shelf {
    fn of(cat: i32) -> Option<Shelf> {
        let (ids, listed) = match cat {
            0 => (83, 60),
            1 => (77, 60),
            2 => (97, 80),
            3 => (76, 62),
            4 => (74, 60),
            5 => (77, 60),
            6 => (69, 60),
            7 => (68, 60),
            8 => (67, 60),
            9 => (68, 60),
            11 => (72, 72),
            14 => (22, 10),
            -2 => (27, 26),
            -3 => (16, 16),
            -4 => (12, 12),
            _ => return None,
        };
        Some(Shelf { ids, listed })
    }
}

/// Whether item `id` of `cat` is left out (0x0058f7b0): the ids past the
/// listed ones of the weapons, armour and treasure, and the items' 22.
fn left_out(cat: i32, id: i32) -> bool {
    match (cat, Shelf::of(cat)) {
        (-2, _) => id == 22,
        (0..=9 | 14, Some(s)) => (s.listed..s.ids).contains(&id),
        _ => false,
    }
}

/// The category and id the page names an entry by: the items' 10 and 13,
/// the key items' 15.
fn item_of(cat: i32, id: i32) -> (i32, i32) {
    match cat {
        -2 if id < 24 => (10, id),
        -2 => (13, id - 24),
        -3 => (15, id + 26),
        -4 => (15, id),
        _ => (cat, id),
    }
}

/// What the registering counts: new from the bag (`itemNum`), new from
/// Elf's Haven (`trapNum`), and the list's registered and listed totals
/// (`temp[0]`, `temp[1]`).
struct Counts {
    bag: i32,
    haven: i32,
    registered: i32,
    listed: i32,
}

/// An item list's entries as (category, id): `n` of `{s16 id, s8 cat, s8
/// num}` at `at`.
fn entries(x: &Ctx, at: usize, n: usize) -> Vec<(i32, i32)> {
    let s = &x.save.save;
    (0..n).map(|k| (i32::from(s.u8(at + 4 * k + 2) as i8), i32::from(s.i16(at + 4 * k)))).collect()
}

/// Item `id` of `cat` registered if it is listed and new; counted in `n`.
fn take(x: &mut Ctx, cat: i32, id: i32, n: &mut i32) {
    if !left_out(cat, id) && !x.save.save.registered(cat, id) {
        *n += 1;
        x.save.save.register(cat, id);
    }
}

/// `0x0058f930`: `cat`'s items in the bag and at Elf's Haven registered,
/// then its registered and listed totals counted.
fn count(x: &mut Ctx, cat: i32, c: &mut Counts) {
    let Some(shelf) = Shelf::of(cat).filter(|s| s.ids > 0) else { return };
    let bag = entries(x, by_id::item_list(0), 40);
    let haven = entries(x, offset::PL_ITEM_LIST, 99);
    let imp = |x: &Ctx, k: usize| x.save.save.u8(offset::IMP_ITEM_LIST + k) as i8;
    match cat {
        -2 => {
            for (list, n) in [(&bag, &mut c.bag), (&haven, &mut c.haven)] {
                for &(ic, id) in list {
                    if ic == 10 && id >= 0 {
                        take(x, cat, id, n);
                    }
                    if ic == 13 && id >= 0 {
                        take(x, cat, id + 24, n);
                    }
                }
            }
        }
        // The key items held: Grunty food 26-41, virus cores 0-11.
        -3 | -4 => {
            let first = if cat == -3 { 26 } else { 0 };
            for k in 0..shelf.ids as usize {
                if imp(x, first + k) > 0 && !x.save.save.registered(cat, k as i32) {
                    c.bag += 1;
                    x.save.save.register(cat, k as i32);
                }
            }
        }
        _ => {
            for (list, n) in [(&bag, &mut c.bag), (&haven, &mut c.haven)] {
                for &(ic, id) in list {
                    if ic == cat && id >= 0 {
                        take(x, cat, id, n);
                    }
                }
            }
        }
    }
    for id in (0..shelf.ids).filter(|&id| !left_out(cat, id)) {
        if x.save.save.registered(cat, id) {
            c.registered += 1;
        }
        c.listed += 1;
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

fn affect_target(x: &mut Ctx, n: i16) {
    if let Some(h) = x.target.as_ref().map(|t| t.handle) {
        talk::affect(x, h, n);
    }
}

/// `StillOn` as the menus write it inline, when the tasks are not asleep
/// already.
fn still_on(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.still == 0 {
        talk::sleep_all(m, x);
    }
}

/// `EventNpcMenu` (MUT gcmn 0x0058d470, menu 90): the Event NPC's list,
/// Talk (47) and Item List (91). The first time it greets by server.
pub fn event_npc_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if x.target.is_none() {
                return talk::close(m, x, Then::Nothing);
            }
            let rows = item_rows(m);
            extract_menu(m, rows);
            if m.first_time != 0 {
                if let Some(base) = talk::target_base(m, x)
                    && base.msg != 0
                {
                    affect_target(x, 14);
                    let server = x.world.game.server;
                    let rec = x.texts.talk.word(base.msg.wrapping_add(4 * server as u32));
                    talk::open_record(m, x, rec, &base.name, -1, -1);
                }
                m.lists[i].select = 0;
            }
            m.talk.talk_num = 1;
            m.map_status = 3;
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() {
                m.map_status = 1;
                return talk::close(m, x, Then::MsgClose);
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                affect_target(x, 0);
                m.map_status = 1;
                return talk::close(m, x, Then::MsgClose);
            }
            if key == x.save.ok() {
                if m.lists[i].select != 0 {
                    still_on(m, x);
                    x.change_target(None);
                }
                let flow = m.change_menu(x);
                m.msg.close();
                return flow;
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// What Item List does after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// The tasks woken: the count cleared, on.
    Woken,
    /// Cancel on the groups: back to menu 90.
    Back,
}

/// Item List's tails: `still` 0 and the layers flipping again, then its
/// own.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    m.still = 0;
    x.req.push(Request::Still(false));
    match t {
        Tail::Woken => {
            m.wait_count = 0;
            m.proccess += 1;
        }
        Tail::Back => {
            m.bg_status = 3;
            let i = idx(m);
            m.menu_next = m.lists[i].prev;
            m.menu_status = 3;
            m.proccess = 0;
            m.wait_count = 0;
            cursors(m);
        }
    }
    None
}

/// The tasks woken, `Disp` and the breath; `t` next frame.
fn wake(m: &mut MenuCtrl, x: &mut Ctx, t: Tail) -> Flow {
    x.req.push(Request::WakeAll);
    crate::disp::disp(m, x);
    talk::breathed(talk::Tail::Registry(t))
}

/// Seven frames counted (`waitCount`), then `then`.
fn waited(m: &mut MenuCtrl) -> bool {
    let w = m.wait_count;
    m.wait_count += 1;
    w >= 7
}

/// `ccMsg->Check(0)`: the window read.
fn check(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    crate::menus::system::push_msg_requests(x, req);
    r != 0
}

/// `OpenInfo(l0, l1, l2, l3, -1, -1)`.
fn info(m: &mut MenuCtrl, x: &Ctx, lines: [Option<&[u8]>; 4]) {
    let names = x.save.names();
    m.msg.open_info(lines, &names);
}

/// A desktop item's window kept open (`ChangeInfo`): `add`'s word and the
/// number in three digits, then `bookItemAddMsg`.
fn desktop_item(m: &mut MenuCtrl, x: &mut Ctx, add: usize, n: i32) {
    let book = world::of(x.texts.volume).book();
    let mut s = encode(book.get(add).copied().unwrap_or(""));
    s.extend(dec2sjis(n, 3, 0));
    let msg = encode(book.first().copied().unwrap_or(""));
    let names = x.save.names();
    m.msg.change_info([Some(&s), Some(&msg), None, None], &names);
    x.se(SE_DESKTOP);
}

/// A bit of a desktop list (`dtBgmList`, `dtStrList`) set.
fn set_bit(x: &mut Ctx, at: usize, n: i32) {
    let at = at + 4 * (n / 32) as usize;
    let v = x.save.save.i32(at) as u32 | 1u32 << (n % 32);
    x.save.save.set_i32(at, v as i32);
}

/// Whether this is Mutation's text building (its pieces with the number
/// inside a line); from Outbreak on the number leads the lines.
fn mutation(x: &Ctx) -> bool {
    x.texts.volume == piney_data::volume::Volume::Mut
}

/// "N items were registered" from the bag or Elf's Haven. Mutation's is
/// its first piece, the number and the second, then the third (MUT gcmn
/// 0x0058e040); Outbreak's the number and the first, then the second (OUT
/// gcmn 0x0058a0c0).
fn registered_info(m: &mut MenuCtrl, x: &Ctx, lines: &[&str], n: i32) {
    let num = dec2sjis(n, 2, 0);
    let p = |k| crate::tables::piece(lines, k);
    let (l0, l1) = if mutation(x) { ([p(0), num, p(1)].concat(), p(2)) } else { ([num, p(0)].concat(), p(1)) };
    info(m, x, [Some(&l0), Some(&l1), None, None]);
}

/// The category of the page `l` shows.
fn page_cat(x: &Ctx, index: i16, page: i16) -> i32 {
    let cats = registry::of(x.texts.volume).cats();
    usize::try_from(index)
        .ok()
        .and_then(|g| cats.get(g))
        .and_then(|r| r.get(page.max(0) as usize))
        .copied()
        .unwrap_or(-1)
}

fn listed(cat: i32) -> i16 {
    Shelf::of(cat).map_or(0, |s| s.listed as i16)
}

/// `ItemListMenu` (MUT gcmn 0x0058d9f0, menu 91).
pub fn item_list_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let s = fieldui::of(x.texts.volume).item_list_str();
    match m.proccess {
        // Everything held, worn and kept at Elf's Haven registered.
        0 => {
            m.lists[i].disp = 4;
            m.menu_status = 3;
            m.bg_status = 1;
            m.lists[i].select = 0;
            m.lists[i].index = 0;
            let mut c = Counts { bag: 0, haven: 0, registered: 0, listed: 0 };
            for cat in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 14, -2, -3, -4] {
                count(x, cat, &mut c);
            }
            // Kite's weapon and armour (`spcParam[0]` +0xd0, +0xc8).
            let p = by_id::spc_param(0);
            let worn = [(0, 0xd0), (6, 0xc8), (7, 0xca), (8, 0xcc), (9, 0xce)];
            for (cat, at) in worn {
                let id = i32::from(x.save.save.i16(p + at));
                if !left_out(cat, id) && !x.save.save.registered(cat, id) {
                    c.bag += 1;
                    c.registered += 1;
                    x.save.save.register(cat, id);
                }
            }
            (m.item_num, m.trap_num) = (c.bag, c.haven);
            (m.talk.temp[0], m.talk.temp[1]) = (c.registered, c.listed);
            if c.bag != 0 || c.haven != 0 {
                m.proccess += 3;
            } else if c.registered < c.listed {
                // One line on Mutation, two from Outbreak on.
                let (l0, l1) = (crate::tables::piece(s.none, 0), crate::tables::piece(s.none, 1));
                let l1 = (!mutation(x)).then_some(&l1[..]);
                info(m, x, [Some(&l0), l1, None, None]);
                m.proccess += 1;
            } else {
                m.proccess = 40;
            }
            Flow::Done
        }
        1 | 4 | 7 | 14 => {
            if check(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
            Flow::Done
        }
        2 => {
            if waited(m) {
                m.proccess = 40;
            }
            Flow::Done
        }
        3 => {
            if m.item_num != 0 {
                registered_info(m, x, s.bag, m.item_num);
                m.proccess += 1;
            } else {
                m.proccess += 3;
            }
            Flow::Done
        }
        5 => {
            if waited(m) {
                m.proccess += 1;
            }
            Flow::Done
        }
        6 => {
            if m.trap_num != 0 {
                registered_info(m, x, s.haven, m.trap_num);
                m.proccess += 1;
            } else {
                m.proccess = 10;
            }
            Flow::Done
        }
        8 => {
            if waited(m) {
                m.proccess = 10;
            }
            Flow::Done
        }
        // The list complete: the NPC's word.
        10 => {
            if m.talk.temp[0] < m.talk.temp[1] {
                m.proccess = 40;
                return Flow::Done;
            }
            let prev = x.target_prev.clone();
            x.change_target(prev.as_ref());
            m.bg_status = 3;
            wake(m, x, Tail::Woken)
        }
        11 => {
            if waited(m) {
                let rec = registry::of(x.texts.volume).done_va();
                let name = talk::target_base(m, x).map(|b| b.name).unwrap_or_default();
                talk::open_record(m, x, rec, &name, -1, -1);
                m.proccess += 1;
            }
            Flow::Done
        }
        12 => {
            if talk::msg_check(m, x) != 0 {
                m.msg.close();
                x.change_target(None);
                m.bg_status = 1;
                talk::sleep_all(m, x);
                m.wait_count = 0;
                m.proccess += 1;
            }
            Flow::Done
        }
        // Wallpaper 56, BGM 51, movies 90-96.
        13 => {
            if waited(m) {
                wallpaper(m, x, 56);
                m.proccess += 1;
            }
            Flow::Done
        }
        15 => {
            if waited(m) {
                desktop_item(m, x, 2, 51);
                set_bit(x, offset::DT_BGM_LIST, 51);
                m.proccess += 1;
            }
            Flow::Done
        }
        16 => {
            if check(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.talk.temp[2] = 0;
                m.proccess += 1;
            }
            Flow::Done
        }
        17 => {
            if waited(m) {
                let k = m.talk.temp[2];
                desktop_item(m, x, 3, k + 90);
                set_bit(x, offset::DT_STR_LIST, k + 89);
                m.proccess += 1;
            }
            Flow::Done
        }
        18 => {
            if !check(m, x) {
                return Flow::Done;
            }
            m.msg.close();
            if m.talk.temp[2] < 6 {
                m.wait_count = 0;
                m.talk.temp[2] += 1;
                m.proccess -= 1;
                return Flow::Done;
            }
            // ITEM COMPLETE's status cleared: the NPC goes.
            x.save.save.set_u8(offset::EVENT_STATUS + 52, 0);
            m.bg_status = 3;
            wake(m, x, Tail::Woken)
        }
        19 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w < 91 {
                return Flow::Done;
            }
            m.map_status = 1;
            talk::close(m, x, Then::Nothing)
        }
        // The groups: Weapons, Armors, Items, Key Items.
        40 => {
            let l = &mut m.lists[i];
            l.disp = 7;
            l.x = 9;
            l.my = 4;
            l.y = 4;
            l.select = l.index;
            m.menu_status = 1;
            let rows = encode(fieldui::of(x.texts.volume).item_list_rows());
            extract_menu(m, rows);
            cursors(m);
            m.proccess += 1;
            Flow::Done
        }
        41 => {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                cursors(m);
                let prev = x.target_prev.clone();
                x.change_target(prev.as_ref());
                return wake(m, x, Tail::Back);
            }
            if key == x.save.ok() {
                cursors(m);
                m.menu_status = 3;
                m.wait_count = 0;
                m.proccess += 1;
                return Flow::Done;
            }
            // The count: the registered after the first piece; those still
            // to come after the second on Mutation, before it later.
            let (reg, all) = (m.talk.temp[0], m.talk.temp[1]);
            let p = |k| crate::tables::piece(s.count, k);
            let l0 = [p(0), dec2sjis(reg, 3, 0)].concat();
            let left = dec2sjis(all - reg, 3, 0);
            let l1 = if mutation(x) { [p(1), left].concat() } else { [left, p(1)].concat() };
            let l2 = p(2);
            let names = x.save.names();
            m.msg.disp_msg(0x100, None, [Some(&l0), Some(&l1), Some(&l2)], &names);
            Flow::Done
        }
        42 => {
            if waited(m) {
                let l = &mut m.lists[i];
                l.index = l.select;
                l.select = 0;
                m.proccess = 50;
            }
            Flow::Done
        }
        // A group's pages: a category each.
        50 => {
            m.exception_disp = 1;
            m.menu_status = 1;
            let pages = registry::of(x.texts.volume).pages();
            let l = &m.lists[i];
            let n = usize::try_from(l.index).ok().and_then(|g| pages.get(g)).map_or(0, |p| p.n);
            let cat = page_cat(x, l.index, 0);
            let l = &mut m.lists[i];
            l.disp = 4;
            l.page_num = n;
            l.page = 0;
            l.x = 15;
            l.y = ROWS as i16;
            l.my = listed(cat);
            l.select = 0;
            l.dy = 0;
            cursors(m);
            m.proccess += 1;
            Flow::Done
        }
        51 => {
            let page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if m.lists[i].page != page {
                m.kanji_alpha = -24;
            }
            let l = &m.lists[i];
            let row = l.select - l.dy;
            let my = listed(page_cat(x, l.index, l.page));
            let l = &mut m.lists[i];
            l.my = my;
            if l.my - l.y < l.dy {
                l.dy = l.my - l.y;
                l.select = l.dy + row;
            }
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.menu_status = 3;
                m.proccess += 1;
            }
            Flow::Done
        }
        52 => {
            if m.menu_status == 0 {
                m.exception_disp = 0;
                m.proccess = 40;
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// `ItemListMenuDisp` (MUT gcmn 0x0058edc0): a category's page, its
/// items named and lit where registered.
pub fn item_list_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let (a, ka) = (m.alpha, m.kanji_alpha);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(ka);
    m.win.set_colour(7);
    m.kanji.set_colour(7);
    m.item_icon.set_colour(7);
    m.win.set_alpha(a);
    m.kanji.set_alpha(ka);
    m.item_icon.set_alpha(ka);
    set_type(&mut m.win, 1);
    m.item_icon.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    (m.win.dx, m.win.dy) = (39.0, 96.0);
    disp_square_tag(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.page), i32::from(l.page_num), 8);
    if l.y < l.my {
        (m.win.dx, m.win.dy) = (39.0, 96.0);
        disp_scroll_bar(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.dy), i32::from(l.my));
    }
    (m.win.dx, m.win.dy) = (55.0, 64.0);
    disp_button(&mut m.win, 4, x.count, x.frame_rate);
    (m.win.dx, m.win.dy) = (from_int((i32::from(l.x) + 2) * 14 + 23), 64.0);
    disp_button(&mut m.win, 6, x.count, x.frame_rate);
    (m.win.cx, m.win.cy) = (0.0, 0.0);
    if ka > 0 {
        set_clm(&mut m.kanji, 16, 0, 0, 1);
        (m.kanji.dx, m.kanji.dy) = (from_int((i32::from(l.page) + 2) * 14 + 39), 84.0);
        m.kanji.make_packet(0);
        let tags = fieldui::of(x.texts.volume).item_list_tags();
        let tag = usize::try_from(l.index).ok().and_then(|g| tags.get(g)).copied().unwrap_or(&[]);
        let t = crate::tables::piece(tag, l.page.max(0) as usize);
        extract_menu(m, t);
    }
    let w = if l.y < l.my { i32::from(l.x) - 1 } else { i32::from(l.x) };
    let fr = x.frame_rate;
    let y = from_int((i32::from(l.select) - i32::from(l.dy)) * 20 + 112);
    m.cursor.disp(&mut m.win, 39.0, y, w, i32::from(l.select) + (i32::from(l.page) << 8), a, 3, fr);
    if ka <= 0 {
        return;
    }
    set_clm(&mut m.setting[0], 16, 0, 0, 1);
    set_clm(&mut m.setting[1], 16, 0, 0, 1);
    let cat = page_cat(x, l.index, l.page);
    let ids = Shelf::of(cat).map_or(0, |s| s.ids);
    let items = &x.texts.items;
    let mut buf = Vec::new();
    let (mut row, mut kanji, mut seen) = (0i32, 0usize, 0i32);
    for id in (0..ids).filter(|&id| !left_out(cat, id)) {
        if seen < i32::from(l.dy) {
            seen += 1;
            continue;
        }
        let (ic, iid) = item_of(cat, id);
        str_cat(&mut buf, &items.item_name(ic, iid), 16);
        let c = if x.save.save.registered(cat, id) { 7 } else { 0 };
        m.setting[kanji].set_colour(c);
        m.item_icon.set_colour(c);
        m.setting[kanji].set_alpha(ka);
        m.item_icon.set_alpha(ka);
        let y = from_int((row + 8 * kanji as i32) * 20 + 112);
        if ic != 15 {
            (m.item_icon.dx, m.item_icon.dy) = (51.0, y);
            m.item_icon.make_packet(items.item_icon(ic));
        }
        (m.setting[kanji].dx, m.setting[kanji].dy) = (67.0, y);
        m.setting[kanji].make_packet(row);
        row += 1;
        if row >= ROWS {
            m.setting_text[kanji] = std::mem::take(&mut buf);
            row = 0;
            kanji += 1;
        }
        if kanji > 0 {
            break;
        }
        seen += 1;
    }
    if row != 0 {
        m.setting_text[kanji] = buf;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The left-out ids are the game's lists (MUT gcmn 0x006823a0 on):
    /// every category lists `listed` of its `ids`.
    #[test]
    fn each_category_lists_its_count() {
        for cat in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 14, -2, -3, -4] {
            let s = Shelf::of(cat).unwrap();
            assert_eq!((0..s.ids).filter(|&id| !left_out(cat, id)).count() as i32, s.listed, "{cat}");
        }
        assert!(left_out(0, 60) && !left_out(0, 59) && left_out(14, 21) && left_out(-2, 22) && !left_out(-2, 23));
    }
}
