//! What shows while `ccSetupDesktop` (0x00168320) runs the event passes at
//! phases 0 and 2, before the desktop exists (`docs/engine/desktop.md`, "Setup
//! and the task"): a black frame and, on a new game, event 1's setup lines in
//! a message window the event makes and draws itself (0x001a930c: its own
//! layer 242, `Disp` after each breath, `Check` from the tenth frame). The
//! runtime calls [`SetupScreen::step`] at the start of each frame, then runs
//! the event pass through the same methods [`crate::Desktop`] has.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_draw::Frame;
use piney_input::Pad;

use crate::anm::Ctx;
use crate::assets::read_fonts;
use crate::kanji::{Fonts, Names};
use crate::message::{MessageKind, MsgWindow, WindowTexture, render};
use crate::save::SaveState;
use crate::view::View;
use crate::{FRAME_RATE, Request};

/// `Disp` calls after `Check` has answered: 15 for a speech line, 8 for
/// information (0x001a9420, 0x001a98b4).
pub const TAIL_SPEECH: u32 = 15;
pub const TAIL_INFO: u32 = 8;

/// The setup screen.
pub struct SetupScreen {
    fonts: Fonts,
    window: Option<WindowTexture>,
    /// The event's window, from `Change` to its deletion.
    msg: Option<MsgWindow>,
    /// `Disp` calls left once `Check` has answered.
    tail: Option<u32>,
    kind: MessageKind,
    names: Names,
    /// `assignPADok`.
    ok: u32,
    requests: Vec<Request>,
}

impl SetupScreen {
    /// The fonts and the window texture (`XWINDOW.CCS` is resident).
    pub fn new(iso: &mut Iso, archive: Arc<Archive>, state: &SaveState) -> piney_data::Result<Self> {
        Ok(SetupScreen {
            fonts: read_fonts(iso.volume()?, &archive)?,
            window: WindowTexture::read(&archive),
            msg: None,
            tail: None,
            kind: MessageKind::Speech,
            names: state.names(),
            ok: state.ok(),
            requests: Vec::new(),
        })
    }

    /// From fonts and a window texture already read (the field UI's, whose
    /// set-ups have the same screen). The names and the button are
    /// [`SetupScreen::update`]'s.
    pub fn with(fonts: Fonts, window: Option<WindowTexture>) -> Self {
        SetupScreen {
            fonts,
            window,
            msg: None,
            tail: None,
            kind: MessageKind::Speech,
            names: Names::default(),
            ok: 0,
            requests: Vec::new(),
        }
    }

    /// One frame: black, and the event's window if one is up (its
    /// `Disp`).
    pub fn step(&mut self, _pad: &Pad) -> Frame {
        let mut ctx = Ctx::new(View::default());
        let draw = self.msg.is_some() && self.tail != Some(0);
        if draw {
            if let Some(t) = self.tail.as_mut() {
                *t -= 1;
            }
            let names = self.names.clone();
            if let Some(m) = self.msg.as_mut() {
                let draws = m.disp(FRAME_RATE);
                render(&draws, &mut ctx, &self.fonts, &names, self.window.as_ref());
            }
        }
        ctx.finish()
    }

    /// `message` / `info` / `info_now` at setup: a fresh window and
    /// `Change` (or `ChangeInfo`); its first `Disp` is the next frame's
    /// step. The arguments are [`crate::Desktop::open_message`]'s.
    pub fn open_message(&mut self, kind: MessageKind, emode: i32, name: Option<&[u8]>, lines: &[&[u8]]) {
        let mut m = MsgWindow::default();
        let line = |i: usize| lines.get(i).copied();
        match kind {
            MessageKind::Speech => {
                let l = |i| Some(line(i).unwrap_or(&[]));
                m.change(emode, name, [l(0), l(1), l(2)], &self.names);
            }
            MessageKind::Info | MessageKind::InfoNow => {
                let l = |i| line(i).filter(|s: &&[u8]| !s.is_empty());
                m.change_info([l(0), l(1), l(2), None], &self.names);
                if kind == MessageKind::InfoNow {
                    m.hide_frame();
                }
            }
        }
        self.msg = Some(m);
        self.kind = kind;
        self.tail = None;
    }

    /// `ccMsg->Check(0)`: 0 while the window waits, else 1 (or a
    /// question's answer); the window then fades over the tail.
    pub fn message_check(&mut self, pad: &Pad) -> i32 {
        let Some(m) = self.msg.as_mut() else { return 1 };
        let r = m.check(pad.push.bits(), pad.repeat.bits(), self.ok, &mut self.requests);
        if r != 0 && self.tail.is_none() {
            self.tail = Some(if self.kind == MessageKind::Speech { TAIL_SPEECH } else { TAIL_INFO });
        }
        r
    }

    /// The event deletes its window (`~ccMessage`, `~ccLayer`).
    pub fn close_message(&mut self) {
        self.msg = None;
        self.tail = None;
    }

    /// The save changed under the screen (name entry wrote the names; the
    /// event may set the buttons): take its names and decide button again.
    pub fn update(&mut self, state: &SaveState) {
        self.names = state.names();
        self.ok = state.ok();
    }

    /// Sounds and voice stops the window asked for since the last call.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }
}

impl SetupScreen {
    /// The event's window, for inspection.
    pub fn window(&self) -> Option<&MsgWindow> {
        self.msg.as_ref()
    }
}
