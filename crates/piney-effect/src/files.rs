//! The effects' files and tables: `effectCCSTbl` (main 0x002fd4f0, the twelve
//! files the effects draw from), `effectTbl` (0x0033f240, 173 rows: file,
//! object, kind by effect id) and `effectTbl2` (0x0033fa60, the 4 rows a town
//! resolves), read from the disc and the executable. In a town
//! `ccEffectCtrl::ccEffectCtrl(0)` (main 0x001c3030) resolves only id 3
//! (`CMP_x032`); the others keep whatever the heap held.

use std::collections::HashMap;

use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use crate::eff::{self, EffChunk};
use crate::sprite::{self, TexInfo};

/// `ccEffectTbl.type`: what `InitEffect` makes of the row's object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 0: a `ccClump` of a `CMP_` object, drawn where the effect is.
    Clump,
    /// 1: a `ccAnm` of an `ANM_` object, played a step a frame.
    Anm,
    /// 2: a `ccEff`, its pattern set by the effect's code.
    Eff,
    /// 3: a `ccEff` stepping one pattern a frame, the effect ending after
    /// the last.
    EffOnce,
    /// 4: ... stepping and wrapping.
    EffLoop,
    /// 5: ... stepping and holding the last.
    EffHold,
    /// 6 (or anything else): no object.
    None,
}

impl Kind {
    pub fn of(t: u32) -> Kind {
        match t {
            0 => Kind::Clump,
            1 => Kind::Anm,
            2 => Kind::Eff,
            3 => Kind::EffOnce,
            4 => Kind::EffLoop,
            5 => Kind::EffHold,
            _ => Kind::None,
        }
    }

    /// Kinds 2-5.
    pub fn is_eff(self) -> bool {
        matches!(self, Kind::Eff | Kind::EffOnce | Kind::EffLoop | Kind::EffHold)
    }
}

/// A row of `effectTbl`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The file's stem (`particle`), empty for none.
    pub ccs: String,
    /// The object's name (`CMP_x032`).
    pub name: String,
    pub kind: Kind,
}

/// An object in one of the effect files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjRef {
    /// Index into [`Assets::files`].
    pub file: usize,
    pub object: u32,
}

/// Everything the effects read from the disc and the executable.
pub struct Assets {
    /// The effect files by `effectCCSTbl` row (stems lower case).
    pub files: Vec<SceneFile>,
    pub morphers: Vec<HashMap<u32, u32>>,
    /// Each file's Eff chunks, and where each EFF_ object's is.
    pub effs: Vec<Vec<EffChunk>>,
    /// Each Eff chunk's texture (`crate::sprite`).
    pub eff_tex: Vec<Vec<Option<TexInfo>>>,
    eff_by_object: HashMap<ObjRef, usize>,
    /// `effectTbl`, `effectTbl2`, `effectStrTbl`.
    pub tbl: Vec<Row>,
    pub tbl2: Vec<Row>,
    pub tbl_str: Vec<Row>,
    /// `alphaBlendTbl`.
    pub alpha_blend: [u64; 8],
    /// The particle system's tables, its objects resolved here.
    pub particle: crate::particle::Tables,
    /// The boss effects' generator rows (GCMN.PRG's).
    pub boss: crate::boss::Tables,
}

fn rows(v: &[piney_data::tables::effect::EffectRow]) -> Vec<Row> {
    v.iter()
        .map(|r| Row {
            ccs: r.ccs.unwrap_or("").to_string(),
            name: r.name.unwrap_or("").to_string(),
            kind: Kind::of(r.kind as u32),
        })
        .collect()
}

impl Assets {
    pub fn read(archive: &Archive, volume: Volume) -> Result<Assets> {
        let t = piney_data::tables::effect::of(volume);
        let stems: Vec<String> =
            t.files().iter().map(|f| f.func.trim_end_matches(".CCS").to_ascii_lowercase()).collect();
        let mut files = Vec::new();
        let mut morphers = Vec::new();
        let mut effs = Vec::new();
        let mut eff_tex = Vec::new();
        let mut eff_by_object = HashMap::new();
        for (k, stem) in stems.iter().enumerate() {
            let f = SceneFile::read(archive, stem)?;
            morphers.push(piney_data::anim::morphers(&f.ccs).unwrap_or_default());
            let e = eff::decode(&f.ccs);
            for (j, c) in e.iter().enumerate() {
                eff_by_object.insert(ObjRef { file: k, object: c.object }, j);
            }
            eff_tex.push(sprite::eff_textures(&f.ccs, &e));
            effs.push(e);
            files.push(f);
        }
        let alpha_blend = crate::spell::first(piney_data::tables::kanji::of(volume).alpha_blend_tbl());
        let mut a = Assets {
            files,
            morphers,
            effs,
            eff_tex,
            eff_by_object,
            tbl: rows(t.rows()),
            tbl2: rows(t.rows2()),
            tbl_str: rows(t.str_rows()),
            alpha_blend,
            particle: Default::default(),
            boss: crate::boss::Tables::read(volume),
        };
        a.particle = crate::particle::Tables::read(volume, &a);
        Ok(a)
    }

    /// Another scene file whose objects are drawn the effects' way (the
    /// magic portal's `XMAGCIR.CCS`, [`crate::portal`]): read once, after
    /// the effect files so their rows keep their places; its index.
    pub fn add_file(&mut self, archive: &Archive, stem: &str) -> Result<usize> {
        if let Some(k) = self.file_index(stem) {
            return Ok(k);
        }
        let k = self.files.len();
        let f = SceneFile::read(archive, stem)?;
        self.morphers.push(piney_data::anim::morphers(&f.ccs).unwrap_or_default());
        let e = eff::decode(&f.ccs);
        for (j, c) in e.iter().enumerate() {
            self.eff_by_object.insert(ObjRef { file: k, object: c.object }, j);
        }
        self.eff_tex.push(crate::sprite::eff_textures(&f.ccs, &e));
        self.effs.push(e);
        self.files.push(f);
        Ok(k)
    }

    /// The file named `stem`.
    pub fn file_index(&self, stem: &str) -> Option<usize> {
        self.files.iter().position(|f| f.stem.eq_ignore_ascii_case(stem))
    }

    /// `GetChunkAdrsF(GetCCSAdrs(ccs), name)`.
    pub fn find(&self, ccs: &str, name: &str) -> Option<ObjRef> {
        let file = self.file_index(ccs)?;
        Some(ObjRef { file, object: self.files[file].ccs.find_object(name)? })
    }

    /// `adrs[id]` as `ccEffectCtrl(0)` resolves it: `effectTbl`'s row in a
    /// field, `effectTbl2`'s in a town (none past its four rows).
    pub fn adrs(&self, id: i16, town: bool) -> Option<ObjRef> {
        let row =
            if town { self.tbl2.get(usize::try_from(id).ok()?)? } else { self.tbl.get(usize::try_from(id).ok()?)? };
        if row.kind == Kind::None || row.ccs.is_empty() {
            return None;
        }
        self.find(&row.ccs, &row.name)
    }

    /// `adrs[id]` as `ccEffectCtrl(1)` resolves it: `effectStrTbl`'s row.
    pub fn adrs_str(&self, id: i16) -> Option<ObjRef> {
        let row = self.tbl_str.get(usize::try_from(id).ok()?)?;
        if row.kind == Kind::None || row.ccs.is_empty() {
            return None;
        }
        self.find(&row.ccs, &row.name)
    }

    /// `effectStrTbl[id].type`, which `MainStr` dispatches on.
    pub fn kind_str(&self, id: i16) -> Kind {
        usize::try_from(id).ok().and_then(|i| self.tbl_str.get(i)).map_or(Kind::None, |r| r.kind)
    }

    /// `effectTbl[id].type`, which `InitEffect` and `Main` dispatch on in
    /// a town too.
    pub fn kind(&self, id: i16) -> Kind {
        usize::try_from(id).ok().and_then(|i| self.tbl.get(i)).map_or(Kind::None, |r| r.kind)
    }

    /// The Eff chunk of an EFF_ object.
    pub fn eff_chunk(&self, o: ObjRef) -> Option<(usize, &EffChunk)> {
        let j = *self.eff_by_object.get(&o)?;
        Some((j, &self.effs[o.file][j]))
    }

    /// The object's name, for reports.
    pub fn name(&self, o: ObjRef) -> &str {
        self.files[o.file].ccs.object_name(o.object).unwrap_or("")
    }

    /// The nodes of a CMP_ object.
    pub fn clump_nodes(&self, o: ObjRef) -> Result<&[u32]> {
        self.files[o.file]
            .scene
            .clumps
            .iter()
            .find(|(c, _)| *c == o.object)
            .map(|(_, n)| n.as_slice())
            .ok_or_else(|| Error::NotFound(self.name(o).into()))
    }
}
