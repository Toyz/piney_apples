//! Mode 6 outside the towns: `ccSetupGameCtrl` for a field (area 1) or a
//! dungeon (area 2), with the field's menus and HUD (`piney-fieldui`) over
//! `piney_world::field_world`. A frame runs the event task (`ccThEvent`,
//! [`crate::area_host`]), the world's tasks, then `ccThMenu`. The event task
//! arrives asleep in the `scene` that asked for this area and ends it before
//! `ccStartThEvent`; then the passes at 0 and 2, `rebootSpcManager`, and
//! `ccEnableThEvent(4)` at F0 (docs/engine/field-game.md).

use std::path::Path;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_event::vm::Vm;
use piney_fieldui::FieldUi;
use piney_input::Pad;
use piney_world::area::{Scene, WorldMan};
use piney_world::evarea::Kept;
use piney_world::field_world::{FieldWorld, Request};
use piney_world::party::Spcs;
use piney_world::story_map::StoryRequest;
use piney_world::talk::{self, TalkRequest};
use piney_world::{Phase, Request as GameRequest};

use crate::area_host::{self, AreaHost};
use crate::field_host::{FieldState, MENU_LAYER};
use crate::mode::{Event, Mode};
use crate::world::Setup;

mod ride;

/// Frames a set-up pass may take before the runtime gives up waiting.
const PASS_FRAMES: u32 = 10_000;

pub struct AreaMode {
    world: Box<FieldWorld>,
    ui: Box<FieldUi>,
    /// The event task.
    vm: Option<Vm>,
    /// What the scripts see besides the world (`ccGame`, the fader, the
    /// call log).
    st: FieldState,
    setup: Setup,
    pass_frames: u32,
    frames: u64,
    count: u32,
    events: Vec<Event>,
    /// A change of scene was asked for (raised once).
    leaving: bool,
    /// What `WORLD_MAN` keeps of the minimap (the map itself is the
    /// field's or the dungeon's), whether every task sleeps, and the fonts
    /// its labels are drawn with.
    map_st: piney_world::map::MapState,
    asleep: bool,
    /// A story map's message is up (area 43's `kiteSelfTalk`).
    story_message: bool,
    fonts: Option<piney_desktop::kanji::Fonts>,
    /// The scripts' cutscene streams (`stream`).
    stream: area_host::AreaStream,
    /// The item use the menu task is inside (`ccUseItemRequest`): the
    /// user's and the target's scene indices, for its steps.
    item_use: Option<(usize, usize)>,
    /// A menu's own screen over nothing (Data Drain's black, the movie's
    /// place): the world draws nothing (`piney_fieldui::Request::WorldHidden`).
    world_hidden: bool,
    /// Data Drain's movie (`ccThExecuteStream`, [`piney_fieldui::Request::DrainMovie`]),
    /// a step a frame over the area until it ends.
    drain_movie: Option<crate::stream::StreamPlayer>,
    /// Its `ccThDrainEnemy`, drawn into it.
    drain_enemy: Option<piney_world::foe::DrainEnemy>,
    /// The members' item uses being played ([`MemberItem`]).
    member_items: Vec<MemberItem>,
    /// The movie is `StreamMenu`'s (menu 74), with its `ccThStrParty`.
    stream_menu: bool,
    /// A boss's stage fader (`ccBoss::InitStageEffect`: `scStageFade` on
    /// its `stageLayer`, priority 2, between the stage and the
    /// characters), and `m_stageEffId`.
    stage_fade: piney_demo::fade::ScFade,
    stage_fade_id: i32,
    str_party: Option<piney_world::foe::StrParty>,
    /// The game over past `ccThGameCtrl`'s signal (`ccThGameOver`).
    game_over: Option<crate::gameover::GameOverTask>,
    /// The riding Grunty's sequence (`ride.rs`).
    ride: ride::RideSeq,
    /// A Fairy's Orb's `WORLD_MAN::ShowMap` still to finish: called once a
    /// frame, before the menu task, until it is done.
    map_showing: bool,
}

/// A battle character as the HUD reads it (`ccChar`, its base and its
/// `personality`): its handle (`1 << 24 | id` a party member, `2 << 24 |
/// index` anything else), the base's type, id, name, size, HP, SP,
/// conditions, and what the field projects of it.
fn char_info(w: &FieldWorld, who: usize, names: &dyn Fn(i32) -> Vec<u8>) -> Option<piney_fieldui::CharInfo> {
    let c = w.combat();
    let ch = c.scene.chars.get(who)?;
    let base = ch.base();
    let pc = ch.is_pc();
    let handle = if pc { (1 << 24) | (ch.id() as u16 as u32) } else { (2 << 24) | who as u32 };
    let name = if pc {
        names(i32::from(ch.id()))
    } else {
        base.label.map(piney_data::tables::sjis::encode).unwrap_or_default()
    };
    let g = w.hud_geometry(who).unwrap_or_default();
    let (ability, elements) = match ch.temp_time() {
        Some((temp, _)) => (
            std::array::from_fn(|i| temp.get(i).copied().unwrap_or(0)),
            std::array::from_fn(|i| temp.get(8 + i).copied().unwrap_or(0)),
        ),
        None => ([0; 8], [0; 6]),
    };
    let f = f32::from_bits;
    Some(piney_fieldui::CharInfo {
        handle,
        types: base.ty as u32,
        id: base.id,
        object_size: c
            .foes
            .get(who)
            .and_then(|f| f.as_ref())
            .map(|e| &e.ent)
            .or(c.ctrl.entry_obj(who).map(|o| &o.ent))
            .map_or(0, |ent| piney_battle::drain::check_object_size(&c.data.t, ent.ty, ent.id)),
        name,
        height: f(base.height),
        hp: ch.hp,
        sp: ch.sp,
        max_hp: ch.max_hp,
        max_sp: ch.max_sp,
        condition: ch.cond.v,
        speed_value: f(ch.cond.speed_value),
        ability,
        elements,
        cmnd_dist: f(g.dist),
        // A box's entry param[1] (+0x14c): its item, -1 for the area's draw.
        item: c.ctrl.entry_obj(who).map_or(-1, |o| o.ent.param[1]),
        // +0x150, its param[2]: the trap (ItemBoxMenu); +0x7c skillID, a
        // trapped box's trap skill (TrapBoxMenu).
        trap: c.ctrl.entry_obj(who).map_or(-1, |o| o.ent.param[2]),
        skill: ch.skill_id,
        // +0x1d4 actNum: the spring's state (FountainMenu3).
        act_num: c.ctrl.entry_obj(who).map_or(0, |o| o.act_num),
        attribute: if ch.is_foe() { piney_battle::chara::check_char_attribute(ch, 0) } else { -1 },
        tag: g.tag,
        bar_res: g.bar_res,
        bar: g.bar,
        pp: ch.foe_state().map_or(0, |fs| fs.pp_count),
        arrow: g.arrow,
        width: f(base.width),
        in_view: g.in_view,
        pos_p: [f(ch.pos_p[0]), f(ch.pos_p[1]), f(ch.pos_p[2])],
        chat: [0; 6],
        // +0xee actNum: a member's CheckChangeEquip reads it.
        act: if pc { ch.spc_char.act_num } else { 0 },
    })
}

/// The scene index a HUD handle names.
fn handle_index(w: &FieldWorld, h: u32) -> Option<usize> {
    match h >> 24 {
        1 => w.combat().who((h & 0xffff) as u16 as i16 as i32),
        2 => Some((h & 0xff_ffff) as usize),
        _ => None,
    }
}

/// A member's item use (its AI's `ccUseItemRequest`) being played: the
/// steps its rules left, and the frames left of a wait.
struct MemberItem {
    user: usize,
    target: usize,
    steps: std::collections::VecDeque<piney_battle::item::Step>,
    wait: u32,
}

impl AreaMode {
    /// The members' item uses: the ones their AI made this frame, and each
    /// one's steps played on as the menu plays Kite's - the world's through
    /// `item_step`, the sounds, a menu it opens (the ocarina's gate-out,
    /// 86), a wait's frames. The rest is the menu's presentation of a use
    /// made from it, which a member's use does not have.
    fn member_item_steps(&mut self) {
        use piney_battle::item::Step;
        for (user, target, steps) in self.world.take_member_items() {
            self.member_items.push(MemberItem { user, target, steps: steps.into(), wait: 0 });
        }
        let mut items = std::mem::take(&mut self.member_items);
        for it in &mut items {
            if it.wait > 0 {
                it.wait -= 1;
                continue;
            }
            while let Some(step) = it.steps.pop_front() {
                match step {
                    Step::Frames(n) => {
                        it.wait = n;
                        break;
                    }
                    Step::Se(n) => self.events.push(Event::Se(n)),
                    Step::SeNote(se, note) => {
                        self.events.push(Event::SeNote { n: se.max(0) as usize, note: note as i8 })
                    }
                    Step::ChangeMenu(n) => self.ui.open_menu(n as i16),
                    step => {
                        if !self.world.item_step(&step, it.user, it.target) {
                            self.st.log(format!("member item step not carried out: {step:?}"));
                        }
                    }
                }
            }
        }
        items.retain(|it| it.wait > 0 || !it.steps.is_empty());
        self.member_items = items;
    }

    /// `ccSetupGameCtrl` in `scene` on `state`; `faded` when the scene that
    /// asked has drawn its fade out already.
    #[allow(clippy::too_many_arguments)]
    pub fn enter(
        iso: &Path,
        archive: Arc<Archive>,
        state: SaveState,
        vm: Option<Vm>,
        scene: Scene,
        world_man: WorldMan,
        kept: Option<Kept>,
        faded: bool,
        spcs: Spcs,
    ) -> Result<AreaMode, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let mut ui = FieldUi::new(&mut disc, archive.clone()).map_err(|e| format!("the field's menus: {e}"))?;
        // The menu faces: party slot 0 Kite with the bracelet's colours or
        // without, then the members the party brought.
        let plcol = state.save.u8(offset::PLCOL) != 0;
        for (slot, &id) in spcs.party().iter().enumerate() {
            if id >= 0 {
                ui.set_menu_face(slot, id, plcol);
            }
        }
        // Area Information (menu 87): the words WORLD_MAN holds.
        ui.set_area_words(world_man.words, scene.server, &state);
        let stories = if vm.is_some() {
            crate::story::from_tables(piney_data::area::AreaTables::of(disc.volume().map_err(|e| e.to_string())?))
        } else {
            Default::default()
        };
        let mut st = FieldState::new(&state.save, stories);
        st.game = area_host::game_of(&scene);
        st.announcements = crate::desktop::announcements(iso);
        let mut world = FieldWorld::enter(&mut disc, archive.clone(), state, scene, world_man, kept, faded, spcs)
            .map_err(|e| format!("area {} field {} dungeon {}: {e}", scene.area, scene.field, scene.dungeon))?;
        // WORLD::Init's or the DUNGEON constructor's map, and the modes
        // ccThGameCtrl's set-up reads.
        let volume = world.volume();
        if let Err(e) = piney_world::map::setup_area(world.place_mut(), &archive, volume, &scene) {
            tracing::warn!("the area's map: {e}");
        }
        // ccThEffect's and ccThParticle's set-up.
        match crate::fx::AreaFx::new(&archive, disc.volume().map_err(|e| e.to_string())?) {
            Ok(fx) => world.set_fx(Box::new(fx)),
            Err(e) => tracing::warn!("the field's effects: {e}"),
        }
        let map_st = piney_world::map::MapState::new(world.state());
        let fonts = disc.volume().ok().and_then(|v| piney_desktop::assets::read_fonts(v, &archive).ok());
        let setup = if vm.is_some() { Setup::Before } else { Setup::Off };
        world.set_loading(vm.is_some());
        // The sound driver reads `saveData.voice` and `parodyFlag` each time
        // it plays a voice; the port's keeps a copy, so hand it the save's
        // (a story start enters here without the desktop or the town).
        let events = {
            let s = &world.state().save;
            vec![crate::mode::voice_options(s), Event::GameArea(world.scene().area)]
        };
        Ok(AreaMode {
            world: Box::new(world),
            ui: Box::new(ui),
            vm,
            st,
            setup,
            pass_frames: 0,
            frames: 0,
            count: 0,
            events,
            leaving: false,
            map_st,
            asleep: false,
            story_message: false,
            item_use: None,
            world_hidden: false,
            drain_movie: None,
            drain_enemy: None,
            member_items: Vec::new(),
            stream_menu: false,
            str_party: None,
            stage_fade: piney_demo::fade::ScFade::default(),
            stage_fade_id: -1,
            game_over: None,
            ride: ride::RideSeq::default(),
            map_showing: false,
            fonts,
            stream: area_host::AreaStream { iso: iso.to_path_buf(), data: Some(archive.clone()), ..Default::default() },
        })
    }

    /// Leave: the save, the event task, the scene, `WORLD_MAN`'s area, the
    /// dungeon (which lives on for its next room or floor) or area 15's
    /// map (for its next block), and the party.
    pub fn leave(mut self) -> (SaveState, Option<Vm>, Scene, WorldMan, Option<Kept>, Spcs) {
        // ccSetupGameCtrl's ccStoreSpcCondition, for the next scene's party.
        self.world.store_conditions();
        let (state, scene, wm) = (self.world.state_out(), self.world.scene(), *self.world.world_man());
        let spcs = self.world.spcs().clone();
        (state, self.vm, scene, wm, self.world.into_kept(), spcs)
    }

    /// The area the scripts sent the player to, once they did.
    pub fn scene_change(&self) -> Option<&crate::field_host::SceneChange> {
        self.st.change.as_ref()
    }

    /// Whether a menu's movie plays (a drain, `StreamMenu`).
    #[cfg(test)]
    pub fn movie_playing(&self) -> bool {
        self.drain_movie.is_some()
    }

    /// The field UI, for tests.
    #[allow(dead_code)]
    pub fn ui(&self) -> &FieldUi {
        &self.ui
    }

    /// The world, for tests.
    #[allow(dead_code)]
    pub fn world(&self) -> &FieldWorld {
        &self.world
    }

    /// The debug console's `heal`.
    pub fn heal_party(&mut self) -> usize {
        self.world.heal_party()
    }

    /// The console's god: the fallen members got up.
    pub fn revive_party(&mut self) {
        self.world.revive_party();
    }

    /// The console's `kill`: each enemy standing takes `EntryAffect(1,
    /// 9999)` from Kite, as a hit that fells it (its exp and drops as the
    /// game gives them). The number hit.
    pub fn kill_enemies(&mut self) -> usize {
        let c = self.world.combat();
        let Some(k) = c.kite else { return 0 };
        let foes: Vec<usize> = c.enemies().into_iter().filter(|&e| !c.dead(e)).collect();
        for &e in &foes {
            self.world.entry_affect(e, Some(k), 1, [9999, 0, 0]);
        }
        foes.len()
    }

    /// The console's `exp N`: `n` experience to each member in the field,
    /// Kite too; `CheckLevelUp` makes each 1000 a level on the member's next
    /// frame, as a fight's exp does. The number given.
    pub fn give_exp(&mut self, n: i16) -> usize {
        let c = self.world.combat_mut();
        let who: Vec<usize> = c.members.iter().map(|&(_, k)| k).collect();
        let mut given = 0;
        for k in who {
            if let Some(p) = c.scene.chars[k].spc_mut() {
                p.base.exp = p.base.exp.saturating_add(n);
                given += 1;
            }
        }
        given
    }

    /// The console's `protect`: each enemy's protect broken for 60 seconds
    /// (`ppCount` 1800 frames), so Data Drain takes it. The number broken.
    pub fn break_protects(&mut self) -> usize {
        use piney_battle::event::{Event as Rule, Who};
        use piney_world::combat::Show;
        let c = self.world.combat_mut();
        let mut n = 0;
        for e in c.enemies() {
            if let Some(f) = c.scene.chars[e].foe_state_mut() {
                f.pp_count = 1800;
                f.pp = 0;
                n += 1;
                // What a breaking hit raises (`CalcBattleDamage`): the
                // effect, the sound and the protect marks.
                c.shows.push(Show::Rule(Rule::Protect { on: Who::Char(e), broken: 0, kind: -1 }));
                c.shows.push(Show::Rule(Rule::SetProtect { state: 0, on: Who::Char(e) }));
            }
        }
        n
    }

    /// The save, for the main loop's play time.
    pub fn save_mut(&mut self) -> &mut piney_data::save::SaveData {
        &mut self.world.state_mut().save
    }

    /// The main loop's `ccAddPlayTime` on the scene this area holds.
    pub fn add_play_time(&mut self, rate: i32) {
        self.world.add_play_time(rate);
    }

    /// The field's world to change (the console's `invite_party`).
    pub fn world_mut(&mut self) -> &mut FieldWorld {
        &mut self.world
    }

    /// Party slot `slot`'s menu face for member `id` (the console's
    /// `invite_party`, as the Party menu sets it).
    pub fn set_menu_face(&mut self, slot: usize, id: i32) {
        let plcol = self.world.state().save.u8(offset::PLCOL) != 0;
        self.ui.set_menu_face(slot, id, plcol);
    }

    pub fn ui_mut(&mut self) -> &mut FieldUi {
        &mut self.ui
    }

    /// The scripts' host over this area, lent to `f` (for tests).
    #[cfg(test)]
    pub fn with_host<R>(&mut self, f: impl FnOnce(&mut AreaHost) -> R) -> R {
        let mut h = AreaHost {
            world: &mut self.world,
            ui: &mut self.ui,
            st: &mut self.st,
            teach: [false; 3],
            reset: std::cell::Cell::new(false),
            stream: &mut self.stream,
        };
        f(&mut h)
    }

    /// The gate hack as the town left it (`gtHackFlag`, `ccGame.setupMode`),
    /// before the set-up runs.
    pub fn set_gate_hack(&mut self, flag: bool, setup_mode: bool) {
        self.world.set_gate_hack(flag, setup_mode);
    }

    /// `ccSetupGameCtrl` after a gate hack (`setupMode` 1): stream 107
    /// through `ccRequestLoadStreamGateHack` while the field's files load
    /// (`ccLoadResourceFL`), then `ResetPause` and the wait for the call to
    /// return; the set-up goes on after it. The port loads nothing then, so
    /// the Chaos Gate's loop (`str7300`, which the reader paused before)
    /// plays once through before `ResetPause`; the game holds it as long as
    /// the field's files take to read.
    fn gate_stream(&mut self, pad: &Pad) {
        if let Some(g) = self.world.take_gate_stream() {
            let save = self.world.state_out();
            let game = piney_audio::stream::StreamGame { status: crate::field_host::STATUS_WORLD, field: g.field };
            let iso = self.stream.iso.clone();
            match crate::stream::StreamPlayer::gate_hack(&iso, g.town, g.field, g.crisis, &save, game, &mut self.events)
            {
                Ok(p) => {
                    self.st.log(format!("gate hack stream {} {}", g.town, g.field));
                    self.stream.player = Some(p);
                }
                Err(e) => {
                    tracing::warn!("{e}; the set-up goes on");
                    self.world.gate_stream_done();
                    return;
                }
            }
        }
        if !self.world.gate_streaming() {
            return;
        }
        let Some(p) = &mut self.stream.player else {
            self.world.gate_stream_done();
            return;
        };
        // The loop's last frame before it rewinds (its frames run 0 to
        // frameEnd - 1 as it plays from memory).
        if let Some((now, end)) = p.at_pause()
            && now + 1 >= end
        {
            p.reset_pause();
        }
        match p.step(pad, &mut self.events) {
            Some(f) => self.stream.frame = Some(f),
            None => {
                self.world.set_rand(p.rand());
                self.stream.player = None;
                self.world.gate_stream_done();
                self.st.log("gate hack stream done".into());
            }
        }
    }

    /// What `WORLD_MAN` keeps of the minimap and the frame last drawn, for
    /// tests.
    #[allow(dead_code)]
    pub fn map_state(&self) -> &piney_world::map::MapState {
        &self.map_st
    }

    /// The event task, while the area holds it.
    #[cfg(test)]
    pub fn vm(&self) -> Option<&Vm> {
        self.vm.as_ref()
    }

    /// [`Self::vm`], to change.
    #[cfg(test)]
    pub fn vm_mut(&mut self) -> Option<&mut Vm> {
        self.vm.as_mut()
    }

    /// Every host call the scripts made here, with its frame.
    #[allow(dead_code)]
    pub fn calls(&self) -> &[(u64, String)] {
        &self.st.calls
    }

    /// Whether the scripts play a stream.
    #[allow(dead_code)]
    pub fn streaming(&self) -> bool {
        self.stream.player.is_some()
    }

    /// The scene a stream playing here plays (or is about to), for tests.
    #[allow(dead_code)]
    pub fn stream_scene(&self) -> Option<String> {
        self.stream.player.as_ref().and_then(crate::stream::StreamPlayer::entry_name)
    }

    /// One frame of the event task (`ccThEvent`, priority 32), before the
    /// world's tasks; then `ccSetupGameCtrl`'s side of the passes.
    fn event_frame(&mut self, pad: &Pad) {
        let Some(vm) = &mut self.vm else { return };
        if self.setup == Setup::Left {
            return;
        }
        if self.setup == Setup::Start && self.world.phase() == Phase::Play(0) {
            vm.enable(4);
            self.setup = Setup::Play;
        }
        // The passes' windows are the set-up screen's.
        self.ui.set_setup(matches!(self.setup, Setup::Pass(_)));
        vm.set_operate_set(self.world.state().operate_set);
        let teach = area_host::teach_input(pad, self.world.camera().scheme.ctrl_type);
        let reset = {
            let reset = std::cell::Cell::new(false);
            let mut h = AreaHost {
                world: &mut self.world,
                ui: &mut self.ui,
                st: &mut self.st,
                teach,
                reset,
                stream: &mut self.stream,
            };
            vm.frame(&mut h);
            h.reset.get()
        };
        // teach_camera3's reset button: cpCtrl 7, in this frame before the
        // camera's task.
        if reset {
            self.world.teach_camera(3);
        }
        let s = self.world.state_mut();
        s.operate = vm.operate();
        s.operate_set = vm.operate_set();
        self.world.set_event_targets(&vm.mng.targets);
        self.events.append(&mut self.st.events);
        if self.st.change.is_some() {
            // ChangeRequest(6, 7): the task asleep inside the instruction,
            // every task with it.
            vm.disable();
            self.setup = Setup::Left;
            self.world.set_asleep(true);
            return;
        }
        match self.setup {
            // The fade and hold are over: ccInitRand, ccStartThEvent (the
            // closed events done), WORLD_MAN::SetEventData,
            // ccEnableThEvent(0).
            Setup::Before if self.world.phase() == Phase::Hold(piney_world::HOLD_FRAMES - 1) => {
                let (teach, reset) = ([false; 3], std::cell::Cell::new(false));
                let mut h = AreaHost {
                    world: &mut self.world,
                    ui: &mut self.ui,
                    st: &mut self.st,
                    teach,
                    reset,
                    stream: &mut self.stream,
                };
                vm.start_thread(&mut h);
                // WORLD_MAN::SetEventData: a story dungeon's rooms and
                // positions as the event manager's points and positions.
                if let piney_world::field_world::Place::Dungeon(d) = self.world.place() {
                    let area = self.world.scene().area;
                    let ed = piney_world::dungeon_area::set_event_data(d.tables, d.event_area, area, 0);
                    for [floor, block, num] in ed.points {
                        vm.mng.set_event_point(floor, block, num);
                    }
                    for ([floor, block, num], dirc, p) in ed.positions {
                        let pos = [f32::from_bits(p[0]), f32::from_bits(p[1]), f32::from_bits(p[2]), 1.0];
                        vm.mng.set_event_pos(floor, block, num, f32::from_bits(dirc), pos);
                    }
                }
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
                    tracing::warn!("the area's set-up pass at phase {p} did not end in {PASS_FRAMES} frames; going on",);
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
                    // ccEntryEventMng: the entries the passes registered
                    // (party members are the registry's): the magic
                    // portals (entry_mc) and the enemies (entry 5, 6) go to
                    // the entry control when the tasks start.
                    let mc: Vec<[i16; 4]> = vm.mng.entries_mc.iter().copied().filter(|e| e[0] >= 0).collect();
                    let en: Vec<[i16; 4]> = vm.mng.entries.iter().copied().filter(|e| e[0] >= 3).collect();
                    for e in &mc {
                        self.st.log(format!("entry_mc {} {} {} {}", e[0], e[1], e[2], e[3]));
                    }
                    for e in &en {
                        self.st.log(format!("entry {} {} {} {}", e[0], e[1], e[2], e[3]));
                    }
                    let positions = vm
                        .mng
                        .positions
                        .iter()
                        .map(|p| piney_battle::entry::EvPos {
                            floor: p.floor,
                            block: p.block,
                            num: p.num,
                            dirc: p.dirc.to_bits(),
                            pos: p.pos.map(f32::to_bits),
                        })
                        .collect();
                    self.world.set_event_entries(mc, en, positions);
                    // ccRegisterEventMng: the party characters' entries.
                    let spc: Vec<[i16; 4]> =
                        vm.mng.entries.iter().copied().filter(|e| (0..=2).contains(&e[0])).collect();
                    for e in &spc {
                        self.st.log(format!("entry {} {} {} {}", e[0], e[1], e[2], e[3]));
                    }
                    self.world.set_spc_entries(spc);
                    self.world.set_loading(false);
                    self.setup = Setup::Start;
                }
            }
            _ => {}
        }
    }

    /// The command target's (kind, code) of a scene index.
    fn target_of(&self, i: usize) -> (piney_world::entry::Kind, i32) {
        let c = self.world.combat();
        let ch = &c.scene.chars[i];
        if ch.is_pc() {
            (piney_world::entry::Kind::Spc, i32::from(ch.id()))
        } else if ch.is_foe() {
            (piney_world::entry::Kind::Enemy, i as i32)
        } else {
            (piney_world::entry::Kind::Gimmick, i as i32)
        }
    }

    /// What `ccMenuCtrl` reads of the field this frame
    /// ([`piney_fieldui::World`]): the party's panels from the battle's
    /// characters (Kite named by the save's `plName`, the others by
    /// `charTbl`), the command target and the one before it,
    /// `cmndSortRoot`'s chain, the three command lists, `game.inBattle`, the
    /// player's skill and attack.
    fn hud_world(&self) -> piney_fieldui::World {
        let save = self.world.state();
        let s = &save.save;
        let names = |id: i32| if id == 0 { s.name().to_vec() } else { self.world.char_name(id) };
        let c = self.world.combat();
        let ids = self.world.party();
        let mut party: [Option<piney_fieldui::CharInfo>; 3] = [None, None, None];
        let mut party_id = [-1i32; 3];
        for (slot, &id) in ids.iter().enumerate() {
            if id < 0 {
                continue;
            }
            party_id[slot] = id;
            if let Some(i) = c.who(id) {
                party[slot] = char_info(&self.world, i, &names);
            }
        }
        let party_handles: Vec<u32> = party.iter().flatten().map(|c| c.handle).collect();
        let info = |i: usize| char_info(&self.world, i, &names);
        let t = self.world.targeting();
        let target = self.world.target_index(t.target).filter(|&i| self.world.listed(i)).and_then(info);
        let target_prev = self.world.target_index(t.prev).filter(|&i| self.world.listed(i)).and_then(info);
        let sorted =
            t.sorted.iter().filter_map(|x| self.world.target_index(Some((x.0, x.1)))).filter_map(info).collect();
        let chain = |l: &[usize]| l.iter().filter_map(|&i| info(i)).collect::<Vec<_>>();
        let scene = self.world.scene();
        let vm = self.vm.as_ref();
        let kite = c.kite;
        let player_skill =
            kite.map_or(0, |k| c.skills.borrow().check(&c.data.t, &c.scene, k, c.scene.chars[k].spc_char.act_num));
        let wm = self.world.world_man();
        piney_fieldui::World {
            game: piney_fieldui::Game {
                status: 5,
                area: scene.area,
                town: scene.town,
                field: scene.field,
                dungeon: scene.dungeon,
                floor: scene.floor,
                server: scene.server,
                in_battle: c.battle.in_battle,
                in_battle_cnt: c.battle.cnt,
                game_cnt: [scene.game_cnt[0], scene.game_cnt[1], scene.game_cnt[2]],
                word_level: wm.area_level,
                bgnum: wm.weather as i32,
                ..Default::default()
            },
            party,
            party_id,
            target,
            target_prev,
            sorted,
            player_dead: kite.is_some_and(|k| c.dead(k)),
            player_skill,
            player_attacking: kite.is_some_and(|k| c.scene.chars[k].spc_char.attack != 0),
            pc_chain: chain(&c.scene.pc_list),
            ene_chain: chain(&c.scene.ene_list),
            obj_chain: chain(&c.scene.obj_list),
            area_codes: vm.map_or([(-1, -1); 16], |vm| vm.mng.area_codes),
            event_status: vm.map_or(0, |vm| vm.mng.status),
            boss_entry: -1,
            field_attr: wm.field_attr(),
            event_area: wm.event,
            area_word: (wm.area_level, wm.item_ofs),
            party_annihilated: c.annihilated(),
            game_over: self.world.compulsion_game_over,
            // pgRideFlag as the menu task sees it, ccPuccigusoStart's fades,
            // ccPgAdultCheck(game.server, k).
            pg_ride: self.ride.seen,
            pg_starting: self.ride.start.is_some(),
            map_showing: self.map_showing,
            pg_adult: self.world.pg_adult(),
            // The balloons up, and the party's (a menu opens the player's
            // line in the frame it is drawn: CHAT's orders).
            chat_at: self
                .ui
                .chat_speakers()
                .into_iter()
                .chain(party_handles)
                .collect::<std::collections::BTreeSet<u32>>()
                .into_iter()
                .filter_map(|h| {
                    let (listed, at) = self.world.chat_point(handle_index(&self.world, h)?)?;
                    Some(piney_fieldui::chat_msg::ChatAt { who: h, listed, at })
                })
                .collect(),
            ..Default::default()
        }
    }

    /// `ccMenuCtrl::GateoutMenu`'s end (menu 10, confirmed): back to the
    /// town. The field's menus raise it; `--press F:gateout` too.
    /// `ccSeOn3D` and the animation notes' `ccSeSetParamEnemy` /
    /// `ccSeSetParamSPC` the battle's tasks called this frame, and the
    /// effects' sounds, as [`Event::Se3d`] in order.
    fn battle_sounds(&mut self, shows: &[piney_world::combat::Show]) {
        use piney_audio::se3d;
        use piney_world::combat::Show;
        let w = &self.world;
        let v = w.volume();
        let c = w.combat();
        let cam = w.camera().active();
        let ear = Some(se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
        let pos = |i: usize| c.scene.chars.get(i).map_or([0, 0, 0, 0x3f80_0000], |ch| ch.pos);
        let spc = |i: usize, param: u32| {
            let ch = c.scene.chars.get(i)?;
            let hit = c.crew.spc.get(&i).map_or(0, |s| s.hit_attribute);
            se3d::spc_note(v, param, ch.id(), hit)
        };
        let mut out = Vec::new();
        for s in shows {
            let (se, at) = match s {
                Show::Enemy(e, piney_battle::enemy_motion::Call::Sound { param, category }) => {
                    (se3d::enemy_note(v, *param, *category), pos(*e))
                }
                Show::Enemy(_, piney_battle::enemy_motion::Call::Sound3d { id, pos: p }) => {
                    (usize::try_from(*id).ok().map(|code| se3d::NoteSe { code, note: None }), *p)
                }
                Show::Kite(k, piney_battle::kite::Out::Sound { param }) => (spc(*k, *param), pos(*k)),
                Show::Member(m, piney_battle::fellow::Out::Se { param }) => (spc(*m, *param), pos(*m)),
                Show::Entry(piney_battle::entry::Out::Se { se, pos: p }) => {
                    (usize::try_from(*se).ok().map(|code| se3d::NoteSe { code, note: None }), *p)
                }
                // The area's last portal opened: ccStartThread(ccThDfComp).
                Show::Entry(piney_battle::entry::Out::AreaCleared) => {
                    let sc = self.world.scene();
                    self.ui.df_comp(sc.area, self.world.world_man().field_type as i32, sc.dungeon);
                    (None, [0; 4])
                }
                // ccSeOn(se): the spring's (215, 104), the boxes' (104)
                // and the virus crystal's (75) sounds, with no place.
                Show::Entry(piney_battle::entry::Out::Se2d(se)) => {
                    out.push(Event::Se(*se));
                    (None, [0; 4])
                }
                // A Grunty food taken: ccSeOn3DNote(179, pos, 70).
                Show::Entry(piney_battle::entry::Out::SeNote { se, pos: p, note }) => {
                    (usize::try_from(*se).ok().map(|code| se3d::NoteSe { code, note: Some(*note as i8) }), *p)
                }
                // ccVoicePgFood: a Grunty food calls out, out of battle
                // (game.inBattle 0).
                Show::Entry(piney_battle::entry::Out::FoodVoice { kind }) => {
                    if c.battle.in_battle == 0 {
                        out.push(Event::Voice { event: piney_data::sound::voice::FOOD_GROUP, msg: *kind });
                    }
                    (None, [0; 4])
                }
                // The boss's ccSeOn3D, ccSeOn3DNote and ccSeOn.
                Show::Boss(_, piney_battle::boss::Out::Se3d { se, pos: p }) => {
                    (usize::try_from(*se).ok().map(|code| se3d::NoteSe { code, note: None }), *p)
                }
                Show::Boss(_, piney_battle::boss::Out::Se3dNote { se, pos: p, note }) => {
                    (usize::try_from(*se).ok().map(|code| se3d::NoteSe { code, note: Some(*note as i8) }), *p)
                }
                Show::Boss(_, piney_battle::boss::Out::Se { se }) => {
                    out.push(Event::Se(*se));
                    (None, [0; 4])
                }
                // Fidchell's prediction: ccEvVoiceStop, then
                // ccEvVoiceRequest(-40, n), a field voice group's line.
                Show::Boss(_, piney_battle::boss::Out::Fidchell(piney_battle::boss::fidchell::Pic::Voice(msg))) => {
                    out.push(Event::Voice { event: piney_battle::boss::fidchell::PRED_VOICE_EVENT, msg: *msg });
                    (None, [0; 4])
                }
                Show::Boss(_, piney_battle::boss::Out::Fidchell(piney_battle::boss::fidchell::Pic::VoiceStop)) => {
                    out.push(Event::VoiceStop);
                    (None, [0; 4])
                }
                // Magus's ccSeOnNote, and its grow's ccSeOn3DLoop (played
                // once here; its ccSeOffLoop has nothing to stop).
                Show::Boss(_, piney_battle::boss::Out::SeNote { se, note }) => {
                    out.push(Event::SeNote { n: (*se).max(0) as usize, note: *note as i8 });
                    (None, [0; 4])
                }
                Show::Boss(_, piney_battle::boss::Out::SeLoop { se, pos: Some(p) }) => {
                    (usize::try_from(*se).ok().map(|code| se3d::NoteSe { code, note: None }), *p)
                }
                // The riding Grunty's notes: ccSeSetParamInu(param, pcgs).
                Show::Ride(piney_battle::ride::Out::Sound { param, attribute }) => {
                    let at = c.ride.obj.as_ref().map_or([0, 0, 0, 0x3f80_0000], |o| o.ride.pos);
                    (se3d::inu_note(v, *param, *attribute), at)
                }
                // The Administrator's act -5: ccSeOn(217).
                Show::Npc(_, piney_world::merchant::SysopEvent::Vanish) => {
                    out.push(Event::Se(217));
                    (None, [0; 4])
                }
                Show::SkillSounds { caster, target_pos, heal, notes } => {
                    if *heal {
                        out.push(Event::Se3d { n: 75, pos: *target_pos, note: None, ear });
                    }
                    for &v in notes {
                        if let Some(se) = caster.and_then(|c| spc(c, v)) {
                            out.push(Event::Se3d {
                                n: se.code,
                                pos: caster.map_or(*target_pos, pos),
                                note: se.note,
                                ear,
                            });
                        }
                    }
                    (None, [0; 4])
                }
                // ccWordsPlay(sid, ch): the caster names its skill; the
                // sound task keeps it for Kite and the party, outside the
                // events (eventMng +0x78c, puppetShow), and plays it with
                // the skill's type bit (ccGetSkillParam(sid)+0x2c). From
                // Mutation on none while a Grunty is out (ccSnd +0x139).
                Show::Words { who, sid } => {
                    if let Some(ch) = c.scene.chars.get(*who).filter(|_| !w.voices_off()) {
                        let base = ch.base();
                        // skillVoicePlay reads saveData.voice as it plays: the
                        // options as the save stands go with the words.
                        out.push(crate::mode::voice_options(&w.state().save));
                        out.push(Event::SkillWords {
                            event_running: w.camera().puppet_show,
                            char_type: base.ty as u32,
                            char_id: base.id,
                            sid: *sid,
                            type_bit: c.data.t.skill(*sid).is_some_and(|p| p.ty & 1 != 0),
                        });
                    }
                    (None, [0; 4])
                }
                _ => (None, [0; 4]),
            };
            if let Some(se) = se {
                out.push(Event::Se3d { n: se.code, pos: at, note: se.note, ear });
            }
        }
        for (se, p, note) in self.world.fx_mut().take_sounds() {
            match (usize::try_from(se), p) {
                (Ok(n), Some(pos)) => out.push(Event::Se3d { n, pos, note, ear }),
                (Ok(n), None) => out.push(match note {
                    Some(note) => Event::SeNote { n, note },
                    None => Event::Se(se),
                }),
                _ => {}
            }
        }
        for (t, colour) in self.world.fx_mut().take_flashes() {
            self.st.fade_def.flash(t as i16, colour);
        }
        for s in self.world.fx_mut().take_shakes() {
            self.world.camera_shake(s);
        }
        for bs in self.world.fx_mut().take_noises() {
            self.ui.noise_bs(bs);
        }
        // A field's weather: the thunder's sound and flash, the steam's.
        for call in self.world.take_ambient_calls() {
            use piney_world::field_ambient::Op;
            match call {
                Op::Se(n) => out.push(Event::Se(n)),
                Op::SeLoopStart => out.push(Event::TobjSeLoopStart),
                Op::SeLoop(pos, rate) => {
                    if let Some(ear) = ear {
                        out.push(Event::TobjSeLoop { pos, rate, ear });
                    }
                }
                Op::Se3d(n, pos) => {
                    if let Ok(n) = usize::try_from(n) {
                        out.push(Event::Se3d { n, pos, note: None, ear });
                    }
                }
                Op::Flash { frames, colour, .. } => {
                    self.st.fade_def.flash(frames as i16, colour);
                }
                _ => {}
            }
        }
        self.events.extend(out);
    }

    /// The party's chat balloons this frame: each member's
    /// `ChatMessageSender` `ccChatMsg::OpenChat` over it (handle `1 << 24 |
    /// id`), in the order the members' frames opened them.
    fn party_chats(&mut self, shows: &[piney_world::combat::Show]) {
        use piney_world::combat::Show;
        let names = self.world.state().names();
        for s in shows {
            if let Show::Chat(w, text) = s {
                let Some(id) = self.world.combat().scene.chars.get(*w).map(|c| c.id()) else { continue };
                self.ui.open_chat((1 << 24) | u32::from(id as u16), text, &names);
            }
        }
    }

    /// What the battle's rules ask of the menu this frame:
    /// `ccMenuCtrl::SetPanelBure(slot, n)` (a party panel shakes as its
    /// member is hit) and `SetProtect(state, ch)` (the protect marks over a
    /// character whose protect a hit broke, or which it regained).
    fn menu_rules(&mut self, shows: &[piney_world::combat::Show]) {
        use piney_battle::event::{Event as Rule, Who};
        use piney_world::combat::Show;
        for s in shows {
            let (me, e) = match s {
                Show::Rule(e) => (None, e),
                Show::Kite(k, piney_battle::kite::Out::Rule(e)) => (Some(*k), e),
                Show::Member(m, piney_battle::fellow::Out::Rule(e)) => (Some(*m), e),
                Show::Enemy(n, piney_battle::enemy_motion::Call::Rule(piney_battle::enemy_ai::Out::Rule(e))) => {
                    (Some(*n), e)
                }
                _ => continue,
            };
            match *e {
                // An enemy's or a member's EntryAffect on Kite: his
                // Influence's DamageActuate (his own rules give Out::Actuate).
                Rule::DamageActuate { value, act } if !matches!(s, Show::Kite(..)) => {
                    let kt = &self.world.combat().data.kt;
                    if let Some(piney_battle::kite::Out::Actuate { power }) =
                        piney_battle::kite::damage_actuate(kt, act, value)
                    {
                        self.events.push(Event::Actuate { small: true, power: power as u8, ms: 100 });
                    }
                }
                Rule::PanelBure { slot, n } => self.ui.ctrl.set_panel_bure(slot, n as u8),
                Rule::SetProtect { state, on } => {
                    let who = match on {
                        Who::Char(c) => Some(c),
                        Who::Me => me,
                        Who::Target | Who::Nobody => None,
                    };
                    let save = &self.world.state().save;
                    let name = |id: i32| if id == 0 { save.name().to_vec() } else { self.world.char_name(id) };
                    if let Some(info) = who.and_then(|w| char_info(&self.world, w, &name)) {
                        self.ui.ctrl.set_protect(state as i16, &info);
                    }
                }
                _ => {}
            }
        }
    }

    /// What the boss asked of the screen, the map and the menus this
    /// frame: `scFadeDef->EntryFlash`, `EVENTAREAB0::SwitchLayer`, the
    /// menus' lock and the cursor.
    fn boss_calls(&mut self, shows: &[piney_world::combat::Show]) {
        use piney_battle::boss::Out;
        use piney_world::combat::Show;
        for s in shows {
            match s {
                Show::Boss(_, Out::Flash { t, colour }) => {
                    self.st.fade_def.flash(*t as i16, *colour);
                }
                // EntryFlash2 / EntryFlash3: one flash over the whole time.
                Show::Boss(_, Out::FlashFade { t, colour }) => {
                    self.st.fade_def.flash((t[0] + t[1] + t[2]) as i16, *colour);
                }
                Show::Boss(_, Out::SwitchLayer) => self.world.arena_switch_layer(),
                // LockPlayer's and UnlockPlayer's ccMenu->forbid (+0xfe) and
                // forbidChatExcept (+0x100), and cursolOff (+0x26).
                Show::Boss(_, Out::MenuForbid { on, chat_except }) => {
                    let c = &mut self.ui.ctrl;
                    c.forbid = i16::from(*on);
                    if !*on {
                        c.forbid_chat_except = 0;
                    } else if *chat_except {
                        c.forbid_chat_except = 1;
                    }
                }
                Show::Boss(_, Out::CursorOff(on)) => self.ui.ctrl.cursol_off = i16::from(*on),
                // BeginStageEffect(rgba, t0, t1, t2): EntryFlash3(t0, t1, t2,
                // rgba) on the stage fader.
                Show::Boss(_, Out::StageBegin { rgba, t }) => {
                    self.stage_fade_id = self.stage_fade.entry_flash3(t[0] as i16, t[1] as i16, t[2] as i16, *rgba);
                }
                // EndStageEffect(&id, rgba, t): while its element still
                // runs, deleted, and with t a flash out from rgba.
                Show::Boss(_, Out::StageEnd { rgba, t }) => {
                    let id = std::mem::replace(&mut self.stage_fade_id, -1);
                    if let Ok(i) = usize::try_from(id)
                        && i < piney_demo::fade::ELEMENTS
                        && self.stage_fade.check(i)
                    {
                        self.stage_fade.elm[i].status = 0;
                        if *t > 0 {
                            self.stage_fade.entry_flash(*t as i16, *rgba);
                        }
                    }
                }
                // ctrlFountain's flashes: EntryFlash2(scFadeDef, t0, t1,
                // colour) and EntryFlash(scFadeDef, t0, colour).
                Show::Entry(piney_battle::entry::Out::Flash { t0, t1, colour }) => match t1 {
                    Some(t1) => {
                        self.stage_fade.entry_flash2(*t0, *t1, *colour);
                    }
                    None => {
                        self.stage_fade.entry_flash(*t0, *colour);
                    }
                },
                // ccSpcShoutOperationName: Kite (party slot 0, id 0) calls
                // the strategy in a balloon.
                Show::Shout(operation) => {
                    let names = self.world.state().names();
                    self.ui.shout(1 << 24, i32::from(*operation), &names);
                }
                // StreamMenu over the fight: a drained member (Skeith's 20,
                // Innis's 42), Innis's images (39-41).
                Show::Boss(_, Out::StreamMenu { stream, mask }) => {
                    self.ui.open_stream_menu(*stream as i16, *mask as i16)
                }
                // The boss's death: ccClearSpcCondition.
                Show::Boss(_, Out::ClearSpcCondition) => self.world.clear_spc_condition(),
                // BeginDeadEffect: ccSqFade(0, 0, 30, 3), the music out;
                // Kyvia's death its own.
                Show::Boss(_, Out::DeadCamera { music_fade: true, .. }) => {
                    self.events.push(Event::SqFade { seq: 0, volume: 0, time: 30, mode: 3 });
                }
                Show::Boss(_, Out::MusicFade { t }) => {
                    self.events.push(Event::SqFade { seq: 0, volume: 0, time: *t, mode: 3 });
                }
                _ => {}
            }
        }
    }

    pub fn gate_out(&mut self) {
        self.world.gate_out();
    }

    fn talk(&mut self, t: TalkRequest) {
        // The talk menus read the one spoken to (cmndTarget) from before the
        // step that opens them, as the town's do: a party member (21), an
        // administrator (23), a breeder (27), a dog (44), a Grunty (45, 46).
        use piney_fieldui::talk::{Speaker, TalkTarget};
        if let TalkRequest::Menu { menu: 21 | 23 | 27 | 44 | 45 | 46, kind, code } = t {
            let target = if kind == piney_world::entry::Kind::Spc {
                TalkTarget { handle: (1 << 24) | u32::from(code as u16), who: Speaker::Spc(code) }
            } else {
                // An event NPC's stand-in: its scene index, its npcTbl row.
                let npcs = &self.world.combat().npcs;
                let row = npcs.iter().find(|n| n.who == code as usize).map_or(code, |n| i32::from(n.code));
                TalkTarget { handle: (2 << 24) | code as u32, who: Speaker::Npc(row) }
            };
            self.ui.talk_to(Some(target));
        }
        let c = &mut self.ui.ctrl;
        let mut open = |menu: i16, mode: i16| {
            c.open_req = menu;
            c.mode = mode;
            c.first_time = 1;
        };
        match t {
            TalkRequest::Open { menu } => open(menu, 0),
            TalkRequest::ClearAttack => c.pl_attack = 0,
            TalkRequest::Menu { menu, .. } => open(menu as i16, talk::action_mode(menu, 1)),
            // CheckOperate(9) refused it: no menu; the event task sees the
            // operation and the character next frame (operateSet,
            // operateTarget).
            TalkRequest::Event { .. } => {
                if let Some(vm) = &mut self.vm {
                    vm.mng.operate_target = AreaHost::target_ref(&self.world);
                }
                self.world.close_menu();
            }
            TalkRequest::Talk { .. } | TalkRequest::Shop { .. } => self.world.close_menu(),
        }
    }

    /// `ccUseItemRequest(plw, target, code, arg)` from the menus: its rules
    /// on the world now, its steps for the menu task.
    fn use_item(&mut self, target: u32, code: i32, arg: i32) -> Vec<piney_battle::item::Step> {
        let (Some(t), Some(k)) = (handle_index(&self.world, target), self.world.combat().kite) else {
            return Vec::new();
        };
        self.st.log(format!("use_item {code:#x}"));
        self.item_use = Some((k, t));
        self.world.use_item(t, code, arg)
    }

    /// One `WORLD_MAN::ShowMap` call for an item: the field's
    /// `WORLD::ShowMap` (the portals on the map) at once, or one room of
    /// the dungeon's floor. True when done.
    fn item_show_map(&mut self) -> bool {
        self.world.show_map()
    }

    /// `ccThExecuteStream(num)` over the field's resident files (a drain
    /// movie, `StreamMenu`'s): the menu task waits for its end
    /// ([`Self::movie_done`]).
    fn start_movie(&mut self, num: i32) {
        self.str_party = None;
        let save = self.world.state_out();
        let started = match (usize::try_from(num), self.stream.data.as_deref()) {
            (Ok(n), Some(data)) => {
                crate::stream::StreamPlayer::drain(&self.stream.iso, data, n, &save, &mut self.events)
            }
            _ => Err(format!("stream {num}: no DATA.BIN")),
        };
        match started {
            Ok(p) => {
                self.st.log(format!("drain_movie {num}"));
                self.drain_movie = Some(p);
            }
            Err(e) => {
                tracing::warn!("the stream: {e}; counted as played");
                self.movie_done();
            }
        }
    }

    /// The movie has ended: the menu that asked for it is answered.
    fn movie_done(&mut self) {
        if std::mem::take(&mut self.stream_menu) {
            self.ui.stream_menu_done();
        } else {
            self.ui.drain_movie_done();
        }
    }

    /// `ccThMenu` (34), after `ccThGameCtrl` and before the camera, the
    /// party and the entries (`ccSetupGameCtrl`'s priorities): Data Drain's
    /// movie, the ride's and the Fairy's Orb's slots, the menus and their
    /// requests, which the world acts on this same frame.
    fn menu_task(&mut self, pad: &Pad, ctx: &mut Ctx) {
        // Data Drain's movie holds the screen; its end is the menu
        // task's to see this frame.
        if let Some(p) = &mut self.drain_movie {
            let enemy = &mut self.drain_enemy;
            let party = &mut self.str_party;
            let mut extra = |ctx: &mut Ctx, scene: &piney_stream::scene::Scene| {
                let lights = |world| piney_stream::draw::lights(scene, world);
                let to_screen = piney_stream::draw::to_screen(scene);
                if let Some(p) = party.as_mut() {
                    let dummy = |name: &str| scene.world_named(name).map(|m| piney_stream::scene::to_mat4(&m));
                    p.draw(&mut ctx.layers, scene.default_layer, to_screen, &lights, &dummy);
                }
                let Some(e) = enemy.as_mut() else { return };
                // The menu task's switch reads the frame the stream
                // drew; the enemy's task draws after it.
                e.at_frame(scene.frame_now);
                e.draw(&mut ctx.layers, scene.default_layer, to_screen, &lights);
            };
            match p.step_with(pad, &mut self.events, &mut extra) {
                Some(f) => self.stream.frame = Some(f),
                None => {
                    self.world.set_rand(p.rand());
                    self.drain_movie = None;
                    self.drain_enemy = None;
                    self.str_party = None;
                    self.movie_done();
                }
            }
        }
        // Inside ccPuccigusoStart, the menu task's slot first.
        self.ride_menu_slot();
        // ccUseItemRequest's loop on a Fairy's Orb: this frame's call.
        if self.map_showing {
            self.map_showing = !self.item_show_map();
        }
        let w = self.hud_world();
        self.ui.step_into(pad, &w, self.world.state_mut(), self.count, ctx);
        // The menu task stops in ccUseItemRequest: the use's rules, and
        // its steps back to the task, which goes on this frame.
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
            let steps = self.use_item(target, code, arg);
            self.ui.answer_item(steps, pad, self.world.state_mut(), self.count, Some(ctx));
        }
        if self.world.targeting().in_menu && self.ui.menu_type() == -1 {
            self.world.close_menu();
        }
    }

    fn menu_request(&mut self, r: piney_fieldui::Request) {
        use piney_fieldui::Request as R;
        match r {
            // The Grunty Flute's: ccClearConditionAllEnemy, ccPuccigusoStart.
            R::ItemStep(piney_battle::item::Step::ClearConditionAllEnemy) => {
                self.world.combat_mut().clear_condition_all_enemy();
            }
            R::ItemStep(piney_battle::item::Step::Pucciguso(n)) => self.ride_start(n),
            // A Fairy's Orb (ccUseItemRequest, gcmn 0x0057b988): the first
            // WORLD_MAN::ShowMap call; the rest come a frame at a time.
            R::ItemStep(piney_battle::item::Step::WaitMap) => self.map_showing = !self.item_show_map(),
            // An item use's step on the world, as the menu task reaches it.
            R::ItemStep(s) => {
                let (user, target) = self.item_use.unwrap_or_default();
                if !self.world.item_step(&s, user, target) {
                    self.st.log(format!("item step not carried out: {s:?}"));
                }
            }
            // ccSpcMessageOpenTreasureBox: the party's line on a box.
            R::SpcMessageOpenTreasureBox => self.world.spc_message(0x10010),
            // The object menus' calls on Kite (a breakable).
            R::BreakSomething => self.world.kite_menu_call(piney_world::combat::KiteMenuCall::Break),
            R::AttackCancel => self.world.kite_menu_call(piney_world::combat::KiteMenuCall::AttackCancel),
            // ccThPartyAdd's ccParty::AddMember(id), PARTY's Remove and
            // Disband (ccParty::DelMember(slot)) and the gate's leave
            // (ccSpcChar::TransferOut of each member) over the field's
            // characters; the menus set the faces themselves.
            R::AddMember(id) => {
                self.world.party_add(i32::from(id));
            }
            R::DelMember(slot) => self.world.party_remove(-slot),
            R::TransferOut(h) => {
                if h >> 24 == 1 {
                    self.world.transfer_out(i32::from(h as u16 as i16));
                }
            }
            R::Se(n) => self.events.push(Event::Se(n)),
            R::SeNote { n, note } => self.events.push(Event::SeNote { n: n.max(0) as usize, note: note as i8 }),
            R::CameraShake { power, cycle, time, dirc } => self.world.camera_shake([power, cycle, time, dirc]),
            R::SleepAll => {
                self.asleep = true;
                self.world.set_asleep(true);
            }
            R::WakeAll => {
                self.asleep = false;
                self.world.set_asleep(false);
            }
            R::VoiceStop => self.events.push(Event::VoiceStop),
            R::VoiceRequest { grp, msg } => {
                // The save as it stands (talkNum changes as members talk).
                self.events.push(crate::mode::voice_options(&self.world.state().save));
                self.events.push(Event::Voice { event: grp, msg });
            }
            // WORLD_MAN::SetMapAlpha: the minimap's fade.
            R::MapAlpha(a) => self.map_st.set_alpha(a),
            // Gate Out's ChangeArea(0, town) (menu 10, after its
            // DeleteNoPartyMember, which has no one to let go here yet),
            // or any other the menus make; once.
            R::ChangeArea { area, n } if !self.leaving => self.world.change_area(area, n),
            // TransFieldMenu (86): WORLD_MAN::GoField, from a dungeon to
            // its field.
            R::GoField if !self.leaving => {
                self.world.go_field();
            }
            // Log Out (4, 7) and OPTION's Title Screen (1, 7):
            // ccGame::ChangeRequest, for the session.
            R::ChangeMode { num, sf } => self.events.push(Event::ChangeMode { num, sf }),
            // ccSaveData::SetSoundEnv from OPTION's Sound page.
            R::SoundEnv { main, bgm, se, output } => self.events.push(Event::Volumes { main, se, bgm, output }),
            R::CameraType(t) => self.world.set_camera_type(t),
            // Switched on, the pad buzzes once.
            R::Vibration { on } => {
                if on {
                    self.events.push(Event::Actuate { small: true, power: 160, ms: 200 });
                }
            }
            // The battle's: targets, skills, Data Drain's hold and cost, the
            // party's orders.
            R::TargetFix(on) => self.world.set_target_fix(on),
            R::TargetClear => self.world.change_command_target(None),
            R::Target(h) => {
                let t = handle_index(&self.world, h).map(|i| self.target_of(i));
                self.world.change_command_target(t);
            }
            R::Skill { target, skill } => {
                let t = handle_index(&self.world, target);
                self.world.player_skill_request(t, i32::from(skill));
            }
            R::DrainAffect(h) => {
                if let Some(i) = handle_index(&self.world, h) {
                    self.world.affect_from_player(i, 13);
                }
            }
            // ccChar::EntryAffect(target, plw, kind, 0, 0, 0) from the
            // menus: a box opened (11, ItemBoxMenuT and the box menus).
            R::Affect { target, kind } => {
                if let Some(i) = handle_index(&self.world, target) {
                    self.world.affect_from_player(i, kind);
                }
            }
            R::PlayerSp(sp) => self.world.set_player_sp(sp),
            // Data Drain (menu 66): its rules on the world, answered before
            // the menu's next frame; the side effect; the level lost; the
            // targets cleared; SYSTEM ERROR's game over.
            R::DataDrain { target, sid } => {
                if let Some(t) = handle_index(&self.world, target) {
                    let d = self.world.data_drain(t, sid);
                    self.st.log(format!("data_drain {sid}: {:?}", &d.drops[..d.count.min(17)]));
                    self.ui.drain_drops(&d.drops, d.count);
                }
            }
            R::DrainSideEffect => {
                let s = self.world.drain_side_effect();
                self.st.log(format!("drain_side_effect {}", s.id));
                self.ui.drain_side_effect(s.id, s.level_down, s.lost);
            }
            R::DrainLevelDown => self.world.kite_level_down(),
            R::TargetsCleared => self.world.clear_command_targets(),
            R::GameOver => self.world.compulsion_game_over = true,
            R::WorldHidden(on) => self.world_hidden = on,
            R::DisplayOffset { x, y } => self.events.push(Event::DisplayOffset { x, y }),
            R::Flash => self.ui.gate_flash(),
            // Data Drain's movie: the stream the target picks, played as
            // ccThExecuteStream plays one (no subtitles, no music of its
            // own) over the field's resident files; the menu task waits for
            // its end.
            // The Key Items refuse a Ryu Book outside a town (help 8), so
            // its cover never plays here.
            R::BookStream(_) => {}
            R::DrainMovie(num) | R::StreamMenu(num) => {
                self.stream_menu = matches!(r, R::StreamMenu(_));
                self.start_movie(num);
            }
            // ccThStrParty: StreamMenu's members drawn into its stream.
            R::StrParty(flags) => {
                self.str_party = Some(self.world.str_party(flags));
                self.st.log(format!("str_party {flags:#x}"));
            }
            // ccThDrainEnemy: the enemy drawn into the movie's scene.
            R::DrainEnemy(h) => {
                self.drain_enemy = handle_index(&self.world, h).and_then(|t| self.world.drain_enemy(t));
                self.st.log(format!("drain_enemy {h:#x} {}", self.drain_enemy.is_some()));
            }
            // ccSPC::ChangeEquip: the new weapon on the model (the
            // combat's CalcReal runs on the save's record every frame).
            R::ChangeEquip { target, cat, .. } => {
                if target >> 24 == 1 {
                    self.world.change_equip((target & 0xffff) as u16 as i16 as i32, cat);
                }
            }
            R::ChangeEquipReport { member, n } => {
                if let Some(m) = handle_index(&self.world, member) {
                    self.world.equip_report(m, n);
                }
            }
            R::ChatCmd { member, cmd } => {
                if let Some(m) = handle_index(&self.world, member) {
                    self.world.chat_cmd(m, cmd, None, 0);
                }
            }
            R::ChatOrder { member, cmd, target, skill } => {
                if let Some(m) = handle_index(&self.world, member) {
                    let t = handle_index(&self.world, target);
                    self.world.chat_cmd(m, cmd, t, skill);
                }
            }
            R::ManualOff(member) => {
                if let Some(m) = handle_index(&self.world, member) {
                    self.world.manual_off(m);
                }
            }
            R::ManualModeAi { member, ev } => {
                if let Some(m) = handle_index(&self.world, member) {
                    self.world.manual_mode_ai(m, ev);
                }
            }
            R::RemoteCmd { member, cmd } => {
                if let Some(m) = handle_index(&self.world, member) {
                    self.world.remote_cmd(m, cmd);
                }
            }
            // The spring's menus: its camera, the camera back, the party
            // held and let go.
            R::Talk(piney_fieldui::talk::TalkReq::FountainCamera) => self.world.fountain_camera(),
            R::Talk(piney_fieldui::talk::TalkReq::Camera(n)) => self.world.change_camera(n as i16),
            R::Talk(piney_fieldui::talk::TalkReq::FountainParty(on)) => self.world.fountain_party(on),
            // The items, the minimap's and the rest have nothing to act on
            // here yet.
            _ => {}
        }
    }
}

impl Mode for AreaMode {
    fn step(&mut self, pad: &Pad) -> Frame {
        self.count = self.count.wrapping_add(1);
        self.frames += 1;
        self.st.pad = *pad;
        self.st.frame = self.frames;
        // The field's tasks gone (ccDeleteAllThread): only ccThGameOver.
        if let Some(t) = self.game_over.as_mut()
            && !t.field_runs()
        {
            return t.frame_alone(&mut self.events);
        }
        // ccThEvent (32) first.
        self.event_frame(pad);
        let mut ctx = Ctx::new(View::default());
        self.world.set_menu_view(talk::MenuView {
            idle: self.ui.menu_type() == -1 && self.ui.ctrl.open_req & 0xfff != 74,
            forbid: self.ui.ctrl.forbid != 0,
            forbid_chat_except: self.ui.ctrl.forbid_chat_except != 0,
            pl_attack: self.ui.ctrl.pl_attack != 0,
        });
        self.world.set_menu_type(self.ui.menu_type());
        // ccThPucciguso's slot (its Main runs with the field's tasks); the
        // menu task, first in the game's frame, sees pgRideFlag before it.
        self.ride.seen = self.world.pg_ride();
        if let Phase::Play(_) = self.world.phase()
            && !self.asleep
            && self.setup != Setup::Left
        {
            self.ride_task_slot(pad);
        }
        // A story map's message: `ccMsg->Check(0)` as its Draw asks, with
        // this frame's pad.
        if self.story_message {
            let r = self.ui.message_check(pad, self.world.state());
            if r != 0 {
                self.story_message = false;
                self.st.log(format!("message_check {r}"));
                self.world.story_message_closed();
            }
        }
        // ccThGameCtrl (33): the target and the buttons; the map button
        // where its test came, past the game over, ghoFlag and menuClrWait.
        self.world.step_game_ctrl(pad);
        if self.world.map_test() && !self.asleep && self.setup != Setup::Left {
            let area = self.world.scene().area;
            self.map_st.button(self.world.state_mut(), area, pad.push.bits());
        }
        // ccThGameCtrl's first step of the game over: CloseMenu,
        // ccMessage::Close, ccMenu.forbid (the party's AI and the enemies'
        // conditions are not ported).
        if self.world.take_game_over() {
            self.ui.message_close();
            self.ui.ctrl.forbid = 1;
            self.game_over = Some(crate::gameover::GameOverTask::new(self.stream.data.clone()));
        }
        for t in self.world.take_talk() {
            self.talk(t);
        }
        if self.world.take_pl_attack() {
            self.ui.ctrl.pl_attack = 1;
        }
        // ccThMenu (34), before the world's other tasks: what it asks for
        // is acted on, and heard, this frame (worklog 336).
        if let Phase::Play(f) = self.world.phase()
            && f >= 1
            && self.setup != Setup::Left
        {
            self.menu_task(pad, &mut ctx);
        }
        // Kite's skill and act around the step (`PINEY_LOG=piney_game::area=trace`).
        let dbg = tracing::enabled!(tracing::Level::TRACE);
        if dbg {
            let c = self.world.combat();
            if let Some(k) = c.kite {
                let ch = &c.scene.chars[k];
                if ch.spc_char.act_num == 17 || ch.skill_id != 0 {
                    tracing::trace!(
                        "PRE {} skill {} act {} cnt {} anm {}",
                        self.frames,
                        ch.skill_id,
                        ch.spc_char.act_num,
                        ch.spc_char.cnt,
                        ch.anm_flag
                    );
                }
            }
        }
        // ccThCamera (40) on; hidden (Data Drain's black, its movie), the
        // world draws nothing.
        if self.world_hidden {
            self.world.step_into(pad, &mut Ctx::new(View::default()));
        } else {
            self.world.step_into(pad, &mut ctx);
        }
        if dbg {
            let c = self.world.combat();
            if let Some(k) = c.kite {
                let ch = &c.scene.chars[k];
                if ch.spc_char.act_num == 17 || ch.skill_id != 0 {
                    tracing::trace!(
                        "POST {} skill {} act {} cnt {} anm {} runs {}",
                        self.frames,
                        ch.skill_id,
                        ch.spc_char.act_num,
                        ch.spc_char.cnt,
                        ch.anm_flag,
                        c.skills.borrow().runs.len()
                    );
                }
            }
        }
        self.gate_stream(pad);
        self.member_item_steps();
        // The battle's sounds this frame, then the effects', heard from the
        // active camera (the effects were started inside the frame); the
        // boss's calls; the party's chat balloons.
        let shows = self.world.take_shows();
        // ccPlayer::DamageActuate: Kite hit, the small motor and the hit's
        // power for 100 ms.
        for s in &shows {
            if let piney_world::combat::Show::Kite(_, piney_battle::kite::Out::Actuate { power }) = s {
                self.events.push(Event::Actuate { small: true, power: *power as u8, ms: 100 });
            }
        }
        // The Administrator's noise2 (ccMenu.interNoiz = 2).
        if self.world.take_noise() {
            self.ui.noise(2);
        }
        self.battle_sounds(&shows);
        // ccSoundMain's scene sounds: area 15's church music reads the
        // camera and the block.
        let scene_sound = piney_audio::scene::SceneInput {
            camera: Some(self.world.camera().active().pos),
            kite: self.world.combat().kite_pos(),
            block: self.world.scene().block,
            mac_anu: false,
            breeder: None,
        };
        self.events.push(Event::SceneSound(scene_sound));
        self.boss_calls(&shows);
        self.menu_rules(&shows);
        self.party_chats(&shows);
        if let Phase::Play(f) = self.world.phase()
            && f >= 2
            && self.setup != Setup::Left
        {
            // ccThFieldDisp's WORLD::Draw -> DrawMiniMap or DUNGEON::Draw ->
            // MakeMiniMap, DrawMap, after ccThMenu has set this frame's alpha.
            let p = self.world.player();
            let (pos, dirc) = (p.body.pos, p.body.dirc);
            let scene = self.world.scene();
            let mut ents = self.world.map_entries();
            // ccThFieldDisp sleeps from the frame a door or the stairs ask
            // for the change (ccSleepNoSleepThread in ChangeRequest).
            let awake = !self.asleep && !self.world.scene_change_asked();
            let in_battle = self.world.combat().battle.in_battle != 0;
            let place = self.world.place_mut();
            let fonts = self.fonts.as_ref();
            // Hidden with the world (Data Drain's black and its movie).
            let mut unseen = Ctx::new(View::default());
            let map_ctx = if self.world_hidden { &mut unseen } else { &mut ctx };
            self.map_st.hud_scale = self.ui.hud_scale;
            let shown = piney_world::map::area_frame(
                place,
                &mut self.map_st,
                &scene,
                pos,
                dirc,
                awake,
                in_battle,
                fonts,
                &mut ents,
                map_ctx,
            );
            if shown {
                self.ui.map_on();
            }
            if awake {
                self.world.set_map_alphas(&ents);
            }
        }
        // The boss's stage fader: ccBoss::DrawStageEffect each frame.
        for (c0, c1, cnt, tcnt) in self.stage_fade.advance() {
            piney_desktop::fade::draw_on(&mut ctx, 2, c0, c1, cnt.max(0) as u32, tcnt.max(0) as u32);
        }
        // ccMenu's fader, sent last by Disp (so drawn first on its layer).
        if self.setup != Setup::Left {
            self.st.fade.send(&mut ctx, MENU_LAYER);
            // scFadeDef: the screen's own flashes (piros_colour's).
            self.st.fade_def.send(&mut ctx, piney_desktop::layers::FONT_LAYER);
        }
        // ccThGameOver, after the field's tasks (the same priority, started
        // later): its noise over the field, which it squeezes away.
        let over = self.game_over.as_mut().map(|t| {
            let events = &mut self.events;
            self.world.with_rands(|cc, rand| t.step(cc, rand, &mut ctx, events))
        });
        let mut frame = ctx.finish();
        self.world.set_clear(&mut frame);
        if let Some(step) = over {
            if let Some(v) = step.view {
                crate::gameover::squeeze(&mut frame, &v);
            }
            if step.black {
                frame.clear = piney_draw::Rgba::BLACK;
            }
        }
        // The set-up's screen, black with the event's window, while a pass
        // waits on one (the church's "Open the book." in event 11); the
        // window's voice and sounds.
        if let Setup::Pass(_) = self.setup {
            if let Some(f) = self.ui.setup_frame(pad) {
                frame = f;
            }
            for r in self.ui.take_requests() {
                self.menu_request(r);
            }
        }
        // A stream the scripts play holds the screen.
        if let Some(f) = self.stream.frame.take() {
            return f;
        }
        frame
    }

    fn take_events(&mut self) -> Vec<Event> {
        let scene = self.world.scene();
        let mut out = Vec::new();
        for r in self.world.take_requests() {
            match r {
                // A story map's scene: the menus (logged as the events'
                // `menu_ban`), its message.
                Request::Story(StoryRequest::MenuBan(on)) => {
                    self.st.log(format!("menu_ban {on}"));
                    self.ui.menu_ban(on);
                }
                Request::Story(StoryRequest::Message { rec }) => {
                    self.st.log("message_open story".into());
                    let save = self.world.state().clone();
                    self.ui.story_message(rec, &save);
                    self.story_message = true;
                }
                Request::Story(StoryRequest::ChangeArea(..)) => {}
                Request::Game(GameRequest::SoundFadeOut) => out.push(Event::SoundFadeOut),
                Request::Game(GameRequest::AllSoundOff) => out.push(Event::AllSoundOff),
                // ccSndSQLoad: the area's bank as ccSetupGameCtrl picks it.
                // The story area's model is GetEventAreaInfo(game.field)'s
                // (main 0x00169160), not WORLD_MAN's: Skeith's arena (field
                // 1) keeps area 27's WORLD_MAN. Piros is character 8 in a
                // party slot (checkPartyMenberNum(8)).
                Request::SqLoad => {
                    let wm = self.world.world_man();
                    let save = &self.world.state().save;
                    let tables = piney_data::area::AreaTables::of(self.world.volume());
                    let field_model = (scene.field != 0)
                        .then(|| tables.event_area_info(scene.field, piney_data::area::flag71(save)))
                        .flatten()
                        .map_or(0, |info| info.model);
                    let music = piney_audio::AreaMusic {
                        area: scene.area,
                        scene_replaced: scene.changed(),
                        town: scene.town.max(0) as u8,
                        crisis: save.u8(offset::CRISIS) != 0,
                        field: scene.field.max(0) as u16,
                        field_model,
                        field_type: wm.field_type as u8,
                        bg: wm.weather as u8,
                        piros: self.world.party().contains(&crate::piros::PIROS),
                        dungeon_type: wm.dungeon_type[scene.dungeon.clamp(0, 2) as usize],
                        special_room: self.world.special_room(),
                        area_prev: scene.area_prev,
                    };
                    if let Some(ctx) = piney_audio::setup_context(&music) {
                        out.push(Event::SqLoad(ctx));
                    }
                }
                Request::GameStart => out.push(Event::GameStart),
                // ccSndBgmCtrl once the fade in is over.
                Request::Game(GameRequest::BgmCtrl) => {
                    let save = &self.world.state().save;
                    out.push(Event::BgmCtrl(piney_audio::BgmWorld {
                        scene_replaced: scene.changed(),
                        town: scene.town,
                        crisis: save.u8(offset::CRISIS) == 1,
                        dt_bgm: i32::from(save.u8(offset::DT_BGM) as i8),
                    }));
                }
                Request::Game(
                    GameRequest::SqLoad(_)
                    | GameRequest::GameStart
                    | GameRequest::EnableReset(_)
                    | GameRequest::Transfer
                    | GameRequest::WarpTransfer,
                ) => {}
                // ChangeScene's ChangeRequest(6, 7) (main 0x001671e0) calls
                // ccDisableThEvent whoever asked: the event task idles from
                // its next frame (phase -1) until the next set-up's
                // ccEnableThEvent(0). Then the session fades this scene out
                // and sets the next one up.
                Request::ChangeScene => {
                    if !self.leaving {
                        self.leaving = true;
                        // ccThPuccigusoDelete: a dungeon's way in ridden into.
                        self.ride_leave();
                        if let Some(vm) = &mut self.vm {
                            vm.disable();
                        }
                        // ccSleepNoSleepThread(1, 1): every task but the
                        // event's sleeps from here (a door's or the stairs'
                        // DUNGEON::Draw would otherwise map the new floor
                        // from where Kite stood on the old).
                        self.asleep = true;
                        self.world.set_asleep(true);
                        out.push(Event::ChangeScene(scene));
                    }
                }
            }
        }
        out.append(&mut self.events);
        // bgmChange's game.inBattle, every frame the area's tasks run.
        if let Phase::Play(_) = self.world.phase() {
            out.push(Event::InBattle(self.world.combat().battle.in_battle != 0));
        }
        out
    }

    fn frame_rate(&self) -> u32 {
        self.world.frame_rate()
    }

    /// A stream's own files while the scripts play one.
    fn archive(&self) -> Option<Arc<Archive>> {
        self.stream.player.as_ref().or(self.drain_movie.as_ref()).map(crate::stream::StreamPlayer::archive)
    }

    fn hook(&mut self, name: &str) -> bool {
        match name {
            "gateout" => {
                self.gate_out();
                true
            }
            // TransFieldMenu's WORLD_MAN::GoField (menu 86).
            "gofield" => self.world.go_field(),
            _ => false,
        }
    }

    fn title(&self) -> String {
        let phase = match self.world.phase() {
            Phase::FadeOut(n) => format!("fade out {n}"),
            Phase::Hold(n) => format!("loading {n}"),
            Phase::Play(n) => format!("play {n}"),
        };
        let pl = self.world.player();
        let p = pl.body.pos.map(f32::from_bits);
        let s = self.world.scene();
        let menu = match self.ui.menu_type() {
            -1 => String::new(),
            m => format!(" - menu {m}"),
        };
        format!(
            "The World - area {} field {} - {phase} - Kite at ({:.0}, {:.0}, {:.0}) act {}{menu}",
            s.area, s.field, p[0], p[1], p[2], pl.acts.act
        )
    }
}

/// The area words' tables (`piney_data::area`) of the disc's volume.
pub fn area_tables(disc: &mut Iso) -> Result<&'static piney_data::area::AreaTables, String> {
    Ok(piney_data::area::AreaTables::of(disc.volume().map_err(|e| e.to_string())?))
}

/// `WORLD_MAN` for story area `n` as the event instruction `area n` leaves
/// it (`piney_data::area::ev_area`): `SimGenerateCode` of the area's own
/// words, read from the executable, on its server.
pub fn story_world_man(disc: &mut Iso, n: i32, crisis: bool) -> Result<WorldMan, String> {
    let tables = area_tables(disc)?;
    let code = piney_data::area::story_area_code(tables, n, false).ok_or(format!("area {n}: no words make it"))?;
    let dungeon_flag = tables.dungeon_rule().save_flag_of_crisis(crisis);
    Ok(WorldMan::from_code(&code, tables, crisis, dungeon_flag))
}

/// Instruction 118 `area n` (`ccEvAreaCodeAdd`, `piney_data::area::ev_area`)
/// on `server` with `save`'s flags: a story area past 13 made from its own
/// words; None for 1 to 13, which name no words.
pub fn ev_area_world_man(
    iso: &Path,
    n: i32,
    server: i32,
    save: &piney_data::save::SaveData,
) -> Result<Option<WorldMan>, String> {
    use piney_data::area::EvArea;
    let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let tables = area_tables(&mut disc)?;
    let crisis = save.u8(offset::CRISIS) != 0;
    match piney_data::area::ev_area(tables, n, server, piney_data::area::flag71(save)) {
        EvArea::Generated(code) => {
            let dungeon_flag = piney_world::area::dungeon_flag(tables, save);
            Ok(Some(WorldMan::from_code(&code, tables, crisis, dungeon_flag)))
        }
        EvArea::Number(_) => Ok(None),
        EvArea::Missing => Err(format!("area {n}: its words are not in the tables")),
    }
}

/// Instruction 118 `area n` for a numbered story area (1-13): `wm` with
/// `eventAreaNumber` n ([`WorldMan::with_event_number`]).
pub fn ev_area_number(
    iso: &Path,
    n: i32,
    wm: &WorldMan,
    save: &piney_data::save::SaveData,
) -> Result<WorldMan, String> {
    let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let tables = area_tables(&mut disc)?;
    let crisis = save.u8(offset::CRISIS) != 0;
    Ok(wm.with_event_number(tables, n, piney_data::area::flag71(save), crisis))
}

/// `WORLD_MAN::SetGenerateCode(a, b, c)` on `server` with `save`'s flags:
/// the area and the change of scene to it.
pub fn set_generate_code(
    iso: &Path,
    words: [i32; 3],
    server: i32,
    save: &piney_data::save::SaveData,
) -> Result<(WorldMan, piney_data::area::Go), String> {
    let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let tables = area_tables(&mut disc)?;
    WorldMan::set_generate_code(tables, words, server, save)
        .ok_or(format!("the words {words:?} are not all in their tables"))
}
