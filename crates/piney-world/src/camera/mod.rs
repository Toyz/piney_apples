//! The field camera (`camera.cpp`, INF main 0x00160610-0x00163440): `tcam`
//! behind the player, turned and zoomed by the right stick and the scheme's
//! buttons, reset behind him, L2 for his eyes; pulled in front of what is
//! between them, and in a field kept above the ground (`avoidObstacle`); and the
//! view matrix (`cameraSet` 0x00161260). Three cameras (`tcam`, `bcam`, `ecam`),
//! drawn from the one `changeCamera` made active. EE single precision on bit
//! patterns ([`crate::ee`]); angles as radians or the game's 16-bit angle, each
//! conversion where the game makes it (docs/engine/field-game.md).

pub mod evcam;

use piney_input::pressure;

use piney_battle::camera::button::{L1, L2, R1, R2};
pub use piney_battle::camera::{Cam, CamId, CamList, CamPad, Scheme, Shake, ShockForce, kind, shake_range};
use piney_data::volume::Volume;
use piney_data::vu0::{UNIT, rot_matrix_x, rot_matrix_y, rot_matrix_z};

use crate::ee::{
    self, F, ONE, V4, add, atan2f, cosf, deg2rad, div, dot, fmodf, from_int, le, lt, mul, normalize, rad2deg, sinf,
    sqrtf_on, sub, tanf, to_int, vadd, vscale, vsub,
};

/// The field drawers' turn of the camera (the lens flare, rain, haze,
/// fireflies): `cameraGetRot2(1)` with the eye view (`checkCameraType()`
/// 1), else `cameraGetRot(1)`; `tcam`'s +0x40 or +0x20, all four lanes.
pub fn field_turn(cams: &CamList) -> V4 {
    if cams.kind() == kind::EYE { cams.get_rot2(CamId::Field, [0; 4]) } else { cams.get_rot(CamId::Field, [0; 4]) }
}

/// `-1.0`.
const MINUS_ONE: F = 0xbf80_0000;
/// `-pi` and `2 pi` as `SetMerchantCamera` loads them.
const MINUS_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
/// The look-at height above the start position `cameraInit` uses.
const INIT_VIEW_Z: F = 0x42f0_0000; // 120.0
/// The distance a reset (and a new camera) sits at.
pub const DIST_DEFAULT: F = 0x4472_8000; // 970.0
/// The nearest and farthest the camera zooms.
pub const DIST_MIN: F = 0x438c_0000; // 280.0
pub const DIST_MAX: F = 0x44f3_c000; // 1950.0
/// The pitch a reset (and type A) holds: 1512 / 65536 of a turn, 8.3 degrees.
pub const PITCH_DEFAULT: i16 = 1512;
/// `ccCam::Init`'s field of view (degrees).
pub const FOV: F = 0x4234_0000; // 45.0

/// The collision queries the camera makes (libhit, `docs/engine/field-game.md`).
pub trait CameraHits {
    /// `ccHitCheckLM(sp, ep, 4)`: the nearest hit on the segment from `sp`
    /// to `ep` as (contact point, distance), or None (-1.0).
    fn line(&mut self, sp: V4, ep: V4) -> Option<(V4, F)>;
    /// `ccModelHitCheckQZ(off, pos, 25.0, mask, 0)` in a town: the push out
    /// of the walls near `pos` (w 1), or None.
    fn sphere(&mut self, pos: V4) -> Option<V4>;
    /// `ccTransPosW2M` then `ccSetGroundHeight` (gcmn 0x0059b900,
    /// 0x0059b8d0; `WORLD_MAN::GetHeight` 0x001a10c0): the ground's height
    /// under a world point. Only a field asks (`avoidObstacle`); a Root
    /// Town's `ROOTTOWN::GetHeight` (0x005b61d0) answers 0.
    fn ground(&mut self, _pos: V4) -> F {
        0
    }
}

/// No walls.
pub struct NoHits;

impl CameraHits for NoHits {
    fn line(&mut self, _: V4, _: V4) -> Option<(V4, F)> {
        None
    }
    fn sphere(&mut self, _: V4) -> Option<V4> {
        None
    }
}

/// What `ccMenuCtrl::SetMerchantCamera` (gcmn 0x005269d0) reads of the
/// character spoken to (`cmndTarget`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MerchantView {
    /// A Grunt Shop's breeder (`base->type & 0x8000000`: `npcTbl` rows 10,
    /// 16, 22, 28): the town's fixed view, `breederCamView[server - 1]` and
    /// `breederCamPos[server - 1]` ([`breeder_view`]).
    Breeder { view: V4, pos: V4 },
    /// Anyone else: its `pos` and `dirc` (`ccChar` +0x40, +0x60).
    Char { pos: V4, dirc: V4 },
}

/// A breeder's [`MerchantView`] for `server` (`ccGame.server`): row
/// `server - 1` of `breederCamView[4][4]` and `breederCamPos[4][4]`
/// (merchan.cpp; the volume's `tables::world`), where the camera looks and
/// stands when a breeder is spoken to. None for a server outside 1-4 (the
/// game reads the words beside the tables; only servers 1-4 have Grunt
/// Shops).
pub fn breeder_view(volume: piney_data::volume::Volume, server: i32) -> Option<MerchantView> {
    let w = piney_data::tables::world::of(volume);
    let row = usize::try_from(server - 1).ok().filter(|&r| r < 4)?;
    let at = |t: [[f32; 4]; 4]| t[row].map(f32::to_bits);
    Some(MerchantView::Breeder { view: at(w.breeder_cam_view), pos: at(w.breeder_cam_pos) })
}

/// camera.cpp's state: `tcam` and the globals beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    /// `cameraList` and `camID`: `tcam` (this driver's), `bcam`, `ecam`.
    pub list: CamList,
    /// `camResetFlag` (0x00378974): a reset is turning the camera.
    pub resetting: bool,
    /// `memDircZ` (0x00378970): the heading the reset turns to.
    pub mem_dirc_z: i16,
    /// `camTypeLock` (0x00378978).
    pub type_lock: bool,
    /// `eventMng.puppetShow` (+0x78c) as camera.cpp reads it: set by the
    /// event's `menu_ban`, cleared by `menu_clear` ([`crate::camera::evcam`]).
    /// While set L2 does nothing and the eye view does not turn.
    pub puppet_show: bool,
    pub scheme: Scheme,
    /// `ccCam.matrix` after the last `cameraSet`: world to view.
    pub world_view: [V4; 4],
    /// `ccView.world_screen`: world to GS primitive coordinates.
    pub world_screen: [V4; 4],
    /// The screen shake (`cameraShake`, `cameraShockAbsorber`).
    pub shake: Shake,
}

/// `off = (dist sin rx, h sin a, -h cos a)` with h = dist cos rx and a the
/// heading turned half round: the camera `dist` behind `target` at pitch
/// `rx`, heading `rz` (`cameraInit`'s tail and cameraPosCalc 0x00161da4).
pub fn place(target: V4, rx: F, rz: F, dist: F) -> V4 {
    let oz = mul(dist, sinf(rx));
    let h = mul(dist, cosf(rx));
    let a = deg2rad((i32::from(rad2deg(rz)) + 0x8000) as i16);
    let ox = mul(h, sinf(a));
    let oy = mul(mul(MINUS_ONE, h), cosf(a));
    vadd(target, [ox, oy, oz, 0])
}

/// Where `avoidObstacle` (0x00162430) stops at first: 300 across the
/// ground, lengthened by the heading's angle off the nearer axis.
const OBSTACLE_STEP: F = 0x4396_0000; // 300.0
/// How far above the ground it keeps the camera.
const OBSTACLE_CLEARANCE: F = 0x4248_0000; // 50.0

/// `avoidObstacle(np, tp, cp, cam)` (0x00162430), a field's floor handling:
/// from the target toward the camera in steps of 300 along the ground (over the
/// cosine of the heading folded into 0-45 degrees; a quarter of that when the
/// camera is nearer), climbing at the camera's pitch; the first step under 50
/// above the ground is lifted to 50 and becomes the camera; if none, the camera
/// itself is checked (docs/engine/field-game.md).
pub fn avoid_obstacle(volume: Volume, tp: V4, cp: V4, cam: &Cam, hits: &mut dyn CameraHits) -> V4 {
    // The heading folded into 0..=8192 (0-45 degrees).
    let mut a = cam.deg[0] as u16;
    if a >= 0x8000 {
        a = a.wrapping_add(0x8000);
    }
    if a >= 16385 {
        a -= 16384;
    }
    if a >= 8193 {
        a = 16384 - a;
    }
    let ang = deg2rad(a as i16);
    let mut step = div(OBSTACLE_STEP, cosf(ang));
    let mut v = vsub(cp, tp);
    v[3] = ONE;
    let whole = sqrtf_on(volume, dot(v, v));
    v[2] = 0;
    let across = sqrtf_on(volume, dot(v, v));
    v = normalize(v);
    if lt(across, step) {
        step = div(div(OBSTACLE_STEP, cosf(ang)), 0x4080_0000);
    }
    v = vscale(v, step);
    v[2] = mul(step, tanf(cam.rot[0]));
    v[3] = ONE;
    let len = sqrtf_on(volume, dot(v, v));
    let mut n = to_int(div(whole, len));
    if lt(fmodf(whole, len), div(len, 0x4080_0000)) {
        n -= 1;
    }
    let mut p = vadd(tp, v);
    p[3] = ONE;
    let mut lift = |p: &mut V4| {
        let g = hits.ground(*p);
        if le(g, sub(p[2], OBSTACLE_CLEARANCE)) {
            false
        } else {
            p[2] = add(OBSTACLE_CLEARANCE, g);
            true
        }
    };
    for _ in 0..n.max(0) {
        if lift(&mut p) {
            return p;
        }
        p = vadd(p, v);
        p[3] = ONE;
    }
    let mut p = cp;
    p[3] = ONE;
    if lift(&mut p) { p } else { cp }
}

impl Camera {
    /// ccThCamera's set-up (0x00160610) and `cameraInit(0)` (0x00160a00) in
    /// a town: `changeCamera(1)`, the camera 970 behind the start position
    /// looking at it (120 up), pitch 1512, of type `camera_mode`
    /// (`saveData.cameraMode`, 3 on a new game; 0 is taken as 3); then
    /// `ecam` and `bcam` copied from it. On `volume`'s disc (its square
    /// root).
    pub fn new(volume: Volume, start_pos: V4, start_dirc: V4, camera_mode: i8, scheme: Scheme) -> Self {
        let mut view = start_pos;
        view[2] = add(view[2], INIT_VIEW_Z);
        let deg = [rad2deg(start_dirc[2]), PITCH_DEFAULT];
        let rot = [deg2rad(deg[1]), 0, deg2rad(deg[0]), 0];
        let dist = DIST_DEFAULT;
        let pos = place(view, rot[0], rot[2], dist);
        let tcam = Cam {
            pos,
            view,
            rot,
            rot2: [0; 4],
            rot3: [0; 4],
            dist,
            deg,
            kind: if camera_mode != 0 { i32::from(camera_mode) } else { kind::FOLLOW },
            reset_flag: false,
            reset_dirc: 0,
        };
        Camera {
            list: CamList::new(volume, tcam),
            resetting: false,
            mem_dirc_z: 0,
            type_lock: false,
            puppet_show: false,
            scheme,
            world_view: [[0; 4]; 4],
            world_screen: [[0; 4]; 4],
            shake: Shake::default(),
        }
    }

    /// `changeCamera(3)`, `cameraSetPos(pos, 3)`, `cameraSetView(view, 3)`:
    /// the event camera at `pos` looking at `view` (the field staff roll's).
    pub fn set_event(&mut self, pos: V4, view: V4) {
        self.list.change(CamId::Event);
        let c = self.list.cam_mut(CamId::Event);
        c.pos = pos;
        c.view = view;
    }

    /// `ccMenuCtrl::SetMerchantCamera()` (gcmn 0x005269d0), as a shop's menu
    /// opens: `changeCamera(3)`, `ecam`'s view and position (a breeder's the town's
    /// fixed pair; else 500 out from the merchant at his heading less 1.3708 and
    /// pitch less 0.1309, looking 100 above him), then `cameraGetRot`. The scratch's
    /// y and w are unset for a breeder in the game; 0 here. `changeCamera(1)` as the
    /// shop closes (docs/engine/field-ui.md).
    pub fn set_merchant(&mut self, target: MerchantView) {
        let l = &mut self.list;
        l.change(CamId::Event);
        let n = l.id();
        let scratch = match target {
            MerchantView::Breeder { view, pos } => {
                let c = l.cam_mut(n);
                c.view = view;
                c.pos = pos;
                [0; 4]
            }
            MerchantView::Char { pos, dirc } => {
                let mut view = pos;
                view[2] = add(view[2], 0x42c8_0000);
                let mut r = dirc;
                r[1] = sub(r[1], 0x3e06_0a92);
                r[2] = sub(r[2], 0x3faf_7641);
                if lt(r[2], MINUS_PI) {
                    r[2] = add(r[2], TWO_PI);
                } else if !le(r[2], ee::PI) {
                    r[2] = sub(r[2], TWO_PI);
                }
                let m = rot_matrix_z(&rot_matrix_y(&rot_matrix_x(&UNIT, r[0]), r[1]), r[2]);
                let c = l.cam_mut(n);
                c.view = view;
                c.pos = vadd(ee::apply(&m, [0x43fa_0000, 0, 0, ONE]), view);
                r
            }
        };
        let rot = l.get_rot(n, scratch);
        l.cam_mut(n).rot = rot;
    }

    /// `ccMenuCtrl::SetFountainCamera()` (gcmn 0x00526c60), each frame of the
    /// spring's talk: `changeCamera(3)`, `ecam` looking at the spring (z + 100, held
    /// to 0..800) from 240 behind Kite at 12 degrees off the line to it, then
    /// `cameraGetRot` (docs/engine/field-ui.md).
    pub fn set_fountain(&mut self, spring: V4, player: V4) {
        let l = &mut self.list;
        l.change(CamId::Event);
        let n = l.id();
        let mut view = spring;
        view[2] = add(view[2], 0x42c8_0000);
        if lt(view[2], 0) {
            view[2] = 0;
        } else if !le(view[2], 0x4448_0000) {
            view[2] = 0x4448_0000;
        }
        let mut pos = player;
        pos[2] = add(pos[2], 0x42c8_0000);
        let mut a = sub(atan2f(sub(pos[1], view[1]), sub(pos[0], view[0])), 0x3e56_7750);
        if lt(a, MINUS_PI) {
            a = add(a, TWO_PI);
        }
        let v = ee::apply(&rot_matrix_z(&UNIT, a), [0x4370_0000, 0, 0, ONE]);
        let c = l.cam_mut(n);
        c.view = view;
        c.pos = vadd(pos, v);
        let rot = l.get_rot(n, v);
        l.cam_mut(n).rot = rot;
    }

    /// `cameraMain` (0x00160cc0) on the camera task (priority 40), before the
    /// player moves: L2 switches following and the eye view (camera 1 active, no
    /// menu, no puppet show), the reset button starts turning the camera behind
    /// the player (not with camera 2 or 3 active), and the reset turns `tcam` a
    /// step. With the party wiped out the eye view goes back to following.
    pub fn main(&mut self, pad: &CamPad, player_dirc_z: F, menu_idle: bool, annihilated: bool, camera_mode: &mut i8) {
        let n = self.list.id();
        if annihilated {
            if n == CamId::Field && self.list.active().kind == kind::EYE {
                self.list.active_mut().kind = kind::FOLLOW;
            } else {
                self.reset_turn();
                return;
            }
        }
        if pad.push & L2 != 0
            && menu_idle
            && n == CamId::Field
            && !self.puppet_show
            && !self.resetting
            && !self.type_lock
        {
            let c = self.list.active_mut();
            let t = c.kind;
            if t == kind::FOLLOW {
                c.kind = kind::EYE;
            } else if t == kind::EYE {
                c.kind = kind::FOLLOW;
            }
            if t == kind::FOLLOW || t == kind::EYE {
                *camera_mode = c.kind as i8;
            }
            c.reset_flag = false;
            if t == kind::FOLLOW {
                let t = self.list.cam_mut(CamId::Field);
                t.rot2[0] = 0;
                t.rot2[2] = 0;
            }
        }
        let reset =
            (self.scheme.ctrl_type == 0 && pad.push & L1 != 0) || (self.scheme.ctrl_type == 1 && pad.push & R2 != 0);
        if reset
            && n != CamId::Event
            && n != CamId::Battle
            && self.list.active().kind == kind::FOLLOW
            && !self.resetting
        {
            self.mem_dirc_z = rad2deg(player_dirc_z);
            self.resetting = true;
            if pad.pow_l != 0 {
                let a = self.list.active_mut();
                a.reset_flag = true;
                a.reset_dirc = pad.dirc_l;
            }
        }
        self.reset_turn();
    }

    /// `cameraMain`'s last part: while the active camera follows and a
    /// reset runs, `tcam` turns one step back behind the player.
    fn reset_turn(&mut self) {
        if self.list.active().kind == kind::FOLLOW && self.resetting {
            let c = self.list.cam_mut(CamId::Field);
            let m = self.mem_dirc_z;
            let mut s = m.wrapping_sub(c.deg[0]);
            if i32::from(s).abs() >= 20481 {
                s = if s > 0 { 20480 } else { -20480 };
            }
            if i32::from(s).abs() < 128 {
                s = if s > 0 { 128 } else { -128 };
            }
            c.deg[0] = c.deg[0].wrapping_add(s >> 2);
            if (i32::from(m) - i32::from(c.deg[0])).abs() < 33 {
                c.deg[0] = m;
            }
            if (i32::from(PITCH_DEFAULT) - i32::from(c.deg[1])).abs() < 128 {
                c.deg[1] = PITCH_DEFAULT;
            }
            c.deg[1] = c.deg[1].wrapping_add(((i32::from(PITCH_DEFAULT) - i32::from(c.deg[1])) >> 2) as i16);
            c.rot[2] = deg2rad(c.deg[0]);
            c.rot[0] = deg2rad(c.deg[1]);
            // |(double)(970 - dist)| < 4: an exact comparison of the float
            // difference.
            if ee::f(sub(DIST_DEFAULT, c.dist)).abs() < 4.0 {
                c.dist = DIST_DEFAULT;
            }
            c.dist = add(c.dist, mul(0x3e80_0000, sub(DIST_DEFAULT, c.dist)));
            if c.deg[0] == m && c.deg[1] == PITCH_DEFAULT && c.dist == DIST_DEFAULT {
                self.resetting = false;
            }
        }
        self.shake.absorb();
    }

    /// `cameraPosCalc(target, &tcam)` (0x00161870) for the following
    /// camera, as `cameraSetManual` calls it: this frame's zoom and turn from
    /// the pad, then the camera placed behind `target` (the player's head,
    /// 140 above his feet) and pulled in front of whatever lies between
    /// (`area` 0 is a town, where it is also pushed out of walls).
    pub fn pos_calc(&mut self, target: V4, pad: &CamPad, area: i32, hits: &mut dyn CameraHits) {
        let (scheme, volume) = (self.scheme, self.list.volume());
        let dist_mod = if self.resetting { 0 } else { self.scheme.dist_mod(pad) };
        let c = self.list.cam_mut(CamId::Field);
        if !self.resetting {
            c.dist = add(c.dist, dist_mod);
            if lt(c.dist, DIST_MIN) {
                c.dist = DIST_MIN;
            }
            if !le(c.dist, DIST_MAX) {
                c.dist = DIST_MAX;
            }
            if pad.direct & 0xf000 == 0 && pad.pow_l == 0 {
                c.reset_flag = false;
            }
            let stick = |k2: F, trig: F| to_int(mul(from_int(i32::from(pad.pow_r)), mul(k2, trig))) as i16;
            if scheme.ctrl_type != 0 {
                let mut a = stick(k(2.0), sinf(pad.dirc_r));
                if a != 0 {
                    if !scheme.rev_lr {
                        a = a.wrapping_neg();
                    }
                } else {
                    if pad.direct & R1 != 0 {
                        let v = pad.pow[pressure::R1];
                        if v >= 11 {
                            a = ((i32::from(v) - 10) * 2 + 384) as i16;
                        } else if v == 0 {
                            a = 500;
                        }
                    }
                    if pad.direct & L1 != 0 {
                        let v = pad.pow[pressure::L1];
                        if v >= 11 {
                            a = (-384 - (i32::from(v) - 10) * 2) as i16;
                        } else if v == 0 {
                            a = -500;
                        }
                    }
                    if scheme.rev_lr {
                        a = a.wrapping_neg();
                    }
                }
                c.deg[0] = c.deg[0].wrapping_add(a);
                c.deg[1] = PITCH_DEFAULT;
            } else {
                let mut a = stick(k(2.0), sinf(pad.dirc_r));
                if scheme.rev_lr {
                    a = a.wrapping_neg();
                }
                c.deg[0] = c.deg[0].wrapping_sub(a);
                let mut b = stick(k(2.0), cosf(pad.dirc_r));
                if scheme.rev_ud {
                    b = b.wrapping_neg();
                }
                c.deg[1] = c.deg[1].wrapping_sub(b);
            }
            if (8192..32767).contains(&c.deg[1]) {
                c.deg[1] = 8191;
            }
            let t = div(sub(DIST_MAX, c.dist), sub(DIST_MAX, DIST_MIN));
            let min = sub(k(896.0), mul(k(896.0), t));
            if lt(from_int(i32::from(c.deg[1])), min) {
                c.deg[1] = to_int(min) as i16;
            }
            c.rot[2] = deg2rad(c.deg[0]);
            c.rot[0] = deg2rad(c.deg[1]);
        }
        let mut p = place(target, c.rot[0], c.rot[2], c.dist);
        // `game.area` 1, a field: over the ground.
        if area == 1 {
            p = avoid_obstacle(volume, target, p, c, hits);
        }
        if let Some((cp, d)) = hits.line(target, p) {
            let a = ee::normalize(vsub(cp, target));
            let mut s = sub(d, k(10.0));
            if lt(s, 0x3dcc_cccd) {
                s = 0x3dcc_cccd;
            }
            p = vadd(target, vscale(a, s));
        }
        if area == 0
            && let Some(off) = hits.sphere(p)
        {
            p = vadd(p, off);
            p[3] = ONE;
        }
        c.pos = p;
        c.view = target;
    }

    /// `cameraSetManual(target)` (0x00161820), `CameraPosSet`'s call for a
    /// camera that is not the eye view: [`Camera::pos_calc`] on `tcam`, but
    /// only while camera 1 is active and follows.
    pub fn set_manual(&mut self, target: V4, pad: &CamPad, area: i32, hits: &mut dyn CameraHits) {
        if self.list.id() == CamId::Field && self.list.active().kind == kind::FOLLOW {
            self.pos_calc(target, pad, area, hits);
        }
    }

    /// `cameraSoftReset()` (0x001631d0): `tcam` behind the player's heading
    /// (`plw->dirc.z`) at the default pitch and distance, at once.
    pub fn soft_reset(&mut self, player_dirc_z: F) {
        self.soft_reset_param(rad2deg(player_dirc_z), PITCH_DEFAULT, DIST_DEFAULT);
    }

    /// `cameraSoftResetParam(yaw, pitch, dist)` (0x00163230): `dist` -1
    /// means 970, clamped to 280-1950; the pitch 1512 for a type A scheme,
    /// else below 8192 and at least `896 - 896 (1950 - dist) / 1670`; then
    /// `tcam`'s angles and distance set.
    pub fn soft_reset_param(&mut self, yaw: i16, pitch: i16, dist: F) {
        let mut d = if ee::eq(MINUS_ONE, dist) { DIST_DEFAULT } else { dist };
        if lt(d, DIST_MIN) {
            d = DIST_MIN;
        }
        if !le(d, DIST_MAX) {
            d = DIST_MAX;
        }
        let mut p = pitch;
        if self.scheme.ctrl_type != 0 {
            p = PITCH_DEFAULT;
        } else {
            if (8192..32767).contains(&p) {
                p = 8191;
            }
            let min = sub(k(896.0), mul(k(896.0), div(sub(DIST_MAX, d), k(1670.0))));
            if lt(from_int(i32::from(p)), min) {
                p = to_int(min) as i16;
            }
        }
        let c = self.list.cam_mut(CamId::Field);
        c.rot[2] = deg2rad(yaw);
        c.rot[0] = deg2rad(p);
        c.deg = [yaw, p];
        c.dist = d;
    }

    /// `cameraSetEyeLevel(pos, rot)` (0x00162020): the eye view from the
    /// player's head, turned by the right stick and L1/R1; `rot` is the
    /// player's `angle` (+0x270), which it writes. Nothing unless camera 1
    /// is active. During a puppet show the pad does not turn it and `rot`
    /// is taken as it is; `tcam.deg[0]` is then stored from a stack slot
    /// the function never wrote (what the frame's earlier calls left
    /// there), which is not modelled: it is left as it was.
    pub fn eye_level(&mut self, pos: V4, rot: &mut V4, pad: &CamPad) {
        if self.list.id() != CamId::Field {
            return;
        }
        if !self.puppet_show {
            self.eye_turn(rot, pad);
        }
        let (f20, f21) = (rot[0], rot[2]);
        let c = self.list.cam_mut(CamId::Field);
        c.rot3 = [f20, 0, f21, 0];
        c.rot = [f20, 0, f21, 0];
        c.rot2[2] = f21;
        c.deg[1] = PITCH_DEFAULT;
        c.dist = DIST_DEFAULT;
        let mut v = pos;
        v[2] = add(v[2], mul(DIST_MIN, sinf(f20)));
        let h = mul(DIST_MIN, cosf(f20));
        v[0] = add(v[0], mul(h, sinf(f21)));
        v[1] = sub(v[1], mul(h, cosf(f21)));
        c.pos = pos;
        c.view = v;
    }

    /// `cameraSetEyeLevel`'s turn (0x00162060-0x001622e4): the heading by
    /// the right stick (2.5 sinf) or L1/R1, the pitch by the stick within
    /// +-7936; `rot` rewritten from them, `tcam.deg[0]` the heading.
    fn eye_turn(&mut self, rot: &mut V4, pad: &CamPad) {
        let mut dz = rad2deg(rot[2]);
        let mut dx = rad2deg(rot[0]);
        let mut a = to_int(mul(from_int(i32::from(pad.pow_r)), mul(0x4020_0000, sinf(pad.dirc_r)))) as i16;
        if a == 0 {
            // (short) dptoli(-384.0 - 1.5 (v - 10)) in doubles: exact here.
            if pad.direct & R1 != 0 {
                let v = pad.pow[pressure::R1];
                if v >= 11 {
                    a = (-384.0 - 1.5 * f64::from(i32::from(v) - 10)) as i16;
                } else if v == 0 {
                    a = -500;
                }
            }
            if pad.direct & L1 != 0 {
                let v = pad.pow[pressure::L1];
                if v >= 11 {
                    a = (384.0 + 1.5 * f64::from(i32::from(v) - 10)) as i16;
                } else if v == 0 {
                    a = 500;
                }
            }
        }
        dz = dz.wrapping_add(a);
        dx = dx.wrapping_add(to_int(mul(from_int(i32::from(pad.pow_r)), mul(ONE, cosf(pad.dirc_r)))) as i16);
        if (7937..32767).contains(&dx) {
            dx = 7936;
        }
        if (-32766..-7936).contains(&dx) {
            dx = -7936;
        }
        rot[2] = deg2rad(dz);
        rot[0] = deg2rad(dx);
        rot[1] = 0;
        self.list.cam_mut(CamId::Field).deg[0] = dz;
    }

    /// `cameraSet` (0x00161260): the view matrix from the active camera's
    /// eye and target (camera 1's through the screen shake,
    /// [`Shake::apply`]), the points taken through the player's map wrap
    /// (`ccTransPosFW2LW`, a no-op in towns but for the rounding of `(p -
    /// player) + player`), then `SetMatrix_PosTarget` and `SetView` on the
    /// one `ccCam` all the cameras share.
    pub fn set(&mut self, player_pos: V4) {
        let fw2lw = |p: V4| {
            [add(sub(p[0], player_pos[0]), player_pos[0]), add(sub(p[1], player_pos[1]), player_pos[1]), p[2], ONE]
        };
        let a = self.list.active();
        let (pos, view) = if self.list.id() == CamId::Field {
            self.shake.apply(self.list.volume(), a.pos, a.view)
        } else {
            (a.pos, a.view)
        };
        let p = fw2lw(pos);
        let v = fw2lw(view);
        self.world_view = pos_target(p, v);
        self.world_screen = piney_data::vu0::mul_matrix(&VIEW_SCREEN, &self.world_view);
    }
}

/// A constant's bits.
const fn k(x: f32) -> F {
    x.to_bits()
}

/// `ccView.view_screen` for the field's view: `ccSetViewScreenClipMatrix`
/// (0x00101f60) with fov 45 on the 512 x 448 frame, near 8, far 2^20, Z
/// 1..2^28, as the EE computes it (scrz 0x441a827a, aspect 0x3f955555, scy
/// 0x44ffffff: `SetFrame`'s float sums, not 2048).
pub const VIEW_SCREEN: [V4; 4] = [
    [0x441a_827a, 0, 0, 0],
    [0, 0x4434_42e3, 0, 0],
    [0x4500_0000, 0x44ff_ffff, 0xc4ff_e07f, ONE],
    [0, 0, 0x4f00_003f, 0],
];

/// `ccCam::SetMatrix_PosTarget(P, V)` (0x001387a0) through
/// `sceVu0CameraMatrix` (0x00110ca0): world to view, looking from `p` at
/// `v` with +y down the screen.
pub fn pos_target(p: V4, v: V4) -> [V4; 4] {
    let zd = vsub(v, p);
    let yd = if ee::eq(0, zd[0]) && ee::eq(0, zd[1]) { [ONE, 0, 0, 0] } else { [0, 0, MINUS_ONE, 0] };
    let x = ee::normalize(ee::cross(yd, zd));
    let z = ee::normalize(zd);
    let y = ee::cross(z, x);
    let cols = [[x[0], y[0], z[0], 0], [x[1], y[1], z[1], 0], [x[2], y[2], z[2], 0]];
    let t: [F; 3] = std::array::from_fn(|i| {
        let acc = mul(cols[0][i], p[0]);
        let acc = add(acc, mul(cols[1][i], p[1]));
        sub(0, add(acc, mul(cols[2][i], p[2])))
    });
    [cols[0], cols[1], cols[2], [t[0], t[1], t[2], ONE]]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values from the game's own `cameraInit(0)`, `cameraMain` and
    /// `CameraPosCalc`/`CameraPosSet` run in eemu: a start at (1500.5,
    /// -2500.25, 30) facing 0.7 radians, scheme A-1, no collision.
    /// `cameraMain` with the party wiped out: the eye view goes back to
    /// following (and the buttons are read that frame); a following camera
    /// ignores L2 and the reset button.
    #[test]
    fn a_wiped_out_party_follows() {
        let sp = [k(1500.5), k(-2500.25), k(30.0), ONE];
        let sd = [0, 0, k(0.7), 0];
        let mut c = Camera::new(piney_data::volume::Volume::Inf, sp, sd, 1, Scheme::new(0));
        assert_eq!(c.list.active().kind, kind::EYE);
        let mut mode = 1;
        c.main(&CamPad::default(), sd[2], true, true, &mut mode);
        assert_eq!(c.list.active().kind, kind::FOLLOW);
        let l2 = CamPad { push: L2, ..CamPad::default() };
        c.main(&l2, sd[2], true, true, &mut mode);
        assert_eq!(c.list.active().kind, kind::FOLLOW, "L2 is not read");
        let reset = CamPad { push: L1, ..CamPad::default() };
        c.main(&reset, sd[2], true, true, &mut mode);
        assert!(!c.resetting, "the reset button is not read");
        // Standing again, L2 works.
        c.main(&l2, sd[2], true, false, &mut mode);
        assert_eq!(c.list.active().kind, kind::EYE);
    }

    #[test]
    fn follows_as_the_game_does() {
        let sp = [k(1500.5), k(-2500.25), k(30.0), ONE];
        let sd = [0, 0, k(0.7), 0];
        let mut c = Camera::new(piney_data::volume::Volume::Inf, sp, sd, 3, Scheme::new(0));
        assert_eq!(c.list.cam(CamId::Field).view, [0x44bb_9000, 0xc51c_4400, 0x4316_0000, ONE]);
        assert_eq!(c.list.cam(CamId::Field).pos, [0x445c_9022, 0xc4dc_c1e9, 0x4391_0f68, ONE]);
        assert_eq!(c.list.cam(CamId::Field).rot, [0x3e14_70b4, 0, 0x3f33_3189, 0]);
        assert_eq!(
            (c.list.cam(CamId::Field).dist, c.list.cam(CamId::Field).deg, c.list.cam(CamId::Field).kind),
            (DIST_DEFAULT, [7301, 1512], kind::FOLLOW)
        );
        let head = [sp[0], sp[1], add(sp[2], k(140.0)), ONE];
        let frame = |c: &mut Camera, pad: CamPad| {
            let mut mode = 3;
            c.main(&pad, sd[2], true, false, &mut mode);
            c.pos_calc(head, &pad, 0, &mut NoHits);
            c.set(sp);
        };
        frame(&mut c, CamPad::default());
        assert_eq!(c.list.cam(CamId::Field).pos, [0x445c_9022, 0xc4dc_c1e9, 0x439b_0f68, ONE]);
        assert_eq!(c.list.cam(CamId::Field).view, [0x44bb_9000, 0xc51c_4400, 0x432a_0000, ONE]);
        assert_eq!(
            c.world_view,
            [
                [0xbf43_d1d0, 0xbdbe_8f22, 0x3f23_2aa0, 0],
                [0xbf24_e560, 0x3de2_4b9f, 0xbf41_c407, 0],
                [0x8000_0000, 0xbf7d_509f, 0xbe13_ebc1, 0],
                [0xc3e7_5af8, 0x4412_0669, 0xc4e7_c7ef, ONE],
            ]
        );
        assert_eq!(
            c.world_screen,
            [
                [0x4450_253b, 0x449a_c7b8, 0xc4a3_168b, 0x3f23_2aa0],
                [0xc4f3_870a, 0xc4b7_ce81, 0x44c1_ac2e, 0xbf41_c407],
                [0xc393_ebc1, 0xc47c_54bf, 0x4393_d98c, 0xbe13_ebc1],
                [0xca79_3c43, 0xca4e_1343, 0x4f00_3a29, 0xc4e7_c7ef],
            ]
        );
        // The right stick left: the heading turns by 509.
        frame(&mut c, CamPad { pow_r: 255, dirc_r: k(std::f32::consts::FRAC_PI_2), ..CamPad::default() });
        assert_eq!(c.list.cam(CamId::Field).deg[0], 7810);
        assert_eq!(c.list.cam(CamId::Field).pos, [0x4453_cb22, 0xc4e0_a2f2, 0x439b_0f68, ONE]);
        // Down: it draws back.
        frame(&mut c, CamPad { pow_r: 255, dirc_r: 0, ..CamPad::default() });
        assert_eq!(c.list.cam(CamId::Field).dist, 0x4484_6666);
        assert_eq!(c.list.cam(CamId::Field).pos, [0x4444_c614, 0xc4d8_8dc7, 0x43a1_80b9, ONE]);
    }
}
