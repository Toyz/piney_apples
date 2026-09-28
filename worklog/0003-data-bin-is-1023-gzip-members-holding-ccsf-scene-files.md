---
number: 3
title: DATA.BIN is 1023 gzip members holding CCSF scene files
date: 2026-09-22
area: format, content
files: docs/formats/data-bin.md, docs/formats/ccs.md
---

# 3. DATA.BIN is 1023 gzip members holding CCSF scene files

`INF DATA/DATA.BIN` (136,665,088 bytes) has no header and no directory. It is
gzip members laid end to end, each starting on a 2048-byte boundary and padded
with zeros to the next. The first bytes of the file are `1f 8b 08 08`, and the
gzip FNAME field right after the fixed header reads `framebuf.cmp`.

Scanning every sector boundary for `1f 8b 08` finds 1,023 members; the magic
occurs 6 more times off-boundary, all inside compressed data. All 1,023 inflate
with Python's `zlib` (wbits 31) with no errors, each ends with between 1 and
2,047 bytes of padding before the next member, and `ISIZE` matches the inflated
length every time. 135,599,928 compressed bytes become 338,646,324. Every member
has FLG `0x08` (FNAME only), every FNAME is lower case ending `.cmp`, and MTIME
runs from 973651006 (2000-11-08) to 1030068730 (2002-08-23).

The game does its own inflating: `ccUngzip` in main, whose methods -
`huft_build`, `inflate_codes`, `inflate_stored`, `inflate_fixed`,
`inflate_dynamic`, `inflate_block`, `flush_window`, `updcrc` - carry the
function names of gzip 1.2's `inflate.c`. It runs on a thread (`UngzipTh`)
fed by a reader thread (`FileReadTh`) through `ccRingBufferTh` ring buffers.

## Every payload is CCSF

All 1,023 inflated payloads begin `01 00 cc cc`. They are CyberConnect2's
chunked scene format - the engine calls it `ccStream`, with one class per chunk
kind (`ccTexChunk`, `ccClutChunk`, `ccModelChunk`, `ccClumpChunk`,
`ccAnmChunk`, `ccMaterialChunk`, `ccCamChunk`, `ccLightChunk`, `ccHitChunk`,
`ccBboxChunk`, `ccEffChunk`). A chunk is `u32 type`, `u32 size in 32-bit
words`, payload.

What holds across all 1,023 files, each checked:

- The first chunk is type `0xcccc0001`, size 13: `"CCSF"`, a 32-byte name, and
  four u32s. The name equals the gzip FNAME without `.cmp` in all 1,023.
- The second is `0xcccc0002`, the name table: `u32 file_count`,
  `u32 object_count`, then `file_count` 32-byte source-file names and
  `object_count` 32-byte object entries (30-byte name, `u16` file index). Its
  size is exactly `2 + 8 * (file_count + object_count)` words in all 1,023.
  Entry 0 of both tables is all zeros in all 1,023. All 316,528 object entries
  point at a valid file.
- The third is type `0x00000003` - note, not `0xcccc` prefixed - in all 1,023.

The source-file names are the artist's paths with the root cut off:
`x\window\tex\xasc00.bmp`, `c\w\dh\sw\02\max\cwdhsw02_6.max`. Each is stored
with a leading `' '` (10,581 names) or `'#'` (184); what the `#` marks is not
known. The `.max` files are 3ds Max scenes, so the art pipeline was Max plus
a CyberConnect2 exporter.

Object names carry a type prefix, and the counts across the archive say what
the scenes are made of: `OBJ_` 244,315, `MDL_` 33,661, `MAT_` 11,071, `CMP_`
7,254, `CLT_` 5,538, `TEX_` 4,306, `ANM_` 4,091, `DMY_` 2,529, `HIT_` 2,388,
`LGT_` 421, `BLT_` 290, `BOX_` 237, `MPH_` 233, `EFF_` 171, `CAM_` 20, `FBR_` 2,
`PAG_` 1.

The header's first u32 after the name is `0x100` in 999 files and `0x90`,
`0x92`, `0x95` or `0x96` in the other 24 - probably an exporter version. The
second varies (0 for `xasc00`, 242 for the `x74*cam` camera files) and looks
like an animation frame count; not checked.

## Where a naive walk breaks

Walking chunks by the size field works through the header and name table and
through CLUT chunks (`0xcccc0400`), but fails on texture chunks
(`0xcccc0300`) and on `0xcccc0800`. In `xasc00` the texture chunk declares
0x2039 words, but its 9-word header plus 0x2000 words of 4-bit pixels
(256x256) leaves 48 declared words that are not in the file; in 552 files the
walk overruns the end by exactly 176 bytes; in others the pixel data runs on
past the declared end. The size field of a texture chunk evidently counts
something the file does not store - plausibly GS packet headers the loader
builds in place - and the loader (`SetBltData__10ccTexChunkFP14ccTexChunkDesc`)
is where the real rule is. That waits on the DWARF reader for
`ccTexChunkDesc`.

## The index is not in the archive

The game asks for files by upper-case `.CCS` name: `cmnFileList`
(`INF SLUS_202.67:0x00306de0`) is `{s32 kind, char *name}` pairs,
`XASC00.CCS` through `XWINDOW.CCS`, ending with kind -1, and `gcmnFileList`
(`0x00306e20`) is the same shape with 31 entries. Nothing next to these lists
says where in `DATA.BIN` a name lives. A search of the executable, all four
overlays, `ICON.BIN` and `OUTSIDE.BIN` for the members' sector numbers - as
u16 or u32, relative to the archive or absolute (the archive starts at LBA
12463), at every stride from 4 to 64 - found two hits, both false: one in code
and one in Shift-JIS text. So either the table is computed, or it is somewhere
not yet looked at. `searchFname__FP8FILELIST` (`0x001651d0`) and
`ccFileListLoad__FPv` (`0x00164540`) will say, once there is a disassembler.

**Still unknown:** how a `.CCS` name is resolved to an offset in `DATA.BIN`;
the texture chunk's real size rule; the meaning of the `#` file-name prefix;
the header's frame-count guess; the `kind` field of the file lists.
