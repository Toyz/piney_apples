//! Event 25 (ML0150, eventTblM125) in story area 23's dungeon (`D0081`):
//! Kite walks from its first room down to floor 2 and through the door of
//! room 3 into room 2, the Aura shrine (`MakeRoom`'s type 25: `se1_3`),
//! where `GotoNextRoom`'s area-23 branch stands the party at the shrine's
//! `OBJ_user_point` (`specialRoom` 0) and the room's event point 1 opens
//! block 16, whose `area_ban 23 0 2 2` bans the shrine before `scene`
//! takes the party back to Dun Loireag.

use piney_data::dungeon::{NO_ROOM, RealMap};
use piney_world::area::{Scene, kind};
use piney_world::dungeon_area::DungeonArea;
use piney_world::field_world::Place;

use super::*;

/// The shrine: floor 2, room 2 of `D0081`.
const SHRINE: (usize, usize) = (2, 2);

/// Event 25's start with the party put in area 23's dungeon, as the gate
/// (server 1, Dun Loireag's) and the field's entrance leave it: `scene 1 1
/// 23` (area 23 made from its words), then `ChangeArea(2, 0)`. The event's
/// blocks before the shrine are marked played, and block 1's `status_set 0
/// 1` (the desktop, the mail read) made: block 16's settings carry block
/// 6's `eventStatus[0] == 1`.
fn in_area_23() -> Option<Session> {
    use piney_event::ScriptSave;
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    story_session_with(25, |start| {
        let save = &mut start.state.save;
        save.update_flags(25, |f| f | 0xffff);
        save.set_byte_at(offset::EVENT_STATUS, 80, 0, 1);
        let mut scene = Scene::log_in(save);
        scene.go(piney_data::area::Go::ChangeScene([1, 1, 23, -1, -1, -1]), save);
        let wm = crate::area::ev_area_world_man(&iso, 23, scene.server, save).unwrap().expect("area 23's words");
        scene.change_area(kind::DUNGEON, 0, save);
        start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
            scene,
            world_man: Some(wm),
            spcs: Some(crate::start::party(25)),
        }));
    })
}

/// The next room from `here` on the way to `to` on the floor `map` lays
/// out: the first step of a breadth-first walk over its door cells.
fn next_room(map: &RealMap, here: usize, to: usize) -> Option<usize> {
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for c in map.cells() {
        if c.door != 0 && c.here != NO_ROOM && c.next != NO_ROOM {
            let e = (usize::from(c.here), usize::from(c.next));
            if !edges.contains(&e) {
                edges.push(e);
            }
        }
    }
    let mut from: Vec<Option<usize>> = vec![None; 16];
    let mut queue = std::collections::VecDeque::from([here]);
    from[here] = Some(here);
    while let Some(r) = queue.pop_front() {
        if r == to {
            let mut step = r;
            while from[step] != Some(here) {
                step = from[step]?;
            }
            return Some(step);
        }
        for &(a, b) in &edges {
            if a == r && from[b].is_none() {
                from[b] = Some(r);
                queue.push_back(b);
            }
        }
    }
    None
}

/// The leg Kite takes in `d` from room `here` to room `goal` (floor,
/// room): the down stairs until its floor, then the doors to it.
fn leg_to(d: &DungeonArea, here: usize, goal: (usize, usize)) -> Option<Leg> {
    let fl = d.floors.get(d.level)?;
    if (d.level, here) == goal {
        return None;
    }
    let to = if d.level < goal.0 { fl.down } else { goal.1 };
    if d.level < goal.0 && here == to {
        return Some(Leg::Stairs);
    }
    next_room(&fl.map, here, to).map(Leg::Door)
}

/// A point of room `room` (a cell's centre) Kite at `p` can walk straight
/// to - no wall between at his knees or waist, his width either side - that
/// sees `q` too, the shortest way through; else the one nearest `q` he can
/// reach.
fn waypoint(d: &DungeonArea, p: [f32; 4], q: [f32; 2], room: usize) -> Option<[f32; 2]> {
    let fl = d.floors.get(d.level)?;
    let at = |x: f32, y: f32, h: f32| [x.to_bits(), y.to_bits(), (p[2] + h).to_bits(), 1f32.to_bits()];
    let clear = |a: [f32; 2], b: [f32; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let n = dx.hypot(dy).max(1.0);
        let (sx, sy) = (-dy / n * 50.0, dx / n * 50.0);
        let mut hits = d.hits.clone();
        [30.0, 95.0].into_iter().all(|h| {
            [-1.0f32, 0.0, 1.0].into_iter().all(|k| {
                let (ox, oy) = (sx * k, sy * k);
                hits.line(at(a[0] + ox, a[1] + oy, h), at(b[0] + ox, b[1] + oy, h), 0x4000_0001, 1).is_none()
            })
        })
    };
    let dist = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let me = [p[0], p[1]];
    let mut open: Vec<[f32; 2]> = Vec::new();
    for x in 0..80 {
        for y in 0..80 {
            let c = fl.map.cells()[x * 80 + y];
            let centre = [x as f32 * 750.0 + 375.0, y as f32 * 750.0 + 375.0];
            if usize::from(c.here) == room && dist(me, centre) > 200.0 && clear(me, centre) {
                open.push(centre);
            }
        }
    }
    let through = open
        .iter()
        .filter(|&&c| clear(c, q))
        .min_by(|&&a, &&b| (dist(me, a) + dist(a, q)).total_cmp(&(dist(me, b) + dist(b, q))));
    through.or_else(|| open.iter().min_by(|&&a, &&b| dist(a, q).total_cmp(&dist(b, q)))).copied()
}

/// [`walk_to`] the shrine.
fn walk_to_shrine(s: &mut Session, frames: u64, each: impl FnMut(&Session, &[Event], &Frame)) {
    walk_to(s, SHRINE, frames, each);
}

/// The walk's state between frames: the leg's stage and goal, where Kite
/// stood at the last check, a waypoint around what stops him, the checks
/// in this room that found him stopped.
#[derive(Default)]
pub(super) struct Walker {
    stage: u8,
    room: Option<(i32, i32)>,
    goal: Option<[f32; 2]>,
    // Where Kite stood 60 frames ago, and a waypoint around what stops him
    // (a wall inside a big room, a box).
    mark: Option<[f32; 2]>,
    via: Option<[f32; 2]>,
    // Checks in this room that found him stopped; past a few, he is put in
    // the leg's doorway (`pc_put`): the rooms whose floors run in rings and
    // ledges, or with a fenced statue in the way, are more than the
    // straight lines here can walk.
    stuck: u32,
    put: Option<[f32; 3]>,
    /// The times he has been put in this room.
    puts: u32,
    /// The story's walk: only the enemies within 600 of Kite fought, and
    /// the magic portals left shut, unless he is stuck in the room (two
    /// checks for the portals; once put, every foe of the room: an event's
    /// entries shut its doors until they fall).
    pub(super) wary: bool,
    /// The story's walk: each foe fought, its HP and the frame it was first
    /// fought; one that has lost none after [`HOPELESS`] frames, or still
    /// stands after three times that, is walked past (a level-49 Squidbod
    /// against a level-35 party), unless Kite is stuck in the room.
    fought: Vec<(usize, i16, u64)>,
    pub(super) hopeless: Vec<usize>,
    /// The foe the walk goes at this frame.
    pub(super) foe: Option<usize>,
    /// The chase of a foe: where Kite stood at the last check, and a
    /// waypoint round what stops him.
    chase_mark: Option<[f32; 2]>,
    chase_via: Option<[f32; 2]>,
}

/// Frames of fighting a foe that loses no HP before the story's walk goes
/// past it.
const HOPELESS: u64 = 1800;

impl Walker {
    /// A foe of this room fought for three times [`HOPELESS`] and still
    /// standing (healers out of a lone Kite's reach): the story pilot
    /// drains any foe whose protect breaks then.
    pub(super) fn held(&self, a: &crate::area::AreaMode, now: u64) -> bool {
        let c = a.world().combat();
        self.fought
            .iter()
            .any(|&(e, _, since)| now > since + 3 * HOPELESS && c.scene.chars.get(e).is_some_and(|ch| ch.hp > 0))
    }

    /// The story's walk ([`Walker::wary`]).
    pub(super) fn wary() -> Walker {
        Walker { wary: true, ..Walker::default() }
    }

    /// One frame of the walk to `to` (floor, room): [`story_player`] while
    /// the event holds Kite, at a live enemy (the attack button in reach)
    /// while there is one, at the room's magic portal (X in reach) while
    /// there is one, else the next leg of [`leg_to`] `to` ([`leg_goal`]'s
    /// points), standing still in `to`. `None` once the session is out of
    /// the dungeon.
    pub(super) fn step(&mut self, s: &Session, to: (usize, usize), f: u64) -> Option<Raw> {
        let raw = {
            let Stage::Area(a) = &s.stage else { return None };
            let w = a.world();
            let sc = w.scene();
            if sc.area != kind::DUNGEON {
                self.foe = None;
                return None;
            }
            if self.room.is_some_and(|r| r != (sc.floor, sc.block)) {
                self.foe = None;
                self.stage = 0;
                self.goal = None;
                self.via = None;
                self.stuck = 0;
                self.puts = 0;
                self.fought.clear();
                self.hopeless.clear();
                self.chase_via = None;
            }
            self.room = Some((sc.floor, sc.block));
            let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                "menu_ban true" => Some(true),
                "menu_ban false" => Some(false),
                _ => None,
            });
            let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
            let c = w.combat();
            match (c.kite, w.place()) {
                (Some(k), Place::Dungeon(d)) if playing && banned != Some(true) && a.ui().menu_type() == -1 => {
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let cam_z = f32::from_bits(w.camera().rot()[2]);
                    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                    let toward = |q: [f32; 2]| stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])));
                    let here = d.here(c.scene.chars[k].pos);
                    // Doors made shut (`SetDoor`) stay so until no foe is
                    // active (`doorFlag` then): with the goal past them,
                    // every foe is fought, however far or hopeless.
                    let in_goal = sc.floor as usize == to.0 && here == Some(to.1);
                    let shut = !d.door.door_flag && !d.doors.is_empty() && !in_goal;
                    // The goal room's foes, however far: its event waits on
                    // them (`no_active`: 203's and 206's Data Bugs and their
                    // drained forms stand off out of reach).
                    let near = |e: usize| {
                        let q = c.scene.chars[e].pos.map(f32::from_bits);
                        !self.wary || self.puts > 0 || shut || in_goal || (q[0] - p[0]).hypot(q[1] - p[1]) < 600.0
                    };
                    let (hopeless, stuck) = (&self.hopeless, self.puts > 0 || shut);
                    let dist = |e: usize| {
                        let q = c.scene.chars[e].pos.map(f32::from_bits);
                        (q[0] - p[0]).hypot(q[1] - p[1])
                    };
                    let mut live = c
                        .enemies()
                        .into_iter()
                        .filter(|&e| c.scene.chars[e].hp > 0 && near(e) && (stuck || !hopeless.contains(&e)));
                    // The story's walk goes at the nearest (a lone Kite's
                    // chases across a room are long).
                    let foe = if self.wary { live.min_by(|&a, &b| dist(a).total_cmp(&dist(b))) } else { live.next() };
                    self.foe = foe;
                    if let Some(e) = foe
                        && self.wary
                    {
                        let hp = c.scene.chars[e].hp;
                        match self.fought.iter().find(|x| x.0 == e) {
                            None => self.fought.push((e, hp, f)),
                            // The goal room's foes are the story's (a Data
                            // Bug the event waits on): never given up.
                            Some(&(_, first, since))
                                if !in_goal
                                    && !self.hopeless.contains(&e)
                                    && f > since + HOPELESS
                                    && (hp >= first || f > since + 3 * HOPELESS) =>
                            {
                                self.hopeless.push(e)
                            }
                            Some(_) => {}
                        }
                    }
                    if let Some(e) = foe {
                        let q = c.scene.chars[e].pos.map(f32::from_bits);
                        if (q[0] - p[0]).hypot(q[1] - p[1]) > 180.0 {
                            // Found stopped on the way (a wall between, event
                            // 206's room 6 of floor 1): round it by a point
                            // of the room that sees the foe.
                            if f.is_multiple_of(60) {
                                if self.chase_mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0) {
                                    self.chase_via = here.and_then(|h| waypoint(d, p, [q[0], q[1]], h));
                                }
                                self.chase_mark = Some([p[0], p[1]]);
                            }
                            if self.chase_via.is_some_and(|v| (v[0] - p[0]).hypot(v[1] - p[1]) < 150.0) {
                                self.chase_via = None;
                            }
                            toward(self.chase_via.unwrap_or([q[0], q[1]]))
                        } else if f.is_multiple_of(8) {
                            Raw { buttons: Buttons::CROSS, ..still }
                        } else {
                            still
                        }
                    } else if let Some(m) = c
                        .ctrl
                        .list(piney_battle::entry::Kind::Circle)
                        .into_iter()
                        // In the goal room too: an event's portal there
                        // holds its `no_active` until its foes fall.
                        .filter(|_| !self.wary || self.stuck >= 2 || self.puts > 0 || in_goal)
                        .find(|&m| {
                            matches!(c.ctrl.objs.get(m), Some(piney_battle::entry::Obj::Circle(o))
                        if (o.obj.ent.floor, o.obj.ent.block) == (sc.floor, sc.block))
                        })
                    {
                        // A magic portal keeps the room's doors shut: up
                        // to it, X to open it; its enemies are the fight's.
                        let q = c.scene.chars[m].pos.map(f32::from_bits);
                        if (q[0] - p[0]).hypot(q[1] - p[1]) > 250.0 {
                            toward([q[0], q[1]])
                        } else if f.is_multiple_of(8) {
                            Raw { buttons: Buttons::CROSS, ..still }
                        } else {
                            still
                        }
                    } else if here != usize::try_from(sc.block).ok() {
                        // The scene has moved on (a door or the stairs taken).
                        self.goal = None;
                        still
                    } else if let Some(l) = here.and_then(|h| leg_to(d, h, to)) {
                        if self.goal.is_none() {
                            self.goal = leg_goal(d, p, l, self.stage);
                        }
                        if f.is_multiple_of(60) {
                            if let (Some(m), Some(q), Some(h)) = (self.mark, self.goal, here)
                                && (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0
                            {
                                self.via = waypoint(d, p, q, h);
                                self.stuck += 1;
                                if self.stuck >= 4 {
                                    self.stuck = 0;
                                    self.puts += 1;
                                    self.via = None;
                                    // The doorway's centre (between the
                                    // leg's points 900 inside and 1500
                                    // beyond it), or the stairs.
                                    let at = match (leg_goal(d, p, l, 1), leg_goal(d, p, l, 2)) {
                                        (Some(a), Some(b)) if l != Leg::Stairs => {
                                            [a[0] + (b[0] - a[0]) * 0.375, a[1] + (b[1] - a[1]) * 0.375]
                                        }
                                        _ => q,
                                    };
                                    let mut hits = d.hits.clone();
                                    let v = |z: f32| [at[0].to_bits(), at[1].to_bits(), z.to_bits(), 1f32.to_bits()];
                                    let mut z = f32::from_bits(hits.land(v(p[2] + 1000.0), 0x2000_0001));
                                    if hits.num == 0 {
                                        z = p[2];
                                    }
                                    self.put = Some([at[0], at[1], z]);
                                    self.stage = 2;
                                    self.goal = leg_goal(d, p, l, 2).or(Some(at));
                                }
                            }
                            self.mark = Some([p[0], p[1]]);
                        }
                        if self.via.is_some_and(|v| (v[0] - p[0]).hypot(v[1] - p[1]) < 150.0) {
                            self.via = None;
                        }
                        match self.goal {
                            Some(_) if self.via.is_some() => self.via.map_or(still, toward),
                            Some(q) if l == Leg::Stairs => toward(q),
                            Some(q) if (q[0] - p[0]).hypot(q[1] - p[1]) > 200.0 => toward(q),
                            Some(_) if self.stage < 2 => {
                                self.stage += 1;
                                self.goal = leg_goal(d, p, l, self.stage);
                                self.goal.map_or(still, toward)
                            }
                            _ => still,
                        }
                    } else {
                        still
                    }
                }
                _ => story_player(s, f),
            }
        };
        Some(raw)
    }

    /// Kite put where [`Walker::step`] found him stuck, if it did.
    pub(super) fn put(&mut self, s: &mut Session) {
        if let (Some(q), Stage::Area(a)) = (self.put.take(), &mut s.stage) {
            let c = |v: f32| (v / 10.0).round() as i16;
            let cmd = piney_event::host::PcCommand::Put { pc: 0, x: c(q[0]), y: c(q[1]), z: c(q[2]) };
            a.world_mut().pc_command(cmd);
            eprintln!("walk_to: Kite put at {q:?}, stuck in room {:?}", self.room);
        }
    }
}

/// The walk ([`Walker`]) to `to` until the session leaves the dungeon or
/// `frames` frames. `each` sees the session and its events after every
/// frame.
pub(super) fn walk_to(
    s: &mut Session,
    to: (usize, usize),
    frames: u64,
    mut each: impl FnMut(&Session, &[Event], &Frame),
) {
    let mut pad = Pad::default();
    let mut walker = Walker::default();
    for f in 0..frames {
        let Some(raw) = walker.step(s, to, f) else { break };
        walker.put(s);
        pad.read(&raw);
        let frame = s.step(&pad);
        let events = s.take_events();
        each(s, &events, &frame);
    }
}

/// `--mode story:25` put in area 23's dungeon: Kite walks down its stairs
/// to floor 2 and through room 3's door into the shrine. `GotoNextRoom`'s
/// area-23 branch builds the shrine from `spccs` (`se1_3`) with the party
/// at its `OBJ_user_point` and `specialRoom` 0 (the story area's sound
/// bank asked for); the room's event point 1 opens event 25's block
/// 16, whose `area_ban` bans the room (23, 0, 2, 2) and whose `scene`
/// leaves for Dun Loireag.
#[test]
fn event_25_opens_in_area_23s_shrine() {
    let Some(mut s) = in_area_23() else { return };
    let mut rooms: Vec<(i32, i32)> = Vec::new();
    // The shrine as GotoNextRoom built it: specialRoom, the room's scene
    // file, WORLD_MAN.position; where Kite stands on the room's scene's
    // first frame of play (the change of scene set up).
    let mut shrine: Option<(i32, Option<String>, [f32; 4])> = None;
    let mut kite_at: Option<[f32; 4]> = None;
    // The sound banks asked for in the shrine.
    let mut banks: Vec<piney_audio::SqContext> = Vec::new();
    walk_to_shrine(&mut s, 20_000, |s, events, _| {
        let Stage::Area(a) = &s.stage else { return };
        let w = a.world();
        let sc = w.scene();
        if sc.area != kind::DUNGEON {
            return;
        }
        if (sc.floor, sc.block) == (2, 2) {
            banks.extend(events.iter().filter_map(|e| match e {
                Event::SqLoad(ctx) => Some(*ctx),
                _ => None,
            }));
        }
        if rooms.last() != Some(&(sc.floor, sc.block)) {
            rooms.push((sc.floor, sc.block));
        }
        if let Place::Dungeon(d) = w.place()
            && d.room_at == Some(SHRINE)
            && shrine.is_none()
        {
            let file = d.spccs.as_ref().map(|f| f.file.stem.clone());
            shrine = Some((d.special_room, file, d.position.map(f32::from_bits)));
        }
        if (sc.floor, sc.block) == (2, 2) && kite_at.is_none() && w.phase() == piney_world::Phase::Play(0) {
            kite_at = Some(w.player().body.pos.map(f32::from_bits));
        }
    });
    eprintln!("rooms {rooms:?}; now {}", Mode::title(&s));
    let (special, file, at) = shrine.unwrap_or_else(|| panic!("never in the shrine: {rooms:?}"));
    assert_eq!(rooms.first(), Some(&(0, 0)), "{rooms:?}");
    assert_eq!(rooms.last(), Some(&(2, 2)), "{rooms:?}");
    assert_eq!(special, 0, "specialRoom in the shrine");
    assert_eq!(file.as_deref(), Some("se1_3"), "the shrine's scene file");
    // GotoNextRoom's area-23 branch: the shrine's OBJ_user_point, where
    // the room's set-up stands Kite.
    let kite = kite_at.expect("Kite in the shrine");
    eprintln!("the arrival {at:?}, Kite {kite:?}");
    assert_eq!((kite[0], kite[1]), (at[0], at[1]), "Kite at the shrine's OBJ_user_point");
    // specialRoom 0: ccSndSQLoad's story bank (sqDataEvent) for area 23.
    assert!(
        banks.iter().any(|b| matches!(b, piney_audio::SqContext::Event { field: 23, .. })),
        "the event bank in the shrine: {banks:?}"
    );
    // On to the town block 16's `scene` asks for.
    let mut pad = Pad::default();
    for _ in 0..2000 {
        if matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(_))) {
            break;
        }
        pad.read(&story_player(&s, 1));
        s.step(&pad);
        s.take_events();
    }
    if !matches!(&s.stage, Stage::World(_)) {
        let title = Mode::title(&s).to_string();
        let calls: Vec<String> = match &s.stage {
            Stage::Area(a) => a.calls().iter().rev().take(30).map(|c| c.1.clone()).collect(),
            _ => Vec::new(),
        };
        let played = take_vm(&mut s).map(|vm| blocks(&vm)).unwrap_or_default();
        panic!("not back in town: {title}; blocks {played:?}; last calls {calls:?}");
    }
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(w.world().town().base.no, 1, "Dun Loireag");
    let bans = piney_world::dungeon_area::bans_of(&w.world().state().save);
    assert!(bans.contains(&[23, 0, 2, 2]), "block 16's area_ban: {bans:?}");
    let vm = take_vm(&mut s).expect("the event task");
    let played = blocks(&vm);
    assert!(played.contains(&(25, 16)), "event 25's block 16: {played:?}");
}

/// The minimap across the floors: on the walk from floor 0 down to floor
/// 2's shrine, every room's play (past its first frames) draws the map but
/// the shrine's, a special room (`specialRoom` 0), where `DrawMap` returns,
/// and the fights (`inBattle`), where `DUNGEON::Draw` does not call it.
#[test]
fn the_minimap_stays_across_floors() {
    let Some(mut s) = in_area_23() else { return };
    // Per room: frames played, frames the map drew something; fight frames
    // and those drawn among them.
    let mut rooms: Vec<((i32, i32), u32, u32)> = Vec::new();
    let mut fights = (0, 0);
    walk_to_shrine(&mut s, 20_000, |s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let w = a.world();
        let sc = w.scene();
        // Not under a change's fade: the scene names the next room while
        // the screen holds the last (and the map task sleeps).
        if sc.area != kind::DUNGEON
            || !matches!(w.phase(), piney_world::Phase::Play(n) if n > 30)
            || s.leaving.is_some()
        {
            return;
        }
        let drawn = u32::from(!a.map_state().last.is_empty());
        if w.combat().battle.in_battle != 0 {
            fights = (fights.0 + 1, fights.1 + drawn);
            return;
        }
        let key = (sc.floor, sc.block);
        if rooms.last().map(|r| r.0) != Some(key) {
            rooms.push((key, 0, 0));
        }
        let r = rooms.last_mut().unwrap();
        r.1 += 1;
        r.2 += drawn;
    });
    eprintln!("rooms (room, frames, map drawn): {rooms:?}; fights {fights:?}");
    assert!(rooms.iter().any(|r| r.0.0 == 2), "never reached floor 2: {rooms:?}");
    assert!(fights.0 > 0 && fights.1 == 0, "the map in {} of {} fight frames", fights.1, fights.0);
    for (room, frames, drawn) in &rooms {
        let shrine = (SHRINE.0 as i32, SHRINE.1 as i32);
        let want = if *room == shrine { 0 } else { *frames };
        assert_eq!(*drawn, want, "room {room:?}: the map drew in {drawn} of {frames} frames");
    }
}

/// Each floor's map is its own: on the walk down to the shrine, in every
/// frame of play (not under a change's fade) the texture holds exactly the
/// current floor's squares - the stairs' `mapHideFlag` 1 repaints it, so
/// the floor above leaves nothing under the new one.
#[test]
fn each_floor_paints_its_own_map() {
    let Some(mut s) = in_area_23() else { return };
    let mut floors = std::collections::BTreeSet::new();
    walk_to_shrine(&mut s, 20_000, |s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let w = a.world();
        let sc = w.scene();
        if sc.area != kind::DUNGEON || !matches!(w.phase(), piney_world::Phase::Play(n) if n > 2) || s.leaving.is_some()
        {
            return;
        }
        let Place::Dungeon(d) = w.place() else { return };
        let Some(m) = d.map.as_ref() else { return };
        let f = d.level;
        let squares = (0..200 * 200).filter(|&k| m.small[f * 200 * 200 + k] != 0).count();
        let texels = m.pixels.iter().filter(|&&p| p > 1).count();
        assert_eq!(texels, squares, "floor {f} room {}: texels {texels}, squares {squares}", sc.block);
        floors.insert(f);
    });
    assert!(floors.len() >= 3, "floors seen {floors:?}");
}

/// Pictures of the minimap on the walk to the shrine: each room's 90th
/// frame of play, named by floor and block. `PINEY_SHOTS=DIR cargo test
/// --release -p piney-game shrine_map_shots -- --ignored --nocapture`
/// (default `/mnt/data/claude/scratch/floors`).
#[test]
#[ignore]
fn shrine_map_shots() {
    let Some(mut s) = in_area_23() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/floors".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    walk_to_shrine(&mut s, 20_000, |s, _, frame| {
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let Stage::Area(a) = &s.stage else { return };
        let w = a.world();
        let sc = w.scene();
        let at = [30, 90, 200, 400].into_iter().find(|&n| w.phase() == piney_world::Phase::Play(n));
        if sc.area == kind::DUNGEON
            && let Some(n) = at
        {
            let (wd, h) = gs.target_size();
            let path = format!("{dir}/f{}-b{}-{n:03}.png", sc.floor, sc.block);
            std::fs::write(&path, piney_gs::png::encode(wd, h, &gs.read_back())).unwrap();
            let (level, info) = match w.place() {
                Place::Dungeon(d) => {
                    let info = d.map.as_ref().map(|m| {
                        let f = d.level;
                        let seen: Vec<usize> =
                            m.floors[f].rooms.iter().enumerate().filter(|r| r.1.seen).map(|r| r.0).collect();
                        let sq = (0..200 * 200).filter(|&k| m.small[f * 200 * 200 + k] != 0).count();
                        let px = m.pixels.iter().filter(|&&p| p > 1).count();
                        format!(
                            "seen {seen:?} squares {sq} texels {px} hide {} here {:?}",
                            m.map_hide,
                            d.here(w.player().body.pos)
                        )
                    });
                    (d.level as i32, info.unwrap_or_default())
                }
                _ => (-1, String::new()),
            };
            println!("{path}: level {level} {info} {}", Mode::title(s));
        }
    });
}

/// Pictures of each dungeon room's first 120 frames, every 6, on the walk
/// to the shrine (the doors shutting on a room with foes and opening as
/// it is cleared). `PINEY_SHOTS=DIR cargo test --release -p piney-game
/// shrine_door_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/doors`).
#[test]
#[ignore]
fn shrine_door_shots() {
    let Some(mut s) = in_area_23() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/doors".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut room = None;
    let mut since = 0u32;
    walk_to_shrine(&mut s, 20_000, |s, _, frame| {
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let Stage::Area(a) = &s.stage else { return };
        let sc = a.world().scene();
        if sc.area != kind::DUNGEON {
            return;
        }
        let key = (sc.floor, sc.block);
        if room != Some(key) {
            room = Some(key);
            since = 0;
        }
        since += 1;
        if since <= 120 && since.is_multiple_of(6) {
            let (wd, h) = gs.target_size();
            let path = format!("{dir}/f{}-b{}-{since:03}.png", sc.floor, sc.block);
            std::fs::write(&path, piney_gs::png::encode(wd, h, &gs.read_back())).unwrap();
        }
    });
}

/// Event 29's point 1 in area 26's dungeon (`WORLD_MAN::SetEventData`'s
/// point rows): floor 2, room 2.
const POINT_1: (usize, usize) = (2, 2);

/// Event `n`'s start with the party put in story area `area`'s dungeon
/// (town `town`'s server): `scene 1 town area`, then the dungeon's room
/// `room` (floor, room; `(0, 0)` its entrance, as the field's leaves it);
/// the blocks in `played` marked played.
fn in_story_dungeon(n: i32, town: i32, area: i32, played: u64, room: (i32, i32)) -> Option<Session> {
    use piney_event::ScriptSave;
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    story_session_with(n, |start| {
        let save = &mut start.state.save;
        save.update_flags(n, |f| f | played);
        let mut scene = Scene::log_in(save);
        scene.go(piney_data::area::Go::ChangeScene([1, town, area, -1, -1, -1]), save);
        let wm = crate::area::ev_area_world_man(&iso, area, scene.server, save).unwrap().expect("the area's words");
        scene.change_scene(2, -2, -2, 0, room.0, room.1, save);
        start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
            scene,
            world_man: Some(wm),
            spcs: Some(crate::start::party(n)),
        }));
    })
}

/// Event 29's start in story area 26's dungeon (Dun Loireag's server),
/// blocks 0-12 (the desktop, the town, the field) played.
fn in_area_26(room: (i32, i32)) -> Option<Session> {
    in_story_dungeon(29, 1, 26, 0x1fff, room)
}

/// Event 29's point 1 (floor, room): [`point`].
fn point_1(s: &mut Session) -> Option<(usize, usize)> {
    point(s, 1)
}

/// The event's point `num` (floor, room), once the dungeon's set-up has
/// handed the event manager its points: the session stepped until then.
fn point(s: &mut Session, num: i32) -> Option<(usize, usize)> {
    let mut pad = Pad::default();
    for f in 0..2000 {
        if let Stage::Area(a) = &s.stage
            && let Some(p) = a.vm().and_then(|vm| vm.mng.points.iter().find(|p| p.num == num))
        {
            return Some((usize::try_from(p.floor).ok()?, usize::try_from(p.block).ok()?));
        }
        pad.read(&story_player(s, f));
        s.step(&pad);
        s.take_events();
    }
    None
}

/// Event 29 in area 26's dungeon, the party put in the room of point 1, where
/// block 14's `entry 3 86` and `entry 4 29` stand Meg and the Administrator
/// ([`piney_world::field_npcs`]), block 15 puts and turns them, has them
/// face each other and Kite, and sends them off (`npc_act 29 4`: the
/// Administrator's `effTransfer` and fade; `npc_act 86 4`: Meg's leaving).
/// Every NPC instruction is carried out, none falls to a host default.
#[test]
fn event_29_meg_and_the_administrator() {
    let Some(mut s) = in_area_26((0, 0)) else { return };
    assert_eq!(point_1(&mut s), Some(POINT_1), "event 29's point 1");
    let goal = POINT_1;
    piney_event::host::take_unported();
    // (code, where) each NPC was first seen; the NPC instructions logged.
    let mut seen: Vec<(i32, [f32; 4])> = Vec::new();
    let mut npc_calls: Vec<String> = Vec::new();
    let mut rooms: Vec<(i32, i32)> = Vec::new();
    walk_to(&mut s, goal, 30_000, |s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let w = a.world();
        let sc = w.scene();
        if rooms.last() != Some(&(sc.floor, sc.block)) {
            rooms.push((sc.floor, sc.block));
        }
        for n in &w.npcs().list {
            let code = n.npc().code();
            if !seen.iter().any(|x| x.0 == code) {
                seen.push((code, n.npc().char().pos.map(f32::from_bits)));
            }
        }
        for (_, c) in a.calls().iter().rev().take(4) {
            if (c.starts_with("npc ") || c.starts_with("trans ")) && !npc_calls.contains(c) {
                npc_calls.push(c.clone());
            }
        }
    });
    eprintln!("rooms {rooms:?}; now {}; NPCs {seen:?}", Mode::title(&s));
    for c in &npc_calls {
        eprintln!("{c}");
    }
    assert!(seen.iter().any(|x| x.0 == 86), "Meg (entry 3 86) never stood: {seen:?}");
    assert!(seen.iter().any(|x| x.0 == 29), "the Administrator (entry 4 29) never stood: {seen:?}");
    assert!(!npc_calls.is_empty(), "no NPC instruction ran");
    let undone: Vec<&String> = npc_calls.iter().filter(|c| c.contains("(not done)")).collect();
    assert!(undone.is_empty(), "NPC instructions not done: {undone:?}");
    let unported = piney_event::host::take_unported();
    assert!(!unported.iter().any(|u| u.contains("npc") || u.contains("trans")), "unported: {unported:?}");
    let vm = take_vm(&mut s).expect("the event task");
    let played = blocks(&vm);
    assert!(played.contains(&(29, 15)), "event 29's block 15: {played:?}");
}

/// Pictures of the walk to event 29's scene with Meg and the
/// Administrator (`PINEY_EVENT=17`: event 17's to its portal), every
/// `PINEY_EVERY` (10) frames from the arrival in the point's room (or
/// `PINEY_ROOM=FLOOR,ROOM`'s; `PINEY_GOAL=FLOOR,ROOM` walks there
/// instead).
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game event_29_shots --
/// --ignored --nocapture` (default `/mnt/data/claude/scratch/e29`).
#[test]
#[ignore]
fn event_29_shots() {
    // PINEY_EVENT=17: event 17's walk to its point 4 instead.
    let e17 = std::env::var("PINEY_EVENT").as_deref() == Ok("17");
    // PINEY_PLAYED=HEX: event 17's blocks marked played (default 0x7f).
    let played = std::env::var("PINEY_PLAYED").ok().and_then(|v| u64::from_str_radix(&v, 16).ok()).unwrap_or(0x7f);
    let s = if e17 { in_story_dungeon(17, 0, 18, played, (0, 0)) } else { in_area_26((0, 0)) };
    let Some(mut s) = s else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/e29".into());
    let every: u64 = std::env::var("PINEY_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    std::fs::create_dir_all(&dir).unwrap();
    let mut goal = point(&mut s, if e17 { 4 } else { 1 }).expect("the event's point");
    // PINEY_GOAL=F,B: walk to that room instead.
    if let Some((f, b)) = std::env::var("PINEY_GOAL").ok().and_then(|v| {
        let (a, b) = v.split_once(',')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    }) {
        goal = (f, b);
    }
    // PINEY_ROOM=F,B: the pictures from that room on (default point 1's).
    let from_room = std::env::var("PINEY_ROOM")
        .ok()
        .and_then(|v| v.split_once(',').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))))
        .unwrap_or((goal.0 as i32, goal.1 as i32));
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut from: Option<u64> = None;
    let mut door_seen = None;
    let mut f = 0u64;
    walk_to(&mut s, goal, 30_000, |s, _, frame| {
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        f += 1;
        let Stage::Area(a) = &s.stage else { return };
        let sc = a.world().scene();
        if from.is_none() && (sc.floor, sc.block) == from_room {
            from = Some(f);
        }
        // The doors' words as they change, from the room on.
        if from.is_some()
            && let Place::Dungeon(d) = a.world().place()
        {
            let now = (d.door, d.doors.len());
            if door_seen != Some(now) {
                println!("frame {}: doors {} {:?}", f - from.unwrap_or(0), now.1, now.0);
                door_seen = Some(now);
            }
        }
        if let Some(t) = from
            && (f - t).is_multiple_of(every)
        {
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{:05}.png", f - t);
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            let c = a.world().combat();
            let kite = c.kite.map(|k| (c.scene.chars[k].hp, c.dead(k)));
            println!("{path}: {} (Kite hp, dead {kite:?})", Mode::title(s));
        }
    });
}

/// Event 17 (the Data Drain's lesson) in story area 18's dungeon: Kite
/// walks to point 4, where block 12 opens a magic portal (`entry_mc` at
/// marker 1) and block 13 walks him part of the way (`pc_walk_pos`) and
/// hands him back; he walks on to the portal, and block 14's `near_marker
/// 1 <= 250` - the distance through the dungeon's bounds - opens. (The
/// walk goes on through the portal's fight, and the event on to block
/// 17's lines and Mac Anu.)
#[test]
fn event_17_walks_kite_to_the_portal() {
    let Some(mut s) = in_story_dungeon(17, 0, 18, 0x7f, (0, 0)) else { return };
    let goal = point(&mut s, 4).expect("event 17's point 4");
    eprintln!("point 4: {goal:?}");
    piney_event::host::take_unported();
    walk_to(&mut s, goal, 30_000, |_, _, _| {});
    let mut near = None;
    let mut pad = Pad::default();
    for f in 0..6000 {
        let raw = match &s.stage {
            Stage::Area(a) => {
                let w = a.world();
                let played = a.vm().map(blocks).unwrap_or_default();
                let marker = a.vm().and_then(|vm| vm.mng.positions.iter().find(|p| p.num == 1).map(|p| p.pos));
                let free = a.calls().last().is_some_and(|c| c.1 == "menu_clear");
                match (played.contains(&(17, 13)) && free, marker, w.combat().kite) {
                    (true, Some(m), Some(k)) => {
                        let p = w.combat().scene.chars[k].pos.map(f32::from_bits);
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        stick_toward(cam_z, (m[0] - p[0]).atan2(-(m[1] - p[1])))
                    }
                    _ => story_player(&s, f),
                }
            }
            _ => break,
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        if a.vm().is_some_and(|vm| blocks(vm).contains(&(17, 14))) {
            near = Some(a.world().player().body.pos.map(f32::from_bits));
            break;
        }
    }
    eprintln!("now {}; block 14 at {near:?}", Mode::title(&s));
    let unported = piney_event::host::take_unported();
    eprintln!("unported {unported:?}");
    let vm = take_vm(&mut s).expect("the event task");
    let played = blocks(&vm);
    assert!(played.contains(&(17, 13)), "event 17's block 13: {played:?}");
    assert!(played.contains(&(17, 14)), "event 17's block 14 (near_marker): {played:?}");
    eprintln!("blocks {played:?}");
    assert!(!unported.iter().any(|u| u.contains("player_distance")), "unported: {unported:?}");
}

/// Event 17 in story area 18's dungeon: block 7's `entry 4 29` stands the
/// Administrator at point 1 and `add_target` makes talking to him the
/// event's. After block 8's lines Kite walks up to him and presses the
/// action button: block 9 (`if talked_to 4 29`) turns him to Kite
/// (`npc_face`) and says line 10, and can be played again.
#[test]
fn event_17_talks_to_the_administrator() {
    let Some(mut s) = in_story_dungeon(17, 0, 18, 0x7f, (0, 0)) else { return };
    piney_event::host::take_unported();
    let mut pad = Pad::default();
    for f in 0..8000u64 {
        let raw = match &s.stage {
            Stage::Area(a) => {
                let w = a.world();
                let played = a.vm().map(blocks).unwrap_or_default();
                let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                    "menu_ban true" => Some(true),
                    "menu_ban false" => Some(false),
                    _ => None,
                });
                let c = w.combat();
                let admin = c.npcs.iter().find(|n| n.code == 29).map(|n| n.who);
                match (played.contains(&(17, 8)) && banned == Some(false), admin, c.kite) {
                    (true, Some(m), Some(k)) if a.ui().menu_type() == -1 => {
                        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                        if w.command_target() == Some(m) {
                            if f.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still }
                        } else {
                            let p = c.scene.chars[k].pos.map(f32::from_bits);
                            let q = c.scene.chars[m].pos.map(f32::from_bits);
                            let cam_z = f32::from_bits(w.camera().rot()[2]);
                            stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                        }
                    }
                    _ => story_player(&s, f),
                }
            }
            _ => break,
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        if a.vm().is_some_and(|vm| blocks(vm).iter().filter(|&&b| b == (17, 9)).count() >= 2) {
            break;
        }
    }
    eprintln!("now {}", Mode::title(&s));
    let unported = piney_event::host::take_unported();
    let vm = take_vm(&mut s).expect("the event task");
    let played = blocks(&vm);
    eprintln!("blocks {played:?}; unported {unported:?}");
    assert!(played.contains(&(17, 8)), "event 17's block 8: {played:?}");
    let talks = played.iter().filter(|&&b| b == (17, 9)).count();
    assert_eq!(talks, 2, "event 17's block 9 (talked_to 4 29), each talk: {played:?}");
}

/// Kite's HP, SP and `dead`, and whether he is a ghost.
fn kite_state(s: &Session) -> Option<(i16, i16, i16, bool)> {
    let Stage::Area(a) = &s.stage else { return None };
    let c = a.world().combat();
    let k = c.kite?;
    let ch = &c.scene.chars[k];
    let ghost = ch.spc_char.flags & piney_battle::chara::spc_flag::GHOST != 0;
    Some((ch.hp, ch.sp, ch.cond[piney_battle::param::cond::DEAD], ghost))
}

/// The party through a door keeps what the last room left it
/// (`ccStoreSpcCondition` as the next scene's set-up starts,
/// `ccRestoreSpcCondition` in `ccPlayer::ccPlayer` and
/// `ccFellow::Initialize` outside the towns): Kite hurt to 17 HP in area
/// 26's first room is at 17 in the next; Kite down (no HP, `dead` 4) comes
/// back a ghost, not whole.
#[test]
fn the_party_keeps_its_hp_through_a_door() {
    for (hp, sp, dead) in [(17i16, 5i16, 0i16), (0, 0, 4)] {
        let Some(mut s) = in_area_26((0, 0)) else { return };
        // Played until the room's set-up is over and Kite stands.
        let mut pad = Pad::default();
        for f in 0..400 {
            pad.read(&story_player(&s, f));
            s.step(&pad);
            s.take_events();
        }
        let Stage::Area(a) = &mut s.stage else { panic!("not in the dungeon: {}", Mode::title(&s)) };
        let c = a.world_mut().combat_mut();
        let k = c.kite.expect("Kite");
        let ch = &mut c.scene.chars[k];
        ch.hp = hp;
        ch.sp = sp;
        ch.cond.v[piney_battle::param::cond::DEAD] = dead;
        let before = kite_state(&s);
        let mut after = None;
        walk_to(&mut s, (0, 1), 4000, |s, _, _| {
            if let Stage::Area(a) = &s.stage
                && (a.world().scene().floor, a.world().scene().block) == (0, 1)
                && after.is_none()
            {
                after = kite_state(s);
            }
        });
        eprintln!("{hp}/{sp} dead {dead}: before {before:?}, next room {after:?}");
        let (h, p, d, ghost) = after.expect("the next room");
        assert_eq!(h, hp, "HP through the door");
        if hp == 0 {
            assert_eq!((p, d, ghost), (0, 4, true), "a ghost");
        } else {
            // SP regenerates on the walk; not back to full.
            assert!((sp..sp + 10).contains(&p), "SP through the door: {p}");
        }
    }
}

/// `MoveDoor`'s sounds: on the walk through area 26's dungeon, rooms with
/// enemies open their doors when cleared, each door's opening sound
/// (`ccSeOn3D`, by the dungeon's type) at the door.
#[test]
fn the_doors_sound_as_they_open() {
    let Some(mut s) = in_area_26((0, 0)) else { return };
    let mut se = None;
    let mut heard = 0;
    walk_to(&mut s, POINT_1, 20_000, |s, events, _| {
        let Stage::Area(a) = &s.stage else { return };
        if let Place::Dungeon(d) = a.world().place() {
            se = se.or(d.door_se());
        }
        heard +=
            events.iter().filter(|e| matches!(e, Event::Se3d { n, .. } if se.is_some_and(|x| *n as i32 == x))).count();
    });
    eprintln!("door sound {se:?}: heard {heard}");
    assert!(heard > 0, "no door sound on the walk");
}

/// `DUNGEON::EntryBreakObject`: area 26's rooms get their breakables from
/// the `OBJ_0pr*` dummies as the party enters each (gimmick rows 7-14,
/// `entRoot` 2), and leaving a room takes them away; the story room with an
/// event (event 29's) gets none.
#[test]
fn the_rooms_have_their_breakables() {
    let Some(mut s) = in_area_26((0, 0)) else { return };
    let mut by_room: Vec<((i32, i32), usize)> = Vec::new();
    walk_to(&mut s, POINT_1, 20_000, |s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let w = a.world();
        // The room's own play (not the change's fade, where the scene
        // names the next room while the last one's entries stand).
        if !matches!(w.phase(), piney_world::Phase::Play(n) if n > 30) || s.leaving.is_some() {
            return;
        }
        let sc = w.scene();
        let c = w.combat();
        let n = c
            .ctrl
            .list(piney_battle::entry::Kind::Gimmick)
            .into_iter()
            .filter(|&g| {
                matches!(c.ctrl.objs.get(g), Some(piney_battle::entry::Obj::Gimmick(o))
                    if o.ent.ent_root == 2 && (7..=14).contains(&o.ent.id))
            })
            .count();
        match by_room.last_mut() {
            Some((r, m)) if *r == (sc.floor, sc.block) => *m = (*m).max(n),
            _ => by_room.push(((sc.floor, sc.block), n)),
        }
    });
    eprintln!("breakables by room {by_room:?}");
    assert!(by_room.iter().any(|r| r.1 > 0), "no room had breakables: {by_room:?}");
    // Event 29's room (floor 2, room 2: a story room with an event) has none.
    assert!(by_room.iter().any(|r| r.0 == (2, 2)), "never in event 29's room: {by_room:?}");
    assert!(by_room.iter().filter(|r| r.0 == (2, 2)).all(|r| r.1 == 0), "{by_room:?}");
}

/// `ccEvent::MenuBan` / `MenuClr` around an event scene: the party's
/// condition stored and cleared (`ccStoreSpcCondition`,
/// `ClearCondition`), then put back as it was (`ccRestoreSpcCondition`,
/// `ConditionAdjustment`), HP too, over whatever the scene did to it.
#[test]
fn menu_ban_keeps_the_partys_condition() {
    use piney_battle::param::cond;
    let Some(mut s) = in_area_26((0, 0)) else { return };
    let mut pad = Pad::default();
    for f in 0..400 {
        pad.read(&story_player(&s, f));
        s.step(&pad);
        s.take_events();
    }
    let Stage::Area(a) = &mut s.stage else { panic!("not in the dungeon: {}", Mode::title(&s)) };
    let w = a.world_mut();
    let k = w.combat().kite.expect("Kite");
    {
        let ch = &mut w.combat_mut().scene.chars[k];
        ch.hp = 20;
        ch.cond.v[cond::POISON] = 300;
    }
    w.menu_ban_party(true);
    let ch = &w.combat().scene.chars[k];
    assert_eq!((ch.hp, ch.cond[cond::POISON]), (20, 0), "menu_ban clears the conditions, not HP");
    // The scene heals him.
    w.combat_mut().scene.chars[k].hp = 55;
    w.menu_ban_party(false);
    let ch = &w.combat().scene.chars[k];
    assert_eq!((ch.hp, ch.cond[cond::POISON]), (20, 300), "menu_clear puts the stored condition back");
}

/// Pictures of the target cursor (the turned diamond and its corners) on
/// event 17's Administrator, as Kite walks up to him:
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game target_cursor_shot
/// -- --ignored` (default `/mnt/data/claude/scratch/cursor`).
#[test]
#[ignore]
fn target_cursor_shot() {
    let Some(mut s) = in_story_dungeon(17, 0, 18, 0x7f, (0, 0)) else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/cursor".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut seen = 0u32;
    let mut taken = 0;
    for f in 0..4000u64 {
        let raw = match &s.stage {
            Stage::Area(a) => {
                let w = a.world();
                let played = a.vm().map(blocks).unwrap_or_default();
                let c = w.combat();
                let admin = c.npcs.iter().find(|n| n.code == 29).map(|n| n.who);
                let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                    "menu_ban true" => Some(true),
                    "menu_ban false" => Some(false),
                    _ => None,
                });
                match (played.contains(&(17, 8)) && banned == Some(false), admin, c.kite) {
                    (true, Some(m), Some(k)) if w.command_target() != Some(m) => {
                        let p = c.scene.chars[k].pos.map(f32::from_bits);
                        let q = c.scene.chars[m].pos.map(f32::from_bits);
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                    }
                    (true, Some(_), Some(_)) => {
                        Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
                    }
                    _ => story_player(&s, f),
                }
            }
            _ => break,
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        let w = a.world();
        let on = w.command_target().is_some_and(|t| w.combat().npcs.iter().any(|n| n.who == t));
        seen = if on { seen + 1 } else { 0 };
        if seen == 10 + 17 * taken {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/cursor{taken}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            eprintln!("{path}");
            taken += 1;
            if taken == 2 {
                break;
            }
        }
    }
    assert!(taken > 0, "the Administrator was never the target");
}

/// What [`open_an_object`] saw.
#[derive(Default)]
struct ObjRun {
    /// The menus in the order they opened.
    menus: Vec<i32>,
    /// Kite in act 25 (`BreakSomething`) under the object's menu.
    broke: bool,
    /// `saveData.breakCount` before and after.
    count: (i16, i16),
    /// The item GetItemMenu (29) gave.
    item: Option<i32>,
}

/// Another object on a breakable's place: its base type, `skillID`
/// (+0x7c) and item (the entry's `param[1]`, +0x14c).
#[derive(Clone, Copy, Debug)]
struct AsObject {
    ty: i32,
    skill: i16,
    item: i32,
}

/// Area 26's dungeon, room (0, 6), with its breakables (`EntryBreakObject`),
/// walked into from the entrance (the room a session starts in has not had
/// its objects' pass): Kite goes up to the nearest and pushes X, which
/// opens ItemObjMenu (34: its base type 0x10000, `openReqNum` 0x1022; or,
/// the breakable made into `as_obj`, that type's menu) with the menu's
/// `rand()` 0 (the item comes); then until the menus are closed. `each`
/// sees every frame drawn.
fn open_an_object(as_obj: Option<AsObject>, mut each: impl FnMut(&Session, &Frame)) -> Option<ObjRun> {
    let mut s = in_area_26((0, 0))?;
    walk_to(&mut s, (0, 6), 3000, |_, _, _| {});
    // Kite comes into the story start with no HP (a ghost: dead 4), who
    // targets nothing; stood up.
    if let Stage::Area(a) = &mut s.stage {
        let c = a.world_mut().combat_mut();
        if let Some(k) = c.kite {
            let ch = &mut c.scene.chars[k];
            ch.hp = ch.max_hp.max(1);
            ch.cond.v[piney_battle::param::cond::DEAD] = 0;
            ch.spc_char.flags &= !piney_battle::chara::spc_flag::GHOST;
        }
    }
    let mut pad = Pad::default();
    let mut run = ObjRun::default();
    let count = |s: &Session| match &s.stage {
        Stage::Area(a) => a.world().state().save.i16(piney_fieldui::menus::objects::BREAK_COUNT),
        _ => 0,
    };
    run.count.0 = count(&s);
    let mut opened = false;
    // Frames under the object's menu: its window is left up a while.
    let mut under = 0u32;
    for f in 0..6000u64 {
        let raw = {
            let Stage::Area(a) = &mut s.stage else { break };
            // The menu's draw: the item path.
            a.ui_mut().ctrl.rng = Box::new(|| 0);
            // The nearest breakable of the room.
            let near = {
                let c = a.world().combat();
                let p = c.kite.map(|k| c.scene.chars[k].pos.map(f32::from_bits));
                c.ctrl
                    .list(piney_battle::entry::Kind::Gimmick)
                    .into_iter()
                    .filter(|&g| {
                        matches!(c.ctrl.objs.get(g), Some(piney_battle::entry::Obj::Gimmick(o))
                            if o.ent.ent_root == 2 && (7..=14).contains(&o.ent.id))
                    })
                    .min_by(|&x, &y| {
                        let d = |g: usize| {
                            let q = c.scene.chars[g].pos.map(f32::from_bits);
                            p.map_or(0.0, |p| (q[0] - p[0]).hypot(q[1] - p[1]))
                        };
                        d(x).total_cmp(&d(y))
                    })
            };
            if let (Some(o), Some(g), false) = (as_obj, near, opened) {
                let c = a.world_mut().combat_mut();
                c.scene.chars[g].base_mut().ty = o.ty;
                c.scene.chars[g].skill_id = o.skill;
                if let Some(piney_battle::entry::Obj::Gimmick(e)) = c.ctrl.objs.get_mut(g) {
                    e.ent.param[1] = o.item;
                }
            }
            let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                "menu_ban true" => Some(true),
                "menu_ban false" => Some(false),
                _ => None,
            });
            let w = a.world();
            let c = w.combat();
            let ui = a.ui();
            let t = ui.menu_type();
            if run.menus.last() != Some(&t) {
                run.menus.push(t);
            }
            opened |= (34..=39).contains(&t);
            let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 30);
            let press = |b: Buttons| if f.is_multiple_of(9) { Raw { buttons: b, ..still } } else { still };
            match c.kite {
                Some(k) if (34..=39).contains(&t) => {
                    run.broke |= c.scene.chars[k].spc_char.act_num == piney_battle::kite::act::BREAK;
                    under += 1;
                    // The windows' OK (TrapObjMenu's own, the others' Check).
                    if under > 60 { press(Buttons::CROSS) } else { still }
                }
                Some(_) if t == 29 || t == 67 => {
                    run.item.get_or_insert(ui.ctrl.item_num);
                    press(Buttons::CROSS)
                }
                Some(k) if t == -1 && !opened && playing && banned != Some(true) => {
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    match near.map(|g| c.scene.chars[g].pos.map(f32::from_bits)) {
                        Some(q) if (q[0] - p[0]).hypot(q[1] - p[1]) > 90.0 => {
                            let cam_z = f32::from_bits(w.camera().rot()[2]);
                            stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                        }
                        Some(_) if f.is_multiple_of(8) => Raw { buttons: Buttons::CROSS, ..still },
                        _ => still,
                    }
                }
                _ if t == -1 && opened => break,
                _ => story_player(&s, f),
            }
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        each(&s, &frame);
    }
    run.count.1 = count(&s);
    Some(run)
}

/// ItemObjMenu (34) in play ([`open_an_object`]): the menu opens on the
/// breakable, Kite breaks it (act 25), `breakCount` goes up, and the
/// item comes through GetItemMenu (29) before the menus close.
#[test]
fn a_breakable_opens_item_obj_menu() {
    let Some(run) = open_an_object(None, |_, _| {}) else { return };
    eprintln!("menus {:?} count {:?} item {:?}", run.menus, run.count, run.item);
    assert!(run.menus.contains(&34), "ItemObjMenu opened: {:?}", run.menus);
    assert!(run.broke, "Kite broke it (act 25)");
    assert_eq!(run.count.1, run.count.0 + 1, "breakCount up by one");
    assert!(run.menus.contains(&29), "GetItemMenu gave its item: {:?}", run.menus);
    assert!(run.item.is_some_and(|i| i >= 0), "an item: {:?}", run.item);
    assert_eq!(run.menus.last(), Some(&-1), "the menus closed: {:?}", run.menus);
}

/// The other object menus in play, on a breakable made into each kind
/// (no setter places them in Infection's port: see
/// `docs/engine/field-ui.md`): a trapped breakable (35, the poison gas),
/// a symbol (36, Rig Saem), a virus crystal (37, a Virus Core) and the
/// Zeit statue (39); each opens by its base type and closes.
#[test]
fn the_object_menus_open_by_type() {
    for (o, menu) in [
        (AsObject { ty: 0x2_0000, skill: 156, item: -1 }, 35),
        (AsObject { ty: 0x4_0000, skill: 175, item: -1 }, 36),
        (AsObject { ty: 0x8_0000, skill: 0, item: -1 }, 37),
        (AsObject { ty: 0x20_0000, skill: 0, item: -1 }, 39),
    ] {
        let Some(run) = open_an_object(Some(o), |_, _| {}) else { return };
        eprintln!("{o:?}: menus {:?} count {:?} item {:?}", run.menus, run.count, run.item);
        assert!(run.menus.contains(&menu), "menu {menu} opened: {:?}", run.menus);
        assert_eq!(run.menus.last(), Some(&-1), "the menus closed: {:?}", run.menus);
        if menu == 37 {
            assert_eq!(run.item, Some(0xf_0000), "a Virus Core");
        }
    }
}

/// A Grunty food in play: in area 26's dungeon, room (0, 6), a Golden Egg
/// (gimmick row 22, `ccGimFood`, `SetItemBox`'s egg) put 300 ahead of
/// Kite. It wobbles and rolls; as he comes within 1000 it calls out
/// (`ccVoicePgFood`: the voice group the port gives `FOOD.BIN`). He walks
/// up and the action button opens FoodMenu (43, its base type 0x800000):
/// it is taken (act 1, `ccSeOn3DNote(179, pos, 70)`), bursts (act 2,
/// sound 77, `effOpenBox`) and goes; key item 26 comes through
/// GetItemMenu (29) and its `foodCount` goes up.
#[test]
fn a_grunty_food_is_picked_up() {
    use piney_battle::entry::Obj;
    use piney_battle::gimmick::Class;
    let Some(mut s) = in_area_26((0, 0)) else { return };
    walk_to(&mut s, (0, 6), 3000, |_, _, _| {});
    let who = {
        let Stage::Area(a) = &mut s.stage else { panic!("left the dungeon") };
        let c = a.world_mut().combat_mut();
        let k = c.kite.expect("Kite");
        let ch = &mut c.scene.chars[k];
        ch.hp = ch.max_hp.max(1);
        ch.cond.v[piney_battle::param::cond::DEAD] = 0;
        ch.spc_char.flags &= !piney_battle::chara::spc_flag::GHOST;
        let p = ch.pos;
        let h = c.cast.get(k).map_or(0.0, |a| f32::from_bits(a.ch.dirc[2]));
        let q = p.map(f32::from_bits);
        let pos = [(q[0] + 300.0 * h.sin()).to_bits(), (q[1] - 300.0 * h.cos()).to_bits(), p[2], p[3]];
        a.world_mut().entry_gimmick(22, pos, [0; 4]).expect("the egg made")
    };
    let food = |s: &Session| -> Option<i32> {
        let Stage::Area(a) = &s.stage else { return None };
        match a.world().combat().ctrl.objs.get(who) {
            Some(Obj::Gimmick(o)) if matches!(o.class, Class::Food(_)) => Some(o.act_num),
            _ => None,
        }
    };
    assert_eq!(food(&s), Some(0));
    let (items0, count0) = {
        let Stage::Area(a) = &s.stage else { unreachable!() };
        let save = &a.world().state().save;
        (save.u8(0x0cfc + 26), save.i16(piney_fieldui::menus::objects::FOOD_COUNT))
    };
    let mut pad = Pad::default();
    let (mut called, mut noted, mut menus, mut acts) = (false, false, Vec::new(), Vec::new());
    let mut opened = false;
    for f in 0..3000u64 {
        let raw = {
            let Stage::Area(a) = &mut s.stage else { break };
            let t = a.ui().menu_type();
            if menus.last() != Some(&t) {
                menus.push(t);
            }
            opened |= t == 43;
            let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let w = a.world();
            let c = w.combat();
            match (t, c.kite) {
                (29, _) if f.is_multiple_of(9) => Raw { buttons: Buttons::CROSS, ..still },
                (-1, Some(k)) if !opened => {
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let q = c.scene.chars[who].pos.map(f32::from_bits);
                    if (q[0] - p[0]).hypot(q[1] - p[1]) > 90.0 {
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])))
                    } else if f.is_multiple_of(8) {
                        Raw { buttons: Buttons::CROSS, ..still }
                    } else {
                        still
                    }
                }
                (-1, _) if opened && food(&s).is_none() => break,
                _ => still,
            }
        };
        pad.read(&raw);
        s.step(&pad);
        for e in s.take_events() {
            match e {
                Event::Voice { event: piney_data::sound::voice::FOOD_GROUP, msg: 0 } => called = true,
                Event::Se3d { n: 179, note: Some(70), .. } => noted = true,
                _ => {}
            }
        }
        if let Some(act) = food(&s)
            && acts.last() != Some(&act)
        {
            acts.push(act);
        }
    }
    eprintln!("menus {menus:?} acts {acts:?} called {called} noted {noted}");
    assert!(called, "the egg never called out");
    assert!(menus.contains(&43), "FoodMenu opened: {menus:?}");
    assert!(menus.contains(&29), "GetItemMenu gave the egg: {menus:?}");
    assert_eq!(acts, [0, 1, 2], "taken, then burst");
    assert!(noted, "no ccSeOn3DNote(179, 70)");
    assert!(food(&s).is_none(), "the egg is still there");
    let Stage::Area(a) = &s.stage else { panic!("left the dungeon") };
    let save = &a.world().state().save;
    assert_eq!(save.u8(0x0cfc + 26), items0 + 1, "key item 26");
    assert_eq!(save.i16(piney_fieldui::menus::objects::FOOD_COUNT), count0 + 1, "foodCount[0]");
}

/// Shots of a Grunty food ([`a_grunty_food_is_picked_up`]'s egg, a Grunt
/// Mints and a Bloody Egg beside it, 300 ahead of Kite): lying there 40
/// frames on, and 10 frames after one is taken (act 2's burst). Into
/// `PINEY_SHOTS` (default /mnt/data/claude/scratch/grunties/food):
/// `cargo test --release -p piney-game grunty_food_shots -- --ignored
/// --nocapture`.
#[test]
#[ignore]
fn grunty_food_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/grunties/food".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = in_area_26((0, 0)) else { return };
    walk_to(&mut s, (0, 6), 3000, |_, _, _| {});
    let first = {
        let Stage::Area(a) = &mut s.stage else { panic!("left the dungeon") };
        let c = a.world_mut().combat_mut();
        let k = c.kite.expect("Kite");
        let p = c.scene.chars[k].pos;
        let h = c.cast.get(k).map_or(0.0, |a| f32::from_bits(a.ch.dirc[2]));
        let q = p.map(f32::from_bits);
        let at = |side: f32| {
            let (x, y) = (q[0] + 300.0 * h.sin() + side * h.cos(), q[1] - 300.0 * h.cos() + side * h.sin());
            [x.to_bits(), y.to_bits(), p[2], p[3]]
        };
        let w = a.world_mut();
        let first = w.entry_gimmick(22, at(0.0), [0; 4]).expect("the egg made");
        w.entry_gimmick(23, at(-120.0), [0; 4]);
        w.entry_gimmick(37, at(120.0), [0; 4]);
        first
    };
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut shot = |s: &Session, frame: &Frame, name: &str| {
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let (w, h) = gs.target_size();
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
        println!("{path}: {}", Mode::title(s));
    };
    let mut pad = Pad::default();
    pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
    let mut frame = s.step(&pad);
    for _ in 0..40 {
        frame = s.step(&pad);
        s.take_events();
    }
    shot(&s, &frame, "1-foods");
    {
        let Stage::Area(a) = &mut s.stage else { panic!("left the dungeon") };
        let c = a.world_mut().combat_mut();
        c.scene.chars[first].affect.ty = 11;
        if let Some(piney_battle::entry::Obj::Gimmick(o)) = c.ctrl.objs.get_mut(first) {
            o.affect_flag = true;
        }
    }
    for _ in 0..31 {
        frame = s.step(&pad);
        s.take_events();
    }
    shot(&s, &frame, "2-taken");
}

/// Shots of the object menus in play ([`open_an_object`]): each menu 12
/// frames in, its window (50 frames in), and GetItemMenu's line (20
/// frames into 29), into `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/obj_menus): `cargo test --release -p
/// piney-game object_menu_shots -- --ignored --nocapture`.
#[test]
#[ignore]
fn object_menu_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/obj_menus".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs: Option<piney_gs::Gs> = None;
    for (name, o) in [
        ("item_obj", None),
        ("trap_obj", Some(AsObject { ty: 0x2_0000, skill: 156, item: -1 })),
        ("symbol", Some(AsObject { ty: 0x4_0000, skill: 175, item: -1 })),
        ("virus", Some(AsObject { ty: 0x8_0000, skill: 0, item: -1 })),
        ("time_idol", Some(AsObject { ty: 0x20_0000, skill: 0, item: -1 })),
    ] {
        let (mut open, mut since, mut taken) = (-1, 0u32, Vec::new());
        let run = open_an_object(o, |s, frame| {
            let Stage::Area(a) = &s.stage else { return };
            let t = a.ui().menu_type();
            if t != open {
                (open, since) = (t, 0);
            }
            since += 1;
            let shot = match t {
                34..=39 if since == 12 => format!("{name}_{t}_open"),
                34..=39 if since == 50 => format!("{name}_{t}_window"),
                29 if since == 20 => format!("{name}_29"),
                _ => return,
            };
            if taken.contains(&shot) {
                return;
            }
            taken.push(shot.clone());
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            let (wd, ht) = g.target_size();
            let path = format!("{dir}/{shot}.png");
            std::fs::write(&path, piney_gs::png::encode(wd, ht, &g.read_back())).unwrap();
            println!("{path}");
        });
        if run.is_none() {
            return;
        }
        assert!(!taken.is_empty(), "{name}: no shots");
    }
}

/// Event 17's dungeon (story area 18) has a symbol in floor 0's room 2
/// (`D0021`'s type-3 kind-4 row): a `ccGimSymbol` of row 17 with its
/// random skill (`symbolSkillTbl`), its light in the group and its fires
/// burning. Kite walks up to it and the action button opens SymbolMenu
/// (36), naming that skill; its close (affect 11) casts it: act 1, then
/// spent (act 2) with its light out, `symbolCount` one up.
#[test]
fn event_17s_symbol_casts_its_skill() {
    use piney_battle::entry::Obj;
    use piney_battle::gimmick::{Class, SYMBOL_SKILLS};
    let Some(mut s) = in_story_dungeon(17, 0, 18, 0x7f, (0, 2)) else { return };
    let symbol = |s: &Session| -> Option<(usize, i32, i16)> {
        let Stage::Area(a) = &s.stage else { return None };
        let c = a.world().combat();
        c.ctrl.objs.iter().enumerate().find_map(|(who, o)| match o {
            Obj::Gimmick(g) => match &g.class {
                Class::Symbol(sym) => Some((who, sym.act, c.scene.chars[who].skill_id)),
                _ => None,
            },
            _ => None,
        })
    };
    let mut pad = Pad::default();
    let mut seen = None;
    let mut fires = 0;
    let mut lit = false;
    let mut opened = false;
    let mut acts = Vec::new();
    for f in 0..6000u64 {
        let raw = match (&s.stage, symbol(&s)) {
            (Stage::Area(a), Some((who, 0, _))) if a.ui().menu_type() == -1 => {
                let w = a.world();
                let c = w.combat();
                let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
                    "menu_ban true" => Some(true),
                    "menu_ban false" => Some(false),
                    _ => None,
                });
                match c.kite {
                    Some(k) if banned != Some(true) && matches!(w.phase(), piney_world::Phase::Play(n) if n > 12) => {
                        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                        let _ = k;
                        if w.command_target() == Some(who) {
                            if f.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still }
                        } else {
                            still
                        }
                    }
                    _ => story_player(&s, f),
                }
            }
            (Stage::Area(a), _) if a.ui().menu_type() == 36 => {
                opened = true;
                let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
                if f.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still }
            }
            _ => story_player(&s, f),
        };
        // Kite put 150 south of the symbol, facing it, until it is his
        // target (the room's walls stand between it and where he starts).
        let sym_now = symbol(&s);
        if let (Stage::Area(a), Some((who, 0, _))) = (&mut s.stage, sym_now)
            && a.ui().menu_type() == -1
            && a.world().command_target() != Some(who)
            && matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 12)
        {
            let q = a.world().combat().scene.chars[who].pos.map(f32::from_bits);
            let t = |v: f32| (v / 10.0).round() as i16;
            let w = a.world_mut();
            w.pc_command(piney_event::host::PcCommand::Put { pc: 0, x: t(q[0]), y: t(q[1]) - 15, z: t(q[2]) });
            w.pc_command(piney_event::host::PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 });
        }
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        if let Some((who, act, skill)) = symbol(&s) {
            seen.get_or_insert((who, skill));
            if acts.last() != Some(&act) {
                acts.push(act);
            }
            let c = a.world().combat();
            fires = fires.max(c.symbol_fires.len());
            lit |= c.symbol_lights.contains_key(&who);
            if act == 2 && a.ui().menu_type() == -1 {
                break;
            }
        }
    }
    let (who, skill) = seen.expect("event 17's symbol");
    eprintln!("symbol {who}: skill {skill}, acts {acts:?}, fires {fires}, lit {lit}, menu {opened}");
    assert!(SYMBOL_SKILLS.contains(&skill), "skill {skill}");
    assert!(lit, "its light never joined the group");
    assert!(fires > 5, "{fires} fires drawn");
    assert!(opened, "SymbolMenu did not open");
    assert_eq!(acts, [0, 1, 2]);
    let Stage::Area(a) = &s.stage else { panic!("left the dungeon") };
    assert!(!a.world().combat().symbol_lights.contains_key(&who), "the light stayed");
    let count = a.world().state().save.u8(piney_fieldui::menus::objects::SYMBOL_COUNT);
    assert_eq!(count, 1, "symbolCount");
}

/// A picture of event 17's symbol burning (its model, fires and light):
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game symbol_shot --
/// --ignored` (default `/mnt/data/claude/scratch/symbol`).
#[test]
#[ignore]
fn symbol_shot() {
    use piney_battle::entry::Obj;
    use piney_battle::gimmick::Class;
    let Some(mut s) = in_story_dungeon(17, 0, 18, 0x7f, (0, 2)) else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/symbol".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut put_at = None;
    for f in 0..3000u64 {
        let sym = match &s.stage {
            Stage::Area(a) => {
                let c = a.world().combat();
                c.ctrl.objs.iter().enumerate().find_map(|(who, o)| match o {
                    Obj::Gimmick(g) if matches!(g.class, Class::Symbol(_)) => Some(who),
                    _ => None,
                })
            }
            _ => None,
        };
        if let (Stage::Area(a), Some(who)) = (&mut s.stage, sym)
            && put_at.is_none()
            && matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 12)
        {
            let q = a.world().combat().scene.chars[who].pos.map(f32::from_bits);
            let t = |v: f32| (v / 10.0).round() as i16;
            let w = a.world_mut();
            w.pc_command(piney_event::host::PcCommand::Put { pc: 0, x: t(q[0]), y: t(q[1]) - 40, z: t(q[2]) });
            w.pc_command(piney_event::host::PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 });
            put_at = Some(f);
        }
        pad.read(&story_player(&s, f));
        let frame = s.step(&pad);
        s.take_events();
        if put_at.is_some_and(|p| f == p + 90) {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/symbol.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            eprintln!("{path}");
            return;
        }
    }
    panic!("no symbol");
}

/// From Mac Anu, the gate's warp with words 1, 136 (Solitary: field type 4,
/// no field, straight into a lake, dungeon type 8) and 150. The lake's
/// `SetMagicCircle` makes its special objects symbols (gimmick 18) at the
/// rolled `OBJ_0ps*` slots: this one rolls one, in room 1. Kite walks to its
/// room, where it runs `objMain`: its light joins the group and its count
/// goes on (a puff of smoke every other frame), still waiting (act 0).
#[test]
fn a_lake_symbol_glows_and_puffs() {
    use crate::session::area15::{disc, hold, playing, start, wait};
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    hold(&mut s, 128, 128, 200, |s| matches!(&s.stage, Stage::World(_)));
    wait(&mut s, 100);
    s.go(Pending::Words([1, 136, 150]));
    hold(&mut s, 128, 128, 600, |s| playing(s).is_some_and(|w| w.scene().area == kind::DUNGEON));
    let symbol = |s: &Session| -> Option<(usize, (i32, i32), i32, i32)> {
        let Stage::Area(a) = &s.stage else { return None };
        let c = a.world().combat();
        c.ctrl.list(piney_battle::entry::Kind::Gimmick).into_iter().find_map(|g| match c.ctrl.objs.get(g) {
            Some(piney_battle::entry::Obj::Gimmick(o)) if o.ent.id == 18 => match &o.class {
                piney_battle::gimmick::Class::Symbol(sym) => Some((g, (o.ent.floor, o.ent.block), sym.act, sym.cnt)),
                _ => None,
            },
            _ => None,
        })
    };
    let (_, room, _, _) = symbol(&s).expect("the lake's symbol");
    let mut seen: Option<(i32, i32, bool)> = None;
    walk_to(&mut s, (room.0 as usize, room.1 as usize), 3_000, |s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let sc = a.world().scene();
        if (sc.floor, sc.block) != room || seen.is_some() {
            return;
        }
        if let Some((g, _, act, cnt)) = symbol(s)
            && cnt > 20
        {
            seen = Some((act, cnt, a.world().combat().symbol_lights.contains_key(&g)));
        }
    });
    eprintln!("symbol in {room:?}: {seen:?}, {}", Mode::title(&s));
    let (act, _, lit) = seen.expect("never in the symbol's room with it running");
    assert_eq!(act, 0);
    assert!(lit, "its light is not in the group");
}
