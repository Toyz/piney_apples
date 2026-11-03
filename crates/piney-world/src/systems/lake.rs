//! A field's lake (`WORLD::SetLake`, `WORLD::Draw` gcmn
//! 0x005a9aac-0x005a9d70): three `ccAnm`s of `ANM_sfwat1_1a` (`field_eff`,
//! `WORLD` +0x53b0, +0x53b4, +0x53b8) in step at the lake object, moved
//! each frame to the copy nearest the player; the first and third on
//! objLayer, the second on refLayer, its `OBJ_water00` sampling the
//! picture under it. Before them the scroll is written (`MAT_sfzwat1_1`'s
//! U, the V of `OBJ_water00`'s materials), then both step by 0.005, back to
//! 0 together once V is past 1. The scroll, `u$4930` and `v$4933`, is the
//! executable's (gp 0x00378d4c, 0x00378d54) and goes on from field to field.

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use piney_data::Result;
use piney_desktop::assets::SceneFile;

use super::haze::{sample, sampled};
use super::{BuildCx, Carried, CarriedMut, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw;
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::Env;
use crate::town::{WaterZero, scroll_statics};

/// The lake's model whose texture is the picture under it, and the
/// material whose U the scroll writes.
const MODEL: &str = "OBJ_water00";
const MATERIAL: &str = "MAT_sfzwat1_1";
/// The scroll's step, 0.005.
const STEP: F = 0x3ba3_d70a;

/// The lake's pieces of `field_eff`: the animation (the three are set up
/// alike and step together), the second's model as `waterUVModifi` leaves
/// it, the scrolled materials, and where they stand (the lake object's
/// place at height 0).
#[derive(Clone)]
struct Water {
    eff: Rc<SceneFile>,
    play: Play,
    mirror: WaterZero,
    u_mat: Option<u32>,
    v_mats: Vec<u32>,
    at: V4,
}

/// The lake as the last `Draw` left it (no water without a lake object),
/// and the scroll.
#[derive(Clone)]
pub struct Lake {
    water: Option<Water>,
    pub uv: [F; 2],
    /// What the last frame wrote into the materials.
    pub shown: [u16; 2],
    pub matrix: [V4; 4],
}

impl Lake {
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Lake> {
        let field = cx.field.and_then(|f| f.field);
        let water = field.and_then(|a| {
            let w = a.scene.water?;
            let eff = a.eff.clone()?;
            let play = Play::new(&eff, w.anime)?;
            let mirror = sampled(&eff, MODEL)?;
            let mut v_mats: Vec<u32> = mirror.model.mmats.iter().filter_map(|m| m.material).collect();
            v_mats.sort_unstable();
            v_mats.dedup();
            let u_mat = eff.ccs.find_object(MATERIAL);
            let at = [w.pos[0].to_bits(), w.pos[1].to_bits(), w.pos[2].to_bits(), ONE];
            Some(Water { eff, play, mirror, u_mat, v_mats, at })
        });
        Ok(Lake { water, uv: [0; 2], shown: [0; 2], matrix: [[0; 4]; 4] })
    }

    /// Where the lake stands, None without one.
    pub fn at(&self) -> Option<V4> {
        self.water.as_ref().map(|w| w.at)
    }

    /// The materials' offsets the scroll wrote, less each one's own, over
    /// the clip's own (`_AnimateForward` comes after the scroll's writes).
    fn rows(&self, w: &Water) -> HashMap<u32, [u8; 2]> {
        let mut rows: HashMap<u32, [u8; 2]> = HashMap::new();
        let [u, v] = self.shown;
        for (&mat, m) in &w.eff.scene.materials {
            let mut row = [0u8; 2];
            if Some(mat) == w.u_mat {
                row[0] = (u.wrapping_sub(m.crop_u) >> 4) as u8;
            }
            if w.v_mats.contains(&mat) {
                row[1] = (v.wrapping_sub(m.crop_v) >> 4) as u8;
            }
            if row != [0, 0] {
                rows.insert(mat, row);
            }
        }
        rows.extend(w.play.uv_rows(&w.eff));
        rows
    }
}

impl MapSystem for Lake {
    /// `Draw`: the scroll written, the three moved, stepped and drawn (the
    /// first and third, then the second), the scroll on. `Again`: the
    /// second's sampling only.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let Some(w) = &mut self.water else { return };
        match hook {
            Hook::Draw => {
                self.shown = [ee::vftoi12(self.uv[0]), ee::vftoi12(self.uv[1])];
                let env = Env::of(cx);
                w.at = env.p2w(env.w2p(w.at));
                self.matrix = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], w.at];
                self.matrix[3][3] = ONE;
                w.play.forward(&w.eff);
                for k in [0, 2, 1] {
                    cx.put(Kind::Lake, Piece::Anim(k));
                }
                self.uv = self.uv.map(|x| ee::add(x, STEP));
                if !ee::le(self.uv[1], ONE) {
                    self.uv = [0, 0];
                }
            }
            Hook::Again => {}
            _ => return,
        }
        sample(&mut w.mirror, &w.eff, &w.play, draw::mat(&self.matrix), cx);
    }

    /// The first and third on objLayer, the second on refLayer through the
    /// picture.
    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Anim(k) = piece else { return None };
        let w = self.water.as_ref()?;
        let mirror = (k == 1 && w.mirror.st.is_some()).then(|| (w.mirror.model.object, w.mirror.edits()));
        Some(Request::Posed {
            file: &w.eff,
            play: &w.play,
            root: draw::mat(&self.matrix),
            layer: crate::field::area::ambient_layer(if k == 1 { 6 } else { 1 }),
            rows: Cow::Owned(self.rows(w)),
            alpha: 1.0,
            edit: mirror,
            shadow: None,
        })
    }

    fn take(&mut self, c: &Carried) {
        scroll_statics::take(&mut self.uv, c.statics, scroll_statics::WORLD_LAKE);
    }

    fn put(&self, c: &mut CarriedMut) {
        scroll_statics::put(&self.uv, c.statics, scroll_statics::WORLD_LAKE);
    }
}
