//! `ccBossCam` (gcmn 0x0045fbe0-0x004610d0): the camera a boss fights under,
//! made by `ccBoss::InitBossCamera(z, y)` (0x0045e750) on camera 2 (`bcam`)
//! and run by `CamMain` each frame: over the player, behind him on the line
//! from the boss, the boss straight ahead, the heading turning at most
//! `RemitRotMax` a frame; the pad raises and lowers it; `QuakeCam` shakes it
//! for a frame. Innis uses modes 5 and 6 and `Xrot`; Kyvia modes 3 and 4
//! (Mutation's `SetTransfer`), `SetRotXLimit` and the stick's turn about
//! the player (`ZrotControlFlg`). `tools/test_bosscam_rs.py` checks it
//! (docs/engine/boss.md).

use crate::camera::{CamPad, Camera, id, kind};
use crate::ee::{self, F, ONE, V4, add, deg2rad, div, from_int, le, lt, mul, rad2deg, sinf, sub, vadd};
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

/// The turn's steps: the stick's sine times its power by this, or L1 and
/// R1 at 0.015 plus 0.0008 a step of pressure past 155 (0.08 more with no
/// pressure); held to +-0.5.
const ZROT_STICK: F = 0x3971_2c28;
const ZROT_BUTTON: F = 0x3c75_c28f;
const ZROT_PRESS: F = 0x3a51_b717;
const ZROT_NO_PRESS: F = 0x3da3_d70a;
const ZROT_MAX: F = 0x3f00_0000;
const ZROT_MIN: F = 0xbf00_0000;

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
    /// +0x128 `ExLock` (Mutation's +0x138): bit 4 from `SetMode(3)` until
    /// 4, bit 0 from 5 until 6; while either is set the pad is not read.
    pub ex_lock: u8,
    /// +0x129 `ZrotControlFlg` (Mutation's +0x139, set by Kyvia): the
    /// stick (or L1 and R1) turns the eye about the player (`ZrotCont`).
    pub zrot_control: bool,
    /// +0x40 `TempCamView`, +0x50 `TempCamPos`, +0x60 `TempCamRot`: what
    /// mode 5 kept and mode 6 puts back.
    pub temp_view: V4,
    pub temp_pos: V4,
    pub temp_rot: V4,
    /// Mutation's +0xd0 (the range `SetMode` was given, then the distance
    /// modes 3 and 4 ease to), +0x130 (their start taken) and +0x134 (the
    /// distance mode 3 left, for mode 4).
    pub trans_target: i32,
    pub trans_started: bool,
    pub trans_saved: F,
    /// Mutation's +0xdc: the eye raised by this times `1 - t` (the zoom's
    /// share), the constructor's 0 (only Quarantine's `ccBoss08_c` sets it).
    pub lift: F,
    /// From Mutation on `CamMain` adds the lift and sets `CamPos.w` 1.
    pub later: bool,
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
            ex_lock: 0,
            zrot_control: false,
            temp_view: [0; 4],
            temp_pos: [0; 4],
            temp_rot: [0; 4],
            trans_target: 0,
            trans_started: false,
            trans_saved: 0,
            lift: 0,
            later: camera.volume != piney_data::volume::Volume::Inf,
        }
    }

    /// `SetMode(mode, range, 0, 0)` (MUT gcmn 0x00475ae0) with a range:
    /// `+0xd0` the range as an int, then [`BossCam::set_mode`].
    pub fn set_mode_range(&mut self, mode: i32, range: F, camera: &mut Camera) {
        self.trans_target = ee::to_int(range);
        self.set_mode(mode, camera);
    }

    /// `SetRotXLimit(v)` (MUT gcmn 0x00475a40): the pitch's share at the
    /// nearest (`v`) and what the zoom adds (`1 - v`).
    pub fn set_rot_x_limit(&mut self, v: F) {
        self.basis_rot = v;
        self.limit_rot = sub(ONE, v);
    }

    /// `ccBossCam::SetMode(mode, 0, 0, 0)` (MUT gcmn 0x00475ae0) for modes
    /// 3 to 6: 3 and 4 set and clear `ExLock` bit 4; 5 locks the eye and
    /// keeps it; 6 puts it back, sets camera 2 there at once and unlocks
    /// (bit 0 set and cleared). `ResetFlg` is the mode (6 then 0).
    pub fn set_mode(&mut self, mode: i32, camera: &mut Camera) {
        self.reset = mode;
        match mode {
            3 => self.ex_lock |= 0x10,
            4 => self.ex_lock &= 1,
            5 => {
                self.ex_lock |= 1;
                self.lock = true;
                self.temp_view = self.view;
                self.temp_pos = self.pos;
                self.temp_rot = self.rot;
            }
            6 => {
                self.ex_lock &= 0x10;
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

    /// `ccBossCam::CamMain()` (0x0045fbe0, MUT gcmn 0x004755a0): the eye
    /// and view for this frame from the pad, `player` (`tempChar.pos`) and
    /// `view_at` (the boss's `pos`), set on camera 2 (`cameraSetPos`,
    /// `cameraSetView`) with the quake added.
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
                if self.later {
                    // MUT gcmn 0x004757a4: (0, 0, lift (1 - t), 1), w then 1.
                    let up = (f64::from(ee::f(self.lift)) * (1.0 - f64::from(ee::f(t)))) as f32;
                    self.pos = vadd(self.pos, [0, 0, up.to_bits(), ONE]);
                    self.pos[3] = ONE;
                }
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
            3 | 4 => self.ease_to_range(),
            _ => {}
        }
    }

    /// Mutation's `SetTransfer` modes 3 and 4 (MUT gcmn 0x00476380): the
    /// first frame takes the distance (mode 3: the eye's y off the view,
    /// kept that far from the range; mode 4: back to what mode 3 left),
    /// then `MoveTransfer.y` eases a quarter of the way a frame and snaps
    /// within 8, ending the mode.
    fn ease_to_range(&mut self) {
        if !self.trans_started {
            if self.reset == 3 {
                let d = ee::vsub(self.pos, self.view);
                let dy = f64::from(ee::f(d[1])).abs() as i32;
                let s2 = f64::from(dy - self.trans_target).abs() as i32;
                let off = ee::from_int(s2);
                let near = f64::from(ee::f(self.pos[1])).abs() < f64::from(self.trans_target);
                let y = self.move_transfer[1];
                self.trans_target = ee::to_int(if near { add(y, off) } else { sub(y, off) });
                self.trans_saved = y;
            } else {
                if !ee::eq(0, self.trans_saved) {
                    self.trans_target = ee::to_int(self.trans_saved);
                }
                self.trans_saved = 0;
            }
            self.trans_started = true;
            return;
        }
        let t = ee::from_int(self.trans_target);
        let y = self.move_transfer[1];
        let d = if le(y, t) { sub(t, y) } else { sub(y, t) };
        self.move_transfer[1] = if le(y, t) { add(y, div(d, FOUR)) } else { sub(y, div(d, FOUR)) };
        if lt(d, EIGHT) {
            self.move_transfer[1] = t;
            self.reset = 0;
            self.trans_started = false;
        }
    }

    /// `ccBossCam::Pad_Control()` (0x004602a0, MUT gcmn 0x00475cf0), not
    /// while `ExLock` holds. Schemes A (`cameraControlType` 0, 1): the
    /// right stick leant (power 2 or more) within pi/4 of angle 0 adds
    /// `checkCameraDistModValue` up to the farthest, within pi/4 of pi
    /// down to the nearest; R2 eases in, else L2 eases out. Schemes B
    /// (2, 3): R2 adds it up to the farthest and R1 down to the nearest,
    /// the same value both; L1 eases in and L2 out. Then the turn.
    fn pad_control(&mut self, camera: &Camera, pad: &CamPad) {
        self.camera_type = camera.scheme.number;
        if (self.ex_lock >> 4) | (self.ex_lock & 1) != 0 {
            return;
        }
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
            _ => return,
        }
        if self.zrot_control {
            self.zrot_turn(pad);
        }
    }

    /// `Pad_Control`'s turn with `ZrotControlFlg`: the right stick's
    /// sideways lean (`sinf(dirc) * pow`, past 1) turns `ZrotCont`; in
    /// schemes A, without it, L1 and R1 turn it by their pressure. The
    /// direction flips with the scheme's second type (1 and 3).
    fn zrot_turn(&mut self, pad: &CamPad) {
        let ty = self.camera_type;
        let lean = mul(from_int(i32::from(pad.pow_r)), sinf(pad.dirc_r));
        let button = |press: u8| match press {
            156.. => add(ZROT_BUTTON, mul(ZROT_PRESS, from_int(i32::from(press) - 155))),
            0 => add(ZROT_BUTTON, ZROT_NO_PRESS),
            _ => ZROT_BUTTON,
        };
        // + turns with a scheme's first type (0 or 2), - with its second.
        let step = if f64::from(ee::f(lean)).abs() > 1.0 {
            Some((mul(lean, ZROT_STICK), matches!(ty, 0 | 2)))
        } else if ty > 1 {
            None
        } else if pad.direct & L1 != 0 {
            Some((button(pad.pow[piney_input::pressure::L1]), ty == 0))
        } else if pad.direct & R1 != 0 {
            Some((button(pad.pow[piney_input::pressure::R1]), ty != 0))
        } else {
            None
        };
        if let Some((v, plus)) = step {
            self.zrot_cont = if plus { add(self.zrot_cont, v) } else { sub(self.zrot_cont, v) };
        }
        if !le(self.zrot_cont, ZROT_MAX) {
            self.zrot_cont = ZROT_MAX;
        }
        if lt(self.zrot_cont, ZROT_MIN) {
            self.zrot_cont = ZROT_MIN;
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
