//! Golden shots of the fields and dungeons on every volume, for the
//! clean-ups of their systems: each field type and background row, plain
//! and hacked; storms long enough for their first flash; TOBJ on each
//! server; each dungeon type's dressed rooms, the lakes by day, by night and
//! hacked, and type 32's tree and leaves. A run writes every frame's
//! picture and events as digests (`{name}.frames`) and a few frames as PNGs.
//! `PINEY_SHOTS=DIR cargo test --release -p piney-game field_shots --
//! --ignored --nocapture`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use piney_data::area::AreaTables;
use piney_data::volume::Volume;
use piney_input::Raw;
use piney_world::area::{Scene, WorldMan, kind};
use piney_world::field::world::Place;

use super::*;

const VOLUMES: [&str; 4] = ["infection", "mutation", "outbreak", "quarantine"];

/// FNV-1a over `bytes`, a digest that is the same on every run.
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// A run's record: each frame rendered, its picture's and events' digests
/// written to `{dir}/{name}.frames`, the frames asked for as PNGs.
struct Record {
    gs: piney_gs::Gs,
    dir: String,
    name: String,
    lines: String,
    n: u32,
}

impl Record {
    fn new(s: &Session, dir: &str, name: &str) -> Record {
        let gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
        Record { gs, dir: dir.into(), name: name.into(), lines: String::new(), n: 0 }
    }

    /// One frame of `s` with `raw` held; written as a PNG with `png`.
    fn frame(&mut self, s: &mut Session, raw: &Raw, png: bool) {
        let mut pad = Pad::default();
        pad.read(raw);
        let frame = s.step(&pad);
        let events = s.take_events();
        self.gs.set_overlay(Mode::archive(s));
        self.gs.render(&frame);
        let pixels = self.gs.read_back();
        let ev = digest(format!("{events:?}").as_bytes());
        writeln!(self.lines, "{:05} {:016x} {:016x} {}", self.n, digest(&pixels), ev, events.len()).unwrap();
        if png {
            let (w, h) = self.gs.target_size();
            let path = format!("{}/{}-{:05}.png", self.dir, self.name, self.n);
            std::fs::write(&path, piney_gs::png::encode(w, h, &pixels)).unwrap();
        }
        self.n += 1;
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        std::fs::write(format!("{}/{}.frames", self.dir, self.name), &self.lines).unwrap();
        println!("{}/{}: {} frames", self.dir, self.name, self.n);
    }
}

/// The left stick as `shoot` (`map_shots`) moves it: still, ahead, then
/// turning while walking.
fn stick(n: u32, walk: u32) -> Raw {
    let (lx, ly) = match n {
        n if n < walk => (128, 128),
        n if n < walk + 60 => (128, 0),
        _ => (255, 64),
    };
    Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() }
}

/// The area world, while the session is in one.
fn world_mut(s: &mut Session) -> Option<&mut piney_world::field::world::FieldWorld> {
    match &mut s.stage {
        Stage::Area(a) => Some(a.world_mut()),
        _ => None,
    }
}

/// `frames` frames of `s`, the tasks put to sleep for frames 150-169 (the
/// pictures drawn again as they stood), PNGs at `pngs`.
fn run(s: &mut Session, rec: &mut Record, frames: u32, pngs: &[u32]) {
    for n in 0..frames {
        if let Some(w) = world_mut(s)
            && matches!(n, 150 | 170)
        {
            w.set_asleep(n == 150);
        }
        rec.frame(s, &stick(n, 60), pngs.contains(&n));
    }
}

/// A disc's archive and tables, None without the disc.
fn disc(name: &str) -> Option<(PathBuf, Arc<Archive>, &'static AreaTables, Volume)> {
    let iso = crate::check_image(name);
    if !iso.exists() {
        eprintln!("{name}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let t = crate::area::area_tables(&mut d).unwrap();
    Some((iso, archive, t, d.volume().unwrap()))
}

/// A new game on `iso` logged in on `server` (its Root Town last), the
/// crisis byte `crisis`, put in the area of `words` (`area`: a field, or
/// its first dungeon).
fn in_area(iso: &Path, archive: &Arc<Archive>, server: i32, crisis: bool, words: [i32; 3], area: i32) -> Session {
    let mut d = Iso::open(iso).unwrap();
    let t = crate::area::area_tables(&mut d).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    let save = &mut state.save;
    save.set_u8(offset::LAST_TOWN, server as u8);
    save.set_u8(offset::CRISIS, u8::from(crisis));
    let mut scene = Scene::log_in(save);
    let (wm, _) = WorldMan::set_generate_code(t, words, scene.server, save).expect("the words");
    scene.change_scene(1, scene.town, 0, -1, -1, -1, save);
    scene.change_area(area, 0, save);
    Session::in_world(iso.to_path_buf(), archive.clone(), None, state, None, scene, Some(wm)).unwrap()
}

/// Each word triple in the tables' order (the first word fastest) whose
/// random area on `server` (no story area) `keep` takes.
fn words_where(
    t: &AreaTables,
    server: i32,
    mut keep: impl FnMut(&piney_data::area::AreaCode) -> bool,
) -> Option<[i32; 3]> {
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    for &z in &c {
        for &y in &b {
            for &x in &a {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, server, false) else { continue };
                if code.event == 0 && keep(&code) {
                    return Some([x, y, z]);
                }
            }
        }
    }
    None
}

/// Each field type (4 has no field) and background row of `disc`, plain
/// and, for the types with a hacked background, hacked (the crisis: data
/// bugs' turning sky and mountains); a storm (weather 3) long enough for
/// its first flash. Server 0.
fn fields_of(disc_name: &str, dir: &str) {
    let Some((iso, archive, t, volume)) = disc(disc_name) else { return };
    let backgrounds = &piney_data::field::tables_of(volume).backgrounds;
    for crisis in [false, true] {
        for ty in (0..=10).filter(|&ty| ty != 4) {
            if crisis && !backgrounds.has_hacked(ty as u32) {
                continue;
            }
            for bg in 0..8 {
                let Some(words) = words_where(t, 0, |c| c.attrs.field_type == ty && c.attrs.weather == bg) else {
                    continue;
                };
                let storm = piney_world::field::ambient::get_weather(ty as u32, bg as u32) == 3;
                let mut s = in_area(&iso, &archive, 0, crisis, words, kind::FIELD);
                let name = format!("{disc_name}-{}-t{ty:02}-b{bg}", if crisis { "hack" } else { "plain" });
                let mut rec = Record::new(&s, dir, &name);
                run(&mut s, &mut rec, if storm { 800 } else { 300 }, &[30, 120, 160, 250]);
            }
        }
    }
}

/// TOBJ (the first field of each server whose layout leaves `fieldrand`
/// at 86 or more out of 100: `WORLD::Generate`'s roll) on `disc`.
fn tobj_fields_of(disc_name: &str, dir: &str) {
    let Some((iso, archive, t, volume)) = disc(disc_name) else { return };
    let tables = piney_data::field::tables_of(volume);
    for server in 0..5 {
        let Some(words) = words_where(t, server, |c| {
            let wm = WorldMan::from_code(c, t, false, false);
            if wm.field_type == 4 || piney_world::field::ambient::get_weather(wm.field_type, wm.weather) >= 2 {
                return false;
            }
            piney_data::field::generate(tables, &wm.field_params()).is_ok_and(|f| {
                let mut rng = f.rng;
                rng.below(100) >= 86
            })
        }) else {
            continue;
        };
        let mut s = in_area(&iso, &archive, server, false, words, kind::FIELD);
        let mut rec = Record::new(&s, dir, &format!("{disc_name}-tobj-s{server}"));
        run(&mut s, &mut rec, 300, &[30, 120, 250]);
    }
}

/// The dungeon of `s` once it plays.
fn dungeon(s: &Session) -> Option<&piney_world::dungeon::DungeonArea> {
    match &s.stage {
        Stage::Area(a) if matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 2) => {
            match a.world().place() {
                Place::Dungeon(d) => Some(d),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `s`'s dungeon walked: its first room, then each of `rooms` (by
/// `RoomSelect`) for 150 frames of play, every frame recorded, the 30th and
/// 120th of each room as PNGs.
fn rooms_of(s: &mut Session, rec: &mut Record, rooms: impl Fn(&piney_world::dungeon::DungeonArea) -> Vec<(i32, i32)>) {
    let mut n = 0;
    while dungeon(s).is_none() && n < 900 {
        rec.frame(s, &stick(0, 1), false);
        n += 1;
    }
    let Some(d) = dungeon(s) else { panic!("{}: not in the dungeon", rec.name) };
    let mut todo = vec![(-1, -1)];
    todo.extend(rooms(d));
    for (k, (f, b)) in todo.into_iter().enumerate() {
        if f >= 0 {
            assert!(world_mut(s).is_some_and(|w| w.room_select(f, b)), "{}: room {f} {b}", rec.name);
            let mut wait = 0;
            while !(dungeon(s).is_some_and(|_| {
                let sc = world_mut_ref(s).scene();
                (sc.floor, sc.block) == (f, b)
            })) && wait < 600
            {
                rec.frame(s, &stick(0, 1), false);
                wait += 1;
            }
        }
        for n in 0..150 {
            if let Some(w) = world_mut(s)
                && k == 0
                && matches!(n, 90 | 110)
            {
                w.set_asleep(n == 90);
            }
            rec.frame(s, &stick(n, 20), matches!(n, 30 | 120));
        }
    }
}

/// The world of a session in an area.
fn world_mut_ref(s: &Session) -> &piney_world::field::world::FieldWorld {
    match &s.stage {
        Stage::Area(a) => a.world(),
        _ => panic!("not in an area"),
    }
}

/// Each dungeon type of `disc` (the first random area making it on any
/// server, its first dungeon), its first room and up to four dressed
/// rooms; the lakes (types 8 and 9) also by night and hacked.
fn dungeons_of(disc_name: &str, dir: &str) {
    let Some((iso, archive, t, _)) = disc(disc_name) else { return };
    let mut runs: Vec<(String, i32, bool, [i32; 3])> = Vec::new();
    for ty in 0..10u8 {
        let lake = ty >= 8;
        let found = (0..5).find_map(|server| {
            words_where(t, server, |c| (c.attrs.field_type == 4) == lake && c.dungeon_type(t, false)[0] == ty)
                .map(|w| (server, w))
        });
        let Some((server, words)) = found else { continue };
        runs.push((format!("{disc_name}-d{ty}"), server, false, words));
        if lake {
            let night = (0..5).find_map(|server| {
                words_where(t, server, |c| {
                    c.attrs.field_type == 4 && c.attrs.weather == 2 && c.dungeon_type(t, false)[0] == ty
                })
                .map(|w| (server, w))
            });
            if let Some((server, words)) = night {
                runs.push((format!("{disc_name}-d{ty}-night"), server, false, words));
            }
            let flag = piney_data::area::AreaTables::dungeon_rule(t).save_flag_of_crisis(true);
            let hacked = (0..5).find_map(|server| {
                words_where(t, server, |c| c.attrs.field_type == 4 && c.dungeon_type(t, flag)[0] == ty)
                    .map(|w| (server, w))
            });
            if let Some((server, words)) = hacked {
                runs.push((format!("{disc_name}-d{ty}-hack"), server, true, words));
            }
        }
    }
    let made: Vec<u8> =
        (0..10).filter(|ty| runs.iter().any(|r| r.0.starts_with(&format!("{disc_name}-d{ty}")))).collect();
    for (name, server, crisis, words) in runs {
        let mut s = in_area(&iso, &archive, server, crisis, words, kind::DUNGEON);
        let mut rec = Record::new(&s, dir, &name);
        rooms_of(&mut s, &mut rec, |d| {
            super::dressing::rooms_with_pieces(d).into_iter().map(|(f, b)| (f as i32, b as i32)).collect()
        });
    }
    // The types no random area makes (`SetDungeonTypeFromField`'s story
    // rows, hacked): the first story area whose dungeon 0 or 1 is of it,
    // plain or in the crisis.
    let rule = &piney_data::dungeon::tables_of(Iso::open(&iso).unwrap().volume().unwrap()).types;
    for ty in (0..10u8).filter(|ty| !made.contains(ty)) {
        let found = [false, true].into_iter().find_map(|crisis| {
            (1..=200).find_map(|n| {
                let wm = crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), n, crisis).ok()?;
                let types = rule.types(wm.field_type, n, wm.hack as i32, false);
                (0..2).find(|&code| types[code] == ty).map(|code| (n, code as i32, crisis))
            })
        });
        let Some((n, code, crisis)) = found else { continue };
        let mut s = in_story_dungeon(&iso, &archive, n, code, crisis);
        let mut rec = Record::new(&s, dir, &format!("{disc_name}-d{ty}-area{n}"));
        rooms_of(&mut s, &mut rec, |d| {
            super::dressing::rooms_with_pieces(d).into_iter().map(|(f, b)| (f as i32, b as i32)).collect()
        });
    }
    // Area 101's dungeon: its event room of type 32, the tree and leaves.
    if crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), 101, false).is_err() {
        return;
    }
    let mut s = in_story_dungeon(&iso, &archive, 101, 0, false);
    let mut rec = Record::new(&s, dir, &format!("{disc_name}-area101"));
    rooms_of(&mut s, &mut rec, |d| {
        let rooms = d.edit.map_or(&[][..], |e| e.rooms);
        rooms
            .iter()
            .filter(|r| r.room_type == piney_world::dungeon::leaves::LEAF_ROOM)
            .map(|r| (r.floor, r.index))
            .collect()
    });
}

/// A new game on `iso`, its crisis byte `crisis`, put on the first floor of
/// story area `n`'s dungeon `code`.
fn in_story_dungeon(iso: &Path, archive: &Arc<Archive>, n: i32, code: i32, crisis: bool) -> Session {
    let mut d = Iso::open(iso).unwrap();
    let wm = crate::area::story_world_man(&mut d, n, crisis).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::CRISIS, u8::from(crisis));
    let mut scene = Scene::log_in(&mut state.save);
    scene.change_scene(2, scene.town, n, code, 0, 0, &mut state.save);
    let mut s = Session::bare(iso.to_path_buf(), archive.clone(), false, None, false);
    s.resume_in(state, None, Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs: None }))).unwrap();
    s
}

fn shots_dir() -> String {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/field_shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[ignore]
fn field_shots() {
    let dir = shots_dir();
    for disc in VOLUMES {
        fields_of(disc, &dir);
        tobj_fields_of(disc, &dir);
    }
}

#[test]
#[ignore]
fn dungeon_shots() {
    let dir = shots_dir();
    for disc in VOLUMES {
        dungeons_of(disc, &dir);
    }
}
