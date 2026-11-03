//! `SNOW` (gcmn 0x00503650-0x00504380, 0x50 bytes) in a field: by the
//! weather the snowflakes `WORLD::Init` makes (100 light, 200 heavy), or
//! field type 0's sixteen embers; `DrawSnow` (0x005a8f40) moves and draws
//! every flake, ten times on `Draw`'s first frame on sysLayer, then once a
//! frame on the effects' layer, and type 0's `DrawEffect` its embers.

use piney_data::Result;
use piney_data::dungeon::Rng;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::{Eff, Env, Ground, Layer, Weather, field_file, fr};
use crate::sprite::EffSprite;

/// What a field type makes: its embers, or flakes as the weather asks,
/// each one of the field file's four sprites by `fieldrand(100)`.
#[derive(Clone, Copy, Debug)]
pub enum SnowDef {
    Embers(usize),
    Flakes { light: usize, heavy: usize, effs: [&'static str; 4] },
}

const K1300: F = 0x44a2_8000;
const K2600: F = 0x4522_8000;
const K800: F = 0x4448_0000;
const K10: F = 0x4120_0000;
const K100: F = 0x42c8_0000;
const TWO: F = 0x4000_0000;
const MINUS_ONE: F = 0xbf80_0000;
const MINUS_500: F = 0xc3fa_0000;

/// One `SNOW`: a snowflake (kinds 0 and 1) or type 0's ember (kind 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flake {
    /// +4: 0 light snow, 1 heavy snow, 2 an ember.
    pub kind: u32,
    /// +0 the ember's life, +8 frames to its next turn.
    pub timer: i32,
    pub turn: i32,
    /// +0xc the sideways spread, +0x10 the fall's scale.
    pub a: F,
    pub b: F,
    pub eff: Eff,
    /// +0x30 place, +0x40 velocity.
    pub pos: V4,
    pub vel: V4,
}

impl Flake {
    /// `calcPos` (0x005028e0): a new flake about the centre, 800 above the
    /// ground (never under 0), drifting and falling.
    fn calc_pos(&mut self, env: &Env, rng: &mut Rng) {
        let x = fr(rng, 2600);
        let y = fr(rng, 2600);
        let c = env.centre;
        self.pos[0] = ee::add(c[0], ee::sub(x, K1300));
        self.pos[1] = ee::add(c[1], ee::sub(y, K1300));
        let mut h = env.height(self.pos[0], self.pos[1]);
        if ee::lt(h, 0) {
            h = 0;
        }
        self.pos[2] = ee::add(K800, h);
        self.pos[3] = ONE;
        self.drift(rng);
        let f = ee::from_int(rng.below(5) as i32 + 1);
        self.vel[2] = ee::mul(ee::mul(MINUS_ONE, f), self.b);
        self.vel[3] = ONE;
    }

    /// `calcPos2` (0x00502ba0): an ember on the ground about the centre,
    /// rising.
    fn calc_pos2(&mut self, env: &Env, rng: &mut Rng) {
        let x = fr(rng, 2600);
        let y = fr(rng, 2600);
        let c = env.centre;
        self.pos[0] = ee::add(c[0], ee::sub(x, K1300));
        self.pos[1] = ee::add(c[1], ee::sub(y, K1300));
        self.pos[2] = env.height(self.pos[0], self.pos[1]);
        self.pos[3] = ONE;
        self.drift(rng);
        let f = ee::from_int(rng.below(3) as i32 + 2);
        self.vel[2] = ee::mul(f, self.b);
        self.vel[3] = ONE;
    }

    /// The sideways drift: `(fieldrand(a) - a / 2) / 10` for x and y.
    fn drift(&mut self, rng: &mut Rng) {
        let n = ee::to_int(self.a) as u32;
        let x = fr(rng, n);
        let half = ee::div(self.a, TWO);
        self.vel[0] = ee::div(ee::sub(x, half), K10);
        let y = fr(rng, n);
        self.vel[1] = ee::div(ee::sub(y, half), K10);
    }

    /// `SNOW(stream, kind)` (0x00503650) for kinds 0-2, a flake one of
    /// `effs`.
    pub fn new(kind: u32, field_type: u32, effs: [&'static str; 4], env: &Env, rng: &mut Rng) -> Flake {
        let (a, b) = match kind {
            0 => (0x4248_0000, TWO),
            1 => (0x4348_0000, 0x4080_0000),
            _ => (0x4170_0000, ONE),
        };
        let mut s = Flake { kind, timer: 0, turn: 0, a, b, eff: Eff::new(""), pos: [0; 4], vel: [0; 4] };
        if kind >= 2 {
            s.turn = rng.below(10) as i32 + 3;
            s.timer = rng.below(50) as i32 + 10;
        }
        if kind <= 1 {
            let r = fr(rng, 100);
            let k = if ee::lt(r, 0x41c8_0000) {
                0
            } else if ee::lt(r, 0x4248_0000) {
                1
            } else if ee::lt(r, 0x4296_0000) {
                2
            } else {
                3
            };
            s.eff = Eff::new(effs[k]);
            s.eff.file = field_file(env.volume, field_type);
            s.calc_pos(env, rng);
            let z0 = rng.below(800) as i32;
            // GetHeight(x, y != -500): the comparison went into the
            // argument, so this reads the height at y 0 or 1.
            let probe = if ee::eq(s.pos[1], MINUS_500) { 0 } else { ONE };
            s.pos[2] = if ee::eq(env.height(s.pos[0], probe), 0) {
                ee::from_int(z0)
            } else {
                ee::add(ee::from_int(z0), env.height(s.pos[0], s.pos[1]))
            };
            if kind == 1 {
                s.vel[2] = ee::from_int(-(rng.below(40) as i32 + 10));
            }
        } else {
            s.eff = Eff::new("EFF_sfa8fir1").scaled(0x3f00_0000);
            s.eff.file = field_file(env.volume, field_type);
            s.calc_pos2(env, rng);
        }
        s
    }

    /// `SNOW::Move` (0x00503e80).
    pub fn step(&mut self, env: &Env, rng: &mut Rng) {
        self.pos = ee::vadd(self.pos, self.vel);
        self.pos[3] = ONE;
        let mut d = env.w2p(self.pos);
        for k in 0..2 {
            if !ee::le(d[k], K1300) {
                d[k] = ee::sub(d[k], K2600);
                self.pos = env.p2w(d);
            }
            if ee::lt(d[k], ee::neg(K1300)) {
                d[k] = ee::add(d[k], K2600);
                self.pos = env.p2w(d);
            }
        }
        self.eff.pos = env.p2w(d);
        if self.kind <= 1 {
            if ee::lt(self.pos[2], 0) {
                self.calc_pos(env, rng);
                if self.kind == 1 {
                    self.vel[2] = ee::from_int(-(rng.below(40) as i32 + 10));
                }
            }
            return;
        }
        self.timer -= 1;
        self.turn -= 1;
        if self.turn == 0 {
            self.turn = rng.below(10) as i32 + 3;
            self.drift(rng);
        }
        if self.timer == 0 {
            self.calc_pos2(env, rng);
            let d = env.w2p(self.pos);
            self.eff.pos = env.p2w(d);
            self.timer = rng.below(60) as i32 + 20;
        }
    }

    /// `SNOW::Draw` (0x005042f0) on `layer`: an ember fades out over its
    /// last 32 frames.
    pub fn draw(&mut self, layer: Layer) -> EffSprite {
        if self.kind >= 2 {
            self.eff.transparency = if self.timer < 32 { ee::div(ee::from_int(self.timer), K100) } else { ONE };
        }
        self.eff.draw(layer, 0)
    }
}

/// The flakes or embers, `Draw`'s first frame made, and this frame's
/// sprites.
#[derive(Clone, Debug)]
pub struct Snow {
    weather: Weather,
    pub flakes: Vec<Flake>,
    started: bool,
    drawn: Vec<EffSprite>,
}

impl Snow {
    /// `WORLD::Init`'s, drawing from its `fieldrand` with the heights it
    /// sees (the height map not yet made) about `WORLD_MAN`'s centre.
    pub fn new(def: &'static SnowDef, cx: &mut BuildCx) -> Result<Snow> {
        let f = cx.field.expect("a field's snow");
        let w = f.weather;
        let env = Env::at(cx.volume, f.centre, Ground::Blank);
        let (count, kind, effs) = match *def {
            SnowDef::Embers(n) => (n, 2, [""; 4]),
            SnowDef::Flakes { heavy, effs, .. } if w.weather() == 5 => (heavy, 1, effs),
            SnowDef::Flakes { light, effs, .. } if w.weather() == 4 => (light, 0, effs),
            SnowDef::Flakes { .. } => (0, 0, [""; 4]),
        };
        let flakes = (0..count).map(|_| Flake::new(kind, w.field_type, effs, &env, &mut cx.rng)).collect();
        Ok(Snow { weather: w, flakes, started: false, drawn: Vec::new() })
    }

    /// Every flake moved, then every one drawn, on `layer`.
    fn fall(&mut self, layer: Layer, env: &Env, rng: &mut Rng) {
        for s in &mut self.flakes {
            s.step(env, rng);
        }
        for s in &mut self.flakes {
            self.drawn.push(s.draw(layer));
        }
    }

    /// `DrawSnow`: nothing unless it snows.
    fn snows(&self) -> bool {
        matches!(self.weather.weather(), 4 | 5)
    }
}

impl MapSystem for Snow {
    /// `Start`: the frame's first; on `Draw`'s first, ten falls on
    /// sysLayer where it snows. `Draw`: `DrawSnow` on the effects' layer,
    /// or the embers.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let Some(mut rng) = cx.core.field_rand else { return };
        let from = self.drawn.len();
        let embers = self.flakes.first().is_some_and(|f| f.kind == 2);
        {
            let env = Env::of(cx);
            match hook {
                Hook::Start => {
                    self.drawn.clear();
                    if !std::mem::replace(&mut self.started, true) && self.snows() {
                        for _ in 0..10 {
                            self.fall(0, &env, &mut rng);
                        }
                    }
                }
                Hook::Draw if embers || self.snows() => self.fall(3, &env, &mut rng),
                _ => return,
            }
        }
        cx.core.field_rand = Some(rng);
        let from = if hook == Hook::Start { 0 } else { from };
        (from..self.drawn.len()).for_each(|k| cx.put(Kind::Snow, Piece::Item(k)));
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(k) = piece else { return None };
        self.drawn.get(k).map(|s| Request::Sprite(*s))
    }
}
