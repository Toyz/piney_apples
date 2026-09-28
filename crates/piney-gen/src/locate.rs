//! Where an entry is in each volume. On Infection at its own address; on
//! the later volumes the carry's place (`carry`'s cache), the volume's own
//! name for it, the places the code that reads it builds, and where its
//! neighbours went, each that reads as the layout scored by how much of it
//! is like Infection's.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::layout::Value;
use crate::manifest::{Entry, Group};
use crate::volume::{Ctx, Vol, ctx};

/// The carry's section of an overlay: 0 main, then the PRG ids.
pub fn section(overlay: Option<&str>) -> u32 {
    match overlay {
        None => 0,
        Some("gcmn") => 1,
        Some("demo") => 2,
        Some("desktop") => 3,
        Some("toppage") => 4,
        Some(o) => crate::die(&format!("unknown overlay {o}")),
    }
}

type Values = Rc<Vec<Value>>;
type Rows = Rc<Vec<(u32, u32, u32, u32)>>;
/// A code range: its start, its words and their opcode keys.
type Code = Rc<(u32, Vec<u32>, Vec<u16>)>;

thread_local! {
    static CARRY: RefCell<HashMap<Vol, Rows>> = RefCell::new(HashMap::new());
    static CODE: RefCell<HashMap<(Vol, Option<&'static str>), Code>> = RefCell::new(HashMap::new());
    static EXTRACTED: RefCell<HashMap<(String, Vol), Values>> = RefCell::new(HashMap::new());
}

fn rows(v: Vol) -> Rows {
    CARRY.with(|c| c.borrow_mut().entry(v).or_insert_with(|| Rc::new(crate::carry::cached(v))).clone())
}

/// Infection's `inf` in the volume by the carry: a global holding it, or a
/// stretch between two rows laid out alike.
pub fn carried(v: Vol, sec: u32, inf: u32) -> Option<u32> {
    if v == Vol::Inf {
        return Some(inf);
    }
    let rows = rows(v);
    let key = (sec, inf, u32::MAX, u32::MAX);
    let i = rows.partition_point(|r| *r <= key);
    for r in rows[i.saturating_sub(8)..i].iter().rev() {
        if r.0 == sec && r.1 <= inf && inf < r.1 + r.2.max(1) {
            return Some(r.3 + inf - r.1);
        }
    }
    if i > 0 && i < rows.len() {
        let (lo, hi) = (rows[i - 1], rows[i]);
        if lo.0 == sec && sec == hi.0 && hi.1.wrapping_sub(lo.1) == hi.3.wrapping_sub(lo.3) {
            return Some(lo.3 + inf - lo.1);
        }
    }
    None
}

fn carry_of(v: Vol, overlay: Option<&str>, inf: u32) -> Option<u32> {
    let sec = if inf >= 0x0040_0000 { section(overlay) } else { 0 };
    carried(v, sec, inf)
        .or_else(|| if sec == 0 && overlay.is_some() { carried(v, section(overlay), inf) } else { None })
}

/// The entry's address in the volume: the carry's, the volume's own name
/// for it, or the code's.
pub fn locate(v: Vol, overlay: Option<&'static str>, inf: u32) -> Option<u32> {
    if v == Vol::Inf {
        return Some(inf);
    }
    if let Some(va) = carry_of(v, overlay, inf) {
        return Some(va);
    }
    let ip = ctx(Vol::Inf, overlay).p;
    if let Some((s, 0)) = ip.symbol_at(inf, 0)
        && ip.symbol_named(&s.name).is_some()
        && let Some(mine) = ctx(v, overlay).p.symbol_named(&s.name)
    {
        return Some(mine.value);
    }
    by_code(v, overlay, inf)
}

/// Infection's `inf` in the context's volume (for a custom entry).
pub fn find(c: &Ctx, inf: u32, overlay: Option<&'static str>) -> u32 {
    let ov = overlay.or(c.overlay);
    locate(c.volume, ov, inf).unwrap_or_else(|| {
        crate::die(&format!("{}: find (Infection 0x{inf:08x}) not found: give it a finder", c.volume.tag()))
    })
}

const LO_OPS: [u32; 19] =
    [0x09, 0x0d, 0x20, 0x21, 0x23, 0x24, 0x25, 0x27, 0x28, 0x29, 0x2b, 0x31, 0x35, 0x37, 0x39, 0x3d, 0x3f, 0x1e, 0x1f];

fn key(w: u32) -> u16 {
    let op = w >> 26;
    if matches!(op, 0x00 | 0x1c | 0x11 | 0x12) { ((op << 8) | (w & 0x3f)) as u16 } else { ((op << 8) | 0xff) as u16 }
}

fn sext(imm: u32) -> u32 {
    (imm as u16 as i16) as i32 as u32
}

fn code(v: Vol, overlay: Option<&'static str>) -> Code {
    CODE.with(|c| {
        c.borrow_mut()
            .entry((v, overlay))
            .or_insert_with(|| {
                let p = ctx(v, overlay).p;
                let (lo, hi) = p.code_range(overlay.is_some());
                let n = (hi - lo) / 4;
                let raw = p.read(lo, 4 * n as usize).unwrap_or_else(|e| crate::die(&e));
                let words: Vec<u32> = raw.chunks(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
                let keys = words.iter().map(|&w| key(w)).collect();
                Rc::new((lo, words, keys))
            })
            .clone()
    })
}

/// (index of the lui, of its low half, the address built); a `$gp`-relative
/// access is its own pair.
fn pairs(ws: &[u32], gp: Option<u32>) -> Vec<(usize, usize, u32)> {
    let mut out = Vec::new();
    for (i, &w) in ws.iter().enumerate() {
        if let Some(gp) = gp
            && LO_OPS.contains(&(w >> 26))
            && (w >> 21) & 31 == 28
        {
            out.push((i, i, gp.wrapping_add(sext(w & 0xffff))));
            continue;
        }
        if w >> 26 != 0x0f {
            continue;
        }
        let (r, hi) = ((w >> 16) & 31, w & 0xffff);
        for (j, &x) in ws.iter().enumerate().take((i + 7).min(ws.len())).skip(i + 1) {
            if LO_OPS.contains(&(x >> 26)) && (x >> 21) & 31 == r {
                let imm = x & 0xffff;
                out.push((i, j, if x >> 26 == 0x0d { (hi << 16) | imm } else { (hi << 16).wrapping_add(sext(imm)) }));
                break;
            }
            if (x >> 16) & 31 == r && ![0x2b, 0x29, 0x28, 0x3f, 0x39].contains(&(x >> 26)) {
                break;
            }
        }
    }
    out
}

fn by_code_window(v: Vol, overlay: Option<&'static str>, inf: u32, window: usize) -> Option<u32> {
    let i = code(Vol::Inf, overlay);
    let igp = ctx(Vol::Inf, overlay).p.gp;
    let sites: Vec<(usize, usize)> = pairs(&i.1, igp).into_iter().filter(|p| p.2 == inf).map(|p| (p.0, p.1)).collect();
    if sites.is_empty() {
        return None;
    }
    let d = code(v, overlay);
    let vgp = ctx(v, overlay).p.gp;
    let (vws, vkeys) = (&d.1, &d.2);
    let mut found = std::collections::BTreeSet::new();
    for (si, sj) in sites {
        let (a, b) = (si.saturating_sub(window), (sj + window + 1).min(i.1.len()));
        let want = &i.2[a..b];
        if vws.len() < want.len() {
            continue;
        }
        for k in 0..vws.len() - want.len() {
            if vkeys[k] != want[0] || &vkeys[k..k + want.len()] != want {
                continue;
            }
            let (lui, low) = (vws[k + si - a], vws[k + sj - a]);
            let imm = low & 0xffff;
            if si == sj {
                if (low >> 21) & 31 != 28 {
                    continue;
                }
                let Some(g) = vgp else { continue };
                found.insert(g.wrapping_add(sext(imm)));
                continue;
            }
            if lui >> 26 != 0x0f || (low >> 21) & 31 != (lui >> 16) & 31 {
                continue;
            }
            let hi = (lui & 0xffff) << 16;
            found.insert(if low >> 26 == 0x0d { hi | imm } else { hi.wrapping_add(sext(imm)) });
        }
    }
    (found.len() == 1).then(|| *found.iter().next().unwrap())
}

/// The widest window of the code round the reference that gives one answer.
pub fn by_code(v: Vol, overlay: Option<&'static str>, inf: u32) -> Option<u32> {
    [8, 5, 3].into_iter().find_map(|w| by_code_window(v, overlay, inf, w))
}

fn code_candidates(v: Vol, overlay: Option<&'static str>, inf: u32) -> Vec<u32> {
    let mut out = Vec::new();
    for w in [8, 5, 3] {
        if let Some(va) = by_code_window(v, overlay, inf, w)
            && !out.contains(&va)
        {
            out.push(va);
        }
    }
    out
}

/// Where the nearest of Infection's other globals went, shifted by their
/// distance: a block of globals keeps its order.
fn neighbour_candidates(v: Vol, overlay: Option<&'static str>, inf: u32) -> Vec<u32> {
    let p = ctx(Vol::Inf, overlay).p;
    let sec = if inf >= 0x0040_0000 { section(overlay) } else { 0 };
    let mut near: Vec<(u32, u32)> = p
        .symbols
        .iter()
        .filter(|s| s.kind == 1 && s.value != inf && s.value.abs_diff(inf) <= 0x100)
        .map(|s| (s.value.abs_diff(inf), s.value))
        .collect();
    near.sort();
    let place = |v: Vol, at: u32| {
        carried(v, sec, at)
            .or_else(|| if sec == 0 && overlay.is_some() { carried(v, section(overlay), at) } else { None })
    };
    // Past Mutation a block laid out as Mutation's (a table that grew there)
    // keeps Mutation's distances, not Infection's.
    let mutation = if matches!(v, Vol::Out | Vol::Qua) { place(Vol::Mut, inf) } else { None };
    let mut out = Vec::new();
    for &(_, at) in near.iter().take(12) {
        if let Some(c) = place(v, at) {
            let va = c.wrapping_add(inf).wrapping_sub(at);
            if !out.contains(&va) {
                out.push(va);
            }
            if let (Some(m), Some(m_at)) = (mutation, place(Vol::Mut, at)) {
                let va = c.wrapping_add(m).wrapping_sub(m_at);
                if !out.contains(&va) {
                    out.push(va);
                }
            }
        }
    }
    out
}

fn read_or_die(g: &Group, e: &Entry, c: &Ctx, va: u32) -> Value {
    e.layout
        .read(c, va)
        .unwrap_or_else(|err| crate::die(&format!("{}: {}.{} at 0x{va:08x}: {err}", c.volume.tag(), g.name, e.name)))
}

/// Every entry's value in the volume.
pub fn extract(g: &Group, v: Vol) -> Rc<Vec<Value>> {
    let k = (g.name.to_string(), v);
    if let Some(x) = EXTRACTED.with(|m| m.borrow().get(&k).cloned()) {
        return x;
    }
    let inf = (v != Vol::Inf).then(|| extract(g, Vol::Inf));
    let mut out = Vec::new();
    let mut found: HashMap<&str, u32> = HashMap::new();
    for (i, e) in g.entries.iter().enumerate() {
        let c = ctx(v, e.overlay);
        if let Some((_, val)) = e.absent.iter().find(|(av, _)| *av == v) {
            out.push(val.clone());
            continue;
        }
        if let Some((base, off)) = e.after {
            let va = found[base].wrapping_add(off as u32);
            out.push(read_or_die(g, e, &c, va));
            found.insert(e.name, va);
            continue;
        }
        let Some(infa) = e.inf else {
            // Its own finder.
            out.push(read_or_die(g, e, &c, 0));
            continue;
        };
        if v == Vol::Inf {
            out.push(read_or_die(g, e, &c, infa));
            found.insert(e.name, infa);
            continue;
        }
        let first = locate(v, e.overlay, infa);
        let mut cands: Vec<u32> = first.into_iter().collect();
        if !(e.carried && first.is_some()) {
            cands.extend(code_candidates(v, e.overlay, infa));
            cands.extend(neighbour_candidates(v, e.overlay, infa));
        }
        let mut tried: Vec<(u32, Value)> = Vec::new();
        let mut errors = Vec::new();
        for va in cands {
            if tried.iter().any(|t| t.0 == va) {
                continue;
            }
            match e.layout.read(&c, va) {
                Ok(val) => tried.push((va, val)),
                Err(err) => errors.push(format!("0x{va:08x}: {err}")),
            }
        }
        if tried.is_empty() {
            let why = if errors.is_empty() { "not found: give it a finder".to_string() } else { errors.join("; ") };
            crate::die(&format!("{}: {}.{} (Infection 0x{infa:08x}): {why}", v.tag(), g.name, e.name));
        }
        let want = &inf.as_ref().unwrap()[i];
        let mut best = 0;
        let mut score = tried[0].1.likeness(want);
        for (j, t) in tried.iter().enumerate().skip(1) {
            let s = t.1.likeness(want);
            if s > score {
                (best, score) = (j, s);
            }
        }
        let (va, val) = tried.swap_remove(best);
        out.push(val);
        found.insert(e.name, va);
    }
    let out = Rc::new(out);
    EXTRACTED.with(|m| m.borrow_mut().insert(k, out.clone()));
    out
}
