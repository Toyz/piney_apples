---
number: 28
title: "A town's other files: the crisis versions, and the canal water"
date: 2026-09-23
area: world, render, build
files: tools/statics.py, crates/piney-data/src/statics, crates/piney-viewer/src/mesh.rs, crates/piney-viewer/src/main.rs, docs/engine/statics.md
---

# 28. A town's other files: the crisis versions, and the canal water

With [[27]]'s placement, Mac Anu stood right, but the user noticed its
canals were empty: "just no water". The water is not in `town01`.

## The files each town loads

`ROOTTOWN01`'s constructor asks `ccStream::GetCCSAdrs` for three scene
files by name. `tools/statics.py` now reads those names out of every
`ROOTTOWN`/`EVENTAREA` constructor: the `$a0` string built just before each
`jal GetCCSAdrs`. It writes them as `SceneSet`s into the generated tables.
- **The towns.** `ROOTTOWN01` loads `town01d`/`town01` and `wat1`. Towns 02,
  04 and 05 add `town_z`, which holds no models; town 03 adds nothing.
- **The event areas.** Most add `town_z`. `EVENTAREAB0` lists nine areas,
  each with its own floor and object set - alternatives, not companions.

The first pair is a choice (`INF gcmn.prg:0x004215a4`): `lb
saveData+0x6772`, the save's `crisis` byte ([[16]]), selects `town01d` when
set and `town01` otherwise. So the `d` files are the towns in crisis. Both
fit the same model table ([[27]] had found `RT_MODELTABLE00` placing
`town01` and `town01d`).

`wat1` is loaded either way:
- It holds one plane, `MDL_wat00`, spanning the town (x ±7,800, y −8,700 to
  8,100) about 100 units below the streets.
- It has 15 morph targets `MDL_wat01`-`15`, driven by `F_Morpher` records
  in `ANM_sr1wat1_a`.
- The constructor makes three `ccAnm`s of that animation, with the name
  taken from the global `wateranmname`, and `Duplicate`s `OBJ_wat00` in
  each: flag 8200 for the first, 8192 for the others.
- It roots all three with `SetMatrix_PosRotZYX` from two constants
  (0x005d4a90, 0x005d4aa0) that are zero vectors with w = 1. So the water
  sits where its file puts it.

## In the port

`statics::companions(scene)` returns the other files of a town's set, less
the alternatives: files that share the scene's model table, like
`town01d` for `town01`. The viewer merges each companion that has models
into the same mesh (`Mesh::append`). Mac Anu now has its canals.

Event areas are left out of this on purpose: `EVENTAREAB0` chooses among its
files, and how is not traced.

The user also asked why the tables' `kind` and `postype` were bare integers
when their meaning is known. They are now enums:
- `DrawPass`: `ROOTTOWN01::DrawFloor`, `DrawObj` and `DrawObj2` test for 1,
  2 and 3, and `DrawObj2` tests the static objects for 3 too. Type 0 (37
  model rows, 25 object rows) is drawn by none of them, so it is `Other`.
- `Position`: none, `Dummy(name)` or `DummyRot(name)`, which folds
  `postype` and `posname` together and cannot hold a type without a name.

The generator refuses any value it does not know.

**Still unknown:** why the water is three copies of one animation (the
first `Duplicate`d with a different flag), whether they are offset in time,
and how they blend; the waves themselves (morph playback, in progress
separately); what `town_z` holds; how `EVENTAREAB0` picks its area; what
draws `type` 0 rows.
