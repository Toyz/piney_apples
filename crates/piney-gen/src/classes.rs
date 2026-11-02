//! The game's classes as `piney_data::classes::GameClass`: the names of
//! Infection's DWARF classes (and of the later volumes' classes it lacks,
//! by their carried vtables), each one's base from the DWARF, and its
//! constructor (`__ct__<n><name>F..`, the lowest when overloaded) and
//! vtable (`__vt__<n><name>`) on each volume, from the symbols
//! (`source::symbols_text`); a class a later volume adds unnamed, and the
//! addresses the carry missed, from the code that makes it ([`found`]:
//! its constructor, its base, the vtable it stores). A class is listed
//! when it has a vtable or a constructor on some volume, or is the base of
//! one that does.

use std::collections::{BTreeMap, BTreeSet};

use crate::dwarf::{TAG_CLASS, TAG_STRUCT, Ty};
use crate::program::Program;
use crate::volume::VOLUMES;

/// DWARF 1's `TAG_inheritance`: a base class, its type the base.
const TAG_INHERITANCE: u16 = 0x1c;

/// One class: its name, its base and its addresses by volume.
struct Class {
    base: Option<String>,
    ctor: [Option<u32>; 4],
    vtable: [Option<u32>; 4],
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
            let c = out.entry(name.to_string()).or_insert_with(|| Class {
                base: bases.get(name).cloned().flatten(),
                ctor: [None; 4],
                vtable: [None; 4],
            });
            let slot = if ctor { &mut c.ctor[v.index()] } else { &mut c.vtable[v.index()] };
            *slot = Some(slot.map_or(va, |old| old.min(va)));
        }
        // The classes a later volume adds that no symbol names, and the
        // addresses the carry missed, read from the code: the Root Towns
        // (Quarantine's `ROOTTOWN06`, which Mutation's and Outbreak's gcmn
        // carry unreached) and `ccSpcStart`'s fellows (Quarantine's 18-20).
        let p = crate::volume::open(v, Some("gcmn"))?.p;
        for f in found(&p)? {
            let c = out.entry(f.class.clone()).or_insert_with(|| Class {
                base: bases.get(&f.class).cloned().flatten(),
                ctor: [None; 4],
                vtable: [None; 4],
            });
            c.base = c.base.take().or(f.base);
            // Where the symbols name them too, the code must agree.
            let k = v.index();
            let agree = |named: Option<u32>, read: Option<u32>| named.is_none() || read.is_none() || named == read;
            if !agree(c.ctor[k], Some(f.ctor)) || !agree(c.vtable[k], f.vtable) {
                return Err(format!(
                    "{} {}: the symbols' constructor {:x?} and vtable {:x?}, the code's {:#x} and {:x?}",
                    v.tag(),
                    f.class,
                    c.ctor[k],
                    c.vtable[k],
                    f.ctor,
                    f.vtable
                ));
            }
            c.ctor[k] = c.ctor[k].or(Some(f.ctor));
            c.vtable[k] = c.vtable[k].or(f.vtable);
        }
    }
    // Each listed class's bases, to the root.
    let mut todo: Vec<String> = out.values().filter_map(|c| c.base.clone()).collect();
    while let Some(b) = todo.pop() {
        if out.contains_key(&b) {
            continue;
        }
        let base = bases.get(&b).cloned().flatten();
        todo.extend(base.clone());
        out.insert(b, Class { base, ctor: [None; 4], vtable: [None; 4] });
    }
    Ok(out)
}

/// A class read from the code: its name, its constructor, its base (whose
/// vtable it stores before its own, that constructor inlined, else whose
/// constructor it calls) and its own vtable (the last it stores).
struct Found {
    class: String,
    ctor: u32,
    base: Option<String>,
    vtable: Option<u32>,
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

/// `jr ra`.
const JR_RA: u32 = 0x03E0_0008;

/// The target of the `jal` `x` at `at`.
fn jal_target(x: u32, at: u32) -> Option<u32> {
    (x >> 26 == 3).then_some((at & 0xF000_0000) | ((x & 0x03FF_FFFF) << 2))
}

/// What the constructor at `ctor` builds up to `end`: the vtables it
/// stores (each a table whose third word is a function), in order, and
/// the class whose constructor it calls.
fn construction(p: &Program, class: String, ctor: u32, end: u32) -> Result<Found, String> {
    let w = code_words(p, ctor, end)?;
    let built = crate::xfer::addresses(&w, p.gp);
    let text = |a: u32| {
        p.function_at(a).is_some_and(|f| f.value == a)
            || p.overlay.as_ref().is_some_and(|o| (o.text..o.text + o.text_size).contains(&a))
    };
    let word = |a: u32| p.read(a, 4).ok().map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let tables: Vec<u32> = (0..w.len())
        .filter_map(|j| built.get(&j).copied())
        .filter(|&a| !text(a) && word(a + 8).is_some_and(text))
        .collect();
    let class_of =
        |name: &str, prefix: &str| name.strip_prefix(prefix).and_then(length_name).map(|(n, _)| n.to_string());
    let called = (0..w.len())
        .filter_map(|j| jal_target(w[j], ctor + 4 * j as u32))
        .filter_map(|t| p.function_at(t).filter(|f| f.value == t))
        .find_map(|f| class_of(&f.name, "__ct__"));
    let stored = (tables.iter().rev().skip(1))
        .find_map(|&a| p.symbols.iter().filter(|s| s.value == a).find_map(|s| class_of(&s.name, "__vt__")));
    Ok(Found { class, ctor, base: stored.or(called), vtable: tables.last().copied() })
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
        out.push(construction(p, format!("ccFellow{k:02}"), ctor, function_end(p, ctor)?)?);
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
/// towns', story maps' and characters' follow as their defs land.
const FAMILIES: [(&str, &str, &str); 1] = [(
    "BossGameClass",
    "ccBoss",
    "A boss's class: `ccBoss` and every class derived from it (the bosses, Kyvia's, the parts and slaves).",
)];

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
