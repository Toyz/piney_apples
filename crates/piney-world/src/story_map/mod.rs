//! Story maps: the `EVENTAREA` subclasses `WORLD_MAN::GO(1)` (main
//! 0x0019f8e0) makes in place of a generated field, by `game.field`, and
//! holds as `eventmap` (docs/engine/evarea.md). Each is a [`StoryMap`] that
//! `Place::Story` holds; [`build`] is `GO(1)`'s choice.

#[doc(alias = "EVENTAREA03")]
pub mod abyss;
#[doc(alias = "EVENTAREAB0")]
pub mod boss_arena;
#[doc(alias = "EVENTAREAB8")]
pub mod disc;
pub mod ending;
#[doc(alias = "EVENTAREA07")]
pub mod giant;
#[doc(alias = "EVENTAREA02")]
pub mod holy_ground;
#[doc(alias = "EVENTAREA04")]
#[doc(alias = "EVENTAREA05")]
#[doc(alias = "EVENTAREA06")]
pub mod sky;
#[doc(alias = "EVENTAREA01")]
pub mod truths_core;

use piney_data::dungeon::Rng;
use piney_desktop::{MOVE_TEX_ROWS, SCROLL_STATICS};
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
use crate::hit::Hits;
use crate::story_map::holy_ground::{Kept, StorySprite};
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
    /// `ccMenu->panelStatus = n` (field 126's `Draw`).
    PanelStatus(i16),
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
    /// `fieldrand` as the map's draws leave it, for a map that draws from
    /// it; None leaves the game's as the map found it.
    fn field_rand(&self) -> Option<&Rng> {
        None
    }
    fn field_rand_mut(&mut self) -> Option<&mut Rng> {
        None
    }
    /// The map's `EA_moveTex` rows' scroll written into gcmn's data
    /// (`evarea::scroll`).
    fn put_scrolls(&self, _gcmn: &mut [[u32; 2]; MOVE_TEX_ROWS]) {}
    /// The scroll taken from gcmn's data, for a map just made.
    fn take_scrolls(&mut self, _gcmn: &[[u32; 2]; MOVE_TEX_ROWS]) {}
    /// The map's scroll statics in the executable's data
    /// (`town::scroll_statics`), taken and handed back.
    fn take_statics(&mut self, _s: &[u32; SCROLL_STATICS]) {}
    fn put_statics(&self, _s: &mut [u32; SCROLL_STATICS]) {}
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

/// What `GO(1)` makes a map from: the disc, `game.field`, `game.areaPrev`,
/// `game.fieldPrev`, `game.server`, the save, and what the scene before
/// left. Every map's `make` takes it.
pub struct MapEnv<'a> {
    pub archive: &'a Arc<Archive>,
    pub volume: Volume,
    pub field: i32,
    pub area_prev: i32,
    /// `game.fieldPrev`.
    pub field_prev: i32,
    pub server: i32,
    pub save: &'a SaveData,
    /// `WORLD_MAN.eventStartPos` as the last map's `SetStartPos` left it.
    pub event_start: V4,
    /// `fieldrand` as the scene before left it.
    pub field_rand: Rng,
    /// gcmn's `EA_moveTex` scrolls.
    pub move_tex: [[u32; 2]; MOVE_TEX_ROWS],
    /// The executable's scroll statics.
    pub scroll_statics: [u32; SCROLL_STATICS],
    /// The field's default sound (`DEF_SE`).
    pub def_se: u32,
    /// What `WORLD_MAN::Quit` kept of the map before (area 15's).
    pub kept: Option<Kept>,
    /// The game's `ccRand`, which `EVENTAREAB8`'s rocks draw from.
    pub cc: &'a mut dyn piney_battle::rand::Rng,
}

/// A map as its `make` hands it over: made anew, or the one kept.
pub enum Made {
    New(Box<dyn StoryMap>),
    Kept(Box<dyn StoryMap>),
}

/// One `EVENTAREA` class of `GO(1)`'s switch: the fields it is made for and
/// how. Each class's quirks are in its own `make`.
pub struct MapDef {
    /// The game's class.
    pub class: &'static str,
    pub claims: fn(&MapEnv<'_>) -> bool,
    pub make: fn(&mut MapEnv<'_>) -> Result<Made>,
}

/// `GO(1)`'s classes in the order its switch tries them.
pub static MAPS: [MapDef; 8] =
    [boss_arena::DEF, holy_ground::DEF, giant::DEF, abyss::DEF, truths_core::DEF, ending::DEF, disc::DEF, sky::DEF];

/// `GO(1)`'s map for `env.field` (`eventAreaNumber`, whose
/// `EVENTAREA_INFO.model` is `model`): the first class that claims it, or
/// None for a generated field.
pub fn build(env: &mut MapEnv<'_>, model: i32) -> Result<Option<Box<dyn StoryMap>>> {
    let Some(def) = MAPS.iter().find(|d| model != 0 && (d.claims)(env)) else { return Ok(None) };
    let mut map = match (def.make)(env)? {
        // gcmn's scrolls carry on into a new map; a kept one has its own.
        Made::New(mut map) => {
            map.take_scrolls(&env.move_tex);
            map
        }
        Made::Kept(map) => map,
    };
    map.take_statics(&env.scroll_statics);
    Ok(Some(map))
}
