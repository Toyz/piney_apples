---
number: 98
title: Event 4 as a player plays it: the dungeon's boxes and Gott statue, their menus, the Fortune Wire and the trap room's doors
date: 2026-09-25
area: battle, world, ui, test
files: crates/piney-battle/src/gimmick.rs, crates/piney-battle/src/prim.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/affect.rs, crates/piney-battle/src/chara.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/src/lib.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, crates/piney-fieldui/src/menus/useitem.rs, crates/piney-fieldui/src/menus/getitem.rs, crates/piney-fieldui/src/menus/item.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/src/menus/target.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/tables.rs, crates/piney-fieldui/src/world.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/foe.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, crates/piney-game/src/world.rs, tools/test_battle_spawn_rs.py, tools/test_fieldui_rs.py, docs/engine/battle.md, docs/engine/dungeon.md, docs/engine/field-ui.md, docs/engine/field-walk.md
---

# 98. Event 4 as a player plays it: the dungeon's boxes and Gott statue, their menus, the Fortune Wire and the trap room's doors

The user played `--mode story:4` and reported these problems:
- no treasure box was drawn;
- the Gott statue was missing;
- the locked room's doors played oddly;
- room changes were a little buggy;
- the minimap was lost after the stairs.

Event 4 (TEACH-D) now plays through its dungeon as a player plays it.

## The dungeon's objects

`WORLD_MAN::EntryGimmick`'s dungeon setters now run once per dungeon:
- `DUNGEON::SetItemBox` places the boxes;
- `DUNGEON::SetMagicCircle` places the portals;
- `DUNGEON::SetIDOL` places the idols.

piney-battle now has two of the objects' classes (`gimmick.rs`).
- **`ccGimBox`.** Its constructor rolls the trap. Its `main` runs
  `boxMain`, `objectMain` or `virusMain`; `invokeTrap` and `breakObject`
  are ported too.
- **`ccGimIdol`.** Its three clips: standing, opening, open.
- **The opening's rays.** They are `ccPrimRadiate` with
  `ccGimBoxRad::ctrl` (`prim.rs`).

piney-world does the rest:
- it draws the boxes and idols with their rows' palettes, and the rays on
  the effect layer;
- it carries a trap out on its opener;
- it keeps a dungeon's objects across its rooms, since every room is a
  scene change (`g_entryList`'s keep and `restoreEntry`);
- a box's or an idol's rays follow it to its new scene index.

**A trap went off without landing.** The damage roll saw the box already
off the command lists, so it always missed. In the game `invokeTrap`
runs before `deleteCmnd` in the same frame. The port now records the lists
as the call saw them.

## The menus that open them, and the Fortune Wire

The action button on a box, a trapped box or an idol asked for menus 32,
33 and 38. The field UI closed them at once, so no box but the tutorial's
could be opened. Four menus are now ported:
- **32, `ItemBoxMenu`:** the box's item, or a draw from the danger list
  or the box list.
- **33, `TrapBoxMenu`:** the trap goes off, then "Set off trap!" and the
  trap's line, then a dud.
- **38, `ItemIdolMenu`:** three items, through
- **67, `DataDrainSubMenu`.**

Nothing carried out an item used from the menus. `ccUseItemRequest` is one
blocking call on the menu task: it opens and waits on the menu's own
windows and breathes the task. In the port:
- the menu task stops at the call;
- the area applies `item::use_item_request`'s rules and answers with its
  steps in the same frame;
- the menu task plays them where the call stood, and hands the world's
  steps back one at a time (`useitem.rs`).

So the Fortune Wire disarms a trapped box as in the game: "Disarmed trap."
until the button, then the untrapped twin in the box's place. The
recovery items, the books and the ocarina's steps now reach the world as
well.

In event 4 itself the player never uses the wire: block 4 runs
`remove_trap` and then says "Orca used the Fortune Wire!". So two tests
cover the player's own path on a box put in the first room: the wire, and
the trapped box opened as it is.

## The trap room's doors

**How it looked.** In the port the gate's bars sank from shut into the
floor, then jumped back up.

**The event.** Block 6 runs `open_door`, waits a frame, then `close_door`.
It opens the doors so that the camera sees them shut.

**The game's order.** `ccThEntryCtrl`'s first slice comes before the
event's first pass that plays. That slice sets up the entry control,
including `ccEntryEventMng`, whose `CloseDoor` shuts the room's doors;
only then does it breathe. `ccSetupGameCtrl` starts the task in the same
slice as `ccEnableThEvent(4)`.

**The port's order.** It ran the set-up in the entry control's first loop
frame, after that pass, so the doors were shut again after `open_door`.
The set-up now runs a frame earlier (`FieldWorld::entry_setup`). The bars
come down from open over 46 frames with no jump, as in shots5's sequence.

## The rest

- **Room changes and the minimap.** Shot a frame at a time over three
  doors and the stairs, and compared with `GotoNextRoom`, `SetRoom`,
  `ClearRoom` and `DUNGEON::Draw`:
  - the old room is dropped while the fade runs, as the game does;
  - the party is placed and faces as in the game, and the camera resets;
  - floor 1's minimap is drawn;
  - the missing piece was the entry control's objects: nothing was kept,
    so each room's boxes and portals were lost.
- **Cameras.** A shot at each of event 4's camera instructions, all inside
  their rooms. Walked into and along the first room's four walls for 600
  frames: no frame has a wall between Kite's head and the camera.
  `cameraPosCalc`'s line test holds it 10 short of the wall.
- **Hosts.** area_host has these now:
  - `item_get_menu` and `item_get_menu_end`;
  - `remove_trap`'s effect;
  - `game_over` from `compulsionGameOver`;
  - `clear_gate_hack`;
  - the stream through `StreamPlayer::event` with the area's
    `StreamGame`.

## Checked

**eemu.** Each of these compares the port with the game's own code run in
eemu.
- **`tools/test_battle_spawn_rs.py` (all 6 tests pass):**
  - the boxes' and idols' native constructors and `main`s, with the
    virus crystal now inside the entry control's frames (`frame`);
  - `gim_frame`: 150 cases, 0 mismatches;
  - `frame`: 150 cases, 0 mismatches;
  - `circle_main` and `frame_enemies`: 150 cases each, 0 mismatches;
  - `SetItemBox` and `SetIDOL`: pass.
- **`tools/test_fieldui_rs.py` (36 tests):** new scenarios for menus 32,
  33 and 38 with 67 and 29. A mutated item count fails it.
- **The other suites:**
  - the fieldui personal, talk, trade, shop and option suites;
  - dungeon_rt and dungeon_rs;
  - world, foe, battle_enemy_motion, battle, battle_event and
    battle_items.

**Runtime.** `cargo test -p piney-game` runs these:
- `story_4_plays_through`, which covers:
  - the 12 legs, and the rooms in order;
  - each block's calls in its room;
  - the tutorial's boxes, and the disarmed twin in the trapped box's
    place;
  - Kite's HP unchanged through Orca's wire;
  - floor 1's minimap;
  - the dead end's box opened through 32;
  - the Gott statue opened through 38;
  - the save: the Resurrect and the spell one each, the idol's item,
    `itemBoxCount` 3 up, `itemIdolCount` 1 up;
  - the desktop, with no host call left at its default.
- `the_fortune_wire_disarms_a_trapped_box`.
- `a_trapped_box_opened_as_it_is_goes_off`.

**The rest.** The workspace's tests, clippy, fmt and the docs check pass.

**Still unknown:**
- **Palettes and sounds.** The doors' palettes for clutType 3 and 4
  (`ChangeClut`) and their sound ids.
- **Missing effects.** `effOpenTrapBox`, `effStatueOfGod`,
  `effVirusCrystal`, the breakables' `effCrush*` and the idol's dust ring
  are not in piney-effect.
- **Not ported yet:**
  - `EntryBreakObject`;
  - the objects' menus 34-37 and 39;
  - `DataDrainMenu` (66).
- **Item steps without a runtime:**
  - the map (`WaitMap` is 20 menu frames);
  - the Grunty ride;
  - the book viewer;
  - the epitaphs;
  - `DisableThEvent`.
- **The party AI's items.** Its `UseItemRequest` still does nothing in
  the field.
- **The exact frame.** Whether `ccThEntryCtrl`'s first slice runs in the
  frame of `ccEnableThEvent(4)` or the next is inferred from the task
  order, not measured. What is certain is that the slice comes before the
  event's first pass that plays: event 4's `open_door` / `close_door`
  depend on it.
