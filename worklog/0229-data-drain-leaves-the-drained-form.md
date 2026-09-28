---
number: 229
title: Data Drain leaves the drained form
date: 2026-09-27
area: battle
files: crates/piney-world/src/combat/mod.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-game/src/session.rs
---

# 229. Data Drain leaves the drained form

Reported from play on Mutation: after a Data Drain the monster stayed as
it was, still marked "Data Drain OK", instead of turning into its weak
drained form. The same happened on Infection. The session test drained
the goblin but never checked what it became.

## The cause

The drained form comes from the enemy's own frame. When `drainFlag` is
set, the frame registers the kill, clears the enemy's conditions, drops
its command, and spawns `ccGetDrainId`'s row (`entryDrainEnemy`). The flag
comes from `ccEnemyInfluence`: an `EntryAffect(13)` that lands on the
enemy sets it.

TargetMenu sends that affect when Kite drains (`Request::DrainAffect`).
The world ran it through `Combat::entry_affect_with`. That set the
character's own bits (`spc_flag::AFFECT`, `enemy_flag::DRAIN`), but not the
enemy AI's copy of them. Only affects made inside an enemy's frame
(`enemy_motion`'s `entry_affect`) updated that copy. So the enemy never
learned it had been drained. For the same reason, an affect from the
menus never interrupted an enemy's think.

## The fix

- `enemy_motion::affect_lands` holds `EntryAffect`'s rule for whether an
  affect is stored and whether the enemy's influence runs.
- `Enemy::note_landed` updates the enemy's copy from that answer:
  - the kind and first parameter once stored;
  - `affectFlag`, and a Data Drain's `drainFlag`, once the influence ran.
- Both the enemy frame's path and the menus' path
  (`Combat::entry_affect_with`) now use them.

The new test `data_drain_leaves_the_drained_form` drains the goblin
(row 130) through the menus and finds row 129, its drained form, standing
afterwards. The battle, world and game tests and the enemy motion harness
still pass.

**Still unknown:** No test yet drains an enemy on Mutation's disc, or
drains an area skill's sub-targets (Drain Arc, Drain Heart), which take
the same affect.
