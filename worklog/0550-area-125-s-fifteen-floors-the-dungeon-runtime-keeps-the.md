---
number: 550
title: Area 125's fifteen floors: the dungeon runtime keeps the disc's floor count, so B10 has its stairs down
date: 2026-11-02
area: world, volumes, ui
files: crates/piney-world/src/dungeon/mod.rs, crates/piney-world/src/map/dungeon.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/pack.rs
started: 2026-11-01T23:13:59-07:00
took: 47m
---

# 550. Area 125's fifteen floors: the dungeon runtime keeps the disc's floor count, so B10 has its stairs down

Issue #78 (QUA post-game, build e70f6a8): in Omega Hidden Darkside Holy Ground
(event area 125) the stairs down on B10 led to an empty void labelled B9. The
player's log, replayed through the launcher (`player_log_replay`), goes down
from floor 9 block 3 and arrives on floor 8 (frame 107123).

## The cause

From Mutation on, `DUNGEON` keeps its floor count in a field (`+0x430`): the
constructor sets 10, then 15 when `GetEventAreaNumber` is 125 (QUA gcmn
0x004d4400-0x004d441c), before field type 4's dungeon 0 clears `isEventArea`.
`Generate`'s story path makes that many floors (0x004dd88c), `MakeFloor`
looks for the down stairs on every floor but count - 1 (0x004dc778) and over
count x 15 `EditDungeon` rows, and the per-floor arrays are `new[]`s of that
length. The map's `levelstr` has fifteen pointers on these volumes, "B 1" to
"B 15" (QUA 0x005c0b60; MUT 0x006ca5f0, OUT 0x006c9150), where Infection's
has ten.

The port's dungeon runtime kept Infection's constant: `FLOORS = 10` in
`piney_world::dungeon` and `piney_world::map::dungeon`. B10 was taken for the
last floor, so `MakeFloor`'s story path never set its `DownRoom`; floors 10 to
14 were never built; the map had ten labels and ten smallmaps. The data side
already knew the count (`Tables::edit_floor_count`, worklog's dungeon page),
the runtime did not ask it. The generated `level_str` was read with
Infection's length (10) on every volume.

## The fix

`DungeonArea::floor_count` is `edit_floor_count(GetEventAreaNumber)`, and the
story floors, their `fieldrand` rolls and the event rooms' file go over it;
the last floor without down stairs is count - 1. The map makes a label and a
smallmap per floor of the dungeon. `level_str` is generated with
`later(10, 15)` (piney-gen `world` group; `DATA_VERSION` 35, since the build's
table changes).

`area_125_has_fifteen_floors_from_mutation_on` builds the area on Quarantine:
15 floors, B10's down stairs, B15 the last, B11 to B15 built, and the map's
labels running to "B 15". The player's log no longer reaches B10 the same way:
Generate now draws fifteen floors' `fieldrand(5)`, so the rooms after floor 0
differ from the run the log was made on.

## The harness

`tools/test_dungeon_rt.py` (Quarantine, Infection) passes as it was. With an
area 125 case added, the game's Generate in the interpreter runs out of its
heap (0x01100000-0x01f00000) building the fifteen floors; with freed blocks
of the same size reused for that case alone, Generate, every floor's up and
down rooms and start positions, the rooms, land and height checks and the
GotoNextRoom answers agree with the port, and one Draw after the stairs up
from B15 does not (below). The case is left out of the committed harness.

**Still unknown:** the dungeon runtime harness (tools/test_dungeon_rt.py) cannot run area 125 as it stands: the game's Generate over fifteen floors outgrows its 14 MB bump heap; with freed blocks reused for that case alone every floor's rooms, stairs and start positions, the land checks and the GotoNextRoom answers agree, but a DUNGEON::Draw after the stairs up from B15 counts 2 door anms in the game and 0 in the port, which reused memory may explain and was not settled
