//! A scene's frame: `ccStreamDrawLayerList::Draw` (0x00148400) through
//! `ccObj::Draw(1.0)` (0x0013f220) and `ccModel::Draw` (0x0013eab0), with the
//! scene's camera, ambient and lights (`docs/engine/stream.md`, "The draw").
//! Every layer shares `sysLayer`'s view ([`Scene::frame`]), the camera from
//! the last `F_Camera`; lights by `ccDrawEnv::SetLightMatrix` (0x00105900);
//! the effect task's fog ([`Scene::fog`]) and render state; and the view's
//! `divZ` ([`Scene::div_z`]), below which VU1 clips a triangle instead of
//! dropping it, so the draws never ask the renderer to drop triangles.

use glam::{Mat3, Mat4, Vec3};
use piney_desktop::anm::{Ctx, OPAQUE_TRANSPARENCY, model_state, sort_key};
use piney_desktop::assets::{TEX_FLAG_CLAMP, TEX_FLAG_SORTED};
use piney_desktop::camera::frame_projection;
use piney_desktop::view::View;
use piney_draw::{AlphaFail, AlphaTest, Cmd, Compare, Filter, Lights, MmatDraw, ModelDraw, TexFunc, TexParams, Wrap};

use crate::file;
use crate::scene::{Light, Loaded, Scene, to_mat4};

/// `ccObj::Draw` skips an object at or below this transparency
/// (`c.le.s tp, 1/128` at 0x0013f2b0), and `ccModel::Draw` a mmat below it.
pub const MIN_TRANSPARENCY: f32 = 1.0 / 128.0;

/// `mc_SetMatrix`'s `loi 2.0` on the light colours.
pub const VU_LIGHT_SCALE: f32 = 2.0;

/// Model type bit: lit.
pub const MTYPE_LIT: u16 = 1;

/// Draw `scene` into `ctx`.
pub fn draw(ctx: &mut Ctx, loaded: &Loaded, scene: &Scene) {
    let view = scene_view(scene);
    ctx.view = view.clone();
    ctx.layers.shadows = shadows(scene, &view);
    for (layer, nodes) in &scene.draw_list {
        for &i in nodes {
            cast_shadow(ctx, loaded, scene, i);
            draw_node(ctx, &view, loaded, scene, i, *layer);
        }
    }
}

/// `InitScene`'s shadow packets (mode 1, darkness 0x20, each on its shadow
/// layer) over the scene's view: its frame's clip box
/// (`ccView::GetScreenClip`), near 8, far 2^20.
fn shadows(scene: &Scene, view: &View) -> piney_desktop::shadow::Shadows {
    use piney_desktop::shadow::{ShadowPacket, ShadowView, Shadows};
    use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
    if scene.shadows.is_empty() {
        return Shadows::default();
    }
    let (lo, hi) = scene.frame.bbox();
    Shadows {
        packets: scene
            .shadows
            .iter()
            .map(|s| ShadowPacket::new(s.layer, s.width, s.height, 0x20).with_copies(s.passes, s.spread))
            .collect(),
        view: Some(ShadowView {
            world_screen: view.world_screen(),
            near: 8.0,
            far: 1_048_576.0,
            clip: [lo[0], lo[1], hi[0], hi[1]],
            offset: [XYOFFSET_X as f32 / 16.0, XYOFFSET_Y as f32 / 16.0],
        }),
        ..Shadows::default()
    }
}

/// `ccStreamDrawLayerList::Draw` (0x00148400) before each node's
/// `ccObj::Draw`: with a packet, the draw environment takes it, its length
/// and light, and the alpha `(fptosi(256 a) + 1) >> 1`; `ccObj::Draw` then
/// casts the node's shadow model whatever its transparency.
fn cast_shadow(ctx: &mut Ctx, loaded: &Loaded, scene: &Scene, i: usize) {
    let n = &scene.nodes[i];
    let (Some(k), Some((g, mdl))) = (n.shadow, n.shadow_model) else { return };
    let Some((scale, Some(mesh))) = loaded.files[g].sf.shadows.get(&mdl) else { return };
    let sh = &scene.shadows[k];
    let env = &mut ctx.layers.shadows;
    env.active = Some(k);
    env.length = sh.length;
    env.light = glam::Vec4::from_array(sh.light);
    let a = piney_data::anim::ee::to_int(piney_data::anim::ee::mul(0x4380_0000, sh.alpha.to_bits()));
    env.alpha = (a.wrapping_add(1) >> 1) as u8;
    env.cast(mesh, *scale, to_mat4(&scene.world(i)));
}

/// The scene's world-to-screen matrix, its camera as [`draw`] sets it: for
/// another task drawing into the scene (`ccThDrainEnemy`).
pub fn to_screen(scene: &Scene) -> Mat4 {
    scene_view(scene).to_screen(Mat4::IDENTITY)
}

/// The scene's view: its frame's projection, its camera.
pub fn scene_view(scene: &Scene) -> View {
    View::new(frame_projection(&scene.frame), scene.view_camera())
}

/// `CheckRange` of one light at world position `at`: (direction the light
/// travels, colour times intensity), or None when it does not reach.
pub fn check_range(l: &Light, at: Vec3) -> Option<(Vec3, Vec3)> {
    let colour = Vec3::from(l.colour);
    match l.kind {
        file::LIGHT_OMNI => {
            if l.intensity <= MIN_TRANSPARENCY {
                return None;
            }
            let d = at - Vec3::from(l.pos);
            let d2 = d.x * d.x + d.y * d.y + d.z * d.z;
            let f = if l.far_end == 0.0 {
                l.intensity
            } else if d2 <= l.far_end * l.far_end {
                let dist = d2.sqrt();
                if dist <= l.far_start {
                    l.intensity
                } else {
                    let f = l.intensity * (l.far_end - dist) / (l.far_end - l.far_start);
                    if f <= MIN_TRANSPARENCY {
                        return None;
                    }
                    f
                }
            } else {
                return None;
            };
            Some((d.normalize_or_zero(), colour * f))
        }
        file::LIGHT_DISTANT => {
            if l.intensity <= MIN_TRANSPARENCY {
                return None;
            }
            let r = to_mat4(&piney_data::anim::rot_bits(l.rot));
            Some((r.transform_vector3(Vec3::new(0.0, 0.0, -1.0)), colour * l.intensity))
        }
        // Direct and spot lights: no stream on this disc records them.
        _ => None,
    }
}

/// `SetLightMatrix` at world position `at`: per slot, the unit vector toward
/// the light (`sceVu0NormalLightMatrix` negates and normalises each
/// `CheckRange` normal) and its colour times intensity, before VU1 doubles
/// it. Lights of equal priority fill slots 2, 1, 0 in group order.
pub fn light_slots(scene: &Scene, at: Vec3) -> [Option<(Vec3, Vec3)>; 3] {
    let mut out = [None; 3];
    let mut used = 0;
    for l in &scene.lights {
        if used == 3 {
            break;
        }
        let Some((normal, colour)) = check_range(l, at) else { continue };
        out[2 - used] = Some((-normal.normalize_or_zero(), colour));
        used += 1;
    }
    out
}

/// What VU1 lights a lit model at `world` with: the light directions in
/// model space (`ccSetMatrixPacket` multiplies the light matrix by
/// `lwMatrix`; VU1 renormalises), the colours doubled, the ambient.
pub fn lights(scene: &Scene, world: Mat4) -> Lights {
    let mut out = Lights { dirs: [[0.0; 3]; 3], colours: [[0.0; 3]; 3], ambient: scene.ambient };
    let w = Mat3::from_mat4(world).transpose();
    for (slot, l) in light_slots(scene, world.w_axis.truncate()).iter().enumerate() {
        if let Some((toward, colour)) = l {
            out.dirs[slot] = (w * *toward).normalize_or_zero().into();
            out.colours[slot] = (*colour * VU_LIGHT_SCALE).into();
        }
    }
    out
}

fn draw_node(ctx: &mut Ctx, view: &View, loaded: &Loaded, scene: &Scene, i: usize, layer: i16) {
    let n = &scene.nodes[i];
    let (Some((g, mdl)), Some(tp)) = (n.model, scene.visible(i)) else { return };
    let world = to_mat4(&scene.world(i));
    let Some(info) = loaded.files[g].sf.models.get(&mdl) else { return };
    let to_screen = view.to_screen(world);
    // Bone and skin mmats: the clump's nodes by slot (a slot without a
    // ccObj here - an effect - keeps the object's own matrix).
    let nodes: Vec<[[f32; 4]; 4]> = if info.mtype & 6 != 0 {
        scene.clumps[n.clump]
            .iter()
            .map(|s| s.map_or(to_screen, |j| view.to_screen(to_mat4(&scene.world(j)))).to_cols_array_2d())
            .collect()
    } else {
        Vec::new()
    };
    // `ccMorpher`: the node's modifier and the weights `F_Morpher` last set,
    // its targets resolved to models of the drawn model's file.
    let morph: Vec<(u32, f32)> = scene.morph.get(&n.modifier).map_or(Vec::new(), |targets| {
        targets
            .iter()
            .filter_map(|&(t, w)| {
                let (tg, to, _) = loaded.follow(scene.file, t)?;
                (tg == g).then_some((to, w))
            })
            .collect()
    });
    let fogged = !scene.fog_off.contains(&i);
    draw_model(ctx, view, loaded, scene, (g, mdl), tp, &scene.world(i), nodes, morph, fogged, layer);
}

/// A rigid model of file `g` drawn at `world` (f32 bits, stored columns)
/// as `ccObj::Draw(1.0)` draws it through the scene's lights and fog:
/// stream 15's rocks.
pub fn draw_rigid(
    ctx: &mut Ctx,
    loaded: &Loaded,
    scene: &Scene,
    model: (usize, u32),
    world: &[[u32; 4]; 4],
    layer: i16,
) {
    let view = scene_view(scene);
    draw_model(ctx, &view, loaded, scene, model, 1.0, world, Vec::new(), Vec::new(), true, layer);
}

/// `ccModel::Draw` of `mdl` of file `g` at `world` with transparency `tp`.
#[allow(clippy::too_many_arguments)]
fn draw_model(
    ctx: &mut Ctx,
    view: &View,
    loaded: &Loaded,
    scene: &Scene,
    (g, mdl): (usize, u32),
    tp: f32,
    lw: &[[u32; 4]; 4],
    nodes: Vec<[[f32; 4]; 4]>,
    morph: Vec<(u32, f32)>,
    fogged: bool,
    layer: i16,
) {
    let world = to_mat4(lw);
    let file = &loaded.files[g];
    let Some(info) = file.sf.models.get(&mdl) else { return };
    let (mut opaque, mut sorted) = (Vec::new(), Vec::new());
    for (k, mm) in info.mmats.iter().enumerate().rev() {
        let mut t = tp * mm.transparency;
        if t < MIN_TRANSPARENCY {
            continue;
        }
        let translucent = t < OPAQUE_TRANSPARENCY || mm.tex_flag & TEX_FLAG_SORTED != 0 || info.blend_type != 0;
        if !translucent {
            t = 1.0;
        }
        let uv_row = mm.material.and_then(|m| file.name(m)).and_then(|name| scene.uv.get(name)).map_or([0, 0], |uv| {
            let u = (i32::from(uv[0]) - i32::from(mm.crop_u)) as u16;
            let v = (i32::from(uv[1]) - i32::from(mm.crop_v)) as u16;
            [(u >> 4) as u8, (v >> 4) as u8]
        });
        let draw = MmatDraw {
            index: k as u32,
            alpha: t,
            alpha_ref: (f32::from(mm.aref) * t) as u8,
            wrap: if mm.tex_flag & TEX_FLAG_CLAMP != 0 { Wrap::Clamp } else { Wrap::Repeat },
            uv_row,
        };
        if translucent { sorted.push(draw) } else { opaque.push(draw) }
    }
    let to_screen = view.to_screen(world);
    let lights = (info.mtype & MTYPE_LIT != 0).then(|| lights(scene, world));
    // The draw environment's fog, which VU1 writes per vertex by its depth.
    let depth_fog = scene.fog.filter(|_| fogged).map(|f| piney_draw::DepthFog {
        a: f.a,
        b: f.b,
        min: f.min.min(f.max),
        max: f.max.max(f.min),
        colour: f.colour,
    });
    let mut state = model_state(info.blend_type);
    state.scissor = scene.frame.layer().scissor;
    if scene.z_write_off.contains(&(g, mdl))
        && let AlphaTest::On { reference, .. } = state.alpha_test
    {
        state.alpha_test = AlphaTest::On { method: Compare::Never, reference, fail: AlphaFail::FbOnly };
    }
    let make = |mmats: Vec<MmatDraw>| {
        Cmd::Model(Box::new(ModelDraw {
            file: file.stem.clone(),
            model: mdl,
            to_screen: to_screen.to_cols_array_2d(),
            nodes: nodes.clone(),
            mmats,
            state: state.clone(),
            tex: TexParams { func: TexFunc::Modulate, use_alpha: true, filter: Filter::Linear },
            clut_swaps: Vec::new(),
            tex_swaps: Vec::new(),
            fog: None,
            depth_fog,
            lights,
            morph: morph.clone(),
            reject_outside: false,
            div_z: scene.div_z,
            edits: None,
        }))
    };
    if !opaque.is_empty() {
        ctx.layers.prepend(layer, vec![make(opaque)]);
    }
    if !sorted.is_empty() {
        let key = match info.centre {
            None => exact_key(&view.camera().matrix, lw),
            Some(_) => sort_key(&to_screen, info.centre),
        };
        ctx.layers.sorted(layer, key, make(sorted));
    }
}

/// `ccModel::Draw`'s sort key for a model without a bounding box (a bone or
/// skin model: a rigid one keys on its vertex box,
/// [`piney_desktop::assets::vertex_box_centre`]), `M[2][3] / M[3][3]` of
/// `world_screen * lw` ([`piney_desktop::anm::sort_key`]), in VU0's
/// arithmetic. The view screen's w row is (0, 0, 1, 0), so the key needs only
/// `world_view`'s third row and `lw`, taken as the game has them
/// (`Scene::view_camera`): for planes square to the view the key is the
/// camera's small tilt, which a float camera gets as rounding noise instead.
fn exact_key(wv: &[[u32; 4]; 4], lw: &[[u32; 4]; 4]) -> f32 {
    use piney_data::anim::ee;
    let w = |c: usize| {
        let acc = ee::mul(wv[0][2], lw[c][0]);
        let acc = ee::add(acc, ee::mul(wv[1][2], lw[c][1]));
        let acc = ee::add(acc, ee::mul(wv[2][2], lw[c][2]));
        ee::add(acc, ee::mul(wv[3][2], lw[c][3]))
    };
    f32::from_bits(ee::div(w(2), w(3)))
}
