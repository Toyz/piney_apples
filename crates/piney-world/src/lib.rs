//! The World (`DATA/GCMN.PRG`, the field game) as a state machine: a pad
//! in, a [`piney_draw::Frame`] out, once per game frame at 30 frames a
//! second (`docs/engine/field-game.md`).
//!
//! This is the arrival of a new game in Mac Anu, the first Root Town, and
//! Kite walking in it: TOPPAGE's New Game queues `ChangeRequest(5)`
//! (`ccSetupNewGame`, main 0x001687a0: GCMN.PRG, frame rate 2) and
//! `ChangeArea(0, lastTown)`, which queues `ChangeRequest(6)`
//! (`ccSetupGameCtrl`, 0x00168960). That one fades out over 10 frames,
//! loads the town, starts the field's tasks, builds the town
//! (`WORLD_MAN::GO`), places Kite (`ccGetStartPositions`: (0, 5600, 600)
//! facing south, 700 units in front of the Chaos Gate) and fades in over
//! 10 frames while the tasks run. [`World::step`] is one frame of those
//! tasks, in their priority order:
//!
//! ```text
//! ccThGameCtrl (33)  the command target, the action button     talk.rs
//! ccThCameraExecute  the event camera, while an event runs it   evcam.rs
//!              (33)
//! ccThCamera   (40)  cameraMain: L2, the reset button          camera.rs
//! ccThPlayer   (49)  ccPlayer::Main: stick, walls, floor,       player.rs, hit.rs
//!                    camera placement, act and animation, draw  motion.rs, chara.rs
//! ccThFellow   (50)  the party members: the battle's            town_party.rs,
//!                    ccFellow::Main (ActInTown, ManualControl)  combat/town.rs
//! ccThEntryCtrl(64)  the Chaos Gate (ccChgate::main), then the  gate.rs
//!                    merchants and the walking PCs              merchant.rs, rtownpc.rs
//! ccThFieldDisp(96)  the town class's Draw (ROOTTOWN01 in Mac   town.rs, town01.rs ..
//!                    Anu .. ROOTTOWN05 in Lia Fail; Dun         town05.rs, cloud.rs,
//!                    Loireag's and Fort Ouph's clouds and lens  lensflare.rs
//!                    flare too)
//! ```
//!
//! Characters are [`char::Char`]s (a [`body::Body`] at a place); the entry
//! control's are [`entry::Npc`]s. Talking comes out of
//! [`World::take_talk`] as [`talk::TalkRequest`]s; the event scripts
//! place and drive characters through [`World::entry`],
//! [`World::pc_command`], [`World::npc_command`] and the rest of event.rs.
//!
//! The party's registry and members (`ccSpcManager`, `ccPartyManager`:
//! party.rs) and their AI under the event scripts' manual control (ai.rs)
//! are here, and the event camera (evcam.rs). The event interpreter runs
//! outside, in the runtime (`piney-game`'s field host), before these tasks
//! each frame; while `ccSetupGameCtrl` waits on its passes the set-up holds
//! on its last black frame ([`World::set_loading`]). Without an event (no
//! registry `bootParam`) Kite arrives and is free to walk at once. The rest
//! of the party's AI, most menus and sound are not here (the docs page
//! lists them).

pub mod ai;
pub mod area;
pub mod arms;
pub mod body;
pub mod bosscam;
pub mod camera;
pub mod char;
pub mod chara;
pub mod cinema;
pub mod cloud;
pub mod combat;
pub mod dog;
pub mod draw;
pub mod dungeon_area;
pub mod ee;
pub mod entry;
pub mod evarea;
pub mod evarea03;
pub mod evarea07;
pub mod evarea_b0;
pub mod evcam;
pub mod event;
pub mod field_ambient;
pub mod field_area;
pub mod field_firefly;
pub mod field_npcs;
pub mod field_world;
pub mod firefly;
pub mod foe;
pub mod gameover;
pub mod gate;
pub mod grunty;
pub mod hit;
pub mod lattice;
pub mod lensflare;
pub mod map;
pub mod merchant;
pub mod motion;
pub mod mt;
pub mod navi;
pub mod npc;
pub mod party;
pub mod player;
pub mod pose;
pub mod rtownpc;
pub mod story_map;
pub mod talk;
pub mod town;
pub mod town01;
pub mod town02;
pub mod town03;
pub mod town04;
pub mod town05;
pub mod town_party;

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_desktop::anm::Ctx;
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_input::Pad;

pub use piney_desktop::SaveState;

use crate::camera::{CamPad, Camera, Scheme};
use crate::chara::Kite;
use crate::ee::{F, ONE, V4};
use crate::entry::Npc as _;
use crate::gate::Gate;
use crate::player::Player;
use crate::town::Town;

/// `ccSystem.frameRate` in the field: two vertical blanks a frame
/// (`ccSetupNewGame` 0x001688d0).
pub const FRAME_RATE: u32 = 2;

/// Frames `ccSetupGameCtrl` fades out and in over (`EntryFade(10, ...)`,
/// `ContinueFade(n, 10, 0)`), and the two frames it holds black between.
pub const FADE_FRAMES: u32 = 10;
pub const HOLD_FRAMES: u32 = 2;

/// `ChangeScene`'s town number of Mac Anu, and its start position
/// (`WORLD_MAN::SetCharPosition` 0x001a1190, town 0).
pub const MAC_ANU: i32 = 0;
pub const START_POS: V4 = [0, 0x45af_0000, 0x4416_0000, ONE];
pub const START_DIRC: V4 = [0, 0, 0, 0];
/// Dun Loireag's town number.
pub const DUN_LOIREAG: i32 = 1;
/// Carmina Gade's (Mutation on).
pub const CARMINA_GADE: i32 = 2;
/// Fort Ouph's and Lia Fail's (Outbreak on).
pub const FORT_OUPH: i32 = 3;
pub const LIA_FAIL: i32 = 4;

/// The towns [`World::enter`] builds: all five Root Towns.
pub fn town_ported(town: i32) -> bool {
    matches!(town, MAC_ANU | DUN_LOIREAG | CARMINA_GADE | FORT_OUPH | LIA_FAIL)
}

/// `WORLD_MAN::SetCharPosition` (main 0x001a1190) in a town: the player's
/// start by `game.town` - (0, 5600, 600), (0, 3500, 0), (0, 600, 0),
/// (0, 6000, 0), and (600, -3900, 0) facing pi - as a position (w 1) and
/// a heading (`dirc`, z).
pub fn start_position(town: i32) -> (V4, V4) {
    match town {
        1 => ([0, 0x455a_c000, 0, ONE], START_DIRC),
        2 => ([0, 0x4416_0000, 0, ONE], START_DIRC),
        3 => ([0, 0x45bb_8000, 0, ONE], START_DIRC),
        4 => ([0x4416_0000, 0xc573_c000, 0, ONE], [0, 0, ee::PI, 0]),
        _ => (START_POS, START_DIRC),
    }
}

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// `ccSoundFadeOut()`: `ccSetupGameCtrl` on changing scene.
    SoundFadeOut,
    /// `ccAllSoundOff()`, after the fade out.
    AllSoundOff,
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

/// newlib's `rand()` (`INF SLUS_202.67:0x00133a38`): a 64-bit LCG, the top
/// half's low 31 bits. Its state is shared with every other caller in the
/// game, so the sequence here is only the same kind of sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rand(pub u64);

impl Rand {
    pub fn rand(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        ((self.0 >> 32) & 0x7fff_ffff) as i32
    }
}

/// One frame of the field's logic tasks in their order: `cameraMain` on
/// the camera task (40), then `ccPlayer::Main` on the player's (49), which
/// moves Kite, places the camera and picks his act. `camera_mode` is
/// `saveData.cameraMode`, which L2 writes.
pub fn tasks(
    camera: &mut Camera,
    player: &mut Player,
    hits: &mut hit::Hits,
    anims: &[&piney_data::anim::Animation],
    rand: &mut Rand,
    pad: &CamPad,
    camera_mode: &mut i8,
) {
    camera.main(pad, player.body.dirc[2], true, false, camera_mode);
    let mut next = || rand.rand();
    let mut f = player::Frame { pad, camera, hits, anims, rand: &mut next, cmnd_target: false, annihilated: false };
    player.main(&mut f);
}

/// The field game in Mac Anu.
pub struct World {
    save: SaveState,
    /// The disc's volume: whose tables these are.
    volume: piney_data::volume::Volume,
    town: Town,
    gate: Gate,
    kite: Kite,
    player: Player,
    camera: Camera,
    phase: Phase,
    requests: Vec<Request>,
    rand: Rand,
    /// `ccEntryCtrl`'s NPC list: `ccSetMerchant(0)`'s merchants first
    /// (merchant.rs), then everyone else (the walking PCs).
    merchants: Vec<merchant::Merchant>,
    /// `ccSetDog`'s dogs (Dun Loireag), after the merchants.
    dogs: Vec<dog::Dog>,
    dog_events: Vec<(i32, dog::DogEvent)>,
    /// `ccSetChibiGuso`'s Grunties (Dun Loireag), after the dogs; the last
    /// one made is `pgPtr`. Their tables, and what they asked for since
    /// the last take with the Grunty's code.
    grunties: Vec<grunty::Grunty>,
    grunty_tables: Option<Rc<grunty::Tables>>,
    grunty_events: Vec<(i32, grunty::GruntyEvent)>,
    npcs: Vec<Box<dyn entry::Npc>>,
    /// `ccSys+0x358` when `ccInitRand` seeds the town's `ccRand`
    /// ([`World::set_rand_count`]), and the walking PCs `ccEntryRandomNpc`
    /// placed (with the state they share: `ccRand`, the navigation map,
    /// the chat groups).
    rand_count: u32,
    town_pcs: Option<Rc<std::cell::RefCell<rtownpc::TownPcs>>>,
    /// Town NPC entries (`entry 3 code marker`) waiting for the entry
    /// control's set-up, and whether it has run.
    event_npcs: Vec<(i16, i16)>,
    placed: bool,
    pcs: Vec<rtownpc::RtownPc>,
    pc_events: Vec<(i32, rtownpc::PcEvent)>,
    /// What the Administrator's `sysopeAct` started, with his `npcID`.
    merchant_events: Vec<(i32, merchant::SysopEvent)>,
    /// The Chaos Gate's `ccSeOn3D`s (`gateAnm`: 71 as the circle opens,
    /// 72 as it closes) since the last take, at the gate.
    gate_sounds: Vec<(i32, V4)>,
    /// Kite's footsteps since the last take: the note's param, where he
    /// stood, the ground's attribute ([`World::take_steps`]).
    steps: Vec<(u32, V4, u32)>,
    /// Kite's running steps' dust since the last take: his feet, his
    /// heading and his speed ([`World::take_paw_smokes`]).
    paw_smokes: Vec<([V4; 2], F, F)>,
    /// `ccThGameCtrl`'s command target and menu state.
    targeting: talk::Targeting,
    talks: Vec<talk::TalkRequest>,
    /// What the field's menus showed `ccThGameCtrl` this frame.
    menu_view: talk::MenuView,
    /// `eventMng.target[]` (type, code): characters whose talk goes to the
    /// event.
    event_targets: Vec<(i16, i16)>,
    /// The party members (the battle's characters) and the town's
    /// navigation for them.
    party: town_party::TownParty,
    /// `ccSpcManager` and `ccPartyManager` (party.rs); the events' party
    /// entries (type, code, marker, param) for `ccEntryEventMng`, and
    /// whether `ccSPC::Reboot` has built the registered characters.
    spcs: party::Spcs,
    spc_entries: Vec<(i16, i16, i16, i16)>,
    rebooted: bool,
    /// `charTbl[i].ccsname` (DEMO.PRG), each party member's file.
    char_files: Vec<String>,
    char_names: Vec<Vec<u8>>,
    /// `ccSleepAllThread`: the tasks stand still, the frame still draws.
    asleep: bool,
    /// `ccSetupGameCtrl` is waiting on the event task's passes at phases 0
    /// and 2 (and the load): the screen stays black after the hold.
    loading: bool,
    archive: Arc<Archive>,
    // --- the event camera (evcam.rs) ---
    /// `eventMng.cam`, its task and `menu_ban`'s camera part.
    evcam: evcam::EventCam,
    /// `WORLD_MAN::SetCharPosition`'s start for the town: Kite's place and
    /// heading.
    start: (V4, V4),
    /// The `ccEff`s the town's last draw asked for (Dun Loireag's lens
    /// flare and clouds), for the host's effects.
    town_sprites: Vec<town::TownSprite>,
}

/// `charTbl` (INF DEMO.PRG 0x0040dc80: 18 rows, 21 from Mutation on), the
/// volume's (`newgame::chars`): each character's name and `ccsname`, the
/// file its body is in (`ctu1body` Kite, `cbu4body` Orca).
pub fn load_char_tbl(iso: &mut Iso) -> piney_data::Result<Vec<(Vec<u8>, String)>> {
    let chars = piney_data::tables::newgame::of(iso.volume()?).chars();
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
    pub kind: entry::Kind,
    pub code: i32,
    /// The name's bytes as the game has them.
    pub name: Vec<u8>,
    /// The base parameters' type flags (+0x08).
    pub flags: u32,
    pub height: F,
    /// A party member's `ccSpcParam` in the save, its offset.
    pub spc_param: Option<usize>,
}

/// `saveData`'s float at `at` as bits.
fn save_f(save: &SaveState, at: usize) -> F {
    save.save.i32(at) as u32
}

impl World {
    /// `ccSetupNewGame` and `ccSetupGameCtrl` for area 0 (a Root Town), town
    /// `lastTown` (`saveData+0x8426`, 0-4): the town and Kite read from the disc, the camera and
    /// player as the tasks set them up at the town's start. The frames that
    /// follow fade out, hold, then fade the town in.
    pub fn enter(iso: &mut Iso, archive: Arc<Archive>, mut save: SaveState) -> piney_data::Result<World> {
        let volume = iso.volume()?;
        let (char_names, char_files): (Vec<Vec<u8>>, Vec<String>) = load_char_tbl(iso)?.into_iter().unzip();
        let town_no = i32::from(save.save.u8(offset::LAST_TOWN) as i8);
        if !town_ported(town_no) {
            return Err(piney_data::Error::NotFound(format!("town {town_no}: there are five Root Towns, 0-4")));
        }
        // ccSetupNewGame: newGameFlag set to 1.
        if save.save.u8(offset::NEW_GAME_FLAG) as i8 != 1 {
            save.save.set_u8(offset::NEW_GAME_FLAG, 1);
        }
        let town = Town::open(&archive, town_no, save.save.u8(offset::CRISIS) != 0)?;
        let gate = Gate::new(&archive, &town.base.file)?;
        let party = town_party::TownParty::new(iso, town_no as usize, &town.base.file)?;
        // ccPlayer::ccPlayer's EquipWeapon: the blades the save has him hold.
        let kite = match crate::body::weapon_of(&party.combat.data.t, &save.save, 0) {
            Some((w, _)) => Kite::read_armed(&archive, &w)?,
            None => Kite::read(&archive)?,
        };
        // Kite's charTbl row as the save holds it (spcParam[0]).
        let spc = offset::SPC_PARAM;
        let velocity = save_f(&save, spc + 0xd4);
        let height = save_f(&save, spc + 0x18);
        let width = save_f(&save, spc + 0x1c);
        let start = start_position(town_no);
        let player = Player::new(start.0, start.1, velocity, width, height);
        let scheme = Scheme::new(i32::from(save.save.u8(offset::CAM_TYPE) as i8));
        let mode = save.save.u8(offset::CAMERA_MODE) as i8;
        let camera = Camera::new(start.0, start.1, mode, scheme);
        Ok(World {
            save,
            volume,
            town,
            gate,
            kite,
            player,
            camera,
            phase: Phase::FadeOut(0),
            requests: vec![Request::SoundFadeOut, Request::EnableReset(false)],
            rand: Rand(1),
            merchants: Vec::new(),
            dogs: Vec::new(),
            dog_events: Vec::new(),
            grunties: Vec::new(),
            grunty_tables: None,
            grunty_events: Vec::new(),
            npcs: Vec::new(),
            rand_count: 0,
            town_pcs: None,
            event_npcs: Vec::new(),
            placed: false,
            pcs: Vec::new(),
            merchant_events: Vec::new(),
            gate_sounds: Vec::new(),
            steps: Vec::new(),
            paw_smokes: Vec::new(),
            pc_events: Vec::new(),
            targeting: talk::Targeting::default(),
            talks: Vec::new(),
            menu_view: talk::MenuView::default(),
            event_targets: Vec::new(),
            party,
            spcs: party::Spcs::new_game(),
            spc_entries: Vec::new(),
            rebooted: false,
            char_files,
            char_names,
            asleep: false,
            loading: false,
            archive,
            evcam: evcam::EventCam::new(),
            start,
            town_sprites: Vec::new(),
        })
    }

    /// Hold the set-up after its two black frames (true) until the caller
    /// says it may go on (false): `ccSetupGameCtrl` waits there on the event
    /// task's passes at phases 0 and 2 (`ccEnableThEvent(0)`, `(2)`) and on
    /// the load, drawing nothing new, before it starts the field's tasks.
    pub fn set_loading(&mut self, on: bool) {
        self.loading = on;
    }

    /// `ccSystem.frameRate`.
    pub fn frame_rate(&self) -> u32 {
        FRAME_RATE
    }

    /// Entered from a field or dungeon whose own tasks ran under
    /// `ccSetupGameCtrl`'s fade out (the session drew it over them): the
    /// set-up goes on from the frames held black, as it would after the
    /// fade.
    pub fn skip_fade_out(&mut self) {
        if let Phase::FadeOut(_) = self.phase {
            self.phase = Phase::Hold(0);
            self.requests.push(Request::AllSoundOff);
        }
    }

    /// What the action button asked for since the last call, in order. The
    /// menu it opens stays open (and Kite still) until [`World::close_menu`].
    pub fn take_talk(&mut self) -> Vec<talk::TalkRequest> {
        std::mem::take(&mut self.talks)
    }

    /// The field's menu closed (`ccMenuCtrl::CheckMenuType() == -1`):
    /// `ccThGameCtrl` back to its idle state and Kite free to move. The
    /// NPC spoken to hears it from the menu itself (`EntryAffect(target,
    /// plw, 0)` as its list closes: [`World::affect`]).
    pub fn close_menu(&mut self) {
        self.targeting.close_menu();
        self.player.acts.pause = false;
    }

    /// `ccChangeCmndTarget` from the field's menus (the shop pages drop the
    /// target and give it back).
    pub fn change_command_target(&mut self, t: Option<(entry::Kind, i32)>) {
        self.targeting.set_target(t);
    }

    /// `setCameraCtrlType(t)` (0x001611a0) from OPTION's Controller page:
    /// the field camera's scheme (A-1, A-2, B-1, B-2).
    pub fn set_camera_type(&mut self, t: i32) {
        self.camera.scheme = Scheme::new(t);
    }

    /// `ccMenuCtrl::SetMerchantCamera()` (gcmn 0x005269d0) as a shop's
    /// menu opens on the NPC `code` (`cmndTarget`): the camera turned to
    /// it ([`Camera::set_merchant`]); a breeder's view is the one for
    /// `server` (`ccGame.server`). Anything but an NPC on the list is left.
    pub fn set_merchant_camera(&mut self, kind: entry::Kind, code: i32, server: i32) {
        let Some(n) = (kind == entry::Kind::Npc).then(|| self.npc(code)).flatten() else { return };
        let target = if n.flags() & npc::TYPE_BREEDER != 0 {
            let Some(v) = camera::breeder_view(self.volume, server) else { return };
            v
        } else {
            camera::MerchantView::Char { pos: n.char().pos, dirc: n.char().dirc }
        };
        self.camera.set_merchant(target);
    }

    /// `changeCamera(n)` (main 0x001617f0) from the field's menus: 1 gives
    /// the field camera (`tcam`, which kept following Kite) back as a shop
    /// closes.
    pub fn change_camera(&mut self, n: i16) {
        self.camera.change_camera(n);
    }

    /// `cmndTargetFix` from the field's menus.
    pub fn set_target_fix(&mut self, on: bool) {
        self.targeting.fix = on;
    }

    /// `ccChar::EntryAffect(c, plw, cmnd, ...)` as the field's menus call
    /// it: the Chaos Gate's `chaosGateInfluence` (11 opens the circle, 0
    /// closes it) or an NPC's `affectFunc`.
    /// A party member's affect tint (`affectColorFix`, `affectColorRate`,
    /// `affectColor`; the event instruction `piros_colour` sets Piros's):
    /// false when the member is not in the town.
    pub fn set_affect_colour(&mut self, code: i32, fix: i16, rate: i16, colour: u32) -> bool {
        match self.party.actor_mut(code) {
            Some(f) => {
                let a = &mut f.ch.affect;
                a.fix = fix;
                a.rate = rate;
                a.colour = colour;
                true
            }
            None => false,
        }
    }

    /// A party member's affect tint, when it is in the town.
    pub fn fellow_affect(&self, code: i32) -> Option<char::AffectColour> {
        self.party.actor(code).map(|f| f.ch.affect)
    }

    /// Whether a party member (`charTbl` row) is in the town
    /// (`ccSpcManager`'s registry).
    pub fn spc_loaded(&self, code: i32) -> bool {
        self.party.member(code).is_some()
    }

    /// `ccChar::EntryAffect(target, plw, cmnd, 0, 0, 0)` from the menus on
    /// the gate, a town NPC or a registered character.
    pub fn affect(&mut self, kind: entry::Kind, code: i32, cmnd: i16) {
        match kind {
            entry::Kind::Spc => self.spc_affect(code, cmnd),
            entry::Kind::Gimmick if code == i32::from(gate::GIMMICK) => self.gate.influence(cmnd),
            entry::Kind::Npc if self.grunty(code).is_some() => self.grunty_affect(code, cmnd, 0, 0),
            entry::Kind::Npc => {
                if let Some(n) = self.npc_mut(code) {
                    n.influence(cmnd);
                }
            }
            _ => {}
        }
    }

    /// A registered character's `ccFellow::Influence` (gcmn 0x0041bdb0)
    /// through the battle's rules, Kite's stand-in the one who affects it:
    /// 14 and 15 (its menu open) `ccAI::Greeting`, which sets its talkFlag
    /// and turns it to face him, so that its AI stands until 0 (the menu
    /// closed) clears the flag.
    fn spc_affect(&mut self, code: i32, cmnd: i16) {
        let Some(w) = self.party.member(code) else { return };
        let hits = &mut self.town.base.hits;
        let bounds = combat::Combat::bounds(0, hits);
        let c = &mut self.party.combat;
        c.entry_affect(w, c.kite, cmnd, hits, bounds);
    }

    /// What the field's menus show `ccThGameCtrl` (`ccMenu`), before the
    /// frame.
    pub fn set_menu_view(&mut self, view: talk::MenuView) {
        self.menu_view = view;
    }

    /// The command target (`cmndTarget`): who the action button would
    /// speak to now.
    pub fn command_target(&self) -> Option<(entry::Kind, i32)> {
        self.targeting.target
    }

    /// `eventMng.target[]` as the event interpreter holds it ((type, code),
    /// -1 free): talking to those characters runs the event
    /// ([`talk::TalkRequest::Event`]) rather than a menu.
    pub fn set_event_targets(&mut self, targets: &[(i16, i16)]) {
        self.event_targets = targets.iter().copied().filter(|(t, _)| *t >= 0).collect();
    }

    /// The event targets as [`World::set_event_targets`] left them.
    pub fn event_targets(&self) -> &[(i16, i16)] {
        &self.event_targets
    }

    /// Put an NPC on the entry control's list (`ccEntryCtrl::entryNpc`).
    pub fn add_npc(&mut self, npc: Box<dyn entry::Npc>) {
        self.npcs.push(npc);
    }

    /// The frame count `ccInitRand` reads (`ccSys+0x358`, frames since
    /// boot), which picks the town's walking PCs; 0 unless set before the
    /// town's tasks start.
    pub fn set_rand_count(&mut self, count: u32) {
        self.rand_count = count;
    }

    /// `ccEntryEventMng` (main 0x001b62e0) for a Root Town, when the town's
    /// tasks start: the events' town NPC entries (`ccSetRtownPC`, then the
    /// marker's position and rotation), `ccSetMerchant(0)`'s merchants (the
    /// Chaos Gate is already up), then `ccEntryRandomNpc` (0x001b6cf0): the
    /// walking PCs `ccRegisterRandomNpc(n)` chose from `ccRand` as
    /// `ccInitRand` seeded it ([`World::set_rand_count`]), `n` counting
    /// Kite, the party members and the NPCs the events placed (15 PCs for
    /// Kite alone, 14 with Orca). Each goes onto the entry control's list
    /// and its body onto the character list in that order. The first frame
    /// of play does it if it has not been called.
    pub fn place_entries(&mut self) -> piney_data::Result<()> {
        if self.placed {
            return Ok(());
        }
        self.reboot();
        self.placed = true;
        self.place_spc_entries();
        let reserved = 1 + self.party.members().count() as i32 + self.event_npcs.len() as i32;
        let mut mt = mt::Mt::init(self.rand_count);
        let rows = rtownpc::register_random_npc(&mut mt, reserved);
        let no = self.town.base.no;
        let town = Rc::new(std::cell::RefCell::new(rtownpc::TownPcs::new(self.volume, no, &self.town.base.file, mt)?));
        self.town_pcs = Some(town);
        let mut files = rtownpc::Files::new();
        for (code, marker) in std::mem::take(&mut self.event_npcs) {
            self.place_event_pc(&mut files, code, marker)?;
        }
        let player = self.player.body.pos;
        self.merchants = merchant::set_merchants(
            &self.archive,
            self.volume,
            &self.town.base.file,
            &mut self.town.base.hits,
            no,
            player,
        )?;
        // ccSetChaosGate (already up), then ccEntryEventMng's by town: 1
        // ccSetDog and ccSetChibiGuso, 2 and 3 ccSetChibiGuso, then the
        // walking PCs.
        self.dogs = dog::set_dogs(&self.archive, self.volume, &self.town.base.file, &mut self.town.base.hits, no)?;
        self.place_grunties(no)?;
        let town = self.town_pcs.clone().expect("placed above");
        for (slot, &r) in rows.iter().enumerate() {
            if r < 0 {
                continue;
            }
            let row = npc::NpcRow::of(self.volume, r as usize)?;
            let model = rtownpc::load_model(&self.archive, &mut files, &town.borrow().tables, &row)?;
            self.pcs.push(rtownpc::RtownPc::place(&town, &row, &model, slot as i32, player, &mut self.town.base.hits));
        }
        Ok(())
    }

    /// An event's town PC entry (`ccEntryEventMng`'s type 3): `npcTbl` row
    /// `code` through `ccSetRtownPC`, then set at `marker` (its position,
    /// and its rotation as the heading).
    fn place_event_pc(&mut self, files: &mut rtownpc::Files, code: i16, marker: i16) -> piney_data::Result<()> {
        let town = self.town_pcs.clone().expect("the entry control's set-up ran");
        let row = npc::NpcRow::of(self.volume, code as usize)?;
        let model = rtownpc::load_model(&self.archive, files, &town.borrow().tables, &row)?;
        let mut pc = rtownpc::RtownPc::place(&town, &row, &model, -1, self.player.body.pos, &mut self.town.base.hits);
        if let Some((pos, rot)) = self.marker_bits(marker) {
            pc.char.pos = pos;
            pc.char.dirc = [0, 0, rot, 0];
        }
        self.pcs.push(pc);
        Ok(())
    }

    /// An event's town NPC entry: queued for [`World::place_entries`], or
    /// placed at once (at the list's end) once it has run. Only PC rows
    /// (base flags 0x08) are ported; administrators (type 4,
    /// `ccSetMerchant(code)`) are not.
    pub(crate) fn entry_npc(&mut self, ty: i16, code: i16, marker: i16) -> bool {
        let Ok(row) = npc::NpcRow::of(self.volume, code.max(0) as usize) else { return false };
        if ty != 3 || row.flags & npc::TYPE_PC == 0 || code < 0 {
            return false;
        }
        if !self.placed {
            self.event_npcs.push((code, marker));
            return true;
        }
        let mut files = rtownpc::Files::new();
        self.place_event_pc(&mut files, code, marker).is_ok()
    }

    /// `ccSetChibiGuso` (gcmn 0x0050f0b0) in towns 1-3: the rows its
    /// growth record gives, each through `setDog` and `ccPGuso::ccPGuso`
    /// onto the list (the record of a grown one with a kind free starts
    /// over in the save).
    fn place_grunties(&mut self, no: i32) -> piney_data::Result<()> {
        if !(1..=3).contains(&no) {
            return Ok(());
        }
        // The grown ones are made before the record starts over, the young
        // one after.
        let before = self.save.save.clone();
        let rows = grunty::set_chibi_guso(&mut self.save.save, no);
        if rows.is_empty() {
            return Ok(());
        }
        let tables = Rc::new(grunty::Tables::of(self.volume)?);
        let mut bodies = grunty::Bodies::load(&self.archive)?;
        for (num, id) in rows.into_iter().enumerate() {
            let g = grunty::set_dog(
                &self.archive,
                &tables,
                &mut bodies,
                &self.town.base.file,
                &mut self.town.base.hits,
                if id < grunty::ROW_YOUNG { &before } else { &self.save.save },
                no,
                id,
                num as u32,
            )?;
            self.grunties.extend(g);
        }
        self.grunty_tables = Some(tables);
        Ok(())
    }

    /// The Grunties `ccSetChibiGuso` placed.
    pub fn grunties(&self) -> &[grunty::Grunty] {
        &self.grunties
    }

    /// The Grunty of `code` (the row it was made from).
    pub fn grunty(&self, code: i32) -> Option<&grunty::Grunty> {
        self.grunties.iter().find(|g| g.code == code)
    }

    pub fn grunty_mut(&mut self, code: i32) -> Option<&mut grunty::Grunty> {
        self.grunties.iter_mut().find(|g| g.code == code)
    }

    /// What the Grunties asked for since the last call, with the Grunty's
    /// code: sounds, smoke and their growing up's effects, voices, chat
    /// lines. (Their cameras and Kite's place are carried out here.)
    pub fn take_grunty_events(&mut self) -> Vec<(i32, grunty::GruntyEvent)> {
        std::mem::take(&mut self.grunty_events)
    }

    /// `pgChatTbl[k]` (a grown Grunty's line over its head).
    pub fn grunty_chat_text(&self, k: usize) -> Vec<u8> {
        self.grunty_tables.as_ref().and_then(|t| t.chat.get(k).cloned()).unwrap_or_default()
    }

    /// `ccChar::EntryAffect(grunty, plw, cmd, a1, a2, 0)`: the Grunty's
    /// `affectFunc` (`dogAction`, `dogAction2`, `dogActionAdult`) at once,
    /// with `ccRand` for the grown ones' lines.
    pub fn grunty_affect(&mut self, code: i32, cmd: i16, a1: i16, a2: i16) {
        let Some(i) = self.grunties.iter().position(|g| g.code == code) else { return };
        let dirc = self.player.body.dirc;
        match &self.town_pcs {
            Some(t) => self.grunties[i].affect(cmd, a1, a2, &mut t.borrow_mut().mt, dirc),
            None => self.grunties[i].affect(cmd, a1, a2, &mut mt::Mt::init(0), dirc),
        }
        let events = std::mem::take(&mut self.grunties[i].events);
        self.apply_grunty_events(code, events);
    }

    /// A Grunty's events: the camera (`changeCamera`, `cameraSetPos` and
    /// `cameraSetView` on `camID`'s camera) and Kite's place carried out,
    /// the rest kept for the host.
    fn apply_grunty_events(&mut self, code: i32, events: Vec<grunty::GruntyEvent>) {
        use grunty::GruntyEvent as E;
        for e in events {
            match e {
                E::Camera(n) => self.camera.change_camera(n),
                E::CamPos(v) => self.camera.active_mut().pos = v,
                E::CamView(v) => self.camera.active_mut().view = v,
                E::Player { pos, dirc_z } => {
                    self.player.body.pos = pos;
                    self.player.body.dirc[2] = dirc_z;
                }
                E::Debug(_) => {}
                e => self.grunty_events.push((code, e)),
            }
        }
    }

    /// Dun Loireag's dogs.
    pub fn dogs(&self) -> &[dog::Dog] {
        &self.dogs
    }

    /// What the dogs asked for since the last call (a step's or a bark's
    /// sound, a running step's dust), with the dog's code.
    pub fn take_dog_events(&mut self) -> Vec<(i32, dog::DogEvent)> {
        std::mem::take(&mut self.dog_events)
    }

    /// The walking PCs.
    pub fn pcs(&self) -> &[rtownpc::RtownPc] {
        &self.pcs
    }

    /// What the walking PCs asked for since the last call (a chat line
    /// over one's head, one leaving through the gate), with the PC's code.
    pub fn take_pc_events(&mut self) -> Vec<(i32, rtownpc::PcEvent)> {
        std::mem::take(&mut self.pc_events)
    }

    /// Kite's `effTransfer` (false) and `effWarpTransfer` (true) asked
    /// for since the last call, taken out of the requests.
    pub fn take_kite_transfers(&mut self) -> Vec<bool> {
        let mut out = Vec::new();
        self.requests.retain(|r| match r {
            Request::Transfer => {
                out.push(false);
                false
            }
            Request::WarpTransfer => {
                out.push(true);
                false
            }
            _ => true,
        });
        out
    }

    /// What the Administrator's `sysopeAct` started since the last call:
    /// (`npcID`, what).
    pub fn take_merchant_events(&mut self) -> Vec<(i32, merchant::SysopEvent)> {
        std::mem::take(&mut self.merchant_events)
    }

    /// Kite's footsteps since the last call: `ccPlayer::CheckNote`'s
    /// `ccSeSetParamSPC(param, plw)` - the note's param, his position, the
    /// ground's attribute (which picks the step's sound).
    pub fn take_steps(&mut self) -> Vec<(u32, V4, u32)> {
        std::mem::take(&mut self.steps)
    }

    /// Kite's running steps since the last call, for `ccEffPawSmoke`: his
    /// feet, heading and speed.
    pub fn take_paw_smokes(&mut self) -> Vec<([V4; 2], F, F)> {
        std::mem::take(&mut self.paw_smokes)
    }

    /// The Chaos Gate's sounds since the last call (`ccSeOn3D(se, pos)`).
    pub fn take_gate_sounds(&mut self) -> Vec<(i32, V4)> {
        std::mem::take(&mut self.gate_sounds)
    }

    /// The party members' `effTransfer`s since the last call: (`charTbl`
    /// row, warp).
    pub fn take_member_transfers(&mut self) -> Vec<(i32, bool)> {
        std::mem::take(&mut self.party.transfers)
    }

    /// Where each character the town's effects may follow stands now, and
    /// its height (`ccChar` +0x40, its row's +0x18): Kite as 0, a party
    /// member as `1 << 24 | id`, a walking PC as `2 << 24 | code`, a
    /// merchant as `3 << 24 | npcID`.
    pub fn fx_chars(&self) -> Vec<(u32, V4, F)> {
        let mut out = vec![(0, self.player.body.pos, self.player.height)];
        for (id, w) in self.party.members() {
            let ch = &self.party.combat.scene.chars[w];
            out.push((1 << 24 | (id as u32 & 0xff_ffff), ch.pos, ch.base().height));
        }
        for pc in &self.pcs {
            out.push((2 << 24 | (entry::Npc::code(pc) as u32 & 0xff_ffff), pc.char.pos, pc.char.height));
        }
        for m in &self.merchants {
            out.push((3 << 24 | (m.id as u32 & 0xff_ffff), m.ch.pos, m.ch.height));
        }
        for g in &self.grunties {
            out.push((4 << 24 | (g.code as u32 & 0xff_ffff), g.ch.pos, g.ch.height));
        }
        out
    }

    /// newlib's `rand()` the town draws from, for the effects' draws.
    pub fn rand_state(&mut self) -> &mut Rand {
        &mut self.rand
    }

    /// The merchants `ccSetMerchant(0)` placed.
    pub fn merchants(&self) -> &[merchant::Merchant] {
        &self.merchants
    }

    /// The entry control's other NPCs.
    pub fn npcs(&self) -> &[Box<dyn entry::Npc>] {
        &self.npcs
    }

    /// Every NPC on the entry control's list, in its order: the merchants,
    /// the walking PCs, the rest.
    pub fn all_npcs(&self) -> impl Iterator<Item = &dyn entry::Npc> {
        let m = self.merchants.iter().map(|m| m as &dyn entry::Npc);
        m.chain(self.dogs.iter().map(|d| d as &dyn entry::Npc))
            .chain(self.grunties.iter().map(|g| g as &dyn entry::Npc))
            .chain(self.pcs.iter().map(|p| p as &dyn entry::Npc))
            .chain(self.npcs.iter().map(|n| n.as_ref()))
    }

    /// `ccEvent::GetNpc(code)`: the NPC whose base id is `code`.
    pub fn npc(&self, code: i32) -> Option<&dyn entry::Npc> {
        self.all_npcs().find(|n| n.code() == code)
    }

    pub fn npc_mut(&mut self, code: i32) -> Option<&mut dyn entry::Npc> {
        if let Some(m) = self.merchants.iter_mut().find(|m| m.code() == code) {
            return Some(m);
        }
        if let Some(d) = self.dogs.iter_mut().find(|d| d.code() == code) {
            return Some(d);
        }
        if let Some(g) = self.grunties.iter_mut().find(|g| g.code == code) {
            return Some(g);
        }
        if let Some(p) = self.pcs.iter_mut().find(|p| p.code() == code) {
            return Some(p);
        }
        self.npcs.iter_mut().find(|n| n.code() == code).map(|n| n.as_mut() as &mut dyn entry::Npc)
    }

    /// What the frames since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// The save as the field has left it (`cameraMode` is written by L2).
    pub fn state(&self) -> &SaveState {
        &self.save
    }

    /// The save, for what the field's menus write (items, tactics, the
    /// operate lock).
    pub fn state_mut(&mut self) -> &mut SaveState {
        &mut self.save
    }

    /// `ccSleepAllThread` / `ccWakeupAllThread` as the field's menus call
    /// them: asleep, no task steps (the command target, camera, player,
    /// party, entries and gate stand still) and the frame draws everyone
    /// where they stand, the town behind them.
    pub fn set_asleep(&mut self, on: bool) {
        self.asleep = on;
    }

    pub fn asleep(&self) -> bool {
        self.asleep
    }

    /// `cmndSortRoot`'s chain after the last frame's `ccSortCmnd`: (kind,
    /// code, `cmndDist`, `cmndDirc` in radians), nearest first; the
    /// target is [`World::command_target`], the one before it
    /// `targeting().prev`.
    pub fn command_sorted(&self) -> &[(entry::Kind, i32, F, F)] {
        &self.targeting.sorted
    }

    /// `ccThGameCtrl`'s state: `cmndTarget`, `cmndTargetPrev`,
    /// `cmndTargetPriNum`, whether a menu is open.
    pub fn targeting(&self) -> &talk::Targeting {
        &self.targeting
    }

    /// Where the character `kind`/`code` stands and its height (Kite is
    /// the party's code 0, the Chaos Gate the gimmick 16).
    pub fn char_place(&self, kind: entry::Kind, code: i32) -> Option<(V4, F)> {
        match kind {
            entry::Kind::Spc if code == 0 => Some((self.player.body.pos, self.player.height)),
            entry::Kind::Spc => {
                let h = self.party.actor(code).map_or(0, |a| a.ch.height);
                self.party.place(code).map(|(p, _)| (p, h))
            }
            entry::Kind::Npc => self.npc(code).map(|n| (n.char().pos, n.char().height)),
            entry::Kind::Gimmick if code == i32::from(gate::GIMMICK) => Some((self.gate.pos, gate::HEIGHT)),
            _ => None,
        }
    }

    /// `ccCalcTagPosChar(c, out, (0, 0, height * height_scale), mode)`
    /// (gcmn 0x0051a500): the screen point over the character in the
    /// frame buffer's pixels (0-511, 0-447) - `sceVu0RotTransPers` through
    /// the field's `world_screen`, less the GS offset (1792, 1824) - and
    /// the call's answer: with `mode` 0, 1 on screen (-127..640, -31..480)
    /// else 0; otherwise 1 inside (-19..532, -15..464) else 2. None when
    /// the character is unknown or behind the camera (`ccCalcTagPos` 0).
    /// The target cursor uses a scale of 0.45, the life bar 0.9.
    pub fn tag_pos(&self, kind: entry::Kind, code: i32, height_scale: f32, mode: i32) -> Option<(i32, i32, i32)> {
        let (pos, height) = self.char_place(kind, code)?;
        let offset = [0, 0, ee::mul(height, height_scale.to_bits()), 0];
        char::calc_tag_pos(&self.camera.world_screen, pos, offset, mode)
    }

    /// What `ccChatMsg::Disp` asks of a speaker in the town: whether it is
    /// on a command list (`ccCheckTarget`: the party and the gate always; a
    /// walking PC while it is listed), and `ccCalcTagPosChar` at 0.9 of its
    /// height, mode 0, when it passes.
    pub fn chat_point(&self, kind: entry::Kind, code: i32) -> Option<(bool, Option<(i32, i32)>)> {
        let listed = match kind {
            entry::Kind::Npc => self.npc(code)?.listed(),
            _ => self.char_place(kind, code).is_some(),
        };
        let at = self.tag_pos(kind, code, 0.9, 0).filter(|t| t.2 != 0).map(|t| (t.0, t.1));
        Some((listed, at))
    }

    /// What the field's HUD shows of a character: its name (the party's
    /// from `charTbl`, an NPC's from `npcTbl`), base type flags, height,
    /// and for a party member where its `ccSpcParam` is in the save
    /// (`saveData.spcParam[code]`: level, HP, SP...).
    pub fn char_info(&self, kind: entry::Kind, code: i32) -> Option<CharInfo> {
        let (_, height) = self.char_place(kind, code)?;
        let spc = |code: i32| piney_data::save::by_id::spc_param(code as usize);
        let (name, flags, spc_param) = match kind {
            entry::Kind::Spc => {
                let name = self.char_names.get(code as usize).cloned().unwrap_or_default();
                (name, self.save.save.i32(spc(code) + 8) as u32, Some(spc(code)))
            }
            // A Grunty's base as it has grown.
            entry::Kind::Npc if self.grunty(code).is_some() => {
                let row = &self.grunty(code)?.row;
                (row.name.clone().into_bytes(), row.flags, None)
            }
            entry::Kind::Npc => {
                let row = npc::NpcRow::of(self.volume, code as usize).ok()?;
                (row.name.into_bytes(), row.flags, None)
            }
            entry::Kind::Gimmick => (b"Chaos Gate".to_vec(), talk::flags::CHAOS_GATE, None),
            entry::Kind::Enemy => return None,
        };
        Some(CharInfo { kind, code, name, flags, height, spc_param })
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// `cameraShake(power, cycle, time, dirc)` (main 0x00162cd0) from the
    /// menus' noise: a turning shake (`dirc` 2) draws `rand()`.
    pub fn camera_shake(&mut self, s: [i32; 4]) {
        let rand = &mut self.rand;
        self.camera.shake.shake(s[0], s[1], s[2], s[3], &mut || rand.rand());
    }

    pub fn town(&self) -> &Town {
        &self.town
    }

    pub fn town_mut(&mut self) -> &mut Town {
        &mut self.town
    }

    /// The `ccEff`s the town's last draw asked for (Dun Loireag's lens
    /// flare and clouds), for the host's effects to draw after the frame.
    /// The party members' chat balloons since the last call: (`charTbl`
    /// row, text), each `ccChatMsg::OpenChat` over the member.
    pub fn take_party_chats(&mut self) -> Vec<(i32, Vec<u8>)> {
        std::mem::take(&mut self.party.chats)
    }

    /// `ccSPC::ChangeEquip(id, cat)` (gcmn 0x0059f9e0) after the
    /// Equipment menu wrote the save's record: a weapon (categories 0-5)
    /// hangs the record's weapon on the character's hands, Kite's or a
    /// member's; the armour shows nothing.
    pub fn change_equip(&mut self, id: i32, cat: i32) {
        if !(0..=5).contains(&cat) {
            return;
        }
        if id != 0 {
            self.party.change_weapon(&self.archive, &self.save.save, id);
        } else if let Some((w, _)) = crate::body::weapon_of(&self.party.combat.data.t, &self.save.save, 0) {
            self.kite.arm(&self.archive, &w);
        }
    }

    /// Kite's model files, his blades on his hands.
    pub fn kite(&self) -> &Kite {
        &self.kite
    }

    pub fn take_town_sprites(&mut self) -> Vec<town::TownSprite> {
        std::mem::take(&mut self.town_sprites)
    }

    /// The effects and sounds the town's draws started since the last
    /// call ([`town::TownEvent`]).
    pub fn take_town_events(&mut self) -> Vec<town::TownEvent> {
        self.town.base.take_events()
    }

    /// `ccSys.bgColor` as the town's constructor set it (None: black).
    pub fn clear_colour(&self) -> Option<[u8; 3]> {
        self.town.base.clear
    }

    /// The disc's volume: whose tables the world reads.
    pub fn volume(&self) -> piney_data::volume::Volume {
        self.volume
    }

    pub fn gate(&self) -> &Gate {
        &self.gate
    }

    /// One game frame.
    pub fn step(&mut self, pad: &Pad) -> Frame {
        let mut ctx = Ctx::new(View::default());
        self.step_into(pad, &mut ctx);
        ctx.finish()
    }

    /// [`World::step`] into a frame the caller owns, so that it can draw on
    /// other layers of it (the field's HUD) before finishing it; the fades
    /// are on [`draw::FADE_LAYER`].
    pub fn step_into(&mut self, pad: &Pad, ctx: &mut Ctx) {
        match self.phase {
            Phase::FadeOut(k) => {
                piney_desktop::fade::draw_on(ctx, draw::FADE_LAYER, 0, 0x8000_0000, k, FADE_FRAMES);
                self.phase = if k + 1 < FADE_FRAMES { Phase::FadeOut(k + 1) } else { Phase::Hold(0) };
                if k + 1 == FADE_FRAMES {
                    self.requests.push(Request::AllSoundOff);
                }
            }
            Phase::Hold(k) => {
                piney_desktop::fade::draw_on(ctx, draw::FADE_LAYER, 0, 0x8000_0000, FADE_FRAMES, FADE_FRAMES);
                if k + 1 < HOLD_FRAMES {
                    self.phase = Phase::Hold(k + 1);
                } else if self.loading {
                    // ccEnableThEvent's waits: still the last held frame.
                } else {
                    self.requests.push(Request::SqLoad(2));
                    self.requests.push(Request::EnableReset(true));
                    self.requests.push(Request::GameStart);
                    self.phase = Phase::Play(0);
                }
            }
            Phase::Play(f) => {
                // F0: the new tasks set themselves up (the camera placed
                // behind the start; ccEntryEventMng's merchants and walking
                // PCs); from F1 they run.
                if f == 0
                    && let Err(e) = self.place_entries()
                {
                    eprintln!("the town's entries: {e}");
                }
                if f >= 1 {
                    self.frame(pad, ctx, f >= 2);
                }
                if f < FADE_FRAMES {
                    piney_desktop::fade::draw_on(ctx, draw::FADE_LAYER, 0x8000_0000, 0, f, FADE_FRAMES);
                } else if f == FADE_FRAMES {
                    self.requests.push(Request::BgmCtrl);
                }
                self.phase = Phase::Play(f.saturating_add(1));
            }
        }
    }

    // --- the event camera (evcam.rs) ------------------------------------------

    /// A camera instruction of the event scripts (`Host::camera`, at play
    /// level): `ccEvent::Execute`'s handler, which starts the event
    /// camera's task (it first runs in this frame's tasks, after
    /// `ccThGameCtrl`) or, for `camera_end` and `camera_end_reset`, stops
    /// it and gives the field camera back. Every [`CameraCommand`] is
    /// handled.
    pub fn event_camera(&mut self, c: piney_event::host::CameraCommand) -> bool {
        let mut ev = std::mem::take(&mut self.evcam);
        let mut cam = self.camera.clone();
        ev.command(c, &mut cam, self);
        self.evcam = ev;
        self.camera = cam;
        true
    }

    /// `menu_ban`'s (true) and `menu_clear`'s (false) camera part:
    /// `puppetShow` (L2 does nothing while set) and, on `menu_ban`,
    /// `normalCamID` (what `camera_end_reset` returns to).
    pub fn menu_ban_camera(&mut self, on: bool) {
        self.evcam.menu_ban(on, &mut self.camera);
    }

    /// `teach_camera1..3`'s camera part (part 1-3): the event camera turned,
    /// zoomed or reset by the pad (cpCtrl 5-7), its task started.
    pub fn teach_camera(&mut self, part: u8) {
        self.evcam.teach(part);
    }

    /// The event camera's state (`eventMng.cam`, the task, `normalCamID`;
    /// `puppetShow` is [`Camera::puppet_show`]).
    pub fn event_cam(&self) -> &evcam::EventCam {
        &self.evcam
    }

    /// The event camera the last area left ([`evcam::EventCam::next_area`]):
    /// `eventMng` outlives the area, its camera state with it.
    pub fn set_event_cam(&mut self, ev: evcam::EventCam) {
        self.evcam = ev;
    }

    /// `ccThCameraExecute` (priority 33, after `ccThGameCtrl`): its first
    /// frame `changeCamera(3)`, then each frame `ccEvent::CamCtrl`.
    fn event_camera_task(&mut self, pad: &CamPad) {
        if !self.evcam.running() {
            return;
        }
        // saveData.assignPAD's zoom buttons (a save with none set takes the
        // new game's R1 and R2).
        let button = |at: usize, default: u16| match self.save.save.i16(at) as u16 {
            0 => default,
            b => b,
        };
        let input = evcam::Input {
            pad: *pad,
            zoom: [button(offset::ASSIGN_PAD_ACTION + 0xe, 0x8), button(offset::ASSIGN_PAD_ACTION + 0x10, 0x2)],
        };
        let mut ev = std::mem::take(&mut self.evcam);
        let mut cam = self.camera.clone();
        ev.frame(&mut cam, self, &input);
        self.evcam = ev;
        self.camera = cam;
    }

    /// `ccThGameCtrl` (33), before the camera and the player: the command
    /// target from where everyone stood last frame, and the action button.
    fn game_ctrl(&mut self, pad: &Pad) {
        let p = &self.player;
        // Everyone's posP is W2PPos of where they stood at the end of last
        // frame: relative to Kite, him at (0, 0, z).
        let leader = talk::Leader { pos_p: char::w2p(p.body.pos, p.body.pos), dirc: p.body.dirc, width: p.width };
        // ccSortCmnd's order: the party's list, the enemies', everyone
        // else's.
        // The party's list in its order; each member's posP as its last
        // frame left it (ccFellow::Main's W2PPos).
        let c = &self.party.combat;
        let mut cands: Vec<talk::Cmnd> = c
            .scene
            .pc_list
            .iter()
            .filter_map(|&w| c.members.iter().find(|m| m.1 == w && m.0 != 0).copied())
            .map(|(id, w)| {
                let ch = &c.scene.chars[w];
                let width = c.cast.get(w).map_or(0, |a| a.ch.width);
                talk::Cmnd::new(entry::Kind::Spc, id, ch.ty() as u32, width, ch.pos_p)
            })
            .collect();
        cands.extend(self.all_npcs().filter(|n| n.listed()).map(|n| {
            let c = n.char();
            talk::Cmnd::new(entry::Kind::Npc, n.code(), n.flags(), c.width, c.pos_p)
        }));
        // The gate's posP is ccTransPosW2P of its pos (ccChgate::main
        // 0x004592a0), from where Kite stood at the end of last frame.
        if self.gate.frame.near {
            cands.push(talk::Cmnd::new(
                entry::Kind::Gimmick,
                i32::from(gate::GIMMICK),
                talk::flags::CHAOS_GATE,
                gate::WIDTH,
                char::w2p(self.gate.pos, p.body.pos),
            ));
        }
        // saveData.assignPAD: action, personal menu, chat, option (a save
        // with none set takes the new game's).
        let pushed = |at: usize, default: u16| {
            let b = self.save.save.i16(at) as u16;
            let b = if b == 0 { default } else { b };
            pad.push.bits() & u32::from(b) != 0
        };
        use piney_input::Buttons as B;
        let m = self.menu_view;
        let input = talk::Input {
            pow_l: pad.pow_l,
            eye: self.camera.active().kind == camera::kind::EYE,
            action: pushed(offset::ASSIGN_PAD_ACTION, DEFAULT_ACTION),
            personal: pushed(offset::ASSIGN_PAD_ACTION + 2, B::TRIANGLE.bits() as u16),
            chat: pushed(offset::ASSIGN_PAD_ACTION + 4, B::SQUARE.bits() as u16),
            option: pushed(offset::ASSIGN_PAD_ACTION + 6, B::START.bits() as u16),
            // ccPlayerMenuCheck: no skill runs in a town; acts 12 and 13
            // hold the menus.
            player_ok: !matches!(p.acts.act, 12 | 13),
            menu_idle: m.idle,
            forbid: m.forbid,
            forbid_chat_except: m.forbid_chat_except,
            pl_attack: m.pl_attack,
            held: false,
            dead: false,
            skill_one: false,
            area: 0,
            field: 0,
            dungeon_type: 0,
        };
        let save = &mut self.save;
        let step = self.targeting.step(&leader, &mut cands, &input, &mut |n| save.check_operate(n));
        let a = match step {
            None => return,
            Some(talk::Step::Open(menu)) => return self.talks.push(talk::TalkRequest::Open { menu }),
            Some(talk::Step::ClearAttack) => return self.talks.push(talk::TalkRequest::ClearAttack),
            Some(talk::Step::Action(a)) => a,
        };
        // The menu holds him (+0xe0 bit 0, pauseSW) until it closes.
        self.player.acts.pause = true;
        // ccEvent::CheckOperate(9)'s test: a target's type is a bit of the
        // character's base type flags (`cmndTarget->base->type`), so the
        // Chaos Gate (type 13, a gimmick) can be one as well as a PC or an
        // NPC.
        let event =
            self.event_targets.iter().any(|&(t, c)| a.flags & (1u32 << (t & 31)) != 0 && i32::from(c) == a.code);
        let req = if event {
            talk::TalkRequest::Event { kind: a.kind, code: a.code }
        } else if let (22, Some(n)) = (a.menu, self.npc(a.code)) {
            let (msg, line) = match n.talk_line() {
                Some((m, l)) => (m, Some(l)),
                None => (npc::NpcRow::of(self.volume, a.code as usize).map(|r| r.msg).unwrap_or(0), None),
            };
            talk::TalkRequest::Talk { npc: a.code, msg, line }
        } else if let (24..=26, Some(shop)) = (a.menu, talk::Shop::of(a.flags)) {
            let (msg, line) = self.npc(a.code).and_then(|n| n.talk_line()).unwrap_or((0, 0));
            talk::TalkRequest::Shop { npc: a.code, shop, msg, line }
        } else {
            talk::TalkRequest::Menu { menu: a.menu, kind: a.kind, code: a.code }
        };
        // The PC and shop menus themselves send EntryAffect(target, plw,
        // 14) as their list opens, and 0 as it closes.
        self.talks.push(req);
    }

    /// The tasks' frame: game control, camera, player, entries, town.
    fn frame(&mut self, pad: &Pad, ctx: &mut Ctx, town: bool) {
        // Asleep (ccSleepAllThread), nothing steps and everyone is drawn
        // where the last frame left them.
        let awake = !self.asleep;
        if awake {
            self.game_ctrl(pad);
            let cpad = CamPad::from_pad(pad);
            self.event_camera_task(&cpad);
            let mut mode = self.save.save.u8(offset::CAMERA_MODE) as i8;
            let anims = self.kite.anims();
            tasks(
                &mut self.camera,
                &mut self.player,
                &mut self.town.base.hits,
                &anims,
                &mut self.rand,
                &cpad,
                &mut mode,
            );
            self.save.save.set_u8(offset::CAMERA_MODE, mode as u8);
            for e in &self.player.events {
                match e {
                    motion::ActEvent::Transfer => self.requests.push(Request::Transfer),
                    motion::ActEvent::WarpTransfer => self.requests.push(Request::WarpTransfer),
                    motion::ActEvent::Step(param) => {
                        let p = &self.player;
                        self.steps.push((*param, p.body.pos, p.hit_attribute));
                        // CheckNote while running: ccEffPawSmoke(this, speed)
                        // at the lower of his feet.
                        if p.acts.act == motion::act::RUN
                            && let Some(feet) = self.kite.feet(p.acts.act, p.acts.posed, p.body.pos, p.body.dirc)
                        {
                            self.paw_smokes.push((feet, p.body.dirc[2], p.body.speed));
                        }
                    }
                    _ => {}
                }
            }
            // The arrival's end puts Kite's body on the character list
            // (bodyHit.HitEnable), after the merchants' and the walking PCs'.
            if self.player.events.contains(&motion::ActEvent::Arrived) {
                self.town.base.hits.hit_enable(&mut self.player.hit_body);
            }
            if let Some(how) = self.player.leave.take() {
                self.with_party(|spcs, chars, _| spcs.leave(0, how, chars));
            }
        }
        let to_screen = draw::screen(&self.camera.world_screen);
        // The draw environment's lights: the town's, then the gate's as its
        // last step left them.
        let lights = self.gate.lights(&self.town.base.lights);
        // WORLD_MAN::GO's shadow packets (a town's too).
        ctx.layers.shadows = draw::go_shadows(&self.town.base.lights, &self.camera.world_screen);
        if self.player.drawn {
            let p = &self.player;
            draw::char_shadow(&mut ctx.layers, p.shadow_t, p.height, |layers| {
                self.kite.draw(
                    layers,
                    to_screen,
                    p.acts.act,
                    p.acts.posed,
                    p.body.pos,
                    p.body.dirc,
                    ee::f(p.transparency),
                    &lights,
                    p.hit_attribute & 0x4_0000 != 0,
                    self.save.save.u8(offset::PLCOL) != 0,
                )
            });
        }
        // ccThFellow (50): the party members an event placed.
        if awake {
            self.party_frame();
        }
        self.party.draw(&mut ctx.layers, to_screen, &lights);
        // ccThEntryCtrl (64): its enemies, magic circles, gimmicks (the
        // Chaos Gate), then its NPC list - the merchants, then the rest;
        // all see the active camera (activeCamPtr).
        let t = self.camera.active();
        if awake {
            let f = self.gate.step(self.player.body.pos, t.pos, t.deg[1], t.kind == camera::kind::EYE);
            if let Some(se) = f.sound {
                self.gate_sounds.push((se, self.gate.pos));
            }
        }
        self.gate.draw(&mut ctx.layers, to_screen, &self.gate.lights(&self.town.base.lights));
        let view =
            char::View { player: self.player.body.pos, cam: t.pos, deg1: t.deg[1], eye: t.kind == camera::kind::EYE };
        let lights = self.gate.lights(&self.town.base.lights);
        let p = &self.player;
        let mut cx = entry::NpcCtx {
            player: p.body.pos,
            player_dirc: p.body.dirc,
            view,
            cam: self.camera.active(),
            hits: &mut self.town.base.hits,
            rand: &mut self.rand,
        };
        if awake {
            merchant::step_all(&mut self.merchants, &mut cx);
            // The Administrator's starts, and his deletion at act 5's end.
            for m in &mut self.merchants {
                let id = m.id;
                self.merchant_events.extend(m.sysop_events.drain(..).map(|e| (id, e)));
            }
            self.merchants.retain(|m| !m.gone);
        }
        for m in &self.merchants {
            m.draw(&mut ctx.layers, to_screen, &lights);
        }
        for d in &mut self.dogs {
            if awake {
                d.step(&mut cx);
                let code = d.code();
                self.dog_events.extend(d.events.iter().map(|e| (code, *e)));
            }
            d.ch.draw(&mut ctx.layers, to_screen, &lights);
        }
        // ccSetChibiGuso's Grunties: each one's frame, then inuCheckNote
        // on the notes its anms passed, on pgPtr (the last one made).
        let mut pg_events = Vec::new();
        let last = self.grunties.len().wrapping_sub(1);
        for i in 0..self.grunties.len() {
            if awake {
                let town = self.town_pcs.clone();
                let mut fallback = mt::Mt::init(0);
                let mut borrowed = town.as_ref().map(|t| t.borrow_mut());
                let mt = match borrowed.as_mut() {
                    Some(t) => &mut t.mt,
                    None => &mut fallback,
                };
                let mut gx = grunty::GruntyCtx {
                    player: cx.player,
                    player_dirc: cx.player_dirc,
                    view: cx.view,
                    hits: &mut *cx.hits,
                    rand: &mut *cx.rand,
                    mt,
                    save: &mut self.save.save,
                };
                let g = &mut self.grunties[i];
                g.step(&mut gx);
                let code = g.code;
                pg_events.push((code, std::mem::take(&mut g.events)));
                let notes = std::mem::take(&mut g.notes);
                let pg = &mut self.grunties[last];
                grunty::inu_check_note(&notes, pg, cx.rand);
                pg_events.push((pg.code, std::mem::take(&mut pg.events)));
            }
            self.grunties[i].draw(&mut ctx.layers, to_screen, &lights);
        }
        for pc in &mut self.pcs {
            if awake {
                pc.step(&mut cx);
                let code = pc.code();
                self.pc_events.extend(pc.events.iter().map(|e| (code, *e)));
            }
            pc.char.draw(&mut ctx.layers, to_screen, &lights);
        }
        for n in &mut self.npcs {
            if awake {
                n.step(&mut cx);
            }
            n.char().draw(&mut ctx.layers, to_screen, &lights);
        }
        // The Grunties' camera writes and Kite's place, after the list (the
        // game's land as each one's main runs; the others' fades this
        // frame see the camera before them).
        for (code, events) in pg_events {
            self.apply_grunty_events(code, events);
        }
        if town {
            // ROOTTOWN0n::Draw: cameraGetPos(camID), and for Dun Loireag's
            // clouds and lens flare the player, the active camera and
            // puppetShow.
            let a = self.camera.active();
            let v = town::TownView {
                eye: a.pos,
                player: self.player.body.pos,
                cam: a.clone(),
                flare: evarea::FlareCamera::of(&self.camera),
                puppet_show: self.camera.puppet_show,
                world_screen: self.camera.world_screen,
            };
            self.town_sprites = self.town.draw(&mut ctx.layers, to_screen, &v);
        }
    }
}

/// What the event camera looks up in the town: the characters by event type
/// and code ([`World::char_pos`]: Kite, the party members an event placed,
/// the NPCs), Kite as the party's leader, Mac Anu's markers.
impl evcam::Scene for World {
    fn char_pos(&self, ty: i32, code: i32) -> Option<V4> {
        World::char_pos(self, ty as i16, code as i16).map(|(p, _)| p)
    }
    fn leader_pos(&self) -> V4 {
        self.player.body.pos
    }
    fn marker_pos(&self, marker: i16) -> Option<V4> {
        self.marker_bits(marker).map(|(p, _)| p)
    }
    fn player_dirc(&self) -> V4 {
        self.player.body.dirc
    }
}
