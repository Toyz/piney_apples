//! The title against Infection's disc, driven by pad input as the game
//! reads it: boot to the menu, the three items, the idle attract loop, the
//! memory-card question, a soft reset. Skipped when the disc image is not
//! extracted; set PINEY_ISO to point at it elsewhere.
//!
//! The game-code checks of the logic are in tools/test_demo_rs.py.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_demo::opening::{act, item};
use piney_demo::{Config, Demo, Phase, Request};
use piney_desktop::card::NoCard;
use piney_draw::{Cmd, Frame};
use piney_input::{Buttons, Pad, Raw};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

struct Run {
    d: Demo,
    pad: Pad,
    requests: Vec<(usize, Request)>,
    frame: usize,
}

impl Run {
    fn new(config: Config) -> Option<Run> {
        let mut iso = Iso::open(iso_path()?).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let d = Demo::new(&mut iso, archive, config).unwrap();
        Some(Run { d, pad: Pad::default(), requests: Vec::new(), frame: 0 })
    }

    fn step(&mut self, b: Buttons) -> Frame {
        self.pad.read(&Raw { buttons: b, ..Raw::default() });
        let f = self.d.step(&self.pad);
        let asked = self.d.take_requests();
        // The stream plays out under `PlayOpeningStream` before the title
        // steps again: its frames 1 to `frames`, the flashes over them.
        if let Some(&Request::Stream { frames, .. }) = asked.iter().find(|r| matches!(r, Request::Stream { .. })) {
            self.pad.read(&Raw::default());
            for n in 0..frames {
                self.d.stream_tick(&self.pad, n);
                self.d.stream_fade(&mut Frame::new());
            }
        }
        for r in asked {
            self.requests.push((self.frame, r));
        }
        self.frame += 1;
        f
    }

    /// Frames with nothing pressed until `done`, at most `max`.
    fn until(&mut self, max: usize, done: impl Fn(&Demo) -> bool) -> Frame {
        let mut f = Frame::new();
        for _ in 0..max {
            if done(&self.d) {
                break;
            }
            f = self.step(Buttons::NONE);
        }
        assert!(done(&self.d), "not reached in {max} frames: {:?}", self.d.phase());
        f
    }

    fn press(&mut self, b: Buttons) -> Frame {
        let f = self.step(b);
        self.step(Buttons::NONE);
        f
    }

    fn asked(&self, want: &Request) -> Option<usize> {
        self.requests.iter().find(|(_, r)| r == want).map(|(f, _)| *f)
    }

    fn reach_menu(&mut self) {
        self.until(400, |d| d.phase() == Phase::Title && d.opening().main_act == act::NEUTRAL);
    }
}

fn models(f: &Frame) -> usize {
    f.cmds.iter().filter(|c| matches!(c, Cmd::Model(_))).count()
}

#[test]
fn boot_to_the_menu() {
    let Some(mut r) = Run::new(Config::default()) else { return };
    let f = r.until(400, |d| d.phase() == Phase::Title);
    assert_eq!(models(&f), 0);
    let first = r.step(Buttons::NONE);
    // The menu comes out of the stream's white flash: 29 objects under the
    // fader's element on the font layer.
    assert_eq!(models(&first), 29);
    assert!(first.cmds.iter().any(|c| matches!(c, Cmd::Prim(_))));
    let order: Vec<Request> = r.requests.iter().map(|(_, q)| q.clone()).collect();
    let movies = [("PSS/LOGO_B.PSS", false), ("PSS/LOGO_C.PSS", false), ("PSS/LOGO_H.PSS", false)]
        .into_iter()
        .chain([("PSS/OPENING.PSS", true)])
        .map(|(path, audio)| Request::Movie { path, audio });
    let mut want = vec![Request::SqStop(0), Request::MenuDisplay(false)];
    want.extend(movies);
    want.extend([
        Request::MainVolume(r.d.save().i16(piney_demo::MAIN_VOL)),
        Request::SqPlay(0),
        Request::MenuDisplay(true),
        Request::Stream { num: 0, frames: 410 },
    ]);
    assert_eq!(order, want);
    // The check passes at once with a card: the first logo on frame 6;
    // LogoMain's 60 frames of wait after the opening movie, then the music.
    let logo = r.asked(&want[2]).unwrap();
    assert_eq!(logo, 6);
    let opening = r.asked(&want[5]).unwrap();
    assert_eq!(r.asked(&Request::SqPlay(0)).unwrap() - opening, 62);
    assert_eq!(r.d.cursor(), item::LOAD);
}

/// `spcParam[0].maxHP` in the save.
fn kite_max_hp(d: &Demo) -> i16 {
    d.save().i16(piney_demo::newgame::offset::SPC_PARAM + 0x24)
}

#[test]
fn new_game_leaves_for_the_desktop() {
    let Some(mut r) = Run::new(Config::default()) else { return };
    // ccSetupDemo's NewGame(1) has filled the characters.
    assert_eq!(kite_max_hp(&r.d), 63);
    r.reach_menu();
    r.press(Buttons::UP);
    assert_eq!(r.d.cursor(), item::NEW_GAME);
    assert!(r.requests.iter().any(|(_, q)| *q == Request::Se(2)));
    r.press(Buttons::UP);
    assert_eq!(r.d.cursor(), item::NEW_GAME, "the cursor stops at the top");
    let decide = r.frame;
    r.press(Buttons::CROSS);
    assert_eq!(r.asked(&Request::Se(4)), Some(decide));
    r.until(200, |d| d.phase() == Phase::Left);
    let new_game = r.asked(&Request::NewGame { parody: false }).unwrap();
    let change = r.asked(&Request::ChangeMode { num: 3, sf: 7 }).unwrap();
    assert!(r.asked(&Request::SqFade { seq: 0, volume: 0, time: 8, mode: 3 }) == Some(new_game));
    // Breath(2) between ccSqFade and ChangeRequest; the screen-out
    // (ANM_xdt_ou00, 61 frames) before.
    assert_eq!(change - new_game, 2);
    assert!(new_game - decide > 60, "{new_game} {decide}");
    assert!(!r.d.save().parody());
    // NewGame(0) applied in the step that asked for it.
    assert_eq!(kite_max_hp(&r.d), 63);
    assert_eq!(r.d.save().i16(piney_demo::newgame::offset::SPC_PARAM + 0x0e), 1);
}

#[test]
fn load_with_no_save_returns_to_the_menu() {
    let Some(mut r) = Run::new(Config::default()) else { return };
    r.reach_menu();
    r.press(Buttons::CROSS);
    r.until(10, |d| d.opening().main_act == act::DATA_LOAD);
    // The window opens (ANM_xdt_op01), then the load screen takes input.
    r.until(100, |d| d.opening().dat_sw != 0);
    r.press(Buttons::CIRCLE);
    assert_eq!(r.d.opening().dat_act, 3);
    r.until(200, |d| d.opening().main_act == act::NEUTRAL);
    assert_eq!(r.d.cursor(), item::LOAD);
    assert_eq!(r.d.opening().transp.to_bits(), 1.0f32.to_bits());
    assert!(r.asked(&Request::LoadGame).is_none());
}

#[test]
fn load_reads_a_save_and_leaves_for_the_desktop() {
    use piney_demo::seam::MemCard;
    use piney_desktop::savesys::{LOAD_DONE, LOAD_QUESTION, LOAD_SELECT};
    // A card with one save, in slot 3: a new game's save with a play time,
    // and its index record.
    let mut data = piney_desktop::SaveState::fresh().save.bytes().to_vec();
    data[0x8400..0x8404].copy_from_slice(&123_456i32.to_le_bytes());
    let sum = data.iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b)));
    let mut card = MemCard { dir: true, ..MemCard::default() };
    let rec = &mut card.index[28 * 2..28 * 3];
    rec[0] = 1;
    rec[1] = 7;
    rec[4..8].copy_from_slice(b"Kite");
    rec[0x16..0x18].copy_from_slice(&sum.to_le_bytes());
    rec[0x18..0x1c].copy_from_slice(&123_456i32.to_le_bytes());
    card.slots[2] = data.clone();
    let Some(mut r) = Run::new(Config { card: Box::new(card), ..Config::default() }) else { return };
    r.reach_menu();
    assert_eq!(r.d.cursor(), item::LOAD);
    r.press(Buttons::CROSS);
    r.until(100, |d| d.opening().dat_sw != 0);
    // MEMORY CARD slot 1, then the list.
    r.press(Buttons::CROSS);
    r.until(20, |d| d.save_sys().result == LOAD_SELECT);
    r.press(Buttons::DOWN);
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    r.until(20, |d| d.save_sys().result == LOAD_QUESTION);
    // YES is the upper line; NO is chosen first.
    r.press(Buttons::UP);
    r.press(Buttons::CROSS);
    r.until(20, |d| d.save_sys().result == LOAD_DONE);
    r.press(Buttons::CROSS);
    r.until(400, |d| d.phase() == Phase::Left);
    assert!(r.asked(&Request::LoadGame).is_some());
    assert!(r.asked(&Request::ChangeMode { num: 3, sf: 7 }).is_some());
    assert!(r.asked(&Request::NewGame { parody: false }).is_none());
    let s = r.d.save().bytes();
    assert_eq!(&s[0x8400..0x8404], &123_456i32.to_le_bytes());
    assert_eq!(&s[..4], b"Kite");
    assert_eq!(r.d.card_position(), (0, 2));
}

#[test]
fn option_opens_the_system_menu_and_comes_back() {
    let Some(mut r) = Run::new(Config::default()) else { return };
    r.reach_menu();
    r.press(Buttons::DOWN);
    assert_eq!(r.d.cursor(), item::OPTION);
    r.press(Buttons::CROSS);
    r.until(10, |d| d.opening().main_act == act::OPTION);
    r.until(100, |d| d.opening().opt_act == 2);
    r.press(Buttons::CIRCLE);
    r.until(200, |d| d.opening().main_act == act::NEUTRAL);
    assert_eq!(r.d.cursor(), item::OPTION);
}

#[test]
fn option_sets_vibration_through_the_system_menu() {
    let Some(mut r) = Run::new(Config::default()) else { return };
    r.reach_menu();
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    r.until(100, |d| d.menu().menu_type() == 1);
    // The title's OPTION list: Controller, Vibrate, ...; Vibrate, then
    // "OFF" (the second row).
    r.until(40, |d| d.opening().opt_act == 2);
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    r.until(20, |d| d.menu().menu_type() == 3);
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.save().u8(piney_data::save::offset::VIBRATION), 0);
    assert!(r.asked(&Request::Menu(piney_desktop::Request::Vibration { on: false })).is_some());
    // The title's own sounds: decide 4, not the desktop's 18.
    assert!(r.asked(&Request::Se(4)).is_some());
    assert!(r.asked(&Request::Se(18)).is_none());
    r.press(Buttons::CIRCLE);
    r.until(20, |d| d.menu().menu_type() == 1);
    r.press(Buttons::CIRCLE);
    r.until(200, |d| d.opening().main_act == act::NEUTRAL);
    assert_eq!(r.d.menu().menu_type(), -1);
    assert_eq!(r.d.cursor(), item::OPTION);
}

#[test]
fn idle_title_goes_back_to_the_opening_movie() {
    let Some(mut r) = Run::new(Config::default()) else { return };
    r.reach_menu();
    let start = r.frame;
    r.until(2500, |d| d.phase() != Phase::Title);
    let movie = Request::Movie { path: "PSS/OPENING.PSS", audio: true };
    let again = r.requests.iter().filter(|(_, q)| *q == movie).map(|(f, _)| *f).max().unwrap();
    // MOVEcount: 2380 frames to the fade, 2400 to the return, counted from
    // the first frame at rest.
    assert!((2398..=2402).contains(&(again - start)), "{}", again - start);
    assert!(r.asked(&Request::SqStop(0)).is_some());
}

#[test]
fn no_card_asks_and_goes_on() {
    let Some(mut r) = Run::new(Config { card: Box::new(NoCard), ..Config::default() }) else { return };
    r.until(20, |d| d.opening().boot_mem.shown.is_some());
    assert_eq!(r.d.opening().boot_mem.shown, Some(0x8028));
    // No (0) is the default; cross on it leaves the question up.
    r.press(Buttons::CROSS);
    r.step(Buttons::NONE);
    assert_eq!(r.d.opening().boot_mem.shown, Some(0x8028));
    r.press(Buttons::UP);
    assert_eq!(r.d.opening().boot_mem.data.dialog, 1);
    r.press(Buttons::CROSS);
    r.until(100, |d| d.phase() != Phase::Boot);
    assert!(r.requests.iter().any(|(_, q)| matches!(q, Request::Movie { .. })));
}

#[test]
fn soft_reset_skips_the_logos() {
    let Some(mut r) = Run::new(Config { first_boot: false, ..Config::default() }) else { return };
    r.until(20, |d| d.phase() == Phase::Title);
    assert!(!r.requests.iter().any(|(_, q)| matches!(q, Request::Movie { .. })));
    assert!(r.asked(&Request::Stream { num: 0, frames: 410 }).is_some());
}
