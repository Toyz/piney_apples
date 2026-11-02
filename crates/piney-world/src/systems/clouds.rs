//! A map's `CLOUD`s ([`crate::sky::cloud`]): made about the player by the
//! first `Draw`, then each moved and drawn every frame from `fieldrand`.

use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::EFF_LAYER;
use crate::sky::cloud::{self, Cloud};
use crate::sky::lensflare;
use crate::town::TownSprite;

/// How many clouds the first `Draw` makes and of which type.
#[derive(Clone, Copy, Debug)]
pub struct CloudsDef {
    pub count: usize,
    pub kind: i32,
}

/// The clouds as the last `Draw` left them.
#[derive(Clone, Debug)]
pub struct Clouds {
    def: &'static CloudsDef,
    /// Set by the first `Draw`, which makes the clouds.
    pub started: bool,
    pub clouds: Vec<Cloud>,
    /// `EFF_srzsmo1`'s `patNum` in `town_z`.
    pat_num: u16,
}

impl Clouds {
    pub fn new(def: &'static CloudsDef, cx: &mut BuildCx) -> Result<Clouds> {
        let eff = SceneFile::read(cx.archive, lensflare::FILE)?;
        let pat_num = cloud::eff_pat_num(&eff.ccs, cloud::EFF_SMOKE)
            .ok_or_else(|| Error::NotFound(format!("{}: {}", lensflare::FILE, cloud::EFF_SMOKE)))?;
        Ok(Clouds { def, started: false, clouds: Vec::new(), pat_num })
    }
}

impl MapSystem for Clouds {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let (volume, v) = (cx.core.hits.volume, cx.view);
        match hook {
            Hook::Start if !self.started => {
                self.started = true;
                if let Some(rng) = &mut cx.core.field_rand {
                    let (kind, pat_num) = (self.def.kind, self.pat_num);
                    self.clouds = (0..self.def.count).map(|_| Cloud::init(kind, pat_num, v.player, rng)).collect();
                }
            }
            // Each cloud's Move, then its Draw.
            Hook::Draw => {
                for (k, c) in self.clouds.iter_mut().enumerate() {
                    if let Some(rng) = &mut cx.core.field_rand {
                        c.step(volume, v.player, rng, &|_, _| 0);
                    }
                    let in_view = crate::town::pc::check_camera_deg(c.pos, cloud::VIEW_DEG, &v.cam, v.player);
                    if c.drawn(volume, v.player, in_view) {
                        cx.put(Kind::Clouds, Piece::Sprite { k, pattern: c.pattern as u16, pos: c.pos });
                    }
                }
            }
            _ => {}
        }
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Sprite { k, pattern, pos } = piece else { return None };
        let c = self.clouds.get(k)?;
        Some(Request::Sprite(TownSprite {
            file: lensflare::FILE,
            eff: cloud::EFF_SMOKE,
            pos,
            pattern,
            layer: EFF_LAYER,
            scale: [c.eff.scale_x, c.eff.scale_y],
            rotate: c.eff.rotate,
            transparency: c.eff.transparency,
            depth: false,
        }))
    }
}
