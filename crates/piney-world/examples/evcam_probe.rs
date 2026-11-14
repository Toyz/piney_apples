//! Answers `tools/test_evcam_rs.py` (`evcam_probe ISO < requests`): the event
//! camera (camera/evcam.rs) and the field's tasks around it - the event's camera
//! instructions, then each frame `CamCtrl`, `cameraMain` and the battle's
//! tasks with Kite's `ccPlayer::Main` - printing the state (`ccEvCamCtrl` as the game lays it
//! out, the three cameras, Kite) as one JSON line. Requests: `start`, `char`,
//! `kite`, `unchar`, `mark`, `markerpos`, `cmd`, `teach`, `ban`, `pad`;
//! numbers hex, floats their bits, the fields the harness's.

use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_event::host::CameraCommand;
use piney_world::camera::Cam;
use piney_world::camera::evcam::{Camz, EvCamCtrl, EventCam, Input, Scene};
use piney_world::ee::{self, V4};
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
    kite: KiteRun,
    ev: EventCam,
    chars: HashMap<(i32, i32), Who>,
    marks: HashMap<i16, V4>,
}

/// The scene the camera sees: the table the test set, Kite as leader.
struct Look<'a> {
    /// Kite's position and heading.
    kite: (V4, V4),
    chars: &'a HashMap<(i32, i32), Who>,
    marks: &'a HashMap<i16, V4>,
}

impl Scene for Look<'_> {
    fn char_pos(&self, ty: i32, code: i32) -> Option<V4> {
        self.chars.get(&(ty, code)).map(|w| match w {
            Who::At(p) => *p,
            Who::Kite => self.kite.0,
        })
    }
    fn leader_pos(&self) -> V4 {
        self.kite.0
    }
    fn marker_pos(&self, marker: i16) -> Option<V4> {
        self.marks.get(&marker).copied()
    }
    fn player_dirc(&self) -> V4 {
        self.kite.1
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
    let c = &r.kite.camera;
    let words = |m: &[[u32; 4]; 4]| list(&m.iter().flatten().copied().collect::<Vec<_>>());
    let mut kv = vec![
        ("ctrl", list(&ctrl_words(&r.ev.ctrl))),
        ("running", u8::from(r.ev.running()).to_string()),
        ("cam_id", c.cam_id.to_string()),
        ("normal", r.ev.normal_cam_id.to_string()),
        ("puppet", u8::from(c.puppet_show).to_string()),
        ("tcam", list(&cam_words(&c.tcam))),
        ("ecam", list(&cam_words(&c.ecam))),
        ("bcam", list(&cam_words(&c.bcam))),
        ("resetting", u8::from(c.resetting).to_string()),
        ("mem_dirc_z", c.mem_dirc_z.to_string()),
        ("mode", r.kite.mode.to_string()),
        ("world_view", words(&c.world_view)),
        ("world_screen", words(&c.world_screen)),
    ];
    kv.extend(r.kite.fields());
    let body: Vec<String> = kv.iter().map(|(k, v)| format!("\"{k}\": {v}")).collect();
    format!("{{{}}}", body.join(", "))
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
    let (data, body) = (kite_run::data(&mut iso), kite_run::body(&archive));
    let town = SceneFile::read(&archive, "town01").unwrap();
    let models = HitModel::read(volume, &Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap()).unwrap();
    let mut run: Option<Run> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let s = |i: usize| hex(w[i]) as i32;
        match w.first().copied() {
            Some("start") => {
                let mut hits = Hits::new(volume, models.clone());
                let start = Start {
                    pos: [n(1), n(2), n(3), ee::ONE],
                    dirc_z: n(4),
                    scheme: n(5) as i32,
                    mode: n(6) as i8,
                    seed: n(7),
                    area: 0,
                    arriving: true,
                };
                let kite = KiteRun::new(&data, &body, &mut hits, &start);
                run = Some(Run { hits, kite, ev: EventCam::new(), chars: HashMap::new(), marks: HashMap::new() });
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
                let name = piney_data::tables::world::of(volume).markers[m as usize];
                let obj = town.ccs.find_object(name).unwrap();
                let d = &town.scene.dummies[&obj];
                println!("{}", list(&[d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ee::ONE]));
            }
            Some("cmd") => {
                let r = run.as_mut().unwrap();
                let args: Vec<i16> = (2..w.len()).map(|i| s(i) as i16).collect();
                let c = command(n(1), &args);
                let look = Look { kite: (r.kite.pos(), r.kite.dirc()), chars: &r.chars, marks: &r.marks };
                r.ev.command(c, &mut r.kite.camera, &look);
                println!("{}", state(r));
            }
            Some("teach") => {
                let r = run.as_mut().unwrap();
                r.ev.teach(n(1) as u8);
                println!("{}", state(r));
            }
            Some("ban") => {
                let r = run.as_mut().unwrap();
                r.ev.menu_ban(n(1) != 0, &mut r.kite.camera);
                println!("{}", state(r));
            }
            Some("pad") => {
                let r = run.as_mut().unwrap();
                let pad = kite_run::pad_at(&w, 1);
                let input = Input { pad, zoom: [n(19) as u16, n(20) as u16] };
                // ccThCameraExecute (33), then ccThCamera (40) and
                // ccThPlayer (49).
                let look = Look { kite: (r.kite.pos(), r.kite.dirc()), chars: &r.chars, marks: &r.marks };
                r.ev.frame(&mut r.kite.camera, &look, &input);
                r.kite.frame(pad, &mut r.hits);
                println!("{}", state(r));
            }
            _ => {}
        }
    }
}
