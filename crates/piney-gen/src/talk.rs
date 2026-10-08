//! The records the talk pages reach through the pointers the game keeps:
//! `npcTbl`'s `base.msg` (which the save's party records hold too),
//! `spcMsgTbl`, the present tables, the breeder's and the Grunty's records,
//! `errorData` and `kiteSelfTalk`: every record array and pointer table
//! reachable from them, with its address, since the port looks records up by
//! the addresses the save stores. Infection's DWARF declares each; a later
//! volume's are told by shape (`emode`, a name and a text), as long as
//! Infection's at least where the carry puts that very object.

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
/// `charTbl`'s rows: Infection's 18, the later volumes' 21.
const INF_CHARACTERS: u32 = 18;
const LATER_CHARACTERS: u32 = 21;

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

/// (kind, count) of an object nothing declares, by its words: records
/// while they have an `emode`, and the one a record of `emode` 1 chains to
/// (`Check(1)` goes on to the next) whatever its own.
fn shape(c: &Ctx, va: u32) -> (Kind, u32) {
    if is_record(c, va) {
        let emode = |k: u32| c.p.u32(va + RECORD * k).unwrap_or(0);
        let mut n = 1;
        while n < 64 && is_record(c, va + RECORD * n) && (emode(n) != 0 || emode(n - 1) & 0xff == 1) {
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
    // From Mutation on, the Flag Race's greeting and its results' table.
    for k in ["greet_va", "results_va"] {
        out.push(get("race", k).int() as u32);
    }
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
    let out = Rc::new(if v == Vol::Inf { inf_closure() } else { later_closure(v) });
    CLOSURE.with(|m| m.borrow_mut().insert(v, out.clone()));
    out
}

/// The size of one of an object's elements.
fn stride(kind: Kind) -> u32 {
    if kind == Kind::Rec { RECORD } else { 4 }
}

/// Infection's closure: each address reached is in the object its DWARF
/// declares there.
fn inf_closure() -> BTreeMap<u32, (Kind, u32)> {
    let c = ctx(Vol::Inf, Some("gcmn"));
    let mut known: BTreeMap<u32, (Kind, u32)> = (*inf_objects()).clone();
    let containing = |known: &BTreeMap<u32, (Kind, u32)>, a: u32| -> Option<u32> {
        let (&s, &(kind, n)) = known.range(..=a).next_back()?;
        (a < s + n * stride(kind)).then_some(s)
    };
    let mut out: BTreeMap<u32, (Kind, u32)> = BTreeMap::new();
    let mut todo = roots(Vol::Inf);
    while let Some(a) = todo.pop() {
        if a == 0 {
            continue;
        }
        let s = match containing(&known, a) {
            Some(s) => s,
            None if mapped(&c, a) => {
                known.insert(a, shape(&c, a));
                a
            }
            None => continue,
        };
        if out.contains_key(&s) {
            continue;
        }
        out.insert(s, known[&s]);
        follow(&c, s, known[&s], &mut todo);
    }
    out
}

/// A pointer table's words, to be reached in turn.
fn follow(c: &Ctx, s: u32, (kind, n): (Kind, u32), todo: &mut Vec<u32>) {
    if kind == Kind::Ptr {
        todo.extend((0..n).map_while(|k| c.p.u32(s + 4 * k).ok()).filter(|&w| w != 0));
    }
}

/// A later volume's closure. The carry places Infection's objects only
/// roughly here (many of Mutation's walking PCs' lines are two records
/// where Infection's were one, and some carried records fall between
/// them), so each address reached is an object of its own shape, at least
/// Infection's count where the carry puts that very object there; objects
/// that overlap are then one.
fn later_closure(v: Vol) -> BTreeMap<u32, (Kind, u32)> {
    let c = ctx(v, Some("gcmn"));
    let mut inf: HashMap<u32, (Kind, u32)> = HashMap::new();
    for (&at, &(kind, n)) in inf_objects().iter() {
        let sec = if at >= 0x0040_0000 { 1 } else { 0 };
        let Some(va) = carried(v, sec, at).or_else(|| if sec == 0 { carried(v, 1, at) } else { None }) else {
            continue;
        };
        // A table by character (`spcMsgTbl`, the gift thanks) has a row for
        // each of the later volumes' 21 (characters 18-20 in the extension).
        let n = if kind == Kind::Ptr && n == INF_CHARACTERS { LATER_CHARACTERS } else { n };
        inf.insert(va, (kind, n));
    }
    let mut found: BTreeMap<u32, (Kind, u32)> = BTreeMap::new();
    let mut todo = roots(v);
    while let Some(a) = todo.pop() {
        if a == 0 || found.contains_key(&a) || !mapped(&c, a) {
            continue;
        }
        // Where the carry puts an object at this very address its kind
        // stands (a record of empty text is not one by its words).
        let object = match (inf.get(&a), shape(&c, a)) {
            (Some(&(k, m)), (kind, n)) if k == kind => (kind, n.max(m)),
            (Some(&object), _) => object,
            (None, object) => object,
        };
        found.insert(a, object);
        follow(&c, a, object, &mut todo);
    }
    // One object for those that overlap, in step with each other.
    let mut out: BTreeMap<u32, (Kind, u32)> = BTreeMap::new();
    for (a, (kind, n)) in found {
        let before = out.range(..=a).next_back().map(|(&s, &o)| (s, o));
        match before {
            Some((s, (k, m))) if k == kind && a < s + m * stride(k) && (a - s) % stride(k) == 0 => {
                out.insert(s, (k, m.max((a - s) / stride(k) + n)));
            }
            _ => {
                out.insert(a, (kind, n));
            }
        }
    }
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
