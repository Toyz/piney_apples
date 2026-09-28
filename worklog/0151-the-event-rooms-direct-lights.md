---
number: 151
title: The event rooms' direct lights
date: 2026-09-26
area: render
files: crates/piney-world/src/town.rs, crates/piney-world/src/chara.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/examples/world_probe.rs, tools/test_lights_rs.py, docs/engine/animation.md, docs/engine/dungeon.md
---

# 151. The event rooms' direct lights

A GAPS lighting item. Every event room of Infection's dungeons (the Aura
shrine `se1_3`, `se1_4`, `se2_2`, `se3_1`, `se3_3`, `se4_2`) has one light,
`LGT_se*lig1`, and it is a direct light (Anime record 0x0605). The port
read only distant (0x0603) and omni (0x0609) records, so the rooms had no
light of their own.

## The record and the light

- **The record.** `ccAnm::SetAnmCtrlWork`'s 0x0605 case reads eight
  controllers by flag: position (bits 0-2), rotation (3-5), colour (6-8),
  intensity (9-11), the beam's start and end (12-14, 15-17) and two radii
  (18-20, 21-23). The rooms' flags are 0x240041: position (0, 0, 2500),
  colour 0x..ffffff (white), radii 498 and 500 (`se2_2`: 800 and 850). An
  earlier note took the radii for one pair of floats and the colour for
  black; it was the packed white read as a tiny float.
- **`ccDirectLight::CheckRange`** (main 0x00139330). The point is taken
  into the light's space (its matrix inverted). Nothing is lit behind the
  light or past the beam's end. The light is full to the beam's start,
  then falls off linearly to its end (no end when 0). It is full within
  the first radius, then falls off linearly to 0 at the second. Its
  direction is `lightVector` (0, 0, -1) through the light's matrix;
  `ccCreateLight` gives it priority 0.
- **The place.** `SetRoom` steps the room's anm and then calls
  `SetMatrix_PosRotZYX(light, (750 (x + 4), 750 (y + 4), 0), rotation)`,
  which replaces the record's place. The room's centre is `750 (x + 8)`, so
  the beam falls a quarter of the way in from the room's corner. The
  rotation is stack left over from an earlier call; it only turns a beam
  down -z about z.

## The port

- `town::AnimLight` reads 0x0605 records. `town::Light` gains kind 2 and
  `radius`. `chara::check_range` computes the beam from its axis alone (the
  beam is round), with the perpendicular taken as `d - axis * z`, exact for
  a beam straight down.
- `DungeonArea::event_room_lights` now replaces the light's place with the
  row's, where it used to add the row's place to the record's.

## Checks

- `tools/test_lights_rs.py` sets up direct lights in the game's own
  `ccDirectLight` (its matrix and inverse, colour, intensity, beam and
  radii) among distant and omni lights, and compares `SetLightMatrix`'s
  slots with `chara::light_matrix`. Beams point straight down or any way,
  with the shrine's 2-unit edge or wider ones. 400 groups in the unit test
  and 3,000 in bulk (391 direct lights reaching the point): 0 mismatches.
- `the_shrine_has_a_direct_light` reads `se1_3`'s record and checks the
  beam at the floor, at half its edge, outside it and above the light.
- Shots of the shrine before and after are the same pixel for pixel. The
  room's models are unlit (mtype 0), and Kite stands at `OBJ_user_point`,
  outside the beam. Only a character standing in the beam is lit.

**Still unknown:** whether anything gives the light a parent coordinate
that would move it off `(750 (x + 4), 750 (y + 4), 0)`, and whether the
room anm's loop re-applies the record's frame-0 place. Neither was run in
the game. No character was pictured in a beam.
