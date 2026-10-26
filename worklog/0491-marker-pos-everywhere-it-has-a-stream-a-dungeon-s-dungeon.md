---
number: 491
title: marker_pos everywhere it has a stream: a dungeon's DUNGEON file and every event map's; a plain field is the game's own crash
date: 2026-10-25
area: script
files: crates/piney-world/src/field_world.rs, crates/piney-world/src/story_map.rs, crates/piney-world/src/evarea03.rs, crates/piney-world/src/evarea_b8.rs, docs/engine/events.md
resolves: 291.3
started: 2026-10-25T19:24:40-06:00
took: 9m
---

# 491. marker_pos everywhere it has a stream: a dungeon's DUNGEON file and every event map's; a plain field is the game's own crash

Play question [[74]]/[[275]] (291.3): `marker_pos` on a plain field
(said to read `WORLD_MAN` +0x438's stream) was not ported, and no event
map but `EVENTAREA01` answered `StoryMap::file`.

**What the case does** (`ccEvMarker2Pos`, INF main 0x001b1aa4). It finds
`markerEvTbl[marker]` with `GetChunkAdrsF` in a stream chosen by `game`
+0x14, then `SetEventPos` with the dummy's position (+0x10) and rotation
z (+0x28):
- 0, a town: `WORLD_MAN` +0x430's stream (+0x1a4);
- 1, a field: with an event map (`game` +0x24), +0x444's stream
  (+0x1a4). Without one, `sw $zero, 0($zero)` (0x001b1b78): a store to
  address 0, the game's own crash;
- 2, a dungeon: `dungeon[game.dungeon]` (+0x438 indexed by `game` +0x28)
  +0xd3884, the stream `DUNGEON` loads (`DungeonName[type]`).

So [[275]]'s "+0x438 on a plain field" was the dungeon's path, and a
plain field has none.

**The port.**
- `FieldWorld::marker` now answers a dungeon from `DungeonArea::file`.
  A plain field answers nothing, which the VM records as a fault, as it
  does every crash of the game's.
- Every event map class now gives its +0x1a4 stream as
  `StoryMap::file`. Each constructor's `GetCCSAdrs` was read:
  - `EventArea` (`se1_2`, EVENTAREA02 0x00401e6c);
  - `Area43` (`se2_1`, EVENTAREA03 0x00403630);
  - `Giant` (`se1_7_1`);
  - `Arena` (the stage's file);
  - `DiscArea` (`se1_6`, EVENTAREAB8 0x0040a060).
  `EVENTAREA01` and the sky maps already did. The empty map of area 126
  has no stream and answers nothing.
- Every script's `marker_pos` (events 115, 206, 216, 218 on all four
  volumes) sits under an earlier block's `in_field` town 2 field 13, so
  in play it is `EVENTAREA01`'s, already checked by
  `event_115_field_13_talks_to_six`.

Docs: events.md, UNKNOWNS.md.

**Still unknown:** nothing
