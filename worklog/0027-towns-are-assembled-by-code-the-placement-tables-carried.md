---
number: 27
title: "Towns are assembled by code: the placement tables, carried into the port as data"
date: 2026-09-23
area: world, render, build, test
files: tools/statics.py, tools/test_statics.py, crates/piney-data/src/statics, crates/piney-data/src/scene.rs, crates/piney-viewer/src/mesh.rs, docs/engine/statics.md, docs/formats/ccs.md
---

# 27. Towns are assembled by code: the placement tables, carried into the port as data

The viewer of [[26]] drew Mac Anu (`town01`) as a heap. The user, looking
straight down on it: "the town is mangled... placements are wrong". They
were. The scene file holds each district's floor, buildings and details in
its own space, all overlapping at the origin, and nothing in the file says
where they go: the `OBJ_floor_*` objects have no parent and no animation.

## Where the placement is

`gcmn.prg`'s strings name both `MDL_floor_01` and `DMY_floor_01pos`. The
town code is one class per town. `ROOTTOWN01::DrawFloor` (`INF
gcmn.prg:0x00422ba0`) draws up to 32 `STATICMODEL`s, and its constructor
builds them from `RT_MODELTABLE00` (0x005d46b0), rows of
`STATIC_MODEL_INFO` `{type, modelname, postype, posname, clip}` (DWARF):
`MDL_floor_01` goes to `DMY_floor_01pos`, and so on for all 32 rows.

- **DummyPos chunks.** The 0x1300 payload turned out to be `u32 object,
  f32 pos[3]`, and 0x1400 adds `f32 rot[3]` in degrees. `town01` has 83 and
  25 of them: district origins, flag and ship posts, merchants, event
  markers.
- **How a model is placed.** `STATICMODEL::STATICMODEL` (0x005cffd0) copies
  the dummy's position, and its rotation for postype 2. But
  `STATICMODEL::Draw` (0x005d01c0) builds a unit matrix and translates it.
  So a piece is its model plus a translation, no rotation, whatever the
  owning object says.
- **Animated pieces.** They go through a second table, `STATIC_OBJ_INFO`
  `{type, clumpname, anmname, postype, posname, clip}` (`RT_OBJTABLE00`,
  11 rows). `STATICOBJECT::STATICOBJECT` (0x005cf9b0) sets the root of the
  clump and the animation with `SetMatrix_PosRotZYX` from the dummy. That
  is how one flag animation stands at six posts and one ship at two.

A naming rule (`MDL_<x>_NN` goes to `DMY_floor_NNpos`) fits only 100 of the
172 model rows across all tables, so the tables themselves are needed.

## Data, not a runtime ELF reader

The first cut read the executable from the disc image at startup and
parsed its symbol table. The user pointed out that a port should carry
what we reverse engineered as its own data. The tables are engine logic -
which piece goes on which dummy - and only names and numbers, not art or
text. So:
- `tools/statics.py` extracts every `*MODELTABLE*` and `*OBJTABLE*` object
  in `gcmn.prg`: 16 model tables with 172 rows, and 11 object tables.
- It matches each table to the `DATA.BIN` members that hold every object
  it names: `RT_MODELTABLE00` to `town01` and `town01d`, the others to
  `town02`-`05` and nine event areas.
- It writes them as `const` Rust (`crates/piney-data/src/statics/inf.rs`,
  exempt from rustfmt).

The positions are still read from the disc's DummyPos chunks at run time.
The runtime ELF and PRG readers are gone.

Two checks:
- `tools/test_statics.py` fails if the generated file differs from what
  the extractor writes today.
- A `piney-data` disc test checks that every table row's model, clump and
  animation exists in each scene it lists, and that every `posname` is a
  DummyPos or DummyPosRot there.

## In the viewer

The viewer draws each file in one of three ways:
- **Table models:** in their own space plus the translation.
- **Table objects:** posed by frame 0 of their animation, under the
  dummy's position and rotation.
- **Everything else:** as before.

`town01` now reads as Mac Anu from any side: the Chaos Gate at the north
end, the bridges, the plazas, the canal grid, banner ropes across the
squares and flags on their posts. T shows only the table-placed pieces.

That fixed a second thing the user saw, grey bars crossing the canal.
Those were morph targets. A Morpher chunk (0x1900) is only `u32 morpher,
u32 base model`. The targets are named by the `F_Morpher` records (0x1901)
in the Anime chunks: `u32 morpher, u32 count, count x (u32 target, f32
weight)`, and every record's size agrees with that. Targets have `mtype`
0x600/0x601 (positions and normals, no colour, no ST). 820 of the 835 such
models in `DATA.BIN` are named by an `F_Morpher` record in their own file.
They are never drawn, and the viewer now skips them.

The LOD copies (`MDL_obj_02lod`, `_05lod`) share their districts' dummies;
the viewer draws the full versions.

**Still unknown:** the canal water: `ROOTTOWN01`'s constructor loads `wat1`,
duplicates `OBJ_wat00` three times and places the copies with
`SetMatrix_PosRotZYX` from values in its own code, which are not read yet;
the sky and `CMP_sr1dat1_*` clumps the constructor builds directly; which
LOD `DrawObj` picks, and the flags at `this+0x1b0`/`+0x1b4` that skip
districts; the 15 morph-target-shaped models no `F_Morpher` names in their
own file; animation beyond frame 0 - `--anime` and A only choose which
animation poses bone and skin models, and in a town, where the tables
place everything, they change nothing.
