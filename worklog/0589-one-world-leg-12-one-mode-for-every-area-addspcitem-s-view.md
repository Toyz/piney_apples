---
number: 589
title: One world, leg 12: one mode for every area, AddSpcItem's view, event_11_warp bisected
date: 2026-11-13
area: world, engine, decomp, test
files: crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-world/src/world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/event/mod.rs
started: 2026-11-13T22:55:04-07:00
took: 1h 4m
---

# 589. One world, leg 12: one mode for every area, AddSpcItem's view, event_11_warp bisected

Leg 12 of the relay on the plan's "One world, one area system" item
(branch `one-world`, after [[588]]). Item 6 is done: a Root Town, a
field, a dungeon and a story map run in one mode (`AreaMode`,
`Stage::Area`); `WorldMode` and `Stage::World` are gone. The next leg is
the last (full checks, goldens, docs, plan text).

## AddSpcItem's view

Leg 11 left open whether a town's `AddSpcItem` may get the area's
game-only view. `ccMenuCtrl::AddSpcItem` (INF gcmn 0x00527950) reads
nothing of the area; `AddSpcItemSub` (0x00527d80) reads `game->area`
(+0x14) at 0x00527e0c and 0x00528004 (the town's half-price sale), the
rest is the save and the price tables. On MUT, OUT and QUA the same two
functions load only `game` of the globals. The view's ccGame (status 5,
the scene) is all the game reads; nothing changed but the doc.

## One mode

Each difference between the two modes went to the game's own test or to
the one path every area takes:

- One World entry: `World::enter` builds a Root Town for area 0
  (`World::town`) on WORLD_MAN as the last area left it (GO(0) sets
  hackFlag), with the set-up's ccInitRand, the registry and party, the
  faded start and `Arrival.before` (the StartPos a town with no start
  keeps); the area's map is made there for every area.
- The menu task's constructor counts the grown Grunties on
  `game.server` only when `game.area` is 0 (MUT gcmn 0x0053a374-0x0053a3c0:
  `lw game; lw +0x14; bnez`, then ccPgAdultCheck(game+0x1c, k)).
- The menu task sleeps from any ChangeRequest (ccSleepNoSleepThread(1, 1)),
  the town's guard for every area.
- One ccEntryEventMng flow: ccRegisterEventMng's registNpcNum after pass 0
  (every area's file list calls it, INF main 0x00169690, 0x001697a0,
  0x001698b0); after pass 2 the cast takes its own (a town's NPCs, types
  3-4), the party's entries go to the registry for Reboot's
  `place_entry`, the rest to the entry control. The town's own spc entry
  path (and its post-set-up build, a stand-in the game does not have) is
  gone; the enemies' looks load only for a field's or dungeon's list
  (ccSetFileListTown has none).
- One take_events (the town's SqLoad(2) mapping gone: the area's
  AreaMusic with area 0 is the town row), one view (`menu_world`: battle
  characters as the battle has them, the cast's through
  `World::char_info`), one talk, one menu_request (the union; GtHackMenu's
  OK is `World::set_gate_hack`, which the session reads at the leave).
- Scene sound's town is `World::town_type()`, WORLD_MAN::GetTownType():
  game.town while a town is built. ChangeArea writes ccGame at once, so a
  town that has asked to leave already reads the next scene's numbers;
  the world keeps the area GO built.
- The session: one `enter_world`, one `leave_area` (gtHackFlag,
  setupMode, blur statics, WORLD_MAN, the kept dungeon or story map, the
  registry, and ccFileExistCheck's kept file for any area); `Event::Go`,
  `Event::GateHacked` and `Pending::Go` gone.

## Checks

piney-world release all pass; piney-game suite 345 passed, 0 failed, 152
ignored (one test fewer: it tested the town's `member()`, gone with its
view); clippy clean on both crates. The thirteen harnesses on all four
volumes: 52/52 rc=0, every line as leg 11's. The town shot subset
differs from leg 11's l11c in 21 of 51 files by 1-5 pixels, 1 in one
channel; leg 11's own tip shot today gives the same 21, and this leg's
shots equal it byte for byte (51/51). The renderer's environment moved
those LSBs between 2026-11-04 and today: shots compare only within a day.

## event_11_warp_shots bisected

Base and leg 9's tip shot today: 25 of 53 differ, as [[588]] counted.
23 (frames 0-110 after Kite's act 12) come from 9c23fd0 (leg 3: the
town's menu_ban through the one world's). That commit with only
MenuClr's two-frame wait skipped in towns gives none. ccEvent::MenuClr
(INF main 0x001b25c0) stores menuClrWait = 2 at 0x001b2788 with no area
test (its only writer): after event 11's menu_ban off ccThGameCtrl waits
two frames in Mac Anu too, so Kite's TransferOut (the shots' clock)
comes two frames later against the gate's own turning. The other 2
(0255, 0260: area 15's warp-in ring) come from 3b67918 (leg 4, One
Kite): the clock is the one ccPlayer's act 12, whose leave runs to the
scene change on its own count. Both are the game's.

**Still unknown:**
- the exact frame offset One Kite moved between Kite's act 12 in Mac Anu and area 15's arrival (event_11_warp 0255, 0260; bisected to 3b67918, not measured)
- whether the tests' tuple patterns (`(Stage::Area(a), ..)`) want the town test the other patterns got (they pass as they are)
- enemies lit with this frame's lights, where the game lights them before the boss (66) and ccThEffect (80) move theirs ([[588]]'s note, unchanged)
