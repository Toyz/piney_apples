//! `ccMenuCtrl::StreamMenu` (gcmn 0x00535340), menu 74: a stream played
//! over the field with party members drawn into it - Skeith's Data Drain
//! on a member (ccBoss01's `OnDataDrainAtk` asks for stream 20 with the
//! member's slot). One call that breathes its own frames: the fade, the
//! stream and `ccThStrParty`, then the menu shut. The runtime plays the
//! stream ([`Request::StreamMenu`]) and draws the members
//! ([`Request::StrParty`]); it is there and playing from its start. The
//! steps are in docs/engine/field-ui.md (a member drained).

use crate::Request;
use crate::ctrl::{After, Cont, Flow, MenuCtrl};
use crate::menus::personal::{check_fade, delete_fade};

/// `StreamMenu`'s locals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamState {
    /// The fade in's element, -1 none.
    pub fade: i32,
    /// The stream has ended ([`crate::FieldUi::stream_menu_done`]).
    pub done: bool,
}

/// Where `StreamMenu`'s frames resume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// The fade in: after its `Disp; Breath`, `CheckFade`.
    Fade,
    /// `ccBreathThread(2)` before the stream: frames left.
    Black(u8),
    /// `ccBreathThread(2)` after its start: frames left.
    Start(u8),
    /// `ccBreathThread(2)` after the fade is deleted: frames left.
    Party(u8),
    /// The stream playing: `ccBreathThread(1)` until it has ended.
    Wait,
}

fn wait(r: Resume) -> Flow {
    Flow::Breathed(Cont { woke: false, cursors: false, after: After::Stream(r) })
}

fn breathe(m: &mut MenuCtrl, x: &mut crate::ctrl::Ctx, r: Resume) -> Flow {
    crate::disp::disp(m, x);
    wait(r)
}

/// Menu 74's handler: the whole function, from its first call.
pub fn stream_menu(m: &mut MenuCtrl, x: &mut crate::ctrl::Ctx) -> Flow {
    m.stream_menu = StreamState { fade: -1, done: false };
    let f = m.stream_flag;
    if f & 0x100 == 0 {
        let (t, col) = if f & 0x200 != 0 { (1, 0x80ff_ffff) } else { (12, 0x8000_0000) };
        m.stream_menu.fade = m.menu_fade.entry_fade(t, 0, col);
        return breathe(m, x, Resume::Fade);
    }
    after_fade(m, x)
}

/// The world's layers back, the screen black, two frames.
fn after_fade(m: &mut MenuCtrl, x: &mut crate::ctrl::Ctx) -> Flow {
    if m.still == 1 {
        m.still = 0;
        x.req.push(Request::Still(false));
    }
    x.req.push(Request::WorldHidden(true));
    wait(Resume::Black(1))
}

pub fn resume(m: &mut MenuCtrl, r: Resume, x: &mut crate::ctrl::Ctx) -> Option<Cont> {
    match run(m, x, r) {
        Flow::Breathed(c) => Some(c),
        Flow::Done => None,
    }
}

fn run(m: &mut MenuCtrl, x: &mut crate::ctrl::Ctx, r: Resume) -> Flow {
    match r {
        Resume::Fade => {
            if check_fade(m, m.stream_menu.fade) {
                return breathe(m, x, Resume::Fade);
            }
            after_fade(m, x)
        }
        Resume::Black(1) => wait(Resume::Black(0)),
        Resume::Black(_) => {
            x.req.push(Request::StreamMenu(i32::from(m.stream_num)));
            m.stream_menu.done = false;
            wait(Resume::Start(1))
        }
        Resume::Start(1) => wait(Resume::Start(0)),
        Resume::Start(_) => {
            if m.stream_menu.fade >= 0 {
                delete_fade(m, m.stream_menu.fade);
            }
            wait(Resume::Party(1))
        }
        Resume::Party(1) => wait(Resume::Party(0)),
        Resume::Party(_) => {
            if m.stream_flag & 7 != 0 {
                x.req.push(Request::StrParty(m.stream_flag));
            }
            ended(m, x)
        }
        Resume::Wait => ended(m, x),
    }
}

/// The stream's end: the world back, the tasks woken, the menu closed.
fn ended(m: &mut MenuCtrl, x: &mut crate::ctrl::Ctx) -> Flow {
    if !m.stream_menu.done {
        return wait(Resume::Wait);
    }
    x.req.push(Request::WorldHidden(false));
    x.req.push(Request::WakeAll);
    m.menu_next = -1;
    m.menu = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    m.panel_status = 1;
    m.panel_alpha = -24;
    x.req.push(Request::TargetFix(false));
    m.first_time = 0;
    Flow::Done
}
