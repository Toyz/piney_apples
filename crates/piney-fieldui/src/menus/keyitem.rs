//! `ImportantItemMenu` (gcmn 0x0052f980, menu 6, Key Items) with
//! `ImportantItemMenuDisp` (0x005305d0), and `ThrowItemMenu` (0x00530d60,
//! menu 7, Discard Item) with `ThrowItemMenuDisp` (0x00531820).
//!
//! Key Items lists `saveData.impItemList[291]` (+0xcfc, a count per key
//! item) by page (`SetImportantItemList` 0x005274f0): 0 Event Item (ids
//! 42-73 and 281-290), 1 Grunty Food (26-41), 2 Virus Core (0-25), 3 Book
//! of 1000 (273-280); two pages before the bracelet, three in Parody Mode.
//!
//! ```text
//! proccess 0   target fixed and dropped; the pages; the dim in
//! proccess 1   SelectScr; cancel (19) back; OK (18) by ccCheckItemUseful:
//!              0 help 0; in town only 12 and 15 (help 1); 2 used at once:
//!              the Grunty Flute (15/49) only in a field (help 2), not on
//!              fields 13 and 67 (help 4), not in battle (help 5): 20; a
//!              Ryu Book (15/273-280) only in town (help 8); else 2;
//!              1 TARGET (65); else the item's name and comment
//! proccess 2   the window gone: ccUseItemRequest on the player; a Ryu
//!              Book fades out, lets the tasks run a frame and fades back
//!              in over 10 (then 3 frames); else 8 frames: the list again
//! proccess 10  the help as an information window, Check, 8 frames
//! proccess 20  the Grunty Flute: from slot rand() % 3 the first Grunty
//!              ccPgAdultCheck finds; none: "There are no Grunties ..."
//! ```
//!
//! Discard Item runs over the player's items as the Items menu shows them
//! (`SetItemList`, `ItemMenuDisp`):
//!
//! ```text
//! proccess 0   as Items (five pages)
//! proccess 1   SelectScr; triangle on equipment: its status (64); cancel
//!              back; OK on an item: the count (exceptionDisp 2)
//! proccess 2   up / down one, left / right ten (1 to the count held);
//!              cancel back; OK: the window out
//! proccess 3   "Discard N #Gitem#W." with OK / Cancel (list disp 11)
//! proccess 4   Select; OK on OK: ccSaveData::DelItem(0, cat, id, N)
//! proccess 5   the list again
//! ```

use piney_desktop::eef::from_int;

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_MOVE, SE_OK};
use crate::items::{self, ITEMS, Item};
use crate::menus::personal::{self, Tail, check, check_fade, delete_fade, key, open_info2, pers};
use crate::menus::system::{extract_menu, str_cat};
use crate::spr::{font_type, make_num, set_clm};
use crate::window::{disp_button, disp_scroll_bar, disp_square_tag, set_type};

/// `saveData.impItemList` (+0xcfc): how many of each key item.
pub const IMP_ITEMS: usize = 291;
/// The Grunty Flute and the Ryu Books.
pub const GRUNTY_FLUTE: i16 = 49;
pub const RYU_BOOKS: std::ops::Range<i16> = 273..281;

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// `SetImportantItemList(page, list, 0)` (0x005274f0): the key items held
/// on a page, `{id, 15, count}` each; the rest of the 291 empty.
pub fn important_list(save: &piney_desktop::SaveState, page: i32) -> Vec<Item> {
    let mut out = vec![Item::NONE; IMP_ITEMS];
    let mut n = 0;
    for id in 0..IMP_ITEMS {
        let count = save.save.u8(items::IMP_ITEM_LIST + id) as i8;
        if count <= 0 {
            continue;
        }
        let take = match page {
            0 => (42..74).contains(&id) || id >= 281,
            1 => (26..42).contains(&id),
            2 => id < 26,
            3 => (273..281).contains(&id),
            _ => false,
        };
        if take {
            out[n] = Item { id: id as i16, cat: 15, num: count };
            n += 1;
        }
    }
    out
}

/// `SetImportantItemList(page, list, 1)`: the list and its count fitted.
fn important_list_fit(m: &mut MenuCtrl, x: &Ctx) -> Vec<Item> {
    let i = idx(m);
    let list = important_list(x.save, i32::from(m.lists[i].page));
    let n = list.iter().filter(|it| it.cat >= 0).count() as i16;
    items::fit_list(&mut m.lists[i], n);
    list
}

/// The item's name and comment as the help line (`DispMsg` emode 0x100),
/// or an empty one.
fn item_help(m: &mut MenuCtrl, x: &Ctx, it: Item) {
    let names = x.save.names();
    let p = x.texts.items.item(i32::from(it.cat), i32::from(it.id)).filter(|_| it.cat >= 0 && it.id >= 0);
    match p {
        Some(p) => {
            let p = p.clone();
            m.msg.disp_msg(
                0x100,
                Some(&p.name),
                [Some(&p.comment[0]), Some(&p.comment[1]), Some(&p.comment[2])],
                &names,
            );
        }
        None => m.msg.disp_msg(0x100, None, [None, None, None], &names),
    }
}

fn refuse(m: &mut MenuCtrl, help: i16) {
    m.wait_count = help;
    m.proccess = 10;
}

/// `ImportantItemMenu` (menu 6).
pub fn important_item_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.exception_disp = 1;
            m.bg_status = 1;
            let l = &mut m.lists[i];
            l.page_num = if x.save.save.u8(items::PLCOL) == 0 {
                2
            } else if x.save.parody() {
                3
            } else {
                4
            };
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            important_list_fit(m, x);
            m.proccess += 1;
        }
        1 => {
            let old_page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old_page != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = important_list_fit(m, x);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            match key(x) {
                Some(false) => {
                    let prev = m.lists[i].prev;
                    m.back_to_prev(prev);
                    return Flow::Done;
                }
                Some(true) if it.cat >= 0 => {
                    let (cat, id) = (i32::from(it.cat), i32::from(it.id));
                    let useful = items::check_item_useful(cat, id);
                    let g = x.world.game;
                    if useful == 0 {
                        m.menu_status = 3;
                        refuse(m, 0);
                        return Flow::Done;
                    }
                    if g.area == 0 && cat != 12 && cat != 15 {
                        // Refused; the help line still goes up this frame.
                        m.menu_status = 3;
                        refuse(m, 1);
                    } else if useful == 2 {
                        m.menu_status = 3;
                        if cat == 15 && it.id == GRUNTY_FLUTE {
                            if g.area == 1 && (g.field == 13 || g.field == 67) {
                                refuse(m, 4);
                            } else if g.in_battle != 0 {
                                refuse(m, 5);
                            } else if g.area == 1 {
                                m.proccess = 20;
                            } else if g.area == 2 && (g.dungeon_type == 8 || g.dungeon_type == 9) {
                                refuse(m, 4);
                            } else {
                                refuse(m, 2);
                            }
                        } else if cat == 15 && RYU_BOOKS.contains(&it.id) && g.area != 0 {
                            refuse(m, 8);
                        } else {
                            m.proccess += 1;
                        }
                        return Flow::Done;
                    } else if useful == 1 {
                        m.change_menu_to(65);
                        return Flow::Done;
                    }
                }
                _ => {}
            }
            item_help(m, x, it);
        }
        2 => {
            if m.menu_status != 0 {
                return Flow::Done;
            }
            let list = important_list_fit(m, x);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let me = x.world.player().map_or(0, |p| p.handle);
            x.req.push(Request::UseItem { target: me, code: it.code() });
            if it.cat == 15 && it.id == GRUNTY_FLUTE {
                return personal::close(m, x);
            }
            if it.cat == 15 && RYU_BOOKS.contains(&it.id) {
                // EntryFade(1, black, black, 0, 0, 512, 448): the screen
                // covered for the book's first frame.
                let id = m.menu_fade.entry_fade(1, 0x8000_0000, 0x8000_0000);
                m.msg.close_instant();
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return Flow::Breathed(pers(Tail::BookWoken(id)));
            }
            m.wait_count = 0;
            m.proccess = 12;
        }
        20 => {
            if m.menu_status != 0 {
                return Flow::Done;
            }
            let mut slot = (m.rng)().rem_euclid(3) as usize;
            let mut found = -1;
            for _ in 0..3 {
                if let Some(n) = x.world.pg_adult[slot].filter(|&n| n >= 0) {
                    found = n;
                    break;
                }
                slot = (slot + 1) % 3;
            }
            if found < 0 {
                let w = x.texts.pers.grunty_warn.clone();
                open_info2(m, x, &w);
                m.wait_count = 0;
                m.proccess += 1;
                return Flow::Done;
            }
            // ccUseItemRequest(plw, plw, 0xf0031, n): the ride, then the
            // menu shuts.
            let me = x.world.player().map_or(0, |p| p.handle);
            crate::menus::useitem::call_arg(m, x, me, 0xf_0031, found, crate::menus::useitem::Resume::Flute);
            return Flow::Done;
        }
        21 | 11 => {
            if check(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        22 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.menu_status = 1;
                m.proccess = 0;
            }
        }
        10 => {
            if m.menu_status == 0 {
                let h = x.texts.item_help.get(m.wait_count.clamp(0, 9) as usize).cloned().unwrap_or_default();
                open_info2(m, x, &h);
                m.proccess += 1;
            }
        }
        12 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.menu_status = 1;
                m.exception_disp = 1;
                m.proccess = 1;
            }
        }
        _ => {}
    }
    Flow::Done
}

/// A Ryu Book's fade back in (0x00530208): a frame drawn and breathed
/// while `CheckFade`; then the fade deleted, the dim back, 3 frames.
pub fn book_fade(m: &mut MenuCtrl, id: i32, x: &mut Ctx) -> Option<crate::ctrl::Cont> {
    if check_fade(m, id) {
        crate::disp::disp(m, x);
        return Some(pers(Tail::BookFade(id)));
    }
    delete_fade(m, id);
    m.bg_status = 1;
    m.wait_count = 5;
    m.proccess = 12;
    None
}

/// Whether ImportantItemMenuDisp greys a key item.
fn key_item_greyed(x: &Ctx, it: Item) -> bool {
    let (cat, id) = (i32::from(it.cat), i32::from(it.id));
    let useful = items::check_item_useful(cat, id);
    let g = x.world.game;
    if useful == 0 {
        return true;
    }
    if g.area == 0 && cat != 12 && cat != 15 {
        return true;
    }
    if useful != 2 {
        return false;
    }
    let mut grey = false;
    if cat == 15 && it.id == GRUNTY_FLUTE {
        grey = g.in_battle != 0 || g.area != 1 || g.field == 13 || g.field == 67;
    }
    if cat == 15 && RYU_BOOKS.contains(&it.id) && g.area != 0 {
        grey = true;
    }
    grey
}

/// `ImportantItemMenuDisp`.
pub fn important_item_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    m.win.set_alpha(m.alpha);
    set_type(&mut m.win, 1);
    m.win.dx = 39.0;
    m.win.dy = 96.0;
    disp_square_tag(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.page), i32::from(l.page_num), 8);
    if l.y < l.my {
        m.win.dx = 39.0;
        m.win.dy = 96.0;
        disp_scroll_bar(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.dy), i32::from(l.my));
    }
    m.win.dx = 55.0;
    m.win.dy = 64.0;
    disp_button(&mut m.win, 4, x.count, x.frame_rate);
    m.win.dx = from_int((i32::from(l.x) + 2) * 14 + 23);
    m.win.dy = 64.0;
    disp_button(&mut m.win, 6, x.count, x.frame_rate);
    m.win.set_colour(7);
    m.win.set_alpha(m.alpha);
    m.win.cx = 0.0;
    m.win.cy = 0.0;
    if m.kanji_alpha > 0 {
        set_clm(&mut m.kanji, 16, 0, 0, 1);
        m.kanji.set_colour(7);
        m.kanji.set_alpha(m.kanji_alpha);
        m.kanji.dx = from_int((i32::from(l.page) + 2) * 14 + 39);
        m.kanji.dy = 84.0;
        m.kanji.make_packet(0);
        let tag = x.texts.pers.imp_tags.get(l.page.max(0) as usize).cloned().unwrap_or_default();
        extract_menu(m, tag);
    }
    let w = if l.y < l.my { i32::from(l.x) - 1 } else { i32::from(l.x) };
    let (a, fr) = (m.alpha, x.frame_rate);
    m.cursor.disp(
        &mut m.win,
        39.0,
        from_int((i32::from(l.select) - i32::from(l.dy)) * 20 + 112),
        w,
        i32::from(l.select) + (i32::from(l.page) << 8),
        a,
        3,
        fr,
    );
    let list = important_list(x.save, i32::from(l.page));
    if m.kanji_alpha <= 0 {
        return;
    }
    set_clm(&mut m.setting[0], 16, 0, 0, 1);
    set_clm(&mut m.setting[1], 16, 0, 0, 1);
    font_type(&mut m.font, 1);
    let dy = i32::from(l.dy);
    let mut buf = Vec::new();
    let (mut row, mut kanji) = (0i32, 0usize);
    for (k, &it) in list.iter().enumerate() {
        if it.cat < 0 || (k as i32) < dy {
            continue;
        }
        let name = x.texts.items.item_name(i32::from(it.cat), i32::from(it.id));
        str_cat(&mut buf, &name, 16);
        let c = if key_item_greyed(x, it) { 0 } else { 7 };
        m.setting[kanji].set_colour(c);
        m.font.set_colour(c);
        m.setting[kanji].set_alpha(m.kanji_alpha);
        m.font.set_alpha(m.kanji_alpha);
        let y = (row + (kanji as i32) * 8) * 20 + 112;
        m.setting[kanji].dx = 67.0;
        m.setting[kanji].dy = from_int(y);
        m.setting[kanji].make_packet(row);
        m.font.dx = 235.0;
        m.font.dy = from_int(y);
        make_num(&mut m.font, 2, i32::from(it.num));
        row += 1;
        if row >= 8 {
            m.setting_text[kanji] = std::mem::take(&mut buf);
            row = 0;
            kanji += 1;
        }
        if kanji > 0 {
            break;
        }
    }
    if row != 0 {
        m.setting_text[kanji] = buf;
    }
}

// --- Discard Item ---------------------------------------------------------

/// `SetItemList(0, page, list, flag)`: the player's items on a page.
fn throw_list(m: &mut MenuCtrl, x: &Ctx, fit: bool) -> [Item; ITEMS] {
    let i = idx(m);
    let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
    if fit {
        let n = list.iter().filter(|it| it.cat >= 0).count() as i16;
        items::fit_list(&mut m.lists[i], n);
    }
    list
}

fn init_cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// `ThrowItemMenu` (menu 7).
pub fn throw_item_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.exception_disp = 1;
            m.bg_status = 1;
            let l = &mut m.lists[i];
            l.page_num = 5;
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            throw_list(m, x, true);
            m.proccess += 1;
        }
        1 => {
            let old_page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old_page != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = throw_list(m, x, true);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&it.cat) {
                x.se(SE_OK);
                m.item_num = (i32::from(it.cat) << 16) | i32::from(it.id);
                m.change_menu_to(64);
                return Flow::Done;
            }
            match key(x) {
                Some(false) => {
                    init_cursors(m);
                    let prev = m.lists[i].prev;
                    m.back_to_prev(prev);
                    return Flow::Done;
                }
                Some(true) => {
                    init_cursors(m);
                    if it.cat >= 0 {
                        m.exception_disp = 2;
                        m.wait_count = 1;
                        m.proccess += 1;
                    }
                }
                None => {}
            }
            item_help(m, x, it);
        }
        2 => {
            let list = throw_list(m, x, false);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let num = i16::from(it.num);
            let repeat = x.pad.repeat.bits();
            if repeat & 0x1000 != 0 {
                m.wait_count += 1;
                if num < m.wait_count {
                    m.wait_count = num;
                } else {
                    x.se(SE_MOVE);
                }
            } else if repeat & 0x4000 != 0 {
                m.wait_count -= 1;
                if m.wait_count <= 0 {
                    m.wait_count = 1;
                } else {
                    x.se(SE_MOVE);
                }
            } else if repeat & 0x8000 != 0 {
                if m.wait_count < num {
                    m.wait_count += 10;
                    x.se(SE_MOVE);
                    if num < m.wait_count {
                        m.wait_count = num;
                    }
                } else {
                    m.wait_count = num;
                }
            } else if repeat & 0x2000 != 0 {
                if m.wait_count >= 2 {
                    m.wait_count -= 10;
                    x.se(SE_MOVE);
                    if m.wait_count <= 0 {
                        m.wait_count = 1;
                    }
                } else {
                    m.wait_count = 1;
                }
            }
            match key(x) {
                Some(false) => {
                    m.exception_disp = 1;
                    init_cursors(m);
                    m.proccess = 1;
                }
                Some(true) => {
                    m.menu_status = 3;
                    init_cursors(m);
                    m.proccess += 1;
                }
                None => {}
            }
            let h = x.texts.item_help.get(6).cloned().unwrap_or_default();
            let names = x.save.names();
            m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(b"")], &names);
        }
        3 => {
            if m.menu_status != 0 {
                return Flow::Done;
            }
            let list = throw_list(m, x, false);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let mut text = x.texts.item_help.get(7).map(|h| h[0].clone()).unwrap_or_default();
            text.extend_from_slice(&piney_desktop::kanji::dec2sjis(i32::from(m.wait_count), 16, 0));
            text.extend_from_slice(b" #G");
            text.extend_from_slice(&x.texts.items.item_name(i32::from(it.cat), i32::from(it.id)));
            text.extend_from_slice(b"#W.");
            let names = x.save.names();
            m.msg.open_info([Some(&text), None, None, None], &names);
            m.exception_disp = 0;
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.disp = 11;
            l.sx = l.x;
            l.sy = l.y;
            l.x = 6;
            l.y = 2;
            l.index = l.select;
            l.select = 0;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            init_cursors(m);
            m.proccess += 1;
        }
        4 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(false) => {
                    m.menu_status = 3;
                    m.msg.close();
                    init_cursors(m);
                    m.proccess += 1;
                }
                Some(true) => {
                    m.msg.close();
                    init_cursors(m);
                    if m.lists[i].select == 1 {
                        m.menu_status = 3;
                        m.proccess += 1;
                    } else {
                        let list = throw_list(m, x, false);
                        let at = m.lists[i].index.max(0) as usize;
                        let it = list.get(at).copied().unwrap_or(Item::NONE);
                        items::del_item(x.save, 0, i32::from(it.cat), i32::from(it.id), i32::from(m.wait_count));
                        m.menu_status = 3;
                        m.proccess += 1;
                    }
                }
                None => {}
            }
        }
        5 => {
            if m.menu_status != 0 {
                return Flow::Done;
            }
            let l = &mut m.lists[i];
            l.select = l.index;
            l.x = l.sx;
            l.y = l.sy;
            l.disp = 4;
            m.exception_disp = 1;
            m.menu_status = 1;
            init_cursors(m);
            m.proccess = 1;
        }
        _ => {}
    }
    Flow::Done
}

/// `ThrowItemMenuDisp`: the Items page, and while the count is asked
/// (`exceptionDisp` 2) its window by the row: "Discard" and the count.
pub fn throw_item_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    crate::menus::item::item_menu_disp(m, x);
    if m.exception_disp != 2 {
        return;
    }
    let l = m.lists[idx(m)].clone();
    let wx = i32::from(l.x) * 14 + 87;
    let wy = (i32::from(l.select) - i32::from(l.dy)) * 20 + 112;
    let a = m.alpha;
    m.win_pr.set_colour(7);
    m.win_pr.set_alpha(a);
    m.win_pr.dx = from_int(wx);
    m.win_pr.dy = from_int(wy - 16);
    crate::window::disp_square_w2(&mut m.win_pr, 5, 2, 1, None);
    let fr = x.frame_rate;
    m.cursor_pr.disp(&mut m.win_pr, from_int(wx), from_int(wy), 8, 0, a, 6, fr);
    m.kanji_pr_text = x.texts.pers.shop_str.clone();
    set_clm(&mut m.kanji_pr, 16, 0, 0, 1);
    m.kanji_pr.set_colour(7);
    m.kanji_pr.set_alpha(a);
    m.kanji_pr.dx = from_int(wx + 14);
    m.kanji_pr.dy = from_int(wy);
    m.kanji_pr.make_packet(6);
    m.font.set_colour(22);
    m.font.set_alpha(a);
    m.font.dx = from_int(wx + 106);
    m.font.dy = from_int(wy);
    make_num(&mut m.font, 2, i32::from(m.wait_count));
}
