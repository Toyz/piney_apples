//! A build of the discs: what `piney-build` makes from one to four disc
//! images, and what [`crate::iso::Iso`] reads in their place (the README's
//! Playing section). A build directory holds `chunks.pak` ("PINEYPAK", u32
//! version, u32 0, then each chunk once however many files hold it, zstd
//! where that is smaller) and one `.disc` per volume (`Manifest`: the disc's
//! files as runs of those chunks). Each `.disc` also holds the port's files
//! under [`PORT_DIR`] (at LBA 0xffffffff), made so that play never reads the
//! executable; a disc image keeps them in [`image_data_dir`].

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::iso::Entry;
use crate::volume::Volume;
use crate::{Error, Result, format_err};

pub const PAK_NAME: &str = "chunks.pak";
pub const PAK_MAGIC: &[u8; 8] = b"PINEYPAK";
pub const DISC_MAGIC: &[u8; 8] = b"PINEYDSC";
pub const VERSION: u32 = 2;
/// `chunks.pak`'s header: magic, version, a reserved word.
pub const PAK_HEADER: u64 = 16;

/// A volume's `.disc` file name in a build.
pub fn disc_name(v: Volume) -> &'static str {
    match v {
        Volume::Inf => "infection.disc",
        Volume::Mut => "mutation.disc",
        Volume::Out => "outbreak.disc",
        Volume::Qua => "quarantine.disc",
    }
}

/// Where a run of a file's bytes is kept in `chunks.pak`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub offset: u64,
    /// Bytes in `chunks.pak`.
    pub stored: u32,
    /// Bytes of the file.
    pub len: u32,
    /// Kept zstd-compressed.
    pub zstd: bool,
}

/// How a chunk is kept.
pub const CODEC_RAW: u8 = 0;
pub const CODEC_ZSTD: u8 = 1;

/// A file or directory of a disc, as a build keeps it.
#[derive(Clone, Debug)]
pub struct PackEntry {
    pub entry: Entry,
    pub hash: [u8; 32],
    pub chunks: Vec<Chunk>,
    /// Where each chunk starts in the file.
    starts: Vec<u64>,
}

impl PackEntry {
    pub fn new(entry: Entry, hash: [u8; 32], chunks: Vec<Chunk>) -> PackEntry {
        let mut starts = Vec::with_capacity(chunks.len());
        let mut at = 0u64;
        for c in &chunks {
            starts.push(at);
            at += u64::from(c.len);
        }
        PackEntry { entry, hash, chunks, starts }
    }

    /// The file's length as its chunks add up.
    pub fn stored_len(&self) -> u64 {
        self.chunks.iter().map(|c| u64::from(c.len)).sum()
    }
}

/// One disc of a build: its `.disc` file, all little-endian: "PINEYDSC", u32
/// version, u32 volume (0-3), u32 entries, u32 chunk references, the image's
/// volume identifier (32 bytes, NUL-padded), the entries (path, LBA, size,
/// directory flag, first chunk reference and count, BLAKE3 of the bytes), then
/// the chunk references (offset in `chunks.pak`, bytes there, bytes of the
/// file, codec). A file's bytes are its chunks', each decompressed, in order.
#[derive(Debug)]
pub struct Manifest {
    pub volume: Volume,
    /// The image's volume identifier (`HACK_VOL1` and the like).
    pub label: String,
    pub entries: Vec<PackEntry>,
    /// Upper-cased path -> entry.
    index: HashMap<String, usize>,
}

impl Manifest {
    pub fn new(volume: Volume, label: String, entries: Vec<PackEntry>) -> Manifest {
        let index = entries.iter().enumerate().map(|(i, e)| (e.entry.path.to_ascii_uppercase(), i)).collect();
        Manifest { volume, label, entries, index }
    }

    /// The entry at `path`, compared without case (`\` or `/`).
    pub fn find(&self, path: &str) -> Option<&PackEntry> {
        let key: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
        self.index.get(&key.join("/").to_ascii_uppercase()).map(|&i| &self.entries[i])
    }

    pub fn encode(&self) -> Vec<u8> {
        let refs: usize = self.entries.iter().map(|e| e.chunks.len()).sum();
        let mut out = Vec::new();
        out.extend_from_slice(DISC_MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.volume as u32).to_le_bytes());
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        out.extend_from_slice(&(refs as u32).to_le_bytes());
        let mut label = [0u8; 32];
        let n = self.label.len().min(32);
        label[..n].copy_from_slice(&self.label.as_bytes()[..n]);
        out.extend_from_slice(&label);
        let mut first = 0u32;
        for e in &self.entries {
            let path = e.entry.path.as_bytes();
            out.extend_from_slice(&(path.len() as u16).to_le_bytes());
            out.extend_from_slice(path);
            out.extend_from_slice(&e.entry.lba.to_le_bytes());
            out.extend_from_slice(&e.entry.size.to_le_bytes());
            out.push(u8::from(e.entry.dir));
            out.extend_from_slice(&first.to_le_bytes());
            out.extend_from_slice(&(e.chunks.len() as u32).to_le_bytes());
            out.extend_from_slice(&e.hash);
            first += e.chunks.len() as u32;
        }
        for e in &self.entries {
            for c in &e.chunks {
                out.extend_from_slice(&c.offset.to_le_bytes());
                out.extend_from_slice(&c.stored.to_le_bytes());
                out.extend_from_slice(&c.len.to_le_bytes());
                out.push(if c.zstd { CODEC_ZSTD } else { CODEC_RAW });
            }
        }
        out
    }

    pub fn decode(b: &[u8]) -> Result<Manifest> {
        let mut r = Reader { b, at: 0 };
        if r.take(8)? != DISC_MAGIC {
            return format_err("not a .disc file");
        }
        let version = r.u32()?;
        if version != VERSION {
            return format_err(format!(".disc version {version}; this build of the port reads {VERSION}"));
        }
        let volume =
            Volume::ALL.get(r.u32()? as usize).copied().ok_or_else(|| Error::Format(".disc: no such volume".into()))?;
        let count = r.u32()? as usize;
        let nrefs = r.u32()? as usize;
        let label = r.take(32)?;
        let label = String::from_utf8_lossy(&label[..label.iter().position(|&c| c == 0).unwrap_or(32)]).into_owned();
        let mut heads = Vec::with_capacity(count);
        for _ in 0..count {
            let n = usize::from(r.u16()?);
            let path = String::from_utf8(r.take(n)?.to_vec())
                .map_err(|_| Error::Format(".disc: a path is not UTF-8".into()))?;
            let lba = r.u32()?;
            let size = r.u32()?;
            let dir = r.take(1)?[0] != 0;
            let first = r.u32()? as usize;
            let chunks = r.u32()? as usize;
            let hash: [u8; 32] = r.take(32)?.try_into().unwrap();
            heads.push((Entry { path, lba, size, dir }, first, chunks, hash));
        }
        let mut refs = Vec::with_capacity(nrefs);
        for _ in 0..nrefs {
            let offset = r.u64()?;
            let stored = r.u32()?;
            let len = r.u32()?;
            let zstd = match r.take(1)?[0] {
                CODEC_RAW => false,
                CODEC_ZSTD => true,
                c => return format_err(format!(".disc: unknown chunk codec {c}")),
            };
            refs.push(Chunk { offset, stored, len, zstd });
        }
        let mut entries = Vec::with_capacity(count);
        for (entry, first, n, hash) in heads {
            let chunks = refs
                .get(first..first + n)
                .ok_or_else(|| Error::Format(".disc: a file's chunks run past the table".into()))?;
            let e = PackEntry::new(entry, hash, chunks.to_vec());
            if !e.entry.dir && e.stored_len() != u64::from(e.entry.size) {
                return format_err(format!(
                    ".disc: {} has {} bytes in its chunks, not {}",
                    e.entry.path,
                    e.stored_len(),
                    e.entry.size
                ));
            }
            entries.push(e);
        }
        Ok(Manifest::new(volume, label, entries))
    }
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.at..self.at + n).ok_or_else(|| Error::Format(".disc: cut short".into()))?;
        self.at += n;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

/// Whether the file at `path` starts as a `.disc` file does.
pub fn is_disc(path: &Path) -> bool {
    let mut magic = [0u8; 8];
    File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && &magic == DISC_MAGIC
}

/// A manifest read once per file: the port opens its disc many times. A
/// file written again (a new build) is read again.
pub fn manifest(path: &Path) -> Result<Arc<Manifest>> {
    type Key = (PathBuf, u64, Option<std::time::SystemTime>);
    static CACHE: OnceLock<Mutex<HashMap<Key, Arc<Manifest>>>> = OnceLock::new();
    let meta = std::fs::metadata(path)?;
    let key = (std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()), meta.len(), meta.modified().ok());
    let cache = CACHE.get_or_init(Default::default);
    if let Some(m) = cache.lock().unwrap().get(&key) {
        return Ok(m.clone());
    }
    let m = Arc::new(Manifest::decode(&std::fs::read(path)?)?);
    cache.lock().unwrap().insert(key, m.clone());
    Ok(m)
}

/// A disc of a build, open for reading: its manifest and `chunks.pak`.
pub struct PackDisc {
    pub manifest: Arc<Manifest>,
    pak: File,
    /// The last chunk read, as the file has it (its offset in `chunks.pak`):
    /// reads in pieces smaller than a chunk decompress it once.
    last: Option<(u64, Vec<u8>)>,
}

impl PackDisc {
    /// `path` is a `.disc` file; `chunks.pak` is beside it.
    pub fn open(path: &Path) -> Result<PackDisc> {
        let manifest = manifest(path)?;
        let pak_path = path.parent().unwrap_or(Path::new(".")).join(PAK_NAME);
        let mut pak = File::open(&pak_path).map_err(|e| Error::NotFound(format!("{}: {e}", pak_path.display())))?;
        let mut head = [0u8; PAK_HEADER as usize];
        pak.read_exact(&mut head)?;
        if &head[..8] != PAK_MAGIC {
            return format_err(format!("{}: not a chunks.pak", pak_path.display()));
        }
        let version = u32::from_le_bytes(head[8..12].try_into().unwrap());
        if version != VERSION {
            return format_err(format!(
                "{}: version {version}; this build of the port reads {VERSION}",
                pak_path.display()
            ));
        }
        Ok(PackDisc { manifest, pak, last: None })
    }

    /// A chunk's bytes as the file has them.
    fn chunk(&mut self, c: Chunk) -> Result<&[u8]> {
        if self.last.as_ref().is_none_or(|(at, _)| *at != c.offset) {
            let mut stored = vec![0u8; c.stored as usize];
            self.pak.seek(SeekFrom::Start(c.offset))?;
            self.pak.read_exact(&mut stored)?;
            let bytes = if c.zstd {
                zstd::bulk::decompress(&stored, c.len as usize)
                    .map_err(|e| Error::Format(format!("chunks.pak+0x{:x}: {e}", c.offset)))?
            } else {
                stored
            };
            if bytes.len() != c.len as usize {
                return format_err(format!("chunks.pak+0x{:x}: {} bytes, not {}", c.offset, bytes.len(), c.len));
            }
            self.last = Some((c.offset, bytes));
        }
        Ok(&self.last.as_ref().unwrap().1)
    }

    /// `len` bytes of `path`'s file from `ofs` on, fewer where it ends.
    pub fn read_at(&mut self, path: &str, ofs: u64, len: usize) -> Result<Vec<u8>> {
        let manifest = self.manifest.clone();
        let e = manifest.find(path).ok_or_else(|| Error::NotFound(path.to_string()))?;
        let len = len.min(u64::from(e.entry.size).saturating_sub(ofs) as usize);
        let mut out = Vec::with_capacity(len);
        if len == 0 {
            return Ok(out);
        }
        let end = ofs + len as u64;
        // The chunk holding `ofs`, then on until `end`.
        let mut k = e.starts.partition_point(|&s| s <= ofs).saturating_sub(1);
        while (out.len() as u64) < len as u64 && k < e.chunks.len() {
            let c = e.chunks[k];
            let start = e.starts[k];
            let from = (ofs.max(start) - start) as usize;
            let to = (end.min(start + u64::from(c.len)) - start) as usize;
            out.extend_from_slice(&self.chunk(c)?[from..to]);
            k += 1;
        }
        Ok(out)
    }
}

/// Where the port keeps its things: `$PINEY_HOME`, else `piney` in the
/// platform's data folder (`~/.local/share/piney` on Linux, `%APPDATA%\piney`
/// on Windows, `~/Library/Application Support/piney` on macOS).
pub fn home() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("PINEY_HOME") {
        return Some(PathBuf::from(p));
    }
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    } else if let Some(x) = std::env::var_os("XDG_DATA_HOME").filter(|x| !x.is_empty()) {
        PathBuf::from(x)
    } else {
        PathBuf::from(std::env::var_os("HOME")?).join(".local/share")
    };
    Some(base.join("piney"))
}

/// The build `piney-build` makes when not told where: `game` in [`home`].
pub fn default_build() -> Option<PathBuf> {
    home().map(|h| h.join("game"))
}

/// The port's own files in a disc (`plans/build-data.md`): what the port
/// takes from the disc, made once by the build (or, for a disc image, into
/// [`image_data_dir`]), under this directory.
pub const PORT_DIR: &str = "PINEY";
/// Which version of the port's data a disc's `PINEY/` holds: raised
/// whenever what a producer writes changes, so older data is made again.
pub const DATA_VERSION: u32 = 10;
/// The file holding [`DATA_VERSION`], as text.
pub const VERSION_FILE: &str = "PINEY/VERSION";

/// Where a disc image's port data lives (it has no `PINEY/` of its own):
/// `data/<image name>-<its size in hex>` in the port's folder, the files
/// named as they are under `PINEY/`.
pub fn image_data_dir(image: &Path) -> Option<PathBuf> {
    let len = std::fs::metadata(image).ok()?.len();
    let stem = image.file_stem()?.to_string_lossy().into_owned();
    home().map(|h| h.join("data").join(format!("{stem}-{len:x}")))
}

/// Whether `iso`'s port data is there and of this version.
pub fn has_port_data(iso: &mut crate::iso::Iso) -> bool {
    iso.read_path(VERSION_FILE)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .is_some_and(|t| t.trim().parse::<u32>().ok() == Some(DATA_VERSION))
}

/// The volumes a build directory holds, with their `.disc` paths.
pub fn volumes_in(dir: &Path) -> Vec<(Volume, PathBuf)> {
    Volume::ALL.iter().map(|&v| (v, dir.join(disc_name(v)))).filter(|(_, p)| p.is_file()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u32, dir: bool) -> Entry {
        Entry { path: path.into(), lba: 100, size, dir }
    }

    #[test]
    fn manifest_round_trips() {
        let m = Manifest::new(
            Volume::Out,
            "HACK_VOL3".into(),
            vec![
                PackEntry::new(entry("DATA", 2048, true), [0; 32], vec![]),
                PackEntry::new(
                    entry("DATA/DATA.BIN", 30, false),
                    [7; 32],
                    vec![
                        Chunk { offset: 16, stored: 10, len: 10, zstd: false },
                        Chunk { offset: 90, stored: 7, len: 20, zstd: true },
                    ],
                ),
            ],
        );
        let back = Manifest::decode(&m.encode()).unwrap();
        assert_eq!(back.volume, Volume::Out);
        assert_eq!(back.label, "HACK_VOL3");
        let e = back.find("data\\data.bin").unwrap();
        assert_eq!(
            e.chunks,
            vec![
                Chunk { offset: 16, stored: 10, len: 10, zstd: false },
                Chunk { offset: 90, stored: 7, len: 20, zstd: true }
            ]
        );
        assert_eq!(e.hash, [7; 32]);
        assert!(back.find("DATA").unwrap().entry.dir);
    }

    #[test]
    fn reads_across_chunks() {
        // Beside the test binary, in the build's target folder.
        let dir = std::env::current_exe().unwrap().with_file_name(format!("piney-pack-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        // chunks.pak: header, "0123456789" as it is, "abcdefghij" in zstd.
        let z = zstd::bulk::compress(b"abcdefghij", 3).unwrap();
        let mut pak = Vec::from(&PAK_MAGIC[..]);
        pak.extend_from_slice(&VERSION.to_le_bytes());
        pak.extend_from_slice(&0u32.to_le_bytes());
        pak.extend_from_slice(b"0123456789");
        pak.extend_from_slice(&z);
        std::fs::write(dir.join(PAK_NAME), &pak).unwrap();
        // The file is "abcdefghij" then "0123456789".
        let m = Manifest::new(
            Volume::Inf,
            "T".into(),
            vec![PackEntry::new(
                entry("F.BIN", 20, false),
                [0; 32],
                vec![
                    Chunk { offset: 26, stored: z.len() as u32, len: 10, zstd: true },
                    Chunk { offset: 16, stored: 10, len: 10, zstd: false },
                ],
            )],
        );
        let disc = dir.join(disc_name(Volume::Inf));
        std::fs::write(&disc, m.encode()).unwrap();
        let mut p = PackDisc::open(&disc).unwrap();
        assert_eq!(p.read_at("F.BIN", 0, 100).unwrap(), b"abcdefghij0123456789");
        assert_eq!(p.read_at("f.bin", 8, 4).unwrap(), b"ij01");
        assert_eq!(p.read_at("F.BIN", 15, 100).unwrap(), b"56789");
        assert!(p.read_at("F.BIN", 20, 5).unwrap().is_empty());
        assert_eq!(volumes_in(&dir), vec![(Volume::Inf, disc)]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
