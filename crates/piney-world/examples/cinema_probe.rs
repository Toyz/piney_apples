//! Answers `tools/test_cinema_rs.py`: the boss fight's cinema
//! (`piney_world::cinema::Cinema`, `ccBossEffCinemaFade`). One request a line,
//! one JSON line back: `on N` (`OnCinemaMode(N)`; only 2, Skeith's, has a
//! name), `off` (`OffCinemaMode`), `draw` (the name's transparency bits and
//! `wv`, the bars' y).

use std::io::BufRead;

use piney_draw::TexRef;
use piney_world::cinema::{Cinema, Name};

fn main() {
    let mut c = Cinema::new();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "on" => {
                let n: i32 = w[1].parse().unwrap();
                let name = (n == 2).then_some(Name { tex: TexRef::Upload(0), tex_h: 256, row: 0 });
                let named = name.is_some() && !(1..=3).contains(&c.mode);
                c.on(name);
                println!("{{\"settex\": {named}}}");
            }
            "off" => {
                c.off();
                println!("{{}}");
            }
            "draw" => {
                let s = c.step();
                let name = s.name.map_or("null".to_string(), |(tp, wv)| format!("[{tp}, {wv}]"));
                let bars = s.bars.map_or("null".to_string(), |[a, b]| format!("[{a}, {b}]"));
                println!("{{\"name\": {name}, \"bars\": {bars}}}");
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
