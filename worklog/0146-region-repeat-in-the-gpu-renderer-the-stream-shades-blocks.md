---
number: 146
title: REGION_REPEAT in the GPU renderer: the stream shades' blocks
date: 2026-09-26
area: render
files: crates/piney-gs/src/convert.rs, crates/piney-gs/src/lib.rs, crates/piney-draw/src/lib.rs, docs/engine/stream.md
---

# 146. REGION_REPEAT in the GPU renderer: the stream shades' blocks

A GAPS drawing item. Stream 15's shades (`ccBufferSampling::SetShade(t,
z, colour)`) read the picture being drawn with CLAMP_1 set to
REGION_REPEAT. The mask is `1024 - t` and the fix `t / 2`, so every `t` x
`t` block reads one texel and the far scene turns blocky behind the
figures. The software rasterizer read `Wrap::Region` that way. `piney-gs`,
the renderer the game runs on, sampled it as a plain repeat, so there the
shades were smooth.

## The change

- **The vertex.** A 2D primitive whose texture wraps by region gets a new
  `Params::REGION` bit and the mask and fix in `GVertex::region`, as a new
  flat vertex attribute.
- **The shader.** With the bit set, it takes the texel under the pixel,
  masks and fixes both coordinates as the GS does, and fetches that texel
  (`textureLoad`, unfiltered) in place of the sample.
- **The check.** `region_repeat_reads_in_blocks` draws eight one-texel
  columns of rising red, copies them, and draws the copy with mask
  `1024 - 4` and fix 2. The eight pixels read columns 2 and 6, four each,
  as the GS reads them.

**Still unknown:** the shades are not compared with the game's pictures;
only the texel choice is checked.
