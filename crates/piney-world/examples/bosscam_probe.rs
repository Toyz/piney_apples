//! Answers `tools/test_bosscam_rs.py`: the boss camera
//! (`piney_world::bosscam`, `ccBossCam`) over the field's camera and player
//! tasks, as `merchcam_probe` runs them. One request a line (numbers in
//! hex), one JSON line back.
//!
//! ```text
//! start T X Y Z DIRCZ SCHEME MODE SEED   Kite and the camera as the field's set-up leaves them
//! frame [DIRECT PUSH POWL DIRCL POWR DIRCR PW0..PW11]
//!                                   one frame of the tasks, then the cameras
//! boss T0..T3                       BossCam::new(transfer T) over Kite, then the cameras
//! quake Q0 Q1 Q2                    QuakeCam with the vector drawn
//! main B0..B3 [pad as frame]        CamMain toward the boss at B, then the boss camera and the cameras
//! off                               OffBossCamera, then the cameras
//! ```

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_world::bosscam::BossCam;
use piney_world::camera::{Cam, CamPad, Camera, Scheme};
use piney_world::hit::{HitModel, Hits};
use piney_world::motion::PLAYER_ANIM_TBL;
use piney_world::player::Player;
use piney_world::{Rand, ee, tasks};

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
        c.cam_id,
        cam(&c.tcam),
        cam(&c.bcam),
        m(&c.world_view),
        m(&c.world_screen)
    )
}

fn boss_cam(b: &BossCam) -> String {
    format!(
        "{{\"init_lock\": {}, \"reset\": {}, \"view\": {}, \"pos\": {}, \"rot\": {}, \"move_transfer\": {}, \
         \"deg\": {}, \"mem_dirc_z\": {}, \"remit_rot_max\": {}, \"count\": {}, \"camera_type\": {}, \
         \"quake_flg\": {}, \"quake\": {}}}",
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
        list(&b.quake)
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
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
    boss: Option<BossCam>,
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
    let models = HitModel::read(&Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap()).unwrap();
    let mut run: Option<Run> = None;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| [n(i), n(i + 1), n(i + 2), n(i + 3)];
        match w.first().copied() {
            Some("start") => {
                let pos = [n(2), n(3), n(4), ee::ONE];
                let dirc = [0, 0, n(5), 0];
                let mode = n(7) as i8;
                run = Some(Run {
                    hits: Hits::new(models.clone()),
                    player: Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000),
                    camera: Camera::new(pos, dirc, mode, Scheme::new(n(6) as i32)),
                    rand: Rand(u64::from(n(8))),
                    mode,
                    boss: None,
                });
                println!("{{}}");
            }
            Some("frame") => {
                let r = run.as_mut().unwrap();
                let pad = pad_at(&w, 1);
                tasks(&mut r.camera, &mut r.player, &mut r.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                println!("{}", cameras(&r.camera));
            }
            Some("boss") => {
                let r = run.as_mut().unwrap();
                let b = BossCam::new(&mut r.camera, v4(1), r.player.body.pos);
                println!("{{\"boss\": {}, \"cameras\": {}}}", boss_cam(&b), cameras(&r.camera));
                r.boss = Some(b);
            }
            Some("quake") => {
                let r = run.as_mut().unwrap();
                r.boss.as_mut().unwrap().quake([n(1), n(2), n(3)]);
                println!("{{}}");
            }
            Some("main") => {
                let r = run.as_mut().unwrap();
                let pad = pad_at(&w, 5);
                let b = r.boss.as_mut().unwrap();
                b.cam_main(&mut r.camera, &pad, r.player.body.pos, v4(1));
                println!("{{\"boss\": {}, \"cameras\": {}}}", boss_cam(b), cameras(&r.camera));
            }
            Some("off") => {
                let r = run.as_mut().unwrap();
                r.camera.boss_cam_off();
                println!("{}", cameras(&r.camera));
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
