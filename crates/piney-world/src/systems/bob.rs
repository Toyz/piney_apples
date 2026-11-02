//! `EVENTAREA07::DrawBG`'s bob: the sky rising and sinking between 0 and
//! its top by `fieldrand` each frame ([`super::sky::At::Bob`] clumps ride
//! it), and where `ChangeBlock` puts the rows that ride it
//! (`ModelsDef::ride`, `ObjectsDef::ride`).

use piney_data::Result;

use super::{BuildCx, Hook, MapSystem, Others, StepCx};
use crate::ee::{self, F};

/// How far a step goes at most (`fieldrand(n)`) and where the bob turns.
#[derive(Clone, Copy, Debug)]
pub struct BobDef {
    pub rand: u32,
    pub top: F,
}

/// The bob (+0x1d8, the z of +0x1d0) and its way (+0x1c4: 1 rising).
#[derive(Clone, Debug)]
pub struct Bob {
    def: &'static BobDef,
    pub z: F,
    pub rising: bool,
}

impl Bob {
    pub fn new(def: &'static BobDef, _: &mut BuildCx) -> Result<Bob> {
        Ok(Bob { def, z: 0, rising: false })
    }

    /// Where it stands.
    pub fn z(&self) -> F {
        self.z
    }
}

impl MapSystem for Bob {
    /// Up by `fieldrand(rand)` until past the top (held there), down until
    /// under 0 (held at 0), turning at each.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let r = cx.core.field_rand.as_mut().map_or(0, |rng| ee::from_int(rng.below(self.def.rand) as i32));
        if self.rising {
            self.z = ee::add(self.z, r);
            if !ee::le(self.z, self.def.top) {
                self.z = self.def.top;
                self.rising = false;
            }
        } else {
            self.z = ee::sub(self.z, r);
            if ee::lt(self.z, 0) {
                self.z = 0;
                self.rising = true;
            }
        }
    }
}
