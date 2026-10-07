//! `ccView`: how a layer maps its coordinates to the GS. The 2D half
//! (`layer_screen`, used by `ccSprite`, `ccKanji`, `ccMask`) is here as
//! [`LayerView`]; the 3D half (the camera and `world_screen`) as [`View`].
//!
//! The frame buffer is 512 x 448 (`ccSystem::SetScreenMode(512, 448, 0)`),
//! drawn around (2048, 2048): XYOFFSET_1 is (0x7000, 0x7200) in 12.4.

use glam::{Mat4, Vec4};
use piney_draw::Scissor;

use crate::camera::{Camera, M4, Projection, default_projection, to_mat4, world_screen};

/// `ccSys.screenW`, `screenH`.
pub const SCREEN_W: u32 = 512;
pub const SCREEN_H: u32 = 448;
/// XYOFFSET_1 in 12.4: `(2048 - 512 / 2) * 16`, `(2048 - 448 / 2) * 16`.
pub const XYOFFSET_X: i32 = 0x7000;
pub const XYOFFSET_Y: i32 = 0x7200;
/// MakePacketStr's cull bounds: a quad is dropped when it lies wholly
/// outside `XYOFFSET .. (0x9000, 0x8e00)`.
pub const CULL_X1: i32 = 0x9000;
pub const CULL_Y1: i32 = 0x8e00;
/// Logical 2D space: `ccView::SetFrame` scales x by W / 512 and y by
/// H / 384.
pub const LOGICAL_W: f32 = 512.0;
pub const LOGICAL_H: f32 = 384.0;

/// The 2D mapping of a view: `ccView::SetFrame` (0x00104ae0) then
/// `SetAspect` / `SetLayerCenter` (0x00104e40 / 0x00105080), which build
/// `layer_screen` from the scissor origin, the aspect and the layer centre.
/// A logical point (x, y) goes to GS 12.4 as
/// `(trunc(x * sx + ox), trunc(y * sy + oy))`, the EE's truncating float
/// to int.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerView {
    pub sx: f32,
    pub sy: f32,
    pub ox: f32,
    pub oy: f32,
    pub scissor: Scissor,
}

/// C's `(float)(unsigned)` of a non-negative float, as `__fixunssfdi` and
/// `fptoui` do: truncation.
fn trunc_u(f: f32) -> i64 {
    if f <= 0.0 { 0 } else { f as i64 }
}

impl LayerView {
    /// `SetFrame(x, y, w, h, cx, cy, ax, ay)` with the layer centre (0, 0)
    /// and no rotation.
    pub fn frame(x: f32, y: f32, w: f32, h: f32, ax: f32, ay: f32) -> Self {
        use crate::eef::{add, div, from_int, mul};
        let xs = div(from_int(SCREEN_W as i32), LOGICAL_W);
        let ys = div(from_int(SCREEN_H as i32), LOGICAL_H);
        let (x, w) = (mul(x, xs), mul(w, xs));
        let (y, h) = (mul(y, ys), mul(h, ys));
        let sx0 = trunc_u(add(0.5, x));
        let sx1 = trunc_u(add(add(0.5, x), w)) - 1;
        let sy0 = trunc_u(add(0.5, y));
        let sy1 = trunc_u(add(add(0.5, y), h)) - 1;
        let scissor = Scissor {
            x0: (sx0 & 0x7ff) as u16,
            x1: (sx1 & 0x7ff) as u16,
            y0: (sy0 & 0x7ff) as u16,
            y1: (sy1 & 0x7ff) as u16,
        };
        Self::aspect(scissor, ax, ay, 0.0, 0.0)
    }

    /// `SetFrame(x, y, w, h, .., ax, ay)` then `SetLayerCenter(fcx, fcy)`:
    /// the frame with its logical origin moved to (fcx, fcy) (the staff
    /// roll's layer, centred at (256, 192)).
    #[allow(clippy::too_many_arguments)]
    pub fn frame_centred(x: f32, y: f32, w: f32, h: f32, ax: f32, ay: f32, fcx: f32, fcy: f32) -> Self {
        let f = Self::frame(x, y, w, h, ax, ay);
        Self::aspect(f.scissor, ax, ay, fcx, fcy)
    }

    /// `SetAspect` / `SetLayerCenter`'s shared body.
    fn aspect(scissor: Scissor, ax: f32, ay: f32, fcx: f32, fcy: f32) -> Self {
        use crate::eef::{add, div, from_int, mul};
        let (w, h) = (SCREEN_W as i32, SCREEN_H as i32);
        let sx = div(mul(from_int(w << 4), ax), 512.0);
        let sy = div(mul(from_int(h << 4), ay), 384.0);
        let ox = add(
            div(from_int(((4096 - w + 2 * i32::from(scissor.x0)) << 4) + 1), 2.0),
            div(mul(mul(16.0, fcx), from_int(w)), 512.0),
        );
        let oy = add(
            div(from_int(((4096 - h + 2 * i32::from(scissor.y0)) << 4) + 1), 2.0),
            div(mul(mul(16.0, fcy), from_int(h)), 384.0),
        );
        LayerView { sx, sy, ox, oy, scissor }
    }

    /// `ccLayer::Init(pri, 0)`'s view: `SetFrame(0, 0, 512, 384, 256, 192,
    /// 1, 1)`, the whole frame with y stretched 448 / 384.
    pub fn default_layer() -> Self {
        Self::frame(0.0, 0.0, LOGICAL_W, LOGICAL_H, 1.0, 1.0)
    }

    /// A logical point in GS 12.4 (`ApplyLayerScreenMatrix`, truncating).
    pub fn apply(&self, x: f32, y: f32) -> (i32, i32) {
        use crate::eef::{add, mul, to_int};
        (to_int(add(mul(x, self.sx), self.ox)), to_int(add(mul(y, self.sy), self.oy)))
    }

    /// GS 12.4 to frame-buffer pixels.
    pub fn pixel(gx: i32, gy: i32) -> (f32, f32) {
        ((gx - XYOFFSET_X) as f32 / 16.0, (gy - XYOFFSET_Y) as f32 / 16.0)
    }
}

/// `ccView::SetFrame(x, y, w, h, cx, cy, ax, ay)`'s arguments: the frame
/// in logical units (512 x 384), its centre from its corner, the aspect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
    pub ax: f32,
    pub ay: f32,
}

// Frames are built from the game's constants, never NaN.
impl Eq for Frame {}

impl Frame {
    /// `ccLayer::Init(pri, 0)`'s: `SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`.
    pub const DEFAULT: Frame =
        Frame { x: 0.0, y: 0.0, w: LOGICAL_W, h: LOGICAL_H, cx: 256.0, cy: 192.0, ax: 1.0, ay: 1.0 };

    /// The 2D mapping and scissor (`layer_screen` with the layer centre at
    /// (0, 0)).
    pub fn layer(&self) -> LayerView {
        LayerView::frame(self.x, self.y, self.w, self.h, self.ax, self.ay)
    }

    /// The frame's corner in frame-buffer pixels around (2048, 2048) before
    /// the half pixel: `x W / 512 + 2048 - W / 2`, `y H / 384 + 2048 - H / 2`.
    fn origin(&self) -> (f32, f32) {
        use crate::eef::{add, div, from_int, mul};
        let xs = div(from_int(SCREEN_W as i32), LOGICAL_W);
        let ys = div(from_int(SCREEN_H as i32), LOGICAL_H);
        let x = add(mul(self.x, xs), 2048.0 - from_int(SCREEN_W as i32 / 2));
        let y = add(mul(self.y, ys), 2048.0 - from_int(SCREEN_H as i32 / 2));
        (x, y)
    }

    /// `scx`, `scy`: the frame's corner plus its centre, in GS pixels.
    pub fn centre(&self) -> (f32, f32) {
        use crate::eef::{add, div, from_int, mul};
        let xs = div(from_int(SCREEN_W as i32), LOGICAL_W);
        let ys = div(from_int(SCREEN_H as i32), LOGICAL_H);
        let (x, y) = self.origin();
        (add(x, mul(self.cx, xs)), add(y, mul(self.cy, ys)))
    }

    /// `bboxClipMin` / `bboxClipMax` x and y: the frame's corners in GS
    /// pixels, rounded (`fptoui(0.5 + v)`).
    pub fn bbox(&self) -> ([f32; 2], [f32; 2]) {
        use crate::eef::{add, div, from_int, mul};
        let xs = div(from_int(SCREEN_W as i32), LOGICAL_W);
        let ys = div(from_int(SCREEN_H as i32), LOGICAL_H);
        let (x, y) = self.origin();
        let r = |v: f32| trunc_u(add(0.5, v)) as f32;
        ([r(x), r(y)], [r(add(x, mul(self.w, xs))), r(add(y, mul(self.h, ys)))])
    }
}

/// The 3D half of a view: the camera `SetView` last gave the layer's view,
/// its projection, and the `world_screen` `SetView` made of them.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    projection: Projection,
    camera: Camera,
    world_screen: M4,
}

impl Default for View {
    fn default() -> Self {
        View::new(default_projection(), Camera::default())
    }
}

impl View {
    /// A view of `projection` that `SetView` gave `camera`.
    pub fn new(projection: Projection, camera: Camera) -> Self {
        View { projection, camera, world_screen: world_screen(&camera, &projection) }
    }

    /// `ccView::SetView(view, cam, 0)`.
    pub fn set_camera(&mut self, cam: &Camera) {
        *self = View::new(self.projection, *cam);
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn projection(&self) -> &Projection {
        &self.projection
    }

    /// `ccView.world_screen` (+0xd0) as `SetView` leaves it.
    pub fn world_screen_bits(&self) -> &M4 {
        &self.world_screen
    }

    pub fn world_screen(&self) -> Mat4 {
        to_mat4(&self.world_screen)
    }

    /// `world_screen * local_world` with the XYOFFSET taken off: a model
    /// draw's `to_screen`, in frame-buffer pixels after the divide.
    pub fn to_screen(&self, world: Mat4) -> Mat4 {
        // (x - ox * w, y - oy * w, z, w): after the divide, x / w - ox.
        let (ox, oy) = (XYOFFSET_X as f32 / 16.0, XYOFFSET_Y as f32 / 16.0);
        let off = Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::Z, Vec4::new(-ox, -oy, 0.0, 1.0));
        off * self.world_screen() * world
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layer_matches_the_game() {
        // Values agent-checked in eemu: ls00 16.0, ls11 0x41955555, ls30
        // 28672.5, ls31 29184.5, scissor 0..511 x 0..447.
        let v = LayerView::default_layer();
        assert_eq!(v.sx, 16.0);
        assert_eq!(v.sy.to_bits(), 0x4195_5555);
        assert_eq!(v.ox, 28672.5);
        assert_eq!(v.oy, 29184.5);
        assert_eq!(v.scissor, Scissor::FULL);
    }

    /// Stream 6's letterbox, `SetFrame(0, 48, 512, 288, 256, 192, 0.75,
    /// 0.75)`: the centre 56 rows below the default's (one ulp short, the
    /// EE truncating), frame rows 56-391, the clip box and scissor as the
    /// game's.
    #[test]
    fn letterbox_frame() {
        let f = Frame { x: 0.0, y: 48.0, w: 512.0, h: 288.0, cx: 256.0, cy: 192.0, ax: 0.75, ay: 0.75 };
        assert_eq!(f.centre().0, 2048.0);
        assert_eq!(f.centre().1.to_bits(), 0x4503_7fff);
        assert_eq!(f.bbox(), ([1792.0, 1880.0], [2304.0, 2216.0]));
        assert_eq!(f.layer().scissor, Scissor { x0: 0, x1: 511, y0: 56, y1: 391 });
        // Agent-checked in eemu: the game's own SetFrame leaves these, and
        // the default's scy one ulp under 2048.
        assert_eq!(Frame::DEFAULT.centre().1.to_bits(), 0x44ff_ffff);
    }

    #[test]
    fn mail_view_is_one_to_one() {
        // MailList_control's SetFrame(90, 270, 224, 80, 112, 40, 1, 6/7).
        let v = LayerView::frame(90.0, 270.0, 224.0, 80.0, 1.0, 6.0 / 7.0);
        assert_eq!(v.scissor, Scissor { x0: 90, x1: 313, y0: 315, y1: 407 });
        assert_eq!(v.sx, 16.0);
        let (px, py) = LayerView::pixel(v.apply(0.0, 0.0).0, v.apply(0.0, 0.0).1);
        assert_eq!((px.floor(), py.floor()), (90.0, 315.0));
    }
}
