//! The readable text form of the instruction set.
//!
//! This is the format a script is written in by hand. [`print_event`] and
//! [`parse_event`] (and the per-part functions) round-trip exactly: parsing
//! printed text gives back the same IR, and printing is canonical.
//!
//! ```text
//! # A comment runs from # to the end of the line (outside quotes).
//! event 900 label="TEST 01"
//! open
//!   if event_done event=0
//!   if game_status status=2
//! block
//!   set phase phase=0 comp=eq
//!   frame_rate rate=2
//!   message msg=1
//! block
//!   set phase phase=4 comp=ge
//!   if operate num=-1 except=1
//!   message msg=0
//!   repeatable
//! end
//! messages
//!   0 mode=0 name="Hero" "Not yet."
//!   1 mode=0 "A line." "A second line."
//! end
//! ```
//!
//! - `open` starts the open conditions, one `if` line each.
//! - `block` starts a block (`block 3` may carry its index, which is
//!   checked). Inside it come, in this order: `set` lines (precondition
//!   settings, [`crate::ir::Tag`]), `if` lines (conditions), then bare
//!   instruction lines ([`crate::ir::Op`]).
//! - `end` closes the script.
//! - Every field is written `name=value`, in any order, all required.
//!   Values are decimal or `0x` hex; comparisons are `eq`, `ge`, `le` (or a
//!   number, which never passes).
//! - `messages` / `parody` tables list records by index: `mode=`, an
//!   optional `name="..."`, then up to three quoted lines.
//! - Strings are the game's bytes: printable ASCII as itself, `\"` `\\`,
//!   `\xNN` for any byte, and any other character as Shift-JIS.

use std::fmt::Write as _;

use crate::ir::{Block, Cond, CondKind, Event, GameText, Message, Op, OpKind, Script, Tag, TagKind, parse_int};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for ParseError {}

// ---------------------------------------------------------------------------
// Strings.

/// A quoted string for game bytes.
pub fn quote(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(b as char),
            0x81..=0x9f | 0xe0..=0xfc if i + 1 < bytes.len() => {
                match sjis_char(&bytes[i..i + 2]) {
                    Some(c) => out.push(c),
                    None => {
                        let _ = write!(out, "\\x{:02x}\\x{:02x}", b, bytes[i + 1]);
                    }
                }
                i += 2;
                continue;
            }
            0xa1..=0xdf => match sjis_char(&bytes[i..i + 1]) {
                Some(c) => out.push(c),
                None => {
                    let _ = write!(out, "\\x{b:02x}");
                }
            },
            _ => {
                let _ = write!(out, "\\x{b:02x}");
            }
        }
        i += 1;
    }
    out.push('"');
    out
}

/// The character these Shift-JIS bytes decode to, if it encodes back to
/// exactly these bytes (so printing it cannot change the text).
fn sjis_char(bytes: &[u8]) -> Option<char> {
    let (s, _, bad) = encoding_rs::SHIFT_JIS.decode(bytes);
    if bad {
        return None;
    }
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() || c.is_control() {
        return None;
    }
    let mut buf = [0u8; 4];
    let (enc, _, bad) = encoding_rs::SHIFT_JIS.encode(c.encode_utf8(&mut buf));
    (!bad && &enc[..] == bytes).then_some(c)
}

/// The bytes of a quoted string (the quotes included in `s`).
pub fn unquote(s: &str) -> Result<Vec<u8>, String> {
    let inner =
        s.strip_prefix('"').and_then(|r| r.strip_suffix('"')).ok_or_else(|| format!("not a quoted string: {s}"))?;
    let mut out = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push(b'"'),
                Some('\\') => out.push(b'\\'),
                Some('x') => {
                    let h: String = chars.by_ref().take(2).collect();
                    let v = u8::from_str_radix(&h, 16).map_err(|_| format!("bad escape \\x{h}"))?;
                    out.push(v);
                }
                other => return Err(format!("bad escape \\{}", other.map(String::from).unwrap_or_default())),
            }
        } else if c.is_ascii() && !c.is_ascii_control() {
            out.push(c as u8);
        } else {
            let mut buf = [0u8; 4];
            let (enc, _, bad) = encoding_rs::SHIFT_JIS.encode(c.encode_utf8(&mut buf));
            if bad {
                return Err(format!("{c:?} has no Shift-JIS code"));
            }
            out.extend_from_slice(&enc);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Printing.

pub fn print_tag(t: &Tag) -> String {
    format!("set {t}")
}

pub fn print_cond(c: &Cond) -> String {
    format!("if {c}")
}

pub fn print_op(o: &Op) -> String {
    o.to_string()
}

/// A script, from `open` to `end`.
pub fn print_script(s: &Script) -> String {
    let mut out = String::from("open\n");
    for c in &s.open {
        let _ = writeln!(out, "  {}", print_cond(c));
    }
    for (i, b) in s.blocks.iter().enumerate() {
        let _ = writeln!(out, "block {i}");
        for t in &b.tags {
            let _ = writeln!(out, "  {}", print_tag(t));
        }
        for c in &b.conds {
            let _ = writeln!(out, "  {}", print_cond(c));
        }
        for o in &b.ops {
            let _ = writeln!(out, "  {}", print_op(o));
        }
    }
    out.push_str("end\n");
    out
}

/// A message table, from `head` (`messages` or `parody`) to `end`.
pub fn print_messages(head: &str, msgs: &[Message]) -> String {
    let mut out = format!("{head}\n");
    for (i, m) in msgs.iter().enumerate() {
        let _ = write!(out, "  {i} mode={}", format_mode(m.mode));
        if let Some(n) = &m.name {
            let _ = write!(out, " name={}", quote(&n.0));
        }
        for l in &m.lines {
            let _ = write!(out, " {}", quote(&l.0));
        }
        out.push('\n');
    }
    out.push_str("end\n");
    out
}

fn format_mode(m: i32) -> String {
    if (0..=9).contains(&m) { m.to_string() } else { format!("0x{:x}", m as u32) }
}

/// An event: header, script, messages and the parody table (each table
/// only when it has records).
pub fn print_event(e: &Event) -> String {
    let mut out = format!("event {} label={}\n", e.number, quote(&e.label.0));
    out.push_str(&print_script(&e.script));
    if !e.messages.is_empty() {
        out.push_str(&print_messages("messages", &e.messages));
    }
    if !e.parody.is_empty() {
        out.push_str(&print_messages("parody", &e.parody));
    }
    out
}

// ---------------------------------------------------------------------------
// Parsing.

/// Splits a line into words: bare words, `key=value` (value may be quoted)
/// and quoted strings. `#` outside quotes starts a comment.
fn words(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            cur.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '#' => break,
            '"' => {
                quoted = true;
                cur.push(c);
            }
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if quoted {
        return Err("unterminated string".into());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

fn fields_of(ws: &[String]) -> Result<Vec<(String, String)>, String> {
    ws.iter()
        .map(|w| {
            w.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| format!("expected name=value, got {w}"))
        })
        .collect()
}

/// Builds from `name=value` words, requiring each field once and no others.
fn build<T>(
    name: &str,
    fields: &[&'static str],
    ws: &[String],
    make: impl FnOnce(&mut dyn FnMut(&'static str) -> Option<String>) -> Result<T, String>,
) -> Result<T, String> {
    let kv = fields_of(ws)?;
    for (k, _) in &kv {
        if !fields.contains(&k.as_str()) {
            return Err(format!("{name} has no field {k}"));
        }
        if kv.iter().filter(|(k2, _)| k2 == k).count() > 1 {
            return Err(format!("{name}: {k} given twice"));
        }
    }
    let mut get = |n: &'static str| kv.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    make(&mut get)
}

pub fn parse_tag(ws: &[String]) -> Result<Tag, String> {
    let (name, rest) = ws.split_first().ok_or("empty setting")?;
    let k = TagKind::from_name(name).ok_or_else(|| format!("no setting named {name}"))?;
    build(name, k.fields(), rest, |g| k.build_text(g))
}

pub fn parse_cond(ws: &[String]) -> Result<Cond, String> {
    let (name, rest) = ws.split_first().ok_or("empty condition")?;
    let k = CondKind::from_name(name).ok_or_else(|| format!("no condition named {name}"))?;
    build(name, k.fields(), rest, |g| k.build_text(g))
}

pub fn parse_op(ws: &[String]) -> Result<Op, String> {
    let (name, rest) = ws.split_first().ok_or("empty instruction")?;
    let k = OpKind::from_name(name).ok_or_else(|| format!("no instruction named {name}"))?;
    build(name, k.fields(), rest, |g| k.build_text(g))
}

/// Lines that carry words, with 1-based line numbers.
struct Lines {
    lines: Vec<(usize, Vec<String>)>,
    pos: usize,
}

impl Lines {
    fn new(text: &str) -> Result<Lines, ParseError> {
        let mut lines = Vec::new();
        for (i, l) in text.lines().enumerate() {
            let ws = words(l).map_err(|msg| ParseError { line: i + 1, msg })?;
            if !ws.is_empty() {
                lines.push((i + 1, ws));
            }
        }
        Ok(Lines { lines, pos: 0 })
    }
    fn peek(&self) -> Option<&(usize, Vec<String>)> {
        self.lines.get(self.pos)
    }
    fn next(&mut self) -> Option<(usize, Vec<String>)> {
        let l = self.lines.get(self.pos).cloned();
        self.pos += 1;
        l
    }
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        let line = self.lines.get(self.pos.saturating_sub(1)).map(|l| l.0).unwrap_or(0);
        Err(ParseError { line, msg: msg.into() })
    }
}

fn at<T>(line: usize, r: Result<T, String>) -> Result<T, ParseError> {
    r.map_err(|msg| ParseError { line, msg })
}

fn parse_script_lines(ls: &mut Lines) -> Result<Script, ParseError> {
    let mut s = Script::default();
    match ls.next() {
        Some((_, w)) if w.len() == 1 && w[0] == "open" => {}
        _ => return ls.err("expected open"),
    }
    while let Some((line, w)) = ls.next() {
        match w[0].as_str() {
            "if" if s.blocks.is_empty() => s.open.push(at(line, parse_cond(&w[1..]))?),
            "block" => {
                if w.len() > 2 {
                    return ls.err("block takes at most its index");
                }
                if let Some(n) = w.get(1) {
                    let n = at(line, parse_int(n).ok_or_else(|| format!("bad block index {n}")))?;
                    if n != s.blocks.len() as i64 {
                        return ls.err(format!("block {n} is block {}", s.blocks.len()));
                    }
                }
                s.blocks.push(Block::default());
            }
            "end" if w.len() == 1 => return Ok(s),
            _ => {
                let Some(b) = s.blocks.last_mut() else {
                    return ls.err(format!("{} before the first block", w[0]));
                };
                match w[0].as_str() {
                    "set" => {
                        if !b.conds.is_empty() || !b.ops.is_empty() {
                            return ls.err("set after if or an instruction");
                        }
                        b.tags.push(at(line, parse_tag(&w[1..]))?);
                    }
                    "if" => {
                        if !b.ops.is_empty() {
                            return ls.err("if after an instruction");
                        }
                        b.conds.push(at(line, parse_cond(&w[1..]))?);
                    }
                    _ => b.ops.push(at(line, parse_op(&w))?),
                }
            }
        }
    }
    ls.err("no end")
}

fn parse_messages_lines(ls: &mut Lines) -> Result<Vec<Message>, ParseError> {
    let mut out = Vec::new();
    while let Some((line, w)) = ls.next() {
        if w.len() == 1 && w[0] == "end" {
            return Ok(out);
        }
        let idx = at(line, parse_int(&w[0]).ok_or_else(|| format!("expected a record index, got {}", w[0])))?;
        if idx != out.len() as i64 {
            return ls.err(format!("record {idx} is record {}", out.len()));
        }
        let mut m = Message::default();
        let mut have_mode = false;
        for word in &w[1..] {
            if let Some(v) = word.strip_prefix("mode=") {
                let v = at(line, parse_int(v).ok_or_else(|| format!("bad mode {v}")))?;
                m.mode = at(
                    line,
                    i32::try_from(v).or_else(|_| u32::try_from(v).map(|u| u as i32)).map_err(|e| e.to_string()),
                )?;
                have_mode = true;
            } else if let Some(v) = word.strip_prefix("name=") {
                if !m.lines.is_empty() || m.name.is_some() {
                    return ls.err("name= comes once, before the lines");
                }
                m.name = Some(GameText(at(line, unquote(v))?));
            } else if word.starts_with('"') {
                if m.lines.len() == 3 {
                    return ls.err("at most three lines");
                }
                m.lines.push(GameText(at(line, unquote(word))?));
            } else {
                return ls.err(format!("unexpected {word}"));
            }
        }
        if !have_mode {
            return ls.err("missing mode=");
        }
        out.push(m);
    }
    ls.err("no end")
}

/// A script from `open` to `end`.
pub fn parse_script(text: &str) -> Result<Script, ParseError> {
    let mut ls = Lines::new(text)?;
    let s = parse_script_lines(&mut ls)?;
    if ls.peek().is_some() {
        ls.pos += 1;
        return ls.err("text after end");
    }
    Ok(s)
}

/// A message table, from its `messages`/`parody` line to `end`.
pub fn parse_messages(text: &str) -> Result<Vec<Message>, ParseError> {
    let mut ls = Lines::new(text)?;
    match ls.next() {
        Some((_, w)) if w.len() == 1 && (w[0] == "messages" || w[0] == "parody") => {}
        _ => return ls.err("expected messages or parody"),
    }
    parse_messages_lines(&mut ls)
}

fn parse_event_lines(ls: &mut Lines) -> Result<Event, ParseError> {
    let (line, w) = ls.next().ok_or(ParseError { line: 0, msg: "empty".into() })?;
    if w[0] != "event" || w.len() < 2 || w.len() > 3 {
        return ls.err("expected event NUMBER [label=\"...\"]");
    }
    let mut e = Event {
        number: at(
            line,
            parse_int(&w[1]).and_then(|v| u16::try_from(v).ok()).ok_or_else(|| format!("bad event number {}", w[1])),
        )?,
        ..Event::default()
    };
    if let Some(l) = w.get(2) {
        let v = l.strip_prefix("label=").ok_or(ParseError { line, msg: format!("unexpected {l}") })?;
        e.label = GameText(at(line, unquote(v))?);
    }
    e.script = parse_script_lines(ls)?;
    while let Some((_, w)) = ls.peek() {
        match (w.len(), w[0].as_str()) {
            (1, "messages") if e.messages.is_empty() => {
                ls.pos += 1;
                e.messages = parse_messages_lines(ls)?;
            }
            (1, "parody") if e.parody.is_empty() => {
                ls.pos += 1;
                e.parody = parse_messages_lines(ls)?;
            }
            _ => break,
        }
    }
    Ok(e)
}

/// One event.
pub fn parse_event(text: &str) -> Result<Event, ParseError> {
    let mut ls = Lines::new(text)?;
    let e = parse_event_lines(&mut ls)?;
    if ls.peek().is_some() {
        ls.pos += 1;
        return ls.err("text after the event");
    }
    Ok(e)
}

/// Any number of events, one after another.
pub fn parse_events(text: &str) -> Result<Vec<Event>, ParseError> {
    let mut ls = Lines::new(text)?;
    let mut out = Vec::new();
    while ls.peek().is_some() {
        out.push(parse_event_lines(&mut ls)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Cmp;

    const SAMPLE: &str = "\
event 900 label=\"TEST 01\"
open
  if event_done event=0   # the boot event
  if game_status status=2
block 0
  set phase phase=0 comp=eq
  frame_rate rate=2
  message msg=1
block 1
  set phase phase=4 comp=ge
  if operate num=-1 except=1
  message msg=0
  repeatable
end
messages
  0 mode=0 name=\"Hero\" \"Not yet.\"
  1 mode=3 \"Go?\" \"\\x81\\x40\"
end
";

    #[test]
    fn sample_round_trips() {
        let e = parse_event(SAMPLE).unwrap();
        assert_eq!(e.number, 900);
        assert_eq!(e.script.blocks.len(), 2);
        assert_eq!(e.script.blocks[1].tags[0], Tag::Phase { phase: 4, comp: Cmp::Ge });
        assert!(e.messages[1].is_question());
        let printed = print_event(&e);
        assert_eq!(parse_event(&printed).unwrap(), e);
        assert_eq!(print_event(&parse_event(&printed).unwrap()), printed);
    }

    #[test]
    fn strings() {
        for bytes in [
            b"plain".to_vec(),
            b"quote \" and \\ back".to_vec(),
            vec![0x82, 0xa0, 0x82, 0xa2],
            vec![0xff, 0x00, 0x81],
            vec![0xb1, b'#', b'0'],
        ] {
            let q = quote(&bytes);
            assert_eq!(unquote(&q).unwrap(), bytes, "{q}");
        }
        assert_eq!(quote(&[0x82, 0xa0]), "\"\u{3042}\"");
    }

    #[test]
    fn errors_name_the_line() {
        let bad = "open\nblock\n  frame_rate speed=2\nend\n";
        let e = parse_script(bad).unwrap_err();
        assert_eq!(e.line, 3);
        assert!(parse_script("open\nblock 1\nend\n").is_err());
        assert!(parse_script("open\nblock\n  wait count=1\n  if member pc=0\nend\n").is_err());
    }
}
