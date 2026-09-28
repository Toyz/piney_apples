//! `Type(name)`: a struct as Infection's DWARF declares it, its members by
//! their own names in snake case, numbers as numbers, fixed arrays as
//! `[T; n]`, nested structs as their own, `char *` as optional text and
//! any other pointer as the address it holds. An override replaces a
//! member's layout (dotted for a nested one).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::dwarf::{self, AT_BIT_OFFSET, AT_BIT_SIZE, AT_BYTE_SIZE, Dwarf, TAG_MEMBER, TAG_TYPEDEF, Ty};
use crate::elf::Elf;
use crate::layout::{Layout, Num, StructDef};
use crate::volume::Vol;

/// A struct's name and its overrides' keys.
type TypeKey = (String, Vec<String>);

thread_local! {
    static DWARF: RefCell<Option<Rc<Dwarf>>> = const { RefCell::new(None) };
    static TYPES: RefCell<HashMap<TypeKey, Rc<StructDef>>> = RefCell::new(HashMap::new());
}

/// Infection's DWARF, read once.
pub fn dwarf() -> Rc<Dwarf> {
    DWARF.with(|d| {
        d.borrow_mut()
            .get_or_insert_with(|| {
                let exe = Vol::Inf.exe();
                let bytes = crate::source::read(Vol::Inf, exe).unwrap_or_else(|e| {
                    crate::die(&format!("{e} (Infection's disc is needed for the tables' layouts)"))
                });
                let elf = Elf::parse(bytes, exe).unwrap_or_else(|e| crate::die(&e));
                Rc::new(Dwarf::read(&elf).unwrap_or_else(|e| crate::die(&e)))
            })
            .clone()
    })
}

/// Member names that are Rust keywords, and what the port calls them.
fn keyword(s: &str) -> Option<&'static str> {
    Some(match s {
        "type" => "kind",
        "ref" => "reference",
        "match" => "matched",
        "move" => "moves",
        "self" => "this",
        "fn" => "func",
        "use" => "used",
        "loop" => "looped",
        "box" => "boxed",
        "static" => "fixed",
        _ => return None,
    })
}

/// battleAbility -> battle_ability, maxHP -> max_hp.
pub fn snake(name: &str) -> String {
    let c: Vec<char> = name.chars().collect();
    let mut a = String::new();
    let mut i = 0;
    while i < c.len() {
        if i + 1 < c.len() && (c[i].is_ascii_lowercase() || c[i].is_ascii_digit()) && c[i + 1].is_ascii_uppercase() {
            a.push(c[i]);
            a.push('_');
            a.push(c[i + 1]);
            i += 2;
        } else {
            a.push(c[i]);
            i += 1;
        }
    }
    let c: Vec<char> = a.chars().collect();
    let mut b = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i].is_ascii_uppercase() {
            let mut j = i;
            while j < c.len() && c[j].is_ascii_uppercase() {
                j += 1;
            }
            if j - i >= 2 && j < c.len() && c[j].is_ascii_lowercase() {
                b.extend(&c[i..j - 1]);
                b.push('_');
                b.push(c[j - 1]);
                b.push(c[j]);
                i = j + 1;
            } else {
                b.extend(&c[i..j]);
                i = j;
            }
        } else {
            b.push(c[i]);
            i += 1;
        }
    }
    let b = b.to_lowercase();
    keyword(&b).map_or(b, str::to_string)
}

/// ccSpcParamData -> SpcParamData, FOOD_PARAM -> FoodParam.
pub fn rust_type_name(name: &str) -> String {
    let c: Vec<char> = name.chars().collect();
    if !name.chars().any(|c| c.is_ascii_lowercase()) {
        return name
            .split('_')
            .filter(|w| !w.is_empty())
            .map(|w| w[..1].to_string() + &w[1..].to_ascii_lowercase())
            .collect();
    }
    if name.starts_with("cc") && c.get(2).is_some_and(|c| c.is_uppercase()) {
        name[2..].to_string()
    } else {
        let mut s = c[0].to_uppercase().collect::<String>();
        s.extend(&c[1..]);
        s
    }
}

pub type Overrides = Vec<(String, Layout)>;

pub fn ty(name: &str, over: Overrides) -> Layout {
    ty_doc(name, over, None)
}

pub fn ty_doc(name: &str, over: Overrides, doc: Option<&str>) -> Layout {
    let mut keys: Vec<String> = over.iter().map(|(k, _)| k.clone()).collect();
    keys.sort();
    let key = (name.to_string(), keys);
    if let Some(s) = TYPES.with(|t| t.borrow().get(&key).cloned()) {
        return Layout::Struct(s);
    }
    let dw = dwarf();
    let Some(&d) = dw.aggregates(name).first() else { crate::die(&format!("no DWARF type {name}")) };
    let d = dw.die(d);
    let lay = |t: &Ty, path: &str| layout(&dw, name, &over, t, path);
    let mut fields = Vec::new();
    for &c in &d.children {
        let c = dw.die(c);
        if c.tag != TAG_MEMBER {
            continue;
        }
        let off = dwarf::member_offset(c).unwrap_or_else(|| crate::die(&format!("{name}: a member without an offset")));
        let cname = c.name().unwrap_or_default();
        let t = dw.type_of(c).unwrap_or(Ty::Unknown);
        let mut l = lay(&t, cname);
        if let (Some(width), Layout::Num(num)) = (c.num(AT_BIT_SIZE), &l) {
            l = Layout::Bits { num: *num, shift: c.num(AT_BIT_OFFSET).unwrap_or(0) as u32, width: width as u32 };
        }
        fields.push((snake(cname), off, l));
    }
    let size = d
        .num(AT_BYTE_SIZE)
        .map(|v| v as u32)
        .filter(|&v| v != 0)
        .unwrap_or_else(|| fields.iter().map(|(_, off, l)| off + l.size().unwrap_or(4)).max().unwrap_or(0));
    let st = Rc::new(StructDef {
        name: rust_type_name(name),
        size,
        fields,
        doc: doc.map_or_else(|| format!("`{name}` (Infection's DWARF)."), str::to_string),
        memory: true,
    });
    TYPES.with(|t| t.borrow_mut().insert(key, st.clone()));
    Layout::Struct(st)
}

fn layout(dw: &Dwarf, name: &str, over: &Overrides, t: &Ty, path: &str) -> Layout {
    if let Some((_, l)) = over.iter().find(|(k, _)| k == path) {
        return l.clone();
    }
    match t {
        Ty::Fund(ft) => match dwarf::fund_name(*ft) {
            "char" | "signed char" => Layout::Num(Num::I8),
            "unsigned char" | "bool" => Layout::Num(Num::U8),
            "short" => Layout::Num(Num::I16),
            "unsigned short" => Layout::Num(Num::U16),
            "int" => Layout::Num(Num::I32),
            "unsigned int" => Layout::Num(Num::U32),
            "float" => Layout::Float,
            other => crate::die(&format!("{name}.{path}: no layout for {other}")),
        },
        Ty::Named(i) => {
            let sub = dw.die(*i);
            if sub.tag == TAG_TYPEDEF {
                let t = dw.type_of(sub).unwrap_or(Ty::Unknown);
                return layout(dw, name, over, &t, path);
            }
            let prefix = format!("{path}.");
            let inner: Overrides = over
                .iter()
                .filter_map(|(k, v)| k.strip_prefix(&prefix).map(|rest| (rest.to_string(), v.clone())))
                .collect();
            ty(sub.name().unwrap_or_default(), inner)
        }
        Ty::Array(n, inner) => Layout::Fixed {
            inner: Box::new(layout(dw, name, over, inner, path)),
            n: n.unwrap_or_else(|| crate::die(&format!("{name}.{path}: an array without a bound"))) as usize,
            empty: None,
        },
        Ty::Ptr(inner) => match **inner {
            Ty::Fund(ft) if matches!(dwarf::fund_name(ft), "char" | "signed char" | "unsigned char") => {
                Layout::Opt(Box::new(Layout::CStr))
            }
            _ => Layout::Num(Num::U32),
        },
        Ty::Cv(inner) => layout(dw, name, over, inner, path),
        other => crate::die(&format!("{name}.{path}: no layout for {other:?}")),
    }
}
