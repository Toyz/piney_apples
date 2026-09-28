//! What the effects send to the GS, as recorded by a step, and how it is
//! drawn into the field's layers: a `ccClump` (`ccClump::Draw`, each of its
//! nodes' models at the clump's matrix), a `ccAnm` (`ccAnm::Draw`: each
//! animated object at its pose and transparency) or a `ccEff` sprite
//! ([`crate::sprite`]); what the first two draw is [`crate::nodes`].

use piney_desktop::layers::Layers;
use piney_world::draw::{self as wdraw, Draw};
use piney_world::pose::Play;

use crate::ee::{self, F, V4};
use crate::eff::Eff;
use crate::files::{Assets, ObjRef};
use crate::nodes;
use crate::sprite::DrawEnv;
use crate::vu::M4;

/// The camera as the effects see it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Camera {
    /// `cameraGetPos(camID)`: the eye the `distSW` fade measures from.
    pub eye: V4,
    /// `activeCamPtr` +0x00 and +0x10: the eye and the point it looks at,
    /// for `ccCheckCameraDeg`.
    pub cam_pos: V4,
    pub cam_view: V4,
    /// The field view's matrices as the camera set them (`cameraSet`):
    /// world to view, and world to screen (XYOFFSET included).
    pub world_view: [V4; 4],
    pub world_screen: [V4; 4],
    /// The view's w range, the draw environment's fog and filtering and
    /// the Z buffer, which `ccEff::Draw` also reads.
    pub env: DrawEnv,
}

/// One draw call of an effect or a particle, in the order sent.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawRec {
    /// `ccClump::SetTransparency(alpha)`, `ccClump::Draw` at `matrix`; a
    /// duplicate's CLUT swap (from, to).
    Clump { obj: ObjRef, matrix: M4, alpha: F, layer: i16, clut: Option<(u32, u32)> },
    /// `ccAnm::Draw`: its transparency `alpha`, posed where `play` is, at
    /// `matrix`.
    Anm { obj: ObjRef, play: Play, matrix: M4, alpha: F, layer: i16 },
    /// `ccEff::Draw(pat)` of `eff` as it stood.
    Eff { eff: Box<Eff>, pat: u16, layer: i16 },
}

impl DrawRec {
    pub fn layer(&self) -> i16 {
        match self {
            DrawRec::Clump { layer, .. } | DrawRec::Anm { layer, .. } | DrawRec::Eff { layer, .. } => *layer,
        }
    }

    /// Into `layers` through `camera`'s view.
    pub fn render(&self, assets: &Assets, layers: &mut Layers, camera: &Camera) {
        let to_screen = wdraw::screen(&camera.world_screen);
        match self {
            DrawRec::Clump { obj, matrix, alpha, layer, clut } => {
                let file = &assets.files[obj.file];
                let rows = Default::default();
                for d in nodes::clump(assets, *obj, matrix, *alpha) {
                    let d = Draw {
                        file,
                        model: d.model,
                        world: wdraw::mat(&d.world),
                        alpha: ee::f(d.alpha),
                        rows: &rows,
                        lights: None,
                        nodes: &[],
                        morph: Vec::new(),
                        clut_swaps: clut.iter().copied().collect(),
                    };
                    wdraw::model(layers, *layer, to_screen, d);
                }
            }
            DrawRec::Anm { obj, play, matrix, alpha, layer } => {
                // As `anim_draw_on`, with each object's own transparency.
                let file = &assets.files[obj.file];
                let rows = play.uv_rows(file);
                let morph = play.morph(file);
                for d in nodes::anm(assets, *obj, play, matrix, *alpha) {
                    let m: Vec<(u32, f32)> = morph
                        .iter()
                        .filter(|(mph, _)| assets.morphers[obj.file].get(mph) == Some(&d.model))
                        .flat_map(|(_, t)| t.iter().copied())
                        .collect();
                    let d = Draw {
                        file,
                        model: d.model,
                        world: wdraw::mat(&d.world),
                        alpha: ee::f(d.alpha),
                        rows: &rows,
                        lights: None,
                        nodes: &[],
                        morph: m,
                        clut_swaps: Vec::new(),
                    };
                    wdraw::model(layers, *layer, to_screen, d);
                }
            }
            DrawRec::Eff { eff, pat, layer } => eff.render(assets, layers, *layer, *pat, camera),
        }
    }
}
