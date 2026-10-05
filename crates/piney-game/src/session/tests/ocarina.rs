//! Issue #51: out of a dungeon by the Sprite Ocarina (PERSONAL, Items,
//! 13/1: `ccUseItemRequest`, menu 86's `WORLD_MAN::GoField`) or by floor
//! 0's stairs up. Below a lake (field type 4, the "forest" areas) both
//! lead back to the lake's room left, where `GO(2)` stands the party at
//! the room's stairs down (`startpos[1][0]`, its `OBJ_0ppp`); from any
//! other dungeon to its field, beside the entrance.

use piney_input::Buttons;
use piney_world::area::kind;
use piney_world::dungeon_area::DungeonArea;
use piney_world::field_world::Place;

use super::ride::{area, raw};
use super::*;

fn still() -> Raw {
    raw(Buttons::NONE, 128)
}

/// The dungeon, once its scene has played `n` frames.
fn dungeon_at(s: &Session, n: u32) -> Option<&DungeonArea> {
    let Stage::Area(a) = &s.stage else { return None };
    let w = a.world();
    let playing = matches!(w.phase(), piney_world::Phase::Play(k) if k >= n);
    match w.place() {
        Place::Dungeon(d) if playing && w.scene().area == kind::DUNGEON => Some(d),
        _ => None,
    }
}

/// Frames with the pad still until `done`, at most `frames`: whether it
/// came.
fn wait(s: &mut Session, frames: u32, done: impl Fn(&Session) -> bool) -> bool {
    let mut pad = Pad::default();
    for _ in 0..frames {
        pad.read(&still());
        s.step(&pad);
        s.take_events();
        if done(s) {
            return true;
        }
    }
    false
}

/// Kite put on floor 0's stairs of `room` and `WORLD_MAN::Enter` asked,
/// as the stairs' ground (attribute 0x80000) asks it.
fn take_stairs(s: &mut Session, room: usize) {
    let Stage::Area(a) = &mut s.stage else { panic!("not in an area") };
    let Place::Dungeon(d) = a.world().place() else { panic!("not in a dungeon") };
    let fl = &d.floors[0];
    let cell = (0..6400usize)
        .find(|&i| {
            let c = fl.map.cells()[i];
            usize::from(c.here) == room && c.next == piney_data::dungeon::NO_ROOM
        })
        .expect("the stairs' cell");
    let at = |k: usize| ((k as f32 * 750.0 + 375.0) / 10.0) as i16;
    a.world_mut().pc_command(piney_event::host::PcCommand::Put { pc: 0, x: at(cell / 80), y: at(cell % 80), z: 0 });
    a.world_mut().combat_mut().enter = true;
}

/// The Sprite Ocarina from PERSONAL's Items, "Return to the field." OK'd:
/// pads until the menu has used it (none left), at most 600 frames.
fn play_ocarina(s: &mut Session) {
    {
        let Stage::Area(a) = &mut s.stage else { panic!("not in an area") };
        let save = &mut a.world_mut().state_mut().save;
        save.set_i16(offset::ITEM_LIST, 1);
        save.set_u8(offset::ITEM_LIST + 2, 13);
        save.set_u8(offset::ITEM_LIST + 3, 1);
    }
    let mut pad = Pad::default();
    for f in 0..600u32 {
        let a = area(s);
        if a.world().state().save.u8(offset::ITEM_LIST + 3) == 0 {
            return;
        }
        let c = &a.ui().ctrl;
        let b = if !f.is_multiple_of(8) {
            Buttons::NONE
        } else {
            // PERSONAL, its second row (Items), the ocarina, OK on OK.
            match (a.ui().menu_type(), c.proccess) {
                (-1, _) => Buttons::TRIANGLE,
                (1 | 2, 1) if c.list().select < 1 => Buttons::DOWN,
                (5, 11) if c.list().select == 1 => Buttons::UP,
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        s.take_events();
    }
    let a = area(s);
    panic!("the ocarina was not played (menu {}, proccess {})", a.ui().menu_type(), a.ui().ctrl.proccess);
}

/// A Δ lake's area (`dressing::lake_areas`) entered, and down its stairs
/// into the second dungeon until that plays: the session and the lake's
/// room left (`lastRoom`).
fn below_a_lake() -> Option<(Session, usize)> {
    let words = *super::dressing::lake_areas(0, &[8]).first()?;
    let mut s = super::dressing::in_random_dungeon(0, words)?;
    s.console("god");
    assert!(wait(&mut s, 600, |s| dungeon_at(s, 30).is_some()), "the lake");
    // Into the room with the stairs down (`WORLD_MAN::RoomSelect`), so
    // that `lastRoom` is it, then down.
    let down = dungeon_at(&s, 0).map(|d| d.floors[0].down).unwrap();
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    assert!(a.world_mut().room_select(0, down as i32));
    assert!(wait(&mut s, 600, |s| dungeon_at(s, 0).is_none()), "the change of room");
    assert!(wait(&mut s, 600, |s| dungeon_at(s, 30).is_some()), "the room");
    take_stairs(&mut s, down);
    assert!(wait(&mut s, 600, |s| dungeon_at(s, 30).is_some_and(|d| d.dtype == 2)), "the second dungeon");
    Some((s, down))
}

/// Back at the lake: its room `last` built, Kite at its stairs down
/// (`OBJ_0ppp`), inside the room.
fn assert_at_the_lake(s: &Session, last: usize) {
    let d = dungeon_at(s, 0).expect("in a dungeon");
    let w = area(s).world();
    assert_eq!((d.dtype, w.scene().dungeon, w.scene().block), (8, 0, last as i32), "not the lake's room left");
    assert_eq!((d.room_at, d.room.is_some()), (Some((0, last)), true), "the room not built");
    let kite = w.player().body.pos;
    let stairs = d.floors[0].start[1];
    assert_eq!(kite[..2], stairs[..2], "Kite not at the stairs down: {:?}", kite.map(f32::from_bits));
    assert_eq!(d.here(kite), Some(last), "Kite outside the room: {:?}", kite.map(f32::from_bits));
}

/// Issue #51: the Sprite Ocarina played below a lake brings the party back
/// up to the lake's room left, standing at its stairs down.
#[test]
fn the_sprite_ocarina_below_a_lake_returns_to_it() {
    let Some((mut s, last)) = below_a_lake() else { return };
    play_ocarina(&mut s);
    assert!(wait(&mut s, 900, |s| dungeon_at(s, 60).is_some_and(|d| d.dtype == 8)), "back at the lake");
    assert_at_the_lake(&s, last);
}

/// The second dungeon's floor-0 stairs up lead to the same place.
#[test]
fn the_stairs_up_below_a_lake_return_to_it() {
    let Some((mut s, last)) = below_a_lake() else { return };
    let up = dungeon_at(&s, 0).map(|d| d.floors[0].up).unwrap();
    take_stairs(&mut s, up);
    assert!(wait(&mut s, 900, |s| dungeon_at(s, 60).is_some_and(|d| d.dtype == 8)), "back at the lake");
    assert_at_the_lake(&s, last);
}

/// Gate Out (menu 10's end, `ChangeArea(0, town)`) from the lake and from
/// below it: Mac Anu, where the party left from.
#[test]
fn gate_out_from_a_lake_reaches_the_town() {
    for below in [false, true] {
        let s = if below {
            below_a_lake().map(|(s, _)| s)
        } else {
            let Some(&words) = super::dressing::lake_areas(0, &[8]).first() else { return };
            super::dressing::in_random_dungeon(0, words).map(|mut s| {
                assert!(wait(&mut s, 600, |s| dungeon_at(s, 30).is_some()), "the lake");
                s
            })
        };
        let Some(mut s) = s else { return };
        let Stage::Area(a) = &mut s.stage else { unreachable!() };
        a.gate_out();
        let in_town = |s: &Session| matches!(&s.stage, Stage::World(w) if w.town_number() == 0);
        assert!(wait(&mut s, 900, in_town), "below {below}: not in Mac Anu: {}", Mode::title(&s));
    }
}

/// The other dungeon types (0-3, from Δ's random areas with a field): the
/// ocarina and floor 0's stairs up both reach the field, the party beside
/// the entrance (`SetCharPosition`: 900 short of it).
#[test]
fn the_ocarina_and_the_stairs_out_of_a_dungeon_reach_its_field() {
    let areas = super::dressing::random_areas(0, &[0, 1, 2, 3]);
    for (words, ocarina) in areas.iter().flat_map(|&w| [(w, true), (w, false)]) {
        let Some(mut s) = super::dressing::in_random_dungeon(0, words) else { return };
        s.console("god");
        assert!(wait(&mut s, 600, |s| dungeon_at(s, 30).is_some()), "the dungeon");
        let (dtype, up) = dungeon_at(&s, 0).map(|d| (d.dtype, d.floors[0].up)).unwrap();
        if ocarina {
            play_ocarina(&mut s);
        } else {
            take_stairs(&mut s, up);
        }
        let in_field = |s: &Session| match &s.stage {
            Stage::Area(a) => {
                let w = a.world();
                matches!(w.place(), Place::Field(_)) && matches!(w.phase(), piney_world::Phase::Play(n) if n > 60)
            }
            _ => false,
        };
        let what = format!("{words:?} type {dtype} by {}", if ocarina { "the ocarina" } else { "the stairs" });
        assert!(wait(&mut s, 900, in_field), "{what}: not in the field");
        let w = area(&s).world();
        let Place::Field(f) = w.place() else { unreachable!() };
        let entrance = f.dungeon_pos().expect("the entrance").map(f32::from_bits);
        let kite = w.player().body.pos.map(f32::from_bits);
        let off = (kite[0] - entrance[0]).hypot(kite[1] - entrance[1]);
        assert!((off - 900.0).abs() < 1.0, "{what}: Kite {kite:?}, the entrance {entrance:?}");
    }
}

/// Shots of the way back up to the lake by the ocarina (ignored; a
/// diagnostic): every 30th frame for 300 frames to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/issue51).
#[test]
#[ignore]
fn ocarina_lake_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/issue51".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some((mut s, _)) = below_a_lake() else { return };
    play_ocarina(&mut s);
    let mut gs: Option<piney_gs::Gs> = None;
    let mut pad = Pad::default();
    for f in 0..300u32 {
        pad.read(&still());
        let frame = s.step(&pad);
        s.take_events();
        if !f.is_multiple_of(30) {
            continue;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(&s));
        g.render(&frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/lake-{f:04}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        println!("{path}");
    }
}
