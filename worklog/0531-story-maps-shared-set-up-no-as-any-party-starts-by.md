---
number: 531
title: Story maps' shared set-up: no as_any, party_starts by StartSpread, one statics loader, event_hits and model_hits, light_group, one ee::ground_dist
date: 2026-10-30
area: world
files: crates/piney-world/src/story_map/, crates/piney-world/src/ee.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/field/firefly.rs, crates/piney-world/src/field/world.rs, crates/piney-world/src/sky/cloud.rs, crates/piney-world/src/town/mod.rs, crates/piney-world/examples/story_map_probe.rs, crates/piney-game/src/session/tests/ending.rs, plans/rust-cleanup.md
started: 2026-10-30T13:23:37-06:00
took: 18m
---

# 531. Story maps' shared set-up: no as_any, party_starts by StartSpread, one statics loader, event_hits and model_hits, light_group, one ee::ground_dist

The story maps' item of section 2 (`plans/rust-cleanup.md`). The eight
`EVENTAREA` classes each carried copies of the same set-up: the statics'
loops, the event area's collision, the light group, the party's start
places, the ground distance, and three `Any` casts. Each is one piece now.
Nothing they compute changed.

## The pieces

- **No `as_any`.** `StoryMap: Any`, and the trait object upcasts to `dyn
  Any` (Rust 1.86 on), so `as_any`, `as_any_mut` and `into_any` are gone
  from the trait and the eight impls. `Place::story`, `story_mut`,
  `WORLD_MAN::Quit`'s kept map and the ending's test cast the object.
- **`party_starts(start, StartSpread)`** is `SetCharPosition`'s event area
  branch: the leader at `eventStartPos` facing its w, the two others at
  `(+-side, back)`, the third at (0, +300).
  - `StartSpread::BEHIND` (+-300, +150: main 0x001a1414) and `AHEAD`
    (+-200, -150: 0x001a1190).
  - `StoryMap::start` and `spread` (`BEHIND` by default) feed a defaulted
    `start_positions`. Seven copies are gone; the giant and the mirror sky
    say `AHEAD`; the disc keeps its field 12 places over the default.
  - The copies wrote `add(c, x)` or `sub(y, c)` where the one writes
    `add(x, c)` and `add(y, -c)`. The EE `add.s` port is symmetric and
    `sub` is `add` of the negated bits, so the bits are the same.
- **`StaticModel::load`, `StaticObject::load`** (the statics' one loader):
  a table's rows over the scene file, each row's object (its name through
  a function: the arena's stage rename), placed at its dummy or nowhere
  (`Placing::Origin`: `EVENTAREA07::ChangeBlock` calls no `SetPos`), the
  objects `SetMatrix_PosRotZYX`'d. Five maps' loops are gone. Each map
  still picks its table (`statics::tables()` or `statics::of(volume)`).
  - The copies that mapped `Position::None` to zeros were doing what
    `dummy_bits` does already.
- **`event_hits(volume, def_se, models)`**: the event area's `Hits` (area
  1, the bounds, no height map, `defSE`), eight literals gone, with
  **`model_hits`**: each model's own Hit chunk in the order the statics
  were built (six copies, the holy ground's block change among them).
- **`light_group(file, anim, names, skip_missing)`**: the light
  animation's ambient and the named lights at frame 1, with their
  objects. The arena skips names its stage lacks, the others fail. Four
  copies gone; Area 43's, which takes every record of its one light, stays.
- **`ee::ground_dist(volume, dx, dy)`**: the double-precision ground
  distance (`sqrt.s` from Outbreak on) of the statics, the town objects,
  the disc's objects, the clouds and both fireflies. Nine copies, two of
  them functions, are one; `field::area::ground_dist` of a vector is a
  call to it.

The trait's `hits`, `hits_mut` and `lights` stay required one-liners: the
maps hold them in two shapes (their own fields, or the towns' `Base`).

The diff is about 260 lines added and 390 removed.

## Checks

- piney-world's and piney-game's tests, release (the story maps' and
  towns' tests and session runs through every map).
- Against the game: `test_evarea_rs` on all four volumes (area 15's
  map: its set-up, statics, lights, hits, draw, ground, door and starts);
  `test_town02_rs`, `test_town03_rs`, `test_area_rs` and
  `test_field_ambient_rs` (the clouds and fireflies) on Infection and
  Outbreak, either side of the `sqrt.s` change.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** nothing
