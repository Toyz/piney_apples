---
number: 10
title: Model chunks, the frame section, and a clean walk over every archive
date: 2026-09-22
area: format, render, decomp, tooling
files: tools/ccs.py, tools/ccsmodel.py, tools/test_ccs.py, docs/formats/ccs.md, docs/formats/ccs-model.md
resolves: 7, 8
---

# 10. Model chunks, the frame section, and a clean walk over every archive

[[7]] left the chunk walk stuck after Model chunks in 176 of 1,023 files and
did not know what `0xccccff01` was. Both are now read out of the code, and
every CCSF file on the disc walks to its exact last byte:

| archive | members clean | members with frames | frames |
| --- | ---: | ---: | ---: |
| `DATA/DATA.BIN` | 1,023 / 1,023 | 0 | 0 |
| `STREAM/STRCMN.BIN` | 93 / 93 | 50 | 8,494 |
| `STREAM/STRCMNE.BIN` | 93 / 93 | 50 | 8,494 |
| `STREAM/STR1.BIN` | 40 / 40 | 22 | 38,569 |
| `STREAM/STR1E.BIN` | 40 / 40 | 22 | 38,569 |

Only two chunk kinds need a rule other than the size field: textures ([[7]])
and one kind of model. `tools/ccs.py survey` reports how many chunks disagree
with their size field: all 4,306 textures in `DATA.BIN`, and 190 models.

## The Model chunk

`ccStream::Decode_Model` (`INF SLUS_202.67:0x0014bce0`) reads a header -
object, `f32 vertexScale`, `u16 mtype`, `u16 mmatNum`, `u16 flag`,
`s16 zoffs`, a byte that is 0x70 in every file, padding - and then `mmatNum`
"mmats", each a run of vertices sharing one material. `mtype` picks one of
three layouts:

- `mtype & 4`: `Decode_Mmat02` (`0x0014a600`), drawn by `ccModel::DrawBoneType`.
  With `offsetNum` 0 every vertex rides one clump node ("bone"); otherwise each
  vertex is one or more weighted entries, one per node, each holding the
  position in that node's space ("skin"). A weight is 9 bits (256 = 1.0) of a
  `u16` that also carries an end-of-vertex bit and a 6-bit node slot.
- `mtype & 8`: `DecodeShadowModel` (`0x00140920`), positions and a triangle
  list for the shadow volume.
- otherwise: `Decode_Mmat01` (`0x0014b890`), rigid vertices in the owning
  object's space, with normals, colours and ST each present unless an `mtype`
  bit (0x40, 0x200, 0x400) turns them off.

The full layout is on [the model page](../docs/formats/ccs-model.md). The size
field disagrees only for `mtype & 4` models, and always by exactly 15 words per
mmat more than the decoder reads; why is not known.

### Units, and how they were settled

- Positions are `s16 * vertexScale / 4096`. `Decode_Mmat02` converts with
  `sceVu0ITOF4Vector` and a `vertexScale / 256` multiply, and
  `ccBbox_SetBox` (`0x001388c0`) uses `/ 4096` for every model type. Rigid
  vertices go to the VU as `s16` with `vertexScale` folded into the matrix
  (`ccSetMatrixPacket`); that the microcode then does an ITOF12 is inferred
  from the bounding-box factor, not read from VU code.
- Normals are `s8 / 64`; their measured length is 64. The fourth byte of the
  normal word is the GS strip flag: 1 starts a strip without drawing, 0 draws
  a triangle with the two vertices before it. All 1,212,751 strips open with
  two 1s. Strips run on across the 48-vertex VU batches
  (`ccModelDmaTag_SetVertex`, `0x0013dae0`), because the GS keeps its vertex
  queue.
- ST is `u16 / 256`: the STROW offset in `ccSetModelPacket` wraps one texture
  repeat at 256, 88% of mmats have UVs inside 0..1, and `v = T / 256` lines up
  with the PNGs of [[8]] as they are - T = 0 is the first stored row, which is
  the bottom of the picture. That answers how the UVs deal with bottom-up rows:
  they simply count from the stored first row.
- Colours are RGBA with 0x80 = 1.0.
- The GS does not cull. With winding alternating from each strip's start,
  1.88 million triangles face their stored normals and 0.60 million do not;
  some models are authored inside out.

### Posing

Bone and skin vertices mean nothing until clump nodes are posed. Local matrix
= T * Rx * Ry * Rz * S with rotation in degrees
(`SetMatrix_PosRotZYXScale`, `0x00138120`); world = parent world * local
(`_SetLWMatrix__7ccCoordFv`, `0x00138380`); `DrawBoneType` (`0x0013f860`)
builds each bone matrix as inv(model world) * node world. That node slot `i`
is the `i`-th node of the model's clump is inferred, and borne out by poses
that come out right. Axes are Z-up, right-handed.

The Anime chunk's sub-stream walks exactly by its size field in all 4,091
chunks; the one controller record decoded so far, 0x0102, gives position,
rotation and scale controllers (constant, or `n` keyed values).

## The frame section

`DecodeFrameChunk` (`0x0014e1b0`) reads a `u16` kind, pads to 4 - skipping the
`0xcccc` half - and dispatches on its own table. The Frame chunk (0x0005,
`Decode_Frame`) that ends the setup section holds a `u32` frame count. **Top**
(`0xccccff01`, `DecodeF_Top`, `0x0014e490`) holds a `u32` frame number and
opens each frame; frame -1 ends the section, and -2 ends it and resets the
scene (only `str7300.tmp`). The header `u32` that [[7]] could not name, at
`ccStream+0x16c`, is the length of the seek table at `ccStream+0x48` that
records each frame's Top chunk.

Every file in `DATA.BIN` has an empty frame section: Frame = 1, then the end
Top. Animated `DATA.BIN` scenes (`x74*cam`, the `c*body` characters) animate
through Anime chunks in the setup section instead. The frame section is for
the cutscenes in `STREAM/`, which come in pairs: `strNNNNe.tmp` carries models
and textures, and `strNNNN.tmp` carries object stubs, audio (`Pcm`) and the
frames - every frame has a camera and an ambient-light chunk. Every
frame-section decoder was read, and each reads exactly its size field; checked
over roughly 10.4 million frame chunks.

## Other answers

- The `#` in front of a source path in the index ([[3]]) marks an object
  defined in another file that is loaded alongside, as the stream pairs are.
- The Setup chunk is 0 words in all 1,289 files and `Decode_Setup` reads
  nothing; the reference page's "0 to 2" came from the broken walk of [[3]]
  and is corrected.
- Layouts read along the way: Obj (object, parent, model, and a shadow model
  from version 0x96), Clump (object, count, node list), ExtObj (object, parent,
  and the target the animation actually drives), Material (object, texture,
  `f32` transparency, `u16` crop U/V), Bbox (a hand-placed culling box, two
  `f32[3]` corners).

With models passable, `tools/ccstex.py dump` now reaches all 4,306 `TEX_`
objects in `DATA.BIN`, where [[8]] reached 3,985.

## Tools and checks

`tools/ccsmodel.py` decodes models and writes Wavefront OBJ, optionally posed
from an Anime chunk (`--anime`) or a stream frame (`--frame N --frames-from
strNNNN.tmp`), with an MTL naming the `ccstex.py` PNGs. `ccsmodel.py check`
decodes every model in an archive and requires each decode to end exactly at
the walker's `model_end`, indices in range, no NaNs, strips opening correctly
and skin weights summing to 1. `DATA.BIN`: 17,254 models - 16,407 rigid,
3,129 shadow, 2,994 bone and 185 skin mmats - 5,417,645 vertices and
2,819,008 triangles, no problems. `STRCMN` 1,344 and `STR1` 6,029 models, no
problems. `tools/test_ccs.py` (8 tests) holds the survey totals and one model's
counts.

Looked at by eye:

- `cw1hst00`, a weapon: 118 vertices and 68 triangles, a rapier with a curved
  guard; its shadow model is 17 vertices and 26 triangles.
- `cbu1body` posed with `ANM_cbu1nut0`: 21 bone mmats plus a skin of 1,137
  vertices, an armoured figure with sword and shield standing from z -0.34 to
  163.2, inside its hand-placed box (+-90, +-80, 0 to 230). The face mesh's UVs
  land on the face texture.
- `town01` posed with `ANM_sr1town1a`: a recognisable town.
- Kite and Helba from `str0130e` + `str0130`, posed at frames 100 and 1,000.

109 of 168 posed skinned models fit inside their Bbox; the rest miss by a few
units, mostly feet just below z = 0. A wrong scale would miss by a factor.

**Still unknown:** the VU1 microcode, so the rigid ITOF12, the ST scale and
any culling are inferred from the EE side; the 15 extra words per mmat in the
size field of `mtype & 4` models; the `flag` bits above blend type and what
`zoffs` does; Anime sub-kinds other than 0x0102; carried over from [[7]] and
[[8]], the texture and CLUT `flag` bits and the CLUT's `unknown_1`.
