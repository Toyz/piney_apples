//! The ending's save: event 31 (ENDING) on the desktop - stream 15, the
//! closing lines, the staff roll - and after the roll the save menus
//! (`ccDtMenu` menus 8 and 9) writing the clear data to a memory card.

use std::path::PathBuf;

use piney_desktop::Desktop;
use piney_desktop::card::{FilesCard, MemoryCard};
use piney_desktop::savesys::{INFO_SIZE, SaveDataInfo, code};
use piney_input::{Buttons, Raw};

use super::*;

/// story:31 (the desktop with event 31 next) with a card in MEMORY CARD
/// slot 1 at `card`. None without the disc.
fn ending(card: PathBuf) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 31).unwrap();
    Some(Session::resume(iso, archive, Some(card), start.state, start.vm, start.at).unwrap())
}

fn desktop(s: &Session) -> Option<&Desktop> {
    match &s.stage {
        Stage::Desktop(d) => d.desktop(),
        _ => None,
    }
}

/// The session played a frame at a time, the last frame kept for shots.
struct Player {
    s: Session,
    pad: Pad,
    frames: u32,
    last: Option<Frame>,
    /// Where [`Player::shot`] writes, when shots are asked for.
    shots: Option<(String, Option<piney_gs::Gs>)>,
}

impl Player {
    fn new(s: Session) -> Self {
        Player { s, pad: Pad::default(), frames: 0, last: None, shots: None }
    }

    fn step(&mut self, buttons: Buttons) {
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        self.pad.read(&Raw { buttons, ..still });
        let f = self.s.step(&self.pad);
        self.s.take_events();
        self.last = Some(f);
        self.frames += 1;
    }

    /// `n` frames with nothing held.
    fn wait(&mut self, n: u32) {
        for _ in 0..n {
            self.step(Buttons::NONE);
        }
    }

    /// One push, then let go.
    fn tap(&mut self, b: Buttons) {
        self.step(b);
        self.step(Buttons::NONE);
    }

    fn desktop(&self) -> &Desktop {
        desktop(&self.s).expect("not on the desktop")
    }

    fn result(&self) -> u32 {
        self.desktop().data().save_sys.result
    }

    /// Frames with nothing held until ccSaveSys's result is `r`, then 30
    /// more for the menu's wait and the message to come up.
    fn until_result(&mut self, r: u32) {
        for _ in 0..600 {
            if self.result() == r {
                self.wait(30);
                return;
            }
            self.step(Buttons::NONE);
        }
        panic!("no result {r:#x}: {:#x} at frame {}", self.result(), self.frames);
    }

    /// The last frame to `<dir>/<name>.png`, when shots are asked for.
    fn shot(&mut self, name: &str) {
        let (Some((dir, gs)), Some(frame)) = (self.shots.as_mut(), self.last.as_ref()) else { return };
        let g =
            gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(self.s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(&self.s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        println!("{path}");
    }

    /// The ending played to its save menu: cross every 30 frames (the
    /// closing lines) until menu 8 is up.
    fn play_to_save_menu(&mut self) {
        for i in 0..40_000u32 {
            if desktop(&self.s).is_some_and(|d| d.menu().menu == 8) {
                println!("menu 8 at frame {}", self.frames);
                return;
            }
            self.step(if i % 30 == 0 { Buttons::CROSS } else { Buttons::NONE });
        }
        panic!("no save menu after the staff roll: {}", Mode::title(&self.s));
    }

    /// Menu 8's OK, card slot 1 (no save directory on it: made), the
    /// first file (empty: "Create new data?" YES), "Data saved.", then
    /// cancel back out of the files and the slots.
    fn save_to_first_file(&mut self) {
        self.wait(40);
        self.shot("menu8");
        self.tap(Buttons::CROSS);
        self.until_result(code::SLOT_SELECT);
        self.shot("menu9-slots");
        self.tap(Buttons::CROSS);
        // "There is no saved data for .hack//INFECTION ..." then "Create
        // new save data ...?" YES (NO is chosen first).
        self.until_result(0x1032);
        self.tap(Buttons::CROSS);
        self.until_result(0x8033);
        self.shot("menu9-create");
        self.tap(Buttons::UP);
        self.wait(4);
        self.tap(Buttons::CROSS);
        self.until_result(0x1026);
        self.tap(Buttons::CROSS);
        self.until_result(code::SAVE_SELECT);
        self.shot("menu9-files");
        self.tap(Buttons::CROSS);
        self.until_result(0x801e);
        self.tap(Buttons::UP);
        self.wait(4);
        self.tap(Buttons::CROSS);
        self.until_result(0x101c);
        self.shot("menu9-saved");
        self.tap(Buttons::CROSS);
        self.until_result(code::SAVE_SELECT);
        self.shot("menu9-files-saved");
        self.tap(Buttons::CIRCLE);
        self.until_result(code::SLOT_SELECT);
        self.tap(Buttons::CIRCLE);
        for _ in 0..120 {
            if self.desktop().menu().menu == -1 {
                return;
            }
            self.step(Buttons::NONE);
        }
        panic!("the save menu did not close");
    }
}

/// Event 31 to the end of its staff roll; then the save menus (at the
/// staff roll's frame rate 2) save the clear data to the empty card in
/// slot 1: its directory made, the first file written. Read back: the
/// index's first record used, with the clear flag (1, from the ending's
/// `clear_count`) and the level; the slot file the save, its clear flag
/// set. The menu closed, the staff roll ends and wakes the desktop.
#[test]
fn ending_saves_the_clear_data() {
    let card = empty_card();
    let Some(s) = ending(card.clone()) else { return };
    let mut p = Player::new(s);
    p.play_to_save_menu();
    assert_eq!(p.desktop().frame_rate(), 2, "the staff roll's SetFrameRate(2)");
    let level = p.desktop().state().save.level();
    assert_eq!(p.desktop().state().save.clear_flag(), 1, "clear_count before the roll");
    p.save_to_first_file();
    // The instruction's end: bgm_control, a frame, staff_roll_done.
    p.wait(10);
    assert!(!p.desktop().slept(), "the desktop was not woken");

    let mut c = FilesCard::slot1(piney_data::volume::Volume::Inf, &card);
    let index = c.read_index(0).expect("no index on the card");
    let rec = SaveDataInfo::from_bytes(&index[..INFO_SIZE]);
    assert_eq!((rec.status, rec.clear_flag, rec.level), (1, 1, level), "{rec:?}");
    for i in 1..12 {
        assert_eq!(SaveDataInfo::from_bytes(&index[INFO_SIZE * i..INFO_SIZE * (i + 1)]).status, 0, "record {i}");
    }
    let slot = c
        .read_slot(0, 0, 1, piney_desktop::savesys::slot_size(piney_data::volume::Volume::Inf))
        .expect("no first file");
    let saved = piney_data::save::SaveData::from_bytes(&slot).unwrap();
    assert_eq!(saved.clear_flag(), 1);
    assert_eq!(saved.cstr(offset::PL_NAME, 24), p.desktop().state().save.cstr(offset::PL_NAME, 24));
    let _ = std::fs::remove_dir_all(&card);
}

/// The save menus after the staff roll to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/savemenus/shots): menu 8's question, menu 9's
/// card slots, the question, the files before and after the save.
#[test]
#[ignore]
fn save_menu_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/savemenus/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let card = empty_card();
    let Some(s) = ending(card.clone()) else { return };
    let mut p = Player::new(s);
    p.shots = Some((dir, None));
    p.play_to_save_menu();
    p.save_to_first_file();
    let _ = std::fs::remove_dir_all(&card);
}
