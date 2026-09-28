//! Rooms and doors placed as `SetRoom` and `SetDoor` place them
//! (`piney_data::dungeon::place`) against the map the generator makes.
//! Skipped when the disc image is not extracted (set PINEY_ISO to point at it
//! elsewhere); `--nocapture` prints the counts. It checks that every
//! `ROOM_INFO` model's exit markers, turned by `Rz(rotate)`, are exactly its
//! row's exits (the opposite sign fails most rows); that on generated floors
//! every exit lands on the map's door cells, joined rooms' exits facing 1,200
//! apart; and that door pieces reach past the edge, so doorways overlap.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::dungeon::place::{self, Exit, ExitKind, Frame};
use piney_data::dungeon::{self, CELL, Dummies, EAST, INF, NO_ROOM, NORTH, Params, RoomSize, SOUTH, WEST};
use piney_data::iso::Iso;
use piney_data::model;
use piney_data::scene::{self, Scene};

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

/// Half a room's side, in world units.
fn half(size: RoomSize) -> f32 {
    size.cells() as f32 * CELL / 2.0
}

/// Where exits sit: this far inside the room's edge.
const INSET: f32 = 600.0;

fn opposite(side: u8) -> u8 {
    match side {
        NORTH => SOUTH,
        SOUTH => NORTH,
        WEST => EAST,
        _ => WEST,
    }
}

/// The sides of `mask` turned by `angle` about z.
fn turned(mask: u8, angle: f32) -> u8 {
    let dirs = [(NORTH, Vec3::NEG_Y), (SOUTH, Vec3::Y), (WEST, Vec3::NEG_X), (EAST, Vec3::X)];
    dirs.iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, v)| place::side_of(Mat4::from_rotation_z(angle).transform_vector3(*v)))
        .fold(0, |a, b| a | b)
}

struct TypeFile {
    c: Ccs,
    sc: Scene,
    models: Vec<model::Model>,
    exits: HashMap<&'static str, Vec<Exit>>,
    reach: HashMap<&'static str, [f32; 4]>,
}

impl TypeFile {
    fn load(arc: &Archive, dtype: u8) -> TypeFile {
        let c = Ccs::parse(arc.inflate_named(INF.ccs_name(dtype, 0)).unwrap()).unwrap();
        let sc = Scene::read(&c).unwrap();
        let models = model::models(&c).unwrap();
        TypeFile { c, sc, models, exits: HashMap::new(), reach: HashMap::new() }
    }

    /// How far a room model's floor reaches along `out` (from the centre)
    /// inside the doorway band - within 300 of the axis, at floor height -
    /// leaving out its gate walls.
    fn floor_reach(&self, anime: &'static str, out: Vec3) -> f32 {
        let (c, sc, models) = (&self.c, &self.sc, &self.models);
        let across = Vec3::new(out.y, -out.x, 0.0);
        let mut best = f32::MIN;
        for p in place::pieces(c, sc, anime, Frame::First).unwrap() {
            if c.object_name(p.target).is_some_and(|n| n.starts_with(INF.doors.gate)) {
                continue;
            }
            for m in models.iter().filter(|m| sc.model_owner.get(&m.object) == Some(&p.target)) {
                let pose = HashMap::from([(p.target, p.world)]);
                for placed in scene::place(sc, m, Some(&pose)).into_iter().flatten() {
                    for v in placed.positions.iter().filter(|v| v.dot(across).abs() < 300.0 && v.z.abs() < 10.0) {
                        best = best.max(v.dot(out));
                    }
                }
            }
        }
        best
    }

    /// How far a room model's geometry reaches from its centre towards
    /// local north, south, west and east.
    fn reach(&mut self, anime: &'static str) -> [f32; 4] {
        let (c, sc, models) = (&self.c, &self.sc, &self.models);
        *self.reach.entry(anime).or_insert_with(|| {
            let mut out = [f32::MIN; 4];
            for p in place::pieces(c, sc, anime, Frame::First).unwrap() {
                for m in models.iter().filter(|m| sc.model_owner.get(&m.object) == Some(&p.target)) {
                    let pose = HashMap::from([(p.target, p.world)]);
                    for placed in scene::place(sc, m, Some(&pose)).into_iter().flatten() {
                        for v in &placed.positions {
                            for (r, d) in out.iter_mut().zip([-v.y, v.y, -v.x, v.x]) {
                                *r = r.max(d);
                            }
                        }
                    }
                }
            }
            out
        })
    }

    fn exits(&mut self, anime: &'static str) -> &[Exit] {
        let (c, sc) = (&self.c, &self.sc);
        self.exits.entry(anime).or_insert_with(|| {
            let pieces = place::pieces(c, sc, anime, Frame::First).unwrap();
            place::exits(c, &INF.doors, &pieces)
        })
    }
}

#[test]
fn every_room_model_marks_its_exits() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let (mut checked, mut fit, mut fit_opposite) = (0, 0, 0);
    let (mut doors, mut gates) = (0, 0);
    // How far each room's own floor runs past its door dummies: the least.
    let mut overlap = f32::MAX;
    for dtype in 0..10u8 {
        let mut f = TypeFile::load(&arc, dtype);
        let mut seen = HashSet::new();
        for set in &INF.rooms {
            let table = set[dtype as usize];
            let size = match table.name.as_bytes()[0] {
                b's' => RoomSize::Small,
                b'm' => RoomSize::Medium,
                _ => RoomSize::Large,
            };
            for row in table.rows {
                for &m in row.models {
                    if !seen.insert((table.name, row.exits.0, m)) {
                        continue;
                    }
                    let exits = f.exits(m).to_vec();
                    let mut local = 0u8;
                    for e in &exits {
                        let side = e.side();
                        let at = e.world.transform_point3(Vec3::ZERO);
                        let out = e.world.transform_vector3(Vec3::Y);
                        // On the side's middle, INSET inside the edge, facing out.
                        let d = half(size) - INSET;
                        assert!(side != 0, "{m}: exit not along an axis");
                        assert!((at - out * d).length() < 0.5 && at.z.abs() < 0.5, "{m}: exit at {at}");
                        assert_eq!(local & side, 0, "{m}: two exits on side {side}");
                        local |= side;
                        // SetDoor places with the dummy's own matrix.
                        if e.kind == ExitKind::Door {
                            assert!(e.own.abs_diff_eq(e.world, 1e-3), "{m}: door dummy under a parent");
                            let past = f.floor_reach(m, out) - d;
                            assert!(past > 0.0, "{m}: the floor stops {past} short of the door");
                            overlap = overlap.min(past);
                            doors += 1;
                        } else {
                            gates += 1;
                        }
                    }
                    checked += 1;
                    fit += usize::from(turned(local, row.rotate.radians()) == row.exits.sides());
                    fit_opposite += usize::from(turned(local, -row.rotate.radians()) == row.exits.sides());
                    assert_eq!(
                        turned(local, row.rotate.radians()),
                        row.exits.sides(),
                        "type {dtype} {} row exits {:#04x} rotate {:?}: {m} marks {local:#04x}",
                        table.name,
                        row.exits.0,
                        row.rotate
                    );
                }
            }
        }
    }
    eprintln!(
        "{checked} (table, row, model) triples of all ten types: {fit} fit Rz(rotate), {fit_opposite} fit \
         Rz(-rotate); {doors} door dummies, {gates} gate walls; every room's floor runs at least {overlap:.1} \
         under its doors"
    );
    assert_eq!(fit, checked);
    assert!(fit_opposite < checked * 2 / 3, "the opposite sign fits {fit_opposite} of {checked}");
}

#[test]
fn doorways_reach_past_the_edge() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    // Along +y, in the piece's own space: the door animation's models (open
    // and closed), and the gate wall's.
    for dtype in [0u8, 1, 2, 3, 8] {
        let mut f = TypeFile::load(&arc, dtype);
        // The gate wall: the target of the first gate copy in the type's rooms.
        let rooms: Vec<&'static str> =
            INF.rooms.iter().flat_map(|s| s[dtype as usize].rows).flat_map(|r| r.models).copied().collect();
        let gate = rooms.iter().find_map(|m| f.exits(m).iter().find(|e| e.kind == ExitKind::Gate).map(|e| e.target));
        let models = &f.models;
        let span = |pieces: &[place::Piece]| {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for p in pieces.iter().filter(|p| p.alpha > 0.0) {
                for m in models.iter().filter(|m| f.sc.model_owner.get(&m.object) == Some(&p.target)) {
                    let pose = HashMap::from([(p.target, p.world)]);
                    for placed in scene::place(&f.sc, m, Some(&pose)).into_iter().flatten() {
                        for v in &placed.positions {
                            lo = lo.min(v.y);
                            hi = hi.max(v.y);
                        }
                    }
                }
            }
            (lo, hi)
        };
        let name = INF.doors.anime[dtype as usize];
        for frame in [Frame::First, Frame::Last] {
            let (lo, hi) = span(&place::pieces(&f.c, &f.sc, name, frame).unwrap());
            eprintln!("type {dtype} door {name} ({frame:?}): y {lo:.1} .. {hi:.1}");
            assert!(hi > INSET + 199.0, "door spans {lo} .. {hi}");
        }
        if let Some(obj) = gate {
            let piece =
                place::Piece { controller: obj, target: obj, own: Mat4::IDENTITY, world: Mat4::IDENTITY, alpha: 1.0 };
            let (lo, hi) = span(&[piece]);
            eprintln!("type {dtype} gate {}: y {lo:.1} .. {hi:.1}", INF.doors.gate);
            assert!(hi > INSET + 199.0, "gate spans {lo} .. {hi}");
        }
    }
}

/// A seed sequence (the game's own RNG step).
fn seeds(n: usize, start: u32) -> Vec<u32> {
    let mut r = dungeon::Rng::new(start);
    (0..n).map(|_| r.advance()).collect()
}

#[test]
fn generated_floors_join_up() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let (mut dungeons, mut floors, mut rooms, mut exits_seen, mut joins) = (0, 0, 0, 0, 0);
    let mut kinds: HashMap<ExitKind, usize> = HashMap::new();
    let (mut opposite_misses, mut statues) = (0, 0);
    for dtype in 0..10u8 {
        let mut f = TypeFile::load(&arc, dtype);
        let dummies = Dummies::read(&f.c, &INF.gim_patterns).unwrap();
        for (n, seed) in seeds(40, 0x5eed + u32::from(dtype)).into_iter().enumerate() {
            let size = INF.dungeon_data[1 + n % 10];
            let p = Params {
                seed,
                dtype,
                level_max: size.levels,
                room_max: size.rooms,
                server: (n % 5) as u32,
                volume: 1,
                word_a: 0,
                field_type: if dungeon::is_lake(dtype) { 4 } else { 0 },
                code: 0,
            };
            let Ok(d) = dungeon::generate(&INF, &p, &dummies) else { continue };
            dungeons += 1;
            for fl in &d.floors {
                floors += 1;
                // (room, side) -> (position, outward, room beyond)
                let mut ends: HashMap<(usize, u8), (Vec3, Vec3, u8)> = HashMap::new();
                for room in &fl.rooms {
                    let Some(m) = &room.model else { continue };
                    rooms += 1;
                    let root = place::room_matrix(room);
                    let flipped = Mat4::from_translation(Vec3::new(room.pos[0], room.pos[1], 0.0))
                        * Mat4::from_rotation_z(-m.rotate.radians());
                    if m.statue {
                        // The Gott statue room (symroom) is built through its
                        // doorway: sd1-sd5's are one model with no marker
                        // (so SetDoor finds no door), sd4's has its gate
                        // wall at the centre.
                        assert!(f.exits(m.name).iter().all(|e| e.kind == ExitKind::Gate), "{}", m.name);
                        let side = room.exits.sides();
                        let local = turned(side, -m.rotate.radians());
                        let k = [NORTH, SOUTH, WEST, EAST].iter().position(|&b| b == local).unwrap();
                        let reach = f.reach(m.name)[k];
                        assert!(reach >= half(room.size) + 200.0 - 1.0, "{}: reaches {reach} towards its exit", m.name);
                        statues += 1;
                        continue;
                    }
                    let mut sides = 0u8;
                    for e in f.exits(m.name).to_vec() {
                        let w = root * e.world;
                        let at = w.transform_point3(Vec3::ZERO);
                        let out = w.transform_vector3(Vec3::Y);
                        let side = place::side_of(out);
                        if room.exits.sides() & place::side_of((flipped * e.world).transform_vector3(Vec3::Y)) == 0 {
                            opposite_misses += 1;
                        }
                        assert!(
                            room.exits.has(side),
                            "room {} exits {:?}: an exit faces {side}",
                            room.index,
                            room.exits
                        );
                        assert_eq!(sides & side, 0);
                        sides |= side;
                        // The two door cells either side of the exit.
                        let across = Vec3::new(out.y, -out.x, 0.0) * (CELL / 2.0);
                        let mut beyond = None;
                        for q in [at + across, at - across] {
                            let cell = fl.map.get((q.x / CELL).floor() as i32, (q.y / CELL).floor() as i32).unwrap();
                            assert_eq!(
                                (cell.here, cell.door),
                                (room.index as u8, side),
                                "room {} side {side}: exit at {at} is not on its door cells",
                                room.index
                            );
                            assert_ne!(cell.next, NO_ROOM);
                            assert!(beyond.is_none_or(|b| b == cell.next));
                            beyond = Some(cell.next);
                        }
                        ends.insert((room.index, side), (at, out, beyond.unwrap()));
                        exits_seen += 1;
                        *kinds.entry(e.kind).or_default() += 1;
                    }
                    assert_eq!(
                        sides,
                        room.exits.sides(),
                        "type {dtype} room {} ({}) is missing exits",
                        room.index,
                        m.name
                    );
                }
                // Each joined pair: facing each other, 2 * INSET apart,
                // either side of the first room's edge.
                for (&(a, side), &(at, out, b)) in &ends {
                    if fl.rooms[b as usize].model.as_ref().is_some_and(|m| m.statue) {
                        continue;
                    }
                    let &(bt, bout, back) = &ends[&(b as usize, opposite(side))];
                    assert_eq!(back as usize, a);
                    assert!(out.abs_diff_eq(-bout, 1e-4));
                    assert!((bt - (at + out * 2.0 * INSET)).length() < 0.5, "rooms {a}, {b}: {at} and {bt}");
                    let ra = &fl.rooms[a];
                    let centre = Vec3::new(ra.pos[0], ra.pos[1], 0.0);
                    let edge = (at + bt) * 0.5 - centre;
                    assert!((edge.dot(out) - half(ra.size)).abs() < 0.5, "rooms {a}, {b}: the edge is off");
                    joins += 1;
                }
            }
        }
    }
    eprintln!(
        "{dungeons} dungeons of all ten types, servers 0-4: {floors} floors, {rooms} rooms, {exits_seen} exits \
         ({} door dummies, {} gate walls) all on their door cells, {} joins all facing on one line {} apart, \
         {statues} statue rooms built through their doorway; with Rz(-rotate) {opposite_misses} exits would face a \
         side the map does not open",
        kinds.get(&ExitKind::Door).unwrap_or(&0),
        kinds.get(&ExitKind::Gate).unwrap_or(&0),
        joins / 2,
        2.0 * INSET
    );
    assert!(joins > 1000 && statues > 100 && opposite_misses > exits_seen / 5);
}
