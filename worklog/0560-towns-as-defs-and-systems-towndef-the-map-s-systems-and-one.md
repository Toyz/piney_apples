---
number: 560
title: Towns as defs and systems: TownDef, the map's systems and one runner, every Root Town made of them
date: 2026-11-02
area: world
files: crates/piney-data/src/classes.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/dun_loireag.rs, crates/piney-game/src/session/tests/kept_dummies.rs, crates/piney-game/src/session/tests/mirror_world.rs, crates/piney-game/src/session/tests/ranch_music.rs, crates/piney-game/src/session/tests/shop_keeper.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/side_events.rs, crates/piney-game/src/session/tests/town06.rs, crates/piney-game/src/session/tests/town_walk.rs, crates/piney-game/src/world.rs, crates/piney-gen/src/classes.rs, crates/piney-world/examples/carmina_gadelica_probe.rs, crates/piney-world/examples/dun_loireag_probe.rs, crates/piney-world/examples/grunty_probe.rs, crates/piney-world/examples/merchcam_probe.rs, crates/piney-world/examples/world_probe.rs, crates/piney-world/src/area.rs, crates/piney-world/src/chara/party.rs, crates/piney-world/src/event/mod.rs, crates/piney-world/src/grunty/race.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/story_map/sky.rs, crates/piney-world/src/story_map/truths_core.rs, crates/piney-world/src/town/carmina_gadelica.rs, crates/piney-world/src/town/dun_loireag.rs, crates/piney-world/src/town/fort_ouph.rs, crates/piney-world/src/town/lia_fail.rs, crates/piney-world/src/town/mac_anu.rs, crates/piney-world/src/town/merchant.rs, crates/piney-world/src/town/mod.rs, crates/piney-world/src/town/relic_city.rs, crates/piney-world/src/town/ride.rs, crates/piney-world/tests/dun_loireag.rs, crates/piney-world/tests/world.rs, docs/engine/evarea.md, docs/engine/field-game.md, docs/engine/root-towns.md, docs/engine/statics.md, docs/engine/town02.md, plans/rust-cleanup.md, crates/piney-world/src/systems/
started: 2026-11-02T04:14:03-07:00
took: 2h 1m
---

# 560. Towns as defs and systems: TownDef, the map's systems and one runner, every Root Town made of them

Section 2's towns item (`plans/rust-cleanup.md`, "Towns: `step_scrolls`,
`Sky` ..."), built the way the bosses were (worklog 537-556) and the way
the maintainer's rule asks: a town is a definition naming its systems and
its `Draw`'s order, each system a real object, its draws generic requests.
Nothing a town computes or draws changed.

## The def

`town::TownDef`, one `static` a town (a def is its address), each
`BASE_TOWN` with its differences: `no`, `class` (`TownGameClass`, now
generated beside `BossGameClass`: `ROOTTOWN` and its six), `name`,
`volumes`, `file` and `crisis_file`, `tables` (Infection's for the first
five on every disc, Quarantine's for the sixth), the light animation and
table, fog, clear colour, `start` (`SetCharPosition`'s place, None for the
sixth: `StartPos` keeps the scene before's), `systems` and `order`.
`town::TOWNS` is `GO(0)`'s switch; `Town::build` reads the `AreaCore`
(`AreaCore::read`, which the story maps on the old `Base` use too) and
builds each `SystemDef`. `fieldrand` is held when one of its systems draws
from it (`SystemKind::FIELD_RAND`: clouds, the airship, the glows), as the
four towns that set it did.

## The systems (`piney_world::systems`)

| system | what | towns |
| --- | --- | --- |
| `Models` | `STATICMODEL`s of a table, on its layer | all |
| `Objects` | `STATICOBJECT`s, stepped within their clip | 01, 02, 04 |
| `Scrolls` | `DrawBG`'s scrolls, `take` / `put` from their slot | all |
| `Sky` | `DrawBG`'s clumps: at the origin or a dummy, on a layer (Dun Loireag's cloud layer by volume), materials offset by a scroll (`MatScroll`: U, V or both); the crisis clumps among them, made only in crisis | all |
| `Water` | `WaterDef`: its file, anim, model, reach, root, `WaterSpec`, slot, the waters drawn, their layer, the copy | 01, 02, 03 |
| `Clouds` | `CLOUD`s, made by the first `Draw` | 02, 04 |
| `LensFlare` | `LENSFLARE` at its sun's dummy | 02, 04 |
| `MacAnu` | Mac Anu's `Clip`: `clip[3]` and the rows and objects `DrawObj2`, `DrawObj`, `DrawFloor` draw by it | 01 |
| `Airship` | Carmina Gade's `AIRSHIP` | 03 |
| `Glows` | Lia Fail's omni points' glows | 05 |

The plan's `Sky` for "the clouds and the flares" is the game's two classes,
`Clouds` and `LensFlare`; `Sky` is `DrawBG`'s clumps with the crisis sky.
Mac Anu's ships are its type-0 static objects; Dun Loireag's ranch is the
world's Grunties, which its `Draw` never touches.

A step is `Run(kind, hook)` (`Draw`, `Start`, `Clip`, `Pass(pass)`,
`Rows`, `PassBelow`), `Map` (`DrawMap`) or `Seq`: `PASSES` (`DrawObj2`,
`DrawObj`, `DrawFloor` with their objects, the old `Base::passes`) and
`DRAW_BG`. A town without a step's system has nothing to step (Lia Fail's
passes have no objects). Each town's order is its `Draw`'s, for example
Carmina Gade's:

```text
Water (0, the copy), Airship, Scrolls, Sky, Models(Obj2), Models(Obj),
Models rows 0-7, rows 29-33 without fog, type-0 rows below 29 without
fog, Map
```

## The runner and the requests

`Town::step(view)` walks the order; a system steps with the core (file,
`fieldrand`, the volume), the view, the frame and the events, and reaches a
sibling only by kind (`Others`: the sky reads the scrolls, Mac Anu's clip
the models and objects). What it shows goes into the frame as
`Drawn::Piece(kind, piece)`; `Town::requests` turns each into a generic
`Request` (models at a matrix, a `ccAnm` with an edited model, a sprite)
and the world paints them with `systems::paint`, the same for every town.
Tests and probes read a system by kind (`Systems::get::<T>()`). Gone:
`Base`, `Spec`, `RootTown`, each town's struct, `Piece`, `select` and
`draw_pieces`, `Town::class` and `parts_mut`, `water0_draw`,
`crisis_rows`, and the town-number arms of `town_start`, `town_ported`
and the game's title (`TownDef::name`).

## Found on the way

`tools/eemu_rs.so` predated worklog 557's `grow`, so `test_map_rs` and
`test_dungeon_rt` failed on every volume at HEAD; rebuilt from
`piney-eemu --features python`.

## Checks

- Golden shots, recorded at 9025c45 and after, every PNG byte for byte:
  `town_shots` of towns 0-4 on Infection and Outbreak and town 5 on
  Quarantine, `pc_menu_shot`, `dun_loireag_shots`, `grunty_shots`,
  `mac_anu_chat_shots`, `ride_shots`, `flag_race_shots`,
  `breeder_race_shot`, `mirror_world_shots`, `staff_roll_shots`,
  `hopeless_nothingness_shot`, `event_11_shots`, `shop_keeper_shot`,
  `shrine_map_shots`, `gate_hack_arrival_shots`, `tolerance_shots`.
- Against the game, on all four volumes, as at 9025c45: `test_town02_rs`,
  `test_town03_rs`, `test_world_rs`, `test_lights_rs`, `test_gate_out_rs`,
  `test_party_rs`, `test_grunty_rs`, `test_merchcam_rs`, `test_race_rs`,
  `test_map_rs`, `test_evarea_rs`, `test_field_ambient_rs`,
  `test_field_rt`, `test_dungeon_rt`, `test_area_rs`.
- piney-world's, piney-data's, piney-desktop's, piney-battle's,
  piney-effect's and piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  149, one fewer; no `dead_code` allows), fmt, `cairns check`,
  `tools/docs.py check`.

**Still unknown:** nothing. Left for the next steps: the minimap's per-town `DrawMap` arms (`map::town::TownMap`) are the next system, then the story maps' and fields' clouds, flares and scrolls on these systems.
