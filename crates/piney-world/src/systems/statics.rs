//! A map's placed scenery: `STATICMODEL`s, the rows of its model table each
//! at its dummy, and `STATICOBJECT`s, the rows of its object table whose
//! animation steps while the eye is within its clip (docs/engine/statics.md).

use std::collections::HashMap;

use glam::Mat4;
use piney_data::statics::{self, DrawPass, ModelTable, ObjTable, PlacedModel};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Fog, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw::{self, OBJ_LAYER};
use crate::ee::{self, F, V4};
use crate::town::{dummy_bits, pos_rot_zyx};

/// A model table and the layer its rows draw on.
#[derive(Clone, Copy, Debug)]
pub struct ModelsDef {
    /// `RT_MODELTABLE00` ..: its symbol.
    pub table: &'static str,
    pub layer: i16,
}

/// The `STATICMODEL`s of a table: by row, where each draws.
#[derive(Clone, Debug)]
pub struct StaticModels {
    def: &'static ModelsDef,
    table: &'static ModelTable,
    models: Vec<Option<PlacedModel>>,
}

impl StaticModels {
    pub fn new(def: &'static ModelsDef, cx: &mut BuildCx) -> Result<StaticModels> {
        StaticModels::read(def, cx.file, cx.tables)
    }

    /// `def`'s rows placed in `file`, from `tables`' table.
    pub fn read(def: &'static ModelsDef, file: &SceneFile, tables: Volume) -> Result<StaticModels> {
        let table = statics::of(tables).0.iter().find(|t| t.name == def.table);
        let table = table.ok_or_else(|| Error::NotFound(def.table.into()))?;
        let mut models = vec![None; table.rows.len()];
        for p in table.place(&file.ccs, &file.scene) {
            models[p.row] = Some(p);
        }
        Ok(StaticModels { def, table, models })
    }

    /// A row's placement.
    pub fn model(&self, row: usize) -> Option<PlacedModel> {
        self.models.get(row).copied().flatten()
    }

    /// Every placed row, in order.
    pub fn placed(&self) -> impl Iterator<Item = &PlacedModel> {
        self.models.iter().flatten()
    }

    /// The table's rows of `pass`, in order.
    pub fn rows(&self, pass: DrawPass) -> impl Iterator<Item = usize> + use<> {
        let rows = self.table.rows;
        (0..rows.len()).filter(move |&row| rows[row].pass == pass)
    }

    /// A row as `STATICMODEL::Draw` draws it on `layer`, with `fog`.
    pub fn request<'a>(
        &'a self,
        file: &'a SceneFile,
        row: usize,
        layer: i16,
        fog: draw::Fogging,
    ) -> Option<Request<'a>> {
        let p = self.models.get(row)?.as_ref()?;
        let world = Mat4::from_translation(p.pos);
        Some(Request::Models { file, models: std::slice::from_ref(&p.model), world, layer, rows: HashMap::new(), fog })
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

/// An object table.
#[derive(Clone, Copy, Debug)]
pub struct ObjectsDef {
    /// `RT_OBJTABLE00` ..: its symbol.
    pub table: &'static str,
}

/// One `STATICOBJECT`.
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

/// The `STATICOBJECT`s of a table: those whose row names an animation.
#[derive(Clone, Debug)]
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
        for (row, r) in table.rows.iter().enumerate() {
            // STATICOBJECT makes only the ccAnm when the row names one.
            let Some(play) = r.anime.and_then(|n| Play::new(file, n)) else { continue };
            let (pos, rot) = dummy_bits(&r.position, c, sc);
            let root = pos_rot_zyx(pos, rot);
            objects.push(Object { row, pass: r.pass, play, root, pos, clip: r.clip.to_bits() });
        }
        Ok(StaticObjects { objects })
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
