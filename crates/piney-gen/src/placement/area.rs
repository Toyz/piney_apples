//! The area generator's tables (`piney_data::area::AreaTables`,
//! `docs/engine/area-words.md`): the twelve `WORDPARAM` tables `word_a1` ..
//! `word_c4` in the order `GetWordParamPtr` searches them, `dungeonData`,
//! `eventAreaInfo` (`eventAreaInfoNum` rows), the substitute
//! `EVENTAREA_INFO` records `ccGetEventAreaInfo` returns from Mutation on,
//! and `volumeNum`. Found by name (the later volumes through the carried
//! symbols).

use super::{Out, latin1};
use crate::program::Program;
use crate::volume::Vol;

fn sym(p: &Program, name: &str) -> Result<(u32, u32), String> {
    p.symbol_named(name).map(|s| (s.value, s.size)).ok_or_else(|| format!("no symbol {name}"))
}

fn words4(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// A text a table points at (`None` for NULL), one char per byte.
fn text(p: &Program, va: u32) -> Result<Option<String>, String> {
    if va == 0 {
        return Ok(None);
    }
    Ok(Some(latin1(&p.cstr(va, 64)?)))
}

/// One `EVENTAREA_INFO` (0x54 bytes) as `EventAreaInfo`.
fn event(p: &Program, va: u32, out: &mut Out) -> Result<(), String> {
    let b = p.read(va, 0x54)?;
    out.i32(words4(&b, 0));
    for k in 0..3 {
        match text(p, words4(&b, 4 + 4 * k) as u32)? {
            Some(t) => {
                out.some(true);
                out.str(&t);
            }
            None => out.some(false),
        }
    }
    // server, circle, enemy, item, model, flag, type, bgnum, dungeonNum,
    // then protect[8].
    for k in 0..17 {
        out.i32(words4(&b, 0x10 + 4 * k));
    }
    Ok(())
}

/// The addresses of the records `ccGetEventAreaInfo` returns in place of
/// the table's, in the order its code builds them: each `li v1, code`
/// followed by `lui` / `addiu v0` making the record's address.
fn substitutes(p: &Program) -> Result<Vec<u32>, String> {
    let (va, size) = sym(p, "ccGetEventAreaInfo__Fi")?;
    let code = p.read(va, (size / 4 * 4) as usize)?;
    let mut out = Vec::new();
    let mut pending = false;
    let mut hi = [0u32; 32];
    for w in code.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)) {
        let (op, rs, rt, imm) = (w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xffff);
        if op == 0x09 && rs == 0 && rt == 3 {
            pending = true;
        } else if op == 0x0f {
            hi[rt as usize] = imm << 16;
        } else if op == 0x09 && rt == 2 && pending && hi[rs as usize] != 0 {
            out.push(hi[rs as usize].wrapping_add(imm as i16 as i32 as u32));
            pending = false;
        }
    }
    Ok(out)
}

/// The volume's `AreaTables`.
pub fn bytes(v: Vol) -> Result<Vec<u8>, String> {
    let ctx = crate::volume::ctx(v, None);
    let p = &ctx.p;
    let mut out = Out::default();
    // words: every table's rows, in search order.
    let mut rows = Vec::new();
    for group in 1..=4u8 {
        for (slot, s) in ["a", "b", "c"].iter().enumerate() {
            let (va, size) = sym(p, &format!("word_{s}{group}"))?;
            for i in 0..size / 0x30 {
                rows.push((slot, group, p.read(va + 0x30 * i, 0x30)?));
            }
        }
    }
    out.count(rows.len());
    for (slot, group, b) in rows {
        out.str(&text(p, words4(&b, 0) as u32)?.unwrap_or_default());
        out.u32(slot as u32);
        out.u8(group);
        // id, pri, then the nine attrs.
        for k in 1..12 {
            out.i32(words4(&b, 4 * k));
        }
    }
    // dungeon_data: eleven (level_max, room_max).
    let (va, size) = sym(p, "dungeonData")?;
    if size / 8 != 11 {
        return Err(format!("dungeonData has {} rows, the port's type 11", size / 8));
    }
    let b = p.read(va, 88)?;
    for k in 0..22 {
        out.i32(words4(&b, 4 * k));
    }
    // events.
    let num = p.u32(sym(p, "eventAreaInfoNum")?.0)?;
    let (base, _) = sym(p, "eventAreaInfo")?;
    out.count(num as usize);
    for i in 0..num {
        event(p, base + 0x54 * i, &mut out)?;
    }
    // substitutes.
    let subs = substitutes(p)?;
    out.count(subs.len());
    for va in subs {
        event(p, va, &mut out)?;
    }
    // volume.
    out.i32(p.u32(sym(p, "volumeNum")?.0)? as i32);
    Ok(out.0)
}
