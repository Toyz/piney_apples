//! Quarantine's field 126, the ending's: the `EVENTAREA` `WORLD_MAN::GO(1)`
//! makes for `eventAreaNumber` 126 (QUA main 0x001b302c), gcmn 0x00305120,
//! 0x1c4 bytes, vtable 0x002766b0, no symbol. Its constructor runs the
//! base's and clears `modelArray`, `anmArray`, `bg` and the light count;
//! it loads nothing and never calls `SetStartPos`. Its `Draw` (0x00305260)
//! only sets `ccMenu` +0x0c (`panelStatus`) to 3 each frame; the other
//! slots are the base's. Event 314's block 3 then hides Kite (`pc_act 0
//! 6`) and runs the staff roll over it. docs/engine/evarea.md.

use std::any::Any;

use glam::{Mat4, Vec3};
use piney_desktop::layers::Layers;

use crate::ee::{self, F, ONE, V4};
use crate::evarea::{BOUNDS, StorySprite};
use crate::hit::Hits;
use crate::story_map::{StoryFrame, StoryMap, StoryRequest};
use crate::town::{TownLights, TownView};

/// `game.field` / `eventAreaNumber` of the ending.
pub const AREA: i32 = piney_data::area::ENDING_AREA;

/// The ending's empty map.
pub struct EndArea {
    hits: Hits,
    lights: TownLights,
    /// `eventStartPos` as the last story map's `SetStartPos` left it.
    start: V4,
}

impl EndArea {
    /// `GO(1)`'s `new` and constructor: an event area with nothing in it.
    /// `start` is `WORLD_MAN.eventStartPos`, which it does not set.
    pub fn new(volume: piney_data::volume::Volume, def_se: u32, start: V4) -> EndArea {
        let hits =
            Hits { volume, area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        EndArea { hits, lights: TownLights { ambient: Vec3::ZERO, lights: Vec::new(), fog: None }, start }
    }
}

impl StoryMap for EndArea {
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    /// `SetCharPosition`'s "any other" map: the leader at `eventStartPos`,
    /// the others at (+300, +150), (-300, +150) and (0, +300).
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        let [x, y, z, dirc] = self.start;
        let starts = [
            [ee::add(0x4396_0000, x), ee::add(0x4316_0000, y), z, ONE],
            [ee::sub(x, 0x4396_0000), ee::add(0x4316_0000, y), z, ONE],
            [x, ee::add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
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
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
