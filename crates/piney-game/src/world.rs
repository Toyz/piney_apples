//! Mode 5, `ccSetupNewGame` (loading `GCMN.PRG`): the new game's save and
//! event task, and what every scene of mode 6 shares with it: where
//! `ccSetupGameCtrl` is with the event task ([`Setup`]; the passes 0, 2, then
//! 4 at F0 are in docs/engine/event-vm.md) and what `ccThGameCtrl` reads of
//! `ccMenu`. Mode 6 itself, a Root Town as a field or a dungeon, is
//! [`crate::area::AreaMode`].

use piney_data::iso::Iso;
use piney_desktop::SaveState;
use piney_event::host::LogHost;
use piney_event::state::ScriptSave as _;
use piney_event::vm::Vm;
use piney_fieldui::FieldUi;
use piney_world::event::talk;

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

/// Frames [`new_game_events`]' passes, run at once, may take before it
/// reports them stuck.
const PASS_FRAMES: u32 = 10_000;

/// A player's New Game as the boot and the title make it: `ccSaveData::Init`,
/// `ccThMother`'s `ccStartEvent(volumeNum, 0)` ([`crate::desktop::boot`]:
/// a later volume's earlier events brought forward, event 11's bracelet
/// among them), the title's `NewGame(1)` and `NewGame(0)`
/// (`piney_fieldui::newgame::new_game_on`) and the port's Helba mail. The
/// save and the boot's event task; Log in adds `ccSetupNewGame`'s part.
pub fn new_game(disc: &mut Iso) -> Result<(SaveState, Vm), String> {
    let text = piney_desktop::InitText::from_disc(disc).map_err(|e| format!("ccSaveData::Init: {e}"))?;
    let mut state = SaveState::fresh_with(&text);
    let vm = crate::desktop::boot(disc, &mut state)?;
    piney_fieldui::newgame::new_game_on(&mut state, disc, false)?;
    piney_desktop::extras::new_game(&mut state.save, disc.volume().map_err(|e| e.to_string())?);
    Ok((state, vm))
}

/// [`new_game`]'s save, as `--mode world` enters with it.
pub fn new_game_state(disc: &mut Iso) -> Result<SaveState, String> {
    new_game(disc).map(|(state, _)| state)
}

/// [`new_game`]'s save and event task as a new game brings them to Log in
/// (for `--mode world`): the desktop's passes and play until mails 4 and 5
/// are read and event 1's block 2 has closed it; the board's
/// `ccStartThEvent` and passes, and Log in. Every window and wait answers
/// at once (`LogHost`).
pub fn new_game_events(disc: &mut Iso) -> Result<(SaveState, Vm), String> {
    let (mut state, mut vm) = new_game(disc)?;
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
    Ok((state, vm))
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
pub(crate) fn repair_buttons(state: &mut SaveState) {
    let at = |k: usize| piney_desktop::save::ASSIGN_PAD + 2 * k;
    let s = &mut state.save;
    let others_zero = (0..11).filter(|&k| k != 5 && k != 6).all(|k| s.i16(at(k)) == 0);
    if others_zero {
        for (k, b) in INIT_BUTTONS.iter().enumerate() {
            s.set_i16(at(k), *b as i16);
        }
    }
}

/// What `ccThGameCtrl` reads of `ccMenu`.
pub(crate) fn menu_view(ui: &FieldUi) -> talk::MenuView {
    let c = &ui.ctrl;
    talk::MenuView {
        idle: ui.menu_type() == -1 && c.open_req & 0xfff != 74,
        forbid: c.forbid != 0,
        forbid_chat_except: c.forbid_chat_except != 0,
        pl_attack: c.pl_attack != 0,
    }
}

#[cfg(test)]
mod tests {

    use std::path::Path;
    use std::sync::Arc;

    use piney_data::archive::Archive;
    use piney_data::save::offset;
    use piney_input::{Buttons, Pad, Raw};
    use piney_world::World;

    use super::*;
    use crate::area::AreaMode;
    use crate::mode::{Event, Mode};

    /// `ccSetupNewGame` and `ccSetupGameCtrl` in the town the save logs in
    /// to, the mode over it.
    fn enter(iso: &Path, archive: &Arc<Archive>, state: SaveState, vm: Option<Vm>) -> AreaMode {
        let mut disc = Iso::open(iso).unwrap();
        let world = World::log_in(&mut disc, archive.clone(), state).unwrap();
        AreaMode::enter(iso, archive, world, vm).unwrap()
    }

    /// A player for event 2's windows and menus: CROSS to type out and
    /// close each line; in the tutorials TRIANGLE for PERSONAL and the
    /// cursor to the one row that goes on (Party, Add, New Keyword), CROSS
    /// for the rest.
    fn player(mode: &AreaMode, f: u64) -> Buttons {
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
    fn play_event_2() -> Option<(AreaMode, u64, Vec<Event>, Vec<(u64, Buttons)>)> {
        let iso = crate::check_image("infection");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return None;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let (state, vm) = new_game_events(&mut disc).unwrap();
        let mut mode = enter(&iso, &archive, state, Some(vm));
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
        let iso = crate::check_image("infection");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let state = new_game_state(&mut disc).unwrap();
        let mut mode = enter(&iso, &archive, state, None);
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
        assert!(events.iter().any(
            |e| matches!(e, Event::SceneSound(s) if s.town == Some(piney_audio::WATER_TOWN) && s.camera.is_some())
        ));
    }

    /// The Equipment menu's new blades in the town: `ChangeEquip`'s
    /// `ChangeWeapon` hangs the save's weapon file on Kite's hands.
    #[test]
    fn new_blades_in_his_hands() {
        use piney_battle::param::SpcParam;
        let iso = crate::check_image("infection");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let state = new_game_state(&mut disc).unwrap();
        let mut mode = enter(&iso, &archive, state, None);
        let t = piney_battle::tables::Tables::read(&mut disc).unwrap();
        let (now, _) = piney_world::chara::body::weapon_of(&t, &mode.world().state().save, 0).expect("Kite's blades");
        let (i, other) = (0..64)
            .find_map(|i| {
                let n = String::from_utf8(t.job_weapon(0, i)?.ccsname.clone()).ok()?;
                (!n.is_empty() && n != now).then_some((i, n))
            })
            .expect("another blade");
        let mut k = SpcParam::from_save(&mode.world().state().save, 0);
        k.equipment[4] = i as i16;
        k.store(&mut mode.world_mut().state_mut().save, 0);
        mode.world_mut().change_equip(0, 0);
        let kite = mode.world().kite_model();
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
        let iso = crate::check_image("infection");
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
        let mut mode = enter(&iso, &archive, state, None);
        for _ in 0..40 {
            mode.step(&Pad::default());
        }
        let w = mode.menu_world();
        let kite = w.party[0].as_ref().expect("Kite in slot 0");
        assert_eq!(kite.condition[cond::CRITICAL], crit);
    }

    #[test]
    fn a_new_game_s_save_is_init_then_new_game() {
        let iso = crate::check_image("infection");
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
        let iso = crate::check_image("infection");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let (state, vm) = new_game_events(&mut disc).unwrap();
        let s = &state.save;
        assert_ne!(s.flags(0) & piney_event::state::DONE, 0, "event 0 done at boot");
        assert_ne!(s.flags(1) & piney_event::state::DONE, 0, "event 1 done");
        assert_eq!(s.flags(2), 0, "event 2 not begun");
        assert!([4, 5].iter().all(|&m| s.mail(m) >= 4), "mails 4 and 5 read");
        assert_eq!(vm.operate(), 0, "event 1 released the operations");
        assert_eq!(vm.phase(), -1, "Log in disabled the task");
    }

    /// Event 2 in Mac Anu, the town's characters' events taken in the frame
    /// their tasks made them (worklog 576): none waits for a later frame.
    #[test]
    fn event_2_s_characters_events_are_taken_in_their_frame() {
        let iso = crate::check_image("infection");
        if !iso.exists() {
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let (state, vm) = new_game_events(&mut disc).unwrap();
        let mut mode = enter(&iso, &archive, state, Some(vm));
        let mut pad = Pad::default();
        let mut f = 0u64;
        while mode.scene_change().is_none() && f < 20_000 {
            f += 1;
            pad.read(&Raw { buttons: player(&mode, f), ..Raw::default() });
            mode.step(&pad);
            mode.take_events();
            let at = mode.world().phase();
            let shows = &mode.world().combat().shows;
            assert!(shows.is_empty(), "frame {f} ({at:?}): the characters' outputs wait: {shows:?}");
        }
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
            (12, "clear_gate_hack (gtHackFlag 0)"),
            (13, "pc Mode { pc: -3, param: 6 }"),
            (15, "entry 2 2 3 5"),
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
        assert!(mode.world().combat().who(2).is_some());
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
        let job = mode.world().state().save.i16(offset::SPC_PARAM + offset::SPC_PARAM_SIZE * 2 + 0xd8);
        let cat = i32::from((job + 1).rem_euclid(6));
        let before = mode.world().state().save.clone();
        mode.with_host(|h| {
            assert!(h.add_spc_item(2, cat as i16, 3, 2), "Orca is in the town");
            assert!(!h.add_spc_item(8, cat as i16, 3, 1), "Piros is not");
        });
        let s = &mode.world().state().save;
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
        let mut own = fresh;
        own.save.set_i16(at(1), 0x80);
        repair_buttons(&mut own);
        assert_eq!(own.save.i16(at(1)), 0x80);
    }
}
