//! `ccOpening_Control` (DEMO.PRG, opening.cpp; DWARF layout 0x250 bytes): the
//! title screen's scene, its menu and the panels behind the menu items, every
//! one a `ccAnm` of `title1` seen through `ANM_xdtcam00`. `Main` (0x004043b0)
//! runs the action `m_MainAct` names, then `AllAnimate`, `AllTransparency`,
//! `AllDraw` and `MOVEcount`. Every function branches on `m_NowVol`, and
//! Infection's overlay holds all four volumes' branches: volume 1 has three
//! items (four with Parody), volumes 2-4 four (five) with the previous
//! volume's save. See docs/engine/title.md ("ccOpening_Control").

use std::rc::Rc;

use glam::Vec3;
use piney_desktop::anm::{Anm, Ctx};
use piney_desktop::assets::SceneFile;
use piney_desktop::eef;
use piney_input::{Buttons, Pad};

use crate::Request;
use crate::card::BootMem;
use crate::dataload::{DataLoad, Draw, Keys};
use crate::draw::draw_anm;
use crate::fade::ScFade;
use crate::names::{self, Panel};
use crate::seam::SystemMenu;

/// `m_MainAct`: what `Main` does this frame (its jump table at 0x0040eeb0).
pub mod act {
    pub const SET_NEUTRAL: i32 = 2;
    pub const NEUTRAL: i32 = 3;
    pub const SET_NEW_GAME: i32 = 4;
    pub const NEW_GAME: i32 = 5;
    pub const SET_DATA_LOAD: i32 = 6;
    pub const DATA_LOAD: i32 = 7;
    pub const SET_OPTION: i32 = 8;
    pub const OPTION: i32 = 9;
    /// Volumes 2-4 only.
    pub const SET_NEXT_DATA: i32 = 10;
    pub const NEXT_DATA: i32 = 11;
    pub const SET_PARODY: i32 = 12;
    pub const PARODY: i32 = 13;
    /// `MOVEcount`'s fade to black before the attract loop.
    pub const DEMO_FADE: i32 = 14;
    /// `Main` returns `m_StartMode`.
    pub const LEAVE: i32 = 15;
}

/// `m_StartMode`: what `Main` hands `ccThDemo` when it leaves.
pub mod start {
    /// New Game (and Parody): `ccSaveData::NewGame(0)`.
    pub const NEW_GAME: i32 = 2;
    /// A save loaded: `ccSaveData::LoadGame()`.
    pub const LOAD: i32 = 3;
    /// Volumes 2-4, a previous volume's save carried over:
    /// `ccStartEventConvert()`, `ccSaveData::ConvGame()`.
    pub const CONVERT: i32 = 4;
    /// Left idle: back to the opening movie.
    pub const ATTRACT: i32 = 5;
}

/// Menu items (`m_Nut_CurNo`).
pub mod item {
    pub const NEW_GAME: i32 = 0;
    pub const LOAD: i32 = 1;
    pub const OPTION: i32 = 2;
    pub const PARODY: i32 = 3;
}

/// `Init`: `m_Alpha`, the transparency step (0x3d4ccccd).
pub const ALPHA_STEP: u32 = 0x3d4c_cccd;
/// `Init`: `m_RepeatCount`, frames a held direction waits before it
/// repeats; `MoveCurNut` sets it to 9 after the first repeat and restarts
/// the count at 2.
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
pub const REPEAT_RESTART: i16 = 2;
/// `Init`: `m_Max_CurNO` (three items; Parody is past the end).
pub const MAX_CUR_NO: i32 = 3;
/// `SetNeutral`: the frame the selected item's animation starts at
/// (`_AnimateFrame(7680)`, frame 30).
pub const SELECTED_START: u32 = 7680;
/// `MOVEcount` (0x00404710): idle frames on the menu before the fade to
/// black, and before `Main` returns `start::ATTRACT`.
pub const DEMO_FADE_AT: i32 = 2380;
pub const DEMO_LEAVE_AT: i32 = 2400;
/// `AllAnimate`: the icons' turn a frame about y, radians (0x3cf5c28f).
pub const ICON_SPIN: u32 = 0x3cf5_c28f;
const PI: u32 = 0x4049_0fdb;
const MINUS_PI: u32 = 0xc049_0fdb;
const TWO_PI: u32 = 0x40c9_0fdb;
/// `AllTransparency`: frames after a panel closes before the icons and
/// items fade back in (`m_Trans_Wait`).
pub const TRANS_WAIT: i32 = 30;
/// `PlayDataLoad`: frames the fade to black runs after a load before
/// `Main` leaves.
pub const LOAD_FLASH_FRAMES: i32 = 20;
/// `EntryFlash3(20, 1, 1, 0x80000000)`: the fades to black of `MOVEcount`
/// and `PlayDataLoad`.
pub const BLACK: u32 = 0x8000_0000;
pub const BLACK_IN: i16 = 20;
pub const BLACK_OUT: i16 = 1;
pub const BLACK_HOLD: i16 = 1;

/// Sound effects (`ccSeOn`, the common bank).
pub mod se {
    /// The cursor moved (`MoveCurNut`).
    pub const CURSOR: i32 = 2;
    /// An item chosen (`PlayNeutral`).
    pub const DECIDE: i32 = 4;
    /// Data_Control's own: a choice moved, chosen, cancelled.
    pub const DATA_CURSOR: i32 = 6;
    pub const DATA_CANCEL: i32 = 7;
}

/// What one frame of the title reads and writes beside its own state.
pub struct Env<'a> {
    pub ctx: &'a mut Ctx,
    pub pad: &'a Pad,
    /// `saveData->assignPADok`, `assignPADcancel`.
    pub ok: u32,
    pub cancel: u32,
    /// `ccSys.count`, for the dialog cursor's blink.
    pub count: u32,
    /// `saveData->mainVol`.
    pub main_vol: i16,
    pub req: &'a mut Vec<Request>,
    pub fade: &'a mut ScFade,
    pub menu: &'a mut dyn SystemMenu,
    /// `saveSys`: the boot check and the load screen's requests.
    pub sys: &'a mut piney_desktop::savesys::SaveSys,
    /// `saveData->parodyFlag`, which `PlayParodyGame` sets.
    pub parody_flag: &'a mut bool,
    /// What the memory-card question and the load screen draw with
    /// (nothing without).
    pub dialog: Option<&'a crate::dialog::DialogAssets>,
}

/// `ccOpening_Control`.
pub struct Opening {
    pub file: Rc<SceneFile>,
    /// +0x004 `m_Mask`: bit 0 skips `AllAnimate`, bit 1 `AllTransparency`,
    /// for one frame.
    pub mask: i32,
    /// +0x008 `m_NowVol`.
    pub now_vol: i16,
    /// +0x00a .. +0x00e: `MoveCurNut`'s repeat.
    pub push_flg: i16,
    pub push_cnt: i16,
    pub repeat_count: i16,
    /// +0x010 `m_NewGameSW`: 0 New Game, 4 Parody; nothing reads it.
    pub new_game_sw: i32,
    /// +0x014 `m_DemoCount`: idle frames on the menu.
    pub demo_count: i32,
    /// +0x018 `m_FlashCount`.
    pub flash_count: i32,
    /// +0x01c `m_Alpha`, +0x020 `m_Transparency` (the items),
    /// +0x024 `m_Transparency_ico`, +0x028 `m_Transparency_win`.
    pub alpha: f32,
    pub transp: f32,
    pub transp_ico: f32,
    pub transp_win: f32,
    /// +0x02c .. +0x0a4: the name arrays.
    pub ico_d: [Option<&'static str>; 5],
    pub ico: [Option<&'static str>; 5],
    pub menu_d: [Option<&'static str>; 5],
    pub menu: [Option<&'static str>; 5],
    pub menu_sel: [Option<&'static str>; 5],
    pub back: [&'static str; 4],
    pub window: &'static str,
    /// +0x0b8 `m_coord`, +0x0cc `m_APP_coord`: the dummy objects (names)
    /// `GetSubstAdrsF` found in `m_A_ICO_D[i]` and `m_A_APP_D[i]`.
    pub coord: [Option<&'static str>; 5],
    pub app_coord: [Option<&'static str>; 5],
    /// +0x0e0 `m_ico_Dvec`, +0x130 `m_app_Dvec`: their positions.
    pub ico_dvec: [Vec3; 5],
    pub app_dvec: [Vec3; 5],
    /// +0x190 `m_Rot_ICO`.
    pub rot_ico: [f32; 3],
    /// +0x1c0 .. +0x1dc: the animations.
    pub camera: Anm,
    pub a_ico: [Anm; 5],
    pub a_ico_d: [Anm; 5],
    pub a_app: [Anm; 5],
    pub a_app_d: [Anm; 5],
    pub a_win: Anm,
    pub a_back: [Anm; 4],
    pub a_boot: Option<Anm>,
    /// +0x1e4 .. +0x208: the actions.
    pub logo_act: i32,
    pub main_act: i32,
    pub nut_act: i32,
    pub dat_act: i32,
    pub opt_act: i32,
    /// +0x1fc `m_NextAct`: `PlayNextData`'s step.
    pub next_act: i32,
    pub boot_act: i32,
    pub err_flg: i32,
    /// +0x20c `m_ParoFLG`: shows the Parody item. `Init` clears it and
    /// nothing in Infection's DEMO.PRG sets it.
    pub paro_flg: bool,
    /// +0x20d `m_NutLock`: the first `SetNeutral` still to reset the
    /// backdrop and icons.
    pub nut_lock: bool,
    /// +0x210 `m_Nut_CurNo`: the menu cursor.
    pub nut_cur_no: i32,
    /// +0x214 `m_Ne_SW`, +0x218 `m_Dat_SW`, +0x224 `m_OptSW`: an
    /// animation the action waits on has ended.
    pub ne_sw: i32,
    pub dat_sw: i32,
    pub opt_sw: i32,
    /// +0x21c `m_NextSW`.
    pub next_sw: i32,
    /// +0x220 `m_LoadSW`: a save was loaded.
    pub load_sw: bool,
    /// +0x228 `m_Trans_Wait`.
    pub trans_wait: i32,
    /// +0x22c `m_Max_CurNO`.
    pub max_cur_no: i32,
    /// +0x230 `m_FlashFlg`: set by every `ChangeMainAct`; nothing reads it.
    pub flash_flg: bool,
    /// +0x234 `m_StartMode`.
    pub start_mode: i32,
    /// +0x238 `m_APP_DataLoad`: the load screen.
    pub data_load: DataLoad,
    /// +0x240 `m_APP_NextDataLoad`: the previous volume's saves.
    pub next_load: DataLoad,
    /// +0x23c `m_APP_BootMem`.
    pub boot_mem: BootMem,
}

const ICO_LABELS: [&str; 5] = ["ico0", "ico1", "ico2", "ico3", "ico4"];
const ICO_D_LABELS: [&str; 5] = ["icod0", "icod1", "icod2", "icod3", "icod4"];
const APP_LABELS: [&str; 5] = ["app0", "app1", "app2", "app3", "app4"];
const APP_D_LABELS: [&str; 5] = ["appd0", "appd1", "appd2", "appd3", "appd4"];
const BACK_LABELS: [&str; 4] = ["back0", "back1", "back2", "back3"];

fn anms<const N: usize>(labels: [&'static str; N]) -> [Anm; N] {
    labels.map(Anm::labelled)
}

/// `if (anm->anmIndex) r = anm->_AnimateForward(anm->frameSpd)`, else 0.
fn fwd(a: &mut Anm) -> i32 {
    if a.is_set() { i32::from(a.forward()) } else { 0 }
}

/// `ccAnm::SetAnm(ccsc, name, 0)`; a null name finds nothing.
fn set(a: &mut Anm, file: &Rc<SceneFile>, name: Option<&str>) {
    a.set(file, name.unwrap_or(""));
}

/// `_AnimateFrame(frame)` on an animation just set (time 0): one step of
/// `frame` ticks with the note callback off.
fn animate_frame(a: &mut Anm, frame: u32) {
    if !a.is_set() {
        return;
    }
    let spd = a.frame_spd;
    a.frame_spd = frame;
    a.forward();
    a.frame_spd = spd;
}

/// `m_coord[i] = anm->GetSubstAdrsF(name, 0)` in the `Set*` functions: the
/// dummy looked up (the name is kept; its position is read each time).
fn subst(a: &Anm, name: &'static str) -> Option<&'static str> {
    #[cfg(feature = "trace")]
    piney_desktop::anm::trace::record("subst", a.label, name.to_string());
    let _ = a;
    Some(name)
}

/// The ccAnm's own matrix: `UnitMatrix` + `TransMatrix(pos)` when `rot` is
/// `None`, `SetMatrix_PosRotXYZ(pos, rot)` otherwise.
fn place(a: &mut Anm, pos: Vec3, rot: Option<[f32; 3]>) {
    #[cfg(feature = "trace")]
    piney_desktop::anm::trace::record("place", a.label, rot.map_or(0, |r| r[1].to_bits()).to_string());
    a.set_pos_rot(pos.into(), rot.unwrap_or([0.0; 3]));
}

/// A dummy's position: `GetSubstAdrsF(name)`'s `lwMatrix` translation
/// (the object's world matrix, `_SetLWMatrix` when it has a parent).
fn dummy_pos(file: &SceneFile, a: &Anm, name: Option<&str>) -> Vec3 {
    let Some(name) = name else { return Vec3::ZERO };
    a.instances_all()
        .into_iter()
        .find(|(obj, _, _)| file.ccs.object_name(*obj) == Some(name))
        .map_or(Vec3::ZERO, |(_, m, _)| m.w_axis.truncate())
}

impl Opening {
    /// The constructor (0x00406610) and `Init` (0x00405f50), then
    /// `SetVolCcs` (0x00405e80) and `SetStream` (0x00407670): the camera
    /// set and stepped once. The caller gives its camera to the layer's
    /// view (`SetView(active->view, m_Camera->cam)`).
    pub fn new(file: Rc<SceneFile>) -> Opening {
        let mut o = Opening {
            mask: 0,
            now_vol: 1,
            push_flg: 0,
            push_cnt: 0,
            repeat_count: REPEAT_FIRST,
            new_game_sw: 0,
            demo_count: 0,
            flash_count: 0,
            alpha: f32::from_bits(ALPHA_STEP),
            transp: 1.0,
            transp_ico: 1.0,
            transp_win: 1.0,
            ico_d: [None; 5],
            ico: [None; 5],
            menu_d: [None; 5],
            menu: [None; 5],
            menu_sel: [None; 5],
            back: names::BACK,
            window: "",
            coord: [None; 5],
            app_coord: [None; 5],
            ico_dvec: [Vec3::ZERO; 5],
            app_dvec: [Vec3::ZERO; 5],
            rot_ico: [0.0; 3],
            camera: Anm::labelled("camera"),
            a_ico: anms(ICO_LABELS),
            a_ico_d: anms(ICO_D_LABELS),
            a_app: anms(APP_LABELS),
            a_app_d: anms(APP_D_LABELS),
            a_win: Anm::labelled("win"),
            a_back: anms(BACK_LABELS),
            a_boot: None,
            logo_act: 0,
            main_act: act::SET_NEUTRAL,
            nut_act: 0,
            dat_act: 0,
            opt_act: 0,
            next_act: 0,
            boot_act: 0,
            err_flg: 0,
            paro_flg: false,
            nut_lock: true,
            nut_cur_no: item::LOAD,
            ne_sw: 0,
            dat_sw: 0,
            opt_sw: 0,
            next_sw: 0,
            load_sw: false,
            trans_wait: 0,
            max_cur_no: MAX_CUR_NO,
            flash_flg: false,
            start_mode: start::NEW_GAME,
            data_load: DataLoad::new(),
            next_load: DataLoad::new_next(),
            boot_mem: BootMem::new(),
            file,
        };
        let file = o.file.clone();
        o.camera.set(&file, names::CAMERA);
        fwd(&mut o.camera);
        o
    }

    /// Volumes 2-4 (`m_NowVol` 2, 3, 4).
    fn later(&self) -> bool {
        (2..=4).contains(&self.now_vol)
    }

    /// Items on the menu: three on volume 1, four on 2-4 (the previous
    /// volume's save), one more with the Parody item.
    fn items(&self) -> usize {
        (if self.later() { 4 } else { 3 }) + usize::from(self.paro_flg)
    }

    /// Mutation's `ccThDemo` (MUT 0x00413a3c, calling its new 0x00419630
    /// after `Init`): `m_Max_CurNO` 4 on volumes 2-4.
    pub fn set_max_cur(&mut self) {
        if self.later() {
            self.max_cur_no = 4;
        }
    }

    // The memory-card check -------------------------------------------------

    /// `SetBootMemCard` (0x004076f0).
    pub fn set_boot_mem_card(&mut self) {
        let file = self.file.clone();
        self.back = names::back(self.now_vol);
        self.boot_act = 0;
        self.err_flg = 1;
        let mut boot = Anm::labelled("boot");
        for (a, name) in self.a_back.iter_mut().zip(self.back) {
            a.set(&file, name);
        }
        boot.set(&file, names::BOOT);
        for _ in 0..names::BOOT_PRESTEP {
            fwd(&mut self.a_back[3]);
            fwd(&mut boot);
        }
        self.a_boot = Some(boot);
    }

    /// `PlayBootMemCard` (0x00406950): 1 once the check is over.
    pub fn play_boot_mem_card(&mut self, env: &mut Env) -> i32 {
        match self.boot_act {
            0 => {
                self.set_boot_mem_card();
                self.boot_act += 1;
            }
            1 => {
                let r = self.boot_mem.main_control(env);
                if r == 1 {
                    self.boot_act = 3;
                    self.err_flg = 1;
                }
                if r == 2 {
                    self.boot_act += 1;
                    self.err_flg = 2;
                }
                if r == 0 {
                    self.err_flg = 0;
                }
            }
            2 => {
                if fwd(&mut self.a_back[3]) != 0 {
                    self.boot_act += 1;
                }
            }
            3 => {
                self.a_boot = None;
                return 1;
            }
            _ => {}
        }
        if self.err_flg == 0 || self.err_flg == 2 {
            for a in &mut self.a_back[..3] {
                fwd(a);
            }
            for a in &self.a_back {
                draw_anm(env.ctx, &self.file, a);
            }
            if let Some(b) = &self.a_boot {
                crate::draw::draw_anm_scaled(env.ctx, &self.file, b, names::BOOT_SCALE);
            }
        }
        0
    }

    // Main ------------------------------------------------------------------

    /// `Main` (0x004043b0): one frame of the title. Non-zero is
    /// `m_StartMode`, returned from `LEAVE` without drawing.
    pub fn main(&mut self, env: &mut Env) -> i32 {
        match self.main_act {
            act::SET_NEUTRAL => self.set_neutral(),
            act::NEUTRAL => self.play_neutral(env),
            act::SET_NEW_GAME => self.set_new_game(),
            act::NEW_GAME => self.play_new_game(),
            act::SET_DATA_LOAD => self.set_data_load(),
            act::DATA_LOAD => self.play_data_load(env),
            act::SET_OPTION => self.set_option(0),
            act::OPTION => self.play_option(env),
            act::SET_NEXT_DATA => self.set_next_data(0),
            act::NEXT_DATA => self.play_next_data(env),
            act::SET_PARODY => self.set_parody_game(),
            act::PARODY => self.play_parody_game(env),
            act::LEAVE => {
                self.nut_cur_no = item::LOAD;
                self.set_neutral();
                return self.start_mode;
            }
            _ => {}
        }
        if self.mask & 1 == 0 {
            self.all_animate();
        }
        if self.mask & 2 == 0 {
            self.all_transparency(&*env.menu);
        }
        self.all_draw(env.ctx);
        self.move_count(env.fade);
        self.mask = 0;
        0
    }

    /// `MOVEcount` (0x00404710): the idle count on the menu, the fade to
    /// black at 2380 and the attract loop at 2400.
    pub fn move_count(&mut self, fade: &mut ScFade) {
        if self.main_act == act::NEUTRAL || self.main_act == act::DEMO_FADE {
            self.demo_count += 1;
        }
        if self.demo_count == DEMO_FADE_AT {
            self.main_act = act::DEMO_FADE;
            fade.entry_flash3(BLACK_IN, BLACK_OUT, BLACK_HOLD, BLACK);
        }
        if self.demo_count == DEMO_LEAVE_AT {
            self.main_act = act::LEAVE;
            self.start_mode = start::ATTRACT;
        }
    }

    /// `ChangeMainAct(mode)` (0x00405d20): volume 1's table (0x0040ef90),
    /// or volumes 2-4's (0x0040ef70, item 3 the previous volume's save);
    /// another volume changes nothing.
    pub fn change_main_act(&mut self, mode: i32) {
        const VOL1: [i32; 7] = [
            act::SET_NEW_GAME,
            act::SET_DATA_LOAD,
            act::SET_OPTION,
            act::SET_PARODY,
            act::SET_PARODY,
            act::SET_NEUTRAL,
            act::LEAVE,
        ];
        const LATER: [i32; 7] = [
            act::SET_NEW_GAME,
            act::SET_DATA_LOAD,
            act::SET_OPTION,
            act::SET_NEXT_DATA,
            act::SET_PARODY,
            act::SET_NEUTRAL,
            act::LEAVE,
        ];
        let table = match self.now_vol {
            1 => Some(&VOL1),
            2..=4 => Some(&LATER),
            _ => None,
        };
        if let Some(&a) = table.and_then(|t| usize::try_from(mode).ok().and_then(|m| t.get(m))) {
            self.main_act = a;
        }
        self.flash_flg = true;
    }

    // The menu --------------------------------------------------------------

    /// `SetNeutral` (0x00407950): the menu at rest, the cursor's item
    /// selected.
    pub fn set_neutral(&mut self) {
        let file = self.file.clone();
        if self.later() {
            self.menu = names::later::MENU;
            self.menu_sel = names::later::MENU_SEL;
            self.ico = names::later::ICO;
            self.ico_d = names::later::ICO_D_NEUTRAL;
        } else {
            self.menu = names::MENU;
            self.menu_sel = names::MENU_SEL;
            self.ico = names::ICO;
            self.ico_d = names::ICO_D_NEUTRAL;
        }
        let dam = if self.later() { names::later::DAM_ICO } else { names::DAM_ICO };
        self.back = names::back(self.now_vol);
        self.demo_count = 0;
        let n = self.items();
        self.main_act = act::NEUTRAL;
        self.a_ico[0].localtp = self.transp_ico;
        self.a_ico[2].localtp = self.transp_ico;
        self.a_app[0].localtp = self.transp;
        self.a_app[2].localtp = self.transp;
        if self.nut_lock {
            self.a_back[1].set(&file, self.back[1]);
            self.a_back[2].set(&file, self.back[2]);
            for i in 0..n {
                set(&mut self.a_ico[i], &file, self.ico[i]);
            }
            self.nut_lock = false;
        }
        self.a_back[0].set(&file, self.back[0]);
        self.a_back[3].set(&file, self.back[3]);
        for i in 0..n {
            place(&mut self.a_app[i], Vec3::ZERO, None);
            let name = if self.nut_cur_no == i as i32 { self.menu_sel[i] } else { self.menu[i] };
            set(&mut self.a_app[i], &file, name);
        }
        if self.paro_flg {
            set(&mut self.a_app[n - 1], &file, self.menu[n - 1]);
        }
        for (i, &dam) in dam.iter().enumerate().take(n) {
            set(&mut self.a_ico_d[i], &file, self.ico_d[i]);
            fwd(&mut self.a_ico_d[i]);
            self.coord[i] = subst(&self.a_ico_d[i], dam);
            self.ico_dvec[i] = dummy_pos(&file, &self.a_ico_d[i], self.coord[i]);
            place(&mut self.a_ico[i], self.ico_dvec[i], None);
        }
        // Volume 1: cursor 3 and 4 both jump A_APP[4], which volume 1 never
        // sets (so nothing moves); volumes 2-4 the cursor's own item.
        let k = match (self.later(), self.nut_cur_no) {
            (_, c @ 0..=2) => Some(c as usize),
            (false, 3 | 4) => Some(4),
            (true, c @ 3..=4) => Some(c as usize),
            _ => None,
        };
        if let Some(k) = k {
            animate_frame(&mut self.a_app[k], SELECTED_START);
        }
    }

    /// `PlayNeutral` (0x00406b50).
    pub fn play_neutral(&mut self, env: &mut Env) {
        match self.nut_act {
            0 => {
                self.move_cur_nut(env.pad, env.req);
                if env.pad.push.bits() & env.ok != 0 {
                    self.nut_act += 1;
                    env.req.push(Request::Se(se::DECIDE));
                }
            }
            1 => {
                self.change_main_act(self.nut_cur_no);
                self.nut_act = 0;
            }
            _ => {}
        }
    }

    /// One direction of `MoveCurNut`: push moves at once; release resets
    /// the wait; held (`ccPad.repeat`), `m_Pushcnt` counts to
    /// `m_RepeatCount`, then moves and repeats every 7.
    fn cur_repeat(&mut self, pad: &Pad, bit: Buttons) -> bool {
        if pad.push.intersects(bit) {
            self.push_flg = 1;
            true
        } else if pad.unpush.intersects(bit) {
            self.repeat_count = REPEAT_FIRST;
            self.push_flg = 0;
            false
        } else if pad.repeat.intersects(bit) {
            self.push_cnt += 1;
            if self.push_cnt == self.repeat_count {
                self.repeat_count = REPEAT_NEXT;
                self.push_cnt = REPEAT_RESTART;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// `MoveCurNut` (0x004070c0): up and down, clamped to the items.
    pub fn move_cur_nut(&mut self, pad: &Pad, req: &mut Vec<Request>) {
        let mut moved = false;
        if self.cur_repeat(pad, Buttons::UP) {
            self.nut_cur_no -= 1;
            moved = true;
            self.demo_count = 0;
        }
        if self.cur_repeat(pad, Buttons::DOWN) {
            self.nut_cur_no += 1;
            moved = true;
            self.demo_count = 0;
        }
        if self.nut_cur_no < 0 {
            self.nut_cur_no = 0;
            moved = false;
        }
        if self.nut_cur_no > self.max_cur_no - 1 {
            self.nut_cur_no = self.max_cur_no - 1;
            moved = false;
        }
        if moved {
            self.switch_cur(self.nut_cur_no);
            req.push(Request::Se(se::CURSOR));
        }
    }

    /// `SwitchCur(cur)` (0x004072a0): the item under the cursor selected,
    /// its neighbours at rest. On volume 1 cursor 4 is item 3; the item
    /// after the last only when Parody shows it (volume 1's item 3,
    /// volumes 2-4's item 4). Another volume does nothing.
    pub fn switch_cur(&mut self, cur: i32) {
        let file = self.file.clone();
        let last = if self.later() { 4 } else { 3 };
        let sel = match (self.now_vol, cur) {
            (1, 3 | 4) => 3,
            (1..=4, c @ 0..=4) if c <= last => c as usize,
            _ => return,
        };
        let mut calls: Vec<(usize, bool)> = Vec::new();
        if sel > 0 {
            calls.push((sel - 1, false));
        }
        calls.push((sel, true));
        // The next item: always below the last but one, the last with
        // Parody's.
        if sel + 1 < last as usize || (sel + 1 == last as usize && self.paro_flg) {
            calls.push((sel + 1, false));
        }
        for (i, on) in calls {
            let name = if on { self.menu_sel[i] } else { self.menu[i] };
            set(&mut self.a_app[i], &file, name);
        }
    }

    /// `SetNewGame` (0x00408240).
    pub fn set_new_game(&mut self) {
        let file = self.file.clone();
        self.main_act = act::NEW_GAME;
        self.a_app[0].set(&file, names::NEW_GAME_DECIDE);
    }

    /// `PlayNewGame` (0x00406bf0): leaves once the screen-out has ended.
    pub fn play_new_game(&mut self) {
        if self.ne_sw != 0 {
            self.main_act = act::LEAVE;
            self.new_game_sw = 0;
            self.start_mode = start::NEW_GAME;
        }
    }

    /// `SetParodyGame` (0x00409d40): the Parody item's decide animation,
    /// item 3 on volume 1, 4 on volumes 2-4.
    pub fn set_parody_game(&mut self) {
        let file = self.file.clone();
        self.main_act = act::PARODY;
        match self.now_vol {
            1 => self.a_app[3].set(&file, names::PARODY_DECIDE),
            2..=4 => self.a_app[4].set(&file, names::later::PARODY_DECIDE),
            _ => {}
        }
    }

    /// `PlayParodyGame` (0x00406c20): as New Game, and every frame
    /// `saveData->parodyFlag = 1`.
    pub fn play_parody_game(&mut self, env: &mut Env) {
        if self.ne_sw != 0 {
            self.main_act = act::LEAVE;
            self.new_game_sw = 4;
            self.start_mode = start::NEW_GAME;
        }
        *env.parody_flag = true;
    }

    // Load and Option ---------------------------------------------------------

    /// The panel half of `SetDataLoad` and `SetOption`: window, backdrop
    /// and screen-out, the items' animations, and the dummies placing the
    /// icons and items. `None` keeps the names already in the arrays.
    fn set_panel(&mut self, p: Option<Panel>, menu: [Option<&'static str>; 5]) {
        let file = self.file.clone();
        if let Some(p) = p {
            self.ico_d = p.ico_d;
            self.menu_d = p.menu_d;
            self.window = p.window;
            self.back[0] = p.back0;
            self.back[3] = p.back3;
        }
        self.menu = menu;
        let n = self.items();
        let (dam_ico, dam_men) = if self.later() {
            (names::later::DAM_ICO, names::later::DAM_MEN)
        } else {
            (names::DAM_ICO, names::DAM_MEN)
        };
        self.a_win.set(&file, self.window);
        for i in 0..n {
            set(&mut self.a_app[i], &file, self.menu[i]);
        }
        self.a_back[0].set(&file, self.back[0]);
        self.a_back[3].set(&file, self.back[3]);
        for (i, &dam) in dam_ico.iter().enumerate().take(n) {
            set(&mut self.a_ico_d[i], &file, self.ico_d[i]);
            fwd(&mut self.a_ico_d[i]);
            self.coord[i] = subst(&self.a_ico_d[i], dam);
            self.ico_dvec[i] = dummy_pos(&file, &self.a_ico_d[i], self.coord[i]);
            place(&mut self.a_ico[i], self.ico_dvec[i], None);
        }
        for (i, &dam) in dam_men.iter().enumerate().take(n) {
            set(&mut self.a_app_d[i], &file, self.menu_d[i]);
            fwd(&mut self.a_app_d[i]);
            self.app_coord[i] = subst(&self.a_app_d[i], dam);
            self.app_dvec[i] = dummy_pos(&file, &self.a_app_d[i], self.app_coord[i]);
            place(&mut self.a_app[i], self.app_dvec[i], None);
        }
    }

    /// A panel's names for this volume: the Load window (`window` 1) or
    /// the Option one (2), opening or closing; volume 1's or 2-4's.
    fn panel_for(&self, open: bool, window: u8) -> Panel {
        if self.later() {
            let w = match (open, window) {
                (true, 2) => "ANM_xdt_op02",
                (true, _) => "ANM_xdt_op01",
                (false, 2) => "ANM_xdt_cl02",
                (false, _) => "ANM_xdt_cl01",
            };
            return names::later::panel(self.now_vol, open, w);
        }
        match (open, window) {
            (true, 2) => names::OPTION_OPEN,
            (true, _) => names::LOAD_OPEN,
            (false, 2) => names::OPTION_CLOSE,
            (false, _) => names::LOAD_CLOSE,
        }
    }

    /// `SetDataLoad` (0x004082c0): the load window opening (`m_DatAct` 0)
    /// or closing (1).
    pub fn set_data_load(&mut self) {
        self.trans_wait = 0;
        // From Main m_DatAct is 0; PlayDataLoad calls it with 1. Anything
        // else keeps the names already set.
        let panel = match self.dat_act {
            0 => {
                self.main_act = act::DATA_LOAD;
                Some(self.panel_for(true, 1))
            }
            1 => Some(self.panel_for(false, 1)),
            _ => None,
        };
        let menu = if self.later() { names::later::MENU_LOAD } else { names::MENU_LOAD };
        self.set_panel(panel, menu);
    }

    /// `SetNextData(mode)` (0x00409470): `SetDataLoad` for the previous
    /// volume's saves, the mode given (0 open, 1 close) and `m_OptSW`
    /// cleared as `SetOption` clears it.
    pub fn set_next_data(&mut self, mode: i32) {
        self.trans_wait = 0;
        self.opt_sw = 0;
        let panel = match mode {
            0 => {
                self.main_act = act::NEXT_DATA;
                Some(self.panel_for(true, 1))
            }
            1 => Some(self.panel_for(false, 1)),
            _ => None,
        };
        let menu = if self.later() { names::later::MENU_NEXT } else { names::MENU_NEXT };
        self.set_panel(panel, menu);
    }

    /// `PlayNextData` (0x00406f40): as `PlayDataLoad` on the
    /// `NextDataLoad_Control`, but a save read marks `m_LoadSW` at once,
    /// the window then closes (the flash's 20 frames, then its
    /// animations), and it leaves with `m_StartMode` 4: the carry-over.
    pub fn play_next_data(&mut self, env: &mut Env) {
        match self.next_act {
            0 => {
                if self.next_sw != 0 {
                    let keys = Keys { pad: env.pad, ok: env.ok, cancel: env.cancel };
                    let mut d = Draw {
                        ctx: &mut *env.ctx,
                        assets: env.dialog,
                        count: env.count,
                        frame_rate: crate::FRAME_RATE,
                    };
                    let r = self.next_load.main_control(&mut d, keys, env.sys, env.req);
                    if r == -1 {
                        self.next_act = 1;
                    }
                    if r == 1 {
                        self.next_act = 2;
                        self.load_sw = true;
                        self.flash_count = 0;
                        env.fade.entry_flash3(BLACK_IN, BLACK_OUT, BLACK_HOLD, BLACK);
                    }
                }
            }
            1 => {
                self.set_next_data(1);
                self.next_act = 3;
                self.next_sw = 0;
                self.load_sw = false;
            }
            2 => {
                let n = self.flash_count;
                self.flash_count += 1;
                if n == LOAD_FLASH_FRAMES {
                    self.next_act = 3;
                    self.next_sw = 0;
                    self.load_sw = true;
                }
            }
            3 if self.next_sw == 1 => {
                self.next_act = 0;
                self.change_main_act(if self.load_sw { 6 } else { 5 });
                self.start_mode = start::CONVERT;
                self.load_sw = false;
            }
            _ => {}
        }
    }

    /// `PlayDataLoad` (0x00406c60).
    pub fn play_data_load(&mut self, env: &mut Env) {
        match self.dat_act {
            0 => {
                if self.dat_sw != 0 {
                    let keys = Keys { pad: env.pad, ok: env.ok, cancel: env.cancel };
                    let mut d = Draw {
                        ctx: &mut *env.ctx,
                        assets: env.dialog,
                        count: env.count,
                        frame_rate: crate::FRAME_RATE,
                    };
                    let r = self.data_load.main_control(&mut d, keys, env.sys, env.req);
                    if r == -1 {
                        self.dat_act = 1;
                    }
                    if r == 1 {
                        self.dat_act = 2;
                        self.flash_count = 0;
                        env.fade.entry_flash3(BLACK_IN, BLACK_OUT, BLACK_HOLD, BLACK);
                    }
                }
            }
            1 => {
                self.set_data_load();
                self.dat_act = 3;
                self.dat_sw = 0;
                self.load_sw = false;
            }
            2 => {
                let n = self.flash_count;
                self.flash_count += 1;
                if n == LOAD_FLASH_FRAMES {
                    self.dat_act = 3;
                    self.dat_sw = 1;
                    self.load_sw = true;
                }
            }
            3 if self.dat_sw == 1 => {
                self.dat_act = 0;
                self.end_data_load();
                self.change_main_act(if self.load_sw { 6 } else { 5 });
                self.load_sw = false;
            }
            _ => {}
        }
    }

    /// `EndDataLoad` (0x00407640).
    pub fn end_data_load(&mut self) {
        self.transp_ico = 1.0;
        self.transp = 1.0;
        self.mask |= 2;
        self.start_mode = start::LOAD;
    }

    /// `SetOption(mode)` (0x00408ba0): the option window opening (0) or
    /// closing (1).
    pub fn set_option(&mut self, mode: i32) {
        self.trans_wait = 0;
        self.opt_sw = 0;
        let panel = match mode {
            0 => {
                self.main_act = act::OPTION;
                Some(self.panel_for(true, 2))
            }
            1 => Some(self.panel_for(false, 2)),
            _ => None,
        };
        let menu = if self.later() { names::later::MENU_OPTION } else { names::MENU_OPTION };
        self.set_panel(panel, menu);
    }

    /// `PlayOption` (0x00406de0): open the window, ask the system menu for
    /// the settings, wait for it to close, close the window.
    pub fn play_option(&mut self, env: &mut Env) {
        match self.opt_act {
            0 => {
                if self.opt_sw != 0 {
                    self.opt_act = 1;
                }
            }
            1 => {
                env.menu.request(crate::seam::MENU_SETTINGS as i16);
                self.opt_act += 1;
            }
            2 => {
                if env.menu.menu_type() == crate::seam::MENU_SETTINGS && env.pad.push.bits() & env.cancel != 0 {
                    self.opt_act += 1;
                }
            }
            3 => {
                if env.menu.menu_type() == crate::seam::MENU_NONE {
                    self.opt_act += 1;
                    self.set_option(1);
                }
            }
            4 if self.opt_sw != 0 => {
                self.opt_act = 0;
                self.change_main_act(5);
                self.transp_ico = 1.0;
                self.transp = 1.0;
                self.mask |= 2;
            }
            _ => {}
        }
    }

    // Every frame -------------------------------------------------------------

    /// A panel's frame of `AllAnimate`: the dummies stepped, the items
    /// moved to theirs, and the window and screen-out.
    fn animate_panel(&mut self, n: usize, sub: i32, closing: i32) -> i32 {
        for a in &mut self.a_ico_d[..n] {
            fwd(a);
        }
        for i in 0..n {
            fwd(&mut self.a_app_d[i]);
            self.app_dvec[i] = dummy_pos(&self.file, &self.a_app_d[i], self.app_coord[i]);
            place(&mut self.a_app[i], self.app_dvec[i], None);
        }
        if sub == 0 {
            let sw = fwd(&mut self.a_win);
            fwd(&mut self.a_back[3]);
            sw
        } else if sub == closing {
            fwd(&mut self.a_win);
            fwd(&mut self.a_back[3])
        } else {
            -1
        }
    }

    /// `AllAnimate` (0x004047e0).
    pub fn all_animate(&mut self) {
        for i in 0..3 {
            fwd(&mut self.a_back[i]);
        }
        for i in 0..3 {
            fwd(&mut self.a_app[i]);
        }
        for i in 0..3 {
            fwd(&mut self.a_ico[i]);
        }
        let n = self.items();
        // Items 3 and 4 as the volume has them (volume 1's item 3 is
        // Parody's).
        for i in 3..n {
            fwd(&mut self.a_ico[i]);
            fwd(&mut self.a_app[i]);
        }
        match self.main_act {
            act::SET_NEW_GAME | act::NEW_GAME | act::PARODY => self.ne_sw = fwd(&mut self.a_back[3]),
            act::DATA_LOAD => {
                let r = self.animate_panel(n, self.dat_act, 3);
                if r >= 0 {
                    self.dat_sw = r;
                }
            }
            act::NEXT_DATA => {
                let r = self.animate_panel(n, self.next_act, 3);
                if r >= 0 {
                    self.next_sw = r;
                }
            }
            act::OPTION => {
                let r = self.animate_panel(n, self.opt_act, 4);
                if r >= 0 {
                    self.opt_sw = r;
                }
            }
            _ => {}
        }
        let y = eef::sub(self.rot_ico[1], f32::from_bits(ICON_SPIN));
        self.rot_ico[1] = y;
        if !eef::le(y, f32::from_bits(PI)) {
            self.rot_ico[1] = eef::sub(y, f32::from_bits(TWO_PI));
        }
        let y = self.rot_ico[1];
        if eef::lt(y, f32::from_bits(MINUS_PI)) {
            self.rot_ico[1] = eef::add(y, f32::from_bits(TWO_PI));
        }
        for i in 0..n {
            self.ico_dvec[i] = dummy_pos(&self.file, &self.a_ico_d[i], self.coord[i]);
            place(&mut self.a_ico[i], self.ico_dvec[i], Some(self.rot_ico));
        }
    }

    fn fade_out(&mut self) {
        self.transp = eef::sub(self.transp, self.alpha);
        if eef::lt(self.transp, 0.0) {
            self.transp = 0.0;
        }
        self.transp_ico = self.transp;
    }

    fn fade_in(&mut self) {
        self.transp_ico = eef::add(self.transp_ico, self.alpha);
        if !eef::le(self.transp_ico, 1.0) {
            self.transp_ico = 1.0;
        }
        self.transp = eef::add(self.transp, self.alpha);
        if !eef::le(self.transp, 1.0) {
            self.transp = 1.0;
        }
    }

    /// `AllTransparency` (0x00405420): the other items fade out while a
    /// panel is up, and back in 30 frames after it closes.
    pub fn all_transparency(&mut self, menu: &dyn SystemMenu) {
        match self.main_act {
            act::DATA_LOAD => {
                if self.dat_act == 0 {
                    self.fade_out();
                } else {
                    let w = self.trans_wait;
                    self.trans_wait += 1;
                    if w >= TRANS_WAIT && self.dat_act == 3 {
                        self.fade_in();
                    }
                }
                self.a_ico[0].localtp = self.transp_ico;
                self.a_ico[2].localtp = self.transp_ico;
                self.a_app[0].localtp = self.transp;
                self.a_app[2].localtp = self.transp;
            }
            act::OPTION => {
                if self.opt_act == 0 {
                    self.fade_out();
                } else if self.opt_act == 2 {
                    let step = eef::mul(2.0, self.alpha);
                    if menu.menu_type() == crate::seam::MENU_SETTINGS {
                        self.transp_win = eef::add(self.transp_win, step);
                        if !eef::le(self.transp_win, 1.0) {
                            self.transp_win = 1.0;
                        }
                    } else {
                        self.transp_win = eef::sub(self.transp_win, step);
                        if eef::lt(self.transp_win, 0.0) {
                            self.transp_win = 0.0;
                        }
                    }
                } else {
                    let w = self.trans_wait;
                    self.trans_wait += 1;
                    if w >= TRANS_WAIT && self.opt_act == 4 {
                        self.fade_in();
                        self.transp_win = 1.0;
                    }
                }
                self.a_ico[0].localtp = self.transp_ico;
                self.a_ico[1].localtp = self.transp_ico;
                self.a_win.localtp = self.transp_win;
                self.a_app[0].localtp = self.transp;
                self.a_app[1].localtp = self.transp;
            }
            act::NEXT_DATA => {
                if self.next_act == 0 {
                    self.fade_out();
                } else {
                    let w = self.trans_wait;
                    self.trans_wait += 1;
                    if w >= TRANS_WAIT && self.next_act == 3 {
                        self.fade_in();
                    }
                }
                for i in 0..3 {
                    self.a_ico[i].localtp = self.transp_ico;
                }
                for i in 0..3 {
                    self.a_app[i].localtp = self.transp;
                }
            }
            _ => {}
        }
        // Item 3 on volumes 2-4 (not while it is the one open), then the
        // Parody item.
        if self.later() && self.main_act != act::NEXT_DATA {
            self.a_ico[3].localtp = self.transp_ico;
            self.a_app[3].localtp = self.transp;
        }
        if self.paro_flg {
            let p = if self.later() { 4 } else { 3 };
            self.a_ico[p].localtp = self.transp_ico;
            self.a_app[p].localtp = self.transp;
        }
    }

    /// `AllDraw` (0x004059d0): backdrop, items, icons, then the screen-out
    /// and the window for the actions that show them.
    pub fn all_draw(&self, ctx: &mut Ctx) {
        let f = &self.file;
        for i in 0..3 {
            draw_anm(ctx, f, &self.a_back[i]);
        }
        for i in 0..3 {
            draw_anm(ctx, f, &self.a_app[i]);
        }
        for i in 0..3 {
            draw_anm(ctx, f, &self.a_ico[i]);
        }
        for i in 3..self.items() {
            draw_anm(ctx, f, &self.a_ico[i]);
            draw_anm(ctx, f, &self.a_app[i]);
        }
        let (back3, win) = match self.main_act {
            4..=5 | 12..=13 | 15 => (true, false),
            6 | 8 | 10 => (true, false),
            7 | 9 | 11 => (true, true),
            _ => (false, false),
        };
        if back3 {
            draw_anm(ctx, f, &self.a_back[3]);
        }
        if win {
            draw_anm(ctx, f, &self.a_win);
        }
    }
}
