//! A field's 2D smoke (types 2, 3, 5, 6): `WORLD`'s `ccMask` on
//! `TEX_sfsmo1` of `field_eff`, puffs drifting across the screen from the
//! left (`SetSmoke2d`, 0x005a6a50; `DrawEffect` 0x005a93ac), each frame's
//! puffs one request ([`super::Request::Masks`]).

use piney_data::Result;
use piney_data::dungeon::Rng;
use piney_desktop::message::WindowTexture;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::ee::{self, F};
use crate::field::ambient::EFF_FILE;

/// The 2D smoke's texture in `field_eff`.
const TEX: &str = "TEX_sfsmo1";

/// The smoke as the last `DrawEffect` left it.
#[derive(Clone, Debug)]
pub struct Smoke2d {
    /// +0x6178: x, y, dx, dy in screen pixels.
    pub puffs: [[i32; 4]; 5],
    /// +0x61c8 puffs shown, +0x61cc frames shown, +0x61d0 frames asleep,
    /// +0x61d4 the transparency (an int: 1 shown, 0 gone).
    pub count: usize,
    pub active: i32,
    pub sleep: i32,
    pub alpha: i32,
    /// The texture, and this frame's puffs: transparency, x, y.
    tex: Option<WindowTexture>,
    pub masks: Vec<[F; 3]>,
}

impl Smoke2d {
    fn puff(rng: &mut Rng) -> [i32; 4] {
        let x = -(rng.below(512) as i32);
        let y = rng.below(384) as i32;
        let dx = rng.below(20) as i32 + 20;
        let dy = rng.below(20) as i32 - 10;
        [x, y, dx, dy]
    }

    /// `WORLD::Init`'s: the sleep, then `SetSmoke2d(0..4)`.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Smoke2d> {
        let rng = &mut cx.rng;
        let sleep = rng.below(300) as i32 + 150;
        let puffs = std::array::from_fn(|_| Smoke2d::puff(rng));
        let tex = WindowTexture::read_named(cx.archive, EFF_FILE, TEX, None);
        Ok(Smoke2d { puffs, count: 0, active: 0, sleep, alpha: 0, tex, masks: Vec::new() })
    }

    /// `DrawEffect`'s frame of it.
    fn frame(&mut self, rng: &mut Rng) {
        self.masks.clear();
        if self.sleep != 0 {
            self.sleep -= 1;
            if self.sleep == 0 {
                self.active = rng.below(300) as i32 + 150;
                self.alpha = 1;
                self.count = rng.below(4) as usize + 1;
            }
            return;
        }
        if self.active != 0 {
            self.active -= 1;
        } else {
            if self.alpha != 0 {
                // fptosi(alpha - 0.005): the int goes straight to 0.
                self.alpha = ee::to_int(ee::sub(ee::from_int(self.alpha), 0x3ba3_d70a));
            }
            if ee::le(ee::from_int(self.alpha), 0) {
                self.sleep = rng.below(300) as i32 + 150;
            }
        }
        let alpha = ee::from_int(self.alpha);
        for i in 0..self.count.min(5) {
            let [x, y, dx, dy] = self.puffs[i];
            self.masks.push([alpha, ee::from_int(x), ee::from_int(y)]);
            let (x, y) = (x + dx, y + dy);
            self.puffs[i][0] = x;
            self.puffs[i][1] = y;
            if !ee::le(ee::from_int(x), 0x4400_0000) || !ee::le(ee::from_int(y), 0x43c0_0000) || y < -512 {
                self.puffs[i] = Smoke2d::puff(rng);
            }
        }
    }
}

impl MapSystem for Smoke2d {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let Some(rng) = cx.core.field_rand.as_mut() else { return };
        self.frame(rng);
        if !self.masks.is_empty() {
            cx.put(Kind::Smoke, Piece::Item(0));
        }
    }

    /// On the effects' layer.
    fn draw<'a>(&'a self, _: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let tex = self.tex.as_ref()?;
        let layer = crate::field::area::ambient_layer(3);
        Some(Request::Masks { layer, tex, puffs: &self.masks })
    }
}
