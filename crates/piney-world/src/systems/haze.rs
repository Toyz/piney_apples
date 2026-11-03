//! `WORLD::Draw`'s heat haze (field types 0-3): `ANM_sfzair1a` of
//! `sfzair1`, its `OBJ_sfzair00` drawn through the picture under it
//! (`waterUVModifi`), on refLayer ahead of the player along the camera's
//! heading, scaled 1.3 wide, at transparency 0.7. While the tasks sleep it
//! stands as it was, its sampling brought up to the camera.

use std::borrow::Cow;
use std::rc::Rc;

use glam::Mat4;
use piney_data::Result;
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw;
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::{Env, pos_rot_xyz_scale};
use crate::town::WaterZero;

/// The haze's file, animation and the model whose texture is the picture
/// under it.
const FILE: &str = "sfzair1";
const ANIM: &str = "ANM_sfzair1a";
const MODEL: &str = "OBJ_sfzair00";

/// How far ahead of the player it stands (negative: along the heading).
#[derive(Clone, Copy, Debug)]
pub struct HazeDef {
    pub ahead: F,
}

/// An animation's object `name`'s model (through its ExtObj target),
/// which `waterUVModifi` writes as `waterUVModifi2` does, with no reach.
pub(super) fn sampled(file: &SceneFile, name: &str) -> Option<WaterZero> {
    let o = file.ccs.find_object(name)?;
    let target = file.scene.ext.get(&o).copied().unwrap_or(o);
    let id = file.obj_model.get(&target).copied()?;
    let model = piney_data::model::models(&file.ccs).ok()?.into_iter().find(|m| m.object == id)?;
    Some(WaterZero { model, st: None, reach: 0x7f7f_ffff })
}

/// `waterUVModifi` of `water` for the posed object whose model is its own,
/// under `root`.
pub(super) fn sample(water: &mut WaterZero, file: &SceneFile, play: &Play, root: Mat4, cx: &StepCx) {
    let worlds = play.worlds(file, root, &[]);
    let lw = worlds.iter().find(|(o, _)| {
        let target = file.scene.ext.get(o).copied().unwrap_or(**o);
        file.obj_model.get(&target) == Some(&water.model.object)
    });
    if let Some((_, lw)) = lw {
        let lw: [V4; 4] = lw.to_cols_array_2d().map(|c| c.map(f32::to_bits));
        water.modify(cx.core.hits.volume, &lw, lw[3], lw[3], &cx.view.world_screen);
    }
}

/// The haze: its file, animation and model as read (none without the
/// file), where the last frame put it.
#[derive(Clone)]
pub struct Haze {
    def: &'static HazeDef,
    made: Option<(Rc<SceneFile>, Play, WaterZero)>,
    pub matrix: [V4; 4],
}

impl Haze {
    pub fn new(def: &'static HazeDef, cx: &mut BuildCx) -> Result<Haze> {
        let made = SceneFile::read(cx.archive, FILE).ok().and_then(|f| {
            let f = Rc::new(f);
            let play = Play::new(&f, ANIM)?;
            let water = sampled(&f, MODEL)?;
            Some((f, play, water))
        });
        Ok(Haze { def, made, matrix: [[0; 4]; 4] })
    }

    /// Where it stands: `ahead` along the camera's heading from the
    /// player on the ground, brought to the player's side of the field.
    fn stand(&self, env: &Env) -> [V4; 4] {
        let mut pos = env.player;
        pos[2] = 0;
        let mut rot = env.cam_rot();
        rot[0] = 0;
        rot[1] = 0;
        let m = piney_data::anim::rot_bits([rot[0], rot[1], rot[2]]);
        let mut off = ee::apply(&m, [0, self.def.ahead, 0, ONE]);
        off = ee::vadd(off, pos);
        off[3] = ONE;
        let off = env.p2w(env.w2p(off));
        pos_rot_xyz_scale(off, rot, [0x3fa6_6666, ONE, ONE])
    }
}

impl MapSystem for Haze {
    /// `Draw`: placed, its animation on, the picture sampled. `Again`: the
    /// sampling only.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        match hook {
            Hook::Draw => {
                self.matrix = self.stand(&Env::of(cx));
                if let Some((file, play, _)) = &mut self.made {
                    play.forward(file);
                }
                cx.put(Kind::Haze, Piece::Anim(0));
            }
            Hook::Again => {}
            _ => return,
        }
        let root = draw::mat(&self.matrix);
        if let Some((file, play, water)) = &mut self.made {
            sample(water, file, play, root, cx);
        }
    }

    /// On refLayer, at 0.7.
    fn draw<'a>(&'a self, _: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let (file, play, water) = self.made.as_ref()?;
        Some(Request::Posed {
            file,
            play,
            root: draw::mat(&self.matrix),
            layer: crate::field::area::ambient_layer(6),
            rows: Cow::Owned(play.uv_rows(file)),
            alpha: ee::f(0x3f33_3333),
            edit: water.st.is_some().then(|| (water.model.object, water.edits())),
            shadow: None,
        })
    }
}
