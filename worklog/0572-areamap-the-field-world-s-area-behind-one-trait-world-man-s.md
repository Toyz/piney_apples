---
number: 572
title: AreaMap: the field world's area behind one trait, WORLD_MAN's switch on its area once
date: 2026-11-03
area: world, build
files: crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/area43.rs, crates/piney-game/src/session/tests/boss_fx.rs, crates/piney-game/src/session/tests/boxes.rs, crates/piney-game/src/session/tests/dressing.rs, crates/piney-game/src/session/tests/ending.rs, crates/piney-game/src/session/tests/fairy_orb.rs, crates/piney-game/src/session/tests/field_gims.rs, crates/piney-game/src/session/tests/field_shots.rs, crates/piney-game/src/session/tests/kyvia_stage.rs, crates/piney-game/src/session/tests/mirror_world.rs, crates/piney-game/src/session/tests/ocarina.rs, crates/piney-game/src/session/tests/ride.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-world/examples/field_probe.rs, crates/piney-world/src/area/map.rs, crates/piney-world/src/area/mod.rs, crates/piney-world/src/chara/party.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/field/world.rs, crates/piney-world/src/field/world/ride.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/story_map/mod.rs, docs/engine/evarea.md, docs/engine/map.md, plans/rust-cleanup.md
started: 2026-11-03T04:35:49-07:00
took: 4h 22m
---

# 572. AreaMap: the field world's area behind one trait, WORLD_MAN's switch on its area once

Section 2's `AreaMap` item (`plans/rust-cleanup.md`), the first step of
"One world, one area system": the field world held its place as
`Place { Field, Dungeon, Story }` and matched it by hand wherever it
reached the area (the collision alone 19 times; the start, the clear
colour, the carry, the marker's stream, `Enter`, `ShowMap`, `RoomSelect`,
`GoField`, the doors, `EntryGimmick`, the kept dungeon, the draw), and
`crate::map` matched it again for the minimap. Now the area is one
`Box<dyn AreaMap>` (`area/map.rs`), each kind's impl in its own module
(`field/area.rs`, `dungeon/mod.rs`, `story_map/mod.rs`); no code outside a
kind's module names its type. Nothing drawn or played changed.

## Why a trait

The game's four area classes share no base (`piney_data::classes`: the
`ROOTTOWN0n` derive `ROOTTOWN`, the story maps `EVENTAREA`; `WORLD` and
`DUNGEON` derive nothing, and `DUNGEON` has no vtable). `WORLD_MAN` holds a
pointer to each, and every call of its that reaches the area switches on
`WORLD_MAN.area` (+0x08): `GetHeight`, `Enter`, `ShowMap`, `RoomSelect`
(only a dungeon builds a room), `GoField` (only from a dungeon),
`EntryGimmick`, `Get2DMapInfo`, `SetEventData`, `ccThFieldDisp`'s `Draw`.
`AreaMap` is that switch written once: its methods are `WORLD_MAN`'s calls
(and the events' `open_door` / `close_door`, which `ccEvent::Execute`
sends to the current dungeon); each kind answers as its class's code does,
and a kind whose class has no case keeps the switch's do-nothing default.
What differs between the kinds stays their defs and systems (worklog 560,
566, 569, 570): the trait has no getter of a system and no kind's name.

## The methods

- `core` / `core_mut`: the `AreaCore` (file, collision, light group,
  `fieldrand`, start), which every kind already held; the 19 hits matches,
  the lights and `fieldrand` go through it.
- `start(scene)`: `SetCharPosition` and `ccGetStartPositions`. The field's
  `field_start` moved to `field/area.rs`, `party_starts` to `area/map.rs`.
- `set_center`: `WORLD_MAN::SetCenter` (the field's `WORLD::SetCenter`).
- `clear`, `put`, `marker_file`, `event_start`, `special_room`: the clear
  colour, the carry into the state out, the stream `ccEvMarker2Pos` reads,
  `eventStartPos` as a story map leaves it, `specialRoom`.
- `disp(Disp)`: `ccThFieldDisp`'s `Draw`. The world hands every kind the
  same frame (view, `ccRand`, the save's bans, `ccCheckActiveObject` and
  its room form, the camera for a story map's scene) and takes back the
  same outputs: the sprites, the calls (sounds, flashes, smoke), the story
  scene's requests and `SetPathFindingMap` due. The symbols' fires and the
  breaths' flames and smoke join the area's sprites for every kind; a
  story map had dropped the breaths' (none in one: no shot moved).
- `enter`, `go_field`, `room_select`, `show_map` (with `EnterCx`) and the
  doors: the ways out come back as one `area::Exit` (the dungeon's own
  `Exit` moved there), which the world carries out as `ChangeArea` or
  `ChangeScene` in one place.
- `dungeon_pos`, `map_2d`, `map_2d_info`, `entries`, `keeps_entries` /
  `keep_entries`, `signal`, `report`, `boss_centre`, `into_kept`: what the
  battle's tasks read of the area (`WORLD_MAN.dungeonPos`, `Get2DMapPtr`,
  `restoreEntry` and `EntryGimmick` as `combat::AreaEntries`,
  `ccThEntryCtrlDelete`'s kept list, the story map's signals and report,
  `WORLD_MAN`'s kept dungeon or area 15's map).
- `setup_map`, `minimap`, `event_data`: the minimap's set-up and frame
  (`map::field_minimap`, `dungeon_minimap`), `SetEventData`.

`room_clear` and `no_active` take the entry control's lists and enemies,
not the whole `Combat`, so the room test and `ccRand` lend apart. The story
map's door test for the replay is `StoryArea::door`. The tests read a kind
through `<dyn AreaMap>::get::<T>()`, the session tests' `MapKind` view.

## Checked

Every field, dungeon and story-map harness and the world's on the four
volumes (40 runs: `test_world_rs`, `test_lights_rs`, `test_gate_out_rs`,
`test_party_rs`, `test_map_rs`, `test_evarea_rs`, `test_field_ambient_rs`,
`test_field_rt`, `test_dungeon_rt`, `test_area_rs`) as the baseline at
75b66cb; the golden shots, 4548 files, none differ; the crates' tests and
the piney-game suite pass; gen --check, clippy, fmt clean;
too_many_arguments 142 (was 144), dead_code 0. The baselines are from a
worktree at 75b66cb: the first run shared this tree while the edit began.

**Still unknown:** nothing
