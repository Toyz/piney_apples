//! A generated field's gimmicks: `WORLD_MAN::EntryGimmick`'s field
//! branch (`WORLD::SetFood`, `SetMagicCircle`, `SetSpecialObj`;
//! `docs/engine/battle.md`, "Where they come from") in random areas of Δ.
//! The party arrives by the gate and the field's entries stand: sixteen
//! portals and enemies, the dungeon entrance's two swirls (row 21), a
//! lake's Spring of Myst (row 20), the foods and symbols the objects'
//! nodes drew.

use piney_battle::entry::Kind;
use piney_world::area::{Scene, WorldMan, kind};
use piney_world::field_world::Place;

use super::*;

/// The first word triple in the tables' order (the first word running
/// fastest) that makes a random area on `server` with a field whose type
/// `lake` wants: a lake's (2, 3, 8, 9, 10) or not.
fn field_area(server: i32, lake: bool) -> Option<[i32; 3]> {
    field_area_by(server, lake, |_| true)
}

/// [`field_area`] of the first area whose code `pred` takes.
fn field_area_by(server: i32, lake: bool, pred: impl Fn(&piney_data::area::AreaCode) -> bool) -> Option<[i32; 3]> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut disc = Iso::open(&iso).ok()?;
    let t = crate::area::area_tables(&mut disc).unwrap();
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    for &z in &c {
        for &y in &b {
            for &x in &a {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, server, false) else { continue };
                let ft = code.attrs.field_type;
                if code.event != 0 || ft == 4 || matches!(ft, 2 | 3 | 8 | 9 | 10) != lake || !pred(&code) {
                    continue;
                }
                return Some([x, y, z]);
            }
        }
    }
    None
}

/// Event 25's start logged in at Mac Anu, the area of `words` made and the
/// party put in its field, as the gate leaves it (`ChangeArea(1, 0)`).
fn in_random_field(words: [i32; 3]) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let tables = crate::area::area_tables(&mut disc).unwrap();
    story_session_with(25, |start| {
        let save = &mut start.state.save;
        save.set_u8(offset::LAST_TOWN, 0);
        let mut scene = Scene::log_in(save);
        let (wm, _) = WorldMan::set_generate_code(tables, words, scene.server, save).expect("the words");
        scene.change_scene(1, 0, 0, -1, -1, -1, save);
        scene.change_area(kind::FIELD, 0, save);
        start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
            scene,
            world_man: Some(wm),
            spcs: Some(crate::start::party(25)),
        }));
    })
}

/// What the field's entry control holds once the field plays: the portals,
/// the enemies, and the gimmicks' rows.
fn entries(words: [i32; 3]) -> Option<(usize, usize, Vec<i32>, i32)> {
    let mut s = in_random_field(words)?;
    let mut pad = Pad::default();
    for _ in 0..900 {
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { continue };
        let w = a.world();
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 4);
        let Place::Field(f) = w.place() else { continue };
        if !playing {
            continue;
        }
        let c = w.combat();
        let gims =
            c.ctrl.list(Kind::Gimmick).into_iter().filter_map(|g| c.ctrl.entry_obj(g).map(|o| o.gim_id)).collect();
        return Some((
            c.ctrl.list(Kind::Circle).len(),
            c.ctrl.list(Kind::Enemy).len(),
            gims,
            f.field.params.field_type as i32,
        ));
    }
    panic!("the field never played");
}

#[test]
fn a_field_has_its_portals_entrance_and_foods() {
    let Some(words) = field_area(0, false) else { return };
    let Some((circles, enemies, gims, ft)) = entries(words) else { return };
    eprintln!("field {words:?} type {ft}: {circles} portals, {enemies} enemies, gimmicks {gims:?}");
    // SetMagicCircle: sixteen sites, 4, 8 or 12 of them portals by
    // circleOfs, the rest a registered row's enemies (a row may be a
    // group).
    assert!(matches!(circles, 4 | 8 | 12), "the field's portals: {circles}");
    assert!(enemies >= 16 - circles, "the other sites' enemies: {enemies}");
    // SetSpecialObj: the entrance's two in-points.
    assert_eq!(gims.iter().filter(|&&g| g == 21).count(), 2, "the entrance's swirls");
    assert!(!gims.contains(&20), "no lake, no spring");
    // SetFood: every food the field's type gives.
    let food = piney_battle::entry::field_food(ft);
    assert!(gims.iter().all(|&g| matches!(g, 18 | 21) || g == food), "only symbols, swirls and {food}: {gims:?}");
}

#[test]
fn a_lake_field_has_its_spring() {
    let Some(words) = field_area(0, true) else { return };
    let Some((circles, enemies, gims, ft)) = entries(words) else { return };
    eprintln!("lake field {words:?} type {ft}: {circles} portals, {enemies} enemies, gimmicks {gims:?}");
    assert_eq!(gims.iter().filter(|&&g| g == 20).count(), 1, "the lake's spring");
    assert_eq!(gims.iter().filter(|&&g| g == 21).count(), 2, "the entrance's swirls");
}

/// Whether enemy `i` is of a race past the opening's (not B, G, K, P, V)
/// and alive.
fn other_race(c: &piney_world::combat::Combat, i: usize) -> bool {
    c.foes.get(i).and_then(|f| f.as_ref()).is_some_and(|e| {
        let race = piney_battle::enemy_ai::enemy_race(&c.data.t, e.ene_id).map_or(-1, |r| r.0);
        !matches!(race, 5 | 10 | 13 | 15 | 19) && c.scene.chars[i].hp > 0
    })
}

/// The pad for frame `n`: Kite walks up to the nearest enemy of another
/// race ([`other_race`]) and, within 200, attacks every 8th frame; and
/// that enemy's distance.
fn toward_other(s: &Session, n: u32) -> (Raw, Option<f32>) {
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let Stage::Area(a) = &s.stage else { return (still, None) };
    let w = a.world();
    let c = w.combat();
    let Some(k) = c.kite else { return (still, None) };
    if !matches!(w.phase(), piney_world::Phase::Play(t) if t > 12) {
        return (still, None);
    }
    let p = c.scene.chars[k].pos.map(f32::from_bits);
    let d = |i: usize| {
        let q = c.scene.chars[i].pos.map(f32::from_bits);
        (q[0] - p[0]).hypot(q[1] - p[1])
    };
    let near = c
        .ctrl
        .list(Kind::Enemy)
        .into_iter()
        .filter(|&i| other_race(c, i))
        .min_by(|&a, &b| d(a).partial_cmp(&d(b)).unwrap_or(std::cmp::Ordering::Equal));
    match near {
        Some(e) if d(e) > 200.0 => {
            let q = c.scene.chars[e].pos.map(f32::from_bits);
            let cam_z = f32::from_bits(w.camera().rot()[2]);
            (stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))), Some(d(e)))
        }
        Some(e) if n.is_multiple_of(8) => (Raw { buttons: Buttons::CROSS, ..still }, Some(d(e))),
        Some(e) => (still, Some(d(e))),
        None => (still, None),
    }
}

/// A higher field of Δ (area level 3 or more: ranks past 50, where the
/// crabs, dogs, snakoids, bats, rocks and ghosts are registered): Kite
/// walks up to the nearest enemy of a race past the opening's and fights
/// it; those enemies walk, turn and act.
#[test]
fn a_higher_fields_other_races_move() {
    let Some(words) = field_area_by(0, false, |c| c.attrs.area_level >= 3) else { return };
    let Some(mut s) = in_random_field(words) else { return };
    let mut pad = Pad::default();
    let mut first: std::collections::HashMap<usize, (i32, [u32; 4])> = Default::default();
    let mut moved = std::collections::BTreeSet::new();
    let mut acted = std::collections::BTreeSet::new();
    for n in 0..3000u32 {
        pad.read(&toward_other(&s, n).0);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        let c = a.world().combat();
        for i in c.ctrl.list(Kind::Enemy) {
            let Some(e) = c.foes.get(i).and_then(|f| f.as_ref()) else { continue };
            let race = piney_battle::enemy_ai::enemy_race(&c.data.t, e.ene_id).map_or(-1, |r| r.0);
            if matches!(race, 5 | 10 | 13 | 15 | 19) {
                continue;
            }
            let pos = c.scene.chars[i].pos;
            let f = *first.entry(i).or_insert((e.ene_id, pos));
            if f.0 == e.ene_id && pos != f.1 {
                moved.insert(e.ene_id);
            }
            if e.act_num != 0 {
                acted.insert((e.ene_id, e.act_num));
            }
        }
    }
    let rows: std::collections::BTreeSet<i32> = first.values().map(|f| f.0).collect();
    eprintln!("field {words:?}: other races' rows {rows:?}, moved {moved:?}, acts {acted:?}");
    if rows.is_empty() {
        return;
    }
    assert!(!moved.is_empty(), "none of {rows:?} moved");
    assert!(acted.iter().any(|&(_, a)| a == 6), "none of {rows:?} attacked: {acted:?}");
}

/// The breathers: `ccEnemyH` types 3-4 (Cerberus, Flame Heads, Black
/// Death) and `ccEnemyL` types 2 and 4 (the wyrms, the dragons).
fn breathes(row: i32) -> bool {
    (174..=177).contains(&row) || (197..=202).contains(&row) || (207..=217).contains(&row)
}

/// Pictures of a fight with a breather: the first level 3-5 area of Δ
/// (the words in the tables' order) whose field holds one; Kite walks up
/// to it and the frames are kept every 5th frame while one of its flames
/// burns. `PINEY_SHOTS=DIR cargo test --release -p piney-game
/// breath_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/races/breath`).
#[test]
#[ignore]
fn breath_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/races/breath".into());
    std::fs::create_dir_all(&dir).unwrap();
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let Ok(mut disc) = Iso::open(&iso) else { return };
    let t = crate::area::area_tables(&mut disc).unwrap();
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    // the triples in an order shuffled by a fixed LCG, so that the search
    // meets several words of each slot early
    let mut all = Vec::new();
    for &z in &c {
        for &y in &b {
            for &x in &a {
                all.push([x, y, z]);
            }
        }
    }
    let mut seed = 12345u64;
    for i in (1..all.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        all.swap(i, (seed >> 33) as usize % (i + 1));
    }
    let mut tried = 0;
    for [x, y, z] in all {
        {
            {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, 0, false) else { continue };
                let ft = code.attrs.field_type;
                if code.event != 0 || ft == 4 || code.attrs.area_level < 3 {
                    continue;
                }
                tried += 1;
                if tried > 80 {
                    return;
                }
                let Some(mut s) = in_random_field([x, y, z]) else { return };
                let mut pad = Pad::default();
                let mut found = None;
                for _ in 0..200 {
                    pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
                    s.step(&pad);
                    s.take_events();
                    let Stage::Area(a) = &s.stage else { continue };
                    let c = a.world().combat();
                    found = c.ctrl.list(Kind::Enemy).into_iter().find_map(|i| {
                        c.foes.get(i).and_then(|f| f.as_ref()).map(|e| e.ene_id).filter(|&r| breathes(r))
                    });
                    if found.is_some() {
                        break;
                    }
                }
                let Some(row) = found else { continue };
                eprintln!("words {:?}: a breather, row {row}", [x, y, z]);
                let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
                gs.set_overlay(Mode::archive(&s));
                let mut shots = 0;
                for n in 0..3000u32 {
                    let raw = {
                        let Stage::Area(a) = &s.stage else { break };
                        let w = a.world();
                        let c = w.combat();
                        match (
                            c.kite,
                            c.ctrl.list(Kind::Enemy).into_iter().find(|&i| {
                                c.foes.get(i).and_then(|f| f.as_ref()).is_some_and(|e| breathes(e.ene_id))
                                    && c.scene.chars[i].hp > 0
                            }),
                        ) {
                            (Some(k), Some(e)) if matches!(w.phase(), piney_world::Phase::Play(t) if t > 12) => {
                                let p = c.scene.chars[k].pos.map(f32::from_bits);
                                let q = c.scene.chars[e].pos.map(f32::from_bits);
                                if (q[0] - p[0]).hypot(q[1] - p[1]) > 250.0 {
                                    let cam_z = f32::from_bits(w.camera().rot()[2]);
                                    stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                                } else {
                                    Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
                                }
                            }
                            _ => Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() },
                        }
                    };
                    // Kite (level 1) is kept alive and the breathers
                    // unkillable (Orca would fell one at a blow), so that
                    // the fight goes on to their breath.
                    if let Stage::Area(a) = &mut s.stage {
                        let c = a.world_mut().combat_mut();
                        for i in 0..c.scene.chars.len() {
                            let foe = c.foes.get(i).and_then(|f| f.as_ref()).map(|e| e.ene_id);
                            let ch = &mut c.scene.chars[i];
                            if Some(i) == c.kite {
                                ch.hp = ch.max_hp;
                            } else if foe.is_some_and(breathes) {
                                ch.max_hp = 30000;
                                ch.hp = 30000;
                            }
                        }
                    }
                    pad.read(&raw);
                    let frame = s.step(&pad);
                    s.take_events();
                    gs.render(&frame);
                    let Stage::Area(a) = &s.stage else { break };
                    let c = a.world().combat();
                    let burning = c.breaths.map.values().any(|b| b.count > 0);
                    if burning && n % 5 == 0 && shots < 40 {
                        let (wd, h) = gs.target_size();
                        let path = format!("{dir}/row{row}-{n:04}.png");
                        std::fs::write(&path, piney_gs::png::encode(wd, h, &gs.read_back())).unwrap();
                        println!("{path}");
                        shots += 1;
                    }
                }
                let Stage::Area(a) = &s.stage else { return };
                let c = a.world().combat();
                eprintln!(
                    "flames drawn {}, bursts {}, breaths {}",
                    c.breaths.drawn,
                    c.breaths.bursts,
                    c.breaths.map.len()
                );
                return;
            }
        }
    }
}

/// A stress run: `PINEY_FIELDS` (default 40) random fields of Δ in a
/// shuffled order, 1500 frames each with Kite (kept alive) walking up to
/// the nearest enemy of another race and fighting it: no panic, and the
/// rows met and the acts they took are printed. `cargo test --release -p
/// piney-game many_fields_fight -- --ignored --nocapture`.
#[test]
#[ignore]
fn many_fields_fight() {
    let n: usize = std::env::var("PINEY_FIELDS").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let Ok(mut disc) = Iso::open(&iso) else { return };
    let t = crate::area::area_tables(&mut disc).unwrap();
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    let mut all = Vec::new();
    for &z in &c {
        for &y in &b {
            for &x in &a {
                all.push([x, y, z]);
            }
        }
    }
    let mut seed = 987_654_321u64;
    for i in (1..all.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        all.swap(i, (seed >> 33) as usize % (i + 1));
    }
    let mut met = std::collections::BTreeMap::<i32, std::collections::BTreeSet<i16>>::new();
    let mut done = 0;
    for [x, y, z] in all {
        let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, 0, false) else { continue };
        if code.event != 0 || code.attrs.field_type == 4 {
            continue;
        }
        done += 1;
        if done > n {
            break;
        }
        let Some(mut s) = in_random_field([x, y, z]) else { return };
        let mut pad = Pad::default();
        for f in 0..1500u32 {
            if let Stage::Area(a) = &mut s.stage {
                let c = a.world_mut().combat_mut();
                if let Some(k) = c.kite {
                    let ch = &mut c.scene.chars[k];
                    ch.hp = ch.max_hp;
                }
            }
            pad.read(&toward_other(&s, f).0);
            s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { break };
            let c = a.world().combat();
            for i in c.ctrl.list(Kind::Enemy) {
                if let Some(e) = c.foes.get(i).and_then(|f| f.as_ref()) {
                    met.entry(e.ene_id).or_default().insert(e.act_num);
                }
            }
        }
        eprintln!("{:?} level {} done", [x, y, z], code.attrs.area_level);
    }
    for (row, acts) in &met {
        eprintln!("row {row}: acts {acts:?}");
    }
}

/// Pictures of fights with the other races in higher fields of Δ (area
/// levels 3, 4 and 5), every 15th frame while Kite is within 1500 of one.
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game higher_field_shots
/// -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/races/shots`).
#[test]
#[ignore]
fn higher_field_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/races/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    for level in [3, 4, 5] {
        let Some(words) = field_area_by(0, false, |c| c.attrs.area_level == level) else { continue };
        let Some(mut s) = in_random_field(words) else { return };
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
        gs.set_overlay(Mode::archive(&s));
        let mut pad = Pad::default();
        let mut shots = 0;
        for n in 0..2400u32 {
            let (raw, d) = toward_other(&s, n);
            pad.read(&raw);
            let frame = s.step(&pad);
            s.take_events();
            gs.render(&frame);
            if d.is_some_and(|d| d < 1500.0) && n % 15 == 0 && shots < 40 {
                let (wd, h) = gs.target_size();
                let [x, y, z] = words;
                let path = format!("{dir}/l{level}-{x}-{y}-{z}-{n:04}.png");
                std::fs::write(&path, piney_gs::png::encode(wd, h, &gs.read_back())).unwrap();
                println!("{path}");
                shots += 1;
            }
        }
    }
}

/// Pictures of the two fields: the arrival (frames 30, 90), then a walk
/// ahead, every 60th frame. `PINEY_SHOTS=DIR cargo test --release -p
/// piney-game field_gims_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/fieldgim/shots`).
#[test]
#[ignore]
fn field_gims_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/fieldgim/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    for lake in [false, true] {
        let Some(words) = field_area(0, lake) else { return };
        let Some(mut s) = in_random_field(words) else { return };
        let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
        gs.set_overlay(Mode::archive(&s));
        let mut pad = Pad::default();
        for n in 0..900u32 {
            let ly = if n > 120 { 0 } else { 128 };
            pad.read(&Raw { analog: true, lx: 128, ly, rx: 128, ry: 128, ..Raw::default() });
            let frame = s.step(&pad);
            s.take_events();
            gs.render(&frame);
            let Stage::Area(a) = &s.stage else { continue };
            let piney_world::Phase::Play(p) = a.world().phase() else { continue };
            if p == 30 || p == 90 || (p > 120 && p % 60 == 0) {
                let (wd, h) = gs.target_size();
                let [x, y, z] = words;
                let path = format!("{dir}/{x}-{y}-{z}-{p:03}.png");
                std::fs::write(&path, piney_gs::png::encode(wd, h, &gs.read_back())).unwrap();
                println!("{path}");
            }
        }
    }
}
