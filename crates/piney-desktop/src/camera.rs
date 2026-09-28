//! The camera and the 3D view: `ccCam` from an `F_Camera` record
//! (`ccStream::DecodeF_Camera` 0x0014ece0, `ccCam::SetMatrix_PosRotXYZ`
//! 0x001386b0) and `world_screen` (`ccView::SetView` 0x001052c0 with
//! Sony's view-screen matrix, `ccSetViewScreenClipMatrix` 0x00101f60).
//!
//! View space is +x right, +y down, +z forward. A point `v` in view space
//! lands at GS primitive coordinates
//!
//! ```text
//! X = scrz * ax * v.x / v.z + scx
//! Y = scrz * ay * aspect * v.y / v.z + scy
//! Z = az / v.z + cz          zmax at the near plane, zmin at the far one
//! scrz = W / (2 tan(fov / 2)),  aspect = 1.3333334 * H / W = 7/6
//! ```

use glam::{Mat4, Vec3, Vec4};

use crate::view::{Frame, SCREEN_H, SCREEN_W};

/// A camera: position, rotation (radians) and the full horizontal field of
/// view in degrees (`ccCam.viewRange`, 45 after `ccCam::Init`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub pos: Vec3,
    pub rot: Vec3,
    pub fov: f32,
}

/// `ccCam::Init`'s field of view.
pub const DEFAULT_FOV: f32 = 45.0;

impl Camera {
    /// An `F_Camera` record's eight floats: position, rotation in degrees,
    /// an unused value, the field of view.
    pub fn from_record(v: &[f32; 8]) -> Self {
        let rad = std::f32::consts::PI / 180.0;
        Camera { pos: Vec3::new(v[0], v[1], v[2]), rot: Vec3::new(v[3], v[4], v[5]) * rad, fov: v[7] }
    }

    /// `ccCam.matrix`: world to view, the inverse of
    /// `T(pos) Rz Ry Rx Rx(pi)` (each `sceVu0RotMatrix*` multiplies on the
    /// left; the extra half turn makes the Max camera's -Z look, +Y up into
    /// the view space's +z forward, +y down).
    pub fn world_view(&self) -> Mat4 {
        let c = Mat4::from_translation(self.pos)
            * Mat4::from_rotation_z(self.rot.z)
            * Mat4::from_rotation_y(self.rot.y)
            * Mat4::from_rotation_x(self.rot.x)
            * Mat4::from_rotation_x(std::f32::consts::PI);
        c.inverse()
    }
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

/// `ccSys.screenAspect` (`ccSystem::SetScreenModeMain` 0x0010adfc):
/// `1.3333334 * H / W`.
pub fn screen_aspect() -> f32 {
    1.333_333_4 * SCREEN_H as f32 / SCREEN_W as f32
}

/// `screenZ`: `W / (2 tan(fov * pi / 360))`.
pub fn screen_z(fov: f32) -> f32 {
    SCREEN_W as f32 / (2.0 * (fov * std::f32::consts::PI / 360.0).tan())
}

/// View space to GS primitive coordinates (homogeneous; divide by w = v.z).
pub fn view_screen(fov: f32, p: &Projection) -> Mat4 {
    let scrz = screen_z(fov);
    let az = p.far * p.near * (p.zmax - p.zmin) / (p.far - p.near);
    let cz = (p.zmin * p.far - p.zmax * p.near) / (p.far - p.near);
    Mat4::from_cols(
        Vec4::new(scrz * p.ax, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scrz * p.ay * screen_aspect(), 0.0, 0.0),
        Vec4::new(p.scx, p.scy, cz, 1.0),
        Vec4::new(0.0, 0.0, az, 0.0),
    )
}

/// `world_screen` for `cam` through `p`.
pub fn world_screen(cam: &Camera, p: &Projection) -> Mat4 {
    view_screen(cam.fov, p) * cam.world_view()
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
        let cam = Camera { pos: Vec3::new(0.0, 0.0, 292_609.3), rot: Vec3::ZERO, fov: 9.999_999 };
        let m = world_screen(&cam, &default_projection());
        let px = |p: Vec3| {
            let q = m * p.extend(1.0);
            ((q.x / q.w) - 1792.0, (q.y / q.w) - 1824.0)
        };
        let close = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
        assert!(close(px(Vec3::ZERO), (256.0, 224.0)), "{:?}", px(Vec3::ZERO));
        assert!(close(px(Vec3::new(100.0, 0.0, 0.0)), (257.0, 224.0)));
        assert!(close(px(Vec3::new(0.0, 100.0, 0.0)), (256.0, 222.833)), "{:?}", px(Vec3::new(0.0, 100.0, 0.0)));
        assert!(close(px(Vec3::new(25600.0, 0.0, 0.0)), (512.0, 224.0)));
        assert!((screen_z(9.999_999) - 2926.094).abs() < 0.01);
    }
}
