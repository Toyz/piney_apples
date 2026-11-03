//! The event scripts' host in The World: `piney-event`'s interpreter as
//! `ccThEvent` (priority 32) runs beside the area's tasks, one host for a
//! town, a field, a dungeon or a story map ([`EvHost`]). Windows go to the
//! field UI's `ccMsg`, menus to `ccMenu`, the camera instructions to the
//! event camera, `pc_*` and `npc_*` to the world's characters, `stream` to
//! `ccEventStream(num, 1)`, `scene` to the session, `sound` to
//! `ccSndEvRequest`; every call is logged with its frame
//! ([`FieldState::calls`]). What differs by world is [`HostWorld`].

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::save::{SaveData, offset};
use piney_desktop::SaveState;
use piney_draw::Frame;
use piney_event::host::{
    Announce, CameraCommand, CharRef, EntryList, Game, GimmickCommand, Host, Marker, MessageCall, MessageKind,
    NpcCommand, Party, PcCommand, StoryArea, Wait,
};
use piney_fieldui::FieldUi;

use crate::field_host::{FieldState, SceneChange};
use crate::mode::Event;
use crate::stream::{FileList, StreamPlayer};

/// The scripts' `stream` (`ccEventStream(num, 1)`, [`crate::stream`]): the
/// disc it plays from and DATA.BIN for its pictures (None: streams count as
/// played), the player while the call waits, and its frame this game frame.
#[derive(Default)]
pub struct EvStream {
    pub iso: Option<PathBuf>,
    pub data: Option<Arc<Archive>>,
    pub player: Option<StreamPlayer>,
    pub frame: Option<Frame>,
}

/// What the event host reads and moves of the world it hosts: the
/// world's own answers to the instructions that reach the area.
pub trait HostWorld {
    fn state(&self) -> &SaveState;
    fn state_mut(&mut self) -> &mut SaveState;
    fn state_out(&self) -> SaveState;

    /// `ccGame` as the scripts read it.
    fn game(&self, st: &FieldState) -> Game;
    /// `ccGame::ChangeArea` and `ccGame::ChangeScene` from the scripts.
    fn change_area(&mut self, st: &mut FieldState, a: i32, n: i32);
    fn change_scene(&mut self, st: &mut FieldState, scene: [i16; 6]);
    /// `ccEventStream`'s `ccGame` and the file list its subtitles read.
    fn stream_game(&self, st: &FieldState) -> (piney_audio::stream::StreamGame, FileList);
    /// The game's `rand()` handed back by a stream.
    fn set_rand(&mut self, rand: u64);
    /// `game.server`.
    fn server(&self) -> i32;
    /// `WORLD_MAN.hackFlag` 3 at the first play pass: `ccMenu.interNoiz`.
    fn menu_noise(&self) -> bool;

    fn party(&self) -> [i32; 3];
    /// The character of event type `ty` and `code`, where it stands.
    fn char_pos(&self, ty: i16, code: i16) -> Option<[u32; 4]>;
    /// `charTbl[pc].name`.
    fn char_name(&self, ui: &FieldUi, pc: i16) -> Vec<u8>;
    fn marker(&self, marker: i16) -> Option<Marker>;
    fn command_target(&self) -> Option<CharRef>;
    fn target_alive(&self, target: &CharRef) -> bool;
    fn player_distance(&self, pos: [u32; 4]) -> u32;
    fn camera_type(&self) -> Option<i32>;
    fn teach_camera(&mut self, part: u8);
    fn event_camera(&mut self, c: CameraCommand) -> bool;
    fn npc_command(&mut self, c: NpcCommand) -> bool;
    fn pc_command(&mut self, c: PcCommand) -> bool;
    fn set_trans(&mut self, ty: i16, code: i16, on: bool) -> bool;
    fn set_affect_colour(&mut self, code: i32, fix: i16, rate: i16, colour: u32) -> bool;
    fn spc_loaded(&self, code: i32) -> bool;
    fn menu_ban(&mut self, on: bool);
    fn party_add(&mut self, pc: i32) -> i32;
    fn party_remove(&mut self, pc: i32);
    fn remove(&mut self, ty: i16, code: i16) -> bool;
    /// `ccMenu->AddSpcItem`'s view of party character `code`: the field UI's
    /// world and the character's handle, None when none is built here.
    fn spc_item(&self, code: i32) -> Option<(piney_fieldui::World, u32)>;

    /// `compulsionGameOver`, `ccClearGtHack` (the flag left, None where the
    /// scene drops it), `spcConditionEffectFlag`. A town keeps none.
    fn game_over(&self) -> bool {
        false
    }
    fn clear_gt_hack(&mut self) -> Option<bool> {
        None
    }
    fn condition_effect(&mut self, _on: bool) {}

    /// The entry control and the boss as the conditions read them: a
    /// town's list has no enemies, portals or boss.
    fn entry_present(&self, _list: EntryList, _ty: i16, _code: i16) -> bool {
        false
    }
    fn boss(&self) -> Option<piney_event::host::Boss> {
        None
    }
    fn no_active_object(&self) -> bool {
        true
    }
    fn no_entries(&self) -> bool {
        true
    }

    /// The area's own instructions: the trap's effect and affect, the
    /// doors and rays, `room`, `ShowMap`, the event positions, `enemy_put`,
    /// `hold`, `battle_ready`; false where the area has nothing of them.
    fn remove_trap(&mut self) -> bool {
        false
    }
    fn remove_trap_done(&mut self) {}
    fn gimmick(&mut self, _g: GimmickCommand) -> bool {
        false
    }
    fn room(&mut self, _floor: i16, _block: i16) -> bool {
        false
    }
    fn show_map(&mut self) -> bool {
        true
    }
    fn event_positions(&mut self, _positions: &[piney_event::vm::EvPos]) {}
    fn enemy_put(&mut self, _enemy: i16, _pos: [u32; 4]) -> bool {
        false
    }
    fn hold(&mut self, _target: Option<(i16, i16)>) -> bool {
        false
    }
    fn battle_ready(&mut self) -> bool {
        false
    }

    /// The waits the area runs: `player_skill` (with `ccMenuCtrl`'s
    /// members), `gate_hack_anim`, `ending_kanji`; false (None) for one it
    /// has none of.
    fn player_skill_begin(&mut self, _m: &mut piney_battle::evparty::SkillMenu) -> bool {
        false
    }
    fn player_skill_busy(&self) -> bool {
        false
    }
    fn player_skill_step(&mut self, _push: u32, _m: &mut piney_battle::evparty::SkillMenu) {}
    fn player_skill_end(&mut self, _m: &mut piney_battle::evparty::SkillMenu) -> bool {
        false
    }
    fn gate_hack_anim(&self) -> Option<bool> {
        None
    }
    fn ending_start(&mut self) -> bool {
        false
    }
    fn ending_running(&mut self) -> bool {
        false
    }
}

/// The scripts' view of The World.
pub struct EvHost<'a, W: HostWorld> {
    pub world: &'a mut W,
    pub ui: &'a mut FieldUi,
    pub st: &'a mut FieldState,
    /// This frame's camera input as `teach_camera1..3` test it (None: the
    /// world answers at once).
    pub teach: Option<[bool; 3]>,
    /// `teach_camera3` took the reset button this frame: its handler then
    /// sets `cpCtrl` 7 and starts the event camera (0x001abfb0), which
    /// turns the camera back behind Kite.
    pub reset: std::cell::Cell<bool>,
    /// The scripts' cutscene streams.
    pub stream: &'a mut EvStream,
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

impl<W: HostWorld> EvHost<'_, W> {
    /// A frame of `piros_colour`: its flashes (`scFadeDef`) and Piros's
    /// tint ([`crate::piros`]).
    fn piros_actions(&mut self, acts: Vec<crate::piros::Action>) {
        use crate::piros::Action;
        for a in acts {
            match a {
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

impl<W: HostWorld> Host for EvHost<'_, W> {
    fn save(&mut self) -> &mut SaveData {
        &mut self.world.state_mut().save
    }

    fn game(&self) -> Game {
        self.world.game(self.st)
    }

    fn frame_rate(&self) -> i32 {
        piney_world::FRAME_RATE as i32
    }

    fn party(&self) -> Party {
        let ids = self.world.party();
        Party { ids, num: ids.iter().filter(|&&i| i >= 0).count() as i32 }
    }

    /// The SPC manager holds Kite and the members the area built.
    fn spc_present(&self, code: i16) -> bool {
        self.world.char_pos(2, code).is_some()
    }

    fn field_menu(&self) -> i32 {
        self.ui.menu_type()
    }

    fn entry_present(&self, list: EntryList, ty: i16, code: i16) -> bool {
        self.world.entry_present(list, ty, code)
    }

    /// `eventMng.bossEntry` (the boss's code) and its task's parameter.
    fn boss(&self) -> Option<piney_event::host::Boss> {
        self.world.boss()
    }

    fn no_active_object(&self) -> bool {
        self.world.no_active_object()
    }

    fn no_entries(&self) -> bool {
        self.world.no_entries()
    }

    /// `cmndTarget`: its base type (+0x08) and code (+0x0c).
    fn command_target(&self) -> Option<CharRef> {
        self.world.command_target()
    }

    fn target_alive(&self, target: &CharRef) -> bool {
        self.world.target_alive(target)
    }

    fn pad_pushed(&self) -> u32 {
        self.st.pad.push.bits()
    }

    fn marker(&self, marker: i16) -> Option<Marker> {
        self.world.marker(marker)
    }

    /// `near_marker`'s distance; a marker the area lacks reads an unset
    /// vector in the game, taken here as out of every bound.
    fn player_distance(&self, pos: Option<[f32; 4]>) -> f32 {
        let Some(p) = pos else { return f32::MAX };
        f32::from_bits(self.world.player_distance(p.map(f32::to_bits)))
    }

    fn story_area(&self, area: i16) -> Option<StoryArea> {
        self.st.stories.get(&area).copied()
    }

    /// `teach_camera1..3` wait for the pad: a turn (1), a zoom (2), a reset
    /// (3). Parts 1 and 2 gave the event camera to the pad as their prompt
    /// opened; part 3 does as the button is taken.
    fn teach_input(&self, part: u8) -> bool {
        let Some(teach) = self.teach else { return true };
        let input = teach.get(usize::from(part).wrapping_sub(1)).copied().unwrap_or(true);
        if part == 3 && input {
            self.reset.set(true);
        }
        input
    }

    /// `camCtrlType`.
    fn camera_type(&self) -> i32 {
        self.world.camera_type().unwrap_or(0)
    }

    // --- Windows --------------------------------------------------------------------------

    fn message_open(&mut self, call: &MessageCall<'_>) {
        self.st.shown.push((call.event, call.msg));
        self.st.log(format!("message_open {} {} {:?}", call.event, call.msg, call.kind));
        let save = self.world.state().clone();
        self.ui.message_open(call, &save);
        // teach_camera1 and 2 turn the event camera over to the pad as
        // their prompt opens (cpCtrl 5, 6).
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

    /// `DispInfo` (0x001b27b0): it waits for the menus to be shut and shows
    /// the lines `Execute` composed: a member's address as the field UI
    /// words it, a gate address or a desktop item as the desktop does
    /// ([`crate::story::Announcements`]).
    fn announce(&mut self, a: Announce) {
        self.st.log(format!("announce {a:?}"));
        let save = self.world.state().clone();
        match a {
            Announce::Member { pc } => {
                let name = self.world.char_name(self.ui, pc);
                self.ui.announce(a, &name, &save);
            }
            _ => {
                let lines = self.st.announcements.as_ref().map(|t| t.lines(a, &save.save)).unwrap_or_default();
                self.ui.announce_lines(&lines, &save);
            }
        }
    }

    // --- Modes and the screen -------------------------------------------------------------

    fn change_request(&mut self, num: i32, sf: i32) {
        self.st.log(format!("change_request {num} {sf}"));
        self.st.events.push(Event::ChangeMode { num, sf });
    }

    fn change_area(&mut self, a: i32, n: i32) {
        self.st.log(format!("change_area {a} {n}"));
        self.world.change_area(self.st, a, n);
    }

    /// `ccGame::ChangeScene` (0x00167380): the scene moves on, recorded for
    /// the session, which sets the next scene up; `ChangeRequest(6, 7)`
    /// stops the task ([`Host::busy`]).
    fn change_scene(&mut self, area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16) {
        self.st.log(format!("change_scene {area} {town} {field} {dungeon} {floor} {block}"));
        let s = [area, town, field, dungeon, floor, block];
        self.world.change_scene(self.st, s);
        self.st.change = Some(SceneChange { scene: s, area: self.st.area });
        self.st.events.push(Event::ChangeMode { num: 6, sf: 7 });
    }

    fn fade(&mut self, count: i16, alpha: i16, more: bool, reset: bool) {
        self.st.log(format!("fade {count} {alpha} more={more}"));
        self.st.fade.event(&mut self.st.fade_num, count, alpha, more, reset);
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

    /// `compulsionGameOver` (0x00378c74).
    fn game_over(&self) -> bool {
        self.world.game_over()
    }

    /// `ccClearGtHack` (main 0x001b77c0), from `ccStartThEvent`.
    fn clear_gate_hack(&mut self) {
        match self.world.clear_gt_hack() {
            Some(flag) => self.st.log(format!("clear_gate_hack (gtHackFlag {})", u8::from(flag))),
            None => self.st.log("clear_gate_hack".into()),
        }
    }

    /// `piros_colour` ([`crate::piros`]): on `eventStatus[1]`, with Piros
    /// among the area's characters; its breaths are the wait's.
    fn piros_colour(&mut self, code: i16) -> bool {
        self.st.log(format!("piros_colour {code}"));
        let status = self.world.state().save.u8(crate::piros::STATUS) as i8;
        let loaded = self.world.spc_loaded(crate::piros::PIROS);
        let seq = crate::piros::Sequence::new(status, code, loaded);
        let tells = seq.tells();
        self.st.piros = Some(seq);
        tells
    }

    /// `DispInfo`'s window with `piros_colour`'s line.
    fn info_lines(&mut self, lines: &[&[u8]]) {
        self.st.log(format!("info_lines {}", lines.len()));
        let lines: Vec<Vec<u8>> = lines.iter().map(|l| l.to_vec()).collect();
        let save = self.world.state().clone();
        self.ui.announce_lines(&lines, &save);
    }

    /// `player_skill`'s set-up, `gate_hack_anim`'s menus, `ending_kanji`'s
    /// start; `piros_colour` needs nothing before its wait.
    fn begin(&mut self, w: Wait) {
        match w {
            Wait::PlayerSkill => {
                let mut m = self.skill_menu();
                if self.world.player_skill_begin(&mut m) {
                    self.st.log("player_skill".into());
                    self.set_skill_menu(m);
                }
            }
            Wait::GateHackAnim if self.world.gate_hack_anim().is_some() => {
                self.st.log("gate_hack_anim".into());
                self.gate_hack_menus(false);
            }
            Wait::EndingKanji if self.world.ending_start() => self.st.log("ending_kanji".into()),
            _ => {}
        }
    }

    /// `scene`'s `ChangeRequest(6, 7)`: asleep from then on. `player_skill`
    /// waits while the command target is alive, `gate_hack_anim` while
    /// `ghoFlag`, `ending_kanji` until the staff roll ends. `stream`: a
    /// frame of it per game frame until it ends. `piros_colour`: its
    /// breaths.
    fn busy(&mut self, w: Wait) -> bool {
        match w {
            Wait::PlayerSkill => self.world.player_skill_busy(),
            Wait::GateHackAnim => self.world.gate_hack_anim().unwrap_or(false),
            Wait::EndingKanji => self.world.ending_running(),
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
                        self.world.set_rand(p.rand());
                        self.stream.player = None;
                        self.st.log("stream done".into());
                        false
                    }
                }
            }
            _ => false,
        }
    }

    /// `ccEventStream(num, 1)` (0x001b5670): the stream with its subtitles
    /// and the music around it, played inside the call. A stream that
    /// cannot start counts as played.
    fn stream(&mut self, num: i16) {
        self.st.log(format!("stream {num}"));
        let Some(iso) = self.stream.iso.clone() else { return };
        let save = self.world.state_out();
        let (game, list) = self.world.stream_game(self.st);
        let data = self.stream.data.clone();
        let started = usize::try_from(num)
            .map_err(|_| format!("stream {num}"))
            .and_then(|n| StreamPlayer::event(&iso, data.as_deref(), n, &save, game, list, &mut self.st.events));
        match started {
            Ok(p) => self.stream.player = Some(p),
            Err(e) => tracing::warn!("events: {e}; counted as played"),
        }
    }

    /// `remove_trap` (case 123): its sound (228) is the interpreter's; the
    /// trap-removal effect at the command target starts with the frame's
    /// other shows.
    fn remove_trap(&mut self) {
        if self.world.remove_trap() {
            self.st.log("remove_trap".into());
        }
    }

    /// `remove_trap`, 20 frames on: `EntryAffect(cmndTarget, plw, 12, 0,
    /// 0, 0)`.
    fn remove_trap_done(&mut self) {
        self.world.remove_trap_done();
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

    /// `open_door` / `close_door` in a dungeon and `radiator`.
    fn gimmick(&mut self, g: GimmickCommand) {
        if self.world.gimmick(g) {
            self.st.log(format!("gimmick {g:?}"));
        }
    }

    /// `player_skill`'s end: the target let go, `plAttack` 0, the panels
    /// and the minimap fading out; `gate_hack_anim`'s menus back.
    fn end(&mut self, w: Wait) {
        match w {
            Wait::PlayerSkill => {
                let mut m = self.skill_menu();
                if self.world.player_skill_end(&mut m) {
                    self.st.log("player_skill done".into());
                    self.set_skill_menu(m);
                }
            }
            Wait::GateHackAnim if self.world.gate_hack_anim().is_some() => {
                self.st.log("gate_hack_anim done".into());
                self.gate_hack_menus(true);
            }
            _ => {}
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
        let done = self.world.event_camera(c);
        self.st.log(format!("camera {c:?}{}", if done { "" } else { " (not ported)" }));
    }

    fn pc(&mut self, c: PcCommand) {
        let done = self.world.pc_command(c);
        self.st.log(format!("pc {c:?}{}", if done { "" } else { " (not done)" }));
    }

    fn event_positions(&mut self, positions: &[piney_event::vm::EvPos]) {
        self.world.event_positions(positions);
    }

    fn npc(&mut self, c: NpcCommand) {
        let done = self.world.npc_command(c);
        self.st.log(format!("npc {c:?}{}", if done { "" } else { " (not done)" }));
    }

    fn trans(&mut self, ty: i16, code: i16, on: bool) {
        let done = self.world.set_trans(ty, code, on);
        self.st.log(format!("trans {ty} {code} {on}{}", if done { "" } else { " (not done)" }));
    }

    /// `ccEvent::MenuBan` / `MenuClr` (0x001b2460, 0x001b25c0): the menus
    /// forbidden and their panels and map hidden (`ccMenu`), the camera id
    /// kept, the party under manual control.
    fn menu_ban(&mut self, on: bool) {
        self.st.log(format!("menu_ban {on}"));
        self.ui.menu_ban(on);
        self.world.menu_ban(on);
    }

    fn open_menu(&mut self, num: i16) {
        self.st.log(format!("open_menu {num}"));
        self.ui.event_menu(num);
    }

    /// `item_add` for a companion (case 93, main 0x001afd04, playing, pc
    /// not 0, category 0-9): `ccEvent::GetSpc(pc)`, the character the area
    /// built for that registry id, then `ccMenu->AddSpcItem(ch, category,
    /// id, num, 0)`. None built: false, and the item goes into the save.
    fn add_spc_item(&mut self, pc: i16, category: i16, id: i16, num: i16) -> bool {
        let code = i32::from(pc);
        let Some((w, h)) = self.world.spc_item(code) else { return false };
        self.st.log(format!("add_spc_item {pc} {category} {id} {num}"));
        let (c, i, n) = (i32::from(category), i32::from(id), i32::from(num));
        self.ui.add_spc_item(&w, self.world.state_mut(), h, code, c, i, n);
        true
    }

    /// `virus_core` (case 124) runs `SimGenerateCode` only at level 1 (the
    /// port's story starts, `start.rs`'s host). Here, as `WORLD_MAN` would
    /// be left: the area the words make on the area's server, as Area
    /// Information shows it.
    fn generate_area(&mut self, words: [Option<i32>; 3]) {
        self.st.log(format!("generate_area {words:?}"));
        if let [Some(a), Some(b), Some(c)] = words {
            let server = self.world.server();
            let save = self.world.state().clone();
            self.ui.set_area_words([a, b, c], server, &save);
        }
    }

    /// `room` (case 130) and `room_point` (131), playing only:
    /// `WORLD_MAN::RoomSelect(floor, block)`, whose `DUNGEON::RoomSelect`
    /// sets `ccMenu`'s map status 3.
    fn room(&mut self, floor: i16, block: i16) {
        self.st.log(format!("room {floor} {block}"));
        if self.world.room(floor, block) {
            self.ui.ctrl.map_status = 3;
        }
    }

    /// `ccSystem::SetFrameRate` (event 31's opening).
    fn set_frame_rate(&mut self, rate: i16) {
        self.st.log(format!("set_frame_rate {rate}"));
        self.st.frame_rate = u32::try_from(rate).ok().filter(|&r| r > 0);
    }

    /// `overlay`: the game loads an overlay's code before a mode change; the
    /// port keeps every overlay's at once, so there is nothing to load.
    fn load_overlay(&mut self, which: piney_event::host::Overlay) {
        self.st.log(format!("load_overlay {which:?}"));
    }

    /// `area` (0x001b04b8): areas 1-13 quit the field (`WORLD_MAN::Quit`)
    /// and set `eventAreaNumber`; any other has `WORLD_MAN::SimGenerateCode`
    /// build it from its words. Recorded: the session builds it as the
    /// scene changes (`area::ev_area_world_man`).
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
    /// the map) at once, or one room of the dungeon's a frame; a town's
    /// answers at once.
    fn show_map(&mut self) -> bool {
        let done = self.world.show_map();
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
        self.world.condition_effect(on);
        self.st.log(format!("condition_effect {on}"));
    }

    fn remove(&mut self, ty: i16, code: i16) {
        let done = self.world.remove(ty, code);
        self.st.log(format!("remove {ty} {code}{}", if done { "" } else { " (not done)" }));
    }

    /// `party_add pc` (case 77): `ccParty::AddMember(pc)` over the area's
    /// characters, and the new slot's menu face (`SetMenuFace`).
    fn party_add(&mut self, pc: i16) {
        let slot = self.world.party_add(i32::from(pc));
        self.st.log(format!("party_add {pc}{}", if slot >= 0 { "" } else { " (not done)" }));
        if slot >= 0 {
            let plcol = self.world.state().save.u8(offset::PLCOL) != 0;
            self.ui.set_menu_face(slot as usize, i32::from(pc), plcol);
        }
    }

    /// `party_remove pc` (case 78): `ccParty::DelMember` of the member's
    /// slot (or of slot `-pc`, or all but Kite's for -3).
    fn party_remove(&mut self, pc: i16) {
        self.st.log(format!("party_remove {pc}"));
        self.world.party_remove(i32::from(pc));
    }

    /// `enemy_put enemy posnum`: the enemy to the event position.
    fn enemy_put(&mut self, enemy: i16, pos: Option<[f32; 4]>) {
        let Some(p) = pos else {
            self.st.log(format!("enemy_put {enemy} (no position)"));
            return;
        };
        let done = self.world.enemy_put(enemy, p.map(f32::to_bits));
        self.st.log(format!("enemy_put {enemy} {p:?}{}", if done { "" } else { " (not done)" }));
    }

    /// `hold type code` / `hold_end`: the event's `ccThEvHold` task.
    fn hold(&mut self, target: Option<(i16, i16)>) {
        if self.world.hold(target) {
            self.st.log(format!("hold {target:?}"));
        }
    }

    fn battle_ready(&mut self) {
        if self.world.battle_ready() {
            self.st.log("battle_ready".into());
        }
    }

    /// The first play pass's end (`ccThEvent`, main 0x001b5eb0): with
    /// `WORLD_MAN.hackFlag` 3 `ccMenu.interNoiz = 1`. `GO` sets `hackFlag`
    /// 3 for a hacked area and any area in the crisis (a town keeps it).
    fn play_pass_done(&mut self) {
        let noise = self.world.menu_noise();
        self.st.log(format!("play_pass_done{}", if noise { " noise 1" } else { "" }));
        if noise {
            self.ui.noise(1);
        }
    }
}
