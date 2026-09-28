//! Reading a stream's files off the disc as `ReadThread` does: each record's
//! gzip member at the header's offset plus its own (both rounded up to a
//! sector) in the archive the header names, `size` bytes of it, inflated to
//! `gzip` bytes (`docs/engine/stream.md`).
//!
//! Only the members a stream names are read, not the whole archive: `STR1.BIN`
//! is 354 MB.

use std::io::Read;

use flate2::read::GzDecoder;
use piney_data::archive::{Archive, SECTOR};
use piney_data::iso::Iso;
use piney_data::{Error, Result};

use crate::table::{Def, Entry};

/// One file of a stream, as read.
#[derive(Clone)]
pub struct Member {
    pub entry: Entry,
    /// Byte offset in the archive.
    pub offset: u64,
    /// The gzip member, `entry.size` bytes.
    pub raw: Vec<u8>,
    /// Its gzip FNAME (`str0001e.tmp`).
    pub fname: String,
}

impl Member {
    /// The stem the renderer's archive knows it by: the FNAME without its
    /// extension, lower case.
    pub fn stem(&self) -> String {
        let n = self.fname.to_ascii_lowercase();
        n.rsplit_once('.').map_or(n.clone(), |(s, _)| s.to_string())
    }

    /// The CCSF file.
    pub fn inflate(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.entry.gzip as usize);
        GzDecoder::new(&self.raw[..])
            .read_to_end(&mut out)
            .map_err(|e| Error::Format(format!("{}: {e}", self.fname)))?;
        if out.len() != self.entry.gzip as usize {
            return Err(Error::Format(format!(
                "{}: inflates to {} bytes, the table says {}",
                self.fname,
                out.len(),
                self.entry.gzip
            )));
        }
        Ok(out)
    }
}

/// The gzip FNAME every member of Outbreak's and Quarantine's archives
/// carries, whatever the file.
const GENERIC_FNAME: &str = "tmp.ccs";

/// The gzip FNAME of the member starting at `raw`.
fn fname(raw: &[u8]) -> Option<String> {
    if raw.len() < 10 || raw[..3] != [0x1f, 0x8b, 0x08] || raw[3] & 8 == 0 {
        return None;
    }
    let mut p = 10;
    if raw[3] & 4 != 0 {
        p += 2 + u16::from_le_bytes([raw[10], raw[11]]) as usize;
    }
    let rest = raw.get(p..)?;
    let nul = rest.iter().position(|&b| b == 0)?;
    Some(rest[..nul].iter().map(|&b| b as char).collect())
}

/// The gzip member at `raw` with its FNAME replaced by `name` (the CRC
/// covers the inflated data only); None without an FNAME, or with a header
/// CRC (FHCRC) the new name would break.
fn renamed(raw: &[u8], name: &str) -> Option<Vec<u8>> {
    if raw.len() < 10 || raw[..3] != [0x1f, 0x8b, 0x08] || raw[3] & 8 == 0 || raw[3] & 2 != 0 {
        return None;
    }
    let mut p = 10;
    if raw[3] & 4 != 0 {
        p += 2 + u16::from_le_bytes([*raw.get(10)?, *raw.get(11)?]) as usize;
    }
    let nul = p + raw.get(p..)?.iter().position(|&b| b == 0)?;
    let mut out = Vec::with_capacity(raw.len() + name.len());
    out.extend_from_slice(&raw[..p]);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&raw[nul..]);
    Some(out)
}

/// Read record `e` of `def` from the disc and check that the member there
/// is the one the record names.
pub fn read(iso: &mut Iso, def: &Def, e: &Entry) -> Result<Member> {
    let path = def.archive().ok_or_else(|| {
        Error::NotFound(format!("stream {}: archive {} is not on this disc", def.num, def.header.kind))
    })?;
    read_in(iso, path, def, e)
}

/// [`read`] from the archive at `path` on the disc, whatever the header's
/// `type` names.
pub fn read_in(iso: &mut Iso, path: &str, def: &Def, e: &Entry) -> Result<Member> {
    let arc = iso.find(path)?;
    let offset = def.offset(e);
    let raw = iso.read_at(&arc, offset, e.size.max(0) as usize)?;
    let name = fname(&raw).ok_or_else(|| Error::Format(format!("{path}+0x{offset:x}: no gzip member")))?;
    let want = format!("{}.tmp", e.name);
    // Outbreak's and Quarantine's archives name every member `tmp.ccs`; the
    // member then goes by the table's name, as the others' do.
    if name.eq_ignore_ascii_case(GENERIC_FNAME) {
        let raw = renamed(&raw, &want)
            .ok_or_else(|| Error::Format(format!("{path}+0x{offset:x}: cannot rename {name} to {want}")))?;
        return Ok(Member { entry: e.clone(), offset, raw, fname: want });
    }
    if !name.eq_ignore_ascii_case(&want) {
        return Err(Error::Format(format!("{path}+0x{offset:x}: {name}, the table says {want}")));
    }
    Ok(Member { entry: e.clone(), offset, raw, fname: name })
}

/// Not the game's: the records of an archive no stream table lists
/// (Outbreak's `STREAM/STRT.BIN`), read off the archive itself. A member
/// starts on a sector with a gzip header; its record takes the CCSF file's
/// own name (after `CCSF` at +0x0c), its bytes to the next member (the
/// padding after the gzip trailer is not read), and its inflated length.
/// Every record is a last scene (`type` -1) until [`loose_streams`] groups
/// them.
pub fn scan(raw: &[u8]) -> Result<Vec<Entry>> {
    let starts: Vec<usize> =
        (0..raw.len()).step_by(SECTOR).filter(|&o| raw.get(o..o + 3) == Some(&[0x1f, 0x8b, 0x08][..])).collect();
    let mut out = Vec::new();
    for (k, &o) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(raw.len());
        let mut ccsf = Vec::new();
        GzDecoder::new(&raw[o..end])
            .read_to_end(&mut ccsf)
            .map_err(|e| Error::Format(format!("the member at 0x{o:x}: {e}")))?;
        let name = ccsf.get(12..44).and_then(|n| {
            let n = &n[..n.iter().position(|&b| b == 0).unwrap_or(n.len())];
            (ccsf.get(8..12) == Some(b"CCSF") && !n.is_empty()).then(|| String::from_utf8_lossy(n).into_owned())
        });
        let name = name.ok_or_else(|| Error::Format(format!("the member at 0x{o:x} is no CCSF file")))?;
        out.push(Entry { name, ofs: o as i32, size: (end - o) as i32, kind: -1, flag: 0, gzip: ccsf.len() as u32 });
    }
    Ok(out)
}

/// The streams a table-less archive's records make ([`scan`]), in order: a
/// setup file (a name ending in `e`: `type` 0, `flag` 16, read whole first,
/// as the tables' `strNNNNe` records are) with the scene after it whose
/// name is that stem, or a record alone as its stream's last scene.
pub fn loose_streams(entries: Vec<Entry>) -> Vec<Vec<Entry>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < entries.len() {
        let e = &entries[i];
        let pairs = e.name.strip_suffix('e').is_some_and(|stem| entries.get(i + 1).is_some_and(|n| n.name == stem));
        if pairs {
            out.push(vec![Entry { kind: 0, flag: crate::table::FLAG_PRELOAD, ..e.clone() }, entries[i + 1].clone()]);
            i += 2;
        } else {
            out.push(vec![e.clone()]);
            i += 1;
        }
    }
    out
}

/// An archive of `members`, each padded to a sector, which
/// `piney_data::archive::Archive` (and so `piney_desktop::assets::SceneFile`
/// and the renderers) reads by stem.
pub fn archive(members: &[&Member]) -> Result<Archive> {
    let mut data = Vec::new();
    let mut seen = Vec::new();
    for m in members {
        if seen.contains(&m.fname) {
            continue;
        }
        seen.push(m.fname.clone());
        data.extend_from_slice(&m.raw);
        data.resize(data.len().div_ceil(SECTOR) * SECTOR, 0);
    }
    Archive::new(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Outbreak's `STREAM/STRT.BIN`, which no stream table lists: six
    /// members, five streams (`str9999e` sets `str9999` up).
    #[test]
    fn outbreak_strt_scans_into_five_streams() {
        let iso = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../work/outbreak/outbreak.iso");
        let Ok(mut disc) = Iso::open(&iso) else { return };
        let raw = disc.read_path("STREAM/STRT.BIN").unwrap();
        let entries = scan(&raw).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["title1_st1t", "title1_st1t2", "title2_st1", "str9999e", "str9999", "trial_v2st"]);
        assert_eq!(entries[4].gzip, 7_703_772);
        let streams = loose_streams(entries);
        assert_eq!(streams.len(), 5);
        assert_eq!((streams[3][0].kind, streams[3][0].flag, streams[3][1].kind), (0, crate::table::FLAG_PRELOAD, -1));
    }

    #[test]
    fn fname_of_a_member() {
        let mut raw = vec![0x1f, 0x8b, 0x08, 0x08, 0, 0, 0, 0, 0, 0];
        raw.extend_from_slice(b"str6100.tmp\0rest");
        assert_eq!(fname(&raw).as_deref(), Some("str6100.tmp"));
        raw[3] = 0;
        assert_eq!(fname(&raw), None);
    }
}
