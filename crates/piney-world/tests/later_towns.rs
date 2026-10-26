//! The characters the later volumes' events place in Fort Ouph (town 3)
//! and Lia Fail (town 4): every `entry` of a block that runs `in_town`
//! there, on each later disc, put through the town as the event VM puts it.

use std::path::Path;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_event::ir::{Cond, Op, Tag};
use piney_input::{Pad, Raw};
use piney_world::{FADE_FRAMES, HOLD_FRAMES, SaveState, World};

/// One `entry type code marker param` and the event that runs it.
#[derive(Clone, Copy, Debug)]
struct Entry {
    event: i32,
    block: usize,
    ty: i16,
    code: i16,
    marker: i16,
    param: i16,
}

/// Every `entry` in a block that runs only in `town`: its own `in_town`
/// (or town `scene`) condition, or the scene tag in force (a tag stays set for the rest of
/// the walk, and the scene tags replace one another).
fn entries_in(events: &[piney_event::ir::Event], town: i16) -> Vec<Entry> {
    let mut out = Vec::new();
    for e in events {
        let mut scene = None;
        for (k, b) in e.script.blocks.iter().enumerate() {
            for t in &b.tags {
                match *t {
                    Tag::InTown { town } | Tag::Scene { area: 0, town, .. } => scene = Some(town),
                    Tag::InField { .. } | Tag::InDungeon { .. } | Tag::Scene { .. } | Tag::InPoint { .. } => {
                        scene = None
                    }
                    _ => {}
                }
            }
            let own = b.conds.iter().find_map(|c| match *c {
                Cond::InTown { town } | Cond::Scene { area: 0, town, .. } => Some(Some(town)),
                Cond::InField { .. } | Cond::InDungeon { .. } | Cond::Scene { .. } | Cond::InPoint { .. } => Some(None),
                _ => None,
            });
            if own.unwrap_or(scene) != Some(town) {
                continue;
            }
            for op in &b.ops {
                if let Op::Entry { ty, code, marker, param } = *op {
                    out.push(Entry { event: i32::from(e.number), block: k, ty, code, marker, param });
                }
            }
        }
    }
    out
}

/// A party character's `entry` at marker 99 or more: from Mutation on
/// it stays where it stands (`ccEntryEventMng`, MUT main 0x001cba88).
fn stays(e: &Entry) -> bool {
    e.ty <= 2 && e.marker >= 99
}

/// A new game's save last in `town`, under the crisis (the town's `d`
/// file) when `crisis`.
fn in_town(volume: piney_data::volume::Volume, town: u8, crisis: bool) -> SaveState {
    let tables = piney_demo::newgame::NewGameTables::of(volume);
    let mut state = SaveState::fresh();
    piney_demo::newgame::new_game(&mut state.save, 0, &tables, piney_demo::newgame::SAVE_VA);
    state.save.set_u8(offset::LAST_TOWN, town);
    state.save.set_u8(offset::CRISIS, u8::from(crisis));
    state
}

/// Each later disc's Fort Ouph and Lia Fail entries, a block's at a time
/// as its event registers them before the town's set-up: the party
/// members (type 2) and NPCs (types 3 and 4) all at markers the town's
/// file has, each built and standing at its marker once the set-up has
/// run (an NPC with `param` -1 where the town's search puts it).
#[test]
fn every_entry_in_fort_ouph_and_lia_fail_is_placed() {
    for disc in ["outbreak", "quarantine", "mutation"] {
        let path = format!("../../work/{disc}/{disc}.iso");
        if !Path::new(&path).exists() {
            eprintln!("skipped: no {path}");
            continue;
        }
        let mut iso = Iso::open(&path).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let events = piney_event::official::events(&mut iso).unwrap();
        let volume = iso.volume().unwrap();
        for town in [3u8, 4] {
            let entries = entries_in(&events, i16::from(town));
            assert!(!entries.is_empty(), "{disc} town {town}");
            let mut blocks: Vec<(i32, usize)> = entries.iter().map(|e| (e.event, e.block)).collect();
            blocks.dedup();
            for (event, block) in blocks {
                let what = format!("{disc} town {town} event {event} block {block}");
                let mine: Vec<Entry> =
                    entries.iter().copied().filter(|e| (e.event, e.block) == (event, block)).collect();
                // Every one of these events runs under the crisis (event
                // 116 sets it, 313-315 clear it): Fort Ouph's town04d, which
                // has ev07-09 besides town04's markers; Lia Fail has one file.
                let world_markers = |world: &World| {
                    mine.iter().all(|e| stays(e) || e.marker < 0 || world.marker_bits(e.marker).is_some())
                };
                let mut world = World::enter(&mut iso, archive.clone(), in_town(volume, town, true)).unwrap();
                assert!(world_markers(&world), "{what}: {mine:?} a marker the file lacks");
                for e in &mine {
                    assert!((2..=4).contains(&e.ty), "{what}: {e:?} a type the towns place");
                    let before = world.char_pos(e.ty, e.code).map(|(p, _)| p);
                    assert!(world.entry(e.ty, e.code, e.marker, e.param), "{what}: {e:?} not taken");
                    if stays(e) && e.code == 0 {
                        assert_eq!(world.char_pos(e.ty, e.code).map(|(p, _)| p), before, "{what}: {e:?} moved");
                    }
                }
                let mut pad = Pad::default();
                for _ in 0..(FADE_FRAMES + HOLD_FRAMES + 1) {
                    pad.read(&Raw::default());
                    world.step(&pad);
                }
                for e in mine.iter().filter(|e| e.marker >= 0 && e.param != -1 && !stays(e)) {
                    let (at, _) = world.marker_bits(e.marker).unwrap();
                    let (pos, _) = world.char_pos(e.ty, e.code).unwrap_or_else(|| panic!("{what}: {e:?} not placed"));
                    assert_eq!(pos, at, "{what}: {e:?}");
                }
            }
            eprintln!("{disc} town {town}: {} entries", entries.len());
        }
    }
}
