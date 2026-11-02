//! A map's placed scenery: `STATICMODEL`s, the rows of its model table each
//! at its dummy, and `STATICOBJECT`s, the rows of its object table whose
//! animation steps while the eye is within its clip (docs/engine/statics.md).

use std::collections::HashMap;

use glam::Mat4;
use piney_data::statics::{self, DrawPass, ObjTable};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Fog, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw::{self, OBJ_LAYER};
use crate::ee::{self, F, V4};
use crate::town::{dummy_bits, pos_rot_zyx};

/// A model table as a class's constructor builds it: one `STATICMODEL` a
/// row, its model `GetChunkAdrsF(name, 0)` (no such chunk stores to
/// address 0: a model the file lacks stops the game, an error here) at its
/// row's dummy (postype 1 or 2; the origin for postype 0).
#[derive(Clone, Copy, Debug)]
pub struct ModelsDef {
    /// `RT_MODELTABLE00`, `EA_MODELTABLE02` ..: its symbol.
    pub table: &'static str,
    pub layer: i16,
    pub names: Names,
}

/// What a row's model name names in the class's file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Names {
    /// The name as the table writes it.
    Table,
    /// The table's names are `from`'s scene's; the class reads them as its
    /// own file's (`EVENTAREAB0`'s stages: `se1_5`'s names in each).
    OfFile { from: &'static str },
}

/// A `STATICMODEL` (0x40 bytes): `type` +0x00, its model +0x04, the
/// dummy's position +0x10 (zero without one), `clip` +0x30.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    /// Its table row.
    pub row: usize,
    pub pass: DrawPass,
    /// The MDL_ object.
    pub model: u32,
    pub pos: V4,
    pub clip: F,
}

/// The `STATICMODEL`s of a table, by row.
#[derive(Clone, Debug)]
pub struct StaticModels {
    def: &'static ModelsDef,
    models: Vec<Model>,
}

impl StaticModels {
    pub fn new(def: &'static ModelsDef, cx: &mut BuildCx) -> Result<StaticModels> {
        StaticModels::read(def, cx.file, cx.tables)
    }

    /// `def`'s rows built over `file`, from `tables`' table.
    pub fn read(def: &'static ModelsDef, file: &SceneFile, tables: Volume) -> Result<StaticModels> {
        let table = statics::of(tables).0.iter().find(|t| t.name == def.table);
        let table = table.ok_or_else(|| Error::NotFound(def.table.into()))?;
        let (c, sc) = (&file.ccs, &file.scene);
        let model = |(row, r): (usize, &statics::StaticModel)| {
            let name = match def.names {
                Names::Table => r.model.to_string(),
                Names::OfFile { from } => r.model.replacen(from, &file.stem, 1),
            };
            let model = c.find_object(&name).ok_or_else(|| Error::NotFound(format!("{}: {name}", file.stem)))?;
            let pos = dummy_bits(&r.position, c, sc).0;
            Ok(Model { row, pass: r.pass, model, pos, clip: r.clip.to_bits() })
        };
        let models = table.rows.iter().enumerate().map(model).collect::<Result<_>>()?;
        Ok(StaticModels { def, models })
    }

    /// A row's model.
    pub fn model(&self, row: usize) -> Option<&Model> {
        self.models.get(row)
    }

    pub fn model_mut(&mut self, row: usize) -> Option<&mut Model> {
        self.models.get_mut(row)
    }

    /// The class's models deleted (`EVENTAREAB0::NextStage`).
    pub fn clear(&mut self) {
        self.models.clear();
    }

    /// Every row, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, Model> {
        self.models.iter()
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// The table's rows of `pass`, in order.
    pub fn rows(&self, pass: DrawPass) -> impl Iterator<Item = usize> + use<'_> {
        self.models.iter().filter(move |m| m.pass == pass).map(|m| m.row)
    }

    /// A row as `STATICMODEL::Draw` draws it on `layer`, with `fog`: a
    /// unit matrix moved to the dummy, its turn left out.
    pub fn request<'a>(
        &'a self,
        file: &'a SceneFile,
        row: usize,
        layer: i16,
        fog: draw::Fogging,
    ) -> Option<Request<'a>> {
        let m = self.models.get(row)?;
        let world = Mat4::from_translation(glam::Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
        Some(Request::Models { file, models: std::slice::from_ref(&m.model), world, layer, rows: HashMap::new(), fog })
    }
}

impl std::ops::Index<usize> for StaticModels {
    type Output = Model;

    fn index(&self, k: usize) -> &Model {
        &self.models[k]
    }
}

impl std::ops::IndexMut<usize> for StaticModels {
    fn index_mut(&mut self, k: usize) -> &mut Model {
        &mut self.models[k]
    }
}

impl MapSystem for StaticModels {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let piece = |row: usize, fog: Fog| match fog {
            Fog::Area => Piece::Model(row),
            Fog::None => Piece::ModelNoFog(row),
        };
        match hook {
            Hook::Pass(pass) => self.rows(pass).for_each(|row| cx.put(Kind::Models, Piece::Model(row))),
            Hook::Rows(from, to, fog) => (from..to).for_each(|row| cx.put(Kind::Models, piece(row, fog))),
            Hook::PassBelow(pass, below, fog) => {
                self.rows(pass).filter(|&row| row < below).for_each(|row| cx.put(Kind::Models, piece(row, fog)))
            }
            _ => {}
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        match piece {
            Piece::Model(row) => self.request(&cx.core.file, row, self.def.layer, cx.fog),
            Piece::ModelNoFog(row) => self.request(&cx.core.file, row, self.def.layer, draw::Fogging::None),
            _ => None,
        }
    }
}

/// An object table as a class's constructor builds it: a `STATICOBJECT`
/// for each of its first `anm_num` rows (the class's `anmNum`; None, every
/// row), with a `ccAnm` where the row names one, at its dummy.
#[derive(Clone, Copy, Debug)]
pub struct ObjectsDef {
    /// `RT_OBJTABLE00`, `EA_OBJTABLE02` ..: its symbol.
    pub table: &'static str,
    pub anm_num: Option<usize>,
}

/// One `STATICOBJECT` of an animation: its `ccAnm`, the root
/// `SetMatrix_PosRotZYX(pos, rot)` gave it, the dummy's position (+0x10)
/// and `clip` (+0x30).
#[derive(Clone, Debug)]
pub struct Object {
    /// Its table row.
    pub row: usize,
    pub pass: DrawPass,
    pub play: Play,
    /// `ccCoord::SetMatrix_PosRotZYX(pos, rot)` of its dummy, as stored.
    pub root: [V4; 4],
    /// +0x10: the dummy's position (zero without one).
    pub pos: V4,
    pub clip: F,
}

/// The `STATICOBJECT`s of a table: those whose row names an animation (by
/// default none, for a class that builds none).
#[derive(Clone, Debug, Default)]
pub struct StaticObjects {
    objects: Vec<Object>,
}

impl StaticObjects {
    pub fn new(def: &'static ObjectsDef, cx: &mut BuildCx) -> Result<StaticObjects> {
        StaticObjects::read(def, cx.file, cx.tables)
    }

    /// `def`'s objects in `file`, from `tables`' table.
    pub fn read(def: &ObjectsDef, file: &SceneFile, tables: Volume) -> Result<StaticObjects> {
        let table: Option<&ObjTable> = statics::of(tables).1.iter().find(|t| t.name == def.table);
        let table = table.ok_or_else(|| Error::NotFound(def.table.into()))?;
        let (c, sc) = (&file.ccs, &file.scene);
        let mut objects = Vec::new();
        for (row, r) in table.rows.iter().enumerate().take(def.anm_num.unwrap_or(usize::MAX)) {
            // STATICOBJECT makes the ccAnm only when the row names one.
            let Some(name) = r.anime else { continue };
            let play = Play::new(file, name).ok_or_else(|| Error::NotFound(format!("{}: {name}", file.stem)))?;
            let (pos, rot) = dummy_bits(&r.position, c, sc);
            let root = pos_rot_zyx(pos, rot);
            objects.push(Object { row, pass: r.pass, play, root, pos, clip: r.clip.to_bits() });
        }
        Ok(StaticObjects { objects })
    }

    /// Every object, in row order.
    pub fn iter(&self) -> std::slice::Iter<'_, Object> {
        self.objects.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Object> {
        self.objects.iter_mut()
    }

    pub fn get(&self, k: usize) -> Option<&Object> {
        self.objects.get(k)
    }

    pub fn get_mut(&mut self, k: usize) -> Option<&mut Object> {
        self.objects.get_mut(k)
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// An object's animation time and root matrix, by its row.
    pub fn object(&self, row: usize) -> Option<(u32, [V4; 4])> {
        self.objects.iter().find(|o| o.row == row).map(|o| (o.play.time, o.root))
    }

    /// `STATICOBJECT::Draw` (0x005cfe70) up to the draw for each object of
    /// `pass`: within its clip of the eye (`sceVu0SubVector`, x x + y y,
    /// `sqrt` in double; `sqrt.s` from Outbreak on) or with a clip of 0, the
    /// animation steps. The rows of those to be drawn, in order.
    pub fn step_pass(&mut self, pass: DrawPass, eye: V4, volume: Volume, file: &SceneFile) -> Vec<usize> {
        let mut out = Vec::new();
        for o in self.objects.iter_mut().filter(|o| o.pass == pass) {
            let (dx, dy) = (ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
            let d = ee::ground_dist(volume, dx, dy);
            if ee::lt(d, o.clip) || ee::eq(o.clip, 0) {
                o.play.forward(file);
                out.push(o.row);
            }
        }
        out
    }

    /// An object as drawn on `layer` with `fog`.
    pub fn request<'a>(
        &'a self,
        file: &'a SceneFile,
        row: usize,
        layer: i16,
        fog: draw::Fogging,
    ) -> Option<Request<'a>> {
        let o = self.objects.iter().find(|o| o.row == row)?;
        let root = draw::mat(&o.root);
        Some(Request::Anim { file, play: &o.play, root, layer, rows: None, fog, edit: None })
    }
}

impl std::ops::Index<usize> for StaticObjects {
    type Output = Object;

    fn index(&self, k: usize) -> &Object {
        &self.objects[k]
    }
}

impl std::ops::IndexMut<usize> for StaticObjects {
    fn index_mut(&mut self, k: usize) -> &mut Object {
        &mut self.objects[k]
    }
}

impl MapSystem for StaticObjects {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if let Hook::Pass(pass) = hook {
            let (eye, volume) = (cx.view.eye, cx.core.hits.volume);
            for row in self.step_pass(pass, eye, volume, &cx.core.file) {
                cx.put(Kind::Objects, Piece::Object(row));
            }
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        match piece {
            Piece::Object(row) => self.request(&cx.core.file, row, OBJ_LAYER, cx.fog),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use piney_data::statics;
    use piney_data::volume::Volume;

    /// `STATICOBJECT` makes a clump for a row naming no animation; no class
    /// builds such a row (`EA_OBJTABLE02`'s placeholder is past its
    /// `anmNum` 0), so every object the loader skips is one no class makes.
    #[test]
    fn every_built_object_row_names_an_animation() {
        for v in Volume::ALL {
            for t in statics::of(v).1 {
                for (row, r) in t.rows.iter().enumerate() {
                    let placeholder = t.name == "EA_OBJTABLE02" && row == 0;
                    assert!(r.anime.is_some() || placeholder, "{v:?} {} row {row}: {:?}", t.name, r.clump);
                }
            }
        }
    }
}
