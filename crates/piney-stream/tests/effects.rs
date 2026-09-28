//! The effect tasks beyond stream 2's against the game's: each
//! `strNNNN_fixture.txt`, written by `python3 tools/test_stream_rs.py effects
//! strNNNN` from `Func_strNNNN` run in `tools/eemu.py` over the whole scene
//! (as `tests/str0001.rs`). The port plays the stream for real and compares,
//! step by step: the cues and their steps, every primitive (by the fixture's
//! FNV-1a 64 of the canonical lines), `rand`'s state after each pass, the hit
//! marks and the view's `divZ`. Streams 10 (`Func_str0300`) and 5
//! (`Func_str0120`). Skipped without the disc image (PINEY_ISO).

use std::collections::BTreeMap;
use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_draw::{AlphaTest, Cmd, Filter, PrimKind, TexRef, Wrap};
use piney_input::Pad;
use piney_stream::effect::{Cue, Rand, Str7100};
use piney_stream::{Options, Request, SkillNames, Stream};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

/// One step of a hashed fixture: its `rand` state, and (lines, FNV) of its
/// primitives.
struct Step {
    rand: u64,
    draws: (usize, u64),
    /// `mark OBJ RX RY RZ`, `transfer OBJ HEIGHT` and `divz BITS` lines
    /// after the step.
    extra: Vec<String>,
}

struct Fixture {
    cues: Vec<(u32, u32)>,
    steps: Vec<Step>,
}

fn fixture(text: &str) -> Fixture {
    let mut f = Fixture { cues: Vec::new(), steps: Vec::new() };
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.first() {
            Some(&"cue") => f.cues.push((w[1].parse().unwrap(), w[2].parse().unwrap())),
            Some(&"step") => {
                assert_eq!(w[1].parse::<usize>().unwrap(), f.steps.len());
                let draws = match w.get(4) {
                    Some(&"same") => f.steps.last().unwrap().draws,
                    Some(n) => (n.parse().unwrap(), u64::from_str_radix(w[5], 16).unwrap()),
                    None => (0, FNV_EMPTY),
                };
                f.steps.push(Step { rand: w[3].parse().unwrap(), draws, extra: Vec::new() });
            }
            Some(&"mark") => {
                let bits = |i: usize| u32::from_str_radix(w[i], 16).unwrap();
                assert_eq!((bits(2), bits(3), w[7]), (0, 0, "effLayer"), "{line}");
                let obj = f32::from_bits(bits(1)) as u32;
                f.steps.last_mut().unwrap().extra.push(format!("mark {obj} {} {} {}", w[4], w[5], w[6]));
            }
            Some(&"transfer") => {
                let bits = |i: usize| u32::from_str_radix(w[i], 16).unwrap();
                assert_eq!((bits(2), bits(3), w[5]), (0, 0, "effLayer"), "{line}");
                let obj = f32::from_bits(bits(1)) as u32;
                f.steps.last_mut().unwrap().extra.push(format!("transfer {obj} {}", w[4]));
            }
            Some(&"divz") => f.steps.last_mut().unwrap().extra.push(line.to_string()),
            _ => {}
        }
    }
    f
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
            TexRef::Ccs { .. } => "ccs".to_string(),
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

/// Stream `num`'s task against `text`, the fixture: (steps, primitives,
/// steps drawing on each layer).
fn check(num: usize, text: &str) -> (usize, usize, BTreeMap<i16, usize>) {
    check_with(num, text, Options::default())
}

/// As [`check`] with the stream's options `opts`.
fn check_with(num: usize, text: &str, opts: Options) -> (usize, usize, BTreeMap<i16, usize>) {
    let fx = fixture(text);
    let mut iso = Iso::open(iso_path().unwrap()).unwrap();
    let mut s = Stream::with_options(&mut iso, num, opts).unwrap();
    let pad = Pad::default();
    let mut cues = Vec::new();
    let mut drawn = 0;
    let mut layers: BTreeMap<i16, usize> = BTreeMap::new();
    for (k, want) in fx.steps.iter().enumerate() {
        let mut extra = Vec::new();
        if k > 0 {
            assert!(!s.done(), "stream {num}: the stream ended before step {k}");
            s.step(&pad);
            for r in s.take_requests() {
                match r {
                    Request::Effect { param, .. } => cues.push((k as u32, param)),
                    Request::HitMark { obj, rot, .. } => {
                        extra.push(format!("mark {obj} {:08x} {:08x} {:08x}", rot[0], rot[1], rot[2]))
                    }
                    Request::Transfer { obj, height, .. } => extra.push(format!("transfer {obj} {height:08x}")),
                    Request::DivZ(z) => extra.push(format!("divz {z:08x}")),
                    _ => {}
                }
            }
        }
        assert_eq!(extra, want.extra, "stream {num} step {k}: marks and divZ");
        let got = lines(s.effect_draws());
        assert_eq!((got.len(), fnv(&got)), want.draws, "stream {num} step {k}: {got:#?}");
        assert_eq!(s.rand().next, want.rand, "stream {num} step {k}: rand");
        drawn += got.len();
        for (l, _) in s.effect_draws() {
            *layers.entry(*l).or_default() += 1;
        }
    }
    assert_eq!(cues, fx.cues, "stream {num}: cues");
    assert!(s.done(), "stream {num}: the stream goes on past the game's task");
    (fx.steps.len(), drawn, layers)
}

#[test]
fn str0300_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(10, include_str!("str0300_fixture.txt"));
    eprintln!("stream 10: {steps} steps, {drawn} primitives, by layer {layers:?}");
    // The noise and inversion on the task's layer, the flashes on the font
    // layer.
    assert!(layers.contains_key(&100) && layers.contains_key(&240));
}

#[test]
fn str0120_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(5, include_str!("str0120_fixture.txt"));
    eprintln!("stream 5: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(layers.contains_key(&100) && layers.contains_key(&240));
}

#[test]
fn str0090_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(3, include_str!("str0090_fixture.txt"));
    eprintln!("stream 3: {steps} steps, {drawn} primitives, by layer {layers:?}");
    // The feedback on the task's layer from cue 5 on.
    assert!(layers.contains_key(&100) && drawn > 0);
}

#[test]
fn str0110_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(4, include_str!("str0110_fixture.txt"));
    eprintln!("stream 4: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(layers.contains_key(&100) && drawn > 0);
}

#[test]
fn str0130_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(6, include_str!("str0130_fixture.txt"));
    eprintln!("stream 6: {steps} steps, {drawn} primitives, by layer {layers:?}");
    // Cue 899's fade across the letterbox, on the font layer.
    assert!(layers.contains_key(&240) && drawn > 0);
}

#[test]
fn str0150_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(7, include_str!("str0150_fixture.txt"));
    eprintln!("stream 7: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str0240_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(8, include_str!("str0240_fixture.txt"));
    eprintln!("stream 8: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str0250_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(9, include_str!("str0250_fixture.txt"));
    eprintln!("stream 9: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(layers.contains_key(&100) && layers.contains_key(&240));
}

#[test]
fn str0301_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(11, include_str!("str0301_fixture.txt"));
    eprintln!("stream 11: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str0305_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(12, include_str!("str0305_fixture.txt"));
    eprintln!("stream 12: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str0350_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(13, include_str!("str0350_fixture.txt"));
    eprintln!("stream 13: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str0570_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(14, include_str!("str0570_fixture.txt"));
    eprintln!("stream 14: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str0610_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(16, include_str!("str0610_fixture.txt"));
    eprintln!("stream 16: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str8000_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    // A small enemy's drain movie (str8100) and Skeith's (str9102): the
    // same task.
    for (num, text) in [(109, include_str!("str8100_fixture.txt")), (18, include_str!("str9102_fixture.txt"))] {
        let (steps, drawn, layers) = check(num, text);
        eprintln!("stream {num}: {steps} steps, {drawn} primitives, by layer {layers:?}");
        assert!(layers.contains_key(&100) && layers.contains_key(&240));
    }
}

#[test]
fn str9000_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    // A member drained (str9104), the banner's texture given as the game
    // has it: xeffect's TEX_detadrain, 128 texels high.
    let names = SkillNames { texture: 0, clut: 0, tex_h: 128 };
    let opts = Options { skill_names: Some(names), ..Options::default() };
    let (steps, drawn, layers) = check_with(20, include_str!("str9104_fixture.txt"), opts);
    eprintln!("stream 20: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(layers.contains_key(&101) && layers.contains_key(&100) && layers.contains_key(&240));
}

#[test]
fn str9001_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(17, include_str!("str9101_fixture.txt"));
    eprintln!("stream 17: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(drawn > 0);
}

#[test]
fn str8800_effects_match_the_game() {
    if iso_path().is_none() {
        eprintln!("infection.iso not present; skipped");
        return;
    }
    let (steps, drawn, layers) = check(112, include_str!("str8801_fixture.txt"));
    eprintln!("stream 112: {steps} steps, {drawn} primitives, by layer {layers:?}");
    assert!(layers.contains_key(&100));
}

/// `Func_str7100` plays inside the gate hack's movie, not a stream of its
/// own: the task alone, fed the fixture's cues on their steps from the
/// default `rand`, as the harness runs the game's.
#[test]
fn str7100_effects_match_the_game() {
    let fx = fixture(include_str!("str7100_fixture.txt"));
    let mut rand = Rand::default();
    let mut task = Str7100::new(&mut rand);
    let mut drawn = 0;
    for (k, want) in fx.steps.iter().enumerate() {
        let cues: Vec<Cue> =
            fx.cues.iter().filter(|(step, _)| *step as usize == k).map(|&(_, param)| Cue { param, obj: 0 }).collect();
        let got = lines(&task.step(&cues, false, &mut rand));
        assert_eq!((got.len(), fnv(&got)), want.draws, "step {k}: {got:#?}");
        assert_eq!(rand.next, want.rand, "step {k}: rand");
        drawn += got.len();
    }
    eprintln!("str7100: {} steps, {drawn} primitives", fx.steps.len());
    assert!(drawn > 0);
}
