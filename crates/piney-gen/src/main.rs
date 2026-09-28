//! The table generator: each group of the manifest read out of the four discs'
//! executables (and their overlays) and written as a typed Rust module under
//! `crates/piney-data/src/tables` (`plans/volumes.md`, "The generator").
//! Commands: `gen [--group G] [--check]` writes or checks the tables, `show
//! GROUP [--volume V]` prints a group, `carry [--volume V] [--check]` makes a
//! later volume's carry, `syms [--volume V] [--check]` carries the symbols.

use std::process::ExitCode;

use piney_gen::volume::{self, Vol};
use piney_gen::{carry, die, dtype, dwarf, locate, manifest, render, syms};

fn usage() -> ! {
    die(
        "usage: piney-gen gen [--group G] [--check] | show GROUP [--volume INF|MUT|OUT|QUA] | carry|syms [--volume MUT|OUT|QUA] [--check]",
    )
}

/// A built group's file written, or (with `check`) compared: whether it
/// was current.
fn write_bytes_or_check(bytes: &[u8], file: &std::path::Path, check: bool) -> bool {
    let same = std::fs::read(file).ok().as_deref() == Some(bytes);
    if check {
        if !same {
            eprintln!("{} is out of date", file.display());
        }
        return same;
    }
    if !same {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).unwrap_or_else(|e| die(&format!("{}: {e}", dir.display())));
        }
        std::fs::write(file, bytes).unwrap_or_else(|e| die(&format!("{}: {e}", file.display())));
    }
    true
}

fn formatted(text: Result<String, String>) -> String {
    text.and_then(|t| render::rustfmt(&t)).unwrap_or_else(|e| die(&e))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let groups = manifest::groups();
    match args.first().map(String::as_str) {
        Some("show") => {
            let Some(name) = args.get(1) else { usage() };
            let v = match args.get(2).map(String::as_str) {
                None => Vol::Inf,
                Some("--volume") => args.get(3).and_then(|t| Vol::from_tag(t)).unwrap_or_else(|| usage()),
                Some(_) => usage(),
            };
            let g = groups.iter().find(|g| g.name == name).unwrap_or_else(|| die(&format!("no group {name}")));
            for (e, v) in g.entries.iter().zip(locate::extract(g, v).iter()) {
                let line = format!("{}: {v:?}", e.name);
                println!("{}", line.chars().take(200).collect::<String>());
            }
            ExitCode::SUCCESS
        }
        Some("gen") => {
            let (mut only, mut check) = (None, false);
            let mut rest = args[1..].iter();
            while let Some(a) = rest.next() {
                match a.as_str() {
                    "--group" => only = Some(rest.next().unwrap_or_else(|| usage()).clone()),
                    "--check" => check = true,
                    _ => usage(),
                }
            }
            let out = volume::root().join("crates/piney-data/src/tables");
            let mut current = true;
            let mut data_current = true;
            let mut put = |text: String, file: &str| {
                current &= render::write_or_check(&text, &out.join(file), check).unwrap_or_else(|e| die(&e));
            };
            for g in groups.iter() {
                if only.as_deref().is_some_and(|o| o != g.name) {
                    continue;
                }
                put(formatted(render::render(g)), &format!("{}.rs", g.name));
                // A built group's values: the tools' and checks' copy.
                if piney_gen::data::in_build(g) {
                    for v in volume::VOLUMES {
                        let bytes = piney_gen::data::group_bytes(g, v).unwrap_or_else(|e| die(&e));
                        let file = volume::work_tables(v).join(format!("{}.bin", g.name));
                        data_current &= write_bytes_or_check(&bytes, &file, check);
                    }
                }
            }
            // The placement groups' values (piney-data's own types).
            for (name, bytes_of) in piney_gen::placement::GROUPS {
                if only.as_deref().is_some_and(|o| o != *name) {
                    continue;
                }
                for v in volume::VOLUMES {
                    let made = bytes_of(v).unwrap_or_else(|e| die(&format!("{name} ({}): {e}", v.tag())));
                    let Some(bytes) = made else { continue };
                    let file = volume::work_tables(v).join(format!("{name}.bin"));
                    data_current &= write_bytes_or_check(&bytes, &file, check);
                }
            }
            if only.is_none() {
                put(formatted(Ok(render::mod_rs(&groups))), "mod.rs");
                put(formatted(render::types_rs(&groups)), "types.rs");
                // Every group rendered: the characters they use are all known.
                put(formatted(render::sjis_rs()), "sjis.rs");
            }
            if current && data_current { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        Some("global") => {
            // Infection's DWARF: each global's type, and its struct's members.
            let dw = dtype::dwarf();
            for name in &args[1..] {
                if dw.global_named(name).is_empty()
                    && let Some((ms, size)) = dw.members(name)
                {
                    println!("{name} (0x{size:x} bytes)");
                    for (off, m, ty) in ms {
                        println!("    +0x{off:02x} {m}: {ty}");
                    }
                }
                for (addr, t) in dw.global_named(name) {
                    println!("{name} 0x{addr:08x}: {}", dw.describe(&t));
                    let mut inner = &t;
                    while let dwarf::Ty::Array(_, x) | dwarf::Ty::Ptr(x) | dwarf::Ty::Cv(x) = inner {
                        inner = x;
                    }
                    if let dwarf::Ty::Named(i) = inner
                        && let Some(n) = dw.die(*i).name()
                        && let Some((ms, size)) = dw.members(n)
                    {
                        println!("  {n} (0x{size:x} bytes)");
                        for (off, m, ty) in ms {
                            println!("    +0x{off:02x} {m}: {ty}");
                        }
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Some(cmd @ ("carry" | "syms")) => {
            let (mut only, mut check) = (None, false);
            let mut rest = args[1..].iter();
            while let Some(a) = rest.next() {
                match a.as_str() {
                    "--volume" => only = Some(rest.next().and_then(|t| Vol::from_tag(t)).unwrap_or_else(|| usage())),
                    "--check" => check = true,
                    _ => usage(),
                }
            }
            let mut current = true;
            for v in volume::VOLUMES.into_iter().filter(|&v| v != Vol::Inf && only.is_none_or(|o| o == v)) {
                current &= if cmd == "carry" { carry::write(v, check) } else { syms::write(v, check) };
            }
            if current { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        _ => usage(),
    }
}
