//! An item got (`GetItemMenu`, gcmn 0x00544020, menu 29), the full bag's
//! exchange (`ReplaceItemMenu`, 0x00544e50, menu 30), the box's draw
//! (`AreaItem`, 0x00544c00) and the treasure box of the item tutorial
//! (`ItemBoxMenuT`, 0x0056a300, menus 84 and 85). Both 29 and 30 end back
//! in the Data Drain (67) when 29 came from it, else shut. The steps are in
//! docs/engine/field-ui.md (an item got, the tutorials).

use piney_event::ScriptSave;

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK, SE_OPEN};
use crate::items::{self, ITEMS, Item};
use crate::menus::system::extract_menu;

/// `saveData.impItemList` (+0xcfc): the key items' counts.
pub const IMP_ITEM_LIST: usize = 0x0cfc;
/// `saveData.itemBoxCount` (+0x7440).
pub const ITEM_BOX_COUNT: usize = 0x7440;
/// `ccSeOn(74)`: an item got.
pub const SE_GET: i32 = 74;

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// `ccSaveData::GetItemNum(pc, cat, id)` (main 0x00177990).
pub fn item_count(x: &Ctx, pc: usize, cat: i32, id: i32) -> i32 {
    if pc == 0 && cat == 15 {
        return i32::from(x.save.save.u8(IMP_ITEM_LIST + (id as u32 as usize & 0xffff)) as i8);
    }
    (0..ITEMS)
        .map(|k| items::save_item(x.save, pc, k))
        .find(|it| i32::from(it.id) == id && i32::from(it.cat) == cat)
        .map_or(0, |it| i32::from(it.num))
}

/// `ccSaveData::GetItemSlot(pc)` (main 0x00177a20): the first free slot,
/// -1 when the bag is full.
pub fn free_slot(x: &Ctx, pc: usize) -> i32 {
    (0..ITEMS)
        .find(|&k| {
            let it = items::save_item(x.save, pc, k);
            it.id < 0 && it.cat < 0
        })
        .map_or(-1, |k| k as i32)
}

fn add_item(x: &mut Ctx, code: i32, n: i16) {
    x.save.save.add_item(0, (code >> 16) as i16, (code & 0xffff) as i16, n);
}

fn name_of(x: &Ctx, code: i32) -> Vec<u8> {
    x.texts.items.item_name(code >> 16, code & 0xffff)
}

/// `getItemMenuStr` line `k`, then "#G", the item's name and `tail`.
fn line(x: &Ctx, k: usize, code: i32, tail: &[u8]) -> Vec<u8> {
    let mut s = x.texts.get_item_str.get(k).cloned().unwrap_or_default();
    s.extend_from_slice(&x.texts.gi_green);
    s.extend_from_slice(&name_of(x, code));
    s.extend_from_slice(tail);
    s
}

fn info(m: &mut MenuCtrl, x: &Ctx, a: &[u8], b: Option<&[u8]>, open: bool) {
    let names = x.save.names();
    if open {
        m.msg.open_info([Some(a), b, None, None], &names);
    } else {
        m.msg.change_info([Some(a), b, None, None], &names);
    }
}

fn check(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    crate::menus::tutorial::check(m, x)
}

fn wait(m: &mut MenuCtrl, n: i16) {
    let w = m.wait_count;
    m.wait_count += 1;
    if w >= n {
        m.proccess += 1;
    }
}

/// The menu's end: back into the Data Drain (67) after 7 frames, or
/// shut; after the close the panels go out when `panels` (the menu came
/// from the box tutorial, 84, only for the plain get) or when opened by
/// itself or while the menus are held.
fn leave(m: &mut MenuCtrl, x: &mut Ctx, from_box: bool) -> Flow {
    let i = idx(m);
    let prev = m.lists[i].prev;
    if prev == 67 {
        let w = m.wait_count;
        m.wait_count += 1;
        if w >= 7 {
            m.back_to_prev(prev);
        }
        return Flow::Done;
    }
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let panels = (from_box && prev == 84) || prev == -1 || m.forbid != 0;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont { woke, cursors: false, after: After::Panels(panels) })
}

/// `GetItemMenu` (menu 29): `itemNum` got.
pub fn get_item_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let code = m.item_num;
    match m.proccess {
        0 => {
            m.lists[i].disp = 4;
            m.menu_status = 0;
            let t = line(x, 0, code, &x.texts.gi_bang.clone());
            if code >> 16 == 15 {
                let k = x.texts.get_item_str.get(8).cloned().unwrap_or_default();
                info(m, x, &t, Some(&k), true);
            } else {
                info(m, x, &t, None, true);
            }
            x.se(SE_GET);
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 | 11 | 13 | 23 => wait(m, 6),
        2 => {
            if check(m, x) {
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        3 => {
            let (cat, id) = (code >> 16, code & 0xffff);
            let add = if cat == 15 {
                if item_count(x, 0, 15, id) < 99 {
                    true
                } else {
                    m.proccess = 10;
                    false
                }
            } else {
                let n = item_count(x, 0, cat, id);
                if n >= 99 {
                    m.proccess = 10;
                    false
                } else if n > 0 || free_slot(x, 0) >= 0 {
                    true
                } else {
                    m.proccess = 20;
                    false
                }
            };
            if add {
                add_item(x, code, 1);
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        4 => return leave(m, x, true),
        10 => {
            let t = line(x, 1, code, &x.texts.gi_dot.clone());
            info(m, x, &t, None, false);
            m.wait_count = 0;
            m.cursor.init(false);
            m.cursor_pr.init(false);
            m.proccess += 1;
        }
        12 | 22 => {
            if m.proccess == 22 || check(m, x) {
                let t = line(x, 4, code, &x.texts.gi_dot.clone());
                info(m, x, &t, None, false);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        14 | 24 => {
            if check(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        15 | 25 => return leave(m, x, false),
        20 => {
            m.menu_status = 1;
            m.lists[i].disp = 11;
            m.lists[i].select = 0;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            let s = &x.texts.get_item_str;
            let (a, b) = (s.get(2).cloned().unwrap_or_default(), s.get(3).cloned().unwrap_or_default());
            info(m, x, &a, Some(&b), false);
            m.cursor.init(false);
            m.cursor_pr.init(false);
            m.proccess += 1;
        }
        21 => {
            m.select(0, 0, false, x);
            if x.pushed_ok() {
                x.se(SE_OK);
                m.cursor.init(true);
                m.cursor_pr.init(true);
                m.menu_status = 3;
                if m.lists[i].select == 1 {
                    m.proccess += 1;
                } else {
                    m.msg.close();
                    m.trap_num = m.item_num;
                    m.change_menu_to(30);
                }
            }
        }
        _ => {}
    }
    Flow::Done
}

/// What ReplaceItemMenu does after its close's breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplaceTail {
    /// The exchange: the panels out when `panels`, then the picked item
    /// out and the new one in.
    Swap { panels: bool },
    /// Given up: the panels out when `panels`, then `proccess` on.
    GaveUp { panels: bool },
}

/// The pick's cancel and OK: cancel (sound 19) wins over OK (18).
fn answer(x: &mut Ctx) -> u8 {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        2
    } else if x.pushed_ok() {
        x.se(SE_OK);
        1
    } else {
        0
    }
}

/// The OK / Cancel question over the list: the list's place kept.
fn question(m: &mut MenuCtrl, x: &mut Ctx, a: &[u8], b: Option<&[u8]>) {
    let i = idx(m);
    info(m, x, a, b, true);
    m.cursor.init(false);
    m.cursor_pr.init(false);
    m.exception_disp = 0;
    m.menu_status = 1;
    let l = &mut m.lists[i];
    l.disp = 11;
    l.sx = l.x;
    l.sy = l.y;
    m.wait_count = l.select;
    let l = &mut m.lists[i];
    l.select = 0;
    l.x = 6;
    l.y = 2;
    let d = x.texts.dialog_default.clone();
    extract_menu(m, d);
    m.proccess += 1;
}

/// The list back as it was before the question.
fn restore(m: &mut MenuCtrl) {
    let i = idx(m);
    let w = m.wait_count;
    let l = &mut m.lists[i];
    l.disp = 4;
    l.select = w;
    l.x = l.sx;
    l.y = l.sy;
}

fn item_at(m: &MenuCtrl, x: &Ctx) -> Item {
    let i = idx(m);
    let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
    list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE)
}

/// The end of the exchange or the give-up: into the Data Drain (67) when
/// 29 came from it, else shut.
fn replace_leave(m: &mut MenuCtrl, x: &mut Ctx, swap: bool) -> Flow {
    let i = idx(m);
    let prev = m.lists[m.lists[i].prev.clamp(0, 88) as usize].prev;
    if prev == 67 {
        m.change_menu_to(67);
        if swap {
            swap_items(m, x);
        } else {
            m.proccess += 1;
        }
        return Flow::Done;
    }
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let panels = prev == -1;
    let woke = m.breathe_close(x);
    let t = if swap { ReplaceTail::Swap { panels } } else { ReplaceTail::GaveUp { panels } };
    Flow::Breathed(Cont { woke, cursors: false, after: After::Replace(t) })
}

fn swap_items(m: &mut MenuCtrl, x: &mut Ctx) {
    let it = item_at(m, x);
    items::del_item(x.save, 0, i32::from(it.cat), i32::from(it.id), 99);
    let t = m.trap_num;
    add_item(x, t, 1);
}

/// ReplaceItemMenu after its close.
pub fn replace_after(m: &mut MenuCtrl, t: ReplaceTail, x: &mut Ctx) {
    match t {
        ReplaceTail::Swap { panels } => {
            if panels {
                m.panel_status = 3;
            }
            swap_items(m, x);
        }
        ReplaceTail::GaveUp { panels } => {
            if panels {
                m.panel_status = 3;
            }
            m.proccess += 1;
        }
    }
}

/// `ReplaceItemMenu` (menu 30): the bag is full; `trapNum` is the new
/// item.
pub fn replace_item_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let new = m.trap_num;
    match m.proccess {
        0 => {
            m.exception_disp = 1;
            let l = &mut m.lists[i];
            l.page_num = 5;
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            crate::menus::item::set_item_list_fit(m, x, 0);
            m.proccess += 1;
        }
        1 => {
            let old = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = crate::menus::item::set_item_list_fit(m, x, 0);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&it.cat) {
                x.se(SE_OK);
                m.item_num = (i32::from(it.cat) << 16) | i32::from(it.id);
                m.change_menu_to(64);
                return Flow::Done;
            }
            match answer(x) {
                2 => {
                    m.cursor.init(true);
                    m.cursor_pr.init(true);
                    m.menu_status = 3;
                    m.proccess = 10;
                    return Flow::Done;
                }
                1 => {
                    m.cursor.init(true);
                    m.cursor_pr.init(true);
                    if it.cat >= 0 {
                        m.menu_status = 3;
                        m.proccess += 1;
                        return Flow::Done;
                    }
                }
                _ => {}
            }
            let names = x.save.names();
            match x.texts.items.item(i32::from(it.cat), i32::from(it.id)).filter(|_| it.cat >= 0 && it.id >= 0) {
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
        2 | 4 => {
            if m.menu_status == 0 {
                if m.proccess == 4 {
                    restore(m);
                }
                let it = item_at(m, x);
                let (k1, k2, tail) =
                    if m.proccess == 2 { (11, 12, &x.texts.gi_question) } else { (5, 9, &x.texts.gi_bang) };
                let tail = tail.clone();
                let a = line(x, k1, it.code(), b"");
                let b = line(x, k2, new, &tail);
                if m.proccess == 2 {
                    question(m, x, &a, Some(&b));
                } else {
                    info(m, x, &a, Some(&b), true);
                    m.wait_count = 0;
                    m.proccess += 1;
                }
            }
        }
        3 | 11 => {
            m.select(0, 0, false, x);
            let a = answer(x);
            let sel = m.lists[i].select;
            if a == 1 && sel == 0 {
                m.cursor.init(true);
                m.cursor_pr.init(true);
                m.menu_status = 3;
                m.msg.close();
                m.proccess += 1;
            } else if (a == 1 && sel == 1) || a == 2 {
                m.cursor.init(true);
                m.cursor_pr.init(true);
                m.menu_status = 3;
                m.msg.close();
                m.proccess = 100;
            }
        }
        5 | 13 => wait(m, 9),
        6 | 14 => {
            if check(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        7 | 15 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 9 {
                return replace_leave(m, x, m.proccess == 7);
            }
        }
        10 => {
            if m.menu_status == 0 {
                let t = line(x, 10, new, &x.texts.gi_question.clone());
                question(m, x, &t, None);
            }
        }
        12 => {
            if m.menu_status == 0 {
                restore(m);
                let t = line(x, 4, new, &x.texts.gi_dot.clone());
                info(m, x, &t, None, false);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        100 if m.menu_status == 0 => {
            m.menu_status = 1;
            restore(m);
            m.cursor.init(true);
            m.cursor_pr.init(true);
            m.proccess = 0;
        }
        _ => {}
    }
    Flow::Done
}

/// `AreaItem(n, kind)`: `n` when it is an item, else a draw from the
/// area's list of `kind` (0 the boxes, 1 the dangerous boxes, 2 the
/// Grunty's, 4 the idols, 5 their second list, else the area's items):
/// by the story area's item level in a story field, else by the words'
/// area level and item offset, plus the floor and `rand() % 5`.
pub fn area_item(m: &mut MenuCtrl, x: &Ctx, n: i32, kind: i32) -> i32 {
    if n >= 0 {
        return n;
    }
    let w = &x.world;
    let server = w.game.server.clamp(0, 4) as usize;
    let attr = w.field_attr.clamp(0, 5) as usize;
    let t = &x.texts.area_items;
    let list = match kind {
        0 => &t.box_list[server * 6 + attr],
        1 => &t.danger[server * 6 + attr],
        2 => &t.suka[server],
        4 => &t.idol[server * 6 + attr],
        5 => &t.idol_sub[server],
        _ => &t.area[server * 6 + attr],
    };
    let mut k = if w.game.field != 0 {
        x.texts.words.event(w.event_area).map_or(0, |e| e.item)
    } else {
        (w.area_word.0 - 1) * 25 + 10 + w.area_word.1
    };
    k += w.game.floor + 1;
    k += (m.rng)() % 5;
    if k >= 130 {
        k = 129;
    }
    list.get(k.max(0) as usize).copied().unwrap_or(-1)
}

/// `ItemBoxMenuT` (menus 84 and 85): the tutorial's treasure box.
pub fn item_box_menu_t(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            let n = if m.menu == 84 { 16 } else { 20 };
            crate::menus::tutorial::open(m, x, 4, n);
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 9 && x.pad.push.bits() & x.assign(0) != 0 {
                x.se(SE_OPEN);
                x.req.push(Request::VoiceStop);
                m.msg.close();
                x.req.push(Request::TargetFix(true));
                m.item_num = -1;
                m.trap_num = -1;
                if let Some(t) = x.target.clone() {
                    x.req.push(Request::Affect { target: t.handle, kind: 11 });
                    m.item_num = t.item;
                }
                x.change_target(None);
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        2 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 21 {
                if m.still == 0 {
                    m.bg_status = 1;
                    m.mode = 0;
                    x.req.push(Request::SleepAll);
                    m.still = 1;
                    x.req.push(Request::Still(true));
                    x.req.push(Request::KeepLayers);
                }
                m.proccess += 1;
            }
        }
        3 => {
            let n = m.item_num;
            m.item_num = area_item(m, x, n, 0);
            let c = (i32::from(x.save.save.i16(ITEM_BOX_COUNT)) + 1).min(10000);
            x.save.save.set_i16(ITEM_BOX_COUNT, c as i16);
            m.change_menu_to(29);
        }
        _ => {}
    }
    Flow::Done
}

/// `saveData.idolCount` (+0x746e): the idols opened.
pub const IDOL_COUNT: usize = 0x746e;

/// The box menus' first frame: `cmndTargetFix`, the target opened
/// (`EntryAffect(cmndTarget, plw, 11)`: the box's act 2, its trap on the
/// opener), its item (+0x14c) kept, the target dropped.
fn open_target(m: &mut MenuCtrl, x: &mut Ctx) -> Option<crate::world::CharInfo> {
    x.req.push(Request::TargetFix(true));
    m.item_num = -1;
    let t = x.target.clone();
    if let Some(t) = &t {
        x.req.push(Request::Affect { target: t.handle, kind: 11 });
        m.item_num = t.item;
    }
    x.change_target(None);
    m.wait_count = 0;
    m.proccess += 1;
    t
}

/// 21 frames, then every other task asleep and their layers held.
fn wait_then_sleep(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let w = m.wait_count;
    m.wait_count += 1;
    if w < 21 {
        return false;
    }
    if m.still == 0 {
        sleep_others(m, x);
    }
    true
}

/// The object menus' sleep (with `still` 0): `bgStatus` 1, `mode` 0,
/// `ccSleepAllThread`, `still` 1 with the flips off, the three layers
/// held.
pub(crate) fn sleep_others(m: &mut MenuCtrl, x: &mut Ctx) {
    m.bg_status = 1;
    m.mode = 0;
    x.req.push(Request::SleepAll);
    m.still = 1;
    x.req.push(Request::Still(true));
    x.req.push(Request::KeepLayers);
}

/// `itemBoxCount` up (at most 10000).
fn count_box(x: &mut Ctx, at: usize) {
    let c = (i32::from(x.save.save.i16(at)) + 1).min(10000);
    x.save.save.set_i16(at, c as i16);
}

/// `ItemBoxMenu` (gcmn 0x00546450, menu 32): a treasure box opened. Its
/// item, or a draw: from the danger list when it was trapped (`param[2]`
/// 0 on) or one time in eight, else the box list; `itemBoxCount` up, the
/// party's line, into 29.
pub fn item_box_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            m.trap_num = -1;
            if let Some(t) = open_target(m, x) {
                m.trap_num = t.trap;
            }
        }
        1 => {
            if wait_then_sleep(m, x) {
                m.proccess += 1;
            }
        }
        2 => {
            let n = m.item_num;
            let kind = if m.trap_num >= 0 || (m.rng)() & 7 == 0 { 1 } else { 0 };
            m.item_num = area_item(m, x, n, kind);
            count_box(x, ITEM_BOX_COUNT);
            x.req.push(Request::SpcMessageOpenTreasureBox);
            m.change_menu_to(29);
        }
        _ => {}
    }
    Flow::Done
}

/// `TrapBoxMenu` (gcmn 0x005466d0, menu 33): a trapped box opened as it
/// is. Its trap goes off on the opener (the box's own frame); "Set off
/// trap!" and the trap's line (skill 1 the explosion, 156 the poison gas,
/// 162 the cursed gas) until the button; then a draw from the dud list
/// (`SukaItemBoxList`) unless the box holds an item, `itemBoxCount` up,
/// into 29.
pub fn trap_box_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            m.trap_num = -1;
            if let Some(t) = open_target(m, x) {
                m.trap_num = i32::from(t.skill);
            }
        }
        1 => {
            if wait_then_sleep(m, x) {
                m.proccess = if m.trap_num == -1 { 6 } else { m.proccess + 1 };
            }
        }
        2 => {
            let line = match m.trap_num {
                162 => 3,
                156 => 2,
                1 => 1,
                _ => {
                    m.proccess = 6;
                    return Flow::Done;
                }
            };
            let names = x.save.names();
            let t = &x.texts.trap_menu;
            let l = |k: usize| t.get(k).map(Vec::as_slice);
            m.msg.open_info([l(0), l(line), None, None], &names);
            m.wait_count = 0;
            m.proccess += 1;
        }
        3 => wait(m, 9),
        4 => {
            if check(m, x) {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        5 => wait(m, 7),
        6 => {
            let n = m.item_num;
            m.item_num = area_item(m, x, n, 2);
            count_box(x, ITEM_BOX_COUNT);
            m.change_menu_to(29);
        }
        _ => {}
    }
    Flow::Done
}

/// `ItemIdolMenu` (gcmn 0x005477e0, menu 38): an idol (a Gott statue)
/// opened. `idolCount` up, the party's line, and three items through the
/// Data Drain's page (67): the idol's own or a draw from the idol list,
/// then two from the server's sub list.
pub fn item_idol_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            open_target(m, x);
        }
        1 => {
            if wait_then_sleep(m, x) {
                m.proccess += 1;
            }
        }
        2 => {
            count_box(x, IDOL_COUNT);
            x.req.push(Request::SpcMessageOpenTreasureBox);
            m.drain_num = 3;
            let n = m.item_num;
            m.talk.drain_item[2] = area_item(m, x, n, 4);
            m.talk.drain_item[1] = area_item(m, x, -1, 5);
            m.talk.drain_item[0] = area_item(m, x, -1, 5);
            m.change_menu_to(67);
        }
        _ => {}
    }
    Flow::Done
}

/// `DataDrainSubMenu` (gcmn 0x00535210, menu 67): the drained (or the
/// idol's) items one at a time through 29, the last first, which comes
/// back here; then the menu shuts.
pub fn data_drain_sub_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if m.drain_num != 0 {
        m.drain_num -= 1;
        m.item_num = m.talk.drain_item[m.drain_num.clamp(0, 16) as usize];
        m.change_menu_to(29);
        return Flow::Done;
    }
    m.close_menu(x)
}
