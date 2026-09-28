//! Comparing a function of Infection's with its twin in a later volume:
//! the words with their addresses masked, the addresses the code builds,
//! and which instructions line up; and the first passes of the symbol
//! transfer (`syms`), which pair the functions and the globals their code
//! builds.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::rc::Rc;

use crate::elf::{STT_OBJECT, Symbol};
use crate::program::Program;
use crate::volume::{Vol, ctx};

/// Words compared before the full body.
const KEY: usize = 6;
/// Shorter functions are too generic to match on their own.
const MIN_WORDS: u32 = 6;
/// The order pass's least shape score, pairing a gap one for one.
const EVEN: f64 = 0.3;
/// The same, pairing within an uneven gap.
const ALONE: f64 = 0.65;
/// The shortest run of agreeing words that counts as aligned.
const ALIGN: usize = 8;
/// The same, inside a gap already bounded by aligned runs.
const INNER: usize = 4;
const GP: u32 = 28;

/// I-type ops whose 16-bit immediate can be the low half of an address:
/// addiu, daddiu, ori, the loads and the stores.
fn lo_op(op: u32) -> bool {
    matches!(
        op,
        0x09 | 0x19
            | 0x0d
            | 0x20..=0x27
            | 0x37
            | 0x1a
            | 0x1b
            | 0x1e
            | 0x31
            | 0x36
            | 0x28..=0x2e
            | 0x3f
            | 0x1f
            | 0x39
            | 0x3e
    )
}

fn load(op: u32) -> bool {
    matches!(op, 0x20..=0x27 | 0x37 | 0x1a | 0x1b | 0x1e)
}

fn sext16(v: u32) -> u32 {
    (v as u16 as i16) as i32 as u32
}

/// Every field that holds an absolute address or a `$gp` offset masked.
pub fn normalise(words: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(words.len());
    // The registers holding a lui'd upper half.
    let mut hi = [false; 32];
    for &w in words {
        let (op, rs, rt) = (w >> 26, ((w >> 21) & 31) as usize, ((w >> 16) & 31) as usize);
        if op == 2 || op == 3 {
            out.push(w & 0xFC00_0000);
            continue;
        }
        if op == 0x0F {
            out.push(w & 0xFFFF_0000);
            hi[rt] = true;
            continue;
        }
        if lo_op(op) && (rs == GP as usize || hi[rs]) {
            out.push(w & 0xFFFF_0000);
            if load(op) {
                hi[rt] = false;
            }
            continue;
        }
        out.push(w);
        if op == 0 {
            let (funct, rd) = (w & 63, ((w >> 11) & 31) as usize);
            if matches!(funct, 0x21 | 0x2D | 0x25) && (hi[rs] || hi[rt]) {
                hi[rd] = true;
            } else if funct != 0x08 && funct != 0x09 {
                hi[rd] = false;
            }
        } else if (0x08..=0x0F).contains(&op) || load(op) || op == 0x19 {
            hi[rt] = false;
        }
    }
    out
}

/// {instruction index: the absolute address it forms}, for a low half
/// paired with a `lui` and for `$gp`-relative accesses.
pub fn addresses(words: &[u32], gp: Option<u32>) -> HashMap<usize, u32> {
    let mut out = HashMap::new();
    let mut hi: HashMap<u32, u32> = HashMap::new();
    let gp = gp.filter(|&g| g != 0);
    for (i, &w) in words.iter().enumerate() {
        let (op, rs, rt) = (w >> 26, (w >> 21) & 31, (w >> 16) & 31);
        if op == 0x0F {
            hi.insert(rt, (w & 0xFFFF) << 16);
            continue;
        }
        if lo_op(op) {
            if let (true, Some(g)) = (rs == GP, gp) {
                out.insert(i, g.wrapping_add(sext16(w & 0xFFFF)));
            } else if let Some(&h) = hi.get(&rs) {
                let imm = w & 0xFFFF;
                out.insert(i, if op == 0x0D { h | imm } else { h.wrapping_add(sext16(imm)) });
            }
            if load(op) || (matches!(op, 0x09 | 0x0D | 0x19) && rt != rs) {
                hi.remove(&rt);
            }
            continue;
        }
        if op == 0 && hi.contains_key(&((w >> 11) & 31)) && !matches!(w & 63, 0x08 | 0x09) {
            hi.remove(&((w >> 11) & 31));
        }
    }
    out
}

/// The opcode sequence, nops dropped: what survives register allocation
/// and scheduling changes between compiler settings.
pub fn shape(words: &[u32]) -> Vec<u32> {
    words
        .iter()
        .filter(|&&w| w != 0)
        .map(|&w| match w >> 26 {
            0 => w & 63,
            0x11 => 0x1100 | ((w >> 21) & 31) << 6 | (w & 63),
            0x1C => 0x1C00 | ((w >> 6) & 31) << 6 | (w & 63),
            1 => 0x100 | ((w >> 16) & 31),
            op => 0x200 | op,
        })
        .collect()
}

/// Python's `difflib.SequenceMatcher(None, a, b, autojunk=False)
/// .get_matching_blocks()`, the same blocks in the same order: the
/// longest match first (the earliest of equals), then recursively either
/// side of it; the blocks sorted, adjacent ones joined, and a final
/// `(len(a), len(b), 0)`.
pub fn matching_blocks<T: Eq + Hash + Copy>(a: &[T], b: &[T]) -> Vec<(usize, usize, usize)> {
    let mut b2j: HashMap<T, Vec<usize>> = HashMap::new();
    for (j, &x) in b.iter().enumerate() {
        b2j.entry(x).or_default().push(j);
    }
    let longest = |alo: usize, ahi: usize, blo: usize, bhi: usize| -> (usize, usize, usize) {
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0);
        let mut j2len: HashMap<usize, usize> = HashMap::new();
        for (i, x) in a.iter().enumerate().take(ahi).skip(alo) {
            let mut next = HashMap::new();
            if let Some(js) = b2j.get(x) {
                for &j in js {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = j.checked_sub(1).and_then(|p| j2len.get(&p)).copied().unwrap_or(0) + 1;
                    next.insert(j, k);
                    if k > bestsize {
                        (besti, bestj, bestsize) = (i + 1 - k, j + 1 - k, k);
                    }
                }
            }
            j2len = next;
        }
        // Nothing is junk: only equal neighbours extend the match.
        while besti > alo && bestj > blo && a[besti - 1] == b[bestj - 1] {
            (besti, bestj, bestsize) = (besti - 1, bestj - 1, bestsize + 1);
        }
        while besti + bestsize < ahi && bestj + bestsize < bhi && a[besti + bestsize] == b[bestj + bestsize] {
            bestsize += 1;
        }
        (besti, bestj, bestsize)
    };
    let mut queue = vec![(0, a.len(), 0, b.len())];
    let mut blocks = Vec::new();
    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        let (i, j, k) = longest(alo, ahi, blo, bhi);
        if k == 0 {
            continue;
        }
        blocks.push((i, j, k));
        if alo < i && blo < j {
            queue.push((alo, i, blo, j));
        }
        if i + k < ahi && j + k < bhi {
            queue.push((i + k, ahi, j + k, bhi));
        }
    }
    blocks.sort();
    let mut out = Vec::new();
    let (mut i1, mut j1, mut k1) = (0, 0, 0);
    for (i2, j2, k2) in blocks {
        if i1 + k1 == i2 && j1 + k1 == j2 {
            k1 += k2;
        } else {
            if k1 != 0 {
                out.push((i1, j1, k1));
            }
            (i1, j1, k1) = (i2, j2, k2);
        }
    }
    if k1 != 0 {
        out.push((i1, j1, k1));
    }
    out.push((a.len(), b.len(), 0));
    out
}

/// Index pairs (i, j) where the two normalised bodies agree, in runs of at
/// least ALIGN words, so an edited function still carries its unchanged
/// parts. Where they do not, runs of ALIGN or more agreeing shape tokens
/// pair what the compiler merely rescheduled, and inside each gap between
/// two such runs, runs of INNER or more.
pub fn aligned(sw: &[u32], dw: &[u32]) -> Vec<(usize, usize)> {
    let (sn, dn) = (normalise(sw), normalise(dw));
    if sn == dn {
        return (0..sn.len()).map(|i| (i, i)).collect();
    }
    let mut out = Vec::new();
    for (a, b, size) in matching_blocks(&sn, &dn) {
        if size >= ALIGN {
            out.extend((0..size).map(|k| (a + k, b + k)));
        }
    }
    if out.len() as i64 >= sn.len().min(dn.len()) as i64 - ALIGN as i64 {
        return out;
    }
    let si: Vec<usize> = (0..sw.len()).filter(|&i| sw[i] != 0).collect();
    let di: Vec<usize> = (0..dw.len()).filter(|&j| dw[j] != 0).collect();
    let (ss, ds) = (shape(sw), shape(dw));
    let have_s: std::collections::HashSet<usize> = out.iter().map(|p| p.0).collect();
    let have_d: std::collections::HashSet<usize> = out.iter().map(|p| p.1).collect();
    let blocks: Vec<(usize, usize, usize)> = matching_blocks(&ss, &ds).into_iter().filter(|b| b.2 >= ALIGN).collect();
    let mut inner = Vec::new();
    let mut ends = vec![(0, 0, 0)];
    ends.extend(&blocks);
    ends.push((ss.len(), ds.len(), 0));
    for w in ends.windows(2) {
        let ((a0, b0, n0), (a1, b1, _)) = (w[0], w[1]);
        let (lo_a, lo_b) = (a0 + n0, b0 + n0);
        if a1 > lo_a && b1 > lo_b {
            for (a, b, size) in matching_blocks(&ss[lo_a..a1], &ds[lo_b..b1]) {
                if size >= INNER {
                    inner.push((lo_a + a, lo_b + b, size));
                }
            }
        }
    }
    for (a, b, size) in blocks.into_iter().chain(inner) {
        for k in 0..size {
            let (i, j) = (si[a + k], di[b + k]);
            if !have_s.contains(&i) && !have_d.contains(&j) {
                out.push((i, j));
            }
        }
    }
    out
}

/// One section of one volume: main, or main and an overlay, as words.
pub struct Side {
    pub p: Rc<Program>,
    pub lo: u32,
    pub hi: u32,
    /// Where the code ends: main's words are all code to the finder.
    pub code_hi: u32,
    pub words: Vec<u32>,
}

impl Side {
    /// `n` words at `va`, None past the section (or for none).
    pub fn run(&self, va: u32, n: u32) -> Option<&[u32]> {
        let i = (i64::from(va) - i64::from(self.lo)) >> 2;
        let n = n as usize;
        (i >= 0 && i as usize + n <= self.words.len() && n > 0).then(|| &self.words[i as usize..i as usize + n])
    }

    pub fn word_at(&self, va: u32) -> u32 {
        self.words[((va - self.lo) >> 2) as usize]
    }

    pub fn owns(&self, va: u32) -> bool {
        self.lo <= va && va < self.hi
    }

    /// Likely function starts, sorted: jal targets, the first non-zero
    /// word after each `jr $ra` and its delay slot, and the first non-zero
    /// word after a `j` whose delay slot is followed by padding (a tail
    /// call ending a function).
    pub fn entries(&self) -> Vec<u32> {
        let mut found = BTreeSet::new();
        let n = ((self.code_hi - self.lo) >> 2) as usize;
        for i in 0..n {
            let w = self.words[i];
            let op = w >> 26;
            if op == 3 {
                let va = self.lo + 4 * i as u32;
                let t = ((va + 4) & 0xF000_0000) | ((w & 0x03FF_FFFF) << 2);
                if self.lo <= t && t < self.code_hi {
                    found.insert(t);
                }
            } else if w == 0x03E0_0008 || op == 2 {
                let mut j = i + 2;
                while j < n && self.words[j] == 0 {
                    j += 1;
                }
                if j < n && (op != 2 || j > i + 2) {
                    found.insert(self.lo + 4 * j as u32);
                }
            }
        }
        found.into_iter().collect()
    }

    /// Starts [`Side::entries`] leaves out: the first non-zero word after a
    /// `b` (`beq $zero, $zero`) whose delay slot is followed by padding, a
    /// thread's endless loop ending its function.
    pub fn loop_ends(&self) -> Vec<u32> {
        let n = ((self.code_hi - self.lo) >> 2) as usize;
        let mut out = Vec::new();
        for i in 0..n {
            if self.words[i] >> 16 != 0x1000 {
                continue;
            }
            let mut j = i + 2;
            while j < n && self.words[j] == 0 {
                j += 1;
            }
            if j < n && j > i + 2 {
                out.push(self.lo + 4 * j as u32);
            }
        }
        out
    }

    /// Whether the code from `va` to `end` branches below `va`: no
    /// function does, so a "likely start" that does lies inside a larger
    /// function (a `jr $ra` in the middle of a switch). A `j` there may be
    /// a tail call, so only the branches count.
    pub fn branches_back(&self, va: u32, end: u32) -> bool {
        let Some(ws) = self.run(va, (end.saturating_sub(va)) >> 2) else { return false };
        ws.iter().enumerate().any(|(k, &w)| {
            let at = va + 4 * k as u32;
            let op = w >> 26;
            let rt = (w >> 16) & 31;
            let rel = || at.wrapping_add(4).wrapping_add((((w & 0xFFFF) as i16 as i32) << 2) as u32);
            let target = match op {
                // beq bne blez bgtz and their likely forms
                4..=7 | 0x14..=0x17 => Some(rel()),
                // bltz bgez bltzl bgezl bltzal bgezal
                1 if matches!(rt, 0..=3 | 0x10..=0x13) => Some(rel()),
                _ => None,
            };
            target.is_some_and(|t| t < va)
        })
    }

    /// The sized functions inside its own section, long enough to match.
    pub fn functions(&self) -> impl Iterator<Item = &Symbol> {
        self.p.functions().filter(|f| self.owns(f.value) && f.size >= 4 * MIN_WORDS)
    }
}

/// Main's bytes (its file part), or the overlay's text and data.
pub fn span(p: &Program, overlay: Option<&str>) -> (u32, u32, u32) {
    match (overlay, &p.overlay) {
        (Some(_), Some(ov)) => (ov.text, ov.text + ov.text_size + ov.data_size, ov.text + ov.text_size),
        _ => {
            let hi = p.main_seg.vaddr + p.main_seg.filesz;
            (p.main_seg.vaddr, hi, hi)
        }
    }
}

type Sides = HashMap<(Vol, Option<&'static str>), Rc<Side>>;

thread_local! {
    static SIDES: RefCell<Sides> = RefCell::new(HashMap::new());
}

/// Drop the volume's sides read so far.
pub fn forget(v: Vol) {
    SIDES.with(|m| m.borrow_mut().retain(|k, _| k.0 != v));
}

/// The volume's section `overlay` (main for None), read once.
pub fn side(v: Vol, overlay: Option<&'static str>) -> Rc<Side> {
    if let Some(s) = SIDES.with(|m| m.borrow().get(&(v, overlay)).cloned()) {
        return s;
    }
    let p = ctx(v, overlay).p;
    let (lo, hi, code_hi) = span(&p, overlay);
    let raw = p.read(lo, (hi - lo) as usize).unwrap_or_else(|e| crate::die(&e));
    let words = raw.as_chunks::<4>().0.iter().map(|&c| u32::from_le_bytes(c)).collect();
    let s = Rc::new(Side { p, lo, hi, code_hi, words });
    SIDES.with(|m| m.borrow_mut().insert((v, overlay), s.clone()));
    s
}

/// A map that keeps its keys in the order first inserted, as Python's
/// dict does; a key removed and put back goes last.
pub struct Ordered<K, V> {
    keys: Vec<K>,
    map: HashMap<K, V>,
}

impl<K: Eq + Hash + Clone, V> Default for Ordered<K, V> {
    fn default() -> Self {
        Ordered { keys: Vec::new(), map: HashMap::new() }
    }
}

impl<K: Eq + Hash + Clone, V> Ordered<K, V> {
    pub fn get(&self, k: &K) -> Option<&V> {
        self.map.get(k)
    }

    pub fn contains(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    pub fn insert(&mut self, k: K, v: V) {
        if !self.map.contains_key(&k) {
            self.keys.push(k.clone());
        }
        self.map.insert(k, v);
    }

    pub fn entry(&mut self, k: K) -> &mut V
    where
        V: Default,
    {
        if !self.map.contains_key(&k) {
            self.keys.push(k.clone());
        }
        self.map.entry(k).or_default()
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let v = self.map.remove(k)?;
        self.keys.retain(|x| x != k);
        Some(v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().map(|k| (k, &self.map[k]))
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.keys.iter().map(|k| &self.map[k])
    }
}

/// Votes by key, each a tally of candidates.
pub type Votes<K, C> = Ordered<K, HashMap<C, u32>>;

/// The keys whose votes all name one candidate, in the order first voted.
pub fn unanimous<K: Eq + Hash + Clone, C: Copy>(votes: &Votes<K, C>) -> Vec<(K, C)> {
    votes.iter().filter(|(_, c)| c.len() == 1).map(|(k, c)| (k.clone(), *c.keys().next().unwrap())).collect()
}

/// A symbol as a vote's key: (name, address, size).
pub type SymKey = (String, u32, u32);

fn key(s: &Symbol) -> SymKey {
    (s.name.clone(), s.value, s.size)
}

/// Exact body matches: [(Infection's function, the volume's address)],
/// one to one: a function whose masked body equals a unique run at a
/// likely start in the other volume, and no other function of Infection's
/// has that body.
pub fn exact(src: &Side, dst: &Side) -> Vec<(Symbol, u32)> {
    let mut index: HashMap<Vec<u32>, Vec<u32>> = HashMap::new();
    for va in dst.entries() {
        if let Some(run) = dst.run(va, KEY as u32) {
            index.entry(normalise(run)).or_default().push(va);
        }
    }
    let mut body_of: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut by_body: HashMap<Vec<u32>, usize> = HashMap::new();
    for f in src.functions() {
        let body = normalise(src.run(f.value, f.size / 4).unwrap_or_default());
        *by_body.entry(body.clone()).or_default() += 1;
        body_of.insert(f.value, body);
    }
    let mut cand: HashMap<u32, HashSet<u32>> = HashMap::new();
    for f in src.functions() {
        let body = &body_of[&f.value];
        let Some(vas) = index.get(&body[..KEY.min(body.len())]) else { continue };
        for &va in vas {
            if let Some(run) = dst.run(va, body.len() as u32)
                && normalise(run) == *body
            {
                cand.entry(f.value).or_default().insert(va);
            }
        }
    }
    let mut used: HashMap<u32, u32> = HashMap::new();
    for vs in cand.values() {
        for &va in vs {
            *used.entry(va).or_default() += 1;
        }
    }
    let mut pairs = Vec::new();
    for f in src.functions() {
        let Some(vs) = cand.get(&f.value) else { continue };
        if vs.len() != 1 || by_body[&body_of[&f.value]] != 1 {
            continue;
        }
        let va = *vs.iter().next().unwrap();
        if used[&va] == 1 {
            pairs.push((f.clone(), va));
        }
    }
    pairs
}

/// Callee and global names voted for by the paired bodies, at aligned
/// instructions. `pairs` are (Infection's function, the volume's address,
/// the volume's words).
pub fn propagate(src: &Side, dst: &Side, pairs: &[(Symbol, u32, u32)]) -> (Votes<SymKey, u32>, Votes<SymKey, u32>) {
    let (mut calls, mut data): (Votes<SymKey, u32>, Votes<SymKey, u32>) = Default::default();
    for (f, dva, n_dst) in pairs {
        let (Some(sw), Some(dw)) = (src.run(f.value, f.size / 4), dst.run(*dva, *n_dst)) else { continue };
        let sa = addresses(sw, src.p.gp);
        let da = addresses(dw, dst.p.gp);
        for (i, j) in aligned(sw, dw) {
            if sw[i] >> 26 == 3 && dw[j] >> 26 == 3 {
                let st = ((f.value + 4 * i as u32 + 4) & 0xF000_0000) | ((sw[i] & 0x03FF_FFFF) << 2);
                let dt = ((dva + 4 * j as u32 + 4) & 0xF000_0000) | ((dw[j] & 0x03FF_FFFF) << 2);
                if let Some((s, 0)) = src.p.symbol_at(st, 0)
                    && src.owns(st)
                    && dst.owns(dt)
                {
                    *calls.entry(key(s)).entry(dt).or_default() += 1;
                }
            }
            let (Some(&s_addr), Some(&d_addr)) = (sa.get(&i), da.get(&j)) else { continue };
            if sw[i] >> 26 != dw[j] >> 26 {
                continue;
            }
            let Some((sym, off)) = src.p.symbol_at(s_addr, 0) else { continue };
            if sym.kind == STT_OBJECT {
                *data.entry(key(sym)).entry(d_addr.wrapping_sub(off)).or_default() += 1;
            } else if sym.kind == crate::elf::STT_FUNC && off == 0 && src.owns(s_addr) && dst.owns(d_addr) {
                // A function's address taken in code - a thread entry, a
                // callback - names it just as a jal would.
                *calls.entry(key(sym)).entry(d_addr).or_default() += 1;
            }
        }
    }
    (calls, data)
}

/// `difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()`; 0 for
/// an empty side.
pub fn likeness(a: &[u32], b: &[u32]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let m: usize = matching_blocks(a, b).iter().map(|x| x.2).sum();
    2.0 * m as f64 / (a.len() + b.len()) as f64
}

/// The longest run of (Infection's address, the volume's) anchors, sorted
/// by Infection's, whose other addresses also increase.
fn in_order(mut anchors: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    anchors.sort();
    let (mut tails, mut at): (Vec<u32>, Vec<usize>) = (Vec::new(), Vec::new());
    let mut prev = vec![usize::MAX; anchors.len()];
    for (k, &(_, d)) in anchors.iter().enumerate() {
        let i = tails.partition_point(|&t| t < d);
        if i == tails.len() {
            tails.push(d);
            at.push(k);
        } else {
            tails[i] = d;
            at[i] = k;
        }
        prev[k] = if i > 0 { at[i - 1] } else { usize::MAX };
    }
    let mut chain = Vec::new();
    let mut k = at.last().copied().unwrap_or(usize::MAX);
    while k != usize::MAX {
        chain.push(anchors[k]);
        k = prev[k];
    }
    chain.reverse();
    chain
}

/// The items whose keys lie strictly between `lo` and `hi`.
fn between<'a, T>(items: &'a [T], keys: &[u32], lo: u32, hi: u32) -> &'a [T] {
    let (a, b) = (keys.partition_point(|&k| k <= lo), keys.partition_point(|&k| k < hi));
    if b > a { &items[a..b] } else { &[] }
}

/// Pairs [(Infection's function, the volume's address)] from the gaps
/// between anchors. The linker kept the source files' order, so the
/// functions between two paired neighbours are the same functions in both
/// volumes; in each gap the remaining functions are aligned to the likely
/// starts by the shape of their code, in order: one for one when the gap
/// holds as many of each and every pair scores at least EVEN, otherwise
/// only pairs scoring ALONE or more.
fn by_order(
    src: &Side,
    dst: &Side,
    named: &BTreeMap<u32, (String, u32, &'static str)>,
    by_name: &HashMap<String, Symbol>,
    starts: &[u32],
    extent: &impl Fn(u32, u32) -> u32,
) -> Vec<(Symbol, u32)> {
    let anchors: Vec<(u32, u32)> =
        named.iter().filter_map(|(&va, (n, _, _))| by_name.get(n).map(|f| (f.value, va))).collect();
    let mut chain = vec![(src.lo - 4, dst.lo - 4)];
    chain.extend(in_order(anchors));
    chain.push((src.code_hi, dst.code_hi));
    let taken: HashSet<&str> = named.values().map(|(n, _, _)| n.as_str()).collect();
    let free: Vec<&Symbol> = src
        .p
        .functions()
        .filter(|f| src.owns(f.value) && f.value < src.code_hi && f.size >= 8 && !taken.contains(f.name.as_str()))
        .collect();
    let fvals: Vec<u32> = free.iter().map(|f| f.value).collect();
    let mut out = Vec::new();
    for w in chain.windows(2) {
        let ((s1, d1), (s2, d2)) = (w[0], w[1]);
        let fs = between(&free, &fvals, s1, s2);
        let ds: Vec<u32> =
            between(starts, starts, d1, d2).iter().copied().filter(|va| !named.contains_key(va)).collect();
        if fs.is_empty() || ds.is_empty() || fs.len() * ds.len() > 4000 {
            continue;
        }
        let (n, m) = (fs.len(), ds.len());
        let mut cache: HashMap<(usize, usize), f64> = HashMap::new();
        let mut score = |i: usize, j: usize| -> f64 {
            *cache.entry((i, j)).or_insert_with(|| {
                let f = fs[i];
                let sw = src.run(f.value, f.size / 4).unwrap_or_default();
                let dw = dst.run(ds[j], extent(ds[j], f.size / 4));
                if (sw.len() as u32) < MIN_WORDS {
                    let ok = dw.is_some_and(|dw| normalise(&dw[..sw.len().min(dw.len())]) == normalise(sw));
                    if ok { 1.0 } else { 0.0 }
                } else {
                    likeness(&shape(sw), &shape(dw.unwrap_or_default()))
                }
            })
        };
        if n == m && (0..n).all(|i| score(i, i) >= EVEN) {
            out.extend((0..n).map(|i| (fs[i].clone(), ds[i])));
            continue;
        }
        // The best monotone pairing, each pair scoring ALONE or more.
        let mut best = vec![vec![0.0f64; m + 1]; n + 1];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                let mut b = best[i + 1][j].max(best[i][j + 1]);
                let sc = score(i, j);
                if sc >= ALONE {
                    b = b.max(sc + best[i + 1][j + 1]);
                }
                best[i][j] = b;
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            let sc = score(i, j);
            if sc >= ALONE && best[i][j] == sc + best[i + 1][j + 1] {
                out.push((fs[i].clone(), ds[j]));
                i += 1;
                j += 1;
            } else if best[i][j] == best[i + 1][j] {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
    out
}

/// A row of the sidecar: (section, address, size, FUNC or OBJECT, name,
/// the pass that found it).
#[derive(Clone, Debug)]
pub struct Row {
    pub sec: &'static str,
    pub va: u32,
    pub size: u32,
    pub kind: &'static str,
    pub name: String,
    pub how: &'static str,
}

/// Per section: (its name, Infection's functions, exact, call, order,
/// globals).
pub type Stats = Vec<(&'static str, usize, usize, usize, usize, usize)>;

/// Main, then each overlay.
pub const SECTIONS: [Option<&str>; 5] = [None, Some("gcmn"), Some("demo"), Some("desktop"), Some("toppage")];

/// The first passes, a section at a time: exact bodies, then for a few
/// rounds the callees and globals the paired bodies name (`propagate`)
/// and the functions between paired neighbours (`by_order`).
pub fn transfer(v: Vol) -> (Vec<Row>, Stats) {
    const ROUNDS: usize = 6;
    let (mut rows, mut stats) = (Vec::new(), Vec::new());
    for ov in SECTIONS {
        let sec = ov.unwrap_or("main");
        let (src, dst) = (side(Vol::Inf, ov), side(v, ov));
        let starts = dst.entries();
        let next_start = |va: u32| -> u32 {
            let i = starts.partition_point(|&s| s <= va);
            starts.get(i).copied().unwrap_or(dst.code_hi)
        };
        // The volume's own size: to the next likely start, less padding.
        let span = |va: u32| -> u32 {
            let mut end = next_start(va);
            while end > va + 4 && dst.word_at(end - 4) == 0 {
                end -= 4;
            }
            end - va
        };
        // To the next likely function start, within reason.
        let extent = |va: u32, n_src: u32| -> u32 { 1.max(((next_start(va) - va) / 4).min(2 * n_src + 16)) };
        let pairs = exact(&src, &dst);
        let mut named: BTreeMap<u32, (String, u32, &'static str)> =
            pairs.iter().map(|(f, va)| (*va, (f.name.clone(), f.size, "exact"))).collect();
        let mut taken: HashSet<String> = pairs.iter().map(|(f, _)| f.name.clone()).collect();
        let mut by_name: HashMap<String, Symbol> = HashMap::new();
        for f in src.p.functions().filter(|f| src.owns(f.value)) {
            by_name.insert(f.name.clone(), f.clone());
        }
        let mut work: Vec<(Symbol, u32, u32)> = pairs.iter().map(|(f, va)| (f.clone(), *va, f.size / 4)).collect();
        let mut done: HashSet<u32> = pairs.iter().map(|p| p.1).collect();
        let mut objects: BTreeMap<u32, (String, u32)> = BTreeMap::new();
        let (mut n_call, mut n_order) = (0, 0);
        for _ in 0..ROUNDS {
            let (calls, data) = propagate(&src, &dst, &work);
            for ((name, _, size), va) in unanimous(&data) {
                if dst.p.mapped(va) && !objects.contains_key(&va) {
                    objects.insert(va, (name, size));
                }
            }
            let mut new = Vec::new();
            for ((name, _, size), va) in unanimous(&calls) {
                if named.contains_key(&va) || taken.contains(&name) {
                    continue;
                }
                named.insert(va, (name.clone(), size, "call"));
                taken.insert(name.clone());
                n_call += 1;
                if let Some(f) = by_name.get(&name)
                    && done.insert(va)
                {
                    new.push((f.clone(), va, extent(va, f.size / 4)));
                }
            }
            for (f, va) in by_order(&src, &dst, &named, &by_name, &starts, &extent) {
                if named.contains_key(&va) || taken.contains(&f.name) {
                    continue;
                }
                named.insert(va, (f.name.clone(), f.size, "order"));
                taken.insert(f.name.clone());
                n_order += 1;
                if done.insert(va) {
                    new.push((f.clone(), va, extent(va, f.size / 4)));
                }
            }
            if new.is_empty() {
                break;
            }
            work.extend(new);
        }
        for (&va, (name, size, how)) in &named {
            let size = if *how == "exact" { *size } else { span(va) };
            rows.push(Row { sec, va, size, kind: "FUNC", name: name.clone(), how });
        }
        for (&va, (name, size)) in &objects {
            rows.push(Row { sec, va, size: *size, kind: "OBJECT", name: name.clone(), how: "data" });
        }
        stats.push((sec, src.functions().count(), pairs.len(), n_call, n_order, objects.len()));
    }
    (rows, stats)
}

#[cfg(test)]
mod tests {
    use super::matching_blocks;

    #[test]
    fn blocks_as_difflib_gives_them() {
        // difflib.SequenceMatcher(None, "abxcd", "abcd", autojunk=False)
        let a: Vec<char> = "abxcd".chars().collect();
        let b: Vec<char> = "abcd".chars().collect();
        assert_eq!(matching_blocks(&a, &b), vec![(0, 0, 2), (3, 2, 2), (5, 4, 0)]);
        // The earliest of two equally long matches: "qabxyab" against "ab".
        let a: Vec<char> = "qabxyab".chars().collect();
        let b: Vec<char> = "ab".chars().collect();
        assert_eq!(matching_blocks(&a, &b), vec![(1, 0, 2), (7, 2, 0)]);
    }
}
