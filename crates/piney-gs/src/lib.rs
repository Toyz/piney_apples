//! Draws [`piney_draw::Frame`]s as the GS draws them, on wgpu, into the game's
//! own 512 x 448 frame buffer in the art's gamma (only [`Presenter`]
//! linearises, stretching to 4:3). Geometry is the CPU's ([`convert`], moved
//! half a pixel for the top-left rule); pixels the shader's; blending the
//! pipeline's ([`convert::blend_state`], GS Z as depth). A frame-buffer
//! texture ends the render pass and copies its rectangle. Not modelled: a
//! failing alpha test's FB_ONLY / RGB_ONLY sparing depth and alpha (here they
//! write them with the colour), dithering, 8-bit rounding in the pipeline.

pub mod assets;
pub mod convert;
pub mod png;
mod shadow;

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_draw::{AlphaFail, AlphaTest, Blend, Cmd, Compare, Filter, Frame, Scissor, TexRef, Wrap, ZTest};
use wgpu::util::DeviceExt;

pub use assets::Assets;
use convert::{GVertex, Params, Screen};

/// The frame buffer's format: 8-bit channels in the art's own gamma.
pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VIn {
    @location(0) clip: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) params: u32,
    @location(4) fog: u32,
    @location(5) fog_f: f32,
    @location(6) region: u32,
};
struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) @interpolate(flat) params: u32,
    @location(3) @interpolate(flat) fog: u32,
    @location(4) @interpolate(linear) fog_f: f32,
    @location(5) @interpolate(flat) region: u32,
};

@vertex
fn vs(v: VIn) -> VOut {
    var o: VOut;
    o.pos = v.clip;
    o.uv = v.uv;
    o.colour = v.colour;
    o.params = v.params;
    o.fog = v.fog;
    o.fog_f = v.fog_f;
    o.region = v.region;
    return o;
}

struct FOut {
    @location(0) colour: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs(i: VOut) -> FOut {
    var t = textureSample(tex, samp, i.uv);
    if ((i.params & (1u << 21u)) != 0u) {
        // REGION_REPEAT: the texel (u & MINU) | MAXU, (v & MINV) | MAXV.
        let dims = vec2<u32>(textureDimensions(tex));
        let m = i.region & 0xffffu;
        let f = i.region >> 16u;
        let tc = vec2<u32>(floor(i.uv * vec2<f32>(dims)));
        let tr = min((tc & vec2<u32>(m)) | vec2<u32>(f), dims - vec2<u32>(1u));
        t = textureLoad(tex, vec2<i32>(tr), 0);
    }
    if ((i.params & (1u << 22u)) != 0u) {
        // A copy of the frame at a smaller size (width, height in the
        // region word), read bilinearly: each of the four texels is itself
        // the frame read bilinearly at that texel's centre.
        let n = vec2<f32>(f32(i.region & 0xffffu), f32(i.region >> 16u));
        let q = i.uv * n - vec2<f32>(0.5);
        let b = floor(q);
        let w = q - b;
        let hi = n - vec2<f32>(1.0);
        let t00 = textureSampleLevel(tex, samp, (clamp(b, vec2<f32>(0.0), hi) + 0.5) / n, 0.0);
        let t10 = textureSampleLevel(tex, samp, (clamp(b + vec2<f32>(1.0, 0.0), vec2<f32>(0.0), hi) + 0.5) / n, 0.0);
        let t01 = textureSampleLevel(tex, samp, (clamp(b + vec2<f32>(0.0, 1.0), vec2<f32>(0.0), hi) + 0.5) / n, 0.0);
        let t11 = textureSampleLevel(tex, samp, (clamp(b + vec2<f32>(1.0), vec2<f32>(0.0), hi) + 0.5) / n, 0.0);
        t = mix(mix(t00, t10, w.x), mix(t01, t11, w.x), w.y);
    }
    let p = i.params;
    let vc = i.colour;
    // The GS interpolates the vertex alpha as an integer, so a constant
    // stays exact; the interpolated float can land a hair below it, which
    // on an alpha test's threshold (a model fading in writes Z where it
    // passes) dithers what passes.
    let va = floor(vc.a + 1.0 / 32.0);
    // Untextured: the vertex colour is the pixel.
    var rgb = vc.rgb / 255.0;
    var a = va;
    if ((p & 8u) != 0u) {
        // The texel's alpha in GS units: textures are stored with 0x80
        // scaled to 255.
        let ta = select(t.a * 127.5, 128.0, t.a >= 1.0);
        let tcc = (p & 4u) != 0u;
        let func = p & 3u;
        if (func == 1u) {
            // DECAL
            rgb = t.rgb;
            if (tcc) { a = ta; }
        } else {
            // MODULATE, HIGHLIGHT, HIGHLIGHT2
            rgb = t.rgb * vc.rgb / 128.0;
            if (func >= 2u) { rgb = rgb + vec3<f32>(vc.a / 255.0); }
            if (tcc) {
                if (func == 0u) { a = ta * va / 128.0; }
                else if (func == 2u) { a = ta + va; }
                else { a = ta; }
            }
        }
    }
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    if ((p & (1u << 18u)) != 0u) {
        // The fog stage: (Cs * F + FOGCOL * (0xff - F)) >> 8.
        // Bit 20: each vertex's F, interpolated as the GS does.
        let f = select(f32((i.fog >> 24u) & 255u), floor(i.fog_f), (p & (1u << 20u)) != 0u);
        let fc = vec3<f32>(f32(i.fog & 255u), f32((i.fog >> 8u) & 255u), f32((i.fog >> 16u) & 255u)) / 255.0;
        rgb = (rgb * f + fc * (255.0 - f)) / 256.0;
    }
    if ((p & 16u) != 0u) {
        let method = (p >> 5u) & 7u;
        let r = f32((p >> 8u) & 255u);
        let ai = floor(min(a, 255.0));
        var ok = true;
        switch method {
            case 0u: { ok = false; }
            case 1u: { ok = true; }
            case 2u: { ok = ai < r; }
            case 3u: { ok = ai <= r; }
            case 4u: { ok = ai == r; }
            case 5u: { ok = ai >= r; }
            case 6u: { ok = ai > r; }
            default: { ok = ai != r; }
        }
        let fail = (p >> 16u) & 3u;
        if (!ok && (fail == 0u || fail == 2u)) {
            discard;
        }
        // The second pass of an FB_ONLY draw: the failing pixels only.
        if (ok && (p & (1u << 19u)) != 0u) {
            discard;
        }
    }
    // Alpha leaves as As / 0x80: the blend's C factor, and what the frame
    // buffer keeps as Ad.
    var o: FOut;
    o.colour = vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), clamp(a / 128.0, 0.0, 1.0));
    // The GS keeps Z as an integer: surfaces less than a unit apart tie,
    // and the test (GEQUAL) lets the later one through. Depth is z / 2^32,
    // exact in a float for every Z below 2^24.
    o.depth = floor(i.pos.z * 4294967296.0) / 4294967296.0;
    return o;
}
"#;

const PRESENT: &str = r#"
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
// The display circuits: the picture moved this much of its own size (x,
// y); read circuit 1's share (z) of the merge with the picture a line (w)
// lower.
@group(0) @binding(2) var<uniform> crtc: vec4<f32>;

struct P {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> P {
    let x = f32(i & 1u) * 4.0 - 1.0;
    let y = f32(i >> 1u) * 4.0 - 1.0;
    var o: P;
    o.pos = vec4<f32>(x, y, 0.0, 1.0);
    o.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return o;
}

// The frame buffer's values are what the television shows; an sRGB surface
// encodes what it is given, so give it the linear value that encodes back
// to them.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

// Sharp bilinear: each frame-buffer pixel is a solid block at any window
// size, and only the one-window-pixel seam between two blocks is blended.
// Plain bilinear at the 4:3 stretch's non-integer scale draws a 1-pixel
// stroke crisp in one place and smeared over two pixels in another, so text
// such as the desktop's icon labels looks uneven. At a whole-number scale
// this samples texel centres exactly; at scale 1 it is plain bilinear.
fn sharp(uv: vec2<f32>) -> vec2<f32> {
    let size = vec2<f32>(textureDimensions(src));
    let texel = uv * size;
    let step = max(abs(vec2<f32>(dpdx(texel.x), dpdy(texel.y))), vec2<f32>(1e-6));
    let scale = max(vec2<f32>(1.0) / step, vec2<f32>(1.0));
    let r = vec2<f32>(0.5) - vec2<f32>(0.5) / scale;
    let d = fract(texel) - vec2<f32>(0.5);
    let f = (d - clamp(d, -r, r)) * scale + vec2<f32>(0.5);
    return (floor(texel) + f) / size;
}

// The frame buffer's pixel at `uv` once the display offset moved it; black
// where the moved picture leaves the screen.
fn picture(uv0: vec2<f32>) -> vec3<f32> {
    let uv = uv0 - crtc.xy;
    let c = textureSample(src, samp, sharp(uv)).rgb;
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    return select(vec3<f32>(0.0), c, inside);
}

// The merge circuit: read circuit 2's line over circuit 1's, the line above.
fn merged(uv: vec2<f32>) -> vec3<f32> {
    let above = picture(uv - vec2<f32>(0.0, crtc.w));
    return mix(picture(uv), above, crtc.z);
}

@fragment
fn fs_srgb(p: P) -> @location(0) vec4<f32> {
    return vec4<f32>(to_linear(merged(p.uv)), 1.0);
}

@fragment
fn fs_plain(p: P) -> @location(0) vec4<f32> {
    return vec4<f32>(merged(p.uv), 1.0);
}

// An overlay: its texels as they are, blended by their alpha.
@fragment
fn fs_overlay_srgb(p: P) -> @location(0) vec4<f32> {
    let c = textureSample(src, samp, p.uv);
    return vec4<f32>(to_linear(c.rgb), c.a);
}

@fragment
fn fs_overlay_plain(p: P) -> @location(0) vec4<f32> {
    return textureSample(src, samp, p.uv);
}
"#;

/// What a texture is known by: a file's texture through a palette, or a
/// run-time upload by its contents (the same line of text uploaded every
/// frame is one texture); or the frame buffer: this frame's `n`-th copy of
/// it ([`TexRef::FrameBuffer`]), or the last frame's picture.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TexKey {
    Ccs { file: String, texture: u32, clut: u32 },
    Upload(u64),
    Copy(usize),
    Previous,
}

/// A copy of the target's rectangle into this frame's texture `n`, made
/// before draw `before`.
struct FrameCopy {
    before: usize,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

struct GpuTex {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
    /// One bind group per sampler (see [`sampler_index`]).
    groups: Vec<wgpu::BindGroup>,
    /// The last frame a run-time upload was in.
    last_used: u64,
}

/// Run-time uploads of at least this many texels (a movie's picture, a new
/// one every frame) go as soon as a frame does not upload them.
const BIG_UPLOAD: u64 = 256 * 256;
/// Smaller uploads (lines of text, a cursor's icon) are kept this many
/// frames after their last use, for when they come back.
const UPLOAD_KEEP: u64 = 300;

fn sampler_index(filter: Filter, wrap: Wrap) -> usize {
    (filter == Filter::Linear) as usize * 2 + (wrap == Wrap::Clamp) as usize
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipeKey {
    blend: Option<Blend>,
    test: ZTest,
    /// Z writes.
    write: bool,
}

struct Draw {
    pipe: PipeKey,
    tex: Option<TexKey>,
    sampler: usize,
    first: u32,
    count: u32,
    fix: u8,
    scissor: Scissor,
}

struct Target {
    color: wgpu::Texture,
    view: wgpu::TextureView,
    depth: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    width: u32,
    height: u32,
}

/// The GS: a frame buffer, a Z buffer, and what draws into them.
pub struct Gs {
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    tex_layout: wgpu::BindGroupLayout,
    samplers: Vec<wgpu::Sampler>,
    pipelines: HashMap<PipeKey, wgpu::RenderPipeline>,
    textures: HashMap<TexKey, GpuTex>,
    missing: HashSet<TexKey>,
    white: GpuTex,
    target: Target,
    /// This frame's copies of the frame buffer, by [`TexKey::Copy`] index.
    copies: Vec<GpuTex>,
    /// The last frame's picture ([`TexKey::Previous`]), target-sized.
    previous: Option<GpuTex>,
    /// The shadow packets' steps ([`Cmd::Shadow`]).
    shadow: shadow::ShadowGpu,
    /// Frames rendered, for the uploads' ages.
    frame_no: u64,
    /// The last frame's [`Frame::field_mode`].
    field_mode: bool,
    pub assets: Assets,
}

impl Gs {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, assets: Assets) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gs"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let tex_layout = texture_layout(&device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gs"),
            bind_group_layouts: &[Some(&tex_layout)],
            immediate_size: 0,
        });
        let samplers = [
            (Filter::Nearest, Wrap::Repeat),
            (Filter::Nearest, Wrap::Clamp),
            (Filter::Linear, Wrap::Repeat),
            (Filter::Linear, Wrap::Clamp),
        ]
        .map(|(f, w)| {
            let filter = if f == Filter::Linear { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest };
            let mode = if w == Wrap::Clamp { wgpu::AddressMode::ClampToEdge } else { wgpu::AddressMode::Repeat };
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: mode,
                address_mode_v: mode,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        })
        .to_vec();
        let white = upload(&device, &queue, &tex_layout, &samplers, 1, 1, &[255; 4]);
        let shadow = shadow::ShadowGpu::new(&device, TARGET_FORMAT);
        let target = make_target(&device, piney_draw::SCREEN_WIDTH.into(), piney_draw::SCREEN_HEIGHT.into());
        Gs {
            device,
            queue,
            shader,
            layout,
            tex_layout,
            samplers,
            pipelines: HashMap::new(),
            textures: HashMap::new(),
            missing: HashSet::new(),
            white,
            target,
            copies: Vec::new(),
            previous: None,
            shadow,
            frame_no: 0,
            field_mode: false,
            assets,
        }
    }

    /// A GS on the first adapter, with no window: for tests and `--shot`.
    pub fn headless(assets: Assets) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .map_err(|e| e.to_string())?;
        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor { label: Some("gs"), ..Default::default() }),
        )
        .map_err(|e| e.to_string())?;
        Ok(Gs::new(device, queue, assets))
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The frame buffer, for [`Presenter`].
    pub fn target_view(&self) -> &wgpu::TextureView {
        &self.target.view
    }

    pub fn target_size(&self) -> (u32, u32) {
        (self.target.width, self.target.height)
    }

    /// Another disc's `DATA.BIN`: the textures read from the old one are
    /// dropped.
    pub fn set_archive(&mut self, archive: Arc<Archive>) {
        self.assets.set_archive(archive);
        self.textures.retain(|k, _| matches!(k, TexKey::Upload(_)));
        self.missing.clear();
    }

    /// Files the frames name that are not in the archive: a stream's own
    /// archive while it plays, looked up first. Changing it drops the file
    /// textures read so far.
    pub fn set_overlay(&mut self, overlay: Option<Arc<Archive>>) {
        if self.assets.set_overlay(overlay) {
            self.textures.retain(|k, _| matches!(k, TexKey::Upload(_)));
            self.missing.clear();
        }
    }

    /// How many textures (file textures and uploads) and pipelines are
    /// made, for timing tools.
    pub fn counts(&self) -> (usize, usize) {
        (self.textures.len(), self.pipelines.len())
    }

    /// Draw `frame` into the frame buffer.
    pub fn render(&mut self, frame: &Frame) {
        let (w, h) = (u32::from(frame.width.max(1)), u32::from(frame.height.max(1)));
        if (w, h) != (self.target.width, self.target.height) {
            self.target = make_target(&self.device, w, h);
        }
        let screen = Screen { width: w as f32, height: h as f32 };
        self.field_mode = frame.field_mode;

        self.frame_no += 1;
        let now = self.frame_no;
        let mut uploads: HashMap<u32, TexKey> = HashMap::new();
        for u in &frame.uploads {
            let mut hasher = DefaultHasher::new();
            u.hash(&mut hasher);
            let key = TexKey::Upload(hasher.finish());
            if let Some(t) = self.textures.get_mut(&key) {
                t.last_used = now;
            } else {
                let img = assets::upload_image(u);
                let mut t = upload(
                    &self.device,
                    &self.queue,
                    &self.tex_layout,
                    &self.samplers,
                    img.width,
                    img.height,
                    &img.rgba,
                );
                t.last_used = now;
                self.textures.insert(key.clone(), t);
            }
            uploads.insert(u.id, key);
        }
        // Run-time textures this frame did not upload go: big ones at once,
        // the rest once unused for `UPLOAD_KEEP` frames.
        self.textures.retain(|k, t| {
            let big = u64::from(t.width) * u64::from(t.height) >= BIG_UPLOAD;
            !matches!(k, TexKey::Upload(_)) || t.last_used == now || (!big && now - t.last_used < UPLOAD_KEEP)
        });

        let mut verts: Vec<GVertex> = Vec::new();
        let mut draws: Vec<Draw> = Vec::new();
        let mut copies: Vec<FrameCopy> = Vec::new();
        // The shadow packets, by where they stand among the draws.
        let mut shadows: Vec<(usize, &piney_draw::ShadowPass)> = Vec::new();
        let mut uses_previous = false;
        for cmd in &frame.cmds {
            match cmd {
                Cmd::Prim(p) => {
                    let (tex, size, sampler, texture) = match &p.state.texture {
                        Some(ts) => {
                            let key = match &ts.tex {
                                TexRef::Ccs { file, texture, clut } => {
                                    Some(TexKey::Ccs { file: file.clone(), texture: *texture, clut: *clut })
                                }
                                TexRef::Upload(id) => uploads.get(id).cloned(),
                                &TexRef::FrameBuffer { x, y, width, height } => {
                                    copies.push(FrameCopy {
                                        before: draws.len(),
                                        x: x.into(),
                                        y: y.into(),
                                        width: width.max(1).into(),
                                        height: height.max(1).into(),
                                    });
                                    Some(TexKey::Copy(copies.len() - 1))
                                }
                                TexRef::PreviousFrame => {
                                    uses_previous = true;
                                    Some(TexKey::Previous)
                                }
                                // The whole frame copied; the shader reads it
                                // through the smaller texture's grid.
                                TexRef::ScaledFrame { .. } => {
                                    copies.push(FrameCopy { before: draws.len(), x: 0, y: 0, width: w, height: h });
                                    Some(TexKey::Copy(copies.len() - 1))
                                }
                            };
                            let size = match (&key, &ts.tex) {
                                (_, &TexRef::ScaledFrame { width, height }) => {
                                    Some((u32::from(width).max(1), u32::from(height).max(1)))
                                }
                                (Some(TexKey::Copy(n)), _) => Some((copies[*n].width, copies[*n].height)),
                                (Some(TexKey::Previous), _) => Some((w, h)),
                                _ => key.as_ref().and_then(|k| self.texture(k)),
                            };
                            let key = key.filter(|_| size.is_some());
                            (key, size, sampler_index(ts.filter, ts.wrap), Some((ts.func, ts.use_alpha)))
                        }
                        None => (None, None, 0, None),
                    };
                    let params = Params::new(texture.filter(|_| tex.is_some()), p.state.alpha_test);
                    let first = verts.len() as u32;
                    verts.extend(convert::prim(p, size, screen, params));
                    let fix = p.state.blend.map_or(0, |b| b.fix);
                    let pipe = PipeKey { blend: p.state.blend, test: p.state.depth.test, write: p.state.depth.write };
                    push(
                        &mut verts,
                        &mut draws,
                        Draw { pipe, tex, sampler, first, count: 0, fix, scissor: p.state.scissor },
                        p.state.alpha_test,
                    );
                }
                Cmd::Shadow(sh) => shadows.push((draws.len(), sh)),
                Cmd::Model(m) => {
                    let Some(file) = self.assets.file(&m.file) else { continue };
                    let Some(model) = file.models.get(&m.model) else { continue };
                    let to_screen = Mat4::from_cols_array_2d(&m.to_screen);
                    let nodes: Vec<Mat4> = m.nodes.iter().map(Mat4::from_cols_array_2d).collect();
                    let pipe = PipeKey { blend: m.state.blend, test: m.state.depth.test, write: m.state.depth.write };
                    let fog =
                        convert::fog_word(m.fog.or(m.depth_fog.map(|d| piney_draw::Fog { f: 0xff, colour: d.colour })));
                    let fix = m.state.blend.map_or(0, |b| b.fix);
                    // A texture put in place of the materials': the frame
                    // buffer copied once, before the command's first draw.
                    let swap = match m.edits.as_ref().and_then(|e| e.tex.as_ref()) {
                        Some(&TexRef::FrameBuffer { x, y, width, height }) => {
                            copies.push(FrameCopy {
                                before: draws.len(),
                                x: x.into(),
                                y: y.into(),
                                width: width.max(1).into(),
                                height: height.max(1).into(),
                            });
                            Some(TexKey::Copy(copies.len() - 1))
                        }
                        Some(TexRef::Ccs { file, texture, clut }) => {
                            Some(TexKey::Ccs { file: file.clone(), texture: *texture, clut: *clut })
                        }
                        _ => None,
                    };
                    // The texture coordinates the EE wrote over the model's.
                    let edited_st;
                    let model = match &m.edits {
                        Some(e) if !e.st.is_empty() => {
                            let mut md = model.clone();
                            for &(mi, v, st) in &e.st {
                                if let Some(uv) = md.mmats.get_mut(mi as usize).and_then(|x| x.uvs.get_mut(v as usize))
                                {
                                    *uv = st;
                                }
                            }
                            edited_st = md;
                            &edited_st
                        }
                        _ => model,
                    };
                    for d in &m.mmats {
                        let Some(mm) = model.mmats.get(d.index as usize) else { continue };
                        let material = mm.material.and_then(|mat| file.scene.materials.get(&mat).copied());
                        let key = swap.clone().or_else(|| {
                            material.and_then(|mat| {
                                // ccClump::ChangeTex: another texture for this
                                // material, with its own palette.
                                let texture = mm
                                    .material
                                    .and_then(|o| m.tex_swaps.iter().find(|(from, _)| *from == o))
                                    .map_or(mat.texture, |s| s.1);
                                let t = file.texture(texture)?;
                                let clut =
                                    m.clut_swaps.iter().find(|(from, _)| *from == t.clut).map_or(t.clut, |s| s.1);
                                Some(TexKey::Ccs { file: m.file.clone(), texture, clut })
                            })
                        });
                        let size = match &key {
                            Some(TexKey::Copy(n)) => Some((copies[*n].width, copies[*n].height)),
                            _ => key.as_ref().and_then(|k| self.texture(k)),
                        };
                        let key = key.filter(|_| size.is_some());
                        // The test's method and fail from the state, its
                        // reference from the mmat.
                        let test = match m.state.alpha_test {
                            AlphaTest::On { method, fail, .. } => {
                                AlphaTest::On { method, reference: d.alpha_ref, fail }
                            }
                            AlphaTest::Off => AlphaTest::Off,
                        };
                        let mut params = Params::new(key.as_ref().map(|_| (m.tex.func, m.tex.use_alpha)), test);
                        if m.fog.is_some() {
                            params.0 |= Params::FOG;
                        } else if m.depth_fog.is_some() {
                            params.0 |= Params::FOG | Params::DEPTH_FOG;
                        }
                        let first = verts.len() as u32;
                        let targets: Vec<(&piney_data::model::Model, f32)> =
                            m.morph.iter().filter_map(|&(t, w)| file.models.get(&t).map(|t| (t, w))).collect();
                        let mut morphed =
                            if targets.is_empty() { None } else { model.morph(d.index as usize, &targets) };
                        // What the EE wrote into this mmat's vertices.
                        let mut colours = None;
                        if let Some(e) = &m.edits {
                            for &(_, v, z) in e.z.iter().filter(|e| e.0 == d.index) {
                                let p = morphed.get_or_insert_with(|| mm.positions.clone());
                                if let Some(p) = p.get_mut(v as usize) {
                                    p[2] = z;
                                }
                            }
                            for &(_, v, rgb) in e.rgb.iter().filter(|e| e.0 == d.index) {
                                let c = colours.get_or_insert_with(|| mm.colours.clone());
                                if let Some(c) = c.get_mut(v as usize) {
                                    c[..3].copy_from_slice(&rgb);
                                }
                            }
                        }
                        verts.extend(convert::mmat(
                            model,
                            d,
                            morphed.as_deref(),
                            colours.as_deref(),
                            to_screen,
                            &nodes,
                            m.lights.as_ref(),
                            m.reject_outside,
                            m.div_z,
                            screen,
                            params,
                            fog,
                            if m.fog.is_none() { m.depth_fog.as_ref() } else { None },
                        ));
                        let sampler = sampler_index(m.tex.filter, d.wrap);
                        let draw = Draw { pipe, tex: key, sampler, first, count: 0, fix, scissor: m.state.scissor };
                        push(&mut verts, &mut draws, draw, test);
                    }
                }
            }
        }
        for d in &draws {
            if !self.pipelines.contains_key(&d.pipe) {
                let p = make_pipeline(&self.device, &self.layout, &self.shader, d.pipe);
                self.pipelines.insert(d.pipe, p);
            }
        }

        let buffer = (!verts.is_empty()).then(|| {
            self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gs vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // The last frame's picture, taken before this frame clears it.
        if uses_previous {
            if self.previous.as_ref().is_none_or(|p| (p.width, p.height) != (w, h)) {
                self.previous = Some(self.copy_texture(w, h));
            }
            if let Some(p) = &self.previous {
                copy_region(&mut encoder, &self.target.color, &p.texture, 0, 0, w, h);
            }
        }
        self.copies = copies.iter().map(|c| self.copy_texture(c.width, c.height)).collect();
        let [r, g, b, _] = frame.clear.0.map(|c| f64::from(c) / 255.0);
        // One pass per run of draws between frame-buffer copies and shadow
        // packets; the first clears, the rest keep the picture and the Z
        // buffer.
        let (mut next, mut clear) = (0, true);
        let mut pending = copies.iter().enumerate().peekable();
        let mut pending_shadows = shadows.iter().peekable();
        loop {
            let stop = pending
                .peek()
                .map_or(draws.len(), |(_, c)| c.before)
                .min(pending_shadows.peek().map_or(draws.len(), |s| s.0));
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("gs"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.target.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if clear {
                                wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 0.0 })
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.target.depth,
                        // GS Z 0, the farthest.
                        depth_ops: Some(wgpu::Operations {
                            load: if clear { wgpu::LoadOp::Clear(0.0) } else { wgpu::LoadOp::Load },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if let Some(buffer) = &buffer {
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    for d in &draws[next..stop] {
                        // SCISSOR_1, inclusive, within the frame buffer.
                        let s = d.scissor;
                        let (x0, y0) = (u32::from(s.x0).min(w), u32::from(s.y0).min(h));
                        let (x1, y1) = ((u32::from(s.x1) + 1).min(w), (u32::from(s.y1) + 1).min(h));
                        if x1 <= x0 || y1 <= y0 {
                            continue;
                        }
                        pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
                        pass.set_pipeline(&self.pipelines[&d.pipe]);
                        let tex = d.tex.as_ref().and_then(|k| self.bound(k)).unwrap_or(&self.white);
                        pass.set_bind_group(0, &tex.groups[d.sampler], &[]);
                        let f = f64::from(d.fix) / 128.0;
                        pass.set_blend_constant(wgpu::Color { r: f, g: f, b: f, a: f });
                        pass.draw(d.first..d.first + d.count, 0..1);
                    }
                }
            }
            (next, clear) = (stop, false);
            if pending.peek().is_none() && pending_shadows.peek().is_none() {
                break;
            }
            // At this point: the shadow packets first (a copy taken here
            // sees them, as it follows them in the list), then the copies.
            while let Some((_, sh)) = pending_shadows.next_if(|s| s.0 == stop) {
                let depth = self.target.depth_texture.create_view(&Default::default());
                self.shadow.run(&self.device, &mut encoder, &self.target.view, &depth, (w, h), sh);
            }
            while let Some((n, c)) = pending.next_if(|(_, c)| c.before == stop) {
                copy_region(&mut encoder, &self.target.color, &self.copies[n].texture, c.x, c.y, c.width, c.height);
            }
        }
        self.queue.submit([encoder.finish()]);
    }

    /// The texture a draw binds.
    fn bound(&self, key: &TexKey) -> Option<&GpuTex> {
        match key {
            TexKey::Copy(n) => self.copies.get(*n),
            TexKey::Previous => self.previous.as_ref(),
            _ => self.textures.get(key),
        }
    }

    /// An empty texture of the frame buffer's format that the target can be
    /// copied into and drawn from; it starts as zeros.
    fn copy_texture(&self, width: u32, height: u32) -> GpuTex {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frame buffer copy"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu_tex(&self.device, &self.tex_layout, &self.samplers, texture, width, height)
    }

    /// A file texture on the GPU, uploaded on first use; its size.
    fn texture(&mut self, key: &TexKey) -> Option<(u32, u32)> {
        if let Some(t) = self.textures.get(key) {
            return Some((t.width, t.height));
        }
        if self.missing.contains(key) {
            return None;
        }
        let TexKey::Ccs { file, texture, clut } = key else { return None };
        match self.assets.image(file, *texture, *clut) {
            Some(img) => {
                let t = upload(
                    &self.device,
                    &self.queue,
                    &self.tex_layout,
                    &self.samplers,
                    img.width,
                    img.height,
                    &img.rgba,
                );
                let size = (t.width, t.height);
                self.textures.insert(key.clone(), t);
                Some(size)
            }
            None => {
                eprintln!("gs: no texture {texture} / clut {clut} in {file}");
                self.missing.insert(key.clone());
                None
            }
        }
    }

    /// The frame buffer as RGBA8 rows, top first.
    pub fn read_back(&self) -> Vec<u8> {
        let (width, height) = (self.target.width, self.target.height);
        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target.color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.expect("map readback"));
        self.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let data = readback.get_mapped_range(..).expect("mapped range");
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            out.extend_from_slice(&data[y * row as usize..y * row as usize + width as usize * 4]);
        }
        // The frame buffer's alpha is the GS's, not an opacity.
        out.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = 255);
        out
    }
}

/// Finish a draw whose vertices run from `draw.first` to the end of
/// `verts`. A draw whose failing alpha test still writes colour but not Z
/// (AFAIL FB_ONLY, as the desktop's models draw) goes in two passes: colour
/// everywhere with no Z write, then Z alone where the test passes. The GS
/// does both per pixel; the two differ only where one draw overlaps itself.
fn push(verts: &mut Vec<GVertex>, draws: &mut Vec<Draw>, mut draw: Draw, test: AlphaTest) {
    draw.count = verts.len() as u32 - draw.first;
    if draw.count == 0 {
        return;
    }
    // NEVER with FB_ONLY (the blended models'): colour always, Z never.
    if let AlphaTest::On { method: Compare::Never, fail: AlphaFail::FbOnly, .. } = test {
        draw.pipe.write = false;
        draws.push(draw);
        return;
    }
    // A test that cannot fail (GEQUAL 0 is the models' usual one) changes
    // nothing.
    let can_fail = !matches!(
        test,
        AlphaTest::On { method: Compare::Always, .. } | AlphaTest::On { method: Compare::GEqual, reference: 0, .. }
    );
    let split = draw.pipe.write && can_fail && matches!(test, AlphaTest::On { fail: AlphaFail::FbOnly, .. });
    if !split {
        draws.push(draw);
        return;
    }
    // FB_ONLY: a pixel that fails keeps its colour but not its Z. The GS
    // writes Z pixel by pixel in drawing order, so a model's nearer layer
    // hides the farther ones drawn after it. The passing pixels are drawn
    // first, with Z, in their order; the failing ones then add their colour
    // without Z. Only where a failing pixel meets a passing one drawn after
    // it in the same draw does the order differ from the GS's.
    let range = draw.first as usize..;
    let fails: Vec<GVertex> =
        verts[range.clone()].iter().map(|v| GVertex { params: Params(v.params).pass_discards().0, ..*v }).collect();
    for v in &mut verts[range] {
        v.params = Params(v.params).fail_discards().0;
    }
    let fail_draw = Draw {
        pipe: PipeKey { write: false, ..draw.pipe },
        tex: draw.tex.clone(),
        sampler: draw.sampler,
        first: verts.len() as u32,
        count: draw.count,
        fix: draw.fix,
        scissor: draw.scissor,
    };
    verts.extend(fails);
    draws.push(draw);
    draws.push(fail_draw);
}

/// Puts the frame buffer on a window: stretched to 4:3 as a television
/// shows 512 x 448, letterboxed in the window, scaled sharp bilinear (each
/// pixel a solid block, only the seams blended), and made linear for an
/// sRGB surface.
pub struct Presenter {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// [`Overlay`]s: drawn blended, texel for pixel.
    overlay: wgpu::RenderPipeline,
    nearest: wgpu::Sampler,
    /// [`Pcrtc`] as the shader's `crtc`.
    shift: wgpu::Buffer,
}

/// Pixels drawn over the picture at the window's own size, not the PS2
/// frame's (the port's console): `width` x `height` RGBA with straight
/// alpha, at the window's top left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Overlay {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The display's shape.
pub const DISPLAY_ASPECT: f32 = 4.0 / 3.0;

impl Presenter {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("present"),
            source: wgpu::ShaderSource::Wgsl(PRESENT.into()),
        });
        let layout = present_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(if format.is_srgb() { "fs_srgb" } else { "fs_plain" }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("present"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let overlay = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(if format.is_srgb() { "fs_overlay_srgb" } else { "fs_overlay_plain" }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("overlay"), ..Default::default() });
        let shift = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("present shift"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Presenter { pipeline, layout, sampler, overlay, nearest, shift }
    }

    /// Draw `gs`'s frame buffer into `view`, `width` x `height` pixels, as
    /// `crtc` shows it, and `overlay` over it.
    pub fn present(
        &self,
        gs: &Gs,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
        crtc: Pcrtc,
        overlay: Option<&Overlay>,
    ) -> wgpu::CommandBuffer {
        let device = gs.device();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(gs.target_view()) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: self.shift.as_entire_binding() },
            ],
        });
        let merge = if crtc.deflicker && !gs.field_mode { MERGE_ALP } else { 0.0 };
        let frac = [crtc.offset[0] / 512.0, crtc.offset[1] / 448.0, merge, 1.0 / 448.0];
        let bytes: Vec<u8> = frac.iter().flat_map(|v: &f32| v.to_le_bytes()).collect();
        gs.queue().write_buffer(&self.shift, 0, &bytes);
        let (w, h) = (width.max(1) as f32, height.max(1) as f32);
        let (vw, vh) = if w / h > DISPLAY_ASPECT { (h * DISPLAY_ASPECT, h) } else { (w, w / DISPLAY_ASPECT) };
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_viewport((w - vw) / 2.0, (h - vh) / 2.0, vw, vh, 0.0, 1.0);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
            if let Some(o) = overlay.filter(|o| o.width > 0 && o.height > 0) {
                let texture = device.create_texture_with_data(
                    gs.queue(),
                    &wgpu::TextureDescriptor {
                        label: Some("overlay"),
                        size: wgpu::Extent3d { width: o.width, height: o.height, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &o.rgba,
                );
                let tv = texture.create_view(&Default::default());
                let og = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tv) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.nearest) },
                        wgpu::BindGroupEntry { binding: 2, resource: self.shift.as_entire_binding() },
                    ],
                });
                let (ow, oh) = (o.width.min(width.max(1)) as f32, o.height.min(height.max(1)) as f32);
                pass.set_viewport(0.0, 0.0, ow, oh, 0.0, 1.0);
                pass.set_pipeline(&self.overlay);
                pass.set_bind_group(0, &og, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        encoder.finish()
    }
}

/// How the display circuits show the frame buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pcrtc {
    /// `ccSystem::SetDisplayOffset`'s move, in pixels of the 512 x 448
    /// frame.
    pub offset: [f32; 2],
    /// Read circuit 1's copy a line lower, merged as in frame mode. The
    /// game's deflicker for an interlaced TV; off, the picture is sharp as
    /// PCSX2's anti-blur shows it.
    pub deflicker: bool,
}

/// PMODE.ALP in frame mode (`SetScreenModeMain`, 0x0010b51c): read circuit
/// 1's share of the merge, of 255.
const MERGE_ALP: f32 = 127.0 / 255.0;

/// The presenter's: [`texture_layout`]'s two, and the display offset's
/// uniform at 2.
fn present_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("present"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn make_target(device: &wgpu::Device, width: u32, height: u32) -> Target {
    let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame buffer"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("z buffer"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH,
        // The shadow packets read it.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth = depth_texture.create_view(&Default::default());
    let view = color.create_view(&Default::default());
    Target { color, view, depth, depth_texture, width, height }
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    samplers: &[wgpu::Sampler],
    width: u32,
    height: u32,
    rgba: &[u8],
) -> GpuTex {
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // The art's own values: the GS does not linearise texels.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        rgba,
    );
    gpu_tex(device, layout, samplers, texture, width, height)
}

/// `texture` with a bind group per sampler.
fn gpu_tex(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    samplers: &[wgpu::Sampler],
    texture: wgpu::Texture,
    width: u32,
    height: u32,
) -> GpuTex {
    let view = texture.create_view(&Default::default());
    let groups = samplers
        .iter()
        .map(|s| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(s) },
                ],
            })
        })
        .collect();
    GpuTex { texture, width, height, groups, last_used: 0 }
}

/// Copy the `width` x `height` rectangle of `src` at (`x`, `y`) to the
/// corner of `dst`, texel (0, 0) from pixel (`x`, `y`); what lies outside
/// `src` is left as `dst` has it.
fn copy_region(
    encoder: &mut wgpu::CommandEncoder,
    src: &wgpu::Texture,
    dst: &wgpu::Texture,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    let clip = |at: i32, len: u32, size: u32| {
        let lo = at.max(0) as i64;
        let hi = (i64::from(at) + i64::from(len)).min(i64::from(size));
        (hi > lo).then(|| (lo as u32, (lo - i64::from(at)) as u32, (hi - lo) as u32))
    };
    let (Some((sx, dx, w)), Some((sy, dy, h))) = (clip(x, width, src.width()), clip(y, height, src.height())) else {
        return;
    };
    encoder.copy_texture_to_texture(
        wgpu::TexelCopyTextureInfo {
            texture: src,
            mip_level: 0,
            origin: wgpu::Origin3d { x: sx, y: sy, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyTextureInfo {
            texture: dst,
            mip_level: 0,
            origin: wgpu::Origin3d { x: dx, y: dy, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
}

fn make_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    key: PipeKey,
) -> wgpu::RenderPipeline {
    let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2, 2 => Float32x4, 3 => Uint32, 4 => Uint32, 5 => Float32, 6 => Uint32];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gs"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            // The GS does not cull.
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(key.write),
            depth_compare: Some(convert::depth_compare(key.test)),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: TARGET_FORMAT,
                blend: key.blend.map(convert::blend_state),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use piney_data::archive::Archive;
    use piney_data::iso::Iso;
    use piney_draw::{AlphaFail, AlphaTest, Compare, DrawState, Prim, PrimKind, Rgba, Vertex, ZTest};

    use super::*;

    fn gs() -> Option<Gs> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return None;
        }
        let archive = Archive::new(Iso::open(iso).ok()?.read_path("DATA/DATA.BIN").ok()?).ok()?;
        match Gs::headless(Assets::new(Arc::new(archive))) {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("no GPU ({e}); skipped");
                None
            }
        }
    }

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32, c: Rgba, state: DrawState) -> Cmd {
        let v = |x, y| Vertex { x, y, rgba: c, ..Default::default() };
        Cmd::Prim(Prim { kind: PrimKind::Sprite, gouraud: false, state, verts: vec![v(x0, y0), v(x1, y1)] })
    }

    fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * width + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    /// Untextured sprites land on exactly the pixels the GS fills, with the
    /// GS's colours, and blend as its ALPHA register says.
    #[test]
    fn sprites_fill_and_blend_like_the_gs() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        frame.clear = Rgba::new(0x40, 0x40, 0x40, 0);
        let opaque = DrawState { blend: None, ..DrawState::sprite(Blend::MIX, None) };
        // 10..20 x 10..20: pixels 10-19 inclusive.
        frame.cmds.push(rect(10.0, 10.0, 20.0, 20.0, Rgba::new(0xff, 0, 0, 0x80), opaque.clone()));
        // Half-transparent white over the grey: (0xff - 0x40) * 0x40 / 0x80 + 0x40.
        frame.cmds.push(rect(
            30.0,
            10.0,
            40.0,
            20.0,
            Rgba::new(0xff, 0xff, 0xff, 0x40),
            DrawState::sprite(Blend::MIX, None),
        ));
        // Additive: 0x40 + 0x20 * 0x80 / 0x80.
        frame.cmds.push(rect(
            50.0,
            10.0,
            60.0,
            20.0,
            Rgba::new(0x20, 0x20, 0x20, 0x80),
            DrawState::sprite(Blend::ADD, None),
        ));
        // An alpha test that fails everything, keeping nothing.
        let test = AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::Keep };
        frame.cmds.push(rect(
            70.0,
            10.0,
            80.0,
            20.0,
            Rgba::new(0, 0xff, 0, 0x80),
            DrawState { alpha_test: test, ..opaque },
        ));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        assert_eq!(pixel(&img, w, 10, 10), [0xff, 0, 0, 255]);
        assert_eq!(pixel(&img, w, 19, 19), [0xff, 0, 0, 255]);
        assert_eq!(pixel(&img, w, 20, 20), [0x40, 0x40, 0x40, 255]);
        assert_eq!(pixel(&img, w, 9, 10), [0x40, 0x40, 0x40, 255]);
        let mixed = pixel(&img, w, 35, 15)[0];
        assert!((0x9f..=0xa0).contains(&mixed), "{mixed:#x}");
        let added = pixel(&img, w, 55, 15)[0];
        assert!((0x5f..=0x61).contains(&added), "{added:#x}");
        assert_eq!(pixel(&img, w, 75, 15), [0x40, 0x40, 0x40, 255]);
    }

    /// A movie uploads a new true-colour picture every frame; only the
    /// current one stays cached. A small upload survives frames without it.
    #[test]
    fn movie_pictures_do_not_pile_up() {
        let Some(mut gs) = gs() else { return };
        let picture = |n: u8| piney_draw::Upload {
            id: 1,
            width: 640,
            height: 448,
            format: piney_draw::UploadFormat::Psmct32,
            pixels: vec![n; 640 * 448 * 4],
            clut: Vec::new(),
        };
        let line = piney_draw::Upload {
            id: 2,
            width: 16,
            height: 16,
            format: piney_draw::UploadFormat::Psmct32,
            pixels: vec![0x80; 16 * 16 * 4],
            clut: Vec::new(),
        };
        let uploads = |gs: &Gs| gs.textures.keys().filter(|k| matches!(k, TexKey::Upload(_))).count();
        let mut frame = Frame::new();
        frame.uploads = vec![line.clone()];
        gs.render(&frame);
        for n in 0..20 {
            frame.uploads = vec![picture(n)];
            gs.render(&frame);
            assert_eq!(uploads(&gs), 2);
        }
        frame.uploads = vec![line];
        gs.render(&frame);
        assert_eq!(uploads(&gs), 1);
    }

    /// NEVER with FB_ONLY, a blended model's test: the colour is drawn and
    /// the Z is not, so a farther draw after it still lands.
    #[test]
    fn never_fb_only_writes_no_z() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        let depth = piney_draw::Depth { test: ZTest::GEqual, write: true };
        let test = AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::FbOnly };
        let add = DrawState { depth, alpha_test: test, ..DrawState::sprite(Blend::ADD, None) };
        let opaque = DrawState { blend: None, depth, ..DrawState::sprite(Blend::MIX, None) };
        let sprite = |state: DrawState, z: u32, c: Rgba| {
            let v = |x, y| Vertex { x, y, z, rgba: c, ..Default::default() };
            Cmd::Prim(Prim { kind: PrimKind::Sprite, gouraud: false, state, verts: vec![v(10.0, 10.0), v(20.0, 20.0)] })
        };
        frame.cmds.push(sprite(add.clone(), 200, Rgba::new(0x40, 0x40, 0x40, 0x80)));
        frame.cmds.push(sprite(add, 200, Rgba::new(0x40, 0x40, 0x40, 0x80)));
        gs.render(&frame);
        let w = gs.target_size().0;
        // Additive: 0x40 twice over black.
        let c = pixel(&gs.read_back(), w, 15, 15)[0];
        assert!((0x7f..=0x81).contains(&c), "{c:#x}");
        frame.cmds.push(sprite(opaque, 100, Rgba::new(0xff, 0, 0, 0x80)));
        gs.render(&frame);
        assert_eq!(pixel(&gs.read_back(), w, 15, 15)[..3], [0xff, 0, 0]);
    }

    /// A shadow packet: the floor's Z copied small, a volume's near face
    /// (in front of the floor, counting up) and far face (behind it,
    /// counting down) over its left half, both faces in front over its
    /// right: the left half darkens by 0x30 of 0x80, the right does not.
    /// Two groups over one spot: the earlier in the list gives the alpha.
    #[test]
    fn shadows_darken_where_a_volume_meets_the_floor() {
        use piney_draw::{ShadowGroup, ShadowPass, ShadowPoly};
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        let depth = piney_draw::Depth { test: ZTest::Always, write: true };
        let opaque = DrawState { blend: None, depth, ..DrawState::sprite(Blend::MIX, None) };
        let v = |x, y| Vertex { x, y, z: 1000, rgba: Rgba::new(0x80, 0x80, 0x80, 0x80), ..Default::default() };
        frame.cmds.push(Cmd::Prim(Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state: opaque,
            verts: vec![v(0.0, 0.0), v(512.0, 448.0)],
        }));
        // Buffer pixels are 2 x 1.75 frame pixels.
        let quad = |x0: f32, y0: f32, x1: f32, y1: f32, z: f32, add: bool| ShadowPoly {
            verts: vec![[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]],
            add,
        };
        let volume = vec![
            quad(20.0, 20.0, 60.0, 60.0, 2000.0, true),
            quad(20.0, 20.0, 40.0, 60.0, 500.0, false),
            quad(40.0, 20.0, 60.0, 60.0, 1500.0, false),
        ];
        let other = vec![quad(100.0, 100.0, 140.0, 140.0, 2000.0, true)];
        frame.cmds.push(Cmd::Shadow(Box::new(ShadowPass {
            width: 256,
            height: 256,
            rect: [0.0, 0.0, 512.0, 448.0],
            darkness: 0x30,
            taps: vec![[0.0, 0.0]],
            groups: vec![
                ShadowGroup { alpha: 0x80, polys: volume },
                ShadowGroup { alpha: 0x40, polys: other.clone() },
                ShadowGroup { alpha: 0x80, polys: other },
            ],
        })));
        gs.render(&frame);
        let (w, _) = gs.target_size();
        let rgba = gs.read_back();
        // Buffer (30, 40) is frame (60, 70); (50, 40) is (100, 70).
        let left = pixel(&rgba, w, 60, 70);
        assert!((0x4f..=0x51).contains(&left[0]), "{left:x?}");
        assert_eq!(pixel(&rgba, w, 100, 70)[0], 0x80);
        assert_eq!(pixel(&rgba, w, 400, 400)[0], 0x80);
        // Of the two groups over (120, 120), the earlier's 0x40:
        // 0x80 - 0x80 * (0x40 * 0x30 / 0x80) / 0x80.
        let both = pixel(&rgba, w, 240, 210);
        assert!((0x67..=0x69).contains(&both[0]), "{both:x?}");
    }

    /// FB_ONLY with every pixel passing: Z is written triangle by triangle,
    /// so a farther layer drawn after a nearer one in the same draw is
    /// hidden, as a translucent model's back faces are on the GS.
    #[test]
    fn fb_only_hides_later_farther_layers() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        let test = AlphaTest::On { method: Compare::GEqual, reference: 0x40, fail: AlphaFail::FbOnly };
        let state = DrawState {
            depth: piney_draw::Depth { test: ZTest::GEqual, write: true },
            alpha_test: test,
            ..DrawState::sprite(Blend::MIX, None)
        };
        let v = |x, y, z| Vertex { x, y, z, rgba: Rgba::new(0xff, 0xff, 0xff, 0x60), ..Default::default() };
        // One draw: the nearer sprite, then a farther one over the same pixels.
        frame.cmds.push(Cmd::Prim(Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state,
            verts: vec![v(10.0, 10.0, 200), v(20.0, 20.0, 200), v(10.0, 10.0, 100), v(20.0, 20.0, 100)],
        }));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        // One layer over black: 0xff * 0x60 / 0x80.
        let c = pixel(&img, w, 15, 15)[0];
        assert!((0xbe..=0xc0).contains(&c), "{c:#x}");
    }

    /// Z a few units apart at the small GS Z the desktop's camera gives
    /// still decides: a farther sprite drawn later does not cover a nearer
    /// one, and a nearer one does.
    #[test]
    fn small_z_differences_decide() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        let state = DrawState {
            blend: None,
            depth: piney_draw::Depth { test: ZTest::GEqual, write: true },
            ..DrawState::sprite(Blend::MIX, None)
        };
        let at = |x: f32, z: u32, c: Rgba| {
            let v = |x, y| Vertex { x, y, z, rgba: c, ..Default::default() };
            Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state: state.clone(),
                verts: vec![v(x, 10.0), v(x + 10.0, 20.0)],
            })
        };
        let (red, blue) = (Rgba::new(0xff, 0, 0, 0x80), Rgba::new(0, 0, 0xff, 0x80));
        // Nearer (7,400) first, then farther (7,390) over it: red stays.
        // A tie (7,400.6 is Z 7,400) lets the later one through, as GEQUAL
        // does on the GS's integer Z.
        frame.cmds.push(at(10.0, 7400, red));
        frame.cmds.push(at(10.0, 7390, blue));
        // Farther first, then nearer: blue wins.
        frame.cmds.push(at(30.0, 7390, red));
        frame.cmds.push(at(30.0, 7400, blue));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        assert_eq!(pixel(&img, w, 15, 15)[..3], [0xff, 0, 0]);
        assert_eq!(pixel(&img, w, 35, 15)[..3], [0, 0, 0xff]);
    }

    /// SCISSOR_1 keeps a sprite inside its inclusive bounds.
    #[test]
    fn scissor_clips() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        let state = DrawState {
            blend: None,
            scissor: piney_draw::Scissor { x0: 12, x1: 15, y0: 12, y1: 13 },
            ..DrawState::sprite(Blend::MIX, None)
        };
        frame.cmds.push(rect(10.0, 10.0, 20.0, 20.0, Rgba::new(0xff, 0xff, 0xff, 0x80), state));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        let drawn = |x, y| pixel(&img, w, x, y)[0] == 0xff;
        assert!(drawn(12, 12) && drawn(15, 13));
        assert!(!drawn(11, 12) && !drawn(16, 12) && !drawn(12, 14) && !drawn(12, 11));
    }

    /// A command textured with the frame buffer sees what the commands
    /// before it drew and not what comes after (the pass splits around the
    /// copy); texels past the frame buffer are 0; a repeating copy wraps at
    /// its own height; the previous frame is the last one as it was left.
    #[test]
    fn frame_buffer_textures() {
        let Some(mut gs) = gs() else { return };
        let opaque = |tex: Option<TexRef>, wrap| DrawState {
            blend: None,
            texture: tex.map(|tex| piney_draw::TexState {
                tex,
                func: piney_draw::TexFunc::Modulate,
                use_alpha: false,
                filter: Filter::Nearest,
                wrap,
            }),
            ..DrawState::sprite(Blend::MIX, None)
        };
        let sprite = |x0: f32, y0: f32, x1: f32, y1: f32, uv: [f32; 4], rgba, state| {
            let v = |x, y, u, v| Vertex { x, y, u, v, rgba, ..Default::default() };
            Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state,
                verts: vec![v(x0, y0, uv[0], uv[1]), v(x1, y1, uv[2], uv[3])],
            })
        };
        let (red, blue) = (Rgba::new(0xc8, 0, 0, 0x80), Rgba::new(0, 0, 0xc8, 0x80));
        let mut frame = Frame::new();
        frame.clear = Rgba::new(0x40, 0x40, 0x40, 0);
        // Red on the frame's last 8 rows, then a copy of rows 440..456 drawn
        // 1:1 at (100, 100), then blue over the red.
        frame.cmds.push(sprite(0.0, 440.0, 16.0, 448.0, [0.0; 4], red, opaque(None, Wrap::Clamp)));
        let copy = TexRef::FrameBuffer { x: 0, y: 440, width: 16, height: 16 };
        frame.cmds.push(sprite(
            100.0,
            100.0,
            116.0,
            116.0,
            [0.0, 0.0, 16.0, 16.0],
            Rgba::NEUTRAL,
            opaque(Some(copy.clone()), Wrap::Clamp),
        ));
        frame.cmds.push(sprite(0.0, 440.0, 16.0, 448.0, [0.0; 4], blue, opaque(None, Wrap::Clamp)));
        // The copy again, 16 rows tall, read at v 16..24: it repeats.
        frame.cmds.push(sprite(
            200.0,
            100.0,
            216.0,
            108.0,
            [0.0, 16.0, 16.0, 24.0],
            Rgba::NEUTRAL,
            opaque(Some(copy), Wrap::Repeat),
        ));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        assert_eq!(pixel(&img, w, 105, 103)[..3], [0xc8, 0, 0], "copied before the blue");
        assert_eq!(pixel(&img, w, 105, 110)[..3], [0, 0, 0], "past the frame buffer");
        assert_eq!(pixel(&img, w, 5, 443)[..3], [0, 0, 0xc8]);
        assert_eq!(pixel(&img, w, 205, 103)[..3], [0, 0, 0xc8], "the second copy, wrapped");
        // The next frame shows the last one through the previous picture.
        let mut next = Frame::new();
        next.cmds.push(sprite(
            0.0,
            0.0,
            512.0,
            448.0,
            [0.0, 0.0, 512.0, 448.0],
            Rgba::NEUTRAL,
            opaque(Some(TexRef::PreviousFrame), Wrap::Clamp),
        ));
        gs.render(&next);
        let img = gs.read_back();
        assert_eq!(pixel(&img, w, 5, 443)[..3], [0, 0, 0xc8]);
        assert_eq!(pixel(&img, w, 105, 103)[..3], [0xc8, 0, 0]);
        assert_eq!(pixel(&img, w, 300, 300)[..3], [0x40, 0x40, 0x40]);
    }

    /// `gs`'s frame put on a `w` x `h` sRGB surface by the [`Presenter`],
    /// shown as `crtc` says, read back: RGB at (x, y).
    fn presented(gs: &Gs, w: u32, h: u32, crtc: Pcrtc) -> impl Fn(u32, u32) -> [u8; 3] + use<> {
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let target = gs.device().create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let presenter = Presenter::new(gs.device(), format);
        gs.queue().submit([presenter.present(gs, &view, w, h, crtc, None)]);
        let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buf = gs.device().create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = gs.device().create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        gs.queue().submit([enc.finish()]);
        buf.map_async(wgpu::MapMode::Read, .., |r| r.expect("map"));
        gs.device().poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let data = buf.get_mapped_range(..).expect("range").to_vec();
        move |x: u32, y: u32| {
            let i = (y * row + x * 4) as usize;
            [data[i], data[i + 1], data[i + 2]]
        }
    }

    /// On an sRGB surface the frame buffer's values arrive as they are,
    /// stretched to 4:3 between black bars.
    /// REGION_REPEAT (the stream shades' `SetShade(4, ...)`): mask 1024 - 4,
    /// fix 2, so each block of four texels reads its third.
    #[test]
    fn region_repeat_reads_in_blocks() {
        let Some(mut gs) = gs() else { return };
        let state = |tex: Option<TexRef>, wrap| DrawState {
            blend: None,
            texture: tex.map(|tex| piney_draw::TexState {
                tex,
                func: piney_draw::TexFunc::Modulate,
                use_alpha: false,
                filter: Filter::Nearest,
                wrap,
            }),
            ..DrawState::sprite(Blend::MIX, None)
        };
        let sprite = |x0: f32, x1: f32, y0: f32, y1: f32, uv: [f32; 4], rgba, st| {
            let v = |x, y, u, v| Vertex { x, y, u, v, rgba, ..Default::default() };
            Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state: st,
                verts: vec![v(x0, y0, uv[0], uv[1]), v(x1, y1, uv[2], uv[3])],
            })
        };
        let mut frame = Frame::new();
        // Eight one-texel columns of rising red on the frame's last rows.
        for k in 0..8u8 {
            let c = Rgba::new(0x10 * (k + 1), 0, 0, 0x80);
            let x = f32::from(k);
            frame.cmds.push(sprite(x, x + 1.0, 440.0, 448.0, [0.0; 4], c, state(None, Wrap::Clamp)));
        }
        let copy = TexRef::FrameBuffer { x: 0, y: 440, width: 8, height: 8 };
        let region = Wrap::Region { mask: 1024 - 4, fix: 2 };
        frame.cmds.push(sprite(
            100.0,
            108.0,
            100.0,
            104.0,
            [0.0, 0.0, 8.0, 4.0],
            Rgba::NEUTRAL,
            state(Some(copy), region),
        ));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        let reds: Vec<u8> = (100..108).map(|x| pixel(&img, w, x, 101)[0]).collect();
        assert_eq!(reds, [0x30, 0x30, 0x30, 0x30, 0x70, 0x70, 0x70, 0x70]);
    }

    /// The frame read through a copy at half its width
    /// (`TexRef::ScaledFrame`, the field's depth shade): a black left half
    /// and a red right half copy to 128 black and 128 red texels (each the
    /// mean of two pixels), which a 512-wide sprite reads back bilinearly:
    /// black, a pixel halfway, red.
    #[test]
    fn a_scaled_frame_reads_through_the_smaller_copy() {
        let Some(mut gs) = gs() else { return };
        let plain = DrawState::sprite(Blend::MIX, None);
        let sprite = |x0: f32, x1: f32, y0: f32, y1: f32, uv: [f32; 4], rgba, st: DrawState| {
            let v = |x, y, u, v| Vertex { x, y, u, v, rgba, ..Default::default() };
            Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state: st,
                verts: vec![v(x0, y0, uv[0], uv[1]), v(x1, y1, uv[2], uv[3])],
            })
        };
        let mut frame = Frame::new();
        frame.cmds.push(sprite(0.0, 256.0, 0.0, 448.0, [0.0; 4], Rgba::new(0, 0, 0, 0x80), plain.clone()));
        frame.cmds.push(sprite(256.0, 512.0, 0.0, 448.0, [0.0; 4], Rgba::new(0xff, 0, 0, 0x80), plain.clone()));
        let scaled = DrawState {
            blend: None,
            texture: Some(piney_draw::TexState {
                tex: TexRef::ScaledFrame { width: 256, height: 128 },
                func: piney_draw::TexFunc::Decal,
                use_alpha: false,
                filter: Filter::Linear,
                wrap: Wrap::Clamp,
            }),
            ..plain
        };
        frame.cmds.push(sprite(0.0, 512.0, 0.0, 448.0, [0.0, 0.0, 256.0, 128.0], Rgba::NEUTRAL, scaled));
        gs.render(&frame);
        let img = gs.read_back();
        let w = gs.target_size().0;
        let reds: Vec<u8> = (253..259).map(|x| pixel(&img, w, x, 200)[0]).collect();
        assert_eq!(reds[..2], [0, 0], "{reds:?}");
        assert!(reds[3].abs_diff(0x80) <= 2, "{reds:?}");
        assert_eq!(reds[4..], [0xff, 0xff], "{reds:?}");
    }

    #[test]
    fn present_letterboxes_and_keeps_values() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        frame.clear = Rgba::new(0x33, 0x66, 0x99, 0);
        gs.render(&frame);
        let at = presented(&gs, 800, 450, Pcrtc::default());
        // 450 high at 4:3 is 600 wide: bars of 100 each side.
        assert_eq!(at(50, 225), [0, 0, 0]);
        assert_eq!(at(750, 225), [0, 0, 0]);
        let mid = at(400, 225);
        for (got, want) in mid.iter().zip([0x33u8, 0x66, 0x99]) {
            assert!(got.abs_diff(want) <= 1, "{mid:?}");
        }
    }

    /// The display offset moves the picture inside its 4:3 box, and black
    /// shows where it left: an eighth across and down is 75 pixels of 600
    /// and 56 of 450 (rounded).
    #[test]
    fn present_moves_the_picture_by_the_display_offset() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        frame.clear = Rgba::new(0x33, 0x66, 0x99, 0);
        gs.render(&frame);
        let at = presented(&gs, 600, 450, Pcrtc { offset: [64.0, 56.0], ..Pcrtc::default() });
        assert_eq!(at(70, 225), [0, 0, 0], "left of the moved picture");
        assert_eq!(at(300, 52), [0, 0, 0], "above it");
        for p in [at(80, 225), at(300, 60), at(599, 449)] {
            for (got, want) in p.iter().zip([0x33u8, 0x66, 0x99]) {
                assert!(got.abs_diff(want) <= 1, "{p:?}");
            }
        }
    }

    /// With the deflicker, each line is merged with the one above it at
    /// PMODE.ALP 127: a white line on black comes out in two rows of about
    /// half, the first a little brighter. A movie's frame (field mode) is not
    /// merged.
    #[test]
    fn present_merges_the_line_above_in_frame_mode() {
        let Some(mut gs) = gs() else { return };
        let line = |field_mode| {
            let mut frame = Frame::new();
            frame.clear = Rgba::new(0, 0, 0, 0);
            frame.field_mode = field_mode;
            let opaque = DrawState { blend: None, ..DrawState::sprite(Blend::MIX, None) };
            frame.cmds.push(rect(0.0, 200.0, 512.0, 201.0, Rgba::new(0xff, 0xff, 0xff, 0x80), opaque));
            frame
        };
        // 448 rows high: a row of the frame is a row of the window.
        let (w, h) = (640, 448);
        let on = Pcrtc { deflicker: true, ..Pcrtc::default() };
        gs.render(&line(false));
        let sharp = presented(&gs, w, h, Pcrtc::default());
        assert_eq!([sharp(320, 199)[0], sharp(320, 200)[0], sharp(320, 201)[0]], [0, 255, 0]);
        let at = presented(&gs, w, h, on);
        let rows = [at(320, 199)[0], at(320, 200)[0], at(320, 201)[0], at(320, 202)[0]];
        assert!(rows[0] == 0 && rows[3] == 0, "{rows:?}");
        assert!(rows[1].abs_diff(128) <= 1 && rows[2].abs_diff(127) <= 1, "{rows:?}");
        gs.render(&line(true));
        let movie = presented(&gs, w, h, on);
        assert_eq!([movie(320, 200)[0], movie(320, 201)[0]], [255, 0]);
    }

    /// Scaled up by more than 2, a 1-pixel line keeps a pure core wherever
    /// it falls against the window's pixels (sharp bilinear). Plain
    /// bilinear at this scale greys some lines' peaks, which is what made
    /// the desktop's labels look uneven.
    #[test]
    fn present_keeps_thin_lines_whole() {
        let Some(mut gs) = gs() else { return };
        let mut frame = Frame::new();
        frame.clear = Rgba::new(0, 0, 0, 0);
        let opaque = DrawState { blend: None, ..DrawState::sprite(Blend::MIX, None) };
        let lines: Vec<u32> = (0..12).map(|k| 100 + 7 * k).collect();
        for &x in &lines {
            frame.cmds.push(rect(
                x as f32,
                0.0,
                x as f32 + 1.0,
                448.0,
                Rgba::new(0xff, 0xff, 0xff, 0x80),
                opaque.clone(),
            ));
        }
        gs.render(&frame);
        let (w, h) = (1600u32, 1200u32);
        let at = presented(&gs, w, h, Pcrtc::default());
        let src_w = gs.target_size().0 as f32;
        let scale = w as f32 / src_w;
        assert!(scale > 2.0, "scale {scale}");
        for &x in &lines {
            let lo = (x as f32 * scale).floor() as u32;
            let hi = ((x + 1) as f32 * scale).ceil() as u32;
            let peak = (lo..hi).map(|px| at(px, h / 2)[0]).max().unwrap();
            assert!(peak >= 254, "line at {x}: peak {peak} over window pixels {lo}..{hi}");
        }
    }

    /// A texture drawn with MODULATE at 0x80 comes out as stored.
    #[test]
    fn modulate_at_0x80_is_the_texture() {
        let Some(mut gs) = gs() else { return };
        let f = gs.assets.file("xddesk01").expect("xddesk01");
        let t = f.textures.iter().find(|t| t.width(0) >= 16 && t.height(0) >= 16).expect("a texture");
        let (tex, clut, tw, th) = (t.object, t.clut, t.width(0), t.height(0));
        let img = gs.assets.image("xddesk01", tex, clut).unwrap();
        let mut frame = Frame::new();
        let state = DrawState {
            blend: None,
            ..DrawState::sprite(
                Blend::MIX,
                Some(piney_draw::TexState::modulate(TexRef::Ccs { file: "xddesk01".into(), texture: tex, clut })),
            )
        };
        let v = |x: f32, y: f32, u: f32, vv: f32| Vertex { x, y, u, v: vv, rgba: Rgba::NEUTRAL, ..Default::default() };
        frame.cmds.push(Cmd::Prim(Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state,
            verts: vec![v(0.0, 0.0, 0.0, 0.0), v(tw as f32, th as f32, tw as f32, th as f32)],
        }));
        gs.render(&frame);
        let out = gs.read_back();
        let w = gs.target_size().0;
        let mut worst = 0;
        for y in 0..th.min(64) {
            for x in 0..tw.min(64) {
                let got = pixel(&out, w, x, y);
                let i = ((y * tw + x) * 4) as usize;
                for (g, want) in got.iter().zip(&img.rgba[i..i + 3]) {
                    worst = worst.max((i32::from(*g) - i32::from(*want)).abs());
                }
            }
        }
        assert!(worst <= 1, "worst channel difference {worst}");
    }
}
