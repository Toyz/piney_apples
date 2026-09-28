//! One stream's own sound, played to its end: the PCM it queues (samples a
//! frame, level, peak, zero crossings) and the other sound requests, with
//! the PCM written as a WAV. `STREAM_ENGLISH=1` for the English voice;
//! `STREAM_DUMP=DIR` writes the stream's files with a Pcm chunk.
//!
//! ```text
//! cargo run --release -p piney-stream --example stream_audio -- STREAM ISO [OUT.wav]
//! ```

use std::collections::BTreeMap;

use piney_data::iso::Iso;
use piney_input::Pad;
use piney_stream::{Options, PCM_RATE, Request, Stream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let num: usize = args.get(1).ok_or("STREAM")?.parse()?;
    let iso = args.get(2).ok_or("ISO")?;
    let mut iso = Iso::open(iso)?;
    // The discs with only the English archives (`STR*E.BIN`) take them.
    // STREAM_ENGLISH=1: the English voice (the E archives, or Outbreak on
    // the English PCM track).
    let english = std::env::var_os("STREAM_ENGLISH").is_some();
    let mut s = match Stream::with_options(&mut iso, num, Options { english, ..Options::default() }) {
        Ok(s) => s,
        Err(_) => Stream::with_options(&mut iso, num, Options { english: true, ..Options::default() })?,
    };
    for e in &s.def().entries {
        println!("entry {} ofs {} flag {}", e.name, e.ofs, e.flag);
    }
    for (k, f) in s.loaded().files.iter().enumerate() {
        for c in &f.setup.pcms {
            let (at, blocks, words) = (c.at, c.blocks, c.words);
            let h = f.data().get(at - 16..at).unwrap_or_default();
            println!("file {k}: PCM lang {} at {at:#x}: {blocks} blocks of {words} words; header {h:02x?}", c.lang);
            if let Ok(dir) = std::env::var("STREAM_DUMP") {
                std::fs::write(format!("{dir}/stream{num}-file{k}.ccs"), f.data())?;
            }
        }
    }
    let pad = Pad::default();
    let mut pcm: Vec<i16> = Vec::new();
    let mut other: BTreeMap<String, usize> = BTreeMap::new();
    let mut frames = 0u32;
    while !s.done() && frames < 100_000 {
        s.step(&pad);
        frames += 1;
        for r in s.take_requests() {
            match r {
                Request::Pcm(p) => pcm.extend(p),
                r => {
                    let k = format!("{r:?}");
                    *other.entry(k.split(['(', ' ', '{']).next().unwrap_or("").to_string()).or_default() += 1;
                }
            }
        }
    }
    let n = pcm.len().max(1) as f64;
    let rms = (pcm.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>() / n).sqrt();
    let peak = pcm.iter().map(|&v| i32::from(v).abs()).max().unwrap_or(0);
    let left: Vec<i16> = pcm.iter().step_by(2).copied().collect();
    let crossings = left.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count();
    let secs = left.len() as f64 / f64::from(PCM_RATE);
    println!(
        "stream {num}: {frames} frames, {} stereo samples ({:.1} per frame, {secs:.1} s at {PCM_RATE} Hz), rms {rms:.0}, peak {peak}, {:.0} zero crossings a second",
        left.len(),
        left.len() as f64 / f64::from(frames.max(1)),
        crossings as f64 / secs.max(1e-9)
    );
    println!("other requests: {other:?}");
    if let Some(out) = args.get(3) {
        let mut w = Vec::new();
        let data = (pcm.len() * 2) as u32;
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + data).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&PCM_RATE.to_le_bytes());
        w.extend_from_slice(&(PCM_RATE * 4).to_le_bytes());
        w.extend_from_slice(&4u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&data.to_le_bytes());
        for v in &pcm {
            w.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(out, w)?;
    }
    Ok(())
}
