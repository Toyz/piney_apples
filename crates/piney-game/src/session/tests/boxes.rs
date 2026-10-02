//! Issue #12: treasure boxes Kite walked through in story area 18's dungeon
//! (Δ, `D0038`'s floors). A box's body is its `ccCharHit` in the list of
//! bodies (`HitEnable`); `ccGimBox::boxMain` (gcmn 0x0045410c) copies the
//! box's `pos` into it each frame, and the list holds the body itself, so
//! the others collide with where the box stands.

use piney_battle::entry::{Kind, Obj};
use piney_world::area::{Scene, kind};
use piney_world::combat::spc::body_id;
use piney_world::field_world::Place;

use super::shrine::walk_to;
use super::*;

/// Event 18's start with the party put in area 18's dungeon, as the gate
/// (Mac Anu's, server 0) and the field's entrance leave it.
fn in_area_18() -> Option<Session> {
    in_story_dungeon(18, 18)
}

/// Event `event`'s start with the party put in story area `area`'s first
/// dungeon, as Mac Anu's gate (server 0) and the field's entrance leave it.
fn in_story_dungeon(event: i32, area: i32) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    story_session_with(event, |start| {
        let save = &mut start.state.save;
        save.set_u8(offset::LAST_TOWN, 0);
        let mut scene = Scene::log_in(save);
        scene.go(piney_data::area::Go::ChangeScene([1, 0, area, -1, -1, -1]), save);
        let wm = crate::area::ev_area_world_man(&iso, area, scene.server, save).unwrap().expect("the area's words");
        scene.change_area(kind::DUNGEON, 0, save);
        start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
            scene,
            world_man: Some(wm),
            spcs: Some(crate::start::party(event)),
        }));
    })
}

/// A box on in this room whose body the list lacks or holds elsewhere:
/// (row, its place, the listed body's place, if any).
type Loose = (i32, [f32; 3], Option<[f32; 3]>);

/// The boxes (rows 0-5) on in this room whose listed body is not where
/// they stand.
fn loose_boxes(s: &Session) -> Vec<Loose> {
    let Stage::Area(a) = &s.stage else { return Vec::new() };
    let w = a.world();
    let Place::Dungeon(d) = w.place() else { return Vec::new() };
    let c = w.combat();
    let mut out = Vec::new();
    for who in c.ctrl.list(Kind::Gimmick) {
        let Some(Obj::Gimmick(o)) = c.ctrl.objs.get(who) else { continue };
        if !o.obj_flag || !(0..=5).contains(&o.gim_id) || o.fade_flag == 2 {
            continue;
        }
        let at = c.scene.chars[who].pos.map(f32::from_bits);
        let id = body_id(who, c.kite);
        let listed = d.hits.chars.iter().find(|b| b.id == id).map(|b| b.pos.map(f32::from_bits));
        let near = listed.is_some_and(|p| (0..3).all(|k| (p[k] - at[k]).abs() < 1.0));
        if !near {
            out.push((o.gim_id, [at[0], at[1], at[2]], listed.map(|p| [p[0], p[1], p[2]])));
        }
    }
    out
}

/// Kite walks area 18's dungeon room by room, floor 0 then floor 1 (B2);
/// in every room, every box standing there has its body in the list at
/// its place.
#[test]
fn area_18_s_boxes_stand_in_the_way() {
    let Some(mut s) = in_area_18() else { return };
    s.console("god");
    let mut pad = Pad::default();
    let mut rooms = Vec::new();
    for _ in 0..600 {
        if let Stage::Area(a) = &s.stage
            && let Place::Dungeon(d) = a.world().place()
        {
            rooms = (0..d.floors.len())
                .flat_map(|f| d.floors[f].order.iter().map(move |&r| (f, r)))
                .filter(|&(f, r)| d.floors[f].rooms.get(r).is_some_and(|sl| sl.made))
                .collect();
            break;
        }
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
    assert!(!rooms.is_empty(), "never in the dungeon");
    let mut seen: Vec<((i32, i32), Loose)> = Vec::new();
    let mut boxes = 0usize;
    for to in rooms {
        walk_to(&mut s, to, 3000, |s, _, _| {
            let Stage::Area(a) = &s.stage else { return };
            let w = a.world();
            if !matches!(w.phase(), piney_world::Phase::Play(n) if n > 2) {
                return;
            }
            let sc = w.scene();
            let c = w.combat();
            boxes = boxes.max(
                c.ctrl
                    .list(Kind::Gimmick)
                    .into_iter()
                    .filter(|&g| c.ctrl.entry_obj(g).is_some_and(|o| o.obj_flag && (0..=5).contains(&o.gim_id)))
                    .count(),
            );
            for l in loose_boxes(s) {
                if !seen.iter().any(|(_, m)| m.0 == l.0 && m.1 == l.1) {
                    seen.push(((sc.floor, sc.block), l));
                }
            }
        });
    }
    assert!(boxes > 0, "no box seen");
    assert!(seen.is_empty(), "boxes without their body where they stand ((floor, room), (row, at, listed)): {seen:?}");
}

/// The report's case: on B2 (floor 1) Kite runs at the story box of room
/// 4 (`GIMMICKDATA` type 0, made at z 250 in another room and set on the
/// ground by `initObject` once he came in). He stops against it: his
/// centre never comes within the box's radius.
#[test]
fn kite_stops_at_a_story_box_on_b2() {
    let Some(s) = in_area_18() else { return };
    kite_stops_at_the_story_box(s, (1, 4));
}

/// Area 31's (event 22's) cure on its last floor: the story box of floor
/// 3, room 11, the First Remedy (`flag` 0x450039).
#[test]
fn kite_stops_at_area_31_s_cure_box() {
    let Some(s) = in_story_dungeon(22, 31) else { return };
    kite_stops_at_the_story_box(s, (3, 11));
}

/// Kite walked to `room` (floor, room), then run at its story box for 400
/// frames: his centre never comes within its radius.
fn kite_stops_at_the_story_box(mut s: Session, (floor, room): (i32, i32)) {
    s.console("god");
    hold_until(&mut s, 600, |s| matches!(&s.stage, Stage::Area(a) if matches!(a.world().place(), Place::Dungeon(_))));
    walk_to(&mut s, (floor as usize, room as usize), 30000, |_, _, _| {});
    let the_box = |s: &Session| {
        let Stage::Area(a) = &s.stage else { return None };
        let c = a.world().combat();
        c.ctrl.list(Kind::Gimmick).into_iter().find_map(|g| {
            let o = c.ctrl.entry_obj(g)?;
            let here =
                o.obj_flag && o.gim_id == 0 && o.ent.floor == floor && o.ent.block == room && o.ent.ent_root == 0;
            here.then(|| (c.scene.chars[g].pos.map(f32::from_bits), f32::from_bits(o.hit.radius)))
        })
    };
    hold_until(&mut s, 300, |s| the_box(s).is_some());
    let (at, radius) = the_box(&s).expect("the room's story box");
    let mut pad = Pad::default();
    let mut nearest = f32::MAX;
    for _ in 0..400 {
        let Stage::Area(a) = &s.stage else { panic!("left the area") };
        let w = a.world();
        let c = w.combat();
        let p = c.scene.chars[c.kite.unwrap()].pos.map(f32::from_bits);
        let (dx, dy) = (at[0] - p[0], at[1] - p[1]);
        nearest = nearest.min((dx * dx + dy * dy).sqrt());
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        pad.read(&stick_toward(cam_z, dx.atan2(-dy)));
        s.step(&pad);
        s.take_events();
    }
    assert!(nearest >= radius, "Kite came within {nearest} of the box's centre (radius {radius})");
}

/// Frames at rest until `done`.
fn hold_until(s: &mut Session, max: u32, done: impl Fn(&Session) -> bool) {
    let mut pad = Pad::default();
    for _ in 0..max {
        if done(s) {
            return;
        }
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
    panic!("not reached in {max} frames: {}", Mode::title(s));
}
