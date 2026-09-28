//! Infection's symbols carried to a stripped later volume: the `<elf>.syms`
//! sidecar the program image loads for Mutation, Outbreak and Quarantine
//! (`tools/xfer.py` wrote it until 2026-09-27). The engine is shared, so most
//! functions exist in all four volumes with the same instructions at other
//! addresses.
//!
//! The passes, main and each overlay against their namesakes:
//!
//! 1. exact, call, data and order (`xfer::transfer`): functions whose
//!    masked bodies match, their callees, the globals their code builds,
//!    and the functions between paired neighbours by the shape of their
//!    code;
//! 2. cross: the globals a paired body builds, by the order of the
//!    addresses it builds, between two it agrees on;
//! 3. code: the data pass's globals moved where paired bodies build them;
//! 4. content: globals found by what they hold;
//! 5. pointer: globals reached through a pointer in a carried global;
//! 6. layout: globals between two that sit alike in both volumes;
//! 7. content again, then ref: functions named by the globals they build;
//! 8. prefix: functions whose opening words match one place only;
//! 9. pointer again;
//! 10. ctor: the overlays' static initialisers, by their constructor lists.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::elf::{STT_FUNC, STT_OBJECT, Symbol};
use crate::program::Program;
use crate::volume::{Vol, ctx, elf_path};
use crate::xfer::{Ordered, Row, SECTIONS, Votes, addresses, normalise, side, span, transfer, unanimous};

/// A section's name as the rows keep it.
fn sec_name(s: &str) -> &'static str {
    match s {
        "main" => "main",
        "gcmn" => "gcmn",
        "demo" => "demo",
        "desktop" => "desktop",
        "toppage" => "toppage",
        other => crate::die(&format!("unknown section {other}")),
    }
}

/// The overlay a section loads: none for main.
fn overlay(sec: &str) -> Option<&'static str> {
    (sec != "main").then(|| sec_name(sec))
}

fn prog(v: Vol, sec: &str) -> Rc<Program> {
    ctx(v, overlay(sec)).p
}

/// The section a section header names: an overlay's, else main.
fn named_section(name: &str) -> &'static str {
    name.strip_suffix(".prg").map_or("main", sec_name)
}

/// A NUL-terminated run of text bytes (ASCII or Shift-JIS).
fn printable(bs: &[u8]) -> bool {
    match bs.iter().position(|&c| c == 0) {
        Some(end) if end > 0 => bs[..end].iter().all(|&c| c >= 0x20 || matches!(c, 9 | 10 | 13)),
        _ => false,
    }
}

/// Where each NUL-terminated string of `data` (at `lo`) starts, by text.
fn strings_in(data: &[u8], lo: u32, out: &mut HashMap<Vec<u8>, Vec<u32>>) {
    let mut start = 0;
    for (k, &b) in data.iter().enumerate() {
        if b == 0 {
            if k > start {
                out.entry(data[start..k].to_vec()).or_default().push(lo + start as u32);
            }
            start = k + 1;
        }
    }
}

/// Globals a paired function builds that the data pass missed, by the
/// order of the addresses it builds. Each body's address-building
/// instructions, in order, are matched between two anchors (an address
/// whose global both volumes name, at the same offset); a gap holding as
/// many addresses in both pairs them in order. This names what a body
/// edited around it hides from the aligned runs, and the overlays' globals
/// main's functions build (`wavPlay`'s gcmn `bgmWavTbl`, which Mutation
/// moved into main), the overlay told by Infection's relocation.
fn cross(v: Vol, rows: &[Row]) -> Vec<Row> {
    let mut named: HashMap<&str, u32> = HashMap::new();
    for r in rows.iter().filter(|r| r.kind == "OBJECT") {
        named.entry(&r.name).or_insert(r.va);
    }
    let have: HashSet<(&str, &str)> = rows.iter().map(|r| (r.sec, r.name.as_str())).collect();
    let mut votes: Votes<(String, u32, &'static str), i64> = Default::default();
    for ov in SECTIONS {
        let sec = ov.unwrap_or("main");
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let mut fns: HashMap<&str, (u32, u32)> = HashMap::new();
        for r in rows.iter().filter(|r| r.sec == sec && r.kind == "FUNC") {
            fns.insert(&r.name, (r.va, r.size));
        }
        let mut targets: HashMap<u32, (Symbol, &'static str)> = HashMap::new();
        for r in src.p.relocs() {
            if !matches!(r.kind, 6 | 7) || !src.owns(r.offset) {
                continue;
            }
            let Some(mut sym) = src.p.elf.symbols.get(r.symbol) else { continue };
            let tsec = named_section(&src.p.section_name(sym.shndx));
            if sym.kind != STT_OBJECT || sym.size == 0 {
                // A local global's relocation names its section: the global
                // is the one at the address built.
                let Some(addr) = r.addr else { continue };
                if tsec != "main" && tsec != sec {
                    continue;
                }
                match src.p.symbol_at(addr, 0) {
                    Some((hit, _)) if hit.kind == STT_OBJECT => sym = hit,
                    _ => continue,
                }
            }
            if !sym.name.starts_with('@') {
                targets.insert(r.offset, (sym.clone(), tsec));
            }
        }
        for f in src.functions() {
            let Some(&(dva, dsize)) = fns.get(f.name.as_str()) else { continue };
            let (Some(sw), Some(dw)) = (src.run(f.value, f.size / 4), dst.run(dva, 1.max(dsize / 4))) else { continue };
            let (sa, da) = (addresses(sw, src.p.gp), addresses(dw, dst.p.gp));
            let mut si: Vec<usize> = sa.keys().copied().collect();
            si.sort();
            let s: Vec<(u32, Option<&(Symbol, &'static str)>)> =
                si.iter().map(|&i| (sa[&i], targets.get(&(f.value + 4 * i as u32)))).collect();
            let mut dj: Vec<usize> = da.keys().copied().collect();
            dj.sort();
            let d: Vec<u32> = dj.iter().map(|j| da[j]).collect();
            let expect = |k: usize| -> Option<i64> {
                let (addr, t) = s[k];
                let (sym, _) = t?;
                let base = *named.get(sym.name.as_str())?;
                Some(i64::from(base) + i64::from(addr) - i64::from(sym.value))
            };
            // Anchors: known addresses found in order in the other body.
            let mut anchors: Vec<(i64, i64)> = vec![(-1, -1)];
            let mut j0 = 0;
            for k in 0..s.len() {
                let Some(e) = expect(k) else { continue };
                let Some(j) = d[j0..].iter().position(|&x| i64::from(x) == e) else { continue };
                anchors.push((k as i64, (j0 + j) as i64));
                j0 += j + 1;
            }
            anchors.push((s.len() as i64, d.len() as i64));
            for w in anchors.windows(2) {
                let ((k1, j1), (k2, j2)) = (w[0], w[1]);
                if k2 - k1 - 1 <= 0 || k2 - k1 != j2 - j1 {
                    continue;
                }
                for (k, j) in ((k1 + 1)..k2).zip((j1 + 1)..j2) {
                    let (addr, t) = s[k as usize];
                    let Some((sym, tsec)) = t else { continue };
                    if named.contains_key(sym.name.as_str()) {
                        continue;
                    }
                    let va = i64::from(d[j as usize]) - (i64::from(addr) - i64::from(sym.value));
                    *votes.entry((sym.name.clone(), sym.size, *tsec)).entry(va).or_default() += 1;
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut taken: HashSet<i64> = rows.iter().map(|r| i64::from(r.va)).collect();
    let main = ctx(v, None).p.main_seg;
    for ((name, size, tsec), va) in unanimous(&votes) {
        if have.contains(&(tsec, name.as_str())) || named.contains_key(name.as_str()) || taken.contains(&va) {
            continue;
        }
        // Which section it is in, in the other volume.
        if i64::from(main.vaddr) <= va && va < i64::from(main.vaddr + main.memsz.max(main.filesz)) {
            out.push(Row { sec: "main", va: va as u32, size, kind: "OBJECT", name, how: "data" });
            taken.insert(va);
            continue;
        }
        if tsec == "main" {
            continue;
        }
        let dp = ctx(v, Some(tsec)).p;
        if let Some(o) = &dp.overlay
            && i64::from(o.text + o.text_size) <= va
            && va < i64::from(dp.overlay_space.1)
        {
            out.push(Row { sec: tsec, va: va as u32, size, kind: "OBJECT", name, how: "data" });
            taken.insert(va);
        }
    }
    out
}

/// A word of a global as the content pass tells it: a pointer to a
/// string, another pointer, or a number.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Pat {
    Str(Vec<u8>),
    Ptr,
    Word(u32),
}

/// The string at `va` when it reads as text.
fn text_at(p: &Program, va: u32) -> Option<Vec<u8>> {
    let b = p.read(va, 256).ok()?;
    let end = b.iter().position(|&c| c == 0)?;
    (end > 0 && printable(&b[..end + 1])).then(|| b[..end].to_vec())
}

/// Globals found by what they hold. A global of Infection's not named yet,
/// whose words are its own numbers and pointers to strings, is looked for
/// in the other volume's data of the same section: every number equal,
/// every pointer to a string pointing at the same string, every other
/// pointer a pointer. One place only, and at least one string or two
/// non-zero numbers to tell it by. This names the tables a recompiled body
/// no longer shows (Outbreak's and Quarantine's `EA_MODELTABLE`s).
fn content(v: Vol, rows: &[Row]) -> Vec<Row> {
    let mut have: HashSet<(&str, String)> = rows.iter().map(|r| (r.sec, r.name.clone())).collect();
    let mut taken: HashSet<(&str, u32)> = rows.iter().map(|r| (r.sec, r.va)).collect();
    let mut out = Vec::new();
    for ov in SECTIONS {
        let sec = ov.unwrap_or("main");
        let (sp, dp) = (ctx(Vol::Inf, ov).p, ctx(v, ov).p);
        let (s_lo, s_hi, d_lo, d_hi) = match (&sp.overlay, &dp.overlay) {
            (Some(so), Some(d_o)) => (
                so.text + so.text_size,
                so.text + so.text_size + so.data_size,
                d_o.text + d_o.text_size,
                d_o.text + d_o.text_size + d_o.data_size,
            ),
            _ => {
                let s_lo = sp
                    .elf
                    .symbols
                    .iter()
                    .filter(|x| x.kind == STT_OBJECT && x.size != 0 && sp.section_name(x.shndx) == "main")
                    .map(|x| x.value)
                    .min()
                    .unwrap();
                let d_lo =
                    rows.iter().filter(|r| r.sec == "main" && r.kind == "OBJECT").map(|r| r.va).min().unwrap_or(s_lo);
                (s_lo, sp.main_seg.vaddr + sp.main_seg.filesz, d_lo, dp.main_seg.vaddr + dp.main_seg.filesz)
            }
        };
        let mut pointer_at: HashMap<u32, u32> = HashMap::new();
        for r in sp.relocs() {
            if let (2, Some(a)) = (r.kind, r.addr)
                && a != 0
                && s_lo <= r.offset
                && r.offset < s_hi
            {
                pointer_at.insert(r.offset, a);
            }
        }
        let raw = dp.read(d_lo, (d_hi - d_lo) as usize).unwrap_or_else(|e| crate::die(&e));
        let words: Vec<u32> = raw.as_chunks::<4>().0.iter().map(|&c| u32::from_le_bytes(c)).collect();
        let mut by_value: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, &w) in words.iter().enumerate() {
            if w != 0 {
                by_value.entry(w).or_default().push(i);
            }
        }
        // Where each string starts in the other volume (main's strings too).
        let mut strings: HashMap<Vec<u8>, Vec<u32>> = HashMap::new();
        let main = (dp.main_seg.vaddr, dp.main_seg.vaddr + dp.main_seg.filesz);
        for (lo, hi) in [(d_lo, d_hi), main] {
            let data = dp.read(lo, (hi - lo) as usize).unwrap_or_else(|e| crate::die(&e));
            strings_in(&data, lo, &mut strings);
        }
        let want_sec = ov.map_or("main".to_string(), |o| format!("{o}.prg"));
        let mut patterns: Vec<(&Symbol, Vec<Pat>, u32)> = Vec::new();
        for sym in &sp.elf.symbols {
            if sym.kind != STT_OBJECT || sym.size < 8 || sym.name.starts_with('@') {
                continue;
            }
            if sp.section_name(sym.shndx) != want_sec || !(s_lo <= sym.value && sym.value + sym.size <= s_hi) {
                continue;
            }
            let (mut pattern, mut info) = (Vec::new(), 0);
            for off in (0..sym.size - 3).step_by(4) {
                let va = sym.value + off;
                if let Some(&a) = pointer_at.get(&va) {
                    match text_at(&sp, a) {
                        Some(t) => {
                            pattern.push(Pat::Str(t));
                            info += 2;
                        }
                        None => pattern.push(Pat::Ptr),
                    }
                } else {
                    let w = sp.u32(va).unwrap_or_else(|e| crate::die(&e));
                    pattern.push(Pat::Word(w));
                    info += u32::from(w != 0);
                }
            }
            patterns.push((sym, pattern, info));
        }
        // Two of Infection's globals that hold the same (pcMsg7_04 and
        // pcMsg7_24, enemyList36 and enemyList46) cannot tell which is which.
        let mut twins: HashMap<&[Pat], usize> = HashMap::new();
        for (_, p, _) in &patterns {
            *twins.entry(p).or_default() += 1;
        }
        for (sym, pattern, info) in &patterns {
            if *info < 2 || have.contains(&(sec, sym.name.clone())) || twins[pattern.as_slice()] > 1 {
                continue;
            }
            // Candidates from the first element that narrows them.
            let mut cands: Vec<i64> = Vec::new();
            for (k, p) in pattern.iter().enumerate() {
                match p {
                    Pat::Word(w) if *w != 0 => {
                        cands =
                            by_value.get(w).map_or(Vec::new(), |is| is.iter().map(|&i| i as i64 - k as i64).collect());
                        break;
                    }
                    Pat::Str(t) => {
                        for &a in strings.get(t).map_or(&[][..], |x| x) {
                            for &i in by_value.get(&a).map_or(&[][..], |x| x) {
                                cands.push(i as i64 - k as i64);
                            }
                        }
                        break;
                    }
                    _ => {}
                }
            }
            let mut found = Vec::new();
            for c in cands {
                if c < 0 || c as usize + pattern.len() > words.len() {
                    continue;
                }
                let ok = pattern.iter().enumerate().all(|(k, p)| {
                    let w = words[c as usize + k];
                    match p {
                        Pat::Word(x) => w == *x,
                        Pat::Str(t) => w != 0 && text_at(&dp, w).as_ref() == Some(t),
                        Pat::Ptr => w == 0 || dp.mapped(w),
                    }
                });
                if ok {
                    found.push(d_lo + 4 * c as u32);
                    if found.len() > 1 {
                        break;
                    }
                }
            }
            if found.len() == 1 && !taken.contains(&(sec, found[0])) {
                out.push(Row {
                    sec,
                    va: found[0],
                    size: sym.size,
                    kind: "OBJECT",
                    name: sym.name.clone(),
                    how: "content",
                });
                taken.insert((sec, found[0]));
                have.insert((sec, sym.name.clone()));
            }
        }
    }
    out
}

/// Functions named by the globals they build. Where exactly one of
/// Infection's functions builds a global's address, and the other volume's
/// code builds that global's address in one function only (its start the
/// nearest likely entry below), that function is Infection's, when every
/// such global agrees, and the start is a function's own (its code never
/// branches below it). This names the recompiled constructors the order
/// pass could not (Outbreak's `EVENTAREA` and `ROOTTOWN`s, found through
/// their tables).
fn refs(v: Vol, rows: &[Row]) -> Vec<Row> {
    let mut out = Vec::new();
    for ov in SECTIONS {
        let sec = ov.unwrap_or("main");
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let mut named_fn: HashSet<u32> =
            rows.iter().filter(|r| r.sec == sec && r.kind == "FUNC").map(|r| r.va).collect();
        let taken_fn: HashSet<&str> =
            rows.iter().filter(|r| r.sec == sec && r.kind == "FUNC").map(|r| r.name.as_str()).collect();
        let mut objects: HashMap<u32, String> = HashMap::new();
        for r in rows.iter().filter(|r| r.kind == "OBJECT" && (r.sec == sec || r.sec == "main")) {
            objects.insert(r.va, r.name.clone());
        }
        // Infection: which functions build each global, and each string
        // literal (by its text).
        let mut users: HashMap<String, HashSet<String>> = HashMap::new();
        let mut literal: Ordered<String, Vec<u8>> = Default::default();
        for r in src.p.relocs() {
            let Some(sym) = src.p.elf.symbols.get(r.symbol) else { continue };
            if !matches!(r.kind, 6 | 7) || sym.kind != STT_OBJECT || !src.owns(r.offset) {
                continue;
            }
            let Some(f) = src.p.function_at(r.offset) else { continue };
            users.entry(sym.name.clone()).or_default().insert(f.name.clone());
            if sym.name.starts_with('@') && !literal.contains(&sym.name) {
                let Ok(b) = src.p.read(sym.value, sym.size.min(256) as usize) else { continue };
                if let Some(end) = b.iter().position(|&c| c == 0)
                    && end > 2
                    && printable(&b[..end + 1])
                {
                    literal.insert(sym.name.clone(), b[..end].to_vec());
                }
            }
        }
        // The literals found once in the other volume's data (this section's
        // and main's), by text.
        let mut places: HashMap<Vec<u8>, Vec<u32>> = HashMap::new();
        let main = (dst.p.main_seg.vaddr, dst.p.main_seg.vaddr + dst.p.main_seg.filesz);
        for (lo, hi) in [(dst.code_hi, dst.hi), main] {
            if hi <= lo {
                continue;
            }
            let data = dst.p.read(lo, (hi - lo) as usize).unwrap_or_else(|e| crate::die(&e));
            strings_in(&data, lo, &mut places);
        }
        for (name, text) in literal.iter() {
            if let Some(at) = places.get(text)
                && at.len() == 1
                && !objects.contains_key(&at[0])
            {
                objects.insert(at[0], name.clone());
            }
        }
        // The other volume: which likely function builds each named global.
        let starts = dst.entries();
        let n = ((dst.code_hi - dst.lo) >> 2) as usize;
        let built = addresses(&dst.words[..n], dst.p.gp);
        let mut idx: Vec<usize> = built.keys().copied().collect();
        idx.sort();
        let mut found: Ordered<String, HashSet<u32>> = Default::default();
        for i in idx {
            let Some(name) = objects.get(&built[&i]) else { continue };
            let k = starts.partition_point(|&s| s <= dst.lo + 4 * i as u32);
            if k > 0 {
                found.entry(name.clone()).insert(starts[k - 1]);
            }
        }
        // Leave out, on both sides, the users already named in the other
        // volume; one left on each is a pair.
        let named_at: HashMap<&str, u32> =
            rows.iter().filter(|r| r.sec == sec && r.kind == "FUNC").map(|r| (r.name.as_str(), r.va)).collect();
        let mut votes: Votes<String, u32> = Default::default();
        for (name, fs) in found.iter() {
            let Some(u) = users.get(name).filter(|u| !u.is_empty()) else { continue };
            let known: HashSet<u32> = u.iter().filter_map(|f| named_at.get(f.as_str()).copied()).collect();
            let u: Vec<&String> = u.iter().filter(|f| !named_at.contains_key(f.as_str())).collect();
            let fs: Vec<u32> = fs.iter().copied().filter(|va| !known.contains(va) && !named_fn.contains(va)).collect();
            if u.len() == 1 && fs.len() == 1 {
                *votes.entry(u[0].clone()).entry(fs[0]).or_default() += 1;
            }
        }
        let by_name: HashSet<&str> = src.p.functions().filter(|f| src.owns(f.value)).map(|f| f.name.as_str()).collect();
        let mut claimed: HashMap<u32, u32> = HashMap::new();
        for c in votes.values() {
            for &va in c.keys() {
                *claimed.entry(va).or_default() += 1;
            }
        }
        for (fname, va) in unanimous(&votes) {
            if taken_fn.contains(fname.as_str())
                || named_fn.contains(&va)
                || claimed[&va] > 1
                || !by_name.contains(fname.as_str())
            {
                continue;
            }
            let i = starts.partition_point(|&s| s <= va);
            let end = starts.get(i).copied().unwrap_or(dst.code_hi);
            // A start inside a larger function (Mutation's `ccEvent::Execute`
            // returns mid-switch) is no function of its own.
            if dst.branches_back(va, end) {
                continue;
            }
            out.push(Row { sec, va, size: end - va, kind: "FUNC", name: fname, how: "ref" });
            named_fn.insert(va);
        }
    }
    out
}

/// Functions changed further on but opening alike: an Infection function
/// not yet named whose first [`PREFIX`] words (addresses and `$gp` offsets
/// masked) no other Infection function shares, and which open exactly one
/// likely start in the other volume (a thread's too, after an endless
/// loop), not named yet and a function's own (see
/// [`crate::xfer::Side::branches_back`]). Mutation's `ccThEvHold` changed
/// after its opening.
fn prefixes(v: Vol, rows: &[Row]) -> Vec<Row> {
    let mut out = Vec::new();
    for ov in SECTIONS {
        let sec = ov.unwrap_or("main");
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let funcs = || rows.iter().filter(|r| r.sec == sec && r.kind == "FUNC");
        let named: HashSet<&str> = funcs().map(|r| r.name.as_str()).collect();
        let named_fn: HashSet<u32> = funcs().map(|r| r.va).collect();
        let mut starts = dst.entries();
        starts.extend(dst.loop_ends());
        starts.sort_unstable();
        starts.dedup();
        let mut index: HashMap<Vec<u32>, Vec<u32>> = HashMap::new();
        for &va in &starts {
            if let Some(run) = dst.run(va, PREFIX) {
                index.entry(normalise(run)).or_default().push(va);
            }
        }
        let mut opening: Vec<(&Symbol, Vec<u32>)> = Vec::new();
        let mut shared: HashMap<Vec<u32>, usize> = HashMap::new();
        for f in src.functions().filter(|f| f.size >= 4 * PREFIX) {
            let Some(run) = src.run(f.value, PREFIX) else { continue };
            let o = normalise(run);
            *shared.entry(o.clone()).or_default() += 1;
            opening.push((f, o));
        }
        let mut found: Vec<(String, u32)> = Vec::new();
        for (f, o) in &opening {
            if named.contains(f.name.as_str()) || shared[o] != 1 {
                continue;
            }
            if let Some([va]) = index.get(o).map(|v| v.as_slice())
                && !named_fn.contains(va)
            {
                found.push((f.name.clone(), *va));
            }
        }
        for (name, va) in found {
            let i = starts.partition_point(|&s| s <= va);
            let end = starts.get(i).copied().unwrap_or(dst.code_hi);
            if dst.branches_back(va, end) {
                continue;
            }
            out.push(Row { sec, va, size: end - va, kind: "FUNC", name, how: "prefix" });
        }
    }
    out
}

/// How many opening words [`prefixes`] compares.
const PREFIX: u32 = 24;

/// Where an address falls: main's data (from `floor`), an overlay's code
/// or an overlay's data.
fn region(p: &Program, va: u32, floor: u32) -> Option<&'static str> {
    let m = &p.main_seg;
    if floor <= va && va < m.vaddr + m.memsz.max(m.filesz) {
        return Some("main");
    }
    let o = p.overlay.as_ref()?;
    if o.text <= va && va < o.text + o.text_size {
        return Some("code");
    }
    (o.text + o.text_size <= va && va < p.overlay_space.1).then_some("data")
}

type Key = (&'static str, String);

/// The carried globals: {(section, name): address} and how each was found,
/// in the order the rows first name them.
fn objects(rows: &[Row]) -> (Ordered<Key, u32>, HashMap<Key, &'static str>) {
    let (mut named, mut how): (Ordered<Key, u32>, HashMap<Key, &'static str>) = Default::default();
    for r in rows.iter().filter(|r| r.kind == "OBJECT") {
        named.insert((r.sec, r.name.clone()), r.va);
        how.insert((r.sec, r.name.clone()), r.how);
    }
    (named, how)
}

/// Globals named through the pointers in carried globals.
///
/// Names are per section (main, or an overlay). A pointer in main may name
/// an overlay's global (main's `voiceData` points at gcmn's rows): its
/// relocation's symbol says which overlay. A carried global votes only when
/// its words in the other volume keep Infection's layout: a pointer in the
/// other volume's data where Infection has a relocation, and no pointer
/// where it has none (95% of each). A candidate must fall in the same
/// region as Infection's target (main's data, an overlay's data), and where
/// the target is text, be text. A unanimous vote also corrects a global the
/// data or pointer pass placed elsewhere.
fn pointers(v: Vol, rows: Vec<Row>) -> (Vec<Row>, usize, usize) {
    const ROUNDS: usize = 6;
    let (mut named, mut how) = objects(&rows);
    let mut at: HashMap<(&'static str, u32), String> = HashMap::new();
    for ((sec, name), &va) in named.iter() {
        at.insert((sec, va), name.clone());
    }
    let dst_floor = named.iter().filter(|(k, _)| k.0 == "main").map(|(_, &va)| va).min().unwrap_or(0);
    let src_main = ctx(Vol::Inf, None).p;
    let src_floor = src_main
        .elf
        .symbols
        .iter()
        .filter(|s| {
            s.kind == STT_OBJECT
                && s.size != 0
                && src_main.main_seg.vaddr <= s.value
                && s.value < src_main.overlay_space.0
        })
        .map(|s| s.value)
        .min()
        .unwrap();
    let mut sec_objects: HashMap<&'static str, HashMap<u32, Symbol>> = HashMap::new();
    for sym in &src_main.elf.symbols {
        if sym.kind == STT_OBJECT && sym.size != 0 && !sym.name.starts_with('@') {
            let name = src_main.section_name(sym.shndx);
            let sec = match name.strip_suffix(".prg") {
                Some(o) => Some(sec_name(o)),
                None => (name == "main").then_some("main"),
            };
            if let Some(sec) = sec {
                sec_objects.entry(sec).or_default().insert(sym.value, sym.clone());
            }
        }
    }
    // (section of the pointer, its offset, target section, target symbol)
    let mut refs: Vec<(&'static str, u32, &'static str, Symbol)> = Vec::new();
    // Every pointer word, whatever it points at.
    let mut relocated: HashSet<(&'static str, u32)> = HashSet::new();
    for ov in SECTIONS {
        let sp = ctx(Vol::Inf, ov).p;
        let sec = ov.unwrap_or("main");
        let (lo, hi, _) = span(&sp, ov);
        for r in sp.relocs() {
            let Some(addr) = r.addr.filter(|&a| a != 0) else { continue };
            if r.kind != 2 || !(lo <= r.offset && r.offset < hi) {
                continue;
            }
            relocated.insert((sec, r.offset));
            let tname = sp.elf.symbols.get(r.symbol).map_or(String::new(), |s| sp.section_name(s.shndx));
            let tsec = match tname.strip_suffix(".prg") {
                Some(o) => sec_name(o),
                None if sp.main_seg.vaddr <= addr && addr < sp.overlay_space.0 => "main",
                None => sec,
            };
            if let Some(target) = sec_objects.get(tsec).and_then(|m| m.get(&addr)) {
                refs.push((sec, r.offset, tsec, target.clone()));
            }
        }
    }
    let mut layout_ok: HashMap<(&'static str, String), bool> = HashMap::new();
    let mut keeps_layout = |sec: &'static str, o: &Symbol, dva: u32| -> bool {
        if let Some(&ok) = layout_ok.get(&(sec, o.name.clone())) {
            return ok;
        }
        let (sp, dp) = (prog(Vol::Inf, sec), prog(v, sec));
        let (mut p_all, mut p_ok, mut n_all, mut n_ok) = (0u32, 0u32, 0u32, 0u32);
        let others: Vec<Rc<Program>> =
            if sec == "main" { SECTIONS.iter().map(|&x| ctx(v, x).p).collect() } else { vec![dp.clone()] };
        for off in (0..o.size.saturating_sub(3)).step_by(4) {
            if !dp.mapped(dva + off) {
                break;
            }
            let w = dp.u32(dva + off).unwrap_or(0);
            if relocated.contains(&(sec, o.value + off)) {
                p_all += 1;
                p_ok += u32::from(w == 0 || others.iter().any(|x| region(x, w, dst_floor).is_some()));
            } else {
                n_all += 1;
                let sw = sp.u32(o.value + off).unwrap_or_else(|e| crate::die(&e));
                n_ok += u32::from(w == sw || region(&dp, w, dst_floor).is_none());
            }
        }
        let ok = p_all > 0 && f64::from(p_ok) >= 0.95 * f64::from(p_all) && f64::from(n_ok) >= 0.95 * f64::from(n_all);
        layout_ok.insert((sec, o.name.clone()), ok);
        ok
    };
    let mut added: Ordered<Key, (u32, u32)> = Default::default();
    let mut dropped: HashSet<(&'static str, String, u32)> = HashSet::new();
    let mut moved = 0;
    for _ in 0..ROUNDS {
        let mut votes: Votes<(String, u32, &'static str), u32> = Default::default();
        for (sec, off_va, tsec, target) in &refs {
            let sp = prog(Vol::Inf, sec);
            let Some((o, off)) = sp.symbol_at(*off_va, 0) else { continue };
            let Some(&dva) = named.get(&(*sec, o.name.clone())) else { continue };
            if o.kind != STT_OBJECT || !keeps_layout(sec, o, dva) {
                continue;
            }
            let dp = prog(v, sec);
            if !dp.mapped(dva + off) {
                continue;
            }
            let cand = dp.u32(dva + off).unwrap_or(0);
            let (tsp, tdp) = (prog(Vol::Inf, tsec), prog(v, tsec));
            let treg = region(&tsp, target.value, src_floor);
            if cand == 0 || treg.is_none() || region(&tdp, cand, dst_floor) != treg {
                continue;
            }
            let n = target.size.min(64) as usize;
            // A string where Infection has one; a pointer's bytes can read as
            // text (0x006f2d98 is "\x98-o").
            if !relocated.contains(&(*tsec, target.value)) {
                let Ok(mut a) = tsp.read(target.value, n) else { continue };
                a.push(0);
                if printable(&a) {
                    let Ok(mut b) = tdp.read(cand, n) else { continue };
                    b.push(0);
                    if !printable(&b) {
                        continue;
                    }
                }
            }
            *votes.entry((target.name.clone(), target.size, *tsec)).entry(cand).or_default() += 1;
        }
        let mut changed = 0;
        let mut won = unanimous(&votes);
        won.sort_by_key(|x| x.1);
        for ((name, size, tsec), va) in won {
            let key = (tsec, name.clone());
            if named.get(&key) == Some(&va) {
                continue;
            }
            // Only the passes that read a global's own uses can be wrong
            // where a table of pointers says otherwise.
            let unsure = |k: &Key| matches!(how.get(k), Some(&"data") | Some(&"pointer"));
            if named.contains(&key) && !unsure(&key) {
                continue;
            }
            let holder = at.get(&(tsec, va)).cloned();
            if let Some(h) = &holder
                && !unsure(&(tsec, h.clone()))
            {
                continue;
            }
            if let Some(h) = holder {
                dropped.insert((tsec, h.clone(), va));
                named.remove(&(tsec, h.clone()));
                added.remove(&(tsec, h));
            }
            if let Some(&old) = named.get(&key) {
                dropped.insert((tsec, name.clone(), old));
                at.remove(&(tsec, old));
                moved += 1;
            }
            named.insert(key.clone(), va);
            at.insert((tsec, va), name);
            how.insert(key.clone(), "pointer");
            added.insert(key, (va, size));
            changed += 1;
        }
        if changed == 0 {
            break;
        }
    }
    let mut kept: Vec<Row> = rows
        .into_iter()
        .filter(|r| !dropped.contains(&(r.sec, r.name.clone(), r.va)) && !added.contains(&(r.sec, r.name.clone())))
        .collect();
    let mut new: Vec<Row> = added
        .iter()
        .map(|((sec, name), &(va, size))| Row { sec, va, size, kind: "OBJECT", name: name.clone(), how: "pointer" })
        .collect();
    new.sort_by(|a, b| (a.sec, a.va).cmp(&(b.sec, b.va)));
    let n = new.len();
    kept.extend(new);
    (kept, n, moved)
}

/// How well a global's words in the other volume agree with Infection's:
/// equal numbers, and pointers (non-zero, mapped) where Infection has a
/// relocated word.
struct Agreement {
    v: Vol,
    /// Infection's sized globals by section, in the order the symbols first
    /// name the section.
    objs: Vec<(&'static str, Vec<Symbol>)>,
    count: HashMap<(&'static str, String), usize>,
    relocated: HashSet<(&'static str, u32)>,
}

impl Agreement {
    fn new(v: Vol) -> Agreement {
        let src_main = prog(Vol::Inf, "main");
        let mut objs: Vec<(&'static str, Vec<Symbol>)> = Vec::new();
        let mut count = HashMap::new();
        for sym in &src_main.elf.symbols {
            if sym.kind != STT_OBJECT || sym.size == 0 {
                continue;
            }
            let name = src_main.section_name(sym.shndx);
            let sec = match name.strip_suffix(".prg") {
                Some(o) => sec_name(o),
                None if name == "main" => "main",
                None => continue,
            };
            match objs.iter_mut().find(|x| x.0 == sec) {
                Some(x) => x.1.push(sym.clone()),
                None => objs.push((sec, vec![sym.clone()])),
            }
            *count.entry((sec, sym.name.clone())).or_default() += 1;
        }
        let mut relocated = HashSet::new();
        for (sec, _) in &objs {
            for r in prog(Vol::Inf, sec).relocs() {
                if r.kind == 2 && r.addr.is_some_and(|a| a != 0) {
                    relocated.insert((*sec, r.offset));
                }
            }
        }
        Agreement { v, objs, count, relocated }
    }

    /// The string at `va`, when it is one and not a pointer's bytes.
    fn text(&self, p: &Program, sec: &'static str, va: u32) -> Option<Vec<u8>> {
        if self.relocated.contains(&(sec, va)) || self.relocated.contains(&("main", va)) {
            return None;
        }
        let mut bs = p.read(va, 64).ok()?;
        bs.push(0);
        printable(&bs).then(|| bs[..bs.iter().position(|&c| c == 0).unwrap()].to_vec())
    }

    /// The share of `o`'s words that agree at `dva`; None in .bss. A
    /// pointer to a string agrees when it points at the same string.
    fn score(&self, sec: &'static str, o: &Symbol, dva: i64) -> Option<f64> {
        let dva = u32::try_from(dva).ok()?;
        let (sp, dp) = (prog(Vol::Inf, sec), prog(self.v, sec));
        let (mut ok, mut total) = (0u32, 0u32);
        for off in (0..o.size.saturating_sub(3)).step_by(4) {
            let (sw, dw) = (sp.u32(o.value + off).ok()?, dp.u32(dva.wrapping_add(off)).ok()?);
            total += 1;
            if self.relocated.contains(&(sec, o.value + off)) {
                if sw == 0 || dw == 0 || !dp.mapped(dw) {
                    ok += u32::from(sw == 0 && dw == 0);
                } else {
                    match self.text(&sp, sec, sw) {
                        None => ok += 1,
                        Some(st) => {
                            if let Ok(c) = dp.cstr(dw, 64) {
                                ok += u32::from(c == st);
                            }
                        }
                    }
                }
            } else {
                ok += u32::from(sw == dw);
            }
        }
        if total == 0 {
            let same = sp.read(o.value, o.size as usize).ok()? == dp.read(dva, o.size as usize).ok()?;
            return Some(if same { 1.0 } else { 0.0 });
        }
        Some(f64::from(ok) / f64::from(total))
    }
}

/// Globals placed by the layout between two that agree.
///
/// First, two data-pass globals of one size, near each other, whose words
/// agree better each at the other's place, change places: the data pass
/// pairs a function's globals by the order its code builds them, which a
/// reordered function breaks (Mutation's `WORLD::Init` builds `BGTBL2`
/// before `BGTBL`).
///
/// Then, two carried globals of one section that sit the same distance
/// apart in both volumes, their words agreeing (90%), bound a span the
/// other volume kept whole, when nothing carried from outside the span
/// lies inside it. The globals Infection has between them keep their
/// offsets there: an unnamed one is named at its offset when its words
/// agree (Mutation's `BgMatName`, which `WORLD::DrawBG` now builds after
/// `BgMatName2`), and a data-pass name placed elsewhere moves there when
/// the words agree better. A span holding a global another pass placed
/// elsewhere is left alone. String literals (`@123`) are not placed.
fn layout(v: Vol, rows: Vec<Row>) -> (Vec<Row>, usize, usize) {
    const MAX_SPAN: u32 = 0x2000;
    const SURE: f64 = 0.9;
    let agree = Agreement::new(v);
    let (named0, how) = objects(&rows);
    let mut named: HashMap<Key, i64> = named0.iter().map(|(k, &va)| (k.clone(), i64::from(va))).collect();
    let mut moves: Ordered<Key, (u32, u32)> = Default::default();
    for (sec, all) in &agree.objs {
        let sec: &'static str = sec;
        let mut lst: Vec<&Symbol> = all.iter().filter(|o| agree.count[&(sec, o.name.clone())] == 1).collect();
        lst.sort_by_key(|o| o.value);
        // The swaps.
        let mut by_size: Ordered<u32, Vec<&Symbol>> = Default::default();
        for &o in &lst {
            if how.get(&(sec, o.name.clone())) == Some(&"data") && !o.name.starts_with('@') {
                by_size.entry(o.size).push(o);
            }
        }
        for group in by_size.values() {
            for (i, a) in group.iter().enumerate() {
                for b in &group[i + 1..] {
                    let (ka, kb) = ((sec, a.name.clone()), (sec, b.name.clone()));
                    let (na, nb) = (named[&ka], named[&kb]);
                    if moves.contains(&ka) || moves.contains(&kb) || (na - nb).abs() > 0x400 {
                        continue;
                    }
                    let (Some(aa), Some(bb), Some(ab), Some(ba)) = (
                        agree.score(sec, a, na),
                        agree.score(sec, b, nb),
                        agree.score(sec, a, nb),
                        agree.score(sec, b, na),
                    ) else {
                        continue;
                    };
                    if ab.min(ba) < SURE {
                        continue;
                    }
                    if ab > aa && ba > bb {
                        moves.insert(ka, (nb as u32, a.size));
                        moves.insert(kb, (na as u32, b.size));
                    }
                }
            }
        }
        for (key, &(va, _)) in moves.iter() {
            if key.0 == sec {
                named.insert(key.clone(), i64::from(va));
            }
        }
        // The spans. Main's globals an overlay's code builds are carried
        // under the overlay's name: they count as inside main's spans too.
        let window = i64::from(prog(v, "main").overlay_space.0);
        let mut placed: Vec<(i64, &str)> = named0
            .iter()
            .map(|(k, _)| (named[k], k))
            .filter(|(va, k)| k.0 == sec || (sec == "main" && *va < window))
            .map(|(va, k)| (va, k.1.as_str()))
            .collect();
        placed.sort();
        let places: Vec<i64> = placed.iter().map(|p| p.0).collect();
        // Where an overlay's code puts one of main's globals: a span ends
        // before one it would move elsewhere (Mutation's `cmndSortRoot`,
        // which gcmn builds 4 bytes past where Infection's spacing puts it).
        let built: HashMap<&str, i64> = named0
            .iter()
            .filter(|(k, _)| sec == "main" && k.0 != sec && named[k] < window)
            .map(|(k, _)| (k.1.as_str(), named[k]))
            .collect();
        let mut anchors: Vec<(usize, i64)> = Vec::new();
        for (k, o) in lst.iter().enumerate() {
            // Only words that agree anchor a span: in .bss nothing tells.
            if let Some(&va) = named.get(&(sec, o.name.clone()))
                && agree.score(sec, o, va).is_some_and(|a| a >= SURE)
            {
                anchors.push((k, va - i64::from(o.value)));
            }
        }
        for (ai, &(k, d)) in anchors.iter().enumerate() {
            let Some(m) = anchors[ai + 1..].iter().find(|x| x.1 == d).map(|x| x.0) else { continue };
            if m == k + 1 || lst[m].value - lst[k].value > MAX_SPAN {
                continue;
            }
            let mut between = &lst[k + 1..m];
            let mut end = lst[m].value;
            // The span is whole only up to a global an overlay's code
            // puts elsewhere: something was inserted before it.
            if let Some(cut) =
                between.iter().position(|o| built.get(o.name.as_str()).is_some_and(|&va| va != i64::from(o.value) + d))
            {
                end = between[cut].value;
                between = &between[..cut];
            }
            let own: HashSet<&str> = between.iter().map(|o| o.name.as_str()).collect();
            let (lo, hi) = (i64::from(lst[k].value) + d, i64::from(end) + d);
            let (a, b) = (places.partition_point(|&p| p <= lo), places.partition_point(|&p| p < hi));
            if b > a && placed[a..b].iter().any(|(_, name)| !own.contains(name)) {
                continue;
            }
            let wrong = between
                .iter()
                .filter(|o| named.get(&(sec, o.name.clone())).is_some_and(|&va| va != i64::from(o.value) + d));
            if wrong.clone().any(|o| how[&(sec, o.name.clone())] != "data" || moves.contains(&(sec, o.name.clone()))) {
                continue;
            }
            for o in between {
                let (key, want) = ((sec, o.name.clone()), i64::from(o.value) + d);
                if named.get(&key) == Some(&want) || moves.contains(&key) || o.name.starts_with('@') {
                    continue;
                }
                let Some(there) = agree.score(sec, o, want).filter(|&t| t >= SURE) else { continue };
                if let Some(&va) = named.get(&key) {
                    match agree.score(sec, o, va) {
                        Some(here) if there > here => {}
                        _ => continue,
                    }
                }
                moves.insert(key, (want as u32, o.size));
            }
        }
    }
    let mut out: Vec<Row> = rows.into_iter().filter(|r| !moves.contains(&(r.sec, r.name.clone()))).collect();
    for ((sec, name), &(va, size)) in moves.iter() {
        out.push(Row { sec, va, size, kind: "OBJECT", name: name.clone(), how: "layout" });
    }
    let n_moved = moves.iter().filter(|(k, _)| how.contains_key(k)).count();
    out.sort_by(|a, b| (a.sec, a.va).cmp(&(b.sec, b.va)));
    let n = moves.iter().count();
    (out, n - n_moved, n_moved)
}

/// Data-pass globals the code puts elsewhere. A paired function whose body
/// builds as many addresses in both volumes pairs them in order, and each
/// global of Infection's built there (in the function's own section)
/// votes for where the other body's address puts it. A global the data
/// pass placed moves where every such vote agrees, two at least, when no
/// other global holds that place. The data pass pairs one body's globals
/// by their order, which an edited body can mislead: it put Mutation's
/// `MailTbl` 194 rows below the table the mailer reads.
fn code_checks(v: Vol, rows: Vec<Row>) -> (Vec<Row>, usize) {
    let (named, how) = objects(&rows);
    let held: HashSet<(&'static str, u32)> = named.iter().map(|(k, &va)| (k.0, va)).collect();
    let mut votes: Votes<Key, u32> = Default::default();
    for ov in SECTIONS {
        let sec = ov.unwrap_or("main");
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let fns: HashMap<&str, (u32, u32)> = rows
            .iter()
            .filter(|r| r.sec == sec && r.kind == "FUNC")
            .map(|r| (r.name.as_str(), (r.va, r.size)))
            .collect();
        let mut targets: HashMap<u32, Symbol> = HashMap::new();
        for r in src.p.relocs() {
            if !matches!(r.kind, 6 | 7) || !src.owns(r.offset) {
                continue;
            }
            let Some(sym) = src.p.elf.symbols.get(r.symbol) else { continue };
            if sym.kind == STT_OBJECT
                && sym.size != 0
                && named_section(&src.p.section_name(sym.shndx)) == sec
                && !sym.name.starts_with('@')
            {
                targets.insert(r.offset, sym.clone());
            }
        }
        for f in src.functions() {
            let Some(&(dva, dsize)) = fns.get(f.name.as_str()) else { continue };
            let (Some(sw), Some(dw)) = (src.run(f.value, f.size / 4), dst.run(dva, 1.max(dsize / 4))) else { continue };
            let (sa, da) = (addresses(sw, src.p.gp), addresses(dw, dst.p.gp));
            if sa.len() != da.len() {
                continue;
            }
            let mut si: Vec<usize> = sa.keys().copied().collect();
            si.sort();
            let mut dj: Vec<usize> = da.keys().copied().collect();
            dj.sort();
            for (i, j) in si.into_iter().zip(dj) {
                let Some(sym) = targets.get(&(f.value + 4 * i as u32)) else { continue };
                let va = da[&j].wrapping_sub(sa[&i].wrapping_sub(sym.value));
                *votes.entry((sec, sym.name.clone())).entry(va).or_default() += 1;
            }
        }
    }
    let mut moves: HashMap<Key, u32> = HashMap::new();
    for (key, cands) in votes.iter() {
        let [(&va, &n)] = cands.iter().collect::<Vec<_>>()[..] else { continue };
        if n >= 2
            && how.get(key) == Some(&"data")
            && named.get(key).is_some_and(|&at| at != va)
            && !held.contains(&(key.0, va))
        {
            moves.insert(key.clone(), va);
        }
    }
    let n = moves.len();
    let out = rows
        .into_iter()
        .map(|r| match moves.get(&(r.sec, r.name.clone())) {
            Some(&va) if r.kind == "OBJECT" => Row { va, how: "code", ..r },
            _ => r,
        })
        .collect();
    (out, n)
}

/// Each overlay's static initialisers, from its constructor list: the
/// volume's entries pair with Infection's by position when both lists are
/// as long. Each runs to the next one up, the last to the list itself. A
/// row another pass gave the name, or the place, gives way. No other pass
/// names them: their bodies are runs of copies that match anywhere (the
/// desktop's `__sinit_NameEntry.cpp`, which fills the name entry's
/// `InfoMsg`).
fn ctors(v: Vol, rows: Vec<Row>) -> (Vec<Row>, usize) {
    let mut add: Vec<Row> = Vec::new();
    for ov in SECTIONS.into_iter().flatten() {
        let (ip, p) = (ctx(Vol::Inf, Some(ov)).p, ctx(v, Some(ov)).p);
        let list = |q: &Program| {
            let o = q.overlay.as_ref().unwrap();
            (o.ctor_start..o.ctor_end).step_by(4).map(|a| q.u32(a).unwrap()).collect::<Vec<_>>()
        };
        let (theirs, mine) = (list(&ip), list(&p));
        if theirs.len() != mine.len() {
            continue;
        }
        let mut ends = mine.clone();
        ends.sort();
        ends.push(p.overlay.as_ref().unwrap().ctor_start);
        for (t, m) in theirs.into_iter().zip(mine) {
            let Some((s, 0)) = ip.symbol_at(t, 0) else { continue };
            let k = ends.iter().position(|&e| e == m).unwrap();
            if s.kind == STT_FUNC && ends[k + 1] > m {
                let sec = sec_name(ov);
                add.push(Row { sec, va: m, size: ends[k + 1] - m, kind: "FUNC", name: s.name.clone(), how: "ctor" });
            }
        }
    }
    let names: HashSet<(&str, String)> = add.iter().map(|r| (r.sec, r.name.clone())).collect();
    let places: HashSet<(&str, u32)> = add.iter().map(|r| (r.sec, r.va)).collect();
    let same = |r: &Row| rows.iter().any(|o| o.sec == r.sec && o.name == r.name && o.va == r.va && o.kind == "FUNC");
    let add: Vec<Row> = add.into_iter().filter(|r| !same(r)).collect();
    let n = add.len();
    let changed: HashSet<(&str, String)> = add.iter().map(|r| (r.sec, r.name.clone())).collect();
    let mut out: Vec<Row> = rows
        .into_iter()
        .filter(|r| {
            r.kind != "FUNC"
                || !(changed.contains(&(r.sec, r.name.clone()))
                    || (places.contains(&(r.sec, r.va)) && !names.contains(&(r.sec, r.name.clone()))))
        })
        .collect();
    out.extend(add);
    out.sort_by(|a, b| (a.sec, a.va).cmp(&(b.sec, b.va)));
    (out, n)
}

/// The volume's sidecar rows, every pass in turn, with what each found.
fn carry_symbols(v: Vol) -> Vec<Row> {
    let (mut rows, stats) = transfer(v);
    let across = cross(v, &rows);
    let n_across = across.len();
    rows.extend(across);
    // The code's own addresses outrank the data pass's pairing.
    let (mut rows, n_code) = code_checks(v, rows);
    // What a global holds (its numbers, the strings it points at) outranks
    // a table's pointer to it: Mutation's pcMsg4 keeps Infection's layout
    // with rows added to what it points at.
    let found = content(v, &rows);
    let mut n_content = found.len();
    rows.extend(found);
    let (rows2, mut n_pointer, mut n_moved) = pointers(v, rows);
    let (mut rows, n_layout, n_relaid) = layout(v, rows2);
    let more = content(v, &rows);
    n_content += more.len();
    rows.extend(more);
    let by_ref = refs(v, &rows);
    let n_ref = by_ref.len();
    rows.extend(by_ref);
    let by_prefix = prefixes(v, &rows);
    let n_prefix = by_prefix.len();
    rows.extend(by_prefix);
    // What the content pass found can name more through its pointers.
    let (rows, n_more, n_moved2) = pointers(v, rows);
    let (rows, n_ctor) = ctors(v, rows);
    n_pointer += n_more;
    n_moved += n_moved2;
    println!("{}:", v.tag());
    println!("{:9} {:>8} {:>7} {:>6} {:>6} {:>8}", "section", "src fns", "exact", "call", "order", "globals");
    for (sec, nsrc, exact, call, order, glob) in stats {
        println!("{sec:9} {nsrc:8} {exact:7} {call:6} {order:6} {glob:8}");
    }
    println!("cross     {n_across} globals by the order of the addresses built");
    println!("code      {n_code} of the data pass's globals moved where the code builds them");
    println!("pointer   {n_pointer} globals ({n_moved} of the data pass's moved)");
    println!("layout    {n_layout} globals ({n_relaid} of the data pass's moved)");
    println!("content   {n_content} globals found by what they hold");
    println!("ref       {n_ref} functions named by the globals they build");
    println!("prefix    {n_prefix} functions named by their opening words");
    println!("ctor      {n_ctor} static initialisers named from the overlays' constructor lists");
    rows
}

/// The symbols carried to the volume, as its sidecar's text.
pub fn text(v: Vol) -> String {
    let rows = carry_symbols(v);
    let mut text =
        format!("# symbols carried from {} by piney-gen syms\n# section\tva\tsize\ttype\tname\thow\n", Vol::Inf.exe());
    for r in &rows {
        text.push_str(&format!("{}\t{:08x}\t{}\t{}\t{}\t{}\n", r.sec, r.va, r.size, r.kind, r.name, r.how));
    }
    text
}

/// Carry the symbols to the volume and write its sidecar (or with `check`,
/// say whether the sidecar holds them).
pub fn write(v: Vol, check: bool) -> bool {
    let text = text(v);
    let rows = text.lines().filter(|l| !l.starts_with('#')).count();
    let path = std::path::PathBuf::from(format!("{}.syms", elf_path(v).display()));
    if check {
        let same = std::fs::read_to_string(&path).is_ok_and(|t| t == text);
        println!("{} {}", path.display(), if same { "is current" } else { "differs" });
        return same;
    }
    std::fs::write(&path, text).unwrap_or_else(|e| crate::die(&format!("{}: {e}", path.display())));
    println!("{rows} symbols -> {}", path.display());
    true
}
