//! The dungeon generator with the dummy counts read from Infection's disc,
//! against numbers `tools/dungeon.py` gives (which `tools/test_dungeon.py`
//! checks against the game). Skipped when the disc image is not extracted;
//! set PINEY_ISO to point at it elsewhere. `tools/test_dungeon_rs.py`
//! compares many more cases field by field.

use std::path::PathBuf;

use piney_data::archive::Archive;
use piney_data::dungeon::{self, Dummies, Gim, INF, Params};
use piney_data::iso::Iso;

fn data_bin() -> Option<Archive> {
    let path = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    if !path.exists() {
        return None;
    }
    let data = Iso::open(path).unwrap().read_path("DATA/DATA.BIN").unwrap();
    Some(Archive::new(data).unwrap())
}

#[test]
fn random_dungeons_from_the_disc() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };

    // Type 1 (sd2), three floors.
    let d = Dummies::load(&arc, &INF, 1, 0).unwrap();
    assert_eq!(d.by_anime.len(), 103);
    assert_eq!(d.get("ANM_sd2m1n01")[..2], [2, 1]);
    let p = Params {
        seed: 12345,
        dtype: 1,
        level_max: 3,
        room_max: 7,
        server: 0,
        volume: 1,
        word_a: 0,
        field_type: 0,
        code: 0,
    };
    let g = dungeon::generate(&INF, &p, &d).unwrap();
    assert_eq!((g.rng.seed, g.rng.count), (4_083_085_511, 2834));
    assert_eq!(g.statue.map(|s| (s.floor, s.room)), Some((2, 4)));
    assert_eq!(g.gimmicks.len(), 28);
    let shape: Vec<_> = g.floors.iter().map(|f| (f.rooms.len(), f.down, f.retries)).collect();
    assert_eq!(shape, [(8, Some(6), 0), (9, Some(7), 4), (7, None, 0)]);
    let room = g.floors[0].rooms[1].model.as_ref().unwrap();
    assert_eq!((room.row, room.pick, room.minimap.code()), (4, 1, 5));
    let boxes = room.dummies.iter().filter(|r| r.gim == Gim::ItemBox && r.keep).count();
    assert_eq!(boxes, 2);

    // A lake dungeon straight from field type 4.
    let d = Dummies::load(&arc, &INF, 9, 0).unwrap();
    let p = Params { seed: 0x0135_7bdf, dtype: 9, field_type: 4, ..p };
    let g = dungeon::generate(&INF, &p, &d).unwrap();
    assert_eq!((g.rng.seed, g.rng.count, g.gimmicks.len()), (4_028_004_216, 761, 7));
    let shape: Vec<_> = g.floors.iter().map(|f| (f.rooms.len(), f.down, f.retries)).collect();
    assert_eq!(shape, [(8, Some(7), 1)]);
    let start = g.floors[0].start[0].unwrap();
    assert!(start.pos.is_some() && start.facing.is_some());
}

#[test]
fn first_story_dungeon() {
    // Needs no disc: the layout is all table.
    let s = dungeon::story(&INF, 14, 0).unwrap();
    assert_eq!(s.edit.name, "D0001_room");
    assert_eq!(s.floors.iter().map(|f| f.rooms.len()).collect::<Vec<_>>(), [5, 5]);
    assert_eq!((s.floors[0].up, s.floors[0].down), (0, Some(4)));
}
