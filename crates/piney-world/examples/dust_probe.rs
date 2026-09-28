//! Answers `tools/test_dust_ctrl_rs.py`: `piney_world::combat::dust::ctrl`
//! (`ccEnemyDustCtrl::ctrl`) over rows the harness sends. One request a line
//! (numbers in hex), one JSON line back: `rows HEX` (the controller's
//! `ccEnemyDustInfo` rows, 0x20 bytes each) and `ctrl FLAG ANM FRAME` (the
//! rings as [row, n, life, s, r]).

use std::io::BufRead;

use piney_battle::enemy_motion::DustAt;
use piney_world::combat::dust;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn main() {
    let mut rows: Vec<[u8; 32]> = Vec::new();
    let mut last: Vec<i16> = Vec::new();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.first().copied() {
            Some("rows") => {
                let b: Vec<u8> =
                    (0..w[1].len() / 2).map(|i| u8::from_str_radix(&w[1][2 * i..2 * i + 2], 16).unwrap()).collect();
                rows = b.chunks(32).map(|c| c.try_into().unwrap()).collect();
                last = vec![0; rows.len()];
                println!("{{}}");
            }
            Some("ctrl") => {
                let at = DustAt {
                    info: None,
                    n: rows.len() as i32,
                    disp_sw: true,
                    smoke: 0,
                    anm_num: hex(w[2]) as i16,
                    frame_num: hex(w[3]) as i16,
                    pos: [0; 4],
                    dirc: [0; 4],
                };
                let out: Vec<String> = dust::ctrl(&rows, &mut last, hex(w[1]) as i32, &at)
                    .iter()
                    .map(|r| format!("[{}, {}, {}, {}, {}]", r.row, r.n, r.life, r.s, r.r))
                    .collect();
                println!("[{}]", out.join(", "));
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
