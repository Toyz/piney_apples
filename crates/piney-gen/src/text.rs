//! The game's Shift-JIS as Rust text, as Windows code page 932 reads it:
//! each character checked to encode back to the same bytes, a two-byte code
//! Unicode does not assign (a glyph of the game's own font) kept as the
//! private-use character U+F0000 + its code.

use std::cell::RefCell;
use std::collections::BTreeSet;

use encoding_rs::SHIFT_JIS;

pub const GLYPH_BASE: u32 = 0xF0000;

thread_local! {
    /// Every non-ASCII character the generated text uses, for `sjis.rs`.
    pub static USED: RefCell<BTreeSet<char>> = const { RefCell::new(BTreeSet::new()) };
}

/// Code page 932's single bytes WHATWG's Shift_JIS leaves unmapped.
fn single_extra(b: u8) -> Option<char> {
    match b {
        0xA0 => Some('\u{F8F0}'),
        0xFD => Some('\u{F8F1}'),
        0xFE => Some('\u{F8F2}'),
        0xFF => Some('\u{F8F3}'),
        _ => None,
    }
}

fn decode_one(bytes: &[u8]) -> Option<char> {
    if bytes.len() == 1
        && let Some(c) = single_extra(bytes[0])
    {
        return Some(c);
    }
    let (s, had_errors) = SHIFT_JIS.decode_without_bom_handling(bytes);
    if had_errors {
        return None;
    }
    let mut it = s.chars();
    let c = it.next()?;
    it.next().is_none().then_some(c)
}

/// A character's code page 932 bytes.
pub fn encode_char(c: char) -> Option<Vec<u8>> {
    if (c as u32) >= GLYPH_BASE {
        return Some(((c as u32 - GLYPH_BASE) as u16).to_be_bytes().to_vec());
    }
    match c {
        '\u{F8F0}' => return Some(vec![0xA0]),
        '\u{F8F1}' => return Some(vec![0xFD]),
        '\u{F8F2}' => return Some(vec![0xFE]),
        '\u{F8F3}' => return Some(vec![0xFF]),
        _ => {}
    }
    let mut buf = [0u8; 4];
    let (b, _, had_errors) = SHIFT_JIS.encode(c.encode_utf8(&mut buf));
    (!had_errors).then(|| b.into_owned())
}

pub fn decode(raw: &[u8]) -> Result<String, String> {
    let mut out = String::new();
    let mut i = 0;
    while i < raw.len() {
        let b = raw[i];
        if ((0x81..=0x9F).contains(&b) || (0xE0..=0xFC).contains(&b)) && i + 1 < raw.len() {
            let pair = &raw[i..i + 2];
            let c = decode_one(pair)
                .filter(|&c| encode_char(c).as_deref() == Some(pair))
                .unwrap_or_else(|| char::from_u32(GLYPH_BASE + (u32::from(b) << 8 | u32::from(raw[i + 1]))).unwrap());
            out.push(c);
            i += 2;
        } else {
            let c = decode_one(&raw[i..i + 1])
                .filter(|&c| encode_char(c).as_deref() == Some(&raw[i..i + 1]))
                .ok_or_else(|| format!("byte 0x{b:02x} does not round-trip: {:?}", &raw[..raw.len().min(24)]))?;
            out.push(c);
            i += 1;
        }
    }
    Ok(out)
}

pub fn encode(t: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for c in t.chars() {
        out.extend(encode_char(c).ok_or_else(|| format!("{c:?} has no Shift-JIS code"))?);
    }
    Ok(out)
}

/// Text the game shows: printable ASCII, Shift-JIS, the escapes, newlines
/// and tabs; no control bytes.
pub fn plausible(b: &[u8]) -> bool {
    b.iter().all(|&c| c >= 0x20 || c == 0x09 || c == 0x0A)
}

/// The game's Shift-JIS bytes as a Rust string literal, checked to encode
/// back to the same bytes (`crate::tables::sjis::encode`).
pub fn literal(raw: &[u8]) -> Result<String, String> {
    let t = decode(raw)?;
    if encode(&t)? != raw {
        return Err(format!("does not round-trip through Shift-JIS: {:?}", &raw[..raw.len().min(24)]));
    }
    let mut out = String::from("\"");
    for c in t.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) >= GLYPH_BASE => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => {
                if (c as u32) >= 0x80 {
                    USED.with(|u| u.borrow_mut().insert(c));
                }
                out.push(c);
            }
        }
    }
    out.push('"');
    Ok(out)
}

/// A Rust byte string literal.
pub fn bytes_literal(raw: &[u8]) -> String {
    let mut out = String::from("b\"");
    for &b in raw {
        match b {
            0x5C => out.push_str("\\\\"),
            0x22 => out.push_str("\\\""),
            0x20..0x7F => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('"');
    out
}
