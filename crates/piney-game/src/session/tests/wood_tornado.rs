//! Issue #48: a level 3 or 4 tornado (RaJuk Rom, 243, the Harpy and
//! Phoenix Queens' and the Wood Stocks'; PhaJuk Rom, 244) held its targets
//! and hit no one. From level 3 the tornado element (`ccTornadeElement`)
//! makes the damage calls in `ccThEffect`; the port routed only the calls
//! of the systems (`ccThSkill`) to the battle, so the element's were lost.

use piney_data::volume::Volume;

use super::revive::{area, field_with_mia_and_elk, step};
use super::*;

/// Mimic, `enemyTbl` row 240: no element resists the wood spells.
const MIMIC: i32 = 240;

/// The field with the party and a Mimic `dx` from Kite, all at 9999 HP;
/// (session, Kite, the Mimic).
fn kite_and_mimic(dx: f32) -> Option<(Session, usize, usize)> {
    let mut s = field_with_mia_and_elk()?;
    let w = area(&mut s).world_mut();
    let k = w.combat().kite.unwrap();
    let mut at = w.combat().scene.chars[k].pos;
    at[0] = (f32::from_bits(at[0]) + dx).to_bits();
    let foe = w.put_enemy(MIMIC, at, 0).expect("the Mimic");
    let c = w.combat_mut();
    let party: Vec<usize> = c.members.iter().map(|m| m.1).collect();
    for i in party.into_iter().chain([foe]) {
        let ch = &mut c.scene.chars[i];
        (ch.hp, ch.max_hp, ch.sp, ch.max_sp) = (9999, 9999, 999, 999);
    }
    // On the lists, so a request takes its type (`tType`).
    step(&mut s, 2);
    assert!(area(&mut s).world().combat().scene.listed(foe));
    Some((s, k, foe))
}

/// `sid` cast by the Mimic on Kite (`by_foe`) or by Kite on the Mimic: the
/// run's count (after the frame) at each frame the target lost HP to it
/// (its last affect the spell's). None without the disc.
fn hits(by_foe: bool, sid: i32) -> Option<Vec<i16>> {
    let (mut s, k, foe) = kite_and_mimic(400.0)?;
    let (by, on) = if by_foe { (foe, k) } else { (k, foe) };
    area(&mut s).world_mut().combat_mut().skill_request(by, Some(on), sid);
    let mut last = area(&mut s).world().combat().scene.chars[on].hp;
    let mut out = Vec::new();
    for _ in 0..200 {
        step(&mut s, 1);
        let c = area(&mut s).world().combat();
        let Some(count) = c.skills.borrow().runs.iter().find(|r| r.id == sid).map(|r| r.count) else { break };
        let ch = &c.scene.chars[on];
        if ch.hp < last && ch.affect.param[1] == sid as i16 {
            out.push(count);
        }
        last = ch.hp;
    }
    Some(out)
}

/// Every hit of `sid` both ways on one of `at`, and at least two.
fn hits_on(sid: i32, at: &[i16]) {
    for by_foe in [true, false] {
        let Some(seen) = hits(by_foe, sid) else { return };
        assert!(seen.len() >= 2, "{sid} by the foe {by_foe}: hits at {seen:?}");
        for c in &seen {
            assert!(at.contains(c), "{sid} by the foe {by_foe}: a hit at {c}, not on {at:?}");
        }
    }
}

/// The tornado element's damage counts (`_Level3` gcmn 0x004ff4a0,
/// `_Level4` 0x004ff8b0): every 5 from `START_2[1] + 5` (level 4
/// `START_2[3] + 5`) to 40 on. The element first runs the frame after the
/// system made it, and the run's count is read after its increment: a hit
/// on the element's count `n` shows at the run's count `n + 2`.
fn element_counts(level: i32) -> Vec<i16> {
    let t = piney_data::tables::effect::of(Volume::Inf);
    let first = if level == 3 { t.tornade_start2()[1] } else { t.tornade_start2b()[3] } + 5;
    (0..9).map(|i| (first + 5 * i + 2) as i16).collect()
}

/// RaJuk Rom and PhaJuk Rom, a foe's on Kite and Kite's on a foe: the
/// element's hits take HP, each on one of its damage counts. In the bug
/// none did.
#[test]
fn a_level_three_or_four_tornado_hits_on_its_elements_counts() {
    hits_on(243, &element_counts(3));
    hits_on(244, &element_counts(4));
}

/// BiJuk Rom (242, the Green Wyrm's) and Juk Rom (241): the system's own
/// hits, every 5 from its count 35 to 75 (`TornadoSystem`, gcmn
/// 0x00577540), shown at the run's count after its increment.
#[test]
fn a_level_one_or_two_tornado_hits_from_count_35() {
    let at: Vec<i16> = (0..9).map(|i| 36 + 5 * i).collect();
    hits_on(241, &at);
    hits_on(242, &at);
}

/// The other spells whose level 3 and 4 elements make their damage calls
/// in `ccThEffect` (fall's, the thunder fall's, upheaval's, summons'; and
/// convergence's, whose system calls once the element is gone): Kite's on
/// a Mimic reach it (its last affect the spell's, a hit or a miss).
#[test]
fn every_level_three_or_four_element_spell_reaches_its_target() {
    for sid in [195, 196, 259, 260, 203, 204, 207, 208, 215, 216] {
        let Some((mut s, k, foe)) = kite_and_mimic(400.0) else { return };
        area(&mut s).world_mut().combat_mut().skill_request(k, Some(foe), sid);
        let mut reached = false;
        for _ in 0..600 {
            step(&mut s, 1);
            let c = area(&mut s).world().combat();
            let a = &c.scene.chars[foe].affect;
            reached |= a.ty == 1 && a.param[1] == sid as i16;
            if reached || !c.skills.borrow().runs.iter().any(|r| r.id == sid) {
                break;
            }
        }
        assert!(reached, "{sid}: no damage call reached the Mimic");
    }
}
