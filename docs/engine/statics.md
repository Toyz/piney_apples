---
title: Town and event-area assembly
status: partial
volumes: INF
covers: INF gcmn.prg:0x005cffd0 STATICMODEL::STATICMODEL, 0x005d01c0 STATICMODEL::Draw, 0x005cf9b0 STATICOBJECT::STATICOBJECT, 0x005cfe70 STATICOBJECT::Draw, 0x00421470 ROOTTOWN01::ROOTTOWN01, 0x00423b10 ROOTTOWN01::Draw, 0x00422ba0 ROOTTOWN01::DrawFloor, 0x005d46b0 RT_MODELTABLE00, 0x005d4930 RT_OBJTABLE00, 0x004240c0 ROOTTOWN02::ROOTTOWN02, 0x004266a0 ROOTTOWN02::Draw, 0x005d4bb0 RT_MODELTABLE02, 0x005d4d40 RT_OBJTABLE02
worklog: 27, 28, 96, 97, 112, 218, 260
---

# Town and event-area assembly

A town's scene file (`town01` is Mac Anu) holds each piece in its own
space: the floor, building and detail pieces of every district overlap at
the origin. The game assembles them from tables in `gcmn.prg`. One C++
class per town (`ROOTTOWN01`-`05`) and per event area (`EVENTAREA01`...)
builds its pieces from those tables and draws them itself.
The port's `piney-gen` (`placement::statics`) reads the tables into the
build as data, and `crates/piney-data/src/statics` holds their types.

## Static models

```
STATIC_MODEL_INFO   0x14 bytes, e.g. RT_MODELTABLE00 (32 rows), EA_MODELTABLE01
  +0x00  int     type        the draw pass: 1 floor, 2 obj, 3 obj2 (0 other)
  +0x04  char *  modelname   an MDL_ object in the scene file
  +0x08  int     postype     0 none, 1 a DummyPos, 2 a DummyPosRot
  +0x0c  char *  posname     the DMY_ object that gives the position
  +0x10  float   clip        drawn only within this distance of the camera
```

- **Construction.** `STATICMODEL::STATICMODEL` (0x005cffd0) looks the model
  up by name and copies the dummy's position; for postype 2 it also copies
  the rotation.
- **Drawing.** `STATICMODEL::Draw` (0x005d01c0) builds `sceVu0UnitMatrix`
  then `sceVu0TransMatrix(position)` and draws it. The rotation is not
  applied, so a piece is its model translated to its dummy. It computes the
  distance to the camera and never uses it: `clip` culls nothing.
- **Mac Anu.** In `RT_MODELTABLE00`, `MDL_floor_NN`, `MDL_obj_NN` and
  `MDL_obj2_NN` all sit at `DMY_floor_NNpos`: district 01 at (0, 9600, 0),
  03 at (−6000, 0, 0), 04 at (6000, 0, 0), and so on. The LOD copies
  `MDL_obj_02lod` and `_05lod` share their districts' dummies and replace
  rows 8 and 11 when the camera is far enough south (below).
- **Which pieces draw.** `ROOTTOWN01::DrawFloor` (0x00422ba0) walks its 32
  `STATICMODEL`s and draws those of type 1, skipping districts by
  `clip[3]` (`this+0x1b0..+0x1b8`), which `ROOTTOWN01::Draw` sets each frame
  from the camera eye: `clip[0]` on the gate plaza, `clip[1]` on the south
  plaza, `clip[2]` beyond y 1600 (`0x44c8 << 16`). The rules are on
  [the field game page](field-game.md#what-the-town-draws) and in
  `crates/piney-world/src/town01.rs`, whose `MacAnu::select` gives what
  the game's `Draw` gives for any camera eye (run side by side in eemu,
  `tools/test_world_rs.py`).

`type` is the draw pass: `ROOTTOWN01::DrawFloor`, `DrawObj` and `DrawObj2`
(0x00422ba0, 0x004224c0, 0x00422900) draw the rows whose type is 1, 2 and
3, and `DrawObj2` draws the static objects of type 3 too. In Mac Anu,
`ROOTTOWN01::Draw` (0x00423b10) draws the type-0 rows itself: the static
objects of type 0 (the ships) first, the model rows of type 0 (the tower
`MDL_sr1tow01`) last. Dun Loireag's `ROOTTOWN02` (0x004240c0, `Draw`
0x004266a0) draws its 20 rows by type without clip rules, its static
objects of types 2 and 3 in `DrawObj` and `DrawObj2`, and the model rows
of type 0 last (it has none): [Dun Loireag](town02.md). The other
classes' type-0 rows have not been traced. The port carries `type` as `DrawPass`, and `postype`
with `posname` as `Position` (none, a DummyPos or a DummyPosRot by name).

## Static objects

```
STATIC_OBJ_INFO     0x18 bytes, e.g. RT_OBJTABLE00 (11 rows)
  +0x00  int     type
  +0x04  char *  clumpname   a CMP_ clump, or NULL
  +0x08  char *  anmname     the ANM_ animation that moves it
  +0x0c  int     postype     as above
  +0x10  char *  posname
  +0x14  float   clip
```

`STATICOBJECT::STATICOBJECT` (0x005cf9b0) creates the animation, or the
clump when the row names no animation. It sets its root matrix with
`ccCoord::SetMatrix_PosRotZYX(position, rotation)` from the dummy (T Rx Ry
Rz, the rotation turned to radians when the chunk is decoded), so one
animation can stand at several dummies. `STATICOBJECT::Draw` draws within
`clip` of the camera eye (in the ground plane) and steps the animation
only then. `piney_data::statics` places both (`ModelTable::place`,
`StaticObj::root`); `piney-world`'s town builds each root as the game
stores it (`sceVu0RotMatrix` of the dummy's radians, `pi * deg / 180` as
`Decode_DummyPosRot` converts them, then the position), matching the
game's matrices bit for bit. Mac Anu's six flags are
`ANM_sr1fla1_a` at `DMY_sr1fla1_01pos`..`06pos`, and its two ships are
`ANM_sr1shi1_a` at `DMY_sr1shi01`/`02`. `STATICOBJECT::Draw` advances the
animation, then draws the animation and the clump.

## The files a town loads

Each constructor asks `ccStream::GetCCSAdrs` for its scene files by name.
`piney-gen` (`placement::statics`) reads the names out of the code: the
`$a0` string built just before each call.

| class | files |
| --- | --- |
| `ROOTTOWN01` | `town01d` or `town01`, `wat1` |
| `ROOTTOWN02` | `town02d` or `town02`, `town_z` |
| `ROOTTOWN03` | `town03d` or `town03` |
| `ROOTTOWN04` | `town04d` or `town04`, `town_z` |
| `ROOTTOWN05` | `town05`, `town_z` |
| `EVENTAREA02`, `04`-`07` | the area's file, `town_z` |
| `EVENTAREAB0` | one of `se1_5`, `se2_3`, `se2_4`, `se3_2`, `se3_4`, `se4_3`, `se4_5`, `se4_7`, `se4_8` |

`EVENTAREAB0` picks its stage by `game.field` (fields 1-8; field 8 adds
`se4_8`), through the constructor's jump table: [the story maps](evarea.md).

The `d` files are the towns in crisis. `ROOTTOWN01` (0x004215a4) takes
`town01d` when `saveData+0x6772` - the `crisis` byte - is set and `town01`
otherwise, and keeps the stream at `this+0x1a4`. `wat1` is loaded either
way; it holds the canal water, one plane (`MDL_wat00`, x ±7,800, y −8,700
to 8,100, about 100 below the streets) with 15 morph targets for its waves.
The constructor makes three `ccAnm`s of `ANM_sr1wat1_a` (named by the
global `wateranmname`), duplicates `OBJ_wat00` in each, and roots all three
at the identity (the constants at 0x005d4a90 and 0x005d4aa0 are all zeros
with w = 1). `town_z` holds no models: its seven Eff chunks are the lens
flare's `EFF_sflenz_1`-`6` and the clouds' `EFF_srzsmo1`, each with its
texture ([Dun Loireag](town02.md)).

The three waters are all drawn in Mac Anu and Dun Loireag: water 0 with
the frame-buffer copy behind it (the refraction), waters 1 and 2 with their
UVs scrolled, water 1 in U and water 2 in V ([Dun Loireag](town02.md)).
`ROOTTOWN01`'s constructor also builds the sky `CMP_sr1bac1` and, in
`town01d`, `CMP_sr1dat1_1`-`3` itself; `DrawBG` draws them at the origin:
the sky on `objLayer`, the three over it on `WORLD_MAN`'s layers at +0x49c,
+0x4a0 and +0x4a4 (priorities -80, -70, -60), `MAT_sr1dat1_2` scrolled in U
and `MAT_sr1dat1_3` in U and V by 0.002 a frame
(`crates/piney-world/src/town01.rs`, `CRISIS_SKY`). `ROOTTOWN03`-`05`
(Carmina Gade, Fort Ouph, Lia Fail) are on [their page](root-towns.md);
their `DrawMap` is on [the minimap](map.md) page.

## The tables

`placement::statics` finds every `*MODELTABLE*` and `*OBJTABLE*` object in
`gcmn.prg`: 16 model tables (172 rows) and 11 object tables. It matches
each to the `DATA.BIN` members holding every object it names:

| table | scenes |
| --- | --- |
| `RT_MODELTABLE00`, `02`-`05` | `town01`-`town05`, with the `d` variants of 01-04 |
| `EA_MODELTABLE01`... | `se1_1`, `se1_2`, `se2_1`, `se2_6`, `se3_5_1`, `se1_7_1`, `se1_7_2`, `se1_5`, `se1_6` |

`EA_MODELTABLE02` and `0202` both fit `se1_2`: the two blocks of area 15's
map, the holy ground and the church, which `EVENTAREA02::ChangeBlock`
builds in turn ([area 15's story map](evarea.md)).

## Unknown

- What the first water copy's different `Duplicate` flag (8200 against
  8192) changes.
