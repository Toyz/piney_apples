---
number: 569
title: Fields as defs: FieldDef by field type, WORLD's pictures and background as systems on the runner
date: 2026-11-02
area: world
files: crates/piney-game/src/area.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/fx/ambient.rs, crates/piney-game/src/loaddisp.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/world.rs, crates/piney-world/examples/ambient_probe.rs, crates/piney-world/examples/carmina_gadelica_probe.rs, crates/piney-world/examples/dun_loireag_probe.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-world/examples/field_probe.rs, crates/piney-world/examples/foe_probe.rs, crates/piney-world/examples/map_probe.rs, crates/piney-world/examples/story_map_probe.rs, crates/piney-world/examples/world_probe.rs, crates/piney-world/src/area.rs, crates/piney-world/src/combat/breath.rs, crates/piney-world/src/combat/gate_out.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/dungeon/dress.rs, crates/piney-world/src/dungeon/lake.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-world/src/field/ambient.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/field/def.rs, crates/piney-world/src/field/firefly.rs, crates/piney-world/src/field/mod.rs, crates/piney-world/src/field/world.rs, crates/piney-world/src/field/world/ride.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/sky/firefly.rs, crates/piney-world/src/story_map/abyss.rs, crates/piney-world/src/story_map/boss_arena.rs, crates/piney-world/src/story_map/disc.rs, crates/piney-world/src/story_map/giant.rs, crates/piney-world/src/story_map/holy_ground.rs, crates/piney-world/src/story_map/mod.rs, crates/piney-world/src/story_map/sky.rs, crates/piney-world/src/story_map/truths_core.rs, crates/piney-world/src/systems/bird.rs, crates/piney-world/src/systems/fireflies.rs, crates/piney-world/src/systems/fires.rs, crates/piney-world/src/systems/flare.rs, crates/piney-world/src/systems/haze.rs, crates/piney-world/src/systems/lake.rs, crates/piney-world/src/systems/mod.rs, crates/piney-world/src/systems/rain.rs, crates/piney-world/src/systems/scrolls.rs, crates/piney-world/src/systems/sky.rs, crates/piney-world/src/systems/smoke.rs, crates/piney-world/src/systems/snow.rs, crates/piney-world/src/systems/steam.rs, crates/piney-world/src/systems/tobj.rs, crates/piney-world/src/town/carmina_gadelica.rs, crates/piney-world/src/town/dun_loireag.rs, crates/piney-world/src/town/fort_ouph.rs, crates/piney-world/src/town/lia_fail.rs, crates/piney-world/src/town/mac_anu.rs, crates/piney-world/src/town/mod.rs, crates/piney-world/src/town/relic_city.rs, docs/engine/effects.md, docs/engine/field-walk.md, docs/engine/field.md, docs/engine/sound.md, docs/engine/town02.md, plans/rust-cleanup.md, tools/test_field_ambient_rs.py
started: 2026-11-02T20:39:53-07:00
took: 2h 39m
---

# 569. Fields as defs: FieldDef by field type, WORLD's pictures and background as systems on the runner

Section 2's map systems item, step 3's first half (`plans/rust-cleanup.md`):
a field was `FieldArea` with an `Ambient` that made every picture
inline by field type (`ty == 0`, `ty == 7`, `ty == 9`, `ty == 10`, `ty <=
3`) and handed a list of `Op`s to the field, which drew the models and
the background itself (`ambient_models`, `bg_draw`) and the sprites and
puffs through the effects. Now each field type is a `FieldDef` and the
pictures are its systems on the runner the towns and story maps use
(worklog 560, 566). Nothing drawn changed.

## The def

`field::def::FieldDef`, one `static` a type (`FIELDS`, `FieldDef::of`),
each `BASE_FIELD` with its differences: `class` (`GameClass::World`),
`ty`, `field` (type 4 has none: its first dungeon is a lake's and stands
for it), `def_se` (`GO`'s jump table, the old `DEF_SE`), `skies`
(`GetWeather` and `GetTime` by background row, the old `get_weather` /
`get_time` switches as data: `DRY`, `WET`, `SNOWY`), `back` (where
`SetCharPosition` stands the party back from the dungeon, the old
`field_start`'s type 10 and 6 arms), `spark` (`FIREFLY::Move`'s sparks,
`EFF_sfzfir1` in type 9), and the systems by who makes them: `init`
(`WORLD::Init`'s, from the seed's `fieldrand`), `plain` / `hacked`
(`DrawBG`'s by `isHacked`), `generate` (`Generate`'s after `SetStartPos`,
from the layout's); `order` (`FIELD_ORDER`) and `asleep`
(`FIELD_ASLEEP`).

| system | game | made | types |
| --- | --- | --- | --- |
| `Snow` | `SNOW`: flakes by weather (type 5's `sfg8`, 6's `sfh8`, `SnowDef::Flakes`), type 0's embers | Init | 0, 5, 6 |
| `Smoke` | `WORLD`'s `ccMask` 2D smoke | Init | 2, 3, 5, 6 |
| `Steam` | `DrawSteam` | Init | 0 |
| `Haze` | the heat haze (`HazeDef::ahead`) | Init | 0-3 |
| `LensFlare` | `LENSFLARE` mode 1: `Sun::Row` (`DMY_<x>lig_<b+1>point`), `field_eff`'s six, `FlareDraw::Field` (the torus wrap) | Init | all |
| `Scrolls`, `Sky` | `DrawBG`: `v$4519` with the turns (`ScrollsDef::turns`, `Wrap::Below` hacked), the row's clumps (`ClumpName::Row`, `Aurora`) at the centre turned (`At::Centre`) | DrawBG | all |
| `Tobj` | `TOBJ` by `Generate`'s roll | Generate | all |
| `Rain`, `Thunder` | `DrawRain`'s drops; its storm | Generate | 7-10 |
| `Fireflies` | `FIREFLY` by night or hacked (`Made::Generate`) | Generate | all but 9 |
| `Firefly2` | `FIREFLY2` roaming (`Firefly2Made::Roam`) | Generate | 9 |
| `Lake` | `SetLake`'s water | Generate | all |
| `Fires` | `SetSpecialObj`'s fires' smoke | Generate | 7 |
| `Bird` | `BIRD` by day out of the rain | Generate | 10 |

Each system's constructor makes what the game's does under the area's
weather, time and hack flag (`FieldCx`: the def, the weather row, the
server, the field as `Init` and `Generate` left it, `WORLD_MAN`'s centre)
and nothing otherwise. The systems' steps are `WORLD::Draw`'s and
`DrawEffect`'s order; asleep, the last frame's pieces are drawn again, the
haze's and the lake's sampling of the picture under them brought up to
the camera (`Hook::Again`). `GameClass` is now a column of the systems
table (`Kind::class`).

## What else became data

- `ScrollsDef`: a material takes its lanes `Shown::Before` or after per
  material (`MatScroll::before`; the arena's sky), names a field's sky or
  aurora material (`Mat::Sky`, `Mat::Aurora`), and the turns ride with
  their statics.
- `FlareDef`: its `Sun` (a dummy or a field row's) and its file (`town_z`,
  or `field_eff` for a field).
- `Firefly2Def`: who makes them (`Markers`, `Roam`) and their sprite
  (`ByField`, `Field`); both steps read the frame's `Env` (`Fly`'s
  ground a `Ground`).
- `TownView`, `TownEvent` are `systems::MapView` (with camera 1's place
  and turns, the frame's low bit) and `MapEvent` (with `effSmoke`, the
  sounds, TOBJ's hum, the flash).
- New requests: `Posed` (a field's `ccAnm::Draw`: drawn models, skin
  nodes, alpha, one model's edits, TOBJ's shadow) and `Masks` (a
  `ccMask`'s puffs in one send).
- The drawing's function statics (`SaveState::field_ambient`) are each
  system's words (`ambient::statics`), the executable's start
  `BOOT_STATICS`; a field without a system keeps its words as they were.

## What the world sees

`FieldArea { def, core: AreaCore, .. }` with `step(view, awake, cc)` (its
`MapEvent`s), `paint` (its sprites), `take` / `put`, `take_made` (TOBJ's
`tobjSeLoopStart`, once). The field's draw, a dungeon's and the breaths'
sprites go through `FieldFx::sprites`, their puffs through
`FieldFx::smoke`; `field::ambient::Op` is gone. No world or game code
names a field type: `WorldMan::no_field()` and `DungeonArea::no_field()`
(type 4's def), the gate-out checks, `member_transfer` and `loaddisp::kind`
take that flag; `defSE` is the def's.

`tools/test_field_ambient_rs.py` and `ambient_probe`: the probe makes the
port's field from the disc (`FieldArea::with_world` with the case's
`Init` centre) and hands the game's side the sun, the fires and Ω's
runners it found; draws and calls are compared apart (each in order).

## Checks

- Golden shots, every file byte for byte against worklog 568's baseline
  (4,548 files: the towns', story maps', fields', dungeons', breaths' and
  568's field and dungeon sets, every frame's digest among them).
- Against the game on all four volumes, all fifteen EE harnesses (60 of
  60, as the baseline), `test_field_ambient_rs` through the new probe.
- The crates' tests, piney-game's suite (release); `piney-gen gen
  --check`, clippy with all features (`too_many_arguments` 144, one fewer:
  `anim_edited` is gone; no `dead_code` allows), fmt, `cairns check`,
  `tools/docs.py check`.

Found on the way, for the next entries: `WORLD::Init` loads `fieldccs2`
(`field_a1` ..) for a hacked field, the port `fieldccs` always.

**Still unknown:** whether a hacked field's objects and ground come from fieldccs2 in the port as WORLD::Init loads them (it loads fieldccs)
