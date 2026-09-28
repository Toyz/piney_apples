//! A 32-bit little-endian MIPS ELF as the EE uses it: program headers,
//! sections, symbols and the relocations that apply to a section.

use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub kind: u32,
    pub offset: u32,
    pub vaddr: u32,
    pub filesz: u32,
    pub memsz: u32,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub kind: u32,
    pub addr: u32,
    pub offset: u32,
    pub size: u32,
    pub link: u32,
    pub info: u32,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub value: u32,
    pub size: u32,
    /// STT_*: 1 object, 2 function, 3 section, 4 file.
    pub kind: u8,
    pub shndx: u16,
}

pub const STT_OBJECT: u8 = 1;
pub const STT_FUNC: u8 = 2;
pub const STT_SECTION: u8 = 3;
pub const STT_FILE: u8 = 4;

pub struct Elf {
    pub data: Vec<u8>,
    pub segments: Vec<Segment>,
    pub sections: Vec<Section>,
    pub symbols: Vec<Symbol>,
}

fn u16_at(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

fn cstr(d: &[u8], at: usize) -> String {
    let end = d[at..].iter().position(|&b| b == 0).map_or(d.len(), |n| at + n);
    // Latin-1: one char a byte.
    d[at..end].iter().map(|&b| b as char).collect()
}

impl Elf {
    pub fn open(path: &Path) -> Result<Elf, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Elf::parse(data, &path.display().to_string())
    }

    /// An executable's bytes (`what` names it in errors).
    pub fn parse(data: Vec<u8>, what: &str) -> Result<Elf, String> {
        if data.len() < 52 || &data[..4] != b"\x7fELF" || data[4] != 1 || data[5] != 1 {
            return Err(format!("{what}: not a 32-bit little-endian ELF"));
        }
        let phoff = u32_at(&data, 28) as usize;
        let shoff = u32_at(&data, 32) as usize;
        let phentsize = u16_at(&data, 42) as usize;
        let phnum = u16_at(&data, 44) as usize;
        let shentsize = u16_at(&data, 46) as usize;
        let shnum = u16_at(&data, 48) as usize;
        let shstrndx = u16_at(&data, 50) as usize;
        let segments = (0..phnum)
            .map(|i| {
                let p = phoff + i * phentsize;
                Segment {
                    kind: u32_at(&data, p),
                    offset: u32_at(&data, p + 4),
                    vaddr: u32_at(&data, p + 8),
                    filesz: u32_at(&data, p + 16),
                    memsz: u32_at(&data, p + 20),
                }
            })
            .collect();
        let raw: Vec<[u32; 10]> = if shoff == 0 {
            Vec::new()
        } else {
            (0..shnum).map(|i| std::array::from_fn(|k| u32_at(&data, shoff + i * shentsize + 4 * k))).collect()
        };
        let mut sections = Vec::with_capacity(raw.len());
        if let Some(strtab) = raw.get(shstrndx) {
            for r in &raw {
                sections.push(Section {
                    name: cstr(&data, (strtab[4] + r[0]) as usize),
                    kind: r[1],
                    addr: r[3],
                    offset: r[4],
                    size: r[5],
                    link: r[6],
                    info: r[7],
                });
            }
        }
        let mut symbols = Vec::new();
        for s in &sections {
            if s.kind != 2 && s.kind != 11 {
                continue;
            }
            let strs = &sections[s.link as usize];
            for off in (s.offset..s.offset + s.size).step_by(16) {
                let off = off as usize;
                symbols.push(Symbol {
                    name: cstr(&data, (strs.offset + u32_at(&data, off)) as usize),
                    value: u32_at(&data, off + 4),
                    size: u32_at(&data, off + 8),
                    kind: data[off + 12] & 15,
                    shndx: u16_at(&data, off + 14),
                });
            }
        }
        Ok(Elf { data, segments, sections, symbols })
    }

    fn file_offset(&self, va: u32) -> Option<usize> {
        self.segments
            .iter()
            .find(|s| s.kind == 1 && s.vaddr <= va && va < s.vaddr + s.filesz)
            .map(|s| (s.offset + (va - s.vaddr)) as usize)
    }

    /// `n` bytes at `va`; `.bss` reads as zeros.
    pub fn read(&self, va: u32, n: usize) -> Option<Vec<u8>> {
        match self.file_offset(va) {
            Some(off) => {
                let mut b = self.data[off..(off + n).min(self.data.len())].to_vec();
                b.resize(n, 0);
                Some(b)
            }
            None => {
                self.segments.iter().any(|s| s.kind == 1 && s.vaddr <= va && va < s.vaddr + s.memsz).then(|| vec![0; n])
            }
        }
    }

    /// (offset, type, symbol index) of every SHT_REL entry applying to the
    /// section `name`.
    pub fn relocs(&self, name: &str) -> Vec<(u32, u8, usize)> {
        let Some(idx) = self.sections.iter().position(|s| s.name == name) else { return Vec::new() };
        let mut out = Vec::new();
        for s in &self.sections {
            if s.kind != 9 || s.info as usize != idx {
                continue;
            }
            for off in (s.offset..s.offset + s.size).step_by(8) {
                let off = off as usize;
                let info = u32_at(&self.data, off + 4);
                out.push((u32_at(&self.data, off), (info & 0xff) as u8, (info >> 8) as usize));
            }
        }
        out
    }
}
