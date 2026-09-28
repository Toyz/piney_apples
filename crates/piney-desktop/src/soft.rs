//! A small CPU GS for checking frames without a GPU: every [`piney_draw::Cmd`]
//! rasterised into a 512 x 448 RGBA buffer with the GS's blend, texture
//! function, fog, alpha test, Z test and scissor, in GS units (0x80 = 1.0). It
//! is a checker, not the renderer (`piney-gs` is): a pixel is taken, and a
//! texture sampled, at its top-left corner. [`TexRef::FrameBuffer`] is the
//! canvas as the command finds it; [`TexRef::PreviousFrame`] is the picture
//! [`Canvas::next_frame`] kept.

use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat4, Vec4};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::model::{self, Model};
use piney_data::scene::Scene;
use piney_data::texture::{self, PSMT4, PSMT8};
use piney_draw::{
    AlphaFail, AlphaTest, Blend, BlendAlpha, BlendColour, Cmd, Compare, DrawState, Filter, Frame, ModelDraw, Prim,
    PrimKind, Rgba, ShadowPass, TexFunc, TexRef, Upload, UploadFormat, Wrap, ZTest,
};

/// A texture as palette colours in GS units, stored row order.
#[derive(Clone)]
struct Image {
    w: usize,
    h: usize,
    texels: Vec<[u8; 4]>,
}

impl Image {
    fn texel(&self, u: i32, v: i32, wrap: Wrap) -> [u8; 4] {
        let (u, v) = match wrap {
            Wrap::Repeat => (u.rem_euclid(self.w as i32), v.rem_euclid(self.h as i32)),
            Wrap::Clamp => (u.clamp(0, self.w as i32 - 1), v.clamp(0, self.h as i32 - 1)),
            Wrap::Region { mask, fix } => {
                let r = |t: i32, n: usize| ((t & i32::from(mask)) | i32::from(fix)).rem_euclid(n as i32);
                (r(u, self.w), r(v, self.h))
            }
        };
        self.texels[v as usize * self.w + u as usize]
    }

    fn sample(&self, u: f32, v: f32, filter: Filter, wrap: Wrap) -> [u8; 4] {
        match filter {
            Filter::Nearest => self.texel(u.floor() as i32, v.floor() as i32, wrap),
            Filter::Linear => {
                let (u, v) = (u - 0.5, v - 0.5);
                let (u0, v0) = (u.floor(), v.floor());
                let (fu, fv) = (u - u0, v - v0);
                let (u0, v0) = (u0 as i32, v0 as i32);
                let t = [
                    self.texel(u0, v0, wrap),
                    self.texel(u0 + 1, v0, wrap),
                    self.texel(u0, v0 + 1, wrap),
                    self.texel(u0 + 1, v0 + 1, wrap),
                ];
                let mut out = [0u8; 4];
                for (c, o) in out.iter_mut().enumerate() {
                    let top = f32::from(t[0][c]) * (1.0 - fu) + f32::from(t[1][c]) * fu;
                    let bot = f32::from(t[2][c]) * (1.0 - fu) + f32::from(t[3][c]) * fu;
                    *o = (top * (1.0 - fv) + bot * fv).round() as u8;
                }
                out
            }
        }
    }
}

struct FileData {
    ccs: Ccs,
    scene: Scene,
    models: HashMap<u32, Rc<Model>>,
}

/// Textures and models, read on first use.
pub struct Assets {
    archive: std::sync::Arc<Archive>,
    files: HashMap<String, Option<Rc<FileData>>>,
    images: HashMap<(String, u32, u32), Option<Rc<Image>>>,
}

impl Assets {
    pub fn new(archive: std::sync::Arc<Archive>) -> Self {
        Assets { archive, files: HashMap::new(), images: HashMap::new() }
    }

    fn file(&mut self, stem: &str) -> Option<Rc<FileData>> {
        if let Some(f) = self.files.get(stem) {
            return f.clone();
        }
        let f = (|| -> piney_data::Result<FileData> {
            let ccs = Ccs::parse(self.archive.inflate_named(stem)?)?;
            let scene = Scene::read(&ccs)?;
            let models = model::models(&ccs)?.into_iter().map(|m| (m.object, Rc::new(m))).collect();
            Ok(FileData { ccs, scene, models })
        })()
        .ok()
        .map(Rc::new);
        self.files.insert(stem.to_string(), f.clone());
        f
    }

    fn image(&mut self, stem: &str, tex: u32, clut: u32) -> Option<Rc<Image>> {
        let key = (stem.to_string(), tex, clut);
        if let Some(i) = self.images.get(&key) {
            return i.clone();
        }
        let img = self.file(stem).and_then(|f| {
            let (textures, cluts) = texture::read(&f.ccs).ok()?;
            let t = textures.iter().find(|t| t.object == tex)?;
            let c = cluts.get(&clut).or_else(|| cluts.get(&t.clut))?;
            let (w, h) = (t.width(0) as usize, t.height(0) as usize);
            let px = &t.levels.first()?.pixels;
            let texels = (0..w * h)
                .map(|i| {
                    let idx = match t.psm {
                        PSMT8 => px.get(i).copied().unwrap_or(0) as usize,
                        PSMT4 => px.get(i / 2).map_or(0, |b| if i % 2 == 0 { b & 15 } else { b >> 4 }) as usize,
                        _ => 0,
                    };
                    c.colours.get(idx).copied().unwrap_or([255, 0, 255, 128])
                })
                .collect();
            Some(Rc::new(Image { w, h, texels }))
        });
        self.images.insert(key, img.clone());
        img
    }

    fn material_texture(&mut self, stem: &str, mat: Option<u32>, tex_swaps: &[(u32, u32)]) -> Option<(u32, u32)> {
        let f = self.file(stem)?;
        let m = f.scene.materials.get(&mat?)?;
        let texture = mat.and_then(|o| tex_swaps.iter().find(|(from, _)| *from == o)).map_or(m.texture, |s| s.1);
        let (textures, _) = texture::read(&f.ccs).ok()?;
        let t = textures.iter().find(|t| t.object == texture)?;
        Some((t.object, t.clut))
    }
}

fn upload_image(u: &Upload) -> Image {
    let (w, h) = (usize::from(u.width), usize::from(u.height));
    let texels = (0..w * h)
        .map(|i| {
            let idx = match u.format {
                UploadFormat::Psmt8 => u.pixels.get(i).copied().unwrap_or(0),
                UploadFormat::Psmt4 => u.pixels.get(i / 2).map_or(0, |b| if i % 2 == 0 { b & 15 } else { b >> 4 }),
                UploadFormat::Psmct32 => {
                    return u.pixels.get(4 * i..4 * i + 4).and_then(|t| t.try_into().ok()).unwrap_or([0; 4]);
                }
            };
            u.clut.get(idx as usize).map_or([0; 4], |c| c.0)
        })
        .collect();
    Image { w, h, texels }
}

/// The frame buffer.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    /// RGBA, alpha in GS units.
    pub px: Vec<[u8; 4]>,
    pub z: Vec<u32>,
    /// The last frame's picture ([`TexRef::PreviousFrame`]): black until
    /// [`Canvas::next_frame`].
    previous: Vec<[u8; 4]>,
}

/// One vertex ready to rasterise: position in pixels, GS Z, texture
/// coordinates in texels (divided by w for perspective when `q` < 1), and
/// colour.
#[derive(Clone, Copy, Debug)]
struct V {
    x: f32,
    y: f32,
    z: f32,
    /// 1 / w (1 for 2D).
    q: f32,
    /// u / w, v / w.
    u: f32,
    v: f32,
    c: [f32; 4],
    /// VU1's fog by depth: this vertex's F (with [`Raster::depth_fog`]).
    f: f32,
}

struct Raster<'a> {
    state: &'a DrawState,
    tex: Option<(&'a Image, TexFunc, bool, Filter, Wrap)>,
    alpha_ref: u8,
    fog: Option<piney_draw::Fog>,
    /// F interpolated from the vertices' own (`fog`'s is FOGCOL's only).
    depth_fog: bool,
    gouraud: bool,
    flat: [f32; 4],
}

fn blend_one(b: &Blend, cs: [u8; 4], cd: [u8; 4]) -> [u8; 4] {
    let pick = |s: BlendColour, k: usize| -> i32 {
        match s {
            BlendColour::Source => i32::from(cs[k]),
            BlendColour::Dest => i32::from(cd[k]),
            BlendColour::Zero => 0,
        }
    };
    let c = match b.c {
        BlendAlpha::Source => i32::from(cs[3]),
        BlendAlpha::Dest => i32::from(cd[3]),
        BlendAlpha::Fix => i32::from(b.fix),
    };
    let mut out = cd;
    for (k, o) in out.iter_mut().enumerate().take(3) {
        *o = ((((pick(b.a, k) - pick(b.b, k)) * c) >> 7) + pick(b.d, k)).clamp(0, 255) as u8;
    }
    out[3] = cs[3];
    out
}

impl Canvas {
    pub fn new(w: usize, h: usize, clear: Rgba) -> Self {
        Canvas {
            w,
            h,
            px: vec![[clear.0[0], clear.0[1], clear.0[2], 0]; w * h],
            z: vec![0; w * h],
            previous: vec![[0; 4]; w * h],
        }
    }

    /// Keep the picture as the previous frame's and clear for the next, as
    /// the double buffer's swap and clear do.
    pub fn next_frame(&mut self, clear: Rgba) {
        self.previous = std::mem::replace(&mut self.px, vec![[clear.0[0], clear.0[1], clear.0[2], 0]; self.w * self.h]);
        self.z.fill(0);
    }

    /// The `width` x `height` rectangle at (`x`, `y`) as a texture; texels
    /// outside the canvas are 0.
    fn region(&self, x: i16, y: i16, width: u16, height: u16) -> Image {
        let (w, h) = (usize::from(width.max(1)), usize::from(height.max(1)));
        let texels = (0..w * h)
            .map(|i| {
                let (px, py) = (i64::from(x) + (i % w) as i64, i64::from(y) + (i / w) as i64);
                let inside = (0..self.w as i64).contains(&px) && (0..self.h as i64).contains(&py);
                if inside { self.px[py as usize * self.w + px as usize] } else { [0; 4] }
            })
            .collect();
        Image { w, h, texels }
    }

    /// The whole buffer resampled to `width` x `height` ([`TexRef::ScaledFrame`]):
    /// each texel the buffer read bilinearly (texel centres at +0.5,
    /// clamped) at its own centre scaled up.
    fn scaled(&self, width: u16, height: u16) -> Image {
        let (w, h) = (usize::from(width.max(1)), usize::from(height.max(1)));
        let (sx, sy) = (self.w as f32 / w as f32, self.h as f32 / h as f32);
        let at = |x: i64, y: i64| {
            let (x, y) = (x.clamp(0, self.w as i64 - 1), y.clamp(0, self.h as i64 - 1));
            self.px[y as usize * self.w + x as usize].map(f32::from)
        };
        let texels = (0..w * h)
            .map(|i| {
                let (u, v) = (((i % w) as f32 + 0.5) * sx - 0.5, ((i / w) as f32 + 0.5) * sy - 0.5);
                let (x0, y0) = (u.floor(), v.floor());
                let (fx, fy) = (u - x0, v - y0);
                let (x0, y0) = (x0 as i64, y0 as i64);
                let (a, b, c, d) = (at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1));
                std::array::from_fn(|k| {
                    let top = a[k] + (b[k] - a[k]) * fx;
                    let bottom = c[k] + (d[k] - c[k]) * fx;
                    (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8
                })
            })
            .collect();
        Image { w, h, texels }
    }

    /// The buffer as 8-bit RGBA for a PNG (alpha opaque).
    pub fn rgba(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn plot(&mut self, r: &Raster, px: usize, py: usize, z: f32, c: [f32; 4], uv: Option<(f32, f32)>, vf: f32) {
        let s = &r.state.scissor;
        if px < s.x0 as usize || px > s.x1 as usize || py < s.y0 as usize || py > s.y1 as usize {
            return;
        }
        let i = py * self.w + px;
        let mut col = [c[0].clamp(0.0, 255.0), c[1].clamp(0.0, 255.0), c[2].clamp(0.0, 255.0), c[3].clamp(0.0, 255.0)];
        if let (Some((img, func, use_alpha, filter, wrap)), Some((u, v))) = (r.tex, uv) {
            let t = img.sample(u, v, filter, wrap);
            let t = [f32::from(t[0]), f32::from(t[1]), f32::from(t[2]), f32::from(t[3])];
            match func {
                TexFunc::Modulate => {
                    for k in 0..3 {
                        col[k] = ((t[k] * col[k]) / 128.0).floor().min(255.0);
                    }
                    col[3] = if use_alpha { ((t[3] * col[3]) / 128.0).floor().min(255.0) } else { col[3] };
                }
                TexFunc::Decal => {
                    col[..3].copy_from_slice(&t[..3]);
                    if use_alpha {
                        col[3] = t[3];
                    }
                }
                _ => {
                    let a = col[3];
                    for (c, tk) in col.iter_mut().zip(t).take(3) {
                        *c = ((tk * *c) / 128.0 + a).floor().min(255.0);
                    }
                    if use_alpha {
                        col[3] += t[3];
                    }
                }
            }
        }
        if let Some(f) = r.fog {
            let ff = if r.depth_fog { vf.floor().clamp(0.0, 255.0) } else { f32::from(f.f) };
            for (c, fc) in col.iter_mut().zip(f.colour) {
                *c = ((*c * ff + f32::from(fc) * (255.0 - ff)) / 256.0).floor();
            }
        }
        let cs = [col[0] as u8, col[1] as u8, col[2] as u8, col[3].min(255.0) as u8];
        let zi = z.clamp(0.0, u32::MAX as f32) as u32;
        let depth = r.state.depth;
        let z_pass = match depth.test {
            ZTest::Never => false,
            ZTest::Always => true,
            ZTest::GEqual => zi >= self.z[i],
            ZTest::Greater => zi > self.z[i],
        };
        if !z_pass {
            return;
        }
        let (mut write_c, mut write_z) = (true, depth.write);
        if let AlphaTest::On { method, fail, .. } = r.state.alpha_test {
            let a = cs[3];
            let rf = r.alpha_ref;
            let pass = match method {
                Compare::Never => false,
                Compare::Always => true,
                Compare::Less => a < rf,
                Compare::LEqual => a <= rf,
                Compare::Equal => a == rf,
                Compare::GEqual => a >= rf,
                Compare::Greater => a > rf,
                Compare::NotEqual => a != rf,
            };
            if !pass {
                match fail {
                    AlphaFail::Keep => return,
                    AlphaFail::FbOnly => write_z = false,
                    AlphaFail::ZbOnly => write_c = false,
                    AlphaFail::RgbOnly => write_z = false,
                }
            }
        }
        if write_c {
            self.px[i] = match &r.state.blend {
                Some(b) => blend_one(b, cs, self.px[i]),
                None => cs,
            };
        }
        if write_z {
            self.z[i] = zi;
        }
    }

    fn triangle(&mut self, r: &Raster, a: V, b: V, c: V) {
        let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if area == 0.0 || !area.is_finite() {
            return;
        }
        let x0 = a.x.min(b.x).min(c.x).ceil().max(0.0) as i64;
        let x1 = a.x.max(b.x).max(c.x).ceil().min(self.w as f32) as i64;
        let y0 = a.y.min(b.y).min(c.y).ceil().max(0.0) as i64;
        let y1 = a.y.max(b.y).max(c.y).ceil().min(self.h as f32) as i64;
        for py in y0..y1 {
            for px in x0..x1 {
                let (fx, fy) = (px as f32, py as f32);
                let w0 = ((b.x - fx) * (c.y - fy) - (c.x - fx) * (b.y - fy)) / area;
                let w1 = ((c.x - fx) * (a.y - fy) - (a.x - fx) * (c.y - fy)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * a.z + w1 * b.z + w2 * c.z;
                let q = w0 * a.q + w1 * b.q + w2 * c.q;
                let col =
                    if r.gouraud { std::array::from_fn(|k| w0 * a.c[k] + w1 * b.c[k] + w2 * c.c[k]) } else { r.flat };
                let uv = r.tex.map(|_| ((w0 * a.u + w1 * b.u + w2 * c.u) / q, (w0 * a.v + w1 * b.v + w2 * c.v) / q));
                let vf = w0 * a.f + w1 * b.f + w2 * c.f;
                self.plot(r, px as usize, py as usize, z, col, uv, vf);
            }
        }
    }

    fn sprite(&mut self, r: &Raster, a: V, b: V) {
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        let (px0, px1) = (x0.ceil().max(0.0) as i64, x1.ceil().min(self.w as f32) as i64);
        let (py0, py1) = (y0.ceil().max(0.0) as i64, y1.ceil().min(self.h as f32) as i64);
        for py in py0..py1 {
            let fy = if b.y != a.y { (py as f32 - a.y) / (b.y - a.y) } else { 0.0 };
            for px in px0..px1 {
                let fx = if b.x != a.x { (px as f32 - a.x) / (b.x - a.x) } else { 0.0 };
                let uv = r.tex.map(|_| (a.u + (b.u - a.u) * fx, a.v + (b.v - a.v) * fy));
                self.plot(r, px as usize, py as usize, 0.0, b.c, uv, 255.0);
            }
        }
    }

    /// Draw a whole frame.
    pub fn draw(&mut self, frame: &Frame, assets: &mut Assets) {
        let uploads: HashMap<u32, Image> = frame.uploads.iter().map(|u| (u.id, upload_image(u))).collect();
        for cmd in &frame.cmds {
            match cmd {
                Cmd::Prim(p) => self.prim(p, &uploads, assets),
                Cmd::Model(m) => self.model(m, assets),
                Cmd::Shadow(s) => self.shadow(s),
            }
        }
    }

    /// [`ShadowPass`]: the Z buffer read small, each group's polygons
    /// counted against it, the buffer laid over the rectangle.
    fn shadow(&mut self, s: &ShadowPass) {
        let (w, h) = (usize::from(s.width), usize::from(s.height));
        let [x0, y0, x1, y1] = s.rect;
        if w == 0 || h == 0 || x1 <= x0 || y1 <= y0 {
            return;
        }
        let (sx, sy) = ((x1 - x0) / w as f32, (y1 - y0) / h as f32);
        let at = |x: f32, y: f32| {
            let (x, y) =
                ((x.floor() as i64).clamp(0, self.w as i64 - 1), (y.floor() as i64).clamp(0, self.h as i64 - 1));
            y as usize * self.w + x as usize
        };
        let zb: Vec<f32> =
            (0..w * h).map(|i| self.z[at(x0 + (i % w) as f32 * sx, y0 + (i / w) as f32 * sy)] as f32).collect();
        let (mut alpha, mut count) = (vec![0u8; w * h], vec![0i32; w * h]);
        for g in s.groups.iter().rev() {
            count.fill(0);
            for p in &g.polys {
                let sign = if p.add { 1 } else { -1 };
                for k in 2..p.verts.len() {
                    let (a, b, c) = (p.verts[0], p.verts[k - 1], p.verts[k]);
                    let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
                    if area == 0.0 || !area.is_finite() {
                        continue;
                    }
                    let lo = |i: usize| a[i].min(b[i]).min(c[i]).ceil().max(0.0) as usize;
                    let hi = |i: usize, n: usize| a[i].max(b[i]).max(c[i]).ceil().clamp(0.0, n as f32) as usize;
                    for py in lo(1)..hi(1, h) {
                        for px in lo(0)..hi(0, w) {
                            let (fx, fy) = (px as f32, py as f32);
                            let w0 = ((b[0] - fx) * (c[1] - fy) - (c[0] - fx) * (b[1] - fy)) / area;
                            let w1 = ((c[0] - fx) * (a[1] - fy) - (a[0] - fx) * (c[1] - fy)) / area;
                            let w2 = 1.0 - w0 - w1;
                            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                                continue;
                            }
                            // VU1's ftoi4 holds Z at 0x7fffffff.
                            let z = (w0 * a[2] + w1 * b[2] + w2 * c[2]).min(2_147_483_647.0);
                            if z > zb[py * w + px] {
                                count[py * w + px] += sign;
                            }
                        }
                    }
                }
            }
            for (a, &n) in alpha.iter_mut().zip(&count) {
                if n > 0 {
                    *a = g.alpha;
                }
            }
        }
        // TransShadowTex: black at the texel's alpha times the darkness,
        // filtered bilinearly, over the frame (MIX), alpha written; once at
        // each tap.
        let texel = |x: i64, y: i64| {
            f32::from(alpha[y.clamp(0, h as i64 - 1) as usize * w + x.clamp(0, w as i64 - 1) as usize])
        };
        for &[ox, oy] in &s.taps {
            let (x0, y0, x1, y1) = (x0 + ox, y0 + oy, x1 + ox, y1 + oy);
            let (px0, px1) = (x0.ceil().max(0.0) as usize, x1.ceil().clamp(0.0, self.w as f32) as usize);
            let (py0, py1) = (y0.ceil().max(0.0) as usize, y1.ceil().clamp(0.0, self.h as f32) as usize);
            for py in py0..py1 {
                let v = (py as f32 - y0) / sy - 0.5;
                let (ty, fy) = (v.floor() as i64, v - v.floor());
                for px in px0..px1 {
                    let u = (px as f32 - x0) / sx - 0.5;
                    let (tx, fx) = (u.floor() as i64, u - u.floor());
                    let top = texel(tx, ty) + (texel(tx + 1, ty) - texel(tx, ty)) * fx;
                    let bottom = texel(tx, ty + 1) + (texel(tx + 1, ty + 1) - texel(tx, ty + 1)) * fx;
                    let ta = (top + (bottom - top) * fy) as u32;
                    let a = (ta * u32::from(s.darkness)) >> 7;
                    let d = &mut self.px[py * self.w + px];
                    for c in &mut d[..3] {
                        *c = (u32::from(*c) - ((u32::from(*c) * a) >> 7).min(u32::from(*c))) as u8;
                    }
                    d[3] = a.min(255) as u8;
                }
            }
        }
    }

    fn prim(&mut self, p: &Prim, uploads: &HashMap<u32, Image>, assets: &mut Assets) {
        let img = p.state.texture.as_ref().and_then(|t| match &t.tex {
            TexRef::Upload(id) => uploads.get(id).cloned().map(Rc::new),
            TexRef::Ccs { file, texture, clut } => assets.image(file, *texture, *clut),
            &TexRef::FrameBuffer { x, y, width, height } => Some(Rc::new(self.region(x, y, width, height))),
            TexRef::PreviousFrame => Some(Rc::new(Image { w: self.w, h: self.h, texels: self.previous.clone() })),
            &TexRef::ScaledFrame { width, height } => Some(Rc::new(self.scaled(width, height))),
        });
        let tex = match (&p.state.texture, &img) {
            (Some(t), Some(i)) => Some((i.as_ref(), t.func, t.use_alpha, t.filter, t.wrap)),
            _ => None,
        };
        let v = |q: &piney_draw::Vertex| V {
            x: q.x,
            y: q.y,
            z: q.z as f32,
            q: 1.0,
            u: q.u,
            v: q.v,
            c: [f32::from(q.rgba.0[0]), f32::from(q.rgba.0[1]), f32::from(q.rgba.0[2]), f32::from(q.rgba.0[3])],
            f: 255.0,
        };
        let vs: Vec<V> = p.verts.iter().map(v).collect();
        let flat = vs.last().map_or([0.0; 4], |l| l.c);
        let alpha_ref = match p.state.alpha_test {
            AlphaTest::On { reference, .. } => reference,
            AlphaTest::Off => 0,
        };
        let r = Raster { state: &p.state, tex, alpha_ref, fog: None, depth_fog: false, gouraud: p.gouraud, flat };
        match p.kind {
            PrimKind::Sprite => {
                for pair in vs.chunks(2) {
                    if pair.len() == 2 {
                        self.sprite(&r, pair[0], pair[1]);
                    }
                }
            }
            PrimKind::Triangles => {
                for t in vs.chunks(3) {
                    if t.len() == 3 {
                        self.triangle(&Raster { flat: t[2].c, ..r_copy(&r) }, t[0], t[1], t[2]);
                    }
                }
            }
            PrimKind::Strip => {
                for i in 2..vs.len() {
                    self.triangle(&Raster { flat: vs[i].c, ..r_copy(&r) }, vs[i - 2], vs[i - 1], vs[i]);
                }
            }
            PrimKind::Fan => {
                for i in 2..vs.len() {
                    self.triangle(&Raster { flat: vs[i].c, ..r_copy(&r) }, vs[0], vs[i - 1], vs[i]);
                }
            }
        }
    }

    fn model(&mut self, m: &ModelDraw, assets: &mut Assets) {
        let Some(file) = assets.file(&m.file) else { return };
        let Some(model) = file.models.get(&m.model).cloned() else { return };
        let to_screen = Mat4::from_cols_array_2d(&m.to_screen);
        let lit = model.mtype & 7 != 0;
        // A constant fog wins over VU1's by depth (ccChar::Draw's blend).
        let depth = if m.fog.is_none() { m.depth_fog } else { None };
        let fog = m.fog.or(depth.map(|d| piney_draw::Fog { f: 0xff, colour: d.colour }));
        let targets: Vec<(std::rc::Rc<piney_data::model::Model>, f32)> =
            m.morph.iter().filter_map(|&(t, w)| file.models.get(&t).cloned().map(|t| (t, w))).collect();
        let targets: Vec<(&piney_data::model::Model, f32)> = targets.iter().map(|(t, w)| (&**t, *w)).collect();
        for md in &m.mmats {
            let Some(mm) = model.mmats.get(md.index as usize) else { continue };
            let morphed = if targets.is_empty() { None } else { model.morph(md.index as usize, &targets) };
            let positions = morphed.as_deref().unwrap_or(&mm.positions);
            let tex = assets
                .material_texture(&m.file, mm.material, &m.tex_swaps)
                .and_then(|(t, c)| assets.image(&m.file, t, c));
            let texw = tex.as_ref().map_or((1.0, 1.0), |i| (i.w as f32, i.h as f32));
            let verts: Vec<Option<V>> = (0..mm.positions.len())
                .map(|i| {
                    let p = model.position(positions[i]);
                    let q: Vec4 = to_screen * p.extend(1.0);
                    if q.w <= 0.0 {
                        return None;
                    }
                    let inv = 1.0 / q.w;
                    let (x, y) = (q.x * inv, q.y * inv);
                    if m.reject_outside {
                        let (gx, gy) = (x + 1792.0, y + 1824.0);
                        if !(0.0..=4095.0).contains(&gx) || !(0.0..=4095.0).contains(&gy) {
                            return None;
                        }
                    }
                    let st = mm.uvs.get(i).copied().unwrap_or([0, 0]);
                    let u = (f32::from(st[0]) + f32::from(md.uv_row[0])) / 256.0 * texw.0;
                    let v = (f32::from(st[1]) + f32::from(md.uv_row[1])) / 256.0 * texw.1;
                    let c = if lit {
                        [128.0, 128.0, 128.0, (128.0 * md.alpha).floor()]
                    } else {
                        let c = mm.colours.get(i).copied().unwrap_or([128; 4]);
                        [f32::from(c[0]), f32::from(c[1]), f32::from(c[2]), (f32::from(c[3]) * md.alpha).floor()]
                    };
                    let f = depth.map_or(255.0, |d| d.at(q.w));
                    let z = q.z * piney_draw::MODEL_Z_SCALE * inv;
                    Some(V { x, y, z, q: inv, u: u * inv, v: v * inv, c, f })
                })
                .collect();
            let texs = tex.as_ref().map(|i| (i.as_ref(), m.tex.func, m.tex.use_alpha, m.tex.filter, md.wrap));
            for t in model::strip_triangles(&mm.flags) {
                let [a, b, c] = t.map(|k| verts.get(k as usize).copied().flatten());
                let (Some(a), Some(b), Some(c)) = (a, b, c) else { continue };
                let r = Raster {
                    state: &m.state,
                    tex: texs,
                    alpha_ref: md.alpha_ref,
                    fog,
                    depth_fog: depth.is_some(),
                    gouraud: true,
                    flat: c.c,
                };
                self.triangle(&r, a, b, c);
            }
        }
    }
}

fn r_copy<'a>(r: &Raster<'a>) -> Raster<'a> {
    Raster {
        state: r.state,
        tex: r.tex,
        alpha_ref: r.alpha_ref,
        fog: r.fog,
        depth_fog: r.depth_fog,
        gouraud: r.gouraud,
        flat: r.flat,
    }
}

/// Minimal PNG (RGBA8, stored deflate blocks).
pub fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc(&body).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((w as usize * 4 + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * w as usize * 4..(y + 1) * w as usize * 4]);
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(u8::from(last));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame-buffer texture is the canvas as the command finds it, 0 past
    /// its edge; the previous frame is what `next_frame` kept.
    #[test]
    fn frame_buffer_textures() {
        use piney_draw::{Prim, TexState, Vertex};
        let mut c = Canvas::new(8, 8, Rgba::new(10, 10, 10, 0));
        let mut assets = Assets::new(std::sync::Arc::new(Archive::new(Vec::new()).unwrap()));
        let opaque = |tex: Option<TexRef>| DrawState {
            blend: None,
            texture: tex.map(|tex| TexState {
                tex,
                func: TexFunc::Modulate,
                use_alpha: false,
                filter: Filter::Nearest,
                wrap: Wrap::Clamp,
            }),
            ..DrawState::sprite(Blend::MIX, None)
        };
        let sprite = |x0: f32, y0: f32, x1: f32, y1: f32, u1: f32, v1: f32, rgba, state| {
            let v = |x, y, u, v| Vertex { x, y, u, v, rgba, ..Default::default() };
            Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state,
                verts: vec![v(x0, y0, 0.0, 0.0), v(x1, y1, u1, v1)],
            })
        };
        let red = Rgba::new(200, 0, 0, 0x80);
        let mut f = Frame { width: 8, height: 8, ..Frame::default() };
        f.cmds.push(sprite(0.0, 6.0, 2.0, 8.0, 0.0, 0.0, red, opaque(None)));
        let copy = TexRef::FrameBuffer { x: 0, y: 6, width: 2, height: 4 };
        f.cmds.push(sprite(4.0, 0.0, 6.0, 4.0, 2.0, 4.0, Rgba::NEUTRAL, opaque(Some(copy))));
        f.cmds.push(sprite(0.0, 6.0, 2.0, 8.0, 0.0, 0.0, Rgba::new(0, 0, 200, 0x80), opaque(None)));
        c.draw(&f, &mut assets);
        assert_eq!(c.px[4][..3], [200, 0, 0], "the copy was made before the blue");
        assert_eq!(c.px[3 * 8 + 4][..3], [0, 0, 0], "rows past the frame read 0");
        assert_eq!(c.px[6 * 8][..3], [0, 0, 200]);
        c.next_frame(Rgba::BLACK);
        let mut g = Frame { width: 8, height: 8, ..Frame::default() };
        g.cmds.push(sprite(0.0, 0.0, 8.0, 8.0, 8.0, 8.0, Rgba::NEUTRAL, opaque(Some(TexRef::PreviousFrame))));
        c.draw(&g, &mut assets);
        assert_eq!(c.px[6 * 8][..3], [0, 0, 200]);
        assert_eq!(c.px[4][..3], [200, 0, 0]);
        assert_eq!(c.px[7][..3], [10, 10, 10]);
    }

    #[test]
    fn mix_blend_halves() {
        let b = Blend::MIX;
        let out = blend_one(&b, [200, 100, 0, 64], [0, 100, 200, 0]);
        assert_eq!(&out[..3], &[100, 100, 100]);
    }
}
