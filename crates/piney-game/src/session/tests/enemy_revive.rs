//! Issue #41: enemies never revived their fallen. `ccEnemyInfluence` sets
//! the enemy's `affectFlag` whoever made the affect; the port kept a copy
//! of it that only an enemy's own frame and the menus wrote, so a skill's
//! revival (Rip Maen, 180) or a party member's blow never reached
//! `interruptThink`.

use piney_battle::enemy_ai::act;
use piney_battle::param::cond;

use super::revive::{area, field_with_mia_and_elk, step};
use super::*;

/// Menhir, `enemyTbl` row 179: mag0 is Rip Maen (180).
const MENHIR: i32 = 179;
const RIP_MAEN: i16 = 180;
const CONVERGENCE: i32 = 213;

/// A group of Menhirs (`put_enemy` makes three) `dx` from Kite, the party
/// kept up: (session, the index `put_enemy` gave).
fn menhirs(dx: f32) -> Option<(Session, usize)> {
    let mut s = field_with_mia_and_elk()?;
    let w = area(&mut s).world_mut();
    let c = w.combat_mut();
    for p in c.members.iter().map(|m| m.1).collect::<Vec<_>>() {
        c.scene.chars[p].no_death = true;
    }
    let mut at = c.scene.chars[c.kite.unwrap()].pos;
    at[0] = (f32::from_bits(at[0]) + dx).to_bits();
    let foe = w.put_enemy(MENHIR, at, 0).expect("the Menhirs");
    Some((s, foe))
}

/// A Menhir felled while the others fight: one of them casts Rip Maen on
/// it while it dies (`checkSkillList` finds it, `dead` 2, within the
/// spell's range; `selectAttack` puts the revival first). The spell's
/// affect 20 restores its HP (`affectEnemy`) and wakes it
/// (`interruptThink`: `dead` 0, wait); it stays on the lists, no
/// experience given. In the bug it got its HP back and died anyway.
#[test]
fn a_menhir_revives_its_fallen() {
    let Some((mut s, fallen)) = menhirs(400.0) else { return };
    step(&mut s, 60);
    let k = area(&mut s).world().combat().kite;
    area(&mut s).world_mut().entry_affect(fallen, k, 1, [9999, 0, 0]);
    let mut revived = None;
    for f in 0..90 {
        step(&mut s, 1);
        let c = area(&mut s).world().combat();
        let ch = &c.scene.chars[fallen];
        if ch.affect.ty == 20 {
            revived = Some((f, ch.affect.person, ch.affect.param[1]));
            break;
        }
    }
    let (f, by, sid) = revived.expect("no Menhir cast Rip Maen on the fallen");
    eprintln!("revived on frame {f} by {by:?}");
    let c = area(&mut s).world().combat();
    let by = by.expect("the caster");
    assert_eq!((c.scene.chars[by].ty(), c.scene.chars[by].id(), sid), (0x20, MENHIR as i16, RIP_MAEN));
    step(&mut s, 2);
    let c = area(&mut s).world().combat();
    let ch = &c.scene.chars[fallen];
    assert_eq!((ch.cond[cond::DEAD], ch.hp, ch.sp), (0, ch.max_hp, 0), "full HP, no SP, up");
    assert!(!matches!(c.foes[fallen].as_ref().unwrap().act_num, act::DYING | act::DEAD));
    // Past the 90 frames dying would have taken: still standing, listed.
    step(&mut s, 120);
    let c = area(&mut s).world().combat();
    assert!(c.scene.listed(fallen), "the revived Menhir left the lists");
    assert_eq!(c.scene.chars[fallen].cond[cond::DEAD], 0);
}

/// A party member's blow (its frame's `EntryAffect(1)`) on a Menhir that
/// is not attacking or flinching makes it flinch (act 7) that frame, as
/// `interruptThink` reads `affectFlag`. The bug's copy missed every blow
/// made outside an enemy's own frame: Kite's, the members', the skills'.
#[test]
fn a_members_blow_flinches_a_foe() {
    let Some((mut s, foe)) = menhirs(300.0) else { return };
    // Kite opens the fight with a spell (Convergence), and the members join.
    let c = area(&mut s).world_mut().combat_mut();
    let k = c.kite.unwrap();
    let members: Vec<usize> = c.members.iter().map(|m| m.1).filter(|&m| m != k).collect();
    c.scene.chars[k].sp = 999;
    step(&mut s, 5);
    area(&mut s).world_mut().combat_mut().skill_request(k, Some(foe), CONVERGENCE);
    for _ in 0..600 {
        let c = area(&mut s).world().combat();
        let before = (c.scene.chars[foe].affect.param, c.foes[foe].as_ref().unwrap().act_num);
        step(&mut s, 1);
        let c = area(&mut s).world().combat();
        let ch = &c.scene.chars[foe];
        let blow = ch.affect.ty == 1 && ch.affect.param[0] > 0 && ch.affect.param != before.0;
        let by_member = ch.affect.person.is_some_and(|p| members.contains(&p));
        if blow && by_member && !matches!(before.1, act::ATTACK | act::DAMAGE) && ch.hp > 0 {
            assert_eq!(c.foes[foe].as_ref().unwrap().act_num, act::DAMAGE, "the blow: {:?}", ch.affect);
            return;
        }
    }
    panic!("no member's blow reached the Menhir");
}
