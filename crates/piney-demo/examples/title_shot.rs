//! Run the title headless for some frames and write the last one as a PNG.
//!
//! ```text
//! cargo run --release -p piney-demo --example title_shot -- \
//!     [--iso work/infection/infection.iso] [--frames 200] [--out title.png] \
//!     [--press 150:down,170:cross] [--hold 150-200:down] [--soft] \
//!     [--no-card | --card DIR] [--reset] [--parody-item] [--skip-movies] [--dump]
//! ```
//!
//! The title's movies and stream are the runtime's; here they count as
//! played (or skipped with `--skip-movies`) at once, so the menu is up a
//! few hundred frames in: the logos take 4 steps and 60 frames of wait,
//! the stream 2. `--reset` starts as after a soft reset (no logos).
//! `--no-card` answers the memory-card check with an empty slot; `--card
//! DIR` puts the card kept as files in DIR (as the desktop saves it) in
//! MEMORY CARD slot 1.
//!
//! The frame is drawn by `piney-gs` on the GPU when one is there (the icons
//! are lit, which only it draws), else, or with `--soft`, by the CPU GS in
//! `piney_desktop::soft`.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_demo::{Config, Demo, Request};
use piney_desktop::card::{FilesCard, NoCard};
use piney_desktop::soft::{self, Canvas};
use piney_draw::{Cmd, Frame};
use piney_input::{Buttons, Pad, Raw};

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

fn render_gpu(archive: Arc<Archive>, frame: &Frame) -> Result<(u32, u32, Vec<u8>), String> {
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive))?;
    gs.render(frame);
    let (w, h) = gs.target_size();
    Ok((w, h, gs.read_back()))
}

fn render_soft(archive: Arc<Archive>, frame: &Frame) -> (u32, u32, Vec<u8>) {
    let mut assets = soft::Assets::new(archive);
    let mut canvas = Canvas::new(frame.width as usize, frame.height as usize, frame.clear);
    canvas.draw(frame, &mut assets);
    (frame.width as u32, frame.height as u32, canvas.rgba())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let mut card_dir: Option<String> = None;
    let mut frames = 200usize;
    let mut out = "title.png".to_string();
    let mut presses: Vec<(usize, Buttons)> = Vec::new();
    let mut holds: Vec<(usize, usize, Buttons)> = Vec::new();
    let mut config = Config::default();
    let (mut soft_only, mut skip, mut dump) = (false, false, false);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--frames" => frames = args.next().ok_or("--frames N")?.parse()?,
            "--out" => out = args.next().ok_or("--out FILE")?,
            "--soft" => soft_only = true,
            "--no-card" => config.card = Box::new(NoCard),
            "--card" => card_dir = Some(args.next().ok_or("--card DIR")?),
            "--reset" => config.first_boot = false,
            "--parody-item" => config.parody_item = true,
            "--skip-movies" => skip = true,
            "--dump" => dump = true,
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
    if let Some(dir) = card_dir {
        config.card = Box::new(FilesCard::slot1(iso.volume()?, dir));
    }
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let mut d = Demo::new(&mut iso, archive.clone(), config)?;
    let mut pad = Pad::default();
    let mut last = None;
    for f in 0..frames {
        let mut raw = Raw::default();
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
        if skip && req.iter().any(|r| matches!(r, Request::Movie { .. })) {
            d.movie_skipped();
        }
        if dump || !req.is_empty() {
            let models = frame.cmds.iter().filter(|c| matches!(c, Cmd::Model(_))).count();
            println!("frame {f} {:?}: {} cmds ({models} models) {req:?}", d.phase(), frame.cmds.len());
        }
        last = Some(frame);
    }
    let mut frame = last.ok_or("no frames")?;
    if let Ok(drop) = std::env::var("TITLE_DROP") {
        let file = d.file().clone();
        frame.cmds.retain(|c| match c {
            Cmd::Model(m) => !file.ccs.object_name(m.model).unwrap_or("").contains(drop.as_str()),
            _ => true,
        });
    }
    if std::env::var("TITLE_LIST").is_ok() {
        let file = d.file().clone();
        for c in &frame.cmds {
            if let Cmd::Model(m) = c {
                let t: Vec<String> = m.mmats.iter().map(|x| format!("{:.2}", x.alpha)).collect();
                println!("{} {:?} lit {}", file.ccs.object_name(m.model).unwrap_or("?"), t, m.lights.is_some());
            }
        }
    }
    let (w, h, rgba) = if soft_only {
        render_soft(archive, &frame)
    } else {
        match render_gpu(archive.clone(), &frame) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("no GPU ({e}); drawing on the CPU, the lit icons unlit");
                render_soft(archive, &frame)
            }
        }
    };
    std::fs::write(&out, piney_gs::png::encode(w, h, &rgba))?;
    println!("{frames} frames, {:?}, cursor {} -> {out}", d.phase(), d.cursor());
    Ok(())
}
