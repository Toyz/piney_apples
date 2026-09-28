//! One frame of drawing, as the game tells the GS to draw it.
//!
//! The game logic (for example `piney-desktop`) builds a [`Frame`] per game
//! frame and a renderer turns it into pixels. Nothing here does any drawing
//! or touches a GPU; the types mirror what the EE sends down PATH1/PATH3, so
//! the render can be exact:
//!
//! - [`Prim`]: an explicit GS primitive (SPRITE, triangle, strip or fan) with
//!   its vertices in frame-buffer pixels, as `ccSprite`, `ccMask`, `ccKanji`
//!   and the screen fader send them.
//! - [`ModelDraw`]: one CCSF model through the VU1 microcode, as
//!   `ccModel::Draw` sends it (`docs/engine/render.md`): the renderer reads
//!   the model and its textures from the named `DATA.BIN` member with
//!   `piney-data`, and applies the matrix and state given here.
//! - [`Upload`]: a texture the EE builds at run time and uploads to VRAM
//!   (the `ccKanji` text textures), referenced by [`TexRef::Upload`].
//! - The frame buffer itself as a texture: [`TexRef::FrameBuffer`], a
//!   rectangle of the picture as drawn so far this frame (screen effects
//!   that shift or copy rows of it), and [`TexRef::PreviousFrame`], the
//!   last frame's finished picture (feedback).
//!
//! Commands are in submission order: the GS draws [`Frame::cmds`] first to
//! last, with the blend, test and depth state each one carries.
//!
//! Units and conventions, all from the GS:
//! - Coordinates are frame-buffer pixels, with the primitive offset
//!   (XYOFFSET) already taken off: (0, 0) is the top-left pixel of the
//!   [`Frame::width`] x [`Frame::height`] buffer (512 x 448 for the whole
//!   game, `ccSystem::Init` 0x0010a900), which the display stretches to 4:3.
//!   The GS samples at pixel centres the way it rasterises: a pixel is drawn
//!   when its top-left corner lies inside the primitive (top-left rule).
//! - Colours are GS colours: 0x80 is 1.0 for RGB under MODULATE and for
//!   alpha, so a vertex colour of (0x80, 0x80, 0x80, 0x80) draws a texture
//!   unchanged and opaque. Values above 0x80 brighten.
//! - Texture coordinates are texels (the GS UV register, FST on), in the
//!   texture's stored row order (see [`TexRef`]).
//! - Depth is the GS Z value: larger is nearer (ZTST GEQUAL / GREATER).

/// One frame for the GS.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// The frame buffer the coordinates refer to: [`SCREEN_WIDTH`] x
    /// [`SCREEN_HEIGHT`].
    pub width: u16,
    pub height: u16,
    /// What the frame buffer is cleared to before the first command
    /// (`ccSystem.bgColor`); alpha is ignored.
    pub clear: Rgba,
    /// Textures built this frame, uploaded before the first command.
    pub uploads: Vec<Upload>,
    /// In the order the GS draws them.
    pub cmds: Vec<Cmd>,
}

/// `ccSystem::SetScreenMode(512, 448, 0)` from `ccSystem::Init`.
pub const SCREEN_WIDTH: u16 = 512;
pub const SCREEN_HEIGHT: u16 = 448;

impl Frame {
    pub fn new() -> Self {
        Frame { width: SCREEN_WIDTH, height: SCREEN_HEIGHT, ..Default::default() }
    }
}

/// R, G, B, A; 0x80 = 1.0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    /// (0x80, 0x80, 0x80, 0x80): texture colours unchanged, opaque.
    pub const NEUTRAL: Rgba = Rgba([0x80; 4]);
    pub const BLACK: Rgba = Rgba([0, 0, 0, 0x80]);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba([r, g, b, a])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Prim(Prim),
    Model(Box<ModelDraw>),
    Shadow(Box<ShadowPass>),
}

// State ----------------------------------------------------------------------

/// The GS state a command draws with: ALPHA_1 and PRIM.ABE, TEST_1, ZBUF_1,
/// and the texture registers.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawState {
    /// None: PRIM.ABE off, the source colour is written as it is.
    pub blend: Option<Blend>,
    pub alpha_test: AlphaTest,
    pub depth: Depth,
    /// None: untextured (PRIM.TME off).
    pub texture: Option<TexState>,
    /// SCISSOR_1: pixels outside are not drawn. Each layer's view sets it
    /// at the head of the layer's list (`ccLayer::AddAll`).
    pub scissor: Scissor,
}

impl DrawState {
    /// What `ccSprite::SetTag` (0x0015c0c0) sets for every 2D sprite: no
    /// alpha test, depth test ALWAYS with no depth write; the whole frame.
    pub fn sprite(blend: Blend, texture: Option<TexState>) -> Self {
        DrawState {
            blend: Some(blend),
            alpha_test: AlphaTest::Off,
            depth: Depth::NONE,
            texture,
            scissor: Scissor::FULL,
        }
    }
}

/// SCISSOR_1: inclusive frame-buffer pixel bounds (SCAX0..=SCAX1,
/// SCAY0..=SCAY1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scissor {
    pub x0: u16,
    pub x1: u16,
    pub y0: u16,
    pub y1: u16,
}

impl Scissor {
    /// The whole 512 x 448 frame, as the default view sets it.
    pub const FULL: Scissor = Scissor { x0: 0, x1: SCREEN_WIDTH - 1, y0: 0, y1: SCREEN_HEIGHT - 1 };
}

impl Default for Scissor {
    fn default() -> Self {
        Scissor::FULL
    }
}

/// The ALPHA register: `((A - B) * C >> 7) + D`, per colour channel, with
/// the result clamped (COLCLAMP) to 0..255. A, B and D pick the source
/// colour, the frame-buffer colour or 0; C picks the source alpha, the
/// frame-buffer alpha or [`Blend::fix`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Blend {
    pub a: BlendColour,
    pub b: BlendColour,
    pub c: BlendAlpha,
    pub d: BlendColour,
    pub fix: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendColour {
    /// Cs, the colour being drawn.
    Source = 0,
    /// Cd, the colour in the frame buffer.
    Dest = 1,
    Zero = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendAlpha {
    /// As.
    Source = 0,
    /// Ad.
    Dest = 1,
    /// FIX.
    Fix = 2,
}

impl Blend {
    /// `(Cs - Cd) * As + Cd`: `alphaBlendTbl[0]`, ordinary transparency.
    pub const MIX: Blend = Blend::from_reg(0x44);
    /// `Cs * As + Cd`: `alphaBlendTbl[1]`, additive.
    pub const ADD: Blend = Blend::from_reg(0x48);
    /// `Cd - Cs * As`: `alphaBlendTbl[2]`, subtractive.
    pub const SUB: Blend = Blend::from_reg(0x42);
    /// `Cd * As + Cs`: `alphaBlendTbl[3]` and `[4]`.
    pub const DEST_MUL: Blend = Blend::from_reg(0x09);
    /// `(Cs - Cd) * 0x80 + Cd`, i.e. Cs: `alphaBlendTbl[5]` and `[8]`.
    pub const REPLACE: Blend = Blend::from_reg(0x80_0000_0064);
    /// `Cs + Cd`: `alphaBlendTbl[6]`.
    pub const ADD_FIX: Blend = Blend::from_reg(0x80_0000_0068);
    /// `Cd - Cs`: `alphaBlendTbl[7]`.
    pub const SUB_FIX: Blend = Blend::from_reg(0x80_0000_0062);

    /// `alphaBlendTbl` (`INF SLUS_202.67:0x00348580`), the ALPHA values
    /// `ccSprite.alphaBlendType` and the model blend types index.
    pub const TABLE: [Blend; 9] = [
        Blend::MIX,
        Blend::ADD,
        Blend::SUB,
        Blend::DEST_MUL,
        Blend::DEST_MUL,
        Blend::REPLACE,
        Blend::ADD_FIX,
        Blend::SUB_FIX,
        Blend::REPLACE,
    ];

    /// From the 64-bit ALPHA register value.
    pub const fn from_reg(reg: u64) -> Blend {
        const fn colour(v: u64) -> BlendColour {
            match v & 3 {
                0 => BlendColour::Source,
                1 => BlendColour::Dest,
                _ => BlendColour::Zero,
            }
        }
        let c = match reg >> 4 & 3 {
            0 => BlendAlpha::Source,
            1 => BlendAlpha::Dest,
            _ => BlendAlpha::Fix,
        };
        Blend { a: colour(reg), b: colour(reg >> 2), c, d: colour(reg >> 6), fix: (reg >> 32) as u8 }
    }

    pub const fn to_reg(self) -> u64 {
        (self.a as u64) | (self.b as u64) << 2 | (self.c as u64) << 4 | (self.d as u64) << 6 | (self.fix as u64) << 32
    }
}

/// TEST_1's alpha test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AlphaTest {
    #[default]
    Off,
    /// Draw only where the source alpha passes `method` against `reference`;
    /// a failing pixel does what `fail` says.
    On { method: Compare, reference: u8, fail: AlphaFail },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compare {
    Never = 0,
    Always = 1,
    Less = 2,
    LEqual = 3,
    Equal = 4,
    GEqual = 5,
    Greater = 6,
    NotEqual = 7,
}

/// TEST.AFAIL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaFail {
    /// Nothing is written.
    Keep = 0,
    /// Only the colour is written.
    FbOnly = 1,
    /// Only depth is written.
    ZbOnly = 2,
    /// Colour without its alpha.
    RgbOnly = 3,
}

/// TEST_1's depth test and ZBUF_1's write mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Depth {
    /// ZTST: [`ZTest::Always`] for every 2D command.
    pub test: ZTest,
    /// Not ZBUF.ZMSK.
    pub write: bool,
}

impl Depth {
    /// Always passes, never writes: all 2D drawing.
    pub const NONE: Depth = Depth { test: ZTest::Always, write: false };
}

/// TEST.ZTST. Larger Z is nearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZTest {
    Never = 0,
    Always = 1,
    GEqual = 2,
    Greater = 3,
}

/// TEX0 / TEX1 / CLAMP for a textured command.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TexState {
    pub tex: TexRef,
    /// TEX0.TFX.
    pub func: TexFunc,
    /// TEX0.TCC: the texture's alpha is used (true), or the vertex alpha
    /// alone (false).
    pub use_alpha: bool,
    /// TEX1.MMAG / MMIN.
    pub filter: Filter,
    /// CLAMP_1.
    pub wrap: Wrap,
}

impl TexState {
    /// MODULATE, texture alpha on, nearest, repeat: what `ccSprite` and
    /// `ccKanji` draw with.
    pub fn modulate(tex: TexRef) -> Self {
        TexState { tex, func: TexFunc::Modulate, use_alpha: true, filter: Filter::Nearest, wrap: Wrap::Repeat }
    }
}

/// Where the texels come from.
///
/// Rows are in stored order, as `piney_data::texture::Texture::rgba` returns
/// them (the art is bottom-up `.bmp`, so v = 0 is the image's bottom row):
/// the game's texel coordinates index that order directly.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TexRef {
    /// A Texture chunk of a CCSF file in `DATA/DATA.BIN`, drawn through a
    /// palette: `file` is the member's stem (`xddesk01`), `texture` the TEX_
    /// object and `clut` the CLT_ object, both indices into that file's
    /// object table. The palette is normally the texture's own, but the game
    /// can draw a texture through another one (`ccAnm::ChangeClut`).
    Ccs { file: String, texture: u32, clut: u32 },
    /// A texture from [`Frame::uploads`], by [`Upload::id`].
    Upload(u32),
    /// The frame buffer as it is when this command starts drawing: the
    /// `width` x `height` rectangle at pixel (`x`, `y`), after the clear and
    /// every command before this one, copied to a texture of that size
    /// (texel (0, 0) is pixel (`x`, `y`), rows top first). Texels outside
    /// the frame buffer (the rectangle may start above or left of it) read
    /// as 0. The copy is made once per command, so all of a command's
    /// primitives see the same picture, not each other.
    ///
    /// The game copies the draw buffer into spare VRAM with a context-2
    /// sprite and textures from the copy (`ccMakePacketDrawBuffTrans`
    /// 0x00108a10, as `ccRasterNoize` does); a texture wrapped with
    /// [`Wrap::Repeat`] repeats at `width` and `height`, as a GS texture of
    /// that TW / TH does.
    FrameBuffer { x: i16, y: i16, width: u16, height: u16 },
    /// The previous frame's finished picture, [`Frame::width`] x
    /// [`Frame::height`]: the buffer the display shows while this frame is
    /// drawn (`fbuffAdrs[page]`, which `ccBufferSampling` page 1 textures
    /// from for its feedback). Black before the first frame. The GS
    /// declares it 512 x 512 and reads past row 447 into the next VRAM; here
    /// it ends at the frame buffer's last row.
    PreviousFrame,
    /// The whole frame buffer as it is when this command starts drawing,
    /// resampled into a `width` x `height` texture as a context-2 GS strip
    /// draws it (`ccBufferSampling::MakePacket` with a texture of its own,
    /// `texuse` 1): texel (i, j) is the frame read with bilinear filtering
    /// at ((i + 0.5) W / `width`, (j + 0.5) H / `height`) pixels, W x H the
    /// frame, clamped at its edges.
    ScaledFrame { width: u16, height: u16 },
}

/// TEX0.TFX.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexFunc {
    /// Colour = texture * vertex / 0x80, alpha likewise (with TCC).
    Modulate = 0,
    /// The texture as it is.
    Decal = 1,
    Highlight = 2,
    Highlight2 = 3,
}

/// TEX1.MMAG / MMIN (no mip levels in 2D).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    Nearest,
    Linear,
}

/// CLAMP_1.WMS / WMT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wrap {
    Repeat,
    Clamp,
    /// REGION_REPEAT (3) both ways: each texel coordinate ANDed with
    /// `mask` (MINU / MINV) and ORed with `fix` (MAXU / MAXV), so the
    /// texture is read in blocks (`ccBufferSampling::SetShade`'s shades).
    Region {
        mask: u16,
        fix: u16,
    },
}

// Uploads --------------------------------------------------------------------

/// A texture the EE builds and uploads itself, e.g. a line of text that
/// `ccKanji::Extract` rasterised.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Upload {
    /// What [`TexRef::Upload`] names; unique within the frame.
    pub id: u32,
    pub width: u16,
    pub height: u16,
    pub format: UploadFormat,
    /// Row-major from v = 0, in the format's packing: PSMT4 is two texels a
    /// byte, low nibble first.
    pub pixels: Vec<u8>,
    /// The palette, 16 or 256 entries.
    pub clut: Vec<Rgba>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UploadFormat {
    Psmt4,
    Psmt8,
    /// PSMCT32: true colour, four bytes a texel - R, G, B, A with A in GS
    /// units (0x80 = 1.0) - and no palette (the IPU's output, which the
    /// movie player uploads).
    Psmct32,
}

// Primitives -----------------------------------------------------------------

/// One GS primitive as the EE sends it.
#[derive(Clone, Debug, PartialEq)]
pub struct Prim {
    pub kind: PrimKind,
    /// PRIM.IIP: colours interpolate across the primitive (false: the last
    /// vertex's colour fills it, as the GS does for flat shading).
    pub gouraud: bool,
    pub state: DrawState,
    pub verts: Vec<Vertex>,
}

/// PRIM.PRIM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimKind {
    /// Pairs of vertices, each an axis-aligned rectangle from the first to
    /// the second; UV interpolates linearly, colour is the second vertex's.
    Sprite,
    Triangles,
    Strip,
    Fan,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    /// Frame-buffer pixels, in the GS's 1/16 steps.
    pub x: f32,
    pub y: f32,
    /// GS Z; unused by 2D.
    pub z: u32,
    /// Texels, in the GS's 1/16 steps.
    pub u: f32,
    pub v: f32,
    pub rgba: Rgba,
}

// Shadows --------------------------------------------------------------------

/// One `ccShadowPacket`'s frame (`ccShadowPacket::SetShadowPacket`, main
/// 0x00142b20; `docs/engine/shadow.md`), as the GS runs it where the command
/// stands:
///
/// 1. The Z buffer as drawn so far, read at `width` x `height` points over
///    [`ShadowPass::rect`] (texel `(x0 + i W / width, y0 + j H / height)`,
///    W x H the rectangle, unfiltered), becomes the buffer's own Z.
/// 2. For each group, the last first: the buffer counts from 0, each
///    polygon adding 1 ([`ShadowPoly::add`]) or taking 1 where its Z beats
///    the buffer's (ZTST GREATER, Z past 0x7fffffff held there, no Z
///    written); then each pixel counted above 0 takes the group's alpha. A
///    pixel no group counted keeps alpha 0; where groups overlap, the
///    first in the list wins.
/// 3. The buffer is laid over the rectangle once for each of
///    [`ShadowPass::taps`], moved by it, filtered bilinearly: the frame
///    goes towards black by `alpha * darkness / 0x80` (in 0x80 units) and
///    the frame buffer's alpha becomes that.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowPass {
    /// The buffer: `1 << tw` by `1 << th`.
    pub width: u16,
    pub height: u16,
    /// The frame-buffer pixels it covers: x0, y0, x1, y1
    /// (`ccView::GetScreenClip` with the XYOFFSET taken off).
    pub rect: [f32; 4],
    /// `ccShadowPacket` +0x15d, 0x80 = black.
    pub darkness: u8,
    /// Where the buffer is laid, in pixels off the rectangle, once each
    /// (`TransShadowTex`'s `zureTbl`): a blur of a few copies.
    pub taps: Vec<[f32; 2]>,
    /// By `ccDrawEnv`'s shadow alpha, in `ccShadowPacket`'s list order.
    pub groups: Vec<ShadowGroup>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShadowGroup {
    pub alpha: u8,
    pub polys: Vec<ShadowPoly>,
}

/// A convex polygon (a fan) in the buffer's pixels, with GS Z.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowPoly {
    pub verts: Vec<[f32; 3]>,
    /// Counts up (the add blend) or down (the subtract).
    pub add: bool,
}

// Models ---------------------------------------------------------------------

/// A 4x4 matrix, column-major (`m[column][row]`), applied to column vectors:
/// the layout of the EE's `sceVu0FMATRIX` and of `glam::Mat4::from_cols_array_2d`.
pub type Mat4 = [[f32; 4]; 4];

/// `view.divZ` as `ccLayer::Init` sets it (`ccSetStreamDemoThread` sets it
/// again): see [`ModelDraw::div_z`].
pub const DIV_Z: f32 = 1000.0;

/// VU1 writes a model vertex's XYZ with `ftoi4` after the divide by w
/// (`docs/engine/render.md`): its GS Z is 16 times `z / w`, on the same
/// scale as the Z the EE's own packets give their sprites and strips.
pub const MODEL_Z_SCALE: f32 = 16.0;

/// One CCSF model drawn through VU1, as `ccModel::Draw` (0x0013eab0) sends
/// it (`docs/engine/render.md`): a matrix packet, then a (material, model)
/// packet pair per mmat, in [`ModelDraw::mmats`] order.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDraw {
    /// The `DATA.BIN` member (stem) holding the model and its textures.
    pub file: String,
    /// The MDL_ object in that file.
    pub model: u32,
    /// Model space to the frame buffer: for a vertex position `p` as
    /// `piney_data::model::Model::position` gives it (vertex scale applied),
    /// `q = to_screen * (p, 1)` and the vertex lands at pixel
    /// `(q.x / q.w, q.y / q.w)` with GS Z `16 q.z / q.w` ([`MODEL_Z_SCALE`]:
    /// VU1 divides by w and converts the whole vector with `ftoi4`). This
    /// is VU1's `world_screen * local_world` with XYOFFSET taken off.
    pub to_screen: Mat4,
    /// For bone and skin mmats: clump node `i`'s matrix to the frame buffer,
    /// used in place of `to_screen` the same way. Empty for rigid models.
    pub nodes: Vec<Mat4>,
    /// The mmats drawn, in the order the GS gets them; mmats not listed are
    /// not drawn by this command.
    pub mmats: Vec<MmatDraw>,
    /// Blend, tests, depth and scissor for every mmat. `texture` is None:
    /// each mmat's material names its texture in the file. When
    /// `alpha_test` is on, its reference is each mmat's
    /// [`MmatDraw::alpha_ref`], not the one given here.
    pub state: DrawState,
    /// How every material's texture is sampled (wrap is per mmat).
    pub tex: TexParams,
    /// Palette replacements: (CLT_ drawn, CLT_ used instead), for
    /// `ccAnm::ChangeClut`.
    pub clut_swaps: Vec<(u32, u32)>,
    /// Texture replacements: (MAT_ whose texture is replaced, TEX_ used
    /// instead, with its own palette), for `ccClump::ChangeTex` (the
    /// walking PCs' `changeTEX`).
    pub tex_swaps: Vec<(u32, u32)>,
    /// PRIM.FGE with a constant fog coefficient; None: no fog (F = 0xff).
    pub fog: Option<Fog>,
    /// PRIM.FGE with VU1's fog by depth (each vertex's own F, interpolated
    /// across the triangle), when `fog` is None.
    pub depth_fog: Option<DepthFog>,
    /// None: the unlit programs, vertex colours as stored.
    pub lights: Option<Lights>,
    /// `ccMorpher` (the object's `MPH_` modifier): target models of the same
    /// file and their weights, blended into the rigid positions
    /// (`piney_data::model::Model::morph`). Empty: none.
    pub morph: Vec<(u32, f32)>,
    /// VU1 rejects a triangle when any of its vertices falls outside the GS
    /// primitive space (0..4095 before the offset) or the view's w range;
    /// the renderer should drop those triangles whole rather than clip them.
    pub reject_outside: bool,
    /// The view's `divZ` (+0x25c, [`DIV_Z`] unless a stream's task set
    /// another): the unlit rigid program cuts a triangle that is not wholly
    /// inside at the near plane when its last vertex is nearer than this,
    /// and drops it otherwise.
    pub div_z: f32,
    /// Vertex data the EE rewrote in the model's own packets before this
    /// draw; None: the model as the file has it.
    pub edits: Option<std::sync::Arc<VertexEdits>>,
}

/// What the EE wrote into a model's vertex data in memory, which every
/// later draw of it sends: a field's ground tiles and ground cover
/// (`FIELD_MESH::SetMESH2`, `FIELD::SetSmallMESH`: a height and a colour
/// per vertex), its objects' lit colours (`WORLD::Init`'s
/// `CalcObjectVertexColor`), and a town's water: the texture coordinates
/// `waterUVModifi2` writes and the texture `ccModel::ChangeTex` put in
/// place of its material's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VertexEdits {
    /// (mmat, vertex, the stored s16 z that replaces the vertex's own).
    pub z: Vec<(u32, u32, i16)>,
    /// (mmat, vertex, RGB that replaces the vertex's colour; alpha kept).
    pub rgb: Vec<(u32, u32, [u8; 3])>,
    /// (mmat, vertex, the stored ST, 256 a texture's width or height, that
    /// replaces the vertex's own).
    pub st: Vec<(u32, u32, [u16; 2])>,
    /// The texture every mmat is drawn with instead of its material's:
    /// the frame buffer as [`TexRef::FrameBuffer`] says, copied when the
    /// model's command starts drawing (the game's
    /// `ccLayer::MakePacketDrawBuffTrans` copy into the texture the water
    /// was given).
    pub tex: Option<TexRef>,
}

/// One mmat's material packet (`ccSetMaterialPacket` 0x0013e6c0) and the
/// per-mmat values the model packet carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MmatDraw {
    /// Index into `piney_data::model::Model::mmats`.
    pub index: u32,
    /// The transparency `t` VU1 draws with: the unlit programs write vertex
    /// alpha `trunc(A_vertex * t)`, the lit ones `trunc(0x80 * t)`.
    pub alpha: f32,
    /// TEST_1.AREF for this mmat.
    pub alpha_ref: u8,
    /// CLAMP_1 for this mmat's texture.
    pub wrap: Wrap,
    /// STROW: added to each vertex's stored S and T (texture coordinate =
    /// `(S + row) / 256` of the texture), the animated `ccMaterial::u/v`
    /// (`(u >> 4) & 0xff`).
    pub uv_row: [u8; 2],
}

/// Fog with a constant coefficient: `C = (Cs * f + colour * (0xff - f)) >> 8`
/// per channel (the GS fog blend), before alpha blending.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Fog {
    pub f: u8,
    /// FOGCOL.
    pub colour: [u8; 3],
}

/// The fog VU1 writes per vertex (`docs/engine/render.md`): F =
/// `clamp(fogB + fogA * w, fMin, fMax)` at the vertex's w, from
/// `ccDrawEnv::SetFog` (main 0x00105820), which the matrix packet carries
/// in vf15. The GS interpolates F across the triangle as it does colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthFog {
    pub a: f32,
    pub b: f32,
    pub min: f32,
    pub max: f32,
    /// FOGCOL.
    pub colour: [u8; 3],
}

impl DepthFog {
    /// `SetFog(near, far, nearRate, farRate, colour)`: fMax = 2.55 (100 -
    /// nearRate) at `near` and nearer, fMin = 2.55 (100 - farRate) at `far`
    /// and beyond, linear between.
    pub fn set_fog(near: f32, far: f32, near_rate: f32, far_rate: f32, colour: [u8; 3]) -> DepthFog {
        let min = 2.55 * (100.0 - far_rate);
        let max = 2.55 * (100.0 - near_rate);
        let d = min - max;
        let span = far - near;
        DepthFog { a: d / span, b: max - near * d / span, min, max, colour }
    }

    /// F at depth `w`, truncated to the GS's eight bits.
    pub fn at(&self, w: f32) -> f32 {
        (self.b + self.a * w).clamp(self.min, self.max).clamp(0.0, 255.0).trunc()
    }
}

/// The texture sampling for every material of a model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TexParams {
    pub func: TexFunc,
    pub use_alpha: bool,
    pub filter: Filter,
}

/// What the lit programs light with (`mc_SetMatrix`, VU memory 12-19).
/// VU1 computes `min(0x80, sum colour_i * max(0, dir_i . N) + 0x80 * ambient)`
/// with `N` the raw s8 normal, whose length is 64; against the unit normal
/// `n` that is `min(0x80, 0x80 * (sum colour_i / 2 * max(0, dir_i . n) + ambient))`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lights {
    /// Unit directions toward each light, in model space.
    pub dirs: [[f32; 3]; 3],
    /// Already doubled as `mc_SetMatrix` doubles them.
    pub colours: [[f32; 3]; 3],
    pub ambient: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_register_round_trips() {
        for b in Blend::TABLE {
            assert_eq!(Blend::from_reg(b.to_reg()), b);
        }
        assert_eq!(Blend::MIX.to_reg(), 0x44);
        assert_eq!(Blend::REPLACE.fix, 0x80);
    }
}
