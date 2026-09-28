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

/// The Bbox chunk kind.
const BBOX: u16 = 0x0c00;

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
    /// The Bbox chunk's centre in model space, which the sorted group is
    /// keyed on when present (`ccModel::Draw` 0x0013eb38). No desktop or
    /// wallpaper file has one.
    pub centre: Option<Vec3>,
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
        // Bbox chunks (0x0c00: u32 box, u32 target, f32 min[3], f32 max[3]).
        let mut boxes = HashMap::new();
        for ch in ccs.walk().chunks {
            if !ch.in_frames && ch.kind == BBOX {
                let q = ch.payload();
                let f = |k: usize| -> Result<f32> {
                    let b = ccs.data.get(q + 8 + 4 * k..q + 12 + 4 * k).ok_or(Error::Format("short Bbox".into()))?;
                    Ok(f32::from_le_bytes(b.try_into().unwrap()))
                };
                let target = u32::from_le_bytes(ccs.data[q + 4..q + 8].try_into().unwrap());
                let lo = Vec3::new(f(0)?, f(1)?, f(2)?);
                let hi = Vec3::new(f(3)?, f(4)?, f(5)?);
                boxes.insert(target, (lo + hi) * 0.5);
            }
        }
        let mut models = HashMap::new();
        let mut shadows = HashMap::new();
        for m in piney_data::model::models(&ccs)? {
            if m.mtype & 8 != 0 {
                let mesh = m.mmats.first().and_then(piney_data::shadow::ShadowMesh::build);
                shadows.insert(m.object, (m.scale, mesh));
            }
            let centre = boxes.get(&m.object).copied();
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
