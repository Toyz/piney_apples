//! Macha's own boss effects, as the manager's pass runs them: the eyes'
//! lasers (`ccBossEffLaserShot`, QUA gcmn 0x003741b0-0x003746b0, made by
//! `ccBossEffLaserShotCreate` 0x0037d250) and the light ball's shock
//! (`ccBossEffLightBallShock`, 0x003746c0-0x00374b8c, made by
//! `ccBossEffLightBallShockCreate` 0x0037d320). Both draw `xeffect`'s
//! clumps; what they draw is handed out as [`Pic`]s.

use piney_data::field::ee;
use piney_data::libm;

use super::{Out, Pic};
use crate::boss::Cx;
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// The land (`ccHitCheckLM`'s mask).
const LAND: u32 = 0x2000_0000;

/// A Macha effect with a life of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fx {
    Laser(Laser),
    Shock(Shock),
}

impl Fx {
    /// The effect's `Draw`: whether it is still enabled.
    pub fn draw(&mut self, cx: &mut Cx) -> bool {
        match self {
            Fx::Laser(l) => l.draw(cx),
            Fx::Shock(s) => s.draw(cx),
        }
    }
}

/// The pitch (`atan2(sp.z - tp.z, |sp tp|)`) and heading from `from` to
/// `to`, both through `ccTransPosW2P`.
fn aim(cx: &Cx, from: V4, to: V4) -> (F, F) {
    let (sp, tp) = (crate::boss::w2p(cx, from), crate::boss::w2p(cx, to));
    let d = crate::boss::dist(cx, sp, tp);
    (libm::atan2f(ee::sub(sp[2], tp[2]), d), get_dirc(sp, tp))
}

/// `ccBossEffLaserShot`: a bolt (`CMP_ex31les0`) from an eye to where the
/// target stood, `speed` a frame along the line between, until the line
/// from its start to it meets the land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Laser {
    /// +0x20 where it is, +0x30 where it left, +0x40 where it was aimed.
    pub pos: V4,
    pub start: V4,
    pub end: V4,
    /// +0x50: its step a frame.
    pub vel: V4,
}

impl Laser {
    /// The constructor (0x003741b0): the step `speed` along `Ry(pitch)`,
    /// `Rz(heading)`, `Rz(-pi/2)`.
    pub fn new(cx: &Cx, from: V4, to: V4, speed: F) -> Laser {
        let (a, r) = aim(cx, from, to);
        let m = geom::rot_matrix_y(&geom::unit_matrix(), a);
        let m = geom::rot_matrix_z(&m, r);
        let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
        let vel = geom::apply_matrix(&m, [speed, 0, 0, ONE]);
        Laser { pos: from, start: from, end: to, vel }
    }

    /// `Draw` (0x00374440): the bolt drawn at a tenth, turned along its
    /// line; where the line from its start to it meets the land, two
    /// bursts and rocks there and the effect off; then a step on.
    fn draw(&mut self, cx: &mut Cx) -> bool {
        let (a, r) = aim(cx, self.start, self.end);
        cx.out(Out::Macha(Pic::Laser { pos: self.pos, pitch: a, heading: r }));
        let mut alive = true;
        if let Some(at) = cx.part_hits.line(self.start, self.pos, LAND) {
            let turn = rand_f(cx.cc, PI);
            cx.out(Out::Macha(Pic::LaserHit { pos: at, turn }));
            alive = false;
        }
        let p = geom::vadd(crate::boss::w2p(cx, self.pos), self.vel);
        self.pos = crate::boss::p2w(cx, p);
        alive
    }
}

/// `ccBossEffLightBallShock`: the light ball's blast (`CMP_ex31exp1`)
/// growing 10/3 a frame as it fades by a thirtieth, with bursts of eight
/// at random waits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shock {
    /// +0x20, +0x14, +0x30, +0x34.
    pub pos: V4,
    pub transparency: F,
    pub scale: F,
    pub wait: i32,
}

impl Shock {
    /// The constructor (0x003746c0), after the ring it makes
    /// (`ccBossEffAutoSamonRingCreate(pos, 0, (0, 2/3, 10), 180)`).
    pub fn new(pos: V4) -> Shock {
        Shock { pos, transparency: ONE, scale: 0, wait: 0 }
    }

    /// `Draw` (0x003748a0).
    fn draw(&mut self, cx: &mut Cx) -> bool {
        cx.out(Out::Macha(Pic::Blast { pos: self.pos, scale: self.scale, transparency: self.transparency }));
        self.scale = ee::add(self.scale, 0x4055_5555);
        let t = ee::sub(self.transparency, 0x3d08_8889);
        let alive = !ee::lt(t, 0);
        self.transparency = t;
        let w = self.wait;
        self.wait -= 1;
        if w > 0 {
            return alive;
        }
        self.wait = cx.cc.rand() & 15;
        for k in 0..8 {
            let reach = libm::fabsf(rand_f(cx.cc, 0x447a_0000));
            let m = geom::rot_matrix_z(&geom::unit_matrix(), rand_f(cx.cc, PI));
            let v = geom::apply_matrix(&m, [reach, 0, 0, ONE]);
            let pos = geom::vadd(self.pos, v);
            let rock = soft_add(45.0, libm::fabsf(rand_f(cx.cc, 0x4234_0000)));
            let size = soft_add(1.0, libm::fabsf(rand_f(cx.cc, 0x41a0_0000)));
            cx.out(Out::Macha(Pic::Burst { pos, rock, size, kind: k & 1 }));
        }
        alive
    }
}

/// `dpadd(a, (double)x)` then `dptofp`: the sum in double, rounded once to
/// float (the soft float's nearest).
fn soft_add(a: f64, x: F) -> F {
    ((a + f64::from(f32::from_bits(x))) as f32).to_bits()
}
