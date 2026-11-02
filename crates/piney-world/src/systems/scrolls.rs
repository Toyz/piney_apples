//! `DrawBG`'s scrolls: statics of the executable each stepped a frame and
//! wrapped ([`Wrap`]); what the sky's materials are offset by
//! (docs/engine/root-towns.md).

use piney_data::Result;

use super::{BuildCx, Hook, MapSystem, Others, Statics, StepCx};
use crate::ee::{self, F, ONE};

/// How a scroll comes back once a step takes it to 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// Above 1, back to 0 (`DrawBG`'s scrolls).
    Past,
    /// At 1 or above, back to 0 (Dun Loireag's and Carmina Gadelica's water).
    AtOrPast,
    /// Above 1, less 1 (Mac Anu's water).
    Less,
}

impl Wrap {
    /// `u` stepped by `step`, wrapped.
    pub fn step(self, u: F, step: F) -> F {
        let u = ee::add(u, step);
        match self {
            Wrap::Past if !ee::le(u, ONE) => 0,
            Wrap::AtOrPast if !ee::lt(u, ONE) => 0,
            Wrap::Less if !ee::le(u, ONE) => ee::sub(u, ONE),
            _ => u,
        }
    }
}

/// `DrawBG`'s scrolls: each stepped by its step, back to 0 past 1.
pub fn step_scrolls(scrolls: &mut [F], steps: &[F]) {
    for (x, &step) in scrolls.iter_mut().zip(steps) {
        *x = Wrap::Past.step(*x, step);
    }
}

/// A `DrawBG`'s scrolls: each one's step a frame, and where the first sits
/// in the statics (the rest follow it).
#[derive(Clone, Copy, Debug)]
pub struct ScrollsDef {
    pub steps: &'static [F],
    pub slot: usize,
}

/// The scrolls as the last `DrawBG` left them.
#[derive(Clone, Debug)]
pub struct Scrolls {
    def: &'static ScrollsDef,
    pub v: Vec<F>,
}

impl Scrolls {
    pub fn new(def: &'static ScrollsDef, _: &mut BuildCx) -> Result<Scrolls> {
        Ok(Scrolls { def, v: vec![0; def.steps.len()] })
    }

    /// `vftoi12` of scroll `k`: the offset `DrawBG` writes from it.
    pub fn uv(&self, k: usize) -> u16 {
        ee::vftoi12(self.v[k])
    }
}

impl MapSystem for Scrolls {
    fn step(&mut self, hook: Hook, _: &mut StepCx, _: &mut Others) {
        if hook == Hook::Draw {
            step_scrolls(&mut self.v, self.def.steps);
        }
    }

    fn take(&mut self, s: &Statics) {
        let at = self.def.slot..self.def.slot + self.v.len();
        self.v.copy_from_slice(&s[at]);
    }

    fn put(&self, s: &mut Statics) {
        s[self.def.slot..self.def.slot + self.v.len()].copy_from_slice(&self.v);
    }
}
