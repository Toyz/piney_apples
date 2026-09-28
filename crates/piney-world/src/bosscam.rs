//! `ccBossCam` (gcmn 0x0045fbe0-0x004610d0): the camera a boss fights under,
//! made by `ccBoss::InitBossCamera(z, y)` (0x0045e750) on camera 2 (`bcam`)
//! and run by `CamMain` each frame: over the player, behind him on the line
//! from the boss, the boss straight ahead, the heading turning at most
//! `RemitRotMax` a frame; the pad raises and lowers it; `QuakeCam` shakes it
//! for a frame. Innis uses modes 5 and 6 and raises `Xrot`; `SetTransfer`'s
//! cases 3 and 4 and Mutation's lift are left out. `tools/test_bosscam_rs.py`
//! checks it (docs/engine/boss.md).

use crate::camera::{CamPad, Camera, id, kind};
use crate::ee::{self, F, ONE, V4, add, deg2rad, div, le, lt, mul, rad2deg, sub, vadd};
use crate::evcam::{UNIT, rot_x, rot_z};

/// `3 pi / 4` and `pi / 4`: where `Pad_Control` reads the right stick as
/// down (either side of +-3 pi / 4) or up (inside +-pi / 4).
const THREE_QUARTER_PI: F = 0x4016_cbe4;
const MINUS_THREE_QUARTER_PI: F = 0xc016_cbe4;
const QUARTER_PI: F = 0x3f49_0fdb;
const MINUS_QUARTER_PI: F = 0xbf49_0fdb;
/// `(0, -1000, 0)`: the point looked at, ahead of the eye before it turns.
const AHEAD: V4 = [0, 0xc47a_0000, 0, ONE];
/// A reset's step (a quarter of what is left) and where it snaps.
const FOUR: F = 0x4080_0000;
const EIGHT: F = 0x4100_0000;

/// Pad buttons (`ccPad.direct`).
const L2: u32 = 0x1;
const R2: u32 = 0x2;
const L1: u32 = 0x4;
const R1: u32 = 0x8;

/// A `ccBossCam` (0x140 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct BossCam {
    /// +0x00 `lock`: `CamMain` leaves the eye and view as they are.
    pub lock: bool,
    /// +0x01 `InitLock`: the first turn goes straight to the boss.
    pub init_lock: bool,
    /// +0x02 `QuakeFlg`, +0xc0 `Quakevector`: this frame's shake.
    pub quake_flg: bool,
    pub quake: V4,
    /// +0x03 `RenXrotFlg`.
    pub ren_xrot: bool,
    /// +0x04 `ResetFlg`: 0 the pad, 1 easing to the nearest, 2 to the
    /// farthest.
    pub reset: i32,
    /// +0x10 `CamView`, +0x20 `CamPos`, +0x30 `CamRot` (z the heading).
    pub view: V4,
    pub pos: V4,
    pub rot: V4,
    /// +0xa0 `transfer`: y the nearest the eye stands behind the player,
    /// z its height. +0xb0 `MoveTransfer`: where it stands now.
    pub transfer: V4,
    pub move_transfer: V4,
    /// +0xd8 `MaxRenge`: how much farther than `transfer.y` it zooms.
    pub max_range: F,
    /// +0xdc `Xrot`: the pitch.
    pub xrot: F,
    /// +0xe0 `deg`, +0xe2 `memDircZ`: the heading and the boss's, as
    /// 16-bit angles.
    pub deg: i16,
    pub mem_dirc_z: i16,
    /// +0xe4 `RemitRotMax`, +0xe6 `RemitRot`: the most a frame turns.
    pub remit_rot_max: i16,
    pub remit_rot: i16,
    /// +0xe8 `count`: frames spent catching up.
    pub count: i32,
    /// +0x100 `ZrotCont`, +0x104 `BasisRot`, +0x108 `LimitRot`.
    pub zrot_cont: F,
    pub basis_rot: F,
    pub limit_rot: F,
    /// +0x10c `CameraType`: `cameraControlType` as `Pad_Control` last read
    /// it (the constructor's 0 or 1 until then).
    pub camera_type: i32,
    /// +0x40 `TempCamView`, +0x50 `TempCamPos`, +0x60 `TempCamRot`: what
    /// mode 5 kept and mode 6 puts back.
    pub temp_view: V4,
    pub temp_pos: V4,
    pub temp_rot: V4,
}

impl BossCam {
    /// `ccBossCam::ccBossCam(view, transfer)` (0x00460ca0): the eye over
    /// `player` (`tempChar.pos`), `MoveTransfer` = `transfer`, camera 2 made
    /// active and of the following kind. `view` is the boss's place.
    pub fn new(camera: &mut Camera, transfer: V4, player: V4) -> Self {
        camera.change_camera(id::BATTLE);
        camera.active_mut().kind = kind::FOLLOW;
        // CamPos gets Rz(CamRot.z) transfer added, then is copied over
        // from tempChar.pos again; RotZ (+0xf0), the heading to the boss,
        // is never read.
        BossCam {
            lock: false,
            init_lock: true,
            quake_flg: false,
            quake: ee::VF0,
            ren_xrot: true,
            reset: 0,
            view: [0; 4],
            pos: player,
            rot: ee::VF0,
            transfer,
            move_transfer: transfer,
            max_range: 0x447a_0000,
            xrot: 0,
            deg: 0,
            mem_dirc_z: 0,
            remit_rot_max: 1280,
            remit_rot: 1280,
            count: 0,
            zrot_cont: 0,
            basis_rot: 0,
            limit_rot: ONE,
            camera_type: i32::from(matches!(camera.scheme.number, 2 | 3)),
            temp_view: [0; 4],
            temp_pos: [0; 4],
            temp_rot: [0; 4],
        }
    }

    /// `ccBossCam::SetMode(mode, 0, 0, 0)` (MUT gcmn 0x00475ae0) for modes
    /// 5 and 6: 5 locks the eye and keeps it; 6 puts it back, sets camera
    /// 2 there at once and unlocks. `ResetFlg` is the mode (6 then 0).
    pub fn set_mode(&mut self, mode: i32, camera: &mut Camera) {
        self.reset = mode;
        match mode {
            5 => {
                self.lock = true;
                self.temp_view = self.view;
                self.temp_pos = self.pos;
                self.temp_rot = self.rot;
            }
            6 => {
                self.lock = false;
                self.view = self.temp_view;
                self.pos = self.temp_pos;
                let c = camera.cam_mut(id::BATTLE);
                c.pos = self.pos;
                c.view = self.view;
                self.reset = 0;
            }
            _ => {}
        }
    }

    /// `ccBossCam::SetFreeCamPosView(pos, view)` (MUT 0x00475c90): in
    /// mode 5 the eye and the point looked at.
    pub fn free_cam_pos_view(&mut self, pos: V4, view: V4) {
        if self.reset == 5 {
            self.view = view;
            self.pos = pos;
        }
    }

    /// `ccBossCam::CheckMoveCamera()` (MUT 0x004766d0): the heading still
    /// catching up (`count`, Mutation's +0xec).
    pub fn check_move_camera(&self) -> bool {
        self.count & 0xffff != 0
    }

    /// `ccBossCam::QuakeCam(q)` (0x0045ff80): the next `CamMain` shakes by
    /// `v`, three `ccRandF(q[k])` the caller has drawn.
    pub fn quake(&mut self, v: [F; 3]) {
        self.quake[..3].copy_from_slice(&v);
        self.quake_flg = true;
    }

    /// `ccBossCam::CamMain()` (0x0045fbe0): the eye and view for this frame
    /// from the pad, `player` (`tempChar.pos`) and `view_at` (the boss's
    /// `pos`), set on camera 2 (`cameraSetPos`, `cameraSetView`) with the
    /// quake added.
    pub fn cam_main(&mut self, camera: &mut Camera, pad: &CamPad, player: V4, view_at: V4) {
        if !self.lock {
            self.view = view_at;
            self.set_transfer(camera, pad);
            let d = self.pos_view_dirc(player, view_at);
            self.rot = ee::apply(&rot_z(&UNIT, self.zrot_cont), self.rot);
            self.mem_dirc_z = rad2deg(d[2]);
            self.deg = rad2deg(self.rot[2]);
            self.rot[2] = self.z_rot_deg2dad(self.mem_dirc_z, self.deg);
            if matches!(self.reset, 0..=4) {
                let mut t = 0;
                let left = sub(add(self.transfer[1], self.max_range), self.move_transfer[1]);
                if !lt(left, 0) {
                    t = div(left, self.max_range);
                }
                if le(t, 0) {
                    t = 0;
                }
                let x =
                    if self.ren_xrot { mul(self.xrot, add(self.basis_rot, mul(t, self.limit_rot))) } else { self.xrot };
                let z = self.zrot_cont;
                let m = rot_z(&rot_z(&UNIT, self.rot[2]), z);
                self.pos = vadd(self.pos, ee::apply(&m, self.move_transfer));
                let m = rot_z(&rot_z(&rot_x(&UNIT, x), self.rot[2]), z);
                self.view = vadd(self.pos, ee::apply(&m, AHEAD));
            } else {
                let m = rot_z(&UNIT, self.rot[2]);
                self.pos = vadd(self.pos, ee::apply(&m, self.move_transfer));
                self.view = vadd(self.pos, ee::apply(&m, AHEAD));
            }
        }
        let c = camera.cam_mut(id::BATTLE);
        if self.quake_flg {
            c.pos = vadd(self.pos, self.quake);
            c.view = vadd(self.view, self.quake);
        } else {
            c.pos = self.pos;
            c.view = self.view;
        }
        self.quake = ee::VF0;
        self.quake_flg = false;
    }

    /// `ccBossCam::GetPosViewDirc(out)` (0x00460010): the eye back over the
    /// player, and (0, 0, the heading from him to the boss, 1).
    fn pos_view_dirc(&mut self, player: V4, view_at: V4) -> V4 {
        self.pos = player;
        [0, 0, crate::merchant::get_dirc(self.pos, view_at), ONE]
    }

    /// `ccBossCam::SetTransfer()` (0x00460930): the pad, or a reset easing
    /// `MoveTransfer.y` a quarter of the way to its end each frame and
    /// snapping there once within 8.
    fn set_transfer(&mut self, camera: &Camera, pad: &CamPad) {
        let y = self.move_transfer[1];
        match self.reset {
            0 => self.pad_control(camera, pad),
            1 => {
                let d = sub(y, self.transfer[1]);
                self.move_transfer[1] = sub(y, div(d, FOUR));
                if lt(d, EIGHT) {
                    self.move_transfer[1] = self.transfer[1];
                    self.reset = 0;
                }
            }
            2 => {
                let d = sub(add(self.transfer[1], self.max_range), y);
                self.move_transfer[1] = add(y, div(d, FOUR));
                if lt(d, EIGHT) {
                    self.move_transfer[1] = add(self.transfer[1], self.max_range);
                    self.reset = 0;
                }
            }
            _ => {}
        }
    }

    /// `ccBossCam::Pad_Control()` (0x004602a0), with `ExLock` and
    /// `ZrotControlFlg` 0. Schemes A (`cameraControlType` 0, 1): the right
    /// stick leant (power 2 or more) within pi/4 of angle 0 adds
    /// `checkCameraDistModValue` up to the farthest, within pi/4 of pi
    /// down to the nearest; R2 eases in, else L2 eases out. Schemes B
    /// (2, 3): R2 adds it up to the farthest and R1 down to the nearest,
    /// the same value both; L1 eases in and L2 out.
    fn pad_control(&mut self, camera: &Camera, pad: &CamPad) {
        self.camera_type = camera.scheme.number;
        let far = |s: &Self| add(s.transfer[1], s.max_range);
        let zoom_out = |s: &mut Self| {
            s.move_transfer[1] = add(s.move_transfer[1], camera.dist_mod(pad));
            let f = far(s);
            if lt(f, s.move_transfer[1]) {
                s.move_transfer[1] = f;
            }
        };
        let zoom_in = |s: &mut Self| {
            s.move_transfer[1] = add(s.move_transfer[1], camera.dist_mod(pad));
            if !le(s.transfer[1], s.move_transfer[1]) {
                s.move_transfer[1] = s.transfer[1];
            }
        };
        match self.camera_type {
            0 | 1 => {
                if pad.pow_r >= 2 {
                    let a = pad.dirc_r;
                    let down = !le(a, THREE_QUARTER_PI) || lt(a, MINUS_THREE_QUARTER_PI);
                    let up = lt(a, QUARTER_PI) && !le(a, MINUS_QUARTER_PI);
                    if up {
                        zoom_out(self);
                    } else if down {
                        zoom_in(self);
                    }
                }
                if pad.direct & R2 != 0 {
                    self.reset = 1;
                } else if pad.direct & L2 != 0 {
                    self.reset = 2;
                }
            }
            2 | 3 => {
                if pad.direct & R2 != 0 {
                    zoom_out(self);
                }
                if pad.direct & R1 != 0 {
                    zoom_in(self);
                }
                if pad.direct & L1 != 0 {
                    self.reset = 1;
                }
                if pad.direct & L2 != 0 {
                    self.reset = 2;
                }
            }
            _ => {}
        }
    }

    /// `ccBossCam::CamZRotDeg2Dad(target, cur)` (0x00460ea0): the heading
    /// `cur` turned toward `target` (16-bit angles), in radians. A quarter
    /// of the difference a frame, the difference first held to
    /// `RemitRotMax` (counting the frames it was), doubled after 20 such
    /// frames, 128 when it is 113-127 and none under 112, where the heading
    /// is the target's. The first call, or a turn of nothing, takes the
    /// target outright; within 128 of it (as ints, no wrap) it is the
    /// target and the count starts again.
    fn z_rot_deg2dad(&mut self, target: i16, cur: i16) -> F {
        let mut s = target.wrapping_sub(cur);
        let a = i32::from(s).abs() as i16;
        if !self.init_lock && self.remit_rot_max < a {
            s = if s > 0 { self.remit_rot_max } else { self.remit_rot_max.wrapping_neg() };
            self.count += 1;
        }
        if self.count >= 21 {
            s = s.wrapping_shl(1);
        }
        if (113..128).contains(&a) {
            s = if s > 0 { 128 } else { -128 };
            self.count = 2;
        }
        if a < 112 {
            s = 0;
            self.count = 0;
        }
        let mut deg = cur;
        if !self.init_lock && s != 0 {
            deg = deg.wrapping_add(s >> 2);
        } else {
            deg = target;
            self.init_lock = false;
        }
        if (i32::from(target) - i32::from(deg)).abs() < 129 {
            deg = target;
            self.count = 0;
            self.remit_rot_max = self.remit_rot;
        }
        deg2rad(deg)
    }
}

impl Camera {
    /// `ccBoss::OffBossCamera` (0x0045ede0) and the first part of
    /// `BeginDeadEffect`'s camera: camera 2's eye, view and rotation
    /// (`cameraGetRot(r, 2)`, y and w of the scratch, zero here) copied to
    /// camera 1, then `changeCamera(1)`.
    pub fn boss_cam_off(&mut self) {
        let (pos, view) = (self.bcam.pos, self.bcam.view);
        let rot = self.get_rot(id::BATTLE, [0; 4]);
        let t = self.cam_mut(id::FIELD);
        t.pos = pos;
        t.view = view;
        t.rot = rot;
        self.change_camera(id::FIELD);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Scheme;

    fn cam() -> (Camera, BossCam) {
        let mut camera = Camera::new([0, 0, 0, ONE], [0; 4], 3, Scheme::new(0));
        let b = BossCam::new(&mut camera, [0, 0x447a_0000, 0x4348_0000, ONE], [0, 0, 0, ONE]);
        (camera, b)
    }

    #[test]
    fn stands_behind_the_player_facing_the_boss() {
        let (mut camera, mut b) = cam();
        assert_eq!(camera.cam_id, id::BATTLE);
        // The boss 500 below (at -y): the eye 1000 above him, 200 up.
        let boss = [0, ee::k(-500.0), 0, ONE];
        b.cam_main(&mut camera, &CamPad::default(), [0, 0, 0, ONE], boss);
        let p = camera.bcam.pos;
        assert!((ee::f(p[1]) - 1000.0).abs() < 0.01 && ee::f(p[2]) == 200.0, "{:?}", p.map(ee::f));
        assert!(ee::f(camera.bcam.view[1]) < ee::f(p[1]));
        assert!(!b.init_lock);
    }

    #[test]
    fn turns_at_most_remit_rot_max_a_frame() {
        let (_, mut b) = cam();
        b.init_lock = false;
        let r = b.z_rot_deg2dad(16384, 0);
        assert_eq!(r, deg2rad(1280 >> 2));
        assert_eq!(b.count, 1);
        // Within 112: the target.
        assert_eq!(b.z_rot_deg2dad(100, 0), deg2rad(100));
        assert_eq!(b.count, 0);
    }

    #[test]
    fn a_quake_lasts_one_frame() {
        let (mut camera, mut b) = cam();
        let boss = [0, ee::k(-500.0), 0, ONE];
        b.cam_main(&mut camera, &CamPad::default(), [0, 0, 0, ONE], boss);
        let still = camera.bcam.pos;
        b.quake([ee::k(10.0), 0, 0]);
        b.cam_main(&mut camera, &CamPad::default(), [0, 0, 0, ONE], boss);
        assert_eq!(camera.bcam.pos[0], ee::add(still[0], ee::k(10.0)));
        b.cam_main(&mut camera, &CamPad::default(), [0, 0, 0, ONE], boss);
        assert_eq!(camera.bcam.pos, still);
    }
}
