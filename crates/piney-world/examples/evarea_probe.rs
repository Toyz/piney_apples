//! Answers `tools/test_evarea_rs.py` (`evarea_probe ISO < requests`): area
//! 15's story map as `EVENTAREA02` builds and draws it (`piney_world::evarea`),
//! its collision, the door, the lens flare, and Kite and the camera walking
//! it; and the boss arena (`EVENTAREAB0`, `piney_world::evarea_b0`). One JSON
//! line a request. Requests: `new`, `block`, `enter`, `revroot`, `charpos`,
//! `draw`, `land`, `start`, `pad`, `arena`, `arenadraw`, `arenaswitch`;
//! numbers hex, floats their bits, the fields the harness's.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::statics::DrawPass;
use piney_desktop::assets::SceneFile;
use piney_world::camera::{CamPad, Camera, Scheme};
use piney_world::ee::{self, V4};
use piney_world::evarea::{EventArea, FlareCamera, Piece};
use piney_world::evarea_b0::{self, Arena};
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

fn pass(p: DrawPass) -> u32 {
    match p {
        DrawPass::Other => 0,
        DrawPass::Floor => 1,
        DrawPass::Obj => 2,
        DrawPass::Obj2 => 3,
    }
}

/// The map's state as the test compares it.
fn state(a: &EventArea) -> String {
    let name = |o: u32| format!("\"{}\"", a.file.ccs.object_name(o).unwrap_or("?"));
    let models =
        list(a.models.iter().map(|m| format!("[{}, {}, {}, {}]", m.row, pass(m.pass), name(m.model), list(m.pos))));
    let objects = list(a.objects.iter().map(|o| {
        format!(
            "[{}, {}, {}, {}, {}, {}]",
            o.row,
            name(a.file.anims[o.play.anim].object),
            mat(&o.root),
            list(o.pos),
            o.clip,
            o.play.time
        )
    }));
    let bg = list(a.bg.iter().map(|b| b.as_ref().map_or("null".to_string(), |(c, _)| name(*c))));
    let lights = list(a.light_objects.iter().map(|&o| name(o)));
    let f = &a.fog;
    let fog = format!(
        "[{}, {}, {}, {}, {}]",
        f.near.to_bits(),
        f.far.to_bits(),
        f.near_rate.to_bits(),
        f.far_rate.to_bits(),
        u32::from(f.colour[0]) | u32::from(f.colour[1]) << 8 | u32::from(f.colour[2]) << 16
    );
    let hits = list(a.hits.models.iter().map(|h| name(h.object)));
    let tex = list(a.move_tex.iter().map(|t| list(t.offsets())));
    format!(
        "{{\"block\": {}, \"models\": {models}, \"objects\": {objects}, \"bg\": {bg}, \"lights\": {lights}, \"fog\": {fog}, \"start\": {}, \"hits\": {hits}, \"rev\": {}, \"tex\": {tex}}}",
        a.block,
        list(a.start),
        a.rev.as_ref().map_or(-1, |r| r.time as i64),
    )
}

/// The arena's state as the test compares it.
fn arena_state(a: &Arena) -> String {
    let name = |o: u32| format!("\"{}\"", a.file.ccs.object_name(o).unwrap_or("?"));
    let models =
        list(a.models.iter().map(|m| format!("[{}, {}, {}, {}]", m.row, pass(m.pass), name(m.model), list(m.pos))));
    let bg = list(a.bg.iter().map(|b| b.as_ref().map_or("null".to_string(), |(c, _)| name(*c))));
    let lights = list(a.light_objects.iter().map(|&o| name(o)));
    let f = &evarea_b0::FOG;
    let fog = format!(
        "[{}, {}, {}, {}, {}]",
        f.near.to_bits(),
        f.far.to_bits(),
        f.near_rate.to_bits(),
        f.far_rate.to_bits(),
        u32::from(f.colour[0]) | u32::from(f.colour[1]) << 8 | u32::from(f.colour[2]) << 16
    );
    let hits = list(a.hits.models.iter().map(|h| name(h.object)));
    format!(
        "{{\"models\": {models}, \"bg\": {bg}, \"lights\": {lights}, \"fog\": {fog}, \"start\": {}, \"hits\": {hits}}}",
        list(a.start)
    )
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
    let volume = iso.volume().unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
    let mut area: Option<EventArea> = None;
    let mut arena: Option<Arena> = None;
    let mut run: Option<Run> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| -> V4 { [n(i), n(i + 1), n(i + 2), n(i + 3)] };
        match w.first().copied() {
            Some("new") => {
                let a = EventArea::new(&archive, volume, 0).unwrap();
                println!("{}", state(&a));
                area = Some(a);
            }
            Some("block") => {
                let a = area.as_mut().unwrap();
                a.change_block(n(1) as i32).unwrap();
                println!("{}", state(a));
            }
            Some("enter") => {
                let a = area.as_mut().unwrap();
                let to = a.enter(n(1) as i32).unwrap();
                println!("{{\"to\": {to}, \"state\": {}}}", state(a));
            }
            Some("arena") => {
                let a = Arena::new_seeded(&archive, volume, n(1) as i32, 0, n(2)).unwrap();
                println!("{}", arena_state(&a));
                arena = Some(a);
            }
            Some("arenadraw") => {
                let a = arena.as_mut().unwrap();
                let pieces = a.select([n(1), n(2), n(3), ee::ONE]);
                let out = list(pieces.iter().map(|p| match p {
                    evarea_b0::Piece::Bg { k, layer } => format!("[\"bg\", {k}, {layer}]"),
                    evarea_b0::Piece::Model { k, layer } => format!("[\"model\", {k}, {layer}]"),
                    evarea_b0::Piece::Firefly { draw: d, layer } => {
                        format!("[\"eff\", {}, {}, {}, {}, {layer}]", list(d.pos), d.pattern, d.transparency, d.scale)
                    }
                }));
                let bob = list(a.models[6..9].iter().map(|m| list(m.pos)));
                let v = (ee::to_int(ee::mul(a.scroll_shown, 0x4580_0000)) & 0xffff) as u16;
                println!("{{\"pieces\": {out}, \"bob\": {bob}, \"v\": {v}, \"seed\": {}}}", a.rng.seed);
            }
            Some("arenaswitch") => {
                arena.as_mut().unwrap().switch_layer();
                println!("{{}}");
            }
            Some("revroot") => println!("{}", mat(&piney_world::evarea::rev_root())),
            Some("charpos") => {
                let a = area.as_ref().unwrap();
                let (p, dirc, m) = a.start_positions();
                let members = list(m.iter().map(|v| list([v[0], v[1], v[2], dirc])));
                println!("[{}, {}, {}, {dirc}, {members}]", p[0], p[1], p[2]);
            }
            Some("draw") => {
                let a = area.as_mut().unwrap();
                let eye = v4(1);
                let player = [n(5), n(6), n(7), ee::ONE];
                let puppet = n(8) != 0;
                let cam = FlareCamera { eye, view: v4(9), rot: v4(13) };
                let pieces = a.select(eye, player, &cam, puppet);
                let out = list(pieces.iter().map(|p| match p {
                    Piece::Bg { k, layer } => format!("[\"bg\", {k}, {layer}]"),
                    Piece::Model { k, layer, fog } => format!("[\"model\", {k}, {layer}, {}]", u8::from(*fog)),
                    Piece::Object { k, layer, time } => format!("[\"obj\", {k}, {layer}, {time}]"),
                    Piece::Rev { layer, time } => format!("[\"rev\", {layer}, {time}]"),
                    Piece::Flare { k, layer, pos } => format!("[\"flare\", {k}, {layer}, {}]", list(*pos)),
                }));
                let tex = list(a.move_tex.iter().map(|t| list(t.offsets())));
                println!("{{\"pieces\": {out}, \"tex\": {tex}}}");
            }
            Some("land") => {
                let a = area.as_mut().unwrap();
                let z = a.hits.land([n(1), n(2), n(3), ee::ONE], hit::LAND_MASK);
                let h = &a.hits;
                println!(
                    "{{\"z\": {z}, \"num\": {}, \"att\": {}, \"cp\": {}, \"attribute\": {}}}",
                    h.num,
                    h.nearest.att,
                    list(h.nearest.cp),
                    h.attribute()
                );
            }
            Some("start") => {
                let a = area.as_mut().unwrap();
                let pos = [n(1), n(2), n(3), ee::ONE];
                let dirc = [0, 0, n(4), 0];
                let scheme = Scheme::new(n(5) as i32);
                let mode = n(6) as i8;
                a.set_center(pos[0], pos[1]);
                a.hits.chars.clear();
                run = Some(Run {
                    player: Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000),
                    camera: Camera::new(pos, dirc, mode, scheme),
                    rand: Rand(u64::from(n(7))),
                    mode,
                });
                println!("{{}}");
            }
            Some("pad") => {
                let a = area.as_mut().unwrap();
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
                tasks(&mut r.camera, &mut r.player, &mut a.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                if r.player.events.contains(&piney_world::motion::ActEvent::Arrived) {
                    a.hits.hit_enable(&mut r.player.hit_body);
                }
                let (p, b, s, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
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
                        "\"world_view\": {}, \"world_screen\": {}, \"enter\": {}, \"center\": {}, ",
                        "\"qualified\": {}}}"
                    ),
                    list(b.pos),
                    list(b.dirc),
                    list(b.move_pos),
                    b.now_speed,
                    b.speed_rate,
                    u8::from(b.move_flag),
                    u8::from(b.run_flag),
                    u8::from(s.stop_flag),
                    u8::from(s.restraint),
                    s.act,
                    s.act_old,
                    s.act_cnt,
                    s.anm_flag,
                    s.react_cnt,
                    s.transfer_lag,
                    s.walk_run_cnt,
                    s.cloak,
                    p.transparency,
                    s.frame_spd,
                    s.time,
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
                    list(a.hits.center),
                    u8::from(p.attribute_qualified)
                );
            }
            _ => println!("{{\"error\": \"{line}\"}}"),
        }
    }
}
