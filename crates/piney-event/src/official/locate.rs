//! Finding the event tables through the game's code, without symbols.
//!
//! The same patterns hold on all four volumes (checked against the symbol
//! table on Infection and `tools/xfer.py`'s names on the others):
//!
//! - `ccEvent::Execute` and `ccEvent::CheckOpen` are the only two loops of
//!   the form `lw a,0(p); addiu b,a,2; sw b,0(p); lh c,0(a); beqz c;
//!   sltiu at,c,N` (step the script pointer, stop at 0, bound the code).
//!   N is 41 for CheckOpen and 168, 169 or 170 for Execute; that count
//!   tells the volume. The jump table is the `lui`/`addiu` pair after it.
//! - `eventTbl` is the table `eventSub` and `ccEventFlagSet` index right
//!   after dividing the event number by 50 (`lui 0x51eb`); both call
//!   CheckOpen.
//! - `evMsgTblp` and `evMsgTbl` are the first two addresses Execute's
//!   message case (code 6) builds: the Parody Mode table first.

use crate::official::Error;
use crate::official::elf::Executable;

/// Which volume's instruction encoding a script uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// `.hack//Infection`: 168 Execute entries, code 99 without an operand.
    Infection,
    /// `.hack//Mutation`: 169 entries, 99 takes a level, 168 added.
    Mutation,
    /// `.hack//Outbreak`: 170 entries; 169's entry is the default, so the game steps over it.
    Outbreak,
    /// `.hack//Quarantine`: 170 entries, 169 has a case.
    Quarantine,
}

impl Dialect {
    pub fn volume(self) -> i32 {
        match self {
            Dialect::Infection => 1,
            Dialect::Mutation => 2,
            Dialect::Outbreak => 3,
            Dialect::Quarantine => 4,
        }
    }
}

/// Where the tables are in one executable.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub dialect: Dialect,
    /// `ccEvent::Execute`'s dispatch loop head and jump table.
    pub execute_loop: u32,
    pub execute_table: u32,
    pub execute_count: u32,
    /// `ccEvent::CheckOpen`'s entry point.
    pub check_open: u32,
    pub event_tbl: u32,
    pub msg_tbl: u32,
    pub msg_tbl_parody: u32,
}

fn rs(w: u32) -> u32 {
    (w >> 21) & 31
}
fn rt(w: u32) -> u32 {
    (w >> 16) & 31
}
fn op(w: u32) -> u32 {
    w >> 26
}
fn simm(w: u32) -> i32 {
    (w & 0xffff) as u16 as i16 as i32
}

/// The dispatch loops: (loop head, N).
fn dispatch_loops(code: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for i in 0..code.len().saturating_sub(10) {
        let w = |k: usize| code[i + k].1;
        let w0 = w(0);
        if op(w0) != 0x23 || w0 & 0xffff != 0 {
            continue;
        }
        let (p, a) = (rs(w0), rt(w0));
        let w1 = w(1);
        if op(w1) != 0x09 || rs(w1) != a || w1 & 0xffff != 2 {
            continue;
        }
        let b = rt(w1);
        let w2 = w(2);
        if op(w2) != 0x2b || rs(w2) != p || rt(w2) != b || w2 & 0xffff != 0 {
            continue;
        }
        let w3 = w(3);
        if op(w3) != 0x21 || rs(w3) != a || w3 & 0xffff != 0 {
            continue;
        }
        let c = rt(w3);
        let w4 = w(4);
        if op(w4) != 0x04 || rs(w4) != c || rt(w4) != 0 {
            continue;
        }
        let mut k = 5;
        while w(k) == 0 && k < 8 {
            k += 1;
        }
        let wk = w(k);
        if op(wk) != 0x0b || rs(wk) != c || rt(wk) != 1 {
            continue;
        }
        out.push((code[i].0, wk & 0xffff));
    }
    out
}

/// `lui`/`addiu` pairs in `[lo, hi)`: (va of the addiu, address built).
fn address_pairs(exe: &Executable, lo: u32, hi: u32) -> Vec<(u32, u32)> {
    let mut hi_regs = [None::<u32>; 32];
    let mut out = Vec::new();
    let mut va = lo;
    while va < hi {
        let Some(w) = exe.u32(va) else { break };
        match op(w) {
            0x0f => hi_regs[rt(w) as usize] = Some((w & 0xffff) << 16),
            0x09 => {
                if let Some(h) = hi_regs[rs(w) as usize] {
                    out.push((va, h.wrapping_add(simm(w) as u32)));
                }
            }
            _ => {}
        }
        va += 4;
    }
    out
}

/// The start of the function holding `va`: the nearest `addiu sp,sp,-n` before it.
fn function_start(exe: &Executable, mut va: u32) -> Option<u32> {
    for _ in 0..0x4000 {
        let w = exe.u32(va)?;
        if w >> 16 == 0x27bd && w & 0x8000 != 0 {
            return Some(va);
        }
        va = va.checked_sub(4)?;
    }
    None
}

pub fn locate(exe: &Executable) -> Result<Layout, Error> {
    let code: Vec<(u32, u32)> = exe.code().collect();
    let loops = dispatch_loops(&code);
    let find = |pred: &dyn Fn(u32) -> bool, what: &str| -> Result<(u32, u32), Error> {
        let hits: Vec<_> = loops.iter().copied().filter(|&(_, n)| pred(n)).collect();
        match hits[..] {
            [one] => Ok(one),
            _ => Err(Error::Format(format!("{} candidates for {what}", hits.len()))),
        }
    };
    let (co_loop, _) = find(&|n| n == 41, "CheckOpen")?;
    let (ex_loop, ex_count) = find(&|n| (160..=200).contains(&n), "Execute")?;
    let check_open = function_start(exe, co_loop).ok_or_else(|| Error::Format("no CheckOpen prologue".into()))?;

    let table = address_pairs(exe, ex_loop, ex_loop + 0x40)
        .first()
        .map(|p| p.1)
        .ok_or_else(|| Error::Format("no jump table after Execute's bound".into()))?;
    let cases: Vec<u32> = (0..ex_count).map(|i| exe.u32(table + 4 * i).unwrap_or(0)).collect();
    let dialect = match ex_count {
        168 => Dialect::Infection,
        169 => Dialect::Mutation,
        170 if cases[169] == ex_loop => Dialect::Outbreak,
        170 => Dialect::Quarantine,
        n => return Err(Error::Format(format!("Execute has {n} cases, which no volume has"))),
    };

    // eventTbl: after the divide by 50 in CheckOpen's callers.
    let jal = 0x0c00_0000 | ((check_open >> 2) & 0x03ff_ffff);
    let mut tbls: Vec<u32> = Vec::new();
    for &(site, w) in &code {
        if w != jal {
            continue;
        }
        let lo = site.saturating_sub(0x100);
        let magic = (lo..site).step_by(4).rfind(|&va| exe.u32(va).is_some_and(|w| w & 0xffe0_ffff == 0x3c00_51eb));
        if let Some(m) = magic
            && let Some(&(_, t)) = address_pairs(exe, lo, site).iter().find(|(va, _)| *va > m)
            && !tbls.contains(&t)
        {
            tbls.push(t);
        }
    }
    let event_tbl = match tbls[..] {
        [one] => one,
        _ => return Err(Error::Format(format!("{} candidates for eventTbl", tbls.len()))),
    };

    // The message tables, from the message case.
    let case6 = cases[6];
    let end = cases.iter().copied().filter(|&c| c > case6).min().unwrap_or(case6 + 0x1000);
    let mut msgs: Vec<u32> = Vec::new();
    for (_, a) in address_pairs(exe, case6, end) {
        if !msgs.contains(&a) {
            msgs.push(a);
        }
    }
    if msgs.len() < 2 {
        return Err(Error::Format("the message case builds fewer than two tables".into()));
    }
    Ok(Layout {
        dialect,
        execute_loop: ex_loop,
        execute_table: table,
        execute_count: ex_count,
        check_open,
        event_tbl,
        msg_tbl_parody: msgs[0],
        msg_tbl: msgs[1],
    })
}
