//! The event scripts outside the towns: `piney-event`'s interpreter hosted
//! by a field or dungeon (`piney_world::field_world`), as the town's
//! [`crate::field_host`] hosts it in Mac Anu, with the same state
//! ([`FieldState`]: `ccGame`, the fader, the call log).
//!
//! ```text
//! message, info, member_add_msg   the field UI's windows, as in the town
//! menu, menu_ban, target_forbid,  ccMenu (the field UI); menu_ban's camera
//! map_on                          and party parts the area's
//! show_map                        WORLD_MAN::ShowMap (piney_world::map)
//! camera, cam*, camz*, teach_*    the area's event camera (evcam.rs)
//! pc_act, pc_mode, pc_turn,       piney-battle's EvParty over Kite and
//! pc_face, pc_command, the walks  the characters built in the area
//! (pc_walk_*), the puts
//! npc_*                           the area's NPCs (field_npcs)
//! entry                           the VM's entries, set up by the area
//!                                 mode (ccEntryEventMng, EntryGimmick)
//! enemy_put, hold, battle_ready   the fights' event instructions
//! fade, fade_more                 ccMenu's ccScFade
//! scene, area                     ccGame::ChangeScene: the session makes
//!                                 the change; the task sleeps in it
//! room, room_point                WORLD_MAN::RoomSelect: a change of scene
//!                                 to the room (FieldWorld::room_select)
//! item_add (a companion's)        ccMenu->AddSpcItem on the member built
//!                                 here (FieldUi::add_spc_item)
//! save_party                      the registry's party (Host::party)
//! ```

use std::path::PathBuf;

use piney_data::save::SaveData;
use piney_draw::Frame;
use piney_event::host::{
    Announce, CameraCommand, CharRef, EntryList, Game, GimmickCommand, Host, MessageCall, MessageKind, NpcCommand,
    Party, PcCommand, StoryArea, Wait,
};
use piney_fieldui::FieldUi;
use piney_world::area::Scene;
use piney_world::field_world::FieldWorld;

use crate::field_host::{CcGame, FieldState, STATUS_WORLD, SceneChange};
use crate::mode::Event;
use crate::stream::StreamPlayer;

/// A cutscene the scripts' `stream` plays in the area (`ccEventStream(num,
/// 1)`: `ccRequestLoadStream` plays it inside the call, the set-up or the
/// event task held meanwhile), and the frame it showed this frame.
#[derive(Default)]
pub struct AreaStream {
    pub iso: PathBuf,
    /// DATA.BIN, for the streams' subtitles.
    pub data: Option<std::sync::Arc<piney_data::archive::Archive>>,
    pub player: Option<StreamPlayer>,
    pub frame: Option<Frame>,
}

/// `ccGame` as the area's scene holds it.
pub fn game_of(scene: &Scene) -> CcGame {
    CcGame {
        status: STATUS_WORLD,
        area: scene.area,
        area_prev: scene.area_prev,
        server: scene.server,
        town: scene.town,
        field: scene.field,
        dungeon: scene.dungeon,
        floor: scene.floor,
        block: scene.block,
        server_prev: scene.server_prev,
        town_prev: scene.town_prev,
        field_prev: scene.field_prev,
        dungeon_prev: scene.dungeon_prev,
        floor_prev: scene.floor_prev,
        block_prev: scene.block_prev,
        ..CcGame::default()
    }
}

/// The scripts' view of a field or dungeon.
pub struct AreaHost<'a> {
    pub world: &'a mut FieldWorld,
    pub ui: &'a mut FieldUi,
    pub st: &'a mut FieldState,
    /// This frame's camera input as `teach_camera1..3` test it.
    pub teach: [bool; 3],
    /// `teach_camera3` took the reset button this frame: its handler then
    /// sets `cpCtrl` 7 and starts the event camera (0x001abfb0), which
    /// turns the camera back behind Kite.
    pub reset: std::cell::Cell<bool>,
    /// The scripts' cutscene streams.
    pub stream: &'a mut AreaStream,
}

/// `teach_camera1..3`'s tests of the pad (`ccEvent::Execute` 0x001ab9b4):
/// with `camCtrlType` 1 (schemes A) part 1 counts L1 or R1 held, part 2 the
/// right stick pushed (power 32 or more), part 3 a press of R2; with 0
/// (schemes B) the right stick, R1 or R2 held, and L1.
pub fn teach_input(pad: &piney_input::Pad, ctrl_type: i32) -> [bool; 3] {
    let held = pad.direct.bits();
    let stick = pad.pow_r >= 32;
    if ctrl_type != 0 {
        [held & 0xc != 0, stick, pad.push.bits() & 0x2 != 0]
    } else {
        [stick, held & 0xa != 0, pad.push.bits() & 0x4 != 0]
    }
}

impl AreaHost<'_> {
    /// `Host::command_target` of `world`: the command target's scene index,
    /// its base type flags and id.
    pub(crate) fn target_ref(world: &FieldWorld) -> Option<CharRef> {
        let i = world.command_target()?;
        let ch = world.combat().scene.chars.get(i)?;
        Some(CharRef { handle: i as u32, types: ch.ty() as u32, code: ch.id() })
    }

    /// A frame of `piros_colour`: its sounds, line, flashes (`scFadeDef`)
    /// and Piros's tint ([`crate::piros`], as the town's host does it).
    fn piros_actions(&mut self, acts: Vec<crate::piros::Action>) {
        use crate::piros::Action;
        for a in acts {
            match a {
                Action::Se(n) => self.sound_effect(n),
                Action::Info(line) => {
                    let save = self.world.state().clone();
                    self.ui.announce_lines(&[line], &save);
                }
                Action::Flash { count, colour } => {
                    self.st.fade_def.flash(count, colour);
                }
                Action::Tint(t) => {
                    self.world.set_affect_colour(crate::piros::PIROS, t.fix, t.rate, t.colour);
                }
            }
        }
    }

    /// `gate_hack_anim` (case 157, main 0x001b20fc) around its wait:
    /// `forbid` and `targetForbid` 1, the panels and the map closing
    /// (status 3); afterwards `forbid` and `targetForbid` 0, both opening
    /// (status 1).
    fn gate_hack_menus(&mut self, on: bool) {
        let c = &mut self.ui.ctrl;
        c.forbid = i16::from(!on);
        c.target_forbid = i16::from(!on);
        c.panel_status = if on { 1 } else { 3 };
        c.map_status = if on { 1 } else { 3 };
    }

    /// The `ccMenuCtrl` members `player_skill` sets, from the field UI.
    fn skill_menu(&self) -> piney_battle::evparty::SkillMenu {
        let c = &self.ui.ctrl;
        piney_battle::evparty::SkillMenu {
            panel_status: c.panel_status,
            map_status: c.map_status,
            pl_attack: c.pl_attack,
            target_forbid: c.target_forbid,
            cmnd_target_fix: 0,
        }
    }

    fn set_skill_menu(&mut self, m: piney_battle::evparty::SkillMenu) {
        let c = &mut self.ui.ctrl;
        c.panel_status = m.panel_status;
        c.map_status = m.map_status;
        c.pl_attack = m.pl_attack;
        c.target_forbid = m.target_forbid;
    }
}

impl Host for AreaHost<'_> {
    fn save(&mut self) -> &mut SaveData {
        &mut self.world.state_mut().save
    }

    fn game(&self) -> Game {
        let mut g = game_of(&self.world.scene()).view();
        g.cnt_stop = self.st.game.cnt_stop;
        g
    }

    fn frame_rate(&self) -> i32 {
        piney_world::FRAME_RATE as i32
    }

    fn party(&self) -> Party {
        let ids = self.world.party();
        Party { ids, num: ids.iter().filter(|&&i| i >= 0).count() as i32 }
    }

    fn spc_present(&self, code: i16) -> bool {
        self.world.char_pos(2, code).is_some()
    }

    fn field_menu(&self) -> i32 {
        self.ui.menu_type()
    }

    fn entry_present(&self, list: EntryList, ty: i16, code: i16) -> bool {
        self.world.entry_present(list == EntryList::Gimmicks, ty, code)
    }

    /// `eventMng.bossEntry` (Skeith's 0) and its task's parameter.
    fn boss(&self) -> Option<piney_event::host::Boss> {
        self.world.boss_task().map(|p| piney_event::host::Boss { entry: 0, task_param: Some(p) })
    }

    fn no_active_object(&self) -> bool {
        self.world.no_active_object()
    }

    fn no_entries(&self) -> bool {
        self.world.no_entries()
    }

    /// `cmndTarget`: its base type (+0x08) and code (+0x0c).
    fn command_target(&self) -> Option<CharRef> {
        AreaHost::target_ref(self.world)
    }

    fn target_alive(&self, target: &CharRef) -> bool {
        self.world.listed(target.handle as usize)
    }

    fn pad_pushed(&self) -> u32 {
        self.st.pad.push.bits()
    }

    fn story_area(&self, area: i16) -> Option<StoryArea> {
        self.st.stories.get(&area).copied()
    }

    /// `teach_camera1..3` wait for the pad: a turn (1), a zoom (2), a reset
    /// (3). Parts 1 and 2 gave the event camera to the pad as their prompt
    /// opened; part 3 does as the button is taken.
    fn teach_input(&self, part: u8) -> bool {
        let input = self.teach.get(usize::from(part).wrapping_sub(1)).copied().unwrap_or(true);
        if part == 3 && input {
            self.reset.set(true);
        }
        input
    }

    /// `camCtrlType`.
    fn camera_type(&self) -> i32 {
        self.world.camera().scheme.ctrl_type
    }

    // --- Windows --------------------------------------------------------------------------

    fn message_open(&mut self, call: &MessageCall<'_>) {
        self.st.shown.push((call.event, call.msg));
        self.st.log(format!("message_open {} {} {:?}", call.event, call.msg, call.kind));
        let save = self.world.state().clone();
        self.ui.message_open(call, &save);
        if let MessageKind::Teach(part @ (1 | 2)) = call.kind {
            self.world.teach_camera(part);
        }
    }

    fn message_check(&mut self) -> i32 {
        let pad = self.st.pad;
        let r = self.ui.message_check(&pad, self.world.state());
        if r != 0 {
            self.st.log(format!("message_check {r}"));
        }
        r
    }

    fn message_close(&mut self) {
        self.st.log("message_close".into());
        self.ui.message_close();
    }

    fn announce(&mut self, a: Announce) {
        self.st.log(format!("announce {a:?}"));
        let name = match a {
            Announce::Member { pc } => self.world.char_name(i32::from(pc)),
            _ => Vec::new(),
        };
        let save = self.world.state().clone();
        self.ui.announce(a, &name, &save);
    }

    // --- Modes and the screen -------------------------------------------------------------

    fn change_request(&mut self, num: i32, sf: i32) {
        self.st.log(format!("change_request {num} {sf}"));
        self.st.events.push(Event::ChangeMode { num, sf });
    }

    fn change_area(&mut self, a: i32, n: i32) {
        self.st.log(format!("change_area {a} {n}"));
        self.world.change_area(a, n);
    }

    /// `ccGame::ChangeScene`: recorded for the session, which changes the
    /// scene (with the `area` before it) and sets the next one up;
    /// `ChangeRequest(6, 7)` puts the task to sleep.
    fn change_scene(&mut self, area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16) {
        self.st.log(format!("change_scene {area} {town} {field} {dungeon} {floor} {block}"));
        let s = [area, town, field, dungeon, floor, block];
        self.st.change = Some(SceneChange { scene: s, area: self.st.area });
        self.st.events.push(Event::ChangeMode { num: 6, sf: 7 });
    }

    fn fade(&mut self, count: i16, alpha: i16, more: bool) {
        self.st.log(format!("fade {count} {alpha} more={more}"));
        let col1 = (alpha as u32 & 0xff) << 24;
        if more {
            self.st.fade.continue_fade(self.st.fade_num, count, col1);
        } else {
            self.st.fade_num = self.st.fade.entry(count, 0, col1);
        }
    }

    fn noise(&mut self, level: i32) {
        self.st.log(format!("noise {level}"));
        self.ui.noise(level);
    }

    /// `ccSndEvRequest` ([`crate::mode::sound_request`]).
    fn sound(&mut self, cmd: i16, p0: i16, p1: i16, p2: i16) {
        self.st.log(format!("sound {cmd} {p0} {p1} {p2}"));
        self.st.events.extend(crate::mode::sound_request(cmd, p0, p1, p2));
    }

    fn sound_effect(&mut self, se: i32) {
        self.st.log(format!("sound_effect {se}"));
        self.st.events.push(Event::Se(se));
    }

    /// `piros_colour` ([`crate::piros`]): on `eventStatus[1]`, with Piros
    /// among the field's characters; its breaths are the wait's
    /// ([`Host::busy`]).
    fn piros_colour(&mut self, code: i16, line: &[u8]) {
        self.st.log(format!("piros_colour {code}"));
        let status = self.world.state().save.u8(crate::piros::STATUS) as i8;
        let loaded = self.world.spc_loaded(crate::piros::PIROS);
        let mut seq = crate::piros::Sequence::new(status, code, line, loaded);
        let acts = seq.start();
        self.piros_actions(acts);
        self.st.piros = Some(seq);
    }

    /// `player_skill`'s set-up: `ccMenu +0xc` (`panelStatus`) 1, `+0x102`
    /// (`targetForbid`) 0, `cmndTargetFix` 1, the target's box withheld.
    fn begin(&mut self, w: Wait) {
        if w == Wait::PlayerSkill {
            self.st.log("player_skill".into());
            let mut m = self.skill_menu();
            self.world.player_skill_begin(&mut m);
            self.set_skill_menu(m);
        }
        if w == Wait::GateHackAnim {
            self.st.log("gate_hack_anim".into());
            self.gate_hack_menus(false);
        }
    }

    /// The town's `scene` woke in this area's set-up (its instruction
    /// ends); a `scene` asked here sleeps until the next area.
    /// `player_skill` waits while the command target is alive. A `stream`
    /// plays a step a frame until it ends.
    fn busy(&mut self, w: Wait) -> bool {
        match w {
            Wait::ChangeRequest => self.st.change.is_some(),
            Wait::PlayerSkill => self.world.player_skill_busy(),
            // gate_hack_anim: while ccCheckGtHackAnm() (ghoFlag).
            Wait::GateHackAnim => self.world.gate_hack_anim(),
            // piros_colour: its breaths.
            Wait::PirosColour => {
                let Some(mut seq) = self.st.piros.take() else { return false };
                let (acts, busy) = seq.tick();
                self.piros_actions(acts);
                if busy {
                    self.st.piros = Some(seq);
                }
                busy
            }
            Wait::Stream => {
                let Some(p) = &mut self.stream.player else { return false };
                match p.step(&self.st.pad, &mut self.st.events) {
                    Some(f) => {
                        self.stream.frame = Some(f);
                        true
                    }
                    None => {
                        self.stream.player = None;
                        self.st.log("stream done".into());
                        false
                    }
                }
            }
            _ => false,
        }
    }

    /// `stream num` (`ccEventStream(num, 1)`, case 10): the cutscene from
    /// the disc, played while the instruction waits ([`Host::busy`]). A
    /// stream that does not start counts as played.
    fn stream(&mut self, num: i16) {
        self.st.log(format!("stream {num}"));
        let save = self.world.state().save.clone();
        let iso = self.stream.iso.clone();
        // ccEventStream(num, 1): the subtitles and ccSndStreamCtrl's music
        // as the town has them.
        let game = piney_audio::stream::StreamGame { status: STATUS_WORLD, field: self.world.scene().field };
        let data = self.stream.data.clone();
        let started = usize::try_from(num)
            .map_err(|_| format!("stream {num}"))
            .and_then(|n| StreamPlayer::event(&iso, data.as_deref(), n, &save, game, &mut self.st.events));
        match started {
            Ok(p) => self.stream.player = Some(p),
            Err(e) => eprintln!("events: {e}; counted as played"),
        }
    }

    /// `remove_trap` (case 123): its sound (228) is the interpreter's; the
    /// trap-removal effect at the command target (`effRemoveTrap(pos, -1,
    /// -1)`) starts with the frame's other shows.
    fn remove_trap(&mut self) {
        self.st.log("remove_trap".into());
        self.world.remove_trap_effect();
    }

    /// `item_add_menu` while playing (case 94): `ccMenu.itemNum` (+0x238)
    /// the item (`category << 16 | id`), the item-get menu (29) asked
    /// (`openReqNum` 29, `mode` 0, `firstTime` 1); the menu gives it.
    fn item_get_menu(&mut self, pc: i16, category: i16, id: i16, num: i16) {
        self.st.log(format!("item_get_menu {pc} {category} {id} {num}"));
        let c = &mut self.ui.ctrl;
        c.item_num = (i32::from(category) << 16) | i32::from(id as u16);
        c.open_req = 29;
        c.mode = 0;
        c.first_time = 1;
    }

    /// After the item-get menu closed: `ccMenu +0x0c` (`panelStatus`) 3.
    fn item_get_menu_end(&mut self) {
        self.st.log("item_get_menu_end".into());
        self.ui.ctrl.panel_status = 3;
    }

    /// `compulsionGameOver`.
    fn game_over(&self) -> bool {
        self.world.compulsion_game_over
    }

    /// `ccClearGtHack` (main 0x001b77c0), from `ccStartThEvent`:
    /// `gtHackFlag` cleared unless a field (or a dungeon of type 8 or 9)
    /// was entered from a town.
    fn clear_gate_hack(&mut self) {
        self.world.clear_gt_hack();
        self.st.log(format!("clear_gate_hack (gtHackFlag {})", u8::from(self.world.gt_hack())));
    }

    /// `remove_trap`, 20 frames on: `EntryAffect(cmndTarget, plw, 12, 0,
    /// 0, 0)`, the trap taken off the command target (none: nothing).
    fn remove_trap_done(&mut self) {
        if let Some(t) = self.world.command_target() {
            self.world.affect_from_player(t, 12);
        }
    }

    /// `open_door` / `close_door` in a dungeon (`DUNGEON::OpenDoor`,
    /// `CloseDoor2`); the others are not reached outside the towns yet.
    fn gimmick(&mut self, g: GimmickCommand) {
        self.st.log(format!("gimmick {g:?}"));
        match g {
            GimmickCommand::OpenDoor => self.world.open_door(),
            GimmickCommand::CloseDoor => self.world.close_door(),
            _ => {}
        }
    }

    /// `player_skill`'s end: the target let go, `plAttack` 0, the panels
    /// and the minimap fading out.
    fn end(&mut self, w: Wait) {
        if w == Wait::PlayerSkill {
            self.st.log("player_skill done".into());
            let mut m = self.skill_menu();
            self.world.player_skill_end(&mut m);
            self.set_skill_menu(m);
        }
        if w == Wait::GateHackAnim {
            self.st.log("gate_hack_anim done".into());
            self.gate_hack_menus(true);
        }
    }

    /// One frame of `player_skill`: the attack button asks Kite's normal
    /// attack on the target, and `plAttack` follows it.
    fn player_skill_step(&mut self) {
        let push = self.st.pad.push.bits();
        let mut m = self.skill_menu();
        self.world.player_skill_step(push, &mut m);
        self.set_skill_menu(m);
    }

    // --- The area -------------------------------------------------------------------------

    fn camera(&mut self, c: CameraCommand) {
        self.st.log(format!("camera {c:?}"));
        self.world.event_camera(c);
    }

    fn pc(&mut self, c: PcCommand) {
        let done = self.world.pc_command(c);
        self.st.log(format!("pc {c:?}{}", if done { "" } else { " (not done)" }));
    }

    /// `near_marker`'s distance from Kite to an event position.
    fn player_distance(&self, pos: Option<[f32; 4]>) -> f32 {
        let Some(p) = pos else { return f32::MAX };
        f32::from_bits(self.world.player_distance(p.map(f32::to_bits)))
    }

    /// The event's NPCs outside the towns (`piney_world::field_npcs`).
    fn npc(&mut self, c: NpcCommand) {
        let done = self.world.npc_command(c);
        self.st.log(format!("npc {c:?}{}", if done { "" } else { " (not done)" }));
    }

    fn trans(&mut self, ty: i16, code: i16, on: bool) {
        let done = self.world.set_trans(ty, code, on);
        self.st.log(format!("trans {ty} {code} {on}{}", if done { "" } else { " (not done)" }));
    }

    fn menu_ban(&mut self, on: bool) {
        self.st.log(format!("menu_ban {on}"));
        self.ui.menu_ban(on);
        self.world.menu_ban_camera(on);
        self.world.menu_ban_party(on);
    }

    fn open_menu(&mut self, num: i16) {
        self.st.log(format!("open_menu {num}"));
        self.ui.event_menu(num);
    }

    /// `item_add` for a companion (case 93, main 0x001afd04, playing, pc
    /// not 0, category 0-9): `ccEvent::GetSpc(pc)`, the character the area
    /// built for that registry id (S108's Sanjuro, S111's Natsume), then
    /// `ccMenu->AddSpcItem(ch, category, id, num, 0)`. None built: false,
    /// and the item goes into the save.
    fn add_spc_item(&mut self, pc: i16, category: i16, id: i16, num: i16) -> bool {
        let code = i32::from(pc);
        if self.world.combat().who(code).is_none() {
            return false;
        }
        self.st.log(format!("add_spc_item {pc} {category} {id} {num}"));
        let scene = self.world.scene();
        let w = piney_fieldui::World {
            game: piney_fieldui::Game {
                status: 5,
                area: scene.area,
                town: scene.town,
                field: scene.field,
                dungeon: scene.dungeon,
                floor: scene.floor,
                server: scene.server,
                ..Default::default()
            },
            ..Default::default()
        };
        // The party's handle as the HUD's view names it: (1 << 24) | id.
        let h = (1 << 24) | u32::from(pc as u16);
        let (c, i, n) = (i32::from(category), i32::from(id), i32::from(num));
        self.ui.add_spc_item(&w, self.world.state_mut(), h, code, c, i, n);
        true
    }

    /// `virus_core` runs `SimGenerateCode` only at level 1 (see the town's
    /// host): the area the words make on this area's server, as `WORLD_MAN`
    /// would be left.
    fn generate_area(&mut self, words: [Option<i32>; 3]) {
        self.st.log(format!("generate_area {words:?}"));
        if let [Some(a), Some(b), Some(c)] = words {
            let server = self.world.scene().server;
            let save = self.world.state().clone();
            self.ui.set_area_words([a, b, c], server, &save);
        }
    }

    /// `room` (case 130, main 0x001b0948) and `room_point` (131, the event
    /// point's room), playing only: `WORLD_MAN::RoomSelect(floor, block)`
    /// ([`FieldWorld::room_select`]), whose `DUNGEON::RoomSelect` sets
    /// `ccMenu`'s map status 3.
    fn room(&mut self, floor: i16, block: i16) {
        self.st.log(format!("room {floor} {block}"));
        if self.world.room_select(i32::from(floor), i32::from(block)) {
            self.ui.ctrl.map_status = 3;
        }
    }

    fn area(&mut self, area: i16) {
        let words = self.st.stories.get(&area).map(|a| a.words).unwrap_or_default();
        self.st.log(format!("area {area} {words:?}"));
        self.st.area = Some((area, words));
    }

    fn map_on(&mut self) {
        self.st.log("map_on".into());
        self.ui.map_on();
    }

    /// `WORLD_MAN::ShowMap`: the field's `WORLD::ShowMap` (the portals on
    /// the map) at once, or one room of the dungeon's a frame.
    fn show_map(&mut self) -> bool {
        let model = self.world.world_man().field_model;
        let pos = self.world.player().body.pos;
        let done = piney_world::map::show_map(self.world.place_mut(), model, pos);
        if done {
            self.st.log("show_map".into());
        }
        done
    }

    fn target_forbid(&mut self, on: bool) {
        self.st.log(format!("target_forbid {on}"));
        self.ui.target_forbid(on);
    }

    /// `condition_fx_on` / `condition_fx_off`: `spcConditionEffectFlag`.
    fn condition_effect(&mut self, on: bool) {
        self.st.log(format!("condition_effect {on}"));
        self.world.condition_effect(on);
    }

    /// `remove 5|6 code`: the enemy deleted from the entry control.
    fn remove(&mut self, ty: i16, code: i16) {
        let done = self.world.remove_entry(ty, code);
        self.st.log(format!("remove {ty} {code}{}", if done { "" } else { " (not done)" }));
    }

    /// `party_add pc` (case 77): `ccParty::AddMember(pc)` over the field's
    /// characters, and the new slot's menu face (`SetMenuFace`).
    fn party_add(&mut self, pc: i16) {
        let slot = self.world.party_add(i32::from(pc));
        self.st.log(format!("party_add {pc}{}", if slot >= 0 { "" } else { " (not done)" }));
        if slot >= 0 {
            let plcol = self.world.state().save.u8(piney_data::save::offset::PLCOL) != 0;
            self.ui.set_menu_face(slot as usize, i32::from(pc), plcol);
        }
    }

    /// `party_remove pc` (case 78): `ccParty::DelMember` of the member's
    /// slot (or of slot `-pc`, or all but Kite's for -3); the member's
    /// character stays where it is, out of the party.
    fn party_remove(&mut self, pc: i16) {
        self.st.log(format!("party_remove {pc}"));
        self.world.party_remove(i32::from(pc));
    }

    /// `enemy_put enemy posnum`: the enemy to the event position (its
    /// heading from the position the area's set-up took, when it is one).
    fn enemy_put(&mut self, enemy: i16, pos: Option<[f32; 4]>) {
        let Some(p) = pos else {
            self.st.log(format!("enemy_put {enemy} (no position)"));
            return;
        };
        let bits = p.map(f32::to_bits);
        let dirc = self.world.event_positions().iter().find(|e| e.pos[..3] == bits[..3]).map(|e| e.dirc);
        let done = self.world.enemy_put(enemy, bits, dirc);
        self.st.log(format!("enemy_put {enemy} {p:?}{}", if done { "" } else { " (not done)" }));
    }

    /// `hold type code` / `hold_end`: the event's `ccThEvHold` task.
    fn hold(&mut self, target: Option<(i16, i16)>) {
        self.st.log(format!("hold {target:?}"));
        self.world.set_event_hold(target);
    }

    fn battle_ready(&mut self) {
        self.st.log("battle_ready".into());
        self.world.battle_ready();
    }

    /// The first play pass's end (`ccThEvent`, main 0x001b5eb0): with
    /// `WORLD_MAN.hackFlag` 3 (a hacked area, or any in the crisis)
    /// `ccMenu.interNoiz = 1`, the menu's noise.
    fn play_pass_done(&mut self) {
        let hacked = self.world.world_man().hack == 3;
        self.st.log(format!("play_pass_done{}", if hacked { " noise 1" } else { "" }));
        if hacked {
            self.ui.noise(1);
        }
    }
}
