//! The desktop against Infection's disc: its tables, the main screen's
//! navigation, the mailer and the World request, driven by pad input as the
//! game reads it. Skipped when the disc image is not extracted; set
//! PINEY_ISO to point at it elsewhere.
//!
//! The game-code checks of the logic (SelectMode, DrawLogo, NewIconDraw, the
//! mailer's cursor and scroll bar, the text) are in tools/test_desktop_rs.py.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::desktop::Icon;
use piney_desktop::{Desktop, MailState, Request, SaveState, Se};
use piney_draw::Cmd;
use piney_input::{Buttons, Pad, Raw};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

struct Run {
    d: Desktop,
    pad: Pad,
    requests: Vec<Request>,
}

impl Run {
    fn new(state: SaveState) -> Option<(Run, Arc<Archive>)> {
        let mut iso = Iso::open(iso_path()?).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let d = Desktop::new(&mut iso, archive.clone(), state).unwrap();
        Some((Run { d, pad: Pad::default(), requests: Vec::new() }, archive))
    }

    fn step(&mut self, b: Buttons) -> piney_draw::Frame {
        self.pad.read(&Raw { buttons: b, ..Raw::default() });
        let f = self.d.step(&self.pad);
        self.requests.extend(self.d.take_requests());
        f
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.step(Buttons::NONE);
        }
    }

    fn press(&mut self, b: Buttons) {
        self.step(b);
        self.step(Buttons::NONE);
    }

    /// Past the opening (skipped with cross) and the first page title.
    fn reach_main_screen(&mut self) {
        self.idle(5);
        self.press(Buttons::CROSS);
        self.idle(60);
    }

    fn se(&self) -> Vec<i32> {
        self.requests
            .iter()
            .filter_map(|r| match r {
                Request::Se(Se(n)) => Some(*n),
                _ => None,
            })
            .collect()
    }
}

#[test]
fn opening_then_icons() {
    let Some((mut r, _)) = Run::new(SaveState::fresh()) else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let f = r.step(Buttons::NONE);
    assert!(f.cmds.is_empty(), "the first breath draws nothing");
    assert_eq!(r.requests[..2], [Request::Bgm(50), Request::EnableReset(true)]);
    r.idle(80);
    assert_eq!(r.se(), vec![0, 1], "sound 0 as the opening starts, 1 at its frame 60");
    r.press(Buttons::CROSS);
    r.idle(60);
    assert_eq!(r.d.selection(), 1, "the desktop starts on the mailer");
    r.requests.clear();
    r.press(Buttons::DOWN);
    assert_eq!(r.d.selection(), 2);
    assert_eq!(r.se(), vec![2]);
    // Input waits for the page title to come in.
    r.press(Buttons::DOWN);
    assert_eq!(r.d.selection(), 2);
    r.idle(30);
    r.press(Buttons::UP);
    r.idle(30);
    r.press(Buttons::UP);
    r.idle(30);
    assert_eq!(r.d.selection(), 0);
    // Back from The World wraps through 7 to the Data icon.
    r.press(Buttons::UP);
    assert_eq!(r.d.selection(), 7);
    r.idle(30);
    assert_eq!(r.d.selection(), 5);
}

#[test]
fn main_screen_draws() {
    let Some((mut r, archive)) = Run::new(SaveState::fresh()) else { return };
    r.reach_main_screen();
    let f = r.step(Buttons::NONE);
    let models = f.cmds.iter().filter(|c| matches!(c, Cmd::Model(_))).count();
    assert!(models > 40, "{models} model draws");
    let mut assets = piney_desktop::soft::Assets::new(archive);
    let mut canvas = piney_desktop::soft::Canvas::new(512, 448, f.clear);
    canvas.draw(&f, &mut assets);
    let lit = canvas.px.iter().filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 30).count();
    assert!(lit > 512 * 448 / 2, "{lit} pixels drawn");
}

#[test]
fn mail_is_listed_read_and_marked() {
    let mut state = SaveState::fresh();
    // What event 1 delivers on a new game (mail 4, 5 and 320).
    for m in [4, 5, 320] {
        state.save.new_mail(m);
    }
    let Some((mut r, _)) = Run::new(state) else { return };
    r.reach_main_screen();
    assert!(r.se().contains(&3), "new mail announced");
    assert_eq!(r.d.mailer().list, vec![320, 5, 4], "newest first");
    assert_eq!(r.d.mailer().new_mail_num, 3);
    assert_eq!(r.d.state().mail(4), MailState::Unread, "seen by the desktop");
    r.idle(10);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mode(), Some(Icon::Mail));
    r.idle(5);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mailer().open_mail(), Some(320));
    assert_eq!(r.d.mailer().mail_mode, 2, "320 takes no reply");
    r.press(Buttons::CIRCLE);
    r.idle(6);
    assert_eq!(r.d.mailer().mail_mode, 1, "back to the list");
    assert_eq!(r.d.state().mail(320), MailState::Read);
    assert_eq!(r.d.mailer().new_mail_num, 2);
    r.requests.clear();
    r.press(Buttons::CIRCLE);
    r.idle(2);
    assert_eq!(r.d.mode(), None, "cancel leaves the mailer");
    assert!(r.se().contains(&7));
}

#[test]
fn reply_is_sent() {
    let mut state = SaveState::fresh();
    state.save.new_mail(8);
    let Some((mut r, _)) = Run::new(state) else { return };
    r.reach_main_screen();
    r.idle(10);
    r.press(Buttons::CROSS);
    r.idle(5);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mailer().mail_mode, 3, "mail 8 waits for a reply");
    r.idle(5);
    // Open the chooser, let it fade in, take the first reply, confirm YES.
    r.press(Buttons::CROSS);
    r.idle(20);
    r.press(Buttons::CROSS);
    r.idle(5);
    r.press(Buttons::CROSS);
    r.idle(20);
    r.press(Buttons::UP);
    r.press(Buttons::CROSS);
    r.idle(5);
    assert_eq!(r.d.state().mail(8), MailState::RepliedOne);
    // "Reply sent." until OK.
    r.idle(10);
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.d.mailer().mail_mode, 1);
    assert_eq!(r.d.mailer().new_mail_num, 0);
}

#[test]
fn the_world_asks_for_the_top_page() {
    let Some((mut r, _)) = Run::new(SaveState::fresh()) else { return };
    r.reach_main_screen();
    r.press(Buttons::UP);
    r.idle(40);
    assert_eq!(r.d.selection(), 0);
    r.requests.clear();
    r.press(Buttons::CROSS);
    r.idle(25);
    assert!(r.se().contains(&8));
    assert!(r.requests.contains(&Request::SoundFadeOut));
    assert!(r.requests.contains(&Request::ChangeMode { num: 4, sf: 7 }));
}

#[test]
fn locked_icon_does_not_open() {
    let mut state = SaveState::fresh();
    state.operate = 1 << Icon::Mail as u32;
    let Some((mut r, _)) = Run::new(state) else { return };
    r.reach_main_screen();
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mode(), None);
    assert_eq!(r.d.state().operate_set, Icon::Mail as i16, "the try is recorded for the events");
}

/// Every volume's mails are numbered by their row and linked to their
/// replies (147 on Infection); Infection's wallpapers are in its DATA.BIN.
#[test]
fn content_tables_read() {
    use piney_data::volume::Volume;
    use piney_desktop::content;
    for v in [Volume::Inf, Volume::Mut, Volume::Out, Volume::Qua] {
        for parody in [false, true] {
            let mails: Vec<_> = (0..content::mail_count(v)).map(|i| content::mail(v, i, parody)).collect();
            assert!(mails.iter().enumerate().all(|(i, m)| m.no == i as i32), "{v}: NO is the index");
            let replied: Vec<usize> = (0..mails.len()).filter(|&i| mails[i].one_res.is_some()).collect();
            if v == Volume::Inf {
                assert_eq!(replied.len(), 147);
            }
            assert!(replied.iter().all(|&i| mails[i].re_flg == 1 && mails[i].two_res.is_some()), "{v}");
            assert!(mails.iter().all(|m| m.mail_flg == 0 && !m.title.is_empty()), "{v}");
        }
        assert!(content::walls(v).iter().all(|w| w.anm_name.starts_with("ANM_")), "{v}");
    }
    let Some(path) = iso_path() else { return };
    let mut iso = Iso::open(path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    for w in content::walls(Volume::Inf) {
        assert!(archive.find(&w.name).is_some(), "wallpaper {}", w.name);
    }
}

#[test]
fn news_page_opens_and_is_read() {
    let mut state = SaveState::fresh();
    // Opcode 111 posts; 5 was read before.
    for n in 0..23 {
        state.save.set_webnews(n, 1);
    }
    state.save.set_webnews(5, 3);
    let Some((mut r, _)) = Run::new(state) else { return };
    r.reach_main_screen();
    assert_eq!(r.d.news().list.len(), 23);
    assert_eq!(r.d.news().new_web_num, 22);
    r.press(Buttons::DOWN);
    r.idle(30);
    assert_eq!(r.d.selection(), 2);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mode(), Some(Icon::News));
    r.press(Buttons::DOWN);
    r.requests.clear();
    r.press(Buttons::CROSS);
    assert_eq!(r.requests[..2], [Request::Se(Se::OPEN), Request::EnableReset(false)]);
    r.idle(2);
    assert_eq!(r.d.news().open_page(), Some(1));
    assert!(r.requests.contains(&Request::EnableReset(true)));
    // The page scrolls while down is held.
    for _ in 0..10 {
        r.step(Buttons::DOWN);
    }
    assert_eq!(r.d.news().hi, 30, "6 on held frames 1, 4, 6, 8, 10");
    r.step(Buttons::NONE);
    r.press(Buttons::CIRCLE);
    r.idle(2);
    assert_eq!(r.d.news().web_mode, 1, "back to the list");
    assert_eq!(r.d.state().save.webnews(1), 3, "read");
    assert_eq!(r.d.news().new_web_num, 21);
    r.requests.clear();
    r.press(Buttons::CIRCLE);
    r.idle(2);
    assert_eq!(r.d.mode(), None, "cancel leaves News after two frames");
    assert!(r.se().contains(&7));
}

#[test]
fn wallpaper_is_changed() {
    let mut state = SaveState::fresh();
    // WallTbl[3] and [7] unlocked (bits of dtWallpaperList).
    state.save.set_i32(piney_desktop::save::offset::DT_WALLPAPER_LIST, (1 << 3) | (1 << 7));
    let Some((mut r, _)) = Run::new(state) else { return };
    r.reach_main_screen();
    assert_eq!(r.d.accessory().list, vec![3, 7, 49, 50, 51], "unlocked, then the three originals");
    for _ in 0..2 {
        r.press(Buttons::DOWN);
        r.idle(30);
    }
    assert_eq!(r.d.selection(), 3);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mode(), Some(Icon::Wallpaper));
    // OK on the wallpaper up already does nothing but the sound.
    r.idle(1);
    r.press(Buttons::DOWN);
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    assert_eq!(r.d.state().dt_wallpaper(), 49, "not yet: the load is a task");
    r.idle(3);
    assert_eq!(r.d.state().dt_wallpaper(), 49, "49 was up");
    r.press(Buttons::UP);
    r.press(Buttons::UP);
    r.requests.clear();
    r.press(Buttons::CROSS);
    r.idle(3);
    assert_eq!(r.d.state().dt_wallpaper(), 3);
    assert_eq!(r.d.accessory().old_wall, 3);
    assert!(r.requests.contains(&Request::EnableReset(true)));
    r.requests.clear();
    r.press(Buttons::CIRCLE);
    r.idle(2);
    assert_eq!(r.d.mode(), None, "cancel leaves after two frames");
    assert!(r.se().contains(&7));
}

#[test]
fn music_is_changed() {
    let mut state = SaveState::fresh();
    // Wave[0] and [2] unlocked (bits of dtBgmList).
    state.save.set_i32(piney_desktop::save::offset::DT_BGM_LIST, 0b101);
    let Some((mut r, _)) = Run::new(state) else { return };
    r.reach_main_screen();
    assert_eq!(r.d.audio().wave_list, vec![0, 2, 50]);
    for _ in 0..3 {
        r.press(Buttons::DOWN);
        r.idle(30);
    }
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mode(), Some(Icon::Audio));
    r.idle(1);
    // Sound Mode, then the second piece.
    r.press(Buttons::CROSS);
    r.press(Buttons::DOWN);
    r.requests.clear();
    r.press(Buttons::CROSS);
    assert!(r.requests.contains(&Request::ChangeBgm { wave: 2, old: 50 }));
    assert_eq!(r.d.state().dt_bgm(), 2);
    assert!(r.requests.contains(&Request::EnableReset(true)), "BgmRead is seen done the next frame");
    // Cancel: back to the menu, then out after two frames.
    r.press(Buttons::CIRCLE);
    assert_eq!(r.d.audio().mode, 2);
    r.requests.clear();
    r.press(Buttons::CIRCLE);
    r.idle(2);
    assert_eq!(r.d.mode(), None);
    assert!(r.se().contains(&7));
}

#[test]
fn data_saves_to_a_files_card() {
    use piney_desktop::card::FilesCard;
    use piney_desktop::savesys::{INDEX_SIZE, SaveDataInfo};

    // The test's disc is Infection's.
    const DIR_NAME: &str = "BASLUS-20267DOTHACK";

    let mut state = SaveState::fresh();
    state.save.set_u8(piney_desktop::save::offset::SPC_PARAM + piney_desktop::save::offset::SPC_LEVEL, 7);
    state.save.set_i32(piney_desktop::save::offset::PLAY_TIME, ((3 * 60 + 25) * 60 + 7) * 60 + 30);
    let Some((mut r, _)) = Run::new(state) else { return };
    // An empty card in MEMORY CARD slot 1: no save directory yet.
    let root = std::env::temp_dir().join(format!("piney-desktop-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    r.d.set_card(Box::new(FilesCard::slot1(piney_data::volume::Volume::Inf, &root)));
    r.reach_main_screen();
    // Up from the mailer: The World, then the wrap to Data.
    for _ in 0..2 {
        r.press(Buttons::UP);
        r.idle(30);
    }
    assert_eq!(r.d.selection(), 5);
    r.requests.clear();
    r.press(Buttons::CROSS);
    assert_eq!(r.d.mode(), Some(Icon::Data));
    assert_eq!(r.requests[..2], [Request::EnableReset(false), Request::Se(Se::OPEN)]);
    assert_eq!(r.d.data().save_mode, 1, "choosing the card slot");
    // Slot 1: no save data; create it.
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.d.data().save_sys.result, 0x1032, "no saved data for .hack//INFECTION");
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.d.data().save_sys.result, 0x8033, "create new save data?");
    r.press(Buttons::UP);
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.d.data().save_sys.result, 0x1026, "save data created");
    assert!(root.join(DIR_NAME).join("dhdata12").exists());
    r.press(Buttons::CROSS);
    r.idle(4);
    assert_eq!(r.d.data().save_mode, 4, "card slot 1's list");
    // The first slot: empty, so "Create new data?"; YES writes.
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.d.data().save_sys.result, 0x801e);
    r.press(Buttons::UP);
    r.requests.clear();
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.d.data().save_sys.result, 0x101c, "data saved");
    assert!(r.se().contains(&74), "the jingle");
    let save = r.d.state().save.clone();
    let slot = std::fs::read(root.join(DIR_NAME).join("dhdata01")).unwrap();
    assert_eq!(slot, save.bytes().to_vec(), "the slot file is ccSaveData");
    let index = std::fs::read(root.join(DIR_NAME).join(DIR_NAME)).unwrap();
    assert_eq!(index.len(), INDEX_SIZE);
    let rec = SaveDataInfo::from_bytes(&index[..28]);
    assert_eq!((rec.status, rec.level, rec.clear_flag, rec.parody_flag), (1, 7, 0, 0));
    assert_eq!(rec.name(), b"Kite");
    assert_eq!(rec.sum, save.sum());
    assert_eq!(rec.sum, save.bytes().iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b))));
    assert_eq!(rec.playtime, save.play_time());
    assert!(index[28..].iter().all(|&b| b == 0), "the other slots empty");
    // Acknowledge: the index is read again and the list comes back on the
    // same slot; then out.
    r.press(Buttons::CROSS);
    r.idle(4);
    assert_eq!((r.d.data().save_mode, r.d.data().list_no), (4, 1));
    r.press(Buttons::CIRCLE);
    r.requests.clear();
    r.press(Buttons::CIRCLE);
    r.idle(2);
    assert_eq!(r.d.mode(), None, "cancel on the card slots leaves");
    assert!(r.se().contains(&7));
    assert!(r.requests.contains(&Request::EnableReset(true)));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn event_message_freezes_the_desktop() {
    use piney_desktop::message::MessageKind;
    let Some((mut r, _)) = Run::new(SaveState::fresh()) else { return };
    r.reach_main_screen();
    r.d.open_message(MessageKind::Speech, 0, None, &[b"A line."]);
    assert_eq!(r.d.menu_state(), (-1, 7), "the event asks for the fade menu");
    r.step(Buttons::NONE);
    assert_eq!(r.d.menu_state(), (7, -1));
    // The desktop sleeps: down does not move its cursor.
    let sel = r.d.selection();
    r.press(Buttons::DOWN);
    assert_eq!(r.d.selection(), sel);
    // The line is typed out by frame 40, so the first cross closes it two
    // checks later.
    let mut answers = Vec::new();
    for f in 0..80 {
        let b = if f == 40 || f == 50 { Buttons::CROSS } else { Buttons::NONE };
        r.step(b);
        let a = r.d.message_check(&r.pad);
        if a != 0 {
            answers.push((f, a));
            r.d.message_done();
            break;
        }
    }
    assert_eq!(answers, vec![(42, 1)]);
    for _ in 0..10 {
        r.step(Buttons::NONE);
    }
    assert!(r.se().contains(&18), "the window closes with sound 18");
    assert_eq!(r.d.menu_state(), (-1, -1), "the dim is gone and the menu closed");
    r.press(Buttons::DOWN);
    assert_ne!(r.d.selection(), sel, "the desktop takes input again");
}

#[test]
fn start_opens_the_options_and_title_screen_resets() {
    let Some((mut r, _)) = Run::new(SaveState::fresh()) else { return };
    r.reach_main_screen();
    r.requests.clear();
    r.press(Buttons::START);
    assert_eq!(r.d.menu_state().0, 0, "the system menu");
    assert!(r.se().contains(&16));
    // Cancel closes it after the dim fades.
    r.idle(12);
    r.press(Buttons::CIRCLE);
    assert!(r.se().contains(&19));
    r.idle(20);
    assert_eq!(r.d.menu_state(), (-1, -1));
    // Again: up wraps to Title Screen; OK, OK on "OK", OK again.
    r.press(Buttons::START);
    r.idle(12);
    r.press(Buttons::UP);
    r.press(Buttons::CROSS);
    r.idle(12);
    assert_eq!(r.d.menu_state().0, 6, "Title Screen's question");
    r.press(Buttons::UP);
    r.press(Buttons::CROSS);
    r.idle(20);
    r.press(Buttons::UP);
    r.requests.clear();
    r.press(Buttons::CROSS);
    assert!(r.requests.contains(&Request::ChangeMode { num: 1, sf: 7 }), "{:?}", r.requests);
}

#[test]
fn setup_line_runs_its_frames() {
    use piney_desktop::message::MessageKind;
    use piney_desktop::setup::SetupScreen;
    let Some(path) = iso_path() else { return };
    let mut iso = Iso::open(path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let state = SaveState::fresh();
    let mut s = SetupScreen::new(&mut iso, archive, &state).unwrap();
    let mut pad = Pad::default();
    // Frame 0: the event's Change, after the frame's step.
    pad.read(&Raw::default());
    assert!(s.step(&pad).cmds.is_empty(), "black");
    let a = [b'a'; 30];
    let b = [b'b'; 17];
    s.open_message(MessageKind::Speech, 0, None, &[&a, &b]);
    let mut answered = None;
    let mut alphas = Vec::new();
    for f in 1..60 {
        let push = if f == 19 || f == 21 { Buttons::CROSS } else { Buttons::NONE };
        pad.read(&Raw { buttons: push, ..Raw::default() });
        let frame = s.step(&pad);
        if answered.is_some() {
            alphas.push((f, s.window().map_or(-1, |m| m.window_alpha), frame.cmds.is_empty()));
        }
        // The event polls from its tenth frame (a Disp, then the Check).
        if f >= 10 && answered.is_none() {
            let r = s.message_check(&pad);
            if r != 0 {
                answered = Some(f);
            }
        }
        if answered.is_some_and(|p| f == p + 17) {
            s.close_message();
        }
    }
    // The tenth call reads the pad; the push at 19 shows everything, the
    // next push (21) answers two checks later.
    assert_eq!(answered, Some(23));
    assert_eq!(&alphas[..5], &[(24, 100, false), (25, 72, false), (26, 44, false), (27, 16, false), (28, 0, true)]);
    assert!(alphas.iter().filter(|a| a.0 >= 28).all(|a| a.2), "nothing after the fade");
    assert!(alphas.iter().any(|a| a.0 == 41 && a.1 == -1), "deleted at P + 17");
    assert!(s.take_requests().contains(&Request::Se(Se(18))));
}

#[test]
fn options_set_the_save() {
    use piney_desktop::save::offset;
    let Some((mut r, _)) = Run::new(SaveState::fresh()) else { return };
    r.reach_main_screen();
    // OPTION, down to Vibrate, OFF.
    r.press(Buttons::START);
    r.idle(12);
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    r.idle(22);
    assert_eq!(r.d.menu_state().0, 3, "Vibrate");
    r.press(Buttons::DOWN);
    r.requests.clear();
    r.press(Buttons::CROSS);
    assert_eq!(r.d.state().save.u8(offset::VIBRATION), 0);
    assert!(r.requests.contains(&Request::Vibration { on: false }));
    // Back to OPTION, down to Sound, one step less main volume.
    r.press(Buttons::CIRCLE);
    r.idle(22);
    assert_eq!(r.d.menu_state().0, 0);
    r.press(Buttons::DOWN);
    r.press(Buttons::DOWN);
    r.press(Buttons::CROSS);
    r.idle(22);
    assert_eq!(r.d.menu_state().0, 5, "Sound");
    r.requests.clear();
    r.press(Buttons::LEFT);
    assert_eq!(r.d.state().save.i16(offset::MAIN_VOL), 248);
    assert!(r.requests.contains(&Request::SoundEnv { main: 248, bgm: 256, se: 256, output: 1 }));
}

/// The new game's name entry from its first page: the two pages pressed
/// through, "ABC" typed as the user name, the default character name
/// accepted, YES, the last three pages; then the names are in the save.
#[test]
fn name_entry_writes_the_names() {
    use piney_desktop::name_entry::NameEntry;
    let Some(p) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut iso = Iso::open(p).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = SaveState::fresh();
    let mut ne = NameEntry::new(&mut iso, archive, &state).unwrap();
    let mut pad = Pad::default();
    let mut se = Vec::new();
    let mut drawn = 0;
    let mut step = |ne: &mut NameEntry, state: &mut SaveState, b: Buttons| {
        pad.read(&Raw { buttons: b, ..Raw::default() });
        let f = ne.step(&pad, state);
        drawn += usize::from(!f.cmds.is_empty());
        se.extend(ne.take_requests());
    };
    let mut press = |ne: &mut NameEntry, state: &mut SaveState, b: Buttons| {
        step(ne, state, b);
        step(ne, state, Buttons::NONE);
        step(ne, state, Buttons::NONE);
        // Until the fade has settled.
        for _ in 0..200 {
            if ne.done() || (ne.logic().tlans == 0 && ne.logic().move_lock == 0) {
                return;
            }
            step(ne, state, Buttons::NONE);
        }
        panic!("the fade does not settle");
    };
    press(&mut ne, &mut state, Buttons::NONE);
    press(&mut ne, &mut state, Buttons::CROSS);
    press(&mut ne, &mut state, Buttons::CROSS);
    assert_eq!((ne.logic().main_act, ne.logic().name_act), (7, 1), "the grid, the user name");
    for b in [Buttons::CROSS, Buttons::RIGHT, Buttons::CROSS, Buttons::RIGHT, Buttons::CROSS] {
        press(&mut ne, &mut state, b);
    }
    assert_eq!(&ne.logic().sur_name[..4], b"ABC_");
    // START puts the cursor on Enter.
    press(&mut ne, &mut state, Buttons::START);
    press(&mut ne, &mut state, Buttons::CROSS);
    assert_eq!(ne.logic().name_act, 2, "on to the character name");
    press(&mut ne, &mut state, Buttons::START);
    press(&mut ne, &mut state, Buttons::CROSS);
    assert_eq!(ne.logic().main_act, 17, "the dialog");
    press(&mut ne, &mut state, Buttons::UP);
    press(&mut ne, &mut state, Buttons::CROSS);
    assert_eq!(ne.logic().main_act, 19, "the last pages");
    for _ in 0..3 {
        press(&mut ne, &mut state, Buttons::CROSS);
    }
    assert!(ne.done());
    assert_eq!(state.save.name(), b"Kite");
    assert_eq!(state.save.real_name(), b"ABC");
    assert!(drawn > 100, "{drawn} frames drawn");
    for n in [4, 6, 17, 18] {
        assert!(se.contains(&Request::Se(Se(n))), "sound {n}");
    }
    // Nothing more once done.
    pad.read(&Raw { buttons: Buttons::CROSS, ..Raw::default() });
    assert!(ne.step(&pad, &mut state).cmds.is_empty());
}
