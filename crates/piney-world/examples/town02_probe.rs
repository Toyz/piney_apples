//! Answers `tools/test_town02_rs.py`: Dun Loireag as `ROOTTOWN02` builds and
//! draws it (`piney_world::town02`, `cloud`, `lensflare`), one request a
//! line, one JSON line an answer, so the test can run the same through the
//! game's code in eemu.
//!
//! ```text
//! cargo build --release -p piney-world --example town02_probe
//! town02_probe ISO < requests
//! ```
//!
//! Numbers are hex; floats travel as their bit patterns. Requests:
//! - `new CRISIS SEED`: `ROOTTOWN02::ROOTTOWN02` (`town02d` when CRISIS),
//!   `fieldrand`'s seed SEED; answers the town's state: its STATICMODELs
//!   (row, type, model, position), STATICOBJECTs (row, animation, root,
//!   position, clip, time), lights (distant or omni, colour, intensity,
//!   place), `SetFog`, the clear colour, the sun's place, the water's
//!   times.
//! - `draw EX EY EZ PX PY PZ PUPPET VX VY VZ VW RX RY RZ RW QX QY QZ QW
//!   KIND`: one `ROOTTOWN02::Draw` with the camera's eye E (`cameraGetPos`,
//!   also `tcam`'s pos), the player at P, `puppetShow`, `tcam`'s view V,
//!   rot R and rot2 Q, its type KIND (1 the eye view): the pieces, then the
//!   state after it - `SetUV`'s value, `DrawBG`'s offsets, the clouds
//!   (angle, step, pattern, speed, pos), `fieldrand`'s seed and count, the
//!   scrolls.
//! - `watermodel TOWN`: the model water 0 draws in Mac Anu (0: `wat1`'s
//!   `MDL_wat00`) or Dun Loireag (1: `MDL_sr2wat00`): its vertex scale and
//!   its first mmat's stored positions.
//! - `wateruv TOWN EX EY EZ LW(16) AX AY AZ AW WS(16)`: `waterUVModifi2` of
//!   that water with the eye E, the object's world matrix LW and place A,
//!   and the view's `world_screen` WS: the texture coordinates it writes
//!   (null when out of reach).

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::iso::Iso;
use piney_data::statics::DrawPass;
use piney_world::camera::Cam;
use piney_world::ee::{self, V4};
use piney_world::evarea::FlareCamera;
use piney_world::town::{RootTown, Town, TownView, water_st};
use piney_world::town02::{DunLoireag, Piece};

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
    let t = &town.base;
    let name = |o: u32| format!("\"{}\"", t.file.ccs.object_name(o).unwrap_or("?"));
    let table = piney_data::statics::tables().iter().find(|x| x.name == "RT_MODELTABLE02").unwrap();
    let models = list((0..table.rows.len()).filter_map(|r| {
        let m = t.model(r)?;
        let pos = m.pos.to_array().map(f32::to_bits);
        Some(format!("[{r}, {}, {}, {}]", pass(table.rows[r].pass), name(m.model), list(pos)))
    }));
    let objects = list((0..9).filter_map(|r| t.object(r).map(|(time, root)| format!("[{r}, {}, {time}]", mat(&root)))));
    let lights = list(t.lights.lights.iter().map(|l| {
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
    let f = &t.fog;
    let fog = format!(
        "[{}, {}, {}, {}, {}]",
        f.near.to_bits(),
        f.far.to_bits(),
        f.near_rate.to_bits(),
        f.far_rate.to_bits(),
        u32::from(f.colour[0]) | u32::from(f.colour[1]) << 8 | u32::from(f.colour[2]) << 16
    );
    let clear = t.clear.map_or(0, |c| u32::from(c[0]) | u32::from(c[1]) << 8 | u32::from(c[2]) << 16);
    let d = town.class::<DunLoireag>().unwrap();
    format!(
        "{{\"models\": {models}, \"objects\": {objects}, \"lights\": {lights}, \"fog\": {fog}, \"clear\": {clear}, \"sun\": {}, \"water\": {}}}",
        list(d.sun_pos),
        list((0..3).map(|k| d.water_time(k)))
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
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
                let mut t = Town::open(&archive, 1, n(1) != 0).unwrap();
                t.parts_mut::<DunLoireag>().unwrap().1.rng = Rng::new(n(2));
                println!("{}", state(&t));
                town = Some(t);
            }
            Some("draw") => {
                let (t, d) = town.as_mut().unwrap().parts_mut::<DunLoireag>().unwrap();
                let eye = [n(1), n(2), n(3), ee::ONE];
                let player = [n(4), n(5), n(6), ee::ONE];
                let puppet = n(7) != 0;
                let (view, rot, rot2, kind) = (v4(8), v4(12), v4(16), n(20) as i32);
                let cam = Cam { pos: eye, view, rot, rot3: rot2, kind, ..Cam::default() };
                let flare = FlareCamera { eye, view, rot: if kind == 1 { rot2 } else { rot } };
                let v = TownView { eye, player, cam, flare, puppet_show: puppet, world_screen: [[0; 4]; 4] };
                let pieces = d.select(t, &v);
                let out = list(pieces.iter().map(|p| match *p {
                    Piece::Object(row) => format!("[\"obj\", {row}, {}]", t.object(row).unwrap().0),
                    Piece::Sky => "[\"sky\"]".into(),
                    Piece::Sun => "[\"sun\"]".into(),
                    Piece::CloudSky(k) => format!("[\"cloudsky\", {k}]"),
                    Piece::CrisisSky(k) => format!("[\"crisis\", {k}]"),
                    Piece::Model(row) => {
                        let m = t.model(row).unwrap();
                        format!("[\"model\", {row}, {}]", list(m.pos.to_array().map(f32::to_bits)))
                    }
                    Piece::Water(k) => format!("[\"water\", {k}, {}]", d.water_time(k)),
                    Piece::Copy => "[\"copy\"]".into(),
                    Piece::Map => "[\"map\"]".into(),
                    Piece::Flare(k, pos) => format!("[\"flare\", {k}, {}]", list(pos)),
                    Piece::Cloud(k) => {
                        let c = &d.clouds[k];
                        format!("[\"cloud\", {k}, {}, {}]", list(c.pos), c.pattern)
                    }
                }));
                let clouds = list(d.clouds.iter().map(|c| {
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
                println!(
                    "{{\"pieces\": {out}, \"uv\": {}, \"clo\": {}, \"bg\": {}, \"clouds\": {clouds}, \"seed\": {}, \"u\": {}, \"scroll\": {}}}",
                    d.water_uv,
                    list(d.cloud_uv),
                    d.bg_uv,
                    d.rng.seed,
                    d.water_u,
                    list([d.bg_scroll[0], d.bg_scroll[1], d.bg_scroll2])
                );
            }
            Some("watermodel") | Some("wateruv") => {
                let (cmd, k) = (w[0], n(1) as usize);
                let w = waters[k].get_or_insert_with(|| {
                    let t = Town::open(&archive, k as i32, false).unwrap();
                    t.water0().unwrap_or_else(|| panic!("town {k} has no water")).clone()
                });
                if cmd == "watermodel" {
                    let p = list(w.model.mmats[0].positions.iter().flatten().map(|&c| i32::from(c)));
                    println!("{{\"scale\": {}, \"positions\": {p}}}", w.model.scale.to_bits());
                    continue;
                }
                let eye = [n(2), n(3), n(4), ee::ONE];
                let m16 = |i: usize| -> [V4; 4] { [v4(i), v4(i + 4), v4(i + 8), v4(i + 12)] };
                let (lw, at, ws) = (m16(5), v4(21), m16(25));
                match water_st(&w.model, &lw, at, eye, w.reach, &ws) {
                    Some(st) => println!("{}", list(st.iter().map(|s| list(*s)))),
                    None => println!("null"),
                }
            }
            _ => println!("{{}}"),
        }
    }
}
