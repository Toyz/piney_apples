---
number: 8
title: Textures and palettes convert to PNG, stored bottom-up
date: 2026-09-22
area: format, render, tooling
files: tools/ccstex.py, tools/png.py, docs/formats/ccs.md
---

# 8. Textures and palettes convert to PNG, stored bottom-up

With the texture layout from [[7]] and the CLUT layout read out of
`ccStream::Decode_Clut` (`INF SLUS_202.67:0x0014d660`), every texture the
chunk walk reaches converts to a correct-looking image. `tools/ccstex.py`
does the conversion and `tools/png.py` is a minimal PNG writer (8-bit RGBA,
one IDAT, standard library `zlib`).

## The CLUT chunk

```
u32       clut             object index of the CLT_ entry
u32       blt_group        only if version >= 0x92
u8        flag
u8        unknown_1        into the descriptor at +2
u8, u8                     read and discarded
u16       buf_x            VRAM placement; 0/0 or x < 768 is replaced by
u16       buf_y            896/992 and the flag gets bit 0
u32       count            16 or 256
u32[count] colour          R, G, B, A with A = 0x80 for opaque
```

What [[3]] called `unknown_0c = 0x03000300` in `xasc00` is `buf_x` 768,
`buf_y` 768. For a 256-colour palette the decoder stores colour `i` at slot
`clut256Tbl[i]` (`0x002fb190`) - the GS's CSM1 arrangement - so the file holds
colours in logical order and a converter uses them as they are. 8-bit
textures confirm it: palettes applied in file order give clean images, where
the CSM1 order would scramble every 8-colour block.

## Survey

Over every texture chunk the walk reaches in `DATA.BIN` (3,985 of the 4,306
`TEX_` objects; the rest sit after a Model chunk the walk cannot yet pass):

| | count |
| --- | ---: |
| `psm` 0x13 (PSMT8, 256 colours) | 3,741 |
| `psm` 0x14 (PSMT4, 16 colours) | 244 |
| `mipmap` 0 / 2 / 3 extra levels | 2,014 / 350 / 1,621 |
| palette in the same file as its texture | 3,985 |
| level 0 exactly `w * h * bpp` bits | 3,985 |
| CLUT chunks with 256 / 16 colours | 4,755 / 346 |

Sizes run from 32x32 to 512x512; 128x128 (1,010) and 64x64 (870) are the most
common. `flag` is 0x01 in 3,494 textures, 0x05 in 239, 0x09 in 167, 0x00 in
54, with a handful of 0x10, 0x11, 0x08, 0x18, 0x04.

## Rows are bottom-up

The first conversion of `TEX_xgttit00` (from `title1`) showed the
`.hack//INFECTION` logo upside down, and the font sheet `TEX_xasc00` had its
glyphs upside down with the used rows at the bottom. Rows are stored bottom row
first. The source paths in the name table say why: the art was authored as
`.bmp` (`x\window\tex\xasc00.bmp`), BMP stores rows bottom-up, and the
exporter copied them in that order. Presumably the game's texture coordinates
account for it; that is not checked. `ccstex.py` flips by default and
`--raw-rows` keeps the stored order.

At 4 bits per pixel the low nibble is the left-hand pixel. Alpha is GS alpha,
0 to 0x80, and is doubled for PNG. Many interface textures are pure white with
the shape entirely in alpha - the title logo is one - so `--matte` composites
over dark grey to make them visible.

## Checked by eye

- `title1::TEX_xgttit00`, 512x256 PSMT4: the `.hack` logo with `INFECTION`
  and `感染拡大`, right way up after the flip.
- `xasc00::TEX_xasc00`, 256x256 PSMT4: four ASCII fonts and a set of large
  digits, crisp.
- `town01::TEX_sr1sky1`, 512x256 PSMT8: a sunset sky over hills; and
  `TEX_sr1cas1`, a castle-wall atlas - 256-colour palettes in file order.
- `xddesk01::TEX_xddback1`: a window frame titled `VISUAL MAILER ver 6.0`,
  the in-game mail client.

`tools/ccstex.py dump work/infection/disc/DATA/DATA.BIN work/infection/png`
writes all 3,985 in about 22 seconds, one directory per scene file.

**Still unknown:** the texture and CLUT `flag` bits; the CLUT's `unknown_1`;
how the game's UVs deal with bottom-up rows; the textures behind Model
chunks, until the walk can pass them.
