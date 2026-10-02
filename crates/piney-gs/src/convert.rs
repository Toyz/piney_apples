//! Commands to triangles: what the GS does to a primitive before its pixel
//! pipeline, done on the CPU. The pixel pipeline itself (texture function,
//! alpha test, blend) is the shader's and the pipeline state's.

use glam::{Mat3, Mat4, Vec3, Vec4};
use piney_data::model::{self, Kind, Model};
use piney_draw::{AlphaFail, AlphaTest, Blend, BlendAlpha, BlendColour, Prim, PrimKind, Rgba, TexFunc, TexRef, ZTest};

/// One vertex as the shader takes it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GVertex {
    /// Clip space. Models keep their w, so texturing is perspective-correct
    /// as the GS's STQ is; 2D has w = 1.
    pub clip: [f32; 4],
    /// Normalised texture coordinates.
    pub uv: [f32; 2],
    /// The GS colour, 0-255 per channel (0x80 = 1.0 under MODULATE).
    pub colour: [f32; 4],
    /// [`Params`] bits.
    pub params: u32,
    /// The fog: F in bits 24-31, FOGCOL in 0-23 (red low), when
    /// [`Params::FOG`] is set.
    pub fog: u32,
    /// With [`Params::DEPTH_FOG`]: this vertex's F (VU1's fog by depth),
    /// which the shader interpolates in place of the word's.
    pub fog_f: f32,
    /// With [`Params::REGION`]: REGION_REPEAT's MINU/MINV in bits 0-15
    /// and MAXU/MAXV in 16-31 (the same both ways). With
    /// [`Params::SCALED`]: the smaller copy's width in bits 0-15, its
    /// height in 16-31.
    pub region: u32,
}

/// The frame's size in pixels, and how many frame-buffer pixels make one of
/// its pixels each way ([`crate::Gs::set_scale`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

/// The GS draws a pixel when its top-left corner lies inside a primitive;
/// wgpu when its centre does. Moving everything half a frame-buffer pixel
/// right and down (half a pixel of the frame at scale 1) makes the two agree.
pub const PIXEL_CENTRE: f32 = 0.5;

/// The GS samples texel `floor(u)` at a pixel; a sprite's edge often sits
/// exactly on a texel boundary (u = 16.0), which interpolation on the GPU
/// can leave a hair below (15.99999), picking the neighbouring texel - a
/// column of the next glyph or icon in the atlas. The GS's UV has 1/16-texel
/// steps, so nudging every coordinate by 1/512 texel keeps boundary values in
/// the texel the GS picks without moving any other.
pub const TEXEL_BIAS: f32 = 1.0 / 512.0;

/// GS Z is 32 bits, larger nearer. Depth is kept the same way round -
/// `z / 2^32`, cleared to 0, larger passing - because floats are densest
/// near 0: the desktop's and the title's camera put everything at GS Z of a
/// few thousand, which `1 - z / 2^32` would squeeze into the last few float
/// steps below 1.
fn depth(z: f64) -> f64 {
    (z / 4_294_967_296.0).clamp(0.0, 1.0)
}

impl Screen {
    /// A frame-buffer point with GS depth `z` to clip space, at w = 1.
    pub fn clip_2d(&self, x: f32, y: f32, z: u32) -> [f32; 4] {
        [
            (x + PIXEL_CENTRE / self.scale) / (self.width / 2.0) - 1.0,
            1.0 - (y + PIXEL_CENTRE / self.scale) / (self.height / 2.0),
            depth(f64::from(z)) as f32,
            1.0,
        ]
    }

    /// A homogeneous frame-buffer point (pixel = q.xy / q.w, GS depth =
    /// q.z / q.w) to clip space, keeping w.
    pub fn clip_3d(&self, q: Vec4) -> [f32; 4] {
        let w = f64::from(q.w);
        [
            ((f64::from(q.x) + f64::from(PIXEL_CENTRE / self.scale) * w) / f64::from(self.width / 2.0) - w) as f32,
            (w - (f64::from(q.y) + f64::from(PIXEL_CENTRE / self.scale) * w) / f64::from(self.height / 2.0)) as f32,
            (w * depth(f64::from(q.z) / w)) as f32,
            q.w,
        ]
    }
}

/// The per-command pixel state the shader reads, packed into one word:
/// bits 0-1 TFX, 2 TCC, 3 textured, 4 alpha test on, 5-7 its method, 8-15
/// its reference, 16-17 AFAIL, 18 fog, 19 passing pixels discarded, 20 the
/// fog's F each vertex's own, 21 REGION_REPEAT, 22 a scaled copy of the
/// frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Params(pub u32);

impl Params {
    /// PRIM.FGE: the fog stage runs.
    pub const FOG: u32 = 1 << 18;
    /// F from [`GVertex::fog_f`], interpolated.
    pub const DEPTH_FOG: u32 = 1 << 20;
    /// The texel at `(texel & MIN) | MAX` of [`GVertex::region`], unfiltered.
    pub const REGION: u32 = 1 << 21;
    /// The texture is the frame at full size, read as the smaller copy
    /// [`TexRef::ScaledFrame`] of [`GVertex::region`]'s size would be.
    pub const SCALED: u32 = 1 << 22;

    /// The same, with a failing alpha test discarding the pixel: the first
    /// pass of an FB_ONLY draw, colour and Z.
    pub fn fail_discards(self) -> Params {
        Params(self.0 & !(3 << 16))
    }

    /// The same, keeping only the pixels that fail the alpha test: the
    /// second pass of an FB_ONLY draw, colour only.
    pub fn pass_discards(self) -> Params {
        Params(self.0 | 1 << 19)
    }

    pub fn new(texture: Option<(TexFunc, bool)>, test: AlphaTest) -> Params {
        let mut p = 0;
        if let Some((func, use_alpha)) = texture {
            p |= func as u32 | u32::from(use_alpha) << 2 | 1 << 3;
        }
        if let AlphaTest::On { method, reference, fail } = test {
            p |= 1 << 4 | (method as u32) << 5 | u32::from(reference) << 8 | (fail as u32) << 16;
        }
        Params(p)
    }
}

fn colour(c: Rgba) -> [f32; 4] {
    c.0.map(f32::from)
}

/// A primitive's triangles. `tex_size` normalises its texel coordinates.
pub fn prim(p: &Prim, tex_size: Option<(u32, u32)>, screen: Screen, params: Params) -> Vec<GVertex> {
    let (tw, th) = tex_size.map_or((1.0, 1.0), |(w, h)| (w as f32, h as f32));
    let (params, region) = match p.state.texture.as_ref().map(|t| (&t.tex, t.wrap)) {
        Some((&TexRef::ScaledFrame { width, height }, _)) if tex_size.is_some() => {
            (Params(params.0 | Params::SCALED), u32::from(width) | u32::from(height) << 16)
        }
        Some((_, piney_draw::Wrap::Region { mask, fix })) => {
            (Params(params.0 | Params::REGION), u32::from(mask) | u32::from(fix) << 16)
        }
        _ => (params, 0),
    };
    let vertex = |x: f32, y: f32, z: u32, u: f32, v: f32, c: Rgba| GVertex {
        clip: screen.clip_2d(x, y, z),
        uv: [(u + TEXEL_BIAS) / tw, (v + TEXEL_BIAS) / th],
        colour: colour(c),
        params: params.0,
        fog: 0,
        fog_f: 0.0,
        region,
    };
    let mut out = Vec::new();
    let vs = &p.verts;
    if p.kind == PrimKind::Sprite {
        for pair in vs.as_chunks::<2>().0 {
            let (a, b) = (pair[0], pair[1]);
            // A sprite takes the second vertex's colour and Z.
            let tl = vertex(a.x, a.y, b.z, a.u, a.v, b.rgba);
            let tr = vertex(b.x, a.y, b.z, b.u, a.v, b.rgba);
            let bl = vertex(a.x, b.y, b.z, a.u, b.v, b.rgba);
            let br = vertex(b.x, b.y, b.z, b.u, b.v, b.rgba);
            out.extend([tl, tr, bl, tr, br, bl]);
        }
        return out;
    }
    let tris: Vec<[usize; 3]> = match p.kind {
        PrimKind::Triangles => (0..vs.len() / 3).map(|t| [3 * t, 3 * t + 1, 3 * t + 2]).collect(),
        PrimKind::Strip => (2..vs.len()).map(|i| [i - 2, i - 1, i]).collect(),
        PrimKind::Fan => (2..vs.len()).map(|i| [0, i - 1, i]).collect(),
        PrimKind::Sprite => unreachable!(),
    };
    for t in tris {
        // Flat shading fills a triangle with the colour of the vertex that
        // completed it.
        let flat = vs[t[2]].rgba;
        for i in t {
            let v = vs[i];
            out.push(vertex(v.x, v.y, v.z, v.u, v.v, if p.gouraud { v.rgba } else { flat }));
        }
    }
    out
}

/// The GS's primitive coordinate space, 0..4096 in each axis before the
/// offset. The frame buffer sits in its middle (XYOFFSET 2048 - size / 2).
const PRIM_SPACE: f32 = 4096.0;

/// The view's w range: `ccLayer::Init` (0x00108260) sets `clipMin.w` and
/// `bboxClipMin.w` to 8, `clipMax.w` and `bboxClipMax.w` to 2^20; VU1 takes a
/// vertex outside it as outside the view, and `mc_ScissorTriPolyXYZ` cuts at
/// the near one.
pub const NEAR_W: f32 = 8.0;
pub const FAR_W: f32 = 1_048_576.0;

/// One vertex of a triangle being clipped: its homogeneous frame-buffer
/// position, texture coordinates and colour, all linear in clip space.
#[derive(Clone, Copy)]
struct ClipVertex {
    q: Vec4,
    uv: [f32; 2],
    colour: [f32; 4],
}

impl ClipVertex {
    fn lerp(self, o: ClipVertex, t: f32) -> ClipVertex {
        let l = |a: f32, b: f32| a + (b - a) * t;
        ClipVertex {
            q: self.q + (o.q - self.q) * t,
            uv: [l(self.uv[0], o.uv[0]), l(self.uv[1], o.uv[1])],
            colour: std::array::from_fn(|k| l(self.colour[k], o.colour[k])),
        }
    }
}

/// A triangle cut to the side of the near plane in front of the camera
/// (Sutherland-Hodgman in homogeneous space, as `mc_ScissorTriPoly` cuts
/// against each plane): the polygon left, 0, 3 or 4 vertices.
fn clip_near(tri: [ClipVertex; 3]) -> Vec<ClipVertex> {
    let mut out = Vec::with_capacity(4);
    for k in 0..3 {
        let (a, b) = (tri[k], tri[(k + 1) % 3]);
        let (ina, inb) = (a.q.w >= NEAR_W, b.q.w >= NEAR_W);
        if ina {
            out.push(a);
        }
        if ina != inb {
            out.push(a.lerp(b, (NEAR_W - a.q.w) / (b.q.w - a.q.w)));
        }
    }
    out
}

/// The fog word of [`GVertex::fog`].
pub fn fog_word(fog: Option<piney_draw::Fog>) -> u32 {
    fog.map_or(0, |f| {
        u32::from(f.f) << 24 | u32::from(f.colour[2]) << 16 | u32::from(f.colour[1]) << 8 | u32::from(f.colour[0])
    })
}

/// One mmat of a model drawn through VU1: its triangles. With
/// `reject_outside` (the lit and skinned programs) a triangle with a vertex
/// outside the GS space or behind the camera is dropped whole; without it, as
/// `mc_DrawTriSFast` (0x0fe), one not wholly inside is cut at [`NEAR_W`] when
/// its last strip vertex has w below `div_z` ([`piney_draw::ModelDraw::div_z`])
/// and dropped otherwise. Vertex alpha is VU1's (`trunc(A * t)` unlit,
/// `trunc(0x80 * t)` lit); texture coordinates `(S + row) / 256`. `morphed`
/// and `colours` are what the EE wrote over the stored ones.
#[allow(clippy::too_many_arguments)]
pub fn mmat(
    m: &Model,
    draw: &piney_draw::MmatDraw,
    morphed: Option<&[[i16; 3]]>,
    colours_edited: Option<&[[u8; 4]]>,
    to_screen: Mat4,
    nodes: &[Mat4],
    lights: Option<&piney_draw::Lights>,
    reject_outside: bool,
    div_z: f32,
    screen: Screen,
    params: Params,
    fog: u32,
    depth_fog: Option<&piney_draw::DepthFog>,
) -> Vec<GVertex> {
    let Some(mm) = m.mmats.get(draw.index as usize) else { return Vec::new() };
    if mm.kind == Kind::Shadow || mm.triangles.is_empty() {
        return Vec::new();
    }
    let node = |slot: u32| nodes.get(slot as usize).copied().unwrap_or(to_screen);
    // The bone and skin programs light a vertex through its bone's matrix
    // (`mc03m`: the light matrix times the bone's, from `DrawBoneType`'s
    // inverse(object) x bone, its rows renormalised): a normal in its
    // bone's space turned into the object's, where the lights are.
    let to_object = if lights.is_some() && mm.kind != Kind::Rigid { to_screen.inverse() } else { Mat4::IDENTITY };
    let bone = |slot: u32| Mat3::from_mat4(to_object * node(slot));
    // Each vertex's homogeneous frame-buffer position and model-space normal.
    let (qs, normals): (Vec<Vec4>, Vec<Vec3>) = match mm.kind {
        Kind::Skin => mm
            .skin
            .iter()
            .map(|entries| {
                let (mut q, mut n) = (Vec4::ZERO, Vec3::ZERO);
                for e in entries {
                    let w = e.weight as f32 / 256.0;
                    q += w * node(u32::from(e.slot)) * m.position(e.position).extend(1.0);
                    let normal = model::normal(e.normal);
                    n += w * if lights.is_some() { bone(u32::from(e.slot)) * normal } else { normal };
                }
                (q, n.normalize_or_zero())
            })
            .unzip(),
        _ => {
            let mat = match mm.kind {
                Kind::Bone => node(mm.slot.unwrap_or(0)),
                _ => to_screen,
            };
            let turn = (lights.is_some() && mm.kind == Kind::Bone).then(|| bone(mm.slot.unwrap_or(0)));
            let q = morphed.unwrap_or(&mm.positions).iter().map(|&p| mat * m.position(p).extend(1.0)).collect();
            let n = (0..mm.positions.len()).map(|i| {
                let n = mm.normals.get(i).map_or(Vec3::ZERO, |&n| model::normal(n));
                turn.map_or(n, |t| (t * n).normalize_or_zero())
            });
            (q, n.collect())
        }
    };
    // GS Z is 16 z / w (VU1's ftoi4).
    let qs: Vec<Vec4> = qs.into_iter().map(|q| Vec4::new(q.x, q.y, q.z * piney_draw::MODEL_Z_SCALE, q.w)).collect();
    let t = draw.alpha;
    let colours: Vec<[f32; 4]> = (0..qs.len())
        .map(|i| match lights {
            Some(l) => {
                let n = normals[i];
                let mut c = Vec3::from(l.ambient);
                // VU1 dots the raw s8 normal (length 64) with the direction
                // and scales the ambient by 128: against the unit normal, the
                // doubled colours count half.
                for (dir, col) in l.dirs.iter().zip(&l.colours) {
                    c += Vec3::from(*col) * 0.5 * Vec3::from(*dir).dot(n).max(0.0);
                }
                let c = (c * 128.0).min(Vec3::splat(128.0));
                [c.x, c.y, c.z, (128.0 * t).trunc()]
            }
            None => match colours_edited.unwrap_or(&mm.colours).get(i) {
                Some(c) => [f32::from(c[0]), f32::from(c[1]), f32::from(c[2]), (f32::from(c[3]) * t).trunc()],
                None => [128.0, 128.0, 128.0, (128.0 * t).trunc()],
            },
        })
        .collect();
    let half = Vec3::new(screen.width / 2.0, screen.height / 2.0, 0.0);
    let in_space = |q: Vec4| {
        let (x, y) = (q.x / q.w + PRIM_SPACE / 2.0 - half.x, q.y / q.w + PRIM_SPACE / 2.0 - half.y);
        (0.0..PRIM_SPACE).contains(&x) && (0.0..PRIM_SPACE).contains(&y)
    };
    // The lit and skinned programs' test (`reject_outside`), and VU1's own
    // (`clipMin` < x, y, w < `clipMax`) for the unlit one.
    let inside = |q: Vec4| {
        if reject_outside { q.w > 0.0 && in_space(q) } else { q.w > NEAR_W && q.w < FAR_W && in_space(q) }
    };
    let row = [f32::from(draw.uv_row[0]) / 256.0, f32::from(draw.uv_row[1]) / 256.0];
    let fog_f = |q: Vec4| depth_fog.map_or(0.0, |d| d.at(q.w));
    let mut out = Vec::with_capacity(mm.triangles.len() * 3);
    let vertex = |i: usize| ClipVertex {
        q: qs[i],
        uv: mm.uvs.get(i).map_or(row, |&st| {
            let uv = model::uv(st);
            [uv[0] + row[0], uv[1] + row[1]]
        }),
        colour: colours[i],
    };
    for tri in &mm.triangles {
        if tri.iter().any(|&i| i as usize >= qs.len()) {
            continue;
        }
        if !tri.iter().all(|&i| inside(qs[i as usize])) {
            let last = tri.iter().copied().max().unwrap_or(0) as usize;
            if reject_outside || qs[last].w >= div_z {
                continue;
            }
            let poly = clip_near(tri.map(|i| vertex(i as usize)));
            for k in 1..poly.len().saturating_sub(1) {
                for v in [poly[0], poly[k], poly[k + 1]] {
                    out.push(GVertex {
                        clip: screen.clip_3d(v.q),
                        uv: v.uv,
                        colour: v.colour,
                        params: params.0,
                        fog,
                        fog_f: fog_f(v.q),
                        region: 0,
                    });
                }
            }
            continue;
        }
        for &i in tri {
            let i = i as usize;
            let uv = mm.uvs.get(i).map_or([0.0, 0.0], |&st| model::uv(st));
            out.push(GVertex {
                clip: screen.clip_3d(qs[i]),
                uv: [uv[0] + row[0], uv[1] + row[1]],
                colour: colours[i],
                params: params.0,
                fog,
                fog_f: fog_f(qs[i]),
                region: 0,
            });
        }
    }
    out
}

/// The GS blend `((A - B) * C >> 7) + D` as a wgpu blend state, where the
/// shader writes alpha as As / 0x80 (so C's factors are 0x80 = 1.0) and the
/// blend constant is FIX / 0x80. The forms the hardware can express exactly
/// are all the ones the game's tables use; anything else falls back to the
/// nearest.
pub fn blend_state(b: Blend) -> wgpu::BlendState {
    use BlendColour::{Dest, Source, Zero};
    use wgpu::{BlendComponent, BlendFactor as F, BlendOperation as Op};
    let (c, one_minus_c) = match b.c {
        BlendAlpha::Source => (F::SrcAlpha, F::OneMinusSrcAlpha),
        BlendAlpha::Dest => (F::DstAlpha, F::OneMinusDstAlpha),
        BlendAlpha::Fix => (F::Constant, F::OneMinusConstant),
    };
    let colour = |src_factor, dst_factor, operation| BlendComponent { src_factor, dst_factor, operation };
    let component = match (b.a, b.b, b.d) {
        // A = B: D alone.
        (a, bb, Source) if a == bb => colour(F::One, F::Zero, Op::Add),
        (a, bb, Dest) if a == bb => colour(F::Zero, F::One, Op::Add),
        (a, bb, Zero) if a == bb => colour(F::Zero, F::Zero, Op::Add),
        (Source, Dest, Dest) => colour(c, one_minus_c, Op::Add),
        (Source, Zero, Dest) => colour(c, F::One, Op::Add),
        (Source, Zero, Zero) => colour(c, F::Zero, Op::Add),
        (Zero, Source, Dest) => colour(c, F::One, Op::ReverseSubtract),
        (Dest, Zero, Source) => colour(F::One, c, Op::Add),
        (Dest, Source, Source) => colour(one_minus_c, c, Op::Add),
        (Source, Dest, Zero) => colour(c, c, Op::Subtract),
        (Dest, Source, Zero) => colour(c, c, Op::ReverseSubtract),
        (Zero, Dest, Source) => colour(F::One, c, Op::Subtract),
        (Dest, Zero, Zero) => colour(F::Zero, c, Op::Add),
        (Zero, Dest, Dest) => colour(F::Zero, one_minus_c, Op::Add),
        (Zero, Source, Zero) => colour(F::Zero, F::Zero, Op::Add),
        // (Cs - Cd) * C + Cs and (Cd - Cs) * C + Cd need a factor above 1;
        // drawn as plain transparency.
        _ => colour(c, one_minus_c, Op::Add),
    };
    // The frame buffer's alpha is written as it is (the GS does not blend
    // alpha).
    wgpu::BlendState { color: component, alpha: colour(F::One, F::Zero, Op::Add) }
}

/// wgpu's compare for a GS depth test: depth runs the GS's way, larger
/// nearer ([`depth`]).
pub fn depth_compare(t: ZTest) -> wgpu::CompareFunction {
    match t {
        ZTest::Never => wgpu::CompareFunction::Never,
        ZTest::Always => wgpu::CompareFunction::Always,
        ZTest::GEqual => wgpu::CompareFunction::GreaterEqual,
        ZTest::Greater => wgpu::CompareFunction::Greater,
    }
}

/// Whether a failing alpha test still writes colour: FB_ONLY and RGB_ONLY
/// do (the depth and alpha they hold back are not modelled).
pub fn fail_writes_colour(fail: AlphaFail) -> bool {
    matches!(fail, AlphaFail::FbOnly | AlphaFail::RgbOnly)
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_draw::{Compare, DrawState, Vertex};

    const SCREEN: Screen = Screen { width: 512.0, height: 448.0, scale: 1.0 };

    #[test]
    fn near_clip_cuts_at_w_8() {
        let v = |x: f32, w: f32, u: f32| ClipVertex { q: Vec4::new(x, 0.0, 0.0, w), uv: [u, 0.0], colour: [w; 4] };
        // One vertex behind the camera: a quad, two new vertices at w = 8.
        let poly = clip_near([v(0.0, -8.0, 0.0), v(100.0, 24.0, 1.0), v(-100.0, 24.0, 1.0)]);
        assert_eq!(poly.len(), 4);
        assert_eq!(poly.iter().filter(|p| p.q.w == NEAR_W).count(), 2);
        let cut = poly.iter().find(|p| p.q.w == NEAR_W).unwrap();
        assert_eq!((cut.uv[0], cut.colour[0]), (0.5, NEAR_W));
        // Two behind: a smaller triangle; none behind: unchanged.
        assert_eq!(clip_near([v(0.0, -8.0, 0.0), v(1.0, 0.0, 0.0), v(2.0, 24.0, 0.0)]).len(), 3);
        assert_eq!(clip_near([v(0.0, 9.0, 0.0), v(1.0, 10.0, 0.0), v(2.0, 24.0, 0.0)]).len(), 3);
        assert!(clip_near([v(0.0, 1.0, 0.0), v(1.0, 2.0, 0.0), v(2.0, -4.0, 0.0)]).is_empty());
    }

    #[test]
    fn sprite_corners_and_colour() {
        let p = Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state: DrawState::sprite(Blend::MIX, None),
            verts: vec![
                Vertex { x: 0.0, y: 0.0, u: 0.0, v: 0.0, rgba: Rgba::BLACK, ..Default::default() },
                Vertex { x: 512.0, y: 448.0, u: 16.0, v: 8.0, rgba: Rgba::NEUTRAL, ..Default::default() },
            ],
        };
        let v = prim(&p, Some((16, 8)), SCREEN, Params::default());
        assert_eq!(v.len(), 6);
        // The whole screen, moved half a pixel: pixel (0, 0)'s corner is
        // wgpu's first centre.
        assert_eq!(v[0].clip[..2], [0.5 / 256.0 - 1.0, 1.0 - 0.5 / 224.0]);
        // The far corner's texel, a hair inside the boundary.
        assert_eq!(v[4].uv, [(16.0 + TEXEL_BIAS) / 16.0, (8.0 + TEXEL_BIAS) / 8.0]);
        assert!(v.iter().all(|x| x.colour == [128.0; 4]));
    }

    #[test]
    fn flat_strip_takes_the_last_vertex() {
        let c = |r| Rgba::new(r, 0, 0, 0x80);
        let p = Prim {
            kind: PrimKind::Strip,
            gouraud: false,
            state: DrawState::sprite(Blend::MIX, None),
            verts: (0..4).map(|i| Vertex { x: i as f32, rgba: c(i * 10), ..Default::default() }).collect(),
        };
        let v = prim(&p, None, SCREEN, Params::default());
        assert_eq!(v.len(), 6);
        assert!(v[..3].iter().all(|x| x.colour[0] == 20.0));
        assert!(v[3..].iter().all(|x| x.colour[0] == 30.0));
    }

    #[test]
    fn clip_3d_matches_2d_at_w_1() {
        let q = Vec4::new(100.0, 50.0, 1000.0, 1.0);
        let a = SCREEN.clip_3d(q);
        let b = SCREEN.clip_2d(100.0, 50.0, 1000);
        for k in 0..4 {
            assert!((a[k] - b[k]).abs() < 1e-6);
        }
        // Scaling q by w moves nothing on screen.
        let c = SCREEN.clip_3d(q * 4.0);
        for k in 0..3 {
            assert!((c[k] / c[3] - a[k]).abs() < 1e-6);
        }
    }

    #[test]
    fn table_blends_are_exact_forms() {
        use wgpu::{BlendFactor as F, BlendOperation as Op};
        let mix = blend_state(Blend::MIX).color;
        assert_eq!((mix.src_factor, mix.dst_factor, mix.operation), (F::SrcAlpha, F::OneMinusSrcAlpha, Op::Add));
        let sub = blend_state(Blend::SUB).color;
        assert_eq!((sub.src_factor, sub.dst_factor, sub.operation), (F::SrcAlpha, F::One, Op::ReverseSubtract));
        // `(Cs - Cd) * FIX + Cd` with FIX 0x80, a constant of 1.0: Cs.
        let rep = blend_state(Blend::REPLACE).color;
        assert_eq!((rep.src_factor, rep.dst_factor), (F::Constant, F::OneMinusConstant));
        let dm = blend_state(Blend::DEST_MUL).color;
        assert_eq!((dm.src_factor, dm.dst_factor), (F::One, F::SrcAlpha));
        let af = blend_state(Blend::ADD_FIX).color;
        assert_eq!((af.src_factor, af.dst_factor), (F::Constant, F::One));
    }

    #[test]
    fn params_pack() {
        let p = Params::new(
            Some((TexFunc::Modulate, true)),
            AlphaTest::On { method: Compare::GEqual, reference: 0x40, fail: AlphaFail::Keep },
        );
        assert_eq!(p.0, 0b100 | 0b1000 | 0b1_0000 | 5 << 5 | 0x40 << 8);
    }
}
