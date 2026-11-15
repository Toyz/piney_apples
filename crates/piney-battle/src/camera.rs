//! camera.cpp's list (INF main 0x002fb6f0, MUT 0x00310f70, OUT 0x003086a0,
//! QUA 0x001fc620): `cameraList[4]` holds `tcam`, `tcam`, `bcam` and `ecam`,
//! and `camID` (a short) names the active one (`activeCamPtr`). Every reader
//! and writer outside camera.cpp goes through its accessors with a camera
//! number (`cameraGet/SetPos`, `View`, `Rot`, `changeCamera`,
//! `checkCameraID`, `checkCameraType`), so this is the one place a camera is
//! kept; the drivers (the field camera, the event camera, a boss's camera,
//! the menus) write through it (docs/engine/field-game.md, "The camera").

use piney_data::volume::Volume;

use piney_input::{Pad, pressure};

use crate::enemy_ai::Frame;
use crate::geom::{
    F, MINUS_ONE, ONE, V4, add, apply_matrix, atan2f, cosf, deg2rad, div, dot, from_int, k, le, lt, mul, neg, rad2deg,
    rot_matrix_y, rot_matrix_z, sinf, sub, unit_matrix, vadd, vscale, vsub,
};
use button::{R1, R2};

/// `CAMERA.type`.
pub mod kind {
    /// Looking from the player's eyes (L2).
    pub const EYE: i32 = 1;
    pub const FOLLOW: i32 = 3;
}

/// Pad buttons the cameras read (`ccPad.direct` / `push` bits).
pub mod button {
    pub const L2: u32 = 0x1;
    pub const R2: u32 = 0x2;
    pub const L1: u32 = 0x4;
    pub const R1: u32 = 0x8;
}

/// The control scheme (`setCameraCtrlType` 0x001611a0): A-1, A-2, B-1, B-2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scheme {
    /// `camCtrlType`: 1 for A (right stick X turns, Y zooms; L1/R1 turn;
    /// R2 resets), 0 for B (right stick turns and pitches; R1/R2 zoom; L1
    /// resets).
    pub ctrl_type: i32,
    /// `camRevLR`, `camRevUD`.
    pub rev_lr: bool,
    pub rev_ud: bool,
    /// `cameraControlType`: the scheme's number.
    pub number: i32,
}

/// `cameraControlType` 0: A-1.
impl Default for Scheme {
    fn default() -> Self {
        Scheme::new(0)
    }
}

impl Scheme {
    /// `setCameraCtrlType(n)`; anything but 1-3 is A-1.
    pub fn new(n: i32) -> Self {
        match n {
            1 => Scheme { ctrl_type: 1, rev_lr: false, rev_ud: false, number: 1 },
            2 => Scheme { ctrl_type: 0, rev_lr: true, rev_ud: false, number: 2 },
            3 => Scheme { ctrl_type: 0, rev_lr: false, rev_ud: false, number: 3 },
            _ => Scheme { ctrl_type: 1, rev_lr: true, rev_ud: false, number: 0 },
        }
    }

    /// `checkCameraDistModValue` (0x00162fb0): this frame's zoom. Type B
    /// (`camCtrlType` 0): R1 in, R2 out, by their pressure; type A: the
    /// right stick's lean down its y past 64.
    pub fn dist_mod(&self, pad: &CamPad) -> F {
        let mut f = 0;
        if self.ctrl_type == 0 {
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
            if pad.direct & R1 != 0 {
                f = neg(press(pad.pow[pressure::R1]));
            }
            if pad.direct & R2 != 0 {
                f = press(pad.pow[pressure::R2]);
            }
        } else {
            let mut g = mul(from_int(i32::from(pad.pow_r)), cosf(pad.dirc_r));
            if f32::from_bits(g).abs() > 64.0 {
                g = if le(g, 0) { add(g, k(32.0)) } else { sub(g, k(32.0)) };
                g = mul(g, 0x3ecc_cccd);
                if self.rev_ud {
                    g = neg(g);
                }
                f = g;
            }
        }
        f
    }
}

/// What the camera reads of the pad, as `ccPad` holds it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CamPad {
    pub direct: u32,
    pub push: u32,
    pub pow_l: u8,
    pub dirc_l: F,
    pub pow_r: u8,
    pub dirc_r: F,
    pub pow: [u8; 12],
}

impl CamPad {
    /// The pad's state this frame as the camera reads it.
    pub fn from_pad(p: &Pad) -> Self {
        CamPad {
            direct: p.direct.bits(),
            push: p.push.bits(),
            pow_l: p.pow_l,
            dirc_l: p.dirc_l.to_bits(),
            pow_r: p.pow_r,
            dirc_r: p.dirc_r.to_bits(),
            pow: p.pow,
        }
    }
}

/// A `CAMERA` (0x70 bytes; `cptr`, +0x50, is the one `ccCam` every camera
/// draws through, so it is not kept here).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cam {
    /// +0x00 the eye, +0x10 the point looked at.
    pub pos: V4,
    pub view: V4,
    /// +0x20: x the pitch, z the heading, radians.
    pub rot: V4,
    pub rot2: V4,
    pub rot3: V4,
    /// +0x54.
    pub dist: F,
    /// +0x58: heading and pitch as 16-bit angles.
    pub deg: [i16; 2],
    /// +0x5c: [`kind`].
    pub kind: i32,
    /// +0x60 `resetFlag`: the stick was leant when a reset started (the
    /// player walks straight ahead while it holds), +0x64 its direction.
    pub reset_flag: bool,
    pub reset_dirc: F,
}

/// `camID`: which of `cameraList`'s four entries is active. All 71
/// `changeCamera` calls (main and gcmn, all four volumes) pass 1, 2 or 3,
/// or a camID read back earlier (`ccEvent::Execute`'s eventMng +0x7b4,
/// `ccBoss04::OffMagicCamera`); only `camera_end_reset` before any
/// `menu_ban` restores eventMng's unwritten 0, entry 0 (`tcam` under
/// another number), which no script reaches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CamId {
    /// `cameraList[0]`: `tcam`, but not camera 1 to `checkCameraID`.
    Zero = 0,
    /// `tcam`, the field camera: what `ccThCamera`'s set-up selects.
    #[default]
    Field = 1,
    /// `bcam` (`ccBoss::InitBossCamera`'s).
    Battle = 2,
    /// `ecam`: the event camera's, the menus' and the bosses' fixed views.
    Event = 3,
}

impl CamId {
    /// camID as the game keeps it.
    pub const fn raw(self) -> i16 {
        self as i16
    }

    /// camID from the game's value: None for anything but 0-3.
    pub fn from_raw(n: i32) -> Option<CamId> {
        match n {
            0 => Some(CamId::Zero),
            1 => Some(CamId::Field),
            2 => Some(CamId::Battle),
            3 => Some(CamId::Event),
            _ => None,
        }
    }
}

/// `cameraList` and `camID`: the three cameras, the active one's number,
/// and the disc's volume (its code's square root, [`CamList::get_rot`]).
/// The default is three zeroed cameras on camera 1, Infection's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CamList {
    tcam: Cam,
    bcam: Cam,
    ecam: Cam,
    id: CamId,
    volume: Volume,
}

impl CamList {
    /// `ccThCamera`'s set-up (0x00160610): `changeCamera(1)`, `tcam` as
    /// made, and `ecam` and `bcam` copied from it (0x00160730-0x00160990).
    pub fn new(volume: Volume, tcam: Cam) -> Self {
        CamList { bcam: tcam.clone(), ecam: tcam.clone(), tcam, id: CamId::Field, volume }
    }

    pub fn volume(&self) -> Volume {
        self.volume
    }

    /// `cameraList[n]`: entries 0 and 1 are both `tcam`.
    pub fn cam(&self, n: CamId) -> &Cam {
        match n {
            CamId::Zero | CamId::Field => &self.tcam,
            CamId::Battle => &self.bcam,
            CamId::Event => &self.ecam,
        }
    }

    pub fn cam_mut(&mut self, n: CamId) -> &mut Cam {
        match n {
            CamId::Zero | CamId::Field => &mut self.tcam,
            CamId::Battle => &mut self.bcam,
            CamId::Event => &mut self.ecam,
        }
    }

    /// `checkCameraID()` (0x00160ca0): camID.
    pub fn id(&self) -> CamId {
        self.id
    }

    /// `changeCamera(n)` (0x001617f0): camID, and `activeCamPtr` with it.
    pub fn change(&mut self, n: CamId) {
        self.id = n;
    }

    /// `activeCamPtr`: `cameraList[camID]`.
    pub fn active(&self) -> &Cam {
        self.cam(self.id)
    }

    pub fn active_mut(&mut self) -> &mut Cam {
        self.cam_mut(self.id)
    }

    /// `checkCameraType()` (0x00160cb0): `activeCamPtr->type`.
    pub fn kind(&self) -> i32 {
        self.active().kind
    }

    /// `cameraGetRot(out, n)` (0x00161610): for camera 1 `tcam.rot`, all
    /// four lanes; for any other, x the pitch `atan2f(pos.z - view.z,
    /// |view - pos| across)` and z the heading `atan2f(view.x - pos.x,
    /// -1 (view.y - pos.y))` of `cameraList[n]`, y and w left as `out` has
    /// them.
    pub fn get_rot(&self, n: CamId, out: V4) -> V4 {
        match n {
            CamId::Field => self.tcam.rot,
            _ => self.rot_of(n, out),
        }
    }

    /// The listener a 3D sound is panned by at its call ([`Ear`]).
    pub fn ear(&self) -> Ear {
        let a = self.active();
        Ear { pos: a.pos, view: a.view, kind: a.kind }
    }

    /// `cameraGetRot(r, camID)` with a zeroed scratch: the active camera's
    /// turn, as most callers ask it.
    pub fn rot(&self) -> V4 {
        self.get_rot(self.id, [0; 4])
    }

    /// `cameraGetRot2(out, n)` (0x00161700): for camera 1 `tcam.rot3`
    /// (+0x40), all four lanes; for any other what `cameraGetRot` works out.
    pub fn get_rot2(&self, n: CamId, out: V4) -> V4 {
        match n {
            CamId::Field => self.tcam.rot3,
            _ => self.rot_of(n, out),
        }
    }

    /// The turn from the eye to the point looked at: `sceVu0SubVector`, z
    /// zeroed, `sqrtf(InnerProduct)`, two `atan2f`s.
    fn rot_of(&self, n: CamId, out: V4) -> V4 {
        let c = self.cam(n);
        let mut d = vsub(c.view, c.pos);
        d[2] = 0;
        let h = piney_data::libm::sqrtf_on(self.volume, dot(d, d));
        let x = atan2f(sub(c.pos[2], c.view[2]), h);
        let z = atan2f(sub(c.view[0], c.pos[0]), mul(MINUS_ONE, sub(c.view[1], c.pos[1])));
        [x, out[1], z, out[3]]
    }
}

/// `camShockForce` (8 bytes): `power` (4 signed bits, -1 a free slot),
/// `cycle` and `rot` (4 bits each), `time`, `dirc` (a 16-bit angle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShockForce {
    pub power: i8,
    pub cycle: i8,
    pub rot: i8,
    pub time: i16,
    pub dirc: i16,
}

/// `shakePowerTbl` (main 0x0034b390) and `shakeCycleTbl` (0x0034b3a0).
const SHAKE_POWER: [F; 4] = [0x4120_0000, 0x41a0_0000, 0x41f0_0000, 0];
const SHAKE_CYCLE: [i16; 4] = [8, 6, 4, 0];

/// camera.cpp's shake: `sfList[8]` (0x00383e30), `vibrateForce`
/// (0x00378998), `vibrateCycle` (0x00378994), `vibrateRotate`
/// (0x0037899c), `vibrateMatrix` (0x00383e80), `vibrateOffset`
/// (0x00383e70, only its z is ever set) and `oscillator` (0x003789a0).
/// `ccThCamera`'s start frees the slots and zeroes the force and offset;
/// the others are statics the executable starts at 0 (so `vibrateRotate`
/// names slot 0 until a shake without `rot` sets it to -1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shake {
    pub sf: [ShockForce; 8],
    pub force: F,
    pub cycle: i16,
    pub rotate: i32,
    pub matrix: [V4; 4],
    pub offset_z: F,
    pub oscillator: i16,
}

impl Default for Shake {
    fn default() -> Self {
        Shake {
            sf: [ShockForce { power: -1, cycle: 0, rot: 0, time: 0, dirc: 0 }; 8],
            force: 0,
            cycle: 0,
            rotate: 0,
            matrix: [[0; 4]; 4],
            offset_z: 0,
            oscillator: 0,
        }
    }
}

impl Shake {
    /// `cameraShake(power, cycle, time, dirc)` (main 0x00162cd0). A shock
    /// weaker than a held one and shorter is dropped; one at least as
    /// strong and as long takes that slot; else the first free slot, if
    /// any. `dirc` 0: straight (dirc 0), 1: across (0x4000), 2: a
    /// turning shake from `rand()` (`(rand() << 7) & 0xf800`, `rot` 1).
    pub fn shake(&mut self, power: i32, cycle: i32, time: i32, dirc: i32, rand: &mut dyn FnMut() -> i32) {
        let nib = |v: i32| ((v << 28) >> 28) as i8;
        let (power, cycle) = (nib(power), (cycle & 0xf) as i8);
        let mut slot = None;
        for (i, s) in self.sf.iter().enumerate() {
            if s.power == -1 {
                continue;
            }
            if power < s.power && time < i32::from(s.time) {
                return;
            }
            if power >= s.power && time >= i32::from(s.time) {
                slot = Some(i);
                break;
            }
        }
        let Some(i) = slot.or_else(|| self.sf.iter().position(|s| s.power == -1)) else { return };
        let s = &mut self.sf[i];
        s.power = power;
        s.cycle = cycle;
        s.time = time as i16;
        match dirc {
            0 => {
                s.dirc = 0;
                s.rot = 0;
            }
            1 => {
                s.dirc = 0x4000;
                s.rot = 0;
            }
            2 => {
                s.dirc = ((rand() << 7) & 0xf800) as i16;
                s.rot = 1;
            }
            _ => {}
        }
    }

    /// `cameraSet`'s shake for camera 1: with `a = atan2(d.x, -d.y)` (in
    /// double) of `d = view - pos` across the ground, `o = 0.5 (Rz(a)
    /// vibrateMatrix) vibrateOffset`; the eye moves by `o`, the target by
    /// `o` and then again by `o` times `min(|view - pos| / 280, 1)`.
    pub fn apply(&self, volume: Volume, pos: V4, view: V4) -> (V4, V4) {
        let mut d = vsub(view, pos);
        d[2] = 0;
        let ang = (f64::from(f32::from_bits(d[0])).atan2(f64::from(f32::from_bits(mul(MINUS_ONE, d[1]))))) as f32;
        let m = piney_data::anim::vu_mul(&rot_matrix_z(&unit_matrix(), ang.to_bits()), &self.matrix);
        let o = vscale(apply_matrix(&m, [0, 0, self.offset_z, 0]), 0x3f00_0000);
        let dv = vsub(view, pos);
        let len = piney_data::libm::sqrtf_on(volume, dot(dv, dv));
        let s = if lt(len, 0x438c_0000) { div(len, 0x438c_0000) } else { ONE };
        (vadd(pos, o), vadd(vadd(view, o), vscale(o, s)))
    }

    /// `cameraShockAbsorber()` (main 0x001629b0), at the end of each `cameraMain`:
    /// the strongest held shock raises `vibrateForce` to its table value, taking its
    /// cycle, turn and `vibrateMatrix = Ry(dirc)`; the oscillator steps 0x10000 /
    /// cycle, the offset's z is `force * sinf`, each crossing of 0 keeps a fifth,
    /// under 1 it stops; each held shock counts down and frees its slot at 0.
    pub fn absorb(&mut self) {
        let mut best: Option<(usize, ShockForce)> = None;
        for (i, s) in self.sf.iter().enumerate() {
            if s.power != -1 && best.is_none_or(|(_, b)| b.power < s.power) && s.time > -1 {
                best = Some((i, *s));
            }
        }
        if let Some((i, s)) = best {
            let p = SHAKE_POWER.get(s.power as usize).copied().unwrap_or(0);
            if lt(self.force, p) {
                self.force = p;
                self.cycle = SHAKE_CYCLE.get(s.cycle as usize).copied().unwrap_or(0);
                self.rotate = if s.rot == 0 { -1 } else { i as i32 };
                self.matrix = rot_matrix_y(&unit_matrix(), deg2rad(s.dirc));
            }
        }
        if let Ok(r) = usize::try_from(self.rotate) {
            let d = &mut self.sf[r].dirc;
            *d = d.wrapping_add(0x1000);
            self.matrix = rot_matrix_y(&unit_matrix(), deg2rad(*d));
        }
        let old = self.oscillator;
        // vibrateCycle is 0 until a first shock (the game's division by 0
        // leaves the oscillator anything, and it is reset below).
        if self.cycle != 0 {
            self.oscillator = old.wrapping_add((0x10000 / i32::from(self.cycle)) as i16);
        }
        self.offset_z = mul(self.force, sinf(deg2rad(self.oscillator)));
        if (old >= 0) != (self.oscillator >= 0) {
            self.force = mul(self.force, 0x3e4c_cccd);
        }
        if lt(self.force, ONE) {
            self.force = 0;
            self.oscillator = 0;
        }
        for s in &mut self.sf {
            if s.time > 0 {
                s.time -= 1;
                if s.time <= 0 {
                    s.time = 0;
                    s.power = -1;
                }
            }
        }
    }
}

/// What `calcVel` and `calcPan` (INF main 0x0017a370, 0x0017a4c0) read of
/// the active camera at a 3D sound's call (`ccSeOn3D` 0x00179d90,
/// `ccSeOn3DNote`, `ccSeOn3DLoop`): its eye, view and type. A camera
/// written between two sounds of a frame pans them differently.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ear {
    pub pos: V4,
    pub view: V4,
    pub kind: i32,
}

/// `checkCameraShakeRange(pos)` (main 0x00162f10): `pos` in the active
/// camera's view (`ccCheckCameraDeg(pos, 12288)`) and nearer its eye than
/// 2000, all through `W2PPos`.
pub fn shake_range(cams: &CamList, frame: &dyn Frame, pos: V4) -> bool {
    let cam = cams.active();
    let (p, c, v) = (frame.w2p(pos), frame.w2p(cam.pos), frame.w2p(cam.view));
    let a = rad2deg(atan2f(sub(p[1], c[1]), sub(p[0], c[0])));
    let b = rad2deg(atan2f(sub(v[1], c[1]), sub(v[0], c[0])));
    let d = (12288 + (i32::from(a) - i32::from(b))) as i16;
    if !(d > 0 && i32::from(d) < 2 * 12288) {
        return false;
    }
    let dv = vsub(pos, cam.pos);
    lt(piney_data::libm::sqrtf_on(cams.volume(), dot(dv, dv)), 0x44fa_0000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{ONE, k};

    #[test]
    fn camera_1_answers_its_own_turns_and_the_others_their_line() {
        let t = Cam { rot: [1, 2, 3, 4], rot3: [5, 6, 7, 8], ..Cam::default() };
        let mut l = CamList::new(Volume::Inf, t);
        assert_eq!(
            (l.id(), l.get_rot(CamId::Field, [9; 4]), l.get_rot2(CamId::Field, [9; 4])),
            (CamId::Field, [1, 2, 3, 4], [5, 6, 7, 8])
        );
        // ecam looking 100 along -y from 100 up: level with the ground below,
        // heading 0; y and w from the scratch.
        let e = l.cam_mut(CamId::Event);
        e.pos = [0, 0, k(100.0), ONE];
        e.view = [0, k(-100.0), k(100.0), ONE];
        l.change(CamId::Event);
        assert_eq!(l.get_rot(l.id(), [0, 7, 0, 9]), [0, 7, 0, 9]);
        assert_eq!(l.get_rot2(CamId::Event, [0, 7, 0, 9]), l.get_rot(CamId::Event, [0, 7, 0, 9]));
        assert_eq!(l.active().pos[2], k(100.0));
        assert_eq!(CamId::from_raw(4), None);
        // Entry 0 is tcam, but turned from its line, not its own turn.
        l.change(CamId::Zero);
        assert_eq!(l.active(), l.cam(CamId::Field));
        assert_ne!(l.rot(), [1, 2, 3, 4]);
        assert_eq!(CamId::from_raw(2).map(CamId::raw), Some(2));
    }
}
