//! Answers `tools/test_town03_rs.py` (`carmina_gadelica_probe ISO < requests`): Carmina
//! Gade as Mutation's `ROOTTOWN03` builds and draws it
//! (`piney_world::town::carmina_gadelica`, with its airship), one JSON line a request:
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
use piney_world::systems::scrolls::Scrolls;
use piney_world::systems::sky::Sky;
use piney_world::systems::statics::StaticModels;
use piney_world::systems::water::Water;
use piney_world::systems::{Drawn, Kind, Piece};
use piney_world::town::carmina_gadelica::Airship;
use piney_world::town::{Town, TownEvent, TownView};

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
    let s = t.systems();
    let name = |o: u32| format!("\"{}\"", t.core.file.ccs.object_name(o).unwrap_or("?"));
    let table = piney_data::statics::tables().iter().find(|x| x.name == "RT_MODELTABLE03").unwrap();
    let models = list((0..table.rows.len()).filter_map(|r| {
        let m = s.get::<StaticModels>()?.model(r)?;
        let pos = [m.pos[0], m.pos[1], m.pos[2]];
        Some(format!("[{r}, {}, {}, {}]", pass(table.rows[r].pass), name(m.model), list(pos)))
    }));
    let lights = list(t.core.lights.lights.iter().map(|l| l.kind));
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
        "{{\"models\": {models}, \"lights\": {lights}, \"fog\": {fog}, \"clear\": {clear}, \"water\": {}, \"ship\": {}}}",
        list((0..3).map(|k| water.time(k))),
        ship(s.get::<Airship>().unwrap())
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/mutation/mutation.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let volume = iso.volume().unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut town: Option<Town> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        match w.first().copied() {
            Some("new") => {
                let mut t = Town::open(&archive, volume, 2, n(1) != 0).unwrap();
                t.set_field_rand(Rng::new(n(2)));
                println!("{}", state(&t));
                town = Some(t);
            }
            Some("draw") => {
                let t = town.as_mut().unwrap();
                t.step(&TownView { eye: [n(1), n(2), n(3), ee::ONE], ..TownView::default() });
                let events = t.take_events();
                let s = t.systems();
                let (sky, water, d) = (s.get::<Sky>().unwrap(), s.get::<Water>().unwrap(), s.get::<Airship>().unwrap());
                let models = s.get::<StaticModels>().unwrap();
                let scrolls = s.get::<Scrolls>().unwrap();
                let out = list(t.frame().iter().filter_map(|p| {
                    Some(match *p {
                        Drawn::Piece(Kind::Water, Piece::Anim(k)) => format!("[\"water\", {k}, {}]", water.time(k)),
                        Drawn::Piece(Kind::Water, Piece::Copy) => "[\"copy\"]".into(),
                        Drawn::Piece(Kind::Airship, _) => format!("[\"ship\", {}]", d.play.time),
                        Drawn::Piece(Kind::Sky, Piece::Clump(k)) => match sky.crisis(k) {
                            Some(c) => format!("[\"crisis\", {c}]"),
                            None => format!("[\"back\", {k}]"),
                        },
                        Drawn::Piece(Kind::Models, Piece::Model(row)) => {
                            let m = models.model(row)?;
                            format!("[\"model\", {row}, {}]", list([m.pos[0], m.pos[1], m.pos[2]]))
                        }
                        Drawn::Piece(Kind::Models, Piece::ModelNoFog(row)) => format!("[\"nofog\", {row}]"),
                        Drawn::Map => "[\"map\"]".into(),
                        d => panic!("Carmina Gade draws no {d:?}"),
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
                // DrawBG writes the crisis materials' offset only in crisis.
                let bg = if sky.made(4) { scrolls.uv(1) } else { 0 };
                println!(
                    "{{\"pieces\": {out}, \"ship\": {}, \"root\": {}, \"events\": {events}, \"seed\": {}, \"scroll\": {}, \"bg\": {}, \"u\": {}}}",
                    ship(d),
                    list(d.root.iter().flatten()),
                    t.core.field_rand.map_or(0, |r| r.seed),
                    list(&scrolls.v),
                    bg,
                    water.u
                );
            }
            _ => println!("{{}}"),
        }
    }
}
