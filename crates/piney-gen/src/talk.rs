//! The records the talk pages reach through the pointers the game keeps:
//! `npcTbl`'s `base.msg` (which the save's party records hold too),
//! `spcMsgTbl`, the present tables, the breeder's and the Grunty's records,
//! `errorData` and `kiteSelfTalk`: every record array and pointer table
//! reachable from them, with its address, since the port looks records up by
//! the addresses the save stores. Infection's DWARF declares each; a later
//! volume's are carried, or told by shape (`emode`, a name and a text).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::dtype::dwarf;
use crate::dwarf::Ty;
use crate::layout::{Value, text_at};
use crate::locate::{carried, extract};
use crate::manifest::{ev_msg, groups};
use crate::text::plausible;
use crate::volume::{Ctx, Vol, ctx};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rec,
    Ptr,
}

const RECORD: u32 = 12;

/// {address: (kind, count)}.
type Objects = Rc<BTreeMap<u32, (Kind, u32)>>;

thread_local! {
    static INF: RefCell<Option<Objects>> = const { RefCell::new(None) };
    static CLOSURE: RefCell<HashMap<Vol, Objects>> = RefCell::new(HashMap::new());
}

fn classify(t: &Ty) -> Option<(Kind, Option<u32>)> {
    let dw = dwarf();
    let (n, inner) = match t {
        Ty::Array(n, inner) => (*n, &**inner),
        t => (Some(1), t),
    };
    let named = |t: &Ty| matches!(t, Ty::Named(i) if dw.die(*i).name() == Some("ccEvMsgData"));
    if named(inner) {
        return Some((Kind::Rec, n));
    }
    if let Ty::Ptr(x) = inner {
        let mut x = &**x;
        while let Ty::Ptr(y) = x {
            x = y;
        }
        if named(x) {
            return Some((Kind::Ptr, n));
        }
    }
    None
}

/// Infection's message globals: {address: (kind, count)}.
fn inf_objects() -> Objects {
    if let Some(x) = INF.with(|i| i.borrow().clone()) {
        return x;
    }
    let mut out: BTreeMap<u32, (Kind, u32)> = BTreeMap::new();
    for (addr, t) in dwarf().globals() {
        let Some((kind, n)) = classify(&t) else { continue };
        let n = match n {
            Some(n) => n,
            None => {
                // An extern declaration: the symbol's size.
                let ov = if addr < 0x0040_0000 { None } else { Some("gcmn") };
                let p = ctx(Vol::Inf, ov).p;
                match p.symbol_at(addr, 0) {
                    Some((s, 0)) if s.size != 0 => s.size / if kind == Kind::Rec { RECORD } else { 4 },
                    _ => continue,
                }
            }
        };
        match out.get(&addr) {
            Some(&(_, m)) if m >= n => {}
            _ => {
                out.insert(addr, (kind, n));
            }
        }
    }
    let out = Rc::new(out);
    INF.with(|i| *i.borrow_mut() = Some(out.clone()));
    out
}

fn text(c: &Ctx, va: u32) -> Option<Vec<u8>> {
    text_at(c, va, 512).ok().filter(|t| plausible(t))
}

fn mapped(c: &Ctx, va: u32) -> bool {
    c.p.u32(va).is_ok()
}

fn is_record(c: &Ctx, va: u32) -> bool {
    let (Ok(emode), Ok(name), Ok(t)) = (c.p.u32(va), c.p.u32(va.wrapping_add(4)), c.p.u32(va.wrapping_add(8))) else {
        return false;
    };
    emode < 0x10000 && (name == 0 || text(c, name).is_some()) && (t == 0 || text(c, t).is_some_and(|x| !x.is_empty()))
}

/// (kind, count) of an object nothing declares, by its words.
fn shape(c: &Ctx, va: u32) -> (Kind, u32) {
    if is_record(c, va) {
        let mut n = 1;
        while n < 64 && is_record(c, va + RECORD * n) && c.p.u32(va + RECORD * n).unwrap_or(0) != 0 {
            n += 1;
        }
        return (Kind::Rec, n);
    }
    let mut n = 0;
    while n < 64 {
        let Ok(w) = c.p.u32(va + 4 * n) else { break };
        if w != 0 && !mapped(c, w) {
            break;
        }
        n += 1;
    }
    (Kind::Ptr, n.max(1))
}

/// The addresses the talk pages start from in the volume.
fn roots(v: Vol) -> Vec<u32> {
    let gs = groups();
    let get = |g: &str, e: &str| -> Value {
        let g = gs.iter().find(|x| x.name == g).unwrap();
        let i = g.entries.iter().position(|x| x.name == e).unwrap();
        extract(g, v)[i].clone()
    };
    let mut out: Vec<u32> = Vec::new();
    for row in get("battle", "npcs").list() {
        // (param (base (name, ccsname, kind, id, level, exp, gold, height, width, msg)), entry)
        let msg = row.list()[0].list()[0].list()[9].int() as u32;
        if msg != 0 {
            out.push(msg);
        }
    }
    out.extend(get("fieldui", "spc_msg_tbl").list().iter().map(|w| w.int() as u32).filter(|&w| w != 0));
    for k in [
        "spc_msg_present10_va",
        "spc_msg_present11_va",
        "breed_teach_va",
        "breed_teach2_va",
        "pg_evo_msg_va",
        "error_data_va",
        "kite_self_talk_va",
    ] {
        out.push(get("fieldui", k).int() as u32);
    }
    out
}

/// {address: (kind, count)} of every object the roots reach.
fn closure(v: Vol) -> Objects {
    if let Some(x) = CLOSURE.with(|m| m.borrow().get(&v).cloned()) {
        return x;
    }
    let c = ctx(v, Some("gcmn"));
    let mut known: BTreeMap<u32, (Kind, u32)> = BTreeMap::new();
    for (&inf, &(kind, n)) in inf_objects().iter() {
        if v == Vol::Inf {
            known.insert(inf, (kind, n));
            continue;
        }
        let sec = if inf >= 0x0040_0000 { 1 } else { 0 };
        let va = carried(v, sec, inf).or_else(|| if sec == 0 { carried(v, 1, inf) } else { None });
        if let Some(va) = va {
            known.insert(va, (kind, n));
        }
    }
    let containing = |known: &BTreeMap<u32, (Kind, u32)>, a: u32| -> Option<u32> {
        let (&s, &(kind, n)) = known.range(..=a).next_back()?;
        (a < s + n * if kind == Kind::Rec { RECORD } else { 4 }).then_some(s)
    };
    let mut out: BTreeMap<u32, (Kind, u32)> = BTreeMap::new();
    let mut todo = roots(v);
    while let Some(a) = todo.pop() {
        if a == 0 {
            continue;
        }
        let s = match containing(&known, a) {
            Some(s) => s,
            None => {
                if !mapped(&c, a) {
                    continue;
                }
                known.insert(a, shape(&c, a));
                a
            }
        };
        if out.contains_key(&s) {
            continue;
        }
        let (kind, n) = known[&s];
        out.insert(s, (kind, n));
        if kind == Kind::Ptr {
            for k in 0..n {
                let Ok(w) = c.p.u32(s + 4 * k) else { break };
                if w != 0 {
                    todo.push(w);
                }
            }
        }
    }
    let out = Rc::new(out);
    CLOSURE.with(|m| m.borrow_mut().insert(v, out.clone()));
    out
}

/// [(address, [record], [(name, str) addresses])] of the record arrays
/// reached, by address.
pub fn records(c: &Ctx) -> Result<Value, String> {
    let ev = ev_msg();
    let mut out = Vec::new();
    for (&s, &(kind, n)) in closure(c.volume).iter() {
        if kind != Kind::Rec {
            continue;
        }
        let (mut rows, mut vas) = (Vec::new(), Vec::new());
        for k in 0..n {
            let at = s + RECORD * k;
            match (ev.read(c, at), c.p.u32(at + 4), c.p.u32(at + 8)) {
                (Ok(r), Ok(name), Ok(text)) => {
                    rows.push(r);
                    vas.push(Value::List(vec![Value::Int(i128::from(name)), Value::Int(i128::from(text))]));
                }
                _ => break,
            }
        }
        out.push(Value::List(vec![Value::Int(i128::from(s)), Value::List(rows), Value::List(vas)]));
    }
    Ok(Value::List(out))
}

/// [(address, [word])] of the pointer tables reached, by address.
pub fn pointers(c: &Ctx) -> Result<Value, String> {
    let mut out = Vec::new();
    for (&s, &(kind, n)) in closure(c.volume).iter() {
        if kind != Kind::Ptr {
            continue;
        }
        let mut words = Vec::new();
        for k in 0..n {
            match c.p.u32(s + 4 * k) {
                Ok(w) => words.push(Value::Int(i128::from(w))),
                Err(_) => break,
            }
        }
        out.push(Value::List(vec![Value::Int(i128::from(s)), Value::List(words)]));
    }
    Ok(Value::List(out))
}
