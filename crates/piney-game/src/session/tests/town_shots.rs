//! Headless shots of a Root Town from where a log-in puts Kite, turned
//! about him in eight steps: `PINEY_TOWN=N PINEY_SHOTS=DIR cargo test
//! --release -p piney-game town_shots -- --ignored --nocapture` (town 0-4,
//! default 4, Lia Fail; shots under `/mnt/data/claude/scratch/towns/shots`).

use std::path::PathBuf;

use piney_input::{Buttons, Raw};

use super::*;

fn still(buttons: Buttons) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

#[test]
#[ignore]
fn town_shots() {
    use piney_event::host::PcCommand;
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let town: u8 = std::env::var("PINEY_TOWN").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/towns/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, town);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive.clone(), None, state, None, scene, None).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
    let mut pad = Pad::default();
    let mut step = |s: &mut Session, b: Buttons| {
        pad.read(&still(b));
        let f = s.step(&pad);
        s.take_events();
        f
    };
    let placed = |s: &Session| match &s.stage {
        Stage::World(w) => matches!(w.world().phase(), piney_world::Phase::Play(f) if f >= 2),
        _ => false,
    };
    while !placed(&s) {
        step(&mut s, Buttons::NONE);
    }
    for _ in 0..90 {
        step(&mut s, Buttons::NONE);
    }
    for k in 0..8i32 {
        {
            let Stage::World(w) = &mut s.stage else { panic!("left the town") };
            let dirc = (k * 8192 - 32768) as i16;
            assert!(w.world_mut().pc_command(PcCommand::Turn { pc: 0, dirc, chg: 0 }));
        }
        step(&mut s, Buttons::R2);
        let mut frame = step(&mut s, Buttons::NONE);
        for _ in 0..45 {
            frame = step(&mut s, Buttons::NONE);
        }
        gs.set_overlay(Mode::archive(&s));
        gs.render(&frame);
        let (w, h) = gs.target_size();
        let path = format!("{dir}/town{town}-{k}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
        println!("{path}: {}", Mode::title(&s));
    }
}

/// A walking PC's menu (`PcMenu`, 22: Talk / Trade) in Mac Anu, shot
/// headless once it is up: Kite walks to the nearest walking PC until it
/// is the command target and presses the action button. Compare with a
/// PS2 capture of the same menu (the name frame at y 96, the list under
/// it from y 114, no HP or SP, the target cursor still on the PC).
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game pc_menu_shot --
/// --ignored --nocapture`.
#[test]
#[ignore]
fn pc_menu_shot() {
    use piney_world::entry::{Kind, Npc};
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/towns/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 0);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive.clone(), None, state, None, scene, None).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
    let mut pad = Pad::default();
    let mut opened = false;
    let mut frame = None;
    for f in 0..3000u32 {
        let raw = {
            let Stage::World(w) = &s.stage else { panic!("left the town") };
            let world = w.world();
            let ui = &w.ui().ctrl;
            if ui.menu == 22 && ui.menu_status == 2 {
                opened = true;
            }
            let placed = matches!(world.phase(), piney_world::Phase::Play(n) if n >= 60);
            let kite = world.player().body.pos.map(f32::from_bits);
            let near = world
                .pcs()
                .iter()
                .map(|p| (p.code(), p.char.pos.map(f32::from_bits)))
                .min_by_key(|(_, q)| ((q[0] - kite[0]).powi(2) + (q[1] - kite[1]).powi(2)) as i64);
            match near {
                _ if !placed || opened => still(Buttons::NONE),
                Some((code, _)) if world.command_target() == Some((Kind::Npc, code)) => {
                    still(if f % 20 == 0 { Buttons::CROSS } else { Buttons::NONE })
                }
                Some((_, q)) => {
                    let (dx, dy) = (q[0] - kite[0], q[1] - kite[1]);
                    super::stick_toward(f32::from_bits(world.camera().active().rot[2]), dx.atan2(-dy))
                }
                None => still(Buttons::NONE),
            }
        };
        pad.read(&raw);
        let fr = s.step(&pad);
        s.take_events();
        if opened {
            frame = Some(fr);
            for _ in 0..40 {
                pad.read(&still(Buttons::NONE));
                frame = Some(s.step(&pad));
                s.take_events();
            }
            break;
        }
    }
    let frame = frame.expect("no walking PC's menu opened");
    gs.set_overlay(Mode::archive(&s));
    gs.render(&frame);
    let (w, h) = gs.target_size();
    let path = format!("{dir}/pc-menu.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
    println!("{path}: {}", Mode::title(&s));
}
