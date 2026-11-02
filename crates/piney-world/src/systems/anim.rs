//! A `ccAnm` a class steps and draws itself, at a root of its own: area
//! 15's `revAnm` (the church's lanterns upside down under its floor),
//! field 12's `bodyanm`.

use std::rc::Rc;

use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw;
use crate::ee::{F, V4};

/// The animation, its file (the area's, or one of its own), its root
/// (`SetMatrix_PosRotZYX(pos, rot)`) and layer. It draws with the area's
/// fog.
#[derive(Clone, Copy, Debug)]
pub struct AnimDef {
    pub anim: &'static str,
    pub file: Option<&'static str>,
    pub pos: V4,
    pub rot: [F; 3],
    pub layer: i16,
}

/// The animation as the last `Draw` stepped it.
#[derive(Clone)]
pub struct Anim {
    def: &'static AnimDef,
    file: Rc<SceneFile>,
    pub play: Play,
}

impl Anim {
    pub fn new(def: &'static AnimDef, cx: &mut BuildCx) -> Result<Anim> {
        let file = match def.file {
            Some(stem) => Rc::new(SceneFile::read(cx.archive, stem)?),
            None => cx.file.clone(),
        };
        let play = Play::new(&file, def.anim).ok_or_else(|| Error::NotFound(def.anim.into()))?;
        Ok(Anim { def, file, play })
    }

    /// Its root.
    pub fn root(&self) -> [V4; 4] {
        crate::town::pos_rot_zyx(self.def.pos, self.def.rot)
    }
}

impl MapSystem for Anim {
    /// Stepped a frame, then drawn.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook == Hook::Draw {
            self.play.forward(&self.file);
            cx.put(Kind::Anim, Piece::Anim(0));
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Anim(_) = piece else { return None };
        let root = draw::mat(&self.root());
        let (file, play, layer, fog) = (&*self.file, &self.play, self.def.layer, cx.fog);
        Some(Request::Anim { file, play, root, layer, rows: None, fog, edit: None })
    }
}
