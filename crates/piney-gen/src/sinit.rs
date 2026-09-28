//! What a volume's static initialisers fill at start, run in the EE
//! interpreter (`piney-eemu`): `saveSysMsg` (`__sinit_sdmng.cpp`) and the
//! mail tables' replies (`__sinit_mailtbl.cpp`, `__sinit_mailtblp.cpp`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use piney_eemu::cpu::Features;
use piney_eemu::machine::Machine;

use crate::locate::carried;
use crate::program::Program;
use crate::volume::{Vol, ctx};
use crate::xfer;

const SAVE_SYS_MSG_WORDS: u32 = 64;
const MAIL_SIZE: u32 = 0x48;
const REMAIL_SIZE: u32 = 0x14;
const ONE_RES: u32 = 0x20;
const TWO_RES: u32 = 0x34;
/// A marker word: MARK | row << 4 | field.
const MARK: u32 = 0x7E00_0000;
/// The carry's section for desktop.prg.
const DESKTOP: u32 = 3;
/// `MailTbl`, `MailTblp` in Infection.
const INF_MAIL_TBL: [u32; 2] = [0x0041_DFD0, 0x0042_5280];

/// The program's memory (every loaded segment and the overlay) in an EE
/// machine, eemu's C library stand-ins where the program names them.
pub(crate) fn machine(p: &Program, features: Features) -> Machine {
    let mut m = Machine::new(features);
    for s in &p.elf.segments {
        if s.kind == 1 && s.filesz != 0 {
            let (a, o) = (s.vaddr as usize, s.offset as usize);
            m.ram[a..a + s.filesz as usize].copy_from_slice(&p.elf.data[o..o + s.filesz as usize]);
        }
    }
    if let Some(ov) = &p.overlay {
        let a = ov.base as usize;
        m.ram[a..a + ov.data.len()].copy_from_slice(&ov.data);
    }
    m.gp = p.gp.unwrap_or(0);
    for (name, h) in piney_eemu::hle::TABLE {
        if let Some(s) = p.symbol_named(name) {
            m.hle(s.value, h);
        }
    }
    m
}

fn load(m: &Machine, a: u32) -> u32 {
    u32::from_le_bytes(m.ram[a as usize..a as usize + 4].try_into().unwrap())
}

fn store(m: &mut Machine, a: u32, v: u32) {
    m.ram[a as usize..a as usize + 4].copy_from_slice(&v.to_le_bytes());
}

thread_local! {
    static SAVE_SYS: RefCell<HashMap<Vol, Rc<Vec<u32>>>> = RefCell::new(HashMap::new());
    static MAIL: RefCell<HashMap<(Vol, bool), Rc<MailLinks>>> = RefCell::new(HashMap::new());
}

/// `saveSysMsg` as `__sinit_sdmng.cpp` fills it: each message's address,
/// 0 for none.
pub fn save_sys_msg(v: Vol) -> Rc<Vec<u32>> {
    if let Some(x) = SAVE_SYS.with(|m| m.borrow().get(&v).cloned()) {
        return x;
    }
    let c = ctx(v, None);
    let mut m = machine(&c.p, Features { vu: true, ..Features::default() });
    let table = c.p.symbol_named("saveSysMsg").unwrap_or_else(|| crate::die("no saveSysMsg")).value;
    m.ram[table as usize..(table + 4 * SAVE_SYS_MSG_WORDS) as usize].fill(0);
    let init = c.p.symbol_named("__sinit_sdmng.cpp").unwrap_or_else(|| crate::die("no __sinit_sdmng.cpp")).value;
    m.call(init, &[], 5_000_000).unwrap_or_else(|e| crate::die(&format!("{}: __sinit_sdmng.cpp: {e:?}", v.tag())));
    let out = Rc::new((0..SAVE_SYS_MSG_WORDS).map(|i| load(&m, table + 4 * i)).collect::<Vec<_>>());
    SAVE_SYS.with(|x| x.borrow_mut().insert(v, out.clone()));
    out
}

/// The words of the program's code or data at `va`.
fn run(p: &Program, va: u32, n: u32) -> Vec<u32> {
    let b = p.read(va, 4 * n as usize).unwrap_or_else(|e| crate::die(&e));
    b.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
}

/// {Infection's name: (address, end)} of the volume's desktop.prg static
/// initialisers, by their places in the overlay's constructor list.
fn initialisers(v: Vol) -> HashMap<String, (u32, u32)> {
    let inf = ctx(Vol::Inf, Some("desktop")).p;
    let p = ctx(v, Some("desktop")).p;
    let listed = |q: &Program| {
        let ov = q.overlay.as_ref().unwrap();
        (ov.ctor_start..ov.ctor_end).step_by(4).map(|a| q.u32(a).unwrap()).collect::<Vec<_>>()
    };
    let names: Vec<String> = listed(&inf)
        .into_iter()
        .map(|w| match inf.symbol_at(w, 0) {
            Some((s, 0)) => s.name.clone(),
            Some((s, off)) => format!("{}+0x{off:x}", s.name),
            None => String::new(),
        })
        .collect();
    let mine = listed(&p);
    if names.len() != mine.len() {
        crate::die(&format!("{}: {} desktop initialisers, Infection {}", v.tag(), mine.len(), names.len()));
    }
    let mut ends = mine.clone();
    ends.sort();
    ends.push(p.overlay.as_ref().unwrap().ctor_start);
    names
        .into_iter()
        .zip(mine)
        .map(|(n, w)| {
            let k = ends.iter().position(|&e| e == w).unwrap();
            (n, (w, ends[k + 1]))
        })
        .collect()
}

pub struct MailLinks {
    /// `ReMail`'s address and its rows.
    pub remail: u32,
    pub remail_rows: u32,
    /// Per mail, the `ReMail` rows its `oneRes` and `twoRes` copy, -1 none.
    pub links: Vec<(i32, i32)>,
}

/// The mail tables' replies, `MailTbl`'s or `MailTblp`'s.
pub fn mail_links(v: Vol, parody: bool) -> Rc<MailLinks> {
    if let Some(x) = MAIL.with(|m| m.borrow().get(&(v, parody)).cloned()) {
        return x;
    }
    let p = ctx(v, Some("desktop")).p;
    let inits = initialisers(v);
    let name = |parody: bool| if parody { "__sinit_mailtblp.cpp" } else { "__sinit_mailtbl.cpp" };
    // The run of addresses the initialiser builds right below its MailTbl:
    // the ReMail rows it copies.
    let remail = |parody: bool| -> (u32, u32) {
        let (a, end) = inits[name(parody)];
        let tbl = carried(v, DESKTOP, INF_MAIL_TBL[usize::from(parody)])
            .unwrap_or_else(|| crate::die(&format!("{}: MailTbl not carried", v.tag())));
        let mut below: Vec<u32> =
            xfer::addresses(&run(&p, a, (end - a) / 4), p.gp).into_values().filter(|&x| x < tbl).collect();
        below.sort();
        let mut lo = *below.last().unwrap_or_else(|| crate::die("no ReMail below MailTbl"));
        for &x in below.iter().rev() {
            if lo - x > 0x100 {
                break;
            }
            lo = x;
        }
        (tbl, lo)
    };
    let (tbl, rem) = remail(parody);
    let nre = (tbl - rem) / REMAIL_SIZE;
    let nmail = (remail(true).1 - remail(false).0) / MAIL_SIZE;
    let mut m = machine(&p, Features::default());
    for k in 0..nre {
        for f in 0..5 {
            store(&mut m, rem + k * REMAIL_SIZE + 4 * f, MARK | k << 4 | f);
        }
    }
    for i in 0..nmail {
        for slot in [ONE_RES, TWO_RES] {
            for f in 0..5 {
                store(&mut m, tbl + i * MAIL_SIZE + slot + 4 * f, 0xFFFF_FFFF);
            }
        }
    }
    let init = inits[name(parody)].0;
    m.call(init, &[], 5_000_000).unwrap_or_else(|e| crate::die(&format!("{}: {}: {e:?}", v.tag(), name(parody))));
    let mut links = Vec::new();
    for i in 0..nmail {
        let mut row = [0i32; 2];
        for (s, slot) in [ONE_RES, TWO_RES].into_iter().enumerate() {
            let w: Vec<u32> = (0..5).map(|f| load(&m, tbl + i * MAIL_SIZE + slot + 4 * f)).collect();
            if w.iter().all(|&x| x == 0xFFFF_FFFF) {
                row[s] = -1;
                continue;
            }
            let k = (w[0] >> 4) & 0xFFFFF;
            if !w.iter().enumerate().all(|(f, &x)| x == MARK | k << 4 | f as u32) || k >= nre {
                crate::die(&format!("{}: mail {i}: reply slot 0x{slot:x} is not a whole ReMail row: {w:x?}", v.tag()));
            }
            row[s] = k as i32;
        }
        links.push((row[0], row[1]));
    }
    let out = Rc::new(MailLinks { remail: rem, remail_rows: nre, links });
    MAIL.with(|x| x.borrow_mut().insert((v, parody), out.clone()));
    out
}
