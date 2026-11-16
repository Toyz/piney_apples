//! `ccThMother`'s pause ([`piney_demo::pause`]) for tools/test_mother_pause_rs.py.
//! Argument: the disc's `volumeNum`. Input: one line a frame, `f PLAYING
//! PAUSE_FLAG PLUGGED PUSH`. Output: one JSON line a frame: the calls, the
//! texts (`MakePacketStr` with `fontSys` as `SetType` left it, floats as
//! bits), `ccGame.fade`'s elements after the frame, and whether the tasks
//! and the event task run.

use std::io::{BufRead, Write};

use piney_data::volume::Volume;
use piney_demo::fade::ScFade;
use piney_demo::pause::{self, Call, Pause};

fn call(c: Call) -> &'static str {
    match c {
        Call::FlipOff => "flip_off",
        Call::StillDown => "still_down",
        Call::SleepAll => "sleep_all",
        Call::SleepEvent => "sleep_event",
        Call::WakeAll => "wake_all",
        Call::FlipOn => "flip_on",
        Call::WakeEvent => "wake_event",
    }
}

fn main() {
    let volume =
        std::env::args().nth(1).and_then(|n| Volume::from_number(n.parse().ok()?)).expect("the disc's volumeNum, 1-4");
    let texts = piney_data::tables::game::of(volume).pause_texts;
    let texts = [texts[0].as_bytes().to_vec(), texts[1].as_bytes().to_vec()];
    let colour = piney_data::tables::kanji::shared().sprite_color_table[pause::COLOUR];
    let mut fade = ScFade::default();
    let mut p: Option<Pause> = None;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<u32> = line.split_whitespace().skip(1).map(|s| s.parse().unwrap()).collect();
        let (playing, flag, plugged, push) = (w[0] != 0, w[1] != 0, w[2] != 0, w[3]);
        let f = match p.as_mut() {
            Some(q) => q.frame(plugged, push),
            None if pause::starts(playing, flag, plugged) => {
                let (q, f) = Pause::begin(texts.clone(), std::mem::take(&mut fade), plugged, push);
                p = Some(q);
                f
            }
            None => pause::Frame { tasks: true, event: true, ..pause::Frame::default() },
        };
        if p.as_ref().is_some_and(Pause::over) {
            fade = p.take().map(|q| q.fade).unwrap_or_default();
        }
        let calls: Vec<String> = f.calls.iter().map(|&c| format!("\"{}\"", call(c))).collect();
        let strs: Vec<String> = f
            .texts
            .iter()
            .map(|t| {
                let (wu, wv, su, sv, wi, sx, sy) = pause::font_cells(t.kind);
                let hex: String = t.text.iter().map(|b| format!("{b:02x}")).collect();
                format!(
                    "[\"{hex}\",{},{},{},{},{su},{sv},{wu},{wv},{wi},[{},{},{},128]]",
                    t.dx.to_bits(),
                    t.dy.to_bits(),
                    sx.to_bits(),
                    sy.to_bits(),
                    colour[0],
                    colour[1],
                    colour[2]
                )
            })
            .collect();
        let held = p.as_ref().map_or(&fade, |q| &q.fade);
        let elems: Vec<String> =
            held.elm.iter().map(|e| format!("[{},{},{},{},{}]", e.status, e.cnt, e.tcnt, e.col0, e.col1)).collect();
        writeln!(
            out,
            "{{\"calls\":[{}],\"str\":[{}],\"fade\":[{}],\"tasks\":{},\"event\":{}}}",
            calls.join(","),
            strs.join(","),
            elems.join(","),
            f.tasks,
            f.event
        )
        .unwrap();
    }
}
