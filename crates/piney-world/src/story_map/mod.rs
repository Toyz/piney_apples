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
use crate::hit::{HitModel, Hits};
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
    /// `eventStartPos` as the map's `SetStartPos` left it: the place, the
    /// facing in w.
    fn start(&self) -> V4;
    /// How the others stand about the leader.
    fn spread(&self) -> StartSpread {
        StartSpread::BEHIND
    }
    /// `WORLD_MAN::SetCharPosition` in a story map: the leader, his facing,
    /// the others.
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        party_starts(self.start(), self.spread())
    }
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
}

/// Where `WORLD_MAN::SetCharPosition`'s event area branches put the two
/// others beside the leader: `side` to either side, `back` along y (the
/// third is always 300 along y).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartSpread {
    pub side: F,
    pub back: F,
}

impl StartSpread {
    /// (+-300, +150): main 0x001a1414, most maps.
    pub const BEHIND: StartSpread = StartSpread { side: 0x4396_0000, back: 0x4316_0000 };
    /// (+-200, -150): main 0x001a1190, fields 16 and 66, the mirror.
    pub const AHEAD: StartSpread = StartSpread { side: 0x4348_0000, back: 0xc316_0000 };
}

/// The leader at `start` facing its w; the others at `spread` and (0, +300)
/// from him, facing as he does.
pub fn party_starts(start: V4, spread: StartSpread) -> (V4, F, [V4; 3]) {
    use crate::ee::{ONE, add, sub};
    let [x, y, z, dirc] = start;
    let StartSpread { side, back } = spread;
    let others =
        [[add(x, side), add(y, back), z, ONE], [sub(x, side), add(y, back), z, ONE], [x, add(y, 0x4396_0000), z, ONE]];
    ([x, y, z, ONE], dirc, others)
}

/// A story map's collision as `GO(1)` sets it: `game.area` 1, the event
/// area's flag and bounds, no height map, `defSE`; the models' Hit chunks
/// are each map's.
pub fn event_hits(volume: Volume, def_se: u32, models: Vec<HitModel>) -> Hits {
    let bounds = Some(holy_ground::BOUNDS);
    Hits { volume, area: 1, bounds, heights: None, def_se, event_area: true, models, ..Hits::default() }
}

/// The Hit chunks `STATICMODEL` enables: each model's own (`parent`), in
/// the order the models were built.
pub fn model_hits(models: impl IntoIterator<Item = u32>, hit_models: &[HitModel]) -> Vec<HitModel> {
    models.into_iter().filter_map(|m| hit_models.iter().find(|h| h.parent == m).cloned()).collect()
}

/// The lights a story map puts in the group: its light animation `anim`'s
/// ambient and each light of `names` at frame 1 (one `_AnimateForward`),
/// with their objects. A name the file lacks is an error, or skipped with
/// `skip_missing` (`eventareaB0Light`'s omni lights).
pub fn light_group<S: AsRef<str>>(
    file: &piney_desktop::assets::SceneFile,
    anim: &str,
    names: impl IntoIterator<Item = S>,
    skip_missing: bool,
) -> Result<(glam::Vec3, Vec<crate::town::Light>, Vec<u32>)> {
    use piney_data::Error;
    let ai = file.anim(anim).ok_or_else(|| Error::NotFound(anim.into()))?;
    let (ambient, records) = crate::town::anim_lights(file, ai)?;
    let (mut group, mut objects) = (Vec::new(), Vec::new());
    for name in names {
        let name = name.as_ref();
        let Some(obj) = file.ccs.find_object(name) else {
            if skip_missing {
                continue;
            }
            return Err(Error::NotFound(name.into()));
        };
        objects.push(obj);
        group.extend(records.iter().find(|l| l.object == obj).map(|l| l.at(256)));
    }
    Ok((ambient.unwrap_or(glam::Vec3::ZERO), group, objects))
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
