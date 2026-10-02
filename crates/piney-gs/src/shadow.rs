//! [`ShadowPass`] on the GPU: the steps `ccShadowPacket::SetShadowPacket` sends
//! the GS (`docs/engine/shadow.md`), each a render pass of its own: the Z copy
//! (`TransFrameBuffer`); per group, the last first, the polygons counted +1 or
//! -1 against the buffer's depth and the counted pixels given the group's alpha
//! (`FillTexBuffer`, `FillShadowBuff`); then black over the rectangle at the
//! alpha times the darkness, read bilinearly (`TransShadowTex`).

use std::collections::HashMap;

use piney_draw::ShadowPass;
use wgpu::util::DeviceExt;

/// The main target's Z, as `render` keeps it: z / 2^32 in a float.
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const COUNT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
const ALPHA: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// What every step reads: the rectangle (frame pixels x0, y0, x1, y1), the
/// buffer's size, the group's alpha, the darkness, the frame's size, and the
/// frame buffer's scale (its pixels to one of the frame's each way).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    rect: [f32; 4],
    size: [f32; 2],
    alpha: f32,
    darkness: f32,
    frame: [f32; 2],
    scale: f32,
    pad: f32,
}

const COMMON: &str = r#"
struct U {
    rect: vec4<f32>,
    size: vec2<f32>,
    alpha: f32,
    darkness: f32,
    frame: vec2<f32>,
    scale: f32,
    pad: f32,
};

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}
"#;

const ZCOPY: &str = r#"
@group(0) @binding(0) var zsrc: texture_depth_2d;
@group(0) @binding(1) var<uniform> u: U;

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @builtin(frag_depth) f32 {
    // Buffer pixel (i, j) takes the frame's texel at x0 + i W / width,
    // unfiltered.
    let ij = floor(p.xy);
    let s = (u.rect.zw - u.rect.xy) / u.size;
    let t = vec2<i32>(floor((u.rect.xy + ij * s) * u.scale));
    let dims = vec2<i32>(textureDimensions(zsrc));
    return textureLoad(zsrc, clamp(t, vec2<i32>(0), dims - vec2<i32>(1)), 0);
}
"#;

const COUNT_SHADER: &str = r#"
struct V {
    @location(0) clip: vec4<f32>,
    @location(1) sign: f32,
};
struct O {
    @builtin(position) pos: vec4<f32>,
    @location(0) @interpolate(flat) sign: f32,
};
struct F {
    @location(0) count: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@vertex
fn vs(v: V) -> O {
    var o: O;
    o.pos = v.clip;
    o.sign = v.sign;
    return o;
}

@fragment
fn fs(i: O) -> F {
    var f: F;
    f.count = vec4<f32>(i.sign, 0.0, 0.0, 0.0);
    // Z comes at half scale (z / 2^33, room past 2^32 before the clip);
    // VU1's ftoi4 holds it at 0x7fffffff.
    let z = min(floor(i.pos.z * 8589934592.0), 2147483647.0);
    f.depth = z / 4294967296.0;
    return f;
}
"#;

const RESOLVE: &str = r#"
@group(0) @binding(0) var count: texture_2d<f32>;
@group(0) @binding(1) var<uniform> u: U;

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let n = textureLoad(count, vec2<i32>(floor(p.xy)), 0).r;
    if (n < 0.5) {
        discard;
    }
    return vec4<f32>(u.alpha / 255.0, 0.0, 0.0, 0.0);
}
"#;

const COMPOSITE: &str = r#"
@group(0) @binding(0) var alpha: texture_2d<f32>;
@group(0) @binding(1) var<uniform> u: U;

fn texel(t: vec2<i32>) -> f32 {
    let hi = vec2<i32>(u.size) - vec2<i32>(1);
    return floor(textureLoad(alpha, clamp(t, vec2<i32>(0), hi), 0).r * 255.0 + 0.5);
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    // The GS samples at the pixel's corner; bilinear about texel centres.
    let corner = (p.xy - vec2<f32>(0.5)) / u.scale;
    let q = (corner - u.rect.xy) * u.size / (u.rect.zw - u.rect.xy) - vec2<f32>(0.5);
    let b = floor(q);
    let w = q - b;
    let t = vec2<i32>(b);
    let top = mix(texel(t), texel(t + vec2<i32>(1, 0)), w.x);
    let bottom = mix(texel(t + vec2<i32>(0, 1)), texel(t + vec2<i32>(1, 1)), w.x);
    let ta = floor(mix(top, bottom, w.y));
    // MODULATE by the vertex alpha (the darkness), then MIX with black.
    let a = floor(ta * u.darkness / 128.0);
    return vec4<f32>(0.0, 0.0, 0.0, clamp(a / 128.0, 0.0, 1.0));
}
"#;

/// One vertex of a counted polygon.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CountVertex {
    clip: [f32; 4],
    sign: f32,
}

/// The buffer-sized textures, kept by size.
struct Buffers {
    depth: wgpu::TextureView,
    count: wgpu::TextureView,
    alpha: wgpu::TextureView,
}

pub struct ShadowGpu {
    zcopy: wgpu::RenderPipeline,
    count: wgpu::RenderPipeline,
    resolve: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    depth_layout: wgpu::BindGroupLayout,
    float_layout: wgpu::BindGroupLayout,
    buffers: HashMap<(u32, u32), Buffers>,
}

fn layout(device: &wgpu::Device, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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

#[allow(clippy::too_many_arguments)]
fn pipeline(
    device: &wgpu::Device,
    label: &str,
    source: &str,
    bind: Option<&wgpu::BindGroupLayout>,
    buffers: &[Option<wgpu::VertexBufferLayout>],
    target: Option<wgpu::ColorTargetState>,
    depth: Option<wgpu::DepthStencilState>,
    full: bool,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(format!("{COMMON}{source}").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &bind.map(|b| vec![Some(b)]).unwrap_or_default(),
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some(if full { "vs_full" } else { "vs" }),
            compilation_options: Default::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: depth,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &target.map(|t| vec![Some(t)]).unwrap_or_default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl ShadowGpu {
    pub fn new(device: &wgpu::Device, frame_format: wgpu::TextureFormat) -> ShadowGpu {
        let depth_layout = layout(device, wgpu::TextureSampleType::Depth);
        let float_layout = layout(device, wgpu::TextureSampleType::Float { filterable: false });
        let depth = |compare, write| wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(write),
            depth_compare: Some(compare),
            stencil: Default::default(),
            bias: Default::default(),
        };
        let zcopy = pipeline(
            device,
            "shadow z copy",
            ZCOPY,
            Some(&depth_layout),
            &[],
            None,
            Some(depth(wgpu::CompareFunction::Always, true)),
            true,
        );
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32];
        let count = pipeline(
            device,
            "shadow count",
            COUNT_SHADER,
            None,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<CountVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
            Some(wgpu::ColorTargetState {
                format: COUNT,
                blend: Some(wgpu::BlendState { color: add, alpha: add }),
                write_mask: wgpu::ColorWrites::RED,
            }),
            Some(depth(wgpu::CompareFunction::Greater, false)),
            false,
        );
        let resolve = pipeline(
            device,
            "shadow resolve",
            RESOLVE,
            Some(&float_layout),
            &[],
            Some(wgpu::ColorTargetState { format: ALPHA, blend: None, write_mask: wgpu::ColorWrites::RED }),
            None,
            true,
        );
        // Cd - Cd As (the black's MIX), and As into the frame's alpha.
        let composite = pipeline(
            device,
            "shadow composite",
            COMPOSITE,
            Some(&float_layout),
            &[],
            Some(wgpu::ColorTargetState {
                format: frame_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            None,
            true,
        );
        ShadowGpu { zcopy, count, resolve, composite, depth_layout, float_layout, buffers: HashMap::new() }
    }

    fn buffers(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        self.buffers.entry((w, h)).or_insert_with(|| {
            let make = |format, label| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            Buffers {
                depth: make(DEPTH, "shadow z"),
                count: make(COUNT, "shadow count"),
                alpha: make(ALPHA, "shadow alpha"),
            }
        });
    }

    /// Run `pass` over the frame: `frame` its colour target, `frame_depth`
    /// its Z buffer (not attached meanwhile).
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: &wgpu::TextureView,
        frame_depth: &wgpu::TextureView,
        frame_size: (u32, u32),
        scale: u32,
        pass: &ShadowPass,
    ) {
        let (w, h) = (u32::from(pass.width), u32::from(pass.height));
        let [x0, y0, x1, y1] = pass.rect;
        if w == 0 || h == 0 || x1 <= x0 || y1 <= y0 {
            return;
        }
        let uniform = |alpha: u8| {
            let u = Uniform {
                rect: pass.rect,
                size: [w as f32, h as f32],
                alpha: f32::from(alpha),
                darkness: f32::from(pass.darkness),
                frame: [frame_size.0 as f32, frame_size.1 as f32],
                scale: scale as f32,
                pad: 0.0,
            };
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadow"),
                contents: bytemuck::bytes_of(&u),
                usage: wgpu::BufferUsages::UNIFORM,
            })
        };
        let bind = |layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                    wgpu::BindGroupEntry { binding: 1, resource: buffer.as_entire_binding() },
                ],
            })
        };
        let common = uniform(0);
        let zgroup = bind(&self.depth_layout, frame_depth, &common);
        self.buffers(device, w, h);
        let (zcopy, count_pipe, resolve, composite) = (&self.zcopy, &self.count, &self.resolve, &self.composite);
        let float_layout = &self.float_layout;
        let b = &self.buffers[&(w, h)];
        // 1. The Z copy.
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow z copy"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &b.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(zcopy);
            rp.set_bind_group(0, &zgroup, &[]);
            rp.draw(0..3, 0..1);
        }
        // The alpha starts at 0 (FillTexBuffer's last fill).
        clear(encoder, &b.alpha);
        // 2. Each group, the last first.
        let to_clip = |x: f32, y: f32, z: f32| {
            // Half-scale Z: see the count shader.
            let z = z.clamp(0.0, 4_294_967_295.0) / 8_589_934_592.0;
            [x / (w as f32 / 2.0) - 1.0, 1.0 - y / (h as f32 / 2.0), z, 1.0]
        };
        for g in pass.groups.iter().rev() {
            let mut verts = Vec::new();
            for p in &g.polys {
                let sign = if p.add { 1.0 } else { -1.0 };
                // The GS takes a pixel whose corner is inside; wgpu, whose
                // centre is: half a pixel over.
                let v = |q: [f32; 3]| CountVertex { clip: to_clip(q[0] + 0.5, q[1] + 0.5, q[2]), sign };
                for k in 2..p.verts.len() {
                    verts.extend([v(p.verts[0]), v(p.verts[k - 1]), v(p.verts[k])]);
                }
            }
            if verts.is_empty() {
                continue;
            }
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadow polygons"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            {
                let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shadow count"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &b.count,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &b.depth,
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rp.set_pipeline(count_pipe);
                rp.set_vertex_buffer(0, vb.slice(..));
                rp.draw(0..verts.len() as u32, 0..1);
            }
            let ub = uniform(g.alpha);
            let group = bind(float_layout, &b.count, &ub);
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &b.alpha,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(resolve);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        // 3. The composite over the pixels whose corner is in the rectangle.
        let (fw, fh) = frame_size;
        let at = |v: f32, max: u32| ((v.ceil().max(0.0) as u32) * scale).min(max);
        let (px0, py0) = (at(x0, fw), at(y0, fh));
        let (px1, py1) = (at(x1, fw), at(y1, fh));
        if px1 <= px0 || py1 <= py0 {
            return;
        }
        let group = bind(float_layout, &b.alpha, &common);
        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rp.set_scissor_rect(px0, py0, px1 - px0, py1 - py0);
        rp.set_pipeline(composite);
        rp.set_bind_group(0, &group, &[]);
        rp.draw(0..3, 0..1);
    }
}

/// A texture cleared to 0.
fn clear(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("shadow clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}
