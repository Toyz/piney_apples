//! Enter Mac Anu as a new game does and write a frame as a PNG:
//! `world_shot [--iso ISO] [--frames 120] [--walk N] [--turn N] [--steps
//! N,LX,LY,RX,RY[,BUTTONS];...] [--out world.png] [--soft] [--info] [--entry
//! TYPE,CODE,MARKER,PARAM;...]`. After the arrival the sticks walk or turn
//! (or `--steps` holds them, 0-255, and buttons, hex); the frame after
//! `--frames` is drawn by `piney-gs` on the GPU, or with `--soft` by the CPU
//! GS (no skinned models). `--entry` places characters as an event's `entry`
//! does; `--info` prints what the action button asked for.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::soft::{self, Canvas};
use piney_draw::Frame;
use piney_input::{Pad, Raw};
use piney_world::{SaveState, World, ee};

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

/// A new game's save as the title leaves it: `ccSaveData::Init` as far as
/// `SaveState::fresh` goes, then `NewGame(0)` (the characters' `charTbl`
/// rows, Kite's velocity among them).
fn new_game(iso: &mut Iso) -> Result<SaveState, Box<dyn std::error::Error>> {
    let tables = piney_demo::newgame::NewGameTables::of(iso.volume()?);
    let mut state = SaveState::fresh();
    piney_demo::newgame::new_game(&mut state.save, 0, &tables, piney_demo::newgame::SAVE_VA);
    Ok(state)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let (mut frames, mut walk, mut turn) = (120u32, 0u32, 0u32);
    let mut out = "world.png".to_string();
    let (mut soft_only, mut info) = (false, false);
    let mut steps: Vec<(u32, Raw)> = Vec::new();
    let mut entries: Vec<[i16; 4]> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--frames" => frames = args.next().ok_or("--frames N")?.parse()?,
            "--walk" => walk = args.next().ok_or("--walk N")?.parse()?,
            "--turn" => turn = args.next().ok_or("--turn N")?.parse()?,
            "--out" => out = args.next().ok_or("--out FILE")?,
            "--steps" => {
                for step in args.next().ok_or("--steps N,LX,LY,RX,RY[,BUTTONS];...")?.split(';') {
                    let v: Vec<&str> = step.split(',').map(str::trim).collect();
                    let n = |i: usize| -> Result<u8, Box<dyn std::error::Error>> {
                        Ok(v.get(i).ok_or("short step")?.parse()?)
                    };
                    let buttons = match v.get(5) {
                        Some(b) => u32::from_str_radix(b.trim_start_matches("0x"), 16)?,
                        None => 0,
                    };
                    let raw = Raw {
                        lx: n(1)?,
                        ly: n(2)?,
                        rx: n(3)?,
                        ry: n(4)?,
                        buttons: piney_input::Buttons(buttons),
                        ..Raw::default()
                    };
                    steps.push((v[0].parse()?, raw));
                }
            }
            "--entry" => {
                for e in args.next().ok_or("--entry TYPE,CODE,MARKER,PARAM;...")?.split(';') {
                    let v: Vec<i16> = e.split(',').map(|x| x.trim().parse()).collect::<Result<_, _>>()?;
                    entries.push(v.try_into().map_err(|_| "--entry takes 4 numbers")?);
                }
            }
            "--soft" => soft_only = true,
            "--info" => info = true,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let mut iso = Iso::open(&iso_path)?;
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let save = new_game(&mut iso)?;
    let mut world = World::enter(&mut iso, archive.clone(), save)?;
    for [t, c, m, p] in entries {
        if !world.entry(t, c, m, p) {
            eprintln!("entry {t} {c} {m} {p}: not placed");
        }
    }
    if info {
        let party = world.town_party();
        for (code, _) in party.members() {
            let Some((pos, dirc)) = party.place(code) else { continue };
            let (p, d) = (pos.map(ee::f), ee::f(dirc[2]));
            let anim = party.actor(code).and_then(|a| a.ch.anim_name().map(str::to_string));
            println!("fellow {code} at ({:.1}, {:.1}, {:.1}) dirc {d:.3} anim {anim:?}", p[0], p[1], p[2]);
        }
    }
    let mut pad = Pad::default();
    let mut frame = Frame::new();
    // The arrival: 10 + 2 frames of fade out and hold, then 74 frames of the
    // fade-in act.
    let free = 90;
    for i in 0..=frames {
        let mut raw = Raw::default();
        if i >= free && i < free + walk {
            raw.ly = 0;
        }
        if i >= free && i < free + turn {
            raw.rx = 255;
            raw.ry = 129;
        }
        let mut at = free;
        for (n, r) in &steps {
            if (at..at + n).contains(&i) {
                raw = *r;
            }
            at += n;
        }
        pad.read(&raw);
        frame = world.step(&pad);
        for t in world.take_talk() {
            if info {
                println!("{i:4} talk {t:?}");
            }
            world.close_menu();
        }
        if info && (i % 10 == 0 || i == frames) {
            let p = world.player();
            let c = world.camera();
            println!(
                "{i:4} {:?} mv {:?} spd {} act {:2} pos ({:.1}, {:.1}, {:.1}) dirc {:.3} cam ({:.1}, {:.1}, {:.1}) deg {:?} dist {:.1} cmds {}",
                world.phase(),
                p.body.move_pos.map(ee::f),
                ee::f(p.body.now_speed),
                p.acts.act,
                ee::f(p.body.pos[0]),
                ee::f(p.body.pos[1]),
                ee::f(p.body.pos[2]),
                ee::f(p.body.dirc[2]),
                ee::f(c.tcam.pos[0]),
                ee::f(c.tcam.pos[1]),
                ee::f(c.tcam.pos[2]),
                c.tcam.deg,
                ee::f(c.tcam.dist),
                frame.cmds.len()
            );
        }
    }
    let (w, h, rgba) = if soft_only {
        render_soft(archive, &frame)
    } else {
        match render_gpu(archive.clone(), &frame) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("no GPU ({e}); drawing on the CPU");
                render_soft(archive, &frame)
            }
        }
    };
    std::fs::write(&out, piney_gs::png::encode(w, h, &rgba))?;
    println!("{out}: {w}x{h}, {} commands", frame.cmds.len());
    Ok(())
}
