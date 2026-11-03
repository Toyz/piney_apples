//! A map's `LENSFLARE` ([`crate::sky::lensflare`]): six `ccEff`s strung
//! from its sun toward the eye while the sun is in view; a field's taken
//! round its torus; or a class's own copy of the drawing ([`FlareDraw`]).

use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::EFF_LAYER;
use crate::ee::{self, ONE, V4};
use crate::field::ambient::Env;
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
    /// `LENSFLARE::Draw(bgstream, name, 1)` (0x00503360) in a field, while
    /// `WORLD_MAN::CheckLensFlare` holds: nothing unless
    /// `ccCheckCameraDeg(cameraGetPos(camID), 12288)`, which, asked of the
    /// camera's own place, tests only that the camera looks within 67.5
    /// degrees of +x; the sun taken from the player's (`ccTransPosP2W`),
    /// each flare brought to the player's side of the map.
    Field,
}

/// Where the sun stands: a dummy of the file, or a field's background
/// row's, `DMY_<name>_<b + 1>point` (none past the table's `rows`).
#[derive(Clone, Copy, Debug)]
pub enum Sun {
    Dummy(&'static str),
    Row { name: &'static str, rows: u32 },
}

/// The sun, the file of the six flares, and the drawing.
#[derive(Clone, Copy, Debug)]
pub struct FlareDef {
    pub sun: Sun,
    pub file: &'static str,
    pub draw: FlareDraw,
}

impl FlareDef {
    /// A map's flare of `town_z` at the dummy `sun`.
    pub const fn at(sun: &'static str, draw: FlareDraw) -> FlareDef {
        FlareDef { sun: Sun::Dummy(sun), file: lensflare::FILE, draw }
    }
}

/// The lens flare as the last `Draw` left it.
#[derive(Clone, Debug)]
pub struct LensFlare {
    def: &'static FlareDef,
    /// The sun's dummy's position (w 1).
    pub sun: V4,
    /// Drawn at all: a field's only while `WORLD_MAN::CheckLensFlare`
    /// holds and its row has a sun.
    pub lit: bool,
    /// Where `LENSFLARE::Draw` put its flares last (None: not drawn).
    pub flares: Option<[V4; 6]>,
}

impl LensFlare {
    /// `LENSFLARE::LENSFLARE`: its six effects, the sun.
    pub fn new(def: &'static FlareDef, cx: &mut BuildCx) -> Result<LensFlare> {
        let eff = SceneFile::read(cx.archive, def.file)?;
        for name in FLARES {
            eff.ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{}: {name}", def.file)))?;
        }
        let file = &**cx.file;
        let dummy = |name: &str| {
            file.ccs
                .find_object(name)
                .and_then(|o| file.scene.dummies.get(&o))
                .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
        };
        let (sun, lit) = match def.sun {
            Sun::Dummy(name) => (dummy(name).ok_or_else(|| Error::NotFound(name.into()))?, true),
            Sun::Row { name, rows } => {
                let w = cx.field.expect("a field's lens flare").weather;
                let sun = (w.b < rows).then(|| dummy(&format!("DMY_{name}_{}point", w.b + 1))).flatten();
                (sun.unwrap_or([0; 4]), sun.is_some() && w.lens_flare())
            }
        };
        Ok(LensFlare { def, sun, lit, flares: None })
    }
}

/// `ccCheckCameraDeg(pos, deg)` (main 0x001da710) over a field's frame:
/// `pos` within `deg` of where the camera (`eye` looking at `view`) looks,
/// on the ground, all taken relative to the player.
fn check_camera_deg(pos: V4, eye: V4, view: V4, deg: i16, env: &Env) -> bool {
    let c = env.w2p(eye);
    let v = env.w2p(view);
    let p = env.w2p(pos);
    let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
    let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
    let x = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
    x > 0 && i32::from(x) < 2 * i32::from(deg)
}

impl MapSystem for LensFlare {
    /// `LENSFLARE::Draw(s, sun, 0)`, or the class's own.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw || !self.lit {
            return;
        }
        let v = cx.view;
        self.flares = match self.def.draw {
            FlareDraw::LensFlare => {
                let in_view = crate::town::pc::check_camera_deg(self.sun, lensflare::VIEW_DEG, &v.cam, v.player);
                lensflare::draw(self.sun, &v.flare, v.puppet_show, in_view)
            }
            FlareDraw::Own => (!v.puppet_show).then(|| lensflare::points(self.sun, &v.flare)),
            FlareDraw::Field => {
                let env = Env::of(cx);
                let f = &v.flare;
                check_camera_deg(f.eye, f.eye, f.view, lensflare::VIEW_DEG, &env).then(|| {
                    let sun = env.p2w(self.sun);
                    lensflare::points(sun, f).map(|p| env.p2w(env.w2p(p)))
                })
            }
        };
        for (k, pos) in self.flares.into_iter().flatten().enumerate() {
            cx.put(Kind::LensFlare, Piece::Sprite { k, pattern: 0, pos });
        }
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Sprite { k, pos, .. } = piece else { return None };
        Some(Request::Sprite(EffSprite::new(self.def.file, FLARES[k], pos, 0, EFF_LAYER, EffState::FLARE)))
    }
}
