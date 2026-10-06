//! The gate hack (gcmn menu.cpp): `GtHackMenu` (62, 0x005661e0), the load
//! task it starts (`ccThGtHackMenu`, 0x00566cb0), its button bar
//! (`GtHackMenuDisp`, 0x00566d00), and the screen it runs, `ccHackMenu`
//! (constructor 0x00564ff0, `Draw` 0x00565510, `Select` 0x00565ed0). The
//! gate's Warp opens it for a story area on this server with protection
//! (`protect[1, 3, 5, 7]`) not yet in `protectArea`; the area's `protect`
//! row is four (virus core, count) pairs, the cores key items of category
//! 15. The steps are in docs/engine/field-ui.md (the gate hack).

use std::rc::Rc;

use glam::Mat4;
use piney_desktop::anm::Anm;
use piney_desktop::assets::SceneFile;
use piney_desktop::eef::{add, div, from_int, mul, sub};

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Draw, Flow, MenuCtrl, SE_BUZZ};
use crate::menus::check_operate;
use crate::menus::gate::{AREA_COUNT, PROTECT_AREA};
use crate::menus::getitem::item_count;
use crate::menus::personal::{check_fade, delete_fade};
use crate::menus::system::extract_menu;
use crate::spr::{Obj, Spr, font_type, make_num, set_clm};
use crate::window::set_type;

/// `flGtHackMenu`'s file.
pub const HACK_FILE: &str = "xdhhack";
/// The virus cores' key-item category.
const CORE: i32 = 15;
/// `ccSpriteColorTable` rows the screen uses.
const COL_TEXT: usize = 7;
const COL_SHORT: usize = 18;
const COL_UNUSED: usize = 17;
const COL_ENOUGH: usize = 22;
/// `ccHackMenu`'s layers (`ccLayer::Init(127)`, `Init(128)`).
pub const HACK_LAYER: i16 = 127;
pub const MASK_LAYER: i16 = 128;

/// `hackCrystalOn`, `hackCrystalOnF`, `hackCrystalOff` (gcmn 0x00651730,
/// 0x00651770, 0x006517b0): slot `i`'s crystal `j` at `4 i + j`.
const CRYSTAL: [u8; 16] = [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 1, 2, 3, 4];

fn crystal_on(k: usize) -> String {
    format!("OBJ_xdhon{:02}", CRYSTAL[k])
}
fn crystal_on_f(k: usize) -> String {
    format!("OBJ_xdhon{:02}f", CRYSTAL[k])
}
fn crystal_off(k: usize) -> String {
    format!("OBJ_xdhoff{:02}", CRYSTAL[k])
}

/// The texts the gate hack reads, and its file once loaded.
#[derive(Clone, Default)]
pub struct HackTexts {
    /// `gtHackInfo`'s two lines.
    pub info: [Vec<u8>; 2],
    /// `gtHackStr`.
    pub bar: Vec<u8>,
    /// `xdhhack` (the runtime reads it with the menu's textures; tests may
    /// leave it out, and the screen then draws nothing 3D).
    pub scene: Option<Rc<SceneFile>>,
}

impl std::fmt::Debug for HackTexts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HackTexts")
            .field("info", &self.info)
            .field("bar", &self.bar)
            .field("scene", &self.scene.as_ref().map(|s| s.stem.clone()))
            .finish()
    }
}

impl HackTexts {
    /// The volume's (`gtHackInfo`'s two lines, `gtHackStr`); the scene is
    /// set by the field UI.
    pub fn of(volume: piney_data::volume::Volume) -> HackTexts {
        let t = piney_data::tables::fieldui::of(volume);
        HackTexts {
            info: [crate::tables::piece(t.hack_info(), 0), crate::tables::piece(t.hack_info(), 1)],
            bar: piney_data::tables::sjis::encode(t.hack_bar()),
            scene: None,
        }
    }
}

/// `ccHackMenu` (0xa0 bytes).
#[derive(Clone)]
pub struct HackMenu {
    /// +0x00 `crystal`, +0x04 `frame`, +0x08 `complete`: which animations
    /// draw.
    pub crystal: bool,
    pub frame: bool,
    pub complete: bool,
    /// +0x0c `fontStatus`: 0 out, 1 coming in, 2 in, 3 going out; +0x10
    /// `fontAlpha`.
    pub font_status: i32,
    pub font_alpha: i32,
    /// +0x14 `select`: the slot in front; +0x18 `selectCnt`: a turn's
    /// frames left (-10 turning left, 10 right); +0x1c `selectOld`.
    pub select: i32,
    pub select_cnt: i32,
    pub select_old: i32,
    /// +0x20 `setNum[4]`: the cores put in each slot.
    pub set_num: [i32; 4],
    /// +0x30 `mat`: OBJ_xdhroll's matrix as the constructor found it.
    pub mat: Mat4,
    /// +0x7c..+0x8c: camAnm, backAnm, crystalAnm, frameAnm, compAnm.
    pub cam: Anm,
    pub back: Anm,
    pub crystals: Anm,
    pub frame_anm: Anm,
    pub comp: Anm,
    /// The area's `protect` row.
    pub protect: [i32; 8],
    /// +0x90 `hackMask` on TEX_xdhroma1 (layer 128).
    pub mask: Spr,
    /// The global `font` (ccFont), as the screen sets it.
    pub font: Spr,
}

fn anm(label: &'static str, scene: Option<&Rc<SceneFile>>, name: &str, m: &mut MenuCtrl) -> Anm {
    let mut a = Anm::labelled(label);
    set_anm(&mut a, scene, name, m);
    // `if (anmIndex) _AnimateForward(frameSpd)`.
    forward(&mut a, m);
    a
}

fn set_anm(a: &mut Anm, scene: Option<&Rc<SceneFile>>, name: &str, m: &mut MenuCtrl) {
    m.note(|| format!("[\"anm_set\",\"{}\",\"{name}\"]", a.label));
    if let Some(s) = scene {
        a.set(s, name);
    }
}

/// `if (anmIndex) _AnimateForward(frameSpd)`: true when a play-once
/// animation ended.
fn forward(a: &mut Anm, m: &mut MenuCtrl) -> bool {
    if !a.is_set() {
        return false;
    }
    m.note(|| format!("[\"anm_fwd\",\"{}\"]", a.label));
    a.forward()
}

impl HackMenu {
    /// `new ccHackMenu(ccs)`.
    pub fn new(scene: Option<&Rc<SceneFile>>, protect: [i32; 8], m: &mut MenuCtrl) -> HackMenu {
        let cam = anm("cam", scene, "ANM_xdhcamer", m);
        let back = anm("back", scene, "ANM_xdhback0", m);
        let mut crystals = anm("crystal", scene, "ANM_xdhcrys0", m);
        // GetSubstAdrs("OBJ_xdhon*"): the lit crystals hidden; then each
        // slot's unlit ones past its count.
        for o in crystals.objects_prefixed("OBJ_xdhon") {
            crystals.set_disp(o, false);
        }
        for i in 0..4 {
            for j in 0..4 {
                if j >= protect[2 * i + 1]
                    && let Some(o) = crystals.object_named(&crystal_off(4 * i + j as usize))
                {
                    crystals.set_disp(o, false);
                }
            }
        }
        let mat = crystals.object_named("OBJ_xdhroll").map_or(Mat4::IDENTITY, |o| crystals.local_of(o));
        let frame_anm = anm("frame", scene, "ANM_xdhfram0", m);
        let comp = anm("comp", scene, "ANM_xdhcomp0", m);
        // hackMask: SetPrim(4, 0), SetTex(xdhhack, TEX_xdhroma1) on the mask
        // layer, cells 56 x 48 (drawn 56 x 50), four a row.
        let mut mask = Spr::new(Obj::HackMask, 64);
        mask.set_grid(56, 48, 56.0, 50.0, 0, 0, 4);
        HackMenu {
            crystal: false,
            frame: false,
            complete: false,
            font_status: 0,
            font_alpha: 0,
            select: 0,
            select_cnt: 0,
            select_old: 0,
            set_num: [0; 4],
            mat,
            cam,
            back,
            crystals,
            frame_anm,
            comp,
            protect,
            mask,
            font: Spr::new(Obj::SysFont, 256),
        }
    }

    /// `ccHackMenu::Select()`: the button that ended it (OK or cancel as
    /// assigned), else 0.
    pub fn select(&mut self, x: &mut Ctx) -> i32 {
        if !check_operate(x, 15, 0) {
            return 0;
        }
        if self.select_cnt != 0 {
            self.select_cnt += if self.select_cnt > 0 { -1 } else { 1 };
            return 0;
        }
        let (push, rep) = (x.pad.push.bits(), x.pad.repeat.bits());
        let (ok, cancel) = (x.assign(5), x.assign(6));
        let p = self.protect;
        if push & ok != 0 {
            if (0..4).all(|k| p[2 * k + 1] == self.set_num[k]) {
                x.se(222);
                return ok as i32;
            }
            x.se(220);
            return 0;
        }
        if push & cancel != 0 {
            return cancel as i32;
        }
        let s = self.select as usize;
        if rep & 0x8000 != 0 {
            x.se(221);
            self.select_old = self.select;
            self.select = (self.select + 1) % 4;
            self.select_cnt = -10;
        } else if rep & 0x2000 != 0 {
            x.se(221);
            self.select_old = self.select;
            self.select -= 1;
            if self.select < 0 {
                self.select = 3;
            }
            self.select_cnt = 10;
        } else if rep & 0x1000 != 0 {
            if item_count(x, 0, CORE, p[2 * s]) - self.set_num[s] > 0 {
                self.set_num[s] += 1;
                if p[2 * s + 1] < self.set_num[s] {
                    self.set_num[s] = p[2 * s + 1];
                    x.se(220);
                } else {
                    x.se(219);
                }
            }
        } else if rep & 0x4000 != 0 {
            self.set_num[s] -= 1;
            if self.set_num[s] < 0 {
                self.set_num[s] = 0;
                x.se(220);
            } else {
                x.se(219);
            }
        }
        0
    }

    /// `ccHackMenu::Draw()`: the animations stepped and drawn, the texts
    /// queued.
    pub fn draw(&mut self, m: &mut MenuCtrl, x: &mut Ctx) {
        // `if (anmIndex) r = _AnimateForward(frameSpd)`; Draw; at the play-
        // once start's end, the loop set for the next frame.
        let ended = forward(&mut self.back, m);
        m.note(|| "[\"anm_draw\",\"back\"]".into());
        let back = self.back.clone();
        if ended {
            let scene = x.texts.hack.scene.clone();
            set_anm(&mut self.back, scene.as_ref(), "ANM_xdhback1", m);
        }
        let mut roll = None;
        if self.crystal {
            forward(&mut self.crystals, m);
            // -pi/2 a slot, and the turn's share: wrapped to -pi..pi.
            const QUARTER: f32 = f32::from_bits(0xbfc9_0fdb);
            const PI: f32 = f32::from_bits(0x4049_0fdb);
            let a = mul(QUARTER, from_int(self.select));
            let b = div(mul(from_int(self.select_cnt), QUARTER), 10.0);
            let mut angle = add(a, b);
            if angle > PI {
                angle = sub(sub(angle, PI), PI);
            } else if angle < -PI {
                angle = add(PI, add(PI, angle));
            }
            if let Some(o) = self.crystals.object_named("OBJ_xdhroll") {
                self.crystals.set_local(o, self.mat * Mat4::from_rotation_z(angle));
            }
            let mut disp = Vec::with_capacity(48);
            for i in 0..4 {
                for j in 0..4i32 {
                    let k = 4 * i + j as usize;
                    // (off, on, on-f)
                    let (off, on) = if j >= self.protect[2 * i + 1] {
                        (false, false)
                    } else if j < self.set_num[i] {
                        (false, true)
                    } else {
                        (true, false)
                    };
                    for (name, show) in [(crystal_off(k), off), (crystal_on(k), on), (crystal_on_f(k), on)] {
                        if let Some(o) = self.crystals.object_named(&name) {
                            self.crystals.set_disp(o, show);
                        }
                        disp.push(if show { b'3' } else { b'0' });
                    }
                }
            }
            roll = Some(angle);
            let d = String::from_utf8(disp).unwrap_or_default();
            m.note(|| format!("[\"anm_draw\",\"crystal\",\"{d}\",\"{:08x}\"]", angle.to_bits()));
        }
        if self.frame {
            forward(&mut self.frame_anm, m);
            m.note(|| "[\"anm_draw\",\"frame\"]".into());
        }
        if self.complete {
            forward(&mut self.comp, m);
            m.note(|| "[\"anm_draw\",\"comp\"]".into());
        }
        m.draws.push(Draw::Hack(Box::new(HackDraw {
            cam: self.cam.clone(),
            back,
            crystals: self.crystal.then(|| self.crystals.clone()),
            frame: self.frame.then(|| self.frame_anm.clone()),
            comp: self.complete.then(|| self.comp.clone()),
            roll,
        })));
        match self.font_status {
            0 => self.font_alpha = 0,
            1 => {
                self.font_alpha += 16;
                if self.font_alpha >= 128 {
                    self.font_alpha = 128;
                    self.font_status = 2;
                }
            }
            2 => self.font_alpha = 128,
            3 => {
                self.font_alpha -= 6;
                if self.font_alpha <= 0 {
                    self.font_alpha = 0;
                    self.font_status = 0;
                }
            }
            _ => {}
        }
        if self.font_alpha == 0 {
            return;
        }
        self.cores(x);
        self.slot();
    }

    /// The 26 cores, "A " to "Z " and the count the player has left of each
    /// (less what the slot needing it holds): the needed ones in colour 22
    /// when there are enough, 18 when not; the rest 17.
    fn cores(&mut self, x: &mut Ctx) {
        let f = &mut self.font;
        font_type(f, 2);
        let p = self.protect;
        for col in 0..2 {
            for row in 0..13 {
                let id = col * 13 + row;
                let have = item_count(x, 0, CORE, id);
                let mut left = have;
                let mut need = 0;
                for k in 0..4 {
                    if p[2 * k + 1] != 0 && p[2 * k] == id {
                        need = p[2 * k + 1];
                        left = have - self.set_num[k];
                        break;
                    }
                }
                let c = if need == 0 {
                    COL_UNUSED
                } else if have >= need {
                    COL_ENOUGH
                } else {
                    COL_SHORT
                };
                f.set_colour(c);
                f.set_alpha(self.font_alpha);
                f.dx = add(365.0, mul(61.0, from_int(col)));
                f.dy = add(58.0, mul(f32::from_bits(0x41ab_3333), from_int(row)));
                f.make_str(&[b'A' + id as u8, b' ']);
                make_num(f, 2, left);
            }
        }
    }

    /// The slot in front: its core's letter and the count still to put in,
    /// faded out and in across a turn.
    fn slot(&mut self) {
        let mut s = self.select;
        let mut a = self.font_alpha - (self.font_alpha >> 3);
        if self.select_cnt != 0 {
            let c = self.select_cnt.abs();
            if c < 5 {
                a = a * (5 - c) / 5;
            } else {
                a = a * (c - 5) / 5;
                s = self.select_old;
            }
        }
        let w = &mut self.mask;
        w.set_colour(COL_TEXT);
        w.set_alpha(a);
        let s = s.clamp(0, 3) as usize;
        let need = self.protect[2 * s + 1];
        if need > 0 {
            let rest = need - self.set_num[s];
            w.dx = 180.0;
            w.dy = 272.0;
            w.make_packet(rest / 10 + 26);
            w.make_packet(rest % 10 + 26);
            w.dx = 116.0;
            w.dy = 272.0;
            w.make_packet(self.protect[2 * s]);
        }
    }

    /// The texts' packets as `SendPacket` sends them (the mask's now, the
    /// font's at the flip).
    fn send(&mut self, m: &mut MenuCtrl) {
        let mask = self.mask.take();
        let font = self.font.take();
        m.draws.push(Draw::Send(mask));
        if !font.is_empty() {
            m.draws.push(Draw::Send(font));
        }
    }
}

/// What the renderer needs of the screen's 3D this frame: the animations
/// as `Draw` left them.
#[derive(Clone)]
pub struct HackDraw {
    pub cam: Anm,
    pub back: Anm,
    pub crystals: Option<Anm>,
    pub frame: Option<Anm>,
    pub comp: Option<Anm>,
    /// The ring's turn, radians (the crystals drawn).
    pub roll: Option<f32>,
}

impl std::fmt::Debug for HackDraw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HackDraw").field("roll", &self.roll).finish_non_exhaustive()
    }
}

/// Where `GtHackMenu`'s own frames resume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// The screen breaking up and going black.
    Fall,
    /// `ccBreathThread(2)` after the fall: frames left.
    Black(u8),
    /// The intro's frames done (55).
    Intro(u8),
    /// The crystals' frames done (60).
    Crystals(u8),
    Select,
    /// OK: ANM_xdhcomp0's frames done (90).
    Complete(u8),
    /// The fade to black.
    Out,
    /// The two `ccBreathThread(2)` after it: frames done (4).
    Gone(u8),
    /// Cancel: the frames of noise.
    Noise,
    /// Its last two `Disp; Breath`: done so far.
    Tail(u8),
    /// The shut's frame (`still` was 1: the flips back after it).
    Shut(bool),
}

/// `Disp; ccBreathThread(1)`, resuming at `r`.
fn breathe(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Flow {
    crate::disp::disp(m, x);
    wait(r)
}

/// A breath without the menu's `Disp` (inside `ccBreathThread(2)`).
fn wait(r: Resume) -> Flow {
    Flow::Breathed(Cont { woke: false, cursors: false, after: After::Hack(r) })
}

/// `ccScFade::EntryFlash(menuFade, 10, 0x50e0c000, ...)`.
fn flash(m: &mut MenuCtrl) {
    m.menu_fade.entry_flash(10, 0x50e0_c000);
}

/// `ccNoiz::SetNoiz(n, n, n)` on the menu's noise.
fn noise(m: &mut MenuCtrl, x: &mut Ctx, n: i32) {
    crate::disp::noiz(m, x, crate::Noise::Set(n, n, n));
}

fn noise_rand(m: &mut MenuCtrl, x: &mut Ctx) {
    let r = m.rng.rand();
    // rand() % 4 as C has it (rand() is never negative).
    noise(m, x, r % 4 + 3);
}

/// `ccHackMenu::Draw(); Disp; Breath`, resuming at `r`.
fn show(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Flow {
    if let Some(mut h) = m.hack.take() {
        h.draw(m, x);
        h.send(m);
        m.hack = Some(h);
    }
    breathe(m, x, r)
}

/// The area's `protect` row (`WORLD_MAN::GetEventAreaInfo()`).
fn protect(m: &MenuCtrl, x: &Ctx) -> [i32; 8] {
    x.texts.words.event(m.generated.event).map_or([0; 8], |e| e.protect)
}

/// `GtHackMenu` (62).
pub fn gt_hack_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            m.menu_status = 3;
            m.exception_disp = 0;
            m.wait_count = 30;
            m.temp[4] = 0;
            m.proccess += 1;
            x.req.push(Request::GateHackSound(0));
            Flow::Done
        }
        1 => {
            m.wait_count += 1;
            if m.wait_count < 25 {
                let names = x.save.names();
                let [l0, l1] = x.texts.hack.info.clone();
                m.msg.disp_info([Some(&l0), Some(&l1), None, None], &names);
            } else if m.wait_count >= 31 {
                m.wait_count = 0;
                m.menu_fade.entry_flash2(2, 8, 0x1c40_40ff);
                x.se(SE_BUZZ);
                m.temp[4] += 1;
            }
            if m.temp[4] >= 4 {
                m.bg_status = 3;
                m.proccess += 1;
            }
            Flow::Done
        }
        2 => {
            // ccStartThread(ccThGtHackMenu): its flag up until the load is
            // done, which the port's first run of it finishes.
            m.note(|| "[\"start_thread\",\"hack\"]".into());
            m.fade = -1;
            m.wait_count = 0;
            run(m, x, Resume::Fall)
        }
        _ => Flow::Done,
    }
}

/// After one of `GtHackMenu`'s own breaths: `Some` when it breathed again,
/// else the task's `Disp` ends the frame.
pub fn resume(m: &mut MenuCtrl, r: Resume, x: &mut Ctx) -> Option<Cont> {
    match run(m, x, r) {
        Flow::Breathed(c) => Some(c),
        Flow::Done => None,
    }
}

/// `GtHackMenu`'s proccess 2 from `r` to its next breath or its end.
fn run(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Flow {
    match r {
        Resume::Fall => {
            let wc = m.wait_count;
            if (26..60).contains(&wc) && wc % 6 == 0 {
                noise_rand(m, x);
            }
            if wc == 45 {
                m.fade = m.menu_fade.entry_fade(20, 0, 0x8000_0000) as i16;
            }
            if m.fade < 0 || check_fade(m, i32::from(m.fade)) {
                m.wait_count += 1;
                return breathe(m, x, Resume::Fall);
            }
            m.note(|| "[\"close_chat\"]".into());
            noise(m, x, 0);
            m.still = 0;
            x.req.push(Request::Still(false));
            x.req.push(Request::WorldHidden(true));
            wait(Resume::Black(1))
        }
        Resume::Black(1) => wait(Resume::Black(0)),
        Resume::Black(_) => {
            delete_fade(m, i32::from(m.fade));
            // The load task's flag is down: ccDeleteThread.
            m.note(|| "[\"delete_thread\"]".into());
            flash(m);
            let p = protect(m, x);
            let scene = x.texts.hack.scene.clone();
            m.hack = Some(Box::new(HackMenu::new(scene.as_ref(), p, m)));
            show(m, x, Resume::Intro(1))
        }
        Resume::Intro(n) if n < 55 => show(m, x, Resume::Intro(n + 1)),
        Resume::Intro(_) => {
            flash(m);
            if let Some(h) = &mut m.hack {
                h.crystal = true;
            }
            show(m, x, Resume::Crystals(1))
        }
        Resume::Crystals(n) if n < 60 => show(m, x, Resume::Crystals(n + 1)),
        Resume::Crystals(_) => {
            flash(m);
            if let Some(h) = &mut m.hack {
                h.frame = true;
                h.font_status = 1;
            }
            m.menu_status = 1;
            m.exception_disp = 1;
            run(m, x, Resume::Select)
        }
        Resume::Select => {
            let s = m.hack.as_mut().map_or(0, |h| h.select(x));
            if s == 0 {
                return show(m, x, Resume::Select);
            }
            m.hack_result = s;
            if s != x.assign(6) as i32 {
                if let Some(h) = &mut m.hack {
                    h.complete = true;
                }
                return show(m, x, Resume::Complete(1));
            }
            out(m, x)
        }
        Resume::Complete(n) if n < 90 => show(m, x, Resume::Complete(n + 1)),
        Resume::Complete(_) => out(m, x),
        Resume::Out => {
            if check_fade(m, i32::from(m.fade)) {
                return show(m, x, Resume::Out);
            }
            wait(Resume::Gone(1))
        }
        Resume::Gone(1) => wait(Resume::Gone(2)),
        Resume::Gone(2) => {
            delete_fade(m, i32::from(m.fade));
            m.hack = None;
            wait(Resume::Gone(3))
        }
        Resume::Gone(3) => wait(Resume::Gone(4)),
        Resume::Gone(_) => {
            if m.hack_result == x.assign(6) as i32 {
                x.req.push(Request::GateHackSound(1));
                x.req.push(Request::WorldHidden(false));
                // `$a2` still holds the 0x80000000 bgColor was or'd with:
                // from black to clear.
                m.fade = m.menu_fade.entry_fade(20, 0x8000_0000, 0) as i16;
                x.req.push(Request::WakeAll);
                m.wait_count = 0;
                run(m, x, Resume::Noise)
            } else {
                hacked(m, x);
                Flow::Done
            }
        }
        Resume::Noise => {
            if m.wait_count % 6 == 0 {
                noise_rand(m, x);
            }
            if m.wait_count < 40 {
                m.wait_count += 1;
                return breathe(m, x, Resume::Noise);
            }
            noise(m, x, 0);
            delete_fade(m, i32::from(m.fade));
            x.req.push(Request::GateHackSound(2));
            breathe(m, x, Resume::Tail(1))
        }
        Resume::Tail(1) => breathe(m, x, Resume::Tail(2)),
        Resume::Tail(_) => {
            m.menu_next = -1;
            m.menu_status = 3;
            m.bg_status = 3;
            if m.still == 1 {
                x.req.push(Request::WakeAll);
                breathe(m, x, Resume::Shut(true))
            } else {
                breathe(m, x, Resume::Shut(false))
            }
        }
        Resume::Shut(woke) => {
            if woke {
                m.still = 0;
                x.req.push(Request::Still(false));
            }
            m.first_time = 0;
            Flow::Done
        }
    }
}

/// OK passed or cancel: the fade to black, the frame closing, the texts
/// out, until the fade is done.
fn out(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    m.fade = m.menu_fade.entry_fade(20, 0, 0x8000_0000) as i16;
    m.menu_status = 3;
    let scene = x.texts.hack.scene.clone();
    if let Some(mut h) = m.hack.take() {
        h.frame = true;
        set_anm(&mut h.frame_anm, scene.as_ref(), "ANM_xdhfram1", m);
        h.font_status = 3;
        m.hack = Some(h);
    }
    run(m, x, Resume::Out)
}

/// The cores put in: the area opened.
fn hacked(m: &mut MenuCtrl, x: &mut Ctx) {
    let p = protect(m, x);
    for k in 0..4 {
        if p[2 * k + 1] != 0 {
            crate::items::del_item(x.save, 0, CORE, p[2 * k], p[2 * k + 1]);
        }
    }
    let area = x.texts.words.event(m.generated.event).map_or(0, |e| e.code);
    if area >= 0 {
        let at = PROTECT_AREA + 4 * (area as usize >> 5);
        let v = x.save.save.i32(at) as u32 | 1 << (area & 31);
        x.save.save.set_i32(at, v as i32);
    }
    x.req.push(Request::DeleteNoPartyMember);
    x.req.push(Request::GateHacked);
    x.req.push(Request::GateHackSound(2));
    let v = (i32::from(x.save.save.i16(AREA_COUNT)) + 1).min(10000);
    x.save.save.set_i16(AREA_COUNT, v as i16);
    x.req.push(Request::GoToArea([m.temp[0], m.temp[1], m.temp[2]]));
}

/// `GtHackMenuDisp`: the button bar under the screen, "Rotate",
/// "Add/Subtract", "Execute", "Cancel" beside the pad's marks.
pub fn gt_hack_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let a = m.alpha;
    extract_menu(m, x.texts.hack.bar.clone());
    let k = &mut m.kanji;
    set_clm(k, 12, 0, 0, 1);
    k.set_colour(COL_TEXT);
    k.set_alpha(a);
    for (i, dx) in [68.0, 180.0, 314.0, 418.0].into_iter().enumerate() {
        k.dx = dx;
        k.dy = 402.0;
        k.make_packet(i as i32);
    }
    let w = &mut m.win;
    set_type(w, 2);
    // The D-pad twice: its left and right lit by "Rotate", its up and down
    // by "Add/Subtract".
    w.set_colour(COL_SHORT);
    w.set_alpha(a);
    for (dx, dy, c) in [(32.0, 404.0, 20), (50.0, 404.0, 22), (153.0, 395.0, 21), (153.0, 413.0, 23)] {
        w.dx = dx;
        w.dy = dy;
        w.make_packet(c);
    }
    w.set_colour(COL_TEXT);
    for (dx, dy, c) in [
        (144.0, 404.0, 20),
        (162.0, 404.0, 22),
        (41.0, 395.0, 21),
        (41.0, 413.0, 23),
        (296.0, 402.0, 19),
        (400.0, 402.0, 16),
    ] {
        w.dx = dx;
        w.dy = dy;
        w.make_packet(c);
    }
}
