//! The subtitles under an event's stream against the game's:
//! `subtitle_fixture.txt`, written by `python3 tools/test_stream_rs.py
//! subtitles` from `ccEventStream(num, 1)` run in `tools/eemu.py` over each
//! stream's own notes (`ccSetStreamDemoNote`, `ccGetStreamDemoMsg`,
//! `ccKanjiStrSeparate` and `ccMessage`'s `Change`, `Close` and `Disp`
//! the game's).
//!
//! The port plays each stream for real ([`EventStream`] over [`Stream`]:
//! its notes, its frames, its skip) and compares, step by step, every call
//! the window's `Disp` makes - each cell's packet (position, size, grid,
//! alpha), each line's text, glyph count, position and colour, and the
//! send - by the fixture's hash of them; and the step the call returns
//! after. Every stream with a table, Movie Text on; then streams 3, 11 and
//! 16 with it off (11's and 16's lines flagged 0x800 show all the same),
//! 3 in Parody Mode, and two skips in the middle of a line. Skipped
//! when the disc image is not there (PINEY_ISO points elsewhere).

use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_data::save::{InitText, SaveData, offset};
use piney_desktop::message::MsgDraw;
use piney_input::{Buttons, Pad};
use piney_stream::event::EventStream;
use piney_stream::subtitle::Subtitles;
use piney_stream::{Options, Stream};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

struct Run {
    num: usize,
    win_mode: u8,
    parody: u8,
    skip: Option<u32>,
    /// (step, lines, FNV-1a 64) where the draws change.
    steps: Vec<(u32, usize, u64)>,
    end: u32,
}

fn runs() -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for line in include_str!("subtitle_fixture.txt").lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.first() {
            Some(&"run") => out.push(Run {
                num: w[1].parse().unwrap(),
                win_mode: w[2].parse().unwrap(),
                parody: w[3].parse().unwrap(),
                skip: w[4].parse().ok(),
                steps: Vec::new(),
                end: 0,
            }),
            Some(&"step") => out.last_mut().unwrap().steps.push((
                w[1].parse().unwrap(),
                w[2].parse().unwrap(),
                u64::from_str_radix(w[3], 16).unwrap(),
            )),
            Some(&"end") => out.last_mut().unwrap().end = w[2].parse().unwrap(),
            _ => {}
        }
    }
    out
}

/// `Disp`'s calls as the fixture writes them.
fn lines(draws: &[MsgDraw]) -> Vec<String> {
    draws
        .iter()
        .map(|d| match d {
            MsgDraw::Cell { code, dx, dy, sx, sy, grid, alpha } => format!(
                "pkt {code} {:x} {:x} {:x} {:x} {} {} {} {} {} {alpha}",
                dx.to_bits(),
                dy.to_bits(),
                sx.to_bits(),
                sy.to_bits(),
                grid.2,
                grid.3,
                grid.0,
                grid.1,
                grid.4
            ),
            MsgDraw::Text { text, count, dx, dy, rgba, .. } => {
                let h: String = text.iter().map(|b| format!("{b:02x}")).collect();
                format!(
                    "kanji {} {count} {:x} {:x} {} {} {} {}",
                    if h.is_empty() { "-" } else { &h },
                    dx.to_bits(),
                    dy.to_bits(),
                    rgba[0],
                    rgba[1],
                    rgba[2],
                    rgba[3]
                )
            }
            MsgDraw::Send => "send".to_string(),
        })
        .collect()
}

fn fnv(lines: &[String]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for l in lines {
        for b in l.bytes().chain(std::iter::once(b'\n')) {
            h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// One run: the stream played with its window, compared step by step;
/// the number of steps whose draws were compared.
fn check(iso_path: &PathBuf, run: &Run) -> usize {
    let mut iso = Iso::open(iso_path).unwrap();
    // tools/test_save_init_rs.fresh_save: the boot's save, the player named
    // Kite.
    let mut save = SaveData::boot(&InitText::default());
    save.bytes_mut()[offset::PL_NAME..offset::PL_NAME + 4].copy_from_slice(b"Kite");
    save.set_u8(offset::STR_WIN_MODE, run.win_mode);
    save.set_u8(offset::PARODY_FLAG, run.parody);
    let stream = Stream::with_rand(&mut iso, run.num, Options::default(), Default::default()).unwrap();
    let subs = Subtitles::read(iso.volume().unwrap(), run.num, &save).unwrap();
    assert!(subs.is_some(), "stream {} has a table", run.num);
    let mut es = EventStream::with_subtitles(stream, subs);
    let mut want = run.steps.iter();
    let mut last: Option<Vec<String>> = None;
    let (mut step, mut checked) = (0, 0);
    while !es.done() {
        step += 1;
        let mut pad = Pad::default();
        if Some(step) == run.skip {
            pad.push = Buttons::START;
        }
        es.step(&pad);
        let l = lines(es.window_draws());
        if last.as_ref() != Some(&l) {
            let tag = format!("stream {} run {:?} step {step}", run.num, (run.win_mode, run.parody, run.skip));
            let &(k, n, h) = want.next().unwrap_or_else(|| panic!("{tag}: the draws change, not in the fixture"));
            assert_eq!((k, n, h), (step, l.len(), fnv(&l)), "{tag}: {l:#?}");
            last = Some(l);
            checked += 1;
        }
    }
    assert!(want.next().is_none(), "stream {}: fixture steps left", run.num);
    assert_eq!(step, run.end, "stream {}: the step the call returns after", run.num);
    // The window is closed and drawn no more.
    es.step(&Pad::default());
    assert!(es.window_draws().is_empty() && es.subtitles().unwrap().closed());
    checked
}

#[test]
fn the_subtitles_are_the_games() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let runs = runs();
    assert_eq!(runs.len(), 19);
    // The runs side by side, a few at a time (each holds its stream's files).
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(6);
    let checked: usize = std::thread::scope(|sc| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                sc.spawn(|| {
                    let mut n = 0;
                    while let Some(run) = runs.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
                        n += check(&path, run);
                    }
                    n
                })
            })
            .collect();
        workers.into_iter().map(|w| w.join().unwrap()).sum()
    });
    assert!(checked > 5000, "{checked} steps of draws checked");
}
