//! The walking PCs' tables that `ccRtownPC::ccRtownPC` and its set-ups read
//! (gcmn rtownnpc.cpp), found through their code: Mutation moved the TV
//! PCs' weapons and added the sign PCs' (rows 180 on), the SEARCH PCs'
//! landmarks by stage and their dummies, and `ccRegisterRandomNpc`'s rows
//! past the fifty walking PCs (main). Each function's tables are its
//! `lui`/`addiu` pairs, as `crate::race` reads them.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::race::{body, code, named, pair, start_of, tables};
use crate::volume::{Ctx, Vol};

/// Where the walking PCs' moved and new tables are (MUT addresses in the
/// docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct PcAddrs {
    /// The TV PCs' weapons, rows 159 on (MUT gcmn 0x0061c250).
    pub tvpc: u32,
    /// From Mutation on: the sign PCs' weapons, rows 180 on (0x0061c280).
    pub sign: Option<u32>,
    /// The SEARCH PCs' marks by row - 159, each by stage (0x0061c890).
    pub marks: Option<u32>,
    /// Their dummies from stage 5 (0x0061c8c0).
    pub dummies: Option<u32>,
    /// `ccRegisterRandomNpc` (MUT main 0x001ccdf0): its rows table, read
    /// from index 50, and the count it draws from (56).
    pub extra: Option<(u32, u32)>,
}

thread_local! {
    static FOUND: RefCell<HashMap<Vol, Result<PcAddrs, String>>> = RefCell::new(HashMap::new());
}

/// `addiu $v0, $v0, -159`, `-180` and `addiu $v0, $a0, -159`: the row
/// less the table's first, before each table's load.
const TVPC_ROW: u32 = 0x2442_ff61;
const SIGN_ROW: u32 = 0x2442_ff4c;
const SEARCH_ROW: u32 = 0x2482_ff61;
/// `div $v0, $v1`, after `li $v1, n`: `ccRand() % n`.
const DIV_V0_V1: u32 = 0x0043_001a;

/// The first table built after the first `marker` word of `f`.
fn after(f: &[u32], marker: u32) -> Option<u32> {
    let i = f.iter().position(|&w| w == marker)?;
    (i..f.len()).find_map(|k| pair(f, k))
}

/// The words of main's function `name`, up to its `jr $ra` and the delay
/// slot.
fn main_words(c: &Ctx, name: &str) -> Result<Vec<u32>, String> {
    let f = c.p.symbol_named(name).ok_or_else(|| format!("no {name}"))?.value;
    let mut out = Vec::new();
    for k in 0..4096 {
        let w = c.p.u32(f + 4 * k)?;
        out.push(w);
        if out.len() >= 2 && out[out.len() - 2] == 0x03e0_0008 {
            return Ok(out);
        }
    }
    Err(format!("{name} has no end"))
}

fn find(c: &Ctx) -> Result<PcAddrs, String> {
    let (lo, words) = code(c)?;
    const CT: &str = "__ct__9ccRtownPCFP7ccEntry";
    let ct = named(c, lo, &words, CT)?;
    let ct_at = ((c.p.symbol_named(CT).ok_or("no ccRtownPC")?.value - lo) / 4) as usize;
    let in_ct = ct_at..ct_at + ct.len();
    let tvpc = after(ct, TVPC_ROW).ok_or("ccRtownPC builds no TV PCs' weapons")?;
    let sign = after(ct, SIGN_ROW);
    let marks = after(ct, SEARCH_ROW);
    // The SEARCH set-up: the other function that builds the marks; its
    // next table is the dummies.
    let dummies = marks.and_then(|m| {
        let at = (0..words.len()).find(|&i| pair(&words, i) == Some(m) && !in_ct.contains(&i))?;
        let f = body(lo, &words, start_of(lo, &words, at)?);
        let t = tables(c, lo, f);
        t.iter().position(|&a| a == m).and_then(|k| t.get(k + 1).copied())
    });
    let reg = main_words(c, "ccRegisterRandomNpc__Fi")?;
    let moduli: Vec<u32> = (1..reg.len())
        .filter(|&k| reg[k] == DIV_V0_V1 && reg[k - 1] >> 16 == 0x2403)
        .map(|k| reg[k - 1] & 0xffff)
        .collect();
    let extra = match (moduli.iter().min(), moduli.iter().max()) {
        (Some(&lo_n), Some(&hi_n)) if hi_n > lo_n => {
            let base = (0..reg.len()).find_map(|k| pair(&reg, k)).ok_or("ccRegisterRandomNpc builds no table")?;
            Some((base + 4 * lo_n, hi_n - lo_n))
        }
        _ => None,
    };
    Ok(PcAddrs { tvpc, sign, marks, dummies, extra })
}

/// The volume's walking PCs' table addresses.
pub fn addrs(c: &Ctx) -> Result<PcAddrs, String> {
    if let Some(r) = FOUND.with(|m| m.borrow().get(&c.volume).cloned()) {
        return r;
    }
    let r = find(c);
    FOUND.with(|m| m.borrow_mut().insert(c.volume, r.clone()));
    r
}
