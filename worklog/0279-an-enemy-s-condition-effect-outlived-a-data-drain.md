---
number: 279
title: An enemy's condition effect outlived a Data Drain: clearConditionEnemy's ClearConditionEffect carried out
date: 2026-09-28
area: battle, render
files: crates/piney-world/src/combat/mod.rs, crates/piney-game/src/session.rs
---

# 279. An enemy's condition effect outlived a Data Drain: clearConditionEnemy's ClearConditionEffect carried out

The report: enemies' effects stay after they die, a floating staff and red
marks after BATTLE MODE OFF. A character under a condition wears a
`ccConditionEffect` (INF main particle.cpp). For a stat change (numbers
7-18 down, 23-34 up) it is:
- particles that follow the character;
- an `effAbilityDown` or `effAbilityUp` controller, the stat's icon.

Only two calls end an enemy's (xrefs, INF gcmn):
- `ccChar::DispConditionEffect` (0x0056f950) kills it (it fades) or
  deletes it (it ends at once) as the conditions change. On a dead
  character (`dead` not 0 or 1) it kills it.
- `ccChar::ClearConditionEffect` (0x00570180) deletes it and sets
  `conditionNum` (+0x30) to -1. Its enemy callers are
  `clearConditionEnemy` (0x004335d0), `interruptThink` and
  `selectTarget`.

`clearConditionEnemy` runs:
- on the dying count;
- on a freeze;
- on a Data Drain (`drainFlag`), just before the enemy leaves the
  command lists (`deleteCmnd`) and its drained form is entered.

## What the port did

The port's enemy AI returns `Out::ClearConditionEffect` from
`clear_condition_enemy`, but no world code read it. The effect was left
to the enemy's next `DispConditionEffect`.

A foe killed outright gets one: its frames run on through the death, and
`dead` 2 makes it kill the effect in the same frame. That is why event
3's goblins, fought to the end and shot afterwards (`after_a_fight_shots`),
left nothing.

A drained foe does not. It is off the lists at once, nothing of it runs
again, and its effect was never ended:
- the particles' generators, which follow the character's position and
  were never killed;
- the icon's controller, unless its own target check (`check_target`)
  ended it. Whether it did was not checked.

With a goblin given attack down (`temp[0]` -10) and drained, its
condition effect 7 was still live when the drain's windows closed.

## The fix

`Combat::disp_conditions_shown` now takes, in order with the
`DispConditionEffect` calls, each enemy's `ClearConditionEffect`. If the
enemy has a live effect, it deletes it (`deleteConditionEffect`, a
`Show::ConditionEffect` of `CondFx::Delete`).

`a_drained_foe_keeps_no_condition_effect` gives the goblins attack down
in the fight of `drain_in_a_fight`. It checks that each one that wore the
effect and left the lists has none. It fails without the fix: foe 52 kept
effect 7. `drain_in_a_fight`'s per-frame hook now takes `&mut Session`,
so a test can change the fight.

Whether the report's staff and red marks were this is not confirmed.
The icon of the attack-down change is one of `effAbilityDown`'s rows,
and its sparks are red. The report did not say whether the foe was
drained.

**Still unknown:**
- Which enemy and area the report's staff came from, and whether it was
  drained.
- Whether the icon rows of `effAbilityDown` include a staff (magic attack
  down): the rows were not rendered one by one.
- A killed foe's effect now ends by the port's `DispConditionEffect`
  kill in the same frame as the game's delete. Which of the two runs
  first in the game's frame, and so whether the effect fades or ends at
  once, was not traced.
