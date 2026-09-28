//! The shops' pages: `BuyMenu` (gcmn 0x00556480, menu 52) with
//! `BuyMenuDisp` (0x005572e0), `SellMenu` (0x005552f0, 51) with
//! `SellMenuDisp` (0x00555ee0), and Elf's Haven's `ItemDepositMenu`
//! (0x00559960, 54) and `ItemDrawMenu` (0x0055ac40, 55) with their Disps.
//! The merchant's list (`VenderMenu`, 24) puts the other tasks to sleep,
//! keeps the shop's type in its own list's `index` and drops the target
//! (it is `cmndTargetPrev` on these pages) before `ChangeMenu`. Buy's stock
//! and the steps are in docs/engine/field-ui.md (Buy, Sell, Elf's Haven).

use piney_battle::item::{self as bitem, CategoryOrder};
use piney_desktop::eef::from_int;

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_MOVE, SE_OK};
use crate::items::{self, Item};
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::spr::{font_type, make_num, set_clm};
use crate::talk;
use crate::window::{disp_scroll_bar, disp_square_tag, disp_square_w2, set_type};

pub const STOCK: usize = 24;
pub const SERVERS: usize = 5;
/// `saveData.spcParam[0].base.gold` (+0x749c): the player's money
/// (`plw.pw->base->gold`).
pub const GOLD: usize = 0x7488 + 0x14;
/// The most gold the shops leave the player.
pub const GOLD_MAX: i32 = 9_999_999;

/// The pages' texts and tables.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    /// By server: the item, weapon and magic shops' stock.
    pub item_stock: Vec<Vec<i32>>,
    pub equip_stock: Vec<Vec<i32>>,
    pub magic_stock: Vec<Vec<i32>>,
    pub buy_help: Vec<[Vec<u8>; 3]>,
    /// `sellMenuHelp[2]` (main .sdata 0x00377e40): the count's help and the
    /// question.
    pub sell_help: Vec<[Vec<u8>; 3]>,
    pub shop_str: Vec<u8>,
    /// `itemDepositMenuHelp[4]`, `itemDrawMenuHelp[4]` (main 0x0033eca0,
    /// 0x0033ecb0): the count's help, the question, no room, 99 there.
    pub deposit_help: Vec<[Vec<u8>; 3]>,
    pub draw_help: Vec<[Vec<u8>; 3]>,
    /// `SetPlItemList`'s page table (gcmn 0x006516b0).
    pub pl_pages: [i32; 5],
    /// `addItemCategoryTbl` (main 0x00307140): AddItem's sort.
    pub order: CategoryOrder,
}

impl Texts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> Texts {
        use crate::tables::piece;
        let f = piney_data::tables::fieldui::of(volume);
        let three = |l: &[&str]| [piece(l, 0), piece(l, 1), piece(l, 2)];
        let stock = |l: &[&[i32]]| l.iter().map(|x| x.to_vec()).collect();
        let order = f.category_order();
        Texts {
            item_stock: stock(f.item_shop()),
            equip_stock: stock(f.equip_shop()),
            magic_stock: stock(f.magic_shop()),
            buy_help: f.buy_help().iter().map(|l| three(l)).collect(),
            sell_help: f.sell_help().iter().map(|l| three(l)).collect(),
            shop_str: piney_data::tables::sjis::encode(f.shop_str()),
            deposit_help: f.deposit_help().iter().map(|l| three(l)).collect(),
            draw_help: f.draw_help().iter().map(|l| three(l)).collect(),
            pl_pages: f.pl_item_pages().try_into().unwrap_or([-1; 5]),
            order: std::array::from_fn(|k| (order[k][0], order[k][1])),
        }
    }
}

/// What the pages do after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// BuyMenu's cancel: after the tasks woke (the flips back on), back to
    /// the list, the item's help.
    BuyCancel,
    /// SellMenu's cancel: the same without the help.
    SellCancel,
}

/// The pages' tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::BuyCancel => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.bg_status = 3;
            let i = idx(m);
            let prev = m.lists[i].prev;
            m.menu_next = prev;
            m.menu_status = 3;
            m.proccess = 0;
            m.wait_count = 0;
            m.cursor.init(true);
            m.cursor_pr.init(true);
            let stock = stock(m, x);
            let code = stock.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(-1);
            item_help(m, x, code);
            None
        }
        Tail::SellCancel => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.bg_status = 3;
            let i = idx(m);
            let prev = m.lists[i].prev;
            m.menu_next = prev;
            m.menu_status = 3;
            m.proccess = 0;
            m.wait_count = 0;
            m.cursor.init(true);
            m.cursor_pr.init(true);
            None
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// The player's gold (`plw.pw->base->gold`).
pub fn gold(x: &Ctx) -> i32 {
    x.save.save.i32(GOLD)
}

/// The stock of the shop the list came from: its `index` (the merchant's
/// type) and `game.server`.
pub fn stock(m: &MenuCtrl, x: &Ctx) -> Vec<i32> {
    let prev = m.lists[idx(m)].prev.clamp(0, 88) as usize;
    let t = m.lists[prev].index;
    let s = &x.texts.talk.shop;
    let lists = if t & 0x100 != 0 {
        &s.equip_stock
    } else if t & 0x800 != 0 {
        &s.magic_stock
    } else {
        &s.item_stock
    };
    lists.get(x.world.game.server.clamp(0, SERVERS as i32 - 1) as usize).cloned().unwrap_or_default()
}

/// An item code's category and id (`code >> 16`, `code & 0xffff`).
fn split(code: i32) -> (i32, i32) {
    (code >> 16, code & 0xffff)
}

/// `ccGetItemPrice(cat, id)` (main 0x00178eb0).
pub fn price(x: &Ctx, cat: i32, id: i32) -> i32 {
    x.texts.items.item(cat, id).map_or(0, |p| p.price)
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

fn cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// `DispMsg({0x100, name, comment}, 0)` for an item, or an empty one.
fn item_help(m: &mut MenuCtrl, x: &mut Ctx, code: i32) {
    let names = x.save.names();
    if code < 0 {
        m.msg.disp_msg(0x100, None, [None, None, None], &names);
        return;
    }
    let (cat, id) = split(code);
    let p = x.texts.items.item(cat, id).cloned().unwrap_or_default();
    m.msg.disp_msg(0x100, Some(&p.name), [Some(&p.comment[0]), Some(&p.comment[1]), Some(&p.comment[2])], &names);
}

/// `DispMsg({0x100, 0, lines}, 0)`.
fn help_lines(m: &mut MenuCtrl, x: &Ctx, l: &[Vec<u8>; 3]) {
    let names = x.save.names();
    m.msg.disp_msg(0x100, None, [Some(&l[0]), Some(&l[1]), Some(&l[2])], &names);
}

/// `OpenInfo(l0, l1, l2, 0, -1, -1)`.
fn open_info(m: &mut MenuCtrl, x: &Ctx, lines: [Option<&[u8]>; 3]) {
    let names = x.save.names();
    m.msg.open_info([lines[0], lines[1], lines[2], None], &names);
}

/// `ccMsg->Check(0)`.
fn check(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    push_msg_requests(x, req);
    r
}

/// The OK / Cancel dialog a page's question turns its list into (disp 11,
/// 6 cells, 2 rows, the cursor kept in `index`).
fn dialog(m: &mut MenuCtrl, x: &Ctx) {
    let i = idx(m);
    let l = &mut m.lists[i];
    l.disp = 11;
    l.x = 6;
    l.y = 2;
    l.index = l.select;
    l.select = 0;
    let d = x.texts.dialog_default.clone();
    extract_menu(m, d);
    cursors(m);
}

/// `BuyMenu`.
pub fn buy_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let stock = stock(m, x);
    let at = |k: i16| stock.get(k.max(0) as usize).copied().unwrap_or(-1);
    if m.proccess == 0 {
        m.exception_disp = 1;
        m.lists[i].disp = 4;
        m.menu_status = 1;
        m.bg_status = 1;
        let l = &mut m.lists[i];
        l.x = 18;
        l.my = 0;
        for &c in stock.iter().take(STOCK) {
            if c <= 0 {
                break;
            }
            l.my += 1;
        }
        l.y = 8;
        if l.my >= 9 {
            if l.my - 8 < l.dy {
                l.dy = l.my - 8;
            }
        } else {
            l.dy = 0;
        }
        if l.my - 1 < l.select {
            l.select = l.my - 1;
        }
        m.proccess += 1;
    }
    match m.proccess {
        1 => {
            m.select_scr(0, 0, 0, x);
            let code = at(m.lists[i].select);
            let (cat, id) = split(code);
            if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&cat) {
                x.se(SE_OK);
                m.item_num = code;
                m.change_menu_to(64);
                return Flow::Done;
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                let prev = x.target_prev.clone();
                x.change_target(prev.as_ref());
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Shop(Tail::BuyCancel));
            }
            if key == x.save.ok() && code >= 0 {
                m.exception_disp = 2;
                let n = bitem::get_item_num(&x.save.save, 0, cat, id);
                m.wait_count = if n >= 99 { 0 } else { 1 };
                cursors(m);
                m.proccess += 1;
            }
            item_help(m, x, code);
        }
        2 => {
            let code = at(m.lists[i].select);
            let (cat, id) = split(code);
            let room = 99 - bitem::get_item_num(&x.save.save, 0, cat, id);
            if room > 0 {
                count_keys(m, x, room);
            } else {
                m.wait_count = 0;
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                cursors(m);
                m.proccess = 0;
                return Flow::Done;
            }
            if key == x.save.ok() {
                cursors(m);
                m.menu_status = 3;
                let cost = price(x, cat, id).wrapping_mul(i32::from(m.wait_count));
                if gold(x) < cost {
                    return refuse(m, 2);
                }
                if bitem::get_item_slot(&x.save.save, 0) < 0 && bitem::get_item_num(&x.save.save, 0, cat, id) <= 0 {
                    return refuse(m, 3);
                }
                if room <= 0 {
                    return refuse(m, 4);
                }
                m.proccess += 1;
                return Flow::Done;
            }
            let h = x.texts.talk.shop.buy_help[0].clone();
            help_lines(m, x, &h);
        }
        3 if m.menu_status == 0 => {
            let code = at(m.lists[i].select);
            let (cat, id) = split(code);
            let h = x.texts.talk.shop.buy_help[1].clone();
            let mut s = h[0].clone();
            s.extend(piney_desktop::kanji::dec2sjis(i32::from(m.wait_count), 16, 0));
            s.extend_from_slice(&x.texts.talk.talk.green);
            s.extend(x.texts.items.item_name(cat, id));
            s.extend_from_slice(&h[1]);
            let cost = price(x, cat, id).wrapping_mul(i32::from(m.wait_count)).min(GOLD_MAX);
            s.extend(piney_desktop::kanji::dec2sjis(cost, 16, 0));
            s.extend_from_slice(&h[2]);
            open_info(m, x, [Some(&s), None, None]);
            m.exception_disp = 0;
            m.menu_status = 1;
            dialog(m, x);
            m.proccess += 1;
        }
        4 => {
            let code = at(m.lists[i].index);
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.menu_status = 3;
                m.msg.close();
                cursors(m);
                m.proccess += 1;
            } else if key == x.save.ok() {
                m.msg.close();
                cursors(m);
                if m.lists[i].select != 1 {
                    let (cat, id) = split(code);
                    let n = i32::from(m.wait_count);
                    let left = gold(x).wrapping_sub(price(x, cat, id).wrapping_mul(n)).min(GOLD_MAX);
                    x.save.save.set_i32(GOLD, left);
                    let order = x.texts.talk.shop.order;
                    bitem::add_item(&mut x.save.save, &order, 0, cat, id, n);
                }
                m.menu_status = 3;
                m.proccess += 1;
            }
        }
        5 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            cursors(m);
            m.proccess = 0;
        }
        10 if m.menu_status == 0 => {
            let k = m.talk.temp[0].clamp(0, 4) as usize;
            let h = x.texts.talk.shop.buy_help[k].clone();
            if k == 2 {
                open_info(m, x, [Some(&h[0]), None, None]);
            } else {
                open_info(m, x, [Some(&h[0]), Some(&h[1]), Some(&h[2])]);
            }
            m.proccess += 1;
        }
        11 => {
            if check(m, x) != 0 {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        12 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 8 {
                m.proccess = 0;
            }
        }
        _ => {}
    }
    Flow::Done
}

/// A refusal: its help in `temp[0]`, proccess 10.
fn refuse(m: &mut MenuCtrl, k: i32) -> Flow {
    m.talk.temp[0] = k;
    m.proccess = 10;
    Flow::Done
}

/// The count (`waitCount`, 1 .. `room`): up and down by one, left up and
/// right down by ten.
fn count_keys(m: &mut MenuCtrl, x: &mut Ctx, room: i32) {
    let rep = x.pad.repeat.bits();
    let room = room as i16;
    if rep & 0x1000 != 0 {
        m.wait_count += 1;
        if room < m.wait_count {
            m.wait_count = room;
        } else {
            x.se(SE_MOVE);
        }
    } else if rep & 0x4000 != 0 {
        m.wait_count -= 1;
        if m.wait_count > 0 {
            x.se(SE_MOVE);
        } else {
            m.wait_count = 1;
        }
    } else if rep & 0x8000 != 0 {
        if m.wait_count < room {
            m.wait_count += 10;
            x.se(SE_MOVE);
            if room < m.wait_count {
                m.wait_count = room;
            }
        } else {
            m.wait_count = room;
        }
    } else if rep & 0x2000 != 0 {
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
}

/// How many entries of a list are held (category and id set, a count
/// above 0).
fn held(list: impl Iterator<Item = Item>) -> i32 {
    list.filter(|it| it.cat >= 0 && it.id >= 0 && it.num > 0).count() as i32
}

/// The counts at the page's top: carried of 40, stored of 99.
fn counts(m: &mut MenuCtrl, x: &Ctx) {
    let slash = x.texts.slash.clone();
    let carried = held((0..items::ITEMS).map(|k| items::save_item(x.save, 0, k)));
    m.font.dx = 87.0;
    m.font.dy = 56.0;
    make_num(&mut m.font, 2, carried);
    m.font.make_str(&slash);
    make_num(&mut m.font, 2, 40);
    let stored = held((0..99).map(|k| items::pl_item(x.save, k)));
    m.font.dx = 159.0;
    m.font.dy = 56.0;
    make_num(&mut m.font, 2, stored);
    m.font.make_str(&slash);
    make_num(&mut m.font, 2, 99);
}

/// `BuyMenuDisp`.
pub fn buy_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let prev = l.prev.clamp(0, 88) as usize;
    let shop_type = m.lists[prev].index;
    let a = m.alpha;
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    counts(m, x);
    let stock = stock(m, x);
    let at = |k: i32| stock.get(k.max(0) as usize).copied().unwrap_or(-1);
    // The rows: the stock's names from dy, y at most.
    let mut buf = Vec::new();
    let mut rows = 0;
    let mut s1 = i32::from(l.dy);
    while s1 < i32::from(l.my) {
        let code = at(s1);
        if code < 0 {
            str_cat(&mut buf, b"", 16);
        } else {
            let (cat, id) = split(code);
            str_cat(&mut buf, &x.texts.items.item_name(cat, id), 16);
        }
        rows += 1;
        if rows >= i32::from(l.y) {
            break;
        }
        s1 += 1;
    }
    m.setting_text[0] = buf;
    set_type(&mut m.win, 1);
    let c = if m.exception_disp == 1 { 7 } else { 0 };
    m.win.set_colour(c);
    m.kanji.set_colour(c);
    m.win.set_alpha(a);
    m.kanji.set_alpha(a);
    m.win.dx = 39.0;
    m.win.dy = 96.0;
    disp_square_tag(&mut m.win, i32::from(l.x), i32::from(l.y), 0, 1, 8);
    if l.y < l.my {
        m.win.dx = 39.0;
        m.win.dy = 96.0;
        disp_scroll_bar(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.dy), i32::from(l.my));
    }
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    m.kanji.dx = 71.0;
    m.kanji.dy = 84.0;
    m.kanji.make_packet(0);
    let w = if l.y < l.my { i32::from(l.x) - 1 } else { i32::from(l.x) };
    let df = if m.exception_disp == 2 { 2 } else { 3 };
    let sn = i32::from(l.select) + (i32::from(l.page) << 8);
    let fr = x.frame_rate;
    m.cursor.disp(&mut m.win, 39.0, from_int((i32::from(l.select) - i32::from(l.dy)) * 20 + 112), w, sn, a, df, fr);
    let tag = if shop_type & 0x100 != 0 {
        4
    } else if shop_type & 0x800 != 0 {
        1
    } else {
        0
    };
    let t = x.texts.item_tags.get(tag).cloned().unwrap_or_default();
    extract_menu(m, t);
    // The stock's rows: price, icon, name.
    font_type(&mut m.font, 1);
    m.font.shadow = true;
    m.item_icon.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    set_clm(&mut m.setting[0], 16, 0, 0, 1);
    let gp = x.texts.gp.clone();
    for r in 0..i32::from(l.y) {
        let code = at(i32::from(l.dy) + r);
        if code < 0 {
            continue;
        }
        let c = if m.exception_disp == 2 { 0 } else { 7 };
        m.font.set_colour(c);
        m.item_icon.set_colour(c);
        m.setting[0].set_colour(c);
        let (cat, id) = split(code);
        m.font.set_alpha(a);
        m.font.dx = 187.0;
        m.font.dy = from_int(r * 20 + 112);
        make_num(&mut m.font, 7, price(x, cat, id));
        m.font.make_str(&gp);
        m.item_icon.set_alpha(a);
        m.item_icon.dx = 51.0;
        m.item_icon.dy = from_int(r * 20 + 110);
        let icon = x.texts.items.item_icon(cat);
        m.item_icon.make_packet(icon);
        m.setting[0].set_alpha(a);
        m.setting[0].dx = 67.0;
        m.setting[0].dy = from_int(r * 20 + 112);
        m.setting[0].make_packet(r);
    }
    money(m, x);
    if m.exception_disp != 2 {
        return;
    }
    // The count's window beside the row.
    let s2 = i32::from(l.x) * 14 + 87;
    let s3 = (i32::from(l.select) - i32::from(l.dy)) * 20 + 112;
    m.win_pr.dx = from_int(s2);
    m.win_pr.dy = from_int(s3 - 16);
    disp_square_w2(&mut m.win_pr, 7, 2, 1, None);
    m.cursor_pr.disp(&mut m.win_pr, from_int(s2), from_int(s3), 10, i32::from(l.select), a, 6, fr);
    let (cat, id) = split(at(i32::from(l.select)));
    let n = i32::from(m.wait_count);
    let mut cost = price(x, cat, id).wrapping_mul(n);
    m.font.set_colour(if gold(x) < cost || n == 0 { 23 } else { 22 });
    cost = cost.min(GOLD_MAX);
    m.font.set_alpha(a);
    m.font.dx = from_int(s2 + 8);
    m.font.dy = from_int(s3);
    make_num(&mut m.font, 7, cost);
    m.font.make_str(&gp);
    m.font.dx = from_int(s2 + 134);
    m.font.dy = from_int(s3);
    make_num(&mut m.font, 2, n);
    m.win.dx = from_int(s2);
    m.win.dy = from_int(s3 + 32);
    disp_square_w2(&mut m.win, 7, 2, 0, None);
    m.kanji_pr.dx = from_int(s2 + 28);
    m.kanji_pr.dy = from_int(s3 + 40);
    m.kanji_pr.make_packet(1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.dx = from_int(s2 + 134);
    m.font.dy = from_int(s3 + 40);
    make_num(&mut m.font, 2, bitem::get_item_num(&x.save.save, 0, cat, id));
}

/// `SellMenu`: the Items pages of the player's bag, sold at half price
/// (`price / 2`, the gold at most 9999999). The steps are in
/// docs/engine/field-ui.md (Sell).
pub fn sell_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
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
            let key = pushed_key(x);
            if key == x.save.cancel() {
                cursors(m);
                let prev = x.target_prev.clone();
                x.change_target(prev.as_ref());
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Shop(Tail::SellCancel));
            }
            if key == x.save.ok() {
                cursors(m);
                if it.cat >= 0 {
                    m.exception_disp = 2;
                    m.wait_count = 1;
                    m.proccess += 1;
                }
            }
            let code = if it.cat >= 0 && it.id >= 0 { it.code() } else { -1 };
            item_help(m, x, code);
        }
        2 => {
            let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            count_keys(m, x, i32::from(it.num));
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.exception_disp = 1;
                cursors(m);
                m.proccess = 1;
            } else if key == x.save.ok() {
                m.menu_status = 3;
                cursors(m);
                m.proccess += 1;
            }
            let h = x.texts.talk.shop.sell_help[0].clone();
            help_lines(m, x, &h);
        }
        3 if m.menu_status == 0 => {
            let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let (cat, id) = (i32::from(it.cat), i32::from(it.id));
            let h = x.texts.talk.shop.sell_help[1].clone();
            let mut s = h[0].clone();
            s.extend(piney_desktop::kanji::dec2sjis(i32::from(m.wait_count), 16, 0));
            s.extend_from_slice(&x.texts.talk.talk.green);
            s.extend(x.texts.items.item_name(cat, id));
            s.extend_from_slice(&h[1]);
            let pay = (i32::from(m.wait_count).wrapping_mul(price(x, cat, id) / 2)).min(GOLD_MAX);
            s.extend(piney_desktop::kanji::dec2sjis(pay, 16, 0));
            s.extend_from_slice(&h[2]);
            open_info(m, x, [Some(&s), None, None]);
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
            cursors(m);
            m.proccess += 1;
        }
        4 => {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.menu_status = 3;
                m.msg.close();
                cursors(m);
                m.proccess += 1;
            } else if key == x.save.ok() {
                m.msg.close();
                cursors(m);
                if m.lists[i].select != 1 {
                    let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
                    let it = list.get(m.lists[i].index.max(0) as usize).copied().unwrap_or(Item::NONE);
                    let (cat, id) = (i32::from(it.cat), i32::from(it.id));
                    let n = i32::from(m.wait_count);
                    let got = gold(x).wrapping_add(n.wrapping_mul(price(x, cat, id) / 2)).min(GOLD_MAX);
                    x.save.save.set_i32(GOLD, got);
                    bitem::del_item(&mut x.save.save, 0, cat, id, n);
                }
                m.menu_status = 3;
                m.proccess += 1;
            }
        }
        5 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            l.x = l.sx;
            l.y = l.sy;
            l.disp = 4;
            m.exception_disp = 1;
            m.menu_status = 1;
            cursors(m);
            m.proccess = 1;
        }
        _ => {}
    }
    Flow::Done
}

/// Which of Elf's Haven's pages: Store Items (54, the bag into the
/// storage) or Withdraw Items (55, back).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Haven {
    Store,
    Withdraw,
}

/// `SetPlItemList(page, list, 0)` (gcmn 0x00527740): the storage's items
/// on a page (its table at gcmn 0x006516b0: -1 the usable items, -2 the
/// equipment, else a category).
pub fn pl_item_list(x: &Ctx, page: i32) -> Vec<Item> {
    let filter = x.texts.talk.shop.pl_pages.get(page.clamp(0, 4) as usize).copied().unwrap_or(-1);
    let mut out = vec![Item::NONE; 99];
    let mut n = 0usize;
    for k in 0..99 {
        let it = items::pl_item(x.save, k);
        let c = i32::from(it.cat);
        let take = match filter {
            -1 => c == 10 || c == 13,
            -2 => (0..10).contains(&c),
            f => c == f,
        };
        if take {
            out[n] = it;
            n += 1;
        }
    }
    out
}

/// The page's list (`SetItemList(0, page, ..)` or `SetPlItemList`), with
/// the count fitted into the list when `fit`.
fn haven_list(m: &mut MenuCtrl, x: &Ctx, h: Haven, fit: bool) -> Vec<Item> {
    let i = idx(m);
    let page = i32::from(m.lists[i].page);
    let list = match h {
        Haven::Store => items::item_list(&x.texts.items, x.save, 0, page).to_vec(),
        Haven::Withdraw => pl_item_list(x, page),
    };
    if fit {
        let n = list.iter().filter(|it| it.cat >= 0).count() as i16;
        items::fit_list(&mut m.lists[i], n);
    }
    list
}

/// How many the other side holds (the storage for Store, the bag for
/// Withdraw), and whether it has a free slot.
fn other_num(x: &Ctx, h: Haven, cat: i32, id: i32) -> i32 {
    match h {
        Haven::Store => bitem::get_pl_item_num(&x.save.save, cat, id),
        Haven::Withdraw => bitem::get_item_num(&x.save.save, 0, cat, id),
    }
}

fn other_slot(x: &Ctx, h: Haven) -> i32 {
    match h {
        Haven::Store => bitem::get_pl_item_slot(&x.save.save),
        Haven::Withdraw => bitem::get_item_slot(&x.save.save, 0),
    }
}

/// `ItemDepositMenu` and `ItemDrawMenu`: an item moved between the bag
/// and Elf's Haven's storage (99 of a kind at most on either side), the
/// same code with the two sides swapped. The rows the other side cannot
/// take are greyed (`reverseHead`); the move adds first, then deletes. The
/// steps are in docs/engine/field-ui.md (Elf's Haven).
fn haven_menu(m: &mut MenuCtrl, x: &mut Ctx, h: Haven) -> Flow {
    let i = idx(m);
    let help = match h {
        Haven::Store => x.texts.talk.shop.deposit_help.clone(),
        Haven::Withdraw => x.texts.talk.shop.draw_help.clone(),
    };
    match m.proccess {
        0 => {
            m.exception_disp = 1;
            m.menu_status = 1;
            m.bg_status = 1;
            let l = &mut m.lists[i];
            l.page_num = 5;
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            haven_list(m, x, h, true);
            m.proccess += 1;
        }
        1 => {
            let old = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = haven_list(m, x, h, true);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            m.reverse_head = 0;
            let (dy, y) = (m.lists[i].dy, m.lists[i].y);
            for s2 in dy..y + dy {
                let e = list.get(s2.max(0) as usize).copied().unwrap_or(Item::NONE);
                if e.cat < 0 || e.id < 0 || e.num <= 0 {
                    continue;
                }
                let n = other_num(x, h, i32::from(e.cat), i32::from(e.id));
                let full = if n > 0 { n >= 99 } else { other_slot(x, h) < 0 };
                if full {
                    m.reverse_head |= 1u32.wrapping_shl((s2 - dy) as u32);
                }
            }
            if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&it.cat) {
                x.se(SE_OK);
                m.item_num = (i32::from(it.cat) << 16) | i32::from(it.id);
                m.change_menu_to(64);
                return Flow::Done;
            }
            let key = pushed_key(x);
            if key == x.save.cancel() {
                cursors(m);
                let prev = x.target_prev.clone();
                x.change_target(prev.as_ref());
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Shop(Tail::SellCancel));
            }
            if key == x.save.ok() {
                cursors(m);
                if it.id >= 0 && it.num > 0 {
                    let (cat, id) = (i32::from(it.cat), i32::from(it.id));
                    if other_slot(x, h) < 0 && other_num(x, h, cat, id) <= 0 {
                        m.menu_status = 3;
                        m.wait_count = 2;
                        m.proccess = 10;
                        return Flow::Done;
                    }
                    if other_num(x, h, cat, id) >= 99 {
                        m.menu_status = 3;
                        m.wait_count = 3;
                        m.proccess = 10;
                        return Flow::Done;
                    }
                    m.exception_disp = 2;
                    m.wait_count = 1;
                    m.proccess += 1;
                }
            }
            let code = if it.cat >= 0 && it.id >= 0 { it.code() } else { -1 };
            item_help(m, x, code);
        }
        2 => {
            let list = haven_list(m, x, h, false);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let room = 99 - other_num(x, h, i32::from(it.cat), i32::from(it.id));
            let most = i32::from(it.num).min(room);
            haven_count_keys(m, x, most);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.exception_disp = 1;
                cursors(m);
                m.proccess = 1;
            } else if key == x.save.ok() {
                m.menu_status = 3;
                cursors(m);
                m.proccess += 1;
            }
            help_lines(m, x, &help[0]);
        }
        3 if m.menu_status == 0 => {
            let list = haven_list(m, x, h, false);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            let mut s = help[1][0].clone();
            s.extend(piney_desktop::kanji::dec2sjis(i32::from(m.wait_count), 16, 0));
            s.extend_from_slice(&x.texts.talk.talk.green);
            s.extend(x.texts.items.item_name(i32::from(it.cat), i32::from(it.id)));
            s.extend_from_slice(&x.texts.talk.talk.end);
            open_info(m, x, [Some(&s), None, None]);
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
            cursors(m);
            m.reverse_head = 0;
            m.proccess += 1;
        }
        4 => {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.menu_status = 3;
                m.msg.close();
                cursors(m);
                m.proccess += 1;
            } else if key == x.save.ok() {
                m.msg.close();
                cursors(m);
                if m.lists[i].select != 1 {
                    let list = haven_list(m, x, h, false);
                    let it = list.get(m.lists[i].index.max(0) as usize).copied().unwrap_or(Item::NONE);
                    let (cat, id, n) = (i32::from(it.cat), i32::from(it.id), i32::from(m.wait_count));
                    let order = x.texts.talk.shop.order;
                    let sv = &mut x.save.save;
                    match h {
                        Haven::Store => {
                            bitem::add_pl_item(sv, &order, cat, id, n);
                            bitem::del_item(sv, 0, cat, id, n);
                        }
                        Haven::Withdraw => {
                            bitem::add_item(sv, &order, 0, cat, id, n);
                            bitem::del_pl_item(sv, cat, id, n);
                        }
                    }
                }
                m.menu_status = 3;
                m.proccess += 1;
            }
        }
        5 if m.menu_status == 0 => {
            let l = &mut m.lists[i];
            l.select = l.index;
            l.x = l.sx;
            l.y = l.sy;
            l.disp = 4;
            m.exception_disp = 1;
            m.menu_status = 1;
            cursors(m);
            m.proccess = 1;
        }
        10 if m.menu_status == 0 => {
            let k = i32::from(m.wait_count).clamp(0, help.len() as i32 - 1) as usize;
            let l = help[k].clone();
            match h {
                Haven::Store => open_info(m, x, [Some(&l[0]), Some(&l[1]), None]),
                Haven::Withdraw => open_info(m, x, [Some(&l[0]), Some(&l[1]), Some(&l[2])]),
            }
            m.proccess += 1;
        }
        11 => {
            if check(m, x) != 0 {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        12 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 8 {
                m.proccess = 0;
            }
        }
        _ => {}
    }
    Flow::Done
}

/// Elf's Haven's count (0x00559f5c): up one while below `most`, down one
/// to 1, left up ten, right down ten.
fn haven_count_keys(m: &mut MenuCtrl, x: &mut Ctx, most: i32) {
    let rep = x.pad.repeat.bits();
    let most = most as i16;
    if rep & 0x1000 != 0 {
        if m.wait_count < most {
            x.se(SE_MOVE);
            m.wait_count += 1;
        } else {
            m.wait_count = most;
        }
    } else if rep & 0x4000 != 0 {
        if m.wait_count >= 2 {
            x.se(SE_MOVE);
            m.wait_count -= 1;
        } else {
            m.wait_count = 1;
        }
    } else if rep & 0x8000 != 0 {
        if m.wait_count < most {
            m.wait_count += 10;
            x.se(SE_MOVE);
            if most < m.wait_count {
                m.wait_count = most;
            }
        } else {
            m.wait_count = most;
        }
    } else if rep & 0x2000 != 0 {
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
}

/// `ItemDepositMenu` (gcmn 0x00559960, menu 54): Store Items.
pub fn item_deposit_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    haven_menu(m, x, Haven::Store)
}

/// `ItemDrawMenu` (gcmn 0x0055ac40, menu 55): Withdraw Items.
pub fn item_draw_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    haven_menu(m, x, Haven::Withdraw)
}

/// The count's three windows beside the row (0x0055a73c on): the count
/// (`shopStr` row `count_row`), then two holdings.
fn haven_counts(m: &mut MenuCtrl, x: &Ctx, rows: [(i32, i32); 3]) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let a = m.alpha;
    let s2 = i32::from(l.x) * 14 + 87;
    let s0 = (i32::from(l.select) - i32::from(l.dy)) * 20 + 112;
    m.win_pr.set_colour(7);
    m.win_pr.set_alpha(a);
    m.kanji_pr_text = x.texts.talk.shop.shop_str.clone();
    set_clm(&mut m.kanji_pr, 16, 0, 0, 1);
    m.kanji_pr.set_colour(7);
    m.kanji_pr.set_alpha(a);
    m.win_pr.dx = from_int(s2);
    m.win_pr.dy = from_int(s0 - 16);
    disp_square_w2(&mut m.win_pr, 7, 2, 1, None);
    let fr = x.frame_rate;
    m.cursor_pr.disp(&mut m.win_pr, from_int(s2), from_int(s0), 10, 0, a, 6, fr);
    for (k, &(row, v)) in rows.iter().enumerate() {
        let y = s0 + 32 * k as i32;
        if k == 1 {
            m.win_pr.set_colour(7);
            m.win_pr.set_alpha(a);
            m.font.set_colour(7);
            m.font.set_alpha(a);
        }
        if k > 0 {
            m.win_pr.dx = from_int(s2);
            m.win_pr.dy = from_int(y);
            disp_square_w2(&mut m.win_pr, 7, 2, 0, None);
        }
        let ky = if k == 0 { s0 } else { y + 8 };
        m.kanji_pr.dx = from_int(s2 + 12);
        m.kanji_pr.dy = from_int(ky);
        m.kanji_pr.make_packet(row);
        if k == 0 {
            m.font.set_colour(22);
            m.font.set_alpha(a);
        }
        m.font.dx = from_int(s2 + 136);
        m.font.dy = from_int(ky);
        make_num(&mut m.font, 2, v);
    }
}

/// `ItemDepositMenuDisp` (gcmn 0x0055a6f0): the Items page, and while
/// counting: Store (the count), In Storage, Possess.
pub fn item_deposit_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    crate::menus::item::item_menu_disp(m, x);
    if m.exception_disp != 2 {
        return;
    }
    let i = idx(m);
    let list = items::item_list(&x.texts.items, x.save, 0, i32::from(m.lists[i].page));
    let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
    let stored = bitem::get_pl_item_num(&x.save.save, i32::from(it.cat), i32::from(it.id));
    haven_counts(m, x, [(2, i32::from(m.wait_count)), (4, stored), (1, i32::from(it.num))]);
}

/// `ItemDrawMenuDisp` (gcmn 0x0055b9f0): the storage's page (as
/// `ItemMenuDisp` draws the bag's), and while counting: Withdraw (the
/// count), In Storage, Possess.
pub fn item_draw_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let a = m.alpha;
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    counts(m, x);
    let c = if m.exception_disp == 1 { 7 } else { 0 };
    for s in [&mut m.win, &mut m.kanji, &mut m.setting[0], &mut m.item_icon, &mut m.font] {
        s.set_colour(c);
    }
    m.win.set_alpha(a);
    let ka = m.kanji_alpha;
    m.kanji.set_alpha(ka);
    m.setting[0].set_alpha(ka);
    m.item_icon.set_alpha(ka);
    m.font.set_alpha(ka);
    set_type(&mut m.win, 1);
    m.item_icon.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    let page = m.lists[i].page;
    let tag = x.texts.item_tags.get(page.max(0) as usize).cloned().unwrap_or_default();
    crate::menus::skill::tab_frame(m, x, &tag, true);
    let list = pl_item_list(x, i32::from(page));
    if m.kanji_alpha > 0 {
        set_clm(&mut m.setting[0], 16, 0, 0, 1);
        set_clm(&mut m.setting[1], 16, 0, 0, 1);
        font_type(&mut m.font, 1);
        let dy = i32::from(m.lists[i].dy);
        let mut buf = Vec::new();
        let (mut row, mut kanji) = (0i32, 0usize);
        for (s2, &it) in list.iter().enumerate() {
            if it.cat < 0 || (s2 as i32) < dy {
                continue;
            }
            let name = x.texts.items.item_name(i32::from(it.cat), i32::from(it.id));
            str_cat(&mut buf, &name, 16);
            let y = (row + (kanji as i32) * 8) * 20 + 112;
            let greyed = m.reverse_head & 1u32.wrapping_shl((s2 as i32 - dy) as u32) != 0;
            let c = match (greyed, m.exception_disp == 1) {
                (true, true) => 0,
                (true, false) => 8,
                (false, true) => 7,
                (false, false) => 0,
            };
            m.setting[kanji].set_colour(c);
            m.item_icon.set_colour(c);
            m.font.set_colour(c);
            m.setting[kanji].set_alpha(ka);
            m.item_icon.set_alpha(ka);
            m.font.set_alpha(ka);
            m.item_icon.dx = 51.0;
            m.item_icon.dy = from_int(y - 2);
            let icon = x.texts.items.item_icon(i32::from(it.cat));
            m.item_icon.make_packet(icon);
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
    if m.exception_disp != 2 {
        return;
    }
    let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
    let held = bitem::get_item_num(&x.save.save, 0, i32::from(it.cat), i32::from(it.id));
    haven_counts(m, x, [(3, i32::from(m.wait_count)), (4, i32::from(it.num)), (1, held)]);
}

/// The money window under a page: the frame, "Money", the player's gold.
fn money(m: &mut MenuCtrl, x: &Ctx) {
    let a = m.alpha;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = 39.0;
    m.win.dy = 282.0;
    disp_square_w2(&mut m.win, 3, 7, 1, None);
    m.kanji_pr_text = x.texts.talk.shop.shop_str.clone();
    set_clm(&mut m.kanji_pr, 16, 0, 0, 1);
    m.kanji_pr.set_colour(7);
    m.kanji_pr.set_alpha(a);
    m.kanji_pr.dx = 51.0;
    m.kanji_pr.dy = 298.0;
    m.kanji_pr.make_packet(0);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.dx = 107.0;
    m.font.dy = 298.0;
    make_num(&mut m.font, 7, gold(x));
    let gp = x.texts.gp.clone();
    m.font.make_str(&gp);
}

/// `SellMenuDisp`: the Items page (`ItemMenuDisp`), the money, and while
/// counting the price and count windows beside the row.
pub fn sell_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    crate::menus::item::item_menu_disp(m, x);
    let i = idx(m);
    let l = m.lists[i].clone();
    let list = items::item_list(&x.texts.items, x.save, 0, i32::from(l.page));
    let it = list.get(l.select.max(0) as usize).copied().unwrap_or(Item::NONE);
    money(m, x);
    if m.exception_disp != 2 {
        return;
    }
    let a = m.alpha;
    let s2 = i32::from(l.x) * 14 + 87;
    let s3 = (i32::from(l.select) - i32::from(l.dy)) * 20 + 112;
    m.win_pr.dx = from_int(s2);
    m.win_pr.dy = from_int(s3 - 16);
    disp_square_w2(&mut m.win_pr, 7, 2, 1, None);
    let fr = x.frame_rate;
    let sn = i32::from(l.select) - i32::from(l.dy);
    m.cursor_pr.disp(&mut m.win_pr, from_int(s2), from_int(s3), 10, sn, a, 6, fr);
    m.font.set_colour(22);
    m.font.set_alpha(a);
    m.font.dx = from_int(s2 + 8);
    m.font.dy = from_int(s3);
    let n = i32::from(m.wait_count);
    let pay = n.wrapping_mul(price(x, i32::from(it.cat), i32::from(it.id)) / 2).min(GOLD_MAX);
    make_num(&mut m.font, 7, pay);
    let gp = x.texts.gp.clone();
    m.font.make_str(&gp);
    m.font.dx = from_int(s2 + 134);
    m.font.dy = from_int(s3);
    make_num(&mut m.font, 2, n);
    m.win.dx = from_int(s2);
    m.win.dy = from_int(s3 + 32);
    disp_square_w2(&mut m.win, 7, 2, 0, None);
    m.kanji_pr.dx = from_int(s2 + 28);
    m.kanji_pr.dy = from_int(s3 + 40);
    m.kanji_pr.make_packet(1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.dx = from_int(s2 + 134);
    m.font.dy = from_int(s3 + 40);
    make_num(&mut m.font, 2, i32::from(it.num));
}
