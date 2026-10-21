//! Issue #68: the town's party call. CHAT's first page in a town ("Everyone
//! gather!", "Let's scatter!", `townChatAct` 9 and 12) sends the order to
//! each member and opens Kite's line over him (`ccChatMsg::OpenChat(plw,
//! chatMenuStr[16 + row])`, MUT gcmn 0x00547ad8), then shuts the menu with
//! `Disp` in the same pass, which must find Kite on the lists and the
//! screen. The town's runtime answered only for the balloons already up.

use std::path::PathBuf;

use super::*;

/// `charTbl` row 15, a member on every disc.
const BLACK_ROSE: i32 = 15;
/// Kite's balloon handle (kind 1, id 0).
const KITE: u32 = 1 << 24;

/// A new game on `disc` logged in to Mac Anu with BlackRose in the party,
/// held until the town's tasks have run 30 frames.
fn mac_anu_with_black_rose(disc: &str) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
    if !iso.exists() {
        eprintln!("{disc}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    state.save.set_u8(offset::LAST_TOWN, 0);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    let spcs = Some(crate::start::party_of(&[BLACK_ROSE]));
    let mut s = Session::bare(iso, archive, false, None, false);
    s.resume_in(state, None, Resume::World(Box::new(InWorld { scene, world_man: None, spcs }))).unwrap();
    let mut pad = Pad::default();
    let playing = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    for _ in 0..900 {
        if playing(&s) {
            break;
        }
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
    assert!(playing(&s), "{disc}: not in Mac Anu: {}", Mode::title(&s));
    Some(s)
}

/// What one party call showed: Kite's line and the frames it was up, the
/// members' lines.
#[derive(Debug, Default)]
struct Call {
    kite: Option<String>,
    kite_frames: u32,
    members: Vec<String>,
}

/// CHAT (square), the `row`th order of its town page, OK; then 150 frames.
/// `each` sees every frame from the order on.
fn party_call(s: &mut Session, row: i16, mut each: impl FnMut(&Session, &Frame, u32)) -> Call {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut call = Call::default();
    let (mut ordered, mut since, mut after) = (false, 0u32, 0u32);
    for i in 0..600u32 {
        let raw = {
            let Stage::World(w) = &s.stage else { panic!("left the town: {}", Mode::title(s)) };
            let m = &w.ui().ctrl;
            since += 1;
            let press =
                |b: Buttons| if i.is_multiple_of(8) && since > 16 { Raw { buttons: b, ..still } } else { still };
            match w.ui().menu_type() {
                -1 if !ordered => press(Buttons::SQUARE),
                3 if m.list().page != 0 => press(Buttons::LEFT),
                3 if m.list().select < row => press(Buttons::DOWN),
                3 if m.list().select > row => press(Buttons::UP),
                3 => press(Buttons::CROSS),
                _ => still,
            }
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::World(w) = &s.stage else { panic!("left the town") };
        let m = &w.ui().ctrl;
        ordered |= m.menu == 3 && m.menu_next == -1;
        if !ordered {
            continue;
        }
        for slot in m.chat.slots.iter().filter(|x| x.cf > 0) {
            let text = String::from_utf8_lossy(&slot.text).into_owned();
            if slot.who == KITE {
                call.kite_frames += 1;
                call.kite.get_or_insert(text);
            } else if slot.who >> 24 == 1 && !call.members.contains(&text) {
                call.members.push(text);
            }
        }
        each(s, &frame, after);
        after += 1;
        if after >= 150 {
            break;
        }
    }
    assert!(ordered, "the order was not given");
    call
}

/// On every disc, both town orders put Kite's line over him for its 120
/// frames (the order's pass is still: `Disp(1)` does not count it down, as
/// tools/test_chat_msg_rs.py's `test_party_call` holds), and BlackRose
/// answers.
#[test]
fn kite_calls_the_party_in_town() {
    for disc in ["infection", "mutation", "outbreak", "quarantine"] {
        let Some(mut s) = mac_anu_with_black_rose(disc) else { continue };
        for (row, line) in [(0, "Everyone gather!"), (1, "Let's scatter!")] {
            let call = party_call(&mut s, row, |_, _, _| {});
            eprintln!("{disc} row {row}: {call:?}");
            assert_eq!(call.kite.as_deref(), Some(line), "{disc} row {row}: Kite's line: {call:?}");
            assert_eq!(call.kite_frames, 120, "{disc} row {row}: Kite's balloon frames: {call:?}");
            assert!(!call.members.is_empty(), "{disc} row {row}: BlackRose's answer: {call:?}");
        }
    }
}

/// The field's orders (Infection's event 3, Orca along): Skill Usage's
/// first, "Everyone, " and the order, over Kite for its two seconds; the
/// field's runtime answers for the party as well (`hud_world`).
#[test]
fn kite_calls_the_party_in_a_field() {
    let Some((mut s, mut f)) = story_to_field(0) else { return };
    let mut log = Vec::new();
    story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut ordered, mut line, mut frames) = (false, None, 0u32);
    for i in 0..400u32 {
        let raw = {
            let Stage::Area(a) = &s.stage else { panic!("left the field: {}", Mode::title(&s)) };
            let m = &a.ui().ctrl;
            let press = |b: Buttons| if i.is_multiple_of(8) && i > 16 { Raw { buttons: b, ..still } } else { still };
            match a.ui().menu_type() {
                -1 if !ordered => press(Buttons::SQUARE),
                3 if m.list().page != 0 => press(Buttons::LEFT),
                3 if m.list().select != 0 => press(Buttons::UP),
                3 => press(Buttons::CROSS),
                _ => still,
            }
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { panic!("left the field") };
        let m = &a.ui().ctrl;
        ordered |= m.menu == 3 && m.menu_next == -1;
        if let Some(slot) = m.chat.slots.iter().find(|x| ordered && x.cf > 0 && x.who == KITE) {
            frames += 1;
            line.get_or_insert_with(|| String::from_utf8_lossy(&slot.text).into_owned());
        }
    }
    eprintln!("field: {line:?} for {frames} frames");
    assert!(line.as_deref().is_some_and(|l| l.starts_with("Everyone, ")), "Kite's line: {line:?}");
    assert_eq!(frames, 120, "Kite's balloon up {frames} frames");
}

/// Shots of Mutation's party call ("Everyone gather!"), 20 and 60 frames
/// after the order, into `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/i68): `cargo test --release -p piney-game
/// party_call_shots -- --ignored --nocapture`.
#[test]
#[ignore]
fn party_call_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i68".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = mac_anu_with_black_rose("mutation") else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    party_call(&mut s, 0, |s, frame, n| {
        if n != 20 && n != 60 {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (wd, ht) = g.target_size();
        let path = format!("{dir}/gather-{n:03}.png");
        std::fs::write(&path, piney_gs::png::encode(wd, ht, &g.read_back())).unwrap();
        println!("{path}");
    });
}
