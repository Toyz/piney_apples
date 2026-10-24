---
number: 458
title: Corbenik's seeds draw twice a frame, its lasers in their own blend, its first press image at the body's last transparency, as the code does
date: 2026-10-24
area: render
files: crates/piney-world/src/combat/boss.rs crates/piney-world/src/combat/corbenik.rs crates/piney-world/src/field_world.rs crates/piney-world/src/town.rs crates/piney-battle/src/boss/corbenik/second.rs docs/engine/boss-corbenik.md
started: 2026-10-24T12:29:07-06:00
took: 22m
---

# 458. Corbenik's seeds draw twice a frame, its lasers in their own blend, its first press image at the body's last transparency, as the code does

## Three of Corbenik's pictures as the code draws them

Worklog 457 left three of Corbenik's pictures short of the game's code.

**The seed's second draw.**
- `ccBoss08Seed::PreDrawAnm` (QUA gcmn 0x003b09f0) sets its clip's matrix
  (50 up, 3x) and calls `ccAnm::Draw`. Then its `ccBoss::Draw` draws the
  clip again.
- `ccAnm::Draw` hands the anm's +0x88 to `ccObj::Draw`. `ccBoss::Draw`
  copies the boss's transparency into the anm's +0x88 every call, before
  its 0.05 test, and `ccCoord`'s constructor starts it at 1. So the first
  draw is at the last `Draw`'s transparency.
- The layer: `ccBoss::Draw` and `DrawAfterImages` both end with
  `SetActiveLayer(0)`, and the seeds run in the first form's `Slave()`
  after them. Case 0 of `SetActiveLayer`'s jump table is sysLayer
  (priority 0, the field layers' shared view, after objLayer).
- The port: `BossRun::anm_alpha` keeps each part's last copy, and
  `Shown::Again` draws the seed's actor once more on the object layer
  before its own draw.

**The lasers' blend.**
- Both effects' constructors (laser rain 0x003795e0, reflected laser
  0x00376f60) set `OBJ_ex31les1`'s model ALPHA to `alphaBlendTbl[1]` and
  call `SetRenderState(1, 0)` on it. They also set the head clip
  `ANM_ex31les3` to `SetBlendType(1)`.
- The field now draws `les1` with blend 1 through `draw::model_blended`,
  and the head clip through the new `town::anim_draw_blended`.

**The first press image.**
- `OnThinkBodyPress` at 0x003b5460 sets the body's matrix to the image
  and draws its clip with no transparency set, so the anm draws at the
  last `ccBoss::Draw`'s. The later images (0x003b598c) set the image's
  own.
- The rules now send the body's transparency for that first image.

**Checked.**
- `corbenik_under_god` still ends event 313 (76,284 frames).
- The second form's harness passes (its pictures are not compared).
- The piney-game suite passes.

**Still unknown:**
- None of the three is compared with game frames; each draw's inputs
  are from the code. The layer constant for sysLayer's place is
  approximated by the object layer (same priority, drawn after it).
