//! A dungeon, area 2: `DUNGEON` (gcmn dungeon.cpp) as `WORLD_MAN::GO(2)`
//! (main 0x0019f8e0) builds it and `ccThFieldDisp` draws it with
//! `DUNGEON::Draw` (gcmn 0x005ce930), one room at a time: `SetRoom` builds
//! the room, its dressing, doors and hits; `GotoNextRoom` takes the party
//! through a door or stairs. The dungeon outlives the scene changes inside
//! it, and its rooms' `ccModelHit`s stay registered across them. The room's
//! collision is its Anime chunk's pieces with Hit chunks, then each door's
//! frame and leaf (docs/engine/dungeon.md, "Entering and walking a dungeon").
//! What each room stands beside (the water, sparks, room lights, pieces, a
//! lake's sky and fireflies, type 32's leaves) are the dungeon type's
//! systems ([`def::DungeonDef`]).

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::dungeon::{self, DOWN, Dummies, FogRow, NO_ROOM, RealMap, RoomSize, Rotation, Tables, UP, special};
use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

pub mod def;

use crate::area::{AreaCore, Exit, Scene, WorldMan};
use crate::chara::pose::Play;
use crate::draw::{self, Draw, OBJ_LAYER};
use crate::ee::{self, F, ONE, PI, V4};
use crate::field::ambient::Weather;
use crate::hit::{HitModel, Hits, UNIT};
use crate::sprite::EffSprite;
use crate::systems::{
    BuildCx, Carried, CarriedMut, DrawCx, Drawn, FieldCx, Joined, MapEvent, MapView, RoomAt, RoomPart, Statics,
    Systems, paint,
};
use crate::town::{Light, TownLights};
use def::{DungeonDef, RoomStep};

/// `WORLD_MAN` bounds in `GO(2)`: 0..60000 both ways.
pub const SIZE: F = 0x476a_6000;
pub const BOUNDS: [F; 4] = [0, 0, SIZE, SIZE];
/// One map cell, 750 units (`GotoNextRoom`, `Draw`).
const CELL: F = 0x443b_8000;
/// `GotoNextRoom`'s step through a door, 1,125 units.
const THROUGH: F = 0x448c_a000;
/// pi / 2 as the code writes it (0x3fc90fdb), and -pi / 2.
const HALF_PI: F = 0x3fc9_0fdb;
const MINUS_HALF_PI: F = 0xbfc9_0fdb;
/// `realmap[x][y]` is 80 x 80.
const MAP: i32 = dungeon::MAP as i32;
/// The name `MakeRoom` finds the player's arrival spot by
/// (`ccAnm::GetSubstAdrsF`, @3158).
const START_DUMMY: &str = "OBJ_0ppp";
/// Where an event room stands the party (@7704).
const USER_POINT: &str = "OBJ_user_point";

/// `ccSaveData`'s `areaBan` (+0x6548): 32 rooms (area, dungeon, floor,
/// block) as four signed bytes each, a free entry all -1.
pub type Bans = [[i32; 4]; 32];

/// No room banned (a new save's entries).
pub const NO_BANS: Bans = [[-1; 4]; 32];

/// The save's area bans.
pub fn bans_of(save: &SaveData) -> Bans {
    std::array::from_fn(|i| {
        let at = piney_data::save::offset::AREA_BAN + 4 * i;
        std::array::from_fn(|k| save.u8(at + k) as i8 as i32)
    })
}

/// `ccSaveData::CheckAreaBan(area, dungeon, floor, block)` (main
/// 0x00178400): one of the 32 `areaBan` entries (four signed bytes each at
/// +0x6548) is the room.
pub fn area_banned(save: &SaveData, room: [i32; 4]) -> bool {
    bans_of(save).contains(&room)
}

/// `ccSaveData::SetAreaBan(area, dungeon, floor, block)`: nothing if the
/// room is banned already, else the first free entry (all -1).
pub fn ban_area(save: &mut SaveData, room: [i32; 4]) {
    if area_banned(save, room) {
        return;
    }
    let at = |i: usize| piney_data::save::offset::AREA_BAN + 4 * i;
    if let Some(i) = (0..32).find(|&i| (0..4).all(|k| save.u8(at(i) + k) == 0xff)) {
        for (k, v) in room.iter().enumerate() {
            save.set_u8(at(i) + k, *v as u8);
        }
    }
}

/// One room slot of a floor as `MakeRoom` leaves it: `pos[f][i]`,
/// `rotate[f][i]`, the model `animIdx` names (or the Gott statue room's).
#[derive(Clone, Debug, Default)]
pub struct Slot {
    /// `pos[f][i]`: the room's centre, z 0.
    pub pos: [F; 2],
    /// `rotate[f][i]`: the `ROOM_INFO` row's turn in radians.
    pub rotate: F,
    /// The Anime chunk `SetRoom` plays: `roomobj[r]` of the row, or the
    /// statue room's (`symroom[type]`); None when the room has no model.
    pub model: Option<&'static str>,
    /// Built by `MakeRoom` (a slot the floor's rooms never reach is not).
    pub made: bool,
    /// `minimap[f][i].size`: the room's size (0-2), as the map scans it.
    pub size: u8,
    /// The room's `SetAllGim` rolls as `Generate` made them (each dummy's
    /// keep, in the searches' order).
    pub rolls: Option<Vec<bool>>,
    /// The model is in the event room's own scene file (`spccs`), not the
    /// dungeon's: a story row of type 25-35 ([`dungeon::special`]).
    pub special: bool,
}

/// `WORLD_MAN`'s dungeons (`dungeon[3]`, +0x438) and `lastRoom` (+0x100):
/// the dungeons made in the area by their number, kept across its scene
/// changes (a field type 4 area has two, the second below the first's
/// floor 0), and the room of the first its down stairs were taken from.
#[derive(Default)]
pub struct Dungeons {
    pub slots: [Option<Box<DungeonArea>>; 3],
    pub last_room: i32,
}

/// A floor: `realmap[f]`, its rooms, `UpRoom[f]`, `DownRoom[f]` and
/// `startpos[0..2][f]`.
#[derive(Clone, Debug, Default)]
pub struct FloorLayout {
    pub map: RealMap,
    pub rooms: Vec<Slot>,
    /// `UpRoom[f]` and `DownRoom[f]` (15: none).
    pub up: usize,
    pub down: usize,
    /// `startpos[0][f]` (arriving by the up stairs) and `startpos[1][f]`
    /// (coming back up to the down stairs): x, y, z and the heading in w.
    pub start: [V4; 2],
    /// The rooms in the order `MakeRoom` built them (`SetAllGim` fills
    /// `gimPos` in it).
    pub order: Vec<usize>,
}

/// One object an anm places: the controller's own object (an ExtObj copy
/// or the object itself), what it draws, its local and world matrices.
#[derive(Clone, Debug)]
pub struct Instance {
    pub controller: u32,
    pub target: u32,
    pub local: [V4; 4],
    pub world: [V4; 4],
}

/// A `ccAnm` as the dungeon holds one: the animation, its root matrix and
/// what its objects are posed at.
#[derive(Clone, Debug)]
pub struct Anm {
    pub play: Play,
    /// `ccCoord.matrix` of the anm itself.
    pub root: [V4; 4],
    pub objects: Vec<Instance>,
    /// Each object's world matrix as `ccAnm::SetHitMatrix` last placed its
    /// hits (a door opening in dungeon types 3, 7, 8 and 9 keeps them
    /// where `SetDoor` put them).
    pub hit_world: Vec<[V4; 4]>,
    /// The leaf's local matrix while `MoveDoor` lowers it (`CloseStart`),
    /// by the object's controller; None: the animation's.
    pub leaf: Option<(u32, [V4; 4])>,
    /// Played from the event room's scene file (`spccs`).
    pub special: bool,
}

impl Anm {
    fn new(play: Play, root: [V4; 4], objects: Vec<Instance>, special: bool) -> Anm {
        let hit_world = objects.iter().map(|o| o.world).collect();
        Anm { play, root, objects, hit_world, leaf: None, special }
    }
}

/// A scene file with its objects' models and Hit chunks, as an anm reads
/// them ([`RoomFile`], or the dungeon's own).
pub type Src<'a> = (&'a Rc<SceneFile>, &'a HashMap<u32, Vec<u32>>, &'a HashMap<u32, Vec<HitModel>>);

/// A scene file rooms are played from, with its objects' models and its
/// Hit chunks by model.
pub struct RoomFile {
    pub file: Rc<SceneFile>,
    obj_models: HashMap<u32, Vec<u32>>,
    hit_models: HashMap<u32, Vec<HitModel>>,
}

/// The dungeon's scene files as its rooms read them: its own
/// (`DungeonName[type]`, its name for the effects) with its objects' models
/// and its Hit chunks by model, the event room's (`spccs`), the Eff chunks'
/// pattern transparencies, water 0's model (`OBJ_f_8s40_`, drawn with the
/// picture behind it), the palette swaps every model of the file draws
/// with, and the texture's clut type.
pub struct Rooms {
    pub file: Rc<SceneFile>,
    pub name: &'static str,
    pub obj_models: HashMap<u32, Vec<u32>>,
    pub hit_models: HashMap<u32, Vec<HitModel>>,
    pub spccs: Option<RoomFile>,
    pub eff_pats: HashMap<u32, Vec<u16>>,
    pub water_model: Option<piney_data::model::Model>,
    pub clut_swaps: Vec<(u32, u32)>,
    pub tex_clut: dungeon::TexClut,
}

/// The water's own object, whose model water 0 draws with the picture
/// behind it (@5659).
const WATER_OBJ: &str = "OBJ_f_8s40_";

impl Rooms {
    /// The scene file an anm plays from (the event room's `spccs`, else
    /// the dungeon's), with its objects' models and Hit chunks.
    pub fn src(&self, special: bool) -> Src<'_> {
        match (special, &self.spccs) {
            (true, Some(r)) => (&r.file, &r.obj_models, &r.hit_models),
            _ => (&self.file, &self.obj_models, &self.hit_models),
        }
    }

    /// The name of an object in the anm's scene file.
    pub fn name_in(&self, special: bool, obj: u32) -> Option<&str> {
        self.src(special).0.ccs.object_name(obj)
    }

    /// The name of an object of the dungeon's file ("?" for none).
    pub fn name(&self, obj: u32) -> String {
        self.name_in(false, obj).unwrap_or("?").to_string()
    }

    /// `ccAnm::SetAnm(name)` from the dungeon's file or the event room's
    /// (`special`), `forwards` `_AnimateForward`s, and the objects posed
    /// under `root`.
    pub fn make_anm_in(&self, special: bool, name: &str, root: &[V4; 4], forwards: usize) -> Option<Anm> {
        let file = self.src(special).0.clone();
        let mut play = Play::new(&file, name)?;
        for _ in 0..forwards {
            play.forward(&file);
        }
        let objects = self.pose_with(special, &play, root, None);
        Some(Anm::new(play, *root, objects, special))
    }

    /// Every object controller of the anm's animation at its time, in the
    /// chunk's order, with its world matrix ([`pose_in`]).
    pub fn pose(&self, play: &Play, root: &[V4; 4]) -> Vec<Instance> {
        self.pose_with(false, play, root, None)
    }

    /// [`Rooms::pose`] with one controller's local matrix replaced (the
    /// door leaf `MoveDoor` lowers).
    pub fn pose_with(&self, special: bool, play: &Play, root: &[V4; 4], over: Option<(u32, [V4; 4])>) -> Vec<Instance> {
        pose_in(self.src(special).0, play, root, over)
    }

    /// The hit models an anm's objects carry (`ccAnm::HitEnable(1)`,
    /// `ccAnm::SetHitMatrix`), each at its object's world matrix when its
    /// hits were last placed and that matrix's inverse, `type` 1, but those
    /// `MoveDoor` took off; `tag` keeps each anm's apart.
    pub fn anm_hits(&self, anm: &Anm, tag: u32) -> Vec<HitModel> {
        (0..anm.objects.len()).flat_map(|k| self.obj_hits(anm, k, tag)).collect()
    }

    /// The hit models of object `k` of an anm (see [`Rooms::anm_hits`]).
    pub fn obj_hits(&self, anm: &Anm, k: usize, tag: u32) -> Vec<HitModel> {
        let mut out = Vec::new();
        let Some(o) = anm.objects.get(k) else { return out };
        let (_, obj_models, hit_models) = self.src(anm.special);
        let Some(models) = obj_models.get(&o.target) else { return out };
        let at = anm.hit_world.get(k).copied().unwrap_or(o.world);
        for m in models {
            let Some(hs) = hit_models.get(m) else { continue };
            for (j, h) in hs.iter().enumerate() {
                let mut h = h.clone();
                h.rm = at;
                h.im = invers(&at);
                h.kind = 1;
                h.id = (tag << 20) | ((k as u32) << 4) | j as u32;
                out.push(h);
            }
        }
        out
    }

    /// Node `k` of a clump's hit models at `m`, `type` 1.
    pub fn clump_hits(&self, nodes: &[u32], m: &[V4; 4], tag: u32, k: usize) -> Vec<HitModel> {
        let mut out = Vec::new();
        let Some(models) = nodes.get(k).and_then(|n| self.obj_models.get(n)) else { return out };
        for model in models {
            for (j, h) in self.hit_models.get(model).map(Vec::as_slice).unwrap_or(&[]).iter().enumerate() {
                let mut h = h.clone();
                h.rm = *m;
                h.im = invers(m);
                h.kind = 1;
                h.id = (tag << 20) | ((k as u32) << 4) | j as u32;
                out.push(h);
            }
        }
        out
    }

    /// `ccAnm::GetSubstAdrs(pattern)` on `room`: its objects whose name (an
    /// ExtObj copy's target's) is the pattern, or with a trailing `*`
    /// starts with it, in the anm's order.
    pub fn subst(&self, room: &Anm, pattern: &str) -> Vec<Instance> {
        let (want, prefix) = match pattern.strip_suffix('*') {
            Some(p) => (p, true),
            None => (pattern, false),
        };
        room.objects
            .iter()
            .filter(|o| {
                self.name_in(room.special, o.target)
                    .is_some_and(|n| if prefix { n.starts_with(want) } else { n == want })
            })
            .cloned()
            .collect()
    }

    /// The number of patterns of the dungeon file's Eff `name`.
    pub fn pat_num(&self, name: &str) -> u32 {
        self.file.ccs.find_object(name).and_then(|o| self.eff_pats.get(&o)).map_or(0, |p| p.len() as u32)
    }

    /// The nodes of the dungeon file's clump `name`, in its order.
    pub fn clump_nodes(&self, name: &str) -> Vec<u32> {
        self.file
            .ccs
            .find_object(name)
            .and_then(|c| self.file.scene.clumps.iter().find(|(o, _)| *o == c))
            .map(|(_, n)| n.clone())
            .unwrap_or_default()
    }

    /// `ccClump::Draw` of `nodes` at `m`: each node's models (those with
    /// colour or ST) at the matrix.
    pub fn clump_pieces(&self, nodes: &[u32], m: &[V4; 4]) -> Vec<(u32, Mat4)> {
        let world = draw::mat(m);
        let mut models: Vec<u32> = nodes.iter().filter_map(|n| self.obj_models.get(n)).flatten().copied().collect();
        models.dedup();
        models.retain(|model| self.file.models.get(model).is_some_and(|i| i.mtype & 0x600 != 0x600));
        models.into_iter().map(|model| (model, world)).collect()
    }
}

/// Each Eff chunk's (0x0e00) pattern transparencies by its `EFF_` object:
/// `patNum` (u16 at +14) patterns of 8 bytes from +36, the transparency
/// (4096 = 1) at +4 of each (`crates/piney-effect/src/eff.rs`).
fn eff_patterns(ccs: &Ccs) -> HashMap<u32, Vec<u16>> {
    let d: &[u8] = &ccs.data;
    let u16_at = |p: usize| u16::from_le_bytes([d[p], d[p + 1]]);
    let mut out = HashMap::new();
    for ch in ccs.walk().chunks {
        if ch.in_frames || ch.kind != 0x0e00 {
            continue;
        }
        let p = ch.payload();
        if p + 36 > d.len() {
            continue;
        }
        let n = usize::from(u16_at(p + 14));
        if p + 36 + 8 * n > d.len() {
            continue;
        }
        let object = u32::from_le_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]);
        out.insert(object, (0..n).map(|i| u16_at(p + 36 + 8 * i + 4)).collect());
    }
    out
}

/// `sceVu0RotMatrix(m, local, (0, 0, rotate))` then `sceVu0TransMatrix(m,
/// m, (x, y, 0))`: a dummy's own matrix turned and moved with the room.
pub fn placed(local: &[V4; 4], slot: &Slot, turn: bool) -> [V4; 4] {
    let mut m = if turn { piney_data::anim::rot_bits_of(*local, [0, 0, slot.rotate]) } else { *local };
    for (k, p) in [slot.pos[0], slot.pos[1], 0].into_iter().enumerate() {
        m[3][k] = ee::add(m[3][k], p);
    }
    m
}

impl RoomFile {
    fn new(volume: Volume, file: Rc<SceneFile>) -> Result<RoomFile> {
        // `ccStream::Decode_Model` (main 0x0014bce0) makes no model of a
        // chunk with no mmats (the data word stays 0), so its object has
        // none and its Hit chunk never joins the list (sd3's
        // `MDL_o_move_l0_` carries only a hit).
        let mut obj_models: HashMap<u32, Vec<u32>> = HashMap::new();
        for (&m, &o) in &file.scene.model_owner {
            if file.models.get(&m).is_some_and(|i| !i.mmats.is_empty()) {
                obj_models.entry(o).or_default().push(m);
            }
        }
        for v in obj_models.values_mut() {
            v.sort_unstable();
        }
        let mut hit_models: HashMap<u32, Vec<HitModel>> = HashMap::new();
        for h in HitModel::read(volume, &file.ccs)? {
            hit_models.entry(h.parent).or_default().push(h);
        }
        Ok(RoomFile { file, obj_models, hit_models })
    }
}

/// `DUNGEON`'s door words (`docs/engine/dungeon.md`, "Doors"):
/// `CloseStart` (+0x1c, `close_door`'s countdown), `lockNum` (+0x20, the
/// room's door dummies), `lockOff` (+0x24, the last opening step's
/// `_AnimateForward`), `doorAnm` (+0x28, `MoveDoor`'s
/// `ccCheckActiveObject()`), `doorFlag` (+0x2c, the doors stand open),
/// `stillOpenDoor` (+0x48, `open_door`'s hold).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DoorState {
    pub close_start: i32,
    pub lock_num: i32,
    pub lock_off: i32,
    pub door_anm: bool,
    pub door_flag: bool,
    pub still_open: bool,
}

/// The block `DUNGEON::SetDoor` (the tail at gcmn 0x005c8370) stands in
/// the doorway of the room beside a banned one: a `ccClump` of the
/// dungeon file's `CMP_o_block_m0_`, its hits enabled (`type` 1) and
/// every node's placed at `m` (`ccClump::SetHitMatrix(float (*)[4])`
/// 0x0013d240), drawn by `DUNGEON::Draw` after the room.
#[derive(Clone, Debug)]
pub struct Block {
    /// The clump's nodes, in its order.
    pub nodes: Vec<u32>,
    /// Its local matrix: the room's gate wall (`OBJ_w_0g10_*`, else door
    /// dummy) nearest the banned room's centre, its local matrix turned by
    /// the room's `rotate` and moved to the room's centre.
    pub m: [V4; 4],
}

/// The clump `SetDoor` makes the ban block of (`@7310`).
const BLOCK_CLUMP: &str = "CMP_o_block_m0_";
/// `SetDoor`'s starting distance for the four candidates (400,000).
const FAR: F = 0x47c3_5000;

/// How far a new door's animation is run: `SetDoor` and `OpenDoor` its
/// whole length (open), `CloseDoor` and a busy room's `SetDoor` one step
/// (shut).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Steps {
    Open,
    Shut,
}

/// The door leaf `MoveDoor` lowers and takes off the list
/// (`GetSubstAdrsF("OBJ_w_9e20_")`, @8658).
const LEAF: &str = "OBJ_w_9e20_";
/// How far the leaf sinks a frame while a door closes (6.4, 0x40cccccd).
const LEAF_DROP: F = 0x40cc_cccd;

/// A dungeon as `DUNGEON` holds it, the room the party is in built.
pub struct DungeonArea {
    /// The dungeon type's def.
    pub def: &'static DungeonDef,
    /// The scene file it loads (`DungeonName[type]`), the collision world
    /// (the room's and its doors' hit models, `game.area` 2), the light
    /// characters are drawn with (the dungeon's ambient and its distant
    /// light, a room's lights with them), `fieldrand` as `Generate` left it,
    /// then each `SetRoom`'s and `Draw`'s.
    pub core: AreaCore,
    /// Its files as the rooms read them.
    pub rooms: Rc<Rooms>,
    pub tables: &'static Tables,
    /// `DUNGEON.type` (0-9).
    pub dtype: u8,
    /// `DUNGEON.isEventArea`: the story area whose `EditDungeon` table laid
    /// the dungeon out, or 0 for a random one.
    pub event: i32,
    /// `DUNGEON+0x430`, the floors a story dungeon has (Infection's
    /// constant 10; from Mutation on 15 for event area 125, QUA gcmn
    /// 0x004d4400): `Generate` and `MakeFloor` go over that many.
    pub floor_count: usize,
    pub floors: Vec<FloorLayout>,
    /// `DUNGEON.level`: the floor the party is on.
    pub level: usize,
    /// The built room: (floor, block) and its anm; its doors.
    pub room_at: Option<(usize, usize)>,
    pub room: Option<Anm>,
    pub doors: Vec<Anm>,
    /// The door words `SetDoor`, `MoveDoor`, `OpenDoor`, `CloseDoor` and
    /// `CloseDoor2` keep.
    pub door: DoorState,
    /// What joined the `ccModelHit` list after the room's, in its order:
    /// the doors' objects, the ban block's nodes, the systems' pieces'.
    /// `ccAnm::HitEnable(1)` appends an object's hit that is not on it,
    /// `HitDisable` takes it off, so a leaf `MoveDoor` took off and put
    /// back comes last.
    joined: Vec<Joined>,
    /// The type's systems, and what the last `Draw` drew of them.
    systems: Systems,
    frame: Vec<Drawn>,
    /// `WORLD_MAN.position` (+0x20): where the leader arrives, heading in w.
    pub position: V4,
    /// `WORLD_MAN.fieldtype` (field type 4's lake dungeons lead into
    /// dungeon 1) and `eventAreaNumber` (the field the up stairs lead back
    /// to).
    pub field_type: u32,
    pub event_area: i32,
    /// `prevBlock`, `prevFloor`, `prevPos` as `WORLD_MAN::SetPrevRoom` left
    /// them (main 0x001a4070).
    pub prev: (i32, i32, [F; 2]),
    /// The fog and ambient row `SetRoom` sets, and `DUNGEON.fog` (also the
    /// frame's clear colour).
    pub fog: FogRow,
    /// `WORLD_MAN::SetDungeonTexClut`'s clutType and texType, and the
    /// `fogParam` table (its VA) and row the constructor picks.
    pub tex_clut: dungeon::TexClut,
    pub fog_table: (u32, usize),
    /// `WORLD_MAN.specialRoom` (+0x160): -1 (the constructor, and
    /// `GotoNextRoom` as it leaves a room), 0 once `GotoNextRoom` or
    /// `RoomSelect` has stood the party in an event room at its
    /// `OBJ_user_point`. The minimap is not drawn and the story area's
    /// sound bank is loaded while it is not -1.
    pub special_room: i32,
    /// The light group as the constructor makes it (the fog row's ambient,
    /// the grey distant light), which every ordinary `SetRoom` restores.
    base_lights: TownLights,
    /// `ccDrawEnv`'s fog as the last `SetRoom` set it: the fog row, or an
    /// event room's `SetFog(32767, 65536, 0, 100, 0)`.
    pub room_fog: FogRow,
    /// `game.dungeon` as the dungeon was made, and the save's area bans as
    /// `DUNGEON::GetBanRoom` reads them (the game reads the save each
    /// time; the port's callers copy them in: [`DungeonArea::new_banned`],
    /// `GotoNextRoom` from its save, the field world before the rest).
    pub dungeon: i32,
    pub bans: Bans,
    /// `DUNGEON` +0xd3880: the block `SetDoor` stands in the room beside
    /// a banned one ([`Block`]).
    pub block: Option<Block>,
    /// `DUNGEON`'s minimap (`crate::map::dungeon`), made by the runtime;
    /// it lives as long as the dungeon.
    pub map: Option<Box<crate::map::dungeon::DungeonMap>>,
    /// The story dungeon's `EditDungeon` entry (`DUNGEON` +0x3351c).
    pub edit: Option<&'static dungeon::EditDungeon>,
    /// `roomEnterFlag` (+0x4c): set by the constructor and `GotoNextRoom`,
    /// cleared by `Draw` when it runs `SetPathFindingMap` for the room.
    pub room_enter: bool,
    /// `WORLD_MAN.entryFlag[1 + dungeon]` (+0x58 + 4 dungeon): `EntryGimmick`
    /// (main 0x001a1fe0) placed this dungeon's boxes, portals and idols.
    /// It lives with the dungeon. A lake's stairs and `GoField` clear it as
    /// the party leaves, and every entry goes with that scene, so they are
    /// placed again on the way back.
    pub gimmicks_placed: bool,
    /// `g_entryList`: what the entry control kept when the party left a
    /// room of the dungeon, for the next room's `restoreEntry`.
    pub kept_entries: Option<piney_battle::entry::KeptEntries>,
    /// The kept objects' actors (their bodies and animation players), in
    /// the kept lists' order.
    pub kept_actors: Vec<Option<crate::combat::Actor>>,
}

/// `WORLD_MAN`'s calls on a dungeon (`DUNGEON`).
impl crate::area::AreaMap for DungeonArea {
    fn core(&self) -> &AreaCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut AreaCore {
        &mut self.core
    }

    fn start(&self, scene: &Scene) -> (V4, F, [V4; 3]) {
        let (pos, rot) = DungeonArea::start(self);
        (pos, rot, crate::area::map::party_starts(scene.area, pos, rot))
    }

    fn clear(&self) -> Option<[u8; 3]> {
        Some(DungeonArea::clear(self))
    }

    fn put(&self, out: &mut piney_desktop::SaveState) {
        DungeonArea::put(self, &mut out.scroll_statics);
    }

    /// `DUNGEON::Draw`: `MoveDoor` for the cell under the player (15 for
    /// none) with its doors' `ccSeOn3D`s, the room and `DrawEff`'s sprites,
    /// and `SetPathFindingMap` once the room entered is built.
    fn disp(&mut self, cx: &mut crate::area::Disp) {
        let player = cx.view.player;
        if cx.awake {
            let here = self.here(player);
            let block = here.map_or(i32::from(NO_ROOM), |h| h as i32);
            let clear_here = (cx.clear)(self.level as i32, block);
            // MoveDoor's SetDoor reads the save's area bans.
            self.bans = bans_of(cx.save);
            let sounds = self.move_door(here, cx.clear_all, clear_here);
            if let Some(se) = self.door_se() {
                cx.events.extend(sounds.into_iter().map(|(_, pos)| MapEvent::Se3d { n: se, pos }));
            }
        }
        let sprites = self.draw(cx.layers, cx.to_screen, cx.view, cx.awake);
        cx.sprites.extend(sprites);
        if cx.awake && self.room_enter && self.shown(player).1 {
            self.room_enter = false;
            cx.path_map = true;
        }
    }

    fn marker_file(&self) -> Option<&SceneFile> {
        Some(&self.rooms.file)
    }

    fn special_room(&self) -> i32 {
        self.special_room
    }

    fn enter(&mut self, cx: &mut crate::area::EnterCx) -> Option<Exit> {
        let exit = DungeonArea::enter(self, cx.player, cx.scene, cx.clear, cx.save, cx.last_room);
        self.settle(cx.cc);
        exit
    }

    /// From the second dungeon of a field type 4 area back to the first
    /// (0x0019e45c: its `entryFlag` cleared), at the room its stairs were
    /// taken from; else `ChangeArea(1, eventAreaNumber)`.
    fn go_field(&mut self, no_field: bool, scene: &Scene, last_room: i32) -> Option<Exit> {
        if !no_field {
            return Some(Exit::Area { area: crate::area::kind::FIELD, n: scene.field.max(0) });
        }
        if scene.dungeon != 1 {
            return None;
        }
        self.gimmicks_placed = false;
        Some(Exit::Scene { area: 2, town: -2, field: -2, dungeon: 0, floor: 0, block: last_room })
    }

    /// The room built afresh and `WORLD_MAN.position` set in it.
    fn room_select(&mut self, floor: usize, block: usize, cx: &mut crate::area::EnterCx) -> bool {
        let clear = (cx.clear)(floor as i32, block as i32);
        self.bans = bans_of(cx.save);
        DungeonArea::room_select(self, floor, block, clear);
        self.settle(cx.cc);
        true
    }

    /// `DUNGEON::ShowMap`: one room of the floor a call.
    fn show_map(&mut self, _field_model: i32, cx: &mut crate::area::EnterCx) -> bool {
        let done = crate::map::dungeon::show_map_step(self, cx.player, cx.clear);
        self.settle(cx.cc);
        done
    }

    fn open_door(&mut self, floor: usize, block: usize) {
        DungeonArea::open_door(self, floor, block);
    }

    fn close_door(&mut self, floor: usize, block: usize) {
        DungeonArea::close_door(self, floor, block);
    }

    fn close_door2(&mut self) {
        DungeonArea::close_door2(self);
    }

    fn map_2d(&self) -> Vec<u8> {
        DungeonArea::map_2d(self)
    }

    fn map_2d_info(&self, block: i32) -> Option<[i32; 3]> {
        Some(DungeonArea::map_2d_info(self, block))
    }

    /// `restoreEntry` (the list kept from the last room) and
    /// `WORLD_MAN::EntryGimmick`'s dungeon branch (once: `entryFlag`).
    fn entries(&mut self, scene: &Scene, wm: &WorldMan) -> crate::combat::AreaEntries {
        let gims = (!self.gimmicks_placed).then(|| self.dungeon_gims(wm.field_attr(), wm.time_sym()));
        self.gimmicks_placed = true;
        let dungeon = crate::combat::DungeonEntries {
            kept: self.kept_entries.take(),
            actors: std::mem::take(&mut self.kept_actors),
            gims,
            rng: *self.rng(),
            rows: self.def.breakables,
            breakables: self.breakables_here(scene.field, scene.block),
        };
        crate::combat::AreaEntries { dungeon: Some(dungeon), field: None }
    }

    fn keeps_entries(&self) -> bool {
        true
    }

    fn keep_entries(&mut self, kept: piney_battle::entry::KeptEntries, actors: Vec<Option<crate::combat::Actor>>) {
        self.kept_entries = Some(kept);
        self.kept_actors = actors;
    }

    /// `WORLD_MAN.dungeon[n]` lives on, with the area's others.
    fn into_kept(self: Box<Self>, dungeons: Dungeons) -> Option<crate::story_map::Kept> {
        let mut all = dungeons;
        let n = self.dungeon.clamp(0, 2) as usize;
        all.slots[n] = Some(self);
        Some(crate::story_map::Kept::Dungeon(Box::new(all)))
    }

    /// The `DUNGEON` constructor's map over the floors `Generate` laid
    /// out (once: the dungeon keeps it from room to room).
    fn setup_map(&mut self, archive: &Arc<Archive>, volume: Volume, scene: &Scene) -> Result<()> {
        if self.map.is_none() {
            let edit = if self.event != 0 { self.tables.edit(self.event, scene.dungeon.clamp(0, 2)) } else { None };
            let floors = crate::map::dungeon::floors_of(self, edit);
            let file = self.rooms.file.clone();
            let map = crate::map::dungeon::DungeonMap::new(archive, volume, &file, self.def.lake, floors)?;
            self.map = Some(Box::new(map));
        }
        Ok(())
    }

    fn minimap(&mut self, f: &mut crate::area::MapFrame) -> bool {
        crate::map::dungeon_minimap(self, f)
    }

    fn event_data(&self, area: i32) -> Option<EventData> {
        Some(set_event_data(self.tables, self.event_area, area, 0))
    }
}

/// `sceVu0InversMatrix` (main 0x001107b0): the transposed rotation and the
/// translation taken back through it, `-(r0 t.x + r1 t.y + r2 t.z)` as VU0
/// multiply-adds, w kept.
pub fn invers(m: &[V4; 4]) -> [V4; 4] {
    let t = m[3];
    let rows: [V4; 3] = std::array::from_fn(|i| [m[0][i], m[1][i], m[2][i], 0]);
    let mut out = [rows[0], rows[1], rows[2], [0, 0, 0, t[3]]];
    for k in 0..3 {
        let acc = ee::mul(rows[0][k], t[0]);
        let acc = ee::add(acc, ee::mul(rows[1][k], t[1]));
        let v = ee::add(acc, ee::mul(rows[2][k], t[2]));
        out[3][k] = ee::sub(0, v);
    }
    out
}

/// [`DungeonArea::pose`] over any scene file: every object controller of
/// `play`'s animation at its time, in the chunk's order, with its world
/// matrix (`ccCoord::_SetLWMatrix` up the parents, `sceVu0MulMatrix`'s
/// rounding, `root` at the top); `over` replaces one controller's local
/// matrix.
pub fn pose_in(file: &SceneFile, play: &Play, root: &[V4; 4], over: Option<(u32, [V4; 4])>) -> Vec<Instance> {
    let a = &file.anims[play.anim];
    let sc = &file.scene;
    let poses = a.poses_at(play.posed);
    let ctrls: Vec<(u32, u32, [V4; 4])> = a
        .tracks
        .iter()
        .zip(&poses)
        .map(|(t, p)| match over {
            Some((o, l)) if o == t.object => (t.object, t.target, l),
            _ => (t.object, t.target, local_bits(p)),
        })
        .collect();
    let locals: HashMap<u32, [V4; 4]> = ctrls.iter().map(|&(o, _, l)| (o, l)).collect();
    let mut cache: HashMap<u32, [V4; 4]> = HashMap::new();
    fn world(
        sc: &piney_data::scene::Scene,
        locals: &HashMap<u32, [V4; 4]>,
        root: &[V4; 4],
        cache: &mut HashMap<u32, [V4; 4]>,
        o: u32,
        depth: u32,
    ) -> [V4; 4] {
        if let Some(m) = cache.get(&o) {
            return *m;
        }
        let local = locals.get(&o).copied().unwrap_or(UNIT);
        let parent = sc.ext_parent.get(&o).or_else(|| sc.parent.get(&o)).copied().unwrap_or(0);
        let up = if parent != 0 && parent != o && depth < 64 {
            world(sc, locals, root, cache, parent, depth + 1)
        } else {
            *root
        };
        let m = piney_data::anim::vu_mul(&up, &local);
        cache.insert(o, m);
        m
    }
    ctrls
        .iter()
        .map(|&(o, t, local)| Instance {
            controller: o,
            target: t,
            local,
            world: world(sc, &locals, root, &mut cache, o, 0),
        })
        .collect()
}

/// A controller's local matrix as `ccAnm::SetAnmCtrlWork` (main 0x00150670)
/// builds it: the rotation times the scale (`sceVu0MulMatrix`), then the
/// position added (`sceVu0TransMatrix`).
pub fn local_bits(pose: &piney_data::anim::Pose) -> [V4; 4] {
    let r = pose.rot;
    let col = |v: Vec3| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits(), 0];
    let rot = [col(r.x_axis), col(r.y_axis), col(r.z_axis), [0, 0, 0, ONE]];
    let s = pose.scale;
    let scale = [[s.x.to_bits(), 0, 0, 0], [0, s.y.to_bits(), 0, 0], [0, 0, s.z.to_bits(), 0], [0, 0, 0, ONE]];
    let mut m = piney_data::anim::vu_mul(&rot, &scale);
    for (k, p) in [pose.pos.x, pose.pos.y, pose.pos.z].into_iter().enumerate() {
        m[3][k] = ee::add(m[3][k], p.to_bits());
    }
    m
}

/// `SetRoom`'s `SetAmbient(row / 255)` (0x005c347c), divided in EE single
/// precision.
pub fn ambient_bits(fog: &FogRow) -> [F; 3] {
    fog.ambient.map(|v| ee::div(v.to_bits(), 0x437f_0000))
}

/// `fptosi(v / 750)`: the map cell of a coordinate.
fn cell_of(v: F) -> i32 {
    ee::to_int(ee::div(v, CELL))
}

/// `realmap[f][x][y]` as the game indexes it, with no bounds check on
/// either index (a cell past a column's end is the next column's).
fn cell(map: &RealMap, x: i32, y: i32) -> dungeon::Cell {
    let i = x * MAP + y;
    usize::try_from(i).ok().and_then(|i| map.cells().get(i).copied()).unwrap_or(dungeon::Cell::EMPTY)
}

/// A `Rotation`'s bits as the tables hold them.
fn rotation_bits(r: Rotation) -> F {
    r.radians().to_bits()
}

/// `MakeRoom`'s heading for a player arriving by a room's stairs: the other
/// way round from the room's turn; anything else leaves `w` as it was.
fn facing(rotate: F, w: F) -> F {
    match rotate {
        0 => PI,
        PI => 0,
        HALF_PI => MINUS_HALF_PI,
        MINUS_HALF_PI => HALF_PI,
        _ => w,
    }
}

/// What `WORLD_MAN::SetEventData` (main 0x001a3c40) hands on from a story
/// dungeon's tables: the event manager's points and positions and
/// `WORLD_MAN`'s warp points, each in the tables' row order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventData {
    /// `ccEvent::SetEventPoint(floor, index, eventFlag)` for each
    /// `ROOMDATA` row whose `eventFlag` (+0x20) is not 0.
    pub points: Vec<[i32; 3]>,
    /// `ccEvent::SetEventPos(floor, index, kind, dirc, (x, y, 0))` for each
    /// `GIMMICKDATA` row of type 2 (floor, index, kind; the heading's and
    /// the position's bits).
    pub positions: Vec<([i32; 3], F, [F; 3])>,
    /// `WORLD_MAN.warpPoint[kind - 7]` (+0x170, 0x20 each) = (x, y, 0) and
    /// its room word (+0x10) = the row's index, for each `GIMMICKDATA` row
    /// of type 3 whose kind is 7-26: (slot, point, room).
    pub warps: Vec<(usize, [F; 3], i32)>,
}

/// `WORLD_MAN::SetEventData()` (main 0x001a3c40), from `ccSetupGameCtrl`
/// between `ccStartThEvent` and `ccEnableThEvent(0)`: in a dungeon of a story
/// area whose `EditDungeon` entry exists (the area's first, whichever dungeon
/// the party is in), the rooms' event numbers become event points, the
/// type-2 gimmick rows event positions (facing by `direc`, else the heading
/// the row before left, the first time `f20`), and the type-3 rows of kind
/// 7-26 warp points.
pub fn set_event_data(tables: &Tables, event_area: i32, area: i32, f20: F) -> EventData {
    let mut out = EventData::default();
    if event_area == 0 || area != 2 {
        return out;
    }
    let Some(edit) = tables.edit.iter().find(|e| e.event == event_area) else { return out };
    for r in edit.rooms.iter().filter(|r| r.event_flag != 0) {
        out.points.push([r.floor, r.index, r.event_flag]);
    }
    let mut dirc = f20;
    for g in edit.gimmicks {
        let pos = [ee::from_int(g.x), ee::from_int(g.y), 0];
        match g.gim_type {
            2 => {
                dirc = match g.direc {
                    0 => 0,
                    1 => PI,
                    2 => HALF_PI,
                    3 => MINUS_HALF_PI,
                    _ => dirc,
                };
                out.positions.push(([g.floor, g.index, g.kind], dirc, pos));
            }
            3 if (7..27).contains(&g.kind) => out.warps.push(((g.kind - 7) as usize, pos, g.index)),
            _ => {}
        }
    }
    out
}

impl DungeonArea {
    /// What outlives the dungeon, into its systems: `DrawWater`'s `u$8862`
    /// and a lake's `DrawBG` `v$8917`, statics of the executable's data, the
    /// same for every dungeon, which carry on across them all
    /// ([`crate::town::scroll_statics`]).
    pub fn take(&mut self, statics: &Statics) {
        self.systems.take(&Carried { statics, move_tex: None, field: None });
    }

    /// The statics written back.
    pub fn put(&self, statics: &mut Statics) {
        self.systems.put(&mut CarriedMut { statics, move_tex: None, field: None });
    }

    /// The systems, for tests and probes to read by kind.
    pub fn systems(&self) -> &Systems {
        &self.systems
    }

    /// `fieldrand` as the dungeon's draws leave it.
    pub fn rng(&mut self) -> &mut dungeon::Rng {
        self.core.field_rand.get_or_insert(dungeon::Rng::POWER_ON)
    }

    /// `GO(2)` for `scene` (`game.dungeon`, floor, block) of `world_man`'s
    /// area: `DUNGEON::DUNGEON(game.dungeon)`, `Generate` (random, or the
    /// story area's `EditDungeon`), `SetRoom(0, 0)`, `GetStartPosition`;
    /// no room banned.
    pub fn new(archive: &Arc<Archive>, world_man: &WorldMan, scene: &Scene) -> Result<DungeonArea> {
        Self::new_banned(archive, piney_data::volume::Volume::Inf, world_man, scene, NO_BANS)
    }

    /// [`DungeonArea::new`] on `volume`'s tables (its story dungeons, floor
    /// counts and exits differ), with the save's area bans (`SetRoom(0, 0)`'s
    /// `SetDoor` reads them).
    pub fn new_banned(
        archive: &Arc<Archive>,
        volume: piney_data::volume::Volume,
        world_man: &WorldMan,
        scene: &Scene,
        bans: Bans,
    ) -> Result<DungeonArea> {
        let tables = dungeon::tables_of(volume);
        let code = scene.dungeon.clamp(0, 2) as usize;
        // eventAreaNumber is game.field, as GO(1) set it; DUNGEON::DUNGEON
        // takes it as isEventArea but for dungeon 0 of an area with no
        // field (field type 4).
        let event_area = scene.field.max(0);
        let mut event = event_area;
        if world_man.no_field() && code == 0 {
            event = 0;
        }
        let edit = if event != 0 { tables.edit(event, code as i32) } else { None };
        // The count is set from GetEventAreaNumber before field type 4's
        // dungeon 0 clears isEventArea.
        let floor_count = tables.edit_floor_count(event_area) as usize;
        let types = tables.types.types(world_man.field_type, event_area, world_man.hack as i32, false);
        let dtype = types[code.min(1)];
        let def = DungeonDef::of(dtype).ok_or_else(|| Error::NotFound(format!("dungeon type {dtype}")))?;
        let server = scene.server.max(0) as u32;
        // WORLD_MAN::SetDungeonTexClut (main 0x0019f4b0): by server and
        // field type, but story area 120 keeps the defaults (2, 0).
        let tc = if event_area == 120 {
            dungeon::TexClut { clut_type: 2, tex_type: 0 }
        } else {
            tables.tex_clut(server, world_man.field_type)
        };
        let file = Rc::new(SceneFile::read(archive, tables.ccs_name(dtype, u32::from(tc.tex_type)))?);
        // MakeRoom(ROOMDATA *)'s GetCCSAdrs for an event room: the last row
        // of type 25-35 MakeFloor reaches (floor by floor, not the lake
        // types) leaves its file in spccs.
        let spccs_name = edit.filter(|_| !def.lake).and_then(|e| {
            (0..floor_count)
                .flat_map(|f| e.rooms.iter().filter(move |r| r.floor == f as i32))
                .filter_map(|r| special::event_room(r.room_type, event_area))
                .next_back()
        });
        let spccs = match spccs_name {
            Some(er) => Some(RoomFile::new(volume, Rc::new(SceneFile::read(archive, er.ccs)?))?),
            None => None,
        };
        let fog = *tables
            .fog_row(dtype, tc, world_man.weather)
            .ok_or_else(|| Error::NotFound(format!("dungeon type {dtype}: no fog row")))?;
        let (fog_table, fog_index) = tables.fog_table(dtype, tc, world_man.weather);
        // SetClutList's pairs; SetRoom's loop stops at the first name the
        // file lacks. A twin the file lacks (the game would set a null
        // palette) is left out.
        let clut_swaps =
            tables.clut_list(dtype, tc.clut_type, world_man.weather).map_or_else(Vec::new, |(names, sfx)| {
                let mut out = Vec::new();
                for n in names {
                    let Some(from) = file.ccs.find_object(n) else { break };
                    if let Some(to) = file.ccs.find_object(&format!("{n}{sfx}")) {
                        out.push((from, to));
                    }
                }
                out
            });

        let RoomFile { obj_models, hit_models, .. } = RoomFile::new(volume, file.clone())?;
        // What the systems read of the file: water 0's model, the glows'
        // and sparks' patterns.
        let water_model = file.ccs.find_object(WATER_OBJ).and_then(|o| {
            let target = file.scene.ext.get(&o).copied().unwrap_or(o);
            let id = file.obj_model.get(&target).copied()?;
            piney_data::model::models(&file.ccs).ok()?.into_iter().find(|m| m.object == id)
        });
        let eff_pats = eff_patterns(&file.ccs);
        let rooms = Rc::new(Rooms {
            file: file.clone(),
            name: tables.ccs_name(dtype, u32::from(tc.tex_type)),
            obj_models,
            hit_models,
            spccs,
            eff_pats,
            water_model,
            clut_swaps,
            tex_clut: tc,
        });

        let heights: Rc<dyn crate::hit::Heights> = Rc::new(Heights);
        let hits = Hits { volume, area: 2, bounds: Some(BOUNDS), heights: Some(heights), ..Hits::default() };
        let amb = ambient_bits(&fog).map(f32::from_bits);
        // The constructor's distant light (0x005b8c10): grey 0.7, turned
        // by (-0.8, 0, -0.6) (SetMatrix_RotZYX), in the light group.
        let rot = piney_data::anim::rot_bits([0xbf4c_cccd, 0, 0xbf19_999a]);
        let dir = draw::mat(&rot).transform_vector3(Vec3::new(0.0, 0.0, -1.0));
        let lights = TownLights {
            ambient: Vec3::from(amb),
            lights: vec![Light {
                kind: 1,
                pos: Vec3::ZERO,
                dir,
                colour: Vec3::splat(f32::from_bits(0x3f33_3333)),
                intensity: 1.0,
                far_start: 0.0,
                far_end: 0.0,
                radius: [0.0; 2],
                priority: -1,
            }],
            fog: Some(room_fog_params(&fog)),
        };
        // The type's systems, as the constructor makes them (no draws of
        // fieldrand): WORLD_MAN's weather row and hack flag the lakes' sky
        // and fireflies read.
        let weather = Weather { field_type: world_man.field_type, b: world_man.weather, hack: world_man.hack as i32 };
        let field_def =
            world_man.field().ok_or_else(|| Error::NotFound(format!("field type {}", world_man.field_type)))?;
        let mut cx = BuildCx {
            archive,
            tables: volume,
            volume,
            file: &rooms.file,
            stem: rooms.name,
            crisis: false,
            rng: dungeon::Rng::POWER_ON,
            map: None,
            cc: None,
            field: Some(FieldCx { def: field_def, weather, server: scene.server, field: None, centre: [0, 0, 0, ONE] }),
            made: Vec::new(),
            rooms: Some(&rooms),
        };
        let systems = Systems::build(def.systems, &mut cx)?;
        let core = AreaCore {
            file,
            hits,
            lights: lights.clone(),
            field_rand: Some(dungeon::Rng::new(world_man.dungeon_seed[code])),
            start: [0, 0, 0, ONE],
            written: Vec::new(),
        };
        let mut area = DungeonArea {
            def,
            core,
            rooms,
            base_lights: lights,
            room_fog: fog,
            special_room: -1,
            dungeon: scene.dungeon,
            bans,
            block: None,
            tables,
            dtype,
            event: if edit.is_some() { event } else { 0 },
            floor_count,
            floors: Vec::new(),
            level: 0,
            room_at: None,
            room: None,
            doors: Vec::new(),
            door: DoorState::default(),
            joined: Vec::new(),
            systems,
            frame: Vec::new(),
            position: [0, 0, 0, ONE],
            field_type: world_man.field_type,
            event_area,
            prev: (0, 0, [0, 0]),
            fog,
            tex_clut: tc,
            fog_table: (fog_table.va, fog_index),
            map: None,
            edit,
            room_enter: true,
            gimmicks_placed: false,
            kept_entries: None,
            kept_actors: Vec::new(),
        };
        area.floors = match edit {
            Some(e) => {
                let floors = area.story_floors(e);
                area.floors = floors;
                area.story_rolls(e);
                std::mem::take(&mut area.floors)
            }
            None => {
                let dummies = Dummies::read(&area.rooms.file.ccs, &tables.gim_patterns)?;
                let params = dungeon::Params {
                    seed: world_man.dungeon_seed[code],
                    dtype,
                    level_max: world_man.level_max,
                    room_max: world_man.room_max,
                    server,
                    // volumeNum: Infection's 1 to Quarantine's 4.
                    volume: volume.number() as u32,
                    word_a: world_man.words[0] as u32,
                    field_type: world_man.field_type,
                    code: code as u32,
                };
                let d = dungeon::generate(tables, &params, &dummies).map_err(|e| Error::Format(e.to_string()))?;
                area.core.field_rand = Some(d.rng);
                area.random_floors(&d)
            }
        };
        // Generate's last act, then GetStartPosition.
        area.set_room(0, 0);
        area.position = area.floors[0].start[0];
        Ok(area)
    }

    /// The area has no field (`WORLD_MAN`'s field type 4): the first
    /// dungeon, a lake's, stands for it.
    pub fn no_field(&self) -> bool {
        crate::field::def::FieldDef::of(self.field_type).is_some_and(|d| !d.field)
    }

    /// `GO(2)` again for a room or floor of the same dungeon: nothing is
    /// rebuilt (`GotoNextRoom` already built the room; `prevFlag`, which
    /// would rebuild `prevFloor`/`prevBlock`, is never set on this path).
    /// The old player's body leaves the character list with him.
    pub fn reenter(&mut self, _scene: &Scene) -> Result<()> {
        self.core.hits.chars.clear();
        Ok(())
    }

    /// `GO(2)`'s way back in a field type 4 area (main 0x001a0cac): from
    /// the second dungeon to the first at `lastRoom`, the room whose down
    /// stairs led on: `position = startpos[1][0]` (arriving at those
    /// stairs), `ClearRoom`, `SetRoom(0, lastRoom)`. (It also sets
    /// `dungeonback` and puts the seed back round the room; the room's doors
    /// are built open, where the game asks `ccCheckActiveObject`.)
    pub fn come_back(&mut self, last_room: i32) {
        let Ok(b) = usize::try_from(last_room) else { return };
        self.level = 0;
        if let Some(f) = self.floors.first() {
            self.position = f.start[1];
        }
        self.set_room(0, b);
    }

    /// Where the party's leader arrives (`WORLD_MAN.position` after `GO`,
    /// `SetCharPosition`'s area-2 case main 0x001a1c2c: x, y, z from it, the
    /// heading its w) and its heading.
    pub fn start(&self) -> (V4, F) {
        let p = self.position;
        ([p[0], p[1], p[2], ONE], p[3])
    }

    /// The room slot of floor `f`, room `i`.
    fn slot(&self, f: usize, i: usize) -> Option<&Slot> {
        self.floors.get(f).and_then(|fl| fl.rooms.get(i))
    }

    /// The Gott statue's room: the one built from the type's statue model
    /// (`symroom`), floor and room.
    pub fn statue_room(&self) -> Option<(usize, usize)> {
        let sym = self.tables.symroom.get(usize::from(self.dtype))?;
        (0..self.floors.len())
            .flat_map(|f| (0..15).map(move |i| (f, i)))
            .find(|&(f, i)| self.slot(f, i).is_some_and(|s| s.model == Some(*sym)))
    }

    /// The random dungeon's floors: `MakeRoom(int, FOOT *, ...)` (0x005ba1d0)
    /// as `piney_data::dungeon` generates them, and each floor's startpos
    /// from the stairs rooms' `OBJ_0ppp` dummies.
    fn random_floors(&self, d: &dungeon::Dungeon) -> Vec<FloorLayout> {
        let mut out = Vec::new();
        for (level, fl) in d.floors.iter().enumerate() {
            let mut layout = FloorLayout {
                map: fl.map.clone(),
                rooms: vec![Slot::default(); 15],
                up: fl.up,
                down: fl.down.unwrap_or(NO_ROOM as usize),
                start: [[0; 4]; 2],
                order: Vec::new(),
            };
            for r in &fl.rooms {
                let Some(m) = &r.model else { continue };
                let slot = Slot {
                    pos: [r.pos[0].to_bits(), r.pos[1].to_bits()],
                    rotate: rotation_bits(m.rotate),
                    model: Some(m.name),
                    made: true,
                    size: r.size.index(),
                    rolls: Some(m.dummies.iter().map(|d| d.keep).collect()),
                    special: false,
                };
                layout.order.push(r.index);
                self.start_of(&mut layout, &slot, r.exits.0, (level, r.index));
                if let Some(s) = layout.rooms.get_mut(r.index) {
                    *s = slot;
                }
            }
            out.push(layout);
        }
        out
    }

    /// The story dungeon's floors: `MakeFloor`'s story path (0x005c0148)
    /// and `MakeRoom(ROOMDATA *, ROOM_INFO *, int)` (0x005bcbb0) over the
    /// `EditDungeon` rows of each of its [`DungeonArea::floor_count`] floors.
    fn story_floors(&self, edit: &'static dungeon::EditDungeon) -> Vec<FloorLayout> {
        let tables = self.tables;
        let mut out = Vec::new();
        for level in 0..self.floor_count {
            let real = dungeon::real_map(edit, level);
            // MakeFloor: UpRoom 0, DownRoom the row with the down stairs.
            let mut layout = FloorLayout {
                map: real.map.clone(),
                rooms: vec![Slot::default(); 15],
                up: 0,
                down: NO_ROOM as usize,
                start: [[0; 4]; 2],
                order: Vec::new(),
            };
            // Generate makes the ten floors (a lake type: floor 0), and
            // MakeFloor's story path builds nothing for the lake types.
            if self.def.lake {
                out.push(layout);
                continue;
            }
            // The last floor has no down stairs to look for (count - 1,
            // QUA gcmn 0x004dc778).
            if level != self.floor_count - 1 {
                for rd in edit.rooms.iter().filter(|r| r.floor == level as i32) {
                    if rd.exits.has(DOWN) {
                        layout.down = rd.index as usize;
                    }
                }
            }
            // The "E" tables of the type's family: types t and t + 4 share
            // them (sroom_infoE0, E1, E2, E4).
            let family = self.def.family;
            for rd in edit.rooms.iter().filter(|r| r.floor == level as i32) {
                let set = match rd.size {
                    RoomSize::Small => 0,
                    RoomSize::Medium => 1,
                    RoomSize::Large => 2,
                };
                let table = tables.rooms[set][family];
                let i = rd.index as usize;
                if i >= 15 {
                    continue;
                }
                // A row of type 16 or more is a large room whatever its size.
                let cells = if rd.room_type >= 16 { 16 } else { rd.size.cells() };
                let half = cells / 2;
                let pos = [ee::mul(CELL, ee::from_int(rd.x + half)), ee::mul(CELL, ee::from_int(rd.y + half))];
                for row in table.rows.iter().filter(|row| row.exits.0 == rd.exits.0) {
                    // 25-35: the event room's anm from spccs.
                    let event_room =
                        special::event_room(rd.room_type, self.event_area).filter(|_| self.rooms.spccs.is_some());
                    let model = match rd.room_type {
                        t if t < 12 => row.models.get(if t < 6 { t as usize } else { 0 }).copied(),
                        12 | 13 => row.models.first().copied(),
                        14 => Some(tables.symroom[self.dtype as usize]),
                        25..=35 => event_room.map(|e| e.anm),
                        // 15, 27 and 36 on: a room with no model.
                        _ => None,
                    };
                    // 16-24 and 34: the centre stored, the room deleted,
                    // and MakeRoom returns.
                    if (16..=24).contains(&rd.room_type) || rd.room_type == 34 {
                        layout.rooms[i].pos = pos;
                        break;
                    }
                    let slot = Slot {
                        pos,
                        rotate: rotation_bits(row.rotate),
                        model,
                        made: true,
                        size: rd.size.index(),
                        rolls: None,
                        special: event_room.is_some(),
                    };
                    self.start_of(&mut layout, &slot, rd.exits.0, (level, i));
                    layout.rooms[i] = slot;
                    if !layout.order.contains(&i) {
                        layout.order.push(i);
                    }
                }
            }
            out.push(layout);
        }
        out
    }

    /// `MakeRoom`'s startpos for a room with stairs, room `at` (floor,
    /// room): the `OBJ_0ppp` dummy's world position under the room's
    /// matrix, the heading the other way from the room's turn. A lake's up
    /// stairs take the room's centre at height 0, heading 1.0, turned only
    /// in field type 4's first dungeon at floor 0, room 0 (0x005badf4);
    /// its down stairs are the dummy's, as in every type (0x005bafac).
    fn start_of(&self, layout: &mut FloorLayout, slot: &Slot, exits: u8, at: (usize, usize)) {
        for (bit, k) in [(UP, 0), (DOWN, 1)] {
            if exits & bit == 0 {
                continue;
            }
            if k == 0 && self.def.lake {
                let first = self.no_field() && self.dungeon == 0 && at == (0, 0);
                let w = if first { facing(slot.rotate, ONE) } else { ONE };
                layout.start[0] = [slot.pos[0], slot.pos[1], 0, w];
                continue;
            }
            let root = Self::room_root(slot);
            let Some(anm) = slot.model.and_then(|m| self.rooms.make_anm_in(slot.special, m, &root, 1)) else {
                continue;
            };
            let Some(o) = anm.objects.iter().find(|o| self.rooms.name_in(slot.special, o.target) == Some(START_DUMMY))
            else {
                continue;
            };
            let t = o.world[3];
            layout.start[k] = [t[0], t[1], t[2], facing(slot.rotate, t[3])];
        }
    }

    /// `SetMatrix_PosRotZYX((x, y, 0), (0, 0, rotate))`.
    fn room_root(slot: &Slot) -> [V4; 4] {
        crate::town::pos_rot_zyx([slot.pos[0], slot.pos[1], 0, ONE], [0, 0, slot.rotate])
    }

    /// New doors: the old ones' hits gone with them (the ban block's and
    /// the dressing's stay where they are), each new one's `HitEnable(1)`
    /// in turn.
    fn put_doors(&mut self, doors: Vec<Anm>) {
        self.doors = doors;
        self.joined.retain(|j| !matches!(j, Joined::Door(..)));
        for k in 0..self.doors.len() {
            self.door_hit_enable(k);
        }
    }

    /// `ccAnm::HitEnable(1)` on door `k`: each object's hit not on the
    /// list appended.
    fn door_hit_enable(&mut self, k: usize) {
        for o in 0..self.doors[k].objects.len() {
            if !self.joined.contains(&Joined::Door(k, o)) {
                self.joined.push(Joined::Door(k, o));
            }
        }
    }

    /// The story row `SetRoom`'s dispatch finds at (f, i) - the first of
    /// type 16 or more there - when it is an event room (25-35) this area
    /// builds, with the first row of any type there, whose map cell `SetRoom`
    /// places the room's light by.
    fn event_room_at(&self, f: usize, i: usize) -> Option<(special::EventRoom, &'static dungeon::RoomData)> {
        let e = self.edit?;
        let (f, i) = (f as i32, i as i32);
        let ty = e.rooms.iter().find(|r| r.room_type >= 16 && r.floor == f && r.index == i)?.room_type;
        let er = special::event_room(ty, self.event_area)?;
        let row = e.rooms.iter().find(|r| r.floor == f && r.index == i)?;
        Some((er, row))
    }

    /// An event room's light group (`SetRoom`'s cases at 0x005c2288 and on):
    /// its `LGT_` object stood at `750 (x + 4, y + 4, 0)` of the row's map
    /// cell and added (`AddGrp`), and the ambient the anm's (`GetAmbient`,
    /// `SetAmbient`); types 26 and 35 add no light and leave the ambient as
    /// the last room set it. The light is the Anime chunk's record at the
    /// frame with its place replaced by that one (for every Infection
    /// room a direct light, a white beam down -z of radius 500 or 850);
    /// that the light has no parent to move it is not checked.
    fn event_room_lights(&mut self, er: special::EventRoom, row: &dungeon::RoomData) {
        let Some(room) = &self.room else { return };
        let Some(spccs) = &self.rooms.spccs else { return };
        let Ok((ambient, lights)) = crate::town::anim_lights(&spccs.file, room.play.anim) else { return };
        let Some(name) = er.light else { return };
        let obj = spccs.file.ccs.find_object(name);
        let at = Vec3::new(750.0 * (row.x + 4) as f32, 750.0 * (row.y + 4) as f32, 0.0);
        if let Some(l) = lights.iter().find(|l| Some(l.object) == obj) {
            let mut light = l.at(room.play.time);
            // SetMatrix_PosRotZYX after the anm's steps: the place replaces
            // the record's; the rotation (stack left over) turns a beam
            // down -z about z only.
            if light.kind != 1 {
                light.pos = at;
            }
            self.core.lights.lights.push(light);
        }
        // GetAmbient: each byte of the chunk's packed ambient over 255 in
        // the EE's division.
        if let Some(a) = ambient {
            let c = |v: f32| f32::from_bits(ee::div(ee::from_int((v * 255.0).round() as i32), 0x437f_0000));
            self.core.lights.ambient = Vec3::new(c(a.x), c(a.y), c(a.z));
        }
    }

    /// `DUNGEON::SetRoom(f, i)` (0x005c1ca0) for a random or story room,
    /// with nothing of the entry control's in it (its doors open): see
    /// [`DungeonArea::set_room_with`].
    pub fn set_room(&mut self, f: usize, i: usize) {
        self.set_room_with(f, i, true);
    }

    /// `DUNGEON::SetRoom(f, i)`: the room built before deleted, then the room's
    /// anm, its doors (`SetDoor` 0x005c7c30; `clear` its `ccCheckActiveObject(f,
    /// i)`) and the hit list rebuilt. An ordinary room sets the fog row's fog
    /// and ambient again. An event room (a row of type 25-35 at (f, i)) is
    /// played from `spccs` lit, at `pos[f][i]` turned by the type's `rotate`,
    /// under `SetFog(32767, 65536, 0, 100, 0)`, with its own light and ambient
    /// ([`DungeonArea::event_room_lights`]).
    pub fn set_room_with(&mut self, f: usize, i: usize, clear: bool) {
        self.room = None;
        self.doors.clear();
        self.joined.clear();
        self.block = None;
        self.room_part(None, RoomPart::Clear, None, f, None);
        self.room_at = Some((f, i));
        self.door.still_open = false;
        self.door.close_start = 0;
        // DelGrp(DUNGEON+0x370): the last event room's light leaves.
        self.core.lights.lights.truncate(self.base_lights.lights.len());
        let Some(mut slot) = self.slot(f, i).cloned() else {
            self.rebuild_hits();
            return;
        };
        let mut event_room = None;
        if let Some((er, row)) = self.event_room_at(f, i) {
            slot.rotate = er.rotate;
            if let Some(s) = self.floors.get_mut(f).and_then(|fl| fl.rooms.get_mut(i)) {
                s.rotate = er.rotate;
            }
            let root = Self::room_root(&slot);
            self.room = if self.rooms.spccs.is_some() { self.rooms.make_anm_in(true, er.anm, &root, 2) } else { None };
            self.event_room_lights(er, row);
            event_room = self.event_room_type(f, i);
            let [near, far, _, max] = special::EVENT_ROOM_FOG.map(f32::from_bits);
            self.room_fog = FogRow { colour: [0.0; 3], near, far, max, ambient: self.fog.ambient };
        } else {
            self.core.lights = self.base_lights.clone();
            self.room_fog = self.fog;
            let root = Self::room_root(&slot);
            self.room = slot.model.and_then(|m| self.rooms.make_anm_in(slot.special, m, &root, 1));
        }
        self.core.lights.fog = Some(room_fog_params(&self.room_fog));
        // After the room's hits, the type's steps: an event room's own, then
        // with the room built SetWater, SetLight, SetObject, SetDoor,
        // SetAnmObject, a lake's fireflies.
        for &step in self.def.room {
            match step {
                RoomStep::Run(kind, part) if part == RoomPart::Leaves || self.room.is_some() => {
                    self.room_part(Some(kind), part, Some(&slot), f, event_room);
                }
                RoomStep::Doors if self.room.is_some() => self.set_door(&slot, clear),
                _ => {}
            }
        }
        self.rebuild_hits();
    }

    /// `kind`'s `part` of `SetRoom` (every system's with None) for room
    /// `slot` on floor `floor`, the room's event type `event_room`.
    fn room_part(
        &mut self,
        kind: Option<crate::systems::Kind>,
        part: RoomPart,
        slot: Option<&Slot>,
        floor: usize,
        event_room: Option<i32>,
    ) {
        let mut rng = self.core.field_rand.unwrap_or(dungeon::Rng::POWER_ON);
        let mut cx = crate::systems::RoomCx {
            rooms: &self.rooms,
            room: self.room.as_ref(),
            slot,
            floor,
            level: self.level,
            event_room,
            rng: &mut rng,
            lights: &mut self.core.lights,
            joined: &mut self.joined,
        };
        self.systems.room(kind, part, &mut cx);
        if self.core.field_rand.is_some() {
            self.core.field_rand = Some(rng);
        }
    }

    /// VU1's fog by depth as the last `SetRoom` set it (`SetFog(near, far,
    /// 0, max, colour)`).
    fn room_depth_fog(&self) -> piney_draw::DepthFog {
        room_fog_params(&self.room_fog).depth()
    }

    /// `WORLD_MAN::RoomSelect(f, i)`'s part in the dungeon (main 0x0019dca0):
    /// `DUNGEON::ClearRoom` and [`DungeonArea::set_room_with`], then
    /// `DUNGEON::RoomSelect` (gcmn 0x005c95c0): the map painted again and left
    /// closed, `level` f, and `position` 200 in front of the room's first gate
    /// wall or door (w 1.0), else the room's centre. In areas 71 and 77 a story
    /// room of type 30 or 31 stands the party at its `OBJ_user_point`. The
    /// caller asks for the scene change and sets the map status 3.
    pub fn room_select(&mut self, f: usize, i: usize, clear: bool) {
        self.set_room_with(f, i, clear);
        self.room_enter = true;
        if let Some(m) = self.map.as_mut() {
            m.map_hide = 2;
        }
        self.level = f;
        let Some(slot) = self.slot(f, i) else { return };
        let centre = [slot.pos[0], slot.pos[1], 0, ONE];
        let pos = slot.pos;
        // Areas 71 and 77: their event room stands the party at its
        // OBJ_user_point (specialRoom 0).
        if let Some(want) = special::room_select_type(self.event) {
            let ty = self
                .edit
                .and_then(|e| e.rooms.iter().find(|r| r.room_type >= 16 && r.floor == f as i32 && r.index == i as i32))
                .map_or(0, |r| r.room_type);
            if ty == want {
                self.special_room = 0;
                let anm = special::event_room(ty, self.event_area).map(|e| e.anm);
                self.position = anm.and_then(|a| self.user_point(a, pos)).unwrap_or(centre);
                return;
            }
        }
        let d = self.tables.doors;
        let found = self.room.as_ref().and_then(|room| {
            let by = |prefix: &str| {
                room.objects
                    .iter()
                    .find(|o| self.rooms.name_in(room.special, o.target).is_some_and(|n| n.starts_with(prefix)))
            };
            by(d.gate).or_else(|| by(d.dummy)).map(|o| o.world)
        });
        self.position = match found {
            Some(m) => ee::apply(&m, [0, 0xc348_0000, 0, ONE]),
            None => centre,
        };
    }

    /// `DUNGEON::DeleteRoom(f, i)` (0x005c1980) of the room built: its anm,
    /// doors, dressing (the room lights out of the group with the event
    /// room's) and hits gone (`DUNGEON::ShowMap` builds each room of the
    /// floor in turn and deletes it).
    pub fn delete_room(&mut self) {
        self.room = None;
        self.doors.clear();
        self.joined.clear();
        self.block = None;
        self.room_at = None;
        self.room_part(None, RoomPart::Clear, None, self.level, None);
        self.core.lights.lights.truncate(self.base_lights.lights.len());
        self.rebuild_hits();
    }

    /// The door anms for the built room's door dummies (`OBJ_0pae0_*`, at
    /// most four; `lockNum` their count), each at `T(pos) Rz(rotate)` times
    /// the dummy's own matrix (`sceVu0RotMatrix`, `sceVu0TransMatrix`),
    /// the type's door animation run its whole length or one step.
    fn make_doors(&mut self, slot: &Slot, steps: Steps) -> Vec<Anm> {
        let Some(room) = &self.room else { return Vec::new() };
        let dummy = self.tables.doors.dummy;
        let locals: Vec<[V4; 4]> = room
            .objects
            .iter()
            .filter(|o| self.rooms.name_in(room.special, o.target).is_some_and(|n| n.starts_with(dummy)))
            .map(|o| o.local)
            .collect();
        self.door.lock_num = locals.len() as i32;
        let anime = self.tables.doors.anime[self.dtype as usize];
        let mut doors = Vec::new();
        for local in locals.into_iter().take(4) {
            let mut m = piney_data::anim::rot_bits_of(local, [0, 0, slot.rotate]);
            for (k, p) in [slot.pos[0], slot.pos[1], 0].into_iter().enumerate() {
                m[3][k] = ee::add(m[3][k], p);
            }
            let Some(mut play) = Play::new(&self.rooms.file, anime) else { continue };
            let frames = match steps {
                Steps::Open => self.rooms.file.anims[play.anim].frames,
                Steps::Shut => 1,
            };
            for _ in 0..frames {
                play.forward(&self.rooms.file);
            }
            let objects = self.rooms.pose(&play, &m);
            doors.push(Anm::new(play, m, objects, false));
        }
        doors
    }

    /// `DUNGEON::SetDoor(f, i)` (0x005c7c30): `lockNum` .. `doorFlag` 0,
    /// then the doors; with the room `clear` (`ccCheckActiveObject(f, i)`)
    /// each is run its whole length (open) and `doorFlag` set, else one
    /// step (shut). Then the ban block ([`DungeonArea::set_block`]).
    fn set_door(&mut self, slot: &Slot, clear: bool) {
        self.door.lock_num = 0;
        self.door.lock_off = 0;
        self.door.door_anm = false;
        self.door.door_flag = false;
        let steps = if clear { Steps::Open } else { Steps::Shut };
        let doors = self.make_doors(slot, steps);
        self.put_doors(doors);
        if clear && !self.doors.is_empty() {
            self.door.door_flag = true;
        }
        self.set_block(slot);
    }

    /// `DUNGEON::GetBanRoom(info)` (gcmn 0x005b61e0): the first story row
    /// whose room `CheckAreaBan(game.field, game.dungeon, floor, index)`
    /// finds banned, as (floor, index, next): `next` the last of its
    /// `next0`-`next3` that is not 20 (none: the caller's stack word, here
    /// None). No row, or a random dungeon: None (floor -1).
    pub fn ban_room(&self) -> Option<(i32, i32, Option<i32>)> {
        let e = self.edit?;
        let r = e.rooms.iter().find(|r| self.bans.contains(&[self.event_area, self.dungeon, r.floor, r.index]))?;
        Some((r.floor, r.index, r.next.iter().rev().copied().find(|&n| n != 20)))
    }

    /// The marks `DUNGEON::GetNearDoorPosition` (gcmn 0x005c9ba0) chooses
    /// among in room (f, i), built as `SetRoom` builds it: the places (the
    /// world matrices' last rows) of its gate dummies (`OBJ_w_0g10_*`), or
    /// with none its door dummies (`OBJ_0pae0_*`). None when the room is
    /// not built.
    pub fn door_marks(&self, f: usize, i: usize) -> Option<Vec<V4>> {
        let mut slot = self.slot(f, i)?.clone();
        let room = if let Some((er, _)) = self.event_room_at(f, i) {
            slot.rotate = er.rotate;
            self.rooms.spccs.as_ref()?;
            self.rooms.make_anm_in(true, er.anm, &Self::room_root(&slot), 2)?
        } else {
            self.rooms.make_anm_in(slot.special, slot.model?, &Self::room_root(&slot), 1)?
        };
        let d = self.tables.doors;
        let named = |prefix: &str| -> Vec<V4> {
            room.objects
                .iter()
                .filter(|o| self.rooms.name_in(room.special, o.target).is_some_and(|n| n.starts_with(prefix)))
                .map(|o| o.world[3])
                .collect()
        };
        let found = named(d.gate);
        Some(if found.is_empty() { named(d.dummy) } else { found })
    }

    /// `DUNGEON::GetNearDoorPosition(out, in, f, i)`: the mark of
    /// [`Self::door_marks`] nearest `at`, within 12000, a later one taken
    /// only when nearer. None with no marks (the game then leaves `out` as
    /// `in`) or none within 12000 (the game then returns its stack's words).
    pub fn near_door(&self, f: usize, i: usize, at: V4) -> Option<V4> {
        let found = self.door_marks(f, i)?;
        let mut best = ee::from_int(12000);
        let mut pick = None;
        for p in found {
            let dist = piney_battle::enemy_ai::get_dist_on(self.core.hits.volume, at, p);
            if !ee::le(best, dist) {
                best = dist;
                pick = Some(p);
            }
        }
        pick
    }

    /// `SetDoor`'s tail (gcmn 0x005c8370): the old block deleted; then with
    /// `game.field` not 0, when [`DungeonArea::ban_room`]'s `next` is this room,
    /// a block of `CMP_o_block_m0_` at the room's gate wall (or door dummy)
    /// nearest the banned room's centre: the nearer of the first two candidates
    /// against the nearer of the next two, ties to the later, a missing one at
    /// 400,000. Its hits join the list's tail at that matrix.
    fn set_block(&mut self, slot: &Slot) {
        self.block = None;
        self.joined.retain(|j| !matches!(j, Joined::Block(_)));
        if self.event_area == 0 {
            return;
        }
        let Some((f, i)) = self.room_at else { return };
        let Some((bf, bi, Some(next))) = self.ban_room() else { return };
        if bf != f as i32 || next != i as i32 {
            return;
        }
        let Some(room) = &self.room else { return };
        let target = usize::try_from(bf)
            .ok()
            .zip(usize::try_from(bi).ok())
            .and_then(|(bf, bi)| self.slot(bf, bi))
            .map_or([0, 0], |s| s.pos);
        let target = [target[0], target[1], 0, ONE];
        let d = self.tables.doors;
        let named = |prefix: &str| -> Vec<[V4; 4]> {
            room.objects
                .iter()
                .filter(|o| self.rooms.name_in(room.special, o.target).is_some_and(|n| n.starts_with(prefix)))
                .map(|o| o.local)
                .collect()
        };
        let mut found = named(d.gate);
        if found.is_empty() {
            found = named(d.dummy);
        }
        let place = |local: [V4; 4]| {
            let mut m = piney_data::anim::rot_bits_of(local, [0, 0, slot.rotate]);
            for (k, p) in [slot.pos[0], slot.pos[1], 0].into_iter().enumerate() {
                m[3][k] = ee::add(m[3][k], p);
            }
            m
        };
        let places: Vec<[V4; 4]> = found.into_iter().map(place).collect();
        if places.is_empty() {
            return;
        }
        let volume = self.core.hits.volume;
        let dist: [F; 4] = std::array::from_fn(|k| {
            places.get(k).map_or(FAR, |m| piney_battle::enemy_ai::get_dist_on(volume, target, m[3]))
        });
        let lt = |a: usize, b: usize| ee::f(dist[a]) < ee::f(dist[b]);
        let a = if lt(0, 1) { 0 } else { 1 };
        let b = if lt(2, 3) { 2 } else { 3 };
        let pick = if lt(a, b) { a } else { b };
        let Some(&m) = places.get(pick) else { return };
        let Some(c) = self.rooms.file.ccs.find_object(BLOCK_CLUMP) else { return };
        let Some((_, nodes)) = self.rooms.file.scene.clumps.iter().find(|(o, _)| *o == c) else { return };
        let nodes = nodes.clone();
        for k in 0..nodes.len() {
            self.joined.push(Joined::Block(k));
        }
        self.block = Some(Block { nodes, m });
    }

    /// The hit models of node `k` of the ban block, at its matrix, `type` 1.
    fn block_hits(&self, k: usize) -> Vec<HitModel> {
        match &self.block {
            Some(b) => self.rooms.clump_hits(&b.nodes, &b.m, 0xff, k),
            None => Vec::new(),
        }
    }

    /// `DUNGEON::OpenDoor(f, b)` (gcmn 0x005c8820), the event instruction
    /// `open_door` on `game.floor`, `game.block`: the room's doors made
    /// again and run their whole length, `doorFlag` and `stillOpenDoor`
    /// set (`MoveDoor` leaves them alone). Nothing unless (f, b) is the
    /// room built.
    pub fn open_door(&mut self, f: usize, b: usize) {
        let Some(slot) = self.built_slot(f, b) else { return };
        self.door.lock_num = 0;
        self.door.lock_off = 0;
        self.door.door_anm = false;
        self.door.door_flag = false;
        let doors = self.make_doors(&slot, Steps::Open);
        self.put_doors(doors);
        if !self.doors.is_empty() {
            self.door.door_flag = true;
            self.door.still_open = true;
        }
        self.rebuild_hits();
    }

    /// `DUNGEON::CloseDoor(f, b)` (gcmn 0x005c8f50), from
    /// `ccEntryEventMng` for each event entry it makes in a dungeon: the
    /// room's doors made again, one step each (shut); `lockNum` ..
    /// `doorFlag` 0. Nothing unless (f, b) is the room built.
    pub fn close_door(&mut self, f: usize, b: usize) {
        let Some(slot) = self.built_slot(f, b) else { return };
        self.door.lock_num = 0;
        self.door.lock_off = 0;
        self.door.door_anm = false;
        self.door.door_flag = false;
        let doors = self.make_doors(&slot, Steps::Shut);
        self.put_doors(doors);
        self.rebuild_hits();
    }

    /// `DUNGEON::CloseDoor2(f, b)` (gcmn 0x005c8800), the event instruction
    /// `close_door`: `stillOpenDoor` 0 and `CloseStart` the first door's
    /// frame count, which `MoveDoor` counts down lowering the leaves. With
    /// no door the game reads through null; here nothing is counted.
    pub fn close_door2(&mut self) {
        self.door.still_open = false;
        if let Some(d) = self.doors.first() {
            self.door.close_start = self.rooms.file.anims[d.play.anim].frames as i32;
        }
    }

    /// The slot of room (f, b) when it is the one built.
    fn built_slot(&self, f: usize, b: usize) -> Option<Slot> {
        if self.room_at != Some((f, b)) || self.room.is_none() {
            return None;
        }
        self.slot(f, b).cloned()
    }

    /// The sound `MoveDoor` (gcmn 0x005cd3d0) plays as a door opens or
    /// shuts, by the dungeon's type, the same for both.
    pub fn door_se(&self) -> Option<i32> {
        Some(self.def.door_se)
    }

    /// `DUNGEON::MoveDoor(here)` (gcmn 0x005cd3d0), from `Draw` each frame for
    /// the room under the player: with the room clear (`ccCheckActiveObject()`)
    /// each door steps open and its hits are placed again (in types 3, 7, 8 and 9
    /// the leaf's hits come off once open); else, while `CloseStart` runs, the
    /// leaf sinks 6.4 a frame, and at `CloseStart` 1 the doors are made again.
    /// Returns the doors' sounds (0 opening, 1 closing, at the door;
    /// [`DungeonArea::door_se`]).
    pub fn move_door(&mut self, here: Option<usize>, clear_all: bool, clear_here: bool) -> Vec<(u8, V4)> {
        let mut sounds = Vec::new();
        self.door.door_anm = clear_all;
        let level = self.level;
        if !self.room_slot_taken(here) {
            return sounds;
        }
        // The cell's byte: 15 for no room, a slot no floor has.
        let here = here.unwrap_or(NO_ROOM as usize);
        let keeps = self.def.leaf_off;
        let file = self.rooms.file.clone();
        for k in 0..self.doors.len() {
            if self.door.still_open {
                continue;
            }
            if self.door.door_anm {
                if !self.door.door_flag {
                    sounds.push((0, self.doors[k].root[3]));
                }
                let d = &mut self.doors[k];
                let ended = d.play.forward(&file);
                d.leaf = None;
                self.door.lock_off = i32::from(ended);
                let objects = self.rooms.pose(&self.doors[k].play, &self.doors[k].root);
                self.doors[k].objects = objects;
                self.door_hit_enable(k);
                let d = &mut self.doors[k];
                if !keeps {
                    d.hit_world = d.objects.iter().map(|o| o.world).collect();
                }
                if self.door.lock_off != 0 && keeps {
                    let leaf = d.objects.iter().position(|o| file.ccs.object_name(o.controller) == Some(LEAF));
                    if let Some(o) = leaf {
                        self.joined.retain(|&h| h != Joined::Door(k, o));
                    }
                }
            } else {
                let frames = file.anims[self.doors[k].play.anim].frames as i32;
                if self.door.close_start == frames {
                    sounds.push((1, self.doors[k].root[3]));
                }
                if self.door.close_start != 0 {
                    let d = &self.doors[k];
                    let Some(o) = d.objects.iter().find(|o| file.ccs.object_name(o.controller) == Some(LEAF)) else {
                        continue;
                    };
                    let local = d.leaf.map_or(o.local, |l| l.1);
                    let mut t = local[3];
                    t[2] = ee::sub(t[2], LEAF_DROP);
                    let mut m = UNIT;
                    m[3] = [t[0], t[1], t[2], ONE];
                    let over = Some((o.controller, m));
                    let objects = self.rooms.pose_with(false, &d.play, &d.root, over);
                    let d = &mut self.doors[k];
                    d.leaf = over;
                    d.objects = objects;
                    d.hit_world = d.objects.iter().map(|o| o.world).collect();
                    self.door_hit_enable(k);
                }
            }
        }
        if self.door.door_anm {
            self.door.door_flag = true;
        }
        if self.door.close_start == 1
            && let Some(slot) = self.slot(level, here).cloned()
        {
            self.set_door(&slot, clear_here);
        }
        if self.door.close_start != 0 {
            self.door.close_start -= 1;
        }
        self.rebuild_hits();
        sounds
    }

    /// The `ccModelHit` list: the room's hit models in its anm's order, then
    /// the doors', the ban block's and the systems' pieces' as they joined it.
    fn rebuild_hits(&mut self) {
        let mut models = Vec::new();
        if let Some(r) = &self.room {
            models.extend(self.rooms.anm_hits(r, 1));
        }
        for &j in &self.joined {
            match j {
                Joined::Block(k) => models.extend(self.block_hits(k)),
                Joined::Door(k, o) => {
                    if let Some(d) = self.doors.get(k) {
                        models.extend(self.rooms.obj_hits(d, o, 2 + k as u32));
                    }
                }
                Joined::Piece(kind, n, o) => models.extend(self.systems.hits(kind, n, o)),
            }
        }
        self.core.hits.models = models;
    }

    /// The names of the registered hit models' objects, in list order (for
    /// the checks).
    pub fn hit_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        let names = |special: bool, o: &Instance| -> Vec<String> {
            let (_, obj_models, hit_models) = self.rooms.src(special);
            let Some(models) = obj_models.get(&o.target) else { return Vec::new() };
            let n: usize = models.iter().map(|m| hit_models.get(m).map_or(0, Vec::len)).sum();
            vec![self.rooms.name_in(special, o.controller).unwrap_or("?").to_string(); n]
        };
        if let Some(r) = &self.room {
            r.objects.iter().for_each(|o| out.extend(names(r.special, o)));
        }
        for &j in &self.joined {
            match j {
                Joined::Block(k) => {
                    let n = self.block_hits(k).len();
                    out.extend(std::iter::repeat_n(BLOCK_CLUMP.to_string(), n));
                }
                Joined::Door(k, o) => {
                    if let Some(o) = self.doors.get(k).and_then(|d| d.objects.get(o)) {
                        out.extend(names(false, o));
                    }
                }
                Joined::Piece(kind, n, o) => out.extend(self.systems.hit_names(kind, n, o)),
            }
        }
        out
    }

    /// `DUNGEON::GotoNextRoom(nxt, now)` (0x005c9e10) with `WORLD_MAN.position`
    /// as `nxt`: 15 for the up stairs, -1 for the down stairs, else the room
    /// behind the door. The story areas' event rooms take their own branches
    /// ([`special::event_branch`]): a banned room rebuilds the room left and
    /// answers -100; areas 108, 73, 47, 66, 46 and 27 answer -255 (a way out).
    /// The warps of a story room's side flags are not ported. `save` holds the
    /// area bans.
    pub fn goto_next_room(&mut self, now: V4, scene: &Scene, save: &mut SaveData) -> i32 {
        self.goto_next_room_with(now, scene, &|_, _| true, save)
    }

    /// [`DungeonArea::goto_next_room`] with `clear(f, i)` the entry
    /// control's `ccCheckActiveObject(f, i)` for the room it builds (its
    /// doors shut when an enemy or magic circle belongs to it).
    pub fn goto_next_room_with(
        &mut self,
        now: V4,
        scene: &Scene,
        clear: &dyn Fn(i32, i32) -> bool,
        save: &mut SaveData,
    ) -> i32 {
        self.bans = bans_of(save);
        let level = self.level;
        let (x, y) = (cell_of(now[0]), cell_of(now[1]));
        let Some(fl) = self.floors.get(level) else { return -100 };
        let c = cell(&fl.map, x, y);
        let (d, next, here) = (c.door, c.next as usize, c.here as usize);
        let (up, down) = (fl.up, fl.down);
        // WORLD_MAN::SetPrevRoom(pos[level][here]).
        let centre = self.slot(level, here).map_or([0, 0], |s| s.pos);
        self.prev = (scene.block, scene.floor, centre);
        // HitDisable, DeleteRoom(level, here); specialRoom -1.
        if self.room_at == Some((level, here)) {
            self.delete_room();
        }
        self.special_room = -1;
        // The stairs (0x005ca074, 0x005ca120): mapHideFlag 1, so the next
        // DrawMap paints the whole texture again from the new floor's
        // squares (the last floor's leave with it), and ccMenu's map
        // status 3.
        if next == NO_ROOM as usize
            && (up == here || down == here)
            && let Some(m) = self.map.as_mut()
        {
            m.map_hide = 1;
        }
        if next == NO_ROOM as usize && up == here {
            if level != 0 {
                self.position = self.floors[level - 1].start[1];
                let r = self.floors[level - 1].down;
                self.set_room_with(level - 1, r, clear(level as i32 - 1, r as i32));
            }
            self.room_enter = true;
            return 15;
        }
        if next == NO_ROOM as usize && down == here {
            if self.def.lake {
                return -1;
            }
            if let Some(n) = self.floors.get(level + 1) {
                self.position = n.start[0];
                let r = n.up;
                self.set_room_with(level + 1, r, clear(level as i32 + 1, r as i32));
            }
            self.room_enter = true;
            return -1;
        }
        // The story areas' event rooms (the dispatch at 0x005ca1bc).
        if let Some((types, ban, arrival)) = special::event_branch(self.event) {
            let (lv, nx) = (level as i32, next as i32);
            let ty = self
                .edit
                .and_then(|e| e.rooms.iter().find(|r| r.room_type >= 16 && r.floor == lv && r.index == nx))
                .map_or(0, |r| r.room_type);
            if (types.is_empty() && ty != 0) || types.contains(&ty) {
                let room = [ban, scene.dungeon, lv, nx];
                if area_banned(save, room) {
                    self.set_room_with(level, here, clear(lv, here as i32));
                    return -100;
                }
                let pos = self.slot(level, next).map_or([0, 0], |s| s.pos);
                match arrival {
                    special::Arrival::Leave => {
                        if self.event == 66 {
                            ban_area(save, [66, scene.dungeon, lv, nx]);
                        }
                        return -255;
                    }
                    special::Arrival::UserPoint(anm) => {
                        self.special_room = 0;
                        if let Some(p) = self.user_point(anm, pos) {
                            self.position = p;
                        }
                        self.set_room_with(level, next, clear(lv, nx));
                        self.room_enter = true;
                        return nx;
                    }
                    special::Arrival::Marker(_, marker) => {
                        if let Some(p) = self.marker_point(marker, pos) {
                            self.position = p;
                        }
                        self.set_room_with(level, next, clear(lv, nx));
                        self.room_enter = true;
                        return nx;
                    }
                    // A temporary anm made and deleted, then the doorway.
                    special::Arrival::Door(_) => {}
                }
            }
        }
        let mut nxt = now;
        match d {
            dungeon::SOUTH => {
                nxt[1] = ee::add(nxt[1], THROUGH);
                nxt[3] = PI;
            }
            dungeon::NORTH => {
                nxt[1] = ee::sub(nxt[1], THROUGH);
                nxt[3] = 0;
            }
            dungeon::EAST => {
                nxt[0] = ee::add(nxt[0], THROUGH);
                nxt[3] = HALF_PI;
            }
            dungeon::WEST => {
                nxt[0] = ee::sub(nxt[0], THROUGH);
                nxt[3] = MINUS_HALF_PI;
            }
            _ => {}
        }
        self.position = nxt;
        self.set_room_with(level, next, clear(level as i32, next as i32));
        self.room_enter = true;
        next as i32
    }

    /// An event room's arrival (`GotoNextRoom`'s area branches, and
    /// `DUNGEON::RoomSelect`'s): a temporary anm of `anm` from spccs whose
    /// matrix is `T(pos, 0)` (`sceVu0TransMatrix` on the unit), one
    /// `_AnimateForward`, and its `OBJ_user_point`'s world translation
    /// (`_SetLWMatrix`), w included.
    fn user_point(&self, anm: &str, pos: [F; 2]) -> Option<V4> {
        let mut root = UNIT;
        root[3] = [pos[0], pos[1], 0, ONE];
        let a = self.rooms.make_anm_in(true, anm, &root, 1)?;
        let o = a.objects.iter().find(|o| self.rooms.name_in(true, o.target) == Some(USER_POINT))?;
        Some(o.world[3])
    }

    /// Area 91's arrival: the spccs chunk `marker`'s decoded position (x,
    /// y, z, 1: `ccStream::Decode_DummyPos` 0x0014d4d0) plus `(pos, 0, 1)`
    /// (`sceVu0AddVector`), so w is 2.
    fn marker_point(&self, marker: &str, pos: [F; 2]) -> Option<V4> {
        let file = &self.rooms.spccs.as_ref()?.file;
        let d = file.scene.dummies.get(&file.ccs.find_object(marker)?)?;
        let m = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE];
        Some([ee::add(m[0], pos[0]), ee::add(m[1], pos[1]), ee::add(m[2], 0), ee::add(m[3], ONE)])
    }

    /// `WORLD_MAN::Enter(pos)` in the dungeon (main 0x0019e018): `GotoNextRoom`,
    /// then the change of scene its answer asks for - a room, the floor above or
    /// below, or from floor 0's up stairs the field (`ChangeArea(1,
    /// eventAreaNumber)`). -255 (an event area's way out) is `ChangeArea(1, n)`
    /// by `eventAreaNumber` ([`special::exit_field`]).
    pub fn enter(
        &mut self,
        pos: V4,
        scene: &Scene,
        clear: &dyn Fn(i32, i32) -> bool,
        save: &mut SaveData,
        last_room: &mut i32,
    ) -> Option<Exit> {
        let r = self.goto_next_room_with(pos, scene, clear, save);
        let keep = -2;
        match r {
            -100 => None,
            -255 => special::exit_field(self.event_area, self.tables.volume).map(|n| Exit::Area { area: 1, n }),
            15 => {
                if scene.floor != 0 {
                    self.level = self.level.saturating_sub(1);
                    let down = self.floors[self.level].down as i32;
                    return Some(Exit::Scene {
                        area: 2,
                        town: keep,
                        field: keep,
                        dungeon: keep,
                        floor: self.level as i32,
                        block: down,
                    });
                }
                if !self.no_field() {
                    return Some(Exit::Area { area: 1, n: self.event_area.max(0) });
                }
                // No field: up to the first dungeon's room left for this one
                // (0x0019e0f8), this one's entryFlag cleared: its boxes,
                // portals and idols are placed again on the way back.
                self.gimmicks_placed = false;
                Some(Exit::Scene { area: 2, town: keep, field: keep, dungeon: 0, floor: 0, block: *last_room })
            }
            -1 => {
                // No field: the first dungeon's stairs down lead into the
                // second, and lastRoom keeps the room (0x0019e238); the
                // first's entryFlag cleared (0x0019e240).
                if self.no_field() && scene.dungeon == 0 {
                    self.gimmicks_placed = false;
                    *last_room = scene.block;
                    return Some(Exit::Scene { area: 2, town: keep, field: keep, dungeon: 1, floor: 0, block: 0 });
                }
                self.level += 1;
                Some(Exit::Scene {
                    area: 2,
                    town: keep,
                    field: keep,
                    dungeon: keep,
                    floor: self.level as i32,
                    block: 0,
                })
            }
            b => Some(Exit::Scene { area: keep, town: keep, field: keep, dungeon: keep, floor: keep, block: b }),
        }
    }

    /// `WORLD_MAN::Get2DMapInfo(info)` (main 0x001a22f0) in a dungeon:
    /// `DUNGEON::GetRoom2DPos(info)` (gcmn 0x005cf080) of room `block` of
    /// the current floor (`game.block`): the window's size, 80 for a story
    /// room of type 16 or more on this floor and block (the `EditDungeon`
    /// rows), else 10, 20 or 40 by the room's minimap size (0-2); its
    /// corner the room's centre / 300 (`WORLD_MAN::Get2DPos`, `fptosi`)
    /// less half the size.
    pub fn map_2d_info(&self, block: i32) -> [i32; 3] {
        let level = self.level;
        let special = self
            .edit
            .is_some_and(|e| e.rooms.iter().any(|r| r.room_type >= 16 && r.floor == level as i32 && r.index == block));
        let slot = usize::try_from(block).ok().and_then(|b| self.slot(level, b));
        let size = if special {
            80
        } else {
            match slot.map_or(0, |s| s.size) {
                0 => 10,
                1 => 20,
                _ => 40,
            }
        };
        let pos = slot.map_or([0, 0], |s| s.pos);
        let cell = |v: F| ee::to_int(ee::div(v, 0x4396_0000));
        [cell(pos[0]) - size / 2, cell(pos[1]) - size / 2, size]
    }

    /// `WORLD_MAN::Get2DMapPtr()` (main 0x001a22b0): the current floor's
    /// 2D map, 256 x 256 bytes `[x][y]`, as `DUNGEON::MakeMiniMap` has
    /// filled it (its first 200 x 200; empty without the map).
    pub fn map_2d(&self) -> Vec<u8> {
        let mut out = vec![0; 0x1_0000];
        let Some(m) = &self.map else { return out };
        let side = 200;
        for x in 0..side {
            for y in 0..side {
                out[x * 256 + y] = m.square(self.level, x, y);
            }
        }
        out
    }

    /// The room `DUNGEON::Draw` draws: `realmap[level]` at the player's
    /// cell (its `here` byte, `lbu $s3, 0x432(...)` at 0x005ceaec; a door
    /// cell is its own room's).
    pub fn here(&self, player: V4) -> Option<usize> {
        let fl = self.floors.get(self.level)?;
        let c = cell(&fl.map, cell_of(player[0]), cell_of(player[1]));
        (c.here != NO_ROOM).then_some(c.here as usize)
    }

    /// What `DUNGEON::Draw` draws of the room for a player at `player`:
    /// the cell's room and, when `room[level][here]` is built, the room
    /// and its doors (`MoveDoor` walks the doors only then). There is no
    /// fallback: once `GotoNextRoom` has deleted the room the player
    /// stands in and built the one behind the door, nothing of either is
    /// drawn until he crosses onto the new room's cells (the scene's fade
    /// out runs meanwhile). Returns (here, the room drawn, doors drawn).
    pub fn shown(&self, player: V4) -> (Option<usize>, bool, usize) {
        let here = self.here(player);
        let taken = self.room_slot_taken(here);
        (here, taken && here.is_some(), if taken { self.doors.len() } else { 0 })
    }

    /// Whether `room[level][here]` is non-null, as `MoveDoor` and `Draw` test
    /// it (QUA gcmn 0x004e8258, 0x004e9a94). A cell of no room has `here`
    /// 15, one past the floor's row: `room[level + 1][0]`, never built,
    /// except past the array's last row (Infection's tenth, the later
    /// volumes' fifteenth: B15 of area 125 alone). There Infection reads
    /// `rotate[0][0]` (+0x2f488) and the later volumes the animIdx pointer
    /// (+0x7bc), never null, so the doors move and are drawn (worklog 557).
    fn room_slot_taken(&self, here: Option<usize>) -> bool {
        // `room[10][15]` at +0x2f230 on Infection, `room[15][15]` at +0x438
        // from Mutation on.
        let rows = if self.tables.volume == 1 { 10 } else { 15 };
        match here {
            Some(h) => self.room_at == Some((self.level, h)) && self.room.is_some(),
            None if self.level + 1 < rows => false,
            None if self.tables.volume == 1 => {
                self.floors.first().and_then(|f| f.rooms.first()).is_some_and(|r| r.rotate != 0)
            }
            None => true,
        }
    }

    /// `DUNGEON.fog` as the frame's clear colour (`ccSys+0x18`), RGB.
    pub fn clear(&self) -> [u8; 3] {
        self.fog.colour_bytes()
    }

    /// `DUNGEON::Draw` for this frame: the type's systems in its order
    /// (asleep, as they stand), then the room under the player if built
    /// ([`DungeonArea::shown`]): its doors (`MoveDoor`), the room's pieces on
    /// `objLayer` fogged by depth, and the ban block. Returns the `ccEff`s
    /// `DrawEff` draws.
    pub fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, view: &MapView, awake: bool) -> Vec<EffSprite> {
        let player = view.player;
        let here = self.here(player);
        self.run(if awake { self.def.order } else { self.def.asleep }, view, here);
        let mut sprites = Vec::new();
        let rows = HashMap::new();
        for r in self.requests(&rows) {
            paint(layers, to_screen, &r, &mut sprites);
        }
        let (_, built, doors) = self.shown(player);
        // Past the array's last row the game hands `ccAnm::Draw` the word
        // after it (a ROOM_INFO row read as a `ccAnime`): no room is drawn.
        if !built && doors == 0 {
            return sprites;
        }
        // VU1 fogs each vertex by its depth: `SetFog(near, far, 0, max,
        // colour)` of the room's row.
        let depth = self.room_depth_fog();
        let rows = HashMap::new();
        let anms: Vec<&Anm> = self.doors.iter().chain(self.room.as_ref().filter(|_| built)).collect();
        for anm in anms {
            let (file, obj_models, _) = self.rooms.src(anm.special);
            for o in &anm.objects {
                let Some(models) = obj_models.get(&o.target) else { continue };
                let world = draw::mat(&o.world);
                for &model in models {
                    if file.models.get(&model).is_none_or(|i| i.mtype & 0x600 == 0x600) {
                        continue;
                    }
                    // An event room's anm is lit (`SetLightEnv(1)`): the
                    // room's light group reaches its lit models.
                    let lights = anm.special.then(|| crate::chara::kite::light_matrix(&self.core.lights, world, false));
                    let d = Draw {
                        file,
                        model,
                        world,
                        alpha: 1.0,
                        rows: &rows,
                        lights,
                        nodes: &[],
                        morph: Vec::new(),
                        clut_swaps: if anm.special { Vec::new() } else { self.rooms.clut_swaps.clone() },
                    };
                    draw::model_edited(layers, OBJ_LAYER, to_screen, &d, None, draw::Fogging::Depth(depth));
                }
            }
        }
        // The ban block (`ccClump::Draw`): its nodes' models at its matrix.
        if let Some(b) = &self.block {
            for (model, world) in self.rooms.clump_pieces(&b.nodes, &b.m) {
                let d = Draw {
                    file: &self.rooms.file,
                    model,
                    world,
                    alpha: 1.0,
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: self.rooms.clut_swaps.clone(),
                };
                draw::model_edited(layers, OBJ_LAYER, to_screen, &d, None, draw::Fogging::Depth(depth));
            }
        }
        sprites
    }

    /// `steps` of the type's systems for a frame seen from `view`, `here`
    /// the room under the player ([`DungeonArea::draw`] runs the def's).
    pub fn run(&mut self, steps: &'static [crate::systems::Step], view: &MapView, here: Option<usize>) {
        let room = here.and_then(|h| self.slot(self.level, h)).map(|s| RoomAt { pos: s.pos, size: s.size });
        let v = MapView { room, ..view.clone() };
        self.frame.clear();
        let mut events = Vec::new();
        self.systems.run(steps, &mut self.core, &v, &mut self.frame, &mut events);
    }

    /// The last frame's pieces as generic draws, fogged by the room's depth
    /// fog, with `rows`.
    pub fn requests<'a>(&'a self, rows: &'a HashMap<u32, [u8; 2]>) -> Vec<crate::systems::Request<'a>> {
        let cx = DrawCx { core: &self.core, fog: draw::Fogging::Depth(self.room_depth_fog()), rows };
        self.systems.requests(&self.frame, &cx)
    }

    /// The game's `ccRand` draws the systems owe (type 32's leaves'
    /// tilts), drawn after each call that can `SetRoom`.
    pub fn settle(&mut self, cc: &mut dyn piney_battle::rand::Rng) {
        self.systems.settle(cc);
    }

    /// The `EditDungeon` row type of an event room at (f, i).
    fn event_room_type(&self, f: usize, i: usize) -> Option<i32> {
        let e = self.edit?;
        e.rooms.iter().find(|r| r.room_type >= 16 && r.floor == f as i32 && r.index == i as i32).map(|r| r.room_type)
    }
}

/// `DUNGEON::GetHeight` (gcmn 0x005cf030): 0 everywhere; what
/// `WORLD_MAN::GetHeight` answers in a dungeon.
#[derive(Debug)]
struct Heights;

impl crate::hit::Heights for Heights {
    fn height(&self, _x: F, _y: F) -> F {
        0
    }
}

// the gimmicks' slots ------------------------------------------------------------------

/// `SetAllGim`'s searches in order, each with the `gimPos` kind it gives
/// (`OBJ_0ppi*` a box 0, `OBJ_0ppm*` a portal 1, `OBJ_0ppg` and the
/// fountain the statue 2, `OBJ_0ps0*`-`OBJ_0ps3*` the special objects 3-6).
const GIM_KINDS: [i8; 8] = [0, 1, 2, 2, 3, 4, 5, 6];

/// A `gimPos` slot's heading as `SetAllGim` turns it, from the dummy's
/// world matrix `w`. `p` is the point 150 down its y axis:
/// - a box: `atan2` in doubles of (x, -y) of `p` less the dummy's place
///   (0x005bb5c0);
/// - a portal: `atan2f(p.x, p.y)`, `p` itself (0x005bb8a4, its first
///   heading overwritten);
/// - the rest: `atan2f` of the x axis's y and x (0x005bbb74).
fn gim_heading(kind: i8, w: &[V4; 4]) -> F {
    let p = ee::apply(w, [0, ee::k(-150.0), 0, ONE]);
    match kind {
        0 => {
            let (x, y) = (ee::sub(p[0], w[3][0]), ee::sub(p[1], w[3][1]));
            let (x, y) = (f64::from(f32::from_bits(x)), f64::from(f32::from_bits(ee::mul(0xbf80_0000, y))));
            (x.atan2(y) as f32).to_bits()
        }
        1 => piney_data::libm::atan2f(p[0], p[1]),
        _ => {
            let mut m = *w;
            m[3] = [0, 0, 0, m[3][3]];
            let a = ee::apply(&m, [ONE, 0, 0, 0]);
            piney_data::libm::atan2f(a[1], a[0])
        }
    }
}

impl DungeonArea {
    /// The dummies of a built room's model that `SetAllGim`'s searches find,
    /// in their order: the search's index and the dummy's world matrix.
    fn room_dummies(&self, slot: &Slot) -> Vec<(usize, [V4; 4])> {
        let Some(model) = slot.model else { return Vec::new() };
        let Some(anm) = self.rooms.make_anm_in(slot.special, model, &Self::room_root(slot), 1) else {
            return Vec::new();
        };
        let patterns = self.tables.gim_patterns;
        let mut out = Vec::new();
        for (k, pat) in patterns.iter().enumerate() {
            for o in &anm.objects {
                if self.rooms.name_in(slot.special, o.target).is_some_and(|n| pat.matches(n)) {
                    out.push((k, o.world));
                }
            }
        }
        out
    }

    /// `DUNGEON::Generate`'s draws for a story dungeon (gcmn 0x005c12a0): each
    /// floor's `fieldrand(5)` (ten floors, a lake one) before its
    /// `MakeFloor`, whose `MakeRoom(ROOMDATA)`s run `SetAllGim` (0x005bb340)
    /// in building order. A room holds no box or portal where a
    /// `GIMMICKDATA` row or an event is; else a box on `fieldrand(100) >=
    /// 20`, a portal always, a special object on `>= 31`.
    fn story_rolls(&mut self, edit: &'static dungeon::EditDungeon) {
        let levels = if self.def.lake { 1 } else { self.floor_count };
        for f in 0..levels.min(self.floors.len()) {
            self.rng().below(5);
            for i in self.floors[f].order.clone() {
                let kinds: Vec<usize> =
                    self.room_dummies(&self.floors[f].rooms[i]).into_iter().map(|(k, _)| k).collect();
                let rolls =
                    kinds.into_iter().map(|k| self.story_keep(edit, GIM_KINDS[k], f as i32, i as i32)).collect();
                self.floors[f].rooms[i].rolls = Some(rolls);
            }
        }
    }

    /// `DUNGEON::SetAllGim(room)` (gcmn 0x005bb340) over every room the dungeon
    /// built, in `MakeRoom`'s order: each dummy of the room's model that a search
    /// finds and its roll kept becomes a `gimPos` slot ([`gim_heading`]).
    pub fn gim_slots(&mut self) -> Vec<piney_battle::entry::GimSlot> {
        let mut out = Vec::new();
        for (f, fl) in self.floors.iter().enumerate() {
            for &i in &fl.order {
                let Some(slot) = fl.rooms.get(i) else { continue };
                let rolls = slot.rolls.as_deref().unwrap_or(&[]);
                for (n, (k, w)) in self.room_dummies(slot).into_iter().enumerate() {
                    if !rolls.get(n).copied().unwrap_or(false) {
                        continue;
                    }
                    let heading = gim_heading(GIM_KINDS[k], &w);
                    out.push(piney_battle::entry::GimSlot {
                        pos: w[3],
                        dirc: [0, 0, heading, 0],
                        floor: f as i8,
                        block: i as i8,
                        kind: GIM_KINDS[k],
                    });
                }
            }
        }
        out
    }

    /// A story room's `CheckEntryItemBox` / `CheckEntryCircle` (gcmn
    /// 0x005cf800, 0x005cf690) and the special objects' roll.
    fn story_keep(&mut self, e: &'static dungeon::EditDungeon, kind: i8, f: i32, b: i32) -> bool {
        let rows = |lo: i32| e.gimmicks.iter().any(|g| g.floor == f && g.index == b && (lo..=4).contains(&g.gim_type));
        let event = e.rooms.iter().any(|r| r.event_flag != 0 && r.floor == f && r.index == b);
        match kind {
            0 => !rows(0) && !event && self.rng().below(100) >= 20,
            1 => !rows(1) && !event,
            2 => true,
            _ => self.rng().below(100) >= 31,
        }
    }

    /// `DUNGEON::EntryBreakObject()` (gcmn 0x005bff10) for the room built at
    /// `level`, `block`: the breakables' dummies `EntryBreakObjectMain`
    /// (0x005bfc40) finds in the room's anm, in its call order (`OBJ_0pr2*`,
    /// then `OBJ_0pr4*` .. `OBJ_0pr7*`), with the family (2, 4-7) and the world
    /// position. None in the lake types, in `game.field` 14 (the tutorial's
    /// dungeon) or in a story room with an event.
    pub fn breakables_here(&self, field: i32, block: i32) -> Vec<(u8, V4)> {
        if self.def.lake || field == 14 {
            return Vec::new();
        }
        let level = self.level as i32;
        if self.edit.is_some_and(|e| e.rooms.iter().any(|r| r.event_flag != 0 && r.floor == level && r.index == block))
        {
            return Vec::new();
        }
        let Some(room) = &self.room else { return Vec::new() };
        let mut out = Vec::new();
        for family in [2u8, 4, 5, 6, 7] {
            let prefix = format!("OBJ_0pr{family}");
            for o in &room.objects {
                if self.rooms.name_in(room.special, o.controller).is_some_and(|n| n.starts_with(&prefix)) {
                    let t = o.world[3];
                    out.push((family, [t[0], t[1], t[2], ONE]));
                }
            }
        }
        out
    }

    /// What `WORLD_MAN::EntryGimmick`'s dungeon setters read of this
    /// dungeon: its type, the story rows, the slots of [`Self::gim_slots`],
    /// each room's turn, `lakeFlag` (the lake types), and `WORLD_MAN`'s
    /// `timeSym` ([`crate::area::WorldMan::time_sym`]), `GetFieldAttrb()`
    /// and `GetFood()` (by the dungeon's type: 34-37 for types 0-7 in fours,
    /// 33 for the lakes, else 23).
    pub fn dungeon_gims(&mut self, field_attr: i32, time_sym: bool) -> piney_battle::entry::DungeonGims {
        use piney_battle::entry::{DungeonGims, EditGim, EditRoom};
        let slots = self.gim_slots();
        let food = self.def.food;
        let edit = self.edit.map(|e| {
            e.gimmicks
                .iter()
                .map(|g| EditGim {
                    floor: g.floor,
                    block: g.index,
                    x: g.x,
                    y: g.y,
                    ty: g.gim_type,
                    kind: g.kind,
                    flag: g.flag,
                    direc: g.direc,
                })
                .collect()
        });
        let rooms = self.edit.map_or(Vec::new(), |e| {
            e.rooms
                .iter()
                .map(|r| EditRoom {
                    floor: r.floor,
                    block: r.index,
                    ty: r.room_type,
                    event: r.event_flag,
                    item: r.item,
                })
                .collect()
        });
        let rotate =
            self.floors.iter().map(|fl| std::array::from_fn(|i| fl.rooms.get(i).map_or(0, |s| s.rotate))).collect();
        // The boss rooms' warnings (type 3, kind 6): SetItemBox builds the
        // row's room and asks GetNearDoorPosition from the row's place.
        let warn_pos = self.edit.map_or(Vec::new(), |e| {
            e.gimmicks
                .iter()
                .map(|g| {
                    if g.gim_type != 3 || g.kind != 6 {
                        return None;
                    }
                    let at = [ee::from_int(g.x), ee::from_int(g.y), 0, 0];
                    self.near_door(usize::try_from(g.floor).ok()?, usize::try_from(g.index).ok()?, at)
                })
                .collect()
        });
        DungeonGims {
            dtype: i32::from(self.dtype),
            story: self.edit.is_some(),
            slots,
            edit,
            counter: 0,
            rooms,
            rotate,
            lake: self.def.lake,
            time_sym: i32::from(time_sym),
            field_attr,
            food,
            warn_pos,
            ban_room: self.ban_room().is_some(),
        }
    }
}

/// A room's fog as `SetRoom` sets it: `cc3d->SetFog(near, far, 0, max,
/// colour)`.
fn room_fog_params(f: &FogRow) -> crate::town::FogParams {
    crate::town::FogParams { near: f.near, far: f.far, near_rate: 0.0, far_rate: f.max, colour: f.colour_bytes() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_of_a_turned_translation() {
        let m = crate::town::pos_rot_zyx([ee::k(1500.0), ee::k(-750.0), ee::k(10.0), ONE], [0, 0, HALF_PI]);
        let i = invers(&m);
        let p = ee::apply(&i, [ee::k(1500.0), ee::k(-750.0), ee::k(10.0), ONE]);
        assert!(p[..3].iter().all(|&v| ee::f(v).abs() < 1e-3), "{:?}", p.map(ee::f));
        assert_eq!(i[3][3], ONE);
    }

    #[test]
    fn area14_story_dungeon() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let wm = WorldMan {
            field_seed: 1_420_855,
            dungeon_seed: [154_874_216, 3_996_388_677, 1_814_669_918],
            field_type: 10,
            weather: 0,
            ground: 1,
            object: 1,
            event: 14,
            protect: false,
            level_max: 4,
            room_max: 9,
            hack: 2,
            field_model: 0,
            // Not read here: the dungeon takes its type from the tables.
            dungeon_type: [0; 3],
            words: [0; 3],
            ..WorldMan::default()
        };
        let mut scene = Scene::init();
        scene.area = 2;
        scene.field = 14;
        scene.dungeon = 0;
        scene.floor = 0;
        scene.block = 0;
        scene.server = 0;
        let d = DungeonArea::new(&archive, &wm, &scene).unwrap();
        assert_eq!(d.event, 14);
        // startpos[0][0] as the game's MakeRoom leaves it (eemu).
        assert_eq!(d.start().0.map(ee::f), [15000.0, 21750.0, 0.0, 1.0]);
        assert_eq!(d.floors[0].down, 4);
        assert_eq!(d.core.hits.models.len(), 43);
    }

    /// Issue #78: from Mutation on, event area 125 (Hidden Darkside Holy
    /// Ground) has fifteen floors (`DUNGEON+0x430`, QUA gcmn 0x004d4400).
    /// B10 keeps its down stairs (`MakeFloor` looks on every floor but
    /// count - 1) and B11 to B15 are built; with Infection's ten, B10 had
    /// none and its stairs led nowhere.
    #[test]
    fn area_125_has_fifteen_floors_from_mutation_on() {
        let Some(archive) = crate::town::tests::archive_of("quarantine") else { return };
        let wm = WorldMan { event: 125, field_type: 10, level_max: 4, room_max: 9, ..WorldMan::default() };
        let mut scene = Scene::init();
        (scene.area, scene.field, scene.dungeon, scene.floor, scene.block, scene.server) = (2, 125, 0, 0, 0, 0);
        let volume = piney_data::volume::Volume::Qua;
        let d = DungeonArea::new_banned(&archive, volume, &wm, &scene, NO_BANS).unwrap();
        assert_eq!((d.event, d.floor_count, d.floors.len()), (125, 15, 15));
        assert_ne!(d.floors[9].down, NO_ROOM as usize, "B10's down stairs");
        assert_eq!(d.floors[14].down, NO_ROOM as usize, "B15 is the last");
        assert!(d.floors[10..].iter().all(|f| f.rooms.iter().any(|r| r.made)), "B11 to B15 built");
        // The map: a label and a smallmap for each floor (`levelstr` runs
        // to "B 15" from Mutation on).
        let edit = d.tables.edit(d.event, 0);
        let floors = crate::map::dungeon::floors_of(&d, edit);
        let map = crate::map::dungeon::DungeonMap::new(&archive, volume, &d.rooms.file, d.def.lake, floors).unwrap();
        assert_eq!(map.labels.len(), 15);
        assert_eq!(map.labels[14], b"B 15");
    }

    /// Issue #78's harness case: a cell of no room (`here` 15) reads
    /// `room[level][15]`, one past the floor's row. On B15 of area 125 that
    /// is past the array's last row, the animIdx pointer, so `MoveDoor` and
    /// `Draw` take it for a room and the built room's doors are drawn; on B10
    /// the slot is B11's first, never built while the party is on B10.
    #[test]
    fn past_the_last_row_a_roomless_cell_still_draws_the_doors() {
        let Some(archive) = crate::town::tests::archive_of("quarantine") else { return };
        let wm = WorldMan { event: 125, field_type: 10, level_max: 4, room_max: 9, ..WorldMan::default() };
        let mut scene = Scene::init();
        (scene.area, scene.field, scene.dungeon, scene.floor, scene.block, scene.server) = (2, 125, 0, 0, 0, 0);
        let volume = piney_data::volume::Volume::Qua;
        let mut d = DungeonArea::new_banned(&archive, volume, &wm, &scene, NO_BANS).unwrap();
        // A room with doors on floor `f`, and a cell of no room there.
        let on = |d: &mut DungeonArea, f: usize| {
            d.level = f;
            let rooms: Vec<usize> = (0..d.floors[f].rooms.len()).filter(|&i| d.floors[f].rooms[i].made).collect();
            let i = rooms.into_iter().find(|&i| {
                d.set_room(f, i);
                !d.doors.is_empty()
            })?;
            let (x, y) = (0..80)
                .flat_map(|x| (0..80).map(move |y| (x, y)))
                .find(|&(x, y)| cell(&d.floors[f].map, x, y).here == NO_ROOM)?;
            let at = |c: i32| (c as f32 * 750.0 + 375.0).to_bits();
            Some((i, [at(x), at(y), 0, ONE]))
        };
        let (_, p) = on(&mut d, 14).expect("a room with doors on B15");
        assert_eq!(d.shown(p), (None, false, d.doors.len()), "B15: the doors past the row");
        let (_, p) = on(&mut d, 9).expect("a room with doors on B10");
        assert_eq!(d.shown(p), (None, false, 0), "B10: B11's slot, empty");
    }

    /// A field type 4 area has no field. Its first dungeon's stairs down
    /// lead into the second (`WORLD_MAN::Enter` 0x0019e238), keeping the
    /// room they were taken from (`lastRoom`); the second's floor-0 stairs
    /// up lead back to that room, which `GO(2)` builds with the party at
    /// its stairs ([`DungeonArea::come_back`]).
    #[test]
    fn a_type_4_area_goes_down_into_its_second_dungeon_and_back() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let wm =
            WorldMan { field_type: 4, level_max: 2, room_max: 4, dungeon_seed: [1234, 5678, 9], ..WorldMan::default() };
        let stairs = |d: &DungeonArea, room: usize| {
            let fl = &d.floors[0];
            let (x, y) = (0..MAP)
                .flat_map(|x| (0..MAP).map(move |y| (x, y)))
                .find(|&(x, y)| {
                    let c = cell(&fl.map, x, y);
                    c.here as usize == room && c.next == NO_ROOM
                })
                .expect("floor 0's stairs");
            let at = |i: i32| ee::k((i as f32 + 0.5) * 750.0);
            [at(x), at(y), 0, ONE]
        };
        let mut save = SaveData::new();
        let mut scene = Scene::init();
        (scene.area, scene.field, scene.dungeon, scene.floor) = (2, 40, 0, 0);
        let mut first = DungeonArea::new(&archive, &wm, &scene).unwrap();
        let down = first.floors[0].down;
        scene.block = down as i32;
        let mut last_room = 0;
        first.gimmicks_placed = true;
        let exit = first.enter(stairs(&first, down), &scene, &|_, _| true, &mut save, &mut last_room);
        assert_eq!(exit, Some(Exit::Scene { area: 2, town: -2, field: -2, dungeon: 1, floor: 0, block: 0 }));
        assert_eq!(last_room, down as i32);
        assert!(!first.gimmicks_placed, "entryFlag[1] cleared going down (0x0019e240)");

        (scene.dungeon, scene.block) = (1, 0);
        let mut second = DungeonArea::new(&archive, &wm, &scene).unwrap();
        let up = second.floors[0].up;
        scene.block = up as i32;
        second.gimmicks_placed = true;
        let exit = second.enter(stairs(&second, up), &scene, &|_, _| true, &mut save, &mut last_room);
        assert_eq!(exit, Some(Exit::Scene { area: 2, town: -2, field: -2, dungeon: 0, floor: 0, block: last_room }));
        assert!(!second.gimmicks_placed, "entryFlag[2] cleared going up (0x0019e104)");

        first.come_back(last_room);
        assert_eq!((first.level, first.room_at), (0, Some((0, down))));
        assert_eq!(first.start().0[..3], first.floors[0].start[1][..3]);
        // The lake's stairs down are its room's OBJ_0ppp, in the room
        // (issue #51: they were left at 0, 0, 0).
        assert_eq!(first.here(first.start().0), Some(down), "back up outside the room");
    }

    /// Theta's dungeons (server 1) of field types 2-4, 7 and 9 take clutType
    /// 3: `SetClutList(DUNGEON, "c1")` pairs every name of the type's list
    /// with its "c1" twin, which the file holds, and the draws swap them.
    /// Server 0 swaps nothing; server 3's types 2-4 take "c2".
    #[test]
    fn theta_dungeons_swap_their_palettes() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let run = |server: i32, field_type: u32| {
            let wm = WorldMan { field_type, level_max: 2, room_max: 4, ..WorldMan::default() };
            let mut scene = Scene::init();
            scene.area = 2;
            scene.field = 40;
            scene.server = server;
            DungeonArea::new(&archive, &wm, &scene).unwrap()
        };
        let d = run(0, 3);
        assert_eq!(d.tex_clut.clut_type, 2);
        assert!(d.rooms.clut_swaps.is_empty());
        for (server, sfx) in [(1, "c1"), (3, "c2")] {
            let d = run(server, 3);
            assert!(!d.rooms.clut_swaps.is_empty(), "server {server}");
            for &(from, to) in &d.rooms.clut_swaps {
                let (a, b) = (d.rooms.file.ccs.object_name(from).unwrap(), d.rooms.file.ccs.object_name(to).unwrap());
                assert!(a.starts_with("CLT_") && b == format!("{a}{sfx}"), "{a} -> {b}");
            }
            eprintln!("server {server} type {}: {} palettes swapped", d.dtype, d.rooms.clut_swaps.len());
        }
    }

    /// A lake's statues and flowers under `GetBG` 1-3: `EntryObject`
    /// (gcmn 0x005c6e80) runs `ccClump::ChangeClut(+0x24c[k], +0x14c[k])`
    /// on each, the same `SetClutList` pairs `SetRoom` runs on the room.
    /// The swap writes the shared materials, and the port's file-wide
    /// `clut_swaps` draw every model of the file with them: each clump has
    /// a palette on the list. Under `GetBG` 0 there is none.
    #[test]
    fn a_lakes_statues_take_its_palettes() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let run = |weather: u32| {
            let wm = WorldMan { field_type: 4, weather, level_max: 2, room_max: 4, ..WorldMan::default() };
            let mut scene = Scene::init();
            scene.area = 2;
            scene.field = 40;
            DungeonArea::new(&archive, &wm, &scene).unwrap()
        };
        assert!(run(0).rooms.clut_swaps.is_empty(), "GetBG 0");
        for bg in 1..=3 {
            let d = run(bg);
            assert!(matches!(d.dtype, 8 | 9), "a lake: {}", d.dtype);
            let (textures, _) = piney_data::texture::read(&d.rooms.file.ccs).unwrap();
            let from: Vec<u32> = d.rooms.clut_swaps.iter().map(|s| s.0).collect();
            let names =
                crate::systems::pieces::LAKE_PIECES.iter().map(|p| p.1).chain(crate::systems::pieces::LAKE_PICKS);
            let mut seen = 0;
            for name in names {
                let Some(c) = d.rooms.file.ccs.find_object(name) else { continue };
                seen += 1;
                let nodes = d.rooms.file.scene.clumps.iter().find(|(k, _)| *k == c).map(|(_, n)| n.clone()).unwrap();
                let cluts: Vec<u32> = (nodes.iter())
                    .filter_map(|n| d.rooms.file.obj_model.get(n))
                    .filter_map(|m| d.rooms.file.models.get(m))
                    .flat_map(|i| i.mmats.iter().filter_map(|mm| mm.material))
                    .filter_map(|mat| d.rooms.file.scene.materials.get(&mat))
                    .filter_map(|m| textures.iter().find(|t| t.object == m.texture))
                    .map(|t| t.clut)
                    .collect();
                assert!(cluts.iter().any(|c| from.contains(c)), "bg {bg} {name}: {cluts:?} not swapped");
            }
            assert_eq!(seen, 8, "bg {bg}: the lake's clumps in {}", d.rooms.file.stem);
        }
    }

    /// The Gott statue room: story area 14's type-14 row (floor 1, room 4)
    /// and a random dungeon's (`symFlag`) are built from `symroom[type]`,
    /// drawn from the dungeon's file, and give a kind-2 slot (their
    /// `OBJ_0ppg` dummy) for the idol `SetIDOL` places there.
    #[test]
    fn the_gott_statue_room() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let story = WorldMan { event: 14, field_type: 10, hack: 2, level_max: 4, room_max: 9, ..WorldMan::default() };
        let mut cases = vec![(story, 14, 0)];
        for (k, ft) in [0u32, 1, 2, 7].into_iter().enumerate() {
            let seed = 0x1234_5677 + 7919 * k as u32;
            let wm = WorldMan {
                dungeon_seed: [seed, seed ^ 0x55, seed ^ 0xaa],
                field_type: ft,
                level_max: 3,
                room_max: 7,
                hack: 2,
                ..WorldMan::default()
            };
            cases.push((wm, 0, 0));
        }
        for (wm, field, server) in cases {
            let mut scene = Scene::init();
            scene.area = 2;
            scene.field = field;
            scene.server = server;
            let mut d = DungeonArea::new(&archive, &wm, &scene).unwrap();
            let sym = d.tables.symroom[d.dtype as usize];
            let rooms: Vec<(usize, usize)> = (0..d.floors.len())
                .flat_map(|f| (0..15).map(move |i| (f, i)))
                .filter(|&(f, i)| d.slot(f, i).is_some_and(|s| s.model == Some(sym)))
                .collect();
            assert_eq!(rooms.len(), 1, "field {field} type {}: the statue rooms {rooms:?}", d.dtype);
            let (f, i) = rooms[0];
            let slots = d.gim_slots();
            assert!(
                slots.iter().any(|s| (s.floor as usize, s.block as usize, s.kind) == (f, i, 2)),
                "field {field} type {}: no idol slot in {f}/{i}",
                d.dtype
            );
            d.set_room(f, i);
            let room = d.room.as_ref().unwrap();
            let drawn = room.objects.iter().filter(|o| d.rooms.obj_models.contains_key(&o.target)).count();
            assert!(drawn > 0, "field {field}: the statue room's pieces");
        }
    }

    /// Debug: the trap room's door leaf through open_door and close_door.
    #[test]
    #[ignore]
    fn area14_door_leaf() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let wm = WorldMan { event: 14, field_type: 10, hack: 2, level_max: 4, room_max: 9, ..WorldMan::default() };
        let mut scene = Scene::init();
        scene.area = 2;
        scene.field = 14;
        let mut d = DungeonArea::new(&archive, &wm, &scene).unwrap();
        d.set_room_with(0, 3, false);
        let leaf = |d: &DungeonArea| {
            d.doors
                .iter()
                .map(|a| {
                    a.objects
                        .iter()
                        .filter(|o| d.rooms.file.ccs.object_name(o.controller) == Some(LEAF))
                        .map(|o| (o.local.map(|c| c.map(ee::f)), ee::f(o.world[3][2])))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let a = &d.doors[0];
        println!("dtype {} anim frames {} time {}", d.dtype, d.rooms.file.anims[a.play.anim].frames, a.play.time);
        println!("shut {:?}", leaf(&d));
        d.open_door(0, 3);
        println!("open {:?} time {}", leaf(&d), d.doors[0].play.time);
        d.close_door2();
        for k in 0..40 {
            d.move_door(Some(3), false, false);
            if k % 5 == 0 {
                println!("close {k} start {} {:?}", d.door.close_start, leaf(&d));
            }
        }
    }
}
