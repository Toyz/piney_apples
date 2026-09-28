//! The staff roll against the game: `tools/test_staffroll_rs.py` ran the
//! game's `ccThStaffRollCtrl` in eemu (seed 12345, names Kite and Tester,
//! `ccRand` as at boot) and kept each frame's state and draws; this runs
//! [`StaffRoll`] the same way and compares every frame.

use piney_data::volume::Volume;
use piney_desktop::staffroll::{CcRand, Draw, LINES, StaffRoll};

const FIXTURE: &str = include_str!("staffroll_fixture.txt");

fn fnv(b: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &c in b {
        h = (h ^ u32::from(c)).wrapping_mul(0x0100_0193);
    }
    h
}

/// The harness's state: the controller's words, its floats' bits, the stop
/// counts, and the lines' bytes +0x0c-+0xec with +0xf0 and +0xf4.
fn state(s: &StaffRoll) -> String {
    let mut bytes = Vec::new();
    for l in &s.lines {
        bytes.extend_from_slice(&l.buf);
        bytes.extend_from_slice(&l.text);
        bytes.extend_from_slice(&l.fixed);
        bytes.extend_from_slice(&l.stopped);
        bytes.extend_from_slice(&[0u8; 44]);
        bytes.extend_from_slice(&l.alpha.to_le_bytes());
        bytes.extend_from_slice(&l.bg.to_le_bytes());
    }
    assert_eq!(bytes.len(), LINES * (0xe0 + 8));
    format!(
        "{} {} {} {} {} {} {} {:08x} {:08x} {:08x} {:08x} {} {} {}",
        s.status,
        s.count,
        s.last,
        s.sub,
        s.page,
        s.pages,
        i32::from(s.done),
        s.sx,
        s.sy,
        s.alpha,
        s.bg,
        s.stop_every,
        s.stop_n,
        fnv(&bytes)
    )
}

fn draw(d: &Draw) -> String {
    match d {
        Draw::Frame { sx, sy } => format!("F 00000000 00000000 44000000 43c00000 43800000 43400000 {sx:08x} {sy:08x}"),
        Draw::Centre => "C 43800000 43400000".into(),
        Draw::Tex(t) => format!("X {t}"),
        Draw::Text { line, node, x, y, alpha, text, .. } => {
            let who = match node {
                None => format!("L{line}"),
                Some((k, false)) => format!("N{line}.{k}"),
                Some((k, true)) => format!("S{line}.{k}"),
            };
            format!("T {who} {x:08x} {y:08x} {alpha:08x} {} {}", text.len(), fnv(text))
        }
        Draw::Picture { alpha, dx, dy, sx, sy, su, sv, flag } => {
            format!("P {alpha:08x} {dx:08x} {dy:08x} {sx:08x} {sy:08x} {su} {sv} 0 0 1 0 {flag}")
        }
    }
}

fn draws(s: &StaffRoll) -> (Vec<String>, String) {
    let all: Vec<String> = s.draws.iter().map(draw).collect();
    let summary = format!("{} {}", all.len(), fnv(all.join("\n").as_bytes()));
    (all, summary)
}

#[test]
fn the_staff_roll_is_the_games() {
    let tables = piney_data::tables::staffroll::of(Volume::Inf);
    assert_eq!(tables.ccs, "stfroll1");
    let names = (b"Kite".to_vec(), b"Tester".to_vec());
    let mut s = StaffRoll::new(tables, 12345, CcRand::seeded(0x1100), names);
    let mut want = FIXTURE.lines().filter(|l| !l.starts_with('#') && !l.starts_with("view "));
    let check = |s: &StaffRoll, label: &str, line: &str| {
        let (all, summary) = draws(s);
        let got = format!("{label} | {} | {summary}", state(s));
        assert_eq!(
            got,
            line,
            "frame {label} differs; the port's draws (compare `python3 tools/test_staffroll_rs.py detail {label}`):\n{}",
            all.join("\n")
        );
    };
    check(&s, "new", want.next().unwrap());
    let mut frames = 0;
    for (f, line) in want.enumerate() {
        s.main();
        check(&s, &f.to_string(), line);
        frames += 1;
    }
    assert!(s.done, "the roll did not end with the fixture ({frames} frames)");
}

/// The layer's view: the game's `SetFrame` at s1, `SetLayerCenter(256, 192)`
/// and `SetFrame` again at s2 keep the centre, as
/// [`piney_desktop::view::LayerView::frame_centred`] builds it; each point
/// through `ApplyLayerScreenMatrix`.
#[test]
fn the_staff_roll_view_is_the_games() {
    let mut n = 0;
    for line in FIXTURE.lines().filter(|l| l.starts_with("view ")) {
        let w: Vec<&str> = line.split(' ').collect();
        let f = |i: usize| f32::from_bits(u32::from_str_radix(w[i], 16).unwrap());
        let (s2, x, y) = (f(2), f(3), f(4));
        let want: (i32, i32) = (w[5].parse().unwrap(), w[6].parse().unwrap());
        let v = piney_desktop::view::LayerView::frame_centred(0.0, 0.0, 512.0, 384.0, s2, s2, 256.0, 192.0);
        assert_eq!(v.apply(x, y), want, "{line}");
        n += 1;
    }
    assert_eq!(n, 36);
}
