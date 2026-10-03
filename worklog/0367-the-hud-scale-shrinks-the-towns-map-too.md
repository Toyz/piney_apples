---
number: 367
title: The HUD scale shrinks the towns' map too
date: 2026-10-03
area: ui
files: crates/piney-world/src/map/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs
---

# 367. The HUD scale shrinks the towns' map too

The HUD scale (not the game's, [[354]]) was reported not to touch the
map. [[354]] had shrunk only the field's and the dungeons' minimap:
`area.rs` marked the layers before `map::area_frame` and shrank what it
drew. The towns' map (`map::town_frame`, from `world.rs`) was drawn at
full size.

The shrink now lives with the maps. `MapState::hud_scale` holds the scale
(each mode sets it from its field UI each frame). `town_frame` and
`area_frame` both mark the layers, draw, and shrink what they drew toward
the screen's top right (`map::shrink_toward`, moved from `area.rs`). At
1.0 nothing changes: `tools/test_map_rs.py` passes.

`the_hud_scale_shrinks_the_town_map` (piney-world) draws Mac Anu's map at
1.0 and at 0.5. The 0.5 frame is the 1.0 frame with every primitive
shrunk toward (512, 0), and the two differ. Before, they were the same.

A doc comment in `area.rs` that had slid off `handle_index` onto the
shrink is back on its function.

**Still unknown:** nothing.
