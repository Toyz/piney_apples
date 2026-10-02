//! Dun Loireag through the session: the Chaos Gate's Other Servers from Mac
//! Anu to Dun Loireag and back, as a player picks them, and `--mode
//! story:22`, where the story sends the player there: the board, Log in to
//! Mac Anu, the gate, and event 22's block for Dun Loireag.

use std::f32::consts::PI;
use std::path::PathBuf;

use piney_input::{Buttons, Raw};
use piney_world::entry::Kind;

use super::*;

/// The gate's menu, and its Other Servers page.
const GATE_MENU: i32 = 28;
const OTHER_SERVERS: i32 = 61;

fn disc() -> Option<(PathBuf, Arc<Archive>)> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    Some((iso, archive))
}

fn still(buttons: Buttons) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// The left stick pushed so that the player turns to heading `h` (radians,
/// `atan2(dx, -dy)` in the world) under a camera at rotation `cam_z`.
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

/// The town the session's World stands in, once its tasks run.
fn town_of(s: &Session) -> Option<i32> {
    match &s.stage {
        Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(f) if f >= 2) => {
            Some(w.world().town().base.no)
        }
        _ => None,
    }
}

/// A player at frame `f` going through the Chaos Gate to town `to`: a
/// waiting window closed every 24 frames; in the gate's menu, Other
/// Servers, the town, Warp; otherwise walking to the gate and speaking to
/// it once it is the command target.
fn to_the_gate(w: &WorldMode, f: u64, to: i32) -> Raw {
    let ui = w.ui();
    let c = &ui.ctrl;
    if ui.menu_type() != -1 {
        if !f.is_multiple_of(8) {
            return still(Buttons::NONE);
        }
        let l = c.list();
        let b = match (ui.menu_type(), c.proccess) {
            (GATE_MENU, 1) => {
                match l.items.iter().take(l.y.max(0) as usize).position(|&it| i32::from(it) == OTHER_SERVERS) {
                    Some(row) if row as i16 == l.select => Buttons::CROSS,
                    Some(row) if (row as i16) > l.select => Buttons::DOWN,
                    Some(_) => Buttons::UP,
                    None => Buttons::NONE,
                }
            }
            (OTHER_SERVERS, 2) => {
                // The gate's list: the towns of townMoveFlag but this one.
                let flag = i32::from(w.world().state().save.i16(offset::TOWN_MOVE_FLAG));
                let here = w.world().town().base.no;
                let towns: Vec<i32> = (0..5).filter(|&t| flag & (1 << t) != 0 && t != here).collect();
                match towns.iter().position(|&t| t == to) {
                    Some(row) if row as i16 == l.select => Buttons::CROSS,
                    Some(row) if (row as i16) > l.select => Buttons::DOWN,
                    Some(_) => Buttons::UP,
                    None => Buttons::CIRCLE,
                }
            }
            // "Warp" (the first of the two).
            (OTHER_SERVERS, 5) if l.select == 0 => Buttons::CROSS,
            (OTHER_SERVERS, 5) => Buttons::UP,
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

/// Frames of `to_the_gate(.., to)` until the session's World is town `to`
/// (at most `max`); the frame it got there.
fn warp(s: &mut Session, to: i32, max: u64) -> Option<u64> {
    let mut pad = Pad::default();
    for f in 0..max {
        if town_of(s) == Some(to) {
            return Some(f);
        }
        let raw = match &s.stage {
            Stage::World(w) if town_of(s).is_some() => to_the_gate(w, f, to),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    None
}

/// A new game in Mac Anu whose gate knows Dun Loireag (`townMoveFlag`
/// bits 0 and 1, as event 20's `town_move 1` leaves it): through the gate's
/// Other Servers to Dun Loireag, where Kite arrives at its start, and back
/// to Mac Anu the same way.
#[test]
fn the_gate_warps_to_dun_loireag_and_back() {
    let Some((iso, archive)) = disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_i16(offset::TOWN_MOVE_FLAG, 0b11);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
    let f = warp(&mut s, 1, 4000).unwrap_or_else(|| panic!("not in Dun Loireag: {}", Mode::title(&s)));
    eprintln!("Dun Loireag after {f} frames: {}", Mode::title(&s));
    assert_eq!((s.scene.area, s.scene.town, s.scene.server), (0, 1, 1));
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert!(Mode::title(w.as_ref()).contains("Dun Loireag"));
    let save = &w.world().state().save;
    assert_eq!(save.u8(offset::LAST_TOWN), 1);
    let p = w.world().player().body.pos.map(f32::from_bits);
    assert_eq!((p[0], p[1]), (0.0, 3500.0));
    let f = warp(&mut s, 0, 4000).unwrap_or_else(|| panic!("not back in Mac Anu: {}", Mode::title(&s)));
    eprintln!("Mac Anu after {f} frames: {}", Mode::title(&s));
    assert_eq!((s.scene.area, s.scene.town, s.scene.town_prev), (0, 0, 1));
}

/// The gate menu opening in Mac Anu flashes the screen: `GateMenu`'s
/// `EntryFlash(menuFade, 8, 0x3040c0c0, ...)` on the menu fader, which the
/// town carries out for the menu's `Request::Flash`.
#[test]
fn the_gate_menu_flashes() {
    let Some((iso, archive)) = disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_i16(offset::TOWN_MOVE_FLAG, 0b11);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
    let mut pad = Pad::default();
    for f in 0..4000u64 {
        let raw = match &s.stage {
            Stage::World(w) => to_the_gate(w, f, 1),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::World(w) = &s.stage else { continue };
        if w.ui().menu_type() == GATE_MENU {
            let flash = w.ui().ctrl.menu_fade.elm.iter().find(|e| e.col0 == 0x3040_c0c0);
            assert!(flash.is_some_and(|e| e.tcnt == 8), "no flash: {:?}", w.ui().ctrl.menu_fade.elm);
            return;
        }
    }
    panic!("the gate menu never opened: {}", Mode::title(&s));
}

/// `--mode story:22`: the board, Log in to Mac Anu (event 21 left the
/// party there), the gate's Other Servers to Dun Loireag - event 20's
/// `town_move 1`, replayed, opened it - where event 22's blocks for the
/// town (from block 11, `in_town 1`) take over from Mac Anu's (5 and 7):
/// without Piros in the party, block 13.
#[test]
fn story_22_reaches_dun_loireag() {
    let Some((iso, archive)) = disc() else { return };
    let mut start = crate::start::build(&iso, 22).unwrap();
    start.vm.trace = Some(Vec::new());
    assert_ne!(start.state.save.i16(offset::TOWN_MOVE_FLAG) & 2, 0, "event 20's town_move 1");
    let mut s = Session::resume(iso, archive, None, start.state, start.vm, start.at).unwrap();
    let mut pad = Pad::default();
    // The board: past its opening to the menu, then LOG IN.
    let mut n = 0;
    while !matches!(s.stage, Stage::World(_)) {
        let b = match n {
            200 => Buttons::CIRCLE,
            n if n > 240 && n % 30 == 0 => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        pad.read(&still(b));
        s.step(&pad);
        s.take_events();
        n += 1;
        assert!(n < 3000, "no Log in: {}", Mode::title(&s));
    }
    warp(&mut s, 1, 8000).unwrap_or_else(|| panic!("not in Dun Loireag: {}", Mode::title(&s)));
    // A few seconds there: the town's pass at phase 5 opens block 11.
    for _ in 0..120 {
        pad.read(&still(Buttons::NONE));
        s.step(&pad);
        s.take_events();
    }
    let Stage::World(w) = std::mem::replace(&mut s.stage, Stage::Gone) else { panic!("left the town") };
    assert_eq!(w.world().town().base.no, 1);
    let vm = w.leave().1.unwrap();
    let played: Vec<(i32, i32)> = vm
        .trace
        .iter()
        .flatten()
        .filter_map(|t| match *t {
            piney_event::vm::Trace::Block { event, block, .. } => Some((event, block)),
            _ => None,
        })
        .collect();
    let first = played.iter().position(|&b| b == (22, 13));
    assert!(first.is_some(), "event 22's Dun Loireag block did not play: {played:?}");
    assert!(played[first.unwrap()..].iter().all(|&(e, b)| e != 22 || b >= 11), "{played:?}");
}

/// Pictures of Dun Loireag: Kite put at a place and turned (`pc_put`,
/// `pc_turn`), the camera reset behind him (R2), then a second's wait -
/// the gate, the bridge south of it, the gate's pool from its side, the
/// Weapon Shop, toward the sun (its lens flare), the south, the canyon
/// either side of the middle, the Grunty shop; the clouds about him in
/// each. `PINEY_SHOTS=DIR cargo test --release -p piney-game
/// dun_loireag_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/dunloireag/shots`).
#[test]
#[ignore]
fn dun_loireag_shots() {
    use piney_event::host::PcCommand;
    let Some((iso, archive)) = disc() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/dunloireag/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 1);
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
    while town_of(&s) != Some(1) {
        step(&mut s, Buttons::NONE);
    }
    for _ in 0..90 {
        step(&mut s, Buttons::NONE);
    }
    // (name, x, y, z (tens), heading (16-bit, 0 faces south)).
    let views: [(&str, i16, i16, i16, i16); 9] = [
        ("1-gate", 0, 300, 0, -32768),
        ("2-bridge", 0, 150, 0, 0),
        ("3-pool", -150, 420, 0, 16384),
        ("4-weapon-shop", -110, 30, 0, -16384),
        ("5-lens-flare", 0, 0, 0, -7400),
        ("6-south", -250, -500, 0, 6000),
        ("7-canyon-west", -60, 0, 0, -16384),
        ("8-canyon-east", 60, 100, 0, 16384),
        ("9-grunty-shop", -180, -540, 0, -26000),
    ];
    for (name, x, y, z, dirc) in views {
        {
            let Stage::World(w) = &mut s.stage else { panic!("left the town") };
            let world = w.world_mut();
            assert!(world.pc_command(PcCommand::Put { pc: 0, x, y, z }));
            assert!(world.pc_command(PcCommand::Turn { pc: 0, dirc, chg: 0 }));
        }
        step(&mut s, Buttons::R2);
        let mut frame = step(&mut s, Buttons::NONE);
        for _ in 0..45 {
            frame = step(&mut s, Buttons::NONE);
        }
        gs.set_overlay(Mode::archive(&s));
        gs.render(&frame);
        let (w, h) = gs.target_size();
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
        println!("{path}: {}", Mode::title(&s));
    }
}

/// Dun Loireag's four stray dogs (`ccSetDog`, rows 141-144): each runs or
/// walks its route of the town's dummies, reaching its next one in 20
/// seconds. Kite put before one and the action button pressed opens its
/// `NorainuMenu` (44): it sits and faces him (`EntryAffect` 14, act 1);
/// Talk (47) and back, then cancel: it walks on (act 4).
#[test]
fn the_dogs_walk_their_routes_and_sit_to_talk() {
    use piney_event::host::PcCommand;
    let Some((iso, archive)) = disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 1);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
    let mut pad = Pad::default();
    let mut step = |s: &mut Session, b: Buttons| {
        pad.read(&still(b));
        s.step(&pad);
        s.take_events();
    };
    while town_of(&s) != Some(1) {
        step(&mut s, Buttons::NONE);
    }
    let dogs = |s: &Session| -> Vec<(i32, [f32; 4], usize, i32)> {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        w.world()
            .dogs()
            .iter()
            .map(|d| (piney_world::entry::Npc::code(d), d.ch.pos.map(f32::from_bits), d.cur, d.act))
            .collect()
    };
    let start = dogs(&s);
    assert_eq!(start.iter().map(|d| d.0).collect::<Vec<_>>(), [141, 142, 143, 144]);
    let mut reached = [false; 4];
    for _ in 0..1200 {
        step(&mut s, Buttons::NONE);
        for (k, d) in dogs(&s).iter().enumerate() {
            reached[k] |= d.2 != start[k].2;
        }
    }
    let now = dogs(&s);
    for (a, b) in start.iter().zip(&now) {
        let moved = ((b.1[0] - a.1[0]).powi(2) + (b.1[1] - a.1[1]).powi(2)).sqrt();
        eprintln!("dog {}: {:?} -> {:?} ({moved:.0}), act {}", a.0, a.1, b.1, b.3);
        assert!(moved > 100.0, "dog {} did not move", a.0);
    }
    assert_eq!(reached, [true; 4], "a dog never reached its next dummy");
    // Kite 120 south of dog 143, facing north, until it is the target.
    let mut opened = false;
    for f in 0..600u64 {
        let Stage::World(w) = &mut s.stage else { panic!("left the town") };
        if w.ui().menu_type() == 44 {
            opened = true;
            break;
        }
        let target = w.world().command_target();
        let b = if target == Some((Kind::Npc, 143)) && f.is_multiple_of(4) { Buttons::CROSS } else { Buttons::NONE };
        if target != Some((Kind::Npc, 143)) {
            let p = w.world().dogs()[2].ch.pos.map(f32::from_bits);
            let (x, y, z) = ((p[0] / 10.0) as i16, (p[1] / 10.0) as i16 - 12, (p[2] / 10.0) as i16);
            let world = w.world_mut();
            assert!(world.pc_command(PcCommand::Put { pc: 0, x, y, z }));
            assert!(world.pc_command(PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 }));
        }
        step(&mut s, b);
    }
    assert!(opened, "NorainuMenu did not open");
    for _ in 0..10 {
        step(&mut s, Buttons::NONE);
    }
    assert_eq!(dogs(&s)[2].3, 1, "the dog did not sit");
    // Talk: its line, then back to the list.
    step(&mut s, Buttons::CROSS);
    let mut talked = false;
    for f in 0..600u64 {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let t = w.ui().menu_type();
        talked |= t == 47;
        if talked && t == 44 {
            break;
        }
        step(&mut s, if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
    }
    assert!(talked, "Talk did not open");
    for _ in 0..10 {
        step(&mut s, Buttons::NONE);
    }
    step(&mut s, Buttons::CIRCLE);
    for _ in 0..30 {
        step(&mut s, Buttons::NONE);
    }
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    assert_eq!(w.ui().menu_type(), -1);
    assert_eq!(dogs(&s)[2].3, 4, "the dog did not walk on");
}

/// The young Grunty's state: (code, pos, act, growthNum, dmylevel).
fn grunties(s: &Session) -> Vec<(i32, [f32; 4], i32, i8, i16)> {
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    w.world()
        .grunties()
        .iter()
        .map(|g| (g.code, g.ch.pos.map(f32::from_bits), g.act, g.growth_num, g.dmylevel))
        .collect()
}

/// Kite put 150 south of the Grunty `code`, facing north, until it is the
/// command target; then the action button, until menu `menu` opens. The
/// frame count it took.
fn open_grunty(s: &mut Session, pad: &mut Pad, code: i32, menu: i32) -> Option<u64> {
    use piney_event::host::PcCommand;
    for f in 0..600u64 {
        let Stage::World(w) = &mut s.stage else { panic!("left the town") };
        if w.ui().menu_type() == menu {
            return Some(f);
        }
        let target = w.world().command_target();
        let b = if target == Some((Kind::Npc, code)) && f.is_multiple_of(4) { Buttons::CROSS } else { Buttons::NONE };
        if target != Some((Kind::Npc, code)) {
            let g = w.world().grunty(code).expect("the Grunty");
            let p = g.ch.pos.map(f32::from_bits);
            let (x, y, z) = ((p[0] / 10.0) as i16, (p[1] / 10.0) as i16 - 15, (p[2] / 10.0) as i16);
            let world = w.world_mut();
            assert!(world.pc_command(PcCommand::Put { pc: 0, x, y, z }));
            assert!(world.pc_command(PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 }));
        }
        pad.read(&still(b));
        s.step(pad);
        s.take_events();
    }
    None
}

/// A new game in Dun Loireag with three of the first Grunty food (key item 26):
/// `ccSetChibiGuso` places the young Grunty (row 154) walking; the action
/// button opens `InuMenu` (46) on the fixed camera, Give Food (`BreedingMenu`,
/// 56) feeds it three (act 6, growthNum 1, size 6) and it grows to level 1
/// (`evoActChibi2`, row 155, `effEvolvePG`); Talk has it sit up with its food
/// line; backing out gives the field camera and its walk again.
#[test]
fn the_grunty_eats_and_grows() {
    let Some((iso, archive)) = disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 1);
    state.save.set_u8(0x0cfc + 26, 3);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
    let mut pad = Pad::default();
    let step = |s: &mut Session, pad: &mut Pad, b: Buttons| {
        pad.read(&still(b));
        s.step(pad);
        s.take_events()
    };
    while town_of(&s) != Some(1) {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    let start = grunties(&s);
    assert_eq!(start.iter().map(|g| (g.0, g.2, g.4)).collect::<Vec<_>>(), [(154, 3, 0)]);
    for _ in 0..300 {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    let now = grunties(&s);
    let moved = ((now[0].1[0] - start[0].1[0]).powi(2) + (now[0].1[1] - start[0].1[1]).powi(2)).sqrt();
    eprintln!("Grunty: {:?} -> {:?} ({moved:.0})", start[0].1, now[0].1);
    assert!(moved > 50.0, "the Grunty did not walk");
    let f = open_grunty(&mut s, &mut pad, 154, 46).expect("InuMenu did not open");
    eprintln!("InuMenu after {f} frames");
    // The fade, the fixed camera, the greeting: proccess 5, the list.
    let mut listed = false;
    for _ in 0..120 {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        if w.ui().menu_type() == 46 && w.ui().ctrl.proccess == 5 {
            listed = true;
            break;
        }
        step(&mut s, &mut pad, Buttons::NONE);
    }
    assert!(listed, "InuMenu's list did not come up");
    {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        assert_eq!(w.world().camera().cam_id, 3, "not the fixed camera");
    }
    assert_eq!(grunties(&s)[0].2, 0, "the Grunty did not sit");
    // Give Food: the food, up to three, OK.
    step(&mut s, &mut pad, Buttons::DOWN);
    step(&mut s, &mut pad, Buttons::NONE);
    step(&mut s, &mut pad, Buttons::CROSS);
    let mut fed = false;
    let mut grew = false;
    let mut evolved = false;
    for f in 0..3000u64 {
        let (menu, proccess) = {
            let Stage::World(w) = &s.stage else { panic!("left the town") };
            (w.ui().menu_type(), w.ui().ctrl.proccess)
        };
        let g = grunties(&s)[0];
        fed |= g.2 == 6;
        grew |= g.4 == 1;
        if fed && grew && menu == 46 && proccess == 5 {
            break;
        }
        let b = match (menu, proccess) {
            (56, 2) if f.is_multiple_of(12) && !fed => {
                let Stage::World(w) = &s.stage else { unreachable!() };
                if w.ui().ctrl.wait_count < 3 { Buttons::UP } else { Buttons::CROSS }
            }
            _ if f.is_multiple_of(24) => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        let events = step(&mut s, &mut pad, b);
        evolved |= events.iter().any(|e| matches!(e, Event::Voice { event: -2, msg: 37 }));
    }
    assert!(fed, "the Grunty did not eat");
    assert!(grew, "the Grunty did not grow");
    assert!(evolved, "no growing voice");
    {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let g = piney_world::grunty::Growth::read(&w.world().state().save, 1);
        eprintln!("record: {g:?}");
        assert_eq!((g.level, g.size), (1, 6));
        assert_eq!(w.world().state().save.u8(0x0cfc + 26), 0, "the food was not taken");
        assert_eq!(w.world().grunty(154).unwrap().row.id, 155);
    }
    // Talk (the first row): its food line, then the list again.
    step(&mut s, &mut pad, Buttons::UP);
    step(&mut s, &mut pad, Buttons::NONE);
    step(&mut s, &mut pad, Buttons::CROSS);
    let mut talked = false;
    for f in 0..600u64 {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let (t, p) = (w.ui().menu_type(), w.ui().ctrl.proccess);
        talked |= t == 47;
        if talked && t == 46 && p == 5 {
            break;
        }
        step(&mut s, &mut pad, if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
    }
    assert!(talked, "Talk did not open");
    assert_eq!(grunties(&s)[0].2, 2, "the Grunty did not sit up to talk");
    // Back out: the fade, the field camera, the walk.
    step(&mut s, &mut pad, Buttons::CIRCLE);
    for _ in 0..60 {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    assert_eq!(w.ui().menu_type(), -1);
    assert_eq!(w.world().camera().cam_id, 1);
    assert_eq!(grunties(&s)[0].2, 3, "the Grunty did not walk on");
}

/// Pictures of the young Grunty: walking its route (toward Kite, the
/// camera reset behind him), `InuMenu`'s list under the fixed camera,
/// growing up after three of the first food (20, 45 and 100 frames after
/// its voice: the stretch, `effEvolvePG`'s particles, grown), and the
/// list again. `PINEY_SHOTS=DIR cargo test --release -p piney-game grunty_shots
/// -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/grunties/shots`).
#[test]
#[ignore]
fn grunty_shots() {
    use piney_event::host::PcCommand;
    let Some((iso, archive)) = disc() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/grunties/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 1);
    state.save.set_u8(0x0cfc + 26, 3);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive.clone(), None, state, None, scene, None).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
    let mut pad = Pad::default();
    let step = |s: &mut Session, pad: &mut Pad, b: Buttons| {
        pad.read(&still(b));
        let f = s.step(pad);
        let e = s.take_events();
        (f, e)
    };
    let mut shot = |s: &Session, frame: &_, name: &str| {
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let (w, h) = gs.target_size();
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
        println!("{path}: {}", Mode::title(s));
    };
    while town_of(&s) != Some(1) {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    for _ in 0..200 {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    // Kite 250 ahead of it (it walks east), facing it.
    {
        let Stage::World(w) = &mut s.stage else { panic!("left the town") };
        let p = w.world().grunty(154).unwrap().ch.pos.map(f32::from_bits);
        let (x, y, z) = ((p[0] / 10.0) as i16 + 25, (p[1] / 10.0) as i16 + 5, (p[2] / 10.0) as i16);
        let world = w.world_mut();
        assert!(world.pc_command(PcCommand::Put { pc: 0, x, y, z }));
        assert!(world.pc_command(PcCommand::Turn { pc: 0, dirc: 16384, chg: 0 }));
    }
    step(&mut s, &mut pad, Buttons::R2);
    let mut frame = step(&mut s, &mut pad, Buttons::NONE).0;
    for _ in 0..30 {
        frame = step(&mut s, &mut pad, Buttons::NONE).0;
    }
    shot(&s, &frame, "1-walking");
    open_grunty(&mut s, &mut pad, 154, 46).expect("InuMenu did not open");
    loop {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        if w.ui().menu_type() == 46 && w.ui().ctrl.proccess == 5 {
            break;
        }
        frame = step(&mut s, &mut pad, Buttons::NONE).0;
    }
    for _ in 0..20 {
        frame = step(&mut s, &mut pad, Buttons::NONE).0;
    }
    shot(&s, &frame, "2-inu-menu");
    step(&mut s, &mut pad, Buttons::DOWN);
    step(&mut s, &mut pad, Buttons::NONE);
    step(&mut s, &mut pad, Buttons::CROSS);
    let mut since_voice: Option<u32> = None;
    let mut grown = false;
    for f in 0..3000u64 {
        let (menu, proccess, wc) = {
            let Stage::World(w) = &s.stage else { panic!("left the town") };
            (w.ui().menu_type(), w.ui().ctrl.proccess, w.ui().ctrl.wait_count)
        };
        if grown && menu == 46 && proccess == 5 {
            break;
        }
        let b = match (menu, proccess) {
            (56, 2) if f.is_multiple_of(12) => {
                if wc < 3 {
                    Buttons::UP
                } else {
                    Buttons::CROSS
                }
            }
            (56, 10) => Buttons::NONE,
            _ if f.is_multiple_of(24) => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        let (fr, events) = step(&mut s, &mut pad, b);
        frame = fr;
        if events.iter().any(|e| matches!(e, Event::Voice { event: -2, msg: 37 })) {
            since_voice = Some(0);
        }
        if let Some(n) = since_voice.as_mut() {
            *n += 1;
            match *n {
                20 => shot(&s, &frame, "3-growing"),
                45 => shot(&s, &frame, "4-evolve-flash"),
                100 => {
                    shot(&s, &frame, "5-grown");
                    grown = true;
                }
                _ => {}
            }
        }
    }
    for _ in 0..20 {
        frame = step(&mut s, &mut pad, Buttons::NONE).0;
    }
    shot(&s, &frame, "6-little-grunty");
}

/// A save whose Dun Loireag record has its first grown kind (`type[0]`)
/// and a grown level: `ccSetChibiGuso` places the grown Grunty (row 145)
/// at `DMY_cdog0` and, the record starting over, a new young one (154).
/// The action button on the grown one opens `OtonainuMenu` (45): its
/// `dogAction2` sets line 7 (msgNum); Talk (47) and back, then cancel: the
/// menu shuts.
#[test]
fn the_grown_grunty_talks() {
    let Some((iso, archive)) = disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 1);
    let g = piney_world::grunty::Growth { level: 4, size: 40, ty: [1, 0, 0], food_num: -1, ..Default::default() };
    g.write(&mut state.save, 1);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
    let mut pad = Pad::default();
    let step = |s: &mut Session, pad: &mut Pad, b: Buttons| {
        pad.read(&still(b));
        s.step(pad);
        s.take_events()
    };
    while town_of(&s) != Some(1) {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    let all = grunties(&s);
    assert_eq!(all.iter().map(|g| (g.0, g.4)).collect::<Vec<_>>(), [(145, 4), (154, 0)]);
    {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let rec = piney_world::grunty::Growth::read(&w.world().state().save, 1);
        assert_eq!((rec.level, rec.size, rec.ty), (0, 0, [1, 0, 0]), "the record did not start over");
    }
    open_grunty(&mut s, &mut pad, 145, 45).expect("OtonainuMenu did not open");
    for _ in 0..10 {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        assert_eq!(w.world().grunty(145).unwrap().msg_num, 7);
    }
    step(&mut s, &mut pad, Buttons::CROSS);
    let mut talked = false;
    for f in 0..600u64 {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let t = w.ui().menu_type();
        talked |= t == 47;
        if talked && t == 45 {
            break;
        }
        step(&mut s, &mut pad, if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
    }
    assert!(talked, "Talk did not open");
    for _ in 0..10 {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    step(&mut s, &mut pad, Buttons::CIRCLE);
    for _ in 0..30 {
        step(&mut s, &mut pad, Buttons::NONE);
    }
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    assert_eq!(w.ui().menu_type(), -1);
}

/// Mac Anu from a new game: a walking PC spoken to (Kite put before it,
/// the action button) opens TalkMenu (22), and the PC stands and faces
/// Kite (`rTownNPCInfluence` 15, act 2) until the talk's pages close.
#[test]
fn a_walker_stands_through_its_talk() {
    use piney_event::host::PcCommand;
    use piney_world::entry::Npc as _;
    let Some((iso, archive)) = disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 0);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
    let mut pad = Pad::default();
    let mut step = |s: &mut Session, b: Buttons| {
        pad.read(&still(b));
        s.step(&pad);
        s.take_events();
    };
    let mut code = None;
    for _ in 0..3000 {
        step(&mut s, Buttons::NONE);
        if let Stage::World(w) = &s.stage {
            code = w.world().pcs().iter().find(|n| n.think_type == 0 && n.act_num == 3).map(|n| n.code());
            if code.is_some() {
                break;
            }
        }
    }
    let code = code.expect("no walker out");
    let pc = |s: &Session| {
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let n = w.world().pcs().iter().find(|n| n.code() == code).expect("the walker");
        (n.act_num, n.char.pos.map(|v| f32::from_bits(v) as i32), w.ui().menu_type())
    };
    let mut opened = false;
    for f in 0..900u64 {
        let Stage::World(w) = &mut s.stage else { panic!("left the town") };
        if w.ui().menu_type() == 22 {
            opened = true;
            break;
        }
        let target = w.world().command_target();
        let on = target == Some((Kind::Npc, code));
        if !on {
            let p = w.world().pcs().iter().find(|n| n.code() == code).unwrap().char.pos.map(f32::from_bits);
            let (x, y, z) = ((p[0] / 10.0) as i16, (p[1] / 10.0) as i16 - 12, (p[2] / 10.0) as i16);
            let world = w.world_mut();
            world.pc_command(PcCommand::Put { pc: 0, x, y, z });
            world.pc_command(PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 });
        }
        step(&mut s, if on && f.is_multiple_of(4) { Buttons::CROSS } else { Buttons::NONE });
    }
    assert!(opened, "TalkMenu did not open for walker {code}");
    // The talk read slowly: ten seconds with nothing pressed, then paged.
    let first = pc(&s);
    let mut moved = Vec::new();
    for f in 0..1500u64 {
        let now = pc(&s);
        if now.2 == -1 {
            break;
        }
        if now.0 != 2 || now.1[..3] != first.1[..3] {
            moved.push((f, now));
        }
        step(&mut s, if f > 600 && f.is_multiple_of(30) { Buttons::CROSS } else { Buttons::NONE });
    }
    assert!(
        moved.is_empty(),
        "walker {code} moved during its talk (first {first:?}): {:?}",
        &moved[..moved.len().min(5)]
    );
}
