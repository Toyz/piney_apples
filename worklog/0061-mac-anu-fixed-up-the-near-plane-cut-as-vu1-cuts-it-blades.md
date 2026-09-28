---
number: 61
title: Mac Anu fixed up: the near plane cut as VU1 cuts it, blades in Kite's hands, the town's lights, and the Chaos Gate
date: 2026-09-23
area: world, render, test
files: crates/piney-gs/src/convert.rs, crates/piney-world/src/camera.rs, crates/piney-world/src/chara.rs, crates/piney-world/src/town.rs, crates/piney-world/src/gate.rs, crates/piney-data/src/libm.rs, tools/test_world_rs.py
---

# 61. Mac Anu fixed up: the near plane cut as VU1 cuts it, blades in Kite's hands, the town's lights, and the Chaos Gate

After [[59]] the user walked around Mac Anu and reported three problems:
- the camera clipped through floors;
- the weapons were not in Kite's hands;
- props were missing, "like the save build room".

The world agent traced each one to the game's code. Its two helpers took
the weapons and the props.

## The camera: a drawing bug, not a camera bug

- **The camera matched.** In a town, `camera.cpp`'s floor handling already
  matched the game. `cameraPosCalc` (0x00161870) pulls the camera to 10
  units short of the first bit-2 polygon on the line from Kite's head, and
  every one of Mac Anu's 828 hit polygons has bit 2.
- **The drawing did not.** VU1's `mc_DrawTriSFast` cuts an unlit
  triangle that is not wholly in view, when its last strip vertex has w
  below `divZ` (1000), at the near plane w = 8
  (`mc_ScissorTriPolyXYZ`). The port dropped such triangles whole, so the
  floors and walls the camera was pulled against lost their nearest
  triangles.
- **The fix.** `piney-gs` now cuts them the same way, interpolating UV and
  colour, for models that do not ask to be rejected whole. The town's
  unlit pieces take that path.
- **Fields too.** `avoidObstacle` (0x00162430), which keeps the camera 50
  above a field's ground, is ported for later. It needed newlib's `tanf`
  and `fmodf`, added bit-exact to `piney-data::libm`.

## The blades

- **What the game does.** `ccPlayer::ccPlayer` (gcmn 0x00597910) finds the
  hands with `ccClump::GetObjAdrsF`, which searches only the clump's own
  nodes. `ccSpcChar::EquipWeapon` (0x0059d650) then sets each blade model
  on its hand node with no offset.
- **The bug.** The body file also holds an unposed copy of each hand name
  per animation. The port's name lookup found that copy first, whose
  matrix falls back to the root: the blade at his feet.
- **The fix.** The port now looks among the clump's nodes, as the game
  does.

## The props

- **The town draw.** `ROOTTOWN01::Draw` run in eemu, over 953 camera eyes
  in `town01` and 713 in `town01d`, selects exactly what the port selects.
  The branch that looked wrong is the game's own dead code.
- **Two real bugs fixed on the way:**
  - the five omni lights were misread (intensity 600, a garbage fall-off);
    they are intensity 1, fading from 600 to 800, so Kite is now shaded as
    the game shades him;
  - the crisis town's sky overlays were not drawn.
- **The missing prop: the Chaos Gate.** It is drawn by the entry control,
  not by the town (`ccSetChaosGate` gcmn 0x00458df0, `ccChgate`):
  - its ring (`CMP_xmgtwav0`, `ANM_xmgtcir1`, a 91-frame loop) at the gate
    dummy, within 4,500 of Kite;
  - its magic-circle state machine with sounds 71 and 72;
  - its light added to the town's.
- **"The save build room".** This is the Recorder's booth, which is town
  geometry and was already drawn. The Recorder standing in it is one of
  the five merchants, which come with the NPCs.

## Checked

- **Against the game's code.** `tools/test_world_rs.py`, 12 tests, 0
  mismatches:
  - `test_avoid_obstacle`: 1,500 cases;
  - `test_frames_floors`: the camera pressed into six stairs and slopes,
    480 frames each, frame by frame;
  - `test_weapon`: 200 frames. The game's `EquipWeapon`, `ccObj::Draw` and
    `_SetLWMatrix` agree to 1.5e-5 per rotation element and 0.0034 in
    position.
  - `test_town_draw`: the draw selection over the camera eyes above;
  - `test_gate`: 800 frames of `ccChgate`;
  - the earlier tests, and `tanf` and `fmodf` added to the math checks.

  I re-ran the weapon, gate, town-draw and obstacle tests in a clean
  worktree (253 s); they pass.
- **The rest.** The workspace's tests pass, and clippy and fmt are clean.
- **Shots.** Kite on the bridge stairs, both blades in his hands and the
  steps drawn to the bottom of the screen. A wall beside the camera that
  was a hole is now drawn.

**Still unknown:**
- **Not yet ported:**
  - the merchants, the Recorder among them, and the town's walking PCs and
    Orca;
  - the gate's menu, which opens the circle;
  - water, fog, shadows and the arrival's effect.
- **Not checked against the game:**
  - the near-plane rule was read from the VU1 microcode, not run;
  - the order in which the gate's light joins the town's.
- **The CPU renderer.** It still drops triangles behind the camera
  instead of cutting them.
- **The user's movement stalls.** With clean input the port runs steadily,
  even with the stick held north-west while the camera turns, so the stall
  is looked for in the DualSense's input.
