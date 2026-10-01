//! The gate hack (menu 62) in Mac Anu as a player plays it: the Chaos
//! Gate's Word List to the protected area 19 (Closed Oblivious Twin Hills,
//! two of virus core M), the cores put in, OK, and the area.

use std::f32::consts::PI;
use std::path::PathBuf;

use piney_fieldui::ctrl::After;
use piney_fieldui::menus::hack::Resume;
use piney_input::{Buttons, Raw};
use piney_world::entry::Kind;

use super::*;

/// The protected story area and its core (`protect` = [12, 2, 0, ...]).
const AREA: i16 = 19;
const CORE_M: usize = 12;
/// `charTbl` rows 1 and 10, whom event 18 takes to area 19.
const MIA: i32 = 1;
const ELK: i32 = 10;

/// `--mode story:19` (Mac Anu after event 18) with area 19 closed again,
/// in the Word List, and three of core M in Key Items.
fn story_19() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut start = crate::start::build(&iso, 19).unwrap();
    let s = &mut start.state.save;
    let p = s.i32(offset::PROTECT_AREA) as u32 & !(1 << AREA);
    s.set_i32(offset::PROTECT_AREA, p as i32);
    let listed = (0..64).any(|k| s.i16(offset::GATE_ORDER_LIST + 2 * k) == AREA);
    if !listed && let Some(k) = (0..64).find(|&k| s.i16(offset::GATE_ORDER_LIST + 2 * k) < 0) {
        s.set_i16(offset::GATE_ORDER_LIST + 2 * k, AREA);
    }
    s.set_u8(offset::IMP_ITEM_LIST + CORE_M, 3);
    Some(Session::resume(iso, archive, None, start.state, start.vm, start.at).unwrap())
}

fn still(buttons: Buttons) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// The left stick pushed so that the player turns to heading `h` under a
/// camera at rotation `cam_z` (as event 11's player does).
fn stick_toward(cam_z: f32, h: f32) -> Raw {
    let want = (cam_z - PI - h).rem_euclid(2.0 * PI);
    let mut best = (f32::MAX, still(Buttons::NONE));
    for k in 0..64 {
        let a = k as f32 * PI / 32.0;
        let (lx, ly) = ((128.0 + 127.0 * a.cos()) as u8, (128.0 + 127.0 * a.sin()) as u8);
        let raw = Raw { lx, ly, ..still(Buttons::NONE) };
        let mut p = Pad::default();
        p.read(&raw);
        let d = (p.dirc_l.rem_euclid(2.0 * PI) - want).abs();
        let d = d.min(2.0 * PI - d);
        if d < best.0 {
            best = (d, raw);
        }
    }
    best.1
}

/// Where the gate hack's own frames are, if it is running them.
fn resume(w: &WorldMode) -> Option<Resume> {
    match w.ui().ctrl.cont.map(|c| c.after) {
        Some(After::Hack(r)) => Some(r),
        _ => None,
    }
}

/// The player: to the Chaos Gate and speak to it; Word List, area 19,
/// Warp; at the cores' slots, up twice (two M in) and OK. Windows closed
/// every 24 frames.
fn player(w: &WorldMode, f: u64) -> Raw {
    let ui = w.ui();
    let c = &ui.ctrl;
    if resume(w) == Some(Resume::Select) {
        let slot = c.hack.as_ref().map_or(0, |h| h.set_num[0]);
        let b = match f % 16 {
            0 if slot < 2 => Buttons::UP,
            0 => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        return still(b);
    }
    if ui.menu_type() != -1 {
        if !f.is_multiple_of(8) {
            return still(Buttons::NONE);
        }
        let go_to_item = |item: i16| {
            let l = c.list();
            match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == item) {
                Some(row) if row as i16 == l.select => Buttons::CROSS,
                Some(row) if (row as i16) > l.select => Buttons::DOWN,
                Some(_) => Buttons::UP,
                None => Buttons::NONE,
            }
        };
        let row = {
            let save = &w.world().state().save;
            (0..64).map(|k| save.i16(offset::GATE_ORDER_LIST + 2 * k)).filter(|&v| v >= 0).position(|v| v == AREA)
        };
        let b = match (ui.menu_type(), c.proccess) {
            (28, 1) => go_to_item(59),
            (59, 2) => match row {
                Some(r) if r as i16 == c.list().select => Buttons::CROSS,
                Some(r) if r as i16 > c.list().select => Buttons::DOWN,
                Some(_) => Buttons::UP,
                None => Buttons::NONE,
            },
            (59, 4) => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        return still(b);
    }
    let waiting = w.calls().iter().rev().find_map(|(_, c)| {
        if c.starts_with("message_open") || c.starts_with("announce") {
            Some(true)
        } else if c.starts_with("message_check") {
            Some(false)
        } else {
            None
        }
    });
    if waiting == Some(true) {
        return still(if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
    }
    let world = w.world();
    if world.command_target() == Some((Kind::Gimmick, 16)) {
        return still(if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
    }
    let Some((gate, _)) = world.char_place(Kind::Gimmick, 16) else { return still(Buttons::NONE) };
    let g = gate.map(f32::from_bits);
    let p = world.player().body.pos.map(f32::from_bits);
    let cam_z = f32::from_bits(world.camera().rot()[2]);
    stick_toward(cam_z, (g[0] - p[0]).atan2(-(g[1] - p[1])))
}

/// Mac Anu's gate to area 19: the hack runs to its slots, two M go in, OK
/// opens the area (protectArea bit 19, the two cores gone, one left), and
/// the party leaves for the field.
#[test]
fn gate_hack_opens_area_19() {
    let Some(mut s) = story_19() else { return };
    let mut pad = Pad::default();
    let mut selected = false;
    let mut arrived = false;
    for f in 0..20_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => {
                selected |= resume(w) == Some(Resume::Select);
                player(w, f)
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 30)
        {
            arrived = true;
            break;
        }
    }
    assert!(selected, "the hack's slots never came: {}", Mode::title(&s));
    assert!(arrived, "not in area 19: {}", Mode::title(&s));
    let Stage::Area(a) = &s.stage else { unreachable!() };
    let w = a.world();
    assert_eq!((w.scene().area, w.scene().field), (1, i32::from(AREA)), "{}", Mode::title(&s));
    let save = &w.state().save;
    assert_ne!(save.i32(offset::PROTECT_AREA) as u32 & (1 << AREA), 0, "protectArea bit 19 not set");
    assert_eq!(save.u8(offset::IMP_ITEM_LIST + CORE_M), 1, "the two cores were not taken");
}

/// Pictures of the hack: the noise, the PROTECTED warning, the intro, the crystals,
/// the slots with two cores in, and the completion. `PINEY_SHOTS=DIR cargo
/// test --release -p piney-game gate_hack_shots -- --ignored --nocapture`
/// (default `/mnt/data/claude/scratch/gatehack`).
#[test]
#[ignore]
fn gate_hack_shots() {
    let Some(mut s) = story_19() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/gatehack".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut taken: Vec<String> = Vec::new();
    let mut one_in = None;
    let mut noisy = None;
    for f in 0..20_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::World(w) = &s.stage else {
            if !taken.is_empty() {
                break;
            }
            continue;
        };
        let r = resume(w);
        let c = &w.ui().ctrl;
        if one_in.is_none() && c.hack.as_ref().is_some_and(|h| h.set_num[0] == 1) {
            one_in = Some(f);
        }
        if noisy.is_none() && c.noiz.rn > 0 {
            noisy = Some(f);
        }
        let name = match r {
            _ if noisy.is_some_and(|at| f == at + 1) && c.noiz.rn > 0 => Some("0-noise".to_string()),
            _ if c.menu == 62 && c.proccess == 1 && c.wait_count == 10 => Some("1-protected".to_string()),
            Some(Resume::Intro(30)) => Some("2-intro".into()),
            Some(Resume::Crystals(50)) => Some("3-crystals".into()),
            Some(Resume::Select) if one_in.is_some_and(|at| f == at + 8) => Some("4-one-core".into()),
            Some(Resume::Complete(45)) => Some("5-complete".into()),
            _ => None,
        };
        if let Some(name) = name
            && !taken.contains(&name)
        {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{name}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            taken.push(name);
        }
    }
    assert!(taken.len() >= 6, "shots taken: {taken:?}");
}

/// A member's arrival: (hidden, on the command list, act) on its first
/// frame, and its own frames until shown.
type Arrival = ((bool, bool, i16), Option<u32>);

/// After the hack, the arrival in area 19 with `setupMode` 1: the Chaos Gate's
/// movie while the field loads (`str7100`, `str7200`, the loop `str7300`, the
/// arrival `str7404`), then Kite in act 24 with `ghoFlag` on `x7404cam`'s
/// markers, his blades hidden, act 23 as they fade in, then standing with the
/// field camera back and `ghoFlag` clear. Mia and Elk arrive hidden and
/// standing, off the command list, and appear 65 frames later.
#[test]
fn gate_hack_arrives_hacking_out() {
    let Some(mut s) = story_19() else { return };
    let mut pad = Pad::default();
    let mut scenes: Vec<String> = Vec::new();
    let mut acts: Vec<i16> = Vec::new();
    let (mut gho_seen, mut ecam_seen, mut arms_hidden, mut arms_fading) = (false, false, false, false);
    let mut stood = None;
    let mut joined = false;
    // Each member's first frame built: (hidden, listed, act); and the
    // frames until shown.
    let mut members: std::collections::BTreeMap<i32, Arrival> = Default::default();
    let mut built_at = None;
    for f in 0..30_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        if !joined {
            joined = true;
            let sp = a.world_mut().spcs_mut();
            for (slot, id) in [(1usize, MIA), (2, ELK)] {
                let i = sp.entry_spc(id) as usize;
                sp.registry[i].party_flag = 1;
                sp.member_id[slot] = id;
                sp.member_char[slot] = Some(id);
                sp.num += 1;
            }
        }
        let a = &*a;
        if let Some(e) = a.stream_scene()
            && scenes.last() != Some(&e)
        {
            scenes.push(e);
        }
        let w = a.world();
        let c = w.combat();
        let Some(k) = c.kite else { continue };
        let act = c.scene.chars[k].spc_char.act_num;
        if acts.last() != Some(&act) {
            acts.push(act);
        }
        built_at.get_or_insert(a.world().phase());
        for &(id, w) in c.members.iter().filter(|m| m.0 != 0) {
            let ch = &c.scene.chars[w];
            let shown = ch.spc_char.flags & (1 << 1) != 0;
            let listed = c.scene.pc_list.contains(&w);
            let e = members.entry(id).or_insert(((!shown, listed, ch.spc_char.act_num), None));
            if shown && e.1.is_none() {
                // Built in the frame that left Play(0); the phase read
                // after a frame is the next one's, so the member's own
                // frames (from Play(1)) number n - b - 1.
                e.1 = Some(match (built_at, a.world().phase()) {
                    (Some(piney_world::Phase::Play(b)), piney_world::Phase::Play(n)) => n - b - 1,
                    _ => 0,
                });
            }
        }
        let g = c.cast.gate.flag();
        gho_seen |= g;
        ecam_seen |= g && w.camera().cam_id == piney_world::camera::id::EVENT;
        if let Some(a) = c.cast.get(k).and_then(|a| a.arms) {
            arms_hidden |= a == 0.0;
            arms_fading |= a > 0.0 && a < 1.0;
        }
        if gho_seen && !g && stood.is_none() {
            stood = Some((act, w.camera().cam_id, w.gt_hack()));
        }
        if stood.is_some() && members.values().all(|m| m.1.is_some()) && members.len() == 2 {
            break;
        }
    }
    assert_eq!(scenes, ["str7100", "str7200", "str7300", "str7404"], "the gate's movie");
    let Stage::Area(a) = &s.stage else { panic!("not in area 19: {}", Mode::title(&s)) };
    let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
    assert!(calls.contains(&"clear_gate_hack (gtHackFlag 1)"), "{calls:?}");
    assert!(gho_seen, "ghoFlag never set; acts {acts:?}");
    assert!(ecam_seen, "the cutscene camera never ran");
    assert!(acts.starts_with(&[24, 23]), "Kite's acts {acts:?}");
    assert!(arms_hidden && arms_fading, "the blades: hidden {arms_hidden}, fading {arms_fading}");
    assert_eq!(stood, Some((2, piney_world::camera::id::FIELD, true)), "after GateHackingOut");
    let want: Vec<(i32, Arrival)> = vec![(MIA, ((true, false, 2), Some(65))), (ELK, ((true, false, 2), Some(65)))];
    assert_eq!(members.into_iter().collect::<Vec<_>>(), want, "the members (hidden, listed, act), shown after");
}

/// Pictures of the hacked arrival: the Chaos Gate's movie (Mac Anu's
/// departure, the loop, the arrival), Kite in act 24 under the cutscene
/// camera, act 23 with the blades fading in, and standing after.
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game
/// gate_hack_arrival_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/gatehack`).
#[test]
#[ignore]
fn gate_hack_arrival_shots() {
    let Some(mut s) = story_19() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/gatehack".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut taken: Vec<String> = Vec::new();
    let mut seen: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for f in 0..30_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { continue };
        let key = match a.stream_scene() {
            Some(sc) => sc,
            None => {
                let c = a.world().combat();
                let Some(k) = c.kite else { continue };
                let act = c.scene.chars[k].spc_char.act_num;
                let arms = c.cast.get(k).and_then(|x| x.arms);
                match (act, arms) {
                    (24, _) => "act24".into(),
                    (23, Some(t)) if t > 0.0 && t < 1.0 => "act23-arms".into(),
                    (23, _) => "act23".into(),
                    _ if !taken.is_empty() => "after".into(),
                    _ => continue,
                }
            }
        };
        let n = seen.entry(key.clone()).or_insert(0);
        *n += 1;
        // The 30th frame of each (the fade's first frame of "after").
        let want = if key == "act23-arms" { 3 } else { 30 };
        if *n == want {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/arrive-{}-{key}.png", taken.len());
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            taken.push(key.clone());
            if key == "after" {
                break;
            }
        }
    }
    println!("{taken:?}");
}

/// The session's sound events into a headless engine, as `main` routes
/// them (those a town and the gate hack raise).
fn hear(a: &piney_audio::Audio, events: Vec<Event>) {
    for e in events {
        match e {
            Event::SqLoad(ctx) => a.sq_load(ctx),
            Event::BgmCtrl(w) => {
                a.bgm_ctrl(w);
            }
            Event::GameStart => a.game_start(),
            Event::GameInterrupt => a.game_interrupt(),
            Event::AllSoundOff => a.all_sound_off(),
            Event::SceneSound(sc) => a.scene_sound(&sc),
            Event::GameArea(n) => a.set_game_area(n),
            Event::InBattle(on) => a.set_in_battle(on),
            Event::SqPlay(n) => a.sq_play(n),
            Event::SqStop(n) => a.sq_stop(n),
            Event::SqFade { seq, volume, time, mode } => a.sq_fade(seq, volume, time, mode),
            Event::GateHackSound(n) => a.gate_hack(n),
            Event::PortVolume { port, volume } => a.port_volume(port, volume),
            Event::MainVolume(v) => a.set_main_volume(v),
            Event::Volumes { main, se, bgm } => a.set_volumes(main, se, bgm),
            Event::HoldBgm => a.hold_bgm(),
            _ => {}
        }
    }
}

/// Issue #13: the gate hack silences Mac Anu's music. Heard on the way to
/// the Chaos Gate; `GtHackMenu`'s `ccSndGateHack(0)` (main 0x00180780) as
/// the menu opens fades sequences 0 and 2 out and stops them, so nothing
/// of the music sounds under the slots; cancelled there, `ccSndGateHack(1)`
/// starts them again at volume 0 and fades them up: the music is back.
#[test]
fn gate_hack_silences_the_town_s_music() {
    let Some(mut s) = story_19() else { return };
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let audio = piney_audio::Audio::headless(&iso).unwrap();
    let mut pad = Pad::default();
    let mut chunk = vec![0i16; 1600];
    // Mean squares: on the way to the gate, at the slots, after a cancel.
    let mut level = [(0f64, 0usize); 3];
    let (mut selecting, mut cancelled, mut after) = (0u32, false, 0u32);
    let mut playing_at_slots = None;
    for f in 0..20_000u64 {
        let phase = match &s.stage {
            Stage::World(w) if cancelled => {
                let open = w.ui().ctrl.menu == 62 || resume(w).is_some();
                after += u32::from(!open);
                (after > 120).then_some(2)
            }
            Stage::World(w) if resume(w) == Some(Resume::Select) => {
                selecting += 1;
                (selecting > 30).then_some(1)
            }
            Stage::World(w) if w.ui().menu_type() == -1 && f > 300 => Some(0),
            _ => None,
        };
        let raw = match &s.stage {
            Stage::World(_) if cancelled => still(Buttons::NONE),
            Stage::World(w) if resume(w) == Some(Resume::Select) && selecting >= 90 => {
                playing_at_slots.get_or_insert_with(|| audio.with_engine(|e| e.driver.sq_status));
                cancelled = true;
                still(Buttons::CIRCLE)
            }
            Stage::World(w) if resume(w) == Some(Resume::Select) => still(Buttons::NONE),
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        hear(&audio, s.take_events());
        audio.frame();
        audio.render(&mut chunk);
        if let Some(k) = phase {
            level[k].0 += chunk.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>();
            level[k].1 += chunk.len();
        }
        if level[2].1 >= 300 * chunk.len() {
            break;
        }
    }
    let rms: Vec<f64> = level.iter().map(|&(sq, n)| (sq / n.max(1) as f64).sqrt()).collect();
    eprintln!("rms on the way {:.0}, at the slots {:.0}, after the cancel {:.0}", rms[0], rms[1], rms[2]);
    assert!(rms[0] > 100.0, "no town music to begin with: {rms:?}");
    // Sequence 0 stopped (2, the crisis layer, is not loaded here).
    assert_eq!(playing_at_slots.map(|p| p[0]), Some(0), "sequence 0 at the slots: {playing_at_slots:?}");
    assert!(rms[1] < 1.0, "the music under the gate hack: {rms:?}");
    assert!(rms[2] > rms[0] / 2.0, "the music did not come back: {rms:?}");
}
