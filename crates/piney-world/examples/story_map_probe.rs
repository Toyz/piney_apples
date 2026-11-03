//! Answers `tools/test_evarea_rs.py` (`story_map_probe ISO < requests`): area
//! 15's story map as `EVENTAREA02` builds and draws it (`piney_world::story_map::holy_ground`),
//! its collision, the door, the lens flare, and Kite and the camera walking
//! it; and the boss arena (`EVENTAREAB0`, `piney_world::story_map::boss_arena`). One JSON
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
use piney_world::chara::motion::PLAYER_ANIM_TBL;
use piney_world::chara::player::Player;
use piney_world::draw::Fogging;
use piney_world::ee::{self, V4};
use piney_world::hit;
use piney_world::sky::lensflare::FlareCamera;
use piney_world::story_map::boss_arena::FIREFLIES;
use piney_world::story_map::holy_ground::{self, HOLY_GROUND};
use piney_world::story_map::{Enter, MapEnv, StoryArea, StoryMapDef};
use piney_world::systems::MapView;
use piney_world::systems::anim::Anim;
use piney_world::systems::fireflies::Fireflies2;
use piney_world::systems::scrolls::Scrolls;
use piney_world::systems::sky::Sky;
use piney_world::systems::statics::{StaticModels, StaticObjects};
use piney_world::systems::{Drawn, Kind, Piece, Request};
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

/// `EVENTAREA02`'s `bg[0..4]` by block: the slots the harness numbers.
const SLOTS: [[&str; 5]; 2] =
    [["CMP_se1_2cl2_1", "CMP_se1_2cl1_1", "CMP_se1_2wa1_1", "CMP_se1_2bg1_1", ""], ["CMP_se1_2bg1_2", "", "", "", ""]];

/// The map `GO(1)` makes for `field`, `fieldrand` seeded `seed`.
fn make(archive: &Arc<Archive>, volume: piney_data::volume::Volume, field: i32, seed: u32) -> StoryArea {
    let def = StoryMapDef::of(field, volume).unwrap();
    let save = piney_data::save::SaveData::default();
    let mut cc = piney_battle::rand::Genrand::default();
    let mut env = MapEnv {
        archive,
        volume,
        field,
        area_prev: 1,
        field_prev: -1,
        server: 0,
        save: &save,
        event_start: [0, 0, 0, ee::ONE],
        field_rand: piney_data::dungeon::Rng::new(seed),
        move_tex: [[0; 2]; piney_desktop::MOVE_TEX_ROWS],
        scroll_statics: [0; piney_desktop::SCROLL_STATICS],
        def_se: 0,
        kept: None,
        cc: &mut cc,
    };
    StoryArea::new(def, &mut env).unwrap()
}

/// The block's light group's objects, by name, those the file has.
fn lights(a: &StoryArea) -> String {
    let block = &a.def.blocks[a.block.max(0) as usize];
    let file = &a.core.file;
    let names = block.lights.map_or(&[][..], |l| l.table).iter().map(|n| match a.def.stage {
        Some(from) => n.replace(from, &file.stem),
        None => n.to_string(),
    });
    list(names.filter(|n| file.ccs.find_object(n).is_some()).map(|n| format!("\"{n}\"")))
}

fn fog_of(a: &StoryArea) -> String {
    let f = a.def.blocks[a.block.max(0) as usize].fog.unwrap();
    format!(
        "[{}, {}, {}, {}, {}]",
        f.near.to_bits(),
        f.far.to_bits(),
        f.near_rate.to_bits(),
        f.far_rate.to_bits(),
        u32::from(f.colour[0]) | u32::from(f.colour[1]) << 8 | u32::from(f.colour[2]) << 16
    )
}

fn models_of(a: &StoryArea) -> String {
    let Some(models) = a.systems().get::<StaticModels>() else { return list(Vec::<String>::new()) };
    let name = |o: u32| format!("\"{}\"", models.file().ccs.object_name(o).unwrap_or("?"));
    list(models.iter().map(|m| format!("[{}, {}, {}, {}]", m.row, pass(m.pass), name(m.model), list(m.pos))))
}

fn tex(a: &StoryArea) -> String {
    let s = a.systems().get::<Scrolls>().unwrap();
    list((0..s.v.len() / 2).map(|k| list([s.uv(2 * k), s.uv(2 * k + 1)])))
}

/// The map's state as the test compares it.
fn state(a: &StoryArea) -> String {
    let name = |o: u32| format!("\"{}\"", a.core.file.ccs.object_name(o).unwrap_or("?"));
    let objects = list(a.systems().get::<StaticObjects>().into_iter().flat_map(|o| o.iter()).map(|o| {
        format!(
            "[{}, {}, {}, {}, {}, {}]",
            o.row,
            name(a.core.file.anims[o.play.anim].object),
            mat(&o.root),
            list(o.pos),
            o.clip,
            o.play.time
        )
    }));
    let sky = a.systems().get::<Sky>();
    let drawn = |n: &str| sky.is_some_and(|s| (0..8).any(|k| s.made(k) && s.name(k) == n));
    let bg = list(
        SLOTS[usize::from(a.block != 0)]
            .iter()
            .map(|&n| if !n.is_empty() && drawn(n) { format!("\"{n}\"") } else { "null".to_string() }),
    );
    let hits = list(a.core.hits.models.iter().map(|h| name(h.object)));
    let rev = a.systems().get::<Anim>().map_or(-1, |r| r.play.time as i64);
    format!(
        "{{\"block\": {}, \"models\": {}, \"objects\": {objects}, \"bg\": {bg}, \"lights\": {}, \"fog\": {}, \"start\": {}, \"hits\": {hits}, \"rev\": {rev}, \"tex\": {}}}",
        a.block,
        models_of(a),
        lights(a),
        fog_of(a),
        list(a.core.start),
        tex(a),
    )
}

/// The arena's state as the test compares it.
fn arena_state(a: &StoryArea) -> String {
    let name = |o: u32| format!("\"{}\"", a.core.file.ccs.object_name(o).unwrap_or("?"));
    let sky = a.systems().get::<Sky>().unwrap();
    let stem = &a.core.file.stem;
    let bg = list((0..3).map(|k| format!("\"{}\"", sky.name(k).replace("se1_5", stem))));
    let hits = list(a.core.hits.models.iter().map(|h| name(h.object)));
    format!(
        "{{\"models\": {}, \"bg\": {bg}, \"lights\": {}, \"fog\": {}, \"start\": {}, \"hits\": {hits}}}",
        models_of(a),
        lights(a),
        fog_of(a),
        list(a.core.start)
    )
}

/// The last step's pieces with their draws.
fn pieces(a: &StoryArea) -> Vec<(Kind, Piece, Option<i16>, bool)> {
    let rows = a.rows();
    let mut reqs = a.requests(&rows).into_iter();
    let mut out = Vec::new();
    for d in a.frame() {
        let Drawn::Piece(kind, piece) = *d else { continue };
        let (layer, fog) = match reqs.next() {
            Some(Request::Models { layer, fog, .. }) => (Some(layer), !matches!(fog, Fogging::None)),
            Some(Request::Anim { layer, fog, .. }) => (Some(layer), !matches!(fog, Fogging::None)),
            Some(Request::Sprite(s)) => (Some(s.layer), s.state.fog),
            Some(Request::Posed { layer, .. } | Request::Masks { layer, .. }) => (Some(layer), false),
            None => (None, false),
        };
        out.push((kind, piece, layer, fog));
    }
    out
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
    let mut area: Option<StoryArea> = None;
    let mut arena: Option<StoryArea> = None;
    let mut run: Option<Run> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| -> V4 { [n(i), n(i + 1), n(i + 2), n(i + 3)] };
        match w.first().copied() {
            Some("new") => {
                let a = make(&archive, volume, holy_ground::AREA, 0);
                assert!(a.def == &HOLY_GROUND);
                println!("{}", state(&a));
                area = Some(a);
            }
            Some("block") => {
                let a = area.as_mut().unwrap();
                a.change_block(n(1) as i32, 1).unwrap();
                println!("{}", state(a));
            }
            Some("enter") => {
                let a = area.as_mut().unwrap();
                let Enter::Block(to) = a.enter(n(1) as i32, 1).unwrap() else { panic!("no door") };
                println!("{{\"to\": {to}, \"state\": {}}}", state(a));
            }
            Some("arena") => {
                let a = make(&archive, volume, n(1) as i32, n(2));
                assert_eq!(a.systems().get::<Fireflies2>().unwrap().fireflies.len(), FIREFLIES);
                println!("{}", arena_state(&a));
                arena = Some(a);
            }
            Some("arenadraw") => {
                let a = arena.as_mut().unwrap();
                let v = MapView { player: [n(1), n(2), n(3), ee::ONE], ..MapView::default() };
                a.step(&v, true, None);
                let mut last = false;
                let mut out = Vec::new();
                for (kind, piece, layer, _) in pieces(a) {
                    let layer = layer.unwrap_or(0);
                    out.push(match (kind, piece) {
                        (Kind::Sky, Piece::Clump(k)) => format!("[\"bg\", {k}, {layer}]"),
                        (Kind::Models, Piece::Model(k) | Piece::ModelOn(k, _)) => format!("[\"model\", {k}, {layer}]"),
                        (Kind::Firefly2, Piece::Fly(d)) => {
                            let (pos, pattern, tr, scale) = (list(d.pos), d.pattern, d.transparency, d.scale);
                            format!("[\"eff\", {pos}, {pattern}, {tr}, {scale}, {layer}]")
                        }
                        // The clip is one ccAnm::Draw, whatever its pieces.
                        (Kind::LastStage, Piece::Item(_)) if last => continue,
                        (Kind::LastStage, Piece::Item(_)) => {
                            last = true;
                            format!("[\"last_anm\", {layer}]")
                        }
                        (Kind::LastStage, _) => format!("[\"last_clump\", {layer}]"),
                        (k, p) => format!("[\"{k:?} {p:?}\"]"),
                    });
                }
                let models = a.systems().get::<StaticModels>();
                let bob = list(models.into_iter().flat_map(|m| m.iter()).skip(6).take(3).map(|m| list(m.pos)));
                let v = ee::vftoi12(a.systems().get::<Scrolls>().unwrap().before[0]);
                println!(
                    "{{\"pieces\": {}, \"bob\": {bob}, \"v\": {v}, \"seed\": {}}}",
                    list(out),
                    a.core.field_rand.map_or(0, |r| r.seed)
                );
            }
            Some("arenaswitch") => {
                arena.as_mut().unwrap().signal(piney_world::systems::Signal::SwitchLayer);
                println!("{{}}");
            }
            Some("revroot") => {
                println!("{}", mat(&piney_world::town::pos_rot_zyx(holy_ground::REV_POS, holy_ground::REV_ROT)))
            }
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
                let flare = FlareCamera { eye, view: v4(9), rot: v4(13) };
                let v = MapView { eye, player, flare, puppet_show: n(8) != 0, ..MapView::default() };
                a.step(&v, true, None);
                let sky = a.systems().get::<Sky>();
                let objects = a.systems().get::<StaticObjects>();
                let anim = a.systems().get::<Anim>();
                let slot = |k: usize| {
                    let name = sky.map_or("", |s| s.name(k));
                    SLOTS[usize::from(a.block != 0)].iter().position(|&n| n == name).unwrap_or(k)
                };
                let out = list(pieces(a).into_iter().map(|(kind, piece, layer, fog)| {
                    let layer = layer.unwrap_or(0);
                    match (kind, piece) {
                        (Kind::Sky, Piece::Clump(k)) => format!("[\"bg\", {}, {layer}]", slot(k)),
                        (Kind::Models, Piece::Model(k) | Piece::ModelNoFog(k)) => {
                            format!("[\"model\", {k}, {layer}, {}]", u8::from(fog))
                        }
                        (Kind::Objects, Piece::Object(k)) => {
                            let time = objects.and_then(|o| o.object(k)).map_or(0, |o| o.0);
                            format!("[\"obj\", {k}, {layer}, {time}]")
                        }
                        (Kind::Anim, _) => format!("[\"rev\", {layer}, {}]", anim.map_or(0, |r| r.play.time)),
                        (Kind::LensFlare, Piece::Sprite { k, pos, .. }) => {
                            format!("[\"flare\", {k}, {layer}, {}]", list(pos))
                        }
                        (k, p) => format!("[\"{k:?} {p:?}\"]"),
                    }
                }));
                println!("{{\"pieces\": {out}, \"tex\": {}}}", tex(a));
            }
            Some("land") => {
                let a = area.as_mut().unwrap();
                let z = a.core.hits.land([n(1), n(2), n(3), ee::ONE], hit::LAND_MASK);
                let h = &a.core.hits;
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
                a.core.hits.center = [pos[0], pos[1]];
                a.core.hits.chars.clear();
                // The disc's square roots (sqrt.s from Outbreak on), as the
                // world probe's player and camera take them.
                run = Some(Run {
                    player: Player::new(volume, pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000),
                    camera: Camera::new(volume, pos, dirc, mode, scheme),
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
                tasks(&mut r.camera, &mut r.player, &mut a.core.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                if r.player.events.contains(&piney_world::chara::motion::ActEvent::Arrived) {
                    a.core.hits.hit_enable(&mut r.player.hit_body);
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
                        "\"arms\": {:?}, \"target_count\": {}, \"angle\": {}, ",
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
                    // ArmsEffect / ClearArmsEffect from ccPlayer::Main, as
                    // the world harness reports them (worklog 378).
                    match p.arms {
                        Some(piney_world::chara::arms::ArmsCall::Effect) => "effect",
                        Some(piney_world::chara::arms::ArmsCall::Clear) => "clear",
                        None => "none",
                    },
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
                    list(a.core.hits.center),
                    u8::from(p.attribute_qualified)
                );
            }
            _ => println!("{{\"error\": \"{line}\"}}"),
        }
    }
}
