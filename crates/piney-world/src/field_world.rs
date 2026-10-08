//! The World outside the towns: a field (area 1) and its dungeon (area 2), as
//! `ccSetupGameCtrl` (main 0x00168960) sets them up - the fade, the file
//! lists, the tasks, `WORLD_MAN::GO(1)`, the start positions,
//! `rebootSpcManager`, the fade in - and the field's tasks run them. Each
//! frame: `ccThGameCtrl` ([`talk::Targeting::frame`]), the event camera,
//! `cameraMain`, the battle's tasks ([`crate::combat`]) over the field's
//! collision, then the draw. The town is [`crate::World`]; each change of
//! scene ends one and makes the other (docs/engine/field-walk.md).

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_input::Pad;

use crate::area::{Scene, WorldMan, kind};
use crate::body::{Body, TRALL};
use crate::camera::{CamPad, Camera, Scheme};
use crate::chara::Kite;
use crate::combat::cast::load_looks;
use crate::combat::{self, BattleData, Combat, Look, Show};
use crate::dungeon_area::{DungeonArea, Exit};
use crate::ee::{self, F, ONE, V4};
use crate::evarea::{EventArea, Kept, StorySprite};
use crate::evarea_b0::Arena;
use crate::field_area::FieldArea;
use crate::hit::Hits;
use crate::player::Player;
use crate::story_map::StoryMap;
use crate::{FADE_FRAMES, FRAME_RATE, HOLD_FRAMES, Phase, draw, talk};

mod ride;

/// `WORLD_MAN.eventmap` as Kyvia's rules read an `EVENTAREAB8`: `IsMove()`,
/// `discPrevPos` and `DMY_marker01`'s place.
fn disc_view(a: &crate::evarea_b8::DiscArea) -> piney_battle::boss::DiscView {
    piney_battle::boss::DiscView { moving: a.is_move(), prev_pos: a.disc_prev, marker: a.disc_pos }
}

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// What the town's set-up asks too: the sound fade, sounds off, the
    /// sequence bank, the BGM, the soft reset, the arrival's effect.
    Game(crate::Request),
    /// `ccGame::ChangeRequest(6, 7)` after a change of scene
    /// ([`FieldWorld::scene`]): `ccSetupGameCtrl` again.
    ChangeScene,
    /// `ccSetupGameCtrl`'s `ccSndSQLoad` for the area (0x00168fac: 3 a
    /// field, 5 a story map's event bank, 4 a dungeon; nothing unless the
    /// scene changed or, in a dungeon, a special room), which the sound's
    /// own choice (`piney_audio::setup_context`) decides from the scene and
    /// `WORLD_MAN`.
    SqLoad,
    /// `ccSnd.gameStart = 1` (0x001694ec), before the fade in.
    GameStart,
    /// A story map's scene: the menus banned or back, or its message.
    Story(crate::story_map::StoryRequest),
}

/// The place the field's tasks run in.
pub enum Place {
    Field(Box<FieldArea>),
    Dungeon(Box<DungeonArea>),
    /// A story area's own map (`WORLD_MAN.eventmap`, [`crate::story_map`]).
    Story(Box<dyn StoryMap>),
}

impl Place {
    /// The story map, if it is a `T`.
    pub fn story<T: StoryMap>(&self) -> Option<&T> {
        match self {
            Place::Story(m) => m.as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    /// The story map, if it is a `T`, to change.
    pub fn story_mut<T: StoryMap>(&mut self) -> Option<&mut T> {
        match self {
            Place::Story(m) => m.as_any_mut().downcast_mut::<T>(),
            _ => None,
        }
    }
}

impl Place {
    fn hits(&mut self) -> &mut Hits {
        match self {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        }
    }

    fn lights(&self) -> &crate::town::TownLights {
        match self {
            Place::Field(f) => &f.lights,
            Place::Dungeon(d) => &d.lights,
            Place::Story(m) => m.lights(),
        }
    }
}

/// `WORLD_MAN::SetCharPosition` (main 0x001a1190) for a field made from
/// words, the party's leader: from a town (`game.areaPrev` not 2) the
/// field's start (`fieldStartPos`, facing 0); back from its dungeon beside
/// the entrance (`dungeonPos[0]`) by field type - type 10 (900, 0, 500) on
/// facing 1.5675, type 6 (0, 900, 500) facing pi, else (0, -900, 500)
/// facing 0. The position also becomes `WORLD_MAN.position` (+0x20).
pub fn field_start(area_prev: i32, field_type: u32, start: V4, dungeon: Option<V4>) -> (V4, F) {
    if area_prev == kind::DUNGEON
        && let Some(d) = dungeon
    {
        let at = |dx: F, dy: F| [ee::add(dx, d[0]), ee::add(dy, d[1]), ee::add(0x43fa_0000, d[2]), ONE];
        return match field_type {
            10 => (at(0x4461_0000, 0), 0x3fc8_f5c3),
            6 => ([d[0], ee::add(0x4461_0000, d[1]), ee::add(0x43fa_0000, d[2]), ONE], ee::PI),
            _ => ([d[0], ee::sub(d[1], 0x4461_0000), ee::add(0x43fa_0000, d[2]), ONE], 0),
        };
    }
    ([start[0], start[1], start[2], ONE], 0)
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
    if area == kind::DUNGEON {
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

/// What the HUD projects of a battle character ([`FieldWorld::hud_geometry`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudGeometry {
    /// `ccCalcTagPosChar(c, p, (0, 0, 0.45 height), 0)`: the target
    /// cursor's point, when it passes.
    pub tag: Option<(i32, i32)>,
    /// `ccCalcTagPosChar(c, p, (0, 0, 160), 1)`'s answer and
    /// `ccCalcTagPos(pos, p, (0, 0, 0.9 height))`'s point: the life bar.
    pub bar_res: i32,
    pub bar: (i32, i32),
    /// The off-screen arrow: `(256 - 384 sin d, 256 - 384 cos d)`, `d` the
    /// heading from Kite less the camera's.
    pub arrow: (i32, i32),
    /// `ccCheckCameraDeg(pos, 0x1400)`.
    pub in_view: bool,
    /// `cmndDist` as the last `ccSortCmnd` left it.
    pub dist: F,
}

/// The field game outside the towns.
pub struct FieldWorld {
    save: SaveState,
    /// The disc's volume: whose tables the field reads.
    volume: piney_data::volume::Volume,
    scene: Scene,
    world_man: WorldMan,
    place: Place,
    /// `WORLD_MAN`'s other dungeons of the area and `lastRoom`, kept
    /// while the one being played is `place`'s.
    dungeons: crate::dungeon_area::Dungeons,
    kite: Kite,
    player: Player,
    camera: Camera,
    phase: Phase,
    requests: Vec<Request>,
    targeting: talk::Targeting,
    talks: Vec<talk::TalkRequest>,
    menu_view: talk::MenuView,
    asleep: bool,
    archive: Arc<Archive>,
    /// `ccSpcManager` and `ccPartyManager`, carried from the area before.
    spcs: crate::party::Spcs,
    /// The fights: Kite, the party members `rebootSpcManager` built, the
    /// entry control's enemies and portals (piney-battle's scene).
    combat: Combat,
    /// The event's entries `ccEntryEventMng` places when the entry control
    /// starts: `entry_mc`, `entry`, and the event positions.
    event_entries: (Vec<[i16; 4]>, Vec<[i16; 4]>, Vec<piney_battle::entry::EvPos>),
    /// `compulsionGameOver` (main 0x00378c74): a game over forced from
    /// outside the battle. Nothing the port runs sets it yet.
    pub compulsion_game_over: bool,
    /// `ccThGameCtrl` left its loop for the game over this frame (taken by
    /// the runtime, which runs `ccThGameOver`).
    game_over_signal: bool,
    /// `ccMenuCtrl::CheckMenuType()` as the field's menus showed it.
    menu_type: i32,
    /// `ccSys.count`.
    count: u32,
    /// `ccThGameCtrl` already run this frame ([`FieldWorld::step_game_ctrl`]).
    ctrl_done: bool,
    /// The field's effects (piney-effect, installed by the runtime above).
    fx: Box<dyn FieldFx>,
    /// The sounds and flashes a field's weather asked for since the
    /// runtime last took them ([`FieldWorld::take_ambient_calls`]).
    ambient_calls: Vec<crate::field_ambient::Op>,
    /// `spcConditionEffectFlag` (main 0x00378ce4): the condition tints
    /// pulse. `ccThSpc`'s start turns it on (`ccSpcConditionEffectON`);
    /// `menu_ban`, the spring's hold and the events' `condition_fx_off`
    /// turn it off, `menu_clear`, the spring's release and
    /// `condition_fx_on` on again.
    condition_fx: bool,
    /// `ccThGameCtrl` set `ccMenu.plAttack` (the action button's attack)
    /// since the menus last looked.
    pl_attack_set: bool,
    /// `eventMng.target[]` (type, code): characters whose talk goes to the
    /// event ([`FieldWorld::set_event_targets`]).
    event_targets: Vec<(i16, i16)>,
    /// `charTbl`'s names and files (DEMO.PRG), by row.
    char_names: Vec<Vec<u8>>,
    char_files: Vec<String>,
    /// Where `ccGetStartPositions` stands the party: the leader and his
    /// facing, then `StartPos[1..3]`.
    start: (V4, F, [V4; 3]),
    /// Held on the set-up's last black frame while the event passes run.
    loading: bool,
    /// Whether `rebootSpcManager` has run.
    rebooted: bool,
    /// The event's `entry`s of party characters (types 0-2), registered by
    /// [`FieldWorld::set_spc_entries`] and placed after `Reboot`.
    spc_entries: Vec<[i16; 4]>,
    /// The event camera (`eventMng.cam`, `ccThCameraExecute`).
    evcam: crate::evcam::EventCam,
    /// From Mutation on, `ccSnd` +0x139: the skill words silent while a
    /// Grunty is out (set by its flute, cleared as the ride ends; a new
    /// scene's `ccFileListLoad` clears it too, which a new field world is).
    voices_off: bool,
    /// `SetPathFindingMap` due before the next battle frame (`ccThSpc`'s
    /// start; `DUNGEON::Draw` after a room change).
    path_map: bool,
    /// The story map's message closed this frame, for its scene.
    story_message_closed: bool,
    /// `gtHackFlag` (main 0x00378a98): the gate hack's `ccSetGtHack`,
    /// carried from the town ([`FieldWorld::set_gate_hack`]) and kept or
    /// cleared by `ccClearGtHack` ([`FieldWorld::clear_gt_hack`]).
    gt_hack: bool,
    /// `ccGame.setupMode` (+0x50): the set-up plays the Chaos Gate's
    /// movie while the field loads.
    setup_mode: bool,
    gate_stream: GateStream,
    /// The event's NPCs' classes ([`crate::field_npcs`]).
    npcs: crate::field_npcs::FieldNpcs,
    /// The event NPCs' starts of the last frame, for the next frame's
    /// effects (the entry control's shows come before them).
    npc_shows: Vec<Show>,
    /// `ccMenu.interNoiz = 2` asked by the Administrator since the last
    /// take ([`FieldWorld::take_noise`]).
    noise: bool,
    /// `ccSys+0x358` at the set-up's `ccInitRand` ([`crate::Arrival`]).
    rand_count: u32,
}

/// `ccSetupGameCtrl`'s stream 107 with `setupMode` set
/// (`ccThExecuteStream`, `ccRequestLoadStreamGateHack`): the set-up waits
/// for it before it builds the party.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum GateStream {
    #[default]
    Idle,
    /// Asked of the host ([`FieldWorld::take_gate_stream`]).
    Asked,
    Playing,
    Done,
}

/// What the host plays for [`FieldWorld::take_gate_stream`]:
/// `ccRequestLoadStreamGateHack(107, game.town, game.field)`, in the crisis
/// or not (`saveData` +0x6772).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateHackStream {
    pub town: i32,
    pub field: i32,
    pub crisis: bool,
}

impl FieldWorld {
    /// `ccSetupGameCtrl` for `scene` (area 1: a field of `world_man`'s
    /// words), reading the field and Kite from the disc. `at.faded`: the
    /// scene that asked for it has already faded out (its own tasks ran
    /// under the fade), so the set-up starts with the frames held black;
    /// `at.frames` gives `ccInitRand` its count. `spcs`: the party the
    /// scene before left.
    #[allow(clippy::too_many_arguments)]
    pub fn enter(
        iso: &mut Iso,
        archive: Arc<Archive>,
        mut save: SaveState,
        scene: Scene,
        world_man: WorldMan,
        kept: Option<Kept>,
        at: crate::Arrival,
        spcs: crate::party::Spcs,
    ) -> piney_data::Result<FieldWorld> {
        let volume = iso.volume()?;
        let (char_names, char_files): (Vec<Vec<u8>>, Vec<String>) = crate::load_char_tbl(iso)?.into_iter().unzip();
        // ccDeleteAllThread: the last area's ccThSpc goes, and its delete
        // hook clears the registry's bootParams before this area's passes.
        let mut spcs = spcs;
        spcs.th_spc_delete();
        if save.save.u8(offset::NEW_GAME_FLAG) as i8 != 1 {
            save.save.set_u8(offset::NEW_GAME_FLAG, 1);
        }
        let mut dungeons = crate::dungeon_area::Dungeons::default();
        // GO(1): the field's story map where it has one (its
        // EVENTAREA_INFO.model set, or an arena), else a generated field.
        let def_se = crate::field_area::DEF_SE.get(world_man.field_type as usize).copied().unwrap_or(0xc000);
        let (kept_map, kept) = match kept {
            Some(Kept::Event(e)) => (Some(Kept::Event(e)), None),
            k => (None, k),
        };
        // ccInitRand, after the set-up's fade and hold: ccRand anew from
        // the frames since power-on, and ccRandS moved on as far. A story
        // map's constructor (EVENTAREAB8's rocks) draws from it first.
        let rand_count = at.rand_count();
        let mut cc = piney_battle::rand::init_rand(rand_count, &mut save.rand_s);
        let mut story = if scene.area == kind::FIELD {
            let at = crate::story_map::At {
                volume,
                field: scene.field,
                area_prev: scene.area_prev,
                field_prev: scene.field_prev,
                server: scene.server,
                save: &save.save,
            };
            crate::story_map::build(&archive, &at, world_man.field_model, kept_map, def_se, &mut cc)?
        } else {
            None
        };
        let (mut place, pos, rot) = match scene.area {
            kind::FIELD if story.is_some() => {
                let m = story.take().expect("a story map");
                let (pos, rot, _) = m.start_positions();
                (Place::Story(m), pos, rot)
            }
            kind::FIELD => {
                // WORLD_MAN::GO(1): the story area is game.field.
                let mut params = world_man.field_params();
                params.event = scene.field.max(0);
                let ft = params.field_type as usize;
                let def_se = crate::field_area::DEF_SE.get(ft).copied().unwrap_or(0xc000);
                let field = FieldArea::with_world(&archive, volume, params, def_se, world_man.hack, scene.server)?;
                let (pos, rot) =
                    field_start(scene.area_prev, params.field_type, field.start_pos(), field.dungeon_pos());
                (Place::Field(Box::new(field)), pos, rot)
            }
            kind::DUNGEON => {
                // WORLD_MAN::GO(2): the dungeon made on the way in is kept
                // (WORLD_MAN.dungeon[n]); a new one is made otherwise.
                if let Some(Kept::Dungeon(k)) = kept
                    && scene.area_prev == kind::DUNGEON
                {
                    dungeons = *k;
                }
                let n = scene.dungeon.clamp(0, 2) as usize;
                let d = match dungeons.slots[n].take() {
                    Some(mut d) => {
                        d.reenter(&scene)?;
                        // Back up from a field type 4 area's second dungeon.
                        if world_man.field_type == 4
                            && scene.dungeon_prev == 1
                            && scene.dungeon == 0
                            && scene.block == dungeons.last_room
                        {
                            d.come_back(dungeons.last_room);
                        }
                        d
                    }
                    None => Box::new(DungeonArea::new_banned(
                        &archive,
                        volume,
                        &world_man,
                        &scene,
                        crate::dungeon_area::bans_of(&save.save),
                    )?),
                };
                let (pos, rot) = d.start();
                (Place::Dungeon(d), pos, rot)
            }
            a => return Err(piney_data::Error::NotFound(format!("area {a}: not a field or a dungeon"))),
        };
        let data = Rc::new(BattleData::read(iso)?);
        // ccPlayer::ccPlayer's EquipWeapon: the blades the save has him hold.
        let kite = match crate::body::weapon_of(&data.t, &save.save, 0) {
            Some((w, _)) => Kite::read_armed(&archive, &w)?,
            None => Kite::read(&archive)?,
        };
        let mut combat = Combat::new(data.clone(), save.rand);
        combat.cc = cc;
        combat.rnds = save.rand_s;
        // The looks the entry control's enemies and portals may take: the
        // rows ccRegisterDifficultyEnemy registers (with their drained
        // forms) and the magic portal.
        let (ty, rank) = world_man.enemy_rank(scene.area, scene.field, scene.floor, world_man.field_attr());
        combat.cast.looks = load_looks(&archive, &data, scene.server, ty, rank, &[]);
        // EFF_xmagpat1's patNum, which the portal's sparks wrap at.
        combat.part_pats = combat.cast.looks.circle.as_ref().map_or(0, |c| c.pat_num);
        let spc = offset::SPC_PARAM;
        let f = |at: usize| save.save.i32(at) as u32;
        let (velocity, height, width) = (f(spc + 0xd4), f(spc + 0x18), f(spc + 0x1c));
        let dirc = [0, 0, rot, 0];
        let mut player = Player::new(pos, dirc, velocity, width, height);
        player.volume = volume;
        let starts = match &place {
            Place::Story(m) => m.start_positions().2,
            _ => party_starts(scene.area, pos, rot),
        };
        let scheme = Scheme::new(i32::from(save.save.u8(offset::CAM_TYPE) as i8));
        let mode = save.save.u8(offset::CAMERA_MODE) as i8;
        let mut camera = Camera::new(pos, dirc, mode, scheme);
        camera.volume = volume;
        // ccPlayer::ccPlayer: WORLD_MAN::SetCenter at his feet; he arrives
        // (act 13) from a town, and stands (act 2) with his body on the list
        // at once coming back from the dungeon or inside one.
        if let Place::Field(f) = &mut place {
            f.set_center(pos[0], pos[1]);
        }
        place.hits().center = [pos[0], pos[1]];
        let mut requests = Vec::new();
        let phase = if at.faded {
            Phase::Hold(0)
        } else {
            if scene.changed() {
                requests.push(Request::Game(crate::Request::SoundFadeOut));
            }
            requests.push(Request::Game(crate::Request::EnableReset(false)));
            Phase::FadeOut(0)
        };
        Ok(FieldWorld {
            save,
            volume,
            scene,
            world_man,
            place,
            dungeons,
            kite,
            player,
            camera,
            phase,
            requests,
            targeting: talk::Targeting::default(),
            talks: Vec::new(),
            menu_view: talk::MenuView::default(),
            asleep: false,
            archive,
            spcs,
            combat,
            event_entries: Default::default(),
            compulsion_game_over: false,
            game_over_signal: false,
            menu_type: -1,
            count: 0,
            ctrl_done: false,
            fx: Box::new(combat::NoFx),
            ambient_calls: Vec::new(),
            condition_fx: true,
            pl_attack_set: false,
            event_targets: Vec::new(),
            char_names,
            char_files,
            start: (pos, rot, starts),
            loading: false,
            rebooted: false,
            spc_entries: Vec::new(),
            evcam: crate::evcam::EventCam::new(),
            voices_off: false,
            path_map: true,
            story_message_closed: false,
            gt_hack: false,
            setup_mode: false,
            gate_stream: GateStream::Idle,
            npcs: Default::default(),
            npc_shows: Vec::new(),
            noise: false,
            rand_count,
        })
    }

    /// The gate hack as the town left it (`gtHackFlag`, `ccGame.setupMode`
    /// from `GtHackMenu`'s OK), before the set-up runs.
    pub fn set_gate_hack(&mut self, flag: bool, setup_mode: bool) {
        self.gt_hack = flag;
        self.setup_mode = setup_mode;
    }

    /// `gtHackFlag` now (the session carries it to the next scene).
    pub fn gt_hack(&self) -> bool {
        self.gt_hack
    }

    /// `ccGame.setupMode` now: 1 until the set-up's end.
    pub fn setup_mode(&self) -> bool {
        self.setup_mode
    }

    /// `ccClearGtHack` (main 0x001b77c0), from `ccStartThEvent`
    /// ([`gate_out::keeps_gt_hack`]).
    pub fn clear_gt_hack(&mut self) {
        let s = &self.scene;
        let ty = self.world_man.dungeon_type.get(s.dungeon.max(0) as usize).map_or(0, |&t| i32::from(t));
        if !combat::gate_out::keeps_gt_hack(s.area, s.area_prev, ty) {
            self.gt_hack = false;
        }
    }

    /// `ccGame.setupMode`'s movie, once the set-up is ready for it: what
    /// to play. The host plays it and says [`FieldWorld::gate_stream_done`].
    pub fn take_gate_stream(&mut self) -> Option<GateHackStream> {
        if self.gate_stream != GateStream::Asked {
            return None;
        }
        self.gate_stream = GateStream::Playing;
        Some(GateHackStream {
            town: self.scene.town,
            field: self.scene.field,
            crisis: self.save.save.u8(offset::CRISIS) != 0,
        })
    }

    /// The movie has ended (or could not play): the set-up goes on.
    pub fn gate_stream_done(&mut self) {
        self.gate_stream = GateStream::Done;
    }

    /// Whether the set-up is held for the gate hack's movie.
    pub fn gate_streaming(&self) -> bool {
        matches!(self.gate_stream, GateStream::Asked | GateStream::Playing)
    }

    /// A party member's affect tint (`affectColorFix`, `affectColorRate`,
    /// `affectColor`; the event instruction `piros_colour` sets Piros's),
    /// on its character in the fights; false when it has none here.
    pub fn set_affect_colour(&mut self, code: i32, fix: i16, rate: i16, colour: u32) -> bool {
        let Some(w) = self.combat.who(code) else { return false };
        let Some(a) = self.combat.cast.get_mut(w) else { return false };
        let c = &mut a.ch.affect;
        c.fix = fix;
        c.rate = rate;
        c.colour = colour;
        true
    }

    /// A party member's affect tint, when it has a character here.
    pub fn affect_of(&self, code: i32) -> Option<crate::char::AffectColour> {
        let w = self.combat.who(code)?;
        self.combat.cast.get(w).map(|a| a.ch.affect)
    }

    /// Whether a party member (`charTbl` row) has a character here
    /// (`ccSpcManager`'s registry, built).
    pub fn spc_loaded(&self, code: i32) -> bool {
        self.combat.who(code).is_some()
    }

    /// `ccCheckGtHackAnm()` (gcmn 0x0059cd60): `ghoFlag`, while a
    /// gate-hacked arrival's cutscene runs.
    pub fn gate_hack_anim(&self) -> bool {
        self.combat.cast.gate.flag()
    }

    /// How the party comes in, as `ccPlayer::ccPlayer`, `ccFellow::Initialize`,
    /// `ccAI::ccAI` and `ccAddRequestFileListSpc` read `game` and
    /// `gtHackFlag`; with a hacked arrival, the camera file it loads.
    fn entrance(&mut self) -> combat::Entrance {
        use crate::combat::gate_out;
        let s = &self.scene;
        let ft = self.world_man.field_type as i32;
        let chat = gate_out::arrival_chat(s.area, s.area_prev, ft, s.dungeon, self.gt_hack);
        let id = gate_out::hack_out_id(s.area, s.area_prev, ft, s.dungeon, s.field, self.gt_hack).unwrap_or(-1);
        let mut hacked = false;
        if let Some(name) = gate_out::ccs_name(id) {
            let a = &mut self.combat.cast.gate.anim;
            a.name = name;
            a.file = SceneFile::read(&self.archive, &format!("x{name}")).ok().map(Rc::new);
            hacked = gate_out::VOLUME_NUM < 4;
        }
        let member_transfer = combat::Entrance::member_transfer(s.area, s.area_prev, ft, s.dungeon, false);
        combat::Entrance { chat, hacked, member_transfer }
    }

    /// `rebootSpcManager` (`ccSPC::Reboot`, gcmn 0x005a00d0) at the end of the
    /// set-up: Kite built at the leader's place (act 13 from a town, act 2 back
    /// from the dungeon or inside one), each registered party member at its
    /// slot's `StartPos` facing as he does, each character outside the party at
    /// the origin, then `SetParty`, all as the battle's characters; then
    /// `ccEntryEventMng` puts the event's party entries at their positions
    /// ([`piney_battle::evparty::EvParty::place_entry`]).
    fn reboot(&mut self) {
        if self.rebooted {
            return;
        }
        self.rebooted = true;
        // The members' AIs start with partyStrategy as the last scene left it.
        self.combat.spc.party_strategy = self.spcs.party_strategy;
        let (pos, rot, starts) = self.start;
        // A registry word's partyFlag bits (3, signed).
        let party_flag = |w: i32| (((w & 7) << 5) as i8) >> 5;
        let boot = |spcs: &crate::party::Spcs, id: i32| {
            spcs.registry.iter().position(|r| r.id == id).map(|i| (i, spcs.registry[i]))
        };
        let (_slot0, r0) =
            boot(&self.spcs, 0).unwrap_or((0, crate::party::Registry { id: 0, party_flag: 1, boot_param: 0 }));
        // ccPlayer::ccPlayer (gcmn 0x00597d84): a lake area's first dungeon
        // (field type 4, which has no field) is its field, so Kite arrives
        // there from a town as on a field (act 13, the gate-in).
        let lake_entry = self.scene.area == kind::DUNGEON && self.world_man.field_type == 4 && self.scene.dungeon == 0;
        let arriving = if lake_entry {
            self.scene.area_prev == kind::TOWN
        } else {
            !(self.scene.area_prev == kind::DUNGEON
                || self.scene.area == kind::DUNGEON
                || (self.scene.area == kind::FIELD && self.scene.area_prev == kind::FIELD))
        };
        let area = self.scene.area;
        self.combat.entrance = self.entrance();
        // ccRestoreSpcCondition's test, for the constructors below.
        let es39 = self.save.save.u8(offset::EVENT_STATUS + 39) as i8;
        self.combat.restore = if crate::party::restores_condition(area, self.scene.area_prev, self.scene.field, es39) {
            self.spcs.store
        } else {
            [None; 18]
        };
        let body = Rc::new(self.kite.body.clone());
        let bracelet = self.save.save.u8(offset::PLCOL) != 0;
        let swaps = self.kite.swaps(bracelet);
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let save = &self.save.save;
        self.combat.add_kite(
            save,
            pos,
            rot,
            arriving,
            r0.boot_param,
            party_flag(r0.party_flag),
            body,
            swaps,
            hits,
            area,
        );
        for (slot, &id) in self.spcs.member_id.clone().iter().enumerate().skip(1) {
            if id <= 0 {
                continue;
            }
            let Some((list_num, r)) = boot(&self.spcs, id) else { continue };
            let Some(file) = self.char_files.get(id as usize).cloned() else { continue };
            let at = starts[slot - 1];
            match Body::read(&self.archive, &file, TRALL) {
                Ok(mut b) => {
                    // ccPlayer::ccPlayer's EquipWeapon.
                    if let Some((w, job)) = crate::body::weapon_of(&self.combat.data.t, &self.save.save, id) {
                        b.equip_weapon(&self.archive, &w, job);
                    }
                    let hits = match &mut self.place {
                        Place::Field(f) => &mut f.hits,
                        Place::Dungeon(d) => &mut d.hits,
                        Place::Story(m) => m.hits_mut(),
                    };
                    let save = &self.save.save;
                    self.combat.add_member(
                        save,
                        id,
                        at,
                        rot,
                        r.boot_param,
                        party_flag(r.party_flag),
                        Rc::new(b),
                        hits,
                        area,
                        list_num as i32,
                    );
                }
                Err(e) => tracing::warn!("party member {id}: {e}"),
            }
        }
        // The registered characters outside the party, at the origin (an
        // event's entry then puts them at its marker).
        let members = self.spcs.member_id;
        for (list_num, r) in self.spcs.registry.into_iter().enumerate() {
            if r.id <= 0 || members.contains(&r.id) || self.combat.who(r.id).is_some() {
                continue;
            }
            let Some(file) = self.char_files.get(r.id as usize).cloned() else { continue };
            match Body::read(&self.archive, &file, TRALL) {
                Ok(mut b) => {
                    if let Some((w, job)) = crate::body::weapon_of(&self.combat.data.t, &self.save.save, r.id) {
                        b.equip_weapon(&self.archive, &w, job);
                    }
                    let hits = match &mut self.place {
                        Place::Field(f) => &mut f.hits,
                        Place::Dungeon(d) => &mut d.hits,
                        Place::Story(m) => m.hits_mut(),
                    };
                    let save = &self.save.save;
                    self.combat.add_member(
                        save,
                        r.id,
                        [0, 0, 0, crate::ee::ONE],
                        0,
                        r.boot_param,
                        party_flag(r.party_flag),
                        Rc::new(b),
                        hits,
                        area,
                        list_num as i32,
                    );
                }
                Err(e) => tracing::warn!("registered character {}: {e}", r.id),
            }
        }
        // ccNavi::ccNavi in a dungeon: SetDungeonMapInfo (gcmn 0x00514890),
        // the window of the room the party arrives in (Get2DMapInfo).
        if let Place::Dungeon(d) = &self.place {
            let [x, y, s] = d.map_2d_info(self.scene.block);
            for &(_, w) in &self.combat.members {
                if let Some(a) = self.combat.crew.ais.get_mut(&w) {
                    (a.navi.map_x, a.navi.map_y, a.navi.map_s) = (x as i16, y as i16, s as i16);
                }
            }
        }
        self.spcs.set_party();
        self.combat.set_party(self.spcs.party());
        // ccEntryEventMng: the event's party characters at their markers.
        let kite = self.combat.kite.map_or(pos, |k| self.combat.scene.chars[k].pos);
        for [_, code, marker, param] in std::mem::take(&mut self.spc_entries) {
            self.ev_party(|p| p.place_entry(code, marker, param, kite));
        }
        self.sync_player();
    }

    /// The town player's view of Kite (what the field's HUD, the map, the
    /// event camera and the title read) from the battle's.
    fn sync_player(&mut self) {
        let c = &self.combat;
        let Some(k) = c.kite else { return };
        let ch = &c.scene.chars[k];
        let s = c.crew.spc.get(&k).copied().unwrap_or_default();
        let p = &mut self.player;
        p.body.pos = ch.pos;
        p.body.dirc = s.dirc;
        p.body.move_flag = s.move_flag;
        p.body.run_flag = s.run_flag;
        p.acts.act = ch.spc_char.act_num;
        p.hit_attribute = s.hit_attribute;
        p.transparency = s.transparency;
        p.set_transparency = s.set_transparency;
        p.dead = ch.cond[piney_battle::param::cond::DEAD];
        p.listed = c.scene.listed(k);
        p.drawn = c.cast.get(k).is_some_and(|a| a.drawn);
    }

    /// The fights.
    pub fn combat(&self) -> &Combat {
        &self.combat
    }

    /// The debug console's `heal`: Kite and the members at their full HP
    /// and SP (the dead left down). The members counted.
    pub fn heal_party(&mut self) -> usize {
        let mut n = 0;
        for &(_, w) in &self.combat.members {
            if let Some(ch) = self.combat.scene.chars.get_mut(w)
                && ch.hp > 0
            {
                ch.hp = ch.max_hp;
                ch.sp = ch.max_sp;
                n += 1;
            }
        }
        n
    }

    /// Not the game's: the console's god gets a fallen member up (affect 20,
    /// the game's revive: condition 5 and its 78 frames), his HP back. Kite
    /// too, by a member standing (a hit past his HP in one frame fells him
    /// before the god's heal, and a ghost has no body to stop him).
    pub fn revive_party(&mut self) {
        let kite = self.combat.kite;
        let members: Vec<usize> = self.combat.members.iter().map(|&(_, w)| w).collect();
        let down = |c: &Combat, w: usize| {
            c.scene
                .chars
                .get(w)
                .is_some_and(|ch| ch.hp <= 0 && !matches!(ch.cond[piney_battle::param::cond::DEAD], 0 | 5))
        };
        let fallen: Vec<usize> = members.iter().copied().filter(|&w| down(&self.combat, w)).collect();
        for w in fallen {
            let by = if Some(w) == kite {
                members.iter().copied().find(|&m| m != w && !down(&self.combat, m))
            } else {
                kite
            };
            self.entry_affect(w, by, 20, [0; 3]);
            if let Some(ch) = self.combat.scene.chars.get_mut(w) {
                ch.hp = ch.max_hp;
            }
        }
    }

    pub fn combat_mut(&mut self) -> &mut Combat {
        &mut self.combat
    }

    /// What the battle's tasks handed to presentation since the last call.
    pub fn take_shows(&mut self) -> Vec<Show> {
        self.combat.fx_seen = 0;
        std::mem::take(&mut self.combat.shows)
    }

    /// The field's effects, which the battle's frame runs at their places
    /// ([`combat::FxTasks`]); and draws after the characters.
    pub fn set_fx(&mut self, fx: Box<dyn FieldFx>) {
        self.fx = fx;
    }

    pub fn fx_mut(&mut self) -> &mut dyn FieldFx {
        &mut *self.fx
    }

    pub fn fx(&self) -> &dyn FieldFx {
        &*self.fx
    }

    /// The field's weather's calls since the last take, in order:
    /// `ccSeOn` (the thunder), `ccSeOn3D` (the steam) and the storm's
    /// `scFadeDef->EntryFlash` ([`crate::field_ambient::Op`]'s `Se`,
    /// `Se3d` and `Flash`).
    pub fn take_ambient_calls(&mut self) -> Vec<crate::field_ambient::Op> {
        std::mem::take(&mut self.ambient_calls)
    }

    /// The event's entries for `ccEntryEventMng` (`eventMng.entryMc[]`,
    /// `entry[]`, the `evPos`s), which the entry control places when the
    /// tasks start. The enemies their rows name are looked up for drawing.
    pub fn set_event_entries(
        &mut self,
        entries_mc: Vec<[i16; 4]>,
        entries: Vec<[i16; 4]>,
        positions: Vec<piney_battle::entry::EvPos>,
    ) {
        let rows: Vec<i32> = entries_mc
            .iter()
            .filter(|e| e[0] == 0)
            .chain(entries.iter().filter(|e| matches!(e[0], 5 | 6)))
            .map(|e| i32::from(e[1]))
            .collect();
        let s = self.scene;
        let (ty, rank) = self.world_man.enemy_rank(s.area, s.field, s.floor, self.world_man.field_attr());
        self.combat.cast.looks = load_looks(&self.archive, &self.combat.data, s.server, ty, rank, &rows);
        self.combat.part_pats = self.combat.cast.looks.circle.as_ref().map_or(0, |c| c.pat_num);
        self.event_entries = (entries_mc, entries, positions);
    }

    /// `ccRegisterEventMng` (main 0x001b6d70) as the area's files are
    /// listed: each event `entry` of a party character (types 0-2,
    /// `[type, code, marker, param]`) registered (`ccSPC::EntrySpc`) with
    /// `bootParam = param`, for `Reboot` to build and `ccEntryEventMng` to
    /// place.
    pub fn set_spc_entries(&mut self, entries: Vec<[i16; 4]>) {
        for e in &entries {
            let i = self.spcs.entry_spc(i32::from(e[1]));
            if i >= 0 {
                self.spcs.registry[i as usize].boot_param = i32::from(e[3]);
            }
        }
        self.spc_entries = entries;
    }

    /// `ccMenuCtrl::CheckMenuType()` for the battle's rules and targeting.
    pub fn set_menu_type(&mut self, t: i32) {
        self.menu_type = t;
    }

    /// `ccSpcManager` and `ccPartyManager`, for the next area.
    pub fn spcs(&self) -> &crate::party::Spcs {
        &self.spcs
    }

    /// The registry and the party, to change before the party is built
    /// (`rebootSpcManager`).
    pub fn spcs_mut(&mut self) -> &mut crate::party::Spcs {
        &mut self.spcs
    }

    /// Party members' `charTbl` rows by slot, -1 empty.
    pub fn party(&self) -> [i32; 3] {
        self.spcs.party()
    }

    /// `charTbl[id].name`.
    pub fn char_name(&self, id: i32) -> Vec<u8> {
        usize::try_from(id).ok().and_then(|i| self.char_names.get(i)).cloned().unwrap_or_default()
    }

    /// Hold the set-up on its last black frame (the event passes).
    pub fn set_loading(&mut self, on: bool) {
        self.loading = on;
    }

    /// Where a party character (type 2 and code, 0 Kite) or an event NPC
    /// (type 3 or 4 and its row) stands, as the events and the event camera
    /// look them up.
    /// `marker_pos`'s marker in a field (`ccEvent::Execute` case 144, INF
    /// main 0x001b1aa4): on an event map, `markerEvTbl[n]`'s dummy of the
    /// map's stream; a plain field's (`WORLD_MAN` +0x438) is not ported.
    /// A story map's door (`WORLD_MAN::Enter` on it swaps the block): the
    /// middle of the floor polygons whose attribute has the Enter bit
    /// (0x8_0000) in the block standing now, in world space.
    pub fn door(&self) -> Option<V4> {
        let Place::Story(m) = &self.place else { return None };
        let (mut sum, mut n) = ([0f32; 3], 0f32);
        for h in &m.hits().models {
            for p in h.polys.iter().filter(|p| p.att & 0x8_0000 != 0) {
                for v in &p.vp {
                    for k in 0..3 {
                        sum[k] += f32::from_bits(v[k]) + f32::from_bits(h.rm[3][k]);
                    }
                    n += 1.0;
                }
            }
        }
        (n > 0.0).then(|| [(sum[0] / n).to_bits(), (sum[1] / n).to_bits(), (sum[2] / n).to_bits(), ONE])
    }

    pub fn marker(&self, n: i16) -> Option<piney_event::host::Marker> {
        let Place::Story(m) = &self.place else { return None };
        let (p, r) = crate::event::marker_in(m.file()?, self.volume, n)?;
        Some(piney_event::host::Marker { pos: p.map(f32::from_bits), dirc: f32::from_bits(r) })
    }

    /// The event manager's 16 positions as the script has set them since
    /// the area's set-up (`set_pos`, `marker_pos`): what an NPC's put or
    /// walk to a marker reads (`evPos`, eventMng +0x1c0).
    pub fn set_event_positions(&mut self, positions: Vec<piney_battle::entry::EvPos>) {
        self.event_entries.2 = positions;
    }

    pub fn char_pos(&self, ty: i16, code: i16) -> Option<V4> {
        if matches!(ty, 3 | 4) {
            return self.npcs.by_code(i32::from(code)).map(|n| n.npc().char().pos);
        }
        if ty != 2 {
            return None;
        }
        let who = self.combat.who(i32::from(code))?;
        Some(self.combat.scene.chars[who].pos)
    }

    pub fn frame_rate(&self) -> u32 {
        FRAME_RATE
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// `ccGame`'s scene, as the frames have left it (a change of scene
    /// raises [`Request::ChangeScene`]).
    pub fn scene(&self) -> Scene {
        self.scene
    }

    /// Whether `ccThGameCtrl` signalled the game over this frame.
    pub fn take_game_over(&mut self) -> bool {
        std::mem::take(&mut self.game_over_signal)
    }

    /// `ccRand` and newlib's `rand`, for tasks the runtime runs itself.
    pub fn with_rands<R>(&mut self, f: impl FnOnce(&mut dyn FnMut() -> i32, &mut dyn FnMut() -> i32) -> R) -> R {
        use piney_battle::rand::Rng as _;
        let crate::combat::Combat { cc, rand, .. } = &mut self.combat;
        let mut c = || cc.rand();
        let mut r = || rand.rand();
        f(&mut c, &mut r)
    }

    /// `ccAddPlayTime`'s `gameCnt` clocks on the scene this area holds.
    pub fn add_play_time(&mut self, rate: i32) {
        self.scene.add_play_time(rate);
    }

    pub fn world_man(&self) -> &WorldMan {
        &self.world_man
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn place(&self) -> &Place {
        &self.place
    }

    pub fn place_mut(&mut self) -> &mut Place {
        &mut self.place
    }

    /// `WORLD_MAN.specialRoom` (+0x160): the dungeon's (0 in a story
    /// area's event room), -1 anywhere else.
    pub fn special_room(&self) -> i32 {
        match &self.place {
            Place::Dungeon(d) => d.special_room,
            _ => -1,
        }
    }

    /// The disc's volume: whose tables the field reads.
    pub fn volume(&self) -> piney_data::volume::Volume {
        self.volume
    }

    pub fn archive(&self) -> &Arc<Archive> {
        &self.archive
    }

    pub fn state(&self) -> &SaveState {
        &self.save
    }

    /// `ccSys+0x358` as the set-up's `ccInitRand` read it.
    pub fn rand_count(&self) -> u32 {
        self.rand_count
    }

    /// The state the next mode takes: the save, with `rand()` as the area
    /// left it.
    pub fn state_out(&self) -> SaveState {
        let mut out = SaveState { rand: self.combat.rand.0, rand_s: self.combat.rnds, ..self.save.clone() };
        crate::mt::put(&self.combat.cc, &mut out.cc);
        out
    }

    /// The game's `rand()` handed back by what played in this mode (a
    /// stream).
    pub fn set_rand(&mut self, rand: u64) {
        self.combat.rand.0 = rand;
    }

    pub fn state_mut(&mut self) -> &mut SaveState {
        &mut self.save
    }

    /// The save with `rand` and `cc` the game's generators as the area's
    /// tasks left them, for a task beside them that draws from them too
    /// (the menus); what `f` drew stays drawn.
    pub fn with_live_state<R>(&mut self, f: impl FnOnce(&mut SaveState) -> R) -> R {
        self.save.rand = self.combat.rand.0;
        crate::mt::put(&self.combat.cc, &mut self.save.cc);
        let r = f(&mut self.save);
        self.combat.rand.0 = self.save.rand;
        crate::mt::take(&mut self.combat.cc, &self.save.cc);
        r
    }

    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// A change of scene asked for this frame, not yet taken
    /// (`ChangeRequest` puts every task after the asker to sleep).
    pub fn scene_change_asked(&self) -> bool {
        self.requests.contains(&Request::ChangeScene)
    }

    pub fn take_talk(&mut self) -> Vec<talk::TalkRequest> {
        std::mem::take(&mut self.talks)
    }

    pub fn set_menu_view(&mut self, view: talk::MenuView) {
        self.menu_view = view;
    }

    pub fn targeting(&self) -> &talk::Targeting {
        &self.targeting
    }

    /// The field's menu closed: `ccThGameCtrl` idle, Kite free.
    pub fn close_menu(&mut self) {
        self.targeting.close_menu();
        self.set_pause(false);
    }

    pub fn set_asleep(&mut self, on: bool) {
        self.asleep = on;
    }

    /// `ccGame::ChangeArea(a, n)`: the scene changed; the runtime takes
    /// [`Request::ChangeScene`] and sets the next one up.
    pub fn change_area(&mut self, a: i32, n: i32) {
        self.scene.change_area(a, n, &mut self.save.save);
        self.requests.push(Request::ChangeScene);
    }

    /// `ccMenuCtrl::GateoutMenu` (gcmn 0x0053c8e0) once it is confirmed and
    /// its own 20-frame fade is over: `ccSPC::DeleteNoPartyMember`, then
    /// `ChangeArea(0, max(game.town, 0))` - back to the Root Town the party
    /// left from.
    pub fn gate_out(&mut self) {
        let town = self.scene.town.max(0);
        self.change_area(kind::TOWN, town);
    }

    /// `setCameraCtrlType(t)` (0x001611a0) from OPTION's Controller page:
    /// the field camera's scheme (A-1, A-2, B-1, B-2).
    pub fn set_camera_type(&mut self, t: i32) {
        self.camera.scheme = Scheme::new(t);
    }

    /// `WORLD_MAN::RoomSelect(floor, block)` (main 0x0019dca0; the events' `room`
    /// and `room_point`): in a dungeon the room built afresh and
    /// `WORLD_MAN.position` set in it ([`DungeonArea::room_select`]), then
    /// `ChangeScene(-2, -2, -2, -2, floor, block)`, made as a door's. Its
    /// `bgColor` and GS resets are not modelled (the fade covers them); elsewhere
    /// nothing happens. True when it asked for the change.
    pub fn room_select(&mut self, floor: i32, block: i32) -> bool {
        let (Ok(f), Ok(b)) = (usize::try_from(floor), usize::try_from(block)) else { return false };
        let clear = room_clear(&self.combat, floor, block);
        let Place::Dungeon(d) = &mut self.place else { return false };
        d.bans = crate::dungeon_area::bans_of(&self.save.save);
        d.room_select(f, b, clear);
        self.scene.change_scene(-2, -2, -2, -2, floor, block, &mut self.save.save);
        self.requests.push(Request::ChangeScene);
        true
    }

    /// One `WORLD_MAN::ShowMap` call ([`crate::map::show_map`]) at the
    /// player: a field's portals on its map at once, or one room of the
    /// dungeon's floor, each room's doors as `ccCheckActiveObject` finds
    /// it. True when done.
    pub fn show_map(&mut self) -> bool {
        let combat = &self.combat;
        let clear = |f: i32, b: i32| room_clear(combat, f, b);
        crate::map::show_map(&mut self.place, self.world_man.field_model, self.player.body.pos, &clear)
    }

    /// `WORLD_MAN::GoField` (main 0x0019e410; `TransFieldMenu`, menu 86):
    /// from a dungeon back to its field, `ChangeArea(1, eventAreaNumber)`,
    /// where `SetCharPosition` stands the party beside the entrance. Nothing
    /// in a field. For field type 4 (no field) from the second dungeon back
    /// to the first, at the room its stairs were taken from
    /// (`ChangeScene(2, -2, -2, 0, 0, lastRoom)`), and nothing in the first.
    /// True when it changed the scene.
    pub fn go_field(&mut self) -> bool {
        if !matches!(self.place, Place::Dungeon(_)) {
            return false;
        }
        if self.world_man.field_type == 4 {
            if self.scene.dungeon != 1 {
                return false;
            }
            // 0x0019e45c: the second dungeon's entryFlag cleared.
            if let Place::Dungeon(d) = &mut self.place {
                d.gimmicks_placed = false;
            }
            self.scene.change_scene(2, -2, -2, 0, 0, self.dungeons.last_room, &mut self.save.save);
            self.requests.push(Request::ChangeScene);
            return true;
        }
        self.change_area(kind::FIELD, self.scene.field.max(0));
        true
    }

    /// `WORLD_MAN::Enter(pos)` (main 0x0019dda0), which the player calls on
    /// ground with attribute bit 0x80000: in a field made from words the
    /// dungeon, `WORLD_MAN.position` cleared and `ChangeArea(2, 0)`; in a
    /// dungeon, `DUNGEON::GotoNextRoom`'s way on.
    fn enter_place(&mut self) {
        let combat = &self.combat;
        let clear = |f: i32, b: i32| room_clear(combat, f, b);
        match &mut self.place {
            Place::Field(_) => self.change_area(kind::DUNGEON, 0),
            // Area 15's door (a block), area 16's (its dungeon from block
            // 0, else back to block 0); the arenas' Enter does nothing.
            Place::Story(m) => match m.enter(self.scene.block, self.scene.area_prev) {
                Ok(crate::story_map::Enter::Stay) => {}
                Ok(crate::story_map::Enter::Dungeon) => self.change_area(kind::DUNGEON, 0),
                Ok(crate::story_map::Enter::Block(b)) => {
                    self.scene.change_scene(-2, -2, -2, -2, -2, b, &mut self.save.save);
                    self.requests.push(Request::ChangeScene);
                }
                Err(err) => tracing::warn!("EVENTAREA::ChangeBlock: {err}"),
            },
            Place::Dungeon(d) => match d.enter(
                self.player.body.pos,
                &self.scene,
                &clear,
                &mut self.save.save,
                &mut self.dungeons.last_room,
            ) {
                Some(Exit::Area { area, n }) => self.change_area(area, n),
                Some(Exit::Scene { area, town, field, dungeon, floor, block }) => {
                    self.scene.change_scene(area, town, field, dungeon, floor, block, &mut self.save.save);
                    self.requests.push(Request::ChangeScene);
                }
                None => {}
            },
        }
    }

    /// `ccThEntryCtrlDelete` (gcmn 0x00431d10) as a dungeon is left. For
    /// another room of the same dungeon (`CheckSceneReplace()` false) what
    /// survives it goes into `g_entryList`, which the dungeon carries to
    /// the next room's set-up (the magic portals, the boxes and idols of
    /// `EntryGimmick`, an event's enemies; see
    /// `piney_battle::entry::EntryCtrl::keep`). For another scene (a lake's
    /// other dungeon) every object is deleted and nothing is kept.
    fn keep_entries(&mut self) {
        if !matches!(self.place, Place::Dungeon(_)) || !self.combat.started {
            return;
        }
        let keep = !self.scene.changed();
        let info = self.task_info();
        let cpad = CamPad::default();
        let kept = {
            let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, cpad, &info);
            self.combat.with_entry_cx(&mut x, |c, cx| c.keep(cx, keep))
        };
        let actors = kept.olds.iter().flatten().map(|o| self.combat.cast.actors.remove(o)).collect();
        if let Place::Dungeon(d) = &mut self.place {
            d.kept_entries = Some(kept);
            d.kept_actors = actors;
        }
    }

    /// A gimmick of row `id` put at `pos` facing `dirc` in the room Kite
    /// is in, as `EntryObject` makes it (tests and tools; see
    /// [`combat::Combat::entry_gimmick`]): its scene index.
    pub fn entry_gimmick(&mut self, id: i32, pos: V4, dirc: V4) -> Option<usize> {
        let info = self.task_info();
        let cpad = CamPad::default();
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, cpad, &info);
        self.combat.entry_gimmick(&mut x, id, pos, dirc)
    }

    /// An enemy of `enemyTbl` row `row` put at `pos` facing `dirc` in the
    /// room Kite is in, as an event's `entry 5` makes one (`entRoot` 0),
    /// its look and its drained form's loaded when the area has none: its
    /// scene index. For tests and tools: the areas put their own.
    pub fn put_enemy(&mut self, row: i32, pos: V4, dirc: F) -> Option<usize> {
        if !self.combat.started {
            return None;
        }
        let mut files = crate::foe::Files::new(self.archive.clone());
        self.combat.cast.looks.add_enemies(&mut files, &self.combat.data, &[row]);
        let s = self.scene;
        let mut ep = piney_battle::entry::entry_param_clear();
        ep.pos = pos;
        ep.dirc = [0, 0, dirc, ONE];
        ep.id = row;
        ep.area = s.area;
        ep.area_num = match s.area {
            1 => s.field,
            2 => s.dungeon,
            _ => s.town,
        };
        ep.floor = s.floor;
        ep.block = s.block;
        ep.ent_root = 0;
        ep.param[2] = 0;
        let info = self.task_info();
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
        self.combat.entry_object(&mut x, &mut ep)
    }

    /// The dungeon, for the next scene of it (`WORLD_MAN.dungeon[n]` lives
    /// on); None in a field.
    pub fn into_dungeon(mut self) -> Option<Box<DungeonArea>> {
        self.keep_entries();
        match self.place {
            Place::Dungeon(d) => Some(d),
            Place::Field(_) | Place::Story(_) => None,
        }
    }

    /// What `WORLD_MAN` keeps for the next scene: the dungeon, or area 15's
    /// map (which the next set-up keeps only for a field).
    pub fn into_kept(mut self) -> Option<Kept> {
        self.keep_entries();
        match self.place {
            Place::Dungeon(d) => {
                let mut all = self.dungeons;
                let n = d.dungeon.clamp(0, 2) as usize;
                all.slots[n] = Some(d);
                Some(Kept::Dungeon(Box::new(all)))
            }
            // WORLD_MAN::Quit deletes any story map but area 15's.
            Place::Story(m) => m.into_any().downcast::<EventArea>().ok().map(Kept::Event),
            Place::Field(_) => None,
        }
    }

    /// One game frame.
    pub fn step(&mut self, pad: &Pad) -> Frame {
        let mut ctx = Ctx::new(View::default());
        self.step_into(pad, &mut ctx);
        let mut frame = ctx.finish();
        self.set_clear(&mut frame);
        frame
    }

    /// `ccSys.bgColor` (+0x18) as the area leaves it: a dungeon's fog
    /// colour (`DUNGEON::SetFog`); a field's sky covers the frame.
    pub fn set_clear(&self, frame: &mut Frame) {
        if let Place::Dungeon(d) = &self.place {
            let [r, g, b] = d.clear();
            frame.clear = piney_draw::Rgba([r, g, b, 0x80]);
        }
        if let Place::Story(m) = &self.place {
            let [r, g, b] = m.clear();
            frame.clear = piney_draw::Rgba([r, g, b, 0x80]);
        }
    }

    /// [`FieldWorld::step`] into a frame the caller finishes (the HUD draws
    /// on its own layers of it).
    pub fn step_into(&mut self, pad: &Pad, ctx: &mut Ctx) {
        match self.phase {
            Phase::FadeOut(k) => {
                piney_desktop::fade::draw_on(ctx, draw::FADE_LAYER, 0, 0x8000_0000, k, FADE_FRAMES);
                self.phase = if k + 1 < FADE_FRAMES { Phase::FadeOut(k + 1) } else { Phase::Hold(0) };
                if k + 1 == FADE_FRAMES && self.scene.changed() {
                    self.requests.push(Request::Game(crate::Request::AllSoundOff));
                }
            }
            Phase::Hold(k) => {
                piney_desktop::fade::draw_on(ctx, draw::FADE_LAYER, 0, 0x8000_0000, FADE_FRAMES, FADE_FRAMES);
                if self.tasks_start() {
                    // After a gate hack's stream (setupMode 0 at the
                    // set-up's end) its ccSndSQLoad came before it.
                    let streamed = self.gate_stream == GateStream::Done;
                    self.setup_mode = false;
                    // GO, ccGetStartPositions, rebootSpcManager.
                    self.reboot();
                    // ccSndSQLoad for the area, enableReset, then (with the
                    // tasks and the area set up) ccSnd.gameStart.
                    if !streamed {
                        self.requests.push(Request::SqLoad);
                    }
                    self.requests.push(Request::Game(crate::Request::EnableReset(true)));
                    self.requests.push(Request::GameStart);
                    self.phase = Phase::Play(0);
                } else if k + 1 < HOLD_FRAMES {
                    self.phase = Phase::Hold(k + 1);
                } else if !self.loading && self.gate_stream == GateStream::Idle {
                    // ccSndSQLoad, then (setupMode) stream 107 through
                    // ccRequestLoadStreamGateHack while the files load; the
                    // set-up goes on when it has ended.
                    self.requests.push(Request::SqLoad);
                    self.gate_stream = GateStream::Asked;
                }
            }
            Phase::Play(f) => {
                if f >= 1 {
                    self.frame(pad, ctx, f >= 2);
                } else if !self.asleep {
                    self.entry_setup(&CamPad::from_pad(pad));
                }
                if f < FADE_FRAMES {
                    piney_desktop::fade::draw_on(ctx, draw::FADE_LAYER, 0x8000_0000, 0, f, FADE_FRAMES);
                } else if f == FADE_FRAMES {
                    self.requests.push(Request::Game(crate::Request::BgmCtrl));
                }
                self.phase = Phase::Play(f.saturating_add(1));
            }
        }
    }

    /// Whether this frame's step ends the hold: the passes made (and a gate
    /// hack's stream played), `ccSetupGameCtrl` starts the field's tasks.
    /// `ccThMenu`'s first run, its new `ccMenuCtrl`, comes before the
    /// step's `WORLD_MAN::GO` and `rebootSpcManager`.
    pub fn tasks_start(&self) -> bool {
        matches!(self.phase, Phase::Hold(k) if k + 1 >= HOLD_FRAMES)
            && !self.loading
            && (!self.setup_mode || self.gate_stream == GateStream::Done)
    }

    /// `ccThGameCtrl` (33), the frame's first world task, run by itself so
    /// that the menu task (34) can follow it before the rest
    /// ([`FieldWorld::step_into`], `ccThCamera` 40 on).
    pub fn step_game_ctrl(&mut self, pad: &Pad) {
        self.targeting.map_test = false;
        if let Phase::Play(f) = self.phase
            && f >= 1
            && !self.asleep
        {
            self.count = self.count.wrapping_add(1);
            self.game_ctrl(pad);
            self.ctrl_done = true;
        }
    }

    /// This frame's `ccThGameCtrl` reached the map button's test
    /// ([`talk::Targeting::map_test`]).
    pub fn map_test(&self) -> bool {
        self.targeting.map_test
    }

    /// `ccThGameCtrl` (33): the command target over the battle's command
    /// lists (the party, the enemies, the objects: `ccSortCmnd`,
    /// `ccSelectTarget`) and the menu buttons.
    fn game_ctrl(&mut self, pad: &Pad) {
        use crate::entry::Kind;
        use piney_battle::param::cond;
        use piney_input::Buttons as B;
        let c = &self.combat;
        let Some(k) = c.kite else { return };
        let kc = &c.scene.chars[k];
        let dirc = c.kite_dirc();
        let leader = talk::Leader { volume: self.volume, pos_p: [0, 0, kc.pos[2], ONE], dirc, width: kc.base().width };
        // The three command lists in order, Kite apart: the party's by
        // charTbl row, the enemies' and the others' by scene index.
        let cmnd = |kind: Kind, code: i32, i: usize| {
            let ch = &c.scene.chars[i];
            let mut m = talk::Cmnd::new(kind, code, ch.ty() as u32, ch.base().width, ch.pos_p);
            m.id = ch.id();
            m.cond = talk::Cond::from_shorts(&ch.cond.v);
            m.act = ch.spc_char.act_num;
            m
        };
        let mut cands: Vec<talk::Cmnd> = Vec::new();
        let mut at = 0;
        for (n, &i) in c.scene.pc_list.iter().enumerate() {
            if i == k {
                at = n;
                continue;
            }
            cands.push(cmnd(Kind::Spc, i32::from(c.scene.chars[i].id()), i));
        }
        for &i in &c.scene.ene_list {
            cands.push(cmnd(Kind::Enemy, i as i32, i));
        }
        for &i in &c.scene.obj_list {
            cands.push(cmnd(Kind::Gimmick, i as i32, i));
        }
        let pushed = |at: usize, default: u16| {
            let b = self.save.save.i16(at) as u16;
            let b = if b == 0 { default } else { b };
            pad.push.bits() & u32::from(b) != 0
        };
        let m = self.menu_view;
        let skill = c.skills.borrow().check(&c.data.t, &c.scene, k, kc.spc_char.act_num);
        let kite_cond = talk::Cond::from_shorts(&kc.cond.v);
        let input = talk::Input {
            pow_l: pad.pow_l,
            eye: self.camera.tcam.kind == crate::camera::kind::EYE,
            action: pushed(offset::ASSIGN_PAD_ACTION, crate::DEFAULT_ACTION),
            personal: pushed(offset::ASSIGN_PAD_ACTION + 2, B::TRIANGLE.bits() as u16),
            chat: pushed(offset::ASSIGN_PAD_ACTION + 4, B::SQUARE.bits() as u16),
            option: pushed(offset::ASSIGN_PAD_ACTION + 6, B::START.bits() as u16),
            player_ok: piney_battle::kite::menu_check(&c.party, &c.scene, k, skill)
                && !matches!(kc.spc_char.act_num, 12 | 13),
            menu_idle: m.idle,
            forbid: m.forbid,
            forbid_chat_except: m.forbid_chat_except,
            pl_attack: m.pl_attack,
            held: kite_cond.held(),
            dead: kite_cond.dead != 0,
            skill_one: skill == 1,
            area: self.scene.area,
            field: self.scene.field,
            // `ccGame.dungeonType`: a lake's (8, 9) PERSONAL is the field's,
            // with Gate Out.
            dungeon_type: self
                .world_man
                .dungeon_type
                .get(self.scene.dungeon.max(0) as usize)
                .map_or(0, |&t| i32::from(t)),
        };
        // ccPartyManager: Kite, then the members, each found on the
        // candidates' list or off it.
        let mut party = talk::Party { slots: [None; 3], num: 0 };
        for (slot, &mc) in c.party.members.iter().enumerate() {
            let Some(mc) = mc else { continue };
            let ch = &c.scene.chars[mc];
            let who = if mc == k {
                talk::Who::Leader
            } else {
                let code = i32::from(ch.id());
                cands
                    .iter()
                    .position(|x| x.kind == Kind::Spc && x.code == code)
                    .map_or(talk::Who::Unlisted, talk::Who::Cand)
            };
            let pos_p = if mc == k { leader.pos_p } else { ch.pos_p };
            party.slots[slot] = Some(talk::Member { pos_p, dead: ch.cond[cond::DEAD], who });
        }
        party.num = c.party.num;
        let control = c.crew.ais.get(&k).is_some_and(|a| a.manual_sw);
        let battle = talk::Battle {
            kite: talk::LeaderState {
                listed: c.scene.pc_list.contains(&k),
                at,
                flags: kc.ty() as u32,
                id: kc.id(),
                cond: kite_cond,
                act: kc.spc_char.act_num,
                control,
            },
            party,
            menu: self.menu_type as i16,
            menu_closed: self.menu_type == -1,
            load_disp: false,
            gt_hack: self.combat.cast.gate.flag(),
            compulsion_game_over: self.compulsion_game_over,
        };
        let mut game = self.combat.battle;
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let mut host =
            CtrlHost { combat: &mut self.combat, save: &mut self.save, hits, area: self.scene.area, kite: k };
        let out = self.targeting.frame(&leader, &mut cands, &input, &battle, &mut game, &mut host);
        self.combat.battle = game;
        if let Some(p) = out.pause {
            self.set_pause(p);
        }
        if m.pl_attack && !out.pl_attack {
            self.talks.push(talk::TalkRequest::ClearAttack);
        }
        match out.step {
            None | Some(talk::Ctrl::Attack { .. }) => {}
            Some(talk::Ctrl::Open(menu)) => self.talks.push(talk::TalkRequest::Open { menu }),
            Some(talk::Ctrl::Action(a)) => {
                // ccEvent::CheckOperate(9)'s test (as the town's): a
                // target's type is a bit of the character's base type
                // flags, its code the base id; the event's then, no menu.
                // A party character's code is its charTbl row, not its
                // scene index.
                let id = self
                    .target_index(Some((a.kind, a.code)))
                    .and_then(|i| self.combat.scene.chars.get(i))
                    .map(|ch| ch.id());
                let event = self.event_targets.iter().any(|&(t, c)| a.flags & (1u32 << (t & 31)) != 0 && Some(c) == id);
                self.talks.push(if event {
                    talk::TalkRequest::Event { kind: a.kind, code: a.code }
                } else {
                    talk::TalkRequest::Menu { menu: a.menu, kind: a.kind, code: a.code }
                });
            }
            // The game over (field-walk.md): the task's first step here,
            // the rest the runtime's (piney-game's gameover.rs).
            Some(talk::Ctrl::GameOver) => {
                self.compulsion_game_over = true;
                self.game_over_signal = true;
                self.set_pause(true);
            }
        }
        self.pl_attack_set |= out.pl_attack && !m.pl_attack;
    }

    /// Kite's `pauseSW` (+0xe0 bit 0): a menu holds him.
    fn set_pause(&mut self, on: bool) {
        self.player.acts.pause = on;
        let Some(k) = self.combat.kite else { return };
        let f = &mut self.combat.scene.chars[k].spc_char.flags;
        if on {
            *f |= piney_battle::chara::spc_flag::PAUSE;
        } else {
            *f &= !piney_battle::chara::spc_flag::PAUSE;
        }
    }

    /// The scene index the command target names: a party member by its
    /// `charTbl` row, anything else by its index.
    pub fn target_index(&self, t: Option<(crate::entry::Kind, i32)>) -> Option<usize> {
        match t? {
            (crate::entry::Kind::Spc, id) => self.combat.who(id),
            (_, i) => usize::try_from(i).ok().filter(|&i| i < self.combat.scene.chars.len()),
        }
    }

    /// `eventMng.target[]` as the event interpreter holds it ((type,
    /// code), -1 free): talking to those characters runs the event
    /// ([`talk::TalkRequest::Event`]) rather than a menu.
    pub fn set_event_targets(&mut self, targets: &[(i16, i16)]) {
        self.event_targets = targets.iter().copied().filter(|(t, _)| *t >= 0).collect();
    }

    /// The command target (`cmndTarget`) as a scene index.
    pub fn command_target(&self) -> Option<usize> {
        self.target_index(self.targeting.target)
    }

    /// The command target (`cmndTarget`) by kind and code.
    pub fn command_target_code(&self) -> Option<(crate::entry::Kind, i32)> {
        self.targeting.target
    }

    /// The event targets as [`FieldWorld::set_event_targets`] left them.
    pub fn event_targets(&self) -> &[(i16, i16)] {
        &self.event_targets
    }

    /// The place's collision.
    pub fn place_hits(&self) -> &Hits {
        match &self.place {
            Place::Field(f) => &f.hits,
            Place::Dungeon(d) => &d.hits,
            Place::Story(m) => m.hits(),
        }
    }

    /// Whether `ccThGameCtrl` set `ccMenu.plAttack` since the last call.
    pub fn take_pl_attack(&mut self) -> bool {
        std::mem::take(&mut self.pl_attack_set)
    }

    /// `ccChangeCmndTarget` from the field's menus.
    pub fn change_command_target(&mut self, t: Option<(crate::entry::Kind, i32)>) {
        self.targeting.set_target(t);
    }

    /// `cmndTarget = cmndTargetPrev = 0` as Data Drain writes them (not
    /// through `ccChangeCmndTarget`).
    pub fn clear_command_targets(&mut self) {
        self.targeting.target = None;
        self.targeting.prev = None;
    }

    /// `remove_trap`'s `effRemoveTrap(cmndTarget.pos, -1, -1)` (case 123):
    /// the effect started with the frame's other shows.
    pub fn remove_trap_effect(&mut self) {
        let Some(i) = self.command_target() else { return };
        let pos = self.combat.scene.chars[i].pos;
        self.combat.shows.push(combat::Show::Entry(piney_battle::entry::Out::TrapRemoved { pos }));
    }

    /// `cmndTargetFix` from the field's menus.
    pub fn set_target_fix(&mut self, on: bool) {
        self.targeting.fix = on;
    }

    /// `plw->BreakSomething(0)` or `AttackCancel()` from the object menus,
    /// played before Kite's next frame.
    pub fn kite_menu_call(&mut self, call: crate::combat::KiteMenuCall) {
        self.combat.kite_menu.push(call);
    }

    /// `ccSkillRequest(plw, target, sid)` from the menus.
    pub fn player_skill_request(&mut self, target: Option<usize>, sid: i32) {
        if let Some(k) = self.combat.kite {
            self.combat.skill_request(k, target, sid);
        }
    }

    /// `ccChar::EntryAffect(on, plw, kind, 0, 0, 0)` from the menus and the
    /// events (13: Data Drain's hold; 5: an event's hold).
    pub fn affect_from_player(&mut self, on: usize, kind: i16) {
        let by = self.combat.kite;
        self.entry_affect(on, by, kind, [0; 3]);
    }

    /// `ccChar::EntryAffect(on, by, kind, p0, p1, p2)` with what it leads
    /// to ([`Combat::entry_affect_with`]).
    pub fn entry_affect(&mut self, on: usize, by: Option<usize>, kind: i16, p: [i16; 3]) {
        let area = self.scene.area;
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let b = Combat::bounds(area, hits);
        let ents = self.combat.ctrl.list(piney_battle::entry::Kind::Enemy);
        self.combat.affect_now(on, by, kind, p, hits, b, &mut self.save.save, &ents);
    }

    /// Data Drain's rules on the scene ([`Combat::data_drain`]): Kite drains
    /// `target` with skill `sid`; the save's infection rises, and the drops
    /// come back.
    pub fn data_drain(&mut self, target: usize, sid: i32) -> piney_battle::drain::Drain {
        self.combat.data_drain(target, sid, &mut self.save.save)
    }

    /// `ccThDrainEnemy`'s enemy for Data Drain's movie: the look of the
    /// scene's enemy `target` (its `enemyTbl` row's).
    pub fn drain_enemy(&self, target: usize) -> Option<crate::foe::DrainEnemy> {
        let c = &self.combat;
        let row = c.foes.get(target)?.as_ref()?.ent.id;
        let look = c.cast.looks.enemy(row)?.clone();
        Some(crate::foe::DrainEnemy::new(look))
    }

    /// `StreamMenu`'s `ccThStrParty`: the party members in the slots
    /// `flags`'s bits 0-2 name, each with its file and `charTbl` row.
    pub fn str_party(&self, flags: i16) -> crate::foe::StrParty {
        let c = &self.combat;
        let mut members = Vec::new();
        for k in 0..3 {
            if flags & (1 << k) == 0 {
                continue;
            }
            let (Some(who), id) = (c.party.members[k], c.party.ids[k]) else { continue };
            if let Some(a) = c.cast.get(who) {
                members.push((a.ch.body.file.clone(), id));
            }
        }
        crate::foe::StrParty::new(&members)
    }

    /// Data Drain's side effect on the party ([`Combat::drain_side_effect`]).
    pub fn drain_side_effect(&mut self) -> piney_battle::drain::SideEffect {
        self.combat.drain_side_effect(&mut self.save.save)
    }

    /// Data Drain's lost level on Kite.
    pub fn kite_level_down(&mut self) {
        self.combat.kite_level_down();
    }

    /// `ccUseItemRequest(plw, target, code, arg)` (gcmn 0x0057aa80) from the
    /// menus, the save's count already down: the item's rules on the scene
    /// now, and the whole use as steps in the game's order, which the menu
    /// plays (its windows and waits) and hands back one at a time
    /// ([`FieldWorld::item_step`]). Nothing without Kite.
    pub fn use_item(&mut self, target: usize, code: i32, arg: i32) -> Vec<piney_battle::item::Step> {
        let Some(k) = self.combat.kite else { return Vec::new() };
        let parody = self.save.save.u8(0x842b) != 0;
        self.combat.use_item(k, target, code, parody, arg)
    }

    /// The members' item uses since the last call (their AI's
    /// `ccUseItemRequest`, the ocarina command): each one's rules run on
    /// the scene now ([`Combat::use_item`]), and its steps handed back with
    /// the user and the target, for the runtime to play as the menu plays
    /// Kite's ([`FieldWorld::item_step`] for the world's).
    pub fn take_member_items(&mut self) -> Vec<(usize, usize, Vec<piney_battle::item::Step>)> {
        let uses = std::mem::take(&mut self.combat.member_items);
        let parody = self.save.save.u8(0x842b) != 0;
        uses.into_iter()
            .map(|u| (u.user, u.target, self.combat.use_item(u.user, u.target, u.code, parody, u.arg)))
            .collect()
    }

    /// One of [`FieldWorld::use_item`]'s steps on the world, as the menu
    /// reaches it (`user` used the item on `target`): the affects, the
    /// skill, the effects, the trap box's entry word, Kite's hold, the
    /// system message. False for a step the port does not carry out here
    /// (the Grunty's ride, the book, the map, the event task's stop).
    pub fn item_step(&mut self, s: &piney_battle::item::Step, user: usize, target: usize) -> bool {
        use piney_battle::event::{Event, Who};
        use piney_battle::item::Step;
        let who = |w: Who| match w {
            Who::Char(c) => Some(c),
            Who::Me => Some(user),
            Who::Target => Some(target),
            Who::Nobody => None,
        };
        match *s {
            Step::Affect(Event::Affect { on, by, kind, p }) => {
                let Some(on) = who(on) else { return true };
                let area = self.scene.area;
                let hits = match &mut self.place {
                    Place::Field(f) => &mut f.hits,
                    Place::Dungeon(d) => &mut d.hits,
                    Place::Story(m) => m.hits_mut(),
                };
                let b = Combat::bounds(area, hits);
                self.combat.entry_affect_with(on, who(by), kind, p, hits, b);
            }
            Step::Affect(_) => {}
            Step::Skill(ref k) => self.combat.item_skill(user, target, k),
            Step::EffHeal(w) => {
                if let Some(c) = who(w) {
                    self.combat.shows.push(combat::Show::Heal { who: c, n: 1 });
                }
            }
            Step::RemoveTrap(pos) => {
                self.combat.shows.push(combat::Show::Entry(piney_battle::entry::Out::TrapRemoved { pos }));
            }
            Step::BoxEnt { on, value } => {
                if let Some(c) = who(on) {
                    self.combat.scene.chars[c].ent_root = value;
                }
            }
            Step::Pause { on, value } => {
                if who(on) == self.combat.kite {
                    self.set_pause(value);
                }
            }
            Step::PlwPause(on) => self.set_pause(on),
            Step::NoDeath { on, value } => {
                if let Some(c) = who(on) {
                    self.combat.scene.chars[c].no_death = value;
                }
            }
            // ccAISysMsgSend(kind, -1, id, 0xffff, 0, param): the last is
            // the delay.
            Step::SysMsg { kind, id, param } => {
                self.combat.crew.send(kind, id, 0xffff, 0, param as u16, -1, None);
            }
            Step::ManualModeAI(ev) => {
                if let Some(k) = self.combat.kite {
                    self.manual_mode_ai(k, ev);
                }
            }
            Step::VoicesOff => self.voices_off = true,
            // The port's arrivals never take the warp's shorter fade
            // (`ActInput::warp`), so the flag is 0 already.
            Step::WarpFlagOff => {}
            _ => return false,
        }
        true
    }

    /// `ccAISysMsgSend(name, -1, Kite's AI, 0xffff, 0, 30)`: the party's
    /// line on a box or idol opened (`ccSpcMessageOpenTreasureBox`, name
    /// 0x10010, from the box menus).
    pub fn spc_message(&mut self, name: i32) {
        let Some(k) = self.combat.kite else { return };
        let id = self.combat.crew.sys_msg_id(k);
        self.combat.crew.send(name, id as u16, 0xffff, 0, 30, -1, None);
    }

    /// The world in hand for a party-AI call from the menus.
    fn ai_call<R>(&mut self, f: impl FnOnce(&mut piney_battle::party_ai::Ctx) -> R) -> R {
        let scene = self.scene;
        let in_battle = self.combat.battle.in_battle;
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        self.combat.with_ai(hits, &mut self.camera, &scene, &mut self.save.save, in_battle, f)
    }

    /// `ccSpcChar::RequestChatCmd(member, cmd, target, skill)` (the CHAT
    /// menu's orders and strategies; `ccAI::RequestChatCmd` 0x00583440).
    pub fn chat_cmd(&mut self, member: usize, cmd: i32, target: Option<usize>, sid: i32) {
        if !self.combat.crew.ais.contains_key(&member) {
            return;
        }
        self.ai_call(|ctx| ctx.request_chat_cmd(member, cmd, target, sid));
    }

    /// `ccSpcChar::ChangeEquipReport(n)` (gcmn 0x0059f4a0), from the CHAT
    /// menu's cancel: a member other than Kite answers the settings it was
    /// left, `n` 0 `ChatMessageEquipOK`, 7 `ChatMessageEquipNOT(1)`, 1-10
    /// otherwise `ChatMessageEquipNOT(0)`, anything else nothing.
    pub fn equip_report(&mut self, member: usize, n: i16) {
        use piney_battle::party_ai::Chat;
        let id = self.combat.scene.chars.get(member).map_or(0, |c| c.id());
        if id == 0 || !(0..11).contains(&n) || !self.combat.crew.ais.contains_key(&member) {
            return;
        }
        let chat = match n {
            0 => Chat::EquipOk,
            7 => Chat::EquipNot(1),
            _ => Chat::EquipNot(0),
        };
        self.ai_call(|ctx| ctx.chat_line(member, chat));
    }

    /// `ccSPC::ChangeEquip(id, cat)` (gcmn 0x0059f9e0) after the
    /// Equipment menu wrote the save's record: a weapon (categories 0-5,
    /// the six jobs' kinds) goes through `ChangeWeapon`, which hangs the
    /// record's weapon file on the character's hands; the armour's
    /// `ChangeHelmet`, `ChangeArmor`, `ChangeGlove` and `ChangeBoots` show
    /// nothing.
    pub fn change_equip(&mut self, id: i32, cat: i32) {
        if !(0..=5).contains(&cat) {
            return;
        }
        let Some((w, job)) = crate::body::weapon_of(&self.combat.data.t, &self.save.save, id) else { return };
        if id == 0 {
            self.kite.arm(&self.archive, &w);
        }
        let Some(who) = self.combat.who(id) else { return };
        if let Some(a) = self.combat.cast.actors.get_mut(&who) {
            let mut b = (*a.ch.body).clone();
            if b.equip_weapon(&self.archive, &w, job) {
                a.ch.body = Rc::new(b);
                // EquipWeapon's dummies of the new file; the trails kept.
                if let Some(arms) = a.weapon.as_mut() {
                    arms.equip(&a.ch.body, job);
                }
            }
        }
    }

    /// `member->ai->manualSW = 0`.
    pub fn manual_off(&mut self, member: usize) {
        if let Some(a) = self.combat.crew.ais.get_mut(&member) {
            a.manual_sw = false;
        }
    }

    /// `ccSpcChar::ManualModeAI(member, ev)` (gcmn 0x0059eba0).
    pub fn manual_mode_ai(&mut self, member: usize, ev: i32) {
        let annihilated = self.combat.annihilated();
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        self.combat.with_spc(member, hits, |r, hits| r.manual_mode_ai(ev, annihilated, hits));
    }

    /// `ccAI::SetRemoteCmd(member->ai, cmd)` (gcmn 0x005832e0).
    pub fn remote_cmd(&mut self, member: usize, cmd: i32) {
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        self.combat.with_spc(member, hits, |r, _| r.set_remote_cmd(cmd as i16));
    }

    /// `plw.pw->SP = sp` (Data Drain's cost).
    pub fn set_player_sp(&mut self, sp: i16) {
        if let Some(k) = self.combat.kite {
            self.combat.scene.chars[k].sp = sp;
        }
    }

    /// What the HUD projects of a battle character: the cursor's point,
    /// the life bar's answer and point, the off-screen arrow, whether it is
    /// in front of the camera, and its `cmndDist`.
    pub fn hud_geometry(&self, who: usize) -> Option<HudGeometry> {
        let c = self.combat.scene.chars.get(who)?;
        let hits = match &self.place {
            Place::Field(f) => &f.hits,
            Place::Dungeon(d) => &d.hits,
            Place::Story(m) => m.hits(),
        };
        let bounds = Combat::bounds(self.scene.area, hits);
        let player = self.player.body.pos;
        // The copy nearest the player (ccTransPosFW2LW).
        let pos = piney_battle::kite::fw2lw(&bounds, player, c.pos);
        let h = c.base().height;
        let ws = &self.camera.world_screen;
        let tag = crate::char::calc_tag_pos(ws, pos, [0, 0, ee::mul(h, 0x3ee6_6666), 0], 0)
            .filter(|t| t.2 != 0)
            .map(|t| (t.0, t.1));
        let bar_res = crate::char::calc_tag_pos(ws, pos, [0, 0, 0x4320_0000, 0], 1).map_or(0, |t| t.2);
        let bar =
            crate::char::calc_tag_pos(ws, pos, [0, 0, ee::mul(h, 0x3f66_6666), 0], 1).map_or((0, 0), |t| (t.0, t.1));
        let d = piney_battle::enemy_ai::get_dirc(player, c.pos);
        let r = ee::sub(d, self.camera.active().rot[2]);
        let arrow = (
            ee::to_int(ee::add(0x4380_0000, ee::mul(0xc3c0_0000, ee::sinf(r)))),
            ee::to_int(ee::add(0x4380_0000, ee::mul(0xc3c0_0000, ee::cosf(r)))),
        );
        let cam = self.camera.active();
        let w2p = |p: V4| piney_battle::kite::w2p_pos(&bounds, player, p).0;
        let (p, cp, vp) = (w2p(c.pos), w2p(cam.pos), w2p(cam.view));
        let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], cp[1]), ee::sub(p[0], cp[0])));
        let b = ee::rad2deg(ee::atan2f(ee::sub(vp[1], cp[1]), ee::sub(vp[0], cp[0])));
        let dd = (0x1400i32 + (i32::from(a) - i32::from(b))) as i16;
        let in_view = dd > 0 && i32::from(dd) < 2 * 0x1400;
        let dist =
            self.targeting.sorted.iter().find(|t| self.target_index(Some((t.0, t.1))) == Some(who)).map_or(0, |t| t.2);
        Some(HudGeometry { tag, bar_res, bar, arrow, in_view, dist })
    }

    /// The entry control's lists as the map reads them
    /// ([`crate::map::Entries`]): the portals (`mcHead`) and gimmicks
    /// (`gimHead`), a fountain (gimmick 20) among them.
    pub fn map_entries(&self) -> crate::map::Entries {
        use piney_battle::entry::Kind;
        let c = &self.combat;
        let mut e = crate::map::Entries::default();
        for i in c.ctrl.list(Kind::Circle) {
            let Some(o) = c.ctrl.entry_obj(i) else { continue };
            e.circles.push(crate::map::field::Circle { mx: o.ent.x, my: o.ent.y, fading: o.dest_flag, alpha: o.alpha });
            e.dungeon_circles.push(crate::map::dungeon::Ent {
                id: o.ent.id,
                floor: o.ent.floor,
                block: o.ent.block,
                pos: [o.ent.pos[0], o.ent.pos[1]],
                param2: o.ent.param[2],
            });
        }
        for i in c.ctrl.list(Kind::Gimmick) {
            let Some(o) = c.ctrl.entry_obj(i) else { continue };
            e.fountain |= o.ent.id == 20;
            e.gims.push(crate::map::dungeon::Ent {
                id: o.ent.id,
                floor: o.ent.floor,
                block: o.ent.block,
                pos: [o.ent.pos[0], o.ent.pos[1]],
                param2: o.ent.param[2],
            });
        }
        e.event_hold = self.camera.puppet_show;
        e
    }

    /// The portals' alphas as the field's map faded them (`+0xec`).
    pub fn set_map_alphas(&mut self, e: &crate::map::Entries) {
        use piney_battle::entry::Kind;
        let list = self.combat.ctrl.list(Kind::Circle);
        for (i, c) in list.into_iter().zip(&e.circles) {
            if let Some(o) = self.combat.ctrl.entry_obj_mut(i) {
                o.alpha = c.alpha;
            }
        }
    }

    /// `ccCheckTarget`'s lists: whether `who` is on a command list.
    pub fn listed(&self, who: usize) -> bool {
        self.combat.scene.listed(who)
    }

    /// What `ccChatMsg::Disp` asks of a speaker: `ccCheckTarget(c)`, and
    /// `ccCalcTagPosChar(c, p, (0, 0, 0.9 height), 0)`'s point when it
    /// passes (on the copy of the character nearest the player).
    pub fn chat_point(&self, who: usize) -> Option<(bool, Option<(i32, i32)>)> {
        let c = self.combat.scene.chars.get(who)?;
        let hits = match &self.place {
            Place::Field(f) => &f.hits,
            Place::Dungeon(d) => &d.hits,
            Place::Story(m) => m.hits(),
        };
        let bounds = Combat::bounds(self.scene.area, hits);
        let pos = piney_battle::kite::fw2lw(&bounds, self.player.body.pos, c.pos);
        let off = [0, 0, ee::mul(0x3f66_6666, c.base().height), 0];
        let at =
            crate::char::calc_tag_pos(&self.camera.world_screen, pos, off, 0).filter(|t| t.2 != 0).map(|t| (t.0, t.1));
        Some((self.listed(who), at))
    }

    /// The riding Grunty's balloon place (from Mutation on): on the command
    /// list from its constructor to the dismount, its tag at 0.9 of
    /// `pcgsTbl`'s height over it (`ccCalcTagPosChar`). None while no ride.
    pub fn ride_chat_point(&self) -> Option<(bool, Option<(i32, i32)>)> {
        let o = self.combat.ride.obj.as_deref()?;
        let hits = match &self.place {
            Place::Field(f) => &f.hits,
            Place::Dungeon(d) => &d.hits,
            Place::Story(m) => m.hits(),
        };
        let bounds = Combat::bounds(self.scene.area, hits);
        let pos = piney_battle::kite::fw2lw(&bounds, self.player.body.pos, o.ride.pos);
        let off = [0, 0, ee::mul(0x3f66_6666, piney_battle::ride::SIZE), 0];
        let at =
            crate::char::calc_tag_pos(&self.camera.world_screen, pos, off, 0).filter(|t| t.2 != 0).map(|t| (t.0, t.1));
        Some((o.listed, at))
    }

    /// `hold type code` (Some) / `hold_end` (None): the event's
    /// `ccThEvHold` task started (unless one runs) or deleted.
    pub fn set_event_hold(&mut self, target: Option<(i16, i16)>) {
        match target {
            Some((ty, code)) => piney_battle::evparty::hold(&mut self.combat.ev_hold, ty, code),
            None => piney_battle::evparty::hold_end(&mut self.combat.ev_hold),
        }
    }

    /// `battle_ready` (`ccEvent::Execute` case 163): `game.inBattleDist =
    /// -1.0`.
    pub fn battle_ready(&mut self) {
        piney_battle::evparty::battle_ready(&mut self.combat.battle.dist);
    }

    /// `remove 5|6 code` (`ccEvent::Execute` case 13): the enemy
    /// `GetEnemy(code)` finds deleted from the entry control
    /// (`deleteEnemy`). `remove -1` (`deleteAllObject`) is not ported.
    pub fn remove_entry(&mut self, ty: i16, code: i16) -> bool {
        if !matches!(ty, 5 | 6) || !self.combat.started {
            return false;
        }
        let info = self.task_info();
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
        let removed = self.combat.with_entry_cx(&mut x, |ctrl, cx| piney_battle::evparty::remove_enemy(ctrl, cx, code));
        if let Some(e) = removed
            && self.command_target() == Some(e)
        {
            self.targeting.set_target(None);
        }
        removed.is_some()
    }

    /// A treasure box of gimmick row `row` (0 a box, 1 a trapped one)
    /// put in Kite's room at `pos` facing `dirc`, holding `item` (-1 a
    /// draw), its trap `trap` (`param[2]`: 0-2, else the box rolls one), as
    /// `DUNGEON::SetItemBox` puts a story row's; its scene index. For tests
    /// and tools: the areas put their own.
    pub fn put_box(&mut self, row: i32, pos: V4, dirc: F, item: i32, trap: i32) -> Option<usize> {
        if !self.combat.started {
            return None;
        }
        let mut ep = piney_battle::entry::entry_param_clear();
        ep.pos = pos;
        ep.dirc = [0, 0, dirc, 0];
        ep.ty = 1;
        ep.id = row;
        ep.area = 2;
        ep.floor = self.scene.floor;
        ep.block = self.scene.block;
        ep.ent_root = 0;
        ep.area_num = self.scene.dungeon;
        ep.param[1] = item;
        ep.param[2] = trap;
        let info = self.task_info();
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
        self.combat.entry_object(&mut x, &mut ep)
    }

    /// `enemy_put enemy posnum` (case 74, main 0x001af028): the enemy
    /// `GetEnemy(enemy)` finds to the event position (its place, and its
    /// heading `(0, 0, rot, 1)`), from the positions the event set.
    pub fn enemy_put(&mut self, enemy: i16, pos: V4, dirc: Option<F>) -> bool {
        let c = &mut self.combat;
        let Some(e) = piney_battle::evparty::get_enemy(&c.ctrl, &c.scene, i32::from(enemy)) else { return false };
        c.scene.chars[e].pos = pos;
        if let (Some(d), Some(Some(f))) = (dirc, c.foes.get_mut(e)) {
            f.dirc = [0, 0, d, ONE];
        }
        true
    }

    /// The event's positions as the area's set-up took them
    /// (`eventMng.evPos`).
    pub fn event_positions(&self) -> &[piney_battle::entry::EvPos] {
        &self.event_entries.2
    }

    /// `player_skill`'s set-up (main 0x001b1f04,
    /// [`piney_battle::evparty::player_skill_begin`]): the menu's members
    /// in `menu`, `cmndTargetFix` here, the command target's box withheld.
    pub fn player_skill_begin(&mut self, menu: &mut piney_battle::evparty::SkillMenu) {
        menu.cmnd_target_fix = i32::from(self.targeting.fix);
        let t = self.command_target();
        piney_battle::evparty::player_skill_begin(menu, t, &mut self.combat.ctrl, &mut self.combat.foes);
        self.targeting.fix = menu.cmnd_target_fix != 0;
    }

    /// `player_skill`'s wait: while the command target is alive.
    pub fn player_skill_busy(&self) -> bool {
        piney_battle::evparty::player_skill_busy(&self.combat.scene, self.command_target())
    }

    /// One frame of `player_skill`: with no skill of Kite's running
    /// (`ccSkillCheck(plw)` 0), the pad's push against the save's confirm
    /// button (+0x840e) sets `plAttack` and asks `ccSkillRequest(plw,
    /// cmndTarget, 1)`.
    pub fn player_skill_step(&mut self, push: u32, menu: &mut piney_battle::evparty::SkillMenu) {
        let c = &self.combat;
        let Some(k) = c.kite else { return };
        let act = c.scene.chars[k].spc_char.act_num;
        let check = c.skills.borrow().check(&c.data.t, &c.scene, k, act);
        let ok = self.save.save.i16(offset::ASSIGN_PAD_ACTION + 0xa);
        let step = piney_battle::evparty::player_skill_step(check, push, ok);
        if piney_battle::evparty::player_skill_apply(menu, step) {
            let t = self.command_target();
            self.combat.skill_request(k, t, 1);
        }
    }

    /// `player_skill`'s end: the command target let go, the panels and the
    /// minimap fading out.
    pub fn player_skill_end(&mut self, menu: &mut piney_battle::evparty::SkillMenu) {
        piney_battle::evparty::player_skill_end(menu);
        self.targeting.fix = menu.cmnd_target_fix != 0;
    }

    /// `present`/`absent` of types 5 and 6 (`g_entCtrl`'s enemies) and 20
    /// (its gimmicks): one whose base type has bit `ty` and whose id is
    /// `code`.
    pub fn entry_present(&self, gimmicks: bool, ty: i16, code: i16) -> bool {
        let c = &self.combat;
        let k = if gimmicks { piney_battle::entry::Kind::Gimmick } else { piney_battle::entry::Kind::Enemy };
        c.ctrl.list(k).into_iter().any(|i| {
            let ch = &c.scene.chars[i];
            ch.ty() & (1 << (ty & 31)) != 0 && ch.id() == code
        })
    }

    /// `no_entries`: `g_entCtrl.enNum` and `mcNum` are 0.
    pub fn no_entries(&self) -> bool {
        let l = &self.combat.ctrl.lists;
        l[0].num == 0 && l[1].num == 0
    }

    /// The event's `no_active` (INF `CheckOpen` 0x001a8614-0x001a86d4):
    /// `ccCheckActiveObject()` (no enemy and no circle switched on), and no
    /// gimmick on `gimHead`'s list switched on (`objFlag`) whose base type
    /// has bit 20 (a Gott statue) and that is still on the command list
    /// (`cmndFlag` clear): the room's statue holds it until opened.
    pub fn no_active_object(&self) -> bool {
        no_active(&self.combat) && self.unopened_statue().is_none()
    }

    /// The gimmick switched on (`objFlag`) and still on the command list
    /// (`cmndFlag` clear) nearest `at` (x, y): a box not yet opened, a
    /// statue; its scene index.
    pub fn unopened_gimmick_near(&self, at: [f32; 2]) -> Option<usize> {
        let c = &self.combat;
        let dist = |i: usize| {
            let p = c.scene.chars[i].pos.map(f32::from_bits);
            (p[0] - at[0]).hypot(p[1] - at[1])
        };
        c.ctrl
            .list(piney_battle::entry::Kind::Gimmick)
            .into_iter()
            .filter(|&i| c.ctrl.entry_obj(i).is_some_and(|o| o.obj_flag && !o.cmnd_flag) && i < c.scene.chars.len())
            .min_by(|&a, &b| dist(a).total_cmp(&dist(b)))
    }

    /// A Gott statue switched on (`objFlag`) and still on the command list
    /// (`cmndFlag` clear), not yet opened: its scene index.
    pub fn unopened_statue(&self) -> Option<usize> {
        let c = &self.combat;
        c.ctrl.list(piney_battle::entry::Kind::Gimmick).into_iter().find(|&i| {
            c.ctrl.entry_obj(i).is_some_and(|o| o.obj_flag && !o.cmnd_flag)
                && c.scene.chars.get(i).is_some_and(|ch| ch.ty() & IDOL_TYPE != 0)
        })
    }

    /// `open_door` (`ccEvent::Execute` case 158, main 0x001b218c):
    /// `DUNGEON::OpenDoor(game.floor, game.block)`; nothing in a field.
    pub fn open_door(&mut self) {
        let (f, b) = (self.scene.floor, self.scene.block);
        if let (Place::Dungeon(d), Ok(f), Ok(b)) = (&mut self.place, usize::try_from(f), usize::try_from(b)) {
            d.open_door(f, b);
        }
    }

    /// `radiator rtype type code x y z roty rotz` (case 45, main
    /// 0x001ac7c8): the character `type code` names (`1 << type`: 4
    /// `GetSpc`, 0x18 `GetNpc`, 0x60 `GetEnemy`, else
    /// `ccCheckTargetTypeId`), then `entryObject(ep, 1)` of gimmick row 19
    /// at (10 x, 10 y, 10 z) turned (0, `DEG2RAD(roty)`, `DEG2RAD(rotz)`),
    /// `entRoot` 0, `param[2]` `rtype`, `param[3]` the character: with
    /// `rtype` 0 the rays at its right hand ([`piney_battle::gimetc`]).
    #[allow(clippy::too_many_arguments)]
    pub fn radiator(&mut self, rtype: i16, ty: i16, code: i16, x: i16, y: i16, z: i16, roty: i16, rotz: i16) {
        use piney_battle::geom::deg2rad;
        let user = self.ev_party(|p| p.radiator_user(ty, code));
        let s = self.scene;
        let mut ep = piney_battle::entry::entry_param_clear();
        let k = |v: i16| ee::mul(0x4120_0000, ee::from_int(i32::from(v)));
        (ep.pos[0], ep.pos[1], ep.pos[2]) = (k(x), k(y), k(z));
        (ep.dirc[0], ep.dirc[1], ep.dirc[2]) = (0, deg2rad(roty), deg2rad(rotz));
        (ep.ty, ep.id, ep.area) = (1, 19, s.area);
        if let Some(n) = match s.area {
            0 => Some(s.town),
            1 => Some(s.field),
            2 => Some(s.dungeon),
            _ => None,
        } {
            ep.area_num = n;
        }
        (ep.floor, ep.block, ep.ent_root) = (s.floor, s.block, 0);
        ep.param[2] = i32::from(rtype);
        ep.param[3] = piney_battle::gimetc::rad_user_param(user);
        let info = self.task_info();
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
        self.combat.entry_object_n(&mut x, &mut ep, 1);
    }

    /// `close_door` (case 159, main 0x001b21cc): `DUNGEON::CloseDoor2`, the
    /// doors' closing counted down by `MoveDoor`; nothing in a field.
    pub fn close_door(&mut self) {
        if let Place::Dungeon(d) = &mut self.place {
            d.close_door2();
        }
    }

    /// An event's party instruction over the battle's characters
    /// ([`piney_battle::evparty::EvParty`]); a character taken off the
    /// command lists that was the command target lets it go
    /// (`ccChangeCmndTarget(0)`).
    fn ev_party<R>(&mut self, f: impl FnOnce(&mut piney_battle::evparty::EvParty) -> R) -> R {
        let ids = std::array::from_fn(|i| self.spcs.registry[i].id);
        let roster = self.combat.roster(ids);
        let area = self.scene.area;
        let positions = self.event_entries.2.clone();
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let (r, out) = self.combat.with_ev_party(hits, &mut self.camera, area, &roster, &positions, f);
        for o in out {
            if let piney_battle::entry::Out::DeleteCmnd(c) = o
                && self.command_target() == Some(c)
            {
                self.targeting.set_target(None);
            }
        }
        r
    }

    /// `ccThEvHold` (priority 33), after the event and before the menus
    /// and the camera.
    fn ev_hold_task(&mut self) {
        let area = self.scene.area;
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let bounds = Combat::bounds(area, hits);
        self.combat.ev_hold_frame(hits, bounds);
    }

    /// The tasks' frame.
    fn frame(&mut self, pad: &Pad, ctx: &mut Ctx, drawn: bool) {
        let awake = !self.asleep;
        let ctrl_done = std::mem::take(&mut self.ctrl_done);
        if awake {
            if !ctrl_done {
                self.count = self.count.wrapping_add(1);
                self.game_ctrl(pad);
            }
            let cpad = CamPad::from_pad(pad);
            self.event_camera_task(&cpad);
            self.ev_hold_task();
            // ccThCamera (40): cameraMain with Kite's heading.
            let mut mode = self.save.save.u8(offset::CAMERA_MODE) as i8;
            let heading = self.combat.kite_dirc()[2];
            let wiped = self.combat.annihilated();
            self.camera.main(&cpad, heading, true, wiped, &mut mode);
            self.save.save.set_u8(offset::CAMERA_MODE, mode as u8);
            // A leaver's task wakes to its exit (set by an earlier frame's
            // ccFellow::Main) and ends before its Main would run.
            self.fellows_gone();
            // ccThSpc .. ccThSkill: the battle's tasks, with the event
            // NPCs' starts of the last frame.
            self.combat.shows.append(&mut self.npc_shows);
            self.combat_frame(&cpad);
            self.sync_player();
            self.npcs_frame();
            let transfers = self
                .combat
                .shows
                .iter()
                .filter(|s| matches!(s, Show::Kite(_, piney_battle::kite::Out::Transfer)))
                .count();
            for _ in 0..transfers {
                self.requests.push(Request::Game(crate::Request::Transfer));
            }
            if std::mem::take(&mut self.combat.enter) {
                self.enter_place();
            }
        }
        let to_screen = draw::screen(&self.camera.world_screen);
        // WORLD_MAN::GO's shadow packets, the area's distant light their
        // direction (SetLightDirection, copied at GO), sysLayer's view.
        ctx.layers.shadows = draw::go_shadows(self.place.lights(), &self.camera.world_screen);
        self.draw_cast(ctx, to_screen);
        {
            let area = self.scene.area;
            let hits = match &mut self.place {
                Place::Field(f) => &mut f.hits,
                Place::Dungeon(d) => &mut d.hits,
                Place::Story(m) => m.hits_mut(),
            };
            let bounds = Combat::bounds(area, hits);
            let c = &self.combat;
            let view =
                FxView { scene: &c.scene, ctrl: &c.ctrl, cast: &c.cast, camera: &self.camera, bounds, kite: c.kite };
            self.fx.draw(&view, ctx);
        }
        // ccThFieldDisp (96): WORLD::Draw or DUNGEON::Draw - drawn from the
        // tasks' second frame; what they step (and the hit list a field's
        // passes leave) only while awake.
        if drawn {
            let eye = self.camera.active().pos;
            let player = self.player.body.pos;
            let mut story_out = Vec::new();
            match &mut self.place {
                Place::Field(field) => {
                    field.ofs = field.hits.center;
                    field.draw(&mut ctx.layers, to_screen, player, eye, awake);
                    // The weather and the ambient pictures (the rest of
                    // WORLD::Draw and DrawEffect): drawn again unchanged
                    // while the tasks sleep.
                    use crate::field_ambient::Op;
                    let odd = self.count & 1 != 0;
                    let cc = &mut self.combat.cc;
                    let mut next = || cc.next_u32();
                    let (mut ops, fresh) = field.ambient_frame(&self.camera, player, odd, &mut next, awake);
                    field.ambient_models(&ops, &mut ctx.layers, to_screen, &self.camera.world_screen, awake);
                    // The enemies' breaths' flames and bursts.
                    ops.extend(self.combat.breath_ops());
                    self.fx.field_ambient(&ops, fresh, &self.camera, ctx);
                    // TOBJ::Init's tobjSeLoopStart, after the field's
                    // ccSndSQLoad as in ccSetupGameCtrl.
                    if std::mem::take(&mut field.ambient.tobj_se_start) {
                        self.ambient_calls.push(Op::SeLoopStart);
                    }
                    if fresh {
                        let calls = ops
                            .iter()
                            .filter(|o| matches!(o, Op::Se(_) | Op::Se3d(..) | Op::SeLoop(..) | Op::Flash { .. }));
                        self.ambient_calls.extend(calls.cloned());
                    }
                }
                Place::Dungeon(d) => {
                    // DUNGEON::Draw's MoveDoor for the room under the player,
                    // its doors' sounds (ccSeOn3D) played with the weather's.
                    if awake && let Some(here) = d.here(player) {
                        let c = &self.combat;
                        let clear_all = no_active(c);
                        let clear_here = room_clear(c, d.level as i32, here as i32);
                        // MoveDoor's SetDoor reads the save's area bans.
                        d.bans = crate::dungeon_area::bans_of(&self.save.save);
                        let sounds = d.move_door(here, clear_all, clear_here);
                        if let Some(se) = d.door_se() {
                            let ops = sounds.into_iter().map(|(_, pos)| crate::field_ambient::Op::Se3d(se, pos));
                            self.ambient_calls.extend(ops);
                        }
                    }
                    // The dressing's sparks and glows (DrawEff) go to the
                    // effects as the field's ambient sprites do.
                    let mut ops = d.draw(&mut ctx.layers, to_screen, &self.camera.world_screen, player, awake);
                    // The symbols' fires (ccSymFire::main's ccEff::Draw on
                    // layer 3): EFF_x008 of XGSYMBOL.CCS, Init(chunk, 0).
                    ops.extend(self.combat.symbol_fires.iter().map(|&(pos, pattern, scale, colour)| {
                        crate::field_ambient::Op::Sprite(crate::field_ambient::Sprite {
                            layer: 3,
                            file: "xgsymbol",
                            name: piney_battle::gimmick::SYMBOL_FIRE,
                            pos,
                            pattern,
                            scale,
                            rotate: 0,
                            transparency: ee::ONE,
                            fog: false,
                            ztest: true,
                            colour: Some(colour),
                            clut: None,
                        })
                    }));
                    // The enemies' breaths' flames and bursts.
                    ops.extend(self.combat.breath_ops());
                    if !ops.is_empty() {
                        self.fx.field_ambient(&ops, awake, &self.camera, ctx);
                    }
                    // SetPathFindingMap once the room under the player is
                    // built after a room change (roomEnterFlag), after its
                    // MakeMiniMap: before the next frame's tasks here.
                    if awake && d.room_enter && d.shown(player).1 {
                        d.room_enter = false;
                        self.path_map = true;
                    }
                }
                Place::Story(m) => {
                    // A map's own scene (area 43's fly-over) as its Draw
                    // runs, while the tasks are awake.
                    if awake {
                        let mut x = crate::story_map::StoryFrame {
                            camera: &mut self.camera,
                            save: &self.save.save,
                            server: self.scene.server,
                            message_closed: std::mem::take(&mut self.story_message_closed),
                            out: Vec::new(),
                        };
                        m.frame(&mut x);
                        story_out = x.out;
                    }
                    let v = crate::town::TownView {
                        eye,
                        player,
                        cam: self.camera.active().clone(),
                        flare: crate::evarea::FlareCamera::of(&self.camera),
                        puppet_show: self.camera.puppet_show,
                        world_screen: self.camera.world_screen,
                    };
                    let sprites = m.draw_rand(&mut ctx.layers, to_screen, &v, awake, &mut self.combat.cc);
                    self.fx.story_sprites(&sprites, &self.camera, ctx);
                }
            }
            for r in story_out {
                self.story_request(r);
            }
        }
    }

    /// What a story map's scene asked: `MenuBan` / `MenuClr` (the camera's
    /// and the party's part here, the menus' for the game), its message (for
    /// the game to open and check), `ChangeArea`.
    fn story_request(&mut self, r: crate::story_map::StoryRequest) {
        use crate::story_map::StoryRequest;
        match r {
            StoryRequest::MenuBan(on) => {
                self.menu_ban_camera(on);
                self.menu_ban_party(on);
                self.requests.push(Request::Story(r));
            }
            StoryRequest::Message { .. } => self.requests.push(Request::Story(r)),
            StoryRequest::ChangeArea(a, n) => self.change_area(a, n),
        }
    }

    /// The story map's message (`ccMsg->Check(0)` answered): its scene
    /// goes on next time it runs.
    pub fn story_message_closed(&mut self) {
        self.story_message_closed = true;
    }

    /// The battle's tasks over the area's collision and the camera (the
    /// entry control set up first if [`FieldWorld::entry_setup`] has not
    /// run).
    fn combat_frame(&mut self, cpad: &CamPad) {
        self.entry_setup(cpad);
        let info = self.task_info();
        // Kyvia's disc as the boss reads it this frame (`IsMove`,
        // `discPrevPos`, `DMY_marker01`).
        if let (Some(r), Some(a)) = (self.combat.boss.as_mut(), self.place.story::<crate::evarea_b8::DiscArea>()) {
            r.disc = disc_view(a);
        }
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, *cpad, &info);
        x.path_map = std::mem::take(&mut self.path_map);
        // resignParty and disbandSpc of a member leaving (remote command 5)
        // on the one registry and party, as in a town.
        let before = self.spcs.party();
        x.spcs = Some(&mut self.spcs);
        self.combat.frame(&mut x, self.fx.tasks());
        self.spcs.party_strategy = self.combat.spc.party_strategy;
        if self.spcs.party() != before {
            self.combat.set_party(self.spcs.party());
        }
        // EVENTAREAB8::Move from the boss's CheckDiscMove.
        if let Some(r) = self.combat.boss.as_mut()
            && std::mem::take(&mut r.disc_next)
            && let Some(a) = self.place.story_mut::<crate::evarea_b8::DiscArea>()
        {
            a.next_stage();
        }
    }

    /// `ccThEntryCtrl`'s first slice (gcmn 0x00431970, before its first
    /// `Breath`): `restoreEntry`, `WORLD_MAN::EntryGimmick` and
    /// `ccEntryEventMng` (whose `DUNGEON::CloseDoor` shuts the room's doors for
    /// each event entry in a dungeon), on the frame of [`Phase::Play`] 0, before
    /// the event's first pass that plays; so event 4's trap room opens its doors
    /// then to shut them on camera a frame later.
    fn entry_setup(&mut self, cpad: &CamPad) {
        if !self.combat.started {
            let (ty, rank) = {
                let s = self.scene;
                self.world_man.enemy_rank(s.area, s.field, s.floor, self.world_man.field_attr())
            };
            let (mc, en, pos) = self.event_entries.clone();
            let field_attr = self.world_man.field_attr();
            let time_sym = self.world_man.time_sym();
            let dungeon = match &mut self.place {
                Place::Dungeon(d) => {
                    let gims = (!d.gimmicks_placed).then(|| d.dungeon_gims(field_attr, time_sym));
                    d.gimmicks_placed = true;
                    Some(combat::DungeonEntries {
                        kept: d.kept_entries.take(),
                        actors: std::mem::take(&mut d.kept_actors),
                        gims,
                        rng: d.rng,
                        dtype: d.dtype,
                        breakables: d.breakables_here(self.scene.field, self.scene.block),
                    })
                }
                _ => None,
            };
            // WORLD_MAN::EntryGimmick's field branch (not on a story map of
            // its own, which is not a Place::Field).
            let field = match &self.place {
                Place::Field(f) => Some(combat::FieldEntries {
                    gims: f.field_gims(self.scene.field),
                    map: f.field.clone(),
                    rng: f.rng,
                    circle_ofs: self.world_man.circle_ofs,
                    event_area: self.scene.field,
                    start: f.start_pos(),
                }),
                _ => None,
            };
            let info = self.task_info();
            let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, *cpad, &info);
            let rng = self.combat.start_entries(&mut x, rank, ty, &mc, &en, &pos, dungeon, field);
            drop(x);
            if let (Some(rng), Place::Field(f)) = (rng, &mut self.place) {
                f.rng = rng;
            }
            self.build_npcs();
            // An `entry 7 code`: ccEntryEventMng's ccBossEntryStart(code),
            // the boss at the arena's centre (DMY_center01).
            if let Some(e) = en.iter().find(|e| e[0] == 7) {
                self.start_boss(i32::from(e[1]));
            }
            let n = mc.iter().filter(|e| e[0] >= 0).count() + en.iter().filter(|e| matches!(e[0], 5 | 6)).count();
            let (f, b) = (self.scene.floor, self.scene.block);
            if let (Place::Dungeon(d), Ok(f), Ok(b)) = (&mut self.place, usize::try_from(f), usize::try_from(b)) {
                for _ in 0..n {
                    d.close_door(f, b);
                }
            }
        }
    }

    /// The classes of the event's NPCs the entry control has just made
    /// ([`crate::field_npcs`]), where `ccEntryEventMng` put their stand-ins.
    fn build_npcs(&mut self) {
        let placed = self.combat.npcs.clone();
        if placed.is_empty() {
            return;
        }
        let (area, town) = (self.scene.area, self.scene.town);
        let player = self.player.body.pos;
        let chars = &self.combat.scene.chars;
        let objs = &self.combat.ctrl.objs;
        let at = |who: usize| {
            let dirc = match objs.get(who) {
                Some(piney_battle::entry::Obj::Npc(o)) => o.dirc,
                _ => [0, 0, 0, ONE],
            };
            (chars[who].pos, dirc)
        };
        let hits = self.place.hits();
        let (archive, volume, cc) = (&self.archive, self.volume, &mut self.combat.cc);
        let missing = self.npcs.with_cc(cc, |n| n.build(archive, volume, &placed, area, town, hits, player, at));
        for m in missing {
            tracing::warn!("the event's NPCs: {m}");
        }
    }

    /// One frame of the event's NPCs (their place on the entry control's
    /// NPC list, after the battle's tasks here): each class steps, its
    /// stand-in follows (position, heading, the command lists), their
    /// starts wait for the next frame's effects, and a
    /// class done with its NPC has the stand-in deleted.
    fn npcs_frame(&mut self) {
        if self.npcs.list.is_empty() {
            return;
        }
        let a = self.camera.active();
        let view = crate::char::View {
            player: self.player.body.pos,
            cam: a.pos,
            deg1: a.deg[1],
            eye: a.kind == crate::camera::kind::EYE,
        };
        let mut rand = crate::Rand(self.combat.rand.0);
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let mut cx = crate::entry::NpcCtx {
            player: self.player.body.pos,
            player_dirc: self.player.body.dirc,
            view,
            cam: self.camera.active(),
            hits,
            rand: &mut rand,
        };
        let out = self.npcs.with_cc(&mut self.combat.cc, |n| n.step(&mut cx));
        self.combat.rand.0 = rand.0;
        for n in &self.npcs.list {
            let c = &mut self.combat;
            let Some(ch) = c.scene.chars.get_mut(n.who) else { continue };
            ch.pos = n.npc().char().pos;
            ch.pos_p = n.npc().char().pos_p;
            if let Some(piney_battle::entry::Obj::Npc(o)) = c.ctrl.objs.get_mut(n.who) {
                o.dirc = n.dirc();
            }
            let listed = c.scene.listed(n.who);
            if n.npc().listed() && !listed {
                piney_battle::fellow::entry_cmnd(&mut c.scene, n.who);
            } else if !n.npc().listed() && listed {
                piney_battle::fellow::delete_cmnd(&mut c.scene, n.who);
            }
        }
        for (who, e) in out.starts {
            match e {
                crate::merchant::SysopEvent::Noise => self.noise = true,
                _ => self.npc_shows.push(Show::Npc(who, e)),
            }
        }
        if !out.gone.is_empty() {
            let info = self.task_info();
            let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
            self.combat.with_entry_cx(&mut x, |ctrl, cx| {
                for &who in &out.gone {
                    ctrl.delete_npc(cx, who);
                }
            });
            self.combat.npcs.retain(|n| !out.gone.contains(&n.who));
        }
    }

    /// The `near_marker` condition's distance outside the towns:
    /// `ccGetDist(ccTransPosW2P(pos), plw->posP)` (`ccEvent::CheckOpen`,
    /// main 0x001a7d9c) through the map's bounds. Kite's `posP` is
    /// `FZeroPosition` with his ground height (0, 0, z), or a carrier's
    /// move this frame while he rides one (worklog 328).
    pub fn player_distance(&self, pos: V4) -> F {
        let hits = match &self.place {
            Place::Field(f) => &f.hits,
            Place::Dungeon(d) => &d.hits,
            Place::Story(m) => m.hits(),
        };
        let b = Combat::bounds(self.scene.area, hits);
        let p = self.player.body.pos;
        let at = |v: V4| piney_battle::kite::w2p_pos(&b, p, v).0;
        let pos_p = self.combat.kite.and_then(|k| self.combat.scene.chars.get(k)).map_or(at(p), |c| c.pos_p);
        piney_battle::enemy_ai::get_dist_on(self.volume, at(pos), pos_p)
    }

    /// `ccStoreSpcCondition()` as the next scene's `ccSetupGameCtrl` runs
    /// it: the party's characters (Kite and the members this area built)
    /// stored in the registry's `storeCondition` for the next area.
    pub fn store_conditions(&mut self) {
        let c = &self.combat;
        self.spcs.store_conditions(c.members.iter().map(|&(id, who)| (id, &c.scene.chars[who])));
    }

    /// The event's NPCs ([`crate::field_npcs`]).
    pub fn npcs(&self) -> &crate::field_npcs::FieldNpcs {
        &self.npcs
    }

    /// Whether the Administrator asked for `ccMenu.interNoiz = 2` since the
    /// last call.
    pub fn take_noise(&mut self) -> bool {
        std::mem::take(&mut self.noise)
    }

    /// `ccBossEntryStart(code)`: the boss's look read and its tasks
    /// started.
    fn start_boss(&mut self, code: i32) {
        let look = match combat::boss::BossLook::load(&self.archive, code, self.combat.data.volume, self.scene.field) {
            Ok(l) => Rc::new(l),
            Err(e) => {
                tracing::warn!("the boss's files: {e}");
                return;
            }
        };
        let center = match self.place.story::<Arena>() {
            Some(a) => [a.start[0], a.start[1], a.start[2], ONE],
            None => self.player.body.pos,
        };
        let disc = self.place.story::<crate::evarea_b8::DiscArea>().map(disc_view).unwrap_or_default();
        let env = piney_battle::chara::Env {
            count: self.count,
            menu_type: self.menu_type,
            area: self.scene.area,
            ..piney_battle::chara::Env::default()
        };
        self.combat.start_boss(&look, center, disc, &env, &mut self.camera);
    }

    /// `worldman->eventArea->SwitchLayer()` in a boss arena.
    pub fn arena_switch_layer(&mut self) {
        if let Some(a) = self.place.story_mut::<Arena>() {
            a.switch_layer();
        }
    }

    /// `eventMng.bossEntry` and the boss task's parameter, for an event's
    /// `present 7` / `absent 7`: None before `entry 7`, 0 while it fights,
    /// 1 once it has exited.
    pub fn boss_task(&self) -> Option<i32> {
        self.combat.boss.as_ref().map(|b| i32::from(b.exit))
    }

    /// `eventMng.bossEntry`: the code of the boss the event entered.
    pub fn boss_entry(&self) -> Option<i32> {
        self.combat.boss.as_ref().map(|b| b.code)
    }

    /// What [`tasks`] copies of the field world.
    fn task_info(&self) -> TaskInfo {
        TaskInfo {
            scene: self.scene,
            wm: self.world_man,
            menu_type: self.menu_type,
            menu_forbid: self.menu_view.forbid,
            cmnd_target: self.targeting.target.is_some(),
            count: self.count,
            effect_sw: self.condition_fx,
        }
    }

    /// The battle's characters where their frames left them: Kite and the
    /// members (`ccChar::Draw` after `SetMatrix_PosRotZYX`), the enemies at
    /// `dispEnemy`'s matrix, the magic portals.
    fn draw_cast(&self, ctx: &mut Ctx, to_screen: glam::Mat4) {
        self.draw_rays(ctx);
        self.draw_trails(ctx);
        self.draw_boss(ctx, to_screen);
        // The party's weapon trails (ccSpcChar::ArmsEffect's Disp).
        for w in self.combat.cast.actors.values().filter_map(|a| a.weapon.as_ref()) {
            w.send(&mut ctx.layers, &self.camera.world_screen);
        }
        let lit = self.cast_lights();
        let lights = &*lit;
        self.npcs.draw(&mut ctx.layers, to_screen, lights);
        // The riding Grunty and Kite on it (ccThPucciguso's draws).
        self.combat.ride.draw(&mut ctx.layers, to_screen, lights);
        let looks = &self.combat.cast.looks;
        for a in self.combat.cast.actors.values() {
            if !a.drawn {
                continue;
            }
            match (a.look, &a.matrix) {
                // ccChar::Draw at dispEnemy's matrix, as it decided.
                (Look::Enemy(row), Some(m)) => {
                    if let (Some(l), Some((m2, alpha2)), Some(play2)) = (looks.enemy(row), &a.second_draw, &a.second) {
                        l.draw_second(&mut ctx.layers, to_screen, play2, m2, *alpha2, lights);
                    }
                    if let (Some(l), Some(how)) = (looks.enemy(row), &a.how) {
                        draw::shadow_env(&mut ctx.layers, how.shadow_alpha, how.shadow_length, |layers| {
                            l.draw(layers, to_screen, &a.ch.play, m, how, lights)
                        });
                    }
                }
                (Look::Enemy(_), None) => {}
                // The spring (ccGimEtc row 20): its water on refLayer at
                // the matrix it set, SetBlendType(4).
                (Look::Gimmick(20), Some(m)) => {
                    a.ch.body.draw(
                        &mut ctx.layers,
                        crate::field_area::REF_LAYER,
                        to_screen,
                        &a.ch.play,
                        draw::mat(m),
                        1.0,
                        lights,
                        false,
                        &[],
                    );
                }
                (Look::Gimmick(20), None) => {}
                // A Grunty food: ccChar::Draw at SetMatrix_PosRotZYXScale's
                // matrix.
                (Look::Gimmick(_), Some(m)) => {
                    let shaded = a.ch.hit_attribute & crate::char::SHADED != 0;
                    draw::char_shadow(&mut ctx.layers, a.alpha, a.ch.height, |layers| {
                        a.ch.body.draw_fog(
                            layers,
                            draw::CHAR_LAYER,
                            to_screen,
                            &a.ch.play,
                            draw::mat(m),
                            ee::f(a.alpha),
                            lights,
                            shaded,
                            &a.swaps,
                            a.ch.blend.into(),
                        )
                    });
                }
                // ccMagicCircle::main's ccAnm::Draw on the effect layer
                // (the effects' when they draw the portals).
                (Look::Circle, _) => {
                    if !self.fx.draws_circles()
                        && let Some(c) = &looks.circle
                    {
                        c.draw(&mut ctx.layers, to_screen, &a.ch.play, a.ch.pos, a.ch.dirc, a.alpha, lights);
                    }
                }
                // Kite's blades fading in after a gate hack.
                (Look::Kite, _) if a.arms.is_some() => {
                    let shaded = a.ch.hit_attribute & crate::char::SHADED != 0;
                    let arms = a.arms.unwrap_or(1.0);
                    let alpha = ee::f(a.alpha);
                    let (p, d) = (a.ch.pos, a.ch.dirc);
                    draw::char_shadow(&mut ctx.layers, a.alpha, a.ch.height, |l| {
                        a.ch.body.draw_char_arms(l, to_screen, &a.ch.play, p, d, alpha, lights, shaded, &a.swaps, arms)
                    });
                }
                // ccChar::Draw with the character's blend (an affect's
                // tint: piros_colour on Piros).
                _ => {
                    let shaded = a.ch.hit_attribute & crate::char::SHADED != 0;
                    draw::char_shadow(&mut ctx.layers, a.alpha, a.ch.height, |layers| {
                        a.ch.body.draw_char_fog(
                            layers,
                            to_screen,
                            &a.ch.play,
                            a.ch.pos,
                            a.ch.dirc,
                            ee::f(a.alpha),
                            lights,
                            shaded,
                            &a.swaps,
                            a.ch.blend.into(),
                        )
                    });
                }
            }
        }
    }

    /// The light group the characters are drawn with: the map's, and the
    /// effects' own lights while they are in it.
    fn cast_lights(&self) -> std::borrow::Cow<'_, crate::town::TownLights> {
        let extra = self.fx.lights();
        let symbols = &self.combat.symbol_lights;
        let springs = &self.combat.spring_lights;
        let rays = &self.combat.rad_lights;
        let flashes = &self.combat.weapons.lights;
        if extra.is_empty() && symbols.is_empty() && springs.is_empty() && rays.is_empty() && flashes.is_empty() {
            return std::borrow::Cow::Borrowed(self.place.lights());
        }
        let mut l = self.place.lights().clone();
        for &(p, intensity) in symbols.values().rev() {
            l.lights.insert(0, symbol_light(p, intensity));
        }
        // The spring's, the rays' and the flashes': `ccLight(4, 1)`, their colour
        // (`ccSetColor`'s low byte red) and fall-off as their frame left it.
        for o in springs.values().flatten().chain(rays.values()).chain(flashes.values()).rev() {
            l.lights.insert(0, omni_light(o));
        }
        l.lights.extend(extra);
        std::borrow::Cow::Owned(l)
    }

    /// The boss's own pictures besides its body: `DrawAfterImages` (each
    /// image a white silhouette of the pose it was entered with, fading, on
    /// effLayer) and the wave's `DrawParts` (`ANM_xx11wave`'s objects at
    /// the boss, on objLayer).
    fn draw_boss(&self, ctx: &mut Ctx, to_screen: glam::Mat4) {
        let Some(run) = self.combat.boss.as_ref().filter(|r| !r.exit) else { return };
        let lights = self.place.lights();
        let body = &run.look.body;
        let white = piney_draw::Fog { f: 0, colour: [0xff; 3] };
        for a in &run.afterimages {
            let Some(mut p) = body.play(&a.clip) else { continue };
            (p.time, p.posed) = (a.time, a.time);
            let root = crate::body::root(a.pos, a.dirc);
            body.draw_fog(
                &mut ctx.layers,
                draw::EFF_LAYER,
                to_screen,
                &p,
                root,
                a.transparency,
                lights,
                false,
                &[],
                draw::Fogging::Const(white),
            );
        }
        if run.reverse {
            ctx.layers.prepend(REVERSE_LAYER, vec![reverse_packet()]);
        }
        // Fidchell's prediction (OnThinkPrediction's step 3): its ccAnm on
        // its own layer, on sysLayer's view through the animation's own
        // camera, no depth test.
        if let Some((_, a)) = run.pred_text.as_ref().filter(|_| run.text_shown) {
            let view = ctx.view.clone();
            a.draw_on(ctx, combat::boss::PRED_TEXT_LAYER, &view);
        }
        // The cinema's bars and name (ccBossEffManager::Draw's last step).
        let cinema = run.cinema.packets(&run.cinema_sent);
        if !cinema.is_empty() {
            ctx.layers.prepend(crate::cinema::CINEMA_LAYER, cinema);
        }
        if let Some(cmd) = trail_packet(&run.trail.0, &self.camera.world_screen) {
            ctx.layers.sorted(draw::EFF_LAYER, f32::from_bits(run.trail.1), cmd);
        }
        if let Some((time, pos, dirc)) = run.wave
            && let Some(mut p) = crate::pose::Play::new(&run.look.eff, combat::boss::ANM_WAVE)
        {
            (p.time, p.posed) = (time, time);
            let root = crate::body::root(pos, dirc);
            let file = (&*run.look.eff, &run.look.eff_morphers);
            crate::town::anim_draw_on(
                &mut ctx.layers,
                draw::OBJ_LAYER,
                to_screen,
                file,
                &p,
                root,
                &[],
                1.0,
                None,
                None,
            );
        }
    }

    /// `ccPrimRadiate::disp` of the boxes' and idols' rays: each ray's four
    /// points through `sceVu0RotTransPers(world_screen)`, a gouraud strip
    /// (`cczGsPrimPoly`, PRIM 0x4c) with its points' colours, blended
    /// `(Cs - Cd) As + Cd` (ALPHA 0x44), depth-tested GREATER with no
    /// depth write (TEST 0x73001, ZMSK), on `refLayer` (priority 30).
    fn draw_rays(&self, ctx: &mut Ctx) {
        use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
        use piney_draw::{
            AlphaFail, AlphaTest, Blend, Cmd, Compare, Depth, DrawState, Prim, PrimKind, Rgba, Scissor, Vertex, ZTest,
        };
        const REF_LAYER: i16 = 30;
        if self.combat.rays.is_empty() {
            return;
        }
        let ws = self.camera.world_screen;
        let state = DrawState {
            blend: Some(Blend::MIX),
            alpha_test: AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::RgbOnly },
            depth: Depth { test: ZTest::Greater, write: false },
            texture: None,
            scissor: Scissor::FULL,
        };
        let mut group = Vec::new();
        for rays in &self.combat.rays {
            for r in rays {
                let verts = r
                    .iter()
                    .map(|v| {
                        let p = ee::rot_trans_pers(&ws, v.pos);
                        Vertex {
                            x: p[0].wrapping_sub(XYOFFSET_X) as f32 / 16.0,
                            y: p[1].wrapping_sub(XYOFFSET_Y) as f32 / 16.0,
                            z: p[2] as u32,
                            u: 0.0,
                            v: 0.0,
                            rgba: Rgba(v.color.to_le_bytes()),
                        }
                    })
                    .collect();
                group.push(Cmd::Prim(Prim { kind: PrimKind::Strip, gouraud: true, state: state.clone(), verts }));
            }
        }
        ctx.layers.prepend(REF_LAYER, group);
    }

    /// `ccEnemyWeapon::dispCell`'s packets, each weapon's sent by
    /// `ccPrimPacket::sendPacket` (gcmn 0x00438820) on `refLayer` (blend `(Cs -
    /// Cd) As + Cd`, depth GREATER, no depth write): the lines, then the strip
    /// through every pair. `makePacket` (0x00438580) sets ADC on a pair that
    /// starts a run, has a point behind the eye, or lies far off the screen; such
    /// a vertex draws nothing. Under two pairs nothing is sent
    /// (docs/engine/battle.md).
    fn draw_trails(&self, ctx: &mut Ctx) {
        use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
        use piney_draw::{
            AlphaFail, AlphaTest, Blend, Cmd, Compare, Depth, DrawState, Prim, PrimKind, Rgba, Scissor, Vertex, ZTest,
        };
        const REF_LAYER: i16 = 30;
        if self.combat.trails.is_empty() {
            return;
        }
        let ws = self.camera.world_screen;
        let state = DrawState {
            blend: Some(Blend::MIX),
            alpha_test: AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::RgbOnly },
            depth: Depth { test: ZTest::Greater, write: false },
            texture: None,
            scissor: Scissor::FULL,
        };
        // A pair's two vertices and whether they draw (no ADC).
        let pair = |q: &piney_battle::weapon::Pair| {
            let p = q.pos.map(|v| ee::rot_trans_pers(&ws, v));
            // addiu -0x8000, newlib abs, slti: the EE's wrapping words (a
            // point far off saturates cvt.w.s).
            let far = |x: i32, lim: i32| x.wrapping_sub(0x8000).wrapping_abs() > lim;
            let off = |k: usize, lim: i32| far(p[0][k], lim) && far(p[1][k], lim);
            let draws = q.cont && p[0][2] >= 0 && p[1][2] >= 0 && !off(0, 5632) && !off(1, 4608);
            let v = |k: usize| Vertex {
                x: p[k][0].wrapping_sub(XYOFFSET_X) as f32 / 16.0,
                y: p[k][1].wrapping_sub(XYOFFSET_Y) as f32 / 16.0,
                z: p[k][2] as u32,
                u: 0.0,
                v: 0.0,
                rgba: Rgba(q.color[k].to_le_bytes()),
            };
            ([v(0), v(1)], draws)
        };
        let mut group = Vec::new();
        for t in &self.combat.trails {
            if t.lines.len() >= 2 {
                let mut tris = Vec::new();
                for q in &t.lines {
                    let ([a, b], draws) = pair(q);
                    if draws {
                        // A one-pixel band across the line.
                        let (dx, dy) = (b.x - a.x, b.y - a.y);
                        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                        let (nx, ny) = (-dy / len * 0.5, dx / len * 0.5);
                        let sh = |v: Vertex, s: f32| Vertex { x: v.x + nx * s, y: v.y + ny * s, ..v };
                        tris.extend([sh(a, 1.0), sh(a, -1.0), sh(b, 1.0), sh(a, -1.0), sh(b, -1.0), sh(b, 1.0)]);
                    }
                }
                if !tris.is_empty() {
                    group.push(Cmd::Prim(Prim {
                        kind: PrimKind::Triangles,
                        gouraud: true,
                        state: state.clone(),
                        verts: tris,
                    }));
                }
            }
            if t.polys.len() >= 2 {
                let verts: Vec<(Vertex, bool)> = t
                    .polys
                    .iter()
                    .flat_map(|q| {
                        let ([a, b], draws) = pair(q);
                        [(a, draws), (b, draws)]
                    })
                    .collect();
                let mut tris = Vec::new();
                for k in 2..verts.len() {
                    if verts[k].1 {
                        tris.extend([verts[k - 2].0, verts[k - 1].0, verts[k].0]);
                    }
                }
                if !tris.is_empty() {
                    group.push(Cmd::Prim(Prim {
                        kind: PrimKind::Triangles,
                        gouraud: true,
                        state: state.clone(),
                        verts: tris,
                    }));
                }
            }
        }
        ctx.layers.prepend(REF_LAYER, group);
    }

    // --- what the event scripts reach (the area's event host) ---------------

    /// A camera instruction (`Host::camera`): the event camera's task
    /// started, or stopped and the field camera given back.
    pub fn event_camera(&mut self, c: piney_event::host::CameraCommand) {
        let mut ev = std::mem::take(&mut self.evcam);
        let mut cam = self.camera.clone();
        ev.command(c, &mut cam, self);
        self.evcam = ev;
        self.camera = cam;
    }

    /// `menu_ban` / `menu_clear`'s camera part.
    pub fn menu_ban_camera(&mut self, on: bool) {
        self.evcam.menu_ban(on, &mut self.camera);
    }

    /// `teach_camera1..3`'s camera part.
    pub fn teach_camera(&mut self, part: u8) {
        self.evcam.teach(part);
    }

    /// `ccThCameraExecute` (33): the event camera's frame while it runs.
    fn event_camera_task(&mut self, pad: &CamPad) {
        if !self.evcam.running() {
            return;
        }
        let button = |at: usize, default: u16| match self.save.save.i16(at) as u16 {
            0 => default,
            b => b,
        };
        let input = crate::evcam::Input {
            pad: *pad,
            zoom: [button(offset::ASSIGN_PAD_ACTION + 0xe, 0x8), button(offset::ASSIGN_PAD_ACTION + 0x10, 0x2)],
        };
        let mut ev = std::mem::take(&mut self.evcam);
        let mut cam = self.camera.clone();
        ev.frame(&mut cam, self, &input);
        self.evcam = ev;
        self.camera = cam;
    }

    /// The field's fellow tasks deleted as the scene changes
    /// ([`crate::party::Spcs::delete_fellows`]).
    pub fn delete_fellows(&mut self) {
        let keep_recalled = self.volume != piney_data::volume::Volume::Inf;
        self.with_party(|spcs, chars, _| spcs.delete_fellows(chars, keep_recalled));
    }

    /// The party's characters as the event instructions see them
    /// ([`crate::combat::spc::BattleChars`]), `f` run on them, written back.
    fn with_party<R>(
        &mut self,
        f: impl FnOnce(&mut crate::party::Spcs, &mut combat::spc::BattleChars, &mut Hits) -> R,
    ) -> R {
        let c = &self.combat;
        let td = |w: usize| c.cast.get(w).is_none_or(|a| a.trans_dist);
        let mut chars = combat::spc::BattleChars::read(&c.scene, &c.crew, &c.members, c.kite, &td);
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Story(m) => m.hits_mut(),
        };
        let r = f(&mut self.spcs, &mut chars, hits);
        let cast = &mut self.combat.cast;
        chars.finish(&mut self.combat.scene, &mut self.combat.crew, &mut |w, t| {
            if let Some(a) = cast.get_mut(w) {
                a.trans_dist = t;
            }
        });
        for &(_, w) in &self.combat.members {
            piney_battle::fellow::sync_in(&self.combat.scene, &mut self.combat.crew, w);
        }
        r
    }

    /// `cameraShake(power, cycle, time, dirc)` (main 0x00162cd0) from the
    /// effects: a turning shake (`dirc` 2) draws `rand()`.
    pub fn camera_shake(&mut self, s: [i32; 4]) {
        use piney_battle::rand::Rng as _;
        let rand = &mut self.combat.rand;
        self.camera.shake.shake(s[0], s[1], s[2], s[3], &mut || rand.rand());
    }

    /// The events' `condition_fx_on` / `condition_fx_off`
    /// (`ccSpcConditionEffectON` / `OFF`, gcmn 0x005a0880, 0x005a08a0's
    /// switch): the condition tints on or off.
    pub fn condition_effect(&mut self, on: bool) {
        self.condition_fx = on;
    }

    /// `ccClearSpcCondition()` (gcmn 0x0056d300), which a boss's death calls:
    /// the condition effects off, then each character built put under manual
    /// control at remote command 0 (its revival of the fallen taken back),
    /// `noDeathFlag` on, off the command lists, its conditions cleared and its
    /// record's stat changes zeroed (`ClearCondition`, `ccClearParamElement`).
    pub fn clear_spc_condition(&mut self) {
        self.condition_fx = false;
        let members = self.combat.members.clone();
        let deads: Vec<i16> =
            members.iter().map(|&(_, w)| self.combat.scene.chars[w].cond[piney_battle::param::cond::DEAD]).collect();
        let annihilated = self.combat.annihilated();
        self.with_party(|_, chars, hits| {
            use crate::party::SpcChars as _;
            for &(id, _) in &members {
                let Some(mut c) = chars.spc(id) else { continue };
                c.manual_mode_ai(1, annihilated, hits);
                c.set_remote_cmd(0);
                *c.no_death = true;
                *c.listed = false;
            }
        });
        for (&(_, w), dead) in members.iter().zip(deads) {
            let ch = &mut self.combat.scene.chars[w];
            piney_battle::affect::clear_condition(ch);
            ch.cond[piney_battle::param::cond::DEAD] = dead;
        }
        self.combat.store_records(&mut self.save.save);
    }

    /// `menu_ban` (true) / `menu_clear` (false): the party's part.
    pub fn menu_ban_party(&mut self, on: bool) {
        self.condition_fx = !on;
        if !on {
            // MenuClr's menuClrWait 2: ccThGameCtrl waits two frames.
            self.targeting.menu_clear();
        }
        let area = self.scene.area;
        if on {
            // ccStoreSpcCondition, then each character's ClearCondition.
            self.store_conditions();
            self.combat.menu_ban_conditions();
            self.combat.store_records(&mut self.save.save);
        }
        self.with_party(|spcs, chars, hits| {
            if on {
                crate::party::menu_ban(spcs, chars, hits);
            } else {
                crate::party::menu_clr(spcs, chars, hits, area);
            }
        });
        if !on {
            // ccRestoreSpcCondition and ConditionAdjustment on each.
            let es39 = self.save.save.u8(offset::EVENT_STATUS + 39) as i8;
            let restore = crate::party::restores_condition(area, self.scene.area_prev, self.scene.field, es39);
            let store = self.spcs.store;
            let info = self.task_info();
            let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
            self.combat.menu_clear_conditions(&mut x, restore, &store);
        }
    }

    /// `FountainMenu3`'s party hold (true, gcmn 0x00548d74:
    /// `ccStoreSpcCondition`, each member's `ClearCondition`, the members
    /// held and the others unseen, `ccClearConditionAllEnemy`) and release
    /// (false, 0x005498bc: each member's condition back and adjusted, then
    /// the members free again) ([`crate::party::fountain_party`]).
    pub fn fountain_party(&mut self, on: bool) {
        self.condition_fx = !on;
        if on {
            self.store_conditions();
            self.combat.menu_ban_conditions();
            self.combat.store_records(&mut self.save.save);
            self.combat.clear_condition_all_enemy();
        } else {
            let store = self.spcs.store;
            let info = self.task_info();
            let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
            self.combat.menu_clear_conditions(&mut x, true, &store);
        }
        self.with_party(|spcs, chars, hits| crate::party::fountain_party(spcs, chars, hits, on));
    }

    /// `ccMenuCtrl::SetFountainCamera()` (gcmn 0x00526c60): the event
    /// camera on the spring (`cmndTargetPrev`) from behind Kite.
    pub fn fountain_camera(&mut self) {
        let Some(spring) = self.target_index(self.targeting.prev) else { return };
        let Some(k) = self.combat.kite else { return };
        let (s, p) = (self.combat.scene.chars[spring].pos, self.combat.scene.chars[k].pos);
        self.camera.set_fountain(s, p);
    }

    /// `changeCamera(n)` from the menus (1: the player's camera back).
    pub fn change_camera(&mut self, n: i16) {
        self.camera.change_camera(n);
    }

    /// `party_add pc` (`ccEvent::Execute` case 77, main 0x001af270) and
    /// the field UI's `AddMember`: `ccParty::AddMember(pc)` (gcmn
    /// 0x0059ce80) over the battle's characters: the first free slot,
    /// `inviteSpc` (the registry's and the character's `partyFlag` 1, a
    /// leaving one recalled); the slot, or -1 (no slot, or no character
    /// built here: `inviteSpc`'s new character at the Chaos Gate is a
    /// town's). The HUD's party follows (`SetParty`).
    pub fn party_add(&mut self, pc: i32) -> i32 {
        let slot = self.with_party(|spcs, chars, _| spcs.party_add(pc, chars));
        self.combat.set_party(self.spcs.party());
        slot
    }

    /// The console's `invite_party` in a field or dungeon, where the party
    /// is built only with the area: `id` registered (`EntrySpc`) with
    /// `partyFlag` 1 and given the first free slot, so that the next area's
    /// `Reboot` builds it beside Kite. The slot, or -1 (no free slot, or
    /// the registry full).
    pub fn invite_next_area(&mut self, id: i32) -> i32 {
        let Some(s) = self.spcs.member_id.iter().position(|&m| m == -1) else { return -1 };
        let Ok(i) = usize::try_from(self.spcs.entry_spc(id)) else { return -1 };
        self.spcs.registry[i].party_flag = 1;
        self.spcs.member_id[s] = id & 0xffff;
        self.spcs.member_char[s] = Some(self.spcs.registry[i].id);
        self.spcs.num += 1;
        s as i32
    }

    /// `party_remove pc` (case 78, 0x001af2c8) and the field UI's
    /// `DelMember(slot)` (as `-slot`): `ccParty::DelMember` (gcmn
    /// 0x0059cf60) of the character's slot, of slot `-pc`, or of every
    /// slot but Kite's (-3): the slot emptied and `disbandSpc` (the
    /// registry's `partyFlag` 0, the character's -1, or -2 when it had been
    /// recalled). The member's AI then keeps to itself (`partyFlag` not 1);
    /// a recalled one goes (`ccFellow::Main`'s exit, [`Self::fellows_gone`]).
    pub fn party_remove(&mut self, pc: i32) {
        self.with_party(|spcs, chars, _| spcs.party_remove(pc, chars));
        self.combat.set_party(self.spcs.party());
    }

    /// `ccSpcChar::TransferOut` on party character `code` (0 Kite), as the
    /// Chaos Gate's leave calls it for each member (the warp out): act 12,
    /// the body off the collision, off the command list unless manual.
    /// False when there is no such character here.
    pub fn transfer_out(&mut self, code: i32) -> bool {
        use crate::party::SpcChars as _;
        self.with_party(|_, chars, hits| match chars.spc(code) {
            Some(mut c) => {
                c.transfer_out(hits);
                true
            }
            None => false,
        })
    }

    /// A member whose `ccFellow::Main` set its exit (`partyFlag` -2 and
    /// away: a recalled member dismissed) leaves the world as the fellow
    /// task's end has it (`ccThFellowNN`, gcmn 0x0041ed60: the frame after,
    /// the task wakes to `exitFlag` and ends instead of running Main):
    /// `expulsionSpc(listNum)` (gcmn 0x005a1070: out of its slot) and the
    /// task's delete (its registry slot freed unless it is a member again:
    /// `DelSpc`), its frames and its draw stopped.
    fn fellows_gone(&mut self) {
        use piney_battle::fellow::flag::EXIT;
        let gone: Vec<(i32, usize)> = self
            .combat
            .members
            .iter()
            .copied()
            .filter(|&(id, w)| id != 0 && self.combat.scene.chars[w].spc_char.flags & EXIT != 0)
            .collect();
        for (id, w) in gone {
            let pf = (((self.combat.scene.chars[w].party_flag & 7) << 5) as i8) >> 5;
            self.with_party(|spcs, chars, _| {
                if let Some(list_num) = spcs.find(id) {
                    spcs.expulsion(list_num, pf, chars);
                }
            });
            self.combat.members.retain(|m| m.1 != w);
            self.combat.cast.actors.remove(&w);
            self.combat.set_party(self.spcs.party());
        }
    }

    /// `Host::pc`: `pc_act`, `pc_mode`, `pc_turn` and `pc_face` of a party
    /// character (party.rs) over the battle's characters. True when done;
    /// the walks, `pc_command` and the puts are the event host's.
    pub fn pc_command(&mut self, c: piney_event::host::PcCommand) -> bool {
        use piney_event::host::PcCommand;
        match c {
            PcCommand::Act { pc, act } => self
                .with_party(|spcs, chars, hits| crate::party::pc_act(spcs, chars, hits, i32::from(pc), i32::from(act))),
            PcCommand::Mode { pc, param } => {
                self.spcs.pc_mode(i32::from(pc), i32::from(param));
                true
            }
            PcCommand::Turn { pc, dirc, chg } => self.ev_party(|p| p.pc_turn(pc, dirc, chg)),
            PcCommand::Command { pc, on } => self.ev_party(|p| p.pc_command(pc, on)),
            PcCommand::WalkPos { pc, x, y, z, run } => self.ev_party(|p| p.pc_walk_pos(pc, x, y, z, run)),
            PcCommand::WalkDir { pc, rot, dist } => self.ev_party(|p| p.pc_walk_dir(pc, rot, dist)),
            PcCommand::WalkMarker { pc, marker } => self.ev_party(|p| p.pc_walk_marker(pc, marker)),
            PcCommand::WalkChar { pc, ty, code, rot, dist } => {
                self.ev_party(|p| p.pc_walk_char(pc, ty, code, rot, dist))
            }
            PcCommand::Put { pc, x, y, z } => self.ev_party(|p| p.pc_put(pc, x, y, z)),
            PcCommand::PartyPut { pc, x, y, z } => self.ev_party(|p| p.party_put(pc, x, y, z)),
            PcCommand::PutMarker { pc, marker } => self.ev_party(|p| p.pc_put_marker(pc, marker)),
            PcCommand::PartyPutMarker { pc, marker } => self.ev_party(|p| p.party_put_marker(pc, marker)),
            PcCommand::Face { pc, ty, code, chg } => self.ev_party(|p| p.pc_face(pc, ty, code, chg)),
            _ => false,
        }
    }

    /// An event's NPC instruction on one of its NPCs
    /// ([`crate::field_npcs`]), as the town does it: the class's event mode
    /// first (`npc_act`, `npc_walk_*`, a gradual turn), else a put or an
    /// instant turn here. A marker is the event position of that number.
    pub fn npc_command(&mut self, c: piney_event::host::NpcCommand) -> bool {
        use piney_event::host::NpcCommand;
        let code = match c {
            NpcCommand::Act { npc, .. }
            | NpcCommand::WalkPos { npc, .. }
            | NpcCommand::WalkDir { npc, .. }
            | NpcCommand::WalkMarker { npc, .. }
            | NpcCommand::WalkChar { npc, .. }
            | NpcCommand::PutMarker { npc, .. }
            | NpcCommand::Put { npc, .. }
            | NpcCommand::Turn { npc, .. }
            | NpcCommand::Face { npc, .. } => i32::from(npc),
        };
        let target = match c {
            NpcCommand::Face { ty, code, .. } => self.char_pos(ty, code),
            _ => None,
        };
        let marker = match c {
            NpcCommand::PutMarker { marker, .. } | NpcCommand::WalkMarker { marker, .. } => self
                .event_entries
                .2
                .iter()
                .take(16)
                .find(|p| p.num == i32::from(marker))
                .map(|p| ([p.pos[0], p.pos[1], p.pos[2], ONE], p.dirc)),
            _ => None,
        };
        let Some(n) = self.npcs.by_code_mut(code) else { return false };
        let n = n.npc_mut();
        if n.command(&c, marker, target) {
            return true;
        }
        let ch = n.char_mut();
        match c {
            NpcCommand::Put { x, y, z, .. } => {
                let s = |v: i16| ee::mul(ee::from_int(i32::from(v)), 0x4120_0000);
                ch.pos = [s(x), s(y), s(z), ONE];
                true
            }
            NpcCommand::PutMarker { .. } => {
                let Some((pos, rot)) = marker else { return false };
                ch.pos = pos;
                ch.dirc[2] = ee::deg2rad(ee::rad2deg(rot));
                true
            }
            NpcCommand::Turn { dirc, chg: 0, .. } => {
                ch.dirc[2] = ee::deg2rad(dirc);
                true
            }
            NpcCommand::Face { chg: 0, .. } => {
                let Some(to) = target else { return false };
                ch.dirc[2] = ee::deg2rad(crate::event::dirc_to(ch.pos, to));
                true
            }
            _ => false,
        }
    }

    /// `trans_on` / `trans_off` on an event NPC (type 3 or 4): `ccChar`
    /// +0x90, `ccChar::Draw`'s fade as the camera comes close.
    pub fn set_trans(&mut self, ty: i16, code: i16, on: bool) -> bool {
        let bit = 1u32.checked_shl(ty as u32).unwrap_or(0);
        if bit & 0x18 == 0 {
            return false;
        }
        match self.npcs.by_code_mut(i32::from(code)) {
            Some(n) => {
                n.npc_mut().char_mut().trans_dist = on;
                true
            }
            None => false,
        }
    }
}

/// What the event camera looks up outside the towns: the party by code,
/// Kite as the leader; the field's event positions are not ported.
impl crate::evcam::Scene for FieldWorld {
    fn char_pos(&self, ty: i32, code: i32) -> Option<V4> {
        FieldWorld::char_pos(self, ty as i16, code as i16)
    }
    fn leader_pos(&self) -> V4 {
        self.player.body.pos
    }
    fn marker_pos(&self, _marker: i16) -> Option<V4> {
        None
    }
    fn player_dirc(&self) -> V4 {
        self.player.body.dirc
    }
}

/// What `ccThGameCtrl` calls outside the command lists, over the battle:
/// `CheckOperate`, Kite's `ccSkillCheck` and `ccSkillRequest`, and
/// `ccCtrlRecoveryReq`'s heals.
struct CtrlHost<'a> {
    combat: &'a mut Combat,
    save: &'a mut SaveState,
    hits: &'a mut crate::hit::Hits,
    area: i32,
    kite: usize,
}

impl talk::Host for CtrlHost<'_> {
    fn check_operate(&mut self, n: i32) -> bool {
        self.save.check_operate(n)
    }
    fn skill_check(&mut self) -> i32 {
        let c = &self.combat;
        let act = c.scene.chars[self.kite].spc_char.act_num;
        c.skills.borrow().check(&c.data.t, &c.scene, self.kite, act)
    }
    fn skill_request(&mut self, kind: crate::entry::Kind, code: i32) {
        let t = match kind {
            crate::entry::Kind::Spc => self.combat.who(code),
            _ => usize::try_from(code).ok(),
        };
        self.combat.skill_request(self.kite, t, 1);
    }
    fn recovery(&mut self) {
        let mut reqs = std::mem::take(&mut self.combat.recovery);
        let mut heals = Vec::new();
        {
            let c = &self.combat;
            reqs.ctrl(
                |ch| c.scene.listed(ch) && c.scene.chars[ch].cond[piney_battle::param::cond::DEAD] == 0,
                |ch, n| heals.push((ch, n)),
            );
        }
        self.combat.recovery = reqs;
        let bounds = Combat::bounds(self.area, self.hits);
        for (ch, n) in heals {
            self.combat.entry_affect_with(ch, Some(ch), 7, [n, 0, 0], self.hits, bounds);
        }
    }
}

/// The field world's values a frame of the battle's tasks reads.
struct TaskInfo {
    scene: Scene,
    wm: WorldMan,
    menu_type: i32,
    menu_forbid: bool,
    cmnd_target: bool,
    count: u32,
    effect_sw: bool,
}

/// What a frame of the battle's tasks reads of the area: its collision,
/// the camera, the save, the scene, the menus; in a dungeon the floor's 2D
/// map (`WORLD_MAN::Get2DMapPtr`) and the room's window (`Get2DMapInfo`
/// for `game.block`).
fn tasks<'a>(
    place: &'a mut Place,
    camera: &'a mut Camera,
    save: &'a mut piney_data::save::SaveData,
    pad: CamPad,
    i: &TaskInfo,
) -> combat::Tasks<'a> {
    let dungeon = match place {
        Place::Field(f) => f.dungeon_pos().unwrap_or([0; 4]),
        _ => [0; 4],
    };
    let (hits, map2d, map2d_info) = match place {
        Place::Field(f) => (&mut f.hits, Vec::new(), [0; 3]),
        Place::Story(m) => (m.hits_mut(), Vec::new(), [0; 3]),
        Place::Dungeon(d) => {
            let (m, n) = (d.map_2d(), d.map_2d_info(i.scene.block));
            (&mut d.hits, m, n)
        }
    };
    let event_area = hits.event_area;
    let puppet_show = camera.puppet_show;
    combat::Tasks {
        hits,
        camera,
        pad,
        save,
        scene: i.scene,
        wm: i.wm,
        menu_type: i.menu_type,
        menu_forbid: i.menu_forbid,
        puppet_show,
        cmnd_target: i.cmnd_target,
        count: i.count,
        event_area,
        dungeon,
        map2d,
        map2d_info,
        path_map: false,
        effect_sw: i.effect_sw,
        spcs: None,
    }
}

/// The base type's bit the event's `no_active` looks for in a gimmick
/// (`lui 0x10`, INF main 0x001a8680): a Gott statue's (`ItemIdolMenu`).
const IDOL_TYPE: i32 = 1 << 20;

/// `ccCheckActiveObject()` (gcmn 0x0042df70): no enemy and no magic
/// circle of the entry control's switched on (`objFlag`).
fn no_active(c: &Combat) -> bool {
    let on = |i: usize| match c.ctrl.objs.get(i) {
        Some(piney_battle::entry::Obj::Enemy) => c.foes.get(i).and_then(Option::as_ref).is_some_and(|e| e.obj_flag),
        _ => c.ctrl.entry_obj(i).is_some_and(|o| o.obj_flag),
    };
    !c.ctrl.list(piney_battle::entry::Kind::Enemy).into_iter().any(on)
        && !c.ctrl.list(piney_battle::entry::Kind::Circle).into_iter().any(on)
}

/// `ccCheckActiveObject(floor, block)` (gcmn 0x0042e010): no enemy and no
/// magic circle of the entry control's lists belongs to that floor and
/// block, switched on or not (`DUNGEON::SetDoor` opens a room's doors
/// then).
fn room_clear(c: &Combat, floor: i32, block: i32) -> bool {
    let here = |i: usize| {
        let ent = match c.ctrl.objs.get(i) {
            Some(piney_battle::entry::Obj::Enemy) => c.foes.get(i).and_then(Option::as_ref).map(|e| e.ent),
            _ => c.ctrl.entry_obj(i).map(|o| o.ent),
        };
        ent.is_some_and(|e| e.floor == floor && e.block == block)
    };
    !c.ctrl.list(piney_battle::entry::Kind::Enemy).into_iter().any(here)
        && !c.ctrl.list(piney_battle::entry::Kind::Circle).into_iter().any(here)
}

/// A symbol's light at `p` (`ccGimSymbol`'s constructor): `ccLight(4, 1)`,
/// an omni light of priority 1, before the group's others; `farDownStart`
/// (+0xc4) 0, `farDownEnd` (+0xc8) 1000 and its square (+0xcc) 1000000, so
/// it fades from the flame to nothing 1000 away; `ccSetColor(color,
/// 0x7fff, 1.0)`, a fire's orange (each byte over 255, red the low one).
/// `symMain` sets the intensity each frame.
fn symbol_light(p: V4, intensity: F) -> crate::town::Light {
    crate::town::Light {
        kind: 4,
        pos: glam::Vec3::new(ee::f(p[0]), ee::f(p[1]), ee::f(p[2])),
        dir: glam::Vec3::ZERO,
        colour: glam::Vec3::new(1.0, 127.0 / 255.0, 0.0),
        intensity: ee::f(intensity),
        far_start: 0.0,
        far_end: 1000.0,
        radius: [0.0; 2],
        priority: 1,
    }
}

/// A spring's or the rays' `ccOmniLight` (`ccLight(4, 1)`) as its frame
/// left it: at its matrix's place, `ccSetColor`'s RGB (red the low byte,
/// each over 255), its intensity and fall-off.
fn omni_light(o: &piney_battle::prim::OmniLight) -> crate::town::Light {
    let p = o.matrix[3];
    let rgb = |k: u32| ((o.rgb >> (8 * k)) & 0xff) as f32 / 255.0;
    crate::town::Light {
        kind: 4,
        pos: glam::Vec3::new(ee::f(p[0]), ee::f(p[1]), ee::f(p[2])),
        dir: glam::Vec3::ZERO,
        colour: glam::Vec3::new(rgb(0), rgb(1), rgb(2)),
        intensity: ee::f(o.intensity),
        far_start: ee::f(o.far_start),
        far_end: ee::f(o.far_end),
        radius: [0.0; 2],
        priority: 1,
    }
}

#[cfg(test)]
mod symbol_light_tests {
    use super::*;
    use crate::chara::light_matrix;
    use crate::town::TownLights;

    /// The symbol's light at 1.0: half its orange (doubled, as VU1 does)
    /// on a model 500 below it, none 1000 or more away. It once reached
    /// the whole field, flickering with `symMain`'s random intensity.
    #[test]
    fn a_symbol_lights_only_near_it() {
        let at = |d: f32| {
            let town = TownLights {
                ambient: glam::Vec3::ZERO,
                lights: vec![symbol_light([0, 0, ee::k(d), ONE], ONE)],
                fog: None,
            };
            light_matrix(&town, glam::Mat4::IDENTITY, false).colours[2]
        };
        let near = at(500.0);
        assert!((near[0] - 1.0).abs() < 1e-5 && (near[1] - 127.0 / 255.0).abs() < 1e-5 && near[2] == 0.0, "{near:?}");
        assert_eq!(at(1000.0), [0.0; 3]);
        assert_eq!(at(2500.0), [0.0; 3]);
    }
}

/// The boss's own layer (`OnMagicAtk`'s `new ccLayer`, `Init(1,
/// sysLayer's view)`): priority 1, over the map and under the characters.
const REVERSE_LAYER: i16 = 1;

/// `ccBufferReverce::MakePacket` (main 0x00106890): a sprite over the whole
/// view (`SetFrame(0, 0, 512, 384, ..)`), white (0x80ffffff), blended
/// `(Cs - Cd) FIX + 0` with FIX 128 (ALPHA 0x80_000000a4): 255 less what
/// is there, the picture inverted; the alpha test failing into the frame
/// buffer only (TEST 0x1001), no depth test and no depth write (ZBUF's
/// ZMSK).
fn reverse_packet() -> piney_draw::Cmd {
    use piney_draw::{
        AlphaFail, AlphaTest, Blend, BlendAlpha, BlendColour, Cmd, Compare, Depth, DrawState, Prim, PrimKind, Rgba,
        Scissor, Vertex, ZTest,
    };
    let state = DrawState {
        blend: Some(Blend {
            a: BlendColour::Source,
            b: BlendColour::Dest,
            c: BlendAlpha::Fix,
            d: BlendColour::Zero,
            fix: 128,
        }),
        alpha_test: AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::FbOnly },
        depth: Depth { test: ZTest::Always, write: false },
        texture: None,
        scissor: Scissor::FULL,
    };
    let v = |x: f32, y: f32| Vertex { x, y, z: 0, u: 0.0, v: 0.0, rgba: Rgba([0xff, 0xff, 0xff, 0x80]) };
    Cmd::Prim(Prim { kind: PrimKind::Sprite, gouraud: false, state, verts: vec![v(0.0, 0.0), v(512.0, 384.0)] })
}

/// `ccLattice::SendPacket` (gcmn 0x0057a0e0) of a trail (the cross's, a
/// weapon's): each vertex through the view's `RotTransPers`,
/// gouraud-shaded and blended `(Cs - Cd) As + Cd` (ALPHA 0x44),
/// depth-tested GREATER with no depth write (TEST 0x73001, as the rays').
/// A vertex with ADC set draws no triangle: each strip's first pair, and
/// (`MakePacket`) one behind the eye or more than 640 x 576 pixels off the
/// display's corner; so the strip goes as the triangles it draws.
pub(crate) fn trail_packet(strip: &[crate::lattice::StripVertex], ws: &[V4; 4]) -> Option<piney_draw::Cmd> {
    use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
    use piney_draw::{
        AlphaFail, AlphaTest, Blend, Cmd, Compare, Depth, DrawState, Prim, PrimKind, Rgba, Scissor, Vertex, ZTest,
    };
    if strip.len() < 3 {
        return None;
    }
    let state = DrawState {
        blend: Some(Blend::MIX),
        alpha_test: AlphaTest::On { method: Compare::Never, reference: 0, fail: AlphaFail::RgbOnly },
        depth: Depth { test: ZTest::Greater, write: false },
        texture: None,
        scissor: Scissor::FULL,
    };
    let verts: Vec<(Vertex, bool)> = strip
        .iter()
        .map(|v| {
            let p = ee::rot_trans_pers(ws, v.pos);
            let (dx, dy) = (p[0].wrapping_sub(XYOFFSET_X), p[1].wrapping_sub(XYOFFSET_Y));
            // MakePacket's own clip: z negative, or more than 640 x 576
            // pixels off the offset (XYOFFSET, the display's corner).
            let off = p[2] < 0 || dx.abs() > 10240 || dy.abs() > 9216;
            let vx =
                Vertex { x: dx as f32 / 16.0, y: dy as f32 / 16.0, z: p[2] as u32, u: 0.0, v: 0.0, rgba: Rgba(v.rgba) };
            (vx, v.restart || off)
        })
        .collect();
    let mut tris = Vec::new();
    for k in 2..verts.len() {
        if !verts[k].1 {
            tris.extend([verts[k - 2].0, verts[k - 1].0, verts[k].0]);
        }
    }
    (!tris.is_empty()).then_some(Cmd::Prim(Prim { kind: PrimKind::Triangles, gouraud: true, state, verts: tris }))
}

/// What the effects draw from after the frame.
pub struct FxView<'a> {
    pub scene: &'a piney_battle::scene::Scene,
    pub ctrl: &'a piney_battle::entry::EntryCtrl,
    pub cast: &'a combat::Cast,
    pub camera: &'a Camera,
    pub bounds: piney_battle::kite::MapBounds,
    pub kite: Option<usize>,
}

/// The field's effects as the world holds them: the tasks the battle's
/// frame runs ([`combat::FxTasks`]) and their draw, after the characters.
pub trait FieldFx {
    fn tasks(&mut self) -> &mut dyn combat::FxTasks;
    fn draw(&mut self, _view: &FxView, _ctx: &mut Ctx) {}
    /// Whether it draws the magic portals (then the world does not).
    fn draws_circles(&self) -> bool {
        false
    }
    /// The sounds the effects asked for since the last call, in order:
    /// `ccSeOn3D(se, pos)`, `ccSeOn3DNote(se, pos, note)` and
    /// `ccSeOnNote(se, note)` (no place).
    fn take_sounds(&mut self) -> Vec<(i32, Option<V4>, Option<i8>)> {
        Vec::new()
    }
    /// The screen flashes the effects asked for since the last call
    /// (`scFadeDef->EntryFlash(time, colour, 0, 0, 512, 384)`).
    fn take_flashes(&mut self) -> Vec<(i32, u32)> {
        Vec::new()
    }
    /// The screen shakes the effects asked for since the last call
    /// (`cameraShake(power, cycle, time, dirc)`).
    fn take_shakes(&mut self) -> Vec<[i32; 4]> {
        Vec::new()
    }
    /// The screen noise the effects asked for since the last call
    /// (`ccMenu->noiz->SetNoizBs(bs)`: the spells' blasts).
    fn take_noises(&mut self) -> Vec<i32> {
        Vec::new()
    }
    /// A story map's `ccEff` sprites of the frame (area 15's lens flare),
    /// after its draw.
    fn story_sprites(&mut self, _sprites: &[StorySprite], _camera: &Camera, _ctx: &mut Ctx) {}
    /// A field's weather and ambient pictures of the frame, after its
    /// draw ([`crate::field_ambient`]): the `ccEff` sprites on their
    /// layers, and with `fresh` (a frame the tasks ran, not one drawn again
    /// while they sleep) the smoke puffs to start (`effSmoke`). The models,
    /// the 2D smoke, the sounds and the flashes are the world's.
    fn field_ambient(&mut self, _ops: &[crate::field_ambient::Op], _fresh: bool, _camera: &Camera, _ctx: &mut Ctx) {}
    /// The lights the effects have put in the light group (the boss's
    /// `ccBossEffLight`), which the characters are lit by with the map's.
    fn lights(&self) -> Vec<crate::town::Light> {
        Vec::new()
    }
    /// What the effects hold now (for tests).
    fn census(&self) -> FxCensus {
        FxCensus::default()
    }
}

/// What the effects hold at a moment ([`FieldFx::census`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FxCensus {
    /// The ids of the live `ccEffect` slots.
    pub effects: Vec<i16>,
    /// The live particles.
    pub particles: usize,
    /// The live generators a party character's `armsEffectSW` holds
    /// (`StartArmsEffect`'s, along a weapon).
    pub weapon_generators: usize,
    /// The scene index each live generator not being ended (`killFlag`
    /// 0 or 1) follows the place of (a condition effect's, a spell's...).
    pub char_generators: Vec<u32>,
    /// The live generators an idol's or a `ccGimEtc`'s `effsw` switches
    /// (`effStatueOfGod`'s glow, `effBossRoomEntrance`'s), not being ended.
    pub effsw_generators: usize,
    /// `ccDamUprStr`'s lines still showing (the characters' numbers and
    /// words), in `flyFont`'s codes.
    pub fly_fonts: Vec<Vec<u8>>,
    /// The lines `AddStr` put up this frame (alphaCnt 23: one `CtrlAll`
    /// since), with the scene index of the character they are over.
    pub new_fly_fonts: Vec<(u32, Vec<u8>)>,
    /// The boss effects' draws of the frame, by the object each drew.
    pub boss_draws: Vec<String>,
}

impl FieldFx for combat::NoFx {
    fn tasks(&mut self) -> &mut dyn combat::FxTasks {
        self
    }
}
