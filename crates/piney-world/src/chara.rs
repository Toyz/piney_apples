//! Kite drawn: `ccChar::Draw` (gcmn 0x0056b1c0) through `ccAnm::Draw`
//! (0x001524d0) on the character layer, his clump `CMP_trall` posed by the
//! act's animation under `T(pos) Rx Ry Rz(dirc)`, the skinned body lit by
//! the town's lights (`ccDrawEnv::SetLightMatrix` 0x00105900), the weapon
//! in each hand, and before the Data Drain bracelet (`saveData.plcol` 0)
//! the body's palette swapped for `CLT_ctu1bodyc1`.
//!
//! The weapons: `ccPlayer::ccPlayer` (gcmn 0x00597910) looks the hands up
//! among the clump's own nodes by name (`ccClump::GetObjAdrsF` 0x0013d300:
//! `OBJ_t0 l hand` into `+0x12c` at 0x00598038, `OBJ_t0 r hand` into
//! `+0x130` at 0x00598060), then `ccSpcChar::EquipWeapon` (gcmn 0x0059d650)
//! takes the weapon's file and, for job 0 (twin blades: jump table
//! 0x006f09e0 to 0x0059d854), makes `ccModel`s of `MDL_<file>r` and
//! `MDL_<file>l` (`ccMakeModelName` 0x005a1530, suffix 1 "r", 2 "l") and
//! `ccObj::SetModel`s (0x0013b8f0) them onto the right and the left hand,
//! in place of the hands' own empty models. From then on a blade is one of
//! the clump's models: `ccAnm::Draw` draws every clump node the act's
//! animation poses (all 22 of `ctu1body`'s pose both hands) through
//! `ccObj::Draw` (0x0013f220), at the node's world matrix from
//! `ccCoord::_SetLWMatrix` (0x00138380). So each blade is a rigid model in
//! its hand's space: the hand's world matrix, no offset, scale or dummy of
//! its own, the body's transparency and lights, in every act and area. The
//! weapon file's own clumps and objects (`CMP_cwdhsw02r`, `OBJ_cwdhsw02r`)
//! are never built; its `DMY_xdummy_w01`-`w04` only place the weapon
//! trails (`ccSpcChar::ArmsEffect` 0x0059ddd0), not drawn here.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat3, Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;
use piney_draw::Lights;

use crate::body::Body;
use crate::ee::{ONE, V4};
use crate::motion::PLAYER_ANIM_TBL;
use crate::pose::Play;
use crate::town::{Light, TownLights};

/// Kite's file (`charTbl[0].ccsname`) and his clump.
pub const BODY: &str = "ctu1body";
pub const CLUMP: &str = "CMP_trall";
/// His weapon's file (`equipmentWeapon1Tbl[0]`, the Amateur Blades) and
/// what `EquipWeapon` hangs where: the clump node, by name, and the model
/// it gets, in the clump's (and so the draw's) order.
pub const WEAPON: &str = "cwdhsw02";
pub const HANDS: [(&str, &str); 2] = [("OBJ_t0 r hand", "MDL_cwdhsw02r"), ("OBJ_t0 l hand", "MDL_cwdhsw02l")];
/// The palette `ccPlayer::ccPlayer` swaps in before the bracelet.
pub const CLUT: &str = "CLT_ctu1body";
pub const CLUT_EARLY: &str = "CLT_ctu1bodyc1";

/// `mc_SetMatrix`'s doubling of the light colours.
pub const VU_LIGHT_SCALE: f32 = 2.0;
/// `WORLD_MAN::SleepDistantLight`: the main light's intensity on shaded
/// ground (`hitAttribute` bit 0x40000).
pub const SHADE: f32 = 0.3;

/// Kite's model files, read once.
pub struct Kite {
    /// His clump with the blades hung on its hands.
    pub body: Body,
    pub file: Rc<SceneFile>,
    pub weapon: Option<Rc<SceneFile>>,
    /// The clump's node objects, by slot.
    pub nodes: Vec<u32>,
    /// `playerAnimTbl` resolved: the file's animation of each act.
    pub act_anims: Vec<usize>,
    clut: Option<(u32, u32)>,
    /// (hand node, weapon model) as `EquipWeapon` leaves them.
    hands: Vec<(u32, u32)>,
}

impl Kite {
    /// With the Amateur Blades.
    pub fn read(archive: &Arc<Archive>) -> Result<Kite> {
        Kite::read_armed(archive, WEAPON)
    }

    /// With the blades of the file `weapon` (`EquipWeapon`'s job 0: its
    /// `r` and `l` models on the hands), the save's
    /// ([`crate::body::weapon_of`]).
    pub fn read_armed(archive: &Arc<Archive>, weapon: &str) -> Result<Kite> {
        let file = Rc::new(SceneFile::read(archive, BODY)?);
        let (c, sc) = (&file.ccs, &file.scene);
        let clump = c.find_object(CLUMP).ok_or_else(|| Error::NotFound(CLUMP.into()))?;
        let nodes = sc.clumps.iter().find(|(o, _)| *o == clump).map(|(_, n)| n.clone()).unwrap_or_default();
        let act_anims = PLAYER_ANIM_TBL
            .iter()
            .map(|n| file.anim(n).ok_or_else(|| Error::NotFound((*n).into())))
            .collect::<Result<Vec<_>>>()?;
        let clut = c.find_object(CLUT).zip(c.find_object(CLUT_EARLY));
        let body = Body::of(file.clone(), CLUMP)?;
        let mut kite = Kite { body, file, weapon: None, nodes, act_anims, clut, hands: Vec::new() };
        kite.arm(archive, weapon);
        Ok(kite)
    }

    /// `EquipWeapon`'s job 0 with the weapon file `weapon`: its `r` and
    /// `l` models on his hands, in place of the blades he held (as
    /// `ccSPC::ChangeWeapon` hangs a new weapon). Nothing changes when the
    /// file cannot be read.
    pub fn arm(&mut self, archive: &Arc<Archive>, weapon: &str) {
        let hands_of = [("OBJ_t0 r hand", format!("MDL_{weapon}r")), ("OBJ_t0 l hand", format!("MDL_{weapon}l"))];
        let Ok(w) = SceneFile::read(archive, weapon).map(Rc::new) else { return };
        // GetObjAdrsF searches the clump's nodes. The file also has an
        // object of each node's name in every animation (ExtObj copies,
        // ANM_ctu1atc0's first), which nothing poses: a blade hung on one
        // of those stands at his feet.
        let c = &self.file.ccs;
        let node = |name: &str| self.nodes.iter().copied().find(|&o| c.object_name(o) == Some(name));
        self.hands = hands_of.iter().filter_map(|(obj, mdl)| Some((node(obj)?, w.ccs.find_object(mdl)?))).collect();
        for (obj, mdl) in &hands_of {
            self.body.attach(obj, w.clone(), mdl);
        }
        self.weapon = Some(w);
    }

    /// The animations of the acts, for `AnimCtrl`.
    pub fn anims(&self) -> Vec<&piney_data::anim::Animation> {
        self.act_anims.iter().map(|&i| &self.file.anims[i]).collect()
    }

    /// The palette swaps his body is drawn with: before the bracelet
    /// (`saveData.plcol` 0) `CLT_ctu1bodyc1` for `CLT_ctu1body`.
    pub fn swaps(&self, bracelet: bool) -> Vec<(u32, u32)> {
        match (bracelet, self.clut) {
            (false, Some(s)) => vec![s],
            _ => Vec::new(),
        }
    }

    /// (hand node, weapon model) as `EquipWeapon` leaves them, right hand
    /// first; empty without the weapon's file.
    pub fn hands(&self) -> &[(u32, u32)] {
        &self.hands
    }

    /// The world matrix of every clump node in act `act` at animation time
    /// `posed`: the animation's local matrices through the node parents
    /// under the anm's `root(pos, dirc)`, as `ccCoord::_SetLWMatrix`
    /// composes them for `ccObj::Draw`.
    pub fn worlds(&self, act: i16, posed: u32, pos: V4, dirc: V4) -> HashMap<u32, Mat4> {
        let play = Play { anim: self.act_anims[act.clamp(0, 25) as usize], time: posed, posed, frame_spd: 256 };
        play.worlds(&self.file, root(pos, dirc), &self.nodes)
    }

    /// His feet (`OBJ_t0 l foot`, `OBJ_t0 r foot`) in the world in act
    /// `act` at time `posed`: where `ccEffPawSmoke` puts a running step's
    /// dust.
    pub fn feet(&self, act: i16, posed: u32, pos: V4, dirc: V4) -> Option<[V4; 2]> {
        let worlds = self.worlds(act, posed, pos, dirc);
        let foot = |name: &str| {
            let t = worlds.get(&self.file.ccs.find_object(name)?)?.w_axis;
            Some([t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE])
        };
        Some([foot("OBJ_t0 l foot")?, foot("OBJ_t0 r foot")?])
    }

    /// Each weapon model and the matrix [`Kite::draw`] draws it at: its
    /// hand node's world matrix as it is (`root` for a node nothing
    /// poses), in [`HANDS`] order.
    pub fn weapons(&self, worlds: &HashMap<u32, Mat4>, root: Mat4) -> Vec<(u32, Mat4)> {
        self.hands.iter().map(|&(hand, model)| (model, worlds.get(&hand).copied().unwrap_or(root))).collect()
    }

    /// `ccChar::Draw` then `ccAnm::Draw`: every clump node with a model,
    /// in the clump's order, the hands drawing the weapons `EquipWeapon`
    /// put in place of their own models, at transparency `alpha`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        act: i16,
        posed: u32,
        pos: V4,
        dirc: V4,
        alpha: f32,
        lights: &TownLights,
        shaded: bool,
        bracelet: bool,
    ) {
        let play = Play { anim: self.act_anims[act.clamp(0, 25) as usize], time: posed, posed, frame_spd: 256 };
        let swaps = match (bracelet, self.clut) {
            (false, Some(s)) => vec![s],
            _ => Vec::new(),
        };
        self.body.draw_char(layers, to_screen, &play, pos, dirc, alpha, lights, shaded, &swaps);
    }
}

pub use crate::body::root;

/// `CheckRange` of one light at `at`: (the direction it travels, colour
/// times intensity), or None when it does not reach (`ccOmniLight::
/// CheckRange` 0x00139830, `ccDirectLight::CheckRange` 0x00139330,
/// `ccDistantLight::CheckRange` 0x00139170).
fn check_range(l: &Light, at: Vec3, shaded: bool) -> Option<(Vec3, Vec3)> {
    let intensity = if l.kind == 1 && shaded { SHADE } else { l.intensity };
    if intensity <= 1.0 / 128.0 {
        return None;
    }
    match l.kind {
        4 => {
            let d = at - l.pos;
            let f = if l.far_end == 0.0 {
                intensity
            } else if d.length_squared() <= l.far_end * l.far_end {
                let dist = d.length();
                if dist <= l.far_start {
                    intensity
                } else {
                    let f = intensity * (l.far_end - dist) / (l.far_end - l.far_start);
                    if f <= 1.0 / 128.0 {
                        return None;
                    }
                    f
                }
            } else {
                return None;
            };
            Some((d.normalize_or_zero(), l.colour * f))
        }
        // A beam down the light's -z: `at` in the light's own space (its
        // matrix inverted), nothing behind it or past the beam's end, full
        // to its start and to the first radius, falling off linearly to the
        // end and to the second radius.
        // The beam is round, so only its axis matters: the depth along it
        // and the square of the distance from it.
        2 => {
            let d = at - l.pos;
            let axis = l.dir.normalize_or_zero();
            let z = axis.dot(d);
            if z < 0.0 || (l.far_end != 0.0 && z > l.far_end) {
                return None;
            }
            let r2 = (d - axis * z).length_squared();
            let [r0, r1] = l.radius;
            if r2 > r1 * r1 {
                return None;
            }
            let mut f = if l.far_end == 0.0 || z <= l.far_start {
                intensity
            } else {
                intensity * (l.far_end - z) / (l.far_end - l.far_start)
            };
            let r = r2.sqrt();
            if r > r0 {
                f = f * (r1 - r) / (r1 - r0);
            }
            if f <= 1.0 / 128.0 {
                return None;
            }
            Some((l.dir, l.colour * f))
        }
        _ => Some((l.dir, l.colour * intensity)),
    }
}

/// `ccDrawEnv::SetLightMatrix` for a lit model at `world`: the first three
/// lights of the group that reach its position fill slots 2, 1, 0 (the
/// group runs in descending priority: omni lights before the distant one);
/// directions toward each light in model space, colours doubled as VU1
/// does, the town's ambient.
pub fn light_matrix(town: &TownLights, world: Mat4, shaded: bool) -> Lights {
    let at = world.w_axis.truncate();
    let mut order: Vec<&Light> = town.lights.iter().collect();
    order.sort_by_key(|l| -i32::from(l.priority));
    let mut out = Lights { dirs: [[0.0; 3]; 3], colours: [[0.0; 3]; 3], ambient: town.ambient.into() };
    let w = Mat3::from_mat4(world).transpose();
    let mut used = 0;
    for l in order {
        if used == 3 {
            break;
        }
        let Some((normal, colour)) = check_range(l, at, shaded) else { continue };
        let slot = 2 - used;
        out.dirs[slot] = (w * -normal.normalize_or_zero()).normalize_or_zero().into();
        out.colours[slot] = (colour * VU_LIGHT_SCALE).into();
        used += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use piney_data::iso::Iso;

    use super::*;
    use crate::ee;

    const ISO: &str = "../../work/infection/infection.iso";

    /// The blades six frames into standing (act 2) and running (act 5,
    /// turned to face +y) at the arrival spot, as the game's own
    /// `EquipWeapon`, `ccObj::Draw` and `ccCoord::_SetLWMatrix` leave them
    /// (`tools/test_world_rs.py`'s `KiteWeapons`, which checks many more).
    #[test]
    fn blades_in_his_hands() {
        if !std::path::Path::new(ISO).exists() {
            eprintln!("skipped: no {ISO}");
            return;
        }
        let mut iso = Iso::open(ISO).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let kite = Kite::read(&archive).unwrap();
        let weapon = &kite.weapon.as_ref().unwrap().ccs;
        let names: Vec<(&str, &str)> = kite
            .hands()
            .iter()
            .map(|&(h, m)| (kite.file.ccs.object_name(h).unwrap(), weapon.object_name(m).unwrap()))
            .collect();
        assert_eq!(names, HANDS);
        assert!(kite.hands().iter().all(|(h, _)| kite.nodes.contains(h)));
        let pos = [0, 0x45af_0000, 0x4416_0000, ee::ONE];
        #[rustfmt::skip]
        let cases: [(i16, u32, [[f32; 16]; 2]); 2] = [
            (2, 0, [
                [-0.1369921, -0.4177183, -0.8981872, 0.0, 0.829825, 0.4467678, -0.334343, 0.0,
                 0.5409435, -0.7911417, 0.28543, 0.0, -27.77782, 5598.042, 683.8307, 1.0],
                [-0.05051316, -0.1227963, -0.9911435, 0.0, -0.8257129, 0.5634058, -0.02772048, 0.0,
                 0.561821, 0.817001, -0.129854, 0.0, 27.11436, 5593.671, 682.38, 1.0],
            ]),
            (5, 0x4049_0fdb, [
                [0.2499207, -0.2513359, -0.9350734, 0.0, -0.9343266, -0.3160318, -0.164776, 0.0,
                 -0.2540998, 0.9148473, -0.3138132, 0.0, 27.69668, 5587.319, 686.4367, 1.0],
                [0.1896931, 0.2642214, -0.9456193, 0.0, 0.7300346, -0.6819801, -0.04411086, 0.0,
                 -0.6565512, -0.6819696, -0.3222591, 0.0, -30.36952, 5609.718, 674.8478, 1.0],
            ]),
        ];
        for (act, dirc_z, want) in cases {
            let dirc = [0, 0, dirc_z, 0];
            let got = kite.weapons(&kite.worlds(act, 1536, pos, dirc), root(pos, dirc));
            assert_eq!(got.len(), 2);
            for ((_, m), w) in got.iter().zip(want) {
                for (k, (g, w)) in m.to_cols_array().iter().zip(w).enumerate() {
                    let tolerance = if k >= 12 { 1e-2 } else { 1e-4 };
                    assert!((g - w).abs() <= tolerance, "act {act} element {k}: {g} against the game's {w}");
                }
            }
        }
    }
}
