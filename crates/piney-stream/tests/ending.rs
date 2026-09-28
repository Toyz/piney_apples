//! Stream 15's effect tasks against the game's: `str0580_fixture.txt` and
//! `str0581_fixture.txt` (`python3 tools/test_stream_rs.py effects str0580`,
//! `str0581`), the tasks run in `tools/eemu.py` over each scene's cues, the
//! table's objects standing in by name (`stand_in_pos(name)`). The port's
//! tasks run on their own over the same cues and stand-ins and are compared
//! step by step: `rand`'s state, every primitive, the parts' draws (hashed)
//! and the transfers. `Func_str0580` gets `param[1]` 2 the step after the
//! scene's last frame, `Func_str0581` 1 with the scene gone.

use std::collections::BTreeMap;
use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_draw::{AlphaTest, Cmd, Filter, PrimKind, TexRef, Wrap};
use piney_stream::effect::{Cue, Rand, Tables};
use piney_stream::ending::{self, PartDraw, StandIns, Str0580, Str0581};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

struct Step {
    rand: u64,
    draws: (usize, u64),
    parts: (usize, u64),
    extra: Vec<String>,
}

struct Fixture {
    end: u32,
    cues: BTreeMap<u32, Vec<(u32, u32)>>,
    steps: Vec<Step>,
    /// The scene view's `divZ` as the task set it: (step, bits).
    div_z: Vec<(usize, u32)>,
}

const FNV_EMPTY: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(lines: &[String]) -> u64 {
    let mut h = FNV_EMPTY;
    for l in lines {
        for b in l.bytes().chain(std::iter::once(b'\n')) {
            h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn fixture(text: &str) -> Fixture {
    let mut f = Fixture { end: 0, cues: BTreeMap::new(), steps: Vec::new(), div_z: Vec::new() };
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.first() {
            Some(&"end") => f.end = w[1].parse().unwrap(),
            Some(&"cue") => {
                let (k, p, obj): (u32, u32, u32) =
                    (w[1].parse().unwrap(), w[2].parse().unwrap(), w[3].parse().unwrap());
                f.cues.entry(k).or_default().push((p, obj));
            }
            Some(&"step") => {
                assert_eq!(w[1].parse::<usize>().unwrap(), f.steps.len());
                let draws = match w.get(4) {
                    Some(&"same") => f.steps.last().unwrap().draws,
                    Some(n) => (n.parse().unwrap(), u64::from_str_radix(w[5], 16).unwrap()),
                    None => (0, FNV_EMPTY),
                };
                f.steps.push(Step { rand: w[3].parse().unwrap(), draws, parts: (0, FNV_EMPTY), extra: Vec::new() });
            }
            Some(&"parts") => {
                f.steps.last_mut().unwrap().parts = (w[1].parse().unwrap(), u64::from_str_radix(w[2], 16).unwrap())
            }
            Some(&"divz") => f.div_z.push((f.steps.len() - 1, u32::from_str_radix(w[1], 16).unwrap())),
            Some(&"transfer") => {
                let bits = |i: usize| u32::from_str_radix(w[i], 16).unwrap();
                let obj = f32::from_bits(bits(1)) as u32;
                f.steps.last_mut().unwrap().extra.push(format!("transfer {obj} {}", w[4]));
            }
            _ => {}
        }
    }
    f
}

/// A primitive as the fixture writes it (`tests/effects.rs`'s form, with
/// page 0's frame-buffer copy as `cur` and REGION_REPEAT as
/// `region:MASK,FIX`).
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
            &TexRef::FrameBuffer { x: 0, y: 0, width: 512, height: 448 } => "cur".to_string(),
            TexRef::FrameBuffer { x, y, width, height } => format!("frame:{x},{y},{width}x{height}"),
            TexRef::Ccs { .. } => "ccs".to_string(),
            _ => "other".to_string(),
        };
        let wrap = match t.wrap {
            Wrap::Clamp => "clamp".to_string(),
            Wrap::Repeat => "repeat".to_string(),
            Wrap::Region { mask, fix } => format!("region:{mask},{fix}"),
        };
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

/// The pass's packets in GS order: layers ascending, each layer's groups
/// last sent first.
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

/// A part's draw as the fixture writes it.
fn part_line(d: &PartDraw) -> String {
    let hex = |v: &[u32]| v.iter().map(|x| format!("{x:08x}")).collect::<Vec<_>>().join(" ");
    match d {
        PartDraw::Puff { pos, pattern, scale, tp, colour } => {
            format!("eff {} {pattern} {} {tp:08x} {colour:08x}", hex(pos), hex(scale))
        }
        PartDraw::Rock { model, matrix } => {
            format!("rock {model} {} 3f800000", hex(&matrix.iter().flatten().copied().collect::<Vec<_>>()))
        }
        PartDraw::Xpart { chunk, matrix } => {
            format!("xpart {chunk} {}", hex(&matrix.iter().flatten().copied().collect::<Vec<_>>()))
        }
    }
}

/// Where the harness stands an object in.
fn stand_in_pos(name: &str) -> [u32; 4] {
    let x = name.bytes().map(u32::from).sum::<u32>() * 10;
    [(x as f32).to_bits(), ((name.len() * 100) as f32).to_bits(), 50f32.to_bits(), 1f32.to_bits()]
}

fn tables() -> Option<Tables> {
    let mut iso = Iso::open(iso_path()?).unwrap();
    Some(Tables::read(iso.volume().unwrap()))
}

enum Task {
    A(Box<Str0580>),
    B(Box<Str0581>),
}

/// The task against its fixture: (steps, primitives, part draws).
fn check(text: &str, first: bool) -> (usize, usize, usize) {
    let fx = fixture(text);
    let t = tables().unwrap();
    let world = StandIns { names: ending::names(&t.ending), pos: stand_in_pos };
    let mut rand = Rand::default();
    let mut task = if first {
        Task::A(Box::new(Str0580::new(&t.ending, &world, &mut rand)))
    } else {
        Task::B(Box::new(Str0581::new(&t.ending, &world, &mut rand)))
    };
    // The task sets the view's divZ before its first pass, and never again.
    assert_eq!(fx.div_z, [(0, if first { ending::DIV_Z_0580 } else { ending::DIV_Z_0581 })], "divZ");
    let (mut drawn, mut parts) = (0, 0);
    for (k, want) in fx.steps.iter().enumerate() {
        let k = k as u32;
        if k > fx.end + u32::from(!first) {
            match &mut task {
                Task::A(a) => a.set_param(2),
                Task::B(b) => {
                    b.set_param(1);
                    b.scene_gone();
                }
            }
        }
        // Each note's object, which a transfer names (the harness's note
        // objects stand at their index on x).
        // The harness queues a step's cues after the pass before it: a note
        // under the scene's Top 0 (str0581's 900) never reaches the task.
        let cues: Vec<Cue> = fx
            .cues
            .get(&k)
            .filter(|_| k > 0)
            .map(|c| c.iter().map(|&(param, obj)| Cue { param, obj }).collect())
            .unwrap_or_default();
        let now = (k + 1).min(fx.end);
        let (draws, part_draws, extra) = match &mut task {
            Task::A(a) => {
                let d = a.step(&world, &cues, now, false, &mut rand);
                (d, a.part_draws().to_vec(), Vec::new())
            }
            Task::B(b) => {
                let d = b.step(&world, &cues, now, false, &mut rand);
                let extra: Vec<String> =
                    b.take_transfers().iter().map(|t| format!("transfer {} {:08x}", t.obj, t.height)).collect();
                (d, b.part_draws().to_vec(), extra)
            }
        };
        let got = lines(&draws);
        assert_eq!((got.len(), fnv(&got)), want.draws, "step {k}: {got:#?}");
        assert_eq!(rand.next, want.rand, "step {k}: rand");
        let p: Vec<String> = part_draws.iter().map(part_line).collect();
        assert_eq!((p.len(), fnv(&p)), want.parts, "step {k}: parts {p:#?}");
        assert_eq!(extra, want.extra, "step {k}: transfers");
        drawn += got.len();
        parts += p.len();
    }
    match &task {
        Task::A(a) => assert!(a.done(), "Func_str0580 goes on past the game's"),
        Task::B(b) => assert!(b.done(), "Func_str0581 goes on past the game's"),
    }
    (fx.steps.len(), drawn, parts)
}

#[test]
fn str0580_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, parts) = check(include_str!("str0580_fixture.txt"), true);
    eprintln!("stream 15 (str0580): {steps} steps, {drawn} primitives, {parts} part draws");
    assert!(parts > 0);
}

#[test]
fn str0581_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, parts) = check(include_str!("str0581_fixture.txt"), false);
    eprintln!("stream 15 (str0581): {steps} steps, {drawn} primitives, {parts} part draws");
    assert!(parts > 0);
}

/// Stream 15 played for real: `Func_str0580` over the scene's own objects
/// sends the fixture's primitives and draws the fixture's `rand` up to the
/// last frame of `str0580` (the part draws differ: the objects stand where
/// the scene has them), with the fixture's cues on the same steps; then it
/// passes four more times beside `Func_str0581` (its fog), whose cue 699
/// starts a transfer, and the stream ends. Each task sets the view's
/// `divZ` as the fixtures have it.
#[test]
fn stream15_plays_through() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    use piney_input::Pad;
    use piney_stream::{Request, Stream};
    let fx = fixture(include_str!("str0580_fixture.txt"));
    let mut iso = Iso::open(iso_path().unwrap()).unwrap();
    let mut s = Stream::new(&mut iso, 15).unwrap();
    let pad = Pad::default();
    let (mut cues, mut want_cues) = (Vec::new(), Vec::new());
    for (&k, c) in &fx.cues {
        if k > 0 {
            want_cues.extend(c.iter().map(|&(p, _)| (k, p)));
        }
    }
    // The last step of str0580: the next scene is set up in it, after the
    // task's pass, and its task's first pass is what the step reports.
    let last = fx.end;
    let (mut parts, mut div_z) = (0, Vec::new());
    for k in 0..=last {
        if k > 0 {
            s.step(&pad);
            for r in s.take_requests() {
                match r {
                    Request::Effect { param, .. } => cues.push((k, param)),
                    Request::DivZ(z) => div_z.push(z),
                    _ => {}
                }
            }
        }
        let want = &fx.steps[k as usize];
        if k < last {
            let got = lines(s.effect_draws());
            assert_eq!((got.len(), fnv(&got)), want.draws, "step {k}");
            assert_eq!(s.rand().next, want.rand, "step {k}: rand");
        }
        parts += s.effect().map_or(0, |t| t.part_draws().len());
    }
    assert_eq!(cues, want_cues, "cues");
    assert!(parts > 0, "no parts drawn");
    let (mut transfers, mut steps) = (0, 0);
    while !s.done() {
        s.step(&pad);
        steps += 1;
        for r in s.take_requests() {
            match r {
                Request::Transfer { height, .. } => {
                    assert_eq!(height, ending::TRANSFER_HEIGHT_0581);
                    transfers += 1;
                }
                Request::DivZ(z) => div_z.push(z),
                _ => {}
            }
        }
    }
    eprintln!("stream 15: {} steps in str0580, {steps} after, {parts} part draws, {transfers} transfers", last + 1);
    assert_eq!(transfers, 1);
    let second = fixture(include_str!("str0581_fixture.txt"));
    let want: Vec<u32> = fx.div_z.iter().chain(&second.div_z).map(|&(_, z)| z).collect();
    assert_eq!(div_z, want);
}
