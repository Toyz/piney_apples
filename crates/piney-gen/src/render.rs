//! Each group as a Rust module, and the modules every group shares.

use std::path::Path;
use std::rc::Rc;

use crate::layout::{Emitter, Layout, StructDef, Value};
use crate::locate::extract;
use crate::manifest::Group;
use crate::text;
use crate::volume::{VOLUMES, Vol};

fn walk_layouts<'a>(l: &'a Layout, out: &mut Vec<&'a Layout>) {
    out.push(l);
    match l {
        Layout::Fixed { inner, .. }
        | Layout::CountedPtr { inner, .. }
        | Layout::Ptr(inner)
        | Layout::Opt(inner)
        | Layout::Array { inner, .. }
        | Layout::Keyed { inner, .. } => walk_layouts(inner, out),
        Layout::Custom { layout, .. } => walk_layouts(layout, out),
        Layout::Struct(s) => s.fields.iter().for_each(|f| walk_layouts(&f.2, out)),
        _ => {}
    }
}

/// Where a field starts: a fixed offset, or a loop's offset plus one.
#[derive(Clone)]
enum At {
    Lit(u32),
    Var(String, u32),
}

impl At {
    fn plus(&self, n: u32) -> String {
        match self {
            At::Lit(o) => format!("0x{:x}", o + n),
            At::Var(k, 0) if n == 0 => k.clone(),
            At::Var(k, o) => format!("{k} + 0x{:x}", o + n),
        }
    }
}

/// Rust statements laying `expr` (of `l`) at `at` of `b`; `depth` names
/// the loops of arrays within arrays apart.
fn write_field(expr: &str, at: &At, l: &Layout, depth: usize) -> Vec<String> {
    match l {
        Layout::Num(_) | Layout::Float => {
            let (a, b) = (at.plus(0), at.plus(l.size().unwrap()));
            vec![format!("b[{a}..{b}].copy_from_slice(&{expr}.to_le_bytes());")]
        }
        Layout::Bits { num, shift, width } => {
            let (a, b) = (at.plus(0), at.plus(l.size().unwrap()));
            let t = num.rust_name();
            vec![
                "{".into(),
                format!("    let w = {t}::from_le_bytes(b[{a}..{b}].try_into().unwrap());"),
                format!("    let m: {t} = ((1u64 << {width}) - 1) as {t};"),
                format!("    let w = (w & !(m << {shift})) | (({expr} & m) << {shift});"),
                format!("    b[{a}..{b}].copy_from_slice(&w.to_le_bytes());"),
                "}".into(),
            ]
        }
        Layout::Struct(_) => vec![format!("{expr}.write(&mut b[{}..]);", at.plus(0))],
        Layout::Fixed { inner, .. } => {
            let suffix = if depth == 0 { String::new() } else { depth.to_string() };
            let (i, v, k) = (format!("i{suffix}"), format!("v{suffix}"), format!("k{suffix}"));
            let w = write_field(&v, &At::Var(k.clone(), 0), inner, depth + 1);
            if w.is_empty() {
                return Vec::new();
            }
            let mut lines = vec![
                format!("for ({i}, {v}) in {expr}.iter().enumerate() {{"),
                format!("    let {k} = {} + {i} * 0x{:x};", at.plus(0), inner.size().unwrap()),
            ];
            lines.extend(w.into_iter().map(|l| format!("    {l}")));
            lines.push("}".into());
            lines
        }
        Layout::FixedText(n) => {
            let (a, b) = (at.plus(0), at.plus(*n));
            vec![
                "{".into(),
                format!("    let t = crate::tables::sjis::encode({expr});"),
                format!("    let n = t.len().min(0x{n:x});"),
                format!("    b[{a}..{a} + n].copy_from_slice(&t[..n]);"),
                format!("    b[{a} + n..{b}].fill(0);"),
                "}".into(),
            ]
        }
        // A pointer (text or an address): the game's code sets it.
        _ => Vec::new(),
    }
}

pub fn definition(s: &StructDef) -> String {
    let mut all = Vec::new();
    s.fields.iter().for_each(|f| walk_layouts(&f.2, &mut all));
    let eq = if all.iter().any(|l| matches!(l, Layout::Float)) { "PartialEq" } else { "PartialEq, Eq" };
    let doc = if s.doc.is_empty() { s.name.clone() } else { s.doc.clone() };
    let mut lines =
        vec![format!("/// {doc}"), format!("#[derive(Clone, Copy, Debug, {eq})]"), format!("pub struct {} {{", s.name)];
    for (n, _, l) in &s.fields {
        if !matches!(l, Layout::Omit) {
            lines.push(format!("    pub {n}: {},", l.rust()));
        }
    }
    lines.push("}".into());
    // Read back from a built group's file (`piney_data::store`).
    lines.push(String::new());
    lines.push(format!("impl crate::store::Load for {} {{", s.name));
    lines.push("    fn load(r: &mut crate::store::Reader) -> Self {".into());
    lines.push(format!("        {} {{", s.name));
    for (n, _, l) in &s.fields {
        if !matches!(l, Layout::Omit) {
            lines.push(format!("            {n}: crate::store::Load::load(r),"));
        }
    }
    lines.extend(["        }".into(), "    }".into(), "}".into()]);
    let writes: Vec<String> = s
        .fields
        .iter()
        .filter(|f| !matches!(f.2, Layout::Omit))
        .flat_map(|(n, off, l)| write_field(&format!("self.{n}"), &At::Lit(*off), l, 0))
        .map(|w| format!("        {w}"))
        .collect();
    // A row of only pointers is never laid into memory: no `write`.
    if s.memory && !writes.is_empty() {
        lines.extend([
            String::new(),
            format!("impl {} {{", s.name),
            "    /// Its size in the game's memory.".into(),
            format!("    pub const SIZE: usize = 0x{:x};", s.size),
            String::new(),
            "    /// Laid into the game's memory at its fields' offsets (a pointer".into(),
            "    /// field, which the game's code sets itself, is left alone).".into(),
            "    pub fn write(&self, b: &mut [u8]) {".into(),
        ]);
        lines.extend(writes);
        lines.extend(["    }".into(), "}".into()]);
    }
    lines.join("\n")
}

/// The Struct layouts a group's entries use, each once, inner first.
pub fn structs_of(g: &Group) -> Vec<Rc<StructDef>> {
    fn walk(l: &Layout, out: &mut Vec<Rc<StructDef>>, g: &Group) {
        match l {
            Layout::Fixed { inner, .. }
            | Layout::CountedPtr { inner, .. }
            | Layout::Ptr(inner)
            | Layout::Opt(inner)
            | Layout::Array { inner, .. }
            | Layout::Keyed { inner, .. } => walk(inner, out, g),
            Layout::Custom { layout, .. } => walk(layout, out, g),
            _ => {}
        }
        if let Layout::Struct(s) = l {
            s.fields.iter().for_each(|f| walk(&f.2, out, g));
            match out.iter().find(|x| x.name == s.name) {
                None => out.push(s.clone()),
                Some(x) if definition(x) != definition(s) => {
                    crate::die(&format!("{}: two layouts named {}: give their uses one override", g.name, s.name))
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    g.entries.iter().for_each(|e| walk(&e.layout, &mut out, g));
    out
}

fn capitalize(t: &str) -> String {
    let mut c = t.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
}

/// The group's module.
pub fn render(g: &Group) -> Result<String, String> {
    if crate::data::in_build(g) {
        return render_built(g);
    }
    let values: Vec<Rc<Vec<Value>>> = VOLUMES.iter().map(|&v| extract(g, v)).collect();
    let n = g.entries.len();
    let alike: Vec<bool> = (0..n).map(|i| values.iter().all(|vv| vv[i] == values[0][i])).collect();
    let keyed: Vec<usize> = (0..n).filter(|&i| matches!(g.entries[i].layout, Layout::Keyed { .. })).collect();
    for &i in &keyed {
        if !alike[i] {
            return Err(format!(
                "{}.{}: differs between the volumes; a per-volume enum method is not made yet",
                g.name, g.entries[i].name
            ));
        }
    }
    let own: Vec<usize> = (0..n).filter(|&i| !alike[i]).collect();
    let mut lines: Vec<String> = vec![
        "// Generated by piney-gen from the four discs' executables - do not edit.".into(),
        String::new(),
        format!("//! {}", g.doc),
        String::new(),
        "use crate::volume::Volume;".into(),
    ];
    let mut used: Vec<&str> = keyed
        .iter()
        .map(|&i| match &g.entries[i].layout {
            Layout::Keyed { enum_, .. } => *enum_,
            _ => unreachable!(),
        })
        .collect();
    used.sort();
    used.dedup();
    if used.len() > 1 {
        lines.push(format!("use crate::world::{{{}}};", used.join(", ")));
    } else if let Some(u) = used.first() {
        lines.push(format!("use crate::world::{u};"));
    }
    let structs = structs_of(g);
    let mut typed: Vec<&str> = structs.iter().filter(|s| s.memory).map(|s| s.name.as_str()).collect();
    typed.sort();
    if typed.len() > 1 {
        lines.push(format!("pub use super::types::{{{}}};", typed.join(", ")));
    } else if let Some(t) = typed.first() {
        lines.push(format!("pub use super::types::{t};"));
    }
    lines.push(String::new());
    for s in &structs {
        if !s.memory {
            lines.push(definition(s));
            lines.push(String::new());
        }
    }
    // The volumes that hold the same own fields share one static.
    let mut distinct: Vec<(Vec<Vol>, Vec<&Value>)> = Vec::new();
    for (k, &v) in VOLUMES.iter().enumerate() {
        let mine: Vec<&Value> = own.iter().map(|&i| &values[k][i]).collect();
        match distinct.iter_mut().find(|d| d.1 == mine) {
            Some(d) => d.0.push(v),
            None => distinct.push((vec![v], mine)),
        }
    }
    let name_of = |vs: &[Vol]| vs.iter().map(|v| v.tag()).collect::<Vec<_>>().join("_");
    let mut em = Emitter::default();
    let (mut shared, mut impls, mut bodies) = (Vec::new(), Vec::new(), Vec::new());
    for counting in [true, false] {
        em.counting = counting;
        shared.clear();
        impls.clear();
        bodies.clear();
        for &i in &keyed {
            let e = &g.entries[i];
            let Layout::Keyed { enum_, variants, inner } = &e.layout else { unreachable!() };
            let mut body = vec![
                if e.doc.is_empty() { String::new() } else { format!("    /// {}", e.doc) },
                format!("    pub fn {}(self) -> {} {{", e.name, inner.rust()),
                "        match self {".into(),
            ];
            for (var, v) in variants.iter().zip(values[0][i].list()) {
                let (_, t) = inner.emit(v, &mut em, &format!("{}_{var}", e.name))?;
                body.push(format!("            {enum_}::{var} => {t},"));
            }
            body.extend(["        }".into(), "    }".into()]);
            impls.push((*enum_, body));
        }
        for i in 0..n {
            let e = &g.entries[i];
            if matches!(e.layout, Layout::Keyed { .. }) || !alike[i] {
                continue;
            }
            let ty = e.layout.rust().replace("&'static ", "&");
            let (_, t) = e.layout.emit(&values[0][i], &mut em, e.name)?;
            let mut s = String::new();
            if !e.doc.is_empty() {
                s.push_str(&format!("/// {}\n", e.doc));
            }
            s.push_str(&format!("pub static {}: {ty} = {t};", e.name.to_uppercase()));
            shared.push(s);
        }
        for (vs, mine) in &distinct {
            if own.is_empty() {
                bodies.push(vec![format!("static {}: {} = {} {{}};", name_of(vs), g.rust, g.rust), String::new()]);
                continue;
            }
            let mut body = vec![format!("static {}: {} = {} {{", name_of(vs), g.rust, g.rust)];
            for (&i, v) in own.iter().zip(mine) {
                let e = &g.entries[i];
                let (r, t) = e.layout.emit(v, &mut em, e.name)?;
                let (_, t) = em.node(&e.layout.rust(), r, t, e.name, true)?;
                body.push(format!("    {}: {t},", e.name));
            }
            body.extend(["};".into(), String::new()]);
            bodies.push(body);
        }
    }
    for c in &em.consts {
        lines.push(c.clone());
        lines.push(String::new());
    }
    for c in &shared {
        lines.push(c.clone());
        lines.push(String::new());
    }
    let mut enums: Vec<&str> = Vec::new();
    for (en, _) in &impls {
        if !enums.contains(en) {
            enums.push(en);
        }
    }
    for en in enums {
        lines.push(format!("impl {en} {{"));
        for (e2, body) in &impls {
            if *e2 == en {
                lines.extend(body.iter().filter(|l| !l.is_empty()).cloned());
            }
        }
        lines.extend(["}".into(), String::new()]);
    }
    lines.push(format!("/// What differs between the volumes in the manifest's `{}` group;", g.name));
    lines.push("/// what they hold alike is the module's own items.".into());
    lines.push("#[derive(Debug)]".into());
    lines.push(format!("pub struct {} {{", g.rust));
    for &i in &own {
        let e = &g.entries[i];
        if !e.doc.is_empty() {
            lines.push(format!("    /// {}", e.doc));
        }
        lines.push(format!("    pub {}: {},", e.name, e.layout.rust()));
    }
    lines.extend(["}".into(), String::new()]);
    for b in &bodies {
        lines.extend(b.iter().cloned());
    }
    // Every field as a method, whether the volume's own or shared.
    lines.push(format!("impl {} {{", g.rust));
    for (e, &same) in g.entries.iter().zip(&alike) {
        if matches!(e.layout, Layout::Keyed { .. }) {
            continue;
        }
        let value = if same { e.name.to_uppercase() } else { format!("self.{}", e.name) };
        lines.push(format!("    pub fn {}(&self) -> {} {{", e.name, e.layout.rust()));
        lines.push(format!("        {value}"));
        lines.push("    }".into());
    }
    lines.extend(["}".into(), String::new()]);
    lines.push(format!("/// The volume's {}.", g.rust));
    lines.push(format!("pub fn of(v: Volume) -> &'static {} {{", g.rust));
    lines.push("    match v {".into());
    for (vs, _) in &distinct {
        let pats: Vec<String> = vs.iter().map(|v| format!("Volume::{}", capitalize(v.tag()))).collect();
        lines.push(format!("        {} => &{},", pats.join(" | "), name_of(vs)));
    }
    lines.extend(["    }".into(), "}".into()]);
    Ok(lines.join("\n").trim_end_matches('\n').to_string() + "\n")
}

/// A group whose values are in the build (`crate::data`): its types, a
/// struct of every entry that `Load` reads from the group's file, each
/// entry as a method, `of(volume)` from `piney_data::store`, and for the
/// entries alike on every volume their statics as before (read on first
/// use) and the keyed entries' enum methods.
fn render_built(g: &Group) -> Result<String, String> {
    let values: Vec<Rc<Vec<Value>>> = VOLUMES.iter().map(|&v| extract(g, v)).collect();
    let n = g.entries.len();
    let alike: Vec<bool> = (0..n).map(|i| values.iter().all(|vv| vv[i] == values[0][i])).collect();
    // The data run registers the functions and characters the values use.
    for &v in &VOLUMES {
        crate::data::group_bytes(g, v)?;
    }
    let mut lines: Vec<String> = vec![
        "// Generated by piney-gen - do not edit. The values are in the build".into(),
        format!("// (`PINEY/TABLES/{}.bin`, plans/build-data.md).", g.name),
        String::new(),
        format!("//! {}", g.doc),
        String::new(),
        "use crate::volume::Volume;".into(),
    ];
    let mut used: Vec<&str> = g
        .entries
        .iter()
        .filter_map(|e| match &e.layout {
            Layout::Keyed { enum_, .. } => Some(*enum_),
            _ => None,
        })
        .collect();
    used.sort();
    used.dedup();
    if !used.is_empty() {
        lines.push(format!("use crate::world::{{{}}};", used.join(", ")));
    }
    let structs = structs_of(g);
    let mut typed: Vec<&str> = structs.iter().filter(|s| s.memory).map(|s| s.name.as_str()).collect();
    typed.sort();
    if !typed.is_empty() {
        lines.push(format!("pub use super::types::{{{}}};", typed.join(", ")));
    }
    lines.push(String::new());
    for s in &structs {
        if !s.memory {
            lines.push(definition(s));
            lines.push(String::new());
        }
    }
    let ty = |e: &crate::manifest::Entry| match &e.layout {
        Layout::Keyed { inner, .. } => format!("&'static [{}]", inner.rust()),
        l => l.rust(),
    };
    lines.push(format!("/// The `{}` group's values for a volume, from the build.", g.name));
    lines.push("#[derive(Debug)]".into());
    lines.push(format!("pub struct {} {{", g.rust));
    for e in &g.entries {
        if !e.doc.is_empty() {
            lines.push(format!("    /// {}", e.doc));
        }
        lines.push(format!("    pub {}: {},", e.name, ty(e)));
    }
    lines.extend(["}".into(), String::new()]);
    lines.push(format!("impl crate::store::Load for {} {{", g.rust));
    lines.push("    fn load(r: &mut crate::store::Reader) -> Self {".into());
    lines.push(format!("        {} {{", g.rust));
    for e in &g.entries {
        lines.push(format!("            {}: crate::store::Load::load(r),", e.name));
    }
    lines.extend(["        }".into(), "    }".into(), "}".into(), String::new()]);
    lines.push(format!("impl {} {{", g.rust));
    for e in &g.entries {
        if matches!(e.layout, Layout::Keyed { .. }) {
            continue;
        }
        lines.push(format!("    pub fn {}(&self) -> {} {{", e.name, e.layout.rust()));
        lines.push(format!("        self.{}", e.name));
        lines.push("    }".into());
    }
    lines.extend(["}".into(), String::new()]);
    for (e, &same) in g.entries.iter().zip(&alike) {
        if !same || matches!(e.layout, Layout::Keyed { .. }) {
            continue;
        }
        if !e.doc.is_empty() {
            lines.push(format!("/// {}", e.doc));
        }
        lines.push(format!(
            "pub static {}: std::sync::LazyLock<{}> = std::sync::LazyLock::new(|| shared().{});",
            e.name.to_uppercase(),
            e.layout.rust(),
            e.name
        ));
        lines.push(String::new());
    }
    for e in &g.entries {
        let Layout::Keyed { enum_, variants, inner } = &e.layout else { continue };
        lines.push(format!("impl {enum_} {{"));
        if !e.doc.is_empty() {
            lines.push(format!("    /// {}", e.doc));
        }
        lines.push(format!("    pub fn {}(self) -> {} {{", e.name, inner.rust()));
        lines.push("        let k = match self {".into());
        for (k, var) in variants.iter().enumerate() {
            lines.push(format!("            {enum_}::{var} => {k},"));
        }
        lines.push("        };".into());
        lines.push(format!("        shared().{}[k]", e.name));
        lines.extend(["    }".into(), "}".into(), String::new()]);
    }
    lines.push(format!("/// The volume's {} (read once a run; kept here per volume).", g.rust));
    lines.push(format!("pub fn of(v: Volume) -> &'static {} {{", g.rust));
    lines.push(format!("    static READ: [std::sync::OnceLock<&'static {}>; 4] =", g.rust));
    lines.push("        [const { std::sync::OnceLock::new() }; 4];".into());
    lines.push(format!("    READ[v as usize].get_or_init(|| crate::store::group(v, \"{}\"))", g.name));
    lines.extend(["}".into(), String::new()]);
    lines.push("/// The values every volume has alike, from whichever volume's can be had.".into());
    lines.push("#[allow(dead_code)]".into());
    lines.push(format!("fn shared() -> &'static {} {{", g.rust));
    lines.push(format!("    static READ: std::sync::OnceLock<&'static {}> = std::sync::OnceLock::new();", g.rust));
    lines.push(format!("    READ.get_or_init(|| crate::store::shared(\"{}\"))", g.name));
    lines.push("}".into());
    Ok(lines.join("\n").trim_end_matches('\n').to_string() + "\n")
}

/// `types.rs`: every struct built from Infection's DWARF, written once.
pub fn types_rs(groups: &[Group]) -> Result<String, String> {
    let mut out: std::collections::BTreeMap<String, Rc<StructDef>> = Default::default();
    for g in groups {
        for s in structs_of(g) {
            if !s.memory {
                continue;
            }
            if let Some(x) = out.get(&s.name)
                && definition(x) != definition(&s)
            {
                return Err(format!("two layouts named {} ({}): give their uses one override", s.name, g.name));
            }
            out.entry(s.name.clone()).or_insert(s);
        }
    }
    let mut lines = vec![
        "// Generated by piney-gen from Infection's DWARF - do not edit.".to_string(),
        String::new(),
        "//! The game's own structs, as the generated tables hold them: each built".into(),
        "//! from Infection's DWARF once, whichever tables use it.".into(),
        String::new(),
    ];
    for s in out.values() {
        lines.push(definition(s));
        lines.push(String::new());
    }
    // The closed sets of functions the tables name (every group rendered).
    crate::layout::FUNCS.with(|f| {
        for (e, variants) in f.borrow().iter() {
            lines.push(format!("/// Which function a `{e}` pointer names, by the function's name."));
            lines.push("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]".into());
            lines.push(format!("pub enum {e} {{"));
            for (v, symbol) in variants {
                lines.push(format!("    /// `{symbol}`"));
                lines.push(format!("    {v},"));
            }
            lines.extend(["}".into(), String::new()]);
            // A built group's file names the variant.
            lines.push(format!("impl crate::store::Load for {e} {{"));
            lines.push("    fn load(r: &mut crate::store::Reader) -> Self {".into());
            lines.push("        match <&str as crate::store::Load>::load(r) {".into());
            for v in variants.keys() {
                lines.push(format!("            \"{v}\" => {e}::{v},"));
            }
            lines.push(format!("            other => panic!(\"no {e} named {{other}}\"),"));
            lines.extend(["        }".into(), "    }".into(), "}".into(), String::new()]);
        }
    });
    Ok(lines.join("\n").trim_end_matches('\n').to_string() + "\n")
}

pub fn mod_rs(groups: &[Group]) -> String {
    let mut lines = vec![
        "// Generated by piney-gen - do not edit.".to_string(),
        String::new(),
        "//! The engine's tables out of each volume's executable, generated by".into(),
        "//! `piney-gen` from its manifest (`crates/piney-gen/src/manifest.rs`;".into(),
        "//! `plans/volumes.md`, \"The generator\"). Each module is one group:".into(),
        "//! `of(volume)` gives that volume's.".into(),
        String::new(),
        "// A float near a constant (1.414214) is the game's number, not the".into(),
        "// constant; an offset keeps its form (`0x0 + i * 0x4`) as the layout has it.".into(),
        "#![allow(clippy::approx_constant, clippy::identity_op)]".into(),
        String::new(),
    ];
    for g in groups {
        lines.push(format!("pub mod {};", g.name));
    }
    lines.push("pub mod sjis;".into());
    lines.push("pub mod types;".into());
    lines.join("\n") + "\n"
}

/// `sjis.rs`: each non-ASCII character the generated text uses, with its
/// Shift-JIS code, and `encode`.
pub fn sjis_rs() -> Result<String, String> {
    let used: Vec<char> = text::USED.with(|u| u.borrow().iter().copied().collect());
    let mut lines: Vec<String> = [
        "// Generated by piney-gen - do not edit.",
        "",
        "//! The generated tables hold the game's text as Rust strings; the game",
        "//! draws Shift-JIS. `encode` gives a string's bytes back: ASCII as it",
        "//! is, every other character by the table (each one the tables use).",
        "",
        "/// Where the game's own glyphs sit: U+F0000 + their Shift-JIS code.",
        "const GLYPH_BASE: u32 = 0xf0000;",
        "",
        "/// (character, Shift-JIS code), sorted by character.",
        "static CODES: &[(char, u16)] = &[",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for c in used {
        let b = text::encode_char(c).ok_or_else(|| format!("{c:?} has no code"))?;
        let code = b.iter().fold(0u32, |a, &x| a << 8 | u32::from(x));
        lines.push(if code > 0xff {
            format!("    ('{c}', 0x{code:04x}),")
        } else {
            format!("    ('{c}', 0x{code:02x}),")
        });
    }
    lines.extend(
        [
            "];",
            "",
            "/// The game's Shift-JIS bytes of a generated string.",
            "pub fn encode(s: &str) -> Vec<u8> {",
            "    let mut out = Vec::with_capacity(s.len());",
            "    for c in s.chars() {",
            "        if c.is_ascii() {",
            "            out.push(c as u8);",
            "            continue;",
            "        }",
            "        // A glyph of the game's font Unicode does not assign: its code.",
            "        if let Some(code) = (c as u32).checked_sub(GLYPH_BASE) {",
            "            out.extend_from_slice(&(code as u16).to_be_bytes());",
            "            continue;",
            "        }",
            "        let code = CODES.binary_search_by_key(&c, |&(k, _)| k).map(|i| CODES[i].1);",
            "        match code {",
            "            Ok(v) if v > 0xff => out.extend_from_slice(&v.to_be_bytes()),",
            "            Ok(v) => out.push(v as u8),",
            "            Err(_) => out.push(b'?'),",
            "        }",
            "    }",
            "    out",
            "}",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    Ok(lines.join("\n") + "\n")
}

/// The text as `cargo fmt` leaves it (the workspace's rustfmt.toml).
pub fn rustfmt(text: &str) -> Result<String, String> {
    use std::io::Write;
    let cfg = crate::volume::root().join("rustfmt.toml");
    let mut child = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--config-path"])
        .arg(&cfg)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("rustfmt: {e}"))?;
    let mut stdin = child.stdin.take().unwrap();
    let input = text.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = child.wait_with_output().map_err(|e| format!("rustfmt: {e}"))?;
    writer.join().map_err(|_| "rustfmt: the writer panicked")?.map_err(|e| format!("rustfmt: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rustfmt: {}",
            String::from_utf8_lossy(&out.stderr).chars().take(2000).collect::<String>()
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("rustfmt: {e}"))
}

/// Write `text` to `out`, or with `check` report whether it differs.
pub fn write_or_check(text: &str, out: &Path, check: bool) -> Result<bool, String> {
    if check {
        let same = std::fs::read_to_string(out).map(|t| t == text).unwrap_or(false);
        if same {
            println!("{} is current", out.display());
        } else {
            println!("{} is out of date; run without --check", out.display());
        }
        return Ok(same);
    }
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(true)
}
