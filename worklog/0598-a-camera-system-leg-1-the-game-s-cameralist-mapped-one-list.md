---
number: 598
title: A camera system, leg 1: the game's cameraList mapped, one list with CamId (0-3) for every reader, the event camera without the clone
date: 2026-11-14
area: engine, volumes, test
files: crates/piney-battle/src/camera.rs, crates/piney-world/src/camera/mod.rs, crates/piney-world/src/camera/evcam.rs, crates/piney-world/src/world.rs, crates/piney-world/examples/shake_probe.rs
started: 2026-11-14T16:56:05-07:00
took: 2h 40m
---

# 598. A camera system, leg 1: the game's cameraList mapped, one list with CamId (0-3) for every reader, the event camera without the clone

Leg 1 of the camera relay (plans/rust-cleanup.md section 2, "A camera
system"; branch `camera-system`, handoff in the relay's scratch): the game's
camera system mapped against the port, the design settled from it, and its
first two steps done.

## What the game keeps

`cameraList[4]` (INF main 0x002fb6f0, MUT 0x00310f70, OUT 0x003086a0, QUA
0x001fc620) holds `tcam`, `tcam`, `bcam`, `ecam` on every volume (the words
read from each executable), each a 0x70-byte `CAMERA` (DWARF: pos, view, rot,
rot2, rot3, cptr, dist, deg[2], type, resetFlag, resetDirc), and `camID`
(0x0037897c, a short) names the active one, `activeCamPtr` (0x0037896c)
following it. The relocations settle who touches them: `bcam` and `ecam`
only `ccThCamera`'s set-up (0x00160610, copies of tcam) and the list's own
words; `tcam` by name only camera.cpp's field driver (`cameraInit`,
`cameraMain`, `cameraSetManual`, `cameraSetEyeLevel`). Everyone else goes
through camera.cpp's accessors with a number (`cameraSet/GetPos`, `View`,
`Rot`, `cameraGetRot2` 0x00161700, `changeCamera` 0x001617f0,
`checkCameraID`, `checkCameraType`) or reads `activeCamPtr`. So the list is
already the one place a camera lives; the drivers write through it:
camera.cpp for tcam (ccThCamera, priority 40; Kite's `CameraPosSet` and
`cameraSet` at 49), eventMng's `ccEvCamCtrl` (`ccThCameraExecute`, 33),
`ccMenuCtrl`'s shop and fountain views (ecam), the NPCs' (`dogAction`,
`ccPGuso`, 64), and `ccBossCam`, owned by the boss (`ccBoss` +0x14c,
bossCamSW +0x150; `InitBossCamera` gcmn 0x0045e750 makes it and does
`changeCamera(2)`; `~ccBoss` deletes it, whose destructor does
`changeCamera(1)`, but `ccThBoss01` never deletes its boss, so that comes
only as the area goes). The readers: culling (`ccCheckCameraDeg`), the
characters' fade (`ccGetCameraTransparency`), the shake range, the effects'
billboards, the towns' and statics' draws, the lens flare, rain and haze
(camera 1 by number), the sound's panning, the minimap's heading.

## camID is 0-3, not 1-3

All 71 `changeCamera` calls (main and gcmn) pass 1, 2 or 3 or restore a camID
read earlier. One of those restores is `camera_end_reset`'s eventMng
+0x7b4, which only `ccEvent::MenuBan` (main 0x001b248c) writes; `ccEvent::Init`
(0x001a6ca0) leaves it and `_ccMalloc` does not clear, so before any
`menu_ban` it is whatever the heap held, 0 in the harness. A scan of every
script on the four volumes (`tools/evscript.py dump`) found 321
`camera_end_reset`s, each after a `menu_ban` in its block except event 312's
block 17 (OUT and QUA), which follows block 15's `menu_ban` in the same scene.
So no script reaches camID 0, but test_evcam_rs's random commands do, and the
game then runs on `cameraList[0]` (tcam, with `checkCameraID()` not 1).
`CamId` therefore has four values, `Zero` among them, and a first draft with
three failed test_evcam_rs at its first comparison (normalCamID port 1, game 0).

## The design

The list (`piney_battle::camera::CamList`: the three CAMERAs, `CamId`, the
volume for `cameraGetRot`'s square root) lives in piney-battle so the bosses
can hold it; the world's `Camera` (camera.cpp) keeps the field driver, the
shake and the one ccCam around it. Each driver stays with its game owner:
the event camera with eventMng (`World.evcam`), the boss camera moves to the
boss in piney-battle, the menus' views stay functions writing ecam. Readers
ask the list live by the game's call. The rest (the maps' `MapView`, the
bosses' `CamView` snapshot and deferred camera `Out`s, the Grunty's camera
events, the cinema leaving camera/) is laid out as numbered steps in the
handoff.

## Done in this leg

- The list and `CamId` (9075f16): `cameraGetRot2`'s other-camera branch
  written (it was only ever read for camera 1), every reader and writer by
  `CamId` (the bosses' camera `Out`s, `KiteWorld`/ride/effect
  `checkCameraID`, the menus' `TalkReq::Camera`, the probes printing `raw()`).
  `TalkReq::Camera`'s doc named changeCamera at 0x00161ff0, which is no
  volume's; INF's is 0x001617f0.
- The event camera on the Camera in place (86e85d0): its scene borrows only
  what `ccEvent::CamCtrl` looks up (`Finder`, `marker_dummy_of`, `Kite::of`),
  so the take-and-clone of eventMng.cam and the whole Camera twice a frame
  is gone.

Behaviour is unchanged by both; the checks below hold it.

## Checks

On the tip (code at 1c05d92), 2026-11-14:

- harness, four volumes each: test_evcam_rs, test_bosscam_rs,
  test_merchcam_rs, test_evchar_rs, test_town02_rs, test_town03_rs,
  test_race_rs, test_grunty_rs, test_field_rt, test_dungeon_rt, test_map_rs,
  test_world_rs, test_lights_rs, test_evarea_rs, test_area_rs,
  test_battle_event_rs, test_battle_kite_rs, test_gate_out_rs,
  test_party_rs, test_ride_rs, test_effect_hit_rs, test_camera_shake_rs,
  test_field_ambient_rs: 92 of 92 after the fix below (90 before it).
- golden shots against the base shot at b1a9d90 the same day: 4550 files,
  0 differ.
- battle pins: 290 calls, 0 differ.
- crate tests 42 ok, 0 failed; piney-game suite (uncapped) 348 passed, 0
  failed, 152 ignored; gen --check, clippy, fmt, cairns and docs clean.

test_camera_shake_rs failed on Outbreak and Quarantine at frame 248 by one
ulp in `cameraSet`'s shaken view. Its probe (`shake_probe`, the same file on
main) always passed Infection's volume to `Shake::apply`, whose square root
truncates from Outbreak on; the probe now takes the volume's number from the
harness (1c05d92), and all four volumes pass.

A side finding for the relays: the NPC relay's pin script replayed the pins
against another worktree's `battle_probe` (`targets/agent-a48c...`), so its
"0 differ" did not test its branch. This leg's copy runs this worktree's
probe; on main b1a9d90 it gives 290 calls, 0 differ, so main is clean.

**Still unknown:** whether the sound's listener, built from the active camera after the frame, ever differs from calcPan's read at the call (an NPC's camera change at priority 64 after a sound called earlier in the frame); assigned to the relay's step 5
