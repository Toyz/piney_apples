//! `ccThToppageCtrl` (toppage.cpp, toppage.prg 0x00400880-0x00401728): The
//! World's top page, a menu of three commands over an animated scene, and
//! the way into the board and out to the field or the desktop.
//!
//! `Main` (0x00400f50) sets the view from the camera animation, then runs
//! the mode:
//!
//! ```text
//! 0 _Enter   the log-in animation; at its end (or cancel) a flash, mode 1
//! 1 _Normal  the menu: up / down pick Log in, BBS, Log out; OK leaves
//! 2 _Exit    30 frames of fade to black, then: Log in ChangeRequest(5, 8)
//!            and ChangeArea(0, lastTown); Log out ChangeRequest(3, 7);
//!            BBS mode 3 and the board built
//! 3 _BBS     the board; when it is left, the scene again and mode 1
//! ```

use std::rc::Rc;

use piney_demo::fade::ScFade;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::{Fonts, Names};
use piney_input::{Buttons, Pad};

use crate::Request;
use crate::bbs::{Bbs, BbsAssets, BbsEnv, check_new_message};
use crate::draw::Anim;
use crate::util::KeyRepeat;

/// The scene's animations in `xdttopen0` (`Init` 0x00400c60).
pub const ANM_ENTER: &str = "ANM_xdttopst";
pub const ANM_NEUTRAL_INIT: &str = "ANM_xdttop1a";
pub const ANM_LOGIN: &str = "ANM_xdtlog1a";
pub const ANM_BBS: &str = "ANM_xdtbbs1a";
pub const ANM_BBS_NEW: &str = "ANM_xdtbbs2a";
pub const ANM_NEW: &str = "ANM_xdtnew1a";
pub const ANM_QUIT: &str = "ANM_xdtqui1a";
pub const ANM_CAMERA: &str = "ANM_xdtcame0";

/// `m_mode`.
pub const MODE_ENTER: i32 = 0;
pub const MODE_NORMAL: i32 = 1;
pub const MODE_EXIT: i32 = 2;
pub const MODE_BBS: i32 = 3;

/// `m_cmd`: the menu's commands.
pub const CMD_LOGIN: i32 = 0;
pub const CMD_BBS: i32 = 1;
pub const CMD_QUIT: i32 = 2;

/// The sounds: the log-in animation's frames 1 and 60, a cursor move, Log
/// in, and BBS / Log out.
pub const SE_ENTER_1: i32 = 9;
pub const SE_ENTER_60: i32 = 10;
pub const SE_MOVE: i32 = 6;
pub const SE_LOGIN: i32 = 13;
pub const SE_DECIDE: i32 = 4;

/// `ccEvent::CheckOperate(cmd + 6)` and `(cmd + 25)`: the events lock the
/// commands as operations 6-8 and 25-27.
pub const OPERATE_BASE: [i32; 2] = [6, 25];

/// `EntryFlash(30, 0x80e0ffff, 0, 0, 512, 384)`: a pale flash fading out
/// over 31 frames, entering the menu from the log-in or the board.
pub const FLASH: (i16, u32) = (30, 0x80e0_ffff);
/// `EntryFlash2(30, 1, 0x80000000, 0, 0, 512, 384)`: to black over 31
/// frames, then out over 2, on a command.
pub const FLASH_OUT: (i16, i16, u32) = (30, 1, 0x8000_0000);

/// `_Exit`: frames counted before the command's hand-off (`m_actCount`
/// reaching 30 after its increment test).
pub const EXIT_FRAMES: i32 = 30;

/// Where the top page hands off.
pub const REQUEST_FIELD: (i32, i32) = (5, 8);
pub const REQUEST_DESKTOP: (i32, i32) = (3, 7);
/// `ccGame::ChangeScene`'s request after the scene is set: mode 6,
/// `ccSetupGameCtrl`.
pub const REQUEST_GAME_CTRL: (i32, i32) = (6, 7);

/// The eight `ccAnm`s `new`ed by the constructor, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnmId {
    Enter = 0,
    Neutral,
    Login,
    Bbs,
    Quit,
    Camera,
    BbsNew,
    New,
}

const LABELS: [&str; 8] = ["enter", "neutral", "login", "bbs", "quit", "camera", "bbsnew", "new"];

/// What a frame needs of the rest of the game.
pub struct Env<'a> {
    pub ctx: &'a mut Ctx,
    pub fonts: &'a Fonts,
    pub names: &'a Names,
    /// `saveData`, and the event manager's operate bits.
    pub save: &'a mut SaveState,
    pub pad: &'a Pad,
    /// `g_TP_pushFlag`, `g_TP_pushCntr`.
    pub keys: &'a mut KeyRepeat,
    /// `ccDtMenu::CheckMenuType()`: -1 when no menu is open (the board
    /// takes OK and cancel only then).
    pub menu_type: i32,
    /// `scFadeDef`.
    pub fade: &'a mut ScFade,
    pub req: &'a mut Vec<Request>,
}

/// `ccScFade::EntryFlash(t, c, 0, 0, 512, 384)`.
fn entry_flash(fade: &mut ScFade, t: i16, c: u32) {
    #[cfg(feature = "trace")]
    crate::trace::record("flash", "fade", || format!("{t} {c:08x}"));
    fade.entry_flash(t, c);
}

/// `ccScFade::EntryFlash2(t0, t1, c, 0, 0, 512, 384)` (0x001602d0): status 5,
/// in from clear to `c` over `t0`, then out over `t1`.
fn entry_flash2(fade: &mut ScFade, t0: i16, t1: i16, c: u32) {
    #[cfg(feature = "trace")]
    crate::trace::record("flash2", "fade", || format!("{t0} {t1} {c:08x}"));
    use piney_demo::fade::{DRAW, Element, THEN_OUT};
    if let Some(e) = fade.elm.iter_mut().find(|e| e.status == 0) {
        *e = Element {
            status: DRAW | THEN_OUT,
            tcnt: t0,
            tcnt1: t1,
            cnt: 0,
            col0: c & 0x00ff_ffff,
            col1: c,
            ..Element::default()
        };
    }
}

/// `ccThToppageCtrl` (0x44 bytes).
pub struct TopPageCtrl {
    file: Rc<SceneFile>,
    /// `@1088[volumeNum - 1]`: the menu's idle animation, `ANM_xdttop1a` on
    /// Infection (volumeNum 1).
    neutral: String,
    anms: Vec<Anim>,
    /// `m_anmCommand`: which of Login, Bbs, BbsNew, Quit the command draws.
    pub command: Option<AnmId>,
    pub mode: i32,
    /// `m_exit`: nothing sets it; the task would stop on it.
    pub exit: i32,
    pub cmd: i32,
    pub act_proccess: i32,
    pub act_count: i32,
    /// `m_bBBSNew`: a post is new, so the BBS command shows its NEW mark.
    pub bbs_new: bool,
    /// `m_bc`.
    pub bbs: Bbs,
}

impl TopPageCtrl {
    /// `ccThToppageCtrl::ccThToppageCtrl` (0x004009c0): the animations, the
    /// board (`ccThBBSCtrl::Init`), `Init`, `_ChangeMode(0)`,
    /// `_SetCommand(0)`.
    pub fn new(file: Rc<SceneFile>, neutral: String, a: &BbsAssets, save: &mut SaveState) -> Self {
        let mut c = TopPageCtrl {
            file,
            neutral,
            anms: LABELS.iter().map(|l| Anim::new(l)).collect(),
            command: None,
            mode: 0,
            exit: 0,
            cmd: 0,
            act_proccess: 0,
            act_count: 0,
            bbs_new: false,
            bbs: Bbs::new(),
        };
        c.bbs.init(a, save);
        c.exit = 0;
        c.init();
        c.change_mode(MODE_ENTER, a, save, None);
        c.set_command(CMD_LOGIN);
        c
    }

    fn anm(&mut self, id: AnmId) -> &mut Anim {
        &mut self.anms[id as usize]
    }

    /// The animation `id`, for inspection.
    pub fn anim(&self, id: AnmId) -> &Anim {
        &self.anms[id as usize]
    }

    /// `Init` (0x00400c60): every animation set, the camera stepped once.
    fn init(&mut self) {
        let f = self.file.clone();
        for (id, name) in [
            (AnmId::Enter, ANM_ENTER),
            (AnmId::Neutral, ANM_NEUTRAL_INIT),
            (AnmId::Login, ANM_LOGIN),
            (AnmId::Bbs, ANM_BBS),
            (AnmId::BbsNew, ANM_BBS_NEW),
            (AnmId::New, ANM_NEW),
            (AnmId::Quit, ANM_QUIT),
            (AnmId::Camera, ANM_CAMERA),
        ] {
            self.anm(id).set(&f, name);
        }
        self.anm(AnmId::Camera).step();
    }

    /// `_ChangeMode(mode)` (0x00400da0): the idle animation restarts; the
    /// menu takes the board's NEW state; a command fades out (and the
    /// music, unless it is the board).
    fn change_mode(
        &mut self,
        mode: i32,
        a: &BbsAssets,
        save: &SaveState,
        out: Option<(&mut ScFade, &mut Vec<Request>)>,
    ) {
        self.mode = mode;
        self.act_proccess = 0;
        self.act_count = 0;
        let (f, n) = (self.file.clone(), self.neutral.clone());
        self.anm(AnmId::Neutral).set(&f, &n);
        if mode == MODE_NORMAL {
            self.bbs_new = check_new_message(a.threads(save), save);
        } else if mode == MODE_EXIT
            && let Some((fade, req)) = out
        {
            if self.cmd != CMD_BBS {
                req.push(Request::SoundFadeOut);
            }
            let (t0, t1, c) = FLASH_OUT;
            entry_flash2(fade, t0, t1, c);
        }
    }

    /// `_SetCommand(cmd)` (0x00400ed0) and `_Normal`'s copy of it: the
    /// command's animation.
    fn set_command(&mut self, cmd: i32) {
        self.cmd = cmd;
        self.command_for(cmd);
    }

    fn command_for(&mut self, cmd: i32) {
        match cmd {
            CMD_LOGIN => self.command = Some(AnmId::Login),
            CMD_BBS => self.command = Some(if self.bbs_new { AnmId::BbsNew } else { AnmId::Bbs }),
            CMD_QUIT => self.command = Some(AnmId::Quit),
            _ => {}
        }
    }

    fn step_draw(&mut self, id: AnmId, ctx: &mut Ctx) -> bool {
        let r = self.anm(id).step();
        self.anm(id).draw(ctx);
        r
    }

    /// `Main` (0x00400f50): one frame.
    pub fn main(&mut self, a: &BbsAssets, x: &mut Env) {
        #[cfg(feature = "trace")]
        crate::trace::record("view", "camera", String::new);
        let cam = self.anms[AnmId::Camera as usize].anm.clone();
        x.ctx.set_view(&cam);
        match self.mode {
            MODE_ENTER => self.enter(a, x),
            MODE_NORMAL => self.normal(a, x),
            MODE_EXIT => self.exit_mode(a, x),
            MODE_BBS => self.bbs_mode(a, x),
            _ => {}
        }
    }

    /// `_Enter` (0x00401010): the log-in animation, sounds 9 and 10 at its
    /// frames 1 and 60; at its end, or on cancel, the flash and the menu.
    fn enter(&mut self, a: &BbsAssets, x: &mut Env) {
        let ended = self.step_draw(AnmId::Enter, x.ctx);
        let f = self.anim(AnmId::Enter).frame_now();
        if f == 1 {
            x.req.push(Request::Se(SE_ENTER_1));
        } else if f == 60 {
            x.req.push(Request::Se(SE_ENTER_60));
        } else if ended || x.pad.push.bits() & x.save.cancel() != 0 {
            let (t, c) = FLASH;
            entry_flash(x.fade, t, c);
            self.change_mode(MODE_NORMAL, a, x.save, None);
        }
    }

    /// `_Normal` (0x00401130): the menu.
    fn normal(&mut self, a: &BbsAssets, x: &mut Env) {
        self.step_draw(AnmId::Neutral, x.ctx);
        let mut c = self.cmd;
        if x.keys.is_key_repeat(x.pad, Buttons::UP.bits(), 6) {
            x.req.push(Request::Se(SE_MOVE));
            c -= 1;
        }
        if x.keys.is_key_repeat(x.pad, Buttons::DOWN.bits(), 6) {
            x.req.push(Request::Se(SE_MOVE));
            c += 1;
        }
        if c < 0 {
            c = 2;
        } else if c >= 3 {
            c = 0;
        }
        self.cmd = c;
        self.command_for(c);
        if x.pad.push.bits() & x.save.ok() != 0
            && x.save.check_operate(c + OPERATE_BASE[0])
            && x.save.check_operate(c + OPERATE_BASE[1])
        {
            x.req.push(Request::Se(if c == CMD_LOGIN { SE_LOGIN } else { SE_DECIDE }));
            self.change_mode(MODE_EXIT, a, x.save, Some((&mut *x.fade, &mut *x.req)));
            if c == CMD_LOGIN || c == CMD_QUIT {
                x.req.push(Request::EnableReset(false));
            }
        }
        if let Some(id) = self.command {
            self.step_draw(id, x.ctx);
        }
        if self.bbs_new {
            self.step_draw(AnmId::New, x.ctx);
        }
    }

    /// `_Exit` (0x00401380): the menu still playing under the fade; on the
    /// 31st frame the command's hand-off.
    fn exit_mode(&mut self, a: &BbsAssets, x: &mut Env) {
        self.step_draw(AnmId::Neutral, x.ctx);
        if let Some(id) = self.command {
            self.step_draw(id, x.ctx);
        }
        if self.bbs_new {
            self.step_draw(AnmId::New, x.ctx);
        }
        let old = self.act_count;
        self.act_count += 1;
        if old < EXIT_FRAMES {
            return;
        }
        match self.cmd {
            CMD_LOGIN => {
                let (num, sf) = REQUEST_FIELD;
                x.req.push(Request::ChangeMode { num, sf });
                let town = i32::from(x.save.save.u8(piney_data::save::offset::LAST_TOWN) as i8);
                x.req.push(Request::ChangeArea { area: 0, town });
                // ChangeScene's own ChangeRequest(6, 7).
                let (num, sf) = REQUEST_GAME_CTRL;
                x.req.push(Request::ChangeMode { num, sf });
            }
            CMD_QUIT => {
                let (num, sf) = REQUEST_DESKTOP;
                x.req.push(Request::ChangeMode { num, sf });
            }
            CMD_BBS => {
                self.change_mode(MODE_BBS, a, x.save, None);
                self.bbs.init(a, x.save);
            }
            _ => {}
        }
    }

    /// `_BBS` (0x004014f0): the board; when it has been left, the scene set
    /// up again, the menu and the flash.
    fn bbs_mode(&mut self, a: &BbsAssets, x: &mut Env) {
        if !self.bbs.initialized {
            return;
        }
        {
            let mut b = BbsEnv {
                ctx: x.ctx,
                fonts: x.fonts,
                names: x.names,
                save: x.save,
                push: x.pad.push.bits(),
                pad: x.pad,
                keys: x.keys,
                menu_type: x.menu_type,
                req: x.req,
            };
            self.bbs.draw(a, &mut b);
        }
        if !self.bbs.exit {
            return;
        }
        self.init();
        self.change_mode(MODE_NORMAL, a, x.save, None);
        let (t, c) = FLASH;
        entry_flash(x.fade, t, c);
    }
}
