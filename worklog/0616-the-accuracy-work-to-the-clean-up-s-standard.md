---
number: 616
title: The accuracy work to the clean-up's standard
date: 2026-11-19
area: engine
files: crates/piney-data/src/vu0.rs, crates/piney-data/src/vu1.rs, crates/piney-data/src/anim.rs, crates/piney-data/src/light.rs, crates/piney-data/src/field/render.rs, crates/piney-data/src/model.rs, crates/piney-data/src/shadow.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-battle/src/geom.rs, crates/piney-battle/src/entry.rs, crates/piney-effect/src/vu.rs, crates/piney-desktop/src/anm.rs, crates/piney-desktop/src/camera.rs, crates/piney-desktop/src/shadow/draw.rs, crates/piney-desktop/src/shadow/gif.rs, crates/piney-desktop/src/shadow/vu1.rs, crates/piney-world/src/chara/kite.rs, crates/piney-world/src/systems/statics.rs, crates/piney-world/src/town/mod.rs, crates/piney-stream/src/draw.rs, crates/piney-stream/src/scene.rs, docs/engine/field.md, docs/engine/effects.md
started: 2026-11-19T08:40:15-07:00
took: 6h 21m
---

# 616. The accuracy work to the clean-up's standard

Worklogs 610-615 made the lights, the keyed rotations, the shadows, the
desktop cursor, the model boxes and the new-game boot bit-exact. This entry
reviews that code against plans/rust-cleanup.md and brings it up to the
standard. It is a refactor: every harness, pin and shot is unchanged.

## One home for libvu0

The accuracy work had added VU0 routines to `piney_data::vu0`, but older
copies stayed. `_sceVu0ecossin` and the rotations were written out seven
times (anim, piney-effect's `vu`, battle `geom` and `entry`, field
`render`, `evcam`, the map sprite), each with its own `MulMatrix`.
`ApplyMatrix`, the unit matrix and the vector ops had more copies still.
Each copy was checked to give the same bits as the kept routine (the extra
`vadd` with zero folds away).

- `piney_data::vu0` now holds `ecossin`, `rot_matrix_x/y/z`, `rot_matrix`,
  the real `mul_matrix` (the old one re-exported anim's `vu_mul`), `vdiv`
  (`sceVu0DivVector`), `scale_matrix`, and `ccCoord`'s setters
  `pos_rot_xyz_scale`, `pos_rot_zyx_scale`, `pos_rot_xyz` and `pos_rot_zyx`.
  Before this there were seven setter copies, in effect `vu`, world ambient
  and town, stream scene, and Corbenik's two.
- Battle `geom` and world `ee` stay as their crates' maths preludes, but
  re-export vu0 under vu0's names. Battle's `apply_matrix` and
  `unit_matrix()` are now `apply` and `UNIT`.
- The glam conversions are now only `vu0::of_mat4` and `to_mat4`. Gone:
  world `draw::mat`, combat boss `mat4`, the stream's and the desktop
  camera's `to_mat4`, and three hand-written `to_cols_array` loops.
- Removed shims: `anim::ee`, town's `Light`/`Falloff`/`LightKind`/`AnimLight`,
  `kite::SHADE`, boss `set_color`, dungeon `invers` and `hit::UNIT`. Their
  users now name `piney_data::light`, `vu0` and `field::ee`.
- The copied `c.eq.s` (light, world `ee`, geom), the `to_f64` comparisons
  (`eef`, cinema) and vu1's `vsub` now use `field::ee` and `vu0`.

## The light group

- `light_regs` and `set_light_matrix` take the group as an iterator.
  `AddGrp`'s order is walked level by level (highest priority first, equal
  priorities in the order added), so a lit model no longer allocates a
  sorted `Vec`.
- `SetLightMatrix`'s three records are a `Slot` (priority, row) instead of
  a tuple.
- One conversion from the VU1 registers to the draw's `Lights`:
  `piney_desktop::anm::model_lights`. It replaces three copies (kite, the
  stream, the title).
- Kite's shaded group is now a `map` over a chain, with no per-model
  allocation.
- `StaticObjects` keeps each light as an `ObjectLight` struct instead of a
  tuple, and the town and story map refill a kept buffer each frame.

## Poses, shadows, data

- `Animation::local_bits_at` gives the locals as float bits. The poses no
  longer go bits to glam and back per object per frame.
- The shadow kicks' triangles and quads are `Polygon<3>` and `Polygon<4>`
  arrays, not a `Vec` per polygon.
- The box check's MAC mask is now a named constant.
- The shadow draw's scale matrices are `vu0::scale_matrix`.
- The shadow `direction` is `vu1::itof12` then `vu0::vdiv`.
- `FogRow::ambient_bits` replaces world dungeon's copy.
- `VertexBox` is typed as `V4`.
- The model's 16, 256 and 4096 are named constants.
- `checked_max` reads `MIN_TRANSPARENCY`.
- Comments: the pose `instances` doc is cut to the cap; the docs name the
  modules that hold the maths (field.md "EE floating point", the effects
  module table).

`ShadowTri::pixels` stays in piney-draw. It states which pixels a shadow
request covers, which both backends need, so it is not the CPU renderer's
alone.

## Checks

All on fe06078, the code of this entry:
- The sweep: 412 of 412 rc 0, each line as in `summary-main.txt`. A
  harness replaced the worktree's `tools/eemu_rs.so` symlink with a build;
  the symlink was put back and main's file is untouched.
- Battle pins: 291 calls, 0 differ (probe wt-review at fe06078).
- Golden shots: a same-day base from a clean worktree of main 1aa226b
  equals `world2/shots/base` (4,550 files, 0 differ). The tip against that
  base: 4,550 files, 0 differ. The base worktree and its target were
  removed afterwards.
- Crate tests: 786 passed, 0 failed. Two tests went with their copies;
  vu0's rotations and setters and the light group's order gained three.
- The piney-game suite: 351 / 0 / 153.
- Clippy on all targets and features: clean. fmt, `piney-gen gen`, `syms`
  and `carry --check`, `cairns check` and `tools/docs.py check` all pass.

## Noticed, not changed

`piney_desktop::anm::fobj_matrix` builds an F_Obj record's matrix from
glam's host-float Euler turns. `piney_data::anim::ObjRecord::pose` builds
the same record through `DecodeF_Obj`'s EE arithmetic. The desktop's F_Obj
objects may therefore be off in the last bits. This was not checked
against the game.

**Still unknown:**
- Whether `fobj_matrix` differs from the game anywhere. No harness
  compares a desktop F_Obj matrix word for word.
- Piney-world's `ee`, battle's `geom` and the desktop's `eef` remain as
  per-crate maths preludes. They re-export the one home and add only their
  own helpers. Whether the clean-up wants them folded into direct imports
  is the maintainer's call.
