//! Answers `tools/test_cinema_rs.py` (`cinema_probe VOLUME`, `volumeNum`
//! 1-4): the boss fight's cinema (`piney_world::combat::cinema::Cinema`,
//! `ccBossEffCinemaFade`). One request a line, one JSON line back: `on N`
//! (`OnCinemaMode(N)` in a field outside 9-12: the name of the volume's
//! `_g_cinemaSkillName[N]` when `CinemaOn` reads that row and it names a
//! file and a texture), `off` (`OffCinemaMode`), `draw` (the name's
//! transparency bits and `wv`, the bars' y).

use std::io::BufRead;

use piney_data::volume::Volume;
use piney_draw::TexRef;
use piney_world::combat::cinema::{self, Cinema, Name};

fn main() {
    let n: i32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let volume = Volume::from_number(n).expect("volumeNum 1-4");
    let rows = piney_data::tables::combat::of(volume).cinema_skill_names;
    let mut c = Cinema::new(volume);
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "on" => {
                let n: i32 = w[1].parse().unwrap();
                let row = usize::try_from(n)
                    .ok()
                    .filter(|_| cinema::takes(volume, n))
                    .and_then(|k| rows.get(k))
                    .and_then(|r| Some((r.file?, r.tex?, r.row)));
                let name = row.map(|(_, _, row)| Name { tex: TexRef::Upload(0), tex_h: 256, row });
                let named = name.is_some() && !(1..=3).contains(&c.mode);
                c.on(name);
                let tex = match row {
                    Some((f, t, _)) if named => format!("[{f:?}, {t:?}]"),
                    _ => "null".into(),
                };
                println!("{{\"settex\": {named}, \"tex\": {tex}}}");
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
