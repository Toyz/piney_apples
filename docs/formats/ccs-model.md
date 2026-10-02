---
title: The CCSF Model chunk
status: solid
volumes: INF
covers: INF SLUS_202.67:0x0014bce0 Decode_Model, 0x0014b890 Decode_Mmat01, 0x0014a600 Decode_Mmat02, 0x00140920 DecodeShadowModel, 0x0013a440 ccModel::Init, VU1 micro 0x063 mc_DrawTriFast
worklog: 10, 17, 244
---

# The CCSF Model chunk

Kind 0x0800 in a [CCSF file](ccs.md): one model, made of "mmats" - runs of
vertices that share a material - in one of three layouts chosen by `mtype`.
Read by `ccStream::Decode_Model` (`0x0014bce0`). The chunk's `size` field is
**wrong for `mtype & 4` models** (15 words per mmat too many); use the layout.

## Layout

All little-endian.

```
header
  u32       model            object index, the MDL_ entry
  f32       vertexScale      ccModelChunk+0x40
  u16       mtype            ccModelChunk+0x50; & 0xff if version < 0x100
  u16       mmatNum          0: no model, the decoder returns here
  u16       flag             blendType = flag & 3; the rest dropped
  s16       zoffs            ccModelChunk+0x4e; 0 in every file
  u8        unknown_10       read and dropped; 0x70 in every file
            padding to 4

mmat[mmatNum], each: header words, then a body
```

What the decoder keeps of the header:
- **`flag`.** Only `flag & 3`, as `ccModelChunk.blendType` (+0x4c). Over the
  11,010 models in DATA.BIN with mmats, `flag` is 0 (9,990), 1 (532), 9
  (266), 8 (136), 10 (82), 3 (3) or 2 (1). Bit 3, on 484 of them, is
  dropped at load like any bit above 1.
- **`zoffs`.** `ccModel::Init` (0x0013a440) turns it into the float
  `ccModel.zoffs` (+0x1c), and `ccModel::Copy` and `Duplicate` copy it.
  Nothing else loads either field, so it has no effect; it is 0 in all
  11,010.
- **`unknown_10`.** Read and not stored; 0x70 in all 11,010.

### Header words per mmat

| mtype | words |
| --- | --- |
| `& 2` | `name, material, vertexNum, boneNum, offsetNum` - never in the data |
| `& 4` | `material, vertexNum, offsetNum` |
| `& 8` | none |
| otherwise | `name, material, vertexNum` |

`name` is an object index (`MDL_x_N`) whose index entry the decoder clears;
`material` is the `MAT_` object.

### Body: rigid (`Decode_Mmat01`, `0x0014b890`)

```
s16[3]    position[vertexNum]
          padding to 4
u32       normal[vertexNum]      unless mtype & 0x40
u32       colour[vertexNum]      unless mtype & 0x200; read but dropped if mtype & 1
u32       st[vertexNum]          unless mtype & 0x400
```

`mtype & 2` here is a deliberate crash (`0x0014b8d8`).

### Body: bone and skin (`Decode_Mmat02`, `0x0014a600`)

With `offsetNum` = 0, "bone" - every vertex follows one clump node:

```
u32       node                   clump node slot
s16[3]    position[vertexNum]    in that node's space
          padding to 4
u32       normal[vertexNum]
u32       st[vertexNum]
```

With `offsetNum` > 0, "skin" - a vertex is one or more weighted entries:

```
entry[offsetNum]                 8 bytes each
  s16[3]  position               in the entry's node's space
  u16     w                      bits 0-8 weight (256 = 1.0)
                                 bit 9    last entry of this vertex
                                 bits 10-15 clump node slot
u32       normal[offsetNum]
u32       st[number of vertices] = number of entries with bit 9 set
```

In all data the count of bit-9 entries equals `vertexNum`.

### Body: shadow (`DecodeShadowModel`, `0x00140920`)

```
s32       vNum                   0: nothing follows
s32       iNum
s16[3]    position[vNum]
          padding to 4
s32[3]    triangle[iNum / 3]
```

A closed mesh (every edge shared by two triangles, bar four edges in all of
`DATA.BIN`) that the engine stretches into a shadow volume: [the shadow
volumes](../engine/shadow.md). Every shadow model has one mmat.

## Values

| field | meaning |
| --- | --- |
| position | `s16 * vertexScale / 4096` (VU1 `itof12`, [render page](../engine/render.md)) |
| normal | bytes 0-2: `s8 / 64` each (length 64); byte 3: strip flag |
| strip flag | 1 = start a strip, no triangle; 0 = triangle with the previous two vertices |
| colour | R, G, B, A, 0x80 = 1.0 |
| st | `u16 S, u16 T`, 256 = one texture repeat (see below); T = 0 is the first stored texture row |

**The ST scale.** The model packet unpacks ST as V2-16 in STMOD offset
mode, with STROW `[(u >> 4) & 0xff, (v >> 4) & 0xff, 1/16, 1.0]`
([the draw path](../engine/render.md)). `mc_DrawTriFast` and the other
programs load the slot as xyz, convert only x and y (`itof12.xy`), and
multiply all three by the vertex's Q. The z they send as the GS's Q is the
slot's z taken as float bits. The EE manual calls a V2 unpack's z and w
indeterminate; PCSX2's unpacker, tested on the console, puts x and y there
(`Vif_UnpackSSE.cpp`, "v1v0v1v0"). With the row added in offset mode, z is
then `S + bits(1/16)` as an integer, the float `1/16 * (1 + S * 2^-23)`,
and a vertex's texture coordinate is

```
u = (S + row.x) / (256 * (1 + S * 2^-23))
```

one repeat per 256 to within `S * 2^-23`: 3 * 10^-5 at S = 256, under
0.8% at the largest S. The port takes z as exactly 1/16.

Strips always open with two vertices flagged 1, and run on across the
48-vertex batches the engine splits a mmat into. VU1 turns the flag into the
GS ADC bit. Nothing culls back faces, and winding is not consistent across
models. See [the draw path](../engine/render.md).

After the mmats `ccBbox_SetBox(chunk+0x10, vmm, vertexScale)` (0x001388c0)
stores the model's box: per axis the integer min and max over every vertex
(starting at 0x10000 and -0x10000), first doubled about the centre for
`mtype & 6`. With `unit = vertexScale / 4096`: min at +0x10, max at +0x20,
each `int * unit`, and the centre at +0x30, `((min + max) >> 1) * unit`, w 1.
`ccModel::Init` (0x0013a550) points `ccModel` +4 at it for a model without
`mtype & 6` and leaves it null for a bone or skin model; `ccModel::Draw`
culls by it (`_ccCheckBoundingBoxEx`) and keys the sorted group on its
centre ([the desktop's draw order](../engine/desktop.md#draw-order)). The
centre matches the game's decoder in eemu for all 41 models of `xdttopen0`
and 138 of `xddesk01`.

## Posing

Bone and skin positions are in clump-node space. Local matrix =
T * Rx * Ry * Rz * S, rotation in degrees (`SetMatrix_PosRotZYXScale`,
`0x00138120`); world = parent world * local (`0x00138380`); a vertex lands at
node world * position. Slot `i` is the `i`-th node of the model's clump -
inferred, consistent with every pose checked. Axes are Z-up, right-handed.

## Types seen

| mtype | what |
| --- | --- |
| 0 | unlit, vertex colours (6,244 are empty skeleton placeholders) |
| 1 | lit |
| 5 | bone and skin |
| 8 | shadow |
| 0x600, 0x601 | no colour, no ST |

`DATA.BIN` holds 17,254 models: 16,407 rigid, 3,129 shadow, 2,994 bone and
185 skin mmats, 5,417,645 vertices, 2,819,008 triangles.

## Notes

The size field of an `mtype & 4` model is 15 words per mmat longer than the
chunk. Nothing reads a chunk's size: `DecodeSetupSection` dispatches on the
kind and each decoder reads what its fields say, so the game is not affected.
Why the exporter wrote it so is not on the discs.

## Unknown

None.
