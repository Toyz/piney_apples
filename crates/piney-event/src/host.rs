//! What the interpreter needs from the rest of the port: [`Host`] carries the
//! save, the facts the conditions test and every effect an instruction has
//! outside the interpreter, which owns only [`crate::vm::EventMng`]. Every
//! method has a default describing an empty world (no party, no menus open,
//! every wait over at once); [`LogHost`] records each call as a line of text.
//! An instruction that takes frames opens something, polls the host once a
//! frame until the wait is over, then closes it, with the game's own frame
//! counts around it (`docs/engine/event-vm.md`, Instructions that take frames).

use std::fmt;

use crate::ir::Message;
use crate::state::SaveData;

/// `ccGame`: the mode and where the player is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Game {
    /// `game.status` (+0x00): 1 title (DEMO), 2 desktop, 3 bulletin board (TOPPAGE), 5 in The World.
    pub status: i32,
    /// `+0x14 area`: 0 town, 1 field, 2 dungeon.
    pub area: i32,
    /// `+0x18 areaPrev`.
    pub area_prev: i32,
    /// `+0x1c server`.
    pub server: i32,
    /// `+0x20 town`.
    pub town: i32,
    /// `+0x24 field`.
    pub field: i32,
    /// `+0x28 dungeon`.
    pub dungeon: i32,
    /// `+0x2c floor`.
    pub floor: i32,
    /// `+0x30 block`.
    pub block: i32,
    /// `+0x74 gameCntStop`: play time is not counting (the party-time clock stops too).
    pub cnt_stop: i32,
}

/// `ccPartyManager` (a `ccParty`): member ids by slot, -1 empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Party {
    /// `+0x0c memberID[3]`.
    pub ids: [i32; 3],
    /// `+0x18 num`.
    pub num: i32,
}

impl Default for Party {
    fn default() -> Self {
        Party { ids: [-1; 3], num: 0 }
    }
}

impl Party {
    /// `ccParty::CheckMemberID(pc)`: the slot holding `pc` (compared as an
    /// unsigned 16-bit value, as the game does), or -1.
    pub fn slot_of(&self, pc: i16) -> i32 {
        let want = pc as u16 as i32;
        self.ids.iter().position(|&id| id == want).map_or(-1, |s| s as i32)
    }
}

/// The two lists of `g_entCtrl` that `present`/`absent` walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryList {
    /// Types 5 and 6: `g_entCtrl +0x10` count, `+0x14` list.
    Characters,
    /// Type 20: `+0x28` count, `+0x2c` list.
    Gimmicks,
}

/// The boss the event manager tracks (`eventMng.bossEntry`, `bossTscb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boss {
    /// `eventMng.bossEntry`.
    pub entry: i32,
    /// The boss task's `param[0]` (`+0x14`), when there is a task: 0 while it fights.
    pub task_param: Option<i32>,
}

/// A character the player acted on: its data's type mask (`+0x08`, bit per
/// character type) and code (`+0x0c`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharRef {
    /// A handle for the host's own use.
    pub handle: u32,
    pub types: u32,
    pub code: i16,
}

/// A story area (`eventAreaInfo` record and its keywords).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoryArea {
    /// `+0x10 server`.
    pub server: i32,
    /// The three keywords' ids (`GetWordParamFromEvCode(area, 0..2)->+4`);
    /// `None` when a word is not in the word tables.
    pub words: [Option<i32>; 3],
    /// `+0x34`: four (important item id, count) pairs `virus_core` takes back.
    pub protect_items: [(i32, i32); 4],
}

/// A marker in the loaded area: position (`+0x10`) and direction (`+0x28`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Marker {
    pub pos: [f32; 4],
    pub dirc: f32,
}

/// Which window a message goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    /// `message`: the speech window (`ccMessage::Change`).
    Speech,
    /// `info`: the information window (`ccMessage::ChangeInfo`).
    Info,
    /// `info_now`: as `Info`, without the opening fade.
    InfoNow,
    /// `teach_camera1..3`: the camera tutorial's prompt (`ccMessage::Open`,
    /// this event's message 7, 9 or 11, or 55-57 with the other camera
    /// type, with event 3's voice).
    Teach(u8),
}

/// The desktop menu's state as windows see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopMenu {
    /// `ccDtMenu::CheckMenuType()`.
    pub menu_type: i32,
    /// `dtMenu +0x06`: a pending request, -1 when none.
    pub request: i16,
}

/// Where the game shows it, chosen from the phase and the mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Before play starts (phase below 4): the event makes its own window
    /// and draws it itself.
    Setup,
    /// On the desktop or the board (mode 2 or 3) during play.
    Desktop,
    /// In The World during play.
    Field,
}

/// A message to show.
#[derive(Clone, Copy, Debug)]
pub struct MessageCall<'a> {
    pub event: u16,
    pub msg: i16,
    /// The record, when the table has one.
    pub record: Option<&'a Message>,
    pub kind: MessageKind,
    pub place: Place,
}

/// Information boxes the game composes itself (`ccEvent::DispInfo`),
/// each with sound effect 74 first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Announce {
    /// `member_add_msg`: a new member's name.
    Member { pc: i16 },
    /// `gate_add_msg`: a story area's server and keywords.
    GateAddress { area: i16 },
    /// `desktop_item`: wallpaper (0), music (1) or movie (2) number `id`.
    DesktopItem { ty: i16, id: i16 },
}

/// The overlays `overlay` loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Demo,
    Desktop,
    Toppage,
    Gcmn,
}

/// Camera instructions, with the script's values (positions and distances
/// are in tenths of world units; the game multiplies them by 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraCommand {
    LookPos {
        x: i16,
        y: i16,
        z: i16,
        half: bool,
    },
    LookChar {
        ty: i16,
        code: i16,
        height: i16,
        half: bool,
    },
    FollowChar {
        ty: i16,
        code: i16,
        height: i16,
    },
    LookMarker {
        marker: i16,
        half: bool,
    },
    PanPos {
        x: i16,
        y: i16,
        z: i16,
        rate: i16,
    },
    PanChar {
        ty: i16,
        code: i16,
        height: i16,
        rate: i16,
        follow: bool,
    },
    PanMarker {
        marker: i16,
        rate: i16,
    },
    Orbit {
        rotx: i16,
        roty: i16,
        dist: i16,
    },
    OrbitMove {
        rotx: i16,
        roty: i16,
        dist: i16,
        rate: i16,
        degrees: bool,
    },
    Mode4,
    ZSet {
        v: [i16; 3],
        c: [i16; 3],
    },
    ZMove {
        v: [i16; 3],
        c: [i16; 3],
        vprate: i16,
        cprate: i16,
    },
    ZSpeed {
        vstype: i16,
        cstype: i16,
    },
    ZPoint {
        num: i16,
        v: [i16; 3],
        c: [i16; 3],
    },
    ZPath {
        num: i16,
        vprate: i16,
        cprate: i16,
        alpha: i16,
    },
    Char {
        ty: i16,
        code: i16,
        height: i16,
        rotx: i16,
        roty: i16,
        dist: i16,
    },
    /// Back to the normal camera (`changeCamera(1)`).
    End,
    /// `changeCamera(normalCamID)` and a soft reset.
    EndReset,
}

/// Town NPC instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcCommand {
    Act { npc: i16, param: i16 },
    WalkPos { npc: i16, x: i16, y: i16, z: i16 },
    WalkDir { npc: i16, rot: i16, dist: i16 },
    WalkMarker { npc: i16, marker: i16 },
    WalkChar { npc: i16, ty: i16, code: i16, rot: i16, dist: i16 },
    PutMarker { npc: i16, marker: i16 },
    Put { npc: i16, x: i16, y: i16, z: i16 },
    Turn { npc: i16, dirc: i16, chg: i16 },
    Face { npc: i16, ty: i16, code: i16, chg: i16 },
}

/// Party character instructions (`pc` is a `charTbl` row, -1/-2 a party
/// slot, -3 the party).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcCommand {
    Act {
        pc: i16,
        act: i16,
    },
    WalkPos {
        pc: i16,
        x: i16,
        y: i16,
        z: i16,
        run: bool,
    },
    WalkDir {
        pc: i16,
        rot: i16,
        dist: i16,
    },
    WalkMarker {
        pc: i16,
        marker: i16,
    },
    WalkChar {
        pc: i16,
        ty: i16,
        code: i16,
        rot: i16,
        dist: i16,
    },
    Mode {
        pc: i16,
        param: i16,
    },
    Command {
        pc: i16,
        on: i16,
    },
    PutMarker {
        pc: i16,
        marker: i16,
    },
    /// The companions (party slots 1 and 2).
    PartyPutMarker {
        pc: i16,
        marker: i16,
    },
    PartyPut {
        pc: i16,
        x: i16,
        y: i16,
        z: i16,
    },
    Put {
        pc: i16,
        x: i16,
        y: i16,
        z: i16,
    },
    Turn {
        pc: i16,
        dirc: i16,
        chg: i16,
    },
    Face {
        pc: i16,
        ty: i16,
        code: i16,
        chg: i16,
    },
    UseSkill {
        pc: i16,
        skill: i16,
    },
    Tint {
        pc: i16,
        r: i16,
        g: i16,
        b: i16,
        rate: i16,
    },
    TintOff {
        pc: i16,
    },
}

/// Gimmick instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GimmickCommand {
    Radiator { rtype: i16, ty: i16, code: i16, x: i16, y: i16, z: i16, roty: i16, rotz: i16 },
    BossSmoke { floor: i16, block: i16, x: i16, y: i16, z: i16 },
    DeleteAll19,
    OpenDoor,
    CloseDoor,
}

/// Blocking instructions the interpreter polls through [`Host::busy`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    /// `stream`: the cutscene is playing (after [`Host::stream`]).
    Stream,
    /// `overlay`: the overlay is loading (after [`Host::load_overlay`]).
    Overlay,
    /// `piros_colour`: the flash and its information boxes (after
    /// [`Host::piros_colour`]; the shape of this wait is not traced).
    PirosColour,
    /// `staff_roll`: `begin` pauses the game, puts every task to sleep and
    /// starts the desktop's staff roll task; `busy` while its `param[0]` is
    /// 0; `end` deletes the task.
    StaffRoll,
    /// `gate_hack_anim`: menus off (`begin`), `ccCheckGtHackAnm()` true
    /// (`busy`), menus back (`end`).
    GateHackAnim,
    /// `player_skill`: `begin` sets the menu up; `busy` while the command
    /// target (`cmndTarget +0x08`) has not acted; after each busy frame the
    /// interpreter calls [`Host::player_skill_step`]; `end` restores the menu.
    PlayerSkill,
}

thread_local! {
    static UNPORTED: std::cell::RefCell<std::collections::BTreeSet<&'static str>> =
        const { std::cell::RefCell::new(std::collections::BTreeSet::new()) };
}

/// A [`Host`] method's default ran: the host has not ported it. The first
/// time on this thread each name is printed to stderr, so a gap shows in
/// play instead of silently doing nothing; [`take_unported`] hands the
/// names to tests.
pub fn unported(name: &'static str) {
    UNPORTED.with(|u| {
        if u.borrow_mut().insert(name) {
            eprintln!("event host: `{name}` is not ported by this host (the default ran)");
        }
    });
}

/// The [`Host`] defaults that ran on this thread since the last call, by
/// name.
pub fn take_unported() -> Vec<&'static str> {
    UNPORTED.with(|u| std::mem::take(&mut *u.borrow_mut()).into_iter().collect())
}

/// Everything outside the interpreter. See the module documentation.
#[allow(unused_variables)]
pub trait Host {
    /// The save the scripts read and write.
    fn save(&mut self) -> &mut SaveData;

    // --- The world, as conditions and the scheduler see it -------------------------------

    fn game(&self) -> Game {
        unported("game");
        Game::default()
    }
    /// `ccSystem::GetFrameRate`: vertical blanks per frame (1 or 2).
    fn frame_rate(&self) -> i32 {
        unported("frame_rate");
        1
    }
    /// `compulsionGameOver`.
    fn game_over(&self) -> bool {
        unported("game_over");
        false
    }
    fn party(&self) -> Party {
        unported("party");
        Party::default()
    }
    /// `present`/`absent` type 2: the SPC manager holds this character.
    fn spc_present(&self, code: i16) -> bool {
        unported("spc_present");
        false
    }
    /// `present`/`absent` types 5, 6, 20: an entry whose data has type bit
    /// `ty` and this code is in the list.
    fn entry_present(&self, list: EntryList, ty: i16, code: i16) -> bool {
        unported("entry_present");
        false
    }
    fn boss(&self) -> Option<Boss> {
        unported("boss");
        None
    }
    /// `no_active`: `ccCheckActiveObject()` returns non-zero (in its low
    /// byte) and no gimmick whose data has type bit 20 is active (entry flag
    /// bit 0 set, bit 6 clear).
    fn no_active_object(&self) -> bool {
        unported("no_active_object");
        true
    }
    /// `no_entries`: `g_entCtrl.enNum` and `mcNum` are 0.
    fn no_entries(&self) -> bool {
        unported("no_entries");
        true
    }
    /// `ccMenuCtrl::CheckMenuType()`: -1 when no field menu is open.
    fn field_menu(&self) -> i32 {
        unported("field_menu");
        -1
    }
    /// The desktop menu (`dtMenu`), which windows on the desktop wait for:
    /// a message until its type is -1 or 7 and its request slot is free, an
    /// information box until its type is -1 or 7.
    fn desktop_menu(&self) -> DesktopMenu {
        unported("desktop_menu");
        DesktopMenu { menu_type: -1, request: -1 }
    }
    /// `ccSys.pad[0].push`.
    fn pad_pushed(&self) -> u32 {
        unported("pad_pushed");
        0
    }
    /// `enemy_pp`: `GetEnemy(enemy)` exists and its PP count is not zero.
    fn enemy_pp(&self, enemy: i16) -> bool {
        unported("enemy_pp");
        false
    }
    /// A town marker (`markerEvTbl[marker]` in the area's stream).
    fn marker(&self, marker: i16) -> Option<Marker> {
        unported("marker");
        None
    }
    /// The event manager's positions after a `set_pos` or `marker_pos`:
    /// outside the towns an NPC's put or walk to a marker reads them.
    fn event_positions(&mut self, _positions: &[crate::vm::EvPos]) {}
    /// The distance from the player (`plw`) to a point after
    /// `ccTransPosW2P`; `None` when the game would use an unset vector.
    fn player_distance(&self, pos: Option<[f32; 4]>) -> f32 {
        unported("player_distance");
        f32::MAX
    }
    /// `cmndTarget`: the character the player is acting on.
    fn command_target(&self) -> Option<CharRef> {
        unported("command_target");
        None
    }
    /// `ccCheckTarget`: that character still exists.
    fn target_alive(&self, target: &CharRef) -> bool {
        unported("target_alive");
        true
    }
    fn story_area(&self, area: i16) -> Option<StoryArea> {
        unported("story_area");
        None
    }

    // --- Messages -------------------------------------------------------------------------

    /// Open a message or information window. On the desktop the game first
    /// sets `dtMenu +0x06 = 7` when `dtMenu +0x00` is -1.
    fn message_open(&mut self, call: &MessageCall<'_>) {
        unported("message_open");
    }
    /// `ccMessage::Check`: 0 while the window waits; otherwise the window
    /// has closed, and for a question the value is the answer
    /// (`eventMng.msgSelect`).
    fn message_check(&mut self) -> i32 {
        unported("message_check");
        1
    }
    /// `ccMessage::Close`, or the end of a window the event made itself.
    fn message_close(&mut self) {
        unported("message_close");
    }
    /// After a plain line on the desktop: `dtMenu +0x10 = 1`.
    fn desktop_message_done(&mut self) {
        unported("desktop_message_done");
    }
    /// An information box the game composes; closed like a message.
    fn announce(&mut self, a: Announce) {
        unported("announce");
    }

    // --- Modes, overlays and the screen ---------------------------------------------------

    /// `ccGame::ChangeRequest(num, sf)`. The interpreter disables its own
    /// phase, as `ChangeRequest` does through `ccDisableThEvent`.
    fn change_request(&mut self, num: i32, sf: i32) {
        unported("change_request");
    }
    /// `ccGame::ChangeArea(a, n)`.
    fn change_area(&mut self, a: i32, n: i32) {
        unported("change_area");
    }
    /// `ccGame::ChangeScene`.
    fn change_scene(&mut self, area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16) {
        unported("change_scene");
    }
    fn load_overlay(&mut self, which: Overlay) {
        unported("load_overlay");
    }
    /// `ccSystem::SetFrameRate`.
    fn set_frame_rate(&mut self, rate: i16) {
        unported("set_frame_rate");
    }
    /// `ccEventStream(num, 1)`; the field's file list is the host's.
    fn stream(&mut self, num: i16) {
        unported("stream");
    }
    /// `ccScFade::EntryFade` (`more`: `ContinueFade`). `reset`, from
    /// Mutation on: the fader emptied first (`ccScFade::Init`), and `more`
    /// without a fade of its own starts one.
    fn fade(&mut self, count: i16, alpha: i16, more: bool, reset: bool) {
        unported("fade");
    }
    /// `ccMenu.interNoiz`.
    fn noise(&mut self, level: i32) {
        unported("noise");
    }
    /// `ccSndEvRequest(cmd, p0, p1, p2)`.
    fn sound(&mut self, cmd: i16, p0: i16, p1: i16, p2: i16) {
        unported("sound");
    }
    /// `ccSeOn(se)` where an instruction plays one itself.
    fn sound_effect(&mut self, se: i32) {
        unported("sound_effect");
    }
    /// `ccClearGtHack`, which `ccStartThEvent` calls.
    fn clear_gate_hack(&mut self) {
        unported("clear_gate_hack");
    }

    // --- The desktop ----------------------------------------------------------------------

    /// `name_entry`: make the desktop's `NameEntry_Control`.
    fn name_entry_start(&mut self) {
        unported("name_entry_start");
    }
    /// One frame of `NameEntry_Control::Main`: true when entry is finished.
    fn name_entry_step(&mut self) -> bool {
        unported("name_entry_step");
        true
    }
    fn name_entry_end(&mut self) {
        unported("name_entry_end");
    }

    // --- Blocking effects without a fixed shape --------------------------------------------

    /// Start a blocking effect.
    fn begin(&mut self, w: Wait) {
        unported("begin");
    }
    /// Once per frame, from the frame it began: true while it goes on.
    fn busy(&mut self, w: Wait) -> bool {
        unported("busy");
        false
    }
    /// After the wait (for the effects that restore something).
    fn end(&mut self, w: Wait) {
        unported("end");
    }
    /// One frame of `player_skill`: `ccSkillCheck`, and a skill request
    /// when the confirm button is down.
    fn player_skill_step(&mut self) {
        unported("player_skill_step");
    }

    /// `teach_camera1..3` (part 1-3): this frame's camera input. Parts 1
    /// and 2 wait for 150 frames of it; part 3 for one press (then 45
    /// frames). The prompt is opened with [`Host::message_open`]
    /// ([`MessageKind::Teach`]) and closed with [`Host::message_close`].
    fn teach_input(&self, part: u8) -> bool {
        unported("teach_input");
        true
    }
    /// `camCtrlType`: which camera control scheme the player chose (the
    /// tutorial then uses messages 55-57 instead of 7, 9, 11).
    fn camera_type(&self) -> i32 {
        unported("camera_type");
        0
    }
    /// `remove_trap`: sound effect 228 and the effect at the command target;
    /// 20 frames later, [`Host::remove_trap_done`].
    fn remove_trap(&mut self) {
        unported("remove_trap");
    }
    /// `EntryAffect(cmndTarget, plw, 12, 0, 0, 0)`.
    fn remove_trap_done(&mut self) {
        unported("remove_trap_done");
    }
    /// After the staff roll: ask the desktop menu for menu `num` (`dtMenu
    /// +0x06 = num`); the interpreter then waits for the menu to close.
    fn desktop_menu_open(&mut self, num: i16) {
        unported("desktop_menu_open");
    }
    /// `ccSndBgmCtrl`.
    fn bgm_control(&mut self) {
        unported("bgm_control");
    }
    /// The staff roll's last step: wake every task, `dtMenu +0x16 = 0`.
    fn staff_roll_done(&mut self) {
        unported("staff_roll_done");
    }

    // --- The field ------------------------------------------------------------------------

    fn camera(&mut self, c: CameraCommand) {
        unported("camera");
    }
    fn npc(&mut self, c: NpcCommand) {
        unported("npc");
    }
    fn pc(&mut self, c: PcCommand) {
        unported("pc");
    }
    fn gimmick(&mut self, g: GimmickCommand) {
        unported("gimmick");
    }
    /// `remove`: type 2 out of the party, 5/6 an entry deleted, -1 everything.
    fn remove(&mut self, ty: i16, code: i16) {
        unported("remove");
    }
    fn affect(&mut self, ty: i16, code: i16, bit: i16, on: bool) {
        unported("affect");
    }
    /// `ccParty::AddMember` and the menu face.
    fn party_add(&mut self, pc: i16) {
        unported("party_add");
    }
    /// `ccParty::DelMember`.
    fn party_remove(&mut self, pc: i16) {
        unported("party_remove");
    }
    /// `enemy_put`: the enemy to event position `pos` (the interpreter's, if set).
    fn enemy_put(&mut self, enemy: i16, pos: Option<[f32; 4]>) {
        unported("enemy_put");
    }
    /// `menu_ban` (true) / `menu_clear` (false).
    fn menu_ban(&mut self, on: bool) {
        unported("menu_ban");
    }
    /// `item_add` while playing, for companions: through the menu
    /// (`GetSpc(pc)` then `ccMenuCtrl::AddSpcItem`). Returns false when
    /// `GetSpc(pc)` has no such character, and the item then goes into the save.
    fn add_spc_item(&mut self, pc: i16, category: i16, id: i16, num: i16) -> bool {
        unported("add_spc_item");
        false
    }
    /// `item_add_menu` while playing: open the item-get menu (29). The menu
    /// gives the item; the interpreter waits for [`Host::field_menu`] to be -1.
    fn item_get_menu(&mut self, pc: i16, category: i16, id: i16, num: i16) {
        unported("item_get_menu");
    }
    /// After the item-get menu: `ccMenu +0x0c = 3`.
    fn item_get_menu_end(&mut self) {
        unported("item_get_menu_end");
    }
    /// `menu`: open field menu `num`; the interpreter waits for it to close.
    fn open_menu(&mut self, num: i16) {
        unported("open_menu");
    }
    /// `area`: areas 1-13 leave the field and set the event area; others
    /// generate the area from its words.
    fn area(&mut self, area: i16) {
        unported("area");
    }
    /// `virus_core` when replaying: `WORLD_MAN::SimGenerateCode` from the words.
    fn generate_area(&mut self, words: [Option<i32>; 3]) {
        unported("generate_area");
    }
    fn map_on(&mut self) {
        unported("map_on");
    }
    /// One frame of `WORLD_MAN::ShowMap`: true when done.
    fn show_map(&mut self) -> bool {
        unported("show_map");
        true
    }
    fn prev_room(&mut self) {
        unported("prev_room");
    }
    /// `WORLD_MAN::RoomSelect`.
    fn room(&mut self, floor: i16, block: i16) {
        unported("room");
    }
    /// Start (`Some`) or stop the `ccThEvHold` task on a character.
    fn hold(&mut self, target: Option<(i16, i16)>) {
        unported("hold");
    }
    /// `piros_colour` with its operand and the information line the
    /// interpreter took from the event's messages.
    fn piros_colour(&mut self, code: i16, line: &[u8]) {
        unported("piros_colour");
    }
    fn condition_effect(&mut self, on: bool) {
        unported("condition_effect");
    }
    fn target_forbid(&mut self, on: bool) {
        unported("target_forbid");
    }
    fn trans(&mut self, ty: i16, code: i16, on: bool) {
        unported("trans");
    }
    /// `battle_ready`: `game.inBattleDist = -1.0`.
    fn battle_ready(&mut self) {
        unported("battle_ready");
    }
    /// The first pass of play (phase 4) is over: while `worldman +0xf0` is
    /// 3 the game sets `ccMenu +0x104 = 1`.
    fn play_pass_done(&mut self) {
        unported("play_pass_done");
    }
    /// Mutation's `grunty_mail`.
    fn grunty_mail(&mut self) {
        unported("grunty_mail");
    }
    /// Quarantine's `ending_kanji`.
    fn ending_kanji(&mut self) {
        unported("ending_kanji");
    }
}

/// A host that records every call and otherwise behaves like the defaults.
pub struct LogHost {
    pub save: SaveData,
    pub game: Game,
    pub party: Party,
    pub log: Vec<String>,
    /// Records for [`MessageCall`]s are not logged in full, only their numbers.
    pub echo: bool,
}

impl LogHost {
    pub fn new(save: SaveData) -> Self {
        LogHost { save, game: Game::default(), party: Party::default(), log: Vec::new(), echo: false }
    }

    fn note(&mut self, s: impl fmt::Display) {
        let line = s.to_string();
        if self.echo {
            println!("{line}");
        }
        self.log.push(line);
    }
}

impl Host for LogHost {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
    fn game(&self) -> Game {
        self.game
    }
    fn party(&self) -> Party {
        self.party
    }
    fn message_open(&mut self, c: &MessageCall<'_>) {
        self.note(format_args!("message_open event={} msg={} {:?} {:?}", c.event, c.msg, c.kind, c.place));
    }
    fn message_close(&mut self) {
        self.note("message_close");
    }
    fn desktop_message_done(&mut self) {
        self.note("desktop_message_done");
    }
    fn announce(&mut self, a: Announce) {
        self.note(format_args!("announce {a:?}"));
    }
    fn change_request(&mut self, num: i32, sf: i32) {
        self.note(format_args!("change_request {num} {sf}"));
    }
    fn change_area(&mut self, a: i32, n: i32) {
        self.note(format_args!("change_area {a} {n}"));
    }
    fn change_scene(&mut self, area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16) {
        self.note(format_args!("change_scene {area} {town} {field} {dungeon} {floor} {block}"));
    }
    fn load_overlay(&mut self, which: Overlay) {
        self.note(format_args!("load_overlay {which:?}"));
    }
    fn set_frame_rate(&mut self, rate: i16) {
        self.note(format_args!("set_frame_rate {rate}"));
    }
    fn stream(&mut self, num: i16) {
        self.note(format_args!("stream {num}"));
    }
    fn fade(&mut self, count: i16, alpha: i16, more: bool, _reset: bool) {
        self.note(format_args!("fade {count} {alpha} more={more}"));
    }
    fn noise(&mut self, level: i32) {
        self.note(format_args!("noise {level}"));
    }
    fn sound(&mut self, cmd: i16, p0: i16, p1: i16, p2: i16) {
        self.note(format_args!("sound {cmd} {p0} {p1} {p2}"));
    }
    fn sound_effect(&mut self, se: i32) {
        self.note(format_args!("sound_effect {se}"));
    }
    fn clear_gate_hack(&mut self) {
        self.note("clear_gate_hack");
    }
    fn name_entry_start(&mut self) {
        self.note("name_entry_start");
    }
    fn name_entry_end(&mut self) {
        self.note("name_entry_end");
    }
    fn begin(&mut self, w: Wait) {
        self.note(format_args!("begin {w:?}"));
    }
    fn camera(&mut self, c: CameraCommand) {
        self.note(format_args!("camera {c:?}"));
    }
    fn npc(&mut self, c: NpcCommand) {
        self.note(format_args!("npc {c:?}"));
    }
    fn pc(&mut self, c: PcCommand) {
        self.note(format_args!("pc {c:?}"));
    }
    fn gimmick(&mut self, g: GimmickCommand) {
        self.note(format_args!("gimmick {g:?}"));
    }
    fn remove(&mut self, ty: i16, code: i16) {
        self.note(format_args!("remove {ty} {code}"));
    }
    fn affect(&mut self, ty: i16, code: i16, bit: i16, on: bool) {
        self.note(format_args!("affect {ty} {code} {bit} {on}"));
    }
    fn party_add(&mut self, pc: i16) {
        self.note(format_args!("party_add {pc}"));
    }
    fn party_remove(&mut self, pc: i16) {
        self.note(format_args!("party_remove {pc}"));
    }
    fn enemy_put(&mut self, enemy: i16, pos: Option<[f32; 4]>) {
        self.note(format_args!("enemy_put {enemy} {pos:?}"));
    }
    fn menu_ban(&mut self, on: bool) {
        self.note(format_args!("menu_ban {on}"));
    }
    fn item_get_menu(&mut self, pc: i16, category: i16, id: i16, num: i16) {
        self.note(format_args!("item_get_menu {pc} {category} {id} {num}"));
    }
    fn open_menu(&mut self, num: i16) {
        self.note(format_args!("open_menu {num}"));
    }
    fn area(&mut self, area: i16) {
        self.note(format_args!("area {area}"));
    }
    fn generate_area(&mut self, words: [Option<i32>; 3]) {
        self.note(format_args!("generate_area {words:?}"));
    }
    fn map_on(&mut self) {
        self.note("map_on");
    }
    fn prev_room(&mut self) {
        self.note("prev_room");
    }
    fn room(&mut self, floor: i16, block: i16) {
        self.note(format_args!("room {floor} {block}"));
    }
    fn hold(&mut self, target: Option<(i16, i16)>) {
        self.note(format_args!("hold {target:?}"));
    }
    fn piros_colour(&mut self, code: i16, line: &[u8]) {
        let _ = line;
        self.note(format_args!("piros_colour {code}"));
    }
    fn condition_effect(&mut self, on: bool) {
        self.note(format_args!("condition_effect {on}"));
    }
    fn target_forbid(&mut self, on: bool) {
        self.note(format_args!("target_forbid {on}"));
    }
    fn trans(&mut self, ty: i16, code: i16, on: bool) {
        self.note(format_args!("trans {ty} {code} {on}"));
    }
    fn battle_ready(&mut self) {
        self.note("battle_ready");
    }
    fn end(&mut self, w: Wait) {
        self.note(format_args!("end {w:?}"));
    }
    fn play_pass_done(&mut self) {
        self.note("play_pass_done");
    }
}
