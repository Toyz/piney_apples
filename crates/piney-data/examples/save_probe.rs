//! Answers `tools/test_save_init_rs.py`: `ccSaveData::Init` and the boot's
//! save as `piney_data::save` ports them, Init's text read from the disc named
//! on the command line. One request a line, one line back: `init FLAG MAIN BGM
//! SE OUTPUT HEX` (the save after `init(FLAG)` in hex, 0x8530 bytes and a later
//! volume's extension), `boot` (`SaveData::boot`) and `repair HEX` (1 or 0 for
//! changed, then the save after `repair_port_save`).

use std::io::{BufRead, Write};

use piney_data::iso::Iso;
use piney_data::save::{InitText, SaveData, SoundLevels};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn main() {
    let iso = std::env::args().nth(1).expect("usage: save_probe ISO");
    let text = InitText::from_disc(&mut Iso::open(&iso).expect("the disc")).expect("Init's text");
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let answer = match w.as_slice() {
            ["init", flag, main, bgm, se, output, data] => {
                let n = |s: &str| s.parse::<i32>().unwrap();
                let mut s = SaveData::from_bytes(&unhex(data)).unwrap();
                s.init(n(flag), &SoundLevels { main: n(main), bgm: n(bgm), se: n(se), output: n(output) }, &text);
                hex(s.slot_bytes(text.volume))
            }
            ["boot"] => hex(SaveData::boot(&text).slot_bytes(text.volume)),
            ["repair", data] => {
                let mut s = SaveData::from_bytes(&unhex(data)).unwrap();
                let changed = s.repair_port_save();
                format!("{} {}", u8::from(changed), hex(s.bytes()))
            }
            _ => panic!("bad request: {line}"),
        };
        writeln!(out, "{answer}").unwrap();
    }
}
