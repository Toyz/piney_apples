---
number: 451
title: Corbenik's second form (ccBoss08_b), its roots, its barrier and its effects are ported from Quarantine's code and agree with it frame by frame
date: 2026-10-24
area: battle, render, test
files: crates/piney-battle/src/boss/corbenik/second.rs, crates/piney-battle/src/boss/corbenik/stardust.rs, crates/piney-battle/src/boss/corbenik/eff.rs, crates/piney-battle/src/boss/corbenik.rs, crates/piney-battle/src/boss/corbenik/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/blur.rs, crates/piney-battle/examples/battle_probe/corbenik_b.rs, crates/piney-battle/examples/battle_probe/corbenik.rs, crates/piney-gen/src/manifest.rs, tools/test_battle_corbenik_b_rs.py, docs/engine/boss-corbenik.md
started: 2026-10-24T08:12:34-06:00
took: 30m
---

# 451. Corbenik's second form (ccBoss08_b), its roots, its barrier and its effects are ported from Quarantine's code and agree with it frame by frame

When Corbenik's first form exits, `ccThBoss08` flashes, deletes it and
plays the second form in the same task. The second form is `ccBoss08_b`
(QUA gcmn 0x003b1c50-0x003b9030), with five roots (`ccBoss08Root`). It
is now ported in `crates/piney-battle/src/boss/corbenik/second.rs`. Its
effects are ported too, because they draw `ccRand`, decide when the form
moves on, or both:

- the transform (`ccBossEffTransform`);
- the root drain (`ccBossEffDrain`);
- the star dust (`ccBossEffStarDustFlash`), with its bolts
  (`ccBossEffThunder3`) and its laser rain (`ccBossEffLaserRain`), in
  `stardust.rs`.

docs/engine/boss-corbenik.md has the form.

## What it does

The body is `bossTbl` row 8. Its acts are:

- a wave;
- a sybly of skill 157 on the whole party;
- a magic of skill 244;
- a press: sixteen images of itself circle the target and sixteen
  elements are shot across it;
- a root drain: five roots come up between it and its farthest member,
  then boss skill 57 drains HP and SP;
- from half its HP, the star dust.

At a fifth of its HP it raises a barrier: the transform, then `bossTbl`
row 48's stats.

Its HP never kills it. Blows on the barrier take the barrier's own
count, which starts at the HP the body had when the barrier was asked
for. When that count runs out, the form dies. The first time it falls
under 4000, menu 74 shows stream 94, and 3630 frames later the form dies
anyway.

What its code does that one would not guess:

- The star dust's `SelectTarget` takes the 3 its step's `switch` left in
  `$a1`. So the star dust falls on the member with the least HP, not on
  the nearest one.
- The root drain's damage runs with the boss's HP and SP drains at 100.
  The drains' `EntryAffect`s on the boss itself (a heal of half the blow)
  happen at once, inside the damage. The port takes them out of the
  frame's events and runs them through the form's `Affect`.
- Quarantine's class has three words that Infection's DWARF lacks
  (+0x294b4-+0x294bc): the message's wait, whether it was shown, and
  whether the wait ran out.
- What the root drain keeps as its circling radius and angle are
  `ccBoss::m_actDirc`'s x and z (+0x1c0, +0x1c8). The word after
  `actPos` is not a member of its own.
- The laser rain's lasers start within 2000 of the target, 1000 up, and
  end within 1000 on the ground. Each trails sixteen segments 100 apart
  back up its way, and they land one after another.
- `ccBossEffDrain` ends when its last particle generator is gone.
  Particles alone decide that, so the rules ask the runtime through
  `Cx::gen_alive` for each generator they keep (`Pic::Gen`'s key).

The blur gains `boss08BBlurParamCB`. It is Boss03's on its own static,
and the press and the root drain set it and clear it (`Set::NoFunc`).

## Checked against the game

`tools/test_battle_corbenik_b_rs.py` runs `ccBoss08_b` natively:

- The WaveShock and the rings are stand-ins.
- The star dust (with its bolts and rain), the transform and the drain
  run natively.
- A particle generator lives a case's number of frames.
- Scripted blows, the drain and the boss's HP set to its thresholds drive
  each case.

With the final code, 42 cases (121,094 frames) agree. They reach acts 0,
3, 4, 5, 6, 7, 14, 16, 18, 20, 21, 22, 23 and 24. The first form's
harness passes 8 more cases, its seeds off the land in 75 seed-frames.

Three unit tests cover the second form:

- the roots start put away;
- half its HP brings the super table and a fifth asks for the barrier;
- the barrier takes the blows, shows its message and its count kills it.

The harness found three port bugs before they shipped: the star dust's
target, the drains' order, and the root drain's radius word.

**Still unknown:**
- The third form (`ccBoss08_c`, its eye and killer eyes) is not ported
  yet.
- The world's side of both forms is not ported yet: the seeds, the
  roots, the elements and the images as actors; the barrier, the star
  dust's stars, bolts and lasers, and the transform's lays as pictures;
  and the two forms in one task.
- What the camera does in the star dust and the press while it is free
  (`ResetFlg` 5) is not checked against the game. The harness's camera
  stays at 0.
