//! A group's values as data for the build (`plans/build-data.md`): what
//! `Layout::emit` writes as a Rust literal, written instead in the form
//! `piney_data::store::Load` reads back into the same types. The groups
//! in [`IN_BUILD`] are generated as types and accessors only; their values
//! live in the build (`PINEY/TABLES/<group>.bin`) and, for the tools and
//! the checks, in `work/data/<volume>/TABLES/`.

use crate::layout::{FUNCS, Layout, Value, func_variant};
use crate::manifest::Group;
use crate::text;
use crate::volume::Vol;

/// The groups whose values are data in the build, not Rust in the repository.
pub const IN_BUILD: &[&str] = &[
    "battle",
    "book",
    "combat",
    "desktop",
    "dtmenu",
    "effect",
    "fieldui",
    "fonts",
    "game",
    "kanji",
    "loaddisp",
    "mail",
    "nameentry",
    "newgame",
    "party_chat",
    "race",
    "staffroll",
    "stream",
    "talk",
    "title",
    "toppage",
    "voice",
    "world",
];

/// Whether a group's values are in the build.
pub fn in_build(g: &Group) -> bool {
    IN_BUILD.contains(&g.name)
}

fn count(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u32).to_le_bytes());
}

fn string(out: &mut Vec<u8>, raw: &[u8]) -> Result<(), String> {
    // As `emit` would: checked to round-trip, its characters counted.
    text::literal(raw)?;
    let s = text::decode(raw)?;
    count(out, s.len());
    out.extend_from_slice(s.as_bytes());
    Ok(())
}

fn strings(out: &mut Vec<u8>, v: &Value) -> Result<(), String> {
    count(out, v.list().len());
    for x in v.list() {
        string(out, x.bytes())?;
    }
    Ok(())
}

/// `value` (of `l`) as `Load` reads the type `l.rust()` names.
pub fn write(l: &Layout, value: &Value, out: &mut Vec<u8>) -> Result<(), String> {
    match l {
        Layout::Num(n) | Layout::Bits { num: n, .. } => {
            let bytes = value.int().to_le_bytes();
            out.extend_from_slice(&bytes[..n.size() as usize]);
        }
        Layout::Float => out.extend_from_slice(&(value.int() as u32).to_le_bytes()),
        Layout::Func(e) => match value {
            Value::None => out.push(0),
            v => {
                let name = String::from_utf8_lossy(v.bytes()).into_owned();
                let variant = func_variant(&name);
                FUNCS.with(|f| f.borrow_mut().entry(e).or_default().insert(variant.clone(), name));
                out.push(1);
                count(out, variant.len());
                out.extend_from_slice(variant.as_bytes());
            }
        },
        Layout::Fixed { inner, empty, .. } => {
            if empty.is_some() {
                count(out, value.list().len());
            }
            for v in value.list() {
                write(inner, v, out)?;
            }
        }
        Layout::CStr | Layout::FixedText(_) => string(out, value.bytes())?,
        Layout::TextRows { .. } | Layout::Slots(_) | Layout::Lines(_) | Layout::LinesCounted(_) => strings(out, value)?,
        Layout::Bytes { .. } => {
            count(out, value.bytes().len());
            out.extend_from_slice(value.bytes());
        }
        Layout::CountedI16s(_) => {
            count(out, value.list().len());
            for v in value.list() {
                out.extend_from_slice(&(v.int() as i16).to_le_bytes());
            }
        }
        Layout::Omit => return Err("an omitted member is not written".into()),
        Layout::CountedPtr { inner, .. } => match value {
            Value::None => out.push(0),
            v => {
                out.push(1);
                count(out, v.list().len());
                for x in v.list() {
                    write(inner, x, out)?;
                }
            }
        },
        Layout::Addr => out.extend_from_slice(&(value.int() as u32).to_le_bytes()),
        Layout::Ptr(inner) => write(inner, value, out)?,
        Layout::Opt(inner) => match value {
            Value::None => out.push(0),
            v => {
                out.push(1);
                write(inner, v, out)?;
            }
        },
        Layout::Array { inner, .. } => {
            count(out, value.list().len());
            for v in value.list() {
                write(inner, v, out)?;
            }
        }
        Layout::Struct(s) => {
            let kept = s.fields.iter().filter(|f| !matches!(f.2, Layout::Omit));
            for ((_, _, l), v) in kept.zip(value.list()) {
                write(l, v, out)?;
            }
        }
        Layout::Keyed { inner, .. } => {
            count(out, value.list().len());
            for v in value.list() {
                write(inner, v, out)?;
            }
        }
        Layout::Custom { layout, .. } => write(layout, value, out)?,
        Layout::MessageLines => {
            count(out, value.list().len());
            for v in value.list() {
                match v {
                    Value::None => out.push(0),
                    v => {
                        out.push(1);
                        strings(out, v)?;
                    }
                }
            }
        }
        Layout::ReplyLinks => {
            count(out, value.list().len());
            for r in value.list() {
                for k in [r.list()[0].int(), r.list()[1].int()] {
                    if k < 0 {
                        out.push(0);
                    } else {
                        out.push(1);
                        out.extend_from_slice(&(k as u16).to_le_bytes());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Every entry of `g` for volume `v`, in the manifest's order: the file a
/// built group's `Load` reads.
pub fn group_bytes(g: &Group, v: Vol) -> Result<Vec<u8>, String> {
    let values = crate::locate::extract(g, v);
    let mut out = Vec::new();
    for (e, value) in g.entries.iter().zip(values.iter()) {
        write(&e.layout, value, &mut out).map_err(|m| format!("{}.{} ({}): {m}", g.name, e.name, v.tag()))?;
    }
    Ok(out)
}
