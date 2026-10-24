//! Corbenik's own boss effects, as the manager's pass runs them: the wave
//! (`ccBossEffWave`, `Draw` 0x00377c80, made by `ccBossEffWaveCreate`
//! 0x0037dd60), whose callback hurts, and a seed's flush
//! (`ccBossEffFlush`, `Draw` 0x00376ea0, `ccBossEffFlushCreate`
//! 0x0037dec0). What they draw is handed out as [`Pic`]s. The energy grow
//! is the flash's alone, which nothing reaches.

use piney_data::field::ee;

use super::Pic;
use crate::boss::{Anm, Boss, Cx, Out};
use crate::geom::V4;

type F = u32;

/// A Corbenik effect with a life of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fx {
    Wave(Box<Wave>),
    Flush(Flush),
}

impl Fx {
    /// The effect's `Draw` (out of its slot of `b`'s manager while it
    /// runs): whether it is still enabled.
    pub fn draw(&mut self, cx: &mut Cx, b: &mut Boss) -> bool {
        match self {
            Fx::Wave(w) => w.draw(cx, b),
            Fx::Flush(f) => f.draw(cx),
        }
    }
}

/// `ccBossEffWave` (0x40 bytes): `xeffect`'s `ANM_xx11wave` at 512, then
/// the callback (`ccBoss08::WaveFuncCB`) every `Draw`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wave {
    pub anm: Anm,
    pub pos: V4,
}

impl Wave {
    pub fn new(cx: &Cx, pos: V4) -> Wave {
        let mut anm = Anm::new();
        anm.set("ANM_xx11wave", cx.clips);
        anm.speed = 512;
        Wave { anm, pos }
    }

    /// `Draw` (0x00377c80): the clip on (drawn at the place), off once it
    /// ends; `WaveFuncCB(anm, pos)` (0x003adf80) after, every time: at the
    /// clip's frame 8 boss skill 27 from the place and a quake (35, 25,
    /// 50).
    fn draw(&mut self, cx: &mut Cx, b: &mut Boss) -> bool {
        let ended = self.anm.forward();
        cx.out(Out::Corbenik(Pic::Wave { time: self.anm.posed, pos: self.pos }));
        if self.anm.frame() == 8 {
            b.skill_damage(cx, None, Some(self.pos), 27);
            if cx.boss_cam {
                cx.out(Out::Quake([0x420c_0000, 0x41c8_0000, 0x4248_0000]));
            }
        }
        !ended
    }
}

/// `ccBossEffFlush` (0x30 bytes): particle's `EFF_x000` on `CLT_x000c5`
/// shrinking from `start` to `end` in ten steps, off after the step past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flush {
    pub pos: V4,
    pub start: F,
    pub end: F,
    pub size: F,
}

impl Flush {
    pub fn new(pos: V4, start: F, end: F) -> Flush {
        Flush { pos, start, end, size: start }
    }

    pub(crate) fn draw(&mut self, cx: &mut Cx) -> bool {
        cx.out(Out::Corbenik(Pic::Flush { pos: self.pos, size: self.size }));
        let v = ee::add(self.size, ee::div(ee::sub(self.end, self.start), 0x4120_0000));
        self.size = v;
        if ee::lt(v, self.end) {
            self.size = self.start;
            return false;
        }
        true
    }
}
