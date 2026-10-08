---
number: 406
title: The Event NPC's turned dummy lasts while the town's file does: a scene change to the same town keeps it, leaving reads the file afresh
date: 2026-10-08
area: world, engine, test
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/kept_dummies.rs, crates/piney-world/src/town.rs, docs/engine/field-game.md
---

# 406. The Event NPC's turned dummy lasts while the town's file does: a scene change to the same town keeps it, leaving reads the file afresh

[[397]] found that, for rows 177 and 179, `setMerchant` writes a new
rotation into the town's dummy (`DMY_marker_ev01`, `DMY_marker_ev02`).
The port kept the write "for the visit" (`Base::written`). This entry
asks the game how long the write lasts.

## What the game does

- **The write lives in the town file's stream.** The dummy chunk is the
  decoded file in memory, so the write lasts as long as that file's
  `ccStream` does.
- **`ccFileExistCheck`** (INF main 0x001640f0, filelib.cpp) runs over the
  previous scene's files (`dbgList`, `sceneFileListNumOld`):
  - a file the new scene's list also holds is marked kept (`exist` 1,
    counted in `existNum`);
  - any other has its `ccStream` destroyed (`~ccStream`, 0x00164288) and
    its slot named "NULL".
- **`ccFileListLoad`** (0x00164540) hands a kept file its old stream back
  (`dbgList` by name, `ccsPtr2` into `ccsLoad`, 0x00164bd4-0x00164c50) and
  decodes only the others.
- **So the dummy comes back exactly when the town's file is read again.**
  - Going to a field, a dungeon, another town or the desktop reads the
    file afresh, and the dummy is the disc's again.
  - A scene change to the same town keeps the file, and the turned dummy
    with it. The session already models this `loadCheck` case (an event's
    `scene -2` setting the same town up again).
  - The Chaos Gate offers no way into the same town (`GtTownMenu` leaves
    the current town out).
  - A crisis town is another file (`town01d`), so it is read afresh too.
- **It is visible.** `ccEntryEventMng` places the events' entries before
  `ccSetMerchant(0)` runs. So in a same-town reload, a character placed at
  marker 1 or 2 reads the kept rotation. The marker also stays turned
  after the Event NPC has gone (its status cleared, nothing writes it).

## Cause

The port opens the town afresh at every town scene (`World::enter`), so a
same-town scene change lost the written dummies.

## Fix

`Session::change_scene` carries the town's written dummies into the next
`World` when that scene's town file is the same one (the same stem). The
new town's entries are placed on its first frame of play, so they read
them. `Base::written` says so.

## Checked

- **`the_event_npcs_turned_marker_lasts_while_the_towns_file_does`**
  (piney-game, Mutation): a new game with ITEM COMPLETE's status set, in
  Carmina Gadelica (row 177, `DMY_marker_ev01`, `DEG2RAD(24576)`) and in
  the fifth town (row 179, `DMY_marker_ev02`, `DEG2RAD(-24576)`).
  - The Event NPC stands, and its marker reads the turned heading.
  - With the status cleared, a `scene` to the same town again: no Event
    NPC, and the marker still turned.
  - After Mac Anu and back, the marker is the disc's.
- With the carry removed, the test fails at the same-town step.
- piney-game's suite (four threads), clippy, fmt, `cairns check` and
  `tools/docs.py check` pass.

**Still unknown:** nothing.
