//! The field game's user interface in .hack//Infection: `ccMenuCtrl` and
//! its task `ccThMenu` (gcmn 0x005280d0, `menu.cpp`), with the event
//! windows' `ccMessage` (`ccMsg`) it draws. One [`FieldUi::step`] is one
//! pass of the task between two `ccTscb::Breath` calls: the pad comes in,
//! the world ([`World`]) is read-only, a [`piney_draw::Frame`] of the menu
//! layer (242) comes out, and what the menus do to the rest of the game
//! comes out as [`Request`]s. The save is [`piney_desktop::SaveState`],
//! shared with the event engine. See docs/engine/field-ui.md.

pub mod book;
pub mod chat_msg;
pub mod ctrl;
pub mod dfcomp;
pub mod disp;
pub mod items;
pub mod menus;
pub mod message;
pub mod newgame;
pub mod panel;
pub mod render;
pub mod spr;
pub mod tables;
pub mod talk;
pub mod window;
pub mod words;
pub mod world;

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::SaveState;
use piney_desktop::kanji::Fonts;
use piney_desktop::setup::SetupScreen;
use piney_draw::Frame;
use piney_input::Pad;

pub use ctrl::{MenuCtrl, MenuGlobals};
pub use world::{CharInfo, Game, World};

use crate::render::Textures;
use crate::tables::Texts;

/// `ccSystem.frameRate` in the field: two vertical blanks a frame
/// (`ccSetupNewGame` calls `SetFrameRate(2)`).
pub const FRAME_RATE: u32 = 2;

/// The overlay the field runs with.
pub const GCMN_PATH: &str = "DATA/GCMN.PRG";

/// What a frame asks of the rest of the game, in the order the game does
/// it.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// `ccSeOn(n)`.
    Se(i32),
    /// `ccSeOnNote(n, note)` (main 0x00179cb0): sound `n` at another note.
    SeNote { n: i32, note: i32 },
    /// `cameraShake(power, cycle, time, dirc)` (main 0x00162cd0): the
    /// events' strongest noise (Mutation on, `interNoiz` 5).
    CameraShake { power: i32, cycle: i32, time: i32, dirc: i32 },
    /// `ccSleepAllThread()`: a menu opened; every task but the menu's and
    /// the events' sleeps until [`Request::WakeAll`].
    SleepAll,
    /// `ccWakeAllThread()`.
    WakeAll,
    /// `fontOffFlip(); ccLayer::OffFlipExcept(menuLayer)` (true) or the
    /// reverse (false): the other layers keep their last picture.
    Still(bool),
    /// `ccEvVoiceStop()`.
    VoiceStop,
    /// `ccEvVoiceRequest(grp, msg)`: a message's voice.
    VoiceRequest { grp: i32, msg: i32 },
    /// `cmndTargetFix` set (true) or cleared.
    TargetFix(bool),
    /// `ccChangeCmndTarget(0)`: the target dropped (the old one becomes
    /// `cmndTargetPrev`).
    TargetClear,
    /// `ccChangeCmndTarget(c)`: the character with this handle becomes
    /// `cmndTarget` (the target menu's cursor).
    Target(u32),
    /// `ccSkillRequest(plw, cmndTarget, skill)` (gcmn 0x00572700): the
    /// player uses a skill on the target.
    Skill { target: u32, skill: i16 },
    /// `ccUseItemRequest(plw, target, code, 0)` (gcmn 0x0057aa80): the
    /// player uses an item (`category << 16 | id`) on a character (the
    /// player's own handle for items used at once). The save's count is
    /// already down.
    UseItem { target: u32, code: i32 },
    /// `ccChar::EntryAffect(c, plw, 13, 0, 0, 0)`: Data Drain's hold on a
    /// target, before the drain menu (66) runs.
    DrainAffect(u32),
    /// `plw.pw->SP = sp` (Data Drain's cost, paid by the menu).
    PlayerSp(i16),
    /// `ccSpcChar::RequestChatCmd(member, cmd, 0, 0)`: a party member told
    /// what to do (the CHAT menu's commands and strategies).
    ChatCmd { member: u32, cmd: i32 },
    /// `ccChatMsg::OpenChat(plw, text)`: the player's line in the chat
    /// balloon.
    OpenChat(Vec<u8>),
    /// `ccSpcChar::ChangeEquipReport(member, n)`: a member told its
    /// equipment changed.
    ChangeEquipReport { member: u32, n: i16 },
    /// `ccChar::EntryAffect(c, plw, kind, 0, 0, 0)`: the Chaos Gate's
    /// circle (11 opens it, 0 closes it: the gate's `chaosGateInfluence`).
    Affect { target: u32, kind: i16 },
    /// `ccScFade::EntryFlash(menuFade, 8, 0x3040c0c0, 0, 0, 512, 448)`: a
    /// white flash over the screen, 8 frames.
    Flash,
    /// The gate's pages also keep the noise, still and game layers drawn
    /// (each layer's `flipFlag` down one), with [`Request::Still`].
    KeepLayers,
    /// `ccEvent::CheckAreaCode(a, b, c)`'s record: the words entered at the
    /// gate (`areaCodeSet`, which the events' `gate_words` reads).
    AreaCodeSet([i16; 3]),
    /// `ccSpcChar::TransferOut(member)`: a party member warps out.
    TransferOut(u32),
    /// `ccSPC::DeleteNoPartyMember()`: the characters not in the party let
    /// go (Other Servers, and Gate Out before its `ChangeArea`).
    DeleteNoPartyMember,
    /// `WORLD_MAN::SetGenerateCode(a, b, c)` (main 0x0019eea0): to the
    /// area the three word IDs make (`SimGenerateCode`, then
    /// `ccGame::ChangeArea(1, story area)` for a field, `ChangeArea(2, 0)`
    /// for a dungeon, or a story dungeon's `ChangeScene(2, -2, ..)`).
    /// Asked every frame until the area changes.
    GoToArea([i32; 3]),
    /// `ccGame::ChangeArea(area, n)`: Other Servers' town (area 0); Gate
    /// Out's town `n` (`game.town`, at least 0) once its fade has made the
    /// screen black.
    ChangeArea { area: i32, n: i32 },
    /// `ccGame.areaLevel = v`: the battle level the keyword screen showed.
    AreaLevel(i32),
    // --- The gate hack (menu 62, [`menus::hack`]) ---
    /// `ccSndGateHack(n)` (main 0x00180780): 0 the field's music fades out
    /// over 20 frames (`ccSnd.gateHack` 1, then `ccSound::ccSceneFade`), 1
    /// it fades back in (the hack cancelled), 2 back to 0.
    GateHackSound(i32),
    /// The world hidden (true) or back (false): the menu flips the layers
    /// again while the world's tasks sleep, so they draw nothing, over
    /// `ccSys.bgColor` 0 (the gate hack's screen, Data Drain's movie); then
    /// `bgColor` is put back.
    WorldHidden(bool),
    // --- Data Drain (menu 66, [`menus::drain`]) ---
    /// `DataDrainMenu`'s rules on the world after the movie (gcmn
    /// 0x00533010-0x00533470, `piney_battle::drain::drain`):
    /// `AddLvErosion` into the save, the drops rolled, and `ccDeleteCmnd`
    /// on a target that is not a boss (a boss's `EntryAffect(21)` came
    /// before the movie, as [`Request::Affect`]). The runtime answers with
    /// [`FieldUi::drain_drops`] before the menu's next frame.
    DataDrain { target: u32, sid: i32 },
    /// Step 0's movie: `ccStartThread(ccThExecuteStream)` playing stream
    /// `num`. The runtime plays it and says when it has ended with
    /// [`FieldUi::drain_movie_done`]; the menu task breathes without its
    /// `Disp` meanwhile.
    DrainMovie(i32),
    /// `StreamMenu`'s (menu 74) stream: `ccStartThread(ccThExecuteStream)`
    /// playing stream `num`, answered with [`FieldUi::stream_menu_done`]
    /// when it has ended.
    StreamMenu(i32),
    /// A Ryu Book's cover ([`book::Cover`]), answered with
    /// [`FieldUi::book_stream_done`]; `None` once the book has taken it
    /// down (`ccDeleteThread`).
    BookStream(Option<book::Cover>),
    /// `StreamMenu`'s `ccThStrParty`: the members `streamFlag`'s bits 0-2
    /// name drawn into its stream.
    StrParty(i16),
    /// The movie's `ccThDrainEnemy` on the drained enemy (by handle): the
    /// enemy, and at the stream's frame 30 its drained form, drawn into
    /// the stream's scene until it ends.
    DrainEnemy(u32),
    /// Step 10's side effect on the party (`piney_battle::drain::
    /// side_effect`): answered with [`FieldUi::drain_side_effect`].
    DrainSideEffect,
    /// Step 11's frame 30: `ccChar::LevelDown(plw)` and
    /// `ccEntryFlyFontNewLevelDown(23, pos, plw)`.
    DrainLevelDown,
    /// `cmndTarget` and `cmndTargetPrev` cleared as they are (not through
    /// `ccChangeCmndTarget`).
    TargetsCleared,
    /// `compulsionGameOver = 1`: Data Drain's SYSTEM ERROR.
    GameOver,
    /// The cores went in: `ccGame.setupMode = 1` and `ccSetGtHack()`
    /// (`gtHackFlag`), so the party arrives in the area hacking in
    /// (`ccPlayer::GateHackingOut`); [`Request::GoToArea`] follows.
    GateHacked,
    /// `ccSpcChar::RequestChatCmd(member, cmd, target, skill)` with a
    /// target and a skill (ChatMenuT's lesson: 5, the player, 155).
    ChatOrder { member: u32, cmd: i32, target: u32, skill: i32 },
    /// `member->ai->manualSW = 0` (ChatMenuT, before its order).
    ManualOff(u32),
    /// `ccSpcChar::ManualModeAI(member, ev)` (gcmn 0x0059eba0).
    ManualModeAi { member: u32, ev: i32 },
    /// `ccAI::SetRemoteCmd(member->ai, cmd)` (gcmn 0x005832e0).
    RemoteCmd { member: u32, cmd: i32 },
    /// `ccParty::AddMember(id)` (gcmn 0x0059ce80), from the thread the
    /// menu starts (`ccThPartyAdd`, 0x0053ba20): `inviteSpc(id)` makes the
    /// member's character and it takes the party's first free slot
    /// (`memberChar`, `memberID`, `num`); then `ccSleepNoSleepThread(3, 0)`
    /// puts to sleep every task not flagged 3 (the new member's too). The
    /// menu reads the party again from its next step.
    AddMember(i16),
    /// `WORLD_MAN::SetMapAlpha(a)`: the minimap's alpha, 0-1.
    MapAlpha(f32),
    /// The menu asked for `ChangeMenu` into an unported menu (its number);
    /// the port closes it at once.
    Unported(i16),
    /// One of an item use's steps on the world, as the menu task inside
    /// `ccUseItemRequest` reaches it ([`crate::menus::useitem`]): the
    /// affects, the skill, the effects, the pauses, the messages to the
    /// party.
    ItemStep(piney_battle::item::Step),
    /// `ccSpcMessageOpenTreasureBox()` (gcmn 0x005a1a10): the party's line
    /// on a box or idol opened, `ccAISysMsgSend(0x10010, -1, Kite's AI,
    /// 0xffff, 0, 30)`.
    SpcMessageOpenTreasureBox,
    /// `plw->BreakSomething(0)` (gcmn 0x0059cbd0): Kite breaks the object
    /// in front of him (act 25, held; ItemObjMenu, TrapObjMenu).
    BreakSomething,
    /// `plw->AttackCancel()` (gcmn 0x0059cad0): his swing or break stops.
    AttackCancel,
    /// What the talk and shop menus ask of the world ([`talk::TalkReq`]).
    Talk(talk::TalkReq),
    // --- PERSONAL's and OPTION's pages (menus/personal.rs and after) ---
    /// `ccUseItemRequest(plw, target, code, arg)` with an argument: the
    /// Grunty Flute (key item 15/49) calls the Grunty of slot `arg`
    /// (`ccPgAdultCheck`'s answer).
    UseItemArg { target: u32, code: i32, arg: i32 },
    /// `ccGame::ChangeRequest(num, sf)`: Log Out leaves The World for its
    /// top page (4, 7, as the desktop's World icon does); OPTION's Title
    /// Screen, OK twice, for the title (1, 7: the mother task's soft reset;
    /// the menu stays up, doing nothing more, until it does). The desktop's
    /// `Request::ChangeMode`.
    ChangeMode { num: i32, sf: i32 },
    /// `WORLD_MAN::GoField()` (menu 86, `TransFieldMenu`): back to the
    /// field from the area the party stands in, 121 frames after the menu
    /// opened.
    GoField,
    /// Equipment (63), a piece changed: `ccChar::CalcReal(1)` on the
    /// member `target`. The save's record (`real`, `tune`, equipment,
    /// skills, items) is already written; the runtime works the character
    /// out again from it: its added effects (condition 2-6, the pieces'
    /// largest) and maximum HP and SP (the record's).
    CalcReal { target: u32 },
    /// Equipment (63), with [`Request::CalcReal`]: `ccThEquipMenu`'s
    /// `ccSPC::ChangeEquip(category, member, item)`: the runtime puts the
    /// new piece's model on the member `target` (category 0-5 the weapon,
    /// 6-9 head, body, arm, leg). The menu takes it as done a frame later.
    ChangeEquip { target: u32, cat: i32, item: i32 },
    /// PARTY's Remove (69) and Disband (70): `ccParty::DelMember(slot)`
    /// (gcmn 0x0059cf60) on party slot 1 or 2: `disbandSpc(id)` (the member
    /// leaves), the slot's character and id cleared, the count down. The
    /// menu's own world already has the slot empty that frame.
    DelMember(i32),
    // --- The OPTION pages (menus 13-20, `crate::menus::option`); the save's
    // fields are already written when these come. ---
    /// Adjust Screen: `ccSystem::SetDisplayOffset(x, y)`, every frame the
    /// page is open: the runtime moves the whole picture by x (-48..48 in
    /// steps of 3, GS DISPLAY DX units) and y (-16..16 lines). The desktop's
    /// `Request::DisplayOffset`.
    DisplayOffset { x: i32, y: i32 },
    /// Sound: `ccSaveData::SetSoundEnv()`, when a setting moved: the runtime
    /// sets the sound driver's main, BGM and SE volumes (0-256,
    /// `ccSetMainVol`, `ccSetBgmVol`, `ccSetSeVol`) and its output
    /// (`ccSetOutputMode`: 0 mono, 1 stereo). The desktop's
    /// `Request::SoundEnv`.
    SoundEnv { main: i32, bgm: i32, se: i32, output: i32 },
    /// Vibrate, OK on a row: `ccPad::actuaterSw = on` (the runtime turns
    /// the pad's vibration on or off), and switching it on buzzes the pad
    /// once (`ccPad::SetActuater(pad 0, 1, 160, 200)`). The desktop's
    /// `Request::Vibration`.
    Vibration { on: bool },
    /// Controller, OK on a row: `setCameraCtrlType(t)`: the runtime puts the
    /// field camera's controls in scheme t (0 A-1, 1 A-2, 2 B-1, 3 B-2:
    /// `camCtrlType` 1, 1, 0, 0 and `camRevLR` 1, 0, 1, 0). The desktop's
    /// `Request::CameraType`.
    CameraType(i32),
}

/// `ccNoiz`'s setters as the menus call them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Noise {
    /// `SetNoiz(a, b, c)`: 0 off.
    Set(i32, i32, i32),
    /// `SetNoizRn(n)`, `SetNoizBs(n)`, `SetNoizBr(n)`.
    Rn(i32),
    Bs(i32),
    Br(i32),
}

/// The field UI.
pub struct FieldUi {
    pub ctrl: MenuCtrl,
    /// Not the game's: the HUD drawn this size (0.5 to 1; 1 is the game's),
    /// each part shrunk toward its own corner ([`spr::Anchor`]).
    pub hud_scale: f32,
    texts: Texts,
    textures: Textures,
    fonts: Fonts,
    requests: Vec<Request>,
    draws: Vec<ctrl::Draw>,
    /// The frame's world, target and previous target as the menu task left
    /// them when it stopped in `ccUseItemRequest`.
    held: Option<(World, Option<CharInfo>, Option<CharInfo>)>,
    /// The set-up's screen: while `ccSetupGameCtrl` runs the event passes
    /// at phases 0 and 2 (`setup_on`), before the menu task exists, the
    /// scripts' windows are the event's own there, as on the desktop's
    /// set-up (`piney_desktop::setup`).
    setup: SetupScreen,
    setup_on: bool,
    /// `ccThDfComp`, while it runs.
    df_comp: Option<dfcomp::DfComp>,
}

impl FieldUi {
    /// [`Request::Flash`] carried out: `ccScFade::EntryFlash(menuFade, 8,
    /// 0x3040c0c0, 0, 0, 512, 448)` (gcmn `GateMenu` 0x0055cca8).
    pub fn gate_flash(&mut self) {
        self.ctrl.menu_fade.entry_flash(8, 0x3040_c0c0);
    }

    /// The task's start: `new ccMenuCtrl` (0x0051c140), its textures and
    /// the menu tables.
    pub fn new(iso: &mut Iso, archive: Arc<Archive>) -> piney_data::Result<FieldUi> {
        let volume = iso.volume()?;
        let mut texts = Texts::read(volume)?;
        let events = piney_event::official::events(iso)
            .map_err(|e| piney_data::Error::Format(format!("the event scripts: {e}")))?;
        texts.read_tutorial(&events).map_err(piney_data::Error::Format)?;
        texts.hack.scene =
            piney_desktop::assets::SceneFile::read(&archive, menus::hack::HACK_FILE).ok().map(std::rc::Rc::new);
        let fonts = piney_desktop::assets::read_fonts(volume, &archive)?;
        let textures = Textures::read(&archive, &texts);
        let mut ui = FieldUi::with(texts, textures, fonts);
        ui.ctrl.talk.record.save_sys.volume = volume;
        Ok(ui)
    }

    /// From tables already read (tests build their own).
    pub fn with(texts: Texts, textures: Textures, fonts: Fonts) -> FieldUi {
        let ctrl = MenuCtrl::new(&texts);
        let setup = SetupScreen::with(fonts.clone(), textures.window.clone());
        FieldUi {
            ctrl,
            hud_scale: 1.0,
            texts,
            textures,
            fonts,
            requests: Vec::new(),
            draws: Vec::new(),
            held: None,
            setup,
            setup_on: false,
            df_comp: None,
        }
    }

    /// What the frames since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// The tables and texts in use.
    pub fn texts(&self) -> &Texts {
        &self.texts
    }

    /// `ccMenuCtrl::CheckMenuType()`: the menu open, -1 none, 88 while one
    /// opens or closes.
    pub fn menu_type(&self) -> i32 {
        self.ctrl.check_menu_type()
    }

    /// One pass of `ccThMenu`. `save` is the game's record (with the event
    /// manager's operation lock); `count` is `ccSys.count`.
    pub fn step(&mut self, pad: &Pad, world: &World, save: &mut SaveState, count: u32) -> Frame {
        self.run(pad, world, save, count);
        let faces = self.ctrl.face_tex;
        render::frame(&self.draws, &self.textures, &self.fonts, &save.names(), &faces, self.hud_scale)
    }

    /// [`FieldUi::step`] drawing the menu layer into the frame the field is
    /// building.
    pub fn step_into(
        &mut self,
        pad: &Pad,
        world: &World,
        save: &mut SaveState,
        count: u32,
        ctx: &mut piney_desktop::anm::Ctx,
    ) {
        self.run(pad, world, save, count);
        let faces = self.ctrl.face_tex;
        render::draw(&self.draws, &self.textures, &self.fonts, &save.names(), &faces, ctx, self.hud_scale);
    }

    /// `ccThMenu`'s first run in each scene's `ccSetupGameCtrl`, after the
    /// event passes (`ccMenu` is NULL through them) and before
    /// `WORLD_MAN::GO`: a whole new `ccMenuCtrl` (gcmn 0x0051c140). Its
    /// `ccNoiz` (main 0x001bb080) draws its bands from the game's `rand()`
    /// (`save.rand`, moved on); its faces are the party's member ids
    /// (`ccCheckMenuFaceNameParty`). [`MenuCtrl::keep`] says what stays.
    /// From Mutation on it counts the town's grown Grunties (`town_server`
    /// is `game.server` in a town, None elsewhere).
    pub fn menu_task_started(&mut self, save: &mut SaveState, party: [i32; 3], town_server: Option<i32>) {
        let mut rng = std::mem::take(&mut self.ctrl.rng);
        rng.load(save.rand);
        let old = std::mem::replace(&mut self.ctrl, MenuCtrl::with_rand(&self.texts, rng));
        self.ctrl.keep(old);
        self.ctrl.count_pg_adults(self.texts.volume, &save.save, town_server);
        // The old task's frame went with it; the new one has drawn nothing.
        self.draws.clear();
        self.held = None;
        if let Some(r) = self.ctrl.rng.state() {
            save.rand = r;
        }
        let plcol = save.save.u8(piney_data::save::offset::PLCOL) != 0;
        for (slot, id) in party.into_iter().enumerate().filter(|&(_, id)| id >= 0) {
            self.set_menu_face(slot, id, plcol);
        }
    }

    /// gcmn's menu data as this scene leaves it, for the next
    /// ([`MenuGlobals`]).
    pub fn globals(&self) -> MenuGlobals {
        self.ctrl.globals()
    }

    /// The last scene's gcmn data: a change of scene in The World keeps the
    /// overlay, and with it every list's cursor.
    pub fn set_globals(&mut self, g: MenuGlobals) {
        self.ctrl.set_globals(g);
    }

    fn run(&mut self, pad: &Pad, world: &World, save: &mut SaveState, count: u32) {
        // ccThSaveSys (priority 20) before ccThMenu (34).
        menus::record::save_sys_task(&mut self.ctrl.talk.record, &save.save);
        self.ctrl.rng.load(save.rand);
        self.ctrl.talk.load_cc(&save.cc);
        let mut x = ctrl::Ctx {
            pad,
            world: world.clone(),
            save,
            texts: &self.texts,
            fonts: &self.fonts,
            req: &mut self.requests,
            count,
            frame_rate: FRAME_RATE,
            target: world.target.clone(),
            target_prev: world.target_prev.clone(),
        };
        self.draws = self.ctrl.frame(&mut x);
        // ccThBook (35) after ccThMenu (34): its sends behind the menu's.
        if let Some(mut b) = self.ctrl.book.take() {
            let draws = b.frame(&mut self.ctrl, &mut x);
            self.draws.extend(draws);
            self.ctrl.book = Some(b);
        }
        if let Some(r) = self.ctrl.rng.state() {
            x.save.rand = r;
        }
        self.ctrl.talk.store_cc(&mut x.save.cc);
        // Stopped inside ccUseItemRequest: the rest of the frame runs on
        // the answer, with the world as the frame left it.
        self.held = self.ctrl.item_asked().then_some((x.world, x.target, x.target_prev));
        // ccThDfComp (priority 33) runs before ccThMenu (34); its banner
        // goes on the menu's layer.
        if let Some(d) = &mut self.df_comp {
            if d.main() {
                let packets = d.take();
                if !packets.is_empty() {
                    self.draws.insert(0, ctrl::Draw::Send(packets));
                }
            } else {
                self.df_comp = None;
            }
        }
    }

    /// `ccStartThread(ccThDfComp, 33, 4096)`: an area's last magic portal
    /// opened; `area` is `game+0x14` (1 a field, 2 a dungeon), with the
    /// field type and `game.dungeon` ([`dfcomp`]).
    pub fn df_comp(&mut self, area: i32, field_type: i32, dungeon: i32) {
        self.df_comp = Some(dfcomp::DfComp::new(area, field_type, dungeon));
    }

    /// Whether `ccThDfComp` runs.
    pub fn df_comp_on(&self) -> bool {
        self.df_comp.is_some()
    }

    /// The answer to [`Request::DataDrain`]: the drops (`drainItem`,
    /// +0x1ac, the aimed target's first) and how many (`+0x1f0`), which
    /// `DataDrainSubMenu` (67) hands out.
    pub fn drain_drops(&mut self, drops: &[i32], count: usize) {
        let d = &mut self.ctrl.talk.drain_item;
        d.fill(-1);
        for (k, &v) in drops.iter().take(d.len()).enumerate() {
            d[k] = v;
        }
        self.ctrl.drain_num = count.min(17) as i32;
    }

    /// The answer to [`Request::StreamMenu`]: its stream has ended.
    /// `ccChatMsg::OpenChat(ch, text)`: a chat balloon over the character
    /// with handle `who` ([`CharInfo::handle`]) for two seconds.
    pub fn open_chat(&mut self, who: u32, text: &[u8], names: &piney_desktop::kanji::Names) {
        self.ctrl.chat.open(who, text, &self.fonts, names);
    }

    /// `ccChatMsg::CloseChat()`: every balloon down.
    pub fn close_chat(&mut self) {
        self.ctrl.chat.close();
    }

    /// `ccSpcShoutOperationName()` (gcmn 0x005a17e0): the leader (`who`)
    /// calls the party's strategy in a balloon, `chatActionStr`'s piece 5,
    /// `chatMenuStr`'s `operation + 1` and `chatActionStr`'s 6.
    pub fn shout(&mut self, who: u32, operation: i32, names: &piney_desktop::kanji::Names) {
        let t = &self.texts;
        let piece =
            |v: &Vec<Vec<u8>>, k: i32| usize::try_from(k).ok().and_then(|k| v.get(k)).cloned().unwrap_or_default();
        let mut line = piece(&t.chat_action, 5);
        line.extend_from_slice(&piece(&t.chat_str, operation + 1));
        line.extend_from_slice(&piece(&t.chat_action, 6));
        self.ctrl.chat.open(who, &line, &self.fonts, names);
    }

    /// The characters with a chat balloon up: the runtime answers where
    /// they are in [`World::chat_at`].
    pub fn chat_speakers(&self) -> Vec<u32> {
        self.ctrl.chat.speakers()
    }

    /// The answer to [`Request::BookStream`]: the book's stream has ended
    /// (`ccThExecuteStream`'s param back to -1).
    pub fn book_stream_done(&mut self) {
        if let Some(b) = self.ctrl.book.as_mut() {
            b.stream_done = true;
        }
    }

    /// Whether a Ryu Book is open (`ccThBook` runs).
    pub fn book_on(&self) -> bool {
        self.ctrl.book.is_some()
    }

    /// Ryu Book `page` read as `ccUseItemRequest`'s book branch starts it
    /// (gcmn 0x0057c154): `ccThBook` started, `CloseChat`, `bgStatus` 3,
    /// the menu task held in the use until the book's state is 0 (the
    /// harnesses' entry; a key item's use goes through its steps).
    pub fn start_book(&mut self, page: i32) {
        self.ctrl.book = Some(Box::new(book::Task::new(page)));
        self.ctrl.chat.close();
        self.ctrl.bg_status = 3;
        self.ctrl.item = Some(menus::useitem::ItemRun::book(menus::useitem::Resume::Ocarina));
        self.ctrl.cont = Some(ctrl::Cont { woke: false, cursors: false, after: ctrl::After::Item });
    }

    pub fn stream_menu_done(&mut self) {
        self.ctrl.stream_menu.done = true;
    }

    /// `StreamMenu` asked for: `streamNum` 20, `streamFlag` the member in
    /// `slot`, `openReqNum` 0x104a (74 without the window's fade in),
    /// `mode` 0, `firstTime` 1 (ccBoss01's `OnDataDrainAtk`).
    pub fn open_stream_menu(&mut self, num: i16, flag: i16) {
        let c = &mut self.ctrl;
        c.stream_num = num;
        c.stream_flag = flag;
        c.open_req = 0x1000 | 74;
        c.mode = 0;
        c.first_time = 1;
    }

    /// The answer to [`Request::DrainMovie`]: the stream has ended
    /// (`ccThExecuteStream`'s param back to -1).
    pub fn drain_movie_done(&mut self) {
        self.ctrl.data_drain.movie_done = true;
    }

    /// The answer to [`Request::DrainSideEffect`]: the effect (0-30), a
    /// level lost, and effect 29's item lost (-1 none).
    pub fn drain_side_effect(&mut self, id: i32, level_down: bool, lost: i32) {
        self.ctrl.data_drain.side = Some(menus::drain::Side { id, level_down, lost });
        self.ctrl.trap_num = lost;
    }

    /// The menu task stopped in `ccUseItemRequest` ([`Request::UseItem`]):
    /// the runtime answers with [`FieldUi::answer_item`] this frame.
    pub fn item_asked(&self) -> bool {
        self.ctrl.item_asked()
    }

    /// The runtime's answer to [`Request::UseItem`]: the use's steps
    /// (`piney_battle::item::use_item_request`, none for nothing to do).
    /// The frame goes on in the call to its first breath, the menu's layer
    /// drawn into `ctx` when given.
    pub fn answer_item(
        &mut self,
        steps: Vec<piney_battle::item::Step>,
        pad: &Pad,
        save: &mut SaveState,
        count: u32,
        ctx: Option<&mut piney_desktop::anm::Ctx>,
    ) {
        let Some((world, target, target_prev)) = self.held.take() else { return };
        self.ctrl.answer_item(steps);
        self.ctrl.rng.load(save.rand);
        self.ctrl.talk.load_cc(&save.cc);
        let mut x = ctrl::Ctx {
            pad,
            world,
            save,
            texts: &self.texts,
            fonts: &self.fonts,
            req: &mut self.requests,
            count,
            frame_rate: FRAME_RATE,
            target,
            target_prev,
        };
        self.draws = self.ctrl.item_frame(&mut x);
        if let Some(r) = self.ctrl.rng.state() {
            save.rand = r;
        }
        self.ctrl.talk.store_cc(&mut save.cc);
        if let Some(ctx) = ctx {
            let faces = self.ctrl.face_tex;
            render::draw(&self.draws, &self.textures, &self.fonts, &save.names(), &faces, ctx, self.hud_scale);
        }
    }

    /// The last step's drawing, as `Disp` sent it.
    pub fn draws(&self) -> &[ctrl::Draw] {
        &self.draws
    }

    // --- The seam to the event engine (`piney_event::host::Host`, Place::Field) ---

    /// `Host::message_open`: the speech, information or tutorial window
    /// (on the set-up's screen while the set-up runs).
    pub fn message_open(&mut self, call: &piney_event::host::MessageCall<'_>, save: &SaveState) {
        if self.setup_on {
            self.setup.update(save);
            message::open_setup(&mut self.setup, call, &mut self.requests);
            return;
        }
        message::open(&mut self.ctrl, call, &save.names(), &mut self.requests);
    }

    /// `ccMsg->Open(rec, rec->name, -1, -1)` from a story map's scene (area
    /// 43's `kiteSelfTalk`): the speech window with the record at `rec`, no
    /// voice; [`FieldUi::message_check`] answers it.
    pub fn story_message(&mut self, rec: u32, save: &SaveState) {
        let r = self.texts.talk.ev_msg(rec);
        let name = self.texts.talk.record(rec).and_then(|d| d.name).map(piney_data::tables::sjis::encode);
        let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
        self.ctrl.msg.open_record(r.emode, name.as_deref(), &lines, &save.names());
    }

    /// `Host::message_check`: `ccMsg->Check(0)` with this frame's pad: 0
    /// while the window waits, else 1 or the question's answer.
    pub fn message_check(&mut self, pad: &Pad, save: &SaveState) -> i32 {
        let mut req = Vec::new();
        let r = if self.setup_on {
            let r = self.setup.message_check(pad);
            req = self.setup.take_requests();
            r
        } else {
            self.ctrl.msg.check(pad.push.bits(), pad.repeat.bits(), save.ok(), &mut req)
        };
        self.desktop_requests(req);
        r
    }

    /// `Host::message_close`: `ccMsg->Close()`, or on the set-up's screen
    /// the event deleting its window.
    pub fn message_close(&mut self) {
        if self.setup_on {
            self.setup.close_message();
            return;
        }
        self.ctrl.msg.close();
    }

    /// The window's sounds and voice stops.
    fn desktop_requests(&mut self, req: Vec<piney_desktop::Request>) {
        for q in req {
            match q {
                piney_desktop::Request::Se(s) => self.requests.push(Request::Se(s.0)),
                piney_desktop::Request::VoiceStop => self.requests.push(Request::VoiceStop),
                _ => {}
            }
        }
    }

    /// Whether the set-up's event passes run (`ccSetupGameCtrl` between
    /// `ccEnableThEvent(0)` and the load's end): the scripts' windows open
    /// on its screen. Its window goes with it.
    pub fn set_setup(&mut self, on: bool) {
        if self.setup_on && !on {
            self.setup.close_message();
        }
        self.setup_on = on;
    }

    /// One frame of the set-up's screen, when it shows a window: black and
    /// the event's window (its `Disp`). None: nothing up.
    pub fn setup_frame(&mut self, pad: &Pad) -> Option<Frame> {
        if !self.setup_on || self.setup.window().is_none() {
            return None;
        }
        let f = self.setup.step(pad);
        let req = self.setup.take_requests();
        self.desktop_requests(req);
        Some(f)
    }

    /// `Host::announce`: `ccEvent::DispInfo`'s window (the event plays sound
    /// 74 itself). `member_name` is `spcParam[pc].base.name`.
    pub fn announce(&mut self, a: piney_event::host::Announce, member_name: &[u8], save: &SaveState) {
        let lines = message::announce_lines(a, member_name, &self.texts.announce);
        if self.setup_on {
            self.setup.update(save);
            let refs: Vec<&[u8]> = lines.iter().map(|l| l.as_deref().unwrap_or(&[])).collect();
            self.setup.open_message(piney_desktop::message::MessageKind::Info, 0, None, &refs);
            return;
        }
        let l = |i: usize| lines[i].as_deref();
        self.ctrl.msg.change_info([l(0), l(1), l(2), l(3)], &save.names());
    }

    /// `ccEvent::DispInfo`'s window with the lines `Execute` composed (a
    /// gate address or a desktop item; at most four).
    pub fn announce_lines(&mut self, lines: &[Vec<u8>], save: &SaveState) {
        let l = |i: usize| lines.get(i).map(Vec::as_slice);
        self.ctrl.msg.change_info([l(0), l(1), l(2), l(3)], &save.names());
    }

    /// `Host::menu_ban`.
    pub fn menu_ban(&mut self, on: bool) {
        self.ctrl.menu_ban(on);
    }

    /// `ccMenu->openReqNum = num`.
    pub fn open_menu(&mut self, num: i16) {
        self.ctrl.open_req = num;
    }

    /// `Host::open_menu`, the event's `menu num=` (`ccEvent::Execute`
    /// 0x001b1e68): `openReqNum = num`, `mode = 1`, `firstTime = 1`. With
    /// `mode` 1, `OpenMenu` neither puts the other tasks to sleep nor
    /// freezes their layers: the field keeps moving under the tutorials'
    /// menus.
    pub fn event_menu(&mut self, num: i16) {
        self.ctrl.open_req = num;
        self.ctrl.mode = 1;
        self.ctrl.first_time = 1;
    }

    /// `Host::target_forbid`.
    pub fn target_forbid(&mut self, on: bool) {
        self.ctrl.target_forbid = i16::from(on);
    }

    /// `Host::map_on`: `ccMenu.mapStatus = 1`.
    pub fn map_on(&mut self) {
        self.ctrl.map_status = 1;
    }

    /// `Host::noise`: `ccMenu.interNoiz`.
    pub fn noise(&mut self, level: i32) {
        self.ctrl.inter_noiz = level as i16;
    }

    /// `ccMenu->noiz->SetNoizBs(bs)` (main 0x001bb410), as the spells'
    /// blasts call it: the screen breaks up for `bs` frames, fading.
    pub fn noise_bs(&mut self, bs: i32) {
        self.ctrl.noiz.set_bs(bs);
    }

    // --- The seam of the talk and shop menus ([`talk`]) ---

    /// Who the action button spoke to: `cmndTarget` (the character with
    /// this handle in [`World`]) is `npcTbl[row]` (or a party member), on
    /// server `server`. Called when `ccThGameCtrl` opens menu 21-27 on it,
    /// before the step that opens it; the pages read its base parameters
    /// until the next call.
    pub fn talk_to(&mut self, target: Option<talk::TalkTarget>) {
        self.ctrl.talk.target = target;
    }

    /// The memory cards the Recorder saves to (none until given).
    pub fn set_card(&mut self, card: Box<dyn piney_desktop::card::MemoryCard>) {
        self.ctrl.talk.record.card = card;
    }

    /// Where `ccSaveSys` was left (`port`, `fileNum`): the game has one, so
    /// the last card slot and file the title's load or the desktop's Data
    /// screen used are where the Recorder's lists start.
    pub fn set_card_position(&mut self, port: i32, file: i32) {
        self.ctrl.talk.record.save_sys.port = port;
        self.ctrl.talk.record.save_sys.file_num = file;
    }

    /// A harness's numbers for NorainuMenu's `ccRand()`
    /// ([`talk::TalkState::cc_rand`]).
    pub fn set_cc_rand(&mut self, f: Box<dyn FnMut() -> i32 + Send>) {
        self.ctrl.talk.set_cc_rand(f);
    }

    /// The area the party stands in as `WORLD_MAN` holds it: its three
    /// keywords (A, B, C) through `SimGenerateCode` (the merged
    /// `WORDPARAM`, the story area). Area Information (menu 87) shows it;
    /// the runtime sets it on entering an area (the Chaos Gate's own
    /// simulations overwrite it, as they do `WORLD_MAN` in the game).
    pub fn set_area_words(&mut self, ids: [i32; 3], server: i32, save: &SaveState) {
        let flag71 = (save.save.i32(0x5bc0 + 4) as u32 >> 30) & 1 != 0;
        self.ctrl.generated = self.texts.words.generate(ids, server, flag71);
    }

    /// `ccMenuCtrl::AddSpcItem(ch, cat, id, num, 0)` (gcmn 0x00527950), as
    /// the events' `item_add` calls it for a companion
    /// ([`menus::talk::add_spc_item`] with no `ccThEquipMenu`, so it never
    /// breathes): the item given to party member `sid` (`member` its
    /// character) is worn, read or bagged; `world` is the frame's view (a
    /// town sells what the bag cannot hold). The requests it makes wait in
    /// the queue. Returns 0 bagged, 1 worn, 2 used.
    #[allow(clippy::too_many_arguments)]
    pub fn add_spc_item(
        &mut self,
        world: &World,
        save: &mut SaveState,
        member: u32,
        sid: i32,
        cat: i32,
        id: i32,
        num: i32,
    ) -> i32 {
        let pad = Pad::default();
        let mut x = ctrl::Ctx {
            pad: &pad,
            world: world.clone(),
            save,
            texts: &self.texts,
            fonts: &self.fonts,
            req: &mut self.requests,
            count: 0,
            frame_rate: FRAME_RATE,
            target: world.target.clone(),
            target_prev: world.target_prev.clone(),
        };
        match menus::talk::add_spc_item(&mut self.ctrl, &mut x, member, sid, cat, id, num, false) {
            menus::talk::Added::Done(r) => r,
            menus::talk::Added::Breathed(g) => menus::talk::add_spc_item_after(&mut x, g),
        }
    }

    /// [`Request::CalcReal`] as the menus see it: the member's `real` and
    /// `tune` into the save, its added effects and maximum HP and SP into
    /// `c` (for a runtime, or a test, without `ccChar::CalcReal` of its
    /// own).
    pub fn calc_real(&self, save: &mut SaveState, c: &mut world::CharInfo) {
        menus::equip::calc_real(&self.texts.pers.battle, save, c);
    }

    /// `SetMenuFace(code)` (0x00526940): party slot `slot` shows `charTbl`
    /// row `code`'s face (Kite's own until the bracelet: row 18).
    pub fn set_menu_face(&mut self, slot: usize, code: i32, plcol: bool) {
        if slot < 4 {
            self.ctrl.face_tex[slot] = if code == 0 && !plcol { 18 } else { code };
        }
    }
}

#[cfg(test)]
mod tests {
    use piney_battle::rand::{Rand, Rng as _};

    use crate::ctrl::MenuRand;
    use crate::menus::option::tests::field_ui;

    /// `ccMenuCtrl`'s `ccNoiz` (issue #52): its bands draw from the save's
    /// `rand()`, which moves on by every draw they made; a harness's
    /// fixed numbers leave it alone.
    #[test]
    fn the_menu_task_draws_its_noise_from_the_saves_rand() {
        let Some((mut ui, _, mut save)) = field_ui() else { return };
        let mut n = 0;
        let _ = piney_desktop::noiz::Noiz::new(&mut || {
            n += 1;
            0
        });
        assert!(n > 0, "the bands draw");
        save.rand = 0x1234_5678;
        ui.menu_task_started(&mut save, [-1; 3], None);
        let mut want = Rand(0x1234_5678);
        (0..n).for_each(|_| {
            want.rand();
        });
        assert_eq!(save.rand, want.0, "{n} draws");
        ui.ctrl.rng = MenuRand::Fixed(Box::new(|| 0));
        ui.menu_task_started(&mut save, [-1; 3], None);
        assert_eq!(save.rand, want.0, "a fixed source leaves the save's");
    }

    /// A room's `ccSetupGameCtrl` deletes the menu task and starts a new
    /// one, so every `ccMenuCtrl` member is the constructor's again: the
    /// events' bans and noise, the panels' fade, the protect marks, the
    /// mail clock. gcmn's lists and `BookOfs`, `WORLD_MAN`'s area and
    /// `ccSaveSys` are not members and stay; the faces are the party's.
    #[test]
    fn a_new_scene_makes_the_whole_menu_anew() {
        let Some((mut ui, _, mut save)) = field_ui() else { return };
        let fresh = |ui: &crate::FieldUi| crate::MenuCtrl::new(ui.texts());
        let c = &mut ui.ctrl;
        (c.forbid, c.target_forbid, c.inter_noiz, c.panel_alpha, c.mail_cnt) = (1, 1, 3, 128, 7);
        (c.panel_status, c.map_status, c.pl_attack, c.cursol_off) = (2, 2, 1, 1);
        c.protect[4] = 1;
        c.protect_cnt[4] = 30;
        (c.lists[0].select, c.lists[0].disp) = (3, 99);
        c.face_tex = [5, 6, 7, 8];
        c.book_ofs[1] = 19;
        c.talk.record.save_sys.port = 1;
        c.generated.ids = [9, 9, 9];
        let kept = (c.book_ofs.clone(), c.generated.clone());
        save.save.set_u8(piney_data::save::offset::PLCOL, 0);
        ui.menu_task_started(&mut save, [0, 2, -1], None);
        let (c, f) = (&ui.ctrl, fresh(&ui));
        assert_eq!(
            (c.forbid, c.target_forbid, c.inter_noiz, c.panel_alpha, c.mail_cnt),
            (f.forbid, f.target_forbid, f.inter_noiz, -48, 240)
        );
        assert_eq!((c.panel_status, c.map_status, c.pl_attack, c.cursol_off), (1, 1, 0, 0));
        assert_eq!((c.protect, c.protect_cnt), (f.protect, f.protect_cnt));
        assert_eq!((c.lists[0].select, c.lists[0].disp), (3, f.lists[0].disp), "gcmn's list, InitMenuList's disp");
        assert_eq!(c.face_tex, [18, 2, -1, -1], "Kite without the bracelet, member 2, none");
        assert_eq!((c.book_ofs.clone(), c.generated.clone()), kept);
        assert_eq!(c.talk.record.save_sys.port, 1);
    }
}
