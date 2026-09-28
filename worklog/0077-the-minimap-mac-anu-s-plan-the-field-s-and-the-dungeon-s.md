---
number: 77
title: The minimap: Mac Anu's plan, the field's and the dungeon's maps, SELECT and event 3's map lesson
date: 2026-09-24
area: ui, world, test
files: crates/piney-world/src/map/mod.rs, crates/piney-world/src/map/sprite.rs, crates/piney-world/src/map/town.rs, crates/piney-world/src/map/field.rs, crates/piney-world/src/map/dungeon.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, tools/test_map_rs.py, docs/engine/map.md
---

# 77. The minimap: Mac Anu's plan, the field's and the dungeon's maps, SELECT and event 3's map lesson

The user asked where the minimap was. Event 3 needs it too: lines 20-25
teach it. An agent has ported it for the town, the field and the
dungeon. docs/engine/map.md has the detail.

## What is drawn

- **The town.** `ROOTTOWN01::DrawMap` draws Mac Anu's plan
  (`TEX_sr1map1`), the six shop signs of `RT01ICONPOS`, and the pulsing
  arrow (`xallow0`).
- **The field.** `WORLD::DrawMiniMap` and its six helpers draw over the
  height map that `Generate` paints into `fieldminimap`. SELECT switches
  between the Default Map, the Overall Map and off.
- **The dungeon.** `MakeMiniMap` scans each room with rays through the
  port's `ccHitCheckLM`. Then `DUNGEON::DrawMap` repaints the map,
  labels the floor ("B 1") and marks the stairs. `DUNGEON::ShowMap`
  reveals a floor.
- **The sprites.** `ccSprite`'s packets, flat and rotated, are ported
  under map/sprite.rs.

## How it is driven

- **The map button.** `ChangeMapMode` takes SELECT (`assignPAD` +0x840c,
  operation 13, the mode kept at save +0x842d-f). It runs before the
  world's frame, as `ccThGameCtrl` checks it before `ccPlayerMenuCheck`.
- **The fade.** The field UI's `SetMapAlpha` (`Request::MapAlpha`) now
  fades the map, and the map's `mapStatus` goes back to the UI.
- **The events.** `map_on` and `show_map` work from the event hosts. By
  event 3's line 21 the map is at full alpha with the entrance and the red
  arrow, and line 25's `show_map` sets `mapFlag`.

## Checked

- **Against the game.** `tools/test_map_rs.py` runs the game's own code in
  eemu beside `map_probe`. It compares every sprite's fields, the order
  they are sent, and the vertices decoded from the GIF packets:
  - the town: 95 frames, 1,456 packets;
  - field 14: 4,748 packets, and every texel of the painted height map;
  - 9 other field types: 40,998 packets;
  - 24 dungeons:
    - 206 rooms' scans;
    - `ShowMap` step by step on 24 second floors;
    - 288 `DrawMap` frames, with 2,594 packets and the repainted texture;
  - the modes: 144 `ChangeMapMode` cases, the alpha, and `ShowMap` for
    126 areas and the town.
- **Event 3** (runtime test): `event_3_shows_the_map`.
- **Re-run in a clean worktree.** The map, world, field, dungeon, field UI
  and merchant camera suites pass, with the workspace's tests, clippy,
  fmt and the docs check.
- **Shots.**
  - Mac Anu's plan with its signs, and SELECT turning it off.
  - Field 14's Default and Overall maps.
  - The dungeon's B 1.
  - Orca's "You see the Red Down Arrow on it?" with the arrow on the map.

**Still unknown:**
- **Portals, gimmicks and fountains are not fed in.** Their drawing is
  ported and checked, but the entry control that places them is not
  ported yet. So event 3's `show_map` finds no portals, and Fairy's Orb's
  "yellow areas" do not appear.
- **Not modelled around the button:** `menuClrWait`,
  `ccCheckGtHackAnm`, the wiped-out party, and `ccGame.inBattle`.
- **Unknown writers.** What sets `mapHideFlag` (+0x42c) and
  `WORLD_MAN.specialRoom` (+0x160) is not known; the port keeps 0 and -1.
- **Game globals kept per map.** `RT01ICONPOS` and the pulse angles are
  globals in the game but kept per map here, so a new area's pulse starts
  at 0.
- **Other towns.** Their `DrawMap` is not ported.
- **Not checked:**
  - the special dungeon types 8 and 9, and story rooms of type 15 and up;
  - the labels' drawing (only the strings are compared).
