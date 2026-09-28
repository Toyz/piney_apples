//! What a model needs from the rest of its file: Obj parents and models
//! (`Decode_Obj` 0x0014ca40), clump node lists (`Decode_Clump` 0x0014c200),
//! material textures (`Decode_Material` 0x0014d300), ExtObj targets
//! (`Decode_ExtObj` 0x0014cb80: an animation's copy of an object and the
//! object it drives), and Anime chunks.
//!
//! A node's local matrix is `T(pos) * Rx * Ry * Rz * S(scale)`, rotation in
//! degrees (`ccCoord::SetMatrix_PosRotZYXScale` 0x00138120); its world matrix
//! is its parent's times that. Bone and skin vertices are in clump-node space:
//! slot `i` is the `i`-th node of the model's clump, and a vertex lands at
//! node world * position (`ccModel::DrawBoneType` 0x0013f860). Z is up.

use std::collections::HashMap;

use glam::{Mat4, Vec3};

use crate::ccs::{self, Ccs};
use crate::model::{self, Kind, Mmat, Model};
use crate::{Bytes, Result, format_err};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub pos: Vec3,
    /// Degrees.
    pub rot: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Transform { pos: Vec3::ZERO, rot: Vec3::ZERO, scale: Vec3::ONE }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.pos)
            * Mat4::from_rotation_x(self.rot.x.to_radians())
            * Mat4::from_rotation_y(self.rot.y.to_radians())
            * Mat4::from_rotation_z(self.rot.z.to_radians())
            * Mat4::from_scale(self.scale)
    }
}

/// A Material chunk (`Decode_Material` 0x0014d300).
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// The TEX_ object.
    pub texture: u32,
    /// VU1 writes vertex alpha as `128 * transparency`.
    pub transparency: f32,
    /// The reference point of a UV animation: `ccModel::SetUV` (0x0013abe0)
    /// sets the material's runtime offset to `u - crop_u`, and at rest that
    /// offset is 0 (`ccMaterial::Init`, 0x001399f0) - so a static model draws
    /// with no offset whatever these say.
    pub crop_u: u16,
    pub crop_v: u16,
}

/// A DummyPos (0x1300: u32 object, f32 pos[3]) or DummyPosRot (0x1400:
/// the same, then f32 rot[3] in degrees) chunk: a named point the game
/// places things at, e.g. `DMY_floor_01pos` for a town's pieces.
#[derive(Clone, Copy, Debug)]
pub struct Dummy {
    pub pos: Vec3,
    pub rot: Option<Vec3>,
}

#[derive(Clone, Debug)]
pub struct Anime {
    pub object: u32,
    pub offset: usize,
    pub frames: u32,
}

#[derive(Clone, Default)]
pub struct Scene {
    /// object -> parent object (0 = none).
    pub parent: HashMap<u32, u32>,
    /// MDL_ object -> the object that owns it.
    pub model_owner: HashMap<u32, u32>,
    /// Obj object -> its shadow model (the Obj chunk's fourth word).
    pub shadow_of: HashMap<u32, u32>,
    /// (clump object, node objects).
    pub clumps: Vec<(u32, Vec<u32>)>,
    /// MAT_ object -> its material.
    pub materials: HashMap<u32, Material>,
    /// ExtObj object -> the object it drives.
    pub ext: HashMap<u32, u32>,
    /// ExtObj object -> its own parent. An animation places a shared piece
    /// many times through ExtObj copies (a dungeon room puts one floor tile
    /// 20 times), each under its own parent.
    pub ext_parent: HashMap<u32, u32>,
    /// DMY_ object -> its position.
    pub dummies: HashMap<u32, Dummy>,
    pub animes: Vec<Anime>,
}

impl Scene {
    pub fn read(c: &Ccs) -> Result<Self> {
        let d = &c.data;
        let mut s = Scene::default();
        for ch in c.walk().chunks {
            if ch.in_frames {
                break;
            }
            let q = ch.payload();
            match ch.kind {
                ccs::OBJ => {
                    let obj = d.u32_at(q)?;
                    s.parent.insert(obj, d.u32_at(q + 4)?);
                    let model = d.u32_at(q + 8)?;
                    // A shadow model word follows from version 0x96 on.
                    let shadow = if c.version >= 0x96 { d.u32_at(q + 12)? } else { 0 };
                    if shadow != 0 {
                        s.shadow_of.insert(obj, shadow);
                    }
                    for m in [model, shadow] {
                        if m != 0 {
                            s.model_owner.entry(m).or_insert(obj);
                        }
                    }
                }
                ccs::CLUMP => {
                    let obj = d.u32_at(q)?;
                    let n = d.u16_at(q + 4)? as usize;
                    let nodes = (0..n).map(|k| d.u32_at(q + 8 + 4 * k)).collect::<Result<_>>()?;
                    s.clumps.push((obj, nodes));
                }
                ccs::MATERIAL => {
                    // The decoder keeps the first chunk for an object and
                    // skips later ones (0x0014d36c).
                    s.materials.entry(d.u32_at(q)?).or_insert(Material {
                        texture: d.u32_at(q + 4)?,
                        transparency: d.f32_at(q + 8)?,
                        crop_u: d.u16_at(q + 12)?,
                        crop_v: d.u16_at(q + 14)?,
                    });
                }
                ccs::EXT_OBJ => {
                    let obj = d.u32_at(q)?;
                    s.ext.insert(obj, d.u32_at(q + 8)?);
                    s.ext_parent.insert(obj, d.u32_at(q + 4)?);
                }
                ccs::DUMMY_POS | ccs::DUMMY_POS_ROT => {
                    let v3 = |at: usize| -> Result<Vec3> {
                        Ok(Vec3::new(d.f32_at(at)?, d.f32_at(at + 4)?, d.f32_at(at + 8)?))
                    };
                    let rot = if ch.kind == ccs::DUMMY_POS_ROT { Some(v3(q + 16)?) } else { None };
                    s.dummies.insert(d.u32_at(q)?, Dummy { pos: v3(q + 4)?, rot });
                }
                ccs::ANIME => {
                    s.animes.push(Anime { object: d.u32_at(q)?, offset: ch.offset, frames: d.u32_at(q + 4)? });
                }
                _ => {}
            }
        }
        Ok(s)
    }

    pub fn clump_of(&self, obj: u32) -> Option<&[u32]> {
        self.clumps.iter().find(|(_, nodes)| nodes.contains(&obj)).map(|(_, n)| n.as_slice())
    }

    /// Object transforms at frame 0 of an Anime chunk.
    ///
    /// The chunk is u32 object, u32 frame count, u32 data words, then
    /// sub-chunks like the frame section's. Object controllers (0x0102) are
    /// u32 object, u32 flags, then position, rotation (degrees) and scale
    /// controllers picked by flag bits 0-2, 3-5, 6-8: 1 is one value, 2 a u32
    /// key count and (u32 frame, value) keys, which hold their first value
    /// before the first key (`ccAnmCtrlFVec3_SetCtrl` 0x00146be0,
    /// `ccAnmCtrlRot_SetCtrl` 0x001470c0).
    pub fn anime_frame0(&self, c: &Ccs, anime: &Anime) -> Result<HashMap<u32, Transform>> {
        let d = &c.data;
        let words = d.u32_at(anime.offset + 16)? as usize;
        let mut p = anime.offset + 20;
        let end = p + 4 * words;
        let mut out = HashMap::new();
        while p < end {
            let kind = d.u32_at(p)? as u16;
            let n = d.u32_at(p + 4)? as usize;
            if kind == 0x0102 {
                let obj = d.u32_at(p + 8)?;
                let flags = d.u32_at(p + 12)?;
                let mut q = p + 16;
                let mut vals = [Vec3::ZERO, Vec3::ZERO, Vec3::ONE];
                for (k, shift) in [0, 3, 6].into_iter().enumerate() {
                    let v3 = |at: usize| -> Result<Vec3> {
                        Ok(Vec3::new(d.f32_at(at)?, d.f32_at(at + 4)?, d.f32_at(at + 8)?))
                    };
                    match flags >> shift & 7 {
                        1 => {
                            vals[k] = v3(q)?;
                            q += 12;
                        }
                        2 => {
                            let keys = d.u32_at(q)? as usize;
                            vals[k] = v3(q + 8)?;
                            q += 4 + 16 * keys;
                        }
                        _ => {}
                    }
                }
                let target = self.ext.get(&obj).copied().unwrap_or(obj);
                out.entry(target).or_insert(Transform { pos: vals[0], rot: vals[1], scale: vals[2] });
            }
            p += 8 + 4 * n;
        }
        if p != end {
            return format_err(format!("anime sub-chunks overrun at 0x{p:x}"));
        }
        Ok(out)
    }

    /// Every object controller of an Anime chunk at frame 0, one per
    /// controller: unlike [`Scene::anime_frame0`], ExtObj copies of the same
    /// piece stay separate. Returns (controller object, local transform).
    pub fn anime_controllers0(&self, c: &Ccs, anime: &Anime) -> Result<Vec<(u32, Transform)>> {
        let d = &c.data;
        let words = d.u32_at(anime.offset + 16)? as usize;
        let mut p = anime.offset + 20;
        let end = p + 4 * words;
        let mut out = Vec::new();
        while p < end {
            let kind = d.u32_at(p)? as u16;
            let n = d.u32_at(p + 4)? as usize;
            if kind == 0x0102 {
                let obj = d.u32_at(p + 8)?;
                let flags = d.u32_at(p + 12)?;
                let mut q = p + 16;
                let mut vals = [Vec3::ZERO, Vec3::ZERO, Vec3::ONE];
                for (k, shift) in [0, 3, 6].into_iter().enumerate() {
                    let v3 = |at: usize| -> Result<Vec3> {
                        Ok(Vec3::new(d.f32_at(at)?, d.f32_at(at + 4)?, d.f32_at(at + 8)?))
                    };
                    match flags >> shift & 7 {
                        1 => {
                            vals[k] = v3(q)?;
                            q += 12;
                        }
                        2 => {
                            let keys = d.u32_at(q)? as usize;
                            vals[k] = v3(q + 8)?;
                            q += 4 + 16 * keys;
                        }
                        _ => {}
                    }
                }
                out.push((obj, Transform { pos: vals[0], rot: vals[1], scale: vals[2] }));
            }
            p += 8 + 4 * n;
        }
        if p != end {
            return format_err(format!("anime sub-chunks overrun at 0x{p:x}"));
        }
        Ok(out)
    }

    /// World matrices of animation controllers: (controller, the object
    /// whose model it draws, world). A controller's parent is its ExtObj
    /// parent, or its Obj parent; a parent that is itself a controller
    /// contributes its transform, any other its own chain at rest.
    pub fn controller_worlds(&self, controllers: &[(u32, Transform)]) -> Vec<(u32, u32, Mat4)> {
        let locals: Vec<(u32, Mat4)> = controllers.iter().map(|&(o, t)| (o, t.matrix())).collect();
        self.controller_worlds_m(&locals)
    }

    /// [`Scene::controller_worlds`] from local matrices, as an animation at
    /// any time gives them (`anim::Animation::controllers_at`).
    pub fn controller_worlds_m(&self, controllers: &[(u32, Mat4)]) -> Vec<(u32, u32, Mat4)> {
        let locals: HashMap<u32, Mat4> = controllers.iter().copied().collect();
        let mut cache: HashMap<u32, Mat4> = HashMap::new();
        fn w(s: &Scene, locals: &HashMap<u32, Mat4>, cache: &mut HashMap<u32, Mat4>, o: u32, depth: u32) -> Mat4 {
            if let Some(m) = cache.get(&o) {
                return *m;
            }
            let mut m = locals.get(&o).copied().unwrap_or(Mat4::IDENTITY);
            let parent = s.ext_parent.get(&o).or_else(|| s.parent.get(&o)).copied().unwrap_or(0);
            if parent != 0 && parent != o && depth < 256 {
                m = w(s, locals, cache, parent, depth + 1) * m;
            }
            cache.insert(o, m);
            m
        }
        controllers
            .iter()
            .map(|&(o, _)| (o, self.ext.get(&o).copied().unwrap_or(o), w(self, &locals, &mut cache, o, 0)))
            .collect()
    }

    /// World matrices for every object with a parent chain or a transform.
    pub fn world(&self, locals: &HashMap<u32, Transform>) -> HashMap<u32, Mat4> {
        fn w(
            s: &Scene,
            locals: &HashMap<u32, Transform>,
            cache: &mut HashMap<u32, Mat4>,
            obj: u32,
            depth: u32,
        ) -> Mat4 {
            if let Some(m) = cache.get(&obj) {
                return *m;
            }
            let mut m = locals.get(&obj).copied().unwrap_or_default().matrix();
            let parent = s.parent.get(&obj).copied().unwrap_or(0);
            if parent != 0 && parent != obj && depth < 256 {
                m = w(s, locals, cache, parent, depth + 1) * m;
            }
            cache.insert(obj, m);
            m
        }
        let mut cache = HashMap::new();
        for &obj in self.parent.keys().chain(locals.keys()) {
            w(self, locals, &mut cache, obj, 0);
        }
        cache
    }
}

/// One mmat's vertices in world space (or model space when unposed).
pub struct Placed<'a> {
    pub mmat: &'a Mmat,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

/// Each mmat of `model` placed by `world`; None for bone and skin mmats when
/// there is no pose or no clump to place them with. Rigid and shadow mmats
/// without a pose stay in their own space.
pub fn place<'a>(scene: &Scene, m: &'a Model, world: Option<&HashMap<u32, Mat4>>) -> Vec<Option<Placed<'a>>> {
    let owner = scene.model_owner.get(&m.object).copied();
    let nodes = owner.and_then(|o| scene.clump_of(o));
    let node_matrix = |slot: usize| -> Mat4 {
        match (world, nodes.and_then(|n| n.get(slot))) {
            (Some(w), Some(node)) => w.get(node).copied().unwrap_or(Mat4::IDENTITY),
            _ => Mat4::IDENTITY,
        }
    };
    m.mmats
        .iter()
        .map(|mm| match mm.kind {
            Kind::Rigid | Kind::Shadow => {
                let mat = match (world, owner) {
                    (Some(w), Some(o)) => w.get(&o).copied().unwrap_or(Mat4::IDENTITY),
                    _ => Mat4::IDENTITY,
                };
                Some(Placed {
                    mmat: mm,
                    positions: mm.positions.iter().map(|&p| mat.transform_point3(m.position(p))).collect(),
                    normals: mm
                        .normals
                        .iter()
                        .map(|&n| mat.transform_vector3(model::normal(n)).normalize_or_zero())
                        .collect(),
                })
            }
            _ if world.is_none() || nodes.is_none() => None,
            Kind::Bone => {
                let mat = node_matrix(mm.slot.unwrap_or(0) as usize);
                Some(Placed {
                    mmat: mm,
                    positions: mm.positions.iter().map(|&p| mat.transform_point3(m.position(p))).collect(),
                    normals: mm
                        .normals
                        .iter()
                        .map(|&n| mat.transform_vector3(model::normal(n)).normalize_or_zero())
                        .collect(),
                })
            }
            Kind::Skin => {
                let mut positions = Vec::with_capacity(mm.skin.len());
                let mut normals = Vec::with_capacity(mm.skin.len());
                for entries in &mm.skin {
                    let (mut p, mut n) = (Vec3::ZERO, Vec3::ZERO);
                    for e in entries {
                        let mat = node_matrix(e.slot as usize);
                        let w = e.weight as f32 / 256.0;
                        p += w * mat.transform_point3(m.position(e.position));
                        n += w * mat.transform_vector3(model::normal(e.normal));
                    }
                    positions.push(p);
                    normals.push(n.normalize_or_zero());
                }
                Some(Placed { mmat: mm, positions, normals })
            }
        })
        .collect()
}
