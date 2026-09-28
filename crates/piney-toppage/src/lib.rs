//! The World's top page and its bulletin board in .hack//Infection
//! (`DATA/TOPPAGE.PRG`, mode 4), as a state machine: a pad in, a
//! [`piney_draw::Frame`] out, once per game frame
//! (`docs/engine/toppage.md`).
//!
//! The desktop's "The World" icon asks for it (`ChangeRequest(4, 7)`).
//! `ccSetupToppage` (0x00168570) loads the overlay and `xdttopen0`, loads
//! the board's music bank (`ccSndSQLoad(0)`), sets one vertical blank a
//! frame, and starts two tasks: the system menu `ccThDtMenu` and
//! `ccThToppage` (toppage.prg 0x004008e0), which builds a
//! `ccThToppageCtrl` ([`control`]) and runs its `Main` once a frame; then
//! the music starts (`ccSndBgmCtrl`).
//!
//! [`TopPage::step`] is one frame: the menu task, the top page's task,
//! then the fader and the menu on top. The content (the scene, the board's
//! posts, the fonts) is read from the disc; the game state the event
//! scripts own is [`SaveState`]. The ways out are requests: Log out goes
//! back to the desktop (`ChangeMode { num: 3, sf: 7 }`), Log in on to the
//! field game (`ChangeMode { num: 5, sf: 8 }`, `ChangeArea { area: 0, town:
//! lastTown }` and the `ChangeMode { num: 6, sf: 7 }` that makes).

pub mod assets;
pub mod bbs;
pub mod content;
pub mod control;
pub mod draw;
pub mod scene;
#[cfg(feature = "trace")]
pub mod trace;
pub mod util;

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_demo::fade::ScFade;
use piney_desktop::anm::Ctx;
use piney_desktop::dtmenu::{self, DtMenu, MenuCtx};
use piney_desktop::message::{MessageKind, WindowTexture};
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_input::Pad;

pub use piney_desktop::SaveState;

use crate::assets::Assets;
use crate::control::{Env, TopPageCtrl};
use crate::scene::Scene;
use crate::util::KeyRepeat;

/// `ccSystem.frameRate` on the top page: `ccSetupToppage` calls
/// `SetFrameRate(1)`.
pub const FRAME_RATE: u32 = 1;
/// `ccGame.status` on the top page (`ccSetupToppage` 0x00168588).
pub const STATUS: i32 = 3;
/// `ccSndSQLoad(0)`: the board's music bank (`sqDataToppage`).
pub const BANK: i32 = 0;

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// `ccSeOn(n)`, the common bank.
    Se(i32),
    /// `ccAllSoundOff()`: `ccSetupToppage`'s first sound call.
    AllSoundOff,
    /// `ccSndSQLoad(n)`: the context's music bank; the top page loads 0,
    /// `sqDataToppage`, and nothing plays yet.
    SqLoad(i32),
    /// Sequence `n` of the loaded bank starts: `ccSndBgmCtrl`'s board case
    /// (`sqLoadParam` 0, 0x0017b9c0) plays sequence 0 as `ccSqPlay(0)`
    /// does, once `ccSnd.gameStart` is set.
    SqPlay(i32),
    /// `ccSoundFadeOut()`: every sequence to silence over 8 frames and
    /// stopped (Log in and Log out; the board keeps the music).
    SoundFadeOut,
    /// `ccGame.enableReset`: 1 when the top page starts, 0 once Log in or
    /// Log out is chosen (the system menu's START test reads it).
    EnableReset(bool),
    /// `ccGame::ChangeRequest(num, sf)`: leave the top page. Log out asks
    /// for 3 (the desktop) with 7; Log in for 5 (`ccSetupNewGame`,
    /// GCMN.PRG) with 8, then its `ChangeArea` asks for 6 with 7.
    ChangeMode { num: i32, sf: i32 },
    /// `ccGame::ChangeArea(area, town)`: Log in's scene, area 0 (a town)
    /// and `town` = `saveData.lastTown` (+0x8426). See [`Scene::login`]
    /// for what it leaves in `ccGame` (the town's server from `@1489`,
    /// field, dungeon, floor and block -1); the `ChangeMode { 6, 7 }` it
    /// makes follows it in the list.
    ChangeArea { area: i32, town: i32 },
    /// What the system menu's options ask of the game, as the desktop's
    /// menu asks it (`DisplayOffset`, `SoundEnv`, `Vibration`,
    /// `CameraType`, `VoiceStop`). Its sounds come as [`Request::Se`], its
    /// Title Screen as [`Request::ChangeMode`] (1, 7).
    Menu(piney_desktop::Request),
}

/// Where the top page's task is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The breath after the constructor: nothing drawn yet.
    Setup,
    /// `Main` once a frame.
    Running,
    /// A `ChangeRequest` made: the task sleeps and the picture is held
    /// (`sf` 7 turns the flip off) until the next mode takes over.
    Left,
}

/// The top page.
pub struct TopPage {
    assets: Assets,
    save: SaveState,
    ctl: TopPageCtrl,
    /// `dtMenu` (`ccThDtMenu`), with the event windows' `ccMsg`.
    menu: DtMenu,
    /// `scFadeDef`, cleared by `ccSetupToppage`.
    fade: ScFade,
    keys: KeyRepeat,
    /// The task's last picture, which stays up while a menu freezes it.
    last: Option<Frame>,
    /// The whole last frame, held once the top page has handed off.
    held: Option<Frame>,
    enable_reset: bool,
    view: View,
    phase: Phase,
    requests: Vec<Request>,
    /// Frames since the top page started.
    count: u32,
}

impl TopPage {
    /// `ccSetupToppage` (0x00168570) and the task's constructor: the
    /// overlay's tables, the scene, the board built from `state`, the music
    /// asked for. `g_TP_pushCntr` starts at 0; see [`TopPage::set_keys`].
    pub fn new(iso: &mut Iso, archive: Arc<Archive>, state: SaveState) -> piney_data::Result<TopPage> {
        let mut state = state;
        let assets = Assets::read(iso, archive)?;
        let ctl = TopPageCtrl::new(assets.bbs.file.clone(), assets.neutral.clone(), &assets.bbs, &mut state);
        let control = WindowTexture::read_named(&assets.archive, dtmenu::CONTROL_FILE, dtmenu::CONTROL_TEX, None);
        let menu = DtMenu::new(assets.disc, WindowTexture::read(&assets.archive), control);
        let requests =
            vec![Request::AllSoundOff, Request::SqLoad(BANK), Request::EnableReset(true), Request::SqPlay(0)];
        Ok(TopPage {
            assets,
            save: state,
            ctl,
            menu,
            fade: ScFade::default(),
            keys: KeyRepeat::default(),
            last: None,
            held: None,
            enable_reset: true,
            view: View::default(),
            phase: Phase::Setup,
            requests,
            count: 0,
        })
    }

    /// `ccSystem.frameRate`: vertical blanks per game frame.
    pub fn frame_rate(&self) -> u32 {
        FRAME_RATE
    }

    /// The save record and the event members, as the top page left them.
    pub fn state(&self) -> &SaveState {
        &self.save
    }

    /// For the event engine to write between frames (posts made, commands
    /// locked), as the game's scripts run beside the top page's task.
    pub fn state_mut(&mut self) -> &mut SaveState {
        &mut self.save
    }

    /// What the frames since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// Where the task is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// `ccThToppageCtrl`, for inspection.
    pub fn control(&self) -> &TopPageCtrl {
        &self.ctl
    }

    /// The menu task, for inspection.
    pub fn menu(&self) -> &DtMenu {
        &self.menu
    }

    /// `g_TP_pushCntr` / `g_TP_pushFlag`: main-executable globals the last
    /// visit may have left counting.
    pub fn keys(&self) -> KeyRepeat {
        self.keys
    }

    pub fn set_keys(&mut self, k: KeyRepeat) {
        self.keys = k;
    }

    /// `@1489`, the server of each town, for [`Scene::login`].
    pub fn servers(&self) -> [i32; 8] {
        self.assets.servers
    }

    /// The scene [`Request::ChangeArea`] leaves in `ccGame`.
    pub fn login_scene(&self, town: i32) -> Scene {
        Scene::login(town, &self.assets.servers)
    }

    /// A line for a window title.
    pub fn status(&self) -> String {
        format!(
            "top page - mode {} - command {} - board page {} - frame {}",
            self.ctl.mode, self.ctl.cmd, self.ctl.bbs.draw_state, self.count
        )
    }

    /// One game frame: the menu task (`ccThDtMenu`, priority 33, started
    /// first), then the top page's task unless the menu has it asleep, its
    /// picture frozen while the menu holds the flip; then the fader (font
    /// layer 240) and the menu layer (242) on top.
    pub fn step(&mut self, pad: &Pad) -> Frame {
        self.count = self.count.wrapping_add(1);
        if let Some(f) = &self.held {
            return f.clone();
        }
        let asked = self.requests.len();
        let mut menu_req = Vec::new();
        {
            let names = self.save.names();
            let mut x = MenuCtx {
                save: &mut self.save,
                req: &mut menu_req,
                pad,
                enable_reset: self.enable_reset,
                names,
                save_sys: None,
            };
            self.menu.frame(&mut x);
        }
        self.menu_requests(menu_req);
        let mut out = if self.menu.asleep {
            // Frame::new, not Default: the frame's size is set.
            match self.last.clone() {
                Some(f) => f,
                None => Frame::new(),
            }
        } else {
            let f = self.task_frame(pad);
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
        self.fade.send(&mut ctx);
        let names = self.save.names();
        self.menu.disp(&mut ctx, &self.assets.fonts, &names, FRAME_RATE);
        let m = ctx.finish();
        out.uploads = m.uploads;
        out.cmds.extend(m.cmds);
        if let Some(on) = self.requests.iter().rev().find_map(|r| match r {
            Request::EnableReset(on) => Some(*on),
            _ => None,
        }) {
            self.enable_reset = on;
        }
        if self.requests[asked..].iter().any(|r| matches!(r, Request::ChangeMode { .. })) {
            self.phase = Phase::Left;
        }
        if self.phase == Phase::Left {
            // ~ccThToppageCtrl comes with the next mode's setup; its
            // ~ccThBBSCtrl writes back the posts read.
            self.ctl.bbs.release(&mut self.save);
            self.held = Some(out.clone());
        }
        out
    }

    fn menu_requests(&mut self, v: Vec<piney_desktop::Request>) {
        for r in v {
            self.requests.push(match r {
                piney_desktop::Request::Se(s) => Request::Se(s.0),
                piney_desktop::Request::ChangeMode { num, sf } => Request::ChangeMode { num, sf },
                piney_desktop::Request::EnableReset(on) => Request::EnableReset(on),
                other => Request::Menu(other),
            });
        }
    }

    /// The top page's task between two breaths.
    fn task_frame(&mut self, pad: &Pad) -> Frame {
        let mut ctx = Ctx::new(self.view.clone());
        match self.phase {
            Phase::Setup => self.phase = Phase::Running,
            Phase::Running => {
                let names = self.save.names();
                let mut x = Env {
                    ctx: &mut ctx,
                    fonts: &self.assets.fonts,
                    names: &names,
                    save: &mut self.save,
                    pad,
                    keys: &mut self.keys,
                    menu_type: self.menu.check_menu_type(),
                    fade: &mut self.fade,
                    req: &mut self.requests,
                };
                self.ctl.main(&self.assets.bbs, &mut x);
            }
            Phase::Left => {}
        }
        self.view = ctx.view.clone();
        ctx.finish()
    }

    // --- The seam to the event engine's windows, as the desktop's ---

    /// `Host::message_open` on the top page: the fade menu (7) when none is
    /// open, then `ccMsg->Change` / `ChangeInfo`.
    pub fn open_message(&mut self, kind: MessageKind, emode: i32, name: Option<&[u8]>, lines: &[&[u8]]) {
        let names = self.save.names();
        dtmenu::open_message(&mut self.menu, kind, emode, name, lines, &names);
    }

    /// `Host::message_check`: `ccMsg->Check(0)` with this frame's pad.
    pub fn message_check(&mut self, pad: &Pad) -> i32 {
        let ok = self.save.ok();
        let mut req = Vec::new();
        let r = self.menu.msg.check(pad.push.bits(), pad.repeat.bits(), ok, &mut req);
        self.menu_requests(req);
        r
    }

    /// `Host::message_close`: `ccMsg->Close()`.
    pub fn close_message(&mut self) {
        self.menu.msg.close();
    }

    /// `Host::desktop_message_done`: `dtMenu +0x10 = 1`.
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
}
