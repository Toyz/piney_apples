//! Answers `tools/test_merchcam_rs.py`: the shop's camera
//! (`Camera::set_merchant`, `SetMerchantCamera`) and `changeCamera(1)` over
//! the camera and the battle's tasks (Kite), as `world_probe` runs them. One
//! request a line (numbers in hex), one JSON line back: `merchants` (Mac
//! Anu's as `ccSetMerchant(0)` places them), `start`, `frame`, `merchant`
//! (`set_merchant` on that character), `change N` and `getrot N S0..S3`.

use piney_world::camera::CamId;
use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_world::camera::{self, Cam, CamPad, Camera, MerchantView};
use piney_world::ee;
use piney_world::hit::{HitModel, Hits};

#[path = "common/kite_run.rs"]
mod kite_run;
use kite_run::{KiteRun, Start};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn cam(c: &Cam) -> String {
    format!("{{\"pos\": {}, \"view\": {}, \"rot\": {}}}", list(&c.pos), list(&c.view), list(&c.rot))
}

fn cameras(c: &Camera) -> String {
    let m = |m: &[[u32; 4]; 4]| list(&m.iter().flatten().copied().collect::<Vec<_>>());
    format!(
        "{{\"cam_id\": {}, \"tcam\": {}, \"ecam\": {}, \"world_view\": {}, \"world_screen\": {}}}",
        c.list.id().raw(),
        cam(c.list.cam(CamId::Field)),
        cam(c.list.cam(CamId::Event)),
        m(&c.world_view),
        m(&c.world_screen)
    )
}

struct Run {
    hits: Hits,
    kite: KiteRun,
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
    let (data, body) = (kite_run::data(&mut iso), kite_run::body(&archive));
    let models = HitModel::read(volume, &Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap()).unwrap();
    let mut run: Option<Run> = None;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| [n(i), n(i + 1), n(i + 2), n(i + 3)];
        match w.first().copied() {
            Some("merchants") => {
                let mut town = piney_world::town::Town::open(&archive, volume, 0, false).unwrap();
                let player = [0, 0x45af_0000, 0x4416_0000, ee::ONE];
                let t = &mut town;
                let list_ = piney_world::town::merchant::set_merchants(
                    &archive,
                    volume,
                    &t.core.file,
                    &mut Vec::new(),
                    &mut t.core.hits,
                    0,
                    player,
                    false,
                )
                .unwrap();
                let out: Vec<String> = list_
                    .iter()
                    .map(|m| {
                        format!(
                            "{{\"id\": {}, \"flags\": {}, \"pos\": {}, \"dirc\": {}}}",
                            m.core.code(),
                            m.core.row.flags,
                            list(&m.core.ch.pos),
                            list(&m.core.ch.dirc)
                        )
                    })
                    .collect();
                println!("[{}]", out.join(", "));
            }
            Some("start") => {
                let mut hits = Hits::new(volume, models.clone());
                let start = Start {
                    pos: [n(2), n(3), n(4), ee::ONE],
                    dirc_z: n(5),
                    scheme: n(6) as i32,
                    mode: n(7) as i8,
                    seed: n(8),
                    area: 0,
                    arriving: true,
                };
                let kite = KiteRun::new(&data, &body, &mut hits, &start);
                run = Some(Run { hits, kite });
                println!("{{}}");
            }
            Some("frame") => {
                let r = run.as_mut().unwrap();
                let mut pad = CamPad::default();
                if w.len() >= 19 {
                    pad = CamPad {
                        direct: n(1),
                        push: n(2),
                        pow_l: n(3) as u8,
                        dirc_l: n(4),
                        pow_r: n(5) as u8,
                        dirc_r: n(6),
                        pow: std::array::from_fn(|k| n(7 + k) as u8),
                    };
                }
                r.kite.frame(pad, &mut r.hits);
                println!("{}", cameras(&r.kite.camera));
            }
            Some("merchant") => {
                let r = run.as_mut().unwrap();
                let target = if n(1) != 0 {
                    camera::breeder_view(volume, n(2) as i32).unwrap()
                } else {
                    MerchantView::Char { pos: v4(3), dirc: v4(7) }
                };
                r.kite.camera.set_merchant(target);
                println!("{}", cameras(&r.kite.camera));
            }
            Some("change") => {
                let r = run.as_mut().unwrap();
                r.kite.camera.list.change(CamId::from_raw(n(1) as i32).expect("camID 0-3"));
                println!("{}", cameras(&r.kite.camera));
            }
            Some("getrot") => {
                let r = run.as_mut().unwrap();
                println!(
                    "{}",
                    list(&r.kite.camera.list.get_rot(CamId::from_raw(n(1) as i32).expect("camID 0-3"), v4(2)))
                );
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
