//! The memory card seam: what `ccSaveSys::MainProccess` asks of `ccMcard`
//! (INF SLUS_202.67, mcard.cpp), as a trait the runtime implements, and a card
//! kept as plain files (`docs/formats/save.md`). Every call completes at once,
//! so the "Saving...." frames the console shows while `ccMcard` waits on
//! `sceMcSync` do not happen here (see `savesys.rs`). Each call takes the card
//! port, 0 or 1; the game always passes slot 0 of the port's multitap.

use std::fs;
use std::path::{Path, PathBuf};

use piney_data::volume::Volume;

use crate::savesys::{INDEX_SIZE, INFO_COUNT, slot_size};

/// `mcDirName[vol - 1]` on the disc of `volume` (`INF SLUS_202.67:0x00306be0`)
/// without its leading `/`: volume `vol`'s (1-4) save directory, which is
/// also its index file's name.
pub fn dir_name(volume: Volume, vol: i32) -> &'static str {
    let names = &piney_data::tables::title::of(volume).mc_dir_names;
    let name = names[usize::try_from(vol - 1).unwrap_or(0).min(3)];
    name.strip_prefix('/').unwrap_or(name)
}

/// The disc's own save directory (`mcDirName[volumeNum - 1]`).
pub fn own_dir_name(volume: Volume) -> &'static str {
    dir_name(volume, volume.number())
}

/// `mcFname[slot + 1]` (0x00306d40): slot `slot`'s file, `dhdata01` to
/// `dhdata12`.
pub fn slot_file_name(slot: usize) -> String {
    format!("dhdata{:02}", slot + 1)
}

/// What `ccMcard::CheckPort(port, 0)` (0x00166c50) reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortState {
    /// -1: the save directory is there (or the card has room for the rest
    /// of a partial one).
    Ready,
    /// 0: no card in the port.
    NoCard,
    /// 1: not a PlayStation 2 memory card.
    NotPs2,
    /// 2: not formatted.
    Unformatted,
    /// 3: no save directory, and less than the 685 KB it needs free.
    Full,
    /// 4: no save directory, room for one.
    NoDirectory,
}

impl PortState {
    /// CheckPort's return value.
    pub fn code(self) -> i32 {
        match self {
            PortState::Ready => -1,
            PortState::NoCard => 0,
            PortState::NotPs2 => 1,
            PortState::Unformatted => 2,
            PortState::Full => 3,
            PortState::NoDirectory => 4,
        }
    }

    pub fn from_code(c: i32) -> Option<PortState> {
        Some(match c {
            -1 => PortState::Ready,
            0 => PortState::NoCard,
            1 => PortState::NotPs2,
            2 => PortState::Unformatted,
            3 => PortState::Full,
            4 => PortState::NoDirectory,
            _ => return None,
        })
    }
}

/// The memory card calls `ccSaveSys` makes, one method each. A `false` or
/// `None` is the call's failure code, which `MainProccess` turns into its
/// message.
pub trait MemoryCard {
    /// `ccMcard::CheckPort(port, 0)`.
    fn check_port(&mut self, port: i32) -> PortState;

    /// `ccMcard::ReadSys(port, 0, buf, 336, volumeNum)` (0x00166a10): the
    /// index file `<dir>/<dir>`, at most 336 bytes of it; `None` when it
    /// cannot be opened or read (ReadSys returns 5).
    fn read_index(&mut self, port: i32) -> Option<Vec<u8>>;

    /// `ccMcard::SaveSys(port, 0, info, 336)` (0x001664b0): write the index
    /// file; false when that fails (SaveSys returns 6).
    fn write_index(&mut self, port: i32, index: &[u8; INDEX_SIZE]) -> bool;

    /// `ccMcard::DataWrite(port, 0, slot, data, size)` (0x001661d0): write
    /// slot file `slot` (0-11), [`slot_size`] bytes; false when that fails
    /// (6).
    fn write_slot(&mut self, port: i32, slot: usize, data: &[u8]) -> bool;

    /// `ccMcard::Format(port, 0)` (0x00165930); false when it fails (8).
    fn format(&mut self, port: i32) -> bool;

    /// `ccMcard::MakeDir(port, 0)` (0x001659b0): the save directory with
    /// `icon.sys`, the three icons and all twelve slot files, then the
    /// index; false when any of it fails (9 or 6).
    fn make_dir(&mut self, port: i32) -> bool;

    /// `ccMcard::DataRead(port, 0, slot, buf, size, vol)` (0x00166750, the
    /// loads): slot file `slot` (0-11) of volume `vol`'s directory (1-4:
    /// the disc's own, or the previous volume's), the whole file when it
    /// holds at least `size` bytes (an Infection slot may be the later
    /// volumes' 0x8d84 on a card from PCSX2); `None` when it cannot be read
    /// (DataRead returns 5). A card that only saves need not have it.
    fn read_slot(&mut self, _port: i32, _slot: usize, _vol: i32, _size: usize) -> Option<Vec<u8>> {
        None
    }
}

/// Neither port holds a card: every Data screen visit ends at "memory card
/// ... is not inserted".
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCard;

impl MemoryCard for NoCard {
    fn check_port(&mut self, _port: i32) -> PortState {
        PortState::NoCard
    }

    fn read_index(&mut self, _port: i32) -> Option<Vec<u8>> {
        None
    }

    fn write_index(&mut self, _port: i32, _index: &[u8; INDEX_SIZE]) -> bool {
        false
    }

    fn write_slot(&mut self, _port: i32, _slot: usize, _data: &[u8]) -> bool {
        false
    }

    fn format(&mut self, _port: i32) -> bool {
        false
    }

    fn make_dir(&mut self, _port: i32) -> bool {
        false
    }
}

/// Memory cards kept as directories, one per port, each holding what the
/// card's file system would for the disc's volume: the index and the twelve
/// `dhdata` slots (`docs/formats/save.md`). A port with no directory has no
/// card; a card is always formatted and roomy, so `check_port` answers `Ready`
/// when the save directory exists and `NoDirectory` otherwise. `make_dir`
/// writes the slots and the index zeroed, without `icon.sys` and the icons;
/// `format` changes nothing (it is asked only of an unformatted card).
#[derive(Clone, Debug)]
pub struct FilesCard {
    /// The disc's volume: its directory and slot size.
    volume: Volume,
    ports: [Option<PathBuf>; 2],
}

impl FilesCard {
    /// Cards at `port1` and `port2` (MEMORY CARD slots 1 and 2), either
    /// absent, as the disc of `volume` uses them.
    pub fn new(volume: Volume, port1: Option<PathBuf>, port2: Option<PathBuf>) -> Self {
        FilesCard { volume, ports: [port1, port2] }
    }

    /// One card, in MEMORY CARD slot 1.
    pub fn slot1(volume: Volume, dir: impl Into<PathBuf>) -> Self {
        FilesCard::new(volume, Some(dir.into()), None)
    }

    /// The card directory of `port`, if it holds one.
    pub fn card(&self, port: i32) -> Option<&Path> {
        let p = self.ports.get(usize::try_from(port).ok()?)?.as_deref()?;
        p.is_dir().then_some(p)
    }

    /// `<card>/BASLUS-20267DOTHACK` (the disc's directory).
    pub fn save_dir(&self, port: i32) -> Option<PathBuf> {
        Some(self.card(port)?.join(own_dir_name(self.volume)))
    }

    /// The index file of `port`'s card.
    pub fn index_path(&self, port: i32) -> Option<PathBuf> {
        Some(self.save_dir(port)?.join(own_dir_name(self.volume)))
    }

    /// Slot file `slot` (0-11) of `port`'s card in volume `vol`'s
    /// directory.
    pub fn slot_path(&self, port: i32, slot: usize, vol: i32) -> Option<PathBuf> {
        Some(self.card(port)?.join(dir_name(self.volume, vol)).join(slot_file_name(slot)))
    }
}

impl MemoryCard for FilesCard {
    fn check_port(&mut self, port: i32) -> PortState {
        match self.save_dir(port) {
            None => PortState::NoCard,
            Some(d) if d.is_dir() => PortState::Ready,
            Some(_) => PortState::NoDirectory,
        }
    }

    fn read_index(&mut self, port: i32) -> Option<Vec<u8>> {
        let mut b = fs::read(self.index_path(port)?).ok()?;
        b.truncate(INDEX_SIZE);
        Some(b)
    }

    fn write_index(&mut self, port: i32, index: &[u8; INDEX_SIZE]) -> bool {
        match self.save_dir(port) {
            Some(d) if d.is_dir() => fs::write(d.join(own_dir_name(self.volume)), index).is_ok(),
            _ => false,
        }
    }

    fn write_slot(&mut self, port: i32, slot: usize, data: &[u8]) -> bool {
        match self.save_dir(port) {
            Some(d) if d.is_dir() && slot < INFO_COUNT => fs::write(d.join(slot_file_name(slot)), data).is_ok(),
            _ => false,
        }
    }

    fn format(&mut self, port: i32) -> bool {
        self.card(port).is_some()
    }

    fn read_slot(&mut self, port: i32, slot: usize, vol: i32, size: usize) -> Option<Vec<u8>> {
        if slot >= INFO_COUNT {
            return None;
        }
        let b = fs::read(self.slot_path(port, slot, vol)?).ok()?;
        (b.len() >= size).then_some(b)
    }

    fn make_dir(&mut self, port: i32) -> bool {
        let Some(d) = self.save_dir(port) else { return false };
        if fs::create_dir_all(&d).is_err() {
            return false;
        }
        let zero = vec![0u8; slot_size(self.volume)];
        (0..INFO_COUNT).all(|s| fs::write(d.join(slot_file_name(s)), &zero).is_ok())
            && fs::write(d.join(own_dir_name(self.volume)), [0u8; INDEX_SIZE]).is_ok()
    }
}

/// The four volumes' save directories, as their discs name them.
fn dothack_dirs() -> Vec<&'static str> {
    Volume::ALL.iter().map(|&v| own_dir_name(v)).collect()
}

/// A save directory's files, as found in an import's source.
struct Found {
    dir: String,
    files: Vec<(String, Vec<u8>)>,
}

/// The `.hack` save directories of `src`: a PCSX2 card image (`.ps2`), a
/// PCSX2 folder card (its save directories inside), or one save directory
/// exported on its own. PCSX2's own files (`_pcsx2_*`) are left out.
fn find_saves(src: &Path) -> Result<Vec<Found>, String> {
    let names = dothack_dirs();
    let err = |e: std::io::Error| format!("{}: {e}", src.display());
    if src.is_file() {
        let card = piney_data::ps2card::Card::open(fs::read(src).map_err(err)?)
            .map_err(|e| format!("{}: {e}", src.display()))?;
        let mut out = Vec::new();
        for d in card.root().map_err(|e| e.to_string())? {
            if !d.is_dir() || !names.contains(&d.name.as_str()) {
                continue;
            }
            let mut files = Vec::new();
            for f in card.dir(&d).map_err(|e| e.to_string())?.into_iter().filter(|f| f.is_file()) {
                files.push((f.name.clone(), card.file(&f).map_err(|e| format!("{}/{}: {e}", d.name, f.name))?));
            }
            out.push(Found { dir: d.name, files });
        }
        return Ok(out);
    }
    let read_dir = |dir: &Path| -> Result<Vec<(String, Vec<u8>)>, String> {
        let mut files = Vec::new();
        for e in fs::read_dir(dir).map_err(err)? {
            let p = e.map_err(err)?.path();
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if p.is_file() && !name.starts_with("_pcsx2") {
                files.push((name, fs::read(&p).map_err(err)?));
            }
        }
        files.sort();
        Ok(files)
    };
    let own = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if names.contains(&own.as_str()) {
        return Ok(vec![Found { dir: own, files: read_dir(src)? }]);
    }
    let mut out = Vec::new();
    for name in names {
        let d = src.join(name);
        if d.is_dir() {
            out.push(Found { dir: name.to_string(), files: read_dir(&d)? });
        }
    }
    Ok(out)
}

/// Copy the `.hack` saves of `src` (see [`find_saves`]) onto the card
/// directory `card`, each save directory whole; one it replaces is moved
/// to `card`'s sibling `backup-<secs>` first. The directories copied, with
/// their files' count.
pub fn import(src: &Path, card: &Path) -> Result<Vec<(String, usize)>, String> {
    let found = find_saves(src)?;
    if found.is_empty() {
        return Err(format!("{}: no .hack save directory ({})", src.display(), dothack_dirs().join(", ")));
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let beside = card.parent().unwrap_or(card);
    let backup = (0..)
        .map(|k| beside.join(if k == 0 { format!("backup-{secs}") } else { format!("backup-{secs}-{k}") }))
        .find(|p| !p.exists())
        .unwrap_or_else(|| beside.join("backup"));
    let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let mut done = Vec::new();
    for f in found {
        let dst = card.join(&f.dir);
        if dst.exists() {
            fs::create_dir_all(&backup).map_err(|e| err(&backup, e))?;
            fs::rename(&dst, backup.join(&f.dir)).map_err(|e| err(&dst, e))?;
        }
        fs::create_dir_all(&dst).map_err(|e| err(&dst, e))?;
        for (name, bytes) in &f.files {
            let p = dst.join(name);
            fs::write(&p, bytes).map_err(|e| err(&p, e))?;
        }
        done.push((f.dir, f.files.len()));
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_state_codes_round_trip() {
        for c in -1..=4 {
            assert_eq!(PortState::from_code(c).map(PortState::code), Some(c));
        }
        assert_eq!(PortState::from_code(5), None);
        assert_eq!(slot_file_name(0), "dhdata01");
        assert_eq!(slot_file_name(11), "dhdata12");
    }

    #[test]
    fn files_card_layout() {
        let root = std::env::temp_dir().join(format!("piney-card-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut c = FilesCard::slot1(Volume::Inf, &root);
        assert_eq!(c.check_port(0), PortState::NoCard);
        assert_eq!(c.check_port(1), PortState::NoCard);
        fs::create_dir_all(&root).unwrap();
        assert_eq!(c.check_port(0), PortState::NoDirectory);
        assert!(c.read_index(0).is_none());
        assert!(!c.write_index(0, &[0; INDEX_SIZE]));
        assert!(c.make_dir(0));
        assert_eq!(c.check_port(0), PortState::Ready);
        assert_eq!(c.read_index(0), Some(vec![0; INDEX_SIZE]));
        let size = slot_size(Volume::Inf);
        assert!(c.write_slot(0, 11, &vec![7; size]));
        assert_eq!(fs::read(root.join("BASLUS-20267DOTHACK").join("dhdata12")).unwrap(), vec![7; size]);
        assert_eq!(c.read_slot(0, 11, 1, size), Some(vec![7; size]));
        assert_eq!(c.read_slot(1, 11, 1, size), None);
        fs::remove_dir_all(&root).unwrap();
    }

    /// A PCSX2 save imported onto the port's card: a folder card's save
    /// directory copied whole but for PCSX2's own `_pcsx2_index`, the
    /// directory it replaces moved to a `backup-` beside the card; one
    /// exported save directory given on its own works the same; a folder
    /// with no .hack save is refused.
    #[test]
    fn a_pcsx2_save_imports() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("piney-import-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (src, card) = (root.join("pcsx2"), root.join("memcard").join("slot1"));
        let dir = own_dir_name(Volume::Inf);
        fs::create_dir_all(src.join(dir)).unwrap();
        fs::write(src.join(dir).join("dhdata01"), [1u8; 64]).unwrap();
        fs::write(src.join(dir).join("_pcsx2_index"), b"{}").unwrap();
        fs::create_dir_all(card.join(dir)).unwrap();
        fs::write(card.join(dir).join("dhdata01"), [9u8; 8]).unwrap();
        assert_eq!(import(&src, &card).unwrap(), vec![(dir.to_string(), 1)]);
        assert_eq!(fs::read(card.join(dir).join("dhdata01")).unwrap(), vec![1u8; 64]);
        assert!(!card.join(dir).join("_pcsx2_index").exists());
        let backup = fs::read_dir(card.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().starts_with("backup-"))
            .expect("the replaced save kept");
        assert_eq!(fs::read(backup.path().join(dir).join("dhdata01")).unwrap(), vec![9u8; 8]);
        assert_eq!(import(&src.join(dir), &card).unwrap(), vec![(dir.to_string(), 1)]);
        assert!(import(&root.join("memcard"), &root.join("other")).is_err());
        let _ = fs::remove_dir_all(&root);
    }
}
