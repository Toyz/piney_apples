---
number: 340
title: The frame-mode deflicker: read circuit 1 a line lower, merged at ALP 127
date: 2026-10-01
area: render, engine
files: crates/piney-gs/src/lib.rs, crates/piney-draw/src/lib.rs, crates/piney-mpeg/src/movie.rs, crates/piney-game/src/main.rs, docs/engine/overview.md, README.md
---

# 340. The frame-mode deflicker: read circuit 1 a line lower, merged at ALP 127

While working out `SetDisplayOffset`'s units ([[339]]), a second pair of
DISPLAY values turned up at ccSystem +0xe10 / +0xe20. It is the game's
deflicker. In frame mode, the mode the game runs in outside the movies, the
GS shows the frame buffer through both read circuits, one a line lower,
merged half and half.

## The game

`SetScreenModeMain` (INF SLUS_202.67:0x0010ad40), when the field flag
(+0xbd6) is 0, after `sceGsSetDefDBuff`:

```
0x0010b4e8  both buffers' PMODE (+0xbe0, +0xc08): bit 0 (EN1) set
0x0010b51c  their ALP byte (+0xbe1, +0xc09) = 127
0x0010b524  +0xe10 = buffer 0's DISPLAY (+0xbf8), +0xe20 = buffer 1's (+0xc20)
0x0010b534  each copy's DY + 1
```

libgraph's PMODE is 0x66 (`sceGsSetDefDispEnv` 0x0010dea0): EN2, CRTMD 1,
MMOD 1 (the merge's alpha is ALP), AMOD 1, SLBG 0 (circuit 2 is the
background). Each frame `SwapDoubleBuffer` (0x0010ac40) writes, in frame
mode, the shown buffer's DISPFB (+0xbf0 + 40 n) to DISPFB1 (0x12000070) and
its copy (+0xe10 + 16 n) to DISPLAY1 (0x12000080). `sceGsSwapDBuff` then puts
the buffer's own environment, with DISPFB2 and DISPLAY2. `SetDisplayOffset`
keeps the copies a line lower ([[339]]). So both circuits read the same
buffer, circuit 1 a line lower, and a screen line comes out as
`ALP / 255 x the buffer's line above + (1 - ALP / 255) x its own`, about
half each. On an interlaced TV that softens one-line detail that would
flicker between the fields. Field mode (the movies' `SetScreenMode(640,
448, 2)`) skips all of it and sets `sceGsSetHalfOffset` instead.

PCSX2's "Anti-Blur" option, on by default, is meant to undo blurs of this
kind, so PCSX2 players see the sharp picture. That is from the option's
description; it was not tried here.

## The port

`Frame::field_mode` (piney-draw) says how a frame is shown. It is false for
the game's frames and true for `piney_mpeg::movie::draw`. `Gs::render`
remembers it. `piney_gs::Pcrtc` carries the display offset and
`deflicker` to `Presenter::present`. The shader's `merged()` mixes the
moved picture with itself one row (1/448) up, at 127/255, before the sRGB
conversion: the GS merges the stored values. Above the top row the line
above is black, as circuit 1 there has nothing to show.

The port's picture is progressive, so there is no flicker to fight, and
the present is kept sharp (its sharp bilinear). So the merge is off by
default, like PCSX2's anti-blur. `--deflicker` and
the console's `deflicker on|off` switch it (README "Play").

`present_merges_the_line_above_in_frame_mode` draws a one-row white line at
row 200. Sharp, rows 199-201 read 0, 255, 0. With the deflicker they read
0, 128, 127, 0. A field-mode frame keeps 255, 0.

**Still unknown:** the merge circuit's arithmetic on the console (ALP /
255 as PCSX2 takes it, or a shift); whether the user wants the deflicker
on by default.
