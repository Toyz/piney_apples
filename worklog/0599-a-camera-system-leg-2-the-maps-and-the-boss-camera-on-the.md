---
number: 599
title: A camera system, leg 2: the maps and the boss camera on the list, the haze's camID turn, the listener settled
date: 2026-11-14
area: engine, world, battle, audio, test
files: crates/piney-world/src/systems/mod.rs, crates/piney-world/src/sky/lensflare.rs, crates/piney-world/src/systems/haze.rs, crates/piney-battle/src/boss/cam.rs, crates/piney-battle/src/camera.rs, crates/piney-world/src/combat/boss.rs, tools/test_field_ambient_rs.py
started: 2026-11-14T19:38:16-07:00
took: 2h 20m
---

# 599. A camera system, leg 2: the maps and the boss camera on the list, the haze's camID turn, the listener settled

Leg 2 of the camera relay (plans/rust-cleanup.md section 2, "A camera
system"; branch `camera-system`, after worklog 598). It did steps 3 and 4 of
the relay's plan (commit 73f8203) and settled the sound listener's question
from the game's code. Steps 5 to 8 are left.

## The maps ask the list (step 3)

A map's `Draw` used to read a per-frame snapshot, `MapView`, with its own
copies of the cameras (`eye`, a clone of the active `CAMERA`, the lens
flare's `FlareCamera`, `eye1`, `rot`, `rot2`). Now `MapView<'a>` lends the
list (`cams: &CamList`) and the ccCam's `world_screen`. Each reader asks the
list by the game's own call:

- `cameraGetPos(camID)`: the statics, Mac Anu, the water and `WORLD::DrawEffect`.
- `activeCamPtr`: `ccCheckCameraDeg` for the clouds, the disc's rocks and the flare.
- camera 1: the rain, the fireflies and LENSFLARE's turn, which is
  `cameraGetRot2(1)` in the eye view and `cameraGetRot(1)` otherwise
  (`camera::field_turn`).

Each area's `disp` makes the view from the camera as it stands. A story map
whose scene moves the camera is therefore drawn from the moved camera, as the
game draws it: `EVENTAREA03::Draw` (gcmn 0x00403bf0) does `changeCamera`,
`cameraSetView/Pos/Rot`, then `STATICMODEL::Draw`.

**WORLD::Draw's heat haze.** The haze turns by `cameraGetRot2(1)` in the eye
view, as the others do. Otherwise it turns by `cameraGetRot(r,
checkCameraID())`, the active camera's (INF gcmn 0x005a9990, MUT 0x005d4f40,
OUT 0x005d2830, QUA 0x004c5290). The port took camera 1's there. This is a
real difference, now fixed (`MapView::draw_turn`).

The harness had hidden it. test_field_ambient_rs hooked the camera
accessors: camID was 0 with a fake `activeCamPtr`, and `checkCameraID()`
answered 1. The cameraList cannot hold that state. The harness now writes
tcam and an active ecam (camID 3) into memory and lets camera.cpp's own
accessors run. The first run of that fixed harness failed on "camera 3
asked", which is how the haze's read was found.

## The boss camera on the boss (step 4)

`ccBossCam` is now `piney_battle::boss::cam::BossCam`, held on the boss:
`BossBase.cam` is `ccBoss` +0x14c `bossCam`, and `cam_sw` is +0x150
`bossCamSW`. `OffBossCamera` (0x0045ede0) is a function over the list.
camera.cpp's `Scheme`, `checkCameraDistModValue` (0x00162fb0) and `CamPad`
moved to piney-battle's camera module. piney-battle now depends on
piney-input.

The maths run on `piney_battle::geom`. Its matrix routines were checked op
for op against evcam's `vu_cossin`/`rot_x`/`rot_z`: the same polynomial
order, the same sign handling, the same row products. `BossRun`'s `cam` and
`cam_sw` are gone. The world still calls `CamMain` after the rules' frame and
applies the camera `Out`s; step 5 moves both into the boss's `Main`.

## The listener

`ccSeOn3D` (INF main 0x00179d90, MUT 0x0017c490, OUT 0x0017c120, QUA
0x0017c080) works out the pan at the call: it calls `calcVel` and `calcPan`
(INF 0x0017a4c0, MUT 0x0017cbe0, OUT 0x0017c8a0, QUA 0x0017c800).
`ccSeOn3DNote`, `ccSeOn3DLoop` and `tobjSeLoop` do the same. `calcPan`
reads `activeCamPtr` (INF 0x0037896c, MUT 0x0038b8ec, OUT 0x00386c74, QUA
0x0027957c) and `checkCameraType` then, and the result goes straight to the
driver (`sceMSIn_PutMsg`).

Cameras do move between a sound and the frame's end. In every boss fight
frame, `ccBossCam::CamMain` (priority 66; INF gcmn 0x0045fbe0, MUT
0x004755a0, OUT 0x00470f00, QUA 0x00363810) writes bcam after sounds made
earlier in the frame:

- Kite's footsteps at 49: ccPlayer::Main runs CameraPosSet (INF 0x00598858,
  MUT 0x005c39c8, OUT 0x005c0e28, QUA 0x004b3888), then AnimCtrl and Draw;
  the anm's notes go through `ccPlayer::CheckNote` (gcmn 0x0059c508) to
  `ccSeSetParamSPC` and `ccSeOn3D`.
- The party's sounds (`ccFellow::CheckNote`) and the enemies' (`ccSeSetParamEnemy`).
- The NPC list's writes at 64 come after those too.

The port builds one listener after the frame. Its boss-fight sounds made
before 66 are therefore panned by this frame's `CamMain`, where the game's
are panned by the last frame's. Main has this difference too.

The fix is for each 3D sound request to carry the listener read from the
list at its call. It is assigned to steps 5 and 6, where the bosses and the
NPC list get the list (the relay's handoff has the plan and a harness for
it).

## Checks

On 73f8203, 2026-11-14:

- harness, four volumes each: test_field_ambient_rs, test_evcam_rs,
  test_bosscam_rs, test_merchcam_rs, test_evchar_rs, test_town02_rs,
  test_town03_rs, test_race_rs, test_grunty_rs, test_field_rt,
  test_dungeon_rt, test_map_rs, test_world_rs, test_lights_rs,
  test_evarea_rs, test_area_rs, test_battle_event_rs,
  test_battle_kite_rs, test_gate_out_rs, test_party_rs, test_ride_rs,
  test_effect_hit_rs, test_camera_shake_rs: 92 of 92.
- golden shots against the base shot at b1a9d90, made the same day: 4550
  files, 0 differ. No shot is of a heat-haze field under a camera other
  than 1.
- battle pins, using this worktree's probe at 73f8203: 290 calls, 0 differ.
- crate tests 42 ok, 0 failed.
- piney-game suite, uncapped: 348 passed, 0 failed, 152 ignored.
- gen --check, clippy, fmt, cairns and docs are clean.

A first step-3 harness run had 25 errors. I edited the tree while it ran,
and the harnesses rebuild their probes themselves, so they built half-edited
code. None of those errors came from the change itself.

**Still unknown:** how often the listener's one-frame lag in a boss fight changes a pan's value as sent to the driver; the step-5 harness (a footstep's calcPan against bcam before and after CamMain) measures it as the fix lands
