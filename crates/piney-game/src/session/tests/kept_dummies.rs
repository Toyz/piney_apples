//! The dummy the Event NPC turns (rows 177 and 179, from Mutation on) lasts
//! as long as the town's file stays loaded: `ccFileExistCheck` (INF main
//! 0x001640f0) destroys a file's stream only when the next scene's list
//! lacks it, and `ccFileListLoad` reuses a kept one. A scene change to the
//! same town keeps the turned marker; going to another town and back reads
//! the file again, and the marker is the disc's.

use std::path::PathBuf;

use piney_data::volume::Volume;

use super::*;

const ITEM_COMPLETE: usize = offset::EVENT_STATUS + 52;
/// Each town whose Event NPC turns a marker: the town, the row, the marker
/// and the heading written (Carmina Gadelica's 177, the fifth town's 179).
const TURNED: [(u8, i32, &str, i16); 2] = [(2, 177, "DMY_marker_ev01", 24576), (4, 179, "DMY_marker_ev02", -24576)];

/// A new game at town `town`'s gate on Mutation with ITEM COMPLETE's status
/// set.
fn in_town(town: u8) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !iso.exists() {
        eprintln!("mutation.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, town);
    state.save.set_u8(ITEM_COMPLETE, 1);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    Some(Session::in_world(iso, archive, None, state, None, scene, None).unwrap())
}

/// Steps until the town `town` plays with its entries placed.
fn play_in(s: &mut Session, town: i32) {
    let pad = Pad::default();
    for _ in 0..3000 {
        if let Stage::World(w) = &s.stage
            && w.world().town().base.no == town
            && matches!(w.world().phase(), piney_world::Phase::Play(n) if n >= 30)
        {
            return;
        }
        s.step(&pad);
        s.take_events();
    }
    panic!("town {town} never played: {}", Mode::title(s));
}

fn world(s: &Session) -> &piney_world::World {
    match &s.stage {
        Stage::World(w) => w.world(),
        _ => panic!("left the town: {}", Mode::title(s)),
    }
}

/// The marker's heading as an event placing someone there reads it.
fn heading(s: &Session, marker: &str) -> u32 {
    let n = piney_data::tables::world::of(Volume::Mut).markers().iter().position(|&m| m == marker).unwrap();
    world(s).marker_dummy(n as i16).unwrap().1[2]
}

/// The event scripts' `scene` to `town`: the fade out, the new scene's set-up,
/// its play.
fn to_town(s: &mut Session, town: i16) {
    s.go(Pending::Story(crate::field_host::SceneChange { scene: [0, town, -1, -1, -1, -1], area: None }));
    let pad = Pad::default();
    for _ in 0..600 {
        s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage
            && !matches!(w.world().phase(), piney_world::Phase::Play(_))
        {
            break;
        }
    }
    play_in(s, i32::from(town));
}

#[test]
fn the_event_npcs_turned_marker_lasts_while_the_towns_file_does() {
    for (town, row, marker, deg) in TURNED {
        let Some(mut s) = in_town(town) else { return };
        play_in(&mut s, i32::from(town));
        let turned = piney_world::ee::deg2rad(deg);
        assert!(world(&s).merchants().iter().any(|m| m.id == row), "the Event NPC {row} stands in town {town}");
        assert_eq!(heading(&s, marker), turned, "row {row} turns {marker}");
        // Sent away (its status cleared), then the same town set up again:
        // the file is kept, no Event NPC writes the dummy, and it stays
        // turned.
        match &mut s.stage {
            Stage::World(w) => w.world_mut().state_mut().save.set_u8(ITEM_COMPLETE, 0),
            _ => unreachable!(),
        }
        to_town(&mut s, i16::from(town));
        assert!(!world(&s).merchants().iter().any(|m| m.id == row), "no Event NPC without the status");
        assert_eq!(heading(&s, marker), turned, "town {town}'s file kept keeps the turned dummy");
        // Mac Anu, then back: the town's file is read again.
        to_town(&mut s, 0);
        to_town(&mut s, i16::from(town));
        assert_ne!(heading(&s, marker), turned, "town {town} read again from the disc: the dummy is the disc's");
    }
}
