---
number: 471
title: Infection's no-aid run passes event 14's Data Bug at level 7 and ends events up to 16: grinding in any field, rests for SP, boxes opened, skills at a Data Bug's protect
date: 2026-10-24
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs crates/piney-game/src/session/tests/shrine.rs
started: 2026-10-24T17:56:31-06:00
took: 1h 20m
---

# 471. Infection's no-aid run passes event 14's Data Bug at level 7 and ends events up to 16: grinding in any field, rests for SP, boxes opened, skills at a Data Bug's protect

## Event 14's Data Bug, and the grinding that reaches it

The brief's third point: event 14's Data Bug is not won by levels. Its
HP holds until its protect breaks and Kite drains it.

**What the code does** (`piney_battle::damage::protect`, INF gcmn
0x0056d910's tail):
- **The gauge.** Each hit on a Data Bug adds
  `(atk + skill atk)^2 / (2 ppDef)`, by the hit rate over 100, the
  elements and the skill's `dmgRate`.
- **The break.** At `maxPP` the protect breaks for 300 frames
  (`pp_count`). Then the drain can take it.
- **Why level matters.** The gauge grows with the square of the
  attack, so the party's level counts for much here. A skill's own
  attack counts too.

The pilot already drains a broken Data Bug (`drainable`, Data Drain
before the break mends). [[468]]'s "three blows" rule held its skills
back from foes its blows reach. A Data Bug is now exempt, and Kite
casts his strongest skill at it.

Measured from story start 14 (`PINEY_SURVEY_EXP`):
- level 1, no experience: the bug is not reached in 40,000 frames;
- level 5 (3,000 experience): the bug's room is reached, and nothing has
  fallen by 40,000 frames;
- level 8 (7,000 experience): the event is done.

## The grinding, made to run

Under [[469]]'s rules the party never got past level 2. Each trace
showed one more gap:
- **No blows in the fields.** Kite fought back only in fields the story
  wanted cleared. In any field he now walks at the foes within 1,500 or
  aiming at the party (`approach_foe`). The portals' foes came and only
  BlackRose fought them.
- **Items spent where rest does.** Out of a fight, with someone under
  70% and a heal known that its caster's SP does not yet pay for, the
  pilot now stands still (`resting`) and lets the SP come back. Standing
  out of a fight it returns at `maxSP / 20` a second
  (`ccSpcChar::CheckSpRegeneSpeed`). The Health Drinks are kept for
  fights.
- **Into the goal hurt.** Below its foes but hurt, the walk went on to the
  goal room and the story's fight there. It now waits until the party is
  mended (`grind_wanted`).
- **The dungeon walked into.** The walk to a far field portal passed the
  dungeon's door. On the way out for new portals, Kite now stands still
  until the gate's quiet runs out.
- **Stale state.** A grinding exit from one area outlived the visit and
  ended the grinding in the next. An exit now counts as come once the
  area changes, and the wanted flag is cleared each call.
- **Progress.** The whole run's stall rule gave up 200,000 frames after
  the last event. A rise in Kite's level now counts as progress too.

The rooms' treasure boxes (`gimmickTbl` rows 0 and 1, `ccGimBox`'s
boxes; rows 2-14 are the breakables and the virus crystal) are opened on
the way (`open_box`). A box not reached in 900 frames is passed by.

**Only unaided** (`StoryPilot::unaided`). These behaviours make a run
longer: the grinding, the boxes, the rests and the fights in any field.
They are on only with no god:
- `whole_story` with `PINEY_SURVEY_AIDS=0`;
- a survey start without `PINEY_SURVEY_GOD`.

The side events' tests run under god in a set number of frames. With
the behaviours on, SANJYURO-4 left its statue unopened, so they keep to
the story's way.

## Where Infection's no-aid run stands

The run ends events 1-4, 10-16:
- event 13 at frame 26,400;
- event 14 at 288,600, the Data Bug drained at level 7;
- events 15 and 16 at 308,400 and 310,500.

Kite's level rose at frames 47,700 (2), 92,100, 115,800, 154,800,
197,100 and 264,300 (7).

The game is then over at frame 313,238 in field 18. Kite, alone at
level 7 with his stock spent, falls to two Bee Armies and a Hob Goblin,
all level 7. His gold stayed 0:
- Infection's kills bring none;
- the boxes so far gave spell charms (Meteor Swarm, The Death, The
  Hanged Man), a Fortune Wire and Fairy's Orbs.

The charms cast spells for no SP, and sell for half their 200.

**Checked.** The piney-game suite, clippy and fmt pass.

**Still unknown:** nothing. Next are the charms used in fights and sold
in town, a lone Kite's grinding before his fights, and then the chain's
next links.
