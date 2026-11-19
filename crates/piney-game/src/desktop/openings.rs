//! Each disc's opening on the desktop against the game's own code
//! (`tools/test_event_vm.py openings`): New Game from the boot through the
//! title to the desktop and into play, as the committed
//! `tests/opening_fixture*.txt` hold it, and CONVERT on the later discs
//! from a cleared save of the volume before, as
//! `work/analysis/openings/convert_*.txt` holds it (made from the user's own
//! save, so kept out of the repository). The port's boot, title and
//! conversion make the start, the event task and the Bridge play it.

use std::path::{Path, PathBuf};

use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_desktop::SaveState;
use piney_event::host::{Announce, DesktopMenu, Game, Host, MessageCall, MessageKind, Overlay, StoryArea};

use super::{Bridge, HostState, Target, boot_with};

type Runs = Vec<(usize, Vec<u8>)>;

/// What the player does after a frame of play.
#[derive(Clone, Copy, Debug)]
enum Action {
    /// `ccEvent::CheckOperate(num, 0)`, as the desktop asks it.
    Operate(i32),
    /// The mail read (`mailList` 4).
    Read(usize),
    /// The news read (`webnewsList` 3).
    News(usize),
}

/// One opening as the game ran it.
#[derive(Default)]
struct Opening {
    actions: Vec<(u32, Action)>,
    extra: u32,
    /// CONVERT's slot as `LoadDataPrevReq` left it.
    loaded: Option<Vec<u8>>,
    /// FNV-1a 64 of the slot at the desktop's start (it holds `charTbl`'s
    /// rows, so the fixture keeps the hash).
    start: u64,
    phases: Vec<i32>,
    calls: Vec<String>,
    saves: Vec<(u8, u64, Runs)>,
}

fn parse_runs(words: &[&str]) -> Runs {
    words
        .iter()
        .map(|w| {
            let (at, bytes) = w.split_once(':').expect("a run is OFFSET:BYTES");
            let (bytes, times) = bytes.split_once('*').map_or((bytes, 1), |(b, n)| (b, n.parse().unwrap()));
            (usize::from_str_radix(at, 16).unwrap(), hex(bytes).repeat(times))
        })
        .collect()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// `FRAME:WHAT:VALUE`, as `write_opening` gives an action.
fn action(a: &str) -> (u32, Action) {
    let (at, rest) = a.split_once(':').expect("FRAME:WHAT:VALUE");
    let (what, v) = rest.split_once(':').expect("FRAME:WHAT:VALUE");
    let action = match what {
        "operate" => Action::Operate(v.parse().unwrap()),
        "read" => Action::Read(v.parse().unwrap()),
        "news" => Action::News(v.parse().unwrap()),
        _ => panic!("action {a}"),
    };
    (at.parse().unwrap(), action)
}

fn parse(text: &str) -> Opening {
    let mut o = Opening::default();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["actions", rest @ ..] => o.actions = rest.iter().map(|a| action(a)).collect(),
            ["extra", n] => o.extra = n.parse().unwrap(),
            ["loaded", h] => o.loaded = Some(hex(h)),
            ["start", h] => o.start = u64::from_str_radix(h, 16).unwrap(),
            ["phases", p @ ..] => o.phases = p.iter().map(|x| x.parse().unwrap()).collect(),
            ["call", rest @ ..] => o.calls.push(rest.join(" ")),
            ["save", stage, frame, runs @ ..] => {
                o.saves.push((stage.parse().unwrap(), frame.parse().unwrap(), parse_runs(runs)));
            }
            _ => panic!("{line}"),
        }
    }
    o
}

fn applied(base: &[u8], runs: &Runs) -> Vec<u8> {
    let mut out = base.to_vec();
    for (at, bytes) in runs {
        out[*at..*at + bytes.len()].copy_from_slice(bytes);
    }
    out
}

/// FNV-1a 64, as `tools/test_event_vm.py` hashes.
fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

fn first_difference(got: &[u8], want: &[u8]) -> String {
    match got.iter().zip(want).position(|(a, b)| a != b) {
        Some(i) => format!("first difference at 0x{i:x}: port {:02x}, game {:02x}", got[i], want[i]),
        None => format!("lengths {} and {}", got.len(), want.len()),
    }
}

/// The Bridge as the harness's hooks see the rest of the game: windows,
/// streams, overlays and name entry finish at once and are logged with
/// their frame; the save, the mode, the story areas and `grunty_mail` are
/// the Bridge's.
struct Logged<'a> {
    b: Bridge<'a>,
    frame: u64,
    calls: Vec<String>,
}

impl Logged<'_> {
    fn log(&mut self, s: &str) {
        self.calls.push(format!("{} {s}", self.frame));
    }
}

impl Host for Logged<'_> {
    fn save(&mut self) -> &mut SaveData {
        self.b.save()
    }
    fn game(&self) -> Game {
        self.b.game()
    }
    fn story_area(&self, area: i16) -> Option<StoryArea> {
        self.b.story_area(area)
    }
    fn grunty_mail(&mut self) {
        self.b.grunty_mail();
    }
    fn generate_area(&mut self, words: [Option<i32>; 3]) {
        self.b.generate_area(words);
    }
    fn desktop_menu(&self) -> DesktopMenu {
        DesktopMenu { menu_type: -1, request: -1 }
    }
    fn clear_gate_hack(&mut self) {
        self.log("clear_gate_hack");
    }
    fn set_frame_rate(&mut self, rate: i16) {
        self.log(&format!("frame_rate {rate}"));
    }
    fn load_overlay(&mut self, which: Overlay) {
        let n = match which {
            Overlay::Demo => 0,
            Overlay::Desktop => 1,
            Overlay::Toppage => 2,
            Overlay::Gcmn => 3,
        };
        self.log(&format!("overlay {n}"));
    }
    fn stream(&mut self, num: i16) {
        self.log(&format!("stream {num}"));
    }
    fn message_open(&mut self, c: &MessageCall<'_>) {
        let kind = if c.kind == MessageKind::Speech { "message" } else { "info" };
        self.log(&format!("{kind} {} {}", c.event, c.msg));
    }
    fn announce(&mut self, _a: Announce) {
        self.log("info -1 -1");
    }
    fn name_entry_start(&mut self) {
        self.log("name_entry");
    }
    fn sound_effect(&mut self, se: i32) {
        self.log(&format!("se {se}"));
    }
    fn sound(&mut self, cmd: i16, p0: i16, p1: i16, p2: i16) {
        self.log(&format!("sound {cmd} {p0} {p1} {p2}"));
    }
    fn menu_ban(&mut self, on: bool) {
        self.log(&format!("menu_ban {}", i32::from(on)));
    }
    fn change_request(&mut self, num: i32, sf: i32) {
        self.log(&format!("change_request {num} {sf}"));
    }
}

/// The port's start and play of `want` on the disc `iso`: the boot (the
/// game's scripts, without the port's own additions), then New Game's
/// `NewGame(1)` and `NewGame(0)` or CONVERT's `ccStartEventConvert` and
/// `ConvGame` on the loaded slot; then `ccSetupDesktop`'s passes at 0, 2
/// and 4, each asked once the one before settled, and play with the
/// player's actions. Compared with the game's at every step.
fn check(iso: &Path, want: &Opening, what: &str) {
    use piney_demo::newgame::{NewGameTables, SAVE_VA, conv_game, new_game, start_event_convert};
    let mut disc = Iso::open(iso).unwrap();
    let volume = disc.volume().unwrap();
    let text = piney_desktop::InitText::from_disc(&mut disc).unwrap();
    let mut state = SaveState::fresh_with(&text);
    let events = piney_event::official::events(&mut disc).unwrap().to_vec();
    let story = std::sync::Arc::new(crate::story::Announcements::from_disc(&mut disc).unwrap());
    let mut vm = boot_with(volume, events, story, &mut state);
    let tables = NewGameTables::of(volume);
    match &want.loaded {
        None => {
            new_game(&mut state.save, 1, &tables, SAVE_VA);
            new_game(&mut state.save, 0, &tables, SAVE_VA);
        }
        Some(loaded) => {
            state.save = SaveData::from_bytes(loaded).unwrap();
            start_event_convert(&mut state.save, volume);
            conv_game(&mut state.save, &tables, SAVE_VA);
        }
    }
    let start = state.save.slot_bytes(volume).to_vec();
    assert_eq!(fnv(&start), want.start, "{what}: the desktop's start");

    let mut st = HostState { status: 2, ..HostState::default() };
    let mut h =
        Logged { b: Bridge { target: Target::Setup(&mut state, None), st: &mut st }, frame: 1, calls: Vec::new() };
    vm.start_thread(&mut h);
    vm.enable(0);
    let (mut stage, mut left) = (0, want.extra);
    let mut phases = Vec::new();
    let mut saves = Vec::new();
    for frame in 1..=10_000u64 {
        h.frame = frame;
        vm.frame(&mut h);
        let phase = vm.phase();
        phases.push(phase);
        match (stage, phase) {
            (0, 1) => {
                saves.push((0, frame, h.b.save_ref().slot_bytes(volume).to_vec()));
                vm.enable(2);
                stage = 1;
            }
            (1, 3) => {
                saves.push((2, frame, h.b.save_ref().slot_bytes(volume).to_vec()));
                vm.enable(4);
                stage = 2;
            }
            (2, 5) => {
                left -= 1;
                let done = want.extra - left;
                for &(_, action) in want.actions.iter().filter(|(at, _)| *at == done) {
                    match action {
                        Action::Operate(n) => {
                            let go = vm.check_operate(n, 0, &mut h);
                            h.calls.push(format!("{frame} check_operate {n} {}", i32::from(go)));
                        }
                        Action::Read(mail) => h.save().set_mail(mail, 4),
                        Action::News(news) => h.save().set_webnews(news, 3),
                    }
                }
                if left == 0 {
                    saves.push((4, frame, h.b.save_ref().slot_bytes(volume).to_vec()));
                    break;
                }
            }
            _ => {}
        }
    }
    assert_eq!(phases, want.phases, "{what}: the phase after each frame");
    assert_eq!(h.calls, want.calls, "{what}: the calls and their frames");
    assert_eq!(saves.len(), want.saves.len(), "{what}");
    for ((stage, frame, got), (want_stage, want_frame, runs)) in saves.iter().zip(&want.saves) {
        assert_eq!((stage, frame), (want_stage, want_frame), "{what}");
        let want = applied(&start, runs);
        assert!(*got == want, "{what}: phase {stage}: {}", first_difference(got, &want));
    }
    eprintln!("{what}: {} frames, {} calls, the start and 3 saves match the game", phases.len(), h.calls.len());
}

/// New Game on every disc, and CONVERT on the later ones where the user's
/// cleared save has made its fixture, against the game's.
#[test]
fn openings_match_the_game() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (volume, disc, suffix) in [
        (Volume::Inf, "infection", ""),
        (Volume::Mut, "mutation", "_mut"),
        (Volume::Out, "outbreak", "_out"),
        (Volume::Qua, "quarantine", "_qua"),
    ] {
        let iso = crate::check_image(disc);
        if !iso.exists() {
            eprintln!("{disc}.iso not present; skipped");
            continue;
        }
        let new = std::fs::read_to_string(root.join(format!("tests/opening_fixture{suffix}.txt"))).unwrap();
        check(&iso, &parse(&new), &format!("{volume} New Game"));
        // Infection's title has no CONVERT.
        if volume == Volume::Inf {
            continue;
        }
        let convert = root.join(format!("../../work/analysis/openings/convert_{disc}.txt"));
        match std::fs::read_to_string(&convert) {
            Ok(text) => check(&iso, &parse(&text), &format!("{volume} CONVERT")),
            Err(_) => eprintln!("{}: not made; CONVERT skipped", convert.display()),
        }
    }
}
