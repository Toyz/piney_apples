//! Event 22 (PIRO02) in Mac Anu: Piros drinks Mia's potion, and the
//! event's `piros_colour` turns him orange ([`crate::piros`]).

use std::path::PathBuf;

use piney_input::{Buttons, Raw};

use super::*;

fn still(buttons: Buttons) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// `--mode story:22` from the board to Mac Anu (LOG IN), the event task
/// recording its blocks.
fn story_22_in_mac_anu() -> Option<Session> {
    story_in_mac_anu(22, 0)
}

/// `--mode story:N` from its start to Mac Anu (the board's LOG IN; a
/// desktop start logged in at once), the blocks in `played` marked played,
/// the event task recording its blocks.
fn story_in_mac_anu(n: i32, played: u64) -> Option<Session> {
    use piney_event::ScriptSave;
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut start = crate::start::build(&iso, n).unwrap();
    start.state.save.update_flags(n, |f| f | played);
    start.vm.trace = Some(Vec::new());
    if matches!(start.at, crate::session::Resume::Desktop) {
        // A desktop start: logged in to Mac Anu at once.
        let scene = piney_world::area::Scene::log_in(&mut start.state.save);
        start.at =
            crate::session::Resume::World(Box::new(crate::session::InWorld { scene, world_man: None, spcs: None }));
    }
    let mut s = Session::resume(iso, archive, None, start.state, start.vm, start.at).unwrap();
    let mut pad = Pad::default();
    let mut n = 0;
    while !matches!(s.stage, Stage::World(_)) {
        let b = match n {
            200 => Buttons::CIRCLE,
            n if n > 240 && n % 30 == 0 => Buttons::CROSS,
            _ => Buttons::NONE,
        };
        pad.read(&still(b));
        s.step(&pad);
        s.take_events();
        n += 1;
        if n > 3000 {
            return None;
        }
    }
    Some(s)
}

/// Windows closed every 24 frames.
fn player(w: &WorldMode, f: u64) -> Raw {
    let waiting = w.calls().iter().rev().find_map(|(_, c)| {
        if c.starts_with("message_open") || c.starts_with("announce") {
            Some(true)
        } else if c.starts_with("message_check") {
            Some(false)
        } else {
            None
        }
    });
    still(if waiting == Some(true) && f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE })
}

/// Piros's tint as the town holds it.
fn tint(w: &WorldMode) -> Option<piney_world::char::AffectColour> {
    w.world().fellow_affect(crate::piros::PIROS)
}

/// Block 4: two `status_add 1` and `piros_colour 0` (status 1: the flash
/// and the tint, five frames, again; status 2: the tint held), then
/// "Aaaargh!!!". Piros stays tinted orange (fix 1, rate 65, 0x002080ff),
/// and no instruction on the way fell to a host default.
#[test]
fn event_22_turns_piros_orange() {
    let Some(mut s) = story_22_in_mac_anu() else { return };
    piney_event::host::take_unported();
    let mut pad = Pad::default();
    let mut seen = None;
    for f in 0..6000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage
            && w.calls().iter().filter(|(_, c)| c.starts_with("piros_colour")).count() >= 2
            && tint(w).is_some_and(|t| t.fix == 1)
        {
            seen = tint(w);
            break;
        }
    }
    let Stage::World(w) = &s.stage else { panic!("left the town: {}", Mode::title(&s)) };
    let calls: Vec<String> = w.calls().iter().rev().take(30).rev().map(|(f, c)| format!("{f} {c}")).collect();
    let t = seen.unwrap_or_else(|| panic!("no tint: {}\n{}", Mode::title(&s), calls.join("\n")));
    assert_eq!((t.fix, t.rate, t.colour), (1, 65, 0x0020_80ff));
    assert_eq!(piney_event::host::take_unported(), Vec::<&str>::new());
}

/// Pictures of the potion: the flash, and Piros tinted at "Aaaargh!!!".
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game event_22_shot --
/// --ignored --nocapture` (default `/mnt/data/claude/scratch/event22`).
#[test]
#[ignore]
fn event_22_shot() {
    let Some(mut s) = story_22_in_mac_anu() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/event22".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut tinted = None;
    let mut flashed = None;
    let mut aargh = None;
    for f in 0..6000u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::World(w) = &s.stage else { continue };
        if flashed.is_none() && w.calls().iter().any(|(_, c)| c.starts_with("piros_colour")) {
            flashed = Some(f);
        }
        if tinted.is_none() && tint(w).is_some_and(|t| t.fix == 1) {
            tinted = Some(f);
        }
        // "Aaaargh!!!": the first window after the tint, on Piros.
        if let Some(t) = tinted
            && aargh.is_none()
            && w.calls().iter().any(|(at, c)| *at >= t && c.starts_with("message_open"))
        {
            aargh = Some(f);
        }
        let name = match (flashed, aargh) {
            (Some(a), _) if f == a + 2 => Some("1-flash"),
            (_, Some(t)) if f == t + 20 => Some("2-orange"),
            _ => None,
        };
        if let Some(name) = name {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/{name}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            if name == "2-orange" {
                break;
            }
        }
    }
}

/// Story area 31 (the field event 22 sends the party to) from Mac Anu, its
/// field or (`dungeon`) its first dungeon's first room: story:22's save
/// and event task with Piros in the party and `eventStatus[1]` 2 (the
/// potion drunk in the town: the tint held).
fn story_22_in_area_31(dungeon: bool) -> Option<Session> {
    use piney_world::area::{Scene, kind};
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut start = crate::start::build(&iso, 22).unwrap();
    start.vm.trace = Some(Vec::new());
    let save = &mut start.state.save;
    // eventStatus[1] 2; eventStatus[0] is the board's ([`board_done`]).
    save.set_u8(crate::piros::STATUS, 2);
    let mut scene = Scene::log_in(save);
    let wm = crate::area::ev_area_world_man(&iso, 31, scene.server, save).unwrap().expect("area 31's words");
    scene.go(piney_data::area::Go::ChangeScene([1, 0, 31, -1, -1, -1]), save);
    if dungeon {
        scene.change_area(kind::DUNGEON, 0, save);
    }
    let mut spcs = piney_world::party::Spcs::new_game();
    let i = spcs.entry_spc(crate::piros::PIROS) as usize;
    spcs.registry[i].party_flag = 1;
    spcs.member_id[1] = crate::piros::PIROS;
    spcs.member_char[1] = Some(crate::piros::PIROS);
    spcs.num = 2;
    let at = Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs: Some(spcs) }));
    Some(Session::resume(iso, archive, None, start.state, start.vm, at).unwrap())
}

/// Event 22's blocks 0 and 1 as the board plays them: block 0 (its first
/// pass anywhere) zeroes `eventStatus[0]`, block 1 (game status 3, the
/// board) sets it to 1. This start skips the board, so the area's set-up
/// frames put the 1 back after block 0 has run.
fn board_done(s: &mut Session) {
    if let Stage::Area(a) = &mut s.stage
        && !matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 1)
    {
        a.world_mut().state_mut().save.set_u8(crate::piros::STATUS - 1, 1);
    }
}

/// The area's player: windows closed every 24 frames.
fn area_player(a: &crate::area::AreaMode, f: u64) -> Raw {
    let waiting = a.calls().iter().rev().find_map(|(_, c)| {
        if c.starts_with("message_open") || c.starts_with("announce") {
            Some(true)
        } else if c.starts_with("message_check") {
            Some(false)
        } else {
            None
        }
    });
    still(if waiting == Some(true) && f.is_multiple_of(24) { Buttons::CROSS } else { Buttons::NONE })
}

/// Event 22 in area 31's field (block 16) and its dungeon (block 19):
/// Piros present, `add_target`, `piros_colour 0` with status 2 - his tint
/// held (fix 1, rate 65, 0x002080ff) on his character in the fights; no
/// instruction fell to a host default.
#[test]
fn event_22_tints_piros_in_area_31() {
    for dungeon in [false, true] {
        let Some(mut s) = story_22_in_area_31(dungeon) else { return };
        piney_event::host::take_unported();
        let mut pad = Pad::default();
        let mut seen = None;
        for f in 0..3000u64 {
            let raw = match &s.stage {
                Stage::Area(a) => area_player(a, f),
                _ => still(Buttons::NONE),
            };
            pad.read(&raw);
            s.step(&pad);
            s.take_events();
            board_done(&mut s);
            if let Stage::Area(a) = &s.stage
                && a.calls().iter().any(|(_, c)| c.starts_with("piros_colour"))
                && let Some(t) = a.world().affect_of(crate::piros::PIROS).filter(|t| t.fix == 1)
            {
                seen = Some(t);
                break;
            }
        }
        let calls = match &s.stage {
            Stage::Area(a) => a.calls().iter().map(|(f, c)| format!("{f} {c}")).collect::<Vec<_>>().join("\n"),
            _ => String::new(),
        };
        let t = seen.unwrap_or_else(|| panic!("no tint (dungeon {dungeon}): {}\n{calls}", Mode::title(&s)));
        assert_eq!((t.fix, t.rate, t.colour), (1, 65, 0x0020_80ff), "dungeon {dungeon}");
        assert_eq!(piney_event::host::take_unported(), Vec::<&str>::new(), "dungeon {dungeon}");
    }
}

/// A picture of Piros tinted orange in area 31's dungeon. `PINEY_SHOTS=DIR
/// cargo test --release -p piney-game event_22_dungeon_shot -- --ignored
/// --nocapture` (default `/mnt/data/claude/scratch/event22`).
#[test]
#[ignore]
fn event_22_dungeon_shot() {
    const TURN: u64 = 32;
    let Some(mut s) = story_22_in_area_31(true) else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/event22".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    let mut tinted = None;
    for f in 0..3000u64 {
        // After the tint, the camera turned round (L1 held) to the party
        // behind Kite.
        let raw = match &s.stage {
            Stage::Area(_) if tinted.is_some_and(|t| f > t + 60 && f < t + 60 + TURN) => still(Buttons::L1),
            Stage::Area(a) => area_player(a, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        board_done(&mut s);
        let Stage::Area(a) = &s.stage else { continue };
        if tinted.is_none() && a.world().affect_of(crate::piros::PIROS).is_some_and(|t| t.fix == 1) {
            tinted = Some(f);
        }
        if tinted.is_some_and(|t| f == t + 90 + TURN) {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/3-dungeon-orange.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            return;
        }
    }
    panic!("no tint: {}", Mode::title(&s));
}

/// Back from area 31 to Mac Anu (Gate Out from the field): the town takes
/// the party the field hands back (`ccSpcManager` and `ccPartyManager` are
/// the game's globals), and its `rebootSpcManager` builds Piros beside
/// Kite as `ccGetStartPositions` places a party member (the start + (200,
/// 100), facing as Kite does), arriving (act 13).
#[test]
fn the_town_takes_the_party_back() {
    use piney_world::entry::Kind;
    let Some(mut s) = story_22_in_area_31(false) else { return };
    let mut pad = Pad::default();
    let mut out = false;
    let mut placed = None;
    for f in 0..4000u64 {
        let raw = match &s.stage {
            Stage::Area(a) => area_player(a, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        board_done(&mut s);
        match &mut s.stage {
            Stage::Area(a) if !out && matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 30) => {
                // Event 22's town blocks (eventStatus[0] 1) are for a party
                // without Piros; they are kept out of Mac Anu.
                a.world_mut().state_mut().save.set_u8(crate::piros::STATUS - 1, 2);
                a.gate_out();
                out = true;
            }
            Stage::World(w) if w.world().char_place(Kind::Spc, 8).is_some() => {
                let p = w.world().char_place(Kind::Spc, 8).map(|(p, _)| p.map(f32::from_bits));
                placed = Some((w.world().party(), p));
                break;
            }
            _ => {}
        }
    }
    let calls = match &s.stage {
        Stage::World(w) => w.calls().iter().map(|(f, c)| format!("{f} {c}")).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    };
    let (party, pos) = placed.unwrap_or_else(|| panic!("not back in Mac Anu with Piros: {}\n{calls}", Mode::title(&s)));
    assert_eq!(party, [0, crate::piros::PIROS, -1]);
    assert_eq!(pos, Some([200.0, 5700.0, 600.0, 1.0]), "Piros's StartPos");
}

/// Mac Anu with Piros in the party (back from area 31, as
/// [`the_town_takes_the_party_back`]): the session and the frame count.
fn mac_anu_with_piros() -> Option<(Session, u64)> {
    use piney_world::entry::Kind;
    let mut s = story_22_in_area_31(false)?;
    let mut pad = Pad::default();
    let mut out = false;
    for f in 0..4000u64 {
        let raw = match &s.stage {
            Stage::Area(a) => area_player(a, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        board_done(&mut s);
        match &mut s.stage {
            Stage::Area(a) if !out && matches!(a.world().phase(), piney_world::Phase::Play(n) if n >= 30) => {
                a.world_mut().state_mut().save.set_u8(crate::piros::STATUS - 1, 2);
                a.gate_out();
                out = true;
            }
            Stage::World(w) if w.world().char_place(Kind::Spc, 8).is_some() => return Some((s, f)),
            _ => {}
        }
    }
    None
}

/// Back in Mac Anu, Piros says his arrival line: `arrivalChatCnt` (150 in
/// a town) runs down in `ChatMessageSender` and at 0, out of a fight,
/// `ChatMessageEnteredTown` picks his row of `arriveDeltaMessages`
/// (server 0), which opens in his balloon (`handle` kind 1, id 8).
#[test]
fn piros_says_his_arrival_line_in_mac_anu() {
    let Some((mut s, f0)) = mac_anu_with_piros() else { return };
    let mut pad = Pad::default();
    let mut said = None;
    for f in f0..f0 + 400 {
        pad.read(&still(Buttons::NONE));
        s.step(&pad);
        s.take_events();
        let Stage::World(w) = &s.stage else { continue };
        let who = crate::world::handle(piney_world::entry::Kind::Spc, crate::piros::PIROS);
        if let Some(slot) = w.ui().ctrl.chat.slots.iter().find(|x| x.cf > 0 && x.who == who) {
            said = Some((f - f0, String::from_utf8_lossy(&slot.text).into_owned()));
            break;
        }
    }
    let (f, text) = said.expect("Piros said nothing in Mac Anu");
    println!("frame {f}: {text}");
    assert!(!text.is_empty() && f > 100, "{f}: {text}");
}

/// The stick pushed toward world heading `h` (0 faces -y) under the
/// camera's heading `cam_z`.
fn stick(cam_z: f32, h: f32) -> Raw {
    use std::f32::consts::PI;
    let want = (cam_z - PI - h).rem_euclid(2.0 * PI);
    let mut best = (f32::MAX, still(Buttons::NONE));
    for k in 0..64 {
        let a = k as f32 * PI / 32.0;
        let (lx, ly) = ((128.0 + 127.0 * a.cos()) as u8, (128.0 + 127.0 * a.sin()) as u8);
        let raw = Raw { lx, ly, ..still(Buttons::NONE) };
        let mut p = Pad::default();
        p.read(&raw);
        let d = (p.dirc_l.rem_euclid(2.0 * PI) - want).abs();
        let d = d.min(2.0 * PI - d);
        if d < best.0 {
            best = (d, raw);
        }
    }
    best.1
}

/// Piros's walk through Mac Anu after Kite arrives, `frames` frames: when
/// `follow` is set, the CHAT menu's "follow" (`RequestChatCmd(8, 9)`) is
/// given 100 frames in; Kite runs south (heading 0, -y: down the gate's
/// plaza) from frame 150 to 450, then stands. Piros's greatest distance from where he started, his distance
/// from Kite at the end, his `actType`s in order, and how far Kite went.
fn piros_in_mac_anu(follow: bool, frames: u64) -> Option<(f32, f32, Vec<i16>, f32)> {
    use piney_world::entry::Kind;
    let (mut s, f0) = mac_anu_with_piros()?;
    let mut pad = Pad::default();
    let (mut start, mut kite_start) = (None, None);
    let (mut moved, mut end, mut kite_moved) = (0f32, 0f32, 0f32);
    let mut acts = Vec::new();
    for f in f0..f0 + frames {
        if follow && f == f0 + 100 {
            let Stage::World(w) = &mut s.stage else { return None };
            w.world_mut().chat_cmd(crate::piros::PIROS, 9, None, 0);
        }
        let raw = match &s.stage {
            Stage::World(w) if f >= f0 + 150 && f < f0 + 450 => stick(f32::from_bits(w.world().camera().rot()[2]), 0.0),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::World(w) = &s.stage else { panic!("left the town: {}", Mode::title(&s)) };
        let world = w.world();
        let (Some((p, _)), Some((k, _))) = (world.char_place(Kind::Spc, 8), world.char_place(Kind::Spc, 0)) else {
            continue;
        };
        let (p, k) = (p.map(f32::from_bits), k.map(f32::from_bits));
        let s0 = *start.get_or_insert(p);
        moved = moved.max(((p[0] - s0[0]).powi(2) + (p[1] - s0[1]).powi(2)).sqrt());
        let k0 = *kite_start.get_or_insert(k);
        kite_moved = kite_moved.max(((k[0] - k0[0]).powi(2) + (k[1] - k0[1]).powi(2)).sqrt());
        end = ((p[0] - k[0]).powi(2) + (p[1] - k[1]).powi(2)).sqrt();
        let party = world.town_party();
        if let Some(a) = party.member(8).and_then(|w| party.combat.crew.ais.get(&w)).map(|a| a.act_type)
            && acts.last() != Some(&a)
        {
            acts.push(a);
        }
    }
    Some((moved, end, acts, kite_moved))
}

/// Back in Mac Anu with no order, Piros is not led (`ActInTown`: `actType`
/// 96 waits for Kite without an invitation, then rests, 99): he walks the
/// town on his own, to a shop (5-9) or a landmark (10) at random, and rests
/// there (11).
#[test]
fn piros_walks_mac_anu_on_his_own() {
    let Some((moved, _, acts, _)) = piros_in_mac_anu(false, 900) else { return };
    let walk = acts.get(2).is_some_and(|a| (5..=10).contains(a));
    assert!(acts.starts_with(&[96, 99]) && walk, "act types {acts:?}");
    assert!(moved > 1000.0, "Piros moved {moved:.0}; act types {acts:?}");
}

/// Told to follow (the CHAT menu's order 9: `actType` 3, and within 200 of
/// Kite 94 at once), Piros follows him (94, and 97 within 150) while he
/// runs south and after he stops: he ends near him.
#[test]
fn piros_follows_kite_in_mac_anu() {
    let Some((moved, end, acts, kite)) = piros_in_mac_anu(true, 900) else { return };
    assert!(acts.contains(&94), "act types {acts:?}");
    assert!(kite > 500.0 && moved > kite - 400.0, "Piros moved {moved:.0}, Kite {kite:.0}; act types {acts:?}");
    assert!(end < 400.0, "Piros {end:.0} from Kite at the end; act types {acts:?}");
}

/// Pictures of Piros following Kite through Mac Anu (the CHAT menu's
/// "follow"), every 100 frames. `PINEY_SHOTS=DIR cargo test --release -p
/// piney-game piros_follow_shots -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/event22`).
#[test]
#[ignore]
fn piros_follow_shots() {
    let Some((mut s, f0)) = mac_anu_with_piros() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/event22".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let mut pad = Pad::default();
    for f in f0..f0 + 700 {
        if f == f0 + 100
            && let Stage::World(w) = &mut s.stage
        {
            w.world_mut().chat_cmd(crate::piros::PIROS, 9, None, 0);
        }
        let raw = match &s.stage {
            Stage::World(w) if f >= f0 + 150 && f < f0 + 450 => stick(f32::from_bits(w.world().camera().rot()[2]), 0.0),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        if (f - f0) % 100 == 50 {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/follow-{:03}.png", f - f0);
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
        }
    }
}

/// Event 16, the merchants' contest in Mac Anu: once a merchant has been
/// spoken to (blocks 6-10 `set_block 5`), block 11 gives the prize through
/// the item-get menu (`item_add_menu 0 15 60 1`, menu 29: the Book of Law),
/// then mail 12, which event 17 waits for. Here blocks 0-5 (the desktop's
/// mail, Mac Anu's set-up) are marked played at the start and the windows
/// are closed as they come.
#[test]
fn event_16_gives_the_contest_prize() {
    let Some(mut s) = story_in_mac_anu(16, 0x3f) else { return };
    // Category 15 of Kite's: the important items' counts (impItemList).
    let book = |s: &Session| match &s.stage {
        Stage::World(w) => w.world().state().save.u8(piney_data::save::offset::IMP_ITEM_LIST + 60),
        _ => 0,
    };
    let before = book(&s);
    piney_event::host::take_unported();
    let mut pad = Pad::default();
    let mut menu = false;
    for f in 0..6000u64 {
        let raw = match &s.stage {
            Stage::World(w) => {
                menu |= w.ui().menu_type() == 29;
                // The item-get menu: X on its OK.
                if w.ui().menu_type() == 29 && f.is_multiple_of(12) { still(Buttons::CROSS) } else { player(w, f) }
            }
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage
            && w.calls().iter().any(|(_, c)| c.starts_with("mail"))
        {
            break;
        }
    }
    let after = book(&s);
    let unported = piney_event::host::take_unported();
    eprintln!("Book of Law {before} -> {after}; menu 29 {menu}; unported {unported:?}");
    assert!(menu, "the item-get menu opened");
    assert_eq!(after, before + 1, "the Book of Law given");
    assert!(!unported.iter().any(|u| u.contains("item_get_menu")), "unported: {unported:?}");
}

/// The Chaos Gate's circle in Mac Anu: the gate menu's command 11 opens it
/// and `gateAnm` sounds 71 thirty frames on; command 0 closes it with 72;
/// both play at the gate (`ccSeOn3D`).
#[test]
fn the_gates_circle_sounds() {
    let Some(mut s) = story_22_in_mac_anu() else { return };
    let mut pad = Pad::default();
    let mut heard: Vec<(u64, usize)> = Vec::new();
    let mut opened = None;
    for f in 0..3000u64 {
        let raw = match &s.stage {
            Stage::World(w) if w.world().phase() == piney_world::Phase::Play(0) || opened.is_some() => {
                still(Buttons::NONE)
            }
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        if opened.is_none()
            && let Stage::World(w) = &mut s.stage
            && matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 60)
        {
            w.world_mut().affect(piney_world::entry::Kind::Gimmick, i32::from(piney_world::gate::GIMMICK), 11);
            opened = Some(f);
        }
        if let Some(o) = opened
            && f == o + 100
            && let Stage::World(w) = &mut s.stage
        {
            w.world_mut().affect(piney_world::entry::Kind::Gimmick, i32::from(piney_world::gate::GIMMICK), 0);
        }
        pad.read(&raw);
        s.step(&pad);
        for e in s.take_events() {
            if let Event::Se3d { n: n @ (71 | 72), .. } = e {
                heard.push((f - opened.unwrap_or(f), n));
            }
        }
        if opened.is_some_and(|o| f > o + 130) {
            break;
        }
    }
    eprintln!("gate sounds {heard:?}");
    assert!(opened.is_some(), "never played in Mac Anu: {}", Mode::title(&s));
    assert_eq!(heard.iter().map(|h| h.1).collect::<Vec<_>>(), [71, 72], "{heard:?}");
}

/// Kite's footsteps in Mac Anu: his run's animation notes (1 and 2) go
/// through `ccPlayer::CheckNote` to `ccSeSetParamSPC`, a step's sound at
/// his feet by the ground's attribute, as in the fields.
#[test]
fn kite_has_footsteps_in_mac_anu() {
    // Event 16 with every block played: nothing holds Kite.
    let Some(mut s) = story_in_mac_anu(16, 0xfff) else { return };
    let mut pad = Pad::default();
    // Settled in the town, then the stick held up for three seconds.
    for f in 0..200u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    let mut steps = 0;
    for _ in 0..90 {
        pad.read(&Raw { ly: 0, ..still(Buttons::NONE) });
        s.step(&pad);
        steps += s.take_events().iter().filter(|e| matches!(e, Event::Se3d { .. })).count();
    }
    eprintln!("footsteps in 90 frames of running: {steps}");
    assert!(steps >= 4, "Kite ran silently: {steps} sounds, {}", Mode::title(&s));
}

/// The walking PCs' steps in Mac Anu: `rtpcCheckNote`'s
/// `ccSeSetParamPC` on each step note of a PC in view, Kite standing still.
#[test]
fn the_walking_pcs_have_footsteps() {
    let Some(mut s) = story_in_mac_anu(16, 0xfff) else { return };
    let mut pad = Pad::default();
    let mut steps = 0;
    for f in 0..500u64 {
        let raw = match &s.stage {
            Stage::World(w) => player(w, f),
            _ => still(Buttons::NONE),
        };
        pad.read(&raw);
        s.step(&pad);
        if f >= 200 {
            steps += s.take_events().iter().filter(|e| matches!(e, Event::Se3d { .. })).count();
        } else {
            s.take_events();
        }
    }
    eprintln!("the walkers' steps in 300 frames: {steps}");
    assert!(steps > 0, "the walking PCs walked silently: {}", Mode::title(&s));
}

/// Piros's menu (`SpcMenu`, 21) in Mac Anu, shot headless when it is up,
/// and where Piros stands while it is (the talk's `EntryAffect(14)` stands
/// him). `PINEY_SHOTS=DIR cargo test --release -p piney-game
/// piros_menu_shot -- --ignored --nocapture` (default
/// `/mnt/data/claude/scratch/spcmenu`).
#[test]
#[ignore]
fn piros_menu_shot() {
    use piney_world::entry::Kind;
    let Some((mut s, _)) = mac_anu_with_piros() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/spcmenu".into());
    std::fs::create_dir_all(&dir).unwrap();
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
    let mut pad = Pad::default();
    let mut step = |s: &mut Session, raw: Raw| {
        pad.read(&raw);
        let f = s.step(&pad);
        s.take_events();
        f
    };
    // Piros as the command target: Kite walks toward him until he is.
    let mut opened = None;
    for f in 0..900u64 {
        let raw = {
            let Stage::World(w) = &s.stage else { panic!("left the town") };
            let world = w.world();
            if w.ui().ctrl.menu == 21 && w.ui().ctrl.menu_status == 2 {
                opened = Some(f);
                break;
            }
            if world.command_target() == Some((Kind::Spc, crate::piros::PIROS)) {
                still(if f % 20 == 0 { Buttons::CROSS } else { Buttons::NONE })
            } else if let (Some((at, _)), kite) =
                (world.char_place(Kind::Spc, crate::piros::PIROS), world.player().body.pos)
            {
                let (dx, dy) =
                    (f32::from_bits(at[0]) - f32::from_bits(kite[0]), f32::from_bits(at[1]) - f32::from_bits(kite[1]));
                let cam = f32::from_bits(world.camera().active().rot[2]);
                stick(cam, dx.atan2(-dy))
            } else {
                still(Buttons::NONE)
            }
        };
        step(&mut s, raw);
    }
    let f = opened.expect("Piros's menu did not open");
    println!("menu 21 up at frame {f}");
    let at = |s: &Session| match &s.stage {
        Stage::World(w) => w.world().char_place(Kind::Spc, crate::piros::PIROS).map(|p| p.0),
        _ => None,
    };
    let before = at(&s);
    let mut frame = step(&mut s, still(Buttons::NONE));
    for _ in 0..60 {
        frame = step(&mut s, still(Buttons::NONE));
    }
    println!("Piros at {before:?}, then {:?}", at(&s));
    gs.set_overlay(Mode::archive(&s));
    gs.render(&frame);
    let (w, h) = gs.target_size();
    let path = format!("{dir}/piros-menu.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
    println!("{path}: {}", Mode::title(&s));
}
