//! `TargetMenu` (gcmn 0x00531af0, menu 65): who a skill or an item goes
//! to, and its use. The candidates are rebuilt every frame (at most 8);
//! the cursor's is `cmndTarget` and an area skill gathers its sub-targets.
//! OK spends an item (`ccUseItemRequest`), marks a Data Drain's targets and
//! opens DATA DRAIN (66), or requests the skill; then the window shuts
//! while the skill runs, the target is dropped, six frames, and the menu
//! closes. The rules are in docs/engine/field-ui.md (TARGET).

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK, UseTail};
use crate::items::{self, Item, SkillParam, drain_blocked, flat_dist, in_reach};
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::world::CharInfo;

fn is_drain(s: i32) -> bool {
    (2..=5).contains(&s)
}

/// What the previous list had under its cursor.
struct Use {
    /// The skill (-1 none) and its row.
    skill: i32,
    param: Option<SkillParam>,
    /// The item (from Items).
    item: Option<Item>,
    /// `targetType` (0 when no skill decides it).
    target_type: i64,
}

fn what(m: &MenuCtrl, x: &Ctx) -> Use {
    let prev = m.list().prev;
    let pl = &m.lists[prev.clamp(0, 88) as usize];
    let mut u = Use { skill: -1, param: None, item: None, target_type: 0 };
    match prev {
        5 => {
            let list = items::item_list(&x.texts.items, x.save, 0, i32::from(pl.page));
            let it = list.get(pl.select.max(0) as usize).copied().unwrap_or(Item::NONE);
            u.item = Some(it);
            if (it.cat == 13 && it.id == 0) || (it.cat == 10 && it.id >= 18) {
                return u;
            }
            let s = x.texts.items.item(i32::from(it.cat), i32::from(it.id)).map_or(-1, |p| p.skill);
            u.skill = s;
            if s >= 0 {
                u.param = x.texts.items.skill(s).cloned();
            }
        }
        4 => {
            let list = items::skill_list(&x.texts.items, x.save, 0, i32::from(pl.page));
            let s = list.get(pl.select.max(0) as usize).copied().unwrap_or(-1);
            u.skill = i32::from(s);
            u.param = x.texts.items.skill(i32::from(s)).cloned();
        }
        _ => {}
    }
    if let Some(p) = &u.param {
        u.target_type = i64::from(p.target_type as i32);
    }
    u
}

/// The candidates, in order.
fn candidates(u: &Use, x: &Ctx) -> Vec<CharInfo> {
    let w = &x.world;
    let mut out: Vec<CharInfo> = Vec::new();
    if let Some(it) = u.item {
        if it.cat == 13 && it.id == 0 {
            for c in &w.sorted {
                if out.len() >= 8 {
                    break;
                }
                if piney_desktop::eef::le(c.cmnd_dist, 7000.0) && c.is(0x28000) && c.in_view {
                    out.push(c.clone());
                }
            }
            return out;
        }
        if it.cat == 10 && it.id >= 18 {
            out.extend(w.party.iter().flatten().filter(|c| c.condition[0] == 0).cloned());
            return out;
        }
    }
    let Some(p) = &u.param else { return out };
    let tt = p.target_type;
    if tt & 3 != 0 {
        for c in w.party.iter().flatten() {
            let d = c.condition[0];
            let take = if u.skill == 180 { d != 0 && d != 5 } else { d == 0 };
            if take {
                out.push(c.clone());
            }
        }
    }
    for c in &w.sorted {
        if out.len() >= 8 {
            break;
        }
        if in_reach(c, p.trigger_range)
            && tt & c.types != 0
            && c.types & 3 == 0
            && c.condition[0] == 0
            && c.in_view
            && !drain_blocked(u.skill, c)
        {
            out.push(c.clone());
        }
    }
    out
}

/// An area skill's other targets around the target (or the user).
fn sub_targets(m: &mut MenuCtrl, u: &Use, x: &Ctx) {
    let Some(p) = &u.param else { return };
    if !(p.kind & 0x6000 != 0 || u.skill == 3 || u.skill == 5) {
        return;
    }
    let Some(t) = x.target.clone() else { return };
    let centre = if p.kind & 0x2000 != 0 { x.world.player().map_or([0.0; 3], |c| c.pos_p) } else { t.pos_p };
    let chain = if t.is(6) {
        &x.world.pc_chain
    } else if t.is(224) {
        &x.world.ene_chain
    } else {
        &x.world.obj_chain
    };
    let mut n = 0;
    for c in chain {
        if c.handle == t.handle || c.condition[0] != 0 {
            continue;
        }
        if (u.skill == 3 || u.skill == 5) && (c.is(0x60) || c.is(0x80)) && c.pp == 0 {
            continue;
        }
        let d = flat_dist(x.texts.volume, c.pos_p, centre);
        if piney_desktop::eef::le(d, piney_desktop::eef::add(p.target_range, c.width)) {
            m.sub_target[n] = c.handle;
            n += 1;
            if n >= 16 {
                break;
            }
        }
    }
}

pub fn target_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = m.menu.clamp(0, 88) as usize;
    let prev = m.lists[i].prev;
    match m.proccess {
        2 => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r != 0 {
                m.msg.close();
                m.back_to_prev(prev);
            }
            return Flow::Done;
        }
        0 => {
            m.lists[i].select = 0;
            m.sub_target = [0; 16];
            m.proccess += 1;
        }
        1 => {}
        _ => return Flow::Done,
    }
    let u = what(m, x);
    let cands = candidates(&u, x);
    let mut names = Vec::new();
    for c in &cands {
        str_cat(&mut names, &c.name, 16);
    }
    let n = cands.len() as i16;
    m.lists[i].my = n;
    m.lists[i].y = n.min(8);
    let texts = x.texts;
    let msg_names = x.save.names();
    if u.target_type != -1 && n == 0 {
        m.lists[i].disp = 4;
        if is_drain(u.skill) {
            let w = &texts.target_drain_warn;
            m.msg.open_info([Some(&w[0]), Some(&w[1]), None, None], &msg_names);
        } else {
            m.msg.open_info([Some(&texts.target_warn), None, None, None], &msg_names);
        }
        m.proccess += 1;
        return Flow::Done;
    }
    if is_drain(u.skill) && m.drain_status == 0 {
        m.drain_status = 1;
    }
    m.bg_status = 3;
    if u.target_type != -1 {
        m.lists[i].disp = 6;
        extract_menu(m, names);
        m.select(0, 0, false, x);
        x.req.push(Request::TargetFix(true));
        let sel = m.lists[i].select.max(0) as usize;
        x.change_target(cands.get(sel));
        sub_targets(m, &u, x);
        // The help line: "Select target you wish to use <name>."
        let what_name = match prev {
            5 => u.item.map(|it| texts.items.item_name(i32::from(it.cat), i32::from(it.id))),
            4 => u.param.as_ref().map(|p| p.name.clone()),
            _ => None,
        };
        if let Some(mut line) = what_name {
            line.extend_from_slice(&texts.target_info[1]);
            m.msg.disp_msg(0x100, None, [Some(&texts.target_info[0]), Some(&line), None], &msg_names);
        }
        if x.pushed_cancel() {
            x.se(SE_BACK);
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.drain_status = 3;
            m.back_to_prev(prev);
            return Flow::Done;
        }
        if !x.pushed_ok() {
            return Flow::Done;
        }
        x.se(SE_OK);
        m.drain_status = 3;
    }
    use_it(m, &u, x)
}

/// The use (0x00532590).
fn use_it(m: &mut MenuCtrl, u: &Use, x: &mut Ctx) -> Flow {
    let prev = m.list().prev;
    let target = x.target.as_ref().map_or(0, |t| t.handle);
    match prev {
        5 => {
            let it = u.item.unwrap_or(Item::NONE);
            items::del_item(x.save, 0, i32::from(it.cat), i32::from(it.id), 1);
            m.menu_status = 3;
            // ccUseItemRequest: the task stays in the call until it is over
            // (panelStatus 1 and the tail after it, `useitem::Resume`).
            crate::menus::useitem::call(m, x, target, it.code(), crate::menus::useitem::Resume::Target);
            Flow::Done
        }
        4 if is_drain(u.skill) => {
            x.req.push(Request::DrainAffect(target));
            for k in 0..16 {
                if m.sub_target[k] != 0 {
                    x.req.push(Request::DrainAffect(m.sub_target[k]));
                }
            }
            m.pl_attack = 0;
            let cost = u.param.as_ref().map_or(0, |p| p.cost);
            if let Some(p) = x.world.party[0].as_mut() {
                let sp = (i32::from(p.sp) - cost).max(0) as i16;
                p.sp = sp;
                x.req.push(Request::PlayerSp(sp));
            }
            m.item_num = u.skill;
            m.change_menu_to(66);
            Flow::Done
        }
        4 => {
            m.menu_status = 3;
            m.panel_status = 1;
            x.req.push(Request::Skill { target, skill: u.skill as i16 });
            start_tail(m, x)
        }
        _ => Flow::Done,
    }
}

/// After the request: wake the tasks if asleep (one frame), then wait.
pub(crate) fn start_tail(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if m.still != 0 {
        x.req.push(Request::WakeAll);
        crate::disp::disp(m, x);
        return Flow::Breathed(Cont { woke: false, cursors: false, after: After::Use(UseTail::Woken) });
    }
    crate::disp::disp(m, x);
    Flow::Breathed(Cont { woke: false, cursors: false, after: After::Use(UseTail::Wait) })
}

/// The rest of the use, a frame at a time (each stage ends in its own
/// `Disp` and breath unless it returns `None`).
pub fn use_tail(m: &mut MenuCtrl, t: UseTail, x: &mut Ctx) -> Option<Cont> {
    let next = |t| Some(Cont { woke: false, cursors: false, after: After::Use(t) });
    match t {
        UseTail::Woken => {
            m.still = 0;
            x.req.push(Request::Still(false));
            crate::disp::disp(m, x);
            next(UseTail::Wait)
        }
        UseTail::Wait => {
            if x.world.player_skill >= 2 {
                crate::disp::disp(m, x);
                return next(UseTail::Wait);
            }
            x.change_target(None);
            crate::disp::disp(m, x);
            next(UseTail::Settle(1))
        }
        UseTail::Settle(n) if n < 6 => {
            crate::disp::disp(m, x);
            next(UseTail::Settle(n + 1))
        }
        UseTail::Settle(_) => {
            m.menu_next = -1;
            m.menu_status = 3;
            m.bg_status = 3;
            let woke = m.still == 1;
            if woke {
                x.req.push(Request::WakeAll);
            }
            crate::disp::disp(m, x);
            next(UseTail::Closed(woke))
        }
        UseTail::Closed(woke) => {
            if woke {
                m.still = 0;
                x.req.push(Request::Still(false));
            }
            m.first_time = 0;
            None
        }
    }
}
