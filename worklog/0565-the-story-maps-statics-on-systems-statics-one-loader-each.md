---
number: 565
title: The story maps' statics on systems::statics: one loader, each class's rules in its defs
date: 2026-11-02
area: world
files: crates/piney-world/examples/carmina_gadelica_probe.rs, crates/piney-world/examples/dun_loireag_probe.rs, crates/piney-world/examples/story_map_probe.rs, crates/piney-world/examples/world_probe.rs, crates/piney-world/src/story_map/abyss.rs, crates/piney-world/src/story_map/boss_arena.rs, crates/piney-world/src/story_map/disc.rs, crates/piney-world/src/story_map/giant.rs, crates/piney-world/src/story_map/holy_ground.rs, crates/piney-world/src/story_map/sky.rs, crates/piney-world/src/story_map/truths_core.rs, crates/piney-world/src/systems/statics.rs, crates/piney-world/src/town/carmina_gadelica.rs, crates/piney-world/src/town/dun_loireag.rs, crates/piney-world/src/town/fort_ouph.rs, crates/piney-world/src/town/lia_fail.rs, crates/piney-world/src/town/mac_anu.rs, crates/piney-world/src/town/relic_city.rs, docs/engine/statics.md, plans/rust-cleanup.md
started: 2026-11-02T11:33:58-07:00
took: 1h 53m
---

# 565. The story maps' statics on systems::statics: one loader, each class's rules in its defs

The map's own systems item (`plans/rust-cleanup.md` section 2): holy
ground (`EVENTAREA02`), the giant (`EVENTAREA07`), the abyss
(`EVENTAREA03`), the boss arenas (`EVENTAREAB0`) and the disc
(`EVENTAREAB8`) built their scenery with `holy_ground::StaticModel::load`
and `StaticObject::load` (worklog 531), which differed from the towns'
`systems::statics` in four ways. Each is settled from the game's code;
there is one loader, its rules in the defs. Nothing drawn changed.

## The four differences, from the game

- **A model the file lacks.** `STATICMODEL::STATICMODEL` (INF gcmn
  0x005cffd0) takes its model with `ccStream::GetChunkAdrsF(name, 0)`
  (main 0x00144570), which, finding no chunk and told not to return null,
  stores to address 0: the game stops. The towns' loader skipped such a
  row (`ModelTable::place`), the story maps' failed. One rule now: an
  error, for the towns too (every town and story map builds).
- **Origin or dummy.** The constructor itself places a row: postype 1
  copies the dummy's position, 2 its position and turn, 0 leaves zeros.
  `Placing::Origin` was `EVENTAREA07`'s, whose rows are all postype 0
  (`EA_MODELTABLE07` at 0x005d2160, `EA_OBJTABLE07` at 0x005d2200): the
  same zeros. No rule is needed; `Placing` is gone.
- **The names.** `EVENTAREAB0`'s constructor writes the field's two
  digits over characters 6 and 8 of `EA_MODELTABLEB0`'s `se1_5` names:
  its `ModelsDef::names` is `Names::OfFile { from: "se1_5" }` (the table's
  names read as the stage file's), every other def's `Names::Table`.
- **The row cap.** A class builds `anmNum` objects: `EVENTAREA02`'s 0 and
  1 (the first table's placeholder row never built), the others their
  whole tables: `ObjectsDef::anm_num` (None, every row). `STATICOBJECT`
  makes a clump in place of the `ccAnm` for a row naming no animation;
  the loader skips such rows, as the towns' did, and
  `every_built_object_row_names_an_animation` checks over all four
  volumes' tables that no class builds one, so none is lost.

## The defs and the systems

`systems::statics::{ModelsDef, ObjectsDef}` gain `names` and `anm_num`.
`StaticModels` keeps each row's `Model` (row, pass, model, place, clip:
the old `StaticModel`), `StaticObjects` its `Object`s. Each story map
names its own `static` defs: `holy_ground::MODELS` and `OBJECTS` by block
and `giant::MODELS` by block (Infection's tables, which those classes read
on every volume), `giant::OBJECTS` (block 0; block 1 builds none,
`StaticObjects::default()`), `abyss::MODELS`, `boss_arena::MODELS`,
`disc::MODELS` and `OBJECTS`. The maps hold the system objects (`models:
StaticModels`, `objects: StaticObjects`) and index them as before;
`StaticModels::clear` is `EVENTAREAB0::NextStage`'s delete. The story maps
as defs with their systems in order (the next steps) move them into each
map's `Systems`.

## Checks

- Golden shots, every PNG byte for byte: the towns' set and the story
  maps', fields', dungeons' and breaths' (worklog 562, 563).
- Against the game on all four volumes, as for worklog 562
  (`test_evarea_rs` among them: every story map's pieces and places).
- The crates' tests, piney-game's suite (release); `piney-gen gen
  --check`, clippy with all features, fmt, `cairns check`, `tools/docs.py
  check`.

**Still unknown:** nothing.
