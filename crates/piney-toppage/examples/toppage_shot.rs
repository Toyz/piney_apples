//! Run the top page headless for some frames and write the last one as a PNG,
//! drawn by the desktop crate's CPU GS (`piney_desktop::soft`). `--post T:P`
//! posts message P of thread T as `bbs_post` does (state 1), `--write` as
//! `bbs_post7` (7), `--read` marks it read (3); `--press F:BUTTON` and `--hold
//! F-G:BUTTON` press and hold buttons; `--info` prints the scene's animations
//! and the board's sizes. `--iso`, `--frames`, `--out`, `--parody` and
//! `--dump` are the rest.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::SaveState;
use piney_desktop::soft::{self, Canvas};
use piney_draw::Cmd;
use piney_input::{Buttons, Pad, Raw};
use piney_toppage::TopPage;
use piney_toppage::assets::Assets;
use piney_toppage::bbs::set_bbs_state;

fn button(name: &str) -> Option<Buttons> {
    Some(match name {
        "up" => Buttons::UP,
        "down" => Buttons::DOWN,
        "left" => Buttons::LEFT,
        "right" => Buttons::RIGHT,
        "cross" | "x" => Buttons::CROSS,
        "circle" | "o" => Buttons::CIRCLE,
        "square" => Buttons::SQUARE,
        "triangle" => Buttons::TRIANGLE,
        "start" => Buttons::START,
        "select" => Buttons::SELECT,
        _ => return None,
    })
}

fn posts(v: &str) -> Result<Vec<(i32, i32)>, Box<dyn std::error::Error>> {
    v.split(',')
        .map(|p| {
            let (t, m) = p.split_once(':').ok_or("T:P")?;
            Ok((t.parse()?, m.parse()?))
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let mut frames = 300usize;
    let mut out = "toppage.png".to_string();
    let mut presses: Vec<(usize, Buttons)> = Vec::new();
    let mut holds: Vec<(usize, usize, Buttons)> = Vec::new();
    let mut states: Vec<(i32, i32, i8)> = Vec::new();
    let (mut parody, mut info, mut dump) = (false, false, false);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--frames" => frames = args.next().ok_or("--frames N")?.parse()?,
            "--out" => out = args.next().ok_or("--out FILE")?,
            "--parody" => parody = true,
            "--info" => info = true,
            "--dump" => dump = true,
            "--post" | "--write" | "--read" => {
                let v = match a.as_str() {
                    "--post" => 1,
                    "--write" => 7,
                    _ => 3,
                };
                for (t, p) in posts(&args.next().ok_or("T:P,...")?)? {
                    states.push((t, p, v));
                }
            }
            "--hold" => {
                for p in args.next().ok_or("--hold F-G:BUTTON,...")?.split(',') {
                    let (r, b) = p.split_once(':').ok_or("F-G:BUTTON")?;
                    let (f, g) = r.split_once('-').ok_or("F-G")?;
                    holds.push((f.parse()?, g.parse()?, button(b).ok_or_else(|| format!("no button {b}"))?));
                }
            }
            "--press" => {
                for p in args.next().ok_or("--press F:BUTTON,...")?.split(',') {
                    let (f, b) = p.split_once(':').ok_or("F:BUTTON")?;
                    presses.push((f.parse()?, button(b).ok_or_else(|| format!("no button {b}"))?));
                }
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let mut iso = Iso::open(&iso_path)?;
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    if info {
        let a = Assets::read(&mut iso, archive.clone())?;
        let f = &a.bbs.file;
        for an in &f.anims {
            let name = f.ccs.object_name(an.object).unwrap_or("?");
            println!("{name}: {} frames, looping {}", an.frames, an.looping);
        }
        println!("volume {}, idle {}, mask {:?}", a.volume, a.neutral, a.bbs.mask_tex);
        for (k, tbl) in a.bbs.tables.iter().enumerate() {
            let posts: usize = tbl.iter().map(|t| t.msgs.len()).sum();
            let most = tbl.iter().map(|t| t.msgs.len()).max().unwrap_or(0);
            let lines = tbl.iter().flat_map(|t| t.msgs.iter().map(|m| m.max_lines)).max().unwrap_or(0);
            let exact = tbl.iter().flat_map(|t| t.msgs.iter()).filter(|m| m.max_lines == 7).count();
            let six = tbl.iter().filter(|t| t.msgs.len() == 6).count();
            let n = tbl.len();
            println!(
                "table {k}: {n} threads, {posts} posts, at most {most} a thread, at most {lines} lines; \
                 {exact} posts of 7 lines, {six} threads of 6 posts"
            );
        }
        println!("servers {:?}", a.servers);
    }
    let mut state = SaveState::fresh();
    if parody {
        state.save.set_u8(piney_data::save::offset::PARODY_FLAG, 1);
    }
    for &(t, p, v) in &states {
        set_bbs_state(&mut state, t, p, v);
    }
    let mut d = TopPage::new(&mut iso, archive.clone(), state)?;
    let mut pad = Pad::default();
    let mut last = None;
    for f in 0..frames {
        let mut raw = Raw { analog: false, ..Raw::default() };
        for &(pf, b) in &presses {
            if pf == f {
                raw.buttons |= b;
            }
        }
        for &(pf, pg, b) in &holds {
            if (pf..=pg).contains(&f) {
                raw.buttons |= b;
            }
        }
        pad.read(&raw);
        let frame = d.step(&pad);
        let req = d.take_requests();
        if dump || !req.is_empty() {
            let models = frame.cmds.iter().filter(|c| matches!(c, Cmd::Model(_))).count();
            println!(
                "frame {f}: {} cmds ({models} models), {} uploads, {} {req:?}",
                frame.cmds.len(),
                frame.uploads.len(),
                d.status()
            );
        }
        last = Some(frame);
    }
    let frame = last.ok_or("no frames")?;
    let mut assets = soft::Assets::new(archive);
    let mut canvas = Canvas::new(frame.width as usize, frame.height as usize, frame.clear);
    canvas.draw(&frame, &mut assets);
    std::fs::write(&out, soft::png(frame.width as u32, frame.height as u32, &canvas.rgba()))?;
    println!("{frames} frames, {} -> {out}", d.status());
    Ok(())
}
