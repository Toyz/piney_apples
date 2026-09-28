//! Answers `tools/test_merchcam_rs.py`: the shop's camera
//! (`Camera::set_merchant`, `SetMerchantCamera`) and `changeCamera(1)` over
//! the field's camera and player tasks, as `world_probe` runs them. One
//! request a line (numbers in hex), one JSON line back.
//!
//! ```text
//! merchants                         Mac Anu's merchants as ccSetMerchant(0) places them
//! start T X Y Z DIRCZ SCHEME MODE SEED   Kite and the camera as the field's set-up leaves them
//! frame [DIRECT PUSH POWL DIRCL POWR DIRCR PW0..PW11]
//!                                   one frame of the tasks, then the cameras
//! merchant BREEDER SERVER P0..P3 D0..D3  set_merchant on that character, then the cameras
//! change N                          changeCamera(N), then the cameras
//! getrot N S0..S3                   cameraGetRot(out = S, N)
//! ```

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_world::camera::{self, Cam, CamPad, Camera, MerchantView, Scheme};
use piney_world::entry::Npc;
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
        "{{\"cam_id\": {}, \"tcam\": {}, \"ecam\": {}, \"world_view\": {}, \"world_screen\": {}}}",
        c.cam_id,
        cam(&c.tcam),
        cam(&c.ecam),
        m(&c.world_view),
        m(&c.world_screen)
    )
}

struct Run {
    hits: Hits,
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
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
            Some("merchants") => {
                let mut town = piney_world::town::Town::open(&archive, 0, false).unwrap();
                let player = [0, 0x45af_0000, 0x4416_0000, ee::ONE];
                let t = &mut town.base;
                let list_ =
                    piney_world::merchant::set_merchants(&archive, volume, &t.file, &mut t.hits, 0, player).unwrap();
                let out: Vec<String> = list_
                    .iter()
                    .map(|m| {
                        format!(
                            "{{\"id\": {}, \"flags\": {}, \"pos\": {}, \"dirc\": {}}}",
                            m.code(),
                            m.flags(),
                            list(&m.char().pos),
                            list(&m.char().dirc)
                        )
                    })
                    .collect();
                println!("[{}]", out.join(", "));
            }
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
                });
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
                tasks(&mut r.camera, &mut r.player, &mut r.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                println!("{}", cameras(&r.camera));
            }
            Some("merchant") => {
                let r = run.as_mut().unwrap();
                let target = if n(1) != 0 {
                    camera::breeder_view(volume, n(2) as i32).unwrap()
                } else {
                    MerchantView::Char { pos: v4(3), dirc: v4(7) }
                };
                r.camera.set_merchant(target);
                println!("{}", cameras(&r.camera));
            }
            Some("change") => {
                let r = run.as_mut().unwrap();
                r.camera.change_camera(n(1) as i16);
                println!("{}", cameras(&r.camera));
            }
            Some("getrot") => {
                let r = run.as_mut().unwrap();
                println!("{}", list(&r.camera.get_rot(n(1) as i16, v4(2))));
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
