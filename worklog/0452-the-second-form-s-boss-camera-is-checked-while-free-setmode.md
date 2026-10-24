---
number: 452
title: The second form's boss camera is checked while free: SetMode(5) is the form's own, and the star dust's, press's and root drain's eye and view writes agree with the game
date: 2026-10-24
area: battle, test
files: tools/test_battle_corbenik_b_rs.py, crates/piney-battle/examples/battle_probe/corbenik.rs, crates/piney-battle/examples/battle_probe/corbenik_b.rs, docs/engine/boss-corbenik.md, UNKNOWNS.md
---

# 452. The second form's boss camera is checked while free: SetMode(5) is the form's own, and the star dust's, press's and root drain's eye and view writes agree with the game

[[451]] left one thing open. While the boss camera's `ResetFlg` (+0x04) is 5, the second form writes its eye and view straight:

- the star dust follows its core with the view;
- the press sets the eye at frame 120;
- the root drain raises the eye a little each frame.

The harness never compared those writes. Its camera was a zeroed `ccBossCam`, and its `SetMode` hook only noted the call, so `ResetFlg` stayed 0.

## What puts the camera in that state

Nothing outside the fight does. `ccBossCam::SetMode` (QUA gcmn 0x00363d70) stores its mode in `ResetFlg` before anything else. Mode 5 then:

- locks the eye;
- sets `ExLock` bit 0;
- keeps the view, eye and turn at +0x40-+0x60.

Mode 6 puts them back on the camera and on camera 2, then sets `ResetFlg` to 0. `SetFreeCamPosView` (0x00363f20) writes only while `ResetFlg` is 5. So the form frees its own camera: the star dust, press, root drain, sybly, barrier and death acts each call `SetMode(5)` first, and `SetMode(6)` when they are done.

## The harness

`tools/test_battle_corbenik_b_rs.py` now does to the camera what the game's two functions do:

- `SetMode` writes the mode, the `ExLock` bits, the lock and the kept vectors. Mode 6 sets camera 2 silently, since the port has it as one `CamMode 6`.
- `SetFreeCamPosView` writes only while the mode is 5.

Each frame the harness compares the camera's lock, `ResetFlg`, `ExLock`, view, eye and kept ones. battle_probe's `Cams` keeps the same camera from the port's `CamMode`, `FreeCam`, `CamEye`, `CamLook` and `CamLock`, and hands its `ResetFlg`, eye and view to the form through `CamView`.

36 cases (seeds 10100-10129 and the first six again) agree. The camera was free in the star dust, press, root drain, sybly, barrier and death acts. Frames where the straight writes moved it: about 200 in each star dust, one in each press, 100-570 in each root drain.

**Still unknown:** whether the world's battle camera honours `CamLook` and `CamEye` while free. That is the world's side of Corbenik, not ported yet.
