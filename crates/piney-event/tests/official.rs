//! The adapter against the discs: every script decodes to the IR and
//! encodes back to identical shorts, and the IR prints to text and parses
//! back to the same IR, and the scripts generated into piney-data are the
//! executable's. Needs the extracted disc images under `work/` (or
//! `PINEY_ISO` for Infection's); a volume whose image is missing is
//! skipped. Nothing read from the disc is written anywhere.

use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_data::volume::Volume;
use piney_event::official::{self, Dialect};
use piney_event::text;

fn image(volume: &str) -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    let p = match (volume, std::env::var_os("PINEY_ISO")) {
        ("infection", Some(p)) => PathBuf::from(p),
        _ => root.join(volume).join(format!("{volume}.iso")),
    };
    p.exists().then_some(p)
}

const VOLUMES: [(&str, Dialect, usize); 4] = [
    ("infection", Dialect::Infection, 192),
    ("mutation", Dialect::Mutation, 196),
    ("outbreak", Dialect::Outbreak, 199),
    ("quarantine", Dialect::Quarantine, 206),
];

#[test]
fn every_script_round_trips() {
    for (vol, dialect, scripts) in VOLUMES {
        let Some(path) = image(vol) else {
            eprintln!("{vol}: no image, skipped");
            continue;
        };
        let mut iso = Iso::open(&path).unwrap();
        let exe = official::boot_executable(&mut iso).unwrap();
        let layout = official::locate(&exe).unwrap();
        assert_eq!(layout.dialect, dialect, "{vol}");
        let raws = official::raw_scripts(&exe, &layout);
        assert_eq!(raws.len(), scripts, "{vol}");
        let mut blocks = 0;
        for raw in &raws {
            let (script, used) =
                official::decode(dialect, &raw.shorts).unwrap_or_else(|e| panic!("{vol} event {}: {e}", raw.event));
            // The -1 is the array's last short: nothing left over.
            assert_eq!(used, raw.shorts.len(), "{vol} event {}", raw.event);
            let back = official::encode(dialect, &script).unwrap();
            assert_eq!(back, raw.shorts, "{vol} event {}", raw.event);
            let printed = text::print_script(&script);
            let parsed = text::parse_script(&printed).unwrap_or_else(|e| panic!("{vol} event {}: {e}", raw.event));
            assert_eq!(parsed, script, "{vol} event {}", raw.event);
            blocks += script.blocks.len();
        }
        eprintln!("{vol}: {} scripts, {blocks} blocks round-trip", raws.len());
    }
}

#[test]
fn events_with_messages_round_trip_as_text() {
    let Some(path) = image("infection") else { return };
    let mut iso = Iso::open(&path).unwrap();
    let off = official::load_iso(&mut iso).unwrap();
    assert_eq!(off.events.len(), 192);
    let mut records = 0;
    for e in &off.events {
        let printed = text::print_event(e);
        let parsed = text::parse_event(&printed).unwrap_or_else(|err| panic!("event {}: {err}", e.number));
        assert_eq!(&parsed, e, "event {}", e.number);
        records += e.messages.len() + e.parody.len();
    }
    assert!(records > 0);
    eprintln!("infection: {} events, {records} message records round-trip as text", off.events.len());
}

/// The text form the build writes into a disc's port data
/// (`official::events_text`) reads back as the executable's scripts and
/// messages, event for event, on every volume.
#[test]
fn the_builds_scripts_are_the_executables() {
    for (vol, v) in
        [("infection", Volume::Inf), ("mutation", Volume::Mut), ("outbreak", Volume::Out), ("quarantine", Volume::Qua)]
    {
        let Some(path) = image(vol) else { continue };
        let mut iso = Iso::open(&path).unwrap();
        let off = official::load_iso(&mut iso).unwrap();
        let text = official::events_text(v, &off.events);
        let read = piney_event::text::parse_events(&text).unwrap();
        assert_eq!(read.len(), off.events.len(), "{vol}");
        for (r, e) in read.iter().zip(&off.events) {
            assert_eq!(r, e, "{vol} event {}", e.number);
        }
        assert_eq!(official::events(&mut iso).unwrap(), off.events, "{vol}");
        assert_eq!(official::events_of(v).unwrap(), off.events, "{vol}");
    }
}
