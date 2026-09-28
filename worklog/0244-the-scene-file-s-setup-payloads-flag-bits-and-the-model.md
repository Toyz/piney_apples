---
number: 244
title: The scene file's setup payloads, flag bits and the model header
date: 2026-09-27
area: format
files: docs/formats/ccs.md, docs/formats/ccs-model.md, docs/engine/render.md
---

# 244. The scene file's setup payloads, flag bits and the model header

Asked for: more of the docs' open questions. `docs/formats/ccs.md` and
`ccs-model.md` are now solid.

## Setup payloads

Every setup kind's decoder was read. Where the DWARF has a `CCSTRM_*`
structure (Hit, Eff, FBRect, FBPage, DummyPos, DummyPosRot, Pcm, Shadow),
the page uses its field names.
- Camera, Light, Obj2 and Layer were already read by the port (`piney-stream`'s
  `file.rs`), Hit by `piney-world`'s `hit.rs`, Eff on the effects page. The
  format page now lays them all out.
- **FBRect and FBPage.** Only `FRAMEBUF.CCS` has them, and no file list
  names it. Their decoders read the records and keep only the object's kind.
  The file is a VRAM plan: a 1024 x 512 draw area and a 512 x 512 Z buffer,
  named in Shift-JIS (`描画領域`, `Zバッファ`), and six page rectangles.

## Anime sub-kinds

`docs/engine/animation.md` already covered all eleven kinds found in
DATA.BIN's 4,091 Anime chunks. The format page now lists them with their
counts.

## Texture and CLUT flags

- **CLUT `unknown_1`** is `cpsm`, which goes into TEX0 bits 51-54. It is 0
  (PSMCT32) in all 5,538.
- **Texture flag bits** (`ccTexChunk` keeps `& 0x1d`):
  - 0x01: keep the pixels, don't upload at load. The decoder also sets it
    for a defaulted VRAM place.
  - 0x04: set or kept, never read.
  - 0x08: translucent (sorted).
  - 0x10: CLAMP.
  - 0x40 (descriptor only): no pixel buffer.
- **CLUT flag bits** (`ccClutChunk` keeps `& 5`): the same 0x01, and 0x04
  never read. 0x08 is dropped.
- **The decoder's bit 0x04** marks a palette whose alpha is other than 0 or
  0x80, and a CLUT-less texture whose alpha is only 0 or 0x80. The file
  bits don't follow either rule. No texture lacks a palette.

## The model header

- **`flag`:** only `& 3` (blendType) is kept; bit 3 is set on 484 models
  and dropped.
- **`zoffs`:** stored as `ccModel.zoffs` and copied, never read; 0 in all
  11,010 models.
- **`unknown_10`:** dropped; 0x70 in all.
- **The size field.** It overstates each `mtype & 4` mmat by 15 words.
  Nothing reads a chunk's size, and the exporter is not on the discs.
  Moved to Notes.

## The ST scale

The microcode loads the ST slot as xyz, converts x and y only, and sends z
times Q as the GS's Q. So the scale depends on what a V2-16 unpack writes
in z.
- PCSX2's unpacker, tested on the console per its source comment, copies
  x and y into z and w.
- With the row added (offset mode), z is `S + bits(1/16)`.
- So u is `(S + row.x) / (256 (1 + S 2^-23))`: 3e-5 off at one repeat.

The port uses exactly 1/16; the difference is far below a texel.
`render.md` loses the same question.

**Still unknown:** Nothing on the two format pages. The port's ST ignores
the `1 + S * 2^-23` factor.
