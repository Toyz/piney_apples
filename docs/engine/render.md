---
title: The model draw path and VU1 microcode
status: partial
volumes: INF
covers: INF SLUS_202.67:0x001dac80-0x001de790 VU1 microcode, 0x001de790-0x001dea80 VU0 microcode, 0x0013eab0 ccModel::Draw, 0x0013e350 ccSetMatrixPacket, 0x0013e6c0 ccSetMaterialPacket, 0x0013e130 ccSetModelPacket, 0x0013e9e0 _ccCheckBoundingBoxEx, 0x00108260 ccLayer::Init
worklog: 10, 17, 244
---

# The model draw path and VU1 microcode

How a [model chunk](../formats/ccs-model.md) becomes GS primitives: the EE
builds VIF packets, VU1 transforms, lights and packs them, and `XGKICK`s
GIF packets to the GS. `tools/vu.py` and `tools/vif.py` decode both sides.

## Microcode

| unit | EE range | contents | upload |
| --- | --- | --- | --- |
| VU1 | 0x001dac80 - 0x001de790 | `MPG` x8, micro 0x000 - 0x758, 1,881 pairs | `ccSystem::RefrashDMA`, REF of 945 qwords |
| VU0 | 0x001de790 - 0x001dea80 | one `MPG`, 90 pairs | `InitCCSys`, once, DMA channel 0 |

Every micro label has a local `_$<label>` symbol at its EE address. EE code
references a label through an undefined `mc*` symbol whose linked value is
the byte address in micro memory; `>> 3` gives the micro address for `MSCAL`.

### Entry points

| micro | label | caller |
| --- | --- | --- |
| VU1 0x000 | `mc_SetMatrix` | `ccSetMatrixPacket` |
| 0x022 | `mc_SetObjParam` | `ccSetMaterialPacket` |
| 0x026 / 0x02e | `mc01_Start00` / `01` - unlit batch prologue, first / later batch | `ccModelDmaTag_SetTag` |
| 0x063 | `mc_DrawTriFast` - unlit, wholly inside | `ccModel::Draw` |
| 0x0fe | `mc_DrawTriSFast` - unlit, partly inside, clips | `ccModel::Draw` |
| 0x141 / 0x148 | `mc02_Start00` / `01` - lit prologue | `ccModelDmaTag_SetTag` |
| 0x154 | `mc_DrawTriL` - lit, wholly inside | `ccModel::Draw` |
| 0x181 | `mc_DrawTriLC` - lit, partly inside, rejects | `ccModel::Draw` |
| 0x40f | `mc03_SetParam` | `ccModel::DrawBoneType` |
| 0x419 / 0x433 | `mc03b_DrawModel0` / `1` | `Decode_Mmat02` |
| 0x4de / 0x4fa | `mc03m_DrawModel0` / `1` | `Decode_Mmat02` |
| 0x290 / 0x298 | `mc04b_DrawModel0` / `1` | branch from `mc03b` when vf16.y != 0 |
| 0x595 | `mc_DrawEff` | `ccEff::Draw` |
| 0x5d2 | `mc_DrawShadow1` | `ccShadowModel::Draw` |
| 0x5ef / 0x641 | `mc_DrawShadow2` / `3` | `DecodeShadowModel` |
| VU0 0x000 | `mc0_CheckBoundingBoxShadow` | `ccShadowModel::Draw` |
| VU0 0x01f | `mc0_CheckBoundingBox` | `ccModel::Draw`, `ccObj::CheckBoundingBox` |

Unreferenced in this build: `mc_DrawTri` (0x03a), `mc_DrawTriC` (0x085),
`mc_DrawTriS` (0x0b2), and the empty labels `mc_Cos`, `mc_Sin`.

## Packets for a rigid model

`ccModel::Draw` (`0x0013eab0`) emits, per model:

```
matrix packet     ccSetMatrixPacket
  FLUSH; DIRECT 2                        A+D FOGCOL
  STMOD normal; STCYCL 4,4
  UNPACK V4-32 num=17 addr=8             VU mem 8..24 (below)
  MSCAL mc_SetMatrix; BASE 0; OFFSET 0x200

material packet   ccSetMaterialPacket
  FLUSH; DIRECT 9                        TEXFLUSH CLAMP_1 TEX0_1 MIPTBP1_1
                                         TEX1_1 ALPHA_1 TEST_1 ZBUF_1
  UNPACK V4-32 num=2 addr=8; MSCAL mc_SetObjParam

model packet      ccSetModelPacket, then CALL into the cached per-mmat list
  FLUSH; STMOD normal; STCYCL 4,1
  STROW [(u >> 4) & 0xff, (v >> 4) & 0xff, 1/16, 1.0]
  BASE 0x0ec; OFFSET 0x18a               two buffers, 0x18a qwords each
  per batch of up to 48 vertices:
    FLUSHE; UNPACK V4-32 flg addr=0 num=1       GIFtag: n vertices, PACKED,
                                                FOG ST RGBAQ XYZ2
    UNPACK V3-16 flg addr=9 num=n               positions -> slot 0
    unlit: STMASK 0x3f3f3f3f; UNPACK V4-8 mask flg addr=9    strip flag -> slot0.w
           UNPACK V4-8 usn flg addr=10                        colours -> slot 1
    lit:   UNPACK V4-8 flg addr=10                            normal + flag -> slot 1
    STMOD offset; UNPACK V2-16 usn flg addr=11                ST + row -> slot 2
    ITOP <other buffer>; MSCAL mc01_Start00|01 (lit: mc02_*)
  STCYCL 4,4
```

The order matrix, material, model is inferred from what the prologues read.

VU memory filled by the matrix packet:

| qwords | contents | register |
| --- | --- | --- |
| 8 - 11 | `world_screen * local_world * diag(vertexScale)` | vf01 - vf04 |
| 12 - 15 | light colours | vf09 - vf12 |
| 16 - 19 | light directions in model space | vf05 - vf08 |
| 20 / 21 | `view.clipMin` / `clipMax` | vf13 / vf14 |
| 22 | `fMin, fMax, fogB, fogA` | vf15 |
| 23 / 24 | clip planes from the screen window, near, far | vf30 / vf31 |

## What VU1 does per vertex

```
position   itof12(s16 xyz) through vf01..vf04; divide by w; ftoi4
ST         itof12(S + row.x, T + row.y) * Q, with z ~ 1/16 -> u ~ (S + row.x) / 256
ADC        strip flag + 0x7fff: flag 1 -> 0x8000 (no triangle), 0 -> 0x7fff
lit        c = min(128, 128 * (sum c_i * max(0, l_i . n/64) + ambient)), 3 lights
unlit      c = vertex colour, 0x80 = 1.0
alpha      128 * t
fog        clamp(fogB + fogA * w, fMin, fMax)
```

The V2-16 unpack puts S and T in z and w as well (as PCSX2's
console-tested unpacker does), so z is `S + bits(1/16)` and u is off by a
factor `1 + S * 2^-23`; see [the model chunk](../formats/ccs-model.md#values).

`ftoi4` converts the whole divided vector, so a vertex's GS Z is 16 z / w,
on the scale of the Z the EE writes for its own sprites and strips (the
effects' `rot_trans_pers`, `ccBufferSampling`'s depth strips). The port
takes a `ModelDraw`'s depth the same way (`piney_draw::MODEL_Z_SCALE`); it
used to take z / w, a sixteenth of the effects' and the strips' Z, so those
drew in front of any model.

The vertex alpha reaches the GS as an integer (`trunc(128 t)`, or
`trunc(A_vertex t)` unlit), and the GS interpolates colours as integers,
so a triangle whose vertices share an alpha has exactly that alpha at
every pixel. The alpha test then passes or fails it as a whole. piney-gs
interpolates the alpha as a float, which can fall a hair under the
integer, so its shader snaps it back (`floor(a + 1/32)`) before the
modulate and the test. A model fading in shows why: its alpha and its
AREF (`trunc(aref t)`) are both 2 at `t` = 0.02, and every pixel must
pass and write Z. Stream 5's drain ring does this, and it erases the
figure drawn after it; unsnapped, the erasure came out dithered.

The port carries the fog as `piney_draw::ModelDraw::depth_fog` (a
`DepthFog`: fogA, fogB, fMin, fMax and FOGCOL as `ccDrawEnv::SetFog`
leaves them). Both renderers compute F at each vertex's w and interpolate
it across the triangle in screen space, as the GS does; a constant `fog`
(`ccChar::Draw`'s blend) wins over it. piney-world fogs a model by the
draw environment's fog (`TownLights::fog`) unless the game's `SetFogSw(0)`
turns it off (the skies, clouds and water).

Light colours are doubled in `mc_SetMatrix` (`loi 2.0`) and light directions
renormalised there. In the `lit` line `c_i` are the colours as the matrix
packet holds them (VU memory 12-14), before that doubling: VU1 computes
`sum 2 c_i max(0, l_i . n) + 128 ambient` on the stored normal `n`, whose
length is 64, so the doubling and the 64 make the 128 (checked against
`mc_DrawTriL` run by `tools/vu.py`'s `Vu`, `tools/test_demo_rs.py`). The bone and skin programs renormalise the blended normal
to 64 and clamp colour at 255.

## Culling and clipping

No back-face culling. Three layers:

1. **Object**: VU0 `mc0_CheckBoundingBox` - 0 skip (all corners outside one
   plane), 1 wholly inside (fast program), 2 partly inside (careful program).
2. **Triangle reject** in `mc_DrawTriLC`, `mc_DrawTriSFast` and every bone
   and skin program: a triangle is drawn only when all three vertices are
   inside `clipMin`/`clipMax` in x, y and w. `ccLayer::Init` sets those to
   (0, 0, -, 8) and (4095, 4095, -, 2^20): the whole GS primitive space.
3. **Clipping** only in `mc_DrawTriSFast` (0x0fe, unlit rigid models partly
   inside): a triangle that is not wholly inside, not at a strip start, and
   whose last vertex has w below `view.divZ` is marked; after the batch is
   kicked, `mc_ScissorTriPolyXYZ` (0x1b4) copies each marked triangle in
   its homogeneous (undivided) form and cuts it with `mc_ScissorTriPoly`
   (Sutherland-Hodgman) against six planes - z, y, x at `vf30` and at
   `vf31` - then divides and sends the polygon as a fan; the strip's last
   two vertices are re-sent. Any other triangle that is not wholly inside is
   dropped. The inside test is strict: `clipMin` < (x / w, y / w, w) <
   `clipMax`, from `suba vf13 - v` and `suba v - vf14` and the MAC sign
   flags (mask 0xd0) of the three vertices ANDed.

   The values: `divZ` is `vf16.x`, which `ccSetMaterialPacket` loads from
   `view + 0x25c` (0x0013e96c); `ccLayer::Init` (0x00108260) sets it to
   1000, `clipMin`/`bboxClipMin` to (0, 0, 0, 8) and `clipMax`/
   `bboxClipMax` to (4095, 4095, 0, 2^20). `ccSetMatrixPacket` builds the
   scissor planes from the bbox clip (0x0013e514-0x0013e5cc): `vf30` =
   (-bbMin.x, -bbMin.y, z at w = bbMax.w over -bbMax.w, -1), `vf31` =
   (bbMax.x, bbMax.y, z at w = bbMin.w over bbMin.w, 1), so the cut is at
   the GS primitive space's edges, the near plane w = 8 and the far one
   w = 2^20. A floor or wall the camera is pressed against therefore still
   draws up to the near plane, while the lit and skinned programs drop
   such triangles.

   `piney-gs` (`convert::mmat`) does the same for a `ModelDraw` with
   `reject_outside` false: wholly inside, drawn; otherwise cut at w = 8 when
   the last strip vertex is nearer than 1000 (the screen edges left to the
   rasteriser), else dropped.

## Unknown

- The DMA order of the three packets.
- The bone and skin packet layout; which of `mc03b`/`mc03m` is skin.
- `view + 0x110`; the `mc04b`/`mc04m` variants; the effect program. The
  shadow programs are in [the shadow volumes](shadow.md).
