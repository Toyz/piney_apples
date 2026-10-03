---
number: 366
title: Every field object has its own hits: rocks and trees of one model all stand in the way
date: 2026-10-03
area: world
files: crates/piney-world/src/field_area.rs, tools/test_field_rt.py, docs/engine/field-walk.md
---

# 366. Every field object has its own hits: rocks and trees of one model all stand in the way

Issue #38 (build 76fc4b6): in a field, Kite walked into a rock at a dead
tree's foot, as if it had no collision ("a couple of materials ... have no
collision").

## Cause

The port kept one `ccModelHit` per Hit chunk, that is per model, and every
field object of that model used it. `set_hit_matrix` moved the shared hit
to each object in turn, so after a draw pass only the last object of a
model drawn stood in the way. Kite walked through every other rock, tree
or plinth of the same kind. [field-walk.md](../docs/engine/field-walk.md)
described this as the game's own behaviour. No worklog entry checked it.
`tools/test_field_rt.py` built its `ccModelHit`s from the probe's list and
so took the same assumption as given.

The game makes one per object. Each `FOBJECT` and `FOBJECT2` builds its own
`ccClump` or `ccAnm` (`FOBJECT::SetPosition2`, INF gcmn.prg:0x005b0c50:
`new` then `ccClump::Init`, or `new ccAnm` then `SetAnm`). A clump or anm
makes a `ccObj` per node or entry. `ccObj::Init` (INF
SLUS_202.67:0x0013b5c0) always makes a new `ccModel` for an Obj with a
model (`__nw__(64)`, `ccModel::Init`, the result at `ccObj` +0x94, from
0x0013b63c). `ccModel::Init` (0x0013a440) makes a new 160-byte
`ccModelHit` when the model chunk names a Hit chunk (chunk +0xc), and
points it at the chunk's polygons (from 0x0013a574). So every object has
its own hit, with its own matrix and its own place on the list; only the
polygons are shared. The dungeon's rooms were already built that way
(`DungeonArea::obj_hits`).

## Fix

`FieldArea::new` gives each object its own copies of its models' hits
(`HitModel` with the polygons behind an `Rc`), indexed in
`model_hits`. An anm object's come from its index entries
(`Animation::objects`, [[363]]), in index order. Before, they came from
`locals_at(0)`'s keys, in a `HashMap`'s order. A bare clump's come from
its nodes, ExtObj copies followed.

## Checked

- **Test.** `objects_of_one_model_each_keep_their_hit` (piney-world)
  places two objects of one model in story area 14's field. The collision
  list holds both hits, each at its own object's place. With one hit per
  model it held one.
- **The game harness.** `tools/test_field_rt.py` passes. Its `ccModelHit`s
  follow the probe's per-object list, so it confirms the hits' use, not
  their number. That rests on the disassembly above. `ccLandHitCheck` now
  spreads its points over every placed object, 4,000 of them: 33 land on
  an object's hit, 0 mismatches.
- **Suites.** piney-world (95) and piney-game (223) pass.

**Still unknown:**
- Which field the report's rock and tree are in is not known; the cause
  covers any two objects of one model drawn in the same pass.
