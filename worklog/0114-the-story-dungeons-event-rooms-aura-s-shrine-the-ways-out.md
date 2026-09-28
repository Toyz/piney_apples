---
number: 114
title: "The story dungeons' event rooms: Aura's shrine, the ways out, and the ban block"
date: 2026-09-25
area: world, test
files: crates/piney-data/src/dungeon/special.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/shrine.rs, tools/test_dungeon_rt.py, docs/engine/dungeon.md
---

# 114. The story dungeons' event rooms: Aura's shrine, the ways out, and the ban block

Story rows of `room_type` 25-35 were built as empty rooms, and a door into
any row of 16 or more was an ordinary door. Area 23's shrine was an empty
room, and area 27's door to Skeith's arena ([[112]]) led into nothing.
Now `MakeRoom(ROOMDATA *)`,
`SetRoom`, `GotoNextRoom` and `WORLD_MAN::Enter` take those rows as the
game does ([dungeon](../docs/engine/dungeon.md#event-rooms)).

## The rooms

- **What each type builds** (`piney_data::dungeon::special`). The names
  are the code's own constants, not a table:
  - types 16-24 and 34 have no room;
  - types 25-35 but 27 are built from a scene file of their own
    (`se1_3` for area 23's shrine, `se1_4`, `se1_7_2`, `se3_5_2`, and the
    later volumes' ones);
  - each has its `LGT_` light, and type 26 is turned by pi.
- **`MakeRoom`.** The rows are centred as 16-cell rooms. The last event
  row's file is loaded as `spccs`, and `DungeonArea` keeps it beside the
  dungeon's file (`RoomFile`). Every anm now knows which file it plays
  from: the poses, hits, names, doors, gim slots and the draw all follow.
- **`SetRoom`** of an event room plays the room's chunk twice forward, lit.
  Its light joins the group at `750 (x + 4, y + 4)`, the ambient is the
  chunk's (`GetAmbient`, in the EE's division), and the fog is
  `SetFog(32767, 65536, 0, 100, 0)`. The next ordinary room puts the fog
  row and the base lights back.
- **Floor 9.** `MakeFloor`'s story path skips floor 9 only for the down
  stairs. The port built no rooms there. Area 108's dungeon has rooms on
  floor 9, which the checks found. The doc said the same wrong thing and
  is fixed.

## The branches

- **`GotoNextRoom`** answers `specialRoom` -1 on leaving a room, then
  takes its `isEventArea` branch:
  - a banned room (`CheckAreaBan`, the save's 32 entries) rebuilds the
    room left and answers -100;
  - 108, 73, 47, 66, 46 and 27 answer -255 (66 bans the room first);
  - 23, 25, 48, 71, 77 and 101 stand the party at the room's
    `OBJ_user_point` (a temporary anm at the centre), with `specialRoom`
    0;
  - 16 makes and drops a temporary anm and takes the doorway;
  - 91 stands at `DMY_marker01` plus the centre, w 2.

  `DUNGEON::RoomSelect`'s 71 and 77 branches are in too.
- **`WORLD_MAN::Enter`** on -255 is `ChangeArea(1, n)`: 66 goes to 67, 108
  to 9, 47 to 9 (volume 2) or 10, 73 to 4, 46 to 2, and 27 to 1.
  - Area 27's call would fall into 46's, but `ChangeRequest` has put the
    task to sleep, so field 1 it is.
  - The call goes through `FieldWorld`'s `change_area`, and `enter` now
    carries the save.
- **The ban block.** `SetDoor`'s tail, `GetBanRoom` and
  `CMP_o_block_m0_`:
  - The room next to a banned one gets the block at its gate wall (or
    door dummy) nearest the banned room's centre, by `ccGetDist` over the
    first four.
  - The block's hits join the list's tail at that matrix
    (`ccClump::HitEnable(1)`, `SetHitMatrix(float (*)[4])`). A door
    rebuilt later lands after it, as the list does.
  - It is drawn after the room.
  - The dungeon holds the save's bans (`DungeonArea.bans`). They are
    copied in by `new_banned`, by `GotoNextRoom` from its save, and by the
    field world before `RoomSelect` and `MoveDoor`. The probe's `ban`
    copies them too.
- **`specialRoom`** now reaches the minimap, whose `DrawMap` returns
  while it is not -1, and `ccSndSQLoad`, which loads the story area's bank
  (`FieldWorld::special_room`).

## Checked

- **`test_special_rooms`** (`tools/test_dungeon_rt.py`, new) covers 15
  story dungeons, each in a fresh game with the event room's file decoded
  on the heap. Placing that file after the main one had overlapped the
  harness's globals, which is what earlier gave wrong poses and a wrong
  start. It compares:
  - the floors as `MakeRoom` leaves them, `spccs` and `GetStartPosition`;
  - 8 event rooms' `SetRoom`: the chunk, hits, fog and ambient;
  - 56 `GotoNextRoom`s through up to three doors into every such row,
    free and then banned: 29 answering -100, 11 answering -255, and 16
    into the room. Each compares the answer, `specialRoom`, the enter
    flag, the arrival, the room built's hits (25 of them with the block)
    and area 66's bans;
  - 56 `WORLD_MAN::Enter`s: their first change of area or scene;
  - 8 `RoomSelect`s.

  The totals are 1,616 hit models identical and 24 within the keyed
  rotation's tolerance, with 0 mismatches. The walks skip area 67's
  roomless row behind a door (no branch takes it), because the game's
  `SetRoom` reads through its null anm.
- **`event_25_opens_in_area_23s_shrine`** (piney-game) puts `--mode
  story:25` in area 23's dungeon. Event 25's blocks 0-15 are marked
  played and block 1's `eventStatus[0]` is 1, since block 16's settings
  carry block 6's status. The walk goes from room 0 by breadth-first legs
  down two floors and through room 3's door, and checks:
  - Kite's arrival at the shrine's `OBJ_user_point`;
  - `specialRoom` 0 and `se1_3`;
  - the story bank asked for;
  - block 16 played, `area_ban 23 0 2 2` in the save, and the scene back
    to Dun Loireag.
- **The Gott statue.** `the_gott_statue_room` (piney-world) finds the
  statue room in story area 14's dungeon and in four random ones:
  - built from `symroom[type]`;
  - its pieces with models, so they are drawn;
  - a kind-2 slot for `SetIDOL`'s idol.

  `story_4_shots` shows the statue, its ring and its box drawn in floor
  1's room 4. Nothing was missing to fix.

**Still unknown:**
- **Type 32's leaves** (`ANM_se4_2lea`, twenty `LEAF`s) are not ported.
  They belong to a later volume's room.
- **Not checked against the game:**
  - how `ccLight` combines an event room's `LGT_` record with the place
    `SetRoom` gives it;
  - the draw of the block's clump nodes, which the port draws at its
    matrix.
- **Area 67's type-34 row** has a door into it and no branch, which would
  crash the game. Whether the story ever lets the party reach it is not
  known.
