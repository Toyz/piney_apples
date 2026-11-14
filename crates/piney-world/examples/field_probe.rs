//! Answers `tools/test_field_rt.py` (`field_probe ISO < requests`): a field as
//! `WORLD_MAN::GO(1)` builds it (`piney_world::field::area`), its collision,
//! its object passes and Kite and the camera walking it, one JSON line a
//! request. Requests: `new`, `charpos`, `dgpos`, `place`, `clear`, `land`,
//! `hitcheck`, `step`, `start`, `pad`; numbers hex, floats their bits, the
//! fields the harness's.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::field;
use piney_data::iso::Iso;
use piney_world::ee::{self, V4};
use piney_world::field::area::FieldArea;
use piney_world::hit;

#[path = "common/kite_run.rs"]
mod kite_run;
use kite_run::{KiteRun, Start};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

/// The hit list: each hit's index and its matrix's translation.
fn hit_list(f: &FieldArea) -> String {
    list(f.list.iter().map(|&h| {
        let t = f.model_hits[h].rm[3];
        format!("[{h}, {}]", list(t))
    }))
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    // The disc's own code: Outbreak's and Quarantine's square roots are
    // `sqrt.s` (worklog 474 found the same in story_map_probe).
    let volume = iso.volume().unwrap();
    let (data, body) = (kite_run::data(&mut iso), kite_run::body(&archive));
    let mut area: Option<FieldArea> = None;
    let mut run: Option<KiteRun> = None;
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
                let f = FieldArea::with_world(&archive, volume, params, 2, 0, None).unwrap();
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
                    f.core.hits.def_se,
                    hit_list(&f)
                );
                area = Some(f);
            }
            Some("charpos") => {
                let start = [n(3), n(4), n(5), ee::ONE];
                let dungeon = [n(6), n(7), n(8), ee::ONE];
                let back = piney_world::field::def::FieldDef::of(n(2)).expect("a field type").back;
                let (p, rot) = piney_world::field::area::field_start(n(1) as i32, &back, start, Some(dungeon));
                let m = piney_world::area::map::party_starts(1, p, rot);
                let members = list(m.iter().map(|v| list([v[0], v[1], v[2], rot])));
                println!("[{}, {}, {}, {rot}, {members}]", p[0], p[1], p[2]);
            }
            Some("dgpos") => {
                let m = piney_world::area::map::party_starts(2, [n(1), n(2), n(3), ee::ONE], n(4));
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
                f.core.hits.models.clear();
                println!("{{}}");
            }
            Some("land") => {
                let f = area.as_mut().unwrap();
                let z = f.core.hits.land([n(1), n(2), n(3), ee::ONE], hit::LAND_MASK);
                let h = &f.core.hits;
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
                let (r, out) = f.core.hits.hit_check(&mut body, pos, mv, 0x4234_0000, n(6), n(7) != 0);
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
                f.set_center(pos[0], pos[1]);
                f.core.hits.center = [pos[0], pos[1]];
                // A field's Kite (game.area 1), arriving.
                let start = Start {
                    pos,
                    dirc_z: n(4),
                    scheme: n(5) as i32,
                    mode: n(6) as i8,
                    seed: n(7),
                    area: 1,
                    arriving: true,
                };
                run = Some(KiteRun::new(&data, &body, &mut f.core.hits, &start));
                println!("{{}}");
            }
            Some("pad") => {
                let f = area.as_mut().unwrap();
                let r = run.as_mut().unwrap();
                r.frame(kite_run::pad_at(&w, 1), &mut f.core.hits);
                f.ofs = f.core.hits.center;
                f.step_objects(r.pos());
                let extra = [
                    ("enter", kite_run::b(r.enter)),
                    ("center", list(f.core.hits.center)),
                    ("list", hit_list(f)),
                    ("qualified", kite_run::b(r.qualified)),
                ];
                println!("{}", r.json(&extra));
            }
            _ => println!("{{\"error\": \"{line}\"}}"),
        }
    }
}
