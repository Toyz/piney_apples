---
number: 23
title: Text rendering reproduced, and what Quarantine's KFED.BIN is
date: 2026-09-22
area: ui, render, volumes, test
files: tools/font.py, tools/test_font.py, docs/engine/font.md, docs/engine/text.md, docs/disc/volumes.md
---

# 23. Text rendering reproduced, and what Quarantine's KFED.BIN is

[[9]] and [[18]] decoded the desktop text and its escapes but left open what
`ccGetExtendedCode` maps and how the `%` codes are drawn. This entry follows
a string all the way to glyph quads, reproduces both drawing paths in
`tools/font.py`, and checks them against the game's code in `tools/eemu.py`.
The reference is [text rendering](../docs/engine/font.md).

## Two paths

**`ccFont`** draws fixed-cell ASCII from `xasc00::TEX_xasc00`
(`ccSprite::MakePacketStr`, `INF SLUS_202.67:0x0015aed0`). There are four
grids by type. Type 3 is big digits and seven letters, so the game's "MISS"
and "EXP" strings are stored as `+,--` and `./0`. A quirk: in string mode
`\n` sets a newline flag and is then skipped as a control byte, so it only
advances one cell.

**`ccKanji`** (`Disp`, 0x0015e380) is everything proportional: mail, the
board, dialogue, menus.
- It expands `#0`/`#1` into the player's names and maps Shift-JIS through
  `ccGetExtendedCode` (0x0015d390). That is a ten-case switch: Δ Λ Σ Ω Θ ○ △
  □ × ‘ become `%0`-`%8` and `%D`.
- An unmapped Shift-JIS pair becomes `_`, but the scan advances only one
  byte, so the trail byte is read again. That is why the Japanese parody
  text renders as `_r______` rather than as one `_` per character.
- `Extract` (0x0015d490) rasterises into the sprite's own 4-bit texture from
  one of two bitmap fonts compiled into the executable: `ef8x16`
  (0x002f5600) and `ef12x20` (0x002f2180). Proportional spacing comes from
  the trim tables `englishFontOfsS`/`L`.
- One sprite, plus a shadow sprite, is drawn per run of one colour. So a
  16-packet budget holds 8 runs.

The game never wraps text by width. Bodies are NUL-separated lines, and
`ccKanjiStrWidth` is used only to centre. It measures kt 2 proportionally
with the L table, though, and measures ‘ as 0 while `Disp` draws it wide.

`%H` onward reads past the 112 glyphs of the bitmap. `%X`-`%Z` also read
their trim from past the end of `englishFontOfsS`, into `cameraList`, and
write outside the texture. The game has no guard; `font.py` has one.

## Checked

`font.py check` runs the game's own functions in eemu:
- `ccGetExtendedCode` on all 65,536 codes;
- `ccKanjiStrWidth`, `Strlen`, `Strcat` and `StrSeparate` over every event
  message, speaker, mail, reply, board post and news headline, in English and
  parody (19,135 table strings), again with names containing escapes and
  Shift-JIS;
- `ccFont::SetType` and `MakePacketStr` - 2,220 quads compared word for word;
- `ccKanji::Disp` with `Extract` - 3,152 calls and 9,272 quads, with the
  texture compared byte for byte.

The agent that wrote it ran the full check: **0 mismatches**. A mutation test
(a flipped texture byte, a trim off by one, a wrong `#Y` colour) is caught.

On integration I re-ran a sampled check (300 strings, 60 Disp strings plus
the synthetic set): 0 mismatches. I also looked at the renders of mail 0 (the
blue Δ address line) and the `ef12x20` sheet, and both are right.

The harness hooks four functions in Python:
- the VU0 macro helpers `ApplyLayerScreenMatrix` and `sceVu0FTOI0Vector`,
  with an integer view so their arithmetic stays exact;
- `__fixunssfdi`, because its `dptoul` uses `dsrlv`, which eemu does not
  interpret (nor `dsllv` or `dsrav`);
- `SendPacket`, to capture the packets.

## The kanji fonts, and KFED.BIN

Infection carries four 4-bit kanji sheets between its code and data -
`kf14x16`, `kf20x20`, `kfa20x20`, `kfa14x16` - that nothing uses. Across main
and all four overlays, no relocation references them, and neither does any
unrelocated `lui` (`disasm.py xrefs --scan`, 20 combinations). They are
leftovers of the Japanese build.

`kfa20x20` holds Δ Λ Σ Ω Θ, then kaomoji. So the `%B`, `%D`, `%E`, `%G`,
`%I`... that end the parody text's lines are probably face marks in the
Japanese build (inference). The US font draws them as halves of ⊘, ‘, ®, or
blanks.

That settles [[20]]'s question about Quarantine:
- Mutation and Outbreak embed all six font blobs byte-identical to
  Infection (whole-blob compare).
- Quarantine keeps only the two `ef` fonts the code uses. It names
  `\DATA\KFED.BIN` and `\DATA\KFAED.BIN`, which have the same sizes and the
  same sheet layout as `kf20x20` and `kfa20x20`: 160-byte rows, 16 glyphs of
  20x20 a row, 3,440 glyphs in Shift-JIS order from 0x8140.

[[20]] concluded from the bytes (0.2% equal) that the encoding or order had
changed. That was wrong. Its test looked for identical 200-byte glyph blocks,
but a glyph in a sheet is 20 separate row slices, not a contiguous block.
What changed is the shading:
- Infection's background is index 1 and Quarantine's is 0.
- The other values are not a permutation of each other.
- 523k of about 630-660k inked nibbles fall in the same places.

So the glyphs are the same set in the same order, re-rasterised or re-shaded
(inference). [[20]] now says so.

**Still unknown:** what Quarantine loads KFED for (Infection's code never
touches its kanji sheets); how the GS samples the kt 0 and kt 2 sprites,
whose `sx` and `su` differ by a texel; which view and scale the font layer
uses; the line spacing of mail and board bodies; where the `ccKanji` texture
sits in VRAM; whether the text code changed in volumes 2-4.
