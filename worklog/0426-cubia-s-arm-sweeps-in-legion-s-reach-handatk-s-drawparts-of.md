---
number: 426
title: Cubia's arm sweeps in Legion's Reach: HandAtk's DrawParts of anmw and its EntryAfterImageAnm drawn as the game's ccBossAfterImageEx, checked against the game's calls on both fights (#72)
date: 2026-10-22
area: battle, render, test, tooling
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session/tests/kyvia_fx.rs, tools/test_battle_kyvia_rs.py, docs/engine/boss-kyvia.md
started: 2026-10-22T16:14:34-06:00
took: 20m
---

# 426. Cubia's arm sweeps in Legion's Reach: HandAtk's DrawParts of anmw and its EntryAfterImageAnm drawn as the game's ccBossAfterImageEx, checked against the game's calls on both fights (#72)

Issue #72's last point: one of Cubia's attacks showed none of its
tendrils. That is Legion's Reach (`HandAtk`, act 3): the arm that sweeps
across the disc was never drawn, an open question since [[272]]
(boss-kyvia.md, Unknown).

## What the game does

`HandAtk` (MUT gcmn 0x004d5930, OUT 0x004d8c70) sets the arm's anm
(`anmw`, `ANM_ex0batc0` of x01 or x02: `OBJ_trall1`, `OBJ_ex0bbod1`/`2`
and their morphers) at `tesPOS`, `tesDIRC` once (process 4,
`SetMatrix_PosRotZYX`). Each frame of the sweep (process 5):

- every `AFtiming`-th frame, `EntryAfterImageAnm(anmw, tesPOS, tesDIRC,
  &t)` (0x004d5d70), `t` the body's +0x29620 (the second fight's
  +0x296c0), set by both constructors alike: colour 0x00525221 at
  `colorAlpha` 30, `colorSpd` 0, `maxFrame` 80, transparency 0.5 by
  `transSpd` -0.02, `ss` and `es` (0, 0, 0, 1), `scaleTime` 0;
- `_AnimateForward`, then `DrawParts(anmw, 0)` (vtable +0x30,
  `ccBoss::DrawParts` 0x00473350): objLayer, no blend, `ccAnm::Draw`.

`ccBossAfterImageEx::Init` (0x004710b0) copies the anm's clip at its
frame (`_AnimateFrame(frame)`; at frame 0 one `_AnimateForward`), the
parameters, and the scale (`ss`, or 1 with no `scaleTime`). `Draw`
(0x00471320) moves `colorAlpha` by `colorSpd` (held 0-100), the
transparency by `transSpd` (dead at 0 or 1), the scale toward `es`,
then draws under `SetFogBlend(colorAlpha, color)` (none for colour -1).
`maxFrame` is counted down and never read.

The port had `HandAtk`'s `DrawArm` output and nothing drawing it, and
sent the after-image as `Out::AfterImage`, an after-image of the body's
pose.

## The port

- piney-battle: `AfterImageParams` (`ENTRYAFTERIMAGE_T`); `HandAtk`
  sends `AfterImageAnm { frame, pos, dirc, params }` with
  `kyvia::ARM_AFTERIMAGE`, and `DrawArm` carries the arm's pose time.
- piney-world: `AfterImage` keeps what it poses (`Image::Boss`, `Leaf`,
  `Anm`) and its blend as a rate, colour and speed
  (`AfterImage::anm` from the parameters; the boss's and the leaf's as
  before); `BossRun::arm`; `draw_boss` draws the arm and its
  after-images as `ccAnm` draws of the body's file (lit, the body file's
  morphers in `BossLook::body_morphers`).

## Checks

- `tools/test_battle_kyvia_rs.py` now records the game's `DrawParts`
  (the anm's frame and flag) and `EntryAfterImageAnm` (frame, place,
  turn and every word of `t`) and compares them with the probe's: 200
  Mutation cases (273,768 frames), 120 of Outbreak's second fight
  (363,530) and 40 of its first (49,358) agree, act 3 reached in each run,
  and the unittest's six.
- `session::tests::kyvia_fx::kyvias_arm_sweeps_with_its_after_images`:
  over 6,000 frames of each fight the arm's `MDL_ex0bbod1` is drawn in
  564 (x01) and 651 (x02) frames of act 3, with after-images in 540 and
  624. Before, in none.

Shots: /mnt/data/claude/scratch/cubia/acts/kyvia-00472-act3.png and
kyvia-00480-act3.png (the arm across the disc under "Legion's Reach").

**Still unknown:** nothing. The gomoras' pushes and the pad's sway stay open under 272.
