//! Render the game's sound headless to a WAV file.
//!
//!     cargo run --release -p piney-audio --example render -- se N OUT.wav [--seconds S]
//!     cargo run --release -p piney-audio --example render -- bgm NO OUT.wav [--seconds S]
//!     cargo run --release -p piney-audio --example render -- desktop NO OUT.wav [--seconds S]
//!     cargo run --release -p piney-audio --example render -- stream TRACK OUT.wav [--seconds S]
//!     cargo run --release -p piney-audio --example render -- play NO [--seconds S]
//!     cargo run --release -p piney-audio --example render -- probe 0
//!
//! `se` plays sound effect N (`ccSeOn`); `bgm` the jukebox row NO
//! (`Wave[NO]`, as the desktop's music player picks it); `desktop` the
//! desktop starting with `dtBgm = NO`; `stream` a `VOICE/BGM.BIN` track;
//! `play` the desktop starting with row NO through the default output
//! device instead of a file; `probe` opens that device, reports it and
//! plays nothing. The disc image is `work/infection/infection.iso` or
//! `$PINEY_ISO`.

use std::path::PathBuf;
use std::time::Duration;

use piney_audio::{Audio, wav};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "render (se|bgm|desktop|stream) N OUT.wav [--seconds S] | render play NO [--seconds S]";
    let (Some(kind), Some(n)) = (args.first(), args.get(1).and_then(|s| s.parse::<usize>().ok())) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let seconds: f64 = args
        .iter()
        .position(|a| a == "--seconds")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(if kind == "se" { 3.0 } else { 20.0 });
    let iso = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    if kind == "probe" {
        // Open the output device, play nothing, report.
        let audio = Audio::open(&iso).expect("the disc image");
        match (&audio.silent_reason, audio.output_format()) {
            (Some(why), _) => println!("silent: {why}"),
            (None, Some((rate, channels))) => println!("output: {rate} Hz, {channels} channels"),
            (None, None) => println!("output"),
        }
        return;
    }
    if kind == "play" {
        let audio = Audio::open(&iso).expect("the disc image");
        if let Some(why) = &audio.silent_reason {
            eprintln!("no output: {why}");
        }
        audio.desktop_bgm(n);
        let frames = (seconds * 60.0) as u32;
        for _ in 0..frames {
            audio.frame();
            std::thread::sleep(Duration::from_micros(16_683));
        }
        return;
    }
    let Some(out) = args.get(2) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let audio = Audio::headless(&iso).expect("the disc image");
    match kind.as_str() {
        "se" => audio.se_on(n),
        "bgm" => audio.bgm(n),
        "desktop" => audio.desktop_bgm(n),
        "stream" => audio.bgm_stream(n).expect("BGM.BIN"),
        _ => {
            eprintln!("{usage}");
            std::process::exit(2);
        }
    }
    // A game frame every 800 samples (60 per second), as the desktop runs.
    let total = (seconds * 48_000.0) as usize;
    let mut pcm = vec![0i16; total * 2];
    for chunk in pcm.chunks_mut(1600) {
        audio.frame();
        audio.render(chunk);
    }
    let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    wav::write(out, &pcm, 48_000, 2).expect("write the WAV");
    println!("{out}: {seconds} s, peak {peak}");
}
