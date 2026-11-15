//! Answers `tools/test_bosscam_rs.py`: the boss camera (`piney_battle::boss::cam`,
//! `ccBossCam`) over the camera and the battle's tasks (Kite), as `merchcam_probe`
//! runs them. One request a line (numbers in hex), one JSON line back:
//! `start` (Kite and the camera), `frame` (one frame of the tasks), `boss`
//! (`BossCam::new`), `quake` (`QuakeCam`), `main` (`CamMain` toward the boss),
//! `sway` (`ZrotControlFlg` set), `mode` (`SetMode(mode, range)`), `note` (a
//! sound of Kite's) and `off` (`OffBossCamera`), each followed by the
//! cameras; `main` with the frame's sounds and their listeners.

use std::io::BufRead;
use std::sync::Arc;

use piney_battle::boss::cam::{BossCam, off_boss_camera};
use piney_battle::kite;
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_world::camera::{Cam, CamPad, Camera};
use piney_world::combat::Show;
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
        "{{\"cam_id\": {}, \"tcam\": {}, \"bcam\": {}, \"world_view\": {}, \"world_screen\": {}}}",
        c.list.id().raw(),
        cam(c.list.cam(piney_world::camera::CamId::Field)),
        cam(c.list.cam(piney_world::camera::CamId::Battle)),
        m(&c.world_view),
        m(&c.world_screen)
    )
}

/// Kite's sounds of the last frame as the show stream hands them on
/// (`piney_world::combat::heard`, the camera as it stands now before any
/// mark): param, his place, the listener's eye, view and type.
fn sounds(k: &KiteRun) -> String {
    let end = k.camera.list.ear();
    let v: Vec<String> = piney_world::combat::heard(&k.combat.shows, end)
        .filter_map(|(s, e)| match s {
            Show::Kite(i, kite::Out::Sound { param }) => Some(format!(
                "[{param}, {}, [{}, {}, {}]]",
                list(&k.combat.scene.chars[*i].pos),
                list(&e.pos),
                list(&e.view),
                e.kind
            )),
            _ => None,
        })
        .collect();
    format!("[{}]", v.join(", "))
}

fn boss_cam(b: &BossCam) -> String {
    format!(
        "{{\"init_lock\": {}, \"reset\": {}, \"view\": {}, \"pos\": {}, \"rot\": {}, \"move_transfer\": {}, \
         \"deg\": {}, \"mem_dirc_z\": {}, \"remit_rot_max\": {}, \"count\": {}, \"camera_type\": {}, \
         \"quake_flg\": {}, \"quake\": {}, \"zrot_cont\": {}, \"ex_lock\": {}}}",
        u8::from(b.init_lock),
        b.reset,
        list(&b.view),
        list(&b.pos),
        list(&b.rot),
        list(&b.move_transfer),
        b.deg,
        b.mem_dirc_z,
        b.remit_rot_max,
        b.count,
        b.camera_type,
        u8::from(b.quake_flg),
        list(&b.quake),
        b.zrot_cont,
        b.ex_lock
    )
}

fn pad_at(w: &[&str], at: usize) -> CamPad {
    let n = |i: usize| hex(w[i]);
    if w.len() < at + 18 {
        return CamPad::default();
    }
    CamPad {
        direct: n(at),
        push: n(at + 1),
        pow_l: n(at + 2) as u8,
        dirc_l: n(at + 3),
        pow_r: n(at + 4) as u8,
        dirc_r: n(at + 5),
        pow: std::array::from_fn(|k| n(at + 6 + k) as u8),
    }
}

struct Run {
    hits: Hits,
    kite: KiteRun,
    boss: Option<BossCam>,
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let volume = iso.volume().unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let (data, body) = (kite_run::data(&mut iso), kite_run::body(&archive));
    let models = HitModel::read(volume, &Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap()).unwrap();
    let mut run: Option<Run> = None;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| [n(i), n(i + 1), n(i + 2), n(i + 3)];
        match w.first().copied() {
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
                run = Some(Run { hits, kite, boss: None });
                println!("{{}}");
            }
            Some("frame") => {
                let r = run.as_mut().unwrap();
                let pad = pad_at(&w, 1);
                r.kite.frame(pad, &mut r.hits);
                println!("{}", cameras(&r.kite.camera));
            }
            Some("boss") => {
                let r = run.as_mut().unwrap();
                let at = r.kite.pos();
                let c = &mut r.kite.camera;
                let b = BossCam::new(&mut c.list, c.scheme, v4(1), at);
                println!("{{\"boss\": {}, \"cameras\": {}}}", boss_cam(&b), cameras(&r.kite.camera));
                r.boss = Some(b);
            }
            Some("sway") => {
                run.as_mut().unwrap().boss.as_mut().unwrap().zrot_control = true;
                println!("{{}}");
            }
            Some("mode") => {
                let r = run.as_mut().unwrap();
                let b = r.boss.as_mut().unwrap();
                b.set_mode_range(n(1) as i32, n(2), &mut r.kite.camera.list);
                println!("{{\"boss\": {}}}", boss_cam(b));
            }
            Some("quake") => {
                let r = run.as_mut().unwrap();
                r.boss.as_mut().unwrap().quake([n(1), n(2), n(3)]);
                println!("{{}}");
            }
            Some("main") => {
                let r = run.as_mut().unwrap();
                let pad = pad_at(&w, 5);
                let at = r.kite.pos();
                let b = r.boss.as_mut().unwrap();
                let c = &mut r.kite.camera;
                b.cam_main(&mut c.list, c.scheme, &pad, at, v4(1));
                let (b, s) = (boss_cam(b), sounds(&r.kite));
                println!("{{\"boss\": {b}, \"cameras\": {}, \"sounds\": {s}}}", cameras(&r.kite.camera));
            }
            // A note's sound at Kite's Draw (ccPlayer::Main's, after its
            // CameraPosSet): `ccSeSetParamSPC(param, Kite)` in the frame's
            // show stream.
            Some("note") => {
                let r = run.as_mut().unwrap();
                let k = r.kite.kite();
                r.kite.combat.shows.push(Show::Kite(k, kite::Out::Sound { param: hex(w[1]) }));
                println!("{{}}");
            }
            Some("off") => {
                let r = run.as_mut().unwrap();
                off_boss_camera(&mut r.kite.camera.list);
                println!("{}", cameras(&r.kite.camera));
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
