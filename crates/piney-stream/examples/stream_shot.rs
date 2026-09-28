//! Play a stream headless to a frame and write it as a PNG.
//!
//! ```text
//! cargo run --release -p piney-stream --example stream_shot -- \
//!     [--iso work/infection/infection.iso] [--stream 0] [--frame 200] \
//!     [--out stream.png] [--english] [--soft] [--info] [--event] [--banner]
//!     [--fx] [--no-shadows]
//! ```
//!
//! `--frame N` stops on the step that draws scene frame N (or the last
//! step when the stream ends first). The picture is drawn by `piney-gs` on
//! the GPU when there is one (it lights and skins models), else, or with
//! `--soft`, by the CPU GS in `piney_desktop::soft`. The steps before it are
//! drawn too, [`LEAD_IN`] of them, for effects that feed on the previous
//! frame (stream 2's feedback). `--info` prints the stream's files, scenes
//! and what it asked for. `--event` plays it as the event instruction does
//! (`ccEventStream(num, 1)`, `piney_stream::event::EventStream`): with its
//! subtitles, for a new game's save (Movie Text on, the player Kite); the
//! window's texture and fonts come from `DATA.BIN`, drawn on the GPU.
//! `--banner` gives the member-drained stream (20) its skill name banner
//! (`Options::skill_names`, from `DATA.BIN`), as the game running does.
//! `--fx` gives it the stream demo's effects (`Stream::set_effects`), which
//! draw the effect tasks' hit marks and transfers. `--no-shadows` leaves
//! the shadow packets' passes out (to see what they darken); `--info` then
//! counts what each pass holds.

use std::collections::VecDeque;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::{InitText, SaveData, offset};
use piney_desktop::soft::{self, Canvas};
use piney_draw::Frame;
use piney_input::Pad;
use piney_stream::event::EventStream;
use piney_stream::{Options, Request, SkillNames, Stream};

/// Frames drawn before the one shot: stream 2's feedback keeps 0x60 / 0x80
/// of the last frame, so 16 frames bring it within half a percent.
const LEAD_IN: usize = 16;

/// Draw `frames` in order, the last one kept; `data` (`DATA.BIN`) under
/// the stream's own archive when given.
fn render_gpu(
    archive: Arc<Archive>,
    data: Option<Arc<Archive>>,
    frames: &[Frame],
) -> Result<(u32, u32, Vec<u8>), String> {
    let mut gs = match data {
        Some(d) => {
            let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(d))?;
            gs.set_overlay(Some(archive));
            gs
        }
        None => piney_gs::Gs::headless(piney_gs::Assets::new(archive))?,
    };
    for f in frames {
        gs.render(f);
    }
    let (w, h) = gs.target_size();
    Ok((w, h, gs.read_back()))
}

fn render_soft(archive: Arc<Archive>, frames: &[Frame]) -> (u32, u32, Vec<u8>) {
    let mut assets = soft::Assets::new(archive);
    let last = frames.last().cloned().unwrap_or_else(Frame::new);
    let mut canvas = Canvas::new(last.width as usize, last.height as usize, last.clear);
    for (i, f) in frames.iter().enumerate() {
        if i > 0 {
            canvas.next_frame(f.clear);
        }
        canvas.draw(f, &mut assets);
    }
    (last.width as u32, last.height as u32, canvas.rgba())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let (mut num, mut target) = (0usize, 200u32);
    let mut out = "stream.png".to_string();
    let (mut soft_only, mut info, mut event, mut banner, mut fx) = (false, false, false, false, false);
    let mut no_shadows = false;
    let mut opts = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--stream" => num = args.next().ok_or("--stream N")?.parse()?,
            "--frame" => target = args.next().ok_or("--frame N")?.parse()?,
            "--out" => out = args.next().ok_or("--out FILE")?,
            "--english" => opts.english = true,
            "--soft" => soft_only = true,
            "--info" => info = true,
            "--event" => event = true,
            "--banner" => banner = true,
            "--fx" => fx = true,
            "--no-shadows" => no_shadows = true,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let mut iso = Iso::open(&iso_path)?;
    let mut data = None;
    if banner {
        let d = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
        let w = piney_desktop::message::WindowTexture::read_named(&d, SkillNames::FILE, SkillNames::TEXTURE, None)
            .ok_or("no TEX_detadrain in DATA.BIN")?;
        if let piney_draw::TexRef::Ccs { texture, clut, .. } = w.tex {
            opts.skill_names = Some(SkillNames { texture, clut, tex_h: w.tex_h });
        }
        data = Some(d);
    }
    let mut es = if event {
        let d = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
        let mut save = SaveData::boot(&InitText::from_disc(&mut iso)?);
        save.bytes_mut()[offset::PL_NAME..offset::PL_NAME + 4].copy_from_slice(b"Kite");
        let es = EventStream::new(&mut iso, &d, num, &save, opts, Default::default())?;
        data = Some(d);
        es
    } else {
        EventStream::with_subtitles(Stream::with_options(&mut iso, num, opts)?, None)
    };
    if fx {
        let d = match &data {
            Some(d) => d.clone(),
            None => Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?),
        };
        es.stream_mut().set_effects(piney_effect::StreamEffects::new(&d, iso.volume()?)?);
        data = Some(d);
    }
    if info {
        let s = es.stream();
        let d = s.def();
        println!("stream {num}: {} archive {:?}", d.header.name, d.archive());
        for e in &d.entries {
            println!("  {:12} type {:2} flag {:3} size {:9} inflated {:9}", e.name, e.kind, e.flag, e.size, e.gzip);
        }
        for f in &s.loaded().files {
            println!(
                "  loaded {:12} frames {:5} clumps {:4} cameras {} lights {:?}",
                f.stem,
                f.setup.frames,
                f.setup.clumps.len(),
                f.setup.cameras.len(),
                f.setup.lights
            );
        }
    }
    let pad = Pad::default();
    let mut frames: VecDeque<Frame> = VecDeque::new();
    let mut samples = 0usize;
    let mut other = Vec::new();
    while !es.done() {
        if frames.len() > LEAD_IN {
            frames.pop_front();
        }
        frames.push_back(es.step(&pad));
        for r in es.take_requests() {
            match r {
                Request::Pcm(v) => samples += v.len() / 2,
                r => other.push((es.stream().steps(), r)),
            }
        }
        if es.stream().frame() >= target {
            break;
        }
    }
    let s = es.stream();
    let mut frames: Vec<Frame> = frames.into();
    if info && let Some(f) = frames.last() {
        for c in &f.cmds {
            if let piney_draw::Cmd::Shadow(p) = c {
                let polys: Vec<usize> = p.groups.iter().map(|g| g.polys.len()).collect();
                let alphas: Vec<u8> = p.groups.iter().map(|g| g.alpha).collect();
                println!("  shadow pass {}x{}: groups {alphas:?}, polygons {polys:?}", p.width, p.height);
            }
        }
    }
    if no_shadows {
        for f in &mut frames {
            f.cmds.retain(|c| !matches!(c, piney_draw::Cmd::Shadow(_)));
        }
    }
    if info {
        println!("  stopped at step {} drawing frame {}; {} PCM samples queued", s.steps(), s.frame(), samples);
        for (step, r) in other {
            println!("  step {step}: {r:?}");
        }
        let n = frames.last().map_or(0, |f| f.cmds.len());
        println!("  {n} draw commands, {} of them the effect task's", s.effect_draws().len());
        for c in frames.last().map(|f| f.cmds.as_slice()).unwrap_or_default() {
            if let piney_draw::Cmd::Model(m) = c {
                let q = glam::Mat4::from_cols_array_2d(&m.to_screen) * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
                println!(
                    "  model {}:{} origin at ({:.1}, {:.1}, z {:.1}, w {:.2}) state {:?} mmats {:?}",
                    m.file,
                    m.model,
                    q.x / q.w,
                    q.y / q.w,
                    16.0 * q.z / q.w,
                    q.w,
                    (&m.state.depth, &m.state.alpha_test, &m.state.blend),
                    m.mmats.iter().map(|d| format!("{d:?}").chars().take(160).collect::<String>()).collect::<Vec<_>>()
                );
            }
        }
        // The scene's draw list: each node's object, model, display switch
        // and transparency.
        if let Some(scene) = s.scene() {
            for (layer, nodes) in &scene.draw_list {
                for &i in nodes {
                    let n = &scene.nodes[i];
                    println!(
                        "  layer {layer} node {i}: obj {:08x} clump {} model {:?} disp {} tp {} (local {})",
                        n.obj, n.clump, n.model, n.disp, n.worldtp, n.localtp
                    );
                }
            }
        }
    }
    let archive = s.archive();
    let (w, h, rgba) = if soft_only {
        render_soft(archive, &frames)
    } else {
        match render_gpu(archive.clone(), data, &frames) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("no GPU ({e}); drawing on the CPU");
                render_soft(archive, &frames)
            }
        }
    };
    std::fs::write(&out, piney_gs::png::encode(w, h, &rgba))?;
    println!("{out}: stream {num} frame {} ({}x{})", s.frame(), w, h);
    Ok(())
}
