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
    // The last raised is grown (level 4).
    if pens == 3 {
        state.save.set_i16(0x2194 + 24 * usize::from(TOWN), 4);
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

/// `spcParam[0]`'s gold (Kite's).
const GOLD: usize = 0x7488 + 0x14;
const FLAG_RACE: i16 = 88;

/// The menu task's (menu, proccess) and the race's (state, running) when
/// it lives.
fn status(s: &Session) -> ((i16, i16), Option<(i8, bool)>) {
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    let c = &w.ui().ctrl;
    ((c.menu, c.proccess), w.world().race().map(|r| (r.state, r.running)))
}

/// Frames of `raw` (`b` pressed every `every` frames) until `done`; false
/// when `max` frames pass first.
fn until(s: &mut Session, max: u32, b: Buttons, every: u32, done: impl Fn(&Session) -> bool) -> bool {
    let mut pad = Pad::default();
    for f in 0..max {
        if done(s) {
            return true;
        }
        let press = every != 0 && f % every == every - 1;
        pad.read(&still(if press { b } else { Buttons::NONE }));
        s.step(&pad);
        s.take_events();
    }
    done(s)
}

/// From the breeder's list to the race: Flag Race, through the greeting,
/// "It costs 100GP" (Yes), the first Grunty and "Start the race" (Yes);
/// then the frames until the countdown ends and the timer runs.
fn start_race(s: &mut Session) {
    press(s, Buttons::DOWN, 8);
    press(s, Buttons::CROSS, 10);
    assert_eq!(status(s).0.0, FLAG_RACE, "Flag Race is not up");
    assert!(until(s, 600, Buttons::CROSS, 20, |s| status(s).0 == (FLAG_RACE, 3)), "no cost question: {:?}", status(s));
    press(s, Buttons::CROSS, 20);
    assert!(until(s, 120, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 11)), "no Grunties: {:?}", status(s));
    press(s, Buttons::CROSS, 20);
    assert!(until(s, 120, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 13)), "no start question: {:?}", status(s));
    press(s, Buttons::CROSS, 2);
    assert!(
        until(s, 2000, Buttons::NONE, 0, |s| status(s).1.is_some_and(|r| r.1)),
        "the race never ran: {:?}",
        status(s)
    );
}

/// The race from the breeder's list, then quit from its pause: 100 GP
/// paid, Kite riding with the timer counting and the flags out; the
/// pause's Quit; the result, the prize's window, and the end: the race
/// gone, Kite on his feet by the breeder.
#[test]
fn mutations_flag_race_runs_and_quits() {
    let Some(mut s) = dun_loireag(3, 4) else { return };
    if let Stage::World(w) = &mut s.stage {
        w.world_mut().state_mut().save.set_i32(GOLD, 1000);
    }
    speak_to_breeder(&mut s).expect("the breeder's list did not open");
    start_race(&mut s);
    {
        let Stage::World(w) = &s.stage else { unreachable!() };
        let world = w.world();
        assert_eq!(world.state().save.i32(GOLD), 900, "100 GP paid");
        assert!(world.town_riding(), "Kite rides");
        assert_eq!(world.flags().len(), 3, "the three flags");
        assert!(world.flags().iter().all(|f| f.state == piney_world::race::FlagState::Out), "the flags out");
    }
    let t0 = s_race_time(&s);
    press(&mut s, Buttons::NONE, 30);
    assert!(s_race_time(&s) > t0, "the timer counts");
    // The pause: cancel; Quit, then Yes.
    press(&mut s, Buttons::CIRCLE, 20);
    assert_eq!(status(&s).0, (FLAG_RACE, 23), "the pause is not up");
    press(&mut s, Buttons::DOWN, 8);
    press(&mut s, Buttons::CROSS, 20);
    assert_eq!(status(&s).0, (FLAG_RACE, 26), "no quit question");
    press(&mut s, Buttons::UP, 8);
    press(&mut s, Buttons::CROSS, 10);
    assert!(
        until(&mut s, 600, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 41)),
        "no result word: {:?}",
        status(&s)
    );
    // The word, the prize's window, then the end.
    assert!(
        until(&mut s, 3000, Buttons::CROSS, 15, |s| status(s).1.is_none() && status(s).0.0 != FLAG_RACE),
        "the race did not end: {:?}",
        status(&s)
    );
    let Stage::World(w) = &s.stage else { unreachable!() };
    let world = w.world();
    assert!(!world.town_riding(), "still riding");
    assert_eq!(w.ui().ctrl.forbid, 0, "the menu's ban lifted");
    let end = piney_data::tables::race::of(piney_data::volume::Volume::Mut).end_pos()[1];
    let kite = world.player().body.pos.map(f32::from_bits);
    assert!((kite[0] - end[0]).abs() < 1.0 && (kite[1] - end[1]).abs() < 1.0, "Kite at {kite:?}, not {end:?}");
}

fn s_race_time(s: &Session) -> i16 {
    let Stage::World(w) = &s.stage else { panic!("left the town") };
    w.world().race().map_or(-1, |r| r.time)
}

/// Kite rides toward each flag still out (the nearest first); one he has
/// not reached in 150 frames (the town's walls stand between), he is set
/// down 120 short of. The frames until the three are taken, or None after
/// `max`.
fn ride_to_flags(s: &mut Session, max: u32) -> Option<u32> {
    let mut pad = Pad::default();
    let mut aim: Option<(usize, u32)> = None;
    for f in 0..max {
        let raw = {
            let Stage::World(w) = &mut s.stage else { panic!("left the town") };
            let world = w.world_mut();
            if world.race().is_some_and(|r| r.taken == 3) {
                return Some(f);
            }
            let kite = world.player().body.pos.map(f32::from_bits);
            let near = world
                .flags()
                .iter()
                .filter(|fl| fl.state == piney_world::race::FlagState::Out)
                .map(|fl| (fl.n, fl.pos))
                .min_by(|a, b| {
                    let d = |p: &[u32; 4]| (f32::from_bits(p[0]) - kite[0]).hypot(f32::from_bits(p[1]) - kite[1]);
                    d(&a.1).total_cmp(&d(&b.1))
                });
            match near {
                Some((n, at)) => {
                    let since = match aim {
                        Some((k, t)) if k == n => t,
                        _ => {
                            aim = Some((n, f));
                            f
                        }
                    };
                    let q = at.map(f32::from_bits);
                    if f - since >= 150 {
                        let put = [q[0] + 120.0, q[1], q[2], 1.0].map(f32::to_bits);
                        world.ride_put(put);
                        aim = Some((n, f));
                    }
                    let (dx, dy) = (q[0] - kite[0], q[1] - kite[1]);
                    super::stick_toward(f32::from_bits(world.camera().active().rot[2]), dx.atan2(-dy))
                }
                None => still(Buttons::NONE),
            }
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    None
}

/// The race won: the save's ranks for Dun Loireag set slow, Kite rides to
/// the three flags; the time is the first rank in the save, the Rankings
/// page blinks it, the town's first win gives its wallpaper, then the
/// prize, and the race ends.
#[test]
fn mutations_flag_race_won() {
    let Some(mut s) = dun_loireag(3, 4) else { return };
    if let Stage::World(w) = &mut s.stage {
        let save = &mut w.world_mut().state_mut().save;
        save.set_i32(GOLD, 1000);
        // Dun Loireag's three ranks: 5:00, 5:00, 5:00 by Grunty 146.
        for k in 0..3 {
            save.set_i16(piney_world::race::RACE_RECORDS + 4 * k, 9000);
            save.set_i16(piney_world::race::RACE_RECORDS + 4 * k + 2, 146);
        }
    }
    speak_to_breeder(&mut s).expect("the breeder's list did not open");
    start_race(&mut s);
    let took = ride_to_flags(&mut s, 9000);
    let flags = {
        let Stage::World(w) = &s.stage else { unreachable!() };
        w.world().flags().iter().map(|f| (f.state, f.pos.map(f32::from_bits))).collect::<Vec<_>>()
    };
    let took = took.unwrap_or_else(|| panic!("the flags were not all taken: {flags:?} {:?}", status(&s)));
    assert!(until(&mut s, 900, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 31)), "no time word: {:?}", status(&s));
    let (time, rank) = {
        let Stage::World(w) = &s.stage else { unreachable!() };
        let r = w.world().race().expect("the race");
        (r.time, r.rank)
    };
    assert_eq!(rank, 1, "first in {time} frames ({took} riding)");
    {
        let Stage::World(w) = &s.stage else { unreachable!() };
        let save = &w.world().state().save;
        assert_eq!(save.i16(piney_world::race::RACE_RECORDS), time, "the time first in the save");
        assert_eq!(save.i16(piney_world::race::RACE_RECORDS + 4), 9000, "the old first moved down");
    }
    // The time's word, then the Rankings page.
    assert!(until(&mut s, 300, Buttons::CROSS, 15, |s| status(s).0 == (FLAG_RACE, 32)), "no ranks: {:?}", status(&s));
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(w.ui().ctrl.exception_disp, 2, "the Rankings page with the blink");
    // Through the words, the wallpaper and the prize to the end.
    assert!(
        until(&mut s, 4000, Buttons::CROSS, 15, |s| status(s).1.is_none() && status(s).0.0 != FLAG_RACE),
        "the race did not end: {:?}",
        status(&s)
    );
    let Stage::World(w) = &s.stage else { unreachable!() };
    let save = &w.world().state().save;
    let t = piney_data::tables::race::of(piney_data::volume::Volume::Mut);
    let wp = i32::from(t.wallpapers()[1]) - 1;
    let bits = save.i32(piney_data::save::offset::DT_WALLPAPER_LIST + 4 * (wp / 32) as usize) as u32;
    assert_ne!(bits & (1 << (wp % 32)), 0, "Dun Loireag's wallpaper");
    assert_eq!(save.u8(piney_world::race::RACE_PRIZES), 1, "the first rank's prize given once");
}

/// The race's screens to `$PINEY_SHOTS` (/mnt/data/claude/scratch/i56):
/// the Grunties' page, the countdown, the ride with a flag taken, the
/// result's cup and the Rankings page with the new rank.
#[test]
#[ignore]
fn flag_race_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i56".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = dun_loireag(3, 4) else { return };
    if let Stage::World(w) = &mut s.stage {
        let save = &mut w.world_mut().state_mut().save;
        save.set_i32(GOLD, 1000);
        for k in 0..3 {
            save.set_i16(piney_world::race::RACE_RECORDS + 4 * k, 9000);
            save.set_i16(piney_world::race::RACE_RECORDS + 4 * k + 2, 146);
        }
    }
    speak_to_breeder(&mut s).expect("the breeder's list did not open");
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    g.set_overlay(Mode::archive(&s));
    let mut shot = |frame: &piney_draw::Frame, name: &str| {
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/{name}");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        println!("{path}");
    };
    let step = |s: &mut Session, b: Buttons| {
        let mut pad = Pad::default();
        pad.read(&still(b));
        let f = s.step(&pad);
        s.take_events();
        f
    };
    press(&mut s, Buttons::DOWN, 8);
    press(&mut s, Buttons::CROSS, 10);
    assert!(until(&mut s, 600, Buttons::CROSS, 20, |s| status(s).0 == (FLAG_RACE, 3)));
    press(&mut s, Buttons::CROSS, 20);
    assert!(until(&mut s, 120, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 11)));
    let f = press(&mut s, Buttons::NONE, 20);
    shot(&f, "mut-race-grunties.png");
    press(&mut s, Buttons::CROSS, 20);
    assert!(until(&mut s, 120, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 13)));
    press(&mut s, Buttons::CROSS, 2);
    // The countdown's "2".
    assert!(until(&mut s, 2000, Buttons::NONE, 0, |s| {
        let Stage::World(w) = &s.stage else { return false };
        w.world().race().is_some_and(|r| r.phase == 0 && r.count == 2 && r.tick == 10)
    }));
    let f = step(&mut s, Buttons::NONE);
    shot(&f, "mut-race-countdown.png");
    assert!(until(&mut s, 400, Buttons::NONE, 0, |s| status(s).1.is_some_and(|r| r.1)));
    let f = press(&mut s, Buttons::NONE, 40);
    shot(&f, "mut-race-riding.png");
    ride_to_flags(&mut s, 9000).expect("the flags");
    assert!(until(&mut s, 400, Buttons::NONE, 0, |s| {
        let Stage::World(w) = &s.stage else { return false };
        w.world().race().is_some_and(|r| r.sub == 2 && r.cnt == 60)
    }));
    let f = step(&mut s, Buttons::NONE);
    shot(&f, "mut-race-cup.png");
    assert!(until(&mut s, 900, Buttons::CROSS, 15, |s| status(s).0 == (FLAG_RACE, 32)));
    let f = press(&mut s, Buttons::NONE, 9);
    shot(&f, "mut-race-rankings.png");
}
