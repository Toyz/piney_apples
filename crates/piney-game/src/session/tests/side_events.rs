//! A survey of Infection's side events (S1, 50-62) where they play: each
//! event's story start (the main event its opening needs), its earlier
//! blocks marked as run and the side events it needs done, the session put
//! in the field (or the dungeon room of the event point) its first placed
//! block names, then driven by the story autopilot (`story_player`). What
//! each reached: the blocks run, the entries and windows its host was
//! asked for, the host calls still at their defaults, any panic. A
//! diagnostic, not a check: `cargo test --release -p piney-game
//! side_event_survey -- --ignored --nocapture`.

use piney_event::state::{CLOSED, DONE, ScriptSave as _};

use super::*;

/// A side event: its number, the story start it plays from, and the side
/// events it needs done.
struct Case {
    event: i32,
    story: i32,
    done: &'static [i32],
}

const CASES: &[Case] = &[
    Case { event: 50, story: 12, done: &[] },
    Case { event: 51, story: 18, done: &[50] },
    Case { event: 52, story: 22, done: &[50, 51] },
    Case { event: 53, story: 25, done: &[50, 51, 52] },
    Case { event: 55, story: 13, done: &[] },
    Case { event: 58, story: 22, done: &[] },
    Case { event: 59, story: 22, done: &[] },
    Case { event: 60, story: 25, done: &[59] },
    Case { event: 61, story: 18, done: &[] },
];

/// Where a block's tags put it: (area, town, field, dungeon, event point).
fn place_of(tags: &[String]) -> Option<(i32, i32, i32, i32, i32)> {
    let num = |t: &str, k: &str| {
        t.split_whitespace().find_map(|w| w.strip_prefix(&format!("{k}="))).and_then(|v| v.parse::<i32>().ok())
    };
    let mut place = None;
    let mut point = -1;
    for t in tags {
        if t.contains("in_field") {
            place = Some((1, num(t, "town")?, num(t, "field")?, -1));
        } else if t.contains("in_dungeon") {
            place = Some((2, num(t, "town")?, num(t, "field")?, num(t, "dungeon")?));
        } else if t.contains("in_point") {
            point = num(t, "num")?;
        }
    }
    place.map(|(a, t, f, d)| (a, t, f, d, point))
}

#[test]
#[ignore]
fn side_event_survey() {
    let frames: u64 = std::env::var("PINEY_SURVEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        return;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    for case in CASES {
        let n = case.event;
        piney_event::host::take_unported();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut start = crate::start::build(&iso, case.story).ok()?;
            let script = start.vm.library().script(n)?.clone();
            // The first block with a place, and the blocks before it run.
            let mut first = None;
            for (b, block) in script.blocks.iter().enumerate() {
                let tags: Vec<String> = block.tags.iter().map(piney_event::text::print_tag).collect();
                if let Some(p) = place_of(&tags) {
                    first = Some((b, p));
                    break;
                }
            }
            let (b0, (area, town, field, dungeon, point)) = first?;
            let bits = (1u64 << b0) - 1;
            start.state.save.update_flags(n, |f| f | bits);
            for &d in case.done {
                start.state.save.update_flags(d, |f| f | DONE);
            }
            let mut scene = piney_world::area::Scene::log_in(&mut start.state.save);
            let (floor, blk) = if area == 2 { (0, 0) } else { (-1, -1) };
            scene.change_scene(area, town, field, dungeon, floor, blk, &mut start.state.save);
            let wm = crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), field, false).ok();
            let at = Resume::World(Box::new(InWorld { scene, world_man: wm, spcs: None }));
            let mut s = Session::resume(iso.clone(), archive.clone(), None, start.state, start.vm, at).ok()?;
            let mut pad = Pad::default();
            let mut moved = point < 0;
            let mut calls: Vec<String> = Vec::new();
            for f in 0..frames {
                let raw = story_player(&s, f);
                pad.read(&raw);
                s.step(&pad);
                s.take_events();
                if let Stage::Area(a) = &s.stage {
                    for (_, c) in a.calls() {
                        if (c.starts_with("entry") || c.starts_with("message_open") || c.starts_with("end_event"))
                            && !calls.contains(c)
                        {
                            calls.push(c.clone());
                        }
                    }
                    // In the dungeon: to the event point's room once its
                    // points are known.
                    if !moved
                        && matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 10)
                        && let Some(vm) = a.vm()
                        && let Some(p) = vm.mng.points.iter().find(|p| p.num == point)
                    {
                        moved = true;
                        let (fl, bl) = (i32::from(p.floor), i32::from(p.block));
                        s.go(Pending::Go(piney_data::area::Go::ChangeScene([2, town, field, dungeon, fl, bl])));
                    }
                }
            }
            // The party-type entries (`entry 2 code`): built in the scene?
            let spc_codes: Vec<i16> = script
                .blocks
                .iter()
                .flat_map(|b| b.ops.iter())
                .filter_map(|o| {
                    let t = piney_event::text::print_op(o);
                    let f = |k: &str| {
                        t.split_whitespace()
                            .find_map(|w| w.strip_prefix(&format!("{k}=")))
                            .and_then(|v| v.parse::<i16>().ok())
                    };
                    (t.starts_with("entry ") && f("type") == Some(2)).then(|| f("code")).flatten()
                })
                .collect();
            for code in spc_codes {
                let built = match &s.stage {
                    Stage::Area(a) => a.world().combat().who(code as i32).is_some(),
                    _ => false,
                };
                calls.push(format!("spc {code} built {built}"));
            }
            let flags = match &s.stage {
                Stage::Area(a) => a.world().state().save.flags(n),
                Stage::World(w) => w.world().state().save.flags(n),
                _ => 0,
            };
            Some((b0, flags, calls, Mode::title(&s), moved))
        }));
        let unported = piney_event::host::take_unported();
        match r {
            Ok(Some((b0, flags, calls, title, moved))) => {
                let run: Vec<usize> = (0..62).filter(|b| flags & (1 << b) != 0).collect();
                let done = flags & (DONE | CLOSED) != 0;
                println!("event {n}: placed from block {b0}; run {run:?}; ended {done}; room reached {moved}");
                println!("    {}", calls.join(" | "));
                println!("    last: {title}");
            }
            Ok(None) => println!("event {n}: no session"),
            Err(e) => {
                let msg =
                    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
                println!("event {n}: PANIC {}", msg.unwrap_or_default());
            }
        }
        if !unported.is_empty() {
            println!("    unported: {unported:?}");
        }
    }
}

/// Side event 61, Natsume, in area 35's dungeon: block 2's `entry 2 11 0 5`
/// registers her (`ccRegisterEventMng`), `Reboot` builds her although she
/// is not in the party, and `ccEntryEventMng` puts her at event position 0.
/// Kite put beside her, block 3's `near_marker 0 <= 65` holds and her scene
/// plays: the fade and her three lines.
#[test]
fn event_61_natsume_in_her_dungeon() {
    use piney_event::host::PcCommand;
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        return;
    }
    let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let n = 61;
    let mut start = crate::start::build(&iso, 18).unwrap();
    start.state.save.update_flags(n, |f| f | 0b11);
    let mut scene = piney_world::area::Scene::log_in(&mut start.state.save);
    scene.change_scene(2, 0, 35, 0, 0, 0, &mut start.state.save);
    let wm = crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), 35, false).ok();
    let at = Resume::World(Box::new(InWorld { scene, world_man: wm, spcs: None }));
    let mut s = Session::resume(iso.clone(), archive, None, start.state, start.vm, at).unwrap();
    let mut pad = Pad::default();
    let (mut moved, mut put) = (false, false);
    let mut lines = Vec::new();
    for f in 0..2400u64 {
        let raw = story_player(&s, f);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let playing = matches!(a.world().phase(), piney_world::Phase::Play(k) if k > 10);
        if !moved && playing {
            let Some(p) = a.vm().and_then(|vm| vm.mng.points.iter().find(|p| p.num == 1).copied()) else { continue };
            moved = true;
            s.go(Pending::Go(piney_data::area::Go::ChangeScene([2, 0, 35, 0, i32::from(p.floor), i32::from(p.block)])));
            continue;
        }
        if moved && !put && playing {
            let w = a.world_mut();
            let Some(natsume) = w.combat().who(11) else { continue };
            let at = w.combat().scene.chars[natsume].pos;
            let tenth = |v: u32| (f32::from_bits(v) / 10.0).round() as i16;
            let (x, y, z) = (tenth(at[0]) + 3, tenth(at[1]), tenth(at[2]));
            assert!(w.pc_command(PcCommand::Put { pc: 0, x, y, z }));
            put = true;
        }
        for (_, c) in a.calls() {
            if c.starts_with("message_open 61") && !lines.contains(c) {
                lines.push(c.clone());
            }
        }
        if lines.len() >= 3 {
            break;
        }
    }
    assert!(put, "Natsume was never built in the room of event point 1");
    assert!(lines.iter().any(|c| c.starts_with("message_open 61 0")), "{lines:?}");
    assert!(lines.len() >= 3, "{lines:?}");
}
