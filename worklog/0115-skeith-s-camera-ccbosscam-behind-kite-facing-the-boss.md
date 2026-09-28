---
number: 115
title: "Skeith's camera: ccBossCam behind Kite, facing the boss"
date: 2026-09-25
area: battle, world, test
files: crates/piney-world/src/bosscam.rs, crates/piney-world/src/camera.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-world/examples/bosscam_probe.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/skeith.rs, tools/test_bosscam_rs.py, docs/engine/boss.md
---

# 115. Skeith's camera: ccBossCam behind Kite, facing the boss

[[113]] left the field's own camera following Kite through the fight. The
game hands the view to the boss's camera, which keeps Skeith in the middle
of the picture.

## ccBossCam

`ccBoss::InitBossCamera(200, 1000)`, at the end of `ccBoss01`'s
constructor, news a `ccBossCam` and takes camera 2 (`bcam`). `ccBoss::Main`
runs its `CamMain` after `Move`. Each frame:
- the eye stands over Kite (`tempChar`, `plw+0x20`), 200 up and
  `MoveTransfer.y` (1000 to 2000) back along the line from the boss;
- it looks level, 1000 ahead;
- the heading turns toward the boss's by a quarter of the difference,
  held to 1280 a frame, twice that after 20 frames of catching up. Within
  128 it is the boss's.

The pad moves `MoveTransfer.y` between its ends:
- schemes A: the right stick;
- schemes B: R1 and R2;
- two buttons ease it to either end.

`QuakeCam` shakes eye and view together for one frame. Its three
`ccRandF(35)` draw from `ccRand` whenever the camera exists. The rules'
harness has none (`InitBossCamera` is a no-op there), so `Cx.boss_cam`
says which: the runtime draws, the harness does not, and the harness still
matches.

`SetMode`, `SetFreeCamPosView` and `SetRotXLimit` belong to other bosses.
Without them the pitch, the extra turn, `ExLock` and `ResetFlg` 3 and 4
never come up for Skeith. They are left out and named in the module.

`BeginDeadEffect` hands camera 2 to camera 1 (as `OffBossCamera`), stops
the boss camera, and puts camera 3 behind the boss looking at it.
`Out::DeadCamera` now carries the boss's place at that moment, and its
`ccSqFade(0, 0, 30, 3)` takes the music out. The boss and its camera live
until the task is deleted at the next mode change, so camera 3 holds
after the fight.

## Checked

- **`tools/test_bosscam_rs.py`.** It builds the game's `ccBossCam` in eemu
  over the field's camera and player tasks (Mac Anu, as the merchant
  camera's test does):
  - the constructor;
  - 120 frames of each control scheme and one 400-frame chase, with the
    boss stepping and sometimes jumping, and `QuakeCam` now and then;
  - `OffBossCamera`, then frames back on camera 1.

  Compared bit for bit after every step:
  - the camera's fields;
  - `camID`, `tcam` and `bcam`;
  - the view matrices;
  - Kite walking by camera 2's heading.

  It reaches both resets, the zoom and a count past 20. Everything
  matches. The one exception is `OffBossCamera`'s rotation y and w: that is
  stack the game never wrote (`cameraGetRot(r, 2)`). It is cleared in the
  game, and the port takes zero.
- **`session::skeith::skeith_fights_in_its_arena`.** It now checks that
  camera 2 is active through the fight, faces the boss (839 of 888
  frames within 0.35 radians), and stands 200 over Kite. The remaining
  frames are the camera catching up after a dash, or a hit or a quake.
- **`event_30_ends_with_skeith`.** Camera 2 is active through the fight
  and camera 3 from the death on, never camera 1.
- The shots show Skeith in the middle of every frame, Kite in front of the
  eye.
- The rules' harness (`test_battle_boss_rs.py`), the Skeith tests, and the
  workspace's tests, clippy and fmt pass.

**Still unknown:**
- **The pictures.** `DrawCross`'s trail, the six effects and the reversed
  layer (`ccBufferReverce`) are still not drawn. The effects read
  `cmn_bossCam` (the Phantom's, for one), which the port will need to hand
  them.
- **`fieldrand`.** Its seed on arrival is still not carried from the
  dungeon ([[112]]).
