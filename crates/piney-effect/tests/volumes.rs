//! The effects built on every disc present: what a field's effects
//! (`Effects`) and a stream's (`StreamEffects`) build from the volume's
//! tables and `DATA.BIN`.

use std::path::PathBuf;

use piney_data::archive::Archive;
use piney_data::iso::Iso;

#[test]
fn effects_on_every_disc() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    let mut failed = Vec::new();
    for dir in ["infection", "mutation", "outbreak", "quarantine"] {
        let path = root.join(dir).join(format!("{dir}.iso"));
        if !path.exists() {
            eprintln!("skipped {dir}: no disc image");
            continue;
        }
        let mut iso = Iso::open(&path).unwrap();
        let volume = iso.volume().unwrap();
        let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
        if let Err(e) = piney_effect::StreamEffects::new(&archive, volume) {
            failed.push(format!("{dir} stream effects: {e}"));
        }
        if let Err(e) = piney_effect::Effects::new(&archive, volume) {
            failed.push(format!("{dir} effects: {e}"));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
