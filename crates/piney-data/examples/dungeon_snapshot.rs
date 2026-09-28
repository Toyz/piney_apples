//! Prints what `piney_data::dungeon` makes, one JSON line per request, for
//! `tools/test_dungeon_rs.py` to compare with `tools/dungeon.py`
//! (`dungeon_snapshot DATA.BIN [--plain-fixed EV=T,...] < requests`). The
//! requests are `random`, `story`, `type`, `dummies`, `fog` and `texclut`, with
//! the arguments the test sends; `type plain` asks a copy of Infection's rule
//! with Mutation's save-flag rule and the `--plain-fixed` areas.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{BufRead, Write as _};

use piney_data::archive::Archive;
use piney_data::dungeon::{self, Dummies, INF, Params, SaveFlag, TexClut, TypeRule};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: dungeon_snapshot DATA.BIN [--plain-fixed EV=T,...] < requests");
        std::process::exit(2);
    };
    let archive = Archive::new(std::fs::read(path).expect("DATA.BIN")).expect("archive");
    let plain_fixed: Vec<(i32, u8)> = match args.iter().position(|a| a == "--plain-fixed") {
        Some(i) => args[i + 1]
            .split(',')
            .map(|kv| {
                let (k, v) = kv.split_once('=').expect("EV=T");
                (k.parse().unwrap(), v.parse().unwrap())
            })
            .collect(),
        None => Vec::new(),
    };
    let plain = TypeRule {
        plain_fixed: Box::leak(plain_fixed.into_boxed_slice()),
        save_flag: SaveFlag::RandomPath { offset: 0x5ec8, bit: 62 },
        ..INF.types
    };
    let mut dummies: HashMap<u8, Dummies> = HashMap::new();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| -> i64 { w[i].parse().unwrap() };
        let json = match w.first().copied() {
            Some("random") => {
                let p = Params {
                    seed: n(1) as u32,
                    dtype: n(2) as u8,
                    level_max: n(3) as u32,
                    room_max: n(4) as u32,
                    server: n(5) as u32,
                    volume: n(6) as u32,
                    word_a: n(7) as u32,
                    field_type: n(8) as u32,
                    code: n(9) as u32,
                };
                let d = dummies
                    .entry(p.dtype)
                    .or_insert_with(|| Dummies::load(&archive, &INF, p.dtype, 0).expect("dungeon CCS"));
                random(&p, d)
            }
            Some("story") => story(n(1) as i32, n(2) as i32),
            Some("type") => {
                let rule = if w[1] == "plain" { &plain } else { &INF.types };
                let t = rule.types(n(2) as u32, n(3) as i32, n(4) as i32, n(5) != 0);
                format!("[{}, {}]", t[0], t[1])
            }
            Some("dummies") => counts(&archive, w[1]),
            Some("fog") => {
                let tc = TexClut { clut_type: n(2) as u8, tex_type: n(3) as u8 };
                let (table, index) = INF.fog_table(n(1) as u8, tc, n(4) as u32);
                let row = &table.rows[index];
                format!(
                    "{{\"table\": {}, \"index\": {index}, \"fog\": {}, \"ambient\": {}}}",
                    table.va,
                    row.packed_colour(),
                    list(row.ambient(), |v| v.to_bits().to_string()),
                )
            }
            Some("texclut") => {
                let tc = INF.tex_clut(n(1) as u32, n(2) as u32);
                format!("[{}, {}]", tc.clut_type, tc.tex_type)
            }
            _ => panic!("bad request: {line}"),
        };
        writeln!(out, "{json}").unwrap();
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

fn list<T>(items: impl IntoIterator<Item = T>, f: impl Fn(T) -> String) -> String {
    format!("[{}]", items.into_iter().map(f).collect::<Vec<_>>().join(", "))
}

fn random(p: &Params, d: &Dummies) -> String {
    let g = match dungeon::generate(&INF, p, d) {
        Ok(g) => g,
        Err(dungeon::GenerateError::NoDownStairs { floor }) => {
            return format!("{{\"error\": \"no down stairs\", \"floor\": {floor}}}");
        }
        Err(e) => return format!("{{\"error\": \"{e}\"}}"),
    };
    let lake = dungeon::is_lake(g.dtype);
    let sym = match g.statue {
        Some(s) => [u32::from(!lake), s.floor as u32, s.room as u32, u32::from(lake)],
        None => [0; 4],
    };
    let floors = list(&g.floors, |fl| {
        let start = list(&fl.start, |s| match s {
            None => "null".to_string(),
            Some(s) => {
                let pos = s.pos.map_or("null".to_string(), |p| format!("[{}, {}]", p[0].to_bits(), p[1].to_bits()));
                format!("{{\"pos\": {pos}, \"w\": {}}}", s.w().to_bits())
            }
        });
        let rooms = list(&fl.rooms, |r| {
            let model = match &r.model {
                None => "null".to_string(),
                Some(m) => format!(
                    "{{\"table\": \"{}\", \"k\": {}, \"r\": {}, \"name\": \"{}\", \"rotate\": {}, \"minimap\": {}, \
                     \"statue\": {}, \"gims\": {}}}",
                    m.table.name,
                    m.row,
                    m.pick,
                    m.name,
                    m.rotate.radians().to_bits(),
                    m.minimap.code(),
                    m.statue,
                    list(&m.dummies, |d| format!("[{}, {}]", d.pattern, d.keep)),
                ),
            };
            format!(
                "{{\"index\": {}, \"x\": {}, \"y\": {}, \"size\": {}, \"direc\": {}, \"pos\": [{}, {}], \"model\": {model}}}",
                r.index,
                r.x,
                r.y,
                r.size.index(),
                r.exits.0,
                r.pos[0].to_bits(),
                r.pos[1].to_bits(),
            )
        });
        format!(
            "{{\"level\": {}, \"up\": {}, \"down\": {}, \"room_num\": {}, \"retries\": {}, \"start\": {start}, \
             \"map\": \"{}\", \"rooms\": {rooms}}}",
            fl.level,
            fl.up,
            fl.down.unwrap_or(dungeon::NO_ROOM as usize),
            fl.rooms.len(),
            fl.retries,
            hex(&fl.map.bytes()),
        )
    });
    format!(
        "{{\"level_max\": {}, \"seed\": {}, \"randcnt\": {}, \"sym\": {}, \"gims\": {}, \"floorRoomNum\": {}, \
         \"floors\": {floors}}}",
        g.level_max,
        g.rng.seed,
        g.rng.count,
        list(sym, |v| v.to_string()),
        list(&g.gimmicks, |x| format!("[{}, {}, {}]", x.floor, x.room, x.gim.slot_type())),
        g.floors.last().map_or(0, |f| f.rooms.len()),
    )
}

fn story(event: i32, index: i32) -> String {
    let Some(s) = dungeon::story(&INF, event, index) else { return "null".to_string() };
    let slot = INF.edit.iter().position(|e| std::ptr::eq(e, s.edit)).unwrap();
    let floors = list(&s.floors, |fl| {
        format!(
            "{{\"level\": {}, \"up\": {}, \"down\": {}, \"rooms\": {}, \"map\": \"{}\"}}",
            fl.level,
            fl.up,
            fl.down.unwrap_or(dungeon::NO_ROOM as usize),
            list(&fl.rooms, |r| r.index.to_string()),
            hex(&fl.map.bytes()),
        )
    });
    format!(
        "{{\"slot\": {slot}, \"name\": \"{}\", \"gimmicks\": {}, \"floors\": {floors}}}",
        s.edit.name,
        s.edit.gimmicks.len()
    )
}

fn counts(archive: &Archive, name: &str) -> String {
    let Ok(data) = archive.inflate_named(name) else { return "null".to_string() };
    let c = piney_data::ccs::Ccs::parse(data).expect("CCSF");
    let d = Dummies::read(&c, &INF.gim_patterns).expect("dummies");
    let mut names: Vec<_> = d.by_anime.keys().collect();
    names.sort();
    let items: Vec<String> = names.iter().map(|k| format!("\"{k}\": {}", list(d.get(k), |v| v.to_string()))).collect();
    format!("{{{}}}", items.join(", "))
}
