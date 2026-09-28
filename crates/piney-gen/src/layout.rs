//! How a table lies in memory (a layout), what it reads as (a [`Value`]),
//! and how it is written as Rust: the pieces `plans/volumes.md` ("The
//! generator") lists.

use std::collections::HashMap;
use std::rc::Rc;

use crate::text;
use crate::volume::{Ctx, Vol};

/// A read value: a number (a float by its bits), text or bytes, nothing
/// (a null pointer), or a list (an array, a struct's fields).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Value {
    None,
    Int(i128),
    Bytes(Vec<u8>),
    List(Vec<Value>),
}

impl Value {
    pub fn int(&self) -> i128 {
        match self {
            Value::Int(v) => *v,
            _ => panic!("not a number: {self:?}"),
        }
    }

    pub fn list(&self) -> &[Value] {
        match self {
            Value::List(v) => v,
            _ => panic!("not a list: {self:?}"),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self {
            Value::Bytes(v) => v,
            _ => panic!("not text: {self:?}"),
        }
    }

    fn leaves<'a>(&'a self, out: &mut Vec<&'a Value>) {
        match self {
            Value::List(v) => v.iter().for_each(|x| x.leaves(out)),
            x => out.push(x),
        }
    }

    /// How much of two values agree, leaf by leaf (0 to 1).
    pub fn likeness(&self, other: &Value) -> f64 {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        self.leaves(&mut a);
        other.leaves(&mut b);
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let same = a.iter().zip(&b).filter(|(x, y)| x == y).count();
        same as f64 / a.len().max(b.len()) as f64
    }
}

pub type Read = Result<Value, String>;
pub type CustomFn = Rc<dyn Fn(&Ctx) -> Read>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Num {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
}

impl Num {
    pub fn size(self) -> u32 {
        match self {
            Num::U8 | Num::I8 => 1,
            Num::U16 | Num::I16 => 2,
            Num::U32 | Num::I32 => 4,
            Num::U64 => 8,
        }
    }

    /// The Rust type's name.
    pub fn rust_name(self) -> &'static str {
        self.rust()
    }

    fn rust(self) -> &'static str {
        match self {
            Num::U8 => "u8",
            Num::I8 => "i8",
            Num::U16 => "u16",
            Num::I16 => "i16",
            Num::U32 => "u32",
            Num::I32 => "i32",
            Num::U64 => "u64",
        }
    }

    fn read(self, b: &[u8]) -> i128 {
        match self {
            Num::U8 => i128::from(b[0]),
            Num::I8 => i128::from(b[0] as i8),
            Num::U16 => i128::from(u16::from_le_bytes([b[0], b[1]])),
            Num::I16 => i128::from(i16::from_le_bytes([b[0], b[1]])),
            Num::U32 => i128::from(u32::from_le_bytes(b[..4].try_into().unwrap())),
            Num::I32 => i128::from(i32::from_le_bytes(b[..4].try_into().unwrap())),
            Num::U64 => i128::from(u64::from_le_bytes(b[..8].try_into().unwrap())),
        }
    }
}

/// A count: a number, or one the volume decides.
#[derive(Clone)]
pub enum Count {
    N(usize),
    By(Rc<dyn Fn(&Ctx) -> usize>),
}

impl Count {
    fn get(&self, ctx: &Ctx) -> usize {
        match self {
            Count::N(n) => *n,
            Count::By(f) => f(ctx),
        }
    }

    fn n(&self) -> usize {
        match self {
            Count::N(n) => *n,
            Count::By(_) => 0,
        }
    }
}

pub type Until = Rc<dyn Fn(&Value) -> bool>;

/// A struct: named fields at offsets.
pub struct StructDef {
    pub name: String,
    pub size: u32,
    pub fields: Vec<(String, u32, Layout)>,
    pub doc: String,
    /// The game's own struct (from the DWARF): laid into memory (`write`).
    pub memory: bool,
}

#[derive(Clone)]
pub enum Layout {
    Num(Num),
    /// A bitfield: `width` bits from `shift` of a `num` storage unit.
    Bits {
        num: Num,
        shift: u32,
        width: u32,
    },
    /// A function pointer, as which of a closed set of functions it names:
    /// a variant of the enum, from the function's name (none for a null).
    Func(&'static str),
    Float,
    Fixed {
        inner: Box<Layout>,
        n: usize,
        empty: Option<Value>,
    },
    CStr,
    FixedText(u32),
    TextRows {
        max: usize,
        stride: u32,
    },
    Bytes {
        n: Count,
        blank: Vec<(usize, usize)>,
    },
    CountedI16s(i16),
    Omit,
    LinesCounted(i32),
    CountedPtr {
        inner: Box<Layout>,
        delta: i32,
    },
    Slots(usize),
    Addr,
    Lines(usize),
    Ptr(Box<Layout>),
    Opt(Box<Layout>),
    Array {
        inner: Box<Layout>,
        n: Count,
        until: Option<Until>,
        /// The end row `until` finds is kept.
        through: bool,
        limit: usize,
        stride: Option<u32>,
    },
    Struct(Rc<StructDef>),
    Keyed {
        enum_: &'static str,
        variants: Vec<&'static str>,
        inner: Box<Layout>,
    },
    Custom {
        f: CustomFn,
        layout: Box<Layout>,
    },
    /// `saveSysMsg`'s messages: each None or its lines.
    MessageLines,
    /// Each mail's two replies: rows of `ReMail`, -1 none.
    ReplyLinks,
}

pub fn text_at(ctx: &Ctx, va: u32, limit: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    while out.len() < limit {
        let b = ctx.p.read(va.wrapping_add(out.len() as u32), 1)?[0];
        if b == 0 {
            return Ok(out);
        }
        out.push(b);
    }
    Err(format!("no NUL within 0x{limit:x} of 0x{va:08x}"))
}

fn not_text(va: u32, t: &[u8]) -> String {
    format!("0x{va:08x} is not text: {:?}", String::from_utf8_lossy(&t[..t.len().min(24)]))
}

impl Layout {
    pub fn size(&self) -> Option<u32> {
        Some(match self {
            Layout::Num(n) | Layout::Bits { num: n, .. } => n.size(),
            Layout::Float | Layout::Omit | Layout::LinesCounted(_) | Layout::CountedPtr { .. } | Layout::Func(_) => 4,
            Layout::Addr | Layout::Ptr(_) | Layout::Opt(_) => 4,
            Layout::Fixed { inner, n, .. } => inner.size()? * *n as u32,
            Layout::FixedText(n) => *n,
            Layout::Bytes { n: Count::N(n), .. } => *n as u32,
            Layout::Array { inner, n, .. } => inner.size()? * n.n() as u32,
            Layout::Struct(s) => s.size,
            _ => return None,
        })
    }

    pub fn rust(&self) -> String {
        match self {
            Layout::Num(n) | Layout::Bits { num: n, .. } => n.rust().to_string(),
            Layout::Float => "f32".into(),
            Layout::Func(e) => format!("Option<crate::tables::types::{e}>"),
            Layout::Fixed { inner, n, empty } => {
                if empty.is_some() {
                    format!("&'static [{}]", inner.rust())
                } else {
                    format!("[{}; {n}]", inner.rust())
                }
            }
            Layout::CStr | Layout::FixedText(_) => "&'static str".into(),
            Layout::TextRows { .. } | Layout::LinesCounted(_) | Layout::Slots(_) | Layout::Lines(_) => {
                "&'static [&'static str]".into()
            }
            Layout::Bytes { .. } => "&'static [u8]".into(),
            Layout::CountedI16s(_) => "&'static [i16]".into(),
            Layout::Omit => panic!("an omitted member has no type"),
            Layout::CountedPtr { inner, .. } => format!("Option<&'static [{}]>", inner.rust()),
            Layout::Addr => "u32".into(),
            Layout::Ptr(inner) => inner.rust(),
            Layout::Opt(inner) => format!("Option<{}>", inner.rust()),
            Layout::Array { inner, .. } => format!("&'static [{}]", inner.rust()),
            Layout::Struct(s) => s.name.clone(),
            Layout::Keyed { inner, .. } => inner.rust(),
            Layout::Custom { layout, .. } => layout.rust(),
            Layout::MessageLines => "&'static [Option<&'static [&'static str]>]".into(),
            Layout::ReplyLinks => "&'static [[Option<u16>; 2]]".into(),
        }
    }

    pub fn read(&self, ctx: &Ctx, va: u32) -> Read {
        Ok(match self {
            Layout::Num(n) => Value::Int(n.read(&ctx.p.read(va, n.size() as usize)?)),
            Layout::Bits { num, shift, width } => {
                let raw = ctx.p.read(va, num.size() as usize)?;
                let unsigned = raw.iter().rev().fold(0u64, |a, &b| a << 8 | u64::from(b));
                Value::Int(i128::from((unsigned >> shift) & ((1u64 << width) - 1)))
            }
            Layout::Float => Value::Int(i128::from(ctx.p.u32(va)?)),
            Layout::Func(_) => match ctx.p.u32(va)? {
                0 => Value::None,
                w => match ctx.p.symbol_at(w, 0) {
                    Some((s, 0)) => Value::Bytes(s.name.as_bytes().to_vec()),
                    _ => match later_func(ctx.volume, w) {
                        Some(name) => Value::Bytes(name.as_bytes().to_vec()),
                        None => return Err(format!("0x{va:08x}: 0x{w:08x} starts no named function")),
                    },
                },
            },
            Layout::Fixed { inner, n, empty } => {
                let step = inner.size().ok_or("a fixed array of unsized rows")?;
                let mut out = Vec::with_capacity(*n);
                for i in 0..*n {
                    out.push(inner.read(ctx, va + step * i as u32)?);
                }
                if let Some(e) = empty {
                    while out.last() == Some(e) {
                        out.pop();
                    }
                }
                Value::List(out)
            }
            Layout::CStr => {
                let t = text_at(ctx, va, 0x10000)?;
                if !text::plausible(&t) {
                    return Err(not_text(va, &t));
                }
                Value::Bytes(t)
            }
            Layout::FixedText(n) => {
                let b = ctx.p.read(va, *n as usize)?;
                let t = b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())].to_vec();
                if !text::plausible(&t) {
                    return Err(not_text(va, &t));
                }
                Value::Bytes(t)
            }
            Layout::TextRows { max, stride } => {
                let mut out = Vec::new();
                for k in 0..*max {
                    let Ok(t) = text_at(ctx, va + stride * k as u32, *stride as usize) else { break };
                    if !text::plausible(&t) {
                        break;
                    }
                    out.push(Value::Bytes(t));
                }
                Value::List(out)
            }
            Layout::Bytes { n, blank } => {
                let mut out = ctx.p.read(va, n.get(ctx))?;
                for &(lo, hi) in blank {
                    out[lo..hi].fill(0);
                }
                Value::Bytes(out)
            }
            Layout::CountedI16s(max) => {
                let b = ctx.p.read(va, 2)?;
                let n = i16::from_le_bytes([b[0], b[1]]).clamp(0, *max) as usize;
                let b = ctx.p.read(va + 2, 2 * n)?;
                Value::List(
                    (0..n).map(|k| Value::Int(i128::from(i16::from_le_bytes([b[2 * k], b[2 * k + 1]])))).collect(),
                )
            }
            Layout::Omit => panic!("an omitted member is not read"),
            Layout::LinesCounted(delta) => {
                let n = i32::from_le_bytes(ctx.p.read(va.wrapping_add(*delta as u32), 4)?.try_into().unwrap());
                let w = ctx.p.u32(va)?;
                if w != 0 && n > 0 { Layout::Lines(n as usize).read(ctx, w)? } else { Value::List(Vec::new()) }
            }
            Layout::CountedPtr { inner, delta } => {
                let n = i32::from_le_bytes(ctx.p.read(va.wrapping_add(*delta as u32), 4)?.try_into().unwrap());
                let w = ctx.p.u32(va)?;
                if w == 0 {
                    Value::None
                } else {
                    Layout::Array {
                        inner: inner.clone(),
                        n: Count::N(n.max(0) as usize),
                        until: None,
                        through: false,
                        limit: 4096,
                        stride: None,
                    }
                    .read(ctx, w)?
                }
            }
            Layout::Slots(width) => {
                let raw = Layout::CStr.read(ctx, va)?;
                let raw = raw.bytes();
                Value::List(
                    raw.chunks(*width)
                        .map(|c| {
                            let end = c.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
                            Value::Bytes(c[..end].to_vec())
                        })
                        .collect(),
                )
            }
            Layout::Addr => Value::Int(i128::from(va)),
            Layout::Lines(n) => {
                let mut out = Vec::with_capacity(*n);
                let mut at = va;
                for _ in 0..*n {
                    let t = text_at(ctx, at, 0x10000)?;
                    if !text::plausible(&t) {
                        return Err(not_text(at, &t));
                    }
                    at += t.len() as u32 + 1;
                    out.push(Value::Bytes(t));
                }
                Value::List(out)
            }
            Layout::Ptr(inner) => {
                let w = ctx.p.u32(va)?;
                if w == 0 {
                    return Err(format!("0x{va:08x}: a NULL pointer where there must be one"));
                }
                inner.read(ctx, w)?
            }
            Layout::Opt(inner) => {
                let w = ctx.p.u32(va)?;
                if w == 0 { Value::None } else { inner.read(ctx, w)? }
            }
            Layout::Array { inner, n, until, through, limit, stride } => {
                let step = stride.or_else(|| inner.size()).ok_or("an array of unsized rows")?;
                let mut out = Vec::new();
                match until {
                    Some(u) => {
                        for i in 0..*limit {
                            let row = inner.read(ctx, va + step * i as u32)?;
                            if u(&row) {
                                if *through {
                                    out.push(row);
                                }
                                return Ok(Value::List(out));
                            }
                            out.push(row);
                        }
                        return Err(format!("0x{va:08x}: no terminator row within {limit}"));
                    }
                    None => {
                        for i in 0..n.get(ctx) {
                            out.push(inner.read(ctx, va + step * i as u32)?);
                        }
                    }
                }
                Value::List(out)
            }
            Layout::Struct(s) => {
                let mut out = Vec::new();
                for (_, off, l) in &s.fields {
                    if !matches!(l, Layout::Omit) {
                        out.push(l.read(ctx, va + off)?);
                    }
                }
                Value::List(out)
            }
            Layout::Keyed { variants, inner, .. } => {
                let step = inner.size().ok_or("keyed rows of no size")?;
                let mut out = Vec::new();
                for i in 0..variants.len() {
                    out.push(inner.read(ctx, va + step * i as u32)?);
                }
                Value::List(out)
            }
            Layout::Custom { f, .. } => f(ctx)?,
            Layout::MessageLines | Layout::ReplyLinks => panic!("read through a Custom"),
        })
    }

    /// The value as a Rust expression: (raw, text), the text with repeated
    /// parts named by `em`.
    pub fn emit(&self, value: &Value, em: &mut Emitter, hint: &str) -> Result<(String, String), String> {
        Ok(match self {
            Layout::Num(n) => {
                let v = value.int();
                let t = if matches!(n, Num::U32 | Num::U64) && v > 9 {
                    format!("0x{:0w$x}", v, w = n.size() as usize * 2)
                } else {
                    v.to_string()
                };
                (t.clone(), t)
            }
            Layout::Bits { .. } => {
                let t = value.int().to_string();
                (t.clone(), t)
            }
            Layout::Func(e) => match value {
                Value::None => ("None".into(), "None".into()),
                v => {
                    let name = String::from_utf8_lossy(v.bytes()).into_owned();
                    let variant = func_variant(&name);
                    FUNCS.with(|f| f.borrow_mut().entry(e).or_default().insert(variant.clone(), name));
                    let t = format!("Some(crate::tables::types::{e}::{variant})");
                    (t.clone(), t)
                }
            },
            Layout::Float => {
                let t = float_literal(value.int() as u32);
                (t.clone(), t)
            }
            Layout::Fixed { inner, empty, .. } => {
                let mut parts = Vec::new();
                for v in value.list() {
                    parts.push(inner.emit(v, em, &format!("{hint}_item"))?);
                }
                let open = if empty.is_some() { "&[" } else { "[" };
                em.node(&self.rust(), join(open, &parts, 0), join(open, &parts, 1), hint, false)?
            }
            Layout::CStr => {
                let t = text::literal(value.bytes())?;
                em.node(&self.rust(), t.clone(), t, hint, false)?
            }
            Layout::FixedText(_) => {
                let t = text::literal(value.bytes())?;
                (t.clone(), t)
            }
            Layout::TextRows { .. } => {
                let mut parts = Vec::new();
                for v in value.list() {
                    parts.push(text::literal(v.bytes())?);
                }
                let t = format!("&[{}]", parts.join(", "));
                (t.clone(), t)
            }
            Layout::Bytes { .. } => {
                let t = text::bytes_literal(value.bytes());
                em.node(&self.rust(), t.clone(), t, hint, false)?
            }
            Layout::CountedI16s(_) => {
                let t =
                    format!("&[{}]", value.list().iter().map(|v| v.int().to_string()).collect::<Vec<_>>().join(", "));
                em.node(&self.rust(), t.clone(), t, hint, false)?
            }
            Layout::Omit => panic!("an omitted member is not written"),
            Layout::LinesCounted(_) => Layout::Lines(value.list().len()).emit(value, em, hint)?,
            Layout::CountedPtr { inner, .. } => match value {
                Value::None => ("None".into(), "None".into()),
                v => {
                    let a = Layout::Array {
                        inner: inner.clone(),
                        n: Count::N(v.list().len()),
                        until: None,
                        through: false,
                        limit: 4096,
                        stride: None,
                    };
                    let (r, t) = a.emit(v, em, hint)?;
                    (format!("Some({r})"), format!("Some({t})"))
                }
            },
            Layout::Slots(_) => {
                let mut parts = Vec::new();
                for v in value.list() {
                    let l = text::literal(v.bytes())?;
                    parts.push(em.node("&'static str", l.clone(), l, &format!("{hint}_slot"), false)?);
                }
                em.node(&self.rust(), join("&[", &parts, 0), join("&[", &parts, 1), hint, false)?
            }
            Layout::Addr => {
                let t = format!("0x{:08x}", value.int());
                (t.clone(), t)
            }
            Layout::Lines(_) => lines_emit(value, em, hint)?,
            Layout::Ptr(inner) => inner.emit(value, em, hint)?,
            Layout::Opt(inner) => match value {
                Value::None => ("None".into(), "None".into()),
                v => {
                    let (r, t) = inner.emit(v, em, hint)?;
                    (format!("Some({r})"), format!("Some({t})"))
                }
            },
            Layout::Array { inner, .. } => {
                let mut parts = Vec::new();
                let ty = inner.rust();
                for (i, v) in value.list().iter().enumerate() {
                    let h = format!("{hint}_{i}");
                    let (r, t) = inner.emit(v, em, &h)?;
                    parts.push(em.node(&ty, r, t, &h, true)?);
                }
                em.node(&self.rust(), join("&[", &parts, 0), join("&[", &parts, 1), hint, false)?
            }
            Layout::Struct(s) => {
                let kept: Vec<&(String, u32, Layout)> =
                    s.fields.iter().filter(|f| !matches!(f.2, Layout::Omit)).collect();
                let mut raw = Vec::new();
                let mut txt = Vec::new();
                for ((n, _, l), v) in kept.iter().map(|f| (&f.0, &f.1, &f.2)).zip(value.list()) {
                    let (r, t) = l.emit(v, em, &format!("{hint}_{n}"))?;
                    raw.push(format!("{n}: {r}"));
                    txt.push(format!("{n}: {t}"));
                }
                let r = format!("{} {{ {} }}", s.name, raw.join(", "));
                let t = format!("{} {{ {} }}", s.name, txt.join(", "));
                em.node(&s.name, r, t, hint, false)?
            }
            Layout::Keyed { .. } => panic!("a Keyed entry is written as a method"),
            Layout::Custom { layout, .. } => layout.emit(value, em, hint)?,
            Layout::MessageLines => {
                let mut some = Vec::new();
                for (i, v) in value.list().iter().enumerate() {
                    match v {
                        Value::None => some.push(("None".to_string(), "None".to_string())),
                        v => {
                            let h = format!("{hint}_{i}");
                            let (r, t) = lines_emit(v, em, &h)?;
                            let (r, t) = em.node("&'static [&'static str]", r, t, &h, true)?;
                            some.push((format!("Some({r})"), format!("Some({t})")));
                        }
                    }
                }
                em.node(&self.rust(), join("&[", &some, 0), join("&[", &some, 1), hint, false)?
            }
            Layout::ReplyLinks => {
                let o = |k: i128| if k < 0 { "None".to_string() } else { format!("Some({k})") };
                let rows: Vec<String> = value
                    .list()
                    .iter()
                    .map(|r| format!("[{}, {}]", o(r.list()[0].int()), o(r.list()[1].int())))
                    .collect();
                let t = format!("&[{}]", rows.join(", "));
                (t.clone(), t)
            }
        })
    }
}

fn join(open: &str, parts: &[(String, String)], k: usize) -> String {
    let close = if open.ends_with('[') { "]" } else { "" };
    let items: Vec<&str> = parts.iter().map(|p| if k == 0 { p.0.as_str() } else { p.1.as_str() }).collect();
    format!("{open}{}{close}", items.join(", "))
}

fn lines_emit(value: &Value, em: &mut Emitter, hint: &str) -> Result<(String, String), String> {
    let mut parts = Vec::new();
    for v in value.list() {
        let l = text::literal(v.bytes())?;
        parts.push(em.node("&'static str", l.clone(), l, &format!("{hint}_line"), false)?);
    }
    em.node("&'static [&'static str]", join("&[", &parts, 0), join("&[", &parts, 1), hint, false)
}

/// {enum: {variant: symbol}}.
type Funcs = std::collections::BTreeMap<&'static str, std::collections::BTreeMap<String, String>>;

thread_local! {
    /// Every function a `Func` layout named, for `types.rs`.
    pub static FUNCS: std::cell::RefCell<Funcs> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

/// A function's variant: its name without the arguments' mangling and the
/// `ccEntry` of the entry constructors, capitalised
/// (`ccEntryGimBox__FP13ccEntryParam` is `GimBox`).
/// Functions Infection lacks that a later volume's tables point at, which
/// no carried symbol names: our name for each, and where it is on
/// Mutation, Outbreak and Quarantine.
const LATER_FUNCS: [(&str, [u32; 3]); 1] = [
    // `gimmickTbl` row 45 (`PG_FLAG`)'s entry: a 0x200-byte `ccGimmick`
    // with gimmick parameter 45.
    ("ccEntryGimPgFlag", [0x005F_CB40, 0x005F_A570, 0x004E_CFD0]),
];

fn later_func(v: Vol, va: u32) -> Option<&'static str> {
    let k = match v {
        Vol::Inf => return None,
        Vol::Mut => 0,
        Vol::Out => 1,
        Vol::Qua => 2,
    };
    LATER_FUNCS.iter().find(|f| f.1[k] == va).map(|f| f.0)
}

pub fn func_variant(symbol: &str) -> String {
    let base = symbol.split("__").next().unwrap_or(symbol);
    let base = base.strip_prefix("ccEntry").unwrap_or(base);
    let mut c = base.chars();
    c.next().map_or_else(|| "Unnamed".to_string(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// Python's `%.{p}e`: a mantissa and a signed exponent of two or more digits.
fn exp_format(x: f64, p: usize) -> String {
    let s = format!("{x:.p$e}");
    let (m, e) = s.split_once('e').unwrap();
    let e: i32 = e.parse().unwrap();
    format!("{m}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
}

/// A float as the shortest decimal that reads back to the same bits
/// (plain decimals first, an exponent only where a plain one is long);
/// NaN and the infinities by their bits.
pub fn float_literal(bits: u32) -> String {
    let x = f64::from(f32::from_bits(bits));
    if x.is_nan() || x.is_infinite() {
        return format!("f32::from_bits(0x{bits:08x})");
    }
    let back = |r: &str| r.parse::<f64>().map(|v| (v as f32).to_bits() == bits).unwrap_or(false);
    if let Some(plain) = (0..10).map(|p| format!("{x:.p$}")).find(|r| back(r))
        && plain.len() <= 14
    {
        return if plain.contains('.') { plain } else { plain + ".0" };
    }
    (0..10).map(|p| exp_format(x, p)).find(|r| back(r)).unwrap_or_else(|| format!("f32::from_bits(0x{bits:08x})"))
}

/// Two passes over every volume's values: the first counts each part,
/// the second writes every part seen more than once as one const and
/// refers to it.
#[derive(Default)]
pub struct Emitter {
    pub counting: bool,
    counts: HashMap<(String, String), usize>,
    names: HashMap<(String, String), String>,
    taken: std::collections::HashSet<String>,
    pub consts: Vec<String>,
}

impl Emitter {
    /// Only a whole row of an entry's array, or a whole entry (`hoist`), is
    /// written once and named when two or more volumes hold it.
    pub fn node(
        &mut self,
        ty: &str,
        raw: String,
        text: String,
        hint: &str,
        hoist: bool,
    ) -> Result<(String, String), String> {
        if !hoist {
            return Ok((raw, text));
        }
        let key = (ty.to_string(), raw.clone());
        if self.counting {
            *self.counts.entry(key).or_insert(0) += 1;
            return Ok((raw, text));
        }
        if self.counts.get(&key).copied().unwrap_or(0) < 2 || raw.len() < 24 {
            return Ok((raw, text));
        }
        if let Some(n) = self.names.get(&key) {
            return Ok((raw, n.clone()));
        }
        let base: String = hint
            .to_uppercase()
            .chars()
            .map(|c| if c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' { c } else { '_' })
            .collect();
        let base = if base.is_empty() { "V".to_string() } else { base };
        let (mut name, mut n) = (base.clone(), 0);
        while self.taken.contains(&name) {
            n += 1;
            name = format!("{base}_{n}");
        }
        self.taken.insert(name.clone());
        self.names.insert(key, name.clone());
        // A const's references are 'static already.
        self.consts.push(format!("const {name}: {} = {text};", ty.replace("&'static ", "&")));
        Ok((raw, name))
    }
}
