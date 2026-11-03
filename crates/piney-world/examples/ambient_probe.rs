//! Answers `tools/test_field_ambient_rs.py`: a field's weather, ambient
//! pictures and background (`piney_world::field::def`'s systems) as
//! `WORLD::Init`, `Generate` and `Draw` make and draw them, one request a
//! line, one JSON line an answer. Numbers are hex, floats their bit
//! patterns. Requests: `field` (the field made: `Init`'s and `Generate`'s
//! systems, `fieldrand`'s counts, and what the port found for the game's
//! side: the lens flare's sun, type 7's fires, the lake) and `frame` (one
//! `WORLD::Draw`: the pictures in order, the calls in order, Ω's runners'
//! places, the counts of `fieldrand` and `ccRand`). Arguments: the disc's
//! image, then `volumeNum`.

use std::io::BufRead;
use std::sync::Arc;

use piney_battle::rand::Genrand;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::field;
use piney_data::iso::Iso;
use piney_data::volume::Volume;
use piney_world::ee::{ONE, V4};
use piney_world::field::area::{FieldArea, active_layer};
use piney_world::sky::lensflare::FlareCamera;
use piney_world::systems::bird::Bird;
use piney_world::systems::fireflies::{Fireflies, Fireflies2};
use piney_world::systems::fires::Fires;
use piney_world::systems::flare::LensFlare;
use piney_world::systems::haze::Haze;
use piney_world::systems::lake::Lake;
use piney_world::systems::rain::{Rain, Thunder};
use piney_world::systems::scrolls::Scrolls;
use piney_world::systems::sky::{ClumpName, Sky, SkyLayer};
use piney_world::systems::smoke::Smoke2d;
use piney_world::systems::snow::Snow;
use piney_world::systems::tobj::Tobj;
use piney_world::systems::{BuildCx, Drawn, FieldCx, Kind, MapEvent, MapView, Piece, Request, Systems};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn v4(w: &[&str]) -> V4 {
    [hex(w[0]), hex(w[1]), hex(w[2]), hex(w[3])]
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

fn bits(m: glam::Mat4) -> Vec<u32> {
    m.to_cols_array().iter().map(|f| f.to_bits()).collect()
}

fn flat(m: &[V4; 4]) -> Vec<u32> {
    m.iter().flatten().copied().collect()
}

/// The field's systems' state as `Init` and `Generate` left it, with the
/// counts of `fieldrand` after each.
fn state_json(f: &FieldArea, init: u32, made: u32, hum: bool) -> String {
    let s = f.systems();
    let snow: Vec<String> = s
        .get::<Snow>()
        .map_or(&[][..], |x| &x.flakes)
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
    let smoke = match s.get::<Smoke2d>() {
        Some(s) => {
            let p: Vec<String> = s.puffs.iter().map(|p| format!("[{},{},{},{}]", p[0], p[1], p[2], p[3])).collect();
            format!("{{\"sleep\":{},\"puffs\":[{}]}}", s.sleep, p.join(","))
        }
        None => "null".into(),
    };
    let drops: Vec<String> = s
        .get::<Rain>()
        .map_or(&[][..], |r| &r.drops)
        .iter()
        .map(|d| format!("[\"{}\",{}]", d.drop.name, d.pattern))
        .collect();
    let thunder = s.get::<Thunder>().is_some_and(|t| t.effs.is_some());
    let tobj = match s.get::<Tobj>().and_then(|t| t.tobj.as_ref()) {
        Some(t) => format!(
            "[{},{},{},{},{},{}]",
            t.server,
            t.timer,
            list(&t.pos[..2]),
            t.angle,
            list(&t.rot[..3]),
            u8::from(hum)
        ),
        None => "null".into(),
    };
    let flies: Vec<String> = s
        .get::<Fireflies>()
        .map_or(&[][..], |x| &x.fireflies)
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
    let roamers: Vec<String> = s
        .get::<Fireflies2>()
        .map_or(&[][..], |x| &x.fireflies)
        .iter()
        .map(|r| format!("[{},{},{},{},{}]", list(&r.pos), list(&r.vel[..3]), r.life, r.timer, r.pattern))
        .collect();
    let bird = match s.get::<Bird>().and_then(|b| b.hawk.as_ref()) {
        Some(b) => list(&b.home[..3]),
        None => "null".into(),
    };
    let sun = s.get::<LensFlare>().filter(|l| l.lit).map_or("null".into(), |l| list(&l.sun));
    let fires: Vec<String> = s.get::<Fires>().map_or(&[][..], |x| &x.places).iter().map(|p| list(&p[..3])).collect();
    let lake = s.get::<Lake>().and_then(Lake::at).map_or("null".into(), |p| list(&p[..3]));
    format!(
        "{{\"init\":{init},\"gen\":{made},\"snow\":[{}],\"smoke\":{smoke},\"drops\":[{}],\"thunder\":{},\"tobj\":{tobj},\
         \"fireflies\":[{}],\"roamers\":[{}],\"bird\":{bird},\"sun\":{sun},\"fires\":[{}],\"lake\":{lake}}}",
        snow.join(","),
        drops.join(","),
        u8::from(thunder),
        flies.join(","),
        roamers.join(","),
        fires.join(",")
    )
}

/// A sprite as the harness records `ccEff::Draw`.
fn eff_json(s: &piney_world::sprite::EffSprite) -> String {
    format!(
        "[\"eff\",{},\"{}\",{},{},{},{},{},{},{}]",
        active_layer(s.layer),
        s.eff,
        list(&s.pos),
        s.pattern,
        s.scale[0],
        s.scale[1],
        s.rotate,
        s.transparency,
        u8::from(s.state.fog)
    )
}

fn model_json(layer: u8, what: &str, m: &[u32], alpha: u32, shadow: Option<(u32, u8)>) -> String {
    let shadow = shadow.map_or("null".into(), |(z, a)| format!("[{z},{a}]"));
    format!("[\"model\",{layer},\"{what}\",{},{alpha},{shadow}]", list(m))
}

/// The frame's pictures in the game's order: each piece as the harness
/// records the call that draws it, with the writes of the scrolls before
/// the lake and the background where the game makes them.
fn draws_json(f: &FieldArea) -> Vec<String> {
    let s = f.systems();
    let rows = f.rows();
    let mut requests = f.requests(&rows).into_iter();
    let mut out = Vec::new();
    let (mut lake_uv, mut bg_v) = (false, false);
    for d in f.frame() {
        let Drawn::Piece(kind, piece) = *d else { continue };
        let request = requests.next();
        match (kind, piece) {
            (Kind::Sky, Piece::Clump(k)) => {
                let (sky, scrolls) = (s.get::<Sky>().unwrap(), s.get::<Scrolls>().unwrap());
                if !std::mem::replace(&mut bg_v, true) {
                    out.push(format!("[\"bgv\",{}]", piney_world::ee::vftoi12(scrolls.before[0])));
                }
                let c = sky.clump(k);
                let slot = match c.name {
                    ClumpName::Row(r) => r,
                    ClumpName::Aurora => {
                        out.push(format!("[\"bgu\",{}]", scrolls.uv(0)));
                        5
                    }
                    ClumpName::Named(_) => 9,
                };
                let SkyLayer::On(layer) = c.layer else { continue };
                out.push(format!("[\"bg\",{slot},{layer},{}]", list(&bits(sky.world(k).unwrap()))));
            }
            (Kind::Lake, Piece::Anim(k)) => {
                let l = s.get::<Lake>().unwrap();
                if !std::mem::replace(&mut lake_uv, true) {
                    out.push(format!("[\"lakeuv\",{},{}]", l.shown[0], l.shown[1]));
                }
                out.push(model_json(if k == 1 { 6 } else { 1 }, &format!("lake{k}"), &flat(&l.matrix), ONE, None));
            }
            (Kind::Haze, _) => {
                out.push(model_json(6, "haze", &flat(&s.get::<Haze>().unwrap().matrix), 0x3f33_3333, None));
            }
            (Kind::Tobj, Piece::Anim(k)) => {
                let d = s.get::<Tobj>().unwrap().drawn[k];
                let what = if d.play == 0 { "tobj".into() } else { format!("runner{}", d.play - 1) };
                out.push(model_json(5, &what, &flat(&d.matrix), d.alpha, Some(d.shadow)));
            }
            (Kind::Bird, _) => out.push(model_json(5, "bird", &flat(&s.get::<Bird>().unwrap().matrix), ONE, None)),
            (Kind::Smoke, _) => {
                for &[alpha, x, y] in &s.get::<Smoke2d>().unwrap().masks {
                    let size = 0x4400_0000u32;
                    out.push(format!("[\"mask\",3,{alpha},0,0,{x},{y},{size},{size},128,128]"));
                }
            }
            _ => match request {
                Some(Request::Sprite(e)) => out.push(eff_json(&e)),
                _ => out.push(format!("[\"{kind:?} {piece:?}\"]")),
            },
        }
    }
    out
}

/// A call as the harness records it.
fn call_json(e: &MapEvent) -> String {
    match *e {
        MapEvent::Smoke(s) => format!(
            "[\"smoke\",{},{},{},{},{},{},{}]",
            list(&s.pos),
            list(&s.v),
            s.s,
            s.life,
            s.t,
            s.fade_in,
            s.fade_out
        ),
        MapEvent::Se(n) => format!("[\"se\",{n}]"),
        MapEvent::Se3d { n, pos } => format!("[\"se3d\",{n},{}]", list(&pos)),
        MapEvent::SeLoop(p, r) => format!("[\"seloop\",{},{r}]", list(&p[..3])),
        MapEvent::Flash { frames, colour, rect } => format!("[\"flash\",{frames},{colour},{}]", list(&rect)),
        MapEvent::SeLoopStart => "[\"seloopstart\"]".into(),
        MapEvent::SmokeN { .. } => "[\"smoken\"]".into(),
    }
}

/// `fieldrand`'s count after `Init`'s systems alone, made again from the
/// seed over the field as made.
fn init_count(archive: &Arc<Archive>, volume: Volume, f: &FieldArea, seed: u32, hack: u32, centre: V4) -> u32 {
    let weather =
        piney_world::field::ambient::Weather { field_type: f.def.ty, b: f.field.params.weather, hack: hack as i32 };
    let fcx = FieldCx { def: f.def, weather, server: 0, field: Some(f), centre };
    let mut cx = BuildCx {
        archive,
        tables: volume,
        volume,
        file: &f.bg,
        stem: "",
        crisis: false,
        rng: Rng::new(seed),
        map: None,
        cc: None,
        field: Some(fcx),
        made: Vec::new(),
    };
    Systems::build(f.def.init, &mut cx).expect("Init's systems");
    cx.rng.count
}

fn main() {
    let iso_path = std::env::args().nth(1).expect("the disc's image");
    let mut iso = Iso::open(&iso_path).unwrap();
    let volume = iso.volume().unwrap();
    assert_eq!(Some(volume), std::env::args().nth(2).and_then(|n| Volume::from_number(n.parse().ok()?)));
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut st: Option<(FieldArea, Genrand, u32)> = None;
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
                let (hack, server) = (n(8), n(9) as i32);
                let centre = [n(10), n(11), 0, ONE];
                let mut f = FieldArea::with_world(&archive, volume, params, hack, server, Some(centre)).unwrap();
                f.take(&[0; piney_desktop::SCROLL_STATICS], None);
                let init = init_count(&archive, volume, &f, params.seed, hack, centre);
                let hum = f.take_made().contains(&MapEvent::SeLoopStart);
                let made = f.core.field_rand.map_or(0, |r| r.count);
                let answer = state_json(&f, init, made, hum);
                st = Some((f, Genrand::default(), 0));
                answer
            }
            "frame" => {
                let (f, cc, cc_count) = st.as_mut().unwrap();
                let p = |i: usize| v4(&w[1 + 4 * i..5 + 4 * i]);
                let (player, eye, eye1, rot, rot2, view) = (p(0), p(1), p(2), p(3), p(4), p(5));
                let r = |i: usize| hex(w[i]);
                let eye_view = r(25) != 0;
                let odd = r(26) != 0;
                f.core.hits.center = [r(27), r(28)];
                let kind = if eye_view { piney_world::camera::kind::EYE } else { 0 };
                let cam = piney_world::camera::Cam { pos: eye, view, kind, ..Default::default() };
                let flare = FlareCamera { eye, view, rot: if eye_view { rot2 } else { rot } };
                let v = MapView { eye, player, cam, flare, eye1, rot, rot2, odd, ..MapView::default() };
                let mut count = 0u32;
                let mut counted = || {
                    count += 1;
                    piney_battle::rand::Rng::rand(cc)
                };
                let events = f.step(&v, true, Some(&mut counted));
                *cc_count += count;
                let draws = draws_json(f);
                let calls: Vec<String> = events.iter().map(call_json).collect();
                let runners: Vec<String> = f
                    .systems()
                    .get::<Tobj>()
                    .map_or(&[][..], |t| &t.drawn)
                    .iter()
                    .filter(|d| d.play > 0)
                    .map(|d| list(&d.matrix[3]))
                    .collect();
                format!(
                    "{{\"draws\":[{}],\"calls\":[{}],\"runners\":[{}],\"rand\":{},\"cc\":{cc_count}}}",
                    draws.join(","),
                    calls.join(","),
                    runners.join(","),
                    f.core.field_rand.map_or(0, |r| r.count)
                )
            }
            _ => format!("{{\"error\":\"{cmd}\"}}"),
        };
        println!("{out}");
    }
}
