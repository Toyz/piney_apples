//! The top page against Infection's disc: its tables, the log-in, the
//! menu's commands and their hand-offs, the board read and written, driven
//! by pad input as the game reads it. Skipped when the disc image is not
//! extracted; set PINEY_ISO to point at it elsewhere.
//!
//! The game-code checks of the logic, frame by frame, are in
//! tools/test_toppage_rs.py.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_draw::Frame;
use piney_input::{Buttons, Pad, Raw};
use piney_toppage::assets::Assets;
use piney_toppage::bbs::{POST_NEW, POST_READ, POST_WRITE, bbs_state, set_bbs_state};
use piney_toppage::control::{MODE_BBS, MODE_ENTER, MODE_EXIT, MODE_NORMAL};
use piney_toppage::{Phase, Request, SaveState, TopPage};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn open() -> Option<(Iso, Arc<Archive>)> {
    let mut iso = Iso::open(iso_path()?).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    Some((iso, archive))
}

struct Run {
    t: TopPage,
    pad: Pad,
    requests: Vec<Request>,
    frame: Frame,
}

impl Run {
    fn new(state: SaveState) -> Option<Run> {
        let (mut iso, archive) = open()?;
        let t = TopPage::new(&mut iso, archive, state).unwrap();
        Some(Run { t, pad: Pad::default(), requests: Vec::new(), frame: Frame::new() })
    }

    fn step(&mut self, b: Buttons) -> Vec<Request> {
        self.pad.read(&Raw { buttons: b, analog: false, ..Raw::default() });
        self.frame = self.t.step(&self.pad);
        let r = self.t.take_requests();
        self.requests.extend(r.iter().cloned());
        r
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.step(Buttons::NONE);
        }
    }

    fn press(&mut self, b: Buttons) -> Vec<Request> {
        let r = self.step(b);
        self.step(Buttons::NONE);
        r
    }

    /// Past the log-in animation (cancel on its second frame) to the menu.
    fn reach_menu(&mut self) {
        self.idle(3);
        self.press(Buttons::CIRCLE);
        assert_eq!(self.t.control().mode, MODE_NORMAL);
    }

    /// The board, from the menu: down to BBS, OK, and the fade's frames.
    fn reach_board(&mut self) {
        self.press(Buttons::DOWN);
        assert_eq!(self.t.control().cmd, 1);
        self.press(Buttons::CROSS);
        self.idle(31);
        assert_eq!(self.t.control().mode, MODE_BBS);
    }
}

fn event1_posts() -> SaveState {
    let mut s = SaveState::fresh();
    for p in 0..7 {
        set_bbs_state(&mut s, 62, p, POST_NEW);
    }
    s
}

#[test]
fn tables_and_names() {
    let Some((mut iso, archive)) = open() else { return };
    let a = Assets::read(&mut iso, archive).unwrap();
    assert_eq!(a.volume, 1);
    assert_eq!(a.neutral, "ANM_xdttop1a");
    assert_eq!(a.servers, [0, 1, 2, 3, 4, 0, 1, 2]);
    for tbl in &a.bbs.tables {
        assert_eq!(tbl.len(), 63);
        assert_eq!(tbl.iter().map(|t| t.msgs.len()).sum::<usize>(), 341);
        assert_eq!(tbl[62].msgs.len(), 7);
        assert!(tbl.iter().all(|t| t.msgs.iter().all(|m| m.lines.len() == m.max_lines as usize)));
    }
    // The parody table is its own script.
    assert_ne!(a.bbs.tables[0][62].title, a.bbs.tables[1][62].title);
    assert!(a.bbs.labels.author.starts_with(b"Author"));
    assert_eq!(a.bbs.mask_tex.as_ref().map(|t| t.1), Some(128));
    let names: Vec<_> = a.bbs.file.anims.iter().filter_map(|x| a.bbs.file.ccs.object_name(x.object)).collect();
    for n in ["ANM_xdttopst", "ANM_xdttop1a", "ANM_xdtbbsa0", "ANM_xdtbbsa1", "ANM_xdtcame0"] {
        assert!(names.contains(&n), "{n}");
    }
}

#[test]
fn log_in_animation_and_log_out() {
    let Some(mut r) = Run::new(SaveState::fresh()) else { return };
    let first = r.step(Buttons::NONE);
    assert_eq!(first, vec![Request::AllSoundOff, Request::SqLoad(0), Request::EnableReset(true), Request::SqPlay(0)]);
    assert!(r.frame.cmds.is_empty());
    // The animation's frames 1 and 60 sound 9 and 10; it runs to its end.
    assert_eq!(r.step(Buttons::NONE), vec![Request::Se(9)]);
    let mut n = 1;
    while r.t.control().mode == MODE_ENTER {
        let q = r.step(Buttons::NONE);
        n += 1;
        if n == 60 {
            assert_eq!(q, vec![Request::Se(10)]);
        }
        assert!(n < 400);
    }
    assert!(!r.frame.cmds.is_empty());
    // Log out: down twice, OK; 31 frames of fade, then the desktop.
    r.press(Buttons::DOWN);
    r.press(Buttons::DOWN);
    assert_eq!(r.t.control().cmd, 2);
    let q = r.step(Buttons::CROSS);
    assert_eq!(q, vec![Request::Se(4), Request::SoundFadeOut, Request::EnableReset(false)]);
    assert_eq!(r.t.control().mode, MODE_EXIT);
    for _ in 0..30 {
        assert!(r.step(Buttons::NONE).is_empty());
    }
    assert_eq!(r.step(Buttons::NONE), vec![Request::ChangeMode { num: 3, sf: 7 }]);
    assert_eq!(r.t.phase(), Phase::Left);
    let held = r.frame.clone();
    r.idle(3);
    assert_eq!(r.frame, held);
    assert!(r.requests.iter().all(|q| !matches!(q, Request::ChangeMode { num: 5, .. })));
}

#[test]
fn log_in_hands_off_to_the_last_town() {
    let mut s = SaveState::fresh();
    s.save.set_u8(offset::LAST_TOWN, 3);
    let Some(mut r) = Run::new(s) else { return };
    r.reach_menu();
    assert_eq!(r.step(Buttons::CROSS), vec![Request::Se(13), Request::SoundFadeOut, Request::EnableReset(false)]);
    r.idle(30);
    let q = r.step(Buttons::NONE);
    assert_eq!(
        q,
        vec![
            Request::ChangeMode { num: 5, sf: 8 },
            Request::ChangeArea { area: 0, town: 3 },
            Request::ChangeMode { num: 6, sf: 7 }
        ]
    );
    let s = r.t.login_scene(3);
    assert_eq!((s.area, s.town, s.server, s.field, s.dungeon), (0, 3, 3, -1, -1));
}

#[test]
fn the_board_marks_posts_read() {
    let Some(mut r) = Run::new(event1_posts()) else { return };
    r.reach_menu();
    assert!(r.t.control().bbs_new);
    r.reach_board();
    let b = &r.t.control().bbs;
    assert_eq!((b.draw_state, b.th_num()), (0, 1));
    assert!(!r.frame.uploads.is_empty(), "the thread's title is drawn");
    // The thread, its first post read, closed: read in the save.
    r.press(Buttons::CROSS);
    assert_eq!(r.t.control().bbs.draw_state, 1);
    r.press(Buttons::CROSS);
    assert_eq!(r.t.control().bbs.msg_page.i_draw_state, 1);
    r.idle(3);
    r.press(Buttons::CIRCLE);
    assert_eq!(bbs_state(r.t.state(), 62, 0), POST_READ);
    assert_eq!(bbs_state(r.t.state(), 62, 1), POST_NEW);
    // Back to the threads, out of the board: the menu again, with its
    // flash; the other posts are still new.
    r.press(Buttons::CIRCLE);
    assert_eq!(r.t.control().bbs.draw_state, 0);
    r.press(Buttons::CIRCLE);
    assert_eq!(r.t.control().mode, MODE_NORMAL);
    assert!(r.t.control().bbs_new);
}

#[test]
fn the_players_own_post_is_written_out() {
    let mut s = event1_posts();
    set_bbs_state(&mut s, 62, 2, POST_WRITE);
    let Some(mut r) = Run::new(s) else { return };
    r.reach_menu();
    r.reach_board();
    let w = &r.t.control().bbs.write;
    assert_eq!((r.t.control().bbs.draw_state, w.th_index, w.msg_index), (2, 62, 2));
    let line = w.line_index;
    r.idle(200);
    assert!(r.t.control().bbs.write.line_index > line, "typed on");
    // OK shows the rest; OK again opens the thread on the post.
    r.press(Buttons::CROSS);
    r.idle(2);
    assert_eq!(r.t.control().bbs.write.b_draw_end, 1);
    r.press(Buttons::CROSS);
    let b = &r.t.control().bbs;
    assert_eq!((b.draw_state, b.msg_page.index), (1, 2));
    assert_eq!(bbs_state(r.t.state(), 62, 2), POST_READ);
}

#[test]
fn start_opens_the_system_menu_over_the_page() {
    let Some(mut r) = Run::new(SaveState::fresh()) else { return };
    r.reach_menu();
    r.idle(40);
    let q = r.press(Buttons::START);
    assert!(q.contains(&Request::Se(16)), "{q:?}");
    assert_eq!(r.t.menu_state().0, 0);
    // The top page sleeps under it: its cursor does not move.
    r.press(Buttons::DOWN);
    assert_eq!(r.t.control().cmd, 0);
}

/// Issue #25: on the top page the Key of the Twilight (`MDL_xdtworld1`)
/// shows on the sword through its glow (`MDL_xdthotw`). Both are
/// translucent and each writes Z where its alpha passes; the glow stands
/// 1000 units nearer. `ccModel::Draw` keys a rigid model on the screen Z of
/// its vertex box's centre, so the farther text is drawn first and the glow
/// over it. Keyed as a model without a box (the w-row key, which the glow's
/// 1.0-1.2 scale puts first), the glow's Z hid the text.
#[test]
fn the_key_of_the_twilight_is_drawn_before_its_glow() {
    let Some(mut r) = Run::new(SaveState::fresh()) else { return };
    let (_, archive) = open().unwrap();
    let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named("xdttopen0").unwrap()).unwrap();
    let index = |name: &str| (0..4096u32).find(|&i| ccs.object_name(i) == Some(name)).unwrap();
    let (text, glow) = (index("MDL_xdtworld1"), index("MDL_xdthotw"));
    let mut done = 0;
    for n in [150usize, 300, 421] {
        r.idle(n - done);
        done = n;
        let order: Vec<u32> = r
            .frame
            .cmds
            .iter()
            .filter_map(|c| match c {
                piney_draw::Cmd::Model(m) if m.file == "xdttopen0" => Some(m.model),
                _ => None,
            })
            .collect();
        let at = |m: u32| order.iter().position(|&x| x == m);
        let (t, g) = (at(text), at(glow));
        assert!(t.is_some() && g.is_some(), "frame {n}: drawn {order:?}");
        assert!(t < g, "frame {n}: the glow drawn before the text: {order:?}");
    }
}

/// The sort centres `ccBbox_SetBox` leaves in the model chunks, as the
/// game's own `Decode_Model` gives them in eemu.
#[test]
fn model_box_centres_are_the_games() {
    let Some((_, archive)) = open() else { return };
    let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named("xdttopen0").unwrap()).unwrap();
    let models = piney_data::model::models(&ccs).unwrap();
    let centre = |name: &str| {
        let m = models.iter().find(|m| ccs.object_name(m.object) == Some(name)).unwrap();
        piney_desktop::assets::vertex_box_centre(m).to_array()
    };
    assert_eq!(centre("MDL_xdtsword"), [0.0, -6784.0, 848.0]);
    assert_eq!(centre("MDL_xdtworld1"), [-3.125, 0.0, 0.0]);
    assert_eq!(centre("MDL_xdthotw"), [0.0, 0.0, 0.0]);
}
