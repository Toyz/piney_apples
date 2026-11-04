---
number: 577
title: The Root Town as an AreaMap, held by the town's world as WORLD_MAN::GO(0) made it
date: 2026-11-03
area: world, build
files: crates/piney-world/src/town/mod.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/area/mod.rs, crates/piney-world/src/area/map.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/event/mod.rs, crates/piney-world/src/chara/party.rs, crates/piney-world/src/town/ride.rs, crates/piney-world/src/grunty/race.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-world/src/story_map/mod.rs, crates/piney-world/examples/world_probe.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, docs/engine/map.md, plans/rust-cleanup.md
started: 2026-11-03T16:34:47-07:00
took: 2h 18m
---

# 577. The Root Town as an AreaMap, held by the town's world as WORLD_MAN::GO(0) made it

The fourth step of "One world, one area system" (`plans/rust-cleanup.md`):
the Root Town is an `area::AreaMap` as the field, the dungeon and the story
map are ([[572]]), and the town's world holds it as `Box<dyn AreaMap>`,
made from its def by the registry (`TownDef::of`, `Town::build`) as
`WORLD_MAN::GO(0)` makes a `ROOTTOWN0n`. `World::town()` and `town_mut()`
are gone; nothing outside `town/` names the `Town` type but the session
tests' view of Dun Loireag's systems (through `<dyn AreaMap>::get`, a test).
Nothing drawn or played changed.

## What the town answers

- `core` / `core_mut`: its `AreaCore`, which every use of the town's file,
  collision and lights in the town's world now goes through.
- `start`: `WORLD_MAN::SetCharPosition`'s town case (INF main
  0x001a120c-0x001a13d4): Kite at the town's place (the def's, else
  `StartPos` as the scene before left it, `Town::keep_start`), and
  `StartPos[1..3]` at (+200, +100), (-200, +100) and (0, +300) from him
  facing as he does, the same arithmetic as a field's
  (`area::map::party_starts`; the game writes a slot only if its x is not
  -1.0, which no caller of the port's sets).
- `clear`: the def's `ccSys.bgColor`.
- `put`: `fieldrand` as its draws leave it and the scroll statics, what the
  town's `state_out` wrote by hand.
- `disp`: `ROOTTOWN0n::Draw` (its def's steps), the pieces painted, its
  `ccEff`s into `Disp.sprites` and what its steps started (the airship's
  smoke and hum) into `Disp.events`, as every area's. The world hands it
  the same `Disp` the field world builds; a town's entry control holds no
  enemies, so `ccCheckActiveObject` is true for every room, which no town
  draw reads.
- `marker_file`: the town's own file (`ccEvMarker2Pos`).
- `setup_map` / `minimap`: its `DrawMap`'s `TownMap`, made from the def's
  `MinimapDef` as a field's and a dungeon's map are, drawn through
  `map::area_frame` (`town_minimap`); the Flag Race's riders come in the
  `MapFrame` (`race`). The town's mode held the map before.

`written` (the dummies the Event NPC turned, [[406]]) moved from `Town` to
`AreaCore`: it belongs to the loaded file, whatever area holds it. The
town's number is `game.town` (`Scene::town`) wherever the world read the
def's `no`; the window's title takes the name from `TownDef::of`.

## Checked

The full harness on the four volumes (60 runs) as the baseline at b95f324;
the golden shots, 4548 files, none differ; the crates' tests and the
piney-game suite pass; clippy and fmt clean; too_many_arguments 141 (was
142: `town_frame` went), dead_code 0.

**Still unknown:** nothing
