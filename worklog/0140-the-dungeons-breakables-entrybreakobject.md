---
number: 140
title: The dungeons' breakables: EntryBreakObject
date: 2026-09-26
area: world
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-battle/src/gimmick.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/dungeon.md, docs/engine/battle.md, docs/engine/effects.md
---

# 140. The dungeons' breakables: EntryBreakObject

A GAPS item. The breakables' own code (`ccGimBox::breakObject`, their
crush effects) and their guard were ported, but `EntryBreakObjectMain`
was not, so no dungeon room had anything to break.

## The game

`DUNGEON::EntryBreakObject()` (gcmn 0x005bff10) runs every time the party
enters a room.

- **The guard.** It places nothing in the lake types, in `game.field` 14,
  or in a story room with an event.
- **The calls.** Otherwise it calls `EntryBreakObjectMain` (0x005bfc40)
  five times, with the patterns `OBJ_0pr2*`, `OBJ_0pr4*` .. `OBJ_0pr7*`.
  `pr2` gets id 0. `pr4`-`pr7` get the four gimmick rows a jump table
  gives the dungeon's type (@4166, two copies of one four-row table).
- **Each object.** `EntryBreakObjectMain` walks the room anm's objects
  that match the pattern (`GetSubstAdrs`). For each it takes the world
  translation and draws `fieldrand(4)`, even when the id is fixed. It
  then makes a gimmick entry with `entRoot` 2 and `land` 0. Id 0 takes
  the random one of the four rows.
- **Leaving.** The entry control deletes gimmicks with that `entRoot`
  when the room is left, so they are made anew on each visit.

## The port

- **The dummies.** `DungeonArea::breakables_here(field, block)` applies
  the guard and lists the dummies in the calls' order.
- **The entries.** `combat::DungeonEntries` carries the list with the
  dungeon's type. `Combat::start_entries` makes the entries after the
  idols, on every room entry, with one `fieldrand` shared with the boxes.
- **The check.** `the_rooms_have_their_breakables` walks area 26's
  dungeon, counting only once each room plays (not during the change's
  fade). Rooms (0, 6), (1, 2) and (2, 1) get two each, and event 29's room
  (2, 2), a story room with an event, gets none.

The same pass removed a stale Unknown in effects.md: Skeith's death
effects (`BeginDeadEffect`, `ccBossEffDead`) are ported.

**Still unknown:** `DungeonEntries.rng` is a copy, so the dungeon's own
`fieldrand` state does not advance with these draws. The game's single
generator does advance, so later draws in the dungeon can differ. Not
compared with the game's pictures.
