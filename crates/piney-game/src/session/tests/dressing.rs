//! The dungeon rooms' dressing (`DUNGEON::SetRoom`'s `SetWater`,
//! `SetLight`, `SetObject` and `SetAnmObject`; `docs/engine/dungeon.md`,
//! "The dressing") in random dungeons of Δ (Mac Anu's server) and Θ (Dun
//! Loireag's): Kite walks from the entrance to the first floor's down
//! stairs, and each room he passes through stands its pieces, its glows
//! flickering and their omni lights in the room's light group. A lake by
//! night has its sky and its fireflies.

use std::collections::{BTreeMap, BTreeSet};

use piney_world::area::{Scene, WorldMan, kind};
use piney_world::dungeon_area::DungeonArea;
use piney_world::field_world::Place;

use super::shrine::walk_to;
use super::*;

/// For each dungeon type of `types`, the first word triple in the tables'
/// order (the first word running fastest, then the second) that makes a
/// random area on `server` with a field (so its dungeon is not a lake)
/// whose first dungeon is of that type.
pub(super) fn random_areas(server: i32, types: &[u8]) -> Vec<[i32; 3]> {
    areas_where(server, types, false)
}

/// [`random_areas`] of field type 4 (no field: the lakes, types 8 and 9).
fn lake_areas(server: i32, types: &[u8]) -> Vec<[i32; 3]> {
    areas_where(server, types, true)
}

/// The first word triple for each dungeon type of `types` that makes a
/// random area on `server` of field type 4 (`lake`) or not.
fn areas_where(server: i32, types: &[u8], lake: bool) -> Vec<[i32; 3]> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut disc = Iso::open(&iso).unwrap();
    let t = crate::area::area_tables(&mut disc).unwrap();
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    let mut found: BTreeMap<u8, [i32; 3]> = BTreeMap::new();
    'search: for &z in &c {
        for &y in &b {
            for &x in &a {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, server, false) else { continue };
                if code.event != 0 || (code.attrs.field_type == 4) != lake {
                    continue;
                }
                found.entry(code.dungeon_type(t, false)[0]).or_insert([x, y, z]);
                if types.iter().all(|ty| found.contains_key(ty)) {
                    break 'search;
                }
            }
        }
    }
    types.iter().filter_map(|ty| found.get(ty).copied()).collect()
}

/// Event 25's start (its party, their levels and gear) logged in at Root
/// Town `town` (0 Mac Anu, Δ; 1 Dun Loireag, Θ), the area of `words` made
/// (`WORLD_MAN::SetGenerateCode`) and the party put in its dungeon's first
/// room, as the field's entrance leaves it (`ChangeArea(2, 0)`). None
/// without the disc.
pub(super) fn in_random_dungeon(town: i32, words: [i32; 3]) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let tables = crate::area::area_tables(&mut disc).unwrap();
    story_session_with(25, |start| {
        let save = &mut start.state.save;
        save.set_u8(offset::LAST_TOWN, town as u8);
        let mut scene = Scene::log_in(save);
        let (wm, _) = WorldMan::set_generate_code(tables, words, scene.server, save).expect("the words");
        scene.change_scene(1, town, 0, -1, -1, -1, save);
        scene.change_area(kind::DUNGEON, 0, save);
        start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
            scene,
            world_man: Some(wm),
            spcs: Some(crate::start::party(25)),
        }));
    })
}

/// What one room showed: its dungeon type, the dressing's counts (room
/// lights, sparks, clumps, walls, animated objects, water), the light
/// group's size, and the glows' patterns seen.
#[derive(Debug, Default)]
struct Seen {
    dtype: u8,
    pieces: [usize; 6],
    group: usize,
    patterns: BTreeSet<Vec<u16>>,
}

/// Kite's walk in each of the areas from the entrance to the rooms `pick`
/// chooses, then floor 0's down stairs, `frames` frames each; `each` also
/// sees every frame. The rooms seen, by (words, floor, block).
fn walk_dressed(
    town: i32,
    areas: &[[i32; 3]],
    pick: impl Fn(&DungeonArea) -> Vec<(usize, usize)>,
    frames: u64,
    mut each: impl FnMut(&Session, &Frame, [i32; 3]),
) -> BTreeMap<([i32; 3], i32, i32), Seen> {
    let mut seen: BTreeMap<([i32; 3], i32, i32), Seen> = BTreeMap::new();
    for &words in areas {
        let Some(mut s) = in_random_dungeon(town, words) else { return seen };
        // Until the dungeon is up, then its first floor's down stairs.
        let mut pad = Pad::default();
        let mut to = None;
        for _ in 0..600 {
            if let Stage::Area(a) = &s.stage
                && let Place::Dungeon(d) = a.world().place()
            {
                to = Some((0, d.floors[0].down));
                break;
            }
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        let to = to.expect("in the dungeon");
        let mut targets: Vec<(usize, usize)> = match &s.stage {
            Stage::Area(a) => match a.world().place() {
                Place::Dungeon(d) => pick(d),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        targets.push(to);
        for to in targets {
            walk_to(&mut s, to, frames, |s, _, frame| {
                each(s, frame, words);
                let Stage::Area(a) = &s.stage else { return };
                let w = a.world();
                let sc = w.scene();
                let Place::Dungeon(d) = w.place() else { return };
                let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 2);
                if sc.area != kind::DUNGEON || !playing || d.room_at != Some((sc.floor as usize, sc.block as usize)) {
                    return;
                }
                let x = &d.dress;
                let e = seen.entry((words, sc.floor, sc.block)).or_default();
                e.dtype = d.dtype;
                e.pieces = [
                    x.lights.len(),
                    x.sparks.len(),
                    x.objects.len(),
                    x.anm_objects.len(),
                    x.anm_objects2.len(),
                    usize::from(x.water.is_some()),
                ];
                e.group = d.lights.lights.len();
                e.patterns.insert(x.lights.iter().map(|r| r.pat).collect());
            });
        }
    }
    seen
}

/// Floor 0's rooms in the order the generator made them, the first five
/// past the entrance.
fn first_rooms(d: &DungeonArea) -> Vec<(usize, usize)> {
    d.floors[0].order.iter().filter(|&&i| i != 0).map(|&i| (0, i)).take(5).collect()
}

/// The rooms (at most four, floor by floor) whose model holds a magma
/// dummy (sparks), the water's, a wall's, an animated object's or a lake
/// statue's.
fn rooms_with_pieces(d: &DungeonArea) -> Vec<(usize, usize)> {
    const DUMMIES: [&str; 8] =
        ["OBJ_o_magma", "OBJ_0paf0_", "OBJ_0paw", "OBJ_0pag0_", "OBJ_0pac0_", "OBJ_0par0_", "OBJ_0pab0_", "OBJ_0ps"];
    let f = &d.file;
    let holds = |model: &str| {
        f.anim(model).is_some_and(|k| {
            f.anims[k]
                .tracks
                .iter()
                .any(|t| f.ccs.object_name(t.target).is_some_and(|n| DUMMIES.iter().any(|p| n.starts_with(p))))
        })
    };
    let mut out = Vec::new();
    for (fl, floor) in d.floors.iter().enumerate() {
        for &i in &floor.order {
            if (fl, i) != (0, 0) && floor.rooms[i].model.is_some_and(holds) {
                out.push((fl, i));
            }
        }
    }
    out.truncate(4);
    out
}

/// Δ's random dungeons of types 1 and 2 and Θ's of types 0 and 3: every
/// room walked through stands its dressing with its omni lights in the
/// group beside the dungeon's distant light, and its glows' patterns run on
/// frame by frame; between them the rooms hold room lights, and sparks,
/// walls or animated objects.
#[test]
fn delta_and_theta_rooms_are_dressed() {
    let mut rooms = 0;
    let mut totals = [0usize; 6];
    for (town, server, types) in [(0, "Δ", [1, 2]), (1, "Θ", [0, 3])] {
        let areas = random_areas(town, &types);
        assert_eq!(areas.len(), 2, "{server}: random areas of dungeon types {types:?}");
        let seen = walk_dressed(town, &areas, first_rooms, 1500, |_, _, _| {});
        if seen.is_empty() {
            return;
        }
        assert!(seen.len() >= 3, "{server}: rooms walked through: {seen:?}");
        for (key, e) in &seen {
            println!("{server} {key:?}: type {} pieces {:?} group {}", e.dtype, e.pieces, e.group);
            assert_eq!(e.group, 1 + e.pieces[0], "{server} {key:?}: the distant light and one omni light a room light");
            if e.pieces[0] > 0 {
                assert!(e.patterns.len() > 1, "{server} {key:?}: the glows' patterns run on");
            }
            for (t, n) in totals.iter_mut().zip(e.pieces) {
                *t += n;
            }
        }
        assert!(seen.values().any(|e| e.pieces[0] > 0), "{server}: room lights");
        rooms += seen.len();
    }
    println!("rooms {rooms}, lights/sparks/clumps/walls/animated/water {totals:?}");
    assert!(totals[1] + totals[3] + totals[4] > 0, "sparks, walls or animated objects: {totals:?}");
}

/// Pictures of dressed rooms: Kite's walks in Δ's random dungeons of types
/// 0 and 1, Θ's of type 3 and two Δ lakes (type 8, the second by night:
/// [`night_lake`]) to rooms with sparks,
/// water, walls, animated objects or statues; each room's 30th, 60th and 150th frame of play,
/// named by server, area, floor and block.
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game dressed_room_shots
/// -- --ignored --nocapture` (default `/mnt/data/claude/scratch/dressing`).
#[test]
#[ignore]
fn dressed_room_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/dressing".into());
    std::fs::create_dir_all(&dir).unwrap();
    let night = || night_lake(0).into_iter().collect::<Vec<_>>();
    for (town, areas) in
        [(0, random_areas(0, &[0, 1])), (1, random_areas(1, &[3])), (0, lake_areas(0, &[8])), (0, night())]
    {
        let mut gs: Option<piney_gs::Gs> = None;
        walk_dressed(town, &areas, rooms_with_pieces, 3000, |s, frame, words| {
            let gs =
                gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            gs.set_overlay(Mode::archive(s));
            gs.render(frame);
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let sc = w.scene();
            let Place::Dungeon(d) = w.place() else { return };
            let at = [30, 60, 150].into_iter().find(|&n| w.phase() == piney_world::Phase::Play(n));
            if sc.area == kind::DUNGEON
                && let Some(n) = at
            {
                let (wd, h) = gs.target_size();
                let [x, y, z] = words;
                let path = format!("{dir}/t{town}-{x}-{y}-{z}-f{}-b{}-{n:03}.png", sc.floor, sc.block);
                std::fs::write(&path, piney_gs::png::encode(wd, h, &gs.read_back())).unwrap();
                let p = &d.dress;
                println!(
                    "{path}: type {} lights {} sparks {} walls {} animated {} water {} statues {}",
                    d.dtype,
                    p.lights.len(),
                    p.sparks.len(),
                    p.anm_objects.len(),
                    p.anm_objects2.len(),
                    p.water.is_some(),
                    p.objects.len()
                );
            }
        });
    }
}

/// The first word triple (in [`areas_where`]'s order) that makes a random
/// area on `server` of field type 4 with bgnum 2: a lake by night
/// (`WORLD_MAN::GetTime` 2).
fn night_lake(server: i32) -> Option<[i32; 3]> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut disc = Iso::open(&iso).ok()?;
    let t = crate::area::area_tables(&mut disc).unwrap();
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    for &z in &c {
        for &y in &b {
            for &x in &a {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, server, false) else { continue };
                if code.event == 0 && code.attrs.field_type == 4 && code.attrs.weather == 2 {
                    return Some([x, y, z]);
                }
            }
        }
    }
    None
}

/// A Δ lake by night (`docs/engine/dungeon.md`, "The lakes' sky and
/// fireflies"): the sky's four clumps (`DrawBG`), bgnum 2's, are the
/// frame's first models, its material's scroll runs, and `SetRoom`'s five
/// fireflies fly about the room.
#[test]
fn a_lake_by_night_has_its_sky_and_fireflies() {
    let Some(words) = night_lake(0) else { return };
    let Some(mut s) = in_random_dungeon(0, words) else { return };
    let mut pad = Pad::default();
    let mut step = |s: &mut Session| {
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        let frame = s.step(&pad);
        s.take_events();
        frame
    };
    for _ in 0..200 {
        step(&mut s);
    }
    let lake_of = |s: &Session| -> piney_world::dungeon_area::lake::Lake {
        let Stage::Area(a) = &s.stage else { panic!("in an area") };
        let Place::Dungeon(d) = a.world().place() else { panic!("in the dungeon") };
        d.lake.clone().expect("a lake")
    };
    let before = lake_of(&s);
    let frame = step(&mut s);
    let after = lake_of(&s);
    assert!(after.night, "bgnum 2 is the lakes' night");
    assert_eq!(after.fireflies.len(), 5, "SetRoom's fireflies");
    assert_ne!(before.v, after.v, "the sky's scroll runs");
    for (a, b) in before.fireflies.iter().zip(&after.fireflies) {
        assert_ne!(a.pos, b.pos, "each firefly flies");
    }
    let Stage::Area(a) = &s.stage else { unreachable!() };
    let Place::Dungeon(d) = a.world().place() else { unreachable!() };
    let sky: Vec<u32> = ["bac_l", "bac_m", "clo_l", "clo_m"]
        .iter()
        .map(|n| d.file.ccs.find_object(&format!("MDL_o_{n}2_")).expect(n))
        .collect();
    let first: Vec<u32> = frame
        .cmds
        .iter()
        .filter_map(|c| match c {
            piney_draw::Cmd::Model(m) => Some(m.model),
            _ => None,
        })
        .take(4)
        .collect();
    assert_eq!(first, sky, "the sky's clumps first (layers -100 to -70)");
}

/// The Spring of Myst in the Δ lake by night (room 8 of its floor):
/// Kite walks up to it and OK is pressed through `FountainMenu` (40),
/// `FountainMenu2` (41: the bag's first item), `FountainMenu3` (42:
/// "Neither"), and the item handed out (67 into 29). The spring rises, talks and goes
/// (`ctrlFountain` states 0-11): its area among the fountains used
/// (`SetFountain`), the throw counted, the item gone from the bag.
#[test]
fn the_spring_of_myst_takes_an_item() {
    let Some(words) = night_lake(0) else { return };
    let Some(mut s) = in_random_dungeon(0, words) else { return };
    // Into room 8.
    walk_to(&mut s, (0, 8), 8000, |_, _, _| {});
    if let Stage::Area(a) = &s.stage {
        let sc = a.world().scene();
        assert_eq!((sc.area, sc.floor, sc.block), (kind::DUNGEON, 0, 8), "Kite in room 8");
    }
    let spring = |s: &Session| -> Option<usize> {
        let Stage::Area(a) = &s.stage else { return None };
        let c = a.world().combat();
        c.ctrl
            .list(piney_battle::entry::Kind::Gimmick)
            .into_iter()
            .find(|&g| c.ctrl.entry_obj(g).is_some_and(|o| o.gim_id == 20))
    };
    assert!(spring(&s).is_some(), "the spring stands in room 8");
    let bag = |s: &Session| -> Vec<i32> {
        let Stage::Area(a) = &s.stage else { return Vec::new() };
        let b = a.world().state().save.bytes().to_vec();
        (0..40)
            .map(|k| 0x30 + 4 * k)
            .filter(|&at| (0..10).contains(&(b[at + 2] as i8)))
            .map(|at| (i32::from(b[at + 2] as i8) << 16) | i32::from(u16::from_le_bytes([b[at], b[at + 1]])))
            .collect()
    };
    // Event 25's bag holds nothing the spring takes (categories 0-9): a
    // blade of category 0, id 1 (feTbl[0]'s second row, within server 0's
    // five), into the first empty slot.
    if let Stage::Area(a) = &mut s.stage {
        let save = &mut a.world_mut().state_mut().save;
        if let Some(k) = (0..40).find(|&k| save.u8(0x30 + 4 * k + 2) == 0xff) {
            let at = 0x30 + 4 * k;
            save.set_i16(at, 1);
            save.set_u8(at + 2, 0);
            save.set_u8(at + 3, 1);
        }
    }
    let before = bag(&s);
    assert_eq!(before, vec![1], "the blade to throw in");
    let mut pad = Pad::default();
    let mut menus = Vec::new();
    let mut gone_at = None;
    for f in 0..4000u64 {
        let raw = {
            let Stage::Area(a) = &s.stage else { break };
            let w = a.world();
            let c = w.combat();
            let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let menu = a.ui().menu_type();
            if menus.last() != Some(&menu) {
                menus.push(menu);
            }
            match (spring(&s), c.kite) {
                (Some(g), Some(k)) if menu == -1 && menus.len() <= 2 => {
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let q = c.scene.chars[g].pos.map(f32::from_bits);
                    let cam_z = f32::from_bits(w.camera().rot()[2]);
                    let targeted = w.command_target() == Some(g);
                    if !targeted && (q[0] - p[0]).hypot(q[1] - p[1]) > 300.0 {
                        stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                    } else if f.is_multiple_of(8) {
                        Raw { buttons: Buttons::CROSS, ..still }
                    } else {
                        still
                    }
                }
                _ if f.is_multiple_of(10) => Raw { buttons: Buttons::CROSS, ..still },
                _ => still,
            }
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if gone_at.is_none() && spring(&s).is_none() {
            gone_at = Some(f);
        }
        if gone_at.is_some_and(|g| f > g + 400) {
            break;
        }
    }
    eprintln!("menus {menus:?}, the spring gone at {gone_at:?}");
    for m in [40, 41, 42, 29] {
        assert!(menus.contains(&m), "menu {m} opened: {menus:?}");
    }
    assert!(gone_at.is_some(), "the spring went");
    let Stage::Area(a) = &s.stage else { panic!("still in the area") };
    let save = &a.world().state().save;
    let code = words[0] * 1_000_000 + words[1] * 1000 + words[2];
    assert!(piney_battle::entry::check_fountain(save, 0, code), "SetFountain(area's code)");
    assert_eq!(save.i16(0x7470), 1, "fountainCount[0]");
    // bgnum 2 by night: a weapon one row on (Looks like it changed
    // into): feTbl[0]'s third, id 2.
    let after = bag(&s);
    eprintln!("bag before {before:x?}\nbag after {after:x?}");
    assert_eq!(after, vec![2], "the blade changed into the next");
}

/// A stress run through dungeons: `PINEY_DUNGEONS` (default 16) random
/// areas of Δ in a shuffled order, Kite walking from the entrance to floor
/// 0's down stairs and fighting what stands in the way (`walk_to`), 6000
/// frames each: no panic; the rows met are printed. `cargo test --release
/// -p piney-game many_dungeons_walk -- --ignored --nocapture`.
#[test]
#[ignore]
fn many_dungeons_walk() {
    let n: usize = std::env::var("PINEY_DUNGEONS").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
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
    let mut seed = 424_242u64;
    for i in (1..all.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        all.swap(i, (seed >> 33) as usize % (i + 1));
    }
    let mut met: BTreeMap<i32, BTreeSet<i16>> = BTreeMap::new();
    let mut done = 0;
    for w in all {
        let Some(code) = piney_data::area::sim_generate_code(t, w[0], w[1], w[2], 0, false) else { continue };
        if code.event != 0 {
            continue;
        }
        done += 1;
        if done > n {
            break;
        }
        let Some(mut s) = in_random_dungeon(0, w) else { return };
        let mut pad = Pad::default();
        let mut to = None;
        for _ in 0..600 {
            if let Stage::Area(a) = &s.stage
                && let Place::Dungeon(d) = a.world().place()
            {
                to = Some((0, d.floors[0].down));
                break;
            }
            pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
            s.step(&pad);
            s.take_events();
        }
        let Some(to) = to else { continue };
        let mut rooms = BTreeSet::new();
        walk_to(&mut s, to, 6000, |s, _, _| {
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            let sc = w.scene();
            rooms.insert((sc.floor, sc.block));
            let c = w.combat();
            for i in c.ctrl.list(piney_battle::entry::Kind::Enemy) {
                if let Some(e) = c.foes.get(i).and_then(|f| f.as_ref()) {
                    met.entry(e.ene_id).or_default().insert(e.act_num);
                }
            }
        });
        eprintln!("{w:?} level {}: rooms {rooms:?}", code.attrs.area_level);
    }
    for (row, acts) in &met {
        eprintln!("row {row}: acts {acts:?}");
    }
}

/// `ccFellow::Initialize`: into a dungeon (and from room to room) the
/// party members stand where they are put (act 2); they come in through
/// the gate (act 13, its effect and sound) only in a town or a field
/// come to from one.
#[test]
fn members_stand_in_a_dungeon() {
    let Some(words) = random_areas(0, &[0]).into_iter().next() else { return };
    let Some(mut s) = in_random_dungeon(0, words) else { return };
    let mut pad = Pad::default();
    let mut seen = BTreeSet::new();
    for _ in 0..300 {
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { continue };
        let c = a.world().combat();
        for &(_, who) in &c.members {
            seen.insert(c.scene.chars[who].spc_char.act_num);
        }
    }
    assert!(!seen.is_empty(), "the members were built");
    assert!(!seen.contains(&piney_battle::fellow::act::TRANSFER_IN), "no transfer in: {seen:?}");
}
