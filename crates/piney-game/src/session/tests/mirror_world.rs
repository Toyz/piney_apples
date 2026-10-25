//! Issue #74: Bitter Fantasy Mirror World, the post-game quest (event 163
//! on every volume). Field 66 is `EVENTAREA04`, its dungeon's last door
//! `ChangeArea(1, 67)`, and field 67 is `EVENTAREA05`, the arena where
//! `entry_mc` makes the ghost town's boss.

use super::*;

/// The player's log replayed through the launcher: each change of scene
/// (area, field, dungeon, floor, the place's kind) printed, and a shot of
/// the first frames in each to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/i74/replay). A diagnostic:
/// `PINEY_PADLOG=FILE cargo test --release -p piney-game issue_74_replay
/// -- --ignored --nocapture`.
#[test]
#[ignore]
fn issue_74_replay() {
    let Some(log) = std::env::var_os("PINEY_PADLOG") else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i74/replay".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs: Option<piney_gs::Gs> = None;
    let mut last = String::new();
    let mut shoot_at = Vec::<u32>::new();
    crate::launcher::replay_pad_log(&log.into_string().unwrap(), |f, mode, _events, _audio, frame| {
        let title = mode.title();
        let overlay = mode.archive();
        let Some(s) = mode.session() else { return };
        let archive = s.archive.clone();
        let now = match &s.stage {
            Stage::Area(a) => {
                let w = a.world();
                let sc = w.scene();
                format!(
                    "area {} field {} dungeon {} floor {} block {} event {} place {}",
                    sc.area,
                    sc.field,
                    sc.dungeon,
                    sc.floor,
                    sc.block,
                    w.world_man().event,
                    place_kind(w.place())
                )
            }
            _ => String::new(),
        };
        if now != last {
            println!("frame {f}: {now} | {title}");
            last = now;
            shoot_at.extend([f + 90, f + 400]);
        }
        let every = std::env::var("PINEY_EVERY").ok().and_then(|v| v.parse::<u32>().ok());
        if shoot_at.contains(&f) || every.is_some_and(|n| f.is_multiple_of(n)) {
            let gs = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap());
            gs.set_overlay(overlay);
            gs.render(frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/f{f:05}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("  {path}");
        }
    });
}

/// The place's kind, for the replay's lines.
fn place_kind(p: &piney_world::field_world::Place) -> &'static str {
    use piney_world::field_world::Place;
    match p {
        Place::Field(_) => "generated field",
        Place::Dungeon(_) => "dungeon",
        Place::Story(_) => "story map",
    }
}

fn still() -> piney_input::Raw {
    piney_input::Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..piney_input::Raw::default() }
}

/// Event 163 brought to the last floor of area 66's dungeon on area 66's
/// `WORLD_MAN`: event 115 done and 163's blocks 0 and 1 played (the mail,
/// the gate's words), as the Chaos Gate and the dungeon leave them.
fn in_area_66_dungeon(disc: &str) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
    if !iso.exists() {
        eprintln!("{disc}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let (mut state, vm) = crate::start::state_at(&iso, 116).unwrap();
    let f = state.save.event_flag(115);
    state.save.set_event_flag(115, f | piney_event::state::DONE);
    state.save.set_event_flag(163, 0b11);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 66, false).unwrap();
    scene.change_scene(2, 1, 66, 0, 2, 0, &mut state.save);
    Some(Session::in_world(iso.clone(), archive, None, state, Some(vm), scene, Some(wm)).unwrap())
}

/// The arena's portal (`entry_mc 0 262` at the event's position 0) on the
/// entry control's list, and its place.
fn portal(s: &Session) -> Option<piney_world::ee::V4> {
    let Stage::Area(a) = &s.stage else { return None };
    let c = a.world().combat();
    let mut circles = c.ctrl.list(piney_battle::entry::Kind::Circle).into_iter();
    circles.find(|&i| c.ctrl.entry_obj(i).is_some_and(|o| o.obj_flag)).map(|i| c.scene.chars[i].pos)
}

/// Issue #74: area 66's last door is `ChangeArea(1, 67)` on area 66's
/// `WORLD_MAN` (`GotoNextRoom`'s -255). `GO(1)` makes field 67's
/// `EVENTAREA05`, the arena: event 163's portal stands on its floor, the
/// ghost town's boss (enemy 262) comes out of it, and once it is beaten
/// the event gives its item and sends the party to Mac Anu. Before the fix
/// field 67 was a generated field with no way out but the fight.
#[test]
fn area_66s_last_door_leads_to_the_arena() {
    for disc in ["mutation", "outbreak", "quarantine"] {
        let Some(mut s) = in_area_66_dungeon(disc) else { continue };
        let mut pad = Pad::default();
        for _ in 0..200 {
            pad.read(&still());
            s.step(&pad);
            s.take_events();
        }
        let field = piney_data::dungeon::special::exit_field(66, 2).unwrap();
        assert_eq!(field, 67);
        // WORLD_MAN::Enter on GotoNextRoom's -255, as the door asks it.
        let Stage::Area(a) = &mut s.stage else { panic!("{disc}: not in the dungeon") };
        a.world_mut().change_area(1, field);
        // ChangeRequest(6, 7)'s ccDisableThEvent, in the door's own frame.
        a.vm_mut().unwrap().disable();
        let (mut arena, mut put, mut foe, mut town) = (false, false, false, false);
        for f in 0..6000u32 {
            let b = if f.is_multiple_of(8) { piney_input::Buttons::CROSS } else { piney_input::Buttons::NONE };
            pad.read(&piney_input::Raw { buttons: b, ..still() });
            s.step(&pad);
            s.take_events();
            if matches!(s.stage, Stage::World(_)) {
                town = true;
                break;
            }
            let Stage::Area(a) = &mut s.stage else { continue };
            let w = a.world();
            let in_dungeon = matches!(w.place(), piney_world::field_world::Place::Dungeon(_));
            if in_dungeon || !matches!(w.phase(), piney_world::Phase::Play(n) if n > 60) {
                continue;
            }
            let sky = w.place().story::<piney_world::evarea_sky::SkyMap>();
            arena |= sky.is_some_and(|m| m.class == piney_world::evarea_sky::Class::Arena);
            assert_eq!(w.world_man().event, 67, "{disc}: eventAreaNumber");
            let c = w.combat();
            let foes: Vec<usize> = c.enemies().into_iter().filter(|&e| !c.dead(e)).collect();
            foe |= foes.iter().any(|&e| c.scene.chars[e].id() == 262);
            if !put && let Some(at) = portal(&s) {
                let tenth = |v: u32| (f32::from_bits(v) / 10.0).round() as i16;
                let Stage::Area(a) = &mut s.stage else { unreachable!() };
                a.world_mut().pc_command(piney_event::host::PcCommand::Put {
                    pc: 0,
                    x: tenth(at[0]),
                    y: tenth(at[1]),
                    z: tenth(at[2]),
                });
                put = true;
            } else if foe && f.is_multiple_of(20) {
                let Stage::Area(a) = &mut s.stage else { unreachable!() };
                a.kill_enemies();
            }
        }
        assert!(arena, "{disc}: field 67 is not EVENTAREA05");
        assert!(put, "{disc}: no portal in the arena");
        assert!(foe, "{disc}: the portal gave no enemy 262");
        assert!(town, "{disc}: event 163 did not end in town: {}", Mode::title(&s));
        let Stage::World(w) = &s.stage else { panic!("{disc}: not in town: {}", Mode::title(&s)) };
        // Block 6 played (the item, the scene to town) and `end_event -1`
        // closed the event (ccStartThEvent makes it done).
        let flags = w.world().state().save.event_flag(163);
        assert_ne!(flags & (1 << 6), 0, "{disc}: block 6 not played: {flags:x}");
        let ended = piney_event::state::DONE | piney_event::state::CLOSED;
        assert_ne!(flags & ended, 0, "{disc}: event 163 not ended: {flags:x}");
    }
}

/// Issue #74: field 66 is `EVENTAREA04` and 91 `EVENTAREA06`, not
/// generated fields of their rows' type 255 (which have no ground): from
/// the Chaos Gate the party stands at the map's start on its own floor
/// (66's at (4600, -4700, 910)), and the floor's door is
/// `WORLD_MAN::Enter`'s `ChangeArea(2, 0)`, the dungeon.
#[test]
fn fields_66_and_91_are_their_maps() {
    use piney_world::evarea_sky::{Class, SkyMap};
    for (disc, field, class) in
        [("mutation", 66, Class::Mirror), ("quarantine", 66, Class::Mirror), ("outbreak", 91, Class::Whale)]
    {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        if !iso.exists() {
            continue;
        }
        let mut d = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let (mut state, vm) = crate::start::state_at(&iso, 116).unwrap();
        let mut scene = piney_world::area::Scene::log_in(&mut state.save);
        let wm = crate::area::story_world_man(&mut d, field, false).unwrap();
        scene.change_scene(1, 1, field, -1, -1, -1, &mut state.save);
        let mut s = Session::in_world(iso.clone(), archive, None, state, Some(vm), scene, Some(wm)).unwrap();
        let mut pad = Pad::default();
        let mut door = None;
        for _ in 0..400 {
            pad.read(&still());
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { continue };
            let w = a.world();
            if matches!(w.phase(), piney_world::Phase::Play(n) if n > 120) {
                door = w.door();
                break;
            }
        }
        let Stage::Area(a) = &mut s.stage else { panic!("{disc}: not in field {field}") };
        let w = a.world();
        let sky = w.place().story::<SkyMap>().unwrap_or_else(|| panic!("{disc}: field {field} is no story map"));
        assert_eq!(sky.class, class, "{disc}");
        let start = sky.start.map(f32::from_bits);
        if class == Class::Mirror {
            assert_eq!(start[..3], [4600.0, -4700.0, 910.0], "{disc}: the start");
        }
        let p = w.player().body.pos.map(f32::from_bits);
        assert_eq!(p[..2], start[..2], "{disc}: Kite is not at the start");
        assert!((p[2] - start[2]).abs() < 200.0, "{disc}: Kite is not on the floor: {p:?}");
        let at = door.expect("the map's door");
        let tenth = |v: u32| (f32::from_bits(v) / 10.0).round() as i16;
        a.world_mut().pc_command(piney_event::host::PcCommand::Put {
            pc: 0,
            x: tenth(at[0]),
            y: tenth(at[1]),
            z: tenth(at[2]),
        });
        let mut dungeon = false;
        for _ in 0..300 {
            pad.read(&still());
            s.step(&pad);
            s.take_events();
            if let Stage::Area(a) = &s.stage
                && matches!(a.world().place(), piney_world::field_world::Place::Dungeon(_))
            {
                dungeon = true;
                break;
            }
        }
        assert!(dungeon, "{disc}: field {field}'s door did not lead to its dungeon: {}", Mode::title(&s));
    }
}

/// Shots of the arena from area 66's last door to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/i74/arena): the arrival, then the boss out of
/// its portal. A diagnostic: `cargo test --release -p piney-game
/// mirror_world_shots -- --ignored --nocapture`.
#[test]
#[ignore]
fn mirror_world_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i74/arena".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = in_area_66_dungeon("mutation") else { return };
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    for _ in 0..200 {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
    }
    let Stage::Area(a) = &mut s.stage else { return };
    a.world_mut().change_area(1, 67);
    a.vm_mut().unwrap().disable();
    let mut put = false;
    for f in 0..900u32 {
        let b = if f.is_multiple_of(8) { piney_input::Buttons::CROSS } else { piney_input::Buttons::NONE };
        pad.read(&piney_input::Raw { buttons: b, ..still() });
        let frame = s.step(&pad);
        s.take_events();
        gs.set_overlay(Mode::archive(&s));
        gs.render(&frame);
        if f.is_multiple_of(100) {
            let (w, h) = gs.target_size();
            let path = format!("{dir}/arena-{f:04}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}");
        }
        if f == 400
            && !put
            && let Some(at) = portal(&s)
        {
            let tenth = |v: u32| (f32::from_bits(v) / 10.0).round() as i16;
            let Stage::Area(a) = &mut s.stage else { return };
            let (x, y, z) = (tenth(at[0]), tenth(at[1]) + 150, tenth(at[2]));
            a.world_mut().pc_command(piney_event::host::PcCommand::Put { pc: 0, x, y, z });
            put = true;
        }
    }
}
