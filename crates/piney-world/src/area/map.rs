//! `WORLD_MAN`'s area: what `GO(area)` made for the scene, a Root Town
//! (`ROOTTOWN0n`), a story map of its own (`EVENTAREA..`), a generated
//! field (`WORLD`) or its dungeon (`DUNGEON`). The game's four classes share
//! no base: `WORLD_MAN` holds a pointer to each and every one of its calls
//! that reaches the area (`GetHeight`, `Enter`, `ShowMap`, `RoomSelect`,
//! `GoField`, `EntryGimmick`, `Get2DMapInfo`, `ccThFieldDisp`'s `Draw`)
//! switches on `WORLD_MAN.area`. [`AreaMap`] is that switch written once:
//! its methods are `WORLD_MAN`'s, each class answers as its own code does.

use std::any::Any;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::Fonts;
use piney_desktop::layers::Layers;

use super::{AreaCore, Scene, WorldMan};
use crate::camera::Camera;
use crate::ee::{self, F, ONE, V4};
use crate::map::{Entries, MapState};
use crate::sprite::EffSprite;
use crate::story_map::{Kept, StoryRequest};
use crate::systems::{MapEvent, MapView, Report, Signal};

/// `ChangeScene` or `ChangeArea` as an area's own way out asks it
/// (`WORLD_MAN::Enter`, `GoField`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// `ChangeScene(a, t, fd, d, f, b)`; -2 keeps a value.
    Scene { area: i32, town: i32, field: i32, dungeon: i32, floor: i32, block: i32 },
    /// `ChangeArea(a, n)`.
    Area { area: i32, n: i32 },
}

/// `ccThFieldDisp`'s frame of the area (`Draw`): what it reads, and what it
/// leaves for the effects, the sound and the scene.
pub struct Disp<'a> {
    pub layers: &'a mut Layers,
    pub to_screen: Mat4,
    pub view: &'a MapView,
    /// The tasks run (asleep the area is drawn as its class draws it then).
    pub awake: bool,
    /// The game's `ccRand`, for a `Draw` that draws from it.
    pub cc: &'a mut dyn piney_battle::rand::Rng,
    /// The save (the area bans `MoveDoor`'s `SetDoor` reads).
    pub save: &'a SaveData,
    /// `ccCheckActiveObject()`, and its `(floor, block)` form.
    pub clear_all: bool,
    pub clear: &'a dyn Fn(i32, i32) -> bool,
    /// What a story map's own scene moves and reads.
    pub camera: &'a mut Camera,
    pub server: i32,
    pub message_closed: bool,
    /// Out: the `ccEff`s it asked for, its calls (sounds, flashes, smoke),
    /// its scene's requests, and `SetPathFindingMap` due.
    pub sprites: Vec<EffSprite>,
    pub events: Vec<MapEvent>,
    pub story: Vec<StoryRequest>,
    pub path_map: bool,
}

/// What `WORLD_MAN::Enter`, `RoomSelect` and `ShowMap` read of the scene.
pub struct EnterCx<'a> {
    pub player: V4,
    pub scene: &'a Scene,
    pub clear: &'a dyn Fn(i32, i32) -> bool,
    pub save: &'a mut SaveData,
    /// `WORLD_MAN.lastRoom` (+0x100).
    pub last_room: &'a mut i32,
    pub cc: &'a mut dyn piney_battle::rand::Rng,
}

/// The minimap's frame (`WORLD::DrawMiniMap`, `DUNGEON::MakeMiniMap` and
/// `DrawMap`, `ROOTTOWN0n::DrawMap`) after `ccThMenu` set its alpha.
pub struct MapFrame<'a> {
    pub st: &'a mut MapState,
    pub scene: &'a Scene,
    pub pos: V4,
    pub dirc: V4,
    pub awake: bool,
    pub in_battle: bool,
    pub fonts: Option<&'a Fonts>,
    pub ents: &'a mut Entries,
    pub ctx: &'a mut Ctx,
}

/// `WORLD_MAN::SetCharPosition`'s other three places (`StartPos[1..3]`,
/// for party slots 1 and 2; `ccGetStartPositions` asks for all three), from
/// the leader's `leader` facing `dirc`. In a field (a generated one, not an
/// `EVENTAREA` map): (+200, +100), (-200, +100) and (0, +300) from him,
/// facing as he does. In a dungeon: (300, -150), (-300, -150) and
/// (0, -300) turned by his facing (`sceVu0RotMatrix` of (0, 0, dirc)),
/// then added to his place.
pub fn party_starts(area: i32, leader: V4, dirc: F) -> [V4; 3] {
    let [x, y, z, _] = leader;
    if area == super::kind::DUNGEON {
        let m = piney_data::anim::rot_bits([0, 0, dirc]);
        let at = |dx: F, dy: F| {
            let v = ee::apply(&m, [dx, dy, 0, ONE]);
            [ee::add(v[0], x), ee::add(v[1], y), ee::add(v[2], z), ONE]
        };
        return [at(0x4396_0000, 0xc316_0000), at(0xc396_0000, 0xc316_0000), at(0, 0xc396_0000)];
    }
    [
        [ee::add(0x4348_0000, x), ee::add(0x42c8_0000, y), z, ONE],
        [ee::sub(x, 0x4348_0000), ee::add(0x42c8_0000, y), z, ONE],
        [x, ee::add(0x4396_0000, y), z, ONE],
    ]
}

/// The area `WORLD_MAN::GO` made, as `WORLD_MAN`'s calls reach it.
pub trait AreaMap: Any {
    /// The scene file, collision, light group, `fieldrand` and start.
    fn core(&self) -> &AreaCore;
    fn core_mut(&mut self) -> &mut AreaCore;

    /// `WORLD_MAN::SetCharPosition` (main 0x001a1190) and
    /// `ccGetStartPositions`: the leader, his facing, `StartPos[1..3]`.
    fn start(&self, scene: &Scene) -> (V4, F, [V4; 3]);

    /// `WORLD_MAN::SetCenter` at the leader's feet as `ccPlayer` is made.
    fn set_center(&mut self, x: F, y: F) {
        self.core_mut().hits.center = [x, y];
    }

    /// `ccSys.bgColor` as the area leaves it (None: as it was).
    fn clear(&self) -> Option<[u8; 3]> {
        None
    }

    /// What outlives the scene, into the state the next mode takes.
    fn put(&self, out: &mut SaveState);

    /// `ccThFieldDisp`'s `Draw`.
    fn disp(&mut self, cx: &mut Disp);

    /// The stream `ccEvMarker2Pos` reads (`WORLD_MAN` +0x444 on an event
    /// map, the dungeon's +0xd3884); a plain field's stores to address 0.
    fn marker_file(&self) -> Option<&SceneFile> {
        None
    }

    /// `WORLD_MAN.eventStartPos` as the area leaves it (x, y, z, facing),
    /// for a story map after it that sets none.
    fn event_start(&self) -> Option<[F; 4]> {
        None
    }

    /// `WORLD_MAN.specialRoom` (+0x160): -1 outside a dungeon.
    fn special_room(&self) -> i32 {
        -1
    }

    /// `WORLD_MAN::Enter(pos)` (main 0x0019dda0): the way out the ground's
    /// Enter bit leads to.
    fn enter(&mut self, _cx: &mut EnterCx) -> Option<Exit> {
        None
    }

    /// `WORLD_MAN::GoField` (main 0x0019e410): from a dungeon back to its
    /// field, or for field type 4 to the first dungeon; None elsewhere.
    fn go_field(&mut self, _no_field: bool, _scene: &Scene, _last_room: i32) -> Option<Exit> {
        None
    }

    /// `WORLD_MAN::RoomSelect(floor, block)` (main 0x0019dca0): true when
    /// the area built the room (a dungeon).
    fn room_select(&mut self, _floor: usize, _block: usize, _cx: &mut EnterCx) -> bool {
        false
    }

    /// `WORLD_MAN::ShowMap` (main 0x001a3ee0), one call: true when done.
    fn show_map(&mut self, _field_model: i32, _cx: &mut EnterCx) -> bool {
        true
    }

    /// The events' `open_door`, `close_door` and `ccEntryEventMng`'s
    /// `CloseDoor` on room `(floor, block)` (`DUNGEON::OpenDoor`,
    /// `CloseDoor2`, `CloseDoor`): nothing outside a dungeon.
    fn open_door(&mut self, _floor: usize, _block: usize) {}
    fn close_door(&mut self, _floor: usize, _block: usize) {}
    fn close_door2(&mut self) {}

    /// `WORLD_MAN`'s dungeon entrance (`dungeonPos[0]`), a field's.
    fn dungeon_pos(&self) -> Option<V4> {
        None
    }

    /// `WORLD_MAN::Get2DMapPtr` and `Get2DMapInfo` of room `block`: a
    /// dungeon's floor map and the room's window.
    fn map_2d(&self) -> Vec<u8> {
        Vec::new()
    }
    fn map_2d_info(&self, _block: i32) -> Option<[i32; 3]> {
        None
    }

    /// `restoreEntry` and `WORLD_MAN::EntryGimmick` as `ccThEntryCtrl`
    /// sets up: what the area puts on the entry control.
    fn entries(&mut self, _scene: &Scene, _wm: &WorldMan) -> crate::combat::AreaEntries {
        crate::combat::AreaEntries::default()
    }

    /// Whether `ccThEntryCtrlDelete` keeps `g_entryList` for the area's
    /// next room (a dungeon), and the list kept.
    fn keeps_entries(&self) -> bool {
        false
    }
    fn keep_entries(&mut self, _kept: piney_battle::entry::KeptEntries, _actors: Vec<Option<crate::combat::Actor>>) {}

    /// The battle's call on the map, and what the map tells the battle.
    fn signal(&mut self, _s: Signal) {}
    fn report(&self) -> Report {
        Report::default()
    }

    /// Where `ccBossEntryStart` centres the boss, if not on the player.
    fn boss_centre(&self) -> Option<V4> {
        None
    }

    /// What `WORLD_MAN` keeps of the area for the next scene, with the
    /// area's other dungeons.
    fn into_kept(self: Box<Self>, _dungeons: crate::dungeon::Dungeons) -> Option<Kept> {
        None
    }

    /// The minimap `WORLD::Init` or the `DUNGEON` constructor makes.
    fn setup_map(&mut self, _archive: &Arc<Archive>, _volume: Volume, _scene: &Scene) -> piney_data::Result<()> {
        Ok(())
    }

    /// The minimap's frame; true when it shows the map status.
    fn minimap(&mut self, _f: &mut MapFrame) -> bool {
        false
    }

    /// `WORLD_MAN::SetEventData`: a story dungeon's rooms and positions.
    fn event_data(&self, _area: i32) -> Option<crate::dungeon::EventData> {
        None
    }
}

impl dyn AreaMap {
    /// The area as its own class, for tests and probes to read.
    pub fn get<T: AreaMap>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }

    pub fn get_mut<T: AreaMap>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut()
    }
}
