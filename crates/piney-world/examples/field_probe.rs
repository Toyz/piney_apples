//! Answers `tools/test_field_rt.py`: a field as `WORLD_MAN::GO(1)` builds
//! it (`piney_world::field_area`), its collision, its object passes and
//! Kite and the camera walking it, one request a line, one JSON line an
//! answer, so the test can run the same through the game's code in eemu.
//!
//! ```text
//! cargo build --release -p piney-world --example field_probe
//! field_probe ISO < requests
//! ```
//!
//! Numbers are hex; floats travel as their bit patterns. Requests:
//! - `new SEED TYPE WEATHER GROUND OBJECT EVENT`: the field; answers its
//!   objects (kind, FOBJECT2, chip, wp, anm, hit indices), its hit models
//!   (HIT_ object and parent model ids), the height map and `check3`,
//!   `defSE`.
//! - `charpos AREAPREV TYPE SX SY SZ DX DY DZ`: where
//!   `WORLD_MAN::SetCharPosition` puts the leader in a field of type TYPE
//!   come from area AREAPREV, `fieldStartPos` S, `dungeonPos[0]` D.
//!   Answers the leader's place and facing, then `StartPos[1..3]` as
//!   `ccGetStartPositions` asks for them.
//! - `dgpos X Y Z W`: `SetCharPosition` in a dungeon (`WORLD_MAN.area`
//!   2) with `WORLD_MAN.position` (X, Y, Z) facing W: `StartPos[1..3]`.
//! - `place K`: object K's hits set at its `wp` and enabled; answers the
//!   list.
//! - `clear`: the hit list emptied.
//! - `land X Y Z`: `ccLandHitCheck(pos, 0x20000001)`: z, the result count,
//!   the nearest result's attribute and point.
//! - `hitcheck X Y Z MX MY NOWSPEED RUN`: `ccSpcChar::HitCheck` for Kite
//!   (width 45) at (X, Y, Z) moving (MX, MY): the result and the move.
//! - `step X Y Z`: the draw's object passes for a player at (X, Y, Z):
//!   each object's place, fade, whether drawn and its anm's time, and the
//!   hit list with each hit's translation.
//! - `start X Y Z DIRCZ SCHEME MODE SEED`: Kite arriving at (X, Y, Z) in
//!   the field last made, the camera behind him, `WORLD_MAN::SetCenter`.
//! - `pad DIRECT PUSH POWL DIRCL POWR DIRCR POW0 .. POW11`: one frame -
//!   `cameraMain`, `ccPlayer::Main`, then the object passes.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::field;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_world::camera::{CamPad, Camera, Scheme};
use piney_world::ee::{self, V4};
use piney_world::field_area::{DEF_SE, FieldArea};
use piney_world::hit;
use piney_world::motion::PLAYER_ANIM_TBL;
use piney_world::player::Player;
use piney_world::{Rand, tasks};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn mat(m: &[[u32; 4]; 4]) -> String {
    list(m.iter().flatten().copied())
}

/// The hit list: each hit's index and its matrix's translation.
fn hit_list(f: &FieldArea) -> String {
    list(f.list.iter().map(|&h| {
        let t = f.model_hits[h].rm[3];
        format!("[{h}, {}]", list(t))
    }))
}

struct Run {
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
    let mut area: Option<FieldArea> = None;
    let mut run: Option<Run> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        match w.first().copied() {
            Some("new") => {
                let params = field::Params {
                    seed: n(1),
                    field_type: n(2),
                    weather: n(3),
                    ground: n(4),
                    object: n(5),
                    event: n(6) as i32,
                    protect: false,
                    skip_init: false,
                };
                let f = FieldArea::new(&archive, params, DEF_SE[params.field_type as usize]).unwrap();
                let objects = list(f.objects.iter().map(|o| {
                    format!(
                        "[{}, {}, {}, {}, {}, {}, {}]",
                        o.kind.number(),
                        u8::from(o.two),
                        o.chip[0],
                        o.chip[1],
                        list(o.wp),
                        u8::from(o.play.is_some()),
                        list(o.hits.iter().copied())
                    )
                }));
                let hits = list(f.model_hits.iter().map(|h| format!("[{}, {}]", h.object, h.parent)));
                println!(
                    "{{\"objects\": {objects}, \"hits\": {hits}, \"map\": {}, \"check3\": {}, \"def_se\": {}, \"list\": {}}}",
                    list(f.field.map.iter().copied()),
                    list(f.field.check3.iter().copied()),
                    f.hits.def_se,
                    hit_list(&f)
                );
                area = Some(f);
            }
            Some("charpos") => {
                let start = [n(3), n(4), n(5), ee::ONE];
                let dungeon = [n(6), n(7), n(8), ee::ONE];
                let (p, rot) = piney_world::field_world::field_start(n(1) as i32, n(2), start, Some(dungeon));
                let m = piney_world::field_world::party_starts(1, p, rot);
                let members = list(m.iter().map(|v| list([v[0], v[1], v[2], rot])));
                println!("[{}, {}, {}, {rot}, {members}]", p[0], p[1], p[2]);
            }
            Some("dgpos") => {
                let m = piney_world::field_world::party_starts(2, [n(1), n(2), n(3), ee::ONE], n(4));
                println!("{}", list(m.iter().map(|v| list([v[0], v[1], v[2], n(4)]))));
            }
            Some("place") => {
                let f = area.as_mut().unwrap();
                f.place(n(1) as usize);
                println!("{{\"list\": {}}}", hit_list(f));
            }
            Some("clear") => {
                let f = area.as_mut().unwrap();
                f.list.clear();
                f.hits.models.clear();
                println!("{{}}");
            }
            Some("land") => {
                let f = area.as_mut().unwrap();
                let z = f.hits.land([n(1), n(2), n(3), ee::ONE], hit::LAND_MASK);
                let h = &f.hits;
                println!(
                    "{{\"z\": {z}, \"num\": {}, \"att\": {}, \"cp\": {}, \"attribute\": {}}}",
                    h.num,
                    h.nearest.att,
                    list(h.nearest.cp),
                    h.attribute()
                );
            }
            Some("hitcheck") => {
                let f = area.as_mut().unwrap();
                let pos: V4 = [n(1), n(2), n(3), ee::ONE];
                let mv: V4 = [n(4), n(5), 0, ee::ONE];
                let mut body = hit::Body { pos, radius: 0x4234_0000, ..hit::Body::default() };
                let (r, out) = f.hits.hit_check(&mut body, pos, mv, 0x4234_0000, n(6), n(7) != 0);
                println!("{{\"r\": {r}, \"move\": {}}}", list(out));
            }
            Some("step") => {
                let f = area.as_mut().unwrap();
                let player: V4 = [n(1), n(2), n(3), ee::ONE];
                f.ofs = [player[0], player[1]];
                let drawn = f.step_objects(player);
                let objs = list(f.objects.iter().map(|o| {
                    format!(
                        "[{}, {}, {}, {}, {}]",
                        list(o.pos),
                        o.dist,
                        o.alpha,
                        u8::from(o.disp),
                        o.play.as_ref().map_or(0, |p| p.time)
                    )
                }));
                println!(
                    "{{\"objects\": {objs}, \"drawn\": {}, \"list\": {}}}",
                    list(drawn.iter().map(|d| d.0)),
                    hit_list(f)
                );
            }
            Some("start") => {
                let f = area.as_mut().unwrap();
                let pos = [n(1), n(2), n(3), ee::ONE];
                let dirc = [0, 0, n(4), 0];
                let scheme = Scheme::new(n(5) as i32);
                let mode = n(6) as i8;
                f.set_center(pos[0], pos[1]);
                f.hits.center = [pos[0], pos[1]];
                run = Some(Run {
                    player: Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000),
                    camera: Camera::new(pos, dirc, mode, scheme),
                    rand: Rand(u64::from(n(7))),
                    mode,
                });
                println!("{{}}");
            }
            Some("pad") => {
                let f = area.as_mut().unwrap();
                let r = run.as_mut().unwrap();
                let mut pow = [0u8; 12];
                for (k, p) in pow.iter_mut().enumerate() {
                    *p = n(7 + k) as u8;
                }
                let pad = CamPad {
                    direct: n(1),
                    push: n(2),
                    pow_l: n(3) as u8,
                    dirc_l: n(4),
                    pow_r: n(5) as u8,
                    dirc_r: n(6),
                    pow,
                };
                tasks(&mut r.camera, &mut r.player, &mut f.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                if r.player.events.contains(&piney_world::motion::ActEvent::Arrived) {
                    f.hits.hit_enable(&mut r.player.hit_body);
                }
                f.ofs = f.hits.center;
                let player = r.player.body.pos;
                f.step_objects(player);
                let (p, b, a, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
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
                        "\"world_view\": {}, \"world_screen\": {}, \"enter\": {}, \"center\": {}, \"list\": {}, ",
                        "\"qualified\": {}}}"
                    ),
                    list(b.pos),
                    list(b.dirc),
                    list(b.move_pos),
                    b.now_speed,
                    b.speed_rate,
                    u8::from(b.move_flag),
                    u8::from(b.run_flag),
                    u8::from(a.stop_flag),
                    u8::from(a.restraint),
                    a.act,
                    a.act_old,
                    a.act_cnt,
                    a.anm_flag,
                    a.react_cnt,
                    a.transfer_lag,
                    a.walk_run_cnt,
                    a.cloak,
                    p.transparency,
                    a.frame_spd,
                    a.time,
                    p.hit_attribute,
                    p.stop_cnt,
                    u8::from(p.drawn),
                    b.target_count,
                    list(p.angle),
                    list(t.pos),
                    list(t.view),
                    list(t.rot),
                    list(t.rot2),
                    list(t.rot3),
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
                    list(f.hits.center),
                    hit_list(f),
                    u8::from(p.attribute_qualified)
                );
            }
            _ => println!("{{\"error\": \"{line}\"}}"),
        }
    }
}
