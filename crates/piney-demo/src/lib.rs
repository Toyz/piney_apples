//! The title screen of .hack//Infection (`DATA/DEMO.PRG`, mode 2), as a state
//! machine: a pad in, a [`piney_draw::Frame`] out, once per game frame
//! (`docs/engine/title.md`). `ccSetupDemo` (0x00168160) starts `ccThDemo`
//! (demo.prg 0x00400900: the memory-card check, the logos, the opening
//! stream, the title, the attract loop) and the system menu `ccThDtMenu`.
//! [`Demo::step`] is one pass between breaths; the movies and the stream are
//! asked of the runtime ([`Request::Movie`], [`Request::Stream`]). The boot
//! check ([`card`]) and the load screen ([`dataload`]) share one `ccSaveSys`.

pub mod card;
pub mod dataload;
pub mod dialog;
pub mod draw;
pub mod fade;
pub mod names;
pub mod newgame;
pub mod opening;
pub mod seam;

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::exe::Overlay;
use piney_data::iso::Iso;
use piney_data::save::{SaveData, offset};
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::Fonts;
use piney_desktop::view::View;
use piney_draw::{Frame, Rgba};
use piney_input::Pad;

use piney_desktop::SaveState;
use piney_desktop::card::MemoryCard;
use piney_desktop::savesys::{OPERATE_TITLE, SaveSys};

use crate::card::SaveSysTask;
use crate::dialog::DialogAssets;
use crate::fade::ScFade;
use crate::newgame::{NewGameTables, load_game, new_game};
use crate::opening::{Env, Opening, start};
use crate::seam::{MemCard, MenuFrame, StubMenu, SystemMenu, TitleMenu};

/// `ccSystem.frameRate` on the title: `ccSetupDemo` calls
/// `SetFrameRate(1)`.
pub const FRAME_RATE: u32 = 1;

/// `ccGame::ChangeRequest(3, 7)`: what `ccThDemo` asks for once the title
/// is left for a game (the desktop, `ccSetupDesktop`).
pub const DESKTOP_REQUEST: (i32, i32) = (3, 7);

/// `ccSaveData.mainVol` (DWARF +0x841e).
pub const MAIN_VOL: usize = 0x841e;

/// `ccSqFade(0, 0, 8, 3)`: the title music faded out on the way out.
pub const LEAVE_FADE: (i32, u16, i32, u8) = (0, 0, 8, 3);

/// `LogoMain`: frames it breathes after the last logo or a skip, before
/// the title music starts.
pub const LOGO_WAIT: u32 = 60;

/// `PlayOpeningStream`'s flashes: at the stream's frame `length - 20`,
/// `EntryFlash3(20, 20, 5, 0x80ffffff)` (in over 21 frames, hold 6, out
/// 21); a cancel push, `EntryFlash(50, 0x80ffffff)`.
pub const STREAM_FLASH: (i16, i16, i16, u32) = (20, 20, 5, 0x80ff_ffff);
pub const STREAM_FLASH_LEAD: u32 = 20;
pub const STREAM_SKIP_FLASH: (i16, u32) = (50, 0x80ff_ffff);

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// `ccSeOn(n)`, the common bank.
    Se(i32),
    /// `ccSqPlay(n)`: a sequence of the demo's bank (`ccSndSQLoad(7)`); 0
    /// is the title music.
    SqPlay(i32),
    /// `ccSqStop(n)`.
    SqStop(i32),
    /// `ccSqFade(n, volume, time, mode)`.
    SqFade { seq: i32, volume: u16, time: i32, mode: u8 },
    /// `ccSetMainVol(saveData->mainVol)`.
    MainVolume(i16),
    /// `ccDecodeMpeg(path, audio)`: a PSS movie, played to its end before
    /// the next step unless the runtime calls [`Demo::movie_skipped`] (the
    /// game's player stops on ok, cancel or START once 11 frames have been
    /// decoded, and returns -1).
    Movie { path: &'static str, audio: bool },
    /// `ccThExecuteStream` / `ccRequestLoadStream(num)`: an in-engine
    /// stream (the title's intro, `streamTbl[num]`), played before the next
    /// step. `frames` is the frame `PlayOpeningStream` counts it to.
    Stream { num: i32, frames: u32 },
    /// `dtMenu->dispFlag`: whether the system menu may open.
    MenuDisplay(bool),
    /// `saveData->NewGame(0)` (New Game or Parody), on a save whose
    /// `parodyFlag` is `parody` (`PlayParodyGame` sets it). [`Demo::new`]'s
    /// title has already applied it to [`Demo::save`] ([`newgame`]).
    NewGame { parody: bool },
    /// `saveData->LoadGame()`: a save was loaded into `saveData`. The title
    /// has already applied it to [`Demo::save`] ([`newgame::load_game`]).
    LoadGame,
    /// `ccStartEventConvert()`, `saveData->ConvGame()`: a previous
    /// volume's save carried over (volumes 2-4; never in Infection). The
    /// title has already applied both to [`Demo::save`]
    /// ([`newgame::conv_game`]).
    ConvGame,
    /// `ccGame::ChangeRequest(num, sf)`: leave the title.
    ChangeMode { num: i32, sf: i32 },
    /// What the system menu's options ask of the game, as the desktop's
    /// menu asks it: `DisplayOffset` (Adjust Screen), `SoundEnv` (Sound),
    /// `Vibration`, `CameraType` (Controller). Its sounds come as
    /// [`Request::Se`].
    Menu(piney_desktop::Request),
}

/// How the title starts, and what answers for the parts not ported.
pub struct Config {
    /// The game's `ccSaveData` after the boot's `ccSaveData::Init(1)`.
    /// [`Demo::new`] applies `ccSetupDemo`'s `NewGame(1)` to it; the title
    /// reads `assignPADok` / `assignPADcancel` and `mainVol`, writes
    /// `parodyFlag`, and applies `NewGame(0)` on New Game.
    pub save: SaveData,
    /// `ResetOpFlg == 0`: the first boot, so the logos play. After a soft
    /// reset (`ResetOpFlg` 1) they are skipped.
    pub first_boot: bool,
    /// `m_ParoFLG`: the Parody item. Infection's code never sets it; this
    /// is here to exercise the branch.
    pub parody_item: bool,
    /// The memory cards: the boot check and the load screen read them.
    pub card: Box<dyn MemoryCard>,
    /// `ccSaveSys`'s `port` and `fileNum` as the title finds them (the
    /// game keeps one `ccSaveSys` from boot): the load screen's cursors
    /// start there.
    pub card_position: (i32, i32),
    /// The system menu; None: the real one ([`seam::TitleMenu`], read from
    /// the disc by [`Demo::new`]; a [`seam::StubMenu`] for
    /// [`Demo::with_file`]).
    pub menu: Option<Box<dyn SystemMenu>>,
    /// The disc's volume: the title's `m_NowVol` (`volumeNum`).
    pub volume: piney_data::volume::Volume,
    /// Not the game's: how many of the logo movies ([`names::LOGO_MOVIES`])
    /// were played before this title (the port's launcher plays the three
    /// logos), so that the first boot starts `m_LogoAct` there.
    pub logos_played: i32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            save: piney_desktop::SaveState::fresh().save,
            first_boot: true,
            parody_item: false,
            card: Box::new(MemCard::default()),
            card_position: (0, 0),
            menu: None,
            volume: piney_data::volume::Volume::Inf,
            logos_played: 0,
        }
    }
}

/// Where `ccThDemo` is, between breaths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Before the first breath: `ccSqStop(0)`, the control, `SetVolCcs`.
    Start,
    /// `while !r: breathe; r = PlayBootMemCard()`.
    Boot,
    /// The logo loop's next `LogoMain` call.
    Logo,
    /// A logo movie requested; the step after it is where `ccDecodeMpeg`
    /// returns.
    Movie,
    /// `LogoMain`'s 60 breaths after `m_LogoAct` became 4; how many are
    /// left.
    LogoWait(u32),
    /// The breath at the end of the logo loop after `LogoMain` returned 1.
    LogoDone,
    /// `PlayOpeningStream`: the stream requested.
    Stream,
    /// The title: `breathe; r = Main()`.
    Title,
    /// `ccSqFade`, then `Breath(2)`: the step `ChangeRequest` is made in.
    Leaving(u32),
    /// `ChangeRequest` made; the task breathes until it is taken down.
    Left,
}

/// The title.
pub struct Demo {
    file: Rc<SceneFile>,
    open: Opening,
    /// `saveData`, and the event manager's operate bits the menu reads.
    state: SaveState,
    parody_flag: bool,
    first_boot: bool,
    card: Box<dyn MemoryCard>,
    /// `saveSys` and its task.
    sys: SaveSys,
    sys_task: SaveSysTask,
    menu: Box<dyn SystemMenu>,
    fade: ScFade,
    view: View,
    phase: Phase,
    movie_skipped: bool,
    requests: Vec<Request>,
    /// `ccSys.count`: frames since the title started.
    count: u32,
    /// The memory-card question's fonts and texts.
    dialog: Option<DialogAssets>,
    /// What `ccSaveData::NewGame` copies.
    tables: Option<NewGameTables>,
    /// `PlayOpeningStream`'s frame count while no flash has come yet (its
    /// `s1`): a cancel push or the stream's frame `count - 20` flashes.
    stream_flash: Option<u32>,
}

impl Demo {
    /// `ccSetupDemo` as far as the title goes: the overlay checked, `title1`
    /// read from `DATA.BIN`.
    pub fn new(iso: &mut Iso, archive: Arc<Archive>, config: Config) -> piney_data::Result<Demo> {
        let prg = Overlay::parse(iso.read_path(names::PRG_PATH)?)?;
        if prg.name != names::PRG_NAME {
            return Err(piney_data::Error::Format(format!("{} is {}", names::PRG_PATH, prg.name)));
        }
        let file = Rc::new(SceneFile::read(&archive, names::title_file(iso.volume()?))?);
        let font = Ccs::parse(archive.inflate_named(piney_desktop::assets::FONT_FILE)?)?;
        let (_, cluts) = piney_data::texture::read(&font)?;
        let clut = font
            .find_object("CLT_xasc00")
            .and_then(|o| cluts.get(&o))
            .ok_or_else(|| piney_data::Error::NotFound("CLT_xasc00".into()))?;
        let fonts = Fonts::of(iso.volume()?, clut.colours.iter().map(|c| Rgba(*c)).collect());
        let dialog = DialogAssets::read(iso.volume()?, fonts, &file)?;
        let tables = NewGameTables::of(iso.volume()?);
        let mut config = config;
        config.volume = iso.volume()?;
        if config.menu.is_none() {
            config.menu = Some(Box::new(TitleMenu::new(iso.volume()?, &archive)));
        }
        let mut demo = Self::with_file(file, config);
        // ccSetupDemo: saveData->NewGame(1), before the task starts.
        new_game(&mut demo.state.save, 1, &tables, newgame::SAVE_VA);
        demo.dialog = Some(dialog);
        demo.tables = Some(tables);
        Ok(demo)
    }

    /// The title on an already read `title1`.
    pub fn with_file(file: Rc<SceneFile>, config: Config) -> Demo {
        let parody_flag = config.save.parody();
        let mut open = Opening::new(file.clone());
        open.paro_flg = config.parody_item;
        open.now_vol = config.volume.number() as i16;
        open.logo_act = config.logos_played.clamp(0, 3);
        // Mutation on, ccThDemo after Init: four items.
        open.set_max_cur();
        // SetStream: SetView(sysLayer's view, m_Camera->cam).
        let mut view = View::default();
        if let Some(cam) = open.camera.camera {
            view.set_camera(&cam);
        }
        // Init: the load screen's control built (its Init asks
        // SlotSelectReq), then saveSys->StartReq(1).
        let mut sys = SaveSys::new(config.volume);
        (sys.port, sys.file_num) = config.card_position;
        open.data_load.init(&mut sys);
        open.next_load.init_next(&mut sys);
        sys.start_req(OPERATE_TITLE);
        Demo {
            file,
            open,
            state: SaveState::new(config.save),
            parody_flag,
            first_boot: config.first_boot,
            card: config.card,
            sys,
            sys_task: SaveSysTask::default(),
            menu: config.menu.unwrap_or_else(|| Box::new(StubMenu::default())),
            fade: ScFade::default(),
            view,
            phase: Phase::Start,
            movie_skipped: false,
            requests: Vec::new(),
            count: 0,
            dialog: None,
            tables: None,
            stream_flash: None,
        }
    }

    /// `ccSystem.frameRate`: vertical blanks per game frame.
    pub fn frame_rate(&self) -> u32 {
        FRAME_RATE
    }

    /// What the frames since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// The player stopped the movie of the last [`Request::Movie`]
    /// (`ccDecodeMpeg` returned -1): the rest of the logos and the opening
    /// movie are skipped.
    pub fn movie_skipped(&mut self) {
        self.movie_skipped = true;
    }

    /// Where the task is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// `ccOpening_Control`, for inspection.
    pub fn opening(&self) -> &Opening {
        &self.open
    }

    /// `title1`, the title's scene.
    pub fn file(&self) -> &Rc<SceneFile> {
        &self.file
    }

    /// The save as the title leaves it: `NewGame(0)` applied for New Game
    /// (`parodyFlag` set by Parody), the slot read and `LoadGame` applied
    /// for Load.
    pub fn save(&self) -> &SaveData {
        &self.state.save
    }

    /// The same, to change: the port puts its kept options in it.
    pub fn save_mut(&mut self) -> &mut SaveData {
        &mut self.state.save
    }

    /// The system menu, for inspection.
    pub fn menu(&self) -> &dyn SystemMenu {
        &*self.menu
    }

    /// `ccSaveSys`, for inspection.
    pub fn save_sys(&self) -> &SaveSys {
        &self.sys
    }

    /// `ccSaveSys`'s `port` and `fileNum`: the card slot and save the load
    /// screen last used, where the desktop's Data screen starts
    /// (`piney_desktop::Desktop::set_card_position`).
    pub fn card_position(&self) -> (i32, i32) {
        (self.sys.port, self.sys.file_num)
    }

    /// The menu cursor (0 New Game, 1 Load, 2 Option).
    pub fn cursor(&self) -> i32 {
        self.open.nut_cur_no
    }

    /// A line for a window title: where the task is, the action, the
    /// cursor.
    pub fn status(&self) -> String {
        format!(
            "title - {:?} - action {} - cursor {} - frame {}",
            self.phase, self.open.main_act, self.open.nut_cur_no, self.count
        )
    }

    /// One game frame.
    pub fn step(&mut self, pad: &Pad) -> Frame {
        self.count = self.count.wrapping_add(1);
        self.sys_task.frame(&mut self.sys, &mut *self.card, &mut self.state.save);
        let mut ctx = Ctx::new(self.view.clone());
        let asked = self.requests.len();
        let ok = u32::from(self.state.save.assign_pad_ok());
        let cancel = u32::from(self.state.save.assign_pad_cancel());
        {
            let mut env = Env {
                ctx: &mut ctx,
                pad,
                ok,
                cancel,
                count: self.count,
                main_vol: self.state.save.i16(MAIN_VOL),
                req: &mut self.requests,
                fade: &mut self.fade,
                menu: &mut *self.menu,
                sys: &mut self.sys,
                parody_flag: &mut self.parody_flag,
                dialog: self.dialog.as_ref(),
            };
            self.phase = task(&mut self.open, self.phase, &mut env, &mut self.movie_skipped, self.first_boot);
        }
        if let Some(&Request::Stream { frames, .. }) =
            self.requests[asked..].iter().find(|r| matches!(r, Request::Stream { .. }))
        {
            self.stream_flash = Some(frames);
        }
        if self.parody_flag {
            self.state.save.set_u8(offset::PARODY_FLAG, 1);
        }
        // ccThDemo: Main returned 2, saveData->NewGame(0).
        if let Some(t) = &self.tables
            && self.requests[asked..].iter().any(|r| matches!(r, Request::NewGame { .. }))
        {
            new_game(&mut self.state.save, 0, t, newgame::SAVE_VA);
            // The port's own: Helba's mail, read, on the later volumes.
            piney_desktop::extras::new_game(&mut self.state.save, t.volume);
        }
        // Main returned 3: saveData->LoadGame().
        if let Some(t) = &self.tables
            && self.requests[asked..].iter().any(|r| matches!(r, Request::LoadGame))
        {
            load_game(&mut self.state.save, t, newgame::SAVE_VA);
        }
        // Main returned 4 (CONVERT): ccStartEventConvert(), then
        // saveData->ConvGame() on the previous volume's save just read.
        if let Some(t) = &self.tables
            && self.requests[asked..].iter().any(|r| matches!(r, Request::ConvGame))
        {
            newgame::start_event_convert(&mut self.state.save, t.volume);
            newgame::conv_game(&mut self.state.save, t, newgame::SAVE_VA);
        }
        // ccThDtMenu, after ccThDemo.
        let names = self.state.names();
        self.menu.frame(&mut MenuFrame { pad, state: &mut self.state, req: &mut self.requests, names: names.clone() });
        if let Some(a) = &self.dialog {
            self.menu.disp(&mut ctx, &a.fonts, &names);
        }
        self.fade.send(&mut ctx);
        ctx.finish()
    }
}

impl Demo {
    /// `scFadeDef`: the title's screen flashes and fades.
    pub fn fade(&self) -> &ScFade {
        &self.fade
    }

    /// `PlayOpeningStream`'s loop (0x00406838) for a frame of the stream,
    /// before the stream's own task: a cancel push flashes white over 50
    /// frames; else, at the stream's frame `count - 20` (`frame`, as the
    /// last step left it), the end flash. Either one only, once.
    pub fn stream_tick(&mut self, pad: &Pad, frame: u32) {
        let Some(count) = self.stream_flash else { return };
        if pad.push.bits() & u32::from(self.state.save.assign_pad_cancel()) != 0 {
            let (t, c) = STREAM_SKIP_FLASH;
            self.fade.entry_flash(t, c);
            self.stream_flash = None;
        } else if frame + STREAM_FLASH_LEAD == count {
            let (t0, t1, t2, c) = STREAM_FLASH;
            self.fade.entry_flash3(t0, t1, t2, c);
            self.stream_flash = None;
        }
    }

    /// `scFadeDef` over a frame of the stream: the flashes advanced a frame
    /// and drawn last.
    pub fn stream_fade(&mut self, frame: &mut Frame) {
        let mut ctx = Ctx::new(self.view.clone());
        self.fade.send(&mut ctx);
        let f = ctx.finish();
        frame.uploads.extend(f.uploads);
        frame.cmds.extend(f.cmds);
    }
}

/// The calls `ccThDemo` makes on its `ccOpening_Control`.
pub trait Control {
    /// `PlayBootMemCard`: 1 when the memory-card check is over.
    fn play_boot_mem_card(&mut self, env: &mut Env) -> i32;
    /// `Main`: non-zero is `m_StartMode`.
    fn main(&mut self, env: &mut Env) -> i32;
    /// `m_LogoAct`.
    fn logo_act(&self) -> i32;
    fn set_logo_act(&mut self, act: i32);
    /// `m_ParoFLG`, which picks the opening stream.
    fn parody_item(&self) -> bool;
    /// `m_NowVol`, which picks it too.
    fn volume(&self) -> i16 {
        1
    }
}

impl Control for Opening {
    fn play_boot_mem_card(&mut self, env: &mut Env) -> i32 {
        Opening::play_boot_mem_card(self, env)
    }

    fn main(&mut self, env: &mut Env) -> i32 {
        Opening::main(self, env)
    }

    fn logo_act(&self) -> i32 {
        self.logo_act
    }

    fn set_logo_act(&mut self, act: i32) {
        self.logo_act = act;
    }

    fn volume(&self) -> i16 {
        self.now_vol
    }

    fn parody_item(&self) -> bool {
        self.paro_flg
    }
}

/// `ccThDemo` from `phase` to its next breath. `skipped` is the answer to
/// the last movie (taken); `first_boot` is `ResetOpFlg == 0`.
pub fn task<C: Control + ?Sized>(
    o: &mut C,
    phase: Phase,
    env: &mut Env,
    skipped: &mut bool,
    first_boot: bool,
) -> Phase {
    match phase {
        Phase::Start => {
            env.req.push(Request::SqStop(0));
            // Opening::new ran the constructor and SetVolCcs.
            Phase::Boot
        }
        Phase::Boot => {
            if o.play_boot_mem_card(env) == 0 {
                return Phase::Boot;
            }
            if first_boot {
                env.menu.set_display(false);
                env.req.push(Request::MenuDisplay(false));
                logo(o, env)
            } else {
                env.menu.set_display(true);
                env.req.push(Request::MenuDisplay(true));
                env.req.push(Request::MainVolume(env.main_vol));
                env.req.push(Request::SqPlay(0));
                stream(o, env)
            }
        }
        Phase::Logo => logo(o, env),
        Phase::Movie => {
            // ccDecodeMpeg returned: -1 skips to the end of the logos.
            let was = o.logo_act();
            let now = if std::mem::take(skipped) || was >= 3 { 4 } else { was + 1 };
            o.set_logo_act(now);
            if now == 4 { Phase::LogoWait(LOGO_WAIT - 1) } else { Phase::Logo }
        }
        Phase::LogoWait(0) => Phase::Logo,
        Phase::LogoWait(n) => Phase::LogoWait(n - 1),
        Phase::LogoDone => {
            env.menu.set_display(true);
            env.req.push(Request::MenuDisplay(true));
            stream(o, env)
        }
        // The stream has ended: PlayOpeningStream returns, r = 0, and the
        // task breathes. Its flashes ran over the stream
        // ([`Demo::stream_tick`]).
        Phase::Stream => Phase::Title,
        Phase::Title => match o.main(env) {
            0 => Phase::Title,
            start::NEW_GAME => {
                env.req.push(Request::NewGame { parody: *env.parody_flag });
                leave(env)
            }
            start::LOAD => {
                env.req.push(Request::LoadGame);
                leave(env)
            }
            start::CONVERT => {
                env.req.push(Request::ConvGame);
                leave(env)
            }
            start::ATTRACT => {
                env.req.push(Request::SqStop(0));
                env.menu.set_display(false);
                env.req.push(Request::MenuDisplay(false));
                o.set_logo_act(3);
                logo(o, env)
            }
            // Any other value stays in r, skips the logo loop and plays the
            // stream again (never returned: m_StartMode is 2 to 5).
            _ => stream(o, env),
        },
        Phase::Leaving(0) => {
            let (num, sf) = DESKTOP_REQUEST;
            env.req.push(Request::ChangeMode { num, sf });
            Phase::Left
        }
        Phase::Leaving(n) => Phase::Leaving(n - 1),
        Phase::Left => Phase::Left,
    }
}

/// `LogoMain` (0x00404540) from the top, to the task's next breath.
fn logo<C: Control + ?Sized>(o: &mut C, env: &mut Env) -> Phase {
    match o.logo_act() {
        act @ 0..=3 => {
            let (path, audio) = names::logo_movie(o.volume(), act as usize).unwrap_or(names::LOGO_MOVIES[0]);
            env.req.push(Request::Movie { path, audio });
            Phase::Movie
        }
        _ => {
            // LogoAct 4: the music, then out of the loop.
            env.req.push(Request::MainVolume(env.main_vol));
            env.req.push(Request::SqPlay(0));
            o.set_logo_act(0);
            Phase::LogoDone
        }
    }
}

/// `PlayOpeningStream` (0x004066b0) up to its first breath.
fn stream<C: Control + ?Sized>(o: &C, env: &mut Env) -> Phase {
    let (num, frames) = names::opening_stream(o.volume(), o.parody_item());
    env.req.push(Request::Stream { num, frames });
    Phase::Stream
}

/// Out of the loop: `ccSqFade(0, 0, 8, 3)`, `Breath(2)`.
fn leave(env: &mut Env) -> Phase {
    let (seq, volume, time, mode) = LEAVE_FADE;
    env.req.push(Request::SqFade { seq, volume, time, mode });
    Phase::Leaving(1)
}
