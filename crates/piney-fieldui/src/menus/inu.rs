//! Dun Loireag's dogs and Grunties: `NorainuMenu` (gcmn 0x0054a680, menu
//! 44, a stray dog: `npcTbl` rows 141-144), `OtonainuMenu` (0x0054ac40, 45,
//! a grown Grunty: rows 145-153) and `InuMenu` (0x0054b1d0, 46, a young one:
//! rows 154-157) with `InuMenuDisp` (0x0054bb10). The game draws
//! NorainuMenu's `talkNum` from `ccRand`, the town's generator, which the
//! town lends the menus ([`talk::TalkState::cc_rand`]). The Grunty's affect
//! functions change its msgNum at once, so the menus make the same change
//! to the world's copy ([`after_affect`]). The steps are in
//! docs/engine/field-ui.md.

use piney_desktop::eef::from_int;

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::breeder::{FOODS, Grunty, IMP_ITEM_LIST, SAVE_GROWTH, make_signed_num};
use crate::menus::personal;
use crate::menus::system::{extract_menu, str_cat};
use crate::spr::{font_type, set_clm};
use crate::talk::{self, TalkReq, Then};
use crate::window::disp_square;

/// What the menus do after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// NorainuMenu after its close for a lost target: ccMsg->Close,
    /// ccEvVoiceStop, then the keys.
    Keys,
    /// NorainuMenu after its close on cancel: ccMsg->Close, ccEvVoiceStop.
    Shut,
    /// OtonainuMenu after its close for a lost target: ccMsg->Close, then
    /// the keys.
    OtonaKeys,
    /// InuMenu after "There is no food." and the tasks woken: the Grunty
    /// the target again, the dim out, proccess 4.
    Woken,
}

/// The tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::Keys => {
            m.msg.close();
            x.req.push(Request::VoiceStop);
            talk::then(keys(m, x))
        }
        Tail::Shut => {
            m.msg.close();
            x.req.push(Request::VoiceStop);
            None
        }
        Tail::OtonaKeys => {
            m.msg.close();
            talk::then(otona_keys(m, x))
        }
        Tail::Woken => {
            m.still = 0;
            x.req.push(Request::Still(false));
            let prev = x.target_prev.clone();
            x.change_target(prev.as_ref());
            m.bg_status = 3;
            m.proccess = 4;
            None
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// `EntryAffect(cmndTarget, plw, n)`.
fn affect_target(x: &mut Ctx, n: i16) {
    if let Some(t) = x.target.as_ref() {
        let target = t.handle;
        talk::affect(x, target, n);
    }
}

/// `NorainuMenu`.
pub fn norainu_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if x.target.is_none() {
                return talk::close(m, x, Then::Nothing);
            }
            let mut buf = Vec::new();
            for k in m.lists[i].items.clone() {
                let name = m.lists.get(k.max(0) as usize).map(|l| l.name.clone()).unwrap_or_default();
                str_cat(&mut buf, &name, 16);
            }
            extract_menu(m, buf);
            if m.first_time != 0 {
                if let Some(base) = talk::target_base(m, x)
                    && base.msg != 0
                {
                    affect_target(x, 14);
                    let grp = crate::menus::talk::check_voice_grp(&x.texts.talk.talk, i32::from(base.id));
                    talk::open_record(m, x, base.msg, &base.name, grp, 0);
                }
                m.lists[i].select = 0;
            }
            let mut r = (m.talk.cc_rand() & 3) as i16;
            if r >= 3 {
                r = 0;
            }
            m.talk.talk_num = (r + 1) * 2 - 1;
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() {
                return talk::close(m, x, Then::Inu(Tail::Keys));
            }
            keys(m, x)
        }
        _ => Flow::Done,
    }
}

/// The keys (0x0054a9a4 on).
fn keys(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let key = if x.pushed_cancel() {
        x.se(SE_BACK);
        x.save.cancel()
    } else if x.pushed_ok() {
        x.se(SE_OK);
        x.save.ok()
    } else {
        0
    };
    if key == x.save.cancel() {
        affect_target(x, 0);
        return talk::close(m, x, Then::Inu(Tail::Shut));
    }
    if key == x.save.ok() {
        if m.lists[i].select != 0 {
            if m.still == 0 {
                talk::sleep_all(m, x);
            }
            x.change_target(None);
        }
        let flow = m.change_menu(x);
        m.msg.close();
        x.req.push(Request::VoiceStop);
        return flow;
    }
    Flow::Done
}

/// The Grunty `handle` as the world gave it this frame, with the menus'
/// changes (an idle one when it is someone else).
fn grunty_of(x: &Ctx, handle: u32) -> Grunty {
    match x.world.grunty {
        Some(g) if g.handle == handle && handle != 0 => g,
        _ => Grunty { handle, ..Grunty::default() },
    }
}

/// What the Grunty's affect function does to its msgNum, which the menus
/// read right after: `dogAction` (a young one, rows 154-157) sets it for a
/// line (15: 7 grown, 2 never fed, else its food line) and clears it as
/// the menu opens when grown (14); `dogAction2` and `dogActionAdult` (the
/// grown rows) set line 7 as the menu opens.
pub fn after_affect(g: &mut Grunty, kind: i16) {
    let young = g.row >= 154;
    match kind {
        15 if young => {
            g.msg = if g.level == 4 {
                7
            } else if g.size == 0 {
                2
            } else {
                g.food_num as i8
            };
        }
        14 if young => {
            if g.level == 4 {
                g.msg = 0;
            }
        }
        14 => g.msg = 7,
        _ => {}
    }
}

/// `EntryAffect(target, plw, kind)` on a Grunty, and its msgNum as the
/// affect function leaves it.
fn affect_grunty(x: &mut Ctx, handle: u32, kind: i16) {
    talk::affect(x, handle, kind);
    if let Some(g) = x.world.grunty.as_mut().filter(|g| g.handle == handle) {
        after_affect(g, kind);
    }
}

/// `OtonainuMenu`.
pub fn otonainu_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            let Some(h) = x.target.as_ref().map(|t| t.handle) else {
                return talk::close(m, x, Then::Nothing);
            };
            let mut buf = Vec::new();
            for k in m.lists[i].items.clone() {
                let name = m.lists.get(k.max(0) as usize).map(|l| l.name.clone()).unwrap_or_default();
                str_cat(&mut buf, &name, 16);
            }
            extract_menu(m, buf);
            if m.first_time != 0
                && let Some(base) = talk::target_base(m, x)
                && base.msg != 0
            {
                affect_grunty(x, h, 14);
                let grp = crate::menus::talk::check_voice_grp(&x.texts.talk.talk, i32::from(base.id));
                talk::open_record(m, x, base.msg, &base.name, grp, 0);
            }
            m.lists[i].select = 0;
            m.talk.talk_num = i16::from(grunty_of(x, h).msg);
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() {
                x.req.push(Request::VoiceStop);
                return talk::close(m, x, Then::Inu(Tail::OtonaKeys));
            }
            otona_keys(m, x)
        }
        _ => Flow::Done,
    }
}

/// OtonainuMenu's keys (0x0054af34 on).
fn otona_keys(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let key = if x.pushed_cancel() {
        x.se(SE_BACK);
        x.save.cancel()
    } else if x.pushed_ok() {
        x.se(SE_OK);
        x.save.ok()
    } else {
        0
    };
    if key == x.save.cancel() {
        if x.target.is_some() {
            affect_target(x, 0);
        }
        x.req.push(Request::VoiceStop);
        return talk::close(m, x, Then::MsgClose);
    }
    if key == x.save.ok() {
        if m.lists[i].select != 0 {
            if m.still == 0 {
                talk::sleep_all(m, x);
            }
            x.change_target(None);
        }
        x.req.push(Request::VoiceStop);
        let flow = m.change_menu(x);
        m.msg.close();
        return flow;
    }
    Flow::Done
}

/// `InuMenu`.
pub fn inu_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            if x.target.is_none() {
                return talk::close(m, x, Then::Nothing);
            }
            x.req.push(Request::TargetFix(true));
            let mut buf = Vec::new();
            for k in m.lists[i].items.clone() {
                let name = m.lists.get(k.max(0) as usize).map(|l| l.name.clone()).unwrap_or_default();
                str_cat(&mut buf, &name, 16);
            }
            extract_menu(m, buf);
            if m.first_time != 0 {
                m.lists[i].select = 0;
                m.fade = m.menu_fade.entry_fade(10, 0, 0x8000_0000) as i16;
                m.map_status = 3;
                m.menu_status = 3;
                m.cursol_off = 1;
                x.change_target(None);
                m.proccess += 1;
            } else {
                m.proccess = 4;
            }
        }
        1 => {
            if !personal::check_fade(m, i32::from(m.fade)) {
                if let Some(h) = x.target_prev.as_ref().map(|t| t.handle) {
                    affect_grunty(x, h, 11);
                }
                m.proccess += 1;
            }
        }
        2 => {
            personal::continue_fade(m, i32::from(m.fade), 10, 0);
            m.proccess += 1;
        }
        3 => {
            let id = i32::from(m.fade);
            if !personal::check_fade(m, id) {
                personal::delete_fade(m, id);
                let prev = x.target_prev.clone();
                x.change_target(prev.as_ref());
                if let Some(base) = talk::target_base(m, x)
                    && base.msg != 0
                {
                    let h = x.target.as_ref().map_or(0, |t| t.handle);
                    affect_grunty(x, h, 14);
                    let grp = crate::menus::talk::check_voice_grp(&x.texts.talk.talk, i32::from(base.id));
                    let msg = i32::from(grunty_of(x, h).msg);
                    talk::open_record(m, x, base.msg, &base.name, grp, msg);
                }
                m.proccess += 1;
            }
        }
        4 => {
            m.exception_disp = 1;
            m.menu_status = 1;
            m.proccess += 1;
            if let Some(h) = x.target.as_ref().map(|t| t.handle) {
                set_food(x, h, 0, None);
            }
        }
        5 => {
            m.select(0, 0, false, x);
            let key = if x.pushed_cancel() {
                x.se(SE_BACK);
                x.save.cancel()
            } else if x.pushed_ok() {
                x.se(SE_OK);
                x.save.ok()
            } else {
                0
            };
            if key == x.save.cancel() {
                x.req.push(Request::VoiceStop);
                m.msg.close();
                m.menu_status = 3;
                m.proccess = 20;
            } else if key == x.save.ok() {
                x.req.push(Request::VoiceStop);
                m.msg.close();
                let h = x.target.as_ref().map_or(0, |t| t.handle);
                if m.lists[i].select == 0 {
                    affect_grunty(x, h, 15);
                    m.talk.talk_num = i16::from(grunty_of(x, h).msg);
                    return m.change_menu(x);
                }
                let held = FOODS.clone().any(|k| x.save.save.u8(IMP_ITEM_LIST + k as usize) as i8 > 0);
                if !held {
                    m.menu_status = 3;
                    m.bg_status = 1;
                    if m.still == 0 {
                        talk::sleep_all(m, x);
                    }
                    x.change_target(None);
                    m.proccess = 10;
                    return Flow::Done;
                }
                set_food(x, h, 1, Some(1));
                x.change_target(None);
                return m.change_menu(x);
            }
        }
        10 => {
            if m.menu_status == 0 {
                let help = x.texts.talk.breeder.no_food.clone();
                let names = x.save.names();
                m.msg.open_info([Some(&help[..]), None, None, None], &names);
                m.proccess += 1;
            }
        }
        11 => {
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
            crate::menus::system::push_msg_requests(x, req);
            if r != 0 {
                m.msg.close();
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Inu(Tail::Woken));
            }
        }
        20 => {
            m.fade = m.menu_fade.entry_fade(10, 0, 0x8000_0000) as i16;
            m.proccess += 1;
        }
        21 => {
            if !personal::check_fade(m, i32::from(m.fade)) {
                if let Some(h) = x.target.as_ref().map(|t| t.handle) {
                    affect_grunty(x, h, 11);
                }
                m.proccess += 1;
            }
        }
        22 => {
            personal::continue_fade(m, i32::from(m.fade), 10, 0);
            m.proccess += 1;
        }
        23 => {
            let id = i32::from(m.fade);
            if !personal::check_fade(m, id) {
                personal::delete_fade(m, id);
                if let Some(h) = x.target.as_ref().map(|t| t.handle) {
                    affect_grunty(x, h, 0);
                }
                m.map_status = 1;
                m.cursol_off = 0;
                return talk::close(m, x, Then::Nothing);
            }
        }
        _ => {}
    }
    Flow::Done
}

/// InuMenu's writes to the Grunty: foodMode (and chatFlag).
fn set_food(x: &mut Ctx, handle: u32, food_mode: u8, chat_flag: Option<u8>) {
    talk::req(x, TalkReq::GruntyFood { target: handle, food_mode, chat_flag });
    if let Some(g) = x.world.grunty.as_mut().filter(|g| g.handle == handle) {
        g.food_mode = food_mode;
        if let Some(c) = chat_flag {
            g.chat_flag = c;
        }
    }
}

/// `InuMenuDisp` (gcmn 0x0054bb10): the STATUS window (menuWindow at
/// (336, 36), `DispSquare(9, 6, "STATUS")`), `breedingMenuStr`'s six rows
/// in settingKanji[0] at (350, 52 + 20 r) and the town's Grunty's size
/// .. pure (`growth[game.town]`) by `MakeSignedNum(3, v)` at (434, ...).
pub fn inu_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let a = m.alpha;
    m.win.dx = 336.0;
    m.win.dy = 36.0;
    let status = x.texts.talk.breeder.status.clone();
    disp_square(&mut m.win, 9, 6, Some(&status));
    let s0 = &mut m.setting[0];
    s0.set_colour(7);
    s0.set_alpha(a);
    set_clm(s0, 16, 0, 0, 1);
    m.setting_text[0] = x.texts.talk.breeder.status_rows.clone();
    for k in 0..6 {
        let s0 = &mut m.setting[0];
        s0.dx = 350.0;
        s0.dy = from_int(k * 20 + 52);
        s0.make_packet(k);
    }
    let town = x.world.game.town.clamp(0, 4) as usize;
    let g = |k: usize| i32::from(x.save.save.i16(SAVE_GROWTH + 24 * town + 2 * k));
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    for (j, y) in [52.0, 72.0, 92.0, 112.0, 132.0, 152.0].into_iter().enumerate() {
        m.font.dx = 434.0;
        m.font.dy = y;
        make_signed_num(&mut m.font, 3, g(j + 1));
    }
}
