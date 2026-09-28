---
number: 29
title: Material crop values are a UV animation's reference, not an offset
date: 2026-09-23
area: render, format
files: docs/formats/ccs.md, crates/piney-data/src/scene.rs
---

# 29. Material crop values are a UV animation's reference, not an offset

[[26]] found the Material chunk's `cropU`/`cropV` nonzero in 218 of 11,071
materials, all on animated UVs and eyes, and left their unit untraced, with
the viewer ignoring them. The render page's VU1 formula
`u = (S + row.x) / 256` suggested they might be a texture offset the viewer
was missing. They are not.

- `ccMaterialChunk` (DWARF, 0x18 bytes) keeps `transparency` at +0x10 and
  `cropU`/`cropV` at +0x14/+0x16, as [[26]] read them.
- The runtime `ccMaterial` has its own pair at +0x14/+0x16.
  `ccMaterial::Init` (`INF SLUS_202.67:0x001399f0`) copies the transparency
  but stores zero in that pair: it never reads the chunk's crops.
- The chunk's crops are read by `ccModel::SetUV(u, v, chunk, flags)`
  (0x0013abe0). For every material of the model that uses the chunk, it
  sets the runtime offset to `u - cropU` and `v - cropV`, with flag bits 1
  and 2 skipping u or v. Its only caller is `ccAnm::SetUV` (0x00152de0), so
  this happens only while a UV animation runs.

So the crop is the value a UV animation's keys are measured from. A model
at rest draws with no offset, and the viewer's static frames were already
right to ignore it. The unit of the animated offset belongs with animation
playback, which is being worked out separately.

**Still unknown:** the unit of the runtime offset once a UV animation drives it,
and which part of `ccSetMaterialPacket` (0x0013e6c0) turns it into the ST
row.
