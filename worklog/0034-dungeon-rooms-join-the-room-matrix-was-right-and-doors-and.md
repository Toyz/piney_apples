---
number: 34
title: Dungeon rooms join: the room matrix was right, and doors and fog were missing
date: 2026-09-23
area: world, render, test
files: crates/piney-data/src/dungeon/place.rs, crates/piney-data/tests/dungeon_place.rs, tools/dungeon_tables.py, tools/test_dungeon_rs.py, crates/piney-viewer/src, docs/engine/dungeon.md
---

# 34. Dungeon rooms join: the room matrix was right, and doors and fog were missing

[[31]] left dungeon floors that did not join and looked wrong, and a
contradiction: a medium room seemed to want the rotation one way, and a
large room the other. The user kept saying the rooms were missing doors.
A helper agent worked it out, and I re-ran its checks.

## The contradiction was a measurement

The large room `ANM_sd1l2n31` has two slope pieces that run down into a pit
and reach 6,650 units out on its local −x and −y, past its own 6,000 edge.
[[31]] read those as the room's openings. They are not.

The rule is the plain one, which the viewer already used. `SetRoom` passes
position (pos.x, pos.y, 0) and rotation (0, 0, rotate) to
`ccCoord::SetMatrix_PosRotZYX`, so the room is `T(pos) * Rz(rotate)`.
libvu0's Rz turns +x toward +y, the same as glam's. `GotoNextRoom`
agrees: stepping east leaves the player facing +π/2.

The rooms themselves prove it. Every room model marks each exit with one
object 600 units inside the middle of that side, facing out:
- a door dummy (`OBJ_0pae0_*`), or
- a gate wall (`OBJ_w_0g10_*`).

Over all 2,569 (table, row, model) triples of the ten types, the markers
turned by Rz(rotate) give exactly the row's exits. With Rz(−rotate) only
1,513 do, the symmetric ones.

## What was missing

- **Doors.** `SetDoor` plays a door animation at every door dummy, one per
  type pair (`ANM_sd1ae0_a` for types 0 and 4, and so on). A door is at its
  last frame (open) when the room is empty and frame 0 (closed) otherwise.
  The viewer now draws them open. They are what fills the gaps: each door
  piece reaches about 800 units out, past the shared edge, and the two halves
  of a doorway overlap.
- **Fog and clear colour.** The `DUNGEON` constructor picks a row of fog and
  ambient values from one of 13 tables, by the clutType/texType pair that
  `WORLD_MAN::SetDungeonTexClut` sets from the server and field type. The
  fog colour is also the frame's clear colour. The viewer applies both, with
  F (or Circle) to toggle fog.
- **Not the ambient.** It does not light the rooms. Every model in every
  dungeon file is unlit, and `ccModel::Draw` sends unlit models to VU1
  programs that write the vertex colour unchanged, with colours averaging
  44-70/128. Dungeons are as dark as that. The ambient only reaches lit
  models (characters, monsters, items). The 1x image is what the GS writes.

## Checked

- **Placement.** `tests/dungeon_place.rs` checks every room table's markers.
  Over 400 generated dungeons (all types, servers 0-4, sizes 1-10), it also
  checks:
  - every exit against the map's door cells;
  - that 11,444 room pairs have their exits facing each other 1,200 apart;
  - that 381 statue rooms reach through their doorway.

  With the rotation's sign flipped, 5,897 exits would face a side the map
  does not open, so the test notices the mistake of [[31]].
- **Fog.** The fog rule was run in eemu for 480 cases and matches the Rust on
  table, row and packed colour. The ambient is within 1 ulp; the EE's
  division truncates. `SetDungeonTexClut` matches on all 78 server and
  field-type cases.
- **On integration:**
  - `test_dungeon_rs.py`: 7 tests OK.
  - `cargo test --test dungeon_place --test dungeon`: 2 and 3 passed.
  - The viewer's tests: 3 passed, including a new `every_dungeon_type_builds`.

The viewer still draws every room of a floor at once. The game shows one.

**Still unknown:**
- **Lighting.** `SetLight`'s omni lights and glow effects, which do not
  touch the unlit rooms but do light characters.
- **Palettes.** The clutType 3/4 palette swaps.
- **Lakes.** `GetBG` for the lake dungeons (taken as 0), and the white plane
  in lake doorways.
- **Fog depth.** Whether the GS fog uses view depth exactly as the viewer
  assumes; `ccView`'s projection was not checked.
