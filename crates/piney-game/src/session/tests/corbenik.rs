//! Corbenik (`ccBoss08`, Quarantine's event 313 in field 8's arena): its
//! first two forms in one task, then the third once the save says so;
//! under god from the arena with shots. docs/engine/boss-corbenik.md.

use piney_battle::boss::Class;

use super::arena::{EVENT_313, EVENT_313_THIRD, arena_session, god_shots_for};
use super::*;

fn corbenik(c: &Class) -> bool {
    matches!(c, Class::Corbenik(_) | Class::CorbenikB(_) | Class::CorbenikC(_))
}

/// Forms one and two under god from the arena to event 313's end (the
/// third made after the scene change): shots in `PINEY_SHOTS`, else the
/// scratch's `qua/corbenik`.
#[test]
#[ignore]
fn corbenik_under_god() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/corbenik".into());
    let Some(mut s) = arena_session(&EVENT_313, &[15, 6]) else { return };
    let (p, music) = god_shots_for(&mut s, 313, corbenik, &dir, "corbenik", 240_000);
    eprintln!("acts {:?} hurt {:?} ended {:?} music {music} of {}", p.acts, p.hurt, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 313 did not end: {}", Mode::title(&s));
}

/// The third form alone under god (block 19 on).
#[test]
#[ignore]
fn corbenik_third_under_god() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/corbenik3".into());
    let Some(mut s) = arena_session(&EVENT_313_THIRD, &[15, 6]) else { return };
    let (p, music) = god_shots_for(&mut s, 313, corbenik, &dir, "corbenik3", 120_000);
    eprintln!("acts {:?} hurt {:?} ended {:?} music {music} of {}", p.acts, p.hurt, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 313 did not end: {}", Mode::title(&s));
}

/// A diagnostic: the first frames of the fight, the boss's HP, the lists
/// and what the pilot does every two seconds (`PINEY_CORBENIK_FRAMES`).
#[test]
#[ignore]
fn corbenik_trace() {
    let n = std::env::var("PINEY_CORBENIK_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let Some(mut s) = arena_session(&EVENT_313, &[15, 6]) else { return };
    s.console("god");
    super::arena::play(&mut s, 313, corbenik, n, |s, _, f| {
        s.take_events();
        if !f.is_multiple_of(600) {
            return;
        }
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        let Some(me) = c.boss_char() else { return };
        let ch = &c.scene.chars[me];
        let act = ch.foe_state().and_then(|f| f.boss.as_ref()).map(|b| (b.act_num, b.exit, b.draw_sw));
        eprintln!(
            "F {f} hp {}/{} act {act:?} listed {} enemies {:?} target {:?} | {}",
            ch.hp,
            ch.max_hp,
            c.scene.listed(me),
            c.enemies(),
            a.world().command_target_code(),
            super::survey::fighters(a)
        );
    });
}
