//! What an effect's `ccClump` and `ccAnm` hand `ccObj::Draw` (main
//! 0x0013f220) and so `ccModel::Draw`: each object's model, world matrix
//! (`lwMatrix`) and transparency, in the order they are drawn.
//!
//! ```text
//! ccClump::Draw (0x0013f470): each node of objTbl in order;
//!   an Obj (ccstag 0x100): ccObj::Draw(node, 1.0)
//!   an EffObj (0xe00): its ccEff drawn at its lwMatrix's translation
//!     (no clump of effectTbl has one)
//! ccAnm::Draw (0x001524d0): each ccAnmIndex entry in order whose flag has
//!   bit 2: an Obj: ccObj::Draw(obj, ccAnm.localtp); an EffObj: DrawNoAnm
//! ccObj::Draw(obj, tp):
//!   lwMatrix = the matrix with no parent, else ccCoord::_SetLWMatrix
//!     (0x00138380): from the topmost coordinate whose matCalcSW is set,
//!     lw = parent's lw * matrix (VU0 multiply-adds), the root's lw its
//!     matrix
//!   tp *= succession bit 0 ? _GetTransparency (the localtp chain) :
//!     worldtp; nothing drawn at or below 1/128 (c.le.s 0x3c000000)
//! ```
//!
//! A clump's nodes: `ccClump::Init` (0x0013c5e0) sets every node's matrix
//! and lwMatrix to the unit matrix and its parent to the node its Obj
//! names (else the clump itself), and nothing in the effects changes them:
//! each node is drawn at the clump's matrix times the unit matrix once per
//! level (the same numbers; only the sign of a zero can change).
//! `ccClump::SetTransparency(t)` (0x0013d7d0) sets each Obj node's localtp
//! and (without succession bit 0) worldtp; `Obj2.succession` is 0 for every
//! object in the effect files, so every node draws at `t`.
//!
//! An animation's objects: each animated object's matrix is its pose, its
//! parent the animated object its Obj names, else the `ccAnm` itself (whose
//! matrix the effect sets); its transparency the pose's (the controllers
//! write localtp and worldtp alike), times `ccAnm.localtp`. Effect
//! animations fade their objects this way.

use std::collections::HashMap;

use piney_world::pose::Play;

use crate::ee::{self, F, ONE};
use crate::files::{Assets, ObjRef};
use crate::vu::{self, M4};

/// 1/128: `ccObj::Draw` draws nothing at or below it (0x0013f2a4).
pub const MIN_TRANSPARENCY: F = 0x3c00_0000;

/// One `ccModel::Draw` of an effect's clump or animation.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjDraw {
    /// The object drawn (an ExtObj copy is its own) and its model (MDL_).
    pub obj: u32,
    pub model: u32,
    /// `lwMatrix`: model space to world. A clump's are the game's bits; an
    /// animation's are the game's products of its poses, which match the
    /// game's but for keyed rotations (`piney_data::anim`: within 1e-5).
    pub world: M4,
    /// The transparency `ccModel::Draw` gets.
    pub alpha: F,
}

/// `ccClump::Draw` of an effect's clump (CMP_ object `obj`) whose matrix
/// is `matrix`, after `SetTransparency(alpha)`.
pub fn clump(assets: &Assets, obj: ObjRef, matrix: &M4, alpha: F) -> Vec<ObjDraw> {
    let file = &assets.files[obj.file];
    let Ok(nodes) = assets.clump_nodes(obj) else { return Vec::new() };
    let tp = ee::mul(ONE, alpha);
    if ee::le(tp, MIN_TRANSPARENCY) {
        return Vec::new();
    }
    let depth = |mut n: u32| {
        let mut d = 1;
        while let Some(&p) = file.scene.parent.get(&n) {
            if p == 0 || p == n || !nodes.contains(&p) || d > nodes.len() {
                break;
            }
            d += 1;
            n = p;
        }
        d
    };
    nodes
        .iter()
        .filter_map(|&n| {
            let model = *file.obj_model.get(&n)?;
            if !has_mesh(file, model) {
                return None;
            }
            let mut world = *matrix;
            for _ in 0..depth(n) {
                world = vu::mul(&world, &vu::UNIT);
            }
            Some(ObjDraw { obj: n, model, world, alpha: tp })
        })
        .collect()
}

/// `ccAnm::Draw` of an effect's animation posed where `play` is, whose
/// matrix is `matrix` and localtp `alpha`: each object a track of the
/// animation names that has a model, in track order - an ExtObj copy (a
/// drain's `EXT_ene_pl_11`) its own instance of the object it drives, under
/// its own parent - at `matrix` times its parents' poses and its own
/// (VU0's product order), at `alpha` times its pose's transparency.
pub fn anm(assets: &Assets, obj: ObjRef, play: &Play, matrix: &M4, alpha: F) -> Vec<ObjDraw> {
    let file = &assets.files[obj.file];
    let sc = &file.scene;
    let mut locals: HashMap<u32, (M4, F)> = HashMap::new();
    let mut order = Vec::new();
    for (o, pose) in file.anims[play.anim].controllers_at(play.posed) {
        let m = pose.matrix().to_cols_array_2d().map(|c| c.map(f32::to_bits));
        if locals.insert(o, (m, pose.alpha.to_bits())).is_none() {
            order.push(o);
        }
    }
    fn world(sc: &piney_data::scene::Scene, l: &HashMap<u32, (M4, F)>, root: &M4, o: u32, d: usize) -> M4 {
        let local = l.get(&o).map_or(vu::UNIT, |x| x.0);
        let parent = sc.ext_parent.get(&o).or_else(|| sc.parent.get(&o)).copied().unwrap_or(0);
        // A parent the animation does not name hangs from the ccAnm.
        if parent != 0 && parent != o && d < 64 && l.contains_key(&parent) {
            vu::mul(&world(sc, l, root, parent, d + 1), &local)
        } else {
            vu::mul(root, &local)
        }
    }
    let mut out = Vec::new();
    for o in order {
        let target = sc.ext.get(&o).copied().unwrap_or(o);
        let Some(&model) = file.obj_model.get(&target) else { continue };
        if !has_mesh(file, model) {
            continue;
        }
        let tp = ee::mul(alpha, locals[&o].1);
        if ee::le(tp, MIN_TRANSPARENCY) {
            continue;
        }
        out.push(ObjDraw { obj: o, model, world: world(sc, &locals, matrix, o, 0), alpha: tp });
    }
    out
}

/// A model `ccObj::Init` gives the object: a Model chunk with meshes (the
/// dummies' five-word chunks, MDL_x100 and the like, leave the object with
/// none, and `ccObj::Draw` draws nothing).
fn has_mesh(file: &piney_desktop::assets::SceneFile, model: u32) -> bool {
    file.models.get(&model).is_some_and(|m| !m.mmats.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: M4 = [[ONE, 0, 0, 0], [0, 0x4000_0000, 0, 0], [0, 0, ONE, 0], [0x4120_0000, 0, 0xc2c8_0000, ONE]];

    #[test]
    fn the_arrival_ring_and_a_fading_drain() {
        let Some(fx) = crate::testing::effects() else { return };
        let a = &fx.assets;
        // CMP_x032: one node, at the clump's matrix and transparency; none
        // at 1/128.
        let ring = a.find("particle", "CMP_x032").unwrap();
        let d = clump(a, ring, &M, 0x3f00_0000);
        assert_eq!(d.len(), 1);
        assert_eq!(
            (a.name(ObjRef { file: ring.file, object: d[0].obj }), d[0].world, d[0].alpha),
            ("OBJ_x032", M, 0x3f00_0000)
        );
        assert!(clump(a, ring, &M, MIN_TRANSPARENCY).is_empty());
        // A dummy node's empty model is not drawn: CMP_x100 draws its two
        // children only.
        assert_eq!(clump(a, a.find("particle", "CMP_x100").unwrap(), &M, ONE).len(), 2);
        // ANM_xdhdref0: ExtObj copies drawn one each (35 tracks), faded
        // one by one.
        let drain = a.find("drain", "ANM_xdhdref0").unwrap();
        let f = &a.files[drain.file];
        let mut play = Play::new(f, "ANM_xdhdref0").unwrap();
        for _ in 0..20 {
            play.forward(f);
        }
        let d = anm(a, drain, &play, &M, ONE);
        assert_eq!(d.len(), 35);
        assert!(d.iter().any(|x| x.alpha != ONE));
    }
}
