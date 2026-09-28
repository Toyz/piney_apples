---
number: 191
title: "A lake's two dungeons: each kept by its number, lastRoom, and the way back up (GoField for field type 4)"
date: 2026-09-26
area: world
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/evarea.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-game/src/session.rs, docs/engine/dungeon.md, docs/engine/field-walk.md
---

# 191. A lake's two dungeons: each kept by its number, lastRoom, and the way back up (GoField for field type 4)

`field-walk.md` listed "`GoField` for field type 4 ... is not ported; what
sets +0x100 is not traced". Field type 4 is the lakes, and Infection has
plenty of them:

- seven words make one ("Solitary", "Haunted Land", "Feeling", "Remains",
  "Seeding", "Valkyrie", "Emerald");
- so do story areas 33, 48, 72 and 103.

A lake has no field. Dungeon 0 is the lake itself; dungeon 1 is below it,
of `dungeonType[1] = 2`.

**What the game does** (read):

- `WORLD_MAN::Enter` on the lake's stairs down (0x0019e238) clears
  `+0x58[game.dungeon]` and sets `lastRoom` (+0x100) to `game.block`.
  It then calls `ChangeScene(2, -2, -2, 1, 0, 0)`.
- On floor 0's stairs up of a field type 4 area (0x0019e0f8), and from
  menu 86's `GoField` in dungeon 1 (0x0019e440), it clears `+0x58[game.dungeon]`
  and calls `ChangeScene(2, -2, -2, 0, 0, lastRoom)`. `GoField` in the
  lake itself does nothing.
- `GO(2)` keeps each dungeon by its number (`dungeon[3]` at +0x438) and
  makes one only where the slot is empty. Its last branch applies with
  field type 4, `dungeonPrev` 1, `block == lastRoom` and dungeon 0: it
  puts `position` at the lake's `startpos[1][0]`, then `ClearRoom` and
  `SetRoom(0, lastRoom)`, with the seed kept round it and `dungeonback` 1.

`dungeon.md` had most of this already. So the only thing not traced was
who sets +0x100: it is `Enter`.

**What the port did.** It kept one dungeon (`Kept::Dungeon`) and reused
it for any scene of area 2 after one of area 2. So the lake's stairs
down gave back the lake itself as "dungeon 1". The way up went to block
0, and `GoField` did nothing in a lake.

**Now:**

- `dungeon_area::Dungeons` holds the three slots and `lastRoom`, and
  `Kept::Dungeon` carries them.
- `FieldWorld::enter` takes the slot of `scene.dungeon` or makes that
  dungeon. On `GO(2)`'s return case it calls `DungeonArea::come_back`:
  level 0, the position at floor 0's stairs down, `SetRoom(0, lastRoom)`.
- `into_kept` puts the dungeon played back in its own slot, by its own
  number, not the next scene's.
- `DungeonArea::enter` takes `lastRoom`. It sets it on the lake's stairs
  down and uses it on the stairs up.
- `go_field` does the field type 4 case.
- `dungeon_probe` keeps a `lastRoom` of its own. `test_dungeon_rt.py`
  passes (10 tests).

Checks:

- `a_type_4_area_goes_down_into_its_second_dungeon_and_back`
  (piney-world): the exits on both stairs, `lastRoom`, and `come_back`'s
  room and position.
- `a_lake_keeps_both_its_dungeons` (piney-game): area 33 entered and left
  through `AreaMode`. The lake is kept while the second dungeon (not a
  lake) is made; on the way back both are there and the party is at the
  lake's stairs.

**Still unknown:** What reads `WORLD_MAN+0x58[n]`. `come_back` builds the
room's doors open, where the game asks `ccCheckActiveObject`. The walk
between the two dungeons was not compared with the game's own run.
