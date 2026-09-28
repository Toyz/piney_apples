//! Where Infection's globals and functions sit in a later volume's
//! executable: a finding aid for the generator, never shipped in the port
//! (`plans/volumes.md`, "The shortcut removed").
//!
//! `work/analysis/carry/<volume>.json` holds rows of (section, Infection's
//! address, Infection's size, the volume's address), sorted; section 0
//! main, then the overlay ids: 1 gcmn, 2 demo, 3 desktop, 4 toppage. A
//! main global can also have an overlay's row: where that overlay's code
//! finds it (a copy the volume moved there). Found, in order:
//!
//! - by the names the symbol transfer carried, unique in their section on
//!   both sides (an overlay's global the volume moved into main found
//!   there);
//! - by what the paired functions build at their aligned instructions, for
//!   names that repeat (string literals' `@123`); this also overrules a
//!   name that disagrees, unless the name's place reads more like
//!   Infection's;
//! - globals left over, of 8 bytes or more and holding no pointer, found
//!   once by their bytes;
//! - globals of data pointers, where the volume's words point at the same
//!   bytes, or, for a table the volume edited, where most of them do
//!   (`by_pointers`);
//! - globals a changed function builds, by order (`by_sequence`);
//! - the two bitmap fonts (no size in Infection's symbols) by content.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use memchr::memmem;

use crate::elf::{STT_FUNC, STT_OBJECT, Symbol};
use crate::program::Program;
use crate::volume::{Vol, ctx, root};
use crate::xfer::{self, Side, side};

/// The overlays, by their PRG header ids (`docs/formats/prg.md`).
const OVERLAYS: [(&str, u32); 4] = [("gcmn", 1), ("demo", 2), ("desktop", 3), ("toppage", 4)];

/// Main, then each overlay: (the overlay, its section id).
fn sections() -> impl Iterator<Item = (Option<&'static str>, u32)> {
    std::iter::once((None, 0)).chain(OVERLAYS.iter().map(|&(n, id)| (Some(n), id)))
}

/// The two bitmap fonts' lengths: their symbols have none. Byte for byte
/// the same on the four discs, so the carry finds them by content
/// (`ef12x20`'s length: `docs/disc/volumes.md`, "The font tables in
/// Quarantine").
const FONTS: [(&str, u32); 2] = [("ef12x20", 13440), ("ef8x16", 7168)];

/// A main global and an overlay's sit either side of this.
const OVERLAY_SPACE: u32 = 0x0040_0000;

/// {(section, Infection's address): (its size, the volume's address)}.
type Out = BTreeMap<(u32, u32), (u32, u32)>;

/// The section index of main or an overlay in a program.
fn section_index(p: &Program, overlay: Option<&str>) -> usize {
    match overlay {
        None => p.main_index,
        Some(o) => p.overlay_index[o],
    }
}

/// The sized functions and objects in main (overlay None) or in `overlay`
/// whose names occur once there, in the order the symbols first name them
/// (a repeat at the same place is the later symbol).
struct Named {
    order: Vec<String>,
    by: HashMap<String, Symbol>,
}

impl Named {
    fn get(&self, name: &str) -> Option<&Symbol> {
        self.by.get(name)
    }

    fn iter(&self) -> impl Iterator<Item = (&str, &Symbol)> {
        self.order.iter().map(|n| (n.as_str(), &self.by[n]))
    }
}

fn section_symbols(p: &Program, overlay: Option<&str>) -> Named {
    let want = section_index(p, overlay);
    let (mut order, mut by, mut dup) = (Vec::new(), HashMap::<String, Symbol>::new(), HashSet::new());
    for s in &p.symbols {
        if s.shndx as usize != want || !matches!(s.kind, STT_OBJECT | STT_FUNC) || s.size == 0 {
            continue;
        }
        match by.get(&s.name) {
            Some(old) => {
                if old.value != s.value {
                    dup.insert(s.name.clone());
                }
            }
            None => order.push(s.name.clone()),
        }
        by.insert(s.name.clone(), s.clone());
    }
    order.retain(|n| !dup.contains(n));
    by.retain(|n, _| !dup.contains(n));
    Named { order, by }
}

/// {word relocation's address: its symbol's section} of main and the
/// loaded overlay (Infection's; the later volumes have none).
fn word_relocs(p: &Program) -> HashMap<u32, u16> {
    let mut names = vec!["main".to_string()];
    if let Some(o) = &p.overlay_name {
        names.push(format!("{o}.prg"));
    }
    let mut out = HashMap::new();
    for n in names {
        for (off, kind, sym) in p.elf.relocs(&n) {
            if kind == 2 {
                out.insert(off, p.elf.symbols.get(sym).map_or(0, |s| s.shndx));
            }
        }
    }
    out
}

/// {word relocation's address: its symbol's section}.
type WordRelocs = Rc<HashMap<u32, u16>>;

thread_local! {
    static RELOCS: RefCell<HashMap<Option<&'static str>, WordRelocs>> = RefCell::new(HashMap::new());
}

/// Infection's word relocations with the overlay loaded.
fn inf_relocs(overlay: Option<&'static str>) -> WordRelocs {
    if let Some(r) = RELOCS.with(|m| m.borrow().get(&overlay).cloned()) {
        return r;
    }
    let r = Rc::new(word_relocs(&ctx(Vol::Inf, overlay).p));
    RELOCS.with(|m| m.borrow_mut().insert(overlay, r.clone()));
    r
}

/// {(name, Infection's address, size): the volume's address} of every
/// global the paired functions build at their aligned instructions, where
/// the votes agree. A vote at the global's own start outranks one into it:
/// a later volume's table can have grown, so an element's offset from the
/// start moves (Mutation's skillTbl).
fn votes(src: &Side, dst: &Side, pairs: &[(Symbol, u32, u32)]) -> BTreeMap<(String, u32, u32), u32> {
    type Tally = HashMap<(String, u32, u32), HashMap<u32, u32>>;
    let (mut at_start, mut inside): (Tally, Tally) = Default::default();
    for (f, dva, n_dst) in pairs {
        let (Some(sw), Some(dw)) = (src.run(f.value, f.size / 4), dst.run(*dva, *n_dst)) else { continue };
        let sa = xfer::addresses(sw, src.p.gp);
        let da = xfer::addresses(dw, dst.p.gp);
        for (i, j) in xfer::aligned(sw, dw) {
            let (Some(&s_addr), Some(&d_addr)) = (sa.get(&i), da.get(&j)) else { continue };
            if sw[i] >> 26 != dw[j] >> 26 {
                continue;
            }
            let Some((sym, off)) = src.p.symbol_at(s_addr, 0) else { continue };
            if sym.kind != STT_OBJECT {
                continue;
            }
            let t = if off == 0 { &mut at_start } else { &mut inside };
            *t.entry((sym.name.clone(), sym.value, sym.size))
                .or_default()
                .entry(d_addr.wrapping_sub(off))
                .or_default() += 1;
        }
    }
    let mut out = BTreeMap::new();
    let keys: HashSet<&(String, u32, u32)> = at_start.keys().chain(inside.keys()).collect();
    for key in keys {
        let counter = at_start.get(key).or_else(|| inside.get(key)).unwrap();
        if counter.len() == 1 {
            out.insert(key.clone(), *counter.keys().next().unwrap());
        }
    }
    out
}

/// The text a pointer points at, up to a NUL or 64 bytes; None where
/// nothing is mapped.
fn text(p: &Program, va: u32) -> Option<Vec<u8>> {
    for n in [64, 16, 4] {
        if let Ok(b) = p.read(va, n) {
            return Some(b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())].to_vec());
        }
    }
    None
}

#[derive(PartialEq)]
enum Sig {
    Pointer(Option<Vec<u8>>),
    Word(u32),
}

/// Whether a global of Infection's (at `value`, `size` bytes) is more like
/// Infection's at the `named` place than at the `built` one: word for
/// word, a pointer (Infection's word relocations) standing for the bytes
/// it points at, up to a NUL or 64; a pointer to data with no text is left
/// out (it points at the volume's own tables). The later volumes change
/// their tables a little, so neither place need match whole.
fn content_agrees(
    src: &Side,
    dst: &Side,
    overlay: Option<&'static str>,
    value: u32,
    size: u32,
    named: u32,
    built: u32,
) -> bool {
    if size == 0 || !size.is_multiple_of(4) {
        return false;
    }
    let size = size.min(0x400);
    let relocated = inf_relocs(overlay);
    let signature = |p: &Program, at: u32| -> Option<Vec<Sig>> {
        let mut out = Vec::new();
        for k in (0..size).step_by(4) {
            let w = p.u32(at.wrapping_add(k)).ok()?;
            out.push(if relocated.contains_key(&(value + k)) { Sig::Pointer(text(p, w)) } else { Sig::Word(w) });
        }
        Some(out)
    };
    let Some(want) = signature(&src.p, value) else { return false };
    let score = |at: u32| -> Option<usize> {
        let mine = signature(&dst.p, at)?;
        Some(mine.iter().zip(&want).filter(|(x, y)| x == y && **y != Sig::Pointer(Some(Vec::new()))).count())
    };
    match (score(named), score(built)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// The volume's carry: the rows, sorted.
pub fn carry(v: Vol) -> Vec<(u32, u32, u32, u32)> {
    let mut out: Out = BTreeMap::new();
    let vol_main = section_symbols(&ctx(v, None).p, None);
    for (ov, sec) in sections() {
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let inf = section_symbols(&src.p, ov);
        let mine = section_symbols(&dst.p, ov);
        for (name, s) in inf.iter() {
            // An overlay's global the volume moved into main (Outbreak's and
            // Quarantine's charTbl, DEMO.PRG's in Infection): main is loaded
            // under every overlay.
            let d = mine.get(name).or_else(|| if ov.is_some() { vol_main.get(name) } else { None });
            if let Some(d) = d {
                out.insert((sec, s.value), (s.size, d.value));
            }
        }
        let pairs: Vec<(Symbol, u32, u32)> = inf
            .iter()
            .filter(|(_, s)| s.kind == STT_FUNC)
            .filter_map(|(n, s)| mine.get(n).map(|d| (s.clone(), d.value, d.size / 4)))
            .collect();
        for ((name, value, size), va) in votes(&src, &dst, &pairs) {
            // A main global seen from an overlay's code is keyed by that
            // overlay when the volume keeps a copy of it there (Outbreak's
            // tpcTradeList in GCMN.PRG); the lookup tries the loaded
            // overlay's key first.
            let key = (if ov.is_some() || value >= OVERLAY_SPACE { sec } else { 0 }, value);
            if ov.is_some() && value < OVERLAY_SPACE && out.get(&(0, value)).map(|x| x.1) == Some(va) {
                continue;
            }
            if size == 0 || !dst.p.mapped(va) {
                continue;
            }
            // What the paired code builds outranks a name the carry put
            // elsewhere (Mutation's skillTbl), unless the global has no
            // pointers in it and only the named place holds Infection's
            // bytes (the play types, which a vote misplaced).
            if let Some(&(_, at)) = out.get(&key)
                && at != va
            {
                if content_agrees(&src, &dst, ov, value, size, at, va) {
                    continue;
                }
                eprintln!("{}: {name} named at {at:08x}, built at {va:08x}; the code wins", v.tag());
            }
            out.insert(key, (size, va));
        }
    }
    by_content(v, &mut out);
    by_pointers(v, &mut out);
    by_sequence(v, &mut out);
    let (src, dp) = (ctx(Vol::Inf, None).p, ctx(v, None).p);
    let main = dp.read(dp.main_seg.vaddr, dp.main_seg.filesz as usize).unwrap_or_else(|e| crate::die(&e));
    for (name, size) in FONTS {
        let f = src.symbol_named(name).unwrap_or_else(|| crate::die(&format!("no {name}")));
        let want = src.read(f.value, size as usize).unwrap_or_else(|e| crate::die(&e));
        let at = memmem::find(&main, &want)
            .unwrap_or_else(|| crate::die(&format!("{}: {name} not found by content", v.tag())));
        out.insert((0, f.value), (size, dp.main_seg.vaddr + at as u32));
    }
    out.into_iter().map(|((sec, inf), (size, va))| (sec, inf, size, va)).collect()
}

/// An uncarried global of 8 bytes or more (no pointers in it, as
/// Infection's relocations say) found exactly once in the volume's section.
fn by_content(v: Vol, out: &mut Out) {
    for (ov, sec) in sections() {
        let (src, dst) = (ctx(Vol::Inf, ov).p, ctx(v, ov).p);
        let want = section_index(&src, ov);
        let (lo, hi, _) = xfer::span(&dst, ov);
        let hay = dst.read(lo, (hi - lo) as usize).unwrap_or_else(|e| crate::die(&e));
        let relocated = inf_relocs(ov);
        for sym in &src.symbols {
            if sym.shndx as usize != want
                || sym.kind != STT_OBJECT
                || sym.size < 8
                || out.contains_key(&(sec, sym.value))
            {
                continue;
            }
            if (sym.value..sym.value + sym.size).step_by(4).any(|a| relocated.contains_key(&a)) {
                continue;
            }
            let Ok(needle) = src.read(sym.value, sym.size as usize) else { continue };
            if needle.iter().all(|&b| b == 0) {
                continue;
            }
            if let Some(at) = memmem::find(&hay, &needle)
                && memmem::find(&hay[at + 1..], &needle).is_none()
            {
                out.insert((sec, sym.value), (sym.size, lo + at as u32));
            }
        }
    }
}

/// A place a pointer of a table points into: main (None) or an overlay.
type Target = Option<&'static str>;

/// A table's words: zero, or a pointer into a section at some text.
type Signature = Vec<Option<(Target, Vec<u8>)>>;

/// Carry, into `out`, the uncarried globals made of data pointers
/// (Infection's word relocations, not into a function) and zeros: each
/// where the volume's section holds words that point into the same section
/// at the same bytes (up to a NUL, at most 64). Where several places match,
/// the one as far from the carried global below as on Infection. A
/// header's `static const char *` repeated in every translation unit is
/// one: the name entry's `STR_INFO_MSG_0`, which only its unnamed `__sinit`
/// builds, so no code votes for it.
fn by_pointers(v: Vol, out: &mut Out) {
    let inf = |ov: Target| ctx(Vol::Inf, ov).p;
    let dst = |ov: Target| ctx(v, ov).p;
    let base = inf(None);
    let mut section_of: HashMap<u16, Target> = HashMap::new();
    section_of.insert(base.main_index as u16, None);
    for &(n, _) in &OVERLAYS {
        section_of.insert(base.overlay_index[n] as u16, Some(n));
    }
    let dspan = |t: Target| {
        let (lo, hi, _) = xfer::span(&dst(t), t);
        (lo, hi)
    };
    // {(section, target): {text: word positions pointing at it}}.
    let mut index: HashMap<(Target, Target), HashMap<Vec<u8>, Vec<u32>>> = HashMap::new();
    let mut pointing = |ov: Target, target: Target, want: &[u8]| -> Vec<u32> {
        let found = index.entry((ov, target)).or_insert_with(|| {
            let (lo, hi) = dspan(ov);
            let (tlo, thi) = dspan(target);
            let (p, tp) = (dst(ov), dst(target));
            let words = p.read(lo, (hi - lo) as usize).unwrap_or_else(|e| crate::die(&e));
            let mut found: HashMap<Vec<u8>, Vec<u32>> = HashMap::new();
            let mut k = 0;
            while k + 3 < words.len() {
                let w = u32::from_le_bytes(words[k..k + 4].try_into().unwrap());
                if tlo <= w
                    && w < thi
                    && let Some(t) = text(&tp, w)
                    && !t.is_empty()
                {
                    found.entry(t).or_default().push(lo + k as u32);
                }
                k += 4;
            }
            found
        });
        found.get(want).cloned().unwrap_or_default()
    };

    for (ov, sec) in sections() {
        let p = inf(ov);
        let want_sec = section_index(&p, ov);
        let words32 = inf_relocs(ov);
        let mut found: Vec<(Symbol, Signature)> = Vec::new();
        'sym: for sym in &p.symbols {
            if sym.shndx as usize != want_sec
                || sym.kind != STT_OBJECT
                || sym.size < 4
                || !sym.size.is_multiple_of(4)
                || out.contains_key(&(sec, sym.value))
            {
                continue;
            }
            let mut sig: Signature = Vec::new();
            for a in (sym.value..sym.value + sym.size).step_by(4) {
                let Ok(w) = p.u32(a) else { continue 'sym };
                let Some(shndx) = words32.get(&a) else {
                    if w != 0 {
                        continue 'sym;
                    }
                    sig.push(None);
                    continue;
                };
                let Some(&target) = section_of.get(shndx) else { continue 'sym };
                let tp = inf(target);
                if tp.symbol_at(w, 0).is_some_and(|(s, _)| s.kind == STT_FUNC) {
                    continue 'sym;
                }
                let Some(t) = text(&tp, w) else { continue 'sym };
                sig.push(Some((target, t)));
            }
            let len: usize = sig.iter().flatten().map(|(_, t)| t.len()).sum();
            if sig.iter().any(Option::is_some) && len >= 8 {
                found.push((sym.clone(), sig));
            }
        }
        let mut repeats: HashMap<&Signature, usize> = HashMap::new();
        for (_, sig) in &found {
            *repeats.entry(sig).or_default() += 1;
        }
        let mut claimed: Vec<(u32, Vec<u32>)> = Vec::new();
        let dp = dst(ov);
        // Whether the volume's words at `start` are the signature's.
        let matches = |start: i64, sig: &Signature| -> bool {
            for (i, x) in sig.iter().enumerate() {
                let at = start + 4 * i as i64;
                let Some(w) = u32::try_from(at).ok().and_then(|a| dp.u32(a).ok()) else { return false };
                let ok = match x {
                    None => w == 0,
                    Some((t, want)) => {
                        let (tlo, thi) = dspan(*t);
                        tlo <= w && w < thi && text(&dst(*t), w).as_deref() == Some(want.as_slice())
                    }
                };
                if !ok {
                    return false;
                }
            }
            true
        };
        for (sym, sig) in &found {
            let (i0, (t0, s0)) = sig
                .iter()
                .enumerate()
                .find_map(|(i, x)| x.as_ref().filter(|x| !x.1.is_empty()).map(|x| (i, x)))
                .unwrap();
            let mut hits: Vec<i64> = Vec::new();
            for pos in pointing(ov, *t0, s0) {
                let start = i64::from(pos) - 4 * i0 as i64;
                if matches(start, sig) {
                    hits.push(start);
                }
            }
            if hits.is_empty() {
                hits = similar(ov, sig, &mut pointing, &dp, &dspan);
            }
            // Content Infection repeats, or the volume does: only the place
            // as far from the carried global below as on Infection.
            if hits.len() > 1 || repeats[sig] > 1 {
                let Some((&(_, i), &(_, v))) = out.range((sec, 0)..(sec, sym.value)).next_back() else { continue };
                hits.retain(|&h| h - i64::from(v) == i64::from(sym.value) - i64::from(i));
            }
            if hits.len() == 1 {
                let at = hits[0] as u32;
                out.insert((sec, sym.value), (sym.size, at));
                match claimed.iter_mut().find(|c| c.0 == at) {
                    Some(c) => c.1.push(sym.value),
                    None => claimed.push((at, vec![sym.value])),
                }
            }
        }
        // Two globals on one place: neither is sure.
        for (_, infs) in claimed {
            if infs.len() > 1 {
                for i in infs {
                    out.remove(&(sec, i));
                }
            }
        }
    }
}

/// A table the volume edited (Outbreak's story areas' notes): the place
/// most of its pointers' texts vote for, when at least a third of them
/// (and three) agree and no other place comes near; every word of it must
/// still be a pointer where Infection's is, or zero.
fn similar(
    ov: Target,
    sig: &Signature,
    pointing: &mut impl FnMut(Target, Target, &[u8]) -> Vec<u32>,
    dp: &Program,
    dspan: &impl Fn(Target) -> (u32, u32),
) -> Vec<i64> {
    let ptrs: Vec<(usize, &(Target, Vec<u8>))> =
        sig.iter().enumerate().filter_map(|(i, x)| x.as_ref().filter(|x| !x.1.is_empty()).map(|x| (i, x))).collect();
    if ptrs.len() < 6 {
        return Vec::new();
    }
    // Counted in the order first seen, as Python's Counter keeps them.
    let mut tally: Vec<(i64, usize)> = Vec::new();
    let mut at: HashMap<i64, usize> = HashMap::new();
    for &(i, (t, want)) in &ptrs {
        for pos in pointing(ov, *t, want) {
            let k = i64::from(pos) - 4 * i as i64;
            match at.get(&k) {
                Some(&n) => tally[n].1 += 1,
                None => {
                    at.insert(k, tally.len());
                    tally.push((k, 1));
                }
            }
        }
    }
    if tally.is_empty() {
        return Vec::new();
    }
    // most_common(2): by count, the first seen of equals first.
    let mut ranked = tally.clone();
    ranked.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    let (best, n) = ranked[0];
    let runner = ranked.get(1).map_or(0, |r| r.1);
    if n < 3.max(ptrs.len() / 3) || runner * 2 > n {
        return Vec::new();
    }
    for (i, x) in sig.iter().enumerate() {
        let Some(w) = u32::try_from(best + 4 * i as i64).ok().and_then(|a| dp.u32(a).ok()) else { return Vec::new() };
        if let Some((t, _)) = x
            && w != 0
        {
            let (tlo, thi) = dspan(*t);
            if !(tlo <= w && w < thi) {
                return Vec::new();
            }
        }
    }
    vec![best]
}

/// Carry, into `out`, the globals a changed function builds, by order.
/// For each pair of same-named functions: the data addresses each builds,
/// in instruction order. Where the two lists are as long and every address
/// the carry already holds sits at its carried place (at least one), an
/// uncarried global there carries to the volume's address in the same
/// place; unanimous votes only. A compiler label (`@123`) the name pass
/// carried counts as neither: the code outranks it, since each compile
/// numbers its labels anew. A global with no pointer in it (Infection's
/// relocations) moves only where the volume's bytes are Infection's: the
/// tracker can pair a `lui` with the wrong low half. Mutation's
/// `ConvergenceSystem` grew, so no alignment reaches its `@3328`, and the
/// name `@3329` sits on it there.
fn by_sequence(v: Vol, out: &mut Out) {
    // {((section, address), size): {the volume's address: votes}}, in the
    // order first voted.
    let mut order: Vec<((u32, u32), u32)> = Vec::new();
    let mut votes: HashMap<((u32, u32), u32), HashMap<u32, u32>> = HashMap::new();
    for (ov, sec) in sections() {
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let relocated = inf_relocs(ov);
        // None for a global with pointers in it, else whether the volume's
        // bytes at `va` are Infection's.
        let bytes_agree = |sym: &Symbol, va: u32| -> Option<bool> {
            if (sym.value..sym.value + sym.size.max(1)).step_by(4).any(|a| relocated.contains_key(&a)) {
                return None;
            }
            Some(match (src.p.read(sym.value, sym.size as usize), dst.p.read(va, sym.size as usize)) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            })
        };
        let (inf, mine) = (section_symbols(&src.p, ov), section_symbols(&dst.p, ov));
        let built = |words: &[u32], gp: Option<u32>| -> Vec<u32> {
            let a = xfer::addresses(words, gp);
            let mut idx: Vec<usize> = a.keys().copied().filter(|&i| words[i] >> 26 != 0x0F).collect();
            idx.sort();
            idx.into_iter().map(|i| a[&i]).collect()
        };
        for (name, f) in inf.iter() {
            let Some(d) = mine.get(name) else { continue };
            if f.kind != STT_FUNC {
                continue;
            }
            let (Some(sw), Some(dw)) = (src.run(f.value, f.size / 4), dst.run(d.value, d.size / 4)) else { continue };
            let (sa, da) = (built(sw, src.p.gp), built(dw, dst.p.gp));
            if sa.len() != da.len() || sa.is_empty() {
                continue;
            }
            let (mut anchors, mut want, mut ok) = (0, Vec::new(), true);
            for (&a, &b) in sa.iter().zip(&da) {
                if a & 0xFFFF == 0 && b & 0xFFFF == 0 {
                    continue;
                }
                let Some((sym, off)) = src.p.symbol_at(a, 0) else { continue };
                if sym.kind != STT_OBJECT {
                    continue;
                }
                let key = (if sym.value >= OVERLAY_SPACE { sec } else { 0 }, sym.value);
                let carried = out.get(&key).map(|x| u64::from(x.1) + u64::from(off));
                if carried.is_some() && !sym.name.starts_with('@') {
                    if carried != Some(u64::from(b)) {
                        ok = false;
                        break;
                    }
                    anchors += 1;
                } else if carried != Some(u64::from(b)) {
                    let agree = bytes_agree(sym, b.wrapping_sub(off));
                    if agree == Some(false) || (agree.is_none() && carried.is_some()) {
                        continue;
                    }
                    want.push((key, sym.size, b.wrapping_sub(off)));
                }
            }
            if ok && anchors >= 1 {
                for (key, size, va) in want {
                    let k = (key, size);
                    if !votes.contains_key(&k) {
                        order.push(k);
                    }
                    *votes.entry(k).or_default().entry(va).or_default() += 1;
                }
            }
        }
    }
    for k in order {
        let counter = &votes[&k];
        if counter.len() == 1 {
            let va = *counter.keys().next().unwrap();
            let (key, size) = k;
            if let Some(&(_, at)) = out.get(&key)
                && at != va
            {
                eprintln!(
                    "{}: a label at {:08x} named at {at:08x}, built at {va:08x} in order; the code wins",
                    v.tag(),
                    key.1
                );
            }
            out.insert(key, (size, va));
        }
    }
}

fn path(v: Vol) -> std::path::PathBuf {
    root().join("work/analysis/carry").join(format!("{}.json", v.module()))
}

/// The rows as the cache holds them: a JSON list of four-number lists.
fn json(rows: &[(u32, u32, u32, u32)]) -> String {
    let rows: Vec<String> = rows.iter().map(|r| format!("[{}, {}, {}, {}]", r.0, r.1, r.2, r.3)).collect();
    format!("[{}]", rows.join(", "))
}

/// Make the volume's carry and write it (or with `check`, say whether the
/// cache holds it).
pub fn write(v: Vol, check: bool) -> bool {
    let text = json(&carry(v));
    let p = path(v);
    if check {
        let same = std::fs::read_to_string(&p).is_ok_and(|t| t == text);
        println!("{} {}", p.display(), if same { "is current" } else { "differs" });
        return same;
    }
    std::fs::create_dir_all(p.parent().unwrap()).unwrap_or_else(|e| crate::die(&e.to_string()));
    std::fs::write(&p, text).unwrap_or_else(|e| crate::die(&format!("{}: {e}", p.display())));
    println!("{} -> {}", v.tag(), p.display());
    true
}

/// A carry's rows: (section, Infection's address, its size, the volume's
/// address).
type Rows = Vec<(u32, u32, u32, u32)>;

thread_local! {
    /// The carries made this run from a named disc, by volume.
    static MADE: RefCell<HashMap<Vol, Rc<Rows>>> = RefCell::new(HashMap::new());
}

/// The volume's carry: made in memory when a disc is named for it
/// ([`crate::source::use_disc`]), else from the cache in `work/`, made
/// when missing.
pub fn cached(v: Vol) -> Rows {
    if crate::source::disc(v).is_some() {
        if let Some(c) = MADE.with(|m| m.borrow().get(&v).cloned()) {
            return (*c).clone();
        }
        let c = Rc::new(carry(v));
        MADE.with(|m| m.borrow_mut().insert(v, c.clone()));
        return (*c).clone();
    }
    let p = path(v);
    if !p.exists() {
        write(v, false);
    }
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| crate::die(&format!("{}: {e}", p.display())));
    let raw: Vec<[u64; 4]> =
        serde_json::from_str(&text).unwrap_or_else(|e| crate::die(&format!("{}: {e}", p.display())));
    raw.into_iter().map(|r| (r[0] as u32, r[1] as u32, r[2] as u32, r[3] as u32)).collect()
}
