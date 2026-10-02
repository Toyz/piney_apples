//! The event scripts in The World's towns: `piney-event`'s interpreter hosted by
//! the town, as `ccThEvent` (priority 32) runs beside its tasks. Windows go to
//! the field UI's `ccMsg` (their voice asked by the window), menus to `ccMenu`,
//! the camera instructions to the event camera, `pc_*` and `npc_*` to the
//! world's characters, `stream` to `ccEventStream(num, 1)`, `scene` to the
//! session (the task sleeps in its `ChangeRequest(6, 7)`), `sound` to
//! `ccSndEvRequest`. Every call is logged with its frame ([`FieldState::calls`]);
//! the instructions are in docs/engine/events.md.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::save::{SaveData, offset};
use piney_desktop::anm::Ctx;
use piney_draw::Frame;
use piney_event::host::{
    Announce, CameraCommand, CharRef, Game, Host, Marker, MessageCall, MessageKind, NpcCommand, Party, PcCommand,
    StoryArea, Wait,
};
use piney_fieldui::FieldUi;
use piney_input::Pad;
use piney_world::World;

use crate::mode::Event;
use crate::stream::StreamPlayer;

/// `ccGame.status` in The World.
pub const STATUS_WORLD: i32 = 5;

/// `ccScFade` (fade.cpp): four fading elements (`status`, `cnt`, `tcnt`, two
/// more counts, the rectangle, `col0`, `col1`) and a layer; the events use
/// `ccMenu`'s. `EntryFade` (0x00160400) takes the first free element (its
/// number the event keeps, `eventMng.fadeNum`), `ContinueFade` (0x00160490)
/// goes on from the old colour, `SendPacket` (0x0015fb80) draws and counts
/// each (docs/engine/event-vm.md).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScFade {
    /// +0x00: nothing is sent while it is set.
    pub off: i32,
    pub elems: [FadeElem; 4],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FadeElem {
    pub status: i16,
    pub cnt: i16,
    pub tcnt: i16,
    /// +0x06, +0x08: the chained fades' counts.
    pub tcnt_back: i16,
    pub tcnt_hold: i16,
    pub col0: u32,
    pub col1: u32,
}

/// The menu layer (`ccMenu`'s sprites, its fader).
pub const MENU_LAYER: i16 = 242;

impl ScFade {
    /// `EntryFade`: the element's number, -1 when all four are busy.
    pub fn entry(&mut self, n: i16, col0: u32, col1: u32) -> i32 {
        match self.elems.iter().position(|e| e.status == 0) {
            Some(i) => {
                let e = &mut self.elems[i];
                e.status = 1;
                e.tcnt = n;
                e.cnt = 0;
                e.col0 = col0;
                e.col1 = col1;
                i as i32
            }
            None => -1,
        }
    }

    /// `EntryFlash(n, col, ...)` (0x00160240): status 3, from `col` to
    /// `col` with alpha 0 over `n`; the element's number, -1 when all four
    /// are busy.
    pub fn flash(&mut self, n: i16, col: u32) -> i32 {
        let i = self.entry(n, col, col & 0x00ff_ffff);
        if let Some(e) = usize::try_from(i).ok().and_then(|i| self.elems.get_mut(i)) {
            e.status = 3;
        }
        i
    }

    /// The event's `fade` (case 145) and `fade_more` (146) on element
    /// `*num` (`eventMng.fadeNum`). From Mutation on (`reset`), `fade`
    /// empties the fader first (`ccScFade::Init`), and `fade_more` with no
    /// element (`*num` outside 0-3) starts a fade from `alpha` to nothing
    /// (MUT SLUS_205.62:0x001c6f50, 0x001c6fc0).
    pub fn event(&mut self, num: &mut i32, count: i16, alpha: i16, more: bool, reset: bool) {
        let col = (alpha as u32 & 0xff) << 24;
        let own = (0..4).contains(num);
        if more && (own || !reset) {
            self.continue_fade(*num, count, col);
            return;
        }
        if reset {
            self.elems.iter_mut().for_each(|e| e.status = 0);
        }
        *num = if more { self.entry(count, col, 0) } else { self.entry(count, 0, col) };
    }

    /// `ContinueFade(i, n, col1)`.
    pub fn continue_fade(&mut self, i: i32, n: i16, col1: u32) {
        let Some(e) = usize::try_from(i).ok().and_then(|i| self.elems.get_mut(i)) else { return };
        e.status = 1;
        e.tcnt = n;
        e.cnt = 0;
        e.col0 = e.col1;
        e.col1 = col1;
    }

    /// `SendPacket`'s counting, after each live element is drawn.
    pub fn count(&mut self) {
        if self.off != 0 {
            return;
        }
        for e in self.elems.iter_mut().filter(|e| e.status & 1 != 0) {
            e.cnt = e.cnt.wrapping_add(1);
            if e.cnt <= e.tcnt {
                continue;
            }
            if e.status & 2 != 0 {
                e.status = 0;
            } else if e.status & 4 != 0 {
                e.status = (e.status & !4) | 2;
                e.cnt = 0;
                e.tcnt = e.tcnt_back;
                e.col0 = e.col1;
                e.col1 &= 0x00ff_ffff;
            } else if e.status & 8 != 0 {
                e.status = (e.status & !8) | 4;
                e.cnt = 0;
                e.tcnt = e.tcnt_hold;
                e.col0 = e.col1;
            } else {
                e.cnt = e.tcnt;
            }
        }
    }

    /// `SendPacket`: draw each live element on `layer`, then count.
    pub fn send(&mut self, ctx: &mut Ctx, layer: i16) {
        if self.off != 0 {
            return;
        }
        for e in self.elems.iter().filter(|e| e.status & 1 != 0) {
            let cnt = u32::try_from(e.cnt).unwrap_or(0);
            let tcnt = u32::try_from(e.tcnt).unwrap_or(0).max(1);
            piney_desktop::fade::draw_on(ctx, layer, e.col0, e.col1, cnt, tcnt);
        }
        self.count();
    }

    /// The alpha of the first live element, for the title and the checks.
    pub fn alpha(&self) -> Option<u8> {
        self.elems.iter().find(|e| e.status & 1 != 0).map(|e| {
            let cnt = u32::try_from(e.cnt).unwrap_or(0);
            let tcnt = u32::try_from(e.tcnt).unwrap_or(0).max(1);
            piney_desktop::fade::colour(e.col0, e.col1, cnt, tcnt)[3]
        })
    }
}

/// `ccGame` (0x88 bytes) as far as the scene goes: +0x14 `area` ...
/// +0x30 `block`, the `*Prev` copies +0x34-+0x48, and the battle state
/// `ChangeScene` resets (+0x58 `inBattle`, +0x5c `inBattleCnt`, +0x60
/// `inBattleDist`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CcGame {
    pub status: i32,
    pub area: i32,
    pub area_prev: i32,
    pub server: i32,
    pub town: i32,
    pub field: i32,
    pub dungeon: i32,
    pub floor: i32,
    pub block: i32,
    pub server_prev: i32,
    pub town_prev: i32,
    pub field_prev: i32,
    pub dungeon_prev: i32,
    pub floor_prev: i32,
    pub block_prev: i32,
    pub in_battle: i32,
    pub in_battle_cnt: i32,
    /// Bits.
    pub in_battle_dist: u32,
    pub cnt_stop: i32,
}

/// `ChangeScene`'s server by town (0x00306dc0, 8 ints).
pub const SERVERS: [i32; 8] = [0, 1, 2, 3, 4, 0, 1, 2];

impl CcGame {
    /// `ccGame::ChangeScene(a, t, fd, d, f, b)` (0x00167380), before its
    /// `ChangeRequest(6, 7)`: each `*Prev` takes the old value and each of
    /// area, town, field, dungeon, floor, block the new one unless it is
    /// below -1; a town also goes into `saveData.lastTown` (+0x8426) and
    /// picks the server (a town of 8 or more would read past the table: not
    /// modelled, the server is kept); the battle state is cleared, the
    /// distance 2200.
    pub fn change_scene(&mut self, save: &mut SaveData, [a, t, fd, d, f, b]: [i32; 6]) {
        self.town_prev = self.town;
        if t >= -1 {
            self.town = t;
            save.set_u8(offset::LAST_TOWN, t as u8);
        }
        self.server_prev = self.server;
        if self.town >= 0
            && let Some(&s) = SERVERS.get(self.town as usize)
        {
            self.server = s;
        }
        for (v, prev, n) in [
            (&mut self.field, &mut self.field_prev, fd),
            (&mut self.dungeon, &mut self.dungeon_prev, d),
            (&mut self.floor, &mut self.floor_prev, f),
            (&mut self.block, &mut self.block_prev, b),
        ] {
            *prev = *v;
            if n >= -1 {
                *v = n;
            }
        }
        self.area_prev = self.area;
        if a >= -1 {
            self.area = a;
        }
        self.in_battle = 0;
        self.in_battle_cnt = 0;
        self.in_battle_dist = 0x4509_8000;
    }

    /// What the scripts read of it.
    pub fn view(&self) -> Game {
        Game {
            status: self.status,
            area: self.area,
            area_prev: self.area_prev,
            server: self.server,
            town: self.town,
            field: self.field,
            dungeon: self.dungeon,
            floor: self.floor,
            block: self.block,
            cnt_stop: self.cnt_stop,
        }
    }
}

/// A mode change the scripts asked for, which the field does not follow
/// yet: `ccGame::ChangeScene` and the `area` before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneChange {
    /// area, town, field, dungeon, floor, block.
    pub scene: [i16; 6],
    /// The `area` instruction's story area and its words (the field
    /// `WORLD_MAN::SimGenerateCode` would build), if one came first.
    pub area: Option<(i16, [Option<i32>; 3])>,
}

/// The event scripts' `stream` in the town ([`crate::stream`]): the disc it
/// plays from and DATA.BIN for its pictures (None: streams count as
/// played), the player while the call waits, and its frame this game frame.
#[derive(Default)]
pub struct TownStream {
    pub iso: Option<PathBuf>,
    pub data: Option<Arc<Archive>>,
    pub player: Option<StreamPlayer>,
    pub frame: Option<Frame>,
}

/// What the runtime answers the scripts with in The World, apart from the
/// world and the UI.
pub struct FieldState {
    /// This frame's pad (`ccSys.pad[0]`).
    pub pad: Pad,
    /// The frame, for the call log.
    pub frame: u64,
    /// `ccGame`: status 5, the scene `ChangeArea(0, lastTown)` set.
    pub game: CcGame,
    /// What the scripts asked of the sound.
    pub events: Vec<Event>,
    /// `ccMenu`'s fader.
    pub fade: ScFade,
    /// `eventMng.fadeNum`.
    pub fade_num: i32,
    /// `scFadeDef`, the screen's own fader (the flashes of
    /// `piros_colour`), on the font layer.
    pub fade_def: ScFade,
    /// A `piros_colour` running ([`crate::piros`]).
    pub piros: Option<crate::piros::Sequence>,
    /// Every host call with its frame.
    pub calls: Vec<(u64, String)>,
    /// `eventAreaInfo` by code.
    pub stories: HashMap<i16, StoryArea>,
    /// The `area` instruction's last request.
    pub area: Option<(i16, [Option<i32>; 3])>,
    /// `ChangeScene`, once asked: the event task sleeps in it.
    pub change: Option<SceneChange>,
    /// The windows opened, (event, message).
    pub shown: Vec<(u16, i16)>,
    /// `stream`'s player (the town's; the fields keep their own).
    pub stream: TownStream,
    /// `DispInfo`'s gate-address and desktop-item lines, as the desktop
    /// composes them (None: those windows open empty).
    pub announcements: Option<Arc<crate::story::Announcements>>,
    /// `ccSystem::SetFrameRate` from the scripts, over the town's own.
    pub frame_rate: Option<u32>,
}

impl FieldState {
    pub fn new(save: &SaveData, stories: HashMap<i16, StoryArea>) -> FieldState {
        let town = i32::from(save.u8(offset::LAST_TOWN) as i8);
        FieldState {
            pad: Pad::default(),
            frame: 0,
            game: {
                // ChangeRequest(5, 8)'s InitScene (every scene field and
                // *Prev -1), then ChangeArea(0, lastTown)'s ChangeScene.
                let mut g = CcGame {
                    status: STATUS_WORLD,
                    area: -1,
                    area_prev: -1,
                    server: -1,
                    town: -1,
                    field: -1,
                    dungeon: -1,
                    floor: -1,
                    block: -1,
                    server_prev: -1,
                    town_prev: -1,
                    field_prev: -1,
                    dungeon_prev: -1,
                    floor_prev: -1,
                    block_prev: -1,
                    ..CcGame::default()
                };
                let mut scratch = save.clone();
                g.change_scene(&mut scratch, [0, town, -1, -1, -1, -1]);
                g
            },
            events: Vec::new(),
            fade: ScFade::default(),
            fade_num: -1,
            fade_def: ScFade::default(),
            piros: None,
            calls: Vec::new(),
            stories,
            area: None,
            change: None,
            shown: Vec::new(),
            stream: TownStream::default(),
            announcements: None,
            frame_rate: None,
        }
    }

    pub(crate) fn log(&mut self, s: String) {
        self.calls.push((self.frame, s));
    }
}

/// The scripts' view of The World.
pub struct FieldHost<'a> {
    pub world: &'a mut World,
    pub ui: &'a mut FieldUi,
    pub st: &'a mut FieldState,
}

impl FieldHost<'_> {
    /// `charTbl[pc].name`, which `spcParam[pc].base.name` points at.
    fn member_name(&self, pc: i16) -> Vec<u8> {
        usize::try_from(pc).ok().and_then(|i| self.ui.texts().char_names.get(i)).cloned().unwrap_or_default()
    }

    /// A frame of `piros_colour`: its flashes and Piros's tint.
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
}

impl Host for FieldHost<'_> {
    fn save(&mut self) -> &mut SaveData {
        &mut self.world.state_mut().save
    }

    fn game(&self) -> Game {
        self.st.game.view()
    }

    fn frame_rate(&self) -> i32 {
        piney_world::FRAME_RATE as i32
    }

    fn party(&self) -> Party {
        let ids = self.world.party();
        Party { ids, num: ids.iter().filter(|&&i| i >= 0).count() as i32 }
    }

    /// The SPC manager holds Kite and the members events placed.
    fn spc_present(&self, code: i16) -> bool {
        self.world.char_pos(2, code).is_some()
    }

    fn field_menu(&self) -> i32 {
        self.ui.menu_type()
    }

    fn pad_pushed(&self) -> u32 {
        self.st.pad.push.bits()
    }

    fn marker(&self, marker: i16) -> Option<Marker> {
        self.world.marker(marker)
    }

    fn command_target(&self) -> Option<CharRef> {
        self.world.command_target_ref()
    }

    fn target_alive(&self, target: &CharRef) -> bool {
        self.world.target_alive(target)
    }

    fn trans(&mut self, ty: i16, code: i16, on: bool) {
        let done = self.world.set_trans(ty, code, on);
        self.st.log(format!("trans {ty} {code} {on}{}", if done { "" } else { " (not placed)" }));
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

    /// `near_marker`'s distance; a marker the town lacks reads an unset
    /// vector in the game, taken here as out of every bound.
    fn player_distance(&self, pos: Option<[f32; 4]>) -> f32 {
        let Some(p) = pos else { return f32::MAX };
        f32::from_bits(self.world.player_distance(p.map(f32::to_bits)))
    }

    fn story_area(&self, area: i16) -> Option<StoryArea> {
        self.st.stories.get(&area).copied()
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

    /// `DispInfo` (0x001b27b0): in The World it waits for the menus to be
    /// shut and shows the lines `Execute` composed: a member's address as
    /// the field UI words it, a gate address or a desktop item as the
    /// desktop does ([`crate::story::Announcements`]).
    fn announce(&mut self, a: Announce) {
        self.st.log(format!("announce {a:?}"));
        let save = self.world.state().clone();
        match a {
            Announce::Member { pc } => {
                let name = self.member_name(pc);
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
        self.st.events.push(Event::ChangeArea { area: a, town: n });
    }

    /// `ccGame::ChangeScene` (0x00167380): the scene moves on, recorded for
    /// the session, which sets the next scene up; `lastTown` follows the
    /// town, and `ChangeRequest(6, 7)` stops the task ([`Host::busy`]).
    fn change_scene(&mut self, area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16) {
        self.st.log(format!("change_scene {area} {town} {field} {dungeon} {floor} {block}"));
        let [a, t, fd, d, f, b] = [area, town, field, dungeon, floor, block].map(i32::from);
        self.st.game.change_scene(&mut self.world.state_mut().save, [a, t, fd, d, f, b]);
        self.st.change = Some(SceneChange { scene: [area, town, field, dungeon, floor, block], area: self.st.area });
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

    /// `compulsionGameOver` (0x00378c74): `ccThGameCtrl` zeroes it as it
    /// starts and only `ccMenuCtrl::DataDrainMenu` sets it, so in a town it
    /// stays 0.
    fn game_over(&self) -> bool {
        false
    }

    /// `ccClearGtHack` (0x001b77c0), from `ccStartThEvent`: `gtHackFlag`
    /// (0x00378a98) cleared, kept only on arriving from a town (areaPrev
    /// 0) in a field or a dungeon of type 8 or 9 - never in a town, where
    /// the session drops the flag as it sets the town up (the gate hack
    /// sets it afterwards, for the next scene); the call is logged.
    fn clear_gate_hack(&mut self) {
        self.st.log("clear_gate_hack".into());
    }

    /// `piros_colour` ([`crate::piros`]): on `eventStatus[1]`, with Piros
    /// in the town.
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

    /// `piros_colour` needs nothing before its wait; the town has no other
    /// wait with a start.
    fn begin(&mut self, w: Wait) {
        if w != Wait::PirosColour {
            piney_event::host::unported("begin");
        }
    }

    fn end(&mut self, w: Wait) {
        if w != Wait::PirosColour {
            piney_event::host::unported("end");
        }
    }

    /// `scene`'s `ChangeRequest(6, 7)`: asleep from then on. `stream`: the
    /// stream plays inside the call, a frame of it per game frame, until
    /// it returns. `piros_colour`: its breaths.
    fn busy(&mut self, w: Wait) -> bool {
        match w {
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
                let st = &mut *self.st;
                let Some(p) = &mut st.stream.player else { return false };
                match p.step(&st.pad, &mut st.events) {
                    Some(f) => {
                        st.stream.frame = Some(f);
                        true
                    }
                    None => {
                        st.stream.player = None;
                        false
                    }
                }
            }
            _ => false,
        }
    }

    /// `ccEventStream(num, 1)` (0x001b5670): the stream with its subtitles
    /// and the music around it, `ccGame` status 5 and this field for
    /// `ccSndStreamCtrl`. It plays inside the call, so the town's tasks do
    /// not run meanwhile. A stream that cannot start counts as played.
    fn stream(&mut self, num: i16) {
        self.st.log(format!("stream {num}"));
        let Some(iso) = self.st.stream.iso.clone() else { return };
        let save = self.world.state().save.clone();
        let game = piney_audio::stream::StreamGame { status: self.st.game.status, field: self.st.game.field };
        let data = self.st.stream.data.clone();
        let started = usize::try_from(num)
            .map_err(|_| format!("stream {num}"))
            .and_then(|n| StreamPlayer::event(&iso, data.as_deref(), n, &save, game, &mut self.st.events));
        match started {
            Ok(p) => self.st.stream.player = Some(p),
            Err(e) => tracing::warn!("events: {e}; counted as played"),
        }
    }

    // --- The field ------------------------------------------------------------------------

    fn camera(&mut self, c: CameraCommand) {
        let done = self.world.event_camera(c);
        self.st.log(format!("camera {c:?}{}", if done { "" } else { " (not ported)" }));
    }

    fn npc(&mut self, c: NpcCommand) {
        let done = self.world.npc_command(c);
        self.st.log(format!("npc {c:?}{}", if done { "" } else { " (not done)" }));
    }

    fn pc(&mut self, c: PcCommand) {
        let done = self.world.pc_command(c);
        self.st.log(format!("pc {c:?}{}", if done { "" } else { " (not done)" }));
    }

    fn remove(&mut self, ty: i16, code: i16) {
        self.st.log(format!("remove {ty} {code}"));
        self.world.remove(ty, code);
    }

    fn party_add(&mut self, pc: i16) {
        self.st.log(format!("party_add {pc}"));
        let slot = self.world.party_add(i32::from(pc));
        // ccEvent's party_add also sets the new slot's menu face.
        if slot >= 0 {
            let plcol = self.world.state().save.u8(offset::PLCOL) != 0;
            self.ui.set_menu_face(slot as usize, i32::from(pc), plcol);
        }
    }

    fn party_remove(&mut self, pc: i16) {
        self.st.log(format!("party_remove {pc}"));
        self.world.party_remove(i32::from(pc));
    }

    /// `ccEvent::MenuBan` / `MenuClr` (0x001b2460, 0x001b25c0): the menus
    /// forbidden and their panels and map hidden (ccMenu), the camera id
    /// kept, the party under manual control.
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
    /// not 0, category 0-9): `ccEvent::GetSpc(pc)`, the character the town
    /// built for that registry id, then `ccMenu->AddSpcItem(ch, category,
    /// id, num, 0)`. None built: false, and the item goes into the save.
    /// `item_add_menu` while playing (event 16's contest prize in Mac Anu,
    /// event 24's): the item-get menu (29), which gives the item, as in the
    /// fields.
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

    fn add_spc_item(&mut self, pc: i16, category: i16, id: i16, num: i16) -> bool {
        let code = i32::from(pc);
        if self.world.town_party().member(code).is_none() {
            return false;
        }
        self.st.log(format!("add_spc_item {pc} {category} {id} {num}"));
        let w = crate::world::ui_world(self.world, None, 0);
        let h = crate::world::handle(piney_world::entry::Kind::Spc, code);
        let (c, i, n) = (i32::from(category), i32::from(id), i32::from(num));
        self.ui.add_spc_item(&w, self.world.state_mut(), h, code, c, i, n);
        true
    }

    /// `virus_core` (case 124, main 0x001b06c8) runs `SimGenerateCode` only
    /// at level 1 (`ccEventFlagSet`: a volume's events brought forward, the
    /// port's story starts), never while playing; there its host is
    /// `start.rs`'s. Here, as `WORLD_MAN` would be left: the area the words
    /// make on the town's server, as Area Information shows it. (The field
    /// generator's `seed` and `randcnt` it also moves are left aside, as
    /// everywhere in the port's area words.)
    fn generate_area(&mut self, words: [Option<i32>; 3]) {
        self.st.log(format!("generate_area {words:?}"));
        if let [Some(a), Some(b), Some(c)] = words {
            let s = self.world.state().save.u8(offset::LAST_TOWN);
            let server = piney_world::area::SERVER_OF_TOWN.get(usize::from(s)).copied().unwrap_or(0);
            let save = self.world.state().clone();
            self.ui.set_area_words([a, b, c], server, &save);
        }
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

    fn target_forbid(&mut self, on: bool) {
        self.st.log(format!("target_forbid {on}"));
        self.ui.target_forbid(on);
    }

    /// `condition_fx_on` / `condition_fx_off`: `spcConditionEffectFlag`,
    /// which a town draws nothing by (no condition tints there) and which
    /// `ccThSpc`'s start sets again in the next area.
    fn condition_effect(&mut self, on: bool) {
        self.st.log(format!("condition_effect {on}"));
    }

    /// The first play pass's end (`ccThEvent`, main 0x001b5eb0): with
    /// `WORLD_MAN.hackFlag` 3 `ccMenu.interNoiz = 1`. `GO` sets `hackFlag`
    /// 2 for every area before its town, field or dungeon, 3 when the
    /// save's crisis byte (+0x6772) is set, and a town keeps that: in the
    /// crisis the town's menu has its noise.
    fn play_pass_done(&mut self) {
        let crisis = self.world.state().save.u8(offset::CRISIS) != 0;
        self.st.log(format!("play_pass_done{}", if crisis { " noise 1" } else { "" }));
        if crisis {
            self.ui.noise(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The event's `fade` and `fade_more` with the fader's four elements
    /// busy: Infection's find none (-1) and the `fade_more` after it does
    /// nothing; from Mutation on the fader is emptied first, and a
    /// `fade_more` with no element starts a fade from its alpha to 0.
    #[test]
    fn event_fades_by_volume() {
        let busy = || ScFade { elems: [FadeElem { status: 1, ..FadeElem::default() }; 4], ..ScFade::default() };
        let (mut inf, mut n) = (busy(), 0);
        inf.event(&mut n, 10, 0x80, false, false);
        assert_eq!(n, -1);
        let before = inf.elems;
        inf.event(&mut n, 10, 0x40, true, false);
        assert_eq!(inf.elems, before);

        let (mut later, mut n) = (busy(), 0);
        later.event(&mut n, 10, 0x80, false, true);
        assert_eq!(n, 0);
        assert_eq!((later.elems[0].col0, later.elems[0].col1), (0, 0x8000_0000));
        assert!(later.elems[1..].iter().all(|e| e.status == 0));
        later.event(&mut n, 5, 0x40, true, true);
        assert_eq!((later.elems[0].tcnt, later.elems[0].col0, later.elems[0].col1), (5, 0x8000_0000, 0x4000_0000));
        let mut n = -1;
        later.event(&mut n, 7, 0x40, true, true);
        assert_eq!(n, 0);
        assert_eq!((later.elems[0].tcnt, later.elems[0].col0, later.elems[0].col1), (7, 0x4000_0000, 0));
    }

    fn fixture() -> Option<String> {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/field_host_fixture.txt");
        std::fs::read_to_string(p).ok()
    }

    fn nums(s: &str) -> Vec<i64> {
        s.split_whitespace().map(|t| t.parse().unwrap()).collect()
    }

    fn fade_from(v: &[i64]) -> ScFade {
        let mut f = ScFade { off: v[0] as i32, ..ScFade::default() };
        for (i, e) in f.elems.iter_mut().enumerate() {
            let w = &v[1 + 7 * i..8 + 7 * i];
            *e = FadeElem {
                status: w[0] as i16,
                cnt: w[1] as i16,
                tcnt: w[2] as i16,
                tcnt_back: w[3] as i16,
                tcnt_hold: w[4] as i16,
                col0: w[5] as u32,
                col1: w[6] as u32,
            };
        }
        f
    }

    /// `EntryFade`, `ContinueFade` and `SendPacket`'s counting against the
    /// game's (tools/test_field_host.py): every fader after every call.
    #[test]
    fn the_fader_counts_as_the_game_does() {
        let Some(text) = fixture() else { return };
        let mut fade = ScFade::default();
        let mut n = 0;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("fade ") {
                fade = fade_from(&nums(rest));
                continue;
            }
            let Some(rest) = line.strip_prefix("op ") else { continue };
            let (op, want) = rest.split_once(" -> ").unwrap();
            let want = nums(want);
            let mut w = op.split_whitespace();
            let r = match w.next().unwrap() {
                "entry" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    i64::from(fade.entry(v[0] as i16, v[1] as u32, v[2] as u32))
                }
                "continue" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    fade.continue_fade(v[0] as i32, v[1] as i16, v[2] as u32);
                    0
                }
                _ => {
                    fade.count();
                    0
                }
            };
            assert_eq!(r, want[0], "{line}");
            assert_eq!(fade, fade_from(&want[1..]), "{line}");
            n += 1;
        }
        assert!(n > 500, "{n} calls");
    }

    /// `ccGame::ChangeScene` against the game's: every field of `ccGame`
    /// and `lastTown`.
    #[test]
    fn change_scene_as_the_game_does() {
        let Some(text) = fixture() else { return };
        let mut n = 0;
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("scene ") else { continue };
            let (args, want) = rest.split_once(" -> ").unwrap();
            let (a, want) = (nums(args), nums(want));
            let w = |k: usize| a[k] as i32;
            let mut g = CcGame {
                status: w(0),
                area: w(5),
                area_prev: w(6),
                server: w(7),
                town: w(8),
                field: w(9),
                dungeon: w(10),
                floor: w(11),
                block: w(12),
                server_prev: w(13),
                town_prev: w(14),
                field_prev: w(15),
                dungeon_prev: w(16),
                floor_prev: w(17),
                block_prev: w(18),
                in_battle: w(22),
                in_battle_cnt: w(23),
                in_battle_dist: w(24) as u32,
                cnt_stop: w(29),
            };
            let mut save = SaveData::new();
            save.set_u8(offset::LAST_TOWN, a[34] as u8);
            g.change_scene(&mut save, [35, 36, 37, 38, 39, 40].map(w));
            let got = [
                g.status,
                g.area,
                g.area_prev,
                g.server,
                g.town,
                g.field,
                g.dungeon,
                g.floor,
                g.block,
                g.server_prev,
                g.town_prev,
                g.field_prev,
                g.dungeon_prev,
                g.floor_prev,
                g.block_prev,
                g.in_battle,
                g.in_battle_cnt,
                g.in_battle_dist as i32,
                g.cnt_stop,
            ];
            let idx = [0, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 22, 23, 24, 29];
            let want_fields: Vec<i32> = idx.iter().map(|&k| want[k] as i32).collect();
            assert_eq!(got.to_vec(), want_fields, "{line}");
            assert_eq!(i64::from(save.u8(offset::LAST_TOWN) as i8), want[34], "{line}");
            // Nothing else of ccGame moves.
            for (k, (&x, &y)) in a[..34].iter().zip(&want[..34]).enumerate() {
                if !idx.contains(&k) {
                    assert_eq!(x, y, "ccGame word {k}: {line}");
                }
            }
            n += 1;
        }
        assert!(n >= 120, "{n} scenes");
    }
}
