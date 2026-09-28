//! `ItemMenu` (gcmn 0x0052db30, menu 5) and `ItemMenuDisp` (0x0052e6c0):
//! the Items list, a page each for items, scrolls, books, treasure and
//! equipment, with the carried and stored counts. OK goes by
//! `ccCheckItemUseful`: used at once, TARGET (65), or refused; the Sprite
//! Ocarina (13/1) asks "Return to the field." first and only in a
//! dungeon. The steps are in docs/engine/field-ui.md (Items).

use piney_desktop::eef::from_int;

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::items::{self, ITEMS, Item};
use crate::menus::check_operate;
use crate::menus::skill::tab_frame;
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::spr::{font_type, make_num, set_clm};
use crate::window::set_type;

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// `SetItemList(0, page, list, 1)`.
pub fn set_item_list_fit(m: &mut MenuCtrl, x: &Ctx, pc: usize) -> [Item; ITEMS] {
    let i = idx(m);
    let page = i32::from(m.lists[i].page);
    let list = items::item_list(&x.texts.items, x.save, pc, page);
    let n = list.iter().filter(|it| it.cat >= 0).count() as i16;
    items::fit_list(&mut m.lists[i], n);
    list
}

/// Whether a use-at-once 13/1 (the Sprite of Return) is refused, and with
/// which help: only in dungeons, not on floors 8 and 9, not in battle, not
/// while the events hold operation 14.
fn sprite_refusal(x: &mut Ctx) -> Option<i16> {
    let g = x.world.game;
    if g.area != 2 || g.dungeon_type == 8 || g.dungeon_type == 9 {
        return Some(3);
    }
    if g.in_battle != 0 {
        return Some(5);
    }
    if !check_operate(x, 14, 1) {
        return Some(4);
    }
    None
}

fn refuse(m: &mut MenuCtrl, wc: i16) {
    m.menu_status = 3;
    m.wait_count = wc;
    m.proccess += 1;
}

pub fn item_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
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
            set_item_list_fit(m, x, 0);
            m.proccess += 1;
        }
        1 => {
            let old_page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old_page != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = set_item_list_fit(m, x, 0);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let push = x.pad.push.bits();
            if push & 0x10 != 0 && (0..10).contains(&it.cat) {
                x.se(SE_OK);
                m.item_num = (i32::from(it.cat) << 16) | i32::from(it.id);
                m.change_menu_to(64);
                return Flow::Done;
            }
            let cancel = x.pushed_cancel();
            let ok = !cancel && x.pushed_ok();
            if cancel {
                x.se(SE_BACK);
                let prev = m.lists[i].prev;
                m.back_to_prev(prev);
                return Flow::Done;
            }
            if ok {
                x.se(SE_OK);
                let (cat, id) = (i32::from(it.cat), i32::from(it.id));
                let useful = items::check_item_useful(cat, id);
                if cat >= 0 {
                    if useful == 0 {
                        refuse(m, 0);
                        return Flow::Done;
                    }
                    if x.world.game.area == 0 && cat != 12 && cat != 15 {
                        // Refused, and the help line still goes up this frame.
                        refuse(m, 1);
                    } else if useful == 2 {
                        m.menu_status = 3;
                        if cat == 13 && id == 1 {
                            match sprite_refusal(x) {
                                Some(wc) => {
                                    m.wait_count = wc;
                                    m.proccess += 1;
                                }
                                None => {
                                    m.cursor.init(true);
                                    m.cursor_pr.init(true);
                                    m.proccess = 10;
                                }
                            }
                            return Flow::Done;
                        }
                        if cat != 15 {
                            items::del_item(x.save, 0, cat, id, 1);
                        }
                        let me = x.world.player().map_or(0, |p| p.handle);
                        // ccUseItemRequest, then the menu shuts
                        // (`useitem::Resume::AtOnce`).
                        crate::menus::useitem::call(m, x, me, it.code(), crate::menus::useitem::Resume::AtOnce);
                        return Flow::Done;
                    } else if useful == 1 {
                        m.change_menu_to(65);
                        return Flow::Done;
                    }
                }
            }
            // The item's name and comment.
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
        2 => {
            if m.menu_status == 0 {
                let h = x.texts.item_help.get(m.wait_count.clamp(0, 9) as usize).cloned().unwrap_or_default();
                let names = x.save.names();
                let second = if h[1].is_empty() { None } else { Some(&h[1][..]) };
                m.msg.open_info([Some(&h[0]), second, None, None], &names);
                m.proccess += 1;
            }
        }
        3 => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r != 0 {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        4 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.menu_status = 1;
                m.exception_disp = 1;
                m.proccess = 1;
            }
        }
        10 => {
            if m.menu_status == 0 {
                let h = x.texts.item_help.get(9).cloned().unwrap_or_default();
                let names = x.save.names();
                m.msg.open_info([Some(&h[0]), None, None, None], &names);
                m.exception_disp = 0;
                m.reverse_head = 0;
                m.menu_status = 1;
                let l = &mut m.lists[i];
                l.disp = 11;
                l.sx = l.x;
                l.sy = l.y;
                l.x = 6;
                l.y = 2;
                l.index = l.select;
                l.select = 1;
                let d = x.texts.dialog_default.clone();
                extract_menu(m, d);
                m.cursor.init(true);
                m.cursor_pr.init(true);
                m.proccess += 1;
            }
        }
        11 => {
            m.select(0, 0, false, x);
            let cancel = x.pushed_cancel();
            let ok = !cancel && x.pushed_ok();
            if cancel {
                x.se(SE_BACK);
                m.menu_status = 3;
                m.msg.close();
                m.cursor.init(true);
                m.cursor_pr.init(true);
                m.proccess += 2;
            } else if ok {
                x.se(SE_OK);
                m.msg.close();
                m.cursor.init(true);
                m.cursor_pr.init(true);
                m.menu_status = 3;
                m.proccess += if m.lists[i].select != 1 { 1 } else { 2 };
            }
        }
        12 => {
            if m.menu_status == 0 {
                let l = &mut m.lists[i];
                l.select = l.index;
                l.x = l.sx;
                l.y = l.sy;
                items::del_item(x.save, 0, 13, 1, 1);
                let me = x.world.player().map_or(0, |p| p.handle);
                crate::menus::useitem::call(m, x, me, 0xd0001, crate::menus::useitem::Resume::Ocarina);
            }
        }
        13 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            l.x = l.sx;
            l.y = l.sy;
            l.disp = 4;
            m.exception_disp = 1;
            m.menu_status = 1;
            m.cursor.init(true);
            m.cursor_pr.init(true);
            m.proccess = 1;
        }
        _ => {}
    }
    Flow::Done
}

/// Whether ItemMenuDisp greys an item of menu 5 (bit 0 of its flags).
fn item_greyed(x: &mut Ctx, it: Item) -> bool {
    let (cat, id) = (i32::from(it.cat), i32::from(it.id));
    let useful = items::check_item_useful(cat, id);
    let g = x.world.game;
    if useful == 0 {
        return true;
    }
    if g.area == 0 && cat != 12 && cat != 15 {
        return true;
    }
    match useful {
        2 => {
            if cat == 13 && id == 1 {
                let mut grey = g.area != 2 || g.dungeon_type == 8 || g.dungeon_type == 9 || g.in_battle != 0;
                if !check_operate(x, 14, 1) {
                    grey = true;
                }
                grey
            } else {
                false
            }
        }
        1 => {
            if cat == 13 && id == 0 {
                !x.world
                    .sorted
                    .iter()
                    .any(|c| piney_desktop::eef::le(c.cmnd_dist, 7000.0) && c.is(0x28000) && c.in_view)
            } else if cat == 10 && id >= 18 {
                false
            } else {
                let skill = x.texts.items.item(cat, id).map_or(-1, |p| p.skill);
                !items::check_skill_useful(
                    &x.texts.items,
                    &x.world,
                    skill,
                    items::drain_off(x.texts.volume, &x.save.save),
                )
            }
        }
        _ => false,
    }
}

/// How many of a list are held (category and id set, a count above 0).
fn held(list: impl Iterator<Item = Item>) -> i32 {
    list.filter(|it| it.cat >= 0 && it.id >= 0 && it.num > 0).count() as i32
}

/// `ItemMenuDisp`.
pub fn item_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    // ItemStatusMenuDisp's (31): the member Status chose.
    let who = if m.menu == 31 { crate::menus::status::items_member(m, x) } else { None };
    // The item box's count only for the player (party slot 0).
    let is_player = who.as_ref().is_none_or(|v| v.0 == 0);
    let pc = who.as_ref().map(|v| &v.1).or(x.world.player()).map_or(0, |c| c.id.max(0) as usize).min(17);
    let page = m.lists[i].page;
    let list = items::item_list(&x.texts.items, x.save, pc, i32::from(page));
    // The counts: carried of 40, and (the player) the item box of 99.
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(m.alpha);
    let carried = held((0..ITEMS).map(|k| items::save_item(x.save, pc, k)));
    let slash = x.texts.slash.clone();
    m.font.dx = 87.0;
    m.font.dy = 56.0;
    make_num(&mut m.font, 2, carried);
    m.font.make_str(&slash);
    make_num(&mut m.font, 2, 40);
    if is_player {
        let stored = held((0..99).map(|k| items::pl_item(x.save, k)));
        m.font.dx = 159.0;
        m.font.dy = 56.0;
        make_num(&mut m.font, 2, stored);
        m.font.make_str(&slash);
        make_num(&mut m.font, 2, 99);
    }
    let c = if m.exception_disp == 1 { 7 } else { 0 };
    for s in [&mut m.win, &mut m.kanji, &mut m.setting[0], &mut m.item_icon, &mut m.font] {
        s.set_colour(c);
    }
    m.win.set_alpha(m.alpha);
    m.kanji.set_alpha(m.kanji_alpha);
    m.setting[0].set_alpha(m.kanji_alpha);
    m.item_icon.set_alpha(m.kanji_alpha);
    m.font.set_alpha(m.kanji_alpha);
    set_type(&mut m.win, 1);
    m.item_icon.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    let tag = x.texts.item_tags.get(page.max(0) as usize).cloned().unwrap_or_default();
    tab_frame(m, x, &tag, true);
    if m.kanji_alpha <= 0 {
        return;
    }
    set_clm(&mut m.setting[0], 16, 0, 0, 1);
    set_clm(&mut m.setting[1], 16, 0, 0, 1);
    font_type(&mut m.font, 1);
    let dy = i32::from(m.lists[i].dy);
    let mut buf = Vec::new();
    let (mut row, mut kanji) = (0i32, 0usize);
    for (s6, &it) in list.iter().enumerate() {
        if it.cat < 0 || (s6 as i32) < dy {
            continue;
        }
        let name = x.texts.items.item_name(i32::from(it.cat), i32::from(it.id));
        str_cat(&mut buf, &name, 16);
        if m.menu == 5 {
            let c = if item_greyed(x, it) { 0 } else { 7 };
            m.setting[kanji].set_colour(c);
            m.item_icon.set_colour(c);
            m.font.set_colour(c);
            m.setting[kanji].set_alpha(m.kanji_alpha);
            m.item_icon.set_alpha(m.kanji_alpha);
            m.font.set_alpha(m.kanji_alpha);
        } else if m.menu == 54 || m.menu == 55 {
            // Elf's Haven: a row its reverseHead bit greys (0x0052f3e0).
            let greyed = m.reverse_head & 1u32.wrapping_shl((s6 as i32 - dy) as u32) != 0;
            let c = match (greyed, m.exception_disp == 1) {
                (true, true) => 0,
                (true, false) => 8,
                (false, true) => 7,
                (false, false) => 0,
            };
            m.setting[kanji].set_colour(c);
            m.item_icon.set_colour(c);
            m.font.set_colour(c);
            m.setting[kanji].set_alpha(m.kanji_alpha);
            m.item_icon.set_alpha(m.kanji_alpha);
            m.font.set_alpha(m.kanji_alpha);
        }
        let y = (row + (kanji as i32) * 8) * 20 + 112;
        m.item_icon.dx = 51.0;
        m.item_icon.dy = from_int(y - 2);
        let icon = x.texts.items.item_icon(i32::from(it.cat));
        m.item_icon.make_packet(icon);
        m.setting[kanji].dx = 67.0;
        m.setting[kanji].dy = from_int(y);
        m.setting[kanji].make_packet(row);
        m.font.dy = from_int(y);
        if m.menu == 51 {
            // Sell: half the price (0x0052f818).
            m.font.dx = 187.0;
            let p = x.texts.items.item(i32::from(it.cat), i32::from(it.id)).map_or(0, |p| p.price);
            make_num(&mut m.font, 7, p / 2);
            let gp = x.texts.gp.clone();
            m.font.make_str(&gp);
        } else {
            m.font.dx = 235.0;
            make_num(&mut m.font, 2, i32::from(it.num));
        }
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
