//! Answers `tools/test_town02_rs.py` (`dun_loireag_probe ISO < requests`): Dun
//! Loireag as `ROOTTOWN02` builds and draws it (`piney_world::town::dun_loireag`,
//! `cloud`, `lensflare`), one JSON line a request. Requests: `new CRISIS SEED`
//! (the town's state), `draw` (one `ROOTTOWN02::Draw`: the pieces, then the
//! clouds, scrolls and `fieldrand`), `watermodel TOWN` and `wateruv`
//! (`waterUVModifi2`'s coordinates); numbers hex, floats their bits.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::iso::Iso;
use piney_data::statics::DrawPass;
use piney_world::camera::Cam;
use piney_world::ee::{self, V4};
use piney_world::story_map::holy_ground::FlareCamera;
use piney_world::systems::clouds::Clouds;
use piney_world::systems::flare::LensFlare;
use piney_world::systems::scrolls::Scrolls;
use piney_world::systems::sky::Sky;
use piney_world::systems::statics::{StaticModels, StaticObjects};
use piney_world::systems::water::Water;
use piney_world::systems::{Drawn, Kind, Piece};
use piney_world::town::dun_loireag::{CLOUD_SKY, SUN};
use piney_world::town::{Town, TownView, water_st};

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

/// The town as the constructor left it.
fn state(town: &Town) -> String {
    let t = town;
    let s = t.systems();
    let (models_, objects_) = (s.get::<StaticModels>().unwrap(), s.get::<StaticObjects>().unwrap());
    let name = |o: u32| format!("\"{}\"", t.core.file.ccs.object_name(o).unwrap_or("?"));
    let table = piney_data::statics::tables().iter().find(|x| x.name == "RT_MODELTABLE02").unwrap();
    let models = list((0..table.rows.len()).filter_map(|r| {
        let m = models_.model(r)?;
        let pos = m.pos.to_array().map(f32::to_bits);
        Some(format!("[{r}, {}, {}, {}]", pass(table.rows[r].pass), name(m.model), list(pos)))
    }));
    let objects =
        list((0..9).filter_map(|r| objects_.object(r).map(|(time, root)| format!("[{r}, {}, {time}]", mat(&root)))));
    let lights = list(t.core.lights.lights.iter().map(|l| {
        let v3 = |v: glam::Vec3| list(v.to_array().map(f32::to_bits));
        format!(
            "[{}, {}, {}, {}, {}, {}, {}]",
            l.kind,
            v3(l.colour),
            l.intensity.to_bits(),
            v3(l.pos),
            v3(l.dir),
            l.far_start.to_bits(),
            l.far_end.to_bits()
        )
    }));
    let f = &t.def.fog;
    let fog = format!(
        "[{}, {}, {}, {}, {}]",
        f.near.to_bits(),
        f.far.to_bits(),
        f.near_rate.to_bits(),
        f.far_rate.to_bits(),
        u32::from(f.colour[0]) | u32::from(f.colour[1]) << 8 | u32::from(f.colour[2]) << 16
    );
    let clear = t.def.clear.map_or(0, |c| u32::from(c[0]) | u32::from(c[1]) << 8 | u32::from(c[2]) << 16);
    let water = s.get::<Water>().unwrap();
    format!(
        "{{\"models\": {models}, \"objects\": {objects}, \"lights\": {lights}, \"fog\": {fog}, \"clear\": {clear}, \"sun\": {}, \"water\": {}}}",
        list(s.get::<LensFlare>().unwrap().sun),
        list((0..3).map(|k| water.time(k)))
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let volume = iso.volume().unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut town: Option<Town> = None;
    let mut waters: [Option<piney_world::town::WaterZero>; 2] = [None, None];
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| -> V4 { [n(i), n(i + 1), n(i + 2), n(i + 3)] };
        match w.first().copied() {
            Some("new") => {
                let mut t = Town::open(&archive, volume, 1, n(1) != 0).unwrap();
                t.set_field_rand(Rng::new(n(2)));
                println!("{}", state(&t));
                town = Some(t);
            }
            Some("draw") => {
                let t = town.as_mut().unwrap();
                let eye = [n(1), n(2), n(3), ee::ONE];
                let player = [n(4), n(5), n(6), ee::ONE];
                let puppet = n(7) != 0;
                let (view, rot, rot2, kind) = (v4(8), v4(12), v4(16), n(20) as i32);
                let cam = Cam { pos: eye, view, rot, rot3: rot2, kind, ..Cam::default() };
                let flare = FlareCamera { eye, view, rot: if kind == 1 { rot2 } else { rot } };
                let v = TownView { eye, player, cam, flare, puppet_show: puppet, world_screen: [[0; 4]; 4] };
                t.step(&v);
                let s = t.systems();
                let (sky, water, clouds) =
                    (s.get::<Sky>().unwrap(), s.get::<Water>().unwrap(), s.get::<Clouds>().unwrap());
                let (models, objects) = (s.get::<StaticModels>().unwrap(), s.get::<StaticObjects>().unwrap());
                let scrolls = s.get::<Scrolls>().unwrap();
                let out = list(t.frame().iter().map(|d| match *d {
                    Drawn::Piece(Kind::Objects, Piece::Object(row)) => {
                        format!("[\"obj\", {row}, {}]", objects.object(row).unwrap().0)
                    }
                    Drawn::Piece(Kind::Sky, Piece::Clump(k)) => match (sky.crisis(k), sky.name(k)) {
                        (Some(c), _) => format!("[\"crisis\", {c}]"),
                        (None, SUN) => "[\"sun\"]".into(),
                        (None, n) => match CLOUD_SKY.iter().position(|&c| c == n) {
                            Some(c) => format!("[\"cloudsky\", {c}]"),
                            None => "[\"sky\"]".into(),
                        },
                    },
                    Drawn::Piece(Kind::Models, Piece::Model(row)) => {
                        let m = models.model(row).unwrap();
                        format!("[\"model\", {row}, {}]", list(m.pos.to_array().map(f32::to_bits)))
                    }
                    Drawn::Piece(Kind::Water, Piece::Anim(k)) => format!("[\"water\", {k}, {}]", water.time(k)),
                    Drawn::Piece(Kind::Water, Piece::Copy) => "[\"copy\"]".into(),
                    Drawn::Map => "[\"map\"]".into(),
                    Drawn::Piece(Kind::LensFlare, Piece::Sprite { k, pos, .. }) => {
                        format!("[\"flare\", {k}, {}]", list(pos))
                    }
                    Drawn::Piece(Kind::Clouds, Piece::Sprite { k, .. }) => {
                        let c = &clouds.clouds[k];
                        format!("[\"cloud\", {k}, {}, {}]", list(c.pos), c.pattern)
                    }
                    d => panic!("Dun Loireag draws no {d:?}"),
                }));
                let clouds = list(clouds.clouds.iter().map(|c| {
                    format!(
                        "[{}, {}, {}, {}, {}, {}]",
                        c.angle,
                        c.pattern,
                        c.speed,
                        list(&c.pos[..3]),
                        c.eff.rotate,
                        c.eff.transparency
                    )
                }));
                // DrawBG writes the cloud layers' V from the first two
                // scrolls, and only in crisis the crisis materials' from
                // the third.
                let bg = if sky.made(4) { scrolls.uv(2) } else { 0 };
                println!(
                    "{{\"pieces\": {out}, \"uv\": {}, \"clo\": {}, \"bg\": {}, \"clouds\": {clouds}, \"seed\": {}, \"u\": {}, \"scroll\": {}}}",
                    water.uv,
                    list([scrolls.uv(0), scrolls.uv(1)]),
                    bg,
                    t.core.field_rand.map_or(0, |r| r.seed),
                    water.u,
                    list(&scrolls.v)
                );
            }
            Some("watermodel") | Some("wateruv") => {
                let (cmd, k) = (w[0], n(1) as usize);
                let w = waters[k].get_or_insert_with(|| {
                    let t = Town::open(&archive, volume, k as i32, false).unwrap();
                    let water = t.systems().get::<Water>();
                    water.unwrap_or_else(|| panic!("town {k} has no water")).zero.clone()
                });
                if cmd == "watermodel" {
                    let p = list(w.model.mmats[0].positions.iter().flatten().map(|&c| i32::from(c)));
                    println!("{{\"scale\": {}, \"positions\": {p}}}", w.model.scale.to_bits());
                    continue;
                }
                let eye = [n(2), n(3), n(4), ee::ONE];
                let m16 = |i: usize| -> [V4; 4] { [v4(i), v4(i + 4), v4(i + 8), v4(i + 12)] };
                let (lw, at, ws) = (m16(5), v4(21), m16(25));
                match water_st(volume, &w.model, &lw, at, eye, w.reach, &ws) {
                    Some(st) => println!("{}", list(st.iter().map(|s| list(*s)))),
                    None => println!("null"),
                }
            }
            _ => println!("{{}}"),
        }
    }
}
