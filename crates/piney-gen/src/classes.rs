//! The game's classes as `piney_data::classes::GameClass`: the names of
//! Infection's DWARF classes (and of the later volumes' classes it lacks,
//! by their carried vtables), each one's base from the DWARF, and its
//! constructor (`__ct__<n><name>F..`, the lowest when overloaded) and
//! vtable (`__vt__<n><name>`) on each volume, from the symbols
//! (`source::symbols_text`); a class a later volume adds unnamed, and the
//! addresses the carry missed, from the code that makes it ([`found`]:
//! its constructor and base; [`vtables`]: the table its constructor
//! stores at its vptr, checked against the symbols). A class is listed
//! when it has a vtable or a constructor on some volume, or is the base of
//! one that does.

use std::collections::{BTreeMap, BTreeSet};

use crate::dwarf::{TAG_CLASS, TAG_STRUCT, Ty};
use crate::program::{OVERLAYS, Program};
use crate::volume::VOLUMES;

/// DWARF 1's `TAG_inheritance`: a base class, its type the base.
const TAG_INHERITANCE: u16 = 0x1c;

/// One class: its name, its base and its addresses by volume, and the
/// tables its constructor stores into its object there ([`stores`]).
#[derive(Default)]
struct Class {
    base: Option<String>,
    ctor: [Option<u32>; 4],
    vtable: [Option<u32>; 4],
    /// The overlay its constructor is in on each volume (None for main).
    overlay: [Option<&'static str>; 4],
    stores: [Vec<(u32, u32)>; 4],
}

/// `<len><name>` at the start of a mangled tail: the name and what follows.
pub(crate) fn length_name(s: &str) -> Option<(&str, &str)> {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    let n: usize = s[..digits].parse().ok()?;
    Some((s.get(digits..digits + n)?, s.get(digits + n..)?))
}

/// The Rust variant for a class name: `ccBoss08_b` -> `CcBoss08B`,
/// `ROOTTOWN01` -> `Roottown01`.
fn variant(name: &str) -> String {
    let all_caps = !name.chars().any(|c| c.is_ascii_lowercase());
    name.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let rest = if all_caps { w[1..].to_ascii_lowercase() } else { w[1..].to_string() };
            w[..1].to_ascii_uppercase() + &rest
        })
        .collect()
}

/// Every class with its base and addresses.
fn classes() -> Result<BTreeMap<String, Class>, String> {
    let dw = crate::dtype::dwarf()?;
    // Infection's DWARF: each named class and struct, with its first base.
    let mut bases: BTreeMap<String, Option<String>> = BTreeMap::new();
    for d in &dw.dies {
        if d.tag != TAG_CLASS && d.tag != TAG_STRUCT {
            continue;
        }
        let Some(name) = d.name().filter(|n| !n.starts_with('@') && !n.is_empty()) else { continue };
        let base = d.children.iter().map(|&c| dw.die(c)).find(|c| c.tag == TAG_INHERITANCE).and_then(|c| {
            match dw.type_of(c)? {
                Ty::Named(i) => dw.die(i).name().map(str::to_string),
                _ => None,
            }
        });
        let e = bases.entry(name.to_string()).or_insert(None);
        if e.is_none() {
            *e = base;
        }
    }
    let mut out: BTreeMap<String, Class> = BTreeMap::new();
    for v in VOLUMES {
        let text = crate::source::symbols_text(v)?;
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            let (Some(va), Some(sym)) = (f.get(1).and_then(|a| u32::from_str_radix(a, 16).ok()), f.get(4)) else {
                continue;
            };
            let (ctor, tail) = match (sym.strip_prefix("__vt__"), sym.strip_prefix("__ct__")) {
                (Some(t), _) => (false, t),
                (_, Some(t)) => (true, t),
                _ => continue,
            };
            // A constructor's name is followed by its arguments (`F..`), a
            // vtable's by nothing.
            let Some((name, rest)) = length_name(tail) else { continue };
            if if ctor { !rest.starts_with('F') } else { !rest.is_empty() } {
                continue;
            }
            let c = out
                .entry(name.to_string())
                .or_insert_with(|| Class { base: bases.get(name).cloned().flatten(), ..Class::default() });
            let slot = if ctor { &mut c.ctor[v.index()] } else { &mut c.vtable[v.index()] };
            if slot.is_none_or(|old| va < old) {
                *slot = Some(va);
                if ctor {
                    c.overlay[v.index()] = OVERLAYS.iter().copied().find(|&o| f[0] == o);
                }
            }
        }
        // The classes a later volume adds that no symbol names, read from
        // the code: the Root Towns (Quarantine's `ROOTTOWN06`, which
        // Mutation's and Outbreak's gcmn carry unreached) and `ccSpcStart`'s
        // fellows (Quarantine's 18-20).
        let p = crate::volume::open(v, Some("gcmn"))?.p;
        let k = v.index();
        for f in found(&p)? {
            let c = out
                .entry(f.class.clone())
                .or_insert_with(|| Class { base: bases.get(&f.class).cloned().flatten(), ..Class::default() });
            if c.ctor[k].is_some_and(|named| named != f.ctor) {
                return Err(format!(
                    "{} {}: the symbols' constructor {:x?}, the code's {:#x}",
                    v.tag(),
                    f.class,
                    c.ctor[k],
                    f.ctor
                ));
            }
            c.ctor[k] = Some(f.ctor);
            c.overlay[k] = Some("gcmn");
            // Its base, where the DWARF lacks it: the class whose vtable
            // its constructor stores (that constructor inlined), else whose
            // constructor it calls.
            let stored = f.stores.iter().rev().find_map(|&(_, t)| {
                p.symbols.iter().filter(|s| s.value == t).find_map(|s| class_of(&s.name, "__vt__"))
            });
            c.base = c.base.take().or(stored.filter(|b| *b != f.class)).or(f.called);
            c.stores[k] = f.stores;
        }
        // What every other constructor stores into its object.
        for c in out.values_mut() {
            if let (Some(ctor), true) = (c.ctor[k], c.stores[k].is_empty()) {
                let p = crate::volume::open(v, c.overlay[k])?.p;
                c.stores[k] = stores(&p, ctor, sized_end(&p, ctor)?)?;
            }
        }
    }
    vtables(&mut out)?;
    // Each listed class's bases, to the root.
    let mut todo: Vec<String> = out.values().filter_map(|c| c.base.clone()).collect();
    while let Some(b) = todo.pop() {
        if out.contains_key(&b) {
            continue;
        }
        let base = bases.get(&b).cloned().flatten();
        todo.extend(base.clone());
        out.insert(b, Class { base, ..Class::default() });
    }
    Ok(out)
}

/// A class read from the code: its name, its constructor, the class whose
/// constructor it calls, and the tables it stores into its object.
struct Found {
    class: String,
    ctor: u32,
    called: Option<String>,
    stores: Vec<(u32, u32)>,
}

/// The class a mangled `<prefix><len><name>..` symbol names.
fn class_of(name: &str, prefix: &str) -> Option<String> {
    name.strip_prefix(prefix).and_then(length_name).map(|(n, _)| n.to_string())
}

/// Each class's vtable on every volume, read from its constructor: the
/// last table it stores at its vptr, the slot in its object its vtable
/// goes in (a static object's table stored later goes elsewhere). The
/// slot is read on the volume (the layouts grow), where a constructor
/// stores a vtable the symbols name, its own or an ancestor's (that
/// constructor inlined); a class with neither takes its nearest
/// ancestor's, else that of any class under the one that brings the vptr
/// down (the root-most with a vtable somewhere: one vptr, single
/// inheritance). On Infection the symbols must agree with the code; on a
/// carried volume a constructor that stores another class's vtable is
/// that class's ([`reassign`]).
fn vtables(out: &mut BTreeMap<String, Class>) -> Result<(), String> {
    let ancestors = |n: &str| {
        std::iter::successors(Some(n.to_string()), |n| out.get(n).and_then(|c| c.base.clone())).collect::<Vec<_>>()
    };
    let mut own: BTreeMap<(String, usize), u32> = BTreeMap::new();
    for (n, c) in out.iter() {
        for (k, v) in VOLUMES.iter().enumerate() {
            for a in ancestors(n) {
                let Some(t) = out.get(&a).and_then(|c| c.vtable[k]) else { continue };
                let Some(&(at, _)) = c.stores[k].iter().rfind(|s| s.1 == t) else { continue };
                if own.insert((a.clone(), k), at).is_some_and(|was| was != at) {
                    return Err(format!("{} {a}: its vtable stored at {at:#x} and elsewhere", v.tag()));
                }
            }
        }
    }
    let polymorphic = |a: &String| out.get(a).is_some_and(|c| c.vtable.iter().any(Option::is_some));
    let mut read: Vec<(String, usize, u32)> = Vec::new();
    for (n, c) in out.iter() {
        for (k, v) in VOLUMES.iter().enumerate() {
            if c.ctor[k].is_none() {
                continue;
            }
            let up = ancestors(n).iter().find_map(|a| own.get(&(a.clone(), k)).copied());
            let down = || {
                let Some(root) = ancestors(n).into_iter().rfind(polymorphic) else { return Ok(None) };
                let below: BTreeSet<u32> = (own.iter())
                    .filter(|((d, j), _)| *j == k && ancestors(d).contains(&root))
                    .map(|(_, &at)| at)
                    .collect();
                match below.len() {
                    0 => Ok(None),
                    1 => Ok(below.first().copied()),
                    _ => Err(format!("{} {n}: {root}'s descendants' vptrs at {below:x?}", v.tag())),
                }
            };
            let Some(at) = up.map_or_else(down, |u| Ok(Some(u)))? else { continue };
            if let Some(&(_, t)) = c.stores[k].iter().rfind(|s| s.0 == at) {
                read.push((n.clone(), k, t));
            }
        }
    }
    for (n, k, t) in read {
        match out[&n].vtable[k] {
            None => out.get_mut(&n).expect("listed").vtable[k] = Some(t),
            Some(named) if named != t => reassign(out, &n, k, t)?,
            Some(_) => {}
        }
    }
    Ok(())
}

/// Class `n`'s constructor on volume `k` stores table `t` at its vptr,
/// not the vtable the symbols name. On Infection the symbols are the
/// ELF's own, so the code is wrong. On a carried volume the carry placed
/// one name wrongly, and `t` says which: another class's vtable (in the
/// symbols) makes the constructor that class's (Outbreak's and
/// Quarantine's `ccDarkUpheavalMngrElement` stores the Soil one's); a table
/// no symbol names is the class's own, the named one then stored by no
/// constructor (Mutation's `ccBossEffEnergyGrow2`).
fn reassign(out: &mut BTreeMap<String, Class>, n: &str, k: usize, t: u32) -> Result<(), String> {
    let v = VOLUMES[k];
    let c = &out[n];
    let named = c.vtable[k];
    if v == crate::volume::Vol::Inf {
        return Err(format!("{} {n}: the symbols' vtable {named:x?}, the code's {t:#x}", v.tag()));
    }
    let Some(m) = out.keys().find(|m| out[*m].vtable[k] == Some(t)).cloned() else {
        if out.values().any(|m| m.stores[k].iter().any(|s| Some(s.1) == named)) {
            return Err(format!(
                "{} {n}: its named vtable {named:x?} is stored, and its constructor stores {t:#x}",
                v.tag()
            ));
        }
        out.get_mut(n).expect("listed").vtable[k] = Some(t);
        return Ok(());
    };
    let c = out.get_mut(n).expect("listed");
    let (ctor, overlay, stores) = (c.ctor[k].take(), c.overlay[k].take(), std::mem::take(&mut c.stores[k]));
    let m = out.get_mut(&m).expect("listed");
    if m.ctor[k].is_some_and(|x| Some(x) != ctor) {
        return Err(format!(
            "{} {n}: its carried constructor {ctor:x?} stores {t:#x}, whose class has its own",
            v.tag()
        ));
    }
    (m.ctor[k], m.overlay[k], m.stores[k]) = (ctor, overlay, stores);
    Ok(())
}

/// The code at `va` up to `end` as words.
fn code_words(p: &Program, va: u32, end: u32) -> Result<Vec<u32>, String> {
    let b = p.read(va, (end - va) as usize)?;
    Ok(b.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
}

/// The end of the function at `va`: past its first `jr ra` and the delay
/// slot (no more than 4 KiB on).
fn function_end(p: &Program, va: u32) -> Result<u32, String> {
    let w = code_words(p, va, va + 0x1000)?;
    let k = w.iter().position(|&x| x == JR_RA).ok_or_else(|| format!("no jr ra after {va:#x}"))?;
    Ok(va + 4 * (k as u32 + 2))
}

/// The end of the function at `va`: its symbol's size, else
/// [`function_end`].
fn sized_end(p: &Program, va: u32) -> Result<u32, String> {
    match p.function_at(va) {
        Some(f) if f.value == va => Ok(va + f.size),
        _ => function_end(p, va),
    }
}

/// `jr ra`.
const JR_RA: u32 = 0x03E0_0008;

/// The target of the `jal` `x` at `at`.
fn jal_target(x: u32, at: u32) -> Option<u32> {
    (x >> 26 == 3).then_some((at & 0xF000_0000) | ((x & 0x03FF_FFFF) << 2))
}

/// What a constructor's register or stack slot holds: an offset into its
/// object (`this`), a constant (an address being built) or anything else.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    This(u32),
    Const(u32),
    Other,
}

/// The tables (each a table whose third word is a function) the
/// constructor at `ctor` stores into its own object up to `end`, in
/// order, with the offset of the slot. `this` comes in `a0` and moves
/// through registers and the stack (the code is unoptimised), offsets
/// past 32 KiB through `lui` and `addu`.
fn stores(p: &Program, ctor: u32, end: u32) -> Result<Vec<(u32, u32)>, String> {
    let w = code_words(p, ctor, end)?;
    let text = |a: u32| {
        p.function_at(a).is_some_and(|f| f.value == a)
            || p.overlay.as_ref().is_some_and(|o| (o.text..o.text + o.text_size).contains(&a))
    };
    let table = |a: u32| !text(a) && p.u32(a + 8).is_ok_and(text);
    let mut reg = [Held::Other; 32];
    reg[A0] = Held::This(0);
    let mut stack: BTreeMap<u32, Held> = BTreeMap::new();
    let mut out = Vec::new();
    // Counts down to the caller-saved registers' loss after a call.
    let mut call = 0;
    for &x in &w {
        if call > 0 {
            call -= 1;
            if call == 0 {
                for r in (1..16).chain(24..26).chain([31]) {
                    reg[r] = Held::Other;
                }
            }
        }
        let (op, rs, rt, rd) = ((x >> 26), (x >> 21) as usize & 31, (x >> 16) as usize & 31, (x >> 11) as usize & 31);
        let imm = x & 0xFFFF;
        let simm = imm as u16 as i16 as i32 as u32;
        let (written, held) = match op {
            // lui
            0x0F => (rt, Held::Const(imm << 16)),
            // addiu, daddiu
            0x09 | 0x19 => (
                rt,
                match (rs, reg[rs]) {
                    (GP, _) => p.gp.map_or(Held::Other, |g| Held::Const(g.wrapping_add(simm))),
                    (_, Held::This(o)) => Held::This(o.wrapping_add(simm)),
                    (_, Held::Const(c)) => Held::Const(c.wrapping_add(simm)),
                    _ => Held::Other,
                },
            ),
            // ori
            0x0D => (rt, if let Held::Const(c) = reg[rs] { Held::Const(c | imm) } else { Held::Other }),
            // lw from the stack: what was spilled there
            0x23 if rs == SP => (rt, stack.get(&imm).copied().unwrap_or(Held::Other)),
            // sw: to the stack a spill, into the object a store
            0x2B => {
                if rs == SP {
                    stack.insert(imm, reg[rt]);
                } else if let (Held::This(o), Held::Const(t)) = (reg[rs], reg[rt])
                    && table(t)
                {
                    out.push((o.wrapping_add(simm), t));
                }
                continue;
            }
            // addu, daddu, or
            0 if matches!(x & 63, 0x21 | 0x2D | 0x25) => (
                rd,
                match (reg[rs], reg[rt], rt == 0, rs == 0) {
                    (h, _, true, _) | (_, h, _, true) => h,
                    (Held::This(o), Held::Const(c), ..) | (Held::Const(c), Held::This(o), ..) => {
                        Held::This(o.wrapping_add(c))
                    }
                    (Held::Const(a), Held::Const(b), ..) if x & 63 != 0x25 => Held::Const(a.wrapping_add(b)),
                    _ => Held::Other,
                },
            ),
            // jal, jalr: the caller-saved registers gone after the delay slot
            3 => {
                call = 2;
                (31, Held::Other)
            }
            0 if x & 63 == 9 => {
                call = 2;
                (rd, Held::Other)
            }
            0 => (rd, Held::Other),
            // other loads and immediates write rt
            0x08..=0x0E | 0x18 | 0x20..=0x27 | 0x37 => (rt, Held::Other),
            _ => continue,
        };
        if written != 0 {
            reg[written] = held;
        }
    }
    Ok(out)
}

/// `a0`, `gp`, `sp`.
const A0: usize = 4;
const GP: usize = 28;
const SP: usize = 29;

/// What the constructor at `ctor` builds up to `end`: the tables it
/// stores into its object and the class whose constructor it calls.
fn construction(p: &Program, class: String, ctor: u32, end: u32) -> Result<Found, String> {
    let w = code_words(p, ctor, end)?;
    let called = (0..w.len())
        .filter_map(|j| jal_target(w[j], ctor + 4 * j as u32))
        .filter_map(|t| p.function_at(t).filter(|f| f.value == t))
        .find_map(|f| class_of(&f.name, "__ct__"));
    Ok(Found { class, ctor, called, stores: stores(p, ctor, end)? })
}

/// The Root Towns no symbol names (`placement::statics::unnamed_towns`),
/// their vtables stored before they read their file; and `ccSpcStart`'s
/// rows past Kite's (row 0), up to the first empty one: row `k`'s start
/// (`ccFellow01Start` on Infection) makes `ccFellow<k>`, the call after
/// its `operator new` the constructor.
fn found(p: &Program) -> Result<Vec<Found>, String> {
    let mut out = Vec::new();
    for t in crate::placement::statics::unnamed_towns(p)? {
        out.push(construction(p, t.class, t.ctor, t.read_at)?);
    }
    let Some(table) = p.symbol_named("ccSpcStart").map(|s| s.value) else { return Ok(out) };
    let new = p.symbol_named("__nw__FUi").map(|s| s.value);
    for k in 1.. {
        let Some(start) = p.read(table + 4 * k, 4).ok().map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) else {
            break;
        };
        if start == 0 {
            break;
        }
        let end = function_end(p, start)?;
        let w = code_words(p, start, end)?;
        // The object `operator new` made goes to the next call: the
        // constructor.
        let calls: Vec<u32> = (0..w.len()).filter_map(|j| jal_target(w[j], start + 4 * j as u32)).collect();
        let Some(ctor) = calls.iter().position(|&t| Some(t) == new).and_then(|k| calls.get(k + 1).copied()) else {
            continue;
        };
        out.push(construction(p, format!("ccFellow{k:02}"), ctor, sized_end(p, ctor)?)?);
    }
    Ok(out)
}

/// `crates/piney-data/src/classes.rs`, unformatted: the enum (names
/// only), one table row a class, and the families as narrow enums with
/// the same discriminants.
pub fn render() -> Result<String, String> {
    let cs = classes()?;
    let mut seen = BTreeSet::new();
    for n in cs.keys() {
        if !seen.insert(variant(n)) {
            return Err(format!("two classes are {}", variant(n)));
        }
    }
    let index: BTreeMap<&String, usize> = cs.keys().enumerate().map(|(k, n)| (n, k)).collect();
    let mut s = String::from(
        "//! The game's classes (generated by `piney-gen gen` from Infection's DWARF\n\
         //! and every volume's symbols; do not edit): [`GameClass`] names one, its\n\
         //! row of [`CLASSES`] holds its name, its base and its constructor and\n\
         //! vtable on each volume. A class is listed when it has a vtable or a\n\
         //! constructor on some volume, or is the base of one that does. The\n\
         //! families are narrow enums with the same discriminants.\n\n\
         use crate::volume::Volume;\n\n\
         /// A class of the game's, by its name in the symbols.\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\n\
         #[repr(u16)]\n\
         pub enum GameClass {\n",
    );
    for n in cs.keys() {
        if variant(n) != *n {
            s += &format!("    #[doc(alias = \"{n}\")]\n");
        }
        s += &format!("    {},\n", variant(n));
    }
    s += "}\n\n/// A class's row: its name, its base (Infection's DWARF, the first where\n\
          /// it has two; none for a root or a class the DWARF lacks), its\n\
          /// constructor (the lowest of its overloads) and vtable by volume.\n\
          #[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct ClassInfo {\n    pub class: GameClass,\n    pub name: &'static str,\n\
          \x20   pub base: Option<GameClass>,\n    pub constructor: [Option<u32>; 4],\n    pub vtable: [Option<u32>; 4],\n}\n\n";
    s += &format!("/// How many classes there are.\npub const COUNT: usize = {};\n\n", cs.len());
    s += "/// Every class's row, by its discriminant (one line a row).\n#[rustfmt::skip]\npub static CLASSES: [ClassInfo; COUNT] = [\n";
    let row = |a: [Option<u32>; 4]| {
        let r: Vec<String> = a.iter().map(|x| x.map_or("None".to_string(), |x| format!("Some(0x{x:08x})"))).collect();
        format!("[{}]", r.join(", "))
    };
    for (n, c) in &cs {
        let base = c.base.as_ref().map_or("None".to_string(), |b| format!("Some(GameClass::{})", variant(b)));
        s += &format!(
            "    ClassInfo {{ class: GameClass::{}, name: \"{n}\", base: {base}, constructor: {}, vtable: {} }},\n",
            variant(n),
            row(c.ctor),
            row(c.vtable)
        );
    }
    s += "];\n\nimpl GameClass {\n    /// Its row.\n    pub const fn info(self) -> &'static ClassInfo {\n        &CLASSES[self as usize]\n    }\n\n\
          \x20   /// Its name in the symbols and the DWARF.\n    pub const fn name(self) -> &'static str {\n        self.info().name\n    }\n\n\
          \x20   /// Its base class.\n    pub const fn base(self) -> Option<GameClass> {\n        self.info().base\n    }\n\n\
          \x20   /// Its constructor's address on `v`.\n    pub const fn constructor(self, v: Volume) -> Option<u32> {\n        self.info().constructor[v as usize]\n    }\n\n\
          \x20   /// Its vtable's address on `v`.\n    pub const fn vtable(self, v: Volume) -> Option<u32> {\n        self.info().vtable[v as usize]\n    }\n\n\
          \x20   /// Every class, in order.\n    pub fn all() -> impl Iterator<Item = GameClass> {\n        CLASSES.iter().map(|c| c.class)\n    }\n}\n";
    for (family, root, doc) in FAMILIES {
        let members: Vec<&String> = cs.keys().filter(|n| descends(&cs, n, root)).collect();
        s += &format!(
            "\n/// {doc}\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\n#[repr(u16)]\npub enum {family} {{\n"
        );
        for n in &members {
            if variant(n) != **n {
                s += &format!("    #[doc(alias = \"{n}\")]\n");
            }
            s += &format!("    {} = {},\n", variant(n), index[n]);
        }
        s += &format!(
            "}}\n\nimpl {family} {{\n    /// The class as [`GameClass`] (the same discriminant).\n\
             \x20   pub const fn class(self) -> GameClass {{\n        CLASSES[self as usize].class\n    }}\n}}\n\n\
             impl From<{family}> for GameClass {{\n    fn from(c: {family}) -> GameClass {{\n        c.class()\n    }}\n}}\n"
        );
    }
    Ok(s)
}

/// The families generated so far: an enum each, of the classes whose
/// ancestry in the DWARF reaches the root (the root among them). The
/// story maps' and characters' follow as their defs land.
const FAMILIES: [(&str, &str, &str); 2] = [
    (
        "BossGameClass",
        "ccBoss",
        "A boss's class: `ccBoss` and every class derived from it (the bosses, Kyvia's, the parts and slaves).",
    ),
    ("TownGameClass", "ROOTTOWN", "A Root Town's class: `ROOTTOWN` and the six towns derived from it."),
];

/// `name`'s ancestry reaches `root`.
fn descends(cs: &BTreeMap<String, Class>, name: &str, root: &str) -> bool {
    let mut at = Some(name.to_string());
    while let Some(n) = at {
        if n == root {
            return true;
        }
        at = cs.get(&n).and_then(|c| c.base.clone());
    }
    false
}
