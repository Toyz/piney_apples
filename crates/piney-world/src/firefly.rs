//! `FIREFLY2` (gcmn firefly.cpp, 0x120 bytes): a firefly that wanders about a
//! base point near the player and throws off sparks, each a `ccEff` sprite.
//! The boss arenas make 54 ([`crate::evarea_b0`]); `WORLD::Generate` makes
//! fifteen of type 1 for field type 9 (`SetDirc`); the fields' `FIREFLY`
//! makes type 0 sparks of its own ([`crate::field_firefly`]). A type-0
//! firefly that lives out its life starts again near the world's origin, by
//! as much as it stood from the player: `SetBasePosition` takes the relative
//! place as the new base (docs/engine/evarea.md, docs/engine/field.md).

use piney_data::dungeon::Rng;

use crate::ee::{self, F, ONE, V4};
use crate::field_area::{ground_dist, p2w, w2p};

/// 1000.0: how far across from the player a firefly moves and is drawn.
const REACH: F = 0x447a_0000;
/// 0.1, a step of the fade.
const TENTH: F = 0x3dcc_cccd;
/// 100.0.
const HUNDRED: F = 0x42c8_0000;
/// 0.5, 10.0, 2.0, -1.0.
const HALF: F = 0x3f00_0000;
const TEN: F = 0x4120_0000;
const TWO: F = 0x4000_0000;
const MINUS_ONE: F = 0xbf80_0000;
/// The ccEff's scale: a firefly 1.5, a drifting spark 0.75, a quick one 1.2.
const SCALE: F = 0x3fc0_0000;
const DRIFT_SCALE: F = 0x3f40_0000;
const QUICK_SCALE: F = 0x3f99_999a;
/// A spark's first transparency, 0.2.
const SPARK_ALPHA: F = 0x3e4c_cccd;

/// What a firefly reads of the world: the player's place and the map's
/// bounds (`ccTransPosW2P` / `P2W`), and for type 1 the camera and the
/// ground.
pub struct Fly<'a> {
    /// The disc's volume: its code's square roots.
    pub volume: piney_data::volume::Volume,
    pub player: V4,
    pub bounds: [F; 4],
    /// `cameraGetPos(camID)`, and the camera's turn (`cameraGetRot2(1)`
    /// in the eye view, else `cameraGetRot(1)`).
    pub eye: V4,
    pub rot: V4,
    /// `WORLD_MAN::GetHeight`.
    pub height: &'a dyn Fn(F, F) -> F,
}

impl Fly<'_> {
    /// The arena's: its bounds, and no camera or ground (type 0 reads
    /// neither).
    fn arena(volume: piney_data::volume::Volume, player: V4) -> Fly<'static> {
        Fly { volume, player, bounds: crate::evarea::BOUNDS, eye: [0; 4], rot: [0; 4], height: &|_, _| 0 }
    }
}

/// One `FIREFLY2`.
#[derive(Clone, Debug, PartialEq)]
pub struct Firefly {
    /// +0: 0 wanders about its base, 1 roams about the camera (field type
    /// 9).
    pub kind: u32,
    /// +0x9c: a spark of another firefly's.
    pub spark: bool,
    /// The ccEff's scale (+0x08's +0x20 and +0x24).
    pub scale: F,
    /// +0x70 velocity, +0x80 transparency, +0x84 speed, +0x88 the turn's
    /// spread (15.0).
    pub vel: V4,
    pub transparency: F,
    pub speed: F,
    pub spread: F,
    /// +0x90 the ccEff's pattern, 0-99.
    pub pattern: u32,
    /// +0x98 sparks, +0x1c the three slots.
    pub count: i32,
    pub sparks: [Option<Box<Firefly>>; 3],
    /// +0xa0 frames to the next turn, +0xa4 life.
    pub timer: i32,
    pub life: i32,
    /// +0xb0 position, +0x100 base, +0x110 how far it has moved.
    pub pos: V4,
    pub base: V4,
    pub moved: V4,
}

/// What one firefly's `Draw` sends: `ccEff::Draw(pos, pattern)` with the
/// transparency and scale the ccEff holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireflyDraw {
    pub pos: V4,
    pub pattern: u16,
    pub transparency: F,
    pub scale: F,
}

fn from_rand(r: u32) -> F {
    ee::from_int(r as i32)
}

/// `(fieldrand(2000) - 1000) / 1000`.
fn wander(rng: &mut Rng) -> F {
    ee::div(ee::from_int(rng.below(2000) as i32 - 1000), 0x447a_0000)
}

/// 2000.0: how far on the ground from the camera a type 1 firefly roams.
const ROAM: F = 0x44fa_0000;

impl Firefly {
    /// `new FIREFLY2`.
    pub fn new(rng: &mut Rng) -> Firefly {
        let timer = rng.below(20) as i32 + 10;
        let pattern = rng.below(100);
        Firefly {
            kind: 0,
            spark: false,
            scale: ONE,
            vel: [0, 0, ONE, 0],
            transparency: 0,
            speed: ONE,
            spread: 0x4170_0000,
            pattern,
            count: 0,
            sparks: [None, None, None],
            timer,
            life: 0,
            pos: [0; 4],
            base: [0; 4],
            moved: [0; 4],
        }
    }

    /// `new FIREFLY2`, `Init(stream, name)` and `SetBasePosition(at)`, as
    /// the arena makes each.
    pub fn arena(volume: piney_data::volume::Volume, rng: &mut Rng, at: V4) -> Firefly {
        let mut f = Firefly::new(rng);
        f.life = rng.below(200) as i32 + 220;
        f.scale = SCALE;
        f.set_base(rng, at, &Fly::arena(volume, at));
        f
    }

    /// Field type 9's (`WORLD::Generate`): `new FIREFLY2`,
    /// `Init(field_eff, "EFF_sfzfir1")`, `SetDirc(dirc)` and
    /// `SetBasePosition(base)`.
    pub fn roamer(rng: &mut Rng, dirc: V4, base: V4, fly: &Fly) -> Firefly {
        let mut f = Firefly::new(rng);
        f.life = rng.below(200) as i32 + 220;
        f.scale = SCALE;
        f.kind = 1;
        f.speed = TWO;
        f.vel = dirc;
        f.set_base(rng, base, fly);
        f
    }

    /// A spark made by another's `Init(stream, name, pos, v)` (life
    /// 60-89) or, with `life`, `Init(stream, name, pos, v, life)` (speed 2,
    /// its base at `pos`), with the scale and speed its maker then sets.
    pub fn spark_of(rng: &mut Rng, pos: V4, v: V4, life: Option<i32>, scale: F, speed: F) -> Firefly {
        let mut s = Firefly::new(rng);
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

    /// `SetBasePosition(p)` (0x005b49f0).
    fn set_base(&mut self, rng: &mut Rng, p: V4, fly: &Fly) {
        self.base = p;
        let (n, off) = if self.kind == 0 { (500, 250) } else { (3000, 1500) };
        self.pos[0] = ee::from_int(rng.below(n) as i32 - off);
        self.pos[1] = ee::from_int(rng.below(n) as i32 - off);
        self.pos = ee::vadd(self.pos, self.base);
        self.pos[2] = from_rand(rng.below(100));
        if self.kind == 1 {
            let h = (fly.height)(self.pos[0], self.pos[1]);
            self.pos[2] = ee::add(h, from_rand(rng.below(300)));
        }
        self.pos[3] = ONE;
    }

    /// `SetBasePosition2(p)` (0x005b4ba0).
    fn set_base2(&mut self, rng: &mut Rng, p: V4, fly: &Fly) {
        self.base = p;
        if self.kind == 0 {
            self.pos[0] = ee::from_int(rng.below(500) as i32 - 250);
            self.pos[1] = ee::from_int(rng.below(500) as i32 - 250);
        } else {
            let mut rot = fly.rot;
            self.pos[0] = ee::from_int(rng.below(3000) as i32 - 1500);
            self.pos[1] = ee::from_int(rng.below(3000) as i32 - 1500);
            self.pos[2] = ee::from_int(rng.below(300) as i32);
            rot[0] = 0;
            rot[1] = 0;
            let m = piney_data::anim::rot_bits([rot[0], rot[1], rot[2]]);
            self.pos = ee::apply(&m, self.pos);
            self.pos[2] = (fly.height)(self.pos[0], self.pos[1]);
        }
        self.pos = ee::vadd(self.pos, self.base);
        self.pos[3] = ONE;
    }

    /// `FIREFLY2::Draw` in the arena, into `out`.
    pub fn draw(&mut self, volume: piney_data::volume::Volume, player: V4, out: &mut Vec<FireflyDraw>) {
        self.draw_in(&Fly::arena(volume, player), out);
    }

    /// `FIREFLY2::Draw` (0x005b5eb0), into `out` (the firefly, then its
    /// sparks).
    pub fn draw_in(&mut self, fly: &Fly, out: &mut Vec<FireflyDraw>) {
        if !self.spark && self.life < 100 {
            self.transparency = ee::mul(TENTH, ee::from_int(self.life));
        }
        let rel = w2p(self.pos, fly.player, fly.bounds);
        if !ee::le(ground_dist(fly.volume, rel), REACH) {
            return;
        }
        out.push(FireflyDraw {
            pos: p2w(rel, fly.player, fly.bounds),
            pattern: self.pattern as u16,
            transparency: self.transparency,
            scale: self.scale,
        });
        if self.count != 0 {
            for s in self.sparks.iter_mut().flatten() {
                s.draw_in(fly, out);
            }
        }
    }

    /// The fade toward 1 by a tenth a frame.
    fn brighten(&mut self) {
        if ee::lt(self.transparency, ONE) {
            self.transparency = ee::add(self.transparency, TENTH);
            if !ee::le(self.transparency, ONE) {
                self.transparency = ONE;
            }
        }
    }

    /// `FIREFLY2::Move` in the arena (type 0).
    pub fn step(&mut self, volume: piney_data::volume::Volume, player: V4, rng: &mut Rng) {
        self.step_in(&Fly::arena(volume, player), rng);
    }

    /// `FIREFLY2::Move` (0x005b5050).
    pub fn step_in(&mut self, fly: &Fly, rng: &mut Rng) {
        let rel = w2p(self.pos, fly.player, fly.bounds);
        // Where SetBasePosition2 takes a type 1 back to at life's end.
        let mut back = fly.eye;
        if self.kind == 0 {
            if !ee::le(ground_dist(fly.volume, rel), REACH) {
                return;
            }
        } else if !ee::le(piney_battle::enemy_ai::get_dist_on(fly.volume, fly.eye, self.pos), ROAM) {
            self.transparency = 0;
            back = fly.player;
            self.set_base2(rng, back, fly);
            self.vel[0] = wander(rng);
            self.vel[1] = wander(rng);
            self.vel[2] = wander(rng);
        }
        if self.spark {
            self.brighten();
        } else {
            if self.life > 100 {
                self.brighten();
            }
            if self.life < 100 {
                self.transparency = ee::div(ee::from_int(self.life), HUNDRED);
            }
        }
        self.life -= 1;
        for k in 0..3 {
            self.pos[k] = ee::add(self.pos[k], ee::mul(self.vel[k], self.speed));
        }
        for k in 0..3 {
            self.moved[k] = ee::add(self.moved[k], ee::mul(self.vel[k], self.speed));
        }
        self.pattern += 1;
        if self.pattern >= 100 {
            self.pattern = 0;
        }
        if !self.spark && self.count < 3 && rng.below(100) >= 61 {
            self.spawn(rng);
        }
        if self.count != 0 {
            for k in 0..3 {
                let Some(s) = self.sparks[k].as_mut() else { continue };
                s.step_in(fly, rng);
                if s.life == 0 {
                    self.sparks[k] = None;
                    self.count -= 1;
                }
            }
        }
        if self.spark {
            return;
        }
        self.timer -= 1;
        if self.timer == 0 {
            if self.kind == 0 {
                self.timer = rng.below(20) as i32 + 10;
                let n = ee::to_int(self.spread) as u32;
                let half = ee::div(self.spread, TWO);
                self.vel[0] = ee::div(ee::sub(from_rand(rng.below(n)), half), TEN);
                self.vel[1] = ee::div(ee::sub(from_rand(rng.below(n)), half), TEN);
            } else {
                self.timer = rng.below(60) as i32 + 60;
                self.vel[0] = wander(rng);
                self.vel[1] = wander(rng);
                self.vel[2] = wander(rng);
            }
        }
        if self.life == 0 {
            if self.kind == 0 {
                self.set_base(rng, rel, fly);
            } else {
                self.set_base2(rng, back, fly);
            }
            self.life = rng.below(200) as i32 + 220;
        }
    }

    /// A new spark in the first free slot: drifting back from the
    /// firefly's way, or a quick one about it.
    fn spawn(&mut self, rng: &mut Rng) {
        let k = self.sparks.iter().position(Option::is_none).unwrap_or(2);
        self.count += 1;
        let mut s = Firefly::new(rng);
        let mut v = [0; 4];
        if rng.below(100) >= 61 {
            let dx = ee::sub(ee::div(from_rand(rng.below(100)), HUNDRED), HALF);
            for (d, &x) in v.iter_mut().zip(&self.vel[..3]) {
                *d = ee::mul(MINUS_ONE, x);
            }
            v[0] = ee::add(v[0], dx);
            v[1] = ee::add(v[1], ee::sub(ee::div(from_rand(rng.below(100)), HUNDRED), HALF));
            v[2] = ee::add(v[2], ee::sub(ee::div(from_rand(rng.below(100)), HUNDRED), HALF));
            s.life = rng.below(30) as i32 + 60;
            s.scale = DRIFT_SCALE;
        } else {
            for d in v.iter_mut().take(3) {
                *d = ee::sub(ee::div(from_rand(rng.below(200)), HUNDRED), ONE);
            }
            s.life = rng.below(5) as i32 + 5;
            s.speed = TWO;
            s.base = self.pos;
            s.scale = QUICK_SCALE;
        }
        s.spark = true;
        s.pos = self.pos;
        s.vel = v;
        if self.kind == 1 {
            s.kind = 1;
        }
        s.transparency = SPARK_ALPHA;
        self.sparks[k] = Some(Box::new(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_data::volume::Volume;

    /// A firefly out of reach neither moves nor draws; one in reach fades
    /// in, moves and in time throws a spark.
    #[test]
    fn reach_and_sparks() {
        let mut rng = Rng::new(7);
        let far = [ee::from_int(5000), 0, 0, ONE];
        let mut f = Firefly::arena(Volume::Inf, &mut rng, far);
        let before = f.clone();
        let mut out = Vec::new();
        f.draw(Volume::Inf, [0, 0, 0, ONE], &mut out);
        f.step(Volume::Inf, [0, 0, 0, ONE], &mut rng);
        assert!(out.is_empty());
        assert_eq!(f, before);
        let mut f = Firefly::arena(Volume::Inf, &mut rng, [0, 0, 0, ONE]);
        for _ in 0..40 {
            f.step(Volume::Inf, [0, 0, 0, ONE], &mut rng);
        }
        assert!(f.count > 0);
        assert!(ee::le(f.transparency, ONE));
        f.draw(Volume::Inf, [0, 0, 0, ONE], &mut out);
        assert_eq!(out.len(), 1 + f.count as usize);
    }
}
