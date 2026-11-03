---
number: 570
title: Dungeons as defs: DungeonDef by dungeon type, the dress, lake and leaves as systems
date: 2026-11-03
area: world
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/boxes.rs, crates/piney-game/src/session/tests/dressing.rs, crates/piney-game/src/session/tests/field_shots.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-world/examples/ambient_probe.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-world/examples/map_probe.rs, crates/piney-world/examples/story_map_probe.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/dungeon/def.rs, crates/piney-world/src/dungeon/dress.rs, crates/piney-world/src/dungeon/lake.rs, crates/piney-world/src/dungeon/leaves.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/field/def.rs, crates/piney-world/src/field/world/ride.rs, crates/piney-world/src/field/world.rs, crates/piney-world/src/map/dungeon.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/story_map/mod.rs, crates/piney-world/src/systems/fireflies.rs, crates/piney-world/src/systems/lake_sky.rs, crates/piney-world/src/systems/leaves.rs, crates/piney-world/src/systems/mod.rs, crates/piney-world/src/systems/pieces.rs, crates/piney-world/src/systems/room_lights.rs, crates/piney-world/src/systems/room_water.rs, crates/piney-world/src/systems/sparks.rs, crates/piney-world/src/town/mod.rs, docs/engine/dungeon.md, plans/rust-cleanup.md
started: 2026-11-02T23:18:48-07:00
took: 2h 1m
---

# 570. Dungeons as defs: DungeonDef by dungeon type, the dress, lake and leaves as systems

Section 2's map systems item, step 3's second half (`plans/rust-cleanup.md`):
a dungeon was `DungeonArea` with a `Dressing` (`dungeon/dress.rs`: the
water, sparks, room lights, objects and anm objects `SetRoom` stands in a
room), a `Lake` (`dungeon/lake.rs`: a lake's `DrawBG` and its fireflies)
and type 32's `Leaves` (`dungeon/leaves.rs`), each a field of the area
called by name, the lake gated on `dtype >= 8`. Now each dungeon type is a
`DungeonDef` and those are its systems on the runner the towns, story maps
and fields use (worklog 560, 566, 569). Nothing drawn changed.

## The def

`dungeon::def::DungeonDef`, one `static` a type (`DUNGEONS`,
`DungeonDef::of`), each `BASE_DUNGEON` with its differences: `class`
(`GameClass::Dungeon`), `ty`, `lake` (types 8 and 9: one floor, no event
rooms, the statues), `food` (`GetFood()`'s gimmick), `breakables`
(`EntryBreakObject`'s rows), `family` (the story rooms' `ROOM_INFO`
family), `door_se` and `leaf_off` (`MoveDoor`'s sound and whether an open
door's leaf leaves the hit list, its jump tables @8659 and @8664), the
systems, `room` (`SetRoom`'s steps, `ROOM`), `order` (`Draw`'s, `ORDER`)
and `asleep` (`ASLEEP`). The old per-type `match`es on `dtype` for these
are gone.

| system | game | types |
| --- | --- | --- |
| `RoomWater` | `SetWater`'s three water anms, `DrawWater` (`u$8862`) | all |
| `Sparks` | `SetWater`'s `SNOW` type 3 embers at `OBJ_o_magma*`, `ChangeClut`'s palettes in 0 and 1 | all |
| `RoomLights` | `SetLight`'s glows and omni lights (`type & 3`, the altar's in 2 and 6) | 0-7 |
| `Pieces` | `object[]`, `anmobj[]`, `anmobj2[]`: the lakes' statues, 3 and 7's walls, 0-3's anms | all |
| `LakeSky` | a lake's `DrawBG` (`v$8917`), about the room under the player | 8, 9 |
| `Fireflies` | `SetRoom`'s five `FIREFLY`s (`Made::Room`) | 8, 9 |
| `Leaves` | type 32's tree and twenty `LEAF`s | all (event room 32) |

## SetRoom as steps

`SetRoom` builds the room's anm and its hits itself, then runs the def's
`RoomStep`s: each a system's `RoomPart` (`Leaves`, `Water`, `Light`,
`Object`, `AnmObject`, `Fireflies`) through `MapSystem::room`, or `Doors`
(the core's `SetDoor` and the ban block) between `SetObject` and
`SetAnmObject`, as the game orders them. `RoomCx` hands each system the
dungeon's files (`Rooms`: the scene file, its objects' models and hits,
the event room's `spccs`, the water model, the palette swaps), the room's
anm and slot, the floor and level, the event room's type, `fieldrand`, the
light group and the hit list's joins. What joins the `ccModelHit` list
after the room's is `Joined::{Door, Block, Piece(Kind, ..)}`, in the
game's order, so `put_doors`, `MoveDoor` and `HitEnable` keep it; the
pieces' hits come back through `MapSystem::hits` and `hit_names`.
`RoomPart::Clear` is `DeleteRoom` and `ClearRoom`. Type 32's owed
`ccRandF` tilts are `MapSystem::settle`, which `DungeonArea::settle` runs
after each call that can `SetRoom`.

## Draw

`DungeonArea::draw` runs `order` (asleep, `asleep`: each piece again, the
water sampling the picture afresh and the fireflies drawn again, as
before) with `MapView::room` the room under the player (its centre and
size, which `LakeSky` reads), paints the requests, then draws the room,
its doors and the ban block. The anms' and clumps' pieces are a new
`Request::Pieces` (models each at its own matrix, the file's palette
swaps, one model's vertex edits). `u$8862` and `v$8917` stay statics of
the executable through `take` / `put` (worklog 474); the page's "lives as
long as the dungeon" lines were stale and are gone.

## Checks

- Golden shots (worklog 568): all 4,548 byte-identical to the baseline,
  the dungeons' (`dungeon_shots`, every type and volume, the lakes by
  night and hacked, area 101's leaves) among them.
- tools/test_dungeon_rt.py, test_map_rs, test_lights_rs, test_field_rt,
  test_world_rs and test_area_rs on all four volumes: 24 of 24 pass, each
  line as before. Crate tests, the piney-game suite (344), `gen --check`,
  clippy (`too_many_arguments` 144, `dead_code` 0), fmt, `cairns check`,
  the docs check.

**Still unknown:** nothing
