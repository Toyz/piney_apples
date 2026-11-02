---
number: 548
title: Tarvos's drain plays its own movie: a boss's Affect runs inside EntryAffect
date: 2026-11-01
area: battle, video, ui
files: crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field/world.rs, crates/piney-fieldui/src/menus/drain.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/tarvos.rs, crates/piney-game/src/session/tests/player_logs.rs
started: 2026-11-01T21:40:15-07:00
took: 26m
---

# 548. Tarvos's drain plays its own movie: a boss's Affect runs inside EntryAffect

Issue #79 (QUA, build e70f6a8): during Data Drain on Tarvos the movie showed the
bracelet's orb and Kite but no Tarvos, before and after the drain fired. The
player's pad log, replayed through the launcher (`player_log_replay`, row 3),
played stream 117 at frame 19805.

## The cause

QUA `ccMenuCtrl::DataDrainMenu` (gcmn 0x0043ede0) sends a boss
`EntryAffect(21)` from Kite (0x0043f020) and then, in the same frame, picks the
movie by the target's base: `type & 0x60` an enemy by its size, anything else by
`base->id` (`lh 12(base)`, 0x0043f0dc): 0-7 give 18, 37, 44, 67, 69, 96, 100,
106, any other 117. Only an enemy gets `ccThDrainEnemy` (0x0043f260), so for a
boss the stream itself must hold the body.

Tarvos's guard is `SetDefMode` (0x003a9040): modes 0 and 1 store
`ccGetBossParam(42)` or `(43)` in `ccChar+0` (0x003a9070, 0x003a9108), the
base, whose ids are 42 and 43 (`bossTbl` read from QUA gcmn 0x005368a0: rows 42
and 43 are "Tarvos", type 0x80). `EntryAffect` (0x0047f170) calls the affect
function at once (`jalr 148(a0)`, 0x0047f2e0); for a boss that is
`bossAffectFunc` (0x00362c70), the vtable's `Affect`. Tarvos's `Affect(21)` is
its drain: `SetDefMode(2)`, `SetBaseParam(row 6)`. So in the game the base is
row 6 again when the menu reads it, and Tarvos plays 100, its own movie.

The port queued every affect that reached a boss from outside its frame (no
`BossEnv` in the menus' context) and ran it at the boss's next task, after the
movie, since the menu sleeps the world. The menu also read the id from its
snapshot of the target. Both made 42 or 43, so 117, a large enemy's stream
with no one in it.

## The fix

`Combat::entry_affect_with` now gives the affect context the fight's
`BossEnv` (its clips, the fight's `ccRand`, and `Combat::env`, the globals
the caller leaves: `FieldWorld::entry_affect` fills it from the save's plcol,
the menu's forbid and type, the count and the area). The boss's `Affect` runs
inside the call, as `bossAffectFunc` does. The menu's movie is now
`Request::DrainMovie(Movie)`: an enemy's stream, or `Movie::ByBaseId`, which the
runtime turns into a stream after the affect before it (`Movie::stream`).

`tarvos_drained_plays_its_own_movie` drives Kite through PERSONAL, Skills, the
Data Drain page and Tarvos in event 310's arena with the guard up, and asserts
`drain_movie 100`; it failed with 117 before. The player's log now plays 100,
with Tarvos's body in the frames before and after the drain. The fieldui
harness's drain tests (`tools/test_fieldui_rs.py -k drain`) still agree.

The replay helper takes `PINEY_LAUNCHER_ROW` for logs begun from the console's
`pad_log`, which keep none of the launcher's titles.

**Still unknown:** the movie's pictures (stream 100's Tarvos) are not compared with the game's frame by frame
