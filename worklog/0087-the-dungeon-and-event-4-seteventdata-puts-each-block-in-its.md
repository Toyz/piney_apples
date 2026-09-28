---
number: 87
title: The dungeon and event 4: SetEventData puts each block in its room, the doors, the 2D map, and the walk in
date: 2026-09-24
area: world, script, battle, test
files: crates/piney-event/src/vm/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/party.rs, crates/piney-world/examples/dungeon_probe.rs, tools/test_dungeon_rt.py, docs/engine/dungeon.md, docs/engine/battle.md, docs/engine/field-walk.md
---

# 87. The dungeon and event 4: SetEventData puts each block in its room, the doors, the 2D map, and the walk in

In `story:4` every block of event 4 fired in the dungeon's first room.
Orca opened with his line for the dead end instead of his line for the
dungeon's start, and the event cameras were placed for rooms Kite
was not in ([[82]], [[86]]). The combat agent's third increment adds what
the game does between entering a story dungeon and its events.

## Each block in its room

`WORLD_MAN::SetEventData()` (main 0x001a3c40) runs from
`ccSetupGameCtrl`, between `ccStartThEvent` and `ccEnableThEvent(0)`.
- **When.** In a dungeon of a story area with an `EditDungeon` entry. It
  reads the area's first entry, whichever dungeon the party is in.
- **Rooms.** Each room row with an event flag becomes an event point
  (`ccEvent::SetEventPoint`, 0x001b3620).
- **Gimmick rows.** Type 2 becomes an event position with its facing
  (`SetEventPos`, 0x001b3590); type 3 with kinds 7-26 becomes a warp point.
- **Slots.** Each call fills the first of 16 slots whose number is
  negative, and does nothing when none is.

A block's `set in_point n` is then true only in the room of point n. In
story area 14's `D0001` the points are:

| point | room | TEACH-D blocks |
|---|---|---|
| 4 | floor 0 room 0 | 4 |
| 1 | floor 0 room 1 | 2 |
| 6 | floor 0 room 2 | 3 |
| 2 | floor 0 room 3 | 5-6 |
| 5 | floor 1 room 4 | 7 |
| 3 | floor 1 room 1 | 8 |

## The doors

- **Setting a room's doors.** `SetDoor` (0x005c7c30) makes a door per door
  dummy. Each stands open if nothing of the entry control belongs to the
  room (`ccCheckActiveObject`), else shut.
- **Each frame.** `MoveDoor` (0x005cd3d0), from `Draw`, opens a held door
  a step a frame from the first frame the entry control has nothing
  switched on, with its sound.
- **From the scripts.**
  - `open_door` (case 158) is `OpenDoor` (0x005c8820): every door runs its
    whole length and stays open.
  - `close_door` (159) is `CloseDoor2` (0x005c8800): each leaf sinks into
    the floor over the animation's length, then the doors are made again.
- **Event entries.** `ccEntryEventMng` calls `CloseDoor` (0x005c8f50) for
  each event entry, so a room an event fills starts shut.

## The rest

- **The 2D map.** `Get2DMapPtr` / `Get2DMapInfo` / `GetRoom2DPos` give the
  party's path finding the current room's part of the floor map.
- **Clearing bootParam.** `ccThSpcDelete` (0x005a06d0) zeroes `bootParam`
  as an area ends. battle.md had said nothing clears it.
- **Event task off during the world's own scene changes.** The entrance,
  the doors and Gate Out turn it off (`ChangeRequest` calls
  `ccDisableThEvent`).
- **area_host.** It now answers `stream`, `remove_trap`, `open_door` and
  `close_door`.
- **Entering.** Walking from field 14 into its dungeon now works.

## Checked

- **Against the game** (`test_dungeon_rt`, 0 mismatches):
  - `EventDataAgainstGame` runs `SetEventData` for 102 cases: 204
    `SetEventPoint`s, 254 `SetEventPos`es and 10 warp points. It also runs
    the real `SetEventPoint`/`SetEventPos` on 300 random slot states.
  - `test_doors` covers 75 rooms with doors: 12,846 `MoveDoor` steps, 44
    `OpenDoor`, 35 `CloseDoor`, 72 `CloseDoor2` and 332 door sounds. The
    hit lists are identical.
  - `test_dungeons` compares `GetRoom2DPos` for all 672 rooms.
- **Runtime test.** `event_4_plays_in_the_dungeon`: every block in its
  room, the trap room's goblin fought down, the doors opening, and out to
  mode 3. It tests the port against itself.
- **The disk failure.** It stopped the first verification ([[86]]).
- **Re-run after the failure.** It ran in the rebased worktree, one job at
  a time with the build guard. Passing:
  - fmt, clippy, the workspace's tests and the docs check;
  - test_dungeon_rt, test_dungeon_rs, test_field_rs and test_world_rs;
  - the battle navi, fellow and event suites;
  - test_gamectrl_rs and test_evarea_rs.

**Still unknown:**
- **Still unported event 4 host methods.** `add_spc_item`,
  `item_get_menu(_end)` (Orca's items), `remove_trap_done`, `room`,
  `prev_room`, `gimmick` and `boss`.
- **The dungeon's own objects.** The treasure box, the trapped box, the
  Fortune Wire and the idols (`WORLD_MAN::EntryGimmick`) are not in the
  runtime.
- **The event cameras in the rooms.** Not yet checked with shots now that
  each block plays in its room. The user saw the camera in the walls.
- **Doors not modelled.** `SetDoor`'s ban room and the door palettes for
  clutType 3 and 4.
