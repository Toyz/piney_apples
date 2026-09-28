---
number: 147
title: The dungeons' palette swaps: SetClutList and ChangeClut
date: 2026-09-26
area: world
files: tools/dungeon_tables.py, crates/piney-data/src/dungeon/mod.rs, crates/piney-data/src/dungeon/inf.rs, crates/piney-world/src/dungeon_area.rs, docs/engine/dungeon.md
---

# 147. The dungeons' palette swaps: SetClutList and ChangeClut

A GAPS item. `WORLD_MAN::SetDungeonTexClut` gives each server's dungeons a
clutType, and on clutType 3 and 4 the game recolours the dungeon with
other palettes of the same file. Theta, the server Infection reaches
through Dun Loireag, is clutType 3 for field types 2-4, 7 and 9. The port
drew those dungeons in Delta's colours.

## The game

- **The lists.** The `DUNGEON` constructor calls `SetClutList(sfx)` (gcmn
  0x005c1440). The suffix is "c1" for clutType 3 and "c2" for 4 (types
  0-3), or "c1" to "c3" by `GetBG` for the lake types. The function copies
  five arrays of `CLT_` names, one per type group (types 0, 1, 2, 3, then 8
  and 9; 52, 62, 37, 42 and 40 names). For each name it looks up the name
  and the name plus the suffix in the dungeon's file.
- **The swap.** `SetRoom` calls `ccAnm::ChangeClut(twin, name)` on the
  room's anm for each pair, stopping at the first missing name.
  `ccModel::ChangeClut(new, old)` (main 0x0013aad0) writes the new palette
  into every material whose palette is the old one, in the shared model.
  So from the first room on, every model of the file draws with the
  twins, the doors included.
- **`DUNGEON::ChangeClut`** (0x005c17b0) is `SetWater`'s own, for the
  water's effects. It belongs with the rooms' dressing, which is not
  ported yet.

## The port

- **The generator.** `tools/dungeon_tables.py` reads the five arrays and
  their counts from `SetClutList`'s code (the arrays formed with lui/addiu
  whose first word points at a `CLT_` string; the counts from `addiu $s1,
  $zero, N`). They land in `Tables::clut_lists`, and `Tables::clut_list`
  picks the list and suffix.
- **The draws.** `DungeonArea::clut_swaps` pairs each name with its twin
  in the dungeon's file. Every draw of that file (rooms, doors, the ban
  block) passes the pairs, which `ccAnm::ChangeClut`'s swap already means
  in the draw list.
- **The check.** `theta_dungeons_swap_their_palettes` builds a type-3
  dungeon on servers 0, 1 and 3: no pairs, 42 pairs to "c1", and 42 pairs
  to "c2", every twin found.

**Still unknown:** no picture of a swapped dungeon was compared with the
game's. A twin the file lacks would get a null palette in the game; the
port leaves that name unswapped.
