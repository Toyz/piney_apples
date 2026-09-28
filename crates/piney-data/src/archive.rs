//! Sector-aligned gzip archives: `DATA/DATA.BIN` and `STREAM/*.BIN`.
//!
//! Neither has a directory of its own. Each is gzip members laid end to end,
//! every member starting on a 2048-byte boundary and named by its gzip FNAME
//! field (`xasc00.cmp`). The game finds members through an index compiled
//! into the executable (`docs/formats/data-bin.md`); walking the sectors
//! finds the same 1,023 members in Infection's archive without it, which
//! works the same on every volume.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Read;

use flate2::read::GzDecoder;

use crate::{Bytes, Error, Result};

pub const SECTOR: usize = 2048;

#[derive(Clone, Debug)]
pub struct Member {
    pub index: usize,
    pub offset: usize,
    /// Up to the next member, padding included.
    pub length: usize,
    /// The gzip FNAME, e.g. `xasc00.cmp`.
    pub name: String,
    pub mtime: u32,
}

impl Member {
    /// The name without its extension, lower case: `xasc00`.
    pub fn stem(&self) -> String {
        let n = self.name.to_ascii_lowercase();
        match n.rsplit_once('.') {
            Some((stem, _)) => stem.to_string(),
            None => n,
        }
    }
}

thread_local! {
    /// The members this thread inflated while a recording is on, by name,
    /// with their offset and length in the archive: what a scene's set-up
    /// read, for the game's `--dvd` estimate of the original's load time.
    static RECORDING: RefCell<Option<BTreeMap<String, (usize, usize)>>> = const { RefCell::new(None) };
}

/// Start noting the members this thread inflates, from none.
pub fn record() {
    RECORDING.with(|r| *r.borrow_mut() = Some(BTreeMap::new()));
}

/// The members inflated since [`record`], by name, with their offset and
/// length in the archive; the recording stops.
pub fn recorded() -> BTreeMap<String, (usize, usize)> {
    RECORDING.with(|r| r.borrow_mut().take()).unwrap_or_default()
}

pub struct Archive {
    data: Vec<u8>,
    members: Vec<Member>,
}

impl Archive {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let starts: Vec<usize> =
            (0..data.len()).step_by(SECTOR).filter(|&o| data.get(o..o + 3) == Some(&[0x1f, 0x8b, 0x08][..])).collect();
        let mut members = Vec::with_capacity(starts.len());
        for (i, &o) in starts.iter().enumerate() {
            let end = starts.get(i + 1).copied().unwrap_or(data.len());
            let flags = data.u8_at(o + 3)?;
            let mtime = data.u32_at(o + 4)?;
            let mut p = o + 10;
            if flags & 4 != 0 {
                p += 2 + data.u16_at(p)? as usize;
            }
            let mut name = String::new();
            if flags & 8 != 0 {
                let rest = data.get(p..end).unwrap_or(&[]);
                let nul = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
                name = rest[..nul].iter().map(|&b| b as char).collect();
            }
            members.push(Member { index: i, offset: o, length: end - o, name, mtime });
        }
        Ok(Archive { data, members })
    }

    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// By name, without case, with or without the extension.
    pub fn find(&self, name: &str) -> Option<&Member> {
        let want = name.to_ascii_lowercase();
        self.members.iter().find(|m| m.name.eq_ignore_ascii_case(&want) || m.stem() == want)
    }

    pub fn inflate(&self, m: &Member) -> Result<Vec<u8>> {
        RECORDING.with(|r| {
            if let Some(r) = r.borrow_mut().as_mut() {
                r.insert(m.name.clone(), (m.offset, m.length));
            }
        });
        let raw = self.data.slice_at(m.offset, m.length)?;
        let mut out = Vec::new();
        GzDecoder::new(raw).read_to_end(&mut out).map_err(|e| Error::Format(format!("{}: {e}", m.name)))?;
        Ok(out)
    }

    pub fn inflate_named(&self, name: &str) -> Result<Vec<u8>> {
        let m = self.find(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
        self.inflate(m)
    }
}
