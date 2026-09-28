---
number: 219
title: Harnesses on Mutation's own code
date: 2026-09-27
area: volumes
files: tools/volume.py, tools/test_world_rs.py, tools/test_save_init_rs.py, tools/test_town03_rs.py, crates/piney-world/examples/town03_probe.rs
---

# 219. Harnesses on Mutation's own code

Every harness so far ran Infection's executable in eemu. Mutation's code
differs wherever the port has work left there: Carmina Gade (0218), the
town maps, and the functions `voldiff` lists as changed. A harness must run
that volume's own code.

## The volume a harness runs

`tools/volume.py` picks the disc from `PINEY_VOLUME` (infection by default):

- `ELF`, `ISO` and `DATA` are that volume's executable, disc image and
  DATA.BIN.
- `va(inf_addr)` carries an Infection address to the volume. It finds the
  Infection symbol that holds the address and the same symbol in the
  volume (a later volume's names come from the `.syms` sidecar that
  `piney-gen syms` writes), and keeps the offset.

`test_world_rs.py`, which the town, field and NPC harnesses build on, now
takes its disc from `volume` and passes each of its 63 Infection addresses
through `va()`. On Infection `va()` returns the address unchanged. On
Mutation, for example, `ccSys` at 0x003788e0 becomes 0x0038b860 and
`npcTbl` at 0x00619460 becomes 0x00647e50.

The boot's save (`test_save_init_rs.boot_save`) is made by the game's own
`ccSaveData` constructor. Mutation's constructor first allocates a 2132-byte
object of its own through `__nw__`, so the machine now has an allocator.
It also finds `ccSnd` by name.

## Carmina Gade against the game

`tools/test_town03_rs.py` runs on Mutation, whatever `PINEY_VOLUME` says.
The game's side is Mutation's `ROOTTOWN03`, built and drawn by its own code
in eemu, `AIRSHIP` included; `town03_probe` is the port's side.

- **The set-up** (both files):
  - `SetFog` and the background colour;
  - the 34 `STATICMODEL`s' rows, types, models and places;
  - the twelve lights, the distant one given to `WORLD_MAN`;
  - the water's times;
  - the airship as its constructor leaves it.
- **The draw.** 2700 frames of `ROOTTOWN03::Draw`, the camera about the
  town and now and then out of the water's reach. They cover the airship's
  first leg, its turn, its wait and the start of the next leg.
  - Every piece in order, with its layer: water 0, the copy, the airship
    (with the root `Move` set), the four background clumps, the rows, the
    rows without fog, `DrawMap`.
  - After each frame: the airship's state, each `effSmokeN`'s and
    `ccSeOn3D`'s arguments, `fieldrand`'s seed, `DrawBG`'s scrolls and the
    water's scroll.
  - Seen over the run: 458 puffs and 29 sounds.
- **The crisis.** 200 frames in `town03d`, with the three crisis clumps on
  -60, -50 and -45 and their materials' offset.

All three tests pass. The town02 and world harnesses still pass on
Infection (22 tests).

**Still unknown:** The other harnesses still name some Infection addresses
of their own, outside `test_world_rs`. They move to `va()` as each is run
on a later volume. Carmina Gade's `DrawMap` and Mutation's changed town
maps are next.
