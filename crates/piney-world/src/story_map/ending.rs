//! Quarantine's field 126, the ending's: the `EVENTAREA` `WORLD_MAN::GO(1)`
//! makes for `eventAreaNumber` 126 (QUA main 0x001b302c), gcmn 0x00305120,
//! 0x1c4 bytes, vtable 0x002766b0, no symbol. Its constructor runs the
//! base's and clears `modelArray`, `anmArray`, `bg` and the light count;
//! it loads nothing and never calls `SetStartPos`. Its `Draw` (0x00305260)
//! only sets `ccMenu` +0x0c (`panelStatus`) to 3 each frame; the other
//! slots are the base's. Event 314's block 3 then hides Kite (`pc_act 0
//! 6`) and runs the staff roll over it. docs/engine/evarea.md.

use piney_data::Result;
use piney_data::volume::Volume;

use crate::story_map::{BASE_BLOCK, BASE_STORY_MAP, BlockDef, StoryFrame, StoryMapDef, StoryRequest};
use crate::systems::{BuildCx, MapSystem, SystemDef};

/// `game.field` / `eventAreaNumber` of the ending.
pub const AREA: i32 = piney_data::area::ENDING_AREA;

/// The ending's own: `Draw`'s one store, the party's panels out.
#[derive(Clone, Debug)]
pub struct Panels;

impl Panels {
    pub fn new(_: &'static (), _: &mut BuildCx) -> Result<Panels> {
        Ok(Panels)
    }
}

impl MapSystem for Panels {
    fn scene(&mut self, x: &mut StoryFrame) {
        x.out.push(StoryRequest::PanelStatus(3));
    }
}

/// Field 126, on Quarantine alone: no file, no lights, `eventStartPos` as
/// the last story map's `SetStartPos` left it; `ccSys.bgColor` untouched
/// (the desktop's set-up left it black, and event 314 comes from the
/// desktop).
pub static ENDING: StoryMapDef = StoryMapDef {
    fields: &[AREA],
    volumes: &[Volume::Qua],
    blocks: &[BlockDef { systems: &[SystemDef::Panels(&())], ..BASE_BLOCK }],
    ..BASE_STORY_MAP
};
