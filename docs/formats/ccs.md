---
title: The CCSF scene file
status: solid
volumes: INF
covers: INF DATA/DATA.BIN::*.cmp, INF STREAM/*.BIN::*.tmp, INF SLUS_202.67:0x00149d20 DecodeHeaderSection, 0x00149ff0 DecodeSetupSection, 0x0014d9a0 Decode_Texture, 0x0014d660 Decode_Clut, 0x0014d3e0 Decode_Camera, 0x0014d440 Decode_Light, 0x0014ce50 Decode_Hit, 0x0014cca0 Decode_Eff, 0x0014a3f0 Decode_FBRect, 0x0014a370 Decode_FBPage, 0x0014c5d0 Decode_Layer, 0x0014c8f0 Decode_Obj2, 0x0014c550 Decode_Morpher, 0x0014d4d0 Decode_DummyPos, 0x0014d570 Decode_DummyPosRot, 0x00136980 ccClutChunk::SetBltData, 0x00136e50 ccTexChunk::SetBltData
worklog: 3, 7, 8, 10, 27, 244
---

# The CCSF scene file

CyberConnect2's chunked container for everything drawn: textures, palettes,
models, materials, animation, cameras, lights, hit volumes and effects. The
engine class that loads it is `ccStream` (`D:\usr\RpgUS\prog\system\libccs2.cpp`).
Every member of [DATA.BIN](data-bin.md) and of the `STREAM/*.BIN` archives is
one, gzip-compressed.

## Layout

All little-endian. The file is a flat sequence of chunks:

```
chunk
  u16       kind             what the dispatcher switches on
  u16       tag              0xcccc, except the Setup chunk (0x0000); ignored
  u32       size             payload length in 32-bit words - NOT used by the
                             game, and wrong for some kinds (see below)
  u8[]      payload
```

**The game never skips a chunk by `size`.** `ccStream::DecodeSetupSection`
(`0x00149ff0`) reads the 8-byte chunk header and calls one decoder per `kind`;
each decoder reads exactly what it needs from the stream. A reader that wants
to find the next chunk has to know each decoder's length rule. `size` is right
for every kind except textures and `mtype & 4` models (see
[the model chunk](ccs-model.md)); with those two rules every CCSF file on the
Infection disc - 1,023 in `DATA.BIN`, 266 in `STREAM/` - walks to its last
byte.

The file is three sections: header (`0x0001`), index (`0x0002`), setup
(everything up to and including the first Frame chunk, `0x0005`), then the
frame section.

## Header, 0x0001

Read by `ccStream::DecodeHeaderSection` (`0x00149d20`); `size` 13 in all 1,023
`DATA.BIN` files.

```
char[4]   magic            "CCSF"
char[32]  name             equals the archive member name without extension
u16       version          ccStream+0x184; below 0x90 is a deliberate crash
          padding to 4
u32       frames           ccStream+0x16c; length of the frame seek table at
                           ccStream+0x48 (see the frame section)
u32       count            ccStream+0x44
u32[count] words           ccMalloc'ed array at ccStream+0x3c
```

`version` is 0x100 in 999 files and 0x90, 0x92, 0x95 or 0x96 in 24; decoders
compare it against 0x90 and 0x92. `count` is 1 and the word is 0 in all 1,023.

## Index, 0x0002

```
u32       file_count       includes the blank entry 0
u32       object_count     includes the blank entry 0
file[file_count]
  char[32]  path           ' ', or '#' for an object defined in another file
                           loaded alongside; then the artist's source path
object[object_count]
  char[30]  name           PREFIX_name
  u16       file           index into file[]
```

`size` is always `2 + 8 * (file_count + object_count)`. Entry 0 of both tables
is all zeros. Chunks name their object by its index in `object[]`, and the
engine keeps one 0x40-byte `ccChunkIndex` per object at `ccStream+0x30`
(`index * 64`).

Object name prefixes across all 1,023 files: `OBJ_` 244,315, `MDL_` 33,661,
`MAT_` 11,071, `CMP_` 7,254, `CLT_` 5,538, `TEX_` 4,306, `ANM_` 4,091, `DMY_`
2,529, `HIT_` 2,388, `LGT_` 421, `BLT_` 290, `BOX_` 237, `MPH_` 233, `EFF_` 171,
`CAM_` 20, `FBR_` 2, `PAG_` 1.

## Chunk kinds

The complete dispatch table of `DecodeSetupSection`. An unknown kind executes
`sw $zero, 0($zero)`.

| kind | decoder | | kind | decoder |
| --- | --- | --- | --- | --- |
| 0x0003 | `Decode_Setup` | | 0x0c00 | `Decode_Bbox` |
| 0x0005 | `Decode_Frame` - ends the setup section | | 0x0d00 | `Decode_Particle` |
| 0x0100 | `Decode_Obj` | | 0x0e00 | `Decode_Eff` |
| 0x0200 | `Decode_Material` | | 0x1000 | `Decode_BltGrp` |
| 0x0300 | `Decode_Texture` | | 0x1100 | `Decode_FBRect` |
| 0x0400 | `Decode_Clut` | | 0x1200 | `Decode_FBPage` |
| 0x0500 | `Decode_Camera` | | 0x1300 | `Decode_DummyPos` |
| 0x0600 | `Decode_Light` | | 0x1400 | `Decode_DummyPosRot` |
| 0x0700 | `Decode_Anime` | | 0x1700 | `Decode_Layer` |
| 0x0800 | `Decode_Model` | | 0x1800 | `Decode_Shadow` |
| 0x0900 | `Decode_Clump` | | 0x1900 | `Decode_Morpher` |
| 0x0a00 | `Decode_ExtObj` | | 0x2000 | `Decode_Obj2` |
| 0x0b00 | `Decode_Hit` | | 0x2200 | `Decode_Pcm` |

### Small chunks

```
0x0003 Setup      no payload; size 0 in all 1,289 files
0x0005 Frame      u32 frame count (ccStream+0x168 = count - 1); ends setup
0x0100 Obj        u32 object, u32 parent, u32 model;
                  u32 shadow model if version >= 0x96
0x0900 Clump      u32 object, u16 n, padding to 4, u32 node[n]
0x0a00 ExtObj     u32 object, u32 parent, u32 target (what animation drives)
0x0200 Material   u32 object, u32 texture, f32 transparency, u16 cropU, u16 cropV
                  (the first chunk for an object wins; crops: see below)
0x0c00 Bbox       u32 box, u32 target, f32 min[3], f32 max[3]
0x1300 DummyPos   u32 object, f32 pos[3]
0x1400 DummyPosRot u32 object, f32 pos[3], f32 rot[3] (degrees)
0x1900 Morpher    u32 morpher (MPH_), u32 base model (MDL_)
0x2200 Pcm        16 bytes + 4 * u32(+8) * u32(+12)
0x0500 Camera     u32 object (CAM_); the camera's values are all in
                  its Anime chunk's F_Camera records
0x0600 Light      u32 object, s16 type, pad to 4
                  type 1 distant, 2 direct, 3 spot, 4 omni; the values
                  are in the Anime chunk's light controllers
0x2000 Obj2       u32 object, u32 flags, u32 modifier, u32 layer, u32 slayer
                  flags (ccObj2Chunk.succession): bit 0 transparency
                  inherits the parent's, bit 1 hidden while at the origin;
                  modifier: the MPH_ morpher; layer, slayer: the draw and
                  shadow layer objects (0 for the default)
0x1700 Layer      u16 n, pad to 4, n x (u8 kind, pad to 4, u32 object)
                  kind 0 a draw layer, 1 a shadow layer
0x0b00 Hit        CCSTRM_HIT: u32 object, u32 parent, u16 groups, s16 pad,
                  u32 total vertices; then per group:
                    u32 vertices (3 per triangle), u32 attribute,
                    f32 pos[vertices][3], f32 normal[vertices][3]
                  the normals are read and not kept
0x0e00 Eff        CCSTRM_EFF (36 bytes), then patNum x 8-byte UV patterns;
                  see the effects page
0x1100 FBRect     CCSTRM_FB_RECT: u32 object, u8 type, pad[3], u16 fx, fy,
                  w, h (pixels)
0x1200 FBPage     CCSTRM_FB_PAGE: u32 object, u32 n, then n x
                  CCSTRM_FB_PAGE_INFO: u8 px, py, pw, ph (GS pages of
                  64 x 32), u8 type, pad[3]
```

`cropU`/`cropV` are not an offset to apply. `ccMaterial::Init`
(`0x001399f0`) keeps its own runtime offset at +0x14/+0x16 and starts it at
0. `ccModel::SetUV(u, v, material, flags)` (`0x0013abe0`, called from
`ccAnm::SetUV` during UV animation) sets that offset to `u - cropU` and
`v - cropV`. So the crop is the reference point of a UV animation, and a
model at rest draws with no offset. They are nonzero in 218 of the 11,071
materials in `DATA.BIN`.

DummyPos and DummyPosRot are named points (`DMY_`) that game code places
things at - a town's pieces, its flags, merchants, event markers; see
[town assembly](../engine/statics.md). A Morpher's targets are named by
the `F_Morpher` records (0x1901) of an Anime chunk: `u32 morpher, u32 count,
count x (u32 target model, f32 weight)`. The targets carry positions only
(`mtype` 0x600/0x601: no colour, no ST); 820 of the 835 such models in
`DATA.BIN` are named by an `F_Morpher` record in their own file.

Every kind above was read in its decoder, and the names are the DWARF's
`CCSTRM_*` structures where the executable has one. The Eff chunk is laid out
on [the effects page](../engine/effects.md#the-eff-chunk-0x0e00), the Hit
mesh's use in [piney-world's `hit.rs`](../../crates/piney-world/src/hit.rs),
the layers and Obj2 on [the stream page](../engine/stream.md), and the
Shadow chunk in [the shadow volumes](../engine/shadow.md#streams).

`Decode_FBRect` and `Decode_FBPage` read their records and keep nothing but
the object's kind. Only `FRAMEBUF.CCS` has them, and no file list names it,
so the game never loads it. It is a picture of the VRAM plan: `FBR_描画領域`
("draw area", type 0) at (0, 0) 1024 x 512, `FBR_Zバッファ` ("Z buffer",
type 1) at (0, 512) 512 x 512, and `PAG_framebuff`'s six page rectangles:
those two (type 0x40), two of type 0x13 at page (8, 16) 1 x 2 and (8, 24)
2 x 4, and two single pages of type 0 at (8, 30) and (8, 31).

In DATA.BIN: 16 Camera, 421 Light, 2,285 Hit, 171 Eff, 1,207 DummyPos,
1,322 DummyPosRot, 233 Morpher, 218,971 Obj2, 2 FBRect and 1 FBPage chunks.
Layer is only in the streams.

### Anime sub-kinds

An Anime chunk (0x0700) is `u32 object, u32 frames, u32 words`, then Top
records and these sub-chunks, all laid out on
[the animation page](../engine/animation.md#the-chunk). Over DATA.BIN's
4,091 Anime chunks:

| kind | record | count |
| --- | --- | ---: |
| 0x0101 | F_Obj, a whole transform | 3,388 |
| 0x0102 | object controllers | 229,107 |
| 0x0108 | F_Note | 2,697 |
| 0x0202 | material U/V controllers | 216 |
| 0x0502 | F_Camera | 4,956 |
| 0x0601 | F_Ambient | 17,764 |
| 0x0603 | distant light controllers | 162 |
| 0x0605 | direct light controllers | 7 |
| 0x0607 | spot light controllers | 1 (`town05`) |
| 0x0609 | omni light controllers | 251 |
| 0x1901 | F_Morpher | 11,086 |
| 0xff01 | Top | 190,663 |

## The frame section

After the Frame chunk, `DecodeFrameChunk` (`0x0014e1b0`) reads a `u16` kind,
pads to 4 (skipping the `0xcccc` half), ignores `size`, and dispatches on its
own table. Every frame decoder reads exactly its size field.

```
0xff01 Top          u32 frame number; opens a frame. -1 ends the section,
                    -2 ends it and resets the scene (str7300.tmp only)
0x0101 F_Obj        52 bytes (CCSTRM_FSET_OBJ)
0x0108 F_Note       12 bytes
0x0201 F_Material   8 bytes, +8 unless flag & 2
0x0502 F_Camera     8 bytes + 4 per clear bit of 0x1fe, or 8 if flag & 1
0x0601 F_Ambient    4 bytes
0x0602 F_DistantLight 24 bytes, +4 if flag & 0x20
0x0604 F_DirectLight 56 bytes     0x0606 F_SpotLight 56 bytes
0x0608 F_OmniLight  36 bytes
0x0802 F_ModelVertex, 0x0803 F_ModelNormal   decoded, never present
0x1801 F_Shadow     20 bytes: u32 obj, f32 rx, ry, rz (degrees), f32 alpha
0x1901 F_Morpher    8 + 8 * n
0x2201 F_Pcm        12 + 4 * blocks(u16 +4) * words(u32 +8)
```

The section is: Frame, then for each frame 0..N-1 a Top and that frame's
chunks, then the end Top. `DecodeF_Top` (`0x0014e490`) records each Top's
address in the seek table while the frame number is below the header's
`frames`. Every `DATA.BIN` file has an empty section (Frame = 1, then the end
Top). The `STREAM/` cutscenes come in pairs: `strNNNNe.tmp` with models and
textures, and `strNNNN.tmp` with object stubs, `Pcm` audio and the frames.


### 0x0400, CLUT

`ccStream::Decode_Clut` (`0x0014d660`):

```
u32       clut             object index of the CLT_ entry
u32       blt_group        only if version >= 0x92
u8        flag             only if version >= 0x90, and so on to count
u8        cpsm             the GS TEX0 CPSM; 0 (PSMCT32) in all 5,538
u8        unknown_2        read and discarded
u8        unknown_3        read and discarded
u16       buf_x            VRAM placement; 0/0 or x < 768 becomes 896/992
u16       buf_y
u32       count            16 or 256 in the files
u32[count] colour          R, G, B, A; A is 0x80 for opaque
```

`cpsm` goes to `ccClutChunkDesc.cpsm` and from there into bits 51-54 of the
chunk's TEX0 (`ccClutChunk::SetBltData`, 0x00136980). The palette's `psm` is
not in the file: over 16 colours it is PSMT8 (0x13), else PSMT4 (0x14).
`unknown_2` and `unknown_3` are 0 in every CLUT but one (0 and 112).

Colours are in logical order. For a 256-colour palette the decoder writes
colour `i` to slot `clut256Tbl[i]` (`0x002fb190`), which is the GS's CSM1
arrangement; a converter should use the file order as it is.

### 0x0300, texture

`ccStream::Decode_Texture` (`0x0014d9a0`):

```
u32       texture          object index of the TEX_ entry
u32       clut             object index of its CLT_ entry, always in the
                           same file (all 4,306 textures in DATA.BIN)
u32       blt_group        only if version >= 0x92
u8        flag             low byte of ccTexChunkDesc.flag
u8        psm              0x13 PSMT8 or 0x14 PSMT4 in every file surveyed
u8        mipmap           only if version >= 0x90: levels after level 0
u8        aref             only if version >= 0x90: alpha-test reference
u8        tw               only if version >= 0x90: log2 width
u8        th               only if version >= 0x90: log2 height
          padding to 4
level[mipmap + 1]          at most 4 (ccTexChunkDesc.pos[4])
  u16     buf_x            VRAM placement; 0/0 or x < 768 takes a default
  u16     buf_y            from warnPos (0x0034ac90)
  u32     count            words that follow
  u32[count] pixels
```

### Texture and CLUT flags

The file's `flag` byte becomes the low byte of the descriptor's `flag`.
`ccTexChunk` keeps `flag & 0x1d`, `ccClutChunk` `flag & 5`:

| bit | texture | CLUT |
| --- | --- | --- |
| 0x01 | the pixels stay in memory and are not sent at load | the same |
| 0x04 | kept, never read | kept, never read |
| 0x08 | translucent: its mmats go to the layer's sorted group | dropped |
| 0x10 | CLAMP both axes (register value 5), else REPEAT | dropped |

- **0x01.** Without it, the decoder uploads every level (`ccBltData::LoadData`)
  and frees the pixels (`DelBltData`, which clears bits 0 and 1). With it,
  the pixels are kept for the blt groups to send. The decoder also sets it
  when a level's (or the palette's) VRAM place is 0/0 or has `x` below 768,
  after moving it to the default.
- **0x04.** The decoder sets it on a palette with an alpha other than 0 or
  0x80, and on a texture without a palette whose alpha bytes are all 0 or
  0x80 (no texture in DATA.BIN lacks one). Nothing reads it afterwards.
- **0x08.** `ccModel::Draw`, `DrawBoneType` and `ccEff::Draw` treat a mmat
  (or sprite) whose texture has it as translucent even at full
  transparency ([the desktop's draw order](../engine/desktop.md)).
- **0x40** in the descriptor stops `SetBltData` making the pixel buffer at
  all; no file sets it.

Over DATA.BIN's 4,306 textures: 0x01 on 4,229, 0x04 on 290, 0x08 on 173,
0x10 on 30; no other bit. Over its 5,538 palettes: 0x01 on 5,337, 0x04 on
313, 0x08 on 2 (`xddesk01`).

The real length is the header plus `2 + count` words per level; the `size`
field disagrees (`xasc00`: 0x2039 declared, 0x2007 read). Level 0 is exactly
`(1 << tw) * (1 << th)` pixels at 8 or 4 bits in all 4,306 textures in
`DATA.BIN`.

Pixels are row-major and **bottom row first**: the art was authored as `.bmp`
and the rows kept BMP order. At 4 bits the low nibble is the left pixel.
Mip level `n` is `(w >> n) x (h >> n)`. Over all 4,306 textures in
`DATA.BIN`: `psm` 0x13 4,046, 0x14 260; `mipmap` 0 in 2,146, 3 in 1,807, 2 in
353. (Worklog 8's survey reached only 3,985 of them; worklog 10 fixed the
model walk that stopped it.)

Worked example, `INF DATA/DATA.BIN::xasc00.cmp` at +0x150:

```
00 03 cc cc  39 20 00 00      kind 0x0300, size 0x2039 (not the real length)
02 00 00 00                   texture = object 2, TEX_xasc00
01 00 00 00                   clut = object 1, CLT_xasc00
00 00 00 00                   blt_group 0 (version 0x100 >= 0x92)
00 14 00 70                   flag 0, psm PSMT4, mipmap 0, aref 0x70
08 08 cd cd                   tw 8, th 8 (256x256), padding
00 03 00 00                   level 0: buf_x 768, buf_y 0
00 20 00 00                   count 0x2000 words = 32,768 bytes of 4-bit pixels
```

## Notes

`0xcd` padding bytes are the Microsoft C debug heap's fill for uninitialised
memory: the exporter that wrote these files was a debug build on Windows.

`tools/ccs.py survey ARCHIVE` walks every member and counts chunks that
disagree with their size field; `tools/ccstex.py` converts textures to PNG and
`tools/ccsmodel.py` models to OBJ.

## Unknown

None.
