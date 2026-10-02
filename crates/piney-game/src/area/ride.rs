//! The riding Grunty's sequence around `ccPucciguso` (gcmn `pgrider.cpp`):
//! `ccPuccigusoStart` (0x005109c0, on the menu task), `ccThPucciguso`
//! (0x00510880, priority 49) and `ccPuccigusoExit` (0x00510bd0), on the area's
//! fader, menu and music and the field's `ride_*` calls. The menu task runs
//! after the field's tasks here: the start's slices run before the menu's frame
//! ([`AreaMode::ride_menu_slot`]), the task's before the field's
//! ([`AreaMode::ride_task_slot`]); a way into a dungeon ends a ride as
//! `ccThPuccigusoDelete` does. The steps are in docs/engine/grunty-ride.md.

use piney_audio::PgBgm;
use piney_input::Pad;

use super::AreaMode;
use crate::mode::Event;

/// Where the sequence stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RideSeq {
    /// `ccPuccigusoStart` on the menu task, with its fade's element.
    pub start: Option<Start>,
    /// `ccThPucciguso`.
    pub task: Option<Task>,
    /// The Grunty's kind (the task's parameter).
    pub kind: i32,
    /// `pgRideFlag` as the menu task sees it this frame.
    pub seen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// `EntryFade(10, 0, 0x80000000)` running.
    Out(i32),
    /// `ContinueFade(15, 0)` running.
    In(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    /// `Main` each frame; the count of those done.
    Riding(u32),
    /// `ccPuccigusoExit`'s fade out, `Main` each frame.
    ExitOut(i32),
    /// Its fade back in.
    ExitIn(i32),
}

/// `EntryFade`'s colours: clear, then the black the fade reaches.
const CLEAR: u32 = 0;
const BLACK: u32 = 0x8000_0000;
/// `ccThPucciguso`'s frames of `Main` before the dismount is looked for:
/// its first loop's 30 and the second's first.
const FIRST_MAINS: u32 = 31;

impl AreaMode {
    /// `ccScFade::CheckFade(i)` (main 0x001604d0) on `scFadeDef`: the
    /// element has not counted to its end.
    fn check_fade(&self, i: i32) -> bool {
        usize::try_from(i).ok().and_then(|i| self.st.fade_def.elems.get(i)).is_some_and(|e| e.cnt < e.tcnt)
    }

    /// `ccScFade::DeleteFade(i)` (main 0x00160510).
    fn delete_fade(&mut self, i: i32) {
        if let Some(e) = usize::try_from(i).ok().and_then(|i| self.st.fade_def.elems.get_mut(i)) {
            e.status = 0;
        }
    }

    /// `ccPuccigusoStart(kind)`'s first slice, from the flute's `Pucciguso`
    /// step: refused outside a field or while riding (the menu goes on).
    pub(super) fn ride_start(&mut self, kind: i32) {
        if !self.world.ride_start() {
            self.st.log(format!("pucciguso {kind} refused"));
            return;
        }
        let c = &mut self.ui.ctrl;
        c.forbid = 1;
        c.panel_status = 3;
        self.events.push(Event::PgBgm(PgBgm::Riding(true)));
        self.events.push(Event::PgBgm(PgBgm::Init));
        let f = self.st.fade_def.entry(10, CLEAR, BLACK);
        self.ride = RideSeq { start: Some(Start::Out(f)), task: None, kind, seen: self.ride.seen };
        self.st.log(format!("pucciguso {kind}"));
    }

    /// The menu task's slot while it is inside `ccPuccigusoStart`, before
    /// the menu's frame: past the fade out the party asleep, the file and
    /// the task, the fade back in; past that the camera free and the music.
    pub(super) fn ride_menu_slot(&mut self) {
        match self.ride.start {
            Some(Start::Out(f)) if !self.check_fade(f) => {
                self.world.ride_sleep_party();
                if let Err(e) = self.world.ride_create(self.ride.kind) {
                    tracing::warn!("the Grunty: {e}");
                }
                self.ride.task = Some(Task::Riding(0));
                self.st.fade_def.continue_fade(f, 15, CLEAR);
                self.ride.start = Some(Start::In(f));
            }
            Some(Start::In(f)) if !self.check_fade(f) => {
                self.delete_fade(f);
                self.world.ride_start_done();
                self.events.push(Event::BgmStream(0));
                self.ride.start = None;
            }
            _ => {}
        }
    }

    /// `ccThPucciguso`'s slot, before the field's tasks: the dismount (the
    /// cancel button, `saveData.assignPadCancel`, pushed after the first
    /// frames while riding) into `ccPuccigusoExit`, and its fades. The
    /// ride's `Main` itself runs in the field's tasks while it is on.
    pub(super) fn ride_task_slot(&mut self, pad: &Pad) {
        match self.ride.task {
            Some(Task::Riding(n)) => {
                let cancel = u32::from(self.world.state().save.assign_pad_cancel());
                if n >= FIRST_MAINS && pad.push.bits() & cancel != 0 && self.world.pg_ride() {
                    let g = self.world.pg_globals();
                    if g.pg_din != 0 || self.world.scene().area != 1 {
                        self.ride_leave();
                        return;
                    }
                    self.world.ride_exit_begin();
                    let f = self.st.fade_def.entry(10, CLEAR, BLACK);
                    self.ride.task = Some(Task::ExitOut(f));
                    self.st.log("pucciguso exit".into());
                } else {
                    self.ride.task = Some(Task::Riding(n + 1));
                }
            }
            Some(Task::ExitOut(f)) if !self.check_fade(f) => {
                self.world.ride_exit_wake();
                self.ui.ctrl.panel_status = 1;
                self.st.fade_def.continue_fade(f, 15, CLEAR);
                self.ride.task = Some(Task::ExitIn(f));
            }
            Some(Task::ExitIn(f)) if !self.check_fade(f) => {
                self.delete_fade(f);
                self.ui.ctrl.forbid = 0;
                self.events.push(Event::PgBgm(PgBgm::End(0)));
                self.events.push(Event::PgBgm(PgBgm::Riding(false)));
                self.world.ride_end();
                self.ride.task = None;
                self.st.log("pucciguso off".into());
            }
            _ => {}
        }
    }

    /// `ccThPuccigusoDelete` (0x00510990) as the scene changes while
    /// riding: `ccPuccigusoExit` with `pgDIN` set (or out of the field):
    /// `ccPgBgmEnd(1)`, the object deleted and the flags cleared.
    pub(super) fn ride_leave(&mut self) {
        if self.world.pg_ride() {
            self.events.push(Event::PgBgm(PgBgm::End(1)));
            self.events.push(Event::PgBgm(PgBgm::Riding(false)));
            self.world.ride_end();
        }
        self.ride.start = None;
        self.ride.task = None;
    }
}
