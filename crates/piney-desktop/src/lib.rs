//! The ALTIMIT desktop of .hack//Infection (`DATA/DESKTOP.PRG`), as a state
//! machine: a pad in, a [`piney_draw::Frame`] out, once per game frame
//! (`docs/engine/desktop.md`). The task `ccThDesktop` (desktop.prg 0x00400910)
//! sets the scene and wallpaper, plays the opening, builds the lists
//! (`AddAllList`), then loops `SelectMode` / `ChooseMode` between two
//! `ccTscb::Breath` calls a frame; [`Desktop::step`] is one pass between
//! breaths. The content is read from the disc image; the game state events
//! own is [`SaveState`].

pub mod acces;
pub mod anm;
pub mod assets;
pub mod audio;
pub mod camera;
pub mod card;
pub mod content;
pub mod data;
pub mod desktop;
pub mod dtmenu;
pub mod eef;
pub mod extras;
pub mod fade;
pub mod frames;
pub mod kanji;
pub mod layers;
pub mod mail;
pub mod message;
pub mod name_entry;
pub mod news;
pub mod noiz;
pub mod save;
pub mod savesys;
pub mod setup;
pub mod shadow;
pub mod soft;
pub mod sprite;
pub mod staffroll;
pub mod view;

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_draw::Frame;
use piney_input::Pad;

pub use save::{InitText, MailState, SaveState};

use crate::acces::Accessory;
use crate::anm::Ctx;
use crate::assets::Assets;
use crate::audio::Audio;
use crate::card::MemoryCard;
use crate::data::Data;
use crate::desktop::{DesktopControl, Icon};
use crate::dtmenu::{DtMenu, MenuCtx};
use crate::mail::{MailCtx, Mailer, Views};
use crate::message::{MessageKind, WindowTexture};
use crate::news::News;
use crate::view::View;

/// `ccSystem.frameRate` on the desktop: one vertical blank a frame
/// (`ccSetupDesktop` 0x00168320 calls `SetFrameRate(1)`).
pub const FRAME_RATE: u32 = 1;

/// A sound effect (`ccSeOn(n)`, the common bank).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Se(pub i32);

impl Se {
    /// `PlayOpening` starts with it.
    pub const OPENING_START: Se = Se(0);
    /// The opening's frame 60.
    pub const OPENING: Se = Se(1);
    /// The icon selection moved.
    pub const CURSOR: Se = Se(2);
    /// New mail on arrival at the main screen.
    pub const NEW_MAIL: Se = Se(3);
    /// A mode opened.
    pub const OPEN: Se = Se(4);
    /// A mode closed.
    pub const CLOSE: Se = Se(7);
    /// The World chosen.
    pub const WORLD: Se = Se(8);
}

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Se(Se),
    /// `ccSndChangeData(&Wave[n], -1)`: the desktop music.
    Bgm(usize),
    /// `ccSoundFadeOut()`.
    SoundFadeOut,
    /// `ccGame::ChangeRequest(num, sf)`: leave the desktop. The World
    /// icon asks for 4 (TOPPAGE) with 7.
    ChangeMode {
        num: i32,
        sf: i32,
    },
    /// `ccGame.enableReset`.
    EnableReset(bool),
    /// `ccSndChangeData(&Wave[wave], old)` from the Audio screen's
    /// `BgmRead` task: the desktop music changed from `old`.
    ChangeBgm {
        wave: usize,
        old: usize,
    },
    /// The Audio screen's movie: `ccSndMoviePlayer(0, bgm)`,
    /// `SetFrameRate(2)`, every task asleep while `SimplePlayStream(stream)`
    /// plays the stream, then `ccSndMoviePlayer(1, bgm)` and
    /// `SetFrameRate(1)`. The desktop's next step is the frame after.
    Movie {
        stream: i32,
        bgm: usize,
    },
    /// `ccSystem::SetDisplayOffset(x, y)` (Adjust Screen, every frame it is
    /// open): the picture moved by x (-48..48 in steps of 3, GS DISPLAY DX
    /// units) and y (-16..16 lines).
    DisplayOffset {
        x: i32,
        y: i32,
    },
    /// `ccSaveData::SetSoundEnv` (Sound): the volumes (0-256) and the
    /// output, 0 mono, 1 stereo.
    SoundEnv {
        main: i32,
        bgm: i32,
        se: i32,
        output: i32,
    },
    /// Vibration switched: `ccPad::actuaterSw = on`, and switching it on
    /// buzzes once (`SetActuater(pad, 1, 160, 200)`: the small motor, the
    /// big one at 160, for 12 ticks).
    Vibration {
        on: bool,
    },
    /// Controller: `setCameraCtrlType(t)` (the field camera's scheme, A-1,
    /// A-2, B-1, B-2).
    CameraType(i32),
    /// `ccEvVoiceStop()`: the confirm button cut a message's voice short.
    /// (`ccEvVoiceRequest(event, msg)` goes with the message itself, which
    /// the event engine opens.)
    VoiceStop,
    /// `ccBgmPlay(n)`: `VOICE/BGM.BIN` track `n` (the staff roll's is 1).
    BgmStream(i32),
    /// `ccBgmStop()`.
    BgmStreamStop,
}

/// `ccThStaffRoll` while it runs: its controller, the `Breath(2)` before the
/// first `Main`, and the page picture found for the texture last asked.
struct StaffRollTask {
    ctrl: staffroll::StaffRoll,
    wait: u8,
    picture: Option<(String, Option<(piney_draw::TexRef, i32)>)>,
}

/// Where the task is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The breath after SetStream / SetWall.
    Setup,
    /// PlayOpening's first breath, after `ccSeOn(0)`.
    OpeningStart,
    Opening,
    Main,
    /// Inside ChooseMode's loop for an icon.
    Mode(Icon),
    /// The World's fade-out, frames drawn so far.
    WorldFade(u32),
    /// ChangeRequest made; the mother task takes the desktop down.
    Left,
}

/// The screen fader's frames for the World (`EntryFade(20, ...)`).
pub const WORLD_FADE_FRAMES: u32 = 20;

/// The new-mail notice: frames up before it fades (`infoFlg` 200), and the
/// alpha step (`Desktop_control::DrawBack` 0x004025a4).
pub const NOTICE_HOLD: i32 = 200;
pub const NOTICE_STEP: i32 = 5;

/// A [`MailCtx`] from the desktop's fields, leaving the screens free to
/// borrow beside it.
macro_rules! mail_ctx {
    ($s:ident, $ctx:expr) => {
        MailCtx {
            ctx: $ctx,
            fonts: &$s.assets.fonts,
            names: $s.save.names(),
            save: &mut $s.save,
            req: &mut $s.requests,
            views: &mut $s.views,
            count: $s.count,
            frame_rate: $s.frame_rate,
        }
    };
}

/// The desktop.
pub struct Desktop {
    /// Kept for the modes that load more files (wallpapers).
    assets: Assets,
    save: SaveState,
    ctl: DesktopControl,
    mailer: Mailer,
    news: News,
    acces: Accessory,
    audio: Audio,
    data: Data,
    /// `dtMenu` (`ccThDtMenu`), with the event windows' `ccMsg`.
    menu: DtMenu,
    /// The desktop's last picture, which stays up while a menu freezes it.
    last: Option<Frame>,
    /// `ccGame.enableReset`, as the frames have set it.
    enable_reset: bool,
    views: Views,
    view: View,
    phase: Phase,
    requests: Vec<Request>,
    /// `ccSys.count`: frames since the desktop started (the game's counts
    /// from boot; only the cursor blink reads it).
    count: u32,
    /// The staff roll, while the event instruction `staff_roll` runs it.
    staff_roll: Option<StaffRollTask>,
    /// `ccSleepAllThread()`: every task but the menu's asleep, so nothing
    /// of the desktop is drawn (the layers flip to nothing), from the
    /// `staff_roll` instruction until its `ccWakeAllThread()`.
    slept: bool,
    /// `ccSystem.frameRate`: [`FRAME_RATE`] from the setup; the staff
    /// roll's `SetFrameRate(2)` for its save menus.
    frame_rate: u32,
}

impl Desktop {
    /// `ccSetupDesktop` (0x00168320) as far as the desktop goes: the
    /// overlay's tables, the scene, the wallpaper, the music.
    pub fn new(iso: &mut Iso, archive: Arc<Archive>, state: SaveState) -> piney_data::Result<Desktop> {
        let mut assets = Assets::read(iso, archive)?;
        let ctl = DesktopControl::new(&mut assets, &state)?;
        let mailer = Mailer::new(&assets, &state)?;
        let news = News::new(&assets)?;
        let acces = Accessory::new(&assets)?;
        let audio = Audio::new(&assets, &state)?;
        let data = Data::new(&assets)?;
        let control = WindowTexture::read_named(&assets.archive, dtmenu::CONTROL_FILE, dtmenu::CONTROL_TEX, None);
        let menu = DtMenu::new(assets.volume, WindowTexture::read(&assets.archive), control);
        let requests = vec![Request::Bgm(state.dt_bgm()), Request::EnableReset(true)];
        Ok(Desktop {
            assets,
            save: state,
            ctl,
            mailer,
            news,
            acces,
            audio,
            data,
            menu,
            last: None,
            enable_reset: true,
            views: Views::default(),
            view: View::default(),
            phase: Phase::Setup,
            requests,
            count: 0,
            staff_roll: None,
            slept: false,
            frame_rate: FRAME_RATE,
        })
    }

    /// The event instruction `staff_roll`: `ccThStaffRoll` started
    /// ([`staffroll`]): the controller over the volume's staff roll,
    /// `srand` of the frame count, the save's two names for `#0` and `#1`,
    /// and `ccBgmPlay(1)`. `ccRand` starts where a fresh boot leaves it.
    pub fn start_staff_roll(&mut self) -> piney_data::Result<()> {
        let tables = piney_data::tables::staffroll::of(self.assets.volume);
        let b = self.save.save.bytes();
        let name = |at: usize| -> Vec<u8> { b[at..at + 24].iter().take_while(|&&c| c != 0).copied().collect() };
        let names = (name(piney_data::save::offset::PL_NAME), name(piney_data::save::offset::PL_REAL_NAME));
        let ctrl = staffroll::StaffRoll::new(tables, self.count, staffroll::CcRand::seeded(0x1100), names);
        self.staff_roll = Some(StaffRollTask { ctrl, wait: 2, picture: None });
        self.requests.push(Request::BgmStream(1));
        Ok(())
    }

    /// `ccSleepAllThread()` (true) and `ccWakeAllThread()` (false) around
    /// the staff roll: while asleep the desktop's tasks draw nothing; the
    /// menu's task (and the roll's) go on.
    pub fn set_slept(&mut self, on: bool) {
        self.slept = on;
    }

    /// Whether the desktop's tasks sleep (see [`Desktop::set_slept`]).
    pub fn slept(&self) -> bool {
        self.slept
    }

    /// `ccSystem::SetFrameRate(rate)`: the staff roll's 2 before its save
    /// menus, which run at it (the select cursor's steps, the message
    /// window's button).
    pub fn set_frame_rate(&mut self, rate: u32) {
        self.frame_rate = rate.max(1);
    }

    /// Whether `ccThStaffRoll` still runs.
    pub fn staff_roll_running(&self) -> bool {
        self.staff_roll.is_some()
    }

    /// The staff roll's frame: after the first `Breath(2)`, `Main` once a
    /// frame, drawn on its layer; past the last page, `ccBgmStop` and the
    /// task ends.
    fn staff_roll_frame(&mut self, ctx: &mut Ctx, names: &kanji::Names) {
        let Some(t) = &mut self.staff_roll else { return };
        if t.wait > 0 {
            t.wait -= 1;
            return;
        }
        t.ctrl.main();
        if t.ctrl.done {
            self.requests.push(Request::BgmStreamStop);
            self.staff_roll = None;
            return;
        }
        let tex = t.ctrl.tex.clone();
        if t.picture.as_ref().is_none_or(|(name, _)| *name != tex) {
            let found = staffroll::picture_tex(&self.assets.archive, t.ctrl.tables.ccs, &tex);
            t.picture = Some((tex, found));
        }
        let pic = t.picture.as_ref().and_then(|(_, p)| p.as_ref());
        t.ctrl.render(ctx, &self.assets.fonts, names, pic);
    }

    /// `ccSystem.frameRate`: vertical blanks per game frame.
    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    /// The save record and the event members, as the desktop left them.
    pub fn state(&self) -> &SaveState {
        &self.save
    }

    /// For the event engine to write between frames (mail delivered, icons
    /// locked), as the game's scripts run beside the desktop task.
    pub fn state_mut(&mut self) -> &mut SaveState {
        &mut self.save
    }

    /// What the frames since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// The icon selected on the main screen (0-5, 6 and 7 while wrapping).
    pub fn selection(&self) -> i32 {
        self.ctl.dsel
    }

    /// The mailer, for inspection.
    pub fn mailer(&self) -> &Mailer {
        &self.mailer
    }

    /// News, for inspection.
    pub fn news(&self) -> &News {
        &self.news
    }

    /// The Accessory screen, for inspection.
    pub fn accessory(&self) -> &Accessory {
        &self.acces
    }

    /// The Audio screen, for inspection.
    pub fn audio(&self) -> &Audio {
        &self.audio
    }

    /// The Data screen, for inspection.
    pub fn data(&self) -> &Data {
        &self.data
    }

    /// The memory cards the Data screen saves to (none until given).
    pub fn set_card(&mut self, card: Box<dyn MemoryCard>) {
        self.data.set_card(card);
    }

    /// Where the title screen's load left `ccSaveSys` (`port`, `fileNum`):
    /// see [`Data::set_card_position`].
    pub fn set_card_position(&mut self, port: i32, file: i32) {
        self.data.set_card_position(port, file);
    }

    /// The mode open, if any.
    pub fn mode(&self) -> Option<Icon> {
        match self.phase {
            Phase::Mode(i) => Some(i),
            _ => None,
        }
    }

    /// One game frame: the menu task (`ccThDtMenu`), then the desktop task
    /// unless the menu has it asleep, its picture frozen while the menu
    /// holds the flip, then the menu layer (242) on top.
    pub fn step(&mut self, pad: &Pad) -> Frame {
        self.count = self.count.wrapping_add(1);
        // ccThSaveSys as the save menu's StartReq made it (priority 20,
        // before the menu task): its first frame only breathes.
        match self.menu.save_task {
            1 => self.menu.save_task = 2,
            2 if self.data.save_sys.running => self.data.save_task(&self.save.save),
            _ => {}
        }
        {
            let names = self.save.names();
            let mut x = MenuCtx {
                save: &mut self.save,
                req: &mut self.requests,
                pad,
                enable_reset: self.enable_reset,
                names,
                save_sys: Some(&mut self.data.save_sys),
            };
            self.menu.frame(&mut x);
        }
        // The desktop's input gates test CheckMenuType() == -1.
        let desk_pad = if self.menu.idle() { *pad } else { Pad::default() };
        let mut out = if self.slept {
            Frame::new()
        } else if self.menu.asleep {
            match self.last.clone() {
                Some(f) => f,
                None => Frame::new(),
            }
        } else {
            let f = self.desktop_frame(&desk_pad);
            match (&self.last, self.menu.still) {
                (Some(l), true) => l.clone(),
                _ => {
                    self.last = Some(f.clone());
                    f
                }
            }
        };
        let mut ctx = Ctx::new(self.view.clone());
        ctx.uploads = std::mem::take(&mut out.uploads);
        let names = self.save.names();
        self.menu.disp(&mut ctx, &self.assets.fonts, &names, self.frame_rate);
        self.staff_roll_frame(&mut ctx, &names);
        let m = ctx.finish();
        out.uploads = m.uploads;
        out.cmds.extend(m.cmds);
        if let Some(on) = self.requests.iter().rev().find_map(|r| match r {
            Request::EnableReset(on) => Some(*on),
            _ => None,
        }) {
            self.enable_reset = on;
        }
        out
    }

    // --- The seam to the event engine's windows (`piney_event::host::Host`) ---

    /// `Host::message_open` on the desktop: when no menu is open, ask for
    /// the fade menu (`openReqNum = 7`), then `ccMsg->Change` (speech) or
    /// `ChangeInfo` (information; `info_now` without its frame). `emode`
    /// is the record's (0 a plain line, 2 kept open, 3 a question); `lines`
    /// are its up to three lines.
    pub fn open_message(&mut self, kind: MessageKind, emode: i32, name: Option<&[u8]>, lines: &[&[u8]]) {
        let names = self.save.names();
        dtmenu::open_message(&mut self.menu, kind, emode, name, lines, &names);
    }

    /// `Host::message_check`: `ccMsg->Check(0)` with this frame's pad: 0
    /// while the window waits; 1 when it has closed; for a question the
    /// answer, 1 or 2.
    pub fn message_check(&mut self, pad: &Pad) -> i32 {
        let ok = self.save.ok();
        self.menu.msg.check(pad.push.bits(), pad.repeat.bits(), ok, &mut self.requests)
    }

    /// `Host::message_close`: `ccMsg->Close()`.
    pub fn close_message(&mut self) {
        self.menu.msg.close();
    }

    /// `Host::desktop_message_done`: `dtMenu +0x10 = 1`, which closes the
    /// fade menu.
    pub fn message_done(&mut self) {
        self.menu.extern_flag = true;
    }

    /// `Host::desktop_menu`: (`CheckMenuType()`, `openReqNum`).
    pub fn menu_state(&self) -> (i32, i16) {
        (self.menu.check_menu_type(), self.menu.open_req)
    }

    /// `Host::desktop_menu_open`: `openReqNum = num`.
    pub fn open_menu(&mut self, num: i16) {
        self.menu.open_req = num;
    }

    /// The staff roll's `dtMenu +0x16` (START does nothing while set).
    pub fn set_menu_forbid(&mut self, on: bool) {
        self.menu.forbid = on;
    }

    /// The menu task, for inspection.
    pub fn menu(&self) -> &DtMenu {
        &self.menu
    }

    /// The desktop task's pass (`ccThDesktop` between breaths).
    fn desktop_frame(&mut self, pad: &Pad) -> Frame {
        let mut ctx = Ctx::new(self.view.clone());
        let push = pad.push.bits();
        match self.phase {
            Phase::Setup => self.phase = Phase::OpeningStart,
            Phase::OpeningStart => {
                self.requests.push(Request::Se(Se::OPENING_START));
                self.phase = Phase::Opening;
            }
            Phase::Opening => {
                if self.ctl.opening_frame(&mut ctx, &self.save, push, &mut self.requests) {
                    self.add_all_list();
                    self.phase = Phase::Main;
                }
            }
            Phase::Main => {
                self.ctl.select_mode(&mut self.save, push, &mut self.requests);
                if let Some(icon) = Icon::from_index(self.ctl.app) {
                    self.enter(icon, pad, &mut ctx);
                } else {
                    self.main_frame(&mut ctx);
                }
            }
            Phase::Mode(icon) => self.mode_frame(icon, pad, &mut ctx),
            Phase::WorldFade(k) => self.world_frame(k, &mut ctx),
            Phase::Left => fade::draw(&mut ctx, WORLD_FADE_FRAMES, WORLD_FADE_FRAMES),
        }
        self.view = ctx.view.clone();
        ctx.finish()
    }

    /// `Desktop_control::AddAllList` (0x00402030): the inbox (a mail never
    /// seen before raises the notice with sound 3), then the headlines.
    fn add_all_list(&mut self) {
        self.ctl.new_mail = self.mailer.add_mail_list(&mut self.save);
        if self.ctl.new_mail != 0 {
            self.ctl.alpha = 0;
            self.ctl.info_flg = 0;
            self.requests.push(Request::Se(Se::NEW_MAIL));
        }
        self.news.add_html_list(&self.save);
        self.acces.add_wall_list(&self.save);
        // The tables were read when the desktop was built; this cannot fail.
        let _ = self.audio.add_lists(&self.save);
    }

    /// The two NEW marks: unread mail, unread news.
    fn new_marks(&self) -> (bool, bool) {
        (self.mailer.new_mail_num > 0, self.news.new_web_num > 0)
    }

    fn mail_ctx<'a>(&'a mut self, ctx: &'a mut Ctx) -> (MailCtx<'a>, &'a mut Mailer) {
        (mail_ctx!(self, ctx), &mut self.mailer)
    }

    /// `Desktop_control::DrawBack` (0x00402300) whole: the scene, then the
    /// new-mail notice while it is up.
    fn draw_back(&mut self, ctx: &mut Ctx) {
        let (new_mail, new_news) = self.new_marks();
        self.ctl.draw_back(ctx, new_mail, new_news);
        self.notice(ctx);
    }

    fn notice(&mut self, ctx: &mut Ctx) {
        if self.ctl.new_mail == 0 {
            return;
        }
        if self.ctl.info_flg > NOTICE_HOLD {
            if self.ctl.new_mail == 1 {
                let a = self.ctl.alpha;
                self.mailer.set_a_info(a);
                self.ctl.alpha -= NOTICE_STEP;
            }
            if self.ctl.alpha <= 0 {
                self.ctl.new_mail = 0;
            }
        }
        if self.ctl.info_flg < NOTICE_HOLD && self.ctl.new_mail == 1 && self.ctl.alpha < 128 {
            self.ctl.alpha = (self.ctl.alpha + NOTICE_STEP).min(128);
            let a = self.ctl.alpha;
            self.mailer.set_a_info(a);
        }
        let (mut x, mailer) = self.mail_ctx(ctx);
        self.ctl.info_flg = mailer.info_window(&mut x);
    }

    /// ChooseMode's default: `check = DrawLogo(); DrawBack(); App = -1`.
    fn main_frame(&mut self, ctx: &mut Ctx) {
        self.ctl.check = self.ctl.draw_logo(ctx);
        self.draw_back(ctx);
        self.ctl.app = -1;
    }

    /// ChooseMode entering an icon's loop, and its first pass.
    fn enter(&mut self, icon: Icon, pad: &Pad, ctx: &mut Ctx) {
        match icon {
            Icon::World => {
                self.requests.push(Request::EnableReset(false));
                self.requests.push(Request::Se(Se::WORLD));
                self.requests.push(Request::SoundFadeOut);
                self.world_frame(0, ctx);
                return;
            }
            Icon::Mail => {
                if self.ctl.draw_flg == 1 {
                    self.ctl.draw_flg = 0;
                }
                self.mailer.set_back(&mut self.views);
                self.requests.push(Request::Se(Se::OPEN));
            }
            Icon::News => {
                if self.ctl.draw_flg == 1 {
                    self.ctl.draw_flg = 0;
                }
                let mut x = mail_ctx!(self, ctx);
                self.news.set_data(&mut x);
                self.requests.push(Request::Se(Se::OPEN));
            }
            Icon::Audio => {
                if self.ctl.draw_flg == 1 {
                    self.ctl.draw_flg = 0;
                }
                let mut x = mail_ctx!(self, ctx);
                self.audio.control_data_set(&mut x);
                self.requests.push(Request::Se(Se::OPEN));
            }
            Icon::Wallpaper => {
                let mut x = mail_ctx!(self, ctx);
                self.acces.control_data_set(&mut x);
                self.requests.push(Request::Se(Se::OPEN));
                if self.ctl.draw_flg == 1 {
                    self.ctl.draw_flg = 0;
                }
            }
            Icon::Data => {
                if self.ctl.draw_flg == 1 {
                    self.ctl.draw_flg = 0;
                }
                self.requests.push(Request::EnableReset(false));
                self.requests.push(Request::Se(Se::OPEN));
                let mut x = mail_ctx!(self, ctx);
                self.data.enter(&mut x);
            }
        }
        self.phase = Phase::Mode(icon);
        self.mode_frame(icon, pad, ctx);
    }

    /// One pass of an icon's loop: the mode's main, then the draws in
    /// ChooseMode's order (the mode's `DrawBack` before the desktop's,
    /// but after it for the Accessory).
    fn mode_frame(&mut self, icon: Icon, pad: &Pad, ctx: &mut Ctx) {
        let r = {
            let mut x = mail_ctx!(self, ctx);
            match icon {
                Icon::Mail => self.mailer.main(&mut x, pad),
                Icon::News => self.news.main(&mut x, pad),
                Icon::Wallpaper => self.acces.main(&mut x, pad),
                Icon::Audio => self.audio.main(&mut x, pad),
                Icon::Data => self.data.main(&mut x, pad),
                // Not ported: the page shows until cancel, as if its screen
                // closed at once.
                _ => {
                    if pad.push.bits() & x.save.cancel() != 0 {
                        -1
                    } else {
                        0
                    }
                }
            }
        };
        if r == -1 {
            self.requests.push(Request::Se(Se::CLOSE));
            if icon == Icon::Data {
                self.requests.push(Request::EnableReset(true));
                self.data.end_req();
            }
            self.phase = Phase::Main;
            self.main_frame(ctx);
            return;
        }
        {
            let mut x = mail_ctx!(self, ctx);
            match icon {
                Icon::Mail => self.mailer.draw_back(&mut x),
                Icon::News => self.news.draw_back(&mut x),
                Icon::Audio => self.audio.draw_back(&mut x),
                Icon::Data => self.data.draw_back(&mut x),
                _ => {}
            }
        }
        if let Some(w) = self.acces.take_swap() {
            // ChangeWall: ScrBack->SetAnm(the new file, AnmName) and one step.
            if let Ok(file) = self.assets.file(&w.name) {
                let name = w.anm_name.clone();
                self.ctl.scr_back.set(&file, &name);
                self.ctl.scr_back.forward();
            }
        }
        let (new_mail, new_news) = self.new_marks();
        match icon {
            Icon::Mail => self.ctl.draw_mailer_back(ctx, new_mail, new_news),
            Icon::News => self.ctl.draw_page(ctx, icon, new_mail, new_news, self.news.closing()),
            _ => self.ctl.draw_page(ctx, icon, new_mail, new_news, false),
        }
        // The new-mail notice, from inside Desktop_control::DrawBack.
        self.notice(ctx);
        self.audio.draw_fade(ctx);
        if icon == Icon::Wallpaper {
            let mut x = mail_ctx!(self, ctx);
            self.acces.draw_back(&mut x);
        }
    }

    /// The World: `while CheckFade: DrawLogo; DrawBack; breathe`, the fade
    /// drawn over each of those 20 frames; then `ChangeRequest(4, 7)` and
    /// the desktop draws nothing more (the held fade leaves black).
    fn world_frame(&mut self, k: u32, ctx: &mut Ctx) {
        if k < WORLD_FADE_FRAMES {
            self.ctl.draw_logo(ctx);
            self.draw_back(ctx);
            fade::draw(ctx, k, WORLD_FADE_FRAMES);
            self.phase = Phase::WorldFade(k + 1);
        } else {
            fade::draw(ctx, WORLD_FADE_FRAMES, WORLD_FADE_FRAMES);
            if self.phase != Phase::Left {
                self.requests.push(Request::ChangeMode { num: 4, sf: 7 });
            }
            self.ctl.app = -1;
            self.phase = Phase::Left;
        }
    }
}
