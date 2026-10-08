---
number: 404
title: Mutation's town camera is pushed out of walls by another mask: from Mutation on cameraPosCalc asks ccModelHitCheckQZ for bit 2, and the world harness lays ROOTTOWN01 out as each disc does
date: 2026-10-08
area: world, volumes, test
files: crates/piney-world/src/hit.rs, crates/piney-world/src/camera.rs, crates/piney-world/examples/world_probe.rs, tools/test_world_rs.py, docs/engine/field-game.md, UNKNOWNS.md
---

# 404. Mutation's town camera is pushed out of walls by another mask: from Mutation on cameraPosCalc asks ccModelHitCheckQZ for bit 2, and the world harness lays ROOTTOWN01 out as each disc does

[[397]] left two classes of tools/test_world_rs.py failing on Mutation:
- `PropsAgainstGame.test_town_draw` stopped on a syscall in
  `ROOTTOWN01::Draw`;
- `WorldAgainstGame.test_frames_random` differed in Kite's camera at frame
  88 of "town01 random 0".

The first was the harness; the second was the port.

## The town's draw: the harness

- **The syscall** was at main 0x00100064, `_start`'s. `Draw` had called
  through a vtable the harness never stored where Mutation reads it.
- **The layout.** From Mutation on, every ROOTTOWN01 member from +0x70 on is
  4 bytes further. The evidence is the code that reads them (MUT gcmn):
  - `Draw` (0x004378c0) reads the vtable at +0x1b0, the clip flags at
    +0x1b4 and the waters at +0x1c0;
  - `DrawBG` (0x00436000) reads +0x74-+0x7c and +0x1a8;
  - `DrawObj` (0x00436270) reads +0x94, +0x118 and +0x1b4.

  `DrawMap`'s +0x10-+0x18 stay where they were. Outbreak's vtable at
  +0x1b0, which the harness already found, is the same shift.
- **The probe** opened Mac Anu as Infection's on every disc
  (`Town::open(.., Volume::Inf, ..)`).

## Kite's camera: the port

- **Where it differs.** At frame 88 a reset starts, and only the eye's x
  differs (game 541.656, port 525.0). The camera's code differs between
  Infection and Mutation only in addresses, except in one place.
- **The mask.** `cameraPosCalc` pushes the camera out of a town's walls
  with `ccModelHitCheckQZ(off, pos, 25, mask, 0)`. The mask is 1 on
  Infection (main 0x00161f8c) and 4 from Mutation on (MUT 0x0016209c, OUT
  0x001610b0, QUA 0x00160f40).
- **What 4 means.** Every polygon of Mac Anu's mesh has bit 2 (field-game.md),
  floors and stairs as well as walls. So the later towns' camera is pushed
  off them too.
- **The port** asked for 1 on every disc.

## Fix

- `Hits`' camera `sphere` takes mask 4 from Mutation on.
- **The harness.** `rt_off` lays ROOTTOWN01 out as the disc does, and
  `TownDraw` calls `Draw` through the vtable's +8, which Outbreak and
  Quarantine do not name. `GateRun` leaves the unnamed `AwakeDistantLight`
  to run.
- **The probe** opens Mac Anu and the gate as the disc has them, and its
  player and camera take the disc's volume. Before this the camera and
  player kept Infection's square root, so Kite's fade differed by an ulp
  on Outbreak and Quarantine once the town draw loaded.
- **A new check.** `test_sphere` runs `ccModelHitCheckQZ` beside
  `Hits::sphere` at 600 points about Mac Anu's walls, masks 1, 2, 4 and 5,
  through the probe's new `sphere` command.

## Checked

- **tools/test_world_rs.py** passes whole on all four volumes: 22 cases
  each, two skipped on Infection (the later class's). That covers the town
  draw, the gate, the frames, the merchants, the talk and the walking PCs.
- Before the fix, `test_frames_random` failed on Mutation at frame 88. With
  the probe's volume set, `test_frames_floors` and `test_frames_random`
  also failed on Outbreak and Quarantine (Kite's `transparency`), until
  the player took the volume too.
- piney-game's suite (four threads), piney-world's tests, clippy, fmt,
  `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing.
