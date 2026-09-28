//! Not part of the build: what keeping the gzip archives (`DATA.BIN`,
//! `STREAM/*.BIN`) as their members' inflated contents would save.
//!
//!     cargo run --release -p piney-build --example estimate -- ISO...
//!
//! Every chunk is deduplicated after inflating and zstd-compressed at
//! `LEVEL` (3 by default); `RAWONLY=1` chunks the files as they are, as the
//! build does. On the four discs (2026-09-27): members at level 3,
//! 7,136.5 MiB; the files as they are at level 3, 7,254.3 MiB, at 9,
//! 7,233.1, at 15, 7,136.9. The members save nothing over level 15 on the
//! files, so the build keeps the files as they are and the port's archive
//! readers stay as they are.
use std::collections::HashSet;
use std::io::Read;

use piney_data::iso::Iso;

fn main() {
    let level: i32 = std::env::var("LEVEL").ok().and_then(|l| l.parse().ok()).unwrap_or(3);
    let mut seen: HashSet<[u8; 32]> = HashSet::new();
    let (mut raw_total, mut stored, mut plain_new, mut member_new) = (0u64, 0u64, 0u64, 0u64);
    for path in std::env::args().skip(1) {
        let mut disc = Iso::open(&path).unwrap();
        let mut entries = disc.list().unwrap();
        entries.sort_by_key(|e| e.lba);
        let before = stored;
        for e in entries.iter().filter(|e| !e.dir) {
            raw_total += u64::from(e.size);
            let upper = e.path.to_ascii_uppercase();
            let gz = std::env::var_os("RAWONLY").is_none()
                && (upper == "DATA/DATA.BIN" || (upper.starts_with("STREAM/") && upper.ends_with(".BIN")));
            let data = disc.read(e).unwrap();
            let mut pieces: Vec<Vec<u8>> = Vec::new();
            if gz {
                let starts: Vec<usize> = (0..data.len())
                    .step_by(2048)
                    .filter(|&o| data.get(o..o + 3) == Some(&[0x1f, 0x8b, 0x08][..]))
                    .collect();
                for (k, &o) in starts.iter().enumerate() {
                    let end = starts.get(k + 1).copied().unwrap_or(data.len());
                    let mut out = Vec::new();
                    flate2::read::GzDecoder::new(&data[o..end]).read_to_end(&mut out).unwrap();
                    pieces.push(out);
                }
            } else {
                pieces.push(data);
            }
            for p in pieces {
                for c in fastcdc::v2020::FastCDC::new(&p, 16 * 1024, 64 * 1024, 256 * 1024) {
                    let d = &p[c.offset..c.offset + c.length];
                    if seen.insert(*blake3::hash(d).as_bytes()) {
                        let z = zstd::bulk::compress(d, level).unwrap();
                        let n = z.len().min(d.len()) as u64;
                        stored += n;
                        if gz { member_new += n } else { plain_new += n }
                    }
                }
            }
        }
        println!("{path}: {:.1} MiB new", (stored - before) as f64 / 1048576.0);
    }
    println!(
        "files {:.1} MiB -> {:.1} MiB stored ({:.1}%): archives' members {:.1} MiB, other files {:.1} MiB",
        raw_total as f64 / 1048576.0,
        stored as f64 / 1048576.0,
        100.0 * stored as f64 / raw_total as f64,
        member_new as f64 / 1048576.0,
        plain_new as f64 / 1048576.0
    );
}
