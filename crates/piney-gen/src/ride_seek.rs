//! The riding Grunty's search in a field (Mutation on), found through its
//! code: the constructor's first two tables past `pcgsTbl` are the search
//! type by kind and the range by type; from `ControlMove` to
//! `PadLeverPower` the code builds four line tables, in the order the found,
//! nothing, cancel and near lines are first used. None on Infection.

use crate::race::{code, named, tables};
use crate::volume::{Ctx, Vol};

/// The search's table addresses (MUT gcmn addresses in the docs).
#[derive(Clone, Debug, Default)]
pub struct SeekAddrs {
    /// The constructor (MUT 0x0052dd10): the type by kind (0x0061d180) and
    /// the range by type (0x0061d1a8).
    pub types: u32,
    pub ranges: u32,
    /// The lines by kind: found (0x00684b50), nothing (0x00684b80),
    /// cancelled (0x00684bb0), near (0x00684be0).
    pub found: u32,
    pub none: u32,
    pub cancel: u32,
    pub near: u32,
}

/// The search's tables on the volume (Mutation on).
pub fn addrs(c: &Ctx) -> Result<SeekAddrs, String> {
    if c.volume == Vol::Inf {
        return Err("Infection's ride has no search".into());
    }
    let (lo, code) = code(c)?;
    let named_va = |n: &str| c.p.symbol_named(n).map(|s| s.value).ok_or_else(|| format!("no {n}"));
    let unnamed = |v: Vec<u32>| v.into_iter().filter(|&a| !matches!(c.p.symbol_at(a, 0), Some((_, 0)))).collect();
    let ct: Vec<u32> = unnamed(tables(c, lo, named(c, lo, &code, "__ct__11ccPuccigusoFi")?));
    let (from, to) = (named_va("ControlMove__11ccPuccigusoFv")?, named_va("PadLeverPower__11ccPuccigusoFf")?);
    let span = &code[((from - lo) / 4) as usize..((to - lo) / 4) as usize];
    // The stretch holds ControlMove and the run; tables() reads it whole.
    let lines: Vec<u32> = unnamed(tables(c, lo, span)).into_iter().filter(|a| Some(a) != ct.first()).collect();
    let pick = |v: &[u32], k: usize, what: &str| {
        v.get(k).copied().ok_or_else(|| format!("the ride's search: too few {what} tables ({v:x?})"))
    };
    Ok(SeekAddrs {
        types: pick(&ct, 0, "constructor")?,
        ranges: pick(&ct, 1, "constructor")?,
        found: pick(&lines, 0, "line")?,
        none: pick(&lines, 1, "line")?,
        cancel: pick(&lines, 2, "line")?,
        near: pick(&lines, 3, "line")?,
    })
}
