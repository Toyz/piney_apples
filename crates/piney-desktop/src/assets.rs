//! What the desktop reads from the disc at run time: `DATA.BIN` scene files
//! (`xddesk01` and the wallpapers), `DATA/DESKTOP.PRG` (its content tables)
//! and the executable (the bitmap fonts).

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Vec3;
use piney_data::anim::Animation;
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::exe::Overlay;
use piney_data::iso::Iso;
use piney_data::scene::Scene;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_draw::Rgba;

use crate::kanji::Fonts;

/// The desktop overlay (`ccSetupDesktop` loads `cdrom0:\DATA\DESKTOP.PRG`).
pub const PRG_PATH: &str = "DATA/DESKTOP.PRG";
/// `desktopFileList` (0x0041bac0): the desktop's own scene file.
pub const DESK_FILE: &str = "xddesk01";
/// The file `ccKanji`'s palette `CLT_xasc00` is in (`fontSetup`).
pub const FONT_FILE: &str = "xasc00";

/// What a model's draw needs of each mmat's material and texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MmatInfo {
    /// The MAT_ object.
    pub material: Option<u32>,
    /// The Material chunk's transparency (1 without a material).
    pub transparency: f32,
    pub crop_u: u16,
    pub crop_v: u16,
    /// The texture's alpha-test reference and flag byte.
    pub aref: u8,
    pub tex_flag: u8,
}

/// Texture flag bit: always drawn in the sorted (translucent) group.
pub const TEX_FLAG_SORTED: u8 = 0x08;
/// Texture flag bit: CLAMP rather than REPEAT.
pub const TEX_FLAG_CLAMP: u8 = 0x10;

/// A model as the desktop draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub mtype: u16,
    /// The header flag's low two bits (`ccModelChunk` +0x4c): the index into
    /// `alphaBlendTbl`. A model of a type other than 0 is drawn in the
    /// sorted group whatever its transparency (`ccModel::Draw` 0x0013ed70,
    /// `ccModel.flag` 0x20), and writes no Z ([`crate::anm::model_state`]).
    pub blend_type: u8,
    pub mmats: Vec<MmatInfo>,
    /// The centre of the model's vertex box in model space, which the sorted
    /// group is keyed on (`ccModel::Draw` 0x0013eb38): [`vertex_box_centre`]
    /// for a model without `mtype & 6`; None for a bone or skin model, whose
    /// `ccModel` +4 `ccModel::Init` leaves null (0x0013a550).
    pub centre: Option<Vec3>,
}

/// The centre `ccBbox_SetBox` (0x001388c0) gives a model's chunk box
/// (`ccModelChunk` +0x10, centre at +0x30), which `Decode_Model` fills from
/// every vertex: per axis the integer min and max (starting at 0x10000 and
/// -0x10000), `((min + max) >> 1) * (vertexScale / 4096)`. A Bbox chunk
/// (`Decode_Bbox`) goes to a list of its own and never reaches it. Checked
/// against the game's decoder in eemu on every model of `xdttopen0` and
/// `xddesk01`.
pub fn vertex_box_centre(m: &piney_data::model::Model) -> Vec3 {
    let (mut lo, mut hi) = ([0x10000i32; 3], [-0x10000i32; 3]);
    for p in m.mmats.iter().flat_map(|mm| &mm.positions) {
        for k in 0..3 {
            lo[k] = lo[k].min(i32::from(p[k]));
            hi[k] = hi[k].max(i32::from(p[k]));
        }
    }
    let unit = m.scale / 4096.0;
    Vec3::from_array(std::array::from_fn(|k| unit * ((lo[k] + hi[k]) >> 1) as f32))
}

/// One CCSF file with what the desktop draws from it.
pub struct SceneFile {
    /// The `DATA.BIN` member stem, as [`piney_draw::TexRef::Ccs`] names it.
    pub stem: String,
    pub ccs: Ccs,
    pub scene: Scene,
    pub anims: Vec<Animation>,
    anim_by_name: HashMap<String, usize>,
    /// Obj object -> its MDL_ object.
    pub obj_model: HashMap<u32, u32>,
    /// MDL_ object -> the model.
    pub models: HashMap<u32, ModelInfo>,
    /// Shadow MDL_ object -> its vertex scale and mesh (None: refused).
    pub shadows: HashMap<u32, (f32, Option<piney_data::shadow::ShadowMesh>)>,
}

impl SceneFile {
    pub fn read(archive: &Archive, stem: &str) -> Result<Self> {
        let ccs = Ccs::parse(archive.inflate_named(stem)?)?;
        let scene = Scene::read(&ccs)?;
        let anims = Animation::all(&ccs, &scene)?;
        let anim_by_name = anims
            .iter()
            .enumerate()
            .filter_map(|(i, a)| ccs.object_name(a.object).map(|n| (n.to_string(), i)))
            .collect();
        let mut obj_model = HashMap::new();
        for (&m, &o) in &scene.model_owner {
            // A model can be a shadow; the Obj's own model is the one named
            // MDL_ after it, which is the lower index in every desktop file.
            obj_model.entry(o).and_modify(|x: &mut u32| *x = (*x).min(m)).or_insert(m);
        }
        let (textures, _) = piney_data::texture::read(&ccs)?;
        let mut models = HashMap::new();
        let mut shadows = HashMap::new();
        for m in piney_data::model::models(&ccs)? {
            if m.mtype & 8 != 0 {
                let mesh = m.mmats.first().and_then(piney_data::shadow::ShadowMesh::build);
                shadows.insert(m.object, (m.scale, mesh));
            }
            let centre = (m.mtype & 6 == 0).then(|| vertex_box_centre(&m));
            let mmats = m
                .mmats
                .iter()
                .map(|mm| {
                    let mat = mm.material.and_then(|o| scene.materials.get(&o));
                    let tex = mat.and_then(|mt| textures.iter().find(|t| t.object == mt.texture));
                    MmatInfo {
                        material: mm.material,
                        transparency: mat.map_or(1.0, |mt| mt.transparency),
                        crop_u: mat.map_or(0, |mt| mt.crop_u),
                        crop_v: mat.map_or(0, |mt| mt.crop_v),
                        aref: tex.map_or(0, |t| t.aref),
                        tex_flag: tex.map_or(0, |t| t.flag),
                    }
                })
                .collect();
            models.insert(m.object, ModelInfo { mtype: m.mtype, blend_type: (m.flag & 3) as u8, mmats, centre });
        }
        Ok(SceneFile { stem: stem.to_string(), ccs, scene, anims, anim_by_name, obj_model, models, shadows })
    }

    /// `ccStream::GetChunkAdrsF(name)` for an Anime chunk.
    pub fn anim(&self, name: &str) -> Option<usize> {
        self.anim_by_name.get(name).copied()
    }
}

/// Everything the desktop reads from the disc.
pub struct Assets {
    /// The disc's volume, whose tables the desktop reads.
    pub volume: Volume,
    pub archive: Arc<Archive>,
    pub desk: Rc<SceneFile>,
    pub fonts: Fonts,
    files: HashMap<String, Rc<SceneFile>>,
}

/// The volume's two bitmap fonts, with `xasc00`'s palette.
pub fn read_fonts(volume: Volume, archive: &Archive) -> Result<Fonts> {
    let font = Ccs::parse(archive.inflate_named(FONT_FILE)?)?;
    let (_, cluts) = piney_data::texture::read(&font)?;
    let clut = font
        .find_object("CLT_xasc00")
        .and_then(|o| cluts.get(&o))
        .ok_or_else(|| Error::NotFound("CLT_xasc00".into()))?;
    let palette = clut.colours.iter().map(|c| Rgba(*c)).collect();
    Ok(Fonts::of(volume, palette))
}

impl Assets {
    pub fn read(iso: &mut Iso, archive: Arc<Archive>) -> Result<Self> {
        let prg = Overlay::parse(iso.read_path(PRG_PATH)?)?;
        if prg.name != "desktop.prg" {
            return Err(Error::Format(format!("{PRG_PATH} is {}", prg.name)));
        }
        let desk = Rc::new(SceneFile::read(&archive, DESK_FILE)?);
        let volume = iso.volume()?;
        let fonts = read_fonts(volume, &archive)?;
        let mut files = HashMap::new();
        files.insert(DESK_FILE.to_string(), desk.clone());
        Ok(Assets { volume, archive, desk, fonts, files })
    }

    /// A scene file by stem, read on first use (`ccStream::GetCCSAdrs`).
    pub fn file(&mut self, stem: &str) -> Result<Rc<SceneFile>> {
        let key = stem.to_ascii_lowercase();
        if let Some(f) = self.files.get(&key) {
            return Ok(f.clone());
        }
        let f = Rc::new(SceneFile::read(&self.archive, &key)?);
        self.files.insert(key, f.clone());
        Ok(f)
    }
}
