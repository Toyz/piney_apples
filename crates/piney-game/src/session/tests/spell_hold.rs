//! A spell or art used against a character holds it where it stands while
//! the skill runs (`ccSkillHold`, gcmn 0x005752b0, each frame of
//! `ccSkill::Main` while `holdFlag` is set). An attack spell's flag lives
//! in its element system's copy too, which the port's effects started
//! clear: the first system frame wrote it back clear, and the hold lasted
//! one frame.

use piney_battle::param::cond;

use super::revive::{area, field_with_mia_and_elk};
use super::*;

/// Mimic, `enemyTbl` row 240.
const MIMIC: i32 = 240;
/// An attack spell (Convergence) and an art.
const SPELL: i32 = 213;
const ART: i32 = 6;

/// One frame with the left stick pushed right.
fn step_right(s: &mut Session) {
    let mut pad = Pad::default();
    pad.read(&Raw { analog: true, lx: 255, ly: 128, rx: 128, ry: 128, ..Raw::default() });
    s.step(&pad);
    s.take_events();
}

fn dist(a: [u32; 4], b: [u32; 4]) -> f32 {
    (0..3).map(|i| (f32::from_bits(b[i]) - f32::from_bits(a[i])).powi(2)).sum::<f32>().sqrt()
}

/// The field with Kite, a Mimic `dx` from him, both with HP and SP to
/// spare; (session, Kite, the Mimic).
fn kite_and_foe(dx: f32) -> Option<(Session, usize, usize)> {
    let mut s = field_with_mia_and_elk()?;
    let w = area(&mut s).world_mut();
    let k = w.combat().kite.unwrap();
    let mut at = w.combat().scene.chars[k].pos;
    at[0] = (f32::from_bits(at[0]) + dx).to_bits();
    let foe = w.put_enemy(MIMIC, at, 0).expect("the Mimic");
    for c in [k, foe] {
        let ch = &mut w.combat_mut().scene.chars[c];
        (ch.hp, ch.max_hp, ch.sp, ch.max_sp) = (9999, 9999, 999, 999);
    }
    Some((s, k, foe))
}

/// `sid` used by `by` on `on`, the stick held; per frame while its run
/// holds: (`on`'s hold condition, how far `on` moved).
fn while_held(s: &mut Session, by: usize, on: usize, sid: i32) -> Vec<(i16, f32)> {
    area(s).world_mut().combat_mut().skill_request(by, Some(on), sid);
    let mut seen = Vec::new();
    for _ in 0..300 {
        let before = area(s).world().combat().scene.chars[on].pos;
        step_right(s);
        let c = area(s).world().combat();
        let holds = c.skills.borrow().runs.iter().any(|r| r.id == sid && r.hold);
        if !holds {
            break;
        }
        seen.push((c.scene.chars[on].cond[cond::HOLD], dist(before, c.scene.chars[on].pos)));
    }
    seen
}

/// A foe's spell on Kite: he stands still under it with the stick held,
/// for as long as its system holds him (48 frames; one in the bug), and
/// runs again after.
#[test]
fn a_foes_spell_holds_kite() {
    let Some((mut s, k, foe)) = kite_and_foe(600.0) else { return };
    let mut ran = 0.0;
    for _ in 0..20 {
        let before = area(&mut s).world().combat().scene.chars[k].pos;
        step_right(&mut s);
        ran = dist(before, area(&mut s).world().combat().scene.chars[k].pos);
    }
    assert!(ran > 10.0, "Kite never ran: {ran}");
    let seen = while_held(&mut s, foe, k, SPELL);
    assert!(seen.len() > 30, "held only {} frames", seen.len());
    for (n, &(hold, moved)) in seen.iter().enumerate().skip(1) {
        assert_eq!(hold, 1, "frame {n}: Kite's hold");
        assert_eq!(moved, 0.0, "frame {n}: Kite moved under the spell");
    }
    // He runs again; how far before the field's walls stop him is the
    // fight's (the Mimic's ccRand moves it).
    let mut ran: f32 = 0.0;
    for _ in 0..60 {
        let before = area(&mut s).world().combat().scene.chars[k].pos;
        step_right(&mut s);
        ran = ran.max(dist(before, area(&mut s).world().combat().scene.chars[k].pos));
    }
    assert!(ran > 10.0, "Kite stayed held: {ran}");
}

/// Kite's spell and art on a foe hold it for their runs.
#[test]
fn kites_spell_and_art_hold_the_foe() {
    for sid in [SPELL, ART] {
        let Some((mut s, k, foe)) = kite_and_foe(120.0) else { return };
        let seen = while_held(&mut s, k, foe, sid);
        assert!(seen.len() > 30, "skill {sid}: held only {} frames", seen.len());
        assert!(seen.iter().all(|&(hold, _)| hold == 1), "skill {sid}: the foe let go: {seen:?}");
    }
}
