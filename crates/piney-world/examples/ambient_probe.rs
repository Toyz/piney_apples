//! Answers `tools/test_field_ambient_rs.py`: a field's weather and ambient
//! pictures (`piney_world::field_ambient`) as `WORLD::Init`, `Generate` and
//! `Draw` make and draw them, one request a line, one JSON line an answer.
//! Numbers are hex, floats their bit patterns. Requests: `field` (`Init` and
//! `Generate`'s ambient parts, with `fieldrand`'s counts), `fires` (type 7's
//! fire places) and `frame` (one `WORLD::Draw`: the pictures in order and the
//! counts of `fieldrand` and `ccRand`); the fields are the harness's.

use std::io::BufRead;

use piney_battle::rand::Genrand;
use piney_data::dungeon::Rng;
use piney_data::field;
use piney_world::ee::{F, ONE, V4};
use piney_world::evarea::FlareCamera;
use piney_world::field_ambient::{Ambient, Cloth, Env, Frame, Model, Op, Statics, Weather};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn v4(w: &[&str]) -> V4 {
    [hex(w[0]), hex(w[1]), hex(w[2]), hex(w[3])]
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

fn op_json(op: &Op) -> String {
    match op {
        Op::Sprite(s) => format!(
            "[\"eff\",{},\"{}\",{},{},{},{},{},{},{}]",
            s.layer,
            s.name,
            list(&s.pos),
            s.pattern,
            s.scale[0],
            s.scale[1],
            s.rotate,
            s.transparency,
            u8::from(s.fog)
        ),
        Op::Smoke { pos, v, s, life, t, fade_in, fade_out } => {
            format!("[\"smoke\",{},{},{},{},{},{},{}]", list(pos), list(v), s, life, t, fade_in, fade_out)
        }
        Op::Mask { layer, alpha, x, y } => {
            format!("[\"mask\",{layer},{alpha},0,0,{x},{y},{},{},128,128]", 0x4400_0000u32, 0x4400_0000u32)
        }
        Op::Model { layer, what, matrix, alpha, shadow } => {
            let m: Vec<u32> = matrix.iter().flatten().copied().collect();
            let shadow = match shadow {
                Some((z, a)) => format!("[{z},{a}]"),
                None => "null".into(),
            };
            let what = match what {
                Model::Bird => "bird".to_string(),
                Model::Tobj => "tobj".to_string(),
                Model::TobjRunner(k) => format!("runner{k}"),
                Model::Haze => "haze".to_string(),
            };
            format!("[\"model\",{layer},\"{what}\",{},{alpha},{shadow}]", list(&m))
        }
        Op::Se(n) => format!("[\"se\",{n}]"),
        Op::Se3d(n, p) => format!("[\"se3d\",{n},{}]", list(p)),
        Op::SeLoopStart => "[\"seloopstart\"]".to_string(),
        Op::SeLoop(p, r) => format!("[\"seloop\",{},{r}]", list(&p[..3])),
        Op::Flash { frames, colour, rect } => format!("[\"flash\",{frames},{colour},{}]", list(rect)),
    }
}

struct State {
    field: field::Field,
    ambient: Ambient,
    rng: Rng,
    statics: Statics,
    cloth: Cloth,
    cc: Genrand,
    cc_count: u32,
}

fn state_json(a: &Ambient, init: u32, made: u32) -> String {
    let snow: Vec<String> = a
        .snow
        .iter()
        .map(|s| {
            format!(
                "[{},\"{}\",{},{},{},{}]",
                s.kind,
                s.eff.name,
                list(&s.pos[..3]),
                list(&s.vel[..3]),
                s.timer,
                s.turn
            )
        })
        .collect();
    let smoke = match &a.smoke {
        Some(s) => {
            let p: Vec<String> = s.puffs.iter().map(|p| format!("[{},{},{},{}]", p[0], p[1], p[2], p[3])).collect();
            format!("{{\"sleep\":{},\"puffs\":[{}]}}", s.sleep, p.join(","))
        }
        None => "null".into(),
    };
    let drops: Vec<String> = a.drops.iter().map(|d| format!("[\"{}\",{}]", d.drop.name, d.pattern)).collect();
    let tobj = match &a.tobj {
        Some(t) => format!(
            "[{},{},{},{},{},{}]",
            t.server,
            t.timer,
            list(&t.pos[..2]),
            t.angle,
            list(&t.rot[..3]),
            u8::from(a.tobj_se_start)
        ),
        None => "null".into(),
    };
    let flies: Vec<String> = a
        .fireflies
        .iter()
        .map(|f| {
            let keys: Vec<String> = f
                .splines
                .iter()
                .map(|s| {
                    let k: Vec<u32> = s.keys.iter().flat_map(|k| [k.val, k.a, k.b, k.c, k.d, k.span]).collect();
                    list(&k)
                })
                .collect();
            format!("[{},{},[{}]]", f.pattern, list(&f.base[..3]), keys.join(","))
        })
        .collect();
    let roamers: Vec<String> = a
        .roamers
        .iter()
        .map(|r| format!("[{},{},{},{},{}]", list(&r.pos), list(&r.vel[..3]), r.life, r.timer, r.pattern))
        .collect();
    let bird = match &a.bird {
        Some(b) => list(&b.home[..3]),
        None => "null".into(),
    };
    format!(
        "{{\"init\":{init},\"gen\":{made},\"snow\":[{}],\"smoke\":{smoke},\"drops\":[{}],\"thunder\":{},\"tobj\":{tobj},\"fireflies\":[{}],\"roamers\":[{}],\"bird\":{bird}}}",
        snow.join(","),
        drops.join(","),
        u8::from(a.thunder.is_some()),
        flies.join(","),
        roamers.join(",")
    )
}

fn main() {
    let tables = &field::INF;
    let mut st: Option<State> = None;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let Some(&cmd) = w.first() else { continue };
        let out = match cmd {
            "field" => {
                let n = |i: usize| hex(w[i]);
                let params = field::Params {
                    seed: n(1),
                    field_type: n(2),
                    weather: n(3),
                    ground: n(4),
                    object: n(5),
                    event: n(6) as i32,
                    protect: n(7) != 0,
                    skip_init: false,
                };
                let weather = Weather { field_type: n(2), b: n(3), hack: n(8) as i32 };
                let server = n(9) as i32;
                let centre = [n(10), n(11), 0, ONE];
                let mut statics = Statics::new();
                let mut rng = Rng::new(params.seed);
                let blank = |x: F, y: F| field::blank_height(x, y);
                let env = Env {
                    player: centre,
                    centre,
                    ofs: [centre[0], centre[1]],
                    height: &blank,
                    eye: centre,
                    eye1: centre,
                    rot: [0; 4],
                    rot2: [0; 4],
                    eye_view: false,
                    odd: false,
                    bounds: piney_world::field_area::BOUNDS,
                };
                let mut ambient = Ambient::init(weather, &env, &mut statics, &mut rng);
                let init = rng.count;
                let fl = field::generate(tables, &params).unwrap();
                let mut rng = fl.rng;
                let heights = |x: F, y: F| fl.get_height(x, y);
                let start = [fl.start_pos[0], fl.start_pos[1], fl.start_pos[2], ONE];
                let entrance = fl.dungeon_pos.map_or([0; 4], |p| [p[0], p[1], p[2], ONE]);
                let env = Env { height: &heights, ..env };
                ambient.generate(server, start, entrance, &env, &mut rng);
                let answer = state_json(&ambient, init, rng.count);
                st = Some(State {
                    field: fl,
                    ambient,
                    rng,
                    statics,
                    cloth: Cloth::default(),
                    cc: Genrand::default(),
                    cc_count: 0,
                });
                answer
            }
            "fires" => {
                let s = st.as_mut().unwrap();
                let k = hex(w[1]) as usize;
                s.ambient.fires =
                    (0..k).map(|i| [hex(w[2 + 3 * i]), hex(w[3 + 3 * i]), hex(w[4 + 3 * i]), 0]).collect();
                "{}".into()
            }
            "frame" => {
                let s = st.as_mut().unwrap();
                let p = |i: usize| v4(&w[1 + 4 * i..5 + 4 * i]);
                let (player, eye, eye1, rot, rot2, view) = (p(0), p(1), p(2), p(3), p(4), p(5));
                let r = |i: usize| hex(w[i]);
                let eye_view = r(25) != 0;
                let odd = r(26) != 0;
                let ofs = [r(27), r(28)];
                let centre = [r(29), r(30), 0, ONE];
                let sun = v4(&w[31..35]);
                let runners = [v4(&w[35..39]), v4(&w[39..43])];
                let fl = &s.field;
                let heights = |x: F, y: F| fl.get_height(x, y);
                let bounds = piney_world::field_area::BOUNDS;
                let env = Env { player, centre, ofs, height: &heights, eye, eye1, rot, rot2, eye_view, odd, bounds };
                let flare_cam = FlareCamera { eye, view, rot: if eye_view { rot2 } else { rot } };
                let at = move |_: &[V4; 4]| runners;
                let frame = Frame { env, flare_cam, sun: Some(sun), runners: &at };
                let cc = &mut s.cc;
                let count = &mut s.cc_count;
                let mut next = || {
                    *count += 1;
                    cc.next_u32()
                };
                let ops = s.ambient.draw(&frame, &mut s.statics, &mut s.cloth, &mut s.rng, &mut next);
                let ops: Vec<String> = ops.iter().map(op_json).collect();
                format!("{{\"ops\":[{}],\"rand\":{},\"cc\":{}}}", ops.join(","), s.rng.count, s.cc_count)
            }
            _ => format!("{{\"error\":\"{cmd}\"}}"),
        };
        println!("{out}");
    }
}
