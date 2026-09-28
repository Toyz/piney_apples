//! Where towns and event areas put their static models
//! (`piney_data::statics`, `docs/engine/statics.md`): every `MODELTABLE`
//! (`STATIC_MODEL_INFO`, 0x14 bytes) and `OBJTABLE` (`STATIC_OBJ_INFO`,
//! 0x18 bytes) object of gcmn.prg, each with the `DATA.BIN` members that
//! hold all it names, and the scene files each `ROOTTOWN` / `EVENTAREA`
//! constructor asks `ccStream::GetCCSAdrs` for.

use super::{Out, latin1};
use crate::program::Program;
use crate::volume::Vol;

const MODEL_ROW: u32 = 0x14;
const OBJ_ROW: u32 = 0x18;

/// A model table's row: (type, model, postype, posname, clip).
type ModelRow = (i32, String, i32, Option<String>, u32);
/// An object table's row: (type, clump, anime, postype, posname, clip).
type ObjRow = (i32, Option<String>, Option<String>, i32, Option<String>, u32);

fn words(b: &[u8]) -> Vec<u32> {
    b.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect()
}

fn text(p: &Program, va: u32) -> Result<Option<String>, String> {
    if va == 0 {
        return Ok(None);
    }
    Ok(Some(latin1(&p.cstr(va, 64)?)))
}

/// gcmn's object symbols whose name holds `word`, sorted by name.
fn tables_named<'a>(p: &'a Program, word: &str) -> Vec<&'a crate::elf::Symbol> {
    let gcmn = p.overlay_index.get("gcmn").copied();
    let mut out: Vec<_> = p
        .symbols
        .iter()
        .filter(|s| s.kind == 1 && s.size != 0 && s.name.contains(word) && Some(s.shndx as usize) == gcmn)
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
    out
}

fn model_tables(p: &Program) -> Result<Vec<(String, u32, Vec<ModelRow>)>, String> {
    let mut out = Vec::new();
    for s in tables_named(p, "MODELTABLE") {
        let mut rows = Vec::new();
        for k in 0..s.size / MODEL_ROW {
            let w = words(&p.read(s.value + MODEL_ROW * k, MODEL_ROW as usize)?);
            if w[1] == 0 {
                continue;
            }
            let pos = if w[2] as i32 != 0 { text(p, w[3])? } else { None };
            rows.push((w[0] as i32, text(p, w[1])?.unwrap_or_default(), w[2] as i32, pos, w[4]));
        }
        out.push((s.name.clone(), s.value, rows));
    }
    Ok(out)
}

fn obj_tables(p: &Program) -> Result<Vec<(String, u32, Vec<ObjRow>)>, String> {
    let mut out = Vec::new();
    for s in tables_named(p, "OBJTABLE") {
        let mut rows = Vec::new();
        for k in 0..s.size / OBJ_ROW {
            let w = words(&p.read(s.value + OBJ_ROW * k, OBJ_ROW as usize)?);
            if w[1] == 0 && w[2] == 0 {
                continue;
            }
            let pos = if w[3] as i32 != 0 { text(p, w[4])? } else { None };
            rows.push((w[0] as i32, text(p, w[1])?, text(p, w[2])?, w[3] as i32, pos, w[5]));
        }
        out.push((s.name.clone(), s.value, rows));
    }
    Ok(out)
}

/// The class a constructor's symbol names (`__ct__10ROOTTOWN01Fv`), if it
/// is a `ROOTTOWN` or `EVENTAREA` one.
fn constructed(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("__ct__")?;
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let class = rest.strip_suffix("Fv")?;
    let ok = (class.starts_with("ROOTTOWN") || class.starts_with("EVENTAREA"))
        && class.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    ok.then_some(class)
}

/// [(class, [file])]: the scene files each constructor asks for, in order:
/// the text whose address `$a0` holds at each `jal GetCCSAdrs` (its delay
/// slot, then back up to five).
fn scene_sets(p: &Program) -> Result<Vec<(String, Vec<String>)>, String> {
    let target = p.symbol_named("GetCCSAdrs__8ccStreamFPCc").ok_or("no GetCCSAdrs")?.value;
    let mut out = Vec::new();
    for f in p.functions() {
        let Some(class) = constructed(&f.name) else { continue };
        let w = words(&p.read(f.value, (f.size / 4 * 4) as usize)?);
        let built = crate::xfer::addresses(&w, p.gp);
        let mut files: Vec<String> = Vec::new();
        for (i, &x) in w.iter().enumerate() {
            if x >> 26 != 3 || (x & 0x03ff_ffff) << 2 != target & 0x0fff_ffff {
                continue;
            }
            // The delay slot, then back five.
            for j in (i.saturating_sub(5)..=(i + 1).min(w.len() - 1)).rev() {
                if let Some(&va) = built.get(&j)
                    && (w[j] >> 16) & 31 == 4
                {
                    let name = latin1(&p.cstr(va, 64)?);
                    if !files.contains(&name) {
                        files.push(name);
                    }
                    break;
                }
            }
        }
        if !files.is_empty() {
            out.push((class.to_string(), files));
        }
    }
    out.sort();
    Ok(out)
}

/// {table: [member stems holding every object it names]}, for `named`
/// ([(table, [object])]).
fn scenes_for(named: &[(String, Vec<String>)], v: Vol) -> Result<Vec<Vec<String>>, String> {
    let data = crate::source::read(v, "DATA/DATA.BIN")?;
    let archive = piney_data::archive::Archive::new(data).map_err(|e| e.to_string())?;
    let mut found = vec![Vec::new(); named.len()];
    for m in archive.members() {
        let Ok(bytes) = archive.inflate(m) else { continue };
        let Ok(c) = piney_data::ccs::Ccs::parse(bytes) else { continue };
        let names: std::collections::HashSet<&str> = c.objects.iter().map(|o| o.name.as_str()).collect();
        let stem = m.name.to_lowercase();
        let stem = stem.rsplit_once('.').map_or(stem.as_str(), |(s, _)| s).to_string();
        for (k, (_, wanted)) in named.iter().enumerate() {
            if !wanted.is_empty() && wanted.iter().all(|w| names.contains(w.as_str())) {
                found[k].push(stem.clone());
            }
        }
    }
    Ok(found)
}

/// `DrawPass` from a row's type.
fn pass(kind: i32, where_: &str) -> Result<u8, String> {
    if (0..=3).contains(&kind) { Ok(kind as u8) } else { Err(format!("{where_}: unknown draw pass {kind}")) }
}

/// `Position` from a row's postype and posname.
fn position(out: &mut Out, postype: i32, posname: &Option<String>, where_: &str) -> Result<(), String> {
    match (postype, posname) {
        (0, _) => out.u8(0),
        (1 | 2, Some(n)) if !n.is_empty() => {
            out.u8(postype as u8);
            out.str(n);
        }
        _ => return Err(format!("{where_}: postype {postype} with posname {posname:?}")),
    }
    Ok(())
}

fn opt(out: &mut Out, s: &Option<String>) {
    match s {
        Some(s) => {
            out.some(true);
            out.str(s);
        }
        None => out.some(false),
    }
}

/// The volume's statics: model tables, object tables, scene sets.
pub fn bytes(v: Vol) -> Result<Vec<u8>, String> {
    let ctx = crate::volume::ctx(v, Some("gcmn"));
    let p = &ctx.p;
    let models = model_tables(p)?;
    let objs = obj_tables(p)?;
    let mut named: Vec<(String, Vec<String>)> =
        models.iter().map(|(n, _, rows)| (n.clone(), rows.iter().map(|r| r.1.clone()).collect())).collect();
    named.extend(
        objs.iter()
            .map(|(n, _, rows)| (n.clone(), rows.iter().flat_map(|r| [r.1.clone(), r.2.clone()]).flatten().collect())),
    );
    let scenes = scenes_for(&named, v)?;
    let mut out = Out::default();
    out.count(models.len());
    for (k, (name, va, rows)) in models.iter().enumerate() {
        out.str(name);
        out.u32(*va);
        out.count(scenes[k].len());
        scenes[k].iter().for_each(|s| out.str(s));
        out.count(rows.len());
        for (kind, model, postype, posname, clip) in rows {
            let where_ = format!("{name} {model}");
            out.u8(pass(*kind, &where_)?);
            out.str(model);
            position(&mut out, *postype, posname, &where_)?;
            out.u32(*clip);
        }
    }
    out.count(objs.len());
    for (k, (name, va, rows)) in objs.iter().enumerate() {
        let k = models.len() + k;
        out.str(name);
        out.u32(*va);
        out.count(scenes[k].len());
        scenes[k].iter().for_each(|s| out.str(s));
        out.count(rows.len());
        for (kind, clump, anime, postype, posname, clip) in rows {
            let where_ = format!("{name} {}", clump.as_deref().or(anime.as_deref()).unwrap_or(""));
            out.u8(pass(*kind, &where_)?);
            opt(&mut out, clump);
            opt(&mut out, anime);
            position(&mut out, *postype, posname, &where_)?;
            out.u32(*clip);
        }
    }
    let sets = scene_sets(p)?;
    out.count(sets.len());
    for (class, files) in &sets {
        out.str(class);
        out.count(files.len());
        files.iter().for_each(|f| out.str(f));
    }
    Ok(out.0)
}
