//! The EE program image: the boot ELF (`SLUS_202.67`) and the `.PRG`
//! overlays that load over it at 0x00400800 (`docs/formats/prg.md`).
//!
//! Code the port has made its own does not come from here; the image is for
//! what the game keeps in memory as content: the mail and wallpaper tables
//! of `DESKTOP.PRG`, the bitmap fonts of the executable. Both are read by
//! virtual address, as the game's pointers name them. The port never reads
//! the executable at run time, in any form ([`Image::elf`] is for the tools
//! and the checks); its main-section data is being rebuilt as generated
//! tables (`plans/volumes.md`, "The shortcut removed").

use crate::{Bytes, Result, format_err};

/// One loaded range of the image.
#[derive(Clone, Debug)]
struct Span {
    va: u32,
    bytes: Vec<u8>,
    /// Bytes past `bytes` up to here read as zero (.bss).
    mem_end: u32,
}

/// Bytes by virtual address: an ELF's loaded segments, or an overlay.
#[derive(Clone, Debug, Default)]
pub struct Image {
    spans: Vec<Span>,
    /// The overlay loaded over main (its header's id), 0 for none.
    overlay: u32,
}

impl Image {
    /// The PT_LOAD segments of a 32-bit little-endian ELF with their file
    /// bytes (the retail ELF's overlay segments carry none).
    pub fn elf(data: &[u8]) -> Result<Self> {
        if data.slice_at(0, 4)? != b"\x7fELF" || data.u8_at(4)? != 1 || data.u8_at(5)? != 1 {
            return format_err("not a 32-bit little-endian ELF");
        }
        let phoff = data.u32_at(28)? as usize;
        let phentsize = data.u16_at(42)? as usize;
        let phnum = data.u16_at(44)? as usize;
        let mut spans = Vec::new();
        for i in 0..phnum {
            let p = phoff + i * phentsize;
            if data.u32_at(p)? != 1 {
                continue;
            }
            let offset = data.u32_at(p + 4)? as usize;
            let va = data.u32_at(p + 8)?;
            let filesz = data.u32_at(p + 16)? as usize;
            let memsz = data.u32_at(p + 20)?;
            if filesz == 0 {
                continue;
            }
            spans.push(Span { va, bytes: data.slice_at(offset, filesz)?.to_vec(), mem_end: va + memsz });
        }
        Ok(Image { spans, ..Image::default() })
    }

    /// Add another image's spans over this one's: an overlay over the ELF.
    pub fn with(mut self, other: &Image) -> Self {
        // Later spans win lookups, so the overlay goes first.
        let mut spans = other.spans.clone();
        spans.append(&mut self.spans);
        self.spans = spans;
        if other.overlay != 0 {
            self.overlay = other.overlay;
        }
        self
    }

    fn span(&self, va: u32, n: usize) -> Option<&Span> {
        self.spans.iter().find(|s| va >= s.va && (va as u64 + n as u64) <= s.mem_end as u64)
    }

    /// `n` bytes at `va`, zeros past the stored bytes (.bss).
    pub fn read(&self, va: u32, n: usize) -> Result<Vec<u8>> {
        let Some(s) = self.span(va, n) else {
            return format_err(format!("0x{va:08x}+{n} is not in the image"));
        };
        let off = (va - s.va) as usize;
        let mut out = vec![0u8; n];
        if off < s.bytes.len() {
            let k = n.min(s.bytes.len() - off);
            out[..k].copy_from_slice(&s.bytes[off..off + k]);
        }
        Ok(out)
    }

    pub fn u8(&self, va: u32) -> Result<u8> {
        Ok(self.read(va, 1)?[0])
    }

    pub fn u16(&self, va: u32) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read(va, 2)?.try_into().unwrap()))
    }

    pub fn i16(&self, va: u32) -> Result<i16> {
        Ok(self.u16(va)? as i16)
    }

    pub fn u32(&self, va: u32) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read(va, 4)?.try_into().unwrap()))
    }

    pub fn i32(&self, va: u32) -> Result<i32> {
        Ok(self.u32(va)? as i32)
    }

    /// The NUL-terminated bytes at `va`, without the NUL (at most 64 KiB).
    pub fn cstr(&self, va: u32) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut p = va;
        loop {
            let b = self.u8(p)?;
            if b == 0 {
                return Ok(out);
            }
            out.push(b);
            p += 1;
            if out.len() > 0xffff {
                return format_err(format!("string at 0x{va:08x} does not end"));
            }
        }
    }

    /// `lines` NUL-terminated strings laid back to back from `va`, as a mail
    /// body is. Returns each without its NUL.
    pub fn lines(&self, va: u32, lines: usize) -> Result<Vec<Vec<u8>>> {
        let mut out = Vec::with_capacity(lines);
        let mut p = va;
        for _ in 0..lines {
            let s = self.cstr(p)?;
            p += s.len() as u32 + 1;
            out.push(s);
        }
        Ok(out)
    }
}

/// A `.PRG` overlay's header (`docs/formats/prg.md`).
#[derive(Clone, Debug)]
pub struct Overlay {
    pub id: u32,
    pub load_va: u32,
    pub text_size: u32,
    pub data_size: u32,
    pub bss_size: u32,
    pub ctor_start: u32,
    pub ctor_end: u32,
    pub name: String,
    /// The file as loaded at `load_va`, with its .bss.
    pub image: Image,
}

impl Overlay {
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.slice_at(0, 4)? != b"MWo3" {
            return format_err("not an MWo3 overlay");
        }
        let id = data.u32_at(4)?;
        let load_va = data.u32_at(8)?;
        let text_size = data.u32_at(12)?;
        let data_size = data.u32_at(16)?;
        let bss_size = data.u32_at(20)?;
        let ctor_start = data.u32_at(24)?;
        let ctor_end = data.u32_at(28)?;
        let name = crate::cstr(data.slice_at(32, 32)?);
        let mem_end = load_va + data.len() as u32 + bss_size;
        let image = Image { spans: vec![Span { va: load_va, bytes: data, mem_end }], overlay: id };
        Ok(Overlay { id, load_va, text_size, data_size, bss_size, ctor_start, ctor_end, name, image })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_header_and_bss() {
        let mut d = b"MWo3".to_vec();
        for w in [3u32, 0x0040_0800, 8, 4, 16, 0, 0] {
            d.extend_from_slice(&w.to_le_bytes());
        }
        d.extend_from_slice(&[0u8; 32]);
        d.extend_from_slice(b"hello\0\0\0");
        d.extend_from_slice(&7u32.to_le_bytes());
        let o = Overlay::parse(d).unwrap();
        assert_eq!(o.image.cstr(0x0040_0840).unwrap(), b"hello");
        assert_eq!(o.image.u32(0x0040_0848).unwrap(), 7);
        assert_eq!(o.image.u32(0x0040_0850).unwrap(), 0);
        assert!(o.image.u32(0x0040_0860).is_err());
    }
}
