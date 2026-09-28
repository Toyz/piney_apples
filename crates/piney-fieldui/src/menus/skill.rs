//! `SkillMenu` (gcmn 0x0052cae0, menu 4) and `SkillMenuDisp` (0x0052d180,
//! menus 4, 72 and 81): the Skills list, a page each for attack, magic,
//! recovery, strengthen, weaken and (with the bracelet) Data Drain.
//!
//! ```text
//! proccess 0  target fixed and dropped; the page (6 with the bracelet);
//!             the drain gauge on the Data Drain page; the dim in
//! proccess 1  SelectScr over the pages; cancel (19) back to PERSONAL;
//!             OK (18) on a skill: in town "no skills in towns" (help 0),
//!             short of SP (help 1), Data Drain while the events hold
//!             operation 17 or eventStatus[40] (help 2), else TARGET (65);
//!             otherwise the skill's name and help (DispMsg)
//! proccess 2  the window gone: the help as an information window
//! proccess 3  until Check: Close
//! proccess 4  8 frames, then the list again
//! ```

use piney_desktop::eef::from_int;

use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::items::{self, PLCOL, SKILL_SLOTS};
use crate::menus::check_operate;
use crate::menus::system::{push_msg_requests, str_cat};
use crate::spr::{font_type, make_num, set_clm};
use crate::window::{disp_button, disp_scroll_bar, disp_square_tag, set_type};
use crate::world::CharInfo;

/// `volumeNum` is 1 on Infection: a blocked Data Drain says help 2.
const DRAIN_BLOCKED_HELP: i16 = 2;

/// The menu's list: `menuList[menu]`.
fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// `SetSkillList(0, page, list, 1)`: the list's count into `my`.
pub fn set_skill_list_fit(m: &mut MenuCtrl, x: &Ctx, pc: usize) -> [i16; SKILL_SLOTS] {
    let i = idx(m);
    let page = i32::from(m.lists[i].page);
    let list = items::skill_list(&x.texts.items, x.save, pc, page);
    let n = items::skill_count(&x.texts.items, x.save, pc, page);
    items::fit_list(&mut m.lists[i], n);
    list
}

fn drain_page_status(m: &mut MenuCtrl) {
    m.drain_status = if m.lists[idx(m)].page == 5 { 1 } else { 3 };
}

pub fn skill_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(crate::Request::TargetFix(true));
            x.change_target(None);
            m.exception_disp = 1;
            if x.save.save.u8(PLCOL) == 0 {
                m.lists[i].page_num = 5;
            } else {
                m.lists[i].page_num = 6;
                drain_page_status(m);
            }
            m.bg_status = 1;
            let l = &mut m.lists[i];
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            set_skill_list_fit(m, x, 0);
            m.proccess += 1;
        }
        1 => {
            let old_page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old_page != m.lists[i].page {
                m.kanji_alpha = -24;
                drain_page_status(m);
            }
            let list = set_skill_list_fit(m, x, 0);
            let cancel = x.pushed_cancel();
            let ok = !cancel && x.pushed_ok();
            if cancel {
                x.se(SE_BACK);
            } else if ok {
                x.se(SE_OK);
            }
            let sel = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(-1);
            if cancel {
                m.drain_status = 3;
                let prev = m.lists[i].prev;
                m.back_to_prev(prev);
                return Flow::Done;
            }
            if ok && sel >= 0 {
                let refuse = |m: &mut MenuCtrl, wc: i16| {
                    m.menu_status = 3;
                    m.drain_status = 3;
                    m.wait_count = wc;
                    m.proccess += 1;
                };
                if x.world.game.area == 0 {
                    refuse(m, 0);
                    return Flow::Done;
                }
                let cost = x.texts.items.skill(i32::from(sel)).map_or(0, |p| p.cost);
                let sp = x.world.player().map_or(0, |p| p.sp);
                if i32::from(sp) < cost {
                    refuse(m, 1);
                    return Flow::Done;
                }
                if (2..=5).contains(&sel) && (!check_operate(x, 17, 0) || x.save.save.u8(items::EVENT_STATUS + 40) != 0)
                {
                    refuse(m, DRAIN_BLOCKED_HELP);
                    return Flow::Done;
                }
                m.change_menu_to(65);
                return Flow::Done;
            }
            let names = x.save.names();
            match x.texts.items.skill(i32::from(sel)).filter(|_| sel >= 0) {
                Some(p) => {
                    let p = p.clone();
                    m.msg.disp_msg(
                        0x100,
                        Some(&p.name),
                        [Some(&p.help[0]), Some(&p.help[1]), Some(&p.help[2])],
                        &names,
                    );
                }
                None => m.msg.disp_msg(0x100, None, [None, None, None], &names),
            }
        }
        2 => {
            if m.menu_status == 0 {
                let wc = m.wait_count.clamp(0, 2) as usize;
                let h = x.texts.skill_help[wc].clone();
                let names = x.save.names();
                let second = if m.wait_count == 2 { Some(&h[1][..]) } else { None };
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
                drain_page_status(m);
                m.menu_status = 1;
                m.exception_disp = 1;
                m.proccess = 1;
            }
        }
        _ => {}
    }
    Flow::Done
}

/// The member a chat menu (72) is about: the `chatMember`th of party
/// slots 1 and 2 that are filled.
pub fn chat_member(m: &MenuCtrl, x: &Ctx) -> Option<CharInfo> {
    let mut n = 0;
    for slot in 1..3 {
        if let Some(c) = &x.world.party[slot] {
            if n == m.chat_member() {
                return Some(c.clone());
            }
            n += 1;
        }
    }
    // The loop leaves the last filled slot's character.
    x.world.party[1..].iter().rev().flatten().next().cloned()
}

/// Colour 7 while the page is live (`exceptionDisp` 1), else 0.
fn page_colour(m: &MenuCtrl) -> usize {
    if m.exception_disp == 1 { 7 } else { 0 }
}

/// The tabs, the scroll bar, the L1 / R1 buttons and the cursor the item
/// and skill pages share.
pub fn tab_frame(m: &mut MenuCtrl, x: &Ctx, tag: &[u8], item_page: bool) {
    let i = idx(m);
    let l = m.lists[i].clone();
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
    m.win.cx = 0.0;
    m.win.cy = 0.0;
    if m.kanji_alpha > 0 {
        set_clm(&mut m.kanji, 16, 0, 0, 1);
        m.kanji.dx = from_int((i32::from(l.page) + 2) * 14 + 39);
        m.kanji.dy = 84.0;
        m.kanji.make_packet(0);
        crate::menus::system::extract_menu(m, tag.to_vec());
    }
    let w = if l.y < l.my { i32::from(l.x) - 1 } else { i32::from(l.x) };
    let df = if m.exception_disp == 2 { 2 } else { 3 };
    let sn = if item_page {
        i32::from(l.select) + (i32::from(l.page) << 8)
    } else {
        i32::from(l.select) + i32::from(l.page) * 20
    };
    let (a, fr) = (m.alpha, x.frame_rate);
    m.cursor.disp(&mut m.win, 39.0, from_int((i32::from(l.select) - i32::from(l.dy)) * 20 + 112), w, sn, a, df, fr);
}

/// `SkillMenuDisp`.
pub fn skill_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let who = if m.menu == 72 { chat_member(m, x) } else { x.world.player().cloned() };
    set_type(&mut m.win, 1);
    let c = page_colour(m);
    m.win.set_colour(c);
    m.kanji.set_colour(c);
    m.setting[0].set_colour(c);
    m.font.set_colour(c);
    m.win.set_alpha(m.alpha);
    m.kanji.set_alpha(m.kanji_alpha);
    m.setting[0].set_alpha(m.kanji_alpha);
    m.font.set_alpha(m.kanji_alpha);
    let page = m.lists[i].page;
    let tag = x.texts.skill_tags.get(page.max(0) as usize).cloned().unwrap_or_default();
    tab_frame(m, x, &tag, false);
    let pc = who.as_ref().map_or(0, |c| c.id.max(0) as usize).min(17);
    let list = items::skill_list(&x.texts.items, x.save, pc, i32::from(page));
    if m.kanji_alpha <= 0 {
        return;
    }
    set_clm(&mut m.setting[0], 16, 0, 0, 1);
    set_clm(&mut m.setting[1], 16, 0, 0, 1);
    font_type(&mut m.font, 1);
    let dy = i32::from(m.lists[i].dy);
    let sp = who.as_ref().map_or(0, |c| c.sp);
    let mut buf = Vec::new();
    let (mut row, mut kanji) = (0i32, 0usize);
    for (s2, &skill) in list.iter().enumerate() {
        if skill < 0 || (s2 as i32) < dy {
            continue;
        }
        let Some(p) = x.texts.items.skill(i32::from(skill)).cloned() else { continue };
        str_cat(&mut buf, &p.name, 16);
        let bit = 1u32 << s2;
        if x.world.game.area == 0 || i32::from(sp) < p.cost {
            m.reverse_head |= bit;
        } else if items::check_skill_useful(
            &x.texts.items,
            &x.world,
            i32::from(skill),
            items::drain_off(x.texts.volume, &x.save.save),
        ) {
            m.reverse_head &= !bit;
        } else {
            m.reverse_head |= bit;
            if m.menu == 72 && skill != 180 {
                let pg = m.lists[i].page;
                if x.world.game.in_battle != 0 || !(pg == 0 || pg == 1 || pg == 4) {
                    m.reverse_head &= !bit;
                }
            }
        }
        let c = if m.reverse_head & bit != 0 { 0 } else { 7 };
        m.setting[kanji].set_colour(c);
        m.font.set_colour(c);
        m.setting[kanji].set_alpha(m.kanji_alpha);
        m.font.set_alpha(m.kanji_alpha);
        let y = from_int((row + (kanji as i32) * 8) * 20 + 112);
        m.setting[kanji].dx = 53.0;
        m.setting[kanji].dy = y;
        m.setting[kanji].make_packet(row);
        m.font.dx = 221.0;
        m.font.dy = y;
        make_num(&mut m.font, 3, p.cost);
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
