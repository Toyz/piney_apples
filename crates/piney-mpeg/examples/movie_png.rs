//! Dump pictures of a PSS movie to PNG, as the IPU converts them (640 x 448
//! RGBA; the game stretches this to the full screen width).
//!
//!     cargo run --release -p piney-mpeg --example movie_png -- [--iso PATH] MOVIE OUTDIR [N,N,...]
//!
//! MOVIE is a disc path (`PSS/OPENING.PSS`) or the game's name
//! (`opening.pss`); N are display-order picture numbers, default 0, 30, 60
//! and the last. Also prints the stream's header and the audio's format.

use std::io::Write;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use piney_data::iso::Iso;
use piney_mpeg::Movie;
use piney_mpeg::movie::disc_path;

fn crc32(chunks: &[&[u8]]) -> u32 {
    let mut crc = !0u32;
    for data in chunks {
        for &b in *data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            }
        }
    }
    !crc
}

/// An 8-bit RGBA PNG, top row first, alpha forced opaque (the IPU's 0x80
/// is the GS's 1.0).
fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for row in rgba.chunks_exact(width as usize * 4) {
        raw.push(0);
        for [r, g, b, _] in row.as_chunks::<4>().0 {
            raw.extend_from_slice(&[*r, *g, *b, 255]);
        }
    }
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(&raw).unwrap();
    let idat = z.finish().unwrap();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [(b"IHDR", &ihdr[..]), (b"IDAT", &idat[..]), (b"IEND", &[][..])] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut iso_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if let Some(i) = args.iter().position(|a| a == "--iso") {
        iso_path = PathBuf::from(args.remove(i + 1));
        args.remove(i);
    }
    let [movie, out, rest @ ..] = &args[..] else {
        eprintln!("movie_png [--iso PATH] MOVIE OUTDIR [N,N,...]");
        std::process::exit(2);
    };
    let path = if movie.contains('/') { movie.clone() } else { disc_path(movie) };
    let mut iso = Iso::open(&iso_path)?;
    let mut m = Movie::open(&mut iso, &path)?;
    let s = m.sequence().clone();
    println!(
        "{path}: {}x{}, {} frames/s, profile/level 0x{:02x}, intra matrix {}",
        s.width,
        s.height,
        m.frame_rate(),
        s.profile_level,
        if s.loaded_intra { "loaded" } else { "default" }
    );
    if let Some(a) = m.audio() {
        println!(
            "audio: SShd type {}, {} Hz, {} channels, interleave 0x{:x}; {:.2} s",
            a.header.format,
            a.rate,
            a.channels,
            a.header.interleave,
            a.frames() as f64 / f64::from(a.rate)
        );
    }
    let mut wanted: Vec<u32> = match rest.first() {
        Some(list) => list.split(',').map(|n| n.parse()).collect::<Result<_, _>>()?,
        None => vec![0, 30, 60, u32::MAX],
    };
    wanted.sort_unstable();
    std::fs::create_dir_all(out)?;
    let stem = path.rsplit('/').next().unwrap_or(&path).to_ascii_lowercase().replace(".pss", "");
    let mut last = None;
    let mut n = 0u32;
    while let Some(p) = m.next_picture()? {
        if wanted.binary_search(&n).is_ok() {
            let file = PathBuf::from(out).join(format!("{stem}_{n:04}.png"));
            std::fs::write(&file, png(p.width as u32, p.height as u32, &p.to_rgba()))?;
            println!("{} ({:?} picture)", file.display(), p.header.kind);
        }
        last = Some(p);
        n += 1;
    }
    if let (Some(p), true) = (last, wanted.contains(&u32::MAX)) {
        let file = PathBuf::from(out).join(format!("{stem}_{:04}.png", n - 1));
        std::fs::write(&file, png(p.width as u32, p.height as u32, &p.to_rgba()))?;
        println!("{} (the last picture)", file.display());
    }
    println!("{n} pictures");
    Ok(())
}
