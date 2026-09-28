//! CHAT's per-member menus: `ChatMenu1` (gcmn 0x0052a950, menu 71) the
//! member's own orders, `ChatMenu2` (0x0052ba80, 72) the skill it is to
//! use (drawn by `SkillMenuDisp`), `ChatMenu3` (0x0052c0b0, 73) whom on.
//! CHAT's Members page opens 71 on the `chatMember`th member; the order
//! goes out as `RequestChatCmd(member, chatAction, target, chatSkill)` with
//! the player's line in the balloon. The rows, warnings and candidates are
//! in docs/engine/field-ui.md (a member's orders).

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::items::{self, flat_dist};
use crate::menus::chat::shut;
use crate::menus::skill::set_skill_list_fit;
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::world::CharInfo;

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// The `chatMember`th filled slot of 1 and 2 and its index (0 or 1); not
/// found, the loop leaves slot 2 (filled or not) and index -1.
fn member_of(m: &MenuCtrl, x: &Ctx) -> (Option<CharInfo>, i32) {
    let mut n = 0;
    for slot in 1..3 {
        if let Some(c) = &x.world.party[slot] {
            if n == m.chat_member() {
                return (Some(c.clone()), slot as i32 - 1);
            }
            n += 1;
        }
    }
    (x.world.party[2].clone(), -1)
}

/// A piece of a text list, empty past its end.
fn piece(v: &[Vec<u8>], k: usize) -> Vec<u8> {
    v.get(k).cloned().unwrap_or_default()
}

/// Back to the list this one came from.
fn back(m: &mut MenuCtrl) {
    let prev = m.list().prev;
    m.back_to_prev(prev);
}

/// 71's warning once the menu's window has gone (proccess 10):
/// `deadInfo` alone, or both pieces of `chatMenuWarn[waitCount]` (the
/// second whatever `ccKanjiStrSeparate` finds past the first).
fn warn(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.menu_status != 0 {
        return;
    }
    let names = x.save.names();
    let w = m.wait_count;
    if w == 0 {
        let d = x.texts.dead_info.clone();
        m.msg.open_info([Some(&d), None, None, None], &names);
    } else {
        let l = x.texts.chat_warn.get(w.max(0) as usize).cloned().unwrap_or_default();
        m.msg.open_info([Some(&l[0]), Some(&l[1]), None, None], &names);
    }
    m.proccess += 1;
}

/// The message's Check: whether it closed.
fn checked(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let ok = x.save.ok();
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
    push_msg_requests(x, req);
    if r != 0 {
        m.msg.close();
    }
    r != 0
}

/// The cancel and OK of the menu's keys (their sounds), in the game's
/// order: cancel first.
fn keys(x: &mut Ctx) -> (bool, bool) {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        (true, false)
    } else if x.pushed_ok() {
        x.se(SE_OK);
        (false, true)
    } else {
        (false, false)
    }
}

/// `menuList[menu]`'s cursor to the top (select and `dy`), then
/// `ChangeMenu(menu)`.
fn open_list(m: &mut MenuCtrl, menu: i16) {
    let l = &mut m.lists[menu as usize];
    l.select = 0;
    l.dy = 0;
    m.change_menu_to(menu);
}

/// 71's rows by where the party is: `chatMenuStr` pieces.
fn rows(x: &Ctx) -> &'static [usize] {
    let g = &x.world.game;
    if g.area == 0 {
        &[18]
    } else if g.in_battle != 0 {
        &[13, 18, 1, 12, 14, 15]
    } else {
        &[13, 18, 1]
    }
}

/// 71's order under the cursor (`chatAction`), `None` past the rows.
fn action(x: &Ctx, sel: i16) -> Option<i16> {
    let g = &x.world.game;
    if g.area == 0 {
        return Some(20);
    }
    let table: &[i16] = if g.in_battle != 0 { &[5, 20, 16, 11, 3, 4] } else { &[5, 20, 16] };
    usize::try_from(sel).ok().and_then(|s| table.get(s).copied())
}

/// `ChatMenu1` (menu 71).
pub fn chat_menu1(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let (member, slot) = member_of(m, x);
    match m.proccess {
        0 => {
            let Some(c) = member else {
                back(m);
                return Flow::Done;
            };
            m.lists[i].disp = 7;
            m.menu_status = 1;
            let mut buf = Vec::new();
            for &k in rows(x) {
                str_cat(&mut buf, &piece(&x.texts.chat_str, k), 16);
            }
            let n = rows(x).len() as i16;
            m.lists[i].y = n;
            let dead = c.condition[0] != 0;
            let g = x.world.game;
            if g.area == 0 {
                if c.check_change_equip() != 0 {
                    m.reverse_head |= 1;
                }
            } else {
                if dead {
                    m.reverse_head |= if g.in_battle != 0 { 0xd } else { 5 };
                }
                if c.check_change_equip() != 0 {
                    m.reverse_head |= 2;
                }
            }
            extract_menu(m, buf);
            let l = &mut m.lists[i];
            if l.select >= l.y {
                l.select = l.y - 1;
            }
            if l.select < 0 {
                l.select = 0;
            }
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            // Select(0, 0, 1): a3 is still the 1 the dispatch compared.
            m.select(0, 0, true, x);
            let (cancel, ok) = keys(x);
            if cancel {
                back(m);
            } else if ok {
                let flow = order(m, x, member.as_ref(), slot);
                if let Some(f) = flow {
                    return f;
                }
            }
            help1(m, x);
            Flow::Done
        }
        10 => {
            warn(m, x);
            Flow::Done
        }
        11 => {
            if checked(m, x) {
                m.wait_count = 0;
                m.proccess += 1;
            }
            Flow::Done
        }
        12 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.proccess = 0;
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// OK on 71's row: a refusal (`Some(Done)`, no help), the menu shut
/// (`Some(Breathed)`), or `None` (the help after).
fn order(m: &mut MenuCtrl, x: &mut Ctx, member: Option<&CharInfo>, slot: i32) -> Option<Flow> {
    let i = idx(m);
    let sel = m.lists[i].select;
    let g = x.world.game;
    let dead = member.is_some_and(|c| c.condition[0] != 0);
    let refuse = |m: &mut MenuCtrl, w: i16| {
        m.menu_status = 3;
        m.wait_count = w;
        m.proccess = 10;
        Some(Flow::Done)
    };
    // Change Equipment: CheckChangeEquip, kept in chatEquipStatus[slot].
    let equip = |m: &mut MenuCtrl| -> Option<Flow> {
        let r = member.map_or(0, CharInfo::check_change_equip);
        if slot >= 0 {
            m.chat_mem[4 + slot as usize] = r;
        }
        match r {
            7 => refuse(m, 1),
            0 => None,
            _ => refuse(m, 2),
        }
    };
    let refused = if g.area == 0 {
        if sel == 0 { equip(m) } else { None }
    } else if g.in_battle != 0 {
        match sel {
            0 | 2 | 3 if dead => refuse(m, 0),
            1 => equip(m),
            _ => None,
        }
    } else {
        match sel {
            0 | 2 if dead => refuse(m, 0),
            1 => equip(m),
            _ => None,
        }
    };
    if refused.is_some() {
        return refused;
    }
    if let Some(a) = action(x, sel) {
        m.chat_mem[1] = a;
    }
    m.chat_mem[2] = -1;
    match m.chat_mem[1] {
        20 => {
            if let Some(c) = member_by_count(m, x) {
                m.set_equip_spc_num(c.id);
                m.change_menu_to(63);
                m.first_time = 1;
            }
            None
        }
        5 => {
            open_list(m, 72);
            None
        }
        11 => {
            open_list(m, 73);
            None
        }
        cmd @ (16 | 3 | 4) => {
            let mut line = Vec::new();
            if let Some(c) = member_by_count(m, x) {
                x.req.push(Request::ChatOrder { member: c.handle, cmd: i32::from(cmd), target: 0, skill: 0 });
                line.extend_from_slice(&c.name);
            }
            line.extend_from_slice(&piece(&x.texts.chat_action, 0));
            let k = match cmd {
                16 => 9,
                3 => 7,
                _ => 8,
            };
            line.extend_from_slice(&piece(&x.texts.chat_action, k));
            crate::chat_msg::open_player(m, x, line);
            Some(shut(m, x))
        }
        _ => None,
    }
}

/// The `chatMember`th filled slot of 1 and 2 (the orders' own loop: none
/// when it is not there).
fn member_by_count(m: &MenuCtrl, x: &Ctx) -> Option<CharInfo> {
    let mut n = 0;
    for slot in 1..3 {
        if let Some(c) = &x.world.party[slot] {
            if n == m.chat_member() {
                return Some(c.clone());
            }
            n += 1;
        }
    }
    None
}

/// 71's help: the row's `chatMenuHelp` (DispMsg).
pub fn help1(m: &mut MenuCtrl, x: &mut Ctx) {
    let sel = m.list().select;
    let g = x.world.game;
    let k: i32 = if g.area == 0 {
        17
    } else if g.in_battle != 0 {
        match sel {
            0 => 13,
            1 => 17,
            2 => 18,
            3 => 12,
            4 => 10,
            5 => 11,
            s => 27 + i32::from(s),
        }
    } else {
        match sel {
            0 => 13,
            2 => 18,
            _ => 17,
        }
    };
    disp_help(m, x, k);
}

/// `ccMsg->DispMsg` of `chatMenuHelp[k]`'s three lines.
fn disp_help(m: &mut MenuCtrl, x: &mut Ctx, k: i32) {
    let h = x.texts.chat_help.get(k.max(0) as usize).cloned().unwrap_or_default();
    let names = x.save.names();
    m.msg.disp_msg(0x100, None, [Some(&h[0]), Some(&h[1]), Some(&h[2])], &names);
}

/// `ChatMenu2` (menu 72).
pub fn chat_menu2(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let (member, _) = member_of(m, x);
    match m.proccess {
        0 => {
            let Some(c) = member else {
                back(m);
                return Flow::Done;
            };
            let pc = c.id.max(0) as usize;
            let known = (0..items::SKILL_SLOTS).filter(|&k| items::save_skill(x.save, pc, k) >= 0).count();
            if known == 0 {
                let w = x.texts.chat_warn[0].clone();
                let names = x.save.names();
                m.msg.open_info([Some(&w[0]), None, None, None], &names);
                m.proccess = 11;
                return Flow::Done;
            }
            m.exception_disp = 1;
            m.lists[i].page_num = 5;
            m.menu_status = 1;
            m.bg_status = 1;
            let l = &mut m.lists[i];
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            set_skill_list_fit(m, x, pc);
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            let c = member.unwrap_or_default();
            let pc = c.id.max(0) as usize;
            let old_page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old_page != m.lists[i].page {
                m.kanji_alpha = 0;
            }
            let list = set_skill_list_fit(m, x, pc);
            let (cancel, ok) = keys(x);
            let sel = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(-1);
            if cancel {
                back(m);
                return Flow::Done;
            }
            if ok && sel >= 0 {
                let cost = x.texts.items.skill(i32::from(sel)).map_or(0, |p| p.cost);
                if i32::from(c.sp) < cost {
                    m.menu_status = 3;
                    m.wait_count = 1;
                    m.proccess = 10;
                } else {
                    m.chat_mem[2] = sel;
                    m.change_menu_to(73);
                    return Flow::Done;
                }
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
            Flow::Done
        }
        10 => {
            if m.menu_status == 0 {
                let h = x.texts.skill_help.get(m.wait_count.clamp(0, 2) as usize).cloned().unwrap_or_default();
                let names = x.save.names();
                m.msg.open_info([Some(&h[0]), None, None, None], &names);
                m.proccess += 1;
            }
            Flow::Done
        }
        11 => {
            if checked(m, x) {
                m.wait_count = 0;
                m.proccess += 1;
            }
            Flow::Done
        }
        12 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                back(m);
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// 73's candidates: the party for a skill that goes to allies, else the
/// enemies in reach and in view.
fn candidates(m: &MenuCtrl, x: &Ctx) -> Vec<CharInfo> {
    let w = &x.world;
    let skill = i32::from(m.chat_mem[2]);
    let mut out = Vec::new();
    let ally = skill >= 0 && x.texts.items.skill(skill).is_some_and(|p| p.target_type & 3 != 0);
    if ally {
        for c in w.party.iter().flatten() {
            let d = c.condition[0];
            let take = if skill == 180 { d != 0 && d != 5 } else { d == 0 };
            if take {
                out.push(c.clone());
            }
        }
        return out;
    }
    for c in &w.sorted {
        if piney_desktop::eef::le(c.cmnd_dist, 2200.0) && c.is(0xe0) && c.condition[0] == 0 && c.in_view {
            out.push(c.clone());
            if out.len() >= 16 {
                break;
            }
        }
    }
    out
}

/// An area skill's other targets around the target (`targetRange` plus
/// each one's width).
fn sub_targets(m: &mut MenuCtrl, x: &Ctx) {
    let Some(p) = x.texts.items.skill(i32::from(m.chat_mem[2])).cloned() else { return };
    if p.kind & 0x6000 == 0 {
        return;
    }
    let Some(t) = x.target.clone() else { return };
    let chain = if t.is(6) {
        &x.world.pc_chain
    } else if t.is(0xe0) {
        &x.world.ene_chain
    } else {
        &x.world.obj_chain
    };
    let mut n = 0;
    for c in chain {
        if c.handle == t.handle || c.condition[0] != 0 {
            continue;
        }
        let d = flat_dist(c.pos_p, t.pos_p);
        if piney_desktop::eef::le(d, piney_desktop::eef::add(p.target_range, c.width)) {
            m.sub_target[n] = c.handle;
            n += 1;
            if n >= 16 {
                break;
            }
        }
    }
}

/// `ChatMenu3` (menu 73).
pub fn chat_menu3(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let (member, _) = member_of(m, x);
    let cands = candidates(m, x);
    match m.proccess {
        0 => {
            let l = &mut m.lists[i];
            l.disp = 4;
            l.select = 0;
            l.dy = 0;
            l.my = cands.len() as i16;
            l.y = 0;
            if l.my <= 0 {
                let names = x.save.names();
                let w = x.texts.target_warn.clone();
                m.msg.open_info([Some(&w), None, None, None], &names);
                m.proccess = 10;
                return Flow::Done;
            }
            l.y = l.my.min(8);
            l.disp = 6;
            m.menu_status = 1;
            m.bg_status = 3;
            m.proccess += 1;
        }
        1 => {}
        10 => {
            if checked(m, x) {
                back(m);
            }
            return Flow::Done;
        }
        _ => return Flow::Done,
    }
    let l = m.lists[i].clone();
    let mut buf = Vec::new();
    for (k, c) in cands.iter().enumerate() {
        let k = k as i16;
        if k < l.dy {
            continue;
        }
        str_cat(&mut buf, &c.name, 16);
        if k - l.dy >= l.y - 1 {
            break;
        }
    }
    extract_menu(m, buf);
    m.select_scr(0, 0, 0, x);
    let sel = m.lists[i].select.max(0) as usize;
    x.change_target(cands.get(sel));
    sub_targets(m, x);
    let (cancel, ok) = keys(x);
    if cancel {
        x.change_target(None);
        m.bg_status = 1;
        back(m);
        return Flow::Done;
    }
    if ok {
        let target = cands.get(sel).cloned();
        let t = &x.texts;
        let mut line = Vec::new();
        if let Some(c) = &member {
            x.req.push(Request::ChatOrder {
                member: c.handle,
                cmd: i32::from(m.chat_mem[1]),
                target: target.as_ref().map_or(0, |t| t.handle),
                skill: i32::from(m.chat_mem[2]),
            });
            line.extend_from_slice(&c.name);
        }
        line.extend_from_slice(&piece(&t.chat_action, 0));
        let tname = target.as_ref().map(|c| c.name.clone()).unwrap_or_default();
        match m.chat_mem[1] {
            11 => {
                line.extend_from_slice(&piece(&t.chat_action, 1));
                line.extend_from_slice(&tname);
                line.extend_from_slice(&piece(&t.chat_action, 10));
                crate::chat_msg::open_player(m, x, line);
            }
            5 => {
                let skill = t.items.skill(i32::from(m.chat_mem[2])).map(|p| p.name.clone()).unwrap_or_default();
                let player = x.world.party[0].as_ref().map(|c| c.handle);
                let th = target.as_ref().map(|c| c.handle);
                line.extend_from_slice(&piece(&t.chat_action, 4));
                line.extend_from_slice(&skill);
                if th.is_some() && th == player {
                    line.extend_from_slice(&piece(&t.chat_action, 10));
                } else {
                    line.extend_from_slice(&piece(&t.chat_action, 3));
                    if th.is_some() && th == member.as_ref().map(|c| c.handle) {
                        line.extend_from_slice(&piece(&t.chat_action, 2));
                    } else {
                        line.extend_from_slice(&tname);
                    }
                    line.extend_from_slice(&piece(&t.chat_action, 10));
                }
                crate::chat_msg::open_player(m, x, line);
            }
            _ => {}
        }
        x.change_target(None);
        return shut(m, x);
    }
    help3(m, x);
    Flow::Done
}

/// 73's help while its `proccess` is 1: `chatMenuHelp` 12, or 13 with a
/// skill.
pub fn help3(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.proccess != 1 {
        return;
    }
    let k = if m.chat_mem[2] >= 0 { 13 } else { 12 };
    disp_help(m, x, k);
}
