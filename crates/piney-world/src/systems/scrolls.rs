//! The scrolls a map's `Draw` steps each frame and writes into its
//! materials' texture offsets: a town's `DrawBG` statics, a story map's
//! `EA_moveTex` rows (`MoveTexture`), the arena's, the disc's, a field's
//! (with its turns) and a lake's `DrawBG` statics. Each lane is stepped and
//! wrapped ([`Wrap`]); the materials take `vftoi12` of a lane less their
//! crop (docs/engine/root-towns.md, docs/engine/evarea.md,
//! docs/engine/field.md).

use std::collections::HashMap;

use piney_data::Result;
use piney_desktop::assets::SceneFile;

use super::{BuildCx, Carried, CarriedMut, Hook, MapSystem, Others, StepCx};
use crate::ee::{self, F, ONE};

/// How a scroll comes back once a step takes it past its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// Above 1, back to 0 (`DrawBG`'s scrolls, `MoveTexture`).
    Past,
    /// At 1 or above, back to 0 (Dun Loireag's and Carmina Gadelica's water).
    AtOrPast,
    /// Above 1, less 1 (Mac Anu's water, the arena's sky).
    Less,
    /// Down by the step; below 0, back to 1 (the disc's sky).
    Under,
    /// Down by the step; below 0, on by 1 (a hacked field's sky).
    Below,
}

impl Wrap {
    /// `u` stepped by `step`, wrapped.
    pub fn step(self, u: F, step: F) -> F {
        match self {
            Wrap::Under => {
                let u = ee::sub(u, step);
                return if ee::lt(u, 0) { ONE } else { u };
            }
            Wrap::Below => {
                let u = ee::sub(u, step);
                return if ee::lt(u, 0) { ee::add(u, ONE) } else { u };
            }
            _ => {}
        }
        let u = ee::add(u, step);
        match self {
            Wrap::Past if !ee::le(u, ONE) => 0,
            Wrap::AtOrPast if !ee::lt(u, ONE) => 0,
            Wrap::Less if !ee::le(u, ONE) => ee::sub(u, ONE),
            _ => u,
        }
    }
}

/// Whether a material takes its lanes as the step leaves them or as it
/// found them (`EVENTAREAB0::DrawBG` and `WORLD::DrawBG` write the sky's,
/// then step).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    After,
    Before,
}

/// Where the lanes live between visits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carry {
    /// Statics of the executable, from this slot on.
    Statics(usize),
    /// gcmn's `EA_moveTex` rows from this one on, a row's (u, v) two lanes.
    MoveTex(usize),
}

/// Which materials a [`MatScroll`] offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mat {
    /// The material of that name.
    Named(&'static str),
    /// `WORLD::DrawBG`'s: the background row's sky material (`BgMatName`,
    /// `BgMatName2` hacked).
    Sky,
    /// `WORLD::DrawBG`'s `ccModel::SetUV(u, 0, 0, 2)`: the materials of the
    /// aurora clump's first object's model.
    Aurora,
}

/// A material the scrolls offset: its U by one lane, its V by one (None
/// leaves that offset 0), and when it takes them.
#[derive(Clone, Copy, Debug)]
pub struct MatScroll {
    pub mat: Mat,
    pub u: Option<usize>,
    pub v: Option<usize>,
    pub shown: Shown,
}

impl MatScroll {
    /// U by lane `lane`.
    pub const fn u(mat: &'static str, lane: usize) -> MatScroll {
        MatScroll { mat: Mat::Named(mat), u: Some(lane), v: None, shown: Shown::After }
    }

    /// V by lane `lane`.
    pub const fn v(mat: &'static str, lane: usize) -> MatScroll {
        MatScroll { mat: Mat::Named(mat), u: None, v: Some(lane), shown: Shown::After }
    }

    /// U and V by the same lane.
    pub const fn uv(mat: &'static str, lane: usize) -> MatScroll {
        MatScroll { mat: Mat::Named(mat), u: Some(lane), v: Some(lane), shown: Shown::After }
    }

    /// `MOVETEX` row `row` of a table: U by its lane `2 row`, V by `2 row + 1`.
    pub const fn move_tex(mat: &'static str, row: usize) -> MatScroll {
        MatScroll { mat: Mat::Named(mat), u: Some(2 * row), v: Some(2 * row + 1), shown: Shown::After }
    }

    /// Taking the lanes as the step found them.
    pub const fn before(self) -> MatScroll {
        MatScroll { shown: Shown::Before, ..self }
    }
}

/// `WORLD::DrawBG`'s turns of its layers (`rot2$4526.z`, `rot3$4527.z`,
/// statics of the executable from `slot`): the first up by `step`, past
/// `max` to `-max`, the second down by it, below `-max` to `max`. Only the
/// hacked drawing steps them (`stepped`); the sky's clumps turn by them
/// either way.
#[derive(Clone, Copy, Debug)]
pub struct Turns {
    pub step: F,
    pub max: F,
    pub slot: usize,
    pub stepped: bool,
}

/// A static with a first-use guard: the first step sets the lanes to
/// `value` and the flag, a static of its own at `flag`.
#[derive(Clone, Copy, Debug)]
pub struct First {
    pub value: F,
    pub flag: usize,
}

/// A map's scrolls: each lane's step a frame, how it wraps, where it lives
/// and what its first use sets, the materials, and a field's turns.
#[derive(Clone, Copy, Debug)]
pub struct ScrollsDef {
    pub steps: &'static [F],
    pub wrap: Wrap,
    pub carry: Option<Carry>,
    pub first: Option<First>,
    pub mats: &'static [MatScroll],
    pub turns: Option<Turns>,
}

impl ScrollsDef {
    /// A town's `DrawBG`: its statics from `slot`, each above 1 back to 0.
    pub const fn statics(steps: &'static [F], slot: usize, mats: &'static [MatScroll]) -> ScrollsDef {
        let carry = Some(Carry::Statics(slot));
        ScrollsDef { steps, wrap: Wrap::Past, carry, first: None, mats, turns: None }
    }

    /// A story map's `EA_moveTex` table, gcmn's rows from `row`: each
    /// `MOVETEX`'s (u_step, v_step) in `steps`.
    pub const fn move_tex(steps: &'static [F], row: usize, mats: &'static [MatScroll]) -> ScrollsDef {
        let carry = Some(Carry::MoveTex(row));
        ScrollsDef { steps, wrap: Wrap::Past, carry, first: None, mats, turns: None }
    }
}

/// The scrolls as the last step left them.
#[derive(Clone, Debug)]
pub struct Scrolls {
    def: &'static ScrollsDef,
    pub v: Vec<F>,
    /// The lanes as the last step found them.
    pub before: Vec<F>,
    /// Its first use made (the def's `first` flag).
    pub used: bool,
    /// The turns (`rot2.z`, `rot3.z`), 0 without.
    pub turn: [F; 2],
    /// Each material's names as the file has them.
    mats: Vec<Vec<String>>,
}

impl Scrolls {
    pub fn new(def: &'static ScrollsDef, cx: &mut BuildCx) -> Result<Scrolls> {
        let v = vec![0; def.steps.len()];
        let mats = def.mats.iter().map(|m| mat_names(m.mat, cx)).collect();
        Ok(Scrolls { def, before: v.clone(), v, used: false, turn: [0; 2], mats })
    }

    /// `vftoi12` of lane `k` as the last step left it.
    pub fn uv(&self, k: usize) -> u16 {
        ee::vftoi12(self.v[k])
    }
}

/// The names `mat` stands for in the file: its own, or a field's sky's or
/// aurora's (none outside a field, or where the row has none).
fn mat_names(mat: Mat, cx: &BuildCx) -> Vec<String> {
    let background = || cx.field.and_then(|f| f.field).map(|f| &f.scene.background);
    match mat {
        Mat::Named(name) => vec![cx.name(name).into_owned()],
        Mat::Sky => background().and_then(|b| b.sky_material).map(String::from).into_iter().collect(),
        Mat::Aurora => {
            let file = &**cx.file;
            let first = background()
                .and_then(|b| b.aurora)
                .and_then(|a| file.ccs.find_object(a))
                .and_then(|o| file.scene.clumps.iter().find(|(c, _)| *c == o))
                .and_then(|(_, nodes)| nodes.first().copied());
            let Some(first) = first else { return Vec::new() };
            let mut mats: Vec<u32> = (file.scene.model_owner.iter())
                .filter(|(_, o)| **o == first)
                .filter_map(|(m, _)| file.models.get(m))
                .flat_map(|i| i.mmats.iter().filter_map(|m| m.material))
                .collect();
            mats.sort_unstable();
            mats.dedup();
            mats.iter().filter_map(|&m| file.ccs.object_name(m)).map(String::from).collect()
        }
    }
}

impl MapSystem for Scrolls {
    fn step(&mut self, hook: Hook, _: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        if let Some(t) = self.def.turns.filter(|t| t.stepped) {
            let [r2, r3] = &mut self.turn;
            *r2 = ee::add(*r2, t.step);
            if !ee::le(*r2, t.max) {
                *r2 = ee::neg(t.max);
            }
            *r3 = ee::sub(*r3, t.step);
            if ee::lt(*r3, ee::neg(t.max)) {
                *r3 = t.max;
            }
        }
        if let Some(f) = self.def.first.filter(|_| !self.used) {
            self.v.fill(f.value);
            self.used = true;
        }
        self.before.clone_from(&self.v);
        for (x, &step) in self.v.iter_mut().zip(self.def.steps) {
            *x = self.def.wrap.step(*x, step);
        }
    }

    fn take(&mut self, c: &Carried) {
        match (self.def.carry, c.move_tex) {
            (Some(Carry::Statics(at)), _) => {
                let n = self.v.len();
                self.v.copy_from_slice(&c.statics[at..at + n]);
            }
            (Some(Carry::MoveTex(at)), Some(rows)) => {
                for (k, x) in self.v.iter_mut().enumerate() {
                    *x = rows.get(at + k / 2).map_or(0, |r| r[k % 2]);
                }
            }
            _ => return,
        }
        if let Some(f) = self.def.first {
            self.used = c.statics[f.flag] != 0;
        }
        if let Some(t) = self.def.turns {
            self.turn.copy_from_slice(&c.statics[t.slot..t.slot + 2]);
        }
        self.before.clone_from(&self.v);
    }

    fn put(&self, c: &mut CarriedMut) {
        match (self.def.carry, c.move_tex.as_deref_mut()) {
            (Some(Carry::Statics(at)), _) => c.statics[at..at + self.v.len()].copy_from_slice(&self.v),
            (Some(Carry::MoveTex(at)), Some(rows)) => {
                for (k, &x) in self.v.iter().enumerate() {
                    rows[at + k / 2][k % 2] = x;
                }
            }
            _ => {}
        }
        if let Some(f) = self.def.first {
            c.statics[f.flag] = u32::from(self.used);
        }
        if let Some(t) = self.def.turns {
            c.statics[t.slot..t.slot + 2].copy_from_slice(&self.turn);
        }
    }

    /// Each material's offsets: `vftoi12` of its lanes, as it takes them,
    /// less its crop.
    fn rows(&self, file: &SceneFile, out: &mut HashMap<u32, [u8; 2]>) {
        for (m, names) in self.def.mats.iter().zip(&self.mats) {
            let lanes = if m.shown == Shown::Before { &self.before } else { &self.v };
            let uv = |k: usize| ee::vftoi12(lanes[k]);
            for name in names {
                let Some(mat) = file.ccs.find_object(name) else { continue };
                let Some(info) = file.scene.materials.get(&mat) else { continue };
                let off = |lane: Option<usize>, crop: u16| lane.map_or(0, |k| uv(k).wrapping_sub(crop));
                out.insert(mat, [(off(m.u, info.crop_u) >> 4) as u8, (off(m.v, info.crop_v) >> 4) as u8]);
            }
        }
    }
}
