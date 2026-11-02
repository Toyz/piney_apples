//! The scrolls a map's `Draw` steps each frame and writes into its
//! materials' texture offsets: a town's `DrawBG` statics, a story map's
//! `EA_moveTex` rows (`MoveTexture`), the arena's and the disc's `DrawBG`
//! statics. Each lane is stepped and wrapped ([`Wrap`]); the materials take
//! `vftoi12` of a lane less their crop (docs/engine/root-towns.md,
//! docs/engine/evarea.md).

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
}

impl Wrap {
    /// `u` stepped by `step`, wrapped.
    pub fn step(self, u: F, step: F) -> F {
        if self == Wrap::Under {
            let u = ee::sub(u, step);
            return if ee::lt(u, 0) { ONE } else { u };
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

/// Whether the materials take a lane as the step leaves it or as it found
/// it (`EVENTAREAB0::DrawBG` writes, then steps).
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

/// A material the scrolls offset: its U by one lane, its V by one (None
/// leaves that offset 0).
#[derive(Clone, Copy, Debug)]
pub struct MatScroll {
    pub mat: &'static str,
    pub u: Option<usize>,
    pub v: Option<usize>,
}

impl MatScroll {
    /// U by lane `lane`.
    pub const fn u(mat: &'static str, lane: usize) -> MatScroll {
        MatScroll { mat, u: Some(lane), v: None }
    }

    /// V by lane `lane`.
    pub const fn v(mat: &'static str, lane: usize) -> MatScroll {
        MatScroll { mat, u: None, v: Some(lane) }
    }

    /// U and V by the same lane.
    pub const fn uv(mat: &'static str, lane: usize) -> MatScroll {
        MatScroll { mat, u: Some(lane), v: Some(lane) }
    }

    /// `MOVETEX` row `row` of a table: U by its lane `2 row`, V by `2 row + 1`.
    pub const fn move_tex(mat: &'static str, row: usize) -> MatScroll {
        MatScroll { mat, u: Some(2 * row), v: Some(2 * row + 1) }
    }
}

/// A map's scrolls: each lane's step a frame, how it wraps, where it
/// starts and lives, when the materials take it, and the materials.
#[derive(Clone, Copy, Debug)]
pub struct ScrollsDef {
    pub steps: &'static [F],
    pub wrap: Wrap,
    /// What a lane holds before anything carried is taken (the disc's 1).
    pub from: F,
    pub shown: Shown,
    pub carry: Option<Carry>,
    pub mats: &'static [MatScroll],
}

impl ScrollsDef {
    /// A town's `DrawBG`: its statics from `slot`, each above 1 back to 0.
    pub const fn statics(steps: &'static [F], slot: usize, mats: &'static [MatScroll]) -> ScrollsDef {
        ScrollsDef { steps, wrap: Wrap::Past, from: 0, shown: Shown::After, carry: Some(Carry::Statics(slot)), mats }
    }

    /// A story map's `EA_moveTex` table, gcmn's rows from `row`: each
    /// `MOVETEX`'s (u_step, v_step) in `steps`.
    pub const fn move_tex(steps: &'static [F], row: usize, mats: &'static [MatScroll]) -> ScrollsDef {
        ScrollsDef { steps, wrap: Wrap::Past, from: 0, shown: Shown::After, carry: Some(Carry::MoveTex(row)), mats }
    }
}

/// The scrolls as the last step left them.
#[derive(Clone, Debug)]
pub struct Scrolls {
    def: &'static ScrollsDef,
    pub v: Vec<F>,
    /// What the materials took last.
    pub shown: Vec<F>,
    /// The materials' names as the file has them.
    mats: Vec<String>,
}

impl Scrolls {
    pub fn new(def: &'static ScrollsDef, cx: &mut BuildCx) -> Result<Scrolls> {
        let v = vec![def.from; def.steps.len()];
        let mats = def.mats.iter().map(|m| cx.name(m.mat).into_owned()).collect();
        Ok(Scrolls { def, shown: v.clone(), v, mats })
    }

    /// `vftoi12` of lane `k` as the materials last took it.
    pub fn uv(&self, k: usize) -> u16 {
        ee::vftoi12(self.shown[k])
    }
}

impl MapSystem for Scrolls {
    fn step(&mut self, hook: Hook, _: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        if self.def.shown == Shown::Before {
            self.shown.clone_from(&self.v);
        }
        for (x, &step) in self.v.iter_mut().zip(self.def.steps) {
            *x = self.def.wrap.step(*x, step);
        }
        if self.def.shown == Shown::After {
            self.shown.clone_from(&self.v);
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
        self.shown.clone_from(&self.v);
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
    }

    /// Each material's offsets: `vftoi12` of its lanes less its crop.
    fn rows(&self, file: &SceneFile, out: &mut HashMap<u32, [u8; 2]>) {
        for (m, name) in self.def.mats.iter().zip(&self.mats) {
            let Some(mat) = file.ccs.find_object(name) else { continue };
            let Some(info) = file.scene.materials.get(&mat) else { continue };
            let off = |lane: Option<usize>, crop: u16| lane.map_or(0, |k| self.uv(k).wrapping_sub(crop));
            out.insert(mat, [(off(m.u, info.crop_u) >> 4) as u8, (off(m.v, info.crop_v) >> 4) as u8]);
        }
    }
}
