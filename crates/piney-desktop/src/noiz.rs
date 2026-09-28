//! The screen breaking up: main's frame-buffer effects and `ccNoiz`
//! (0x001bb080), which drives them for the field menu (`docs/engine/stream.md`,
//! "Effects"; `docs/engine/field-ui.md`, "The noise"): `ccRasterNoize`
//! ([`Band`], up to 32 frame rows each shifted sideways), `ccBufferSampling`
//! ([`Sampling`], the last frame a little larger, blended over) and
//! `ccBufferReverce` ([`reverse_prim`], the frame inverted). [`Noiz`] keeps
//! eight bands, a sampling and an inversion on [`NOIZ_LAYER`]; `ccMenuCtrl`'s
//! (+0xe8) is the field menu's; `ccGameOverNoise`'s is not ported.

use piney_data::anim::ee;
use piney_draw::{
    AlphaFail, AlphaTest, Blend, Cmd, Compare, Depth, DrawState, Filter, Prim, PrimKind, Rgba, TexFunc, TexRef,
    TexState, Vertex, Wrap, ZTest,
};

use crate::view::{Frame, LayerView, XYOFFSET_X, XYOFFSET_Y};

/// sysLayer's view, which the effects' layers share (`ccLayer::Init(pri,
/// sysLayer's view)`, `SetFrame(0, 0, 512, 384, ...)`): `bboxClipMin` /
/// `Max` in GS pixels, the frame buffer around (2048, 2048).
pub const BBOX_MIN: [f32; 2] = [1792.0, 1824.0];
pub const BBOX_MAX: [f32; 2] = [2304.0, 2272.0];

/// What the effects read of their layer's view: `layer_screen`'s scales,
/// the scissor and `bboxClipMin` / `Max`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffView {
    pub layer: LayerView,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl EffView {
    /// A view `SetFrame` gave `frame`.
    pub fn of(frame: &Frame) -> EffView {
        let (min, max) = frame.bbox();
        EffView { layer: frame.layer(), min, max }
    }

    /// sysLayer's, the default frame.
    pub fn sys() -> EffView {
        EffView { layer: LayerView::default_layer(), min: BBOX_MIN, max: BBOX_MAX }
    }
}

/// `ccRasterNoize::MakePacket`'s rows go at Z 0xffffffff (the sampling's
/// at the view's `z / w` for its depth: for the reflex's 0 a division by
/// zero on the EE, the largest float, saturated by `vftoi4`).
const BAND_Z: u32 = 0xffff_ffff;
/// `ccBufferReverce::MakePacket`'s Z: 0xffffffff, the far plane.
const REVERSE_Z: u32 = 0xffff_ffff;

/// `ccRasterNoize::Init`'s band height (24 lines of 384, 27 frame rows) and
/// noise amplitude (32 pixels).
const BAND_INIT_H: i32 = 24;
const BAND_INIT_AMP: i32 = 32;
/// Rows a band's texture holds: `ccMakePacketDrawBuffTrans(tbp, 8, psm, 512,
/// 32, ...)` copies 512 x 32 from the band's first row.
const BAND_COPY_W: u16 = 512;
const BAND_COPY_H: u16 = 32;

/// `vftoi0` / `cvt.w.s`: truncate, saturating.
fn ftoi(v: u32) -> i32 {
    ee::to_int(v)
}

fn f(v: f32) -> u32 {
    v.to_bits()
}

/// Logical lines (of 384) to frame rows: `vftoi0(ls11 * v) >> 4`.
pub fn scale_y(v: i32) -> i32 {
    scale_y_in(&EffView::sys(), v)
}

/// As [`scale_y`] through `view`'s `ls11`.
pub fn scale_y_in(view: &EffView, v: i32) -> i32 {
    ftoi(ee::mul(view.layer.sy.to_bits(), ee::from_int(v))) >> 4
}

/// One `ccRasterNoize` (0x54 bytes: layer, `short y, h`, colour, tbp0, tbw,
/// tpsm, `short ampli[32]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Band {
    /// First frame row and rows (`SetYH`: logical lines scaled to 448).
    pub y: i16,
    pub h: i16,
    /// Per row, the horizontal offset in 1/16 pixel (`SetNoize`).
    pub ampli: [i16; 32],
}

impl Band {
    /// `ccRasterNoize::Init` (0x00106170): y 0, 27 rows, then `SetNoize(32)`,
    /// 27 numbers from `rand`.
    pub fn new(rand: &mut dyn FnMut() -> i32) -> Band {
        Band::new_in(&EffView::sys(), rand)
    }

    /// As [`Band::new`] on a layer with `view`.
    pub fn new_in(view: &EffView, rand: &mut dyn FnMut() -> i32) -> Band {
        let mut b = Band { y: 0, h: scale_y_in(view, BAND_INIT_H).min(32) as i16, ampli: [0; 32] };
        b.set_noize_in(view, BAND_INIT_AMP, rand);
        b
    }

    /// `SetNoize(max)` (0x00106320): each row `rand() % 2m - m`, m the
    /// amplitude in 1/16 pixel.
    pub fn set_noize(&mut self, max: i32, rand: &mut dyn FnMut() -> i32) {
        self.set_noize_in(&EffView::sys(), max, rand);
    }

    /// As [`Band::set_noize`] through `view`'s `ls00`.
    pub fn set_noize_in(&mut self, view: &EffView, max: i32, rand: &mut dyn FnMut() -> i32) {
        let m = ftoi(ee::mul(view.layer.sx.to_bits(), ee::from_int(max)));
        for a in self.ampli.iter_mut().take(self.h.max(0) as usize) {
            *a = (rand() % (2 * m) - m) as i16;
        }
    }

    /// `SetYH` (0x001061e0).
    pub fn set_yh(&mut self, y: i32, h: i32) {
        self.set_yh_in(&EffView::sys(), y, h);
    }

    /// As [`Band::set_yh`] through `view`.
    pub fn set_yh_in(&mut self, view: &EffView, y: i32, h: i32) {
        self.y = scale_y_in(view, y) as i16;
        self.h = scale_y_in(view, h).min(32) as i16;
    }

    /// `SetY` (0x00106280).
    pub fn set_y(&mut self, y: i32) {
        self.set_y_in(&EffView::sys(), y);
    }

    /// As [`Band::set_y`] through `view`.
    pub fn set_y_in(&mut self, view: &EffView, y: i32) {
        self.y = scale_y_in(view, y) as i16;
    }

    /// `ccRasterNoize::MakePacket` (0x001063d0): the band's 32 rows copied
    /// from the frame buffer (`ccMakePacketDrawBuffTrans`), then one sprite
    /// a row, shifted by its offset, textured from the copy at the row's own
    /// V: the copy's 32 rows repeat, so row `y + i` shows copy row
    /// `(y + i) mod 32`.
    pub fn prim(&self) -> Prim {
        self.prim_in(&EffView::sys())
    }

    /// As [`Band::prim`] on a layer with `view`: its scissor and clip box.
    pub fn prim_in(&self, view: &EffView) -> Prim {
        let sc = view.layer.scissor;
        let tex = TexState {
            tex: TexRef::FrameBuffer {
                x: sc.x0 as i16,
                y: (i32::from(sc.y0) + i32::from(self.y)) as i16,
                width: BAND_COPY_W,
                height: BAND_COPY_H,
            },
            func: TexFunc::Modulate,
            use_alpha: false,
            filter: Filter::Nearest,
            wrap: Wrap::Repeat,
        };
        let width = f32::from(sc.x1 - sc.x0 + 1);
        let (x0, x1) = (view.min[0] * 16.0, view.max[0] * 16.0);
        let mut verts = Vec::with_capacity(2 * self.h.max(0) as usize);
        for i in 0..self.h.max(0) {
            let row = i32::from(self.y) + i32::from(i);
            let dx = f32::from(self.ampli[i as usize]);
            let v = |x: f32, r: i32, u: f32| Vertex {
                x: (x + dx - XYOFFSET_X as f32) / 16.0,
                y: (view.min[1] * 16.0 + (r * 16) as f32 - XYOFFSET_Y as f32) / 16.0,
                z: BAND_Z,
                u,
                v: r as f32,
                rgba: Rgba::NEUTRAL,
            };
            verts.push(v(x0, row, 0.0));
            verts.push(v(x1, row + 1, width));
        }
        Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state: DrawState {
                blend: None,
                alpha_test: AlphaTest::Off,
                depth: Depth::NONE,
                texture: Some(tex),
                scissor: sc,
            },
            verts,
        }
    }
}

/// `ccBufferSampling` as `SetReflex(scale, roll, colour)` (0x00106bb0:
/// CLAMP_1 5, page 1, the last frame), `SetShade(type, z, colour)`
/// (0x00106bf0: page 0, the picture being drawn, read in `type`-texel
/// blocks) or `SetShade(page, x, y, tw, th, z, colour)` (0x00106c70: a
/// texture of its own, `texuse` +0x2e 1, into which the picture being drawn
/// is first copied at `2^tw` x `2^th`) leaves it, with what a task sets
/// after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sampling {
    /// The scale about the view's centre (f32 bits), both axes (+0x10,
    /// +0x14).
    pub scale: u32,
    /// The turn about the view's centre (radians, f32 bits; +0x0c).
    pub roll: u32,
    /// `color` (+0x28): RGBA with R in the low byte; A is the blend's
    /// weight.
    pub colour: u32,
    /// +0x18, +0x1c: the centre the strip is placed about, less, in half
    /// view sizes; +0x20, +0x24: an offset in sixteenths of the view's
    /// width and height (`MakePacket` adds it in 1/16 pixel unscaled); f32
    /// bits.
    pub centre: [u32; 2],
    pub offset: [u32; 2],
    /// +0x08: the depth the strip is drawn at (f32 bits): its Z is the
    /// view's `z / w` there (0, the reflex's: a division by zero, the far
    /// plane).
    pub z: u32,
    /// +0x2c: 1 the last frame's picture, 0 the one being drawn.
    pub page: u8,
    /// +0x00: CLAMP_1.
    pub wrap: Wrap,
    /// +0x2e `texuse` 1: the texture's width and height as powers of two
    /// (+0x32, +0x34); None for 0, the frame read in place.
    pub own: Option<(u8, u8)>,
}

/// Page 0: the draw buffer itself, as it is when the strip starts
/// drawing.
pub const CURRENT_FRAME: TexRef =
    TexRef::FrameBuffer { x: 0, y: 0, width: piney_draw::SCREEN_WIDTH, height: piney_draw::SCREEN_HEIGHT };

/// `ccSetViewScreenClipMatrix`'s view-screen Z terms for the views' Z
/// range (1 to 2^28, near 8, far 2^20): Z = `CZ + AZ / w`.
const VIEW_CZ: u32 = 0xc4ff_e07f;
const VIEW_AZ: u32 = 0x4f00_003f;

impl Default for Sampling {
    fn default() -> Self {
        Sampling::reflex(f(1.0), 0, 0)
    }
}

impl Sampling {
    /// `SetReflex(scale, roll, colour)`: the last frame's picture, clamped,
    /// at the far plane.
    pub fn reflex(scale: u32, roll: u32, colour: u32) -> Sampling {
        Sampling { scale, roll, colour, centre: [0; 2], offset: [0; 2], z: 0, page: 1, wrap: Wrap::Clamp, own: None }
    }

    /// `SetShade(t, z, colour)`: the picture being drawn, read in blocks of
    /// `t` texels (REGION_REPEAT, mask `1024 - t`, fixed `t / 2`), at depth
    /// `z`, scale 1.
    pub fn shade(t: i32, z: u32, colour: u32) -> Sampling {
        let wrap = Wrap::Region { mask: (1024 - t) as u16 & 0x3ff, fix: (t >> 1) as u16 & 0x3ff };
        Sampling { scale: f(1.0), roll: 0, colour, centre: [0; 2], offset: [0; 2], z, page: 0, wrap, own: None }
    }

    /// `SetShade(page, x, y, tw, th, z, colour)` (the field's depth shades,
    /// `WORLD_MAN::WORLD_MAN`): CLAMP_1 5, scale 1, a texture of its own at
    /// (`x`, `y`) of buffer `page` sized `2^tw` x `2^th`. `MakePacket` first
    /// copies the whole picture being drawn into it with a context-2 strip
    /// (bilinear, no test, no blend, Z masked), then draws it back over the
    /// view's box as [`Sampling::prim`] does, its UVs the texture's size.
    pub fn shade_own(tw: u8, th: u8, z: u32, colour: u32) -> Sampling {
        let own = Some((tw, th));
        Sampling { scale: f(1.0), roll: 0, colour, centre: [0; 2], offset: [0; 2], z, page: 0, wrap: Wrap::Clamp, own }
    }

    /// The strip's Z: `(0, 0, z, 1)` through the view-screen matrix
    /// (`sceVu0ApplyMatrix`), `z / w`, `vftoi4`.
    fn depth(&self) -> u32 {
        let zz = ee::add(ee::add(ee::add(ee::mul(0, 0), ee::mul(0, 0)), ee::mul(VIEW_CZ, self.z)), VIEW_AZ);
        let w = ee::add(ee::add(ee::add(ee::mul(0, 0), ee::mul(0, 0)), ee::mul(f(1.0), self.z)), ee::mul(0, f(1.0)));
        ftoi(ee::mul(ee::div(zz, w), f(16.0))) as u32
    }

    /// `ccBufferSampling::MakePacket` (0x00106dd0) without a texture of its
    /// own (`texuse` 0, page 1): a strip over the view's box scaled about
    /// its centre, textured with the whole of the previous frame's picture
    /// (bilinear, clamped), blended `(Cs - Cd) As + Cd` at the colour's
    /// alpha; TEST NEVER / FB_ONLY, Z GEQUAL at the largest Z, no Z write.
    pub fn prim(&self) -> Prim {
        self.prim_in(&EffView::sys())
    }

    /// As [`Sampling::prim`] on a layer with `view`.
    pub fn prim_in(&self, view: &EffView) -> Prim {
        let sc = view.layer.scissor;
        let r =
            [i32::from(sc.x0) << 4, i32::from(sc.y0) << 4, (i32::from(sc.x1) + 1) << 4, (i32::from(sc.y1) + 1) << 4];
        let uv = match self.own {
            // The own texture's whole extent, in 1/16 texels.
            Some((tw, th)) => {
                let (w, h) = (16 << tw, 16 << th);
                [(0, 0), (w, 0), (0, h), (w, h)]
            }
            None => [(r[0], r[1]), (r[2], r[1]), (r[0], r[3]), (r[2], r[3])],
        };
        let (min, max) = (view.min.map(f), view.max.map(f));
        let w = ee::sub(max[0], min[0]);
        let h = ee::sub(max[1], min[1]);
        let (sx, sy) = (self.scale, self.scale);
        let ([cx, cy], [ox, oy]) = (self.centre, self.offset);
        let eight = f(8.0);
        let m00 = ee::mul(eight, ee::mul(sx, w));
        // fptodp, dpmul by 8.0, dptofp: exact.
        let m11 = f((f64::from(f32::from_bits(ee::mul(sy, h))) * 8.0) as f32);
        // sceVu0RotMatrixZ by the roll, then the translation.
        let one = f(1.0);
        let m = piney_data::anim::rot_z_bits_of(
            [[m00, 0, 0, 0], [0, m11, 0, 0], [0, 0, one, 0], [0, 0, 0, one]],
            self.roll,
        );
        let m30 = ee::add(ee::mul(eight, ee::add(max[0], min[0])), ee::mul(w, ox));
        let m31 = ee::add(ee::mul(eight, ee::add(max[1], min[1])), ee::mul(h, oy));
        let rgba = Rgba(self.colour.to_le_bytes());
        let z = self.depth();
        // Corners (-1, -1), (1, -1), (-1, 1), (1, 1) less the centre.
        let corner = |k: usize| {
            let side = |bit: usize| if k & bit == 0 { f(-1.0) } else { f(1.0) };
            let (vx, vy) = (ee::sub(side(1), cx), ee::sub(side(2), cy));
            // sceVu0ApplyMatrix: vmulax, vmadday, vmaddaz, vmaddw.
            let x = ee::add(ee::add(ee::add(ee::mul(m[0][0], vx), ee::mul(m[1][0], vy)), ee::mul(m[2][0], 0)), m30);
            let y = ee::add(ee::add(ee::add(ee::mul(m[0][1], vx), ee::mul(m[1][1], vy)), ee::mul(m[2][1], 0)), m31);
            let (gx, gy) = (ftoi(x) - 8, ftoi(y) - 8);
            Vertex {
                x: (gx - XYOFFSET_X) as f32 / 16.0,
                y: (gy - XYOFFSET_Y) as f32 / 16.0,
                z,
                u: (uv[k].0 & 0x3fff) as f32 / 16.0,
                v: (uv[k].1 & 0x3fff) as f32 / 16.0,
                rgba,
            }
        };
        Prim {
            kind: PrimKind::Strip,
            gouraud: false,
            state: DrawState {
                blend: Some(Blend::TABLE[0]),
                alpha_test: AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::FbOnly },
                depth: Depth { test: ZTest::GEqual, write: false },
                texture: Some(TexState {
                    tex: match self.own {
                        Some((tw, th)) => TexRef::ScaledFrame { width: 1 << tw, height: 1 << th },
                        None if self.page == 1 => TexRef::PreviousFrame,
                        None => CURRENT_FRAME,
                    },
                    func: TexFunc::Modulate,
                    use_alpha: false,
                    filter: Filter::Linear,
                    wrap: self.wrap,
                }),
                scissor: sc,
            },
            verts: (0..4).map(corner).collect(),
        }
    }
}

/// `ccBufferReverce::MakePacket` (0x00106890): one untextured white sprite
/// over the view's box, blended `(Cs - Cd) FIX + 0` with FIX 0x80 - the
/// frame inverted; alpha NEVER with FB_ONLY, no Z test, no Z write, at the
/// far Z.
pub fn reverse_prim() -> Prim {
    reverse_prim_in(&EffView::sys())
}

/// As [`reverse_prim`] on a layer with `view`.
pub fn reverse_prim_in(view: &EffView) -> Prim {
    let v = |x: f32, y: f32| Vertex {
        x: (x * 16.0 - XYOFFSET_X as f32) / 16.0,
        y: (y * 16.0 - XYOFFSET_Y as f32) / 16.0,
        z: REVERSE_Z,
        u: 0.0,
        v: 0.0,
        rgba: Rgba([0xff, 0xff, 0xff, 0x80]),
    };
    Prim {
        kind: PrimKind::Sprite,
        gouraud: false,
        state: DrawState {
            blend: Some(Blend::from_reg(0x80_0000_00a4)),
            alpha_test: AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::FbOnly },
            depth: Depth::NONE,
            texture: None,
            scissor: view.layer.scissor,
        },
        verts: vec![v(view.min[0], view.min[1]), v(view.max[0], view.max[1])],
    }
}

/// `ccNoiz`'s own layer: `ccLayer::Init(241, sysLayer's view)`, over the
/// font layer (240).
pub const NOIZ_LAYER: i16 = 241;
/// `SetNoiz`'s sound when any part is switched on.
pub const NOIZ_SE: i32 = 94;
/// `SetReflex(1.0284f, 0, colour)` (0x3f839581), and the colour
/// `SetNoizBs` gives: alpha 0x50.
const NOIZ_SCALE: u32 = 0x3f83_9581;
const NOIZ_COLOUR: u32 = 0x5080_8080;
/// The sampling's fade after `SetNoizBs`'s frames: 6 frames of alpha
/// `80 n / 6`.
const NOIZ_FADE: i32 = 6;

/// `ccNoiz` (0x20 bytes): frame counters for the bands (+0), the sampling
/// (+4) and its fade (+8), and the inversion (+0xc); the layer (+0x10),
/// eight bands (+0x14, 96 bytes each: a `ccRasterNoize` and three words
/// `Draw` never reads), the sampling (+0x18), the inversion (+0x1c).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Noiz {
    pub rn: i32,
    pub bs: i32,
    pub fade: i32,
    pub br: i32,
    pub bands: Vec<Band>,
    pub sampling: Sampling,
}

impl Noiz {
    /// The constructor (0x001bb080): eight bands `Init` (216 numbers from
    /// `rand`) and `SetTex(896, 0)`; the sampling `Init` (scale 1, colour
    /// 0x80808080); every counter 0.
    pub fn new(rand: &mut dyn FnMut() -> i32) -> Noiz {
        Noiz {
            rn: 0,
            bs: 0,
            fade: 0,
            br: 0,
            bands: (0..8).map(|_| Band::new(rand)).collect(),
            sampling: Sampling::reflex(f(1.0), 0, 0x8080_8080),
        }
    }

    /// Band `i` somewhere new: `rand() % 50 + 48 i` for its line and
    /// `rand() % 11 + 2` for its height, then one more number (`rand() & 31
    /// - 16`, kept in a word `Draw` never reads).
    fn restart(&mut self, rand: &mut dyn FnMut() -> i32) {
        for (i, b) in self.bands.iter_mut().enumerate() {
            let y = rand() % 50 + 48 * i as i32;
            let h = rand() % 11 + 2;
            let _drift = (rand() & 31) - 16;
            b.set_yh(y, h);
        }
    }

    /// `SetNoizRn(n)` (0x001bb2e0): n > 0 the bands for n frames, each
    /// somewhere new; 0 off; below 0 nothing.
    pub fn set_rn(&mut self, n: i32, rand: &mut dyn FnMut() -> i32) {
        if n > 0 {
            self.rn = n;
            self.restart(rand);
        } else if n == 0 {
            self.rn = 0;
        }
    }

    /// `SetNoizBs(n)` (0x001bb410): n > 0 the sampling for n frames at
    /// alpha 0x50, then its fade; 0 off (the fade still runs); below 0
    /// nothing.
    pub fn set_bs(&mut self, n: i32) {
        if n > 0 {
            self.bs = n;
            self.fade = NOIZ_FADE;
            self.sampling = Sampling::reflex(NOIZ_SCALE, 0, NOIZ_COLOUR);
        } else if n == 0 {
            self.bs = 0;
        }
    }

    /// `SetNoizBr(n)` (0x001bb480): n >= 0 the inversion for n frames;
    /// below 0 nothing.
    pub fn set_br(&mut self, n: i32) {
        if n >= 0 {
            self.br = n;
        }
    }

    /// `SetNoiz(a, b, c)` (0x001bb4b0): `SetNoizRn(a)`, `SetNoizBs(b)`,
    /// `SetNoizBr(c)`. True when any is nonzero: the game plays sound
    /// [`NOIZ_SE`] first.
    pub fn set_noiz(&mut self, a: i32, b: i32, c: i32, rand: &mut dyn FnMut() -> i32) -> bool {
        self.set_rn(a, rand);
        self.set_bs(b);
        self.set_br(c);
        a != 0 || b != 0 || c != 0
    }

    /// Anything to draw on the next [`Noiz::draw`].
    pub fn on(&self) -> bool {
        self.rn != 0 || self.bs != 0 || self.fade != 0 || self.br != 0
    }

    /// `Draw` (0x001bb690): each running counter down a frame (not below 0)
    /// and its effect's packets on [`NOIZ_LAYER`]: the bands; the sampling,
    /// or once its frames are out its fade (`SetReflex` at alpha `80 fade /
    /// 6`, then the fade down one); the inversion. The commands come back
    /// in the order the GS draws them: the layer takes each packet at its
    /// front, so the inversion first, then the sampling, then band 7 to 0.
    pub fn draw(&mut self) -> Vec<Cmd> {
        let mut sent = Vec::new();
        if self.rn != 0 {
            self.rn = (self.rn - 1).max(0);
            sent.extend(self.bands.iter().map(|b| Cmd::Prim(b.prim())));
        }
        if self.bs != 0 {
            self.bs = (self.bs - 1).max(0);
            sent.push(Cmd::Prim(self.sampling.prim()));
        } else if self.fade != 0 {
            let alpha = (self.fade * 80 / 6) as u32;
            self.sampling = Sampling::reflex(NOIZ_SCALE, 0, alpha << 24 | 0x0080_8080);
            sent.push(Cmd::Prim(self.sampling.prim()));
            self.fade -= 1;
        }
        if self.br != 0 {
            self.br = (self.br - 1).max(0);
            sent.push(Cmd::Prim(reverse_prim()));
        }
        sent.reverse();
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn newlib() -> impl FnMut() -> i32 {
        let mut next: u64 = 1;
        move || {
            next = next.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (next >> 32 & 0x7fff_ffff) as i32
        }
    }

    #[test]
    fn bands_are_27_rows() {
        let mut r = newlib();
        let n = Noiz::new(&mut r);
        assert!(n.bands.iter().all(|b| b.h == 27 && b.y == 0));
    }

    /// `SetNoiz(0, 8, 0)`: 8 frames at 0x50, then 6 of the fade (80 n / 6),
    /// then nothing.
    #[test]
    fn sampling_fades() {
        let mut r = newlib();
        let mut n = Noiz::new(&mut r);
        assert!(n.set_noiz(0, 8, 0, &mut r));
        let mut alphas = Vec::new();
        while n.on() {
            let cmds = n.draw();
            assert_eq!(cmds.len(), 1);
            alphas.push(n.sampling.colour >> 24);
        }
        assert_eq!(alphas, [0x50, 0x50, 0x50, 0x50, 0x50, 0x50, 0x50, 0x50, 80, 66, 53, 40, 26, 13]);
    }

    /// The field's depth shades (`SetShade(0, 896, 0, 8, 7, 3000, ...)` and
    /// `(0, 896, 0, 7, 6, 6000, ...)`) as the game's `MakePacket` drew them
    /// on sysLayer's view in eemu: the draw back's strip over the view's
    /// box at the depth's Z, its UVs the own texture's size, the frame
    /// scaled into it.
    #[test]
    fn the_field_shades_draw_their_own_texture_back() {
        for (tw, th, z, gz, texture) in
            [(8, 7, 3000.0f32, 0x00ae_43a3u32, (256, 128)), (7, 6, 6000.0, 0x0056_e1d9, (128, 64))]
        {
            let p = Sampling::shade_own(tw, th, z.to_bits(), 0x4880_8080).prim();
            assert_eq!(p.kind, PrimKind::Strip);
            let gs = |x: u32, y: u32| ((x as i32 - XYOFFSET_X) as f32 / 16.0, (y as i32 - XYOFFSET_Y) as f32 / 16.0);
            let corners = [gs(0x6ff8, 0x71f8), gs(0x8ff8, 0x71f8), gs(0x6ff8, 0x8df8), gs(0x8ff8, 0x8df8)];
            let (w, h) = (texture.0 as f32, texture.1 as f32);
            let uvs = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)];
            for (k, v) in p.verts.iter().enumerate() {
                assert_eq!(((v.x, v.y), v.z, (v.u, v.v)), (corners[k], gz, uvs[k]), "corner {k} of {texture:?}");
                assert_eq!(v.rgba, Rgba([0x80, 0x80, 0x80, 0x48]));
            }
            let t = p.state.texture.unwrap();
            assert_eq!(
                (t.tex, t.wrap, t.filter),
                (TexRef::ScaledFrame { width: texture.0, height: texture.1 }, Wrap::Clamp, Filter::Linear)
            );
            assert_eq!(p.state.depth, Depth { test: ZTest::GEqual, write: false });
        }
    }

    /// The inversion draws first, then the sampling, then band 7 to 0.
    #[test]
    fn draw_order() {
        let mut r = newlib();
        let mut n = Noiz::new(&mut r);
        n.set_noiz(2, 2, 2, &mut r);
        let cmds = n.draw();
        assert_eq!(cmds.len(), 10);
        let Cmd::Prim(p) = &cmds[0] else { panic!() };
        assert!(p.state.texture.is_none());
        let Cmd::Prim(p) = &cmds[1] else { panic!() };
        assert_eq!(p.kind, PrimKind::Strip);
        let Cmd::Prim(p) = &cmds[9] else { panic!() };
        assert_eq!(p.verts.len() as i16, 2 * n.bands[0].h);
    }
}
