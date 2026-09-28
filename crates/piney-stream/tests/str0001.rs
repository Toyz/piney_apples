//! Stream 2's effect task against the game's: `str0001_fixture.txt`, written
//! by `python3 tools/test_stream_rs.py effects` from `Func_str0001` run in
//! `tools/eemu.py` over the whole stream (its cues from `str0001`'s notes)
//! and every layer's DMA list decoded to GS primitives.
//!
//! Checked, step by step from the task's first pass to the stream's end:
//! the cues the port's notes raise and on which step; every primitive the
//! task sends - its layer and order, kind, texture (the previous frame, a
//! frame-buffer copy's rectangle and size), filter, wrap, blend, alpha and Z
//! tests, Z write, scissor, and each vertex's position, Z, texture
//! coordinates and colour in GS units; and the C library's `rand` state
//! after each pass. Skipped when the disc image is not there (PINEY_ISO
//! points elsewhere).

use std::collections::BTreeMap;
use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_draw::{AlphaTest, Cmd, Filter, PrimKind, TexRef, Wrap};
use piney_input::Pad;
use piney_stream::{Request, Stream};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

/// One step of the fixture: its `rand` state and primitive lines.
struct Step {
    rand: u64,
    lines: Vec<String>,
}

struct Fixture {
    cues: Vec<(u32, u32)>,
    steps: Vec<Step>,
}

fn fixture() -> Fixture {
    let mut f = Fixture { cues: Vec::new(), steps: Vec::new() };
    for line in include_str!("str0001_fixture.txt").lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.first() {
            Some(&"cue") => f.cues.push((w[1].parse().unwrap(), w[2].parse().unwrap())),
            Some(&"step") => {
                assert_eq!(w[1].parse::<usize>().unwrap(), f.steps.len());
                let lines = if w.get(4) == Some(&"same") { f.steps.last().unwrap().lines.clone() } else { Vec::new() };
                f.steps.push(Step { rand: w[3].parse().unwrap(), lines });
            }
            Some(l) if l.starts_with('L') => f.steps.last_mut().unwrap().lines.push(line.to_string()),
            _ => {}
        }
    }
    f
}

/// A primitive as the fixture writes it: GS units, 1/16 pixel off the
/// frame buffer's corner, 1/16 texel in the UV register's 14 bits.
fn canon(layer: i16, cmd: &Cmd) -> String {
    let Cmd::Prim(p) = cmd else { return format!("L{layer} model") };
    let kind = match p.kind {
        PrimKind::Sprite => "sprite",
        PrimKind::Strip => "strip",
        PrimKind::Triangles => "tri",
        PrimKind::Fan => "fan",
    };
    let st = &p.state;
    let tex = st.texture.as_ref().map_or("none".to_string(), |t| {
        let src = match &t.tex {
            TexRef::PreviousFrame => "prev".to_string(),
            TexRef::FrameBuffer { x, y, width, height } => format!("frame:{x},{y},{width}x{height}"),
            _ => "other".to_string(),
        };
        let wrap = if t.wrap == Wrap::Clamp { "clamp" } else { "repeat" };
        let filter = if t.filter == Filter::Linear { "linear" } else { "nearest" };
        format!("{src}/{filter}/{wrap}/{wrap}/{}/{}", t.func as u8, u8::from(t.use_alpha))
    });
    let blend = st.blend.map_or("none".to_string(), |b| format!("{:#x}", b.to_reg()));
    let atest = match st.alpha_test {
        AlphaTest::Off => "off".to_string(),
        AlphaTest::On { method, reference, fail } => format!("{}/{reference}/{}", method as u8, fail as u8),
    };
    let fixed = |v: f32| {
        let n = v * 16.0;
        assert_eq!(n, n.round(), "not on the GS's 1/16 grid: {v}");
        n as i32
    };
    let verts: Vec<String> = p
        .verts
        .iter()
        .map(|v| {
            let (u, t) = if st.texture.is_some() {
                (fixed(v.u).rem_euclid(0x4000).to_string(), fixed(v.v).rem_euclid(0x4000).to_string())
            } else {
                ("-".to_string(), "-".to_string())
            };
            format!("{},{},{},{u},{t},{:08x}", fixed(v.x), fixed(v.y), v.z, u32::from_le_bytes(v.rgba.0))
        })
        .collect();
    let sc = st.scissor;
    format!(
        "L{layer} {kind} {} tex={tex} blend={blend} atest={atest} ztest={} zwrite={} scissor={},{},{},{} v={}",
        if p.gouraud { "gouraud" } else { "flat" },
        st.depth.test as u8,
        u8::from(st.depth.write),
        sc.x0,
        sc.x1,
        sc.y0,
        sc.y1,
        verts.join(" ")
    )
}

/// The task's packets in GS order: layers ascending, each layer's groups
/// last sent first (they go to the front of its list).
fn lines(draws: &[(i16, Vec<Cmd>)]) -> Vec<String> {
    let mut by_layer: BTreeMap<i16, Vec<&Vec<Cmd>>> = BTreeMap::new();
    for (layer, cmds) in draws {
        by_layer.entry(*layer).or_default().push(cmds);
    }
    let mut out = Vec::new();
    for (layer, groups) in by_layer {
        for g in groups.iter().rev() {
            out.extend(g.iter().map(|c| canon(layer, c)));
        }
    }
    out
}

#[test]
fn str0001_effects_match_the_game() {
    let Some(p) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let fx = fixture();
    let mut iso = Iso::open(p).unwrap();
    let mut s = Stream::new(&mut iso, 2).unwrap();
    let pad = Pad::default();
    let mut cues = Vec::new();
    let (mut drawn, mut noise_steps) = (0, 0);
    for (k, want) in fx.steps.iter().enumerate() {
        if k > 0 {
            assert!(!s.done(), "the stream ended before step {k}");
            s.step(&pad);
            for r in s.take_requests() {
                if let Request::Effect { param, .. } = r {
                    cues.push((k as u32, param));
                }
            }
        }
        let got = lines(s.effect_draws());
        if got != want.lines {
            for (i, (g, w)) in got.iter().zip(&want.lines).enumerate() {
                if g != w {
                    panic!("step {k} primitive {i}:\n port {g}\n game {w}");
                }
            }
            panic!("step {k}: {} primitives, the game {}", got.len(), want.lines.len());
        }
        assert_eq!(s.rand().next, want.rand, "step {k}: rand");
        drawn += got.len();
        noise_steps += usize::from(got.iter().any(|l| l.starts_with("L100")));
    }
    assert_eq!(cues, fx.cues);
    assert!(s.done(), "the stream goes on past the game's task");
    eprintln!("{} steps, {drawn} primitives, {noise_steps} steps of raster noise", fx.steps.len());
    assert_eq!(noise_steps, 10);
}
