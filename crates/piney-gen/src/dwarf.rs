//! The DWARF 1 (`.debug`) Infection's executable carries, as far as the
//! generator reads it: the DIE tree, types, the structs' members and the
//! globals' addresses (`tools/dwarf1.py` has the whole of it).

use std::collections::HashMap;

use crate::elf::Elf;

pub const TAG_ARRAY: u16 = 0x01;
pub const TAG_CLASS: u16 = 0x02;
pub const TAG_ENUM: u16 = 0x04;
pub const TAG_GLOBAL_VARIABLE: u16 = 0x07;
pub const TAG_LOCAL_VARIABLE: u16 = 0x0c;
pub const TAG_MEMBER: u16 = 0x0d;
pub const TAG_POINTER: u16 = 0x0f;
pub const TAG_REFERENCE: u16 = 0x10;
pub const TAG_COMPILE_UNIT: u16 = 0x11;
pub const TAG_STRUCT: u16 = 0x13;
pub const TAG_SUBROUTINE_TYPE: u16 = 0x15;
pub const TAG_TYPEDEF: u16 = 0x16;
pub const TAG_UNION: u16 = 0x17;

pub const AT_SIBLING: u16 = 0x0012;
pub const AT_LOCATION: u16 = 0x0023;
pub const AT_NAME: u16 = 0x0038;
pub const AT_FUND_TYPE: u16 = 0x0055;
pub const AT_MOD_FUND_TYPE: u16 = 0x0063;
pub const AT_USER_DEF_TYPE: u16 = 0x0072;
pub const AT_MOD_U_D_TYPE: u16 = 0x0083;
pub const AT_SUBSCR_DATA: u16 = 0x00a3;
pub const AT_BYTE_SIZE: u16 = 0x00b6;

/// A struct member as `members` lists it: offset, name, type.
pub type Member = (u32, String, String);

/// A bitfield's first bit, counted from the storage unit's least
/// significant (not the most, as DWARF 1 says), and its width.
pub const AT_BIT_OFFSET: u16 = 0x00c5;
pub const AT_BIT_SIZE: u16 = 0x00d6;

const AGGREGATES: [u16; 4] = [TAG_CLASS, TAG_STRUCT, TAG_UNION, TAG_ENUM];

#[derive(Clone, Debug)]
pub enum Attr {
    Num(u64),
    Block(Vec<u8>),
    Str(String),
}

pub struct Die {
    pub tag: u16,
    attrs: Vec<(u16, Attr)>,
    pub children: Vec<usize>,
}

impl Die {
    pub fn attr(&self, code: u16) -> Option<&Attr> {
        self.attrs.iter().find(|(c, _)| *c == code).map(|(_, v)| v)
    }

    pub fn num(&self, code: u16) -> Option<u64> {
        match self.attr(code)? {
            Attr::Num(v) => Some(*v),
            _ => None,
        }
    }

    pub fn block(&self, code: u16) -> Option<&[u8]> {
        match self.attr(code)? {
            Attr::Block(b) => Some(b),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self.attr(AT_NAME)? {
            Attr::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// A type as dwarf1.py builds it.
#[derive(Clone, Debug)]
pub enum Ty {
    /// A fundamental type (its FT code) or an aggregate or typedef (its DIE).
    Fund(u16),
    Named(usize),
    Ptr(Box<Ty>),
    /// A C++ reference: no table holds one.
    Ref,
    Cv(Box<Ty>),
    Array(Option<u32>, Box<Ty>),
    Func,
    Unknown,
}

/// The fundamental types' C names.
pub fn fund_name(ft: u16) -> &'static str {
    match ft {
        0x01 => "char",
        0x02 => "signed char",
        0x03 => "unsigned char",
        0x04 | 0x05 => "short",
        0x06 => "unsigned short",
        0x07 | 0x08 => "int",
        0x09 => "unsigned int",
        0x0a | 0x0b => "long",
        0x0c => "unsigned long",
        0x0d => "void *",
        0x0e => "float",
        0x0f => "double",
        0x14 => "void",
        0x15 => "bool",
        0x8008 | 0x8108 => "long long",
        0x8208 => "unsigned long long",
        _ => "?",
    }
}

fn u16_at(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}

fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}

fn read_form(b: &[u8], p: usize, form: u16) -> Result<(Attr, usize), String> {
    Ok(match form {
        1 | 2 | 6 => (Attr::Num(u64::from(u32_at(b, p))), p + 4),
        5 => (Attr::Num(u64::from(u16_at(b, p))), p + 2),
        7 => (Attr::Num(u64::from_le_bytes(b[p..p + 8].try_into().unwrap())), p + 8),
        3 => {
            let k = u16_at(b, p) as usize;
            (Attr::Block(b[p + 2..p + 2 + k].to_vec()), p + 2 + k)
        }
        4 => {
            let k = u32_at(b, p) as usize;
            (Attr::Block(b[p + 4..p + 4 + k].to_vec()), p + 4 + k)
        }
        8 => {
            let q = b[p..].iter().position(|&c| c == 0).map(|n| p + n).ok_or("an unended string")?;
            (Attr::Str(b[p..q].iter().map(|&c| c as char).collect()), q + 1)
        }
        _ => return Err(format!("unknown form {form}")),
    })
}

/// The location expression's operations: (op, operand).
pub fn loc_ops(b: &[u8]) -> Vec<(u8, Option<u32>)> {
    let mut out = Vec::new();
    let mut p = 0;
    while p < b.len() {
        let op = b[p];
        p += 1;
        if [1, 2, 3, 4, 0x80].contains(&op) && p + 4 <= b.len() {
            out.push((op, Some(u32_at(b, p))));
            p += 4;
        } else {
            out.push((op, None));
        }
    }
    out
}

pub struct Dwarf {
    pub dies: Vec<Die>,
    by_off: HashMap<u32, usize>,
    /// The compile units, in order.
    pub cus: Vec<usize>,
}

impl Dwarf {
    pub fn read(elf: &Elf) -> Result<Dwarf, String> {
        let sec = elf.sections.iter().find(|s| s.name == ".debug").ok_or("no .debug section")?;
        let buf = &elf.data[sec.offset as usize..(sec.offset + sec.size) as usize];
        let n = buf.len();
        let mut dies: Vec<Die> = Vec::new();
        let mut roots: Vec<usize> = Vec::new();
        let mut by_off = HashMap::new();
        // (parent die, its children's end); None is the root.
        let mut stack: Vec<(Option<usize>, usize)> = vec![(None, n)];
        let mut off = 0usize;
        while off < n {
            while stack.len() > 1 && off >= stack.last().unwrap().1 {
                stack.pop();
            }
            if n - off < 4 {
                break;
            }
            let length = u32_at(buf, off) as usize;
            if length < 8 {
                off += length.max(4);
                continue;
            }
            let end = off + length;
            if end > n {
                return Err(format!("DIE at 0x{off:x} runs past .debug"));
            }
            let tag = u16_at(buf, off + 4);
            let mut attrs = Vec::new();
            let mut p = off + 6;
            while p < end {
                let code = u16_at(buf, p);
                let (v, q) = read_form(buf, p + 2, code & 15)?;
                attrs.push((code, v));
                p = q;
            }
            let i = dies.len();
            dies.push(Die { tag, attrs, children: Vec::new() });
            by_off.insert(off as u32, i);
            match stack.last().unwrap().0 {
                Some(parent) => dies[parent].children.push(i),
                None => roots.push(i),
            }
            if let Some(sib) = dies[i].num(AT_SIBLING)
                && sib as usize > end
            {
                stack.push((Some(i), sib as usize));
            }
            off = end;
        }
        let cus = roots.into_iter().filter(|&i| dies[i].tag == TAG_COMPILE_UNIT).collect();
        Ok(Dwarf { dies, by_off, cus })
    }

    pub fn die(&self, i: usize) -> &Die {
        &self.dies[i]
    }

    /// A DIE's type attribute.
    pub fn type_of(&self, d: &Die) -> Option<Ty> {
        if let Some(ft) = d.num(AT_FUND_TYPE) {
            return Some(Ty::Fund(ft as u16));
        }
        if let Some(r) = d.num(AT_USER_DEF_TYPE) {
            return Some(self.udt(r as u32));
        }
        if let Some(b) = d.block(AT_MOD_FUND_TYPE) {
            return Some(modified(&b[..b.len() - 2], Ty::Fund(u16_at(b, b.len() - 2))));
        }
        if let Some(b) = d.block(AT_MOD_U_D_TYPE) {
            return Some(modified(&b[..b.len() - 4], self.udt(u32_at(b, b.len() - 4))));
        }
        None
    }

    fn udt(&self, r: u32) -> Ty {
        let Some(&i) = self.by_off.get(&r) else { return Ty::Unknown };
        let d = &self.dies[i];
        match d.tag {
            t if AGGREGATES.contains(&t) || t == TAG_TYPEDEF => Ty::Named(i),
            TAG_ARRAY => self.array_type(d),
            TAG_SUBROUTINE_TYPE => Ty::Func,
            TAG_POINTER => Ty::Ptr(Box::new(self.type_of(d).unwrap_or(Ty::Fund(0x14)))),
            TAG_REFERENCE => Ty::Ref,
            _ => Ty::Unknown,
        }
    }

    fn array_type(&self, d: &Die) -> Ty {
        let b = d.block(AT_SUBSCR_DATA).unwrap_or_default();
        let mut p = 0;
        let mut dims: Vec<(Option<u32>, Option<u32>)> = Vec::new();
        let mut elem = None;
        while p < b.len() {
            let fmt = b[p];
            p += 1;
            if fmt == 8 {
                let code = u16_at(b, p);
                if let Ok((v, _)) = read_form(b, p + 2, code & 15) {
                    let one = Die { tag: 0, attrs: vec![(code, v)], children: Vec::new() };
                    elem = self.type_of(&one);
                }
                break;
            }
            p += if fmt & 4 != 0 { 4 } else { 2 };
            let mut bounds = [None, None];
            for (k, expr) in [fmt & 2, fmt & 1].into_iter().enumerate() {
                if expr != 0 {
                    let k2 = u16_at(b, p) as usize;
                    p += 2 + k2;
                } else {
                    bounds[k] = Some(u32_at(b, p));
                    p += 4;
                }
            }
            dims.push((bounds[0], bounds[1]));
        }
        let mut t = elem.unwrap_or(Ty::Unknown);
        for (lo, hi) in dims.into_iter().rev() {
            let n = match (lo, hi) {
                (Some(lo), Some(hi)) if hi != 0xffff_ffff => Some(hi - lo + 1),
                _ => None,
            };
            t = Ty::Array(n, Box::new(t));
        }
        t
    }

    /// Every class, struct, union or enum DIE called `name`, in order.
    pub fn aggregates(&self, name: &str) -> Vec<usize> {
        let name = name.rsplit("::").next().unwrap_or(name);
        (0..self.dies.len())
            .filter(|&i| AGGREGATES.contains(&self.dies[i].tag) && self.dies[i].name() == Some(name))
            .collect()
    }

    /// Each global and file static with an address: (address, its type).
    pub fn globals(&self) -> Vec<(u32, Ty)> {
        let mut out = Vec::new();
        for &cu in &self.cus {
            for &i in &self.dies[cu].children {
                let d = &self.dies[i];
                if d.tag != TAG_GLOBAL_VARIABLE && d.tag != TAG_LOCAL_VARIABLE {
                    continue;
                }
                let Some(t) = self.type_of(d) else { continue };
                if matches!(t, Ty::Func) {
                    continue;
                }
                let ops = d.block(AT_LOCATION).map(loc_ops).unwrap_or_default();
                if let Some(&(3, Some(addr))) = ops.first()
                    && addr != 0
                {
                    out.push((addr, t));
                }
            }
        }
        out
    }

    /// The globals named `name`: (address, type).
    pub fn global_named(&self, name: &str) -> Vec<(u32, Ty)> {
        let mut out = Vec::new();
        for &cu in &self.cus {
            for &i in &self.dies[cu].children {
                let d = &self.dies[i];
                if (d.tag != TAG_GLOBAL_VARIABLE && d.tag != TAG_LOCAL_VARIABLE) || d.name() != Some(name) {
                    continue;
                }
                let (Some(t), Some(&(3, Some(addr)))) =
                    (self.type_of(d), d.block(AT_LOCATION).map(loc_ops).unwrap_or_default().first())
                else {
                    continue;
                };
                out.push((addr, t));
            }
        }
        out
    }

    /// A type as C writes it.
    pub fn describe(&self, t: &Ty) -> String {
        match t {
            Ty::Fund(f) => fund_name(*f).to_string(),
            Ty::Named(i) => self.dies[*i].name().unwrap_or("<anon>").to_string(),
            Ty::Ptr(x) => format!("{} *", self.describe(x)),
            Ty::Ref => "&".into(),
            Ty::Cv(x) => self.describe(x),
            Ty::Array(n, x) => format!("{}[{}]", self.describe(x), n.map_or(String::new(), |n| n.to_string())),
            Ty::Func => "function".into(),
            Ty::Unknown => "?".into(),
        }
    }

    /// A struct's members: (offset, name, type), and its size.
    pub fn members(&self, name: &str) -> Option<(Vec<Member>, u64)> {
        let d = self.die(*self.aggregates(name).first()?);
        let mut out = Vec::new();
        for &c in &d.children {
            let c = self.die(c);
            if c.tag == TAG_MEMBER {
                let t = self.type_of(c).unwrap_or(Ty::Unknown);
                out.push((member_offset(c).unwrap_or(0), c.name().unwrap_or("").to_string(), self.describe(&t)));
            }
        }
        Some((out, d.num(AT_BYTE_SIZE).unwrap_or(0)))
    }
}

/// A struct member's offset (`CONST n; ADD`).
pub fn member_offset(d: &Die) -> Option<u32> {
    let ops = loc_ops(d.block(AT_LOCATION)?);
    match ops[..] {
        [(4, Some(n)), (7, None)] => Some(n),
        _ => None,
    }
}

fn modified(mods: &[u8], t: Ty) -> Ty {
    // Listed outermost first.
    let mut t = t;
    for &m in mods.iter().rev() {
        t = match m {
            1 => Ty::Ptr(Box::new(t)),
            2 => Ty::Ref,
            _ => Ty::Cv(Box::new(t)),
        };
    }
    t
}
