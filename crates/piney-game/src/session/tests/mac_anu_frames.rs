//! Issue #77: frames near Mac Anu's bridge and weapon shop. Kite walked
//! from the gate to each merchant in turn and back, every frame's step
//! and draw timed (the draw's encode apart), by place: a diagnostic,
//! `PINEY_FRAMES_MS=M` printing the frames slower than M ms (default 8).

use super::town_walk::TownWalker;
use super::*;

/// A new game on Infection logged in to Mac Anu.
fn in_mac_anu() -> Option<Session> {
    let iso = crate::check_image("infection");
    if !iso.exists() {
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 0);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    Some(Session::in_world(iso, archive, None, state, None, scene, None).unwrap())
}

/// The frames spent in one 1000-unit square of the town: run-time
/// textures uploaded, the most vertices, the draw's encode and the step's
/// time (sum, most).
#[derive(Default)]
struct Cell {
    frames: u32,
    uploads: usize,
    verts: usize,
    encode: (f64, f64),
    step: (f64, f64),
}

#[test]
#[ignore]
fn mac_anu_frame_times() {
    let Some(mut s) = in_mac_anu() else { return };
    let mut gs = match piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())) {
        Ok(gs) => gs,
        Err(e) => return eprintln!("no GPU ({e}); skipped"),
    };
    let audio = piney_audio::Audio::headless(&s.iso).ok();
    let mut buf = vec![0i16; 2 * 800];
    let mut sounds = Vec::new();
    let slow = std::env::var("PINEY_FRAMES_MS").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(8.0);
    let mut pad = Pad::default();
    let still = piney_input::Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Default::default() };
    let (mut steps, mut draws) = (Vec::new(), Vec::new());
    let mut cells: std::collections::BTreeMap<(i32, i32), Cell> = Default::default();
    let mut targets: Vec<[f32; 2]> = Vec::new();
    let mut walk: Option<TownWalker> = None;
    for f in 0..12000u32 {
        let raw = match &s.stage {
            Stage::World(w) if f > 200 => {
                let world = w.world();
                if targets.is_empty() && walk.is_none() {
                    // Round the merchants three times, back to the start.
                    let p = world.player().body.pos.map(f32::from_bits);
                    for _ in 0..3 {
                        targets.extend(
                            world
                                .merchants()
                                .iter()
                                .map(|m| [f32::from_bits(m.ch.pos[0]), f32::from_bits(m.ch.pos[1])]),
                        );
                        targets.push([p[0], p[1]]);
                    }
                    targets.reverse();
                    println!("{} stops", targets.len());
                }
                let w_ = walk.get_or_insert_with(|| TownWalker::new(targets.pop().unwrap_or([0.0, 0.0]), 300.0));
                match w_.stick(world, f) {
                    Some(r) => r,
                    None => {
                        walk = None;
                        if targets.is_empty() {
                            println!("walked at frame {f}");
                            break;
                        }
                        still
                    }
                }
            }
            _ => still,
        };
        pad.read(&raw);
        let t = std::time::Instant::now();
        let frame = s.step(&pad);
        let step_ms = t.elapsed().as_secs_f64() * 1000.0;
        let events = s.take_events();
        let t = std::time::Instant::now();
        crate::handle(events, audio.as_ref());
        if let Some(a) = &audio {
            a.frame();
            a.render(&mut buf);
        }
        let sound_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        gs.set_overlay(Mode::archive(&s));
        gs.render(&frame);
        let encode_ms = t.elapsed().as_secs_f64() * 1000.0;
        let _ = gs.read_back();
        let draw_ms = t.elapsed().as_secs_f64() * 1000.0;
        let (at, pcs) = match &s.stage {
            Stage::World(w) => (
                w.world().player().body.pos.map(|v| f32::from_bits(v) as i32),
                w.world().pcs().iter().filter(|c| c.disp_sw).count(),
            ),
            _ => ([0; 4], 0),
        };
        if step_ms > slow || draw_ms > slow || sound_ms > slow {
            println!(
                "frame {f}: step {step_ms:.1} ms sound {sound_ms:.1} ms draw {draw_ms:.1} ms, {} cmds, {} verts, {pcs} PCs shown, Kite at {:?} | {}",
                frame.cmds.len(),
                gs.vertices(),
                &at[..3],
                Mode::title(&s)
            );
        }
        let e = cells.entry((at[0] / 1000, at[1] / 1000)).or_default();
        e.frames += 1;
        e.uploads += frame.uploads.len();
        e.verts = e.verts.max(gs.vertices());
        e.encode = (e.encode.0 + encode_ms, e.encode.1.max(encode_ms));
        e.step = (e.step.0 + step_ms, e.step.1.max(step_ms));
        if f > 200 {
            sounds.push(sound_ms);
            steps.push(step_ms);
            draws.push(draw_ms);
        }
    }
    let stat = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        let n = v.len().max(1);
        (v[n / 2], v[n * 99 / 100], v.last().copied().unwrap_or(0.0))
    };
    for ((x, y), c) in &cells {
        let n = f64::from(c.frames);
        println!(
            "near ({}, {}): {} frames, {:.2} uploads, {} verts at most, encode {:.2}/{:.2} ms, step {:.2}/{:.2} ms",
            x * 1000,
            y * 1000,
            c.frames,
            c.uploads as f64 / n,
            c.verts,
            c.encode.0 / n,
            c.encode.1,
            c.step.0 / n,
            c.step.1
        );
    }
    println!(
        "{} frames; step median/p99/max {:?}; sound {:?}; draw {:?}",
        steps.len(),
        stat(&mut steps),
        stat(&mut sounds),
        stat(&mut draws)
    );
}
