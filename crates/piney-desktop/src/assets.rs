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
use piney_data::model::VertexBox;
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
    /// The model's vertex box (`ccModel` +4), in whose centre the sorted
    /// group is keyed (`ccModel::Draw` 0x0013eb38) and against which it
    /// culls: [`piney_data::model::Model::vertex_box`]; None for a bone or
    /// skin model, whose `ccModel` +4 `ccModel::Init` leaves null
    /// (0x0013a550). A Bbox chunk (`Decode_Bbox`) goes to a list of its own
    /// and never reaches it.
    pub vertex_box: Option<VertexBox>,
}

impl ModelInfo {
    /// The box's centre in model space.
    pub fn centre(&self) -> Option<Vec3> {
        self.vertex_box.map(|b| b.centre_point())
    }

    /// The box's greatest corner when `ccModel::Draw` checks it: a rigid
    /// model with mmats (+0x22) seen above 1/128 (`ccObj::Draw` 0x0013f2b0).
    pub fn checked_max(&self, transparency: u32) -> Option<[u32; 4]> {
        let seen = piney_data::field::ee::lt(0x3c00_0000, transparency);
        self.vertex_box.filter(|_| seen && !self.mmats.is_empty()).map(|b| b.max)
    }
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
    pub shadows: HashMap<u32, (f32, Option<Arc<piney_data::shadow::ShadowMesh>>)>,
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
                let mesh = m.mmats.first().and_then(piney_data::shadow::ShadowMesh::build).map(Arc::new);
                shadows.insert(m.object, (m.scale, mesh));
            }
            let vertex_box = m.vertex_box();
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
            models.insert(m.object, ModelInfo { mtype: m.mtype, blend_type: (m.flag & 3) as u8, mmats, vertex_box });
        }
        Ok(SceneFile { stem: stem.to_string(), ccs, scene, anims, anim_by_name, obj_model, models, shadows })
    }

    /// A file with nothing in it: the stream of a map that loads none
    /// (Quarantine's ending), where no name is found.
    pub fn empty() -> SceneFile {
        let ccs = Ccs {
            data: Vec::new(),
            name: String::new(),
            version: 0,
            frames: 0,
            header_words: Vec::new(),
            files: Vec::new(),
            objects: Vec::new(),
        };
        let (anims, anim_by_name) = (Vec::new(), HashMap::new());
        let (obj_model, models, shadows) = (HashMap::new(), HashMap::new(), HashMap::new());
        SceneFile { stem: String::new(), ccs, scene: Scene::default(), anims, anim_by_name, obj_model, models, shadows }
    }

    /// `ccStream::GetChunkAdrsF(name)` for an Anime chunk.
    pub fn anim(&self, name: &str) -> Option<usize> {
        self.anim_by_name.get(name).copied()
    }

    /// The model `ccObj::Draw` draws for the Obj `obj`: its chunk's, when
    /// that is not a shadow and has something to draw.
    pub fn drawn_model(&self, obj: u32) -> Option<(u32, &ModelInfo)> {
        let &m = self.scene.model_of.get(&obj)?;
        self.models.get(&m).filter(|i| i.mtype & 8 == 0 && !i.mmats.is_empty()).map(|i| (m, i))
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
