//! Answers `tools/test_sound3d_rs.py`: the positioned sound effects and the
//! animation notes' sounds ([`piney_audio::se3d`]) on the cases the harness
//! runs through the game's own code, one request a line and one answer line
//! back. Floats are bit patterns in hex, other numbers decimal; a camera is
//! `-` or `CX,CY,CZ,VX,VY,VZ,KIND`. The requests name the function: `vel`,
//! `pan`, `on`, `note`, `reset`, `loop`, `off`, `spc`, `pc`, `enemy`, `inu`,
//! `treset`, `tstart`, `tloop`; the answers are the bytes that reach port 0,
//! the loop slots, and for the notes the call they end in.

use std::io::{BufRead, Write};

use piney_audio::driver::Driver;
use piney_audio::se3d::{self, Listener, NoteSe, V4};
use piney_data::sound::INF;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap()
}

fn cam(s: &str) -> Option<Listener> {
    if s == "-" {
        return None;
    }
    let w: Vec<&str> = s.split(',').collect();
    Some(Listener {
        pos: [hex(w[0]), hex(w[1]), hex(w[2]), 0],
        view: [hex(w[3]), hex(w[4]), hex(w[5]), 0],
        kind: w[6].parse().unwrap(),
    })
}

fn bytes(m: &[u8]) -> String {
    if m.is_empty() { "-".into() } else { m.iter().map(|b| format!("{b:02x}")).collect() }
}

fn ids(d: &Driver) -> String {
    d.loop_id.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn play(s: Option<NoteSe>, c: Option<&Listener>, pos: &V4) -> String {
    let Some(s) = s else { return "- - -".into() };
    let m = INF.se.get(s.code).map(|se| Driver::se_3d(se, c, pos, s.note)).unwrap_or_default();
    let note = s.note.map_or("-".into(), |k| k.to_string());
    format!("{} {} {}", s.code, note, bytes(&m))
}

fn main() {
    let mut d = Driver::new();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let int = |i: usize| w[i].parse::<i64>().unwrap();
        let pos = |i: usize| -> V4 { [hex(w[i]), hex(w[i + 1]), hex(w[i + 2]), 0] };
        let se = |i: usize| &INF.se[int(i) as usize];
        let answer = match w[0] {
            "vel" => se3d::calc_vel(se(1), cam(w[2]).as_ref(), &pos(3)).to_string(),
            "pan" => {
                let (p, c) = se3d::calc_pan(cam(w[1]).as_ref(), &pos(2));
                format!("{p} {c}")
            }
            "on" => bytes(&Driver::se_3d(se(1), cam(w[2]).as_ref(), &pos(3), None)),
            "note" => bytes(&Driver::se_3d(se(1), cam(w[3]).as_ref(), &pos(4), Some(int(2) as i8))),
            "reset" => {
                d.loop_id = [-1; 8];
                ids(&d)
            }
            "loop" => {
                let (id, m) = d.se_3d_loop(se(1), cam(w[2]).as_ref(), &pos(3));
                format!("{id} {} {}", bytes(&m), ids(&d))
            }
            "off" => {
                let m = d.se_off_loop(se(1), int(2) as i32);
                format!("{} {}", bytes(&m), ids(&d))
            }
            "treset" => {
                d.loop_id = [-1; 8];
                d.tobj_loop = false;
                d.looptest = 0;
                ids(&d)
            }
            "tstart" => {
                let m = d.tobj_se_loop_start(&INF.se[se3d::TOBJ_SE]);
                format!("{} {} {} {}", bytes(&m), ids(&d), u8::from(d.tobj_loop), d.looptest)
            }
            "tloop" => {
                let c = cam(w[1]).unwrap();
                bytes(&d.tobj_se_loop(&INF.se[se3d::TOBJ_SE], &c, &pos(2), hex(w[5])))
            }
            "spc" => {
                let s = se3d::spc_note(int(1) as u32, int(2) as i16, int(3) as u32);
                play(s, cam(w[4]).as_ref(), &pos(5))
            }
            "pc" => {
                let s = se3d::pc_note(int(1) as u32, int(2) as i32, int(3) as u32);
                play(s, cam(w[4]).as_ref(), &pos(5))
            }
            "enemy" => {
                let s = se3d::enemy_note(int(1) as u32, int(2) as i32);
                play(s, cam(w[3]).as_ref(), &pos(4))
            }
            "inu" => {
                let s = se3d::inu_note(int(1) as u32, int(2) as u32);
                play(s, cam(w[3]).as_ref(), &pos(4))
            }
            other => panic!("unknown request {other}"),
        };
        writeln!(out, "{answer}").unwrap();
    }
}
