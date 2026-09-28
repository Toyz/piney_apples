//! An item used from the menus: `ccUseItemRequest(plw, target, code, 0)`
//! (gcmn 0x0057aa80) is one blocking call on the menu task in the game.
//! The port splits it: `piney_battle::item::use_item_request` works out
//! the whole use as steps ([`Step`]), the runtime answers the menu's
//! [`Request::UseItem`] with them ([`crate::FieldUi::answer_item`]), and
//! this module plays them where the call stood: the menu's own steps here,
//! the world's back one at a time as [`Request::ItemStep`], then the
//! handler goes on ([`Resume`]). See docs/engine/field-ui.md.

use std::collections::VecDeque;

use piney_battle::item::{Info, Step};

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl};

/// Where the handler goes on once the use is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// TargetMenu's use (0x00532590): `panelStatus = 1`, then its tail.
    Target,
    /// ItemMenu's item used at once (`ccCheckItemUseful` 2): the menu
    /// shuts (`menuNext -1`, statuses 3, the tasks woken, the breath).
    AtOnce,
    /// ItemMenu's Sprite Ocarina (proccess 12): nothing more that frame.
    Ocarina,
    /// ImportantItemMenu's Grunty Flute (proccess 20, 0x00530370): the menu
    /// shuts as [`crate::menus::personal::close`] does.
    Flute,
}

/// What the call is doing between frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wait {
    /// The next step.
    Next,
    /// A `Frames(n)` loop: this many passes left.
    Frames(u32),
    /// `WaitMessage`'s loop.
    Message,
    /// `CloseMenuDisp` breathed after waking the tasks: `still = 0` and
    /// the flips back on come first.
    Closed,
    /// `WaitParty`'s loop.
    Party,
    /// Inside `ccPuccigusoStart`: breaths while it fades.
    Start,
    /// `WaitRide`'s loop.
    Ride,
    /// `WaitMap`'s loops: the frames breathed so far.
    Map(u32),
}

/// The call in progress.
pub struct ItemRun {
    pub resume: Resume,
    /// None until the runtime answers ([`MenuCtrl::answer_item`]).
    steps: Option<VecDeque<Step>>,
    wait: Wait,
}

impl ItemRun {
    pub fn new(resume: Resume) -> Self {
        ItemRun { resume, steps: None, wait: Wait::Next }
    }

    /// Waiting for the runtime's steps.
    pub fn asked(&self) -> bool {
        self.steps.is_none()
    }
}

/// The handler calls `ccUseItemRequest`: [`Request::UseItem`] out, the
/// menu task held in the call until the runtime answers.
pub fn call(m: &mut MenuCtrl, x: &mut Ctx, target: u32, code: i32, resume: Resume) {
    x.req.push(Request::UseItem { target, code });
    m.item = Some(ItemRun::new(resume));
}

/// The handler calls `ccUseItemRequest(plw, target, code, arg)`: as
/// [`call`], with the argument ([`Request::UseItemArg`]).
pub fn call_arg(m: &mut MenuCtrl, x: &mut Ctx, target: u32, code: i32, arg: i32, resume: Resume) {
    x.req.push(Request::UseItemArg { target, code, arg });
    m.item = Some(ItemRun::new(resume));
}

/// `Disp`, then the breath: the frame ends inside the call.
fn breathe(m: &mut MenuCtrl, x: &mut Ctx) -> Option<Cont> {
    crate::disp::disp(m, x);
    Some(Cont { woke: false, cursors: false, after: After::Item })
}

/// A breath with nothing drawn (`ccBreathThread(1)` alone).
fn breathe_bare() -> Option<Cont> {
    Some(Cont { woke: false, cursors: false, after: After::Item })
}

/// `ccChatMsg::Disp(0)`, then the breath (`WaitParty`, `WaitRide`).
fn breathe_chat(m: &mut MenuCtrl, x: &mut Ctx) -> Option<Cont> {
    let player = x.world.party[0].as_ref().map(|c| c.handle);
    m.chat.disp(false, &x.world.chat_at, player, &mut m.draws);
    breathe_bare()
}

/// The call from where it stands to its next breath, or to its end and
/// the handler's rest. Some: the frame is over (its `Disp` done); None:
/// the task loop's `Disp` ends it.
pub fn run(m: &mut MenuCtrl, x: &mut Ctx) -> Option<Cont> {
    loop {
        let r = m.item.as_mut()?;
        match r.wait {
            Wait::Frames(n) if n > 0 => {
                r.wait = Wait::Frames(n - 1);
                return breathe(m, x);
            }
            Wait::Frames(_) => r.wait = Wait::Next,
            Wait::Message => {
                if !crate::menus::tutorial::check(m, x) {
                    return breathe(m, x);
                }
                if let Some(r) = m.item.as_mut() {
                    r.wait = Wait::Next;
                }
                continue;
            }
            Wait::Closed => {
                r.wait = Wait::Next;
                m.still = 0;
                x.req.push(Request::Still(false));
                continue;
            }
            Wait::Party if x.world.party_annihilated || x.world.game_over => return breathe_chat(m, x),
            Wait::Start if x.world.pg_starting => return breathe_bare(),
            Wait::Ride if x.world.pg_ride => return breathe_chat(m, x),
            Wait::Party | Wait::Start | Wait::Ride => r.wait = Wait::Next,
            Wait::Map(n) if x.world.map_showing || n < 20 => {
                r.wait = Wait::Map(n + 1);
                return breathe(m, x);
            }
            Wait::Map(_) => r.wait = Wait::Next,
            Wait::Next => {}
        }
        let r = m.item.as_mut()?;
        let Some(step) = r.steps.as_mut().and_then(|s| s.pop_front()) else {
            let resume = r.resume;
            m.item = None;
            return resume_after(m, x, resume);
        };
        if let Some(c) = one(m, x, step) {
            return Some(c);
        }
    }
}

/// One step; Some when it breathed.
fn one(m: &mut MenuCtrl, x: &mut Ctx, step: Step) -> Option<Cont> {
    let set = |m: &mut MenuCtrl, w: Wait| {
        if let Some(r) = m.item.as_mut() {
            r.wait = w;
        }
    };
    match step {
        Step::CloseMenuDisp => {
            m.menu_status = 3;
            m.bg_status = 3;
            m.panel_status = 1;
            m.panel_alpha = -24;
            if m.still == 1 {
                x.req.push(Request::WakeAll);
                set(m, Wait::Closed);
            }
            return breathe(m, x);
        }
        Step::Frames(n) => {
            set(m, Wait::Frames(n));
        }
        // The runtime's first ShowMap call; its answer comes in the next
        // frame's World::map_showing, after that frame's call.
        Step::WaitMap => {
            x.req.push(Request::ItemStep(step));
            set(m, Wait::Map(0));
        }
        Step::WaitMessage => {
            set(m, Wait::Message);
        }
        Step::OpenInfo(info) => open_info(m, x, info),
        Step::PanelStatus(v) => m.panel_status = v,
        Step::BgStatus(v) => m.bg_status = v,
        Step::CloseMessage => m.msg.close(),
        Step::CloseMessageInstant => m.msg.close_instant(),
        Step::ClearCmndTarget => x.change_target(None),
        Step::SleepAllThread => x.req.push(Request::SleepAll),
        Step::WakeAllThread => x.req.push(Request::WakeAll),
        Step::StillOn => {
            m.still = 1;
            x.req.push(Request::Still(true));
            x.req.push(Request::KeepLayers);
        }
        Step::StillOff => {
            m.still = 0;
            x.req.push(Request::Still(false));
        }
        Step::ChangeMenu(n) => m.change_menu_to(n as i16),
        Step::Se(n) => x.se(n),
        Step::WaitParty => set(m, Wait::Party),
        Step::WaitRide => set(m, Wait::Ride),
        // ccPuccigusoStart: its first slice on the world now, then breaths
        // while it fades out and back in.
        Step::Pucciguso(_) => {
            x.req.push(Request::ItemStep(step));
            set(m, Wait::Start);
            return breathe_bare();
        }
        _ => x.req.push(Request::ItemStep(step)),
    }
    None
}

/// `ccMessage::OpenInfo` with the use's text.
fn open_info(m: &mut MenuCtrl, x: &mut Ctx, info: Info) {
    let t = &x.texts.use_texts;
    let names = x.save.names();
    match info {
        Info::TrapDischarge => m.msg.open_info([Some(&t.trap_discharge), None, None, None], &names),
        Info::ShowMap => m.msg.open_info([Some(&t.show_map), None, None, None], &names),
        Info::StatusUp { stat, amount } => {
            let piece = |k: i32| t.status_up.get(k.max(0) as usize).cloned().unwrap_or_default();
            let mut line = piece(stat);
            line.extend_from_slice(&piece(if amount < 0 { 1 } else { 0 }));
            line.extend_from_slice(&piney_desktop::kanji::dec2sjis(amount.abs(), 16, 0));
            line.extend_from_slice(&t.status_up_stop);
            m.msg.open_info([Some(&line), None, None, None], &names);
        }
        Info::EpitaphUnknown => m.msg.open_info([Some(&t.epitaph_unknown), None, None, None], &names),
        Info::InstallWarn => {
            let w = &t.install_warn;
            m.msg.open_info([Some(&w[0]), Some(&w[1]), Some(&w[2]), None], &names);
        }
    }
}

/// The handler's rest after the call.
fn resume_after(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Option<Cont> {
    let flow = match r {
        Resume::Target => {
            m.panel_status = 1;
            crate::menus::target::start_tail(m, x)
        }
        Resume::AtOnce => {
            m.menu_next = -1;
            m.menu_status = 3;
            m.bg_status = 3;
            let woke = m.breathe_close(x);
            Flow::Breathed(Cont::close(woke))
        }
        Resume::Ocarina => Flow::Done,
        Resume::Flute => crate::menus::personal::close(m, x),
    };
    match flow {
        Flow::Breathed(c) => Some(c),
        Flow::Done => None,
    }
}

impl MenuCtrl {
    /// The runtime's answer to [`Request::UseItem`]: the use's steps (none
    /// when nothing is done here). The call goes on this frame.
    pub fn answer_item(&mut self, steps: Vec<Step>) {
        if let Some(r) = self.item.as_mut().filter(|r| r.asked()) {
            r.steps = Some(steps.into());
        }
    }

    /// The call waits for the runtime's steps.
    pub fn item_asked(&self) -> bool {
        self.item.as_ref().is_some_and(ItemRun::asked)
    }
}
