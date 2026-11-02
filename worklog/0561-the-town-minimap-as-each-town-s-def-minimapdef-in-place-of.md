---
number: 561
title: The town minimap as each town's def: MinimapDef in place of TownMap's per-town arms
date: 2026-11-02
area: world
files: crates/piney-game/src/world.rs, crates/piney-world/examples/map_probe.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/map/town.rs, crates/piney-world/src/town/carmina_gadelica.rs, crates/piney-world/src/town/dun_loireag.rs, crates/piney-world/src/town/fort_ouph.rs, crates/piney-world/src/town/lia_fail.rs, crates/piney-world/src/town/mac_anu.rs, crates/piney-world/src/town/mod.rs, docs/engine/map.md, plans/rust-cleanup.md
started: 2026-11-02T06:15:44-07:00
took: 36m
---

# 561. The town minimap as each town's def: MinimapDef in place of TownMap's per-town arms

The towns' next system after worklog 560: the minimap. `map::town::TownMap`
held `game.town` and branched on it in its constructor and its `DrawMap`
(eleven `match self.town` and `town == n` tests); each town's numbers are
now its def's. Nothing it draws changed.

## The def

`map::town::MinimapDef`, a `static` in each town's module, named by the
town's `TownDef::minimap` (None for `BASE_TOWN` and the sixth, whose
`DrawMap` returns at once; the game opens no map for it, as before):

| field | what | towns that differ |
| --- | --- | --- |
| `iconpos`, `tex`, `step` | `RT0NICONPOS`, the map's texture, `stepw` / `steph` | each its own |
| `race` | from Mutation on its own layers and the racers | all but Mac Anu |
| `later_only` | its own layers on any disc (a class the port takes from the later volumes) | 2-4 |
| `offset`, `racer_offset` | where the map lies in its texture, for the player and for the racers | Carmina Gade (16, 46) both; Fort Ouph (0, 16) the player's only |
| `mark_mask`, `mark_x`, `mark_y` | the racers' marks' `ccMask` and place (`MarkY`: 12 up, 12 down, level) | Fort Ouph down, Lia Fail `ccMask(6, 6)` 24 across and level |
| `arrow_base`, `arrow` | the arrow's base and where it draws (`ArrowPlace`: Mac Anu's on the map, Dun Loireag's in screen pixels off its own layers, the rest at the base) | |
| `odd_sign`, `sign_window`, `balloons` | Carmina Gade's turned sign 3, Lia Fail's signs only within the 160 rows, the balloons centred always (Carmina Gade) or outside the race (Fort Ouph, Lia Fail) | |
| `dim` | the box about a sign the arrow's point dims it within | Dun Loireag (0, 54, 10, 49); Fort Ouph and Lia Fail (8, 46, 10, 45) |
| `hold_fade` | the signs hold their fade while the race runs | Fort Ouph |

The constructor's own-layer test (`later && town != 0 || town >= 2`) is
`later && race || later_only`, the draw's (`later && town != 0`) `later &&
race`, Infection's Dun Loireag's font-layer arrow `ArrowPlace::
ScreenOffOwnLayers` off its own layers. Offsets of 0 are not added, so no
sum changes its zero's sign. The game opens the map from the town's def;
`map_probe` finds the def by the file it is asked for; `TownMap::new`
(Mac Anu's) is gone for `open` with the def.

## Checks

- Golden shots as for worklog 560 (towns 0-4 on Infection and Outbreak,
  town 5, the flag race's, the shrine map's): every PNG byte for byte.
- Against the game on all four volumes: `test_map_rs`, `test_race_rs`,
  `test_grunty_rs`.
- piney-world's and the other crates' tests, piney-game's suite (release);
  `piney-gen gen --check`, clippy with all features, fmt, `cairns check`,
  `tools/docs.py check`.

**Still unknown:** nothing.
