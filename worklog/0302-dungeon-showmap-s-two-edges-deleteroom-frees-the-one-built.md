---
number: 302
title: DUNGEON::ShowMap's two edges: DeleteRoom frees the one built room, and room 15 is never under the player
date: 2026-09-28
area: render, world
files: crates/piney-world/src/map/dungeon.rs
resolves: 301
---

# 302. DUNGEON::ShowMap's two edges: DeleteRoom frees the one built room, and room 15 is never under the player

[[301]] left two places where the port's `show_map_step` looked different
from `DUNGEON::ShowMap` (INF GCMN.PRG:0x005cf260). Neither is a difference
the port can or should copy, so no code changed.

## DeleteRoom frees the built room, whatever room it is given

[[301]] read `DeleteRoom(level, here)` as freeing only the slot
`room[level][here]`. The code says otherwise (INF GCMN.PRG:0x005c1980):
- Only one thing is indexed by the arguments: the `ccAnm` at
  `this + 0x2f230 + level * 60 + room * 4` (15 rooms a floor), which
  `SetRoom` (0x005c1ca0) allocates at 0x005c1d74.
- Everything else it frees is the single built room, whichever it is:
  `ccClutRoll` +0x404, the `ccAnm` +0xd38bc, the room's `ccClump` +0xd3880,
  the light group +0x370, the `ccAnm`s +0x364 (3), the `LEAF`s (20), `SNOW`s
  (60), `FIREFLY`s (5), the clumps and anms of the ten objects, the
  `ROOMLIGHT`s (32) and the `ccAnm`s +0x3f4 (4).

The port's `delete_room` drops the built room whole, which is the same. The
only gap is the `GotoNextRoom` window ([[301]]), where the room built is
not `here`. There the game frees the built room anyway and leaks its
`ccAnm` object. `DUNGEON::Draw` draws only `room[level][here]`, so nothing
of it is shown.

## Room 15 under the player

The cell lookup (0x005cf300-0x005cf404) is the port's `here`: cell
((30000 + x) / 750, (30000 - y) / 750), byte +0x432. Its "no room" value is
15 (`piney_data::dungeon::NO_ROOM`), not 0xff as [[301]] had it. With 15 the
game would run:
- `DeleteRoom(level, 15)`: the `ccAnm` slot 15 of a 15-slot row is the next
  floor's slot 0, which is empty while only this floor's room is built;
- `SetRoom(level, 15)`: its own tables are read past each floor's 15 rows,
  with a different stride in each, so it builds no real room.

A player only stands on a room's cells. Door cells belong to their room.
Every other cell is outside the walls the collision keeps him in. So the
game never takes this path in play. The port skips the rebuild there,
because copying out-of-bounds reads with no source for their values would
be guesswork.

**Still unknown:** nothing about these two paths. Whether any dungeon has a
reachable cell whose room byte is 15 was not surveyed over every floor of
every area.
