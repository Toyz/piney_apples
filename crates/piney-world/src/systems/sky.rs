//! `DrawBG`'s clumps: the sky, the sun, the cloud layers and mountains, and
//! in crisis (`town0Nd`) the clumps over them, each at the origin, a dummy
//! or the bob ([`super::bob::Bob`]) on its own layer, its materials offset
//! as the scrolls ([`super::scrolls::Scrolls`]) last wrote them.

use std::borrow::Cow;
use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::bob::Bob;
use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::Fogging;
use crate::town::clump_models;

/// The layer a clump draws on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyLayer {
    /// One of `WORLD_MAN`'s layers, by priority.
    On(i16),
    /// Dun Loireag's cloud layers: effLayer in Infection; from Mutation on
    /// `SetActiveLayer(7)`, a layer `GO` adds at +0x4d4 with priority 5
    /// (MUT main 0x001b51c4, `DrawBG` gcmn 0x00439334).
    CloudSky,
}

impl SkyLayer {
    fn of(self, volume: Volume) -> i16 {
        match self {
            SkyLayer::On(p) => p,
            SkyLayer::CloudSky if volume == Volume::Inf => crate::draw::EFF_LAYER,
            SkyLayer::CloudSky => 5,
        }
    }
}

/// Where a clump draws: at the origin, at a dummy's position, or raised
/// by the bob (`EVENTAREA07`'s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Origin,
    Dummy(&'static str),
    Bob,
}

/// One clump of `DrawBG`.
#[derive(Clone, Copy, Debug)]
pub struct SkyClump {
    pub name: &'static str,
    pub layer: SkyLayer,
    pub at: At,
    /// Made and drawn only in crisis.
    pub crisis: bool,
    /// Drawn with the area's fog (`SetFogSw(1)`), not without.
    pub fog: bool,
}

impl SkyClump {
    /// A clump at the origin, no fog.
    pub const fn on(name: &'static str, layer: i16) -> SkyClump {
        SkyClump { name, layer: SkyLayer::On(layer), at: At::Origin, crisis: false, fog: false }
    }

    /// A crisis clump at the origin.
    pub const fn crisis(name: &'static str, layer: i16) -> SkyClump {
        SkyClump { crisis: true, ..SkyClump::on(name, layer) }
    }
}

/// `DrawBG`'s clumps in its order.
#[derive(Clone, Copy, Debug)]
pub struct SkyDef {
    pub clumps: &'static [SkyClump],
}

/// A clump as made: its models, and where the last `DrawBG` put it.
#[derive(Clone, Debug)]
struct Made {
    models: Vec<u32>,
    world: Mat4,
}

/// `DrawBG`'s clumps: by the def's order, None where it makes none (the
/// crisis sky outside crisis).
#[derive(Clone)]
pub struct Sky {
    def: &'static SkyDef,
    file: Rc<SceneFile>,
    made: Vec<Option<Made>>,
}

impl Sky {
    pub fn new(def: &'static SkyDef, cx: &mut BuildCx) -> Result<Sky> {
        let file = &**cx.file;
        let made = def
            .clumps
            .iter()
            .map(|c| {
                if c.crisis && !cx.crisis {
                    return Ok(None);
                }
                let world = match c.at {
                    At::Origin | At::Bob => Mat4::IDENTITY,
                    At::Dummy(name) => {
                        let d = file.ccs.find_object(name).and_then(|o| file.scene.dummies.get(&o));
                        let d = d.ok_or_else(|| Error::NotFound(name.into()))?;
                        Mat4::from_translation(Vec3::new(d.pos.x, d.pos.y, d.pos.z))
                    }
                };
                Ok(Some(Made { models: clump_models(file, &cx.name(c.name)), world }))
            })
            .collect::<Result<_>>()?;
        Ok(Sky { def, file: cx.file.clone(), made })
    }

    /// Whether clump `k` was made (a crisis clump only in crisis).
    pub fn made(&self, k: usize) -> bool {
        self.made.get(k).is_some_and(Option::is_some)
    }

    /// Clump `k`'s name.
    pub fn name(&self, k: usize) -> &'static str {
        self.def.clumps[k].name
    }

    /// Which of the crisis clumps clump `k` is, None for one drawn always.
    pub fn crisis(&self, k: usize) -> Option<usize> {
        let first = self.def.clumps.iter().position(|c| c.crisis)?;
        self.def.clumps[k].crisis.then(|| k - first)
    }
}

impl MapSystem for Sky {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let bob = others.get::<Bob>().map_or(0, Bob::z);
        for (k, (c, made)) in self.def.clumps.iter().zip(&mut self.made).enumerate() {
            let Some(made) = made else { continue };
            if c.at == At::Bob {
                made.world = Mat4::from_translation(Vec3::new(0.0, 0.0, crate::ee::f(bob)));
            }
            cx.put(Kind::Sky, Piece::Clump(k));
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Clump(k) = piece else { return None };
        let made = self.made.get(k)?.as_ref()?;
        let c = &self.def.clumps[k];
        Some(Request::Models {
            file: &self.file,
            models: Cow::Borrowed(&made.models),
            world: made.world,
            layer: c.layer.of(cx.core.hits.volume),
            rows: cx.rows.clone(),
            fog: if c.fog { cx.fog } else { Fogging::None },
            alpha: 1.0,
        })
    }
}
