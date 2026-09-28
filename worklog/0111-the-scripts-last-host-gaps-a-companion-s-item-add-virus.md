---
number: 111
title: The scripts' last host gaps: a companion's item_add, virus_core's level, room by RoomSelect, save_party
date: 2026-09-25
area: script, world, test
files: crates/piney-game/src/field_host.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-fieldui/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/examples/dungeon_probe.rs, tools/test_dungeon_rt.py, docs/engine/event-vm.md, docs/engine/dungeon.md
---

# 111. The scripts' last host gaps: a companion's item_add, virus_core's level, room by RoomSelect, save_party

A static survey of Infection's scripts found four instructions whose
`Host` methods still fell to the defaults: `item_add` (`add_spc_item`),
`virus_core` (`generate_area`), `room` and `room_point` (`room`), and
`save_party` in M130's block 20. I checked each against the game's own
case in `ccEvent::Execute` first. Two of them turned out never to reach
the host where the survey placed them.

## item_add

**Case 93** (main 0x001afd04):
- At level 1, for Kite (pc 0), or for a category of 10 and up, it calls
  `ccSaveData::AddItem`.
- Otherwise it calls `ccEvent::GetSpc(pc)`. If that finds a character,
  `ccMenu->AddSpcItem(ch, cat, id, num, 0)` runs instead, and the item
  does not go into the save.

**M119's `item_add 0 15 14 1`** is for Kite, so it never asks the host.
Neither does any other volume-1 `item_add` to Kite.

**The companions' items are in dungeons.** S108 gives Sanjuro (4) the
Kotetsu Sword (2/80), and S111 gives Natsume (11) item 0/60. Both are
players met in their own dungeons.

**The port.**
- `FieldUi::add_spc_item` runs the ported `AddSpcItem` (menus/talk.rs)
  with `tf` 0, so it never breathes. It wears the item if it is better,
  reads it if it is a book, and otherwise puts it in the bag.
- The town's host looks up the member in `TownParty`. The area's host
  looks up the battle character (`Combat::who`). If there is none, the
  host declines and the interpreter puts the item in the save.

## virus_core

**Case 124** (0x001b06c8) returns at once unless the level is exactly 1:
`slti $at, $s5, 2; beqz $at` skips level 2 and above. At level 1 it does
three things:
- `SimGenerateCode` from the area's three word IDs;
- sets the `protectArea` bit;
- deletes the area's protect items (category 15) from Kite.

**Level 1 means `ccEventFlagSet`.** That is a volume's events brought
forward, or the port's story starts, and `start.rs`'s replay host
already answers those. So M128 in Mac Anu never asks the town's host
while playing.

**The hosts implement it anyway**, as `WORLD_MAN` would be left: the
area the words make on the current server, via `FieldUi::set_area_words`.

## room and room_point

**Cases 130 and 131** play only. They call `WORLD_MAN::RoomSelect(floor,
block)` (main 0x0019dca0). `room_point` uses the room of the first event
point with that number (already in the VM).

**`RoomSelect`** runs, in order:
- `bgColor` 0 and the two GS words;
- `ClearRoom` and `SetRoom`;
- `DUNGEON::RoomSelect(&position, f, b)` (gcmn 0x005c95c0);
- `ChangeScene(-2, -2, -2, -2, f, b)`.

**`DUNGEON::RoomSelect`:**
- It sets `roomEnterFlag` 1, `mapHideFlag` 2 (the map painted again but
  left closed), `ccMenu+0x10` (the map status) 3, and `level` = f.
- The position starts at the room's centre (x, y, 0, 1).
- It then searches the room's anm for `OBJ_w_0g10_*` (a gate wall), and
  failing that `OBJ_0pae0_*` (a door dummy). The first match's world
  matrix, applied to (0, -200, 0, 1), is the position. Its w is 1.0, which
  is the heading the arrival takes.
- It also builds an `Rz(rotate)` matrix and never uses it.
- Areas 71 and 77 have branches of their own (`OBJ_user_point` in story
  rooms of types 30 and 31). Those are later volumes' areas, and the port
  leaves them out.

**The port.**
- `DungeonArea::room_select` does the dungeon's part.
- `FieldWorld::room_select` asks for the change of scene the way a door
  does.
- The area host's `room` calls it, then sets the map status to 3.

## save_party

**Case 88** (0x001af784) sets `partyMemberSave` to `1 << memberID[s]`
for each filled slot. The VM already does this. The only host method it
needs is `Host::party`, which both hosts answer from the registry.

**M130's block 20** runs at phase 0 in the field. It holds the Skeith
`entry 7 0 0 0` (which goes into the event manager's entry list for the
entry control), `battle_ready` (already hosted) and `save_party`. Nothing
more was needed.

## Checked

- **`test_dungeon_rt`'s new `test_room_select`** runs the game's
  `ClearRoom`, `SetRoom` and `DUNGEON::RoomSelect` in eemu beside the
  probe's new `select` request.
  - 208 rooms over the suite's dungeons: 202 in front of a way in and 6
    at the centre.
  - It compares the position, the level, `roomEnterFlag` and the room's
    hit list (18,947 hit models identical, 60 within the keyed-rotation
    tolerance).
  - The game's `mapHideFlag` 2 and map status 3 are asserted.
  - 0 mismatches. The rest of the suite still passes (5 tests).
- **`piney-game`'s `story_4_item_add_and_room`**, in story 4's dungeon:
  - `add_spc_item` gives Orca the item, and declines Sanjuro, who is not
    built there.
  - `room 0 4` builds room 4. The scene changes to it, and Kite arrives
    where RoomSelect put him.
  - Room 1 would play the story's stream 3 on entry, so the test uses the
    floor's last room instead.
- **`item_add_for_a_companion_goes_through_the_menu`** in the town, after
  event 2: Orca gets 2 more of a weapon of another job's category in his
  bag. Piros, who is not in town, is declined. Kite's items are unchanged.

**Still unknown:**
- `RoomSelect`'s `bgColor` 0 and GS words are not modelled; the change's
  fade covers those frames. The 71 and 77 branches (`OBJ_user_point`) are
  not ported.
- `prev_room` (`WORLD_MAN::GoPrevRoom`, main 0x001a40b0) still falls to
  the default. No volume-1 script was found that needs it.
- `SimGenerateCode`'s moves of the field generator's `seed` and `randcnt`
  are left aside here, as everywhere in the port's area words.
