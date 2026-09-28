---
number: 72
title: Into the field: Kite leaves Mac Anu for field 14 and its dungeon, the set-up and the walk checked against the game
date: 2026-09-23
area: world, test
files: crates/piney-data/src/area.rs, crates/piney-world/src/area.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/hit.rs, crates/piney-world/src/player.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, tools/test_field_rt.py, tools/test_area_rs.py, tools/test_dungeon_rt.py, docs/engine/field-walk.md
---

# 72. Into the field: Kite leaves Mac Anu for field 14 and its dungeon, the set-up and the walk checked against the game

[[69]] played event 2 to its scene change, and there it stopped: the
runtime had no field. An agent has now built the way out.
docs/engine/field-walk.md has the detail.

## What happens now

- **Out of Mac Anu.** Event 2's `area 14` and `scene` reach the session.
  It then:
  - fades Mac Anu out;
  - builds story area 14's `WORLD_MAN`, as the `area` instruction does on
    the town's server;
  - runs `ccSetupGameCtrl` for area 1.

  Kite arrives in Bursting Passed Over Aqua Field: a grass field under a
  pink sky, by the great horn.
- **The gate's warp.** The Chaos Gate's warp (`WORLD_MAN::SetGenerateCode`
  from the entered words) goes the same way.
- **Getting back.** Gate Out and the field menus' `ChangeArea` lead back
  to town. Log Out and the title leave a field as they leave the town.
- **The field.** Kite walks and runs on its height map, with the objects'
  collision coming and going with their draw. The field wraps at its
  edges. The field camera keeps above the ground (`avoidObstacle`, ported
  in [[61]]).
- **The dungeon.** The entrance is `WORLD_MAN::Enter`, which is
  `ChangeArea(2, 0)`. The dungeon is built (story area 14's is the
  hand-made `D0001`); doors go room to room, and the up stairs lead back
  to the field beside the entrance. `GoField` (86) leaves it too.
- **Test starts.** `--mode field:14` and `--mode dungeon:14` start there
  directly.

## Checked

- **The field against the game.** `tools/test_field_rt.py` lays story
  area 14's generated field out in eemu's memory as `WORLD::Generate`
  leaves it, then compares:
  - `SetCharPosition` for every field type;
  - `ccLandHitCheck` at 1,500 points;
  - the object passes (`WORLD::DrawObject`, the entrance and key) at 300
    places.
  - 2,620 frames of camera, player and objects: walking, the dungeon
    entrance, random pads, and running off all four edges. None differ.
- **The gate's words.** `tools/test_area_rs.py` checks `SetGenerateCode`
  for every story area's words and 300 random triples.
- **The dungeon.** `tools/test_dungeon_rt.py` checks the dungeon's walk.
- **Re-run in a clean worktree.** These pass, with the world, event,
  field, dungeon, field UI and top page suites. So do the workspace's
  tests, clippy, fmt and the docs check.
- **In the runtime.** Event 2's presses, played for 2,420 frames, end in
  area 1, field 14. The shot shows Kite by the horn with the HUD.

**Still unknown:**
- **The party is left behind.** Orca joins in event 2, but he is not
  brought into the field, and the HUD shows only Kite.
- **Event 3 does not start.** In the game the event task wakes after
  `scene` (the thread's no-sleep bit), runs `end_event`, and so closes
  event 2, which opens event 3 in the field. The port leaves the VM asleep
  inside `scene`. The agent is fixing both.
- **The field's entry control is empty.** No enemies, magic portals,
  treasure or field events yet; those come with the battle and effects
  work.
- **Drawing approximations.** Fog is taken per model, not per vertex. The
  water's run-time textures and the clouds' blending are approximated.
- **Story maps of their own** (an `EVENTAREA` file) are not built; only
  generated fields are.
- **Towns.** Only Mac Anu is ported, so Other Servers is refused.
- **The global generator.** `SimGenerateCode` and `GO` move it; the port
  leaves those draws aside.
- **The rest is listed** on the docs page.
