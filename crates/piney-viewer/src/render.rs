//! wgpu: one pipeline, a scene's buffers and textures, and two targets - a
//! window surface, or an offscreen texture read back for `--shot`.

use std::sync::Arc;

use glam::{Mat4, Vec4};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::mesh::{Env, Mesh, Vertex};

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CLEAR: wgpu::Color = wgpu::Color { r: 0.02, g: 0.025, b: 0.035, a: 1.0 };

const SHADER: &str = r#"
struct Frame {
    view_proj: mat4x4<f32>,
    light: vec4<f32>,
    // near, far, the fog fraction at far, 1 when fog is on
    fog: vec4<f32>,
    // the fog colour, in the art's gamma
    fog_colour: vec4<f32>,
    // ambient light for lit models, w 1 when the scene sets one
    ambient: vec4<f32>,
};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) colour: vec4<f32>,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    // The view depth, as VU1's w.
    @location(3) depth: f32,
};

@vertex
fn vs(v: VIn) -> VOut {
    var o: VOut;
    o.clip = frame.view_proj * vec4<f32>(v.pos, 1.0);
    o.normal = v.normal;
    o.uv = v.uv;
    // Vertex colours are in the art's gamma space, like the textures; the
    // GS multiplies them there. A power is multiplicative, so decoding each
    // side to linear first gives the same product.
    o.colour = vec4<f32>(pow(max(v.colour.rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), v.colour.a);
    o.depth = o.clip.w;
    return o;
}

@fragment
fn fs(i: VOut) -> @location(0) vec4<f32> {
    var c = textureSample(tex, samp, i.uv) * i.colour;
    if (dot(i.normal, i.normal) > 0.25) {
        // Two-sided: the GS does not cull and winding is not consistent.
        let d = abs(dot(normalize(i.normal), frame.light.xyz));
        if (frame.ambient.w > 0.5) {
            // VU1's lit colour, ambient plus lights, capped at 1, in the
            // art's gamma; the viewer's light stands in for the room's.
            let l = min(frame.ambient.rgb + vec3<f32>(0.55 * d), vec3<f32>(1.0));
            c = vec4<f32>(c.rgb * pow(l, vec3<f32>(2.2)), c.a);
        } else {
            c = vec4<f32>(c.rgb * (0.45 + 0.55 * d), c.a);
        }
    }
    if (c.a < 0.02) {
        discard;
    }
    if (frame.fog.w > 0.5) {
        // The GS blends the fog colour in by the fog value, in its own
        // (the art's) gamma.
        let t = clamp((i.depth - frame.fog.x) / (frame.fog.y - frame.fog.x), 0.0, 1.0) * frame.fog.z;
        let g = mix(pow(c.rgb, vec3<f32>(1.0 / 2.2)), frame.fog_colour.rgb, t);
        c = vec4<f32>(pow(g, vec3<f32>(2.2)), c.a);
    }
    // light.w: the viewer's exposure (1 = as the GS would write it).
    return vec4<f32>(c.rgb * frame.light.w, c.a);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    view_proj: [[f32; 4]; 4],
    light: [f32; 4],
    fog: [f32; 4],
    fog_colour: [f32; 4],
    ambient: [f32; 4],
}

pub struct GpuScene {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    groups: Vec<wgpu::BindGroup>,
    /// (texture, blend, first index, index count).
    batches: Vec<(Option<usize>, u8, u32, u32)>,
    /// The fog and light the scene is drawn in.
    env: Option<Env>,
}

/// Device, queue and everything that does not depend on the target.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// One per blend type, indexed by `Batch::blend`.
    pipelines: [wgpu::RenderPipeline; 4],
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    frame_buffer: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    white: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> Result<Self, String> {
        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor { label: Some("piney"), ..Default::default() }),
        )
        .map_err(|e| e.to_string())?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("piney shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("piney layout"),
            bind_group_layouts: &[Some(&frame_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let pipelines = [0u8, 1, 2, 3].map(|blend| make_pipeline(&device, &layout, &shader, format, blend));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("repeat"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: frame_buffer.as_entire_binding() }],
        });
        let white = texture_group(&device, &queue, &texture_layout, &sampler, 1, 1, &[vec![255; 4]]);
        Ok(Renderer { device, queue, pipelines, texture_layout, sampler, frame_buffer, frame_group, white })
    }

    pub fn upload(&self, mesh: &Mesh) -> GpuScene {
        let groups = mesh
            .textures
            .iter()
            .map(|t| {
                texture_group(
                    &self.device,
                    &self.queue,
                    &self.texture_layout,
                    &self.sampler,
                    t.width,
                    t.height,
                    &t.levels,
                )
            })
            .collect();
        // wgpu refuses zero-sized buffers.
        let vbytes: &[u8] = if mesh.vertices.is_empty() { &[0; 64] } else { bytemuck::cast_slice(&mesh.vertices) };
        let ibytes: &[u8] = if mesh.indices.is_empty() { &[0; 16] } else { bytemuck::cast_slice(&mesh.indices) };
        GpuScene {
            vertices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices"),
                contents: vbytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
            indices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("indices"),
                contents: ibytes,
                usage: wgpu::BufferUsages::INDEX,
            }),
            groups,
            batches: mesh.batches.iter().map(|b| (b.texture, b.blend, b.first, b.count)).collect(),
            env: mesh.env,
        }
    }

    /// Replace `scene`'s vertices from `first` on (an animation posed them
    /// again).
    pub fn update_vertices(&self, scene: &GpuScene, first: usize, vertices: &[Vertex]) {
        let offset = (first * std::mem::size_of::<Vertex>()) as u64;
        self.queue.write_buffer(&scene.vertices, offset, bytemuck::cast_slice(vertices));
    }

    /// Record one frame of `scene` into `color`; `fog` turns the scene's
    /// fog on, when it has one.
    fn encode(
        &self,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        scene: Option<&GpuScene>,
        view_proj: Mat4,
        light: Vec4,
        fog: bool,
    ) -> wgpu::CommandEncoder {
        let env = scene.and_then(|s| s.env);
        let uniform = FrameUniform {
            view_proj: view_proj.to_cols_array_2d(),
            light: light.to_array(),
            fog: env.map_or([0.0; 4], |e| [e.fog_near, e.fog_far, e.fog_max, f32::from(u8::from(fog))]),
            fog_colour: env.map_or([0.0; 4], |e| [e.fog_colour[0], e.fog_colour[1], e.fog_colour[2], 0.0]),
            ambient: env.map_or([0.0; 4], |e| [e.ambient[0], e.ambient[1], e.ambient[2], 1.0]),
        };
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&uniform));
        // The game clears to the fog colour (`DUNGEON.fog` into ccSys+0x18);
        // wgpu takes a clear colour linear.
        let clear = match env {
            Some(e) => {
                let [r, g, b] = e.fog_colour.map(|c| f64::from(c).powf(2.2));
                wgpu::Color { r, g, b, a: 1.0 }
            }
            None => CLEAR,
        };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(s) = scene {
                pass.set_bind_group(0, &self.frame_group, &[]);
                pass.set_vertex_buffer(0, s.vertices.slice(..));
                pass.set_index_buffer(s.indices.slice(..), wgpu::IndexFormat::Uint32);
                let mut current = None;
                for &(tex, blend, first, count) in &s.batches {
                    let blend = blend.min(3) as usize;
                    if current != Some(blend) {
                        pass.set_pipeline(&self.pipelines[blend]);
                        current = Some(blend);
                    }
                    let group = tex.map(|t| &s.groups[t]).unwrap_or(&self.white);
                    pass.set_bind_group(1, group, &[]);
                    pass.draw_indexed(first..first + count, 0, 0..1);
                }
            }
        }
        encoder
    }

    /// Render one frame offscreen and read it back as RGBA8 rows, top first.
    pub fn snapshot(
        &self,
        scene: Option<&GpuScene>,
        view_proj: Mat4,
        light: Vec4,
        fog: bool,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shot"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHOT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = depth_view(&self.device, width, height);
        let view = target.create_view(&Default::default());
        let mut encoder = self.encode(&view, &depth, scene, view_proj, light, fog);
        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
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
            size,
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.expect("map readback"));
        self.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let data = readback.get_mapped_range(..).expect("mapped range");
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            out.extend_from_slice(&data[y * row as usize..y * row as usize + width as usize * 4]);
        }
        out
    }
}

/// The offscreen format: sRGB like a window surface, so shots match the
/// window.
pub const SHOT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// A window and its surface, with the renderer that draws into it.
pub struct Gpu {
    pub window: Arc<Window>,
    pub renderer: Renderer,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
}

impl Gpu {
    pub fn new(instance: &wgpu::Instance, window: Arc<Window>) -> Result<Self, String> {
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("the surface is not supported by this adapter")?;
        let caps = surface.get_capabilities(&adapter);
        let caps_alpha = caps.alpha_modes.clone();
        if let Some(f) = caps.formats.iter().find(|f| f.is_srgb()) {
            config.format = *f;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        // Opaque, or a compositor shows what is behind the window through the
        // frame's alpha.
        if caps_alpha.contains(&wgpu::CompositeAlphaMode::Opaque) {
            config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        }
        let renderer = Renderer::new(&adapter, config.format)?;
        surface.configure(&renderer.device, &config);
        let depth = depth_view(&renderer.device, config.width, config.height);
        Ok(Gpu { window, renderer, surface, config, depth })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.renderer.device, &self.config);
        self.depth = depth_view(&self.renderer.device, width, height);
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    pub fn draw(&mut self, scene: Option<&GpuScene>, view_proj: Mat4, light: Vec4, fog: bool) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.renderer.device, &self.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let encoder = self.renderer.encode(&view, &self.depth, scene, view_proj, light, fog);
        self.renderer.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.renderer.queue.present(frame);
    }
}

/// A renderer with no window, for `--shot`.
pub fn headless() -> Result<Renderer, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .map_err(|e| e.to_string())?;
    Renderer::new(&adapter, SHOT_FORMAT)
}

fn depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn texture_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
    levels: &[Vec<u8>],
) -> wgpu::BindGroup {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, rgba) in levels.iter().enumerate() {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }
    let view = texture.create_view(&Default::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

/// The pipeline for one `alphaBlendTbl` entry. GS alpha blending is
/// `((A - B) * C >> 7) + D`; entries 0-3 are, with As the source alpha:
/// 0 `(Cs - Cd) * As + Cd`, 1 `Cs * As + Cd`, 2 `Cd - Cs * As`,
/// 3 `Cd * As + Cs`. `ccModel::Init` turns depth writes off for 1-3.
fn make_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: u8,
) -> wgpu::RenderPipeline {
    use wgpu::{BlendComponent, BlendFactor as F, BlendOperation as Op};
    let keep_alpha = BlendComponent { src_factor: F::Zero, dst_factor: F::One, operation: Op::Add };
    let state = match blend {
        0 => wgpu::BlendState::ALPHA_BLENDING,
        1 => wgpu::BlendState {
            color: BlendComponent { src_factor: F::SrcAlpha, dst_factor: F::One, operation: Op::Add },
            alpha: keep_alpha,
        },
        2 => wgpu::BlendState {
            color: BlendComponent { src_factor: F::SrcAlpha, dst_factor: F::One, operation: Op::ReverseSubtract },
            alpha: keep_alpha,
        },
        _ => wgpu::BlendState {
            color: BlendComponent { src_factor: F::One, dst_factor: F::SrcAlpha, operation: Op::Add },
            alpha: keep_alpha,
        },
    };
    let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("piney pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(blend == 0),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend: Some(state), write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
