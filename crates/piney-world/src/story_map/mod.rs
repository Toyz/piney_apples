//! Story maps: the `EVENTAREA` subclasses `WORLD_MAN::GO(1)` (main
//! 0x0019f8e0) makes in place of a generated field, by `game.field`, and
//! holds as `eventmap` (docs/engine/evarea.md). Each class is a
//! [`StoryMapDef`]: its file, its blocks (what `ChangeBlock` makes: the
//! systems, lights, fog, start and `Draw`'s steps), its doors and its own
//! systems; one [`StoryArea`] holds whichever `GO(1)` made ([`build`]).

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

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::classes::StoryMapGameClass;
use piney_data::dungeon::Rng;
use piney_data::save::SaveData;
use piney_data::statics::Position;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;
use piney_desktop::{MOVE_TEX_ROWS, SCROLL_STATICS};

use crate::area::AreaCore;
use crate::camera::Camera;
use crate::draw::Fogging;
use crate::ee::{F, ONE, V4};
use crate::hit::{HitModel, Hits};
use crate::sprite::EffSprite;
use crate::systems::MapView;
use crate::systems::{
    BuildCx, Carried, CarriedMut, DrawCx, Drawn, MapCx, PlaceCx, Report, Request, Signal, Step, SystemDef, Systems,
    paint, shorten,
};
use crate::town::{FogParams, TownLights};

/// `WORLD_MAN::GO(1)`'s bounds for a story map: -48000..48000 both ways.
pub const BOUNDS: [F; 4] = [0xc73b_8000, 0xc73b_8000, 0x473b_8000, 0x473b_8000];

/// `WORLD_MAN`'s layers (`GO`, `ccLayer::Init` priorities) the maps draw on.
pub mod layer {
    /// `bgLayer[k]` (+0x494 on): -100, -90, .. -50.
    pub const fn bg(k: i16) -> i16 {
        -100 + 10 * k
    }
    pub const BG0: i16 = bg(0);
    pub const BG1: i16 = bg(1);
    pub const BG2: i16 = bg(2);
    pub const BG3: i16 = bg(3);
    /// `SetActiveLayer(1)` objLayer (+0x4bc), `(2)` obj2Layer (+0x4c0),
    /// `(3)` effLayer (+0x4c8: the jump table at main 0x00355790 sends 3
    /// there and 4 to floorLayer).
    pub const OBJ: i16 = crate::draw::OBJ_LAYER;
    pub const OBJ2: i16 = crate::draw::OBJ2_LAYER;
    pub const EFF: i16 = crate::draw::EFF_LAYER;
    pub const FLOOR: i16 = crate::draw::FLOOR_LAYER;
    /// `refLayer`.
    pub const REF: i16 = 30;
}

/// Where each class's `EA_moveTex` rows keep their scroll in
/// `SaveState::move_tex`: gcmn's data, which lasts from `ccSetupNewGame`'s
/// load of GCMN.PRG to the next, across visits.
pub mod move_tex {
    /// `EA_moveTex02`'s two rows.
    pub const EA02: usize = 0;
    pub const EA04: usize = 2;
    pub const EA05: usize = 3;
    pub const EA06: usize = 4;
    /// `EA_moveTex07`'s two rows.
    pub const EA07: usize = 5;
}

/// What `WORLD_MAN::Enter` does from a story map's door.
pub enum Enter {
    /// Nothing (the arenas).
    Stay,
    /// `ChangeBlock`, then `ChangeScene` to this block.
    Block(i32),
    /// `ChangeArea(2, 0)`: the area's dungeon.
    Dungeon,
}

/// A door's way as `WORLD_MAN::Enter` takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Door {
    Stay,
    Dungeon,
    /// `ChangeBlock(n)`, then the scene to block n.
    Block(i32),
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

/// How `SetCharPosition` stands the party in a map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spread {
    /// The others about the leader ([`party_starts`]).
    About(StartSpread),
    /// Field 12's own: [`StartSpread::BEHIND`], then the first two at
    /// their places and everyone at height `z`.
    Placed { others: [[F; 2]; 2], z: F },
}

/// The leader at `start` facing its w; the others at `spread` and (0, +300)
/// from him, facing as he does.
pub fn party_starts(start: V4, spread: StartSpread) -> (V4, F, [V4; 3]) {
    use crate::ee::{add, sub};
    let [x, y, z, dirc] = start;
    let StartSpread { side, back } = spread;
    let others =
        [[add(x, side), add(y, back), z, ONE], [sub(x, side), add(y, back), z, ONE], [x, add(y, 0x4396_0000), z, ONE]];
    ([x, y, z, ONE], dirc, others)
}

/// What a constructor or `ChangeBlock` hands `WORLD_MAN::SetStartPos`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// Nothing: `eventStartPos` as it was (the ending's; a system sets it).
    Keep,
    At(V4),
    /// A dummy's position, w its turn about z (the DummyPosRot chunk's
    /// +0x28 copied into +0x1c), the class's own values over the lanes it
    /// writes. A dummy the file lacks is an error, unless `or_keep`
    /// (`EVENTAREA07`'s block 1: the game reads through a null chunk).
    Marker {
        name: &'static str,
        lanes: [Option<F>; 4],
        or_keep: bool,
    },
    /// By `game.areaPrev`: from the dungeon (2) the first, else the second.
    FromDungeon(&'static Start, &'static Start),
}

impl Start {
    /// A dummy's place and turn.
    pub const fn marker(name: &'static str) -> Start {
        Start::Marker { name, lanes: [None; 4], or_keep: false }
    }

    /// Where it puts the party in `file`, `area_prev` being
    /// `game.areaPrev`; None keeps the start.
    fn of(&self, file: &SceneFile, area_prev: i32) -> Result<Option<V4>> {
        match *self {
            Start::Keep => Ok(None),
            Start::At(v) => Ok(Some(v)),
            Start::FromDungeon(d, other) => if area_prev == 2 { d } else { other }.of(file, area_prev),
            Start::Marker { name, lanes, or_keep } => {
                if file.ccs.find_object(name).is_none() {
                    return if or_keep { Ok(None) } else { Err(Error::NotFound(format!("{}: {name}", file.stem))) };
                }
                let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot(name), &file.ccs, &file.scene);
                let at = [pos[0], pos[1], pos[2], rot[2]];
                Ok(Some(std::array::from_fn(|k| lanes[k].unwrap_or(at[k]))))
            }
        }
    }
}

/// The light group a block takes: its light animation's ambient and each
/// light of its class's table at frame 1 (one `_AnimateForward`), in the
/// table's order. A name the file lacks is an error, or skipped with
/// `skip_missing` (`eventareaB0Light`'s omni lights).
#[derive(Clone, Copy, Debug)]
pub struct LightsDef {
    pub anim: &'static str,
    pub table: &'static [&'static str],
    pub skip_missing: bool,
}

/// What a class's `ChangeBlock` makes of a block (or its constructor, for a
/// class of one): the file its names are in (None the class's), its
/// lights, fog, start, systems, and its `Draw`'s steps, awake and through
/// a frame the tasks sleep (None the same).
#[derive(Debug)]
pub struct BlockDef {
    pub file: Option<&'static str>,
    pub lights: Option<LightsDef>,
    pub fog: Option<FogParams>,
    pub start: Start,
    pub systems: &'static [SystemDef],
    pub order: &'static [Step],
    pub asleep: Option<&'static [Step]>,
}

/// A block with nothing: no lights or fog, the start kept.
pub const BASE_BLOCK: BlockDef =
    BlockDef { file: None, lights: None, fog: None, start: Start::Keep, systems: &[], order: &[], asleep: None };

/// How the port draws a frame the field's tasks sleep through (the game's
/// `Draw` does not run in it; the picture stays): from the block's sleeping
/// steps, run on a copy of the map that is then dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asleep {
    /// The pieces as the steps leave the copy.
    Stepped,
    /// The pieces the steps chose, the map as it stands (its scrolls,
    /// animations and places as the last awake frame left them).
    Standing,
}

/// A save byte that starts a map at another block (QUA gcmn 0x0030a170:
/// field 8's last stage once Corbenik's third form has begun).
#[derive(Clone, Copy, Debug)]
pub struct StageByte {
    pub volume: Volume,
    pub byte: usize,
    pub block: usize,
}

/// What a story map is: its class, the fields and discs `GO(1)` makes it
/// for, its file, whose statics tables it reads, the stage its class's
/// names are written for, its clear colour, how the party stands, its
/// doors (from block 0, or -1 the constructor's, and from any other), its
/// blocks, the constructor's own start after the first block, its systems
/// beside the blocks', and what of it outlives the scene. One `static` a
/// map, so a def is its address.
#[derive(Debug)]
pub struct StoryMapDef {
    pub class: StoryMapGameClass,
    pub fields: &'static [i32],
    pub volumes: &'static [Volume],
    /// The stream (+0x1a4); empty for none (the ending's).
    pub file: &'static str,
    /// Infection's for the classes that read them on every disc, None the
    /// disc's own.
    pub tables: Option<Volume>,
    pub stage: Option<&'static str>,
    /// `ccSys.bgColor` as the class leaves it.
    pub clear: [u8; 3],
    pub spread: Spread,
    pub doors: [Door; 2],
    pub blocks: &'static [BlockDef],
    pub start: Option<Start>,
    pub stage_byte: Option<StageByte>,
    pub systems: &'static [SystemDef],
    pub asleep: Asleep,
    /// `WORLD_MAN::Quit` spares it for a field (area 15's).
    pub kept: bool,
    /// `ccBossEntryStart` centres the boss on the map's start (the arenas'
    /// `DMY_center01`), not on the player.
    pub boss_at_start: bool,
}

/// One def is one map.
impl PartialEq for StoryMapDef {
    fn eq(&self, other: &StoryMapDef) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for StoryMapDef {}

/// What `EVENTAREA` gives every map: every disc, the disc's tables, black,
/// the others behind the leader, doors that do nothing, one empty block.
pub const BASE_STORY_MAP: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea,
    fields: &[],
    volumes: &Volume::ALL,
    file: "",
    tables: None,
    stage: None,
    clear: [0; 3],
    spread: Spread::About(StartSpread::BEHIND),
    doors: [Door::Stay; 2],
    blocks: &[BASE_BLOCK],
    start: None,
    stage_byte: None,
    systems: &[],
    asleep: Asleep::Stepped,
    kept: false,
    boss_at_start: false,
};

/// `GO(1)`'s maps in the order its switch tries them.
pub static MAPS: [&StoryMapDef; 20] = [
    &boss_arena::ARENA_1,
    &boss_arena::ARENA_2,
    &boss_arena::ARENA_3,
    &boss_arena::ARENA_4,
    &boss_arena::ARENA_5,
    &boss_arena::ARENA_6,
    &boss_arena::ARENA_7,
    &boss_arena::ARENA_8,
    &holy_ground::HOLY_GROUND,
    &giant::GIANT,
    &abyss::AREA_43,
    &truths_core::AREA_13,
    &ending::ENDING,
    &disc::DISC_9,
    &disc::DISC_10,
    &disc::DISC_11,
    &disc::DISC_12,
    &sky::MIRROR,
    &sky::MIRROR_ARENA,
    &sky::WHALE,
];

impl StoryMapDef {
    /// The map `GO(1)` makes for `field` on `volume`, if any.
    pub fn of(field: i32, volume: Volume) -> Option<&'static StoryMapDef> {
        MAPS.iter().copied().find(|d| d.fields.contains(&field) && d.volumes.contains(&volume))
    }
}

/// A story map's collision as `GO(1)` sets it: `game.area` 1, the event
/// area's flag and bounds, no height map, `defSE`; the models' Hit chunks
/// are each block's.
pub fn event_hits(volume: Volume, def_se: u32) -> Hits {
    let bounds = Some(BOUNDS);
    Hits { volume, area: 1, bounds, heights: None, def_se, event_area: true, ..Hits::default() }
}

/// What `GO(1)` makes a map from: the disc, `game.field`, `game.areaPrev`,
/// `game.fieldPrev`, `game.server`, the save, and what the scene before
/// left.
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

/// What `WORLD_MAN` keeps of an area between its scenes: the dungeons made
/// on the way in (`WORLD_MAN.dungeon[n]`, with `lastRoom`), or the story
/// map `WORLD_MAN::Quit` spares for a field (area 15's `eventmap`).
pub enum Kept {
    Dungeon(Box<crate::dungeon::Dungeons>),
    Story(Box<StoryArea>),
}

/// `GO(1)`'s map for `env.field` (`eventAreaNumber`, whose
/// `EVENTAREA_INFO.model` is `model`), or None for a generated field: the
/// map `WORLD_MAN::Quit` kept, coming back from a field, else a new one,
/// gcmn's scrolls carried into it.
pub fn build(env: &mut MapEnv<'_>, model: i32) -> Result<Option<Box<StoryArea>>> {
    let Some(def) = StoryMapDef::of(env.field, env.volume).filter(|_| model != 0) else { return Ok(None) };
    let statics = env.scroll_statics;
    if def.kept {
        let kept = match env.kept.take() {
            Some(Kept::Story(a)) if a.def == def && env.area_prev == crate::area::kind::FIELD => Some(a),
            _ => None,
        };
        if let Some(mut a) = kept {
            a.take(&Carried { statics: &statics, move_tex: None, field: None });
            return Ok(Some(a));
        }
    }
    let mut a = StoryArea::new(def, env)?;
    let move_tex = env.move_tex;
    a.take(&Carried { statics: &statics, move_tex: Some(&move_tex), field: None });
    Ok(Some(Box::new(a)))
}

/// A story map as `WORLD_MAN.eventmap` holds it: its def, the core every
/// area holds, its block, the blocks' own files, its systems and what its
/// last `Draw` made.
pub struct StoryArea {
    pub def: &'static StoryMapDef,
    /// The stream, the block's lights, the collision, `fieldrand` when a
    /// system draws from it (None leaves the game's as the map found it),
    /// `eventStartPos` as the last `SetStartPos` left it.
    pub core: AreaCore,
    /// `blk`: the block `ChangeBlock` made last.
    pub block: i32,
    archive: Arc<Archive>,
    files: Vec<Option<Rc<SceneFile>>>,
    systems: Systems,
    frame: Vec<Drawn>,
}

impl StoryArea {
    /// The class's constructor: its stream and the blocks' own, its
    /// systems, the first block (or the save's), then its own start.
    pub fn new(def: &'static StoryMapDef, env: &mut MapEnv) -> Result<StoryArea> {
        let file = match def.file {
            "" => Rc::new(SceneFile::empty()),
            stem => Rc::new(SceneFile::read(env.archive, stem)?),
        };
        let files = def
            .blocks
            .iter()
            .map(|b| b.file.map(|stem| SceneFile::read(env.archive, stem).map(Rc::new)).transpose())
            .collect::<Result<_>>()?;
        let lights = TownLights { ambient: glam::Vec3::ZERO, lights: Vec::new(), fog: None };
        let hits = event_hits(env.volume, env.def_se);
        let core = AreaCore { file: file.clone(), hits, lights, field_rand: None, start: env.event_start };
        let mut a = StoryArea {
            def,
            core,
            block: -1,
            archive: env.archive.clone(),
            files,
            systems: Systems::default(),
            frame: Vec::new(),
        };
        let map = MapCx {
            field: env.field,
            field_prev: env.field_prev,
            server: env.server,
            save: Some(env.save),
            stage: def.stage,
        };
        let mut cx = BuildCx {
            archive: env.archive,
            tables: def.tables.unwrap_or(env.volume),
            volume: env.volume,
            file: &file,
            stem: def.file,
            crisis: false,
            rng: env.field_rand,
            map: Some(map),
            cc: shorten(Some(&mut *env.cc)),
            field: None,
            made: Vec::new(),
        };
        let systems = Systems::build(def.systems, &mut cx)?;
        let mut rng = cx.rng;
        a.systems = systems;
        a.make_block(0, env.area_prev, &mut rng, map, shorten(Some(&mut *env.cc)))?;
        if let Some(s) = def.stage_byte.filter(|s| s.volume == env.volume && env.save.record()[s.byte] != 0) {
            a.make_block(s.block, env.area_prev, &mut rng, map, None)?;
        }
        if let Some(start) = &def.start
            && let Some(at) = start.of(&a.core.file, env.area_prev)?
        {
            a.core.start = at;
        }
        a.core.field_rand = def.uses_rand().then_some(rng);
        Ok(a)
    }

    /// `ChangeBlock(b)`: the block's systems made, its lights, fog and
    /// start, then every system placed (the collision list anew).
    fn make_block(
        &mut self,
        b: usize,
        area_prev: i32,
        rng: &mut Rng,
        map: MapCx,
        cc: Option<&mut (dyn piney_battle::rand::Rng + '_)>,
    ) -> Result<()> {
        let block = &self.def.blocks[b];
        let file = self.files[b].clone().unwrap_or_else(|| self.core.file.clone());
        let volume = self.core.hits.volume;
        let archive = self.archive.clone();
        let mut cx = BuildCx {
            archive: &archive,
            tables: self.def.tables.unwrap_or(volume),
            volume,
            file: &file,
            stem: self.def.file,
            crisis: false,
            rng: *rng,
            map: Some(map),
            cc: shorten(cc),
            field: None,
            made: Vec::new(),
        };
        let made = self.systems.build_block(block.systems, &mut cx);
        *rng = cx.rng;
        made?;
        self.block = b as i32;
        self.core.lights = match &block.lights {
            Some(l) => {
                let names = l.table.iter().map(|n| cx.name(n).into_owned());
                let (ambient, lights, _) = light_group(&file, &cx.name(l.anim), names, l.skip_missing)?;
                TownLights { ambient, lights, fog: block.fog }
            }
            None => TownLights { ambient: glam::Vec3::ZERO, lights: Vec::new(), fog: block.fog },
        };
        if let Some(at) = block.start.of(&file, area_prev)? {
            self.core.start = at;
        }
        let hit_models = HitModel::read(volume, &file.ccs)?;
        self.core.hits.models.clear();
        let mut place = PlaceCx { core: &mut self.core, hit_models: &hit_models, rng };
        self.systems.place(&mut place);
        Ok(())
    }

    /// The block standing.
    fn block_def(&self) -> &'static BlockDef {
        &self.def.blocks[self.block.max(0) as usize]
    }

    /// `ChangeBlock(block)` from a door or a script, `game.areaPrev` being
    /// `area_prev`.
    pub fn change_block(&mut self, block: i32, area_prev: i32) -> Result<()> {
        let b = (block.max(0) as usize).min(self.def.blocks.len() - 1);
        let mut rng = self.core.field_rand.unwrap_or(Rng::POWER_ON);
        let map = MapCx { field: -1, field_prev: -1, server: -1, save: None, stage: self.def.stage };
        self.make_block(b, area_prev, &mut rng, map, None)?;
        if let Some(r) = &mut self.core.field_rand {
            *r = rng;
        }
        Ok(())
    }

    /// `WORLD_MAN::Enter` from `block` (the scene's), `game.areaPrev`
    /// being `area_prev`: its door from block 0 (or -1, the
    /// constructor's), else its other.
    pub fn enter(&mut self, block: i32, area_prev: i32) -> Result<Enter> {
        Ok(match self.def.doors[usize::from(block != 0 && block != -1)] {
            Door::Stay => Enter::Stay,
            Door::Dungeon => Enter::Dungeon,
            Door::Block(b) => {
                self.change_block(b, area_prev)?;
                Enter::Block(b)
            }
        })
    }

    /// The systems, for tests and probes to read by kind.
    pub fn systems(&self) -> &Systems {
        &self.systems
    }

    pub fn hits(&self) -> &Hits {
        &self.core.hits
    }

    pub fn hits_mut(&mut self) -> &mut Hits {
        &mut self.core.hits
    }

    pub fn lights(&self) -> &TownLights {
        &self.core.lights
    }

    /// `ccSys.bgColor` as the map leaves it.
    pub fn clear(&self) -> [u8; 3] {
        self.def.clear
    }

    /// `WORLD_MAN::SetCharPosition` in a story map: the leader, his facing,
    /// the others.
    pub fn start_positions(&self) -> (V4, F, [V4; 3]) {
        match self.def.spread {
            Spread::About(s) => party_starts(self.core.start, s),
            Spread::Placed { others: [a, b], z } => {
                let (leader, dirc, mut others) = party_starts(self.core.start, StartSpread::BEHIND);
                others[0] = [a[0], a[1], z, ONE];
                others[1] = [b[0], b[1], z, ONE];
                others[2][2] = z;
                ([leader[0], leader[1], z, ONE], dirc, others)
            }
        }
    }

    /// The map's stream, where `marker_pos` finds `markerEvTbl`'s dummies
    /// (`WORLD_MAN` +0x444's); None for a class that loads none.
    pub fn file(&self) -> Option<&SceneFile> {
        (!self.def.file.is_empty()).then_some(&*self.core.file)
    }

    /// `fieldrand` as the map's draws leave it, for a map that draws from
    /// it; None leaves the game's as the map found it.
    pub fn field_rand(&self) -> Option<&Rng> {
        self.core.field_rand.as_ref()
    }

    pub fn field_rand_mut(&mut self) -> Option<&mut Rng> {
        self.core.field_rand.as_mut()
    }

    /// `WORLD_MAN::GetTransMode()` and `GetTransCenter`: the centre the
    /// party rides about, while the map carries it.
    pub fn trans_center(&self) -> Option<V4> {
        self.core.hits.trans
    }

    /// Where `ccBossEntryStart` centres the boss in this map, if not on
    /// the player.
    pub fn boss_centre(&self) -> Option<V4> {
        let [x, y, z, _] = self.core.start;
        self.def.boss_at_start.then_some([x, y, z, ONE])
    }

    /// What outlives the scene, into the systems.
    pub fn take(&mut self, c: &Carried) {
        self.systems.take(c);
    }

    /// The systems' scroll statics and gcmn's scrolls written back.
    pub fn put(&self, statics: &mut [u32; SCROLL_STATICS], move_tex: &mut [[u32; 2]; MOVE_TEX_ROWS]) {
        self.systems.put(&mut CarriedMut { statics, move_tex: Some(move_tex), field: None });
    }

    /// The battle's call on `WORLD_MAN.eventmap`.
    pub fn signal(&mut self, s: Signal) {
        let mut rng = self.core.field_rand.unwrap_or(Rng::POWER_ON);
        let hit_models = Vec::new();
        let mut cx = PlaceCx { core: &mut self.core, hit_models: &hit_models, rng: &mut rng };
        self.systems.signal(s, &mut cx);
    }

    /// What the map tells the battle.
    pub fn report(&self) -> Report {
        self.systems.report()
    }

    /// The map's own scene, while the tasks run (area 43's fly-over).
    pub fn scene(&mut self, x: &mut StoryFrame) {
        self.systems.scene(x);
    }

    /// The block's `Draw` for this frame: its steps in order, into the
    /// frame; asleep, its sleeping steps, everything they moved put back
    /// after the frame is drawn ([`StoryArea::draw`]).
    pub fn step(&mut self, v: &MapView, awake: bool, cc: Option<&mut dyn piney_battle::rand::Rng>) {
        let block = self.block_def();
        let steps = if awake { block.order } else { block.asleep.unwrap_or(block.order) };
        self.frame.clear();
        let mut events = Vec::new();
        self.systems.run_rand(steps, &mut self.core, v, &mut self.frame, &mut events, cc);
    }

    /// What the last [`StoryArea::step`] put in the frame, in its order.
    pub fn frame(&self) -> &[Drawn] {
        &self.frame
    }

    /// The scrolled materials' offsets as the last step left them.
    pub fn rows(&self) -> HashMap<u32, [u8; 2]> {
        self.systems.rows(&self.core.file)
    }

    /// The last frame's pieces as generic draws, with `rows`.
    pub fn requests<'a>(&'a self, rows: &'a HashMap<u32, [u8; 2]>) -> Vec<Request<'a>> {
        let fog = self.block_def().fog.map_or(Fogging::None, |f| Fogging::Depth(f.depth()));
        let cx = DrawCx { core: &self.core, fog, rows };
        self.systems.requests(&self.frame, &cx)
    }

    /// `Draw` into the field's layers, its sprites for the effects. While
    /// the tasks sleep the frame is drawn as the def's [`Asleep`] says.
    pub fn draw(
        &mut self,
        layers: &mut Layers,
        to_screen: Mat4,
        v: &MapView,
        awake: bool,
        cc: &mut dyn piney_battle::rand::Rng,
    ) -> Vec<EffSprite> {
        let before = (!awake).then(|| (self.systems.clone(), self.core.field_rand));
        self.step(v, awake, Some(cc));
        // Asleep the map is put back as it stood: before the draw
        // (Standing: the stepped copy's pieces drawn from it), or after.
        let after = match (before, self.def.asleep) {
            (Some((systems, rng)), Asleep::Standing) => {
                (self.systems, self.core.field_rand) = (systems, rng);
                None
            }
            (before, _) => before,
        };
        let rows = self.rows();
        let mut sprites = Vec::new();
        for r in self.requests(&rows) {
            paint(layers, to_screen, &r, &mut sprites);
        }
        if let Some((systems, rng)) = after {
            (self.systems, self.core.field_rand) = (systems, rng);
        }
        sprites
    }
}

impl StoryMapDef {
    /// Whether any of its systems draws from `fieldrand` (the map then
    /// holds it).
    fn uses_rand(&self) -> bool {
        let block = self.blocks.iter().flat_map(|b| b.systems);
        self.systems.iter().chain(block).any(SystemDef::field_rand)
    }
}

/// The lights a story map puts in the group: its light animation `anim`'s
/// ambient and each light of `names` at frame 1 (one `_AnimateForward`),
/// with their objects. A name the file lacks is an error, or skipped with
/// `skip_missing`.
pub fn light_group<S: AsRef<str>>(
    file: &SceneFile,
    anim: &str,
    names: impl IntoIterator<Item = S>,
    skip_missing: bool,
) -> Result<(glam::Vec3, Vec<crate::town::Light>, Vec<u32>)> {
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// `def` as `GO(1)` makes it on `volume` from a new save on server 1,
    /// `game.areaPrev` being `area_prev` and `fieldrand` seeded `seed`.
    pub(crate) fn made(
        archive: &Arc<Archive>,
        volume: Volume,
        def: &'static StoryMapDef,
        area_prev: i32,
        seed: u32,
    ) -> StoryArea {
        made_with(archive, volume, def, area_prev, seed, &SaveData::default(), 1)
    }

    /// [`made`] from `save` on `server`.
    pub(crate) fn made_with(
        archive: &Arc<Archive>,
        volume: Volume,
        def: &'static StoryMapDef,
        area_prev: i32,
        seed: u32,
        save: &SaveData,
        server: i32,
    ) -> StoryArea {
        let mut cc = piney_battle::rand::Genrand::default();
        let mut env = MapEnv {
            archive,
            volume,
            field: def.fields[0],
            area_prev,
            field_prev: -1,
            server,
            save,
            event_start: [0, 0, 0, ONE],
            field_rand: Rng::new(seed),
            move_tex: [[0; 2]; MOVE_TEX_ROWS],
            scroll_statics: [0; SCROLL_STATICS],
            def_se: 0,
            kept: None,
            cc: &mut cc,
        };
        StoryArea::new(def, &mut env).unwrap_or_else(|e| panic!("{:?}: {e}", def.class))
    }
}
