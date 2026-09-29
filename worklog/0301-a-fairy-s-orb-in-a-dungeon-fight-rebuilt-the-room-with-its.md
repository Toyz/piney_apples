---
number: 301
title: A Fairy's Orb in a dungeon fight rebuilt the room with its doors open; the camera went through
date: 2026-09-28
area: render, world
files: crates/piney-world/src/map/dungeon.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session/tests/fairy_orb.rs, docs/engine/map.md
---

# 301. A Fairy's Orb in a dungeon fight rebuilt the room with its doors open; the camera went through

Reported in play (INF): a Fairy's Orb (TOOL 2, PERSONAL > Items) used
right as a dungeon fight started "broke the camera", and it stayed wrong.
The room vanishing for a moment is the game's own `DUNGEON::ShowMap` and
is kept.

## Reproduced

`an_orb_in_a_dungeon_fight` (fairy_orb.rs) plays two runs. Kite walks
from a random Δ dungeon's entrance (`random_areas(0, &[0])`) into room
(0, 1), where its portal starts a fight. PERSONAL then opens:

- in one run the orb is used;
- in the other the menu is only shut.

The world is still while the menu is up, so both runs leave it on the same
state. From there Kite backs into the door he came through, and each
frame's camera (`camID`, `tcam` type, eye, target, angles, distance), room
and doors are compared.

- **Without the orb** the doors stay shut (`doorFlag` 0). The camera stops
  at the leaf (eye y 33310) and Kite stops at 33372.
- **With the orb** the room came back with its doors open (`doorFlag` 1)
  while `inBattle` was still 1. The camera passed through the doorway to
  y 33010. Kite walked out into room (0, 0) mid-fight, which ended the
  fight.

Standing still, the orb changed nothing of the camera. Only the doors
differ, and the camera shows it as soon as it reaches them.

## The game's rule

- **ShowMap.** `DUNGEON::ShowMap` (INF GCMN.PRG:0x005cf260) reads the room
  under the player from `realmap[level][x/750][y/750]` +0x432 (`lbu`
  0x005cf404). The cell index is the port's `here`. It deletes that room
  (0x005cf410), and builds and deletes each unseen room. Its last call
  runs `SetRoom(level, here)` (0x005cf658).
- **SetDoor.** `SetRoom`'s `SetDoor` (0x005c7c30) calls
  `ccCheckActiveObject(f, i)` (0x0042e010) at 0x005c81a4. When it answers
  1 (no foe, no portal of that room) the door anms run their whole length
  and `doorFlag` (+0x2c) is set. Otherwise they take one step and stay
  shut (0x005c8210).
- **Camera.** Neither function touches the camera. The camera's pull-in
  (`cameraPosCalc`'s line check) meets the shut leaf and misses an open
  one.

## The port's divergence and the fix

`show_map_step` called `set_room`, which is `set_room_with(f, i, true)`.
Every room ShowMap built had its doors open, fight or not. It now takes
`clear: &dyn Fn(i32, i32) -> bool` and gives each `SetRoom` its
`ccCheckActiveObject(f, i)`. That covers the unseen rooms too, whose shut
leaves `MakeMiniMap`'s rays meet.

`FieldWorld::show_map` passes `room_clear`, the test `GotoNextRoom` and
`RoomSelect` already use. The orb's steps (`item_show_map`) and the
events' `show_map` (area host) both call it. `map_probe`'s `dshow` passes
"all clear".

- **Before the fix:** the test failed at frame 25 after the menu (the
  camera's eye y differs).
- **After it:** 240 frames equal. `fairy_orb` passes 3/3, `piney-game`
  164/164, `piney-world` all green.

`dressing.rs`'s `random_areas` and `in_random_dungeon` are now
`pub(super)` for the test.

**Still unknown:** whether this is all the user saw (no capture of their
play). Two edges are left as they were. When the cell under the player has
no room (0xff), the game still runs `SetRoom(level, 0xff)`; the port skips
the rebuild. And `DeleteRoom(level, here)` frees the slot `room[f][here]`,
so in the `GotoNextRoom` window, where the room built is not `here`, the
game would keep the built room; the port's `delete_room` drops whatever is
built.
