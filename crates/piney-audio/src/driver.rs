//! The EE side of the sound system: what `sndlib.cpp` and `sndSYS.CPP`
//! (`INF SLUS_202.67`) decide before anything reaches the IOP - which
//! messages a sound effect sends, which bank and sequence a jukebox row
//! loads and plays, the port volumes the options give, and the per-frame
//! fades. `docs/engine/sound.md` has the rules with their addresses.

use piney_data::sound::{SeTbl, SqLoad, SqTbl, Tables, WaveData};
use piney_data::tables::voice::{VoiceData, VoiceTable};

use crate::se3d::{self, Listener, V4};

/// `ccSound`'s state that matters to what is heard.
#[derive(Clone, Debug)]
pub struct Driver {
    /// The disc's volume: its code's square root in the distances
    /// ([`piney_data::libm::sqrtf_on`]).
    pub volume: piney_data::volume::Volume,
    /// `saveData.mainVol`, `seVol`, `bgmVol`: 0..256, 256 by default
    /// (`ccSound::ccSound`, 0x00180ed0).
    pub main_vol: i32,
    pub se_vol: i32,
    pub bgm_vol: i32,
    /// `ccSound.sqtbl`: the loaded bank's three sequences.
    pub sqtbl: [SqTbl; 3],
    /// `ccSound.sqNum`: sequences loaded.
    pub sq_num: usize,
    /// `sqStatus[3]`: -1 loaded, 0 stopped, 1 playing.
    pub sq_status: [i32; 3],
    /// `hdSynPortVol[4]`: what `ccPortVolSet` last set per port.
    pub port_vol: [u16; 4],
    /// `ccSound.fade[3]`, by sequencer.
    pub fade: [Fade; 3],
    /// `ccSound.changeBgmVol` / `changeSeVol` and `sdRemote[0]`: applied
    /// on the next frame.
    change_bgm: bool,
    change_se: bool,
    change_main: bool,
    /// The jukebox row playing (`WaveData.NO`), -1 before any.
    pub wave: i32,
    /// A jukebox change waiting for its fade (`BgmRead`'s 10 frames).
    pending: Option<(WaveData, u32)>,
    /// `ccSound.gameStart` (+0x17): set by the desktop's, the board's and
    /// the field's set-up, cleared by every mode change
    /// (`ccSound::gameInterrupt`). The sound task fades with `ccFade` while
    /// it is set and with `ccSceneFade` while it is not.
    pub game_start: bool,
    /// `ccSound` +0xd0: the skill words `ccWordsPlay` queued for
    /// `skillVoicePlay`, (`charTbl` row, skill id, bit 0 of the skill's type
    /// `ccGetSkillParam(sid)+0x2c`), -1 free; +0xec how many.
    pub words: [(i16, i16, bool); 4],
    pub words_num: i32,
    /// `game.area`, which the sound task reads before the skill words (1 a
    /// field, 2 a dungeon); 0 until the runtime says.
    pub game_area: i32,
    /// `ccSnd +0x134`: all sound off at the end of the next frame.
    sound_off: bool,
    /// `saveData.voice` (+0x842c): the English files and tables when set.
    pub voice_english: bool,
    /// `saveData.parodyFlag` (+0x842b).
    pub parody: bool,
    /// `talkNum` by `charTbl` row (`ccSaveData` +0x220c; 18-20 in the
    /// extension): Mutation's `ccVoiceRequest` asks it whether Mia's
    /// English lines come from `MIAE.BIN`
    /// ([`piney_data::tables::voice::VoiceAlt`]).
    pub talk_num: [i8; 21],
    /// `ccSnd +0x136`: `evVoicePlay` has slots to send.
    voice_flag: bool,
    /// `ccSnd +0x140`: the two voice slots.
    pub voice_slots: [VoiceSlot; 2],
    /// `vdRequest`: the row last asked for, `None` for a NULL table entry.
    vd_request: Option<VoiceData>,
    /// `voiceFile`: which name of `evVoiceFile` / `evVoiceFileE`.
    voice_file: usize,
    /// The disc's voice tables and file names (Infection's until
    /// [`Driver::set_voice`]).
    voice: &'static piney_data::tables::voice::Voice,
    /// `ccSnd +0xf0`: the [`SqContext::number`] of the last bank
    /// `ccSndSQLoad` loaded, which `ccSndBgmCtrl` switches on.
    pub context: i32,
    /// `ccSnd +0x104`: [`Pick::play_type`] of the last load.
    pub play_type: i8,
    /// `ccSnd +0x5f`: [`Pick::battle_bank`] of the last load.
    pub battle_bank: bool,
    /// `ccSnd +0x105`: [`Pick::scene_mode`] of the last load.
    pub scene_mode: i8,
    /// `ccSnd +0x138`: the event instruction `sound 10`
    /// (`ccSndEvRequest(10)`, 0x0017e260) holds the next `ccSndBgmCtrl`.
    pub hold: bool,
    /// `ccSnd +0x137`: `ccSndBgmCtrl` has run since the last load.
    pub bgm_started: bool,
    /// `ccSnd +0x60`: the battle music (sequence 1) is on.
    pub battle_music: bool,
    /// `ccSnd +0x61 bgmStopFlag`: set while `ccSndSQLoad` loads (and after
    /// one that found no bank); the fades and `bgmChange` wait.
    bgm_stop: bool,
    /// `game.inBattle` (+0x58), which `bgmChange` reads each frame.
    pub in_battle: bool,
    /// `pgRideFlag` (0x00378cdc) is 1: riding a Grunty, no battle music.
    pub riding: bool,
    /// `ccSnd +0x63 gateHack`: the gate hack's music ([`GateHack`]).
    pub gate_hack: GateHack,
    /// `ccSound.loopID[8]` (+0x65): the looping sound effects' slots, -1
    /// free, else the slot's own index (`ccSeOn3DLoop`, `ccSeOffLoop`);
    /// all -1 from the constructor and after every `ccSndSQLoad`
    /// (`initAfterLoad`, 0x001830c0).
    pub loop_id: [i8; 8],
    /// `ccSnd +0x133`: TOBJ's hum plays (`tobjSeLoopStart`); cleared by
    /// the constructor, `initBeforeLoad` and the desktop's
    /// `ccSndChangeData`.
    pub tobj_loop: bool,
    /// `looptest` (0x003789dc): the hum's `loopID` slot (and the canals',
    /// [`crate::scene`]).
    pub looptest: i32,
    /// `ccSnd +0x132`: Mac Anu's canals play (`waterTest`); cleared by the
    /// constructor and `ccSndBgmCtrl` in Mac Anu.
    pub water_loop: bool,
    /// `ccSnd +0x108`: `bgmChurch` has started its music, or `bgmBreed`'s
    /// breeder tune is on; cleared by every `ccSndSQLoad`.
    pub scene_bgm: bool,
    /// `ccSnd +0x10c`: the block `bgmChurch` last saw.
    pub church_block: i32,
    /// `ccSnd +0x114`, +0x120: the breeder's position, read once.
    pub breeder: Option<se3d::V4>,
    /// `game.status` is 5 (The World): from the area's arrival
    /// ([`Driver::game_area`] set) to the next mode change.
    pub in_world: bool,
}

/// A voice slot (`ccSnd +0x140`, two words).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceSlot {
    /// -1.
    Free,
    /// 0x80e0, set by `ccMesVoicePlay`: send the requested line.
    Play,
    /// 0x120, set by `ccEvVoiceStop`: stop channel 0.
    Stop,
}

/// `ccSnd +0x63 gateHack`, the gate hack's hold on the music.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateHack {
    /// 0.
    Off,
    /// 1: fading out under the menu (`ccSndGateHack(0)`).
    Out,
    /// 2: to be brought back on the next frame (`ccSndGateHack(1)`).
    Back,
    /// 3: brought back, fading in.
    Faded,
}

/// `vBank` as `evVoicePlay` (0x0017eca0) fills it for `sewordCmd(0x80e0,
/// &vBank)`: SEWORDS.IRX's `wordPlay` opens `file`, seeks to `ofs` and
/// streams `size` bytes into core 0's sound-data input at `vol`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceCmd {
    /// The name as sent (`cdrom0:\VOICE_E\EVVOL1_E.BIN`).
    pub file: &'static str,
    pub ofs: i32,
    pub size: i32,
    /// `BVOLL` in the high half, `BVOLR` in the low.
    pub vol: u32,
}

/// `evVoicePlay`'s volume: 0x6fff left and right, whatever the options say.
pub const VOICE_VOL: u32 = 0x6fff_6fff;

/// `SQ_FADE` (20 bytes).
#[derive(Clone, Copy, Debug, Default)]
pub struct Fade {
    /// Bit 0: set the port each counted frame; bit 1: stop at the end.
    pub sw: u8,
    pub end_vol: u16,
    pub rate: i32,
    pub time: i32,
    pub volume: i32,
}

/// What the driver asks of the IOP side, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// MIDI bytes for synthesizer port 0 (`sceMSIn_PutMsg`).
    Msg(Vec<u8>),
    /// `sceHSyn_SetVolume(port, vol)` (`ccSndCmd(0xb0 | port)`).
    PortVolume(usize, u16),
    /// Core 1's master volume (`sdCommand` case 1).
    Master(u16),
    /// Load the bank a loading row names (`ccSndCmd(0x9310)`: the IOP stops
    /// the sequencers, silences every port and streams the samples in).
    Load(SqLoad),
    /// All sound off on every port (`ccSndCmd(0x140)`).
    AllSoundOff,
    /// Give ports `i + 1` the bank and sequencer `i` its sequence
    /// (`ccSndCmd(0x9051 + i)`, `0xa1 + i`, `0x40 + i`).
    Seq(usize),
    /// Play sequencer `n` from the start (`ccSndCmd3(0x110 | n)`).
    Play(usize),
    /// Stop sequencer `n` (`ccSndCmd3(0x20 | n)`).
    Stop(usize),
    /// A voice line to SEWORDS's channel 0 (`sewordCmd(0x80e0, &vBank)`).
    Voice(VoiceCmd),
    /// Stop SEWORDS's channel 0 (`sewordCmd(0x120, 0)`): `BgmStop`,
    /// `BgmClose`, `BgmQuit`, input volume 0.
    VoiceStop,
    /// SNDBASE's `area` (`ccSndCmd3(0x200, n)`): the context `ccSndSQLoad`
    /// loads, which SNDBASE only prints.
    Area(i32),
    /// SNDBASE's `playtype` (`ccSndCmd3(0x300, t)`), which its `bgmChange`
    /// reads.
    PlayType(i32),
    /// SNDBASE's `bgmChange` (`ccSndCmd3(0x130, 0)`, SNDBASE 0x7e4): with
    /// play type 0 or 1, sequencer 1 starts at sequencer 0's position when
    /// 0 plays alone, or 0 at 1's when 1 does; with play type 2, sequencer
    /// 1 starts from its start unless it plays.
    BgmChange,
}

/// `ccSndSQLoad(n)`'s contexts: those whose bank is a fixed row, and those
/// whose row the world picks, with what picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqContext {
    /// 0: the board (`sqDataToppage`, `ccSetupToppage`).
    Toppage,
    /// 1: `sqDataDesktop` row 0 (the desktop itself loads through the
    /// jukebox's `ccSndChangeData`).
    Desktop,
    /// 7: the title (`sqDataTitle`, `ccSetupDemo`); sequence 0 is its music.
    Title,
    /// 2: a Root Town (`sqDataTown`, `sqVolTblTown`), row
    /// `WORLD_MAN::GetTownType()`, plus 5 while `saveData.crisis`
    /// (+0x6772) is set (`ccSndSQLoad` 0x00182380, 0x001829d4).
    Town { row: u8 },
    /// 3: a field (0x00182260, 0x00182968): `sqDataField[field_type]`,
    /// `sqVolTblField[field_type]` and `playTypeTbl[field_type]`, row `bg`;
    /// `WORLD_MAN::GetFieldType()` (`fieldtype`, +0x10) and `GetBG()`
    /// (`bgnum`, +0xc), the area's field type and weather. With Piros in
    /// the party (`checkPartyMenberNum(8) != -1`: character 8 in one of
    /// `ccPartyManager`'s three member slots) row 0 of the twelfth table,
    /// `piroshi`, whatever the field.
    Field { field_type: u8, bg: u8, piros: bool },
    /// 4: a dungeon (0x00182324, 0x00182a40): `sqDataDungeon`,
    /// `sqVolTblDungeon` and `dungeonPlayType`, row
    /// `WORLD_MAN::GetDungeonType()` (`dungeonType[game.dungeon]`).
    Dungeon { dungeon_type: u8 },
    /// 5: an event bank (0x00182494, 0x00182a8c): `sqDataEvent`,
    /// `sqVolTblEvent` and `eventPlayType`, row `game.field` (+0x24, the
    /// story area); `area_prev` is `game.areaPrev` (+0x18), 2 (from a
    /// dungeon) making the play type 1 except in area 48.
    Event { field: u16, area_prev: i32 },
}

impl SqContext {
    /// The number `ccSndSQLoad` takes.
    pub fn number(self) -> i32 {
        match self {
            SqContext::Toppage => 0,
            SqContext::Desktop => 1,
            SqContext::Town { .. } => 2,
            SqContext::Field { .. } => 3,
            SqContext::Dungeon { .. } => 4,
            SqContext::Event { .. } => 5,
            SqContext::Title => 7,
        }
    }

    /// What `ccSndSQLoad`'s two switches pick: the loading row and its
    /// port volumes (`None` past the end of a table - no input the game
    /// makes gets there), and the `ccSnd` fields they set.
    pub fn pick(self, tables: &Tables) -> Pick {
        let mut p = Pick { row: None, play_type: 0, battle_bank: false, scene_mode: 0 };
        let (c, index) = match self {
            SqContext::Title => (&tables.title, 0),
            SqContext::Toppage => (&tables.toppage, 0),
            SqContext::Desktop => (&tables.desktop, 0),
            SqContext::Town { row } => {
                // GetTownType() is 0 in Mac Anu: +0x105 = 1; 3 in the others.
                p.scene_mode = if row % 5 == 0 { 1 } else { 3 };
                (&tables.town, usize::from(row))
            }
            SqContext::Field { field_type, bg, piros } => {
                p.battle_bank = true;
                let (table, index) = if piros { (11, 0) } else { (usize::from(field_type), usize::from(bg)) };
                let Some(c) = tables.field.get(table) else {
                    p.play_type = -1;
                    return p;
                };
                p.play_type = c.play.get(index).copied().unwrap_or(-1);
                (c, index)
            }
            SqContext::Dungeon { dungeon_type } => {
                p.battle_bank = true;
                let c = &tables.dungeon;
                p.play_type = c.play.get(usize::from(dungeon_type)).copied().unwrap_or(-1);
                (c, dungeon_type.into())
            }
            SqContext::Event { field, area_prev } => {
                let c = &tables.event;
                let t = c.play.get(usize::from(field)).copied().unwrap_or(-1);
                // 4: the battle arrangement, play type 0.
                if t == 4 {
                    p.battle_bank = true;
                } else {
                    p.play_type = t;
                }
                if field == 15 {
                    p.scene_mode = 2;
                }
                if field == 48 {
                    p.play_type = 0;
                } else if area_prev == 2 {
                    p.play_type = 1;
                }
                (c, field.into())
            }
        };
        p.row = c.load.get(index).copied().zip(c.vol.get(index).copied());
        p
    }
}

/// What [`SqContext::pick`] finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pick {
    pub row: Option<(SqLoad, [SqTbl; 3])>,
    /// `ccSnd +0x104`, sent to SNDBASE as its `playtype`: which sequences
    /// `ccSndBgmCtrl` starts and how `bgmChange` switches (0 sequence 0; 1
    /// sequences 0 and 2; 2 sequence 2; 3 none; -1 or 4 none).
    pub play_type: i8,
    /// `ccSnd +0x5f`: `bgmChange` switches between sequences 0 and 1 as
    /// battles start and end.
    pub battle_bank: bool,
    /// `ccSnd +0x105`: 1 Mac Anu, 3 another town, 2 story area 15's event
    /// bank (`ccAllSoundOff` then does nothing), else 0.
    pub scene_mode: i8,
}

/// What `ccSetupGameCtrl` (0x00168960) reads to pick an area's bank, as it
/// runs when `game.request[0]` is 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AreaMusic {
    /// `game.area` (+0x14): 0 a town, 1 a field, 2 a dungeon.
    pub area: i32,
    /// `ccGame::CheckSceneReplace()` (0x00167580): `area`, `town`, `field`
    /// or `dungeon` is not its `...Prev` (+0x18, +0x38, +0x3c, +0x40). The
    /// floor and block are not compared.
    pub scene_replaced: bool,
    /// `game.town` (+0x20) and `saveData.crisis` (+0x6772) not 0.
    pub town: u8,
    pub crisis: bool,
    /// `game.field` (+0x24): the story area, 0 for a random one.
    pub field: u16,
    /// `WORLD_MAN::GetEventAreaInfo(game.field)->model` (+0x20) when
    /// `field` is not 0.
    pub field_model: i32,
    /// `WORLD_MAN.fieldtype` (+0x10) and `bgnum` (+0xc): the area's field
    /// type and weather from `SimGenerateCode`.
    pub field_type: u8,
    pub bg: u8,
    /// Piros (character 8) is in the party.
    pub piros: bool,
    /// `WORLD_MAN.dungeonType[game.dungeon]`.
    pub dungeon_type: u8,
    /// `WORLD_MAN.specialRoom` (+0x160): -1 unless the room is a special
    /// one.
    pub special_room: i32,
    /// `game.areaPrev` (+0x18).
    pub area_prev: i32,
}

/// The `ccSndSQLoad` `ccSetupGameCtrl` makes for an area, `None` when it
/// makes none: a town (area 0) loads its bank when the scene changed; a
/// field (1) when the scene changed, its story area's event bank (5) if
/// its `model` is 1, else the field's (3); a dungeon (2) its bank (4) when
/// the scene changed, else the story area's event bank (5) when the room is
/// special.
pub fn setup_context(a: &AreaMusic) -> Option<SqContext> {
    let event = SqContext::Event { field: a.field, area_prev: a.area_prev };
    match a.area {
        0 if a.scene_replaced => Some(SqContext::Town { row: a.town + if a.crisis { 5 } else { 0 } }),
        1 if a.scene_replaced => Some(if a.field != 0 && a.field_model == 1 {
            event
        } else {
            SqContext::Field { field_type: a.field_type, bg: a.bg, piros: a.piros }
        }),
        2 if a.scene_replaced => Some(SqContext::Dungeon { dungeon_type: a.dungeon_type }),
        2 if a.special_room >= 0 => Some(event),
        _ => None,
    }
}

/// What `ccSndBgmCtrl` reads of the world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BgmWorld {
    /// `ccGame::CheckSceneReplace()` (see [`AreaMusic::scene_replaced`]).
    pub scene_replaced: bool,
    /// `WORLD_MAN::GetTownType()`: `game.town`.
    pub town: i32,
    /// `saveData.crisis` (+0x6772) is 1.
    pub crisis: bool,
    /// `saveData.dtBgm` (+0x2237).
    pub dt_bgm: i32,
}

/// What `ccSndBgmCtrl` (0x0017b020) does, from the context of the last
/// bank loaded, its play type, the hold flag and the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BgmPlan {
    /// The sequences it starts, in order, each as `ccSqPlay` does: only
    /// one that is loaded (below `sqNum`) and not playing.
    pub play: &'static [usize],
    /// `ccSnd +0x138` was set (the event instruction `sound 10`): it is
    /// cleared and nothing starts.
    pub held: bool,
    /// `ccSnd +0x137` is set: from the next frame `bgmChange` may switch
    /// to the battle music.
    pub started: bool,
}

/// [`BgmPlan`] as a pure function. `context` is the last loaded
/// [`SqContext::number`] (`ccSnd +0xf0`), `play_type` [`Pick::play_type`],
/// `hold` `ccSnd +0x138`. The cases by context (town 0x0017b064, field and
/// event bank 0x0017b444, dungeon only when the scene changed, desktop
/// 0x0017b7d4, the rest sequence 0 or nothing) are in docs/engine/sound.md
/// ("ccSndBgmCtrl").
pub fn bgm_plan(context: i32, play_type: i8, hold: bool, w: &BgmWorld) -> BgmPlan {
    let started = |play| BgmPlan { play, held: false, started: true };
    let held = BgmPlan { play: &[], held: true, started: true };
    let by_type = |t: i8| match t {
        0 => &[0usize][..],
        1 => &[0, 2],
        2 => &[2],
        _ => &[],
    };
    match context {
        1 if hold => held,
        1 => started(if matches!(w.dt_bgm, 27 | 7) { &[1] } else { &[0] }),
        2 => started(match w.town {
            0 | 2 if w.crisis => &[2, 0],
            1 | 3 => &[2, 0],
            _ => &[0],
        }),
        4 if !w.scene_replaced => BgmPlan { play: &[], held: false, started: false },
        3..=5 if hold => held,
        3..=5 => started(by_type(play_type)),
        6 | 7 => started(&[]),
        _ => started(&[0]),
    }
}

/// Rows of `Wave` whose bank plays its second sequence: `ccSndChangeData`
/// (0x001834e0) tests the row it starts and the row it replaces for these.
pub const SECOND_SEQUENCE: [i32; 3] = [47, 27, 7];

impl Default for Driver {
    fn default() -> Self {
        Driver::new()
    }
}

impl Driver {
    pub fn new() -> Driver {
        let t = SqTbl { midi_port: 0, hd_port: 1, vol: 0 };
        Driver {
            main_vol: 256,
            se_vol: 256,
            bgm_vol: 256,
            sqtbl: [t, SqTbl { midi_port: 1, hd_port: 2, vol: 0 }, SqTbl { midi_port: 2, hd_port: 3, vol: 0 }],
            sq_num: 0,
            sq_status: [-1; 3],
            port_vol: [256, 0, 0, 0],
            fade: [Fade::default(); 3],
            change_bgm: false,
            change_se: false,
            change_main: false,
            wave: -1,
            pending: None,
            game_start: false,
            words: [(-1, -1, false); 4],
            words_num: 0,
            game_area: 0,
            sound_off: false,
            voice_english: false,
            parody: false,
            talk_num: [0; 21],
            voice_flag: false,
            voice_slots: [VoiceSlot::Free; 2],
            vd_request: None,
            voice_file: 0,
            voice: piney_data::tables::voice::of(piney_data::volume::Volume::Inf),
            volume: piney_data::volume::Volume::Inf,
            context: 0,
            play_type: 0,
            battle_bank: false,
            scene_mode: 0,
            hold: false,
            bgm_started: false,
            battle_music: false,
            bgm_stop: false,
            in_battle: false,
            riding: false,
            gate_hack: GateHack::Off,
            loop_id: [-1; 8],
            tobj_loop: false,
            water_loop: false,
            scene_bgm: false,
            church_block: 0,
            breeder: None,
            in_world: false,
            looptest: 0,
        }
    }

    /// `ccSeOn(n)` (0x00179c10): program change then note on, on the row's
    /// channel of port 0.
    pub fn se_on(se: &SeTbl) -> Vec<u8> {
        Driver::se_note(se, se.note)
    }

    /// `ccSeOnNote(n, note)` (0x00179cb0): the same with the note replaced,
    /// clamped to 0..127.
    pub fn se_note(se: &SeTbl, note: i8) -> Vec<u8> {
        let note = note.clamp(0, 127) as u8;
        let ch = se.ch as u8;
        vec![0xc0 | ch, (se.prog as u8) & 0x7f, 0x90 | ch, note & 0x7f, (se.velocity as u8) & 0x7f]
    }

    /// `ccSeOn3D(n, pos)` (0x00179d90), or `ccSeOn3DNote(n, pos, note)`
    /// (0x00179f50) with a `note`: the sound effect at `pos` as heard from
    /// the active camera `cam` (`None` when there is none) - the velocity
    /// from the distance, the pan from the direction, the bend when behind
    /// ([`se3d::se_on_3d`]). Empty when out of reach, or for a note below 0.
    pub fn se_3d(&self, se: &SeTbl, cam: Option<&Listener>, pos: &V4, note: Option<i8>) -> Vec<u8> {
        se3d::se_on_3d(self.volume, se, cam, pos, note)
    }

    /// `ccSeOn3DLoop(n, pos)` (0x0017a140): a looping sound effect in the
    /// first free slot of [`Driver::loop_id`]; the slot (the note's id,
    /// for [`Driver::se_off_loop`]) and the messages, or -1 and nothing
    /// when out of reach or with every slot taken.
    pub fn se_3d_loop(&mut self, se: &SeTbl, cam: Option<&Listener>, pos: &V4) -> (i32, Vec<u8>) {
        se3d::se_on_3d_loop(self.volume, &mut self.loop_id, se, cam, pos)
    }

    /// `ccSeOffLoop(n, id)` (0x0017a620): loop `id`'s note off; its slot
    /// freed.
    pub fn se_off_loop(&mut self, se: &SeTbl, id: i32) -> Vec<u8> {
        se3d::se_off_loop(&mut self.loop_id, se, id)
    }

    /// `tobjSeLoopStart(pos)` (0x0017bf20): TOBJ's hum started in a free
    /// loop slot, once ([`se3d::tobj_se_loop_start`]).
    pub fn tobj_se_loop_start(&mut self, se: &SeTbl) -> Vec<u8> {
        if self.tobj_loop {
            return Vec::new();
        }
        let Some((id, m)) = se3d::tobj_se_loop_start(&mut self.loop_id, se) else { return Vec::new() };
        self.looptest = id;
        self.tobj_loop = true;
        m
    }

    /// `tobjSeLoop(pos, rate)` (0x0017c0d0): the hum's pan and volume
    /// while it plays ([`se3d::tobj_se_loop`]).
    pub fn tobj_se_loop(&self, se: &SeTbl, cam: &Listener, pos: &V4, rate: se3d::F) -> Vec<u8> {
        if !self.tobj_loop {
            return Vec::new();
        }
        se3d::tobj_se_loop(self.volume, se, cam, pos, rate, self.looptest)
    }

    /// `ccPortVolSet(port, vol)` (0x0017ad70): the SE port takes `seVol`
    /// whatever it is given; the music ports `vol * bgmVol >> 8`.
    pub fn port_vol_set(&mut self, port: usize, vol: u16, out: &mut Vec<Command>) {
        let v = match port {
            0 => self.se_vol as u16,
            1..=3 => ((vol as i32 * self.bgm_vol) >> 8) as u16,
            _ => vol,
        };
        self.port_vol[port] = v;
        out.push(Command::PortVolume(port, v));
    }

    /// `ccSetMainVol`, `ccSetSeVol`, `ccSetBgmVol` (0x001794b0...): the
    /// options, 0..256, applied on the next frame - the music and SE ports
    /// by `ccSndChangeOption`, then the master (`(v << 14) - v >> 8`) by
    /// `sdCommand`.
    pub fn set_volumes(&mut self, main: i32, se: i32, bgm: i32, out: &mut Vec<Command>) {
        let _ = out;
        if main != self.main_vol {
            self.main_vol = main;
            self.change_main = true;
        }
        let se = se.clamp(0, 256);
        let bgm = bgm.clamp(0, 256);
        if se != self.se_vol {
            self.se_vol = se;
            self.change_se = true;
        }
        if bgm != self.bgm_vol {
            self.bgm_vol = bgm;
            self.change_bgm = true;
        }
    }

    /// `ccSqPlay(sq)` (0x001798f0).
    pub fn sq_play(&mut self, sq: usize, out: &mut Vec<Command>) {
        if sq >= self.sq_num || matches!(self.sq_status[sq], 1 | 3) {
            return;
        }
        let t = self.sqtbl[sq];
        let hd = t.hd_port as usize;
        self.port_vol[hd] = t.vol;
        self.port_vol_set(hd, t.vol, out);
        out.push(Command::Play(t.midi_port as usize));
        self.sq_status[sq] = 1;
    }

    /// `ccSqStop(sq)` (0x00179aa0).
    pub fn sq_stop(&mut self, sq: usize, out: &mut Vec<Command>) {
        if sq >= self.sq_num || !matches!(self.sq_status[sq], 1 | 3) {
            return;
        }
        let m = self.sqtbl[sq].midi_port as usize;
        out.push(Command::Stop(m));
        self.fade[m].sw = 0;
        self.sq_status[sq] = 0;
    }

    /// `ccSqFade(sq, per, t, sw)` (0x00179b50): from the port's volume to
    /// `per` 256ths of the table volume over `t` frames.
    pub fn sq_fade(&mut self, sq: usize, per: u16, t: i32, sw: u8) {
        if sq >= self.sq_num {
            return;
        }
        let tb = self.sqtbl[sq];
        let f = &mut self.fade[tb.midi_port as usize];
        f.sw = sw;
        f.time = t;
        let now = (self.port_vol[tb.hd_port as usize] as i32) << 8;
        let end = (tb.vol as i32 * per as i32) & 0x1ff00;
        f.rate = if t != 0 { (now - end) / t } else { 0 };
        f.volume = now;
        f.end_vol = (end >> 8) as u16;
    }

    /// `ccSound::ccFade` (0x00181250), once a frame: sequencer 0's fade on
    /// port 1, and sequencer 1's on port 2 when two are loaded. The port is
    /// only written every other frame, on odd counts for port 1 and even
    /// ones for port 2, as the game does.
    fn fades(&mut self, out: &mut Vec<Command>) {
        for (m, port) in [(0usize, 1usize), (1, 2)] {
            if m == 1 && self.sq_num < 2 {
                continue;
            }
            let f = self.fade[m];
            if f.sw == 0 {
                continue;
            }
            if f.time == 0 {
                self.port_vol[port] = f.end_vol;
                self.port_vol_set(port, f.end_vol, out);
                if f.sw & 2 != 0 && f.rate >= 0 {
                    self.sq_stop(m, out);
                }
                self.fade[m].sw = 0;
                continue;
            }
            let v = (f.volume - f.rate).clamp(0, 0x10000);
            self.fade[m].volume = v;
            self.port_vol[port] = (v >> 8) as u16;
            let odd = f.time & 1 != 0;
            if (m == 0 && odd) || (m == 1 && !odd) {
                self.port_vol_set(port, self.port_vol[port], out);
            }
            self.fade[m].time -= 1;
        }
    }

    /// `ccSndChangeOption` (0x00179630): the music ports back to their
    /// table volumes under the new `bgmVol`, the SE port to the new `seVol`.
    /// In The World the ports a scene's own music sets are left to it: in
    /// a field or dungeon with area 15's bank (`+0x105` 2) only port 2, or
    /// ports 1 and 2 inside the church (`+0x10c` 1); in a town other than
    /// Mac Anu (`+0x105` 3) ports 1 and 3, or 2 and 3 while the breeder's
    /// tune plays (`+0x108`).
    fn change_option(&mut self, out: &mut Vec<Command>) {
        if self.change_bgm {
            let ports: &[usize] = match (self.in_world, self.game_area, self.scene_mode) {
                (true, 1 | 2, 2) if self.church_block == 1 => &[1, 2],
                (true, 1 | 2, 2) => &[2],
                (true, 0, 3) if self.scene_bgm => &[2, 3],
                (true, 0, 3) => &[1, 3],
                _ => &[1, 2, 3],
            };
            for &p in ports {
                self.port_vol_set(p, self.sqtbl[p - 1].vol, out);
            }
            self.change_bgm = false;
        }
        if self.change_se {
            self.port_vol_set(0, 256, out);
            self.change_se = false;
        }
    }

    /// One frame of the sound task (`ccSoundRpc`, 0x00182dc0): the battle
    /// music switch and the fades (unless the music is held,
    /// `ccSound.bgmStopFlag`), the voice slots (`evVoicePlay`), the options
    /// (`ccSndChangeOption`), the master volume (`sdCommand`).
    fn task(&mut self, fades: bool, out: &mut Vec<Command>) {
        if fades && !self.bgm_stop && self.game_start {
            self.bgm_change(out);
            self.fades(out);
        }
        if !self.game_start {
            self.scene_fades(out);
        }
        if self.gate_hack != GateHack::Off {
            self.gate_hack_ctrl(out);
        }
        // In a field or dungeon, the skill words first.
        if self.game_start && matches!(self.game_area, 1 | 2) {
            self.skill_voice_play(out);
        }
        self.voice_play(out);
        self.change_option(out);
        if self.change_main {
            let m = self.main_vol;
            out.push(Command::Master((((m << 14) - m) >> 8).clamp(0, 0x3fff) as u16));
            self.change_main = false;
        }
        if self.sound_off {
            out.push(Command::AllSoundOff);
            self.sound_off = false;
        }
    }

    /// `ccSound::ccSceneFade` (0x00181440), once a frame while the game has
    /// not started (the title): fade `k` drives port `k + 1` - fades 1 and 2
    /// only with that many sequences loaded - and writes it every frame; at
    /// the end the port takes the end volume and, with `sw` bit 1 and a
    /// falling volume, `ccSqStop(k)`.
    fn scene_fades(&mut self, out: &mut Vec<Command>) {
        for k in 0..3 {
            if k >= 1 && self.sq_num < k + 1 {
                break;
            }
            let f = self.fade[k];
            if f.sw == 0 {
                continue;
            }
            let port = k + 1;
            if f.time == 0 {
                self.port_vol[port] = f.end_vol;
                self.port_vol_set(port, f.end_vol, out);
                if f.sw & 2 != 0 && f.rate > 0 {
                    self.sq_stop(k, out);
                }
                self.fade[k].sw = 0;
                continue;
            }
            let v = (f.volume - f.rate).clamp(0, 0x10000);
            self.fade[k].volume = v;
            self.port_vol[port] = (v >> 8) as u16;
            self.port_vol_set(port, self.port_vol[port], out);
            self.fade[k].time -= 1;
        }
    }

    /// Once per game frame: the sound task, and a pending jukebox change
    /// whose fade has run out.
    pub fn frame(&mut self, tables: &Tables, out: &mut Vec<Command>) {
        self.task(true, out);
        if let Some((w, n)) = self.pending {
            if n > 1 {
                self.pending = Some((w, n - 1));
            } else {
                self.pending = None;
                self.finish_change(tables, &w, out);
            }
        }
    }

    /// The jukebox (`Audio_control::ChangeWeve` -> `BgmRead` ->
    /// `ccSndChangeData(&Wave[no], old)`): fade the playing sequence out
    /// over 10 frames (`ccSqFade(sq, 0, 10, 3)`, `ccBreathThread(10)`), then
    /// load the row's bank and play its sequence.
    pub fn change(&mut self, tables: &Tables, w: WaveData, out: &mut Vec<Command>) {
        let old = self.wave;
        if old < 0 {
            self.desktop(tables, w, out);
            return;
        }
        let sq = if SECOND_SEQUENCE.contains(&old) { 1 } else { 0 };
        self.sq_fade(sq, 0, 10, 3);
        self.pending = Some((w, 10));
    }

    fn finish_change(&mut self, tables: &Tables, w: &WaveData, out: &mut Vec<Command>) {
        let old = self.wave;
        let sq = if SECOND_SEQUENCE.contains(&old) { 1 } else { 0 };
        self.port_vol_set(sq + 1, 0, out);
        self.sq_stop(sq, out);
        self.load(tables, w, true, out);
        let sq = if SECOND_SEQUENCE.contains(&w.no) { 1 } else { 0 };
        self.sq_play(sq, out);
    }

    /// The load half of `ccSndChangeData`, in its order, with the frames
    /// its `ccBreathThread(1)` calls let the sound task run: the bank; a
    /// frame; all sound off and the sequences; a frame; the SE port; a
    /// frame; the music ports.
    fn load(&mut self, tables: &Tables, w: &WaveData, fades: bool, out: &mut Vec<Command>) {
        let Some((row, vol)) = tables.wave(w) else { return };
        out.push(Command::Load(row));
        self.task(fades, out);
        out.push(Command::AllSoundOff);
        // sqNum: 1, then 2 if the second size is non-zero, then 3 if the
        // third is - whatever the second was.
        out.push(Command::Seq(0));
        self.sq_num = 1;
        if row.sq_size[1] != 0 {
            out.push(Command::Seq(1));
            self.sq_num = 2;
        }
        if row.sq_size[2] != 0 {
            out.push(Command::Seq(2));
            self.sq_num = 3;
        }
        self.task(fades, out);
        self.port_vol_set(0, self.port_vol[0], out);
        self.sqtbl = vol;
        for i in 1..4 {
            self.port_vol[i] = self.sqtbl[i - 1].vol;
        }
        self.task(fades, out);
        for i in 1..4 {
            self.port_vol_set(i, self.port_vol[i], out);
        }
        self.sq_status = [-1; 3];
        self.wave = w.no;
    }

    /// The desktop starting (`ccSetupDesktop`, 0x00168320):
    /// `ccSndChangeData(&Wave[dtBgm], -1)` - fades cleared, all sound off,
    /// the music held while the bank loads, no play - then `ccSndBgmCtrl`
    /// (0x0017b020, case 1) plays sequence 1 for rows 27 and 7 and
    /// sequence 0 for every other - row 47 included, which the jukebox plays
    /// as sequence 1.
    pub fn desktop(&mut self, tables: &Tables, w: WaveData, out: &mut Vec<Command>) {
        self.pending = None;
        self.fade[0].sw = 0;
        self.fade[1].sw = 0;
        self.tobj_loop = false;
        out.push(Command::AllSoundOff);
        self.load(tables, &w, false, out);
        self.game_start = true;
        let sq = if matches!(w.no, 27 | 7) { 1 } else { 0 };
        self.sq_play(sq, out);
    }

    /// `ccSndSQLoad(n)` (0x001821d0): the bank of a context
    /// ([`SqContext::pick`]): the music held and all sound off for a frame,
    /// SNDBASE told the context and play type, each sequence of the bank to
    /// its sequencer, the ports to the row's volumes (port 2 to 0 in the
    /// field, where sequence 1 is the battle music), `ccSnd +0xf0` = `n`. A row
    /// of -1 loads nothing, waits 60 frames and leaves the music held. The steps
    /// are in docs/engine/sound.md ("A mode's bank").
    pub fn sq_load(&mut self, tables: &Tables, ctx: SqContext, out: &mut Vec<Command>) {
        let pick = ctx.pick(tables);
        self.pending = None;
        self.fade[0].sw = 0;
        self.fade[1].sw = 0;
        self.bgm_stop = true;
        self.sound_off = true;
        self.bgm_started = false;
        self.tobj_loop = false;
        self.scene_bgm = false;
        self.task(false, out);
        let n = ctx.number();
        out.push(Command::Area(n));
        self.battle_bank = pick.battle_bank;
        self.play_type = pick.play_type;
        self.scene_mode = pick.scene_mode;
        out.push(Command::PlayType(i32::from(pick.play_type)));
        let Some((row, vol)) = pick.row.filter(|(r, _)| r.is_used()) else {
            self.sq_num = 0;
            for _ in 0..60 {
                self.task(false, out);
            }
            self.port_vol_set(0, self.port_vol[0], out);
            self.loop_id = [-1; 8];
            return;
        };
        out.push(Command::Load(row));
        out.push(Command::Seq(0));
        self.sq_num = 1;
        if row.sq_size[1] != 0 {
            out.push(Command::Seq(1));
            self.sq_num = 2;
        }
        if row.sq_size[2] != 0 {
            out.push(Command::Seq(2));
            self.sq_num = 3;
        }
        self.sqtbl = vol;
        for i in 1..4 {
            self.port_vol[i] = if i == 2 && n == 3 { 0 } else { self.sqtbl[i - 1].vol };
        }
        for i in 0..4 {
            self.port_vol_set(i, self.port_vol[i], out);
        }
        self.sq_status = [-1; 3];
        self.loop_id = [-1; 8];
        self.bgm_stop = false;
        self.context = n;
        self.wave = -1;
    }

    /// `ccSndBgmCtrl()` (0x0017b020), once a mode's fade in has ended: the
    /// [`bgm_plan`] for the last load under `world`, carried out.
    pub fn bgm_ctrl(&mut self, world: &BgmWorld, out: &mut Vec<Command>) -> BgmPlan {
        let plan = bgm_plan(self.context, self.play_type, self.hold, world);
        // In Mac Anu (town type 0): `+0x105` = 1 and the canals' loop to be
        // started again (`+0x132` = 0).
        if self.context == 2 && world.town == 0 {
            self.scene_mode = 1;
            self.water_loop = false;
        }
        if plan.held {
            self.hold = false;
        }
        for &sq in plan.play {
            self.sq_play(sq, out);
        }
        if plan.started {
            self.bgm_started = true;
        }
        plan
    }

    /// `ccSound::bgmChange` (0x00183380), each frame of the sound task
    /// while the music is not held and the game has started. In a bank with
    /// a battle arrangement ([`Pick::battle_bank`]) of two or more
    /// sequences, once `ccSndBgmCtrl` has run and not on a Grunty: a battle
    /// starting (`game.inBattle`) has SNDBASE start sequence 1 where
    /// sequence 0 is ([`Command::BgmChange`]), fades sequence 1 in and 0 out
    /// (0 stays with play type 2) over 30 frames; the battle's end does the
    /// reverse.
    fn bgm_change(&mut self, out: &mut Vec<Command>) {
        if !self.bgm_started || !self.battle_bank || self.sq_num < 2 || self.riding {
            return;
        }
        if !self.battle_music && self.in_battle {
            out.push(Command::BgmChange);
            self.battle_music = true;
            self.sq_status[1] = 1;
            if self.play_type != 2 {
                self.sq_fade(0, 0, 30, 3);
            }
            self.sq_fade(1, 256, 30, 1);
        } else if self.battle_music && !self.in_battle {
            out.push(Command::BgmChange);
            self.battle_music = false;
            self.sq_status[0] = 1;
            self.sq_fade(1, 0, 30, 3);
            if self.play_type != 2 {
                self.sq_fade(0, 256, 30, 1);
            }
        }
    }

    /// The event instruction `sound 10` (`ccSndEvRequest(10, ..)`,
    /// 0x0017e260): the next `ccSndBgmCtrl` of the desktop, a field, a
    /// dungeon or an event bank starts nothing.
    pub fn hold_bgm(&mut self) {
        self.hold = true;
    }

    /// `ccSetMainVol(v)` (0x001794b0): core 1's master volume, `(v << 14) -
    /// v >> 8`, sent on the next frame.
    pub fn set_main_volume(&mut self, v: i32) {
        if v != self.main_vol {
            self.main_vol = v;
            self.change_main = true;
        }
    }

    /// `ccAllSoundOff()` (0x0017ae10), at the start of each mode's set-up:
    /// fades 0 and 1 stopped, all sound off at the end of the next frame,
    /// and `ccEvVoiceStop` - none of it while [`Driver::scene_mode`] is 2
    /// (story area 15's event bank).
    pub fn all_sound_off(&mut self) {
        if self.scene_mode == 2 {
            return;
        }
        self.fade[0].sw = 0;
        self.fade[1].sw = 0;
        self.sound_off = true;
        self.voice_stop();
    }

    /// `ccSound::gameInterrupt()` (0x001811f0), from `ccGame::ChangeRequest`:
    /// the game has not started until the next mode says so.
    pub fn game_interrupt(&mut self) {
        self.game_start = false;
        self.in_world = false;
    }

    /// `ccSndGameOver()` (0x00180580): `gameInterrupt`, then every loaded
    /// sequence faded out over 20 frames and stopped (`ccSqFade(i, 0, 20,
    /// 3)`, written inline).
    pub fn game_over(&mut self) {
        self.game_interrupt();
        for sq in 0..self.sq_num {
            self.sq_fade(sq, 0, 20, 3);
        }
    }

    /// `ccPgBgmInit()` (0x001801d0), as the Grunty Flute's call starts: the
    /// voice slots flagged and the first free one set to stop channel 0
    /// (as `ccEvVoiceStop`), then sequence 0's fade and, with two or more
    /// loaded, sequence 1's from the port's volume to nothing over 8 frames,
    /// stopping them (`ccSqFade(n, 0, 8, 3)`, written inline).
    pub fn pg_bgm_init(&mut self) {
        self.voice_stop();
        self.sq_fade(0, 0, 8, 3);
        self.sq_fade(1, 0, 8, 3);
    }

    /// `ccPgBgmEnd(n)` (0x00180360), the sequences' part (the `BGM.BIN`
    /// stop, queued when no `wavPlay` command is, is
    /// [`crate::Audio::pg_bgm`]'s): with `n` 0 and play type 0 or 1,
    /// sequence 0 started at volume 0 (`ccSqPlayVol(0, 0)`) and faded in to
    /// its table volume over 10 frames (`ccSqFade(0, 256, 10, 1)`, written
    /// inline); then the battle music's flag (+0x60) cleared.
    pub fn pg_bgm_end(&mut self, n: i32, out: &mut Vec<Command>) {
        if n == 0 && matches!(self.play_type, 0 | 1) {
            crate::stream::sq_play_vol(self, 0, 0, out);
            self.sq_fade(0, 256, 10, 1);
        }
        self.battle_music = false;
    }

    /// `ccSndGateHack(n)` (main 0x00180780), from `GtHackMenu`: 0 as the
    /// menu opens, sequence 0 and, with three loaded, sequence 2 faded from
    /// their ports' volumes to nothing over 20 frames and stopped (the fade
    /// written inline with switch 3); 1 as a cancelled hack closes, the
    /// music back ([`Driver::gate_hack_ctrl`]); 2 the control off.
    pub fn gate_hack(&mut self, n: i32) {
        match n {
            0 => {
                self.gate_hack = GateHack::Out;
                self.sq_fade(0, 0, 20, 3);
                if self.sq_num >= 3 {
                    self.sq_fade(2, 0, 20, 3);
                }
            }
            1 => self.gate_hack = GateHack::Back,
            2 => self.gate_hack = GateHack::Off,
            _ => {}
        }
    }

    /// `ccSndGateHackCtrl()` (main 0x00180910), each frame of the sound
    /// task while `gateHack` is set, after `ccFade`: `ccSceneFade` as well
    /// (so the fades run twice a frame); on the way back sequences 0 and 2
    /// restarted at volume 0 (`ccSqPlayVol(n, 0)`) and faded up to their
    /// table volumes over 20 frames (switch 1), once.
    fn gate_hack_ctrl(&mut self, out: &mut Vec<Command>) {
        match self.gate_hack {
            GateHack::Off => {}
            GateHack::Out | GateHack::Faded => self.scene_fades(out),
            GateHack::Back => {
                crate::stream::sq_play_vol(self, 0, 0, out);
                self.sq_fade(0, 256, 20, 1);
                crate::stream::sq_play_vol(self, 2, 0, out);
                if self.sq_num >= 3 {
                    self.sq_fade(2, 256, 20, 1);
                }
                self.gate_hack = GateHack::Faded;
            }
        }
    }

    /// Stop the music: `ccSqStop` on every sequence.
    pub fn stop(&mut self, out: &mut Vec<Command>) {
        self.pending = None;
        for sq in 0..self.sq_num {
            self.sq_stop(sq, out);
        }
    }

    /// `ccEvVoiceRequest(event, msg)` (0x0017e810), which every message window
    /// calls as it opens: the row of the event's table (the disc's own,
    /// [`piney_data::tables::voice`], by `event / 100`: main, Parody, side or
    /// English), then `ccMesVoicePlay` claims a slot; true when one was asked
    /// for. The request is the shared `vdRequest`: a second in the same frame
    /// changes what the first one's slot sends. Events below -1 are
    /// [`Driver::field_voice_request`]'s. Not ported: a message past its table.
    pub fn voice_request(&mut self, event: i32, msg: i32) -> bool {
        if event < -1 {
            return self.field_voice_request(event, msg);
        }
        let Ok(e) = usize::try_from(event) else { return false };
        let Some(section) = self.voice.events.get(e / 100) else { return false };
        let sub = e % 100;
        let branch = match sub {
            0..50 if self.parody => section.parody,
            0..50 => section.main,
            _ => section.side,
        };
        let Some(b) = branch else { return false };
        let table = match b.en {
            Some(en) if self.voice_english => en,
            _ => b.jp,
        };
        self.voice_file = b.file as usize;
        let rows = table.get(sub % 50).copied().flatten();
        let row = rows.and_then(|r| usize::try_from(msg).ok().and_then(|m| r.get(m)));
        self.vd_request = row.copied();
        match self.vd_request {
            Some(r) if r.ofs != -1 => {
                self.claim(VoiceSlot::Play);
                true
            }
            _ => false,
        }
    }

    /// `ccVoiceRequest(group, msg)` (0x0017eeb0): the field's voices, from the
    /// disc's tables ([`piney_data::tables::voice::FieldVoice`]; English when
    /// `saveData.voice` is set, no Parody test). A line sets `voiceFile` to the
    /// group's and claims a slot; a group with no case, or a message past its
    /// table, has no voice. From Mutation on, English messages in a group's
    /// `alt` range take `MIAE.BIN`'s rows while `talkNum[talk]` is set. Not
    /// ported: Mutation dropping requests while a scene loads (`ccSnd +0x139`).
    pub fn field_voice_request(&mut self, group: i32, msg: i32) -> bool {
        let Some(g) = self.voice.field.iter().find(|g| g.group == group) else { return false };
        if self.voice_english
            && let Some(a) = g.alt
            && (a.first..a.end).contains(&msg)
            && usize::try_from(a.talk).ok().and_then(|t| self.talk_num.get(t)).is_some_and(|&t| t != 0)
        {
            return self.voice_row(a.rows, msg + a.add, a.file);
        }
        self.field_voice(&g.table, msg)
    }

    /// `ccVoicePgFood(food)` (0x001800e0): a Grunty food calling out (its
    /// caller, `ccGimFood::main`, and the runtime keep it to out of
    /// battle): row `food` of `voiceFoodTbl` (the English table with
    /// `saveData.voice`), `voiceFile` 18, a slot claimed.
    pub fn food_voice_request(&mut self, food: i32) -> bool {
        self.field_voice(&piney_data::tables::voice::FOOD, food)
    }

    /// A row of a field voice table as the request.
    fn field_voice(&mut self, t: &VoiceTable, msg: i32) -> bool {
        let rows = if self.voice_english { t.en } else { t.jp };
        self.voice_row(rows, msg, t.file)
    }

    /// Row `msg` of `rows` as the request, from file `file` when it is a
    /// line.
    fn voice_row(&mut self, rows: &[VoiceData], msg: i32, file: i32) -> bool {
        let row = usize::try_from(msg).ok().and_then(|m| rows.get(m)).copied();
        self.vd_request = row;
        match row {
            Some(r) if r.ofs != -1 => {
                self.voice_file = file as usize;
                self.claim(VoiceSlot::Play);
                true
            }
            _ => false,
        }
    }

    /// `ccWordsPlay(sid, ch)` (0x0017e290): a skill's name, queued for
    /// `skillVoicePlay`, unless an event is running (`eventMng` +0x78c), the
    /// caster is not Kite nor a party member (`ch->base->type & 5`) or the
    /// id is 304 or more (-1). The queue's fourth entry is overwritten while
    /// it is full. `char_id` is `ch->base` +0x0c, `type_bit` bit 0 of
    /// `ccGetSkillParam(sid)+0x2c` (which `skillVoicePlay` reads as it plays).
    pub fn words_play(&mut self, event_running: bool, char_type: u32, char_id: i16, sid: i32, type_bit: bool) -> i32 {
        if event_running || char_type & 5 == 0 || sid >= 304 {
            return -1;
        }
        let n = self.words_num.clamp(0, 3) as usize;
        self.words[n] = (char_id, sid as i16, type_bit);
        self.words_num += 1;
        if self.words_num >= 4 {
            self.words_num = 3;
        }
        0
    }

    /// `skillVoicePlay` (0x0017e350), from the sound task in a field or
    /// dungeon: the first queued word, from the character's file and rows in
    /// the disc's tables ([`piney_data::tables::voice::SkillVoice`]), the row
    /// by a per-character rule on the skill id, sent as `evVoicePlay` sends a
    /// line; then the queue emptied. A character without a file, or a row of
    /// -1, leaves the queue for next frame. A row outside the character's
    /// table sends what lies there (`skill_memory`) and empties it too.
    pub fn skill_voice_play(&mut self, out: &mut Vec<Command>) {
        if self.words_num == 0 {
            return;
        }
        let Some(i) = self.words.iter().position(|w| w.0 != -1) else { return };
        let (c, sid, type_bit) = self.words[i];
        let Some(v) = usize::try_from(c).ok().and_then(|k| self.voice.skill.get(k)) else { return };
        let words = if self.voice_english { &v.en } else { &v.jp };
        let at = i64::from(words.row0) + i64::from(skill_row(c, sid, type_bit));
        let row = usize::try_from(at).ok().and_then(|k| self.voice.skill_memory.get(k)).copied();
        let Some(r) = row.filter(|r| r.ofs != -1) else { return };
        out.push(Command::Voice(VoiceCmd { file: words.file, ofs: r.ofs, size: r.siz, vol: VOICE_VOL }));
        self.words = [(-1, -1, false); 4];
        self.words_num = 0;
    }

    /// The disc's voice tables (`Audio` sets them from the disc).
    pub fn set_voice(&mut self, voice: &'static piney_data::tables::voice::Voice) {
        self.voice = voice;
    }

    /// `ccEvVoiceStop()` (0x0017ee40): a slot that stops channel 0.
    pub fn voice_stop(&mut self) {
        self.claim(VoiceSlot::Stop);
    }

    /// `ccMesVoicePlay` / `ccEvVoiceStop`: flag the slots and take the first
    /// free one; with both taken the call is lost.
    fn claim(&mut self, what: VoiceSlot) {
        self.voice_flag = true;
        if let Some(s) = self.voice_slots.iter_mut().find(|s| **s == VoiceSlot::Free) {
            *s = what;
        }
    }

    /// `evVoicePlay` (0x0017eca0), once a frame from the sound task: each
    /// taken slot, in order, sends its command - a play sends the shared
    /// request with the file name of the language set now.
    fn voice_play(&mut self, out: &mut Vec<Command>) {
        if !self.voice_flag {
            return;
        }
        for i in 0..2 {
            match self.voice_slots[i] {
                VoiceSlot::Free => continue,
                VoiceSlot::Stop => out.push(Command::VoiceStop),
                VoiceSlot::Play => {
                    let names = if self.voice_english { self.voice.files_e } else { *piney_data::tables::voice::FILES };
                    // A NULL request would send what lies at address 0.
                    if let (Some(vd), Some(&file)) = (self.vd_request, names.get(self.voice_file)) {
                        out.push(Command::Voice(VoiceCmd { file, ofs: vd.ofs, size: vd.siz, vol: VOICE_VOL }));
                    }
                }
            }
            self.voice_flag = false;
            self.voice_slots[i] = VoiceSlot::Free;
        }
    }

    /// The voice slots waiting for the next frame.
    pub fn voice_pending(&self) -> bool {
        self.voice_flag && self.voice_slots.iter().any(|s| *s != VoiceSlot::Free)
    }
}

/// `skillVoicePlay`'s row for character `c` (`charTbl` row) and skill `sid`
/// (the jump table at 0x0034d830): the id less a base by character, the
/// base by bit 0 of the skill's type (clear: 123 for most, 114 for 4 and
/// 15, 150 for 9, 10, 16 and 17); past character 17, less 6.
pub fn skill_row(c: i16, sid: i16, type_bit: bool) -> i32 {
    let clear = !type_bit;
    let s = i32::from(sid);
    match c {
        0 | 7 | 11 => {
            if clear {
                s - 123
            } else {
                s - 6
            }
        }
        1 | 2 | 3 | 6 | 12 => {
            if clear {
                s - 123
            } else {
                s - 33
            }
        }
        5 | 13 => s - 123,
        8 | 14 => {
            if clear {
                s - 123
            } else {
                s - 96
            }
        }
        4 | 15 => {
            if clear {
                s - 114
            } else {
                s - 60
            }
        }
        9 | 10 | 16 | 17 => {
            if clear {
                s - 150
            } else {
                0
            }
        }
        _ => s - 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_contexts() {
        let field = AreaMusic { area: 1, scene_replaced: true, field_type: 10, special_room: -1, ..Default::default() };
        assert_eq!(setup_context(&field), Some(SqContext::Field { field_type: 10, bg: 0, piros: false }));
        // A story field whose EVENTAREA_INFO.model is 1 plays its event bank.
        let story = AreaMusic { field: 66, field_model: 1, area_prev: 1, ..field };
        assert_eq!(setup_context(&story), Some(SqContext::Event { field: 66, area_prev: 1 }));
        assert_eq!(setup_context(&AreaMusic { scene_replaced: false, ..field }), None);
        let dungeon = AreaMusic { area: 2, dungeon_type: 3, ..field };
        assert_eq!(setup_context(&dungeon), Some(SqContext::Dungeon { dungeon_type: 3 }));
        // The same dungeon: nothing, unless the room is special.
        let same = AreaMusic { scene_replaced: false, field: 14, area_prev: 2, ..dungeon };
        assert_eq!(setup_context(&same), None);
        assert_eq!(
            setup_context(&AreaMusic { special_room: 0, ..same }),
            Some(SqContext::Event { field: 14, area_prev: 2 })
        );
        let town = AreaMusic { area: 0, town: 0, crisis: true, ..field };
        assert_eq!(setup_context(&town), Some(SqContext::Town { row: 5 }));
    }

    #[test]
    fn bgm_plans() {
        let w = BgmWorld { scene_replaced: true, ..Default::default() };
        assert_eq!(bgm_plan(3, 1, false, &w).play, &[0, 2]);
        assert_eq!(bgm_plan(5, 2, false, &w).play, &[2]);
        assert_eq!(bgm_plan(3, 3, false, &w), BgmPlan { play: &[], held: false, started: true });
        assert_eq!(bgm_plan(3, 0, true, &w), BgmPlan { play: &[], held: true, started: true });
        let same = BgmWorld { scene_replaced: false, ..w };
        assert_eq!(bgm_plan(4, 0, true, &same), BgmPlan { play: &[], held: false, started: false });
        // Mac Anu in the crisis: the crisis layer, then the town's music.
        assert_eq!(bgm_plan(2, 0, false, &BgmWorld { crisis: true, ..w }).play, &[2, 0]);
        assert_eq!(bgm_plan(1, 0, false, &BgmWorld { dt_bgm: 27, ..w }).play, &[1]);
        assert_eq!(bgm_plan(7, 0, false, &w).play, &[] as &[usize]);
    }

    /// `ccSndGateHack` in a town with three sequences playing: 0 opens the
    /// menu, and sequences 0 and 2 fade out and stop (0 in 11 frames, as
    /// `ccFade` and `ccSceneFade` both run its fade; 2 in 21, only
    /// `ccSceneFade` runs fade 2) while 1 plays on. 1 (cancelled) starts
    /// them again at volume 0 and brings them up to their table volumes;
    /// 2 ends the control.
    #[test]
    fn the_gate_hack_silences_the_music_and_a_cancel_brings_it_back() {
        let mut d = Driver::new();
        d.sq_num = 3;
        for t in &mut d.sqtbl {
            t.vol = 200;
        }
        d.game_start = true;
        let mut out = Vec::new();
        for sq in 0..3 {
            d.sq_play(sq, &mut out);
        }
        d.gate_hack(0);
        let mut stopped = [None; 3];
        for f in 1..=40 {
            out.clear();
            d.task(true, &mut out);
            for c in &out {
                if let Command::Stop(m) = *c {
                    stopped[m].get_or_insert(f);
                }
            }
        }
        assert_eq!(stopped, [Some(11), None, Some(21)]);
        assert_eq!(d.sq_status, [0, 1, 0]);
        assert_eq!((d.port_vol[1], d.port_vol[2], d.port_vol[3]), (0, 200, 0));
        d.gate_hack(1);
        out.clear();
        d.task(true, &mut out);
        assert!(out.contains(&Command::Play(0)) && out.contains(&Command::Play(2)), "{out:?}");
        assert_eq!(d.gate_hack, GateHack::Faded);
        for _ in 0..40 {
            d.task(true, &mut out);
        }
        assert_eq!(d.sq_status, [1, 1, 1]);
        assert_eq!((d.port_vol[1], d.port_vol[3]), (200, 200));
        d.gate_hack(2);
        assert_eq!(d.gate_hack, GateHack::Off);
    }
}
