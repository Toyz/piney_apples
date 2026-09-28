//! Writes every event of a disc in the text form, one file per event.
//!
//!     cargo run -p piney-event --example export [ISO] [OUT]
//!
//! ISO defaults to `work/infection/infection.iso`; OUT to `script-text`
//! beside it, and must be inside the repository's `work/` directory (these
//! per-event files are a working copy; the port's own copy is the build's
//! `PINEY/EVENTS.EVS`, `plans/build-data.md`).

use std::path::{Path, PathBuf};

use piney_data::iso::Iso;
use piney_event::{official, text};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let work = root.join("work");
    let mut args = std::env::args().skip(1);
    let iso = args.next().map(PathBuf::from).unwrap_or_else(|| work.join("infection/infection.iso"));
    let out =
        args.next().map(PathBuf::from).unwrap_or_else(|| iso.parent().unwrap_or(Path::new(".")).join("script-text"));
    let out = normalise(&std::env::current_dir()?.join(out));
    if !out.starts_with(&work) {
        return Err(
            format!("{} is outside {}: the scripts are not to leave work/", out.display(), work.display()).into()
        );
    }
    std::fs::create_dir_all(&out)?;
    let mut disc = Iso::open(&iso)?;
    let off = official::load_iso(&mut disc)?;
    for e in &off.events {
        std::fs::write(out.join(format!("event-{:03}.txt", e.number)), text::print_event(e))?;
    }
    println!("{:?}: {} events written to {}", off.layout.dialect, off.events.len(), out.display());
    Ok(())
}

/// The path with `.` and `..` resolved, without touching the file system.
fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}
