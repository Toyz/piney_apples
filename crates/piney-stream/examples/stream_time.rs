//! Time a stream's frames: each step and its draw on the GPU (`piney-gs`,
//! headless), for finding where a stream stalls.
//!
//! ```text
//! cargo run --release -p piney-stream --example stream_time -- \
//!     [--iso work/infection/infection.iso] [--stream 4] [--top 12] \
//!     [--shots DIR FROM TO EVERY] [--dump FRAME] [--drop I,J,..]
//! ```
//!
//! The stream plays as the event instruction does (`EventStream::new`, a
//! new game's save), with the stream demo's effects. Prints the total and
//! mean step and draw times and the slowest frames of each. `--shots`
//! also writes every EVERYth scene frame from FROM to TO as `DIR/fN.png`;
//! `--dump` prints the draw commands of scene frame FRAME; `--drop` leaves
//! those of its commands out of its picture (to find which draw shows
//! what).

use std::sync::Arc;
use std::time::Instant;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::{InitText, SaveData, offset};
use piney_input::Pad;
use piney_stream::Options;
use piney_stream::event::EventStream;

/// A frame's scene frame, step and draw times (ms), commands, and the
/// textures and pipelines its draw made.
type Timed = (u32, f64, f64, usize, (usize, usize));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let (mut num, mut top) = (4usize, 12usize);
    let mut shots: Option<(String, u32, u32, u32)> = None;
    let mut dump: Option<u32> = None;
    let mut drop: Vec<usize> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--stream" => num = args.next().ok_or("--stream N")?.parse()?,
            "--top" => top = args.next().ok_or("--top N")?.parse()?,
            "--dump" => dump = Some(args.next().ok_or("--dump FRAME")?.parse()?),
            "--drop" => {
                drop = args.next().ok_or("--drop I,J")?.split(',').map(str::parse).collect::<Result<_, _>>()?;
            }
            "--shots" => {
                let mut n = || args.next().ok_or("--shots DIR FROM TO EVERY");
                let dir = n()?;
                shots = Some((dir, n()?.parse()?, n()?.parse()?, n()?.parse::<u32>()?.max(1)));
            }
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let t0 = Instant::now();
    let mut iso = Iso::open(&iso_path)?;
    let data = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let mut save = SaveData::boot(&InitText::from_disc(&mut iso)?);
    save.bytes_mut()[offset::PL_NAME..offset::PL_NAME + 4].copy_from_slice(b"Kite");
    let ms = |t: Instant| t.elapsed().as_secs_f64() * 1e3;
    let base = ms(t0);
    let t = Instant::now();
    let mut es = EventStream::new(&mut iso, &data, num, &save, Options::default(), Default::default())?;
    let stream = ms(t);
    let t = Instant::now();
    es.stream_mut().set_effects(piney_effect::StreamEffects::new(&data, iso.volume()?)?);
    println!(
        "loaded in {:.1} ms: the disc, executable and DATA.BIN {base:.1}, the stream {stream:.1}, its effects {:.1}",
        ms(t0),
        ms(t)
    );
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(data))?;
    gs.set_overlay(Some(es.stream().archive()));
    let pad = Pad::default();
    let mut times: Vec<Timed> = Vec::new();
    while !es.done() {
        let t = Instant::now();
        let mut frame = es.step(&pad);
        let step = t.elapsed().as_secs_f64() * 1e3;
        es.take_requests();
        if dump == Some(es.stream().frame()) && !drop.is_empty() {
            let mut i = 0;
            frame.cmds.retain(|_| {
                i += 1;
                !drop.contains(&(i - 1))
            });
        }
        let t = Instant::now();
        let before = gs.counts();
        gs.render(&frame);
        let rgba = gs.read_back();
        let draw = t.elapsed().as_secs_f64() * 1e3;
        let at = es.stream().frame();
        if dump == Some(at) {
            for (i, c) in frame.cmds.iter().enumerate() {
                match c {
                    piney_draw::Cmd::Prim(p) => println!("{i:4} prim {:?} n {} {:?}", p.kind, p.verts.len(), p.state),
                    piney_draw::Cmd::Model(m) => println!(
                        "{i:4} model {} {:#x} mmats {} nodes {} morph {:?} edits {:?} {:?} to_screen {:?}",
                        m.file,
                        m.model,
                        m.mmats.len(),
                        m.nodes.len(),
                        m.morph,
                        m.edits.as_ref().map(|e| (&e.tex, e.st.len())),
                        m.state,
                        m.to_screen
                    ),
                    piney_draw::Cmd::Shadow(_) => println!("{i:4} shadow"),
                }
            }
        }
        times.push((at, step, draw, frame.cmds.len(), (gs.counts().0 - before.0, gs.counts().1 - before.1)));
        if let Some((dir, from, to, every)) = &shots
            && (*from..=*to).contains(&at)
            && (at - from) % every == 0
        {
            let (w, h) = gs.target_size();
            std::fs::write(format!("{dir}/f{at}.png"), piney_gs::png::encode(w, h, &rgba))?;
        }
    }
    let n = times.len().max(1) as f64;
    let (ts, td): (f64, f64) = times.iter().fold((0.0, 0.0), |a, t| (a.0 + t.1, a.1 + t.2));
    println!("{} frames: step {:.1} ms ({:.2} mean), draw {:.1} ms ({:.2} mean)", times.len(), ts, ts / n, td, td / n);
    let mut by_step = times.clone();
    by_step.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("slowest steps (scene frame, step ms, draw ms, commands, new textures and pipelines):");
    for t in by_step.iter().take(top) {
        println!("  {:5} {:8.2} {:8.2} {:6} {:4} {:4}", t.0, t.1, t.2, t.3, t.4.0, t.4.1);
    }
    let mut by_draw = times;
    by_draw.sort_by(|a, b| b.2.total_cmp(&a.2));
    println!("slowest draws:");
    for t in by_draw.iter().take(top) {
        println!("  {:5} {:8.2} {:8.2} {:6} {:4} {:4}", t.0, t.1, t.2, t.3, t.4.0, t.4.1);
    }
    Ok(())
}
