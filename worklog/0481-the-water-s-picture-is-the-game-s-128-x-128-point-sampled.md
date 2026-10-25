---
number: 481
title: The water's picture is the game's 128 x 128 point-sampled copy: towns' water 0, the field's haze and lake mirror
date: 2026-10-25
area: render, test
files: crates/piney-draw/src/lib.rs, crates/piney-gs/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-desktop/src/soft.rs, crates/piney-world/src/town.rs, docs/engine/town02.md
resolves: 283.2
started: 2026-10-25T17:06:36-06:00
took: 14m
---

# 481. The water's picture is the game's 128 x 128 point-sampled copy: towns' water 0, the field's haze and lake mirror

[[97]] left the towns' water reading the whole 512 x 448 frame where the
game reads a 128 x 128 copy, and [[480]] gave the field's lake mirror the
same machinery. Every `waterUVModifi` and `waterUVModifi2` draw goes
through one: Mac Anu's and Dun Loireag's water 0 (a town's +0x1d4,
`ccTexChunkDesc` +0x1bc), and in a field the heat haze and the lake's
mirror (`WORLD` +0x53bc, `ccTexChunk::Init` in `WORLD::Init` 0x005a69e8
and `Generate` 0x005a7e38: `tw` and `th` 7, `psm` 0, 128 x 128, 32 bits).

`ccLayer::MakePacketDrawBuffTrans` (main 0x00108e30) queues one context-2
sprite: FRAME_2 the texture's page, SCISSOR_2 its 128 x 128 less one
(+0x3c, +0x3e), TEX0_2 the draw buffer (`iFuncDrawBuffTrans` 0x001089c0
picks the page at send time), TEX1_2 0 (nearest), UV from the layer
view's scissor (+0x220 to +0x226, each `& 0x7ff` times 16, the far edge
plus one) onto the copy. So each texel is one pixel of the frame, picked
without filtering, and the water then reads that small texture with its
own filter. The port read the whole frame at full resolution: a sharp
picture where the game's is coarse, and, through the lake's moving
vertices, a speckle of single pixels.

piney-draw has `TexRef::SampledFrame { width, height }`. piney-gs copies
the whole frame as for `FrameBuffer` and reads it through the copy's grid
(`Params::SAMPLED`, bit 23): each of the four grid texels a bilinear read
mixes is the frame's pixel at that texel's centre; with the draw's filter
nearest (`SAMPLED_NEAREST`, bit 24) the one texel. Prims and models both
take it. The soft renderer's `sampled` makes the small image itself.
`WaterZero::edits` asks for it at 128 x 128, so the towns' water, the haze
and the lake all change.

Where in each texel the sprite picks is the GS's sprite rule: the port
takes the texel's centre, as `ScaledFrame` does. A pixel-corner rule
would move the pick by half a copy texel. That is hardware rasterisation,
not code, and is left in the docs as research.

Tests: `a_sampled_frame_reads_the_pixels_at_its_texels_centres`
(piney-gs: a red column at x 64 is texel 0 of a 4-wide copy, read back
over the first quarter) and `a_sampled_frame_takes_its_texels_centres`
(the soft renderer). Area 14's lake was rendered again: the speckle is
gone.

The other steps of the clock: piney-game suite 334 passed; piney-gs,
piney-desktop, piney-draw and piney-world tests, clippy and fmt clean.

**Still unknown:** whether the GS samples a sprite's texels at each pixel's corner or its centre (half a copy texel; research, hardware)
