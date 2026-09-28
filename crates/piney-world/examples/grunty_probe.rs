//! Answers `tools/test_grunty_rs.py`: Dun Loireag's Grunties
//! (`piney_world::grunty`: `ccSetChibiGuso`, `ccPGuso`) one request a
//! line, one JSON line an answer, so the test can run the same through the
//! game's code in eemu.
//!
//! ```text
//! cargo build --release -p piney-world --example grunty_probe
//! grunty_probe ISO < requests
//! ```
//!
//! Numbers are hex; floats travel as their bit patterns. Requests:
//! - `set TOWN L S SM CR CU IQ PU T0 T1 T2 FOOD FLUTE`: the save's growth
//!   record for TOWN (level, size, the five stats, the three kinds, the
//!   food line) and key item 49's count; answers the rows
//!   `ccSetChibiGuso` places and the record after it.
//! - `new ID TOWN SEED`: row ID built by `setDog` and `ccPGuso::ccPGuso`
//!   in town TOWN over its collision, `rand()` seeded SEED, `ccRand`
//!   fresh; answers the state.
//! - `poke growth V`, `poke foodmode V`: the menus' writes to the Grunty
//!   (`growthNum`, `foodMode`), no answer.
//! - `frame PX PY PZ PD CX CY CZ DEG1 EYE [CMD A1 A2]`: Kite at P facing
//!   PD, the camera at C with pitch DEG1 (EYE the eye view), the menu's
//!   affect CMD (with its arguments) first when given, then the frame:
//!   the state, the events, the notes and what it drew.

use std::io::BufRead;
use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::save::SaveState;
use piney_world::char::View;
use piney_world::ee::{ONE, V4};
use piney_world::grunty::{self, Bodies, Growth, Grunty, GruntyCtx, GruntyEvent, Tables};
use piney_world::mt::Mt;
use piney_world::town::Town;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

fn growth(g: &Growth) -> String {
    list([g.level, g.size, g.smell, g.crooked, g.cruel, g.iq, g.pure, g.ty[0], g.ty[1], g.ty[2]].map(i32::from))
        .trim_end_matches(']')
        .to_string()
        + &format!(",{}]", g.food_num)
}

fn anm_name(body: &piney_world::body::Body, p: Option<&piney_world::pose::Play>) -> String {
    match p {
        Some(p) => {
            let a = &body.file.anims[p.anim];
            format!("[\"{}\",{}]", body.file.ccs.object_name(a.object).unwrap_or("?"), p.time)
        }
        None => "null".into(),
    }
}

fn state(g: &Grunty, save: &piney_data::save::SaveData) -> String {
    let a = if g.has_a { anm_name(&g.ch.body, Some(&g.ch.play)) } else { "null".into() };
    let b = anm_name(&g.body_b, g.play_b.as_ref());
    let c = match &g.body_c {
        Some(body) => anm_name(body, g.play_c.as_ref()),
        None => "null".into(),
    };
    let inu: Vec<i32> = (0..9).map(|k| i32::from(save.i16(grunty::SAVE_INU_COUNT + 2 * k))).collect();
    format!(
        concat!(
            "{{\"id\":{},\"local\":{},\"act\":[{},{},{},{},{}],\"pos\":{},\"posp\":{},\"dirc\":{},",
            "\"hit\":[{},{},{}],\"next\":{},\"old\":{},\"mark\":{},\"scale\":{},",
            "\"growth\":[{},{},{},{}],\"grow\":{},\"evo\":[{},{},{},{},{},{},{},{}],",
            "\"trans\":{},\"cam\":[{},{},{}],\"spdeg\":{},\"speed\":{},\"adult\":{},\"gpsub\":{},",
            "\"attr\":{},\"t\":{},\"a\":{},\"b\":{},\"c\":{},\"save\":{},\"inu\":{},\"fn\":{}}}"
        ),
        g.inu_id,
        g.local_id,
        g.act,
        g.act_old,
        g.act_process,
        g.act_cnt,
        g.anm_num,
        list(g.ch.pos),
        list(g.ch.pos_p),
        list(g.ch.dirc),
        g.body_hit_flag,
        g.body_hit_cnt,
        g.pos_cnt,
        list(g.next_pos),
        list(g.old_pos),
        g.mark,
        list(g.scale),
        g.growth_num,
        g.msg_num,
        g.dmylevel,
        g.pg_scale,
        growth(&g.grow),
        g.evolevel,
        g.evonum,
        g.evocnt,
        g.evoflag,
        g.exist,
        g.chat_flag,
        g.food_mode,
        g.chatcnt,
        list(g.transrate),
        g.cam_flag,
        list(g.cam_pos),
        list(g.cam_view),
        g.spdeg,
        g.speed,
        g.adult_type,
        list(g.gp_sub),
        g.ch.hit_attribute,
        g.ch.transparency,
        a,
        b,
        c,
        growth(&Growth::read(save, g.town)),
        list(inu),
        match g.affect_fn {
            grunty::AffectFn::Young => 0,
            grunty::AffectFn::Grown => 1,
            grunty::AffectFn::NewGrown => 2,
        },
    )
}

fn event(e: &GruntyEvent) -> String {
    match e {
        GruntyEvent::Note { param, pos, attribute } => format!("[\"note\",{param},{},{attribute}]", list(*pos)),
        GruntyEvent::Se3d { n, pos } => format!("[\"se3d\",{n},{}]", list(*pos)),
        GruntyEvent::Smoke { pos, v, scale, life, kind } => {
            format!("[\"smoke\",{},{},{scale},{life},{kind}]", list(*pos), list(*v))
        }
        GruntyEvent::Evolve { pos, height } => format!("[\"evolve\",{},{height}]", list(*pos)),
        GruntyEvent::Grow { pos, height } => format!("[\"grow\",{},{height}]", list(*pos)),
        GruntyEvent::Voice { grp, n } => format!("[\"voice\",{grp},{n}]"),
        GruntyEvent::VoiceStop => "[\"voicestop\"]".into(),
        GruntyEvent::Chat(k) => format!("[\"chat\",{k}]"),
        GruntyEvent::ChatClose => "[\"chatclose\"]".into(),
        GruntyEvent::Camera(n) => format!("[\"camera\",{n}]"),
        GruntyEvent::CamPos(v) => format!("[\"campos\",{}]", list(*v)),
        GruntyEvent::CamView(v) => format!("[\"camview\",{}]", list(*v)),
        GruntyEvent::Player { pos, dirc_z } => format!("[\"player\",{},{dirc_z}]", list(*pos)),
        GruntyEvent::Dropped => "[\"dropped\"]".into(),
        GruntyEvent::Debug(n) => format!("[\"debug\",{n}]"),
    }
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let tables = Rc::new(Tables::of(iso.volume().unwrap()).unwrap());
    let mut bodies = Bodies::load(&archive).unwrap();
    let mut town: Option<Town> = None;
    let mut save = SaveState::fresh().save;
    let mut g: Option<Grunty> = None;
    let mut rand = piney_world::Rand(1);
    let mut mt = Mt::init(0);
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        match w.first().copied() {
            Some("set") => {
                let t = n(1) as i32;
                let s = |i: usize| n(i) as i16;
                let rec = Growth {
                    level: s(2),
                    size: s(3),
                    smell: s(4),
                    crooked: s(5),
                    cruel: s(6),
                    iq: s(7),
                    pure: s(8),
                    ty: [s(9), s(10), s(11)],
                    food_num: n(12) as i32,
                };
                rec.write(&mut save, t);
                save.set_u8(0x0cfc + 49, n(13) as u8);
                let rows = grunty::set_chibi_guso(&mut save, t);
                println!("{{\"rows\":{},\"grow\":{}}}", list(rows), growth(&Growth::read(&save, t)));
            }
            Some("new") => {
                let (id, t) = (n(1) as i32, n(2) as i32);
                if town.as_ref().is_none_or(|x| x.base.no != t) {
                    town = Some(Town::open(&archive, t, false).unwrap());
                }
                let tw = town.as_mut().unwrap();
                rand = piney_world::Rand(u64::from(n(3)));
                mt = Mt::init(0);
                g = grunty::set_dog(&archive, &tables, &mut bodies, &tw.base.file, &mut tw.base.hits, &save, t, id, 0)
                    .unwrap();
                match &g {
                    Some(x) => println!("{}", state(x, &save)),
                    None => println!("null"),
                }
            }
            Some("poke") => {
                // The menus' writes: growthNum (BreedingMenu), foodMode
                // (InuMenu).
                if let Some(x) = g.as_mut() {
                    match w[1] {
                        "growth" => x.growth_num = n(2) as i8,
                        "foodmode" => x.food_mode = n(2) as u8,
                        _ => {}
                    }
                }
            }
            Some("frame") => {
                let (Some(x), Some(tw)) = (g.as_mut(), town.as_mut()) else {
                    println!("null");
                    continue;
                };
                let mut player: V4 = [n(1), n(2), n(3), ONE];
                let mut player_dirc: V4 = [0, 0, n(4), 0];
                let view = View { player, cam: [n(5), n(6), n(7), ONE], deg1: n(8) as i16, eye: n(9) != 0 };
                x.events.clear();
                if w.len() > 12 {
                    x.affect(n(10) as i16, n(11) as i16, n(12) as i16, &mut mt, player_dirc);
                }
                // Kite put at his place (affect 11) before the frame.
                for e in &x.events {
                    if let GruntyEvent::Player { pos, dirc_z } = e {
                        player = *pos;
                        player_dirc[2] = *dirc_z;
                    }
                }
                let before: Vec<String> = x.events.iter().map(event).collect();
                let mut ctx = GruntyCtx {
                    player,
                    player_dirc,
                    view,
                    hits: &mut tw.base.hits,
                    rand: &mut rand,
                    mt: &mut mt,
                    save: &mut save,
                };
                x.step(&mut ctx);
                let notes = std::mem::take(&mut x.notes);
                grunty::inu_check_note(&notes, x, &mut rand);
                let ev: Vec<String> = before.into_iter().chain(x.events.iter().map(event)).collect();
                let drawn: Vec<String> =
                    x.drawn.iter().map(|(k, a, s)| format!("[{k},{a},{}]", i32::from(*s))).collect();
                println!(
                    "{{\"s\":{},\"ev\":[{}],\"notes\":{},\"drawn\":[{}]}}",
                    state(x, &save),
                    ev.join(","),
                    list(notes.iter().map(|(e, p)| format!("[{e},{p}]"))),
                    drawn.join(","),
                );
            }
            _ => println!("null"),
        }
    }
}
