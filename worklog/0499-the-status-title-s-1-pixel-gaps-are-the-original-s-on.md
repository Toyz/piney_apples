---
number: 499
title: The STATUS title's 1-pixel gaps are the original's on hardware: the GS samples the blank cell's clear ninth column under the game's REPEAT clamp
date: 2026-10-25
area: render
files: crates/piney-desktop/src/soft.rs, BUGS.md
started: 2026-10-25T23:28:35-06:00
took: 10m
---

# 499. The STATUS title's 1-pixel gaps are the original's on hardware: the GS samples the blank cell's clear ninth column under the game's REPEAT clamp

BUGS.md: the STATUS title bar has 1-pixel gaps. The earlier finding was
that the game's code and data give the same gaps. It was left open,
waiting on a PS2 capture of a titled window. The question was whether
the GS, with the game's own settings, samples the cell's clear ninth
texel column.

**What the game sends.**
- `DispLine(0, n, title)` draws the title from `xwindow` grid 0, whose
  cells are 9 x 16. Each blank cell is code 38: 8 texels of bar and a
  clear ninth column, in all four discs' texture.
- `MakePacketStr` sends each cell as a SPRITE, 9 logical units wide (9
  pixels in the menu's view), with UV from `u0` to `u0 + 9` texels
  (docs/engine/map.md, "The sprites").
- `ccSprite::SetTag` (INF main 0x0015c0c0) sets the registers the
  sprites go out with:
  - `CLAMP_1` 0: REPEAT both ways, so there is no region clamp that
    could hold U inside the first eight columns;
  - `TEX1_1` from the sprite with MXL cleared;
  - `TEST_1`, `ZBUF_1`, `TEX0_1` and `ALPHA_1` from `alphaBlendTbl`.

**What the GS does with it.** A sprite covers the pixels whose integer
point lies in [x0, x1). Each takes the texel under that point, with U
interpolated linearly across the sprite.
- Over 9 pixels and 9 texels, U at the last pixel is `u0 + 8`, or
  `u0 + 8.5` with a half-pixel offset. Either way it is texel column 8,
  the clear one.
- No sub-pixel offset avoids it: column 8 is always sampled once.
So on hardware the title shows the same one-pixel gap after each blank
cell. It is the original's look.

The port's software canvas implements exactly this rule (`Canvas::sprite`:
`ceil` coverage, the UV at the pixel's point, `floor` for nearest), and the
GPU path shows the same gaps.

**Checked.** New piney-desktop test
`a_sprite_samples_its_last_texel_column`: a 9-pixel sprite over a 9-texel
cell whose ninth column differs takes that column at its last pixel, on a
whole-pixel and on a half-pixel offset. piney-game's suite passes (336).

Docs: BUGS.md.

**Still unknown:** nothing
