---
number: 512
title: Corbenik's Magic shakes the active camera's view, not camera 2's: a Magic picked again starts at step 1 and shakes camera 1, as the game's does
date: 2026-10-29
area: battle, volumes
files: crates/piney-battle/src/boss/corbenik/first.rs, crates/piney-battle/src/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-battle/examples/battle_probe/corbenik.rs, docs/engine/boss-corbenik.md
started: 2026-10-29T21:00:59-06:00
took: 13m
---

# 512. Corbenik's Magic shakes the active camera's view, not camera 2's: a Magic picked again starts at step 1 and shakes camera 1, as the game's does

`test_battle_corbenik_rs.py` failed at case 4, frame 622, at HEAD too
(worklog 508). The game called `cameraSetView(view, 1)`, the port
camera 2. Every frame from 622 to the Magic's end differed the same
way.

**The code.** In `OnThinkMagic` (QUA gcmn 0x003af990), step 0 does
`changeCamera(2)`, then `cameraSetPos` and `cameraSetView` with `camID`
(0x003afc80). The shake in step 2 does `cameraGetView(camID)`, adds the
kick, and `cameraSetView(camID)` (0x003afd98). `OnThinkDie`'s eye move
is the same: `cameraGetPos(camID)`, `cameraSetPos(camID)`
(0x003af748, 0x003af804). The port hard-coded camera 2 for these sets.

**Why camera 1.** The trace of case 4 on both sides:

| frame | act | camera |
| --- | --- | --- |
| 288 | Magic step 2 | camera 2, skill 252 |
| 465 | Magic step 1 | `changeCamera(1)` |
| 526 | Magic step 2 | skill 252, no camera change |
| 622 | step 2, count 96 | the shake begins: game camera 1 |

At 465, the last step does `changeCamera(1)`, then `ChangeNextPattern`
(which zeroes `m_actProccess`), then `m_actProccess++`. The next pattern
was Magic again, so it started at step 1. It never set camera 2, and
`camID` stayed 1. The game shakes camera 1's view, and the port now
does the same.

**The change.** `CamView` gains `active: i32`, holding `camID`:

- the world fills it from `Camera::cam_id`;
- the Corbenik probe fills it from the camera it tracks;
- the other probes name camera 1, or camera 2 for Kyvia's, which nothing
  changes.

Corbenik's shake and its dying eye move set `cx.cam.active`.
`docs/engine/boss-corbenik.md` says so.

**Checks.**
- `test_battle_corbenik_rs`, `_b_rs` and `_c_rs` on Quarantine pass, all
  six cases.
- piney-game's suite passes, and so do piney-battle's and piney-world's
  tests.

**Still unknown:** nothing
