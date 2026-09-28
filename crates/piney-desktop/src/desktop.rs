//! `Desktop_control` (DESKTOP.PRG, desktop.cpp / desktopMode.cpp): the main
//! screen's scene, its icon ring and page titles, the opening, and which
//! mode an icon opens.
//!
//! Everything here is drawn by `ccAnm` animations out of `xddesk01`, seen
//! through the camera of `ANM_xddcamer`; see `anm.rs`.

use std::rc::Rc;

use crate::anm::{Anm, Ctx, FogBlend};
use crate::assets::{Assets, SceneFile};
use crate::{Request, SaveState, Se};

/// `Back[5]` (`BackAnm`): the camera, the static back, the tunnel, the
/// wave and the rainbow.
pub const BACK_ANM: [&str; 5] = ["ANM_xddcamer", "ANM_xddback0", "ANM_xddtunne", "ANM_xddwave0", "ANM_xddrainb"];
pub const BACK_CAMERA: usize = 0;
pub const BACK_STATIC: usize = 1;
/// `Back[2..5]` are played and restarted when they end.
pub const BACK_PLAYED: std::ops::Range<usize> = 2..5;
/// `IconAnm`: each icon turning slowly (`Icon[0..6]`).
pub const ICON_ANM: [&str; 6] =
    ["ANM_xddroll1", "ANM_xddroll2", "ANM_xddroll3", "ANM_xddroll4", "ANM_xddroll5", "ANM_xddroll6"];
/// `IconAnm_ON`: the selected icon (`Icon[6..12]`).
pub const ICON_ANM_ON: [&str; 6] =
    ["ANM_xddsele1", "ANM_xddsele2", "ANM_xddsele3", "ANM_xddsele4", "ANM_xddsele5", "ANM_xddsele6"];
/// `PageTitleAnm_R_O`, `_L_O`, `_R_I`, `_L_I`: a page title leaving or
/// entering, to the right or the left.
pub const PAGE_R_OUT: [&str; 6] =
    ["ANM_xddr_ou1", "ANM_xddr_ou2", "ANM_xddr_ou3", "ANM_xddr_ou4", "ANM_xddr_ou5", "ANM_xddr_ou6"];
pub const PAGE_L_OUT: [&str; 6] =
    ["ANM_xddl_ou1", "ANM_xddl_ou2", "ANM_xddl_ou3", "ANM_xddl_ou4", "ANM_xddl_ou5", "ANM_xddl_ou6"];
pub const PAGE_R_IN: [&str; 6] =
    ["ANM_xddr_in1", "ANM_xddr_in2", "ANM_xddr_in3", "ANM_xddr_in4", "ANM_xddr_in5", "ANM_xddr_in6"];
pub const PAGE_L_IN: [&str; 6] =
    ["ANM_xddl_in1", "ANM_xddl_in2", "ANM_xddl_in3", "ANM_xddl_in4", "ANM_xddl_in5", "ANM_xddl_in6"];
/// `TitleLogo`: the fade under each page title (`Title`).
pub const TITLE_ANM: [&str; 6] =
    ["ANM_xddfade1", "ANM_xddfade2", "ANM_xddfade3", "ANM_xddfade4", "ANM_xddfade5", "ANM_xddfade6"];
/// `LogoAnm`: the logo of each icon (`Logo`).
pub const LOGO_ANM: [&str; 6] =
    ["ANM_xddlogo1", "ANM_xddlogo2", "ANM_xddlogo3", "ANM_xddlogo4", "ANM_xddlogo5", "ANM_xddlogo6"];
/// `OpenAnm[0]`: the opening (`Opening`).
pub const OPEN_ANM: &str = "ANM_xddesk01";
/// `MailviewAnm`: the mail page and the sender's photo frame.
pub const MAILVIEW_ANM: [&str; 2] = ["ANM_xddpage2", "ANM_xddphot0"];
/// `WebnewsAnm` by `volumeNum` (Infection's is 1: `ANM_xddpage3_1`).
pub const WEBNEWS_ANM: [&str; 4] = ["ANM_xddpage3_1", "ANM_xddpage3_2", "ANM_xddpage3_3", "ANM_xddpage3_4"];
pub const ACCES_ANM: &str = "ANM_xddpage4";
pub const AUDIO_ANM: &str = "ANM_xddpage5";
pub const DATA_ANM: [&str; 4] = ["ANM_xddpage6", "ANM_xddsel60", "ANM_xddsel61", "ANM_xddsel62"];
pub const NEW_ICON_ANM: &str = "ANM_xddnew0";
pub const BACK_BAR_ANM: &str = "ANM_xddback1";

/// `IconPos` (set in `SetStream`): where the NEW mark sits over the mail
/// icon and the news icon.
pub const ICON_POS: [[f32; 3]; 2] = [[-13500.0, 8300.0, 90000.0], [-13500.0, 5300.0, 90000.0]];
/// The NEW mark's spin: `4 pi / 40` a frame (0x41490fdb / 40), two turns
/// (4 pi) a cycle.
pub const NEW_SPIN_END: f32 = 12.566_371;
pub const NEW_SPIN_DIVISIONS: i32 = 40;
/// Frames the NEW mark rests between spins (`TmpCou`).
pub const NEW_REST: i32 = 126;
/// `ccDrawEnv::SetFogBlend(90.0, colour)` for the NEW mark.
pub const NEW_FOG_BLEND: f32 = 90.0;

/// `ccAnm.frameSpd` DrawLogo gives the page-title animations after their
/// first step: three frames a step.
pub const PAGE_TITLE_SPEED: u32 = 768;

/// The six icons, by `dsel` / `App`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// "The World": fades out and asks for TOPPAGE.
    World = 0,
    Mail = 1,
    News = 2,
    /// Wallpaper (`Acces_control`).
    Wallpaper = 3,
    Audio = 4,
    /// Save data (`SaveData_control`).
    Data = 5,
}

impl Icon {
    pub const ALL: [Icon; 6] = [Icon::World, Icon::Mail, Icon::News, Icon::Wallpaper, Icon::Audio, Icon::Data];

    pub fn from_index(i: i32) -> Option<Icon> {
        Icon::ALL.get(usize::try_from(i).ok()?).copied()
    }
}

/// `ccEvent::CheckOperate` numbers the desktop asks about: an icon `n`, and
/// its second lock `n + 19`.
pub const OPERATE_SECOND: i32 = 19;

/// Which page-title animation a DrawLogo case sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    RIn(usize),
    LIn(usize),
    ROut(usize),
    LOut(usize),
}

impl Page {
    fn name(self) -> &'static str {
        match self {
            Page::RIn(k) => PAGE_R_IN[k],
            Page::LIn(k) => PAGE_L_IN[k],
            Page::ROut(k) => PAGE_R_OUT[k],
            Page::LOut(k) => PAGE_L_OUT[k],
        }
    }
}

/// One direction of one DrawLogo case.
#[derive(Clone, Copy, Debug)]
struct Turn {
    title: usize,
    page_in: Page,
    page_out: Option<Page>,
    /// OUT is drawn before IN (only case 0 turning back).
    out_first: bool,
    /// OUT is drawn only once `StartFlg` is set (case 1 turning forward:
    /// the very first title at start-up comes in alone).
    out_needs_start: bool,
    /// `dsel` once all three animations have ended (the wrap cases).
    then: Option<i32>,
}

const fn turn(title: usize, page_in: Page, page_out: Option<Page>) -> Turn {
    Turn { title, page_in, page_out, out_first: false, out_needs_start: false, then: None }
}

/// `DrawLogo`'s eight cases (jump table 0x0042e610), by `dsel`: the logo,
/// then the turn for `LRflg` set (moved on, down or left) and clear (moved
/// back, up or right). Case 5 turning back reuses case 4's page titles, as
/// the code does; 6 and 7 are the wraps from 5 to 0 and from 0 to 5.
const LOGO_CASES: [(usize, Option<Turn>, Option<Turn>); 8] = [
    (
        0,
        Some(turn(0, Page::RIn(0), None)),
        Some(Turn { out_first: true, ..turn(0, Page::LIn(0), Some(Page::ROut(1))) }),
    ),
    (
        1,
        Some(Turn { out_needs_start: true, ..turn(1, Page::RIn(1), Some(Page::LOut(0))) }),
        Some(turn(1, Page::LIn(1), Some(Page::ROut(2)))),
    ),
    (2, Some(turn(2, Page::RIn(2), Some(Page::LOut(1)))), Some(turn(2, Page::LIn(2), Some(Page::ROut(3))))),
    (3, Some(turn(3, Page::RIn(3), Some(Page::LOut(2)))), Some(turn(3, Page::LIn(3), Some(Page::ROut(4))))),
    (4, Some(turn(4, Page::RIn(4), Some(Page::LOut(3)))), Some(turn(4, Page::LIn(4), Some(Page::ROut(5))))),
    (5, Some(turn(5, Page::RIn(5), Some(Page::LOut(4)))), Some(turn(5, Page::LIn(4), Some(Page::ROut(5))))),
    (0, Some(Turn { then: Some(0), ..turn(0, Page::RIn(0), Some(Page::LOut(5))) }), None),
    (5, None, Some(Turn { then: Some(5), ..turn(5, Page::LIn(5), Some(Page::ROut(0))) })),
];

/// `Desktop_control`'s animations and state (DWARF layout, 0x1d0 bytes).
pub struct DesktopControl {
    pub desk: Rc<SceneFile>,
    /// The news page's animation: `WEBNEWS_ANM[volumeNum - 1]`.
    webnews_anm: &'static str,
    /// +0x114
    pub back: [Anm; 5],
    /// +0x118: `IconAnm` then `IconAnm_ON`.
    pub icon: [Anm; 12],
    /// +0x120, +0x124
    pub page_in: Anm,
    pub page_out: Anm,
    /// +0x10c
    pub title: Anm,
    /// +0x128
    pub logo: Anm,
    /// +0x12c
    pub opening: Anm,
    /// +0x130
    pub mailview: [Anm; 2],
    /// +0x134, +0x138, +0x13c, +0x140
    pub webnews: Anm,
    pub acces: Anm,
    pub audio: Anm,
    pub data: [Anm; 4],
    /// +0x144
    pub new_icon: Anm,
    /// +0x108
    pub back_bar: Anm,
    /// +0x110: the wallpaper (`SetWall`).
    pub scr_back: Anm,
    /// +0x150 `TmpCou`, +0x154 `fl`, +0x180 `IconY`, +0x184 `tmpIrot`: the
    /// NEW mark's rest count, spin phase and angles.
    pub tmp_cou: i32,
    pub fl: i32,
    pub icon_y: f32,
    pub tmp_irot: f32,
    /// +0x190: the selected icon, 0-5; 6 and 7 while wrapping.
    pub dsel: i32,
    /// +0x194: the last move went on (down / left).
    pub lr_flg: bool,
    /// +0x198: the open mode, -1 for none.
    pub app: i32,
    /// +0x19c: DrawLogo's result; input is taken only while it is 1.
    pub check: i32,
    /// +0x1a0
    pub flg: i32,
    /// +0x1a4: the new-mail notice is up (AddMailList's result).
    pub new_mail: i32,
    /// +0x1a8: `MailList_control::InfoWindow`'s result.
    pub info_flg: i32,
    /// +0x1ac: SELECT hides the page title and back (1).
    pub draw_flg: i32,
    /// +0x1b0: the new-mail notice's alpha.
    pub alpha: i32,
    /// +0x1b4: the first page title has come in.
    pub start_flg: bool,
    /// +0x1b5: the opening's sound at frame 60 is still to play.
    pub open_flg: bool,
    /// +0x1b6: the selection changed; DrawLogo sets the animations.
    pub lock: bool,
}

/// The opening frame `PlayOpening` plays sound 1 on.
pub const OPENING_SE_FRAME: u32 = 60;

impl DesktopControl {
    /// The constructor (0x00400a00), `SetStream` (0x004015c0) and `SetWall`
    /// (0x004014d0).
    pub fn new(assets: &mut Assets, save: &SaveState) -> piney_data::Result<Self> {
        let desk = assets.desk.clone();
        let set = |name: &str| {
            let mut a = Anm::new();
            a.set(&desk, name);
            a
        };
        let set_fwd = |name: &str| {
            let mut a = set(name);
            a.forward();
            a
        };
        // SetStream steps new_icon, back_bar and Back[0..2] once after
        // setting them.
        let back = [set_fwd(BACK_ANM[0]), set_fwd(BACK_ANM[1]), set(BACK_ANM[2]), set(BACK_ANM[3]), set(BACK_ANM[4])];
        let icon = std::array::from_fn(|i| if i < 6 { set(ICON_ANM[i]) } else { set(ICON_ANM_ON[i - 6]) });
        let mut scr_back = Anm::new();
        let wall = crate::content::wall(assets.volume, save.dt_wallpaper());
        let wall_file = assets.file(&wall.name)?;
        scr_back.set(&wall_file, &wall.anm_name);
        scr_back.forward();
        let mut c = DesktopControl {
            new_icon: set_fwd(NEW_ICON_ANM),
            back_bar: set_fwd(BACK_BAR_ANM),
            back,
            icon,
            page_in: set(PAGE_R_IN[0]),
            page_out: set(PAGE_L_OUT[0]),
            title: set(TITLE_ANM[0]),
            logo: set(LOGO_ANM[0]),
            opening: set(OPEN_ANM),
            mailview: [set(MAILVIEW_ANM[0]), set(MAILVIEW_ANM[1])],
            webnews: set(WEBNEWS_ANM[assets.volume as usize]),
            webnews_anm: WEBNEWS_ANM[assets.volume as usize],
            acces: set(ACCES_ANM),
            audio: set(AUDIO_ANM),
            data: std::array::from_fn(|i| set(DATA_ANM[i])),
            scr_back,
            desk,
            tmp_cou: NEW_REST,
            fl: 0,
            icon_y: 0.0,
            tmp_irot: 0.0,
            dsel: 1,
            lr_flg: true,
            app: -1,
            check: 0,
            flg: 1,
            new_mail: 0,
            info_flg: 0,
            draw_flg: 0,
            alpha: 0,
            start_flg: false,
            open_flg: true,
            lock: true,
        };
        c.label();
        Ok(c)
    }

    /// Names the ccAnms after their members, for `anm::trace`.
    fn label(&mut self) {
        const BACK: [&str; 5] = ["back0", "back1", "back2", "back3", "back4"];
        const ICON: [&str; 12] =
            ["icon0", "icon1", "icon2", "icon3", "icon4", "icon5", "on0", "on1", "on2", "on3", "on4", "on5"];
        for (a, l) in self.back.iter_mut().zip(BACK) {
            a.label = l;
        }
        for (a, l) in self.icon.iter_mut().zip(ICON) {
            a.label = l;
        }
        self.page_in.label = "in";
        self.page_out.label = "out";
        self.title.label = "title";
        self.logo.label = "logo";
        self.opening.label = "opening";
        self.new_icon.label = "new";
        self.back_bar.label = "backbar";
        self.scr_back.label = "wall";
        self.webnews.label = "webnews";
        self.acces.label = "acces";
        self.audio.label = "audio";
        self.data[0].label = "data0";
        self.mailview[0].label = "mailview0";
    }

    /// One frame of `PlayOpening` (0x00400d10) after its breath: returns
    /// true when the opening is over (it ended, or ok / cancel was pressed).
    pub fn opening_frame(&mut self, ctx: &mut Ctx, save: &SaveState, push: u32, req: &mut Vec<Request>) -> bool {
        if self.opening.frame_now() == OPENING_SE_FRAME && self.open_flg {
            req.push(Request::Se(Se::OPENING));
            self.open_flg = false;
        }
        ctx.set_view(&self.back[BACK_CAMERA]);
        let ended = self.opening.forward();
        self.opening.draw(ctx);
        self.scr_back.draw(ctx);
        ended || push & (save.ok() | save.cancel()) != 0
    }

    /// `SelectMode` (0x00400e40).
    pub fn select_mode(&mut self, save: &mut SaveState, push: u32, req: &mut Vec<Request>) {
        use piney_input::Buttons;
        if self.check != 1 {
            return;
        }
        if self.app == -1 {
            if push & (Buttons::UP | Buttons::RIGHT).bits() != 0 {
                req.push(Request::Se(Se::CURSOR));
                self.draw_flg = 0;
                self.lr_flg = false;
                self.dsel -= 1;
                self.lock = true;
                self.check = 0;
                if self.dsel < 0 {
                    self.dsel = 7;
                }
            } else if push & (Buttons::DOWN | Buttons::LEFT).bits() != 0 {
                req.push(Request::Se(Se::CURSOR));
                self.draw_flg = 0;
                self.lr_flg = true;
                self.dsel += 1;
                self.lock = true;
                self.check = 0;
                if self.dsel >= 6 {
                    self.dsel = 6;
                }
                if self.flg != 0 {
                    self.flg = 0;
                }
            } else if self.start_flg
                && push & save.ok() != 0
                && push & Buttons::START.bits() == 0
                && save.check_operate(self.dsel)
                && save.check_operate(self.dsel + OPERATE_SECOND)
            {
                self.draw_flg = 0;
                self.new_mail = 0;
                self.app = self.dsel;
                if self.dsel == Icon::World as i32 {
                    req.push(Request::EnableReset(false));
                }
            }
        }
        if push & Buttons::SELECT.bits() != 0 {
            self.draw_flg = if self.draw_flg == 1 { 0 } else { 1 };
        }
    }

    /// `DrawLogo` (0x00402690): the logo and page titles of the selection.
    /// Returns 1 once they have all come in (or while hidden), else 0.
    pub fn draw_logo(&mut self, ctx: &mut Ctx) -> i32 {
        let Some(&(logo, forward, back)) = usize::try_from(self.dsel).ok().and_then(|d| LOGO_CASES.get(d)) else {
            return self.logo_end(false, false, false);
        };
        if self.lock {
            self.logo.set(&self.desk, LOGO_ANM[logo]);
        }
        self.logo.forward();
        self.logo.draw(ctx);
        let Some(t) = (if self.lr_flg { forward } else { back }) else {
            return self.logo_end(false, false, false);
        };
        if self.lock {
            self.title.set(&self.desk, TITLE_ANM[t.title]);
            self.page_in.set(&self.desk, t.page_in.name());
            if let Some(o) = t.page_out {
                self.page_out.set(&self.desk, o.name());
            }
            self.lock = false;
        }
        let s0 = self.page_in.forward();
        let s1 = if t.page_out.is_some() { self.page_out.forward() } else { true };
        let s2 = self.title.forward();
        if self.draw_flg == 1 {
            return 1;
        }
        let draw_out = t.page_out.is_some() && (!t.out_needs_start || self.start_flg);
        if t.out_first {
            self.page_out.draw(ctx);
            self.page_in.draw(ctx);
        } else {
            self.page_in.draw(ctx);
            if draw_out {
                self.page_out.draw(ctx);
            }
        }
        if let Some(d) = t.then
            && s0
            && s1
            && s2
        {
            self.dsel = d;
        }
        self.logo_end(s0, s1, s2)
    }

    fn logo_end(&mut self, s0: bool, s1: bool, s2: bool) -> i32 {
        if s0 && s1 && s2 {
            self.start_flg = true;
            1
        } else {
            self.page_in.frame_spd = PAGE_TITLE_SPEED;
            self.page_out.frame_spd = PAGE_TITLE_SPEED;
            self.title.frame_spd = PAGE_TITLE_SPEED;
            0
        }
    }

    /// `NewIconDraw` (0x004020c0): the spinning, flashing NEW mark over the
    /// mail and news icons.
    pub fn new_icon_draw(&mut self, ctx: &mut Ctx, new_mail: bool, new_news: bool) {
        use crate::eef::{add, div, from_int, le, lt, sub};
        let mut rot = [0.0f32; 3];
        let mut fog = 0;
        if self.fl != 0 {
            let step = div(NEW_SPIN_END, from_int(NEW_SPIN_DIVISIONS));
            self.icon_y = add(self.icon_y, step);
            self.tmp_irot = add(self.tmp_irot, step);
            if !le(self.tmp_irot, std::f32::consts::PI) {
                self.tmp_irot = sub(self.tmp_irot, std::f32::consts::TAU);
            }
            if !lt(self.icon_y, NEW_SPIN_END) {
                self.fl += 1;
                self.icon_y = 0.0;
                self.tmp_irot = 0.0;
                self.tmp_cou = NEW_REST;
            }
            if self.fl == 2 {
                self.fl = 0;
            }
        } else {
            self.tmp_cou -= 1;
            if self.tmp_cou == 0 {
                self.fl = 1;
            }
        }
        if self.fl != 0 {
            rot[0] = self.tmp_irot;
        } else {
            let a = 2 * self.tmp_cou;
            fog = if 256 - a < 128 { 256 - a } else { a };
        }
        let c = (255 - fog) as u32;
        ctx.fog_blend = Some(FogBlend { amount: NEW_FOG_BLEND, colour: [c as u8, c as u8, 0xff, 0] });
        for (on, pos) in [(new_mail, ICON_POS[0]), (new_news, ICON_POS[1])] {
            if on {
                self.new_icon.set_pos_rot(pos, rot);
                self.new_icon.draw(ctx);
            }
        }
        ctx.fog_blend = None;
    }

    /// `DrawBack` (0x00402300) without the new-mail notice, which the
    /// caller draws (it belongs to the mailer): the wallpaper, the NEW
    /// marks, the scene behind, and the icons.
    pub fn draw_back(&mut self, ctx: &mut Ctx, new_mail: bool, new_news: bool) {
        ctx.set_view(&self.back[BACK_CAMERA]);
        self.scr_back.draw(ctx);
        self.new_icon_draw(ctx, new_mail, new_news);
        if self.draw_flg != 1 {
            self.back[BACK_STATIC].draw(ctx);
            for i in BACK_PLAYED {
                let ended = self.back[i].forward();
                self.back[i].draw(ctx);
                if ended {
                    self.back[i].set(&self.desk, BACK_ANM[i]);
                }
            }
            self.title.draw(ctx);
        } else {
            self.back_bar.draw(ctx);
        }
        for i in 0..6 {
            self.icon[i].forward();
            self.icon[i + 6].forward();
            let on = match self.dsel {
                6 => 0,
                7 => 5,
                d => d as usize,
            };
            if i == on {
                self.icon[i + 6].draw(ctx);
            } else {
                self.icon[i].draw(ctx);
            }
        }
    }

    /// `DrawMailer` (0x00403940): the main screen behind the mailer.
    pub fn draw_mailer_back(&mut self, ctx: &mut Ctx, new_mail: bool, new_news: bool) {
        self.draw_back(ctx, new_mail, new_news);
        self.logo.draw(ctx);
        self.mailview[0].forward();
    }

    /// `DrawWebnews`, `DrawAccess`, `DrawAudio`, `DrawData`: the main screen
    /// and the mode's page, played and restarted when it ends.
    pub fn draw_page(&mut self, ctx: &mut Ctx, icon: Icon, new_mail: bool, new_news: bool, twice: bool) {
        self.draw_back(ctx, new_mail, new_news);
        self.logo.draw(ctx);
        let (anm, name) = match icon {
            Icon::News => (&mut self.webnews, self.webnews_anm),
            Icon::Wallpaper => (&mut self.acces, ACCES_ANM),
            Icon::Audio => (&mut self.audio, AUDIO_ANM),
            Icon::Data => (&mut self.data[0], DATA_ANM[0]),
            _ => return,
        };
        let ended = anm.forward();
        if twice {
            anm.draw(ctx);
        }
        anm.draw(ctx);
        if ended {
            anm.set(&self.desk, name);
        }
    }
}
