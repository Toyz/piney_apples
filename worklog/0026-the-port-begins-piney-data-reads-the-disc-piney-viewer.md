---
number: 26
title: The port begins: piney-data reads the disc, piney-viewer draws it
date: 2026-09-23
area: build, render, test
files: Cargo.toml, rustfmt.toml, crates/piney-data, crates/piney-viewer, docs/formats/ccs.md, README.md
---

# 26. The port begins: piney-data reads the disc, piney-viewer draws it

The formats of [[3]], [[7]], [[8]] and [[10]] are decoded well enough to
build on, so the port starts. It is Rust, a Cargo workspace under `crates/`,
with crates named `piney-*`. It uses crates.io libraries - wgpu 30, winit
0.30, glam, flate2, gilrs - and nothing else of the user's.

## piney-data

`piney-data` is the Python readers carried over to Rust, with nothing added:
- `iso`: ISO 9660, read from the image directly.
- `archive`: `DATA.BIN` members found the way `tools/gzarc.py` finds them,
  as gzip headers on sector boundaries. So it needs no executable and works
  on any volume.
- `ccs`: the header, the name table, and the chunk walk with the texture and
  model length rules of `tools/ccs.py`, including the frame section.
- `texture`: palettes and PSMT4/PSMT8 textures, kept in stored row order.
- `model`: all four mmat layouts, with strip triangles.
- `scene`: Obj parents, clumps, materials, ExtObj targets, frame 0 of an
  Anime chunk, world matrices, and placed geometry, as `tools/ccsmodel.py`
  does it.

Reads are bounds-checked and return errors instead of panicking.

`cargo test` walks the whole of Infection's `DATA.BIN` in about a second and
checks the totals the Python tools measured:
- 1,023 members and 338,646,324 inflated bytes, every file walking to its
  last byte;
- 17,254 models: 16,407 rigid, 3,129 shadow, 2,994 bone and 185 skin mmats;
- 5,417,645 vertices and 2,819,008 triangles;
- every texture's level 0 exactly `w * h * bpp` bits.

All match except one, and the mismatch was in the docs. `ccs.md` said 3,985
textures (PSMT8 3,741, PSMT4 244). That count dates from [[8]], whose walk
stopped early in files until [[10]] fixed the model lengths. Re-running
`tools/ccstex.py` over the archive gives what the Rust reads:
- 4,306 textures: PSMT8 4,046, PSMT4 260;
- mip levels 0, 2 and 3 in 2,146, 353 and 1,807 textures;
- every texture's palette in its own file.

`ccs.md` is corrected.

## piney-viewer

`piney-viewer` reads `DATA.BIN` from the disc image and shows one scene file
at a time:
- every model, textured;
- bone and skin models posed from frame 0 of the file's first Anime chunk;
- A cycles the poses, M shows one model at a time;
- Left/Right, or L1/R1 on a gamepad, steps through the 1,023 files.

Faithful to the GS:
- There is no back-face culling, and lighting is two-sided, since winding is
  not consistent ([[10]]).
- Textures are uploaded in stored row order and sampled with v = T / 256,
  with no flip.
- Vertex colours use 0x80 = 1.0, and alpha is the material's transparency
  (VU1 writes `128 * t`).
- Each model draws with its blend mode.

The first version drew everything with plain alpha blending, and the user
said it looked mangled. It was, in three ways:
- **Blend modes.** `Decode_Model` keeps `flag & 3`, and `ccModel::Init`
  (`INF SLUS_202.67:0x0013a4f8`) loads `alphaBlendTbl[flag & 3]` into the GS
  `ALPHA` register. Decoded, entry 0 is `(Cs - Cd) * As + Cd`, 1 is
  `Cs * As + Cd` (additive), 2 is `Cd - Cs * As` (subtractive) and 3 is
  `Cd * As + Cs`. Types 1-3 get `SetRenderState(1, 0)` where type 0 gets
  `(1, 1)`, most likely depth writes off. 885 models use 1-3 (799, 83, 3):
  fire, glows, the title and hack effects. Drawn as ordinary alpha they
  were black quads.
- **Mipmaps.** Without them the towns' textures shimmered to noise at a
  distance. Each texture now gets a box-filtered chain; the files' own mip
  levels are not used yet.
- **The starting file.** With no argument it opened the first file with a
  model, `xdl_load`: a 2D loading card seen edge-on. It now opens `town01`.

Two things that look wrong are the data:
- The dark floors and walls in `town01` and `sd1` are unlit (mtype 0)
  models with baked vertex colours around 60/128 over dark textures. VU1
  passes unlit colours through unchanged ([the render page](../docs/engine/render.md)),
  and the only other user of `alphaBlendTbl`, `ccBufferSampling::SetShade`,
  is called by a single cutscene function, so nothing brightens the frame
  as a whole.
- `zoffs` is 0 in every model.

`Material` chunks are now read in full: texture, transparency and the
`cropU`/`cropV` offsets. The decoder keeps the first chunk for an object
(0x0014d36c). The crops are nonzero in 218 of 11,071 materials, all
animated UVs and eyes; their unit is not traced and the viewer ignores
them.

The camera frames the 2nd to 98th percentile of the vertices, so a town's
sky dome does not decide the view. Input: mouse, keyboard, or a gamepad
through gilrs (left stick orbits, right stick pans, L2/R2 zoom).

`--shot OUT.png NAME` renders one frame offscreen with no window, which is
how this was checked (`--cam YAW,PITCH,ZOOM` moves the camera):
- `town01` shows Mac Anu under its sunset dome.
- `e1f1`, an enemy knight, comes out skinned, posed from `ANM_e1f1atc0` and
  textured.
- `sd1` is a dungeon's library of room parts, all at the origin, since the
  generator of [[19]] places them.
- `field_b`, a field landmark, has its additive fire drawn as light, not
  black.

A test flattens all 1,023 files and checks every index and texture slot.

**Still unknown:** whether the game really shows `town01`'s floors that dark
(nothing compared against the running game); the unit of the material crop
offsets; the lit models' lighting (the viewer uses a headlight, not the
scene's `LGT_` lights and VU1's three-light formula); `town01` has no water,
presumably because it is in another file (`wat1`?) loaded alongside;
textures defined in other files (`#` entries) show white; animation playback
needs the key interpolation, which has not been read. (The window path,
untested here, was run by the user: it works, and it showed the placement
problem [[27]] fixes.)
