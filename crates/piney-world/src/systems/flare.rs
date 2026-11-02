//! A map's `LENSFLARE` ([`crate::sky::lensflare`]): six `ccEff`s of `town_z`
//! strung from its sun toward the eye while the sun is in view; or a
//! class's own copy of the drawing ([`FlareDraw`]).

use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::EFF_LAYER;
use crate::ee::{ONE, V4};
use crate::sky::lensflare::{self, FLARES};
use crate::sprite::{EffSprite, EffState};

/// Which drawing strings the flares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlareDraw {
    /// `LENSFLARE::Draw(ccs, sun, 0)`: while the sun is in view and no
    /// puppet show runs.
    LensFlare,
    /// `EVENTAREA02::DrawLensFlare` (gcmn 0x00402dc0): the same points
    /// without the view's test, while no puppet show runs.
    Own,
}

/// The dummy the sun stands at, and the drawing.
#[derive(Clone, Copy, Debug)]
pub struct FlareDef {
    pub sun: &'static str,
    pub draw: FlareDraw,
}

/// The lens flare as the last `Draw` left it.
#[derive(Clone, Debug)]
pub struct LensFlare {
    def: &'static FlareDef,
    /// The sun's dummy's position (w 1).
    pub sun: V4,
    /// Where `LENSFLARE::Draw` put its flares last (None: not drawn).
    pub flares: Option<[V4; 6]>,
}

impl LensFlare {
    /// `LENSFLARE::LENSFLARE`: its six effects of `town_z`, the sun.
    pub fn new(def: &'static FlareDef, cx: &mut BuildCx) -> Result<LensFlare> {
        let eff = SceneFile::read(cx.archive, lensflare::FILE)?;
        for name in FLARES {
            eff.ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{}: {name}", lensflare::FILE)))?;
        }
        let file = &**cx.file;
        let sun = file
            .ccs
            .find_object(def.sun)
            .and_then(|o| file.scene.dummies.get(&o))
            .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
            .ok_or_else(|| Error::NotFound(def.sun.into()))?;
        Ok(LensFlare { def, sun, flares: None })
    }
}

impl MapSystem for LensFlare {
    /// `LENSFLARE::Draw(s, sun, 0)`, or the class's own.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let v = cx.view;
        self.flares = match self.def.draw {
            FlareDraw::LensFlare => {
                let in_view = crate::town::pc::check_camera_deg(self.sun, lensflare::VIEW_DEG, &v.cam, v.player);
                lensflare::draw(self.sun, &v.flare, v.puppet_show, in_view)
            }
            FlareDraw::Own => (!v.puppet_show).then(|| lensflare::points(self.sun, &v.flare)),
        };
        for (k, pos) in self.flares.into_iter().flatten().enumerate() {
            cx.put(Kind::LensFlare, Piece::Sprite { k, pattern: 0, pos });
        }
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Sprite { k, pos, .. } = piece else { return None };
        Some(Request::Sprite(EffSprite::new(lensflare::FILE, FLARES[k], pos, 0, EFF_LAYER, EffState::FLARE)))
    }
}
