//! Answers `tools/test_chat_msg_rs.py`: the chat balloons
//! (`piney_fieldui::chat_msg::ChatMsg`, `ccChatMsg`). One request a line
//! (numbers in hex), one JSON line back: `open WHO TEXT-HEX` (OpenChat),
//! `close` (CloseChat) and `disp STILL PLAYER [WHO LISTED ON X Y]...`
//! (Disp(still) with the speakers where they are: the slots, the window's
//! cells and the texts).

use std::io::BufRead;

use piney_data::volume::Volume;
use piney_desktop::kanji::{Fonts, Names};
use piney_fieldui::chat_msg::{ChatAt, ChatMsg};
use piney_fieldui::ctrl::Draw;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap()
}

fn main() {
    let mut chat = ChatMsg::new();
    let names = Names::default();
    // Infection's, the game the harness runs.
    let fonts = Fonts::of(Volume::Inf, Vec::new());
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "open" => {
                let text: Vec<u8> =
                    (0..w[2].len() / 2).map(|i| u8::from_str_radix(&w[2][2 * i..2 * i + 2], 16).unwrap()).collect();
                chat.open(hex(w[1]), &text, &fonts, &names);
                println!("{{}}");
            }
            "close" => {
                chat.close();
                println!("{{}}");
            }
            "disp" => {
                let still = hex(w[1]) != 0;
                let player = Some(hex(w[2]));
                let at: Vec<ChatAt> = w[3..]
                    .chunks(5)
                    .map(|c| ChatAt {
                        who: hex(c[0]),
                        listed: hex(c[1]) != 0,
                        at: (hex(c[2]) != 0).then(|| (hex(c[3]) as i32, hex(c[4]) as i32)),
                    })
                    .collect();
                let mut draws = Vec::new();
                chat.disp(still, &at, player, &mut draws);
                let mut cells = Vec::new();
                let mut texts = Vec::new();
                for d in &draws {
                    match d {
                        Draw::Send(ps) => {
                            for p in ps {
                                cells.push(format!(
                                    "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
                                    p.code,
                                    p.dx.to_bits(),
                                    p.dy.to_bits(),
                                    p.sx.to_bits(),
                                    p.sy.to_bits(),
                                    p.su,
                                    p.sv,
                                    p.wu,
                                    p.wv,
                                    p.wi,
                                    p.rgba[0],
                                    p.rgba[1],
                                    p.rgba[2],
                                    p.rgba[3]
                                ));
                            }
                        }
                        Draw::Text { text, dx, dy, rgba, .. } => {
                            let t: Vec<String> = text.iter().map(|b| b.to_string()).collect();
                            texts.push(format!(
                                "[[{}], {}, {}, {}, {}, {}, {}]",
                                t.join(", "),
                                dx.to_bits(),
                                dy.to_bits(),
                                rgba[0],
                                rgba[1],
                                rgba[2],
                                rgba[3]
                            ));
                        }
                        _ => {}
                    }
                }
                let slots: Vec<String> = chat
                    .slots
                    .iter()
                    .map(|s| format!("[{}, {}, {}, {}, {}]", s.cf, s.cn, s.scope[0], s.scope[1], s.scope[2]))
                    .collect();
                println!(
                    "{{\"slots\": [{}], \"cells\": [{}], \"texts\": [{}]}}",
                    slots.join(", "),
                    cells.join(", "),
                    texts.join(", ")
                );
            }
            _ => panic!("bad request: {line}"),
        }
    }
}
