//! A character's model as the field draws it: a scene file's clump
//! (`ccClump::Init`), models hung on its nodes (`ccObj::SetModel` 0x0013b8f0,
//! the weapons), palette swaps (`ccAnm::ChangeClut`), posed by an animation
//! ([`Play`]) and drawn by `ccAnm::Draw` (0x001524d0) through `ccObj::Draw`
//! (0x0013f220) at each node's world matrix under `T(pos) Rx Ry Rz(dirc)`.
//! Kite, the towns' merchants and walking PCs and every `CMP_trall` skeleton
//! share it.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::chara::light_matrix;
use crate::draw::{self, CHAR_LAYER, Draw};
use crate::ee::{self, V4};
use crate::pose::Play;
use crate::town::TownLights;

/// The skeleton every playable and non-playable character file carries.
pub const TRALL: &str = "CMP_trall";

/// Member `id`'s weapon file and job as `ccAddRequestFileListEquip`
/// (gcmn 0x005a14b0) finds them for [`Body::equip_weapon`]:
/// `saveData.spcParam[id]`'s job (+0xd8) and weapon (+0xd0) through
/// `ccGetJobWeaponParam`, its `ccsname`.
pub fn weapon_of(
    t: &piney_battle::tables::Tables,
    save: &piney_data::save::SaveData,
    id: i32,
) -> Option<(String, i16)> {
    let id = usize::try_from(id).ok().filter(|&i| i < t.chars.len())?;
    let p = piney_battle::param::SpcParam::from_save(save, id);
    let w = t.job_weapon(i32::from(p.job), i32::from(p.equipment[4]))?;
    let name = String::from_utf8(w.ccsname.clone()).ok().filter(|s| !s.is_empty())?;
    Some((name, p.job))
}

/// A model hung on a clump node: `ccObj::SetModel(model)` on the node,
/// the model from `file` (the body's own file or another, a weapon's).
#[derive(Clone)]
pub struct Attached {
    pub node: u32,
    pub file: Rc<SceneFile>,
    pub model: u32,
}

/// A character's clump and what is hung on it.
#[derive(Clone)]
pub struct Body {
    pub file: Rc<SceneFile>,
    /// The clump's node objects, by slot (`ccClump`'s node table: skin and
    /// bone meshes index it).
    pub nodes: Vec<u32>,
    /// Models replacing their nodes' own.
    pub attached: Vec<Attached>,
    /// Materials drawn with another texture of the file (MAT_, TEX_):
    /// `ccClump::ChangeTex`, the walking PCs' `changeTEX`.
    pub tex_swaps: Vec<(u32, u32)>,
    /// `ccAnm::SetBlendType(t)` (main 0x00152be0): every model drawn with
    /// `alphaBlendTbl[t]` instead of its own blend (the spring's water, 4).
    pub blend: Option<u8>,
}

impl Body {
    /// `ccClump::Init` of `clump` in the scene file `stem`.
    pub fn read(archive: &Arc<Archive>, stem: &str, clump: &str) -> Result<Body> {
        Body::of(Rc::new(SceneFile::read(archive, stem)?), clump)
    }

    /// The clump `clump` of an already read file. Its nodes are the
    /// objects `ccClump::Init` finds for them: an ExtObj copy followed to
    /// the object it stands for, as `ccMatchIndex` (0x00101ad0) follows
    /// them (a file whose first clump of the name is an animation's - the
    /// Grunties' - poses and draws the model's objects through it).
    pub fn of(file: Rc<SceneFile>, clump: &str) -> Result<Body> {
        let c = file.ccs.find_object(clump).ok_or_else(|| Error::NotFound(clump.into()))?;
        let ext = &file.scene.ext;
        let resolve = |mut o: u32| {
            for _ in 0..16 {
                match ext.get(&o) {
                    Some(&t) if t != o => o = t,
                    _ => break,
                }
            }
            o
        };
        let nodes = file
            .scene
            .clumps
            .iter()
            .find(|(o, _)| *o == c)
            .map(|(_, n)| n.iter().map(|&o| resolve(o)).collect())
            .unwrap_or_default();
        Ok(Body { file, nodes, attached: Vec::new(), tex_swaps: Vec::new(), blend: None })
    }

    /// `ccClump::GetObjAdrsF(name)` (0x0013d300): the first of the clump's
    /// own nodes with that name. A character file also has an object of each
    /// node's name in every animation (ExtObj copies) that nothing poses;
    /// only the clump's count.
    pub fn node(&self, name: &str) -> Option<u32> {
        self.nodes.iter().copied().find(|&o| self.file.ccs.object_name(o) == Some(name))
    }

    /// `ccObj::SetModel` of `model` from `file` on the node `node`: true
    /// when both exist.
    pub fn attach(&mut self, node: &str, file: Rc<SceneFile>, model: &str) -> bool {
        let (Some(n), Some(m)) = (self.node(node), file.ccs.find_object(model)) else { return false };
        self.attached.retain(|a| a.node != n);
        self.attached.push(Attached { node: n, file, model: m });
        true
    }

    /// `ccSpcChar::EquipWeapon` (gcmn 0x0059d650) as `ccPlayer::ccPlayer`
    /// calls it: the models of the weapon file `weapon` (`ccMakeModelName`:
    /// `MDL_` and the file's name, then `r`, `l` or nothing) on the hands
    /// (`ccPlayer`'s +0x12c `OBJ_t0 l hand`, +0x130 `OBJ_t0 r hand`) by
    /// job: job 0's two blades `r` on the right and `l` on the left, job
    /// 3's one model on the left, every other job's on the right. False
    /// when the file cannot be read.
    pub fn equip_weapon(&mut self, archive: &Arc<Archive>, weapon: &str, job: i16) -> bool {
        const L: &str = "OBJ_t0 l hand";
        const R: &str = "OBJ_t0 r hand";
        let Ok(file) = SceneFile::read(archive, weapon) else { return false };
        let file = Rc::new(file);
        match job {
            0 => {
                self.attach(R, file.clone(), &format!("MDL_{weapon}r"));
                self.attach(L, file, &format!("MDL_{weapon}l"));
            }
            3 => {
                self.attach(L, file, &format!("MDL_{weapon}"));
            }
            _ => {
                self.attach(R, file, &format!("MDL_{weapon}"));
            }
        }
        true
    }

    /// `ccStream::GetChunkAdrsF` of an Anime chunk in the body's file.
    pub fn anim(&self, name: &str) -> Option<usize> {
        self.file.anim(name)
    }

    /// `ccAnm::SetAnm` of the named animation: frame 0.
    pub fn play(&self, name: &str) -> Option<Play> {
        Play::new(&self.file, name)
    }

    /// The world matrix of every clump node and every object `play` poses,
    /// Each object `play`'s animation drives, with its parent object in
    /// that animation: `ccAnm::SetAnm` puts an ExtObj copy under its own
    /// ExtObj parent (what that copy drives; 0, the anm itself), so a node
    /// can have another parent in each animation than its Obj chunk gives
    /// it (`ebl1`'s `OBJ_dummy05` is under the tail in the clump and under
    /// the head in every animation); an object driven directly keeps its
    /// Obj parent. The first track for a target wins, as for the pose.
    pub fn parents(&self, play: &Play) -> HashMap<u32, u32> {
        let file = &self.file;
        let sc = &file.scene;
        let mut out = HashMap::new();
        for tr in &file.anims[play.anim].tracks {
            let p = if sc.ext.contains_key(&tr.object) {
                let copy = sc.ext_parent.get(&tr.object).copied().unwrap_or(0);
                if copy == 0 { 0 } else { sc.ext.get(&copy).copied().unwrap_or(copy) }
            } else {
                sc.parent.get(&tr.target).copied().unwrap_or(0)
            };
            out.entry(tr.target).or_insert(p);
        }
        out
    }

    /// Each clump node's transparency from the animation (`ccCoord::
    /// _GetTransparency`, main 0x00138490): its own animated value.
    /// `SetAnmCtrlWork` (0x001507c8) multiplies the parent's in only for a
    /// node whose Obj2 flags (`succession`) have bit 0, and no Obj2 chunk
    /// of any volume's `DATA.BIN` sets it; a node without one has flags 0.
    /// So a root keyed to 0 (the d, f, l and w Gott statues' `OBJ_trall`
    /// as they fall) hides nothing under it.
    pub fn node_alphas(&self, play: &Play) -> HashMap<u32, f32> {
        let a = &self.file.anims[play.anim];
        // The node is the last entry's object that names it (`SetAnm`).
        let local: HashMap<u32, f32> = a
            .objects
            .iter()
            .zip(a.entry_poses_at(play.posed))
            .map(|(&(_, target), pose)| (target, pose.map_or(1.0, |p| p.alpha)))
            .collect();
        self.nodes.iter().map(|&n| (n, local.get(&n).copied().unwrap_or(1.0))).collect()
    }

    /// under `root`.
    pub fn worlds(&self, play: &Play, root: Mat4) -> HashMap<u32, Mat4> {
        play.worlds(&self.file, root, &self.nodes)
    }

    /// `ccAnm::Draw` of the body posed by `play` under `root`, at
    /// transparency `alpha`, on layer `layer`: each of the anm's objects
    /// with a model ([`Play::instances`]; an attached one in place of its
    /// node's own), in the index's order; lit models lit by `lights` at
    /// their position (the main light dimmed when `shaded`); `clut_swaps`
    /// for the body's own models.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        play: &Play,
        root: Mat4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        clut_swaps: &[(u32, u32)],
    ) {
        self.draw_fog(
            layers,
            layer,
            to_screen,
            play,
            root,
            alpha,
            lights,
            shaded,
            clut_swaps,
            crate::draw::Fogging::None,
        );
    }

    /// [`Body::draw`] with the character's blend (`ccDrawEnv::SetFogBlend`,
    /// each model's fog forced to it).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_fog(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        play: &Play,
        root: Mat4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        clut_swaps: &[(u32, u32)],
        fog: crate::draw::Fogging,
    ) {
        self.draw_parts(layers, layer, to_screen, play, root, alpha, lights, shaded, clut_swaps, fog, alpha);
    }

    /// [`Body::draw_fog`] with the attached models (the blades) at their
    /// own transparency `arms` (`GateHackingOut`'s `ghoCamArmsT`; 0 hides
    /// them, their `dispSW` 0).
    #[allow(clippy::too_many_arguments)]
    fn draw_parts(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        play: &Play,
        root: Mat4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        clut_swaps: &[(u32, u32)],
        fog: crate::draw::Fogging,
        arms: f32,
    ) {
        let worlds = self.worlds(play, root);
        let node_mats: Vec<Mat4> = self.nodes.iter().map(|o| worlds.get(o).copied().unwrap_or(root)).collect();
        let rows = Default::default();
        // No blend: the draw environment's fog, by each vertex's depth.
        let fog = match (fog, lights.fog) {
            (crate::draw::Fogging::None, Some(d)) => crate::draw::Fogging::Depth(d),
            (f, _) => f,
        };
        let instances = play.instances(&self.file, root);
        // A model hung on a node is the clump's ccObj's: the last entry
        // naming the node (`ccAnm::SetAnm`); the others are the anm's own.
        let hung = |k: usize| {
            let obj = instances[k].object;
            let last = instances[k + 1..].iter().all(|i| i.object != obj);
            self.attached.iter().position(|a| a.node == obj).filter(|_| last && self.nodes.contains(&obj))
        };
        let hung: Vec<Option<usize>> = (0..instances.len()).map(hung).collect();
        for (inst, &k) in instances.iter().zip(&hung) {
            if k.is_some() && arms <= 0.0 {
                continue;
            }
            let (file, model, info) = match k {
                Some(k) => {
                    let a = &self.attached[k];
                    let Some(info) = a.file.models.get(&a.model) else { continue };
                    (&a.file, a.model, info)
                }
                None => {
                    let Some((model, info)) = self.file.drawn_model(inst.object) else { continue };
                    (&self.file, model, info)
                }
            };
            let world = inst.world;
            let lit = info.mtype & 1 != 0;
            let d = Draw {
                file,
                model,
                world,
                alpha: if k.is_some() { arms } else { alpha * inst.alpha },
                rows: &rows,
                lights: lit.then(|| light_matrix(lights, world, shaded)),
                nodes: if info.mtype & 6 != 0 { &node_mats } else { &[] },
                morph: Vec::new(),
                // ChangeClut swaps the body's palette; models from another
                // file use their own.
                clut_swaps: if k.is_some() { Vec::new() } else { clut_swaps.to_vec() },
            };
            // ChangeTex is the body's clump's: attached models keep theirs.
            let swaps: &[(u32, u32)] = if k.is_some() { &[] } else { &self.tex_swaps };
            draw::model_blended(layers, layer, to_screen, d, None, fog, swaps, self.blend);
        }
        // The shadow models: the instances', and each attached model's own
        // object's, at the node it rides.
        draw::cast_shadows(layers, &self.file, instances.iter().map(|i| (i.object, i.world)));
        for (inst, k) in instances.iter().zip(hung) {
            let Some(a) = k.map(|k| &self.attached[k]).filter(|_| arms > 0.0) else { continue };
            let Some(&owner) = a.file.scene.model_owner.get(&a.model) else { continue };
            draw::cast_shadows(layers, &a.file, [(owner, inst.world)]);
        }
    }

    /// [`Body::draw`] on the character layer (`WORLD_MAN::SetActiveLayer(5)`,
    /// priority 10), as `ccChar::Draw` sets it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_char(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        play: &Play,
        pos: V4,
        dirc: V4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        clut_swaps: &[(u32, u32)],
    ) {
        self.draw(layers, CHAR_LAYER, to_screen, play, root(pos, dirc), alpha, lights, shaded, clut_swaps);
    }

    /// [`Body::draw_char`] with the character's blend.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_char_fog(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        play: &Play,
        pos: V4,
        dirc: V4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        clut_swaps: &[(u32, u32)],
        fog: crate::draw::Fogging,
    ) {
        let r = root(pos, dirc);
        self.draw_fog(layers, CHAR_LAYER, to_screen, play, r, alpha, lights, shaded, clut_swaps, fog);
    }

    /// [`Body::draw_char`] with the blades at their own transparency
    /// (`GateHackingOut`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_char_arms(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        play: &Play,
        pos: V4,
        dirc: V4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        clut_swaps: &[(u32, u32)],
        arms: f32,
    ) {
        let r = root(pos, dirc);
        self.draw_parts(
            layers,
            CHAR_LAYER,
            to_screen,
            play,
            r,
            alpha,
            lights,
            shaded,
            clut_swaps,
            crate::draw::Fogging::None,
            arms,
        );
    }
}

/// `ccCoord::SetMatrix_PosRotZYX(pos, dirc)`: T(pos) Rx Ry Rz.
pub fn root(pos: V4, dirc: V4) -> Mat4 {
    let p = Vec3::new(ee::f(pos[0]), ee::f(pos[1]), ee::f(pos[2]));
    Mat4::from_translation(p)
        * Mat4::from_rotation_x(ee::f(dirc[0]))
        * Mat4::from_rotation_y(ee::f(dirc[1]))
        * Mat4::from_rotation_z(ee::f(dirc[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #20: every Gott statue lies where it fell. Its fall
    /// (`ANM_xgs?dwn0`, last frame) and opened pose (`ANM_xgs?nut1`) key
    /// the ring and the box to 0, and in the d, f, l and w statues the
    /// root `OBJ_trall` too; the statue itself (`OBJ_o_god_m0_`) stays
    /// drawn, as no node inherits its parent's transparency.
    #[test]
    fn a_fallen_gott_statue_stays_drawn() {
        let Some(archive) = crate::town::tests::archive() else { return };
        for e in ["a", "d", "e", "f", "l", "t", "w"] {
            let b = Body::read(&archive, &format!("xgs{e}bod1"), "CMP_trall").unwrap();
            for (anim, frame) in [(format!("ANM_xgs{e}dwn0"), 140u32), (format!("ANM_xgs{e}nut1"), 1)] {
                let mut p = crate::pose::Play::new(&b.file, &anim).unwrap();
                (p.time, p.posed) = (frame * 256, frame * 256);
                let alphas = b.node_alphas(&p);
                let of = |name: &str| b.node(name).and_then(|n| alphas.get(&n).copied());
                assert_eq!(of("OBJ_o_god_m0_"), Some(1.0), "{anim}: the statue");
                assert_eq!(of("OBJ_o_magic_m0_"), Some(0.0), "{anim}: the ring");
            }
        }
    }
}
