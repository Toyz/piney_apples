//! Issue #57: Chosen Hopeless Nothingness (field 27) on Mutation. Its
//! dungeon is where event 101 takes Kite and BlackRose back ("There are no
//! enemies..."), and `WORLD_MAN::EntryGimmick` places no portals there:
//! it skips `DUNGEON::SetMagicCircle` when `volumeNum` is 2 and
//! `game.field` is 27. Its boxes and idols are placed as anywhere.

use piney_battle::entry::Kind;
use piney_data::volume::Volume;
use piney_world::area::Scene;

use super::area15::{hold_frame, playing, wait};
use super::*;

/// A new game put on the first floor of field 27's dungeon on `volume`'s
/// disc; None without it.
fn in_field_27_dungeon(volume: Volume) -> Option<Session> {
    let file = piney_data::pack::disc_name(volume).trim_end_matches(".disc");
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{file}/{file}.iso"));
    if !iso.exists() {
        eprintln!("{file}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    let mut scene = Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 27, false).unwrap();
    scene.change_scene(2, scene.town, 27, 0, 0, 0, &mut state.save);
    let mut s = Session::bare(iso, archive, false, None, false);
    s.resume_in(state, None, Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs: None }))).unwrap();
    for _ in 0..600 {
        if playing(&s).is_some() {
            return Some(s);
        }
        wait(&mut s, 1);
    }
    panic!("not playing in field 27's dungeon: {}", Mode::title(&s));
}

/// The dungeon's portals and its other gimmicks (the boxes and idols) on
/// the entry control's list: every room's, placed once for the dungeon.
fn placed(s: &Session) -> (usize, usize) {
    let w = playing(s).expect("playing");
    assert_eq!((w.scene().area, w.scene().field), (2, 27));
    let c = w.combat();
    (c.ctrl.list(Kind::Circle).len(), c.ctrl.list(Kind::Gimmick).len())
}

/// Mutation's field 27 dungeon has no portals but keeps its boxes; the
/// same dungeon on Infection has its portals.
#[test]
fn mutations_hopeless_nothingness_has_no_portals() {
    let Some(s) = in_field_27_dungeon(Volume::Mut) else { return };
    let (circles, gimmicks) = placed(&s);
    assert_eq!(circles, 0, "Mutation's field 27 dungeon placed {circles} portals");
    assert!(gimmicks > 0, "its boxes and idols are placed as anywhere");
    let Some(s) = in_field_27_dungeon(Volume::Inf) else { return };
    let (circles, _) = placed(&s);
    assert!(circles > 0, "Infection's field 27 dungeon keeps its portals");
}

/// Mutation's dungeon in `ROOM` (where Infection's rules put a portal),
/// Kite walked in for 40 frames, to `$PINEY_SHOTS` (/mnt/data/claude/scratch/i57) as
/// `mut-field27-dungeon.png`, with the rooms of the portals placed.
#[test]
#[ignore]
fn hopeless_nothingness_shot() {
    const ROOM: (i32, i32) = (0, 2);
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i57".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = in_field_27_dungeon(Volume::Mut) else { return };
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    let c = a.world().combat();
    let rooms: Vec<(i32, i32)> = c
        .ctrl
        .list(Kind::Circle)
        .into_iter()
        .filter_map(|i| c.ctrl.entry_obj(i))
        .map(|o| (o.ent.floor, o.ent.block))
        .collect();
    println!("portals' rooms {rooms:?}");
    assert!(a.world_mut().room_select(ROOM.0, ROOM.1));
    for _ in 0..600 {
        if playing(&s).is_some_and(|w| (w.scene().floor, w.scene().block) == ROOM) {
            break;
        }
        wait(&mut s, 1);
    }
    wait(&mut s, 20);
    let frame = hold_frame(&mut s, 128, 0, 40);
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    g.set_overlay(Mode::archive(&s));
    g.render(&frame);
    let (w, h) = g.target_size();
    let path = format!("{dir}/mut-field27-dungeon.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    println!("{path}: portals and gimmicks {:?}", placed(&s));
}
