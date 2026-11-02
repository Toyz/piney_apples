//! A town's water: `waterAnm[0..2]`, three copies of one animation each
//! stepped once after `SetAnm`; water 0 drawn through the picture behind it
//! (`waterUVModifi2`, [`WaterZero`]), 1 and 2 with the scroll `ccAnm::SetUV`
//! gives them, which steps by its [`WaterSpec`] and carries in the statics
//! (docs/engine/town02.md).

use std::rc::Rc;

use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::scrolls::Wrap;
use super::{BuildCx, Carried, CarriedMut, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw::{self, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::town::{WaterZero, pos_rot_zyx, water_rows};

/// A town's water scroll: its step a frame and how it wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaterSpec {
    pub step: F,
    pub wrap: Wrap,
}

impl WaterSpec {
    pub fn step(self, u: F) -> F {
        self.wrap.step(u, self.step)
    }
}

/// The scene file the water's animation is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterFile {
    /// The town's own.
    Area,
    /// A file of its own (Mac Anu's `wat1`).
    Own(&'static str),
}

/// What a town's water is made of and how its `Draw` steps it.
#[derive(Clone, Copy, Debug)]
pub struct WaterDef {
    pub file: WaterFile,
    /// `wateranmname[0]` and the model water 0 draws.
    pub anim: &'static str,
    pub model: &'static str,
    /// `waterUVModifi2`'s reach.
    pub reach: F,
    /// Its root's place (`SetMatrix_PosRotZYX`, no turn).
    pub root: V4,
    pub spec: WaterSpec,
    /// Where its scroll sits in the statics.
    pub slot: usize,
    /// The waters `Draw` steps and draws, in its order.
    pub drawn: &'static [usize],
    /// The layer of waters 1 and 2 (water 0's is objLayer).
    pub layer: i16,
    /// Whether `Draw` copies the picture into water 0's texture after it
    /// (`MakePacketDrawBuffTrans` on objLayer).
    pub copy: bool,
}

/// The water as the last `Draw` left it.
#[derive(Clone)]
pub struct Water {
    def: &'static WaterDef,
    file: Rc<SceneFile>,
    /// `waterAnm[0..2]`.
    plays: [Play; 3],
    /// The scroll (`u$1841`, `u$1777`, `u$1875`), and what the last draw
    /// passed `ccAnm::SetUV` (`vftoi12` of it).
    pub u: F,
    pub uv: u16,
    /// Water 0's texture coordinates and texture.
    pub zero: WaterZero,
}

impl Water {
    pub fn new(def: &'static WaterDef, cx: &mut BuildCx) -> Result<Water> {
        let file = match def.file {
            WaterFile::Area => cx.file.clone(),
            WaterFile::Own(stem) => Rc::new(SceneFile::read(cx.archive, stem)?),
        };
        let play = Play::new(&file, def.anim).ok_or_else(|| Error::NotFound(def.anim.into()))?;
        let mut plays = [play.clone(), play.clone(), play];
        for p in &mut plays {
            // SetMatrix_PosRotZYX, then one _AnimateForward.
            p.forward(&file);
        }
        let zero = WaterZero::new(&file, def.model, def.reach)?;
        Ok(Water { def, file, plays, u: 0, uv: 0, zero })
    }

    /// `waterAnm[k]`'s animation time.
    pub fn time(&self, k: usize) -> u32 {
        self.plays[k].time
    }

    /// The root: `SetMatrix_PosRotZYX(root, 0)`.
    pub fn root(&self) -> [V4; 4] {
        pos_rot_zyx(self.def.root, [0; 3])
    }
}

impl MapSystem for Water {
    /// `ccAnm::SetUV` of `vftoi12(u)` into water 1's U and water 2's V;
    /// `waterUVModifi2` of water 0 (its UVs, for the draw); the drawn ones
    /// stepped; the copy; the scroll stepped.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        self.uv = ee::vftoi12(self.u);
        let (volume, eye) = (cx.core.hits.volume, cx.view.eye);
        self.zero.modify(volume, &self.root(), [0, 0, 0, ONE], eye, &cx.view.world_screen);
        for &k in self.def.drawn {
            self.plays[k].forward(&self.file);
            cx.put(Kind::Water, Piece::Anim(k));
        }
        if self.def.copy {
            cx.put(Kind::Water, Piece::Copy);
        }
        self.u = self.def.spec.step(self.u);
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Anim(k) = piece else { return None };
        let (file, play, root) = (&*self.file, &self.plays[k], draw::mat(&self.root()));
        let fog = draw::Fogging::None;
        Some(match k {
            // Water 0: the picture so far as its texture, at the UVs
            // waterUVModifi2 wrote.
            0 => {
                let edit = Some((self.zero.model.object, self.zero.edits()));
                Request::Anim { file, play, root, layer: OBJ_LAYER, rows: None, fog, edit }
            }
            k => {
                let rows = Some(water_rows(file, k, self.uv));
                Request::Anim { file, play, root, layer: self.def.layer, rows, fog, edit: None }
            }
        })
    }

    fn take(&mut self, c: &Carried) {
        self.u = c.statics[self.def.slot];
    }

    fn put(&self, c: &mut CarriedMut) {
        c.statics[self.def.slot] = self.u;
    }
}
