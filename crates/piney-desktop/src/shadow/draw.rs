//! `ccShadowModel::Draw` (main 0x001412b0) up to its packet, in the EE's and
//! VU0's arithmetic: the light in the model's axes and the stretch, the box
//! check (`_ccCheckBoundingBoxEx` with `mc0_CheckBoundingBoxShadow`), the
//! matrix to the buffer, the near point, the scissor and guard band, and the
//! two blends by the packet's mode: the [`Setup`] VU1 gets at 0x184-0x194.

use piney_data::field::ee;
use piney_data::model::VertexBox;
use piney_data::shadow::ShadowMesh;
use piney_data::vu0::{self, F, M4, ONE, V4};
use piney_data::vu1::sign;

const MINUS_ONE: F = 0xbf80_0000;
const HALF: F = 0x3f00_0000;
const TWO: F = 0x4000_0000;
const F8: F = 0x4100_0000;
const F4096: F = 0x4580_0000;
/// The guard band's far corner, 4095.75 (`lui 0x457f; ori 0xfc00`).
const GUARD: F = 0x457f_fc00;
/// `ccLayer::Init`'s +0x200 and +0x204, 4095.0.
const LAYER_EDGE: F = 0x457f_f000;

/// What `ccShadowModel::Draw` reads of its `ccView`, as float bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowView {
    /// +0xd0 (world to GS pixels) and +0x90 (view to GS pixels).
    pub world_screen: M4,
    pub view_screen: M4,
    /// +0x1dc and +0x1ec, the near and far w.
    pub near: F,
    pub far: F,
    /// `GetScreenClip`, GS pixels (+0x1d0, +0x1d4, +0x1e0, +0x1e4).
    pub clip: [F; 4],
    /// XYOFFSET in pixels: the clip less it is the frame's rectangle.
    pub offset: [f32; 2],
}

/// The boxes `_ccCheckBoundingBoxEx` loads from the view (vf13 to vf16).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    /// +0x1f0 and +0x200: `ccLayer::Init`'s (0, 0, 0, 8) and (4095, 4095,
    /// 0, far).
    guard: [V4; 2],
    /// +0x1d0 and +0x1e0: the clip with the near and far w.
    clip: [V4; 2],
}

impl ShadowView {
    fn bounds(&self) -> Bounds {
        let c = self.clip;
        Bounds {
            guard: [[0, 0, 0, F8], [LAYER_EDGE, LAYER_EDGE, 0, self.far]],
            clip: [[c[0], c[1], 0, self.near], [c[2], c[3], 0, self.far]],
        }
    }
}

/// `ccShadowPacket` +0x15c: how the buffer counts the volumes. Every packet
/// the game makes counts (1); the code has two more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Front faces add 1 to red, back faces take 1 away.
    Count,
    /// Front faces into red, back faces into green, both added.
    Colours,
    /// The model flat, front faces opaque, unstretched.
    Flat,
}

impl Mode {
    /// The byte's mode; None for 0 (no shadows) and any the code has no
    /// registers for.
    pub fn of(byte: u8) -> Option<Mode> {
        match byte {
            1 => Some(Mode::Count),
            2 => Some(Mode::Colours),
            3 => Some(Mode::Flat),
            _ => None,
        }
    }
}

/// What `ccShadowModel::Draw` takes from the model, the object, the draw
/// environment and the shadow packet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The model's vertex scale (`ccModel` +0x0c).
    pub scale: F,
    /// The object's world matrix (`ccDrawModelParam` +0x90).
    pub local_world: M4,
    /// `ccDrawEnv` +0x90, the light's direction, and +0xa0, the length.
    pub light: V4,
    pub length: F,
    pub mode: Mode,
    /// The buffer, `1 << tw` by `1 << th`.
    pub tw: u8,
    pub th: u8,
}

/// The colour (RGBAQ) and blend (ALPHA_2) the GS draws a polygon with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blend {
    pub colour: u64,
    pub alpha: u64,
}

/// What `ccShadowModel::Draw` sends VU1 for a draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setup {
    /// The scissor: (-x0, -y0, 0, -1) and (x1, y1, 0, 1).
    pub scissor: [V4; 2],
    /// The near plane's point, `B P (0, 0, max(near, 8), 1)`.
    pub near: V4,
    /// The guard band: (0, 0, 0, near) and (4095.75, 4095.75, 0, far).
    pub guard: [V4; 2],
    /// `B * world_screen * local_world * S`.
    pub m: M4,
    /// For a polygon wound clockwise on screen (a negative cross product),
    /// and for one wound the other way.
    pub blends: [Blend; 2],
    /// The light in the model's axes, and the stretch.
    pub light: V4,
    pub stretch: V4,
    /// The buffer's XYOFFSET in sixteenths, `(4096 - w) / 2` pixels.
    pub offset: [i32; 2],
}

impl Setup {
    /// The blend a polygon of this winding takes.
    pub fn blend(&self, winding: F) -> Blend {
        if sign(winding) { self.blends[0] } else { self.blends[1] }
    }
}

/// The box a shadow model's `ccModel` +4 points at: `DecodeShadowModel`'s
/// extremes through `ccBbox_SetBox` ([`VertexBox`]), its least and
/// greatest corners.
pub fn model_box(mesh: &ShadowMesh, scale: F) -> [V4; 2] {
    let b = VertexBox::of(&mesh.positions, scale);
    [b.min, b.max]
}

/// VU0's vf21 as a box check finds it. The shadow's reads its x, y and z
/// (`mc0_CheckBoundingBoxShadow`'s `add.w` sets only w); each check leaves
/// its matrix times its last corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vf21 {
    /// A vector the port has: a check's last corner, or VU0's reset zero.
    Known(V4),
    /// What `ccCoord::_SetLWMatrix` (0x00138380) leaves: the qword of VU0's
    /// stack above the topmost changed coord, vf0 at the root, else that
    /// coord's parent's address over `$a0`'s sign and upper half (the heap's
    /// and the drawing thread's bits, which the port reads as 0;
    /// docs/engine/shadow.md).
    Chain,
}

impl Vf21 {
    fn xyz(self) -> V4 {
        match self {
            Vf21::Known(v) => v,
            Vf21::Chain => [0, 0, 0, ONE],
        }
    }
}

/// What an object's `ccObj::Draw` (0x0013f220) does to vf21 before its
/// shadow model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Prelude {
    /// It has a parent, so `_SetLWMatrix` runs (else `sceVu0CopyMatrix`).
    pub parent: bool,
    /// `ccModel::Draw` checks its model's box (`mc0_CheckBoundingBox`): the
    /// box's greatest corner ([`crate::assets::ModelInfo::checked_max`]).
    pub checked: Option<V4>,
}

impl Prelude {
    /// vf21 after it, from `before`, the object at `local_world` in `view`.
    pub fn vf21(self, before: Vf21, view: &ShadowView, local_world: &M4) -> Vf21 {
        match (self.checked, self.parent) {
            (Some(max), _) => Vf21::Known(vu0::apply(&vu0::mul_matrix(&view.world_screen, local_world), max)),
            (None, true) => Vf21::Chain,
            (None, false) => before,
        }
    }
}

/// What `_ccCheckBoundingBoxEx` returns (vi01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seen {
    /// No corner within one side of the clip box: culled (0).
    Culled,
    /// Every corner within the guard box (1).
    Inside,
    /// Otherwise (2).
    Partly,
}

/// `mc0_CheckBoundingBoxShadow` (VU0 0x000) through `_ccCheckBoundingBoxEx`
/// (0x0013e9e0): 16 corners through `m`. The first eight are the box's;
/// the program's `add.w` sets only w, so the other eight are built from the
/// reach itself and from `vf21`'s x, y, z. It leaves `vf21` `m` times its
/// last corner.
fn check_box(m: &M4, bounds: Bounds, [lo, hi]: [V4; 2], reach: V4, vf21: &mut Vf21) -> Seen {
    let corner = |k: usize, lo: V4, hi: V4| -> V4 {
        let pick = |bit: usize, i: usize| if k & bit == 0 { lo[i] } else { hi[i] };
        [pick(1, 0), pick(2, 1), pick(4, 2), ONE]
    };
    let stale = vf21.xyz();
    // A side as the program tests it: its x and y scaled by the corner's w,
    // its w plus 0 (`MULw.xy`, `ADDx.w`).
    let side = |b: V4, w: F| [ee::mul(b[0], w), ee::mul(b[1], w), ee::add(b[3], 0)];
    // The MAC flag's x, y and w sign bits of `a - b` (`SUBA.xyw`).
    let below = |a: [F; 3], b: [F; 3]| -> u16 {
        let s = |k: usize| u16::from(sign(ee::sub(a[k], b[k])));
        s(0) << 7 | s(1) << 6 | s(2) << 4
    };
    let (mut inside, mut seen) = (0xd0u16, 0u16);
    for k in 0..16 {
        let c = if k < 8 { corner(k, lo, hi) } else { corner(k - 8, reach, stale) };
        let v = vu0::apply(m, c);
        let p = [v[0], v[1], v[3]];
        let [g0, g1, c0, c1] =
            [bounds.guard[0], bounds.guard[1], bounds.clip[0], bounds.clip[1]].map(|b| side(b, v[3]));
        inside &= below(g0, p) & below(p, g1);
        seen |= below(c0, p) | below(p, c1) << 4;
        *vf21 = Vf21::Known(v);
    }
    match (seen, inside) {
        (s, _) if s != 0xdd0 => Seen::Culled,
        (_, 0xd0) => Seen::Inside,
        _ => Seen::Partly,
    }
}

/// `ccShadowModel::Draw` up to the packet. None when the box check culls
/// it. `alpha` is `alphaBlendTbl`; `vf21` VU0's, as the object's
/// `ccObj::Draw` left it ([`Prelude`]).
pub fn draw(mesh: &ShadowMesh, view: &ShadowView, p: &Params, alpha: &[u64], vf21: &mut Vf21) -> Option<Setup> {
    let diag = |s: F| [[s, 0, 0, 0], [0, s, 0, 0], [0, 0, s, 0], [0, 0, 0, ONE]];
    let lw = &p.local_world;
    // sceVu0InversMatrix less its translation: the light in the model's
    // axes, then (over the scale) the stretch, w 0.
    let mut r = vu0::invers(lw);
    r[3] = [0, 0, 0, r[3][3]];
    let light = vu0::apply(&r, p.light);
    let stretch = match p.mode {
        Mode::Flat => [0; 4],
        Mode::Count | Mode::Colours => {
            let e = vu0::apply(&vu0::mul_matrix(&r, &diag(ee::div(ONE, p.scale))), p.light);
            let [x, y, z, _] = vu0::vscale(e, p.length);
            [x, y, z, 0]
        }
    };
    let screen = vu0::mul_matrix(&view.world_screen, lw);
    let reach = vu0::vscale(light, p.length);
    if check_box(&screen, view.bounds(), model_box(mesh, p.scale), reach, vf21) == Seen::Culled {
        return None;
    }
    // The buffer: the clip box's corner to the origin (`neg.s`), scaled to
    // the buffer, (4096 - w) / 2 over.
    let c = view.clip;
    let (w, h) = (ee::from_int(1 << p.tw), ee::from_int(1 << p.th));
    let mut to_corner = diag(ONE);
    to_corner[3] = [c[0] ^ 0x8000_0000, c[1] ^ 0x8000_0000, 0, ONE];
    let mut to_buffer = diag(ONE);
    to_buffer[0][0] = ee::div(w, ee::sub(c[2], c[0]));
    to_buffer[1][1] = ee::div(h, ee::sub(c[3], c[1]));
    to_buffer[3] = [ee::div(ee::sub(F4096, w), TWO), ee::div(ee::sub(F4096, h), TWO), 0, ONE];
    let b = vu0::mul_matrix(&to_buffer, &to_corner);
    let m = vu0::mul_matrix(&b, &vu0::mul_matrix(&screen, &diag(p.scale)));
    let near_w = if ee::lt(view.near, F8) { F8 } else { view.near };
    let near = vu0::apply(&vu0::mul_matrix(&b, &view.view_screen), [0, 0, near_w, ONE]);
    let lo = |s: F| ee::mul(HALF, ee::sub(F4096, s));
    let hi = |s: F| ee::mul(HALF, ee::add(F4096, s));
    let scissor = [[ee::mul(MINUS_ONE, lo(w)), ee::mul(MINUS_ONE, lo(h)), 0, MINUS_ONE], [hi(w), hi(h), 0, ONE]];
    let table = |i: usize| alpha.get(i).copied().unwrap_or(0);
    let blend = |colour, alpha| Blend { colour, alpha };
    let blends = match p.mode {
        Mode::Count => [blend(0x8000_0001, table(1)), blend(0x8000_0001, table(2))],
        Mode::Colours => [blend(0x8000_0040, table(1)), blend(0x8000_4000, table(1))],
        Mode::Flat => [blend(0x8000_00ff, table(0)), blend(0x8000_0000, table(1))],
    };
    // A world matrix that mirrors swaps the two.
    let mirrored = ee::lt(vu0::dot(vu0::cross(lw[0], lw[1]), lw[2]), 0);
    let edge = |s: u8| ((4096 - (1i32 << s)) / 2) << 4;
    Some(Setup {
        scissor,
        near,
        guard: [[0, 0, 0, view.near], [GUARD, GUARD, 0, view.far]],
        m,
        blends: if mirrored { [blends[1], blends[0]] } else { blends },
        light,
        stretch,
        offset: [edge(p.tw), edge(p.th)],
    })
}
