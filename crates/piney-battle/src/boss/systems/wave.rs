//! The wave: a boss class's ring clip (`m_anmw`), set and run by its acts,
//! drawn where it is posed ([`super::super::Out::DrawWave`]).

use super::super::{Anm, BossBase, Cx, Output};
use super::{BossSystem, Systems};

/// `m_anmw`: a `ccAnm` of its own, from `ccAnm::ccAnm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wave {
    pub anm: Anm,
}

impl Default for Wave {
    fn default() -> Wave {
        Wave { anm: Anm::new() }
    }
}

impl BossSystem for Wave {
    /// The wave's pose, which `DrawWave` draws.
    fn draw(&self, _b: &BossBase, _all: &Systems, _cx: &mut Cx, out: &mut Output) {
        out.wave = Some(self.anm.posed);
    }
}
