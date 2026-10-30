---
number: 533
title: AreaCore: one shape for every story map and town, the getters defaulted through core()
date: 2026-10-30
area: world
files: crates/piney-world/src/area.rs, crates/piney-world/src/story_map/, crates/piney-world/src/town/, crates/piney-desktop/src/assets.rs, plans/rust-cleanup.md
started: 2026-10-30T13:54:39-06:00
took: 49m
---

# 533. AreaCore: one shape for every story map and town, the getters defaulted through core()

The first step of this run on section 2 (`plans/rust-cleanup.md`, the
story maps' item, whose worklog 531 left `hits` and `lights` as
one-liners because the maps held their state in two shapes). Every map
now holds one core. Nothing it computes changed.

## The shape: `area::AreaCore`

`AreaCore { file, hits, lights, field_rand, start }` is what every area
holds: its scene file (the stream `marker_pos` reads), its collision, its
light group, `fieldrand` when its draws take from it, and
`eventStartPos` as a story map's `SetStartPos` left it. It sits in
`area.rs`, the shared core, under the plan's name for it.

- The six story maps that held their own fields (the abyss, the boss
  arena, the disc, the ending, the giant, the holy ground) hold `core`.
- The towns' `Base` embeds it in place of its `file`, `hits` and
  `lights`, so the two maps on `Base` (the sky maps, area 13) hold their
  start and `fieldrand` there too.
- `StoryMap::core()` / `core_mut()` and defaults for `hits`, `hits_mut`,
  `lights`, `start`, `file`, `field_rand`, `field_rand_mut` through it:
  41 one-liners are 16 (`core` and `core_mut` a map).
- The towns' `fieldrand` is the core's: `RootTown::field_rand` and
  `field_rand_mut` and their four impls are gone; Dun Loireag, Fort Ouph,
  Lia Fail and Carmina Gadelica set `Some(env.rng)` in `new`.

## Why a core, not every map on a `Base`

Measured both. A `Base` for every map would carry the towns' statics
(`ModelTable`, `PlacedModel`, `Object`) into six maps that load theirs
with `StaticModel`/`StaticObject`, and a town number, events and kept
dummies they never have; the ending, with no file, could not hold one.
The core is the five things every area shares and nothing else, so it is
the seam for the one `World` of the systems audit (towns, fields,
dungeons and story maps on one `AreaCore`).

## The quirks kept

- `fieldrand` stays None for the maps that never draw from it (the holy
  ground, the abyss, the ending): the world keeps the game's stream, and
  `set_field_seed` keeps its two branches. The maps that draw hold
  `Some`, and each draw takes it with `if let`.
- The ending loads no stream: its core holds `SceneFile::empty()` (no
  objects, so no name is found) and its `file()` still answers None.
- The abyss starts from its table's constant, the disc keeps its field
  12 places over the default `start_positions`.
- `Town::open_with` still sets the class's `fieldrand` after `new`, so
  Lia Fail's glows' draws in its constructor are reset as before (see
  below).

## Found, not changed

`Town::open_with` sets `fieldrand` back to the value it was given after
the class's `new`. For Lia Fail, whose constructor draws 21 values from
it for its glows, that drops those draws. Whether `GO(0)` reseeds after
`ROOTTOWN05::ROOTTOWN05` decides it; no harness covers Lia Fail's glows.
Behaviour is kept here.

The diff is about 550 lines added and 600 removed (most of it the
`base.core.` paths).

## Checks

- Golden shots before and after, all 532 PNGs byte for byte the same:
  `dun_loireag_shots`, `mac_anu_chat_shots`, `ride_shots`,
  `flag_race_shots`, `mirror_world_shots`, `staff_roll_shots`,
  `tarvos_under_god`, `corbenik_under_god`, `kyvia_stage_shots`,
  `event_11_shots`, `skeith_shots`, `hopeless_nothingness_shot`.
- Against the game: `test_evarea_rs` on all four volumes;
  `test_town02_rs`, `test_town03_rs`, `test_area_rs`,
  `test_field_ambient_rs` and `test_world_rs` on Infection and Outbreak.
- piney-world's, piney-desktop's and piney-game's tests, release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** whether GO(0) reseeds fieldrand after ROOTTOWN05's constructor, which decides if Lia Fail's glows' draws in it should carry on (Town::open_with resets them).
