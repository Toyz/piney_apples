---
number: 522
title: test_town02_rs on every volume; from Mutation on Dun Loireag's cloud skies draw on layer 5, and from Outbreak on the water's screen point is held to the frame buffer
date: 2026-10-30
area: world, test
files: tools/test_town02_rs.py, crates/piney-world/src/town/dun_loireag.rs, crates/piney-world/src/town/mod.rs, docs/engine/town02.md
started: 2026-10-30T10:09:42-06:00
took: 25m
---

# 522. test_town02_rs on every volume; from Mutation on Dun Loireag's cloud skies draw on layer 5, and from Outbreak on the water's screen point is held to the frame buffer

`test_town02_rs` ran on Infection only (worklog 521's open question).
The harness now builds ROOTTOWN02 on every disc and runs on all four
volumes. On the later ones it found two port differences, and both are
fixed.

## The harness

- **A stale probe name.** Its build step still named `town02_probe`.
  A partial revert while writing worklog 521 had put it back, so at
  1639f06 the harness could not build its probe on any volume. It names
  `dun_loireag_probe` again.
- **The layout.** From Mutation on, ROOTTOWN's members from +0x70 to
  the vtable (+0x1ac) are 4 bytes further on, and ROOTTOWN02's own
  members after it are 12 further (`rt_off`). The evidence is the
  offsets MUT gcmn `DrawBG` (0x00439110) and `Draw` (0x0043ab30) read:
  the crisis clumps at 0x74, the water at 0x1bc, the clouds at 0x1e0,
  the flare at 0x248 and the sky clumps at 0x250-0x25c.
- **The vtable.** Outbreak's and Quarantine's carry does not name it.
  It is the second vtable the constructor stores at +0x1b0, the first
  being ROOTTOWN's, which is how `test_world_rs`'s `rt_vtable` finds
  ROOTTOWN01's. `Draw` and `DrawMap` are called through its slots +8
  and +28 where they have no name.
- **The statics.** Outbreak's and Quarantine's carry names none of
  them. They are found through the code that uses them:
  - `u$1777` is the one float `Draw` stores through $gp;
  - `v$1346`, `v$1346+4` and `v2$1347` are the first three floats
    `DrawBG` reads through $gp.

  From Outbreak on they are plain globals with no `init$` guard, so
  there is no init flag to clear.
- **The layers.** From Mutation on, `GO` adds a layer at
  `WORLD_MAN +0x4d4` with priority 5 (MUT main 0x001b51c4). The harness
  now makes it too.

## The port: Dun Loireag's cloud skies on the later volumes

From Mutation on, `DrawBG` draws `CMP_sr2clo_1_1` and `_1_2` after
`SetActiveLayer(7)` (MUT gcmn 0x00439334), which picks that priority-5
layer. That puts them under the characters (10), where Infection's
`SetActiveLayer(3)` drew them on effLayer (20), over the characters. It
is the only `SetActiveLayer(7)` on the three later discs. The port drew
them on effLayer on every disc. They now go on `layer::cloud_sky(volume)`:
20 on Infection, 5 from Mutation on.

## The port: the water's screen point from Outbreak on

Outbreak's `waterUVModifi2` (OUT gcmn 0x00518170, 1,044 bytes where
Mutation's is 780) changes three things:

- it takes the distance with `sqrt.s`, which the port already did;
- it holds `sx - 1792` to 0-512 and `sy - 1824` to 0-448 before the
  divisions: a value below 0 goes to 0, and anything not at most the top
  goes to the top (0x006e1150 holds 512.0);
- it converts with the FPU's `cvt.w.s`, the same truncation on the now
  non-negative values.

Before the fix the port gave, for example, an S of 1,400 where the game
gives 256. `waterUVModifi`, the field's heat haze, reads the same
constant at the same four places, and the port's haze goes through the
same `water_st`. `water_st` now holds the point on Outbreak and
Quarantine; Mutation's code has no hold.

docs/engine/town02.md has both.

## Checks

- `test_town02_rs` passes on all four volumes: setup, draw, crisis draw
  and water UVs.
- `test_world_rs` (Mac Anu's water), `test_field_ambient_rs` (the haze)
  and `test_town03_rs` pass on Outbreak and Quarantine.
- piney-world's tests and piney-game's suite, release; clippy with all
  features, fmt.

**Still unknown:** nothing
