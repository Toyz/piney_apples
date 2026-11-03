//! `WORLD::DrawSteam(1)` (gcmn 0x005ad600), field type 0's steam vents:
//! asleep, or a burst of 2-6 `effSmoke` puffs at a place about the player
//! (chosen, with its hiss `ccSeOn3D(43)`, at the first of eight bursts
//! 3-6 frames apart), then asleep for 120-270 frames. Its counters are
//! the function's statics, which outlive every field.

use piney_data::Result;

use super::{BuildCx, Carried, CarriedMut, Hook, MapEvent, MapSystem, Others, Smoke, StepCx};
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::{Env, statics, uint_float};

const K100: F = 0x42c8_0000;

/// `DrawSteam`'s statics (gcmn 0x00378d5c-74, 0x00730580).
#[derive(Clone, Debug, Default)]
pub struct Steam {
    pub life: i32,
    pub sleep: i32,
    pub step: F,
    pub num: i32,
    pub pos: V4,
}

impl Steam {
    pub fn new(_: &'static (), _: &mut BuildCx) -> Result<Steam> {
        Ok(Steam::default())
    }
}

impl MapSystem for Steam {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let Some(mut rng) = cx.core.field_rand else { return };
        let mut out = Vec::new();
        {
            let env = Env::of(cx);
            if self.sleep != 0 {
                self.sleep -= 1;
                return;
            }
            if self.num == 0 {
                let x = rng.below(2000) as i32 - 1000;
                let y = rng.below(2000) as i32 - 1000;
                self.pos[0] = ee::from_int(x);
                self.pos[1] = ee::from_int(y);
                self.pos = env.p2w(self.pos);
                self.pos[2] = ee::add(0x41a0_0000, env.height(self.pos[0], self.pos[1]));
                self.pos[3] = ONE;
                out.push(MapEvent::Se3d { n: 43, pos: self.pos });
            }
            self.sleep = rng.below(4) as i32 + 3;
            let n = rng.below(5) as i32 + 2;
            for _ in 0..n {
                let vx = ee::div(uint_float(rng.below(50)), K100);
                let vy = ee::div(uint_float(rng.below(50)), K100);
                let vz = uint_float(rng.below(10) + 10);
                let s = rng.below(5) as i32 + 3;
                self.life = (rng.below(3) as i32 + 1) * 10;
                out.push(MapEvent::Smoke(Smoke {
                    pos: self.pos,
                    v: [vx, vy, vz, ONE],
                    s: ee::from_int(s),
                    life: self.life,
                    t: 109,
                    fade_in: 512,
                    fade_out: 32,
                }));
            }
            self.num += 1;
            if self.num == 8 {
                self.sleep = (rng.below(6) as i32 + 4) * 30;
                self.num = 0;
            }
            self.sleep -= 1;
        }
        cx.core.field_rand = Some(rng);
        cx.events.extend(out);
    }

    /// The statics as the last field left them, then `WORLD::Init`'s
    /// `DrawSteam(0)`: asleep 0, `step` 0.5, `num` 0.
    fn take(&mut self, c: &Carried) {
        let Some(w) = c.field else { return };
        let w = &w[statics::STEAM..];
        let i = |k: usize| w[k] as i32;
        *self = Steam { life: i(0), sleep: 0, step: 0x3f00_0000, num: 0, pos: [w[4], w[5], w[6], w[7]] };
    }

    fn put(&self, c: &mut CarriedMut) {
        let Some(w) = c.field.as_deref_mut() else { return };
        let [p0, p1, p2, p3] = self.pos;
        let words = [self.life as u32, self.sleep as u32, self.step, self.num as u32, p0, p1, p2, p3];
        w[statics::STEAM..statics::STEAM + 8].copy_from_slice(&words);
    }
}
