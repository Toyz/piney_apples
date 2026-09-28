//! How the title draws a `ccAnm`: `ccAnm::Draw` (0x001524d0) through
//! `ccObj::Draw` (0x0013f220) and `ccModel::Draw` (0x0013eab0), as
//! `piney_desktop::anm` ports them, but on `sysLayer` (0x00379b10, priority 0)
//! under the dialogs (130) and the fader (240), and with the title's light:
//! one white `ccOmniLight` at (-8500, 0, 6500) from `ccOpening_Control::Init`
//! (0x00405f50) and the ambient `ccDefAmbientColor` (0.35, 0.35, 0.35), which
//! light the icons (`MDL_xdt_ico_*`). See docs/engine/title.md ("The 3D
//! draw").

use glam::{Mat3, Mat4, Vec3};
use piney_desktop::anm::{
    Anm, Ctx, FOG_BLEND_F, Instance, MIN_TRANSPARENCY, OPAQUE_TRANSPARENCY, model_state, sort_key,
};
use piney_desktop::assets::{SceneFile, TEX_FLAG_CLAMP, TEX_FLAG_SORTED};
use piney_desktop::view::View;
use piney_draw::{Cmd, Filter, Fog, Lights, MmatDraw, ModelDraw, TexFunc, TexParams, Wrap};

/// `sysLayer`'s priority (`InitCCSys` stores 0 at `sysLayer+4`).
pub const SYS_LAYER: i16 = 0;
/// The Data_Control dialogs' layer (`ccLayer::Init(130, 0)`).
pub const DIALOG_LAYER: i16 = 130;

/// Model type bit: lit (`ccModel::Draw` picks `mc_DrawTriL`).
pub const MTYPE_LIT: u16 = 1;

/// The title's `ccOmniLight`: world position (`Init` 0x00406378..0x00406388,
/// -8500 = 0xc604d000, 6500 = 0x45cb2000).
pub const OMNI_POS: [f32; 3] = [-8500.0, 0.0, 6500.0];
/// Its colour times intensity, before VU1 doubles it.
pub const OMNI_COLOUR: [f32; 3] = [1.0, 1.0, 1.0];
/// The slot `SetLightMatrix` sorts a lone priority-1 light into.
pub const OMNI_SLOT: usize = 2;
/// `ccDefAmbientColor` (0x002f7340).
pub const AMBIENT: [f32; 3] = [0.35, 0.35, 0.35];
/// `mc_SetMatrix`'s `loi 2.0` on the light colours.
pub const VU_LIGHT_SCALE: f32 = 2.0;

/// What VU1 lights a lit model with, for the object at `world`.
pub fn omni_lights(world: Mat4) -> Lights {
    let obj = world.w_axis.truncate();
    // CheckRange: normal = normalize(checkPos - lightPos); the matrix then
    // takes its negation: toward the light.
    let toward = (Vec3::from(OMNI_POS) - obj).normalize_or_zero();
    // Row i of lightNormal x lwMatrix is W^T l_i; VU1 renormalises it.
    let dir = (Mat3::from_mat4(world).transpose() * toward).normalize_or_zero();
    let mut l = Lights { dirs: [[0.0; 3]; 3], colours: [[0.0; 3]; 3], ambient: AMBIENT };
    l.dirs[OMNI_SLOT] = dir.into();
    l.colours[OMNI_SLOT] = (Vec3::from(OMNI_COLOUR) * VU_LIGHT_SCALE).into();
    l
}

/// `ccAnm::Draw` of `anm` (an animation of `file`) on the title's layer.
pub fn draw_anm(ctx: &mut Ctx, file: &SceneFile, anm: &Anm) {
    draw_under(ctx, file, anm, Mat4::IDENTITY);
}

/// `ccAnm::Draw` of an animation whose own matrix was set with
/// `SetMatrix_PosRotXYZScale(0, 0, scale)` (`SetBootMemCard`'s `m_A_BOOT`):
/// every object hangs under the scale.
pub fn draw_anm_scaled(ctx: &mut Ctx, file: &SceneFile, anm: &Anm, scale: [f32; 3]) {
    draw_under(ctx, file, anm, Mat4::from_scale(Vec3::from(scale)));
}

fn draw_under(ctx: &mut Ctx, file: &SceneFile, anm: &Anm, root: Mat4) {
    #[cfg(feature = "trace")]
    if piney_desktop::anm::trace::record("draw", anm.label, String::from("None")).is_some() {
        return;
    }
    if !anm.is_set() {
        return;
    }
    let mut view = ctx.view.clone();
    if let Some(cam) = anm.camera {
        view.set_camera(&cam);
    }
    for mut inst in anm.instances() {
        inst.world = root * inst.world;
        draw_instance(ctx, &view, file, &inst, anm.localtp);
    }
}

/// `ccObj::Draw(tp)` / `ccModel::Draw` for one object: the opaque mmats
/// prepended to the layer, the translucent ones a node of its sorted group,
/// each in reverse mmat order (`piney_desktop::anm::draw_model`, with the
/// light added and the title's layer).
fn draw_instance(ctx: &mut Ctx, view: &View, file: &SceneFile, inst: &Instance, localtp: f32) {
    let tp = localtp * inst.alpha;
    if tp < MIN_TRANSPARENCY {
        return;
    }
    let Some(info) = file.models.get(&inst.model) else { return };
    let (mut opaque, mut sorted) = (Vec::new(), Vec::new());
    for (i, mm) in info.mmats.iter().enumerate().rev() {
        let mut t = tp * mm.transparency;
        if t < MIN_TRANSPARENCY {
            continue;
        }
        let translucent = t < OPAQUE_TRANSPARENCY || mm.tex_flag & TEX_FLAG_SORTED != 0 || info.blend_type != 0;
        if !translucent {
            t = 1.0;
        }
        let draw = MmatDraw {
            index: i as u32,
            alpha: t,
            alpha_ref: (f32::from(mm.aref) * t) as u8,
            wrap: if mm.tex_flag & TEX_FLAG_CLAMP != 0 { Wrap::Clamp } else { Wrap::Repeat },
            // No title animation has material (texture offset) records.
            uv_row: [0, 0],
        };
        if translucent { sorted.push(draw) } else { opaque.push(draw) }
    }
    let to_screen = view.to_screen(inst.world);
    let fog = ctx.fog_blend.map(|f| Fog { f: FOG_BLEND_F, colour: [f.colour[0], f.colour[1], f.colour[2]] });
    let lights = (info.mtype & MTYPE_LIT != 0).then(|| omni_lights(inst.world));
    let make = |mmats: Vec<MmatDraw>| {
        Cmd::Model(Box::new(ModelDraw {
            file: file.stem.clone(),
            model: inst.model,
            to_screen: to_screen.to_cols_array_2d(),
            nodes: Vec::new(),
            mmats,
            state: model_state(info.blend_type),
            tex: TexParams { func: TexFunc::Modulate, use_alpha: true, filter: Filter::Linear },
            clut_swaps: Vec::new(),
            tex_swaps: Vec::new(),
            fog,
            depth_fog: None,
            lights,
            morph: Vec::new(),
            reject_outside: true,
            div_z: piney_draw::DIV_Z,
            edits: None,
        }))
    };
    if !opaque.is_empty() {
        ctx.layers.prepend(SYS_LAYER, vec![make(opaque)]);
    }
    if !sorted.is_empty() {
        ctx.layers.sorted(SYS_LAYER, sort_key(&to_screen, info.centre), make(sorted));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_points_at_the_omni_in_model_space() {
        // An object at the origin, turned a quarter about y: the light's
        // model-space direction, turned back, points at (-8500, 0, 6500).
        let world = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let l = omni_lights(world);
        let d = Vec3::from(l.dirs[OMNI_SLOT]);
        let toward = Vec3::from(OMNI_POS).normalize();
        assert!((Mat3::from_mat4(world) * d - toward).length() < 1e-5);
        assert_eq!(l.colours[OMNI_SLOT], [2.0; 3]);
        assert_eq!(l.colours[0], [0.0; 3]);
    }
}
