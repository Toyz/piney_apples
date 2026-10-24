//! A story boss's fight from its arena (Quarantine's Macha and Tarvos):
//! the session put in the arena as the event's `area` and `scene` leave
//! it, the story autopilot's play there, and the god run's shots and
//! music. docs/engine/boss-macha.md, docs/engine/boss-tarvos.md.

use std::collections::{BTreeMap, BTreeSet};

use piney_battle::boss::Class;

use super::survey::{StoryPilot, event_flag, fighters};
use super::*;

/// Where a story event fights its boss: the event, the blocks done before
/// its boss's `entry` (a mask), the dungeon its gate leads to, and the
/// `scene 1 town field` its `area field` opens.
pub(super) struct Arena {
    pub event: i32,
    pub done: u64,
    pub dungeon: i32,
    pub town: i32,
    pub field: i32,
    /// `eventStatus[k]` as the blocks done leave them (0 else).
    pub status: &'static [(usize, u8)],
}

/// Event 307: area 107's dungeon, field 6 (blocks 0-1 done; block 2 makes
/// Macha, block 4 waits for her to be gone).
pub(super) const EVENT_307: Arena = Arena { event: 307, done: 0b11, dungeon: 107, town: 3, field: 6, status: &[] };

/// Event 310: area 102's dungeon, field 7 (blocks 0-6 done; block 7 makes
/// Tarvos).
pub(super) const EVENT_310: Arena = Arena { event: 310, done: 0x7f, dungeon: 102, town: 4, field: 7, status: &[] };

/// Event 312: area 108's dungeon, field 12's disc (blocks 0-9 done; block
/// 10 makes Kyvia's fourth body).
pub(super) const EVENT_312: Arena = Arena { event: 312, done: 0x3ff, dungeon: 108, town: 0, field: 12, status: &[] };

/// Event 313: field 8's arena from town 4 (blocks 0-14 done,
/// `eventStatus[0]` 1; block 15 makes Corbenik's first form, which plays
/// the second in the same task).
pub(super) const EVENT_313: Arena =
    Arena { event: 313, done: 0x7fff, dungeon: 108, town: 4, field: 8, status: &[(0, 1)] };

/// Event 313 past the second form (blocks 0-18 done, `eventStatus[0]` 2
/// and `[55]` 1; block 19 makes the third form).
pub(super) const EVENT_313_THIRD: Arena =
    Arena { event: 313, done: 0x7ffff, dungeon: 108, town: 4, field: 8, status: &[(0, 2), (55, 1)] };

/// The event in its arena with Kite and `members` as a player calls them
/// before the gate.
pub(super) fn arena_session(a: &Arena, members: &[i32]) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/quarantine/quarantine.iso");
    if !iso.exists() {
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, a.event).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(a.event as usize);
    state.save.set_event_flag(a.event as usize, f | a.done);
    state.save.set_u8(offset::EVENT_STATUS, 0);
    for &(k, v) in a.status {
        state.save.set_u8(offset::EVENT_STATUS + k, v);
    }
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, a.dungeon, false).unwrap();
    let wm = crate::area::ev_area_number(&iso, a.field, &wm, &state.save).unwrap();
    scene.change_scene(1, a.town, a.field, -1, -1, -1, &mut state.save);
    let spcs = Some(crate::start::party_of(members));
    let at = Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs }));
    Some(Session::resume(iso, archive, None, state, start.vm, at).unwrap())
}

/// What the fight showed: the boss's acts, the party's levels as it met
/// the boss, what each of its acts took off the party, and how the run
/// ended.
#[derive(Default)]
pub(super) struct Played {
    pub acts: BTreeSet<i16>,
    pub levels: Vec<i16>,
    pub hurt: BTreeMap<i16, i32>,
    pub frames: u32,
    pub ended_at: Option<u32>,
    pub over_at: Option<u32>,
}

/// `frames` of the pilot's play until `event` ends or the game is over;
/// `boss` picks the fight's boss by its class; `each` takes every frame's
/// events. `PINEY_TRACE_BOSS` prints the fight a line a second,
/// `PINEY_TRACE_HITS` each hit on the party, `PINEY_TRACE_KITE` Kite and
/// the pilot a frame.
pub(super) fn play(
    s: &mut Session,
    event: i32,
    boss: fn(&Class) -> bool,
    frames: u32,
    mut each: impl FnMut(&mut Session, &Frame, u32),
) -> Played {
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let mut p = Played::default();
    let mut last_hp: Vec<i16> = Vec::new();
    for f in 0..frames {
        let raw = pilot.next(s, u64::from(f));
        pad.read(&raw);
        let frame = s.step(&pad);
        each(s, &frame, f);
        p.frames = f + 1;
        if matches!(s.stage, Stage::Title(_)) {
            p.over_at = Some(f);
            break;
        }
        if event_flag(s, event).is_some_and(|x| x & 3 << 62 != 0) {
            p.ended_at = Some(f);
            break;
        }
        let Stage::Area(a) = &s.stage else { continue };
        let c = a.world().combat();
        let Some(me) = c.boss_char() else { continue };
        let ch = &c.scene.chars[me];
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| boss(&b.class)) else {
            continue;
        };
        p.acts.insert(b.act_num);
        if p.levels.is_empty() {
            p.levels = c.members.iter().filter_map(|&(_, k)| c.scene.chars[k].spc()).map(|q| q.base.level).collect();
        }
        let hp: Vec<i16> = c.members.iter().map(|&(_, k)| c.scene.chars[k].hp).collect();
        if hp.len() == last_hp.len() {
            for (k, (now, was)) in hp.iter().zip(&last_hp).enumerate() {
                if now < was {
                    *p.hurt.entry(b.act_num).or_default() += i32::from(was - now);
                    if std::env::var_os("PINEY_TRACE_HITS").is_some() {
                        let m = &c.scene.chars[c.members[k].1];
                        eprintln!("HIT {f} act {} member {k} -{} -> {now} cond {:?}", b.act_num, was - now, m.cond.v);
                    }
                }
            }
        }
        last_hp = hp;
        if std::env::var_os("PINEY_TRACE_KITE").is_some() {
            let w = a.world();
            let (k, q) = (w.player().body.pos.map(f32::from_bits), ch.pos.map(f32::from_bits));
            let kite = c.members.first().map(|&(_, m)| c.scene.chars[m].spc_char.act_num);
            let d = (k[0] - q[0]).hypot(k[1] - q[1]) as i32;
            eprintln!(
                "KITE {f} boss {} d {d} kite {kite:?} target {:?} | {}",
                b.act_num,
                w.command_target_code(),
                pilot.doing()
            );
        }
        if std::env::var_os("PINEY_TRACE_BOSS").is_some() && f.is_multiple_of(60) {
            let pp = c.scene.param(me).map_or(0, |q| q.pp);
            eprintln!("BOSS {f} act {} pp {pp} | {} | {}", b.act_num, pilot.doing(), fighters(a));
        }
    }
    p
}

/// The fight under god from the arena to the event's end, every frame's
/// events through [`crate::handle`] and every frame rendered from the
/// start: the frames the music played, and a shot every ten seconds and at each act's first frame in
/// `dir` (`name-FRAME-actN.png`); with `PINEY_SHOTS_AT=A-B` every frame
/// from A to B too, its command target printed.
pub(super) fn god_shots(s: &mut Session, event: i32, boss: fn(&Class) -> bool, dir: &str, name: &str) -> (Played, u32) {
    god_shots_for(s, event, boss, dir, name, 60_000)
}

/// [`god_shots`] over at most `frames` frames.
pub(super) fn god_shots_for(
    s: &mut Session,
    event: i32,
    boss: fn(&Class) -> bool,
    dir: &str,
    name: &str,
    frames: u32,
) -> (Played, u32) {
    let mut shots = Shots::new(dir);
    s.console("god");
    let audio = piney_audio::Audio::headless(&s.iso).ok();
    let mut seen: BTreeSet<i16> = BTreeSet::new();
    let mut music = 0u32;
    // PINEY_SHOTS_AT=A-B: a shot every frame from A to B as well.
    let range = std::env::var("PINEY_SHOTS_AT").ok().and_then(|v| {
        let (a, b) = v.split_once('-')?;
        Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))
    });
    let p = play(s, event, boss, frames, |s, frame, n| {
        let rate = Mode::frame_rate(s);
        crate::handle(s.take_events(), audio.as_ref());
        let playing = audio.as_ref().is_some_and(|a| {
            a.frame();
            a.render(&mut vec![0i16; 1600 * rate as usize]);
            a.with_engine(|e| e.seq.iter().any(|q| q.playing()))
        });
        music += u32::from(playing);
        let act = match &s.stage {
            Stage::Area(a) => {
                let c = a.world().combat();
                c.boss_char().and_then(|me| c.scene.chars[me].foe_state()?.boss.as_ref().map(|b| b.act_num))
            }
            _ => None,
        };
        let first = act.is_some_and(|x| seen.insert(x));
        let span = range.is_some_and(|(a, b)| (a..=b).contains(&n));
        if span && let Stage::Area(a) = &s.stage {
            eprintln!("FRAME {n} act {act:?} target {:?}", a.world().command_target_code());
        }
        let shot = first || span || n.is_multiple_of(600);
        let path = shot.then(|| format!("{dir}/{name}-{n:05}-act{}.png", act.unwrap_or(-1)));
        shots.frame(s, frame, path);
    });
    (p, music)
}
