//! `ccBossBlur`: the boss effect manager's +4 (`ccBossEffManager`'s
//! constructor, INF gcmn 0x004614b0), a `ccBufferSampling` feedback that
//! every boss fight's manager task updates and draws on the manager's own
//! layer (`ccLayer::Init(200, sysLayer's view)`) each frame after its
//! effects (`Main`, 0x00461200). The bosses only write it, through
//! `ccBoss` +0x148: [`Set`]s, which the field applies in order
//! ([`super::Out::Blur`]). docs/engine/boss.md "The blur".

use piney_data::field::ee;
use piney_data::libm;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
/// `Main`'s easing: a thirtieth of the gap a frame.
const THIRTY: F = 0x41f0_0000;

/// The layer the manager draws on (`ccLayer::Init(200, ..)`).
pub const LAYER: i16 = 200;

/// A callback a boss gives the blur (+0x2c), which `Main` calls each
/// frame while the blur is enabled, with its scale, turn and colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurFunc {
    /// `ccBoss03BlurParamCB` (INF gcmn 0x004610e0): Skeith's and
    /// Fidchell's.
    Boss03,
    /// `ccBoss06BlurParamFuncCB` (QUA gcmn 0x003a34a0): Macha's
    /// `Transfer`'s.
    Boss06,
}

/// A boss's write to the blur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Set {
    /// +0x24 1 and `m_exit` 0, always written together.
    Enable,
    Scale(F),
    Turn(F),
    Abgr(u32),
    Func(BlurFunc),
    /// +0x24 0, the scale, turn and colour taken as where `Main` eases
    /// from (`ccBossKyvia01::CheckDiscMove`'s next stage).
    Ease,
}

/// What `Main` hands `ccBufferSampling::SetReflex` before `MakePacket`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reflex {
    pub scale: F,
    pub turn: F,
    pub abgr: u32,
}

/// The callbacks' function statics, which last until power-off: none of
/// them is reset by a fight (`init$1068`, `blurRad$1067`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statics {
    /// `ccBoss03BlurParamCB`'s `blurRad`, None before its first call.
    pub boss03_rad: Option<F>,
    /// `g_sinRot` (QUA gp 0x00279814): `ccBoss06BlurParamFuncCB`'s phase,
    /// which Macha's `Transfer` sets.
    pub sin_rot: F,
}

/// `ccBossBlur`'s members (offsets from the blur, the manager's +4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blur {
    /// +0x0c and the scale `Main` eases from (+0x10).
    pub scale: F,
    pub scale0: F,
    /// +0x14 (radians) and the turn it eases from (+0x18).
    pub turn: F,
    pub turn0: F,
    /// +0x1c, and its copy (+0x20) that nothing reads.
    pub abgr: u32,
    pub abgr0: u32,
    /// +0x24: the callback runs and nothing eases.
    pub enabled: bool,
    /// +0x28 `m_exit`: nonzero, nothing is updated or drawn.
    pub exit: u32,
    /// +0x2c.
    pub func: Option<BlurFunc>,
}

impl Blur {
    /// The manager's constructor: scale 1.01, turn 0, colour 0x40808080,
    /// off, no callback. It never writes `m_exit`, which keeps `exit`, the
    /// word the heap left there; every boss's constructor but Gorre's
    /// writes 0 over it (see the docs).
    pub fn new(exit: u32) -> Blur {
        let (scale, turn, abgr) = (0x3f81_47ae, 0, 0x4080_8080);
        Blur { scale, scale0: scale, turn, turn0: turn, abgr, abgr0: abgr, enabled: false, exit, func: None }
    }

    /// A boss's write.
    pub fn set(&mut self, s: Set) {
        match s {
            Set::Enable => (self.enabled, self.exit) = (true, 0),
            Set::Scale(v) => self.scale = v,
            Set::Turn(v) => self.turn = v,
            Set::Abgr(v) => self.abgr = v,
            Set::Func(f) => self.func = Some(f),
            Set::Ease => {
                self.enabled = false;
                (self.scale0, self.turn0, self.abgr0) = (self.scale, self.turn, self.abgr);
            }
        }
    }

    /// `ccBossBlur::Main`: the callback, or the ease of the scale to 1.0
    /// and the turn to 0 (both arrived sets `m_exit`), then the feedback's
    /// reflex to draw; None once exited.
    pub fn main(&mut self, statics: &mut Statics) -> Option<Reflex> {
        if self.exit != 0 {
            return None;
        }
        if self.enabled
            && let Some(f) = self.func
        {
            f.call(statics, &mut self.scale, &mut self.turn, &mut self.abgr);
        }
        if !self.enabled {
            let scaled = self.ease_scale();
            let turned = self.ease_turn();
            if scaled && turned {
                self.exit = 1;
            }
        }
        Some(Reflex { scale: self.scale, turn: self.turn, abgr: self.abgr })
    }

    /// The scale a thirtieth of its start's gap to 1.0 nearer, held at 1.0
    /// when it passes it: true then.
    fn ease_scale(&mut self) -> bool {
        if ee::lt(self.scale0, ONE) {
            self.scale = ee::add(self.scale, ee::div(ee::sub(ONE, self.scale0), THIRTY));
            if ee::le(self.scale, ONE) {
                return false;
            }
        } else {
            self.scale = ee::sub(self.scale, ee::div(ee::sub(self.scale0, ONE), THIRTY));
            if !ee::lt(self.scale, ONE) {
                return false;
            }
        }
        self.scale = ONE;
        true
    }

    /// The turn a thirtieth of its start nearer 0, held at 0 when it
    /// passes it: true then. From a turn of 0 it never arrives.
    fn ease_turn(&mut self) -> bool {
        self.turn = ee::add(self.turn, ee::div(self.turn0 ^ 0x8000_0000, THIRTY));
        let passed =
            if ee::lt(self.turn0, 0) { !ee::le(self.turn, 0) } else { !ee::le(self.turn0, 0) && ee::lt(self.turn, 0) };
        if passed {
            self.turn = 0;
        }
        passed
    }
}

impl BlurFunc {
    /// The callback on the blur's scale, turn and colour.
    fn call(self, statics: &mut Statics, scale: &mut F, turn: &mut F, abgr: &mut u32) {
        match self {
            BlurFunc::Boss03 => {
                // blurRad += pi/30, back by 2 pi past pi; the scale 1.01
                // about 1 and the turn pi/1800 about 0 with its sine.
                let mut r = ee::add(statics.boss03_rad.unwrap_or(0), 0x3dd6_7750);
                if !ee::le(r, PI) {
                    r = ee::sub(r, TWO_PI);
                }
                statics.boss03_rad = Some(r);
                *scale = ee::add(ONE, ee::mul(0x3c23_d70a, libm::sinf(r)));
                *turn = ee::mul(PI, ee::div(libm::sinf(r), 0x44e1_0000));
                *abgr = 0x4880_8080;
            }
            BlurFunc::Boss06 => {
                // g_sinRot += 0.0349 (2 pi / 180), back by 2 pi past pi; the
                // scale 1 + 0.2 sin, the turn kept.
                let mut r = ee::add(statics.sin_rot, 0x3d0e_fa35);
                if !ee::le(r, PI) {
                    r = ee::sub(r, TWO_PI);
                }
                statics.sin_rot = r;
                *scale = ee::add(ONE, ee::mul(0x3e4c_cccd, libm::sinf(r)));
                let _ = turn;
                *abgr = 0x7080_8080;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manager's own blur, as Gorre's fight has it with a zero
    /// `m_exit`: the scale falls from 1.01 to 1.0 in 31 frames and stays,
    /// the turn never arrives, so it never exits.
    #[test]
    fn the_managers_own_blur_eases_and_never_exits() {
        let mut b = Blur::new(0);
        let mut st = Statics::default();
        let seen: Vec<Reflex> = (0..200).filter_map(|_| b.main(&mut st)).collect();
        assert_eq!(seen.len(), 200);
        assert_eq!(seen[199], Reflex { scale: ONE, turn: 0, abgr: 0x4080_8080 });
        assert!(seen[20].scale > ONE);
        assert_eq!(b.exit, 0);
        assert_eq!(Blur::new(7).main(&mut st), None);
    }
}
