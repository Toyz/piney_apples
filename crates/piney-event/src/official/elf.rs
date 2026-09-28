//! Just enough of an EE ELF to read the loaded image by virtual address.

use crate::official::Error;

#[derive(Clone, Copy, Debug)]
struct Segment {
    vaddr: u32,
    offset: u32,
    filesz: u32,
    memsz: u32,
    exec: bool,
}

/// The boot executable's loadable segments.
pub struct Executable {
    data: Vec<u8>,
    segs: Vec<Segment>,
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

impl Executable {
    pub fn parse(data: Vec<u8>) -> Result<Executable, Error> {
        if data.get(0..4) != Some(b"\x7fELF") || data.get(4) != Some(&1) || data.get(5) != Some(&1) {
            return Err(Error::Format("not a 32-bit little-endian ELF".into()));
        }
        let bad = || Error::Format("truncated ELF header".into());
        let phoff = u32_at(&data, 0x1c).ok_or_else(bad)? as usize;
        let phentsize = u16_at(&data, 0x2a).ok_or_else(bad)? as usize;
        let phnum = u16_at(&data, 0x2c).ok_or_else(bad)? as usize;
        let mut segs = Vec::new();
        for i in 0..phnum {
            let p = phoff + i * phentsize;
            let ty = u32_at(&data, p).ok_or_else(bad)?;
            if ty != 1 {
                continue;
            }
            let seg = Segment {
                offset: u32_at(&data, p + 4).ok_or_else(bad)?,
                vaddr: u32_at(&data, p + 8).ok_or_else(bad)?,
                filesz: u32_at(&data, p + 16).ok_or_else(bad)?,
                memsz: u32_at(&data, p + 20).ok_or_else(bad)?,
                exec: u32_at(&data, p + 24).ok_or_else(bad)? & 1 != 0,
            };
            if seg.offset as usize + seg.filesz as usize > data.len() {
                return Err(Error::Format("segment runs past the end of the file".into()));
            }
            segs.push(seg);
        }
        if segs.is_empty() {
            return Err(Error::Format("no loadable segment".into()));
        }
        Ok(Executable { data, segs })
    }

    /// `len` bytes at `va`; bytes in a segment's memory but past its file
    /// data (BSS) read as zero. None when any byte is unmapped.
    pub fn read(&self, va: u32, len: usize) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        let mut at = va;
        while out.len() < len {
            let s = self.segs.iter().find(|s| s.vaddr <= at && at < s.vaddr.wrapping_add(s.memsz))?;
            let rel = at - s.vaddr;
            let take = ((s.memsz - rel) as usize).min(len - out.len());
            for k in 0..take as u32 {
                let r = rel + k;
                out.push(if r < s.filesz { self.data[(s.offset + r) as usize] } else { 0 });
            }
            at = at.wrapping_add(take as u32);
        }
        Some(out)
    }

    pub fn mapped(&self, va: u32) -> bool {
        self.segs.iter().any(|s| s.vaddr <= va && va < s.vaddr.wrapping_add(s.memsz))
    }

    pub fn u32(&self, va: u32) -> Option<u32> {
        let b = self.read(va, 4)?;
        Some(u32::from_le_bytes(b.try_into().ok()?))
    }

    /// A NUL-terminated string of at most `max` bytes.
    pub fn cstr(&self, va: u32, max: usize) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        for k in 0..max as u32 {
            let b = *self.read(va + k, 1)?.first()?;
            if b == 0 {
                return Some(out);
            }
            out.push(b);
        }
        Some(out)
    }

    /// The executable segments' words: (va, word) for every aligned word.
    pub fn code(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.segs.iter().filter(|s| s.exec).flat_map(move |s| {
            let d = &self.data[s.offset as usize..(s.offset + s.filesz) as usize];
            d.as_chunks::<4>().0.iter().enumerate().map(move |(i, w)| (s.vaddr + 4 * i as u32, u32::from_le_bytes(*w)))
        })
    }
}
