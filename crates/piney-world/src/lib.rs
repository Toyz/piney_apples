//! The World (`DATA/GCMN.PRG`, the field game) as a state machine: a pad in,
//! a [`piney_draw::Frame`] out, once per game frame at 30 frames a second.
//! [`World::step`] is one frame of the town's tasks in priority order:
//! `ccThGameCtrl` (33, `event::talk`), `ccThCameraExecute` (33,
//! `camera::evcam`), `ccThCamera` (40, `camera`), `ccThSpc` / `ccThPlayer` /
//! `ccThFellowNN` (48-50, the battle's tasks: `combat`), `ccThEntryCtrl` (64,
//! `event::gate`, `town::merchant`, `town::pc`) and `ccThFieldDisp` (96, the
//! towns). The event interpreter runs outside, in `piney-game`
//! (docs/engine/field-game.md). The modules go by subsystem: `town`,
//! `story_map`, `field`, `dungeon`, `chara`, `camera`, `event`, `grunty`,
//! `sky`; `ee`, `mt`, `hit`, `draw`, `area` are the shared core.
// A library reports through its caller, never on stdout.
#![deny(clippy::print_stdout)]
#![forbid(unsafe_code)]

pub mod area;
pub mod camera;
pub mod chara;
pub mod combat;
pub mod draw;
pub mod dungeon;
pub mod ee;
pub mod event;
pub mod field;
pub mod grunty;
pub mod hit;
pub mod map;
pub mod mt;
pub mod sky;
pub mod sprite;
pub mod story_map;
pub mod systems;
pub mod town;
pub mod world;

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
pub use piney_desktop::SaveState;

use crate::area::AreaMap;
// The town world's façade forwards (the one-world relay) name these.
use crate::camera::{Camera, Scheme};
use crate::chara::kite::Kite;
use crate::chara::npc;
use crate::ee::{F, ONE, V4};
use crate::event::entry;
use crate::event::gate::Gate;
use crate::town::Town;

/// `ccSystem.frameRate` in the field: two vertical blanks a frame
/// (`ccSetupNewGame` 0x001688d0).
pub const FRAME_RATE: u32 = 2;

/// Frames `ccSetupGameCtrl` fades out and in over (`EntryFade(10, ...)`,
/// `ContinueFade(n, 10, 0)`), and the two frames it holds black between.
pub const FADE_FRAMES: u32 = 10;
pub const HOLD_FRAMES: u32 = 2;

/// How a scene's set-up (`ccSetupGameCtrl`) begins: `ccSys+0x358` (frames
/// since power-on) as the scene is made, and whether the scene before drew
/// the fade out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Arrival {
    pub frames: u32,
    pub faded: bool,
    /// The start (`StartPos`) the scene before left, which a Root Town
    /// with no start of its own keeps.
    pub before: (V4, V4),
}

impl Arrival {
    /// `ccSys+0x358` at the set-up's `ccInitRand`, after its fade out
    /// (unless the scene before drew it) and its hold.
    pub fn rand_count(self) -> u32 {
        self.frames.wrapping_add(HOLD_FRAMES + if self.faded { 0 } else { FADE_FRAMES })
    }
}

/// Kite's start in Mac Anu (`WORLD_MAN::SetCharPosition` 0x001a1190, town
/// 0), the one the others' starts share the heading of.
pub const START_POS: V4 = [0, 0x45af_0000, 0x4416_0000, ONE];
pub const START_DIRC: V4 = [0, 0, 0, 0];

/// The towns [`World::enter`] builds: every [`town::TOWNS`] def's number
/// (Quarantine's sixth, which [`Town::open`] refuses on another volume,
/// among them).
pub fn town_ported(town: i32) -> bool {
    town::TOWNS.iter().any(|d| d.no == town)
}

/// `WORLD_MAN::SetCharPosition` (main 0x001a1190) in a town: the player's
/// start by `game.town` ([`town::TownDef::start`]), as a position (w 1) and
/// a heading (`dirc`, z); where it has no case (Quarantine's town 5, QUA
/// main 0x001b3ac0), `StartPos` (main bss 0x00670190) keeps `before`.
pub fn town_start(town: i32, before: (V4, V4)) -> (V4, V4) {
    town::TOWNS.iter().find(|d| d.no == town).and_then(|d| d.start).unwrap_or(before)
}

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// `ccSoundFadeOut()`: `ccSetupGameCtrl` on changing scene.
    SoundFadeOut,
    /// `ccAllSoundOff()`, after the fade out.
    AllSoundOff,
    /// `ccEvVoiceStop()`: `ccSetupGameCtrl`'s in place of `ccAllSoundOff`
    /// when the scene stays (MUT main 0x00168d1c): a room or floor of the
    /// same dungeon.
    EvVoiceStop,
    /// `ccSndSQLoad(n)`: the town's sequence bank (2).
    SqLoad(i32),
    /// `ccSndBgmCtrl()`: the area's music, once the fade in ends.
    BgmCtrl,
    /// `ccSnd.gameStart` = 1, just before the fade in (`ccSetupGameCtrl`,
    /// 0x001694ec).
    GameStart,
    /// `ccGame.enableReset`.
    EnableReset(bool),
    /// `effTransfer(this)`: the arrival's and the leave's effect around
    /// Kite.
    Transfer,
    /// `effWarpTransfer(this)`.
    WarpTransfer,
}

/// Where `ccSetupGameCtrl` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The fade out, frames drawn so far.
    FadeOut(u32),
    /// Held black while the town loads.
    Hold(u32),
    /// The field's tasks running; the count of frames since they started
    /// (the fade in runs over the first ten).
    Play(u32),
}

/// An event's town NPC (`entry 3|4 code marker`) waiting for the entry
/// control's set-up: the class its row makes, the row, the marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EventEntry {
    class: npc::EventClass,
    code: i16,
    marker: i16,
    /// From Mutation on, `entry 3 code marker -1`: a SEARCH PC, made by
    /// its own set-up (MUT gcmn 0x00521e50) and not set at the marker.
    search: bool,
}

/// newlib's `rand()` (`INF SLUS_202.67:0x00133a38`): a 64-bit LCG, the top
/// half's low 31 bits. Its state is shared with every other caller in the
/// game, so the sequence here is only the same kind of sequence.
pub use piney_battle::rand::Rand;

pub use world::World;

impl World {
    /// `ccSetupNewGame` and `ccSetupGameCtrl` for area 0 (a Root Town), town
    /// `lastTown` (`saveData+0x8426`, 0-4): the town and Kite read from the disc, the camera and
    /// player as the tasks set them up at the town's start. The frames that
    /// follow fade out, hold, then fade the town in.
    pub fn log_in(iso: &mut Iso, archive: Arc<Archive>, mut save: SaveState) -> piney_data::Result<World> {
        let scene = area::Scene::log_in(&mut save.save);
        World::town(iso, archive, save, scene, area::WorldMan::default(), ([0; 4], [0; 4]))
    }

    /// `ccSetupGameCtrl` for a Root Town (area 0, `WORLD_MAN::GO(0)`) on
    /// `WORLD_MAN` as the last area left it; `before` the start
    /// (`StartPos`) of the scene before, which a town with no start of its
    /// own keeps ([`World::enter`] for area 0).
    pub(crate) fn town(
        iso: &mut Iso,
        archive: Arc<Archive>,
        mut save: SaveState,
        scene: area::Scene,
        world_man: area::WorldMan,
        before: (V4, V4),
    ) -> piney_data::Result<World> {
        let volume = iso.volume()?;
        let (char_names, char_files): (Vec<Vec<u8>>, Vec<String>) = load_char_tbl(iso)?.into_iter().unzip();
        let town_no = i32::from(save.save.u8(offset::LAST_TOWN) as i8);
        if !town_ported(town_no) {
            return Err(piney_data::Error::NotFound(format!("town {town_no}: no such Root Town")));
        }
        // ccSetupNewGame: newGameFlag set to 1.
        if save.save.u8(offset::NEW_GAME_FLAG) as i8 != 1 {
            save.save.set_u8(offset::NEW_GAME_FLAG, 1);
        }
        let crisis = save.save.u8(offset::CRISIS) != 0;
        let mut town = Town::open_with(&archive, volume, town_no, crisis, save.field_rand)?;
        town.take_statics(&save.scroll_statics);
        town.keep_start(before);
        if let Err(e) = town.setup_map(&archive, volume, &scene) {
            tracing::warn!("the town's map: {e}");
        }
        let gate = Gate::new(&archive, &town.core.file)?;
        let mut combat = crate::combat::Combat::new(Rc::new(crate::combat::BattleData::read(iso)?), 1);
        combat.town_nav =
            Some(crate::combat::stage::TownNavData::read(volume, town_no as usize, &town.core.file, &mut combat.keep)?);
        // ccPlayer::ccPlayer's EquipWeapon: the blades the save has him hold.
        let kite = match crate::chara::body::weapon_of(&combat.data.t, &save.save, 0) {
            Some((w, _)) => Kite::read_armed(&archive, &w)?,
            None => Kite::read(&archive)?,
        };
        // Kite's charTbl row as the save holds it (spcParam[0]).
        let (pos, rot, starts) = town.start(&scene);
        let start = (pos, [0, 0, rot, 0]);
        let scheme = Scheme::new(i32::from(save.save.u8(offset::CAM_TYPE) as i8));
        let mode = save.save.u8(offset::CAMERA_MODE) as i8;
        let camera = Camera::new(volume, start.0, start.1, mode, scheme);
        combat.rand = Rand(save.rand);
        // ccSoundFadeOut for a replaced scene (CheckSceneReplace, INF main
        // 0x00168a34), as in every area.
        let requests = [scene.changed().then_some(Request::SoundFadeOut), Some(Request::EnableReset(false))]
            .into_iter()
            .flatten()
            .map(world::Request::Game)
            .collect();
        let w = world::World::town_core(
            save,
            volume,
            scene,
            Box::new(town),
            (kite, camera),
            combat,
            archive,
            (char_names, char_files),
            (pos, rot, starts),
            requests,
        );
        let mut w = w;
        // WORLD_MAN::GO (main 0x0019f8e0): hackFlag 2 for every area, 3
        // when the save's crisis byte is set, before it branches to the
        // town (the menu's noise reads it, event-vm.md's phase 4).
        w.world_man = area::WorldMan { hack: if crisis { 3 } else { 2 }, ..world_man };
        w.cast = Box::new(town::cast::TownCast {
            gate,
            merchants: Vec::new(),
            dogs: Vec::new(),
            grunties: Vec::new(),
            grunty_tables: None,
            npcs: Vec::new(),
            town_pcs: None,
            event_npcs: Vec::new(),
            placed: false,
            pcs: Vec::new(),
            outs: Vec::new(),
            flags: Vec::new(),
            race: None,
        });
        Ok(w)
    }
}

/// `charTbl` (INF DEMO.PRG 0x0040dc80: 18 rows, 21 from Mutation on), the
/// volume's (`newgame::chars`): each character's name and `ccsname`, the
/// file its body is in (`ctu1body` Kite, `cbu4body` Orca).
pub fn load_char_tbl(iso: &mut Iso) -> piney_data::Result<Vec<(Vec<u8>, String)>> {
    let chars = piney_data::tables::newgame::of(iso.volume()?).chars;
    Ok(chars
        .iter()
        .map(|c| {
            let name = c.base.name.map(piney_data::tables::sjis::encode).unwrap_or_default();
            (name, c.base.ccsname.unwrap_or("").to_string())
        })
        .collect())
}

/// `ccSaveData::Init`'s `assignPADaction` (0x001743d0): X; a save made by
/// `SaveState::fresh` leaves it 0.
pub const DEFAULT_ACTION: u16 = 0x40;

/// An event instruction's character `type` as [`entry::Kind`]
/// (`ccEntryEventMng`'s bit per type: 0-2 the party, 3-4 town NPCs, 5-6
/// objects and enemies).
pub fn event_kind(ty: i16) -> Option<entry::Kind> {
    match ty {
        0..=2 => Some(entry::Kind::Spc),
        3 | 4 => Some(entry::Kind::Npc),
        5 | 6 => Some(entry::Kind::Enemy),
        _ => None,
    }
}

/// A character as the HUD shows it ([`World::char_info`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharInfo {
    pub who: world::CharId,
    /// The name's bytes as the game has them.
    pub name: Vec<u8>,
    /// The base parameters' type flags (+0x08).
    pub flags: u32,
    pub height: F,
    /// A party member's `ccSpcParam` in the save, its offset.
    pub spc_param: Option<usize>,
}
