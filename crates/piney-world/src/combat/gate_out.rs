//! A gate-hacked arrival: `ccPlayer::GateHackingOut()` (gcmn 0x0059bc00) and
//! the globals around it. After the gate hack (menu 62's OK), a field entered
//! from the town keeps `gtHackFlag`, and its set-up loads the field's
//! `x<name>.ccs`, builds Kite in act 24 with `ghoFlag` set and hides the
//! party for 65 frames. `GateHackingOut` then runs the camera along the
//! file's markers (`progCtrlFlag` 0, 1, 7, 8; 2-6 are never reached) and
//! gives the blades `ghoCamArmsT` as their transparency. `ccThGameCtrl`, the
//! party's AI and `gate_hack_anim` wait on `ghoFlag` (docs/engine/field-walk.md).

use std::rc::Rc;

use piney_desktop::anm::Anm;
use piney_desktop::assets::SceneFile;

use crate::camera::{Camera, id};
use crate::ee::{self, F, ONE, V4};

/// `ccAddRequestFileListSpc`'s `gateHackingOutID` by `game.field` (gcmn
/// 0x005a11dc-0x005a1400), and `GateHackOutCcsName[id]` (0x006540c0):
/// the camera file `x<name>` and its animation `ANM_str<name>0`.
pub const GATE_OUT: [(i32, &str); 19] = [
    (19, "7404cam"),
    (22, "7407cam"),
    (23, "7408cam"),
    (25, "7410cam"),
    (27, "7412cam"),
    (16, "7424cam"),
    (44, "7429cam"),
    (45, "7430cam"),
    (46, "7431cam"),
    (48, "7433cam"),
    (50, "7435cam"),
    (52, "7437cam"),
    (66, "7447cam"),
    (71, "7451cam"),
    (73, "7453cam"),
    (74, "7454cam"),
    (76, "7456cam"),
    (77, "7457cam"),
    (91, "7467cam"),
];

/// `gateHackingOutID` for `field`: its index in [`GATE_OUT`], -1 for none.
pub fn gate_hacking_out_id(field: i32) -> i32 {
    GATE_OUT.iter().position(|&(f, _)| f == field).map_or(-1, |i| i as i32)
}

/// `ccGateHackOutCcsName()`'s name for `gateHackingOutID`.
pub fn ccs_name(id: i32) -> Option<&'static str> {
    usize::try_from(id).ok().and_then(|i| GATE_OUT.get(i)).map(|&(_, n)| n)
}

/// `volumeNum`: this disc is volume 1 (the hacked arrival needs below 4).
pub const VOLUME_NUM: i32 = 1;

/// `ccClearGtHack` (main 0x001b77c0), from `ccStartThEvent`: whether
/// `gtHackFlag` stays: in a field, or a dungeon of type 8 or 9
/// (`WORLD_MAN::GetDungeonType`, `dungeonType[game.dungeon]`), entered from
/// a town (`areaPrev` 0).
pub fn keeps_gt_hack(area: i32, area_prev: i32, dungeon_type: i32) -> bool {
    let place = area == 1 || (area == 2 && matches!(dungeon_type, 8 | 9));
    place && area_prev == 0
}

/// Where the hacked arrival's checks look (`ccPlayer::ccPlayer`,
/// `ccAddRequestFileListSpc`, `ccFellow::Initialize`): `game.area` 1, or
/// `WORLD_MAN`'s field type 4 with `game.dungeon` (+0x28) 0.
fn arrival_place(area: i32, field_type: i32, dungeon: i32) -> bool {
    area == 1 || (field_type == 4 && dungeon == 0)
}

/// `ccAddRequestFileListSpc` (gcmn 0x005a10e0)'s gate-hack part: from a
/// town into [`arrival_place`] with `gtHackFlag`, `gateHackingOutID` by
/// `game.field` ([`gate_hacking_out_id`], -1 for an area without one) and
/// the camera file `x<name>.ccs` on the file list. None when it does not
/// look (the ID then keeps whatever it was).
pub fn hack_out_id(area: i32, area_prev: i32, field_type: i32, dungeon: i32, field: i32, gt_hack: bool) -> Option<i32> {
    (arrival_place(area, field_type, dungeon) && area_prev == 0 && gt_hack).then(|| gate_hacking_out_id(field))
}

/// `ccAI::ccAI`'s `arrivalChatCnt` (+0x80, gcmn 0x0057c8f4): 150 in a
/// town, and in a field (or a dungeon of a field of type 4 with
/// `game.dungeon` 0) entered from a town unless the gate was hacked; else
/// -1.
pub fn arrival_chat(area: i32, area_prev: i32, field_type: i32, dungeon: i32, gt_hack: bool) -> i16 {
    let place = area == 1 || (area == 2 && field_type == 4 && dungeon == 0);
    match area {
        0 => 150,
        _ if place && area_prev == 0 && !gt_hack => 150,
        _ => -1,
    }
}

/// 0.1 (0x3dcccccd), `ghoCamArmsT`'s step.
const ARMS_STEP: F = 0x3dcc_cccd;
/// 7.5, `speedRate`'s divisor in step 4.
const RATE_DIV: F = 0x40f0_0000;
/// 0.029296875 (0x3cf00000), where step 4 snaps to 1.
const RATE_SNAP: F = 0x3cf0_0000;
/// 100 and 120 (+0x224), 1.25.
const K100: F = 0x42c8_0000;
const K120: F = 0x42f0_0000;
const K125: F = 0x3fa0_0000;

/// Which of `ghoCam`'s markers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker {
    /// `OBJ_marker_cam1` (`ghoCamPosC`): the camera's eye.
    Cam,
    /// `OBJ_marker_target0` (`ghoCamPosV`): what it looks at.
    Target,
    /// `OBJ_marker_man0` (`ghoCamPosM`): where Kite stands.
    Man,
}

impl Marker {
    pub fn name(self) -> &'static str {
        match self {
            Marker::Cam => "OBJ_marker_cam1",
            Marker::Target => "OBJ_marker_target0",
            Marker::Man => "OBJ_marker_man0",
        }
    }
}

/// `ghoCam` as `GateHackingOut` uses it.
pub trait GhoCam {
    /// `ghoCam = new ccAnm; SetAnm(GetChunkAdrsF(x<name>, ANM_str<name>0))`:
    /// whether an animation is set (`anmIndex` +0xac not 0).
    fn start(&mut self) -> bool;
    /// `_AnimateForward(frameSpd)`: whether the animation has ended.
    fn forward(&mut self) -> bool;
    /// A marker's world position: `sceVu0ApplyMatrix(p, parent->lwMatrix,
    /// marker->matrix[3])`, the last pose.
    fn marker(&self, m: Marker) -> Option<V4>;
    /// `delete ghoCam`.
    fn stop(&mut self);
}

/// `ghoCam` over the camera file the set-up loaded.
#[derive(Clone, Default)]
pub struct GhoAnim {
    /// `x<name>.ccs` and its name (`ccGateHackOutCcsName`).
    pub file: Option<Rc<SceneFile>>,
    pub name: &'static str,
    /// The `ccAnm` while it exists.
    pub cam: Option<Anm>,
}

impl GhoCam for GhoAnim {
    fn start(&mut self) -> bool {
        let mut cam = Anm::new();
        if let Some(f) = &self.file {
            cam.set(f, &format!("ANM_str{}0", self.name));
        }
        let set = cam.is_set();
        self.cam = Some(cam);
        set
    }
    fn forward(&mut self) -> bool {
        self.cam.as_mut().is_some_and(|c| c.is_set() && c.forward())
    }
    fn marker(&self, m: Marker) -> Option<V4> {
        let cam = self.cam.as_ref()?;
        let obj = cam.object_named(m.name())?;
        let (_, w, _) = cam.instances_all().into_iter().find(|(o, _, _)| *o == obj)?;
        let t = w.w_axis;
        Some([t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE])
    }
    fn stop(&mut self) {
        self.cam = None;
    }
}

/// The gate-hacked arrival's globals and the `ccPlayer` members only
/// `GateHackingOut` uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GateState {
    /// `ghoFlag` (main 0x00378cc0): set by `ccPlayer::ccPlayer` on a hacked
    /// arrival, cleared by `GateHackingOut`'s last step.
    pub flag: bool,
    /// `ghoCamArmsT` (0x00378cd4): the blades' transparency.
    pub arms_t: F,
    /// The blades' `dispSW`: 3 (shown) or 0.
    pub arms_disp: bool,
    /// `ccPlayer` +0x114 (a counter), +0x224 (100, then toward 120).
    pub cnt: i32,
    pub k224: F,
}

/// A gate-hacked arrival's state and its camera animation.
#[derive(Clone, Default)]
pub struct GateOut {
    pub state: GateState,
    pub anim: GhoAnim,
}

impl GateOut {
    /// `ghoFlag`.
    pub fn flag(&self) -> bool {
        self.state.flag
    }

    /// The blades' transparency in acts 23 and 24 (`ghoCamArmsT`, 0 while
    /// their `dispSW` is 0).
    pub fn arms_alpha(&self) -> f32 {
        if self.state.arms_disp { ee::f(self.state.arms_t) } else { 0.0 }
    }

    /// `ccPlayer::GateHackingOut()` on this state and animation.
    pub fn run(&mut self, camera: &mut Camera, k: Kite) {
        gate_hacking_out(&mut self.state, &mut self.anim, camera, k);
    }
}

/// The camera and Kite from the markers: the eye (`cameraSetPos`, kept at
/// +0x250), the target (`cameraSetView`, +0x240), his position, then
/// `cameraParamChange(+0x250, +0x240)`.
fn place(cam: &dyn GhoCam, camera: &mut Camera, k: &mut Kite) {
    let n = camera.cam_id;
    if let Some(c) = cam.marker(Marker::Cam) {
        camera.cam_mut(n).pos = c;
        *k.pos_cam = c;
    }
    if let Some(v) = cam.marker(Marker::Target) {
        camera.cam_mut(n).view = v;
        *k.pos_view = v;
    }
    if let Some(m) = cam.marker(Marker::Man) {
        *k.pos = m;
    }
    param_change(camera, *k.pos_cam, *k.pos_view);
}

/// `cameraParamChange(pos, view)` (main 0x00162870) on the active camera:
/// `dist` the eye's distance from the target, the heading and pitch
/// (`deg`, 16-bit; the pitch less 16384) and `rot` from them.
pub fn param_change(camera: &mut Camera, pos: V4, view: V4) {
    let mut d = ee::vsub(pos, view);
    d[3] = ONE;
    let dist = ee::sqrtf(ee::dot(d, d));
    let z = d[2];
    d[2] = 0;
    let across = ee::sqrtf(ee::dot(d, d));
    let pitch = ee::atan2f(across, ee::mul(0xbf80_0000, z));
    let heading = ee::atan2f(d[0], d[1]);
    let deg_pitch = ee::rad2deg(pitch).wrapping_sub(16384);
    let deg_heading = ee::rad2deg(heading);
    let c = camera.active_mut();
    c.deg = [deg_heading, deg_pitch];
    c.rot[2] = ee::deg2rad(deg_heading);
    c.rot[0] = ee::deg2rad(deg_pitch);
    c.dist = dist;
}

/// What `GateHackingOut` reads and writes of Kite.
pub struct Kite<'a> {
    /// `ccChar.pos` (+0x40).
    pub pos: &'a mut V4,
    /// `ccSpcChar` +0xe0 flags ([`piney_battle::chara::spc_flag`]).
    pub flags: &'a mut u32,
    /// `actNum`.
    pub act: &'a mut i16,
    /// `speedRate` (+0x108).
    pub speed_rate: &'a mut F,
    /// `progCtrlFlag` (+0x208), `cameraFlag` (+0x200 bit 0).
    pub prog: &'a mut i32,
    pub camera_flag: &'a mut bool,
    /// +0x250 (the eye `GateHackingOut` keeps) and `posView` (+0x240).
    pub pos_cam: &'a mut V4,
    pub pos_view: &'a mut V4,
}

/// `ccPlayer::GateHackingOut()` (gcmn 0x0059bc00): one frame.
pub fn gate_hacking_out(g: &mut GateState, cam: &mut dyn GhoCam, camera: &mut Camera, mut k: Kite) {
    use piney_battle::chara::spc_flag::{MOVE, RESTRAINT, STOP};
    match *k.prog {
        0 => {
            let set = cam.start();
            camera.change_camera(id::EVENT);
            if set {
                cam.forward();
            }
            place(cam, camera, &mut k);
            g.arms_t = 0;
            g.cnt = 0;
            *k.speed_rate = ONE;
            *k.camera_flag = true;
            g.k224 = K100;
            *k.flags = (*k.flags | STOP) & !MOVE;
            g.arms_disp = false;
            *k.prog += 1;
        }
        1 => {
            if cam.forward() {
                *k.prog = 7;
                g.arms_disp = true;
            }
            place(cam, camera, &mut k);
        }
        2 => {
            if g.cnt >= 41 {
                g.cnt = 0;
                *k.prog += 1;
            } else {
                g.cnt += 1;
            }
        }
        3 => {
            if g.cnt >= 111 {
                *k.prog += 1;
            } else {
                g.cnt += 1;
            }
        }
        4 => {
            let r = *k.speed_rate;
            if ee::eq(r, ONE) {
                *k.prog += 1;
                g.cnt = 0;
            } else {
                let r = ee::add(r, ee::div(ee::sub(ONE, r), RATE_DIV));
                *k.speed_rate = if ee::lt(ee::sub(ONE, r), RATE_SNAP) { ONE } else { r };
            }
        }
        5 => {
            let v = g.k224;
            if ee::eq(v, K120) {
                *k.prog += 1;
                g.cnt = 0;
            } else {
                let v = ee::add(v, ee::mul(K125, ee::sub(K120, v)));
                g.k224 = if ee::le(v, K120) { v } else { K120 };
            }
        }
        6 => {
            *k.prog += 1;
            g.cnt = 0;
        }
        7 => {
            if *k.act == 23 && !ee::lt(g.arms_t, ONE) {
                *k.prog += 1;
                g.cnt = 0;
            } else {
                g.cnt += 1;
                let t = ee::add(g.arms_t, ARMS_STEP);
                g.arms_t = if ee::lt(t, ONE) { t } else { ONE };
            }
        }
        8 => {
            camera.change_camera(id::FIELD);
            let n = camera.cam_id;
            camera.cam_mut(n).pos = *k.pos_cam;
            camera.cam_mut(n).view = *k.pos_view;
            param_change(camera, *k.pos_cam, *k.pos_view);
            *k.act = 2;
            *k.flags &= !RESTRAINT;
            *k.camera_flag = false;
            *k.prog = 0;
            g.flag = false;
            cam.stop();
        }
        _ => {}
    }
}
