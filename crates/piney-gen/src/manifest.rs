//! The manifest the generator works from: each group of tables the port
//! reads, entry by entry (`plans/volumes.md`, "The generator").
//!
//! An entry names the Rust field, Infection's address, its layout and the
//! overlay loaded when the game reads it. The addresses are the ones the
//! port's readers used; the INF citations in the crates stay beside the
//! readers.

use std::cell::RefCell;
use std::rc::Rc;

use crate::dtype;
use crate::layout::{Count, CustomFn, Layout, Num, Read, StructDef, Until, Value};
use crate::locate::find;
use crate::sinit;
use crate::talk;
use crate::volume::{Ctx, Vol};

/// A field of a group. `inf` is Infection's address; or `after` is
/// (another entry's name, bytes past it): what follows that entry in the
/// same volume (a font's glyphs the game reads past its end). With neither,
/// its layout is a custom one that finds its own place.
pub struct Entry {
    pub name: &'static str,
    pub inf: Option<u32>,
    pub layout: Layout,
    pub overlay: Option<&'static str>,
    pub doc: &'static str,
    pub after: Option<(&'static str, i64)>,
    /// Where the volume has no such table (Quarantine's desktop has no
    /// staff roll): the value that stands for it.
    pub absent: Vec<(Vol, Value)>,
    /// Placed where the carry (or the volume's name) puts it, without the
    /// vote by likeness: an entry read past its table's end, whose bytes
    /// there are the volume's own.
    pub carried: bool,
}

impl Entry {
    fn after(mut self, base: &'static str, off: i64) -> Entry {
        self.after = Some((base, off));
        self
    }

    fn absent(mut self, v: Vol, value: Value) -> Entry {
        self.absent.push((v, value));
        self
    }

    fn carried(mut self) -> Entry {
        self.carried = true;
        self
    }
}

pub struct Group {
    pub name: &'static str,
    pub rust: &'static str,
    pub doc: &'static str,
    pub entries: Vec<Entry>,
}

fn group(name: &'static str, rust: &'static str, doc: &'static str, entries: Vec<Entry>) -> Group {
    Group { name, rust, doc, entries }
}

/// An entry at Infection's `inf`.
fn e(name: &'static str, inf: u32, layout: Layout, overlay: Option<&'static str>, doc: &'static str) -> Entry {
    Entry { name, inf: Some(inf), layout, overlay, doc, after: None, absent: Vec::new(), carried: false }
}

/// An entry placed by its own layout, or `after` another.
fn derived(name: &'static str, layout: Layout, overlay: Option<&'static str>, doc: &'static str) -> Entry {
    Entry { name, inf: None, layout, overlay, doc, after: None, absent: Vec::new(), carried: false }
}

// The overlay loaded when the game reads an entry.
const MAIN: Option<&str> = None;
const GCMN: Option<&str> = Some("gcmn");
const DEMO: Option<&str> = Some("demo");
const DESKTOP: Option<&str> = Some("desktop");
const TOPPAGE: Option<&str> = Some("toppage");

const U8: Layout = Layout::Num(Num::U8);
const I8: Layout = Layout::Num(Num::I8);
const I16: Layout = Layout::Num(Num::I16);
const U16: Layout = Layout::Num(Num::U16);
const U32: Layout = Layout::Num(Num::U32);
const I32: Layout = Layout::Num(Num::I32);
const U64: Layout = Layout::Num(Num::U64);

fn cstr() -> Layout {
    Layout::CStr
}

fn float() -> Layout {
    Layout::Float
}

fn fixed_text(n: u32) -> Layout {
    Layout::FixedText(n)
}

fn text_rows(max: usize, stride: u32) -> Layout {
    Layout::TextRows { max, stride }
}

fn bytes(n: usize) -> Layout {
    Layout::Bytes { n: Count::N(n), blank: Vec::new() }
}

fn counted_i16s(max: i16) -> Layout {
    Layout::CountedI16s(max)
}

fn omit() -> Layout {
    Layout::Omit
}

fn lines_counted(delta: i32) -> Layout {
    Layout::LinesCounted(delta)
}

fn counted_ptr(inner: Layout, delta: i32) -> Layout {
    Layout::CountedPtr { inner: Box::new(inner), delta }
}

fn slots() -> Layout {
    Layout::Slots(16)
}

fn addr() -> Layout {
    Layout::Addr
}

fn lines(n: usize) -> Layout {
    Layout::Lines(n)
}

fn ptr(inner: Layout) -> Layout {
    Layout::Ptr(Box::new(inner))
}

fn opt(inner: Layout) -> Layout {
    Layout::Opt(Box::new(inner))
}

fn fixed(inner: Layout, n: usize) -> Layout {
    Layout::Fixed { inner: Box::new(inner), n, empty: None }
}

/// A fixed array whose trailing `empty` rows are dropped.
fn fixed_empty(inner: Layout, n: usize, empty: Value) -> Layout {
    Layout::Fixed { inner: Box::new(inner), n, empty: Some(empty) }
}

fn array(inner: Layout, n: usize) -> Layout {
    Layout::Array { inner: Box::new(inner), n: Count::N(n), until: None, through: false, limit: 4096, stride: None }
}

/// `n` rows, the count the volume's.
fn array_by(inner: Layout, n: Rc<dyn Fn(&Ctx) -> usize>) -> Layout {
    Layout::Array { inner: Box::new(inner), n: Count::By(n), until: None, through: false, limit: 4096, stride: None }
}

/// The rows before the first `until` holds for (a table that ends in a
/// terminator row).
fn array_until(inner: Layout, until: Until) -> Layout {
    Layout::Array {
        inner: Box::new(inner),
        n: Count::N(0),
        until: Some(until),
        through: false,
        limit: 4096,
        stride: None,
    }
}

/// The rows up to and with the first `until` holds for (a table whose
/// reader walks onto its end row).
/// A `boss05*ActTbl` through its closing -1: the words read as patterns
/// and their operands (one for 2, 6, 10, 11 and 12, two for 9), so pattern
/// 10's own -1 operand does not end it.
fn gorre_patterns() -> Until {
    let owed = std::cell::Cell::new(0u32);
    Rc::new(move |v| {
        if owed.get() > 0 {
            owed.set(owed.get() - 1);
            return false;
        }
        let w = v.int();
        owed.set(match w {
            2 | 6 | 10 | 11 | 12 => 1,
            9 => 2,
            _ => 0,
        });
        w == -1
    })
}

fn array_through(inner: Layout, until: Until) -> Layout {
    Layout::Array {
        inner: Box::new(inner),
        n: Count::N(0),
        until: Some(until),
        through: true,
        limit: 4096,
        stride: None,
    }
}

/// Rows `stride` bytes apart, where a row holds more than the port reads
/// (ccSpriteColorTable's 8-byte entries, 4 of them RGBA).
fn array_stride(inner: Layout, n: usize, stride: u32) -> Layout {
    Layout::Array {
        inner: Box::new(inner),
        n: Count::N(n),
        until: None,
        through: false,
        limit: 4096,
        stride: Some(stride),
    }
}

/// A struct of the manifest's own.
fn strukt(name: &str, size: u32, fields: Vec<(&str, u32, Layout)>, doc: &str) -> Layout {
    Layout::Struct(Rc::new(StructDef {
        name: name.to_string(),
        size,
        fields: fields.into_iter().map(|(n, off, l)| (n.to_string(), off, l)).collect(),
        doc: doc.to_string(),
        memory: false,
    }))
}

/// The struct Infection's DWARF declares as `name`, a member's layout
/// overridden by its name (dotted for a nested one).
fn ty(name: &str, over: Vec<(&str, Layout)>) -> Layout {
    dtype::ty(name, over.into_iter().map(|(k, l)| (k.to_string(), l)).collect())
}

/// A table indexed by one of The World's fixed sets (`crate::world`): one
/// `inner` per variant, in the game's order. The group writes it as a
/// method on the enum (`Server::Delta.symbol()`), not a field.
fn keyed(enum_: &'static str, variants: &[&'static str], inner: Layout) -> Layout {
    Layout::Keyed { enum_, variants: variants.to_vec(), inner: Box::new(inner) }
}

/// A value read by `f`, written as `layout`.
fn custom(f: CustomFn, layout: Layout) -> Layout {
    Layout::Custom { f, layout: Box::new(layout) }
}

/// A count by volume: Infection's, then the later volumes'.
fn later(inf: usize, later: usize) -> Rc<dyn Fn(&Ctx) -> usize> {
    Rc::new(move |c| if c.volume == Vol::Inf { inf } else { later })
}

/// A row whose field `i` is the text `t` (a table's terminator row).
fn titled(i: usize, t: &'static [u8]) -> Until {
    Rc::new(move |r| matches!(&r.list()[i], Value::Bytes(b) if b == t))
}

/// `[0, n)`: a table that maps each index to itself.
fn identity(n: i128) -> Value {
    Value::List((0..n).map(Value::Int).collect())
}

/// `saveSysMsg` as `__sinit_sdmng.cpp` fills it (run in eemu for the
/// volume, `sinit`), each message's first five NUL-separated pieces as
/// ccKanjiStrSeparate reads them (a reader stops at its own count, or at
/// the first empty one); None where the entry is NULL.
fn save_sys_msg_lines(c: &Ctx) -> Read {
    let mut out = Vec::new();
    for &va in sinit::save_sys_msg(c.volume).iter() {
        out.push(if va == 0 { Value::None } else { lines(5).read(c, va)? });
    }
    Ok(Value::List(out))
}

/// An array indexed by `volumeNum - 1`: the volume's own element.
fn by_volume(inf: u32, layout: Layout) -> CustomFn {
    Rc::new(move |c| layout.read(c, find(c, inf, None) + 4 * c.volume.index() as u32))
}

fn sext16(w: u32) -> i128 {
    i128::from(w as u16 as i16)
}

/// A table only the later volumes have, reached through the code that
/// reads it: the function at Infection's `caller` calls, as its `nth`
/// callee without a name (from 0, in the order of the calls), a function
/// Infection lacks, whose first `lui`/`addiu` pair builds the table's
/// address.
fn callee_table(caller: u32, nth: usize, layout: Layout) -> CustomFn {
    Rc::new(move |c| {
        let f = find(c, caller, None);
        let size = match c.p.symbol_at(f, 0) {
            Some((s, 0)) => s.size,
            _ => return Err(format!("no function starts at 0x{f:08x}")),
        };
        let mut unnamed = Vec::new();
        for k in 0..size / 4 {
            let w = c.p.u32(f + 4 * k)?;
            if w >> 26 == 3 {
                let t = (w & 0x03ff_ffff) << 2;
                if !matches!(c.p.symbol_at(t, 0), Some((_, 0))) {
                    unnamed.push(t);
                }
            }
        }
        let t = *unnamed.get(nth).ok_or_else(|| format!("0x{f:08x} makes {} unnamed calls", unnamed.len()))?;
        for k in 0..64 {
            let (w, x) = (c.p.u32(t + 4 * k)?, c.p.u32(t + 4 * k + 4)?);
            if w == 0x03e0_0008 {
                break;
            }
            if w >> 26 == 0x0f && x >> 26 == 0x09 && (x >> 21) & 31 == (w >> 16) & 31 {
                return layout.read(c, ((w & 0xffff) << 16).wrapping_add(sext16(x) as u32));
            }
        }
        Err(format!("0x{t:08x} builds no table"))
    })
}

/// The cinema's name for a skill (OUT gcmn 0x00472550): the lookup the
/// function after `CinemaOn__19ccBossEffCinemaFadeFi` calls first
/// (Outbreak's `CinemaOn` with a skill, 0x0047c0a0), run in eemu for
/// `game.field` 0-31 and every skill below 512; the rows it answers (none
/// where the row lacks a file or texture). None where `CinemaOn` is
/// followed by a named function (`CinemaOff`: Infection, Mutation).
fn cinema_skill_rows(c: &Ctx) -> Read {
    use piney_eemu::cpu::Features;
    const GAME: u32 = 0x0100_0000;
    let on = find(c, 0x0046_A9B0, None);
    let size = match c.p.symbol_at(on, 0) {
        Some((s, 0)) => s.size,
        _ => return Err(format!("no function starts at 0x{on:08x}")),
    };
    let next = (on + size + 15) & !15;
    if matches!(c.p.symbol_at(next, 0), Some((_, 0))) {
        return Ok(Value::List(Vec::new()));
    }
    let mut look = None;
    for k in 0..64 {
        let w = c.p.u32(next + 4 * k)?;
        if w >> 26 == 3 {
            look = Some((w & 0x03ff_ffff) << 2);
            break;
        }
        if w == 0x03e0_0008 {
            break;
        }
    }
    let f = look.ok_or_else(|| format!("0x{next:08x} calls nothing"))?;
    // `game`: the first `lw $vN, off($gp)`.
    let gp = c.p.gp.ok_or("no $gp")?;
    let mut game = None;
    for k in 0..8 {
        let w = c.p.u32(f + 4 * k)?;
        if w >> 26 == 0x23 && (w >> 21) & 31 == 28 {
            game = Some(gp.wrapping_add(sext16(w) as u32));
            break;
        }
    }
    let game = game.ok_or_else(|| format!("0x{f:08x} reads no game"))? as usize;
    let mut m = sinit::machine(&c.p, Features::default());
    m.ram[game..game + 4].copy_from_slice(&GAME.to_le_bytes());
    m.ram[GAME as usize..GAME as usize + 0x100].fill(0);
    let mut out = Vec::new();
    for field in 0..32u32 {
        m.ram[GAME as usize + 0x24..GAME as usize + 0x28].copy_from_slice(&field.to_le_bytes());
        for sid in 0..512u32 {
            let r = m.call(f, &[sid], 10_000).map_err(|e| format!("0x{f:08x}({sid}): {e:?}"))?;
            if r == 0 {
                continue;
            }
            out.push(Value::List(vec![
                Value::Int(i128::from(field)),
                Value::Int(i128::from(sid)),
                opt(cstr()).read(c, r)?,
                opt(cstr()).read(c, r + 4)?,
                I32.read(c, r + 8)?,
            ]));
        }
    }
    Ok(Value::List(out))
}

/// The pages `ccUseItemRequest` (gcmn 0x0057aa80) shows for important
/// item `item`: the first `lui`/`addiu` pair where its `li n; beq` goes.
/// Outbreak and Quarantine rewrote notes 288 and 289, and the carry pairs
/// Infection's two tables with each other's (OUT gcmn 0x0059e0a0,
/// 0x0059e93c for 288, 0x0059e964 for 289).
fn item_pages(item: u32, layout: Layout) -> CustomFn {
    Rc::new(move |c| {
        let f = find(c, 0x0057_AA80, None);
        let size = match c.p.symbol_at(f, 0) {
            Some((s, 0)) => s.size,
            _ => return Err(format!("no function starts at 0x{f:08x}")),
        };
        let word = |k: u32| c.p.u32(f + 4 * k);
        for k in 0..size / 4 - 1 {
            let (w, x) = (word(k)?, word(k + 1)?);
            // `addiu $rt, $zero, item`, then `beq $rs, $rt`.
            if w >> 21 != 0x09 << 5 || w & 0xffff != item || x >> 26 != 0x04 || (x >> 16) & 31 != (w >> 16) & 31 {
                continue;
            }
            let to = (k as i128 + 2 + sext16(x)) as u32;
            for q in to..(to + 40).min(size / 4 - 1) {
                let (hi, lo) = (word(q)?, word(q + 1)?);
                if hi >> 26 == 0x0f && lo >> 26 == 0x09 && (lo >> 21) & 31 == (hi >> 16) & 31 {
                    return layout.read(c, ((hi & 0xffff) << 16).wrapping_add(sext16(lo) as u32));
                }
            }
            return Err(format!("item {item}'s branch builds no table"));
        }
        Err(format!("0x{f:08x} tests no item {item}"))
    })
}

/// A member's remark table from Mutation on (none on Infection).
fn remark(name: &'static str, caller: u32, nth: usize, doc: &'static str) -> Entry {
    let rows = || array(opt(cstr()), 19);
    derived(name, custom(callee_table(caller, nth, rows()), rows()), GCMN, doc)
        .absent(Vol::Inf, Value::List(Vec::new()))
}

/// `ccCheckVoiceGrp` (main 0x0017fff0): its `sltiu` bound (+4), its jump
/// table (`lui`/`addiu` at +16, +20), each case `li $v0, n` or -1.
fn voice_groups(c: &Ctx) -> Read {
    let f = find(c, 0x0017_FFF0, None);
    let (lui, addiu) = (c.p.u32(f + 16)?, c.p.u32(f + 20)?);
    let table = ((lui & 0xffff) << 16).wrapping_add(sext16(addiu) as u32);
    let n = c.p.u32(f + 4)? & 0xffff;
    let mut out = Vec::new();
    for k in 0..n {
        let w = c.p.u32(c.p.u32(table + 4 * k)?)?;
        out.push(Value::Int(if w >> 16 == 0x2402 { sext16(w) } else { -1 }));
    }
    if out.len() != 17 {
        return Err(format!("ccCheckVoiceGrp at 0x{f:08x} has {} cases", out.len()));
    }
    Ok(Value::List(out))
}

/// `ccGetItemIcon`'s icon by category: its jump table (main 0x0034d250,
/// found in each volume) holds a case per category, `li $v0, n` or
/// `move $v0, $zero`.
fn item_icons(c: &Ctx) -> Read {
    let table = find(c, 0x0034_D250, None);
    let mut out = Vec::new();
    for k in 0..16 {
        let w = c.p.u32(c.p.u32(table + 4 * k)?)?;
        out.push(Value::Int(if w >> 16 == 0x2402 { sext16(w) } else { 0 }));
    }
    Ok(Value::List(out))
}

/// `saveSysMsg`: 64 messages, each None or its lines.
fn message_lines() -> Layout {
    Layout::MessageLines
}

/// Per mail, the `ReMail` rows its `oneRes` and `twoRes` copy (the mail
/// table's static initialiser, run for the volume): None for none.
fn reply_links() -> Layout {
    Layout::ReplyLinks
}

fn mail_row() -> Layout {
    ty("ccMailData", vec![("line", omit()), ("sentence", lines_counted(-4)), ("oneRes", omit()), ("twoRes", omit())])
}

fn remail_row() -> Layout {
    ty("ccReMailData", vec![("line", omit()), ("sentence", lines_counted(-4))])
}

/// `MailTbl` (`MailTblp`), as many rows as its initialiser fills.
fn mails(parody: bool) -> CustomFn {
    Rc::new(move |c| {
        let m = sinit::mail_links(c.volume, parody);
        array(mail_row(), m.links.len()).read(c, find(c, if parody { 0x0042_5280 } else { 0x0041_DFD0 }, DESKTOP))
    })
}

/// `ReMail` (`ReMailp`): the rows below `MailTbl` its initialiser copies.
fn remails(parody: bool) -> CustomFn {
    Rc::new(move |c| {
        let m = sinit::mail_links(c.volume, parody);
        array(remail_row(), m.remail_rows as usize).read(c, m.remail)
    })
}

/// Each mail's two replies, rows of `ReMail` (-1 none).
fn replies(parody: bool) -> CustomFn {
    Rc::new(move |c| {
        let m = sinit::mail_links(c.volume, parody);
        let row = |&(a, b): &(i32, i32)| Value::List(vec![Value::Int(i128::from(a)), Value::Int(i128::from(b))]);
        Ok(Value::List(m.links.iter().map(row).collect()))
    })
}

const SERVERS: [&str; 5] = ["Delta", "Theta", "Lambda", "Sigma", "Omega"];
const TOWNS: [&str; 5] = ["MacAnu", "DunLoireag", "CarminaGadelica", "FortOuph", "LiaFail"];

/// An unused trade slot: no item (id -1, category -1, none), switched off.
fn no_trade() -> Value {
    Value::List(vec![Value::List(vec![Value::Int(-1), Value::Int(-1), Value::Int(0)]), Value::Int(0)])
}

/// A `dtMenuElementData` row.
fn menu_element() -> Layout {
    strukt(
        "MenuElement",
        12,
        vec![
            ("name", 0, opt(cstr())),
            ("title", 4, opt(cstr())),
            (
                "data",
                8,
                opt(strukt(
                    "MenuData",
                    0,
                    vec![("disp", 0, I16), ("width", 2, I16), ("items", 4, counted_i16s(16))],
                    "A menu list's data: where it shows, its width and its items (at most 16).",
                )),
            ),
        ],
        "A `dtMenuElementData` row: the list's name, title and data.",
    )
}

/// A comment that is two NUL-separated lines (the artist or maker, then
/// the caption: ccKanjiStrSeparate(comment, 1)).
fn two_lines() -> Layout {
    opt(lines(2))
}

/// The top page's (toppage.prg) board thread: its posts.
fn bbs_thread() -> Layout {
    ty(
        "ccBBSThreadList",
        vec![
            ("msgList", counted_ptr(ty("ccBBSMsgList", vec![("message", lines_counted(-4)), ("maxLines", omit())]), 4)),
            ("msgNum", omit()),
        ],
    )
}

/// A `menuElementData` row (the field UI's).
fn field_menu_element() -> Layout {
    strukt(
        "FieldMenuElement",
        12,
        vec![
            ("name", 0, opt(cstr())),
            ("title", 4, opt(cstr())),
            (
                "data",
                8,
                opt(strukt(
                    "FieldMenuData",
                    0,
                    vec![("disp", 0, I16), ("width", 2, I16), ("items", 4, counted_i16s(64))],
                    "A field menu list's data: how it shows, its width in cells and its items.",
                )),
            ),
        ],
        "A `menuElementData` row: the list's name, title and data.",
    )
}

/// A talk record (`ccEvMsgData`), its text three lines.
pub fn ev_msg() -> Layout {
    ty("ccEvMsgData", vec![("str", opt(lines(3)))])
}

/// A `char *` to `n` NUL-separated lines (`ccKanjiStrSeparate` 0..n-1).
fn text_lines(n: usize) -> Layout {
    ptr(lines(n))
}

// The battle's and the menus' parameters (gcmn): skills, items, equipment,
// enemies, bosses, level-ups, the party's AI. A description (`str`) is its
// three lines (`ccKanjiStrSeparate` 0..2).

/// ccEntry (in the enemies', gimmicks' and people's rows): the pointers the
/// running game fills left out; `anm` is the row's animation names
/// (`char[][30]`: the attacks 0-5, wait 6, walk 7, the flinches, dying and
/// some, the 16 the motion code reads; none for a middle boss), `clut` the
/// CLUT its model takes.
fn entry_over() -> Vec<(&'static str, Layout)> {
    vec![
        ("entry.func", Layout::Func("EntryFunc")),
        ("entry.ep", omit()),
        ("entry.ccsc", omit()),
        ("entry.ccsc2", omit()),
        ("entry.anm", opt(text_rows(16, 30))),
        ("entry.clut", fixed_text(30)),
    ]
}

/// An enemy's row: its entry, and its four skills' descriptions as lines.
fn enemy_over() -> Vec<(&'static str, Layout)> {
    let mut v = entry_over();
    for k in ["skill.atc0.str", "skill.atc1.str", "skill.ski0.str", "skill.ski1.str"] {
        v.push((k, opt(lines(3))));
    }
    v
}

fn skill() -> Layout {
    ty("ccSkillParam", vec![("str", opt(lines(3)))])
}

fn item() -> Layout {
    ty("ccItemParam", vec![("str", opt(lines(3)))])
}

fn equip() -> Layout {
    ty("ccEquipmentParam", vec![("str", opt(lines(3)))])
}

fn item_list() -> Layout {
    ty("ccItemList", vec![])
}

/// What an equipment row's numbers read as where there is no row.
fn equip_before() -> Layout {
    strukt(
        "EquipBefore",
        0x48,
        vec![
            ("elm", 0x0A, ty("ccCharParamElement", vec![])),
            ("beff", 0x2A, ty("ccBattleEffect", vec![])),
            ("level", 0x38, I16),
            ("skill_id", 0x3A, fixed(I16, 3)),
            ("price", 0x40, I32),
        ],
        "What an equipment row's numbers read as where there is no row (id -1, the bytes before a table).",
    )
}

// The talk pages' records by address (`talk`): every record array and
// pointer table the people's lines reach, where the volume keeps it, since
// the save and npcTbl hold those addresses.

fn msg_records() -> Layout {
    let record_va = strukt(
        "RecordVa",
        12,
        vec![("name", 4, U32), ("str", 8, U32)],
        "Where a record's name and text are: the addresses it holds.",
    );
    strukt(
        "MsgRecords",
        0,
        vec![("va", 0, U32), ("records", 4, array(ev_msg(), 0)), ("addresses", 8, array(record_va, 0))],
        "A record array (`ccEvMsgData[]`), where the volume keeps it, and the addresses its records hold (a \
         table read as text shows them).",
    )
}

fn msg_pointers() -> Layout {
    strukt(
        "MsgPointers",
        0,
        vec![("va", 0, U32), ("words", 4, array(U32, 0))],
        "A pointer table (`ccEvMsgData *[]` or `**[]`), its words the addresses it holds.",
    )
}

fn title() -> Group {
    group(
        "title",
        "Title",
        "The title's texts: the save's time-idol ranks, the memory-card dialog's words and messages.",
        vec![
            e(
                "time_idol_rank",
                0x0033_EB90,
                array(ptr(cstr()), 10),
                MAIN,
                "`timeIdolRankDefStr`: a name then a time for each of the five ranks.",
            ),
            e("yes", 0x0037_83EC, ptr(cstr()), DEMO, "`STR_YES`"),
            e("no", 0x0037_83F0, ptr(cstr()), DEMO, "`STR_NO`"),
            e("lv", 0x0037_83F4, ptr(cstr()), DEMO, "`STR_LV`"),
            e("alltime", 0x0037_83F8, ptr(cstr()), DEMO, "`STR_ALLTIME`"),
            e("nodeta", 0x0037_83FC, ptr(cstr()), DEMO, "`STR_NODETA`"),
            e("memorycard", 0x0037_8400, ptr(cstr()), DEMO, "`STR_MEMORYCARD`"),
            e("slot1", 0x0037_8404, ptr(cstr()), DEMO, "`STR_SLOT1`"),
            e("slot2", 0x0037_8408, ptr(cstr()), DEMO, "`STR_SLOT2`"),
            e("data", 0x0037_840C, ptr(cstr()), DEMO, "`STR_DATA`"),
            e("clear", 0x0037_8410, ptr(cstr()), DEMO, "`STR_CLEAR`"),
            e("parody", 0x0037_8414, ptr(cstr()), DEMO, "`STR_PARODY`"),
            e("noflg_deta", 0x0037_8418, ptr(cstr()), DEMO, "`STR_NOFLG_DETA`"),
            e("highlight", 0x0040_EBA8, cstr(), DEMO, "demo.prg's highlight escape"),
            e("vol", 0x0040_EBD8, cstr(), DEMO, "demo.prg's volume label"),
            derived(
                "save_sys_msg",
                custom(Rc::new(save_sys_msg_lines), message_lines()),
                MAIN,
                "`saveSysMsg` by `result & 0xfff`: each message's first five pieces, None for none.",
            ),
            e(
                "mc_dir_names",
                0x0030_6BE0,
                array(ptr(cstr()), 4),
                MAIN,
                "`mcDirName`: each volume's save directory on the memory card, by `volumeNum - 1`, with its leading `/` (Infection names the later ones by placeholders).",
            ),
        ],
    )
}

// The bitmap fonts: 9 rows of glyphs each, which reaches glyph 130 (%Z),
// past the 112 drawn (piney_desktop::kanji).
fn fonts() -> Group {
    group(
        "fonts",
        "Fonts",
        "The two bitmap fonts in main that ccKanji draws ASCII with.",
        vec![
            e("ef8x16", 0x002F_5600, bytes(7 * 16 * 64), MAIN, "`ef8x16`: 8x16 glyphs, 16 a row, 64 bytes a row line."),
            derived(
                "ef8x16_past",
                bytes(2 * 16 * 64),
                MAIN,
                "The two glyph rows past `ef8x16`'s end, which glyphs 112-130 read: the volume's next globals.",
            )
            .after("ef8x16", 7 * 16 * 64),
            e("ef12x20", 0x002F_2180, bytes(9 * 20 * 96), MAIN, "`ef12x20`: 12x20 glyphs, 96 bytes a row line."),
        ],
    )
}

fn loaddisp() -> Group {
    group(
        "loaddisp",
        "LoadDisp",
        "The loading display's words (`ccLoadDisp`): the towns' cards, the servers' symbols.",
        vec![
            e(
                "card",
                0x0031_1730,
                keyed(
                    "Town",
                    &TOWNS,
                    strukt(
                        "TownCard",
                        12,
                        vec![("name", 0, ptr(cstr())), ("sub", 4, ptr(cstr()))],
                        "A town's loading card: its two lines (`townNameTbl`'s first two `char *`; the display does not read the third).",
                    ),
                ),
                MAIN,
                "`townNameTbl` (main 0x00311730): the town's loading card.",
            ),
            e(
                "symbol",
                0x0031_1770,
                keyed("Server", &SERVERS, ptr(cstr())),
                MAIN,
                "`serverNameTbl` (0x00311770): the server's symbol (Shift-JIS).",
            ),
            e("server", 0x0035_3F98, cstr(), MAIN, "\"Server\" (0x00353f98), after the symbol."),
        ],
    )
}

fn newgame() -> Group {
    group(
        "newgame",
        "NewGame",
        "What `ccSaveData::NewGame` copies from the disc: `charTbl` and the default trade lists.",
        vec![
            e(
                "chars",
                0x0040_DC80,
                array_by(ty("ccSpcParamData", vec![]), later(18, 21)),
                DEMO,
                "`charTbl` (demo.prg; main from Outbreak on): 18 characters, 21 from Mutation on.",
            ),
            e(
                "spc_trade",
                0x0034_5F10,
                array_by(fixed_empty(ty("ccTradeList", vec![]), 16, no_trade()), later(17, 20)),
                MAIN,
                "`spcDefTradeList`: characters 1-17 (1-20 from Mutation on), each its trades (up to 16; the rest are empty).",
            ),
            e(
                "npc_trade",
                0x0034_6790,
                array_by(fixed_empty(ty("ccTradeList", vec![]), 16, no_trade()), later(48, 54)),
                MAIN,
                "`npcDefTradeList`: 48 NPCs (54 from Mutation on), each its trades (up to 16).",
            ),
            e(
                "tpc_trade",
                0x0034_7F90,
                array(fixed(fixed(ty("ccItemList", vec![]), 4), 3), 24),
                MAIN,
                "`tpcTradeList`: per town PC, three lists of four items.",
            ),
            e(
                "spc_name_list",
                0x0038_7840,
                addr(),
                MAIN,
                "Where `spcNameList` (`char[17][24]`) is: the address the save's name pointers hold.",
            ),
            e("ccs_name_list", 0x0038_7600, addr(), MAIN, "Where `ccsNameList` (`char[n][32]`) is."),
        ],
    )
}

fn dtmenu() -> Group {
    group(
        "dtmenu",
        "DtMenu",
        "The desktop's (and the title's) system menu: `dtMenuElementData`, the option texts and the save menu's.",
        vec![
            e("elements", 0x0030_6F70, array(menu_element(), 13), MAIN, "`dtMenuElementData`: the 13 lists."),
            e("dialog", 0x0037_7DAC, ptr(slots()), MAIN, "`dialogDefault`: the dialog's two answers."),
            e("reset_info", 0x0037_7D70, ptr(cstr()), MAIN, "`resetMenuInfo[0]`"),
            e("reset_info2", 0x0037_7D74, ptr(lines(2)), MAIN, "`resetMenuInfo[1]`: two lines."),
            e("on_off", 0x0037_7DB0, ptr(slots()), MAIN, "`dialogOnOff`"),
            e("voice", 0x0037_7DB4, ptr(slots()), MAIN, "`dialogVoice`"),
            e("sound", 0x0037_7D9C, ptr(slots()), MAIN, "`soundMenuStr`"),
            e("ctrl", 0x0037_7D98, ptr(slots()), MAIN, "`ctrlMenuStr`: the controller types."),
            e("ctrl_btn", 0x0037_7DA0, ptr(slots()), MAIN, "`ctrlMenuStrBtn`: the buttons' actions."),
            e("ctrl_cam", 0x0037_7DA4, ptr(slots()), MAIN, "`ctrlMenuStrCam`: the camera's."),
            e("ctrl_mov", 0x0037_7DA8, ptr(slots()), MAIN, "`ctrlMenuStrMov`: the sticks'."),
            e(
                "vibration_info",
                0x0037_7D78,
                array(ptr(lines(2)), 2),
                MAIN,
                "`vibrationMenuInfo`: on and off, two lines each.",
            ),
            e("voice_info", 0x0037_7D88, array(ptr(lines(2)), 2), MAIN, "`voiceMenuInfo`: two lines each."),
            e("strwin_info", 0x0037_7D90, array(ptr(lines(2)), 2), MAIN, "`strwinMenuInfo`: two lines each."),
            e("help", 0x0033_E860, array(ptr(lines(2)), 4), MAIN, "`ctrlMenuHelp`: two lines each."),
            e("screen_x", 0x0034_C080, cstr(), MAIN, "The screen position's first label."),
            e("screen_y", 0x0034_C088, cstr(), MAIN, "Its second."),
            e(
                "clear_flag",
                0x0033_ECC0,
                custom(by_volume(0x0033_ECC0, ptr(lines(3))), lines(3)),
                MAIN,
                "`clearFlagStr[volumeNum - 1]`: the volume's question, three lines.",
            ),
            e("record", 0x0037_7E48, ptr(slots()), MAIN, "`recordMenuStr`: slot 1, slot 2, Unused, Data, MEMORY CARD."),
            e("colon", 0x0034_C090, cstr(), MAIN, "`@3200` \":\""),
            e("lv", 0x0034_C098, cstr(), MAIN, "`@3201` \"LV\""),
            e("time_max", 0x0034_C0A0, cstr(), MAIN, "`@3202` \"999:59:59\""),
        ],
    )
}

fn desktop() -> Group {
    group(
        "desktop",
        "Desktop",
        "The desktop's content tables (desktop.prg): wallpapers, music, movies, news.",
        vec![
            e(
                "walls",
                0x0042_B1B0,
                array_until(ty("WallData", vec![("Wall", omit()), ("comment", two_lines())]), titled(0, b"NULL")),
                DESKTOP,
                "`WallTbl`: the wallpapers, up to its row titled \"NULL\".",
            ),
            e(
                "waves",
                0x0042_B6F0,
                array_until(ty("WaveData", vec![("comment", two_lines())]), titled(0, b"NULL")),
                DESKTOP,
                "`Wave`: the jukebox's music, up to its row titled \"NULL\" (Mutation's \"BGM 51\" before it has category -1).",
            ),
            e(
                "streams",
                0x0041_B640,
                array_until(ty("StrData", vec![("comment", two_lines())]), titled(0, b"NULL")),
                DESKTOP,
                "`Stream`: the movies the Audio screen plays, up to its row named \"NULL\".",
            ),
            e(
                "stream_volumes",
                0x0041_BBC0,
                array(I32, 3),
                DESKTOP,
                "`AddStrList`'s bounds: a movie below the first is volume 1's, below the second 2's, below the third 3's.",
            ),
            e(
                "news",
                0x0041_C120,
                array_until(ty("HtmlData", vec![("html", omit())]), titled(2, b"NULL")),
                DESKTOP,
                "`HtmlTbl`: the news headlines and their pages, up to its row titled \"NULL\".",
            ),
            // The mailer's words (mailer.cpp's STR_ pointers).
            e("mail_yes", 0x0037_85B8, ptr(cstr()), DESKTOP, "The mailer's `STR_YES`."),
            e("mail_no", 0x0037_85BC, ptr(cstr()), DESKTOP, "The mailer's `STR_NO`."),
            e("mail_nores", 0x0037_85C0, ptr(cstr()), DESKTOP, "`STR_NORES`: a mail that takes no reply."),
            e("mail_send_comp", 0x0037_85C4, ptr(cstr()), DESKTOP, "`STR_SEND_COMP`: the reply is sent."),
            e("mail_sel_remail", 0x0037_85C8, ptr(cstr()), DESKTOP, "`STR_SEL_REMAIL`: choose a reply."),
            e("mail_send_mail", 0x0037_85CC, ptr(cstr()), DESKTOP, "`STR_SEND_MAIL`: send it?"),
            e("mail_newmail", 0x0037_85D0, ptr(cstr()), DESKTOP, "`STR_NEWMAIL`"),
            e(
                "mail_count",
                0x0037_85D4,
                ptr(lines(4)),
                DESKTOP,
                "`STR_MAIL`: the count's words (\"You have \", \" new mail.\", \"No new mail.\", \" new mails.\").",
            ),
            // The audio screen's (audio.cpp's).
            e("audio_str_mode", 0x0037_8574, ptr(cstr()), DESKTOP, "`STR_STR_MODE`"),
            e("audio_snd_mode", 0x0037_8578, ptr(cstr()), DESKTOP, "`STR_SND_MODE`"),
            e("audio_lock", 0x0037_857C, ptr(lines(11)), DESKTOP, "`STR_STR_LOCK`: eleven lines."),
        ],
    )
}

fn kanji() -> Group {
    group(
        "kanji",
        "Kanji",
        "What `ccKanji` and the sprites draw text with (main): the ASCII glyphs' trims, the colours, the blends.",
        vec![
            e(
                "english_font_ofs_s",
                0x002F_B5F0,
                array(I16, 128),
                MAIN,
                "`englishFontOfsS`: each glyph's trim for kt 0 and 1.",
            ),
            derived(
                "english_font_ofs_s_past",
                array(I16, 3),
                MAIN,
                "The three halfwords past `englishFontOfsS`'s end, which %X-%Z read: the volume's next global.",
            )
            .after("english_font_ofs_s", 256),
            e("english_font_ofs_l", 0x002F_B4F0, array(I16, 128), MAIN, "`englishFontOfsL`: the same for kt 2 and 3."),
            derived(
                "english_font_ofs_l_past",
                array(I16, 3),
                MAIN,
                "The three halfwords past `englishFontOfsL`'s end (`englishFontOfsS`'s first three).",
            )
            .after("english_font_ofs_l", 256),
            e(
                "sprite_color_table",
                0x002F_B430,
                array_stride(fixed(U8, 4), 24, 8),
                MAIN,
                "`ccSpriteColorTable`: the 24 RGBA colours the #R #G #B #Y escapes and the sprites index (0x80 = 1.0; each entry 8 bytes, the low four used).",
            ),
            e("alpha_blend_tbl", 0x0034_8580, array(U64, 9), MAIN, "`alphaBlendTbl`: the nine ALPHA register values."),
        ],
    )
}

fn staffroll() -> Group {
    group("staffroll", "StaffRoll", "The staff roll's pages (desktop.prg `g_srDataGrp`), its scene file and its timings.", vec![
        e("pages", 0x0042_C500, array_until(ty("ccSRDataGrp", vec![("data", counted_ptr(ty("ccSRData", vec![]), 4)), ("dataNum", omit())]), Rc::new(|r| r.list() == [Value::None, Value::None])), DESKTOP, "`g_srDataGrp`: the pages, each its credits (none for a picture alone) and picture; none on Quarantine, whose desktop has no staff roll.").absent(Vol::Qua, Value::List(Vec::new())),
        e("ccs", 0x0042_BBF0, custom(by_volume(0x0042_BBF0, ptr(cstr())), cstr()), DESKTOP, "`SR_CCS_NAME`: the volume's staff roll scene file (the table `ccThStaffRoll` indexes by `volumeNum - 1`); none on Quarantine.").absent(Vol::Qua, Value::Bytes(Vec::new())),
        e("rnd_fadein", 0x0037_8758, I32, DESKTOP, "`SR_RND_FADEIN_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("rnd_fadeout", 0x0037_875C, I32, DESKTOP, "`SR_RND_FADEOUT_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("bg_fix", 0x0037_8760, I32, DESKTOP, "`SR_BG_FIX_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("bg_fadein", 0x0037_8764, I32, DESKTOP, "`SR_BG_FADEIN_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("bg_fadeout", 0x0037_8768, I32, DESKTOP, "`SR_BG_FADEOUT_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("onlybg_fadein", 0x0037_876C, I32, DESKTOP, "`SR_ONLYBG_FADEIN_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("onlybg_fadeout", 0x0037_8770, I32, DESKTOP, "`SR_ONLYBG_FADEOUT_TIME`").absent(Vol::Qua, Value::Int(0)),
        e("onlybg_fix", 0x0037_8774, I32, DESKTOP, "`SR_ONLYBG_FIX_TIME`").absent(Vol::Qua, Value::Int(0)),
    ])
}

fn mail() -> Group {
    group(
        "mail",
        "Mail",
        "The mailer's tables (desktop.prg): `MailTbl`, the replies `ReMail`, and their Parody Mode twins.",
        vec![
            e(
                "mails",
                0x0041_DFD0,
                custom(mails(false), array(mail_row(), 0)),
                DESKTOP,
                "`MailTbl`: 326 mails, 375 from Mutation on.",
            ),
            derived(
                "replies",
                custom(replies(false), reply_links()),
                DESKTOP,
                "Each mail's two replies, as rows of [`Mail::remails`].",
            ),
            derived(
                "remails",
                custom(remails(false), array(remail_row(), 0)),
                DESKTOP,
                "`ReMail`: the replies (294; 364 from Mutation on).",
            ),
            e("mails_parody", 0x0042_5280, custom(mails(true), array(mail_row(), 0)), DESKTOP, "`MailTblp`."),
            derived(
                "replies_parody",
                custom(replies(true), reply_links()),
                DESKTOP,
                "`MailTblp`'s replies, as rows of `remails_parody`.",
            ),
            derived("remails_parody", custom(remails(true), array(remail_row(), 0)), DESKTOP, "`ReMailp`."),
        ],
    )
}

// The name entry (NameEntry.cpp in desktop.prg; the STR_ pointers are main's,
// which __sinit_NameEntry.cpp copies into InfoMsg, ResetMsg and StrMsg).
fn nameentry() -> Group {
    group(
        "nameentry",
        "NameEntry",
        "The name entry's texts, the names it refuses and its grid of letters.",
        vec![
            e("yes", 0x0037_8778, ptr(cstr()), DESKTOP, "`STR_YES_J`"),
            e("no", 0x0037_877C, ptr(cstr()), DESKTOP, "`STR_NO_J`"),
            e(
                "str_msg",
                0x0037_8784,
                array(ptr(lines(4)), 3),
                DESKTOP,
                "`STR_NAME`, `STR_SUR_NAME`, `STR_GAME_NAME` (`StrMsg[0..3]`): four lines each.",
            ),
            e(
                "reset",
                0x0037_8790,
                array(ptr(cstr()), 3),
                DESKTOP,
                "`STR_NAME_RESET`, `STR_SURNAME_RESET`, `STR_GAMENAME_RESET` (`ResetMsg`).",
            ),
            e("enter", 0x0037_879C, ptr(cstr()), DESKTOP, "`STR_ENTER2`"),
            e("all_reset", 0x0037_87A0, ptr(cstr()), DESKTOP, "`STR_ALLNAME_RESET`"),
            e("err_name", 0x0037_87A4, ptr(lines(4)), DESKTOP, "`STR_ERR_NAME` (`StrMsg[3]`): the refusals."),
            e("affirmation", 0x0037_87A8, ptr(lines(2)), DESKTOP, "`STR_AFFIRMATION2`: the question's two lines."),
            e("entry_name", 0x0037_87AC, ptr(cstr()), DESKTOP, "`STR_ENTRY_NAME`"),
            e("entry_game_name", 0x0037_87B0, ptr(cstr()), DESKTOP, "`STR_ENTRY_GAME_NAME`"),
            e(
                "info",
                0x0037_87B4,
                array(ptr(lines(2)), 5),
                DESKTOP,
                "`STR_INFO_MSG_0..4` (`InfoMsg`): the pages shown, two lines each.",
            ),
            e(
                "name_info",
                0x0042_C660,
                array(ptr(lines(3)), 3),
                DESKTOP,
                "`NameInfoBlock`: per name, lines 1 and 2 are its labels (`#G` on the one being entered).",
            ),
            e("menu", 0x0042_C6B0, array(ptr(cstr()), 4), DESKTOP, "`MenuBlockE`: the menu row per `CurAct`."),
            e("forbidden", 0x0042_C6C0, array(ptr(cstr()), 21), DESKTOP, "`@2319`: the character names refused."),
            e(
                "hira_block",
                0x0042_C760,
                array_stride(cstr(), 1, 14),
                DESKTOP,
                "`HiraBlock` (`char[][14]`): the kana grid, one empty row in the US builds.",
            ),
            e(
                "eigo_block",
                0x0042_C7E0,
                array_stride(cstr(), 18, 14),
                DESKTOP,
                "`EigoBlock` (`char[][14]`): the English grid, 18 rows of five two-byte cells.",
            ),
            e("default_game_name", 0x0046_10C8, cstr(), DESKTOP, "`@1389`: the default character name."),
            e("face_dashes", 0x0037_2C70, cstr(), MAIN, "`@2275`: the face panel's HP and SP before a name."),
            e(
                "face_tex",
                0x0037_2C60,
                cstr(),
                MAIN,
                "`@2095`: the face panel's picture in `xwin_f00` (`ccFacePanel`): Kite before the bracelet on Infection, with it after.",
            ),
        ],
    )
}

// The top page (toppage.prg): the board's threads and posts, the words it
// draws, the idle animation by volume.
fn toppage() -> Group {
    group(
        "toppage",
        "TopPage",
        "The top page's board (toppage.prg): its threads and posts, normal and parody, and its words.",
        vec![
            e(
                "threads",
                0x0040_6450,
                array(bbs_thread(), 63),
                TOPPAGE,
                "`bbsThreadTbl`: the 63 threads, each its posts (`message` is the post's lines).",
            ),
            e("threads_parody", 0x0040_8200, array(bbs_thread(), 63), TOPPAGE, "`bbsThreadTblP`: Parody Mode's."),
            e(
                "neutral",
                0x0040_4990,
                array(ptr(cstr()), 4),
                TOPPAGE,
                "`@1088`: the idle animation by volume; `_ChangeMode` reads entry `volumeNum - 1`.",
            ),
            e("new_msg", 0x0040_8618, cstr(), TOPPAGE, "The writing page's label."),
            e("no_msg", 0x0040_8630, cstr(), TOPPAGE, "The empty thread's line."),
            e("author", 0x0040_8650, cstr(), TOPPAGE, "A post's author label."),
            e("time", 0x0040_8660, cstr(), TOPPAGE, "A post's time label."),
            e("player", 0x0040_8670, cstr(), TOPPAGE, "The player's label."),
        ],
    )
}

// ccGame (main).
fn game() -> Group {
    group("game", "Game", "`ccGame`'s own tables (main).", vec![
        // Mutation on dropped the table: their ChangeScene sets the server
        // to the town number itself.
        e("town_server", 0x0030_6DC0, array(I32, 8), MAIN, "`ccGame::ChangeScene`'s server of each town: Infection's `@1489`; the later volumes' ChangeScene sets the server to the town itself.").absent(Vol::Mut, identity(8)).absent(Vol::Out, identity(8)).absent(Vol::Qua, identity(8)),
    ])
}

// The field UI (ccMenuCtrl, gcmn): its lists, help texts, windows' words and
// the item lists the boxes draw from.
fn fieldui() -> Group {
    group(
        "fieldui",
        "FieldUi",
        "The field's menus (`ccMenuCtrl`, gcmn with main's texts): lists, help, the windows' words, the item lists.",
        vec![
            e(
                "elements",
                0x0065_1000,
                array_by(field_menu_element(), later(89, 93)),
                GCMN,
                "`menuElementData`: the lists `InitMenuList` fills `menuList` from (89; 93 from Mutation on, its loop's bound).",
            ),
            e("personal_help", 0x0033_E810, array(text_lines(3), 10), GCMN, "`personalMenuHelp`"),
            e("option_help", 0x0033_E840, array(text_lines(3), 8), GCMN, "`optionMenuHelp`"),
            e("help", 0x0033_ED50, array(opt(cstr()), 4), GCMN, "`helpStr`: \": Talk\", \": Attack\", ..."),
            // From Mutation on, the Flag Race's texts follow `helpStr`.
            derived("race_str", race_str(), GCMN, "The Flag Race's texts (MUT main 0x00353d40).")
                .after("help", 16)
                .absent(Vol::Inf, Value::List(vec![Value::List(Vec::new()); 12])),
            e("dead_info", 0x0037_7E6C, ptr(cstr()), GCMN, "`deadInfo`"),
            e("new_mail", 0x0037_7E8C, ptr(cstr()), GCMN, "`newMailStr`"),
            e("kyvia_status", 0x0037_7E90, text_lines(8), GCMN, "`kyviaStatusStr`: eight pieces."),
            e("cheat_hp", 0x0037_7E88, ptr(cstr()), GCMN, "`cheatHpStr`: the digits for 5-digit HP."),
            e("panel_flash", 0x006E_01D0, array(U32, 5), GCMN, "`panelFlashTbl`: the panel flash's grey steps."),
            e(
                "faces",
                0x0065_17F0,
                array(
                    strukt(
                        "MenuFace",
                        8,
                        vec![("file", 0, opt(cstr())), ("texture", 4, opt(cstr()))],
                        "A face: its scene file and texture.",
                    ),
                    19,
                ),
                GCMN,
                "`menuFaceCcsList`: per `charTbl` row; row 18 is Kite before the bracelet.",
            ),
            e("skill_tags", 0x0037_7DC4, text_lines(6), GCMN, "`skillMenuTag`: the pages' tabs."),
            e("item_tags", 0x0037_7DC8, text_lines(6), GCMN, "`itemMenuTag`"),
            e("skill_help", 0x0033_E8D0, array(text_lines(2), 3), GCMN, "`skillMenuHelp`"),
            e("item_help", 0x0033_E8E0, array(text_lines(2), 10), GCMN, "`itemMenuHelp`"),
            e("target_info", 0x0037_7DD0, text_lines(2), GCMN, "`targetMenuInfo`"),
            e("target_warn", 0x0037_7DD4, ptr(cstr()), GCMN, "`targetMenuWarn`"),
            e("target_drain_warn", 0x0037_7DD8, text_lines(2), GCMN, "`targetMenuDrainWarn`"),
            e(
                "virus_col",
                0x0065_1570,
                array(fixed(I32, 12), 6),
                GCMN,
                "`virusCol`: the bracelet gauge's two colour pairs by `erosion / 17`.",
            ),
            e("chat_tags", 0x0037_7DBC, text_lines(3), GCMN, "`chatMenuTag`"),
            e("chat_str", 0x0037_7DB8, text_lines(19), GCMN, "`chatMenuStr`: the orders."),
            e("chat_action", 0x0037_7DC0, text_lines(11), GCMN, "`chatActionStr`'s pieces."),
            e("chat_help", 0x0033_E870, array(text_lines(3), 19), GCMN, "`chatMenuHelp`"),
            e("chat_warn", 0x0033_E8C0, array(opt(lines(2)), 6), GCMN, "`chatMenuWarn`: the orders' refusals."),
            e("party_warn", 0x0033_EB60, array(text_lines(3), 5), GCMN, "`partyMenuWarn`"),
            e("bt_chat_act", 0x0065_16D0, array(I32, 12), GCMN, "`btInChatAct`: the orders' command numbers."),
            e("town_chat_act", 0x0037_8258, array(I32, 2), GCMN, "`townChatAct`"),
            e("gate_help", 0x0033_ECD0, array(text_lines(3), 5), GCMN, "`gateMenuHelp`"),
            e("gt_new_info", 0x0033_ECE8, array(text_lines(2), 3), GCMN, "`gtNewInfo`"),
            e("gt_new_str1", 0x0033_ED00, array(opt(cstr()), 12), GCMN, "`gtNewStr1`: the attributes and parts."),
            e("gt_town_help", 0x0033_ED30, array(text_lines(3), 5), GCMN, "`gtTownMenuHelp`"),
            e(
                "gate_word_list_msg",
                0x0033_ED60,
                array(opt(lines(3)), 133),
                GCMN,
                "`gateWordListMsg`: each story area's note in the Word List.",
            ),
            e("gt_menu_info", 0x0037_7E50, ptr(cstr()), GCMN, "`gtMenuInfo`: \"Warp to\"."),
            e("gt_new_str0", 0x0037_7E54, text_lines(12), GCMN, "`gtNewStr0`'s pieces."),
            e("gt_town_str", 0x0037_7E58, text_lines(5), GCMN, "`gtTownMenuStr`: the servers."),
            e("server_str", 0x0037_7E5C, text_lines(5), GCMN, "`serverStr`: the servers' symbols."),
            e("gt_town_warn", 0x0037_7E64, ptr(cstr()), GCMN, "`gtTownMenuWarn`"),
            e("gt_list_warn", 0x0037_7E68, ptr(cstr()), GCMN, "`gtListMenuWarn`"),
            e("gt_area_info", 0x006E_0140, cstr(), GCMN, "The Area Information row."),
            e("gt_new_colours", 0x0065_1718, array(I32, 3), GCMN, "`ccGetGtNewColor`'s colours."),
            derived(
                "gate_refusal",
                custom(
                    Rc::new(gate_refusal),
                    opt(strukt(
                        "GateRefusal",
                        0,
                        vec![("status", 0, I32), ("areas", 4, array(I32, 0)), ("msg", 8, ev_msg())],
                        "The gate's refusal: while `saveData.eventStatus[status]` is set, a warp to one of these story areas (`GetEventAreaNumber`) shows `msg` and closes the gate instead.",
                    )),
                ),
                GCMN,
                "From Mutation on, the story areas the gate refuses while the bracelet is off, and Kite's line (None on Infection).",
            ),
            e("status_menu", 0x0033_E9E0, array(ptr(cstr()), 6), GCMN, "`statusMenuStr`: the Status page's labels."),
            e("status_class", 0x0037_7DE0, text_lines(6), GCMN, "`statusClassStr`: the classes."),
            e("party_in_info", 0x0037_7DF0, array(text_lines(2), 2), GCMN, "`partyInMenuInfo`"),
            e("get_item_str", 0x0037_7E0C, text_lines(13), GCMN, "`getItemMenuStr`'s lines."),
            e("trap_menu", 0x0037_7E10, text_lines(5), GCMN, "`getTrapMenuStr`: TrapBoxMenu's lines."),
            e("trap_discharge", 0x0037_7E78, ptr(cstr()), GCMN, "`trapDischargeStr`"),
            e("show_map", 0x0037_7E70, ptr(cstr()), GCMN, "`showMapInfo`"),
            e(
                "status_up",
                0x0037_7E74,
                text_lines(18),
                GCMN,
                "`statusUpStr`: \"increased by \", \"decreased by \", then the stats' names.",
            ),
            e("install_warn", 0x0037_7E80, text_lines(3), GCMN, "`installWarnStr`"),
            e("epitaph_unknown", 0x0037_7E94, ptr(cstr()), GCMN, "`epitaphStr0X`'s first piece."),
            // The epitaphs and notes `ccEpitaphMsg(strs, pages)` shows (the
            // important items' use, piney-battle's `item::epitaph`): each a
            // `char *[pages]` of three NUL-ended lines (`ccKanjiStrSeparate`
            // 0..2), the parody mode's after it.
            e("epitaph_00", 0x0037_8270, array(ptr(lines(3)), 2), GCMN, "`epitaphStr00`: item 42's pages."),
            e("epitaph_00p", 0x0037_8278, array(ptr(lines(3)), 2), GCMN, "`epitaphStr00p`"),
            e("epitaph_01", 0x0037_8280, array(ptr(lines(3)), 1), GCMN, "`epitaphStr01`: item 43's."),
            e("epitaph_01p", 0x0037_8284, array(ptr(lines(3)), 1), GCMN, "`epitaphStr01p`"),
            e("epitaph_02", 0x0065_1A30, array(ptr(lines(3)), 3), GCMN, "`epitaphStr02`: item 44's."),
            e("epitaph_02p", 0x0065_1A40, array(ptr(lines(3)), 3), GCMN, "`epitaphStr02p`"),
            e("epitaph_03", 0x0037_8288, array(ptr(lines(3)), 2), GCMN, "`epitaphStr03`: item 45's."),
            e("epitaph_03p", 0x0037_8290, array(ptr(lines(3)), 2), GCMN, "`epitaphStr03p`"),
            e("epitaph_04", 0x0065_1A50, array(ptr(lines(3)), 3), GCMN, "`epitaphStr04`: item 46's."),
            e("epitaph_04p", 0x0065_1A60, array(ptr(lines(3)), 3), GCMN, "`epitaphStr04p`"),
            e("epitaph_10", 0x0065_1A70, array(ptr(lines(3)), 3), GCMN, "`epitaphStr10`: item 48's."),
            e("epitaph_10p", 0x0065_1A80, array(ptr(lines(3)), 3), GCMN, "`epitaphStr10p`"),
            e("epitaph_11", 0x0065_1A90, array(ptr(lines(3)), 4), GCMN, "`epitaphStr11`: item 68's."),
            e("epitaph_11p", 0x0065_1AA0, array(ptr(lines(3)), 4), GCMN, "`epitaphStr11p`"),
            e("epitaph_m0", 0x0065_1AB0, array(ptr(lines(3)), 3), GCMN, "`epitaphStrM0`: item 287's, both modes."),
            derived(
                "epitaph_m1",
                custom(item_pages(288, array(ptr(lines(3)), 3)), array(ptr(lines(3)), 3)),
                GCMN,
                "`epitaphStrM1`: item 288's.",
            ),
            derived(
                "epitaph_m2",
                custom(item_pages(289, array(ptr(lines(3)), 3)), array(ptr(lines(3)), 3)),
                GCMN,
                "`epitaphStrM2`: item 289's.",
            ),
            e("epitaph_m3", 0x0037_8298, array(ptr(lines(3)), 2), GCMN, "`epitaphStrM3`: item 290's."),
            e(
                "item_box_list",
                0x0064_C450,
                array(ptr(array(I32, 130)), 30),
                GCMN,
                "`ItemBoxList`: 130 item codes by server * 6 + element.",
            ),
            e("danger_item_box_list", 0x006A_1ED0, array(ptr(array(I32, 130)), 30), GCMN, "`DangerItemBoxList`"),
            e("area_item_list", 0x0069_E070, array(ptr(array(I32, 130)), 30), GCMN, "`areaItemList`"),
            e("idol_item_list", 0x0065_02B0, array(ptr(array(I32, 130)), 30), GCMN, "`idolItemList`"),
            e("suka_item_box_list", 0x006A_29A0, array(ptr(array(I32, 130)), 5), GCMN, "`SukaItemBoxList` by server."),
            e("idol_sub_item_list", 0x0065_0D80, array(ptr(array(I32, 130)), 5), GCMN, "`idolSubItemList` by server."),
            e(
                "record_str",
                0x0037_7E48,
                ptr(slots()),
                GCMN,
                "`recordMenuStr`: \"slot 1\", \"slot 2\", \"Unused\", \"Data\", \"MEMORY CARD\".",
            ),
            // PERSONAL's pages.
            e("imp_tags", 0x0037_7DCC, text_lines(4), GCMN, "`impItemMenuTag`: Key Items' four tabs."),
            e("grunty_warn", 0x0037_7E7C, text_lines(2), GCMN, "`puccigusoCallWarnStr`"),
            e("shop_str", 0x0037_7E3C, ptr(cstr()), GCMN, "`shopStr`'s first line: the count window's labels."),
            e("in_battle_warn", 0x0037_7E08, ptr(cstr()), GCMN, "`inBattleMenuWarn`"),
            e("gateout_info", 0x0037_7E00, ptr(cstr()), GCMN, "`gateoutMenuInfo`"),
            e("logout_info", 0x0037_7E04, ptr(cstr()), GCMN, "`logoutMenuInfo`"),
            e(
                "status_help",
                0x0033_EA00,
                array(text_lines(3), 75),
                GCMN,
                "`statusMenuHelp`: three lines for each cell of the Status grid, 15 rows a column.",
            ),
            e(
                "status_beff",
                0x0033_E9C0,
                array(text_lines(4), 5),
                GCMN,
                "`statusBeffStr`: each added effect's help (lines 0-2) and name (line 3).",
            ),
            e(
                "equip_change",
                0x0033_EB30,
                array(ptr(cstr()), 5),
                GCMN,
                "`equipChangeStr`: each page's four labels, 16 characters each.",
            ),
            e("equip_warn", 0x0037_7DE8, array(text_lines(3), 2), GCMN, "`equipMenuWarn`: the refusals."),
            e(
                "category_order",
                0x0030_7140,
                array(fixed(I16, 2), 15),
                GCMN,
                "`addItemCategoryTbl`: the order `AddItem` sorts a list in, (category, rows) pairs.",
            ),
            // The party's.
            e(
                "spc_msg_tbl",
                0x0063_7E30,
                array_by(U32, later(18, 21)),
                GCMN,
                "`spcMsgTbl`: where each member's line tables are, the pointers `SetSpcBaseMsg` stores in the save.",
            ),
            e("party_help", 0x0033_EB48, array(text_lines(3), 3), GCMN, "`partyMenuHelp`"),
            e("party_out_info", 0x0037_7DF8, array(text_lines(2), 2), GCMN, "`partyOutMenuInfo`"),
            e(
                "spc_msg0",
                0x0063_7F90,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "A party message table, a record a member.",
            ),
            e(
                "spc_msg1",
                0x0063_7FF0,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "A party message table, a record a member.",
            ),
            e(
                "spc_msg2",
                0x0063_8150,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "A party message table, a record a member.",
            ),
            e(
                "spc_msg3",
                0x0063_81B0,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "A party message table, a record a member.",
            ),
            e(
                "spc_msg4",
                0x0063_8310,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "A party message table, a record a member.",
            ),
            e(
                "spc_msg5",
                0x0063_8370,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "A party message table, a record a member.",
            ),
            // OPTION's.
            e("drain_demo_info", 0x0037_7D80, array(text_lines(2), 2), GCMN, "`datadrainDemoMenuInfo`"),
            e("control_file", 0x006E_0658, cstr(), GCMN, "The Controller picture's file."),
            e("control_tex", 0x006E_0668, cstr(), GCMN, "The Controller picture's texture."),
            // The field objects' (time idols) and the fountain's.
            e("time_idol_item", 0x0065_1700, array(I32, 5), GCMN, "`timeIdolItem`: the items for the places gained."),
            e("get_skill", 0x0037_7E14, ptr(cstr()), GCMN, "`getSkillMenuStr`'s first piece."),
            e("time_idol_str", 0x0037_7E18, text_lines(10), GCMN, "`timeIdolMenuStr`"),
            e("time_idol_help", 0x0033_EB80, array(text_lines(2), 4), GCMN, "`timeIdolMenuHelp`"),
            e("fountain_help", 0x0033_EBC0, array(text_lines(3), 29), GCMN, "`fountainMenuHelp`"),
            e("fountain_answers", 0x0037_7E1C, ptr(cstr()), GCMN, "`fountainMenuStr`"),
            e("fountain_names", 0x0037_7E20, text_lines(10), GCMN, "`fountainNameStr`"),
            e(
                "fe_tbl",
                0x0069_A260,
                array(ptr(array(I16, 80)), 10),
                GCMN,
                "The fountains' elements: 80 each, by fountain.",
            ),
            e(
                "f_limit",
                0x0030_7010,
                array(I16, 20),
                GCMN,
                "`f_limitTbl`: how many count, by server (+ 10 from level 4, + 5 for category 2).",
            ),
            // The talk pages' (trading, shops, the breeder).
            e("tpc_talk", 0x0037_7E84, text_lines(2), GCMN, "`tpcTalkStr`: the trading PCs' offer's two pieces."),
            e("trade_help", 0x0033_EC40, array(text_lines(3), 7), GCMN, "`tradeMenuHelp`"),
            e("present_help", 0x0033_EC60, array(text_lines(3), 6), GCMN, "`presentMenuHelp`"),
            e("trade_menu_str", 0x0037_7E38, ptr(cstr()), GCMN, "`tradeMenuStr`"),
            e("buy_help", 0x0033_EC80, array(text_lines(3), 5), GCMN, "`buyMenuHelp`"),
            e(
                "sell_help",
                0x0037_7E40,
                array(text_lines(3), 2),
                GCMN,
                "`sellMenuHelp`: the count's help, the question.",
            ),
            e("deposit_help", 0x0033_ECA0, array(text_lines(3), 4), GCMN, "`itemDepositMenuHelp`"),
            e("draw_help", 0x0033_ECB0, array(text_lines(3), 4), GCMN, "`itemDrawMenuHelp`"),
            e(
                "item_shop",
                0x0064_8230,
                array(ptr(array(I32, 24)), 5),
                GCMN,
                "`ItemShopItemList`: the stock by server.",
            ),
            e("equip_shop", 0x0064_8430, array(ptr(array(I32, 24)), 5), GCMN, "`EquipShopItemList`"),
            e("magic_shop", 0x0064_8650, array(ptr(array(I32, 24)), 5), GCMN, "`MagicShopItemList`"),
            e("pl_item_pages", 0x0065_16B0, array(I32, 5), GCMN, "`SetPlItemList`'s page table."),
            e(
                "breeder_str",
                0x0037_7E24,
                custom(
                    Rc::new(|c| {
                        text_lines(if c.volume == Vol::Inf { 3 } else { 5 }).read(c, find(c, 0x0037_7E24, GCMN))
                    }),
                    text_lines(5),
                ),
                GCMN,
                "`breederMenuStr`: the breeder's rows (from Mutation on also Flag Race and Rankings).",
            ),
            // From Mutation on, the Flag Race's tables (MUT gcmn 0x005ff9f0,
            // 0x005ffa20: by `game.server - 1`); none on Infection.
            derived(
                "race_ranks",
                custom(Rc::new(|c| race_ranks().read(c, race_tables(c)?[0])), race_ranks()),
                GCMN,
                "The Flag Race's rankings before the player's, three a town (MUT gcmn 0x006d8610).",
            )
            .absent(Vol::Inf, Value::List(Vec::new())),
            derived(
                "race_grunties",
                custom(Rc::new(|c| race_grunties().read(c, race_tables(c)?[1])), race_grunties()),
                GCMN,
                "The three Grunties a town's Flag Race offers, with their stars (MUT gcmn 0x006d8670).",
            )
            .absent(Vol::Inf, Value::List(Vec::new())),
            e("breeding_str", 0x0037_7E28, ptr(cstr()), GCMN, "`breedingMenuStr`: the status rows."),
            e(
                "breeding_help",
                0x0037_7E30,
                array(text_lines(3), 2),
                GCMN,
                "`breedingMenuHelp`: \"There is no food.\", then the help.",
            ),
            e(
                "food",
                0x005E_E450,
                array(ty("FOOD_PARAM", vec![]), 16),
                GCMN,
                "`foodTbl`: what each food (key items 26-41) does to a Grunty's six stats.",
            ),
            e(
                "spc_trade_rate",
                0x0034_57F0,
                array_by(fixed(I16, 14), later(17, 20)),
                GCMN,
                "`spcTradeRateTbl`: how much members 1-17 (1-20 from Mutation on) value a kind of item, in tenths.",
            ),
            e("npc_trade_rate", 0x0034_59D0, array_by(fixed(I16, 14), later(48, 54)), GCMN, "`npcTradeRateTbl`"),
            // From Mutation on, Gift weighs the gift against what the member
            // has before the thanks (MUT gcmn 0x00573660); none on Infection.
            derived(
                "gift_wants",
                custom(Rc::new(|c| wants().read(c, gift_tables(c)?[0])), wants()),
                GCMN,
                "Members 1-20's wanted gifts, 16 each: one worth 50000 unless the member has it (MUT main 0x0035f850).",
            )
            .absent(Vol::Inf, Value::List(Vec::new())),
            derived(
                "gift_bands",
                custom(Rc::new(|c| bands().read(c, gift_tables(c)?[1])), bands()),
                GCMN,
                "The worth bands of the thanks: equipment's (over the member's best of its kind), then the rest's.",
            )
            .absent(Vol::Inf, Value::List(Vec::new())),
            derived(
                "gift_rates",
                custom(Rc::new(|c| rates().read(c, gift_tables(c)?[2])), rates()),
                GCMN,
                "Members 1-20's rates for a gift, as `spcTradeRateTbl`'s (MUT main 0x0035f620; Trade keeps that one).",
            )
            .absent(Vol::Inf, Value::List(Vec::new())),
            e("friendship_cap", 0x0030_7180, array(I32, 5), GCMN, "`@3218`: `AddFriendship`'s cap by volume."),
            // Where the talk pages start from (`talk`): read as the
            // records they point at, each with its address.
            e(
                "spc_msg_present10",
                0x0063_8800,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "`spcMsgPresent10`: each member's first thanks record for a gift.",
            ),
            derived("spc_msg_present10_va", addr(), GCMN, "Where it is.").after("spc_msg_present10", 0),
            e(
                "spc_msg_present11",
                0x0063_8890,
                array_by(opt(ev_msg()), later(18, 21)),
                GCMN,
                "`spcMsgPresent11`: the same once the member's story has begun.",
            ),
            derived("spc_msg_present11_va", addr(), GCMN, "Where it is.").after("spc_msg_present11", 0),
            e(
                "breed_teach",
                0x0063_1C90,
                array(opt(ev_msg()), 3),
                GCMN,
                "`breedTeachMsgTbl`: the About pages' records.",
            ),
            derived("breed_teach_va", addr(), GCMN, "Where it is.").after("breed_teach", 0),
            e("breed_teach2", 0x0063_1CA0, array(opt(ev_msg()), 3), GCMN, "`breedTeachMsgTbl2`: elsewhere."),
            derived("breed_teach2_va", addr(), GCMN, "Where it is.").after("breed_teach2", 0),
            e("pg_evo_msg", 0x0063_8DC0, array(ev_msg(), 2), GCMN, "`pgEvoMsg`: a Grunty growing up."),
            derived("pg_evo_msg_va", addr(), GCMN, "Where it is.").after("pg_evo_msg", 0),
            e("error_data", 0x0035_5888, ev_msg(), GCMN, "`errorData`: the record shown for a missing line."),
            derived("error_data_va", addr(), GCMN, "Where `errorData` is, which a chain of records holds.")
                .after("error_data", 0),
            e(
                "kite_self_talk",
                0x0063_9548,
                ev_msg(),
                GCMN,
                "`kiteSelfTalk`: Kite's line as area 43's map sends the party back (`EVENTAREA03::Draw`).",
            ),
            derived("kite_self_talk_va", addr(), GCMN, "Where `kiteSelfTalk` is.").after("kite_self_talk", 0),
            derived(
                "voice_groups",
                custom(Rc::new(voice_groups), array(I32, 17)),
                MAIN,
                "`ccCheckVoiceGrp(id)`: ids 141-157's voice groups (its jump table's `li $v0, n`), -1 none.",
            ),
            e("hack_info", 0x0037_7E4C, text_lines(2), GCMN, "`gtHackInfo`"),
            e("hack_bar", 0x0037_7E60, ptr(cstr()), GCMN, "`gtHackStr`"),
            e("drain_warn", 0x0037_7DDC, text_lines(34), GCMN, "`dataDrainWarn`: 34 pieces."),
            e("drain_evolution", 0x0033_E910, array(text_lines(2), 44), GCMN, "`dataDrainEvolutionStr`"),
        ],
    )
}

fn battle() -> Group {
    group(
        "battle",
        "Battle",
        "The battle's and menus' parameters (gcmn): skills, items, equipment, enemies, bosses, the party's AI.",
        vec![
            e("skills", 0x0061_F4A0, array(skill(), 304), GCMN, "`skillTbl`: `ccGetSkillParam(i)`."),
            e(
                "boss_skills",
                0x0069_6CE0,
                array_by(skill(), later(61, 79)),
                GCMN,
                "`BossSkillTbl`: 79 from Mutation on.",
            ),
            e("enemies", 0x005F_1E70, array(ty("ccEnemyTable", enemy_over()), 303), GCMN, "`enemyTbl`"),
            e(
                "gimmicks",
                0x0061_E0F0,
                array_by(ty("ccGimmickTable", entry_over()), later(45, 46)),
                GCMN,
                "`gimmickTbl`: 46 from Mutation on.",
            ),
            e(
                "npcs",
                0x0061_9460,
                array_by(ty("ccNpcTable", entry_over()), later(175, 184)),
                GCMN,
                "`npcTbl`: the towns' people (`param.base.msg` is where their lines are); 184 from Mutation on.",
            ),
            e(
                "enemy_lists",
                0x005D_A200,
                array(fixed(ty("ccEnemyList", vec![("list", counted_ptr(I32, 4)), ("num", omit())]), 7), 5),
                GCMN,
                "`ccEnemyListInfo`: by server and type, the `enemyTbl` rows an area spawns.",
            ),
            e("bosses", 0x0061_30B0, array(ty("ccBossParamData", vec![]), 49), GCMN, "`bossTbl`"),
            e("item_r", 0x0062_3720, array(item(), 24), GCMN, "`itemTblR`: category 10."),
            e("item_d", 0x0062_3900, array(item(), 72), GCMN, "`itemTblD`: category 11."),
            e("item_u", 0x0062_3EA0, array(item(), 34), GCMN, "`itemTblU`: category 12."),
            e("item_x", 0x0062_4150, array(item(), 3), GCMN, "`itemTblX`: category 13."),
            e("item_t", 0x0062_4190, array(item(), 22), GCMN, "`itemTblT`: category 14."),
            e("item_e", 0x0062_4350, array(item(), 291), GCMN, "`itemTblE`: category 15."),
            e("head", 0x0063_A8D0, array(equip(), 69), GCMN, "`equipmentHeadTbl`: category 6."),
            e("body", 0x0063_BC40, array(equip(), 68), GCMN, "`equipmentBodyTbl`: category 7."),
            e("arm", 0x0063_CF60, array(equip(), 67), GCMN, "`equipmentArmTbl`: category 8."),
            e("leg", 0x0063_E240, array(equip(), 68), GCMN, "`equipmentLegTbl`: category 9."),
            e(
                "weapon1",
                0x0063_F560,
                array_by(equip(), later(82, 83)),
                GCMN,
                "`equipmentWeapon1Tbl`: job 0's, category 0 (a row more from Mutation on).",
            ),
            e("weapon2", 0x0064_0C70, array(equip(), 77), GCMN, "`equipmentWeapon2Tbl`"),
            e("weapon3", 0x0064_2220, array(equip(), 97), GCMN, "`equipmentWeapon3Tbl`"),
            e(
                "weapon4",
                0x0064_3D70,
                array_by(equip(), later(75, 76)),
                GCMN,
                "`equipmentWeapon4Tbl` (a row more from Mutation on)",
            ),
            e("weapon5", 0x0064_5290, array(equip(), 74), GCMN, "`equipmentWeapon5Tbl`"),
            e(
                "weapon6",
                0x0064_6760,
                array_by(equip(), later(76, 77)),
                GCMN,
                "`equipmentWeapon6Tbl` (a row more from Mutation on)",
            ),
            // An empty equipment slot (id -1) reads the row before its table:
            // the numbers the equipment page reads there.
            derived("head_before", equip_before(), GCMN, "The 0x48 bytes before `head`, as a row.")
                .after("head", -0x48),
            derived("body_before", equip_before(), GCMN, "The 0x48 bytes before `body`, as a row.")
                .after("body", -0x48),
            derived("arm_before", equip_before(), GCMN, "The 0x48 bytes before `arm`, as a row.").after("arm", -0x48),
            derived("leg_before", equip_before(), GCMN, "The 0x48 bytes before `leg`, as a row.").after("leg", -0x48),
            derived("weapon1_before", equip_before(), GCMN, "The 0x48 bytes before `weapon1`, as a row.")
                .after("weapon1", -0x48),
            derived("weapon2_before", equip_before(), GCMN, "The 0x48 bytes before `weapon2`, as a row.")
                .after("weapon2", -0x48),
            derived("weapon3_before", equip_before(), GCMN, "The 0x48 bytes before `weapon3`, as a row.")
                .after("weapon3", -0x48),
            derived("weapon4_before", equip_before(), GCMN, "The 0x48 bytes before `weapon4`, as a row.")
                .after("weapon4", -0x48),
            derived("weapon5_before", equip_before(), GCMN, "The 0x48 bytes before `weapon5`, as a row.")
                .after("weapon5", -0x48),
            derived("weapon6_before", equip_before(), GCMN, "The 0x48 bytes before `weapon6`, as a row.")
                .after("weapon6", -0x48),
            e(
                "level_up",
                0x005D_1800,
                array_by(ty("ccLevelUpTbl", vec![]), later(18, 21)),
                GCMN,
                "`LevelUpParamTbl`, by character (21 from Mutation on).",
            ),
            e("exp_calc", 0x0065_18F0, array(I16, 21), GCMN, "`expCalcTbl`, by level difference + 10."),
            e(
                "races",
                0x005F_1D60,
                array(ty("ccEnemyRace", vec![("func", omit())]), 22),
                GCMN,
                "`ccEntryRaceTbl`: `enemyTbl` in groups of `raceNum` rows.",
            ),
            e(
                "drain_erosion",
                0x0065_1430,
                array(fixed(I32, 16), 5),
                GCMN,
                "`dataDrainErosionTbl`: a Data Drain's side effect by infection / 25.",
            ),
            e("erosion", 0x0030_71A0, array(I32, 16), GCMN, "`ccSaveData::AddLvErosion`'s `erosionTbl`."),
            e(
                "spc_default_items",
                0x0064_7D80,
                array_by(fixed(item_list(), 10), later(18, 21)),
                GCMN,
                "`spcDefaultItemList` (21 from Mutation on)",
            ),
            // charitem.cpp's three lists sit together on every volume; the later
            // volumes' InitSpcParam changed round the first two.
            derived("player_default_items", array(item_list(), 24), GCMN, "`playerDefaultItemList`")
                .after("spc_default_items", -0xC0),
            derived("player_default_important", array(item_list(), 24), GCMN, "`playerDefaultImportantItemList`")
                .after("spc_default_items", -0x60),
            e("ai_params", 0x0065_3CA0, array(ty("ccAIParam", vec![]), 18), GCMN, "`spcAIParam`"),
            e("debuff_priority", 0x0065_3F30, array(fixed(I16, 9), 3), GCMN, "`spcDebuffPriorityType`"),
            e("buff_priority", 0x0065_3F70, array(fixed(I16, 5), 3), GCMN, "`spcBuffPriorityType`"),
            e("debuff_table_index", 0x0065_3F90, array_by(I16, later(18, 21)), GCMN, "`debuffTableIndex`"),
            e("buff_table_index", 0x0065_3FC0, array_by(I16, later(18, 21)), GCMN, "`buffTableIndex`"),
            e("debuff_skill_ability", 0x0065_3FE8, array(I16, 6), GCMN, "The debuffs' ability skills."),
            e("debuff_skill_attribute", 0x0065_3FF8, array(I16, 6), GCMN, "The debuffs' attribute skills."),
            e("buff_skill_ability", 0x0065_4008, array(I16, 6), GCMN, "The buffs' ability skills."),
            e("buff_skill_attribute", 0x0065_4018, array(I16, 6), GCMN, "The buffs' attribute skills."),
            derived(
                "item_icons",
                custom(Rc::new(item_icons), array(I32, 16)),
                MAIN,
                "`ccGetItemIcon(cat, id)` by category: each case of its jump table is `li $v0, icon` (or 0).",
            ),
            e(
                "item_pages",
                0x0065_1690,
                array(I32, 5),
                GCMN,
                "`SetItemList`'s page to category: -1 the usable items, -2 the equipment, else the category.",
            ),
        ],
    )
}

// The party members' chat lines (`ccAI::ChatMessage*`, gcmn): each table one
// line a character (19), three (57) or three in all; None for no line.
/// Skeith's act tables (`ptr[3]`: normal, super, Epitaph), each its words
/// to the -1 and the one past it (a wrap reads the first again).
fn skeith_acts(c: &Ctx) -> Read {
    let t = find(c, 0x005E_B598, None);
    let mut out = Vec::new();
    for k in 0..3 {
        let va = c.p.u32(t + 4 * k)?;
        let mut words = Vec::new();
        for i in 0..256 {
            let w = c.p.u32(va + 4 * i)? as i32;
            words.push(Value::Int(i128::from(w)));
            if w == -1 {
                words.push(Value::Int(i128::from(c.p.u32(va + 4 * (i + 1))? as i32)));
                break;
            }
        }
        out.push(Value::List(words));
    }
    Ok(Value::List(out))
}

/// `AllGomoraList_1[0]` (a `short **` in main's small data): the lists
/// `kyviaGomora::GomoraInit` (INF gcmn 0x004e3570) gives Kyvia 01's gomoras,
/// found by the function's first `lw rX, off($gp)`; each list a gomora's
/// attribute by slave, the pointers to a null.
fn kyvia_gomora_lists(c: &Ctx) -> Read {
    let f = find(c, 0x004E_3570, GCMN);
    let gp = c.p.gp.ok_or("no gp")?;
    let mut global = None;
    for k in 0..64 {
        let w = c.p.u32(f + 4 * k)?;
        if w >> 26 == 0x23 && (w >> 21) & 31 == 28 {
            global = Some(gp.wrapping_add(sext16(w) as u32));
            break;
        }
    }
    let lists = c.p.u32(global.ok_or_else(|| format!("0x{f:08x} reads no small global"))?)?;
    let mut out = Vec::new();
    for k in 0..16 {
        let l = c.p.u32(lists + 4 * k)?;
        if l == 0 {
            break;
        }
        let row = (0..5)
            .map(|j| c.p.read(l + 2 * j, 2).map(|b| Value::Int(i128::from(i16::from_le_bytes([b[0], b[1]])))))
            .collect::<Result<Vec<_>, _>>()?;
        out.push(Value::List(row));
    }
    Ok(Value::List(out))
}

/// `AllGomoraList_2` (INF gcmn 0x005eced0, `short **[3]`): the lists
/// `kyviaGomora::GomoraInit(2)` and `ListStepUp(n)` give level 2's gomoras,
/// by the core's deaths, each to its null; each list's rows to theirs.
fn kyvia_gomora_lists_2(c: &Ctx) -> Read {
    let at = find(c, 0x005E_CED0, GCMN);
    let mut out = Vec::new();
    for k in 0..3 {
        let lists = c.p.u32(at + 4 * k)?;
        if lists == 0 {
            break;
        }
        let mut step = Vec::new();
        for j in 0..16 {
            let l = c.p.u32(lists + 4 * j)?;
            if l == 0 {
                break;
            }
            let row = (0..5)
                .map(|i| c.p.read(l + 2 * i, 2).map(|b| Value::Int(i128::from(i16::from_le_bytes([b[0], b[1]])))))
                .collect::<Result<Vec<_>, _>>()?;
            step.push(Value::List(row));
        }
        out.push(Value::List(step));
    }
    Ok(Value::List(out))
}

/// Every GCMN.PRG global of Infection's named `*{suffix}` (the enemies'
/// weapon and dust blocks), by name: each where the volume keeps it, and
/// its rows.
fn named_blocks(suffix: &'static str, row: &'static str, size: u32) -> CustomFn {
    Rc::new(move |c| {
        let inf = crate::volume::ctx(Vol::Inf, GCMN).p;
        let gcmn = inf.overlay_index["gcmn"] as u16;
        let mut syms: Vec<(String, u32, u32)> = inf
            .symbols
            .iter()
            .filter(|s| s.kind == crate::elf::STT_OBJECT && s.shndx == gcmn && s.name.ends_with(suffix))
            .map(|s| (s.name.clone(), s.value, s.size))
            .collect();
        syms.sort();
        syms.dedup();
        let layout = ty(row, vec![]);
        let mut out = Vec::new();
        for (name, value, bytes) in syms {
            let va = find(c, value, GCMN);
            let mut rows = Vec::new();
            for k in 0..bytes / size {
                rows.push(layout.read(c, va + size * k)?);
            }
            out.push(Value::List(vec![Value::Bytes(name.into_bytes()), Value::Int(i128::from(va)), Value::List(rows)]));
        }
        Ok(Value::List(out))
    })
}

/// A named block's layout: its name, where the volume keeps it, its rows.
fn named_block(name: &str, row: &str, doc: &str) -> Layout {
    strukt(name, 0, vec![("name", 0, cstr()), ("va", 4, U32), ("rows", 8, array(ty(row, vec![]), 0))], doc)
}

fn combat() -> Group {
    group(
        "combat",
        "Combat",
        "What Kite's, the members' and the ride's frames read besides the battle's parameters: their clips, \
         the following's constants, the bosses' patterns and clips.",
        vec![
            e(
                "player_anims",
                0x006F_0560,
                array(fixed_text(21), 26),
                GCMN,
                "`playerAnimTbl` (`char[26][21]`): Kite's clip for each act.",
            ),
            e(
                "dam_actu",
                0x0037_82B8,
                array(I16, 3),
                MAIN,
                "`DamActuTbl`: the pad's rumble for a light, medium and heavy hit.",
            ),
            e("tsp", 0x0037_82B0, float(), MAIN, "`tsp`: Kite's walking speed a frame."),
            e(
                "fellow_anims",
                0x005D_4050,
                array_stride(fixed(ptr(cstr()), 22), 17, 0x60),
                GCMN,
                "`fellowAnimTbl` (one per `fellowNN.cpp`, 0x60 apart): the 22 clips of `charTbl` rows 1-17.",
            ),
            e("fp_angle_offset", 0x0037_82A0, array(U16, 4), MAIN, "`fpAngleOffset[4]`."),
            derived(
                "fp_angle_before",
                U16,
                MAIN,
                "The halfword before `fpAngleOffset`, which `checkPartyMenberNum` -1 indexes.",
            )
            .after("fp_angle_offset", -2),
            e("fp_ok_range", 0x0037_82A8, float(), MAIN, "`fpOkRange`: how near a member keeps."),
            e("cfofp", 0x0037_82AC, float(), MAIN, "`CFOFP`."),
            e(
                "ride_anims",
                0x006A_EA50,
                array(fixed_text(21), 7),
                GCMN,
                "`puccigusoAnimTbl`: Kite's clips on the Grunty.",
            ),
            e("ride_anims_pg", 0x005E_E870, array(fixed_text(21), 7), GCMN, "`puccigusoAnimTblPG`: the Grunty's own."),
            e(
                "ride_files",
                0x005E_E840,
                array(ptr(cstr()), 9),
                GCMN,
                "`puccigusoCharTbl`: the Grunty's model file by kind.",
            ),
            e("ride_angles", 0x005E_E908, array(I16, 5), GCMN, "`puccigusoAngleTbl`."),
            e(
                "skeith_acts",
                0x005E_B598,
                custom(Rc::new(skeith_acts), array(array(I32, 0), 0)),
                GCMN,
                "Skeith's act tables (normal, super, Epitaph): each to its -1 and the word past it.",
            ),
            e(
                "skeith_anims",
                0x005E_B5B0,
                array(opt(cstr()), 22),
                GCMN,
                "`Boss01AnmTbl`: Skeith's clip by act, none for some.",
            ),
            e("skeith_rand_skills", 0x005E_B608, array(I32, 3), GCMN, "The skills Skeith picks from at random."),
            e(
                "innis_pattern",
                0x005E_B660,
                array(I32, 150),
                GCMN,
                "`Pattern`: Innis's action words, three runs by protect gauge, each ended by a 29.",
            ),
            e("innis_epitaph", 0x005E_B8C0, array(I32, 53), GCMN, "`EPITAPH_Pattern`: Innis's words once drained."),
            e("innis_anims", 0x005E_B620, array(opt(cstr()), 15), GCMN, "`boss02AnmTbl`: Innis's clip by act."),
            e(
                "innis_various_skills",
                0x005E_B998,
                array(I16, 7),
                GCMN,
                "`Skill_VARIOUS_INIS`: the skills Innis picks from in its first mode.",
            ),
            e(
                "innis_downer_skills",
                0x005E_B9B0,
                array(I16, 12),
                GCMN,
                "`Skill_DOWNER_INIS`: the skills Innis picks from in its second.",
            ),
            e(
                "innis_monster_anims",
                0x005E_B9D0,
                array_stride(fixed(opt(cstr()), 15), 3, 0x40),
                GCMN,
                "`Mon1`-`Mon3`: the clips of the three images Innis sends out, by act.",
            ),
            e(
                "innis_ring_models",
                0x005E_BA90,
                array(I32, 3),
                GCMN,
                "`EnemyBurst`'s rings' model (`particle` clump) by image.",
            ),
            e(
                "kyvia01_anims",
                0x005E_CB70,
                array(opt(cstr()), 15),
                GCMN,
                "`Kyvia01AnmTbl`: Kyvia's clip by act in its first fight.",
            ),
            e(
                "kyvia02_anims",
                0x005E_CBB0,
                array(opt(cstr()), 15),
                GCMN,
                "`Kyvia02AnmTbl`: Kyvia's clip by act in its second fight.",
            ),
            e(
                "kyvia_core_anims",
                0x005E_CD20,
                array(opt(cstr()), 18),
                GCMN,
                "`kyviaCoreAnmTbl`: the core's clip by act.",
            ),
            e(
                "kyvia_gomora_anims",
                0x005E_CF30,
                array(opt(cstr()), 15),
                GCMN,
                "`kyviaGomoraAnmTbl`: a gomora's clip by act.",
            ),
            e(
                "kyvia_various_skills",
                0x005E_CEF8,
                array(I16, 7),
                GCMN,
                "`Skill_VARIOUS`: the spells a gomora of attribute 1 picks from.",
            ),
            e("kyvia_downer_skills", 0x005E_CF10, array(I16, 12), GCMN, "`Skill_DOWNER`: those of attribute 2."),
            derived(
                "kyvia_gomora_lists",
                custom(Rc::new(kyvia_gomora_lists), array(array(I16, 5), 0)),
                GCMN,
                "`AllGomoraList_1[0]`: Kyvia 01's gomora lists, each gomora's attribute (4 none) by slave.",
            ),
            e(
                "kyvia_gomora_lists_2",
                0x005E_CED0,
                custom(Rc::new(kyvia_gomora_lists_2), array(array(array(I16, 5), 0), 0)),
                GCMN,
                "`AllGomoraList_2`: Kyvia 02's gomora lists by the core's deaths, as `kyvia_gomora_lists`.",
            ),
            e(
                "magus_epitaph",
                0x005E_BAA0,
                array_through(I32, Rc::new(|v| v.int() == -1)),
                GCMN,
                "`boss03EpitaphActTbl`: Magus's patterns once drained, up to and with its -1.",
            ),
            e("magus_anims", 0x005E_BB20, array(opt(cstr()), 32), GCMN, "`Boss03AnmTbl`: Magus's clip by act."),
            e(
                "magus_leaf_anims",
                0x005E_BBC0,
                array(opt(cstr()), 24),
                GCMN,
                "`Boss03SlaveAnmTbl`: a leaf's clip by act.",
            ),
            e("magus_skills", 0x005E_BBA0, array(I32, 3), GCMN, "`@1538`: pattern 10's skills, one by `ccRand() % 3`."),
            e(
                "magus_drop_skills",
                0x005E_BBB0,
                array(I32, 3),
                GCMN,
                "`@2301`: the skills `OnThinkLeafDrop` casts as its sixth leaf falls.",
            ),
            e(
                "fidchell_normal",
                0x005E_BC20,
                array_through(I32, Rc::new(|v| v.int() == -1)),
                GCMN,
                "`boss04NormalActTbl`: Fidchell's patterns, up to and with its -1.",
            ),
            e(
                "fidchell_super",
                0x005E_BCF0,
                array_through(I32, Rc::new(|v| v.int() == -1)),
                GCMN,
                "`boss04SuperActTbl`: Fidchell's once its gauge is half full.",
            ),
            e(
                "fidchell_epitaph",
                0x005E_BDC0,
                array_through(I32, Rc::new(|v| v.int() == -1)),
                GCMN,
                "`boss04EpitaphActTbl`: Fidchell's once drained.",
            ),
            e("fidchell_anims", 0x005E_BE60, array(opt(cstr()), 26), GCMN, "`boss0xAnmTbl`: Fidchell's clip by act."),
            e(
                "fidchell_pred_texts",
                0x005E_BE40,
                array(opt(cstr()), 4),
                GCMN,
                "`@1038`: the prediction's text clip (x41) by `m_predId`.",
            ),
            e(
                "fidchell_pred_voices",
                0x005E_BE50,
                array(I32, 4),
                GCMN,
                "`@1039`: the prediction's voice (`ccEvVoiceRequest(-40, n)`) by `m_predId`.",
            ),
            e(
                "fidchell_rand_skills",
                0x005E_BEC8,
                array(I32, 3),
                GCMN,
                "`@1272`: pattern 10's skills (read, never used).",
            ),
            e(
                "fidchell_pred_skills",
                0x005E_BEE0,
                array(I32, 4),
                GCMN,
                "`@1441`: the skill the prediction lays on each member, by `m_predId`.",
            ),
            e(
                "fidchell_skills",
                0x005E_BEF0,
                array(I32, 3),
                GCMN,
                "`@2048`: `OnThinkSkill`'s skills, one by `ccRand() % 3`.",
            ),
            e(
                "fidchell_magic_skills",
                0x005E_BF00,
                array(I32, 4),
                GCMN,
                "`@2081`: `OnThinkMagic`'s spells, by `ccSys.count & 3`.",
            ),
            e("gorre_anims", 0x005E_BF10, array(opt(cstr()), 25), GCMN, "`Boss05AnmTbl`: Gorre's clip by act."),
            e(
                "gorre_brother_anims",
                0x005E_BF80,
                array(opt(cstr()), 25),
                GCMN,
                "`boss05SlaveAnmTbl1`: the first brother's clip by act.",
            ),
            e(
                "gorre_brother2_anims",
                0x005E_BFF0,
                array(opt(cstr()), 25),
                GCMN,
                "`boss05SlaveAnmTbl2`: the second brother's clip by act.",
            ),
            e(
                "gorre_normal",
                0x005E_C060,
                array_through(I32, gorre_patterns()),
                GCMN,
                "`boss05NormalActTbl`: Gorre's patterns, through the -1 that closes it (pattern 10's own operand is a -1 too).",
            ),
            e(
                "gorre_super",
                0x005E_C100,
                array_through(I32, gorre_patterns()),
                GCMN,
                "`boss05SuperActTbl`: Gorre's once its gauge is half full, through its closing -1 (Outbreak's runs 4 words past Infection's).",
            ),
            e(
                "gorre_epitaph",
                0x005E_C1C0,
                array_through(I32, gorre_patterns()),
                GCMN,
                "`boss05EpitaphActTbl`: Gorre's once drained, through its closing -1.",
            ),
            e(
                "gorre_magic_skills",
                0x005E_C240,
                array(I32, 4),
                GCMN,
                "`@1672`: `OnThinkMagic`'s spells, one by `abs(ccRand()) & 3`, cast by the second brother.",
            ),
            e(
                "gorre_skills",
                0x005E_C250,
                array(I32, 4),
                GCMN,
                "`@1777`: `OnThinkSkill`'s spells, one by `ccRand() & 3` (157 and 158 the first brother's, 159 and 160 the second's).",
            ),
            e(
                "gorre_tornade_skills",
                0x005E_C260,
                array(I32, 4),
                GCMN,
                "`@2074`: `SendMessage`'s msg 7, the brother's spell in the Tornade, one by `ccRand() & 3`.",
            ),
            e(
                "cinema_skill_names",
                0x005E_B040,
                array(
                    strukt(
                        "CinemaSkillName",
                        12,
                        vec![("file", 0, opt(cstr())), ("tex", 4, opt(cstr())), ("row", 8, I32)],
                        "A boss cinema's skill name: the file and texture it is in, and its row.",
                    ),
                    72,
                ),
                GCMN,
                "`_g_cinemaSkillName`: `OnCinemaMode(n)`'s name, by `n`.",
            ),
            derived(
                "cinema_skill_rows",
                custom(
                    Rc::new(cinema_skill_rows),
                    array(
                        strukt(
                            "CinemaSkillRow",
                            20,
                            vec![
                                ("field", 0, I32),
                                ("sid", 4, I32),
                                ("file", 8, opt(cstr())),
                                ("tex", 12, opt(cstr())),
                                ("row", 16, I32),
                            ],
                            "A boss cinema's name for a skill in a field: the file and texture it is in, and its row.",
                        ),
                        0,
                    ),
                ),
                GCMN,
                "The cinema's names by `game.field` and skill (`OnCinemaMode` with a skill, from Outbreak on); none before.",
            ),
            // The enemies' weapon trails, dust and breath (enemy1.cpp - enemyZ.cpp).
            derived(
                "weapon_infos",
                custom(
                    named_blocks("WpInfo", "ccEnemyWpInfo", 0x60),
                    array(
                        named_block(
                            "WpInfos",
                            "ccEnemyWpInfo",
                            "An enemy weapon block (`*WpInfo`, `ccEnemyWpInfo[]`) by name, where the volume keeps it.",
                        ),
                        0,
                    ),
                ),
                GCMN,
                "Every `*WpInfo`: the races' weapon trail blocks, by name.",
            ),
            derived(
                "dust_infos",
                custom(
                    named_blocks("DustInfo", "ccEnemyDustInfo", 0x20),
                    array(
                        named_block(
                            "DustInfos",
                            "ccEnemyDustInfo",
                            "An enemy dust block (`*DustInfo`, `ccEnemyDustInfo[]`) by name, where the volume keeps it.",
                        ),
                        0,
                    ),
                ),
                GCMN,
                "Every `*DustInfo`: the races' dust blocks, by name.",
            ),
            e(
                "ehk_breath_info",
                0x005E_27A0,
                array(ty("ccEnemyBrInfo", vec![("objp", omit()), ("ccsc", omit())]), 2),
                GCMN,
                "`ehkBreathInfo`: `ccEnemyH`'s two fire breaths.",
            ),
            derived("ehk_breath_info_va", addr(), GCMN, "Where the volume keeps it.").after("ehk_breath_info", 0),
            e(
                "el_br_info",
                0x005E_5AE0,
                array(ty("ccEnemyBrInfo", vec![("objp", omit()), ("ccsc", omit())]), 17),
                GCMN,
                "`elBrInfo`: `ccEnemyL`'s breaths.",
            ),
            derived("el_br_info_va", addr(), GCMN, "Where the volume keeps it.").after("el_br_info", 0),
            e(
                "ehk_br_param",
                0x005E_2760,
                array(ty("ccEnemyBrParam", vec![]), 1),
                GCMN,
                "`ehkBrParam`: the fire breath's timing and parameters.",
            ),
            derived("ehk_br_param_va", addr(), GCMN, "Where the volume keeps it.").after("ehk_br_param", 0),
            e("el_br_param", 0x005E_5A20, array(ty("ccEnemyBrParam", vec![]), 3), GCMN, "`elBrParam`: `ccEnemyL`'s."),
            derived("el_br_param_va", addr(), GCMN, "Where the volume keeps it.").after("el_br_param", 0),
            e(
                "foot_objs",
                0x005E_0640,
                array(fixed_text(30), 4),
                GCMN,
                "`footObjTbl` (`char[4][30]`): a turtle's feet, by foot.",
            ),
        ],
    )
}

/// A `STREAMDATA` record: a file of a stream.
fn stream_data() -> Layout {
    ty("STREAMDATA", vec![])
}

/// The streams each volume numbers: 134 on Infection, 140 from Mutation on
/// (six inserted before `str6100`, which moves from 106 to 112, and the
/// shared `STRSUB` ones from 126 to 138). `streamTbl`, `streamTblE` and
/// `strSndTbl` have as many rows; the code indexes them unchecked.
fn streams() -> Rc<dyn Fn(&Ctx) -> usize> {
    later(134, 140)
}

/// `streamTbl[]` (or `streamTblE`): each stream's list, its header and
/// files up to the record with no name.
fn stream_lists() -> Layout {
    array_by(opt(array_until(stream_data(), Rc::new(|r| r.list()[0] == Value::None))), streams())
}

/// A gate-hack table: per row its setup file and its scene.
fn gate_rows(n: usize) -> Layout {
    array(fixed(stream_data(), 2), n)
}

/// `evStrMsgTbl` (`evStrMsgTblp`): each stream's subtitle records, read
/// on past its own as a note past the table does (the next stream's, as
/// the game's pointer walks), while they read as records with text, 64 at
/// most. One row a stream: Mutation's tables keep Infection's 136 slots,
/// so its streams 136-139 read `evStrMsgTblp`'s first rows, as its
/// `ccEventStream` does.
fn subtitles(inf: u32) -> CustomFn {
    Rc::new(move |c| {
        let t = find(c, inf, None);
        let ev = ev_msg();
        let mut out = Vec::new();
        for k in 0..streams()(c) as u32 {
            let base = c.p.u32(t + 4 * k)?;
            if base == 0 {
                out.push(Value::None);
                continue;
            }
            let mut rows = Vec::new();
            for i in 0..64 {
                match ev.read(c, base + 12 * i) {
                    Ok(r) if r.list()[2] != Value::None => rows.push(r),
                    _ => break,
                }
            }
            out.push(Value::List(rows));
        }
        Ok(Value::List(out))
    })
}

/// A `ccEventObjTbl` row as the ending's tasks take it: its cue, name, and
/// by its `type` the puff burst's parameters (0), the rock creator's (1)
/// or both through a pair of pointers (2); none for another type.
fn event_obj() -> Layout {
    strukt(
        "EventObj",
        0x10,
        vec![
            ("cue", 0, U32),
            ("name", 4, opt(cstr())),
            ("eff", 12, opt(ty("ccEffPartParam0580", vec![]))),
            ("rock", 12, opt(ty("ccObjPartParam0580", vec![]))),
        ],
        "An ending event entry (`ccEventObjTbl`): its cue, name and what it starts.",
    )
}

/// `eventObjTbl_0580` read on through to `eventObjTbl_0581` and its rows,
/// as the ending's walk reads the run (a stray string between them reads
/// as nothing).
fn event_objs(c: &Ctx) -> Read {
    let (a, b) = (find(c, 0x0034_EB60, None), find(c, 0x0034_F1F0, None));
    let mut out = Vec::new();
    for at in (a..b).step_by(16).chain((b..b + 6 * 16).step_by(16)) {
        out.push(event_obj_row(c, at)?);
    }
    Ok(Value::List(out))
}

/// One `ccEventObjTbl` row at `at`, as [`event_obj`] lays it out.
fn event_obj_row(c: &Ctx, at: u32) -> Read {
    let (eff, rock) = (ty("ccEffPartParam0580", vec![]), ty("ccObjPartParam0580", vec![]));
    let (cue, name, kind, param) = (c.p.u32(at)?, c.p.u32(at + 4)?, c.p.u32(at + 8)?, c.p.u32(at + 12)?);
    let name = if name == 0 { Value::None } else { cstr().read(c, name).unwrap_or(Value::None) };
    let (e, r) = match kind {
        0 => (eff.read(c, param).ok(), None),
        1 => (None, rock.read(c, param).ok()),
        2 => match (c.p.u32(param), c.p.u32(param.wrapping_add(4))) {
            (Ok(p0), Ok(p1)) => match (eff.read(c, p0), rock.read(c, p1)) {
                (Ok(e), Ok(r)) => (Some(e), Some(r)),
                _ => (None, None),
            },
            _ => (None, None),
        },
        _ => (None, None),
    };
    Ok(Value::List(vec![Value::Int(i128::from(cue)), name, e.unwrap_or(Value::None), r.unwrap_or(Value::None)]))
}

/// The code of the effect task `StreamDemoFuncTbl` (main 0x0034f420)
/// names for `scene`, up to its `jr $ra`; None where the volume's table
/// has no such row before its end (a name pointer that is 0 or not
/// mapped: Infection's 22 rows run into other data).
fn demo_func(c: &Ctx, scene: &[u8]) -> Result<Option<Vec<u32>>, String> {
    let t = find(c, 0x0034_F420, None);
    for k in 0..64 {
        let name = c.p.u32(t + 8 * k)?;
        if name == 0 || !c.p.mapped(name) {
            return Ok(None);
        }
        if c.p.cstr(name, 16)? == scene {
            let f = c.p.u32(t + 8 * k + 4)?;
            let mut code = Vec::new();
            for i in 0..0x1000 {
                let w = c.p.u32(f + 4 * i)?;
                code.push(w);
                if w == 0x03e0_0008 {
                    break;
                }
            }
            return Ok(Some(code));
        }
    }
    Ok(None)
}

/// The address a `lui rt` / `addiu rt, rt` pair at `code[i..]` builds
/// (`rt` any when None).
fn lui_addiu(code: &[u32], i: usize, rt: Option<u32>) -> Option<u32> {
    let (w, x) = (*code.get(i)?, *code.get(i + 1)?);
    let r = (w >> 16) & 31;
    let pair = w >> 26 == 0x0f && x >> 26 == 0x09 && (x >> 21) & 31 == r && (x >> 16) & 31 == r;
    (pair && rt.is_none_or(|t| t == r)).then(|| ((w & 0xffff) << 16).wrapping_add(sext16(x) as u32))
}

/// The Flag Race's texts after `helpStr`, as its menus split them.
fn race_str() -> Layout {
    let names = [
        ("cost", 2),
        ("retry", 2),
        ("no_money", 3),
        ("select", 1),
        ("start", 1),
        ("paused", 1),
        ("quit", 1),
        ("record", 3),
        ("rankings", 1),
        ("colon", 1),
        ("specs", 1),
        ("stars", 1),
    ];
    let fields = names.iter().enumerate().map(|(k, &(n, l))| (n, 4 * k as u32, text_lines(l))).collect();
    strukt("RaceStr", 48, fields, "The Flag Race's texts: each its pieces (`ccKanjiStrSeparate`).")
}

/// `race_ranks`' and `race_grunties`' layouts: four towns of three.
fn race_ranks() -> Layout {
    let rank = strukt(
        "RaceRank",
        8,
        vec![("name", 0, ptr(cstr())), ("time", 4, I16), ("row", 6, I16)],
        "A ranking: the racer, the time in frames, the Grunty's `npcTbl` row.",
    );
    array(fixed(rank, 3), 4)
}
fn race_grunties() -> Layout {
    let pg = strukt(
        "RaceGrunty",
        8,
        vec![("row", 0, I16), ("speed", 2, I16), ("accel", 4, I16), ("turn", 6, I16)],
        "A Grunty to race: its `npcTbl` row and its stars.",
    );
    array(fixed(pg, 3), 4)
}

/// The Flag Race's two getters by town (MUT gcmn 0x005ff9f0, 0x005ffa20):
/// `addiu $v1, $a0, -1; sll; addu; sll 3`, then the table's `lui/addiu`.
/// Two places do this, the rankings first.
fn race_tables(c: &Ctx) -> Result<[u32; 2], String> {
    const HEAD: [u32; 4] = [0x2483_ffff, 0x0003_1040, 0x0043_1021, 0x0002_18c0];
    let (lo, hi) = c.p.code_range(true);
    let code = (lo..hi).step_by(4).map(|a| c.p.u32(a)).collect::<Result<Vec<u32>, String>>()?;
    let found: Vec<u32> = (0..code.len())
        .filter(|&i| code[i..].starts_with(&HEAD))
        .filter_map(|i| lui_addiu(&code, i + 4, None))
        .collect();
    match found[..] {
        [ranks, grunties] => Ok([ranks, grunties]),
        _ => Err(format!("{} Flag Race getters, want two", found.len())),
    }
}

/// `gift_wants`' and `gift_bands`' layouts.
fn wants() -> Layout {
    array(fixed(item_list(), 16), 20)
}
fn bands() -> Layout {
    array(fixed(I32, 4), 2)
}
fn rates() -> Layout {
    array(fixed(I16, 14), 20)
}

/// The later volumes' gift tables (wants, bands, rates), found in the code
/// that weighs a gift (MUT gcmn 0x00573710): `lui/addiu $fp` loads the
/// bands, and later `addiu $fp, $fp, 16` takes the second row
/// (0x00573aac). Just after the load a member's `sll 6` row is added to
/// the wants (0x00573728); just before it the rates are added to
/// `ccGetItemTradeRate`'s `$a0` (0x005736d0). One place does this.
fn gift_tables(c: &Ctx) -> Result<[u32; 3], String> {
    const SECOND_ROW: u32 = 0x27de_0010;
    let (lo, hi) = c.p.code_range(true);
    let code = (lo..hi).step_by(4).map(|a| c.p.u32(a)).collect::<Result<Vec<u32>, String>>()?;
    let sll6 = |w: u32| w != 0 && w >> 26 == 0 && w & 63 == 0 && (w >> 6) & 31 == 6;
    let to_a0 = |w: u32| w >> 26 == 0 && w & 63 == 0x21 && (w >> 11) & 31 == 4;
    let found: Vec<[u32; 3]> = (0..code.len())
        .filter_map(|i| {
            let bands = lui_addiu(&code, i, Some(30))?;
            code.get(i + 2..(i + 400).min(code.len()))?.contains(&SECOND_ROW).then_some(())?;
            let wants = (i + 2..(i + 40).min(code.len()))
                .filter(|&k| sll6(code[k]))
                .find_map(|k| (k + 1..k + 4).find_map(|j| lui_addiu(&code, j, None)))?;
            let rates = (i.saturating_sub(24)..i)
                .rev()
                .filter(|&k| code.get(k + 2).is_some_and(|&w| to_a0(w)))
                .find_map(|k| lui_addiu(&code, k, None))?;
            Some([wants, bands, rates])
        })
        .collect();
    match found[..] {
        [one] => Ok(one),
        _ => Err(format!("{} places weigh a gift, want one", found.len())),
    }
}

/// `eventObjTbl_0710` (MUT main 0x00366e30): the table `Func_str0710`
/// gives its walker (the first address it builds after its `new` of 12
/// bytes), up to its end row; none on Infection.
fn opening_events(c: &Ctx) -> Read {
    let Some(code) = demo_func(c, b"str0710")? else { return Ok(Value::List(Vec::new())) };
    let at = code.iter().position(|&w| w == 0x2404_000c).ok_or("Func_str0710 makes no ccEventObj")?;
    let table = (at..code.len()).find_map(|i| lui_addiu(&code, i, None)).ok_or("Func_str0710 builds no table")?;
    let mut out = Vec::new();
    for k in 0..64 {
        let row = event_obj_row(c, table + 16 * k)?;
        let end = row.list()[0].int() == 100_000;
        out.push(row);
        if end {
            break;
        }
    }
    Ok(Value::List(out))
}

/// The opening's texts: after each `lb $v0, -0x7bd5($at)` in
/// `Func_str0710` (`saveData.parodyFlag`), the two strings it builds in
/// $a1, the normal one then Parody Mode's; cue 700's, then cue 710's.
fn opening_text(c: &Ctx) -> Read {
    let Some(code) = demo_func(c, b"str0710")? else { return Ok(Value::List(vec![Value::None; 4])) };
    let mut out = Vec::new();
    for (i, &w) in code.iter().enumerate() {
        if w != 0x8022_842b {
            continue;
        }
        let mut n = 0;
        for j in i..(i + 16).min(code.len()) {
            if let Some(va) = lui_addiu(&code, j, Some(5)) {
                out.push(cstr().read(c, va)?);
                n += 1;
                if n == 2 {
                    break;
                }
            }
        }
    }
    if out.len() != 4 {
        return Err(format!("Func_str0710 builds {} texts", out.len()));
    }
    Ok(Value::List(out))
}

/// A `Func_str1070` part creator's block (0x50 bytes): base and random
/// pairs for the orbit's radius, its angle and turn (degrees), the height,
/// the rise, and the spins about y and z (degrees); then the parts a frame
/// (1/4096), its random part, and the frames it makes them for.
fn part_param_1070() -> Layout {
    strukt(
        "PartParam1070",
        0x50,
        vec![("f", 0, fixed(float(), 14)), ("rate", 0x38, I32), ("rate_rand", 0x3c, I32), ("life", 0x40, I32)],
        "A `Func_str1070` part creator's block: base and random pairs (radius, angle, turn, height, rise, the two spins), then the rate, its random part and the life.",
    )
}

/// The addresses each `ccPartCreate` of `Func_str1070` takes (`sw $zero,
/// 56($v0)`, then its block built), in the order made, and its creator's
/// `Ctrl` (the class's vtable slot 3). Mutation's only: Outbreak's
/// `Func_str1070` is other code, not read yet.
fn str1070_blocks(c: &Ctx) -> Result<Option<(Vec<u32>, u32)>, String> {
    if c.volume != Vol::Mut {
        return Ok(None);
    }
    let Some(code) = demo_func(c, b"str1070")? else { return Ok(None) };
    let mut blocks = Vec::new();
    for i in 0..code.len() {
        if code[i] == 0xac40_0038
            && let Some(b) = (i + 1..(i + 4).min(code.len())).find_map(|j| lui_addiu(&code, j, None))
            && !blocks.contains(&b)
        {
            blocks.push(b);
        }
    }
    // The class's own vtable: the last `sw $v1, 44($v0)` before the first
    // creator takes its group (`sw $s2, 48($v0)`).
    let group = code.iter().position(|&w| w == 0xac52_0030).ok_or("Func_str1070 makes no ccPartCreate")?;
    let vt = (2..group)
        .rev()
        .find(|&i| code[i] == 0xac43_002c)
        .and_then(|i| lui_addiu(&code, i - 2, Some(3)))
        .ok_or("Func_str1070's creator has no vtable")?;
    Ok(Some((blocks, c.p.u32(vt + 12)?)))
}

/// `Func_str1070`'s creators' blocks (MUT main 0x00321c00, seven); none on
/// Infection.
fn part_params_1070(c: &Ctx) -> Read {
    let Some((blocks, _)) = str1070_blocks(c)? else { return Ok(Value::List(Vec::new())) };
    blocks.into_iter().map(|b| part_param_1070().read(c, b)).collect::<Result<Vec<_>, _>>().map(Value::List)
}

/// The models its parts take (MUT main 0x00366f70, `rand() % 10`): the
/// chunk (of [`part_chunks_1070`]) and the scale; the table its creators'
/// `Ctrl` builds the address of twice (the scale's +4 first).
fn part_models_1070(c: &Ctx) -> Read {
    let Some((_, ctrl)) = str1070_blocks(c)? else { return Ok(Value::List(Vec::new())) };
    let mut code = Vec::new();
    for i in 0..0x200 {
        let w = c.p.u32(ctrl + 4 * i)?;
        code.push(w);
        if w == 0x03e0_0008 {
            break;
        }
    }
    let table = (0..code.len())
        .filter_map(|i| lui_addiu(&code, i, Some(2)))
        .filter(|&a| a >= 0x0010_0000)
        .min()
        .ok_or("the creator's Ctrl builds no table")?;
    let row = strukt(
        "PartModel1070",
        8,
        vec![("chunk", 0, I32), ("scale", 4, float())],
        "A `Func_str1070` part's model: the chunk and its scale.",
    );
    array(row, 10).read(c, table)
}

/// The chunks `Func_str1070` finds in `str1070e` (MUT main 0x00321e30, six
/// name pointers): the first address it builds after its effect file's.
fn part_chunks_1070(c: &Ctx) -> Read {
    let none = || Ok(Value::List(vec![Value::None; 6]));
    if c.volume != Vol::Mut {
        return none();
    }
    let Some(code) = demo_func(c, b"str1070")? else { return none() };
    let at = code.iter().position(|&w| w == 0x0c05_0eb4).ok_or("Func_str1070 reads no effect file")?;
    let table = (at..code.len()).find_map(|i| lui_addiu(&code, i, Some(2))).ok_or("Func_str1070 names no chunks")?;
    fixed(ptr(cstr()), 6).read(c, table)
}

/// `Func_str0880`'s hit marks' rotations (MUT main 0x00366ed0): the table
/// it builds just before loading pi (`ori $v0, $v0, 0x0fdb`), four rows of
/// x, y, z degrees and a pad; none on Infection.
fn hit_rot_0880(c: &Ctx) -> Read {
    let Some(code) = demo_func(c, b"str0880")? else { return Ok(Value::List(Vec::new())) };
    let pi = code.iter().position(|&w| w == 0x3442_0fdb).ok_or("Func_str0880 loads no pi")?;
    let table = (pi.saturating_sub(12)..pi)
        .rev()
        .find_map(|i| lui_addiu(&code, i, None))
        .ok_or("Func_str0880 builds no table")?;
    array_stride(fixed(float(), 3), 4, 16).read(c, table)
}

fn stream() -> Group {
    group(
        "stream",
        "Stream",
        "What the in-engine streams read (main): each stream's files, its music changes, the gate hack's \
         files, the subtitles, the stream effects' and the ending's tables.",
        vec![
            e("lists", 0x0030_EF90, stream_lists(), MAIN, "`streamTbl`: each stream's header record, then its files; none on Outbreak and Quarantine, whose `ccStreamInit` takes `streamTblE` whatever the voice.")
                .absent(Vol::Out, Value::List(Vec::new()))
                .absent(Vol::Qua, Value::List(Vec::new())),
            e("lists_e", 0x0031_02F0, stream_lists(), MAIN, "`streamTblE`: the same with English voices (`saveData.voice`)."),
            e("bgm", 0x0030_B950, array_by(ty("ccSndStrTbl", vec![("strse", omit()), ("strbgm", opt(array_through(ty("CCSND_STR_BGM", vec![]), Rc::new(|r| r.list()[0].int() < 0 && r.list()[5].int() == 5))))]), streams()), MAIN, "`strSndTbl`: per stream its BGM table (`strbgm`), up to and with its end record (`bgm1` below 0, `param` 5)."),
            // The Japanese-voice gate-hack tables: none on Outbreak and
            // Quarantine, whose gate hack reads the E ones.
            e("gate_pre", 0x0030_E1A0, array(stream_data(), 1), MAIN, "`str7000TblPre`: the Chaos Gate movie's setup file.")
                .absent(Vol::Out, Value::List(Vec::new()))
                .absent(Vol::Qua, Value::List(Vec::new())),
            e("gate_town", 0x0030_E1D0, gate_rows(5), MAIN, "`str7000TblTown`: per town its setup file and departure scene.")
                .absent(Vol::Out, Value::List(Vec::new()))
                .absent(Vol::Qua, Value::List(Vec::new())),
            e("gate_town_crisis", 0x0030_E2B0, gate_rows(5), MAIN, "`str7000TblTownC`: the same in the crisis.")
                .absent(Vol::Out, Value::List(Vec::new()))
                .absent(Vol::Qua, Value::List(Vec::new())),
            e("gate_after", 0x0030_E390, gate_rows(27), MAIN, "`str7000TblAfter`: per arrival its setup file and scene.")
                .absent(Vol::Out, Value::List(Vec::new()))
                .absent(Vol::Qua, Value::List(Vec::new())),
            e("gate_pre_e", 0x0030_F2B0, array(stream_data(), 1), MAIN, "`str7000TblPreE`."),
            e("gate_town_e", 0x0030_F2E0, gate_rows(5), MAIN, "`str7000TblTownE`."),
            e("gate_town_crisis_e", 0x0030_F3C0, gate_rows(5), MAIN, "`str7000TblTownCE`."),
            e("gate_after_e", 0x0030_F4A0, gate_rows(27), MAIN, "`str7000TblAfterE`."),
            e("gate_out", 0x0030_F1B0, array_until(ty("ccGateHackOut", vec![]), Rc::new(|r| r.list()[0].int() < 0)), MAIN, "`str7000Out`: a field's arrival row, up to a negative field."),
            e("subtitles", 0x0031_12F0, custom(subtitles(0x0031_12F0), array(opt(array(ev_msg(), 0)), 0)), MAIN, "`evStrMsgTbl`: each stream's subtitles, read on past its own records as the game's pointer walks (none for a stream without)."),
            e("subtitles_parody", 0x0031_1510, custom(subtitles(0x0031_1510), array(opt(array(ev_msg(), 0)), 0)), MAIN, "`evStrMsgTblp`: Parody Mode's."),
            e("hit_rot_0120a", 0x0034_F320, array_stride(fixed(float(), 3), 8, 16), MAIN, "`hitRot0120a` (x, y, z degrees; three), read on to eight as the cues 610-614 do (the next data, the jump table's denormal floats, which the EE takes as 0).").carried(),
            e("hit_rot_0240", 0x0034_E3D0, array_stride(fixed(float(), 3), 8, 16), MAIN, "`hitRot0240` (seven), its eighth mark reading on into the next data.").carried(),
            e("hit_rot_0250", 0x0034_E450, fixed(float(), 3), MAIN, "`hitRot0250`: `Func_str0250`'s rotation."),
            e("hit_rot_0301", 0x0034_F310, fixed(float(), 3), MAIN, "`hitRot0301`: `Func_str0301`'s rotation."),
            e("event_objs", 0x0034_EB60, custom(Rc::new(event_objs), array(event_obj(), 0)), MAIN, "`eventObjTbl_0580` read on through `eventObjTbl_0581`: the ending's cues."),
            e("rock_scale", 0x0034_E490, fixed(float(), 3), MAIN, "`rockScaleTbl`: a rock's scale by its model."),
            derived("opening_events", custom(Rc::new(opening_events), array(event_obj(), 0)), MAIN, "`eventObjTbl_0710` (MUT main 0x00366e30): the opening's (`Func_str0710`) cues, up to its end row; none on Infection."),
            derived("opening_text", custom(Rc::new(opening_text), fixed(opt(cstr()), 4)), MAIN, "The opening's texts (`Func_str0710`'s cues 700 and 710), each normal then Parody Mode's; none on Infection."),
            derived("part_params_1070", custom(Rc::new(part_params_1070), array(part_param_1070(), 0)), MAIN, "`Func_str1070`'s part creators' blocks (MUT main 0x00321c00); none on Infection."),
            derived("part_models_1070", custom(Rc::new(part_models_1070), array(strukt("PartModel1070", 8, vec![("chunk", 0, I32), ("scale", 4, float())], "A `Func_str1070` part's model: the chunk and its scale."), 0)), MAIN, "`Func_str1070`'s part models (MUT main 0x00366f70, ten); none on Infection."),
            derived("part_chunks_1070", custom(Rc::new(part_chunks_1070), fixed(opt(cstr()), 6)), MAIN, "The `str1070e` chunks `Func_str1070`'s parts draw (MUT main 0x00321e30); none on Infection."),
            derived("hit_rot_0880", custom(Rc::new(hit_rot_0880), array_stride(fixed(float(), 3), 4, 16)), MAIN, "`Func_str0880`'s hit marks' rotations (MUT main 0x00366ed0: x, y, z degrees); none on Infection."),
        ],
    )
}

/// A particle generator's row (`ccParticleGeneratorParam`).
fn gen_param() -> Layout {
    ty("ccParticleGeneratorParam", vec![])
}

/// A particle force field's row (`ccParticleForceFieldParam`).
fn ff_param() -> Layout {
    ty("ccParticleForceFieldParam", vec![])
}

/// An `effectTbl` row: the file, the object and its kind.
fn effect_row() -> Layout {
    strukt(
        "EffectRow",
        12,
        vec![("ccs", 0, opt(cstr())), ("name", 4, opt(cstr())), ("kind", 8, I32)],
        "An `effectTbl` row (`ccEffectTbl`): the effect's file, its object and its kind.",
    )
}

/// `ccParticle::Setup`'s texture-id switch as compiled: the bias and bound
/// from its `addi a0, id, -first` and `sltiu at, a0, n` (found within 0x200
/// bytes of its start: Outbreak and Quarantine recompiled it an instruction
/// later), the table from the `lui`/`addiu` 0x10 after, and each case's two
/// assignments (`li r, k`, `move r, zero` or `move r, id`), the row's
/// register and the CLUT's being those the first case assigns, in that
/// order: from `first`, each id's (particleTbl row, CLUT flag).
fn tex_switch(c: &Ctx) -> Read {
    let setup = find(c, 0x001B_F0F0, None);
    let bad = |what: &str| format!("ccParticle::Setup's switch: {what}");
    let is_head = |a: u32| -> bool {
        let (Ok(addi), Ok(sltiu)) = (c.p.u32(a), c.p.u32(a + 4)) else { return false };
        addi >> 26 == 0x08 && (addi >> 16) & 31 == 4 && sltiu >> 16 == 0x2c81
    };
    let head = (0..0x200 / 4).map(|k| setup + 4 * k).find(|&a| is_head(a)).ok_or_else(|| bad("unexpected code"))?;
    let (addi, sltiu) = (c.p.u32(head)?, c.p.u32(head + 4)?);
    let (lui, addiu) = (c.p.u32(head + 0x10)?, c.p.u32(head + 0x14)?);
    if lui >> 26 != 0x0f || addiu >> 26 != 0x09 {
        return Err(bad("unexpected code"));
    }
    let id = (addi >> 21) & 31;
    let first = -i32::from(addi as u16 as i16);
    let n = sltiu & 0xffff;
    let table = ((lui & 0xffff) << 16).wrapping_add(addiu as u16 as i16 as i32 as u32);
    let assign = |w: u32, tex: i32| -> Option<(u32, i32)> {
        let (op, rs, rt, rd) = (w >> 26, (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31);
        if op == 0x09 && rs == 0 {
            return Some((rt, i32::from(w as u16 as i16)));
        }
        if op == 0 && w & 0x3f == 0x2d && rt == 0 {
            return match rs {
                0 => Some((rd, 0)),
                r if r == id => Some((rd, tex)),
                _ => None,
            };
        }
        None
    };
    let mut regs = None;
    let mut map = Vec::new();
    for i in 0..n {
        let tex = first + i as i32;
        let target = c.p.u32(table + 4 * i)?;
        let a = assign(c.p.u32(target)?, tex).ok_or_else(|| bad("unexpected case"))?;
        let b = assign(c.p.u32(target + 4)?, tex).ok_or_else(|| bad("unexpected case"))?;
        let (row_reg, clut_reg) = *regs.get_or_insert((a.0, b.0));
        let value = |r: u32| [a, b].iter().find(|x| x.0 == r).map(|x| x.1);
        match (value(row_reg), value(clut_reg)) {
            (Some(row), Some(clut)) => {
                map.push(Value::List(vec![Value::Int(i128::from(row)), Value::Int(i128::from(clut))]))
            }
            _ => return Err(bad("unexpected case")),
        }
    }
    Ok(Value::List(vec![Value::Int(i128::from(first)), Value::List(map)]))
}

/// `tornadeThunderTbl[16]`'s pointers as offsets into the time lists at
/// 0x0033fd40 (INF), where each entry's list starts.
fn thunder_offsets(c: &Ctx) -> Read {
    let (t, lists) = (find(c, 0x0033_FDB0, None), find(c, 0x0033_FD40, None));
    let mut out = Vec::new();
    for k in 0..16 {
        out.push(Value::Int(i128::from(c.p.u32(t + 4 * k)?.wrapping_sub(lists) as i32)));
    }
    Ok(Value::List(out))
}

fn effect() -> Group {
    let ints = |n| array(I32, n);
    let floats = |n| array(float(), n);
    let names = |n| array(ptr(cstr()), n);
    group(
        "effect",
        "Effect",
        "What the effects read: the effect files and rows, the particle system's tables, the spells', the hits' \
         and the fly font's, the boss effects' generator rows.",
        vec![
            // The files and the rows (effect.cpp, main).
            e("files", 0x002F_D4F0, array_until(ty("FILEDATA", vec![("fn", fixed_text(32))]), Rc::new(|r| r.list()[0].bytes().is_empty())), MAIN, "`effectCCSTbl`: the effect files, up to the row with no name."),
            e("rows", 0x0033_F240, array(effect_row(), 173), MAIN, "`effectTbl`."),
            e("rows2", 0x0033_FA60, array(effect_row(), 4), MAIN, "`effectTbl2`."),
            e("str_rows", 0x0033_FA90, array(effect_row(), 3), MAIN, "`effectStrTbl`."),
            // The particle system (main).
            e("generators", 0x0034_02F0, array(gen_param(), 244), MAIN, "`particleGeneratorTbl`."),
            derived("generators_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("generators", 0),
            e("force_fields", 0x0034_3850, array(ff_param(), 148), MAIN, "`particleForceFieldTbl`."),
            derived("force_fields_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("force_fields", 0),
            e("particles", 0x0034_0220, array(ty("ccParticleParam", vec![]), 245), MAIN, "`particleTbl` (101 rows), read on to 245 as `Setup` indexes it by texture id (id 237 falls through its switch): rows 101 on are the halfwords that follow.").carried(),
            e("particle_effects", 0x0034_4AD0, array(ty("ccParticleEffectParam", vec![]), 49), MAIN, "`particleEffectTbl`: up to four generators about a character."),
            e("ccs_anm", 0x0037_39E0, array(ty("P_CCS", vec![]), 245), MAIN, "`particleCcsAnmTbl`: each particle object's file and name."),
            e("polyhedron", 0x0033_EF90, array(fixed(float(), 4), 42), MAIN, "`Polyhedron82Table`: 42 points about the origin (w 1), `Generate`'s rType 3."),
            e("smoke_ff1", 0x0033_FAC0, ff_param(), MAIN, "`ccpffpSmoke1`: effect.cpp's force field for its static generators."),
            derived("smoke_ff1_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("smoke_ff1", 0),
            e("smoke_ff2", 0x0033_FAE0, ff_param(), MAIN, "`ccpffpSmoke2`."),
            derived("smoke_ff2_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("smoke_ff2", 0),
            e("smoke_ff3", 0x0033_FB00, ff_param(), MAIN, "`ccpffpSmoke3`."),
            derived("smoke_ff3_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("smoke_ff3", 0),
            e("smoke_ff4", 0x0033_FB20, ff_param(), MAIN, "`ccpffpSmoke4`."),
            derived("smoke_ff4_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("smoke_ff4", 0),
            e("hit_photon_ff1", 0x0033_FED0, ff_param(), MAIN, "`hitPhotonDummyF1`."),
            derived("hit_photon_ff1_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("hit_photon_ff1", 0),
            e("hit_photon_ff2", 0x0033_FEF0, ff_param(), MAIN, "`hitPhotonDummyF2`."),
            derived("hit_photon_ff2_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("hit_photon_ff2", 0),
            e("tex_switch", 0x001B_F0F0, custom(Rc::new(tex_switch), strukt("TexSwitch", 0, vec![("first", 0, I32), ("map", 4, array(fixed(I32, 2), 0))], "`ccParticle::Setup`'s texture-id switch: from `first`, each id's (particleTbl row, CLUT flag); ids outside are their own row with no CLUT.")), MAIN, "`ccParticle::Setup`'s switch on a texture id, decoded from its code."),
            e("element_generators", 0x005E_CF70, array(gen_param(), 13), GCMN, "`effElementGeneratorTbl`: the element spells' generator rows."),
            derived("element_generators_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("element_generators", 0),
            e("element_ffs", 0x005E_D250, array(ff_param(), 22), GCMN, "`effElementFFTbl`: their force fields."),
            derived("element_ffs_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("element_ffs", 0),
            // The hits and the fly font.
            e("panel_cam_dist", 0x0037_7EEC, float(), MAIN, "`panelCamDist`."),
            e("miss", 0x0037_8228, cstr(), MAIN, "`ffMissStr`."),
            e("ff_miss", 0x006D_FED8, cstr(), GCMN, "`ffstrMISS`."),
            e("ff_exp", 0x006D_FEE0, cstr(), GCMN, "`ffstrEXP`."),
            e("ff_level_down", 0x006D_FEC8, cstr(), GCMN, "`ffstrLEVELDOWN`."),
            e("pow10", 0x0065_0DB0, ints(9), GCMN, "`Int2StrFF`'s powers of ten."),
            // The spells.
            e("tornado_se", 0x0065_1920, ints(4), GCMN, "`TornadoSystem`'s sound codes."),
            e("ring89", 0x0033_FF50, array(fixed(float(), 4), 4), MAIN, "Ring 89's rows."),
            e("ring90", 0x0033_FF90, array(fixed(fixed(float(), 4), 2), 5), MAIN, "Ring 90's rows."),
            e("ring91", 0x0034_0030, array(fixed(fixed(float(), 4), 4), 5), MAIN, "Ring 91's rows."),
            e("turn90", 0x0037_7FA8, array(I16, 2), MAIN, "Ring 90's turns."),
            e("turn91", 0x0037_7FB0, array(I16, 4), MAIN, "Ring 91's turns."),
            e("tornado_smoke1", 0x0033_FB40, array(ff_param(), 4), MAIN, "`skillTornadeSmokePFF1`, by level."),
            derived("tornado_smoke1_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("tornado_smoke1", 0),
            e("tornado_smoke2", 0x0033_FBC0, array(ff_param(), 4), MAIN, "`skillTornadeSmokePFF2`."),
            derived("tornado_smoke2_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("tornado_smoke2", 0),
            e("tornado_smoke3", 0x0033_FC40, array(ff_param(), 4), MAIN, "`skillTornadeSmokePFF3`."),
            derived("tornado_smoke3_va", addr(), MAIN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("tornado_smoke3", 0),
            e("tornade_start1", 0x005E_D760, ints(4), GCMN, "The tornado's first start frames, by element (fire)."),
            e("tornade_start1b", 0x005E_D790, ints(4), GCMN, "The same, the second table."),
            e("tornade_start2", 0x005E_D770, ints(4), GCMN, "The second start frames."),
            e("tornade_start2b", 0x005E_D7A0, ints(4), GCMN, "The same, the second table."),
            e("tornade_sndcode", 0x005E_D780, ints(4), GCMN, "The tornado's sound codes."),
            e("tornade_sndcodeb", 0x005E_D7B0, ints(4), GCMN, "The same, the second table."),
            e("thunder_eff", 0x0033_FCC0, ty("ccEffThunderData", vec![]), MAIN, "`skillThunderEff`."),
            e("thunder_eff2", 0x0033_FD00, ty("ccEffThunderData", vec![]), MAIN, "`skillThunderEff2`."),
            e("thunder_times", 0x0033_FD40, array(ty("ccEffTimeTbl", vec![]), 256), MAIN, "The lists `tornadeThunderTbl` points into (`ccEffTimeTbl` pairs), read on as an entry's cursor walks.").carried(),
            e("thunder_offsets", 0x0033_FDB0, custom(Rc::new(thunder_offsets), ints(16)), MAIN, "`tornadeThunderTbl`: where each entry's list starts, in bytes into `thunder_times`."),
            e("fall_level", 0x0037_8260, array(I16, 4), MAIN, "`FallSystemLevelTbl`."),
            e("fall_delay", 0x0037_8268, array(I16, 4), MAIN, "`fsDelay`."),
            e("fall_et", 0x0034_01D0, array(I16, 12), MAIN, "The fall's element table, three a level."),
            e("fall_elements", 0x005E_D510, ints(4), GCMN, "The fall's elements."),
            e("fall_dark_scale", 0x005E_D520, floats(4), GCMN, "The fall's darkness scales."),
            e("fall_hits", 0x005E_D540, ints(4), GCMN, "The fall's hits."),
            e("fall_bolts", 0x005E_D550, ints(4), GCMN, "The fall's bolts."),
            e("fall_bolt_hits", 0x005E_D560, ints(4), GCMN, "The fall's bolts' hits."),
            e("convergence_radiate", 0x0065_1930, ints(4), GCMN, "`ConvergenceSystem`'s radiate frames."),
            e("convergence_smoke", 0x0065_1940, ints(4), GCMN, "Its smoke frames."),
            e("convergence_wait", 0x0065_1950, ints(4), GCMN, "Its wait frames."),
            e("convergence_pieces", 0x0034_01C0, ints(4), MAIN, "Its pieces."),
            e("convergence_iv", 0x0037_7FB8, float(), MAIN, "`effIV`."),
            e("convergence_sr", 0x0037_7EE8, float(), MAIN, "`effSR`."),
            e("convergence_elements", 0x005E_D570, ints(4), GCMN, "Its elements."),
            e("pillar_deg", 0x0065_1960, array(I16, 16), GCMN, "`pillarDegTbl`."),
            e("hand_deg", 0x005E_D580, array(I16, 16), GCMN, "The upheaval's hands' angles."),
            e("tree_deg", 0x005E_D5B0, array(I16, 16), GCMN, "Its trees' angles."),
            e("rock_deg", 0x005E_D5D0, array(I16, 16), GCMN, "Its rocks' angles."),
            e("ice_raise", 0x005E_D5A0, ints(4), GCMN, "The ice's raise frames."),
            e("ice_hit", 0x0037_81F8, ints(2), MAIN, "The ice's hit frames."),
            e("upheaval_rocks", 0x005E_D6F0, names(4), GCMN, "The upheaval's rock models."),
            e("upheaval_trees", 0x005E_D700, names(4), GCMN, "Its tree models."),
            e("summons_wave", 0x0033_FF10, array_stride(fixed(float(), 3), 4, 16), MAIN, "Shock wave 64's rows."),
            e("summoned_fire", 0x005E_D5F0, ints(4), GCMN, "The summoned fire's counts."),
            e("summoned_water", 0x005E_D600, ints(4), GCMN, "The water's."),
            e("summoned_thunder_rings", 0x005E_D620, ints(4), GCMN, "The thunder's rings."),
            e("summoned_spline_deg", 0x005E_D630, floats(3), GCMN, "The spline's angles."),
            e("summoned_soil", 0x005E_D640, ints(4), GCMN, "The soil's."),
            e("summoned_tree_rings", 0x005E_D650, ints(4), GCMN, "The tree's rings."),
            e("summoned_goblin_rings", 0x005E_D660, ints(4), GCMN, "The goblin's rings."),
            e("summoned_circle_clut", 0x005E_D670, names(7), GCMN, "The circle's CLUTs by element."),
            e("summoned_dark_star_tex", 0x005E_D690, I32, GCMN, "The dark star's texture."),
            e("summoned_needles", 0x005E_D6A0, names(4), GCMN, "The needles' models."),
            e("summoned_energy_clut", 0x005E_D740, names(7), GCMN, "The energy's CLUTs by element."),
            e("draw_rock", 0x005E_D6B0, names(4), GCMN, "The element draw's rocks."),
            e("draw_ice", 0x005E_D6C0, names(9), GCMN, "Its ice."),
            e("draw_blantch", 0x005E_D710, names(4), GCMN, "Its branches."),
            e("draw_subst", 0x005E_D720, names(4), GCMN, "Its substitutes."),
            e("draw_clut", 0x005E_D730, names(4), GCMN, "Its CLUTs."),
            e("draw_suffix", 0x006A_C540, array(fixed_text(8), 6), GCMN, "Its suffixes (`char[6][8]`)."),
            e("draw_kinds", 0x005E_D7C0, ints(6), GCMN, "Its kinds."),
            e("exec_ring", 0x0037_7F48, floats(4), MAIN, "The skill start's execution ring."),
            e("force_ring", 0x0037_7F58, floats(4), MAIN, "Its force ring."),
            e("force_ring2", 0x0037_7F68, floats(8), MAIN, "Its second force ring."),
            e("circle", 0x0037_7F88, floats(4), MAIN, "Its circle."),
            e("circle_fade", 0x0037_7F98, ints(4), MAIN, "Its circle's fade frames."),
            e("wave2", 0x0037_7EF0, floats(8), MAIN, "Shock wave 80's rows."),
            e("wave2_turn", 0x0037_7F10, array(I16, 2), MAIN, "Its turns."),
            e("wave3", 0x0037_7F18, floats(10), MAIN, "Shock wave 81's rows."),
            e("wave3_turn", 0x0037_7F40, array(I16, 2), MAIN, "Its turns."),
            e("cure_a", 0x0034_0170, ints(3), MAIN, "The cure's rows A-D."),
            e("cure_b", 0x0034_0180, ints(3), MAIN, "Row B."),
            e("cure_c", 0x0034_0190, ints(3), MAIN, "Row C."),
            e("cure_d", 0x0034_01A0, ints(3), MAIN, "Row D."),
            e("ability_up", 0x0033_FDF0, array(ints(8), 4), MAIN, "`tableAbilityUpS1-4`."),
            e("ability_down", 0x0033_FE70, array(ints(8), 3), MAIN, "`tableAbilityDownS1-3`."),
            // The boss effects' rows (GCMN.PRG).
            e("boss_eruption", 0x005E_EA30, gen_param(), GCMN, "`g_eruptionGenerator`."),
            derived("boss_eruption_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_eruption", 0),
            e("boss_eruption_ff", 0x005E_EA70, array(ff_param(), 2), GCMN, "`g_eruptionForceField`."),
            derived("boss_eruption_ff_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_eruption_ff", 0),
            e("boss_magic_square", 0x005E_FF60, array(gen_param(), 8), GCMN, "`MagicSquareGenerator`."),
            derived("boss_magic_square_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_magic_square", 0),
            e("boss_magic_square_ff", 0x005F_0120, array(ff_param(), 13), GCMN, "`MagicSquareForceField`."),
            derived("boss_magic_square_ff_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_magic_square_ff", 0),
            e("boss_energy_out", 0x005F_0580, gen_param(), GCMN, "`g_energyOutGenerator`."),
            derived("boss_energy_out_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_energy_out", 0),
            e("boss_energy_out_ff", 0x005F_05C0, array(ff_param(), 3), GCMN, "`g_energyOutForceField`."),
            derived("boss_energy_out_ff_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_energy_out_ff", 0),
            e("boss_ice_smoke", 0x005F_02C0, gen_param(), GCMN, "`g_IceSmokeGenerator`."),
            derived("boss_ice_smoke_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_ice_smoke", 0),
            e("boss_ice_smoke_ff", 0x005F_0300, array(ff_param(), 3), GCMN, "`g_IceSmokeForceField`."),
            derived("boss_ice_smoke_ff_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_ice_smoke_ff", 0),
            e("boss_dead", 0x005E_EED0, array(gen_param(), 3), GCMN, "`BossDeadGenerator`."),
            derived("boss_dead_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_dead", 0),
            e("boss_dead_ff", 0x005E_EF80, array(ff_param(), 7), GCMN, "`BossDeadForceField`."),
            derived("boss_dead_ff_va", addr(), GCMN, "Where the volume keeps it: its rows' identity, as the game's pointers to them.").after("boss_dead_ff", 0),
        ],
    )
}

/// `naviMapTownTable[town]`'s landmarks, `naviMarkTable[town]` + 1 of
/// them: `ccNaviGetLandmarkPos` reads one past the last (the next town's
/// first).
fn navi_landmarks(c: &Ctx) -> Read {
    let (marks, towns) = (find(c, 0x0061_9408, None), find(c, 0x0061_9420, None));
    let row = ty("ccLandmark", vec![]);
    let mut out = Vec::new();
    for town in 0..5 {
        let num = c.p.read(marks + 2 * town, 2).map(|b| i16::from_le_bytes([b[0], b[1]]))?;
        let base = c.p.u32(towns + 4 * town)?;
        let mut rows = Vec::new();
        for k in 0..=num.max(0) as u32 {
            rows.push(row.read(c, base + 0x30 * k)?);
        }
        out.push(Value::List(rows));
    }
    Ok(Value::List(out))
}

/// `naviMainLinesOfTownTable[town]`: each town's main lines (8, 10, 10, 18
/// and 14 of them), each an `unsigned char` run: two bytes, `n`, `2n`
/// bytes, a count and that many bytes more; none for a null.
fn navi_lines(c: &Ctx) -> Read {
    let t = find(c, 0x0061_9440, None);
    let mut out = Vec::new();
    for (town, n_lines) in [8u32, 10, 10, 18, 14].into_iter().enumerate() {
        let tbl = c.p.u32(t + 4 * town as u32)?;
        let mut lines = Vec::new();
        for k in 0..n_lines {
            let a = c.p.u32(tbl + 4 * k)?;
            if a == 0 {
                lines.push(Value::None);
                continue;
            }
            let n = u32::from(c.p.read(a + 2, 1)?[0]);
            let count = u32::from(c.p.read(a + 3 + 2 * n, 1)?[0]);
            let bytes = c.p.read(a, (4 + 2 * n + count) as usize)?;
            lines.push(Value::List(bytes.into_iter().map(|b| Value::Int(i128::from(b))).collect()));
        }
        out.push(Value::List(lines));
    }
    Ok(Value::List(out))
}

/// `rtpcAnmTbl[ccsType]`: each walking PC file's clips, standing, walking
/// and running (`ctw2AnmTbl`, row 15, four more).
fn rtpc_anms(c: &Ctx) -> Read {
    let t = find(c, 0x005E_DEB0, GCMN);
    let mut out = Vec::new();
    for k in 0..49 {
        let tbl = c.p.u32(t + 4 * k)?;
        out.push(array(opt(cstr()), if k == 15 { 7 } else { 3 }).read(c, tbl)?);
    }
    Ok(Value::List(out))
}

/// A `FIELDMAPICON *[11]`: by field type, its objects' icons (as many as
/// the volume's symbol there holds), none for a null.
fn map_icons(inf: u32) -> CustomFn {
    Rc::new(move |c| {
        let t = find(c, inf, GCMN);
        let row = ty("FIELDMAPICON", vec![]);
        let mut out = Vec::new();
        for k in 0..11 {
            let a = c.p.u32(t + 4 * k)?;
            if a == 0 {
                out.push(Value::None);
                continue;
            }
            let n = c.p.symbol_at(a, 0).filter(|&(_, o)| o == 0).map_or(0, |(s, _)| s.size / 0x18);
            let rows = (0..n).map(|i| row.read(c, a + 0x18 * i)).collect::<Result<Vec<_>, _>>()?;
            out.push(Value::List(rows));
        }
        Ok(Value::List(out))
    })
}

fn world() -> Group {
    group(
        "world",
        "World",
        "What The World's towns, fields and story read besides the battle's and the menus' tables.",
        vec![
            e(
                "markers",
                0x0031_7DA0,
                array(ptr(cstr()), 33),
                GCMN,
                "`markerEvTbl`: the town's event markers, by the scripts' number (dummy objects of its file).",
            ),
            e(
                "map_labels",
                0x0037_82F8,
                array(ptr(cstr()), 2),
                GCMN,
                "`mapmsg`: the field map's two labels (\"Overall Map\", \"Default Map\").",
            ),
            e(
                "book",
                0x0037_8150,
                array(ptr(cstr()), 4),
                GCMN,
                "`bookItemAddMsg`, `bookWallPaperAdd`, `bookBgmAdd`, `bookMovieAdd`: the story's announcement of a desktop item.",
            ),
            // The towns' navigation (navi.cpp).
            e("navi_marks", 0x0061_9408, array(I16, 5), GCMN, "`naviMarkTable`: each town's landmarks."),
            e(
                "navi_landmarks",
                0x0061_9420,
                custom(Rc::new(navi_landmarks), array(array(ty("ccLandmark", vec![]), 0), 0)),
                GCMN,
                "`naviMapTownTable`: each town's landmarks, one past its last as `ccNaviGetLandmarkPos` reads (the next town's first).",
            ),
            e(
                "navi_lines",
                0x0061_9440,
                custom(Rc::new(navi_lines), array(array(opt(array(U8, 0)), 0), 0)),
                GCMN,
                "`naviMainLinesOfTownTable`: each town's main lines (`unsigned char` runs), by line number.",
            ),
            e(
                "navi_points",
                0x0065_3C80,
                array(opt(cstr()), 8),
                GCMN,
                "`naviPointNameTable`: the dummies of the points the party walks to.",
            ),
            // The walking PCs (rtownnpc.cpp).
            e("rtpc_ccs_names", 0x005E_D900, array(opt(cstr()), 49), GCMN, "`rtpcCcsName`: the files by `ccsType`."),
            e(
                "rtpc_weapon_names",
                0x005E_D9D0,
                array(opt(cstr()), 98),
                GCMN,
                "`rtpcWeaponName`: the weapons of `npcTbl` rows 30-127.",
            ),
            e(
                "tvpc_weapon_names",
                0x005E_DB60,
                array(opt(cstr()), 9),
                GCMN,
                "`tvpcWeaponName`: the weapons of `npcTbl` rows 159-167.",
            ),
            derived(
                "rtpc_anms",
                custom(Rc::new(rtpc_anms), array(array(opt(cstr()), 0), 0)),
                GCMN,
                "`rtpcAnmTbl`: by `ccsType`, the clips standing, walking and running (`ctw2`, row 15, four more).",
            ),
            e("rtpc_chat", 0x005E_DF80, array(opt(cstr()), 4), GCMN, "`rtpcChatTbl`: what a PC says passing Kite."),
            e(
                "rtpc_chat_shop",
                0x005E_E0F0,
                array(ptr(array(opt(cstr()), 4)), 6),
                GCMN,
                "`rtpcChatMesShop`: what a PC says leaving a shop, by shop.",
            ),
            e(
                "rtpc_chat_mes",
                0x005E_E110,
                array(ptr(array(opt(cstr()), 13)), 4),
                GCMN,
                "`rtpcChatMes`: a chat group's conversations.",
            ),
            e(
                "mark_pos",
                0x005E_E120,
                array(ptr(fixed(fixed(I8, 4), 5)), 16),
                GCMN,
                "`markPosTbl` -> `markerTbl[town]`: a PC's start, first target, chat group and place in it.",
            ),
            e(
                "merchan_num",
                0x005E_E160,
                fixed(fixed(I8, 5), 5),
                GCMN,
                "`merchanNum[town]`: each merchant's landmark.",
            ),
            // The Grunties (pg.cpp) and the breeders' camera.
            e("pg_cam_pos", 0x005E_E550, fixed(fixed(float(), 4), 4), GCMN, "`camPos`: the Grunty's camera by stage."),
            e("pg_cam_view", 0x005E_E590, fixed(fixed(float(), 4), 4), GCMN, "`camView`: what it looks at."),
            e("pg_inu_pos", 0x005E_E5D0, fixed(fixed(float(), 4), 4), GCMN, "`inuPos`: where the Grunty stands."),
            e("pg_player_pos", 0x005E_E610, fixed(fixed(float(), 4), 4), GCMN, "`playerPos`: where Kite stands."),
            e("pg_chat", 0x005E_E650, array(opt(cstr()), 16), GCMN, "`pgChatTbl`: the Grunty's lines."),
            e(
                "breeder_cam_pos",
                0x005E_D880,
                fixed(fixed(float(), 4), 4),
                GCMN,
                "`breederCamPos`: where the camera stands when a breeder is spoken to, by server.",
            ),
            e(
                "breeder_cam_view",
                0x005E_D8C0,
                fixed(fixed(float(), 4), 4),
                GCMN,
                "`breederCamView`: what it looks at, by server.",
            ),
            // The maps.
            derived(
                "town_icons",
                custom(
                    named_blocks("ICONPOS", "ICONPOS", 0x24),
                    array(named_block("TownIcons", "ICONPOS", "A town map's signs (`RT0nICONPOS`) by name."), 0),
                ),
                GCMN,
                "`RT01ICONPOS` ..: each town map's signs.",
            ),
            derived(
                "key_icons",
                custom(map_icons(0x0065_8680), array(opt(array(ty("FIELDMAPICON", vec![]), 0)), 0)),
                GCMN,
                "`KeyIconTBL`: by field type, the key objects' icons on the field map.",
            ),
            derived(
                "sub_icons",
                custom(map_icons(0x0065_86B0), array(opt(array(ty("FIELDMAPICON", vec![]), 0)), 0)),
                GCMN,
                "`SubIconTBL`.",
            ),
            derived(
                "base_icons",
                custom(map_icons(0x0065_86E0), array(opt(array(ty("FIELDMAPICON", vec![]), 0)), 0)),
                GCMN,
                "`BaseIconTBL`.",
            ),
            derived(
                "tree_icons",
                custom(map_icons(0x0065_8710), array(opt(array(ty("FIELDMAPICON", vec![]), 0)), 0)),
                GCMN,
                "`TreeIconTBL`.",
            ),
            e("fp_dungeon", 0x0065_84B0, ty("FIELDMAPICON", vec![]), GCMN, "`FP_DUNGEON`: the dungeon's icon."),
            e(
                "field_minimap",
                0x0065_71F0,
                array(opt(cstr()), 11),
                GCMN,
                "`fieldminimap`: the field map's texture by field type.",
            ),
            e("level_str", 0x0069_5550, array(opt(cstr()), 10), GCMN, "`levelstr`: the dungeon map's floors."),
        ],
    )
}

/// A `VOICE_DATA` table of an event: its rows by message.
fn voice_rows() -> Layout {
    array(opt(array(ty("VOICE_DATA", vec![]), 0)), 0)
}

/// One of `ccEvVoiceRequest`'s tables: its file (`voiceFile`), the
/// Japanese rows and, where the code asks `saveData.voice`, the English.
fn voice_branch() -> Layout {
    strukt(
        "VoiceBranch",
        0,
        vec![("file", 0, I32), ("jp", 4, voice_rows()), ("en", 8, opt(voice_rows()))],
        "A table `ccEvVoiceRequest` picks: its `voiceFile`, then per event (from its first) the rows by message; `en` where the code takes another table when `saveData.voice` is set.",
    )
}

/// `ccEvVoiceRequest` (main 0x0017e810) as the volume's own code has it, by
/// `event / 100`: each hundred's main (0-49), side (50-99) and, where the code
/// has one, Parody Mode table. Every `lui`/`addiu` table base before a
/// `voiceFile` store is that file's, Japanese first; the file names the
/// hundred and kind (6 + 3 (v - 1) main, + 1 side, + 2 parody); a base is its
/// table less 4 times the first event, and a table runs to the next (at most
/// 50 events).
fn event_voices(c: &Ctx) -> Read {
    use std::collections::{BTreeSet, HashMap};
    let f = find(c, 0x0017_E810, MAIN);
    let size = match c.p.symbol_at(f, 0) {
        Some((s, 0)) => s.size,
        _ => return Err(format!("no function starts at 0x{f:08x}")),
    };
    let voice_file = find(c, 0x0037_89E4, MAIN);
    let gp = c.p.gp.ok_or("no $gp")?;
    let (mut hi, mut pending, mut li) = (HashMap::new(), Vec::new(), None);
    let mut stores: Vec<(i128, Vec<u32>)> = Vec::new();
    for k in 0..size / 4 {
        let w = c.p.u32(f + 4 * k)?;
        let (op, rs, rt) = (w >> 26, (w >> 21) & 31, (w >> 16) & 31);
        match op {
            0x0f => {
                hi.insert(rt, (w & 0xffff) << 16);
            }
            0x09 if rs == 0 => li = Some(sext16(w)),
            0x09 if rs == rt => {
                if let Some(h) = hi.remove(&rs) {
                    pending.push(h.wrapping_add(sext16(w) as u32));
                }
            }
            0x2b if rs == 28 && gp.wrapping_add(sext16(w) as u32) == voice_file => {
                stores.push((li.ok_or("a voiceFile store with no li before it")?, std::mem::take(&mut pending)));
            }
            _ => {}
        }
    }
    // Each table's start: the base plus 4 times its first event.
    let mut tables: Vec<(i128, Vec<u32>)> = Vec::new();
    for (file, bases) in &stores {
        let (vol, kind) = ((file - 6) / 3, (file - 6) % 3);
        if !(0..4).contains(&vol) {
            return Err(format!("voiceFile {file} is no event volume's"));
        }
        let first = (vol * 100 + if kind == 1 { 50 } else { 0 }) as u32;
        tables.push((*file, bases.iter().map(|b| b.wrapping_add(4 * first)).collect()));
    }
    let starts: BTreeSet<u32> = tables.iter().flat_map(|t| t.1.iter().copied()).collect();
    let bound = |set: &BTreeSet<u32>, a: u32| set.range(a + 1..).next().copied();
    let mut entries: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut arrays: BTreeSet<u32> = BTreeSet::new();
    for &t in &starts {
        let n = bound(&starts, t).map_or(50, |b| ((b - t) / 4).min(50));
        // A placeholder table (the volumes a disc does not voice) runs into
        // other words: it ends at the first that is neither null nor an
        // address.
        let mut e = Vec::new();
        for i in 0..n {
            let a = c.p.u32(t + 4 * i)?;
            if a != 0 && c.p.u32(a).is_err() {
                break;
            }
            e.push(a);
        }
        arrays.extend(e.iter().copied().filter(|&a| a != 0));
        entries.insert(t, e);
    }
    let edges: BTreeSet<u32> = arrays.iter().chain(&starts).copied().collect();
    let row = ty("VOICE_DATA", vec![]);
    let read = |t: u32| -> Read {
        let mut out = Vec::new();
        for &a in &entries[&t] {
            if a == 0 {
                out.push(Value::None);
                continue;
            }
            // A row is a line (an offset in whole sectors, a size) or
            // none (-1, -1); the array ends before the first that is
            // neither (the padding to the next array, or other data).
            let n = bound(&edges, a).map_or(1, |b| ((b - a) / 8).min(128));
            let mut rows = Vec::new();
            for i in 0..n {
                let (ofs, siz) = (c.p.u32(a + 8 * i)? as i32, c.p.u32(a + 8 * i + 4)? as i32);
                let line = ofs >= 0 && ofs % 2048 == 0 && siz > 0;
                if !(line || (ofs == -1 && siz == -1)) {
                    break;
                }
                rows.push(row.read(c, a + 8 * i)?);
            }
            out.push(if rows.is_empty() { Value::None } else { Value::List(rows) });
        }
        Ok(Value::List(out))
    };
    let mut sections = vec![[Value::None, Value::None, Value::None]; 4];
    for (file, starts) in &tables {
        let (vol, kind) = (((file - 6) / 3) as usize, ((file - 6) % 3) as usize);
        let jp = read(starts[0])?;
        let en = match starts.get(1) {
            Some(&t) => read(t)?,
            None => Value::None,
        };
        sections[vol][kind] = Value::List(vec![Value::Int(*file), jp, en]);
    }
    Ok(Value::List(sections.into_iter().map(|s| Value::List(s.into())).collect()))
}

/// A voice file's name at `va` as a path on the disc: the IOP's
/// `cdrom0:\\VOICE_E\\EVVOL1_E.BIN` is `VOICE_E/EVVOL1_E.BIN`.
fn disc_path(c: &Ctx, va: u32) -> Read {
    let Value::Bytes(name) = cstr().read(c, va)? else { unreachable!() };
    let path = name.iter().position(|&b| b == b':').map_or(&name[..], |i| &name[i + 1..]);
    let path = path.strip_prefix(b"\\").unwrap_or(path);
    Ok(Value::Bytes(path.iter().map(|&b| if b == b'\\' { b'/' } else { b }).collect()))
}

/// A list of voice files at Infection's `inf` as paths on the disc: its
/// pointers up to a NULL or the next symbol (Mutation's `evVoiceFile` runs
/// straight into `evVoiceFileE`, which ends in a NULL).
fn file_names(inf: u32) -> CustomFn {
    Rc::new(move |c| {
        let a = find(c, inf, MAIN);
        let syms = &c.p.symbols;
        let next = syms.get(syms.partition_point(|s| s.value <= a)).map_or(a + 256, |s| s.value);
        let mut out = Vec::new();
        for k in 0..(next - a) / 4 {
            let w = c.p.u32(a + 4 * k)?;
            if w == 0 {
                break;
            }
            out.push(disc_path(c, w)?);
        }
        Ok(Value::List(out))
    })
}

/// The gate's refusal, from the code of `GtNewMenu` (gcmn 0x0055df10;
/// the random, list and history menus have the same): an `lb` of
/// `saveData.eventStatus[n]` (+0x64f8), then the table its loop compares
/// `GetEventAreaNumber` with (the next `lui`/`addiu`) and the count it runs
/// to (the next `$gp` word), and the record its `ccMessage::Open` shows
/// (built into `$a1`). None where the code has no such test (Infection).
fn gate_refusal(c: &Ctx) -> Read {
    use std::collections::HashMap;
    let f = find(c, 0x0055_DF10, GCMN);
    let w = body(c, f)?;
    let Some(k0) = w.iter().position(|&x| x >> 26 == 0x20 && (0x64f8..0x6548).contains(&(x & 0xffff))) else {
        return Ok(Value::None);
    };
    let status = i128::from((w[k0] & 0xffff) - 0x64f8);
    let gp = c.p.gp.ok_or("no $gp")?;
    let (mut hi, mut table, mut count) = (HashMap::new(), None, None);
    for &x in &w[k0..(k0 + 40).min(w.len())] {
        let (op, rs, rt) = (x >> 26, (x >> 21) & 31, (x >> 16) & 31);
        match op {
            0x0f => {
                hi.insert(rt, (x & 0xffff) << 16);
            }
            0x09 if rs == rt && table.is_none() && hi.contains_key(&rs) => {
                table = Some(hi[&rs].wrapping_add(sext16(x) as u32));
            }
            0x23 if rs == 28 && count.is_none() => count = Some(c.p.u32(gp.wrapping_add(sext16(x) as u32))?),
            _ => {}
        }
    }
    let (Some(table), Some(count)) = (table, count) else {
        return Err(format!("0x{:08x}: the refusal's table or count not found", f + 4 * k0 as u32));
    };
    let mut areas = Vec::new();
    for i in 0..count.min(256) {
        areas.push(Value::Int(i128::from(c.p.u32(table + 4 * i)? as i32)));
    }
    let open = c.p.symbol_named("Open__9ccMessageFP11ccEvMsgDataPcii").ok_or("no ccMessage::Open")?.value;
    let kj =
        w.iter().position(|&x| x == 0x0c00_0000 | ((open >> 2) & 0x03ff_ffff)).ok_or("no call to ccMessage::Open")?;
    let (mut hi, mut record) = (None, None);
    for &x in &w[kj.saturating_sub(12)..kj] {
        let (op, rs, rt) = (x >> 26, (x >> 21) & 31, (x >> 16) & 31);
        if op == 0x0f && rt == 5 {
            hi = Some((x & 0xffff) << 16);
        } else if op == 0x09 && rs == 5 && rt == 5 {
            record = hi.map(|h| h.wrapping_add(sext16(x) as u32));
        }
    }
    let record = record.ok_or("the refusal's record is not built into $a1")?;
    Ok(Value::List(vec![Value::Int(status), Value::List(areas), ev_msg().read(c, record)?]))
}

/// A function's words, from its symbol's size.
fn body(c: &Ctx, f: u32) -> Result<Vec<u32>, String> {
    let size = match c.p.symbol_at(f, 0) {
        Some((s, 0)) => s.size,
        _ => return Err(format!("no function starts at 0x{f:08x}")),
    };
    (0..size / 4).map(|k| c.p.u32(f + 4 * k)).collect()
}

/// A voice table's rows from `a`, up to `bound` (the next table) or the
/// next symbol, and at most 256. A row is a line (an offset in whole sectors, a size) or none
/// (-1, -1); the rows end before the first that is neither (the padding to
/// the next table, or other data). With `end`, a (0, 0) row ends them and
/// is kept: the skill words' tables close with one, which `skillVoicePlay`
/// sends like any other.
fn voice_table(c: &Ctx, a: u32, bound: Option<u32>, end: bool) -> Read {
    let syms = &c.p.symbols;
    let next = syms.get(syms.partition_point(|s| s.value <= a)).map(|s| s.value);
    let bound = match (bound, next) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, y) => x.or(y),
    };
    let n = bound.map_or(256, |b| (b.saturating_sub(a) / 8).min(256));
    let row = ty("VOICE_DATA", vec![]);
    let mut rows = Vec::new();
    for i in 0..n {
        let (ofs, siz) = (c.p.u32(a + 8 * i)? as i32, c.p.u32(a + 8 * i + 4)? as i32);
        let line = ofs >= 0 && ofs % 2048 == 0 && siz > 0;
        let last = end && ofs == 0 && siz == 0;
        if !(line || last || (ofs == -1 && siz == -1)) {
            break;
        }
        rows.push(row.read(c, a + 8 * i)?);
        if last {
            break;
        }
    }
    if rows.is_empty() {
        return Err(format!("0x{a:08x} holds no voice rows"));
    }
    Ok(Value::List(rows))
}

/// What one of `ccVoiceRequest`'s cases, or `ccVoicePgFood`, builds: the
/// Japanese table, the English one, the file both set, and Mutation's
/// alternative for some English rows (`VoiceAlt`).
struct VoiceCase {
    jp: u32,
    en: u32,
    file: i128,
    alt: Option<VoiceCaseAlt>,
}

struct VoiceCaseAlt {
    first: i128,
    end: i128,
    talk: i128,
    add: i128,
    table: u32,
    file: i128,
}

/// A voice case's code, words `lo..hi` of the function at `f`, read as it
/// stands: the `lb` of `saveData.voice` (+0x842c) and its `bnez` start the
/// English branch; each `lui`/`addiu` pair builds a table whose file is the
/// next `voiceFile` store; a call to a `talkNum` (+0x220c) getter makes the
/// next table an alternative for the messages its two `slti` bound, its row
/// the message plus the `addiu` before it. One Japanese table before the
/// branch, one English after it besides any alternative, both one file.
fn voice_case(c: &Ctx, w: &[u32], f: u32, lo: usize, hi: usize) -> Result<VoiceCase, String> {
    use std::collections::HashMap;
    let voice_file = find(c, 0x0037_89E4, MAIN);
    let gp = c.p.gp.ok_or("no $gp")?;
    let at = |k: usize| f + 4 * k as u32;
    let branch = |k: usize| (k as i128 + 1 + sext16(w[k])) as usize;
    let mut en_start = None;
    let (mut hi_reg, mut li) = (HashMap::new(), HashMap::new());
    // (word, address, the message's addend when the table is an alternative)
    let mut builds: Vec<(usize, u32, Option<i128>)> = Vec::new();
    let mut stores: Vec<(usize, i128)> = Vec::new();
    let (mut slti, mut add, mut talk) = (Vec::new(), None, None);
    for k in lo..hi {
        let x = w[k];
        let (op, rs, rt) = (x >> 26, (x >> 21) & 31, (x >> 16) & 31);
        match op {
            0x0f => {
                hi_reg.insert(rt, (x & 0xffff) << 16);
            }
            0x09 if rs == 0 => {
                li.insert(rt, sext16(x));
            }
            0x09 if rs == rt && hi_reg.contains_key(&rs) => {
                let a = hi_reg.remove(&rs).unwrap().wrapping_add(sext16(x) as u32);
                builds.push((k, a, add.take()));
            }
            0x09 if rs != rt && rs != 0 && add.is_some() => add = Some(sext16(x)),
            0x0a => slti.push((k, sext16(x))),
            0x20 if sext16(x) == sext16(0x842c) && en_start.is_none() => {
                let b = w[k + 1];
                if b >> 26 != 0x05 || (b >> 21) & 31 != rt || (b >> 16) & 31 != 0 {
                    return Err(format!("0x{:08x}: saveData.voice is not tested", at(k)));
                }
                en_start = Some(branch(k + 1));
            }
            0x03 => {
                let callee = (x & 0x03ff_ffff) << 2;
                let reads_talk =
                    (0..16).any(|i| c.p.u32(callee + 4 * i).is_ok_and(|y| y >> 26 == 0x20 && y & 0xffff == 0x220c));
                if !reads_talk {
                    return Err(format!("0x{:08x}: the call to 0x{callee:08x} reads no talkNum", at(k)));
                }
                if talk.is_some() {
                    return Err(format!("0x{:08x}: a second call", at(k)));
                }
                talk = Some(li.get(&5).copied().ok_or(format!("0x{:08x}: no li $a1 before the call", at(k)))?);
                add = Some(0);
            }
            0x2b if rs == 28 && gp.wrapping_add(sext16(x) as u32) == voice_file => {
                let v =
                    if rt == 0 { 0 } else { *li.get(&rt).ok_or(format!("0x{:08x}: voiceFile from no li", at(k)))? };
                stores.push((k, v));
            }
            _ => {}
        }
    }
    let en_start = en_start.ok_or(format!("0x{:08x}: no saveData.voice test", at(lo)))?;
    let file_of = |k: usize| {
        stores
            .iter()
            .find(|s| s.0 > k)
            .map(|s| s.1)
            .ok_or(format!("0x{:08x}: no voiceFile store after the table", at(k)))
    };
    let jp: Vec<_> = builds.iter().filter(|b| b.0 < en_start).collect();
    let en: Vec<_> = builds.iter().filter(|b| b.0 >= en_start && b.2.is_none()).collect();
    let alt: Vec<_> = builds.iter().filter(|b| b.0 >= en_start && b.2.is_some()).collect();
    let ([jp], [en]) = (jp.as_slice(), en.as_slice()) else {
        return Err(format!("0x{:08x}: {} Japanese and {} English tables", at(lo), jp.len(), en.len()));
    };
    let file = file_of(jp.0)?;
    if file_of(en.0)? != file {
        return Err(format!("0x{:08x}: the English table sets another file", at(en.0)));
    }
    let alt = match alt.as_slice() {
        [] => None,
        [a] => {
            let bounds: Vec<i128> = slti.iter().filter(|s| s.0 >= en_start && s.0 < a.0).map(|s| s.1).collect();
            let (&[first, end], Some(talk)) = (bounds.as_slice(), talk) else {
                return Err(format!("0x{:08x}: an alternative without its two bounds and call", at(a.0)));
            };
            Some(VoiceCaseAlt { first, end, talk, add: a.2.unwrap(), table: a.1, file: file_of(a.0)? })
        }
        _ => return Err(format!("0x{:08x}: {} alternatives", at(lo), alt.len())),
    };
    Ok(VoiceCase { jp: jp.1, en: en.1, file, alt })
}

/// `ccVoiceRequest` (main 0x0017eeb0) and `ccVoicePgFood` (0x001800e0) as
/// the volume's code has them: the groups by the dispatch (`li r, group`,
/// `beq $a0, r`), each case read by `voice_case`, then the food. A table
/// runs to the next table either builds.
fn field_voice_cases(c: &Ctx) -> Result<(Vec<(i128, VoiceCase)>, VoiceCase), String> {
    let f = find(c, 0x0017_EEB0, MAIN);
    let w = body(c, f)?;
    let mut targets: Vec<(i128, usize)> = Vec::new();
    let mut k = 0;
    while k + 1 < w.len() && targets.iter().all(|t| k < t.1) {
        let (x, b) = (w[k], w[k + 1]);
        let li = x >> 26 == 0x09 && (x >> 21) & 31 == 0;
        if li && b >> 26 == 0x04 && (b >> 21) & 31 == 4 && (b >> 16) & 31 == (x >> 16) & 31 {
            targets.push((sext16(x), (k as i128 + 2 + sext16(b)) as usize));
        }
        k += 1;
    }
    let mut starts: Vec<usize> = targets.iter().map(|t| t.1).collect();
    starts.sort();
    let mut cases = Vec::new();
    for &(group, lo) in &targets {
        let hi = starts.iter().copied().find(|&s| s > lo).unwrap_or(w.len());
        cases.push((group, voice_case(c, &w, f, lo, hi)?));
    }
    cases.sort_by_key(|g| -g.0);
    let food = find(c, 0x0018_00E0, MAIN);
    let fw = body(c, food)?;
    Ok((cases, voice_case(c, &fw, food, 0, fw.len())?))
}

/// The field's voices: `ccVoiceRequest`'s groups, then the food's
/// (`ccVoicePgFood`), with each table's rows.
fn field_voices(c: &Ctx, food: bool) -> Read {
    use std::collections::BTreeSet;
    let (cases, pg_food) = field_voice_cases(c)?;
    let mut starts = BTreeSet::new();
    for v in cases.iter().map(|g| &g.1).chain([&pg_food]) {
        starts.extend([v.jp, v.en]);
        starts.extend(v.alt.as_ref().map(|a| a.table));
    }
    let rows = |a: u32| voice_table(c, a, starts.range(a + 1..).next().copied(), false);
    let table = |v: &VoiceCase| -> Read { Ok(Value::List(vec![Value::Int(v.file), rows(v.jp)?, rows(v.en)?])) };
    if food {
        return table(&pg_food);
    }
    let mut out = Vec::new();
    for (group, v) in &cases {
        let alt = match &v.alt {
            Some(a) => Value::List(vec![
                Value::Int(a.first),
                Value::Int(a.end),
                Value::Int(a.talk),
                Value::Int(a.add),
                Value::Int(a.file),
                rows(a.table)?,
            ]),
            None => Value::None,
        };
        out.push(Value::List(vec![Value::Int(*group), table(v)?, alt]));
    }
    Ok(Value::List(out))
}

/// `skillVoicePlay`'s tables (main 0x0017e350) by `charTbl` row, as
/// (file, table) for Japanese (`spcVoiceData`, `voiceData`) and English
/// (`spcVoiceDataE`, `voiceDataE`). Only the rows with a file are read: the
/// game gives up on a character without one before it looks at the rows
/// (Mutation's `voiceData` has three more than files).
fn skill_voice_tables(c: &Ctx) -> Result<Vec<Vec<(Value, u32)>>, String> {
    let sides = [(0x0030_7230, 0x0030_71E0), (0x0030_72D0, 0x0030_7280)];
    let mut out = Vec::new();
    for (files, rows) in sides {
        let (files, rows) = (find(c, files, MAIN), find(c, rows, MAIN));
        let mut side = Vec::new();
        for k in 0..64 {
            let name = c.p.u32(files + 4 * k)?;
            if name == 0 {
                break;
            }
            let t = c.p.u32(rows + 4 * k)?;
            if c.p.u32(t).is_err() {
                return Err(format!("voiceData row {k} is no table (0x{t:08x})"));
            }
            side.push((disc_path(c, name)?, t));
        }
        out.push(side);
    }
    if out[0].len() != out[1].len() {
        return Err(format!("{} Japanese and {} English skill files", out[0].len(), out[1].len()));
    }
    Ok(out)
}

/// The rows `skillVoicePlay` can reach from a table: ids 0-303 less a base
/// of 6 to 150, so 150 rows before it to 297 after.
const SKILL_ROWS_BEFORE: u32 = 150;
const SKILL_ROWS_AFTER: u32 = 298;

/// Where `skill_memory` starts: 150 rows before the lowest table.
fn skill_memory_start(t: &[Vec<(Value, u32)>]) -> u32 {
    t.iter().flatten().map(|r| r.1).min().unwrap_or(0) - 8 * SKILL_ROWS_BEFORE
}

/// Each character's file and where its table's row 0 is in `skill_memory`.
fn skill_voices(c: &Ctx) -> Read {
    let t = skill_voice_tables(c)?;
    let start = skill_memory_start(&t);
    let words = |(file, a): &(Value, u32)| match (a - start) % 8 {
        0 => Ok(Value::List(vec![file.clone(), Value::Int(i128::from((a - start) / 8))])),
        _ => Err(format!("skill voice table 0x{a:08x} is off the rows of 0x{start:08x}")),
    };
    let rows = t[0].iter().zip(&t[1]).map(|(j, e)| Ok(Value::List(vec![words(j)?, words(e)?])));
    Ok(Value::List(rows.collect::<Result<Vec<_>, String>>()?))
}

/// The memory `skillVoicePlay` reads its rows from, as `VOICE_DATA`: the
/// tables of both languages and what lies within a row's reach of them
/// (the next table, `itemTblE`, the message tables after them). A row
/// outside a character's table plays what lies there, as the game does.
fn skill_memory(c: &Ctx) -> Read {
    let t = skill_voice_tables(c)?;
    let start = skill_memory_start(&t);
    let end = t.iter().flatten().map(|r| r.1).max().unwrap_or(0) + 8 * SKILL_ROWS_AFTER;
    let row = ty("VOICE_DATA", vec![]);
    let rows = (start..end).step_by(8).map(|a| row.read(c, a)).collect::<Result<Vec<_>, _>>()?;
    Ok(Value::List(rows))
}

fn skill_words() -> Layout {
    strukt(
        "SkillWords",
        0,
        vec![("file", 0, cstr()), ("row0", 4, I32)],
        "One language's skill words for a character: the file, and where its table's row 0 is in `skill_memory`.",
    )
}

fn voice_table_layout() -> Layout {
    strukt(
        "VoiceTable",
        0,
        vec![
            ("file", 0, I32),
            ("jp", 4, array(ty("VOICE_DATA", vec![]), 0)),
            ("en", 8, array(ty("VOICE_DATA", vec![]), 0)),
        ],
        "A field voice table: the file it sets (`voiceFile`), its rows by message, and the English rows (`saveData.voice`).",
    )
}

fn voice() -> Group {
    group(
        "voice",
        "Voice",
        "The event voices (sndlib.cpp, main): the voice files and `ccEvVoiceRequest`'s tables.",
        vec![
            derived(
                "files",
                custom(file_names(0x0030_7330), array(ptr(cstr()), 0)),
                MAIN,
                "`evVoiceFile`: the voice files by `voiceFile`, as paths on the disc (`VOICE/EVVOL1.BIN`; the IOP opens `cdrom0:\\VOICE\\EVVOL1.BIN`).",
            ),
            derived(
                "files_e",
                custom(file_names(0x0030_7380), array(ptr(cstr()), 0)),
                MAIN,
                "`evVoiceFileE`: the same in `VOICE_E`; from Mutation on one more, `MIAE.BIN` (20).",
            ),
            derived(
                "events",
                custom(
                    Rc::new(event_voices),
                    array(
                        strukt(
                            "VoiceSection",
                            0,
                            vec![
                                ("main", 0, opt(voice_branch())),
                                ("side", 4, opt(voice_branch())),
                                ("parody", 8, opt(voice_branch())),
                            ],
                            "A hundred of events in `ccEvVoiceRequest`: the main events' table, the side events', Parody Mode's for the main events.",
                        ),
                        4,
                    ),
                ),
                MAIN,
                "`ccEvVoiceRequest`'s tables by `event / 100`, as the volume's code picks them.",
            ),
            derived(
                "field",
                custom(
                    Rc::new(|c| field_voices(c, false)),
                    array(
                        strukt(
                            "FieldVoice",
                            0,
                            vec![
                                ("group", 0, I32),
                                ("table", 4, voice_table_layout()),
                                (
                                    "alt",
                                    8,
                                    opt(strukt(
                                        "VoiceAlt",
                                        0,
                                        vec![
                                            ("first", 0, I32),
                                            ("end", 4, I32),
                                            ("talk", 8, I32),
                                            ("add", 12, I32),
                                            ("file", 16, I32),
                                            ("rows", 20, array(ty("VOICE_DATA", vec![]), 0)),
                                        ],
                                        "English messages `first..end` another file plays while `talkNum[talk]` is set: row `msg + add` of `rows` from file `file` (Mutation's `MIAE.BIN`).",
                                    )),
                                ),
                            ],
                            "One of `ccVoiceRequest`'s cases: its group (the event number below -1), its table, and an alternative for some English rows.",
                        ),
                        0,
                    ),
                ),
                GCMN,
                "`ccVoiceRequest`'s groups, the field's voices (the Grunties, the fountain, party members joining and leaving, their talk and presents, the dogs, Fidchell), as the volume's code builds them.",
            ),
            derived(
                "food",
                custom(Rc::new(|c| field_voices(c, true)), voice_table_layout()),
                GCMN,
                "`ccVoicePgFood`'s table: a Grunty food calling out.",
            ),
            derived(
                "skill",
                custom(
                    Rc::new(skill_voices),
                    array(
                        strukt(
                            "SkillVoice",
                            0,
                            vec![("jp", 0, skill_words()), ("en", 8, skill_words())],
                            "A character's skill words, Japanese (`spcVoiceData`, `voiceData`) and English (`spcVoiceDataE`, `voiceDataE`).",
                        ),
                        0,
                    ),
                ),
                GCMN,
                "`skillVoicePlay`'s tables by `charTbl` row, for the characters with a file.",
            ),
            derived(
                "skill_memory",
                custom(Rc::new(skill_memory), array(ty("VOICE_DATA", vec![]), 0)),
                GCMN,
                "The memory `skillVoicePlay` reads its rows from: every table and the rows within reach of them (ids 0-303), as `VOICE_DATA`.",
            ),
        ],
    )
}

/// The Ryu Books' texts (main `char *` globals into gcmn's data), each
/// carried to a later volume by its own symbol.
const BOOK_TEXTS: &[(&str, u32, &str)] = &[
    ("title1", 0x0037_8028, "`bookTitle1`"),
    ("area_msg", 0x0037_802C, "`bookAreaMsg`"),
    ("time_msg", 0x0037_8030, "`bookTimeMsg`"),
    ("cnt3", 0x0037_8034, "`bookCnt3`"),
    ("title2", 0x0037_8038, "`bookTitle2`"),
    ("magic_circle_msg", 0x0037_803C, "`bookMagicCircleMsg`"),
    ("magic_circle_all_open_msg", 0x0037_8040, "`bookMagicCircleAllOpenMsg`"),
    ("magic_circle_all_open_msg2", 0x0037_8044, "`bookMagicCircleAllOpenMsg2`"),
    ("title3", 0x0037_8048, "`bookTitle3`"),
    ("char_list", 0x0037_804C, "`bookCharList`"),
    ("trade_total", 0x0037_8050, "`bookTradeTotal`"),
    ("char_msg0", 0x0037_8054, "`bookCharMsg0`"),
    ("char_msg1", 0x0037_8058, "`bookCharMsg1`"),
    ("trade_msg0", 0x0037_805C, "`bookTradeMsg0`"),
    ("trade_msg1", 0x0037_8060, "`bookTradeMsg1`"),
    ("trade_msg2", 0x0037_8064, "`bookTradeMsg2`"),
    ("trade_msg3", 0x0037_8068, "`bookTradeMsg3`"),
    ("on_line", 0x0037_806C, "`bookOnLine`"),
    ("trade_item", 0x0037_8070, "`bookTradeItem`"),
    ("char_msg2", 0x0037_8074, "`bookCharMsg2`"),
    ("back_msg", 0x0037_8078, "`bookBackMsg`"),
    ("title4", 0x0037_807C, "`bookTitle4`"),
    ("enemy_list", 0x0037_8080, "`bookEnemyList`"),
    ("enemy_list_msg0", 0x0037_8084, "`bookEnemyListMsg0`"),
    ("enemy_list_msg1", 0x0037_8088, "`bookEnemyListMsg1`"),
    ("enemy_list_msg2", 0x0037_808C, "`bookEnemyListMsg2`"),
    ("enemy_slain_msg0", 0x0037_8090, "`bookEnemySlainMsg0`"),
    ("enemy_slain_msg1", 0x0037_8094, "`bookEnemySlainMsg1`"),
    ("enemy_name", 0x0037_8098, "`bookEnemyName`"),
    ("enemy_data", 0x0037_809C, "`bookEnemyData`"),
    ("enemy_level", 0x0037_80A0, "`bookEnemyLevel`"),
    ("enemy_hp", 0x0037_80A4, "`bookEnemyHP`"),
    ("enemy_sp", 0x0037_80A8, "`bookEnemySP`"),
    ("enemy_skill", 0x0037_80AC, "`bookEnemySKILL`"),
    ("enemy_item", 0x0037_80B0, "`bookEnemyItem`"),
    ("nothing", 0x0037_80B4, "`bookNothing`"),
    ("enemy_weak0", 0x0037_80B8, "`bookEnemyWeak0`"),
    ("enemy_weak1", 0x0037_80BC, "`bookEnemyWeak1`"),
    ("enemy_weak2", 0x0037_80C0, "`bookEnemyWeak2`"),
    ("enemy_encount_msg0", 0x0037_80C4, "`bookEnemyEncountMsg0`"),
    ("title5", 0x0037_80C8, "`bookTitle5`"),
    ("present_total", 0x0037_80CC, "`bookPresentTotal`"),
    ("present_total2", 0x0037_80D0, "`bookPresentTotal2`"),
    ("present2", 0x0037_80D4, "`bookPresent2`"),
    ("play_time", 0x0037_80D8, "`bookPlayTime`"),
    ("time", 0x0037_80DC, "`bookTime`"),
    ("friendly", 0x0037_80E0, "`bookFriendly`"),
    ("friendly1", 0x0037_80E4, "`bookFriendly1`"),
    ("gp", 0x0037_80E8, "`bookGP`"),
    ("title6", 0x0037_80EC, "`bookTitle6`"),
    ("box_open", 0x0037_80F0, "`bookBoxOpen`"),
    ("break_cnt", 0x0037_80F4, "`bookBreakCnt`"),
    ("idol_cnt", 0x0037_80F8, "`bookIdolCnt`"),
    ("title7", 0x0037_80FC, "`bookTitle7`"),
    ("fountain", 0x0037_8100, "`bookFountain`"),
    ("mush", 0x0037_8104, "`bookMush`"),
    ("granpa", 0x0037_8108, "`bookGranpa`"),
    ("symbl", 0x0037_810C, "`bookSymbl`"),
    ("encount_msg", 0x0037_8110, "`bookEncountMsg`"),
    ("encount_msg2", 0x0037_8114, "`bookEncountMsg2`"),
    ("title8", 0x0037_8118, "`bookTitle8`"),
    ("puchi_list", 0x0037_811C, "`bookPuchiList`"),
    ("puchi_food_list", 0x0037_8120, "`bookPuchiFoodList`"),
    ("puchi_name", 0x0037_8124, "`bookPuchiName`"),
    ("puchi_cnt", 0x0037_8128, "`bookPuchiCnt`"),
    ("puchi_food_total", 0x0037_812C, "`puchiFoodTotal`"),
    ("puchi_num", 0x0037_8130, "`bookPuchiNum`"),
    ("puchi_food_total2", 0x0037_8134, "`puchiFoodTotal2`"),
    ("puchi_food", 0x0037_8138, "`bookPuchiFood`"),
    ("puchi_food_num", 0x0037_813C, "`bookPuchiFoodNum`"),
    ("cnt0", 0x0037_8140, "`bookCnt0`"),
    ("cnt1", 0x0037_8144, "`bookCnt1`"),
    ("cnt2", 0x0037_8148, "`bookCnt2`"),
    ("hidden_name", 0x0037_814C, "`hiddenName`"),
    ("movie_notice", 0x0037_8160, "`bookMovieNotice`"),
];

/// The Ryu Books' messages (gcmn `ccMsgData`: `emode`, then `str[4]` at
/// +8; `str[1..3]` are the window's lines).
const BOOK_MSGS: &[(&str, u32, &str)] = &[
    ("counter_stop_help", 0x005D_29A0, "`BookCounterStopHelpMsg`"),
    ("help10", 0x005D_29C0, "`BookHelp10`"),
    ("help11", 0x005D_29E0, "`BookHelp11`"),
    ("help20", 0x005D_2A00, "`BookHelp20`"),
    ("help21", 0x005D_2A20, "`BookHelp21`"),
    ("help22", 0x005D_2A40, "`BookHelp22`"),
    ("help30", 0x005D_3FF0, "`BookHelp30`"),
    ("help40", 0x005D_4010, "`BookHelp40`"),
    ("help50", 0x005D_2A60, "`BookHelp50`"),
    ("help60", 0x005D_2A80, "`BookHelp60`"),
    ("help61", 0x005D_2AA0, "`BookHelp61`"),
    ("help62", 0x005D_2AC0, "`BookHelp62`"),
    ("help70", 0x005D_2AE0, "`BookHelp70`"),
    ("help71", 0x005D_2B00, "`BookHelp71`"),
    ("help72", 0x005D_2B20, "`BookHelp72`"),
    ("help80", 0x005D_2B40, "`BookHelp80`"),
    ("help81", 0x005D_2B60, "`BookHelp81`"),
    ("help82", 0x005D_2B80, "`BookHelp82`"),
    ("item_msg1", 0x005D_2BA0, "`BookItemMsg1`"),
    ("item_msg10", 0x005D_2BC0, "`BookItemMsg10`"),
    ("item_msg11", 0x005D_2BE0, "`BookItemMsg11`"),
    ("item_msg2", 0x005D_2C00, "`BookItemMsg2`"),
    ("item_msg20", 0x005D_2C20, "`BookItemMsg20`"),
    ("item_msg21", 0x005D_2C40, "`BookItemMsg21`"),
    ("item_msg22", 0x005D_2C60, "`BookItemMsg22`"),
    ("item_msg3", 0x005D_2C80, "`BookItemMsg3`"),
    ("item_msg30", 0x005D_2CA0, "`BookItemMsg30`"),
    ("item_msg31", 0x005D_2CC0, "`BookItemMsg31`"),
    ("item_msg30_comp", 0x005D_2CE0, "`BookItemMsg30_comp`"),
    ("item_msg31_comp", 0x005D_2D00, "`BookItemMsg31_comp`"),
    ("item_msg4", 0x005D_2D20, "`BookItemMsg4`"),
    ("item_msg40", 0x005D_2D40, "`BookItemMsg40`"),
    ("item_msg40_comp", 0x005D_2D60, "`BookItemMsg40_comp`"),
    ("item_msg5", 0x005D_2D80, "`BookItemMsg5`"),
    ("item_msg50", 0x005D_2DA0, "`BookItemMsg50`"),
    ("item_msg6", 0x005D_2DC0, "`BookItemMsg6`"),
    ("item_msg60", 0x005D_2DE0, "`BookItemMsg60`"),
    ("item_msg61", 0x005D_2E00, "`BookItemMsg61`"),
    ("item_msg62", 0x005D_2E20, "`BookItemMsg62`"),
    ("item_msg7", 0x005D_2E40, "`BookItemMsg7`"),
    ("item_msg70", 0x005D_2E60, "`BookItemMsg70`"),
    ("item_msg71", 0x005D_2E80, "`BookItemMsg71`"),
    ("item_msg72", 0x005D_2EA0, "`BookItemMsg72`"),
    ("item_msg8", 0x005D_2EC0, "`BookItemMsg8`"),
    ("item_msg80", 0x005D_2EE0, "`BookItemMsg80`"),
    ("item_msg81", 0x005D_2F00, "`BookItemMsg81`"),
    ("item_msg81_comp", 0x005D_2F20, "`BookItemMsg81_comp`"),
];

/// The rewards' thresholds by book and row (`BOOKITEM` tables, each to
/// its `cnt` -1 row).
const BOOK_ITEMS: &[(&str, u32, &str)] = &[
    ("items_10", 0x005D_3590, "`Book10Item`"),
    ("items_11", 0x005D_36A0, "`Book11Item`"),
    ("items_20", 0x005D_3730, "`Book20Item`"),
    ("items_21", 0x005D_3830, "`Book21Item`"),
    ("items_22", 0x005D_38A0, "`Book22Item`"),
    ("items_30", 0x005D_3910, "`Book30Item`"),
    ("items_31", 0x005D_3990, "`Book31Item`"),
    ("items_32", 0x005D_3970, "`Book32Item`"),
    ("items_33", 0x005D_3A20, "`Book33Item`"),
    ("items_40", 0x005D_3A40, "`Book40Item`"),
    ("items_41", 0x005D_3AA0, "`Book41Item`"),
    ("items_50", 0x005D_3AC0, "`Book50Item`"),
    ("items_60", 0x005D_3B40, "`Book60Item`"),
    ("items_61", 0x005D_3C00, "`Book61Item`"),
    ("items_62", 0x005D_3CC0, "`Book62Item`"),
    ("items_70", 0x005D_3D40, "`Book70Item`"),
    ("items_71", 0x005D_3DD0, "`Book71Item`"),
    ("items_72", 0x005D_3E60, "`Book72Item`"),
    ("items_80", 0x005D_3F20, "`Book80Item`"),
    ("items_81", 0x005D_3F40, "`Book81Item`"),
    ("items_82", 0x005D_3FD0, "`Book82Item`"),
];

/// `CheckBookLimit`'s tables: each book's counter caps, by volume.
const BOOK_LIMITS: &[(&str, u32, &str, usize)] = &[
    ("limits_1", 0x005D_2870, "`book01CountLimit`", 6),
    ("limits_2", 0x005D_2890, "`book02CountLimit`", 9),
    ("limits_3", 0x005D_28C0, "`book03CountLimit`", 6),
    ("limits_4", 0x005D_28D8, "`book04CountLimit`", 3),
    ("limits_5", 0x005D_28E8, "`book05CountLimit`", 3),
    ("limits_6", 0x005D_2900, "`book06CountLimit`", 9),
    ("limits_7", 0x005D_2930, "`book07CountLimit`", 9),
    ("limits_8", 0x005D_2958, "`book08CountLimit`", 3),
];

fn book_msg() -> Layout {
    strukt(
        "BookMsg",
        0x18,
        vec![("emode", 0, I32), ("str", 8, fixed(opt(cstr()), 4))],
        "A `ccMsgData`: its mode and four strings (the name, then the lines).",
    )
}

/// `ccThBook` (INF gcmn 0x0041a990): `streamNum = tsm->arg + N`, its
/// `lw $v0, 0x14(rs)` then `addiu rt, $v0, N`. Mutation and later add 118
/// (MUT gcmn 0x0042e510, OUT 0x00429f40, QUA 0x0031c850): their stream
/// tables put six streams before the covers.
fn cover_stream(c: &Ctx) -> Read {
    let w = body(c, find(c, 0x0041_A990, None))?;
    let arg = |x: u32| x >> 26 == 0x23 && (x >> 16) & 31 == 2 && x & 0xffff == 0x14;
    let add = |x: u32| x >> 26 == 0x09 && (x >> 21) & 31 == 2;
    w.windows(2)
        .find(|p| arg(p[0]) && add(p[1]))
        .map(|p| Value::Int(sext16(p[1])))
        .ok_or_else(|| "ccThBook adds nothing to its argument".into())
}

fn book() -> Group {
    let mut v = vec![
        e("ofs", 0x005D_3530, array(I32, 24), GCMN, "`BookOfs`: each book's window (y, width, height in cells)."),
        e(
            "item_list",
            0x005D_2F40,
            array(ty("BOOKITEMDATA", vec![]), 189),
            GCMN,
            "`bookItemList`: the rewards in the order given (`saveData.hyItem`): 1 a BGM, 2 a wallpaper, 4 a movie.",
        ),
        e("server", 0x005D_2970, array(opt(cstr()), 5), GCMN, "`bookServer`: the servers' names."),
        e(
            "count_stop",
            0x005D_2988,
            array_by(opt(cstr()), Rc::new(|c| if matches!(c.volume, Vol::Out | Vol::Qua) { 4 } else { 3 })),
            GCMN,
            "`countStopMsg`: by `volumeNum` - 1; three, four from Outbreak on (\"in Vol. 4.\").",
        ),
        e(
            "stream_cluts",
            0x005D_4030,
            array(ptr(cstr()), 8),
            GCMN,
            "The palette `ccThBook` puts on the cover's `MAT_clut` for each book past the first.",
        ),
        derived(
            "cover_stream",
            custom(Rc::new(cover_stream), I32),
            GCMN,
            "`ccThBook`'s stream for the first book's cover (the book's is this + the book): 112 on Infection, 118 from Mutation on.",
        ),
    ];
    for &(name, inf, doc) in BOOK_TEXTS {
        v.push(e(name, inf, ptr(cstr()), GCMN, doc));
    }
    // Mutation on: the `char *` after `bookNothing` (MUT 0x0038b058),
    // which book IV's sub-window shows for bosses 203-206's first skill.
    v.push(
        derived(
            "unknown",
            custom(Rc::new(|c| ptr(cstr()).read(c, find(c, 0x0037_80B4, None) + 4)), ptr(cstr())),
            GCMN,
            "Book IV's skill for enemies 203-206, from Mutation on (none on Infection).",
        )
        .absent(Vol::Inf, Value::Bytes(Vec::new())),
    );
    for &(name, inf, doc) in BOOK_MSGS {
        v.push(e(name, inf, book_msg(), GCMN, doc));
    }
    for &(name, inf, doc) in BOOK_ITEMS {
        let end: Until = Rc::new(|r| r.list()[0].int() == -1);
        v.push(e(name, inf, array_until(ty("BOOKITEM", vec![]), end), GCMN, doc));
    }
    // Three volumes' caps, four from Outbreak on (its tables add a row
    // for Quarantine).
    for &(name, inf, doc, n) in BOOK_LIMITS {
        let rows = Rc::new(move |c: &Ctx| if matches!(c.volume, Vol::Out | Vol::Qua) { n / 3 * 4 } else { n });
        v.push(e(name, inf, array_by(I32, rows), GCMN, doc));
    }
    group("book", "Book", "The Ryu Books (`BOOK`, gcmn book.cpp): the windows, the rewards and their texts.", v)
}

fn party_chat() -> Group {
    group(
        "party_chat",
        "PartyChat",
        "The party members' chat lines (`ccAI::ChatMessage*`'s tables, gcmn).",
        vec![
            e("standby_accept", 0x0065_1AE0, array(opt(cstr()), 19), GCMN, ""),
            e("accept", 0x0065_1B30, array(opt(cstr()), 57), GCMN, ""),
            e("use_all_attack_skill_accept", 0x0065_1C20, array(opt(cstr()), 57), GCMN, ""),
            e("use_physical_skill_accept", 0x0065_1D10, array(opt(cstr()), 19), GCMN, ""),
            e("use_magic_skill_accept", 0x0065_1D60, array(opt(cstr()), 19), GCMN, ""),
            e("not_use_skill_accept", 0x0065_1DB0, array(opt(cstr()), 19), GCMN, ""),
            e("buff_accept", 0x0065_1E00, array(opt(cstr()), 19), GCMN, ""),
            e("debuff_accept", 0x0065_1E50, array(opt(cstr()), 19), GCMN, ""),
            e("heal_start", 0x0065_1EA0, array(opt(cstr()), 19), GCMN, ""),
            e("cure_start", 0x0065_1EF0, array(opt(cstr()), 19), GCMN, ""),
            e("resurrect_start", 0x0065_1F40, array(opt(cstr()), 19), GCMN, ""),
            e("short_of_sp", 0x0065_1F90, array(opt(cstr()), 19), GCMN, ""),
            e("nothing_deny", 0x0065_1FE0, array(opt(cstr()), 19), GCMN, ""),
            e("no_battle_mode_deny", 0x0065_2030, array(opt(cstr()), 19), GCMN, ""),
            e("ghost_deny", 0x0065_2080, array(opt(cstr()), 19), GCMN, ""),
            e("condition_red", 0x0065_20D0, array(opt(cstr()), 19), GCMN, ""),
            e("thx_heal", 0x0065_2120, array(opt(cstr()), 19), GCMN, ""),
            e("thx_resurrect", 0x0065_2170, array(opt(cstr()), 19), GCMN, ""),
            e("thx_buff", 0x0065_21C0, array(opt(cstr()), 19), GCMN, ""),
            e("use_ocarina", 0x0065_2210, array(opt(cstr()), 19), GCMN, ""),
            e("no_ocarina_deny", 0x0065_2260, array(opt(cstr()), 19), GCMN, ""),
            e("disable_ocarina_deny", 0x0065_22B0, array(opt(cstr()), 19), GCMN, ""),
            e("entered_field", 0x0065_2300, array(opt(cstr()), 57), GCMN, ""),
            e("victory", 0x0065_23F0, array(opt(cstr()), 57), GCMN, ""),
            e("neutral", 0x0065_24E0, array(opt(cstr()), 57), GCMN, ""),
            e("heal_plz", 0x0065_25D0, array(opt(cstr()), 19), GCMN, ""),
            e("treatment_plz", 0x0065_2620, array(opt(cstr()), 19), GCMN, ""),
            e("resurrect_plz", 0x0065_2670, array(opt(cstr()), 19), GCMN, ""),
            e("bull", 0x0065_26C0, array(opt(cstr()), 57), GCMN, ""),
            e("dash_for_enemy", 0x0065_27B0, array(opt(cstr()), 57), GCMN, ""),
            e("timid", 0x0065_28A0, array(opt(cstr()), 57), GCMN, ""),
            e("small_damaged", 0x0065_2990, array(opt(cstr()), 57), GCMN, ""),
            e("damaged", 0x0065_2A80, array(opt(cstr()), 57), GCMN, ""),
            e("fatal_damaged", 0x0065_2B70, array(opt(cstr()), 57), GCMN, ""),
            e("attack_small_hit", 0x0065_2C60, array(opt(cstr()), 57), GCMN, ""),
            e("attack_hit", 0x0065_2D50, array(opt(cstr()), 57), GCMN, ""),
            e("attack_fatal_hit", 0x0065_2E40, array(opt(cstr()), 57), GCMN, ""),
            e("reencounter", 0x0065_2F30, array(opt(cstr()), 57), GCMN, ""),
            e("goto_weapon_shop", 0x0065_3020, array(opt(cstr()), 19), GCMN, ""),
            e("goto_goods_shop", 0x0065_3070, array(opt(cstr()), 19), GCMN, ""),
            e("goto_fairy_shop", 0x0065_30C0, array(opt(cstr()), 19), GCMN, ""),
            e("goto_magic_shop", 0x0065_3110, array(opt(cstr()), 19), GCMN, ""),
            e("goto_etc", 0x0065_3160, array(opt(cstr()), 19), GCMN, ""),
            e("byebye", 0x0065_31B0, array(opt(cstr()), 19), GCMN, ""),
            e("arrive_delta", 0x0065_3200, array(opt(cstr()), 19), GCMN, ""),
            e("arrive_lambda", 0x0065_3250, array(opt(cstr()), 19), GCMN, ""),
            e("arrive_sigma", 0x0065_32A0, array(opt(cstr()), 19), GCMN, ""),
            e("arrive_omega", 0x0065_32F0, array(opt(cstr()), 19), GCMN, ""),
            e("arrive_theta", 0x0065_3340, array(opt(cstr()), 19), GCMN, ""),
            e("goto_record_shop", 0x0065_3390, array(opt(cstr()), 19), GCMN, ""),
            e("change_equip_ok", 0x0065_33E0, array(opt(cstr()), 19), GCMN, ""),
            e("change_equip_not_skill", 0x0065_3430, array(opt(cstr()), 19), GCMN, ""),
            e("change_equip_not_etc", 0x0065_3480, array(opt(cstr()), 19), GCMN, ""),
            e("attribute_follow", 0x0065_34D0, array(opt(cstr()), 19), GCMN, ""),
            e("attribute_guard", 0x0065_3520, array(opt(cstr()), 19), GCMN, ""),
            e("attribute_critical", 0x0065_3570, array(opt(cstr()), 19), GCMN, ""),
            e("heal_plz_accept", 0x0065_35C0, array(opt(cstr()), 19), GCMN, ""),
            e("skill_accept", 0x0065_3610, array(opt(cstr()), 19), GCMN, ""),
            e("assign_accept", 0x0065_3660, array(opt(cstr()), 19), GCMN, ""),
            e("poison_curse", 0x0065_36B0, array(opt(cstr()), 19), GCMN, ""),
            e("paralysis_sleep", 0x0065_3700, array(opt(cstr()), 19), GCMN, ""),
            e("charm_confusion", 0x0065_3750, array(opt(cstr()), 19), GCMN, ""),
            e("wait_plz", 0x0065_37A0, array(opt(cstr()), 19), GCMN, ""),
            e("ghost_condition", 0x0065_37F0, array(opt(cstr()), 19), GCMN, ""),
            e("level_down", 0x0065_3840, array(opt(cstr()), 19), GCMN, ""),
            e("level_up", 0x0065_3890, array(opt(cstr()), 19), GCMN, ""),
            e("present_other_fellow", 0x0065_38E0, array(opt(cstr()), 19), GCMN, ""),
            e("dead_other_fellow", 0x0065_3930, array(opt(cstr()), 19), GCMN, ""),
            e("open_trap_box", 0x0065_3980, array(opt(cstr()), 19), GCMN, ""),
            e("quit_pucciguso", 0x0065_39D0, array(opt(cstr()), 19), GCMN, ""),
            e("grats_level_up", 0x0065_3A20, array(opt(cstr()), 19), GCMN, ""),
            e("condition_modify_enemy", 0x0065_3A70, array(opt(cstr()), 19), GCMN, ""),
            e("open_treasure_box", 0x0065_3AC0, array(opt(cstr()), 19), GCMN, ""),
            e("a_lot_of_enemy", 0x0065_3B10, array(opt(cstr()), 19), GCMN, ""),
            e("better_weapon", 0x0065_3B60, array(opt(cstr()), 3), GCMN, ""),
            e("worse_weapon", 0x0065_3B70, array(opt(cstr()), 3), GCMN, ""),
            e("fellow_is_high_level", 0x0065_3B80, array(opt(cstr()), 3), GCMN, ""),
            e("walking_talk", 0x0065_3B90, array(opt(cstr()), 57), GCMN, ""),
            remark(
                "physical_tolerance",
                0x0059_0790,
                1,
                "From Mutation on, `ChatMessageAttack`'s line for a foe immune to physical attacks.",
            ),
            remark("magic_tolerance", 0x0059_0790, 2, "The same for magic."),
            remark(
                "use_item",
                0x0058_9410,
                2,
                "From Mutation on, `ccAI::UseItem`'s line as a member uses an item (`#a` its name).",
            ),
            remark("use_last_item", 0x0058_9410, 1, "The same when it was the member's last."),
            remark(
                "only_buff",
                0x0058_5C70,
                2,
                "From Mutation on, `ChatCommandBuffPlz`'s line from a member with no other buff to cast.",
            ),
            remark("only_debuff", 0x0058_57C0, 1, "The same for `ChatCommandDeBuffPlz` and debuffs."),
            e("attribute_str", 0x0065_4030, array(ptr(cstr()), 8), GCMN, "`attributeStrTbl`: the elements' names."),
            e("attribute_str2", 0x0065_4050, array(ptr(cstr()), 8), GCMN, "`attributeStrTbl2`"),
        ],
    )
}

fn talk() -> Group {
    group(
        "talk",
        "Talk",
        "The records the talk pages reach through the addresses the game keeps (`npcTbl`, the save, `spcMsgTbl`).",
        vec![
            derived(
                "records",
                custom(Rc::new(talk::records), array(msg_records(), 0)),
                GCMN,
                "The record arrays, by address.",
            ),
            derived(
                "pointers",
                custom(Rc::new(talk::pointers), array(msg_pointers(), 0)),
                GCMN,
                "The pointer tables, by address.",
            ),
        ],
    )
}

thread_local! {
    static GROUPS: RefCell<Option<Rc<Vec<Group>>>> = const { RefCell::new(None) };
}

/// Every group, in the order the tables module lists them; built once.
pub fn groups() -> Rc<Vec<Group>> {
    if let Some(g) = GROUPS.with(|g| g.borrow().clone()) {
        return g;
    }
    let g = Rc::new(vec![
        title(),
        fonts(),
        loaddisp(),
        newgame(),
        dtmenu(),
        desktop(),
        mail(),
        kanji(),
        staffroll(),
        nameentry(),
        toppage(),
        game(),
        fieldui(),
        battle(),
        combat(),
        stream(),
        effect(),
        world(),
        party_chat(),
        talk(),
        voice(),
        book(),
    ]);
    GROUPS.with(|x| *x.borrow_mut() = Some(g.clone()));
    g
}
