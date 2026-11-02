//! Golden shots of the story maps and the fields by type, for clean-ups
//! that must not change a pixel: every story map's field on each volume
//! that makes it, and the first random area of each field type and
//! background row. `PINEY_SHOTS=DIR cargo test --release -p piney-game
//! story_map_shots -- --ignored --nocapture` (and `field_type_shots`).

use piney_input::Raw;
use piney_world::area::{Scene, WorldMan, kind};

use super::*;

const VOLUMES: [&str; 4] = ["infection", "mutation", "outbreak", "quarantine"];

/// The fields `GO(1)` makes a story map for.
const STORY_FIELDS: [i32; 19] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 43, 66, 67, 91];

/// A new game's session in story field `n` of `disc`, None where the disc
/// makes no map for it.
fn in_story_map(disc: &str, n: i32) -> Option<Session> {
    let iso = crate::check_image(disc);
    let mut d = Iso::open(&iso).ok()?;
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").ok()?).ok()?);
    let mut state = crate::world::new_game_state(&mut d).ok()?;
    let mut scene = Scene::log_in(&mut state.save);
    let wm = match crate::area::story_world_man(&mut d, n, false) {
        Ok(wm) => wm,
        Err(_) => {
            let base = crate::area::story_world_man(&mut d, 15, false).ok()?;
            crate::area::ev_area_number(&iso, n, &base, &state.save).ok()?
        }
    };
    scene.change_scene(1, scene.town, n, -1, -1, -1, &mut state.save);
    Session::in_world(iso, archive, None, state, None, scene, Some(wm)).ok()
}

/// `frames` frames of `s`, every one rendered, the left stick turning
/// after `walk`; the frames of play in `at` written as `{dir}/{name}-N.png`.
fn shoot(s: &mut Session, dir: &str, name: &str, frames: u32, walk: u32, at: &[u32]) {
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    for n in 0..frames {
        let (lx, ly) = match n {
            n if n < walk => (128, 128),
            n if n < walk + 60 => (128, 0),
            _ => (255, 64),
        };
        pad.read(&Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() });
        let frame = s.step(&pad);
        s.take_events();
        gs.set_overlay(Mode::archive(s));
        gs.render(&frame);
        let play = match &s.stage {
            Stage::Area(a) => a.world().phase(),
            _ => continue,
        };
        if let piney_world::Phase::Play(p) = play
            && at.contains(&p)
        {
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{name}-{p:03}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(s));
        }
    }
}

#[test]
#[ignore]
fn story_map_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/story_maps".into());
    std::fs::create_dir_all(&dir).unwrap();
    for disc in VOLUMES {
        for n in STORY_FIELDS {
            let Some(mut s) = in_story_map(disc, n) else { continue };
            shoot(&mut s, &dir, &format!("{disc}-{n:02}"), 420, 90, &[10, 60, 150, 280, 400]);
        }
    }
}

/// The first words of Δ making a random field of type `ty` with
/// background row `bg`.
fn field_of(t: &piney_data::area::AreaTables, ty: i32, bg: i32) -> Option<[i32; 3]> {
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    for &z in &c {
        for &y in &b {
            for &x in &a {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, 0, false) else { continue };
                if code.event == 0 && code.attrs.field_type == ty && code.attrs.weather == bg {
                    return Some([x, y, z]);
                }
            }
        }
    }
    None
}

#[test]
#[ignore]
fn field_type_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/field_types".into());
    std::fs::create_dir_all(&dir).unwrap();
    let iso = crate::check_image("infection");
    let Ok(mut disc) = Iso::open(&iso) else { return };
    let t = crate::area::area_tables(&mut disc).unwrap();
    // Type 4 makes no field (its areas are dungeons).
    for ty in (0..=10).filter(|&ty| ty != 4) {
        for bg in 0..8 {
            let Some(words) = field_of(t, ty, bg) else { continue };
            let Some(mut s) = story_session_with(25, |start| {
                let save = &mut start.state.save;
                save.set_u8(offset::LAST_TOWN, 0);
                let mut scene = Scene::log_in(save);
                let (wm, _) = WorldMan::set_generate_code(t, words, scene.server, save).expect("the words");
                scene.change_scene(1, 0, 0, -1, -1, -1, save);
                scene.change_area(kind::FIELD, 0, save);
                start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
                    scene,
                    world_man: Some(wm),
                    spcs: Some(crate::start::party(25)),
                }));
            }) else {
                return;
            };
            shoot(&mut s, &dir, &format!("t{ty:02}-b{bg}"), 300, 60, &[30, 120, 250]);
        }
    }
}
