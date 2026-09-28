//! `FIREFLY` (gcmn firefly.cpp, 0x1e0 bytes): the fields' fireflies at
//! night (or, hacked, the data bugs): five made by `WORLD::Generate`, each
//! flying a loop of three natural splines about its base with a trail of
//! sixteen sprites, throwing off `FIREFLY2` sparks
//! ([`crate::firefly`]).
//!
//! ```text
//! FIREFLY()          0x005b1ed0  pattern fieldrand(100); the trail's
//!                                count 0, period 1, timer 1, most 8
//! SetBasePosition(p) 0x005b2100  base p, its z raised by fieldrand(200)
//!                                + 100; the last place the base
//! Init(s, a, b, T)   0x005b21a0  a (EFF_sfpfir_1) the firefly, sixteen b
//!                                (EFF_sfpfir_2) its trail, scale 0.6
//!                                down by 0.04, all without fog; the x
//!                                spline 5 keys over T frames, 0 600 1200
//!                                600 0 when fieldrand(100) >= 51, else
//!                                1200 600 0 600 1200; y 4 keys of
//!                                fieldrand(1200), z of fieldrand(350),
//!                                each closed (key 3 = key 0)
//! Init(s, T)         0x005b2900  hacked: EFF_sfzdigi0 and sixteen
//!                                EFF_sfzdigi1, scale 2.4; the trail
//!                                every sixth frame, up to sixteen; the
//!                                same splines
//! Move()             0x005b30a0  the fade in (0.1 a frame); the place the
//!                                base plus the three splines; the trail
//!                                moved on every period; farther than 3000
//!                                across: transparency 0 and a new base
//!                                2500 about the player on the ground; not
//!                                hacked, a spark when fieldrand(100) >=
//!                                81 (under three); the sparks' Move
//! Draw()             0x005b3a60  (nothing on a hidden chip, base z -500)
//!                                the firefly, its trail (hacked: each
//!                                pattern fieldrand(15)), the sparks
//! ```
//!
//! `NSPLINE::PreCalc` (0x005b1b60) is `FRAC2::InitNaturalSpline`'s
//! Hermite spline; `Move` evaluates it as `u^3 a + u^2 b + u c + d`.

use piney_data::dungeon::Rng;

use crate::ee::{self, F, ONE, V4};
use crate::field_ambient::{Env, Op, Sprite};
use crate::firefly::{Firefly, Fly};

/// One key of an `NSPLINE` (28 bytes): its value, the cubic's
/// coefficients, the key's span as a float and its frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Key {
    pub val: F,
    pub a: F,
    pub b: F,
    pub c: F,
    pub d: F,
    pub span: F,
    pub frame: i32,
}

/// An `NSPLINE` (24 bytes): the keys, the frame, the key the frame is in,
/// the period and whether it wrapped this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spline {
    pub keys: Vec<Key>,
    pub frame: i32,
    pub key: usize,
    pub period: i32,
    pub wrapped: bool,
}

const TWO: F = 0x4000_0000;
const THREE: F = 0x4040_0000;
const MINUS_THREE: F = 0xc040_0000;
/// The last key's span, 10.0.
const LAST_SPAN: F = 0x4120_0000;

impl Spline {
    fn new(vals: &[F], period: i32) -> Spline {
        let keys = vals.iter().map(|&val| Key { val, ..Key::default() }).collect();
        let mut s = Spline { keys, frame: 0, key: 0, period, wrapped: false };
        s.pre_calc();
        s
    }

    /// `NSPLINE::PreCalc` (0x005b1b60).
    fn pre_calc(&mut self) {
        let n = self.keys.len();
        let step = self.period / (n as i32 - 1);
        self.frame = 0;
        self.key = 0;
        let keys = &mut self.keys;
        let mut val = vec![0; n + 2];
        for (v, k) in val.iter_mut().zip(keys.iter()) {
            *v = k.val;
        }
        val[n] = keys[n - 1].val;
        val[n + 1] = keys[n - 1].val;
        for (i, k) in keys.iter_mut().enumerate() {
            k.frame = if i == n - 1 { self.period } else { step * i as i32 };
        }
        for i in 0..n {
            keys[i].span = if i < n - 1 { ee::from_int(keys[i + 1].frame - keys[i].frame) } else { LAST_SPAN };
        }
        for i in 0..n {
            let (f4, f5) = (val[i], val[i + 1]);
            let f2 = if i == 0 {
                ee::sub(f5, f4)
            } else {
                let (span, prev) = (keys[i].span, keys[i - 1].span);
                let acc = ee::mul(span, ee::sub(f4, val[i - 1]));
                let f1 = ee::add(acc, ee::mul(prev, ee::sub(f5, f4)));
                ee::div(f1, ee::add(prev, span))
            };
            let f3 = if i == n - 1 {
                ee::sub(val[i + 2], f5)
            } else {
                let (next, span) = (keys[i + 1].span, keys[i].span);
                let acc = ee::mul(span, ee::sub(val[i + 2], f5));
                let f1 = ee::add(acc, ee::mul(next, ee::sub(f5, f4)));
                ee::div(f1, ee::add(span, next))
            };
            let acc = ee::mul(TWO, f4);
            let f0 = ee::sub(acc, ee::mul(TWO, f5));
            keys[i].a = ee::add(f3, ee::add(f2, f0));
            let acc = ee::add(ee::mul(MINUS_THREE, f4), ee::mul(THREE, f5));
            keys[i].b = ee::sub(ee::sub(acc, ee::mul(TWO, f2)), f3);
            keys[i].c = f2;
            keys[i].d = f4;
        }
    }

    /// One frame on, as `FIREFLY::Move` steps each spline: the value.
    fn step(&mut self) -> F {
        self.wrapped = false;
        self.frame = ee::to_int(ee::add(ee::from_int(self.frame), ONE));
        let k = self.keys[self.key];
        if !ee::lt(ee::from_int(self.frame), ee::add(ee::from_int(k.frame), k.span)) {
            self.key += 1;
            if self.key >= self.keys.len() {
                self.key = 0;
                self.frame = 0;
                self.wrapped = true;
            }
        }
        let k = self.keys[self.key];
        let u = ee::div(ee::from_int(self.frame - k.frame), k.span);
        let u2 = ee::mul(u, u);
        let f1 = ee::mul(ee::mul(u2, u), k.a);
        let acc = ee::add(f1, ee::mul(u2, k.b));
        let f1 = ee::add(acc, ee::mul(u, k.c));
        ee::add(k.d, f1)
    }
}

/// The trail's sixteen sprites and the firefly's own: name, scale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Looks {
    pub name: &'static str,
    pub scale: F,
    pub trail: &'static str,
    pub trail_scale: [F; 16],
}

/// One `FIREFLY`.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldFirefly {
    pub looks: Looks,
    /// +0x74: the hacked kind (its trail's patterns random).
    pub hacked: bool,
    /// +0x48 transparency, +0x50 pattern (0-99).
    pub transparency: F,
    pub pattern: u32,
    /// +0x58 sparks (+0x1b0, three slots), +0x5c the trail's length.
    pub sparks: [Option<Box<Firefly>>; 3],
    pub count: i32,
    pub trail_len: usize,
    /// +0x68 frames to the trail's next step, +0x6c its period, +0x70 its
    /// most.
    pub trail_timer: i32,
    pub trail_period: i32,
    pub trail_max: usize,
    /// +0x80 place, +0x90 base, +0xa0 last place, +0xb0 the trail.
    pub pos: V4,
    pub base: V4,
    pub last: V4,
    pub trail: [V4; 16],
    /// +0x1c8, +0x1cc, +0x1d0: x, y and z about the base.
    pub splines: [Spline; 3],
}

/// 0.6 less 0.04 each, as `Init` counts the trail's scales down.
fn trail_scales(first: F, step: F) -> [F; 16] {
    let mut s = first;
    std::array::from_fn(|_| {
        let v = s;
        s = ee::sub(s, step);
        v
    })
}

impl FieldFirefly {
    /// `new FIREFLY` at a random place of the field, its base set and
    /// `Init` for `hacked` or not (`WORLD::Generate`).
    pub fn new(hacked: bool, bounds: [F; 2], height: &dyn Fn(F, F) -> F, rng: &mut Rng) -> FieldFirefly {
        let pattern = rng.below(100);
        let x = ee::from_int(rng.below(ee::to_int(bounds[0]) as u32) as i32);
        let y = ee::from_int(rng.below(ee::to_int(bounds[1]) as u32) as i32);
        let z = height(x, y);
        FieldFirefly::init(hacked, pattern, [x, y, z, 0], rng)
    }

    /// `new FIREFLY` in a lake room (`DUNGEON::SetRoom`, 0x005c38c0): the
    /// base up to 1000 each way from the room's centre on the ground, then
    /// `SetBasePosition` and `Init` as in a field.
    pub fn lake(hacked: bool, centre: [F; 2], rng: &mut Rng) -> FieldFirefly {
        let pattern = rng.below(100);
        let x = ee::add(centre[0], ee::from_int(rng.below(1000) as i32));
        let y = ee::add(centre[1], ee::from_int(rng.below(1000) as i32));
        FieldFirefly::init(hacked, pattern, [x, y, 0, 0], rng)
    }

    /// `SetBasePosition(at)` (the base raised by `fieldrand(200) + 100`)
    /// then `Init`.
    fn init(hacked: bool, pattern: u32, at: V4, rng: &mut Rng) -> FieldFirefly {
        let mut base = at;
        base[2] = ee::add(base[2], ee::from_int(rng.below(200) as i32 + 100));
        // The hacked kind's trail: every sixth frame, up to sixteen.
        let (looks, period, timer, trail_period, trail_max) = if hacked {
            let s = 0x4019_999a;
            (Looks { name: "EFF_sfzdigi0", scale: s, trail: "EFF_sfzdigi1", trail_scale: [s; 16] }, 600, 6, 6, 16)
        } else {
            let scales = trail_scales(0x3f19_999a, 0x3d23_d70a);
            (Looks { name: "EFF_sfpfir_1", scale: ONE, trail: "EFF_sfpfir_2", trail_scale: scales }, 300, 1, 1, 8)
        };
        let xs: [F; 5] = if rng.below(100) >= 51 {
            [0, 0x4416_0000, 0x4496_0000, 0x4416_0000, 0]
        } else {
            [0x4496_0000, 0x4416_0000, 0, 0x4416_0000, 0x4496_0000]
        };
        let mut ys: Vec<F> = (0..4).map(|_| ee::from_int(rng.below(1200) as i32)).collect();
        let mut zs: Vec<F> = (0..4).map(|_| ee::from_int(rng.below(350) as i32)).collect();
        ys[3] = ys[0];
        zs[3] = zs[0];
        FieldFirefly {
            looks,
            hacked,
            transparency: 0,
            pattern,
            sparks: [None, None, None],
            count: 0,
            trail_len: 0,
            trail_timer: timer,
            trail_period,
            trail_max,
            pos: [0; 4],
            base,
            last: base,
            trail: [[0; 4]; 16],
            splines: [Spline::new(&xs, period), Spline::new(&ys, period), Spline::new(&zs, period)],
        }
    }

    /// `FIREFLY::Move` (0x005b30a0). `type9` names the sparks' sprite.
    pub fn step(&mut self, env: &Env, field_type: u32, rng: &mut Rng) {
        if ee::lt(self.transparency, ONE) {
            self.transparency = ee::add(self.transparency, 0x3dcc_cccd);
        }
        for k in 0..3 {
            let v = self.splines[k].step();
            self.pos[k] = ee::add(self.base[k], v);
        }
        self.trail_timer -= 1;
        if self.trail_timer == 0 {
            for i in (1..self.trail_max).rev() {
                self.trail[i] = self.trail[i - 1];
            }
            self.trail[0] = self.last;
            if self.trail_len < self.trail_max {
                self.trail_len += 1;
            }
            self.trail_timer = self.trail_period;
        }
        let vel = ee::vsub(self.last, self.pos);
        self.last = self.pos;
        self.pattern += 1;
        if self.pattern >= 100 {
            self.pattern = 0;
        }
        let d = env.w2p(self.pos);
        let s = ee::add(ee::mul(d[0], d[0]), ee::mul(d[1], d[1]));
        let dist = (f64::from(ee::f(s)).sqrt() as f32).to_bits();
        if !ee::le(dist, 0x453b_8000) {
            self.transparency = 0;
            self.base[0] = ee::sub(ee::from_int(rng.below(5000) as i32), 0x451c_4000);
            self.base[1] = ee::sub(ee::from_int(rng.below(5000) as i32), 0x451c_4000);
            self.base = env.p2w(self.base);
            self.base[2] = if field_type == 4 { 0 } else { env.height(self.base[0], self.base[1]) };
        }
        if self.hacked {
            return;
        }
        if self.count < 3 && rng.below(100) >= 81 {
            let k = self.sparks.iter().position(Option::is_none).unwrap_or(3);
            self.count += 1;
            let child = Firefly::new(rng);
            let s = if rng.below(100) >= 41 {
                let mut v = ee::normalize(vel);
                for c in v.iter_mut().take(3) {
                    let r = ee::sub(ee::div(ee::from_int(rng.below(100) as i32), 0x42c8_0000), 0x3f00_0000);
                    *c = ee::add(*c, r);
                }
                spark(child, rng, self.pos, v, None, 0x3f00_0000, 0x3fc0_0000)
            } else {
                let mut v = [0; 4];
                for c in v.iter_mut().take(3) {
                    *c = ee::sub(ee::div(ee::from_int(rng.below(200) as i32), 0x42c8_0000), ONE);
                }
                let life = rng.below(5) as i32 + 15;
                spark(child, rng, self.pos, v, Some(life), 0x3f33_3333, 0x40a0_0000)
            };
            if k < 3 {
                self.sparks[k] = Some(Box::new(s));
            }
        }
        let fly = fly(env);
        for k in 0..3 {
            let Some(s) = self.sparks[k].as_mut() else { continue };
            s.step_in(&fly, rng);
            if s.scale == 0x3f33_3333 {
                s.pos = ee::vadd(self.pos, s.moved);
            }
            if s.life == 0 {
                self.sparks[k] = None;
                self.count -= 1;
            }
        }
    }

    /// `FIREFLY::Draw` (0x005b3a60), on layer 3; the hacked trail's
    /// patterns draw from `fieldrand`.
    pub fn draw(&mut self, env: &Env, spark_name: &'static str, rng: &mut Rng, out: &mut Vec<Op>) {
        if self.base[2] == 0xc3fa_0000 {
            return;
        }
        self.pos = env.p2w(env.w2p(self.pos));
        let pattern = if self.hacked { 0 } else { self.pattern as u16 };
        out.push(sprite(self.looks.name, self.pos, pattern, self.looks.scale, self.transparency, false));
        for i in 0..self.trail_len.min(16) {
            self.trail[i] = env.p2w(env.w2p(self.trail[i]));
            let pattern = if self.hacked { rng.below(15) as u16 } else { self.pattern as u16 };
            out.push(sprite(self.looks.trail, self.trail[i], pattern, self.looks.trail_scale[i], ONE, false));
        }
        if self.count != 0 {
            let fly = fly(env);
            for s in self.sparks.iter_mut().flatten() {
                let mut draws = Vec::new();
                s.draw_in(&fly, &mut draws);
                for d in draws {
                    out.push(sprite(spark_name, d.pos, d.pattern, d.scale, d.transparency, true));
                }
            }
        }
    }
}

/// A spark as `FIREFLY::Move` makes it: `new FIREFLY2` (drawn before the
/// choice), then `Init(stream, name, pos, v[, life])` and the speed and
/// scale it then sets.
fn spark(mut s: Firefly, rng: &mut Rng, pos: V4, v: V4, life: Option<i32>, scale: F, speed: F) -> Firefly {
    match life {
        None => s.life = rng.below(30) as i32 + 60,
        Some(l) => {
            s.life = l;
            s.base = pos;
        }
    }
    s.spark = true;
    s.pos = pos;
    s.vel = v;
    s.scale = scale;
    s.speed = speed;
    s
}

fn fly<'a>(env: &'a Env<'a>) -> Fly<'a> {
    Fly { player: env.player, bounds: env.bounds, eye: env.eye, rot: env.cam_rot(), height: env.height }
}

fn sprite(name: &'static str, pos: V4, pattern: u16, scale: F, transparency: F, fog: bool) -> Op {
    let file = crate::field_ambient::EFF_FILE;
    Op::Sprite(Sprite {
        layer: 3,
        file,
        name,
        pos,
        pattern,
        scale: [scale; 2],
        rotate: 0,
        transparency,
        fog,
        ztest: true,
        colour: None,
        clut: None,
    })
}
