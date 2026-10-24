//! Tarvos's own boss effects, as the manager's pass runs them: the spike
//! rain (`ccBossEffSpikeRain`, QUA gcmn 0x003738f0-0x003741ac, made by
//! `ccBossEffSpikeRainCreate` 0x0037d180) and the phantom
//! (`ccBossEffPhantom`, 0x0037a4b0-0x0037b7a4, made by
//! `ccBossEffPhantomCreate` 0x0037dca0). What they draw is handed out as
//! [`Pic`]s; what they do to the boss that made them (the spikes' hits,
//! the phantom's ring) is done on it as they draw, in their order.

use piney_data::field::ee;

use super::{Out, Pic};
use crate::boss::{Boss, Cx, EffKind};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
const QUARTER_PI: F = 0x3f49_0fdb;

/// A Tarvos effect with a life of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fx {
    Spikes(Box<SpikeRain>),
    Phantom(Box<Phantom>),
}

impl Fx {
    /// The effect's `Draw` (taken out of its slot of `b`'s manager while
    /// it runs): whether it is still enabled.
    pub fn draw(&mut self, cx: &mut Cx, b: &mut Boss) -> bool {
        match self {
            Fx::Spikes(s) => s.draw(cx, b),
            Fx::Phantom(p) => p.draw(cx, b),
        }
    }
}

/// `atan2` through the soft float's doubles (`fptodp`, `atan2`, `dptofp`).
fn atan2d(y: F, x: F) -> F {
    (f64::from(f32::from_bits(y)).atan2(f64::from(f32::from_bits(x))) as f32).to_bits()
}

/// `(len, 0, 0, 1)` turned by `Ry(pitch)`, `Rz(heading)`, `Rz(-pi/2)`: a
/// step along the line from a point to what it looks at.
fn along(pitch: F, heading: F, len: F) -> V4 {
    let m = geom::rot_matrix_y(&geom::unit_matrix(), pitch);
    let m = geom::rot_matrix_z(&m, heading);
    let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
    geom::apply_matrix(&m, [len, 0, 0, ONE])
}

/// One of the rain's 32 spikes (`SPIKE_T`, 0xe0 bytes: a `ccClump` of
/// `xeffect`'s `CMP_ex71body2`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spike {
    /// +0xc0 where it is, +0xb0 its first step.
    pub pos: V4,
    pub vel: V4,
    /// +0xa0 the clump's transparency.
    pub transparency: F,
    /// +0xd0: 0 waiting, 1 flying, 2 fading, 3 gone; +0xd4 the wait.
    pub state: i32,
    pub wait: i32,
}

/// `ccBossEffSpikeRain` (0x1c60 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpikeRain {
    pub spikes: [Spike; 32],
    /// +0x1c20 the point they fly to (half the target's height up), +0x1c10
    /// the target.
    pub aim: V4,
    pub target: usize,
    /// +0x1c50: the first flight's sound made.
    pub sounded: bool,
}

impl SpikeRain {
    /// The constructor (0x003738f0): 32 spikes 200 from a point 500 over
    /// the target's head, each at a random bearing (`ccRandF(pi)` twice),
    /// spike `k` waiting `k * 60 / 32` frames.
    pub fn new(cx: &mut Cx, target: usize) -> SpikeRain {
        let ch = &cx.scene.chars[target];
        let (tp, h) = (ch.pos, ch.base().height);
        let mut top = tp;
        top[2] = ee::add(tp[2], ee::add(0x43fa_0000, h));
        let mut aim = tp;
        aim[2] = ee::add(tp[2], ee::div(h, 0x4000_0000));
        let mut spikes = [Spike::default(); 32];
        for (k, s) in spikes.iter_mut().enumerate() {
            s.transparency = ONE;
            s.wait = (k as i32 * 60) >> 5;
            let mut a = rand_f(cx.cc, PI);
            if ee::lt(a, 0) {
                a = ee::add(a, PI);
            }
            let m = geom::rot_matrix_y(&geom::unit_matrix(), a);
            let m = geom::rot_matrix_z(&m, rand_f(cx.cc, PI));
            s.pos = geom::vadd(top, geom::apply_matrix(&m, [0x4348_0000, 0, 0, ONE]));
            let (p, q) = (crate::boss::w2p(cx, s.pos), crate::boss::w2p(cx, aim));
            let pitch = atan2d(ee::sub(p[2], q[2]), crate::boss::dist(cx, q, p));
            s.vel = along(pitch, get_dirc(p, q), 0x4248_0000);
        }
        SpikeRain { spikes, aim, target, sounded: false }
    }

    /// `Draw` (0x00373d60): each spike drawn at a quarter, looking at the
    /// aim. A waiting one counts down (the first to fly sounds 63), a
    /// flying one steps 50 at the aim until within 50 of it, where it hits
    /// (`_SpikeShotCB(target, pos, index)`, the index 31 for the first) and
    /// fades a fifteenth a frame. The target gone, all fade. Over once
    /// every spike is gone; a frame with hits sounds 171.
    fn draw(&mut self, cx: &mut Cx, b: &mut Boss) -> bool {
        let aim_p = crate::boss::w2p(cx, self.aim);
        let t = self.target;
        if !(cx.valid(Some(t)) && !cx.dead(t)) {
            for s in self.spikes.iter_mut().filter(|s| s.state != 3) {
                s.state = 2;
            }
        }
        let (mut done, mut hit) = (true, false);
        for (k, s) in self.spikes.iter_mut().enumerate() {
            if s.state == 3 {
                continue;
            }
            done = false;
            let p = crate::boss::w2p(cx, s.pos);
            let pitch = atan2d(ee::sub(p[2], aim_p[2]), crate::boss::dist(cx, p, aim_p));
            let heading = get_dirc(p, aim_p);
            cx.out(Out::Tarvos(Pic::Spike { pos: s.pos, pitch, heading, transparency: s.transparency }));
            match s.state {
                0 => {
                    let w = s.wait;
                    s.wait -= 1;
                    if w < 0 {
                        s.wait = 0;
                        s.state = 1;
                        if !self.sounded {
                            self.sounded = true;
                            cx.out(Out::Se3d { se: 63, pos: self.aim });
                        }
                    }
                }
                1 => {
                    let d = geom::length_on(cx.actx.volume, geom::vsub(aim_p, p));
                    if ee::le(d, 0x4248_0000) {
                        s.state = 2;
                        super::spike_hit(b, cx, t, s.pos, 31 - k as i32);
                        hit = true;
                    } else {
                        let p = geom::vadd(p, along(pitch, heading, 0x4248_0000));
                        s.pos = crate::boss::p2w(cx, p);
                    }
                }
                2 => {
                    let mut tr = ee::sub(s.transparency, 0x3d88_8889);
                    if ee::lt(tr, 0) {
                        tr = 0;
                        s.state = 3;
                    }
                    s.transparency = tr;
                }
                _ => {}
            }
        }
        if hit {
            cx.out(Out::Se3d { se: 171, pos: self.aim });
        }
        !done
    }
}

/// One of the phantom's 14 lights (80 bytes from +0x20): its particles'
/// generator (the picture's), where it is, its step, state, turn, wait
/// and the burst it ends in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Light {
    /// +0x10 where it is, +0x30 its step.
    pub pos: V4,
    pub vel: V4,
    /// +0x40: 0 circling, 3 waiting to fall, 2 falling, 5 burst.
    pub state: i32,
    /// +0x44 its bearing about the centre, +0x48 its wait, +0x4c the burst
    /// (`@6654`'s row).
    pub angle: F,
    pub wait: i32,
    pub kind: i32,
}

/// `@6654`: the bursts' kinds (`effExplode3`'s last argument).
pub const BURSTS: [i32; 8] = [2, 1, 0, 3, 4, 5, 1, 2];

/// `ccBossEffPhantom` (0x4f0 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phantom {
    pub lights: [Light; 14],
    /// +0x480 the sprite's size, +0x484 the circle's radius, +0x4d8 the
    /// radius it shrinks from.
    pub size: F,
    pub radius: F,
    pub radius0: F,
    /// +0x490 the centre, +0x4a0 the same in the player's frame, +0x4b0 the
    /// camera's eye, +0x4c0 the centre it started at, +0x4e4 the heading.
    pub pos: V4,
    pub pos_p: V4,
    pub eye: V4,
    pub origin: V4,
    pub dirc: F,
    /// +0x4d0 the step, +0x4d4 its count, +0x4dc the lights' moves left,
    /// +0x4e0 the burst made.
    pub state: i32,
    pub count: i32,
    pub moves: i32,
    pub burst: bool,
}

impl Phantom {
    /// The constructor (0x0037a4b0): the eye 1500 out from the target's
    /// place along the boss's heading, 500 up; eight lights on a circle of
    /// 400 about the place, 200 up, light `k` waiting `15 k`; the boss
    /// camera free on them.
    pub fn new(cx: &mut Cx, pos: V4, dirc: F) -> Phantom {
        let p = crate::boss::w2p(cx, pos);
        let m = geom::rot_matrix_z(&geom::unit_matrix(), dirc);
        let v = geom::apply_matrix(&m, [0x44bb_8000, 0, 0, ONE]);
        let mut eye = crate::boss::p2w(cx, geom::vadd(p, v));
        eye[2] = 0x43fa_0000;
        let radius = 0x43c8_0000;
        let mut lights = [Light::default(); 14];
        for (k, l) in lights.iter_mut().enumerate().take(8) {
            let mut a = ee::mul(QUARTER_PI, ee::from_int(k as i32));
            if !ee::le(a, PI) {
                a = ee::sub(a, TWO_PI);
            }
            l.angle = a;
            let c = ee::mul(radius, piney_data::libm::cosf(a));
            let s = ee::mul(radius, piney_data::libm::sinf(a));
            l.pos = geom::vadd(pos, [c, s, 0x4348_0000, ONE]);
            l.wait = 15 * k as i32;
            l.kind = k as i32 & 7;
        }
        // The view is sp+144, never written here: operator new's frame
        // left `$s1` there (OnThinkPhantom's target, a pointer: a float
        // under 1e-37, then its sign word; above, `$s1`'s upper half,
        // which no code of the game writes but by saving and restoring).
        if cx.boss_cam {
            cx.out(Out::CamMode { mode: 5 });
            cx.out(Out::FreeCam { pos, view: [0; 4] });
        }
        cx.out(Out::Tarvos(Pic::PhantomStart));
        cx.out(Out::Tarvos(Pic::PhantomLights(Box::new(lights.map(|l| (l.pos, l.state))))));
        Phantom {
            lights,
            size: 0x447a_0000,
            radius,
            radius0: radius,
            pos,
            pos_p: crate::boss::w2p(cx, pos),
            eye,
            origin: pos,
            dirc,
            state: 0,
            count: 0,
            moves: 0,
            burst: false,
        }
    }

    /// `InitRadiateParam` (0x0037a980): every light to the centre; four
    /// stepping out 20 a frame round the level, one up, one down, eight on
    /// the diagonals; 25 moves.
    fn init_radiate(&mut self) {
        for l in &mut self.lights {
            l.pos = self.pos;
        }
        let mut f = 0;
        for l in &mut self.lights[..4] {
            let m = geom::rot_matrix_z(&geom::unit_matrix(), f);
            l.vel = geom::apply_matrix(&m, [0x41a0_0000, 0, 0, ONE]);
            f = wrap(ee::add(f, HALF_PI));
        }
        self.lights[4].vel = [0, 0, 0x41a0_0000, ONE];
        self.lights[5].vel = [0, 0, 0xc1a0_0000, ONE];
        let mut f = QUARTER_PI;
        for k in [6, 10] {
            let m = geom::rot_matrix_y(&geom::unit_matrix(), QUARTER_PI);
            let m = geom::rot_matrix_z(&m, f);
            self.lights[k].vel = geom::apply_matrix(&m, [0x41a0_0000, 0, 0, ONE]);
            self.lights[k + 1].vel = geom::vscale(self.lights[k].vel, 0xbf80_0000);
            let mut v = self.lights[k + 1].vel;
            v[2] = ee::mul(v[2], 0xbf80_0000);
            self.lights[k + 2].vel = v;
            self.lights[k + 3].vel = geom::vscale(self.lights[k + 2].vel, 0xbf80_0000);
            f = wrap(ee::add(f, HALF_PI));
        }
        self.moves = 25;
    }

    /// `Draw` (0x0037add0), by step: 0 a wait of 31 frames; 1 and 2 the
    /// eight lights circling up 200/3 a frame (2 once over 2000, the
    /// circle shrinking over 60 frames); 3 the sprite shrinking 100 a
    /// frame, then a ring and the lights sent out; 4 their 25 moves and 31
    /// frames' wait; 5 their falls, waits of 5 a pair, into the ground
    /// (bursts, a quake each; at three a radiate and an explosion); 6 25
    /// frames, the camera back, the effect off.
    fn draw(&mut self, cx: &mut Cx, b: &mut Boss) -> bool {
        let cam = cx.boss_cam;
        let mut view = self.pos;
        match self.state {
            0 => {
                let c = self.count;
                self.count += 1;
                if c >= 31 {
                    self.count = 0;
                    self.state += 1;
                    cx.out(Out::SeNote { se: 61, note: 60 });
                } else if self.count == 1 {
                    cx.out(Out::SeNote { se: 245, note: 62 });
                    cx.out(Out::SeNote { se: 63, note: 55 });
                }
                if cam {
                    cx.out(Out::FreeCam { pos: self.eye, view });
                }
            }
            1 | 2 => {
                let centre = self.pos_p;
                for l in &mut self.lights[..8] {
                    let m = geom::rot_matrix_z(&geom::unit_matrix(), l.angle);
                    let v = geom::apply_matrix(&m, [self.radius, 0, 0, ONE]);
                    l.pos = crate::boss::p2w(cx, geom::vadd(centre, v));
                    l.angle = ee::add(l.angle, 0x3d56_7750);
                    if !ee::le(l.angle, PI) {
                        l.angle = ee::sub(l.angle, TWO_PI);
                    }
                }
                self.pos_p[2] = ee::add(self.pos_p[2], 0x4285_5555);
                if self.state == 1 && !ee::le(self.pos_p[2], 0x44fa_0000) {
                    self.state += 1;
                } else if self.state == 2 {
                    let r = ee::sub(self.radius, ee::div(self.radius0, 0x4270_0000));
                    self.radius = r;
                    if ee::lt(r, 0) {
                        self.radius = 0;
                        self.state += 1;
                        self.eye[2] = 0x459c_4000;
                    }
                }
                self.pos = crate::boss::p2w(cx, self.pos_p);
                if self.state == 2 && !ee::le(self.pos[2], 0x44fa_0000) {
                    self.eye[2] = 0x451c_4000;
                    view[2] = ee::sub(view[2], 0x4348_0000);
                }
                if cam {
                    cx.out(Out::FreeCam { pos: self.eye, view });
                }
            }
            3 => {
                cx.out(Out::Tarvos(Pic::PhantomSprite { pos: self.pos, size: self.size }));
                let mut s = ee::sub(self.size, 0x42c8_0000);
                if ee::lt(s, 0) {
                    let param = [0, 0x3f2a_aaab, 0x4120_0000, 0];
                    b.effect_at(cx, EffKind::AutoSamonRingOf { n: 195, param }, self.pos, [0; 4]);
                    s = 0;
                    self.init_radiate();
                    cx.out(Out::SeNote { se: 35, note: 52 });
                    self.count = 0;
                    self.state += 1;
                }
                self.size = s;
            }
            4 => {
                if self.moves != 0 {
                    self.moves -= 1;
                    for l in &mut self.lights {
                        let p = geom::vadd(crate::boss::w2p(cx, l.pos), l.vel);
                        l.pos = crate::boss::p2w(cx, p);
                    }
                } else {
                    let c = self.count;
                    self.count += 1;
                    if c == 30 {
                        for (k, l) in self.lights.iter_mut().enumerate() {
                            l.vel = [0, 0, 0xc2c8_0000, ONE];
                            l.wait = (k as i32 >> 1) * 5;
                            l.state = 3;
                            l.kind = k as i32 & 7;
                        }
                        self.count = 0;
                        self.state += 1;
                    }
                }
            }
            5 => {
                let mut done = true;
                for (k, l) in self.lights.iter_mut().enumerate() {
                    let index = 13 - k as i32;
                    if l.state == 3 {
                        let w = l.wait;
                        l.wait -= 1;
                        if w == 0 {
                            l.state = 2;
                            done = false;
                            continue;
                        }
                    }
                    if l.state == 2 {
                        let p = geom::vadd(crate::boss::w2p(cx, l.pos), l.vel);
                        l.pos = crate::boss::p2w(cx, p);
                        l.vel[2] = ee::sub(l.vel[2], 0x40a0_0000);
                        if ee::lt(p[2], 0) {
                            cx.out(Out::Tarvos(Pic::Burst { pos: l.pos, kind: BURSTS[l.kind as usize & 7] }));
                            cx.out(Out::Se { se: 35 });
                            if index == 13 || index == 0 {
                                cx.out(Out::SeNote { se: 56, note: 57 });
                            }
                            l.state = 5;
                            if cam {
                                cx.quake_vec([0x41f0_0000, 0x4170_0000, 0x41a0_0000]);
                            }
                        }
                    }
                    if l.state != 5 {
                        done = false;
                    }
                }
                let c = self.count;
                self.count += 1;
                if matches!(c, 27 | 23 | 20 | 11) {
                    cx.out(Out::Se { se: 57 });
                }
                if cam {
                    view[2] = ee::sub(view[2], 0x4348_0000);
                    cx.out(Out::FreeCam { pos: self.eye, view });
                }
                self.pos[2] = ee::sub(self.pos[2], 0x42c8_0000);
                if ee::lt(self.lights[0].pos[2], 0x447a_0000) {
                    let m = geom::rot_matrix_z(&geom::unit_matrix(), self.dirc);
                    let v = geom::apply_matrix(&m, [0x447a_0000, 0, 0, ONE]);
                    self.eye = crate::boss::p2w(cx, geom::vadd(crate::boss::w2p(cx, self.origin), v));
                    self.eye[2] = 0x42c8_0000;
                    self.pos[2] = 0x4396_0000;
                }
                if !self.burst && self.lights.iter().filter(|l| l.state == 5).count() >= 3 {
                    cx.out(Out::Tarvos(Pic::Radiate { pos: self.pos }));
                    cx.out(Out::Tarvos(Pic::Explode { pos: self.origin }));
                    self.burst = true;
                }
                if done {
                    self.count = 0;
                    self.state += 1;
                }
            }
            6 => {
                let c = self.count;
                self.count += 1;
                if c == 25 {
                    cx.out(Out::CamMode { mode: 6 });
                    return false;
                }
            }
            _ => {}
        }
        if (0..=5).contains(&self.state) {
            cx.out(Out::Tarvos(Pic::PhantomLights(Box::new(self.lights.map(|l| (l.pos, l.state))))));
        }
        true
    }
}

/// An angle past pi taken a turn back.
fn wrap(v: F) -> F {
    if ee::le(v, PI) { v } else { ee::sub(v, TWO_PI) }
}
