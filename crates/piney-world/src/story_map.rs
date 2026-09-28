//! Story maps: the `EVENTAREA` subclasses `WORLD_MAN::GO(1)` (main
//! 0x0019f8e0) makes in place of a generated field, by `game.field`, and
//! holds as `eventmap` (docs/engine/evarea.md). Each is a [`StoryMap`] that
//! `Place::Story` holds; [`build`] is `GO(1)`'s choice.

use std::any::Any;
use std::sync::Arc;

use glam::Mat4;
use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_desktop::layers::Layers;

use crate::camera::Camera;
use crate::ee::{F, V4};
use crate::evarea::{EventArea, Kept, StorySprite};
use crate::evarea_b0::Arena;
use crate::evarea03::Area43;
use crate::evarea07::Giant;
use crate::hit::Hits;
use crate::town::{TownLights, TownView};

/// What `WORLD_MAN::Enter` does from a story map's door.
pub enum Enter {
    /// Nothing (the arenas).
    Stay,
    /// `ChangeBlock`, then `ChangeScene` to this block.
    Block(i32),
    /// `ChangeArea(2, 0)`: the area's dungeon.
    Dungeon,
}

/// What a story map's own scene asks of the field and the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryRequest {
    /// `ccEvent::MenuBan` / `MenuClr` (the menus' part; the field does the
    /// camera's and the party's).
    MenuBan(bool),
    /// `ccMsg->Open(rec, rec->name, -1, -1)`: a `ccEvMsgData` by address.
    Message { rec: u32 },
    /// `ccGame::ChangeArea(a, n)`.
    ChangeArea(i32, i32),
}

/// The frame a story map's scene sees: the cameras, the save, the server,
/// whether the message it opened has closed (`ccMsg->Check(0)`), and what
/// it asks.
pub struct StoryFrame<'a> {
    pub camera: &'a mut Camera,
    pub save: &'a SaveData,
    pub server: i32,
    pub message_closed: bool,
    pub out: Vec<StoryRequest>,
}

/// An `EVENTAREA` as the field's tasks use it.
pub trait StoryMap: Any {
    fn hits(&self) -> &Hits;
    fn hits_mut(&mut self) -> &mut Hits;
    fn lights(&self) -> &TownLights;
    /// `WORLD_MAN::SetCharPosition` in a story map: the leader, his facing,
    /// the others.
    fn start_positions(&self) -> (V4, F, [V4; 3]);
    /// `ccSys.bgColor` as the map leaves it.
    fn clear(&self) -> [u8; 3];
    /// `WORLD_MAN::Enter` from `block`.
    fn enter(&mut self, _block: i32, _area_prev: i32) -> Result<Enter> {
        Ok(Enter::Stay)
    }
    /// The part of `Draw` that is a scene of its own (area 43's fly-over).
    fn frame(&mut self, _x: &mut StoryFrame) {}
    /// `Draw` into the field's layers, stepping only when `step`; the
    /// sprites for the effects to draw.
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite>;
    /// [`StoryMap::draw`] with the game's `ccRand` (the battle's), for a
    /// map whose `Draw` draws from it (`EVENTAREAB8`'s rocks).
    fn draw_rand(
        &mut self,
        layers: &mut Layers,
        to_screen: Mat4,
        v: &TownView,
        step: bool,
        _cc: &mut dyn piney_battle::rand::Rng,
    ) -> Vec<StorySprite> {
        self.draw(layers, to_screen, v, step)
    }
    /// `WORLD_MAN::GetTransMode()` and `GetTransCenter`: the centre the
    /// party rides about, when the map carries it.
    fn trans_center(&self) -> Option<V4> {
        None
    }
    /// The map's scene file, where `marker_pos` finds `markerEvTbl`'s
    /// dummies (`WORLD_MAN` +0x444's stream); None where not ported.
    fn file(&self) -> Option<&piney_desktop::assets::SceneFile> {
        None
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

/// Where `GO(1)` makes a map: the volume, `game.field`, `game.areaPrev`,
/// `game.fieldPrev`, `game.server`, and the save.
pub struct At<'a> {
    pub volume: Volume,
    pub field: i32,
    pub area_prev: i32,
    /// `game.fieldPrev`.
    pub field_prev: i32,
    pub server: i32,
    pub save: &'a SaveData,
}

/// `GO(1)`'s map for `at.field` (its `EVENTAREA_INFO.model` `model`), or
/// None for a generated field. Area 15's map comes back as `WORLD_MAN::Quit`
/// kept it (`kept`, from a field); area 16's starts by `game.areaPrev`.
/// `cc` is the game's `ccRand`, which `EVENTAREAB8`'s rocks draw from.
pub fn build(
    archive: &Arc<Archive>,
    at: &At,
    model: i32,
    kept: Option<Kept>,
    def_se: u32,
    cc: &mut dyn piney_battle::rand::Rng,
) -> Result<Option<Box<dyn StoryMap>>> {
    let (field, area_prev) = (at.field, at.area_prev);
    let map: Box<dyn StoryMap> = match field {
        f if crate::evarea_b0::is_arena(f) => Box::new(Arena::new(archive, f, def_se)?),
        _ if model == 0 => return Ok(None),
        crate::evarea::AREA => {
            let kept = match kept {
                Some(Kept::Event(e)) if area_prev == crate::area::kind::FIELD => Some(e),
                _ => None,
            };
            EventArea::for_scene(archive, kept, def_se)?
        }
        crate::evarea07::AREA => Box::new(Giant::new(archive, def_se, area_prev, 0)?),
        crate::evarea03::AREA => Box::new(Area43::new(archive, at.volume, at.save, at.server, def_se)?),
        crate::evarea01::AREA => Box::new(crate::evarea01::Area13::new(archive, at.volume, def_se)?),
        f if crate::evarea_b8::is_disc(f) => {
            Box::new(crate::evarea_b8::DiscArea::new(archive, at.volume, f, at.field_prev, def_se, cc)?)
        }
        _ => return Ok(None),
    };
    Ok(Some(map))
}

impl StoryMap for EventArea {
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        EventArea::start_positions(self)
    }
    fn clear(&self) -> [u8; 3] {
        EventArea::clear(self)
    }
    /// Area 15's door: `ChangeBlock`, then the scene to that block.
    fn enter(&mut self, block: i32, _area_prev: i32) -> Result<Enter> {
        EventArea::enter(self, block).map(Enter::Block)
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        EventArea::draw(self, layers, to_screen, v.eye, v.player, &v.flare, v.puppet_show, step)
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

impl StoryMap for Arena {
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        Arena::start_positions(self)
    }
    fn clear(&self) -> [u8; 3] {
        crate::evarea_b0::BG_COLOR
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        Arena::draw(self, layers, to_screen, v.player, step)
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

impl StoryMap for Giant {
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        Giant::start_positions(self)
    }
    fn clear(&self) -> [u8; 3] {
        Giant::clear(self)
    }
    /// Area 16: from block 0 its dungeon, else back to block 0.
    fn enter(&mut self, block: i32, area_prev: i32) -> Result<Enter> {
        Ok(match Giant::enter(self, block, area_prev)? {
            None => Enter::Dungeon,
            Some(b) => Enter::Block(b),
        })
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        Giant::draw(self, layers, to_screen, v, step)
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
