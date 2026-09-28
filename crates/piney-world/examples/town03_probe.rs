//! Answers `tools/test_town03_rs.py` (`town03_probe ISO < requests`): Carmina
//! Gade as Mutation's `ROOTTOWN03` builds and draws it
//! (`piney_world::town03`, with its airship), one JSON line a request:
//! `new CRISIS SEED` (the town's state) and `draw EX EY EZ` (one `Draw`: the
//! pieces, the airship, its puffs and sounds, `fieldrand`, the scrolls).
//! Numbers hex, floats their bits.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::iso::Iso;
use piney_data::statics::DrawPass;
use piney_world::ee;
use piney_world::town::{RootTown, Town, TownEvent, TownView};
use piney_world::town03::{Airship, CarminaGade, Piece};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn pass(p: DrawPass) -> u32 {
    match p {
        DrawPass::Other => 0,
        DrawPass::Floor => 1,
        DrawPass::Obj => 2,
        DrawPass::Obj2 => 3,
    }
}

fn ship(s: &Airship) -> String {
    format!(
        "{{\"leg\": {}, \"turn\": {}, \"wait\": {}, \"t\": {}, \"puff\": {}, \"pos\": {}, \"dirc\": {}, \"way\": {}, \"from\": {}, \"to\": {}}}",
        s.leg,
        s.turn,
        s.wait,
        s.t,
        s.puff,
        list(s.pos),
        list(s.dirc),
        list(s.way),
        list(s.from),
        list(s.to)
    )
}

/// The town as the constructor left it.
fn state(t: &Town) -> String {
    let name = |o: u32| format!("\"{}\"", t.base.file.ccs.object_name(o).unwrap_or("?"));
    let table = piney_data::statics::tables().iter().find(|x| x.name == "RT_MODELTABLE03").unwrap();
    let models = list((0..table.rows.len()).filter_map(|r| {
        let m = t.base.model(r)?;
        let pos = m.pos.to_array().map(f32::to_bits);
        Some(format!("[{r}, {}, {}, {}]", pass(table.rows[r].pass), name(m.model), list(pos)))
    }));
    let lights = list(t.base.lights.lights.iter().map(|l| l.kind));
    let f = &t.base.fog;
    let fog = format!(
        "[{}, {}, {}, {}, {}]",
        f.near.to_bits(),
        f.far.to_bits(),
        f.near_rate.to_bits(),
        f.far_rate.to_bits(),
        u32::from(f.colour[0]) | u32::from(f.colour[1]) << 8 | u32::from(f.colour[2]) << 16
    );
    let clear = t.base.clear.map_or(0, |c| u32::from(c[0]) | u32::from(c[1]) << 8 | u32::from(c[2]) << 16);
    let d = t.class::<CarminaGade>().unwrap();
    format!(
        "{{\"models\": {models}, \"lights\": {lights}, \"fog\": {fog}, \"clear\": {clear}, \"water\": {}, \"ship\": {}}}",
        list((0..3).map(|k| d.water_time(k))),
        ship(&d.ship)
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/mutation/mutation.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut town: Option<Town> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        match w.first().copied() {
            Some("new") => {
                let mut t = Town::open(&archive, 2, n(1) != 0).unwrap();
                t.parts_mut::<CarminaGade>().unwrap().1.rng = Rng::new(n(2));
                println!("{}", state(&t));
                town = Some(t);
            }
            Some("draw") => {
                let (t, d) = town.as_mut().unwrap().parts_mut::<CarminaGade>().unwrap();
                let v = TownView { eye: [n(1), n(2), n(3), ee::ONE], ..TownView::default() };
                let pieces = d.select(t, &v);
                let events = t.take_events();
                let out = list(pieces.iter().filter_map(|p| {
                    Some(match *p {
                        Piece::Water(k) => format!("[\"water\", {k}, {}]", d.water_time(k)),
                        Piece::Copy => "[\"copy\"]".into(),
                        Piece::Ship => format!("[\"ship\", {}]", d.ship.play.time),
                        Piece::Back(k) => format!("[\"back\", {k}]"),
                        Piece::CrisisSky(k) => format!("[\"crisis\", {k}]"),
                        Piece::Model(row) => {
                            let m = t.model(row)?;
                            format!("[\"model\", {row}, {}]", list(m.pos.to_array().map(f32::to_bits)))
                        }
                        Piece::ModelNoFog(row) => format!("[\"nofog\", {row}]"),
                        Piece::Map => "[\"map\"]".into(),
                    })
                }));
                let events = list(events.iter().map(|e| match *e {
                    TownEvent::SmokeN { pos, v, scale, life, kind, fade } => format!(
                        "[\"smoke\", {}, {}, {scale}, {life}, {kind}, {}, {}]",
                        list(&pos[..3]),
                        list(&v[..3]),
                        fade.0,
                        fade.1
                    ),
                    TownEvent::Se3d { n, pos } => format!("[\"se\", {n}, {}]", list(&pos[..3])),
                }));
                println!(
                    "{{\"pieces\": {out}, \"ship\": {}, \"root\": {}, \"events\": {events}, \"seed\": {}, \"scroll\": {}, \"bg\": {}, \"u\": {}}}",
                    ship(&d.ship),
                    list(d.ship.root.iter().flatten()),
                    d.rng.seed,
                    list(d.bg_scroll),
                    d.bg_uv,
                    d.water_u
                );
            }
            _ => println!("{{}}"),
        }
    }
}
