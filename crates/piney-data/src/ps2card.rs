//! A PS2 memory card image, as PCSX2 keeps one (`Mcd001.ps2`, 8 MB): the
//! card's own file system, read so that a game's save directory can be
//! taken off it. Little-endian throughout.
//!
//! ```text
//! page          page_len bytes of data (512), then 16 of ECC when the
//!               image carries it (528 a page on the disc of an 8 MB card)
//! cluster       pages_per_cluster pages (2): 1024 bytes of data
//! superblock    cluster 0: "Sony PS2 Memory Card Format ", page_len +0x28,
//!               pages_per_cluster +0x2a, alloc_offset +0x34,
//!               rootdir_cluster +0x3c, ifc_list[32] +0x50
//! FAT           ifc_list: indirect clusters of FAT cluster numbers; the
//!               FAT's entries, by cluster relative to alloc_offset: bit 31
//!               in use, the low 31 bits the next cluster, 0x7fffffff the end
//! dir entry     512 bytes: mode +0x00 (0x8000 exists, 0x20 a directory,
//!               0x10 a file), length +0x04 (a directory's entries or a
//!               file's bytes), cluster +0x10 (relative), name +0x40 (32)
//! ```

use crate::{Error, Result};

const MAGIC: &[u8] = b"Sony PS2 Memory Card Format ";
const ENTRY: usize = 512;
const EXISTS: u16 = 0x8000;
const DIRECTORY: u16 = 0x20;
const FILE: u16 = 0x10;
const CHAIN_END: u32 = 0x7fff_ffff;

/// An entry of a directory on the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub mode: u16,
    /// A directory's entries ("." and ".." included), or a file's bytes.
    pub length: u32,
    /// The first cluster, relative to the allocatable area.
    pub cluster: u32,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.mode & DIRECTORY != 0
    }
    pub fn is_file(&self) -> bool {
        self.mode & FILE != 0
    }
    fn parse(raw: &[u8]) -> Entry {
        let name = &raw[0x40..0x60];
        let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
        Entry {
            name: String::from_utf8_lossy(name).into_owned(),
            mode: u16::from_le_bytes([raw[0], raw[1]]),
            length: u32_at(raw, 4),
            cluster: u32_at(raw, 0x10),
        }
    }
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// A card image, opened.
pub struct Card {
    data: Vec<u8>,
    raw_page: usize,
    page_len: usize,
    pages_per_cluster: usize,
    alloc_offset: u32,
    root: u32,
    fat: Vec<u32>,
}

impl Card {
    /// The image's superblock and FAT read.
    pub fn open(data: Vec<u8>) -> Result<Card> {
        if data.len() < 0x154 || !data.starts_with(MAGIC) {
            return Err(Error::Format("not a PS2 memory card image (no superblock)".into()));
        }
        let page_len = usize::from(u16::from_le_bytes([data[0x28], data[0x29]]));
        let pages_per_cluster = usize::from(u16::from_le_bytes([data[0x2a], data[0x2b]]));
        if page_len == 0 || pages_per_cluster == 0 {
            return Err(Error::Format("the superblock's page sizes are 0".into()));
        }
        // 16 bytes of ECC a 512-byte page: an 8 MB card is 16384 pages of
        // 528 (8,650,752 bytes) with it, of 512 without.
        let with_ecc = page_len + page_len / 32;
        let raw_page = if data.len().is_multiple_of(with_ecc) && (data.len() / with_ecc).is_power_of_two() {
            with_ecc
        } else {
            page_len
        };
        let mut card = Card {
            alloc_offset: u32_at(&data, 0x34),
            root: u32_at(&data, 0x3c),
            data,
            raw_page,
            page_len,
            pages_per_cluster,
            fat: Vec::new(),
        };
        card.fat = card.read_fat()?;
        Ok(card)
    }

    fn cluster_len(&self) -> usize {
        self.page_len * self.pages_per_cluster
    }

    /// Cluster `c` (absolute): its pages' data, without the ECC.
    fn cluster(&self, c: u32) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.cluster_len());
        for p in 0..self.pages_per_cluster {
            let at = (c as usize * self.pages_per_cluster + p) * self.raw_page;
            let page = self
                .data
                .get(at..at + self.page_len)
                .ok_or_else(|| Error::Format(format!("cluster {c} past the image's end")))?;
            out.extend_from_slice(page);
        }
        Ok(out)
    }

    fn read_fat(&self) -> Result<Vec<u32>> {
        let mut fat = Vec::new();
        for k in 0..32 {
            let ifc = u32_at(&self.data, 0x50 + 4 * k);
            if ifc == 0 || ifc == u32::MAX {
                break;
            }
            let ind = self.cluster(ifc)?;
            for e in ind.as_chunks::<4>().0 {
                let fc = u32::from_le_bytes(*e);
                if fc == 0 || fc == u32::MAX {
                    break;
                }
                let words = self.cluster(fc)?;
                fat.extend(words.as_chunks::<4>().0.iter().map(|w| u32::from_le_bytes(*w)));
            }
        }
        if fat.is_empty() {
            return Err(Error::Format("the card has no FAT".into()));
        }
        Ok(fat)
    }

    /// `len` bytes from the chain that starts at relative cluster `first`.
    fn read_chain(&self, first: u32, len: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        let mut c = first;
        while out.len() < len {
            out.extend(self.cluster(self.alloc_offset + c)?);
            let next = *self.fat.get(c as usize).ok_or_else(|| Error::Format(format!("cluster {c} past the FAT")))?;
            if next & 0x8000_0000 == 0 {
                return Err(Error::Format(format!("cluster {c} is free in the FAT")));
            }
            c = next & 0x7fff_ffff;
            if c == CHAIN_END {
                break;
            }
        }
        if out.len() < len {
            return Err(Error::Format(format!("a chain ends {} bytes short", len - out.len())));
        }
        out.truncate(len);
        Ok(out)
    }

    /// The entries of the directory whose first cluster is `cluster`, "."
    /// and ".." and the deleted ones left out.
    fn dir_at(&self, cluster: u32, count: u32) -> Result<Vec<Entry>> {
        let raw = self.read_chain(cluster, count as usize * ENTRY)?;
        Ok(raw
            .as_chunks::<ENTRY>()
            .0
            .iter()
            .map(|e| Entry::parse(e))
            .filter(|e| e.mode & EXISTS != 0 && e.name != "." && e.name != "..")
            .collect())
    }

    /// The root directory: its own "." entry gives its count.
    pub fn root(&self) -> Result<Vec<Entry>> {
        let dot = Entry::parse(&self.read_chain(self.root, ENTRY)?);
        self.dir_at(self.root, dot.length)
    }

    /// A directory's entries.
    pub fn dir(&self, e: &Entry) -> Result<Vec<Entry>> {
        if !e.is_dir() {
            return Err(Error::Format(format!("{} is not a directory", e.name)));
        }
        self.dir_at(e.cluster, e.length)
    }

    /// A file's bytes.
    pub fn file(&self, e: &Entry) -> Result<Vec<u8>> {
        if !e.is_file() {
            return Err(Error::Format(format!("{} is not a file", e.name)));
        }
        if e.length == 0 {
            return Ok(Vec::new());
        }
        self.read_chain(e.cluster, e.length as usize)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small card written the way the format says (page_len 512, two
    /// pages a cluster, ECC bytes zero): one directory of `files`.
    pub(crate) fn build(dir: &str, files: &[(&str, Vec<u8>)], ecc: bool) -> Vec<u8> {
        const CL: usize = 1024;
        let raw_page = if ecc { 528 } else { 512 };
        let (ifc, fat_c, alloc) = (8u32, 9u32, 10u32);
        let clusters = 256usize;
        let mut cl = vec![vec![0u8; CL]; clusters];
        let mut fat = vec![0u32; CL / 4];
        let mut next = 0u32;
        let mut put = |bytes: &[u8], cl: &mut Vec<Vec<u8>>, fat: &mut Vec<u32>| -> u32 {
            let n = bytes.len().div_ceil(CL).max(1) as u32;
            let first = next;
            for i in 0..n {
                let c = first + i;
                let chunk = &bytes[(i as usize * CL).min(bytes.len())..((i as usize + 1) * CL).min(bytes.len())];
                cl[(alloc + c) as usize][..chunk.len()].copy_from_slice(chunk);
                fat[c as usize] = 0x8000_0000 | if i + 1 == n { CHAIN_END } else { c + 1 };
            }
            next += n;
            first
        };
        let entry = |name: &str, mode: u16, length: u32, cluster: u32| {
            let mut e = vec![0u8; ENTRY];
            e[0..2].copy_from_slice(&mode.to_le_bytes());
            e[4..8].copy_from_slice(&length.to_le_bytes());
            e[0x10..0x14].copy_from_slice(&cluster.to_le_bytes());
            e[0x40..0x40 + name.len()].copy_from_slice(name.as_bytes());
            e
        };
        // The files first, then the directory, then the root.
        let mut file_entries = Vec::new();
        for (name, bytes) in files {
            let c = if bytes.is_empty() { 0 } else { put(bytes, &mut cl, &mut fat) };
            file_entries.push(entry(name, EXISTS | FILE | 7, bytes.len() as u32, c));
        }
        let dir_n = file_entries.len() as u32 + 2;
        let mut dir_bytes = entry(".", EXISTS | DIRECTORY | 7, dir_n, 0);
        dir_bytes.extend(entry("..", EXISTS | DIRECTORY | 7, 0, 0));
        for e in &file_entries {
            dir_bytes.extend_from_slice(e);
        }
        let dir_c = put(&dir_bytes, &mut cl, &mut fat);
        // The directory's "." points at itself.
        cl[(alloc + dir_c) as usize][0x10..0x14].copy_from_slice(&dir_c.to_le_bytes());
        let mut root = entry(".", EXISTS | DIRECTORY | 7, 3, 0);
        root.extend(entry("..", EXISTS | DIRECTORY | 7, 0, 0));
        root.extend(entry(dir, EXISTS | DIRECTORY | 7, dir_n, dir_c));
        let root_c = put(&root, &mut cl, &mut fat);
        for (k, w) in fat.iter().enumerate() {
            cl[fat_c as usize][4 * k..4 * k + 4].copy_from_slice(&w.to_le_bytes());
        }
        cl[ifc as usize][0..4].copy_from_slice(&fat_c.to_le_bytes());
        let sb = &mut cl[0];
        sb[..MAGIC.len()].copy_from_slice(MAGIC);
        sb[0x28..0x2a].copy_from_slice(&512u16.to_le_bytes());
        sb[0x2a..0x2c].copy_from_slice(&2u16.to_le_bytes());
        sb[0x34..0x38].copy_from_slice(&alloc.to_le_bytes());
        sb[0x3c..0x40].copy_from_slice(&root_c.to_le_bytes());
        sb[0x50..0x54].copy_from_slice(&ifc.to_le_bytes());
        let mut out = Vec::with_capacity(clusters * 2 * raw_page);
        for c in &cl {
            for p in c.as_chunks::<512>().0 {
                out.extend_from_slice(p);
                out.resize(out.len() + raw_page - 512, 0);
            }
        }
        out
    }

    /// A save directory read back off a card, with and without ECC: its
    /// files' names and bytes, one of them over two clusters and one empty.
    #[test]
    fn a_save_directory_reads_back() {
        let big: Vec<u8> = (0..0x8530u32).map(|i| (i * 7) as u8).collect();
        let files = [("icon.sys", vec![1u8; 964]), ("dhdata01", big.clone()), ("dhdata02", Vec::new())];
        for ecc in [false, true] {
            let card = Card::open(build("BASLUS-20267DOTHACK", &files, ecc)).unwrap();
            let root = card.root().unwrap();
            assert_eq!(root.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["BASLUS-20267DOTHACK"]);
            let entries = card.dir(&root[0]).unwrap();
            assert_eq!(entries.len(), 3);
            for (e, (name, bytes)) in entries.iter().zip(&files) {
                assert_eq!(&e.name, name);
                assert_eq!(&card.file(e).unwrap(), bytes, "{name} (ecc {ecc})");
            }
        }
    }

    #[test]
    fn not_a_card() {
        assert!(Card::open(vec![0; 4096]).is_err());
    }
}
