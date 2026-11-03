//! `SetWater`'s embers (gcmn 0x005c3f30): a `SNOW` of type 3 (0x50 bytes,
//! `fire[k]` +0x54) at each `OBJ_o_magma*` dummy's place, z -100, rising
//! and starting again there; `DrawEff` (0x005cdee0) moves and draws each
//! (`SNOW::Move`, `SNOW::Draw`) on the effects' layer, its palette changed
//! by `DUNGEON::ChangeClut` in types 0 and 1.

use std::rc::Rc;

use piney_data::Result;
use piney_data::dungeon::Rng;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, RoomCx, RoomPart, StepCx};
use crate::dungeon::Rooms;
use crate::ee::{self, F, ONE, V4};
use crate::sprite::{Clut, EffSprite, EffState};

/// The dummies, the Eff (@1150) and the height a spark starts from (-100).
const MAGMA: &str = "OBJ_o_magma*";
const SPARK: &str = "EFF_o_spark_s0_";
const SPARK_Z: F = 0xc2c8_0000;
/// `SNOW(ccs, pos)` (gcmn 0x00503cf0): type 3, range 15, speed 1, the
/// Eff's scale 0.5.
const RANGE: F = 0x4170_0000;
const SPEED: F = ONE;
const HALF: F = 0x3f00_0000;
const TWO: F = 0x4000_0000;
const K10: F = 0x4120_0000;
const K100: F = 0x42c8_0000;

/// The type's palettes `ChangeClut` swaps (old, clutType 3's, clutType
/// 4's), none for most.
#[derive(Clone, Copy, Debug)]
pub struct SparksDef {
    pub clut: Option<[&'static str; 3]>,
}

/// `fieldrand(n)` (main 0x0019c460); the game divides by `n` with no
/// guard, so for 0 the port takes the step alone.
pub(crate) fn fieldrand(rng: &mut Rng, n: u32) -> u32 {
    if n == 0 { rng.advance() } else { rng.below(n) }
}

/// A `fieldrand` result as a float (the game converts it unsigned; the
/// results are small).
fn fr(rng: &mut Rng, n: u32) -> F {
    ee::from_int(fieldrand(rng, n) as i32)
}

/// One spark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spark {
    /// +0 frames to its restart, +8 frames to its next turn.
    pub life: i32,
    pub cnt: i32,
    /// +0x20 where it starts, +0x30 its place, +0x40 its step.
    pub base: V4,
    pub pos: V4,
    pub dir: V4,
    /// Its `ccEff`'s place and transparency as the last `Move` and `Draw`
    /// left them.
    pub eff_pos: V4,
    pub transparency: F,
}

impl Spark {
    /// `SNOW(ccs, base)`: `cnt = fieldrand(10) + 3`, `life = fieldrand(50) +
    /// 10`, then `calcPos3`.
    fn new(base: V4, rng: &mut Rng) -> Spark {
        let cnt = fieldrand(rng, 10) as i32 + 3;
        let life = fieldrand(rng, 50) as i32 + 10;
        let mut s = Spark { life, cnt, base, pos: [0; 4], dir: [0; 4], eff_pos: [0, 0, 0, ONE], transparency: ONE };
        s.calc_pos3(rng);
        s
    }

    /// `(fieldrand(range) - range / 2) / 10`, `fptosi` of the range.
    fn drift(rng: &mut Rng) -> F {
        let n = ee::to_int(RANGE) as u32;
        let x = fr(rng, n);
        ee::div(ee::sub(x, ee::div(RANGE, TWO)), K10)
    }

    /// `calcPos3(base, pos, dir, range, speed)` (gcmn 0x00502e20): two
    /// draws for a place about the base that `sceVu0CopyVector(pos, base)`
    /// then overwrites (the spark starts at its base, w 1); the step's x
    /// and y drift, z `(fieldrand(3) + 2) * speed`, w 1.
    fn calc_pos3(&mut self, rng: &mut Rng) {
        fieldrand(rng, 300);
        fieldrand(rng, 300);
        self.pos = self.base;
        self.pos[3] = ONE;
        self.dir[0] = Spark::drift(rng);
        self.dir[1] = Spark::drift(rng);
        self.dir[2] = ee::mul(ee::from_int(fieldrand(rng, 3) as i32 + 2), SPEED);
        self.dir[3] = ONE;
    }

    /// `SNOW::Move` (0x00503e80) for type 3: no wrap about the player; a
    /// new drift every `cnt` frames, back to the base when `life` runs out.
    fn step(&mut self, rng: &mut Rng) {
        self.pos = ee::vadd(self.pos, self.dir);
        self.pos[3] = ONE;
        self.eff_pos = self.pos;
        self.life -= 1;
        self.cnt -= 1;
        if self.cnt == 0 {
            self.cnt = fieldrand(rng, 10) as i32 + 3;
            self.dir[0] = Spark::drift(rng);
            self.dir[1] = Spark::drift(rng);
        }
        if self.life == 0 {
            self.calc_pos3(rng);
            self.eff_pos = self.pos;
            self.life = fieldrand(rng, 60) as i32 + 20;
        }
    }

    /// `SNOW::Draw` (0x005042f0): fading over its last 32 frames.
    fn fade(&mut self) {
        self.transparency = if self.life < 32 { ee::div(ee::from_int(self.life), K100) } else { ONE };
    }
}

/// The room's sparks (`fire[0..60]`), and the palette change each asks of
/// its `ccEff` (`SetWater`'s `ChangeClut(eff, "c1")` for clutType 3, `"c2"`
/// for 4; `ccEff::ChangeClut` changes it only while the palette is the old).
#[derive(Clone)]
pub struct Sparks {
    rooms: Rc<Rooms>,
    pub sparks: Vec<Spark>,
    pub clut: Option<Clut>,
}

impl Sparks {
    pub fn new(def: &'static SparksDef, cx: &mut BuildCx) -> Result<Sparks> {
        let rooms = cx.rooms.expect("a dungeon's sparks").clone();
        let clut = def.clut.and_then(|[old, c1, c2]| match rooms.tex_clut.clut_type {
            3 => Some(Clut::Change { new: c1, old }),
            4 => Some(Clut::Change { new: c2, old }),
            _ => None,
        });
        Ok(Sparks { rooms, sparks: Vec::new(), clut })
    }
}

impl MapSystem for Sparks {
    /// `Draw`: each moved and drawn; `Again`: drawn as it stands.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        match hook {
            Hook::Draw => {
                let Some(rng) = cx.core.field_rand.as_mut() else { return };
                for s in &mut self.sparks {
                    s.step(rng);
                    s.fade();
                }
            }
            Hook::Again => {}
            _ => return,
        }
        (0..self.sparks.len()).for_each(|k| cx.put(Kind::Sparks, Piece::Item(k)));
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(k) = piece else { return None };
        let s = self.sparks.get(k)?;
        let state = EffState { clut: self.clut, ..EffState::NO_FOG };
        Some(Request::Sprite(EffSprite {
            scale: [HALF; 2],
            transparency: s.transparency,
            ..EffSprite::new(self.rooms.name, SPARK, s.eff_pos, 0, crate::draw::EFF_LAYER, state)
        }))
    }

    /// `SetWater`'s tail: a spark at each magma dummy's place as its world
    /// matrix holds it, z -100; gone with the room.
    fn room(&mut self, part: RoomPart, cx: &mut RoomCx, _: &mut Others) {
        match part {
            RoomPart::Clear => self.sparks.clear(),
            RoomPart::Water => {
                let Some(room) = cx.room else { return };
                for d in self.rooms.subst(room, MAGMA) {
                    let t = d.world[3];
                    self.sparks.push(Spark::new([t[0], t[1], SPARK_Z, t[3]], cx.rng));
                }
            }
            _ => {}
        }
    }
}
