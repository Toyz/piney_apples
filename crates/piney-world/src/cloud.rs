//! `CLOUD` (gcmn cloud.cpp, 0x00504380-0x00504d94): a drifting cloud or puff of
//! smoke, one `ccEff` of `town_z` a map scatters round the player and moves
//! each frame (Dun Loireag's 25, `EVENTAREA04`-`07`, `EVENTAREAB8`,
//! `ROOTTOWN04`): `Init` (0x00504860), `SetPos` (0x00504400), `Move`
//! (0x00504a10) and `Draw` (0x00504ce0), each type (0-3) with its own place,
//! speed and drift from `fieldrand` (main 0x0019c460). The rules are in
//! docs/engine/town02.md.

use piney_data::ccs::Ccs;
use piney_data::dungeon::Rng;
use piney_data::volume::Volume;

use crate::char::{p2w, w2p};
use crate::ee::{self, F, ONE, V4};

/// The Eff chunk's kind.
const EFF_CHUNK: u16 = 0x0e00;
/// `@1333` and `@1334`: the sprites of types 0, 1, 3 and of type 2.
pub const EFF_SMOKE: &str = "EFF_srzsmo1";
pub const EFF_SMOKE_2: &str = "EFF_sfsmo1";
/// The steps a frame: 0.25 degrees, 5 for type 2.
const STEP: F = 0x3e80_0000;
const STEP_2: F = 0x40a0_0000;
/// `SetPos`'s scale (2, and 0.5 for type 2) and transparency (0.8).
const SCALE: F = 0x4000_0000;
const SCALE_2: F = 0x3f00_0000;
const TRANSPARENCY: F = 0x3f4c_cccd;
/// 182.04445 (0x43360b61: 65536 / 360) and pi / 32768 (0x38c90fdb).
const DEG_TO_S16: F = 0x4336_0b61;
const S16_TO_RAD: F = 0x38c9_0fdb;
const FULL_TURN: F = 0x43b4_0000;
/// `Move`'s reach: 5000 (type 0), 3000 (type 3) and type 1's floor.
const REACH_0: F = 0x459c_4000;
const REACH_3: F = 0x453b_8000;
const FLOOR_1: F = 0xc6c3_5000;
/// `Draw`'s reach: 7000 on the ground.
const DRAW_DIST: F = 0x45da_c000;
/// `ccCheckCameraDeg(pos, 12288)`.
pub const VIEW_DEG: i16 = 12288;

/// What `CLOUD` sets on its `ccEff` (+0x20 scale, +0x28 rotate, +0x34
/// transparency), and the chunk's `patNum` (+0x30) it counts by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloudEff {
    pub scale_x: F,
    pub scale_y: F,
    pub rotate: F,
    pub transparency: F,
    pub pat_num: u16,
}

/// One `CLOUD`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cloud {
    /// +0x00 degrees, +0x04 their step a frame.
    pub angle: F,
    pub step: F,
    /// +0x08.
    pub kind: i32,
    /// +0x0c: the pattern `Draw` asks for.
    pub pattern: i32,
    /// +0x10.
    pub speed: i32,
    /// +0x20.
    pub pos: V4,
    pub eff: CloudEff,
}

/// The number of patterns (`patNum`) of the Eff chunk of `name` in `ccs`.
pub fn eff_pat_num(ccs: &Ccs, name: &str) -> Option<u16> {
    let object = ccs.find_object(name)?;
    let d = &ccs.data;
    let u32_at = |p: usize| d.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    ccs.walk().chunks.iter().filter(|c| !c.in_frames && c.kind == EFF_CHUNK).find_map(|c| {
        let p = c.payload();
        (u32_at(p)? == object).then(|| d.get(p + 14..p + 16).map(|b| u16::from_le_bytes([b[0], b[1]])))?
    })
}

/// `fptosi(v)`, then `(float)` of it as a short and times pi / 32768.
fn rotate_of(angle: F) -> F {
    let s = ee::to_int(ee::mul(DEG_TO_S16, angle)) as i16;
    ee::mul(S16_TO_RAD, ee::from_int(i32::from(s)))
}

/// `(float)(n)` of a `fieldrand` result taken as a signed int.
fn fl(n: u32) -> F {
    ee::from_int(n as i32)
}

/// `sqrt(x x + y y)` in double, as `Move` and `Draw` measure (`sqrt.s`
/// from Outbreak on, [`ee::dsqrt_on`], OUT gcmn 0x0051a7b0 `CLOUD::Move`).
fn ground_dist(volume: Volume, p: V4) -> F {
    ee::dsqrt_on(volume, ee::add(ee::mul(p[0], p[0]), ee::mul(p[1], p[1])))
}

impl Cloud {
    /// `Init(s, kind)`: the eff made from the scene file (its `patNum`),
    /// then `SetPos` with the player at `player`.
    pub fn init(kind: i32, pat_num: u16, player: V4, rng: &mut Rng) -> Cloud {
        let mut c = Cloud {
            angle: 0,
            step: if kind == 2 { STEP_2 } else { STEP },
            kind,
            pattern: 0,
            speed: 0,
            pos: [0, 0, 0, 0],
            eff: CloudEff { scale_x: ONE, scale_y: ONE, rotate: 0, transparency: ONE, pat_num },
        };
        c.set_pos(player, rng);
        c
    }

    /// `SetPos` (0x00504400).
    pub fn set_pos(&mut self, player: V4, rng: &mut Rng) {
        self.eff.scale_x = SCALE;
        self.eff.scale_y = SCALE;
        match self.kind {
            0 => {
                self.eff.transparency = TRANSPARENCY;
                self.pos[0] = ee::add(fl(rng.below(5000).wrapping_sub(2500)), player[0]);
                self.pos[1] = ee::add(fl(rng.below(5000).wrapping_sub(2500)), player[1]);
                self.pos[2] = ee::add(fl(rng.below(400).wrapping_sub(200)), player[2]);
                self.speed = rng.below(15) as i32 + 5;
            }
            1 => {
                self.pos[0] = fl(rng.below(25000).wrapping_sub(12500));
                self.pos[1] = fl(rng.below(25000).wrapping_sub(12500));
                self.pos[2] = fl(0u32.wrapping_sub(800).wrapping_sub(rng.below(300)));
                self.speed = rng.below(100) as i32 + 100;
            }
            2 => {
                self.eff.scale_x = SCALE_2;
                self.eff.scale_y = SCALE_2;
                self.pos[0] = fl(rng.below(15000).wrapping_sub(7500));
                self.pos[1] = fl(rng.below(15000).wrapping_sub(7500));
                self.pos[2] = 0;
                self.speed = rng.below(20) as i32 + 60;
            }
            3 => {
                self.eff.transparency = TRANSPARENCY;
                for (p, &pl) in self.pos[..2].iter_mut().zip(&player[..2]) {
                    let d = fl(rng.below(1000) + 500);
                    *p = if rng.below(100) >= 51 { ee::add(pl, d) } else { ee::sub(pl, d) };
                }
                self.pos[2] = ee::add(fl(rng.below(400).wrapping_sub(200)), player[2]);
                self.speed = rng.below(15) as i32 + 20;
            }
            _ => {}
        }
        // Each type's case ends on the angle; an unknown type keeps its own.
        if (0..=3).contains(&self.kind) {
            self.angle = fl(rng.below(360));
        }
        self.eff.rotate = rotate_of(self.angle);
        self.pattern = rng.below(u32::from(self.eff.pat_num).max(1)) as i32;
    }

    /// `Move` (0x00504a10) with the player at `player`; `height` is
    /// `WORLD_MAN::GetHeight(x, y)` for type 2.
    pub fn step(&mut self, volume: Volume, player: V4, rng: &mut Rng, height: &dyn Fn(F, F) -> F) {
        self.pattern += 1;
        if self.pattern == i32::from(self.eff.pat_num) {
            self.pattern = 0;
        }
        self.angle = ee::add(self.angle, self.step);
        if ee::eq(FULL_TURN, self.angle) {
            self.angle = 0;
        }
        self.eff.rotate = rotate_of(self.angle);
        let speed = ee::from_int(self.speed);
        match self.kind {
            0 | 3 => {
                self.pos[0] = ee::add(self.pos[0], speed);
                let reach = if self.kind == 0 { REACH_0 } else { REACH_3 };
                if !ee::le(ground_dist(volume, w2p(self.pos, player)), reach) {
                    self.set_pos(player, rng);
                }
            }
            1 => {
                self.pos[1] = ee::sub(self.pos[1], speed);
                if ee::lt(self.pos[1], FLOOR_1) {
                    self.set_pos(player, rng);
                }
            }
            2 => {
                self.pos[0] = ee::sub(self.pos[0], speed);
                self.pos[1] = ee::sub(self.pos[1], speed);
                let p = w2p(self.pos, player);
                let w = p2w(p, player);
                self.pos = [w[0], w[1], w[2], self.pos[3]];
                let z = height(self.pos[0], self.pos[1]);
                self.pos[2] = if ee::lt(z, 0) { 0 } else { z };
            }
            _ => {}
        }
    }

    /// `Draw` (0x00504ce0): whether `ccEff::Draw(pos, pattern)` is made,
    /// `in_view` being `ccCheckCameraDeg(pos, 12288)`.
    pub fn drawn(&self, volume: Volume, player: V4, in_view: bool) -> bool {
        in_view && ee::lt(ground_dist(volume, w2p(self.pos, player)), DRAW_DIST)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Type 0 about the player: within 2500 either way, its speed 5-19,
    /// its angle a whole degree and its rotation that angle as a short.
    #[test]
    fn type_0_about_the_player() {
        let mut rng = Rng::new(13);
        let player = [ee::k(0.0), ee::k(3500.0), ee::k(0.0), ONE];
        for _ in 0..50 {
            let c = Cloud::init(0, 16, player, &mut rng);
            assert!(ee::f(c.pos[0]).abs() <= 2500.0 && (ee::f(c.pos[1]) - 3500.0).abs() <= 2500.0);
            assert!((5..20).contains(&c.speed) && (0..16).contains(&c.pattern));
            assert_eq!(ee::f(c.angle).fract(), 0.0);
            assert_eq!(c.eff.transparency, TRANSPARENCY);
        }
        // 90 degrees: (short)(90 x 182.04445) = 16384, times pi / 32768;
        // 1 degree truncates to 182.
        assert_eq!(rotate_of(ee::k(90.0)), ee::mul(S16_TO_RAD, ee::from_int(16384)));
        assert_eq!(rotate_of(ONE), ee::mul(S16_TO_RAD, ee::from_int(182)));
    }
}
