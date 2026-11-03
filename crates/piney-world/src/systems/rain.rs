//! `DrawRain` (gcmn 0x005a8860) in rain and storms, on the effects' layer:
//! the fifty drops `SetRainEffect` (0x005a4840) made, each placed anew about
//! the camera every frame, with its splash ([`Rain`]); in a storm the
//! lightning, the screen's flash and the thunder ([`Thunder`]). Its
//! counters are the function's statics, which outlive every field (the
//! first `DrawRain` of the whole game seeds the bolts' count).

use piney_data::Result;
use piney_data::dungeon::Rng;

use super::{BuildCx, Carried, CarriedMut, DrawCx, Hook, Kind, MapEvent, MapSystem, Others, Piece, Request, StepCx};
use crate::ee::{self, F, V4};
use crate::field::ambient::{Eff, Env, Weather, statics, uint_float};
use crate::sprite::EffSprite;

const K10: F = 0x4120_0000;
const K100: F = 0x42c8_0000;
const MINUS_ONE: F = 0xbf80_0000;

/// One of the fifty rain drops (`WORLD` +0x5400, 64 bytes each).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drop {
    /// +0 the drop, +4 its splash (`EFF_sfp8rai6`).
    pub drop: Eff,
    pub splash: Eff,
    /// +0x10 where it falls (x, y about the eye), +0x20 the splash's
    /// (kept, never drawn: the splash is drawn at the drop's place).
    pub pos: V4,
    pub splash_pos: V4,
    /// +0x30 the drop's pattern (0-19), +0x34 the splash's (0-14).
    pub pattern: i32,
    pub splash_pattern: i32,
    /// +0x38 the splash shows.
    pub splash_on: bool,
}

/// `SetRainEffect`: the drops for rain (weather 2: `EFF_sfp8rai1-4`) or
/// storm (3: `rai7-10`), each picked by `fieldrand(4000) / 1000`, with its
/// pattern from `fieldrand(20)`.
fn set_rain(weather: u32, rng: &mut Rng) -> Vec<Drop> {
    const RAIN: [&str; 4] = ["EFF_sfp8rai1", "EFF_sfp8rai2", "EFF_sfp8rai3", "EFF_sfp8rai4"];
    const STORM: [&str; 4] = ["EFF_sfp8rai7", "EFF_sfp8rai8", "EFF_sfp8rai9", "EFF_sfp8rai10"];
    (0..50)
        .map(|_| {
            let k = (rng.below(4000) / 1000) as usize;
            let drop = Eff::fogged(if weather == 2 { RAIN[k] } else { STORM[k] });
            let pattern = rng.below(20) as i32;
            Drop {
                drop,
                splash: Eff::fogged("EFF_sfp8rai6"),
                pos: [0; 4],
                splash_pos: [0; 4],
                pattern,
                splash_pattern: 0,
                splash_on: false,
            }
        })
        .collect()
}

/// The drops (none unless it rains), `DrawRain`'s statics (`tcnt`, None
/// until the first `DrawRain` draws it, `tcnt2`, `secnt`, `seflag`; gcmn
/// 0x00378d24-40), and this frame's sprites.
#[derive(Clone, Debug)]
pub struct Rain {
    pub drops: Vec<Drop>,
    pub tcnt: Option<i32>,
    pub tcnt2: i32,
    pub secnt: i32,
    pub seflag: bool,
    drawn: Vec<EffSprite>,
}

impl Rain {
    /// `Generate`'s `SetRainEffect`, in rain or a storm.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Rain> {
        let w = cx.field.expect("a field's rain").weather;
        let drops = if w.rains() { set_rain(w.weather(), &mut cx.rng) } else { Vec::new() };
        Ok(Rain { drops, tcnt: None, tcnt2: 750, secnt: 0, seflag: false, drawn: Vec::new() })
    }
}

impl MapSystem for Rain {
    /// Each drop placed again every frame: the flag the code tests (+0x3c)
    /// is only ever cleared, and the one it sets is `WORLD` +0x1c.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw || self.drops.is_empty() {
            return;
        }
        let Some(mut rng) = cx.core.field_rand else { return };
        self.drawn.clear();
        {
            let env = Env::of(cx);
            if self.tcnt.is_none() {
                self.tcnt = Some(rng.below(60) as i32 + 60);
            }
            let eye = env.eye1;
            for d in &mut self.drops {
                let x = rng.below(3000) as i32 - 1500;
                let y = rng.below(3000) as i32 - 1500;
                d.pos[0] = ee::add(ee::from_int(x), eye[0]);
                d.pos[1] = ee::add(ee::from_int(y), eye[1]);
                let mut p = env.p2w(env.w2p(d.pos));
                p[2] = env.height(p[0], p[1]);
                d.drop.pos = p;
                self.drawn.push(d.drop.draw(3, d.pattern as u16));
                d.pattern += 1;
                if d.pattern == 20 {
                    d.pattern = 0;
                    d.splash_pattern = 0;
                    d.splash_on = true;
                    d.splash_pos = d.pos;
                    d.splash_pos[2] = ee::add(d.splash_pos[2], K10);
                }
                if d.splash_on {
                    p[2] = ee::add(K10, env.height(p[0], p[1]));
                    d.splash.pos = p;
                    self.drawn.push(d.splash.draw(3, d.splash_pattern as u16));
                    d.splash_pattern += 1;
                    if d.splash_pattern == 15 {
                        d.splash_on = false;
                    }
                }
            }
        }
        cx.core.field_rand = Some(rng);
        (0..self.drawn.len()).for_each(|k| cx.put(Kind::Rain, Piece::Item(k)));
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(k) = piece else { return None };
        self.drawn.get(k).map(|s| Request::Sprite(*s))
    }

    fn take(&mut self, c: &Carried) {
        let Some(w) = c.field else { return };
        let w = &w[statics::RAIN..];
        self.tcnt = (w[0] != 0).then_some(w[1] as i32);
        self.tcnt2 = w[2] as i32;
        self.secnt = w[3] as i32;
        self.seflag = w[4] != 0;
    }

    fn put(&self, c: &mut CarriedMut) {
        let Some(w) = c.field.as_deref_mut() else { return };
        let words = [
            u32::from(self.tcnt.is_some()),
            self.tcnt.unwrap_or(0) as u32,
            self.tcnt2 as u32,
            self.secnt as u32,
            u32::from(self.seflag),
        ];
        w[statics::RAIN..statics::RAIN + 5].copy_from_slice(&words);
    }
}

/// The storm's lightning (none outside a storm): `EFF_sfp8thu1-4`
/// (+0x6080..), the one struck (+0x6094) and the frames it shows (+0x6090).
#[derive(Clone, Debug)]
pub struct Thunder {
    pub effs: Option<[Eff; 4]>,
    pub index: usize,
    pub frames: i32,
    drawn: Option<EffSprite>,
}

impl Thunder {
    /// `Generate`'s, in a storm.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Thunder> {
        let w: Weather = cx.field.expect("a field's thunder").weather;
        let effs =
            (w.weather() == 3).then(|| ["EFF_sfp8thu1", "EFF_sfp8thu2", "EFF_sfp8thu3", "EFF_sfp8thu4"].map(Eff::new));
        Ok(Thunder { effs, index: 0, frames: 0, drawn: None })
    }
}

/// The thunder's sound: 236 on Infection, 235 from Mutation on, whose
/// `DrawRain` passes one less (MUT gcmn 0x005d43d4, OUT 0x005d1c90, QUA
/// 0x004c46f0).
fn thunder_se(volume: piney_data::volume::Volume) -> i32 {
    if volume == piney_data::volume::Volume::Inf { 236 } else { 235 }
}

impl MapSystem for Thunder {
    /// `DrawRain`'s storm, after the drops: a bolt somewhere ahead every
    /// 30-59 frames (the first after 60-119), shown four frames at the
    /// centre's offset; a flash of the screen every 900-1499 frames (the
    /// first after 750) and its thunder ([`thunder_se`]) 30-49 frames later.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let (Some(effs), Some(rain)) = (&mut self.effs, others.get_mut::<Rain>()) else { return };
        let Some(mut rng) = cx.core.field_rand else { return };
        self.drawn = None;
        let mut out = Vec::new();
        {
            let env = Env::of(cx);
            let tcnt = rain.tcnt.get_or_insert(0);
            *tcnt -= 1;
            rain.tcnt2 -= 1;
            if *tcnt == 0 {
                self.index = (rng.below(4000) / 1000) as usize;
                let e = &mut effs[self.index];
                e.pos[0] = 0;
                e.pos[1] = 0xc63b_8000;
                let dy = rng.below(6000) as i32 - 3000;
                // The difference is converted as unsigned: a negative one
                // comes out near 2^32.
                e.pos[1] = ee::add(e.pos[1], uint_float(dy as u32));
                e.pos[2] = 0x4461_0000;
                let mut rot = env.cam_rot();
                rot[0] = 0;
                rot[1] = 0;
                let r = ee::sub(ee::div(uint_float(rng.below(157)), K100), 0x3f48_f5c3);
                let z = ee::add(rot[2], r);
                if ee::lt(z, 0xc048_f5c3) || !ee::le(z, 0x4048_f5c3) {
                    rot[2] = ee::mul(MINUS_ONE, rot[2]);
                }
                rot[2] = ee::add(rot[2], r);
                let m = piney_data::anim::rot_bits([rot[0], rot[1], rot[2]]);
                e.pos = ee::apply(&m, e.pos);
                self.frames = 4;
                *tcnt = rng.below(30) as i32 + 30;
            }
            if rain.tcnt2 == 0 {
                out.push(MapEvent::Flash { frames: 10, colour: 0x80ff_ffff, rect: [0, 0, 0x4400_0000, 0x43c0_0000] });
                rain.tcnt2 = rng.below(600) as i32 + 900;
                rain.secnt = rng.below(20) as i32 + 30;
                rain.seflag = true;
            }
            if rain.seflag {
                rain.secnt -= 1;
            }
            if rain.secnt == 0 && rain.seflag {
                out.push(MapEvent::Se(thunder_se(env.volume)));
                rain.seflag = false;
            }
            if self.frames != 0 {
                let e = &mut effs[self.index];
                // Added for the draw and taken off again, rounding each way.
                e.pos[0] = ee::add(e.pos[0], env.ofs[0]);
                e.pos[1] = ee::add(e.pos[1], env.ofs[1]);
                self.drawn = Some(e.draw(3, 0));
                e.pos[0] = ee::sub(e.pos[0], env.ofs[0]);
                e.pos[1] = ee::sub(e.pos[1], env.ofs[1]);
                self.frames -= 1;
            }
        }
        cx.core.field_rand = Some(rng);
        cx.events.extend(out);
        if self.drawn.is_some() {
            cx.put(Kind::Thunder, Piece::Item(0));
        }
    }

    fn draw<'a>(&'a self, _: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        self.drawn.map(Request::Sprite)
    }
}
