//! `DrawBG`'s clumps: the sky, the sun, the cloud layers and mountains, and
//! in crisis (`town0Nd`) the clumps over them, each at the origin or a
//! dummy on its own layer, its materials offset by the scrolls
//! ([`super::scrolls::Scrolls`]) as `DrawBG` writes them.

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::scrolls::Scrolls;
use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
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

/// Where a clump draws: at the origin, or at a dummy's position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Origin,
    Dummy(&'static str),
}

/// Which of a material's offsets a scroll writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axes {
    U,
    V,
    UV,
}

/// A material `DrawBG` offsets: by `vftoi12` of scroll `scroll`, less the
/// material's crop, in `axes`.
#[derive(Clone, Copy, Debug)]
pub struct MatScroll {
    pub mat: &'static str,
    pub scroll: usize,
    pub axes: Axes,
}

/// One clump of `DrawBG`.
#[derive(Clone, Copy, Debug)]
pub struct SkyClump {
    pub name: &'static str,
    pub layer: SkyLayer,
    pub at: At,
    pub mats: &'static [MatScroll],
    /// Made and drawn only in crisis.
    pub crisis: bool,
}

impl SkyClump {
    /// A clump at the origin with no scroll.
    pub const fn on(name: &'static str, layer: i16) -> SkyClump {
        SkyClump { name, layer: SkyLayer::On(layer), at: At::Origin, mats: &[], crisis: false }
    }

    /// A crisis clump at the origin, its materials scrolled.
    pub const fn crisis(name: &'static str, layer: i16, mats: &'static [MatScroll]) -> SkyClump {
        SkyClump { name, layer: SkyLayer::On(layer), at: At::Origin, mats, crisis: true }
    }
}

impl MatScroll {
    pub const fn new(mat: &'static str, scroll: usize, axes: Axes) -> MatScroll {
        MatScroll { mat, scroll, axes }
    }
}

/// `DrawBG`'s clumps in its order.
#[derive(Clone, Copy, Debug)]
pub struct SkyDef {
    pub clumps: &'static [SkyClump],
}

/// A clump as made: its models, where, and the offsets `DrawBG` last wrote.
#[derive(Clone, Debug)]
struct Made {
    models: Vec<u32>,
    world: Mat4,
    rows: HashMap<u32, [u8; 2]>,
}

/// `DrawBG`'s clumps: by the def's order, None where it makes none (the
/// crisis sky outside crisis).
#[derive(Clone, Debug)]
pub struct Sky {
    def: &'static SkyDef,
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
                    At::Origin => Mat4::IDENTITY,
                    At::Dummy(name) => {
                        let d = file.ccs.find_object(name).and_then(|o| file.scene.dummies.get(&o));
                        let d = d.ok_or_else(|| Error::NotFound(name.into()))?;
                        Mat4::from_translation(Vec3::new(d.pos.x, d.pos.y, d.pos.z))
                    }
                };
                Ok(Some(Made { models: clump_models(file, c.name), world, rows: HashMap::new() }))
            })
            .collect::<Result<_>>()?;
        Ok(Sky { def, made })
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

/// The offsets `mats` take from the scrolls (STROW values).
fn rows(file: &SceneFile, mats: &[MatScroll], scrolls: Option<&Scrolls>) -> HashMap<u32, [u8; 2]> {
    let mut rows = HashMap::new();
    for m in mats {
        let Some(mat) = file.ccs.find_object(m.mat) else { continue };
        let Some(info) = file.scene.materials.get(&mat) else { continue };
        let uv = scrolls.map_or(0, |s| s.uv(m.scroll));
        let du = matches!(m.axes, Axes::U | Axes::UV).then(|| uv.wrapping_sub(info.crop_u)).unwrap_or(0);
        let dv = matches!(m.axes, Axes::V | Axes::UV).then(|| uv.wrapping_sub(info.crop_v)).unwrap_or(0);
        rows.insert(mat, [(du >> 4) as u8, (dv >> 4) as u8]);
    }
    rows
}

impl MapSystem for Sky {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let scrolls = others.get_mut::<Scrolls>().map(|s| &*s);
        for (k, (c, made)) in self.def.clumps.iter().zip(&mut self.made).enumerate() {
            let Some(made) = made else { continue };
            if !c.mats.is_empty() {
                made.rows = rows(&cx.core.file, c.mats, scrolls);
            }
            cx.put(Kind::Sky, Piece::Clump(k));
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Clump(k) = piece else { return None };
        let made = self.made.get(k)?.as_ref()?;
        Some(Request::Models {
            file: &cx.core.file,
            models: &made.models,
            world: made.world,
            layer: self.def.clumps[k].layer.of(cx.core.hits.volume),
            rows: made.rows.clone(),
            fog: crate::draw::Fogging::None,
        })
    }
}
