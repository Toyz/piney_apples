//! Answers `tools/test_toppage_rs.py`: runs the top page's task
//! (`ccThToppageCtrl`, without the system menu) on scenarios it is given and
//! prints one JSON line per frame, so the test can run the same scenario
//! through the game's own code in eemu (`toppage_probe ISO < requests`, built
//! with the `trace` feature). The requests: `reset`, `save OFFSET HEX`,
//! `operate HEX`, `keys CNTR FLAG`, `start` (the constructor), `frame RAW
//! MENUTYPE` (one `Main`) and `dump` (the board as the port reads it).

use std::io::{BufRead, Write};
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_demo::fade::ScFade;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::view::View;
use piney_input::{Buttons, Pad, Raw};
use piney_toppage::Request;
use piney_toppage::assets::Assets;
use piney_toppage::bbs::{BBS_SIZE, Bbs};
use piney_toppage::control::{AnmId, Env, TopPageCtrl};
use piney_toppage::trace;
use piney_toppage::util::{KeyRepeat, ScrollBar};

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn calls(c: &[trace::Call]) -> String {
    let v: Vec<String> = c.iter().map(|(op, l, a)| format!("[\"{op}\",\"{l}\",\"{}\"]", esc(a))).collect();
    format!("[{}]", v.join(","))
}

fn reqs(r: &[Request]) -> String {
    let v: Vec<String> = r
        .iter()
        .map(|r| match r {
            Request::Se(n) => format!("[\"se\",{n}]"),
            Request::SoundFadeOut => "[\"fadeout\"]".to_string(),
            Request::EnableReset(on) => format!("[\"reset\",{}]", i32::from(*on)),
            Request::ChangeMode { num, sf } => format!("[\"mode\",{num},{sf}]"),
            Request::ChangeArea { area, town } => format!("[\"area\",{area},{town}]"),
            other => format!("[\"other\",\"{}\"]", esc(&format!("{other:?}"))),
        })
        .collect();
    format!("[{}]", v.join(","))
}

fn sb(s: &ScrollBar) -> String {
    format!("[{},{},{},{},{},{},{},{}]", s.s_pos, s.s_length, s.b_pos, s.b_length, s.l_pos, s.lmin, s.lmax, s.n_page)
}

fn bbs_state(b: &Bbs) -> String {
    let m = &b.msg_page;
    let w = &b.write;
    let threads: Vec<String> = b
        .threads
        .iter()
        .map(|t| {
            let msgs: Vec<String> = t.msgs.iter().map(|m| format!("[{},{}]", m.logical, m.read)).collect();
            format!("[{},{},[{}]]", t.logical, t.read, msgs.join(","))
        })
        .collect();
    let idx = |o: Option<usize>| o.map_or(-1, |i| i as i64);
    format!(
        "{{\"draw\":{},\"exit\":{},\"init\":{},\"thnum\":{},\"tindex\":{},\"tstart\":{},\"sb\":[{},{},{}],\
         \"m\":[{},{},{},{},{},{}],\"w\":[{},{},{},{},{},{},{},{},{},{},{}],\"mark\":[{},{},{},{}],\"threads\":[{}]}}",
        b.draw_state,
        i32::from(b.exit),
        i32::from(b.initialized),
        b.th_num(),
        b.thread_page.index,
        b.thread_page.start_line,
        sb(&b.thread_page.sb),
        sb(&m.sb_low),
        sb(&m.sb_high),
        m.index,
        m.th_start,
        m.th_extra,
        m.i_draw_state,
        m.msg_start,
        m.msg_last,
        idx(w.thread),
        idx(w.msg),
        w.th_index,
        w.msg_index,
        w.frame_cnt,
        w.line_index,
        w.str_cnt,
        w.no_col_length,
        w.b_draw_end,
        w.start_line,
        w.b_write,
        b.scroll_mark.blink_flag,
        b.scroll_mark.blink_interval,
        b.scroll_mark.count,
        i32::from(b.scroll_mark.set_mask),
        threads.join(",")
    )
}

fn label(c: Option<AnmId>) -> &'static str {
    match c {
        Some(AnmId::Login) => "login",
        Some(AnmId::Bbs) => "bbs",
        Some(AnmId::BbsNew) => "bbsnew",
        Some(AnmId::Quit) => "quit",
        Some(_) => "other",
        None => "none",
    }
}

fn state(c: &TopPageCtrl, save: &SaveState, keys: &KeyRepeat, reset: bool) -> String {
    let bytes = &save.save.bytes()[offset::BBS_LIST..offset::BBS_LIST + BBS_SIZE];
    let posts: Vec<String> =
        bytes.iter().enumerate().filter(|(_, v)| **v != 0).map(|(i, v)| format!("[{i},{v}]")).collect();
    format!(
        "{{\"mode\":{},\"cmd\":{},\"act\":{},\"proc\":{},\"bbsnew\":{},\"command\":\"{}\",\"exit\":{},\
         \"bbs\":{},\"keys\":[{},{}],\"reset\":{},\"opset\":{},\"posts\":[{}],\"lasttown\":{}}}",
        c.mode,
        c.cmd,
        c.act_count,
        c.act_proccess,
        i32::from(c.bbs_new),
        label(c.command),
        c.exit,
        bbs_state(&c.bbs),
        keys.push_flag,
        keys.push_cntr,
        i32::from(reset),
        save.operate_set,
        posts.join(","),
        save.save.u8(offset::LAST_TOWN) as i8
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path)?;
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let assets = Assets::read(&mut iso, archive)?;
    // The port's fresh save, Init's text from the executable.
    let init_text = piney_desktop::InitText::from_disc(&mut iso)?;
    let mut save = SaveState::fresh_with(&init_text);
    let mut keys = KeyRepeat::default();
    let mut fade = ScFade::default();
    let mut view = View::default();
    let mut pad = Pad::default();
    let mut reset = true;
    let mut ctl: Option<TopPageCtrl> = None;
    let stdin = std::io::stdin();
    let out = std::io::stdout();
    let mut out = std::io::BufWriter::new(out.lock());
    for line in stdin.lock().lines() {
        let line = line?;
        let mut it = line.split_whitespace();
        let Some(cmd) = it.next() else { continue };
        match cmd {
            "reset" => {
                save = SaveState::fresh_with(&init_text);
                keys = KeyRepeat::default();
                fade = ScFade::default();
                view = View::default();
                pad = Pad::default();
                reset = true;
                ctl = None;
            }
            "save" => {
                let at = usize::from_str_radix(it.next().ok_or("offset")?.trim_start_matches("0x"), 16)?;
                let hex = it.next().ok_or("hex")?;
                for i in 0..hex.len() / 2 {
                    save.save.set_u8(at + i, u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)?);
                }
            }
            "operate" => save.operate = u64::from_str_radix(it.next().ok_or("mask")?.trim_start_matches("0x"), 16)?,
            "keys" => {
                keys.push_cntr = it.next().ok_or("cntr")?.parse()?;
                keys.push_flag = it.next().ok_or("flag")?.parse()?;
            }
            "start" => {
                trace::start();
                let c = TopPageCtrl::new(assets.bbs.file.clone(), assets.neutral.clone(), &assets.bbs, &mut save);
                let cl = trace::take();
                writeln!(out, "{{\"calls\":{},\"state\":{}}}", calls(&cl), state(&c, &save, &keys, reset))?;
                ctl = Some(c);
            }
            "frame" => {
                let raw = u32::from_str_radix(it.next().ok_or("raw")?.trim_start_matches("0x"), 16)?;
                let menu_type: i32 = it.next().ok_or("menu type")?.parse()?;
                pad.read(&Raw { buttons: Buttons(raw), analog: false, ..Raw::default() });
                let c = ctl.as_mut().ok_or("no start")?;
                let mut ctx = Ctx::new(view.clone());
                let mut req = Vec::new();
                let names = save.names();
                let mut x = Env {
                    ctx: &mut ctx,
                    fonts: &assets.fonts,
                    names: &names,
                    save: &mut save,
                    pad: &pad,
                    keys: &mut keys,
                    menu_type,
                    fade: &mut fade,
                    req: &mut req,
                };
                c.main(&assets.bbs, &mut x);
                fade.advance();
                view = ctx.view.clone();
                for r in &req {
                    if let Request::EnableReset(on) = r {
                        reset = *on;
                    }
                }
                let cl = trace::take();
                writeln!(
                    out,
                    "{{\"pad\":[{},{},{},{}],\"calls\":{},\"req\":{},\"state\":{}}}",
                    pad.direct.bits(),
                    pad.push.bits(),
                    pad.unpush.bits(),
                    pad.repeat.bits(),
                    calls(&cl),
                    reqs(&req),
                    state(c, &save, &keys, reset)
                )?;
            }
            "dump" => {
                let tbl: Vec<String> = assets
                    .bbs
                    .tables
                    .iter()
                    .map(|t| {
                        let th: Vec<String> = t
                            .iter()
                            .map(|th| {
                                let m: Vec<String> = th.msgs.iter().map(|m| m.max_lines.to_string()).collect();
                                format!("[{}]", m.join(","))
                            })
                            .collect();
                        format!("[{}]", th.join(","))
                    })
                    .collect();
                writeln!(out, "{{\"tables\":[{}]}}", tbl.join(","))?;
            }
            other => return Err(format!("unknown request {other}").into()),
        }
    }
    out.flush()?;
    Ok(())
}
