---
number: 163
title: The Spring of Myst and the boss rooms' warnings: ccGimEtc and menus 40-42
date: 2026-09-26
area: ui
files: crates/piney-battle/src/gimetc.rs, crates/piney-fieldui/src/menus/fountain.rs, crates/piney-battle/src/gimmick.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-effect/src/gimmick.rs, tools/test_battle_spawn_rs.py, tools/test_fieldui_rs.py, tools/test_dungeon_rt.py
---

# 163. The Spring of Myst and the boss rooms' warnings: ccGimEtc and menus 40-42

The lakes' spring and the story dungeons' boss-room warnings were the
last `gimmickTbl` rows the dungeons place that the port did not make.
Both are `ccGimEtc` (gcmn gmetc.cpp). The spring also needs its three
menus. Details are in `docs/engine/battle.md` ("The spring and the boss
room's warning") and `docs/engine/field-ui.md` ("The spring (40 - 42)").

**The object.** `crates/piney-battle/src/gimetc.rs` covers:

- the constructor for rows 19-21;
- `initFountain`: `CMP_trall1` or `trall2` by area level, blend 4, two
  omni lights;
- `main`: `effFountain` or `effBossRoomEntrance` on the first frame;
- `ctrlFountain`'s twelve states.

`FountainMenu3`'s `EntryAffect(spring, plw, 11)` moves the states on. The
object also has the spring's rise, bounce and spring (the squash of
`scale1`), the flashes, the dust rings, `effRemoveTrap` and `effOpenBox`.
At the end, `SetFountain` records the area's code in the save.
piney-world draws it on layer 6 with `alphaBlendTbl[4]`, and needed one
change for that: `model_state` masked the blend to 3, so the draw takes
the table's entry directly. It lights the spring with its two lights.

`tools/test_battle_spawn_rs.py`'s `etc_frame` builds rows 19-21 with the
game's own constructor. Each spring gets a random state, count and set of
affects, then frames of `ccGimEtc::main`. All 150 scenes are equal.
Three fixes came on the way:

- The constructor was stubbed in the harness while the port built it.
- Etc's defaults were wrong: every word should be zero except `effsw`,
  and only row 20 has a clean-up.
- Row 19 with `param[2]` 0 takes a garbage path, so it is excluded.

The Grunties' commit (0162) took class tag 4 for the food, so the Etc
class is tag 5 in the probe and the harness. `SetMatrix_PosRotZYXScale`
is recorded only for a spring's animation.

**The menus.** `crates/piney-fieldui/src/menus/fountain.rs` has three
parts:

- 40 asks.
- 41 lists the bag's items of categories 0-9.
- 42 plays Monsieur's game. The spring rises, you answer Golden Axe,
  Silver Axe or Neither, and the item is looked up in
  `feTbl`/`f_limitTbl`. It changes by the lake's bgnum (+2, +1 or -1 a
  row, the other way round for armour), or both axes come back. 67 hands
  the items out.

Its pages draw the list and Monsieur's name plate. `SetFountainCamera`
runs after each frame of steps 2-14. The party hold and release go out as
one request each (`TalkReq::FountainParty`).

The fieldui harness's `test_spring_leave`, `test_spring_list` and
`test_spring_throw` are new. The first runs found six things, all fixed:

- `FountainMenuDisp2` passes the list's `strT`, which is null for 41, so
  the list has no tab.
- Triangle into 64 ends the frame there, without the item's help.
- The camera call is part of the frame's tail. After a breath it runs on
  the next frame.
- The name plate is on `settingKanji[0]` (+0x80), not `nameKanji`.
- Step 15 does not set `exceptionDisp` (its +0x10 write is `mapStatus`).
- The answers' message is `Open` with `saveData` as its name, so the
  window has the player's name above it.

The release's `manualSW` clears belong to the one request, so the harness
no longer reports them as `manual_off`s. All 54 fieldui tests pass.

**The warning.** `SetItemBox`'s kind-6 rows now make row 19. Each row's
room is built (`SetRoom`, then `DeleteRoom`), and `GetBanRoom` skips a
banned room without taking a counter number. `GetNearDoorPosition` moves
the row to the nearest gate dummy (else door dummy) within 12,000.
`entryObject` runs while `CheckBossEffect` holds, that is until the boss's
save bit is set. `DungeonArea::door_marks` and `near_door` port the
search. `tools/test_dungeon_rt.py`'s `test_near_door` builds rooms with
the game's `SetRoom` and gives each of the port's marks back as `in`.
All 338 marks came back unchanged. Random places near them gave 617
equal picks and 493 beyond 12,000. For a moment it looked as if the game
ran the search on rooms not yet built: at `EntryGimmick` only the start
room exists, and the rows sit on floors 2-4. `SetItemBox`'s own
`SetRoom` per row settles it.

piney-game's `the_spring_of_myst_takes_an_item` (session) enters a Δ lake
by night and walks Kite to the spring. A blade is thrown in and
"Neither" is answered. The spring rises, talks and goes, the area is
recorded, `fountainCount[0]` becomes 1, and the blade comes back one row
on. There are 603 workspace tests.

**Still unknown:** Row 21 (`ENTRANCE`, `effDungeonEntrance`) has no setter
in Infection, so it is made only by the harness. What row 19 with
`param[2]` 0 (the rays with `param[3]` as their user) was for; no setter
leaves it 0. `GetNearDoorPosition`'s result when every mark is beyond
12,000 is its stack's words, and the port keeps the row's place there.
