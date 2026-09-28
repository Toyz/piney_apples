//! Answers `tools/test_evcam_rs.py`: the event camera (evcam.rs) and the
//! field's tasks around it - the event's camera instructions, then each
//! frame `ccThCameraExecute`'s `CamCtrl`, `cameraMain` and
//! `ccPlayer::Main` - printing the state as one JSON line, so the test can
//! run the same through the game's own code in eemu.
//!
//! ```text
//! cargo build --release -p piney-world --example evcam_probe
//! evcam_probe ISO < requests
//! ```
//!
//! Numbers are hex; floats travel as their bit patterns, signed values as
//! their 32-bit two's complement. Requests, one a line:
//! - `start X Y Z DIRCZ SCHEME MODE SEED`: Mac Anu (`town01`), Kite
//!   arriving at (X, Y, Z) facing DIRCZ, camera scheme SCHEME and
//!   `cameraMode` MODE, `rand` seeded with SEED; the camera as ccThCamera's
//!   set-up leaves it, no event camera. Prints the state.
//! - `char TYPE CODE X Y Z W`: what the scene answers for the character
//!   (TYPE, CODE); `kite TYPE CODE`: Kite's position, as he moves;
//!   `unchar TYPE CODE`: nothing (the leader, Kite, stands in).
//! - `mark N X Y Z W`: marker N's position; `markerpos N`: `markerEvTbl[N]`'s
//!   dummy in `town01` (`World::marker_bits`), printed.
//! - `cmd OP A0 A1 ..`: one camera instruction (code OP, its operands as the
//!   script has them) at play level. Prints the state.
//! - `teach PART`, `ban ON`: `teach_camera`'s and `menu_ban` / `menu_clear`'s
//!   camera parts. Print the state.
//! - `pad DIRECT PUSH POWL DIRCL POWR DIRCR POW0 .. POW11 ZOOMIN ZOOMOUT`:
//!   one frame (the event camera's task if it runs, `cameraMain`,
//!   `ccPlayer::Main`). Prints the state.
//!
//! The state: `ctrl`, `ccEvCamCtrl` as the game lays it out (196 words,
//! padding zero); `running`, `cam_id`, `normal`, `puppet`; `tcam`, `ecam`,
//! `bcam` as `CAMERA`'s first 26 words (`cptr` zero); `resetting`,
//! `mem_dirc_z`, `mode`, `world_view`, `world_screen`; and Kite's
//! position, heading, move, speeds, flags, act, counters, cloak,
//! transparency, `angle`, `lost_head`.

use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_event::host::CameraCommand;
use piney_world::camera::{Cam, CamPad, Camera, Scheme};
use piney_world::ee::{self, V4};
use piney_world::evcam::{Camz, EvCamCtrl, EventCam, Input, Scene};
use piney_world::hit::{HitModel, Hits};
use piney_world::motion::PLAYER_ANIM_TBL;
use piney_world::player::Player;
use piney_world::{Rand, tasks};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn pair(a: i16, b: i16) -> u32 {
    u32::from(a as u16) | u32::from(b as u16) << 16
}

/// `ccEvCamz`'s 0x150 bytes as words.
fn camz_words(z: &Camz, out: &mut Vec<u32>) {
    out.extend([pair(z.ctrl_type, z.spd_type), pair(z.inp_num, z.pad), z.inp_alpha, z.rate, z.cnt, 0, 0, 0]);
    out.extend(z.now);
    out.extend(z.start);
    out.extend(z.target);
    for p in &z.inp_pos {
        out.extend(p);
    }
}

/// `ccEvCamCtrl`'s 0x310 bytes as words.
fn ctrl_words(e: &EvCamCtrl) -> Vec<u32> {
    let mut w = vec![pair(e.vp_ctrl, e.cp_ctrl), e.char_type as u32, e.char_id as u32, e.height, e.vp_rate, 0, 0, 0];
    w.extend(e.vp);
    w.extend(e.vp_target);
    w.extend([e.cp_rate, e.dist, e.dist_target, e.rot[0], e.rot[1], pair(e.rot_target[0], e.rot_target[1]), 0, 0]);
    w.extend(e.cp);
    for z in &e.zcam {
        camz_words(z, &mut w);
    }
    assert_eq!(w.len(), 0x310 / 4);
    w
}

/// `CAMERA`'s first 26 words (0x68 bytes), `cptr` zero.
fn cam_words(c: &Cam) -> Vec<u32> {
    let mut w = Vec::new();
    for v in [c.pos, c.view, c.rot, c.rot2, c.rot3] {
        w.extend(v);
    }
    w.extend([0, c.dist, pair(c.deg[0], c.deg[1]), c.kind as u32, u32::from(c.reset_flag), c.reset_dirc]);
    w
}

/// Where a character stands.
#[derive(Clone, Copy)]
enum Who {
    At(V4),
    Kite,
}

struct Run {
    hits: Hits,
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
    ev: EventCam,
    chars: HashMap<(i32, i32), Who>,
    marks: HashMap<i16, V4>,
}

/// The scene the camera sees: the table the test set, Kite as leader.
struct Look<'a> {
    player: &'a Player,
    chars: &'a HashMap<(i32, i32), Who>,
    marks: &'a HashMap<i16, V4>,
}

impl Scene for Look<'_> {
    fn char_pos(&self, ty: i32, code: i32) -> Option<V4> {
        self.chars.get(&(ty, code)).map(|w| match w {
            Who::At(p) => *p,
            Who::Kite => self.player.body.pos,
        })
    }
    fn leader_pos(&self) -> V4 {
        self.player.body.pos
    }
    fn marker_pos(&self, marker: i16) -> Option<V4> {
        self.marks.get(&marker).copied()
    }
    fn player_dirc(&self) -> V4 {
        self.player.body.dirc
    }
}

/// An instruction's code and operands as `ccEvent::Execute` reads them, as
/// the interpreter hands them to its host (`piney-event`'s exec.rs).
fn command(op: u32, a: &[i16]) -> CameraCommand {
    let g = |i: usize| a.get(i).copied().unwrap_or(0);
    let v = |i: usize| [g(i), g(i + 1), g(i + 2)];
    match op {
        22 => CameraCommand::LookPos { x: g(0), y: g(1), z: g(2), half: false },
        23 => CameraCommand::LookChar { ty: g(0), code: g(1), height: g(2), half: false },
        24 => CameraCommand::FollowChar { ty: g(0), code: g(1), height: g(2) },
        25 => CameraCommand::LookMarker { marker: g(0), half: false },
        26 => CameraCommand::LookPos { x: g(0), y: g(1), z: g(2), half: true },
        27 => CameraCommand::LookChar { ty: g(0), code: g(1), height: g(2), half: true },
        28 => CameraCommand::LookMarker { marker: g(0), half: true },
        29 => CameraCommand::PanPos { x: g(0), y: g(1), z: g(2), rate: g(3) },
        30 => CameraCommand::PanChar { ty: g(0), code: g(1), height: g(2), rate: g(3), follow: false },
        31 => CameraCommand::PanChar { ty: g(0), code: g(1), height: g(2), rate: g(3), follow: true },
        32 => CameraCommand::PanMarker { marker: g(0), rate: g(1) },
        33 => CameraCommand::Orbit { rotx: g(0), roty: g(1), dist: g(2) },
        34 => CameraCommand::OrbitMove { rotx: g(0), roty: g(1), dist: g(2), rate: g(3), degrees: false },
        35 => CameraCommand::OrbitMove { rotx: g(0), roty: g(1), dist: g(2), rate: g(3), degrees: true },
        36 => CameraCommand::Mode4,
        40 => CameraCommand::ZSet { v: v(0), c: v(3) },
        41 => CameraCommand::ZMove { v: v(0), c: v(3), vprate: g(6), cprate: g(7) },
        42 => CameraCommand::ZSpeed { vstype: g(0), cstype: g(1) },
        43 => CameraCommand::ZPoint { num: g(0), v: v(1), c: v(4) },
        44 => CameraCommand::ZPath { num: g(0), vprate: g(1), cprate: g(2), alpha: g(3) },
        48 => CameraCommand::Char { ty: g(0), code: g(1), height: g(2), rotx: g(3), roty: g(4), dist: g(5) },
        49 => CameraCommand::End,
        50 => CameraCommand::EndReset,
        _ => panic!("not a camera instruction: {op}"),
    }
}

fn state(r: &Run) -> String {
    let (p, b, a, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
    format!(
        concat!(
            "{{\"ctrl\": {}, \"running\": {}, \"cam_id\": {}, \"normal\": {}, \"puppet\": {}, ",
            "\"tcam\": {}, \"ecam\": {}, \"bcam\": {}, \"resetting\": {}, \"mem_dirc_z\": {}, \"mode\": {}, ",
            "\"world_view\": {}, \"world_screen\": {}, ",
            "\"pos\": {}, \"dirc\": {}, \"move\": {}, \"now_speed\": {}, \"speed_rate\": {}, ",
            "\"move_flag\": {}, \"run_flag\": {}, \"restraint\": {}, \"act\": {}, \"act_cnt\": {}, ",
            "\"cloak\": {}, \"transparency\": {}, \"drawn\": {}, \"angle\": {}, \"target_count\": {}, ",
            "\"attribute\": {}}}"
        ),
        list(&ctrl_words(&r.ev.ctrl)),
        u8::from(r.ev.running()),
        c.cam_id,
        r.ev.normal_cam_id,
        u8::from(r.camera.puppet_show),
        list(&cam_words(&c.tcam)),
        list(&cam_words(&c.ecam)),
        list(&cam_words(&c.bcam)),
        u8::from(c.resetting),
        c.mem_dirc_z,
        r.mode,
        list(&c.world_view.iter().flatten().copied().collect::<Vec<_>>()),
        list(&c.world_screen.iter().flatten().copied().collect::<Vec<_>>()),
        list(&b.pos),
        list(&b.dirc),
        list(&b.move_pos),
        b.now_speed,
        b.speed_rate,
        u8::from(b.move_flag),
        u8::from(b.run_flag),
        u8::from(b.restraint),
        a.act,
        a.act_cnt,
        a.cloak,
        p.transparency,
        u8::from(p.drawn),
        list(&p.angle),
        b.target_count,
        p.hit_attribute,
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let town = SceneFile::read(&archive, "town01").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
    let models = HitModel::read(&Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap()).unwrap();
    let mut run: Option<Run> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let s = |i: usize| hex(w[i]) as i32;
        match w.first().copied() {
            Some("start") => {
                let pos = [n(1), n(2), n(3), ee::ONE];
                let dirc = [0, 0, n(4), 0];
                let mode = n(6) as i8;
                run = Some(Run {
                    hits: Hits::new(models.clone()),
                    player: Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000),
                    camera: Camera::new(pos, dirc, mode, Scheme::new(n(5) as i32)),
                    rand: Rand(u64::from(n(7))),
                    mode,
                    ev: EventCam::new(),
                    chars: HashMap::new(),
                    marks: HashMap::new(),
                });
                println!("{}", state(run.as_ref().unwrap()));
            }
            Some("char") => {
                let r = run.as_mut().unwrap();
                r.chars.insert((s(1), s(2)), Who::At([n(3), n(4), n(5), n(6)]));
            }
            Some("kite") => {
                let r = run.as_mut().unwrap();
                r.chars.insert((s(1), s(2)), Who::Kite);
            }
            Some("unchar") => {
                let r = run.as_mut().unwrap();
                r.chars.remove(&(s(1), s(2)));
            }
            Some("mark") => {
                let r = run.as_mut().unwrap();
                r.marks.insert(s(1) as i16, [n(2), n(3), n(4), n(5)]);
            }
            Some("markerpos") => {
                let m = n(1);
                let name = piney_data::tables::world::of(volume).markers()[m as usize];
                let obj = town.ccs.find_object(name).unwrap();
                let d = &town.scene.dummies[&obj];
                println!("{}", list(&[d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ee::ONE]));
            }
            Some("cmd") => {
                let r = run.as_mut().unwrap();
                let args: Vec<i16> = (2..w.len()).map(|i| s(i) as i16).collect();
                let c = command(n(1), &args);
                let look = Look { player: &r.player, chars: &r.chars, marks: &r.marks };
                r.ev.command(c, &mut r.camera, &look);
                println!("{}", state(r));
            }
            Some("teach") => {
                let r = run.as_mut().unwrap();
                r.ev.teach(n(1) as u8);
                println!("{}", state(r));
            }
            Some("ban") => {
                let r = run.as_mut().unwrap();
                r.ev.menu_ban(n(1) != 0, &mut r.camera);
                println!("{}", state(r));
            }
            Some("pad") => {
                let r = run.as_mut().unwrap();
                let mut pow = [0u8; 12];
                for (k, p) in pow.iter_mut().enumerate() {
                    *p = n(7 + k) as u8;
                }
                let pad = CamPad {
                    direct: n(1),
                    push: n(2),
                    pow_l: n(3) as u8,
                    dirc_l: n(4),
                    pow_r: n(5) as u8,
                    dirc_r: n(6),
                    pow,
                };
                let input = Input { pad, zoom: [n(19) as u16, n(20) as u16] };
                // ccThCameraExecute (33), then ccThCamera (40) and
                // ccThPlayer (49).
                let look = Look { player: &r.player, chars: &r.chars, marks: &r.marks };
                r.ev.frame(&mut r.camera, &look, &input);
                tasks(&mut r.camera, &mut r.player, &mut r.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                println!("{}", state(r));
            }
            _ => {}
        }
    }
}
