//! Answers `tools/test_demo_rs.py`: runs pieces of the title's logic on states
//! it is given and prints one JSON line per request, so the test can run the
//! same states through the game's own code in eemu (`demo_probe ISO <
//! requests`, built with the `trace` feature). Every request starts from a
//! fresh `ccOpening_Control` after `SetNeutral`; numbers may be hex, floats
//! travel as their bit patterns. The requests name the function they run
//! (`movecur`, `switch`, `neutral`, `fade`, `animate`, `play*`, `newgame`,
//! `draw`, `main`, `logo`, `boot`, `thread`, `load`, `loadgame`, `lit`, ...).

use std::collections::HashMap;
use std::io::BufRead;
use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_demo::card::SaveSysTask;
use piney_demo::dataload::{self, DataLoad, Draw, Keys};
use piney_demo::dialog::DialogAssets;
use piney_demo::fade::{ScFade, colour};
use piney_demo::newgame::{NewGameTables, load_game, new_game};
use piney_demo::opening::{Env, Opening};
use piney_demo::seam::{MenuFrame, SystemMenu};
use piney_demo::{Control, Phase, Request, task};
use piney_desktop::anm::{Ctx, trace};
use piney_desktop::assets::SceneFile;
use piney_desktop::card::FilesCard;
use piney_desktop::kanji::Fonts;
use piney_desktop::savesys::{OPERATE_TITLE, SaveSys};
use piney_desktop::view::View;
use piney_input::{Buttons, Pad};

/// A system menu stuck at one `CheckMenuType`, recording the requests.
#[derive(Default)]
struct Menu {
    ty: i32,
    request: i16,
    display: Option<bool>,
}

impl SystemMenu for Menu {
    fn request(&mut self, n: i16) {
        self.request = n;
    }

    fn set_display(&mut self, on: bool) {
        self.display = Some(on);
    }

    fn menu_type(&self) -> i32 {
        self.ty
    }

    fn frame(&mut self, _x: &mut MenuFrame) {}
}

/// The probe's disc's volume, once read.
static DISC: std::sync::OnceLock<piney_data::volume::Volume> = std::sync::OnceLock::new();

const OK: u32 = 0x40;
const CANCEL: u32 = 0x20;

fn nums(it: &mut std::str::SplitWhitespace) -> Vec<i64> {
    it.map(
        |s| if let Some(h) = s.strip_prefix("0x") { i64::from_str_radix(h, 16).unwrap() } else { s.parse().unwrap() },
    )
    .collect()
}

fn calls(c: &[(&'static str, &'static str, String)]) -> String {
    let v: Vec<String> = c.iter().map(|(op, l, a)| format!("[\"{op}\",\"{l}\",\"{a}\"]")).collect();
    format!("[{}]", v.join(","))
}

fn ses(req: &[Request]) -> String {
    let v: Vec<String> = req
        .iter()
        .filter_map(|r| match r {
            Request::Se(s) => Some(s.to_string()),
            _ => None,
        })
        .collect();
    format!("[{}]", v.join(","))
}

fn fade_state(f: &ScFade) -> String {
    let v: Vec<String> = f
        .elm
        .iter()
        .map(|e| format!("[{},{},{},{},{},{},{}]", e.status, e.cnt, e.tcnt, e.tcnt1, e.tcnt2, e.col0, e.col1))
        .collect();
    format!("[{}]", v.join(","))
}

fn pad(push: u32, unpush: u32, repeat: u32) -> Pad {
    Pad { push: Buttons(push), unpush: Buttons(unpush), repeat: Buttons(repeat), ..Pad::default() }
}

fn tps(o: &Opening) -> String {
    let mut v: Vec<String> = o.a_ico.iter().map(|a| a.localtp.to_bits().to_string()).collect();
    v.extend(o.a_app.iter().map(|a| a.localtp.to_bits().to_string()));
    v.push(o.a_win.localtp.to_bits().to_string());
    format!("[{}]", v.join(","))
}

/// A fresh control of volume `vol` (its `title<vol>`) with SetNeutral
/// run, and the trace started.
fn fresh(files: &[Rc<SceneFile>], vol: i16, paro: bool, fwd: HashMap<&'static str, bool>) -> Opening {
    let mut o = Opening::new(files[(vol.clamp(1, 4) - 1) as usize].clone());
    o.now_vol = vol;
    o.paro_flg = paro;
    trace::start(HashMap::new());
    o.set_neutral();
    trace::stop();
    trace::start(fwd);
    o
}

/// The world `Env` points into.
struct World {
    ctx: Ctx,
    req: Vec<Request>,
    fade: ScFade,
    menu: Menu,
    sys: SaveSys,
    parody: bool,
}

impl World {
    fn new(menu_type: i32) -> Self {
        World {
            ctx: Ctx::new(View::default()),
            req: Vec::new(),
            fade: ScFade::default(),
            menu: Menu { ty: menu_type, ..Menu::default() },
            // The disc's own ccSaveSys (its volumeNum).
            sys: SaveSys::new(DISC.get().copied().unwrap_or(piney_data::volume::Volume::Inf)),
            parody: false,
        }
    }

    fn env<'a>(&'a mut self, pad: &'a Pad) -> Env<'a> {
        Env {
            ctx: &mut self.ctx,
            pad,
            ok: OK,
            cancel: CANCEL,
            count: 0,
            main_vol: 0x3fff,
            req: &mut self.req,
            fade: &mut self.fade,
            menu: &mut self.menu,
            sys: &mut self.sys,
            parody_flag: &mut self.parody,
            dialog: None,
        }
    }
}

/// `ccThDemo`'s control with `PlayBootMemCard` and `Main` scripted.
struct Script {
    boot: Vec<i32>,
    main: Vec<i32>,
    logo_act: i32,
    events: Vec<String>,
}

impl Control for Script {
    fn play_boot_mem_card(&mut self, _env: &mut Env) -> i32 {
        self.events.push("[\"boot\"]".into());
        if self.boot.is_empty() { 1 } else { self.boot.remove(0) }
    }

    fn main(&mut self, _env: &mut Env) -> i32 {
        self.events.push("[\"main\"]".into());
        if self.main.is_empty() { 0 } else { self.main.remove(0) }
    }

    fn logo_act(&self) -> i32 {
        self.logo_act
    }

    fn set_logo_act(&mut self, act: i32) {
        self.logo_act = act;
    }

    fn parody_item(&self) -> bool {
        false
    }
}

fn request_event(r: &Request) -> Option<String> {
    Some(match r {
        Request::SqStop(n) => format!("[\"sqstop\",{n}]"),
        Request::SqPlay(n) => format!("[\"sq\",{n}]"),
        Request::MainVolume(v) => format!("[\"vol\",{v}]"),
        Request::Movie { path, audio } => format!("[\"mpeg\",\"{path}\",{}]", *audio as i32),
        Request::Stream { num, .. } => format!("[\"stream\",{num}]"),
        Request::NewGame { .. } => "[\"newgame\",0]".into(),
        Request::LoadGame => "[\"loadgame\"]".into(),
        Request::ConvGame => "[\"convgame\"]".into(),
        Request::SqFade { seq, volume, time, mode } => format!("[\"sqfade\",{seq},{volume},{time},{mode}]"),
        Request::ChangeMode { num, sf } => format!("[\"change\",{num},{sf}]"),
        _ => return None,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path)?;
    let disc = iso.volume()?;
    let _ = DISC.set(disc);
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let file = Rc::new(SceneFile::read(&archive, piney_demo::names::TITLE_FILE)?);
    // Each volume's title (Infection's archive has all four), and the
    // volume the requests after a `vol N` line run as.
    let mut files = vec![file.clone()];
    for v in [piney_data::volume::Volume::Mut, piney_data::volume::Volume::Out, piney_data::volume::Volume::Qua] {
        files.push(Rc::new(SceneFile::read(&archive, piney_demo::names::title_file(v))?));
    }
    let mut vol: i16 = 1;
    let tables = NewGameTables::of(piney_data::volume::Volume::Inf);
    let font = piney_data::ccs::Ccs::parse(archive.inflate_named(piney_desktop::assets::FONT_FILE)?)?;
    let (_, cluts) = piney_data::texture::read(&font)?;
    let clut = font.find_object("CLT_xasc00").and_then(|o| cluts.get(&o)).ok_or("CLT_xasc00")?;
    // The load screens' texts and fonts: the disc's own.
    let fonts = Fonts::of(disc, clut.colours.iter().map(|c| piney_draw::Rgba(*c)).collect());
    let assets = DialogAssets::read(disc, fonts, &files[(disc.number() - 1) as usize])?;
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    while let Some(line) = lines.next() {
        let line = line?;
        let mut it = line.split_whitespace();
        let Some(cmd) = it.next() else { continue };
        match cmd {
            "vol" => {
                vol = nums(&mut it)[0] as i16;
            }
            "movecur" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[5] != 0, HashMap::new());
                (o.nut_cur_no, o.push_flg, o.push_cnt, o.repeat_count, o.demo_count) =
                    (v[0] as i32, v[1] as i16, v[2] as i16, v[3] as i16, v[4] as i32);
                trace::stop();
                let mut out = Vec::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let p = nums(&mut l.split_whitespace().skip(1).collect::<Vec<_>>().join(" ").split_whitespace());
                    trace::start(HashMap::new());
                    let mut req = Vec::new();
                    o.move_cur_nut(&pad(p[0] as u32, p[1] as u32, p[2] as u32), &mut req);
                    let c = trace::stop();
                    out.push(format!(
                        "[{},{},{},{},{},{},{}]",
                        o.nut_cur_no,
                        o.push_flg,
                        o.push_cnt,
                        o.repeat_count,
                        o.demo_count,
                        ses(&req),
                        calls(&c)
                    ));
                }
                println!("[{}]", out.join(","));
            }
            "switch" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[1] != 0, HashMap::new());
                o.switch_cur(v[0] as i32);
                println!("{}", calls(&trace::stop()));
            }
            "neutral" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[3] != 0, HashMap::new());
                o.nut_act = v[0] as i32;
                o.nut_cur_no = v[1] as i32;
                let p = pad(v[2] as u32, 0, 0);
                let mut w = World::new(-1);
                o.play_neutral(&mut w.env(&p));
                let c = trace::stop();
                println!(
                    "[{},{},{},{},{},{}]",
                    o.nut_act,
                    o.main_act,
                    o.nut_cur_no,
                    o.flash_flg as i32,
                    ses(&w.req),
                    calls(&c)
                );
            }
            "change" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, false, HashMap::new());
                trace::stop();
                o.main_act = 0;
                o.change_main_act(v[0] as i32);
                println!("[{},{}]", o.main_act, o.flash_flg as i32);
            }
            "movecount" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, false, HashMap::new());
                trace::stop();
                (o.main_act, o.demo_count) = (v[0] as i32, v[1] as i32);
                let mut f = ScFade::default();
                o.move_count(&mut f);
                println!("[{},{},{},{}]", o.main_act, o.demo_count, o.start_mode, fade_state(&f));
            }
            "fade" => {
                let mut f = ScFade::default();
                let mut out = Vec::new();
                for op in it {
                    let a: Vec<i64> = op.split(':').skip(1).map(|x| x.parse().unwrap()).collect();
                    match op.split(':').next().unwrap() {
                        "flash" => {
                            f.entry_flash(a[0] as i16, a[1] as u32);
                        }
                        "flash3" => {
                            f.entry_flash3(a[0] as i16, a[1] as i16, a[2] as i16, a[3] as u32);
                        }
                        "fadein" => {
                            f.entry_fade(a[0] as i16, a[1] as u32, a[2] as u32);
                        }
                        "send" => {
                            for _ in 0..a[0] {
                                let drawn: Vec<String> = f
                                    .advance()
                                    .iter()
                                    .map(|&(c0, c1, cnt, t)| {
                                        let c = colour(c0, c1, cnt, t);
                                        format!("[{},{},{},{}]", c[0], c[1], c[2], c[3])
                                    })
                                    .collect();
                                out.push(format!("[[{}],{}]", drawn.join(","), fade_state(&f)));
                            }
                        }
                        _ => panic!("fade op {op}"),
                    }
                }
                println!("[{}]", out.join(","));
            }
            "transp" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[8] != 0, HashMap::new());
                trace::stop();
                (o.main_act, o.dat_act, o.opt_act, o.trans_wait) = (v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32);
                o.transp = f32::from_bits(v[4] as u32);
                o.transp_ico = f32::from_bits(v[5] as u32);
                o.transp_win = f32::from_bits(v[6] as u32);
                for a in o.a_ico.iter_mut().chain(o.a_app.iter_mut()) {
                    a.localtp = 1.0;
                }
                o.a_win.localtp = 1.0;
                let menu = Menu { ty: v[7] as i32, ..Menu::default() };
                o.all_transparency(&menu);
                println!(
                    "[{},{},{},{},{}]",
                    o.transp.to_bits(),
                    o.transp_ico.to_bits(),
                    o.transp_win.to_bits(),
                    o.trans_wait,
                    tps(&o)
                );
            }
            "animate" => {
                let v: Vec<String> = it.map(String::from).collect();
                let n = |i: usize| -> i64 {
                    let s = &v[i];
                    if let Some(h) = s.strip_prefix("0x") {
                        i64::from_str_radix(h, 16).unwrap()
                    } else {
                        s.parse().unwrap()
                    }
                };
                let labels: Vec<&'static str> =
                    v[5..].iter().map(|s| &*Box::leak(s.clone().into_boxed_str())).collect();
                let fwd = labels.iter().map(|&l| (l, true)).collect();
                let mut o = fresh(&files, vol, n(3) != 0, fwd);
                (o.main_act, o.dat_act, o.opt_act) = (n(0) as i32, n(1) as i32, n(2) as i32);
                o.rot_ico = [0.0, f32::from_bits(n(4) as u32), 0.0];
                (o.ne_sw, o.dat_sw, o.opt_sw) = (7, 7, 7);
                o.all_animate();
                let c = trace::stop();
                println!("[{},{},{},{},{}]", o.ne_sw, o.dat_sw, o.opt_sw, o.rot_ico[1].to_bits(), calls(&c));
            }
            "setn" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[2] != 0, HashMap::new());
                o.nut_lock = v[0] != 0;
                o.nut_cur_no = v[1] as i32;
                o.transp = f32::from_bits(v[3] as u32);
                o.transp_ico = f32::from_bits(v[4] as u32);
                o.main_act = 0;
                o.demo_count = 99;
                o.set_neutral();
                let c = trace::stop();
                println!("[{},{},{},{},{}]", o.main_act, o.demo_count, o.nut_lock as i32, tps(&o), calls(&c));
            }
            "setnew" | "setparo" => {
                let mut o = fresh(&files, vol, false, HashMap::new());
                o.main_act = 0;
                if cmd == "setnew" {
                    o.set_new_game();
                } else {
                    o.set_parody_game();
                }
                println!("[{},{}]", o.main_act, calls(&trace::stop()));
            }
            "setload" | "setopt" | "setnext" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[1] != 0, HashMap::new());
                o.main_act = 0;
                o.trans_wait = 9;
                o.opt_sw = 9;
                if cmd == "setload" {
                    o.dat_act = v[0] as i32;
                    o.set_data_load();
                } else if cmd == "setnext" {
                    o.set_next_data(v[0] as i32);
                } else {
                    o.set_option(v[0] as i32);
                }
                let c = trace::stop();
                println!("[{},{},{},{}]", o.main_act, o.trans_wait, o.opt_sw, calls(&c));
            }
            "playload" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[5] != 0, HashMap::new());
                (o.dat_act, o.dat_sw, o.flash_count, o.load_sw) = (v[0] as i32, v[1] as i32, v[2] as i32, v[3] != 0);
                (o.main_act, o.start_mode, o.mask, o.transp, o.transp_ico) = (7, 0, 0, 0.5, 0.25);
                // DataLoad_Control::Main_Control made to return RESULT: a
                // cancel push on the slot choice (-1), a load flagged in
                // LoadData (1), nothing (0).
                let p = match v[4] {
                    -1 => pad(CANCEL, 0, CANCEL),
                    _ => Pad::default(),
                };
                if v[4] == 1 {
                    o.data_load.data.slot_proc = 3;
                    o.data_load.data.load_flg = true;
                }
                let mut w = World::new(-1);
                o.play_data_load(&mut w.env(&p));
                let c = trace::stop();
                println!(
                    "[{},{},{},{},{},{},{},{},{},{},{}]",
                    o.dat_act,
                    o.dat_sw,
                    o.flash_count,
                    o.load_sw as i32,
                    o.main_act,
                    o.start_mode,
                    o.mask,
                    o.transp.to_bits(),
                    o.transp_ico.to_bits(),
                    fade_state(&w.fade),
                    calls(&c)
                );
            }
            "playnext" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[5] != 0, HashMap::new());
                (o.next_act, o.next_sw, o.flash_count, o.load_sw) = (v[0] as i32, v[1] as i32, v[2] as i32, v[3] != 0);
                (o.main_act, o.start_mode, o.mask, o.transp, o.transp_ico) = (11, 0, 0, 0.5, 0.25);
                // NextDataLoad_Control::Main_Control made to return RESULT,
                // as for playload.
                let p = match v[4] {
                    -1 => pad(CANCEL, 0, CANCEL),
                    _ => Pad::default(),
                };
                if v[4] == 1 {
                    o.next_load.data.slot_proc = 3;
                    o.next_load.data.load_flg = true;
                }
                let mut w = World::new(-1);
                o.play_next_data(&mut w.env(&p));
                let c = trace::stop();
                println!(
                    "[{},{},{},{},{},{},{},{},{},{},{}]",
                    o.next_act,
                    o.next_sw,
                    o.flash_count,
                    o.load_sw as i32,
                    o.main_act,
                    o.start_mode,
                    o.mask,
                    o.transp.to_bits(),
                    o.transp_ico.to_bits(),
                    fade_state(&w.fade),
                    calls(&c)
                );
            }
            "playopt" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[4] != 0, HashMap::new());
                (o.opt_act, o.opt_sw) = (v[0] as i32, v[1] as i32);
                (o.main_act, o.mask, o.transp, o.transp_ico, o.trans_wait) = (9, 0, 0.5, 0.25, 5);
                let p = pad(v[3] as u32, 0, 0);
                let mut w = World::new(v[2] as i32);
                o.play_option(&mut w.env(&p));
                let c = trace::stop();
                println!(
                    "[{},{},{},{},{},{},{},{},{}]",
                    o.opt_act,
                    o.opt_sw,
                    o.main_act,
                    o.mask,
                    o.transp.to_bits(),
                    o.transp_ico.to_bits(),
                    o.trans_wait,
                    w.menu.request,
                    calls(&c)
                );
            }
            "playnew" | "playparo" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, false, HashMap::new());
                trace::stop();
                (o.ne_sw, o.main_act, o.new_game_sw, o.start_mode) = (v[0] as i32, 99, 9, 9);
                let p = Pad::default();
                let mut w = World::new(-1);
                if cmd == "playnew" {
                    o.play_new_game();
                } else {
                    o.play_parody_game(&mut w.env(&p));
                }
                println!("[{},{},{},{}]", o.main_act, o.new_game_sw, o.start_mode, w.parody as i32);
            }
            "main" => {
                let v: Vec<String> = it.map(String::from).collect();
                let n = |i: usize| -> i64 { v[i].parse().unwrap() };
                let labels: Vec<&'static str> =
                    v[6..].iter().map(|s| &*Box::leak(s.clone().into_boxed_str())).collect();
                let mut o = fresh(&files, vol, false, labels.iter().map(|&l| (l, true)).collect());
                (o.main_act, o.mask, o.nut_act, o.dat_act, o.opt_act) =
                    (n(0) as i32, n(1) as i32, n(3) as i32, n(4) as i32, n(5) as i32);
                o.start_mode = 9;
                let p = pad(n(2) as u32, 0, 0);
                let mut w = World::new(1);
                let r = o.main(&mut w.env(&p));
                let c = trace::stop();
                println!(
                    "[{r},{},{},{},{},{},{},{},{},{}]",
                    o.main_act,
                    o.mask,
                    o.nut_act,
                    o.dat_act,
                    o.opt_act,
                    o.demo_count,
                    o.nut_cur_no,
                    ses(&w.req),
                    calls(&c)
                );
            }
            "newgame" => {
                // newgame SW SAVE_VA HEX: ccSaveData::NewGame on that save.
                let v: Vec<&str> = it.collect();
                let sw: i32 = v[0].parse()?;
                let va = u32::from_str_radix(v[1].trim_start_matches("0x"), 16)?;
                let bytes: Vec<u8> =
                    (0..v[2].len() / 2).map(|i| u8::from_str_radix(&v[2][2 * i..2 * i + 2], 16).unwrap()).collect();
                let mut save = piney_data::save::SaveData::from_bytes(&bytes)?;
                let names = new_game(&mut save, sw, &tables, va);
                let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
                let list = |l: &[Vec<u8>]| l.iter().map(|n| format!("\"{}\"", hex(n))).collect::<Vec<_>>().join(",");
                println!("[\"{}\",[{}],[{}]]", hex(save.bytes()), list(&names.spc), list(&names.ccs));
            }
            "consts" => {
                // The boot questions' texts, Infection's (the harness's).
                use piney_data::tables::{sjis::encode, title};
                let hex = |s: &str| encode(s).iter().map(|x| format!("{x:02x}")).collect::<String>();
                let msg = |n: usize| {
                    let lines = title::of(piney_data::volume::Volume::Inf).save_sys_msg[n].unwrap_or_default();
                    lines.iter().map(|s| format!("\"{}\"", hex(s))).collect::<Vec<_>>().join(",")
                };
                println!(
                    "[[{}],[{}],\"{}\",\"{}\",\"{}\"]",
                    msg(0x28),
                    msg(0x29),
                    hex(*title::YES),
                    hex(*title::NO),
                    hex(*title::HIGHLIGHT)
                );
            }
            "draw" => {
                let v = nums(&mut it);
                let mut o = fresh(&files, vol, v[1] != 0, HashMap::new());
                o.main_act = v[0] as i32;
                let mut ctx = Ctx::new(View::default());
                o.all_draw(&mut ctx);
                println!("{}", calls(&trace::stop()));
            }
            "logo" => {
                let v = nums(&mut it);
                let mut s = Script { boot: Vec::new(), main: Vec::new(), logo_act: v[0] as i32, events: Vec::new() };
                let p = Pad::default();
                let mut w = World::new(-1);
                let mut skipped = v[1] != 0;
                let mut phase = task(&mut s, Phase::Logo, &mut w.env(&p), &mut skipped, true);
                let mut breaths = 0;
                let mut ret = 0;
                if phase == Phase::Movie {
                    phase = task(&mut s, Phase::Movie, &mut w.env(&p), &mut skipped, true);
                    while let Phase::LogoWait(_) = phase {
                        breaths += 1;
                        phase = task(&mut s, phase, &mut w.env(&p), &mut skipped, true);
                    }
                } else if phase == Phase::LogoDone {
                    ret = 1;
                }
                let ev: Vec<String> = w.req.iter().filter_map(request_event).collect();
                println!("[{},{},{},[{}]]", s.logo_act, ret, breaths, ev.join(","));
            }
            "boot" => {
                let mut o = fresh(&files, vol, false, HashMap::new());
                trace::stop();
                let mut w = World::new(-1);
                // The test starts saveSys->result at 0.
                w.sys.result = 0;
                let mut out = Vec::new();
                for l in lines.by_ref() {
                    let l = l?;
                    if l.trim() == "end" {
                        break;
                    }
                    let p = nums(&mut l.split_whitespace().skip(1).collect::<Vec<_>>().join(" ").split_whitespace());
                    if p[0] != -99 {
                        w.sys.result = p[0] as u32;
                    }
                    trace::start(HashMap::new());
                    let pd = pad(p[1] as u32, p[2] as u32, p[3] as u32);
                    w.req.clear();
                    let r = o.play_boot_mem_card(&mut w.env(&pd));
                    let c = trace::stop();
                    out.push(format!(
                        "[{r},{},{},{},{},{},{},{},{},{}]",
                        o.boot_act,
                        o.err_flg,
                        o.boot_mem.state,
                        o.boot_mem.result,
                        o.boot_mem.data.dialog,
                        w.sys.result as i32,
                        o.boot_mem.shown.map_or(-1, i64::from),
                        ses(&w.req),
                        calls(&c)
                    ));
                }
                println!("[{}]", out.join(","));
            }
            "thread" => {
                // thread RESET SKIPS BOOT / MAIN, lists as comma-separated.
                let v: Vec<&str> = it.collect();
                let list = |s: &str| -> Vec<i32> {
                    if s == "-" { Vec::new() } else { s.split(',').map(|x| x.parse().unwrap()).collect() }
                };
                let first_boot = v[0] == "0";
                let mut skips = list(v[1]);
                let mut s = Script { boot: list(v[2]), main: list(v[3]), logo_act: 0, events: Vec::new() };
                let frames: usize = v[4].parse().unwrap();
                let p = Pad::default();
                let mut w = World::new(-1);
                let mut phase = Phase::Start;
                let mut skipped = false;
                let mut out: Vec<String> = Vec::new();
                for _ in 0..frames {
                    phase = task(&mut s, phase, &mut w.env(&p), &mut skipped, first_boot);
                    for e in s.events.drain(..) {
                        out.push(e);
                    }
                    for r in w.req.drain(..) {
                        if let Some(e) = request_event(&r) {
                            out.push(e);
                        }
                        if matches!(r, Request::Movie { .. }) && !skips.is_empty() {
                            skipped = skips.remove(0) != 0;
                        }
                    }
                    let d = w.menu.display.map_or(-1, |b| b as i32);
                    out.push(format!("[\"breath\",{d}]"));
                }
                println!("[{}]", out.join(","));
            }
            cmd @ ("load" | "loadnext") => {
                // The disc's own LOAD, or its CONVERT (NextDataLoad_Control
                // over the card's saves of the volume before); the save
                // after it as the disc's slot file.
                let v: Vec<&str> = it.collect();
                let mut card = FilesCard::slot1(disc, v[0]);
                let mut sys = SaveSys::new(disc);
                (sys.port, sys.file_num) = (v[1].parse()?, v[2].parse()?);
                let mut dl = if cmd == "load" { DataLoad::new() } else { DataLoad::new_next() };
                if cmd == "load" {
                    dl.init(&mut sys);
                } else {
                    dl.init_next(&mut sys);
                }
                // The title's save before the load: the port's fresh one.
                let mut save = piney_desktop::SaveState::fresh_with(&piney_desktop::InitText::of(disc)).save;
                let out = load_frames(&mut lines, &mut dl, &mut card, &mut sys, &mut save, &assets)?;
                let hex: String = save.slot_bytes(disc).iter().map(|b| format!("{b:02x}")).collect();
                println!("{{\"frames\":[{}],\"save\":\"{hex}\"}}", out.join(","));
            }
            "loadgame" => {
                let v: Vec<&str> = it.collect();
                let va = u32::from_str_radix(v[0].trim_start_matches("0x"), 16)?;
                let bytes: Vec<u8> =
                    (0..v[1].len() / 2).map(|i| u8::from_str_radix(&v[1][2 * i..2 * i + 2], 16).unwrap()).collect();
                let mut save = piney_data::save::SaveData::from_bytes(&bytes)?;
                let names = load_game(&mut save, &tables, va);
                let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
                let ccs: Vec<String> = names.ccs.iter().map(|n| format!("\"{}\"", hex(n))).collect();
                println!("[\"{}\",[{}]]", hex(save.bytes()), ccs.join(","));
            }
            "lit" => {
                let frames = nums(&mut it);
                println!("{}", lit(&iso_path, &archive, &frames)?);
            }
            other => panic!("unknown request {other}"),
        }
    }
    Ok(())
}

/// The load screen's frames up to an `end` line, one `frame PUSH UNPUSH
/// REPEAT COUNT` line each: `ccSaveSys`'s task (started by the title,
/// as the boot check leaves it), then `Main_Control`; each frame's state,
/// texts, packets and sounds as JSON.
fn load_frames(
    lines: &mut impl Iterator<Item = std::io::Result<String>>,
    dl: &mut DataLoad,
    card: &mut FilesCard,
    sys: &mut SaveSys,
    save: &mut piney_data::save::SaveData,
    assets: &DialogAssets,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    sys.start_req(OPERATE_TITLE);
    (sys.result, sys.proccess) = (1, 1);
    let mut task = SaveSysTask::default();
    let mut out = Vec::new();
    for l in lines.by_ref() {
        let l = l?;
        if l.trim() == "end" {
            break;
        }
        let p = nums(&mut l.split_whitespace().skip(1).collect::<Vec<_>>().join(" ").split_whitespace());
        task.frame(sys, card, save);
        let (ok, cancel) = (u32::from(save.assign_pad_ok()), u32::from(save.assign_pad_cancel()));
        let pd = pad(p[0] as u32, p[1] as u32, p[2] as u32);
        let mut ctx = Ctx::new(View::default());
        let mut req = Vec::new();
        dataload::trace::start();
        let mut d = Draw { ctx: &mut ctx, assets: Some(assets), count: p[3] as u32, frame_rate: 1 };
        let r = dl.main_control(&mut d, Keys { pad: &pd, ok, cancel }, sys, &mut req);
        let ev = dataload::trace::stop();
        let c = &dl.data;
        out.push(format!(
            "[{r},{},{},{},{},{},{},{},{},{},{},{},[{}]]",
            c.slot_proc,
            c.cur_no,
            c.dialog,
            c.load_flg as i32,
            c.sw,
            c.mask,
            c.temp_pn,
            sys.result as i32,
            sys.proccess,
            sys.port,
            sys.file_num,
            ev.join(",")
        ));
    }
    Ok(out)
}

/// The `lit` request: see the module docs.
fn lit(iso_path: &str, archive: &Arc<Archive>, frames: &[i64]) -> Result<String, Box<dyn std::error::Error>> {
    use glam::Mat4;
    use piney_demo::opening::act;
    use piney_demo::{Config, Demo};
    use piney_draw::{Cmd, Frame};
    use piney_gs::convert::{Params, Screen};

    let mut iso = Iso::open(iso_path)?;
    let mut d = Demo::new(&mut iso, archive.clone(), Config::default())?;
    let pad = Pad::default();
    for _ in 0..2000 {
        if d.phase() == Phase::Title && d.opening().main_act == act::NEUTRAL {
            break;
        }
        d.step(&pad);
        d.take_requests();
    }
    let mut assets = piney_gs::assets::Assets::new(archive.clone());
    let gfile = assets.file(piney_demo::names::TITLE_FILE).ok_or("title1")?;
    let screen = Screen { width: 512.0, height: 448.0, scale: 1.0 };
    let (mut at, mut out) = (0i64, Vec::new());
    for &want in frames {
        let mut frame = Frame::new();
        while at <= want {
            frame = d.step(&pad);
            d.take_requests();
            at += 1;
        }
        let draws: Vec<_> = frame
            .cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Model(m) if m.lights.is_some() => Some(m),
                _ => None,
            })
            .collect();
        let o = d.opening();
        let mut anms: Vec<&piney_desktop::anm::Anm> = vec![&o.camera, &o.a_win];
        for group in [&o.a_ico, &o.a_ico_d, &o.a_app, &o.a_app_d] {
            anms.extend(group.iter());
        }
        anms.extend(o.a_back.iter());
        for anm in anms {
            if !anm.is_set() {
                continue;
            }
            for inst in anm.instances() {
                let Some(info) = o.file.models.get(&inst.model) else { continue };
                if info.mtype & piney_demo::draw::MTYPE_LIT == 0 {
                    continue;
                }
                let lights = piney_demo::draw::omni_lights(inst.world);
                // The ModelDraw the frame holds for this object.
                let Some(m) = draws.iter().find(|m| m.model == inst.model && m.lights == Some(lights)) else {
                    continue;
                };
                let model = gfile.models.get(&inst.model).ok_or("model")?;
                let l = m.lights.as_ref().unwrap();
                let mut mmats = Vec::new();
                for md in &m.mmats {
                    // Only the lit colours are wanted, and they come from
                    // the normals and the lights: the positions are shrunk
                    // so that every triangle lies inside the GS space and
                    // comes back (the lit program keeps whole triangles).
                    let v = piney_gs::convert::mmat(
                        model,
                        md,
                        None,
                        None,
                        Mat4::from_scale(glam::Vec3::splat(1.0 / 4096.0)),
                        &[],
                        Some(l),
                        true,
                        piney_draw::DIV_Z,
                        screen,
                        Params(0),
                        0,
                        None,
                    );
                    let mm = &model.mmats[md.index as usize];
                    let mut cols = vec![[0.0f32; 4]; mm.positions.len()];
                    for (t, tri) in mm.triangles.iter().enumerate() {
                        for (k, &i) in tri.iter().enumerate() {
                            cols[i as usize] = v[3 * t + k].colour;
                        }
                    }
                    let cols: Vec<String> =
                        cols.iter().map(|c| format!("[{},{},{},{}]", c[0], c[1], c[2], c[3])).collect();
                    mmats.push(format!(
                        "{{\"index\":{},\"alpha\":{},\"colours\":[{}]}}",
                        md.index,
                        md.alpha,
                        cols.join(",")
                    ));
                }
                let f = |a: &[f32]| a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
                let v3 = |a: &[[f32; 3]; 3]| a.iter().map(|x| format!("[{}]", f(x))).collect::<Vec<_>>().join(",");
                out.push(format!(
                    "{{\"frame\":{want},\"label\":\"{}\",\"model\":{},\"world\":[{}],\"dirs\":[{}],\"colours\":[{}],\"ambient\":[{}],\"mmats\":[{}]}}",
                    anm.label,
                    inst.model,
                    f(&inst.world.to_cols_array()),
                    v3(&l.dirs),
                    v3(&l.colours),
                    f(&l.ambient),
                    mmats.join(",")
                ));
            }
        }
    }
    Ok(format!("[{}]", out.join(",")))
}
