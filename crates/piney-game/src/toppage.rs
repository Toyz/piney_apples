//! Mode 4, The World's top page (`ccSetupToppage`, `TOPPAGE.PRG`), with the
//! event task running over it as on the desktop: the passes at 0 and 2
//! (`ccGame.status` 3) in the setup, then the page with phase 4, the event task
//! first each frame. Log out is `ChangeRequest(3, 7)`; Log in is
//! `ChangeRequest(5, 8)`, `ChangeArea(0, lastTown)`, `ChangeRequest(6, 7)`.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::SaveState;
use piney_draw::Frame;
use piney_event::vm::Vm;
use piney_input::Pad;
use piney_toppage::{Request, TopPage};

use crate::desktop::{self, Bridge, HostState, Target};
use crate::mode::{Event, Mode};

/// `ccGame.status` on the board.
const STATUS_TOPPAGE: i32 = 3;
/// Frames an event pass may take before the setup goes on anyway.
const SETUP_FRAMES: u32 = 600;

pub struct TopPageMode {
    /// None when the setup's scripts asked for another mode instead.
    page: Option<Box<TopPage>>,
    /// The save while there is no page.
    save: Option<SaveState>,
    vm: Option<Vm>,
    st: HostState,
    frames: u64,
}

impl TopPageMode {
    /// `ccSetupToppage` on a running game: the event task's passes at phases
    /// 0 and 2 on the save, then the page and phase 4.
    pub fn enter(
        iso: PathBuf,
        archive: Arc<Archive>,
        mut state: SaveState,
        mut vm: Option<Vm>,
    ) -> Result<TopPageMode, String> {
        let mut st = HostState::new(STATUS_TOPPAGE, iso.clone(), archive.clone(), false);
        let mut abandoned = false;
        if let Some(vm) = &mut vm {
            for phase in [0, 2] {
                {
                    let mut h = Bridge { target: Target::Setup(&mut state, None), st: &mut st };
                    if phase == 0 {
                        vm.start_thread(&mut h);
                    }
                    vm.enable(phase);
                    let mut n = 0;
                    while !vm.enable_settled(phase) && n < SETUP_FRAMES {
                        vm.frame(&mut h);
                        n += 1;
                    }
                }
                if st.events.iter().any(|e| matches!(e, Event::ChangeMode { .. })) {
                    abandoned = true;
                    break;
                }
            }
        }
        if abandoned {
            // The scripts want another mode: no page.
            let mut mode = TopPageMode { page: None, save: Some(state), vm, st, frames: 0 };
            mode.flush();
            return Ok(mode);
        }
        let mut disc = Iso::open(&iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let page = TopPage::new(&mut disc, archive, state).map_err(|e| format!("top page: {e}"))?;
        if let Some(vm) = &mut vm {
            vm.enable(4);
        }
        let mut mode = TopPageMode { page: Some(Box::new(page)), save: None, vm, st, frames: 0 };
        mode.flush();
        Ok(mode)
    }

    /// The save the board holds, for the main loop's play time.
    pub fn save_mut(&mut self) -> Option<&mut piney_data::save::SaveData> {
        match (&mut self.page, &mut self.save) {
            (Some(p), _) => Some(&mut p.state_mut().save),
            (None, Some(s)) => Some(&mut s.save),
            (None, None) => None,
        }
    }

    /// Leave the board: the save and the event task, for the next mode.
    /// The page, while there is one.
    #[cfg(test)]
    pub(crate) fn page(&self) -> Option<&TopPage> {
        self.page.as_deref()
    }

    /// Whether the scripts play a stream over the board.
    #[cfg(test)]
    pub(crate) fn streaming(&self) -> bool {
        self.st.stream.is_some()
    }

    /// Whether an event block plays (its windows wait for OK).
    #[cfg(test)]
    pub(crate) fn event_playing(&self) -> bool {
        self.vm.as_ref().is_some_and(|v| v.playing().is_some())
    }

    pub fn leave(self) -> (SaveState, Option<Vm>) {
        let TopPageMode { page, save, mut vm, .. } = self;
        if let Some(vm) = &mut vm {
            vm.disable();
        }
        let state = match (page, save) {
            (Some(p), _) => p.state().clone(),
            (None, Some(s)) => s,
            (None, None) => SaveState::fresh(),
        };
        (state, vm)
    }

    fn flush(&mut self) {
        for line in self.st.lines.drain(..) {
            eprintln!("{line}");
        }
    }
}

impl Mode for TopPageMode {
    fn step(&mut self, pad: &Pad) -> Frame {
        self.frames += 1;
        self.st.pad = *pad;
        let Some(page) = &mut self.page else { return Frame::new() };
        // The event task first, as on the desktop.
        if let Some(vm) = &mut self.vm {
            vm.set_operate_set(page.state().operate_set);
            let mut h = Bridge { target: Target::TopPage(page), st: &mut self.st };
            vm.frame(&mut h);
            let s = page.state_mut();
            s.operate = vm.operate();
            s.operate_set = vm.operate_set();
        }
        let frame = match self.st.stream_frame.take() {
            Some(f) => f,
            None => page.step(pad),
        };
        self.flush();
        frame
    }

    fn take_events(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        if let Some(page) = &mut self.page {
            for r in page.take_requests() {
                match r {
                    Request::Se(n) => out.push(Event::Se(n)),
                    Request::AllSoundOff => out.push(Event::AllSoundOff),
                    // `sqDataToppage`, the only bank the page loads.
                    Request::SqLoad(_) => out.push(Event::SqLoad(piney_audio::SqContext::Toppage)),
                    Request::SqPlay(n) => out.push(Event::SqPlay(n)),
                    Request::SoundFadeOut => out.push(Event::SoundFadeOut),
                    Request::ChangeMode { num, sf } => out.push(Event::ChangeMode { num, sf }),
                    Request::ChangeArea { area, town } => out.push(Event::ChangeArea { area, town }),
                    Request::Menu(r) => out.extend(desktop::event(r)),
                    // Soft reset is not modelled.
                    Request::EnableReset(_) => {}
                }
            }
        }
        out.append(&mut self.st.events);
        out
    }

    fn frame_rate(&self) -> u32 {
        match self.st.frame_rate.filter(|_| self.st.stream.is_some()) {
            Some(r) => r,
            None => self.page.as_ref().map_or(1, |p| p.frame_rate()),
        }
    }

    fn archive(&self) -> Option<Arc<Archive>> {
        self.st.stream.as_ref().map(crate::stream::StreamPlayer::archive)
    }

    fn title(&self) -> String {
        let events = match &self.vm {
            Some(vm) => format!(" - events phase {}", vm.phase()),
            None => String::new(),
        };
        match &self.page {
            Some(p) => format!("top page - frame {} - {}{events}", self.frames, p.status()),
            None => format!("top page (abandoned by the scripts){events}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use piney_data::save::offset;
    use piney_event::state::ScriptSave as _;
    use piney_input::{Buttons, Raw};
    use piney_toppage::bbs::{POST_READ, set_bbs_state};

    use super::*;

    /// The board at event 14's start (`--mode story:14`): each post read -
    /// 3/0 (event 14 block 1), 5/0 (event 14 block 2), 9/0 (event 50 block
    /// 1), 10/1 (event 55 block 1) - plays its `gate_add_msg`: sound effect 74, the
    /// announcement (the gate address, "#B" and the server's letter before
    /// the three words, then the Word List line) in the board's message
    /// window over the fade menu until the player closes it, and the area
    /// added (`gate_add`: its three words in the word list, from its
    /// `eventAreaInfo` record) and marked on its server.
    #[test]
    fn the_boards_keyword_announcements() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let start = crate::start::build(&iso, 14).unwrap();
        let mut mode = TopPageMode::enter(iso, archive, start.state, Some(start.vm)).unwrap();
        let areas = mode.st.announcements.clone().expect("the story areas");
        let mut pad = Pad::default();
        let mut step = |mode: &mut TopPageMode, buttons: Buttons| {
            pad.read(&Raw { buttons, ..Raw::default() });
            mode.step(&pad);
            mode.take_events()
        };
        // Past the log-in animation, to the menu.
        for _ in 0..400 {
            step(&mut mode, Buttons::NONE);
        }
        for (thread, post, area) in [(3, 0, 17i16), (5, 0, 19), (9, 0, 28), (10, 1, 29)] {
            let info = areas.areas[&area];
            let shown = mode.st.announced.len();
            set_bbs_state(mode.page.as_mut().unwrap().state_mut(), thread, post, POST_READ);
            let mut events = Vec::new();
            let mut n = 0;
            while mode.st.announced.len() == shown {
                events.extend(step(&mut mode, Buttons::NONE));
                n += 1;
                assert!(n < 30, "post {thread}/{post} read: no announcement");
            }
            assert!(events.contains(&Event::Se(74)), "{events:?}");
            let lines = mode.st.announced.last().unwrap().clone();
            assert_eq!(lines.len(), 2, "area {area}");
            assert!(lines[0].starts_with(b"#B") && lines[1].starts_with(b"#W"), "area {area}");
            // The fade menu (`dtMenu +0x06 = 7`), opened by the page's menu
            // task in the same frame.
            assert_eq!(mode.page.as_ref().unwrap().menu_state(), (7, -1), "the fade menu is up");
            // Closed with a press; then the gate instructions' bookkeeping.
            let mut n = 0;
            while mode.page.as_ref().unwrap().menu_state().0 != -1 {
                step(&mut mode, if n % 20 == 10 { Buttons::CROSS } else { Buttons::NONE });
                n += 1;
                assert!(n < 200, "area {area}: the window does not close");
            }
            for _ in 0..20 {
                step(&mut mode, Buttons::NONE);
            }
            let save = &mode.page.as_ref().unwrap().state().save;
            for w in info.words {
                let w = w.expect("the area's words are in the word tables");
                let bits = save.word_at(offset::WORD_LIST, 15, w / 32).unwrap();
                assert_ne!(bits & (1 << (w % 32)), 0, "area {area}: word {w} not added");
            }
            let marks = save.word_at(offset::GATE_LIST_MARK + 20 * info.server as usize, 5, i32::from(area) / 32);
            assert_ne!(marks.unwrap() & (1 << (area % 32)), 0, "area {area} not marked");
        }
    }
}
