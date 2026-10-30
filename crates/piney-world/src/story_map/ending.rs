//! Quarantine's field 126, the ending's: the `EVENTAREA` `WORLD_MAN::GO(1)`
//! makes for `eventAreaNumber` 126 (QUA main 0x001b302c), gcmn 0x00305120,
//! 0x1c4 bytes, vtable 0x002766b0, no symbol. Its constructor runs the
//! base's and clears `modelArray`, `anmArray`, `bg` and the light count;
//! it loads nothing and never calls `SetStartPos`. Its `Draw` (0x00305260)
//! only sets `ccMenu` +0x0c (`panelStatus`) to 3 each frame; the other
//! slots are the base's. Event 314's block 3 then hides Kite (`pc_act 0
//! 6`) and runs the staff roll over it. docs/engine/evarea.md.

use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::area::AreaCore;
use crate::ee::V4;
use crate::story_map::event_hits;
use crate::story_map::holy_ground::StorySprite;
use crate::story_map::{Made, MapDef};
use crate::story_map::{StoryFrame, StoryMap, StoryRequest};
use crate::town::{TownLights, TownView};

/// `game.field` / `eventAreaNumber` of the ending.
pub const AREA: i32 = piney_data::area::ENDING_AREA;

/// The ending's empty map: no file, no lights, `eventStartPos` as the
/// last story map's `SetStartPos` left it.
pub struct EndArea {
    core: AreaCore,
}

impl EndArea {
    /// `GO(1)`'s `new` and constructor: an event area with nothing in it.
    /// `start` is `WORLD_MAN.eventStartPos`, which it does not set.
    pub fn new(volume: piney_data::volume::Volume, def_se: u32, start: V4) -> EndArea {
        let hits = event_hits(volume, def_se, Vec::new());
        let lights = TownLights { ambient: Vec3::ZERO, lights: Vec::new(), fog: None };
        let file = Rc::new(SceneFile::empty());
        EndArea { core: AreaCore { file, hits, lights, field_rand: None, start } }
    }
}

impl StoryMap for EndArea {
    fn core(&self) -> &AreaCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut AreaCore {
        &mut self.core
    }
    /// The constructor loads no stream (+0x1a4 stays 0).
    fn file(&self) -> Option<&SceneFile> {
        None
    }
    /// `ccSys.bgColor` is not touched: the desktop's set-up left it black,
    /// and event 314 comes from the desktop.
    fn clear(&self) -> [u8; 3] {
        [0, 0, 0]
    }
    /// `Draw`'s one store: the party's panels out.
    fn frame(&mut self, x: &mut StoryFrame) {
        x.out.push(StoryRequest::PanelStatus(3));
    }
    fn draw(&mut self, _layers: &mut Layers, _to_screen: Mat4, _v: &TownView, _step: bool) -> Vec<StorySprite> {
        Vec::new()
    }
}

/// `GO(1)`'s field 126, on Quarantine alone.
pub const DEF: MapDef = MapDef {
    class: "EVENTAREA",
    claims: |e| e.field == AREA && e.volume == piney_data::volume::Volume::Qua,
    make: |e| Ok(Made::New(Box::new(EndArea::new(e.volume, e.def_se, e.event_start)))),
};
