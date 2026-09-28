---
number: 145
title: The fog by each vertex's depth, as VU1 writes it
date: 2026-09-26
area: render
files: crates/piney-draw/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-gs/src/lib.rs, crates/piney-desktop/src/soft.rs, crates/piney-world/src/draw.rs, crates/piney-world/src/body.rs, crates/piney-world/src/town.rs, crates/piney-world/src/gate.rs, crates/piney-world/src/foe.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/evarea.rs, crates/piney-world/src/evarea_b0.rs, docs/engine/render.md
---

# 145. The fog by each vertex's depth, as VU1 writes it

A GAPS drawing item. VU1 writes each vertex's F as `clamp(fogB + fogA *
w, fMin, fMax)` from `ccDrawEnv::SetFog`'s terms (render.md). The GS
then interpolates F across the triangle.

## Before

The port's draw list carried only one F per model.

- **Fields and dungeons** took each piece's F at its centre's distance.
  A whole room or ground tile was one shade: the far wall of a corridor
  no hazier than its near end.
- **Towns and story maps** had no fog at all. Mac Anu's orange-brown haze
  (`SetFog(1000, 7500, 0, 85, ...)`) and Dun Loireag's pale blue one
  (`SetFog(1500, 10000, 0, 75, ...)`) were missing, and so was the
  characters' fog everywhere.

## The change

- **The draw list.** `piney_draw::DepthFog` (fogA, fogB, fMin, fMax,
  FOGCOL; `DepthFog::set_fog` is `SetFog`'s arithmetic) is a new
  `ModelDraw::depth_fog`. A constant `fog` (`ccChar::Draw`'s blend) still
  wins over it, as `SetFogBlend` does.
- **piney-gs.** Each vertex gets its F (`GVertex::fog_f`), computed from
  its clip w, also for the vertices made by the near-plane cut. The
  shader interpolates it linearly in screen space under a new
  `Params::DEPTH_FOG` bit.
- **The software rasterizer.** Its vertices carry F and it interpolates
  it the same way.
- **piney-world.** `draw::Fogging` (none, constant, by depth) replaces
  the `Option<Fog>` of the model draws.
- **The draw environment.** `TownLights` gains the environment's fog.
  The town sets it from its class's `SetFog`, the field from its
  background row, the dungeon from the room's fog row (again at each
  room), and the story maps from their blocks' `SetFog`.
- **Who is fogged.** The characters' bodies, the enemies and the Chaos
  Gate fall back to the environment's fog when they have no blend of
  their own. The town's rows and objects, and the story maps' models
  and objects, are fogged. Their skies, clouds, background clumps and
  water are not, being the ones the game gives `SetFogSw(0)`.

## Checks

- All 572 workspace tests pass. The one draw-list comparison that fogs
  (Dun Loireag's) compares pieces, not F.
- Pictures: `dun_loireag_shots` shows the distant canyon wall and the
  balloon in the town's blue haze where they were sharp before. Kite, at
  a few hundred units, is unchanged. In `target_cursor_shot`'s small
  dungeon room the change is at most ten levels a channel.

**Still unknown:** no picture of the game's own was compared, and the
F values of the models are not checked against VU1 runs. Whether every
town object has PRIM.FGE set is taken from `SetFogSw` alone.
