//! `FIREFLY2`s ([`crate::sky::firefly`]): the boss arenas' fireflies, each
//! made over one of the file's markers, then every frame drawn and moved
//! from `fieldrand`. Their sprite is the field's `bossFireFlyEff` of the
//! area's own file.

use piney_data::statics::Position;
use piney_data::{Error, Result};

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::EFF_LAYER;
use crate::sky::firefly::{Firefly, FireflyDraw};
use crate::sprite::{EffSprite, EffState};

/// How many the constructor makes, the markers the k-th starts over (`k %
/// markers`), and each field's sprite (`bossFireFlyEff`, by `game.field`).
#[derive(Clone, Copy, Debug)]
pub struct Firefly2Def {
    pub count: usize,
    pub markers: &'static [&'static str],
    pub effs: &'static [&'static str],
}

/// The fireflies (+0x1c4), and their sprite.
#[derive(Clone, Debug)]
pub struct Fireflies2 {
    pub fireflies: Vec<Firefly>,
    file: &'static str,
    eff: &'static str,
}

impl Fireflies2 {
    pub fn new(def: &'static Firefly2Def, cx: &mut BuildCx) -> Result<Fireflies2> {
        let file = cx.file;
        let (c, sc) = (&file.ccs, &file.scene);
        let volume = cx.volume;
        let mut fireflies = Vec::with_capacity(def.count);
        for k in 0..def.count {
            let marker = def.markers[k % def.markers.len()];
            if c.find_object(marker).is_none() {
                return Err(Error::NotFound(format!("{}: {marker}", file.stem)));
            }
            let (at, _) = crate::town::dummy_bits(&Position::Dummy(marker), c, sc);
            fireflies.push(Firefly::arena(volume, &mut cx.rng, at));
        }
        let eff = usize::try_from(cx.field()).ok().and_then(|f| def.effs.get(f)).copied().unwrap_or("");
        Ok(Fireflies2 { fireflies, file: cx.stem, eff })
    }
}

impl MapSystem for Fireflies2 {
    /// Each one's `Draw`, then its `Move`.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let (volume, player) = (cx.core.hits.volume, cx.view.player);
        let mut drawn: Vec<FireflyDraw> = Vec::new();
        for f in &mut self.fireflies {
            f.draw(volume, player, &mut drawn);
            if let Some(rng) = &mut cx.core.field_rand {
                f.step(volume, player, rng);
            }
        }
        drawn.into_iter().for_each(|d| cx.put(Kind::Firefly2, Piece::Fly(d)));
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Fly(d) = piece else { return None };
        Some(Request::Sprite(EffSprite {
            scale: [d.scale; 2],
            transparency: d.transparency,
            ..EffSprite::new(self.file, self.eff, d.pos, d.pattern, EFF_LAYER, EffState::INIT)
        }))
    }
}
