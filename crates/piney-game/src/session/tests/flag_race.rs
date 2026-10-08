//! Issue #56: Mutation's Grunty breeders offer the Flag Race. From Mutation
//! on `BreederMenu` (MUT gcmn 0x005624c0) lists Talk, Flag Race and Rankings
//! in place of the three "About" pages once the town's three pens hold a
//! grown Grunty (the menu's constructor counts them, MUT gcmn 0x0053a394)
//! and mail 324, which tells of the race, has been read.

use std::path::PathBuf;

use piney_input::{Buttons, Raw};
use piney_world::entry::Kind;

use super::*;

/// The breeders' type bit (`npcTbl` 10, 16, 22 and 28).
const BREEDER: u32 = 0x0800_0000;
/// The mail that tells of the race; `BreederMenu` and Rankings.
const RACE_MAIL: usize = 324;
const BREEDER_MENU: i16 = 27;
const RANKINGS: i32 = 89;
/// Dun Loireag (town 1, server 1).
const TOWN: u8 = 1;

fn still(buttons: Buttons) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// A new Mutation game logged in to Dun Loireag, its three pens holding
/// `pens` grown Grunties and mail 324 in state `mail`. None without the disc.
fn dun_loireag(pens: usize, mail: u8) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !iso.exists() {
        eprintln!("mutation.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    for k in 0..pens {
        // growth[server].type[k]: a grown Grunty in pen k.
        state.save.set_i16(0x2194 + 24 * usize::from(TOWN) + 0xe + 2 * k, 1);
    }
    state.save.set_mail(RACE_MAIL, mail);
    state.save.set_u8(offset::LAST_TOWN, TOWN);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    Some(Session::in_world(iso, archive, None, state, None, scene, None).unwrap())
}

/// Kite walks to the town's breeder and speaks to him; the frames until
/// his list is up and held 40 more. The last frame, or None if no list.
fn speak_to_breeder(s: &mut Session) -> Option<piney_draw::Frame> {
    let mut pad = Pad::default();
    let mut put = false;
    for f in 0..3000u32 {
        // Once placed, Kite is set down before the breeder, inside his
        // ranch's fence.
        if !put && let Stage::World(w) = &mut s.stage {
            let world = w.world_mut();
            let placed = matches!(world.phase(), piney_world::Phase::Play(n) if n >= 30);
            let front = world.merchants().iter().find(|m| m.flags & BREEDER != 0).map(|m| {
                let (q, h) = (m.ch.pos.map(f32::from_bits), f32::from_bits(m.ch.dirc[2]));
                [q[0] + 250.0 * h.sin(), q[1] - 250.0 * h.cos(), q[2]]
            });
            if let (true, Some(p)) = (placed, front) {
                // `pc_put`'s units are tenths of the world's.
                let (x, y, z) = ((p[0] / 10.0) as i16, (p[1] / 10.0) as i16, (p[2] / 10.0) as i16);
                put = world.pc_command(piney_event::host::PcCommand::Put { pc: 0, x, y, z });
            }
        }
        let (raw, open) = {
            let Stage::World(w) = &s.stage else { panic!("left the town") };
            let world = w.world();
            let ui = &w.ui().ctrl;
            let open = ui.menu == BREEDER_MENU && ui.menu_status == 2;
            let placed = matches!(world.phase(), piney_world::Phase::Play(n) if n >= 60);
            let kite = world.player().body.pos.map(f32::from_bits);
            let breeder = world.merchants().iter().find(|m| m.flags & BREEDER != 0);
            let raw = match breeder {
                _ if !placed || open => still(Buttons::NONE),
                Some(m) if world.command_target() == Some((Kind::Npc, m.id)) => {
                    still(if f % 20 == 0 { Buttons::CROSS } else { Buttons::NONE })
                }
                Some(m) => {
                    let q = m.ch.pos.map(f32::from_bits);
                    let (dx, dy) = (q[0] - kite[0], q[1] - kite[1]);
                    super::stick_toward(f32::from_bits(world.camera().active().rot[2]), dx.atan2(-dy))
                }
                None => panic!("no breeder in Dun Loireag"),
            };
            (raw, open)
        };
        if open {
            let mut frame = None;
            for _ in 0..40 {
                pad.read(&still(Buttons::NONE));
                frame = Some(s.step(&pad));
                s.take_events();
            }
            return frame;
        }
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    if let Stage::World(w) = &s.stage {
        let world = w.world();
        eprintln!(
            "phase {:?} merchants {:?} target {:?} kite {:?} menu {}",
            world.phase(),
            world.merchants().iter().map(|m| (m.id, m.flags, m.ch.pos.map(f32::from_bits))).collect::<Vec<_>>(),
            world.command_target(),
            world.player().body.pos.map(f32::from_bits),
            w.ui().ctrl.menu
        );
    } else {
        eprintln!("stage {}", Mode::title(s));
    }
    None
}

/// The breeder's list as it is drawn: its rows' text (`menuKanji`).
fn breeder_rows(s: &Session) -> String {
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    String::from_utf8_lossy(&w.ui().ctrl.kanji_text).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// With the three pens grown and mail 324 read, Dun Loireag's breeder lists
/// Talk, Flag Race and Rankings; with two pens, or the mail unread, the
/// "About" pages. Before the fix every case listed the "About" pages.
#[test]
fn mutations_breeder_offers_the_flag_race() {
    for (pens, mail, race) in [(3, 4, true), (3, 6, true), (2, 4, false), (3, 1, false), (3, 0, false)] {
        let Some(mut s) = dun_loireag(pens, mail) else { return };
        assert!(speak_to_breeder(&mut s).is_some(), "the breeder's list did not open");
        let rows = breeder_rows(&s);
        let Stage::World(w) = &s.stage else { unreachable!() };
        assert_eq!(w.ui().ctrl.pg_adult_num, pens as i16, "the pens counted");
        let want = if race { "Talk Flag Race Rankings" } else { "Talk About Grunties About Food About Breeding" };
        assert_eq!(rows, want, "pens {pens} mail {mail}");
    }
}

/// Rankings from the breeder's list: the page (89) opens on Dun Loireag's
/// three racers and closes back to the list.
#[test]
fn mutations_rankings_open_and_close() {
    let Some(mut s) = dun_loireag(3, 4) else { return };
    speak_to_breeder(&mut s).expect("the breeder's list did not open");
    to_rankings(&mut s);
    let menu = |s: &Session| match &s.stage {
        Stage::World(w) => (w.ui().menu_type(), w.ui().ctrl.exception_disp),
        _ => (-1, 0),
    };
    assert_eq!(menu(&s), (RANKINGS, 1), "Rankings is not up");
    press(&mut s, Buttons::CIRCLE, 30);
    assert_eq!(menu(&s).0, i32::from(BREEDER_MENU), "not back on the breeder's list");
}

/// `b` for a frame, then `n` frames of nothing; the last frame.
fn press(s: &mut Session, b: Buttons, n: u32) -> piney_draw::Frame {
    let mut pad = Pad::default();
    pad.read(&still(b));
    let mut frame = s.step(&pad);
    s.take_events();
    pad.read(&still(Buttons::NONE));
    for _ in 0..n {
        frame = s.step(&pad);
        s.take_events();
    }
    frame
}

/// Down twice to Rankings and OK, from the breeder's list; the page's
/// frame 30 frames on.
fn to_rankings(s: &mut Session) -> piney_draw::Frame {
    press(s, Buttons::DOWN, 8);
    press(s, Buttons::DOWN, 8);
    press(s, Buttons::CROSS, 38)
}

/// The breeder's list of the issue's screenshots, then Rankings, to
/// `$PINEY_SHOTS` (/mnt/data/claude/scratch/i56) as `mut-breeder-race.png`
/// and `mut-rankings.png`.
#[test]
#[ignore]
fn breeder_race_shot() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i56".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = dun_loireag(3, 4) else { return };
    let frame = speak_to_breeder(&mut s).expect("the breeder's list did not open");
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    g.set_overlay(Mode::archive(&s));
    let mut shot = |frame: &piney_draw::Frame, name: &str| {
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/{name}");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        path
    };
    println!("{}: {}", shot(&frame, "mut-breeder-race.png"), breeder_rows(&s));
    let frame = to_rankings(&mut s);
    println!("{}", shot(&frame, "mut-rankings.png"));
}
