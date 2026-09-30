//! Surveys of the side events where they play: Infection's (S1, 50-62) and
//! Outbreak's (S3, 250-275, and the S4 table's 361, which opens on 201).
//! Each from its main event's start, the side events it needs brought
//! forward, its blocks before the first one set in a field or dungeon (else
//! a town) marked run, put there with the members it asks for, then driven
//! by the story autopilot following it alone. Diagnostics (`--ignored
//! --nocapture`, best with `PINEY_SURVEY_GOD`): `side_event_survey`,
//! `outbreak_side_event_survey`; the story survey's other variables too.

use std::path::Path;

use piney_event::ir::{Block, Cond, Tag};
use piney_event::state::{CLOSED, DONE, ScriptSave as _};

use super::survey::{LONE_LEVEL, StoryPilot, cores_for_hack, event_flag, levels_for_boss};
use super::*;

/// A side event: its number, the story start it plays from (the one after
/// the last main event it opens on), and the side events it needs done.
struct Case {
    event: i32,
    story: i32,
    done: &'static [i32],
}

const INF_CASES: &[Case] = &[
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

/// Outbreak's (plans/outbreak-side-events.md). Those below 200 are
/// Mutation's, which a carried save would hold done. The SEARCH chain
/// (267-275) opens on 201, but its NPC goes on to Fort Ouph, which 203
/// opens (`town_move 3`): it plays from 204.
const OUT_CASES: &[Case] = &[
    Case { event: 250, story: 205, done: &[154] },
    Case { event: 251, story: 205, done: &[154, 250] },
    Case { event: 252, story: 211, done: &[154, 250, 251] },
    Case { event: 253, story: 211, done: &[154, 250, 251, 252] },
    Case { event: 254, story: 219, done: &[154, 250, 251, 252, 253] },
    Case { event: 255, story: 205, done: &[155] },
    Case { event: 256, story: 205, done: &[156] },
    Case { event: 257, story: 211, done: &[157] },
    Case { event: 259, story: 205, done: &[159] },
    Case { event: 260, story: 205, done: &[160] },
    Case { event: 261, story: 211, done: &[162] },
    Case { event: 262, story: 205, done: &[] },
    Case { event: 263, story: 219, done: &[] },
    Case { event: 264, story: 202, done: &[] },
    Case { event: 265, story: 202, done: &[] },
    Case { event: 266, story: 205, done: &[262] },
    Case { event: 267, story: 204, done: &[] },
    Case { event: 268, story: 204, done: &[267] },
    Case { event: 269, story: 204, done: &[267, 268] },
    Case { event: 270, story: 204, done: &[267, 268, 269] },
    Case { event: 271, story: 204, done: &[267, 268, 269, 270] },
    Case { event: 272, story: 204, done: &[267, 268, 269, 270, 271] },
    Case { event: 273, story: 204, done: &[267, 268, 269, 270, 271, 272] },
    Case { event: 274, story: 204, done: &[267, 268, 269, 270, 271, 272, 273] },
    Case { event: 275, story: 204, done: &[267, 268, 269, 270, 271, 272, 273, 274] },
    Case { event: 361, story: 202, done: &[264] },
];

/// Key items (category 15) an earlier volume's side event gives, which a
/// carried save holds: (the event that checks, the item). Mutation's
/// SIGN-02 to SIGN-05 (165-168) give 287, 288, 289 and 290.
const CARRIED: &[(i32, i16)] = &[(267, 287), (269, 288), (271, 290), (273, 289)];

/// Where a block's settings put the player.
#[derive(Clone, Copy, Debug)]
enum Spot {
    Town(i16),
    Field {
        town: i16,
        field: i16,
    },
    /// The dungeon's first room, then event point `point`'s once known.
    Dungeon {
        town: i16,
        field: i16,
        dungeon: i16,
        point: Option<i16>,
    },
}

/// The place a block is set in, if it names one.
fn spot(block: &Block) -> Option<Spot> {
    let point = block
        .tags
        .iter()
        .find_map(|t| match *t {
            Tag::InPoint { num } => Some(num),
            _ => None,
        })
        .or_else(|| {
            block.conds.iter().find_map(|c| match *c {
                Cond::InPoint { num } => Some(num),
                _ => None,
            })
        });
    block.tags.iter().rev().find_map(|t| match *t {
        Tag::InTown { town } | Tag::Scene { area: 0, town, .. } if town >= 0 => Some(Spot::Town(town)),
        Tag::InField { town, field } | Tag::Scene { area: 1, town, field, .. } => Some(Spot::Field { town, field }),
        Tag::InDungeon { town, field, dungeon } | Tag::Scene { area: 2, town, field, dungeon, .. } => {
            Some(Spot::Dungeon { town, field, dungeon, point })
        }
        _ => None,
    })
}

/// Event `n` of `start` made ready to play: the side events in `done`
/// brought forward (the earlier volume's only marked done, its key items
/// given), the blocks before the first set in a field or dungeon (else a
/// town) marked run, and the start moved there with the members the event
/// asks for. That block and its place.
fn place(iso: &Path, start: &mut crate::start::Start, n: i32, done: &[i32]) -> Option<(usize, Spot)> {
    let lib = start.vm.library().clone();
    let base = 100 * (lib.volume.number() - 1);
    for &d in done {
        if d >= base {
            start.bring_forward(d);
        }
        start.state.save.update_flags(d, |f| f | DONE);
    }
    for &(_, item) in CARRIED.iter().filter(|c| c.0 == n) {
        start.state.save.add_item(0, 15, item, 1);
    }
    let blocks = &lib.script(n)?.blocks;
    let spots: Vec<(usize, Spot)> = blocks.iter().enumerate().filter_map(|(b, k)| spot(k).map(|s| (b, s))).collect();
    let (b0, at) = spots.iter().find(|(_, s)| !matches!(s, Spot::Town(_))).or(spots.first()).copied()?;
    // The leading blocks with no settings play wherever the event is open
    // (361's end once 264 is done): left to play.
    let first = blocks.iter().position(|k| !k.tags.is_empty()).unwrap_or(b0).min(b0);
    start.state.save.update_flags(n, |f| f | ((1u64 << b0) - (1u64 << first)));
    let save = &mut start.state.save;
    let (scene, world_man) = match at {
        Spot::Town(town) => {
            save.set_u8(piney_data::save::offset::LAST_TOWN, town as u8);
            (piney_world::area::Scene::log_in(save), None)
        }
        Spot::Field { town, field } | Spot::Dungeon { town, field, .. } => {
            let mut scene = piney_world::area::Scene::log_in(save);
            let (area, dungeon, floor) = match at {
                Spot::Dungeon { dungeon, .. } => (2, i32::from(dungeon), 0),
                _ => (1, -1, -1),
            };
            let (town, field) = (i32::from(town), i32::from(field));
            scene.change_scene(area, town, field, dungeon, floor, floor, save);
            let wm = crate::area::story_world_man(&mut Iso::open(iso).ok()?, field, false).ok();
            (scene, wm)
        }
    };
    // The members the event refuses to go on without (`not_in_party`),
    // in the party, as its town's gate asks.
    let mut members: Vec<i32> = blocks
        .iter()
        .flat_map(|k| &k.conds)
        .filter_map(|c| match *c {
            Cond::NotInParty { pc } if pc > 0 => Some(i32::from(pc)),
            _ => None,
        })
        .collect();
    members.sort_unstable();
    members.dedup();
    let spcs = (!members.is_empty()).then(|| crate::start::party_of(&members));
    start.at = Resume::World(Box::new(InWorld { scene, world_man, spcs }));
    Some((b0, at))
}

/// What a survey run saw: the block it was placed from, the event's flags,
/// the entries, its windows, the places, whether the event point's room was
/// reached, what the pilot still wanted, what played, and when it ended.
struct Seen {
    b0: usize,
    at: Spot,
    flags: u64,
    entries: Vec<String>,
    windows: usize,
    places: Vec<(u64, String)>,
    room: Option<bool>,
    wants: Vec<Want>,
    playing: Option<(i32, usize, usize)>,
    ended: Option<u64>,
    frames: u64,
}

/// A lone Kite in a field or dungeon raised to [`LONE_LEVEL`], as
/// `levels_for_boss` raises one the story wants alone: the side events send
/// him alone at the story's levels (50 or so) against goblins that heal and
/// rooms of 2,000-HP foes that fell him in a blow. A harness aid, as god is.
fn lone_level(s: &mut Session) {
    let Stage::Area(a) = &s.stage else { return };
    let c = a.world().combat();
    if c.members.len() != 1 {
        return;
    }
    let low = c.members.iter().filter_map(|&(_, k)| c.scene.chars[k].spc()).map(|p| p.base.level).min();
    if let Some(lv) = low.filter(|&lv| lv < LONE_LEVEL) {
        s.console(&format!("exp {}", (i32::from(LONE_LEVEL - lv) * 1000).min(30000)));
    }
}

/// One side event played for `frames` frames, with the survey's aids when
/// `god` (the party kept up, the infection at 0, the levels for a boss).
fn play(disc: &str, iso: &Path, case: &Case, frames: u64, god: bool) -> Option<Seen> {
    let n = case.event;
    let mut placed = None;
    let mut s = story_session_on(disc, case.story, |start| placed = place(iso, start, n, case.done))?;
    let (b0, at) = placed?;
    let _follow = Follow::side(n);
    if god {
        s.console("god");
    }
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let (mut entries, mut windows, mut places) = (Vec::new(), Vec::new(), Vec::<(u64, String)>::new());
    let point = match at {
        Spot::Dungeon { point, .. } => point,
        _ => None,
    };
    let mut room = point.map(|_| false);
    let (mut seen, mut last) = (0usize, None::<(u64, String)>);
    let (mut left, mut ended, mut ran) = (None, None, 0);
    // `PINEY_SURVEY_CALLS`: every host call as it is made.
    let trace = std::env::var_os("PINEY_SURVEY_CALLS").is_some();
    for f in 0..frames {
        ran = f + 1;
        if god && f.is_multiple_of(30) {
            s.console("infection 0");
            cores_for_hack(&mut s);
            levels_for_boss(&mut s);
            lone_level(&mut s);
        }
        // `PINEY_DEBUG_PILOT`: every 500 frames the place, the pilot's
        // wants, the event targets and the foes' HP.
        if std::env::var_os("PINEY_DEBUG_PILOT").is_some() && f.is_multiple_of(500) {
            let (vm, save, targets) = match &s.stage {
                Stage::World(w) => (w.vm(), Some(&w.world().state().save), w.world().event_targets().to_vec()),
                Stage::Area(a) => (a.vm(), Some(&a.world().state().save), a.world().event_targets().to_vec()),
                _ => (None, None, Vec::new()),
            };
            let wants = vm.zip(save).map(|(vm, save)| story_wants(vm, save));
            let foes: Vec<(i16, i16)> = match &s.stage {
                Stage::Area(a) => {
                    let c = a.world().combat();
                    c.enemies().into_iter().map(|e| (c.scene.chars[e].hp, c.scene.chars[e].max_hp)).collect()
                }
                _ => Vec::new(),
            };
            eprintln!("SIDE {f} {} wants {wants:?} targets {targets:?} foes {foes:?}", Mode::title(&s));
        }
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let calls = match &s.stage {
            Stage::Area(a) => a.calls(),
            Stage::World(w) => w.calls(),
            _ => &[],
        };
        // Only the calls new since the last frame (a new mode, or area,
        // starts its list over).
        if seen > 0 && calls.get(seen - 1) != last.as_ref() {
            seen = 0;
        }
        for (_, c) in &calls[seen..] {
            if trace {
                eprintln!("CALL {f} {c}");
            }
            let list = if c.starts_with("entry") {
                &mut entries
            } else if c.starts_with(&format!("message_open {n} ")) {
                &mut windows
            } else {
                continue;
            };
            if !list.contains(c) {
                list.push(c.clone());
            }
        }
        (seen, last) = (calls.len(), calls.last().cloned());
        let title = Mode::title(&s);
        let key: String = title
            .split(" - ")
            .filter(|p| !p.starts_with("frame") && !p.starts_with("play") && !p.starts_with("Kite at"))
            .collect::<Vec<_>>()
            .join(" - ");
        if places.last().map(|(_, k)| k) != Some(&key) {
            places.push((f, key));
        }
        // In the dungeon: to the event point's room once the set-up has
        // registered the points, as `room_point` goes (`RoomSelect`). Its
        // ChangeRequest idles the event task at once, not at the area's
        // next frame: the room's blocks wait for its passes.
        if room == Some(false)
            && let Stage::Area(a) = &mut s.stage
            && matches!(a.world().phase(), piney_world::Phase::Play(k) if k > 10)
            && let Some(p) =
                a.vm().and_then(|vm| vm.mng.points.iter().find(|p| Some(p.num) == point.map(i32::from)).copied())
        {
            if let Some(vm) = a.vm_mut() {
                vm.disable();
            }
            room = Some(a.world_mut().room_select(i32::from(p.floor), i32::from(p.block)));
        }
        // Ended: a little longer for its closing lines, then stop.
        if left.is_none() && event_flag(&mut s, n).is_some_and(|x| x & (DONE | CLOSED) != 0) {
            (left, ended) = (Some(f + 600), Some(f));
        }
        if left.is_some_and(|end| f >= end) {
            break;
        }
    }
    let (vm, save) = match &s.stage {
        Stage::Area(a) => (a.vm(), Some(&a.world().state().save)),
        Stage::World(w) => (w.vm(), Some(&w.world().state().save)),
        _ => (None, None),
    };
    let wants = vm.zip(save).map(|(vm, save)| story_wants(vm, save)).unwrap_or_default();
    let playing = vm.and_then(|vm| vm.playing_op());
    if trace && let Some(t) = vm.and_then(|vm| vm.trace.as_ref()) {
        eprintln!("TRACE {t:?}");
    }
    // The party-type entries (`entry 2 code`): built in the scene?
    if let (Stage::Area(a), Some(script)) = (&s.stage, vm.and_then(|vm| vm.library().script(n))) {
        for op in script.blocks.iter().flat_map(|b| &b.ops) {
            if let piney_event::ir::Op::Entry { ty: 2, code, .. } = *op {
                entries.push(format!("spc {code} built {}", a.world().combat().who(i32::from(code)).is_some()));
            }
        }
    }
    let flags = event_flag(&mut s, n).unwrap_or(0);
    Some(Seen { b0, at, flags, entries, windows: windows.len(), places, room, wants, playing, ended, frames: ran })
}

/// Each case on the disc `disc` for `PINEY_SURVEY_FRAMES` frames (`frames`
/// without); `PINEY_SURVEY_ONLY` the events listed (`250,251`).
fn side_survey(disc: &str, cases: &[Case], frames: u64) {
    let frames = std::env::var("PINEY_SURVEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(frames);
    let only: Vec<i32> = std::env::var("PINEY_SURVEY_ONLY")
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_default();
    let god = std::env::var_os("PINEY_SURVEY_GOD").is_some();
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
    if !iso.exists() {
        return;
    }
    for case in cases.iter().filter(|c| only.is_empty() || only.contains(&c.event)) {
        let n = case.event;
        piney_event::host::take_unported();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| play(disc, &iso, case, frames, god)));
        let unported = piney_event::host::take_unported();
        match r {
            Ok(Some(v)) => {
                let run: Vec<usize> = (0..62).filter(|b| v.flags & (1 << b) != 0).collect();
                let state = match v.ended {
                    Some(f) => format!("DONE at {f}"),
                    None => format!("open after {}", v.frames),
                };
                println!(
                    "event {n} (story {}): {state}; placed {:?} from block {}; run {run:?}",
                    case.story, v.at, v.b0
                );
                let wants = v.wants.iter().fold(Vec::new(), |mut all, w| {
                    if !all.contains(w) {
                        all.push(*w);
                    }
                    all
                });
                println!("    room reached {:?}; wants {wants:?}; playing {:?}", v.room, v.playing);
                println!("    entries {}; windows {}", v.entries.join(" | "), v.windows);
                // `PINEY_SURVEY_PATH`: every place, else the last three.
                let tail =
                    if std::env::var_os("PINEY_SURVEY_PATH").is_some() { 0 } else { v.places.len().saturating_sub(3) };
                println!(
                    "    {} places; last: {}",
                    v.places.len(),
                    v.places[tail..].iter().map(|(f, k)| format!("{f}: {k}")).collect::<Vec<_>>().join(" > ")
                );
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

#[test]
#[ignore]
fn side_event_survey() {
    side_survey("infection", INF_CASES, 3000);
}

/// Outbreak's, as [`side_event_survey`].
#[test]
#[ignore]
fn outbreak_side_event_survey() {
    side_survey("outbreak", OUT_CASES, 30_000);
}

/// Outbreak's disc, when present.
fn outbreak() -> Option<PathBuf> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/outbreak/outbreak.iso");
    iso.exists().then_some(iso)
}

/// Outbreak's case for side event `n`.
fn out_case(n: i32) -> &'static Case {
    OUT_CASES.iter().find(|c| c.event == n).expect("a surveyed side event")
}

/// The wants of a followed side event, from its placed start: GOB3-1's
/// field 78 waits for its foes all down (block 8's `no_active`), and
/// RACHEL-2's point 1 for Kite near marker 0 (block 6's `near_marker 0 <=
/// 65`). Not followed, the main story's wants have neither.
#[test]
fn a_followed_side_event_wants_its_foes_down_and_its_marker() {
    let Some(iso) = outbreak() else { return };
    let wants = |n: i32, follow: bool| {
        let case = out_case(n);
        let mut start = crate::start::build(&iso, case.story).unwrap();
        place(&iso, &mut start, n, case.done).unwrap();
        // The next mode's `ccStartThEvent`: the closed events done.
        for e in 0..450 {
            start.state.save.update_flags(e, |f| if f & CLOSED != 0 { f | DONE } else { f });
        }
        let _follow = follow.then(|| Follow::side(n));
        story_wants(&start.vm, &start.state.save)
    };
    let gob = wants(250, true);
    assert!(gob.contains(&Want::Clear(78)), "{gob:?}");
    assert!(!wants(250, false).contains(&Want::Clear(78)));
    let rachel = wants(257, true);
    assert!(rachel.contains(&Want::Marker(0)) && rachel.contains(&Want::Point(1)), "{rachel:?}");
    assert!(!wants(257, false).contains(&Want::Marker(0)));
}

/// SEARCH BT (268) from Carmina Gadelica: its NPC stands at marker 0, the
/// Chaos Gate's dummy (`DMY_gate`), where the gate is the command target
/// first. The pilot lets the stick go and pushes it again, which steps the
/// target on (`ccSelectTarget` mode 2), and speaks to the NPC in each of
/// the six towns in turn (blocks 2-12), to the event's end.
#[test]
fn search_bt_meets_its_npc_at_six_gates() {
    let Some(iso) = outbreak() else { return };
    let seen = play("outbreak", &iso, out_case(268), 6000, false).unwrap();
    let run: Vec<usize> = (0..13).filter(|b| seen.flags & (1 << b) != 0).collect();
    assert_eq!(run, [0, 2, 4, 6, 8, 10, 12]);
    assert!(seen.ended.is_some(), "open after {} frames", seen.frames);
}

/// GOB3-1 (250) in field 78: the golden goblin block 5 enters runs from
/// Kite; the pilot walks after it (`Want::Clear`) and fights it down, and
/// block 8's `no_active` ends the event.
#[test]
fn gob3_1_golden_goblin_is_run_down() {
    let Some(iso) = outbreak() else { return };
    let seen = play("outbreak", &iso, out_case(250), 14_000, true).unwrap();
    assert!(seen.ended.is_some(), "open after {} frames: {:?}", seen.frames, seen.places.last());
    assert!(seen.flags & (1 << 8) != 0);
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
