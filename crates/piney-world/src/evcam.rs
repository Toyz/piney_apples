//! The event camera: the camera instructions of the event scripts
//! (`ccEvent::Execute`, INF main 0x001a8d20, codes 22-36 and 40-50) and the
//! task that runs it, `ccThCameraExecute` (0x001b4ff0, priority 33). The
//! state is `eventMng.cam` (`ccEvCamCtrl`, 0x310 bytes): a look-at point
//! driven by `vpCtrl`, a camera point by `cpCtrl`, and the two-point `zcam`
//! of the `camz_*` instructions; `ccEvent::CamCtrl` (0x001b3680) writes
//! `ecam` each frame. EE single precision ([`crate::ee`]), checked by
//! `tools/test_evcam_rs.py` (docs/engine/field-game.md).

use piney_event::host::CameraCommand;

use crate::camera::{self, CamPad, Camera, DIST_DEFAULT, DIST_MAX, DIST_MIN, PITCH_DEFAULT};
use crate::ee::{
    self, F, ONE, PI, V4, add, apply, cosf, deg2rad, div, eq, from_int, le, lt, mul, neg, rad2deg, sinf, sqrt, sub,
    to_int, vadd, vsub,
};

/// `TWO_PI` (0x40c90fdb) and `HALF_PI` (0x3fc90fdb) as the code loads them.
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const MINUS_PI: F = 0xc049_0fdb;
/// 10.0: the scripts' positions and distances are in tenths.
const TEN: F = 0x4120_0000;
const TWO: F = 0x4000_0000;
const THREE: F = 0x4040_0000;
const HALF: F = 0x3f00_0000;

/// `ccEvCamz` (0x150 bytes): one point of the two-point camera.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Camz {
    /// +0x00 8 (`camz_set`, `camz_move`) or 9 (`camz_path`).
    pub ctrl_type: i16,
    /// +0x02 the curve (`camz_speed`): 3 linear, 2 `sinf` ease-out, 0 and
    /// anything else a 1/rate approach with the rate counting down, 1 the
    /// approach at a fixed rate; along a path 4 a Bezier through four
    /// points, anything else a Hermite curve.
    pub spd_type: i16,
    /// +0x04 the path's point count, +0x06 unused.
    pub inp_num: i16,
    pub pad: i16,
    /// +0x08 the Hermite curve's tension (`alpha / 10`).
    pub inp_alpha: F,
    /// +0x0c frames to arrive in (0: at once), +0x10 frames so far.
    pub rate: F,
    pub cnt: F,
    /// +0x20 where it is, +0x30 where it started, +0x40 where it goes.
    pub now: V4,
    pub start: V4,
    pub target: V4,
    /// +0x50 the path's points (`camz_point`).
    pub inp_pos: [V4; 16],
}

/// `ccEvCamCtrl` (0x310 bytes, `ccEvent`+0x440).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvCamCtrl {
    /// +0x00, +0x02: how the look-at point and the camera move (the
    /// module's table).
    pub vp_ctrl: i16,
    pub cp_ctrl: i16,
    /// +0x04 `charType`, +0x08 `charID`: the character vpCtrl 1 follows;
    /// +0x0c `height` above its feet.
    pub char_type: i32,
    pub char_id: i32,
    pub height: F,
    /// +0x10 frames left to reach `vp_target` (0: at once).
    pub vp_rate: F,
    /// +0x20 the look-at point, +0x30 where it goes.
    pub vp: V4,
    pub vp_target: V4,
    /// +0x40 frames left to reach `rot_target` and `dist_target` (for
    /// cpCtrl 3, the turn a frame in radians).
    pub cp_rate: F,
    /// +0x44 the distance from the look-at point, +0x48 where it goes.
    pub dist: F,
    pub dist_target: F,
    /// +0x4c the pitch and heading (radians), +0x54 where they go (16-bit).
    pub rot: [F; 2],
    pub rot_target: [i16; 2],
    /// +0x60 the camera point.
    pub cp: V4,
    /// +0x70 the two-point camera: [0] the look-at point, [1] the camera.
    pub zcam: [Camz; 2],
}

/// Where the event camera's task is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Task {
    /// `eventMng.camTscb` null: no task.
    #[default]
    None,
    /// Started this frame: its first frame switches to the event camera.
    Started,
    Running,
}

/// What the camera instructions and the task look up in the rest of the
/// game.
pub trait Scene {
    /// The position (+0x40, all four lanes) of the character an event
    /// names by `type` and `code`: `ccEvent::GetSpc(code)` for type 2,
    /// `GetNpc` for 3 and 4, `GetEnemy` for 5 and 6, otherwise
    /// `ccCheckTargetTypeId(1 << type, code)` on the command lists; None
    /// when there is none.
    fn char_pos(&self, ty: i32, code: i32) -> Option<V4>;
    /// `ccPartyManager.memberChar[0]`'s position: what a missing character
    /// falls back to.
    fn leader_pos(&self) -> V4;
    /// A marker's position (all four lanes): in a town
    /// `markerEvTbl[marker]`'s dummy (+0x10), in a field or dungeon the
    /// event position of that number; None for none (the instruction then
    /// leaves the look-at target alone).
    fn marker_pos(&self, marker: i16) -> Option<V4>;
    /// The player's `dirc` (+0x60): `camera_end_reset`'s soft reset and the
    /// tutorial's reset turn behind its z.
    fn player_dirc(&self) -> V4;
}

/// What a frame of the task reads besides the scene: the camera tutorial's
/// input (cpCtrl 5-7).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    /// `ccSys.pad[0]`: `direct`, the right stick, the pressures.
    pub pad: CamPad,
    /// `saveData.assignPAD` +0x8412 and +0x8414: the zoom buttons of a
    /// type B scheme (a new game's R1 0x8 and R2 0x2).
    pub zoom: [u16; 2],
}

/// The event camera: `eventMng.cam`, the task, and the camera part of
/// `menu_ban`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventCam {
    pub ctrl: EvCamCtrl,
    /// `eventMng.camTscb` (+0x7b8).
    pub task: Task,
    /// `eventMng.normalCamID` (+0x7b4): the camera `menu_ban` found
    /// active, which `camera_end_reset` returns to. (`eventMng.puppetShow`
    /// is kept by the camera, [`Camera::puppet_show`].)
    pub normal_cam_id: i32,
}

/// The fixed 16-lane scratch `CamCtrl` works in (`ccSys+0x25c`, 0x70
/// bytes): what it does not write there is what an earlier function left,
/// taken here as zero.
#[derive(Clone, Copy, Default)]
struct Scratch {
    /// +0x00 the camera point, +0x10 the look-at point, +0x20 the
    /// rotation work vector.
    pos: V4,
    view: V4,
    rot: V4,
}

impl EventCam {
    pub fn new() -> Self {
        EventCam::default()
    }

    /// Whether the task is running (started and not stopped).
    pub fn running(&self) -> bool {
        self.task != Task::None
    }

    /// What the next area starts with: `eventMng` lives on, and
    /// `ccEvent::Init` (0x001a6ca0, from `ccStartThEvent` in the area's
    /// set-up) clears `camTscb` (the set-up's `ccDeleteAllThread` ended the
    /// task) and `puppetShow` (a new [`Camera`] starts without it) but not
    /// `cam` or `normalCamID`: a `camz_move` before any `camz_set` starts
    /// from where the last area's event camera was.
    pub fn next_area(&self) -> EventCam {
        EventCam { ctrl: self.ctrl.clone(), task: Task::None, normal_cam_id: self.normal_cam_id }
    }

    /// Every handler but `camz_speed` and `camz_point` ends so:
    /// `ccStartThread(ccThCameraExecute, 33, 0x800)` unless `camTscb` is
    /// set.
    fn start(&mut self) {
        if self.task == Task::None {
            self.task = Task::Started;
        }
    }

    /// `camera_end` / `camera_end_reset`'s `ccDeleteThread(camTscb)`:
    /// the task does not run again.
    fn stop(&mut self) {
        self.task = Task::None;
    }

    /// One camera instruction at play level (`ccEvent::Execute`'s
    /// handlers, level 2; below that they only step over their operands).
    pub fn command(&mut self, c: CameraCommand, cam: &mut Camera, scene: &dyn Scene) {
        let e = &mut self.ctrl;
        let ten = |v: i16| mul(TEN, from_int(i32::from(v)));
        let find = |ty: i16, code: i16| -> V4 {
            scene.char_pos(i32::from(ty), i32::from(code)).unwrap_or_else(|| scene.leader_pos())
        };
        match c {
            // cam_look_pos (22) / cam_look_pos_half (26).
            CameraCommand::LookPos { x, y, z, half } => {
                e.vp_ctrl = 0;
                e.vp_rate = 0;
                let t = [ten(x), ten(y), ten(z), e.vp_target[3]];
                if half {
                    e.vp_target = halfway(e.vp_target, t);
                } else {
                    e.vp_target[..3].copy_from_slice(&t[..3]);
                }
                self.start();
            }
            // cam_look_char (23) / cam_look_char_half (27).
            CameraCommand::LookChar { ty, code, height, half } => {
                e.vp_ctrl = 0;
                e.vp_rate = 0;
                let mut t = find(ty, code);
                t[2] = add(t[2], ten(height));
                if half {
                    e.vp_target = halfway(e.vp_target, t);
                } else {
                    e.vp_target = t;
                }
                self.start();
            }
            // cam_follow_char (24).
            CameraCommand::FollowChar { ty, code, height } => {
                e.vp_ctrl = 1;
                e.vp_rate = 0;
                e.char_type = i32::from(ty);
                e.char_id = i32::from(code);
                e.height = ten(height);
                e.vp_target = find(ty, code);
                e.vp_target[2] = add(e.vp_target[2], ten(height));
                self.start();
            }
            // cam_look_marker (25) / cam_look_marker_half (28).
            CameraCommand::LookMarker { marker, half } => {
                e.vp_ctrl = 0;
                e.vp_rate = 0;
                if let Some(t) = scene.marker_pos(marker) {
                    e.vp_target = if half { halfway(e.vp_target, t) } else { t };
                } else if half {
                    e.vp_target[3] = ONE;
                }
                self.start();
            }
            // cam_pan_pos (29).
            CameraCommand::PanPos { x, y, z, rate } => {
                e.vp_ctrl = 0;
                e.vp_rate = from_int(i32::from(rate));
                e.vp_target[0] = ten(x);
                e.vp_target[1] = ten(y);
                e.vp_target[2] = ten(z);
                self.start();
            }
            // cam_pan_char (30): the character's place now.
            CameraCommand::PanChar { ty, code, height, rate, follow: false } => {
                e.vp_ctrl = 0;
                e.vp_rate = from_int(i32::from(rate));
                e.vp_target = find(ty, code);
                e.vp_target[2] = add(e.vp_target[2], ten(height));
                self.start();
            }
            // cam_pan_follow (31): the character from the next frame on.
            CameraCommand::PanChar { ty, code, height, rate, follow: true } => {
                e.vp_ctrl = 1;
                e.vp_rate = from_int(i32::from(rate));
                e.char_type = i32::from(ty);
                e.char_id = i32::from(code);
                e.height = ten(height);
                self.start();
            }
            // cam_pan_marker (32).
            CameraCommand::PanMarker { marker, rate } => {
                e.vp_ctrl = 0;
                e.vp_rate = from_int(i32::from(rate));
                if let Some(t) = scene.marker_pos(marker) {
                    e.vp_target = t;
                }
                self.start();
            }
            // cam_orbit (33).
            CameraCommand::Orbit { rotx, roty, dist } => {
                e.cp_ctrl = 2;
                e.cp_rate = 0;
                e.rot_target = [rotx, roty];
                e.dist_target = ten(dist);
                self.start();
            }
            // cam_orbit_move (34) / cam_orbit_turn (35).
            CameraCommand::OrbitMove { rotx, roty, dist, rate, degrees } => {
                if degrees {
                    e.cp_ctrl = 3;
                    e.cp_rate = deg2rad(rate);
                } else {
                    e.cp_ctrl = 2;
                    e.cp_rate = from_int(i32::from(rate));
                }
                e.rot_target = [rotx, roty];
                e.dist_target = ten(dist);
                self.start();
            }
            // cam_mode4 (36).
            CameraCommand::Mode4 => {
                e.cp_ctrl = 4;
                self.start();
            }
            // camz_set (40): both points at once, the curve reset to 0.
            CameraCommand::ZSet { v, c } => {
                let tv = [ten(v[0]), ten(v[1]), ten(v[2]), ONE];
                let z = &mut e.zcam[0];
                z.ctrl_type = 8;
                z.spd_type = 0;
                z.rate = 0;
                z.cnt = 0;
                z.target = tv;
                e.vp_ctrl = 8;
                e.vp_rate = 0;
                e.vp_target = tv;
                let z = &mut e.zcam[1];
                z.ctrl_type = 8;
                z.spd_type = 0;
                z.rate = 0;
                z.cnt = 0;
                z.target = [ten(c[0]), ten(c[1]), ten(c[2]), ONE];
                e.cp_ctrl = 8;
                e.cp_rate = 0;
                self.start();
            }
            // camz_move (41): from where vp and cp are; the curve kept. The
            // camera point's rate goes into vpRate a second time (the
            // game's store to +0x450, not cpRate's +0x480).
            CameraCommand::ZMove { v, c, vprate, cprate } => {
                let tv = [ten(v[0]), ten(v[1]), ten(v[2]), ONE];
                let (vp, cp) = (e.vp, e.cp);
                let z = &mut e.zcam[0];
                z.ctrl_type = 8;
                z.rate = from_int(i32::from(vprate));
                z.cnt = 0;
                z.target = tv;
                z.start = vp;
                z.now = vp;
                e.vp_ctrl = 8;
                e.vp_rate = from_int(i32::from(vprate));
                e.vp_target = tv;
                let z = &mut e.zcam[1];
                z.ctrl_type = 8;
                z.rate = from_int(i32::from(cprate));
                z.cnt = 0;
                z.target = [ten(c[0]), ten(c[1]), ten(c[2]), ONE];
                z.start = cp;
                z.now = cp;
                e.cp_ctrl = 8;
                e.vp_rate = from_int(i32::from(cprate));
                self.start();
            }
            // camz_speed (42): no task.
            CameraCommand::ZSpeed { vstype, cstype } => {
                e.zcam[0].spd_type = vstype;
                e.zcam[1].spd_type = cstype;
            }
            // camz_point (43): no task. The script's num is clamped to 16
            // in place; point 16 lies past the array: zcam[0]'s lands on
            // zcam[1]'s head, zcam[1]'s on `ccEvent.currentOpen` (the
            // interpreter's, not stored here).
            CameraCommand::ZPoint { num, v, c } => {
                let n = num.min(16);
                let pv = [ten(v[0]), ten(v[1]), ten(v[2]), ONE];
                if (0..16).contains(&n) {
                    e.zcam[0].inp_pos[n as usize] = pv;
                    e.zcam[1].inp_pos[n as usize] = [ten(c[0]), ten(c[1]), ten(c[2]), ONE];
                } else if n == 16 {
                    e.zcam[1].set_head(pv);
                }
            }
            // camz_path (44): along the points; the camera point's rate
            // again goes into vpRate.
            CameraCommand::ZPath { num, vprate, cprate, alpha } => {
                let n = num.min(16);
                let a = div(from_int(i32::from(alpha)), TEN);
                for (z, rate) in e.zcam.iter_mut().zip([vprate, cprate]) {
                    z.ctrl_type = 9;
                    z.inp_num = n;
                    z.inp_alpha = a;
                    z.rate = from_int(i32::from(rate));
                    z.cnt = 0;
                }
                e.vp_ctrl = 9;
                e.cp_ctrl = 9;
                e.vp_rate = from_int(i32::from(cprate));
                self.start();
            }
            // camera (48): look at the character from angles and a distance,
            // at once.
            CameraCommand::Char { ty, code, height, rotx, roty, dist } => {
                e.vp_ctrl = 1;
                e.vp_rate = 0;
                e.char_type = i32::from(ty);
                e.char_id = i32::from(code);
                e.height = ten(height);
                e.vp_target = find(ty, code);
                e.vp_target[2] = add(e.vp_target[2], ten(height));
                e.cp_ctrl = 2;
                e.cp_rate = 0;
                e.rot_target = [rotx, roty];
                e.dist_target = ten(dist);
                self.start();
            }
            // camera_end (49): back to the field camera.
            CameraCommand::End => {
                cam.change_camera(camera::id::FIELD);
                cam.resetting = false;
                self.stop();
            }
            // camera_end_reset (50): back to the camera menu_ban found,
            // tcam turned behind the player at once.
            CameraCommand::EndReset => {
                cam.change_camera(self.normal_cam_id as i16);
                cam.resetting = false;
                cam.soft_reset(scene.player_dirc()[2]);
                self.stop();
            }
        }
    }

    /// `teach_camera1..3` (37-39)'s camera part: cpCtrl 5, 6 or 7 (the pad
    /// turns, zooms or resets the event camera) and the task started.
    pub fn teach(&mut self, part: u8) {
        self.ctrl.cp_ctrl = 4 + i16::from(part);
        self.start();
    }

    /// `menu_ban`'s camera part (`MenuBan` 0x001b2460: `puppetShow = 1`,
    /// `normalCamID = checkCameraID()`) and `menu_clear`'s (`MenuClr`
    /// 0x001b25c0: `puppetShow = 0`; the camera is left as it is).
    pub fn menu_ban(&mut self, on: bool, cam: &mut Camera) {
        cam.puppet_show = on;
        if on {
            self.normal_cam_id = i32::from(cam.cam_id);
        }
    }

    /// One frame of `ccThCameraExecute` (0x001b4ff0) if it is running: on
    /// its first `changeCamera(3)`, then `ccEvent::CamCtrl`.
    pub fn frame(&mut self, cam: &mut Camera, scene: &dyn Scene, input: &Input) {
        match self.task {
            Task::None => return,
            Task::Started => {
                cam.change_camera(camera::id::EVENT);
                self.task = Task::Running;
            }
            Task::Running => {}
        }
        self.cam_ctrl(cam, scene, input);
    }

    /// `ccEvent::CamCtrl` (0x001b3680).
    pub fn cam_ctrl(&mut self, cam: &mut Camera, scene: &dyn Scene, input: &Input) {
        let n = cam.cam_id;
        let e = &mut self.ctrl;
        if e.vp_ctrl == 8 || e.vp_ctrl == 9 {
            let path = e.vp_ctrl == 9;
            return camz_ctrl(e, cam, path);
        }
        let mut s = Scratch::default();
        // The look-at point.
        if e.vp_ctrl == 1 {
            e.vp_target = scene.char_pos(e.char_type, e.char_id).unwrap_or_else(|| scene.leader_pos());
            e.vp_target[2] = add(e.vp_target[2], e.height);
        }
        if !eq(e.vp_rate, 0) {
            let mut d = vsub(e.vp_target, e.vp);
            for x in &mut d[..3] {
                *x = div(*x, e.vp_rate);
            }
            e.vp = vadd(e.vp, d);
            e.vp[3] = ONE;
            e.vp_rate = sub(e.vp_rate, ONE);
            if lt(e.vp_rate, 0) {
                e.vp_rate = 0;
            }
        } else {
            e.vp = e.vp_target;
        }
        s.view = e.vp;
        cam.cam_mut(n).view = s.view;
        // The camera's angles and distance (the jump table at 0x00356270).
        let scheme = cam.scheme;
        let pad = &input.pad;
        match e.cp_ctrl {
            2 if !eq(e.cp_rate, 0) => {
                let mut w = [
                    wrap_pi(sub(deg2rad(e.rot_target[0]), e.rot[0])),
                    wrap_pi(sub(deg2rad(e.rot_target[1]), e.rot[1])),
                    sub(e.dist_target, e.dist),
                    s.rot[3],
                ];
                for x in &mut w[..3] {
                    *x = div(*x, e.cp_rate);
                }
                // Each step is added, then wrapped against pi/2 in the
                // scratch (only y survives there, into the camera's rot.y).
                e.rot[0] = add(e.rot[0], w[0]);
                w[0] = wrap_half_pi(w[0]);
                e.rot[1] = add(e.rot[1], w[1]);
                w[1] = wrap_half_pi(w[1]);
                e.dist = add(e.dist, w[2]);
                s.rot = w;
                e.cp_rate = sub(e.cp_rate, ONE);
                if lt(e.cp_rate, 0) {
                    e.cp_rate = 0;
                }
            }
            2 => {
                e.rot = [deg2rad(e.rot_target[0]), deg2rad(e.rot_target[1])];
                e.dist = e.dist_target;
            }
            3 => {
                e.rot_target[1] = e.rot_target[1].wrapping_add(rad2deg(e.cp_rate));
                e.rot = [deg2rad(e.rot_target[0]), deg2rad(e.rot_target[1])];
                e.dist = e.dist_target;
            }
            // teach_camera1: turn.
            5 => {
                if scheme.ctrl_type != 0 {
                    turn_buttons(e, pad, scheme.rev_lr);
                } else {
                    turn_stick(e, pad, scheme.rev_lr, scheme.rev_ud);
                }
            }
            // teach_camera2: zoom (and turn).
            6 => {
                if scheme.ctrl_type != 0 {
                    turn_buttons(e, pad, scheme.rev_lr);
                    let mut f = mul(from_int(i32::from(pad.pow_r)), cosf(pad.dirc_r));
                    if ee::f(f).abs() > 64.0 {
                        f = if le(f, 0) { add(f, k(32.0)) } else { sub(f, k(32.0)) };
                        f = mul(f, 0x3ecc_cccd);
                        if scheme.rev_ud {
                            f = neg(f);
                        }
                        e.dist = add(e.dist, f);
                    }
                } else {
                    turn_stick(e, pad, scheme.rev_lr, scheme.rev_ud);
                    let press = |v: u8| {
                        let g = k(30.0);
                        if v >= 129 {
                            add(g, mul(0x3f40_0000, from_int(i32::from(v) - 128)))
                        } else if v == 0 {
                            add(g, 0x42be_8000)
                        } else {
                            g
                        }
                    };
                    // lh: the masks sign-extended.
                    let mask = |b: u16| b as i16 as i32 as u32;
                    if pad.direct & mask(input.zoom[0]) != 0 {
                        e.dist = sub(e.dist, press(pad.pow[9]));
                    }
                    if pad.direct & mask(input.zoom[1]) != 0 {
                        e.dist = add(e.dist, press(pad.pow[11]));
                    }
                }
                if lt(e.dist, DIST_MIN) {
                    e.dist = DIST_MIN;
                }
                if !le(e.dist, DIST_MAX) {
                    e.dist = DIST_MAX;
                }
            }
            // teach_camera3: back behind the player, a quarter of the way a
            // frame (as cameraMain's reset, on the event camera).
            7 => {
                let dirc = scene.player_dirc();
                s.rot = dirc;
                let m = rad2deg(dirc[2]);
                let mut pitch = rad2deg(e.rot[0]);
                let mut yaw = rad2deg(e.rot[1]);
                let mut d = m.wrapping_sub(yaw);
                if i32::from(d).abs() >= 20481 {
                    d = if d > 0 { 20480 } else { -20480 };
                }
                if i32::from(d).abs() < 128 {
                    d = if d > 0 { 128 } else { -128 };
                }
                yaw = yaw.wrapping_add(d >> 2);
                if (i32::from(m) - i32::from(yaw)).abs() < 33 {
                    yaw = m;
                }
                pitch = pitch.wrapping_add(((i32::from(PITCH_DEFAULT) - i32::from(pitch)) >> 2) as i16);
                if (i32::from(PITCH_DEFAULT) - i32::from(pitch)).abs() < 128 {
                    pitch = PITCH_DEFAULT;
                }
                e.rot = [deg2rad(pitch), deg2rad(yaw)];
                // |(double)(970 - dist)| < 4: exact on the float difference.
                if ee::f(sub(DIST_DEFAULT, e.dist)).abs() < 4.0 {
                    e.dist = DIST_DEFAULT;
                }
                e.dist = add(e.dist, mul(0x3e80_0000, sub(DIST_DEFAULT, e.dist)));
            }
            _ => {}
        }
        // The camera point: (0, dist, 0) turned about x by the pitch, then
        // about z by the heading, from the look-at point.
        if e.cp_ctrl != 4 {
            let m = rot_z(&rot_x(&UNIT, e.rot[0]), e.rot[1]);
            s.pos = vadd(s.view, apply(&m, [0, e.dist, 0, ONE]));
            cam.cam_mut(n).pos = s.pos;
        }
        e.cp = s.pos;
        let r = cam.get_rot(n, s.rot);
        cam.cam_mut(n).rot = r;
    }
}

/// A constant's bits.
const fn k(x: f32) -> F {
    x.to_bits()
}

/// The `_half` instructions: `t - target`, x y z halved, added back, w 1.
fn halfway(target: V4, t: V4) -> V4 {
    let mut d = vsub(t, target);
    for x in &mut d[..3] {
        *x = div(*x, TWO);
    }
    let mut r = vadd(target, d);
    r[3] = ONE;
    r
}

/// An angle step brought into [-pi, pi]: above pi, `-(2 pi - a)`; below
/// -pi, `2 pi + a`.
fn wrap_pi(a: F) -> F {
    if !le(a, PI) {
        neg(sub(TWO_PI, a))
    } else if lt(a, MINUS_PI) {
        add(TWO_PI, a)
    } else {
        a
    }
}

/// `CamCtrl`'s second wrap of the step (0x001b3b04): above pi/2,
/// `-(pi - a)`; below pi/2, `pi + a`.
fn wrap_half_pi(a: F) -> F {
    if !le(a, HALF_PI) {
        neg(sub(PI, a))
    } else if lt(a, HALF_PI) {
        add(PI, a)
    } else {
        a
    }
}

/// The tutorial's turn for a type A scheme (cpCtrl 5 and 6): R1 and L1
/// held turn the heading by 500 (or 384 + 2 (p - 10) with pressure), both
/// adding up, reversed with `camRevLR`; the pitch 1512.
fn turn_buttons(e: &mut EvCamCtrl, pad: &CamPad, rev_lr: bool) {
    let mut a: i16 = 0;
    if pad.direct & 0x8 != 0 {
        let v = pad.pow[9];
        if v >= 11 {
            a = a.wrapping_add(((i32::from(v) - 10) * 2 + 384) as i16);
        } else if v == 0 {
            a = a.wrapping_add(500);
        }
    }
    if pad.direct & 0x4 != 0 {
        let v = pad.pow[8];
        if v >= 11 {
            a = a.wrapping_add((-384 - (i32::from(v) - 10) * 2) as i16);
        } else if v == 0 {
            a = a.wrapping_sub(500);
        }
    }
    if rev_lr {
        a = a.wrapping_neg();
    }
    e.rot[1] = deg2rad(rad2deg(e.rot[1]).wrapping_add(a));
    e.rot[0] = deg2rad(PITCH_DEFAULT);
}

/// The tutorial's turn for a type B scheme: the right stick turns the
/// heading (`2 sinf(dircR) powR` a frame) and the pitch (`2 cosf`), the
/// pitch kept within 896-8191.
fn turn_stick(e: &mut EvCamCtrl, pad: &CamPad, rev_lr: bool, rev_ud: bool) {
    let stick = |trig: F| to_int(mul(from_int(i32::from(pad.pow_r)), mul(TWO, trig))) as i16;
    let mut a = stick(sinf(pad.dirc_r));
    if rev_lr {
        a = a.wrapping_neg();
    }
    e.rot[1] = deg2rad(rad2deg(e.rot[1]).wrapping_sub(a));
    let mut b = stick(cosf(pad.dirc_r));
    if rev_ud {
        b = b.wrapping_neg();
    }
    let mut p = rad2deg(e.rot[0]).wrapping_sub(b);
    if (8192..32767).contains(&p) {
        p = 8191;
    }
    if p < 896 {
        p = 896;
    }
    e.rot[0] = deg2rad(p);
}

/// `ccEvent::CamzCtrl` (0x001b4780) and `CamzInpCtrl` (0x001b4b20): each
/// point moved (rate 0: at its target), zcam[0] the look-at point and
/// zcam[1] the camera; the rotation from the two.
fn camz_ctrl(e: &mut EvCamCtrl, cam: &mut Camera, path: bool) {
    let n = cam.cam_id;
    let step = |zc: &mut [Camz; 2], w: usize| {
        if eq(zc[w].rate, 0) {
            zc[w].now = zc[w].target;
        } else if path {
            camz_inp_main(zc, w);
        } else {
            camz_mov_main(&mut zc[w]);
        }
    };
    step(&mut e.zcam, 0);
    cam.cam_mut(n).view = e.zcam[0].now;
    e.vp = e.zcam[0].now;
    step(&mut e.zcam, 1);
    let pos = e.zcam[1].now;
    cam.cam_mut(n).pos = pos;
    e.cp = pos;
    let r = cam.get_rot(n, pos);
    cam.cam_mut(n).rot = r;
}

/// `ccEvent::CamzMovMain` (0x001b48e0): one frame along the curve.
pub fn camz_mov_main(z: &mut Camz) {
    if lt(z.cnt, z.rate) {
        z.cnt = add(z.cnt, ONE);
    }
    let rate = z.rate;
    let lper = div(z.cnt, rate);
    let along = |z: &mut Camz, t: F| {
        let mut d = vsub(z.target, z.start);
        for x in &mut d[..3] {
            *x = mul(*x, t);
        }
        z.now = vadd(z.start, d);
        z.now[3] = ONE;
    };
    match z.spd_type {
        3 => along(z, lper),
        2 => along(z, sinf(mul(PI, mul(HALF, lper)))),
        t => {
            let mut d = vsub(z.target, z.now);
            for x in &mut d[..3] {
                *x = div(*x, rate);
            }
            z.now = vadd(z.now, d);
            z.now[3] = ONE;
            if t != 1 {
                z.rate = sub(z.rate, ONE);
                if lt(z.rate, 0) {
                    z.rate = 0;
                }
            }
        }
    }
}

impl Camz {
    /// The struct's first 16 bytes as floats' bits: `ctrlType | spdType
    /// << 16`, `inpNum | pad << 16`, `inpAlpha`, `rate`.
    fn head(&self) -> V4 {
        let pair = |a: i16, b: i16| u32::from(a as u16) | u32::from(b as u16) << 16;
        [pair(self.ctrl_type, self.spd_type), pair(self.inp_num, self.pad), self.inp_alpha, self.rate]
    }

    fn set_head(&mut self, v: V4) {
        self.ctrl_type = v[0] as u16 as i16;
        self.spd_type = (v[0] >> 16) as u16 as i16;
        self.inp_num = v[1] as u16 as i16;
        self.pad = (v[1] >> 16) as u16 as i16;
        self.inp_alpha = v[2];
        self.rate = v[3];
    }

    /// The 16 bytes at `inpPos[i]` as the path reads them: before the
    /// points the struct's own vectors (a Bezier path of fewer than four
    /// points reads back to -4: `target`, `start`, `now`, `cnt` and the
    /// padding after it, taken as zero), past them `after`.
    fn inp(&self, i: i32, after: V4) -> V4 {
        match i {
            0..=15 => self.inp_pos[i as usize],
            -1 => self.target,
            -2 => self.start,
            -3 => self.now,
            -4 => [self.cnt, 0, 0, 0],
            -5 => self.head(),
            _ => after,
        }
    }
}

/// `ccEvent::CamzInpMain` (0x001b4c80) on `zc[w]`: one frame along the
/// path of `inp_num` points: `t = (num - 1) cnt / rate` split into the
/// segment and the fraction; spdType 4 a Bezier (`eneSplineB`) over four
/// points from the segment before (the fraction / 3), anything else a
/// Hermite curve (`eneSplineH`) with tension `inpAlpha`, the ends' missing
/// neighbours made up as the game does. A path of 16 read at its end reads
/// past the points: zcam[0] into zcam[1]'s head; zcam[1] into
/// `ccEvent.currentOpen`, taken as zero.
pub fn camz_inp_main(zc: &mut [Camz; 2], w: usize) {
    let after = if w == 0 { zc[1].head() } else { [0; 4] };
    let z = &mut zc[w];
    if lt(z.cnt, z.rate) {
        z.cnt = add(z.cnt, ONE);
    }
    let lper = div(z.cnt, z.rate);
    let num = i32::from(z.inp_num);
    let mut f = mul(from_int(num - 1), lper);
    // modff then fptosi: the whole part.
    let mut s0 = to_int(f);
    let inp = |z: &Camz, i: i32| z.inp(i, after);
    if z.spd_type == 4 {
        s0 = (s0 - 1).max(0);
        if num - 4 < s0 {
            s0 = num - 4;
        }
        f = div(sub(f, from_int(s0)), THREE);
        z.now = spline_b(inp(z, s0), inp(z, s0 + 1), inp(z, s0 + 2), inp(z, s0 + 3), f);
        return;
    }
    let v1 = s0 - 1;
    let p = if v1 < 0 {
        let (a, b) = (inp(z, 1), inp(z, 2));
        [vadd(vsub(a, b), a), inp(z, 0), a, b]
    } else {
        f = sub(f, add(ONE, from_int(v1)));
        let (a, b) = (inp(z, s0), inp(z, s0 + 1));
        let last = if num - 4 < v1 { vadd(vsub(a, b), b) } else { inp(z, s0 + 2) };
        [inp(z, v1), a, b, last]
    };
    z.now = spline_h(p[0], p[1], p[2], p[3], f, z.inp_alpha);
}

/// `eneSplineB(out, p0, p1, p2, p3, t)` (gcmn 0x004381d0): the cubic
/// Bernstein blend `(1-t)^3 p0 + 3t(1-t)^2 p1 + 3t^2(1-t) p2 + t^3 p3` in
/// the FPU's multiply-adds; w 1.
fn spline_b(p0: V4, p1: V4, p2: V4, p3: V4, t: F) -> V4 {
    let u = sub(ONE, t);
    let b0 = mul(u, mul(u, u));
    let u3 = mul(THREE, u);
    let b1 = mul(t, mul(u3, u));
    let b2 = mul(t, mul(u3, t));
    let b3 = mul(t, mul(t, t));
    let mut out = [0, 0, 0, ONE];
    for i in 0..3 {
        let acc = add(mul(p0[i], b0), mul(p1[i], b1));
        let acc = add(mul(p2[i], b2), acc);
        out[i] = add(acc, mul(p3[i], b3));
    }
    out
}

/// `eneSplineH(out, p0, p1, p2, p3, t, alpha)` (gcmn 0x004382a0): the
/// Hermite curve from p1 to p2 with tangents `(1 - alpha)/2` of
/// `((p1 - p0) + p2) - p1` and `((p2 - p1) + p3) - p2`; w 1.
fn spline_h(p0: V4, p1: V4, p2: V4, p3: V4, t: F, alpha: F) -> V4 {
    let t2 = mul(t, t);
    let t3 = mul(t2, t);
    let tension = div(sub(ONE, alpha), TWO);
    let tangent = |a: V4, b: V4, c: V4| {
        let mut m = vsub(vadd(vsub(b, a), c), b);
        for x in &mut m[..3] {
            *x = mul(*x, tension);
        }
        m
    };
    let m0 = tangent(p0, p1, p2);
    let m1 = tangent(p1, p2, p3);
    let three_t2 = mul(THREE, t2);
    let h00 = add(ONE, sub(mul(TWO, t3), three_t2));
    let h10 = add(t, sub(t3, mul(TWO, t2)));
    let h11 = sub(t3, t2);
    let h01 = add(mul(0xc000_0000, t3), three_t2);
    let scale = |v: V4, h: F| [mul(v[0], h), mul(v[1], h), mul(v[2], h), v[3]];
    let mut out = vadd(scale(p1, h00), scale(m0, h10));
    out = vadd(out, scale(m1, h11));
    out = vadd(out, scale(p2, h01));
    out[3] = ONE;
    out
}

/// A matrix as stored columns of float bits.
pub(crate) type Mat = [V4; 4];

pub(crate) const UNIT: Mat = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// `_sceVu0ecossin` (0x001109b8) as `sceVu0RotMatrixX/Z` call it: cos an
/// odd polynomial (S5432, 0x002f7520) in `pi/2 - |angle|`, sin
/// `+-sqrt(1 - cos^2)`, negative for an angle below zero. (sin, cos).
fn vu_cossin(angle: F) -> (F, F) {
    const S: [F; 4] = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];
    let negative = lt(angle, 0);
    let x = if negative { add(HALF_PI, angle) } else { sub(HALF_PI, angle) };
    let x2 = mul(x, x);
    let mut p = S.map(|c| mul(mul(c, x), x2));
    for v in &mut p[..3] {
        *v = mul(*v, x2);
    }
    let mut r = add(x, p[3]);
    for v in &mut p[..2] {
        *v = mul(*v, x2);
    }
    r = add(r, p[2]);
    p[0] = mul(p[0], x2);
    r = add(add(r, p[1]), p[0]);
    let q = sqrt(sub(ONE, mul(r, r)));
    (if negative { sub(0, q) } else { add(0, q) }, r)
}

/// `rot`'s columns times each column of `m` (VU0 multiply-adds, each
/// product rounded).
fn vu_mul(rot: &Mat, m: &Mat) -> Mat {
    m.map(|col| {
        std::array::from_fn(|i| {
            let acc = mul(rot[0][i], col[0]);
            let acc = add(acc, mul(rot[1][i], col[1]));
            let acc = add(acc, mul(rot[2][i], col[2]));
            add(acc, mul(rot[3][i], col[3]))
        })
    })
}

/// `sceVu0RotMatrixX(m, m, a)` (0x00110ad8): Rx(a) m.
pub(crate) fn rot_x(m: &Mat, a: F) -> Mat {
    let (s, c) = vu_cossin(a);
    let (c, sn, ns) = (add(0, c), add(0, s), sub(0, s));
    vu_mul(&[[ONE, 0, 0, 0], [0, c, sn, 0], [0, ns, c, 0], [0, 0, 0, ONE]], m)
}

/// `sceVu0RotMatrixY(m, m, a)` (0x00110b80): Ry(a) m, its columns
/// (c, 0, -s, 0), (0, 1, 0, 0), (s, 0, c, 0).
pub(crate) fn rot_y(m: &Mat, a: F) -> Mat {
    let (s, c) = vu_cossin(a);
    let (c, sn, ns) = (add(0, c), add(0, s), sub(0, s));
    vu_mul(&[[c, 0, ns, 0], [0, ONE, 0, 0], [sn, 0, c, 0], [0, 0, 0, ONE]], m)
}

/// `sceVu0RotMatrixZ(m, m, a)` (0x00110a30): Rz(a) m.
pub(crate) fn rot_z(m: &Mat, a: F) -> Mat {
    let (s, c) = vu_cossin(a);
    let (c, sn, ns) = (add(0, c), add(0, s), sub(0, s));
    vu_mul(&[[c, sn, 0, 0], [ns, c, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]], m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Scheme;

    struct One(V4);

    impl Scene for One {
        fn char_pos(&self, ty: i32, code: i32) -> Option<V4> {
            (ty == 2 && code == 2).then_some(self.0)
        }
        fn leader_pos(&self) -> V4 {
            [0, 0x45af_0000, 0x4416_0000, ONE]
        }
        fn marker_pos(&self, _: i16) -> Option<V4> {
            None
        }
        fn player_dirc(&self) -> V4 {
            [0; 4]
        }
    }

    /// The task starts with the first instruction and switches to the
    /// event camera on its first frame; `camera_end` goes back.
    #[test]
    fn switches_cameras() {
        let pos = [0, 0x45af_0000, 0x4416_0000, ONE];
        let mut cam = Camera::new(pos, [0; 4], 3, Scheme::new(0));
        let mut ev = EventCam::new();
        let scene = One([0x4200_0000, 0x45b0_0000, 0x4416_0000, ONE]);
        ev.command(CameraCommand::ZSet { v: [-2, 574, 102], c: [-3, 537, 68] }, &mut cam, &scene);
        assert_eq!((ev.task, cam.cam_id), (Task::Started, camera::id::FIELD));
        ev.frame(&mut cam, &scene, &Input::default());
        assert_eq!((ev.task, cam.cam_id), (Task::Running, camera::id::EVENT));
        assert_eq!(cam.ecam.view, [k(-20.0), k(5740.0), k(1020.0), ONE]);
        assert_eq!(cam.ecam.pos, [k(-30.0), k(5370.0), k(680.0), ONE]);
        ev.command(
            CameraCommand::Char { ty: 2, code: 2, height: 10, rotx: 1024, roty: -30720, dist: 40 },
            &mut cam,
            &scene,
        );
        ev.frame(&mut cam, &scene, &Input::default());
        assert_eq!(ev.ctrl.vp, [0x4200_0000, 0x45b0_0000, add(0x4416_0000, k(100.0)), ONE]);
        assert_eq!(ev.ctrl.dist, k(400.0));
        ev.command(CameraCommand::End, &mut cam, &scene);
        assert_eq!((ev.task, cam.cam_id), (Task::None, camera::id::FIELD));
    }
}
