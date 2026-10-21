//! Kyvia's pictures in its fights: the meteors (`ANM_x300`) with their
//! trails, the beams' particles following their ends, the core's auras
//! following it, and the second fight's thunderbolts (`CMP_x012`). Before,
//! the fight's generators were never started and its meteors and bolts had
//! no picture.

use std::collections::BTreeMap;
use std::path::PathBuf;

use piney_battle::boss::Class;
use piney_input::{Buttons, Raw};
use piney_world::combat::Combat;

use super::*;

/// Event `n` of `disc` brought to Kyvia's disc, field `field` of town 2
/// (`EVENTAREAB8`), as area 47's dungeon leads there: blocks below `block`
/// played and `eventStatus[0]` 1 (block `block` makes Kyvia).
fn on_the_disc(disc: &str, n: i32, block: u32, field: i32) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
    if !iso.exists() {
        eprintln!("{disc}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, n).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(n as usize);
    state.save.set_event_flag(n as usize, f | ((1 << block) - 1));
    state.save.set_u8(offset::EVENT_STATUS, 1);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 47, false).unwrap();
    let wm = crate::area::ev_area_number(&iso, field, &wm, &state.save).unwrap();
    scene.change_scene(1, 2, field, -1, -1, -1, &mut state.save);
    Some(Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap())
}

/// Mutation's event 108 on field 9's disc (block 21 makes Kyvia 01).
fn event_108_disc() -> Option<Session> {
    on_the_disc("mutation", 108, 21, 9)
}

/// Outbreak's event 211 on field 10's disc (block 11 makes Kyvia 02).
fn event_211_disc() -> Option<Session> {
    on_the_disc("outbreak", 211, 11, 10)
}

/// What one frame of the fight showed: the body's act, the boss effects'
/// draws, the generators following the fight's moving places (the beams'
/// ends, a gomora's aura, the meteors) and those following the core.
#[derive(Default)]
struct Seen {
    act: i16,
    draws: Vec<String>,
    anchored: usize,
    on_core: usize,
}

/// The core's character.
fn core_of(c: &Combat, me: usize) -> Option<usize> {
    match &c.scene.chars[me].foe_state()?.boss.as_ref()?.class {
        Class::Kyvia(x) => Some(x.core),
        _ => None,
    }
}

/// The fight for `frames` frames from the body's coming, the party kept
/// up, the event's lines moved on, and each time the core is out its
/// time run short (`NowDmgtime` to `Dmgtime`), so the body strikes in
/// turn: the arm, the beams, the meteors (and in the second fight the
/// thunder). `each` sees every frame.
fn fight(s: &mut Session, frames: u32, mut each: impl FnMut(&Session, &Frame, u32, &Seen)) -> Vec<Seen> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut seen) = (None, Vec::new());
    for n in 0..frames + 6000 {
        let talking = match &s.stage {
            Stage::Area(a) => a.vm().is_some_and(|v| v.playing().is_some()) || a.ui().ctrl.check_menu_type() != -1,
            _ => false,
        };
        pad.read(&if talking && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let Some(core) = core_of(c, me) else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        if c.scene.listed(core)
            && let Some(Class::KyviaCore(x)) =
                c.scene.chars[core].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
        {
            x.now_dmg_time = x.now_dmg_time.max(x.dmg_time - 1);
        }
        let now = Seen {
            act: c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map_or(-1, |b| b.act_num),
            draws: census.boss_draws,
            anchored: census.anchored_generators,
            on_core: census.char_generators.iter().filter(|&&g| g as usize == core).count(),
        };
        each(s, &frame, i, &now);
        seen.push(now);
    }
    seen
}

/// Frames each boss effect draw showed in, and the frames with generators
/// following the fight's places and the core.
fn tally(seen: &[Seen]) -> (BTreeMap<String, u32>, u32, u32) {
    let mut drawn: BTreeMap<String, u32> = BTreeMap::new();
    for now in seen {
        let mut names = now.draws.clone();
        names.dedup();
        for d in names {
            *drawn.entry(d).or_default() += 1;
        }
    }
    let anchored = seen.iter().filter(|n| n.anchored > 0).count() as u32;
    let on_core = seen.iter().filter(|n| n.on_core > 0).count() as u32;
    (drawn, anchored, on_core)
}

/// Mutation's first fight: the meteors fall drawn (`ANM_x300`) with their
/// trails, the beams' and the gomoras' generators follow their places,
/// and the core's auras follow the core.
#[test]
fn kyvia_shows_its_meteors_beams_and_auras() {
    let Some(mut s) = event_108_disc() else { return };
    let seen = fight(&mut s, 12000, |_, _, _, _| {});
    let (drawn, anchored, on_core) = tally(&seen);
    let acts: std::collections::BTreeSet<i16> = seen.iter().map(|n| n.act).collect();
    println!("acts {acts:?}, drawn {drawn:?}, anchored {anchored} frames, on the core {on_core} frames");
    assert!(drawn.get("ANM_x300").is_some_and(|&n| n > 10), "the meteors not drawn: {drawn:?}");
    assert!(anchored > 100, "no generator followed the fight's places ({anchored} frames)");
    assert!(on_core > 100, "no aura followed the core ({on_core} frames)");
}

/// Outbreak's second fight: its thunder's bolts are drawn (`CMP_x012`),
/// and its meteors.
#[test]
fn kyvia_s_second_fight_shows_its_thunderbolts() {
    let Some(mut s) = event_211_disc() else { return };
    let seen = fight(&mut s, 16000, |_, _, _, _| {});
    let (drawn, anchored, on_core) = tally(&seen);
    let acts: std::collections::BTreeSet<i16> = seen.iter().map(|n| n.act).collect();
    println!("acts {acts:?}, drawn {drawn:?}, anchored {anchored} frames, on the core {on_core} frames");
    assert!(acts.contains(&6), "no thunder: {acts:?}");
    assert!(drawn.get("CMP_x012").is_some_and(|&n| n > 30), "the bolts not drawn: {drawn:?}");
    assert!(on_core > 100, "no aura followed the core ({on_core} frames)");
}

/// Shots of both fights to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a420): every 6th frame a meteor or a bolt is
/// drawn, or the beams' generators run, through each fight's first 4000.
#[test]
#[ignore]
fn kyvia_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a420".into());
    std::fs::create_dir_all(&dir).unwrap();
    for (name, s) in [("mut", event_108_disc()), ("out", event_211_disc())] {
        let Some(mut s) = s else { continue };
        let mut gs: Option<piney_gs::Gs> = None;
        fight(&mut s, 4000, |s, frame, i, now| {
            let shown = now.draws.iter().any(|d| d == "ANM_x300" || d == "CMP_x012") || now.anchored > 0;
            if !(shown && i.is_multiple_of(6)) {
                return;
            }
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            let (w, h) = g.target_size();
            let path = format!("{dir}/kyvia-{name}-{i:05}-act{}.png", now.act);
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        });
    }
}
