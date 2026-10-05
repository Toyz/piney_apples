//! Modes 5 and 6, The World itself (`ccSetupNewGame` loading `GCMN.PRG`, then
//! `ccSetupGameCtrl`): the Root Towns (`piney-world`) with the field's menus and
//! HUD (`piney-fieldui`) and the event scripts ([`crate::field_host`]). A frame
//! runs `ccThEvent` (32), the world's tasks (`ccThGameCtrl` 33 ... the town 96),
//! then `ccThMenu` (34); a button's menu reaches the camera and player a frame
//! later than in the game. The set-up's passes (0, 2, then 4 at F0) are in
//! docs/engine/event-vm.md.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_event::host::LogHost;
use piney_event::state::ScriptSave as _;
use piney_event::vm::Vm;
use piney_fieldui::FieldUi;
use piney_input::Pad;
use piney_world::entry::Kind;
use piney_world::talk::{self, Shop, TalkRequest};
use piney_world::{Phase, Request, World};

use crate::field_host::{FieldHost, FieldState, MENU_LAYER, SceneChange};
use crate::mode::{Event, Mode};

/// Where `ccSetupGameCtrl` is with the event task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setup {
    /// No event task (a world entered without the scripts).
    Off,
    /// The fade out and the hold: the task idles (phase -1).
    Before,
    /// `ccEnableThEvent(0)` / `(2)`: waiting for the pass at that phase.
    Pass(i32),
    /// The passes made: the tasks start; `ccEnableThEvent(4)` at F0.
    Start,
    /// Play: one event frame per game frame.
    Play,
    /// `ChangeScene` asked for another area: the task sleeps in it and
    /// the field is not followed there.
    Left,
}

impl Setup {
    /// Whether `ccSetupGameCtrl` is past its event passes, at its load
    /// (`ccLoadResourceFL`) or later. A set-up a pass abandoned
    /// (`Left` before `Start`) never loads; the session latches this.
    pub fn passes_made(self) -> bool {
        matches!(self, Setup::Off | Setup::Start | Setup::Play)
    }
}

/// Frames a set-up pass may take before the runtime gives up waiting.
const PASS_FRAMES: u32 = 10_000;

/// A new game's save as the title hands it on (the boot's
/// `ccSaveData::Init`, `NewGame(1)`, `NewGame(0)`: [`new_game_save`]), as
/// `--mode world` enters with it; Log in adds `ccSetupNewGame`'s part
/// ([`crate::session::Session::in_world`]).
///
/// [`new_game_save`]: piney_fieldui::newgame::new_game_save
pub fn new_game_state(disc: &mut Iso) -> Result<SaveState, String> {
    piney_fieldui::newgame::new_game_save(disc, false)
}

/// The event task and the save as a new game brings them to Log in (for
/// `--mode world`): the boot's `ccStartEvent(1, 0)`; the desktop's passes and
/// play until mails 4 and 5 are read and event 1's block 2 has closed it; the
/// board's `ccStartThEvent` and passes, and Log in. Every window and wait
/// answers at once (`LogHost`).
pub fn new_game_events(disc: &mut Iso, state: &mut SaveState) -> Result<Vm, String> {
    let mut vm = crate::desktop::boot(disc, state)?;
    let mut h = LogHost::new(state.save.clone());
    let settle = |vm: &mut Vm, h: &mut LogHost| -> Result<(), String> {
        vm.start_thread(h);
        for p in [0, 2] {
            vm.enable(p);
            let mut n = 0;
            while !vm.enable_settled(p) {
                vm.frame(h);
                n += 1;
                if n > PASS_FRAMES {
                    return Err(format!("events: the pass at phase {p} does not end"));
                }
            }
        }
        vm.enable(4);
        for _ in 0..3 {
            vm.frame(h);
        }
        Ok(())
    };
    // The desktop.
    h.game.status = 2;
    settle(&mut vm, &mut h)?;
    for m in [4, 5] {
        if h.save.mail(m) != 0 && h.save.mail(m) < 4 {
            h.save.set_mail(m, 4);
        }
    }
    let mut n = 0;
    while h.save.flags(1) & piney_event::state::CLOSED == 0 {
        vm.frame(&mut h);
        n += 1;
        if n > PASS_FRAMES {
            return Err("events: event 1 does not close".into());
        }
    }
    vm.disable();
    // The board, then Log in.
    h.game.status = 3;
    settle(&mut vm, &mut h)?;
    vm.disable();
    state.save = h.save;
    state.operate = vm.operate();
    state.operate_set = -1;
    Ok(vm)
}

pub struct WorldMode {
    world: Box<World>,
    /// `ccThMenu`: the HUD, the menus and the event windows.
    ui: Box<FieldUi>,
    /// The event task.
    vm: Option<Vm>,
    /// What the scripts see of the field besides the world and the UI.
    st: FieldState,
    setup: Setup,
    /// Frames the current set-up pass has taken.
    pass_frames: u32,
    frames: u64,
    /// `ccSys.count`.
    count: u32,
    /// What the menus asked of the sound since the last take.
    events: Vec<Event>,
    /// The last menu the port closed at once (unported).
    unported: Option<i16>,
    /// The gate hack's screen is up: the world, asleep with its layers
    /// flipping again, draws nothing (`piney_fieldui::Request::WorldHidden`).
    hack_screen: bool,
    /// `ccGame.areaLevel`, which the Chaos Gate's keyword screen writes.
    area_level: i32,
    /// A Ryu Book's cover stream plays (`ccThBook`'s `ccThExecuteStream`).
    book_stream: bool,
    /// The words entered at the Chaos Gate (`ccEvent.areaCodeSet`), for the
    /// events.
    area_code_set: Option<[i16; 3]>,
    /// Where the Chaos Gate asked to go (`WORLD_MAN::SetGenerateCode` or
    /// `ChangeArea`), for the title.
    leaving: Option<String>,
    /// The minimap (`ROOTTOWN01::DrawMap`) and what `WORLD_MAN` keeps of it.
    map: Option<Box<piney_world::map::town::TownMap>>,
    map_st: piney_world::map::MapState,
    /// The effects that draw the town's own sprites (Dun Loireag's).
    town_fx: Option<Box<crate::town_fx::TownFx>>,
}

impl WorldMode {
    /// `ccSetupNewGame` and `ccSetupGameCtrl` in town `town` on `state`.
    pub fn enter(iso: &Path, archive: Arc<Archive>, mut state: SaveState, vm: Option<Vm>) -> Result<WorldMode, String> {
        repair_buttons(&mut state);
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let mut ui = FieldUi::new(&mut disc, archive.clone()).map_err(|e| format!("the field's menus: {e}"))?;
        // The constructor's faces (ccCheckMenuFaceNameParty, gcmn
        // 0x0056a930): party slot 0 is member 0, Kite, with the bracelet's
        // colours (saveData+0x6771) or without.
        ui.set_menu_face(0, 0, state.save.u8(offset::PLCOL) != 0);
        let stories = if vm.is_some() {
            crate::story::from_tables(piney_data::area::AreaTables::of(disc.volume().map_err(|e| e.to_string())?))
        } else {
            HashMap::new()
        };
        let mut st = FieldState::new(&state.save, stories);
        st.stream.iso = Some(iso.to_path_buf());
        st.stream.data = Some(archive.clone());
        st.announcements = crate::desktop::announcements(iso);
        let mut world = World::enter(&mut disc, archive.clone(), state).map_err(|e| format!("The World: {e}"))?;
        let t = &world.town().base;
        let map = piney_world::map::town::TownMap::open(&archive, world.volume(), t.no, &t.file.stem)
            .map_err(|e| tracing::warn!("the town's map: {e}"))
            .ok()
            .map(Box::new);
        let town_fx = Some(crate::town_fx::TownFx::new(&archive, disc.volume().map_err(|e| e.to_string())?))
            .and_then(|f| f.map_err(|e| tracing::warn!("the town's effects: {e}")).ok())
            .map(Box::new);
        let map_st = piney_world::map::MapState::new(world.state());
        let setup = if vm.is_some() { Setup::Before } else { Setup::Off };
        world.set_loading(vm.is_some());
        let mut events = Vec::new();
        {
            let s = &world.state().save;
            events.push(crate::mode::voice_options(s));
            events.push(Event::GameArea(0));
        }
        Ok(WorldMode {
            world: Box::new(world),
            ui: Box::new(ui),
            vm,
            st,
            setup,
            pass_frames: 0,
            frames: 0,
            count: 0,
            events,
            unported: None,
            hack_screen: false,
            area_level: 0,
            book_stream: false,
            area_code_set: None,
            leaving: None,
            map,
            map_st,
            town_fx,
        })
    }

    /// Where the set-up is with the event task.
    pub fn setup(&self) -> Setup {
        self.setup
    }

    /// Every host call the scripts made, with its frame.
    #[allow(dead_code)]
    pub fn calls(&self) -> &[(u64, String)] {
        &self.st.calls
    }

    /// Whether the scripts play a stream.
    #[cfg(test)]
    pub fn streaming(&self) -> bool {
        self.st.stream.player.is_some()
    }

    /// The playing stream's frame (`ccGetStreamFrame`), once it has
    /// stepped.
    #[cfg(test)]
    pub fn stream_frame(&self) -> Option<u32> {
        self.st.stream.player.as_ref().and_then(|p| match p.frame() {
            (f, true) => Some(f),
            _ => None,
        })
    }

    /// `game.server`: the server of the town the player is in.
    #[cfg(test)]
    pub fn server(&self) -> i32 {
        self.st.game.server
    }

    /// `game.town`: the town the player is in.
    #[cfg(test)]
    pub fn town_number(&self) -> i32 {
        self.st.game.town
    }

    /// The area the scripts sent the player to, once they did.
    pub fn scene_change(&self) -> Option<&SceneChange> {
        self.st.change.as_ref()
    }

    /// The event task, for tests.
    #[allow(dead_code)]
    pub fn vm(&self) -> Option<&Vm> {
        self.vm.as_ref()
    }

    /// The world, for tests.
    #[allow(dead_code)]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The world: the session hands it the party the last area left; tests
    /// place the player.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// The party the last area left (`ccSpcManager`, `ccPartyManager`), and
    /// each slot's menu face from it: `ccCheckMenuFaceNameParty` (gcmn
    /// 0x0056a930) reads `memberID[slot]` whenever it is asked.
    pub fn set_spcs(&mut self, spcs: piney_world::party::Spcs) {
        self.world.set_spcs(spcs);
        let plcol = self.world.state().save.u8(offset::PLCOL) != 0;
        for (slot, &id) in self.world.party().iter().enumerate() {
            if id >= 0 {
                self.ui.set_menu_face(slot, id, plcol);
            }
        }
    }

    /// Party slot `slot`'s menu face for member `id`, as the Party menu
    /// sets it once `ccThPartyAdd` is done (the console's `invite_party`).
    pub fn set_menu_face(&mut self, slot: usize, id: i32) {
        let plcol = self.world.state().save.u8(offset::PLCOL) != 0;
        self.ui.set_menu_face(slot, id, plcol);
    }

    /// The field UI, for tests.
    #[allow(dead_code)]
    pub fn ui(&self) -> &FieldUi {
        &self.ui
    }

    pub fn ui_mut(&mut self) -> &mut FieldUi {
        &mut self.ui
    }

    /// One frame of the event task (`ccThEvent`, priority 32), before the
    /// world's tasks; then `ccSetupGameCtrl`'s side of the passes.
    fn event_frame(&mut self) {
        let Some(vm) = &mut self.vm else { return };
        if self.setup == Setup::Left {
            return;
        }
        // F0: ccSetupGameCtrl's ccEnableThEvent(4), after GO and the start
        // positions, before the frame's Breath.
        if self.setup == Setup::Start && self.world.phase() == Phase::Play(0) {
            vm.enable(4);
            self.setup = Setup::Play;
        }
        // The operation the field recorded last frame (CheckOperate), which
        // the task sees and then clears for this one.
        vm.set_operate_set(self.world.state().operate_set);
        {
            let mut h = FieldHost { world: &mut self.world, ui: &mut self.ui, st: &mut self.st };
            vm.frame(&mut h);
        }
        let s = self.world.state_mut();
        s.operate = vm.operate();
        s.operate_set = vm.operate_set();
        self.world.set_event_targets(&vm.mng.targets);
        self.events.append(&mut self.st.events);
        if self.st.change.is_some() {
            // ChangeRequest(6, 7): ccDisableThEvent, and every task asleep
            // (the task itself inside the instruction).
            vm.disable();
            self.setup = Setup::Left;
            self.world.set_asleep(true);
            return;
        }
        match self.setup {
            // The fade out and hold are over: ccInitRand, ccStartThEvent,
            // WORLD_MAN::SetEventData, ccEnableThEvent(0).
            Setup::Before if self.world.phase() == Phase::Hold(piney_world::HOLD_FRAMES - 1) => {
                let mut h = FieldHost { world: &mut self.world, ui: &mut self.ui, st: &mut self.st };
                vm.start_thread(&mut h);
                vm.enable(0);
                self.setup = Setup::Pass(0);
                self.pass_frames = 0;
            }
            Setup::Pass(p) => {
                self.pass_frames += 1;
                if !vm.enable_settled(p) && self.pass_frames < PASS_FRAMES {
                    return;
                }
                // The game would wait on; the port gives up, and says so
                // (a block that waits inside the pass holds the screen).
                if !vm.enable_settled(p) {
                    tracing::warn!("the town's set-up pass at phase {p} did not end in {PASS_FRAMES} frames; going on",);
                }
                // A pass that asked for another mode (ChangeRequest's
                // ccDisableThEvent): ccSetupGameCtrl reads game+4 and
                // returns (0x00168ca8, 0x00168fa4).
                if vm.mng.enable_phase < 0 {
                    self.setup = Setup::Left;
                    return;
                }
                if p == 0 {
                    vm.enable(2);
                    self.setup = Setup::Pass(2);
                    self.pass_frames = 0;
                } else {
                    // ccEntryEventMng, at the entry control's set-up: the
                    // entries the passes registered go to the world before
                    // its merchants and walking PCs.
                    let entries: Vec<[i16; 4]> = vm.mng.entries.iter().copied().filter(|e| e[0] >= 0).collect();
                    for [ty, code, marker, param] in entries {
                        if !self.world.entry(ty, code, marker, param) {
                            tracing::warn!("events: entry {ty} {code} at marker {marker} not placed");
                        }
                    }
                    self.world.set_loading(false);
                    self.setup = Setup::Start;
                }
            }
            _ => {}
        }
    }

    /// Leave The World: the save and the event task, for the next mode.
    pub fn leave(self) -> (SaveState, Option<Vm>) {
        (self.world.state_out(), self.vm)
    }

    /// `ccSys+0x358` at the set-up's `ccInitRand`: the draws before the
    /// walking PCs are picked ([`piney_world::World::set_rand_count`]).
    pub fn set_rand_count(&mut self, count: u32) {
        self.world.set_rand_count(count);
    }

    /// The memory card the Recorder saves to (the directory standing for
    /// slot 1, as the desktop's Data screen uses it) and where `ccSaveSys`
    /// was left (`port`, `fileNum`).
    pub fn set_card(&mut self, dir: Option<&Path>, (port, file): (i32, i32)) {
        if let Some(dir) = dir {
            self.ui.set_card(Box::new(piney_desktop::card::FilesCard::slot1(self.world.volume(), dir)));
        }
        self.ui.set_card_position(port, file);
    }

    /// Back from a field or dungeon: its scene drew the fade out
    /// (`World::skip_fade_out`).
    pub fn skip_fade_out(&mut self) {
        self.world.skip_fade_out();
    }

    /// What `ccThGameCtrl` asked the menus for.
    /// `ccChar::CalcReal(1)` as `ccPlayer::Main` (gcmn 0x005983c8) and
    /// `ccFellow::Main` (0x0041b6e8) run it each frame on the party: each
    /// member's `real` and `tune` into the save, which Status reads.
    fn calc_real_party(&mut self) {
        let ids: Vec<i32> = self.world.party().iter().copied().filter(|&c| c >= 0).collect();
        for code in ids {
            let at = piney_data::save::by_id::spc_param(code as usize);
            let mut m = member(self.world.state(), at, Vec::new(), handle(Kind::Spc, code));
            self.ui.calc_real(self.world.state_mut(), &mut m);
        }
    }

    fn talk(&mut self, t: TalkRequest) {
        // The talk and shop menus read the character spoken to (cmndTarget,
        // its npcTbl row) from before the step that opens them.
        use piney_fieldui::talk::{Speaker, TalkTarget};
        let spoken_to = match t {
            TalkRequest::Talk { npc, .. } | TalkRequest::Shop { npc, .. } => Some((Kind::Npc, npc)),
            // A party member (21), an administrator (23), a Grunty breeder
            // (27), a dog (44), a Grunty (45 grown, 46 young).
            TalkRequest::Menu { menu: 21 | 23 | 27 | 44 | 45 | 46, kind, code } => Some((kind, code)),
            _ => None,
        };
        if let Some((kind, code)) = spoken_to {
            let who = if kind == Kind::Spc { Speaker::Spc(code) } else { Speaker::Npc(code) };
            self.ui.talk_to(Some(TalkTarget { handle: handle(kind, code), who }));
        }
        // openReqNum, mode, firstTime.
        let c = &mut self.ui.ctrl;
        let mut open = |menu: i16, mode: i16| {
            c.open_req = menu;
            c.mode = mode;
            c.first_time = 1;
        };
        match t {
            TalkRequest::Open { menu } => open(menu, 0),
            TalkRequest::ClearAttack => c.pl_attack = 0,
            TalkRequest::Talk { .. } => open(22, 1),
            TalkRequest::Shop { shop, .. } => open(
                match shop {
                    Shop::Recorder => 25,
                    Shop::Fairy => 26,
                    Shop::Weapon | Shop::Item | Shop::Magic => 24,
                },
                1,
            ),
            TalkRequest::Menu { menu, .. } => open(menu as i16, talk::action_mode(menu, 0)),
            // CheckOperate(9) refused it: no menu; the event task sees the
            // operation and the character next frame (operateSet,
            // operateTarget), as it sees ccThGameCtrl's call after its own
            // Breath.
            TalkRequest::Event { .. } => {
                if let Some(vm) = &mut self.vm {
                    vm.mng.operate_target = self.world.command_target_ref();
                }
                self.world.close_menu();
            }
        }
    }

    /// The Grunties' events of the frame ([`piney_world::grunty::
    /// GruntyEvent`]): sounds at the listener, effects to start, voices
    /// (a grown kind's through `ccCheckVoiceGrp` of its row) and the chat
    /// balloon over the Grunty.
    fn grunty_events(&mut self, names: &piney_desktop::kanji::Names, starts: &mut Vec<crate::town_fx::Start>) {
        use crate::town_fx::Start;
        use piney_world::grunty::GruntyEvent as E;
        let events = self.world.take_grunty_events();
        if events.is_empty() {
            return;
        }
        let cam = self.world.camera().active();
        let ear = Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
        for (code, e) in events {
            let c = 4 << 24 | (code as u32 & 0xff_ffff);
            match e {
                E::Note { param, pos, attribute } => {
                    if let Some(se) = piney_audio::se3d::inu_note(self.world.volume(), param, attribute) {
                        self.events.push(Event::Se3d { n: se.code, pos, note: se.note, ear });
                    }
                }
                E::Se3d { n, pos } => {
                    if let Ok(n) = usize::try_from(n) {
                        self.events.push(Event::Se3d { n, pos, note: None, ear });
                    }
                }
                E::Smoke { pos, v, scale, life, kind } => starts.push(Start::Puff { pos, v, scale, life, kind }),
                E::Evolve { .. } => starts.push(Start::Evolve(c)),
                E::Grow { pos, height } => starts.push(Start::Grow(pos, height)),
                E::Voice { grp, n } => {
                    let grp = if grp >= 141 {
                        piney_data::tables::fieldui::of(self.ui.texts().volume)
                            .voice_groups()
                            .get((grp - 141) as usize)
                            .copied()
                            .unwrap_or(-1)
                    } else {
                        grp
                    };
                    let save = &self.world.state().save;
                    self.events.push(crate::mode::voice_options(save));
                    self.events.push(Event::Voice { event: grp, msg: n });
                }
                E::VoiceStop => self.events.push(Event::VoiceStop),
                E::Chat(k) => {
                    let text = self.world.grunty_chat_text(k);
                    self.ui.open_chat(handle(Kind::Npc, code), &text, names);
                }
                E::ChatClose => self.ui.close_chat(),
                // Carried out by the world; off the lists (its own flag).
                E::Camera(_) | E::CamPos(_) | E::CamView(_) | E::Player { .. } | E::Dropped | E::Debug(_) => {}
            }
        }
    }

    /// What the menus asked of the rest of the game.
    /// `ccThMenu` (34), after `ccThGameCtrl` and before the camera, the
    /// party and the entries (`ccSetupGameCtrl`'s priorities): the book's
    /// stream, the menus and their requests, which the town acts on this
    /// same frame.
    fn menu_task(&mut self, pad: &Pad, ctx: &mut Ctx) {
        let mut w = ui_world(&self.world, self.vm.as_ref(), self.area_level);
        w.chat_at = self
            .ui
            .chat_speakers()
            .into_iter()
            .filter_map(|h| {
                let (kind, code) = unhandle(h)?;
                let (listed, at) = self.world.chat_point(kind, code)?;
                Some(piney_fieldui::chat_msg::ChatAt { who: h, listed, at })
            })
            .collect();
        // The book's cover stream, a frame a menu frame until it ends.
        if self.book_stream
            && let Some(p) = &mut self.st.stream.player
        {
            match p.step(&self.st.pad, &mut self.events) {
                Some(f) => self.st.stream.frame = Some(f),
                None => {
                    self.world.set_rand(p.rand());
                    self.st.stream.player = None;
                    self.book_stream = false;
                    self.ui.book_stream_done();
                }
            }
        }
        self.ui.step_into(pad, &w, self.world.state_mut(), self.count, ctx);
        // The menu task stops in ccUseItemRequest (a book, a key item):
        // the use's rules on the town's party, and its steps back to the
        // task, which goes on this frame.
        loop {
            let mut asked = None;
            for r in self.ui.take_requests() {
                match r {
                    piney_fieldui::Request::UseItem { target, code } if self.ui.item_asked() => {
                        asked = Some((target, code, 0));
                    }
                    piney_fieldui::Request::UseItemArg { target, code, arg } if self.ui.item_asked() => {
                        asked = Some((target, code, arg));
                    }
                    r => self.menu_request(r),
                }
            }
            let Some((target, code, arg)) = asked else { break };
            let steps = self.world.use_item(unhandle(target), code, arg);
            self.ui.answer_item(steps, pad, self.world.state_mut(), self.count, Some(ctx));
        }
        // ccThGameCtrl's states 1-5 end when CheckMenuType() is -1.
        if self.world.targeting().in_menu && self.ui.menu_type() == -1 {
            self.world.close_menu();
        }
    }

    fn menu_request(&mut self, r: piney_fieldui::Request) {
        use piney_fieldui::Request as R;
        use piney_fieldui::talk::TalkReq;
        match r {
            R::Se(n) => self.events.push(Event::Se(n)),
            R::SeNote { n, note } => self.events.push(Event::SeNote { n: n.max(0) as usize, note: note as i8 }),
            R::CameraShake { power, cycle, time, dirc } => self.world.camera_shake([power, cycle, time, dirc]),
            // No drain in a town (the Skills menu refuses it); a movie asked
            // anyway ends at once rather than hold the menu.
            R::DrainMovie(_) => self.ui.drain_movie_done(),
            // No boss in a town: menu 74 is never opened there.
            R::StreamMenu(_) => self.ui.stream_menu_done(),
            // ccThBook's cover, stream 112 + the book, over the town; a
            // stream that cannot start counts as played.
            R::BookStream(Some(page)) => {
                let save = self.world.state_out();
                let started = match (self.st.stream.iso.clone(), self.st.stream.data.clone()) {
                    (Some(iso), Some(data)) => usize::try_from(112 + page)
                        .map_err(|_| format!("stream {}", 112 + page))
                        .and_then(|n| crate::stream::StreamPlayer::drain(&iso, &data, n, &save, &mut self.events)),
                    _ => Err("no disc for the book's stream".into()),
                };
                match started {
                    Ok(p) => {
                        self.st.stream.player = Some(p);
                        self.book_stream = true;
                    }
                    Err(e) => {
                        tracing::warn!("the book's stream: {e}; counted as played");
                        self.ui.book_stream_done();
                    }
                }
            }
            R::BookStream(None) => {
                if std::mem::take(&mut self.book_stream)
                    && let Some(p) = self.st.stream.player.take()
                {
                    self.world.set_rand(p.rand());
                }
            }
            R::StrParty(_) => {}
            R::SleepAll => self.world.set_asleep(true),
            R::WakeAll => self.world.set_asleep(false),
            R::VoiceStop => self.events.push(Event::VoiceStop),
            R::VoiceRequest { grp, msg } => {
                let save = &self.world.state().save;
                self.events.push(crate::mode::voice_options(save));
                self.events.push(Event::Voice { event: grp, msg });
            }
            R::Unported(n) => self.unported = Some(n),
            // EntryAffect on the gate (the circle) or a character.
            R::Affect { target, kind } => {
                if let Some((k, code)) = unhandle(target) {
                    self.world.affect(k, code, kind);
                }
            }
            // ccParty::AddMember (from ccThPartyAdd): the party is the
            // world's (ccPartyManager), whose panels the UI reads from the
            // next frame.
            R::AddMember(id) => {
                self.world.party_add(i32::from(id));
            }
            R::AreaLevel(v) => self.area_level = v,
            // CheckAreaCode's record: the words entered at the gate, which
            // the event task's next pass reads (areaCodeSet, cleared at its
            // top).
            R::AreaCodeSet(words) => {
                self.area_code_set = Some(words);
                if let Some(vm) = &mut self.vm {
                    vm.mng.area_code_set = words;
                }
            }
            // ccSpcChar::TransferOut: a party member warps out.
            R::TransferOut(h) => {
                if let Some((Kind::Spc, code)) = unhandle(h) {
                    self.world.transfer_out(code);
                }
            }
            // ccChangeCmndTarget and cmndTargetFix from the menus (the shop
            // pages drop the target and give it back).
            R::TargetClear => self.world.change_command_target(None),
            R::Target(h) => self.world.change_command_target(unhandle(h)),
            R::TargetFix(on) => self.world.set_target_fix(on),
            // The session changes the scene (`area.rs`, the fields).
            // ccGame::ChangeArea goes through ChangeRequest, which calls
            // ccDisableThEvent: the event task idles until the next set-up
            // enables it. Left running, its play pass would start the next
            // area's phase-4 blocks during the hold, before the set-up's
            // passes, and hold them (event 11's talk on the holy ground).
            R::GoToArea(words) => {
                let [a, b, c] = words;
                self.leaving = Some(format!("to the area of words {a} {b} {c}"));
                if let Some(vm) = &mut self.vm {
                    vm.disable();
                }
                self.events.push(Event::GoToArea(words));
            }
            R::ChangeArea { area, n } => {
                self.leaving = Some(format!("ChangeArea({area}, {n})"));
                if let Some(vm) = &mut self.vm {
                    vm.disable();
                }
                self.events.push(Event::Go(piney_data::area::Go::ChangeArea(area, n)));
            }
            // Log Out (4, 7) and OPTION's Title Screen (1, 7):
            // ccGame::ChangeRequest, for the session.
            R::ChangeMode { num, sf } => self.events.push(Event::ChangeMode { num, sf }),
            // ccSaveData::SetSoundEnv from OPTION's Sound page (the save
            // already holds them; the output mode is not modelled).
            R::SoundEnv { main, bgm, se, output } => self.events.push(Event::Volumes { main, se, bgm, output }),
            // ccParty::DelMember(slot): Remove and Disband.
            R::DelMember(slot) => self.world.party_remove(-slot),
            R::CameraType(t) => self.world.set_camera_type(t),
            // Switched on, the pad buzzes once.
            R::Vibration { on } => {
                if on {
                    self.events.push(Event::Actuate { small: true, power: 160, ms: 200 });
                }
            }
            // The shop's camera on the merchant spoken to (SetMerchantCamera:
            // changeCamera(3), ecam filled) and back (changeCamera(1)).
            R::Talk(t) => match t {
                TalkReq::MerchantCamera => {
                    if let Some((k, code)) = self.world.command_target() {
                        self.world.set_merchant_camera(k, code, self.st.game.server);
                    }
                }
                TalkReq::Camera(n) => self.world.change_camera(n as i16),
                // Give Food: EntryAffect(grunty, plw, 19, food, num, 0).
                TalkReq::Feed { target, food, num } => {
                    if let Some((Kind::Npc, code)) = unhandle(target) {
                        self.world.grunty_affect(code, 19, food as i16, num as i16);
                    }
                }
                // The menus' writes to the Grunty (growthNum, foodMode,
                // chatFlag).
                TalkReq::GruntyGrowth { target, growth } => {
                    if let Some(g) = unhandle(target).and_then(|(_, code)| self.world.grunty_mut(code)) {
                        g.growth_num = growth;
                    }
                }
                // AddSpcItem's ccUseItemRequest(spc, spc, code, 0): a book
                // given in a trade or as a present is read at once; what
                // lasts is the member's record.
                TalkReq::SpcUseItem { spc, code } => {
                    if let Some((Kind::Spc, id)) = unhandle(spc)
                        && code >> 16 == piney_battle::item::category::BOOK
                        && let Ok(i) = usize::try_from(id)
                    {
                        let volume = self.world.volume();
                        let save = &mut self.world.state_mut().save;
                        let mut p = piney_battle::param::SpcParam::from_save(save, i);
                        piney_battle::item::book_on_record(&mut p, code & 0xffff, volume);
                        p.store(save, i);
                    }
                }
                // ccSpcMessagePresentOtherFellow: the third member remarks.
                TalkReq::PresentOther(c) => {
                    if let Some((Kind::Spc, id)) = unhandle(c) {
                        self.world.present_other(id);
                    }
                }
                TalkReq::GruntyFood { target, food_mode, chat_flag } => {
                    if let Some(g) = unhandle(target).and_then(|(_, code)| self.world.grunty_mut(code)) {
                        g.food_mode = food_mode;
                        if let Some(c) = chat_flag {
                            g.chat_flag = c;
                        }
                    }
                }
                // The talk menus' other requests.
                #[allow(unreachable_patterns)]
                _ => {}
            },
            // WORLD_MAN::SetMapAlpha: the minimap's fade.
            R::MapAlpha(a) => self.map_st.set_alpha(a),
            R::Flash => self.ui.gate_flash(),
            // The frozen layers: asleep, the world draws everyone where
            // they stand. Skills and items belong to the battle; the
            // party's warps are not ported here.
            R::Still(_)
            | R::KeepLayers
            | R::DeleteNoPartyMember
            | R::Skill { .. }
            | R::UseItem { .. }
            | R::DrainAffect(_)
            | R::PlayerSp(_)
            | R::OpenChat(_)
            // Not carried out yet: the Grunty Flute's use, the way back
            // to a field (86) and Adjust Screen's offset. A member's
            // CalcReal runs every frame
            // (calc_real_party).
            | R::UseItemArg { .. }
            | R::SpcMessageOpenTreasureBox
            // No breakables in a town.
            | R::BreakSomething
            | R::AttackCancel
            | R::GoField
            | R::CalcReal { .. }
            // Data Drain is the fields' (area.rs).
            | R::DataDrain { .. }
            | R::DrainEnemy(_)
            | R::DrainSideEffect
            | R::DrainLevelDown
            | R::TargetsCleared
            | R::GameOver => {}
            // An item use's step on the town, as the menu task reaches it.
            R::ItemStep(st) => {
                if !self.world.item_step(&st) {
                    tracing::warn!("item step not carried out in town: {st:?}");
                }
            }
            R::WorldHidden(on) => self.hack_screen = on,
            // ccSPC::ChangeEquip: the new weapon on the model.
            R::ChangeEquip { target, cat, .. } => {
                if let Some((Kind::Spc, code)) = unhandle(target) {
                    self.world.change_equip(code, cat);
                }
            }
            // The CHAT menu's orders and the event menus' manual switches
            // on a party member (the battle's AI: ActInTown reads 9 and 12).
            R::ChatCmd { member, cmd } => {
                if let Some((Kind::Spc, code)) = unhandle(member) {
                    self.world.chat_cmd(code, cmd, None, 0);
                }
            }
            R::DisplayOffset { x, y } => self.events.push(Event::DisplayOffset { x, y }),
            R::ChangeEquipReport { member, n } => {
                if let Some((Kind::Spc, code)) = unhandle(member) {
                    self.world.equip_report(code, n);
                }
            }
            R::ChatOrder { member, cmd, target, skill } => {
                if let Some((Kind::Spc, code)) = unhandle(member) {
                    let t = match unhandle(target) {
                        Some((Kind::Spc, c)) => Some(c),
                        _ => None,
                    };
                    self.world.chat_cmd(code, cmd, t, skill);
                }
            }
            R::ManualOff(member) => {
                if let Some((Kind::Spc, code)) = unhandle(member) {
                    self.world.manual_off(code);
                }
            }
            R::ManualModeAi { member, ev } => {
                if let Some((Kind::Spc, code)) = unhandle(member) {
                    self.world.manual_mode_ai(code, ev);
                }
            }
            R::RemoteCmd { member, cmd } => {
                if let Some((Kind::Spc, code)) = unhandle(member) {
                    self.world.remote_cmd(code, cmd);
                }
            }
            // GtHackMenu's OK: setupMode 1 and gtHackFlag, for the next
            // scene's set-up (the session carries them).
            R::GateHacked => self.events.push(Event::GateHacked),
            // ccSndGateHack: the town's music out under the menu and
            // stopped, back if the hack is cancelled.
            R::GateHackSound(n) => self.events.push(Event::GateHackSound(n)),
        }
    }
}

/// `saveData.assignPAD` from +0x8404 as `ccSaveData::Init` (0x001743d0)
/// writes it: action cross, personal triangle, chat square, option start,
/// map select, ok cross, cancel circle, camera R1, R2, L1, L2.
const INIT_BUTTONS: [u16; 11] = [0x40, 0x10, 0x80, 0x800, 0x100, 0x40, 0x20, 0x08, 0x02, 0x04, 0x01];

/// Saves the port wrote before its new game set every button (only ok and
/// cancel, the rest 0) leave the field's menu buttons dead: no press
/// matches an assignment of 0, so triangle cannot open the Party
/// tutorial's menu. The game never writes such a save; it gets `Init`'s
/// assignment.
fn repair_buttons(state: &mut SaveState) {
    let at = |k: usize| piney_desktop::save::ASSIGN_PAD + 2 * k;
    let s = &mut state.save;
    let others_zero = (0..11).filter(|&k| k != 5 && k != 6).all(|k| s.i16(at(k)) == 0);
    if others_zero {
        for (k, b) in INIT_BUTTONS.iter().enumerate() {
            s.set_i16(at(k), *b as i16);
        }
    }
}

/// `ccCharBaseParam`'s and `ccSpcParam`'s fields from the record's start.
mod spc {
    pub const TYPE: usize = 0x08;
    pub const ID: usize = 0x0c;
    pub const HEIGHT: usize = 0x18;
    pub const WIDTH: usize = 0x1c;
    pub const MAX_HP: usize = 0x24;
    pub const MAX_SP: usize = 0x26;
}

/// A character's identity in the field UI (a pointer in the game).
pub(crate) fn handle(kind: Kind, code: i32) -> u32 {
    let k = match kind {
        Kind::Spc => 1,
        Kind::Npc => 2,
        Kind::Enemy => 3,
        Kind::Gimmick => 4,
    };
    (k << 24) | (code as u32 & 0x00ff_ffff)
}

/// The character a [`handle`] names.
fn unhandle(h: u32) -> Option<(Kind, i32)> {
    let kind = match h >> 24 {
        1 => Kind::Spc,
        2 => Kind::Npc,
        3 => Kind::Enemy,
        4 => Kind::Gimmick,
        _ => return None,
    };
    Some((kind, (h & 0x00ff_ffff) as i32))
}

/// A party member from its `saveData.spcParam` record, at full HP and SP
/// (the save keeps no current HP; a character is made at its maximum).
fn member(save: &SaveState, at: usize, name: Vec<u8>, handle: u32) -> piney_fieldui::CharInfo {
    let s = &save.save;
    let f = |o: usize| f32::from_bits(s.i32(at + o) as u32);
    piney_fieldui::CharInfo {
        handle,
        types: s.i32(at + spc::TYPE) as u32,
        id: s.i16(at + spc::ID),
        name,
        height: f(spc::HEIGHT),
        width: f(spc::WIDTH),
        hp: s.i16(at + spc::MAX_HP),
        sp: s.i16(at + spc::MAX_SP),
        max_hp: s.i16(at + spc::MAX_HP),
        max_sp: s.i16(at + spc::MAX_SP),
        attribute: -1,
        in_view: true,
        ..Default::default()
    }
}

/// A character of the town as the HUD's target window reads it.
fn char_info(world: &World, kind: Kind, code: i32) -> Option<piney_fieldui::CharInfo> {
    let c = world.char_info(kind, code)?;
    let h = handle(kind, code);
    let mut info = match c.spc_param {
        Some(at) => member(world.state(), at, c.name, h),
        None => piney_fieldui::CharInfo {
            handle: h,
            types: c.flags,
            id: code as i16,
            name: c.name,
            attribute: -1,
            in_view: true,
            ..Default::default()
        },
    };
    info.height = f32::from_bits(c.height);
    // The cursor over the character (ccCalcTagPosChar at 0.45 of its
    // height, mode 0: 1 on screen).
    info.tag = world.tag_pos(kind, code, 0.45, 0).filter(|t| t.2 != 0).map(|t| (t.0, t.1));
    if let Some(&(_, _, dist, _)) = world.command_sorted().iter().find(|s| (s.0, s.1) == (kind, code)) {
        info.cmnd_dist = f32::from_bits(dist);
    }
    Some(info)
}

/// The field UI's view of the town (`ccGame`, `ccPartyManager`,
/// `cmndTarget`, `cmndSortRoot`, and the event manager's `status` and
/// `areaCode[16]`): status 5, area 0, the party's slots from `spcParam`
/// (Kite's named by the save's `plName`, the others by `charTbl`).
pub(crate) fn ui_world(world: &World, vm: Option<&Vm>, area_level: i32) -> piney_fieldui::World {
    let save = world.state();
    let s = &save.save;
    let ids = world.party();
    let mut party: [Option<piney_fieldui::CharInfo>; 3] = [None, None, None];
    let mut party_id = [-1i32; 3];
    for (slot, &code) in ids.iter().enumerate() {
        if code < 0 {
            continue;
        }
        let at = piney_data::save::by_id::spc_param(code as usize);
        let name = if code == 0 {
            s.name().to_vec()
        } else {
            world.char_info(Kind::Spc, code).map(|c| c.name).unwrap_or_default()
        };
        let mut m = member(save, at, name, handle(Kind::Spc, code));
        // The Status menu reads the character's conditions: its CalcReal's
        // ConditionBattleEffect puts the equipment's added effects there.
        if let Some(ch) = world.town_party().char(code) {
            m.condition = ch.cond.v;
            m.speed_value = f32::from_bits(ch.cond.speed_value);
        }
        if let Some((pos, _)) = world.char_pos(2, code as i16) {
            m.pos_p = pos.map(f32::from_bits)[..3].try_into().unwrap_or_default();
        }
        party[slot] = Some(m);
        party_id[slot] = code;
    }
    let t = world.targeting();
    let info = |c: Option<(Kind, i32)>| c.and_then(|(k, n)| char_info(world, k, n));
    let sorted: Vec<_> = world.command_sorted().iter().filter_map(|s| char_info(world, s.0, s.1)).collect();
    piney_fieldui::World {
        game: piney_fieldui::Game {
            status: 5,
            area: 0,
            town: i32::from(s.u8(offset::LAST_TOWN) as i8),
            // ChangeScene's server by town (0x00306dc0): Mac Anu's is 0,
            // Dun Loireag's 1.
            server: piney_world::area::SERVER_OF_TOWN.get(usize::from(s.u8(offset::LAST_TOWN))).copied().unwrap_or(0),
            area_level,
            ..Default::default()
        },
        party,
        party_id,
        // The story areas the events bar at the gate (none without them).
        area_codes: vm.map_or([(-1, -1); 16], |vm| vm.mng.area_codes),
        event_status: vm.map_or(0, |vm| vm.mng.status),
        target: info(t.target),
        target_prev: info(t.prev),
        // The Grunty the menus read (cmndTargetPrev's, else cmndTarget's).
        grunty: [t.prev, t.target].into_iter().flatten().find_map(|(k, code)| {
            let g = world.grunty(code).filter(|_| k == Kind::Npc)?;
            Some(piney_fieldui::menus::breeder::Grunty {
                handle: handle(k, code),
                row: i32::from(g.row.id),
                growth: g.growth_num,
                msg: g.msg_num,
                exist: g.exist,
                level: g.grow.level,
                size: g.grow.size,
                food_num: g.grow.food_num,
                chat_flag: g.chat_flag,
                food_mode: g.food_mode,
            })
        }),
        obj_chain: sorted.iter().filter(|c| !c.is(0x07)).cloned().collect(),
        sorted,
        boss_entry: -1,
        // ccSpcManager.registry[registryNum]: PartyInMenu calls a member
        // only while fewer than five are loaded or when it is one of them
        // (CheckSpc), and greets by its bootParam.
        spc: {
            let r = world.spcs();
            r.registry[..r.registry_num.clamp(0, r.registry.len() as i32) as usize]
                .iter()
                .map(|e| (e.id, e.boot_param))
                .collect()
        },
        // g_entCtrl's NPC list (each entParam.id, its npcTbl row): who is
        // in town today, Ryu Book III's "Online".
        npcs: world.all_npcs().map(|n| n.code()).collect(),
        ..Default::default()
    }
}

/// What `ccThGameCtrl` reads of `ccMenu`.
fn menu_view(ui: &FieldUi) -> talk::MenuView {
    let c = &ui.ctrl;
    talk::MenuView {
        idle: ui.menu_type() == -1 && c.open_req & 0xfff != 74,
        forbid: c.forbid != 0,
        forbid_chat_except: c.forbid_chat_except != 0,
        pl_attack: c.pl_attack != 0,
    }
}

impl Mode for WorldMode {
    fn step(&mut self, pad: &Pad) -> Frame {
        self.frames += 1;
        self.count = self.count.wrapping_add(1);
        self.st.pad = *pad;
        self.st.frame = self.frames;
        // ccThEvent (32) first.
        self.event_frame();
        // A stream the scripts play runs inside the event task's call: the
        // town's tasks wait, and its frame is what shows.
        if let Some(f) = self.st.stream.frame.take() {
            self.events.append(&mut self.st.events);
            return f;
        }
        let mut ctx = Ctx::new(View::default());
        self.world.set_menu_view(menu_view(&self.ui));
        // ccThGameCtrl (33): the command target and the buttons; the map
        // button where its test came (past ghoFlag and menuClrWait).
        self.world.step_game_ctrl(pad);
        if self.world.map_test() {
            self.map_st.button(self.world.state_mut(), piney_world::area::kind::TOWN, pad.push.bits());
        }
        for t in self.world.take_talk() {
            self.talk(t);
        }
        // ccThMenu (34), before the town's other tasks: what it asks for is
        // acted on, and heard, this frame (worklog 336). A menu that asked
        // for another area (the gate's SetGenerateCode, ChangeArea) sleeps
        // in ChangeRequest: it does not run through the leave's fade, where
        // the gate counted the area once a frame (#22).
        if let Phase::Play(f) = self.world.phase()
            && f >= 1
            && self.setup != Setup::Left
            && self.leaving.is_none()
        {
            self.menu_task(pad, &mut ctx);
        }
        // A Ryu Book's cover stream (ccThBook's), stepped by the menu task:
        // its frame shows now, as the scripts' streams' do. Kept for the
        // next frame's check, it showed every other frame, with the town's
        // frame between, and half the presses never reached it (#21).
        if let Some(f) = self.st.stream.frame.take() {
            self.events.append(&mut self.st.events);
            return f;
        }
        // ccThCamera (40) on, and the town's own sprites (ccEff::Draw
        // inside its Draw); with the gate hack's screen, only that, over
        // black.
        let mut hidden = Ctx::new(View::default());
        let wctx = if self.hack_screen { &mut hidden } else { &mut ctx };
        self.world.step_into(pad, wctx);
        let sprites = self.world.take_town_sprites();
        if let Some(fx) = &mut self.town_fx {
            fx.draw(&sprites, self.world.camera(), wctx);
        }
        if matches!(self.world.phase(), Phase::Play(f) if f >= 1) && !self.world.asleep() {
            self.calc_real_party();
        }
        let mut pc_starts = Vec::new();
        // ccThMenu breathes until party slot 0 is filled: from the tasks'
        // first frame on.
        if let Phase::Play(f) = self.world.phase()
            && f >= 2
            && self.setup != Setup::Left
        {
            // The walking PCs' chat lines this frame (ccRtownPC's OpenChat)
            // and their effTransfers.
            let names = self.world.state().names();
            for (code, e) in self.world.take_pc_events() {
                match e {
                    piney_world::rtownpc::PcEvent::Chat(text) => {
                        let text = piney_data::tables::sjis::encode(text);
                        self.ui.open_chat(handle(Kind::Npc, code), &text, &names);
                    }
                    piney_world::rtownpc::PcEvent::Transfer => {
                        pc_starts.push(crate::town_fx::Start::Transfer(2 << 24 | (code as u32 & 0xff_ffff)));
                    }
                    piney_world::rtownpc::PcEvent::Step { param, ccs_type, pos, attribute, feet, dirc_z, speed } => {
                        if let Some(se) = piney_audio::se3d::pc_note(param, ccs_type, attribute) {
                            let cam = self.world.camera().active();
                            let ear =
                                Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
                            self.events.push(Event::Se3d { n: se.code, pos, note: se.note, ear });
                        }
                        if let Some(feet) = feet {
                            pc_starts.push(crate::town_fx::Start::PawSmoke(feet, dirc_z, speed));
                        }
                    }
                }
            }
            // Dun Loireag's dogs: inuCheckNote's ccSeSetParamInu and
            // ccDog::effect's effSmoke.
            for (_, e) in self.world.take_dog_events() {
                match e {
                    piney_world::dog::DogEvent::Note { param, pos, attribute } => {
                        if let Some(se) = piney_audio::se3d::inu_note(self.world.volume(), param, attribute) {
                            let cam = self.world.camera().active();
                            let ear =
                                Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
                            self.events.push(Event::Se3d { n: se.code, pos, note: se.note, ear });
                        }
                    }
                    piney_world::dog::DogEvent::Smoke { pos, v } => {
                        pc_starts.push(crate::town_fx::Start::Smoke(pos, v));
                    }
                }
            }
            // Dun Loireag's Grunties: inuCheckNote's ccSeSetParamInu, their
            // ccSeOn3Ds, smoke and growing up's effects, voices and lines.
            self.grunty_events(&names, &mut pc_starts);
            // Carmina Gade's airship: its chimneys' effSmokeN and ccSeOn3D.
            for e in self.world.take_town_events() {
                match e {
                    piney_world::town::TownEvent::SmokeN { pos, v, scale, life, kind, fade } => {
                        pc_starts.push(crate::town_fx::Start::PuffN { pos, v, scale, life, kind, fade });
                    }
                    piney_world::town::TownEvent::Se3d { n, pos } => {
                        if let Ok(n) = usize::try_from(n) {
                            let cam = self.world.camera().active();
                            let ear =
                                Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
                            self.events.push(Event::Se3d { n, pos, note: None, ear });
                        }
                    }
                }
            }
            // ccSoundMain's scene sounds: Mac Anu's canals, the breeder's
            // tune by DMY_merchant6.
            let town = &self.world.town().base;
            let breeder = town.file.ccs.find_object("DMY_merchant6").and_then(|o| town.file.scene.dummies.get(&o));
            let scene_sound = piney_audio::scene::SceneInput {
                camera: Some(self.world.camera().active().pos),
                kite: self.world.player().body.pos,
                block: 0,
                mac_anu: town.no == 0,
                breeder: breeder.map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), 0x3f80_0000]),
            };
            self.events.push(Event::SceneSound(scene_sound));
            // The party members' lines (ccAI::ChatMessageSender's OpenChat).
            for (id, text) in self.world.take_party_chats() {
                self.ui.open_chat(handle(Kind::Spc, id), &text, &names);
            }
            // ccThFieldDisp's ROOTTOWN01::Draw -> DrawMap, after ccThMenu has
            // set this frame's alpha.
            if let Some(map) = self.map.as_mut().filter(|_| !self.hack_screen) {
                let p = self.world.player();
                let (pos, dirc) = (p.body.pos, p.body.dirc);
                self.map_st.hud_scale = self.ui.hud_scale;
                piney_world::map::town_frame(map, &mut self.map_st, pos, dirc, !self.world.asleep(), &mut ctx);
            }
        }
        // ccThEffect (80) and ccThParticle (98): Kite's, the members' and
        // the walking PCs' transfers started, the effects stepped (not
        // while the tasks sleep) and drawn.
        if let Some(fx) = &mut self.town_fx {
            use crate::town_fx::Start;
            let mut starts: Vec<Start> = Vec::new();
            for warp in self.world.take_kite_transfers() {
                starts.push(if warp { Start::WarpTransfer(0) } else { Start::Transfer(0) });
            }
            for (id, warp) in self.world.take_member_transfers() {
                let c = 1 << 24 | (id as u32 & 0xff_ffff);
                starts.push(if warp { Start::WarpTransfer(c) } else { Start::Transfer(c) });
            }
            // The Administrator (ccMerchan::sysopeAct).
            for (id, e) in self.world.take_merchant_events() {
                use piney_world::merchant::SysopEvent;
                let c = 3 << 24 | (id as u32 & 0xff_ffff);
                match e {
                    SysopEvent::Transfer => starts.push(Start::Transfer(c)),
                    SysopEvent::Vanish => {
                        self.events.push(Event::Se(217));
                        starts.push(Start::Vanish(c));
                    }
                    SysopEvent::Noise => self.ui.noise(2),
                }
            }
            starts.extend(pc_starts);
            // Kite's running steps' dust (CheckNote's ccEffPawSmoke).
            for (feet, dz, speed) in self.world.take_paw_smokes() {
                starts.push(Start::PawSmoke(feet, dz, speed));
            }
            let camera = self.world.camera().clone();
            // Kite's footsteps (ccPlayer::CheckNote's ccSeSetParamSPC, by
            // the ground under him), as the fields play them.
            let steps = self.world.take_steps();
            if !steps.is_empty() {
                let cam = camera.active();
                let ear = Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
                for (param, pos, attribute) in steps {
                    if let Some(se) = piney_audio::se3d::spc_note(self.world.volume(), param, 0, attribute) {
                        self.events.push(Event::Se3d { n: se.code, pos, note: se.note, ear });
                    }
                }
            }
            // The Chaos Gate's circle opening (71) and closing (72).
            let gate = self.world.take_gate_sounds();
            if !gate.is_empty() {
                let cam = camera.active();
                let ear = Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
                for (n, pos) in gate {
                    if let Ok(n) = usize::try_from(n) {
                        self.events.push(Event::Se3d { n, pos, note: None, ear });
                    }
                }
            }
            if !self.world.asleep() {
                let chars = self.world.fx_chars();
                let player = self.world.player().body.pos;
                let sounds = fx.frame(&starts, &chars, player, &camera, self.world.rand_state());
                let cam = camera.active();
                let ear = Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
                for (n, pos) in sounds {
                    if let Ok(n) = usize::try_from(n) {
                        self.events.push(Event::Se3d { n, pos, note: None, ear });
                    }
                }
            }
            if !self.hack_screen {
                fx.draw_effects(&camera, &mut ctx);
            }
        }
        // ccMenu's fader, sent last by Disp (so drawn first on its layer).
        if self.setup != Setup::Left {
            self.st.fade.send(&mut ctx, MENU_LAYER);
            // scFadeDef: the screen's own flashes.
            self.st.fade_def.send(&mut ctx, piney_desktop::layers::FONT_LAYER);
        }
        let mut frame = ctx.finish();
        // ccSys.bgColor, which the town's constructor set.
        if let Some([r, g, b]) = self.world.clear_colour() {
            frame.clear = piney_draw::Rgba([r, g, b, 0x80]);
        }
        frame
    }

    fn take_events(&mut self) -> Vec<Event> {
        let save = &self.world.state().save;
        let town = save.u8(offset::LAST_TOWN);
        let crisis = save.u8(offset::CRISIS) as i8 != 0;
        let crisis_one = save.u8(offset::CRISIS) == 1;
        let dt_bgm = i32::from(save.u8(offset::DT_BGM) as i8);
        let mut out: Vec<Event> = self
            .world
            .take_requests()
            .into_iter()
            .filter_map(|r| match r {
                Request::SoundFadeOut => Some(Event::SoundFadeOut),
                Request::AllSoundOff => Some(Event::AllSoundOff),
                // `ccSndSQLoad(2)`: `sqDataTown[townType]`, five rows on
                // during the crisis.
                Request::SqLoad(_) => {
                    Some(Event::SqLoad(piney_audio::SqContext::Town { row: town + if crisis { 5 } else { 0 } }))
                }
                // `ccSndBgmCtrl`'s town case (0x0017b064, `bgm_plan`): Mac
                // Anu in the crisis and Dun Loireag with a third sequence
                // start sequence 2, then every town sequence 0; Mac Anu's
                // +0x105 and canals' flag reset.
                Request::BgmCtrl => Some(Event::BgmCtrl(piney_audio::BgmWorld {
                    scene_replaced: false,
                    town: i32::from(town),
                    crisis: crisis_one,
                    dt_bgm,
                })),
                Request::GameStart => Some(Event::GameStart),
                // Soft reset is not modelled; the transfers went to the
                // effects in the frame.
                Request::EnableReset(_) | Request::Transfer | Request::WarpTransfer => None,
            })
            .collect();
        out.append(&mut self.events);
        out
    }

    fn frame_rate(&self) -> u32 {
        self.st.frame_rate.unwrap_or_else(|| self.world.frame_rate())
    }

    /// While the scripts play a stream, its archive (the stream's pictures).
    fn archive(&self) -> Option<Arc<Archive>> {
        self.st.stream.player.as_ref().map(crate::stream::StreamPlayer::archive)
    }

    fn title(&self) -> String {
        let phase = match self.world.phase() {
            Phase::FadeOut(n) => format!("fade out {n}"),
            Phase::Hold(n) => format!("loading {n}"),
            Phase::Play(n) => format!("play {n}"),
        };
        // Positions and speeds are EE float bits.
        let pl = self.world.player();
        let p = pl.body.pos.map(f32::from_bits);
        let target = match self.world.command_target() {
            Some((k, n)) => format!(" - target {k:?} {n}"),
            None => String::new(),
        };
        let leaving = match &self.leaving {
            Some(to) => format!(" - the gate asks to go {to}"),
            None => String::new(),
        };
        let menu = match (self.ui.menu_type(), self.unported) {
            (-1, Some(n)) => format!(" - menu {n} not ported"),
            (-1, None) => String::new(),
            (m, _) => format!(" - menu {m}"),
        };
        let events = match (&self.vm, self.scene_change()) {
            (_, Some(c)) => {
                let [a, t, f, d, fl, b] = c.scene;
                let area = c.area.map_or(String::new(), |(n, w)| format!(" (area {n}, words {w:?})"));
                format!(" - ChangeScene({a}, {t}, {f}, {d}, {fl}, {b}){area}: the next area is not ported")
            }
            (Some(vm), None) => match (self.setup(), vm.playing()) {
                (Setup::Pass(p), _) => format!(" - events: the pass at phase {p}"),
                (_, Some((e, b))) => format!(" - event {e} block {b}"),
                _ => format!(" - events phase {}", vm.phase()),
            },
            (None, None) => String::new(),
        };
        let fade = self.st.fade.alpha().map_or(String::new(), |a| format!(" - fade {a}"));
        let town = match self.world.town().base.no {
            0 => "Mac Anu",
            1 => "Dun Loireag",
            2 => "Carmina Gadelica",
            3 => "Fort Ouph",
            4 => "Lia Fail",
            _ => "a Root Town",
        };
        format!(
            "The World - {town} - {phase} - Kite at ({:.0}, {:.0}, {:.0}) act {} speed {:.1}{target}{menu}{leaving}{events}{fade}",
            p[0],
            p[1],
            p[2],
            pl.acts.act,
            f32::from_bits(pl.body.speed)
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use piney_input::{Buttons, Raw};

    use super::*;

    /// A player for event 2's windows and menus: CROSS to type out and
    /// close each line; in the tutorials TRIANGLE for PERSONAL and the
    /// cursor to the one row that goes on (Party, Add, New Keyword), CROSS
    /// for the rest.
    fn player(mode: &WorldMode, f: u64) -> Buttons {
        let ui = mode.ui();
        let c = &ui.ctrl;
        let go_to = |row: i16| match c.list().select {
            s if s == row => Buttons::CROSS,
            s if s < row => Buttons::DOWN,
            _ => Buttons::UP,
        };
        match (ui.menu_type(), c.proccess) {
            (_, _) if !f.is_multiple_of(8) => return Buttons::NONE,
            (75, 1) => return Buttons::TRIANGLE,
            (75, 2) => return go_to(6),
            (76, 2) => return go_to(0),
            (78, 4) => return go_to(1),
            (77..=79, _) => return Buttons::CROSS,
            (-1, _) => {}
            _ => return Buttons::NONE,
        }
        // A window waiting: the last window call was an open.
        let waiting = mode.calls().iter().rev().find_map(|(_, c)| {
            if c.starts_with("message_open") || c.starts_with("announce") {
                Some(true)
            } else if c.starts_with("message_check") {
                Some(false)
            } else {
                None
            }
        });
        if waiting == Some(true) && f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE }
    }

    /// Event 2 from a new game's arrival in Mac Anu to its scene change,
    /// headless, the pad scripted by [`player`]: the mode at the end, the
    /// frames, the sound asked for, and the presses (frame, buttons).
    #[allow(clippy::type_complexity)]
    fn play_event_2() -> Option<(WorldMode, u64, Vec<Event>, Vec<(u64, Buttons)>)> {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return None;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = new_game_state(&mut disc).unwrap();
        let vm = new_game_events(&mut disc, &mut state).unwrap();
        let mut mode = WorldMode::enter(&iso, archive, state, Some(vm)).unwrap();
        let mut pad = Pad::default();
        let (mut f, mut events, mut presses) = (0u64, Vec::new(), Vec::new());
        while mode.scene_change().is_none() {
            f += 1;
            let buttons = player(&mode, f);
            if buttons != Buttons::NONE {
                presses.push((f, buttons));
            }
            pad.read(&Raw { buttons, ..Raw::default() });
            mode.step(&pad);
            events.extend(mode.take_events());
            assert!(f < 20_000, "event 2 does not end: {}", mode.title());
        }
        Some((mode, f, events, presses))
    }

    /// Prints the calls and the presses as `--press` wants them (its
    /// frames count from 0): `cargo test -p piney-game event_2_log --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn event_2_log() {
        let Some((mode, f, events, presses)) = play_event_2() else { return };
        for (fr, c) in mode.calls() {
            println!("{fr:5} {c}");
        }
        let name = |b: Buttons| match b {
            Buttons::CROSS => "cross",
            Buttons::TRIANGLE => "triangle",
            Buttons::DOWN => "down",
            Buttons::UP => "up",
            _ => "?",
        };
        let list: Vec<String> = presses.iter().map(|&(f, b)| format!("{}:{}", f - 1, name(b))).collect();
        println!("--press {}", list.join(","));
        println!("ended at {f}: {}", mode.title());
        let voices: Vec<i32> = events
            .iter()
            .filter_map(|e| match e {
                Event::Voice { event: 2, msg } => Some(*msg),
                _ => None,
            })
            .collect();
        println!("voices {voices:?}");
        println!("party {:?}", mode.world().party());
    }

    /// `--mode world`'s save: the boot's `ccSaveData::Init`, then
    /// `NewGame(0)`: Kite's stats from `charTbl`, the bag and the skill
    /// lists empty (-1), Init's buttons, the time idols' text.
    /// Mac Anu's arrival asks the sound for what `ccSetupGameCtrl` does:
    /// the town bank, `gameStart` before the fade in, then `ccSndBgmCtrl`'s
    /// town case (Mac Anu, no crisis); and every frame after, the scene
    /// sounds' inputs (the canals).
    #[test]
    fn mac_anu_starts_its_sound() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let state = new_game_state(&mut disc).unwrap();
        let mut mode = WorldMode::enter(&iso, archive, state, None).unwrap();
        let pad = Pad::default();
        let mut events = Vec::new();
        for _ in 0..40 {
            mode.step(&pad);
            events.extend(mode.take_events());
        }
        let at = |pred: &dyn Fn(&Event) -> bool| events.iter().position(pred);
        let start = at(&|e| matches!(e, Event::GameStart)).expect("gameStart");
        let bgm = at(&|e| matches!(e, Event::BgmCtrl(w) if w.town == 0 && !w.crisis)).expect("the town's music");
        assert!(start < bgm, "gameStart comes before the fade in ends");
        assert!(events.iter().any(|e| matches!(e, Event::SceneSound(s) if s.mac_anu && s.camera.is_some())));
    }

    /// The Equipment menu's new blades in the town: `ChangeEquip`'s
    /// `ChangeWeapon` hangs the save's weapon file on Kite's hands.
    #[test]
    fn new_blades_in_his_hands() {
        use piney_battle::param::SpcParam;
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let state = new_game_state(&mut disc).unwrap();
        let mut mode = WorldMode::enter(&iso, archive, state, None).unwrap();
        let t = piney_battle::tables::Tables::read(&mut disc).unwrap();
        let (now, _) = piney_world::body::weapon_of(&t, &mode.world.state().save, 0).expect("Kite's blades");
        let (i, other) = (0..64)
            .find_map(|i| {
                let n = String::from_utf8(t.job_weapon(0, i)?.ccsname.clone()).ok()?;
                (!n.is_empty() && n != now).then_some((i, n))
            })
            .expect("another blade");
        let mut k = SpcParam::from_save(&mode.world.state().save, 0);
        k.equipment[4] = i as i16;
        k.store(&mut mode.world.state_mut().save, 0);
        mode.world.change_equip(0, 0);
        let kite = mode.world.kite();
        let w = &kite.weapon.as_ref().expect("the blades' file").ccs;
        let hung: Vec<&str> = kite.hands().iter().filter_map(|&(_, m)| w.object_name(m)).collect();
        assert_eq!(hung, [format!("MDL_{other}r"), format!("MDL_{other}l")]);
    }

    /// Issue #1: a save loaded into the town with a critical-hit blade on
    /// Kite. The Status menu's added effects are his character's conditions
    /// 2-6, which the town's CalcReal sets from the equipment each frame.
    #[test]
    fn a_loaded_town_shows_the_equipment_s_added_effects() {
        use piney_battle::param::{SpcParam, cond};
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = new_game_state(&mut disc).unwrap();
        let t = piney_battle::tables::Tables::read(&mut disc).unwrap();
        let (i, crit) = (0..64)
            .find_map(|i| Some((i, t.job_weapon(0, i)?.beff[2])).filter(|&(_, c)| c > 0))
            .expect("a blade with a critical hit");
        let mut k = SpcParam::from_save(&state.save, 0);
        k.equipment[4] = i as i16;
        k.store(&mut state.save, 0);
        let mut mode = WorldMode::enter(&iso, archive, state, None).unwrap();
        for _ in 0..40 {
            mode.step(&Pad::default());
        }
        let w = ui_world(&mode.world, None, 0);
        let kite = w.party[0].as_ref().expect("Kite in slot 0");
        assert_eq!(kite.condition[cond::CRITICAL], crit);
    }

    #[test]
    fn a_new_game_s_save_is_init_then_new_game() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let s = new_game_state(&mut Iso::open(&iso).unwrap()).unwrap().save;
        assert_eq!(s.name(), b"Kite");
        let kite = offset::SPC_PARAM;
        assert_eq!((s.i16(kite + 0x24), s.i16(kite + 0x26)), (63, 13));
        assert!((0..18 * 40).all(|i| s.i16(offset::ITEM_LIST + 4 * i) == -1));
        assert!((0..18 * 20).all(|i| s.i16(offset::SKILL_LIST + 2 * i) == -1));
        assert_eq!((s.i16(offset::ASSIGN_PAD_ACTION + 2), s.i16(offset::ASSIGN_PAD_OK)), (0x10, 0x40));
        assert_ne!(s.u8(offset::TIME_IDOL_RANK_STR), 0);
    }

    /// The event task as a new game brings it to Log in: event 1 done, 2
    /// not begun, the task disabled.
    #[test]
    fn a_new_game_reaches_the_world_with_event_1_done() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let mut state = new_game_state(&mut disc).unwrap();
        let vm = new_game_events(&mut disc, &mut state).unwrap();
        let s = &state.save;
        assert_ne!(s.flags(0) & piney_event::state::DONE, 0, "event 0 done at boot");
        assert_ne!(s.flags(1) & piney_event::state::DONE, 0, "event 1 done");
        assert_eq!(s.flags(2), 0, "event 2 not begun");
        assert!([4, 5].iter().all(|&m| s.mail(m) >= 4), "mails 4 and 5 read");
        assert_eq!(vm.operate(), 0, "event 1 released the operations");
        assert_eq!(vm.phase(), -1, "Log in disabled the task");
    }

    /// Event 2 plays in Mac Anu from the arrival to its `scene`: every call the
    /// field host takes, on the frame the event task makes it. The frames follow
    /// from the set-up (fade 1-10, hold 11-12, the pass at 0 in 13, at 2 in 15, F0
    /// 16, play from 17) and the instructions' own counts (docs/engine/event-vm.md);
    /// the player presses CROSS every 24 frames while a window waits and walks the
    /// tutorial menus.
    #[test]
    fn event_2_plays_to_the_scene_change() {
        let Some((mode, f, events, _)) = play_event_2() else { return };
        let got: Vec<(u64, String)> = mode
            .calls()
            .iter()
            .map(|(f, c)| {
                let c = c.strip_suffix(" (not ported)").or(c.strip_suffix(" (not done)")).unwrap_or(c);
                (*f, c.to_string())
            })
            .collect();
        let want: Vec<(u64, &str)> = vec![
            (12, "clear_gate_hack"),
            (13, "pc Mode { pc: -3, param: 6 }"),
            (17, "menu_ban true"),
            (17, "pc Face { pc: 2, ty: 2, code: 0, chg: 0 }"),
            (17, "camera ZSet { v: [-2, 574, 102], c: [-3, 537, 68] }"),
            (19, "camera ZSpeed { vstype: 2, cstype: 2 }"),
            (19, "camera ZMove { v: [0, 560, 70], c: [8, 521, 74], vprate: 120, cprate: 120 }"),
            (100, "pc Act { pc: 0, act: 3 }"),
            (221, "camera Char { ty: 2, code: 2, height: 10, rotx: 1024, roty: -30720, dist: 40 }"),
            (221, "message_open 2 0 Speech"),
            (266, "message_check 1"),
            (277, "pc Face { pc: 0, ty: 2, code: 2, chg: 64 }"),
            (277, "camera ZSpeed { vstype: 2, cstype: 2 }"),
            (277, "camera ZMove { v: [-12, 584, 69], c: [0, 536, 77], vprate: 20, cprate: 20 }"),
            (318, "message_open 2 1 Speech"),
            (338, "message_check 1"),
            (349, "message_open 2 2 Speech"),
            (410, "message_check 1"),
            (421, "message_open 2 3 Speech"),
            (482, "message_check 1"),
            (493, "message_open 2 4 Speech"),
            (554, "message_check 1"),
            (555, "message_open 2 5 Speech"),
            (578, "message_check 1"),
            (589, "message_open 2 6 Speech"),
            (626, "message_check 1"),
            (637, "camera ZSet { v: [-33, 635, 71], c: [-29, 585, 72] }"),
            (637, "message_open 2 7 Speech"),
            (674, "message_check 1"),
            (685, "sound_effect 74"),
            (685, "announce Member { pc: 2 }"),
            (722, "message_check 1"),
            (722, "message_close"),
            (732, "message_open 2 8 Speech"),
            (794, "message_check 1"),
            (795, "message_open 2 9 Speech"),
            (842, "message_check 1"),
            (843, "message_open 2 10 Speech"),
            (890, "message_check 1"),
            (891, "message_open 2 11 Speech"),
            (938, "message_check 1"),
            (949, "camera ZSet { v: [-11, 586, 72], c: [1, 534, 68] }"),
            (949, "open_menu 75"),
            (1268, "menu_ban true"),
            (1268, "message_open 2 21 Speech"),
            (1322, "message_check 1"),
            (1323, "message_open 2 22 Speech"),
            (1370, "message_check 1"),
            (1371, "message_open 2 23 Speech"),
            (1418, "message_check 1"),
            (1419, "message_open 2 24 Speech"),
            (1466, "message_check 1"),
            (1467, "message_open 2 25 Speech"),
            (1514, "message_check 1"),
            (1515, "message_open 2 26 Speech"),
            (1562, "message_check 1"),
            (1563, "message_open 2 27 Speech"),
            (1610, "message_check 1"),
            (1611, "message_open 2 28 Speech"),
            (1658, "message_check 1"),
            (1680, "message_open 2 29 Speech"),
            (1730, "message_check 1"),
            (1741, "fade 10 128 more=false"),
            (1752, "pc Turn { pc: 0, dirc: -32768, chg: 0 }"),
            (1752, "pc Turn { pc: 2, dirc: 28672, chg: 0 }"),
            (1752, "camera ZSet { v: [-7, 577, 69], c: [-13, 527, 64] }"),
            (1752, "fade 15 0 more=true"),
            (1783, "open_menu 78"),
            (2295, "area 14 [Some(0), Some(13), Some(26)]"),
            (2295, "change_scene 1 0 14 -1 -1 -1"),
        ];
        let want: Vec<(u64, String)> = want.into_iter().map(|(f, c)| (f, c.to_string())).collect();
        assert_eq!(got, want);
        assert_eq!(f, 2295);
        // Where it leaves: area 14 from its words, ChangeScene(1, 0, 14).
        // The event task is never put to sleep there: block 2 runs on to
        // its end_event in the same frame (worklog 330).
        let c = mode.scene_change().unwrap();
        assert_eq!(c.scene, [1, 0, 14, -1, -1, -1]);
        assert_eq!(c.area, Some((14, [Some(0), Some(13), Some(26)])));
        assert!(mode.title().contains("ChangeScene(1, 0, 14, -1, -1, -1)"), "{}", mode.title());
        let vm = mode.vm().unwrap();
        assert_eq!(vm.playing(), None);
        assert_eq!(vm.phase(), -1);
        let s = &mode.world().state().save;
        assert_eq!(s.flags(2), piney_event::state::CLOSED | 0b111, "blocks 0-2 run, event 2 closed");
        // gate_add / gate_mark 14: Bursting Passed Over Aqua Field on server 0,
        // its three words; member_add_msg 2: Orca's address.
        let bit = |at: usize, n: usize| s.u8(at + n / 8) >> (n % 8) & 1;
        assert_eq!(bit(offset::GATE_LIST, 14), 1);
        assert_eq!(bit(offset::GATE_LIST_MARK, 14), 1);
        assert!([0, 13, 26].iter().all(|&w| bit(offset::WORD_LIST, w) == 1));
        assert_ne!(s.member_word(offset::PARTY_MEMBER_FLAG) & 4, 0);
        assert_ne!(s.member_word(offset::PARTY_MEMBER_EXP) & 4, 0);
        // Orca at marker 3 and, after the tutorial, in party slot 1.
        assert!(mode.world().town_party().member(2).is_some());
        assert_eq!(mode.world().party(), [0, 2, -1]);
        // Each line's voice, as its window opens: messages 0-11; the party
        // tutorial's 12-19 (menus 75-77); 21-29; the gate tutorial's 30-33
        // (menu 78) and 35-56 (79).
        let voices: Vec<i32> = events
            .iter()
            .filter_map(|e| match e {
                Event::Voice { event: 2, msg } => Some(*msg),
                _ => None,
            })
            .collect();
        let want: Vec<i32> = (0..20).chain(21..34).chain(35..57).collect();
        assert_eq!(voices, want);
        assert!(events.contains(&Event::Se(74)), "member_add_msg's sound");
    }

    /// `item_add` for a companion in the town (case 93): the character the
    /// town built for the registry id takes it through `AddSpcItem` (a
    /// weapon of another job's category goes into its bag); none built, the
    /// host declines and the interpreter puts it in the save.
    #[test]
    fn item_add_for_a_companion_goes_through_the_menu() {
        use piney_battle::item::get_item_num;
        use piney_event::host::Host;
        let Some((mut mode, _, _, _)) = play_event_2() else { return };
        let job = mode.world.state().save.i16(offset::SPC_PARAM + offset::SPC_PARAM_SIZE * 2 + 0xd8);
        let cat = i32::from((job + 1).rem_euclid(6));
        let before = mode.world.state().save.clone();
        let mut h = FieldHost { world: &mut mode.world, ui: &mut mode.ui, st: &mut mode.st };
        assert!(h.add_spc_item(2, cat as i16, 3, 2), "Orca is in the town");
        assert!(!h.add_spc_item(8, cat as i16, 3, 1), "Piros is not");
        let s = &mode.world.state().save;
        assert_eq!(get_item_num(s, 2, cat, 3), get_item_num(&before, 2, cat, 3) + 2);
        assert_eq!(get_item_num(s, 0, cat, 3), get_item_num(&before, 0, cat, 3));
        assert_eq!(get_item_num(s, 8, cat, 3), get_item_num(&before, 8, cat, 3));
    }

    #[test]
    fn an_old_port_save_gets_its_buttons() {
        let at = |k: usize| piney_desktop::save::ASSIGN_PAD + 2 * k;
        let fresh = SaveState::fresh();
        let mut old = fresh.clone();
        for k in (0..11).filter(|&k| k != 5 && k != 6) {
            old.save.set_i16(at(k), 0);
        }
        repair_buttons(&mut old);
        assert_eq!(old.save.bytes()[..], fresh.save.bytes()[..]);
        // A save with its own buttons keeps them.
        let mut own = fresh.clone();
        own.save.set_i16(at(1), 0x80);
        repair_buttons(&mut own);
        assert_eq!(own.save.i16(at(1)), 0x80);
    }

    #[test]
    fn handles_are_distinct_and_never_zero() {
        let hs = [handle(Kind::Spc, 0), handle(Kind::Npc, 0), handle(Kind::Npc, 29), handle(Kind::Gimmick, 16)];
        assert!(hs.iter().all(|&h| h != 0));
        assert_eq!(unhandle(handle(Kind::Gimmick, 16)), Some((Kind::Gimmick, 16)));
        assert_eq!(unhandle(handle(Kind::Npc, 62)), Some((Kind::Npc, 62)));
        assert_eq!(unhandle(0), None);
        for (i, a) in hs.iter().enumerate() {
            assert!(hs[i + 1..].iter().all(|b| a != b));
        }
    }

    #[test]
    fn kite_from_the_new_game_save() {
        let save = SaveState::fresh();
        let k = member(&save, offset::SPC_PARAM, b"Kite".to_vec(), 1);
        assert_eq!((k.hp, k.max_hp, k.sp, k.max_sp), (k.max_hp, k.max_hp, k.max_sp, k.max_sp));
        assert_eq!(k.name, b"Kite");
    }
}
