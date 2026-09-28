//! The area generator (`piney_data::area`) on commands from stdin, one JSON
//! line out per command, for `tools/test_area_rs.py` to hold against the
//! game's own code run in `tools/eemu.py`.
//!
//! ```text
//! area_probe ISO
//!   gen A B C SERVER FLAG71 CRISIS VOLUME    SimGenerateCode + dungeon types
//!   protect N                                 IsProtectArea with eventAreaNumber N
//!   info N                                    GetEventAreaInfo(N): the row, -1 for none
//!   fromev N PART                             GetWordParamFromEvCode(N, PART): the ID, null for none
//!   ev N SERVER FLAG71                        instruction 118 `area N`
//!   wordid TEXT                               GetWordParamID(TEXT) (the rest of the line)
//!   go A B C SERVER                           SetGenerateCode: the ChangeArea or ChangeScene
//! ```

use std::io::{BufRead, Write};

use piney_data::area::{self, AreaCode, AreaTables, EvArea};
use piney_data::iso::Iso;

fn code_json(t: &AreaTables, r: &AreaCode, crisis: bool) -> String {
    let a = r.attrs;
    // The harness sets the crisis byte, not a later volume's flag.
    let dt = r.dungeon_type(t, t.dungeon_rule().save_flag_of_crisis(crisis));
    format!(
        "{{\"a\":{},\"b\":{},\"c\":{},\"code\":{},\"fieldSeed\":{},\"dungeonSeed\":[{},{},{}],\"fieldType\":{},\
         \"dungeonSize\":{},\"weather\":{},\"ground\":{},\"object\":{},\"areaLevel\":{},\"enemyOfs\":{},\
         \"itemOfs\":{},\"circleOfs\":{},\"event\":{},\"timeSym\":{},\"levelMax\":{},\"roomMax\":{},\"seed\":{},\
         \"randcnt\":{},\"dungeonType\":[{},{}]}}",
        r.a,
        r.b,
        r.c,
        r.code,
        r.field_seed,
        r.dungeon_seed[0],
        r.dungeon_seed[1],
        r.dungeon_seed[2],
        a.field_type,
        a.dungeon_size,
        a.weather,
        a.ground,
        a.object,
        a.area_level,
        a.enemy_ofs,
        a.item_ofs,
        a.circle_ofs,
        r.event,
        r.time_sym as i32,
        r.level_max,
        r.room_max,
        r.seed,
        r.randcnt,
        dt[0],
        dt[1]
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso = std::env::args().nth(1).ok_or("usage: area_probe ISO")?;
    let tables = AreaTables::of(Iso::open(iso)?.volume()?);
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines() {
        let line = line?;
        let (cmd, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let n: Vec<i64> = rest.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        let json = match cmd {
            "gen" => {
                let mut t = tables.clone();
                t.volume = n[6] as i32;
                match area::sim_generate_code(&t, n[0] as i32, n[1] as i32, n[2] as i32, n[3] as i32, n[4] != 0) {
                    Some(r) => code_json(&t, &r, n[5] != 0),
                    None => "null".to_string(),
                }
            }
            "protect" => format!("{{\"protect\":{}}}", tables.is_protect_area(n[0] as i32, false) as i32),
            "info" => {
                let row = tables.events.iter().take(tables.search()).position(|e| e.code == n[0] as i32);
                format!("{{\"row\":{}}}", row.map_or(-1, |r| r as i64))
            }
            "fromev" => match tables.word_from_event(n[0] as i32, n[1] as usize, false) {
                Some(w) => format!("{{\"id\":{}}}", w.id),
                None => "{\"id\":null}".to_string(),
            },
            "ev" => match area::ev_area(tables, n[0] as i32, n[1] as i32, n[2] != 0) {
                EvArea::Number(k) => format!("{{\"number\":{k}}}"),
                EvArea::Generated(r) => format!("{{\"generated\":{}}}", code_json(tables, &r, false)),
                EvArea::Missing => "{\"missing\":1}".to_string(),
            },
            "wordid" => format!("{{\"id\":{}}}", tables.word_id(rest)),
            "go" => match area::sim_generate_code(tables, n[0] as i32, n[1] as i32, n[2] as i32, n[3] as i32, false)
                .map(|r| r.go())
            {
                Some(area::Go::ChangeArea(a, k)) => format!("{{\"area\":[{a},{k}]}}"),
                Some(area::Go::ChangeScene(v)) => {
                    format!("{{\"scene\":[{}]}}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
                }
                None => "null".to_string(),
            },
            _ => return Err(format!("unknown command {line:?}").into()),
        };
        writeln!(out, "{json}")?;
    }
    Ok(())
}
