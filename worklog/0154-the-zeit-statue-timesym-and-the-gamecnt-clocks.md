---
number: 154
title: "The Zeit statue: timeSym and the gameCnt clocks"
date: 2026-09-26
area: world
files: crates/piney-world/src/area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/area.rs, docs/engine/field-ui.md
---

# 154. The Zeit statue: timeSym and the gameCnt clocks

A GAPS item that asked whether "Chronicling" (word 131) can be had in
Infection. It can. The BBS's thread 29, "Zeit Statue", highlights it
("select Chronicling as your part A at the Chaos Gate"). Its dungeon ends
at the Zeit statue, which times the run from the gate and ranks it (menu
39, `TimeIdolMenu`, already ported with `CheckTimeIdolRankIn` and
`SetTimeIdolRank`). Two pieces were missing, so the statue was never
placed and would have read 0.

## timeSym

`SimGenerateCode` sets `WORLD_MAN.timeSym` (+0x134) when the first word is
131. `piney_battle::gimmick::set_idol` already makes every idol the
statue (row 44) under it, outside the lakes, but the dungeon handed it 0.
`WorldMan::time_sym()` now reads the first word, and `FieldWorld` passes
it to `DungeonArea::dungeon_gims`.

## gameCnt

`ccAddPlayTime` (main 0x00167740) adds the frame rate each frame to four
clocks, `ccGame` +0x64, +0x68, +0x6c and +0x70, each held at 999:59:59,
along with the play time; `gameCntStop` (+0x74) skips all of it.
`ccGame::ChangeRequest` (0x001671e0) zeroes [0] at every change, [1] when
`CheckSceneReplace` says the scene is new, and [2] when the area left was
a town or none (`areaPrev` 0 or -1). So [2] is the time since the gate
and survives the field and the dungeon's floors.

The port had the three-slot `game_cnt` in piney-fieldui's `Game`, but
nothing counted it. Now:

- `piney_world::area::Scene` carries `game_cnt: [i32; 4]`.
- `Scene::add_play_time(rate)` is `ccAddPlayTime`'s part.
- `change_scene` ends with `ChangeRequest`'s resets.
- The session counts on the scene a field or dungeon holds while it plays,
  else on its own. It is the one that goes on to the next area.
- The area's menus see `game_cnt` through `Game`.

## Checks

- `the_clocks_run_from_the_gate` (piney-world): the town's clocks, then
  the field ([0], [1] and [2] zeroed), the dungeon ([2] kept), a floor
  change (not a new scene: [1] kept), and the limit.
- `chronicling_leads_to_the_zeit_statue` (session): from Mac Anu, the
  gate's warp with Chronicling, Passed Over, Aqua Field. The field has
  `timeSym` and a clock running since the gate. Its dungeon's clock is
  further on, and its one idol is row 44.

**Still unknown:** the statue was not reached and opened in play (it
stands on the dungeon's last floor). The fourth clock (+0x70) has no
reader found yet.
