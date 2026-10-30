//! What piney-build writes of the port's own into a disc: the data the port
//! takes from the disc ([`port_files`], under `PINEY/`), made once from the
//! disc's executable and files so that play never reads the executable
//! (`plans/build-data.md`). A build keeps them in each disc's manifest;
//! a disc image played on its own keeps them in its data folder in the
//! port's home ([`image_data`]).

use std::path::Path;

use piney_data::iso::Iso;
use piney_data::pack::{self, DATA_VERSION, VERSION_FILE};
use piney_data::volume::Volume;

/// The disc the generator reads volume `v`'s files from (its executable,
/// overlays and `DATA.BIN`), from here on. The later volumes also need
/// Infection's: their carried names and the tables' layouts are made from
/// it. A volume with no disc named is read from the discs extracted in
/// `work/` (the tools' copy).
pub fn use_disc(v: Volume, disc: &Path) {
    piney_gen::source::use_disc(piney_gen::volume::VOLUMES[v as usize], disc.to_path_buf());
}

/// Every port file of a disc, by its path under `PINEY/`: the data
/// version, the event scripts and messages, and the generator's groups
/// whose values are in the build (`piney_gen::data::IN_BUILD`).
pub fn port_files(disc: &mut Iso) -> Result<Vec<(String, Vec<u8>)>, String> {
    let v = disc.volume().map_err(|e| e.to_string())?;
    let off = piney_event::official::load_iso(disc).map_err(|e| format!("{v}'s scripts: {e}"))?;
    let mut files = vec![
        (VERSION_FILE.to_string(), format!("{DATA_VERSION}\n").into_bytes()),
        (
            piney_event::official::EVENTS_FILE.to_string(),
            piney_event::official::events_text(v, &off.events).into_bytes(),
        ),
    ];
    // The generator reads the volume's files from the disc named for it
    // ([`use_disc`]).
    let gv = piney_gen::volume::VOLUMES[v as usize];
    for g in piney_gen::manifest::groups()?.iter().filter(|g| piney_gen::data::in_build(g)) {
        let bytes = piney_gen::data::group_bytes(g, gv)?;
        files.push((format!("{}/{}.bin", piney_data::store::TABLES_DIR, g.name), bytes));
    }
    for (name, bytes_of) in piney_gen::placement::GROUPS {
        if let Some(bytes) = bytes_of(gv).map_err(|e| format!("{name}: {e}"))? {
            files.push((format!("{}/{name}.bin", piney_data::store::TABLES_DIR), bytes));
        }
    }
    Ok(files)
}

/// Whether volume `v`'s data can be made: a later volume's needs
/// Infection's disc beside it, once (the tables' layouts are Infection's
/// DWARF, and its globals are found by Infection's own; the owner chose
/// that over committing them). An error to show the player, where the
/// generator would end the process.
pub fn infection_beside(v: Volume) -> Result<(), String> {
    if v == Volume::Inf {
        return Ok(());
    }
    let inf = piney_gen::volume::Vol::Inf;
    piney_gen::source::read(inf, inf.exe())
        .map(|_| ())
        .map_err(|e| format!("{v}'s data is made once with Infection's disc beside it; give its image too ({e})"))
}

/// A disc image's port data, made in its data folder
/// ([`pack::image_data_dir`]) when missing or of another version; true
/// when it was made. A build's disc has its own and is left alone.
pub fn image_data(image: &Path) -> Result<bool, String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", image.display());
    let mut iso = Iso::open(image).map_err(|e| err(&e))?;
    if iso.is_pack() || pack::has_port_data(&mut iso) {
        return Ok(false);
    }
    let dir = iso.data_dir().ok_or_else(|| err(&"no port folder to keep its data in"))?.to_path_buf();
    let v = iso.volume().map_err(|e| err(&e))?;
    use_disc(v, image);
    infection_beside(v).map_err(|e| err(&e))?;
    let files = port_files(&mut iso).map_err(|e| err(&e))?;
    // The version last (it is first in the list): until it is there, the
    // data counts as missing.
    for (path, bytes) in files.iter().rev() {
        let rest = path.split_once('/').map_or(path.as_str(), |(_, r)| r);
        let target = dir.join(rest);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| err(&e))?;
        }
        let part = target.with_extension("part");
        std::fs::write(&part, bytes).map_err(|e| err(&e))?;
        std::fs::rename(&part, &target).map_err(|e| err(&e))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A later volume without Infection's disc is an error to show, not
    /// the generator's end of the process; Infection needs no other.
    #[test]
    fn a_later_volume_asks_for_infections_disc() {
        assert_eq!(infection_beside(Volume::Inf), Ok(()));
        use_disc(Volume::Inf, Path::new("/nonexistent/infection.iso"));
        let e = infection_beside(Volume::Mut).unwrap_err();
        assert!(e.contains("Infection's disc"), "{e}");
    }
}
