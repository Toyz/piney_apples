//! The memory card seam: what `ccSaveSys::MainProccess` asks of `ccMcard`
//! (INF SLUS_202.67, mcard.cpp), as a trait the runtime implements, and a card
//! kept as plain files (`docs/formats/save.md`). Every call completes at once,
//! so the "Saving...." frames the console shows while `ccMcard` waits on
//! `sceMcSync` do not happen here (see `savesys.rs`). Each call takes the card
//! port, 0 or 1; the game always passes slot 0 of the port's multitap.

use std::fs;
use std::path::{Path, PathBuf};

use piney_data::volume::Volume;

use crate::savesys::{INDEX_SIZE, INFO_COUNT, INFO_SIZE, SaveDataInfo, slot_size};

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

    /// `ccMcard::ReadSys(port, 0, buf, 336, vol)` (0x00166a10): volume
    /// `vol`'s (1-4) index file `<dir>/<dir>`, at most 336 bytes of it: the
    /// disc's own (`volumeNum`), or the previous volume's for CONVERT;
    /// `None` when it cannot be opened or read (ReadSys returns 5).
    fn read_index(&mut self, port: i32, vol: i32) -> Option<Vec<u8>>;

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

    fn read_index(&mut self, _port: i32, _vol: i32) -> Option<Vec<u8>> {
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
        self.index_path_of(port, self.volume.number())
    }

    /// Volume `vol`'s (1-4) index file on `port`'s card.
    pub fn index_path_of(&self, port: i32, vol: i32) -> Option<PathBuf> {
        let dir = dir_name(self.volume, vol);
        Some(self.card(port)?.join(dir).join(dir))
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

    fn read_index(&mut self, port: i32, vol: i32) -> Option<Vec<u8>> {
        let mut b = fs::read(self.index_path_of(port, vol)?).ok()?;
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

/// The slot of a file named `dhdata01` to `dhdata12`.
fn slot_of(name: &str) -> Option<usize> {
    (0..INFO_COUNT).find(|&s| slot_file_name(s) == name)
}

/// Slot files given without their directory's index: `src` itself, or the
/// `dhdataNN` files of the directory `src`. Empty when there are none.
fn lone_slots(src: &Path) -> Result<Vec<(usize, Vec<u8>)>, String> {
    let err = |e: std::io::Error| format!("{}: {e}", src.display());
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if src.is_file() {
        return Ok(match slot_of(&name(src)) {
            Some(s) => vec![(s, fs::read(src).map_err(err)?)],
            None => Vec::new(),
        });
    }
    let mut out = Vec::new();
    for e in fs::read_dir(src).map_err(err)? {
        let p = e.map_err(err)?.path();
        if let (true, Some(s)) = (p.is_file(), slot_of(&name(&p))) {
            out.push((s, fs::read(&p).map_err(err)?));
        }
    }
    out.sort();
    Ok(out)
}

/// The volume a lone slot file is of: Infection's size (0x8530) is
/// Infection's; the later volumes' (0x8d84, also an Infection slot as a
/// PCSX2 export can hold it) is `hint`'s, the disc being played.
fn slot_volume(len: usize, hint: Option<Volume>) -> Result<Volume, String> {
    match len {
        n if n == piney_data::save::SIZE => Ok(Volume::Inf),
        n if n == piney_data::save::FULL => {
            hint.ok_or_else(|| "a slot of 0x8d84 bytes: which part's is it? (--volume N)".to_string())
        }
        n => Err(format!("a slot of {n:#x} bytes is no .hack save")),
    }
}

/// The index record `ccSaveSys`'s save writes for a slot file's bytes
/// (`SaveDataInfo`: used, level, clear and parody flags, the player's
/// name, the sum of the file's bytes, the play time).
fn slot_record(bytes: &[u8]) -> Result<[u8; INFO_SIZE], String> {
    let save = piney_data::save::SaveData::from_bytes(bytes).map_err(|e| e.to_string())?;
    let mut name = [0u8; 18];
    let b = save.bytes();
    for (d, &c) in name.iter_mut().zip(b[piney_data::save::offset::PL_NAME..].iter().take(17).take_while(|&&c| c != 0))
    {
        *d = c;
    }
    let info = SaveDataInfo {
        status: 1,
        level: save.level(),
        clear_flag: save.clear_flag(),
        parody_flag: save.u8(piney_data::save::offset::PARODY_FLAG) as i8,
        name,
        sum: bytes.iter().fold(0u16, |s, &c| s.wrapping_add(u16::from(c))),
        playtime: save.play_time(),
    };
    Ok(info.to_bytes())
}

/// Lone slot files onto the card: into their volume's directory (made as
/// a card's `make_dir` makes it when missing), each with its index record
/// rebuilt from it, so the load screens see them used. The directory as it
/// was is copied to `backup` first.
fn import_slots(
    card: &Path,
    slots: Vec<(usize, Vec<u8>)>,
    hint: Option<Volume>,
    backup: &Path,
) -> Result<Vec<(String, usize)>, String> {
    let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let mut done = Vec::new();
    for volume in Volume::ALL {
        let mine: Vec<&(usize, Vec<u8>)> =
            slots.iter().filter(|(_, b)| slot_volume(b.len(), hint).is_ok_and(|v| v == volume)).collect();
        if mine.is_empty() {
            continue;
        }
        let dir_name = own_dir_name(volume);
        let dir = card.join(dir_name);
        let index_path = dir.join(dir_name);
        if dir.is_dir() {
            let to = backup.join(dir_name);
            fs::create_dir_all(&to).map_err(|e| err(&to, e))?;
            for e in fs::read_dir(&dir).map_err(|e| err(&dir, e))? {
                let p = e.map_err(|e| err(&dir, e))?.path();
                if p.is_file() {
                    fs::copy(&p, to.join(p.file_name().unwrap_or_default())).map_err(|e| err(&p, e))?;
                }
            }
        } else {
            fs::create_dir_all(&dir).map_err(|e| err(&dir, e))?;
            let zero = vec![0u8; slot_size(volume)];
            for s in 0..INFO_COUNT {
                let p = dir.join(slot_file_name(s));
                fs::write(&p, &zero).map_err(|e| err(&p, e))?;
            }
        }
        let mut index = fs::read(&index_path).unwrap_or_default();
        index.resize(INDEX_SIZE, 0);
        for &(slot, bytes) in &mine {
            let p = dir.join(slot_file_name(*slot));
            fs::write(&p, bytes).map_err(|e| err(&p, e))?;
            index[slot * INFO_SIZE..(slot + 1) * INFO_SIZE].copy_from_slice(&slot_record(bytes)?);
        }
        fs::write(&index_path, &index).map_err(|e| err(&index_path, e))?;
        done.push((dir_name.to_string(), mine.len()));
    }
    if done.is_empty() {
        // Every slot was refused: say why for the first.
        if let Some((_, b)) = slots.first() {
            slot_volume(b.len(), hint)?;
        }
    }
    Ok(done)
}

/// Copy the `.hack` saves of `src` (see [`find_saves`]) onto the card
/// directory `card`, each save directory whole; one it replaces is moved
/// to `card`'s sibling `backup-<secs>` first. With no save directory, lone
/// slot files (`dhdata01`-`12`, the file or a directory of them) go into
/// their volume's directory, their index records rebuilt ([`import_slots`];
/// `hint` the volume a later part's size is taken for). The directories
/// written, with their files' count.
pub fn import(src: &Path, card: &Path, hint: Option<Volume>) -> Result<Vec<(String, usize)>, String> {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let beside = card.parent().unwrap_or(card);
    let backup = (0..)
        .map(|k| beside.join(if k == 0 { format!("backup-{secs}") } else { format!("backup-{secs}-{k}") }))
        .find(|p| !p.exists())
        .unwrap_or_else(|| beside.join("backup"));
    let slots = lone_slots(src)?;
    let found = if src.is_file() && !slots.is_empty() { Vec::new() } else { find_saves(src)? };
    if found.is_empty() {
        if !slots.is_empty() {
            return import_slots(card, slots, hint, &backup);
        }
        return Err(format!("{}: no .hack save directory ({})", src.display(), dothack_dirs().join(", ")));
    }
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
        assert!(c.read_index(0, 1).is_none());
        assert!(!c.write_index(0, &[0; INDEX_SIZE]));
        assert!(c.make_dir(0));
        assert_eq!(c.check_port(0), PortState::Ready);
        assert_eq!(c.read_index(0, 1), Some(vec![0; INDEX_SIZE]));
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
        assert_eq!(import(&src, &card, None).unwrap(), vec![(dir.to_string(), 1)]);
        assert_eq!(fs::read(card.join(dir).join("dhdata01")).unwrap(), vec![1u8; 64]);
        assert!(!card.join(dir).join("_pcsx2_index").exists());
        let backup = fs::read_dir(card.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().starts_with("backup-"))
            .expect("the replaced save kept");
        assert_eq!(fs::read(backup.path().join(dir).join("dhdata01")).unwrap(), vec![9u8; 8]);
        assert_eq!(import(&src.join(dir), &card, None).unwrap(), vec![(dir.to_string(), 1)]);
        assert!(import(&root.join("memcard"), &root.join("other"), None).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    /// A slot file given alone (a player's `dhdata12`): written into its
    /// volume's directory with its index record rebuilt as the save writes
    /// it, so the load screen sees it used; the other slots' records kept.
    /// A later part's size needs the volume; one of no save's size is
    /// refused.
    #[test]
    fn a_lone_slot_imports_with_its_record() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("piney-lone-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let card = root.join("memcard").join("slot1");
        fs::create_dir_all(&root).unwrap();
        let mut save = piney_data::save::SaveData::new();
        save.bytes_mut()[..5].copy_from_slice(b"Kite\0");
        let lone = root.join("dhdata12");
        fs::write(&lone, save.bytes()).unwrap();
        let dir = own_dir_name(Volume::Inf);
        assert_eq!(import(&lone, &card, None).unwrap(), vec![(dir.to_string(), 1)]);
        let mut c = FilesCard::slot1(Volume::Inf, &card);
        let index: [u8; INDEX_SIZE] = c.read_index(0, 1).unwrap().try_into().unwrap();
        assert!(crate::savesys::check_right_info(&index));
        let r = SaveDataInfo::from_bytes(&index[11 * INFO_SIZE..]);
        assert_eq!((r.status, r.name(), r.sum), (1, &b"Kite"[..], save.sum()));
        assert!((0..11).all(|s| index[s * INFO_SIZE] == 0), "the other slots empty");
        assert_eq!(c.read_slot(0, 11, 1, piney_data::save::SIZE).unwrap(), save.bytes().to_vec());
        let later = root.join("dhdata01");
        fs::write(&later, save.record()).unwrap();
        assert!(import(&later, &card, None).is_err(), "a later part's slot needs its volume");
        let mdir = own_dir_name(Volume::Mut);
        assert_eq!(import(&later, &card, Some(Volume::Mut)).unwrap(), vec![(mdir.to_string(), 1)]);
        fs::write(&later, [0u8; 100]).unwrap();
        assert!(import(&later, &card, None).is_err());
        let _ = fs::remove_dir_all(&root);
    }
}
