//! Drawing the field: `ccModel::Draw` (0x0013eab0) for a model at a world matrix
//! through the field view, into `WORLD_MAN::GO`'s layers, with the per-mmat rules
//! `piney_desktop::anm::draw_model` applies, plus the screen matrix, lights,
//! nodes, morph weights and palette swaps. Near the camera the unlit pieces go
//! through `mc_DrawTriSFast`, which cuts a triangle crossing the near plane, and
//! the lit or skinned ones drop it (docs/engine/render.md).

use std::collections::HashMap;

use glam::{Mat4, Vec3, Vec4};
use piney_desktop::anm::{MIN_TRANSPARENCY, OPAQUE_TRANSPARENCY, model_state, sort_key};
use piney_desktop::assets::{SceneFile, TEX_FLAG_CLAMP, TEX_FLAG_SORTED};
use piney_desktop::layers::Layers;
use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
use piney_draw::{Cmd, Filter, Lights, MmatDraw, ModelDraw, TexFunc, TexParams, Wrap};

use crate::ee::V4;

/// `WORLD_MAN::GO`'s layers (priorities): the sky and background below,
/// the town on `objLayer`, characters over it.
pub const BG_LAYER: i16 = -100;
pub const FLOOR_LAYER: i16 = -20;
pub const OBJ2_LAYER: i16 = -10;
pub const OBJ_LAYER: i16 = 0;
pub const CHAR_LAYER: i16 = 10;
pub const EFF_LAYER: i16 = 20;
/// `ccGame.layer`, where `ccGame.fade` (the field's entry fades) draws.
pub const FADE_LAYER: i16 = 254;

/// Stored columns of float bits to a matrix.
pub fn mat(m: &[V4; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(&m.map(|c| c.map(f32::from_bits)))
}

/// `world_screen` with the XYOFFSET taken off: world to frame-buffer
/// pixels after the divide, as a [`ModelDraw::to_screen`] wants it.
pub fn screen(world_screen: &[V4; 4]) -> Mat4 {
    let (ox, oy) = (XYOFFSET_X as f32 / 16.0, XYOFFSET_Y as f32 / 16.0);
    let off = Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::Z, Vec4::new(-ox, -oy, 0.0, 1.0));
    off * mat(world_screen)
}

/// How a model is fogged (PRIM.FGE): not at all, with one coefficient
/// (`ccChar::Draw`'s blend), or with VU1's fog by each vertex's depth
/// (`ccDrawEnv::SetFog`'s).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Fogging {
    #[default]
    None,
    Const(piney_draw::Fog),
    Depth(piney_draw::DepthFog),
}

impl From<Option<piney_draw::Fog>> for Fogging {
    fn from(f: Option<piney_draw::Fog>) -> Fogging {
        f.map_or(Fogging::None, Fogging::Const)
    }
}

impl From<piney_draw::DepthFog> for Fogging {
    fn from(d: piney_draw::DepthFog) -> Fogging {
        Fogging::Depth(d)
    }
}

/// One model to draw.
pub struct Draw<'a> {
    pub file: &'a SceneFile,
    /// The MDL_ object.
    pub model: u32,
    /// Model space to world.
    pub world: Mat4,
    /// The object's transparency (times the anm's `localtp`).
    pub alpha: f32,
    /// Animated texture offsets by material (STROW values).
    pub rows: &'a HashMap<u32, [u8; 2]>,
    pub lights: Option<Lights>,
    /// Bone and skin models: each clump node's world matrix, by slot.
    pub nodes: &'a [Mat4],
    pub morph: Vec<(u32, f32)>,
    pub clut_swaps: Vec<(u32, u32)>,
}

/// `ccObj::Draw` / `ccModel::Draw` into layer `layer`: the opaque mmats as
/// one command at the front of the layer, the translucent ones in its
/// sorted group; each list in reverse mmat order.
pub fn model(layers: &mut Layers, layer: i16, to_screen: Mat4, d: Draw) {
    model_edited(layers, layer, to_screen, d, None, Fogging::None);
}

/// [`model`] with the vertex data the EE rewrote in the model (a field's
/// ground tiles, its objects' lit colours) and a fog coefficient.
pub fn model_edited(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    d: Draw,
    edits: Option<&std::sync::Arc<piney_draw::VertexEdits>>,
    fog: Fogging,
) {
    model_swapped(layers, layer, to_screen, d, edits, fog, &[]);
}

/// [`model_edited`] with materials drawn with other textures
/// (`ccClump::ChangeTex`: (MAT_, TEX_)).
pub fn model_swapped(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    d: Draw,
    edits: Option<&std::sync::Arc<piney_draw::VertexEdits>>,
    fog: Fogging,
    tex_swaps: &[(u32, u32)],
) {
    model_blended(layers, layer, to_screen, d, edits, fog, tex_swaps, None);
}

/// [`model_swapped`] with the model's blend replaced (`ccAnm::SetBlendType`:
/// `alphaBlendTbl[t]`, as the model's own `blendType` t would draw).
#[allow(clippy::too_many_arguments)]
pub fn model_blended(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    d: Draw,
    edits: Option<&std::sync::Arc<piney_draw::VertexEdits>>,
    fog: Fogging,
    tex_swaps: &[(u32, u32)],
    blend: Option<u8>,
) {
    if d.alpha < MIN_TRANSPARENCY {
        return;
    }
    let Some(info) = d.file.models.get(&d.model) else { return };
    let blend_type = blend.unwrap_or(info.blend_type);
    let (mut opaque, mut sorted) = (Vec::new(), Vec::new());
    for (i, mm) in info.mmats.iter().enumerate().rev() {
        let mut t = d.alpha * mm.transparency;
        if t < MIN_TRANSPARENCY {
            continue;
        }
        let translucent = t < OPAQUE_TRANSPARENCY || mm.tex_flag & TEX_FLAG_SORTED != 0 || blend_type != 0;
        if !translucent {
            t = 1.0;
        }
        let draw = MmatDraw {
            index: i as u32,
            alpha: t,
            alpha_ref: (f32::from(mm.aref) * t) as u8,
            wrap: if mm.tex_flag & TEX_FLAG_CLAMP != 0 { Wrap::Clamp } else { Wrap::Repeat },
            uv_row: mm.material.and_then(|m| d.rows.get(&m).copied()).unwrap_or([0, 0]),
        };
        if translucent { sorted.push(draw) } else { opaque.push(draw) }
    }
    let m = to_screen * d.world;
    // `ccModel::Draw`'s programs for a model partly in view: the unlit rigid
    // one (`mc_DrawTriSFast`) clips triangles near the camera; the lit
    // (`mc_DrawTriLC`) and the bone and skin ones drop them.
    let reject_outside = d.lights.is_some() || !d.nodes.is_empty();
    let nodes: Vec<[[f32; 4]; 4]> = d.nodes.iter().map(|n| (to_screen * *n).to_cols_array_2d()).collect();
    let make = |mmats: Vec<MmatDraw>| {
        Cmd::Model(Box::new(ModelDraw {
            file: d.file.stem.clone(),
            model: d.model,
            to_screen: m.to_cols_array_2d(),
            nodes: nodes.clone(),
            mmats,
            state: {
                let mut st = model_state(blend_type);
                // SetBlendType's own entry of alphaBlendTbl (4 and on are
                // not a model's).
                if let Some(b) = blend {
                    st.blend = piney_draw::Blend::TABLE.get(usize::from(b)).copied().or(st.blend);
                }
                st
            },
            tex: TexParams { func: TexFunc::Modulate, use_alpha: true, filter: Filter::Linear },
            clut_swaps: d.clut_swaps.clone(),
            tex_swaps: tex_swaps.to_vec(),
            fog: match fog {
                Fogging::Const(f) => Some(f),
                _ => None,
            },
            depth_fog: match fog {
                Fogging::Depth(d) => Some(d),
                _ => None,
            },
            lights: d.lights,
            morph: d.morph.clone(),
            reject_outside,
            div_z: piney_draw::DIV_Z,
            edits: edits.cloned(),
        }))
    };
    if !opaque.is_empty() {
        layers.prepend(layer, vec![make(opaque)]);
    }
    if !sorted.is_empty() {
        layers.sorted(layer, sort_key(&m, info.centre), make(sorted));
    }
}

/// A point's position on the frame buffer, for tests and shots.
pub fn project(to_screen: Mat4, p: Vec3) -> Option<(f32, f32)> {
    let q = to_screen * p.extend(1.0);
    (q.w > 0.0).then(|| (q.x / q.w, q.y / q.w))
}

/// `ccChar::Draw`'s shadow around its `ccAnm::Draw` (`f`): the draw
/// environment's shadow alpha `(fptosi(256 t) + 1) >> 1` of the
/// transparency `t` (`setTransparency` when the camera fade is off), the
/// length three times the height, the alpha put back to 128 after (the
/// length is left).
pub fn char_shadow<R>(
    layers: &mut Layers,
    t: crate::ee::F,
    height: crate::ee::F,
    f: impl FnOnce(&mut Layers) -> R,
) -> R {
    use crate::ee;
    let alpha = ((ee::to_int(ee::mul(0x4380_0000, t)).wrapping_add(1)) >> 1) as u8;
    let length = ee::add(height, ee::mul(0x4000_0000, height));
    shadow_env(layers, alpha, length, f)
}

/// The same with the alpha and length decided ([`crate::foe::CharDraw`]).
pub fn shadow_env<R>(layers: &mut Layers, alpha: u8, length: crate::ee::F, f: impl FnOnce(&mut Layers) -> R) -> R {
    layers.shadows.alpha = alpha;
    layers.shadows.length = f32::from_bits(length);
    let r = f(layers);
    layers.shadows.alpha = 128;
    r
}

/// `ccObj::Draw`'s shadow half for each of `nodes` that has a shadow model
/// in `file` (`ccObj` +0x9c, its switch on: `ccObj::Init` sets it with
/// the model), at the node's world matrix: [`piney_desktop::shadow::
/// Shadows::cast`] into the draw environment's packet. Drawn whatever the
/// object's transparency.
pub fn cast_shadows(layers: &mut Layers, file: &SceneFile, nodes: &[u32], worlds: &HashMap<u32, Mat4>, root: Mat4) {
    if layers.shadows.active.is_none() {
        return;
    }
    for &obj in nodes {
        let Some(model) = file.scene.shadow_of.get(&obj) else { continue };
        let Some((scale, Some(mesh))) = file.shadows.get(model) else { continue };
        let world = worlds.get(&obj).copied().unwrap_or(root);
        layers.shadows.cast(mesh, *scale, world);
    }
}

/// `WORLD_MAN::GO`'s shadow packets ([`piney_desktop::shadow::Shadows::
/// field`]) for a frame: the light the area's distant light (`WORLD_MAN::
/// SetLightDirection`: `ccDistantLight::GetDirc`, the way it travels), the
/// view sysLayer's (`world_screen` as the camera left it, near 8, far
/// 2^20, the whole frame).
pub fn go_shadows(lights: &crate::town::TownLights, world_screen: &[V4; 4]) -> piney_desktop::shadow::Shadows {
    let dir = lights.lights.iter().find(|l| l.kind == 1).map_or(Vec3::new(0.0, 0.0, -1.0), |l| l.dir);
    let (ox, oy) = (XYOFFSET_X as f32 / 16.0, XYOFFSET_Y as f32 / 16.0);
    let (w, h) = (f32::from(piney_draw::SCREEN_WIDTH), f32::from(piney_draw::SCREEN_HEIGHT));
    let view = piney_desktop::shadow::ShadowView {
        world_screen: mat(world_screen),
        near: 8.0,
        far: 1_048_576.0,
        clip: [ox, oy, ox + w, oy + h],
        offset: [ox, oy],
    };
    piney_desktop::shadow::Shadows::field(dir.extend(0.0), view)
}
