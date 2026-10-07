//! Answers `tools/test_desktop_rs.py` (and `tools/test_desktop_name_rs.py`):
//! runs pieces of the desktop's logic on states it is given and prints one
//! JSON line per request, so the tests can run the same states through the
//! game's own code in eemu (`desktop_probe ISO < requests`, built with the
//! `trace` feature). The requests name what they run: the main screen
//! (`select`, `logo`, `newicon`), the mailer, news, accessory and audio lists,
//! `kanji`, `dec`, the menu task (`menu`), `ccMessage` (`msg*`), the Data
//! screen (`datascen`, `data*`) and the name entry (`nerun`).

use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::acces::Accessory;
use piney_desktop::anm::{Ctx, trace};
use piney_desktop::assets::Assets;
use piney_desktop::audio::Audio;
use piney_desktop::desktop::DesktopControl;
use piney_desktop::kanji::{Kanji, Kt, Names};
use piney_desktop::mail::{MailCtx, Mailer, Views, cursor_alpha};
use piney_desktop::message::{MsgDraw, MsgWindow};
use piney_desktop::news::News;
use piney_desktop::view::{LayerView, View};
use piney_desktop::{Request, SaveState};
use piney_draw::Scissor;

fn nums(it: &mut std::str::SplitWhitespace) -> Vec<i64> {
    it.map(
        |s| if let Some(h) = s.strip_prefix("0x") { i64::from_str_radix(h, 16).unwrap() } else { s.parse().unwrap() },
    )
    .collect()
}

fn ses(req: &[Request]) -> String {
    let v: Vec<String> = req
        .iter()
        .filter_map(|r| match r {
            Request::Se(s) => Some(s.0.to_string()),
            _ => None,
        })
        .collect();
    format!("[{}]", v.join(","))
}

fn calls(c: &[(&'static str, &'static str, String)]) -> String {
    let v: Vec<String> = c.iter().map(|(op, l, a)| format!("[\"{op}\",\"{l}\",\"{}\"]", a.replace('"', "'"))).collect();
    format!("[{}]", v.join(","))
}

/// The port's fresh save as the runtime starts it: `ccSaveData::Init` for
/// the disc's volume.
fn fresh(assets: &Assets) -> SaveState {
    SaveState::fresh_with(&piney_desktop::InitText::of(assets.volume))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path)?;
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let mut assets = Assets::read(&mut iso, archive)?;
    let mut save = fresh(&assets);
    let mut ctl = DesktopControl::new(&mut assets, &save)?;
    let mut mailer = Mailer::new(&assets, &save)?;
    let mut news = News::new(&assets)?;
    let mut acces = Accessory::new(&assets)?;
    let mut audio = Audio::new(&assets, &save)?;
    let mut msg = MsgWindow::default();
    let msg_names = Names { name: b"Kite".to_vec(), real: Vec::new() };
    let hex = |w: &str| -> Option<Vec<u8>> {
        if w == "-" {
            return None;
        }
        if w == "=" {
            return Some(Vec::new());
        }
        Some((0..w.len() / 2).map(|i| u8::from_str_radix(&w[2 * i..2 * i + 2], 16).unwrap()).collect())
    };
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    while let Some(line) = lines.next() {
        let line = line?;
        let mut it = line.split_whitespace();
        let Some(cmd) = it.next() else { continue };
        match cmd {
            "select" => {
                let v = nums(&mut it);
                ctl.dsel = v[0] as i32;
                ctl.lr_flg = v[1] != 0;
                ctl.app = v[2] as i32;
                ctl.check = v[3] as i32;
                ctl.flg = v[4] as i32;
                ctl.start_flg = v[5] != 0;
                ctl.draw_flg = v[6] as i32;
                ctl.lock = v[7] != 0;
                save.operate = v[8] as u64;
                save.operate_set = -1;
                ctl.new_mail = 5;
                let mut req = Vec::new();
                ctl.select_mode(&mut save, v[9] as u32, &mut req);
                println!(
                    "{{\"dsel\":{},\"lr\":{},\"app\":{},\"check\":{},\"flg\":{},\"drawflg\":{},\"lock\":{},\"newmail\":{},\"operate_set\":{},\"se\":{},\"reset\":{}}}",
                    ctl.dsel,
                    ctl.lr_flg as i32,
                    ctl.app,
                    ctl.check,
                    ctl.flg,
                    ctl.draw_flg,
                    ctl.lock as i32,
                    ctl.new_mail,
                    save.operate_set,
                    ses(&req),
                    req.contains(&Request::EnableReset(false)) as i32
                );
            }
            "logo" => {
                let v = nums(&mut it);
                ctl.dsel = v[0] as i32;
                ctl.lr_flg = v[1] != 0;
                ctl.lock = v[2] != 0;
                ctl.draw_flg = v[3] as i32;
                ctl.start_flg = v[4] != 0;
                for a in [&mut ctl.logo, &mut ctl.page_in, &mut ctl.page_out, &mut ctl.title] {
                    a.frame_spd = 256;
                }
                let fwd =
                    HashMap::from([("logo", v[5] != 0), ("in", v[6] != 0), ("out", v[7] != 0), ("title", v[8] != 0)]);
                trace::start(fwd);
                let mut ctx = Ctx::new(View::default());
                let r = ctl.draw_logo(&mut ctx);
                let c = trace::stop();
                println!(
                    "{{\"ret\":{r},\"dsel\":{},\"start\":{},\"lock\":{},\"spd\":[{},{},{}],\"calls\":{}}}",
                    ctl.dsel,
                    ctl.start_flg as i32,
                    ctl.lock as i32,
                    ctl.page_in.frame_spd,
                    ctl.page_out.frame_spd,
                    ctl.title.frame_spd,
                    calls(&c)
                );
            }
            "newicon" => {
                let v = nums(&mut it);
                ctl.tmp_cou = 126;
                ctl.fl = 0;
                ctl.icon_y = 0.0;
                ctl.tmp_irot = 0.0;
                let mut out = Vec::new();
                for _ in 0..v[0] {
                    trace::start(HashMap::new());
                    let mut ctx = Ctx::new(View::default());
                    ctl.new_icon_draw(&mut ctx, v[1] != 0, v[2] != 0);
                    let c = trace::stop();
                    out.push(format!(
                        "[{},{},{},{},{}]",
                        ctl.fl,
                        ctl.tmp_cou,
                        ctl.icon_y.to_bits(),
                        ctl.tmp_irot.to_bits(),
                        calls(&c)
                    ));
                }
                println!("[{}]", out.join(","));
            }
            "list" | "body" => {
                let v = nums(&mut it);
                let mut req = Vec::new();
                let mut views = Views::default();
                let mut ctx = Ctx::new(View::default());
                let mut x = MailCtx {
                    ctx: &mut ctx,
                    fonts: &assets.fonts,
                    names: Names::default(),
                    save: &mut save,
                    req: &mut req,
                    views: &mut views,
                    count: 0,
                    frame_rate: 1,
                };
                if cmd == "list" {
                    mailer.probe_list(v[0] as usize);
                }
                mailer.probe_set((0, 0, 0, 0, 0, 30));
                mailer.probe_set_pre_pos(0);
                let mut out = Vec::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let mut words = l.split_whitespace();
                    words.next();
                    let p = nums(&mut words);
                    let pad = [p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32];
                    x.req.clear();
                    if cmd == "list" {
                        mailer.probe_move_list(&mut x, pad);
                    } else {
                        mailer.probe_move_body(&mut x, pad, v[0] as i32);
                    }
                    let s = mailer.probe_state();
                    out.push(format!(
                        "[{},{},{},{},{},{},{},{}]",
                        s.0,
                        s.1,
                        s.2,
                        s.3,
                        s.4,
                        s.5,
                        mailer.probe_pre_pos(),
                        ses(x.req)
                    ));
                }
                println!("[{}]", out.join(","));
            }
            "newscur" | "webmove" | "newslen" | "weblen" => {
                let v = nums(&mut it);
                let mut req = Vec::new();
                let mut views = Views::default();
                let mut ctx = Ctx::new(View::default());
                let mut x = MailCtx {
                    ctx: &mut ctx,
                    fonts: &assets.fonts,
                    names: Names::default(),
                    save: &mut save,
                    req: &mut req,
                    views: &mut views,
                    count: 0,
                    frame_rate: 1,
                };
                match cmd {
                    "newslen" => {
                        news.probe_list(v[0] as usize);
                        let (dy, sy) = news.probe_length(&mut x, v[1] as i32, v[2] as i32);
                        println!("[{},{}]", dy.to_bits(), sy.to_bits());
                        continue;
                    }
                    "weblen" => {
                        news.probe_page(v[0] as i32, v[1] as i32);
                        let (dy, sy) = news.probe_web_length(&mut x);
                        println!("[{},{}]", dy.to_bits(), sy.to_bits());
                        continue;
                    }
                    "newscur" => news.probe_list(v[0] as usize),
                    _ => news.probe_page(v[0] as i32, 0),
                }
                let mut out = Vec::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let mut words = l.split_whitespace();
                    words.next();
                    let p = nums(&mut words);
                    let pad = [p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32];
                    x.req.clear();
                    if cmd == "newscur" {
                        news.probe_move_cur(&mut x, pad);
                        let s = news.probe_state();
                        out.push(format!("[{},{},{},{},{},{}]", s.0, s.1, s.2, s.3, s.4, ses(x.req)));
                    } else {
                        let s = news.probe_web_move(&mut x, pad);
                        out.push(format!("[{},{},{}]", s.0, s.1, s.2));
                    }
                }
                println!("[{}]", out.join(","));
            }
            "menu" => {
                // menu FRAMES [OPEN]: with OPEN, the title screen's menu
                // (ccGame.status 1, no reset), openReqNum 1 at frame OPEN.
                let v = nums(&mut it);
                let n = v[0] as usize;
                let title_open = v.get(1).map(|&f| f as usize);
                let mut script = HashMap::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let mut words = l.split_whitespace();
                    words.next();
                    let v = nums(&mut words);
                    script.insert(v[0] as usize, (v[1] as u32, v[2] as u32));
                }
                let mut menu = piney_desktop::dtmenu::DtMenu::new(assets.volume, None, None);
                menu.title = title_open.is_some();
                let mut state = fresh(&assets);
                let names = state.names();
                for f in 1..n {
                    if title_open == Some(f) {
                        menu.open_req = 1;
                    }
                    let (push, repeat) = script.get(&f).copied().unwrap_or((0, 0));
                    let pad = piney_input::Pad {
                        push: piney_input::Buttons(push),
                        repeat: piney_input::Buttons(repeat),
                        ..Default::default()
                    };
                    let mut req = Vec::new();
                    let mut x = piney_desktop::dtmenu::MenuCtx {
                        save: &mut state,
                        req: &mut req,
                        pad: &pad,
                        enable_reset: title_open.is_none(),
                        names: names.clone(),
                        save_sys: None,
                    };
                    menu.trace.clear();
                    menu.frame(&mut x);
                    let mut ctx = Ctx::new(View::default());
                    menu.disp(&mut ctx, &assets.fonts, &names, 1);
                    let r: Vec<String> = req
                        .iter()
                        .map(|q| match q {
                            Request::Se(s) => format!("[\"se\",{}]", s.0),
                            Request::DisplayOffset { x, y } => format!("[\"display_offset\",{x},{y}]"),
                            Request::SoundEnv { main, bgm, se, output } => {
                                format!("[\"sound_env\",{main},{se},{bgm},{output}]")
                            }
                            Request::Vibration { on } => format!("[\"vibration\",{}]", u8::from(*on)),
                            Request::CameraType(t) => format!("[\"camera\",{t}]"),
                            Request::ChangeMode { num, sf } => format!("[\"change_request\",{num},{sf}]"),
                            other => format!("[\"other\",\"{other:?}\"]"),
                        })
                        .collect();
                    println!(
                        "{{\"f\":{f},\"st\":[{},{},{},{},{},{}],\"ev\":[{}],\"req\":[{}]}}",
                        menu.menu,
                        menu.menu_next,
                        menu.menu_status,
                        menu.proccess,
                        menu.alpha,
                        menu.check_menu_type(),
                        menu.trace.join(","),
                        r.join(",")
                    );
                }
            }
            "msgchange" => {
                let w: Vec<&str> = it.collect();
                let emode = w[0].parse::<i32>()?;
                let name = hex(w[1]);
                let l: Vec<Option<Vec<u8>>> = w[2..5].iter().map(|x| hex(x)).collect();
                msg.change(emode, name.as_deref(), [l[0].as_deref(), l[1].as_deref(), l[2].as_deref()], &msg_names);
            }
            "msginfo" => {
                let w: Vec<&str> = it.collect();
                let l: Vec<Option<Vec<u8>>> = w[..4].iter().map(|x| hex(x)).collect();
                msg.change_info([l[0].as_deref(), l[1].as_deref(), l[2].as_deref(), l[3].as_deref()], &msg_names);
            }
            "msghide" => msg.hide_frame(),
            "msgclose" => msg.close(),
            "msgframe" => {
                let v = nums(&mut it);
                let mut req = Vec::new();
                let r = if v[2] != 0 {
                    msg.check(v[0] as u32, v[1] as u32, 0x40, &mut req).to_string()
                } else {
                    "null".to_string()
                };
                let draws: Vec<String> = msg
                    .disp(1)
                    .iter()
                    .map(|d| match d {
                        MsgDraw::Cell { code, dx, dy, sx, sy, grid, alpha } => format!(
                            "[\"pkt\",{code},{},{},{},{},{},{},{},{},{},{alpha}]",
                            dx.to_bits(),
                            dy.to_bits(),
                            sx.to_bits(),
                            sy.to_bits(),
                            grid.2,
                            grid.3,
                            grid.0,
                            grid.1,
                            grid.4
                        ),
                        MsgDraw::Text { text, count, dx, dy, rgba, centred, .. } => {
                            let dx = piney_desktop::message::text_dx(*dx, text, *centred, &assets.fonts, &msg_names);
                            let h: String = text.iter().map(|b| format!("{b:02x}")).collect();
                            format!(
                                "[\"kanji\",\"{h}\",{count},{},{},[{},{},{},{}]]",
                                dx.to_bits(),
                                dy.to_bits(),
                                rgba[0],
                                rgba[1],
                                rgba[2],
                                rgba[3]
                            )
                        }
                        MsgDraw::Send => "[\"send\"]".to_string(),
                    })
                    .collect();
                let voice = req.iter().filter(|q| matches!(q, Request::VoiceStop)).count();
                let m = &msg;
                println!(
                    "{{\"r\":{r},\"st\":[{},{},{},{},{},{},{},{},{},{},{},{},{},{}],\"draws\":[{}],\"se\":{},\"stop\":{voice}}}",
                    m.window_status,
                    m.window_alpha,
                    m.kanji_status,
                    m.kanji_alpha,
                    m.str_line,
                    m.str_clm[0],
                    m.str_clm[1],
                    m.str_clm[2],
                    m.str_clm[3],
                    m.cursol,
                    m.wait_cnt,
                    m.wait_close_cnt,
                    m.mode,
                    m.select,
                    draws.join(","),
                    ses(&req)
                );
            }
            "audlen" => {
                let v = nums(&mut it);
                let q = audio.probe_length(v[1] as i32, v[2] as i32, v[0] as i32);
                println!("[{},{},{},{}]", q[0].to_bits(), q[1].to_bits(), q[2].to_bits(), q[3].to_bits());
            }
            "audadd" => {
                let v = nums(&mut it);
                let mut s = fresh(&assets);
                for (w, &bits) in v[..3].iter().enumerate() {
                    s.save.set_i32(piney_desktop::save::offset::DT_BGM_LIST + 4 * w, bits as i32);
                }
                for (w, &bits) in v[3..].iter().enumerate() {
                    s.save.set_i32(piney_desktop::save::offset::DT_STR_LIST + 4 * w, bits as i32);
                }
                let (waves, strs) = audio.probe_add(&s);
                let w: Vec<String> = waves.iter().map(|i| i.to_string()).collect();
                let st: Vec<String> = strs.iter().map(|(i, v)| format!("[{i},{v}]")).collect();
                println!("[[{}],[{}]]", w.join(","), st.join(","));
            }
            "dec" => {
                let v = nums(&mut it);
                let s = piney_desktop::kanji::dec2sjis(v[0] as i32, v[1] as i32, v[2] as i32);
                let h: String = s.iter().map(|b| format!("{b:02x}")).collect();
                println!("\"{h}\"");
            }
            "audlist" => {
                let v = nums(&mut it);
                let mut req = Vec::new();
                let mut views = Views::default();
                let mut ctx = Ctx::new(View::default());
                let mut x = MailCtx {
                    ctx: &mut ctx,
                    fonts: &assets.fonts,
                    names: Names::default(),
                    save: &mut save,
                    req: &mut req,
                    views: &mut views,
                    count: 0,
                    frame_rate: 1,
                };
                audio.probe_reset(v[1] != 0);
                let mut out = Vec::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let mut words = l.split_whitespace();
                    words.next();
                    let p = nums(&mut words);
                    x.req.clear();
                    let s = audio.probe_move(&mut x, [p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32], v[0] as i32);
                    out.push(format!("[{},{},{},{},{},{}]", s.0, s.1, s.2, s.3, s.4, ses(x.req)));
                }
                println!("[{}]", out.join(","));
            }
            "acclen" => {
                let v = nums(&mut it);
                acces.probe_list(v[0] as usize);
                let p = acces.probe_length(v[1] as i32);
                let b: Vec<String> = p
                    .iter()
                    .map(|q| format!("[{},{},{},{}]", q[0].to_bits(), q[1].to_bits(), q[2].to_bits(), q[3].to_bits()))
                    .collect();
                println!("[{}]", b.join(","));
            }
            "walls" => {
                let v = nums(&mut it);
                let mut s = fresh(&assets);
                for (w, &bits) in v.iter().enumerate() {
                    s.save.set_i32(piney_desktop::save::offset::DT_WALLPAPER_LIST + 4 * w, bits as i32);
                }
                let l: Vec<String> = acces.probe_add(&s).iter().map(|i| i.to_string()).collect();
                println!("[{}]", l.join(","));
            }
            "acclist" => {
                let v = nums(&mut it);
                let mut req = Vec::new();
                let mut views = Views::default();
                let mut ctx = Ctx::new(View::default());
                let mut x = MailCtx {
                    ctx: &mut ctx,
                    fonts: &assets.fonts,
                    names: Names::default(),
                    save: &mut save,
                    req: &mut req,
                    views: &mut views,
                    count: 0,
                    frame_rate: 1,
                };
                acces.probe_list(v[0] as usize);
                let mut out = Vec::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let mut words = l.split_whitespace();
                    words.next();
                    let p = nums(&mut words);
                    x.req.clear();
                    let s = acces.probe_move(&mut x, [p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32]);
                    out.push(format!("[{},{},{},{},{},{}]", s.0, s.1, s.2, s.3, s.4, ses(x.req)));
                }
                println!("[{}]", out.join(","));
            }
            "length" => {
                let v = nums(&mut it);
                mailer.probe_list(v[0] as usize);
                let mut st = mailer.probe_state();
                st.0 = v[1] as i32;
                mailer.probe_set(st);
                let mut req = Vec::new();
                let mut views = Views::default();
                let mut ctx = Ctx::new(View::default());
                let mut x = MailCtx {
                    ctx: &mut ctx,
                    fonts: &assets.fonts,
                    names: Names::default(),
                    save: &mut save,
                    req: &mut req,
                    views: &mut views,
                    count: 0,
                    frame_rate: 1,
                };
                let (dy, sy) = mailer.probe_length(&mut x, v[2] as i32);
                println!("[{},{}]", dy.to_bits(), sy.to_bits());
            }
            "inbox" => {
                // inbox PARODY ORDER... ; STATE...  (order then states, split by ';')
                let rest: Vec<&str> = it.collect();
                let text = rest.join(" ");
                let (a, b) = text.split_once(';').unwrap();
                let order: Vec<i64> = nums(&mut a.split_whitespace());
                let states: Vec<i64> = nums(&mut b.split_whitespace());
                let mut st = fresh(&assets);
                st.save.set_u8(piney_data::save::offset::PARODY_FLAG, order[0] as u8);
                for (i, (&m, &s)) in order[1..].iter().zip(&states).enumerate() {
                    st.save.set_mail_order(i, m as i16);
                    st.save.set_mail(m as usize, s as u8);
                }
                let mut m = Mailer::new(&assets, &st).unwrap();
                m.probe_reset_inbox();
                let ret = m.add_mail_list(&mut st);
                let after: Vec<String> = order[1..].iter().map(|&n| st.save.mail(n as usize).to_string()).collect();
                let flags: Vec<String> = order[1..]
                    .iter()
                    .map(|&n| {
                        let (r, f) = m.probe_flags(n as usize);
                        format!("[{r},{f}]")
                    })
                    .collect();
                let list: Vec<String> = m.list.iter().map(|x| x.to_string()).collect();
                println!(
                    "{{\"ret\":{ret},\"list\":[{}],\"new\":{},\"after\":[{}],\"flags\":[{}]}}",
                    list.join(","),
                    m.new_mail_num,
                    after.join(","),
                    flags.join(",")
                );
            }
            "alpha" => {
                let v = nums(&mut it);
                println!("{}", cursor_alpha(v[0] as u32, v[1] as u32));
            }
            "kanji" => {
                let kt = match it.next().unwrap() {
                    "0" => Kt::SmallProportional,
                    "1" => Kt::SmallFixed,
                    "2" => Kt::LargeProportional,
                    _ => Kt::LargeFixed,
                };
                let f: Vec<f32> = (0..4).map(|_| it.next().unwrap().parse().unwrap()).collect();
                let hex = it.next().unwrap_or("");
                let bytes: Vec<u8> =
                    (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
                let view = LayerView { sx: f[0], sy: f[1], ox: f[2], oy: f[3], scissor: Scissor::FULL };
                let mut k = Kanji::new(kt, 128, 16);
                let quads = k.disp(&assets.fonts, &bytes, -1, &view, &Names::default());
                let q: Vec<String> = quads
                    .iter()
                    .map(|q| {
                        format!(
                            "[[{},{},{},{}],[{},{}],[{},{}],[{},{}],[{},{}],{}]",
                            q.rgba[0],
                            q.rgba[1],
                            q.rgba[2],
                            q.rgba[3],
                            q.uv0.0,
                            q.uv0.1,
                            q.xy0.0,
                            q.xy0.1,
                            q.uv1.0,
                            q.uv1.1,
                            q.xy1.0,
                            q.xy1.1,
                            q.shadow as i32
                        )
                    })
                    .collect();
                let hexs: String = k.tex.iter().map(|b| format!("{b:02x}")).collect();
                println!("{{\"clm\":{},\"tex\":\"{hexs}\",\"quads\":[{}]}}", k.clm, q.join(","));
            }
            "datascen" => println!("{}", data_probe::scenario(&assets, &mut lines)?),
            "savemenu" => data_probe::save_menu(&assets, &mut lines)?,
            "datapar" | "datacur" | "datacursor" | "databtn" => {
                println!("{}", data_probe::piece(&assets, cmd, &mut it)?)
            }
            "nerun" => println!("{}", name_probe::run(&assets, &mut it, &mut lines)?),
            _ => println!("null"),
        }
    }
    Ok(())
}

/// The Data screen's requests (`tools/test_desktop_data_rs.py`).
mod data_probe {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use piney_data::volume::Volume;
    use piney_desktop::anm::Ctx;
    use piney_desktop::assets::Assets;
    use piney_desktop::card::{self, MemoryCard, PortState};
    use piney_desktop::data::{Data, trace};
    use piney_desktop::mail::{MailCtx, Views};
    use piney_desktop::savesys::{INDEX_SIZE, INFO_SIZE};
    use piney_desktop::view::View;
    use piney_input::{Buttons, Pad};

    type Res<T> = Result<T, Box<dyn std::error::Error>>;

    /// One port of the in-memory card: what the scenario set, and the files
    /// by the game's paths.
    #[derive(Clone, Default)]
    struct SimPort {
        present: bool,
        ps2: bool,
        formatted: bool,
        full: bool,
        fail_write: bool,
        fail_sys: bool,
        fail_fmt: bool,
        dir: bool,
        files: BTreeMap<String, Vec<u8>>,
    }

    /// The card the test's Python side keeps too: ccMcard::CheckPort's
    /// answer from the flags, writes to slot files failing with FAILWRITE,
    /// to the index with FAILSYS.
    #[derive(Clone)]
    struct SimCard(Rc<RefCell<[SimPort; 2]>>, Volume);

    fn index_path(v: Volume) -> String {
        format!("/{0}/{0}", card::own_dir_name(v))
    }

    fn slot_path(v: Volume, slot: usize) -> String {
        format!("/{}/{}", card::own_dir_name(v), card::slot_file_name(slot))
    }

    impl SimCard {
        fn new(v: Volume) -> Self {
            SimCard(Rc::default(), v)
        }

        fn port(&self, port: i32) -> std::cell::RefMut<'_, SimPort> {
            std::cell::RefMut::map(self.0.borrow_mut(), |p| &mut p[port.clamp(0, 1) as usize])
        }
    }

    impl MemoryCard for SimCard {
        fn check_port(&mut self, port: i32) -> PortState {
            let p = self.port(port);
            if !p.present {
                PortState::NoCard
            } else if !p.ps2 {
                PortState::NotPs2
            } else if !p.formatted {
                PortState::Unformatted
            } else if p.dir {
                PortState::Ready
            } else if p.full {
                PortState::Full
            } else {
                PortState::NoDirectory
            }
        }

        fn read_index(&mut self, port: i32, _vol: i32) -> Option<Vec<u8>> {
            let p = self.port(port);
            p.files.get(&index_path(self.1)).map(|b| b[..b.len().min(INDEX_SIZE)].to_vec())
        }

        fn write_index(&mut self, port: i32, index: &[u8; INDEX_SIZE]) -> bool {
            let mut p = self.port(port);
            if p.fail_sys {
                // sceMcOpen created the file; the write failed.
                p.files.entry(index_path(self.1)).or_default();
                return false;
            }
            p.files.insert(index_path(self.1), index.to_vec());
            true
        }

        fn write_slot(&mut self, port: i32, slot: usize, data: &[u8]) -> bool {
            let mut p = self.port(port);
            if p.fail_write {
                p.files.entry(slot_path(self.1, slot)).or_default();
                return false;
            }
            p.files.insert(slot_path(self.1, slot), data.to_vec());
            true
        }

        fn format(&mut self, port: i32) -> bool {
            let mut p = self.port(port);
            if p.fail_fmt {
                return false;
            }
            p.formatted = true;
            p.dir = false;
            p.files.clear();
            true
        }

        fn make_dir(&mut self, port: i32) -> bool {
            let v = self.1;
            p_make_dir(&mut self.port(port), v)
        }
    }

    /// ccMcard::MakeDir: the directory, twelve slot files, the index.
    fn p_make_dir(p: &mut SimPort, v: Volume) -> bool {
        p.dir = true;
        if p.fail_write {
            p.files.entry(slot_path(v, 0)).or_default();
            return false;
        }
        for s in 0..12 {
            p.files.insert(slot_path(v, s), vec![0; piney_desktop::savesys::slot_size(v)]);
        }
        if p.fail_sys {
            p.files.entry(index_path(v)).or_default();
            return false;
        }
        p.files.insert(index_path(v), vec![0; INDEX_SIZE]);
        true
    }

    fn nums(w: &mut std::str::SplitWhitespace) -> Vec<i64> {
        w.map(
            |s| {
                if let Some(h) = s.strip_prefix("0x") {
                    i64::from_str_radix(h, 16).unwrap()
                } else {
                    s.parse().unwrap()
                }
            },
        )
        .collect()
    }

    fn unhex(h: &str) -> Vec<u8> {
        (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|c| format!("{c:02x}")).collect()
    }

    /// `datascen`: the frames' states and events, then the card's files.
    pub fn scenario<I: Iterator<Item = std::io::Result<String>>>(assets: &Assets, lines: &mut I) -> Res<String> {
        let card = SimCard::new(assets.volume);
        let mut data = Data::new(assets)?;
        data.set_card(Box::new(card.clone()));
        let mut save = super::fresh(assets);
        let mut views = Views::default();
        let mut rate = 1;
        let mut entered = false;
        let mut frames = Vec::new();
        for l in lines.by_ref() {
            let l = l?;
            let mut w = l.split_whitespace();
            match w.next() {
                Some("end") => break,
                Some("card") => {
                    let v = nums(&mut w);
                    let mut p = card.port(v[0] as i32);
                    (p.present, p.ps2, p.formatted, p.full) = (v[1] != 0, v[2] != 0, v[3] != 0, v[4] != 0);
                    (p.fail_write, p.fail_sys, p.fail_fmt) = (v[5] != 0, v[6] != 0, v[7] != 0);
                }
                Some("dir") => card.port(nums(&mut w)[0] as i32).dir = true,
                Some("file") => {
                    let port: i32 = w.next().unwrap().parse()?;
                    let name = w.next().unwrap().to_string();
                    let bytes = unhex(w.next().unwrap_or(""));
                    let mut p = card.port(port);
                    p.dir = true;
                    p.files.insert(format!("/{}/{name}", card::own_dir_name(assets.volume)), bytes);
                }
                Some("save") => {
                    let at: usize = w.next().unwrap().parse()?;
                    let b = unhex(w.next().unwrap_or(""));
                    save.save.bytes_mut()[at..at + b.len()].copy_from_slice(&b);
                }
                Some("sys") => {
                    let v = nums(&mut w);
                    data.set_card_position(v[0] as i32, v[1] as i32);
                }
                Some("rate") => rate = nums(&mut w)[0] as u32,
                Some("frame") => {
                    let v = nums(&mut w);
                    let b = |i: usize| Buttons(v[i] as u32);
                    let pad = Pad { direct: b(0), push: b(1), unpush: b(2), repeat: b(3), ..Pad::default() };
                    let mut req = Vec::new();
                    let mut ctx = Ctx::new(View::default());
                    let names = save.names();
                    let mut x = MailCtx {
                        ctx: &mut ctx,
                        fonts: &assets.fonts,
                        names,
                        save: &mut save,
                        req: &mut req,
                        views: &mut views,
                        count: v[4] as u32,
                        frame_rate: rate,
                    };
                    trace::start();
                    if !entered {
                        data.enter(&mut x);
                        entered = true;
                    }
                    let r = data.main(&mut x, &pad);
                    if r == -1 {
                        // Left: the next frame opens the screen again.
                        data.end_req();
                        entered = false;
                    } else {
                        data.draw_back(&mut x);
                    }
                    let ev = trace::stop();
                    let s = &data.save_sys;
                    let st = data.probe_state();
                    frames.push(format!(
                        "{{\"state\":[{},{},{},{},{},{},{},{},{},{}],\"ret\":{r},\"ev\":[{}]}}",
                        st[0],
                        st[1],
                        st[2],
                        st[3],
                        st[4],
                        st[5],
                        s.result,
                        s.proccess,
                        s.file_num,
                        s.port,
                        ev.join(",")
                    ));
                }
                _ => {}
            }
        }
        let mut files = Vec::new();
        for (port, p) in card.0.borrow().iter().enumerate() {
            for (path, b) in &p.files {
                let v = if b.iter().all(|&c| c == 0) { format!("zeros:{}", b.len()) } else { hex(b) };
                files.push(format!("\"{port}:{path}\":\"{v}\""));
            }
        }
        Ok(format!("{{\"frames\":[{}],\"files\":{{{}}}}}", frames.join(","), files.join(",")))
    }

    /// `savemenu`: the save menus after the staff roll (dtmenu.rs, menus 8
    /// and 9) over the card, frame by frame (`tools/test_desktop_savemenu_rs.py`).
    /// Lines: `frames N`, `open F` (openReqNum 8 at frame F), the card, save
    /// and `sys` lines of `datascen`, `pad F PUSH REPEAT`, `end`. Prints a
    /// line a frame (state, the menu's events, its requests), then the
    /// card's files.
    pub fn save_menu<I: Iterator<Item = std::io::Result<String>>>(assets: &Assets, lines: &mut I) -> Res<()> {
        use std::collections::HashMap;

        use piney_desktop::Request;
        use piney_desktop::dtmenu::{DtMenu, MenuCtx};
        use piney_desktop::savesys::SaveSys;

        let mut card = SimCard::new(assets.volume);
        let mut sys = SaveSys::new(assets.volume);
        let mut save = super::fresh(assets);
        let (mut n, mut open, mut rate) = (0usize, 0usize, 1u32);
        let mut script: HashMap<usize, (u32, u32)> = HashMap::new();
        for l in lines.by_ref() {
            let l = l?;
            let mut w = l.split_whitespace();
            match w.next() {
                Some("end") => break,
                Some("frames") => n = nums(&mut w)[0] as usize,
                Some("open") => open = nums(&mut w)[0] as usize,
                Some("rate") => rate = nums(&mut w)[0] as u32,
                Some("pad") => {
                    let v = nums(&mut w);
                    script.insert(v[0] as usize, (v[1] as u32, v[2] as u32));
                }
                Some("card") => {
                    let v = nums(&mut w);
                    let mut p = card.port(v[0] as i32);
                    (p.present, p.ps2, p.formatted, p.full) = (v[1] != 0, v[2] != 0, v[3] != 0, v[4] != 0);
                    (p.fail_write, p.fail_sys, p.fail_fmt) = (v[5] != 0, v[6] != 0, v[7] != 0);
                }
                Some("dir") => card.port(nums(&mut w)[0] as i32).dir = true,
                Some("file") => {
                    let port: i32 = w.next().unwrap().parse()?;
                    let name = w.next().unwrap().to_string();
                    let bytes = unhex(w.next().unwrap_or(""));
                    let mut p = card.port(port);
                    p.dir = true;
                    p.files.insert(format!("/{}/{name}", card::own_dir_name(assets.volume)), bytes);
                }
                Some("save") => {
                    let at: usize = w.next().unwrap().parse()?;
                    let b = unhex(w.next().unwrap_or(""));
                    save.save.bytes_mut()[at..at + b.len()].copy_from_slice(&b);
                }
                Some("sys") => {
                    let v = nums(&mut w);
                    (sys.port, sys.file_num) = (v[0] as i32, v[1] as i32);
                }
                _ => {}
            }
        }
        let mut menu = DtMenu::new(assets.volume, None, None);
        let names = save.names();
        for f in 1..n {
            if f == open {
                menu.open_req = 8;
            }
            // ccThSaveSys (priority 20) before the menu task; its first
            // frame only breathes.
            match menu.save_task {
                1 => menu.save_task = 2,
                2 if sys.running => sys.main_proccess(&mut card, &save.save),
                _ => {}
            }
            let (push, repeat) = script.get(&f).copied().unwrap_or((0, 0));
            let pad = Pad { push: Buttons(push), repeat: Buttons(repeat), ..Pad::default() };
            let mut req = Vec::new();
            let mut x = MenuCtx {
                save: &mut save,
                req: &mut req,
                pad: &pad,
                enable_reset: true,
                names: names.clone(),
                save_sys: Some(&mut sys),
            };
            menu.trace.clear();
            menu.frame(&mut x);
            let mut ctx = Ctx::new(View::default());
            menu.disp(&mut ctx, &assets.fonts, &names, rate);
            let r: Vec<String> = req
                .iter()
                .map(|q| match q {
                    Request::Se(s) => format!("[\"se\",{}]", s.0),
                    other => format!("[\"other\",\"{other:?}\"]"),
                })
                .collect();
            let ss = menu.save_state();
            let ss: Vec<String> = ss.iter().map(|v| v.to_string()).collect();
            println!(
                "{{\"f\":{f},\"st\":[{},{},{},{},{},{}],\"save\":[{}],\"sys\":[{},{},{},{},{}],\"ev\":[{}],\"req\":[{}]}}",
                menu.menu,
                menu.menu_next,
                menu.menu_status,
                menu.proccess,
                menu.alpha,
                menu.check_menu_type(),
                ss.join(","),
                sys.result,
                sys.proccess,
                sys.port,
                sys.file_num,
                i32::from(sys.running),
                menu.trace.join(","),
                r.join(",")
            );
        }
        let mut files = Vec::new();
        for (port, p) in card.0.borrow().iter().enumerate() {
            for (path, b) in &p.files {
                let v = if b.iter().all(|&c| c == 0) { format!("zeros:{}", b.len()) } else { hex(b) };
                files.push(format!("\"{port}:{path}\":\"{v}\""));
            }
        }
        println!("{{\"files\":{{{}}}}}", files.join(","));
        Ok(())
    }

    /// `datapar`, `datacur`, `datacursor`, `databtn`: one call's events.
    pub fn piece(assets: &Assets, cmd: &str, w: &mut std::str::SplitWhitespace) -> Res<String> {
        let mut data = Data::new(assets)?;
        let mut save = super::fresh(assets);
        let mut views = Views::default();
        let mut req = Vec::new();
        let mut ctx = Ctx::new(View::default());
        let words: Vec<&str> = w.collect();
        let v: Vec<i64> = words.iter().skip(if cmd == "datapar" { 1 } else { 0 }).map(|s| s.parse().unwrap()).collect();
        let (count, rate) = match cmd {
            "datacur" => (v[2], v[3]),
            "datacursor" => (v[5], v[6]),
            "databtn" => (v[2], v[3]),
            _ => (0, 1),
        };
        let names = save.names();
        let mut x = MailCtx {
            ctx: &mut ctx,
            fonts: &assets.fonts,
            names,
            save: &mut save,
            req: &mut req,
            views: &mut views,
            count: count as u32,
            frame_rate: rate as u32,
        };
        data.probe_set_data(&mut x);
        trace::start();
        match cmd {
            "datapar" => {
                let mut rec = [0u8; INFO_SIZE];
                rec.copy_from_slice(&unhex(words[0]));
                data.probe_set_save_par(&mut x, &rec, v[0] as usize, v[1] as i32);
            }
            "datacur" => data.probe_draw_cur(&mut x, v[0] as i32, v[1] as u32),
            "datacursor" => data.probe_cursor(&mut x, v[0] as i32, v[1] as i32, v[2] != 0, v[3] as i32, v[4] as u32),
            _ => data.probe_button(&mut x, v[0] as i32, v[1] as i32),
        }
        Ok(format!("[{}]", trace::stop().join(",")))
    }
}

/// The name entry's requests (`tools/test_desktop_name_rs.py`).
mod name_probe {
    use piney_desktop::assets::Assets;
    use piney_desktop::name_entry::{NameEntry, SPC_MAX_HP, SPC_MAX_SP, trace};
    use piney_desktop::save::offset;
    use piney_desktop::{Request, Se};
    use piney_input::{Buttons, Pad};

    type Res<T> = Result<T, Box<dyn std::error::Error>>;

    fn cstr(b: &[u8]) -> String {
        b.iter().take_while(|&&c| c != 0).map(|c| format!("{c:02x}")).collect()
    }

    /// `nerun`: each frame's state, sounds, return and (where asked) draw
    /// calls, then the names in the save.
    pub fn run<I: Iterator<Item = std::io::Result<String>>>(
        assets: &Assets,
        it: &mut std::str::SplitWhitespace,
        lines: &mut I,
    ) -> Res<String> {
        let v: Vec<i64> = it.map(|s| s.parse().unwrap()).collect();
        let mut state = super::fresh(assets);
        state.save.set_i16(offset::SPC_PARAM + SPC_MAX_HP, v[0] as i16);
        state.save.set_i16(offset::SPC_PARAM + SPC_MAX_SP, v[1] as i16);
        state.save.set_u8(offset::NEW_GAME_FLAG, v[2] as u8);
        let mut ne = NameEntry::of(assets.volume, &assets.archive, &state)?;
        let mut frames = Vec::new();
        for l in lines.by_ref() {
            let l = l?;
            let mut w = l.split_whitespace();
            match w.next() {
                Some("end") => break,
                Some("pad") => {}
                _ => continue,
            }
            let p: Vec<u32> = w.map(|s| s.parse().unwrap()).collect();
            let pad = Pad { push: Buttons(p[0]), unpush: Buttons(p[1]), repeat: Buttons(p[2]), ..Pad::default() };
            let was = ne.done();
            let draw = p[3] != 0;
            if draw {
                trace::start();
            }
            ne.step(&pad, &mut state);
            let ev = if draw { format!(",\"ev\":[{}]", trace::stop().join(",")) } else { String::new() };
            let se: Vec<String> = ne
                .take_requests()
                .iter()
                .filter_map(|r| match r {
                    Request::Se(Se(n)) => Some(n.to_string()),
                    _ => None,
                })
                .collect();
            let g = ne.logic();
            let st = [
                i32::from(g.main_act),
                i32::from(g.cur_act),
                g.x,
                g.y,
                i32::from(g.name_act),
                i32::from(g.name_num),
                i32::from(g.sw),
                i32::from(g.tlans),
                g.trans,
                g.err,
                i32::from(g.dialog),
                i32::from(g.win_act),
                g.info_count,
                i32::from(g.all_lock),
                g.play_lock,
                g.move_lock,
                i32::from(g.str_max),
                i32::from(g.q),
                i32::from(g.now_mode),
                g.win_trans,
                g.no,
                g.disp_cur.0,
                g.disp_cur.1,
                i32::from(g.push_cnt),
                i32::from(g.repeat_count),
                g.time_count,
                g.switch,
                i32::from(g.name_flg),
                i32::from(g.temp_main_act),
                i32::from(g.tmp_win_act),
            ];
            let st: Vec<String> = st.iter().map(|x| x.to_string()).collect();
            let strs = [
                &g.sur_name[..],
                &g.game_name,
                &g.temp_name,
                &g.cbuf,
                &g.blockbuf,
                &g.true_sur_name,
                &g.entry_name,
                &g.entry_game_name,
            ]
            .map(|b| format!("\"{}\"", cstr(b)));
            frames.push(format!(
                "{{\"st\":[{}],\"str\":[{}],\"se\":[{}],\"ret\":{}{ev}}}",
                st.join(","),
                strs.join(","),
                se.join(","),
                i32::from(ne.done() && !was)
            ));
        }
        Ok(format!(
            "{{\"frames\":[{}],\"names\":[\"{}\",\"{}\"]}}",
            frames.join(","),
            cstr(state.save.name()),
            cstr(state.save.real_name())
        ))
    }
}
