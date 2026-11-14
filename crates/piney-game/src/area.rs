//! Mode 6, `ccSetupGameCtrl` for every scene: a Root Town (area 0), a field
//! (1), a dungeon (2), a story map, with the menus and HUD (`piney-fieldui`)
//! over `piney_world::world`. A frame runs the event task (`ccThEvent`,
//! [`crate::ev_host`]), `ccThGameCtrl`, `ccThMenu`, then the area's tasks.
//! The event task arrives asleep in the scene that asked for this one and
//! ends it before `ccStartThEvent`; then the passes at 0 and 2,
//! `rebootSpcManager`, and `ccEnableThEvent(4)` at F0 (docs/engine/field-game.md).

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
use piney_world::chara::party::Spcs;
use piney_world::event::talk::{self, TalkRequest};
use piney_world::story_map::Kept;
use piney_world::story_map::StoryRequest;
use piney_world::world::CharId;
use piney_world::world::cast::Ask;
use piney_world::world::{Request, World};
use piney_world::{Phase, Request as GameRequest};

use crate::ev_host::{self, EvHost, EvStream, HostWorld};
use crate::field_host::{FieldState, MENU_LAYER, STATUS_WORLD};
use crate::mode::{Event, Mode};
use crate::world::Setup;
use piney_data::store::Table;

mod ride;

pub struct AreaMode {
    world: Box<World>,
    ui: Box<FieldUi>,
    /// The event task.
    vm: Option<Vm>,
    /// What the scripts see besides the world (`ccGame`, the fader, the
    /// call log).
    st: FieldState,
    setup: Setup,
    frames: u64,
    events: Vec<Event>,
    /// Where a change of scene was asked to go (raised once; the title
    /// shows it).
    leaving: Option<String>,
    /// The last menu the port closed at once (unported).
    unported: Option<i16>,
    /// `ccGame.areaLevel`, which the Chaos Gate's keyword screen writes.
    area_level: i32,
    /// A Ryu Book's cover stream plays (`ccThBook`'s `ccThExecuteStream`).
    book_stream: bool,
    /// The palette a Ryu Book last put on the resident `str8800e`'s
    /// backdrop, kept while the town's files are (Book I sets none).
    cover_palette: Option<piney_fieldui::book::CoverPalette>,
    /// What `WORLD_MAN` keeps of the minimap (the map itself is the
    /// field's or the dungeon's), whether every task sleeps, and the fonts
    /// its labels are drawn with.
    map_st: piney_world::map::MapState,
    asleep: bool,
    /// A story map's message is up (area 43's `kiteSelfTalk`).
    story_message: bool,
    fonts: Option<piney_desktop::kanji::Fonts>,
    /// The scripts' cutscene streams (`stream`).
    stream: EvStream,
    /// The item use the menu task is inside (`ccUseItemRequest`): the
    /// user's and the target's scene indices, for its steps.
    item_use: Option<(usize, usize)>,
    /// A menu's own screen over nothing (Data Drain's black, the movie's
    /// place, the gate hack's screen): the world draws nothing
    /// (`piney_fieldui::Request::WorldHidden`).
    world_hidden: bool,
    /// Data Drain's movie (`ccThExecuteStream`, [`piney_fieldui::Request::DrainMovie`]),
    /// a step a frame over the area until it ends.
    drain_movie: Option<crate::stream::StreamPlayer>,
    /// Its `ccThDrainEnemy`, drawn into it.
    drain_enemy: Option<piney_world::chara::foe::DrainEnemy>,
    /// The members' item uses being played ([`MemberItem`]).
    member_items: Vec<MemberItem>,
    /// The movie is `StreamMenu`'s (menu 74), with its `ccThStrParty`.
    stream_menu: bool,
    /// A boss's stage fader (`ccBoss::InitStageEffect`: `scStageFade` on
    /// its `stageLayer`, priority 2, between the stage and the
    /// characters), and `m_stageEffId`.
    stage_fade: piney_demo::fade::ScFade,
    stage_fade_id: i32,
    /// `ccMenu` +0x238 `dummyTarget` a boss set (Kyvia's fourth fight's
    /// death): the character whose HP the gauge shows, read each frame.
    dummy_target: Option<usize>,
    str_party: Option<piney_world::chara::foe::StrParty>,
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
fn char_info(w: &World, who: usize, names: &dyn Fn(i32) -> Vec<u8>) -> Option<piney_fieldui::CharInfo> {
    let c = w.combat();
    let ch = c.scene.chars.get(who)?;
    let base = ch.base();
    let pc = ch.is_pc();
    let handle = c.char_id(who).handle();
    let name = if pc {
        names(i32::from(ch.id()))
    } else {
        base.label.map(piney_data::tables::sjis::encode).unwrap_or_default()
    };
    let g = w.hud_geometry(who).unwrap_or_default();
    let (ability, elements) = match c.scene.temp_time(who) {
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
        exdefense: ch.foe_state().map_or(0, |f| f.row.exdefense),
        exdefense_lowered: piney_battle::damage::exdefense_lowered(&c.scene, who) as i16,
        tag: g.tag,
        bar_res: g.bar_res,
        bar: g.bar,
        pp: c.scene.param(who).map_or(0, |p| p.pp_count),
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
fn handle_index(w: &World, h: u32) -> Option<usize> {
    w.scene_index(CharId::from_handle(h)?)
}

/// One of the cast's own characters (a town's NPC, its Chaos Gate) as the
/// HUD's target window reads it: its name, base type flags, id and height,
/// the cursor over it (`ccCalcTagPosChar` at 0.45 of its height, mode 0:
/// 1 on screen) and its `cmndDist`.
fn cast_info(w: &World, id: CharId) -> Option<piney_fieldui::CharInfo> {
    let c = w.char_info(id)?;
    Some(piney_fieldui::CharInfo {
        handle: id.handle(),
        types: c.flags,
        id: w.base_id(id).unwrap_or(0),
        name: c.name,
        height: f32::from_bits(c.height),
        attribute: -1,
        in_view: true,
        tag: w.tag_pos(id, 0x3ee6_6666, 0).filter(|t| t.2 != 0).map(|t| (t.0, t.1)),
        cmnd_dist: w.command_sorted().iter().find(|s| s.0 == id).map_or(0.0, |s| f32::from_bits(s.1)),
        ..Default::default()
    })
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

    /// `ccSetupGameCtrl` on the scene [`World::enter`] made (a Root Town,
    /// a field, a dungeon, a story map): the menu task's `ccMenuCtrl` and
    /// its faces, Area Information's words, the effects' and particles'
    /// set-up, the event task to start once the fade and hold are over.
    pub fn enter(iso: &Path, archive: &Arc<Archive>, mut world: World, vm: Option<Vm>) -> Result<AreaMode, String> {
        let mut disc = crate::open_disc(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let volume = disc.volume().map_err(|e| e.to_string())?;
        let mut ui = FieldUi::new(&mut disc, archive).map_err(|e| format!("the field's menus: {e}"))?;
        crate::world::repair_buttons(world.state_mut());
        // The menu faces (ccCheckMenuFaceNameParty, gcmn 0x0056a930): party
        // slot 0 Kite with the bracelet's colours or without, then the
        // members the party brought.
        let plcol = world.state().save.u8(offset::PLCOL) != 0;
        ui.set_menu_face(0, 0, plcol);
        for (slot, &id) in world.party().iter().enumerate() {
            if id >= 0 {
                ui.set_menu_face(slot, id, plcol);
            }
        }
        let scene = world.scene();
        // Area Information (menu 87): the words WORLD_MAN holds.
        ui.set_area_words(world.world_man().words, scene.server, world.state());
        let stories = if vm.is_some() {
            crate::story::from_tables(piney_data::area::AreaTables::of(volume))
        } else {
            Default::default()
        };
        let mut st = FieldState::new(&world.state().save, stories);
        st.game = crate::field_host::game_of(&scene);
        st.announcements = crate::desktop::announcements(iso);
        // ccThEffect's and ccThParticle's set-up.
        match crate::fx::AreaFx::new(archive, volume) {
            Ok(fx) => world.set_fx(Box::new(fx)),
            Err(e) => tracing::warn!("the area's effects: {e}"),
        }
        let map_st = piney_world::map::MapState::new(world.state());
        let fonts = piney_desktop::assets::read_fonts(volume, archive).ok();
        let setup = if vm.is_some() { Setup::Before } else { Setup::Off };
        world.set_loading(vm.is_some());
        // The sound driver reads `saveData.voice` and `parodyFlag` each time
        // it plays a voice; the port's keeps a copy, so hand it the save's
        // (a story start enters here without the desktop).
        let events = vec![crate::mode::voice_options(&world.state().save), Event::GameArea(scene.area)];
        Ok(AreaMode {
            world: Box::new(world),
            ui: Box::new(ui),
            vm,
            st,
            setup,
            frames: 0,
            events,
            leaving: None,
            unported: None,
            area_level: 0,
            book_stream: false,
            cover_palette: None,
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
            dummy_target: None,
            game_over: None,
            ride: ride::RideSeq::default(),
            map_showing: false,
            fonts,
            stream: EvStream { iso: Some(iso.to_path_buf()), data: Some(archive.clone()), ..Default::default() },
        })
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

    /// Leave: the save, the event task, the scene, `WORLD_MAN`'s area, the
    /// dungeon (which lives on for its next room or floor) or area 15's
    /// map (for its next block), and the party.
    pub fn leave(mut self) -> (SaveState, Option<Vm>, Scene, WorldMan, Option<Kept>, Spcs) {
        // ccSetupGameCtrl's ccStoreSpcCondition, for the next scene's party.
        self.world.store_conditions();
        let (state, scene, wm) = (self.world.state_out(), self.world.scene(), self.world.world_man_out());
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

    /// The field UI.
    pub fn ui(&self) -> &FieldUi {
        &self.ui
    }

    /// The world.
    pub fn world(&self) -> &World {
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
            if let Some(f) = c.scene.param_mut(e) {
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
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// `scFadeDef`, the fader the bosses' flashes and the events use.
    #[cfg(test)]
    pub(crate) fn fade_def(&self) -> &crate::field_host::ScFade {
        &self.st.fade_def
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
    pub fn with_host<R>(&mut self, f: impl FnOnce(&mut EvHost<World>) -> R) -> R {
        let mut h = EvHost {
            world: &mut *self.world,
            ui: &mut self.ui,
            st: &mut self.st,
            teach: Some([false; 3]),
            reset: std::cell::Cell::new(false),
            stream: &mut self.stream,
        };
        f(&mut h)
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
            let iso = self.stream.iso.clone().unwrap_or_default();
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
    #[cfg(test)]
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
    #[cfg(test)]
    pub fn calls(&self) -> &[(u64, String)] {
        &self.st.calls
    }

    /// Where the set-up is with the event task.
    pub fn setup(&self) -> Setup {
        self.setup
    }

    /// Whether the scripts play a stream.
    #[cfg(test)]
    pub fn streaming(&self) -> bool {
        self.stream.player.is_some()
    }

    /// The playing stream's number.
    #[cfg(test)]
    pub fn stream_num(&self) -> Option<usize> {
        self.stream.player.as_ref().map(|p| p.num())
    }

    /// The playing stream's frame (`ccGetStreamFrame`), once it has
    /// stepped.
    #[cfg(test)]
    pub fn stream_frame(&self) -> Option<u32> {
        self.stream.player.as_ref().and_then(|p| match p.frame() {
            (f, true) => Some(f),
            _ => None,
        })
    }

    /// `game.server`, for tests.
    #[cfg(test)]
    pub fn server(&self) -> i32 {
        self.world.scene().server
    }

    /// Whether this scene is a Root Town's (one is built), for tests.
    #[cfg(test)]
    pub fn in_town(&self) -> bool {
        self.world.town_type().is_some()
    }

    /// The scene a stream playing here plays (or is about to), for tests.
    #[cfg(test)]
    pub fn stream_scene(&self) -> Option<String> {
        self.stream.player.as_ref().and_then(crate::stream::StreamPlayer::entry_name)
    }

    /// What Quarantine's field staff roll (`STFROLL_VOL4`) asked this
    /// frame: its camera (`changeCamera(3)`, the position and view) and
    /// its music (`ccBgmPlay(2)`, `ccBgmStop`).
    fn ending_asks(&mut self) {
        use piney_world::world::EndingAsk;
        for a in self.world.fx_mut().take_ending_asks() {
            match a {
                EndingAsk::Camera { pos, view } => self.world.ending_camera(pos, view),
                EndingAsk::Bgm(n) => self.events.extend(usize::try_from(n).ok().map(Event::BgmStream)),
                EndingAsk::BgmStop => self.events.push(Event::BgmStreamStop),
            }
        }
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
        // From Mutation on the event task idles while the cast's tasks hold
        // it (a town's Flag Race, MUT main 0x001cb154).
        if self.setup == Setup::Play && self.world.holds_events() {
            return;
        }
        let teach = ev_host::teach_input(pad, self.world.camera().scheme.ctrl_type);
        let reset = {
            let reset = std::cell::Cell::new(false);
            let mut h = EvHost {
                world: &mut *self.world,
                ui: &mut self.ui,
                st: &mut self.st,
                teach: Some(teach),
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
                let (teach, reset) = (Some([false; 3]), std::cell::Cell::new(false));
                let mut h = EvHost {
                    world: &mut *self.world,
                    ui: &mut self.ui,
                    st: &mut self.st,
                    teach,
                    reset,
                    stream: &mut self.stream,
                };
                vm.start_thread(&mut h);
                // WORLD_MAN::SetEventData: a story dungeon's rooms and
                // positions as the event manager's points and positions.
                if let Some(ed) = self.world.area().event_data(self.world.scene().area) {
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
            }
            Setup::Pass(p) => {
                // `ccEnableThEvent` breathes until the pass settles, however
                // long its streams and waits run (QUA main 0x001c8620).
                if !vm.enable_settled(p) {
                    return;
                }
                // A pass that asked for another mode (ChangeRequest's
                // ccDisableThEvent): ccSetupGameCtrl reads game+4 and
                // returns (0x00168ca8, 0x00168fa4).
                if vm.mng.enable_phase < 0 {
                    self.setup = Setup::Left;
                    return;
                }
                if p == 0 {
                    // ccRegisterEventMng(0) from every area's file list
                    // (INF main 0x00169690, 0x001697a0, 0x001698b0), between
                    // the passes: the count the walking PCs' draw reserves.
                    self.world.set_regist_npc_num(vm.mng.register_npcs());
                    vm.enable(2);
                    self.setup = Setup::Pass(2);
                } else {
                    // ccEntryEventMng: the entries the passes registered
                    // (party members are the registry's): what the cast
                    // places itself (a town's NPCs), then the magic portals
                    // (entry_mc) and the rest (enemies, entry 5, 6) to the
                    // entry control when the tasks start.
                    let mc: Vec<[i16; 4]> = vm.mng.entries_mc.iter().copied().filter(|e| e[0] >= 0).collect();
                    let en: Vec<[i16; 4]> = vm.mng.entries.iter().copied().filter(|e| e[0] >= 3).collect();
                    let en: Vec<[i16; 4]> = en.into_iter().filter(|&e| !self.world.event_entry(e)).collect();
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

    /// What `ccMenuCtrl` reads of the area this frame
    /// ([`piney_fieldui::World`]): `ccGame` (status 5, the scene,
    /// `areaLevel`), the party's panels from the battle's characters (Kite
    /// named by the save's `plName`, the others by `charTbl`), the command
    /// target and the one before it, `cmndSortRoot`'s chain, the three
    /// command lists, `game.inBattle`, the player's skill and attack, the
    /// registry, and what the cast's characters show (a town's NPCs, its
    /// Grunty, its race).
    pub(crate) fn menu_world(&self) -> piney_fieldui::World {
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
        // A battle character as the battle has it, else the cast's own.
        let who = |id: CharId| match self.world.scene_index(id) {
            Some(i) => info(i),
            None => cast_info(&self.world, id),
        };
        let t = self.world.targeting();
        let listed = |t: Option<CharId>| t.filter(|&t| self.world.listed(t)).and_then(who);
        let target = listed(t.target);
        let target_prev = listed(t.prev);
        let sorted = t.sorted.iter().filter_map(|x| who(x.0)).collect();
        let chain = |l: &[usize]| l.iter().filter_map(|&i| info(i)).collect::<Vec<_>>();
        // cmndObjRoot's chain: the battle's objects, then the cast's (a
        // town's Chaos Gate) as they stand on cmndSortRoot.
        let mut obj_chain = chain(&c.scene.obj_list);
        obj_chain.extend(t.sorted.iter().filter(|x| matches!(x.0, CharId::Object(_))).filter_map(|x| who(x.0)));
        let seen = self.world.seen();
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
                area_level: self.area_level,
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
            obj_chain,
            area_codes: vm.map_or([(-1, -1); 16], |vm| vm.mng.area_codes),
            event_status: vm.map_or(0, |vm| vm.mng.status),
            // eventMng.bossEntry (+0x08): the code `entry 7` made, which
            // keeps the battle banner off through a boss's fight.
            boss_entry: self.world.boss_entry().unwrap_or(-1),
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
            // The Grunty the menus read (cmndTargetPrev's, else cmndTarget's).
            grunty: seen.grunty.map(|g| piney_fieldui::menus::breeder::Grunty {
                handle: g.who.handle(),
                row: g.row,
                growth: g.growth,
                msg: g.msg,
                exist: g.exist,
                level: g.level,
                size: g.size,
                food_num: g.food_num,
                chat_flag: g.chat_flag,
                food_mode: g.food_mode,
            }),
            // ccSpcManager.registry[registryNum]: PartyInMenu calls a member
            // only while fewer than five are loaded or when it is one of
            // them (CheckSpc), and greets by its bootParam.
            spc: {
                let r = self.world.spcs();
                r.registry[..r.registry_num.clamp(0, r.registry.len() as i32) as usize]
                    .iter()
                    .map(|e| (e.id, e.boot_param))
                    .collect()
            },
            // g_entCtrl's NPC list (each entParam.id, its npcTbl row): who is
            // in town today, Ryu Book III's "Online".
            npcs: seen.npcs,
            race: seen.race.map(|r| piney_fieldui::menus::flag_race::RaceView {
                state: r.state,
                rank: r.rank,
                running: r.running,
                time: r.time,
            }),
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
                    let (listed, at) = self.world.chat_point(CharId::from_handle(h)?)?;
                    Some(piney_fieldui::chat_msg::ChatAt { who: h, listed, at })
                })
                .collect(),
        }
    }

    /// What the boss asked of the screen, the map and the menus this
    /// frame: `scFadeDef->EntryFlash`, `EVENTAREAB0::SwitchLayer`, the
    /// menus' lock and the cursor.
    /// The boss's counts ([`piney_battle::boss::Out::Count`]; a leaf's
    /// `ccBoss03Leaf::OnThinkCountDown`, MUT gcmn 0x004a6384): where
    /// `ccCalcTagPos` puts the place 100 up, the global font, type 1,
    /// `ccSpriteColorTable[7]`, shadowed (+4 bit 0x10), `MakeSignedNum(2,
    /// seconds)`.
    fn boss_counts(&mut self, shows: &[piney_world::combat::Show], ctx: &mut Ctx) {
        use piney_battle::boss::{Out, Spot};
        use piney_fieldui::spr::{Obj, Spr, font_type, make_signed_num};
        let mut font = Spr::new(Obj::SysFont, 256);
        for s in shows {
            // A count over a character (a leaf's, `OnThinkCountDown`) or a
            // place (a seed's, `ccBoss08Seed::OnThinkCountDown`, QUA gcmn
            // 0x003b1500), where it is then.
            let piney_world::combat::Show::Boss(_, Out::Count { at, n }) = s else { continue };
            let pos = match *at {
                Spot::Char(c) => {
                    let Some(pos) = self.world.combat().scene.chars.get(c).map(|c| c.pos) else { continue };
                    pos
                }
                Spot::At(pos) => pos,
            };
            let up = [0, 0, 0x42c8_0000, 0];
            let ws = &self.world.camera().world_screen;
            let Some((x, y, _)) = piney_world::chara::char::calc_tag_pos(ws, pos, up, 0) else { continue };
            font_type(&mut font, 1);
            font.set_colour(7);
            (font.dx, font.dy, font.shadow) = (x as f32, y as f32, true);
            make_signed_num(&mut font, 2, i64::from(*n));
        }
        let packets = font.take();
        if !packets.is_empty() && !self.world_hidden {
            self.ui.send_font(packets, self.world.state(), ctx);
        }
    }

    fn boss_calls(&mut self, shows: &[piney_world::combat::Show]) {
        use piney_battle::boss::Out;
        use piney_world::combat::Show;
        for s in shows {
            match s {
                Show::Boss(_, Out::Flash { t, colour }) => {
                    self.st.fade_def.flash(*t as i16, *colour);
                }
                // Corbenik's third form's death: EntryFade to black.
                Show::Boss(_, Out::EntryFade { t, from, to }) => {
                    self.st.fade_def.entry(*t as i16, *from, *to);
                }
                // Magus's EntryFlash2 (the rise) and EntryFlash3 (the laser).
                Show::Boss(_, Out::FlashFade { t_in, t_out, hold: None, colour }) => {
                    self.st.fade_def.flash2(*t_in as i16, *t_out as i16, *colour);
                }
                Show::Boss(_, Out::FlashFade { t_in, t_out, hold: Some(h), colour }) => {
                    self.st.fade_def.flash3(*t_in as i16, *t_out as i16, *h as i16, *colour);
                }
                Show::Boss(_, Out::SwitchLayer) => self.world.switch_layer(),
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
                Show::Boss(_, Out::DummyTarget(t)) => {
                    self.dummy_target = *t;
                    if t.is_none() {
                        self.ui.ctrl.dummy_target = None;
                    }
                }
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

    /// `ccGame::ChangeArea(area, n)` as a menu makes it (the console's
    /// `town`): the world's change of scene, whose `ChangeRequest` disables
    /// the event task and puts the tasks to sleep as any other does.
    pub fn change_area(&mut self, area: i32, n: i32) {
        if self.leaving.is_none() {
            self.world.change_area(area, n);
        }
    }

    fn talk(&mut self, t: TalkRequest) {
        // The talk and shop menus read the one spoken to (cmndTarget, its
        // npcTbl row) from before the step that opens them: an NPC, a party
        // member (21), an administrator (23), a breeder (27), a dog (44), a
        // Grunty (45 grown, 46 young).
        use piney_fieldui::talk::{Speaker, TalkTarget};
        use talk::Shop;
        let spoken_to = match t {
            TalkRequest::Talk { npc, .. } | TalkRequest::Shop { npc, .. } => Some(CharId::Npc(npc)),
            TalkRequest::Menu { menu: 21 | 23 | 27 | 44 | 45 | 46, who } => Some(who),
            _ => None,
        };
        let speaker = spoken_to.and_then(|who| match who {
            CharId::Member(code) => Some((who, Speaker::Spc(code))),
            // An event NPC's stand-in: its npcTbl row.
            CharId::Scene(i) => {
                let npcs = &self.world.combat().npcs;
                Some((who, Speaker::Npc(npcs.iter().find(|n| n.who == i).map_or(i as i32, |n| i32::from(n.code)))))
            }
            CharId::Npc(code) => Some((who, Speaker::Npc(code))),
            CharId::Ride | CharId::Object(_) => None,
        });
        if let Some((who, speaker)) = speaker {
            self.ui.talk_to(Some(TalkTarget { handle: who.handle(), who: speaker }));
        }
        let area = self.world.scene().area;
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
            TalkRequest::Menu { menu, .. } => open(menu as i16, talk::action_mode(menu, area)),
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
            (Ok(n), Some(data)) => crate::stream::StreamPlayer::drain(
                self.stream.iso.as_deref().unwrap_or(Path::new("")),
                data,
                n,
                &save,
                crate::stream::FileList::Field,
                &mut self.events,
            ),
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
        // A Ryu Book's cover stream (ccThBook's), a frame a menu frame until
        // it ends.
        if self.book_stream
            && let Some(p) = &mut self.stream.player
        {
            match p.step(&self.st.pad, &mut self.events) {
                Some(f) => self.stream.frame = Some(f),
                None => {
                    self.world.set_rand(p.rand());
                    self.stream.player = None;
                    self.book_stream = false;
                    self.ui.book_stream_done();
                }
            }
        }
        // Inside ccPuccigusoStart, the menu task's slot first.
        self.ride_menu_slot();
        // ccUseItemRequest's loop on a Fairy's Orb: this frame's call.
        if self.map_showing {
            self.map_showing = !self.item_show_map();
        }
        let w = self.menu_world();
        if let Some(i) = self.dummy_target {
            let save = self.world.state();
            let names = |id: i32| if id == 0 { save.save.name().to_vec() } else { self.world.char_name(id) };
            self.ui.ctrl.dummy_target = char_info(&self.world, i, &names);
        }
        // The menus draw from the game's one rand(), as the world left it.
        let (ui, count) = (&mut self.ui, self.world.count());
        self.world.with_live_state(|s| ui.step_into(pad, &w, s, count, ctx));
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
            let (ui, count) = (&mut self.ui, self.world.count());
            self.world.with_live_state(|s| ui.answer_item(steps, pad, s, count, Some(ctx)));
        }
        if self.world.targeting().in_menu && self.ui.menu_type() == -1 {
            self.world.close_menu();
        }
    }

    /// What a town's own characters started at the entry control's slot
    /// (Show::Cast): the race's asks, the gate's sounds, the dogs', the
    /// Grunties' and the walking PCs' sounds and lines (their effects are
    /// the effect task's).
    fn cast_out(
        &mut self,
        o: &piney_world::world::cast::Out,
        names: &piney_desktop::kanji::Names,
        ear: Option<piney_audio::se3d::Listener>,
    ) {
        use piney_world::world::cast::Out;
        match o.clone() {
            Out::Race(e) => self.race_event(e, ear),
            // The Chaos Gate's circle opening (71) and closing (72).
            Out::GateSound { n, pos } => {
                if let Ok(n) = usize::try_from(n) {
                    self.events.push(Event::Se3d { n, pos, note: None, ear });
                }
            }
            // Dun Loireag's dogs: inuCheckNote's ccSeSetParamInu.
            Out::Dog { e, .. } => {
                if let piney_world::town::dog::DogEvent::Note { param, pos, attribute } = e
                    && let Some(se) = piney_audio::se3d::inu_note(self.world.volume(), param, attribute)
                {
                    self.events.push(Event::Se3d { n: se.code, pos, note: se.note, ear });
                }
            }
            // Dun Loireag's Grunties: their notes, ccSeOn3Ds, voices and
            // lines.
            Out::Grunty { e, .. } => self.grunty_event(&e, ear),
            // A grown Grunty's line over its head.
            Out::Chat { code, text } => self.ui.open_chat(CharId::Npc(code).handle(), &text, names),
            // The walking PCs (ccRtownPC): chat lines, the crystal's and
            // the drain's sounds, their steps.
            Out::Pc { code, e } => {
                use piney_world::town::pc::PcEvent;
                match e {
                    PcEvent::Chat(text) => {
                        let text = piney_data::tables::sjis::encode(text);
                        self.ui.open_chat(CharId::Npc(code).handle(), &text, names);
                    }
                    PcEvent::Transfer => {}
                    PcEvent::Vanish { pos, crystal } => {
                        if crystal {
                            self.events.push(Event::Se3d { n: 167, pos, note: Some(40), ear });
                        }
                    }
                    PcEvent::Drained { .. } => {
                        self.events.push(Event::Se(79));
                        self.events.push(Event::Se(60));
                    }
                    PcEvent::Step { param, ccs_type, pos, attribute, .. } => {
                        if let Some(se) = piney_audio::se3d::pc_note(param, ccs_type, attribute) {
                            self.events.push(Event::Se3d { n: se.code, pos, note: se.note, ear });
                        }
                    }
                }
            }
        }
    }

    /// A Grunty's ask of the frame ([`piney_world::grunty::GruntyEvent`]):
    /// a sound at the listener, an effect to start, a voice (a grown
    /// kind's through `ccCheckVoiceGrp` of its row) or the chat balloon over
    /// the Grunty.
    fn grunty_event(&mut self, e: &piney_world::grunty::GruntyEvent, ear: Option<piney_audio::se3d::Listener>) {
        use piney_world::grunty::GruntyEvent as E;
        {
            match e.clone() {
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
                // The effects' (AreaFx's cast starts).
                E::Smoke { .. } | E::Evolve { .. } | E::Grow { .. } => {}
                E::Voice { grp, n } => {
                    let grp = if grp >= 141 {
                        piney_data::tables::fieldui::of(self.ui.texts().volume)
                            .voice_groups
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
                E::ChatClose => self.ui.close_chat(),
                // Carried out by the world (its line as Out::Chat); off the
                // lists (its own flag).
                E::Chat(_)
                | E::Camera(_)
                | E::CamPos(_)
                | E::CamView(_)
                | E::Player { .. }
                | E::Dropped
                | E::Debug(_) => {}
            }
        }
    }

    /// One of the Flag Race's asks (piney_world::grunty::race::RaceEvent):
    /// its sounds and music, the menu's panels, map and ban.
    fn race_event(&mut self, e: piney_world::grunty::race::RaceEvent, ear: Option<piney_audio::se3d::Listener>) {
        use piney_world::grunty::race::RaceEvent as R;
        {
            match e {
                R::Se(n) => self.events.push(Event::Se(n)),
                R::Se3d(n, pos) => {
                    if let Ok(n) = usize::try_from(n) {
                        self.events.push(Event::Se3d { n, pos, note: None, ear });
                    }
                }
                // The flags' sparks are the effects' (AreaFx's cast starts).
                R::OpenBox(_) => {}
                R::Bgm(n) => {
                    if let Ok(n) = usize::try_from(n) {
                        self.events.push(Event::BgmStream(n));
                    }
                }
                R::BgmStop => self.events.push(Event::BgmStreamStop),
                R::PgBgmInit => self.events.push(Event::PgBgm(piney_audio::PgBgm::InitTown)),
                R::PgBgmEnd => self.events.push(Event::PgBgm(piney_audio::PgBgm::RaceEnd)),
                R::Panel(n) => self.ui.ctrl.panel_status = n,
                R::Map(n) => self.ui.ctrl.map_status = n,
                R::Forbid(n) => self.ui.ctrl.forbid = n,
            }
        }
    }

    fn menu_request(&mut self, r: piney_fieldui::Request) {
        use piney_fieldui::Request as R;
        match r {
            // The gate menu's SimGenerateCode reseeds fieldrand.
            R::FieldSeed(seed) => self.world.set_field_seed(seed),
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
            R::ChangeArea { area, n } if self.leaving.is_none() => self.world.change_area(area, n),
            // TransFieldMenu (86): WORLD_MAN::GoField, from a dungeon to
            // its field.
            R::GoField if self.leaving.is_none() => {
                self.world.go_field();
            }
            // Asked again while leaving: the first change goes on.
            R::ChangeArea { .. } | R::GoField => {}
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
            R::Target(h) => self.world.change_command_target(CharId::from_handle(h)),
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
            // menus: a box opened (11, ItemBoxMenuT and the box menus), a
            // town's gate (the circle) or one of its characters.
            R::Affect { target, kind } => {
                if let Some(who) = CharId::from_handle(target) {
                    self.world.affect(who, kind, [0; 2]);
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
            // ccThBook's cover (the Key Items refuse a Ryu Book outside a
            // town, help 8), on the palette the town's str8800e last took;
            // a stream that cannot start counts as played.
            R::BookStream(Some(cover)) => self.book_start(cover),
            R::BookStream(None) => {
                if std::mem::take(&mut self.book_stream)
                    && let Some(p) = self.stream.player.take()
                {
                    self.world.set_rand(p.rand());
                }
            }
            R::DrainMovie(movie) => {
                self.stream_menu = false;
                // The boss's Affect(21), a request before this one, has
                // been carried out: its base->id is the one it left.
                let chars = &self.world.combat().scene.chars;
                let id = |h| handle_index(&self.world, h).and_then(|i| chars.get(i)).map(|c| c.base().id);
                let num = movie.stream(self.ui.texts().volume, id);
                self.start_movie(num);
            }
            R::StreamMenu(num) => {
                self.stream_menu = true;
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
            R::Talk(t) => self.talk_request(&t),
            R::Unported(n) => self.unported = Some(n),
            R::AreaLevel(v) => self.area_level = v,
            // The Flag Race: started (its task runs from this frame's slot),
            // quit from its pause, done with (its end begins).
            R::RaceStart(kind) => {
                if !self.world.ask(Ask::RaceStart(kind)) {
                    tracing::warn!("the flag race did not start");
                }
            }
            R::RaceQuit => {
                self.world.ask(Ask::RaceQuit);
            }
            R::RaceDone => {
                self.world.ask(Ask::RaceDone);
            }
            // CheckAreaCode's record: the words entered at the gate, which
            // the event task's next pass reads (areaCodeSet, cleared at its
            // top).
            R::AreaCodeSet(words) => {
                if let Some(vm) = &mut self.vm {
                    vm.mng.area_code_set = words;
                }
            }
            // The gate's SetGenerateCode: ChangeRequest's ccDisableThEvent,
            // then the session's change of scene to the words' area.
            R::GoToArea(words) => {
                let [a, b, c] = words;
                self.leaving = Some(format!("to the area of words {a} {b} {c}"));
                if let Some(vm) = &mut self.vm {
                    vm.disable();
                }
                self.events.push(Event::GoToArea(words));
            }
            // GtHackMenu's OK: gtHackFlag (ccSetGtHack, main 0x001b7840) and
            // ccGame.setupMode 1, for the next scene's set-up.
            R::GateHacked => self.world.set_gate_hack(true, true),
            // ccSndGateHack: the town's music out under the menu and
            // stopped, back if the hack is cancelled.
            R::GateHackSound(n) => self.events.push(Event::GateHackSound(n)),
            // The frozen layers: asleep, the world draws everyone where
            // they stand. Not carried out yet: the Grunty Flute's argument
            // use and Adjust Screen's offset; a member's CalcReal is the
            // battle's, every frame.
            R::Still(_)
            | R::KeepLayers
            | R::DeleteNoPartyMember
            | R::UseItem { .. }
            | R::UseItemArg { .. }
            | R::OpenChat(_)
            | R::CalcReal { .. } => {}
        }
    }

    /// The talk menus' asks of the world: the shop's camera on the merchant
    /// and back, the spring's camera and party, a Grunty's food and
    /// growth, a book given and read, the third member's remark.
    fn talk_request(&mut self, t: &piney_fieldui::talk::TalkReq) {
        use piney_fieldui::talk::TalkReq;
        match t.clone() {
            // SetMerchantCamera: changeCamera(3), ecam filled; a breeder's
            // view is the one for ccGame.server.
            TalkReq::MerchantCamera => {
                if let Some(who) = self.world.command_target_id() {
                    self.world.set_merchant_camera(who, self.world.scene().server);
                }
            }
            TalkReq::Camera(n) => self.world.change_camera(n as i16),
            TalkReq::FountainCamera => self.world.fountain_camera(),
            TalkReq::FountainParty(on) => self.world.fountain_party(on),
            // Give Food: EntryAffect(grunty, plw, 19, food, num, 0).
            TalkReq::Feed { target, food, num } => {
                if let Some(who) = CharId::from_handle(target) {
                    self.world.affect(who, 19, [food as i16, num as i16]);
                }
            }
            // The menus' writes to the Grunty (growthNum, foodMode,
            // chatFlag).
            TalkReq::GruntyGrowth { target, growth } => {
                if let Some(CharId::Npc(code)) = CharId::from_handle(target) {
                    self.world.ask(Ask::Growth { code, growth });
                }
            }
            TalkReq::GruntyFood { target, food_mode, chat_flag } => {
                if let Some(CharId::Npc(code)) = CharId::from_handle(target) {
                    self.world.ask(Ask::Food { code, food_mode, chat_flag });
                }
            }
            // AddSpcItem's ccUseItemRequest(spc, spc, code, 0): a book given
            // in a trade or as a present is read at once; what lasts is the
            // member's record.
            TalkReq::SpcUseItem { spc, code } => {
                if let Some(CharId::Member(id)) = CharId::from_handle(spc)
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
                if let Some(CharId::Member(id)) = CharId::from_handle(c) {
                    self.world.present_other(id);
                }
            }
            // The talk menus' other requests.
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }

    /// ccThBook's cover stream over the area, on the palette the town's
    /// `str8800e` last took; one that cannot start counts as played.
    fn book_start(&mut self, cover: piney_fieldui::book::Cover) {
        let save = self.world.state_out();
        self.cover_palette = cover.palette.or(self.cover_palette);
        let num = cover.stream;
        let started = match (self.stream.iso.clone(), self.stream.data.clone()) {
            (Some(iso), Some(data)) => usize::try_from(num).map_err(|_| format!("stream {num}")).and_then(|n| {
                crate::stream::StreamPlayer::cover(&iso, &data, n, &save, self.cover_palette, &mut self.events)
            }),
            _ => Err("no disc for the book's stream".into()),
        };
        match started {
            Ok(p) => {
                self.stream.player = Some(p);
                self.book_stream = true;
            }
            Err(e) => {
                tracing::warn!("the book's stream: {e}; counted as played");
                self.ui.book_stream_done();
            }
        }
    }
}

impl Mode for AreaMode {
    fn set_sys_frames(&mut self, frames: u32) {
        self.world.set_count(frames);
    }

    fn step(&mut self, pad: &Pad) -> Frame {
        self.frames += 1;
        self.st.pad = *pad;
        self.st.frame = self.frames;
        // The area's tasks gone (ccDeleteAllThread): only ccThGameOver.
        if let Some(t) = self.game_over.as_mut()
            && !t.field_runs()
        {
            return t.frame_alone(&mut self.events);
        }
        // ccThEvent (32) first.
        self.event_frame(pad);
        // A stream the scripts play runs inside the event task's call,
        // which breathes a frame at a time with no task asleep
        // (ccEventStream): the area's tasks go on, its frame is what shows.
        let stream_frame = self.stream.frame.take();
        let mut ctx = Ctx::new(View::default());
        self.world.set_menu_view(crate::world::menu_view(&self.ui));
        self.world.set_menu_type(self.ui.menu_type());
        // scFadeDef as the boss's task finds it.
        let fades =
            self.st.fade_def.elems.map(|e| piney_battle::boss::FadeView { status: e.status, cnt: e.cnt, tcnt: e.tcnt });
        self.world.set_fades(fades);
        // ccThPucciguso's slot (its Main runs with the area's tasks); the
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
        // Each scene's ccSetupGameCtrl makes the menu task, and its menu,
        // anew once the passes are made, before the step's GO and
        // rebootSpcManager draw (worklog 384). Its constructor counts the
        // grown Grunties in a town's pens (game.area 0: ccPgAdultCheck on
        // game.server, MUT gcmn 0x0053a398-0x0053a3c0).
        if self.world.tasks_start() {
            let scene = self.world.scene();
            let server = (scene.area == piney_world::area::kind::TOWN).then_some(scene.server);
            let (ui, party) = (&mut self.ui, self.world.party());
            self.world.with_live_state(|s| ui.menu_task_started(s, party, server));
        }
        // ccThMenu (34), before the area's other tasks: what it asks for is
        // acted on, and heard, this frame (worklog 336). It sleeps from a
        // change of scene's ChangeRequest (ccSleepNoSleepThread(1, 1)): a
        // gate's SetGenerateCode does not count the area once a frame
        // through the leave's fade (#22). Its party check is F0's, with the
        // tasks' set-up; its first Disp F1's (worklog 384).
        if let Phase::Play(_) = self.world.phase()
            && self.setup != Setup::Left
            && self.leaving.is_none()
        {
            self.menu_task(pad, &mut ctx);
        }
        // A Ryu Book's cover stream, stepped by the menu task: its frame
        // shows now, as the scripts' streams' do. Kept for the next frame's
        // check, it showed every other frame, with the area's frame
        // between, and half the presses never reached it (#21).
        if self.book_stream
            && let Some(f) = self.stream.frame.take()
        {
            return f;
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
        // ccThCamera (40) on, a town's Flag Race fading on the menu's
        // fader; hidden (Data Drain's black, its movie, the gate hack's
        // screen), the world draws nothing.
        {
            let mut unseen = Ctx::new(View::default());
            let wctx = if self.world_hidden { &mut unseen } else { &mut ctx };
            let mut fader = MenuFader(&mut self.ui.ctrl.menu_fade);
            self.world.step_into_with(pad, wctx, &mut fader);
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
        self.ending_asks();
        self.gate_stream(pad);
        self.member_item_steps();
        // What the tasks showed this frame, in their order: sounds and
        // lines as each comes (the cast's own characters' as Show::Cast),
        // then the effects', the boss's calls and the menus' rules.
        let shows = self.world.take_shows();
        // The Administrator's noise2 (ccMenu.interNoiz = 2).
        if self.world.take_noise() {
            self.ui.noise(2);
        }
        {
            use piney_world::combat::Show;
            let names = self.world.state().names();
            let cam = self.world.camera().active();
            let ear = Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
            for s in &shows {
                match s {
                    Show::Cast(o) => self.cast_out(o, &names, ear),
                    // ccPlayer::DamageActuate: Kite hit, the small motor and
                    // the hit's power for 100 ms.
                    Show::Kite(_, piney_battle::kite::Out::Actuate { power }) => {
                        self.events.push(Event::Actuate { small: true, power: *power as u8, ms: 100 });
                    }
                    s => {
                        let one = std::slice::from_ref(s);
                        show_sounds(&self.world, &mut self.ui, one, &mut self.events);
                        party_chats(&self.world, &mut self.ui, one);
                    }
                }
            }
        }
        fx_sounds(&mut self.world, &mut self.ui, &mut self.st, &mut self.events);
        // ccSoundMain's scene sounds: area 15's church music reads the
        // camera and the block; WORLD_MAN::GetTownType() the canals;
        // bgmBreed the scene's DMY_merchant6 (a town's Grunty breeder; no
        // other file has one).
        let scene = self.world.scene();
        let file = &self.world.area().core().file;
        let breeder = file.ccs.find_object("DMY_merchant6").and_then(|o| file.scene.dummies.get(&o));
        let scene_sound = piney_audio::scene::SceneInput {
            camera: Some(self.world.camera().active().pos),
            kite: self.world.kite().pos(),
            block: scene.block,
            town: self.world.town_type(),
            breeder: breeder.map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), 0x3f80_0000]),
        };
        self.events.push(Event::SceneSound(scene_sound));
        self.boss_calls(&shows);
        menu_rules(&self.world, &mut self.ui, &mut self.events, &shows);
        if let Phase::Play(f) = self.world.phase()
            && f >= 2
            && self.setup != Setup::Left
        {
            // ccThFieldDisp's DrawMap (a town's ROOTTOWN0n::Draw, WORLD::Draw
            // -> DrawMiniMap, DUNGEON::Draw -> MakeMiniMap), after ccThMenu
            // has set this frame's alpha.
            let (pos, dirc) = (self.world.kite().pos(), self.world.kite().dirc());
            let scene = self.world.scene();
            let mut ents = self.world.map_entries();
            // ccThFieldDisp sleeps from the frame a door or the stairs ask
            // for the change (ccSleepNoSleepThread in ChangeRequest), and
            // with every task (the gate hack's screen).
            let awake = !self.asleep && !self.world.asleep() && !self.world.scene_change_asked();
            let in_battle = self.world.combat().battle.in_battle != 0;
            let fonts = self.fonts.as_ref();
            // Hidden with the world (Data Drain's black and its movie).
            let mut unseen = Ctx::new(View::default());
            let map_ctx = if self.world_hidden { &mut unseen } else { &mut ctx };
            self.map_st.hud_scale = self.ui.hud_scale;
            let (area, statics) = self.world.map_parts();
            let mut f = piney_world::area::MapFrame {
                st: &mut self.map_st,
                statics,
                scene: &scene,
                pos,
                dirc,
                awake,
                in_battle,
                fonts,
                ents: &mut ents,
                ctx: map_ctx,
            };
            let shown = piney_world::map::area_frame(area, &mut f);
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
        // The global font, sent at the flip after the area's tasks.
        self.boss_counts(&shows, &mut ctx);
        // ccThGameOver, after the area's tasks (the same priority, started
        // later): its noise over the area, which it squeezes away.
        let over = self.game_over.as_mut().map(|t| {
            let events = &mut self.events;
            self.world.with_rands(|cc, rand| t.step(cc, rand, &mut ctx, events))
        });
        // The noise's view squeezes only the layers sharing sysLayer's.
        let mut frame = match over.as_ref().and_then(|s| s.view) {
            Some(v) => ctx.finish_with(|pri, older, c| {
                if crate::gameover::through_sys_view(pri, older) {
                    crate::gameover::squeeze(c, &v);
                }
            }),
            None => ctx.finish(),
        };
        // ccSys.bgColor as the area set it (a town's constructor, a
        // dungeon's fog).
        self.world.set_clear(&mut frame);
        // `ccSys.bgColor` 0 at the TV; a dungeon's `DUNGEON.fog` 0 with it,
        // so `DUNGEON::Draw` (gcmn 0x005cee68) finds them equal and never
        // puts the fog colour back: black to the end.
        if over.is_some_and(|s| s.black) {
            frame.clear = piney_draw::Rgba::BLACK;
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
        // A stream holds the screen: a menu's movie or the gate hack's,
        // else the scripts'.
        self.stream.frame.take().or(stream_frame).unwrap_or(frame)
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
                Request::Story(StoryRequest::PanelStatus(n)) => self.ui.ctrl.panel_status = n,
                Request::Game(GameRequest::SoundFadeOut) => out.push(Event::SoundFadeOut),
                Request::Game(GameRequest::AllSoundOff) => out.push(Event::AllSoundOff),
                Request::Game(GameRequest::EvVoiceStop) => out.push(Event::VoiceStop),
                // ccSndSQLoad: the area's bank as ccSetupGameCtrl picks it
                // (a town's sqDataTown[townType], or its crisis row). The
                // story area's model is GetEventAreaInfo(game.field)'s (main
                // 0x00169160), not WORLD_MAN's: Skeith's arena (field 1)
                // keeps area 27's WORLD_MAN. Piros is character 8 in a party
                // slot (checkPartyMenberNum(8)).
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
                // ccSndBgmCtrl once the fade in is over (its town case,
                // 0x0017b064, `bgm_plan`: Mac Anu in the crisis and Dun
                // Loireag with a third sequence start sequence 2, then every
                // town sequence 0).
                Request::Game(GameRequest::BgmCtrl) => {
                    let save = &self.world.state().save;
                    out.push(Event::BgmCtrl(piney_audio::BgmWorld {
                        scene_replaced: scene.changed(),
                        town: scene.town,
                        crisis: save.u8(offset::CRISIS) == 1,
                        dt_bgm: i32::from(save.u8(offset::DT_BGM) as i8),
                    }));
                }
                // Soft reset is not modelled; the transfers went to the
                // effects in the frame.
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
                    if self.leaving.is_none() {
                        self.leaving = Some(format!("ChangeScene({}, {})", scene.area, scene.town));
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

    /// `ccSys`'s rate: the scripts' `frame_rate` once one ran here (event
    /// 314's `frame_rate 1` in field 126), else the area's.
    fn frame_rate(&self) -> u32 {
        self.st.frame_rate.unwrap_or_else(|| self.world.frame_rate())
    }

    /// A stream's own files while the scripts play one, a menu's movie or
    /// a Ryu Book's cover.
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
        // Positions and speeds are EE float bits.
        let p = self.world.kite().pos().map(f32::from_bits);
        let s = self.world.scene();
        let place = match piney_world::town::TownDef::of(s.town, self.world.volume()) {
            Some(d) if s.area == piney_world::area::kind::TOWN => d.name.to_string(),
            _ => format!("area {} field {}", s.area, s.field),
        };
        let target = match self.world.command_target_id() {
            Some(id) => format!(" - target {id:?}"),
            None => String::new(),
        };
        let leaving = match &self.leaving {
            Some(to) => format!(" - leaving: {to}"),
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
                format!(" - ChangeScene({a}, {t}, {f}, {d}, {fl}, {b}){area}")
            }
            (Some(vm), None) => match (self.setup(), vm.playing()) {
                (Setup::Pass(p), _) => format!(" - events: the pass at phase {p}"),
                (_, Some((e, b))) => format!(" - event {e} block {b}"),
                _ => format!(" - events phase {}", vm.phase()),
            },
            (None, None) => String::new(),
        };
        let fade = self.st.fade.alpha().map_or(String::new(), |a| format!(" - fade {a}"));
        format!(
            "The World - {place} - {phase} - Kite at ({:.0}, {:.0}, {:.0}) act {} speed {:.1}{target}{menu}{leaving}{events}{fade}",
            p[0],
            p[1],
            p[2],
            self.world.kite().built().map_or(0, |k| k.ch.spc_char.act_num),
            f32::from_bits(self.world.kite().built().map_or(0, |k| k.spc.speed))
        )
    }
}

/// `ccMenu.menuFade` as the Flag Race fades on it.
struct MenuFader<'a>(&'a mut piney_demo::fade::ScFade);

impl piney_world::grunty::race::RaceHost for MenuFader<'_> {
    fn entry_fade(&mut self, n: i16, col1: u32) -> i32 {
        self.0.entry_fade(n, 0, col1)
    }
    fn continue_fade(&mut self, i: i32, n: i16, col1: u32) {
        if let Some(e) = usize::try_from(i).ok().and_then(|i| self.0.elm.get_mut(i)) {
            e.status = piney_demo::fade::DRAW;
            e.tcnt = n;
            e.cnt = 0;
            e.col0 = e.col1;
            e.col1 = col1;
        }
    }
    fn check_fade(&mut self, i: i32) -> bool {
        usize::try_from(i).is_ok_and(|i| self.0.check(i))
    }
    fn delete_fade(&mut self, i: i32) {
        if let Some(e) = usize::try_from(i).ok().and_then(|i| self.0.elm.get_mut(i)) {
            e.status = 0;
        }
    }
}

/// The scripts' instructions as a field, a dungeon or a story map answers
/// them: the characters are the battle's, the entries its entry control's.
impl HostWorld for World {
    fn state(&self) -> &SaveState {
        World::state(self)
    }

    fn state_mut(&mut self) -> &mut SaveState {
        World::state_mut(self)
    }

    fn state_out(&self) -> SaveState {
        World::state_out(self)
    }

    /// `ccGame` as the area's scene holds it, `cntStop` the scripts'.
    fn game(&self, st: &FieldState) -> piney_event::host::Game {
        let mut g = crate::field_host::game_of(&self.scene()).view();
        g.cnt_stop = st.game.cnt_stop;
        g
    }

    /// `ccGame::ChangeArea`: the world's change of scene.
    fn change_area(&mut self, _st: &mut FieldState, a: i32, n: i32) {
        World::change_area(self, a, n);
    }

    /// `ccGame::ChangeScene` (main 0x00167380) writes the one `ccGame`,
    /// the world's scene, which the session changes with the `area`
    /// before it (`SceneChange`); nothing reads it here before.
    fn change_scene(&mut self, _st: &mut FieldState, _scene: [i16; 6]) {}

    /// `ccEventStream(num, 1)`: the subtitles and `ccSndStreamCtrl`'s music
    /// by `ccGame`, the file list the area's set-up loaded
    /// (`ccSetFileListTown` for area 0, main 0x00169690; `Field` and
    /// `Dungeon`'s the same, 0x001697a0, 0x001698b0).
    fn stream_game(&self, _st: &FieldState) -> (piney_audio::stream::StreamGame, crate::stream::FileList) {
        let scene = self.scene();
        let game = piney_audio::stream::StreamGame { status: STATUS_WORLD, field: scene.field };
        let list = match scene.area {
            piney_world::area::kind::TOWN => crate::stream::FileList::Town,
            _ => crate::stream::FileList::Field,
        };
        (game, list)
    }

    fn set_rand(&mut self, rand: u64) {
        World::set_rand(self, rand);
    }

    fn server(&self) -> i32 {
        self.scene().server
    }

    /// `WORLD_MAN.hackFlag` 3: a hacked area, or any in the crisis (a
    /// town's `GO` sets it from the crisis byte).
    fn menu_noise(&self) -> bool {
        self.world_man().hack == 3
    }

    fn party(&self) -> [i32; 3] {
        World::party(self)
    }

    fn char_pos(&self, ty: i16, code: i16) -> Option<[u32; 4]> {
        World::char_pos(self, ty, code)
    }

    fn char_name(&self, _ui: &FieldUi, pc: i16) -> Vec<u8> {
        World::char_name(self, i32::from(pc))
    }

    /// `marker_pos`'s marker: an event map's `markerEvTbl` dummy.
    fn marker(&self, marker: i16) -> Option<piney_event::host::Marker> {
        World::marker(self, marker)
    }

    fn command_target(&self) -> Option<piney_event::host::CharRef> {
        World::command_target_ref(self)
    }

    fn target_alive(&self, target: &piney_event::host::CharRef) -> bool {
        World::target_alive(self, target)
    }

    /// `near_marker`'s distance from Kite to an event position.
    fn player_distance(&self, pos: [u32; 4]) -> u32 {
        World::player_distance(self, pos)
    }

    /// `camCtrlType`.
    fn camera_type(&self) -> Option<i32> {
        Some(self.camera().scheme.ctrl_type)
    }

    fn teach_camera(&mut self, part: u8) {
        World::teach_camera(self, part);
    }

    fn event_camera(&mut self, c: piney_event::host::CameraCommand) -> bool {
        World::event_camera(self, c);
        true
    }

    /// The event's NPCs outside the towns (`piney_world::field::npcs`).
    fn npc_command(&mut self, c: piney_event::host::NpcCommand) -> bool {
        World::npc_command(self, c)
    }

    fn pc_command(&mut self, c: piney_event::host::PcCommand) -> bool {
        World::pc_command(self, c)
    }

    fn set_trans(&mut self, ty: i16, code: i16, on: bool) -> bool {
        World::set_trans(self, ty, code, on)
    }

    fn set_affect_colour(&mut self, code: i32, fix: i16, rate: i16, colour: u32) -> bool {
        World::set_affect_colour(self, code, fix, rate, colour)
    }

    fn spc_loaded(&self, code: i32) -> bool {
        World::spc_loaded(self, code)
    }

    fn menu_ban(&mut self, on: bool) {
        self.menu_ban_camera(on);
        self.menu_ban_party(on);
    }

    /// `ccParty::AddMember(pc)` over the field's characters.
    fn party_add(&mut self, pc: i32) -> i32 {
        World::party_add(self, pc)
    }

    /// `ccParty::DelMember` of the member's slot; the member's character
    /// stays where it is, out of the party.
    fn party_remove(&mut self, pc: i32) {
        World::party_remove(self, pc);
    }

    /// `remove type code` ([`World::remove`]).
    fn remove(&mut self, ty: i16, code: i16) -> bool {
        World::remove(self, ty, code)
    }

    /// The character the area built for the registry id (S108's Sanjuro,
    /// S111's Natsume, a town's party), its handle as the HUD's view names
    /// it, and `ccGame`: all AddSpcItem reads of the area is game.area
    /// (AddSpcItemSub's town sale, INF gcmn 0x00527e0c, 0x00528004; the
    /// same on MUT, OUT, QUA).
    fn spc_item(&self, code: i32) -> Option<(piney_fieldui::World, u32)> {
        self.combat().who(code)?;
        let scene = self.scene();
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
        Some((w, (1 << 24) | u32::from(code as u16)))
    }

    fn game_over(&self) -> bool {
        self.compulsion_game_over
    }

    /// `gtHackFlag` cleared unless a field (or a dungeon of type 8 or 9)
    /// was entered from a town.
    fn clear_gt_hack(&mut self) -> Option<bool> {
        World::clear_gt_hack(self);
        Some(self.gt_hack())
    }

    fn condition_effect(&mut self, on: bool) {
        World::condition_effect(self, on);
    }

    fn entry_present(&self, list: piney_event::host::EntryList, ty: i16, code: i16) -> bool {
        World::entry_present(self, list == piney_event::host::EntryList::Gimmicks, ty, code)
    }

    /// `eventMng.bossEntry` (the boss's code) and its task's parameter.
    fn boss(&self) -> Option<piney_event::host::Boss> {
        let entry = self.boss_entry().unwrap_or(0);
        self.boss_task().map(|p| piney_event::host::Boss { entry, task_param: Some(p) })
    }

    fn no_active_object(&self) -> bool {
        World::no_active_object(self)
    }

    fn no_entries(&self) -> bool {
        World::no_entries(self)
    }

    /// The trap-removal effect at the command target (`effRemoveTrap(pos,
    /// -1, -1)`), with the frame's other shows.
    fn remove_trap(&mut self) -> bool {
        self.remove_trap_effect();
        true
    }

    /// `EntryAffect(cmndTarget, plw, 12, 0, 0, 0)`: the trap taken off the
    /// command target (none: nothing).
    fn remove_trap_done(&mut self) {
        if let Some(t) = World::command_target(self) {
            self.affect_from_player(t, 12);
        }
    }

    /// `open_door` / `close_door` in a dungeon (`DUNGEON::OpenDoor`,
    /// `CloseDoor2`) and `radiator` (event 30's bracelet); the others are
    /// not reached outside the towns yet.
    fn gimmick(&mut self, g: piney_event::host::GimmickCommand) -> bool {
        use piney_event::host::GimmickCommand;
        match g {
            GimmickCommand::OpenDoor => self.open_door(),
            GimmickCommand::CloseDoor => self.close_door(),
            GimmickCommand::Radiator { rtype, ty, code, x, y, z, roty, rotz } => {
                self.radiator(rtype, ty, code, x, y, z, roty, rotz)
            }
            _ => {}
        }
        true
    }

    /// `WORLD_MAN::RoomSelect(floor, block)`.
    fn room(&mut self, floor: i16, block: i16) -> bool {
        self.room_select(i32::from(floor), i32::from(block))
    }

    /// `WORLD_MAN::ShowMap`: the field's portals at once, or one room of the
    /// dungeon's a frame.
    fn show_map(&mut self) -> bool {
        World::show_map(self)
    }

    fn event_positions(&mut self, positions: &[piney_event::vm::EvPos]) {
        let to = |p: &piney_event::vm::EvPos| piney_battle::entry::EvPos {
            floor: p.floor,
            block: p.block,
            num: p.num,
            dirc: p.dirc.to_bits(),
            pos: p.pos.map(f32::to_bits),
        };
        self.set_event_positions(positions.iter().map(to).collect());
    }

    /// The enemy to the event position (its heading from the position the
    /// area's set-up took, when it is one).
    fn enemy_put(&mut self, enemy: i16, pos: [u32; 4]) -> bool {
        let dirc = World::event_positions(self).iter().find(|e| e.pos[..3] == pos[..3]).map(|e| e.dirc);
        World::enemy_put(self, enemy, pos, dirc)
    }

    /// `hold type code` / `hold_end`: the event's `ccThEvHold` task.
    fn hold(&mut self, target: Option<(i16, i16)>) -> bool {
        self.set_event_hold(target);
        true
    }

    fn battle_ready(&mut self) -> bool {
        World::battle_ready(self);
        true
    }

    /// `player_skill`'s set-up: `ccMenu +0xc` (`panelStatus`) 1, `+0x102`
    /// (`targetForbid`) 0, `cmndTargetFix` 1, the target's box withheld.
    fn player_skill_begin(&mut self, m: &mut piney_battle::evparty::SkillMenu) -> bool {
        World::player_skill_begin(self, m);
        true
    }

    fn player_skill_busy(&self) -> bool {
        World::player_skill_busy(self)
    }

    fn player_skill_step(&mut self, push: u32, m: &mut piney_battle::evparty::SkillMenu) {
        World::player_skill_step(self, push, m);
    }

    fn player_skill_end(&mut self, m: &mut piney_battle::evparty::SkillMenu) -> bool {
        World::player_skill_end(self, m);
        true
    }

    /// `gate_hack_anim`: while `ccCheckGtHackAnm()` (`ghoFlag`).
    fn gate_hack_anim(&self) -> Option<bool> {
        Some(World::gate_hack_anim(self))
    }

    fn ending_start(&mut self) -> bool {
        World::ending_start(self);
        true
    }

    /// `ending_kanji`: until `STFROLL_VOL4`'s tscb +0x14 is set.
    fn ending_running(&mut self) -> bool {
        World::ending_running(self)
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
    let mut disc = crate::open_disc(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
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
    let mut disc = crate::open_disc(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let tables = area_tables(&mut disc)?;
    let crisis = save.u8(offset::CRISIS) != 0;
    Ok(wm.with_event_number(tables, n, piney_data::area::flag71(save), crisis))
}

/// `SimGenerateCode`'s mark on `state.field_rand` (`AreaCode::field_rand`):
/// of the gate's `words`, or (`words` None) of story area `n`'s past 13 as
/// instruction 118 `area n` makes it; `n` 1-13 make nothing. On `server`.
pub fn sim_field_rand(iso: &Path, words: Option<[i32; 3]>, n: i32, server: i32, state: &mut SaveState) {
    use piney_data::area::{EvArea, ev_area, flag71, sim_generate_code};
    let Ok(tables) = crate::open_disc(iso).map_err(|e| e.to_string()).and_then(|mut d| area_tables(&mut d)) else {
        return;
    };
    let f71 = flag71(&state.save);
    let code = match words {
        Some([a, b, c]) => sim_generate_code(tables, a, b, c, server, f71),
        None => match ev_area(tables, n, server, f71) {
            EvArea::Generated(code) => Some(code),
            _ => None,
        },
    };
    if let Some(code) = code {
        state.field_rand = code.field_rand(state.field_rand);
    }
}

/// `WORLD_MAN::SetGenerateCode(a, b, c)` on `server` with `save`'s flags:
/// the area and the change of scene to it.
pub fn set_generate_code(
    iso: &Path,
    words: [i32; 3],
    server: i32,
    save: &piney_data::save::SaveData,
) -> Result<(WorldMan, piney_data::area::Go), String> {
    let mut disc = crate::open_disc(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let tables = area_tables(&mut disc)?;
    WorldMan::set_generate_code(tables, words, server, save)
        .ok_or(format!("the words {words:?} are not all in their tables"))
}

/// `ccMenuCtrl::GateoutMenu`'s end (menu 10, confirmed): back to the
/// town. The field's menus raise it; `--press F:gateout` too.
/// `ccSeOn3D` and the animation notes' `ccSeSetParamEnemy` /
/// `ccSeSetParamSPC` the battle's tasks called this frame, and the
/// effects' sounds, as [`Event::Se3d`] in order.
pub(crate) fn show_sounds(
    world: &World,
    ui: &mut FieldUi,
    shows: &[piney_world::combat::Show],
    events: &mut Vec<Event>,
) {
    use piney_audio::se3d;
    use piney_world::combat::Show;
    let w = world;
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
                let sc = world.scene();
                ui.df_comp(sc.area, world.world_man().field_type as i32, sc.dungeon);
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
            // ccEvVoiceRequest(event, n), a field voice group's line
            // (Fidchell's prediction), and ccEvVoiceStop.
            Show::Boss(_, piney_battle::boss::Out::Voice { event, msg }) => {
                out.push(Event::Voice { event: *event, msg: *msg });
                (None, [0; 4])
            }
            Show::Boss(_, piney_battle::boss::Out::VoiceStop) => {
                out.push(Event::VoiceStop);
                (None, [0; 4])
            }
            // Magus's ccSeOnNote, and its grow's ccSeOn3DLoop and
            // ccSeOffLoop on the id it keeps.
            Show::Boss(_, piney_battle::boss::Out::SeNote { se, note }) => {
                out.push(Event::SeNote { n: (*se).max(0) as usize, note: *note as i8 });
                (None, [0; 4])
            }
            Show::Boss(_, piney_battle::boss::Out::SeLoop { se, op }) => {
                if let Ok(n) = usize::try_from(*se) {
                    out.push(match *op {
                        piney_battle::boss::LoopOp::On(pos) => Event::SeLoopOn { n, pos, ear },
                        piney_battle::boss::LoopOp::Off { forget } => Event::SeLoopOff { n, forget },
                    });
                }
                (None, [0; 4])
            }
            // The riding Grunty's notes: ccSeSetParamInu(param, pcgs).
            Show::Ride(piney_battle::ride::Out::Sound { param, attribute }) => {
                let at = c.ride.obj.as_ref().map_or([0, 0, 0, 0x3f80_0000], |o| o.ride.pos);
                (se3d::inu_note(v, *param, *attribute), at)
            }
            // The Administrator's act -5: ccSeOn(217).
            Show::Npc(_, piney_world::town::merchant::SysopEvent::Vanish) => {
                out.push(Event::Se(217));
                (None, [0; 4])
            }
            Show::SkillSounds { caster, target_pos, heal, notes } => {
                if *heal {
                    out.push(Event::Se3d { n: 75, pos: *target_pos, note: None, ear });
                }
                for &v in notes {
                    if let Some(se) = caster.and_then(|c| spc(c, v)) {
                        out.push(Event::Se3d { n: se.code, pos: caster.map_or(*target_pos, pos), note: se.note, ear });
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
    events.extend(out);
}

/// The effects' sounds (heard from the active camera), flashes, shakes
/// and noises, then the area's own draw's calls (a field's thunder, steam,
/// TOBJ's hum, a dungeon's doors, a town's airship), after the shows'.
pub(crate) fn fx_sounds(world: &mut World, ui: &mut FieldUi, st: &mut FieldState, events: &mut Vec<Event>) {
    let cam = world.camera().active();
    let ear = Some(piney_audio::se3d::Listener { pos: cam.pos, view: cam.view, kind: cam.kind });
    let mut out = Vec::new();
    for (se, p, note) in world.fx_mut().take_sounds() {
        match (usize::try_from(se), p) {
            (Ok(n), Some(pos)) => out.push(Event::Se3d { n, pos, note, ear }),
            (Ok(n), None) => out.push(match note {
                Some(note) => Event::SeNote { n, note },
                None => Event::Se(se),
            }),
            _ => {}
        }
    }
    for (t, colour) in world.fx_mut().take_flashes() {
        st.fade_def.flash(t as i16, colour);
    }
    for s in world.fx_mut().take_shakes() {
        world.camera_shake(s);
    }
    for bs in world.fx_mut().take_noises() {
        ui.noise_bs(bs);
    }
    // An area's calls: a field's thunder's sound and flash, its steam's,
    // TOBJ's hum, a dungeon's doors.
    for call in world.take_ambient_calls() {
        use piney_world::systems::MapEvent;
        match call {
            MapEvent::Se(n) => out.push(Event::Se(n)),
            MapEvent::SeLoopStart => out.push(Event::TobjSeLoopStart),
            MapEvent::SeLoop(pos, rate) => {
                if let Some(ear) = ear {
                    out.push(Event::TobjSeLoop { pos, rate, ear });
                }
            }
            MapEvent::Se3d { n, pos } => {
                if let Ok(n) = usize::try_from(n) {
                    out.push(Event::Se3d { n, pos, note: None, ear });
                }
            }
            MapEvent::Flash { frames, colour, .. } => {
                st.fade_def.flash(frames as i16, colour);
            }
            MapEvent::Smoke(_) | MapEvent::SmokeN { .. } => {}
        }
    }
    events.extend(out);
}

/// The chat balloons this frame, in the order the tasks opened them:
/// each member's `ChatMessageSender` `ccChatMsg::OpenChat` over it
/// (handle `1 << 24 | id`), and from Mutation on the riding Grunty's
/// line of its search over itself ([`CharId::Ride`]).
pub(crate) fn party_chats(world: &World, ui: &mut FieldUi, shows: &[piney_world::combat::Show]) {
    use piney_world::combat::Show;
    let names = world.state().names();
    for s in shows {
        match s {
            Show::Chat(w, text) => {
                let Some(id) = world.combat().scene.chars.get(*w).map(|c| c.id()) else { continue };
                ui.open_chat((1 << 24) | u32::from(id as u16), text, &names);
            }
            Show::Ride(piney_battle::ride::Out::Chat(text)) => ui.open_chat(CharId::Ride.handle(), text, &names),
            _ => {}
        }
    }
}

/// What the battle's rules ask of the menu this frame:
/// `ccMenuCtrl::SetPanelBure(slot, n)` (a party panel shakes as its
/// member is hit) and `SetProtect(state, ch)` (the protect marks over a
/// character whose protect a hit broke, or which it regained).
pub(crate) fn menu_rules(
    world: &World,
    ui: &mut FieldUi,
    events: &mut Vec<Event>,
    shows: &[piney_world::combat::Show],
) {
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
                let kt = &world.combat().data.kt;
                if let Some(piney_battle::kite::Out::Actuate { power }) =
                    piney_battle::kite::damage_actuate(kt, act, value)
                {
                    events.push(Event::Actuate { small: true, power: power as u8, ms: 100 });
                }
            }
            Rule::PanelBure { slot, n } => ui.ctrl.set_panel_bure(slot, n as u8),
            Rule::SetProtect { state, on } => {
                let who = match on {
                    Who::Char(c) => Some(c),
                    Who::Me => me,
                    Who::Target | Who::Nobody => None,
                };
                let save = &world.state().save;
                let name = |id: i32| if id == 0 { save.name().to_vec() } else { world.char_name(id) };
                if let Some(info) = who.and_then(|w| char_info(world, w, &name)) {
                    ui.ctrl.set_protect(state as i16, &info);
                }
            }
            _ => {}
        }
    }
}
