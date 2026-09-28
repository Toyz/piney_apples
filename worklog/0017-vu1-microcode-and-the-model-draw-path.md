---
number: 17
title: VU1 microcode and the model draw path
date: 2026-09-22
area: render, decomp, tooling
files: tools/vu.py, tools/vif.py, tools/test_vu.py, docs/engine/render.md, docs/formats/ccs-model.md
---

# 17. VU1 microcode and the model draw path

[[10]] decoded the model chunk from the EE side and left four things
inferred: that rigid positions are converted with ITOF12, that 256 ST units
are one texture repeat, how the strip flag reaches the GS, and that nothing
culls. The microcode settles all four, and the lighting with them. The flow
is on [the render page](../docs/engine/render.md).

## Tools

`tools/vu.py` decodes VU micro-mode instruction pairs - the full upper and
lower sets, including the flag bits and `LOI` immediates - and
`tools/vif.py` decodes VIF code streams, DMA tags and GIFtags. `vu.py labels`
maps every micro label, `vu.py dis` disassembles a program, `vu.py dump`
writes all of them to `work/infection/vu/`, and `vu.py packet` runs the
game's own packet builders (`ccModelDmaTag_Set*`) in `tools/eemu.py` and
decodes the chain they produce. `tools/test_vu.py`: 16 tests, about 130
instruction vectors, 17 of them literal words from the game.

## Where the microcode is

All in main; no overlay has any.

- **VU1**: `ccInitDL_dsmS` .. `ccInitDL_dsmE`, `INF SLUS_202.67:0x001dac80`
  - `0x001de790`, a bare VIF stream of `MPG` blocks filling micro memory
  0x000-0x758 (1,881 instruction pairs of 2,048). `ccSystem::RefrashDMA`
  (`0x0010aa00`) sends it as a 945-qword REF in a double-buffered chain -
  every frame, by the look of the double buffering (inferred).
- **VU0**: `mcVu0_Top`, `0x001de790` - `0x001dea80`, a DMA chain with one
  `MPG` of 90 pairs, sent once by `InitCCSys` (`0x00109eb0`) on channel 0.
  It is the bounding-box test.

The assembler left a local `_$<label>` symbol at the EE address of every one
of the 114 micro labels. EE code reaches them through the value-0 `mc*`
symbols of [[2]] - HI16/LO16 pairs whose linked value is the byte address in
micro memory, shifted right by 3 and OR'ed into an `MSCAL` - and the ten
type-123 relocations of [[4]] are `iaddiu vi14, vi0, imm` loading clip
routine addresses for `bal`/`jr vi14`. All 22 EE reference sites and all 10
relocations agree with the label map.

Checks on the decoding: 1,881 + 90 pairs, 0 invalid encodings; all 110
VU1 branches (and 4 of 4 in VU0) land on named labels inside the blob; there
are 25 end bits, 17 right after an `XGKICK`; walking every path from all 26
entry points ends at an end bit or a `jr` hand-off. Three programs -
`mc_DrawTri`, `mc_DrawTriC`, `mc_DrawTriS` - are referenced by nothing in this
build, and `mc_Cos` and `mc_Sin` are empty labels.

## The answers

**Positions** are `s16 * vertexScale / 4096`, settled: the model packet
unpacks them as V3-16 signed, `mc_DrawTriFast` converts with
`itof12.xyz vf20, vf25` (micro 0x063), and `ccSetMatrixPacket` folds
`diag(vertexScale)` into the matrix (`0x0013e4a0` - `0x0013e4c8`). Every
rigid program does the same. The bone and skin path instead converts on the
EE ([[10]]) and sends floats; skin weights reach the VU as
`(w & 0x1ff) << 4` and come out of `itof12` as `weight / 256`.

**ST**: 256 units are one repeat, settled. ST is unpacked V2-16 unsigned
with the STROW offset added, `itof12`'d, multiplied by Q and stored as STQ
with the unpacked z - the row's z, 1/16 - so the GS computes
`u = (S + row.x) / 256`. `row.x`/`row.y` are the material's scroll values,
`ccMaterial.u`/`v` taken `>> 4 & 0xff` so they wrap at one repeat. That the
V2 unpack delivers exactly row.z in z (the manual calls it indeterminate) is
the one inference; if z were row.z plus S instead, it would be off by under
0.05% for S below 4096.

**The strip flag**: the normal word's fourth byte reaches the VU (through a
masked `UNPACK` for unlit models, as part of the normal for lit ones) and is
turned into the ADC bit: `ilw.w vi01, ...; iaddiu vi01, vi01, 0x7fff;
isw.w vi01, 3(vi04)` - flag 1 gives 0x8000, "queue the vertex, draw
nothing"; flag 0 gives 0x7fff, "draw". The first batch of each mmat resets
the GS vertex queue through the GIFtag's PRE bit; later batches do not, so
strips carry across the 48-vertex batches, as [[10]] found from the data.

**Culling**: there is no back-face culling anywhere - no `OPMULA`/`OPMSUB`
in VU1, nothing in the GS setup. There are three layers instead:

1. VU0's `mc0_CheckBoundingBox` transforms the model's eight box corners;
   all outside one plane skips the object, all inside the clip box picks the
   fast program (`mc_DrawTriFast`, `mc_DrawTriL`), anything else the careful
   one (`mc_DrawTriSFast`, `mc_DrawTriLC`).
2. The careful programs and every bone and skin program reject a triangle
   unless all three vertices are inside the clip box, by setting the ADC bit.
   The box is set in `ccLayer::Init` (`0x001082a8`) to the whole GS primitive
   space, 0-4095 in x and y, and a w range - a guard band; the GS scissor
   does the screen edges.
3. Only `mc_DrawTriSFast` (unlit, partly visible) clips for real:
   Sutherland-Hodgman in homogeneous space, six passes through
   `mc_ScissorTriPoly` against the screen window and near/far planes,
   interpolating position, colour and ST, emitting a triangle fan, then
   re-sending the last two strip vertices so the strip survives. Lit models
   and bone/skin models only reject.

**Lighting** (`mc_DrawTriL`, micro 0x154 - 0x17c): three directional lights
plus ambient, in model space. `ccSetMatrixPacket` moves the light directions
into model space and `mc_SetMatrix` renormalises them, which cancels object
scale; light colours are doubled (`loi 2.0`). With normals of length 64:

```
colour = min(128, 128 * (sum_i c_i * max(0, l_i . n) + ambient))
alpha  = 128 * t
fog    = clamp(fogB + fogA * w, fMin, fMax)
```

Unlit models use the vertex colour (0x80 = 1.0). The bone and skin path
renormalises the blended normal to 64 and clamps at 255 rather than 128.

**Still unknown:** the exact z the V2 unpack delivers; the DMA order of the
matrix, material and model packets (matrix, material, model is inferred from
what the prologues read); the full bone and skin packet layout and which of
the `b`/`m` program families is skin and which is bone (inferred: `b` skin,
`m` bone); what `view + 0x110` holds; the `mc04b`/`mc04m` variants; the
shadow and effect programs beyond their entry points.
