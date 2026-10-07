//! The game from power-on, as `ccThMother` runs it: the boot, then one mode
//! at a time (2 the title, 3 the desktop, 4 the top page, 5 and 6 The World,
//! 1 a reset), each asking for the next with `ccGame::ChangeRequest`. The
//! save and the event task are handed from mode to mode, as are `ccGame`'s
//! scene, `WORLD_MAN`'s area and `ccSystem`'s frame rate. The modes and their
//! overlays are in docs/engine/overview.md.

use std::path::PathBuf;
use std::sync::Arc;

use crate::movie::{Played, Playing};
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::{SaveData, offset};
use piney_demo::{Config, Demo, Request};
use piney_desktop::card::{FilesCard, MemoryCard, NoCard};
use piney_desktop::{InitText, SaveState};
use piney_draw::Frame;
use piney_event::vm::Vm;
use piney_input::Pad;
use piney_world::Arrival;

use crate::area::AreaMode;
use crate::desktop::{self, DesktopMode};
use crate::mode::{Event, Mode};
use crate::stream::StreamPlayer;
use crate::toppage::TopPageMode;
use crate::world::WorldMode;

/// `ccGame::ChangeRequest` numbers.
mod request {
    pub const RESET: i32 = 1;
    pub const TITLE: i32 = 2;
    pub const DESKTOP: i32 = 3;
    pub const TOPPAGE: i32 = 4;
    pub const NEW_GAME: i32 = 5;
    pub const GAME_CTRL: i32 = 6;
}

/// The title, with the save and event task it will hand on.
struct Title {
    demo: Demo,
    state: SaveState,
    vm: Option<Vm>,
    /// A movie the title waits on.
    movie: Option<Playing>,
    /// The intro stream it waits on (`PlayOpeningStream`).
    stream: Option<StreamPlayer>,
}

enum Stage {
    Title(Box<Title>),
    Desktop(Box<DesktopMode>),
    TopPage(Box<TopPageMode>),
    World(Box<WorldMode>),
    /// A field or a dungeon.
    Area(Box<AreaMode>),
    /// Only while changing over.
    Gone,
}

pub struct Session {
    iso: PathBuf,
    archive: Arc<Archive>,
    scripts: bool,
    card: Option<PathBuf>,
    /// New Game's name entry runs (false keeps the default names).
    name_entry: bool,
    stage: Stage,
    events: Vec<Event>,
    /// A change asked for, and the frames left of the overlay's load
    /// before it happens.
    loading: Option<(i32, u32)>,
    /// `DESKTOP_FLG`: the desktop has run since power-on, so cancel skips
    /// the title's stream.
    desktop_run: bool,
    /// The last `ChangeArea(area, town)`: where The World is entered.
    area: Option<(i32, i32)>,
    /// Where the title's Load left `ccSaveSys` (`port`, `fileNum`): the
    /// card the field's Recorder starts on.
    card_position: (i32, i32),
    /// `ccGame`'s scene in The World, and `WORLD_MAN`'s area (the words'
    /// field and dungeons).
    scene: piney_world::area::Scene,
    world_man: Option<piney_world::area::WorldMan>,
    /// The dungeon the party is in, between its rooms' scenes (or area
    /// 15's story map, between its blocks'): what `WORLD_MAN` keeps.
    dungeon: Option<piney_world::evarea::Kept>,
    /// A change of scene asked for in The World, and the frames of
    /// `ccSetupGameCtrl`'s fade out drawn over the old scene since (its
    /// tasks still run under it).
    leaving: Option<u32>,
    /// A change asked by a mode that does not hold the scene, made on it
    /// when that mode leaves (with its save).
    pending: Option<Pending>,
    /// The last change refused (a town not ported), not to repeat it.
    refused: Option<piney_data::area::Go>,
    /// `ccSpcManager` and `ccPartyManager`, carried from one area to the
    /// next (the town's own until it leaves).
    spcs: Option<piney_world::party::Spcs>,
    /// `gtHackFlag` and `ccGame.setupMode`, from the gate hack's OK to the
    /// next scene's set-up (`ccClearGtHack` in each set-up decides whether
    /// the flag stays).
    gt_hack: bool,
    setup_mode: bool,
    /// The picture on screen: the last frame of The World the layers
    /// flipped to, held through a change of scene's fade.
    shown: Option<Frame>,
    /// `ccLoadDisp`, put up as a scene's set-up loads its files, whether
    /// the set-up has reached that load (its event passes made) and whether
    /// the scene has started (`ccSnd.gameStart`).
    load_disp: Option<crate::loaddisp::LoadDisp>,
    at_load: bool,
    load_started: bool,
    /// `--dvd`'s disc timing, when on ...
    dvd: Option<crate::dvd::Dvd>,
    /// ... the frames the new scene waits for its files, under the loading
    /// display ...
    hold: u32,
    /// ... and the files the last scene's set-up read, which the next does
    /// not read again (`loadCheck`).
    resident: std::collections::BTreeSet<String>,
    /// The console's `god`: the party at full HP and SP every frame.
    god: bool,
    /// Not the game's: the HUD's size ([`Mode::set_hud_scale`]), handed to
    /// the field UI every frame.
    hud_scale: f32,
    /// `ccSys+0x358`, which `ccSystem::Ctrl` (main 0x0010a6bc) adds one to
    /// every frame from power-on. A town's `ccInitRand` draws the walking
    /// PCs' generator that many times, so each arrival has its own PCs.
    sys_frames: u32,
    /// Not the game's: the logo movies a launcher played before this
    /// power-on, which the title's first boot then starts after.
    logos_played: i32,
    /// Not the game's: the options kept across the parts
    /// ([`crate::settings`]): the file, the options last seen in the
    /// save, and the stage they were seen in.
    settings_path: Option<PathBuf>,
    settings: Option<crate::settings::Settings>,
    settings_stage: u8,
}

/// Log in's `ccSetupNewGame` (main 0x001687a0), its part in the save:
/// `InitSpcParam` (a new game's kit while `newGameFlag` is 0),
/// `newGameFlag` = 1, `SetSpcBaseMsg`
/// ([`piney_fieldui::newgame::setup_new_game`]).
pub(crate) fn log_in(iso: &std::path::Path, state: &mut SaveState) {
    let done = Iso::open(iso)
        .map_err(|e| e.to_string())
        .and_then(|mut disc| piney_fieldui::newgame::setup_new_game(state, &mut disc).map_err(|e| e.to_string()));
    if let Err(e) = done {
        tracing::warn!("ccSetupNewGame: {e}");
    }
}

/// A change of scene the session makes itself.
#[derive(Clone, Debug, PartialEq)]
enum Pending {
    /// `ccGame::ChangeArea` or `ChangeScene`.
    Go(piney_data::area::Go),
    /// `WORLD_MAN::SetGenerateCode(a, b, c)`.
    Words([i32; 3]),
    /// The event scripts' `scene` (`ccGame::ChangeScene`), after the
    /// `area` that made its story area.
    Story(crate::field_host::SceneChange),
}

/// Frames between the title's `ChangeRequest` and the desktop's setup:
/// `ccThMother` loads `DESKTOP.PRG` from the disc meanwhile, with the
/// screen black and the sound task still stepping the title's music fade
/// (nine frames, three of them before the request). Not measured; long
/// enough for the fade to finish, as the disc's load is.
const LOAD_FRAMES: u32 = 8;

/// `ccAddPlayTime`'s limit: 0x0cdfe5c4 sixtieths, 999:59:59.
const PLAY_TIME_MAX: i32 = 0x0cdf_e5c4;

/// Where a session made from a save and an event task starts
/// ([`Session::resume`]).
pub enum Resume {
    /// `ccSetupDesktop`.
    Desktop,
    /// `ccSetupToppage`: the top page and the board.
    Board,
    /// Log in's `ccSetupNewGame`, then mode 6.
    World(Box<InWorld>),
}

/// Mode 6 in `scene`: Mac Anu for area 0, else a field or dungeon of
/// `world_man`'s area, the party `spcs` (a new game's when None).
pub struct InWorld {
    pub scene: piney_world::area::Scene,
    pub world_man: Option<piney_world::area::WorldMan>,
    pub spcs: Option<piney_world::party::Spcs>,
}

impl Session {
    /// A session with no mode yet.
    fn bare(iso: PathBuf, archive: Arc<Archive>, scripts: bool, card: Option<PathBuf>, name_entry: bool) -> Self {
        Session {
            iso,
            archive,
            scripts,
            card,
            name_entry,
            stage: Stage::Gone,
            events: Vec::new(),
            loading: None,
            desktop_run: false,
            area: None,
            card_position: (0, 0),
            scene: piney_world::area::Scene::init(),
            world_man: None,
            dungeon: None,
            leaving: None,
            pending: None,
            refused: None,
            spcs: None,
            gt_hack: false,
            setup_mode: false,
            shown: None,
            load_disp: None,
            at_load: false,
            load_started: false,
            dvd: crate::dvd::get(),
            hold: 0,
            resident: Default::default(),
            god: false,
            hud_scale: 1.0,
            sys_frames: 0,
            logos_played: 0,
            settings_path: None,
            settings: None,
            settings_stage: u8::MAX,
        }
    }

    /// Power on: the boot, then the title with its logos.
    #[allow(dead_code)]
    pub fn new(
        iso: PathBuf,
        archive: Arc<Archive>,
        scripts: bool,
        card: Option<PathBuf>,
        name_entry: bool,
    ) -> Result<Self, String> {
        Session::after_logos(iso, archive, scripts, card, name_entry, 0, None)
    }

    /// Power on after a launcher that played the first `played` of the
    /// logo movies (`piney_demo::names::LOGO_MOVIES`): the title's logos
    /// start at the next.
    pub fn after_logos(
        iso: PathBuf,
        archive: Arc<Archive>,
        scripts: bool,
        card: Option<PathBuf>,
        name_entry: bool,
        played: i32,
        settings: Option<PathBuf>,
    ) -> Result<Self, String> {
        let mut session = Session::bare(iso, archive, scripts, card, name_entry);
        session.logos_played = played;
        session.settings_path = settings;
        session.stage = session.boot_title(true)?;
        session.title_sound();
        if let Stage::Title(t) = &session.stage {
            session.events.push(crate::mode::display_offset(&t.state.save));
        }
        Ok(session)
    }

    /// Straight into The World on `state` (a new game's save) with the event
    /// task `vm`, in `scene`: the town (area 0) or a field of `world_man`'s
    /// area.
    pub fn in_world(
        iso: PathBuf,
        archive: Arc<Archive>,
        card: Option<PathBuf>,
        state: SaveState,
        vm: Option<Vm>,
        scene: piney_world::area::Scene,
        world_man: Option<piney_world::area::WorldMan>,
    ) -> Result<Self, String> {
        let mut session = Session::bare(iso, archive, false, card, false);
        session.resume_in(state, vm, Resume::World(Box::new(InWorld { scene, world_man, spcs: None })))?;
        Ok(session)
    }

    /// A game resumed on `state` with the event task `vm` (the story's start
    /// points, `crate::start`): the desktop, the board or The World, the
    /// desktop counted as run since power-on.
    pub fn resume(
        iso: PathBuf,
        archive: Arc<Archive>,
        card: Option<PathBuf>,
        state: SaveState,
        vm: Vm,
        at: Resume,
    ) -> Result<Self, String> {
        let mut session = Session::bare(iso, archive, true, card, false);
        session.resume_in(state, Some(vm), at)?;
        Ok(session)
    }

    fn resume_in(&mut self, state: SaveState, vm: Option<Vm>, at: Resume) -> Result<(), String> {
        self.desktop_run = true;
        self.stage = match at {
            Resume::Desktop => self.enter_desktop(state, vm, piney_desktop::FRAME_RATE)?,
            Resume::Board => {
                Stage::TopPage(Box::new(TopPageMode::enter(self.iso.clone(), self.archive.clone(), state, vm)?))
            }
            Resume::World(w) => {
                let InWorld { scene, world_man, spcs } = *w;
                let mut state = state;
                log_in(&self.iso, &mut state);
                self.area = Some((scene.area, scene.town));
                (self.scene, self.world_man, self.spcs) = (scene, world_man, spcs);
                self.world_stage(state, vm, false)?
            }
        };
        Ok(())
    }

    /// Mode 6 for the session's scene: Mac Anu for area 0, else a field or
    /// dungeon. `faded`: the scene before has drawn the fade out.
    fn world_stage(&mut self, state: SaveState, vm: Option<Vm>, faded: bool) -> Result<Stage, String> {
        self.put_up_load_disp();
        // What the set-up reads, timed as the disc would take (`--dvd`).
        piney_data::archive::record();
        let stage = self.enter_world(state, vm, faded);
        let files = piney_data::archive::recorded();
        // loadCheck: no card when the scene reads nothing the last did not,
        // as an event's `scene -2` setting the same town up again.
        if files.keys().all(|f| self.resident.contains(f)) {
            self.load_disp = None;
        }
        if let Some(dvd) = self.dvd {
            self.hold = dvd.frames(&files, &self.resident);
        }
        self.resident = files.into_keys().collect();
        stage
    }

    fn enter_world(&mut self, mut state: SaveState, vm: Option<Vm>, faded: bool) -> Result<Stage, String> {
        self.restock(&mut state);
        if self.scene.area == piney_world::area::kind::TOWN {
            // ccClearGtHack in the town's set-up: the flag goes.
            self.gt_hack = false;
            let mut w = WorldMode::enter(&self.iso, self.archive.clone(), state, vm)?;
            w.init_rand(Arrival { frames: self.sys_frames, faded }.rand_count());
            // ccSpcManager and ccPartyManager as the last area left them.
            if let Some(spcs) = self.spcs.clone() {
                w.set_spcs(spcs);
            }
            w.set_card(self.card.as_deref(), self.card_position);
            if faded {
                w.skip_fade_out();
            }
            return Ok(Stage::World(Box::new(w)));
        }
        let wm = self.world_man.ok_or_else(|| format!("field {}: no area words", self.scene.field))?;
        let kept = self.dungeon.take();
        let spcs = self.spcs.clone().unwrap_or_default();
        let at = Arrival { frames: self.sys_frames, faded };
        let mut a = AreaMode::enter(&self.iso, self.archive.clone(), state, vm, self.scene, wm, kept, at, spcs)?;
        a.set_gate_hack(self.gt_hack, self.setup_mode);
        self.setup_mode = false;
        Ok(Stage::Area(Box::new(a)))
    }

    /// `ccSetupGameCtrl`'s `SetSpcItemTown` and `SetTradeItemTown` for the
    /// scene being set up ([`piney_world::area::Scene::restock`]), drawing
    /// from the game's one `rand()` as the last mode left it.
    fn restock(&mut self, state: &mut SaveState) {
        match Iso::open(&self.iso).and_then(|mut d| d.volume()) {
            Ok(v) => {
                let mut rand = piney_world::Rand(state.rand);
                self.scene.restock().apply(&mut state.save, v, self.scene.server, &mut rand);
                state.rand = rand.0;
            }
            Err(e) => tracing::warn!("the restock: {e}"),
        }
    }

    /// `ccFileListLoad`'s `ccLoadDispInit` for the scene being set up
    /// ([`crate::loaddisp::kind`]): the gate hack's stream (`setupMode`)
    /// plays instead of any card.
    fn put_up_load_disp(&mut self) {
        use crate::loaddisp::{LoadDisp, kind};
        let field_type = self.world_man.map_or(0, |w| w.field_type as i32);
        (self.at_load, self.load_started) = (false, false);
        self.load_disp = kind(&self.scene, field_type, self.setup_mode).map(|k| {
            let mut disc = Iso::open(&self.iso).ok();
            let v = disc.as_mut().and_then(|d| d.volume().ok()).unwrap_or(piney_data::volume::Volume::Inf);
            let tables = piney_data::area::AreaTables::of(v);
            let word = |id: i32| tables.word_by_id(id).map_or("", |w| w.text);
            let ids = self.world_man.map_or([-1; 3], |w| w.words);
            let words = ids.map(word);
            LoadDisp::new(k, &self.archive, v, &self.scene, words)
        });
    }

    /// The change of scene once its fade out is drawn: the old mode 6 left,
    /// the next set up.
    fn change_scene(&mut self) -> Result<(), String> {
        let stage = std::mem::replace(&mut self.stage, Stage::Gone);
        // gcmn stays resident through mode 6: its menu lists go on.
        let globals = match &stage {
            Stage::World(w) => Some(w.ui().globals()),
            Stage::Area(a) => Some(a.ui().globals()),
            _ => None,
        };
        let (mut state, vm) = match stage {
            Stage::World(mut w) => {
                // ccSetupGameCtrl's ccStoreSpcCondition, for the next
                // scene's party; the old scene's fellow tasks deleted.
                w.world_mut().store_conditions();
                w.world_mut().delete_fellows();
                self.spcs = Some(w.world().spcs().clone());
                w.leave()
            }
            Stage::Area(mut a) => {
                self.gt_hack = a.world().gt_hack();
                a.world_mut().delete_fellows();
                let (state, vm, _, wm, dungeon, spcs) = a.leave();
                self.world_man = Some(wm);
                self.dungeon = dungeon;
                self.spcs = Some(spcs);
                (state, vm)
            }
            other => {
                self.stage = other;
                return Err("a change of scene outside The World".into());
            }
        };
        match self.pending.take() {
            Some(Pending::Go(go)) => self.scene.go(go, &mut state.save),
            Some(Pending::Words(words)) => {
                match crate::area::set_generate_code(&self.iso, words, self.scene.server, &state.save) {
                    Ok((wm, go)) => {
                        // A new area: the last one's dungeon is gone.
                        self.world_man = Some(wm);
                        self.dungeon = None;
                        self.scene.go(go, &mut state.save);
                    }
                    Err(e) => tracing::warn!("{e}"),
                }
            }
            Some(Pending::Story(c)) => {
                // Instruction 118 `area n` (ccEvAreaCodeAdd): a story area
                // past 13 is SimGenerateCode of its words on this server.
                if let Some((n, _)) = c.area {
                    match crate::area::ev_area_world_man(&self.iso, i32::from(n), self.scene.server, &state.save) {
                        Ok(Some(wm)) => {
                            self.world_man = Some(wm);
                            self.dungeon = None;
                        }
                        // 1-13: no words; the same WORLD_MAN takes the
                        // number (its fixed story map, model and flags).
                        Ok(None) => {
                            if let Some(cur) = self.world_man {
                                match crate::area::ev_area_number(&self.iso, i32::from(n), &cur, &state.save) {
                                    Ok(wm) => {
                                        self.world_man = Some(wm);
                                        self.dungeon = None;
                                    }
                                    Err(e) => tracing::warn!("{e}"),
                                }
                            }
                        }
                        Err(e) => tracing::warn!("{e}"),
                    }
                }
                let go = piney_data::area::Go::ChangeScene(c.scene.map(i32::from));
                self.scene.go(go, &mut state.save);
            }
            None => {}
        }
        self.stage = self.world_stage(state, vm, true)?;
        if let Some(g) = globals {
            match &mut self.stage {
                Stage::World(w) => w.ui_mut().set_globals(g),
                Stage::Area(a) => a.ui_mut().set_globals(g),
                _ => {}
            }
        }
        Ok(())
    }

    /// Whether the port has the scene `go` leads to: Mac Anu and Dun
    /// Loireag of the Root Towns, and the fields and dungeons.
    fn ported(go: piney_data::area::Go) -> bool {
        use piney_data::area::Go;
        match go {
            Go::ChangeArea(a, n) => a != piney_world::area::kind::TOWN || piney_world::town_ported(n),
            Go::ChangeScene([a, t, ..]) => a != piney_world::area::kind::TOWN || piney_world::town_ported(t),
        }
    }

    /// The options kept across the parts ([`crate::settings`]): in a new
    /// mode the kept ones put into its save (a mode's set-up may have made
    /// its own); else, when the save's differ from those last seen, a menu
    /// changed them, and the file takes them.
    fn keep_settings(&mut self) {
        use crate::settings::Settings;
        let Some(path) = self.settings_path.clone() else { return };
        let stage = match &self.stage {
            Stage::Title(_) => 0,
            Stage::Desktop(_) => 1,
            Stage::TopPage(_) => 2,
            Stage::World(_) => 3,
            Stage::Area(_) => 4,
            Stage::Gone => return,
        };
        let kept = self.settings;
        let fresh = stage != self.settings_stage;
        self.settings_stage = stage;
        let save = match &mut self.stage {
            Stage::Title(t) => Some(t.demo.save_mut()),
            _ => self.save_mut(),
        };
        let Some(save) = save else { return };
        let now = Settings::of(save);
        match kept {
            Some(k) if fresh && k != now => k.apply(save),
            Some(k) if k != now => {
                now.store(&path);
                self.settings = Some(now);
            }
            Some(_) => {}
            None => self.settings = Some(now),
        }
    }

    /// The save of the stage that holds one: the desktop's, the board's,
    /// The World's (not the title's, which Load replaces).
    fn save_mut(&mut self) -> Option<&mut SaveData> {
        match &mut self.stage {
            Stage::Desktop(d) => d.save_mut(),
            Stage::TopPage(t) => t.save_mut(),
            Stage::World(w) => Some(&mut w.world_mut().state_mut().save),
            Stage::Area(a) => Some(a.save_mut()),
            Stage::Title(_) | Stage::Gone => None,
        }
    }

    /// A debug console line ([`crate::console`]).
    fn console_line(&mut self, line: &str) -> String {
        use piney_event::ScriptSave;
        let w: Vec<&str> = line.split_whitespace().collect();
        let num = |i: usize| w.get(i).and_then(|s| s.parse::<i32>().ok());
        let Some(&cmd) = w.first() else { return String::new() };
        if cmd == "help" {
            return [
                "item CAT ID [N]   an item to Kite (15 key items)",
                "core LETTER [N]   virus cores (A-Z)",
                "gold N            Kite's gold, plus N",
                "heal              the party at full HP/SP (field)",
                "god               the party kept at full HP/SP, the fallen got up (on / off)",
                "kill              every enemy here felled by Kite",
                "protect           every enemy's protect broken (Data Drain)",
                "town N            to a Root Town (0 Mac Anu, 1 Dun Loireag, 2 Carmina Gadelica, 3 Fort Ouph, 4 Lia Fail)",
                "infection N       the bracelet's infection (0-100)",
                "address ID        a member's address (call them: PERSONAL > Party)",
                "invite_party ID   a character into the party whatever the story says (a town: now, at the gate; a field: with the next area)",
                "exp N             N experience to each member in a field or dungeon (1000 a level)",
                "flag N [VALUE]    event N's flag (hex VALUE sets it)",
                "time              the play time",
                "where             where the game is",
            ]
            .join("\n");
        }
        if cmd == "where" {
            return Mode::title(self);
        }
        if cmd == "heal" {
            return match &mut self.stage {
                Stage::Area(a) => format!("healed {}", a.heal_party()),
                _ => "heal works in a field or dungeon".into(),
            };
        }
        if cmd == "god" {
            self.god = !self.god;
            return format!("god mode {}", if self.god { "on" } else { "off" });
        }
        if cmd == "kill" {
            return match &mut self.stage {
                Stage::Area(a) => format!("{} felled", a.kill_enemies()),
                _ => "kill works in a field or dungeon".into(),
            };
        }
        if cmd == "town" {
            use piney_data::world::Town;
            let Some((n, town)) = num(1).and_then(|n| Some((n, Town::from_index(n)?))) else {
                return "town N (0 Mac Anu, 1 Dun Loireag, 2 Carmina Gadelica, 3 Fort Ouph, 4 Lia Fail)".into();
            };
            if !matches!(self.stage, Stage::World(_) | Stage::Area(_)) {
                return "town works in The World (a town, field or dungeon)".into();
            }
            self.go(Pending::Go(piney_data::area::Go::ChangeArea(piney_world::area::kind::TOWN, n)));
            return format!("to {town:?}");
        }
        if cmd == "invite_party" {
            return self.invite_party(num(1));
        }
        if cmd == "exp" {
            return match (&mut self.stage, num(1).filter(|n| (1..=30000).contains(n))) {
                (Stage::Area(a), Some(n)) => format!("{n} exp to {} members", a.give_exp(n as i16)),
                (_, None) => "exp N (1-30000; 1000 a level)".into(),
                _ => "exp works in a field or dungeon".into(),
            };
        }
        if cmd == "protect" {
            return match &mut self.stage {
                Stage::Area(a) => format!("{} protects broken for 60 s", a.break_protects()),
                _ => "protect works in a field or dungeon".into(),
            };
        }
        let Some(save) = self.save_mut() else { return "no save here yet".into() };
        match cmd {
            "item" => match (num(1), num(2)) {
                (Some(cat), Some(id)) => {
                    let n = num(3).unwrap_or(1);
                    save.add_item(0, cat as i16, id as i16, n as i16);
                    format!("item {cat} {id} x{n} to Kite")
                }
                _ => "item CAT ID [N]".into(),
            },
            "core" => {
                let letter = w.get(1).and_then(|s| s.chars().next()).map(|c| c.to_ascii_uppercase());
                match letter.filter(char::is_ascii_uppercase) {
                    Some(c) => {
                        let n = num(2).unwrap_or(1);
                        save.add_item(0, 15, (c as u8 - b'A') as i16, n as i16);
                        format!("Virus Core {c} x{n}")
                    }
                    None => "core LETTER [N]".into(),
                }
            }
            "infection" => match num(1) {
                Some(n) => {
                    let v = n.clamp(0, 100) as i16;
                    save.set_i16(piney_battle::exp::SAVE_EROSION, v);
                    format!("infection {v}%")
                }
                None => format!("infection {}% (infection N sets it)", save.i16(piney_battle::exp::SAVE_EROSION)),
            },
            "gold" => match num(1) {
                Some(n) => {
                    let g = save.gold(0).saturating_add(n).clamp(0, 9_999_999);
                    save.set_gold(0, g);
                    format!("gold {g}")
                }
                None => "gold N".into(),
            },
            "flag" => match num(1).and_then(|n| usize::try_from(n).ok()).filter(|&n| n < 512) {
                Some(n) => {
                    if let Some(v) = w.get(2).and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()) {
                        save.set_event_flag(n, v);
                    }
                    format!("event {n}: {:#x}", save.event_flag(n))
                }
                None => "flag N [VALUE]".into(),
            },
            "address" => match num(1).filter(|id| (1..18).contains(id)) {
                Some(id) => {
                    let w = save.member_word(offset::PARTY_MEMBER_FLAG) | (1 << id);
                    save.set_member_word(offset::PARTY_MEMBER_FLAG, w);
                    format!("member address {id}")
                }
                None => "address ID (1-17: 2 Orca, 15 BlackRose, 1 Mia, 10 Elk...)".into(),
            },
            "time" => {
                let t = save.play_time() / 60;
                format!("play time {}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
            }
            _ => format!("unknown: {cmd} (help)"),
        }
    }

    /// The console's `invite_party ID`: `charTbl` row `id` into the party
    /// whatever the story says, its address known too. In a Root Town it
    /// joins at once (`ccParty::AddMember`, a character the registry does
    /// not hold built at the Chaos Gate by `inviteSpc`); in a field or
    /// dungeon it is registered with a slot and comes with the next area.
    fn invite_party(&mut self, id: Option<i32>) -> String {
        use piney_event::ScriptSave;
        let Some(id) = id.filter(|id| (1..=20).contains(id)) else {
            return "invite_party ID (1-20: 2 Orca, 15 BlackRose, 1 Mia, 10 Elk...)".into();
        };
        if let Some(save) = self.save_mut() {
            let w = save.member_word(offset::PARTY_MEMBER_FLAG) | (1 << id);
            save.set_member_word(offset::PARTY_MEMBER_FLAG, w);
        }
        let (party, slot, later) = match &mut self.stage {
            Stage::World(w) => {
                let world = w.world_mut();
                let party = world.party();
                let slot = if party.contains(&id) { -2 } else { world.party_add(id) };
                if let Ok(s) = usize::try_from(slot) {
                    w.set_menu_face(s, id);
                }
                (party, slot, false)
            }
            Stage::Area(a) => {
                let world = a.world_mut();
                let party = world.party();
                let slot = if party.contains(&id) { -2 } else { world.invite_next_area(id) };
                if let Ok(s) = usize::try_from(slot) {
                    a.set_menu_face(s, id);
                }
                (party, slot, true)
            }
            _ => return "invite_party works in The World (a town, field or dungeon)".into(),
        };
        match slot {
            -2 => format!("{id} is in the party already ({party:?})"),
            s if s < 0 => format!("no room for {id}: the party is {party:?} (PERSONAL > Party to remove one)"),
            s if later => format!("{id} in slot {s}: with the next area"),
            s => format!("{id} in slot {s}"),
        }
    }

    /// `ccAddPlayTime` (main 0x00167740), which `main` calls every frame
    /// of its loop: `saveData.playTime` plus the frame rate (vertical
    /// blanks a frame), held at 0x0cdfe5c4. `gameCntStop` holds it only
    /// inside a card write, which ends within its frame. The title's save
    /// is left: New Game resets the time (`ccResetPlayTime`) and Load
    /// replaces it.
    fn add_play_time(&mut self) {
        let rate = Mode::frame_rate(self) as i32;
        if let Some(s) = self.save_mut() {
            let t = s.play_time().wrapping_add(rate).min(PLAY_TIME_MAX);
            s.set_i32(offset::PLAY_TIME, t);
        }
        // `ccGame.gameCnt`, on the scene a field or dungeon holds until it
        // hands it over, else on the session's.
        match &mut self.stage {
            Stage::Area(a) if self.leaving.is_none() => a.add_play_time(rate),
            _ => self.scene.add_play_time(rate),
        }
    }

    /// Start the fade out of a change of scene the session makes.
    fn go(&mut self, p: Pending) {
        if self.leaving.is_some() {
            return;
        }
        let go = match &p {
            Pending::Go(go) => Some(*go),
            Pending::Story(c) => Some(piney_data::area::Go::ChangeScene(c.scene.map(i32::from))),
            Pending::Words(_) => None,
        };
        if let Some(go) = go
            && !Self::ported(go)
        {
            if self.refused != Some(go) {
                tracing::warn!("{go:?}: not a Root Town the port has");
                self.refused = Some(go);
            }
            return;
        }
        self.pending = Some(p);
        self.leaving = Some(0);
        self.events.push(Event::GameInterrupt);
    }

    /// What The World's modes ask of the session: a change of scene (its
    /// fade out starts over the old scene; `ChangeRequest` interrupts the
    /// sound), `WORLD_MAN`'s area; the rest goes on.
    fn world_events(&mut self, events: Vec<Event>) -> Option<i32> {
        let mut change = None;
        for e in events {
            if matches!(e, Event::GameStart) {
                self.load_started = true;
            }
            match e {
                Event::ChangeScene(scene) => {
                    if self.leaving.is_none() {
                        self.scene = scene;
                        self.leaving = Some(0);
                        self.events.push(Event::GameInterrupt);
                    }
                }
                Event::Go(go) => self.go(Pending::Go(go)),
                Event::GoToArea(words) => self.go(Pending::Words(words)),
                Event::GateHacked => {
                    self.gt_hack = true;
                    self.setup_mode = true;
                }
                // ChangeScene's ChangeRequest(6, 7): the session's own change.
                Event::ChangeMode { num: 6, .. } => {}
                // Log Out (4) and OPTION's Title Screen (1).
                Event::ChangeMode { num, .. } => change = Some(num),
                Event::WorldMan(wm) => {
                    self.world_man = Some(wm);
                    self.dungeon = None;
                }
                e => self.events.push(e),
            }
        }
        change
    }

    /// The boot and the title: a fresh save, the event task started, and
    /// `ccSetupDemo`'s title - with its logos on the first boot only.
    fn boot_title(&self, first_boot: bool) -> Result<Stage, String> {
        let mut disc = Iso::open(&self.iso).map_err(|e| format!("{}: {e}", self.iso.display()))?;
        let text = InitText::from_disc(&mut disc).map_err(|e| format!("ccSaveData::Init: {e}"))?;
        let mut state = SaveState::fresh_with(&text);
        // The options kept across the parts, over the new save's.
        if let Some(s) = self.settings_path.as_deref().and_then(|p| crate::settings::Settings::load(p, &state.save)) {
            s.apply(&mut state.save);
        }
        let vm = if self.scripts { Some(desktop::boot(&mut disc, &mut state)?) } else { None };
        // The card the desktop's Data screen saves to, for the boot's check
        // and the title's Load.
        let card: Box<dyn MemoryCard> = match &self.card {
            Some(dir) => Box::new(FilesCard::slot1(disc.volume().map_err(|e| e.to_string())?, dir.clone())),
            None => Box::new(NoCard),
        };
        let logos_played = if first_boot { self.logos_played } else { 0 };
        let config = Config { save: state.save.clone(), first_boot, card, logos_played, ..Config::default() };
        let demo = Demo::new(&mut disc, self.archive.clone(), config).map_err(|e| format!("title: {e}"))?;
        Ok(Stage::Title(Box::new(Title { demo, state, vm, movie: None, stream: None })))
    }

    /// `ccSetupDemo`'s sound: everything off, then the title's bank.
    fn title_sound(&mut self) {
        self.events.push(Event::AllSoundOff);
        self.events.push(Event::SqLoad(piney_audio::SqContext::Title));
    }

    /// `ccSetupDesktop` on `state` and the event task, at `rate`, the frame
    /// rate the mode before left.
    fn enter_desktop(&self, state: SaveState, vm: Option<Vm>, rate: u32) -> Result<Stage, String> {
        let mut d =
            DesktopMode::enter(self.iso.clone(), self.archive.clone(), state, vm, self.card.clone(), self.name_entry)?;
        d.carry_frame_rate(rate);
        Ok(Stage::Desktop(Box::new(d)))
    }

    /// Go where `ChangeRequest(num)` asks.
    fn change(&mut self, num: i32) -> Result<(), String> {
        // `ccSystem`'s frame rate, which the next setup starts with.
        let rate = Mode::frame_rate(self);
        // The mode's own set-up loads its lists: none of The World's stays.
        self.resident.clear();
        let stage = std::mem::replace(&mut self.stage, Stage::Gone);
        self.stage = match (num, stage) {
            (request::DESKTOP, Stage::Title(t)) => {
                let Title { demo, mut state, vm, .. } = *t;
                // The title writes into its own copy of the save: New
                // Game's, or the one Load read from the card.
                state.save = demo.save().clone();
                let mut d = DesktopMode::enter(
                    self.iso.clone(),
                    self.archive.clone(),
                    state,
                    vm,
                    self.card.clone(),
                    self.name_entry,
                )?;
                // One `ccSaveSys` from boot: the Data screen starts where the
                // title's Load left it.
                self.card_position = demo.card_position();
                d.set_card_position(self.card_position);
                self.desktop_run = true;
                Stage::Desktop(Box::new(d))
            }
            (request::RESET | request::TITLE, old) => {
                // The soft reset is the mother task's (no new boot): the
                // C library's rand() runs on.
                let rand = match old {
                    Stage::Title(t) => Some(t.state.rand),
                    Stage::Desktop(d) => Some(d.leave().0.rand),
                    Stage::TopPage(t) => Some(t.leave().0.rand),
                    Stage::World(w) => Some(w.leave().0.rand),
                    Stage::Area(a) => Some(a.leave().0.rand),
                    Stage::Gone => None,
                };
                let mut stage = self.boot_title(false)?;
                self.title_sound();
                if let Stage::Title(t) = &mut stage {
                    t.state.rand = rand.unwrap_or(t.state.rand);
                    self.events.push(crate::mode::display_offset(&t.state.save));
                }
                stage
            }
            (request::TOPPAGE, Stage::Desktop(d)) => {
                let (state, vm) = d.leave();
                Stage::TopPage(Box::new(TopPageMode::enter(self.iso.clone(), self.archive.clone(), state, vm)?))
            }
            // Log Out: The World's ChangeRequest(4, 7), back to its top
            // page; the field or dungeon is left behind.
            (request::TOPPAGE, Stage::World(w)) => {
                let (state, vm) = w.leave();
                Stage::TopPage(Box::new(TopPageMode::enter(self.iso.clone(), self.archive.clone(), state, vm)?))
            }
            (request::TOPPAGE, Stage::Area(a)) => {
                let (state, vm, ..) = a.leave();
                self.dungeon = None;
                Stage::TopPage(Box::new(TopPageMode::enter(self.iso.clone(), self.archive.clone(), state, vm)?))
            }
            (request::DESKTOP, Stage::TopPage(t)) => {
                let (state, vm) = t.leave();
                self.enter_desktop(state, vm, rate)?
            }
            // The event scripts' `mode 3` from The World (event 4 +0x02a2,
            // from the dungeon) and from the desktop's own setup (+0x02d8,
            // which sets the desktop up again on the same event task).
            (request::DESKTOP, Stage::World(w)) => {
                let (state, vm) = w.leave();
                self.enter_desktop(state, vm, rate)?
            }
            (request::DESKTOP, Stage::Area(a)) => {
                let (state, vm, ..) = a.leave();
                self.dungeon = None;
                self.enter_desktop(state, vm, rate)?
            }
            (request::DESKTOP, Stage::Desktop(d)) => {
                let (state, vm) = d.leave();
                self.enter_desktop(state, vm, rate)?
            }
            (request::NEW_GAME | request::GAME_CTRL, Stage::TopPage(t)) => {
                // Log in: `ccSetupNewGame` (GCMN.PRG), then `ccSetupGameCtrl`
                // in the area `ChangeArea` left (area 0, a Root Town).
                let (mut state, vm) = t.leave();
                log_in(&self.iso, &mut state);
                self.scene = piney_world::area::Scene::log_in(&mut state.save);
                self.restock(&mut state);
                match WorldMode::enter(&self.iso, self.archive.clone(), state.clone(), vm) {
                    Ok(mut w) => {
                        w.init_rand(Arrival { frames: self.sys_frames, faded: false }.rand_count());
                        w.set_card(self.card.as_deref(), self.card_position);
                        Stage::World(Box::new(w))
                    }
                    Err(e) => {
                        tracing::warn!("{e} (area {:?}); back to the desktop", self.area);
                        let d = DesktopMode::enter(
                            self.iso.clone(),
                            self.archive.clone(),
                            state,
                            None,
                            self.card.clone(),
                            self.name_entry,
                        )?;
                        Stage::Desktop(Box::new(d))
                    }
                }
            }
            (num, stage) => {
                tracing::warn!("mode {num} is not ported");
                stage
            }
        };
        Ok(())
    }
}

impl Mode for Session {
    fn set_hud_scale(&mut self, k: f32) {
        self.hud_scale = k;
    }

    fn step(&mut self, pad: &Pad) -> Frame {
        self.sys_frames = self.sys_frames.wrapping_add(1);
        match &mut self.stage {
            Stage::Area(a) => a.ui_mut().hud_scale = self.hud_scale,
            Stage::World(w) => w.ui_mut().hud_scale = self.hud_scale,
            Stage::Desktop(d) => d.set_sys_frames(self.sys_frames),
            _ => {}
        }
        // `ccLoadResourceFL` comes after the set-up's passes at phases 0
        // and 2 (`ccEnableThEvent(2)` waits at 0x00168f94), so a pass's
        // streams play before the load and its display.
        self.at_load |= match &self.stage {
            Stage::Area(a) => a.setup().passes_made(),
            Stage::World(w) => w.setup().passes_made(),
            _ => true,
        };
        // `--dvd`: the new scene's files still loading; only the loading
        // display runs.
        if self.hold > 0 && self.at_load {
            self.hold -= 1;
            let mut frame = Frame::new();
            if let Some(ld) = &mut self.load_disp
                && ld.frame(false)
            {
                ld.draw(&mut frame);
            }
            return frame;
        }
        if let Some((num, left)) = self.loading {
            if left > 0 {
                self.loading = Some((num, left - 1));
                return Frame::new();
            }
            self.loading = None;
            if let Err(e) = self.change(num) {
                tracing::warn!("{e}");
            }
        }
        let mut change = None;
        let frame = match &mut self.stage {
            Stage::Title(t) => 'title: {
                // `PlayOpeningStream` holds the title's task until the
                // stream ends, flashing over it.
                if let Some(p) = &mut t.stream {
                    if let (frame, true) = p.frame() {
                        t.demo.stream_tick(pad, frame);
                    }
                    match p.step(pad, &mut self.events) {
                        Some(mut frame) => {
                            t.demo.stream_fade(&mut frame);
                            break 'title frame;
                        }
                        None => {
                            t.state.rand = p.rand();
                            t.stream = None;
                        }
                    }
                }
                // `ccDecodeMpeg` holds the title's task until the movie ends.
                if let Some(p) = &mut t.movie {
                    match p.step(pad, t.demo.save()) {
                        Played::Showing(frame) => break 'title frame,
                        Played::Ended => {}
                        Played::Skipped => t.demo.movie_skipped(),
                    }
                    t.movie = None;
                    self.events.push(Event::MovieAudioStop);
                }
                let frame = t.demo.step(pad);
                for r in t.demo.take_requests() {
                    match r {
                        Request::Se(n) => self.events.push(Event::Se(n)),
                        Request::SqPlay(n) => self.events.push(Event::SqPlay(n)),
                        Request::SqStop(n) => self.events.push(Event::SqStop(n)),
                        Request::SqFade { seq, volume, time, mode } => {
                            self.events.push(Event::SqFade { seq, volume, time, mode })
                        }
                        Request::MainVolume(v) => self.events.push(Event::MainVolume(i32::from(v))),
                        Request::ChangeMode { num, .. } => change = Some(num),
                        // Option's system menu, as on the desktop.
                        Request::Menu(r) => self.events.extend(desktop::event(r)),
                        // `LoadGame` ends with `SetSoundEnv`: the loaded
                        // save's volumes.
                        Request::LoadGame => {
                            // The options kept across the parts win over
                            // the loaded save's.
                            if let Some(s) = self.settings.filter(|_| self.settings_path.is_some()) {
                                s.apply(t.demo.save_mut());
                            }
                            let save = t.demo.save();
                            self.events.push(Event::Volumes {
                                main: i32::from(save.i16(offset::MAIN_VOL)),
                                se: i32::from(save.i16(offset::SE_VOL)),
                                bgm: i32::from(save.i16(offset::BGM_VOL)),
                                output: i32::from(save.i16(offset::OUTPUT)),
                            });
                            self.events.push(crate::mode::display_offset(save));
                        }
                        Request::Movie { path, audio } => match Playing::open(&self.iso, path, audio, &mut self.events)
                        {
                            Ok(p) => t.movie = Some(p),
                            // Counted as played.
                            Err(e) => tracing::warn!("{e}"),
                        },
                        // `PlayOpeningStream`: the intro, stream 0. A
                        // stream that does not start counts as played.
                        Request::Stream { num, .. } => {
                            // The title's own copy of the save, the game's `rand()`.
                            let state = SaveState { save: t.demo.save().clone(), ..t.state.clone() };
                            let started = usize::try_from(num).map_err(|_| format!("stream {num}")).and_then(|n| {
                                StreamPlayer::start(&self.iso, n, &state, self.desktop_run, &mut self.events)
                            });
                            match started {
                                Ok(p) => t.stream = Some(p),
                                Err(e) => tracing::warn!("{e}"),
                            }
                        }
                        // `NewGame` is applied inside the title.
                        Request::MenuDisplay(_) => {}
                        other => tracing::debug!("title asks: {other:?}"),
                    }
                }
                frame
            }
            Stage::Desktop(d) => {
                let frame = d.step(pad);
                for e in d.take_events() {
                    match e {
                        Event::ChangeMode { num, .. } => change = Some(num),
                        e => self.events.push(e),
                    }
                }
                frame
            }
            Stage::World(w) => {
                let frame = w.step(pad);
                let events = w.take_events();
                // The event scripts' `scene`: the event task sleeps in it,
                // and the session makes the change.
                let story = w.scene_change().cloned();
                change = self.world_events(events);
                if let Some(c) = story {
                    self.go(Pending::Story(c));
                }
                frame
            }
            Stage::Area(a) => {
                let frame = a.step(pad);
                if self.god {
                    a.revive_party();
                    a.heal_party();
                }
                let events = a.take_events();
                let story = a.scene_change().cloned();
                change = self.world_events(events);
                if let Some(c) = story {
                    self.go(Pending::Story(c));
                }
                frame
            }
            Stage::TopPage(t) => {
                let frame = t.step(pad);
                // Log in asks for 5, the area, then 6 in one frame; the
                // last request is the one taken.
                for e in t.take_events() {
                    match e {
                        Event::ChangeMode { num, .. } => change = Some(num),
                        Event::ChangeArea { area, town } => self.area = Some((area, town)),
                        e => self.events.push(e),
                    }
                }
                frame
            }
            Stage::Gone => Frame::new(),
        };
        self.add_play_time();
        if let Some(num) = change {
            // `ChangeRequest` interrupts the sound (`gameInterrupt`).
            self.events.push(Event::GameInterrupt);
            if matches!(self.stage, Stage::Title(_)) && num == request::DESKTOP {
                self.loading = Some((num, LOAD_FRAMES));
            } else if let Err(e) = self.change(num) {
                tracing::warn!("{e}");
            }
        }
        // `ccSetupGameCtrl`'s fade out over the scene that asked for the
        // change, its tasks still running under it; then the next scene.
        // ChangeRequest(6, 7) calls ccLayer::OffFlipExcept (main
        // 0x00108830): every layer but the fade's stops flipping, and
        // AddAll sends each one's last list again, so the screen holds the
        // picture of the frame before the request (the tasks' new draws go
        // unseen: a door's ChangeBlock has already put Kite in the next
        // block, the church's nave).
        let mut frame = frame;
        if let Some(k) = self.leaving {
            let held = self.shown.get_or_insert_with(|| frame.clone());
            frame = held.clone();
            let mut ctx = piney_desktop::anm::Ctx::new(piney_desktop::view::View::default());
            piney_desktop::fade::draw_on(
                &mut ctx,
                piney_world::draw::FADE_LAYER,
                0,
                0x8000_0000,
                k,
                piney_world::FADE_FRAMES,
            );
            frame.cmds.extend(ctx.finish().cmds);
            if k + 1 < piney_world::FADE_FRAMES {
                self.leaving = Some(k + 1);
            } else {
                self.leaving = None;
                self.shown = None;
                if let Err(e) = self.change_scene() {
                    tracing::warn!("{e}");
                }
            }
        } else if matches!(self.stage, Stage::World(_) | Stage::Area(_)) {
            self.shown = Some(frame.clone());
        } else {
            self.shown = None;
        }
        // ccLoadDispTh (priority 17) over the new scene, from the load.
        if let Some(ld) = self.load_disp.as_mut()
            && self.at_load
        {
            if ld.frame(self.load_started) {
                ld.draw(&mut frame);
            } else {
                self.load_disp = None;
            }
        }
        self.keep_settings();
        frame
    }

    fn hook(&mut self, name: &str) -> bool {
        match &mut self.stage {
            Stage::Area(a) => a.hook(name),
            _ => false,
        }
    }

    fn console(&mut self, line: &str) -> String {
        self.console_line(line)
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn vibration(&mut self) -> Option<bool> {
        // The title's save is its demo's (the Option pages write it).
        if let Stage::Title(t) = &self.stage {
            return Some(t.demo.save().u8(offset::VIBRATION) != 0);
        }
        self.save_mut().map(|s| s.u8(offset::VIBRATION) != 0)
    }

    fn save_copy(&mut self) -> Option<SaveData> {
        if let Stage::Title(t) = &self.stage {
            return Some(t.demo.save().clone());
        }
        self.save_mut().map(|s| s.clone())
    }

    fn frame_rate(&self) -> u32 {
        match &self.stage {
            Stage::Title(t) => t.demo.frame_rate(),
            Stage::Desktop(d) => d.frame_rate(),
            Stage::TopPage(t) => t.frame_rate(),
            Stage::World(w) => w.frame_rate(),
            Stage::Area(a) => a.frame_rate(),
            Stage::Gone => 1,
        }
    }

    fn archive(&self) -> Option<Arc<Archive>> {
        match &self.stage {
            Stage::Title(t) => t.stream.as_ref().map(StreamPlayer::archive),
            Stage::Desktop(d) => d.archive(),
            Stage::TopPage(t) => t.archive(),
            Stage::Area(a) => a.archive(),
            Stage::World(w) => w.archive(),
            Stage::Gone => None,
        }
    }

    fn title(&self) -> String {
        match &self.stage {
            Stage::Title(t) => match &t.stream {
                Some(p) => format!("title - {}", p.status()),
                None => format!("title - {}", t.demo.status()),
            },
            Stage::Desktop(d) => d.title(),
            Stage::TopPage(t) => t.title(),
            Stage::World(w) => w.title(),
            Stage::Area(a) => a.title(),
            Stage::Gone => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use piney_demo::Phase;
    use piney_demo::opening::act;
    use piney_input::{Buttons, Raw};

    use super::*;

    fn press(s: &mut Session, pad: &mut Pad, buttons: Buttons) -> Vec<Event> {
        pad.read(&Raw { buttons, ..Raw::default() });
        s.step(pad);
        s.take_events()
    }

    fn run(s: &mut Session, pad: &mut Pad, frames: std::ops::Range<u32>, presses: &[(u32, Buttons)]) {
        for f in frames {
            let buttons = presses.iter().filter(|p| p.0 == f).fold(Buttons::NONE, |b, p| b | p.1);
            press(s, pad, buttons);
        }
    }

    fn title(s: &Session) -> Option<&Title> {
        match &s.stage {
            Stage::Title(t) => Some(t),
            _ => None,
        }
    }

    /// From power-on to the title's menu, skipping the logo movies with
    /// START as a player would; each frame's events go to `each`.
    fn to_menu(s: &mut Session, pad: &mut Pad, mut each: impl FnMut(Vec<Event>)) {
        let mut n = 0;
        while !title(s).is_some_and(|t| matches!(t.demo.phase(), Phase::Title)) {
            let skip = title(s).is_some_and(|t| t.movie.is_some()) && n % 20 == 19;
            let events = press(s, pad, if skip { Buttons::START } else { Buttons::NONE });
            each(events);
            n += 1;
            assert!(n < 2000, "no menu: {}", Mode::title(s));
        }
    }

    /// NEWGAME from the menu (Up from DATALOAD, then OK), to the desktop's
    /// setup.
    fn new_game(s: &mut Session, pad: &mut Pad) {
        to_menu(s, pad, drop);
        // The menu's fade-in.
        run(s, pad, 0..40, &[(20, Buttons::UP), (30, Buttons::CROSS)]);
        let mut n = 0;
        while title(s).is_some() {
            press(s, pad, Buttons::NONE);
            n += 1;
            assert!(n < 200, "New Game does not leave the title: {}", Mode::title(s));
        }
    }

    /// Whether the desktop plays a stream.
    fn streaming(s: &Session) -> bool {
        matches!(&s.stage, Stage::Desktop(d) if d.streaming())
    }

    /// The press every 30 frames that gets a player through the desktop's
    /// setup: cancel and START skip a stream, Cross answers a line.
    fn setup_press(s: &Session, n: u32) -> Buttons {
        match n % 30 {
            29 if streaming(s) => Buttons::CIRCLE | Buttons::START,
            29 => Buttons::CROSS,
            _ => Buttons::NONE,
        }
    }

    /// Through the desktop's setup, skipping event 1's streams and pressing
    /// Cross for its lines; the frames it took.
    fn settle(s: &mut Session, pad: &mut Pad) -> u32 {
        let mut n = 0;
        while !matches!(&s.stage, Stage::Desktop(d) if d.playing()) {
            let buttons = setup_press(s, n);
            press(s, pad, buttons);
            n += 1;
            assert!(n < 3000, "the setup does not end: {}", Mode::title(s));
        }
        n
    }

    /// A formatted card with nothing on it: an empty directory of its own.
    fn empty_card() -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("piney-card-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A session with an empty card in slot 1, so the boot's card check
    /// passes without a question.
    fn session(scripts: bool) -> Option<Session> {
        session_with(scripts, Some(empty_card()))
    }

    fn session_with(scripts: bool, card: Option<PathBuf>) -> Option<Session> {
        session_on("infection", scripts, card)
    }

    /// A session on the disc extracted as `work/<disc>/<disc>.iso`.
    fn session_on(disc: &str, scripts: bool, card: Option<PathBuf>) -> Option<Session> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        if !iso.exists() {
            eprintln!("{disc}.iso not present; skipped");
            return None;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        Some(Session::new(iso, archive, scripts, card, false).unwrap())
    }

    /// One push of `b`, then a frame without it.
    fn tap(s: &mut Session, pad: &mut Pad, b: Buttons) {
        press(s, pad, b);
        press(s, pad, Buttons::NONE);
    }

    /// Frames until the title reaches `done`.
    fn until(s: &mut Session, pad: &mut Pad, max: u32, done: impl Fn(&Title) -> bool) {
        for _ in 0..max {
            if title(s).is_some_and(&done) {
                return;
            }
            press(s, pad, Buttons::NONE);
        }
        panic!("not reached in {max} frames: {}", Mode::title(s));
    }

    /// Power on, the title, New Game, the setup and the desktop with event
    /// 1's mail; START is one of the operations event 1 holds back until the
    /// first mails are read, so it does nothing.
    #[test]
    fn power_on_to_the_desktop() {
        let Some(mut s) = session(true) else { return };
        let mut pad = Pad::default();
        assert!(matches!(s.stage, Stage::Title(_)));
        new_game(&mut s, &mut pad);
        assert!(settle(&mut s, &mut pad) > 61);
        let Stage::Desktop(d) = &s.stage else { panic!("not on the desktop: {}", Mode::title(&s)) };
        assert!(d.title().starts_with("desktop - "), "{}", d.title());
        run(&mut s, &mut pad, 0..400, &[]);
        run(&mut s, &mut pad, 0..100, &[(0, Buttons::START)]);
        assert!(matches!(s.stage, Stage::Desktop(_)));
        // Event 1's mail in the order it came: Yasuhiko's, the port's from
        // Helba, CC Corporation's two. The mailer lists the newest first,
        // so Helba's shows straight below CC Corporation's.
        let helba = piney_event::extras::HELBA_MAIL;
        let save = s.save_mut().expect("the desktop's save");
        let order: Vec<i16> =
            (0..piney_data::save::MAIL_SLOTS).map(|i| save.mail_order(i)).take_while(|&m| m >= 0).collect();
        assert_eq!(order, [4, helba, 5, 320]);
    }

    /// Mutation from power-on to its desktop: the title, New Game, the
    /// setup's events (name entry, the streams) and the desktop.
    #[test]
    fn mutation_power_on_to_the_desktop() {
        let Some(mut s) = session_on("mutation", true, Some(empty_card())) else { return };
        let mut pad = Pad::default();
        assert!(matches!(s.stage, Stage::Title(_)), "{}", Mode::title(&s));
        new_game(&mut s, &mut pad);
        settle(&mut s, &mut pad);
        let Stage::Desktop(d) = &s.stage else { panic!("not on the desktop: {}", Mode::title(&s)) };
        assert!(d.title().starts_with("desktop - "), "{}", d.title());
        run(&mut s, &mut pad, 0..400, &[]);
        assert!(matches!(s.stage, Stage::Desktop(_)), "{}", Mode::title(&s));
        // The port's mail from Helba is in the box, already read.
        let helba = piney_event::extras::HELBA_MAIL as usize;
        let save = s.save_mut().expect("the desktop's save");
        assert_eq!(save.mail(helba), 4, "Helba's mail read");
        assert!((0..piney_data::save::MAIL_SLOTS).any(|i| save.mail_order(i) == helba as i16), "in the order list");
    }

    /// Each disc from New Game through its setup (the name entry, the
    /// opening streams) to the desktop, every event routed as `main` routes
    /// it into a headless audio engine: the sound's level each second with
    /// the mode, and the events that asked for sound. A diagnostic:
    /// `cargo test --release -p piney-game volume_sound_survey -- --ignored
    /// --nocapture` (`PINEY_SURVEY_ONLY=outbreak` for one disc).
    #[test]
    #[ignore]
    fn volume_sound_survey() {
        let only = std::env::var("PINEY_SURVEY_ONLY").ok();
        for disc in ["infection", "mutation", "outbreak", "quarantine"] {
            if only.as_deref().is_some_and(|o| o != disc) {
                continue;
            }
            // PINEY_SURVEY_BUILD=DIR: the disc as a build keeps it.
            let iso = match std::env::var_os("PINEY_SURVEY_BUILD") {
                Some(dir) => PathBuf::from(dir).join(format!("{disc}.disc")),
                None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso")),
            };
            let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
            let mut s = Session::new(iso.clone(), archive, true, Some(empty_card()), false).unwrap();
            let audio = piney_audio::Audio::headless(&iso).unwrap();
            let mut pad = Pad::default();
            new_game(&mut s, &mut pad);
            println!("== {disc}");
            let mut chunk = vec![0i16; 1600];
            let (mut sq, mut n) = (0f64, 0usize);
            let mut kinds: std::collections::BTreeMap<String, u32> = Default::default();
            for f in 0..5400u32 {
                // The streams play out; Cross answers a line.
                let buttons = if matches!(&s.stage, Stage::Desktop(d) if d.playing()) || streaming(&s) {
                    Buttons::NONE
                } else if f % 30 == 29 {
                    Buttons::CROSS
                } else {
                    Buttons::NONE
                };
                let events = press(&mut s, &mut pad, buttons);
                for e in &events {
                    let k = format!("{e:?}");
                    let k = k.split(['(', ' ', '{']).next().unwrap_or("").to_string();
                    *kinds.entry(k).or_default() += 1;
                }
                crate::handle(events, Some(&audio));
                audio.frame();
                audio.render(&mut chunk);
                sq += chunk.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>();
                n += chunk.len();
                if f % 60 == 59 {
                    let rms = (sq / n as f64).sqrt();
                    println!("{:4}s rms {rms:6.0}  {}", f / 60, Mode::title(&s));
                    (sq, n) = (0.0, 0);
                }
            }
            println!("events: {kinds:?}");
        }
    }

    /// New Game's name entry heard (the report: no sound from its keyboard
    /// nor from the confirm after). Every event through `main`'s routing into
    /// a headless engine: each press that asks for a sound (the dialog's 18,
    /// the keyboard's 6, 4 and 7) is heard over the 12 frames after it. The
    /// name entry runs in event 1's phase-0 pass, before `ccSetupDesktop`'s
    /// `ccAllSoundOff` zeroes the ports (0x0016847c).
    /// `PINEY_SURVEY_BUILD=DIR` plays a build's disc, `PINEY_SURVEY_CARD=DIR`
    /// a copy of a player's card, `PINEY_SURVEY_SKIP` skips the streams.
    #[test]
    fn the_name_entry_is_heard() {
        // PINEY_SURVEY_BUILD=DIR: the disc as a build keeps it.
        let iso = match std::env::var_os("PINEY_SURVEY_BUILD") {
            Some(dir) => PathBuf::from(dir).join("infection.disc"),
            None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"),
        };
        if !iso.exists() {
            return;
        }
        let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
        // PINEY_SURVEY_CARD=DIR: a copy of a player's card instead of an
        // empty one.
        let card = std::env::var_os("PINEY_SURVEY_CARD").map_or_else(empty_card, PathBuf::from);
        let mut s = Session::new(iso.clone(), archive, true, Some(card), true).unwrap();
        let audio = piney_audio::Audio::headless(&iso).unwrap();
        let mut pad = Pad::default();
        // NEWGAME as `new_game` picks it, every event routed: the title's
        // `GameInterrupt` and the setup's `AllSoundOff` among them.
        to_menu(&mut s, &mut pad, |events| crate::handle(events, Some(&audio)));
        let mut k = 0u32;
        while title(&s).is_some() {
            let b = match k {
                20 => Buttons::UP,
                30 => Buttons::CROSS,
                _ => Buttons::NONE,
            };
            crate::handle(press(&mut s, &mut pad, b), Some(&audio));
            k += 1;
            assert!(k < 300, "New Game does not leave the title: {}", Mode::title(&s));
        }
        let level = |s: &mut Session, pad: &mut Pad, b: Buttons, frames: u32| {
            let mut chunk = vec![0i16; 1600];
            let mut sq = 0f64;
            let mut ses = Vec::new();
            for k in 0..frames {
                let events = press(s, pad, if k == 0 { b } else { Buttons::NONE });
                ses.extend(events.iter().filter_map(|e| if let Event::Se(n) = e { Some(*n) } else { None }));
                crate::handle(events, Some(&audio));
                audio.frame();
                audio.render(&mut chunk);
                sq += chunk.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>();
            }
            ((sq / f64::from(frames * 1600)).sqrt(), ses)
        };
        // The setup's streams and lines to the name entry.
        let mut chunk = vec![0i16; 1600];
        let mut n = 0u32;
        while !matches!(&s.stage, Stage::Desktop(d) if d.naming()) {
            // PINEY_SURVEY_SKIP unset: the streams play out, as a player
            // who watches them has it.
            let skip = std::env::var_os("PINEY_SURVEY_SKIP").is_some();
            let b = if streaming(&s) {
                if skip && n % 30 == 29 { Buttons::CIRCLE | Buttons::START } else { Buttons::NONE }
            } else if n % 30 == 29 {
                Buttons::CROSS
            } else {
                Buttons::NONE
            };
            let events = press(&mut s, &mut pad, b);
            crate::handle(events, Some(&audio));
            audio.frame();
            audio.render(&mut chunk);
            n += 1;
            assert!(n < 20000, "no name entry: {}", Mode::title(&s));
        }
        level(&mut s, &mut pad, Buttons::NONE, 60);
        let mut heard = Vec::new();
        let keys = [
            Buttons::CROSS,
            Buttons::CROSS,
            Buttons::RIGHT,
            Buttons::RIGHT,
            Buttons::DOWN,
            Buttons::CROSS,
            Buttons::CIRCLE,
            Buttons::LEFT,
            Buttons::CROSS,
        ];
        for b in keys {
            let (quiet, _) = level(&mut s, &mut pad, Buttons::NONE, 12);
            let (loud, ses) = level(&mut s, &mut pad, b, 12);
            eprintln!("{b:?}: {quiet:.0} -> {loud:.0}, se {ses:?}");
            heard.push((loud > 1000.0, ses));
        }
        for (k, (louder, ses)) in heard.iter().enumerate() {
            if !ses.is_empty() {
                assert!(louder, "press {k}: sound {ses:?} asked for, none heard");
            }
        }
    }

    /// The options kept across the parts: the file's go into the title's
    /// new save (the screen's place moving the picture at power-on), and a
    /// change a menu makes to the save goes into the file.
    #[test]
    fn options_kept_across_the_parts() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let dir = std::env::current_exe().unwrap().parent().unwrap().join("settings-session");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        std::fs::write(&path, "main_volume = 100\nvoice = 1\nscreen_x = -12\nscreen_y = 7\n").unwrap();
        let mut s = Session::after_logos(iso, archive, false, None, false, 0, Some(path.clone())).unwrap();
        assert!(s.take_events().contains(&Event::DisplayOffset { x: -12, y: 7 }));
        let mut pad = Pad::default();
        pad.read(&Raw::default());
        s.step(&pad);
        let Stage::Title(t) = &mut s.stage else { panic!("not the title") };
        assert_eq!(t.demo.save().i16(offset::MAIN_VOL), 100);
        assert_eq!(t.demo.save().u8(offset::VOICE), 1);
        // A menu's change, taken into the file on the next frame.
        t.demo.save_mut().set_i16(offset::MAIN_VOL, 180);
        s.step(&pad);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("main_volume = 180") && text.contains("voice = 1"), "{text}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The logo movies play one picture every two frames, silent; OPENING
    /// plays with its sound; START stops it once twelve pictures are
    /// decoded and the title goes on to its music.
    #[test]
    fn movies_play() {
        let Some(mut s) = session(false) else { return };
        let audio = piney_audio::Audio::headless(&s.iso).unwrap();
        let mut pad = Pad::default();
        let mut out = vec![0i16; 2 * 800];
        // The three logos, silent: 146, 180 and 151 pictures, two frames
        // each, up to OPENING's sound.
        let mut frames = 0;
        loop {
            let playing = |s: &Session| title(s).is_some_and(|t| t.movie.is_some());
            let before = playing(&s);
            pad.read(&Raw::default());
            let frame = s.step(&pad);
            let events = s.take_events();
            let opening = events.iter().any(|e| matches!(e, Event::MovieAudio(_)));
            crate::handle(events, Some(&audio));
            audio.frame();
            audio.render(&mut out);
            if opening {
                break;
            }
            // A frame the player drew: not the one that opened a movie nor
            // the one that found it ended.
            if before && playing(&s) {
                let u = frame.uploads.first().expect("a movie frame without its picture");
                assert_eq!((u.width, u.height), (640, 448));
                assert!(out.iter().all(|&x| x == 0), "a logo is not silent");
                frames += 1;
            }
            assert!(frames < 2000, "no opening: {}", Mode::title(&s));
        }
        assert_eq!(frames, 2 * (146 + 180 + 151));
        // OPENING, with its sound: silent for its first 2.57 s.
        let mut heard = 0;
        for f in 0..240 {
            press(&mut s, &mut pad, Buttons::NONE);
            audio.frame();
            audio.render(&mut out);
            heard += u32::from(f >= 160 && out.iter().any(|&x| x != 0));
        }
        assert!(heard > 75, "OPENING heard for {heard} of 80 frames");
        let events = press(&mut s, &mut pad, Buttons::START);
        assert!(title(&s).is_some_and(|t| t.movie.is_none()));
        assert!(events.contains(&Event::MovieAudioStop));
        // Then the intro, stream 0, drawn from its own files.
        until(&mut s, &mut pad, 200, |t| t.stream.is_some());
        assert!(Mode::archive(&s).is_some());
        let drawn = (0..200)
            .filter(|_| {
                pad.read(&Raw::default());
                let frame = s.step(&pad);
                s.take_events();
                !frame.cmds.is_empty()
            })
            .count();
        assert!(drawn > 150, "stream 0 drew {drawn} of 200 frames");
    }

    /// From power-on to the opening stream's first frame, the logo movies
    /// skipped with START.
    fn to_opening_stream(s: &mut Session, pad: &mut Pad) {
        let mut n = 0;
        while !title(s).is_some_and(|t| t.stream.is_some()) {
            let skip = title(s).is_some_and(|t| t.movie.is_some()) && n % 20 == 19;
            press(s, pad, if skip { Buttons::START } else { Buttons::NONE });
            n += 1;
            assert!(n < 3000, "no opening stream: {}", Mode::title(s));
        }
    }

    /// The opening stream's flashes, frame by frame: (the stream's frame,
    /// streaming, scFadeDef active) after each step, `cancel` pushed at
    /// step `at`.
    fn opening_flashes(cancel_at: Option<u32>) -> Option<Vec<(u32, bool, bool)>> {
        let mut s = session(false)?;
        let mut pad = Pad::default();
        to_opening_stream(&mut s, &mut pad);
        let cancel = Buttons(u32::from(title(&s).unwrap().demo.save().assign_pad_cancel()));
        let mut seen = Vec::new();
        for f in 0..700 {
            press(&mut s, &mut pad, if cancel_at == Some(f) { cancel } else { Buttons::NONE });
            let Some(t) = title(&s) else { break };
            let frame = t.stream.as_ref().map_or(0, |p| p.frame().0);
            seen.push((frame, t.stream.is_some(), t.demo.fade().active()));
            if t.stream.is_none() && !t.demo.fade().active() {
                break;
            }
        }
        Some(seen)
    }

    /// `PlayOpeningStream`'s flashes over the stream (`scFadeDef`). Played
    /// through, the check sees the stream's frame 390 (410 - 20): white
    /// comes in over its last 20 frames (391-410) and the menu comes out
    /// of it, 47 frames lit in all. A cancel push flashes white at once,
    /// for 50 frames; the stream ends and no end flash follows.
    #[test]
    fn the_opening_stream_flashes() {
        let Some(through) = opening_flashes(None) else { return };
        let first = through.iter().position(|x| x.2).expect("no end flash");
        assert_eq!(through[first].0, 391);
        assert!(through[first..first + 20].iter().all(|x| x.1), "the flash over the stream");
        assert!(through.iter().skip_while(|x| x.1).any(|x| x.2), "the menu out of white");
        assert_eq!(through.iter().filter(|x| x.2).count(), 47);
        let Some(cancelled) = opening_flashes(Some(100)) else { return };
        assert_eq!(cancelled.iter().position(|x| x.2), Some(100));
        assert_eq!(cancelled.iter().filter(|x| x.2).count(), 50, "the cancel's flash, and no other");
    }

    /// The title's music plays, and event 1's setup lines are voiced: the
    /// requests reach the sound driver and channel 0 streams a line.
    #[test]
    fn setup_lines_are_voiced() {
        let Some(mut s) = session(true) else { return };
        let audio = piney_audio::Audio::headless(&s.iso).unwrap();
        let mut pad = Pad::default();
        // One frame of 48 kHz stereo.
        let mut out = vec![0i16; 2 * 800];
        let mut frame = |events: Vec<Event>| {
            crate::handle(events, Some(&audio));
            audio.frame();
            audio.render(&mut out);
            out.iter().any(|&x| x != 0)
        };
        to_menu(&mut s, &mut pad, |e| {
            frame(e);
        });
        // The menu's music.
        let music = (0..60).filter(|_| frame(press(&mut s, &mut pad, Buttons::NONE))).count();
        assert!(music > 55, "title music for {music} of 60 frames");
        let (mut voiced, mut fade, mut off) = (0, None, None);
        // Event 1's streams play 120 frames each before they are skipped.
        let (mut streamed, mut stream_heard) = (0, 0);
        for f in 0..1200 {
            let buttons = match f {
                20 => Buttons::UP,
                30 => Buttons::CROSS,
                _ if streaming(&s) && streamed > 0 && streamed % 120 == 0 => Buttons::CIRCLE | Buttons::START,
                _ if streaming(&s) => Buttons::NONE,
                f if f > 100 => setup_press(&s, f),
                _ => Buttons::NONE,
            };
            let was_streaming = streaming(&s);
            let events = press(&mut s, &mut pad, buttons);
            if events.iter().any(|e| matches!(e, Event::SqFade { .. })) {
                fade = Some(f);
            }
            if fade.is_some() && off.is_none() && events.contains(&Event::AllSoundOff) {
                off = Some(f);
            }
            let heard = frame(events);
            if was_streaming && streaming(&s) {
                streamed += 1;
                stream_heard += u32::from(heard);
            }
            voiced += u32::from(audio.voice_playing());
        }
        assert!(streamed >= 200, "event 1's streams played {streamed} frames");
        assert!(stream_heard > 100, "their sound heard in {stream_heard} of {streamed} frames");
        assert!(voiced > 60, "voiced for {voiced} frames");
        // New Game's fade runs its nine frames before the desktop's
        // `ccAllSoundOff`.
        let (fade, off) = (fade.unwrap(), off.unwrap());
        assert!(off - fade >= 9, "the fade was cut after {} frames", off - fade);
    }

    /// A save on the card (slot 3, with its index record, as the Data
    /// screen writes them), loaded from the title's DATALOAD: the desktop
    /// starts on it, its Data screen at the slot the load used.
    #[test]
    fn load_from_the_card() {
        use piney_desktop::savesys::{INDEX_SIZE, LOAD_DONE, LOAD_QUESTION, LOAD_SELECT};
        let dir = empty_card();
        let mut card = FilesCard::slot1(piney_data::volume::Volume::Inf, &dir);
        assert!(card.make_dir(0));
        let mut data = SaveState::fresh().save.bytes().to_vec();
        data[0x8400..0x8404].copy_from_slice(&123_456i32.to_le_bytes());
        let sum = data.iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b)));
        let mut index = [0u8; INDEX_SIZE];
        let rec = &mut index[28 * 2..28 * 3];
        rec[0] = 1;
        rec[1] = 7;
        rec[4..8].copy_from_slice(b"Kite");
        rec[0x16..0x18].copy_from_slice(&sum.to_le_bytes());
        rec[0x18..0x1c].copy_from_slice(&123_456i32.to_le_bytes());
        assert!(card.write_slot(0, 2, &data) && card.write_index(0, &index));

        let Some(mut s) = session_with(false, Some(dir.clone())) else { return };
        let mut pad = Pad::default();
        to_menu(&mut s, &mut pad, drop);
        run(&mut s, &mut pad, 0..40, &[]);
        // The menu starts on DATALOAD. OK, then MEMORY CARD slot 1, the
        // third save, YES (the upper line; NO is chosen first), and OK on
        // "Data loaded."
        tap(&mut s, &mut pad, Buttons::CROSS);
        until(&mut s, &mut pad, 100, |t| t.demo.opening().dat_sw != 0);
        tap(&mut s, &mut pad, Buttons::CROSS);
        until(&mut s, &mut pad, 20, |t| t.demo.save_sys().result == LOAD_SELECT);
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::CROSS);
        until(&mut s, &mut pad, 20, |t| t.demo.save_sys().result == LOAD_QUESTION);
        tap(&mut s, &mut pad, Buttons::UP);
        tap(&mut s, &mut pad, Buttons::CROSS);
        until(&mut s, &mut pad, 20, |t| t.demo.save_sys().result == LOAD_DONE);
        tap(&mut s, &mut pad, Buttons::CROSS);
        let mut n = 0;
        while title(&s).is_some() {
            press(&mut s, &mut pad, Buttons::NONE);
            n += 1;
            assert!(n < 400, "Load does not leave the title: {}", Mode::title(&s));
        }
        let what = Mode::title(&s);
        let Stage::Desktop(d) = std::mem::replace(&mut s.stage, Stage::Gone) else {
            panic!("not on the desktop: {what}")
        };
        let (state, _) = d.leave();
        // The loaded play time, and the desktop's first frames counted on it.
        let t = state.save.play_time();
        assert!((123_456..123_456 + 60).contains(&t), "play time {t}");
        assert_eq!(&state.save.bytes()[..4], b"Kite");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The title's OPTION is the desktop's system menu: Vibrate set OFF
    /// there is in the save New Game hands to the desktop.
    #[test]
    fn title_option_carries_into_the_game() {
        use piney_data::save::offset::VIBRATION;
        let Some(mut s) = session(false) else { return };
        let mut pad = Pad::default();
        to_menu(&mut s, &mut pad, drop);
        run(&mut s, &mut pad, 0..40, &[]);
        // DATALOAD to OPTION, OK: the OPTION list; Vibrate, then OFF.
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::CROSS);
        until(&mut s, &mut pad, 100, |t| t.demo.menu().menu_type() == 1);
        until(&mut s, &mut pad, 40, |t| t.demo.opening().opt_act == 2);
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::CROSS);
        until(&mut s, &mut pad, 20, |t| t.demo.menu().menu_type() == 3);
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::CROSS);
        assert_eq!(title(&s).unwrap().demo.save().u8(VIBRATION), 0);
        // Back out twice, then up to NEWGAME and OK.
        tap(&mut s, &mut pad, Buttons::CIRCLE);
        until(&mut s, &mut pad, 20, |t| t.demo.menu().menu_type() == 1);
        tap(&mut s, &mut pad, Buttons::CIRCLE);
        until(&mut s, &mut pad, 200, |t| t.demo.opening().main_act == act::NEUTRAL);
        assert_eq!(title(&s).unwrap().demo.menu().menu_type(), -1);
        tap(&mut s, &mut pad, Buttons::UP);
        tap(&mut s, &mut pad, Buttons::UP);
        tap(&mut s, &mut pad, Buttons::CROSS);
        let mut n = 0;
        while title(&s).is_some() {
            press(&mut s, &mut pad, Buttons::NONE);
            n += 1;
            assert!(n < 200, "New Game does not leave the title: {}", Mode::title(&s));
        }
        let what = Mode::title(&s);
        let Stage::Desktop(d) = std::mem::replace(&mut s.stage, Stage::Gone) else {
            panic!("not on the desktop: {what}")
        };
        assert_eq!(d.leave().0.save.u8(VIBRATION), 0);
    }

    /// The desktop's THE WORLD icon opens the top page (mode 4); Log out goes
    /// back to the desktop; Log in enters The World in the save's last town,
    /// Mac Anu, where the D-pad walks Kite.
    #[test]
    fn the_world_top_page_and_into_mac_anu() {
        let Some(mut s) = session(false) else { return };
        let mut pad = Pad::default();
        new_game(&mut s, &mut pad);
        settle(&mut s, &mut pad);
        // The desktop's opening ends about 400 frames after it starts; up
        // from MAILER to THE WORLD, OK.
        let to_top_page = |s: &mut Session, pad: &mut Pad| {
            run(s, pad, 0..425, &[]);
            tap(s, pad, Buttons::UP);
            run(s, pad, 0..20, &[]);
            tap(s, pad, Buttons::CROSS);
            let mut n = 0;
            while !matches!(s.stage, Stage::TopPage(_)) {
                press(s, pad, Buttons::NONE);
                n += 1;
                assert!(n < 200, "no top page: {}", Mode::title(s));
            }
            // Past the log-in animation to the menu.
            run(s, pad, 0..3, &[]);
            tap(s, pad, Buttons::CIRCLE);
            run(s, pad, 0..10, &[]);
        };
        to_top_page(&mut s, &mut pad);
        assert!(Mode::title(&s).starts_with("top page"), "{}", Mode::title(&s));
        // Log out: down to QUIT, OK, the 31-frame fade.
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..40, &[]);
        assert!(matches!(s.stage, Stage::Desktop(_)), "{}", Mode::title(&s));
        // Log in: OK on LOG IN; area 0 (a town) and the save's last town.
        settle(&mut s, &mut pad);
        to_top_page(&mut s, &mut pad);
        let town = match &s.stage {
            Stage::TopPage(_) => i32::from(SaveState::fresh().save.u8(offset::LAST_TOWN) as i8),
            _ => unreachable!(),
        };
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..40, &[]);
        assert_eq!(s.area, Some((0, town)));
        // The World: Mac Anu, at 30 frames a second, the town drawn.
        let Stage::World(w) = &s.stage else { panic!("not in The World: {}", Mode::title(&s)) };
        assert_eq!(w.frame_rate(), 2);
        let mut n = 0;
        while !Mode::title(&s).contains("play 100") {
            press(&mut s, &mut pad, Buttons::NONE);
            n += 1;
            assert!(n < 400, "no play: {}", Mode::title(&s));
        }
        pad.read(&Raw::default());
        assert!(!s.step(&pad).cmds.is_empty());
        // Up on the D-pad for two seconds: Kite walks off.
        let before = Mode::title(&s);
        for _ in 0..60 {
            pad.read(&Raw { buttons: Buttons::UP, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        let after = Mode::title(&s);
        let at = |t: &str| t.split("Kite at ").nth(1).map(str::to_string);
        assert_ne!(at(&before), at(&after), "{before} / {after}");
    }

    /// The desktop's Controller page reaches The World: START, Controller,
    /// down twice to B-1, OK (the save's `camType` 2), out of the menus;
    /// then THE WORLD, log in, and Mac Anu's camera is in scheme B-1 (R1/R2
    /// zoom, L1 resets, the right stick turns and tilts).
    #[test]
    fn the_desktop_controller_setting_reaches_the_world() {
        let Some(mut s) = session(false) else { return };
        let mut pad = Pad::default();
        new_game(&mut s, &mut pad);
        settle(&mut s, &mut pad);
        run(&mut s, &mut pad, 0..425, &[]);
        tap(&mut s, &mut pad, Buttons::START);
        run(&mut s, &mut pad, 0..14, &[]);
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..30, &[]);
        for _ in 0..2 {
            tap(&mut s, &mut pad, Buttons::DOWN);
            run(&mut s, &mut pad, 0..3, &[]);
        }
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..10, &[]);
        assert_eq!(s.save_mut().unwrap().u8(offset::CAM_TYPE), 2, "the save's camType");
        for _ in 0..2 {
            tap(&mut s, &mut pad, Buttons::CIRCLE);
            run(&mut s, &mut pad, 0..30, &[]);
        }
        tap(&mut s, &mut pad, Buttons::UP);
        run(&mut s, &mut pad, 0..20, &[]);
        tap(&mut s, &mut pad, Buttons::CROSS);
        let mut n = 0;
        while !matches!(s.stage, Stage::TopPage(_)) {
            press(&mut s, &mut pad, Buttons::NONE);
            n += 1;
            assert!(n < 200, "no top page: {}", Mode::title(&s));
        }
        run(&mut s, &mut pad, 0..3, &[]);
        tap(&mut s, &mut pad, Buttons::CIRCLE);
        run(&mut s, &mut pad, 0..10, &[]);
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..40, &[]);
        let Stage::World(w) = &s.stage else { panic!("not in The World: {}", Mode::title(&s)) };
        assert_eq!(w.world().camera().scheme, piney_world::camera::Scheme::new(2));
    }

    /// Mutation without its events: New Game, the desktop, THE WORLD, log
    /// in to the save's town, play, and Kite walks.
    #[test]
    fn mutation_the_world_and_into_the_town() {
        let Some(mut s) = session_on("mutation", false, Some(empty_card())) else { return };
        let mut pad = Pad::default();
        new_game(&mut s, &mut pad);
        settle(&mut s, &mut pad);
        run(&mut s, &mut pad, 0..425, &[]);
        tap(&mut s, &mut pad, Buttons::UP);
        run(&mut s, &mut pad, 0..20, &[]);
        tap(&mut s, &mut pad, Buttons::CROSS);
        let mut n = 0;
        while !matches!(s.stage, Stage::TopPage(_)) {
            press(&mut s, &mut pad, Buttons::NONE);
            n += 1;
            assert!(n < 200, "no top page: {}", Mode::title(&s));
        }
        run(&mut s, &mut pad, 0..3, &[]);
        tap(&mut s, &mut pad, Buttons::CIRCLE);
        run(&mut s, &mut pad, 0..10, &[]);
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..40, &[]);
        let Stage::World(_) = &s.stage else { panic!("not in The World: {}", Mode::title(&s)) };
        let mut n = 0;
        while !Mode::title(&s).contains("play 100") {
            press(&mut s, &mut pad, Buttons::NONE);
            n += 1;
            assert!(n < 400, "no play: {}", Mode::title(&s));
        }
        let before = Mode::title(&s);
        for _ in 0..60 {
            pad.read(&Raw { buttons: Buttons::UP, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        let after = Mode::title(&s);
        let at = |t: &str| t.split("Kite at ").nth(1).map(str::to_string);
        assert_ne!(at(&before), at(&after), "{before} / {after}");
    }

    /// Without the events holding it back: START, down to Title Screen (the
    /// last of seven rows), OK, and YES to the two questions - a soft reset
    /// back to the title, whose logos it skips.
    #[test]
    fn title_screen_resets() {
        let Some(mut s) = session(false) else { return };
        let mut pad = Pad::default();
        new_game(&mut s, &mut pad);
        settle(&mut s, &mut pad);
        assert!(matches!(s.stage, Stage::Desktop(_)), "{}", Mode::title(&s));
        // The desktop's opening ends about 400 frames after it starts.
        run(&mut s, &mut pad, 0..425, &[]);
        let mut presses = vec![(0, Buttons::START)];
        presses.extend((0..6).map(|i| (20 + 10 * i, Buttons::DOWN)));
        // Each question starts on NO (`ResetMenu` selects 1): up to YES, OK.
        presses.extend([
            (100, Buttons::CROSS),
            (130, Buttons::UP),
            (150, Buttons::CROSS),
            (190, Buttons::UP),
            (210, Buttons::CROSS),
        ]);
        run(&mut s, &mut pad, 0..300, &presses);
        assert!(title(&s).is_some_and(|t| t.movie.is_none()), "{}", Mode::title(&s));
    }

    /// The story player on the desktop, a press every twelfth frame: the
    /// mail unread read (each opened from the list, then closed), then the
    /// headlines unread, then The World. OK is CROSS and cancel CIRCLE (the
    /// new game's buttons); the setup's windows take CROSS.
    fn desktop_player(d: &DesktopMode, f: u64) -> Raw {
        use piney_desktop::desktop::Icon;
        use piney_desktop::save::MailState;
        let press = |buttons| Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        if !f.is_multiple_of(12) {
            return press(Buttons::NONE);
        }
        let Some(desk) = d.desktop() else { return press(Buttons::CROSS) };
        // An event block playing: its windows take OK.
        if d.vm().is_some_and(|v| v.playing().is_some()) {
            return press(if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
        }
        let save = desk.state();
        let mail_unread = |n: usize| matches!(save.mail(n), MailState::New | MailState::Unread);
        let news = desk.news();
        // PINEY_SURVEY_TRACE: the desktop's state every 120 frames.
        if std::env::var("PINEY_SURVEY_TRACE").is_ok() && f.is_multiple_of(120) {
            let m = desk.mailer();
            eprintln!(
                "desktop f{f}: events {:?} icon {} operate {:x} menu {} {} {} {} mode {:?} mail {} {:?} cursor {} unread {:?} news {} {:?}",
                d.vm().map(|v| (v.phase(), v.playing())),
                desk.selection(),
                save.operate,
                desk.menu().menu,
                desk.menu().menu_status,
                desk.menu().open_req,
                desk.menu().proccess,
                desk.mode(),
                m.mail_mode,
                m.reply_state(),
                m.mail_no,
                (0..512).filter(|&n| mail_unread(n)).collect::<Vec<_>>(),
                news.web_mode,
                news.list.iter().filter(|&&r| news.unread(r)).collect::<Vec<_>>()
            );
        }
        // The cursor to row `i` of a list, then OK.
        let to = |at: i32, i: usize| match at.cmp(&(i as i32)) {
            std::cmp::Ordering::Less => Buttons::DOWN,
            std::cmp::Ordering::Greater => Buttons::UP,
            std::cmp::Ordering::Equal => Buttons::CROSS,
        };
        press(match desk.mode() {
            Some(Icon::Mail) => {
                let m = desk.mailer();
                match m.mail_mode {
                    1 => m.list.iter().position(|&n| mail_unread(n)).map_or(Buttons::CIRCLE, |i| to(m.mail_no, i)),
                    2 => Buttons::CIRCLE,
                    // A mail with replies: OK through the text, the first
                    // reply, and YES (up from NO) to send it.
                    3 => match m.reply_state() {
                        (_, _, 14, _, 0) => Buttons::UP,
                        _ => Buttons::CROSS,
                    },
                    _ => Buttons::NONE,
                }
            }
            Some(Icon::News) => match news.web_mode {
                1 => news.list.iter().position(|&r| news.unread(r)).map_or(Buttons::CIRCLE, |i| to(news.list_no, i)),
                3 => Buttons::CIRCLE,
                _ => Buttons::NONE,
            },
            Some(_) => Buttons::CIRCLE,
            // Mail an event sends while the desktop is up
            // is listed only at its next opening (`AddAllList`): only the
            // listed inbox counts, or the pilot goes in and out for ever.
            None => {
                let target = if desk.mailer().list.iter().any(|&n| mail_unread(n)) {
                    Icon::Mail
                } else if news.list.iter().any(|&r| news.unread(r)) {
                    Icon::News
                } else {
                    Icon::World
                };
                match desk.selection() {
                    s if s == target as i32 => Buttons::CROSS,
                    0..=5 => Buttons::RIGHT,
                    _ => Buttons::NONE,
                }
            }
        })
    }

    /// The story player on The World's top page: Log in, a press every
    /// twelfth frame (the board left with cancel).
    fn top_page_player(t: &TopPageMode, f: u64) -> Raw {
        use piney_toppage::control::{CMD_BBS, CMD_LOGIN, CMD_QUIT, MODE_BBS, MODE_NORMAL};
        let press = |buttons| Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        // An event's stream over the board: cancel and START skip it; its
        // windows take OK.
        if t.streaming() {
            return press(if f.is_multiple_of(30) { Buttons::CIRCLE | Buttons::START } else { Buttons::NONE });
        }
        if t.event_playing() {
            return press(if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
        }
        if !f.is_multiple_of(12) {
            return press(Buttons::NONE);
        }
        let Some(page) = t.page() else { return press(Buttons::CROSS) };
        let ctl = page.control();
        // Kite's own post waiting to be written (`bbs_post7`, event 10's)
        // or a post new on the board: the board (`bbs_read`). A mail unread,
        // or the story wanting only the desktop (event 208's news): Log out
        // to the desktop (the events' `mail_got`, `news_read`); else Log in.
        let save = &page.state().save;
        let wants = t.vm().map(|vm| story_wants(vm, save)).unwrap_or_default();
        let desk =
            wants.contains(&Want::Leave { desktop: true }) && wants.iter().all(|w| matches!(w, Want::Leave { .. }));
        let to_desktop = desk || (0..piney_data::save::MAIL_SLOTS).any(|n| matches!(save.mail(n), 1 | 2));
        // A command the events take over (`add_operate`, command + 6) is
        // passed by: its message would only come round again.
        let taken = |c: i32| page.state().operate & (1u64 << (c + piney_toppage::control::OPERATE_BASE[0])) != 0;
        let writing = piney_toppage::bbs::check_write_bbs(page.state()).0 >= 0;
        // New posts are read before leaving: a block may wait on them here
        // while another waits on the desktop (event 23's four posts, then
        // its mail).
        let want = if (writing || ctl.bbs_new) && !taken(CMD_BBS) {
            CMD_BBS
        } else if to_desktop && !taken(CMD_QUIT) {
            CMD_QUIT
        } else if taken(CMD_LOGIN) && !taken(CMD_QUIT) {
            // Log in and the board both taken (event 107's end): Quit.
            CMD_QUIT
        } else {
            CMD_LOGIN
        };
        let row = |at: i32, to: usize| match at {
            i if i as usize == to => Buttons::CROSS,
            i if (i as usize) < to => Buttons::DOWN,
            _ => Buttons::UP,
        };
        press(match ctl.mode {
            MODE_NORMAL if ctl.cmd < want => Buttons::DOWN,
            MODE_NORMAL if ctl.cmd > want => Buttons::UP,
            MODE_NORMAL => Buttons::CROSS,
            // The board: each thread with a new post, each new post opened
            // (read on closing), then out.
            MODE_BBS => {
                let b = &ctl.bbs;
                let fresh = |t: &piney_toppage::bbs::ThreadObj| t.msgs.iter().position(|m| m.read == 1);
                match b.draw_state {
                    0 => match b.threads.iter().position(|t| fresh(t).is_some()) {
                        Some(r) => row(b.thread_page.index, r),
                        None => Buttons::CIRCLE,
                    },
                    1 if b.msg_page.i_draw_state == 1 => Buttons::CIRCLE,
                    1 => match b.threads.get(b.thread_page.index.max(0) as usize).and_then(fresh) {
                        Some(r) => row(b.msg_page.index, r),
                        None => Buttons::CIRCLE,
                    },
                    // A post to write (state 7): OK types it out, OK posts it.
                    2 => Buttons::CROSS,
                    _ => Buttons::CIRCLE,
                }
            }
            _ => Buttons::NONE,
        })
    }

    /// The event 2 player of `world.rs`'s test: CROSS every 24 frames while
    /// a window waits, and the tutorial menus walked; in a field or dungeon,
    /// the same for the windows and the camera tutorial's prompts held (a
    /// scheme A player: L1 to turn, the right stick to zoom, R2 to reset).
    fn story_player(s: &Session, f: u64) -> Raw {
        story_player_with(s, f, &[], &[])
    }

    /// [`story_player`], passing by the town NPCs in `talked`.
    pub(super) fn story_player_with(
        s: &Session,
        f: u64,
        talked: &[(piney_world::entry::Kind, i32)],
        buys: &[Buy],
    ) -> Raw {
        if let Stage::World(w) = &s.stage
            && !w.streaming()
            && let Some(raw) = gate_player(w, f, talked, buys)
        {
            return raw;
        }
        let (ui, calls) = match &s.stage {
            // An event's stream in a town, field or dungeon: cancel and
            // START skip it.
            Stage::World(w) if w.streaming() => {
                let buttons = if f.is_multiple_of(30) { Buttons::CIRCLE | Buttons::START } else { Buttons::NONE };
                return Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            }
            Stage::World(w) => (w.ui(), w.calls()),
            Stage::Area(a) if a.streaming() => {
                let buttons = if f.is_multiple_of(30) { Buttons::CIRCLE | Buttons::START } else { Buttons::NONE };
                return Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            }
            Stage::Area(a) => (a.ui(), a.calls()),
            Stage::Desktop(d) => return desktop_player(d, f),
            Stage::TopPage(t) => return top_page_player(t, f),
            _ => return Raw::default(),
        };
        let c = &ui.ctrl;
        let go_to = |row: i16| match c.list().select {
            s if s == row => Buttons::CROSS,
            s if s < row => Buttons::DOWN,
            _ => Buttons::UP,
        };
        let raw =
            |buttons: Buttons| Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        // The camera tutorial's prompt open: its input held.
        let last_open = calls.iter().rev().find_map(|(_, c)| {
            if c.starts_with("message_open") || c.starts_with("announce") || c.starts_with("info_lines") {
                Some(c.as_str())
            } else if c.starts_with("message_check") || c.starts_with("message_close") {
                Some("")
            } else {
                None
            }
        });
        match last_open {
            Some(c) if c.starts_with("message_open 3 55") || c.starts_with("message_open 3 7 ") => {
                return raw(Buttons::L1);
            }
            Some(c) if c.starts_with("message_open 3 56") || c.starts_with("message_open 3 9 ") => {
                return Raw { ry: 0, ..raw(Buttons::NONE) };
            }
            Some(c) if c.starts_with("message_open 3 57") || c.starts_with("message_open 3 11 ") => {
                return raw(if f.is_multiple_of(24) { Buttons::R2 } else { Buttons::NONE });
            }
            _ => {}
        }
        if !f.is_multiple_of(8) {
            return raw(Buttons::NONE);
        }
        match (ui.menu_type(), c.proccess) {
            (75, 1) => return raw(Buttons::TRIANGLE),
            (75, 2) => return raw(go_to(6)),
            (76, 2) => return raw(go_to(0)),
            (78, 4) => return raw(go_to(1)),
            (77..=79, _) => return raw(Buttons::CROSS),
            // Event 3's skill lesson: PERSONAL, Skills, the third page's
            // first skill (Repth), a party member.
            (80, 1) => return raw(Buttons::TRIANGLE),
            (80, 2) => return raw(go_to(0)),
            (81, 2) if c.list().page < 2 => return raw(Buttons::RIGHT),
            (81, 2) => return raw(go_to(0)),
            (82, 2) => return raw(Buttons::CROSS),
            // The chat lesson: CHAT, the first page's second row.
            (83, 1) => return raw(Buttons::SQUARE),
            (83, 20) => return raw(go_to(1)),
            // Event 4's treasure boxes (84, 85: the action button opens
            // the box) and the item they give (29); a box, a trapped box
            // and an idol opened (32, 33, 38 and its items' 67).
            (84 | 85 | 29 | 32 | 33 | 38 | 67, _) => return raw(Buttons::CROSS),
            (-1, _) => {}
            _ => return raw(Buttons::NONE),
        }
        let waiting = last_open.is_some_and(|c| !c.is_empty());
        // player_skill: the attack button until the goblin is down.
        let fighting = calls.iter().rev().find_map(|(_, c)| match c.as_str() {
            "player_skill" => Some(true),
            "player_skill done" => Some(false),
            _ => None,
        });
        if fighting == Some(true) {
            return raw(Buttons::CROSS);
        }
        raw(if waiting && f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE })
    }

    /// The story areas the events marked on the town's server
    /// (`gate_mark`: `gateListMark[server]`) that its Word List
    /// (`gateOrderList[server]`) holds, by their row in the list.
    fn marked_areas(w: &crate::world::WorldMode) -> Vec<(usize, i16)> {
        use piney_fieldui::menus::gate::{GATE_LIST_MARK, GATE_ORDER_LIST};
        let save = &w.world().state().save;
        let server = w.server().clamp(0, 4) as usize;
        let marked = |a: i16| {
            let a = a as usize;
            save.i32(GATE_LIST_MARK + 20 * server + 4 * (a / 32)) as u32 & (1 << (a % 32)) != 0
        };
        (0..64)
            .map(|k| save.i16(GATE_ORDER_LIST + 128 * server + 2 * k))
            .filter(|&a| a >= 0)
            .enumerate()
            .filter(|&(_, a)| marked(a))
            .collect()
    }

    /// The row, in the gate's Towns list (`townMoveFlag`'s towns but this
    /// one), of a town on a server where the events marked a story area.
    fn marked_town(w: &crate::world::WorldMode) -> Option<usize> {
        use piney_fieldui::menus::gate::{GATE_LIST_MARK, TOWN_MOVE_FLAG};
        let save = &w.world().state().save;
        let here = w.town_number();
        let flag = i32::from(save.i16(TOWN_MOVE_FLAG));
        let towns: Vec<i32> = (0..5).filter(|&t| flag & (1 << t) != 0 && t != here).collect();
        let marked = |server: i32| (0..5).any(|k| save.i32(GATE_LIST_MARK + 20 * server as usize + 4 * k) != 0);
        towns.iter().position(|&t| {
            let s = piney_world::area::SERVER_OF_TOWN[t as usize];
            s != w.server() && marked(s)
        })
    }

    /// What the volume's main story (or the side event [`Follow`] names)
    /// waits for next, read from its scripts: for every open event (its
    /// `event_done` preconditions met) not done or closed, the blocks not
    /// yet played whose status settings and conditions hold, walking the
    /// precondition settings in as the event walk does. A block that wants
    /// someone `not_in_party`, or an answer, is a refusal and wants
    /// nothing, and a member is wanted only where such a refusal names him.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(super) enum Want {
        Town(i32),
        /// A story area, by its words.
        Area(i16),
        /// That area's dungeon, by its index (a lake's 1 is below it).
        Dungeon(i16, i16),
        /// A block of that field's story map (area 15's church, 1), which
        /// its door swaps to.
        FieldBlock(i16, i16),
        /// Event point `num` of the dungeon.
        Point(i32),
        Party(i32),
        /// Kite alone: a block takes a wanted area off the gate's barred
        /// list (`del_area_code`) only with no one else in the party
        /// (`not_in_party -1`: event 204's area 15).
        Alone,
        /// The desktop or the top page (a block set on `game_status` 2 or
        /// 3): `desktop` for 2 (event 208's news).
        Leave {
            desktop: bool,
        },
        /// That field's foes all down: a block there waits on `no_active`
        /// after the event put a foe there (`entry 5`: side event 250's
        /// golden goblin, which runs).
        Clear(i16),
        /// Kite near event position `n` of the place: a block waits on
        /// `near_marker n` (side event 257's trader at point 1).
        Marker(i16),
        /// Kite with this member and no one else: a block refuses the gate
        /// while anyone else is along (`party_other`, event 21's Elk).
        Only(i32),
        /// An important item (category 15) Kite lacks, which a block waits
        /// on (`has_item`: event 22's cures); a story room's statue holds it.
        Item(i16),
    }

    thread_local! {
        /// The side event [`story_wants`] reads in place of the main story.
        static FOLLOWED: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
    }

    /// While held, the autopilot follows side event `n` alone (the side
    /// event surveys); the main story again once dropped.
    pub(super) struct Follow;

    impl Follow {
        pub(super) fn side(n: i32) -> Follow {
            FOLLOWED.set(Some(n));
            Follow
        }
    }

    impl Drop for Follow {
        fn drop(&mut self) {
            FOLLOWED.set(None);
        }
    }

    pub(super) fn story_wants(vm: &piney_event::vm::Vm, save: &piney_data::save::SaveData) -> Vec<Want> {
        use piney_event::ir::{Cond, Tag};
        let lib = vm.library();
        let base = 100 * (lib.volume.number() - 1);
        let done = |e: i16| save.event_flag(e.max(0) as usize) & (1 << 62) != 0;
        let status = |i: i16| i32::from(save.event_status(i.clamp(0, 79) as usize));
        let mut out = Vec::new();
        let events = FOLLOWED.get().map_or(base..base + 50, |n| n..n + 1);
        for n in events {
            let Some(script) = lib.script(n) else { continue };
            let flag = save.event_flag(n as usize);
            if flag & (3 << 62) != 0 {
                continue;
            }
            if !script.open.iter().all(|c| !matches!(*c, Cond::EventDone { event } if !done(event))) {
                continue;
            }
            // The members the event refuses to go on without (a block
            // that wants them `not_in_party` stops the player): the only
            // `in_party` wants. Other `in_party` blocks are lines for
            // whoever came along.
            let required: Vec<i16> = script
                .blocks
                .iter()
                .flat_map(|b| &b.conds)
                .filter_map(|c| match *c {
                    Cond::NotInParty { pc } => Some(pc),
                    _ => None,
                })
                .collect();
            let mut scene = [-1i16; 6];
            let mut point = -1i32;
            let mut after = -1i16;
            let mut held = true;
            // The `game_status` (2 or 3) the block is set on, off The World.
            let mut away: Option<i16> = None;
            // The fields where the event put a foe (`entry 5`).
            let mut foes: Vec<i16> = Vec::new();
            for (b, block) in script.blocks.iter().enumerate() {
                for t in &block.tags {
                    match *t {
                        Tag::GameStatus { status } if status != 5 => {
                            (scene, point) = ([-1; 6], -1);
                            away = matches!(status, 2 | 3).then_some(status);
                        }
                        Tag::GameStatus { .. } => away = None,
                        Tag::Scene { area, town, field, dungeon, floor, block } => {
                            (scene, point, away) = ([area, town, field, dungeon, floor, block], -1, None)
                        }
                        Tag::InTown { town } => (scene, point, away) = ([0, town, -1, -1, -1, -1], -1, None),
                        Tag::InField { town, field } => (scene, point, away) = ([1, town, field, -1, -1, -1], -1, None),
                        Tag::InDungeon { town, field, dungeon } => {
                            (scene, point, away) = ([2, town, field, dungeon, -1, -1], -1, None)
                        }
                        Tag::BlockDone { num } => after = num,
                        Tag::InPoint { num } => point = i32::from(num),
                        Tag::Status { index, num, comp } => {
                            held = comp.test(status(index), i32::from(num)).unwrap_or(false)
                        }
                        Tag::StatusRange { index, lo, hi } => {
                            held = (i32::from(lo)..=i32::from(hi)).contains(&status(index))
                        }
                        _ => {}
                    }
                }
                if scene[0] == 1 && block.ops.iter().any(|o| matches!(o, piney_event::ir::Op::Entry { ty: 5, .. })) {
                    foes.push(scene[2]);
                }
                if b < 62 && flag & (1 << b) != 0 {
                    continue;
                }
                // A block that plays again each time is a side line: its
                // event point is not where the story goes next (event 115's
                // point 3, 113's point 2).
                let repeats = block.ops.iter().any(|o| matches!(o, piney_event::ir::Op::Repeatable {}));
                if !held || (after >= 0 && flag & (1 << (after & 63)) == 0) {
                    continue;
                }
                if let Some(pc) = block.conds.iter().find_map(|c| match *c {
                    Cond::PartyOther { pc } => Some(i32::from(pc)),
                    _ => None,
                }) {
                    out.push(Want::Only(pc));
                }
                let reachable = block.conds.iter().all(|c| match *c {
                    Cond::Status { index, num, comp } => comp.test(status(index), i32::from(num)).unwrap_or(false),
                    Cond::StatusRange { index, lo, hi } => (i32::from(lo)..=i32::from(hi)).contains(&status(index)),
                    Cond::EventDone { event } => done(event),
                    Cond::HasItem { pc, category, id, num, comp } => {
                        piney_event::vm::has_item(save, pc, category, id, num, comp)
                    }
                    Cond::NotInParty { .. } | Cond::Answer { .. } => false,
                    _ => true,
                });
                // A block that unbars an area only with Kite alone is no
                // refusal: it is the way there.
                let unbars = block.ops.iter().any(|o| matches!(o, piney_event::ir::Op::DelAreaCode { .. }));
                if unbars && block.conds.iter().any(|c| matches!(*c, Cond::NotInParty { pc: -1 })) {
                    out.push(Want::Alone);
                }
                if !reachable {
                    // Short of nothing but an important item: the way on.
                    let item = block.conds.iter().find_map(|c| match *c {
                        Cond::HasItem { pc: 0, category: 15, id, num, comp }
                            if !piney_event::vm::has_item(save, 0, 15, id, num, comp) =>
                        {
                            Some(id)
                        }
                        _ => None,
                    });
                    let rest = block.conds.iter().all(|c| match *c {
                        Cond::HasItem { .. } => true,
                        Cond::Status { index, num, comp } => comp.test(status(index), i32::from(num)).unwrap_or(false),
                        Cond::StatusRange { index, lo, hi } => (i32::from(lo)..=i32::from(hi)).contains(&status(index)),
                        Cond::EventDone { event } => done(event),
                        Cond::NotInParty { .. } | Cond::Answer { .. } => false,
                        _ => true,
                    });
                    if let Some(id) = item.filter(|_| rest) {
                        out.push(Want::Item(id));
                    }
                    continue;
                }
                if let Some(status) = away {
                    out.push(Want::Leave { desktop: status == 2 });
                }
                let clear = block.conds.iter().any(|c| matches!(c, Cond::NoActive {}));
                match scene {
                    [0, town, ..] if town >= 0 => out.push(Want::Town(i32::from(town))),
                    [1, _, field, .., block] if field >= 0 => {
                        out.push(Want::Area(field));
                        if block >= 0 {
                            out.push(Want::FieldBlock(field, block));
                        }
                        if clear && foes.contains(&field) {
                            out.push(Want::Clear(field));
                        }
                    }
                    [2, _, field, dungeon, ..] if field >= 0 => {
                        out.extend([Want::Area(field), Want::Dungeon(field, dungeon)])
                    }
                    _ => {}
                }
                if point >= 0 && !repeats {
                    out.push(Want::Point(point));
                }
                for c in &block.conds {
                    match *c {
                        Cond::GateWords { area, .. } => out.push(Want::Area(area)),
                        Cond::InParty { pc } if required.contains(&pc) => out.push(Want::Party(i32::from(pc))),
                        Cond::InPoint { num } if !repeats => out.push(Want::Point(i32::from(num))),
                        Cond::NearMarker { marker, bounds, comp }
                            if !repeats && comp.test(0, i32::from(bounds)) == Some(true) =>
                        {
                            out.push(Want::Marker(marker))
                        }
                        _ => {}
                    }
                }
            }
        }
        out
    }

    /// The row of `town` in the gate's Towns list (`townMoveFlag`'s towns
    /// but this one).
    fn town_row(w: &crate::world::WorldMode, town: i32) -> Option<usize> {
        use piney_fieldui::menus::gate::TOWN_MOVE_FLAG;
        let save = &w.world().state().save;
        let here = w.town_number();
        let flag = i32::from(save.i16(TOWN_MOVE_FLAG));
        (0..5).filter(|&t| flag & (1 << t) != 0 && t != here).position(|t| t == town)
    }

    /// The row of `area` in this server's Word List, or the town (by its
    /// row in the Towns list) on a server whose list has it.
    fn area_way(w: &crate::world::WorldMode, area: i16) -> Option<GateGoal> {
        use piney_fieldui::menus::gate::{GATE_ORDER_LIST, TOWN_MOVE_FLAG};
        let save = &w.world().state().save;
        let list = |server: usize| -> Vec<i16> {
            (0..64).map(|k| save.i16(GATE_ORDER_LIST + 128 * server + 2 * k)).filter(|&a| a >= 0).collect()
        };
        let server = w.server().clamp(0, 4) as usize;
        if let Some(row) = list(server).iter().position(|&a| a == area) {
            return Some(GateGoal::Area(row));
        }
        let flag = i32::from(save.i16(TOWN_MOVE_FLAG));
        let town = (0..5).find(|&t| {
            flag & (1 << t) != 0 && list(piney_world::area::SERVER_OF_TOWN[t as usize] as usize).contains(&area)
        })?;
        town_row(w, town).map(GateGoal::Town)
    }

    /// The member to call into the party: of those with an address who
    /// answer calls (`callable`'s bits) and are not in it, one who knows
    /// Rip Maen, else one with a recovery skill, else the first.
    fn companion(w: &crate::world::WorldMode, party: &[i32], callable: u32) -> Option<i32> {
        let state = w.world().state();
        let items = &w.ui().texts().items;
        let skills = |pc: i32| (0..20).map(move |k| piney_fieldui::items::save_skill(state, pc as usize, k));
        let rank = |pc: i32| {
            if skills(pc).any(|x| x == 180) {
                0
            } else if skills(pc).any(|x| x >= 0 && items.skill(i32::from(x)).is_some_and(|p| p.kind & 0x40000 != 0)) {
                1
            } else {
                2
            }
        };
        (1..32).filter(|&pc| callable & (1 << pc) != 0 && !party.contains(&pc)).min_by_key(|&pc| rank(pc))
    }

    /// What the story autopilot goes to the Chaos Gate for.
    #[derive(Clone, Copy)]
    pub(super) enum GateGoal {
        /// The marked story area at this row of the Word List.
        Area(usize),
        /// The town at this row of the Towns list, whose server has one.
        Town(usize),
        /// Log out: a mail waits unread (the events' `mail_got` wants it
        /// read on the desktop), or a post on the board (`bbs_read`).
        LogOut,
        /// Speak to this one in the town, an event's target (`add_target`):
        /// an NPC or merchant, a party character (event 11's BlackRose) or
        /// the Chaos Gate (event 11's, type 13).
        Talk(piney_world::entry::Kind, i32),
        /// Walk to this marker of the town (`markerEvTbl`), which an
        /// event's `near_marker` waits for (event 13's 31).
        Marker(i16),
        /// Call this member into the party (PERSONAL, Party, Add).
        Invite(i32),
        /// Send the members away (PERSONAL, Party, Disband): the story
        /// wants Kite alone.
        Disband,
        /// Buy from this merchant (an NPC's code) what [`Buy`] asks for.
        Shop(i32),
    }

    /// An item the autopilot carries out of town: so many of it, bought
    /// at a shop of the town when Kite has fewer (the survey pilot's
    /// Speed Charms for golden goblins).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) struct Buy {
        pub(super) item: (i8, i16),
        pub(super) carry: i32,
    }

    /// The Buy menu's stock row of `item` at merchant `code` of this town:
    /// its list by the merchant's type (`BuyMenu`: 0x100 the equipment's,
    /// 0x800 the magic's, else the items') on this server.
    fn stock_row(w: &crate::world::WorldMode, code: i32, (cat, id): (i8, i16)) -> Option<usize> {
        let world = w.world();
        let ty = world.merchants().iter().find(|m| piney_world::entry::Npc::code(*m) == code)?.flags;
        let f = piney_data::tables::fieldui::of(world.volume());
        let lists = match ty {
            t if t & 0x100 != 0 => f.equip_shop(),
            t if t & 0x800 != 0 => f.magic_shop(),
            _ => f.item_shop(),
        };
        let stock = lists.get(w.server().clamp(0, 4) as usize)?;
        stock.iter().take_while(|&&c| c > 0).position(|&c| c == (i32::from(cat) << 16 | i32::from(id)))
    }

    /// How many of `buy` Kite still wants (none past 99 carried), as many
    /// as his gold pays for.
    fn short_of(w: &crate::world::WorldMode, buy: &Buy) -> i32 {
        let state = w.world().state();
        let (cat, id) = buy.item;
        let have: i32 = (0..piney_fieldui::items::ITEMS)
            .map(|k| piney_fieldui::items::save_item(state, 0, k))
            .filter(|it| (it.cat, it.id) == (cat, id))
            .map(|it| i32::from(it.num))
            .sum();
        let price = w.ui().texts().items.item(i32::from(cat), i32::from(id)).map_or(0, |p| p.price).max(1);
        let gold = state.save.i32(piney_fieldui::menus::shop::GOLD);
        (buy.carry.min(99) - have).min(gold / price).max(0)
    }

    /// The merchant of this town who sells the first of `buys` Kite is
    /// short of: its code.
    fn shop_for(w: &crate::world::WorldMode, buys: &[Buy]) -> Option<i32> {
        let world = w.world();
        buys.iter().filter(|b| short_of(w, b) > 0).find_map(|b| {
            let code = |m: &piney_world::merchant::Merchant| piney_world::entry::Npc::code(m);
            world.merchants().iter().map(code).find(|&c| stock_row(w, c, b.item).is_some())
        })
    }

    /// What the story autopilot goes for in a town, in order: an event's
    /// NPC to talk to, a mail or post unread (log out), another town, a
    /// member to call, an area's words, the marks. None with none.
    pub(super) fn gate_goal(
        w: &crate::world::WorldMode,
        talked: &[(piney_world::entry::Kind, i32)],
        buys: &[Buy],
    ) -> Option<GateGoal> {
        use piney_world::entry::Kind;
        let save = &w.world().state().save;
        // A mail unread, or a post new on the board (posted, or written
        // for the player: 1, 7): the top page reads them. Not the posts
        // while the events hold the board (operate 7, event 108).
        let board = w.vm().is_none_or(|vm| vm.operate() & (1 << 7) == 0);
        let mail = (0..piney_data::save::MAIL_SLOTS).any(|n| matches!(save.mail(n), 1 | 2));
        let posts = board
            && (0..piney_data::save::BBS_THREADS)
                .any(|t| (0..piney_data::save::BBS_MESSAGES).any(|m| matches!(save.bbs(t, m), 1 | 7)));
        // An NPC of this town the events wait to be spoken to (`add_target`)
        // comes before the gate.
        let world = w.world();
        let talk = world.event_targets().iter().find_map(|&(ty, code)| {
            // 3 and 4 a PC, 8-12 a merchant (event 16's five), both town NPCs.
            let kind = match ty {
                2 => Kind::Spc,
                3 | 4 | 8..=12 => Kind::Npc,
                13 => Kind::Gimmick,
                _ => return None,
            };
            let who = (kind, i32::from(code));
            (!talked.contains(&who) && world.char_place(kind, who.1).is_some()).then_some(who)
        });
        // What the story waits for, in order: a talk here, another town,
        // a member it wants in the party (one with an address who answers
        // calls, while there is room), an area's words.
        let wants = w.vm().map_or_else(Vec::new, |vm| story_wants(vm, save));
        let here = w.town_number();
        let party = world.party();
        let flags = save.i32(piney_data::save::offset::PARTY_MEMBER_FLAG) as u32;
        let calls = save.i32(piney_data::save::offset::PARTY_MEMBER_CALL) as u32;
        let alone = wants.contains(&Want::Alone);
        // The one member the story takes along, the others sent away.
        let only = wants.iter().find_map(|&x| match x {
            Want::Only(pc) => Some(pc),
            _ => None,
        });
        let others = only.is_some_and(|pc| party.iter().skip(1).any(|&m| m != -1 && m != pc));
        let closing = crate::start::story(world.volume()).iter().any(|&n| {
            let x = save.event_flag(n as usize);
            x & 1 << 63 != 0 && x & 1 << 62 == 0
        });
        // A marker of this town the story waits at, unless Kite is there.
        let kite = world.player().body.pos.map(f32::from_bits);
        let marker = wants.iter().find_map(|&x| match x {
            Want::Marker(m) => world
                .marker(m)
                .filter(|mk| (mk.pos[0] - kite[0]).hypot(mk.pos[1] - kite[1]) > 200.0)
                .map(|_| GateGoal::Marker(m)),
            _ => None,
        });
        // What Kite is to carry out, bought here first.
        let wanted = shop_for(w, buys)
            .map(GateGoal::Shop)
            .or(marker)
            .or_else(|| {
                // Not while this town is wanted too (event 22's Mac Anu and
                // Dun Loireag): the two would send Kite back and forth.
                let here_wanted = wants.contains(&Want::Town(here));
                wants.iter().find_map(|&x| match x {
                    Want::Town(t) if t != here && !here_wanted => town_row(w, t).map(GateGoal::Town),
                    _ => None,
                })
            })
            .or_else(|| (alone && party.iter().skip(1).any(|&m| m != -1)).then_some(GateGoal::Disband))
            .or_else(|| others.then_some(GateGoal::Disband))
            .or_else(|| {
                wants.iter().find_map(|&x| match x {
                    Want::Party(pc)
                        if !alone
                            && !party.contains(&pc)
                            && party.contains(&-1)
                            && flags & (1 << pc) != 0
                            && calls & (1 << pc) != 0 =>
                    {
                        Some(GateGoal::Invite(pc))
                    }
                    _ => None,
                })
            })
            .or_else(|| {
                // Such a member, the party full (event 210's Piros, after
                // 209's Balmung and Wiseman): the others sent away first.
                let full = !party.contains(&-1);
                let callable = |pc: i32| flags & calls & (1 << pc) != 0;
                wants
                    .iter()
                    .any(|&x| matches!(x, Want::Party(pc) if !alone && full && !party.contains(&pc) && callable(pc)))
                    .then_some(GateGoal::Disband)
            })
            .or_else(|| {
                // Going out: the free slots filled first, as a player
                // would, with the members who answer calls, those who
                // revive or heal first.
                let out = wants.iter().any(|&x| matches!(x, Want::Area(a) if area_way(w, a).is_some()));
                (out && !alone && only.is_none() && party.contains(&-1))
                    .then(|| companion(w, &party, flags & calls))
                    .flatten()
                    .map(GateGoal::Invite)
            })
            .or_else(|| {
                wants.iter().find_map(|&x| match x {
                    Want::Area(a) => area_way(w, a),
                    _ => None,
                })
            });
        // A mail comes before the story's wants (events wait on it); a
        // post only when the story wants nothing here (event 108 holds the
        // board on the top page, not in town).
        Some(match (talk, wanted, marked_areas(w).last(), marked_town(w)) {
            (Some((kind, code)), ..) => GateGoal::Talk(kind, code),
            _ if mail => GateGoal::LogOut,
            (None, Some(g), ..) => g,
            _ if posts => GateGoal::LogOut,
            // The story goes on on the desktop or the top page (event 107's
            // Quit after its town).
            _ if wants.iter().any(|w| matches!(w, Want::Leave { .. })) => GateGoal::LogOut,
            // A story event closed (its `end_event`) turns done, and the
            // next one opens on it, only at the next mode's
            // `ccStartThEvent` (event 12 after 11, 14 after 13): out first.
            (None, None, ..) if closing => GateGoal::LogOut,
            (None, None, Some(&(row, _)), _) => GateGoal::Area(row),
            (None, None, None, Some(row)) => GateGoal::Town(row),
            (None, None, None, None) => return None,
        })
    }

    /// The story autopilot's way from a town: Kite walks to the Chaos Gate
    /// (gimmick 16) and speaks to it; the gate's menu (28), then Word List
    /// (59), the marked area's row and Warp, or else, with a mail unread,
    /// Log Out (11) and OK. None with neither, or while a window waits
    /// (the rest of [`story_player`] then).
    fn gate_player(
        w: &crate::world::WorldMode,
        f: u64,
        talked: &[(piney_world::entry::Kind, i32)],
        buys: &[Buy],
    ) -> Option<Raw> {
        use piney_world::entry::Kind;
        let still =
            |buttons: Buttons| Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let goal = gate_goal(w, talked, buys)?;
        let world = w.world();
        let ui = w.ui();
        let c = &ui.ctrl;
        let every = |n: u64, b: Buttons| still(if f.is_multiple_of(n) { b } else { Buttons::NONE });
        let go_to = |to: usize| match c.list().select {
            s if s as usize == to => Buttons::CROSS,
            s if (s as usize) < to => Buttons::DOWN,
            _ => Buttons::UP,
        };
        let go_to_item = |item: i16| {
            let l = c.list();
            match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == item) {
                Some(r) => go_to(r),
                None => Buttons::NONE,
            }
        };
        match (ui.menu_type(), c.proccess, goal) {
            (28, 1, GateGoal::Area(_)) => return Some(every(8, go_to_item(59))),
            (28, 1, GateGoal::Town(_)) => return Some(every(8, go_to_item(61))),
            (61, 2, GateGoal::Town(row)) => return Some(every(8, go_to(row))),
            (61, 5, _) => return Some(every(8, go_to(0))),
            (0, 1, GateGoal::LogOut) => return Some(every(8, go_to_item(11))),
            (59, 2, GateGoal::Area(row)) => return Some(every(8, go_to(row))),
            (59, 4, _) => return Some(every(8, Buttons::CROSS)),
            (11, 1, _) => return Some(every(8, go_to(0))),
            // A walking PC's list opened on a passer-by, not on the one
            // the pilot went to (he stepped in as the command target as
            // OK went): back out.
            (22, ..) if !matches!(goal, GateGoal::Talk(k, n) if world.command_target() == Some((k, n))) => {
                return Some(every(8, Buttons::CIRCLE));
            }
            // A talk the events do not take (the NPC's own line): on.
            (22, _, GateGoal::Talk(..)) => return Some(every(8, Buttons::CROSS)),
            // The PC's line (`TalkMenu`): OK closes it, back to his list.
            (47, ..) => return Some(every(8, Buttons::CROSS)),
            // The shop: Buy (52), the item's row, the count up to what
            // Kite is short of, OK, and OK to the question; a refusal's
            // lines closed. Once nothing is short, back out.
            (24, 1, GateGoal::Shop(_)) => return Some(every(8, go_to_item(52))),
            (52, 1, GateGoal::Shop(code)) => {
                let row = buys.iter().filter(|b| short_of(w, b) > 0).find_map(|b| stock_row(w, code, b.item));
                return Some(every(8, row.map_or(Buttons::CIRCLE, go_to)));
            }
            (52, 2, GateGoal::Shop(code)) => {
                let want = buys
                    .iter()
                    .find(|b| short_of(w, b) > 0 && stock_row(w, code, b.item) == Some(c.list().select.max(0) as usize))
                    .map_or(0, |b| short_of(w, b));
                let b = match i32::from(c.wait_count) {
                    n if n < want => Buttons::UP,
                    n if n > want => Buttons::DOWN,
                    _ => Buttons::CROSS,
                };
                return Some(every(8, b));
            }
            (52, 4, GateGoal::Shop(_)) => return Some(every(8, go_to(0))),
            (52, 10..=12, _) => return Some(every(8, Buttons::CROSS)),
            (52, _, GateGoal::Shop(_)) => return Some(still(Buttons::NONE)),
            // A shop's or a breeder's own menu, opened by a talk: back out.
            (23..=27 | 44..=46 | 52, ..) => return Some(every(8, Buttons::CIRCLE)),
            // The call: PERSONAL, Party, Add, the member's face, OK, and
            // the greeting closed.
            (0, 1, GateGoal::Invite(_)) => return Some(every(8, go_to_item(9))),
            (9, 1, GateGoal::Invite(_)) => return Some(every(8, go_to_item(68))),
            (68, 2, GateGoal::Invite(pc)) if i32::from(c.face_num) == pc => return Some(every(8, Buttons::CROSS)),
            (68, 2, GateGoal::Invite(_)) => return Some(every(8, Buttons::DOWN)),
            (68, 5, _) => return Some(every(8, Buttons::CROSS)),
            (68, p, _) if p >= 20 => return Some(every(24, Buttons::CROSS)),
            // The disband: PERSONAL, Party, Disband, Yes, and each
            // member's goodbye closed.
            (0, 1, GateGoal::Disband) => return Some(every(8, go_to_item(9))),
            (9, 1, GateGoal::Disband) => return Some(every(8, go_to_item(70))),
            (70, 1, _) => return Some(every(8, go_to(0))),
            (70, p, _) if p >= 10 => return Some(every(24, Buttons::CROSS)),
            // A protected area's hack: each slot's cores up to the count
            // its protect row asks, the next slot, and OK.
            // An event's lines over the hack (event 18's lesson on the
            // cores after `virus_core`): OK through them.
            (62, ..) if w.vm().is_some_and(|v| v.playing().is_some()) => return Some(every(24, Buttons::CROSS)),
            (62, ..) => {
                let Some(h) = c.hack.as_deref() else { return Some(still(Buttons::NONE)) };
                let need = |k: usize| h.protect[2 * k + 1];
                let b = if (0..4).all(|k| h.set_num[k] == need(k)) {
                    Buttons::CROSS
                } else if h.set_num[h.select as usize & 3] < need(h.select as usize & 3) {
                    Buttons::UP
                } else {
                    Buttons::LEFT
                };
                return Some(every(8, b));
            }
            (-1, ..) => {}
            _ => return None,
        }
        let waiting = w.calls().iter().rev().find_map(|(_, c)| {
            if c.starts_with("message_open") || c.starts_with("announce") || c.starts_with("info_lines") {
                Some(true)
            } else if c.starts_with("message_check") || c.starts_with("message_close") {
                Some(false)
            } else {
                None
            }
        });
        if waiting == Some(true) {
            return None;
        }
        if let GateGoal::LogOut | GateGoal::Invite(_) | GateGoal::Disband = goal {
            return Some(every(30, Buttons::TRIANGLE));
        }
        let (kind, code) = match goal {
            GateGoal::Talk(kind, code) => (kind, code),
            GateGoal::Shop(code) => (Kind::Npc, code),
            _ => (Kind::Gimmick, 16),
        };
        let g = match goal {
            GateGoal::Marker(m) => world.marker(m)?.pos,
            _ => {
                if world.command_target() == Some((kind, code)) {
                    return Some(every(24, Buttons::CROSS));
                }
                world.char_place(kind, code)?.0.map(f32::from_bits)
            }
        };
        let p = world.player().body.pos.map(f32::from_bits);
        // Beside the NPC with the Chaos Gate the target (the SEARCH events'
        // NPC stands at its dummy, `DMY_gate`, and the gate comes first):
        // the stick let go and pushed again, which steps the target on
        // (`ccSelectTarget` mode 2, the stick's power rising past 64).
        let beside = (g[0] - p[0]).hypot(g[1] - p[1]) < 400.0;
        let gate = world.command_target() == Some((Kind::Gimmick, 16));
        if matches!(goal, GateGoal::Talk(..)) && kind != Kind::Gimmick && beside && gate && f % 16 < 8 {
            return Some(still(Buttons::NONE));
        }
        let cam_z = f32::from_bits(world.camera().rot()[2]);
        Some(stick_toward(cam_z, (g[0] - p[0]).atan2(-(g[1] - p[1]))))
    }

    /// A new game's Log in played through event 2 into story area 14's
    /// field, then `frames` more frames there: the session and the frames
    /// it took to reach the field. None without the disc.
    fn story_to_field(frames: u64) -> Option<(Session, u64)> {
        story_to_field_with(frames, |_| {})
    }

    /// [`story_to_field`], `each` seeing the session after every frame in
    /// the field.
    fn story_to_field_with(frames: u64, mut each: impl FnMut(&Session)) -> Option<(Session, u64)> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return None;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let vm = crate::world::new_game_events(&mut disc, &mut state).unwrap();
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let mut s = Session::in_world(iso, archive, None, state, Some(vm), scene, None).unwrap();
        let mut pad = Pad::default();
        let mut f = 0u64;
        while !matches!(s.stage, Stage::Area(_)) {
            f += 1;
            let raw = story_player(&s, f);
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            assert!(f < 20_000, "no field: {}", Mode::title(&s));
        }
        let arrived = f;
        for _ in 0..frames {
            f += 1;
            let raw = story_player(&s, f);
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            each(&s);
        }
        Some((s, arrived))
    }

    /// Prints what event 3 asked in the field: `cargo test --release -p
    /// piney-game event_3_log -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_3_log() {
        let Some((s, arrived)) = story_to_field(4000) else { return };
        let Stage::Area(a) = &s.stage else { panic!("left the field: {}", Mode::title(&s)) };
        println!("the field from frame {arrived}; {}", Mode::title(&s));
        for (fr, c) in a.calls() {
            println!("{fr:5} {c}");
        }
    }

    /// The session stepped by [`story_player`] until the area's call log
    /// has a call starting with `until`, then `more` frames; each frame's
    /// raw pad with the session's frame number.
    fn story_until(s: &mut Session, f: &mut u64, until: &str, more: u64, log: &mut Vec<(u64, Raw)>) {
        let mut pad = Pad::default();
        let seen = |s: &Session| match &s.stage {
            Stage::Area(a) => a.calls().iter().any(|(_, c)| c.starts_with(until)),
            _ => false,
        };
        let mut left = None;
        while left != Some(0) {
            *f += 1;
            let raw = story_player(s, *f);
            log.push((*f, raw));
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            left = match left {
                None if seen(s) => Some(more),
                Some(n) => Some(n - 1),
                l => l,
            };
            assert!(*f < 30_000, "no {until}: {}", Mode::title(s));
        }
    }

    /// Prints the battle's state through event 3: `cargo test --release -p
    /// piney-game event_3_battle_log -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_3_battle_log() {
        let mut n = 0u32;
        let r = story_to_field_with(4500, |s| {
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let c = w.combat();
            n += 1;
            if !n.is_multiple_of(50) || c.kite.is_none() {
                return;
            }
            let pos = |i: usize| c.scene.chars[i].pos.map(f32::from_bits);
            let k = c.kite.unwrap_or(0);
            let ens: Vec<String> = c
                .ctrl
                .list(piney_battle::entry::Kind::Enemy)
                .iter()
                .map(|&e| {
                    format!(
                        "{e}:{:?} hp{} act{}",
                        pos(e),
                        c.scene.chars[e].hp,
                        c.foes[e].as_ref().map_or(-1, |x| x.act_num)
                    )
                })
                .collect();
            let mcs: Vec<String> =
                c.ctrl.list(piney_battle::entry::Kind::Circle).iter().map(|&e| format!("{e}:{:?}", pos(e))).collect();
            println!(
                "{} kite {:?} act {} hp {}/{} lv{} exp{} skill {}/{} runs {} listed {} target {:?} enemies {:?} circles {:?} inBattle {}",
                Mode::title(s),
                pos(k),
                c.scene.chars[k].spc_char.act_num,
                c.scene.chars[k].hp,
                c.scene.chars[k].max_hp,
                c.scene.chars[k].level(),
                c.scene.chars[k].spc().map_or(0, |p| p.base.exp),
                c.scene.chars[k].skill_id,
                c.scene.chars[k].skill_status,
                c.skills.borrow().runs.len(),
                c.scene.listed(k),
                w.command_target(),
                ens,
                mcs,
                c.battle.in_battle
            );
        });
        let (s, _) = r.unwrap();
        let Stage::Area(a) = &s.stage else { return };
        for (fr, c) in a.calls().iter().filter(|(f, _)| *f > 2000) {
            println!("{fr:5} {c}");
        }
    }

    /// Event 3's map lesson (TEACH-F lines 20-25, docs/engine/map.md):
    /// `map_on` gives the map back after the camera lesson's `menu_ban`, and
    /// by Orca's "You see the Red Down Arrow on it?" it is drawn at full
    /// alpha with the dungeon entrance and its red arrow (FP_DUNGEON, then
    /// the arrow's cell at u 81, v 65); "OK, wait a sec." ends in
    /// `show_map`, which puts the magic portals on the map
    /// (`WORLD::ShowMap`: mapFlag).
    #[test]
    fn event_3_shows_the_map() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "message_open 3 21 ", 30, &mut log);
        let Stage::Area(a) = &s.stage else { panic!("left the field: {}", Mode::title(&s)) };
        let st = a.map_state();
        assert_eq!(st.alpha, 1f32.to_bits(), "the map faded in");
        let cells: Vec<(i32, i32)> = st
            .last
            .iter()
            .filter_map(|o| match o {
                piney_world::map::sprite::Out::Send(s) => Some(s),
                _ => None,
            })
            .flat_map(|s| s.packets.iter().map(|p| (p.wu, p.wv)))
            .collect();
        assert!(cells.contains(&(96 << 4, 12 << 4)), "the entrance on the map: {cells:?}");
        assert!(cells.contains(&(1296, 1040)), "its red arrow");
        let flag = |s: &Session| match &s.stage {
            Stage::Area(a) => match a.world().place() {
                piney_world::field_world::Place::Field(fa) => fa.map.as_ref().is_some_and(|m| m.map_flag),
                _ => false,
            },
            _ => false,
        };
        assert!(!flag(&s), "no portals before the Fairy's Orb");
        story_until(&mut s, &mut f, "show_map", 1, &mut log);
        assert!(flag(&s), "show_map set WORLD's mapFlag");
    }

    /// The map is away while a battle is on (`WORLD::Draw` skips
    /// `DrawMiniMap` under `ccGame.inBattle`): walking to event 3's east
    /// portal and fighting its goblins, no fight frame draws the map, and
    /// the walk before it does.
    #[test]
    fn the_map_is_away_in_a_fight() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let (mut fight, mut walk) = ((0, 0), (0, 0));
        walk_to_east_portal(&mut s, 1500, true, |s| {
            let Stage::Area(a) = &s.stage else { return };
            let drawn = usize::from(!a.map_state().last.is_empty());
            let n = if a.world().combat().battle.in_battle != 0 { &mut fight } else { &mut walk };
            *n = (n.0 + 1, n.1 + drawn);
        });
        assert!(fight.0 > 0, "no fight");
        assert_eq!(fight.1, 0, "the map in {} of {} fight frames", fight.1, fight.0);
        assert!(walk.1 > 0, "the map never drawn on the walk ({} frames)", walk.0);
    }

    /// The presses [`event_3_shows_the_map`]'s player makes up to Orca's
    /// "You see the Red Down Arrow on it?", as `--press` wants them (its
    /// frames count from 0) with the frame count for `--frames`:
    /// `cargo test --release -p piney-game event_3_map_presses -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore]
    fn event_3_map_presses() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let vm = crate::world::new_game_events(&mut disc, &mut state).unwrap();
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let mut s = Session::in_world(iso, archive, None, state, Some(vm), scene, None).unwrap();
        let mut f = 0;
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "message_open 3 21 ", 40, &mut log);
        let mut out = Vec::new();
        for (fr, raw) in &log {
            for (b, name) in [
                (Buttons::CROSS, "cross"),
                (Buttons::TRIANGLE, "triangle"),
                (Buttons::UP, "up"),
                (Buttons::DOWN, "down"),
                (Buttons::L1, "l1"),
                (Buttons::R2, "r2"),
            ] {
                if raw.buttons.contains(b) {
                    out.push(format!("{}:{name}", fr - 1));
                }
            }
            if raw.ry == 0 {
                out.push(format!("{}:rup", fr - 1));
            }
        }
        println!("--frames {f} --press {}", out.join(","));
    }

    /// The left stick pushed so that Kite, whose camera looks along
    /// `cam_z`, walks toward heading `h` (radians, 0 facing -y): the raw
    /// of 64 directions whose pad direction comes nearest what
    /// `ControlMove` turns into `h` (`h = cam_z - (pi + dircL)`).
    fn stick_toward(cam_z: f32, h: f32) -> Raw {
        use std::f32::consts::PI;
        let want = (cam_z - PI - h).rem_euclid(2.0 * PI);
        let mut best = (f32::MAX, Raw::default());
        for k in 0..64 {
            let a = k as f32 * PI / 32.0;
            let (lx, ly) = ((128.0 + 127.0 * a.cos()) as u8, (128.0 + 127.0 * a.sin()) as u8);
            let raw = Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() };
            let mut p = Pad::default();
            p.read(&raw);
            let d = (p.dirc_l.rem_euclid(2.0 * PI) - want).abs();
            let d = d.min(2.0 * PI - d);
            if d < best.0 {
                best = (d, raw);
            }
        }
        best.1
    }

    /// Through event 3 (its goblin, the skill and chat lessons), then on
    /// to another of its magic portals: [`walk_to_east_portal`]. None
    /// without the disc.
    fn event_3_then_portal(frames: u64, each: impl FnMut(&Session)) -> Option<Session> {
        let (mut s, mut f) = story_to_field(0)?;
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        walk_to_east_portal(&mut s, frames, std::env::var("PINEY_ATTACK").is_ok(), each);
        Some(s)
    }

    /// Kite walked toward event 3's magic portal east of the start
    /// (28600, 24600) until its goblins are out or he is 2500 from it,
    /// then left standing (with `attack`, the attack button pushed every
    /// eighth frame while the goblins are out); `frames` frames, `each`
    /// seeing the session after every one.
    fn walk_to_east_portal(s: &mut Session, frames: u64, attack: bool, mut each: impl FnMut(&Session)) {
        walk_to_east_portal_frames(s, frames, attack, |s, _| each(s));
    }

    /// [`walk_to_east_portal`], `each` seeing each frame drawn too.
    fn walk_to_east_portal_frames(s: &mut Session, frames: u64, attack: bool, mut each: impl FnMut(&Session, &Frame)) {
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        for i in 0..frames {
            let raw = {
                let Stage::Area(a) = &s.stage else { break };
                let w = a.world();
                let c = w.combat();
                let k = c.kite.unwrap();
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                let enemies = c.enemies();
                let foe = enemies.iter().copied().find(|&e| c.scene.chars[e].hp > 0);
                let cam_z = f32::from_bits(w.camera().rot()[2]);
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                match foe {
                    // Up to the goblin, then the attack button.
                    Some(e) if attack => {
                        let q = c.scene.chars[e].pos.map(f32::from_bits);
                        let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
                        if ex * ex + ey * ey > 180.0 * 180.0 {
                            stick_toward(cam_z, ex.atan2(-ey))
                        } else if i.is_multiple_of(8) {
                            Raw { buttons: Buttons::CROSS, ..still }
                        } else {
                            still
                        }
                    }
                    _ if !enemies.is_empty() || dx * dx + dy * dy < 2500.0 * 2500.0 => still,
                    _ => stick_toward(cam_z, dx.atan2(-dy)),
                }
            };
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            each(s, &frame);
        }
    }

    /// Kite walked up to `near` of event 3's east portal (28600, 24600)
    /// and left standing there, facing it; `frames` frames, `each` seeing
    /// the session and the frame drawn after every one.
    fn walk_near_east_portal(s: &mut Session, frames: u64, near: f32, mut each: impl FnMut(&Session, &Frame)) {
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        for i in 0..frames {
            let raw = {
                let Stage::Area(a) = &s.stage else { break };
                let w = a.world();
                let c = w.combat();
                let k = c.kite.unwrap();
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                let cam_z = f32::from_bits(w.camera().rot()[2]);
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                if dx * dx + dy * dy < near * near {
                    still
                } else if i.is_multiple_of(40) && i > 0 {
                    // The camera reset: behind him as he heads for it.
                    Raw { buttons: Buttons::R2, ..stick_toward(cam_z, dx.atan2(-dy)) }
                } else {
                    stick_toward(cam_z, dx.atan2(-dy))
                }
            };
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            each(s, &frame);
        }
    }

    /// A skill used in a fight names itself: after event 3, in the battle at
    /// the east portal, Kite uses Repth on a member through PERSONAL, Skills
    /// and TARGET; `ccWordsPlay(sid, Kite)` reaches the session as
    /// `Event::SkillWords` (base type 5, `charTbl` row 0, the skill's id and
    /// type bit, outside an event), and `skillVoicePlay` has Kite's line.
    #[test]
    fn a_skill_in_a_fight_names_itself() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        // The Voiceover option set to Japanese mid-game, as OPTION's Voice
        // page leaves it: the next skill word reads it.
        {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
            a.world_mut().state_mut().save.set_u8(offset::VOICE, 0);
        }
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        let mut words = Vec::new();
        let mut options_with_words = None;
        let mut fought = false;
        // The frame the target is confirmed, and the one the words come in.
        let (mut ok_at, mut words_at) = (None, None);
        for i in 0..3000u64 {
            let raw = {
                let Stage::Area(a) = &s.stage else { panic!("left the area") };
                let w = a.world();
                let c = w.combat();
                let ui = a.ui();
                let m = &ui.ctrl;
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                let press = |b: Buttons| if i.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
                let go_to = |row: usize| match m.list().select.max(0) as usize {
                    s if s == row => Buttons::CROSS,
                    s if s < row => Buttons::DOWN,
                    _ => Buttons::UP,
                };
                fought |= c.battle.in_battle != 0;
                if ui.menu_type() == 65 && i.is_multiple_of(8) {
                    ok_at = Some(i);
                }
                match ui.menu_type() {
                    -1 if fought && words.is_empty() => press(Buttons::TRIANGLE),
                    -1 => {
                        let k = c.kite.unwrap();
                        let p = c.scene.chars[k].pos.map(f32::from_bits);
                        let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        if dx * dx + dy * dy < 1500.0 * 1500.0 { still } else { stick_toward(cam_z, dx.atan2(-dy)) }
                    }
                    0..=2 => press(go_to(m.list().items.iter().position(|&x| x == 4).unwrap_or(0))),
                    4 if m.list().page < 2 => press(Buttons::RIGHT),
                    4 => press(go_to(0)),
                    65 => press(Buttons::CROSS),
                    _ => still,
                }
            };
            pad.read(&raw);
            s.step(&pad);
            let events = s.take_events();
            if words_at.is_none() && events.iter().any(|e| matches!(e, Event::SkillWords { .. })) {
                words_at = Some(i);
            }
            if options_with_words.is_none() && events.iter().any(|e| matches!(e, Event::SkillWords { .. })) {
                options_with_words =
                    Some(events.iter().any(|e| matches!(e, Event::VoiceOptions { english: false, .. })));
            }
            words.extend(events.into_iter().filter(|e| matches!(e, Event::SkillWords { .. })));
            if !words.is_empty() && i > 0 && fought {
                break;
            }
        }
        assert!(fought, "the battle mode came on");
        // The menu task (34) asks before the party's (48) and the skills'
        // (82) tasks run: the words come out in the confirming frame.
        assert_eq!(words_at, ok_at, "the words in the frame the target was confirmed");
        // skillVoicePlay reads saveData.voice as it plays: the options go
        // with the words.
        assert_eq!(options_with_words, Some(true), "the Japanese option with the skill's words");
        let Some(&Event::SkillWords { event_running, char_type, char_id, sid, type_bit }) = words.first() else {
            panic!("no SkillWords: {}", Mode::title(&s));
        };
        let Stage::Area(a) = &s.stage else { panic!() };
        let t = &a.world().combat().data.t;
        assert!(!event_running, "outside an event");
        assert!(char_type & 5 != 0 && char_id == 0, "Kite: type {char_type:#x}, row {char_id}");
        assert!(sid >= 6, "a skill, not a normal attack: {sid}");
        assert_eq!(type_bit, t.skill(sid).unwrap().ty & 1 != 0, "the skill's type bit");
        // The sound task plays it: ccWordsPlay's queue, then skillVoicePlay
        // sends Kite's line for the skill.
        let mut d = piney_audio::driver::Driver::new();
        assert_eq!(d.words_play(event_running, char_type, char_id, sid, type_bit), 0);
        let mut out = Vec::new();
        d.skill_voice_play(&mut out);
        assert!(
            out.iter().any(|c| matches!(c, piney_audio::driver::Command::Voice(_))),
            "a voice line for skill {sid}: {out:?}"
        );
    }

    /// Data Drain in a fight: after event 3, Kite (given Data Drain) walks
    /// to the east portal; once the battle is on the goblin's protect is
    /// broken, and PERSONAL, Skills, the Data Drain page, the goblin: menu
    /// 66 runs its frames, the area's rules answer (the infection rises, the
    /// goblin leaves the command lists, its drop), the windows are closed,
    /// and the drop is handed out through 67 and 29.
    fn drain_in_a_fight(demo: bool, each: impl FnMut(&mut Session, u64, &Frame)) -> Option<Session> {
        drain_with(demo, |_| Some(4), each)
    }

    /// [`drain_in_a_fight`] with PERSONAL's row as `plan` answers it when
    /// the goblin is near: `None` waits, 4 is Skills (the drain), 5 Items
    /// (the scrolls' page, category 11: its first on the target offered).
    fn drain_with(
        demo: bool,
        plan: impl Fn(&Session) -> Option<i16>,
        mut each: impl FnMut(&mut Session, u64, &Frame),
    ) -> Option<Session> {
        let (mut s, mut f) = story_to_field(0)?;
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
            // The fight as it goes from the game's rand() at 1, whatever
            // the set-up drew before.
            a.world_mut().set_rand(1);
            let save = &mut a.world_mut().state_mut().save;
            let free = (0..20).find(|&k| save.i16(piney_fieldui::items::SKILL_LIST + 2 * k) < 0).unwrap();
            save.set_i16(piney_fieldui::items::SKILL_LIST + 2 * free, 2);
            // The bracelet (the Data Drain page), and its movie.
            save.set_u8(offset::PLCOL, 1);
            save.set_u8(piney_fieldui::menus::drain::DRAIN_DEMO, u8::from(demo));
        }
        let erosion0 = {
            let Stage::Area(a) = &s.stage else { panic!() };
            a.world().state().save.i16(piney_fieldui::menus::drain::EROSION)
        };
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        let (mut fought, mut drained, mut handed) = (false, false, false);
        for i in 0..4000u64 {
            let row = plan(&s);
            let raw = {
                let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
                let in_battle = a.world().combat().battle.in_battle != 0;
                if in_battle {
                    // The goblin's protect broken, as enough hits would.
                    let c = a.world_mut().combat_mut();
                    for &e in &c.enemies() {
                        if let Some(fs) = c.scene.chars[e].foe_state_mut() {
                            fs.pp_count = 5;
                        }
                    }
                }
                fought |= in_battle;
                let w = a.world();
                let c = w.combat();
                let ui = a.ui();
                let m = &ui.ctrl;
                drained |= m.menu == 66;
                handed |= drained && m.menu == 29;
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                let press = |b: Buttons| if i.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
                let go_to = |row: usize| match m.list().select.max(0) as usize {
                    s if s == row => Buttons::CROSS,
                    s if s < row => Buttons::DOWN,
                    _ => Buttons::UP,
                };
                let k = c.kite.unwrap();
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                // In the fight, the nearest enemy is where to go.
                let near = c.enemies().into_iter().map(|e| c.scene.chars[e].pos.map(f32::from_bits)).min_by(|a, b| {
                    let d = |q: [f32; 4]| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
                    d(*a).total_cmp(&d(*b))
                });
                let (to, reach) = match near {
                    Some(q) if fought => ([q[0], q[1]], 120.0f32),
                    _ => (goal, 1500.0),
                };
                let (dx, dy) = (to[0] - p[0], to[1] - p[1]);
                let close = dx * dx + dy * dy < reach * reach;
                match ui.menu_type() {
                    -1 if handed => break,
                    -1 if fought && !drained && close && row.is_some() => press(Buttons::TRIANGLE),
                    -1 => {
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        if close { still } else { stick_toward(cam_z, dx.atan2(-dy)) }
                    }
                    0..=2 => press(go_to(m.list().items.iter().position(|&x| Some(x) == row).unwrap_or(0))),
                    4 if m.list().page < 5 => press(Buttons::RIGHT),
                    // Items: the scrolls' page (category 11), its first.
                    5 if piney_fieldui::items::item_list(&ui.texts().items, w.state(), 0, i32::from(m.list().page))
                        [0]
                    .cat != 11 =>
                    {
                        press(Buttons::RIGHT)
                    }
                    4 => press(go_to(0)),
                    // The target, the windows, the item's.
                    _ => press(Buttons::CROSS),
                }
            };
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            each(&mut s, i, &frame);
        }
        assert!(fought, "the battle mode came on");
        assert!(drained, "menu 66 never ran: {}", Mode::title(&s));
        let Stage::Area(a) = &s.stage else { panic!() };
        let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
        assert!(calls.iter().any(|c| c.starts_with("data_drain 2")), "the rules did not run: {calls:?}");
        let erosion = a.world().state().save.i16(piney_fieldui::menus::drain::EROSION);
        assert_ne!(erosion, erosion0, "the infection did not move");
        assert!(handed, "the drop was not handed out: {}", Mode::title(&s));
        Some(s)
    }

    /// A hit on Kite rumbles the pad: in event 3's field, `EntryAffect(1,
    /// 20)` on him from the goblin nearest (as its attack lands) makes his
    /// `Influence` call `DamageActuate`, and the mode asks for
    /// `SetActuater(pad 0, 1, DamActuTbl[1], 100)`: 128 for 10 to 99.
    #[test]
    fn a_hit_on_kite_rumbles() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        s.take_events();
        {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
            let c = a.world().combat();
            let k = c.kite.unwrap();
            let by = c.enemies().first().copied();
            a.world_mut().entry_affect(k, by, 1, [20, 0, 0]);
        }
        let mut pad = Pad::default();
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        let rumble: Vec<Event> = s.take_events().into_iter().filter(|e| matches!(e, Event::Actuate { .. })).collect();
        assert!(matches!(rumble.as_slice(), [Event::Actuate { small: true, power: 128, ms: 100 }]), "{rumble:?}");
    }

    /// The console's `town N`: from event 3's field to Dun Loireag through
    /// the fade and the town's set-up; a number past the five is refused.
    #[test]
    fn the_console_goes_to_a_town() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        assert_eq!(
            s.console("town 7"),
            "town N (0 Mac Anu, 1 Dun Loireag, 2 Carmina Gadelica, 3 Fort Ouph, 4 Lia Fail)"
        );
        assert_eq!(s.console("town 1"), "to DunLoireag");
        let mut pad = Pad::default();
        for _ in 0..600 {
            if matches!(&s.stage, Stage::World(_)) {
                break;
            }
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        let Stage::World(w) = &s.stage else { panic!("not in a town: {}", Mode::title(&s)) };
        assert_eq!(w.world().town().base.no, 1, "{}", Mode::title(&s));
    }

    /// The console's `town N` as the sound hears it: from event 3's field to
    /// Dun Loireag, then on to Mac Anu, the session's own sound events
    /// driving a headless audio engine as `main` routes them; each town's
    /// music is heard (`--nocapture` prints the levels).
    #[test]
    fn town_jumps_keep_the_music() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let audio = piney_audio::Audio::headless(&iso).unwrap();
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let feed = |a: &piney_audio::Audio, events: Vec<Event>| {
            for e in events {
                match e {
                    Event::SqLoad(ctx) => a.sq_load(ctx),
                    Event::BgmCtrl(w) => {
                        a.bgm_ctrl(w);
                    }
                    Event::GameStart => a.game_start(),
                    Event::GameInterrupt => a.game_interrupt(),
                    Event::AllSoundOff => a.all_sound_off(),
                    Event::SceneSound(sc) => a.scene_sound(&sc),
                    Event::GameArea(n) => a.set_game_area(n),
                    Event::InBattle(on) => a.set_in_battle(on),
                    Event::SqPlay(n) => a.sq_play(n),
                    Event::SqStop(n) => a.sq_stop(n),
                    Event::SqFade { seq, volume, time, mode } => a.sq_fade(seq, volume, time, mode),
                    Event::PortVolume { port, volume } => a.port_volume(port, volume),
                    Event::MainVolume(v) => a.set_main_volume(v),
                    Event::Volumes { main, se, bgm, output } => {
                        a.set_volumes(main, se, bgm);
                        a.set_output_mode(output);
                    }
                    Event::HoldBgm => a.hold_bgm(),
                    _ => {}
                }
            }
        };
        let mut pad = Pad::default();
        let mut chunk = vec![0i16; 1600];
        for to in [1, 0, 1] {
            eprintln!("-- {}", s.console(&format!("town {to}")));
            let mut sq = 0f64;
            let mut n = 0usize;
            for k in 0..600u32 {
                pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
                s.step(&pad);
                feed(&audio, s.take_events());
                audio.frame();
                audio.render(&mut chunk);
                if k >= 300 {
                    sq += chunk.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>();
                    n += chunk.len();
                }
            }
            let rms = (sq / n as f64).sqrt();
            let playing = audio.with_engine(|e| e.seq.iter().map(|q| q.playing()).collect::<Vec<_>>());
            let ports = audio.with_engine(|e| e.driver.port_vol);
            eprintln!("   {}: rms {rms:.0}, playing {playing:?}, ports {ports:?}", Mode::title(&s));
            assert!(rms > 100.0, "silent in {}", Mode::title(&s));
        }
    }

    /// Issue #32. Each arrival in a town draws its own walking PCs: the
    /// set-up's `ccInitRand` draws the generator `ccSys+0x358` times, the
    /// frames since power-on. Mac Anu twice (the console's `town`, a visit
    /// to Dun Loireag between) has two different crowds.
    #[test]
    fn each_arrival_in_town_has_its_own_pcs() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let mut crowd = |s: &mut Session, to: i32| {
            s.console(&format!("town {to}"));
            for _ in 0..200 {
                pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
                s.step(&pad);
                s.take_events();
            }
            let Stage::World(w) = &s.stage else { panic!("not in a town: {}", Mode::title(s)) };
            w.world().pcs().iter().map(|p| p.row.row).collect::<Vec<_>>()
        };
        let first = crowd(&mut s, 0);
        crowd(&mut s, 1);
        let again = crowd(&mut s, 0);
        assert!(!first.is_empty() && first.len() == again.len(), "{first:?} {again:?}");
        assert_ne!(first, again, "the same PCs on both arrivals");
    }

    /// The console's `invite_party`: in event 3's field BlackRose (15) is
    /// given the party's free slot and comes with the next area (the
    /// console's `town 0`), where she is built; there Mia (1) joins at
    /// once, at the Chaos Gate; each takes its slot's menu face; a member
    /// already in the party, or a party full, is refused.
    #[test]
    fn the_console_invites() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        assert_eq!(s.console("invite_party 15"), "15 in slot 2: with the next area");
        let Stage::Area(a) = &s.stage else { panic!("left the field") };
        assert_eq!(a.ui().ctrl.face_tex[2], 15, "BlackRose's menu face");
        assert_eq!(s.console("invite_party 2"), "2 is in the party already ([0, 2, 15])");
        s.console("town 0");
        let mut pad = Pad::default();
        for _ in 0..900 {
            if matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30))
            {
                break;
            }
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        let Stage::World(w) = &s.stage else { panic!("not in Mac Anu: {}", Mode::title(&s)) };
        assert_eq!(w.world().party(), [0, 2, 15]);
        assert!(w.world().town_party().rec(15).is_some(), "BlackRose built in the town");
        assert_eq!(
            s.console("invite_party 1"),
            "no room for 1: the party is [0, 2, 15] (PERSONAL > Party to remove one)"
        );
        let Stage::World(w) = &mut s.stage else { unreachable!() };
        w.world_mut().party_remove(2);
        assert_eq!(s.console("invite_party 1"), "1 in slot 1");
        let Stage::World(w) = &s.stage else { unreachable!() };
        assert_eq!(w.world().party(), [0, 1, 15]);
        assert_eq!(w.ui().ctrl.face_tex[1], 1, "Mia's menu face, not Orca's");
        assert!(w.world().town_party().rec(1).is_some(), "Mia built at the gate");
    }

    /// The console's god-mode commands in event 3's field: `protect` breaks
    /// each enemy's protect as a hit does (the marks come up), `kill` fells
    /// them all, `god` keeps Kite's HP
    /// full and `infection` sets the bracelet's infection, clamped to 100.
    #[test]
    fn the_console_cheats() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        // To the goblins' ground, as drain_in_a_fight walks, till the fight.
        let mut pad = Pad::default();
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let mut foes = 0;
        for _ in 0..2000 {
            let raw = {
                let Stage::Area(a) = &s.stage else { panic!("left the area") };
                let w = a.world();
                let c = w.combat();
                foes = c.enemies().into_iter().filter(|&e| !c.dead(e)).count();
                if c.battle.in_battle != 0 && foes > 0 {
                    break;
                }
                let p = c.scene.chars[c.kite.unwrap()].pos.map(f32::from_bits);
                let (dx, dy) = (28600.0 - p[0], 24600.0 - p[1]);
                if dx * dx + dy * dy < 1500.0 * 1500.0 {
                    still
                } else {
                    stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
                }
            };
            pad.read(&raw);
            s.step(&pad);
        }
        assert!(foes > 0, "no fight");
        assert_eq!(s.console("protect"), format!("{foes} protects broken for 60 s"));
        // The break as a hit raises it: the protect marks over the foes
        // (`SetProtect`) on the next frame.
        pad.read(&still);
        s.step(&pad);
        {
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let marks = a.ui().ctrl.protect_cnt.iter().filter(|&&n| n > 0).count();
            assert_eq!(marks, foes, "the protect marks");
        }
        assert_eq!(s.console("kill"), format!("{foes} felled"));
        assert_eq!(s.console("god"), "god mode on");
        {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
            let c = a.world_mut().combat_mut();
            let ch = &mut c.scene.chars[c.kite.unwrap()];
            ch.hp = 1;
        }
        pad.read(&still);
        for _ in 0..30 {
            s.step(&pad);
        }
        {
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let c = a.world().combat();
            assert!(c.enemies().into_iter().all(|e| c.dead(e)), "an enemy still stands");
            let ch = &c.scene.chars[c.kite.unwrap()];
            assert_eq!(ch.hp, ch.max_hp);
        }
        assert_eq!(s.console("infection 150"), "infection 100%");
        assert_eq!(s.save_mut().unwrap().i16(piney_battle::exp::SAVE_EROSION), 100);
    }

    /// A member's item use reaches the world: in event 3's field Orca's AI
    /// asks `ccUseItemRequest(Orca, Kite, recovery row 0, 0)` (as its
    /// `UseItem` does; the stage queues the call), and the runtime runs the
    /// use: the item's heal skill (150) runs.
    #[test]
    fn a_member_uses_an_item() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
            let c = a.world_mut().combat_mut();
            let (Some(k), Some(o)) = (c.kite, c.party.members[1]) else { panic!("no Orca in the party") };
            let ch = &mut c.scene.chars[k];
            ch.hp = ch.max_hp / 4;
            c.member_items.push(piney_world::combat::MemberItem { user: o, target: k, code: 10 << 16, arg: 0 });
        }
        let mut pad = Pad::default();
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let mut cast = None;
        for _ in 0..30 {
            pad.read(&still);
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { break };
            let skills = a.world().combat().skills.borrow();
            if let Some(r) = skills.runs.iter().find(|r| r.id == 150) {
                cast = Some(r.id);
                break;
            }
        }
        assert_eq!(cast, Some(150), "Orca's item skill (heal, 150) never ran");
    }

    /// After [`drain_in_a_fight`] the goblin is its drained form: an enemy
    /// whose row is `ccGetDrainId` of a row fought before.
    #[test]
    fn data_drain_leaves_the_drained_form() {
        let mut seen = std::collections::BTreeSet::new();
        let mut last = Vec::new();
        let Some(s) = drain_in_a_fight(false, |s, _, _| {
            if let Stage::Area(a) = &s.stage {
                let c = a.world().combat();
                last = c.enemies().iter().filter_map(|&e| c.foes.get(e)?.as_ref().map(|f| (e, f.ent.id))).collect();
                seen.extend(last.iter().map(|x| x.1));
            }
        }) else {
            return;
        };
        let Stage::Area(a) = &s.stage else { panic!() };
        let t = &a.world().combat().data.t;
        let forms: Vec<i32> = seen.iter().map(|&id| piney_battle::enemy_ai::drain_id(t, id)).collect();
        eprintln!("rows seen {seen:?}, their drained forms {forms:?}, at the end {last:?}");
        assert!(last.iter().any(|x| forms.contains(&x.1)), "no drained form: {last:?}");
    }

    /// A goblin drained with its attack down: the drain's
    /// `clearConditionEnemy` deletes its condition's effect as it leaves
    /// the command lists. Nothing of it runs after, so no
    /// `DispConditionEffect` would end the effect later.
    #[test]
    fn a_drained_foe_keeps_no_condition_effect() {
        let (mut worn, mut gone) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        let Some(s) = drain_in_a_fight(false, |s, _, _| {
            let Stage::Area(a) = &mut s.stage else { return };
            let c = a.world_mut().combat_mut();
            if c.battle.in_battle == 0 {
                return;
            }
            let listed = c.enemies();
            for &e in &listed {
                let (temp, time) = c.scene.chars[e].temp_time_mut().unwrap();
                if time[0] == 0 {
                    temp[0] = -10;
                    time[0] = 7200;
                }
                if c.condition_effect(e).is_some() {
                    worn.insert(e);
                }
            }
            gone.extend(worn.iter().copied().filter(|e| !listed.contains(e)));
        }) else {
            return;
        };
        let Stage::Area(a) = &s.stage else { panic!() };
        let c = a.world().combat();
        assert!(!gone.is_empty(), "no foe wore its effect and left the lists: worn {worn:?}");
        for &e in &gone {
            assert_eq!(c.condition_effect(e), None, "foe {e}'s effect outlived its drain");
        }
    }

    /// Issue #5: a foe paralysed as skill 157 leaves it (900 frames,
    /// `conditionNum` 1) wears effect 1, the sparks: two generators of row
    /// 198 that never end by themselves. Drained, its `clearConditionEnemy`
    /// deletes them; once the drop is handed out none may follow it.
    #[test]
    fn a_drained_paralysed_foe_keeps_no_sparks() {
        use piney_battle::param::cond;
        let (mut worn, mut gone) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        let Some(s) = drain_in_a_fight(false, |s, _, _| {
            let Stage::Area(a) = &mut s.stage else { return };
            let c = a.world_mut().combat_mut();
            if c.battle.in_battle == 0 {
                return;
            }
            let listed = c.enemies();
            for &e in &listed {
                let ch = &mut c.scene.chars[e];
                // Kept up for the drain: the members' blows fall on it.
                if ch.cond[cond::DEAD] == 0 {
                    ch.hp = 9999;
                }
                if ch.cond[cond::PARALYSIS] == 0 && !worn.contains(&e) {
                    ch.cond.v[cond::PARALYSIS] = 900;
                    ch.condition_num = 1;
                }
                if c.condition_effect(e) == Some(1) {
                    worn.insert(e);
                }
            }
            gone.extend(worn.iter().copied().filter(|e| !listed.contains(e)));
        }) else {
            return;
        };
        let Stage::Area(a) = &s.stage else { panic!() };
        assert!(!gone.is_empty(), "no foe wore the sparks and left the lists: worn {worn:?}");
        for &e in &gone {
            assert_eq!(a.world().combat().condition_effect(e), None, "foe {e}'s effect outlived its drain");
        }
        let left: Vec<u32> = a.world().fx().census().char_generators;
        assert!(left.iter().all(|&g| !gone.contains(&(g as usize))), "sparks on drained foes {gone:?}: {left:?}");
    }

    /// Issue #5 with the reporter's scroll: The Hanged Man (`itemTblD` row
    /// 37, skill 157) used on the goblin from PERSONAL's Items, then the
    /// drain once it holds. No generator may follow the drained foe (the
    /// sparks, row 198, `killFlag` 1); the drain file's animations over it
    /// (`effAfterDrain` 12-14, `effProtect` 19-24) end with their clips,
    /// as a field's `ccEffectCtrl` finds them in `effectTbl`.
    #[test]
    fn a_foe_paralysed_by_the_hanged_man_and_drained_leaves_nothing() {
        use piney_battle::param::cond;
        let paralysed = |s: &Session| match &s.stage {
            Stage::Area(a) => {
                let c = a.world().combat();
                c.enemies().iter().any(|&e| c.scene.chars[e].cond[cond::PARALYSIS] != 0)
            }
            _ => false,
        };
        let given = std::cell::Cell::new(false);
        let plan = |s: &Session| {
            if paralysed(s) { Some(4) } else { given.get().then_some(5) }
        };
        let (mut held, mut gone) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        let Some(mut s) = drain_with(false, plan, |s, _, _| {
            let Stage::Area(a) = &mut s.stage else { return };
            if !given.get() {
                // Kite's first item: The Hanged Man (category 11), nine
                // of them should one be resisted.
                let save = &mut a.world_mut().state_mut().save;
                save.set_i16(offset::ITEM_LIST, 37);
                save.set_u8(offset::ITEM_LIST + 2, 11);
                save.set_u8(offset::ITEM_LIST + 3, 9);
                given.set(true);
            }
            let c = a.world_mut().combat_mut();
            if c.battle.in_battle == 0 {
                return;
            }
            let listed = c.enemies();
            for &e in &listed {
                let ch = &mut c.scene.chars[e];
                // Kept up for the drain: the members' blows fall on it.
                if ch.cond[cond::DEAD] == 0 {
                    ch.hp = 9999;
                }
                if ch.cond[cond::PARALYSIS] != 0 && c.condition_effect(e) == Some(1) {
                    held.insert(e);
                }
            }
            gone.extend(held.iter().copied().filter(|e| !listed.contains(e)));
        }) else {
            return;
        };
        assert!(!gone.is_empty(), "no foe the scroll paralysed left the lists: held {held:?}");
        let drain_clips = |e: &i16| (12..=14).contains(e) || (19..=24).contains(e);
        let mut pad = Pad::default();
        let mut left = Vec::new();
        for _ in 0..120 {
            run(&mut s, &mut pad, 0..1, &[]);
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let census = a.world().fx().census();
            let sparks: Vec<u32> =
                census.char_generators.iter().copied().filter(|&g| gone.contains(&(g as usize))).collect();
            assert!(sparks.is_empty(), "sparks on drained foes {gone:?}: {sparks:?}");
            left = census.effects.into_iter().filter(drain_clips).collect();
        }
        assert!(left.is_empty(), "the drain's clips still running 120 frames on: {left:?}");
    }

    /// Issue #10: an attack scroll (`itemTblD` row 0, skill 193, type
    /// 0xc106) used on the goblin from PERSONAL's Items by a Kite with no
    /// `targetChar`. `_ccSkillRequest`
    /// (stype 2) aims Kite at it (gcmn 0x00572b80), so his `AnimCtrl` casts
    /// it as act 17 (type 0x100), held where he stands (`restraintSW`,
    /// `stopFlag`), until the act ends.
    #[test]
    fn a_scroll_on_a_foe_is_cast_by_kite() {
        use piney_battle::chara::spc_flag::{RESTRAINT, STOP};
        use piney_battle::param::cond;
        // 0 the scroll to give, 1 to use, 2 watched, 3 the drain the
        // helper ends on.
        let stage = std::cell::Cell::new(0);
        let (mut used, mut acts, mut held, mut landed) = (None, Vec::new(), false, false);
        let plan = |_: &Session| match stage.get() {
            1 => Some(5),
            3 => Some(4),
            _ => None,
        };
        let Some(_) = drain_with(false, plan, |s, i, _| {
            let Stage::Area(a) = &mut s.stage else { return };
            if stage.get() == 0 {
                let save = &mut a.world_mut().state_mut().save;
                save.set_i16(offset::ITEM_LIST, 0);
                save.set_u8(offset::ITEM_LIST + 2, 11);
                save.set_u8(offset::ITEM_LIST + 3, 1);
                stage.set(1);
            }
            let c = a.world_mut().combat_mut();
            for e in c.enemies() {
                let ch = &mut c.scene.chars[e];
                landed |= used.is_some() && ch.hp < 9999;
                if ch.cond[cond::DEAD] == 0 {
                    ch.hp = 9999;
                }
            }
            let Some(k) = c.kite else { return };
            // The use: the scroll's skill (193) asked for, which the
            // party's and the skills' tasks take up in the menu's frame, or
            // the cast it starts (act 17).
            let ch = &c.scene.chars[k];
            if used.is_none() && (ch.skill_id == 193 || ch.spc_char.act_num == 17) {
                used = Some(i);
                stage.set(2);
            }
            // Until then no aim of his own, as when his last target fell
            // (`SetTargetDist` drops it) or none has struck him yet.
            if stage.get() == 1 {
                c.scene.chars[k].target_char = None;
            }
            let ch = &c.scene.chars[k];
            let Some(u) = used else { return };
            if i < u + 240 {
                acts.push((ch.skill_id, ch.spc_char.act_num));
                let f = ch.spc_char.flags;
                held |= ch.spc_char.act_num == 17 && f & RESTRAINT != 0 && f & STOP != 0;
            } else {
                stage.set(3);
            }
        }) else {
            return;
        };
        let u = used.expect("the scroll was never used");
        assert!(landed, "the spell never hurt the goblin");
        let cast = acts.iter().position(|&(_, a)| a == 17);
        assert!(cast.is_some(), "Kite never cast it (from frame {u}): {acts:?}");
        assert!(held, "Kite not held through the cast");
        assert!(acts.iter().skip(cast.unwrap_or(0)).any(|&(_, a)| a != 17), "the cast never ended: {acts:?}");
    }

    /// The goblin's drained form after [`drain_in_a_fight`] takes a blow:
    /// `EntryAffect(1, 20)` from Kite lowers its HP.
    #[test]
    fn a_drained_form_can_be_hurt() {
        let Some(mut s) = drain_in_a_fight(false, |_, _, _| {}) else { return };
        let (form, hp0) = {
            let Stage::Area(a) = &mut s.stage else { panic!() };
            let c = a.world().combat();
            let k = c.kite.unwrap();
            let form = c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0).expect("no drained form");
            let hp0 = c.scene.chars[form].hp;
            a.world_mut().entry_affect(form, Some(k), 1, [20, 0, 0]);
            (form, hp0)
        };
        let mut pad = Pad::default();
        run(&mut s, &mut pad, 0..30, &[]);
        let Stage::Area(a) = &s.stage else { panic!() };
        let c = a.world().combat();
        let ch = &c.scene.chars[form];
        eprintln!("drained form {form}: row {:?} hp {hp0} -> {}", ch.id(), ch.hp);
        assert!(ch.hp < hp0, "the drained form took no damage: {hp0} -> {}", ch.hp);
    }

    /// The goblin's drained form after [`drain_in_a_fight`] (row 129, 50
    /// HP, no Exdefense), fought with the attack button: after its grace
    /// (`dead` 1 for 60 frames) it is a target and falls.
    #[test]
    fn a_drained_form_falls_to_kites_blows() {
        let Some(mut s) = drain_in_a_fight(false, |_, _, _| {}) else { return };
        let mut pad = Pad::default();
        let mut last = String::new();
        let mut fell = None;
        for i in 0..6000u64 {
            let raw = {
                let Stage::Area(a) = &s.stage else { panic!("left the area") };
                let w = a.world();
                let c = w.combat();
                let k = c.kite.unwrap();
                let Some(form) = c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0) else {
                    fell = Some(i);
                    break;
                };
                let ch = &c.scene.chars[form];
                let st = format!(
                    "form {form} row {} hp {} dead {} hold {} aff {} {:?} listed {} kite target {:?}",
                    ch.id(),
                    ch.hp,
                    ch.cond[piney_battle::param::cond::DEAD],
                    ch.cond[piney_battle::param::cond::HOLD],
                    ch.affect.ty,
                    ch.affect.param,
                    c.scene.listed(form),
                    c.scene.chars[k].target_char,
                );
                last = st;
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                let q = ch.pos.map(f32::from_bits);
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                if (q[0] - p[0]).hypot(q[1] - p[1]) > 150.0 {
                    stick_toward(f32::from_bits(w.camera().rot()[2]), (q[0] - p[0]).atan2(-(q[1] - p[1])))
                } else if i.is_multiple_of(8) {
                    Raw { buttons: Buttons::CROSS, ..still }
                } else {
                    still
                }
            };
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
        }
        assert!(fell.is_some(), "the drained form still stands: {last}");
    }

    /// [`drain_in_a_fight`] with `drainDemo` off: no movie.
    #[test]
    fn data_drain_in_a_fight() {
        let Some(s) = drain_in_a_fight(false, |_, _, _| {}) else { return };
        let Stage::Area(a) = &s.stage else { panic!() };
        let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
        assert!(!calls.iter().any(|c| c.starts_with("drain_movie")), "a movie with drainDemo off: {calls:?}");
    }

    /// The same with `drainDemo` on: the goblin's movie (stream 109, a
    /// small enemy's) plays over the field, the menu waits for its end,
    /// then the rules and the drop as before.
    #[test]
    fn data_drain_movie_in_a_fight() {
        let mut movie_frames = 0;
        let Some(s) = drain_in_a_fight(true, |s, _, _| {
            if let Stage::Area(a) = &s.stage
                && a.ui().ctrl.menu == 66
                && Mode::archive(s).is_some()
            {
                movie_frames += 1;
            }
        }) else {
            return;
        };
        let Stage::Area(a) = &s.stage else { panic!() };
        let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
        assert!(calls.contains(&"drain_movie 109"), "no movie: {calls:?}");
        let enemy: Vec<&&str> = calls.iter().filter(|c| c.starts_with("drain_enemy")).collect();
        assert!(enemy.iter().any(|c| c.ends_with("true")), "no enemy drawn into it: {enemy:?}");
        assert!(movie_frames > 60, "the movie played {movie_frames} frames");
    }

    /// Pictures of the drain movie: `PINEY_SHOTS=DIR cargo test --release
    /// -p piney-game drain_movie_shots -- --ignored --nocapture` (default
    /// `/mnt/data/claude/scratch/drainmovie`).
    #[test]
    #[ignore]
    fn drain_movie_shots() {
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/drainmovie".into());
        std::fs::create_dir_all(&dir).unwrap();
        let mut gs: Option<piney_gs::Gs> = None;
        let mut n = 0u32;
        drain_in_a_fight(true, |s, _, frame| {
            let Stage::Area(a) = &s.stage else { return };
            if a.ui().ctrl.menu != 66 || Mode::archive(s).is_none() {
                return;
            }
            n += 1;
            if n % 40 != 1 {
                return;
            }
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            let (w, h) = g.target_size();
            let path = format!("{dir}/movie-{n:03}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
            println!("{path}");
        });
        assert!(n > 0, "no movie frames");
    }
    /// Event 3 plays its fights (TEACH-F, eventTblM103): the portal south
    /// of the start opens on a goblin the event holds; `player_skill` has
    /// the player's X attack it until it is down (Kite's hits land: its HP
    /// falls from 50 to 0 over several blows); Kite's experience rises; the
    /// skill lesson's menus (80 PERSONAL, 81 Skills, 82 the member) and the
    /// chat lesson's (83) open and close; the event ends with
    /// `menu_ban false`. Then, walking east, the next portal's goblin comes
    /// out and Orca, following, targets it and kills it.
    #[test]
    fn event_3_plays_its_fights() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut pad = Pad::default();
        let mut hps: Vec<i16> = Vec::new();
        let mut hit_in_skill = 0;
        let mut menus: Vec<i32> = Vec::new();
        let mut exp0 = None;
        let mut end = None;
        let mut sounds = 0;
        while end.is_none_or(|e| f < e) {
            f += 1;
            let raw = story_player(&s, f);
            pad.read(&raw);
            s.step(&pad);
            sounds += s.take_events().iter().filter(|e| matches!(e, Event::Se3d { .. })).count();
            let Stage::Area(a) = &s.stage else { panic!("left the field: {}", Mode::title(&s)) };
            let c = a.world().combat();
            let Some(k) = c.kite else { continue };
            exp0.get_or_insert(c.scene.chars[k].spc().map_or(0, |p| p.base.exp));
            if let Some(&e) = c.enemies().first() {
                let hp = c.scene.chars[e].hp;
                if hps.last() != Some(&hp) {
                    let skill = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                        "player_skill" => Some(true),
                        "player_skill done" => Some(false),
                        _ => None,
                    });
                    if !hps.is_empty() && skill == Some(true) {
                        hit_in_skill += 1;
                    }
                    hps.push(hp);
                }
            }
            let m = a.ui().menu_type();
            if m >= 0 && menus.last() != Some(&m) {
                menus.push(m);
            }
            if end.is_none() && a.calls().iter().any(|(_, c)| c == "menu_ban false") {
                end = Some(f + 30);
            }
            assert!(f < 30_000, "event 3 did not end: {}", Mode::title(&s));
        }
        assert_eq!(hps.first(), Some(&50), "the goblin came out whole: {hps:?}");
        assert_eq!(hps.last(), Some(&0), "and went down: {hps:?}");
        assert!(hps.windows(2).all(|w| w[1] < w[0]), "only falling: {hps:?}");
        assert!(hit_in_skill >= 3, "Kite's blows during player_skill: {hps:?}");
        for want in 80..=83 {
            assert!(menus.contains(&want), "menu {want} opened: {menus:?}");
        }
        let Stage::Area(a) = &s.stage else { unreachable!() };
        let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
        for want in ["player_skill done", "open_menu 80", "open_menu 83", "hold None"] {
            assert!(calls.contains(&want), "{want}: {calls:?}");
        }
        let c = a.world().combat();
        let k = c.kite.unwrap();
        let exp = c.scene.chars[k].spc().map_or(0, |p| p.base.exp);
        assert!(exp > exp0.unwrap(), "Kite gained experience: {exp0:?} -> {exp}");
        assert!(c.enemies().is_empty(), "the goblin gone");
        assert!(sounds > 0, "the fight's positioned sounds (the portal, the blows, the steps)");
        // The goblin's ccEnemyDustCtrl: dust at its feet as it ran.
        println!("dust rings {}", c.dust_rings);
        assert!(c.dust_rings > 0, "the goblin raised no dust");

        // On to the east portal: Orca fights.
        let mut goblin = None;
        let mut orca_on_it = false;
        let mut kite_on_it = false;
        let mut killed = false;
        let mut fought = false;
        walk_to_east_portal(&mut s, 600, true, |s| {
            let Stage::Area(a) = &s.stage else { return };
            let c = a.world().combat();
            fought |= c.battle.in_battle == 1;
            if let Some(&e) = c.enemies().first() {
                goblin.get_or_insert(e);
                let orca = c.members.iter().find(|m| Some(m.1) != c.kite).map(|m| m.1).unwrap();
                orca_on_it |= c.scene.chars[orca].target_char == Some(e);
                let k = c.kite.unwrap();
                kite_on_it |= c.scene.chars[k].skill_id == 1 && c.scene.chars[k].target_char == Some(e);
                killed |= c.scene.chars[e].hp == 0;
            }
        });
        assert!(goblin.is_some(), "the east portal's goblin came out");
        assert!(fought, "ccThGameCtrl's inBattle went to 1");
        assert!(orca_on_it, "Orca targeted it");
        assert!(kite_on_it, "the attack button sent Kite's normal attack at it");
        assert!(killed, "and it went down");
    }

    /// Kite walked from where he stands into the field's dungeon: to the
    /// entrance's middle (`FieldArea::dungeon_pos`), then on west down its
    /// steps onto the doorway's floor, the two triangles of the entrance's
    /// hit with attribute 0x20686f6f (bit 0x80000) 652-1182 west of the
    /// middle and 302 south to 290 north of it, z -465.5, where
    /// `ccPlayer::CollisionTest` calls `WORLD_MAN::Enter`. Until the scene is
    /// the dungeon's; `each` sees the session and the frame count after
    /// every frame. The frames it took.
    fn walk_into_dungeon(s: &mut Session, mut each: impl FnMut(&Session, u64, &Frame)) -> u64 {
        let mut pad = Pad::default();
        let mut n = 0u64;
        let mut down = false;
        loop {
            n += 1;
            assert!(n < 5_000, "never reached the dungeon: {}", Mode::title(s));
            let raw = {
                let Stage::Area(a) = &s.stage else { break };
                let w = a.world();
                if w.scene().area == 2 {
                    break;
                }
                let piney_world::field_world::Place::Field(fa) = w.place() else { break };
                let mid = fa.dungeon_pos().unwrap().map(f32::from_bits);
                let p = w.player().body.pos.map(f32::from_bits);
                let (dx, dy) = (mid[0] - p[0], mid[1] - p[1]);
                down |= dx * dx + dy * dy < 300.0 * 300.0;
                // Past the middle: toward the doorway's floor.
                let (dx, dy) = if down { (mid[0] - 1000.0 - p[0], mid[1] - p[1]) } else { (dx, dy) };
                let cam_z = f32::from_bits(w.camera().rot()[2]);
                stick_toward(cam_z, dx.atan2(-dy))
            };
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            each(s, n, &frame);
        }
        n
    }

    /// Event 4's way through its dungeon (`D0001`): floor 0's room 0 north
    /// to the large room 1, west into the dead end (room 2) and back, east
    /// to room 3 (the trap room: its portal and goblins), north to room 4
    /// and down its stairs; floor 1's room 0 north to room 1 and on to the
    /// statue room 4, then back to room 1.
    const EVENT_4_LEGS: [Leg; 10] = [
        Leg::Door(1),
        Leg::Door(2),
        Leg::Door(1),
        Leg::Door(3),
        Leg::Portal,
        Leg::Door(4),
        Leg::Stairs,
        Leg::Door(1),
        Leg::Door(4),
        Leg::Door(1),
    ];

    /// [`EVENT_4_LEGS`] as a player takes them: the dead end's treasure
    /// box opened on the way (menu 32), and the statue room's Gott statue
    /// (menu 38) before the way back.
    const STORY_4_LEGS: [Leg; 12] = [
        Leg::Door(1),
        Leg::Door(2),
        Leg::Open,
        Leg::Door(1),
        Leg::Door(3),
        Leg::Portal,
        Leg::Door(4),
        Leg::Stairs,
        Leg::Door(1),
        Leg::Door(4),
        Leg::Open,
        Leg::Door(1),
    ];

    /// A stretch of event 4's way through its dungeon.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Leg {
        /// Through the door of Kite's room to room `n` of the floor.
        Door(usize),
        /// Onto the stairs of Kite's room.
        Stairs,
        /// Up to the room's magic portal, then at its goblins (the attack
        /// button) until the portal and the goblins are gone.
        Portal,
        /// Up to the room's closed treasure box or idol, the action button
        /// on it (its menu played by [`story_player`]) until it is open.
        Open,
    }

    /// The point Kite heads for on a door or stairs leg in `d`, standing
    /// at `kite`: for a door, his room's centre (`stage` 0, round the
    /// large rooms' pillars), a point 900 inside the doorway (1: the door
    /// cells from his room to the next, `realmap[level]`), then 1500
    /// beyond it (2); for the stairs, the nearest point of his room (a
    /// 150-unit grid) whose ground has bit 0x80000 and leads no room on
    /// (`GotoNextRoom`'s stairs).
    fn leg_goal(d: &piney_world::dungeon_area::DungeonArea, kite: [f32; 4], leg: Leg, stage: u8) -> Option<[f32; 2]> {
        use piney_data::dungeon::{EAST, NO_ROOM, NORTH, SOUTH, WEST};
        let fl = d.floors.get(d.level)?;
        let cell = |v: f32| (v / 750.0) as i32;
        let at = |x: i32, y: i32| fl.map.cells().get((x * 80 + y) as usize).copied();
        let here = at(cell(kite[0]), cell(kite[1]))?.here;
        match leg {
            // The centre only when a wall stands between him and the
            // doorway (the wall segments at his waist, 95 up).
            Leg::Door(_) if stage == 0 => {
                let inner = leg_goal(d, kite, leg, 1)?;
                let at = |x: f32, y: f32| [x.to_bits(), y.to_bits(), (kite[2] + 95.0).to_bits(), 1f32.to_bits()];
                let clear = d.hits.clone().line(at(kite[0], kite[1]), at(inner[0], inner[1]), 0x4000_0001, 1).is_none();
                if clear {
                    return Some(inner);
                }
                let slot = fl.rooms.get(usize::from(here))?;
                Some([f32::from_bits(slot.pos[0]), f32::from_bits(slot.pos[1])])
            }
            Leg::Door(to) => {
                let mut sum = [0.0f32; 2];
                let mut n = 0.0;
                let mut dir = 0;
                for x in 0..80 {
                    for y in 0..80 {
                        let c = at(x, y)?;
                        if c.here == here && usize::from(c.next) == to && c.door != 0 {
                            sum[0] += x as f32 * 750.0 + 375.0;
                            sum[1] += y as f32 * 750.0 + 375.0;
                            n += 1.0;
                            dir = c.door;
                        }
                    }
                }
                if n == 0.0 {
                    return None;
                }
                let (dx, dy) = match dir {
                    NORTH => (0.0, -1.0),
                    SOUTH => (0.0, 1.0),
                    WEST => (-1.0, 0.0),
                    EAST => (1.0, 0.0),
                    _ => (0.0, 0.0),
                };
                let k = if stage >= 2 { 1500.0 } else { -900.0 };
                Some([sum[0] / n + dx * k, sum[1] / n + dy * k])
            }
            Leg::Stairs => {
                let mut hits = d.hits.clone();
                let mut best: Option<(f32, [f32; 2])> = None;
                for x in 0..80 {
                    for y in 0..80 {
                        let c = at(x, y)?;
                        if c.here != here || c.next != NO_ROOM {
                            continue;
                        }
                        for i in 0..5 {
                            for j in 0..5 {
                                let p = [
                                    x as f32 * 750.0 + 75.0 + 150.0 * i as f32,
                                    y as f32 * 750.0 + 75.0 + 150.0 * j as f32,
                                ];
                                let q = [p[0].to_bits(), p[1].to_bits(), kite[2].to_bits(), 1f32.to_bits()];
                                hits.land(q, 0x2000_0001);
                                if hits.num == 0 || hits.nearest.att & 0x8_0000 == 0 {
                                    continue;
                                }
                                let dd = (p[0] - kite[0]).powi(2) + (p[1] - kite[1]).powi(2);
                                if best.is_none_or(|b| dd < b.0) {
                                    best = Some((dd, p));
                                }
                            }
                        }
                    }
                }
                best.map(|b| b.1)
            }
            Leg::Portal | Leg::Open => None,
        }
    }

    /// Event 4 in its dungeon: [`story_player`] while the event holds Kite
    /// (its `menu_ban`, a menu open), else `legs` walked in turn - a door
    /// or stairs leg over once the scene's floor or block changes, the
    /// portal's once it and its goblins are gone - until the session leaves
    /// the areas or `frames` frames. `each` sees the session after every
    /// frame. The legs done.
    fn play_dungeon(
        s: &mut Session,
        f: &mut u64,
        legs: &[Leg],
        frames: u64,
        mut each: impl FnMut(&Session, &Frame),
    ) -> usize {
        play_dungeon_events(s, f, legs, frames, |s, frame, _| each(s, frame))
    }

    thread_local! {
        /// [`play_dungeon`] lets a stream play out rather than skip it.
        static WATCH_STREAMS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// [`play_dungeon`], `each` also seeing the frame's events.
    fn play_dungeon_events(
        s: &mut Session,
        f: &mut u64,
        legs: &[Leg],
        frames: u64,
        mut each: impl FnMut(&Session, &Frame, &[Event]),
    ) -> usize {
        let mut pad = Pad::default();
        let mut leg = 0;
        let mut stage = 0u8;
        let mut room = None;
        let mut goal: Option<[f32; 2]> = None;
        let mut fought = false;
        for _ in 0..frames {
            *f += 1;
            let watch = WATCH_STREAMS.with(|w| w.get()) && matches!(&s.stage, Stage::Area(a) if a.streaming());
            let raw = {
                let Stage::Area(a) = &s.stage else { break };
                let w = a.world();
                let sc = w.scene();
                if room.is_some_and(|r| r != (sc.floor, sc.block)) {
                    leg += 1;
                    stage = 0;
                    goal = None;
                }
                room = Some((sc.floor, sc.block));
                let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                    "menu_ban true" => Some(true),
                    "menu_ban false" => Some(false),
                    _ => None,
                });
                let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
                let c = w.combat();
                match (legs.get(leg), c.kite, w.place()) {
                    (Some(&l), Some(k), piney_world::field_world::Place::Dungeon(d))
                        if playing && banned != Some(true) && a.ui().menu_type() == -1 =>
                    {
                        let p = c.scene.chars[k].pos.map(f32::from_bits);
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                        let toward = |q: [f32; 2]| stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])));
                        if l == Leg::Portal {
                            // The portals of this room (the dungeon's others are
                            // on the list too, switched off).
                            let circles: Vec<usize> = c
                                .ctrl
                                .list(piney_battle::entry::Kind::Circle)
                                .into_iter()
                                .filter(|&m| c.ctrl.entry_obj(m).is_some_and(|o| o.obj_flag))
                                .collect();
                            let foe = c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0);
                            fought |= !circles.is_empty() || foe.is_some();
                            match (foe, circles.first()) {
                                (Some(e), _) => {
                                    let q = c.scene.chars[e].pos.map(f32::from_bits);
                                    if (q[0] - p[0]).hypot(q[1] - p[1]) > 180.0 {
                                        toward([q[0], q[1]])
                                    } else if f.is_multiple_of(8) {
                                        Raw { buttons: Buttons::CROSS, ..still }
                                    } else {
                                        still
                                    }
                                }
                                (None, Some(&m)) => {
                                    let q = c.scene.chars[m].pos.map(f32::from_bits);
                                    if (q[0] - p[0]).hypot(q[1] - p[1]) > 1500.0 { toward([q[0], q[1]]) } else { still }
                                }
                                (None, None) => {
                                    if fought && c.enemies().is_empty() {
                                        leg += 1;
                                    }
                                    still
                                }
                            }
                        } else if l == Leg::Open {
                            // The room's boxes and idols still shut (on the
                            // command lists); none: open.
                            let shut = c.ctrl.list(piney_battle::entry::Kind::Gimmick).into_iter().find(|&g| {
                                c.ctrl.entry_obj(g).is_some_and(|o| {
                                    o.obj_flag && o.act_num == 0 && (o.gim_id <= 5 || (38..=44).contains(&o.gim_id))
                                }) && c.scene.listed(g)
                            });
                            // The entry control not yet in this room: wait.
                            let set_up = (c.ctrl.floor, c.ctrl.block) == (sc.floor, sc.block);
                            match shut {
                                None if !set_up => still,
                                Some(g) => {
                                    let q = c.scene.chars[g].pos.map(f32::from_bits);
                                    // The statue stands on a base: in reach,
                                    // targeted, short of it.
                                    let d = (q[0] - p[0]).hypot(q[1] - p[1]);
                                    let aimed = w.command_target() == Some(g);
                                    if !(d < 220.0 || aimed && d < 500.0) {
                                        toward([q[0], q[1]])
                                    } else if aimed && f.is_multiple_of(8) {
                                        Raw { buttons: Buttons::CROSS, ..still }
                                    } else if aimed {
                                        still
                                    } else {
                                        toward([q[0], q[1]])
                                    }
                                }
                                None => {
                                    leg += 1;
                                    still
                                }
                            }
                        } else if d.here(c.scene.chars[k].pos) != usize::try_from(sc.block).ok() {
                            // The scene has moved on (a door or the stairs
                            // taken, the fade out running): nothing.
                            goal = None;
                            still
                        } else {
                            if goal.is_none() {
                                goal = leg_goal(d, p, l, stage);
                            }
                            match goal {
                                // Onto the stairs' slope: on until the scene changes.
                                Some(q) if l == Leg::Stairs => toward(q),
                                Some(q) if (q[0] - p[0]).hypot(q[1] - p[1]) > 200.0 => toward(q),
                                Some(_) if stage < 2 => {
                                    stage += 1;
                                    goal = leg_goal(d, p, l, stage);
                                    goal.map_or(still, toward)
                                }
                                _ => still,
                            }
                        }
                    }
                    _ => story_player(s, *f),
                }
            };
            let raw =
                if watch { Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() } } else { raw };
            pad.read(&raw);
            let frame = s.step(&pad);
            let events = s.take_events();
            each(s, &frame, &events);
        }
        leg
    }

    /// Event 4 (TEACH-D) plays through in the field's dungeon `D0001`, each
    /// block in its own room (the points from `WORLD_MAN::SetEventData`):
    /// the treasure boxes (menus 84, 85, `remove_trap`), stream 3, the dead
    /// end, the magic portal whose goblin falls and opens the doors
    /// (`MoveDoor`), the statue room, and floor 1's `mode 3` back to the
    /// desktop.
    #[test]
    fn event_4_plays_in_the_dungeon() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        walk_into_dungeon(&mut s, |_, _, _| {});
        let mut rooms: Vec<(i32, i32)> = Vec::new();
        // Each call the dungeon's areas made, with the room it was made in.
        let mut calls: Vec<((i32, i32), String)> = Vec::new();
        let (mut seen, mut mode) = (0, 0usize);
        let mut hps: Vec<i16> = Vec::new();
        let (mut orca_on_it, mut kite_on_it) = (false, false);
        // Room 3's doors: shut while the portal stood, open after.
        let (mut shut, mut opened) = (false, false);
        let done = play_dungeon(&mut s, &mut f, &EVENT_4_LEGS, 9000, |s, _| {
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let sc = w.scene();
            if sc.area != 2 {
                return;
            }
            let room = (sc.floor, sc.block);
            if rooms.last() != Some(&room) {
                rooms.push(room);
            }
            let here = &**a as *const AreaMode as usize;
            if here != mode || a.calls().len() < seen {
                (mode, seen) = (here, 0);
            }
            calls.extend(a.calls()[seen..].iter().map(|c| (room, c.1.clone())));
            seen = a.calls().len();
            let c = w.combat();
            if let Some(&e) = c.enemies().first() {
                let hp = c.scene.chars[e].hp;
                if hps.last() != Some(&hp) {
                    hps.push(hp);
                }
                let orca = c.members.iter().find(|m| Some(m.1) != c.kite).map(|m| m.1);
                orca_on_it |= orca.is_some_and(|o| c.scene.chars[o].target_char == Some(e));
                let k = c.kite.unwrap();
                kite_on_it |= c.scene.chars[k].skill_id == 1 && c.scene.chars[k].target_char == Some(e);
            }
            if room == (0, 3)
                && let piney_world::field_world::Place::Dungeon(d) = w.place()
            {
                // This room's portal (the dungeon's others are kept on the
                // list, switched off).
                let circle = c
                    .ctrl
                    .list(piney_battle::entry::Kind::Circle)
                    .into_iter()
                    .any(|m| c.ctrl.entry_obj(m).is_some_and(|o| o.obj_flag));
                shut |= circle && !d.doors.is_empty() && !d.door.door_flag && !d.door.still_open;
                opened |= shut && !circle && d.door.door_flag;
            }
        });
        assert_eq!(done, EVENT_4_LEGS.len(), "the legs walked: {rooms:?}");
        assert_eq!(
            rooms,
            [(0, 0), (0, 1), (0, 2), (0, 1), (0, 3), (0, 4), (1, 0), (1, 1), (1, 4), (1, 1)],
            "the rooms, in order"
        );
        // Each block's calls, only in its event point's room.
        let rooms_of = |want: &str| -> Vec<(i32, i32)> {
            let mut r: Vec<(i32, i32)> = calls.iter().filter(|c| c.1.starts_with(want)).map(|c| c.0).collect();
            r.dedup();
            r
        };
        for (want, room) in [
            ("message_open 4 0 ", (0, 0)),
            ("open_menu 84", (0, 0)),
            ("remove_trap", (0, 0)),
            ("open_menu 85", (0, 0)),
            ("stream 3", (0, 1)),
            ("stream done", (0, 1)),
            ("message_open 4 21 ", (0, 2)),
            ("entry_mc 0 130 100 1", (0, 3)),
            ("gimmick OpenDoor", (0, 3)),
            ("gimmick CloseDoor", (0, 3)),
            ("message_open 4 4 ", (0, 3)),
            ("message_open 4 7 ", (0, 3)),
            ("message_open 4 8 ", (1, 4)),
            ("message_open 4 12 ", (1, 4)),
            ("fade 1 128", (1, 1)),
        ] {
            assert_eq!(rooms_of(want), [room], "{want}: {calls:?}");
        }
        // Block 1 (talking to Orca) was not asked for.
        assert!(rooms_of("message_open 4 13 ").is_empty(), "{calls:?}");
        assert_eq!(hps.first(), Some(&50), "the goblin came out whole: {hps:?}");
        assert_eq!(hps.last(), Some(&0), "and went down: {hps:?}");
        assert!(hps.len() >= 2, "hits landed: {hps:?}");
        assert!(orca_on_it, "Orca took the goblin on");
        assert!(kite_on_it, "Kite's normal attack went at it");
        assert!(shut, "room 3's doors shut while its portal stood");
        assert!(opened, "and opened once it and its goblin were gone");
        // Block 8's `mode 3` (ChangeRequest(3, 7)): the desktop, where
        // block 9 plays at its phase 2.
        assert!(matches!(s.stage, Stage::Desktop(_)), "mode 3: {}", Mode::title(&s));
    }

    /// After event 3, Kite walked to the field's dungeon entrance and on
    /// in ([`walk_into_dungeon`]); then up to `$PINEY_FRAMES` frames of
    /// [`play_dungeon`] (event 4, TEACH-D) and `$PINEY_AFTER` frames on the
    /// desktop. Prints each room, the party and the foes every
    /// `$PINEY_EVERY` frames and every event call: `cargo test --release
    /// -p piney-game event_4_log -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_4_log() {
        let frames: u64 = std::env::var("PINEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let n = walk_into_dungeon(&mut s, |s, n, _| {
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let piney_world::field_world::Place::Field(fa) = w.place() else { return };
            if n.is_multiple_of(100) {
                let c = w.combat();
                let k = c.kite.unwrap();
                println!(
                    "{n}: kite {:?} hits {} att {:#x} act {} listed {}",
                    w.player().body.pos.map(f32::from_bits),
                    fa.hits.num,
                    fa.hits.nearest.att,
                    c.scene.chars[k].spc_char.act_num,
                    c.scene.listed(k)
                );
            }
        });
        println!("{n}: {}", Mode::title(&s));
        let mut k = 0u64;
        let mut last = None;
        let mut seen = 0;
        let done = play_dungeon(&mut s, &mut f, &EVENT_4_LEGS, frames, |s, _| {
            k += 1;
            let Stage::Area(a) = &s.stage else { return };
            let sc = a.world().scene();
            if last != Some((sc.floor, sc.block)) {
                last = Some((sc.floor, sc.block));
                println!("{k}: room {}/{} {}", sc.floor, sc.block, Mode::title(s));
            }
            if a.calls().len() < seen {
                seen = 0;
            }
            for (fr, c) in &a.calls()[seen..] {
                println!("  {k} ({fr}) {c}");
            }
            seen = a.calls().len();
            let every: u64 = std::env::var("PINEY_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(250);
            if !k.is_multiple_of(every) {
                return;
            }
            let c = a.world().combat();
            let d = |i: usize| {
                let ch = &c.scene.chars[i];
                let p = ch.pos.map(|v| f32::from_bits(v) as i32);
                format!(
                    "{i}@({},{},{}) hp{} act{} sk{} tg{:?}",
                    p[0], p[1], p[2], ch.hp, ch.spc_char.act_num, ch.skill_id, ch.target_char
                )
            };
            let party: Vec<String> = c.members.iter().map(|m| d(m.1)).collect();
            let foes: Vec<String> = c.enemies().into_iter().map(d).collect();
            let circles: Vec<String> = c.ctrl.list(piney_battle::entry::Kind::Circle).into_iter().map(d).collect();
            let last_call = a.calls().last().map(|c| c.1.clone()).unwrap_or_default();
            println!(
                "{k}: {} party {party:?} foes {foes:?} circles {circles:?} inBattle {} menu {} last {last_call}",
                Mode::title(s),
                c.battle.in_battle,
                a.ui().menu_type()
            );
        });
        println!("legs done {done}: {}", Mode::title(&s));
        // After the dungeon: the desktop, where block 9 plays.
        let mut pad = Pad::default();
        for i in 0..std::env::var("PINEY_AFTER").ok().and_then(|v| v.parse().ok()).unwrap_or(0u64) {
            if matches!(s.stage, Stage::Area(_)) {
                break;
            }
            f += 1;
            let raw = story_player(&s, f);
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            if i.is_multiple_of(200) {
                println!("after {i}: {}", Mode::title(&s));
            }
        }
        if let Stage::Area(a) = &s.stage {
            for (fr, c) in a.calls() {
                println!("{fr:5} {c}");
            }
        }
    }

    /// Prints the fight at event 3's east portal after the lessons.
    #[test]
    #[ignore]
    fn portal_fight_log() {
        let mut n = 0u32;
        let r = event_3_then_portal(1500, |s| {
            let Stage::Area(a) = &s.stage else { return };
            let c = a.world().combat();
            n += 1;
            if !n.is_multiple_of(10) || n > 500 {
                return;
            }
            let d = |i: usize| {
                let ch = &c.scene.chars[i];
                let p = ch.pos.map(|v| f32::from_bits(v) as i32);
                format!(
                    "{i}@({},{}) hp{} act{} sk{}/{} tg{:?}",
                    p[0], p[1], ch.hp, ch.spc_char.act_num, ch.skill_id, ch.skill_status, ch.target_char
                )
            };
            let party: Vec<String> = c.members.iter().map(|m| d(m.1)).collect();
            let foes: Vec<String> = c
                .enemies()
                .into_iter()
                .map(|e| {
                    format!(
                        "{} fact{} posP {:?} listed {} ty {:#x}",
                        d(e),
                        c.foes[e].as_ref().map_or(-1, |x| x.act_num),
                        c.scene.chars[e].pos_p.map(|v| f32::from_bits(v) as i32),
                        c.scene.listed(e),
                        c.scene.chars[e].ty()
                    )
                })
                .collect();
            // The portals: act, its count, the clip's end, switched on.
            let circles: Vec<String> = c
                .ctrl
                .list(piney_battle::entry::Kind::Circle)
                .into_iter()
                .filter_map(|m| {
                    let o = c.ctrl.entry_obj(m)?;
                    Some(format!(
                        "{m}:act{}/{} anm{} on{} drawn{:?}",
                        o.act_num,
                        o.act_cnt,
                        o.anm_flag,
                        o.obj_flag,
                        c.cast.get(m).map(|a| a.drawn)
                    ))
                })
                .collect();
            println!(
                "{n}: circles {circles:?} party {party:?} foes {foes:?} inBattle {} kdirc {} target {:?} fix {} plAttack {}",
                c.battle.in_battle,
                f32::from_bits(c.kite_dirc()[2]),
                a.world().command_target(),
                a.world().targeting().fix,
                a.ui().ctrl.pl_attack
            );
        });
        assert!(r.is_some());
    }

    /// Event 3's east portal from the frame it opens (act 1) until 40
    /// frames after the entry control deletes it, a shot into
    /// `$PINEY_SHOTS` every `$PINEY_EVERY` frames (default 5), each named
    /// for its frame and the portal's act, with the portal's act, count,
    /// transparency and the battle state printed:
    /// `PINEY_SHOTS=DIR cargo test --release -p piney-game portal_shots --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn portal_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        let every: u64 = std::env::var("PINEY_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut watched: Option<usize> = None;
        let (mut n, mut gone) = (0u64, None::<u64>);
        // `$PINEY_NEAR`: how close Kite walks to the portal (default 2500,
        // where the walk stops; nearer, the camera behind him looks at it).
        let near: f32 = std::env::var("PINEY_NEAR").ok().and_then(|v| v.parse().ok()).unwrap_or(2500.0);
        walk_near_east_portal(&mut s, 700, near, |s, frame| {
            n += 1;
            let Stage::Area(a) = &s.stage else { return };
            let c = a.world().combat();
            let circles = c.ctrl.list(piney_battle::entry::Kind::Circle);
            if watched.is_none() {
                watched = circles.iter().copied().find(|&m| c.ctrl.entry_obj(m).is_some_and(|o| o.act_num >= 1));
            }
            let Some(w) = watched else { return };
            let state = c.ctrl.entry_obj(w).filter(|_| circles.contains(&w)).map(|o| {
                let alpha = c.cast.get(w).map_or(0.0, |x| f32::from_bits(x.alpha));
                (o.act_num, o.act_cnt, f32::from_bits(o.set_transparency), alpha)
            });
            if state.is_none() && gone.is_none() {
                gone = Some(n);
            }
            if gone.is_some_and(|g| n > g + 40) {
                return;
            }
            let at = |i: usize| c.scene.chars[i].pos.map(|v| f32::from_bits(v) as i32);
            let cam = a.world().camera().active().pos.map(|v| f32::from_bits(v) as i32);
            let info = format!(
                "frame {n}: portal {state:?} at {:?} drawn {:?} play {:?} kite {:?} cam {:?} foes {} inBattle {}",
                at(w),
                c.cast.get(w).map(|x| x.drawn),
                c.cast.get(w).map(|x| (x.ch.play.anim, x.ch.play.time >> 8, x.ch.play.posed >> 8)),
                at(c.kite.unwrap()),
                cam,
                c.enemies().iter().filter(|&&e| c.scene.chars[e].hp > 0).count(),
                c.battle.in_battle
            );
            if n.is_multiple_of(every) {
                gs.set_overlay(Mode::archive(s));
                gs.render(frame);
                let (tw, th) = gs.target_size();
                let act = state.map_or(9, |x| x.0);
                let path = format!("{dir}/portal_{n:04}_act{act}.png");
                std::fs::write(&path, piney_gs::png::encode(tw, th, &gs.read_back())).unwrap();
                println!("{path}: {info}");
            } else {
                println!("{info}");
            }
        });
    }

    /// Pictures of event 3's fight: the story player of [`story_player`]
    /// from Log in, each frame drawn by the GS; at each mark (a call's start
    /// in the area's log, then some frames) the frame buffer goes to
    /// `$PINEY_SHOTS/<name>.png`. `PINEY_SHOTS=DIR cargo test --release -p
    /// piney-game event_3_shots -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_3_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let marks: Vec<(String, u64, String)> = std::env::var("PINEY_MARKS")
            .unwrap_or_else(|_| {
                "pc Command { pc: 0,60,portal;hold Some,30,goblins;player_skill,40,attack;player_skill,70,combo;\
                 player_skill done,2,down;open_menu 80,80,skills;open_menu 83,90,chat;menu_ban false,120,after"
                    .into()
            })
            .split(';')
            .filter_map(|m| {
                let mut it = m.split(',');
                Some((it.next()?.trim().to_string(), it.next()?.trim().parse().ok()?, it.next()?.trim().to_string()))
            })
            .collect();
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let vm = crate::world::new_game_events(&mut disc, &mut state).unwrap();
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let mut s = Session::in_world(iso, archive.clone(), None, state, Some(vm), scene, None).unwrap();
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
        let mut pad = Pad::default();
        let mut due: Vec<Option<u64>> = vec![None; marks.len()];
        let mut done = vec![false; marks.len()];
        let mut f = 0u64;
        while done.iter().any(|d| !d) && f < 12_000 {
            f += 1;
            let raw = story_player(&s, f);
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let Stage::Area(a) = &s.stage else { continue };
            for (k, (call, after, name)) in marks.iter().enumerate() {
                if done[k] {
                    continue;
                }
                if due[k].is_none() && a.calls().iter().any(|(_, c)| c.starts_with(call.as_str())) {
                    due[k] = Some(f + after);
                }
                if due[k] == Some(f) {
                    let (w, h) = gs.target_size();
                    let path = format!("{dir}/{name}.png");
                    std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
                    println!("{path}: {}", Mode::title(&s));
                    done[k] = true;
                }
            }
        }
    }

    /// Pictures of the fight at event 3's east portal after the lessons,
    /// each frame drawn by the GS: `approach` (Kite walking up, Orca on the
    /// goblin), `target` (the goblin the command target, its window and
    /// bar), `hit` (Kite's first blow landing), `orca` (Orca's blow),
    /// `skills` (a second run: the PERSONAL menu's Skills opened with the
    /// goblin targeted). `PINEY_SHOTS=DIR cargo test --release -p
    /// piney-game portal_fight_shots -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn portal_fight_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        for skills in [false, true] {
            let Some((mut s, mut f)) = story_to_field(0) else { return };
            let mut log = Vec::new();
            story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
            let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
            let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
            let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
            let mut pad = Pad::default();
            let goal = [28600.0f32, 24600.0];
            let mut taken: Vec<&str> = Vec::new();
            let mut due: Vec<(u64, &'static str)> = Vec::new();
            let mut opened = None;
            for i in 0..900u64 {
                let (raw, marks) = {
                    let Stage::Area(a) = &s.stage else { break };
                    let w = a.world();
                    let c = w.combat();
                    let k = c.kite.unwrap();
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let cam_z = f32::from_bits(w.camera().rot()[2]);
                    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                    let foe = c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0);
                    let targeted = foe.is_some() && w.command_target() == foe;
                    let mut marks: Vec<(u64, &'static str)> = Vec::new();
                    if let Some(e) = foe {
                        if !taken.contains(&"approach") {
                            marks.push((40, "approach"));
                        }
                        if targeted && !taken.contains(&"target") {
                            marks.push((2, "target"));
                        }
                        if c.scene.chars[e].hp < 50 && !taken.contains(&"hit") {
                            marks.push((1, "hit"));
                        }
                        let orca = c.members.iter().find(|m| Some(m.1) != c.kite).map(|m| m.1).unwrap();
                        if c.scene.chars[orca].spc_char.act_num == 15 && !taken.contains(&"orca") {
                            marks.push((6, "orca"));
                        }
                    }
                    let menu = a.ui().menu_type();
                    let raw = match foe {
                        // PERSONAL (SystemMenu, menu 1 in this field): its
                        // first row, Skills.
                        _ if skills && opened.is_some() => match (menu, a.ui().ctrl.proccess) {
                            (1, 1) if i.is_multiple_of(8) => Raw { buttons: Buttons::CROSS, ..still },
                            _ => still,
                        },
                        Some(_) if skills && targeted => {
                            opened = Some(i);
                            Raw { buttons: Buttons::TRIANGLE, ..still }
                        }
                        Some(e) => {
                            let q = c.scene.chars[e].pos.map(f32::from_bits);
                            let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
                            if ex * ex + ey * ey > 180.0 * 180.0 {
                                stick_toward(cam_z, ex.atan2(-ey))
                            } else if !skills && i.is_multiple_of(8) {
                                Raw { buttons: Buttons::CROSS, ..still }
                            } else {
                                still
                            }
                        }
                        None => {
                            let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                            if dx * dx + dy * dy < 2500.0 * 2500.0 { still } else { stick_toward(cam_z, dx.atan2(-dy)) }
                        }
                    };
                    if skills && opened.is_some() && menu >= 0 && menu != 1 && !taken.contains(&"skills") {
                        marks.push((10, "skills"));
                    }
                    (raw, marks)
                };
                for (after, name) in marks {
                    if !skills || name == "skills" {
                        taken.push(name);
                        due.push((i + after, name));
                    } else {
                        taken.push(name);
                    }
                }
                pad.read(&raw);
                let frame = s.step(&pad);
                s.take_events();
                gs.set_overlay(Mode::archive(&s));
                gs.render(&frame);
                for &(at, name) in due.iter().filter(|d| d.0 == i) {
                    let (w, h) = gs.target_size();
                    let path = format!("{dir}/{name}.png");
                    std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
                    println!("{path}: {} ({at})", Mode::title(&s));
                }
            }
        }
    }

    /// After event 3, at the east portal: the goblin targeted, PERSONAL,
    /// Skills, the first skill (Saber Dance) on it; prints Kite's act and
    /// skill and the goblin's HP: `cargo test --release -p piney-game
    /// art_on_a_goblin_log -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn art_on_a_goblin_log() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        let mut opened = false;
        for i in 0..900u64 {
            let raw = {
                let Stage::Area(a) = &s.stage else { break };
                let w = a.world();
                let c = w.combat();
                let k = c.kite.unwrap();
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                let cam_z = f32::from_bits(w.camera().rot()[2]);
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                let foe = c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0);
                let menu = a.ui().menu_type();
                if i.is_multiple_of(4) && (opened || foe.is_some()) {
                    let ch = &c.scene.chars[k];
                    println!(
                        "{i}: menu {menu}/{} kite act {} skill {}/{} sp {} target {:?} foe {:?}",
                        a.ui().ctrl.proccess,
                        ch.spc_char.act_num,
                        ch.skill_id,
                        ch.skill_status,
                        ch.sp,
                        w.command_target(),
                        foe.map(|e| c.scene.chars[e].hp)
                    );
                }
                match foe {
                    _ if opened => {
                        if menu >= 0 && i.is_multiple_of(8) {
                            Raw { buttons: Buttons::CROSS, ..still }
                        } else {
                            still
                        }
                    }
                    Some(e) if w.command_target() == Some(e) => {
                        opened = true;
                        Raw { buttons: Buttons::TRIANGLE, ..still }
                    }
                    Some(e) => {
                        let q = c.scene.chars[e].pos.map(f32::from_bits);
                        stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                    }
                    None => {
                        let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                        if dx * dx + dy * dy < 2500.0 * 2500.0 { still } else { stick_toward(cam_z, dx.atan2(-dy)) }
                    }
                }
            };
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
        }
    }

    /// After event 3 at the east portal: as soon as the goblin is out,
    /// Kite's SP filled and an attack spell (`PINEY_SID`, default
    /// 197, a Tornado) requested on it as the Skills menu would; prints the
    /// run and the goblin's HP: `cargo test --release -p piney-game
    /// spell_on_a_goblin_log -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn spell_on_a_goblin_log() {
        let sid: i32 = std::env::var("PINEY_SID").ok().and_then(|v| v.parse().ok()).unwrap_or(197);
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        let mut cast = false;
        let shots = std::env::var("PINEY_SHOTS").ok();
        let mut gs = shots.as_ref().map(|_| {
            let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
            let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
            piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap()
        });
        let mut shot_at: Option<u64> = None;
        for i in 0..700u64 {
            let raw = {
                let Stage::Area(a) = &mut s.stage else { break };
                let foe = {
                    let c = a.world().combat();
                    c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0)
                };
                if !cast && let Some(e) = foe {
                    let w = a.world_mut();
                    w.set_player_sp(200);
                    w.player_skill_request(Some(e), sid);
                    cast = true;
                }
                let w = a.world();
                let c = w.combat();
                let k = c.kite.unwrap();
                if cast && shot_at.is_none() && c.skills.borrow().runs.iter().any(|r| r.id == sid && r.count >= 40) {
                    shot_at = Some(i);
                }
                if cast && i.is_multiple_of(3) {
                    let runs: Vec<String> = c
                        .skills
                        .borrow()
                        .runs
                        .iter()
                        .map(|r| format!("{}:{} c{} st{} lv{}", r.key, r.id, r.count, r.status, r.level))
                        .collect();
                    println!(
                        "{i}: kite act {} skill {}/{} runs {runs:?} foe {:?}",
                        c.scene.chars[k].spc_char.act_num,
                        c.scene.chars[k].skill_id,
                        c.scene.chars[k].skill_status,
                        foe.map(|e| c.scene.chars[e].hp)
                    );
                }
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                let cam_z = f32::from_bits(w.camera().rot()[2]);
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                match foe {
                    _ if cast => still,
                    Some(e) => {
                        let q = c.scene.chars[e].pos.map(f32::from_bits);
                        let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
                        if ex * ex + ey * ey > 400.0 * 400.0 { stick_toward(cam_z, ex.atan2(-ey)) } else { still }
                    }
                    None => {
                        let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                        if dx * dx + dy * dy < 2500.0 * 2500.0 { still } else { stick_toward(cam_z, dx.atan2(-dy)) }
                    }
                }
            };
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            if let (Some(gs), Some(dir)) = (gs.as_mut(), shots.as_ref()) {
                gs.render(&frame);
                if shot_at == Some(i) {
                    let (w, h) = gs.target_size();
                    let path = format!("{dir}/spell_{sid}.png");
                    std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
                    println!("{path}");
                }
            }
        }
    }

    /// An attack spell lands through the effects: after event 3, as the
    /// east portal's goblin comes out, Kite casts Tornado (197) on it as
    /// the Skills menu would; its element system (piney-effect's
    /// `TornadoSystem`, run from `ccThSkill`) sets level 1 at its first
    /// frame, releases Kite, and from frame 35 its `ccSkillDamage` calls
    /// take the goblin's HP down while Orca's own attack is not running.
    /// Its blast in the camera's view breaks the screen up
    /// (`SetNoizBs(20)` on the field UI's noise).
    #[test]
    fn a_spell_lands_through_the_effects() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        let mut cast = None;
        let mut level = 0;
        let mut spell_hits = 0;
        let mut last_hp = None;
        let mut noise = 0;
        for _ in 0..500u64 {
            let raw = {
                let Stage::Area(a) = &mut s.stage else { break };
                noise = noise.max(a.ui().ctrl.noiz.bs);
                let foe = {
                    let c = a.world().combat();
                    c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0)
                };
                // Cast from near enough that the blast is in the camera's
                // reach (checkCameraShakeRange: within 2000 of the eye).
                let near = foe.is_some_and(|e| {
                    let c = a.world().combat();
                    let (k, t) = (c.scene.chars[c.kite.unwrap()].pos, c.scene.chars[e].pos);
                    let d = |i: usize| f32::from_bits(k[i]) - f32::from_bits(t[i]);
                    d(0) * d(0) + d(1) * d(1) < 1200.0 * 1200.0
                });
                if cast.is_none()
                    && near
                    && let Some(e) = foe
                {
                    let w = a.world_mut();
                    w.set_player_sp(200);
                    w.player_skill_request(Some(e), 197);
                    cast = Some(e);
                }
                let w = a.world();
                let c = w.combat();
                if let Some(e) = cast {
                    let run = c.skills.borrow().runs.iter().find(|r| r.id == 197).map(|r| (r.count, r.level));
                    let orca = c.members.iter().find(|m| Some(m.1) != c.kite).map(|m| m.1).unwrap();
                    let hp = c.scene.chars[e].hp;
                    if let Some((count, lv)) = run {
                        level = level.max(lv);
                        if last_hp.is_some_and(|h| hp < h) && count >= 35 && c.scene.chars[orca].skill_id != 1 {
                            spell_hits += 1;
                        }
                    }
                    last_hp = Some(hp);
                }
                let k = c.kite.unwrap();
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                // Toward the portal, then toward the goblin once it is out.
                let goal = foe.map_or(goal, |e| {
                    let t = c.scene.chars[e].pos.map(f32::from_bits);
                    [t[0], t[1]]
                });
                let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                let stop = if foe.is_some() { 1000.0 } else { 2500.0 };
                if cast.is_some() || dx * dx + dy * dy < stop * stop {
                    Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
                } else {
                    stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
                }
            };
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
        }
        assert!(cast.is_some(), "the goblin came out");
        assert_eq!(level, 1, "TornadoSystem set the spell's level");
        assert!(spell_hits >= 1, "the tornado's blows took HP");
        // SetNoizBs(20) by the effects (80) after the menu task (34) has
        // drawn this frame's noise: 20 at the frame's end.
        assert_eq!(noise, 20, "the blast's screen noise");
    }

    /// Event 3's field: Kite walked up to its goblin and, within 300, cast
    /// Flame Dance (art 9, fire) on it, then left standing; 500 frames,
    /// `each` seeing the session and the frame drawn after every one.
    /// Whether he reached the goblin; None without the disc.
    fn flame_dance(mut each: impl FnMut(&Session, &Frame)) -> Option<bool> {
        let (mut s, mut f) = story_to_field(0)?;
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let goal = [28600.0f32, 24600.0];
        let mut cast = None;
        for _ in 0..500u64 {
            let raw = {
                let Stage::Area(a) = &mut s.stage else { break };
                let foe = {
                    let c = a.world().combat();
                    c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0)
                };
                let near = foe.is_some_and(|e| {
                    let c = a.world().combat();
                    let (k, t) = (c.scene.chars[c.kite.unwrap()].pos, c.scene.chars[e].pos);
                    let d = |i: usize| f32::from_bits(k[i]) - f32::from_bits(t[i]);
                    d(0) * d(0) + d(1) * d(1) < 300.0 * 300.0
                });
                if cast.is_none()
                    && near
                    && let Some(e) = foe
                {
                    let w = a.world_mut();
                    w.set_player_sp(200);
                    w.player_skill_request(Some(e), 9);
                    cast = Some(e);
                }
                let w = a.world();
                let c = w.combat();
                let p = c.scene.chars[c.kite.unwrap()].pos.map(f32::from_bits);
                let goal = foe.map_or(goal, |e| {
                    let t = c.scene.chars[e].pos.map(f32::from_bits);
                    [t[0], t[1]]
                });
                let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                let stop = if foe.is_some() { 250.0 } else { 2500.0 };
                if cast.is_some() || dx * dx + dy * dy < stop * stop {
                    Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
                } else {
                    stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
                }
            };
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            each(&s, &frame);
        }
        Some(cast.is_some())
    }

    /// A skill from the menus in a fight on Mutation (reported from play:
    /// the Skills menu never let go, and the skill did nothing): field 27,
    /// Kite walked to the monster portal; in the fight PERSONAL, Skills, the
    /// first skill (Staccatto, 8), the target. The skill runs its own
    /// animation (`ANM_ctu0ski4`, `_ccSkillRequest`'s `ccAnm`): its notes
    /// hit the monster, its end ends the skill, and the menu shuts.
    #[test]
    fn mutation_skill_from_the_menus() {
        use piney_battle::entry::Kind;
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut s = super::area15::start(&iso, &archive, Some(27));
        let mut pad = Pad::default();
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let mut asked = None::<u64>;
        let (mut hp0, mut hp_low, mut shut) = (None::<i16>, i16::MAX, None::<u64>);
        for i in 0..3000u64 {
            let raw = {
                let Stage::Area(a) = &s.stage else {
                    pad.read(&still);
                    s.step(&pad);
                    continue;
                };
                let w = a.world();
                let c = w.combat();
                let Some(k) = c.kite else {
                    pad.read(&still);
                    s.step(&pad);
                    continue;
                };
                let ui = a.ui();
                let m = &ui.ctrl;
                let alive: Vec<usize> = c.enemies().into_iter().filter(|&e| !c.dead(e)).collect();
                if let Some(&e) = alive.first() {
                    let hp = c.scene.chars[e].hp;
                    if asked.is_some() {
                        hp_low = hp_low.min(hp);
                    } else {
                        hp0 = Some(hp);
                    }
                }
                if asked.is_some() && shut.is_none() && ui.menu_type() == -1 {
                    shut = Some(i);
                }
                if shut.is_some_and(|f| i > f + 30) {
                    break;
                }
                let press = |b: Buttons| if i.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
                let go_to = |row: usize| match m.list().select.max(0) as usize {
                    s if s == row => Buttons::CROSS,
                    s if s < row => Buttons::DOWN,
                    _ => Buttons::UP,
                };
                let p = c.scene.chars[k].pos.map(f32::from_bits);
                let list = if alive.is_empty() { c.ctrl.list(Kind::Circle) } else { alive.clone() };
                let target = list.iter().map(|&e| c.scene.chars[e].pos.map(f32::from_bits)).min_by(|a, b| {
                    let d = |q: [f32; 4]| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
                    d(*a).total_cmp(&d(*b))
                });
                let close = target.is_some_and(|q| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) < 350.0 * 350.0);
                match ui.menu_type() {
                    -1 if asked.is_some() => still,
                    -1 if c.battle.in_battle != 0 && close => press(Buttons::TRIANGLE),
                    -1 if !close => {
                        let q = target.unwrap_or(p);
                        stick_toward(f32::from_bits(w.camera().rot()[2]), (q[0] - p[0]).atan2(-(q[1] - p[1])))
                    }
                    -1 => still,
                    0..=2 if asked.is_none() => press(go_to(m.list().items.iter().position(|&x| x == 4).unwrap_or(0))),
                    4 if asked.is_none() => press(go_to(0)),
                    // The target: OK until the skill is asked for.
                    65 if asked.is_none() => {
                        if i.is_multiple_of(8) && c.scene.chars[k].skill_id == 8 {
                            asked = Some(i);
                        }
                        press(Buttons::CROSS)
                    }
                    _ => still,
                }
            };
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
        }
        let asked = asked.unwrap_or_else(|| panic!("no skill asked for: {}", Mode::title(&s)));
        let shut = shut.expect("the menu never shut after the skill");
        assert!(shut - asked < 400, "the skill held the menu {} frames", shut - asked);
        let hp0 = hp0.expect("no monster");
        assert!(hp_low < hp0, "Staccatto did no damage: {hp0} -> {hp_low}");
    }

    /// Kite's Flame Dance lights his blades: `StartArmsEffect` turns his
    /// `armsEffectSW` on and starts `particleEffectTbl[42]`'s generators
    /// between his weapon points, which run while the switch holds;
    /// `SetArmsEffectColor` turns both trails red (type 4) as he swings.
    #[test]
    fn flame_dance_lights_his_blades() {
        let (mut sw, mut gens, mut red) = (0, 0, 0);
        let cast = flame_dance(|s, _| {
            let Stage::Area(a) = &s.stage else { return };
            gens = gens.max(a.world().fx().census().weapon_generators);
            let c = a.world().combat();
            let k = c.kite.unwrap();
            sw = sw.max(c.scene.chars[k].spc_char.arms_effect_sw);
            if let Some(arms) = c.cast.get(k).and_then(|x| x.weapon.as_ref()) {
                red = red.max(arms.strips.iter().filter(|(st, _)| st[0].rgba[..3] == [0xf0, 0, 0]).count());
            }
        });
        let Some(cast) = cast else { return };
        assert!(cast, "Kite reached the goblin");
        assert_eq!(sw, 1, "armsEffectSW on");
        // Two blades: particleEffectTbl[42]'s generators on each.
        assert!(gens >= 2, "the blades' generators: {gens}");
        assert_eq!(red, 2, "both trails red");
    }

    /// Pictures of the Flame Dance: every eighth frame while his blades'
    /// generators run (`flame-N.png`), at most 8.
    /// `PINEY_SHOTS=DIR cargo test --release -p piney-game flame_dance_shots
    /// -- --ignored --nocapture` (default `/mnt/data/claude/scratch/weapon/shots`).
    #[test]
    #[ignore]
    fn flame_dance_shots() {
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/weapon/shots".into());
        std::fs::create_dir_all(&dir).unwrap();
        let mut gs: Option<piney_gs::Gs> = None;
        let (mut n, mut lit) = (0, 0);
        flame_dance(|s, frame| {
            let Stage::Area(a) = &s.stage else { return };
            if a.world().fx().census().weapon_generators == 0 || n >= 8 {
                return;
            }
            lit += 1;
            if lit % 8 != 1 {
                return;
            }
            let gs = gs.get_or_insert_with(|| {
                let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
                let mut disc = Iso::open(&iso).unwrap();
                let data = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
                piney_gs::Gs::headless(piney_gs::Assets::new(Mode::archive(s).unwrap_or(data))).unwrap()
            });
            gs.set_overlay(Mode::archive(s));
            gs.render(frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/flame-{n}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}");
            n += 1;
        });
    }

    /// A heal shows its light: Kite casts Repth (150) on himself in event
    /// 3's field; his `effSkillStart` puts up the controller -12, and
    /// `ccSkillRecovery`'s `effHealSkill` effect 130 (and the controller
    /// -19 of the rising lights). Before, both were dropped on the way to
    /// the effects.
    #[test]
    fn a_heal_shows_its_light() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let mut seen = std::collections::BTreeSet::new();
        for i in 0..200u32 {
            let Stage::Area(a) = &mut s.stage else { break };
            if i == 10 {
                let w = a.world_mut();
                let k = w.combat().kite;
                w.set_player_sp(200);
                w.player_skill_request(k, 150);
            }
            seen.extend(a.world_mut().fx_mut().census().effects);
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        assert!(seen.contains(&130) && seen.contains(&-19), "the heal's effects: {seen:?}");
        // ccPlayer::AnimCtrl's effSkillStart(this, 150, 0, 0) as he casts:
        // the controller -12 (its sparks, rings and circle).
        assert!(seen.contains(&-12), "the skill's start: {seen:?}");
    }

    /// A poisoned Kite shows it: his frame's `DispConditionEffect` picks
    /// condition 0 (poison: the green tint and its particles) and makes
    /// the effect; cured, the number goes back to -1 and the effect ends.
    #[test]
    fn a_poisoned_kite_shows_it() {
        let Some((mut s, mut f)) = story_to_field(0) else { return };
        let mut log = Vec::new();
        story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
        let mut pad = Pad::default();
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let mut nums = Vec::new();
        let mut particles = (0, 0);
        for i in 0..90u32 {
            let Stage::Area(a) = &mut s.stage else { break };
            let c = a.world_mut().combat_mut();
            let k = c.kite.unwrap();
            if i == 5 {
                c.scene.chars[k].cond[piney_battle::param::cond::POISON] = 5400;
            }
            if i == 60 {
                c.scene.chars[k].cond[piney_battle::param::cond::POISON] = 0;
            }
            nums.push(c.scene.chars[k].condition_num);
            let n = a.world_mut().fx_mut().census().particles;
            if i < 5 {
                particles.0 = particles.0.max(n);
            } else if i < 60 {
                particles.1 = particles.1.max(n);
            }
            pad.read(&still);
            s.step(&pad);
            s.take_events();
        }
        assert!(nums[10..60].iter().all(|&n| n == 0), "poison shown: {nums:?}");
        assert_eq!(nums[89], -1, "cured: {nums:?}");
        assert!(particles.1 > particles.0, "the poison's particles: {particles:?}");
    }

    /// Pictures of event 4 in the dungeon, as [`event_3_shots`] takes
    /// them: event 3 played out, the walk into the dungeon and
    /// [`play_dungeon`]'s legs, every frame drawn by the GS; at each mark (a
    /// call's start in the area's log, or `fight` the goblin's first hit,
    /// `down` its fall, `open` room 3's doors opening, then some frames) the
    /// frame buffer goes to `$PINEY_SHOTS/<name>.png`. `PINEY_SHOTS=DIR
    /// cargo test --release -p piney-game event_4_shots -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore]
    fn event_4_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let marks: Vec<(String, u64, String)> = std::env::var("PINEY_MARKS")
            .unwrap_or_else(|_| {
                "message_open 4 0,30,arrival;message_open 4 14,30,box_lesson;open_menu 84,40,box_menu;\
                 remove_trap,10,trap;stream 3,90,stream;gimmick CloseDoor,30,doors_closing;\
                 message_open 4 6,30,portal_lesson;fight,4,fight;fight,40,fight2;down,10,goblin_down;\
                 open,30,doors_open;message_open 4 9,30,statue;fade 1 128,1,end"
                    .into()
            })
            .split(';')
            .filter_map(|m| {
                let mut it = m.split(',');
                Some((it.next()?.trim().to_string(), it.next()?.trim().parse().ok()?, it.next()?.trim().to_string()))
            })
            .collect();
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let vm = crate::world::new_game_events(&mut disc, &mut state).unwrap();
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let mut s = Session::in_world(iso, archive.clone(), None, state, Some(vm), scene, None).unwrap();
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
        let mut pad = Pad::default();
        let mut f = 0u64;
        // Event 3 to its end.
        let mut left: Option<u64> = None;
        while left != Some(0) {
            f += 1;
            let raw = story_player(&s, f);
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let ended = matches!(&s.stage, Stage::Area(a) if a.calls().iter().any(|c| c.1 == "menu_ban false"));
            left = match left {
                None if ended => Some(60),
                Some(n) => Some(n - 1),
                l => l,
            };
            assert!(f < 30_000, "event 3 did not end");
        }
        walk_into_dungeon(&mut s, |s, _, frame| {
            gs.set_overlay(Mode::archive(s));
            gs.render(frame);
        });
        let mut due: Vec<Option<u64>> = vec![None; marks.len()];
        let mut done = vec![false; marks.len()];
        let (mut n, mut shut) = (0u64, false);
        play_dungeon(&mut s, &mut f, &EVENT_4_LEGS, 8000, |s, frame| {
            n += 1;
            gs.set_overlay(Mode::archive(s));
            gs.render(frame);
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let c = w.combat();
            let foe = c.enemies().first().map(|&e| c.scene.chars[e].hp);
            let room3 = (w.scene().floor, w.scene().block) == (0, 3);
            let (doors_open, circles) = match w.place() {
                piney_world::field_world::Place::Dungeon(d) => {
                    (d.door.door_flag, c.ctrl.list(piney_battle::entry::Kind::Circle).len())
                }
                _ => (false, 0),
            };
            shut |= room3 && circles > 0 && !doors_open;
            for (k, (call, after, name)) in marks.iter().enumerate() {
                if done[k] {
                    continue;
                }
                let fired = match call.as_str() {
                    "fight" => foe.is_some_and(|hp| hp < 50 && hp > 0),
                    "down" => foe == Some(0),
                    "open" => room3 && shut && circles == 0 && doors_open,
                    c if c.starts_with("room ") => c == format!("room {} {}", w.scene().floor, w.scene().block),
                    c => a.calls().iter().any(|(_, x)| x.starts_with(c)),
                };
                if due[k].is_none() && fired {
                    due[k] = Some(n + after);
                }
                if due[k] == Some(n) {
                    let (w, h) = gs.target_size();
                    let path = format!("{dir}/{name}.png");
                    std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
                    println!("{path}: {}", Mode::title(s));
                    done[k] = true;
                }
            }
        });
    }

    /// The presses a player makes through event 3 up to a moment of its
    /// fight, as `--press` wants them, with `--frames`: `PINEY_UNTIL` (a
    /// call's start, default `player_skill`) and `PINEY_MORE` frames after
    /// it (default 60). `cargo test --release -p piney-game
    /// event_3_fight_presses -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_3_fight_presses() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let until = std::env::var("PINEY_UNTIL").unwrap_or_else(|_| "player_skill".into());
        let more = std::env::var("PINEY_MORE").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let vm = crate::world::new_game_events(&mut disc, &mut state).unwrap();
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let mut s = Session::in_world(iso, archive, None, state, Some(vm), scene, None).unwrap();
        let mut f = 0;
        let mut log = Vec::new();
        story_until(&mut s, &mut f, &until, more, &mut log);
        let mut out = Vec::new();
        for (fr, raw) in &log {
            for (b, name) in [
                (Buttons::CROSS, "cross"),
                (Buttons::TRIANGLE, "triangle"),
                (Buttons::SQUARE, "square"),
                (Buttons::UP, "up"),
                (Buttons::DOWN, "down"),
                (Buttons::RIGHT, "right"),
                (Buttons::L1, "l1"),
                (Buttons::R2, "r2"),
            ] {
                if raw.buttons.contains(b) {
                    out.push(format!("{}:{name}", fr - 1));
                }
            }
            if raw.ry == 0 {
                out.push(format!("{}:rup", fr - 1));
            }
        }
        println!("--frames {f} --press {}", out.join(","));
    }

    /// Event 2's `scene` takes the party into story area 14's field: Orca
    /// comes too (party slot 1, beside Kite as `ccGetStartPositions` puts
    /// him), event 2 wakes to its `end_event` and is done by the set-up's
    /// `ccStartThEvent`, and event 3 (TEACH-F) opens there - its passes at
    /// phases 0 and 2, then Orca's camera lesson.
    /// Equipment the menu writes into the save for a member standing in the
    /// field stays there: the member's record is not written back over it,
    /// and the member's stats take the new piece at once.
    #[test]
    fn a_members_new_equipment_stays() {
        use piney_battle::param::SpcParam;
        let Some((mut s, _)) = story_to_field(700) else { return };
        let Stage::Area(a) = &mut s.stage else { panic!("left the field: {}", Mode::title(&s)) };
        let before = SpcParam::from_save(&a.world().state().save, 2);
        let mut p = before;
        // A body piece Orca does not wear (a real row of the body table).
        p.equipment[1] = if before.equipment[1] == 3 { 4 } else { 3 };
        p.store(a.save_mut(), 2);
        let pad = Pad::default();
        for _ in 0..3 {
            s.step(&pad);
        }
        let Stage::Area(a) = &s.stage else { panic!() };
        let after = SpcParam::from_save(&a.world().state().save, 2);
        assert_eq!(after.equipment, p.equipment, "the save's equipment was written over");
        // CalcReal took the piece: the equipment's sum changed.
        let c = a.world().combat();
        let orca = c.who(2).expect("Orca built");
        let tune = c.scene.chars[orca].spc().unwrap().tune;
        assert_ne!(tune, before.tune, "Orca's stats did not take the new piece");
        // Kite's own record the same way.
        let Stage::Area(a) = &mut s.stage else { panic!() };
        let mut k = SpcParam::from_save(&a.world().state().save, 0);
        let body = if k.equipment[1] == 3 { 4 } else { 3 };
        k.equipment[1] = body;
        k.store(a.save_mut(), 0);
        s.step(&pad);
        let Stage::Area(a) = &mut s.stage else { panic!() };
        assert_eq!(SpcParam::from_save(&a.world().state().save, 0).equipment[1], body);
        // A blade of another file: ChangeEquip's ChangeWeapon hangs it on
        // his hands.
        let t = &a.world().combat().data.t;
        let (now, _) = piney_world::body::weapon_of(t, &a.world().state().save, 0).expect("Kite's blades");
        let (i, other) = (0..64)
            .find_map(|i| {
                let n = String::from_utf8(t.job_weapon(0, i)?.ccsname.clone()).ok()?;
                (!n.is_empty() && n != now).then_some((i, n))
            })
            .expect("another blade");
        k.equipment[4] = i as i16;
        k.store(a.save_mut(), 0);
        a.world_mut().change_equip(0, 0);
        let c = a.world().combat();
        let kite = c.cast.get(c.who(0).expect("Kite built")).expect("Kite drawn");
        let hung: Vec<&str> = kite.ch.body.attached.iter().filter_map(|x| x.file.ccs.object_name(x.model)).collect();
        for side in ["r", "l"] {
            assert!(hung.contains(&format!("MDL_{other}{side}").as_str()), "{other} not in his hands: {hung:?}");
        }
    }

    /// Issue #34. A lake (field type 4: story area 33) is entered from the
    /// town straight into its first dungeon, which is its field:
    /// `ccPlayer::ccPlayer` has Kite arrive there (act 13, the gate-in), and
    /// the triangle opens the field's PERSONAL (menu 1, Gate Out its last
    /// row), as the lakes' dungeon types (8, 9) choose it.
    #[test]
    fn a_lake_entered_from_town_is_its_field() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let wm = crate::area::story_world_man(&mut disc, 33, false).unwrap();
        assert_eq!(wm.field_type, 4, "area 33 is a lake");
        let mut scene = piney_world::area::Scene::log_in(&mut state.save);
        scene.change_scene(2, scene.town, 33, 0, 0, 0, &mut state.save);
        assert_eq!(scene.area_prev, 0, "from the town");
        let mut a = AreaMode::enter(
            &iso,
            archive,
            state,
            None,
            scene,
            wm,
            None,
            Arrival::default(),
            piney_world::party::Spcs::default(),
        )
        .unwrap();
        let mut pad = Pad::default();
        let kite_act = |a: &AreaMode| {
            let c = a.world().combat();
            c.kite.map(|k| c.scene.chars[k].spc_char.act_num)
        };
        let mut arrived = false;
        for _ in 0..600 {
            arrived |= kite_act(&a) == Some(piney_battle::kite::act::ARRIVE);
            if arrived && matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 120) {
                break;
            }
            a.step(&pad);
        }
        assert!(arrived, "Kite did not arrive (act 13)");
        let mut opened = None;
        for f in 0..120u32 {
            let b = if f.is_multiple_of(30) { Buttons::TRIANGLE } else { Buttons::NONE };
            pad.read(&Raw { buttons: b, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            a.step(&pad);
            if a.ui().menu_type() != -1 {
                opened = Some(a.ui().menu_type());
                break;
            }
        }
        assert_eq!(opened, Some(1), "the lake's PERSONAL is the field's");
        let l = a.ui().ctrl.list();
        let rows: Vec<i16> = l.items.iter().take(l.y.max(0) as usize).copied().collect();
        assert_eq!(rows.last(), Some(&10), "Gate Out is its last row: {rows:?}");
    }

    /// A lake (field type 4, story area 33) has two dungeons, and
    /// `WORLD_MAN` keeps each (`dungeon[n]`): the lake's stairs down make
    /// the second, of its own type (`dungeonType[1]`, not a lake), and the
    /// way back up finds the lake as it was left, the party arriving at
    /// its stairs (`GO(2)`, `DungeonArea::come_back`). Before, the one
    /// dungeon kept was taken for whichever came next.
    #[test]
    fn a_lake_keeps_both_its_dungeons() {
        use piney_world::evarea::Kept;
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut disc).unwrap();
        let wm = crate::area::story_world_man(&mut disc, 33, false).unwrap();
        assert_eq!(wm.field_type, 4, "area 33 is a lake");
        let mut scene = piney_world::area::Scene::log_in(&mut state.save);
        scene.change_scene(1, scene.town, 33, -1, -1, -1, &mut state.save);
        scene.change_area(2, 0, &mut state.save);
        // Each visit plays a few frames, past the entry control's set-up:
        // the portals and gimmicks then in the lake (dungeon 0).
        let visit = |state, scene, kept| {
            let mut a = AreaMode::enter(
                &iso,
                archive.clone(),
                state,
                None,
                scene,
                wm,
                kept,
                Arrival::default(),
                piney_world::party::Spcs::default(),
            )
            .unwrap();
            let pad = Pad::default();
            for _ in 0..400 {
                if matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 2) {
                    break;
                }
                a.step(&pad);
            }
            let e = a.world().map_entries();
            let placed = (e.dungeon_circles.len(), e.gims.len());
            let (state, _, scene, _, kept, _) = a.leave();
            let Some(Kept::Dungeon(k)) = kept else { panic!("no dungeon kept") };
            let lakes: Vec<Option<bool>> =
                k.slots.iter().map(|d| d.as_ref().map(|d| piney_data::dungeon::is_lake(d.dtype))).collect();
            let back = k.slots[0].as_ref().map(|d| (d.position, d.floors[0].start[1]));
            (state, scene, Some(Kept::Dungeon(k)), lakes, back, placed)
        };
        // The lake; then down its stairs into the second dungeon.
        let (mut state, mut scene, mut kept, lakes, _, first) = visit(state, scene, None);
        assert_eq!(lakes, [Some(true), None, None]);
        assert!(first.0 + first.1 > 0, "nothing placed in the lake: {first:?}");
        // The lake's stairs down clear its entryFlag (WORLD_MAN::Enter).
        if let Some(Kept::Dungeon(k)) = &mut kept {
            k.slots[0].as_mut().unwrap().gimmicks_placed = false;
        }
        scene.change_scene(2, -2, -2, 1, 0, 0, &mut state.save);
        let (mut state, mut scene, kept, lakes, _, _) = visit(state, scene, kept);
        assert_eq!(lakes, [Some(true), Some(false), None], "the second dungeon made, the lake kept");
        // Back up to the lake's room left (lastRoom 0 here): the lake again,
        // its portals and gimmicks placed again.
        scene.change_scene(2, -2, -2, 0, 0, 0, &mut state.save);
        let (_, _, _, lakes, back, again) = visit(state, scene, kept);
        assert_eq!(lakes, [Some(true), Some(false), None]);
        let (pos, stairs) = back.unwrap();
        assert_eq!(pos, stairs, "at the lake's stairs down");
        assert_eq!(again, first, "the lake's portals and gimmicks again");
    }

    #[test]
    fn event_3_opens_in_the_field() {
        let Some((s, _)) = story_to_field(700) else { return };
        let Stage::Area(a) = &s.stage else { panic!("left the field: {}", Mode::title(&s)) };
        let w = a.world();
        assert_eq!((w.scene().area, w.scene().field), (1, 14));
        assert_eq!(w.party(), [0, 2, -1]);
        let orca = w.char_pos(2, 2).expect("Orca in the field");
        let kite = w.player().body.pos;
        // Placed at StartPos[1] (Kite's start + (200, 100)); Kite's own
        // place is where he arrived.
        let d = |i: usize| f32::from_bits(orca[i]) - f32::from_bits(kite[i]);
        assert!((d(0) - 200.0).abs() < 1.0 && (d(1) - 100.0).abs() < 1.0, "Orca at {:?}", orca.map(f32::from_bits));
        let st = &w.state().save;
        use piney_event::state::{DONE, ScriptSave as _};
        assert_ne!(st.flags(2) & DONE, 0, "event 2 done");
        let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
        for msg in 2..=6 {
            let want = format!("message_open 3 {msg} ");
            assert!(calls.iter().any(|c| c.starts_with(&want)), "Orca's line {msg}: {calls:?}");
        }
    }

    /// `--mode story:4` played as a player plays it ([`STORY_4_LEGS`]): the
    /// tutorial's boxes (the trapped one disarmed by Orca's Fortune Wire,
    /// EntryAffect 12's untrapped twin), the dead end's box, floor 1's
    /// minimap, the Gott statue's three items; the save's items and counts
    /// (`itemBoxCount` 3 up, `itemIdolCount` 1 up); `mode 3` to the desktop;
    /// no host call left at its default.
    #[test]
    fn story_4_plays_through() {
        use piney_data::save::offset;
        let Some(mut s) = story_session(4) else { return };
        let _ = piney_event::host::take_unported();
        let count = |s: &Session, cat: i32, id: i32| -> Option<(i32, i16, i16)> {
            let Stage::Area(a) = &s.stage else { return None };
            let save = &a.world().state().save;
            Some((
                piney_battle::item::get_item_num(save, 0, cat, id),
                save.i16(offset::ITEM_BOX_COUNT),
                save.i16(offset::ITEM_IDOL_COUNT),
            ))
        };
        let before = [count(&s, 10, 5).unwrap(), count(&s, 11, 0x38).unwrap(), count(&s, 0, 1).unwrap()];
        let mut after = before;
        let mut f = 0u64;
        let (mut seen, mut mode) = (0, 0usize);
        let mut rooms: Vec<(i32, i32)> = Vec::new();
        let mut calls: Vec<((i32, i32), String)> = Vec::new();
        // The first room's boxes: (row, param[2], position) as seen.
        let mut boxes: Vec<(i32, i32, [i32; 2])> = Vec::new();
        let mut hp_at_trap: Vec<i16> = Vec::new();
        let mut minimap_f1 = false;
        let mut idol_open = false;
        let legs = play_dungeon(&mut s, &mut f, &STORY_4_LEGS, 12000, |s, _| {
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let sc = w.scene();
            if sc.area != 2 {
                return;
            }
            let room = (sc.floor, sc.block);
            if rooms.last() != Some(&room) {
                rooms.push(room);
            }
            let here = &**a as *const AreaMode as usize;
            if here != mode || a.calls().len() < seen {
                (mode, seen) = (here, 0);
            }
            calls.extend(a.calls()[seen..].iter().map(|c| (room, c.1.clone())));
            seen = a.calls().len();
            let c = w.combat();
            let gims: Vec<(i32, i32, [i32; 2], i32)> = c
                .ctrl
                .list(piney_battle::entry::Kind::Gimmick)
                .into_iter()
                .filter_map(|g| {
                    let o = c.ctrl.entry_obj(g)?;
                    let p = c.scene.chars[g].pos.map(|v| f32::from_bits(v) as i32);
                    o.obj_flag.then_some((o.gim_id, o.ent.param[2], [p[0], p[1]], o.act_num))
                })
                .collect();
            if room == (0, 0) {
                for g in gims.iter().filter(|g| g.0 <= 1) {
                    if !boxes.iter().any(|b| (b.0, b.1, b.2) == (g.0, g.1, g.2)) {
                        boxes.push((g.0, g.1, g.2));
                    }
                }
                // From the Fortune Wire to the box opened.
                let wired = calls.iter().any(|c| c.1 == "remove_trap");
                let opened = calls.iter().any(|c| c.1.starts_with("open_menu 85"));
                if wired || opened {
                    hp_at_trap.push(c.kite.map_or(0, |k| c.scene.chars[k].hp));
                }
            }
            if room == (1, 4) {
                idol_open |= gims.iter().any(|g| (38..=44).contains(&g.0) && g.3 >= 1 && g.1 == 0);
            }
            if sc.floor == 1 && !a.map_state().last.is_empty() && f32::from_bits(a.map_state().alpha) > 0.0 {
                minimap_f1 = true;
            }
            if let Some(n) = count(s, 10, 5) {
                after = [n, count(s, 11, 0x38).unwrap(), count(s, 0, 1).unwrap()];
            }
        });
        assert_eq!(legs, STORY_4_LEGS.len(), "the legs walked: {rooms:?}");
        assert_eq!(
            rooms,
            [(0, 0), (0, 1), (0, 2), (0, 1), (0, 3), (0, 4), (1, 0), (1, 1), (1, 4), (1, 1)],
            "the rooms, in order"
        );
        let rooms_of = |want: &str| -> Vec<(i32, i32)> {
            let mut r: Vec<(i32, i32)> = calls.iter().filter(|c| c.1.starts_with(want)).map(|c| c.0).collect();
            r.dedup();
            r
        };
        for (want, room) in [
            ("message_open 4 0 ", (0, 0)),
            ("open_menu 84", (0, 0)),
            ("remove_trap", (0, 0)),
            ("open_menu 85", (0, 0)),
            ("stream 3", (0, 1)),
            ("message_open 4 21 ", (0, 2)),
            ("entry_mc 0 130 100 1", (0, 3)),
            ("gimmick OpenDoor", (0, 3)),
            ("gimmick CloseDoor", (0, 3)),
            ("message_open 4 7 ", (0, 3)),
            ("message_open 4 8 ", (1, 4)),
            ("message_open 4 12 ", (1, 4)),
            ("fade 1 128", (1, 1)),
        ] {
            assert_eq!(rooms_of(want), [room], "{want}: {calls:?}");
        }
        // The tutorial's boxes: the plain one and the trapped one (trap
        // 0), then the trapped one's untrapped twin in its place.
        let trapped = boxes.iter().find(|b| b.0 == 1).copied();
        assert!(boxes.iter().any(|b| b.0 == 0 && b.1 != 3), "the plain box: {boxes:?}");
        let Some(trapped) = trapped else { panic!("no trapped box: {boxes:?}") };
        assert_eq!(trapped.1, 0, "its trap: {boxes:?}");
        assert!(boxes.iter().any(|b| b.0 == 0 && b.1 == 3 && b.2 == trapped.2), "the disarmed twin: {boxes:?}");
        // Orca's Fortune Wire: the trap does not go off on Kite.
        assert!(!hp_at_trap.is_empty() && hp_at_trap.iter().all(|&h| h == hp_at_trap[0]), "Kite's HP: {hp_at_trap:?}");
        assert!(minimap_f1, "floor 1's minimap");
        assert!(idol_open, "the Gott statue opened");
        // The Resurrect, the dead end's item, the idol's own (0:1).
        assert_eq!(after[0].0, before[0].0 + 1, "the Resurrect: {before:?} -> {after:?}");
        assert_eq!(after[1].0, before[1].0 + 1, "the dead end's box: {before:?} -> {after:?}");
        assert!(after[2].0 > before[2].0, "the idol's item: {before:?} -> {after:?}");
        assert_eq!(after[0].1, before[0].1 + 3, "itemBoxCount");
        assert_eq!(after[0].2, before[0].2 + 1, "itemIdolCount");
        assert!(matches!(s.stage, Stage::Desktop(_)), "mode 3: {}", Mode::title(&s));
        assert_eq!(piney_event::host::take_unported(), Vec::<&str>::new(), "host defaults");
    }

    /// Event 4's end as the sound hears it: from block 8's `mode 3` in
    /// room (1, 1) through the desktop set-up's streams 4-6 (Skeith), every
    /// sound event printed with its frame (`--nocapture`), the streams let
    /// play.
    #[test]
    #[ignore]
    fn story_4_end_sound_log() {
        let Some(mut s) = story_session(4) else { return };
        WATCH_STREAMS.with(|w| w.set(true));
        let mut f = 0u64;
        let show = |f: u64, events: &[Event]| {
            for e in events {
                let d = format!("{e:?}");
                if !["StreamPcm", "SceneSound", "InBattle", "Se3d", "MovieAudio("].iter().any(|p| d.starts_with(p)) {
                    eprintln!("{f:5} {}", d.chars().take(160).collect::<String>());
                }
            }
        };
        let mut logging = false;
        play_dungeon_events(&mut s, &mut f, &STORY_4_LEGS, 20000, |s, _, events| {
            let Stage::Area(a) = &s.stage else { return };
            logging |= a.world().scene().floor == 1 && a.calls().iter().any(|c| c.1.starts_with("fade 1 128"));
            if logging {
                show(0, events);
            }
        });
        eprintln!("-- left the area: {}", Mode::title(&s));
        let mut pad = Pad::default();
        for k in 1..=3000u64 {
            let streaming = match &s.stage {
                Stage::Desktop(d) => d.streaming(),
                _ => false,
            };
            let raw = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let raw = if streaming || k % 8 != 0 { raw } else { Raw { buttons: Buttons::CROSS, ..raw } };
            pad.read(&raw);
            s.step(&pad);
            let events = s.take_events();
            if k == 1 || k % 500 == 0 {
                eprintln!("-- {k}: {} streaming {streaming}", Mode::title(&s));
            }
            show(k, &events);
        }
    }

    /// `--mode story:4` to the end of block 4 in the first room (its boxes
    /// opened, Kite free), then a treasure box of `row` holding item 10:1
    /// with trap 0 put 250 east of Kite (`FieldWorld::put_box`), and a
    /// Fortune Wire in the bag. The session, the frame count and the box.
    fn story_4_with_box(row: i32) -> Option<(Session, u64, usize)> {
        use piney_event::state::ScriptSave as _;
        let (mut s, f) = story_4_first_room()?;
        let Stage::Area(a) = &mut s.stage else { return None };
        let w = a.world_mut();
        let k = w.combat().kite?;
        let mut pos = w.combat().scene.chars[k].pos;
        pos[0] = (f32::from_bits(pos[0]) + 250.0).to_bits();
        let b = w.put_box(row, pos, 0, 0xa_0001, 0)?;
        if piney_battle::item::get_item_num(&w.state().save, 0, 13, 0) == 0 {
            w.state_mut().save.add_item(0, 13, 0, 1);
        }
        Some((s, f, b))
    }

    /// `--mode story:4` to the end of block 4 in the first room: its boxes
    /// opened, Kite free for 30 frames. The session and the frame count.
    fn story_4_first_room() -> Option<(Session, u64)> {
        let mut s = story_session(4)?;
        let mut pad = Pad::default();
        let mut f = 0u64;
        let mut free = 0;
        while free < 30 {
            f += 1;
            assert!(f < 3000, "block 4 did not end: {}", Mode::title(&s));
            let raw = story_player(&s, f);
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let done = a.calls().iter().any(|c| c.1.starts_with("open_menu 85"))
                && a.calls().last().is_some_and(|c| c.1 == "play_pass_done")
                && a.ui().menu_type() == -1;
            free = if done { free + 1 } else { 0 };
        }
        Some((s, f))
    }

    /// The dungeon's side of the events' `item_add` and `room` in story 4's
    /// first room (the ones S108 and S111 use with Sanjuro and Natsume):
    /// `item_add` for a member the area built (Orca) goes to him through
    /// `AddSpcItem`, one not built (Sanjuro) is declined; `room 0 B`
    /// builds room B, stands the leader 200 in front of its first way in
    /// (w 1.0) and changes the scene to it, as a door does.
    #[test]
    fn story_4_item_add_and_room() {
        use piney_battle::item::get_item_num;
        use piney_event::host::Host;
        use piney_world::field_world::Place;
        let Some((mut s, _)) = story_4_first_room() else { return };
        let Stage::Area(a) = &mut s.stage else { panic!("not in the area") };
        let save = a.world().state().save.clone();
        let job = save.i16(offset::SPC_PARAM + offset::SPC_PARAM_SIZE * 2 + 0xd8);
        let cat = (job + 1).rem_euclid(6);
        let (orca, sanjuro) = a.with_host(|h| (h.add_spc_item(2, cat, 3, 1), h.add_spc_item(4, cat, 3, 1)));
        assert!(orca && !sanjuro);
        let now = &a.world().state().save;
        assert_eq!(get_item_num(now, 2, i32::from(cat), 3), get_item_num(&save, 2, i32::from(cat), 3) + 1);
        let Place::Dungeon(d) = a.world().place() else { panic!("not a dungeon") };
        // The last room of the floor (room 1 plays the story's stream 3 as
        // it is entered).
        let rooms = &d.floors[0].rooms;
        let block = (2..rooms.len()).rev().find(|&i| rooms[i].made).unwrap();
        a.with_host(|h| h.room(0, block as i16));
        let Place::Dungeon(d) = a.world().place() else { panic!("not a dungeon") };
        assert_eq!((d.level, d.room_at, d.room_enter), (0, Some((0, block)), true));
        assert_eq!(d.position[3], piney_world::ee::ONE);
        let at = d.position;
        assert_eq!(a.world().scene().block, block as i32);
        assert_eq!(a.ui().ctrl.map_status, 3);
        // The next scene: the leader where RoomSelect put him.
        let mut pad = Pad::default();
        let mut left = false;
        for f in 0..600 {
            pad.read(&Raw::default());
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { continue };
            let playing = matches!(a.world().phase(), piney_world::Phase::Play(_));
            left |= !playing;
            if left && playing {
                assert_eq!(a.world().scene().block, block as i32);
                let p = a.world().player().body.pos;
                assert_eq!((p[0], p[1]), (at[0], at[1]), "frame {f}");
                return;
            }
        }
        panic!("room {block} never came up");
    }

    /// The following camera along the first room's walls: after block 4,
    /// Kite pushed north, east, south and west (`$PINEY_WALK` frames each,
    /// default 150) into the walls and along them; a shot into
    /// `$PINEY_SHOTS` every 15 frames, and every frame the line from his
    /// head to the camera tested against the walls (the camera's own
    /// `ccHitCheckLM` mask) and the camera's height over the floor:
    /// `cargo test --release -p piney-game story_4_wall_shots -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore]
    fn story_4_wall_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        let walk: u64 = std::env::var("PINEY_WALK").ok().and_then(|v| v.parse().ok()).unwrap_or(150);
        let Some((mut s, _)) = story_4_first_room() else { return };
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
        let mut pad = Pad::default();
        let mut behind = 0;
        let q = std::f32::consts::FRAC_PI_2;
        for (k, h) in [0.0f32, q, 2.0 * q, -q].into_iter().enumerate() {
            for n in 0..walk {
                let raw = {
                    let Stage::Area(a) = &s.stage else { panic!("left the area") };
                    let cam_z = f32::from_bits(a.world().camera().rot()[2]);
                    stick_toward(cam_z, h)
                };
                pad.read(&raw);
                let frame = s.step(&pad);
                s.take_events();
                gs.set_overlay(Mode::archive(&s));
                gs.render(&frame);
                let Stage::Area(a) = &s.stage else { panic!("left the area") };
                let w = a.world();
                let piney_world::field_world::Place::Dungeon(d) = w.place() else { panic!("not in the dungeon") };
                let c = w.combat();
                let kp = c.scene.chars[c.kite.unwrap()].pos;
                let cam = w.camera().active().pos;
                let mut head = kp;
                head[2] = (f32::from_bits(kp[2]) + 140.0).to_bits();
                let mut hits = d.hits.clone();
                let cut = hits.line(head, cam, piney_world::hit::CAMERA_MASK, 0).is_some();
                behind += usize::from(cut);
                if n % 15 == 0 || cut {
                    let (kf, cf) = (kp.map(f32::from_bits), cam.map(f32::from_bits));
                    let info = format!(
                        "kite ({:.0},{:.0},{:.0}) cam ({:.0},{:.0},{:.0}) wall between {cut}",
                        kf[0], kf[1], kf[2], cf[0], cf[1], cf[2]
                    );
                    if n % 15 == 0 {
                        let (tw, th) = gs.target_size();
                        let path = format!("{dir}/wall{k}_{n:03}.png");
                        std::fs::write(&path, piney_gs::png::encode(tw, th, &gs.read_back())).unwrap();
                        println!("{path}: {info}");
                    } else {
                        println!("frame {k}/{n}: {info}");
                    }
                }
            }
        }
        println!("frames with a wall between Kite's head and the camera: {behind}");
    }

    /// What a player's pad does at a treasure box `b` in the first room:
    /// walks up until it is the target; with `wire`, PERSONAL, Items, the
    /// Fortune Wire and the box in the target menu, the item's window
    /// closed; then the action button on the box; each menu it opens (32,
    /// 33, 29) taken with the OK button.
    fn at_box(s: &Session, f: u64, b: usize, wire: bool) -> Raw {
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let Stage::Area(a) = &s.stage else { return still };
        let (ui, w) = (a.ui(), a.world());
        let c = &ui.ctrl;
        let press = |b: Buttons| if f.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
        let go_to = |row: usize| match c.list().select.max(0) as usize {
            s if s == row => Buttons::CROSS,
            s if s < row => Buttons::DOWN,
            _ => Buttons::UP,
        };
        if c.item.is_some() {
            return press(Buttons::CROSS);
        }
        match ui.menu_type() {
            -1 => {}
            2 => return press(go_to(c.list().items.iter().position(|&m| m == 5).unwrap_or(0))),
            5 => {
                let list = piney_fieldui::items::item_list(&ui.texts().items, w.state(), 0, i32::from(c.list().page));
                let row = list.iter().position(|it| it.cat == 13 && it.id == 0);
                return press(row.map_or(Buttons::RIGHT, go_to));
            }
            _ => return press(Buttons::CROSS),
        }
        let cm = w.combat();
        let Some(k) = cm.kite else { return still };
        let p = cm.scene.chars[k].pos.map(f32::from_bits);
        let q = cm.scene.chars[b].pos.map(f32::from_bits);
        if w.command_target() != Some(b) {
            let cam_z = f32::from_bits(w.camera().rot()[2]);
            return stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])));
        }
        press(if wire { Buttons::TRIANGLE } else { Buttons::CROSS })
    }

    /// The Fortune Wire used on a booby-trapped box as a player uses it:
    /// PERSONAL, Items, the wire, the box in the target menu
    /// (`ccUseItemRequest`, item 13:0: sound 228, `effRemoveTrap`, "Disarmed
    /// trap." until the button, `EntryAffect(box, 12)`); the box gives way
    /// to its untrapped twin (`param[2]` 3) where it stood, and Kite, who
    /// then opens that with the action button (menu 32), is not hurt: the
    /// wire used up, the box's item in the bag, `itemBoxCount` one up.
    #[test]
    fn the_fortune_wire_disarms_a_trapped_box() {
        use piney_data::save::offset;
        let Some((mut s, mut f, b)) = story_4_with_box(1) else { return };
        let (wires, got, boxes, hp) = {
            let Stage::Area(a) = &s.stage else { panic!() };
            let w = a.world();
            let save = &w.state().save;
            let k = w.combat().kite.unwrap();
            (
                piney_battle::item::get_item_num(save, 0, 13, 0),
                piney_battle::item::get_item_num(save, 0, 10, 1),
                save.i16(offset::ITEM_BOX_COUNT),
                w.combat().scene.chars[k].hp,
            )
        };
        let spot = {
            let Stage::Area(a) = &s.stage else { panic!() };
            a.world().combat().scene.chars[b].pos
        };
        let mut pad = Pad::default();
        let (mut twin, mut hps, mut opened) = (None, Vec::new(), false);
        for _ in 0..3000 {
            f += 1;
            let target = twin.unwrap_or(b);
            let raw = at_box(&s, f, target, twin.is_none());
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let c = a.world().combat();
            hps.push(c.scene.chars[c.kite.unwrap()].hp);
            if twin.is_none() {
                twin = c.ctrl.list(piney_battle::entry::Kind::Gimmick).into_iter().find(|&g| {
                    c.ctrl.entry_obj(g).is_some_and(|o| o.gim_id == 0 && o.ent.param[2] == 3)
                        && c.scene.chars[g].pos == spot
                });
            }
            let save = &a.world().state().save;
            if piney_battle::item::get_item_num(save, 0, 10, 1) > got && a.ui().menu_type() == -1 {
                opened = true;
                break;
            }
        }
        let Stage::Area(a) = &s.stage else { panic!() };
        let calls: Vec<&str> = a.calls().iter().map(|c| c.1.as_str()).collect();
        assert!(calls.contains(&"use_item 0xd0000"), "the wire used: {calls:?}");
        assert!(!calls.iter().any(|c| c.starts_with("item step not carried out")), "{calls:?}");
        assert!(twin.is_some(), "the untrapped twin in the box's place");
        assert!(opened, "the twin opened, its item got");
        assert!(hps.iter().all(|&h| h == hp), "Kite's HP: {hp} then {hps:?}");
        let save = &a.world().state().save;
        assert_eq!(piney_battle::item::get_item_num(save, 0, 13, 0), wires - 1, "the wire used up");
        assert_eq!(save.i16(offset::ITEM_BOX_COUNT), boxes + 1, "itemBoxCount");
    }

    /// A booby-trapped box opened as it is (the action button: menu 33):
    /// trap 0 goes off on Kite (`invokeTrap`: skill 1's damage), "Set off
    /// trap!" with the explosion's line until the button, then the box's
    /// item into the bag.
    #[test]
    fn a_trapped_box_opened_as_it_is_goes_off() {
        use piney_data::save::offset;
        let Some((mut s, mut f, b)) = story_4_with_box(1) else { return };
        let (got, boxes, hp) = {
            let Stage::Area(a) = &s.stage else { panic!() };
            let w = a.world();
            let save = &w.state().save;
            let k = w.combat().kite.unwrap();
            (
                piney_battle::item::get_item_num(save, 0, 10, 1),
                save.i16(offset::ITEM_BOX_COUNT),
                w.combat().scene.chars[k].hp,
            )
        };
        let mut pad = Pad::default();
        let (mut menus, mut low, mut opened) = (Vec::new(), hp, false);
        for _ in 0..3000 {
            f += 1;
            let raw = at_box(&s, f, b, false);
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let c = a.world().combat();
            let k = c.kite.unwrap();
            // The trap's number over Kite, once (#44: it came up twice).
            let lost = i32::from(low.min(hp)) - i32::from(c.scene.chars[k].hp);
            if lost > 0 && low == hp {
                let new = a.world().fx().census().new_fly_fonts;
                let over: Vec<_> = new.iter().filter(|f| f.0 as usize == k).map(|f| f.1.clone()).collect();
                let digits: Vec<u8> = lost.to_string().bytes().map(|c| c - b'0' + 0x21).collect();
                assert_eq!(over, [digits], "the trap's number over Kite");
            }
            low = low.min(c.scene.chars[k].hp);
            // 88: a menu changing to the next.
            let m = a.ui().menu_type();
            if m != -1 && m != 88 && menus.last() != Some(&m) {
                menus.push(m);
            }
            if piney_battle::item::get_item_num(&a.world().state().save, 0, 10, 1) > got && m == -1 {
                opened = true;
                break;
            }
        }
        assert!(opened, "the box's item got: menus {menus:?}");
        assert_eq!(menus, [33, 29], "the trapped box's menu, then the item's");
        assert!(low < hp, "the trap went off on Kite: {hp} -> {low}");
        let Stage::Area(a) = &s.stage else { panic!() };
        assert_eq!(a.world().state().save.i16(offset::ITEM_BOX_COUNT), boxes + 1, "itemBoxCount");
    }

    /// `--mode story:4` played by [`play_dungeon`]'s legs: prints every
    /// event call with its room, then the host defaults that ran:
    /// `cargo test --release -p piney-game story_4_log -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore]
    fn story_4_log() {
        let Some(mut s) = story_session(4) else { return };
        let _ = piney_event::host::take_unported();
        let mut f = 0u64;
        let (mut seen, mut mode) = (0, 0usize);
        let (mut printed, mut count) = ((-1, -1), 0usize);
        let legs = play_dungeon(&mut s, &mut f, &STORY_4_LEGS, 9000, |s, _| {
            let Stage::Area(a) = &s.stage else { return };
            let sc = a.world().scene();
            let here = &**a as *const AreaMode as usize;
            if here != mode || a.calls().len() < seen {
                (mode, seen) = (here, 0);
            }
            let c = a.world().combat();
            let n = c.ctrl.list(piney_battle::entry::Kind::Gimmick).len()
                + c.ctrl.list(piney_battle::entry::Kind::Circle).len();
            if c.started && (printed != (sc.floor, sc.block) || n != count) {
                (printed, count) = ((sc.floor, sc.block), n);
                for k in [piney_battle::entry::Kind::Circle, piney_battle::entry::Kind::Gimmick] {
                    for i in c.ctrl.list(k) {
                        let o = c.ctrl.entry_obj(i).unwrap();
                        let p = c.scene.chars[i].pos.map(f32::from_bits);
                        println!(
                            "      ({},{}) ctrl ({},{}) {:?} {i} gim {} at ({},{}) f{} b{} on {} act {} item {:x} trap {} listed {}",
                            sc.floor,
                            sc.block,
                            c.ctrl.floor,
                            c.ctrl.block,
                            k,
                            o.gim_id,
                            p[0],
                            p[1],
                            o.ent.floor,
                            o.ent.block,
                            o.obj_flag,
                            o.act_num,
                            o.ent.param[1],
                            o.ent.param[2],
                            c.scene.listed(i)
                        );
                    }
                }
            }
            for (fr, c) in &a.calls()[seen..] {
                println!("{fr:5} ({},{}) {c}", sc.floor, sc.block);
            }
            seen = a.calls().len();
        });
        println!("legs {legs} of {}; {}", STORY_4_LEGS.len(), Mode::title(&s));
        println!("unported: {:?}", piney_event::host::take_unported());
    }

    /// `--mode story:4` played by [`play_dungeon`], with shots into
    /// `$PINEY_SHOTS`: one at each mark of `$PINEY_MARKS` (`CALL,DELAY,NAME`
    /// by `;`; or `box`, `rays`, `idol`), and every other frame around each
    /// room change of `$PINEY_RUNS` (numbers from 0). `cargo test --release
    /// -p piney-game story_4_shots -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn story_4_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        let Some(mut s) = story_session(4) else { return };
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
        let marks: Vec<(String, u64, String)> = std::env::var("PINEY_MARKS")
            .unwrap_or_else(|_| {
                "box,20,boxes;open_menu 84,30,box_open;open_menu 84,45,box_rays;remove_trap,4,trap_wire;\
                 remove_trap,21,trap_disarmed;open_menu 85,30,box2_open;idol,10,statue_idol;\
                 message_open 4 9,30,statue"
                    .into()
            })
            .split(';')
            .filter_map(|m| {
                let mut it = m.split(',');
                Some((it.next()?.trim().to_string(), it.next()?.trim().parse().ok()?, it.next()?.trim().to_string()))
            })
            .collect();
        let runs: Vec<usize> = std::env::var("PINEY_RUNS")
            .unwrap_or_else(|_| "0,3,5".into())
            .split(',')
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        let mut due: Vec<Option<u64>> = vec![None; marks.len()];
        let mut done = vec![false; marks.len()];
        let mut n = 0u64;
        let mut room = (0, 0);
        let mut changes = 0usize;
        type Shot = (u64, (u32, u32), Vec<u8>, String);
        let mut ring: std::collections::VecDeque<Shot> = Default::default();
        let mut run_left = 0u32;
        let mut run_name = String::new();
        let mut f = 0u64;
        // `$PINEY_LEGS`: d<N> a door, s the stairs, p the portal, o a box
        // or idol opened.
        let legs: Vec<Leg> = std::env::var("PINEY_LEGS").map_or(STORY_4_LEGS.to_vec(), |v| {
            v.split(',')
                .filter_map(|l| match l.trim() {
                    "s" => Some(Leg::Stairs),
                    "p" => Some(Leg::Portal),
                    "o" => Some(Leg::Open),
                    d => d.strip_prefix('d').and_then(|n| n.parse().ok()).map(Leg::Door),
                })
                .collect()
        });
        play_dungeon(&mut s, &mut f, &legs, 9000, |s, frame| {
            n += 1;
            gs.set_overlay(Mode::archive(s));
            gs.render(frame);
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let sc = w.scene();
            let c = w.combat();
            let kite = c.kite.map_or([0.0; 4], |k| c.scene.chars[k].pos.map(f32::from_bits));
            let cam = w.camera().active().pos.map(f32::from_bits);
            let info = format!(
                "room ({},{}) kite ({:.0},{:.0},{:.0}) dirc {:.3} player {:.3} cam ({:.0},{:.0},{:.0}) rot {:.3} {:?} {:?}",
                sc.floor,
                sc.block,
                kite[0],
                kite[1],
                kite[2],
                f32::from_bits(c.kite_dirc()[2]),
                f32::from_bits(w.player().body.dirc[2]),
                cam[0],
                cam[1],
                cam[2],
                f32::from_bits(w.camera().rot()[2]),
                w.phase(),
                Mode::title(s)
            );
            let here = (sc.floor, sc.block);
            let save = |name: &str, gs: &piney_gs::Gs, info: &str| {
                let (w, h) = gs.target_size();
                let path = format!("{dir}/{name}.png");
                std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
                println!("{path}: {info}");
            };
            if sc.area == 2 && here != room {
                if runs.contains(&changes) {
                    run_name = format!("run{changes}_{}{}_{}{}", room.0, room.1, here.0, here.1);
                    for (k, sz, px, inf) in ring.drain(..) {
                        let path = format!("{dir}/{run_name}_m{:02}.png", n - k);
                        std::fs::write(&path, piney_gs::png::encode(sz.0, sz.1, &px)).unwrap();
                        println!("{path}: {inf}");
                    }
                    run_left = 30;
                }
                changes += 1;
                room = here;
            }
            if run_left > 0 {
                if run_left.is_multiple_of(2) {
                    save(&format!("{run_name}_p{:02}", 30 - run_left), &gs, &info);
                }
                run_left -= 1;
            } else if n.is_multiple_of(2) && runs.contains(&changes) {
                ring.push_back((n, gs.target_size(), gs.read_back(), info.clone()));
                if ring.len() > 3 {
                    ring.pop_front();
                }
            }
            let gims: Vec<(i32, bool, i32, Option<bool>)> = c
                .ctrl
                .list(piney_battle::entry::Kind::Gimmick)
                .into_iter()
                .filter_map(|i| {
                    c.ctrl.entry_obj(i).map(|o| (o.gim_id, o.obj_flag, o.act_num, c.cast.get(i).map(|a| a.drawn)))
                })
                .collect();
            for (k, (call, after, name)) in marks.iter().enumerate() {
                if done[k] {
                    continue;
                }
                let fired = match call.as_str() {
                    "box" => gims.iter().filter(|g| g.1 && g.0 <= 1).count() == 2,
                    "rays" => !c.rays.is_empty(),
                    "idol" => gims.iter().any(|g| g.1 && g.0 >= 38),
                    c if c.starts_with("room ") => c == format!("room {} {}", here.0, here.1),
                    c if c.starts_with("change ") => c == format!("change {changes}"),
                    // CALL#N: the call's Nth time in the area.
                    c if c.contains('#') => {
                        let (call, k) = c.split_once('#').unwrap();
                        let k: usize = k.parse().unwrap_or(1);
                        a.calls().iter().filter(|(_, x)| x.starts_with(call)).count() >= k
                    }
                    c => a.calls().iter().any(|(_, x)| x.starts_with(c)),
                };
                if due[k].is_none() && fired {
                    due[k] = Some(n + after);
                }
                if due[k] == Some(n) {
                    save(name, &gs, &format!("{info} gimmicks {gims:?} rays {}", c.rays.len()));
                    done[k] = true;
                }
            }
        });
    }

    /// Event 25 in story area 23's dungeon: the Aura shrine.
    mod shrine;

    /// Issue #12: the boxes' bodies in story area 18's dungeon.
    mod boxes;

    /// The dungeon rooms' dressing in Δ and Θ random dungeons.
    mod dressing;

    /// A generated field's gimmicks (`WORLD_MAN::EntryGimmick`).
    mod field_gims;

    /// The chat balloons: Mac Anu's walking players.
    mod chat;

    /// A survey of the main events' starts (ignored; a diagnostic).
    mod survey;

    /// A survey of the side events where they play (ignored; a diagnostic).
    mod side_events;

    /// The play time the main loop counts.
    mod play_time;

    /// Back in a town from a field: faces, kit and trade lists.
    mod town_return;

    /// Issue #16: members who leave by an event's `pc_act 5`.
    mod party_leave;

    /// Issues #14 and #15: a member felled and revived, the markers.
    mod revive;

    /// Issue #41: an enemy revives a dying ally.
    mod enemy_revive;

    /// A foe's condition effect ended where the game clears it.
    mod cond_fx;

    /// A spell or art holds its targets where they stand.
    mod spell_hold;

    /// Issue #48: the tornados' hits, the system's and the element's.
    mod wood_tornado;

    /// A dungeon portal's group: ccInitRand's draws.
    mod portal_draws;

    // Playthroughs: the story's scripts (run by piney-event's VM, as every
    // event is) played from a start point with a scripted pad, checked.

    /// Event 11 in Mac Anu as a player plays it.
    mod event11;

    /// The gate hack in Mac Anu as a player plays it.
    mod gate_hack;

    /// Dun Loireag through the Chaos Gate and the story.
    mod dun_loireag;

    /// Any Root Town's shots from its start (ignored; a diagnostic).
    mod town_shots;

    /// Event 22 in Mac Anu: Piros's colour.
    mod event22;

    /// Skeith in its arena.
    mod skeith;

    /// Issue #40: the bracelet shines in Chosen Hopeless Nothingness.
    mod bracelet;

    /// Fidchell in its arena: its spells' pictures, voice and names.
    mod fidchell;

    /// The ending's save menus after the staff roll.
    mod ending_save;

    /// The riding Grunty: the flute, the ride, the dismount.
    mod ride;

    /// Items used in a field: a Fairy's Orb's portals, a Speed Charm on Kite.
    mod fairy_orb;

    /// Mutation's area 43's story map (`EVENTAREA03`).
    mod area43;
    /// The enemies' weapon trails: a goblin's swing.
    mod weapon;

    /// A Data Bug in a field: its two models, its HP held at half.
    mod data_bug;

    /// Issue #51: out of a dungeon by the Sprite Ocarina or the stairs.
    mod ocarina;

    /// Issue #49: a foe's immunity in its target window.
    mod tolerance;

    /// Issue #50: Ryu Book III's "Online" for the people in town.
    mod ryu_book_online;

    /// A story start's session (`--mode story:N`), its event task recording
    /// the blocks it plays. None without the disc.
    fn story_session(n: i32) -> Option<Session> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return None;
        }
        story_session_with(n, |_| {})
    }

    /// [`story_session`], `prepare` making changes to the start first.
    fn story_session_with(n: i32, prepare: impl FnOnce(&mut crate::start::Start)) -> Option<Session> {
        story_session_on("infection", n, prepare)
    }

    /// [`story_session_with`] on the disc `disc` (`work/DISC/DISC.iso`).
    fn story_session_on(disc: &str, n: i32, prepare: impl FnOnce(&mut crate::start::Start)) -> Option<Session> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        if !iso.exists() {
            return None;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut start = crate::start::build(&iso, n).unwrap();
        start.vm.trace = Some(Vec::new());
        prepare(&mut start);
        Some(Session::resume(iso, archive, None, start.state, start.vm, start.at).unwrap())
    }

    /// The event task, out of the mode it was in (which is left).
    fn take_vm(s: &mut Session) -> Option<Vm> {
        match std::mem::replace(&mut s.stage, Stage::Gone) {
            Stage::Desktop(d) => d.leave().1,
            Stage::TopPage(t) => t.leave().1,
            Stage::World(w) => w.leave().1,
            Stage::Area(a) => a.leave().1,
            _ => None,
        }
    }

    /// The blocks the task played, (event, block) in order.
    fn blocks(vm: &Vm) -> Vec<(i32, i32)> {
        use piney_event::vm::Trace;
        vm.trace
            .iter()
            .flatten()
            .filter_map(|t| match *t {
                Trace::Block { event, block, .. } => Some((event, block)),
                Trace::Pass { .. } => None,
            })
            .collect()
    }

    /// story:3 against event 2 played (from a new game's Log in, the town's
    /// tutorials walked, to its `scene`), as each arrives in area 14's
    /// field: the scene, `WORLD_MAN`'s area and the party managers; then,
    /// 60 frames on (the set-up's passes made, event 3 open), the flags of
    /// events 0-4 and the gate, word and member bits.
    #[test]
    fn story_3_is_where_event_2_leaves_the_game() {
        use piney_data::save::offset;
        use piney_event::state::ScriptSave as _;
        let Some((mut played, _)) = story_to_field(0) else { return };
        let Some(mut start) = story_session(3) else { return };
        {
            let (Stage::Area(p), Stage::Area(s)) = (&played.stage, &start.stage) else { panic!("not in the field") };
            let (pw, sw) = (p.world(), s.world());
            // The clocks differ: one path played the town, the other did not.
            let clockless = |mut sc: piney_world::area::Scene| {
                sc.game_cnt = [0; 4];
                sc
            };
            assert_eq!(clockless(pw.scene()), clockless(sw.scene()));
            assert_eq!(pw.world_man(), sw.world_man());
            let (ps, ss) = (pw.spcs(), sw.spcs());
            assert_eq!((ps.registry, ps.registry_num), (ss.registry, ss.registry_num));
            assert_eq!((ps.member_id, ps.member_char, ps.num), (ss.member_id, ss.member_char, ss.num));
        }
        let mut pad = Pad::default();
        run(&mut played, &mut pad, 0..60, &[]);
        run(&mut start, &mut pad, 0..60, &[]);
        let (Stage::Area(p), Stage::Area(s)) = (&played.stage, &start.stage) else { panic!("left the field") };
        let (ps, ss) = (&p.world().state().save, &s.world().state().save);
        for e in 0..=4 {
            assert_eq!(ps.flags(e), ss.flags(e), "event {e}'s flags");
        }
        for (at, len) in
            [(offset::GATE_LIST, offset::WORD_LIST + 60 - offset::GATE_LIST), (offset::PARTY_MEMBER_FLAG, 16)]
        {
            assert_eq!(&ps.bytes()[at..at + len], &ss.bytes()[at..at + len], "save +0x{at:04x}");
        }
    }

    /// An area's last magic portal opened (the entry control's
    /// `ccStartThread(ccThDfComp)`) puts up "ALL FIELD PORTALS OPEN" on
    /// the menu's layer: its 90 frames, then gone.
    #[test]
    fn the_last_portal_puts_up_its_banner() {
        let Some(mut s) = story_session(3) else { return };
        let mut pad = Pad::default();
        run(&mut s, &mut pad, 0..60, &[]);
        let Stage::Area(a) = &mut s.stage else { panic!("not in the field: {}", Mode::title(&s)) };
        assert!(!a.ui().df_comp_on());
        let shows = &mut a.world_mut().combat_mut().shows;
        shows.push(piney_world::combat::Show::Entry(piney_battle::entry::Out::AreaCleared));
        let mut on = 0;
        for _ in 0..120 {
            press(&mut s, &mut pad, Buttons::NONE);
            let Stage::Area(a) = &s.stage else { break };
            on += u32::from(a.ui().df_comp_on());
        }
        // Started by the entry control (64) after the menu task, the banner
        // task makes its first Main the next frame: on from that frame's
        // end, then its 90 Mains.
        assert_eq!(on, 91, "the banner's frames");
    }

    /// Pictures of [`the_last_portal_puts_up_its_banner`]'s banner: frames
    /// 5 (sliding in), 40 (held) and 88 (parting) to
    /// `$PINEY_SHOTS/dfcomp-NN.png`.
    #[test]
    #[ignore]
    fn last_portal_banner_shots() {
        let Ok(dir) = std::env::var("PINEY_SHOTS") else { return };
        let Some(mut s) = story_session(3) else { return };
        let mut pad = Pad::default();
        run(&mut s, &mut pad, 0..60, &[]);
        let Stage::Area(a) = &mut s.stage else { panic!() };
        let shows = &mut a.world_mut().combat_mut().shows;
        shows.push(piney_world::combat::Show::Entry(piney_battle::entry::Out::AreaCleared));
        std::fs::create_dir_all(&dir).unwrap();
        let mut gs: Option<piney_gs::Gs> = None;
        for n in 1..=90u32 {
            pad.read(&Raw::default());
            let frame = s.step(&pad);
            s.take_events();
            if ![5, 40, 88].contains(&n) {
                continue;
            }
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(&s));
            g.render(&frame);
            let (w, h) = g.target_size();
            let path = format!("{dir}/dfcomp-{n:02}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
            println!("{path}");
        }
    }

    #[test]
    fn a_dungeon_start_sets_the_voice_language() {
        let Some(mut s) = story_session(4) else { return };
        let mut pad = Pad::default();
        let mut seen = Vec::new();
        for _ in 0..30 {
            seen.extend(press(&mut s, &mut pad, Buttons::NONE));
        }
        let voice: Vec<_> = seen.iter().filter(|e| matches!(e, Event::VoiceOptions { .. })).collect();
        assert!(!voice.is_empty(), "no voice options in the first 30 frames");
        assert!(voice.iter().all(|e| matches!(e, Event::VoiceOptions { english: true, .. })), "{voice:?}");
    }

    #[test]
    fn story_starts_open_their_event() {
        starts_open_their_event("infection");
    }

    #[test]
    fn mutation_story_starts_open_their_event() {
        starts_open_their_event("mutation");
    }

    #[test]
    fn outbreak_story_starts_open_their_event() {
        starts_open_their_event("outbreak");
    }

    /// Each start point of the disc's story opens its event where it
    /// should, without the story before it playing again.
    fn starts_open_their_event(disc: &str) {
        use crate::start::{POINTS, Place};
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        let Some(volume) = Iso::open(&iso).ok().and_then(|mut d| d.volume().ok()) else { return };
        let story = crate::start::story(volume);
        let mut checked = 0;
        for n in POINTS.into_iter().filter(|n| story.contains(n)) {
            let Some(mut s) = story_session_on(disc, n, |_| {}) else { return };
            checked += 1;
            let mut pad = Pad::default();
            run(&mut s, &mut pad, 0..120, &[]);
            let place = crate::start::place(n).unwrap();
            let what = Mode::title(&s);
            match (&s.stage, place) {
                (Stage::Desktop(_), Place::Desktop) | (Stage::TopPage(_), Place::Board) => {}
                (Stage::World(w), Place::Town) => assert_eq!(w.world().party(), [0, -1, -1], "story:{n}"),
                (Stage::Area(a), Place::Field | Place::Dungeon) => {
                    let sc = a.world().scene();
                    let area = if place == Place::Field { 1 } else { 2 };
                    assert_eq!((sc.area, sc.field), (area, 14), "story:{n}");
                    assert_eq!(a.world().party(), [0, 2, -1], "story:{n}");
                    assert!(a.world().char_pos(2, 2).is_some(), "story:{n}: Orca is not there");
                }
                _ => panic!("story:{n} is not where event {n} opens: {what}"),
            }
            let vm = take_vm(&mut s).unwrap();
            let played = blocks(&vm);
            assert!(played.contains(&(n, 0)), "story:{n}: block 0 did not play: {played:?} ({what})");
            let before: Vec<_> = played.iter().filter(|(e, _)| story.contains(e) && *e < n).collect();
            assert!(before.is_empty(), "story:{n}: the story before it played again: {before:?}");
        }
        assert_eq!(checked, story.iter().filter(|n| POINTS.contains(n)).count(), "{disc}: starts checked");
        eprintln!("{disc}: {checked} starts open their events");
    }

    /// Event 4's end (E4-7, E4-8, E4-9): its `mode 3` from the dungeon
    /// (+0x02a2) is `ChangeRequest(3, 7)`, which the session takes to the
    /// desktop; the setup keeps The World's frame rate 2, so block 9's
    /// streams 4, 5 and 6 play at 30 frames a second; its own `mode 3`
    /// (+0x02d8) abandons that setup and sets the desktop up again on the
    /// same task, where event 4 and event 3 are done and event 10 opens -
    /// its windows still at rate 2 - and `SetFrameRate(1)` comes after the
    /// pass at phase 2.
    #[test]
    fn event_4_ends_on_the_desktop() {
        use piney_event::state::{DONE, ScriptSave as _};
        // Event 4 in the dungeon up to block 8's `mode 3`: blocks 2-8 played
        // by the task's own `Execute`, every wait over at once (block 7
        // sets `eventStatus[0]` = 1, which block 9 is set to need).
        let Some(mut s) = story_session_with(4, |start| {
            let mut h = piney_event::host::LogHost::new(start.state.save.clone());
            h.game.status = 5;
            for b in 2..=8 {
                start.vm.play_block(4, b, &mut h);
            }
            start.state.save = h.save;
        }) else {
            return;
        };
        let mut pad = Pad::default();
        // In the dungeon's fade out, the task idle, as block 8's `mode 3`
        // leaves it (the instruction ends its pass).
        run(&mut s, &mut pad, 0..1, &[]);
        assert!(matches!(s.stage, Stage::Area(_)), "{}", Mode::title(&s));
        assert_eq!(Mode::frame_rate(&s), 2);
        // Block 8's `mode 3`, as the area's host hands it on.
        assert_eq!(s.world_events(vec![Event::ChangeMode { num: 3, sf: 7 }]), Some(3));
        s.change(request::DESKTOP).unwrap();
        assert!(matches!(&s.stage, Stage::Desktop(d) if !d.playing()), "{}", Mode::title(&s));
        let stream_of = |s: &Session| {
            let t = Mode::title(s);
            t.split(" - stream ").nth(1).and_then(|r| r.split(' ').next()).and_then(|k| k.parse::<i32>().ok())
        };
        // Each stream: 60 frames at rate 2, then skipped.
        let (mut seen, mut n, mut streamed) = (Vec::new(), 0, 0);
        while seen.len() < 3 || stream_of(&s).is_some() {
            let playing = stream_of(&s);
            if let Some(k) = playing {
                if seen.last() != Some(&k) {
                    seen.push(k);
                    streamed = 0;
                }
                assert_eq!(Mode::frame_rate(&s), 2, "stream {k}");
                streamed += 1;
            }
            let skip = playing.is_some() && streamed % 60 == 0;
            press(&mut s, &mut pad, if skip { Buttons::CIRCLE | Buttons::START } else { Buttons::NONE });
            n += 1;
            assert!(n < 2000, "streams {seen:?}: {}", Mode::title(&s));
        }
        assert_eq!(seen, [4, 5, 6]);
        // The second setup: event 10's lines at rate 2, then the desktop at 1.
        let mut rates = Vec::new();
        while !matches!(&s.stage, Stage::Desktop(d) if d.playing()) {
            rates.push(Mode::frame_rate(&s));
            let buttons = setup_press(&s, n);
            press(&mut s, &mut pad, buttons);
            n += 1;
            assert!(n < 5000, "the second setup does not end: {}", Mode::title(&s));
        }
        assert!(rates.len() > 60 && rates.iter().all(|&r| r == 2), "the second setup's rates {rates:?}");
        assert_eq!(Mode::frame_rate(&s), 1);
        let Stage::Desktop(d) = std::mem::replace(&mut s.stage, Stage::Gone) else { unreachable!() };
        let (state, vm) = d.leave();
        let played = blocks(vm.as_ref().unwrap());
        assert!(played.contains(&(4, 9)) && played.contains(&(10, 0)), "{played:?}");
        for e in [3, 4] {
            assert_ne!(state.save.flags(e) & DONE, 0, "event {e} done");
        }
        // Block 0's mail 6 and key item 59, The Twilight.
        assert_ne!(state.save.mail(6), 0);
    }

    /// Event 4's streams (Skeith's coming, the drain, the fade) timed as
    /// the window plays them: each session step and its draw on the GPU,
    /// the slowest printed: `cargo test --release -p piney-game
    /// event_4_stream_times -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_4_stream_times() {
        let Some(mut s) = story_session_with(4, |start| {
            let mut h = piney_event::host::LogHost::new(start.state.save.clone());
            h.game.status = 5;
            for b in 2..=8 {
                start.vm.play_block(4, b, &mut h);
            }
            start.state.save = h.save;
        }) else {
            return;
        };
        let mut pad = Pad::default();
        run(&mut s, &mut pad, 0..1, &[]);
        let t = std::time::Instant::now();
        s.world_events(vec![Event::ChangeMode { num: 3, sf: 7 }]);
        s.change(request::DESKTOP).unwrap();
        println!("to the desktop: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
        // The sound as the window runs it: the events handed on, a frame of
        // the driver, a 30th of a second rendered.
        let audio = piney_audio::Audio::headless("../../work/infection/infection.iso").ok();
        let mut buf = vec![0i16; 3200];
        let mut times = Vec::new();
        for n in 0..6000u32 {
            let title = Mode::title(&s);
            if n > 100 && !title.contains(" - stream ") {
                break;
            }
            pad.read(&Raw::default());
            let t = std::time::Instant::now();
            let frame = s.step(&pad);
            crate::handle(s.take_events(), audio.as_ref());
            if let Some(a) = &audio {
                a.frame();
                a.render(&mut buf);
            }
            let step = t.elapsed().as_secs_f64() * 1e3;
            let t = std::time::Instant::now();
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let _ = gs.read_back();
            let draw = t.elapsed().as_secs_f64() * 1e3;
            times.push((n, step, draw, title));
        }
        let total: f64 = times.iter().map(|t| t.1 + t.2).sum();
        println!("{} frames, {:.0} ms", times.len(), total);
        times.sort_by(|a, b| (b.1 + b.2).total_cmp(&(a.1 + a.2)));
        for (n, step, draw, title) in times.iter().take(16) {
            println!("{n:5} step {step:8.2} draw {draw:8.2}  {title}");
        }
    }

    /// Event 21 in Mac Anu, block 4: Elk (placed at marker 3 by block 3)
    /// walks 300 toward Kite (`pc_walk_dir 10 6144 30`) under his AI's
    /// remote control - a town's remote walk, which the battle's
    /// `ManualControl` runs for a member - and the call is done.
    #[test]
    fn event_21_elk_walks_up_in_mac_anu() {
        let Some(mut s) = story_session(21) else { return };
        let mut pad = Pad::default();
        let (mut before, mut walked) = (None, None);
        for f in 0..6000u64 {
            let raw = story_player(&s, f);
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            let Stage::World(w) = &s.stage else { continue };
            let elk = w.world().town_party().rec(10).map(|r| r.pos);
            if let Some((_, c)) = w.calls().iter().rev().find(|(_, c)| c.starts_with("pc WalkDir")) {
                assert!(!c.contains("not done"), "{c}");
                if before.is_none() {
                    before = elk;
                }
                walked = elk;
                if f > 0 && w.calls().iter().any(|(_, c)| c.starts_with("gate_add_msg 21")) {
                    break;
                }
            }
        }
        let (Some(a), Some(b)) = (before, walked) else { panic!("block 4 never walked Elk: {}", Mode::title(&s)) };
        let moved = ((f32::from_bits(b[0]) - f32::from_bits(a[0])).powi(2)
            + (f32::from_bits(b[1]) - f32::from_bits(a[1])).powi(2))
        .sqrt();
        assert!(moved > 100.0, "Elk moved {moved}");
    }

    /// `mode 3` from Mac Anu (The World's `ChangeRequest(3, 7)`): the
    /// desktop, its setup at The World's rate until `SetFrameRate(1)`.
    #[test]
    fn mode_3_from_the_town() {
        let Some(mut s) = story_session(11) else { return };
        let mut pad = Pad::default();
        run(&mut s, &mut pad, 0..30, &[]);
        assert!(matches!(s.stage, Stage::World(_)), "{}", Mode::title(&s));
        s.change(request::DESKTOP).unwrap();
        assert_eq!(Mode::frame_rate(&s), 2);
        settle(&mut s, &mut pad);
        assert_eq!(Mode::frame_rate(&s), 1);
    }

    /// Event 3's camera lesson moves the camera, not only the count: while
    /// `teach_camera1`'s prompt is up L1 turns it, while `teach_camera2`'s
    /// the right stick zooms it, and `teach_camera3`'s R2 turns it back
    /// behind Kite (cpCtrl 5, 6, 7 on the event camera; scheme A-1).
    #[test]
    fn the_camera_lesson_moves_the_camera() {
        // (prompt, the camera's heading and distance from its look-at point)
        let mut seen: Vec<(String, f32, f32)> = Vec::new();
        let r = story_to_field_with(1000, |s| {
            let Stage::Area(a) = &s.stage else { return };
            let prompt = a
                .calls()
                .iter()
                .rev()
                .find_map(|(_, c)| {
                    if c.starts_with("message_open") {
                        Some(c.clone())
                    } else if c.starts_with("message_close") || c.starts_with("message_check") {
                        Some(String::new())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let c = a.world().camera().active();
            let d: Vec<f32> = (0..3).map(|i| f32::from_bits(c.pos[i]) - f32::from_bits(c.view[i])).collect();
            seen.push((prompt, d[1].atan2(d[0]), (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()));
        });
        if r.is_none() {
            return;
        }
        let during = |p: &str| -> Vec<(f32, f32)> {
            seen.iter().filter(|(c, _, _)| c.starts_with(p)).map(|&(_, h, d)| (h, d)).collect()
        };
        let turn = during("message_open 3 55 ");
        assert!(turn.len() > 100, "teach_camera1 not reached ({} frames)", turn.len());
        let (h0, h1) = (turn[0].0, turn[turn.len() - 1].0);
        assert!((h1 - h0).abs() > 0.5, "L1 did not turn the camera: {h0} -> {h1}");
        let zoom = during("message_open 3 56 ");
        assert!(zoom.len() > 100, "teach_camera2 not reached");
        let (d0, d1) = (zoom[0].1, zoom[zoom.len() - 1].1);
        assert!((d1 - d0).abs() > 50.0, "the stick did not zoom the camera: {d0} -> {d1}");
        // The reset: from the press on, the heading goes back.
        let reset = during("message_open 3 57 ");
        assert!(!reset.is_empty(), "teach_camera3 not reached");
        let (r0, r1) = (reset[0].0, reset[reset.len() - 1].0);
        assert!((r1 - r0).abs() > 0.3, "R2 did not reset the camera: {r0} -> {r1}");
    }
}

/// Area 15's story map through the runtime (`piney_world::evarea`):
/// arriving from the Chaos Gate's words and from a start in the field, the
/// church's door in and out, each a change of scene the session makes.
#[cfg(test)]
mod area15 {
    use std::path::{Path, PathBuf};

    use piney_input::{Buttons, Raw};

    use super::*;

    pub(super) fn disc() -> Option<(PathBuf, Arc<Archive>)> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return None;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        Some((iso, archive))
    }

    /// A new game's save in `scene`, without the event task, as `--mode`
    /// starts it.
    pub(super) fn start(iso: &Path, archive: &Arc<Archive>, field: Option<i32>) -> Session {
        let mut d = Iso::open(iso).unwrap();
        let mut state = crate::world::new_game_state(&mut d).unwrap();
        let mut scene = piney_world::area::Scene::log_in(&mut state.save);
        let mut wm = None;
        if let Some(n) = field {
            wm = Some(crate::area::story_world_man(&mut d, n, false).unwrap());
            scene.change_scene(1, scene.town, n, -1, -1, -1, &mut state.save);
        }
        Session::in_world(iso.to_path_buf(), archive.clone(), None, state, None, scene, wm).unwrap()
    }

    /// The area mode's world once its fade in has begun.
    pub(super) fn playing(s: &Session) -> Option<&piney_world::field_world::FieldWorld> {
        match &s.stage {
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(f) if f >= 2) => Some(a.world()),
            _ => None,
        }
    }

    /// A new area mode's world set up, before its tasks' first frame (the
    /// scene before plays on under its fade out, the change already made).
    fn arrived(s: &Session) -> Option<&piney_world::field_world::FieldWorld> {
        match &s.stage {
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(0)) => Some(a.world()),
            _ => None,
        }
    }

    /// Frames with the left stick at (lx, ly) until `done`; the sequence
    /// banks the frames asked for.
    pub(super) fn hold(
        s: &mut Session,
        lx: u8,
        ly: u8,
        max: u32,
        done: impl Fn(&Session) -> bool,
    ) -> Vec<piney_audio::SqContext> {
        let mut pad = Pad::default();
        let mut banks = Vec::new();
        for _ in 0..max {
            if done(s) {
                return banks;
            }
            pad.read(&Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            for e in s.take_events() {
                if let Event::SqLoad(c) = e {
                    banks.push(c);
                }
            }
        }
        panic!("not reached in {max} frames: {}", Mode::title(s));
    }

    /// `n` frames with the left stick at (lx, ly): the last one's picture.
    pub(super) fn hold_frame(s: &mut Session, lx: u8, ly: u8, n: u32) -> Frame {
        let mut pad = Pad::default();
        let mut last = Frame::new();
        for _ in 0..n {
            pad.read(&Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() });
            last = s.step(&pad);
            s.take_events();
        }
        last
    }

    /// Whether Kite stands within a unit of (x, y, z).
    fn at(w: &piney_world::field_world::FieldWorld, want: [f32; 3]) -> bool {
        let p = w.player().body.pos.map(f32::from_bits);
        (0..3).all(|k| (p[k] - want[k]).abs() < 1.0)
    }

    /// `n` frames with the pad at rest.
    pub(super) fn wait(s: &mut Session, n: u32) {
        let mut pad = Pad::default();
        for _ in 0..n {
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
    }

    /// `--mode field:15`: the holy ground at DMY_marker01; up the bridge
    /// into the church's door (block 1, Kite standing at DMY_marker01_2),
    /// and back out of it (block 0, at DMY_marker02), the story map kept
    /// through both set-ups.
    #[test]
    fn the_church_door_in_and_out() {
        let Some((iso, archive)) = disc() else { return };
        let mut s = start(&iso, &archive, Some(15));
        let banks = hold(&mut s, 128, 128, 200, |s| playing(s).is_some());
        // ccSndSQLoad(5): the story map's event bank.
        assert!(matches!(banks[..], [piney_audio::SqContext::Event { field: 15, .. }]), "{banks:?}");
        let w = playing(&s).unwrap();
        assert!(w.place().story::<piney_world::evarea::EventArea>().is_some_and(|e| e.block == 0));
        assert_eq!((w.scene().field, w.scene().block), (15, -1));
        assert!(at(w, [0.0, -3600.0, 0.0]), "{}", Mode::title(&s));
        // Up the bridge: Enter on the door, the scene's block 1, a new set-up.
        hold(&mut s, 128, 128, 120, |s| playing(s).is_some_and(|w| w.player().acts.act == 2));
        let banks = hold(&mut s, 128, 0, 1500, |s| arrived(s).is_some_and(|w| w.scene().block == 1));
        assert!(banks.is_empty(), "the same scene: no bank loaded");
        let w = arrived(&s).unwrap();
        assert!(w.place().story::<piney_world::evarea::EventArea>().is_some_and(|e| e.block == 1 && e.rev.is_some()));
        assert!(at(w, [0.0, 900.0, 0.0]), "{}", Mode::title(&s));
        assert_eq!(w.player().acts.act, 2, "through the door he stands at once");
        assert_eq!(w.scene().area_prev, 1);
        // Back down the nave and out.
        hold(&mut s, 128, 255, 1500, |s| arrived(s).is_some_and(|w| w.scene().block == 0));
        let w = arrived(&s).unwrap();
        assert!(w.place().story::<piney_world::evarea::EventArea>().is_some_and(|e| e.block == 0));
        assert!(at(w, [0.0, 2700.0, 200.0]), "{}", Mode::title(&s));
    }

    /// `--mode field:16`: area 16's own map, `EVENTAREA07`, at
    /// DMY_marker01; up the giant's arm into its doorway, `Enter`'s
    /// `ChangeArea(2, 0)` into its dungeon; back out (`GoField`), a new map
    /// that stands the party at DMY_marker02, the dungeon's side.
    #[test]
    fn the_giants_door_to_its_dungeon_and_back() {
        let Some((iso, archive)) = disc() else { return };
        let mut s = start(&iso, &archive, Some(16));
        let banks = hold(&mut s, 128, 128, 200, |s| playing(s).is_some());
        assert!(matches!(banks[..], [piney_audio::SqContext::Event { field: 16, .. }]), "{banks:?}");
        let w = playing(&s).unwrap();
        assert!(
            w.place().story::<piney_world::evarea07::Giant>().is_some_and(|g| g.block == 0 && g.clouds.len() == 25)
        );
        assert!(at(w, [0.0, -5400.0, -50.0]), "{}", Mode::title(&s));
        hold(&mut s, 128, 128, 120, |s| playing(s).is_some_and(|w| w.player().acts.act == 2));
        hold(&mut s, 128, 0, 1500, |s| arrived(s).is_some_and(|w| w.scene().area == 2));
        let w = arrived(&s).unwrap();
        assert_eq!((w.scene().area, w.scene().field), (2, 16), "{}", Mode::title(&s));
        let Stage::Area(a) = &mut s.stage else { panic!("not in the dungeon") };
        assert!(a.world_mut().go_field());
        hold(&mut s, 128, 128, 600, |s| {
            arrived(s).is_some_and(|w| w.place().story::<piney_world::evarea07::Giant>().is_some())
        });
        let w = arrived(&s).unwrap();
        assert!(w.place().story::<piney_world::evarea07::Giant>().is_some_and(|g| g.block == 0), "{}", Mode::title(&s));
        assert_eq!(w.scene().area_prev, 2);
        let back = piney_world::evarea07::Giant::new(&archive, piney_data::volume::Volume::Inf, 0, 2, 0)
            .unwrap()
            .start
            .map(f32::from_bits);
        assert!(at(w, [back[0], back[1], back[2]]), "{} (DMY_marker02 {back:?})", Mode::title(&s));
    }

    /// From Mac Anu, the Chaos Gate's warp with area 15's words
    /// (`WORLD_MAN::SetGenerateCode`): the story map, not a generated field.
    #[test]
    fn the_gate_takes_its_words_to_the_holy_ground() {
        let Some((iso, archive)) = disc() else { return };
        let mut d = Iso::open(&iso).unwrap();
        let tables = crate::area::area_tables(&mut d).unwrap();
        let code = piney_data::area::story_area_code(tables, 15, false).unwrap();
        let mut s = start(&iso, &archive, None);
        hold(&mut s, 128, 128, 200, |s| matches!(&s.stage, Stage::World(_)));
        wait(&mut s, 100);
        s.go(Pending::Words([code.a, code.b, code.c]));
        hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
        let w = playing(&s).unwrap();
        assert_eq!((w.scene().area, w.scene().field), (1, 15));
        assert_eq!(w.world_man().field_model, 1);
        assert!(w.place().story::<piney_world::evarea::EventArea>().is_some_and(|e| e.block == 0));
        assert!(at(w, [0.0, -3600.0, 0.0]), "{}", Mode::title(&s));
    }

    /// `--dvd`: the Chaos Gate's warp to area 15 waits for its files under
    /// the loading display (the area's card fading in, the scene not yet
    /// stepped), then plays; the frames the files take are printed.
    #[test]
    fn a_warp_waits_for_the_disc_under_the_loading_display() {
        let Some((iso, archive)) = disc() else { return };
        let mut d = Iso::open(&iso).unwrap();
        let tables = crate::area::area_tables(&mut d).unwrap();
        let code = piney_data::area::story_area_code(tables, 15, false).unwrap();
        let mut s = start(&iso, &archive, None);
        s.dvd = Some(crate::dvd::Dvd { rate: 3.0 * 1_385_000.0, seek: 0.12 });
        hold(&mut s, 128, 128, 200, |s| matches!(&s.stage, Stage::World(_)));
        wait(&mut s, 100);
        s.go(Pending::Words([code.a, code.b, code.c]));
        hold(&mut s, 128, 128, 400, |s| s.hold > 0);
        let frames = s.hold;
        eprintln!("area 15 from Mac Anu: {frames} frames of disc");
        assert!(s.load_disp.is_some(), "no loading display");
        assert!(playing(&s).is_none());
        wait(&mut s, frames - 1);
        assert_eq!(s.hold, 1);
        assert!(!s.load_started, "the scene ran during the load");
        hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
        assert_eq!(s.hold, 0);
    }

    /// The game over in area 14's field, forced as a lost key item forces it
    /// (`compulsionGameOver`): the next frame `ccThGameCtrl` signals it,
    /// then `ccSndGameOver`, `ccThGameOver`'s noise and the television
    /// switching off (sound 90), the field's tasks gone, the GAME OVER clip,
    /// and the soft reset to the title.
    #[test]
    fn a_game_over_ends_at_the_title() {
        let Some((iso, archive)) = disc() else { return };
        let mut s = start(&iso, &archive, Some(14));
        hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
        wait(&mut s, 30);
        let Stage::Area(a) = &mut s.stage else { panic!("not in the field") };
        a.world_mut().compulsion_game_over = true;
        let (mut sound, mut tv, mut title_at) = (None, None, None);
        let mut pad = Pad::default();
        for f in 0..1500u32 {
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            for e in s.take_events() {
                match e {
                    Event::SoundGameOver => sound = sound.or(Some(f)),
                    Event::Se(90) => tv = tv.or(Some(f)),
                    _ => {}
                }
            }
            if matches!(s.stage, Stage::Title(_)) {
                title_at = Some(f);
                break;
            }
        }
        eprintln!("game over: sound {sound:?}, the TV off {tv:?}, the title {title_at:?}");
        let (sound, tv, title) =
            (sound.expect("no ccSndGameOver"), tv.expect("no sound 90"), title_at.expect("never at the title"));
        // ccThGameOver waits 180 frames, then the noise's phases 1 and 2
        // (51 and 41 frames) before the TV squeezes to its line (9 frames).
        assert!(tv > sound + 180 + 92, "{sound} {tv}");
        assert!(title > tv + 40, "{tv} {title}");
    }

    /// Pictures of the game over (`a_game_over_ends_at_the_title`'s run):
    /// `PINEY_SHOTS=DIR cargo test --release -p piney-game game_over_shots
    /// -- --ignored` (default `/mnt/data/claude/scratch/gameover`).
    #[test]
    #[ignore]
    fn game_over_shots() {
        let Some((iso, archive)) = disc() else { return };
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/gameover".into());
        std::fs::create_dir_all(&dir).unwrap();
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive.clone())).unwrap();
        let mut s = start(&iso, &archive, Some(14));
        hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
        wait(&mut s, 30);
        let Stage::Area(a) = &mut s.stage else { panic!("not in the field") };
        a.world_mut().compulsion_game_over = true;
        let mut pad = Pad::default();
        for f in 0..470u32 {
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            let frame = s.step(&pad);
            s.take_events();
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            if [150, 200, 250, 276, 283, 286, 292, 300, 360, 400, 440].contains(&f) {
                let (w, h) = gs.target_size();
                std::fs::write(format!("{dir}/over-{f:03}.png"), piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            }
            if matches!(s.stage, Stage::Title(_)) {
                break;
            }
        }
    }

    /// Kite's shadow: in story area 14's field and in Mac Anu, once he
    /// walks, the frame carries the characters' shadow packet (256 x 256,
    /// darkness 0x30, on priority 2) holding his volume at full alpha, and
    /// the CPU GS darkens the ground with it.
    #[test]
    fn kite_casts_his_shadow() {
        let Some((iso, archive)) = disc() else { return };
        let mut soft_assets = piney_desktop::soft::Assets::new(archive.clone());
        for (name, field) in [("field", Some(14)), ("town", None)] {
            let mut s = start(&iso, &archive, field);
            if field.is_some() {
                hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
            } else {
                wait(&mut s, 200);
            }
            let frame = hold_frame(&mut s, 200, 40, 120);
            let pass = frame
                .cmds
                .iter()
                .find_map(|c| match c {
                    piney_draw::Cmd::Shadow(p) => Some(p),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name}: no shadow pass"));
            assert_eq!((pass.width, pass.height, pass.darkness), (256, 256, 0x30), "{name}");
            let polys: usize = pass.groups.iter().map(|g| g.polys.len()).sum();
            assert!(pass.groups.iter().any(|g| g.alpha == 128) && polys > 100, "{name}: {polys} polygons");
            let mut render = |f: &Frame| {
                let mut c = piney_desktop::soft::Canvas::new(512, 448, f.clear);
                c.draw(f, &mut soft_assets);
                c.rgba()
            };
            let with = render(&frame);
            let mut bare = frame.clone();
            bare.cmds.retain(|c| !matches!(c, piney_draw::Cmd::Shadow(_)));
            let without = render(&bare);
            let dark = (0..512 * 448).filter(|&i| with[4 * i] < without[4 * i]).count();
            eprintln!("{name}: {polys} polygons, {dark} pixels darkened");
            assert!(dark > 300, "{name}: {dark}");
        }
    }

    /// Pictures of Kite and the party in story area 14's field and walking
    /// on, their shadows on the ground: `PINEY_SHOTS=DIR cargo test
    /// --release -p piney-game field_shadow_shots -- --ignored` (default
    /// `/mnt/data/claude/scratch/shadow/shots`).
    #[test]
    #[ignore]
    fn field_shadow_shots() {
        let Some((iso, archive)) = disc() else { return };
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/shadow/shots".into());
        std::fs::create_dir_all(&dir).unwrap();
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive.clone())).unwrap();
        for (name, field) in [("field", Some(14)), ("town", None)] {
            let mut s = start(&iso, &archive, field);
            if field.is_some() {
                hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
            } else {
                wait(&mut s, 200);
            }
            let mut pad = Pad::default();
            for f in 0..160u32 {
                // Still, then walking up and to the right.
                let (lx, ly) = if f < 60 { (128, 128) } else { (200, 40) };
                pad.read(&Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() });
                let frame = s.step(&pad);
                s.take_events();
                gs.set_overlay(Mode::archive(&s));
                gs.render(&frame);
                if [59, 150].contains(&f) {
                    let (w, h) = gs.target_size();
                    let with = gs.read_back();
                    std::fs::write(format!("{dir}/{name}-{f:03}.png"), piney_gs::png::encode(w, h, &with)).unwrap();
                    let mut bare = frame.clone();
                    bare.cmds.retain(|c| !matches!(c, piney_draw::Cmd::Shadow(_)));
                    gs.render(&bare);
                    let without = gs.read_back();
                    let n =
                        (0..(w * h) as usize).filter(|&i| with[4 * i..4 * i + 3] != without[4 * i..4 * i + 3]).count();
                    eprintln!("{name} frame {f}: {n} pixels in shadow");
                }
            }
        }
    }

    /// From Mac Anu, the gate's warp with "Chronicling" (131) as the first
    /// word, "Passed Over" and "Aqua Field": `WORLD_MAN.timeSym`, the
    /// field's `gameCnt[2]` running from the gate, and in its dungeon
    /// `SetIDOL` making each idol the Zeit statue (row 44).
    #[test]
    fn chronicling_leads_to_the_zeit_statue() {
        let Some((iso, archive)) = disc() else { return };
        let mut s = start(&iso, &archive, None);
        hold(&mut s, 128, 128, 200, |s| matches!(&s.stage, Stage::World(_)));
        wait(&mut s, 100);
        s.go(Pending::Words([131, 13, 26]));
        hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
        let w = playing(&s).unwrap();
        assert!(w.world_man().time_sym());
        let since_gate = w.scene().game_cnt[2];
        assert!((1..400).contains(&since_gate), "gameCnt {:?}", w.scene().game_cnt);
        s.go(Pending::Go(piney_data::area::Go::ChangeArea(piney_world::area::kind::DUNGEON, 0)));
        hold(&mut s, 128, 128, 400, |s| playing(s).is_some_and(|w| w.scene().area == 2));
        wait(&mut s, 30);
        let w = playing(&s).unwrap();
        assert!(w.scene().game_cnt[2] > since_gate, "gameCnt {:?}", w.scene().game_cnt);
        let c = w.combat();
        let idols: Vec<i32> = c
            .ctrl
            .list(piney_battle::entry::Kind::Gimmick)
            .into_iter()
            .filter_map(|g| match c.ctrl.objs.get(g) {
                // The idols: the spring (20), the Gott statues (38-43), Zeit (44).
                Some(piney_battle::entry::Obj::Gimmick(o)) if matches!(o.ent.id, 20 | 38..=44) => Some(o.ent.id),
                _ => None,
            })
            .collect();
        eprintln!("idols {idols:?}, {}", Mode::title(&s));
        assert!(!idols.is_empty() && idols.iter().all(|&i| i == 44), "{idols:?}");
    }

    /// `--mode story:11` (Mac Anu at event 11, the event task with it), then
    /// the gate's warp with the words BlackRose gives: the holy ground's
    /// set-up runs event 11's passes there and play begins.
    #[test]
    fn story_11_warps_to_the_holy_ground() {
        let Some((iso, archive)) = disc() else { return };
        let mut d = Iso::open(&iso).unwrap();
        let tables = crate::area::area_tables(&mut d).unwrap();
        let code = piney_data::area::story_area_code(tables, 15, false).unwrap();
        let mut start = crate::start::build(&iso, 11).unwrap();
        start.vm.trace = Some(Vec::new());
        let crate::start::Start { state, vm, at: resume } = start;
        let mut s = Session::resume(iso.clone(), archive, None, state, vm, resume).unwrap();
        hold(&mut s, 128, 128, 300, |s| matches!(&s.stage, Stage::World(_)));
        // Event 11's first line in Mac Anu, closed as a player would (the
        // gate's menu would not open over it).
        let mut pad = Pad::default();
        for n in 0..240 {
            let buttons = if n % 40 == 39 { Buttons::CROSS } else { Buttons::NONE };
            pad.read(&Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        s.go(Pending::Words([code.a, code.b, code.c]));
        hold(&mut s, 128, 128, 600, |s| playing(s).is_some());
        let w = playing(&s).unwrap();
        assert_eq!((w.scene().area, w.scene().field), (1, 15));
        assert!(w.place().story::<piney_world::evarea::EventArea>().is_some_and(|e| e.block == 0));
        assert!(at(w, [0.0, -3600.0, 0.0]), "{}", Mode::title(&s));
        // The event task's passes there are over: play goes on, the events
        // at phase 5.
        wait(&mut s, 60);
        let Stage::Area(a) = std::mem::replace(&mut s.stage, Stage::Gone) else { panic!("left the holy ground") };
        let vm = a.leave().1.unwrap();
        let passes: Vec<i32> = vm
            .trace
            .iter()
            .flatten()
            .filter_map(|t| match *t {
                piney_event::vm::Trace::Pass { phase, .. } => Some(phase),
                _ => None,
            })
            .collect();
        assert!(passes.ends_with(&[5, 5, 5]), "{:?}", &passes[passes.len().saturating_sub(8)..]);
    }
}
