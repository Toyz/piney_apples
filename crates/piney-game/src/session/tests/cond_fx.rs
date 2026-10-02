//! Worklog 310's open paths: a foe's condition effect (`ccConditionEffect`)
//! ended where the game calls `ClearConditionEffect` (gcmn 0x00570180) on
//! it: `selectTarget`'s clear with no party to fight (0x00436d84) run from
//! a blow outside the foe's frame, and `ccClearConditionAllEnemy`
//! (0x0042e4f0, the Grunty Flute's). Another foe's blow is
//! `piney_battle::enemy_motion`'s test.

use piney_battle::enemy_ai::act;
use piney_battle::param::cond;

use super::revive::{area, field_with_mia_and_elk, generators_on, step};
use super::*;

/// Mimic, `enemyTbl` row 240.
const MIMIC: i32 = 240;

/// A Mimic `dx` from Kite, paralysed until its sparks show (condition
/// effect 1).
fn paralysed_foe(s: &mut Session, dx: f32) -> usize {
    let w = area(s).world_mut();
    let k = w.combat().kite.unwrap();
    let mut at = w.combat().scene.chars[k].pos;
    at[0] = (f32::from_bits(at[0]) + dx).to_bits();
    let foe = w.put_enemy(MIMIC, at, 0).expect("the Mimic");
    w.combat_mut().scene.chars[foe].cond.v[cond::PARALYSIS] = 3000;
    step(s, 30);
    assert_eq!(area(s).world().combat().condition_effect(foe), Some(1), "the paralysis sparks");
    foe
}

/// No effect held for `foe`, nor drawn, nor 40 frames on (an effect left
/// with `conditionNum` -1 is never ended).
fn sparks_gone(s: &mut Session, foe: usize, how: &str) {
    assert_eq!(area(s).world().combat().condition_effect(foe), None, "{how}: the sparks outlived the clear");
    step(s, 40);
    assert_eq!(area(s).world().combat().condition_effect(foe), None, "{how}: the sparks came back");
    assert_eq!(generators_on(s, foe), 0, "{how}: the sparks still drawn");
}

/// The Grunty Flute's `ccClearConditionAllEnemy`: each active foe's
/// `clearConditionEnemy`, its effect ended at once and `conditionNum` -1.
/// The port cleared the conditions alone and left the sparks to the foe's
/// next look, or for good where it ran no frames.
#[test]
fn the_flute_ends_the_foes_sparks() {
    let Some(mut s) = field_with_mia_and_elk() else { return };
    let foe = paralysed_foe(&mut s, 600.0);
    area(&mut s).world_mut().combat_mut().clear_condition_all_enemy();
    let c = area(&mut s).world().combat();
    assert_eq!(c.scene.chars[foe].cond[cond::PARALYSIS], 0);
    assert_eq!(c.scene.chars[foe].condition_num, -1);
    sparks_gone(&mut s, foe, "the flute");
}

/// A blow outside the foe's frame (`EntryAffect` kind 1 from the world)
/// while no one of the party stands: `selectTarget`'s first branch clears
/// the waiting foe and its effect. The world dropped the rule's
/// `ClearConditionEffect`, leaving the sparks with `conditionNum` -1, which
/// `DispConditionEffect` never ends.
#[test]
fn a_blow_with_the_party_down_ends_the_sparks() {
    let Some(mut s) = field_with_mia_and_elk() else { return };
    let foe = paralysed_foe(&mut s, 600.0);
    let w = area(&mut s).world_mut();
    let c = w.combat_mut();
    for p in c.scene.pc_list.clone() {
        c.scene.chars[p].cond.v[cond::DEAD] = 3;
    }
    c.foes[foe].as_mut().unwrap().act_num = act::WAIT;
    w.entry_affect(foe, None, 1, [0, 0, 0]);
    // The party up again, so that no game over follows.
    let c = area(&mut s).world_mut().combat_mut();
    for p in c.scene.pc_list.clone() {
        c.scene.chars[p].cond.v[cond::DEAD] = 0;
    }
    assert_eq!(c.scene.chars[foe].condition_num, -1);
    sparks_gone(&mut s, foe, "the blow");
}
