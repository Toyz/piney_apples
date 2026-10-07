//! The camera and the 3D view: `ccCam` from an `F_Camera` record
//! (`ccStream::DecodeF_Camera` 0x0014ece0, `ccCam::SetMatrix_PosRotXYZ`
//! 0x001386b0) and `world_screen` (`ccView::SetView` 0x001052c0,
//! `ccSetViewScreenClipMatrix` 0x00101f60), in VU0's and the EE's arithmetic:
//! the matrices are the game's bit for bit. View space is +x right, +y down,
//! +z forward; the projection is in docs/engine/desktop.md ("The 3D draw").

use glam::Mat4;
use piney_data::anim::{const_radians, ee, rot_x_bits_of, rot_y_bits_of, rot_z_bits_of, vu_mul};
use piney_data::libm::{neg, tanf};

use crate::view::{Frame, SCREEN_H, SCREEN_W};

/// A matrix as VU0 stores it: four columns of float bits.
pub type M4 = [[u32; 4]; 4];

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const PI: u32 = 0x4049_0fdb;
/// `SetView`'s degrees-to-radians factor (0x3c8efa35).
const DEG: u32 = 0x3c8e_fa35;

/// `sceVu0UnitMatrix`.
pub const UNIT: M4 = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// `ccCam::Init`'s field of view, 45 degrees.
pub const DEFAULT_FOV: u32 = 0x4234_0000;

/// A camera: position, rotation in radians and the full horizontal field of
/// view in degrees (`ccCam.viewRange`), as float bits; `matrix` is world to
/// view (`ccCam.matrix`, +0x10) as the `SetMatrix_*` call that placed the
/// camera left it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub pos: [u32; 3],
    pub rot: [u32; 3],
    pub fov: u32,
    pub matrix: M4,
}

impl Default for Camera {
    /// At the origin, unturned, `ccCam::Init`'s field of view.
    fn default() -> Self {
        Camera::pos_rot_xyz([0; 3], [0; 3], DEFAULT_FOV)
    }
}

impl Camera {
    /// `ccCam::SetMatrix_PosRotXYZ(pos, rot)` (0x001386b0).
    pub fn pos_rot_xyz(pos: [u32; 3], rot: [u32; 3], fov: u32) -> Self {
        Camera { pos, rot, fov, matrix: pos_rot_matrix(pos, rot, None) }
    }

    /// `ccCam::SetMatrix_PosRotXYZDebug(pos, rot, m)` (0x001385a0), as
    /// `ccStream::PlaySceneMain` places the stream's camera each frame.
    pub fn debug(&self, m: &M4) -> Self {
        Camera { matrix: pos_rot_matrix(self.pos, self.rot, Some(m)), ..*self }
    }

    /// `DecodeF_Camera` on this camera: a record's flag and its eight values
    /// (position, rotation in degrees, unused, fov; bits). The fov changes
    /// only when the record holds one; no record on the disc leaves out a
    /// position or turn (the game would read stack leftovers).
    pub fn decode(&self, flag: u32, v: &[u32; 8]) -> Self {
        let fov = if flag & 0x100 == 0 { v[7] } else { self.fov };
        Camera::pos_rot_xyz([v[0], v[1], v[2]], const_radians([v[3], v[4], v[5]]), fov)
    }
}

/// The unit matrix turned a half turn about x (the Max camera's -Z look,
/// +Y up made the view's +z forward, +y down), times `m` for the debug
/// call, turned about x, y, then z (`sceVu0RotMatrixX/Y/Z`, libvu0's
/// sine), moved to `pos` (`sceVu0TransMatrix`), then `sceVu0InversMatrix`.
fn pos_rot_matrix(pos: [u32; 3], rot: [u32; 3], m: Option<&M4>) -> M4 {
    let turned = rot_x_bits_of(UNIT, PI);
    let turned = m.map_or(turned, |m| vu_mul(&turned, m));
    let mut c = rot_z_bits_of(rot_y_bits_of(rot_x_bits_of(turned, rot[0]), rot[1]), rot[2]);
    for (k, p) in pos.iter().enumerate() {
        c[3][k] = ee::add(c[3][k], *p);
    }
    invert(&c)
}

/// `sceVu0InversMatrix` (0x001107b0) of a rotation and translation: the
/// transposed 3x3, and the translation taken back through it,
/// `0 - ((r0 tx + r1 ty) + r2 tz)`.
fn invert(m: &M4) -> M4 {
    let t = m[3];
    let rows: [[u32; 4]; 3] = std::array::from_fn(|i| [m[0][i], m[1][i], m[2][i], 0]);
    let back: [u32; 3] = std::array::from_fn(|k| {
        let acc = ee::add(ee::mul(rows[0][k], t[0]), ee::mul(rows[1][k], t[1]));
        ee::sub(0, ee::add(acc, ee::mul(rows[2][k], t[2])))
    });
    [rows[0], rows[1], rows[2], [back[0], back[1], back[2], t[3]]]
}

/// The view parameters `world_screen` is built from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    /// The screen centre in GS primitive coordinates.
    pub scx: f32,
    pub scy: f32,
    /// `ccView.ax`, `ay`.
    pub ax: f32,
    pub ay: f32,
    /// `zmin`, `zmax`: the GS Z range; `near`, `far`: `bboxClipMin.w`,
    /// `bboxClipMax.w`.
    pub zmin: f32,
    pub zmax: f32,
    pub near: f32,
    pub far: f32,
}

/// `ccLayer::Init(pri, 0)`'s view, which the desktop's 3D layer (126)
/// keeps: `SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`, `zmin` 1, `zmax`
/// 2^28, near 8, far 2^20.
pub fn default_projection() -> Projection {
    frame_projection(&Frame::DEFAULT)
}

/// The projection of a view `SetFrame` gave `frame`: its centre and aspect,
/// the default Z range and clip distances.
pub fn frame_projection(frame: &Frame) -> Projection {
    let (scx, scy) = frame.centre();
    Projection { scx, scy, ax: frame.ax, ay: frame.ay, zmin: 1.0, zmax: 268_435_456.0, near: 8.0, far: 1_048_576.0 }
}

/// `ccSys.screenAspect` (`ccSystem::SetScreenModeMain` 0x0010ad40):
/// `(1.3333334 * H) / W`.
pub fn screen_aspect() -> u32 {
    ee::div(ee::mul(0x3faa_aaab, ee::from_int(SCREEN_H as i32)), ee::from_int(SCREEN_W as i32))
}

/// `SetView`'s `screenZ`: `W / (2 tan((0.0174533 fov) / 2))`, newlib's
/// `tanf`.
pub fn screen_z(fov: u32) -> u32 {
    let tan = tanf(ee::div(ee::mul(DEG, fov), TWO));
    ee::div(ee::from_int(SCREEN_W as i32), ee::mul(TWO, tan))
}

/// View space to GS primitive coordinates (homogeneous; divide by w =
/// v.z): `ccSetViewScreenClipMatrix`'s view-screen half, Sony's
/// `sceVu0ViewScreenMatrix`, the scale and centre times the screen distance.
pub fn view_screen(fov: u32, p: &Projection) -> M4 {
    let scrz = screen_z(fov);
    let [cx, cy, ax, ay, zmin, zmax, near, far] =
        [p.scx, p.scy, p.ax, p.ay, p.zmin, p.zmax, p.near, p.far].map(f32::to_bits);
    let ay = ee::mul(ay, screen_aspect());
    let depth = ee::add(neg(near), far);
    let az = ee::div(ee::mul(ee::mul(far, near), ee::add(neg(zmin), zmax)), depth);
    // mula.s, then madd.s: the product rounded before the add.
    let cz = ee::div(ee::add(ee::mul(neg(zmax), near), ee::mul(zmin, far)), depth);
    let scale = [[ax, 0, 0, 0], [0, ay, 0, 0], [0, 0, az, 0], [cx, cy, cz, ONE]];
    let distance = [[scrz, 0, 0, 0], [0, scrz, 0, 0], [0, 0, 0, ONE], [0, 0, ONE, 0]];
    vu_mul(&scale, &distance)
}

/// `world_screen` for `cam` through `p`: `SetView`'s `sceVu0MulMatrix` of
/// the view screen and the camera's matrix.
pub fn world_screen(cam: &Camera, p: &Projection) -> M4 {
    vu_mul(&view_screen(cam.fov, p), &cam.matrix)
}

/// A VU0 matrix as glam's, for the draw.
pub fn to_mat4(m: &M4) -> Mat4 {
    Mat4::from_cols_array_2d(&m.map(|c| c.map(f32::from_bits)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The desktop camera (ANM_xddcamer: pos (0, 0, 292609.3125), no
    /// rotation, fov 9.999999) against the points agent-checked in eemu:
    /// (0,0,0) -> (256, 224), (100,0,0) -> (257, 224), (0,100,0) ->
    /// (256, 222.833), (25600,0,0) -> (512, 224) in frame pixels.
    #[test]
    fn desktop_camera_lands_where_the_game_puts_it() {
        let cam = Camera::pos_rot_xyz([0, 0, 292_609.3f32.to_bits()], [0; 3], 9.999_999f32.to_bits());
        let m = to_mat4(&world_screen(&cam, &default_projection()));
        let px = |p: glam::Vec3| {
            let q = m * p.extend(1.0);
            ((q.x / q.w) - 1792.0, (q.y / q.w) - 1824.0)
        };
        let close = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
        let at = |x, y| px(glam::Vec3::new(x, y, 0.0));
        assert!(close(at(0.0, 0.0), (256.0, 224.0)), "{:?}", at(0.0, 0.0));
        assert!(close(at(100.0, 0.0), (257.0, 224.0)));
        assert!(close(at(0.0, 100.0), (256.0, 222.833)), "{:?}", at(0.0, 100.0));
        assert!(close(at(25600.0, 0.0), (512.0, 224.0)));
        assert!((f32::from_bits(screen_z(9.999_999f32.to_bits())) - 2926.094).abs() < 0.01);
    }

    /// The EE's own values for the default view (`SetView` run in eemu,
    /// as `piney_world::camera::VIEW_SCREEN` holds them): scrz 0x441a827a
    /// at fov 45, aspect 0x3f955555, `az` 0x4f00003f, `cz` 0xc4ffe07f.
    #[test]
    fn default_view_screen_is_the_ee_s() {
        assert_eq!(screen_aspect(), 0x3f95_5555);
        assert_eq!(screen_z(DEFAULT_FOV), 0x441a_827a);
        let vs = view_screen(DEFAULT_FOV, &default_projection());
        assert_eq!(vs[0], [0x441a_827a, 0, 0, 0]);
        assert_eq!(vs[1], [0, 0x4434_42e3, 0, 0]);
        assert_eq!(vs[2], [0x4500_0000, 0x44ff_ffff, 0xc4ff_e07f, ONE]);
        assert_eq!(vs[3], [0, 0, 0x4f00_003f, 0]);
    }
}
