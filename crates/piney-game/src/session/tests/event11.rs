//! Event 11 (MG0150) in Mac Anu as a player plays it: BlackRose's stream
//! in the town's set-up, her talk at the Chaos Gate with her member
//! address, the call from PERSONAL's Party > Add that brings her into the
//! party, and the Chaos Gate's Word List to area 15 with her.

use std::f32::consts::PI;
use std::path::PathBuf;

use piney_event::ScriptSave;
use piney_input::{Buttons, Raw};
use piney_world::entry::Kind;

use super::*;

/// BlackRose's `charTbl` row.
const BLACKROSE: i32 = 15;

/// `--mode story:11`'s session, the event task recording its blocks.
fn story_11() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut start = crate::start::build(&iso, 11).unwrap();
    start.vm.trace = Some(Vec::new());
    Some(Session::resume(iso, archive, None, start.state, start.vm, start.at).unwrap())
}

/// A save from the card (`dhdataNN`, as the Data screen writes it)
/// resumed in its town on a new boot's event task, as the title's Load
/// leaves it.
fn from_card(file: &str) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let bytes = std::fs::read(file).ok()?;
    let mut disc = Iso::open(&iso).ok()?;
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut disc).unwrap();
    let mut vm = crate::desktop::boot(&mut disc, &mut state).unwrap();
    vm.trace = Some(Vec::new());
    state.save = piney_data::save::SaveData::from_bytes(&bytes).unwrap();
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let at = Resume::World(Box::new(InWorld { scene, world_man: None, spcs: None }));
    Some(Session::resume(iso, archive, None, state, vm, at).unwrap())
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

/// Whether BlackRose's member address has been given
/// (`partyMemberFlag` bit 15, set by `member_add_msg 15`).
fn has_address(w: &WorldMode) -> bool {
    w.world().state().save.member_word(offset::PARTY_MEMBER_FLAG) & (1 << BLACKROSE) != 0
}

fn joined(w: &WorldMode) -> bool {
    w.world().party().contains(&BLACKROSE)
}

/// The row of story area `area` in the Word List (`gateOrderList[0]`, the
/// areas the events gave, in order).
fn word_list_row(w: &WorldMode, area: i16) -> Option<usize> {
    let save = &w.world().state().save;
    (0..64).map(|k| save.i16(offset::GATE_ORDER_LIST + 2 * k)).filter(|&v| v >= 0).position(|v| v == area)
}

/// The player's pad in town on frame `f`: a waiting window closed every
/// 24 frames; before her address, Kite walks to the Chaos Gate and speaks
/// to it; after it, PERSONAL (triangle), Party, Add, BlackRose, OK; with
/// her in the party, the gate again: Word List, area 15, Warp.
fn player(w: &WorldMode, f: u64) -> Raw {
    let ui = w.ui();
    let c = &ui.ctrl;
    let go_to_item = |item: i16| {
        let l = c.list();
        match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == item) {
            Some(row) if row as i16 == l.select => Buttons::CROSS,
            Some(row) if (row as i16) > l.select => Buttons::DOWN,
            Some(_) => Buttons::UP,
            None => Buttons::NONE,
        }
    };
    if ui.menu_type() != -1 {
        if !f.is_multiple_of(8) {
            return still(Buttons::NONE);
        }
        let b = match (ui.menu_type(), c.proccess) {
            (0, 1) => go_to_item(9),
            (9, 1) => go_to_item(68),
            (68, 2) if i32::from(c.face_num) == BLACKROSE => Buttons::CROSS,
            (68, 2) => Buttons::DOWN,
            (68, 5) => Buttons::CROSS,
            // Her greeting after the call, closed as a window.
            (68, p) if p >= 20 && f.is_multiple_of(24) => Buttons::CROSS,
            (28, 1) => go_to_item(59),
            (59, 2) => match word_list_row(w, 15) {
                Some(row) if row as i16 == c.list().select => Buttons::CROSS,
                Some(row) if row as i16 > c.list().select => Buttons::DOWN,
                Some(_) => Buttons::UP,
                None => Buttons::NONE,
            },
            (59, 4) => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        return still(b);
    }
    // A window waiting: the last window call was an open.
    let waiting = w.calls().iter().rev().find_map(|(_, c)| {
        if c.starts_with("message_open") || c.starts_with("announce") || c.starts_with("info_lines") {
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
    if has_address(w) && !joined(w) {
        return still(if f.is_multiple_of(30) { Buttons::TRIANGLE } else { Buttons::NONE });
    }
    if world.command_target() == Some((Kind::Gimmick, 16)) {
        return still(if f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE });
    }
    let Some((gate, _)) = world.char_place(Kind::Gimmick, 16) else { return still(Buttons::NONE) };
    let g = gate.map(f32::from_bits);
    let p = world.player().body.pos.map(f32::from_bits);
    let cam_z = f32::from_bits(world.camera().rot()[2]);
    stick_toward(cam_z, (g[0] - p[0]).atan2(-(g[1] - p[1])))
}

/// Event 11 in Mac Anu: stream 7 plays in the set-up; speaking to the
/// Chaos Gate brings BlackRose over with her member address; PERSONAL's
/// Party > Add calls her, and she joins the party; no event instruction
/// fell to a host default on the way.
#[test]
fn event_11_blackrose_joins_in_mac_anu() {
    let Some(mut s) = story_11() else { return };
    piney_event::host::take_unported();
    let mut pad = Pad::default();
    let mut joined = None;
    for f in 0..20_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage
            && w.world().party().contains(&BLACKROSE)
        {
            joined = Some(f);
            break;
        }
    }
    let Stage::World(w) = &s.stage else { panic!("left the town: {}", Mode::title(&s)) };
    let calls: Vec<&str> = w.calls().iter().map(|(_, c)| c.as_str()).collect();
    assert!(calls.contains(&"stream 7"), "stream 7 did not play: {calls:?}");
    let tail: Vec<String> = w.calls().iter().rev().take(40).rev().map(|(f, c)| format!("{f} {c}")).collect();
    assert!(has_address(w), "no member address: {}\n{}", Mode::title(&s), tail.join("\n"));
    assert!(joined.is_some(), "BlackRose did not join: {} party {:?}", Mode::title(&s), w.world().party());
    assert_eq!(piney_event::host::take_unported(), Vec::<&str>::new());
}

/// Past the join: back at the Chaos Gate, the Word List's area 15 (the
/// address event 11 gave) and Warp. The party leaves through the gate, and
/// Kite arrives on the holy ground with BlackRose in the party and placed
/// beside him.
#[test]
fn event_11_gates_to_area_15_with_blackrose() {
    let Some(mut s) = story_11() else { return };
    let mut pad = Pad::default();
    let mut joined_at = None;
    let mut arrived = false;
    let mut entered = None;
    for f in 0..30_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => {
                if joined_at.is_none() && joined(w) {
                    joined_at = Some(f);
                }
                player(w, f)
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage {
            let at = *entered.get_or_insert(f);
            if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 30) {
                // The set-up's passes are not held by the arrival's talk
                // (the town's event task disabled as it left).
                assert!(f - at < 200, "the holy ground's set-up took {} frames", f - at);
                arrived = true;
                break;
            }
        }
    }
    assert!(joined_at.is_some(), "BlackRose did not join: {}", Mode::title(&s));
    assert!(arrived, "not on the holy ground: {}", Mode::title(&s));
    let Stage::Area(a) = &s.stage else { unreachable!() };
    let w = a.world();
    assert_eq!((w.scene().area, w.scene().field), (1, 15), "{}", Mode::title(&s));
    assert!(w.party().contains(&BLACKROSE), "party {:?}", w.party());
    assert!(w.char_pos(2, BLACKROSE as i16).is_some(), "BlackRose is not placed");
}

/// Pictures of event 11 as [`event_11_gates_to_area_15_with_blackrose`]
/// plays it: stream 7, BlackRose's address at the gate, the call, and the
/// pair on the holy ground. `PINEY_SHOTS=DIR cargo test --release -p
/// piney-game event_11_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/event11`).
#[test]
#[ignore]
fn event_11_shots() {
    let Some(mut s) = story_11() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/event11".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut taken: Vec<&str> = Vec::new();
    let mut in_area = None;
    let mut gate_msg = None;
    for f in 0..30_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage
            && gate_msg.is_none()
            && w.calls().iter().any(|(_, c)| c.starts_with("announce GateAddress"))
        {
            gate_msg = Some(f);
        }
        let name = match &s.stage {
            Stage::World(w) if Mode::archive(w.as_ref()).is_some() && f == 400 => Some("1-stream-7"),
            Stage::World(_) if gate_msg == Some(f.saturating_sub(20)) => Some("2b-gate-address"),
            Stage::World(w) if has_address(w) && !taken.contains(&"2-address") => Some("2-address"),
            Stage::World(w) if joined(w) && !taken.contains(&"3-joined") => Some("3-joined"),
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 2) => {
                let at = *in_area.get_or_insert(f);
                (f == at + 90).then_some("4-holy-ground")
            }
            _ => None,
        };
        if let Some(name) = name {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{name}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            taken.push(name);
            if name == "4-holy-ground" {
                break;
            }
        }
    }
}

/// The field side of the player: windows closed every 24 frames, else Kite
/// walked toward `goal` (x, y) under the field's camera.
fn field_player(a: &crate::area::AreaMode, f: u64, goal: [f32; 2]) -> Raw {
    let waiting = a.calls().iter().rev().find_map(|(_, c)| {
        if c.starts_with("message_open") || c.starts_with("announce") || c.starts_with("info_lines") {
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
    let w = a.world();
    let p = w.player().body.pos.map(f32::from_bits);
    let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
    if dx * dx + dy * dy < 60.0 * 60.0 {
        return still(Buttons::NONE);
    }
    stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
}

/// Event 11 from the join through the holy ground and the church: its
/// tail (block 25) fades out, `party_remove 15` takes BlackRose out of the
/// party in the field (`ccParty::DelMember`: her slot emptied, her
/// registry and character `partyFlag` 0 and -1), and the scene goes back
/// to Mac Anu, which takes the party the field left (the session's
/// `spcs`) without her; no event instruction fell to a host default. The
/// church's set-up pass shows "The book. / Open the book." (block 24's
/// `info_now` at phase 2) on the set-up's screen while the load holds.
#[test]
fn event_11_church_ends_with_blackrose_out() {
    let Some(mut s) = story_11() else { return };
    let mut pad = Pad::default();
    piney_event::host::take_unported();
    let (mut arrived, mut area_calls, mut in_area_party) = (None, Vec::new(), None);
    let mut left_party = None;
    let mut book_shown = false;
    for f in 0..60_000u64 {
        if let Stage::Area(a) = &s.stage {
            area_calls = a.calls().to_vec();
            let w = a.world();
            in_area_party.get_or_insert(w.party());
            if area_calls.iter().any(|(_, c)| c == "party_remove 15") && left_party.is_none() {
                let reg = w.spcs().registry.iter().find(|r| r.id == BLACKROSE).map(|r| r.party_flag);
                let c = w.combat();
                let own = c.who(BLACKROSE).map(|b| (((c.scene.chars[b].party_flag & 7) << 5) as i8) >> 5);
                left_party = Some((w.party(), reg, own));
            }
        }
        let raw = match &s.stage {
            Stage::World(_) if arrived.is_some() => break,
            Stage::World(w) => player(w, f),
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 2) => {
                arrived.get_or_insert(f);
                field_player(a, f, [95.0, 3800.0])
            }
            Stage::Area(a) => {
                field_player(a, f, a.world().player().body.pos.map(f32::from_bits)[..2].try_into().unwrap())
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && matches!(a.world().phase(), piney_world::Phase::Hold(_))
            && a.calls().last().is_some_and(|(_, c)| c.starts_with("message_open 11 24"))
            && frame.cmds.iter().any(|c| matches!(c, piney_draw::Cmd::Prim(p) if p.state.texture.is_some()))
        {
            book_shown = true;
        }
    }
    let calls: Vec<&str> = area_calls.iter().map(|(_, c)| c.as_str()).collect();
    assert!(book_shown, "the set-up's window was not drawn");
    assert!(
        in_area_party.is_some_and(|p| p.contains(&BLACKROSE)),
        "BlackRose was not in the party on the holy ground: {in_area_party:?}"
    );
    assert!(calls.contains(&"party_remove 15"), "no party_remove: {calls:?}");
    let Some((party, reg, own)) = left_party else { panic!("the church's tail did not run") };
    assert!(!party.contains(&BLACKROSE), "still in the party after party_remove: {party:?}");
    assert_eq!(reg, Some(0), "her registry partyFlag");
    assert_eq!(own, Some(-1), "her character's partyFlag (left)");
    let Stage::World(w) = &s.stage else { panic!("not back in Mac Anu: {}", Mode::title(&s)) };
    assert!(!w.world().party().contains(&BLACKROSE), "in Mac Anu's party: {:?}", w.world().party());
    let Some(sp) = &s.spcs else { panic!("the field left no party") };
    assert!(!sp.party().contains(&BLACKROSE), "in the party the field left: {:?}", sp.party());
    // Her fellow task's delete at the change of scene (`ccThFellow15Delete`,
    // her own partyFlag not 1) freed her registry slot.
    assert_eq!(sp.registry.iter().find(|r| r.id == BLACKROSE).map(|r| r.party_flag), None);
    assert_eq!(piney_event::host::take_unported(), Vec::<&str>::new());
}

/// What event 11 does from the holy ground into the church: `cargo test
/// --release -p piney-game event_11_church_log -- --ignored --nocapture`.
#[test]
#[ignore]
fn event_11_church_log() {
    let Some(mut s) = story_11() else { return };
    let mut pad = Pad::default();
    piney_event::host::take_unported();
    let mut arrived = None;
    // The last area's calls (the church's field), kept as it is left.
    let mut area_calls: Vec<(u64, String)> = Vec::new();
    for f in 0..60_000u64 {
        if let Stage::Area(a) = &s.stage {
            area_calls = a.calls().to_vec();
        }
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 2) => {
                arrived.get_or_insert(f);
                field_player(a, f, [95.0, 3800.0])
            }
            // A window the set-up's pass opened (the church's block 24).
            Stage::Area(a) => {
                field_player(a, f, a.world().player().body.pos.map(f32::from_bits)[..2].try_into().unwrap())
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if arrived.is_some_and(|a| f > a + 24000) {
            break;
        }
    }
    println!("{}", Mode::title(&s));
    for (f, c) in &area_calls {
        println!("{f:6} {c}");
    }
    println!("unported {:?}", piney_event::host::take_unported());
}

/// Pictures of the church as [`event_11_church_ends_with_blackrose_out`]
/// plays it, every 30 frames from the door to Mac Anu, with the titles:
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game event_11_church_shots
/// -- --ignored --nocapture` (default `/mnt/data/claude/scratch/church`).
#[test]
#[ignore]
fn event_11_church_shots() {
    // PINEY_CARD=FILE: a save (a card slot's dhdataNN) resumed in Mac Anu
    // in place of the story's start.
    let s = match std::env::var("PINEY_CARD") {
        Ok(card) => from_card(&card),
        Err(_) => story_11(),
    };
    let Some(mut s) = s else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/church".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut arrived = None;
    let mut church = None;
    // PINEY_MASH=cross or start: that button every 10 frames once in the
    // church; PINEY_EVERY: frames between pictures.
    let mash = match std::env::var("PINEY_MASH").as_deref() {
        Ok("cross") => Some(Buttons::CROSS),
        Ok("start") => Some(Buttons::START),
        _ => None,
    };
    let attack = std::env::var("PINEY_ATTACK").is_ok();
    let every: u64 = std::env::var("PINEY_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    for f in 0..60_000u64 {
        let raw = match &s.stage {
            Stage::World(_) if church.is_some() => break,
            Stage::World(w) => player(w, f),
            Stage::Area(_) if church.is_some() && mash.is_some() => {
                still(if f.is_multiple_of(10) { mash.unwrap() } else { Buttons::NONE })
            }
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 2) => {
                arrived.get_or_insert(f);
                let mut raw = field_player(a, f, [95.0, 3800.0]);
                // PINEY_ATTACK: X every 8 frames by the door.
                let y = f32::from_bits(a.world().player().body.pos[1]);
                if attack && church.is_none() && y > 3440.0 && f.is_multiple_of(8) {
                    raw.buttons |= Buttons::CROSS;
                }
                raw
            }
            Stage::Area(a) => {
                field_player(a, f, a.world().player().body.pos.map(f32::from_bits)[..2].try_into().unwrap())
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && church.is_none()
            && a.world().scene().block == 1
        {
            church = Some(f);
        }
        // Every frame rendered (textures load as frames ask for them).
        gs.set_overlay(Mode::archive(&s));
        gs.render(&frame);
        if let Some(c) = church
            && (f - c).is_multiple_of(every)
        {
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{:05}.png", f - c);
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
        }
    }
}

/// Pictures of the party's leave through Mac Anu's Chaos Gate (the Word
/// List's Warp to area 15): every 5 frames from the first TransferOut to
/// the field. `PINEY_SHOTS=DIR cargo test --release -p piney-game
/// event_11_warp_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/warp`).
#[test]
#[ignore]
fn event_11_warp_shots() {
    let Some(mut s) = story_11() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/warp".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut leave = None;
    for f in 0..30_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage
            && leave.is_none()
            && w.world().player().acts.act == 12
        {
            leave = Some(f);
        }
        gs.set_overlay(Mode::archive(&s));
        gs.render(&frame);
        if let Some(l) = leave
            && (f - l).is_multiple_of(5)
        {
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{:04}.png", f - l);
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            if f - l >= 260 {
                break;
            }
        }
    }
}

/// Event 11 with BlackRose in the party on the holy ground, then back to
/// Mac Anu before the church (the console's `town 0`, as a Gate Out): she
/// is still a member at the change of scene, so the fellows' delete keeps
/// her; the town has her in the party and the registry, `partyFlag` 1.
#[test]
fn event_11_gate_out_keeps_blackrose() {
    let Some(mut s) = story_11() else { return };
    let mut pad = Pad::default();
    let mut left = false;
    for f in 0..40_000u64 {
        let raw = match &s.stage {
            Stage::World(_) if left => break,
            Stage::World(w) => player(w, f),
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 2) => {
                if !left && a.world().party().contains(&BLACKROSE) {
                    left = true;
                    s.console("town 0");
                    continue;
                }
                field_player(a, f, [95.0, 3800.0])
            }
            Stage::Area(a) => {
                field_player(a, f, a.world().player().body.pos.map(f32::from_bits)[..2].try_into().unwrap())
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    assert!(left, "never on the holy ground with BlackRose: {}", Mode::title(&s));
    let Stage::World(w) = &s.stage else { panic!("not back in Mac Anu: {}", Mode::title(&s)) };
    let reg = w.world().spcs().registry.iter().find(|r| r.id == BLACKROSE).map(|r| r.party_flag);
    assert!(w.world().party().contains(&BLACKROSE), "not in Mac Anu's party: {:?}", w.world().party());
    assert_eq!(reg, Some(1), "her registry slot");
    let slot = w.world().party().iter().position(|&id| id == BLACKROSE).unwrap();
    assert_eq!(w.ui().ctrl.face_tex[slot], BLACKROSE, "her menu face");
    let at = piney_data::save::by_id::spc_param(BLACKROSE as usize);
    let save = &w.world().state().save;
    println!("her record: id {} head {:?}", save.i16(at + 0x0c), &save.bytes()[at..at + 0x40]);
}

/// On the holy ground, BlackRose spoken to: her menu (21) opens with her
/// as the one spoken to (`TalkTarget`), which Talk, Trade and Gift read
/// (`cmndTargetPrev->base`); without it Gift gave to id 0, unnamed.
#[test]
fn event_11_blackrose_spoken_to_on_the_holy_ground() {
    let Some(mut s) = story_11() else { return };
    let mut pad = Pad::default();
    let mut opened = None;
    for f in 0..40_000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 2) => {
                let w = a.world();
                let c = w.combat();
                if a.ui().menu_type() == 21 {
                    opened = a.ui().ctrl.talk.target;
                    break;
                }
                match c.who(BLACKROSE) {
                    Some(k) if w.party().contains(&BLACKROSE) => {
                        if w.command_target_code() == Some((piney_world::entry::Kind::Spc, BLACKROSE)) {
                            still(if f.is_multiple_of(12) { Buttons::CROSS } else { Buttons::NONE })
                        } else {
                            let q = c.scene.chars[k].pos.map(f32::from_bits);
                            field_player(a, f, [q[0], q[1]])
                        }
                    }
                    _ => field_player(a, f, [95.0, 3800.0]),
                }
            }
            Stage::Area(a) => {
                field_player(a, f, a.world().player().body.pos.map(f32::from_bits)[..2].try_into().unwrap())
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    let t = opened.expect("her menu never opened");
    assert_eq!(t.who, piney_fieldui::talk::Speaker::Spc(BLACKROSE));
    assert_eq!(t.handle, (1 << 24) | BLACKROSE as u32);
}
