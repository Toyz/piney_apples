---
number: 492
title: Type 32's tree and leaves ported: area 101's event room's twenty LEAFs, 400 frames bit-exact with the game
date: 2026-10-25
area: world
files: crates/piney-world/src/dungeon_area/leaves.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/dungeon_area/dress.rs, crates/piney-world/src/field_world.rs, crates/piney-world/examples/dungeon_probe.rs, tools/test_dungeon_rt.py, docs/engine/dungeon.md
resolves: 291.10
started: 2026-10-25T19:34:02-06:00
took: 27m
---

# 492. Type 32's tree and leaves ported: area 101's event room's twenty LEAFs, 400 frames bit-exact with the game

Play question [[114]]/[[157]] (291.10): type 32's tree and leaves
(`ANM_se4_2lea`, `LEAF`) were not ported, so area 101's event room
(`se4_2`) stood bare.

**What the game does.**
- `DUNGEON::SetRoom` for type 32 (INF gcmn 0x005c2fe8) makes a `ccAnm` of
  `ANM_se4_2lea` from `spccs` at the room's centre (+0xd38bc; +0x8d 1, one
  `_AnimateForward`), then twenty `LEAF`s (+0xd38c0).
- `LEAF::LEAF` (0x00505050):
  - a timer `fieldrand(20) + 10`;
  - a spin of -0.05 or 0.05 by `fieldrand(100)`;
  - a turn about x of `ccRandF() * -2`;
  - `MDL_se4_2lea`;
  - `calcLeafPos` (0x00504da0) for its place and step (all from
    `fieldrand`).
- `DrawEff`'s end (0x005ce108) steps the tree and draws it. Then for each
  leaf it runs:
  - `Move` (0x00505360): the step added, and below z 300 a new place;
  - `Draw` (0x005051f0): `RotZ`, `RotX`, moved to centre plus place; the
    turn about z on by the spin; at the timer's end a new timer and the
    spin turned back toward 0; the model drawn with PRIM 0xc (no fog).

**The port** (`crates/piney-world/src/dungeon_area/leaves.rs`).
- `set_leaves` runs from `set_room_with` for a type 32 row. `Leaf` and
  `Leaves` live in the dressing, so `clear_dress` frees them as
  `DeleteRoom` does.
- `ccRandF` draws from `ccRand`, which the dungeon does not hold. Each
  leaf's tilt is therefore owed until the world calls
  `DungeonArea::settle_leaves` with `combat.cc`. `FieldWorld` does so
  after each call that can `SetRoom`:
  - the dungeon's set-up (after `ccInitRand`, as in the game's `GO`);
  - `room_select`;
  - `show_map`;
  - `Enter`.
  If two rooms are built before one settle, the leaves standing take the
  last draws.
- `step_leaves` is `DrawEff`'s end. The tree draws depth-fogged and the
  leaves unfogged, on the effects' layer.

**Checked.** tools/test_dungeon_rt.py `test_leaves` (new, with the
probe's `leaves` and `leafeff`):
- area 101's type 32 room set up with `ccRand` seeded alike on both
  sides; the twenty leaves made match, field by field;
- then 400 frames of the game's whole `DrawEff`: each leaf, its model's
  matrix and `fieldrand` match every frame (19 leaves start again from
  the top).
The whole harness passes (12 tests). The test sets the port's `fieldrand`
to the game's before the room, because a story dungeon's `fieldrand`
after `Generate` is still the open question in dungeon.md.

Docs: dungeon.md (a new section, "Type 32's tree and leaves"), UNKNOWNS.md.

**Still unknown:** nothing
