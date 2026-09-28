//! Answers `tools/test_dungeon_rt.py`: builds a dungeon as `WORLD_MAN::GO(2)`
//! does (`piney_world::dungeon_area`), sets its rooms up, queries their
//! collision and walks through their doors and stairs on the requests it
//! is sent, and prints one JSON line for each, so the test can run the same
//! through the game's own code in eemu.
//!
//! ```text
//! cargo build --release -p piney-world --example dungeon_probe
//! dungeon_probe ISO < requests
//! ```
//!
//! Numbers are decimal; floats travel as their bit patterns (hex in the
//! requests). Requests, one a line:
//! - `new S0 S1 S2 FIELDTYPE WEATHER LEVELMAX ROOMMAX HACK FIELD DUNGEON
//!   SERVER`: the dungeon of an area with those `WORLD_MAN` words and
//!   `game.field`, `game.dungeon`, `game.server` (`DungeonArea::new`):
//!   its type, file, fog row, each floor's stairs rooms, startpos and
//!   rooms, and `SetRoom(0, 0)`'s hit list and dressing.
//! - `room F I`: `DUNGEON::SetRoom(F, I)`; the hit list, and the dressing
//!   (the water, the sparks, the room lights and their omni lights, the
//!   clumps and animated objects) with `fieldrand` after it.
//! - `land X Y Z`: `ccLandHitCheck(pos, 0x20000001)` over the hit list:
//!   the height, the result count, the nearest result and
//!   `checkHitResultAttlibute`.
//! - `height X Y`: `WORLD_MAN::GetHeight`.
//! - `goto X Y Z LEVEL FLOOR BLOCK`: `DUNGEON::GotoNextRoom(position, now)`
//!   on floor LEVEL with `game.floor` FLOOR and `game.block` BLOCK: the
//!   answer, `WORLD_MAN.position`, the room built and its hit list.
//! - `drawn X Y` (hex): what `DUNGEON::Draw` draws of the room for a
//!   player at (X, Y): the cell's room (-1 none), the room (0 / 1) and its
//!   doors.
//! - `roomc F I CLEAR`: `DUNGEON::SetRoom(F, I)` with `ccCheckActiveObject(F,
//!   I)` answering CLEAR (0 / 1): the doors, the door words and the hit
//!   list.
//! - `movedoor LEVEL HERE CLEARALL CLEARHERE`: one `DUNGEON::MoveDoor(HERE)`
//!   on floor LEVEL with `ccCheckActiveObject()` answering CLEARALL and
//!   `ccCheckActiveObject(LEVEL, HERE)` CLEARHERE: the doors' sounds (0
//!   opening, 1 closing, and where), the door words and the hit list.
//! - `opendoor F I`, `closedoor F I`, `closedoor2`: `DUNGEON::OpenDoor`,
//!   `CloseDoor`, `CloseDoor2`; the door words and the hit list.
//! - `info F I`: `DUNGEON::GetRoom2DPos` of room I on floor F.
//! - `draweff N [X Y]`: N frames of `DUNGEON::DrawEff`: each frame's
//!   `ccEff::Draw`s (name, place, pattern, transparency), the room lights'
//!   intensities and `fieldrand`; in a lake, the fireflies' too, with the
//!   player at (X, Y, 0) (hex).
//! - `drawbg HERE N`: N frames of a lake's `DUNGEON::DrawBG(HERE)`: the
//!   halfword it writes to the scrolled material's V, and the clumps'
//!   matrix.
//! - `rng SEED COUNT`: `fieldrand`'s seed and `randcnt` set (a story
//!   dungeon's `Generate` draws for its gimmicks, which the port does not).
//! - `select F I`: `WORLD_MAN::RoomSelect(F, I)`'s dungeon part
//!   (`ClearRoom`, `SetRoom(F, I)` with the room clear,
//!   `DUNGEON::RoomSelect`): `WORLD_MAN.position`, `level`,
//!   `roomEnterFlag`, `specialRoom` and the hit list.
//! - `ban A D F B`: `ccSaveData::SetAreaBan` on the probe's save (a new
//!   game's: every entry -1), which the dungeon reads from then on
//!   (`GetBanRoom`, `GotoNextRoom`); `bans`: the 32 entries.
//! - `enter X Y Z LEVEL FLOOR BLOCK`: `WORLD_MAN::Enter`'s dungeon part: the
//!   change it asks for (`scene` or `area`, or none).
//! - `fog`: the last `SetRoom`'s `SetFog` (near, far, 0, max, colour) and
//!   `SetAmbient` (the light group's ambient, bits).
//! - `eventdata EVENT AREA F20` (F20 hex): `WORLD_MAN::SetEventData` for
//!   story area EVENT with `game.area` AREA and the caller's `$f20`: the
//!   points, positions and warp points.
//! - `slots P0..P15 Q0..Q15 CALL..`: `ccEvent::SetEventPoint` (`p:F:B:N`)
//!   and `SetEventPos` (`q:F:B:N:DIRC:X:Y:Z`, the floats hex) called in
//!   turn on an event manager whose 16 points and 16 positions hold the
//!   numbers P and Q (-1 free; floor and block -1, the rest 0): every
//!   point and position after.
//! - `start X Y Z DIRCZ SCHEME MODE SEED` (hex): Kite standing in the room
//!   built, as `ccPlayer::ccPlayer` leaves him in a dungeon (act 2 at once,
//!   his body on the character list), the camera as `cameraInit(0)` leaves
//!   it, scheme SCHEME, `cameraMode` MODE, `rand` seeded with SEED.
//! - `pad DIRECT PUSH POWL DIRCL POWR DIRCR POW0 .. POW11` (hex): one frame
//!   of `cameraMain` and `ccPlayer::Main` over the room's collision, as
//!   `world_probe`'s, and whether he stepped on an entrance
//!   (`WORLD_MAN::Enter`).

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_world::area::{Scene, WorldMan};
use piney_world::camera::{CamPad, Camera, Scheme};
use piney_world::dungeon_area::{DungeonArea, Exit};
use piney_world::ee::{self, V4};
use piney_world::hit::LAND_MASK;
use piney_world::motion::{self, PLAYER_ANIM_TBL};
use piney_world::player::Player;
use piney_world::{Rand, tasks};

/// Kite and the camera in the dungeon.
struct Run {
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn mat(m: &[V4; 4]) -> String {
    list(&m.iter().flatten().copied().collect::<Vec<_>>())
}

fn hit_list(d: &DungeonArea) -> String {
    let names = d.hit_names();
    let items: Vec<String> = d
        .hits
        .models
        .iter()
        .zip(names)
        .map(|(h, n)| format!("[\"{n}\", {}, {}, {}]", h.kind, mat(&h.rm), mat(&h.im)))
        .collect();
    format!("[{}]", items.join(", "))
}

/// What `SetRoom` stood in the room ([`piney_world::dungeon_area::Dressing`]),
/// and `fieldrand` after it.
fn dress(d: &DungeonArea) -> String {
    let x = &d.dress;
    let anm_name = |a: &piney_world::dungeon_area::Anm| -> String {
        d.file.ccs.object_name(d.file.anims[a.play.anim].object).unwrap_or("?").to_string()
    };
    let water = match &x.water {
        Some(w) => format!("[\"{}\", {}]", w.name, mat(&w.root)),
        None => "null".into(),
    };
    let sparks: Vec<String> = x
        .sparks
        .iter()
        .map(|s| {
            format!(
                "[{}, {}, {}, {}, {}, {}, {}]",
                list(&s.base),
                list(&s.pos),
                list(&s.dir),
                s.life,
                s.cnt,
                list(&s.eff_pos),
                s.transparency
            )
        })
        .collect();
    let lights: Vec<String> = x
        .lights
        .iter()
        .map(|r| {
            let l = &d.lights.lights[r.light];
            let (e2, p2) = r.eff2.map_or(("", [0; 4]), |(e, p)| (e, p));
            format!(
                "[\"{}\", {}, {}, \"{e2}\", {}, {}, {}, {}, {}]",
                r.eff,
                list(&r.pos),
                r.pat,
                list(&p2),
                r.pat2,
                list(&l.pos.to_array().map(f32::to_bits)),
                list(&l.colour.to_array().map(f32::to_bits)),
                l.intensity.to_bits()
            )
        })
        .collect();
    let objects: Vec<String> = x.objects.iter().map(|c| format!("[\"{}\", {}]", c.name, mat(&c.m))).collect();
    let anms = |v: &[piney_world::dungeon_area::Anm]| -> String {
        let items: Vec<String> = v.iter().map(|a| format!("[\"{}\", {}]", anm_name(a), mat(&a.root))).collect();
        format!("[{}]", items.join(", "))
    };
    let clut = match d.spark_clut() {
        Some((n, o)) => format!("[\"{n}\", \"{o}\"]"),
        None => "null".into(),
    };
    let fireflies: Vec<String> =
        d.lake.iter().flat_map(|l| &l.fireflies).map(|f| format!("[{}, {}]", list(&f.base[..3]), f.pattern)).collect();
    format!(
        "{{\"water\": {water}, \"sparks\": [{}], \"lights\": [{}], \"objects\": [{}], \"anmobj\": {}, \
         \"anmobj2\": {}, \"clut\": {clut}, \"fireflies\": [{}], \"rng\": [{}, {}]}}",
        sparks.join(", "),
        lights.join(", "),
        objects.join(", "),
        anms(&x.anm_objects),
        anms(&x.anm_objects2),
        fireflies.join(", "),
        d.rng.seed,
        d.rng.count
    )
}

fn doors(d: &DungeonArea) -> String {
    let s = &d.door;
    format!(
        "\"doors\": {}, \"state\": [{}, {}, {}, {}, {}, {}], \"hits\": {}",
        d.doors.len(),
        s.close_start,
        s.lock_num,
        s.lock_off,
        u8::from(s.door_anm),
        u8::from(s.door_flag),
        u8::from(s.still_open),
        hit_list(d)
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut d: Option<DungeonArea> = None;
    let mut scene = Scene::init();
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
    let mut run: Option<Run> = None;
    // A new game's save: every area ban free (-1).
    let mut save = piney_data::save::SaveData::new();
    // WORLD_MAN.lastRoom (+0x100), which a field type 4 area's stairs keep.
    let mut last_room = 0;
    for k in 0..32 * 4 {
        save.set_u8(piney_data::save::offset::AREA_BAN + k, 0xff);
    }
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let Some(&cmd) = w.first() else { continue };
        let n = |i: usize| -> i64 { w[i].parse().unwrap() };
        match cmd {
            "new" => {
                for k in 0..32 * 4 {
                    save.set_u8(piney_data::save::offset::AREA_BAN + k, 0xff);
                }
                let wm = WorldMan {
                    field_seed: 0,
                    dungeon_seed: [n(1) as u32, n(2) as u32, n(3) as u32],
                    field_type: n(4) as u32,
                    weather: n(5) as u32,
                    ground: 0,
                    object: 0,
                    event: n(9) as i32,
                    protect: false,
                    level_max: n(6) as u32,
                    room_max: n(7) as u32,
                    hack: n(8) as u32,
                    field_model: 0,
                    dungeon_type: [0; 3],
                    words: [0; 3],
                    ..WorldMan::default()
                };
                scene = Scene::init();
                scene.area = 2;
                scene.field = n(9) as i32;
                scene.dungeon = n(10) as i32;
                scene.floor = 0;
                scene.block = 0;
                scene.server = n(11) as i32;
                match DungeonArea::new(&archive, &wm, &scene) {
                    Ok(a) => {
                        let floors: Vec<String> = a
                            .floors
                            .iter()
                            .map(|f| {
                                let rooms: Vec<String> = f
                                    .rooms
                                    .iter()
                                    .map(|s| {
                                        format!(
                                            "[{}, {}, {}, \"{}\", {}]",
                                            s.pos[0],
                                            s.pos[1],
                                            s.rotate,
                                            s.model.unwrap_or(""),
                                            u8::from(s.made)
                                        )
                                    })
                                    .collect();
                                format!(
                                    "{{\"up\": {}, \"down\": {}, \"start\": [{}, {}], \"rooms\": [{}]}}",
                                    f.up,
                                    f.down,
                                    list(&f.start[0]),
                                    list(&f.start[1]),
                                    rooms.join(", ")
                                )
                            })
                            .collect();
                        let fog = &a.fog;
                        println!(
                            "{{\"dtype\": {}, \"event\": {}, \"file\": \"{}\", \"spccs\": \"{}\", \"clut\": {}, \"tex\": {}, \
                             \"fog_va\": {}, \"fog_index\": {}, \"fog\": {}, \"near\": {}, \"far\": {}, \
                             \"max\": {}, \"ambient\": {}, \"position\": {}, \"floors\": [{}], \"hits\": {}, \
                             \"dress\": {}}}",
                            a.dtype,
                            a.event,
                            a.file.stem,
                            a.spccs.as_ref().map_or("", |r| r.file.stem.as_str()),
                            a.tex_clut.clut_type,
                            a.tex_clut.tex_type,
                            a.fog_table.0,
                            a.fog_table.1,
                            fog.packed_colour(),
                            fog.near.to_bits(),
                            fog.far.to_bits(),
                            fog.max.to_bits(),
                            list(&piney_world::dungeon_area::ambient_bits(fog)),
                            list(&a.position),
                            floors.join(", "),
                            hit_list(&a),
                            dress(&a)
                        );
                        d = Some(a);
                    }
                    Err(e) => println!("{{\"error\": \"{e}\"}}"),
                }
            }
            "room" => {
                let a = d.as_mut().unwrap();
                a.set_room(n(1) as usize, n(2) as usize);
                println!("{{\"doors\": {}, \"hits\": {}, \"dress\": {}}}", a.doors.len(), hit_list(a), dress(a));
            }
            "draweff" => {
                let a = d.as_mut().unwrap();
                let mut frames = Vec::new();
                let player = (w.len() > 3).then(|| [hex(w[2]), hex(w[3]), 0, ee::ONE]);
                for _ in 0..n(1) {
                    let mut ops = a.draw_eff(true);
                    if let Some(p) = player {
                        a.draw_fireflies(p, true, &mut ops);
                    }
                    let sprites: Vec<String> = ops
                        .iter()
                        .filter_map(|o| match o {
                            piney_world::field_ambient::Op::Sprite(s) => {
                                Some(format!("[\"{}\", {}, {}, {}]", s.name, list(&s.pos), s.pattern, s.transparency))
                            }
                            _ => None,
                        })
                        .collect();
                    let lights: Vec<u32> =
                        a.dress.lights.iter().map(|r| a.lights.lights[r.light].intensity.to_bits()).collect();
                    frames.push(format!(
                        "{{\"sprites\": [{}], \"lights\": {}, \"rng\": [{}, {}]}}",
                        sprites.join(", "),
                        list(&lights),
                        a.rng.seed,
                        a.rng.count
                    ));
                }
                println!("{{\"frames\": [{}]}}", frames.join(", "));
            }
            "drawbg" => {
                let a = d.as_mut().unwrap();
                let here = Some(n(1) as usize);
                let frames: Vec<String> = (0..n(2))
                    .map(|_| {
                        let v = a.bg_scroll(true).map_or(-1, |(_, v)| i64::from(v));
                        let m =
                            a.bg_world(here).map_or_else(Vec::new, |m| m.to_cols_array().map(f32::to_bits).to_vec());
                        format!("[{v}, {}]", list(&m))
                    })
                    .collect();
                let clumps = a.lake.as_ref().map_or(0, |l| l.clumps.len());
                println!("{{\"clumps\": {clumps}, \"frames\": [{}]}}", frames.join(", "));
            }
            "rng" => {
                let a = d.as_mut().unwrap();
                a.rng = piney_data::dungeon::Rng { seed: n(1) as u32, count: n(2) as u32 };
                println!("{{\"rng\": [{}, {}]}}", a.rng.seed, a.rng.count);
            }
            "select" => {
                let a = d.as_mut().unwrap();
                a.room_select(n(1) as usize, n(2) as usize, true);
                println!(
                    "{{\"position\": {}, \"level\": {}, \"enter\": {}, \"special\": {}, \"hits\": {}}}",
                    list(&a.position),
                    a.level,
                    u8::from(a.room_enter),
                    a.special_room,
                    hit_list(a)
                );
            }
            "roomc" => {
                let a = d.as_mut().unwrap();
                a.level = n(1) as usize;
                a.set_room_with(n(1) as usize, n(2) as usize, n(3) != 0);
                println!("{{{}}}", doors(a));
            }
            "movedoor" => {
                let a = d.as_mut().unwrap();
                a.level = n(1) as usize;
                let sounds = a.move_door(n(2) as usize, n(3) != 0, n(4) != 0);
                let sounds: Vec<String> =
                    sounds.iter().map(|(k, p)| format!("[{k}, {}, {}, {}]", p[0], p[1], p[2])).collect();
                println!("{{\"sounds\": [{}], {}}}", sounds.join(", "), doors(a));
            }
            "opendoor" => {
                let a = d.as_mut().unwrap();
                a.open_door(n(1) as usize, n(2) as usize);
                println!("{{{}}}", doors(a));
            }
            "closedoor" => {
                let a = d.as_mut().unwrap();
                a.close_door(n(1) as usize, n(2) as usize);
                println!("{{{}}}", doors(a));
            }
            "closedoor2" => {
                let a = d.as_mut().unwrap();
                a.close_door2();
                println!("{{{}}}", doors(a));
            }
            "warnrows" => {
                // The boss rooms' warnings' rows (type 3, kind 6): floor,
                // room, x, y.
                let a = d.as_ref().unwrap();
                let rows: Vec<String> = a
                    .edit
                    .map(|e| {
                        e.gimmicks
                            .iter()
                            .filter(|g| g.gim_type == 3 && g.kind == 6)
                            .map(|g| format!("[{}, {}, {}, {}]", g.floor, g.index, g.x, g.y))
                            .collect()
                    })
                    .unwrap_or_default();
                println!("{{\"rows\": [{}]}}", rows.join(", "));
            }
            "doormarks" => {
                let a = d.as_ref().unwrap();
                match a.door_marks(n(1) as usize, n(2) as usize) {
                    Some(v) => {
                        let v: Vec<String> = v.iter().map(|p| list(p)).collect();
                        println!("{{\"marks\": [{}]}}", v.join(", "));
                    }
                    None => println!("{{\"marks\": null}}"),
                }
            }
            "neardoor" => {
                let a = d.as_ref().unwrap();
                let at = [n(3) as u32, n(4) as u32, n(5) as u32, n(6) as u32];
                match a.near_door(n(1) as usize, n(2) as usize, at) {
                    Some(p) => println!("{{\"pos\": {}}}", list(&p)),
                    None => println!("{{\"pos\": null}}"),
                }
            }
            "info" => {
                let a = d.as_mut().unwrap();
                a.level = n(1) as usize;
                println!("{{\"info\": {:?}}}", a.map_2d_info(n(2) as i32));
            }
            "eventdata" => {
                let e = piney_world::dungeon_area::set_event_data(
                    &piney_data::dungeon::INF,
                    n(1) as i32,
                    n(2) as i32,
                    hex(w[3]),
                );
                let points: Vec<String> = e.points.iter().map(|p| format!("{p:?}")).collect();
                let pos: Vec<String> = e
                    .positions
                    .iter()
                    .map(|(k, dirc, p)| format!("[{}, {}, {}, {dirc}, {}, {}, {}]", k[0], k[1], k[2], p[0], p[1], p[2]))
                    .collect();
                let warps: Vec<String> =
                    e.warps.iter().map(|(i, p, r)| format!("[{i}, {}, {}, {}, {r}]", p[0], p[1], p[2])).collect();
                println!(
                    "{{\"points\": [{}], \"positions\": [{}], \"warps\": [{}]}}",
                    points.join(", "),
                    pos.join(", "),
                    warps.join(", ")
                );
            }
            "slots" => {
                let mut m = piney_event::vm::EventMng::default();
                for k in 0..16 {
                    m.points[k] = piney_event::vm::EvPoint { floor: -1, block: -1, num: n(1 + k) as i32 };
                    m.positions[k] = piney_event::vm::EvPos {
                        floor: -1,
                        block: -1,
                        num: n(17 + k) as i32,
                        dirc: 0.0,
                        pos: [0.0; 4],
                    };
                }
                for c in &w[33..] {
                    let v: Vec<&str> = c.split(':').collect();
                    let i = |k: usize| v[k].parse::<i32>().unwrap();
                    let fl = |k: usize| f32::from_bits(hex(v[k]));
                    match v[0] {
                        "p" => m.set_event_point(i(1), i(2), i(3)),
                        _ => m.set_event_pos(i(1), i(2), i(3), fl(4), [fl(5), fl(6), fl(7), 0.0]),
                    }
                }
                let points: Vec<String> =
                    m.points.iter().map(|p| format!("[{}, {}, {}]", p.floor, p.block, p.num)).collect();
                let pos: Vec<String> = m
                    .positions
                    .iter()
                    .map(|p| {
                        format!(
                            "[{}, {}, {}, {}, {}, {}, {}, {}]",
                            p.floor,
                            p.block,
                            p.num,
                            p.dirc.to_bits(),
                            p.pos[0].to_bits(),
                            p.pos[1].to_bits(),
                            p.pos[2].to_bits(),
                            p.pos[3].to_bits()
                        )
                    })
                    .collect();
                println!("{{\"points\": [{}], \"positions\": [{}]}}", points.join(", "), pos.join(", "));
            }
            "land" => {
                let a = d.as_mut().unwrap();
                let pos = [hex(w[1]), hex(w[2]), hex(w[3]), 0x3f80_0000];
                let z = a.hits.land(pos, LAND_MASK);
                let h = &a.hits;
                println!(
                    "{{\"z\": {z}, \"num\": {}, \"cp\": {}, \"dist\": {}, \"att\": {}, \"attribute\": {}}}",
                    h.num,
                    list(&h.nearest.cp),
                    h.nearest.dist,
                    h.nearest.att,
                    h.attribute()
                );
            }
            "height" => {
                let a = d.as_ref().unwrap();
                println!("{{\"h\": {}}}", a.hits.ground_height(hex(w[1]), hex(w[2])));
            }
            "goto" => {
                let a = d.as_mut().unwrap();
                let now = [hex(w[1]), hex(w[2]), hex(w[3]), 0x3f80_0000];
                a.level = n(4) as usize;
                scene.floor = n(5) as i32;
                scene.block = n(6) as i32;
                let r = a.goto_next_room(now, &scene, &mut save);
                let at = a.room_at.map_or("null".to_string(), |(f, i)| format!("[{f}, {i}]"));
                println!(
                    "{{\"ret\": {r}, \"position\": {}, \"room_at\": {at}, \"special\": {}, \"enter\": {},                      \"hits\": {}}}",
                    list(&a.position),
                    a.special_room,
                    u8::from(a.room_enter),
                    hit_list(a)
                );
            }
            "enter" => {
                let a = d.as_mut().unwrap();
                let now = [hex(w[1]), hex(w[2]), hex(w[3]), 0x3f80_0000];
                a.level = n(4) as usize;
                scene.floor = n(5) as i32;
                scene.block = n(6) as i32;
                let out = match a.enter(now, &scene, &|_, _| true, &mut save, &mut last_room) {
                    Some(Exit::Scene { area, town, field, dungeon, floor, block }) => {
                        format!("[\"scene\", {area}, {town}, {field}, {dungeon}, {floor}, {block}]")
                    }
                    Some(Exit::Area { area, n }) => format!("[\"area\", {area}, {n}]"),
                    None => "null".to_string(),
                };
                println!("{{\"exit\": {out}, \"level\": {}}}", a.level);
            }
            "ban" => {
                piney_world::dungeon_area::ban_area(&mut save, [n(1) as i32, n(2) as i32, n(3) as i32, n(4) as i32]);
                // The dungeon reads the save's bans when it builds a room.
                if let Some(a) = d.as_mut() {
                    a.bans = piney_world::dungeon_area::bans_of(&save);
                }
                println!("{{}}");
            }
            "bans" => {
                let at = piney_data::save::offset::AREA_BAN;
                let b: Vec<u32> = (0..128).map(|k| u32::from(save.u8(at + k))).collect();
                println!("{{\"bans\": {}}}", list(&b));
            }
            "fog" => {
                let a = d.as_ref().unwrap();
                let f = &a.room_fog;
                let amb = a.lights.ambient;
                println!(
                    "{{\"fog\": [{}, {}, 0, {}, {}], \"ambient\": [{}, {}, {}]}}",
                    f.near.to_bits(),
                    f.far.to_bits(),
                    f.max.to_bits(),
                    f.packed_colour(),
                    amb.x.to_bits(),
                    amb.y.to_bits(),
                    amb.z.to_bits()
                );
            }
            "drawn" => {
                let a = d.as_ref().unwrap();
                let (here, room, doors) = a.shown([hex(w[1]), hex(w[2]), 0, ee::ONE]);
                let here = here.map_or(-1, |h| h as i64);
                println!("{{\"here\": {here}, \"room\": {}, \"doors\": {doors}}}", u8::from(room));
            }
            "start" => {
                let a = d.as_mut().unwrap();
                let pos = [hex(w[1]), hex(w[2]), hex(w[3]), ee::ONE];
                let dirc = [0, 0, hex(w[4]), 0];
                let mode = hex(w[6]) as i8;
                let mut player = Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000);
                player.acts.act = motion::act::IDLE;
                player.acts.act_old = motion::act::IDLE;
                player.acts.anim = motion::act::IDLE as usize;
                player.acts.restraint = false;
                player.acts.transfer_lag = 0;
                player.acts.cloak = ee::ONE;
                a.hits.chars.clear();
                a.hits.center = [pos[0], pos[1]];
                a.hits.hit_enable(&mut player.hit_body);
                let camera = Camera::new(pos, dirc, mode, Scheme::new(hex(w[5]) as i32));
                run = Some(Run { player, camera, rand: Rand(u64::from(hex(w[7]))), mode });
                println!("{{}}");
            }
            "pad" => {
                let a = d.as_mut().unwrap();
                let r = run.as_mut().unwrap();
                let mut pow = [0u8; 12];
                for (k, p) in pow.iter_mut().enumerate() {
                    *p = hex(w[7 + k]) as u8;
                }
                let pad = CamPad {
                    direct: hex(w[1]),
                    push: hex(w[2]),
                    pow_l: hex(w[3]) as u8,
                    dirc_l: hex(w[4]),
                    pow_r: hex(w[5]) as u8,
                    dirc_r: hex(w[6]),
                    pow,
                };
                tasks(&mut r.camera, &mut r.player, &mut a.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                let (p, b, ac, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
                let t = &c.tcam;
                println!(
                    concat!(
                        "{{\"pos\": {}, \"dirc\": {}, \"move\": {}, \"now_speed\": {}, \"speed_rate\": {}, ",
                        "\"move_flag\": {}, \"run_flag\": {}, \"stop_flag\": {}, \"restraint\": {}, ",
                        "\"act\": {}, \"act_old\": {}, \"act_cnt\": {}, \"anm_flag\": {}, \"react_cnt\": {}, ",
                        "\"transfer_lag\": {}, \"walk_run_cnt\": {}, \"cloak\": {}, \"transparency\": {}, ",
                        "\"frame_spd\": {}, \"time\": {}, \"attribute\": {}, \"stop_cnt\": {}, \"drawn\": {}, ",
                        "\"target_count\": {}, \"angle\": {}, ",
                        "\"cam_pos\": {}, \"cam_view\": {}, \"cam_rot\": {}, \"cam_rot2\": {}, \"cam_rot3\": {}, ",
                        "\"cam_dist\": {}, \"cam_deg\": [{}, {}], \"cam_type\": {}, \"cam_reset_flag\": {}, ",
                        "\"cam_reset_dirc\": {}, \"resetting\": {}, \"mem_dirc_z\": {}, \"mode\": {}, ",
                        "\"world_view\": {}, \"world_screen\": {}, \"enter\": {}}}"
                    ),
                    list(&b.pos),
                    list(&b.dirc),
                    list(&b.move_pos),
                    b.now_speed,
                    b.speed_rate,
                    u8::from(b.move_flag),
                    u8::from(b.run_flag),
                    u8::from(ac.stop_flag),
                    u8::from(ac.restraint),
                    ac.act,
                    ac.act_old,
                    ac.act_cnt,
                    ac.anm_flag,
                    ac.react_cnt,
                    ac.transfer_lag,
                    ac.walk_run_cnt,
                    ac.cloak,
                    p.transparency,
                    ac.frame_spd,
                    ac.time,
                    p.hit_attribute,
                    p.stop_cnt,
                    u8::from(p.drawn),
                    b.target_count,
                    list(&p.angle),
                    list(&t.pos),
                    list(&t.view),
                    list(&t.rot),
                    list(&t.rot2),
                    list(&t.rot3),
                    t.dist,
                    t.deg[0],
                    t.deg[1],
                    t.kind,
                    u8::from(t.reset_flag),
                    t.reset_dirc,
                    u8::from(c.resetting),
                    c.mem_dirc_z,
                    r.mode,
                    mat(&c.world_view),
                    mat(&c.world_screen),
                    u8::from(p.enter),
                );
            }
            _ => println!("{{\"error\": \"unknown request {cmd}\"}}"),
        }
    }
}
