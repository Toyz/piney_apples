//! Play every stream this disc has to its end and summarise each: its files,
//! scenes, frames, what it draws, its audio and its notes.
//!
//! ```text
//! cargo run --release -p piney-stream --example stream_survey -- [ISO] [--english]
//! ```

use std::collections::BTreeMap;

use piney_data::iso::Iso;
use piney_input::Pad;
use piney_stream::{Options, Request, Stream, table};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let mut opts = Options::default();
    for a in std::env::args().skip(1) {
        if a == "--english" {
            opts.english = true;
        } else {
            iso_path = a;
        }
    }
    let mut iso = Iso::open(&iso_path)?;
    let pad = Pad::default();
    for num in 0..table::count(iso.volume()?) {
        let mut s = match Stream::with_options(&mut iso, num, opts) {
            Ok(s) => s,
            Err(e) => {
                println!("{num:3} -: {e}");
                continue;
            }
        };
        let files: Vec<String> = s.def().entries.iter().map(|e| e.name.clone()).collect();
        let (mut samples, mut steps, mut max_cmds) = (0usize, 0u32, 0usize);
        let mut other: BTreeMap<String, usize> = BTreeMap::new();
        let mut frames = Vec::new();
        let mut last = 0;
        while !s.done() && steps < 100_000 {
            let f = s.step(&pad);
            steps += 1;
            max_cmds = max_cmds.max(f.cmds.len());
            if s.frame() == 0 && last != 0 {
                frames.push(last);
            }
            last = s.frame();
            for r in s.take_requests() {
                match r {
                    Request::Pcm(v) => samples += v.len() / 2,
                    Request::Effect { param, .. } => *other.entry(format!("effect {param}")).or_default() += 1,
                    r => *other.entry(format!("{r:?}")).or_default() += 1,
                }
            }
        }
        println!(
            "{num:3} {}: files {} scene frames {:?} steps {steps} ({:.1} s at 60, {:.1} s at 30) draws <= {max_cmds} pcm {:.1} s {:?}",
            s.def().header.name,
            files.join(","),
            frames,
            f64::from(steps) / 60.0,
            f64::from(steps) / 30.0,
            samples as f64 / 48_000.0,
            other
        );
    }
    Ok(())
}
