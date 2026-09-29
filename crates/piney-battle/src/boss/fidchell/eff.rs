//! Fidchell's spells as the rules keep them (bosseff.cpp): what each
//! `Draw` changes that its boss waits on or reads, the `ccRand` it draws,
//! the sounds it plays and the pictures it asks for. The meteors
//! (`ccBossEffMeteoSworm`, `ccBossEffMeteo2`), the thunders
//! (`ccBossEffThunderStorm`, `ccBossEffThunder2`) and the rock towers
//! (`ccBossEffRockTower`), Outbreak's gcmn.

use piney_data::field::ee;

use super::Pic;
use crate::boss::{Cx, Out, VF0};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// 1/15.
const FIFTEENTH: F = 0x3d88_8889;

/// `effSkillStart`'s controller (-12): the boss frames until its `endFlag`
/// is seen (its life of 7, then the frame that ends it). `OnThinkSkill`
/// waits 30 frames as well, so only a life under 30 matters.
pub const SKILL_START_LIFE: i32 = 8;

/// One of Fidchell's spells in its effect slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fx {
    Meteo(MeteoSworm),
    Storm(ThunderStorm),
    Tower(RockTower),
}

// --- the meteors ---------------------------------------------------------------------------

/// `ccBossEffMeteo2` (OUT gcmn 0x0047da10-0x0047e0c8): one meteor's fall
/// in a straight line from `sp` toward `ep`, gathering speed, until it
/// is at or under `ep`'s height.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meteor {
    /// `METEO_T`: `enabled`, `interval`, `bSEOn`.
    pub enabled: bool,
    pub interval: i32,
    pub se_on: bool,
    pub pos: V4,
    pub end_height: F,
    pub velocity: V4,
    pub accel: V4,
    pub dirc: V4,
    pub end_flag: bool,
}

impl Meteor {
    /// `Init(sp, ep, v, 0, accel)` (0x0047dab0): heading and pitch from
    /// `sp` to `ep`, the speed `v` along them, the pull `accel` too.
    fn init(cx: &Cx, sp: V4, ep: V4, v: F, accel: F) -> Meteor {
        let a = crate::boss::w2p(cx, sp);
        let b = crate::boss::w2p(cx, ep);
        let yaw = get_dirc(a, b);
        let d = super::dist(cx, a, b);
        let mut pitch = piney_data::libm::atan2f(ee::sub(a[2], b[2]), d);
        if ee::lt(pitch, NEG_PI) {
            pitch = ee::add(pitch, TWO_PI);
        }
        if !ee::le(pitch, PI) {
            pitch = ee::sub(pitch, TWO_PI);
        }
        let m = geom::rot_matrix_y(&geom::unit_matrix(), pitch);
        let m = geom::rot_matrix_z(&m, yaw);
        let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
        let velocity = geom::apply_matrix(&m, [v, 0, 0, ONE]);
        let mut acc = VF0;
        if !ee::le(accel, 0) {
            acc[0] = accel;
            acc = geom::apply_matrix(&m, acc);
        }
        Meteor {
            enabled: false,
            interval: 0,
            se_on: false,
            pos: sp,
            end_height: ep[2],
            velocity,
            accel: acc,
            dirc: [0, pitch, yaw, ONE],
            end_flag: false,
        }
    }

    /// `Move` (0x0047df80): a step, the speed gathered; at the ground the
    /// landing's dust, bursts and sounds.
    fn step(&mut self, cx: &mut Cx) {
        let p = geom::vadd(crate::boss::w2p(cx, self.pos), self.velocity);
        self.pos = crate::boss::p2w(cx, p);
        self.velocity = geom::vadd(self.velocity, self.accel);
        if ee::le(self.pos[2], self.end_height) {
            self.end_flag = true;
            let at = [self.pos[0], self.pos[1], self.end_height, self.pos[3]];
            cx.out(Out::Fidchell(Pic::MeteorLand { pos: at }));
            cx.out(Out::Se3d { se: 35, pos: at });
            cx.out(Out::Se3d { se: 40, pos: at });
        }
    }
}

/// `ccBossEffMeteoSworm` (OUT gcmn 0x0047d290, `Draw` 0x0047d800): `n`
/// meteors from `sp` to points round `ep` within `radius`, each after its
/// own wait; the last, to `ep` itself, carries the light and the boss's
/// camera view (`m_syncPos`, 200 over it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeteoSworm {
    pub meteors: Vec<Meteor>,
    pub light: bool,
}

impl MeteoSworm {
    /// The constructor: for all but the last a point `ccRandF(radius)` off
    /// `ep` at `ccRandF(pi)`, the pull `2 + |ccRandF(2)|` (in doubles) and
    /// the wait `(ccRand() & 31) + 31`; the last's pull 3 and wait 31.
    pub fn new(cx: &mut Cx, sp: V4, ep: V4, n: usize, radius: F, v: F) -> MeteoSworm {
        let mut meteors = Vec::with_capacity(n);
        for _ in 0..n.saturating_sub(1) {
            let e = crate::boss::w2p(cx, ep);
            let turn = rand_f(cx.cc, PI);
            let r = rand_f(cx.cc, radius);
            let m = geom::trans_matrix(&geom::unit_matrix(), [r, 0, 0, ONE]);
            let m = geom::rot_matrix_z(&m, turn);
            let off = geom::apply_matrix(&m, VF0);
            let end = crate::boss::p2w(cx, geom::vadd(e, off));
            let pull = rand_f(cx.cc, 0x4000_0000);
            let pull = (2.0 + f64::from(f32::from_bits(pull)).abs()) as f32;
            let mut m = Meteor::init(cx, sp, end, v, pull.to_bits());
            m.interval = (cx.cc.rand() & 31) + 31;
            meteors.push(m);
        }
        if n > 0 {
            let mut m = Meteor::init(cx, sp, ep, v, 0x4040_0000);
            m.interval = 31;
            meteors.push(m);
        }
        MeteoSworm { meteors, light: true }
    }

    /// One `Draw`: (still enabled, the boss's camera view when the last
    /// meteor moved).
    pub fn draw(&mut self, cx: &mut Cx) -> (bool, Option<V4>) {
        let n = self.meteors.len();
        let mut all_down = true;
        let mut waiting = false;
        let mut sync = None;
        for k in 0..n {
            let m = &mut self.meteors[k];
            let iv = m.interval;
            m.interval -= 1;
            if iv > 0 {
                waiting = true;
                continue;
            }
            m.enabled = true;
            if !m.end_flag {
                all_down = false;
                m.step(cx);
                if k == n - 1 {
                    let mut v = m.pos;
                    v[2] = ee::add(v[2], 0x4348_0000);
                    sync = Some(v);
                }
                if !m.se_on {
                    m.se_on = true;
                    if k & 7 == 0 {
                        cx.out(Out::Se { se: 57 });
                    }
                }
            } else if k == n - 1 {
                self.light = false;
            }
        }
        (waiting || !all_down, sync)
    }
}

// --- the thunders --------------------------------------------------------------------------

/// `ccBossEffThunder2` (OUT gcmn 0x0047e660, `Draw` 0x0047e9a0, `Shock`
/// 0x0047ee60) as far as its storm reads it: a bolt of nine segments grown
/// one a `Draw`, then the strike, then a 15-frame fade and 31 frames more.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Thunder {
    pub enabled: bool,
    pub pos: V4,
    pub anm_index: i32,
    pub shock: i32,
    pub transparency: F,
    pub act_count: i32,
}

impl Thunder {
    /// The constructor: ten `ccRand()` for its segments' patterns.
    fn new(cx: &mut Cx, pos: V4) -> Thunder {
        for _ in 0..10 {
            let _ = cx.cc.rand();
        }
        Thunder { enabled: true, pos, anm_index: 0, shock: 0, transparency: ONE, act_count: 0 }
    }

    /// One `Draw`: eight points of the bolt drawn anew (two `ccRandF`
    /// each), a `ccRand()` a segment shown.
    fn draw(&mut self, cx: &mut Cx) {
        for _ in 0..8 {
            let _ = rand_f(cx.cc, 0x42c8_0000);
            let _ = rand_f(cx.cc, PI);
        }
        let mut s0 = self.anm_index;
        for _ in 0..s0 {
            let _ = cx.cc.rand();
        }
        if s0 != 9 {
            s0 += 1;
            if s0 == 9 {
                self.shock = 1;
            }
        }
        if self.shock == 1 {
            self.strike(cx);
            self.shock = 2;
        } else if self.shock >= 2 {
            let mut f = ee::sub(self.transparency, FIFTEENTH);
            if ee::lt(f, 0) {
                let c = self.act_count;
                self.act_count += 1;
                if c == 30 {
                    self.enabled = false;
                }
                f = 0;
            }
            self.transparency = f;
            self.shock = 3;
        }
        self.anm_index = s0;
    }

    /// `Shock`: the flare, the rocks, five bursts: sixteen draws.
    fn strike(&mut self, cx: &mut Cx) {
        let _ = rand_f(cx.cc, PI);
        for _ in 0..5 {
            let _ = rand_f(cx.cc, 0x4120_0000);
            let _ = rand_f(cx.cc, 0x4120_0000);
            let _ = cx.cc.rand();
        }
        let mut at = self.pos;
        at[2] = 0x4120_0000;
        cx.out(Out::Fidchell(Pic::Shock { pos: at }));
    }
}

/// `ccBossEffThunderStorm` (OUT gcmn 0x0047efd0, `Draw` 0x0047f290): `num`
/// thunders `radius` round a place at `ccRandF(pi)`, three at a time five
/// frames apart; once the first is gone a magic square and five more
/// rumbles, and it ends 85 frames on once all are gone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThunderStorm {
    pub pos: V4,
    /// `THUNDER_T`-like rows: the bolt, its state (0 waiting, 1 on, -1
    /// gone) and its wait.
    pub bolts: Vec<(Thunder, i8, i32)>,
    pub act_count: i32,
    pub act_proccess: i32,
}

impl ThunderStorm {
    /// `ccBossEffThunderStormCreate(pos, radius, num)` (0x00489fa0): its SE
    /// first, then the constructor.
    pub fn new(cx: &mut Cx, pos: V4, radius: F, num: usize) -> ThunderStorm {
        cx.out(Out::SeNote { se: 41, note: 67 });
        let mut bolts = Vec::with_capacity(num);
        for k in 0..num {
            let turn = rand_f(cx.cc, PI);
            let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
            let off = geom::apply_matrix(&m, [radius, 0, 0, ONE]);
            let at = geom::vadd(pos, off);
            let t = Thunder::new(cx, at);
            bolts.push((t, 0, (k as i32 / 3) * 5 + 15));
        }
        ThunderStorm { pos, bolts, act_count: 0, act_proccess: 0 }
    }

    /// One `Draw`: (still enabled, where it makes a magic square).
    pub fn draw(&mut self, cx: &mut Cx) -> (bool, Option<V4>) {
        let mut all_gone = true;
        let mut square = None;
        for k in 0..self.bolts.len() {
            let (t, state, wait) = &mut self.bolts[k];
            if *state == 0 {
                let w = *wait;
                *wait -= 1;
                if w == 0 {
                    *state = 1;
                    cx.out(Out::Se { se: 68 });
                    if k & 1 != 0 {
                        cx.out(Out::SeNote { se: 41, note: 67 });
                    }
                }
                all_gone = false;
            } else if *state == 1 {
                all_gone = false;
                if t.enabled {
                    t.draw(cx);
                    if k & 1 != 0 && t.shock == 2 {
                        cx.out(Out::Se { se: 35 });
                    }
                } else {
                    *state = -1;
                    if k == 0 {
                        square = Some(self.pos);
                        self.act_proccess += 1;
                    }
                }
            }
        }
        let mut alive = true;
        if self.bolts.first().is_some_and(|b| b.1 == -1) {
            let c = self.act_count;
            self.act_count += 1;
            if matches!(c, 57 | 48 | 39 | 23 | 4) {
                self.act_proccess += 1;
                let r = (cx.cc.rand() & 7).abs();
                cx.out(Out::SeNote { se: 68, note: (63 - r) as u8 });
            } else if all_gone && self.act_count >= 85 {
                alive = false;
            }
        }
        (alive, square)
    }
}

// --- the rock towers -----------------------------------------------------------------------

/// A `TOWER_T`: where it rises, its state (0 waiting, 1 rising, 2 fading,
/// 3 gone), its wait and fade.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tower {
    pub pos: V4,
    pub scale: F,
    pub dirc: V4,
    pub transparency: F,
    pub interval: i32,
    pub status: i32,
}

/// `ccBossEffRockTower` (OUT gcmn 0x0047fcd0, `Draw` 0x004800f0): `n`
/// towers of rock within 400 of a place, each out of the ground from
/// 1000 under at 33.3 a frame after its wait, then fading.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RockTower {
    pub towers: Vec<Tower>,
    pub count: i32,
    pub shake: V4,
}

impl RockTower {
    /// The constructor: four draws a tower, two for the shake.
    pub fn new(cx: &mut Cx, pos: V4, n: usize) -> RockTower {
        let mut towers = Vec::with_capacity(n);
        for k in 0..n as i32 {
            let r = rand_f(cx.cc, 0x43c8_0000);
            let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
            let turn = rand_f(cx.cc, PI);
            let m = geom::rot_matrix_z(&m, turn);
            let off = geom::apply_matrix(&m, [r, 0, 0, ONE]);
            let mut at = geom::vadd(pos, off);
            at[2] = 0xc47a_0000;
            let s = ee::add(ONE, rand_f(cx.cc, 0x3f00_0000));
            let dirc = [0, 0, rand_f(cx.cc, PI), ONE];
            towers.push(Tower { pos: at, scale: s, dirc, transparency: ONE, interval: ((k * 5) >> 2) + 15, status: 0 });
        }
        let mut shake = VF0;
        shake[0] = ee::add(0x4248_0000, rand_f(cx.cc, 0x42c8_0000));
        shake[2] = ee::add(0x4248_0000, rand_f(cx.cc, 0x42c8_0000));
        RockTower { towers, count: 0, shake }
    }

    /// One `Draw`: false once every tower is gone.
    pub fn draw(&mut self, cx: &mut Cx) -> bool {
        let mut gone = true;
        for t in &mut self.towers {
            match t.status {
                0 => {
                    let w = t.interval;
                    t.interval -= 1;
                    if w == 0 {
                        t.status += 1;
                    }
                    gone = false;
                }
                1 => {
                    t.pos[2] = ee::add(t.pos[2], 0x4205_5555);
                    if !ee::le(t.pos[2], 0xc2c8_0000) {
                        t.pos[2] = 0xc2c8_0000;
                        t.status += 1;
                        cx.out(Out::Fidchell(Pic::Smoke { pos: t.pos, vel: VF0 }));
                        let rot = [geom::deg2rad(-16384), 0, rand_f(cx.cc, PI), ONE];
                        cx.out(Out::Fidchell(Pic::Radiate { pos: t.pos, rot }));
                    }
                    gone = false;
                }
                2 => {
                    let mut f = ee::sub(t.transparency, 0x3d88_8889);
                    if ee::lt(f, 0) {
                        t.status += 1;
                        f = 0;
                    }
                    t.transparency = f;
                    gone = false;
                }
                _ => {}
            }
        }
        let c = self.count;
        self.count += 1;
        if c >= 30 && self.count % 12 == 0 {
            let r = cx.cc.rand().abs();
            cx.out(Out::SeNote { se: 56, note: ((r & 7) + 60) as u8 });
        }
        !gone
    }
}
