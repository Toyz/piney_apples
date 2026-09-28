//! The tables the game's sound code reads (`piney_data::sound::Tables`,
//! `docs/engine/sound.md`), and setbl.cpp's rows the animation notes'
//! sounds read (`piney_audio::setbl`). Numbers only: no titles or
//! comments. The later volumes' executables carry Infection's names with
//! Infection's sizes, so their tables are sized by their own layout
//! ([`extent`]).

use super::Out;
use crate::program::Program;
use crate::volume::{Vol, ctx};

const FIELD_LOAD: [&str; 9] = ["typeA", "typeC", "typeE", "typeG", "typeI", "typeK", "typeM", "typeO", "piroshi"];
const FIELD_VOL: [&str; 8] = [
    "sqVoltypeA",
    "sqVoltypeC",
    "sqVoltypeG",
    "sqVoltypeI",
    "sqVoltypeK",
    "sqVoltypeM",
    "sqVoltypeO",
    "sqVoltypePiroshi",
];
const FIELD_PLAY: [&str; 9] = [
    "typeAplayType",
    "typeCplayType",
    "typeEplayType",
    "typeGplayType",
    "typeIplayType",
    "typeKplayType",
    "typeMplayType",
    "typeOplayType",
    "piroshiPlayType",
];
/// (SQ_LOAD table, SQTBL table, play type table), in `Tables`' order:
/// dungeon, town, title, desktop, toppage, event, stream.
const CONTEXTS: [(&str, &str, Option<&str>); 7] = [
    ("sqDataDungeon", "sqVolTblDungeon", Some("dungeonPlayType")),
    ("sqDataTown", "sqVolTblTown", None),
    ("sqDataTitle", "sqVolTblTitle", None),
    ("sqDataDesktop", "sqVolTblDesktop", None),
    ("sqDataToppage", "sqVolTblToppage", None),
    ("sqDataEvent", "sqVolTblEvent", Some("eventPlayType")),
    ("sqDataStream", "sqVolTblStream", None),
];

fn sym<'a>(p: &'a Program, name: &str) -> Result<&'a crate::elf::Symbol, String> {
    p.symbol_named(name).ok_or_else(|| format!("no symbol {name}"))
}

/// How many `row`-byte rows a later volume's table has: where the next
/// global is the same one as on Infection, Infection's size plus however
/// much the gap to it grew, rounded up to a row and at most the gap; where
/// Infection's neighbour sits elsewhere, up to the next global; where it
/// has no name here, Infection's size, at most the gap.
fn extent(p: &Program, re: &Program, name: &str, row: u32) -> Result<u32, String> {
    let (s, r) = (sym(p, name)?, sym(re, name)?);
    let n = p.following(s.value).ok_or("no global after")?;
    let rn = re.following(r.value).ok_or("no global after")?;
    let (gap, ref_gap) = (i64::from(n.value - s.value), i64::from(rn.value - r.value));
    let row = i64::from(row);
    let rows = if n.name == rn.name {
        let grown = i64::from(r.size) + gap - ref_gap;
        (-((-grown).div_euclid(row))).min(gap.div_euclid(row))
    } else if p.symbol_named(&rn.name).is_some() {
        gap.div_euclid(row)
    } else {
        i64::from(r.size).min(gap).div_euclid(row)
    };
    Ok(rows.max(0) as u32)
}

/// A table's rows as raw bytes, each `row` long.
fn rows(p: &Program, re: Option<&Program>, name: &str, row: u32) -> Result<Vec<Vec<u8>>, String> {
    let s = sym(p, name)?;
    let count = match re {
        Some(re) => extent(p, re, name, row)?,
        None => s.size / row,
    };
    (0..count).map(|i| p.read(s.value + row * i, row as usize)).collect()
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(b[at..at + 2].try_into().unwrap())
}

/// An `SQ_LOAD` table: {ofs, hdSize, sqSize1..3, bdSize}.
fn load(out: &mut Out, rows: &[Vec<u8>]) {
    out.count(rows.len());
    for r in rows {
        for k in 0..6 {
            out.i32(i32_at(r, 4 * k));
        }
    }
}

/// An `SQTBL` table: per row three {int midiPort, int hdPort, u16 vol} of
/// 12 bytes.
fn vol(out: &mut Out, rows: &[Vec<u8>]) {
    out.count(rows.len());
    for r in rows {
        for k in 0..3 {
            out.i32(i32_at(r, 12 * k));
            out.i32(i32_at(r, 12 * k + 4));
            out.0.extend_from_slice(&u16_at(r, 12 * k + 8).to_le_bytes());
        }
    }
}

/// A play type table: an s8 per row.
fn play(out: &mut Out, rows: &[Vec<u8>]) {
    out.count(rows.len());
    for r in rows {
        out.u8(r[0]);
    }
}

/// The volume's `Tables`.
pub fn bytes(v: Vol) -> Result<Vec<u8>, String> {
    let main = ctx(v, Some("gcmn"));
    let desk = ctx(v, Some("desktop"));
    let later = v != Vol::Inf;
    let (rmain, rdesk) = (ctx(Vol::Inf, Some("gcmn")), ctx(Vol::Inf, Some("desktop")));
    let (m, d) = (&*main.p, &*desk.p);
    let (rm, rd) = (later.then_some(&*rmain.p), later.then_some(&*rdesk.p));
    let mut out = Out::default();
    // commse.
    for r in rows(m, rm, "commseTbl", 4)?.iter().take(3) {
        out.u32(u32::from_le_bytes(r[..4].try_into().unwrap()));
    }
    // The field's contexts, by the tables sqDataField, sqVolTblField and
    // playTypeTbl point at.
    let pointed = |name: &str| -> Result<Vec<String>, String> {
        rows(m, rm, name, 4)?
            .iter()
            .map(|r| {
                let w = u32::from_le_bytes(r[..4].try_into().unwrap());
                m.name_at(w, 0).ok_or_else(|| format!("{name}: nothing at 0x{w:08x}"))
            })
            .collect()
    };
    let (fl, fv, fp) = (pointed("sqDataField")?, pointed("sqVolTblField")?, pointed("playTypeTbl")?);
    if fl.len() != 12 || fv.len() != 12 || fp.len() != 12 {
        return Err(format!(
            "the field's tables have {}, {}, {} rows, the port's type 12",
            fl.len(),
            fv.len(),
            fp.len()
        ));
    }
    for k in 0..12 {
        for (known, name) in [(&FIELD_LOAD[..], &fl[k]), (&FIELD_VOL[..], &fv[k]), (&FIELD_PLAY[..], &fp[k])] {
            if !known.contains(&name.as_str()) {
                return Err(format!("the field's tables point at {name}"));
            }
        }
        load(&mut out, &rows(m, rm, &fl[k], 24)?);
        vol(&mut out, &rows(m, rm, &fv[k], 36)?);
        play(&mut out, &rows(m, rm, &fp[k], 1)?);
    }
    for (lo, vo, pl) in CONTEXTS {
        load(&mut out, &rows(m, rm, lo, 24)?);
        vol(&mut out, &rows(m, rm, vo, 36)?);
        match pl {
            Some(pl) => play(&mut out, &rows(m, rm, pl, 1)?),
            None => out.count(0),
        }
    }
    // se: seData, {s8 progNo, port, ch, note, velocity, dummy; s16 decay}.
    let se = rows(m, rm, "seData", 8)?;
    out.count(se.len());
    for r in &se {
        out.0.extend_from_slice(&r[..8]);
    }
    // wave: the desktop's jukebox, {category, fieldType, bgNum, NO}, to the
    // NULL row.
    let wave: Vec<Vec<u8>> = rows(d, rd, "Wave", 24)?.into_iter().take_while(|r| i32_at(r, 4) >= 0).collect();
    out.count(wave.len());
    for r in &wave {
        for at in [4, 8, 12, 20] {
            out.i32(i32_at(r, at));
        }
    }
    // bgm: bgmWavTbl {ofs, siz} with bgmParam {mode; volR, volL}, the
    // tracks that have a size.
    let (wav, param) = (rows(m, rm, "bgmWavTbl", 8)?, rows(m, rm, "bgmParam", 8)?);
    let tracks: Vec<_> = wav.iter().zip(&param).filter(|(w, _)| i32_at(w, 4) > 0).collect();
    out.count(tracks.len());
    for (w, pa) in tracks {
        out.i32(i32_at(w, 0));
        out.i32(i32_at(w, 4));
        out.i32(i32_at(pa, 0));
        out.0.extend_from_slice(&u16_at(pa, 4).to_le_bytes());
        out.0.extend_from_slice(&u16_at(pa, 6).to_le_bytes());
    }
    Ok(out.0)
}

/// setbl.cpp's .data from `spc0SeData` to the end of `inuSeData` as 8-byte
/// `SE_NT` rows {int code; char note, velocity; short dummy} (padding and
/// the pointer tables included), `spcSeTbl` and `enemySeTbl` as the row
/// each pointer names (None for NULL), and `inuSeData`'s row.
pub fn setbl(v: Vol) -> Result<Option<Vec<u8>>, String> {
    let c = ctx(v, Some("gcmn"));
    let p = &*c.p;
    // Outbreak's and Quarantine's carried names miss `spc0SeData`; it is
    // `spcSeTbl[0]` on all four (setbl.cpp's first array).
    let start = match p.symbol_named("spc0SeData") {
        Some(first) => first.value,
        None => p.u32(sym(p, "spcSeTbl")?.value)?,
    };
    let last = sym(p, "inuSeData")?;
    let end = last.value + last.size;
    if (end - start) % 8 != 0 || start % 8 != 0 {
        return Err("setbl.cpp's data is not in 8-byte rows".into());
    }
    let mut out = Out::default();
    out.u32(start);
    out.count(((end - start) / 8) as usize);
    for a in (start..end).step_by(8) {
        let b = p.read(a, 8)?;
        out.i32(i32_at(&b, 0));
        out.u8(b[4]);
        out.u8(b[5]);
    }
    // Each pointer table runs to the next object in the data (padding read
    // as NULL): Outbreak's and Quarantine's `spcSeTbl` holds 21 tables, but
    // the carried size is Infection's 19.
    let tables = [sym(p, "spcSeTbl")?, sym(p, "enemySeTbl")?];
    let mut starts = vec![last.value];
    for s in &tables {
        starts.push(s.value);
        for i in 0..s.size / 4 {
            starts.push(p.u32(s.value + 4 * i)?);
        }
    }
    for (name, s) in ["spcSeTbl", "enemySeTbl"].into_iter().zip(&tables) {
        let next = starts.iter().copied().filter(|&a| a > s.value).min().unwrap_or(end);
        let n = (next - s.value) / 4;
        out.count(n as usize);
        for i in 0..n {
            let w = p.u32(s.value + 4 * i)?;
            if w == 0 {
                out.some(false);
                continue;
            }
            if !(start..end).contains(&w) || (w - start) % 8 != 0 {
                return Err(format!("{name}[{i}] = 0x{w:08x} is not a row of setbl.cpp"));
            }
            out.some(true);
            out.0.extend_from_slice(&(((w - start) / 8) as u16).to_le_bytes());
        }
    }
    out.0.extend_from_slice(&(((last.value - start) / 8) as u16).to_le_bytes());
    Ok(Some(out.0))
}
