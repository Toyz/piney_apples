//! Macha (`ccBoss06`, Quarantine's event 307 in field 6's arena): the
//! fight played from the arena and from the event's own start under the
//! story autopilot with no aid, and its pictures and music under god.
//! docs/engine/boss-macha.md.

use piney_battle::boss::Class;

use super::arena::{EVENT_307, arena_session, god_shots, play};
use super::*;

fn macha(c: &Class) -> bool {
    matches!(c, Class::Macha(_))
}

/// Macha fought to the end of event 307 with no aid: no god, the party
/// as the story's start leaves it (Kite 70, BlackRose 70, Balmung 75:
/// the strongest who answer calls there), the pilot's own tactics.
/// `PINEY_MACHA_GOD` (the console's god) and `PINEY_MACHA_EXP` (that much
/// experience to each member) are diagnostics; the test is without them.
#[test]
#[ignore]
fn machas_fight_to_its_end() {
    let Some(mut s) = arena_session(&EVENT_307, &[15, 6]) else { return };
    if std::env::var_os("PINEY_MACHA_GOD").is_some() {
        s.console("god");
    }
    let exp = std::env::var("PINEY_MACHA_EXP").ok();
    let p = play(&mut s, 307, macha, 60_000, |s, _, f| {
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
    let p = play(&mut s, 307, macha, 400_000, |s, _, _| {
        s.take_events();
    });
    eprintln!(
        "levels {:?} acts {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 307 did not end: {}", Mode::title(&s));
}

/// The fight under god from the arena to the event's end: Macha's music
/// heard through it, and its shots (`PINEY_SHOTS`, else the scratch's
/// `qua/macha`).
#[test]
#[ignore]
fn macha_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/macha".into());
    let Some(mut s) = arena_session(&EVENT_307, &[15, 6]) else { return };
    let (p, music) = god_shots(&mut s, 307, macha, &dir, "macha");
    eprintln!("acts {:?} ended {:?} music {music} of {}", p.acts, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 307 did not end: {}", Mode::title(&s));
    assert!(music * 2 > p.frames, "the fight's music through most of it");
}
