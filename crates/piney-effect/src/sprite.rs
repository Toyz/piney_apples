//! How a `ccEff` sprite reaches the GS: `ccEff::Draw(pat)` (main
//! 0x0013bcb0) builds a 27-qword VIF packet, sorts it into the active
//! layer, and VU1's `mc_DrawEff` (micro 0x595) turns it into one textured
//! triangle strip facing the camera.
//!
//! ```text
//! ccEff::Draw(pat):
//!   a = ftoi0(transparency * pat[pat].transparency) >> 5; a < 0: nothing;
//!     above 255: 255
//!   m = unit, m[0][0] scaleX, m[1][1] scaleY; RotMatrixZ(rotate) unless
//!     rotate == 0; m = view.fview * m; m.t += pos; m = view.world_screen * m
//!   m[3][3] (the centre's w) below view +0x1dc (8) or not <= +0x1ec (2^20):
//!     nothing
//!   GetWork(432); ccDLSort::Add(m[3][2] / m[3][3], the packet)
//!   (a texture or palette not yet in VRAM: its upload is chained in first)
//! the packet (qword: contents):
//!    0  DMA NEXT qwc 0 -> 1 (the head, which uploads may be put before)
//!    1  DMA NEXT qwc 25 (the tail) | STMOD 0, STCYCL 4,4
//!    2  STMASK 0x50505050, FLUSHA, UNPACK V4-32 addr 0 num 18
//!    3-6   VU 0-3    m
//!    7     VU 4      not written (not read)
//!    8     VU 5      ccDrawEnv fMin, fMax, fogB, fogA
//!    9     VU 6      GIFtag A+D nloop 1 nreg 10          (0x002fb110)
//!   10-19  VU 7-16   ALPHA_1  TEST_1 (below)  TEXFLUSH  TEX0_1 (tex | clut)
//!                    TEX1_1 (tex | ccDrawEnv +0xa8)  MIPTBP1_1  CLAMP_1
//!                    RGBAQ (color RGB, a, Q 1.0)  FOGCOL (+0xd8)
//!                    ZBUF_1 (ccSys +0xbc8)
//!   20     VU 17     GIFtag PACKED eop nloop 1 nreg 9, PRE PRIM = prim;
//!                    FOG, (ST, XYZ2) x 4                 (0x002fb120)
//!   21  STMOD 1 (add the row), STROW (u, v, 0, 1.0 bits)
//!   22  NOP, UNPACK V2-16 masked usn addr 32 num 4: (0,0) (w,0) (0,h) (w,h)
//!   23  NOP, UNPACK V2-32 masked addr 36 num 4: (x0,y0) (x1,y0) (x0,y1)
//!       (x1,y1) as float bits
//!   26  NOP, MSCAL mc_DrawEff
//! TEST: the eff's, with (for flag bit 0x20 clear, blend type 0) AREF |=
//!   aref * a >> 7 from the texture; ZTE off (SetRenderState(0, 0)) draws
//!   ZTST ALWAYS with ZBUF's ZMSK set.
//! ```
//!
//! The mask (z, w from the row, x, y data) and offset mode make VU 32-35
//! the UVs `(u + du, v + dv, 0, 1.0)` and VU 36-39 the corners - with `u`
//! and `v` added to the float bits of x and y too, which moves them by a
//! few units in the last place: the port does the same.
//!
//! ```text
//! mc_DrawEff (VU1 0x595-0x5d1), per corner p = (x, y, 0, 1):
//!   q = m p (((m0 x + m1 y) + m2 0) + m3 1); v = q.xyz * (1 / q.w)
//!   inside: 0 < v.x < 4095 and 0 < v.y < 4095 (the MAC sign flags of
//!     0 - v and v - 4095, read four pairs later); any corner outside: no
//!     XGKICK, nothing drawn (the whole sprite)
//!   ST = itof12(u + du, v + dv): 1/4096ths of the texture; Q stays the
//!     A+D RGBAQ's 1.0
//!   XYZ2 = ftoi4(v), word 3 0 (drawing kick)
//!   FOG = ftoi4(clamp(fogB + fogA m[3][3], fMin, fMax)): the centre's
//!     depth for all four
//!   XGKICK VU 6
//! ```
//!
//! Nothing here depends on the effect: [`Eff::packet`] gives what the GS
//! receives (bit for bit, checked by `tools/test_effect_draw_rs.py`
//! against the game's `Draw` and `mc_DrawEff` run in eemu and
//! `tools/vu.py`), [`Eff::render`] puts it in the layer's sorted group as a
//! [`Prim`]. `Prim` has no fog; the effects' sprites have PRIM.FGE off
//! (`ccEffect::InitEffect` clears it), and FOG is in [`GsSprite::fog`] for
//! the rest.

use piney_data::ccs::Ccs;
use piney_desktop::layers::Layers;
use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
use piney_draw::{
    AlphaFail, AlphaTest, Blend, Cmd, Compare, Depth, DrawState, Filter, Prim, PrimKind, Rgba, Scissor, TexFunc,
    TexRef, TexState, Vertex, Wrap, ZTest,
};

use crate::draw::Camera;
use crate::ee::{self, F, ONE};
use crate::eff::{Eff, EffChunk};
use crate::files::Assets;
use crate::vu::{self, M4};

/// `-pi`, `ccView::SetView`'s turn about x.
const MINUS_PI: F = 0xc049_0fdb;
const K16: F = 0x4180_0000;
const K4095: F = 0x457f_f000;
const K4096: F = 0x4580_0000;
/// `mc_DrawEff`'s entry, as the MSCAL in the packet names it.
pub const MC_DRAW_EFF: u16 = 0x595;
/// The packet's size (`GetWork(432)`).
pub const PACKET_BYTES: usize = 432;

/// What `ccEff::Draw` reads of the frame besides the camera's matrices:
/// the view's w range, the active `ccDrawEnv`'s fog and filtering, and
/// `ccSys`'s Z buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawEnv {
    /// `ccView` +0x1dc `bboxClipMin.w`, +0x1ec `bboxClipMax.w`: the centre
    /// is drawn when near <= w <= far. `ccLayer::Init` (0x00108260) sets 8
    /// and 2^20 for the view every field layer shares (`sysLayer`'s).
    pub near: F,
    pub far: F,
    /// `ccDrawEnv` +0xb8 `fogNear`, +0xbc `fogFar`, +0xc0 `nearRate`, +0xc4
    /// `farRate`: [`DrawEnv::set_fog`]'s arguments.
    pub fog_near: F,
    pub fog_far: F,
    pub near_rate: F,
    pub far_rate: F,
    /// +0xc8 `fMin`, +0xcc `fMax`, +0xd0 `fogA`, +0xd4 `fogB`: FOG is
    /// `clamp(fogB + fogA * w, fMin, fMax)`.
    pub fog_min: F,
    pub fog_max: F,
    pub fog_a: F,
    pub fog_b: F,
    /// +0xd8 `fogColor`: FOGCOL.
    pub fog_color: u32,
    /// +0xa8 `tex1`: ORed into each texture's TEX1.
    pub tex1: u64,
    /// `ccSys` +0xbc8: ZBUF_1.
    pub zbuf: u64,
}

impl Default for DrawEnv {
    /// The field's view and a `ccDrawEnv` as `Reset` (0x001054d0) leaves
    /// it: `SetFog(0.1, 16777215, 0, 0, 0)` (fMin = fMax = 2.55 * 100, which
    /// the EE's truncating multiply makes 254.99998: FOG 254 everywhere),
    /// `SetMipmapMode(1, 5)` (MMAG linear, MMIN 5), `SetMipmap(0xff18, 0)`
    /// (K 0xf18, L 0); ZBUF as `ccSystem`'s screen set-up leaves it (ZBP
    /// 224, Z32, written).
    fn default() -> Self {
        let mut e = DrawEnv {
            near: 0x4100_0000,
            far: 0x4980_0000,
            fog_near: 0,
            fog_far: 0,
            near_rate: 0,
            far_rate: 0,
            fog_min: 0,
            fog_max: 0,
            fog_a: 0,
            fog_b: 0,
            fog_color: 0,
            tex1: 0xf18 << 32 | 5 << 6 | 1 << 5,
            zbuf: 0xe0,
        };
        e.set_fog(0x3dcc_cccd, 0x4b7f_ffff, 0, 0, 0);
        e
    }
}

impl DrawEnv {
    /// `ccDrawEnv::SetFog(near, far, nearRate, farRate, color)` (main
    /// 0x00105820): fMax = 2.55 (100 - nearRate) at `near` and nearer, fMin
    /// = 2.55 (100 - farRate) at `far` and beyond, linear between; the
    /// colour's low 24 bits. Mac Anu's (`ROOTTOWN01::ROOTTOWN01`, gcmn
    /// 0x00421630) is `SetFog(1000, 7500, 0, 85, 0x144870)`.
    pub fn set_fog(&mut self, near: F, far: F, near_rate: F, far_rate: F, color: u32) {
        const K2_55: F = 0x4023_3333;
        const K100: F = 0x42c8_0000;
        let fmin = ee::mul(K2_55, ee::sub(K100, far_rate));
        let fmax = ee::mul(K2_55, ee::sub(K100, near_rate));
        let d = ee::sub(fmin, fmax);
        let span = ee::sub(far, near);
        let b = ee::div(ee::mul(near, d), span);
        self.fog_near = near;
        self.fog_far = far;
        self.near_rate = near_rate;
        self.far_rate = far_rate;
        self.fog_min = fmin;
        self.fog_max = fmax;
        self.fog_a = ee::div(d, span);
        self.fog_b = ee::sub(fmax, b);
        self.fog_color = color & 0xff_ffff;
    }
}

/// An Eff chunk's texture as `ccEff::Draw` reads its `ccTexChunk`: the
/// Texture chunk's header (`ccStream::Decode_Texture` 0x0014d9a0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TexInfo {
    /// The TEX_ object and its CLT_ object.
    pub texture: u32,
    pub clut: u32,
    /// Bit 0x10: CLAMP_1 5 (clamp both ways), else 0 (repeat).
    pub flag: u8,
    pub psm: u8,
    pub mipmap: u8,
    /// +0x42: the alpha test's reference at full transparency.
    pub aref: u8,
    /// log2 width and height.
    pub tw: u8,
    pub th: u8,
}

impl TexInfo {
    /// TEX0's format bits `SetBuffAdrs` (0x00137090) sets: PSM, TW, TH,
    /// TCC 1, TFX 0 (MODULATE). TBP0, TBW and the palette's CBP, CPSM, CSM,
    /// CSA and CLD depend on where VRAM holds them and are not modelled.
    pub fn tex0_format(&self) -> u64 {
        u64::from(self.psm) << 20 | u64::from(self.tw) << 26 | u64::from(self.th) << 30 | 1 << 34
    }

    /// CLAMP_1 (`ccTexChunk::SetBltData` 0x00136e50).
    pub fn clamp(&self) -> u64 {
        if self.flag & 0x10 != 0 { 5 } else { 0 }
    }
}

/// Each of `effs`' textures in `ccs` (None: no Texture chunk names it,
/// and `Draw` sends the texture registers as zeros).
pub fn eff_textures(ccs: &Ccs, effs: &[EffChunk]) -> Vec<Option<TexInfo>> {
    let textures = piney_data::texture::read(ccs).map(|(t, _)| t).unwrap_or_default();
    effs.iter()
        .map(|e| {
            textures.iter().find(|t| t.object == e.texture).map(|t| TexInfo {
                texture: t.object,
                clut: t.clut,
                flag: t.flag,
                psm: t.psm,
                mipmap: t.mipmap,
                aref: t.aref,
                tw: t.tw,
                th: t.th,
            })
        })
        .collect()
}

/// `ccView::SetView`'s +0x10 `fview` (main 0x001052c0) for a view whose
/// `world_view` is `wv`: `sceVu0InversMatrix(wv)` (0x001107b0: the 3x3
/// transposed, the translation `-(R^T t)`), times `RotMatrixX(-pi)`, its
/// translation then set to (0, 0, 0, 1). It turns a sprite's local x/y
/// plane to face the camera (y up the screen).
pub fn fview(wv: &M4) -> M4 {
    let t = wv[3];
    let mut c: M4 = [[0; 4]; 4];
    for (j, col) in c.iter_mut().enumerate().take(3) {
        *col = [wv[0][j], wv[1][j], wv[2][j], 0];
    }
    let zero: [F; 3] = std::array::from_fn(|k| ee::sub(t[k], t[k]));
    let p: [F; 3] = std::array::from_fn(|k| {
        let acc = ee::mul(c[0][k], t[0]);
        let acc = ee::add(acc, ee::mul(c[1][k], t[1]));
        ee::add(acc, ee::mul(c[2][k], t[2]))
    });
    c[3] = [ee::sub(zero[0], p[0]), ee::sub(zero[1], p[1]), ee::sub(zero[2], p[2]), t[3]];
    let mut f = vu::mul(&c, &vu::rot_x(&vu::UNIT, MINUS_PI));
    f[3] = [0, 0, 0, ONE];
    f
}

/// One corner as the GS gets it: ST (float bits), then XYZ2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GsVertex {
    pub s: F,
    pub t: F,
    /// 12.4 fixed point, XYOFFSET included.
    pub x: u16,
    pub y: u16,
    pub z: u32,
}

/// Everything one sprite sends the GS, as `mc_DrawEff` kicks it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GsSprite {
    pub alpha: u64,
    pub test: u64,
    /// TEX1_1 and CLAMP_1 (0 without a texture); TEX0_1's format bits
    /// ([`TexInfo::tex0_format`]). MIPTBP1_1 is 0 for every effect texture
    /// (none has mip levels).
    pub tex1: u64,
    pub clamp: u64,
    pub tex: Option<TexInfo>,
    /// RGB from the eff's colour, A the pattern-scaled transparency, Q 1.0.
    pub rgbaq: u64,
    pub fog_color: u32,
    pub zbuf: u64,
    pub prim: u16,
    /// FOG's F (bits 4-11 of the qword's last word).
    pub fog: u8,
    /// The strip's corners: (x0, y0), (x1, y0), (x0, y1), (x1, y1).
    pub verts: [GsVertex; 4],
}

/// What one `Draw(pat)` gives.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Packet {
    /// `ccDLSort::Add`'s key: the centre's z / w.
    pub key: F,
    /// VU 0-3: model to screen.
    pub matrix: M4,
    /// None when `mc_DrawEff` finds a corner outside the GS's 0..4095
    /// window and kicks nothing (the packet is still sorted).
    pub gs: Option<GsSprite>,
}

/// `ACC = m0 x; ACC += m1 y; ACC += m2 z; m3 w + ACC` lane by lane, as VU1's
/// `mulax`/`madday`/`maddaz`/`maddw` do it.
fn apply(m: &M4, p: [F; 4]) -> [F; 4] {
    std::array::from_fn(|k| {
        let acc = ee::mul(m[0][k], p[0]);
        let acc = ee::add(acc, ee::mul(m[1][k], p[1]));
        let acc = ee::add(acc, ee::mul(m[2][k], p[2]));
        ee::add(acc, ee::mul(m[3][k], p[3]))
    })
}

fn sign(v: F) -> bool {
    v & 0x8000_0000 != 0
}

impl Eff {
    /// `ccEff::Draw(pat)` (main 0x0013bcb0) and `mc_DrawEff`: what the GS
    /// gets for pattern `pat`, through `camera`. None when nothing is sent:
    /// the transparency below 0, the centre outside the view's w range, or
    /// `pat` past the chunk's patterns (the game reads past its table).
    pub fn packet(&self, assets: &Assets, pat: u16, camera: &Camera) -> Option<Packet> {
        let chunk = assets.effs.get(self.file)?.get(self.chunk)?;
        let p = chunk.pats.get(usize::from(pat))?;
        let mut tex = assets.eff_tex.get(self.file).and_then(|t| t.get(self.chunk).copied()).flatten();
        // A palette the effect put in the ccTex's +0x3c (a particle's CLUT
        // texture, effDrain's orbs, the attribute critical's word): drawn
        // through it. Only a palette of the sprite's own file can be named.
        if let (Some(t), Some(c)) = (tex.as_mut(), self.clut.filter(|c| c.file == self.file)) {
            t.clut = c.object;
        }
        let a = ee::to_int(ee::mul(self.transparency, ee::from_int(i32::from(p.transparency)))) >> 5;
        if a < 0 {
            return None;
        }
        let a = a.min(255) as u32;
        let env = &camera.env;
        let mut m = vu::UNIT;
        m[0][0] = self.scale_x;
        m[1][1] = self.scale_y;
        if !ee::eq(self.rotate, 0) {
            m = vu::rot_z(&m, self.rotate);
        }
        m = vu::mul(&fview(&camera.world_view), &m);
        m = vu::trans(&m, self.pos);
        m = vu::mul(&camera.world_screen, &m);
        let w = m[3][3];
        if ee::lt(w, env.near) || !ee::le(w, env.far) {
            return None;
        }
        let key = ee::div(m[3][2], w);
        Some(Packet { key, matrix: m, gs: self.gs(p.u, p.v, a, tex, &m, env) })
    }

    /// The packet's GS registers and `mc_DrawEff`'s corners.
    fn gs(&self, u: u16, v: u16, a: u32, tex: Option<TexInfo>, m: &M4, env: &DrawEnv) -> Option<GsSprite> {
        let (u, v) = (u32::from(u), u32::from(v));
        let (w, h) = (self.wh & 0xffff, self.wh >> 16);
        let corners =
            [(self.x0, self.y0, 0, 0), (self.x1, self.y0, w, 0), (self.x0, self.y1, 0, h), (self.x1, self.y1, w, h)];
        let mut verts = [GsVertex::default(); 4];
        for (out, &(x, y, du, dv)) in verts.iter_mut().zip(&corners) {
            // The row (u, v, 0, 1.0 bits) added to the data lanes, the
            // masked lanes taken from it.
            let q = apply(m, [x.wrapping_add(u), y.wrapping_add(v), 0, ONE]);
            let r = ee::div(ONE, q[3]);
            let s: [F; 3] = std::array::from_fn(|k| ee::mul(q[k], r));
            let inside = sign(ee::sub(0, s[0]))
                && sign(ee::sub(0, s[1]))
                && sign(ee::sub(s[0], K4095))
                && sign(ee::sub(s[1], K4095));
            if !inside {
                return None;
            }
            let fixed = |x: F| ee::to_int(ee::mul(x, K16)) as u32;
            *out = GsVertex {
                s: ee::div(ee::from_int((u + du) as i32), K4096),
                t: ee::div(ee::from_int((v + dv) as i32), K4096),
                x: fixed(s[0]) as u16,
                y: fixed(s[1]) as u16,
                z: fixed(s[2]),
            };
        }
        let mut test = self.test;
        if self.flag & 0x20 == 0 {
            let r = tex.map_or(0, |t| (i32::from(t.aref) * a as i32) >> 7);
            test |= (i64::from(r) << 4) as u64;
        }
        let (test, zbuf) =
            if test & 0x1_0000 != 0 { (test, env.zbuf) } else { (test & !0x7_0000 | 0x3_0000, env.zbuf | 1 << 32) };
        let mut fog = ee::add(ee::mul(ONE, env.fog_b), ee::mul(env.fog_a, m[3][3]));
        if ee::cmp(fog, env.fog_max).is_gt() {
            fog = env.fog_max;
        }
        if ee::cmp(fog, env.fog_min).is_lt() {
            fog = env.fog_min;
        }
        Some(GsSprite {
            alpha: self.alpha,
            test,
            tex1: tex.map_or(0, |t| u64::from(t.mipmap) << 2 | env.tex1),
            clamp: tex.map_or(0, |t| t.clamp()),
            tex,
            rgbaq: u64::from(self.color & 0xff_ffff | a << 24) | u64::from(ONE) << 32,
            fog_color: env.fog_color,
            zbuf,
            prim: self.prim,
            fog: (ee::to_int(ee::mul(fog, K16)) as u32 >> 4) as u8,
            verts,
        })
    }

    /// `ccEff::Draw(pat)` (main 0x0013bcb0) and the VU1 program
    /// `mc_DrawEff` into `layer`'s sorted group, keyed as `ccDLSort::Add`
    /// gets it.
    pub fn render(&self, assets: &Assets, layers: &mut Layers, layer: i16, pat: u16, camera: &Camera) {
        let Some(Packet { key, gs: Some(g), .. }) = self.packet(assets, pat, camera) else { return };
        let stem = &assets.files[self.file].stem;
        layers.sorted(layer, f32::from_bits(key), Cmd::Prim(g.prim(stem)));
    }
}

fn compare(v: u64) -> Compare {
    [
        Compare::Never,
        Compare::Always,
        Compare::Less,
        Compare::LEqual,
        Compare::Equal,
        Compare::GEqual,
        Compare::Greater,
        Compare::NotEqual,
    ][(v & 7) as usize]
}

impl GsSprite {
    /// As a [`Prim`] in frame-buffer pixels (XYOFFSET taken off), texels
    /// (ST times the texture's size: Q is 1), the state from ALPHA_1,
    /// TEST_1, ZBUF_1, TEX1_1 and CLAMP_1. FOG is dropped.
    pub fn prim(&self, stem: &str) -> Prim {
        let (tw, th) = self.tex.map_or((0, 0), |t| (t.tw, t.th));
        let c = (self.rgbaq as u32).to_le_bytes();
        let verts = self
            .verts
            .iter()
            .map(|v| Vertex {
                x: (i32::from(v.x) - XYOFFSET_X) as f32 / 16.0,
                y: (i32::from(v.y) - XYOFFSET_Y) as f32 / 16.0,
                z: v.z,
                u: f32::from_bits(v.s) * (1u32 << tw) as f32,
                v: f32::from_bits(v.t) * (1u32 << th) as f32,
                rgba: Rgba(c),
            })
            .collect();
        let t = self.test;
        let alpha_test = if t & 1 == 0 {
            AlphaTest::Off
        } else {
            let fail =
                [AlphaFail::Keep, AlphaFail::FbOnly, AlphaFail::ZbOnly, AlphaFail::RgbOnly][(t >> 12 & 3) as usize];
            AlphaTest::On { method: compare(t >> 1), reference: (t >> 4) as u8, fail }
        };
        let depth = Depth {
            test: [ZTest::Never, ZTest::Always, ZTest::GEqual, ZTest::Greater][(t >> 17 & 3) as usize],
            write: self.zbuf >> 32 & 1 == 0,
        };
        let texture = (self.prim & 0x10 != 0).then_some(self.tex).flatten().map(|x| TexState {
            tex: TexRef::Ccs { file: stem.to_string(), texture: x.texture, clut: x.clut },
            func: TexFunc::Modulate,
            use_alpha: true,
            filter: if self.tex1 >> 5 & 1 != 0 { Filter::Linear } else { Filter::Nearest },
            wrap: if self.clamp & 3 == 1 { Wrap::Clamp } else { Wrap::Repeat },
        });
        Prim {
            kind: match self.prim & 7 {
                3 => PrimKind::Triangles,
                5 => PrimKind::Fan,
                6 => PrimKind::Sprite,
                _ => PrimKind::Strip,
            },
            gouraud: self.prim & 8 != 0,
            state: DrawState {
                blend: (self.prim & 0x40 != 0).then(|| Blend::from_reg(self.alpha)),
                alpha_test,
                depth,
                texture,
                scissor: Scissor::FULL,
            },
            verts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_as_reset_and_mac_anu_leave_it() {
        let e = DrawEnv::default();
        // 2.55 * 100 truncates to 254.99998 at every depth: FOG 254.
        assert_eq!((e.fog_min, e.fog_max, e.fog_a), (0x437e_ffff, 0x437e_ffff, 0));
        let mut t = DrawEnv::default();
        t.set_fog(ee::k(1000.0), ee::k(7500.0), 0, ee::k(85.0), 0x0014_4870);
        assert!((ee::f(t.fog_min) - 38.25).abs() < 1e-4);
        assert!((ee::f(t.fog_b) + ee::f(t.fog_a) * 1000.0 - 255.0).abs() < 1e-3);
        assert_eq!(t.fog_color, 0x14_4870);
    }

    #[test]
    fn a_spark_in_front_of_the_camera() {
        let Some(fx) = crate::testing::effects() else { return };
        let a = &fx.assets;
        let p = a.file_index("particle").unwrap();
        // The field's view from (0, -1000, 0) looking at the origin.
        let wv = piney_world::camera::pos_target([0, ee::k(-1000.0), 0, ONE], [0, 0, 0, ONE]);
        let camera = Camera {
            world_view: wv,
            world_screen: piney_data::anim::vu_mul(&piney_world::camera::VIEW_SCREEN, &wv),
            ..Camera::default()
        };
        let mut e = Eff::init(p, 0, &a.effs[p][0], false, &a.alpha_blend);
        e.pos = [0, 0, 0, ONE];
        let k = e.packet(a, 0, &camera).unwrap();
        let g = k.gs.unwrap();
        // EFF_x000: a 40 x 40 square round the centre, opaque (128),
        // the centre 1000 away.
        assert!((ee::f(k.matrix[3][3]) - 1000.0).abs() < 1e-3);
        assert_eq!(g.rgbaq >> 24 & 0xff, 128);
        let (x0, x1) = (i32::from(g.verts[0].x), i32::from(g.verts[1].x));
        assert!((x0 + x1 - 2 * 2048 * 16).abs() <= 2, "{x0} {x1}");
        assert_eq!((g.verts[0].s, g.verts[3].s), (0, ONE));
        // Behind the eye, or with the transparency below 0: nothing.
        e.pos = [0, ee::k(-1500.0), 0, ONE];
        assert_eq!(e.packet(a, 0, &camera), None);
        e.pos = [0, 0, 0, ONE];
        e.transparency = ee::k(-0.5);
        assert_eq!(e.packet(a, 0, &camera), None);
        // Rendered into the layer's sorted group as one strip.
        e.transparency = ONE;
        let mut layers = Layers::default();
        e.render(a, &mut layers, 20, 0, &camera);
        let cmds = layers.flatten();
        assert!(matches!(&cmds[..], [Cmd::Prim(Prim { kind: PrimKind::Strip, .. })]));
    }

    #[test]
    fn fview_faces_the_camera() {
        // Looking down -y from (0, 500, 0): the sprite's x stays the
        // view's x, its y turns to the world's z (up the screen).
        let wv = piney_world::camera::pos_target([0, ee::k(500.0), 0, ONE], [0, 0, 0, ONE]);
        let f = fview(&wv);
        let r = apply(&vu::mul(&wv, &f), [ONE, 0, 0, 0]);
        assert!((ee::f(r[0]) - 1.0).abs() < 1e-6, "{r:x?}");
        let r = apply(&vu::mul(&wv, &f), [0, ONE, 0, 0]);
        assert!((ee::f(r[1]) + 1.0).abs() < 1e-6, "{r:x?}");
        assert_eq!(f[3], [0, 0, 0, ONE]);
    }
}
