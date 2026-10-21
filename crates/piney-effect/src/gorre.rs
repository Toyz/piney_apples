//! Gorre's Kerse (OUT gcmn boss05.cpp, bosseff.cpp): the
//! `ccBossEffFinalPhotonFlash` its last step makes at the target
//! (`Create` 0x0048a6e0, `Draw` 0x00480b70, `Explode` 0x00480d00).
//! docs/engine/boss-gorre.md, "The flash".

use crate::boss::{Tables, ff_row, gen_row, va};
use crate::draw::DrawRec;
use crate::ee::{self, F, V4};
use crate::eff::Eff;
use crate::effect::EFFECT_LAYER;
use crate::{Cx, Event};

/// +0x30's start, and what each `Draw` takes off it.
const SCALE: F = 0x447a_0000;
const SHRINK: F = 0x42c8_0000;
/// `ccEff` +0x2c: RGB (0x80 = 1).
const COLOUR: u32 = 0x80e0_ffff;
/// The `Draw`s after the burst before the slot frees (+0x34 reaching 60).
const AFTER: i32 = 60;

/// `ccBossEffFinalPhotonFlash` (0x50 bytes): particle's `EFF_x000` on
/// `CLT_x000c5` shrinking onto the place, then a burst.
#[derive(Clone, Debug)]
pub struct PhotonFlash {
    eff: Option<Box<Eff>>,
    /// +0x20.
    pos: V4,
    /// +0x30.
    scale: F,
    /// +0x38 1: burst; +0x34 the `Draw`s since.
    burst: bool,
    count: i32,
    /// +0x3c, +0x40: the flash's and the burst's sounds played.
    hum: bool,
    boom: bool,
}

/// What one `Draw` left: whether the slot stays, and the burst's ring
/// (`ccBossEffAutoSamonRingCreate(pos, 0, (0, 2/3, 10), 176)`) to make.
pub struct Drawn {
    pub on: bool,
    pub ring: Option<V4>,
}

impl PhotonFlash {
    /// `ccBossEffFinalPhotonFlashCreate(pos)`: `ccEff::Init(EFF_x000, 1)`
    /// with `CLT_x000c5` as its palette, scale 1000.
    pub fn new(cx: &Cx, pos: V4) -> PhotonFlash {
        let mut eff = crate::boss::particle_eff(cx, "EFF_x000");
        if let Some(e) = eff.as_mut() {
            e.clut = cx.assets.find("particle", "CLT_x000c5");
        }
        PhotonFlash { eff, pos, scale: SCALE, burst: false, count: 0, hum: false, boom: false }
    }

    /// One `Draw`. Shrinking: SE 159 (note 70) the first time, the sprite at
    /// the place and scale, no depth test (`SetRenderState(0, 0)`), on the
    /// manager's layer; then the scale less 100, and below 0 the ring and
    /// the burst. Burst: 60 more `Draw`s, then the slot frees.
    pub fn draw(&mut self, cx: &mut Cx, t: &Tables) -> Drawn {
        if self.burst {
            let c = self.count;
            self.count += 1;
            return Drawn { on: c != AFTER, ring: None };
        }
        if !self.hum {
            cx.raise(Event::SoundNote { se: 159, note: 70 });
        }
        self.hum = true;
        if let Some(e) = self.eff.as_mut() {
            e.scale_x = self.scale;
            e.scale_y = self.scale;
            e.test &= !(1 << 16);
            e.color = COLOUR;
            e.pos[..3].copy_from_slice(&self.pos[..3]);
            cx.draws.push(DrawRec::Eff { eff: e.clone(), pat: 0, layer: EFFECT_LAYER });
        }
        let s = ee::sub(self.scale, SHRINK);
        if !ee::lt(s, 0) {
            self.scale = s;
            return Drawn { on: true, ring: None };
        }
        self.explode(cx, t);
        self.burst = true;
        self.scale = 0;
        Drawn { on: true, ring: Some(self.pos) }
    }

    /// `Explode`: SEs 35 and 40 (note 55) the first time; five
    /// `BurstSmokeGenerator`s up the place (rows 2 at +100 and 2 and 3 at
    /// +170 with ff 2, 4 at z 0 with ff 3 and 2, 7 at z 90 with ff 6, 7
    /// and 8); `ccEnemyEffDust(pos, 10, 5)`.
    fn explode(&mut self, cx: &mut Cx, t: &Tables) {
        if !self.boom {
            self.boom = true;
            cx.raise(Event::SoundNote { se: 35, note: 55 });
            cx.raise(Event::SoundNote { se: 40, note: 55 });
        }
        let mut at = self.pos;
        let rows: [(u32, &[u32], Option<F>); 5] = [
            (2, &[2], Some(0x42c8_0000)),
            (2, &[2], Some(0x428c_0000)),
            (3, &[2], None),
            (4, &[3, 2], None),
            (7, &[6, 7, 8], Some(0x42b4_0000)),
        ];
        for (k, (row, ffs, up)) in rows.into_iter().enumerate() {
            // The fourth's z is set to 0 before its start.
            if k == 3 {
                at[2] = 0;
            }
            if let Some(dz) = up {
                at[2] = ee::add(at[2], dz);
            }
            let ff = std::array::from_fn(|j| ffs.get(j).map_or(0, |&r| ff_row(va::BURST_SMOKE_FF, r)));
            if let Some(mut g) = t.generator(cx, gen_row(va::BURST_SMOKE, row), ff) {
                g.pos = at;
                cx.particles.start(g);
            }
        }
        let (life, tex) = (crate::dust::DUST_LIFE, crate::dust::DUST_TEX);
        crate::dust::enemy_eff_dust(cx, self.pos, 10, 0x40a0_0000, life, tex);
    }
}
