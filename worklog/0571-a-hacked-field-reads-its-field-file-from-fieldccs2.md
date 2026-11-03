---
number: 571
title: A hacked field reads its field file from fieldccs2
date: 2026-11-03
area: world
files: crates/piney-data/src/field/mod.rs, crates/piney-world/src/field/ambient.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/map/field.rs, crates/piney-world/src/systems/mod.rs, crates/piney-world/src/systems/snow.rs, docs/engine/field.md
resolves: 569
started: 2026-11-03T01:20:19-07:00
took: 1h 29m
---

# 571. A hacked field reads its field file from fieldccs2

Worklog 569 left this open: `WORLD::Init` reads a hacked field's own file
from `fieldccs2`, while the port always read it from `fieldccs`.

## The game

`WORLD::Init` (INF gcmn 0x005a4fd4 .. 0x005a5030) checks `isHacked`
(`WORLD` +0x2c): when it is 2 it calls `GetCCSAdrs(fieldccs[type])`
(0x00657130: `field_a`, `field_b` ..), otherwise
`GetCCSAdrs(fieldccs2[type])` (0x00657160: `field_a1`, `field_b1` ..),
and stores the file at +0x6148. That word is what `FIELD::SetCCS` takes
the ground, cover and object models from (0x005a5120), what each of the
three `SNOW` constructors gets as its stream (0x005a66a0, 0x005a66f8,
0x005a6758) and where the minimap's `fieldminimap[type]` texture comes
from (docs/engine/map.md).

## The port

`piney_data::field::Tables::field_ccs(type, hack)` picks the name;
`field::ambient::Weather::file` (the type and `isHacked` the field already
carries) replaces `field_file`. `FieldArea::with_world` reads the field
file through it, so the hacked field's ground, cover, objects, hits and
minimap come from `field_X1`, and `Snow` hands its flakes and embers the
same file. The plain fields are unchanged.

## What moved

- `field_shots`: all 1,120 hacked shots (every type, row and volume,
  plain ones untouched): the hacked ground, cover and objects in
  `field_X1`'s textures, and its minimap.
- `gate_hack_arrival_shots`: the three frames in the hacked field
  (`arrive-4-act24`, `arrive-5-act23-arms`, `arrive-6-after`), for the
  same reason.
- Every other shot byte-identical. tools/test_field_rt.py,
  test_field_ambient_rs, test_map_rs, test_world_rs and test_lights_rs
  on all four volumes: 20 of 20 pass, each line as before; crate tests,
  the piney-game suite (344), `gen --check`, clippy, fmt, the checks.

**Still unknown:** nothing
