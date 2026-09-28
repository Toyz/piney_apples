---
number: 59
title: Into The World: Log in lands in Mac Anu, and Kite walks under pad control
date: 2026-09-23
area: world, engine, test
files: crates/piney-world, crates/piney-data/src/libm.rs, crates/piney-data/src/statics/mod.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, tools/test_world_rs.py, docs/engine/field-game.md
---

# 59. Into The World: Log in lands in Mac Anu, and Kite walks under pad control

After [[58]] the top page's Log in asked for the field game (mode 5,
`ccSetupNewGame` with `GCMN.PRG`, then 6, `ccSetupGameCtrl`), which was not
ported. A helper agent traced that path to Kite standing in the first Root
Town and ported what it takes to walk there, as `piney-world`. I put it in
the runtime. The reference is
[entering The World](../docs/engine/field-game.md).

## What the game does

- **The hand-off.** Log in leaves area 0 (a Root Town) and
  `saveData.lastTown` in `ccGame`.
- **`ccSetupGameCtrl`.** It fades out for 10 frames and holds black for 2
  while the town loads: its sequence bank, `ccSndSQLoad(2)`. It then starts
  the field's tasks at 30 frames a second and fades in over 10 frames.
- **The town.** Town 0, Mac Anu, is `town01`, or `town01d` once the save's
  crisis flag is set.
- **Kite.**
  - He arrives at the new-game spot by the Chaos Gate.
  - His speed, height and width come from `spcParam[0]`, which
    `ccSaveData::NewGame` fills.
  - Before the Data Drain bracelet (`plcol` 0), his body takes a plain
    palette.
- **Each frame.** The camera task (`cameraMain`, priority 40) runs, then
  `ccPlayer::Main` (49):
  - the pad becomes a move vector and turning;
  - the act picks idle, walk or run, with the walking lean and idle
    fidgets;
  - he lands on the town's floors and is stopped by its walls (828
    polygons: 301 floors, 527 walls);
  - the camera follows in the scheme `camType` picks (A-1, A-2, B-1,
    B-2), with an eye view and pressure turning and zoom.

## The port

- **`piney-world`.** `World::enter` on the save and `step` each frame,
  about 3,300 lines:
  - the town from its placement tables;
  - Kite's clump, pose and animation;
  - movement, collision and the camera.

  Every float step goes through the EE's own arithmetic.
- **`piney-data::libm`.** newlib's `sinf`, `cosf`, `atanf` and `atan2f`,
  bit-exact to the game's.
- **Moved into `piney-data`.** The town's placement code
  (`ModelTable::place`, `Position::find`, `StaticObj::root`) moved there
  from the viewer, which now calls it. `docs/engine/statics.md` gained
  corrections from the town's draw code.
- **`piney-input`.** It carries a DS2's button pressure (`Pad.pow`). A pad
  that reports none reads 0, and the camera then turns at its fixed rate.
- **The runtime.** Log in now enters `WorldMode` (frame rate 2), with a
  fall back to the desktop if the town cannot be entered. The save is the
  one New Game made, as the game's is. The event task is carried but does
  not run in the field yet.

## Checked

- **Against the game's code.** `tools/test_world_rs.py` runs the game's own
  `gcmn.prg` and main code in eemu beside the port and compares every
  frame bit for bit: position, rotation, move vector, speeds, flags, act
  and animation state, transparency, the ground's attribute, and the
  camera's position, view, rotations and distance with its matrices.
  - The math functions: 9,000 cases.
  - Control, movement and landing.
  - Scripted frames: walk, run, turn, idle fidget, the buttons in all four
    camera schemes, the eye view and pressure.

  I re-ran these four (580 s) in a clean worktree and they pass. The
  random runs (2 towns x 4 runs x 420 frames) passed in the agent's
  worktree; I did not re-run them.
- **The crates.** `piney-world`'s tests cover the collision mesh, the
  arrival and walking (the arrival ends on frame 74, and Kite runs down
  the stairs and stops on release) and walls holding him.
- **The runtime.** `the_world_top_page_and_into_mac_anu` goes from a new
  game's desktop to the top page, out and back, and Log in: The World at
  30 frames a second, the town drawn, and Up on the D-pad moves Kite. The
  workspace's tests pass, and clippy and fmt are clean.
- **Shots.** GPU shots show Kite at the Chaos Gate plaza facing the bridge
  and the bell tower, and after walking and turning.

**Still unknown:**
- **Not yet in the field:**
  - the event task (event 2's arrival scene with Orca);
  - NPCs and the Chaos Gate;
  - the HUD, map and field menus;
  - leaving the town.
- **Sound.** The town's bank and `ccSndBgmCtrl`'s town case are not
  traced, so the town is silent. Footsteps and ambient loops are not
  ported either.
- **Not drawn:**
  - the town's fog, which needs fog from depth in `ModelDraw`;
  - the water, textured from the frame buffer at screen-space UVs;
  - Kite's shadow;
  - the arrival's effect.
- **Other towns.** Only Mac Anu. The crisis town's drawing is not checked
  by eye.
- **Randomness.** `rand` is not seeded from the game's shared state, so
  idle fidgets can come at other times than in a real run.
