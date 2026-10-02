//! The ALTIMIT desktop (`piney_desktop`) as a mode, with the event scripts
//! (`piney_event`) running beside it as `ccThEvent` runs beside the desktop
//! task. `ccSetupDesktop` calls `ccStartThEvent`, then `ccEnableThEvent` 0, 2
//! and 4, a pass one event frame per game frame (a new game's event 1 plays in
//! the pass at 0). Each frame the desktop checks its operations against
//! `ccEvent.operate` (`operateSet`), then the event task passes. The scripts'
//! windows open in the desktop's message window, a setup's on the setup
//! screen. The order is in docs/engine/desktop.md.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::{SaveData, offset};
use piney_desktop::card::FilesCard;
use piney_desktop::message::MessageKind as Window;
use piney_desktop::name_entry::NameEntry;
use piney_desktop::setup::SetupScreen;
use piney_desktop::{Desktop, Request, SaveState};
use piney_draw::Frame;
use piney_event::host::{Announce, DesktopMenu, Game, Host, MessageCall, MessageKind, Place, StoryArea, Wait};
use piney_event::vm::{Library, Vm};
use piney_input::Pad;
use piney_toppage::TopPage;

use crate::mode::{Event, Mode};
use crate::story::Announcements;
use crate::stream::StreamPlayer;

/// `game.status` on the desktop.
const STATUS_DESKTOP: i32 = 2;

/// Frames a setup pass may take before the runtime gives up waiting.
const SETUP_FRAMES: u32 = 10_000;

/// The boot as `ccThMother` makes it: the scripts read from the disc, the
/// event manager initialised, and `ccStartEvent(volumeNum, 0)` on the save
/// (1 on Infection; each later volume counts the earlier ones cleared and
/// opens its own story: event 100 done on Mutation, 200 on Outbreak, 300 on
/// Quarantine).
pub fn boot(iso: &mut Iso, state: &mut SaveState) -> Result<Vm, String> {
    let volume = iso.volume().map_err(|e| e.to_string())?;
    let mut events = piney_event::official::events(iso).map_err(|e| format!("event scripts: {e}"))?;
    // The port's own additions (piney_event::extras: Helba's mail).
    piney_event::extras::apply(&mut events, volume);
    let mut vm = Vm::new(Arc::new(Library::new(volume, events)));
    vm.init();
    let mut st = HostState::default();
    let mut h = Bridge { target: Target::Setup(state, None), st: &mut st };
    vm.start_event(volume.number(), 0, &mut h);
    Ok(vm)
}

/// What the runtime answers the scripts with, apart from the save.
#[derive(Default)]
pub(crate) struct HostState {
    /// `ccGame.status` the scripts see: 2 on the desktop, 3 on the board.
    pub(crate) status: i32,
    /// Name entry, while event 1 runs it; the frame it drew this frame.
    name_entry: Option<NameEntry>,
    name_frame: Option<Frame>,
    /// Name entry is skipped, the new game's default names kept (for
    /// tests, which have their own check of it).
    pub(crate) skip_name_entry: bool,
    /// What name entry is built from.
    pub(crate) iso: PathBuf,
    pub(crate) archive: Option<Arc<Archive>>,
    /// This frame's pad (`ccSys.pad[0]`).
    pub(crate) pad: Pad,
    /// The window last opened is a camera tutorial prompt, whose voice
    /// stops when it closes.
    teach_open: bool,
    /// Lines to print.
    pub(crate) lines: VecDeque<String>,
    /// Every window opened: (event, message).
    shown: Vec<(u16, i16)>,
    /// What the scripts asked of the rest of the game.
    pub(crate) events: Vec<Event>,
    /// The stream the scripts' `stream` plays, and the frame it drew this
    /// frame.
    pub(crate) stream: Option<StreamPlayer>,
    pub(crate) stream_frame: Option<Frame>,
    /// `ccSystem::SetFrameRate` from the scripts (`frame_rate`), and the
    /// rate a setup was entered with until its own `SetFrameRate(1)`.
    pub(crate) frame_rate: Option<u32>,
    /// The story areas the gate instructions read and the text of
    /// `DispInfo`'s announcements.
    pub(crate) announcements: Option<Arc<Announcements>>,
    /// Every announcement's lines, as shown.
    pub(crate) announced: Vec<Vec<Vec<u8>>>,
}

impl HostState {
    /// The scripts' host for a mode of `ccGame.status` `status`.
    pub(crate) fn new(status: i32, iso: PathBuf, archive: Arc<Archive>, name_entry: bool) -> Self {
        let announcements = announcements(&iso);
        HostState {
            status,
            skip_name_entry: !name_entry,
            iso,
            archive: Some(archive),
            announcements,
            ..HostState::default()
        }
    }
}

/// The announcements' text from the disc at `iso`, read once.
pub(crate) fn announcements(iso: &std::path::Path) -> Option<Arc<Announcements>> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static READ: OnceLock<Mutex<HashMap<PathBuf, Arc<Announcements>>>> = OnceLock::new();
    let mut read = READ.get_or_init(Default::default).lock().ok()?;
    if let Some(a) = read.get(iso) {
        return Some(a.clone());
    }
    let made = Iso::open(iso).map_err(|e| e.to_string()).and_then(|mut disc| Announcements::from_disc(&mut disc));
    match made {
        Ok(a) => {
            let a = Arc::new(a);
            read.insert(iso.to_path_buf(), a.clone());
            Some(a)
        }
        Err(e) => {
            tracing::warn!("{e}; the gate instructions see no story areas");
            None
        }
    }
}

/// Where the scripts' effects land: the save alone during the desktop's
/// setup, the desktop (and its save) once it exists, or the board.
pub(crate) enum Target<'a> {
    /// The save, and the setup screen when the setup draws one.
    Setup(&'a mut SaveState, Option<&'a mut SetupScreen>),
    Desktop(&'a mut Desktop),
    TopPage(&'a mut TopPage),
}

/// The scripts' view of the game.
pub(crate) struct Bridge<'a> {
    pub(crate) target: Target<'a>,
    pub(crate) st: &'a mut HostState,
}

impl Bridge<'_> {
    fn desktop(&mut self) -> Option<&mut Desktop> {
        match &mut self.target {
            Target::Desktop(d) => Some(d),
            Target::Setup(..) | Target::TopPage(_) => None,
        }
    }

    fn save_ref(&self) -> &SaveData {
        match &self.target {
            Target::Setup(state, _) => &state.save,
            Target::Desktop(d) => &d.state().save,
            Target::TopPage(t) => &t.state().save,
        }
    }
}

/// A message record as text: the game's bytes, ASCII kept, anything else
/// shown as `?`.
fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '?' }).collect()
}

impl Host for Bridge<'_> {
    fn save(&mut self) -> &mut SaveData {
        match &mut self.target {
            Target::Setup(state, _) => &mut state.save,
            Target::Desktop(d) => &mut d.state_mut().save,
            Target::TopPage(t) => &mut t.state_mut().save,
        }
    }

    fn game(&self) -> Game {
        Game { status: self.st.status, ..Game::default() }
    }

    fn pad_pushed(&self) -> u32 {
        self.st.pad.push.bits()
    }

    fn desktop_menu(&self) -> DesktopMenu {
        match &self.target {
            Target::Desktop(d) => {
                let (menu_type, request) = d.menu_state();
                DesktopMenu { menu_type, request }
            }
            Target::TopPage(t) => {
                let (menu_type, request) = t.menu_state();
                DesktopMenu { menu_type, request }
            }
            Target::Setup(..) => DesktopMenu { menu_type: -1, request: -1 },
        }
    }

    fn message_open(&mut self, call: &MessageCall<'_>) {
        self.st.shown.push((call.event, call.msg));
        // `ccEvVoiceRequest` as the window opens, reading the save's
        // Voiceover option and parody flag then; the camera tutorial's
        // prompts ask with event 3.
        let save = self.save();
        let options = crate::mode::voice_options(save);
        let event = if matches!(call.kind, MessageKind::Teach(_)) { 3 } else { i32::from(call.event) };
        self.st.events.extend([options, Event::Voice { event, msg: i32::from(call.msg) }]);
        self.st.teach_open = matches!(call.kind, MessageKind::Teach(_));
        let record = call.record;
        let name = record.and_then(|m| m.name.as_ref()).map(|n| n.as_bytes());
        let lines: Vec<&[u8]> = record.map_or(Vec::new(), |m| m.lines.iter().map(|l| l.as_bytes()).collect());
        let emode = record.map_or(0, |m| m.mode);
        let window = match call.kind {
            MessageKind::Info => Window::Info,
            MessageKind::InfoNow => Window::InfoNow,
            MessageKind::Speech | MessageKind::Teach(_) => Window::Speech,
        };
        match (call.place, &mut self.target) {
            (Place::Desktop, Target::Desktop(d)) => d.open_message(window, emode, name, &lines),
            (Place::Desktop, Target::TopPage(t)) => t.open_message(window, emode, name, &lines),
            (Place::Setup, Target::Setup(_, Some(screen))) => screen.open_message(window, emode, name, &lines),
            _ => {
                let lines: Vec<String> = lines.iter().map(|l| text(l)).collect();
                let name = name.map(text).unwrap_or_default();
                self.st.lines.push_back(format!(
                    "event {} message {} {name}: {}",
                    call.event,
                    call.msg,
                    lines.join(" / ")
                ));
            }
        }
    }

    fn message_check(&mut self) -> i32 {
        let pad = self.st.pad;
        match &mut self.target {
            Target::Desktop(d) => d.message_check(&pad),
            Target::TopPage(t) => t.message_check(&pad),
            Target::Setup(_, Some(screen)) => screen.message_check(&pad),
            Target::Setup(_, None) => 1,
        }
    }

    fn message_close(&mut self) {
        if std::mem::take(&mut self.st.teach_open) {
            self.st.events.push(Event::VoiceStop);
        }
        match &mut self.target {
            Target::Desktop(d) => d.close_message(),
            Target::TopPage(t) => t.close_message(),
            Target::Setup(_, Some(screen)) => screen.close_message(),
            Target::Setup(_, None) => {}
        }
    }

    fn desktop_message_done(&mut self) {
        match &mut self.target {
            Target::Desktop(d) => d.message_done(),
            Target::TopPage(t) => t.message_done(),
            Target::Setup(..) => {}
        }
    }

    /// `ccEvent::DispInfo` on the desktop or the board: when no menu is
    /// open, the fade menu (`dtMenu +0x06 = 7`), then `ccMsg->ChangeInfo`
    /// with the lines `Execute` composed; before play, the setup screen's
    /// own window.
    fn announce(&mut self, a: Announce) {
        let lines = self.st.announcements.as_ref().map(|t| t.lines(a, self.save_ref())).unwrap_or_default();
        let refs: Vec<&[u8]> = lines.iter().map(|l| &l[..]).collect();
        match &mut self.target {
            Target::Desktop(d) => d.open_message(Window::Info, 0, None, &refs),
            Target::TopPage(t) => t.open_message(Window::Info, 0, None, &refs),
            Target::Setup(_, Some(screen)) => screen.open_message(Window::Info, 0, None, &refs),
            Target::Setup(_, None) => {
                let lines: Vec<String> = lines.iter().map(|l| text(l)).collect();
                self.st.lines.push_back(format!("event announcement {a:?}: {}", lines.join(" / ")));
            }
        }
        self.st.announced.push(lines);
    }

    /// `WORLD_MAN::GetEventAreaInfo` and `GetWordParamFromEvCode`: the
    /// gate instructions' server and words.
    fn story_area(&self, area: i16) -> Option<StoryArea> {
        self.st.announcements.as_ref().and_then(|t| t.areas.get(&area).copied())
    }

    fn desktop_menu_open(&mut self, num: i16) {
        match &mut self.target {
            Target::Desktop(d) => d.open_menu(num),
            Target::TopPage(t) => t.open_menu(num),
            Target::Setup(..) => {}
        }
    }

    /// `staff_roll` (case 147): the menus forbidden and `ccThStaffRoll`
    /// started; [`Host::busy`] waits for it.
    fn begin(&mut self, w: Wait) {
        if w == Wait::StaffRoll
            && let Some(d) = self.desktop()
        {
            // game.pauseFlag 1, dtMenu's forbid, ccSleepAllThread(): the
            // roll runs over nothing.
            d.set_menu_forbid(true);
            d.set_slept(true);
            if let Err(e) = d.start_staff_roll() {
                tracing::warn!("the staff roll: {e}");
            }
        }
    }

    /// Nothing is restored after a wait on the desktop (the staff roll's
    /// menus come back with `staff_roll_done`).
    fn end(&mut self, _w: Wait) {}

    /// `ccSndBgmCtrl` (the staff roll's step 5): the desktop's case plays
    /// sequence 1 for `dtBgm` 27 and 7, else 0, unless `sound 10` held it
    /// (piney-audio's driver keeps the loaded bank and the hold).
    fn bgm_control(&mut self) {
        let save = self.save().clone();
        self.st.events.push(Event::BgmCtrl(piney_audio::BgmWorld {
            scene_replaced: false,
            town: 0,
            crisis: save.u8(offset::CRISIS) != 0,
            dt_bgm: i32::from(save.u8(offset::DT_BGM) as i8),
        }));
    }

    /// The end of `staff_roll`: `ccWakeAllThread()` and dtMenu's forbid
    /// off.
    fn staff_roll_done(&mut self) {
        if let Some(d) = self.desktop() {
            d.set_slept(false);
            d.set_menu_forbid(false);
        }
    }

    /// `virus_core`'s `SimGenerateCode` while a volume's events are brought
    /// forward (level 1, Mutation's boot pass over Infection's events): it
    /// leaves `WORLD_MAN`'s area, which only a field's Area Information
    /// shows, and entering any area (or the Chaos Gate's own simulation)
    /// sets it anew first. Nothing here shows it.
    fn generate_area(&mut self, _words: [Option<i32>; 3]) {}

    /// Mutation's `grunty_mail` (case 168, MUT main 0x001c77a8), in M201's
    /// set-up: unless mail 324 was ever delivered (`mailList[324]`), it
    /// arrives once some town 1-4 has a grown Grunty in each of its three
    /// pens (`ccPgAdultCheck(town, 0..2)` not negative).
    fn grunty_mail(&mut self) {
        const MAIL: usize = 324;
        let save = self.save();
        if save.mail(MAIL) != 0 {
            return;
        }
        if (1..=4).any(|town| (0..3).all(|pen| piney_battle::ride::adult_check(save, town, pen) >= 0)) {
            save.new_mail(MAIL);
        }
    }

    fn name_entry_start(&mut self) {
        if self.st.skip_name_entry {
            return;
        }
        let Target::Setup(state, _) = &mut self.target else { return };
        let Some(archive) = self.st.archive.clone() else { return };
        let made = Iso::open(&self.st.iso)
            .map_err(|e| e.to_string())
            .and_then(|mut disc| NameEntry::new(&mut disc, archive, state).map_err(|e| e.to_string()));
        match made {
            Ok(n) => self.st.name_entry = Some(n),
            Err(e) => tracing::warn!("name entry: {e}; keeping the default name"),
        }
    }

    fn name_entry_step(&mut self) -> bool {
        let Target::Setup(state, _) = &mut self.target else { return true };
        let Some(n) = &mut self.st.name_entry else { return true };
        let pad = self.st.pad;
        self.st.name_frame = Some(n.step(&pad, state));
        for r in n.take_requests() {
            if let Request::Se(se) = r {
                self.st.events.push(Event::Se(se.0));
            }
        }
        n.done()
    }

    fn name_entry_end(&mut self) {
        self.st.name_entry = None;
        self.st.name_frame = None;
        // The lines after it name the player as entered.
        if let Target::Setup(state, Some(screen)) = &mut self.target {
            screen.update(state);
        }
    }

    fn change_request(&mut self, num: i32, sf: i32) {
        self.st.events.push(Event::ChangeMode { num, sf });
    }

    fn sound_effect(&mut self, se: i32) {
        self.st.events.push(Event::Se(se));
    }

    /// `overlay`: the game loads DEMO.PRG, DESKTOP.PRG, TOPPAGE.PRG or
    /// GCMN.PRG over its overlay region before a mode change. The port
    /// keeps every overlay's code and data at once, so there is nothing to
    /// load.
    fn load_overlay(&mut self, _which: piney_event::host::Overlay) {}

    /// The phase-4 pass's end: the game sets `ccMenu +0x104` while
    /// `WORLD_MAN +0xf0` is 3. `ccMenu` is the field's menu, which does not
    /// run on the desktop, so nothing here reads it.
    fn play_pass_done(&mut self) {}

    /// `ccClearGtHack` (0x001b77c0), from `ccStartThEvent`: `gtHackFlag`
    /// cleared (it is kept only in a field or dungeon entered from a town).
    /// The port sets the flag nowhere yet, so there is nothing to clear.
    fn clear_gate_hack(&mut self) {}

    /// `ccEventStream(num, 1)`: `ccRequestLoadStream` plays the stream
    /// inside the call; here it starts, and [`Host::busy`] steps it a frame
    /// at a time. A stream not on this disc counts as played.
    fn stream(&mut self, num: i16) {
        let iso = self.st.iso.clone();
        let save = self.save().clone();
        let game = piney_audio::stream::StreamGame { status: self.st.status, field: 0 };
        let data = self.st.archive.clone();
        let started = usize::try_from(num)
            .map_err(|_| format!("stream {num}"))
            .and_then(|n| StreamPlayer::event(&iso, data.as_deref(), n, &save, game, &mut self.st.events));
        match started {
            Ok(p) => self.st.stream = Some(p),
            Err(e) => tracing::warn!("events: {e}; counted as played"),
        }
    }

    /// The stream plays while the call waits: one frame of it per game
    /// frame.
    fn busy(&mut self, w: Wait) -> bool {
        if w == Wait::StaffRoll {
            return self.desktop().is_some_and(|d| d.staff_roll_running());
        }
        let st = &mut *self.st;
        if w != Wait::Stream {
            return false;
        }
        let Some(p) = &mut st.stream else { return false };
        match p.step(&st.pad, &mut st.events) {
            Some(f) => {
                st.stream_frame = Some(f);
                true
            }
            None => {
                st.stream = None;
                false
            }
        }
    }

    /// `ccSystem::SetFrameRate(rate)`: vertical blanks per game frame.
    fn set_frame_rate(&mut self, rate: i16) {
        let rate = u32::try_from(rate).unwrap_or(1).max(1);
        self.st.frame_rate = Some(rate);
        // The staff roll's `SetFrameRate(2)` (everything but the menu
        // asleep) holds for the save menus it opens.
        if let Target::Desktop(d) = &mut self.target
            && d.slept()
        {
            d.set_frame_rate(rate);
        }
    }

    /// `ccSndEvRequest` ([`crate::mode::sound_request`]).
    fn sound(&mut self, cmd: i16, p0: i16, p1: i16, p2: i16) {
        self.st.lines.push_back(format!("event sound request {cmd} ({p0}, {p1}, {p2})"));
        self.st.events.extend(crate::mode::sound_request(cmd, p0, p1, p2));
    }
}

/// Where the mode is.
enum Stage {
    /// `ccSetupDesktop`'s waits: the event task's pass at `phase` (0, then
    /// 2) runs one event frame per game frame on the save until it settles.
    Setup {
        save: SaveState,
        phase: i32,
        frames: u32,
        /// Black, and the event's own window when it opens one.
        screen: Box<SetupScreen>,
    },
    Play(Box<Desktop>),
    /// Only while changing over.
    Gone,
}

pub struct DesktopMode {
    stage: Stage,
    /// The event scripts, when they run.
    vm: Option<Vm>,
    st: HostState,
    frames: u64,
    /// What the desktop is built from when the setup ends.
    iso: PathBuf,
    archive: Arc<Archive>,
    card: Option<PathBuf>,
    /// `ccSaveSys`'s port and file for the Data screen, when the title
    /// left them somewhere.
    card_position: Option<(i32, i32)>,
    /// A movie of the Audio screen (`SimplePlayStream`): an in-engine
    /// stream, with every desktop task asleep; and the sequence slot
    /// `ccSndMoviePlayer` stopped for it.
    movie: Option<(StreamPlayer, i32)>,
    /// A script asked for another mode during a setup pass: `ccSetupDesktop`
    /// returns after that wait (it tests `game +0x04`), with no more passes,
    /// no `SetFrameRate(1)` and no desktop.
    abandoned: bool,
}

impl DesktopMode {
    /// The desktop on `state`; with `scripts`, the official event scripts
    /// from the disc run from the boot on, as for a new game.
    pub fn new(
        iso: PathBuf,
        archive: Arc<Archive>,
        mut state: SaveState,
        scripts: bool,
        card: Option<PathBuf>,
        name_entry: bool,
    ) -> Result<Self, String> {
        let vm = if scripts {
            let mut disc = Iso::open(&iso).map_err(|e| format!("{}: {e}", iso.display()))?;
            Some(boot(&mut disc, &mut state)?)
        } else {
            None
        };
        DesktopMode::enter(iso, archive, state, vm, card, name_entry)
    }

    /// `ccSetupDesktop` on a running game. With the event task (started at
    /// boot) it makes its passes at phases 0 and 2 first, one frame at a
    /// time, and the desktop is built on the save they leave; without it the
    /// desktop is built at once. `card` is the directory that stands for
    /// memory card slot 1 (its `BASLUS-20267DOTHACK` directory and slot
    /// files, as the Data screen writes them); None for no card.
    pub fn enter(
        iso: PathBuf,
        archive: Arc<Archive>,
        mut state: SaveState,
        mut vm: Option<Vm>,
        card: Option<PathBuf>,
        name_entry: bool,
    ) -> Result<Self, String> {
        let mut st = HostState::new(STATUS_DESKTOP, iso.clone(), archive.clone(), name_entry);
        if let Some(vm) = &mut vm {
            let mut h = Bridge { target: Target::Setup(&mut state, None), st: &mut st };
            vm.start_thread(&mut h);
            vm.enable(0);
        }
        let scripts = vm.is_some();
        // `ccAllSoundOff` (0x0016847c) comes after the event task's phase-0
        // pass and `DESKTOP.PRG`'s load ([`Self::setup_frame`]); without the
        // scripts there is no pass to wait for.
        if !scripts {
            st.events.push(Event::AllSoundOff);
        }
        let mut mode = DesktopMode {
            stage: Stage::Gone,
            vm,
            st,
            frames: 0,
            iso,
            archive,
            card,
            card_position: None,
            movie: None,
            abandoned: false,
        };
        mode.stage = if scripts {
            let mut disc = Iso::open(&mode.iso).map_err(|e| format!("{}: {e}", mode.iso.display()))?;
            let screen =
                SetupScreen::new(&mut disc, mode.archive.clone(), &state).map_err(|e| format!("setup: {e}"))?;
            Stage::Setup { save: state, phase: 0, frames: 0, screen: Box::new(screen) }
        } else {
            Stage::Play(Box::new(mode.build(state)?))
        };
        mode.flush();
        Ok(mode)
    }

    /// The desktop itself, on `state`.
    fn build(&self, state: SaveState) -> Result<Desktop, String> {
        let mut disc = Iso::open(&self.iso).map_err(|e| format!("{}: {e}", self.iso.display()))?;
        let mut desktop = Desktop::new(&mut disc, self.archive.clone(), state).map_err(|e| format!("desktop: {e}"))?;
        if let Some(dir) = &self.card {
            let volume = disc.volume().map_err(|e| e.to_string())?;
            desktop.set_card(Box::new(FilesCard::slot1(volume, dir.clone())));
        }
        if let Some((port, file)) = self.card_position {
            desktop.set_card_position(port, file);
        }
        Ok(desktop)
    }

    /// Where the Data screen's card cursor starts: the title's `ccSaveSys`
    /// is the desktop's too.
    pub fn set_card_position(&mut self, (port, file): (i32, i32)) {
        self.card_position = Some((port, file));
        if let Stage::Play(desktop) = &mut self.stage {
            desktop.set_card_position(port, file);
        }
    }

    /// `ccSystem`'s frame rate as the mode before left it: `ccSetupDesktop`
    /// does not set one until its `SetFrameRate(1)` after the pass at phase
    /// 2, so a setup entered from The World runs at 2 (30 frames a second),
    /// its passes' windows and streams with it (event 4's streams 4-6).
    pub fn carry_frame_rate(&mut self, rate: u32) {
        if matches!(self.stage, Stage::Setup { .. }) {
            self.st.frame_rate = Some(rate.max(1));
        }
    }

    /// One frame of the setup: the setup screen (black, and the event's
    /// window when one is up), the event task's pass, then, when it has
    /// settled, the next wait or the desktop.
    fn setup_frame(&mut self, pad: &Pad) -> Result<Frame, String> {
        let Stage::Setup { save, phase, frames, screen } = &mut self.stage else { return Ok(Frame::new()) };
        let Some(vm) = &mut self.vm else { return Ok(Frame::new()) };
        if self.abandoned {
            return Ok(Frame::new());
        }
        let mut frame = screen.step(pad);
        let mut h = Bridge { target: Target::Setup(save, Some(&mut **screen)), st: &mut self.st };
        vm.frame(&mut h);
        // A stream the scripts play draws instead.
        if let Some(f) = self.st.stream_frame.take() {
            frame = f;
        }
        // Name entry draws over the setup's black while event 1 runs it.
        if let Some(f) = self.st.name_frame.take()
            && self.st.name_entry.is_some()
        {
            frame = f;
        }
        for r in screen.take_requests() {
            match r {
                Request::Se(se) => self.st.events.push(Event::Se(se.0)),
                Request::VoiceStop => self.st.events.push(Event::VoiceStop),
                _ => {}
            }
        }
        *frames += 1;
        if !vm.enable_settled(*phase) && *frames < SETUP_FRAMES {
            return Ok(frame);
        }
        tracing::debug!("events: the pass at phase {phase} took {frames} frames");
        // After each wait `ccSetupDesktop` returns if a mode change was asked
        // (event 4's `mode 3` at +0x02d8, in the pass at phase 2).
        if self.st.events.iter().any(|e| matches!(e, Event::ChangeMode { .. })) {
            self.abandoned = true;
            return Ok(frame);
        }
        if *phase == 0 {
            // `ccSetupDesktop` (0x00168320): the phase-0 pass (New Game's name
            // entry among it) still sounds on the last mode's ports; then
            // `ccAllSoundOff`, which zeroes every port's volume until the
            // desktop's bank loads, and the pass at phase 2.
            self.st.events.push(Event::AllSoundOff);
            vm.enable(2);
            (*phase, *frames) = (2, 0);
            return Ok(frame);
        }
        vm.enable(4);
        // `ccSystem::SetFrameRate(1)` (0x001684dc), after the load: until
        // here the setup ran at the rate it was entered with.
        self.st.frame_rate = Some(piney_desktop::FRAME_RATE);
        let Stage::Setup { save, .. } = std::mem::replace(&mut self.stage, Stage::Gone) else { unreachable!() };
        self.stage = Stage::Play(Box::new(self.build(save)?));
        Ok(frame)
    }

    /// The save the desktop holds, for the main loop's play time.
    pub fn save_mut(&mut self) -> Option<&mut SaveData> {
        match &mut self.stage {
            Stage::Play(d) => Some(&mut d.state_mut().save),
            Stage::Setup { save, .. } => Some(&mut save.save),
            Stage::Gone => None,
        }
    }

    /// Leave the desktop: the save and the event task, for the next mode.
    /// `ccGame::ChangeRequest` disables the task's phase.
    pub fn leave(self) -> (SaveState, Option<Vm>) {
        let DesktopMode { stage, mut vm, .. } = self;
        if let Some(vm) = &mut vm {
            vm.disable();
        }
        let state = match stage {
            Stage::Play(desktop) => desktop.state().clone(),
            Stage::Setup { save, .. } => save,
            Stage::Gone => SaveState::fresh(),
        };
        (state, vm)
    }

    /// Whether the setup is over and the desktop runs.
    #[cfg(test)]
    pub(crate) fn playing(&self) -> bool {
        matches!(self.stage, Stage::Play(_))
    }

    /// The event task, for inspection.
    #[cfg(test)]
    pub(crate) fn vm(&self) -> Option<&Vm> {
        self.vm.as_ref()
    }

    /// The desktop, once the setup is over.
    #[cfg(test)]
    pub(crate) fn desktop(&self) -> Option<&Desktop> {
        match &self.stage {
            Stage::Play(d) => Some(d),
            _ => None,
        }
    }

    /// Whether New Game's name entry is up.
    #[cfg(test)]
    pub(crate) fn naming(&self) -> bool {
        self.st.name_entry.is_some()
    }

    /// Whether a stream plays: the scripts' or an Audio screen movie.
    #[cfg(test)]
    pub(crate) fn streaming(&self) -> bool {
        self.movie.is_some() || self.st.stream.is_some()
    }

    /// Print what the scripts showed.
    fn flush(&mut self) {
        for line in self.st.lines.drain(..) {
            tracing::debug!("{line}");
        }
    }
}

/// The sequence `ccSndMoviePlayer` (0x00180b30) stops for a movie and plays
/// again after it: slot 1 under the desktop music 27 or 7, else slot 0.
/// Stopping is `ccSqStop`'s (sequencer off, its fade off, status 0), only
/// when that slot plays; playing again is `ccSqPlay`'s from tick 0.
fn movie_sq_slot(bgm: usize) -> i32 {
    if bgm == 27 || bgm == 7 { 1 } else { 0 }
}

/// A request of the desktop or its system menu, for the runtime.
pub fn event(r: Request) -> Option<Event> {
    match r {
        Request::Se(se) => Some(Event::Se(se.0)),
        Request::Bgm(n) => Some(Event::DesktopBgm(n)),
        Request::SoundFadeOut => Some(Event::SoundFadeOut),
        Request::ChangeMode { num, sf } => Some(Event::ChangeMode { num, sf }),
        Request::ChangeBgm { wave, .. } => Some(Event::Bgm(wave)),
        Request::Movie { stream, .. } => Some(Event::Movie(stream)),
        Request::SoundEnv { main, bgm, se, output } => Some(Event::Volumes { main, se, bgm, output }),
        Request::VoiceStop => Some(Event::VoiceStop),
        Request::BgmStream(n) => usize::try_from(n).ok().map(Event::BgmStream),
        Request::BgmStreamStop => Some(Event::BgmStreamStop),
        // Switched on, the pad buzzes once (the switch itself is the
        // save's, which the runtime reads: `Mode::vibration`).
        Request::Vibration { on } => on.then_some(Event::Actuate { small: true, power: 160, ms: 200 }),
        Request::DisplayOffset { x, y } => Some(Event::DisplayOffset { x, y }),
        // The field's camera scheme has nothing to act on yet.
        Request::CameraType(_) | Request::EnableReset(_) => None,
    }
}

impl Mode for DesktopMode {
    fn step(&mut self, pad: &Pad) -> Frame {
        self.frames += 1;
        self.st.pad = *pad;
        let frame = match &mut self.stage {
            Stage::Play(desktop) => 'play: {
                // The Audio screen's movie: the desktop and the event task
                // sleep until it ends.
                if let Some((m, slot)) = &mut self.movie {
                    match m.step(pad, &mut self.st.events) {
                        Some(f) => break 'play f,
                        None => {
                            // `ccSndMoviePlayer(1, bgm)`: the music again
                            // from the start.
                            self.st.events.push(Event::SqPlay(*slot));
                            self.movie = None;
                        }
                    }
                }
                // The event task first: its pass sees the operation the
                // desktop recorded last frame, then clears it for this one.
                if let Some(vm) = &mut self.vm {
                    vm.set_operate_set(desktop.state().operate_set);
                    let mut h = Bridge { target: Target::Desktop(desktop), st: &mut self.st };
                    vm.frame(&mut h);
                    let s = desktop.state_mut();
                    s.operate = vm.operate();
                    s.operate_set = vm.operate_set();
                }
                // A stream the scripts play holds the desktop too.
                if let Some(f) = self.st.stream_frame.take() {
                    break 'play f;
                }
                desktop.step(pad)
            }
            _ => self.setup_frame(pad).unwrap_or_else(|e| {
                tracing::warn!("{e}");
                Frame::new()
            }),
        };
        self.flush();
        frame
    }

    fn take_events(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        if let Stage::Play(desktop) = &mut self.stage {
            for r in desktop.take_requests() {
                match r {
                    // `SimplePlayStream`: the movie is a stream, played
                    // between `ccSndMoviePlayer(0, bgm)` and `(1, bgm)`.
                    Request::Movie { stream, bgm } => {
                        let save = desktop.state().save.clone();
                        let started = usize::try_from(stream)
                            .map_err(|_| format!("stream {stream}"))
                            .and_then(|n| StreamPlayer::start(&self.iso, n, &save, false, &mut out));
                        match started {
                            Ok(p) => {
                                let slot = movie_sq_slot(bgm);
                                out.push(Event::SqStop(slot));
                                self.movie = Some((p, slot));
                            }
                            Err(e) => tracing::warn!("{e}"),
                        }
                    }
                    r => out.extend(event(r)),
                }
            }
        }
        out.append(&mut self.st.events);
        out
    }

    fn frame_rate(&self) -> u32 {
        match &self.stage {
            // `SimplePlayStream` sets rate 2 for its movie.
            Stage::Play(_) if self.movie.is_some() => 2,
            Stage::Play(desktop) => {
                self.st.frame_rate.filter(|_| self.st.stream.is_some()).unwrap_or(desktop.frame_rate())
            }
            _ => self.st.frame_rate.unwrap_or(piney_desktop::FRAME_RATE),
        }
    }

    fn archive(&self) -> Option<Arc<Archive>> {
        self.movie.as_ref().map(|m| &m.0).or(self.st.stream.as_ref()).map(StreamPlayer::archive)
    }

    fn title(&self) -> String {
        let events = match &self.vm {
            Some(vm) => format!(" - events phase {}", vm.phase()),
            None => String::new(),
        };
        let stream = match self.movie.as_ref().map(|m| &m.0).or(self.st.stream.as_ref()) {
            Some(p) => format!(" - {}", p.status()),
            None => String::new(),
        };
        match &self.stage {
            Stage::Play(desktop) => {
                let roll = if desktop.staff_roll_running() { " - staff roll" } else { "" };
                format!("desktop - frame {} - icon {}{events}{stream}{roll}", self.frames, desktop.selection())
            }
            _ => format!("desktop setup - frame {}{events}{stream}", self.frames),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use piney_input::{Buttons, Raw};

    use super::*;

    /// FNV-1a, as `tools/test_desktop_announce_rs.py` hashes the lines.
    fn fnv(b: &[u8]) -> u32 {
        b.iter().fold(0x811c_9dc5u32, |h, &c| (h ^ u32::from(c)).wrapping_mul(0x0100_0193))
    }

    /// The Bridge as the harness's `Execute` sees the rest of the game: the
    /// desktop menu's type fixed, `ccMessage::Check` 0 until poll `poll`,
    /// every call logged with its frame.
    struct Rec<'a> {
        b: Bridge<'a>,
        menu: i32,
        poll: u32,
        polls: u32,
        frame: u32,
        log: Vec<(&'static str, u32, i32)>,
    }

    impl Host for Rec<'_> {
        fn save(&mut self) -> &mut SaveData {
            self.b.save()
        }
        fn game(&self) -> Game {
            self.b.game()
        }
        fn story_area(&self, area: i16) -> Option<StoryArea> {
            self.b.story_area(area)
        }
        fn desktop_menu(&self) -> DesktopMenu {
            DesktopMenu { menu_type: self.menu, request: -1 }
        }
        fn sound_effect(&mut self, se: i32) {
            self.log.push(("se", self.frame, se));
        }
        fn announce(&mut self, a: Announce) {
            self.log.push(("change", self.frame, 0));
            self.b.announce(a);
        }
        fn message_check(&mut self) -> i32 {
            self.log.push(("check", self.frame, 0));
            self.polls += 1;
            i32::from(self.polls >= self.poll)
        }
        fn message_close(&mut self) {
            self.log.push(("close", self.frame, 0));
        }
    }

    /// Every case of `crates/piney-game/tests/announce_fixture.txt`, the
    /// game's own `ccEvent::Execute` of `gate_add_msg`, `member_add_msg` and
    /// `desktop_item` on the desktop and the board (tools/
    /// test_desktop_announce_rs.py), replayed through the port's event task
    /// with the Bridge and its announcements on a new game's save: the
    /// sounds, each line `DispInfo` hands `ChangeInfo` (length and hash), the
    /// frames of the window, its polls, its close and the instruction's end,
    /// and every save byte it changed.
    #[test]
    fn announcements_match_the_game() {
        use piney_event::ir::{Block, Event as Script, GameText, Op};
        use piney_event::state::ScriptSave as _;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let iso = root.join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let text = std::fs::read_to_string(root.join("tests/announce_fixture.txt")).unwrap();
        let mut disc = Iso::open(&iso).unwrap();
        let base = crate::world::new_game_state(&mut disc).unwrap();
        let ann = Arc::new(Announcements::from_disc(&mut disc).unwrap());
        let mut n = 0;
        for line in text.lines().filter(|l| l.starts_with("announce ")) {
            let parts: Vec<Vec<&str>> = line.split(" | ").map(|p| p.split_whitespace().collect()).collect();
            let num = |s: &str| s.parse::<i64>().unwrap();
            let head = &parts[0];
            let (kind, status, menu, poll) = (head[1], num(head[2]) as i32, num(head[3]) as i32, num(head[4]) as u32);
            let (a, b) = (num(head[5]) as i16, num(head[6]) as i16);
            let op = match kind {
                "gate" => Op::GateAddMsg { area: a },
                "member" => Op::MemberAddMsg { pc: a },
                _ => Op::DesktopItem { ty: a, id: b },
            };
            let script = piney_event::ir::Script {
                open: Vec::new(),
                blocks: vec![Block { tags: Vec::new(), conds: Vec::new(), ops: vec![op] }],
            };
            let event =
                Script { number: 1, label: GameText::default(), script, messages: Vec::new(), parody: Vec::new() };
            let mut vm = Vm::new(Arc::new(Library::new(piney_data::volume::Volume::Inf, [event])));
            let mut state = base.clone();
            let mut st = HostState { status, announcements: Some(ann.clone()), ..HostState::default() };
            let mut h = Rec {
                b: Bridge { target: Target::Setup(&mut state, None), st: &mut st },
                menu,
                poll,
                polls: 0,
                frame: 0,
                log: Vec::new(),
            };
            // Play (phase 4, then 5): the pass walks event 1 and plays it.
            vm.enable(4);
            let mut end = None;
            for f in 0..200 {
                h.frame = f;
                vm.frame(&mut h);
                if end.is_none() && h.b.save_ref().flags(1) & 1 != 0 {
                    end = Some(f);
                }
            }
            let first = h.log.first().map_or(0, |e| e.1);
            let at = |w: &str| h.log.iter().filter(|e| e.0 == w).map(|e| (e.1 - first).to_string()).collect::<Vec<_>>();
            let se: Vec<String> = h.log.iter().filter(|e| e.0 == "se").map(|e| e.2.to_string()).collect();
            assert_eq!(se, parts[1][1..], "{line}: sounds");
            let lines = h.b.st.announced.last().cloned().unwrap_or_default();
            let got: Vec<String> = (0..4)
                .flat_map(|i| match lines.get(i) {
                    Some(l) => [l.len().to_string(), fnv(l).to_string()],
                    None => ["-1".to_string(), "0".to_string()],
                })
                .collect();
            assert_eq!(got, parts[2][1..], "{line}: lines");
            let mut frames = [at("change"), at("check"), at("close")].concat();
            frames.push((end.expect("the instruction ends") - first).to_string());
            assert_eq!(frames, parts[3][1..], "{line}: frames");
            // DispInfo on the desktop: the fade menu asked for when none is
            // open, `dtMenu +0x10` set after the close (the port's
            // `dtmenu::open_message` and `desktop_message_done`).
            let req = if menu == -1 { "7" } else { "-1" };
            assert_eq!(parts[4][1..], [req, "1"], "{line}: dtMenu");
            let after = h.b.save_ref().bytes().to_vec();
            let diff: Vec<String> = base
                .save
                .bytes()
                .iter()
                .zip(&after)
                .enumerate()
                .filter(|(_, (x, y))| x != y)
                .flat_map(|(i, (_, y))| [i.to_string(), y.to_string()])
                .collect();
            assert_eq!(diff, parts[5][1..], "{line}: the save");
            n += 1;
        }
        assert_eq!(n, 159);
    }

    /// The Audio screen's Movie 01 plays as a stream with the desktop held:
    /// `ccSndMoviePlayer` stops the music's sequence as it starts and plays
    /// it again once the movie is skipped, and the desktop takes over again.
    #[test]
    fn audio_movie_plays_as_a_stream() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        // Movies 1 and 2 unlocked (`dtStrList`), and the game cleared so the
        // first may be watched.
        let mut state = SaveState::fresh();
        let at = offset::DT_STR_LIST;
        state.save.set_i32(at, state.save.i32(at) | 0b11);
        state.save.set_u8(offset::CLEAR_FLAG, 1);
        let mut mode = DesktopMode::new(iso, archive, state, false, None, false).unwrap();
        let mut pad = Pad::default();
        // Down to AUDIO, OK; down to Movie, OK; OK on Movie 01.
        let presses = [
            (500, Buttons::DOWN),
            (510, Buttons::DOWN),
            (520, Buttons::DOWN),
            (540, Buttons::CROSS),
            (640, Buttons::DOWN),
            (660, Buttons::CROSS),
            (720, Buttons::CROSS),
        ];
        let mut events = Vec::new();
        let mut f = 0;
        while !mode.streaming() {
            let buttons = presses.iter().filter(|p| p.0 == f).fold(Buttons::NONE, |b, p| b | p.1);
            pad.read(&Raw { buttons, ..Raw::default() });
            mode.step(&pad);
            events.extend(mode.take_events());
            f += 1;
            assert!(f < 900, "no movie: {}", mode.title());
        }
        assert!(events.contains(&Event::SqStop(0)), "{events:?}");
        assert_eq!(mode.frame_rate(), 2);
        let drawn = (0..60)
            .filter(|_| {
                pad.read(&Raw::default());
                let frame = mode.step(&pad);
                mode.take_events();
                !frame.cmds.is_empty()
            })
            .count();
        assert!(drawn > 30, "the movie drew {drawn} of 60 frames");
        // Cancel and START until it ends.
        let mut events = Vec::new();
        let mut n = 0;
        while mode.streaming() {
            let buttons = if n % 30 == 29 { Buttons::CIRCLE | Buttons::START } else { Buttons::NONE };
            pad.read(&Raw { buttons, ..Raw::default() });
            mode.step(&pad);
            events.extend(mode.take_events());
            n += 1;
            assert!(n < 20_000, "the movie does not end: {}", mode.title());
        }
        mode.step(&pad);
        events.extend(mode.take_events());
        assert!(events.contains(&Event::SqPlay(0)), "{events:?}");
        assert_eq!(mode.frame_rate(), piney_desktop::FRAME_RATE);
        assert!(mode.title().starts_with("desktop - "), "{}", mode.title());
    }

    /// The ML events are in Infection's script set and open in its play:
    /// ML-BLACKROSE-01 (event 401) opens once event 14 is done, on the
    /// desktop (`game_status` 2), and with Black Rose's friendship (row 15)
    /// at 50 its first block, which waits for the desktop's set-up
    /// (`phase >= 4`), sends mail 85. (With event 15 open, its block 2
    /// holds the pass, so every Infection event is marked done here.)
    #[test]
    fn black_rose_writes_when_friendly() {
        use piney_event::state::{DONE, ScriptSave as _};
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let run = |friendship: i16| {
            let mut state = SaveState::fresh();
            // Infection's story and side events all done, so none holds the
            // pass that walks the ML events after them.
            for e in 0..100 {
                state.save.update_flags(e, |f| f | DONE);
            }
            state.save.set_friendship(15, friendship);
            let mut mode = DesktopMode::new(iso.clone(), archive.clone(), state, true, None, false).unwrap();
            let mut pad = Pad::default();
            for f in 0..600 {
                let buttons = if f % 30 == 29 { Buttons::CROSS } else { Buttons::NONE };
                pad.read(&Raw { buttons, ..Raw::default() });
                mode.step(&pad);
            }
            assert!(mode.playing(), "the desktop is set up");
            match &mode.stage {
                Stage::Play(d) => d.state().save.mail(85),
                _ => unreachable!(),
            }
        };
        // Delivered (1), and seen by the desktop's mailer icon (2).
        assert_eq!(run(50), 2, "mail 85 delivered");
        assert_eq!(run(49), 0, "not below 50");
    }

    /// A new game on the desktop, the scripts running: event 1 delivers
    /// mails 4, 5 and 320 and locks every icon but the mailer; trying NEWS
    /// is refused; reading the three mails releases the locks.
    #[test]
    fn new_game_events_run_on_the_desktop() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let iso_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let mut mode = DesktopMode::new(iso_path, archive, SaveState::fresh(), true, None, false).unwrap();
        // The setup: event 1's pass at phase 0 - streams 2 and 106, skipped
        // with cancel or START; two lines that wait for a press each - then
        // phase 2.
        let mut setup = 0;
        let mut pad = Pad::default();
        while !mode.playing() {
            let buttons = match setup % 30 {
                29 if mode.streaming() => Buttons::CIRCLE | Buttons::START,
                29 => Buttons::CROSS,
                _ => Buttons::NONE,
            };
            pad.read(&Raw { buttons, ..Raw::default() });
            mode.step(&pad);
            setup += 1;
            assert!(setup < 2000, "the setup does not end");
        }
        assert!(setup > 61, "{setup} frames: the lines waited for no one");
        fn desktop(m: &DesktopMode) -> &Desktop {
            match &m.stage {
                Stage::Play(d) => d,
                _ => unreachable!(),
            }
        }
        let save = &desktop(&mode).state().save;
        assert!([4, 5, 320].iter().all(|&m| save.mail(m) == 1), "mail delivered");
        let locked = mode.vm.as_ref().unwrap().operate();
        assert_ne!(locked, 0);

        let mut run = |mode: &mut DesktopMode, frames: std::ops::Range<u32>, presses: &[(u32, Buttons)]| {
            for f in frames {
                let buttons = presses.iter().filter(|p| p.0 == f).fold(Buttons::NONE, |b, p| b | p.1);
                pad.read(&Raw { buttons, ..Raw::default() });
                mode.step(&pad);
            }
        };
        // NEWS while locked: event 1 block 1 opens its window on the desktop
        // (Kite's line that the mail comes first): a first Cross finishes the typing,
        // a second closes it.
        run(&mut mode, 0..460, &[(400, Buttons::DOWN), (430, Buttons::CROSS)]);
        assert_eq!(mode.vm.as_ref().unwrap().operate(), locked);
        assert!(mode.st.shown.contains(&(1, 0)), "{:?}", mode.st.shown);
        assert_ne!(desktop(&mode).menu_state().0, -1, "the window is open");
        run(&mut mode, 460..510, &[(460, Buttons::CROSS), (490, Buttons::CROSS)]);
        assert_eq!(desktop(&mode).menu_state().0, -1, "the window has closed");
        // Up to the mailer, open it and read the four mails (the game's
        // three and the port's from Helba, piney_event::extras, on top).
        let (c, o, d) = (Buttons::CROSS, Buttons::CIRCLE, Buttons::DOWN);
        let reads = [
            (510, Buttons::UP),
            (530, c),
            (570, c),
            (620, o),
            (640, d),
            (660, c),
            (710, o),
            (730, d),
            (750, c),
            (800, o),
            (820, d),
            (840, c),
            (890, o),
            (920, o),
        ];
        run(&mut mode, 510..980, &reads);
        let save = &desktop(&mode).state().save;
        let helba = piney_event::extras::HELBA_MAIL as usize;
        assert!([4, 5, 320, helba].iter().all(|&m| save.mail(m) >= 4), "mail read");
        assert_eq!(mode.vm.as_ref().unwrap().operate(), 0, "locks released");
    }
}
