//! `DrawBG`'s clumps: the sky, the sun, the cloud layers and mountains, and
//! in crisis (`town0Nd`) the clumps over them, each at the origin, a dummy,
//! the bob ([`super::bob::Bob`]) or a field's centre turned by its scrolls'
//! turns on its own layer, its materials offset as the scrolls
//! ([`super::scrolls::Scrolls`]) last wrote them.

use std::borrow::Cow;
use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::bob::Bob;
use super::scrolls::Scrolls;
use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::Fogging;
use crate::ee::ONE;
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

/// Where a clump draws: at the origin, at a dummy's position, raised by the
/// bob (`EVENTAREA07`'s), or at a field's centre (`WORLD` +0x6140, z 0)
/// turned about z by the scrolls' turn `k` (`WORLD::DrawBG`'s
/// `SetMatrix_PosRotZYX`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Origin,
    Dummy(&'static str),
    Bob,
    Centre(Option<usize>),
}

/// Which clump: one of the file's by name, or `WORLD::DrawBG`'s by its
/// background row (`BGTBL`'s four, or `BGTBL2`'s `p` and four, by slot;
/// field type 6's aurora).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClumpName {
    Named(&'static str),
    Row(usize),
    Aurora,
}

/// One clump of `DrawBG`.
#[derive(Clone, Copy, Debug)]
pub struct SkyClump {
    pub name: ClumpName,
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
        SkyClump { name: ClumpName::Named(name), layer: SkyLayer::On(layer), at: At::Origin, crisis: false, fog: false }
    }

    /// A field's: the row's clump or the aurora at the centre, turned by
    /// turn `turn`, no fog.
    pub const fn field(name: ClumpName, layer: i16, turn: Option<usize>) -> SkyClump {
        SkyClump { name, layer: SkyLayer::On(layer), at: At::Centre(turn), crisis: false, fog: false }
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
/// crisis sky outside crisis); each one's name as the file has it.
#[derive(Clone)]
pub struct Sky {
    def: &'static SkyDef,
    file: Rc<SceneFile>,
    made: Vec<Option<Made>>,
    names: Vec<String>,
}

impl Sky {
    pub fn new(def: &'static SkyDef, cx: &mut BuildCx) -> Result<Sky> {
        let file = &**cx.file;
        let background = cx.field.and_then(|f| f.field).map(|f| &f.scene.background);
        // A row's clump the row lacks is made empty (its slot drawn with
        // nothing); a row with no aurora makes none.
        let names: Vec<Option<String>> = def
            .clumps
            .iter()
            .map(|c| match c.name {
                ClumpName::Named(name) => Some(cx.name(name).into_owned()),
                ClumpName::Row(k) => {
                    Some(background.and_then(|b| b.clumps.get(k)).map_or_else(String::new, |n| n.to_string()))
                }
                ClumpName::Aurora => background.and_then(|b| b.aurora).map(String::from),
            })
            .collect();
        let made = def
            .clumps
            .iter()
            .zip(&names)
            .map(|(c, name)| {
                let Some(name) = name.as_deref().filter(|_| !c.crisis || cx.crisis) else { return Ok(None) };
                let world = match c.at {
                    At::Origin | At::Bob | At::Centre(_) => Mat4::IDENTITY,
                    At::Dummy(name) => {
                        let d = file.ccs.find_object(name).and_then(|o| file.scene.dummies.get(&o));
                        let d = d.ok_or_else(|| Error::NotFound(name.into()))?;
                        Mat4::from_translation(Vec3::new(d.pos.x, d.pos.y, d.pos.z))
                    }
                };
                Ok(Some(Made { models: clump_models(file, name), world }))
            })
            .collect::<Result<_>>()?;
        let names = names.into_iter().map(Option::unwrap_or_default).collect();
        Ok(Sky { def, file: cx.file.clone(), made, names })
    }

    /// Whether clump `k` was made (a crisis clump only in crisis).
    pub fn made(&self, k: usize) -> bool {
        self.made.get(k).is_some_and(Option::is_some)
    }

    /// Clump `k`'s name as the file has it.
    pub fn name(&self, k: usize) -> &str {
        &self.names[k]
    }

    /// Clump `k` as the def gives it.
    pub fn clump(&self, k: usize) -> &SkyClump {
        &self.def.clumps[k]
    }

    /// Where the last `DrawBG` put clump `k`.
    pub fn world(&self, k: usize) -> Option<Mat4> {
        self.made.get(k)?.as_ref().map(|m| m.world)
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
        let turn = others.get::<Scrolls>().map_or([0; 2], |s| s.turn);
        let [x, y] = cx.core.hits.center;
        for (k, (c, made)) in self.def.clumps.iter().zip(&mut self.made).enumerate() {
            let Some(made) = made else { continue };
            match c.at {
                At::Bob => made.world = Mat4::from_translation(Vec3::new(0.0, 0.0, crate::ee::f(bob))),
                At::Centre(t) => {
                    let rz = t.map_or(0, |t| turn[t]);
                    made.world = crate::draw::mat(&crate::town::pos_rot_zyx([x, y, 0, ONE], [0, 0, rz]));
                }
                At::Origin | At::Dummy(_) => {}
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
