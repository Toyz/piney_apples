//! Run the desktop headless for some frames and write the last one as a PNG,
//! drawn by the crate's CPU GS (`piney_desktop::soft`).
//!
//! ```text
//! cargo run --release -p piney-desktop --example desktop_shot -- \
//!     [--iso work/infection/infection.iso] [--frames 400] [--out desktop.png] \
//!     [--press 300:down,420:cross] [--hold 430-470:down] [--mail 4,5,320] \
//!     [--news 0,1,2] [--walls 0,5,9] [--bgm 0,1] [--movies 0,1] \
//!     [--say F:NAME:LINE/LINE] [--dump]
//! ```
//!
//! `--mail` delivers mails as event 1 does (`ccSaveData::NewMail`) and
//! `--news` posts headlines as event opcode 111 does (`webnewsList[n] = 1`)
//! before the desktop starts; `--walls`, `--bgm` and `--movies` unlock
//! wallpapers, music and movies (`dtWallpaperList`, `dtBgmList`,
//! `dtStrList` bits, as event opcode 167 sets them); `--press F:BUTTON` presses a button on frame F
//! and `--hold F-G:BUTTON` holds it from frame F to G (up, down, left, right,
//! cross, circle, square, triangle, start, select). `--say` opens an event
//! speech window at frame F as `message` does on the desktop (NAME may be
//! empty; lines split by `/`), polls it each frame from five frames on, and
//! says the line is done when it closes.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::message::MessageKind;
use piney_desktop::save::offset;
use piney_desktop::soft::{self, Canvas};
use piney_desktop::{Desktop, SaveState};
use piney_draw::Cmd;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let mut frames = 400usize;
    let mut out = "desktop.png".to_string();
    let mut presses: Vec<(usize, Buttons)> = Vec::new();
    let mut holds: Vec<(usize, usize, Buttons)> = Vec::new();
    let mut mails: Vec<usize> = Vec::new();
    let mut news: Vec<usize> = Vec::new();
    let mut unlocks: Vec<(usize, usize)> = Vec::new();
    let mut says: Vec<(usize, String, Vec<String>)> = Vec::new();
    let mut dump = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--frames" => frames = args.next().ok_or("--frames N")?.parse()?,
            "--out" => out = args.next().ok_or("--out FILE")?,
            "--dump" => dump = true,
            "--mail" => {
                for n in args.next().ok_or("--mail N,N")?.split(',') {
                    mails.push(n.parse()?);
                }
            }
            "--news" => {
                for n in args.next().ok_or("--news N,N")?.split(',') {
                    news.push(n.parse()?);
                }
            }
            "--walls" | "--bgm" | "--movies" => {
                let base = match a.as_str() {
                    "--walls" => offset::DT_WALLPAPER_LIST,
                    "--bgm" => offset::DT_BGM_LIST,
                    _ => offset::DT_STR_LIST,
                };
                for n in args.next().ok_or("--walls N,N")?.split(',') {
                    unlocks.push((base, n.parse()?));
                }
            }
            "--say" => {
                let v = args.next().ok_or("--say F:NAME:LINE/LINE")?;
                let mut parts = v.splitn(3, ':');
                let f = parts.next().ok_or("F")?.parse()?;
                let name = parts.next().unwrap_or("").to_string();
                let lines = parts.next().unwrap_or("").split('/').map(str::to_string).collect();
                says.push((f, name, lines));
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
    let mut state = SaveState::fresh();
    for &m in &mails {
        state.save.new_mail(m);
    }
    for &n in &news {
        state.save.set_webnews(n, 1);
    }
    for &(base, k) in &unlocks {
        let at = base + 4 * (k >> 5);
        let v = state.save.i32(at) | (1 << (k & 31));
        state.save.set_i32(at, v);
    }
    let mut d = Desktop::new(&mut iso, archive.clone(), state)?;
    let mut pad = Pad::default();
    let mut last = None;
    let mut waiting: Option<usize> = None;
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
        // The event side of `message` on the desktop, after the frame.
        for (sf, name, lines) in &says {
            if f == *sf {
                let ls: Vec<&[u8]> = lines.iter().map(|l| l.as_bytes()).collect();
                let n = (!name.is_empty()).then_some(name.as_bytes());
                d.open_message(MessageKind::Speech, 0, n, &ls);
            }
            if f >= sf + 5 && waiting == Some(*sf) && d.message_check(&pad) != 0 {
                d.message_done();
                waiting = None;
            }
        }
        if says.iter().any(|(sf, ..)| *sf == f) {
            waiting = Some(f);
        }
        let req = d.take_requests();
        if dump || !req.is_empty() {
            let models = frame.cmds.iter().filter(|c| matches!(c, Cmd::Model(_))).count();
            if dump {
                println!(
                    "frame {f}: {} cmds ({models} models), {} uploads, {req:?}",
                    frame.cmds.len(),
                    frame.uploads.len()
                );
            } else {
                println!("frame {f}: {req:?}");
            }
        }
        last = Some(frame);
    }
    let frame = last.ok_or("no frames")?;
    let mut assets = soft::Assets::new(archive);
    let mut canvas = Canvas::new(frame.width as usize, frame.height as usize, frame.clear);
    canvas.draw(&frame, &mut assets);
    std::fs::write(&out, soft::png(frame.width as u32, frame.height as u32, &canvas.rgba()))?;
    println!("{} frames, mode {:?}, icon {} -> {out}", frames, d.mode(), d.selection());
    Ok(())
}
