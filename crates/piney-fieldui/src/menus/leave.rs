//! Leaving the area or The World: `GateoutMenu` (gcmn 0x0053c8e0, menu 10,
//! Gate Out), `LogoutMenu` (0x0053cd60, menu 11, Log Out) and
//! `TransFieldMenu` (0x0056a640, menu 86); and where the party is:
//! `AreaInfoMenu` (0x0056a7f0, menu 87, Area Information), whose page is
//! the Chaos Gate's keyword screen (`GtNewMenuDisp`, [`crate::menus::gate`])
//! over the area's keywords ([`crate::FieldUi::set_area_words`]). The steps
//! are in docs/engine/field-ui.md (PERSONAL's pages).

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl};
use crate::menus::personal::{self, Tail, check, check_fade, key};
use crate::menus::system::extract_menu;

/// What a leaving menu does after its close's breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveTail {
    /// Gate Out: back to town.
    GateOut,
    /// Log Out: The World's top page.
    LogOut,
    /// 86's first frames: the tasks woken.
    TransWoken,
    /// 86's end: `WORLD_MAN::GoField`.
    TransGo,
}

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

fn back(m: &mut MenuCtrl) {
    let prev = m.lists[idx(m)].prev;
    m.back_to_prev(prev);
}

/// The close after a fade or a wait (inlined `CloseMenu`): the tasks woken
/// if asleep, `Disp`, the breath; `then` next frame.
fn close_then(m: &mut MenuCtrl, x: &mut Ctx, then: LeaveTail) -> Flow {
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont { woke, ..personal::pers(Tail::Leave(then)) })
}

/// The frame after a leaving menu's breath.
pub fn tail(m: &mut MenuCtrl, c: Cont, t: LeaveTail, x: &mut Ctx) -> Option<Cont> {
    if c.woke {
        m.still = 0;
        x.req.push(Request::Still(false));
    }
    match t {
        LeaveTail::TransWoken => {
            m.wait_count = 0;
            m.proccess += 1;
        }
        LeaveTail::GateOut => {
            m.first_time = 0;
            let town = x.world.game.town.max(0);
            x.req.push(Request::DeleteNoPartyMember);
            x.req.push(Request::ChangeArea { area: 0, n: town });
        }
        LeaveTail::LogOut => {
            m.first_time = 0;
            x.req.push(Request::ChangeMode { num: 4, sf: 7 });
        }
        LeaveTail::TransGo => {
            m.first_time = 0;
            x.req.push(Request::GoField);
        }
    }
    None
}

/// The confirmation: `info` with OK / Cancel (list disp 11), Cancel on.
fn ask(m: &mut MenuCtrl, x: &mut Ctx, info: &[u8]) {
    let i = idx(m);
    m.lists[i].disp = 11;
    let names = x.save.names();
    m.msg.open_info([Some(info), None, None, None], &names);
    let d = x.texts.dialog_default.clone();
    extract_menu(m, d);
    m.lists[i].select = 1;
    m.wait_count = 0;
    m.proccess += 1;
}

/// Proccess 1 of both: OK on OK starts the fade to black.
fn answer(m: &mut MenuCtrl, x: &mut Ctx) {
    m.select(0, 0, false, x);
    match key(x) {
        Some(true) => {
            m.msg.close();
            if m.lists[idx(m)].select == 1 {
                back(m);
                return;
            }
            m.menu_status = 3;
            // EntryFade(20, clear, black, 0, 0, 512, 448).
            m.fade = m.menu_fade.entry_fade(20, 0, 0x8000_0000) as i16;
            m.cursor.init(true);
            m.cursor_pr.init(true);
            m.proccess += 1;
        }
        Some(false) => {
            m.msg.close();
            back(m);
        }
        None => {}
    }
}

/// `GateoutMenu` (menu 10).
pub fn gateout_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let step = m.proccess;
    match step {
        0 => {
            if x.world.game.in_battle != 0 {
                let i = idx(m);
                m.lists[i].disp = 4;
                let names = x.save.names();
                let w = x.texts.pers.in_battle_warn.clone();
                m.msg.open_info([Some(&w), None, None, None], &names);
                m.proccess = 10;
            } else {
                let info = x.texts.pers.gateout_info.clone();
                ask(m, x, &info);
            }
        }
        1 => answer(m, x),
        2 => {
            if !check_fade(m, i32::from(m.fade)) {
                return close_then(m, x, LeaveTail::GateOut);
            }
        }
        10 if check(m, x) => {
            m.msg.close();
            back(m);
        }
        _ => {}
    }
    Flow::Done
}

/// `LogoutMenu` (menu 11).
pub fn logout_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let step = m.proccess;
    match step {
        0 => {
            let info = x.texts.pers.logout_info.clone();
            ask(m, x, &info);
        }
        1 => answer(m, x),
        2 if !check_fade(m, i32::from(m.fade)) => {
            return close_then(m, x, LeaveTail::LogOut);
        }
        _ => {}
    }
    Flow::Done
}

/// `TransFieldMenu` (menu 86).
pub fn trans_field_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let step = m.proccess;
    match step {
        0 => {
            m.menu_status = 3;
            m.bg_status = 3;
            if m.still == 1 {
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return Flow::Breathed(Cont { woke: true, ..personal::pers(Tail::Leave(LeaveTail::TransWoken)) });
            }
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w < 121 {
                return Flow::Done;
            }
            m.wait_count = 120;
            if x.world.party_annihilated || x.world.game_over {
                return Flow::Done;
            }
            return close_then(m, x, LeaveTail::TransGo);
        }
        _ => {}
    }
    Flow::Done
}

/// `AreaInfoMenu` (menu 87).
pub fn area_info_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let step = m.proccess;
    match step {
        0 => {
            m.exception_disp = 4;
            let ids = m.generated.ids;
            m.temp[..3].copy_from_slice(&ids);
            m.proccess += 1;
        }
        1 if x.pushed_cancel() => {
            x.se(crate::ctrl::SE_BACK);
            back(m);
        }
        _ => {}
    }
    Flow::Done
}
