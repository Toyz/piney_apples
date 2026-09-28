//! Where the generator reads a volume's files: the disc the caller named for
//! it ([`use_disc`]: piney-build's discs, or the image the game plays), else
//! the extracted copy in `work/<volume>/disc/` that the tools and checks use.
//!
//! With a disc named, the later volumes' carried names ([`syms`]) and carry
//! ([`crate::carry::cached`]) are made in memory from it and Infection's
//! disc, rather than read from their caches in `work/`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use piney_data::iso::Iso;

use crate::volume::{Vol, elf_path};

static DISCS: Mutex<[Option<PathBuf>; 4]> = Mutex::new([None, None, None, None]);

/// Volume `v`'s files come from `disc` (an image, or a build's `.disc`)
/// from here on.
pub fn use_disc(v: Vol, disc: PathBuf) {
    DISCS.lock().unwrap_or_else(|e| e.into_inner())[v.index()] = Some(disc);
}

/// The disc named for `v`, if any.
pub fn disc(v: Vol) -> Option<PathBuf> {
    DISCS.lock().unwrap_or_else(|e| e.into_inner())[v.index()].clone()
}

/// A file of the volume's disc by its path there (`SLUS_205.62`,
/// `DATA/GCMN.PRG`).
pub fn read(v: Vol, path: &str) -> Result<Vec<u8>, String> {
    match disc(v) {
        Some(d) => {
            let mut iso = Iso::open(&d).map_err(|e| format!("{}: {e}", d.display()))?;
            iso.read_path(path).map_err(|e| format!("{}: {path}: {e}", d.display()))
        }
        None => {
            let p = elf_path(v).parent().unwrap().join(path);
            std::fs::read(&p)
                .map_err(|e| format!("{}: {e} (no {} disc named, and not extracted in work/)", p.display(), v.tag()))
        }
    }
}

thread_local! {
    /// The carried names made this run, by volume.
    static MADE: RefCell<HashMap<Vol, String>> = RefCell::new(HashMap::new());
    /// The volumes whose names are being made: their programs open bare.
    static MAKING: RefCell<Vec<Vol>> = const { RefCell::new(Vec::new()) };
}

/// Every named function and object of the volume as a sidecar's text
/// (`section va size type name how`): Infection's from its own executable
/// (`how` "elf"), a later volume's the names carried to it ([`syms`]).
/// What `piney-build --export-symbols` writes.
pub fn symbols_text(v: Vol) -> Result<String, String> {
    if v != Vol::Inf {
        return syms(v)?.map(|(t, _)| t).ok_or_else(|| format!("{}: no carried names", v.tag()));
    }
    // The ELF's own table: main's and every overlay's (a program opened
    // with no overlay keeps only main's).
    let p = crate::volume::ctx(v, None).p;
    let mut rows: Vec<(String, u32, u32, &str, &str)> = Vec::new();
    for s in &p.elf.symbols {
        let kind = match s.kind {
            crate::elf::STT_FUNC => "FUNC",
            crate::elf::STT_OBJECT => "OBJECT",
            _ => continue,
        };
        if s.name.is_empty() {
            continue;
        }
        let sec = p.section_name(s.shndx);
        let sec = match sec.strip_suffix(".prg") {
            Some(o) => o.to_string(),
            None if sec == "main" || s.value < 0x0040_0000 => "main".to_string(),
            None => continue,
        };
        rows.push((sec, s.value, s.size, kind, &s.name));
    }
    rows.sort_by(|a, b| (&a.0, a.1, a.4).cmp(&(&b.0, b.1, b.4)));
    rows.dedup();
    let mut text = format!("# symbols of {} from its own executable\n# section\tva\tsize\ttype\tname\thow\n", v.exe());
    for (sec, va, size, kind, name) in rows {
        text.push_str(&format!("{sec}\t{va:08x}\t{size}\t{kind}\t{name}\telf\n"));
    }
    Ok(text)
}

/// A stripped volume's carried names as a sidecar's text, and what to call
/// it: made from the discs when one is named for it (`piney-gen syms`'s
/// transfer, a few seconds), else the sidecar in `work/`. None for
/// Infection (its own symbols) and while the names are being made.
pub fn syms(v: Vol) -> Result<Option<(String, String)>, String> {
    if v == Vol::Inf || MAKING.with(|m| m.borrow().contains(&v)) {
        return Ok(None);
    }
    if disc(v).is_none() {
        let side = PathBuf::from(format!("{}.syms", elf_path(v).display()));
        return side
            .exists()
            .then(|| std::fs::read_to_string(&side).map(|t| (t, side.display().to_string())))
            .transpose()
            .map_err(|e| format!("{}: {e}", side.display()));
    }
    if let Some(t) = MADE.with(|m| m.borrow().get(&v).cloned()) {
        return Ok(Some((t, format!("{}'s carried names", v.tag()))));
    }
    // The transfer opens the volume's programs without names; they are
    // dropped after, so the ones opened next carry them.
    MAKING.with(|m| m.borrow_mut().push(v));
    let text = crate::syms::text(v);
    MAKING.with(|m| m.borrow_mut().retain(|&x| x != v));
    crate::volume::forget(v);
    crate::xfer::forget(v);
    MADE.with(|m| m.borrow_mut().insert(v, text.clone()));
    Ok(Some((text, format!("{}'s carried names", v.tag()))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Infection's names as `piney-build --export-symbols` writes them: the
    /// overlays' as well as main's (`ccFellow::Influence` in gcmn).
    #[test]
    fn infections_names_cover_the_overlays() {
        if !elf_path(Vol::Inf).exists() {
            eprintln!("skipped: no Infection executable in work/");
            return;
        }
        let text = symbols_text(Vol::Inf).unwrap();
        for sec in ["main", "gcmn", "demo", "desktop", "toppage"] {
            assert!(text.lines().any(|l| l.starts_with(&format!("{sec}\t"))), "no {sec} names");
        }
        assert!(text.contains("gcmn\t0041bdb0\t2228\tFUNC\tInfluence__8ccFellowFv\telf\n"));
    }
}
