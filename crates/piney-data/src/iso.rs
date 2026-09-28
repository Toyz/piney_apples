//! ISO 9660 images, read directly. PS2 DVDs are plain ISO 9660 (no Joliet,
//! no Rock Ridge) with 2048-byte sectors (`docs/disc/layout.md`).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::{Bytes, Error, Result, format_err};

pub const SECTOR: u64 = 2048;

#[derive(Clone, Debug)]
pub struct Entry {
    /// The path with `/` separators and the `;1` version stripped.
    pub path: String,
    pub lba: u32,
    pub size: u32,
    pub dir: bool,
}

/// Set by [`deny_executable`].
static DENY_EXECUTABLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// From here on, a read of the boot executable (`SLUS_*`) fails: the game
/// calls it at start, so a reader that still wants the executable fails
/// loudly (`plans/volumes.md`). The tools and the checks, which compare with
/// the executable, do not.
pub fn deny_executable() {
    DENY_EXECUTABLE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Where a disc's files come from.
enum Source {
    /// A disc image.
    Image(File),
    /// A disc of a build ([`crate::pack`]): its `.disc` file and the build's
    /// `chunks.pak`.
    Pack(crate::pack::PackDisc),
}

pub struct Iso {
    source: Source,
    /// A disc image's port data ([`crate::pack::image_data_dir`]), served
    /// as its `PINEY/` files.
    data: Option<PathBuf>,
    root: Entry,
    /// Which volume the disc is, once asked ([`Iso::volume`]).
    volume: Option<crate::volume::Volume>,
}

impl Iso {
    /// A disc image, or one disc of a build (its `.disc` file): the port
    /// reads both the same way.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if crate::pack::is_disc(path) {
            let pack = crate::pack::PackDisc::open(path)?;
            let volume = Some(pack.manifest.volume);
            let root = Entry { path: String::new(), lba: 0, size: 0, dir: true };
            return Ok(Iso { source: Source::Pack(pack), data: None, root, volume });
        }
        let mut file = File::open(path)?;
        let mut pvd = vec![0u8; SECTOR as usize];
        file.seek(SeekFrom::Start(16 * SECTOR))?;
        file.read_exact(&mut pvd)?;
        if pvd[0] != 1 || &pvd[1..6] != b"CD001" {
            return format_err("no primary volume descriptor at sector 16");
        }
        let root = record(&pvd[156..190], "")?.ok_or_else(|| Error::Format("empty root record".into()))?;
        Ok(Iso { source: Source::Image(file), data: crate::pack::image_data_dir(path), root, volume: None })
    }

    /// Whether the disc is one of a build rather than an image.
    pub fn is_pack(&self) -> bool {
        matches!(self.source, Source::Pack(_))
    }

    /// The image's volume identifier (`HACK_VOL1` and the like).
    pub fn label(&mut self) -> Result<String> {
        match &mut self.source {
            Source::Pack(p) => Ok(p.manifest.label.clone()),
            Source::Image(f) => {
                let mut pvd = vec![0u8; SECTOR as usize];
                f.seek(SeekFrom::Start(16 * SECTOR))?;
                f.read_exact(&mut pvd)?;
                Ok(String::from_utf8_lossy(&pvd[40..72]).trim_end().to_string())
            }
        }
    }

    /// Which of the four discs this is, from its `DATA/GCMN.PRG`
    /// ([`crate::volume`]); read once.
    pub fn volume(&mut self) -> Result<crate::volume::Volume> {
        if let Some(v) = self.volume {
            return Ok(v);
        }
        let gcmn = self.read_path(crate::volume::GCMN_PATH)?;
        let v = crate::volume::Volume::detect(&gcmn).map_err(|e| Error::Format(e.to_string()))?;
        self.volume = Some(v);
        Ok(v)
    }

    pub fn read(&mut self, entry: &Entry) -> Result<Vec<u8>> {
        self.read_at(entry, 0, entry.size as usize)
    }

    /// `len` bytes of a file from `ofs` on, fewer where the file ends.
    pub fn read_at(&mut self, entry: &Entry, ofs: u64, len: usize) -> Result<Vec<u8>> {
        let len = len.min((entry.size as u64).saturating_sub(ofs) as usize);
        match &mut self.source {
            Source::Pack(p) => p.read_at(&entry.path, ofs, len),
            Source::Image(f) => {
                let mut out = vec![0u8; len];
                f.seek(SeekFrom::Start(entry.lba as u64 * SECTOR + ofs))?;
                f.read_exact(&mut out)?;
                Ok(out)
            }
        }
    }

    fn children(&mut self, dir: &Entry) -> Result<Vec<Entry>> {
        if let Source::Pack(p) = &self.source {
            return Ok(p
                .manifest
                .entries
                .iter()
                .filter(|e| e.entry.path.rsplit_once('/').map_or("", |(parent, _)| parent) == dir.path)
                .map(|e| e.entry.clone())
                .collect());
        }
        // A directory's records: read as the image stores it.
        let mut data = vec![0u8; dir.size as usize];
        if let Source::Image(f) = &mut self.source {
            f.seek(SeekFrom::Start(dir.lba as u64 * SECTOR))?;
            f.read_exact(&mut data)?;
        }
        let mut out = Vec::new();
        let mut p = 0usize;
        while p < data.len() {
            let len = data[p] as usize;
            if len == 0 {
                // Records never cross a sector; the rest of this one is padding.
                p = (p / SECTOR as usize + 1) * SECTOR as usize;
                continue;
            }
            let rec = data.slice_at(p, len)?;
            // The root's `.` record comes back as the root itself: not a
            // child (a walk would enter it again and again).
            if let Some(e) = record(rec, &dir.path)?.filter(|e| !e.path.is_empty()) {
                out.push(e);
            }
            p += len;
        }
        Ok(out)
    }

    /// Every file and directory, depth first.
    pub fn list(&mut self) -> Result<Vec<Entry>> {
        let mut out = Vec::new();
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            for e in self.children(&dir)? {
                if e.dir {
                    stack.push(e.clone());
                }
                out.push(e);
            }
        }
        Ok(out)
    }

    /// A path such as `DATA/DATA.BIN`, compared without case.
    pub fn find(&mut self, path: &str) -> Result<Entry> {
        if let Source::Pack(p) = &self.source {
            if path.split(['/', '\\']).all(|s| s.is_empty()) {
                return Ok(self.root.clone());
            }
            return p.manifest.find(path).map(|e| e.entry.clone()).ok_or_else(|| Error::NotFound(path.to_string()));
        }
        let mut cur = self.root.clone();
        for part in path.split(['/', '\\']).filter(|s| !s.is_empty()) {
            let next = self
                .children(&cur)?
                .into_iter()
                .find(|e| e.path.rsplit('/').next().unwrap_or("").eq_ignore_ascii_case(part));
            cur = next.ok_or_else(|| Error::NotFound(path.to_string()))?;
        }
        Ok(cur)
    }

    pub fn read_path(&mut self, path: &str) -> Result<Vec<u8>> {
        if DENY_EXECUTABLE.load(std::sync::atomic::Ordering::Relaxed) && path.to_ascii_uppercase().starts_with("SLUS_")
        {
            return Err(Error::NotFound(format!("{path}: the port does not read the boot executable")));
        }
        // A disc image's port files come from its data folder.
        if let (Source::Image(_), Some(rest)) = (&self.source, port_path(path)) {
            let Some(dir) = &self.data else { return Err(Error::NotFound(path.to_string())) };
            return std::fs::read(dir.join(rest)).map_err(|_| Error::NotFound(path.to_string()));
        }
        let e = self.find(path)?;
        self.read(&e)
    }

    /// Where a disc image's port files are kept (None for a build's disc,
    /// whose `PINEY/` is its own).
    pub fn data_dir(&self) -> Option<&Path> {
        match self.source {
            Source::Image(_) => self.data.as_deref(),
            Source::Pack(_) => None,
        }
    }
}

/// The part of a path under `PINEY/`, if it is one of the port's files.
fn port_path(path: &str) -> Option<&str> {
    let (dir, rest) = path.split_once(['/', '\\'])?;
    dir.eq_ignore_ascii_case(crate::pack::PORT_DIR).then_some(rest)
}

/// A directory record; None for the `.` and `..` entries.
fn record(rec: &[u8], parent: &str) -> Result<Option<Entry>> {
    let lba = rec.u32_at(2)?;
    let size = rec.u32_at(10)?;
    let flags = rec.u8_at(25)?;
    let nlen = rec.u8_at(32)? as usize;
    let raw = rec.slice_at(33, nlen)?;
    if nlen == 1 && (raw[0] == 0 || raw[0] == 1) {
        return Ok(if parent.is_empty() && raw[0] == 0 {
            // The root's own record.
            Some(Entry { path: String::new(), lba, size, dir: true })
        } else {
            None
        });
    }
    let mut name: String = raw.iter().map(|&b| b as char).collect();
    if let Some(i) = name.find(';') {
        name.truncate(i);
    }
    let path = if parent.is_empty() { name } else { format!("{parent}/{name}") };
    Ok(Some(Entry { path, lba, size, dir: flags & 2 != 0 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `list` walks the disc once: the root's own record is not taken for
    /// a directory to enter (it was, and the walk never ended).
    #[test]
    fn list_walks_infection_once() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let Ok(mut iso) = Iso::open(&path) else {
            eprintln!("infection.iso not present; skipped");
            return;
        };
        let all = iso.list().unwrap();
        assert!(all.len() < 1000, "{} entries", all.len());
        assert!(all.iter().all(|e| !e.path.is_empty()));
        assert!(all.iter().any(|e| e.path == "DATA/DATA.BIN" && !e.dir));
        let mut paths: Vec<&str> = all.iter().map(|e| e.path.as_str()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), all.len());
    }
}
