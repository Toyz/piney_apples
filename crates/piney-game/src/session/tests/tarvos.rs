//! Tarvos (`ccBoss07`, Quarantine's event 310 in field 7's arena): the
//! fight under god from the arena, its acts, pictures and music; and with
//! no aid from the arena and from the event's own start under the story
//! autopilot. docs/engine/boss-tarvos.md.

use piney_battle::boss::Class;
use piney_battle::boss::tarvos::act;

use super::arena::{EVENT_310, arena_session, god_shots, play};
use super::*;

fn tarvos(c: &Class) -> bool {
    matches!(c, Class::Tarvos(_))
}

/// The fight under god from the arena to event 310's end: its patterns'
/// acts met (the spike rain, the wave, the sacrifice, the phantom, the
/// transfer, the skills and spells), the drain and the Epitaph, the music
/// through most of it; shots in `PINEY_SHOTS`, else the scratch's
/// `qua/tarvos`.
#[test]
#[ignore]
fn tarvos_under_god() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/tarvos".into());
    let Some(mut s) = arena_session(&EVENT_310, &[15, 6]) else { return };
    let (p, music) = god_shots(&mut s, 310, tarvos, &dir, "tarvos");
    eprintln!("acts {:?} hurt {:?} ended {:?} music {music} of {}", p.acts, p.hurt, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 310 did not end: {}", Mode::title(&s));
    for a in [act::NEUTRAL, act::EPITAPH, act::DEAD] {
        assert!(p.acts.contains(&a), "act {a} met");
    }
    assert!(music * 2 > p.frames, "the fight's music through most of it");
}

/// Tarvos fought to the end of event 310 with no aid: no god, Kite with
/// BlackRose and Balmung as the story leaves them, the pilot's own
/// tactics. `PINEY_TARVOS_GOD` (the console's god) is a diagnostic.
#[test]
#[ignore]
fn tarvos_fight_to_its_end() {
    let Some(mut s) = arena_session(&EVENT_310, &[15, 6]) else { return };
    if std::env::var_os("PINEY_TARVOS_GOD").is_some() {
        s.console("god");
    }
    let p = play(&mut s, 310, tarvos, 60_000, |s, _, _| {
        s.take_events();
    });
    eprintln!(
        "levels {:?} acts {:?} hurt {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.hurt, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 310 did not end: {}", Mode::title(&s));
}

/// Event 310 from its own story start (Lia Fail, area 102's dungeon, the
/// arena) with no aid.
#[test]
#[ignore]
fn event_310_from_its_start() {
    let Some(mut s) = story_session_on("quarantine", 310, |_| {}) else { return };
    let p = play(&mut s, 310, tarvos, 400_000, |s, _, _| {
        s.take_events();
    });
    eprintln!(
        "levels {:?} acts {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 310 did not end: {}", Mode::title(&s));
}
