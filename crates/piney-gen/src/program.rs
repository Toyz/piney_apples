//! The EE's view of memory: a volume's boot executable plus, optionally,
//! one overlay (`DATA/<NAME>.PRG`, loaded whole at 0x00400800). The later
//! volumes' executables are stripped: their sections are known by position
//! and their names come from the `<elf>.syms` sidecar (`tools/xfer.py`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::elf::{Elf, STT_FILE, STT_FUNC, STT_OBJECT, STT_SECTION, Segment, Symbol};

pub const OVERLAYS: [&str; 4] = ["gcmn", "demo", "desktop", "toppage"];
/// A stripped executable keeps its section headers in the linker's order.
const SECTION_ORDER: [&str; 6] = ["main", "gcmn.prg", "demo.prg", "desktop.prg", "toppage.prg", "heap"];

/// A `.PRG` overlay: its header and the bytes as loaded.
pub struct Overlay {
    pub data: Vec<u8>,
    pub base: u32,
    pub text: u32,
    pub text_size: u32,
    pub data_size: u32,
    pub ctor_start: u32,
    pub ctor_end: u32,
    pub end: u32,
}

impl Overlay {
    pub fn open(path: &Path) -> Result<Overlay, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Overlay::parse(data, &path.display().to_string())
    }

    /// An overlay's bytes (`what` names it in errors).
    pub fn parse(data: Vec<u8>, what: &str) -> Result<Overlay, String> {
        if data.len() < 0x40 || &data[..4] != b"MWo3" {
            return Err(format!("{what}: not an MWo3 overlay"));
        }
        let w = |k: usize| u32::from_le_bytes(data[4 * k..4 * k + 4].try_into().unwrap());
        let (base, text_size, data_size, bss_size) = (w(2), w(3), w(4), w(5));
        let text = base + 0x40;
        Ok(Overlay {
            base,
            text,
            text_size,
            data_size,
            ctor_start: w(6),
            ctor_end: w(7),
            end: text + text_size + data_size + bss_size,
            data,
        })
    }

    fn read(&self, va: u32, n: usize) -> Vec<u8> {
        let off = (va - self.base) as usize;
        let mut b = self.data.get(off..(off + n).min(self.data.len())).unwrap_or_default().to_vec();
        // Past the file is .bss.
        b.resize(n, 0);
        b
    }
}

/// A relocation of main or the loaded overlay, with the address it
/// resolves to in the linked image: the word (R_MIPS_32), the `lui`/low
/// half pair (R_MIPS_LO16) or `$gp` and its offset (R_MIPS_GPREL16).
pub struct Reloc {
    pub offset: u32,
    pub kind: u8,
    /// Its symbol, an index into the ELF's symbols.
    pub symbol: usize,
    pub addr: Option<u32>,
}

pub struct Program {
    pub elf: Elf,
    pub main_index: usize,
    pub main_seg: Segment,
    pub overlay_index: HashMap<String, usize>,
    pub overlay_space: (u32, u32),
    pub gp: Option<u32>,
    pub overlay: Option<Overlay>,
    pub overlay_name: Option<String>,
    /// The active symbols by (value, -size).
    pub symbols: Vec<Symbol>,
    starts: Vec<u32>,
    named: HashMap<String, usize>,
    functions: Vec<usize>,
    fstarts: Vec<u32>,
    relocs: std::cell::OnceCell<Vec<Reloc>>,
}

/// Symbols from a sidecar's text: `section va size type name`,
/// tab-separated (`what` names it in errors).
fn symbols_from(text: &str, what: &str, index: &HashMap<String, usize>) -> Result<Vec<Symbol>, String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        let sec = if f[0] == "main" { "main".to_string() } else { format!("{}.prg", f[0]) };
        let Some(&shndx) = index.get(&sec) else { continue };
        out.push(Symbol {
            name: f[4].to_string(),
            value: u32::from_str_radix(f[1], 16).map_err(|e| format!("{what}: {e}"))?,
            size: f[2].parse().map_err(|e| format!("{what}: {e}"))?,
            kind: match f[3] {
                "FUNC" => STT_FUNC,
                "OBJECT" => STT_OBJECT,
                _ => 0,
            },
            shndx: shndx as u16,
        });
    }
    Ok(out)
}

impl Program {
    /// The executable at `elf_path` with its sidecar (`<elf>.syms`) and the
    /// overlay from `DATA/` beside it.
    pub fn open(elf_path: &Path, overlay: Option<&str>) -> Result<Program, String> {
        let side = PathBuf::from(format!("{}.syms", elf_path.display()));
        let dir = elf_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Program::from_parts(
            Elf::open(elf_path)?,
            || {
                side.exists()
                    .then(|| std::fs::read_to_string(&side).map(|t| (t, side.display().to_string())))
                    .transpose()
                    .map_err(|e| format!("{}: {e}", side.display()))
            },
            |path| std::fs::read(dir.join(path)).map_err(|e| format!("{}: {e}", dir.join(path).display())),
            overlay,
        )
    }

    /// The executable `elf`, a stripped one's symbols from `syms` (a
    /// sidecar's text and what to call it), and `overlay` read by
    /// `read(path on the disc)`.
    pub fn from_parts(
        mut elf: Elf,
        syms: impl FnOnce() -> Result<Option<(String, String)>, String>,
        read: impl FnOnce(&str) -> Result<Vec<u8>, String>,
        overlay: Option<&str>,
    ) -> Result<Program, String> {
        let mut index: HashMap<String, usize> =
            elf.sections.iter().enumerate().map(|(i, s)| (s.name.clone(), i)).collect();
        if !index.contains_key("main") {
            let loaded: Vec<usize> =
                elf.sections.iter().enumerate().filter(|(_, s)| s.kind == 1 && s.addr != 0).map(|(i, _)| i).collect();
            for (name, i) in SECTION_ORDER.iter().zip(loaded) {
                index.insert(name.to_string(), i);
            }
            if let Some((text, what)) = syms()? {
                let extra = symbols_from(&text, &what, &index)?;
                elf.symbols.extend(extra);
            }
        }
        let main_index = index["main"];
        let main = &elf.sections[main_index];
        let main_seg = *elf
            .segments
            .iter()
            .find(|s| s.kind == 1 && s.vaddr <= main.addr && main.addr < s.vaddr + s.memsz.max(1))
            .ok_or("no segment holds main")?;
        let overlay_index: HashMap<String, usize> =
            OVERLAYS.iter().filter_map(|n| index.get(&format!("{n}.prg")).map(|&i| (n.to_string(), i))).collect();
        let base = overlay_index.values().map(|&i| elf.sections[i].addr).min().unwrap_or(0);
        let top = elf.segments.iter().filter(|s| s.vaddr == base).map(|s| s.vaddr + s.memsz).max().unwrap_or(base);
        let gp = elf
            .sections
            .iter()
            .find(|s| s.name == ".reginfo")
            .map(|s| u32::from_le_bytes(elf.data[s.offset as usize + 20..s.offset as usize + 24].try_into().unwrap()))
            .filter(|&g| g != 0)
            .or_else(|| elf.symbols.iter().find(|s| s.name == "_gp").map(|s| s.value));
        let (ov, overlay_name) = match overlay {
            None => (None, None),
            Some(o) => {
                let name = o.to_lowercase().trim_end_matches(".prg").to_string();
                if !overlay_index.contains_key(&name) {
                    return Err(format!("unknown overlay {o:?}"));
                }
                let path = format!("DATA/{}.PRG", name.to_uppercase());
                (Some(Overlay::parse(read(&path)?, &path)?), Some(name))
            }
        };
        let other: Vec<usize> = overlay_index
            .iter()
            .filter(|(n, _)| Some(n.as_str()) != overlay_name.as_deref())
            .map(|(_, &i)| i)
            .collect();
        let active = |s: &Symbol| {
            !s.name.is_empty()
                && s.kind != STT_SECTION
                && s.kind != STT_FILE
                && s.shndx != 0
                && !other.contains(&(s.shndx as usize))
        };
        let mut symbols: Vec<Symbol> = elf.symbols.iter().filter(|s| active(s)).cloned().collect();
        symbols.sort_by_key(|s| (s.value, std::cmp::Reverse(s.size)));
        let starts = symbols.iter().map(|s| s.value).collect();
        let mut named: HashMap<String, usize> = HashMap::new();
        let rank = |k: u8| match k {
            STT_FUNC => 0,
            STT_OBJECT => 1,
            _ => 2,
        };
        for (i, s) in symbols.iter().enumerate() {
            match named.get(&s.name) {
                Some(&j) if rank(symbols[j].kind) <= rank(s.kind) => {}
                _ => {
                    named.insert(s.name.clone(), i);
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut functions = Vec::new();
        for (i, s) in symbols.iter().enumerate() {
            if s.kind == STT_FUNC && s.size != 0 && seen.insert(s.value) {
                functions.push(i);
            }
        }
        let fstarts = functions.iter().map(|&i| symbols[i].value).collect();
        Ok(Program {
            relocs: std::cell::OnceCell::new(),
            elf,
            main_index,
            main_seg,
            overlay_index,
            overlay_space: (base, top),
            gp,
            overlay: ov,
            overlay_name,
            symbols,
            starts,
            named,
            functions,
            fstarts,
        })
    }

    /// Whether `va` has bytes (or .bss) in this program.
    pub fn mapped(&self, va: u32) -> bool {
        if let Some(ov) = &self.overlay
            && ov.base <= va
            && va < ov.end
        {
            return true;
        }
        let m = &self.main_seg;
        m.vaddr <= va && va < m.vaddr + m.memsz
    }

    pub fn read(&self, va: u32, n: usize) -> Result<Vec<u8>, String> {
        if let Some(ov) = &self.overlay
            && ov.base <= va
            && va < ov.end
        {
            return Ok(ov.read(va, n));
        }
        let m = &self.main_seg;
        if m.vaddr <= va && va < m.vaddr + m.memsz {
            return self.elf.read(va, n).ok_or_else(|| format!("0x{va:08x} is not in a loaded segment"));
        }
        if self.overlay.is_none() && self.overlay_space.0 <= va && va < self.overlay_space.1 {
            return Err(format!("0x{va:08x} is overlay space; choose one with --overlay"));
        }
        Err(format!("0x{va:08x} is not mapped"))
    }

    pub fn u32(&self, va: u32) -> Result<u32, String> {
        let b = self.read(va, 4)?;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }

    pub fn symbol_named(&self, name: &str) -> Option<&Symbol> {
        self.named.get(name).map(|&i| &self.symbols[i])
    }

    /// (symbol, offset) for the innermost sized symbol covering `va`, else
    /// one starting exactly there, else the nearest at most `near` below.
    pub fn symbol_at(&self, va: u32, near: u32) -> Option<(&Symbol, u32)> {
        let i = self.starts.partition_point(|&s| s <= va);
        let mut exact = None;
        for j in (i.saturating_sub(200)..i).rev() {
            let s = &self.symbols[j];
            if s.value <= va && va < s.value.wrapping_add(s.size) {
                return Some((s, va - s.value));
            }
            if s.value == va && exact.is_none() {
                exact = Some(s);
            }
        }
        if let Some(s) = exact {
            return Some((s, 0));
        }
        if i > 0 && va - self.symbols[i - 1].value <= near {
            let s = &self.symbols[i - 1];
            return Some((s, va - s.value));
        }
        None
    }

    /// The first symbol starting past `va` (in address order, larger first
    /// at one address).
    pub fn following(&self, va: u32) -> Option<&Symbol> {
        self.symbols.get(self.starts.partition_point(|&s| s <= va))
    }

    /// `symbol_at`'s name, with `+0x..` for an offset into it.
    pub fn name_at(&self, va: u32, near: u32) -> Option<String> {
        let (s, off) = self.symbol_at(va, near)?;
        Some(if off == 0 { s.name.clone() } else { format!("{}+0x{off:x}", s.name) })
    }

    /// The sized function covering `va`.
    pub fn function_at(&self, va: u32) -> Option<&Symbol> {
        let i = self.fstarts.partition_point(|&s| s <= va);
        let f = &self.symbols[*self.functions.get(i.checked_sub(1)?)?];
        (f.value <= va && va < f.value + f.size).then_some(f)
    }

    /// The code range of main (with its data) or of the loaded overlay.
    pub fn code_range(&self, overlay: bool) -> (u32, u32) {
        match (&self.overlay, overlay) {
            (Some(ov), true) => (ov.text, ov.text + ov.text_size),
            _ => (self.main_seg.vaddr, self.main_seg.vaddr + self.main_seg.filesz),
        }
    }

    /// The C string at `va`, up to a NUL within `limit` bytes.
    pub fn cstr(&self, va: u32, limit: usize) -> Result<Vec<u8>, String> {
        let b = self.read(va, limit)?;
        Ok(b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())].to_vec())
    }

    pub fn section_name(&self, shndx: u16) -> String {
        match self.elf.sections.get(shndx as usize) {
            Some(s) if shndx != 0 => s.name.clone(),
            _ => match shndx {
                0 => "UND".into(),
                0xfff1 => "ABS".into(),
                0xfff2 => "COMMON".into(),
                n => n.to_string(),
            },
        }
    }

    /// The sized functions, one a place, by address.
    pub fn functions(&self) -> impl Iterator<Item = &Symbol> {
        self.functions.iter().map(|&i| &self.symbols[i])
    }

    /// Every relocation of main and the loaded overlay, resolved.
    pub fn relocs(&self) -> &[Reloc] {
        self.relocs.get_or_init(|| {
            let mut names = vec!["main".to_string()];
            if let Some(o) = &self.overlay_name {
                names.push(format!("{o}.prg"));
            }
            names.iter().flat_map(|n| self.resolve(self.elf.relocs(n))).collect()
        })
    }

    /// One section's relocations resolved. A LO16's address is its
    /// partner lui's immediate plus its own. The ABI pairs a LO16 with the
    /// latest HI16 in table order, but code that shares one lui across uses
    /// breaks that: the nearest lui before it in the function that writes
    /// the register it uses is its partner.
    fn resolve(&self, raw: Vec<(u32, u8, usize)>) -> Vec<Reloc> {
        // Per symbol, its HI16s: (offset, the lui's rt, index), by offset.
        let mut his: HashMap<usize, Vec<(u32, u32, usize)>> = HashMap::new();
        for (k, &(off, kind, sym)) in raw.iter().enumerate() {
            if kind == 5
                && let Ok(w) = self.u32(off)
            {
                his.entry(sym).or_default().push((off, (w >> 16) & 31, k));
            }
        }
        for v in his.values_mut() {
            v.sort_by_key(|h| h.0);
        }
        let mut out: Vec<Reloc> =
            raw.iter().map(|&(offset, kind, symbol)| Reloc { offset, kind, symbol, addr: None }).collect();
        let mut last_hi: HashMap<usize, usize> = HashMap::new();
        for k in 0..out.len() {
            let Ok(w) = self.u32(out[k].offset) else { continue };
            let (offset, sym) = (out[k].offset, out[k].symbol);
            match out[k].kind {
                4 => out[k].addr = Some((offset.wrapping_add(4) & 0xf000_0000) | ((w & 0x03ff_ffff) << 2)),
                2 => out[k].addr = Some(w),
                7 => out[k].addr = Some(self.gp.unwrap_or(0).wrapping_add(sext16(w))),
                5 => {
                    last_hi.insert(sym, k);
                }
                6 => {
                    let hi = self.partner(his.get(&sym).map_or(&[][..], |v| v), offset, (w >> 21) & 31);
                    let hi = hi.or_else(|| last_hi.get(&sym).copied());
                    let lo = sext16(w);
                    match hi {
                        Some(h) => {
                            let a = ((self.u32(out[h].offset).unwrap_or(0) & 0xffff) << 16).wrapping_add(lo);
                            out[k].addr = Some(a);
                            if out[h].addr.is_none() {
                                out[h].addr = Some(a);
                            }
                        }
                        None => {
                            let v = self.elf.symbols.get(sym).map_or(0, |s| s.value);
                            out[k].addr = Some(v.wrapping_add(sext16(lo.wrapping_sub(v))));
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The HI16 before `offset` in its function (or within 0x1000) whose
    /// lui writes `reg`: an index into the section's relocations.
    fn partner(&self, his: &[(u32, u32, usize)], offset: u32, reg: u32) -> Option<usize> {
        let lo = self.function_at(offset).map_or(offset.saturating_sub(0x1000), |f| f.value);
        let i = his.partition_point(|h| h.0 < offset);
        for &(off, rt, k) in his[i.saturating_sub(64)..i].iter().rev() {
            if off < lo {
                break;
            }
            if rt == reg {
                return Some(k);
            }
        }
        None
    }
}

fn sext16(v: u32) -> u32 {
    (v as u16 as i16) as i32 as u32
}
