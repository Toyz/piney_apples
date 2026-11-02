//! A map's placed scenery: `STATICMODEL`s, the rows of its model table each
//! at its dummy, and `STATICOBJECT`s, the rows of its object table whose
//! animation steps while the eye is within its clip (docs/engine/statics.md).

use std::borrow::Cow;
use std::rc::Rc;

use glam::Mat4;
use piney_data::statics::{self, DrawPass, ObjTable};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::bob::Bob;
use super::{BuildCx, DrawCx, Fog, Hook, Kind, MapSystem, Others, Piece, PlaceCx, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw;
use crate::ee::{self, F, ONE, V4};
use crate::hit::UNIT;
use crate::town::{dummy_bits, pos_rot_zyx};

/// The layer each draw pass's rows go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassLayers {
    pub other: i16,
    pub floor: i16,
    pub obj: i16,
    pub obj2: i16,
}

impl PassLayers {
    /// Every pass on `layer`.
    pub const fn all(layer: i16) -> PassLayers {
        PassLayers { other: layer, floor: layer, obj: layer, obj2: layer }
    }

    pub fn of(&self, pass: DrawPass) -> i16 {
        match pass {
            DrawPass::Other => self.other,
            DrawPass::Floor => self.floor,
            DrawPass::Obj => self.obj,
            DrawPass::Obj2 => self.obj2,
        }
    }
}

/// A model table as a class's constructor builds it: one `STATICMODEL` a
/// row, its model `GetChunkAdrsF(name, 0)` (no such chunk stores to
/// address 0: a model the file lacks stops the game, an error here) at its
/// row's dummy (postype 1 or 2; the origin for postype 0); the layer of
/// each pass's rows, whether they take the area's fog, the rows `DrawObj2`
/// draws with `DrawWithOutFog`, and those `ChangeBlock` puts at the bob.
#[derive(Clone, Copy, Debug)]
pub struct ModelsDef {
    /// `RT_MODELTABLE00`, `EA_MODELTABLE02` ..: its symbol.
    pub table: &'static str,
    pub layers: PassLayers,
    pub fog: Fog,
    pub without_fog: &'static [usize],
    pub ride: &'static [usize],
}

impl ModelsDef {
    /// `table` on `layer`, with the area's fog.
    pub const fn on(table: &'static str, layer: i16) -> ModelsDef {
        ModelsDef { table, layers: PassLayers::all(layer), fog: Fog::Area, without_fog: &[], ride: &[] }
    }
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

/// The `STATICMODEL`s of a table, by row, over their file.
#[derive(Clone)]
pub struct StaticModels {
    def: &'static ModelsDef,
    file: Rc<SceneFile>,
    models: Vec<Model>,
}

impl StaticModels {
    /// `def`'s rows built over the file, from the tables' table (each name
    /// as the file has it).
    pub fn new(def: &'static ModelsDef, cx: &mut BuildCx) -> Result<StaticModels> {
        let file = cx.file;
        let table = statics::of(cx.tables).0.iter().find(|t| t.name == def.table);
        let table = table.ok_or_else(|| Error::NotFound(def.table.into()))?;
        let (c, sc) = (&file.ccs, &file.scene);
        let model = |(row, r): (usize, &statics::StaticModel)| {
            let name = cx.name(r.model);
            let model = c.find_object(&name).ok_or_else(|| Error::NotFound(format!("{}: {name}", file.stem)))?;
            let pos = dummy_bits(&r.position, c, sc).0;
            Ok(Model { row, pass: r.pass, model, pos, clip: r.clip.to_bits() })
        };
        let models = table.rows.iter().enumerate().map(model).collect::<Result<_>>()?;
        Ok(StaticModels { def, file: file.clone(), models })
    }

    /// The file its names are in.
    pub fn file(&self) -> &SceneFile {
        &self.file
    }

    /// A row's model.
    pub fn model(&self, row: usize) -> Option<&Model> {
        self.models.get(row)
    }

    pub fn model_mut(&mut self, row: usize) -> Option<&mut Model> {
        self.models.get_mut(row)
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

    /// A row as `STATICMODEL::Draw` draws it on `layer`, with `fog`, its
    /// materials offset by `cx`'s scrolls: a unit matrix moved to the
    /// dummy, its turn left out.
    pub fn request<'a>(&'a self, row: usize, layer: i16, fog: draw::Fogging, cx: &DrawCx) -> Option<Request<'a>> {
        let m = self.models.get(row)?;
        let world = Mat4::from_translation(glam::Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
        let models = Cow::Borrowed(std::slice::from_ref(&m.model));
        Some(Request::Models { file: &self.file, models, world, layer, rows: cx.rows.clone(), fog, alpha: 1.0 })
    }

    /// The fog a row takes: the def's, or none.
    fn fog(&self, fog: Fog, cx: &DrawCx) -> draw::Fogging {
        match (self.def.fog, fog) {
            (Fog::Area, Fog::Area) => cx.fog,
            _ => draw::Fogging::None,
        }
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
    /// A pass's rows (those `DrawWithOutFog` draws only within their clip
    /// of the player, `w2p` over the story maps' bounds, without fog), rows
    /// by number, a pass's rows below one, or every row as it stands.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let piece = |row: usize, fog: Fog| match fog {
            Fog::Area => Piece::Model(row),
            Fog::None => Piece::ModelNoFog(row),
        };
        match hook {
            Hook::Pass(pass) => {
                for m in self.models.iter().filter(|m| m.pass == pass) {
                    if pass != DrawPass::Obj2 || !self.def.without_fog.contains(&m.row) {
                        cx.put(Kind::Models, Piece::Model(m.row));
                        continue;
                    }
                    let d = crate::field::area::w2p(m.pos, cx.view.player, crate::story_map::BOUNDS);
                    let d = ee::ground_dist(cx.core.hits.volume, d[0], d[1]);
                    if ee::lt(d, m.clip) || ee::eq(0, m.clip) {
                        cx.put(Kind::Models, Piece::ModelNoFog(m.row));
                    }
                }
            }
            Hook::Rows(from, to, fog) => (from..to).for_each(|row| cx.put(Kind::Models, piece(row, fog))),
            Hook::PassBelow(pass, below, fog) => {
                self.rows(pass).filter(|&row| row < below).for_each(|row| cx.put(Kind::Models, piece(row, fog)))
            }
            Hook::Still(layer) => {
                (0..self.models.len()).for_each(|row| cx.put(Kind::Models, Piece::ModelOn(row, layer)))
            }
            _ => {}
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let layer = |row: usize| self.models.get(row).map(|m| self.def.layers.of(m.pass));
        match piece {
            Piece::Model(row) => self.request(row, layer(row)?, self.fog(Fog::Area, cx), cx),
            Piece::ModelNoFog(row) => self.request(row, layer(row)?, draw::Fogging::None, cx),
            Piece::ModelOn(row, layer) => self.request(row, layer, self.fog(Fog::Area, cx), cx),
            _ => None,
        }
    }

    /// `STATICMODEL` enables its model's Hit chunk, in the order the rows
    /// were built; `ChangeBlock` puts the riders at the bob
    /// (`STATICMODEL::SetPos`: the hit's matrix the translation, and its
    /// inverse).
    fn place(&mut self, cx: &mut PlaceCx, others: &mut Others) {
        let bob = others.get::<Bob>().map(Bob::z);
        for (m, &row) in self.def.ride.iter().filter_map(|r| Some((bob?, r))) {
            if let Some(x) = self.models.get_mut(row) {
                x.pos = [0, 0, m, 0];
            }
        }
        for m in &self.models {
            let Some(mut h) = cx.hit_models.iter().find(|h| h.parent == m.model).cloned() else { continue };
            if self.def.ride.contains(&m.row) && m.pos[..3] != [0; 3] {
                h.rm = UNIT;
                h.rm[3] = [m.pos[0], m.pos[1], m.pos[2], ONE];
                h.im = UNIT;
                h.im[3] = [m.pos[0] ^ 0x8000_0000, m.pos[1] ^ 0x8000_0000, m.pos[2] ^ 0x8000_0000, ONE];
            }
            cx.core.hits.models.push(h);
        }
    }
}

/// An object table as a class's constructor builds it: a `STATICOBJECT`
/// for each of its first `anm_num` rows (the class's `anmNum`; None, every
/// row), with a `ccAnm` where the row names one, at its dummy; the layer of
/// each pass's objects, and those `ChangeBlock` puts at the bob.
#[derive(Clone, Copy, Debug)]
pub struct ObjectsDef {
    /// `RT_OBJTABLE00`, `EA_OBJTABLE02` ..: its symbol.
    pub table: &'static str,
    pub anm_num: Option<usize>,
    pub layers: PassLayers,
    pub ride: &'static [usize],
}

impl ObjectsDef {
    /// Every row of `table`, on `layer`.
    pub const fn on(table: &'static str, layer: i16) -> ObjectsDef {
        ObjectsDef { table, anm_num: None, layers: PassLayers::all(layer), ride: &[] }
    }
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

/// The `STATICOBJECT`s of a table over their file: those whose row names
/// an animation.
#[derive(Clone)]
pub struct StaticObjects {
    def: &'static ObjectsDef,
    file: Rc<SceneFile>,
    objects: Vec<Object>,
}

impl StaticObjects {
    /// `def`'s objects in the file, from the tables' table.
    pub fn new(def: &'static ObjectsDef, cx: &mut BuildCx) -> Result<StaticObjects> {
        let file = cx.file;
        let table: Option<&ObjTable> = statics::of(cx.tables).1.iter().find(|t| t.name == def.table);
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
        Ok(StaticObjects { def, file: file.clone(), objects })
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
    pub fn step_pass(&mut self, pass: DrawPass, eye: V4, volume: Volume) -> Vec<usize> {
        let mut out = Vec::new();
        for o in self.objects.iter_mut().filter(|o| o.pass == pass) {
            let (dx, dy) = (ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
            let d = ee::ground_dist(volume, dx, dy);
            if ee::lt(d, o.clip) || ee::eq(o.clip, 0) {
                o.play.forward(&self.file);
                out.push(o.row);
            }
        }
        out
    }

    /// An object as drawn on `layer` with `fog`.
    pub fn request(&self, row: usize, layer: i16, fog: draw::Fogging) -> Option<Request<'_>> {
        let o = self.objects.iter().find(|o| o.row == row)?;
        let root = draw::mat(&o.root);
        Some(Request::Anim { file: &self.file, play: &o.play, root, layer, rows: None, fog, edit: None })
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
        match hook {
            Hook::Pass(pass) => {
                for row in self.step_pass(pass, cx.view.eye, cx.core.hits.volume) {
                    cx.put(Kind::Objects, Piece::Object(row));
                }
            }
            Hook::Still(layer) => {
                self.objects.iter().for_each(|o| cx.put(Kind::Objects, Piece::ObjectOn(o.row, layer)));
            }
            _ => {}
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        match piece {
            Piece::Object(row) => {
                let pass = self.objects.iter().find(|o| o.row == row)?.pass;
                self.request(row, self.def.layers.of(pass), cx.fog)
            }
            Piece::ObjectOn(row, layer) => self.request(row, layer, cx.fog),
            _ => None,
        }
    }

    /// `ChangeBlock` puts the riders at the bob: their place and root.
    fn place(&mut self, _: &mut PlaceCx, others: &mut Others) {
        let Some(z) = others.get::<Bob>().map(Bob::z) else { return };
        for &k in self.def.ride {
            if let Some(o) = self.objects.get_mut(k) {
                o.pos = [0, 0, z, 0];
                o.root = pos_rot_zyx(o.pos, [0; 3]);
            }
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
