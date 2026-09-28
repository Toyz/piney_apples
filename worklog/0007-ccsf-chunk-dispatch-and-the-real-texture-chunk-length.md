---
number: 7
title: CCSF chunk dispatch and the real texture chunk length
date: 2026-09-22
area: format, decomp, render
files: tools/ccs.py, docs/formats/ccs.md
supersedes: 3
---

# 7. CCSF chunk dispatch and the real texture chunk length

[[3]] guessed that a texture chunk's size field counts something the file
does not store. That is the wrong way round: **the game never uses a chunk's
size field to find the next chunk**, and for textures it simply disagrees with
the data. [[3]] also guessed the header's first word was a version and the
next a frame count; the first is now confirmed and re-typed as a `u16`.

## The dispatcher

The loader is `ccStream`, in `D:\usr\RpgUS\prog\system\libccs2.cpp`
([[6]]). `ccStream::DecodeSetupSection` (`INF SLUS_202.67:0x00149ff0`) loops:
read `u16 kind`, `u16` (the `0xcccc` half, ignored), `u32 size` from the ring
buffer, then call one decoder by `kind`. Each decoder reads exactly the fields
it needs; nothing afterwards skips to `size`. A kind it does not know executes
`sw $zero, 0($zero)` - a deliberate crash. The table, from the compare chain:

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

Because only the low half is compared, the Setup chunk being stored as
`0x00000003` rather than `0xcccc0003` is harmless.

`Decode_Frame` ends the loop (it sets the state at `ccStream+0x182` to 2 and
calls `CompleteDecodeSetup`); what follows belongs to
`DecodeFrameSection`.

## The header chunk

`ccStream::DecodeHeaderSection` (`0x00149d20`) reads, after the magic:
`char[32] name` into `ccStream+8`; a `u16` into `ccStream+0x184`, which every
decoder compares against constants and which is therefore the format version;
padding to 4; a `u32` into `ccStream+0x16c`; a `u32` count into
`ccStream+0x44` followed by that many `u32`s into a `ccMalloc`ed array. A
version below 0x90 crashes on purpose. In all 1,023 files the count is 1 and
the one word is 0.

## The texture chunk

`ccStream::Decode_Texture` (`0x0014d9a0`) reads:

```
u32       texture          object index of the TEX_ entry
u32       clut             object index of the CLT_ entry
u32       blt_group        only if version >= 0x92
u8        flag             low byte of ccTexChunkDesc.flag; the decoder
                           later ors in 1 and 4
u8        psm              GS pixel storage mode
u8        mipmap           only if version >= 0x90: extra levels after level 0
u8        aref             only if version >= 0x90: alpha-test reference
u8        tw               only if version >= 0x90: log2 width
u8        th               only if version >= 0x90: log2 height
          padding to 4 bytes
level[mipmap + 1]          at most 4 - ccTexChunkDesc.pos has 4 entries
  u16     buf_x            VRAM placement, into ccTexChunkDesc.pos[level];
  u16     buf_y            0/0 or x < 768 takes a default from warnPos
                           (0x0034ac90)
  u32     count            words that follow
  u32[count] data
```

The names are `ccTexChunkDesc`'s, from the DWARF: `u16 flag`, `u8 psm, tw,
th, mipmap, aref`, `ccBuffPos pos[4]`, 0x18 bytes. The file stores the bytes
in a different order from the struct.

So a texture chunk's real length is its header plus, per level, 2 words plus
`count`. In `xasc00` that is 0x2007 words against a declared 0x2039. The
decoder also skips an already-loaded texture by reading
`sum(ccGetTexTransSize(psm, w, h) * 4)` words, which must therefore equal the
per-level `2 + count`.

## Survey

`tools/ccs.py` now walks by the size field except for textures, and its
`survey` command runs the walk over a whole archive. Over `DATA.BIN`:
**847 of 1,023 members walk to their exact last byte**. Every one of the other
176 goes wrong immediately after a Model chunk (`0x0800`), so Model is the next
decoder whose length rule must be read. In every clean file the setup section
ends with one `0xcccc0005` Frame chunk and one `0xccccff01`.

Chunks counted before any walk went wrong: `ExtObj` 228,999, `Obj2` 218,971,
`Model` 16,877, `Obj` 14,958, `Material` 10,540, `Clump` 7,193, `Clut` 5,101,
`Anime` 4,090, `Texture` 3,985, `Hit` 2,255, `DummyPosRot` 1,251, `DummyPos`
1,207, `Light` 421, `Morpher` 233, `Eff` 162, `Bbox` 64, `Camera` 16, `FBRect`
2, `FBPage` 1. No `Particle`, `BltGrp`, `Layer`, `Shadow` or `Pcm` chunk was
reached.

**Still unknown:** the Model chunk's length rule; what `0xccccff01` is (it is
not in the setup dispatcher, so the frame section reads it); the meaning of
the texture `flag` bits; the header `u32` at `ccStream+0x16c`.
