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
fn place_kind(p: &piney_world::field::world::Place) -> &'static str {
    use piney_world::field::world::Place;
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
    let iso = crate::check_image(disc);
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
    Some(Session::in_world(iso, archive, None, state, Some(vm), scene, Some(wm)).unwrap())
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
            let in_dungeon = matches!(w.place(), piney_world::field::world::Place::Dungeon(_));
            if in_dungeon || !matches!(w.phase(), piney_world::Phase::Play(n) if n > 60) {
                continue;
            }
            let sky = w.place().story::<piney_world::story_map::sky::SkyMap>();
            arena |= sky.is_some_and(|m| m.class == piney_world::story_map::sky::Class::Arena);
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
    use piney_world::story_map::sky::{Class, SkyMap};
    for (disc, field, class) in
        [("mutation", 66, Class::Mirror), ("quarantine", 66, Class::Mirror), ("outbreak", 91, Class::Whale)]
    {
        let iso = crate::check_image(disc);
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
                && matches!(a.world().place(), piney_world::field::world::Place::Dungeon(_))
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

/// Mutation logged in to Dun Loireag (Theta) after event 115, its tasks
/// run 100 frames (its clouds draw from `fieldrand`).
fn in_dun_loireag() -> Option<(Session, &'static piney_data::area::AreaTables)> {
    let iso = crate::check_image("mutation");
    if !iso.exists() {
        eprintln!("mutation.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let tables = crate::area::area_tables(&mut d).unwrap();
    let (mut state, vm) = crate::start::state_at(&iso, 116).unwrap();
    state.save.set_u8(offset::LAST_TOWN, piney_world::DUN_LOIREAG as u8);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, Some(vm), scene, None).unwrap();
    let mut pad = Pad::default();
    for _ in 0..300 {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
    }
    assert!(matches!(s.stage, Stage::World(_)), "{}", Mode::title(&s));
    Some((s, tables))
}

/// The gate's words for area 66, and the session stepped until the
/// Mirror World's map is up: its `fieldrand` and scroll as made, before
/// its first stepped `Draw`.
fn gate_to_66(s: &mut Session, tables: &piney_data::area::AreaTables) -> (piney_data::dungeon::Rng, [u32; 2]) {
    use piney_world::story_map::sky::SkyMap;
    let code = piney_data::area::story_area_code(tables, 66, false).unwrap();
    s.go(Pending::Words([code.a, code.b, code.c]));
    let mut pad = Pad::default();
    for _ in 0..400 {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && let Some(m) = a.world().place().story::<SkyMap>()
        {
            assert!(!m.started, "the map drew before the first look");
            return (m.rng, m.move_tex.scroll());
        }
    }
    panic!("no Mirror World: {}", Mode::title(s));
}

/// Worklog 473: the sky maps' clouds draw from `fieldrand`, the game's one
/// stream. `GO(1)` sets no seed for a story map, so the gate's
/// `SimGenerateCode` (the code of area 66's words, moved on by 13 draws)
/// is where the Mirror World's clouds begin, whatever the town drew
/// before. Before, the map started its own from 0.
#[test]
fn the_mirror_worlds_clouds_draw_on_from_the_gates_words() {
    let Some((mut s, tables)) = in_dun_loireag() else { return };
    let code = piney_data::area::story_area_code(tables, 66, false).unwrap();
    let (rng, _) = gate_to_66(&mut s, tables);
    assert_eq!(rng.seed, code.seed);
    assert_ne!(rng.seed, 0);
}

/// Worklog 473: `EA_moveTex04`'s scroll is gcmn's data. It carries on
/// from the last visit to field 66 (out to the town and back), and only
/// `ccSetupNewGame`'s load of GCMN.PRG (log in) sets it to 0 again.
/// Before, each map started its scroll at 0.
#[test]
fn the_mirror_worlds_scroll_carries_across_visits() {
    use piney_world::story_map::sky::SkyMap;
    let Some((mut s, tables)) = in_dun_loireag() else { return };
    let (_, first) = gate_to_66(&mut s, tables);
    assert_eq!(first, [0, 0], "a new log in's scroll");
    let mut pad = Pad::default();
    for _ in 0..300 {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
    }
    // Gate Out: the scroll the map leaves is the last it drew.
    let Stage::Area(a) = &mut s.stage else { panic!("{}", Mode::title(&s)) };
    a.gate_out();
    a.vm_mut().unwrap().disable();
    let mut left = [0; 2];
    for _ in 0..400 {
        if let Stage::Area(a) = &s.stage
            && let Some(m) = a.world().place().story::<SkyMap>()
        {
            left = m.move_tex.scroll();
        }
        if matches!(s.stage, Stage::World(_)) {
            break;
        }
        pad.read(&still());
        s.step(&pad);
        s.take_events();
    }
    assert!(matches!(s.stage, Stage::World(_)), "{}", Mode::title(&s));
    assert_ne!(left, [0, 0], "the scroll never moved");
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(
        w.world().state().move_tex[piney_world::story_map::holy_ground::scroll::EA04],
        left,
        "the town carries gcmn's data"
    );
    for _ in 0..100 {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
    }
    let (_, again) = gate_to_66(&mut s, tables);
    assert_eq!(again, left, "the second visit's scroll");
    // Log in loads GCMN.PRG again.
    let Stage::Area(a) = &s.stage else { unreachable!() };
    let mut state = a.world().state_out();
    crate::session::log_in(&s.iso, &mut state, 0);
    assert_eq!(state.move_tex, [[0; 2]; piney_desktop::MOVE_TEX_ROWS]);
}

/// Worklog 473: area 15's `EA_moveTex02` is gcmn's data too: Infection's
/// holy ground, left by Gate Out and entered again by the gate's words,
/// goes on with the clouds' scroll where it was.
#[test]
fn the_holy_grounds_scroll_carries_across_visits() {
    use piney_world::story_map::holy_ground::{EventArea, scroll};
    let Some((iso, archive)) = crate::session::area15::disc() else { return };
    let mut d = Iso::open(&iso).unwrap();
    let tables = crate::area::area_tables(&mut d).unwrap();
    let code = piney_data::area::story_area_code(tables, 15, false).unwrap();
    let mut s = crate::session::area15::start(&iso, &archive, None);
    let mut pad = Pad::default();
    let mut step = |s: &mut Session| {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
    };
    let scroll_of = |s: &Session| match &s.stage {
        Stage::Area(a) => a.world().place().story::<EventArea>().map(|e| e.move_tex[0].scroll()),
        _ => None,
    };
    let mut visit = |s: &mut Session| {
        for _ in 0..200 {
            step(s);
        }
        s.go(Pending::Words([code.a, code.b, code.c]));
        let mut first = None;
        for _ in 0..700 {
            step(s);
            first = first.or(scroll_of(s));
        }
        let Stage::Area(a) = &mut s.stage else { panic!("{}", Mode::title(s)) };
        a.gate_out();
        a.vm_mut().into_iter().for_each(|v| v.disable());
        let mut left = None;
        for _ in 0..400 {
            left = scroll_of(s).or(left);
            if matches!(s.stage, Stage::World(_)) {
                break;
            }
            step(s);
        }
        (first.expect("the holy ground"), left.expect("the holy ground's last scroll"))
    };
    let (first, left) = visit(&mut s);
    assert_eq!(first, [0, 0], "a new log in's scroll");
    assert_ne!(left, [0, 0], "the scroll never moved");
    let Stage::World(w) = &s.stage else { panic!("{}", Mode::title(&s)) };
    assert_eq!(w.world().state().move_tex[scroll::EA02], left);
    let (again, _) = visit(&mut s);
    assert_eq!(again, left, "the second visit's scroll");
}

/// Worklog 474: Dun Loireag's `DrawBG` and `Draw` scrolls (`v$1346`,
/// `v2$1347`, `u$1777`) are statics of the executable's own data, set by
/// no load. The town left for field 66 and come back to by Gate Out goes
/// on where it was, and log in (GCMN.PRG loaded) leaves them. Before,
/// each visit started them at 0.
#[test]
fn dun_loireags_scrolls_carry_across_visits() {
    use piney_world::town::dun_loireag::DunLoireag;
    use piney_world::town::scroll_statics as statics;
    let Some((mut s, tables)) = in_dun_loireag() else { return };
    let town = |s: &Session| match &s.stage {
        Stage::World(w) => w.world().town().class::<DunLoireag>().map(|d| (d.bg_scroll, d.bg_scroll2, d.water_u)),
        _ => None,
    };
    let left = town(&s).expect("Dun Loireag");
    assert_ne!(left, ([0, 0], 0, 0), "the scrolls never moved");
    let (_, _) = gate_to_66(&mut s, tables);
    let Stage::Area(a) = &mut s.stage else { panic!("{}", Mode::title(&s)) };
    let carried = a.world().state_out().scroll_statics;
    a.gate_out();
    a.vm_mut().unwrap().disable();
    let mut pad = Pad::default();
    let mut back = None;
    for _ in 0..600 {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
        if let Some(t) = town(&s) {
            back = Some(t);
            break;
        }
    }
    let back = back.expect("back in Dun Loireag");
    // u$1777 steps 0.005 a frame and wraps at 1: the town back goes on
    // from what the field carried, a few steps on, not from 0.
    let carried_u = f32::from_bits(carried[statics::TOWN02_WATER]);
    let on = (f32::from_bits(back.2) - carried_u).rem_euclid(1.0);
    assert!(carried_u > 0.0, "the field carried no u$1777");
    assert!(on < 0.05, "u$1777 restarted: {carried_u} then {}", f32::from_bits(back.2));
    let carried_v = f32::from_bits(carried[statics::TOWN02_BG]);
    let v_on = (f32::from_bits(back.0[0]) - carried_v).rem_euclid(1.0);
    assert!(v_on < 0.1, "v$1346 restarted: {carried_v} then {}", f32::from_bits(back.0[0]));
    let mut state = match &s.stage {
        Stage::World(w) => w.world().state_out(),
        _ => unreachable!(),
    };
    let before = state.scroll_statics;
    crate::session::log_in(&s.iso, &mut state, 0);
    assert_eq!(state.scroll_statics, before, "log in leaves the executable's statics");
}
