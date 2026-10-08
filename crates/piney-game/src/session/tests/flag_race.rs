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

/// A session stepped a frame at a time: its events dropped, or heard
/// ([`Ear`]).
trait Frames {
    fn session(&self) -> &Session;
    fn frame(&mut self, pad: &Pad) -> piney_draw::Frame;
}

impl Frames for Session {
    fn session(&self) -> &Session {
        self
    }

    fn frame(&mut self, pad: &Pad) -> piney_draw::Frame {
        let f = self.step(pad);
        self.take_events();
        f
    }
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

/// Kite walks round the town's walls to the breeder and speaks to him;
/// the frames until his list is up and held 40 more. The last frame, or
/// None if no list.
fn speak_to_breeder(s: &mut impl Frames) -> Option<piney_draw::Frame> {
    let mut pad = Pad::default();
    let mut walk = None;
    for f in 0..4000u32 {
        let (raw, open) = {
            let Stage::World(w) = &s.session().stage else { panic!("left the town") };
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
                    let w = walk.get_or_insert_with(|| super::town_walk::TownWalker::new([q[0], q[1]], 300.0));
                    w.stick(world, f).unwrap_or_else(|| {
                        let (dx, dy) = (q[0] - kite[0], q[1] - kite[1]);
                        super::stick_toward(f32::from_bits(world.camera().active().rot[2]), dx.atan2(-dy))
                    })
                }
                None => panic!("no breeder in Dun Loireag"),
            };
            (raw, open)
        };
        if open {
            let mut frame = None;
            for _ in 0..40 {
                pad.read(&still(Buttons::NONE));
                frame = Some(s.frame(&pad));
            }
            return frame;
        }
        pad.read(&raw);
        s.frame(&pad);
    }
    let s = s.session();
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

/// A new save holds no rank of the player's (`Init` zeroes the records at
/// +0x8432), so Rankings shows Dun Loireag's three racers (`race_ranks`,
/// MUT gcmn 0x006d8610) in order; with the player's time first, the racers
/// fill the ranks below it from the first (the page, MUT gcmn 0x0058ca90,
/// and the ranking, main 0x0017a860, take an empty rank's racer in turn).
#[test]
fn mutations_rankings_show_the_towns_racers() {
    for own in [None, Some((400, 146))] {
        let Some(mut s) = dun_loireag(3, 4) else { return };
        if let (Stage::World(w), Some((time, row))) = (&mut s.stage, own) {
            let save = &mut w.world_mut().state_mut().save;
            save.set_i16(piney_world::race::RACE_RECORDS, time);
            save.set_i16(piney_world::race::RACE_RECORDS + 2, row);
        }
        speak_to_breeder(&mut s).expect("the breeder's list did not open");
        to_rankings(&mut s);
        let Stage::World(w) = &s.stage else { unreachable!() };
        let kite = String::from_utf8_lossy(w.world().state().save.name()).into_owned();
        let names: Vec<String> = w.ui().ctrl.setting_text[1..4]
            .iter()
            .map(|t| String::from_utf8_lossy(&t[..t.len().min(16)]).trim_end().to_string())
            .collect();
        let want = match own {
            None => ["Balmung", "Gardenia", "Cima"].map(String::from),
            Some(_) => [kite, "Balmung".into(), "Gardenia".into()],
        };
        assert_eq!(names, want, "the ranks with {own:?} of the player's");
    }
}

/// `b` for a frame, then `n` frames of nothing; the last frame.
fn press(s: &mut impl Frames, b: Buttons, n: u32) -> piney_draw::Frame {
    let mut pad = Pad::default();
    pad.read(&still(b));
    let mut frame = s.frame(&pad);
    pad.read(&still(Buttons::NONE));
    for _ in 0..n {
        frame = s.frame(&pad);
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
fn until(s: &mut impl Frames, max: u32, b: Buttons, every: u32, done: impl Fn(&Session) -> bool) -> bool {
    let mut pad = Pad::default();
    for f in 0..max {
        if done(s.session()) {
            return true;
        }
        let press = every != 0 && f % every == every - 1;
        pad.read(&still(if press { b } else { Buttons::NONE }));
        s.frame(&pad);
    }
    done(s.session())
}

/// From the breeder's list to the race: Flag Race, through the greeting,
/// "It costs 100GP" (Yes), the first Grunty and "Start the race" (Yes);
/// then the frames until the countdown ends and the timer runs.
fn start_race(s: &mut impl Frames) {
    press(s, Buttons::DOWN, 8);
    press(s, Buttons::CROSS, 10);
    assert_eq!(status(s.session()).0.0, FLAG_RACE, "Flag Race is not up");
    let ok = until(s, 600, Buttons::CROSS, 20, |s| status(s).0 == (FLAG_RACE, 3));
    assert!(ok, "no cost question: {:?}", status(s.session()));
    press(s, Buttons::CROSS, 20);
    let ok = until(s, 120, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 11));
    assert!(ok, "no Grunties: {:?}", status(s.session()));
    press(s, Buttons::CROSS, 20);
    let ok = until(s, 120, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 13));
    assert!(ok, "no start question: {:?}", status(s.session()));
    press(s, Buttons::CROSS, 2);
    let ok = until(s, 2000, Buttons::NONE, 0, |s| status(s).1.is_some_and(|r| r.1));
    assert!(ok, "the race never ran: {:?}", status(s.session()));
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

/// Kite rides to each flag still out, the nearest first, as a player
/// would: the way round the town's walls (the Grunty's width either side)
/// planned each second, the stick toward its next turn. The frames until
/// the three are taken, or None after `max`.
fn ride_to_flags(s: &mut impl Frames, max: u32) -> Option<u32> {
    let mut pad = Pad::default();
    let mut ride: Option<(usize, super::town_walk::TownWalker)> = None;
    for f in 0..max {
        let raw = {
            let Stage::World(w) = &s.session().stage else { panic!("left the town") };
            let world = w.world();
            if world.race().is_some_and(|r| r.taken == 3) {
                return Some(f);
            }
            let kite = world.player().body.pos.map(f32::from_bits);
            let near = world
                .flags()
                .iter()
                .filter(|fl| fl.state == piney_world::race::FlagState::Out)
                .map(|fl| (fl.n, fl.pos.map(f32::from_bits)))
                .min_by(|a, b| {
                    (a.1[0] - kite[0]).hypot(a.1[1] - kite[1]).total_cmp(&(b.1[0] - kite[0]).hypot(b.1[1] - kite[1]))
                });
            match near {
                Some((n, q)) => {
                    if ride.as_ref().is_none_or(|(k, _)| *k != n) {
                        ride = Some((n, super::town_walk::TownWalker::new([q[0], q[1]], 150.0).riding(RIDE_WIDE)));
                    }
                    let (_, walk) = ride.as_mut().unwrap();
                    walk.stick(world, f).unwrap_or_else(|| {
                        let cam = f32::from_bits(world.camera().active().rot[2]);
                        super::stick_toward(cam, (q[0] - kite[0]).atan2(-(q[1] - kite[1])))
                    })
                }
                None => still(Buttons::NONE),
            }
        };
        pad.read(&raw);
        s.frame(&pad);
    }
    None
}

/// The way's clearance either side for the ridden Grunty.
const RIDE_WIDE: f32 = 85.0;

/// The race won: the save's ranks for Dun Loireag are the player's own
/// earlier 5:00s (the ride takes about half a minute, slower than the
/// town's racers' 0:20), Kite rides to the three flags; the time is the
/// first rank in the save, the Rankings page blinks it, the town's first
/// win gives its wallpaper, then the prize, and the race ends.
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

/// A new save's race: Kite's time against Dun Loireag's racers (Balmung
/// 600, Gardenia 626, Cima 646): the rank the game's ranking gives (main
/// 0x0017a860: the first it beats, else 4 within 30 of the third, else 0),
/// and the save's records written only for a rank.
#[test]
fn mutations_flag_race_against_the_towns_racers() {
    let Some(mut s) = dun_loireag(3, 4) else { return };
    if let Stage::World(w) = &mut s.stage {
        w.world_mut().state_mut().save.set_i32(GOLD, 1000);
    }
    speak_to_breeder(&mut s).expect("the breeder's list did not open");
    start_race(&mut s);
    let took = ride_to_flags(&mut s, 9000).expect("the flags were not all taken");
    assert!(until(&mut s, 900, Buttons::NONE, 0, |s| status(s).0 == (FLAG_RACE, 31)), "no time word: {:?}", status(&s));
    let Stage::World(w) = &s.stage else { unreachable!() };
    let r = w.world().race().expect("the race");
    let racers: Vec<i16> = piney_data::tables::fieldui::of(piney_data::volume::Volume::Mut).race_ranks()[0]
        .iter()
        .map(|k| k.time)
        .collect();
    let want = match racers.iter().position(|&t| r.time < t) {
        Some(k) => k as i8 + 1,
        None if r.time - 30 < racers[2] => 4,
        None => 0,
    };
    println!("the ride took {took} frames; the time {} against {racers:?}: rank {}", r.time, r.rank);
    assert_eq!(r.rank, want, "the rank of {} against {racers:?}", r.time);
    let save = &w.world().state().save;
    let records: Vec<i16> = (0..3).map(|k| save.i16(piney_world::race::RACE_RECORDS + 4 * k)).collect();
    match want {
        1..=3 => assert_eq!(records[want as usize - 1], r.time, "the time in the save: {records:?}"),
        _ => assert_eq!(records, [0; 3], "no rank, nothing written"),
    }
}

/// The race's screens to `$PINEY_SHOTS` (/mnt/data/claude/scratch/i56):
/// the Grunties' page, the countdown, the ride with a flag taken, the
/// result's clip (a cup for a rank) and the Rankings page.
#[test]
#[ignore]
fn flag_race_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i56".into());
    std::fs::create_dir_all(&dir).unwrap();
    // A new save's ranks: the town's racers (no record of the player's).
    let Some(mut s) = dun_loireag(3, 4) else { return };
    if let Stage::World(w) = &mut s.stage {
        w.world_mut().state_mut().save.set_i32(GOLD, 1000);
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
        w.world().race().is_some_and(|r| matches!(r.sub, 2..=4) && r.cnt == 60)
    }));
    let f = step(&mut s, Buttons::NONE);
    shot(&f, "mut-race-result.png");
    // The race over, Rankings from the breeder's list.
    assert!(until(&mut s, 4000, Buttons::CROSS, 15, |s| status(s).1.is_none() && status(s).0.0 != FLAG_RACE));
    press(&mut s, Buttons::NONE, 30);
    speak_to_breeder(&mut s).expect("the breeder's list did not open");
    let f = to_rankings(&mut s);
    shot(&f, "mut-race-rankings.png");
}

/// What a frame asked of SEWORDS's channel 0, and what it held after.
#[derive(Clone, Copy, Debug)]
struct Heard {
    /// The menu task's (menu, proccess).
    menu: (i16, i16),
    /// The race's rank and `+0xa5` (the menu is done with it) while it
    /// lives.
    race: Option<(i8, bool)>,
    /// `ccBgmPlay(n)`, `ccBgmStop()`, `ccEvVoiceStop()` this frame.
    play: Option<usize>,
    stop: bool,
    voice_stop: bool,
    /// A `BGM.BIN` track streams after the frame.
    streaming: bool,
    /// Stereo samples rendered to the frame's end.
    at: u64,
}

/// A session whose every frame's events go to a headless engine as `main`
/// routes them, from its start, each frame's [`Heard`] kept.
struct Ear {
    s: Session,
    audio: piney_audio::Audio,
    log: Vec<Heard>,
    at: u64,
}

impl Frames for Ear {
    fn session(&self) -> &Session {
        &self.s
    }

    fn frame(&mut self, pad: &Pad) -> piney_draw::Frame {
        use crate::mode::Event;
        let f = self.s.step(pad);
        let events = self.s.take_events();
        let play = events.iter().find_map(|e| if let Event::BgmStream(n) = e { Some(*n) } else { None });
        let stop = events.iter().any(|e| matches!(e, Event::BgmStreamStop));
        let voice_stop = events.iter().any(|e| matches!(e, Event::VoiceStop));
        crate::handle(events, Some(&self.audio));
        self.audio.frame();
        // A vertical blank's 800 stereo samples (48 kHz), interleaved.
        let n = 1600 * Mode::frame_rate(&self.s) as usize;
        self.audio.render(&mut vec![0i16; n]);
        self.at += n as u64 / 2;
        let menu = status(&self.s).0;
        let race = match &self.s.stage {
            Stage::World(w) => w.world().race().map(|r| (r.rank, r.done)),
            _ => None,
        };
        self.log.push(Heard { menu, race, play, stop, voice_stop, streaming: self.audio.bgm_streaming(), at: self.at });
        f
    }
}

/// Issue #64: the race's result music lasts the results. Ranks 1-3 play
/// `BGM.BIN` track 5 (looped) after the cup's 4; rank 4 and none, track 6
/// once. Each box's OK calls `ccEvVoiceStop`, which from Mutation on does
/// nothing while the race holds the music (`ccSnd +0x13a`, MUT main
/// 0x00181f84). The music stops at the race's end (gcmn 0x005fe0d0), 30
/// frames after the prize's step sets `+0xa5`. The records are set once
/// the flags are taken, for each rank. Before the fix a box's OK cut it.
#[test]
fn mutations_race_music_lasts_the_results() {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    let bgm = piney_data::sound::tables_of(piney_data::volume::Volume::Mut).bgm;
    for want in [1i8, 3, 4, 0] {
        let Some(s) = dun_loireag(3, 4) else { return };
        let audio = piney_audio::Audio::headless(&iso).unwrap();
        let mut e = Ear { s, audio, log: Vec::new(), at: 0 };
        if let Stage::World(w) = &mut e.s.stage {
            w.world_mut().state_mut().save.set_i32(GOLD, 1000);
        }
        speak_to_breeder(&mut e).expect("the breeder's list did not open");
        start_race(&mut e);
        ride_to_flags(&mut e, 9000).expect("the flags were not all taken");
        // Dun Loireag's three records against the time, before the finish
        // enters it (main 0x0017a860): ahead of the third for 3, within
        // 30 of it for 4, far ahead of it for none.
        if let Stage::World(w) = &mut e.s.stage {
            let t = w.world().race().expect("the race").time;
            let records = match want {
                1 => [t + 300, t + 400, t + 500],
                3 => [t - 200, t - 100, t + 100],
                4 => [t - 300, t - 200, t - 10],
                _ => [t - 300, t - 200, t - 100],
            };
            let save = &mut w.world_mut().state_mut().save;
            for (k, r) in records.into_iter().enumerate() {
                save.set_i16(piney_world::race::RACE_RECORDS + 4 * k, r);
                save.set_i16(piney_world::race::RACE_RECORDS + 4 * k + 2, 146);
            }
        }
        let from = e.log.len();
        let over = |s: &Session| status(s).1.is_none() && status(s).0.0 != FLAG_RACE;
        // OK every 5 frames: each box passed as soon as it takes it.
        assert!(until(&mut e, 4000, Buttons::CROSS, 5, over), "rank {want}: the race did not end");
        let log = &e.log[from..];
        let rank = log.iter().find(|h| h.menu == (FLAG_RACE, 31)).and_then(|h| h.race).map(|r| r.0);
        assert_eq!(rank, Some(want), "the rank at the time word");
        let track = if (1..=3).contains(&want) { 5 } else { 6 };
        let k0 =
            log.iter().position(|h| h.play == Some(track)).unwrap_or_else(|| panic!("rank {want}: no track {track}"));
        let k1 = k0 + log[k0..].iter().position(|h| !h.streaming).expect("the music never stopped");
        let boxes: std::collections::BTreeSet<i16> =
            log[k0..k1].iter().filter(|h| h.menu.0 == FLAG_RACE).map(|h| h.menu.1).collect();
        let voice_stops = log[k0..k1].iter().filter(|h| h.voice_stop).count();
        let heard = log[k1].at - log[k0 - 1].at;
        eprintln!(
            "rank {want}: track {track} for {} frames ({heard} samples) over boxes {boxes:?}, {voice_stops} voice stops; \
             at its end: {:?}",
            k1 - k0,
            log[k1]
        );
        if track == 5 {
            // Every box of the results heard, then the race's own stop
            // once the menu is done with it.
            for p in [31, 32, 41] {
                assert!(boxes.contains(&p), "rank {want}: the music stopped before box {p}: {boxes:?}");
            }
            if want == 1 {
                assert!(boxes.contains(&51), "rank 1: the music stopped before the wallpaper: {boxes:?}");
            }
            assert!(voice_stops > 0, "rank {want}: no box was passed while the music played");
            let done = log[k1 - 1].race.is_some_and(|r| r.1);
            assert!(log[k1].stop && done, "rank {want}: not the race's end: {:?}", log[k1]);
        } else {
            // Track 6 plays out (or the race's end stops it).
            let len = bgm[6].size as u64 / 4;
            assert!(heard >= len || log[k1].stop, "rank {want}: track 6 cut after {heard} of {len} samples");
        }
    }
}
