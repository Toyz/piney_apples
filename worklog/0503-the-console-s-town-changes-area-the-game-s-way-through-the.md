---
number: 503
title: The console's town changes area the game's way: through the scene's ChangeArea, its event task disabled, so the music holds between towns
date: 2026-10-26
area: audio
files: crates/piney-game/src/session.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, BUGS.md
started: 2026-10-26T00:11:24-06:00
took: 11m
---

# 503. The console's town changes area the game's way: through the scene's ChangeArea, its event task disabled, so the music holds between towns

BUGS.md: the music breaks between towns after the console's `town`. The
session's own events gave each town its music on field -> Dun Loireag ->
Mac Anu -> Dun Loireag, so the report was left open. It was never told
what was heard or where the jump started.

**Every way the jump can go.** The `town` command and the game's own
`ChangeArea` were compared side by side.
- **The game's.** Every in-game `ccGame::ChangeArea` goes through
  `ChangeRequest` (main 0x001671e0), which calls `ccDisableThEvent`: the
  event task idles from the next frame until the next set-up's
  `ccEnableThEvent(0)`. The port's menus do the same:
  - in a town, `WorldMode`'s `ChangeArea` request disables the VM and
    marks the mode as leaving;
  - in a field or dungeon, `FieldWorld::change_area` asks for
    `ChangeScene`; `AreaMode` then disables the VM and puts every task
    but the event's to sleep (`ccSleepNoSleepThread`).
  - Gate Out goes these ways.
- **The console's (the fault).** `town` went straight to `Session::go`.
  The scene's event task kept running through the fade and the hold, and
  a field's tasks did not sleep. A town's or field's next event blocks
  could start under the change, their sound and music instructions among
  them.

**The fix.** `town` now asks the scene itself:
- in a town, `WorldMode::change_area` (the menu's handling, factored out
  so both use it);
- in a field or dungeon, the new `AreaMode::change_area`, which calls
  `FieldWorld::change_area`, as Gate Out does;
- elsewhere it answers as before.

**Checked.** `town_jumps_keep_the_music` now takes every way between
Infection's towns:
- from the field to Dun Loireag;
- to the town it is already in;
- to each other town (Mac Anu, Carmina Gadelica, and back);
- a second `town` 20 frames into an arrival.
Each town plays its music afterwards: rms 2556 for Dun Loireag, 5361 for
Mac Anu, 5544 for Carmina Gadelica, with the sequences playing. Every
`town` test and piney-game's suite (336) pass.

Docs: BUGS.md.

**Still unknown:** nothing
