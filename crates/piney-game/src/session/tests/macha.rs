//! Macha (`ccBoss06`, Quarantine's event 307 in field 6's arena): the
//! fight played from the arena and from the event's own start under the
//! story autopilot with no aid, and its pictures and music under god.
//! docs/engine/boss-macha.md.

use std::collections::{BTreeMap, BTreeSet};

use piney_battle::boss::Class;

use super::survey::{StoryPilot, event_flag, fighters};
use super::*;

/// Event 307 in field 6's arena, from area 107's dungeon (block 2 makes
/// Macha; block 4 waits for it to be gone), with Kite and `members` as a
/// player calls them before the gate.
fn event_307_arena(members: &[i32]) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/quarantine/quarantine.iso");
    if !iso.exists() {
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 307).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(307);
    state.save.set_event_flag(307, f | 0b11);
    state.save.set_u8(offset::EVENT_STATUS, 0);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 107, false).unwrap();
    let wm = crate::area::ev_area_number(&iso, 6, &wm, &state.save).unwrap();
    scene.change_scene(1, 3, 6, -1, -1, -1, &mut state.save);
    let spcs = Some(crate::start::party_of(members));
    let at = Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs }));
    Some(Session::resume(iso, archive, None, state, start.vm, at).unwrap())
}

/// What the fight showed: Macha's acts, the party's levels as it met
/// her, what each of her acts took off the party, and how the run ended.
#[derive(Default)]
struct Played {
    acts: BTreeSet<i16>,
    levels: Vec<i16>,
    hurt: BTreeMap<i16, i32>,
    frames: u32,
    ended_at: Option<u32>,
    over_at: Option<u32>,
}

/// `frames` of the pilot's play; `each` takes every frame's events.
/// `PINEY_TRACE_BOSS` prints the fight a line a second, `PINEY_TRACE_HITS`
/// each hit on the party, `PINEY_TRACE_KITE` Kite and the pilot a frame.
fn play(s: &mut Session, frames: u32, mut each: impl FnMut(&mut Session, &Frame, u32)) -> Played {
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
        if event_flag(s, 307).is_some_and(|x| x & 3 << 62 != 0) {
            p.ended_at = Some(f);
            break;
        }
        let Stage::Area(a) = &s.stage else { continue };
        let c = a.world().combat();
        let Some(me) = c.boss_char() else { continue };
        let ch = &c.scene.chars[me];
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| matches!(b.class, Class::Macha(_)))
        else {
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

/// Macha fought to the end of event 307 with no aid: no god, the party
/// as the story's start leaves it (Kite 70, BlackRose 70, Balmung 75:
/// the strongest who answer calls there), the pilot's own tactics.
/// `PINEY_MACHA_GOD` (the console's god) and `PINEY_MACHA_EXP` (that much
/// experience to each member) are diagnostics; the test is without them.
#[test]
#[ignore]
fn machas_fight_to_its_end() {
    let Some(mut s) = event_307_arena(&[15, 6]) else { return };
    if std::env::var_os("PINEY_MACHA_GOD").is_some() {
        s.console("god");
    }
    let exp = std::env::var("PINEY_MACHA_EXP").ok();
    let p = play(&mut s, 60_000, |s, _, f| {
        s.take_events();
        if f == 100
            && let Some(n) = &exp
        {
            eprintln!("{}", s.console(&format!("exp {n}")));
        }
    });
    eprintln!(
        "levels {:?} acts {:?} hurt {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.hurt, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 307 did not end: {}", Mode::title(&s));
}

/// Event 307 from its own story start (the desktop, town 4's gate, area
/// 107's dungeon, the arena) with no aid: the party the pilot calls,
/// levelled only by what it fights on the way.
#[test]
#[ignore]
fn event_307_from_its_start() {
    let Some(mut s) = story_session_on("quarantine", 307, |_| {}) else { return };
    let p = play(&mut s, 400_000, |s, _, _| {
        s.take_events();
    });
    eprintln!(
        "levels {:?} acts {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 307 did not end: {}", Mode::title(&s));
}

/// The fight under god from the arena to the event's end, every frame's
/// events through [`crate::handle`] from the start: Macha's music heard
/// through it, and a shot every ten seconds and at each act's first
/// frame (`PINEY_SHOTS`, else the scratch's `qua/macha`).
#[test]
#[ignore]
fn macha_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/macha".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_307_arena(&[15, 6]) else { return };
    s.console("god");
    let audio = piney_audio::Audio::headless(&s.iso).ok();
    let mut gs: Option<piney_gs::Gs> = None;
    let mut seen: BTreeSet<i16> = BTreeSet::new();
    let mut music = 0u32;
    let p = play(&mut s, 60_000, |s, frame, n| {
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
        if !first && !n.is_multiple_of(600) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/macha-{n:05}-act{}.png", act.unwrap_or(-1));
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
    eprintln!("acts {:?} ended {:?} music {music} of {}", p.acts, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 307 did not end: {}", Mode::title(&s));
    assert!(music * 2 > p.frames, "the fight's music through most of it");
}
