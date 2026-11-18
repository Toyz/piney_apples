---
title: The shadow volumes
status: partial
volumes: INF
covers: INF SLUS_202.67:0x00140920 DecodeShadowModel, 0x001400e0 SetShadowNormal2, 0x001403c0 SetShadowWork2, 0x001412b0 ccShadowModel::Draw, 0x0013f220 ccObj::Draw, 0x00138380 ccCoord::_SetLWMatrix, 0x0013eab0 ccModel::Draw (its check), 0x0014badc Decode_Mmat01 (the box's extremes), 0x0013b5d8 ccObj::Init, 0x00141960 ccShadowPacket::SetShadowGSReg, 0x00141c80 TransFrameBuffer, 0x00142060 FillTexBuffer, 0x00142360 ccShadowPacket::TransShadowTex, 0x00142820 ccShadowPacket::FillShadowBuff, 0x00142b20 ccShadowPacket::SetShadowPacket, 0x00142e70 ccShadowPacket::SetShadowPacketAll, 0x00142ec0 ccShadowPacket::GetPacketList, 0x00142ff0 ccShadowPacket::AddPacket, 0x001430a0 ccShadowPacket::ccShadowPacket(short, ccView *), 0x00143150 ccShadowPacket::ccShadowPacket(ccShadowChunk *, ccView *), 0x001432f0 ccShadowPacket::SetBuffer, 0x0014c3e0 ccStream::Decode_Shadow, 0x0014e830 ccStream::DecodeF_Shadow, 0x00148400 ccStreamDrawLayerList::Draw, 0x001054d0 ccDrawEnv::Reset, 0x0013e9e0 _ccCheckBoundingBoxEx, 0x001388c0 ccBbox_SetBox, 0x00108260 ccLayer::Init (the view's bounds), 0x001a0d60 WORLD_MAN::GO (the packets), 0x001a21a0 WORLD_MAN::SetLightDirection, 0x00139240 ccDistantLight::GetDirc, 0x0013d760 ccClump::SetShadowSw, 0x001529e0 ccAnm::SetShadowSw, 0x002fb130 zureTbl, VU1 0x5d2-0x758 mc_DrawShadow1, mc_DrawShadow2, mc_DrawShadow3, mc_DrawShadowClip, mc_ScissorShadowPolyXY, mc_ScissorShadowPoly, VU0 0x000 mc0_CheckBoundingBoxShadow; INF gcmn.prg:0x005d0cf0 TOBJ::Draw (its packet)
worklog: 160, 613, 614
---

# The shadow volumes

Characters, weapons and a few scenery objects cast stencil shadows: a
second, closed model beside the drawn one (the shadow model, `mtype` bit
3, [format](../formats/ccs-model.md)) is stretched away from the light
into a volume, the volume's faces are counted into a small buffer against
the frame's Z, and the frame is darkened where the count is positive.

Three pieces:

- **`ccShadowModel`**: the shadow model, prepared once when its file loads
  (`DecodeShadowModel`) and drawn with every `ccObj::Draw` of its object
  (`ccShadowModel::Draw` and the VU1 programs) into the draw environment's
  packet.
- **`ccShadowPacket`** (0x160 bytes, 0x180 in a stream): a buffer, a
  darkness and a layer; each frame the volumes sent to it, grouped by the
  shadow's alpha, and at the frame's end (`SetShadowPacketAll`, from
  `ccSystem::Ctrl`) the GS steps that count them and darken the frame.
- **The draw environment**: `ccDrawEnv` +0x8c the packet (none: no
  shadows), +0x90 the light's direction, +0xa0 the shadow's length, +0xa4
  its alpha. `ccDrawEnv::Reset` (0x001054d0): none, (0, 0, -1), 1000.0,
  128.

## Which objects cast

`ccObj::Init` (0x0013b5d8) makes the Obj chunk's shadow model (its fourth
word, [ccs](../formats/ccs.md)) the object's +0x9c and sets +0xa0 bit 1
(has one) and bit 2 (the switch, on). `ccObj::Draw` (0x0013f220) draws the
shadow model whenever it has one and the switch is on, after the model and
whatever the object's transparency. `SetShadowSw(0)` turns it off: a middle
boss's second model (`ccEnemy::initEnemyCCS`), `ccThDrainEnemy` during a
drain, `ccThStrParty`, `effTCDrillMissile`, a tree of Upheaval.

`DATA.BIN` holds 3,129 shadow models, one mmat each (1,957 distinct): every
character body (a PC's 15 to 31 objects each carry one), the dogs, the
weapons, TOBJ.

## Where the environment comes from

| who | packet (+0x8c) | light (+0x90) | length (+0xa0) | alpha (+0xa4) |
| --- | --- | --- | --- | --- |
| `WORLD_MAN::GO` | packet A | `WORLD_MAN` +0x90 | | |
| `ccChar::Draw` | | | 3 x the height | `(fptosi(256 t) + 1) >> 1`, then 128 after |
| `TOBJ::Draw` | packet B for its anms, then A | | 3 x its height | as `ccChar::Draw` |
| `ccStreamDrawLayerList::Draw` | the node's packet | the packet's +0x160 | the packet's +0x170 | `(fptosi(256 a) + 1) >> 1`, `a` the packet's +0x174 |
| `ccBossAfterImageEx::Draw`, `ccBoss08_b::_DrawElement` | none | | | |

`t` in `ccChar::Draw` is `setTransparency` while the camera fade is off
(`hide`), else the transparency ([battle](battle.md#drawing-the-enemies-and-the-portals)).

`WORLD_MAN` +0x90 is `SetLightDirection` (0x001a21a0): the area's distant
light's `GetDirc` (0x00139240), its matrix times (0, 0, -1): the way the
light travels. `WORLD::Init` (a field), `DUNGEON::DUNGEON`, each
`ROOTTOWNnn` and `EVENTAREAnn` constructor set it; `EVENTAREA02` and `07`
again in `ChangeBlock`. `GO` copies it into the draw environment once, at
its end.

## The packets

`WORLD_MAN::GO` (every area, towns included, 0x001a0d60) makes two, on
sysLayer's view, at `objLayer`'s priority plus 2 (priority 2: after the
ground and the objects, before the characters, so the Z the volumes meet
is the scenery's):

| | buffer (`SetBuffer`) | +0x15d darkness | +0x15e copies | +0x15f spread | used by |
| --- | --- | --- | --- | --- | --- |
| A (`WORLD_MAN` +0x410) | 256 x 256 at (896, 0) | 0x30 | 1 | 14 | every character |
| B (`WORLD_MAN` +0x414) | 128 x 128 at (896, 0) | 0x20 | 2 | 14 | TOBJ |

Both buffers' Z is `ccGetPageAdrs(48, 960, 0) << 5`. A stream makes one per
shadow layer that has a Shadow chunk (below), mode 1, darkness 0x20, the
chunk's buffer, copies and spread.

`+0x15c` is the mode: 1 in every packet Infection makes (both constructors
set it; nothing else writes it). The code also has modes 2 and 3 (below).

## Preparing a shadow model

`DecodeShadowModel` (0x00140920) reads the mmat (`vNum`, `iNum`, the s16
positions, the triangles) and builds:

- **Directions.** Each triangle's `(p1 - p0) x (p2 - p0)` in 64-bit
  integers kept to their low 32 bits (`SetShadowNormal2`, 0x001400e0).
  Normalised as floats (`sceVu0ITOF0Vector`, `sceVu0Normalize`), a
  direction whose dot product (`sceVu0InnerProduct`) with one already kept
  exceeds 0.99609375 (`c.le.s` against `0x3f7f0000`) is that one: nearly
  flat neighbours share a direction. 26,321 of the triangles in
  Infection's `DATA.BIN` share one. A kept direction goes to VU1 as
  `vitof12` of the integers (a `cvt.s.w`, then a divide by 4096) times
  `1 / 4096` (`sceVu0DivVector`'s `vdiv` and `vmulq`), w 0: so a lane past
  2^24 is truncated to 24 bits. The same code on every disc (MUT
  0x00140160, OUT 0x0013e8b0, QUA 0x0013e670). `piney_data::shadow` runs it
  in the EE's arithmetic (`shadow::direction` for the floats), and
  `tools/test_shadow_rs.py` compares the stream's float bits exactly.
- **Edges.** For each triangle's edges `(a, b)`, `(b, c)`, `(c, a)` in turn
  (`SetShadowWork2`, 0x001403c0): an edge kept as `(b, a)` - matched by
  vertex index - joins: with the same direction it is interior and goes
  (the last edge moves into its place); otherwise the new triangle's
  direction becomes its first face and the old one its second, and the
  join counts convex when the third vertex lies behind the old face's
  plane (the integer dot product negative), concave otherwise. No match: the
  edge is kept with the face's direction and `-1 - direction` (open). Four
  edges in all of `DATA.BIN` stay open.
- **Refusal.** More concave joins than convex, 495 directions or more, or
  no edges: the model is dropped (`ccPrtChunkName`) and casts nothing.
  None in `DATA.BIN` is.

The VIF stream it leaves (mmat +0x40, size +0x44; the direction count at
+0x10): the directions (UNPACK V4-32 to VU1 0x212, each `itof12(n) / 4096`,
w 0) then `FLUSHE`, `MSCNT`; `BASE 0`, `OFFSET 0xc2`, `STROW (0, 0, 0,
1.0)`, `STMASK 0x00404000`, `STCYCL 3, 3`; the triangles in batches of 24,
each a GIF tag (TRIANGLE, ABE, context 2; A+D, A+D, XYZ2 x 3) and V4-16
vertices (the first vertex's w is its direction's address, 0x212 plus the
index), then `MSCAL mc_DrawShadow2`; `STCYCL 4, 4`, `BASE`, `OFFSET`; the
edges in batches of 24, each a GIF tag (TRISTRIP; A+D x 2, XYZ2 x 4) and two
V4-16 vertices (w the two faces' addresses), then `MSCAL mc_DrawShadow3`.

## Drawing one (`ccShadowModel::Draw`, 0x001412b0)

Nothing without a packet, with mode 0, without directions, or with alpha 0
(the alpha is held at 128). With `L` the draw environment's light, `R` the
transpose of the object's world matrix's 3 x 3 (`sceVu0InversMatrix`, the
translation dropped), `s` the model's vertex scale and `len` the length:

- `L_local = R L` (for the lit test) and the stretch `e = R (L / s) len`
  in stored units (mode 3: none).
- `M = B V W S`: `S` the scale, `W` the world matrix, `V` the view's
  `world_screen` (+0xd0), and `B` from the view's `GetScreenClip` (c0, c1,
  c2, c3) to the buffer: `x' = (x - c0) w / (c2 - c0) + (4096 - w) / 2`,
  likewise y.
- The near point `B P (0, 0, max(near, 8), 1)`, `P` the view's +0x90; its
  w is the near distance and its Z always past what `ftoi4` holds.
- `_ccCheckBoundingBoxEx` (0x0013e9e0) with `mc0_CheckBoundingBoxShadow`
  (VU0 0x000), on `V W` (before `S`): 16 corners through the matrix, culled
  (0) when for one side of the clip box (+0x1d0, +0x1e0: the clip with the
  near and far w) none lies within, 1 when all lie within the guard box
  (+0x1f0, +0x200: `ccLayer::Init`'s (0, 0, 0, 8) and (4095, 4095, 0,
  far)), else 2. The first eight are the model's box (`ccModel` +4); the
  program's first two `add` only touch w (`add.w vf21, vf19, vf20`), so the
  other eight are built from the reach `L_local len` itself (the tip of the
  model origin's shadow) and from `vf21`'s x, y, z as the object's
  `ccObj::Draw` left it (below); the check leaves `vf21` its matrix times
  its last corner.
- The box: `DecodeShadowModel`'s extremes of the stored positions (from
  0x10000 and -0x10000, `Decode_Model` 0x0014bddc; a value below the least
  so far is not also tried as the greatest, 0x00140ab4, so the first
  vertex never sets a maximum) through `ccBbox_SetBox` (`cvt.s.w` times
  `scale / 4096`).
- The GS registers by mode, swapped when the world matrix mirrors
  (`(col0 x col1) . col2 < 0`):

| mode | first colour | first ALPHA | second colour | second ALPHA |
| --- | --- | --- | --- | --- |
| 1 | R 1, A 0x80 | `alphaBlendTbl[1]` (add) | R 1, A 0x80 | `alphaBlendTbl[2]` (subtract) |
| 2 | R 0x40 | add | G 0x40 | add |
| 3 | R 0xff | mix | 0 | add |

- 17 qwords to VU1 0x184 (the scissor rectangle, the near point, the guard
  band `(0, 0, *, near)` to `(4095.75, 4095.75, *, far)`, `M`, the clip
  path's GIF tags, the four A+D registers, `L_local`, `e`), `MSCAL
  mc_DrawShadow1`, then the model's stream by REF, the packet given to
  `AddPacket(alpha)`.

The programs, in VU1's arithmetic (every lane through the EE's FPU model,
a multiply-add rounding its product first; `piney_data::vu1`):

- `mc_DrawShadow1` (VU1 0x5d2) loads the registers, copies the fan's two
  tags to 0x203, 0x204 and 0x207, and ends; `MSCNT` runs its tail after the
  directions arrive: each direction's flag (its w) is 0x80 when `(L.x n.x
  + L.z n.z) + (0 + L.y n.y)` is negative (its sign bit: it faces the
  light), else 0. The software pipeline stores three flags below the
  directions first: 0x20f's is the MAC flag the last program left, 0x210's
  the sign of that program's `vf18.x + vf18.y`, 0x211's the test on the
  fan's tag (0x18d) taken as floats.
- `mc_DrawShadow2` (0x5ef): each triangle of a batch of 24, its three
  vertices (`ITOF12`, w 1) moved by `e` when its face's flag is 0 (the far
  cap; the lit faces stay, the near cap), times `M`, divided (`DIV Q`,
  `MULQ.xyz`), `FTOI4`; `(v1 - v0) x (v2 - v0)` of the divided x, y
  negative (wound clockwise on screen): the first colour and ALPHA, else
  the second. The kick holds every triangle: its five qwords, the first two
  vertices' w 0, the third's 0x7f80, or 0x8000 (ADC) when it goes through
  the clip path.
- `mc_DrawShadow3` (0x641): each edge of a batch of 24 whose two faces'
  flags differ (the silhouette): `(a, b, a + e, b + e)` when the first
  face is lit, `(b, a, b + e, a + e)` when not, as a strip (the first two
  vertices' ADC set, the last two 0x7f80 or 0x8000); the tag's NLOOP counts
  the quads. Its first triangle's winding picks the registers.
- **The path.** A polygon with a vertex outside the guard band (x or y
  below 0 or above 4095.75, w below near or above far) takes the clip path:
  `FSSET 0` clears the status flag's sticky bits, the six (eight)
  `SUBA.xyw` against vf07 and vf08 follow, and `FSAND 0x80` reads the
  sticky sign four pairs after the last. Every flag the shadow, model and
  box programs test (`FMAND`, `FSAND`) stands exactly four pairs after the
  instruction it tests (the status flag's non-sticky sign at 0x0e1 and
  0x128 too), so the flags land at the end of the FMAC's four stages; the
  `FSSET` sits in the pair before the first subtraction in both programs,
  which only isolates them if its write lands among those of the FMAC in
  issue order. So only the guard band's subtractions reach the test:
  earlier operations (the next vertex's transform, the screen edges, the
  cross product) do not, and a polygon inside the guard band is drawn
  directly, its vertices held where Z passes what `FTOI4` gives.
- The queued polygons (raw, before the divide; a quad as `a, b, b + e, a +
  e`) go through `mc_DrawShadowClip` (0x6a7): cut at the near plane's w
  into the part in front and the part behind (at 0x1ed and 0x1f3), the
  behind part's vertices put on the plane (x and y kept, the near point's
  z and w: the near cap the camera sees when it is inside the volume), a
  crossing made from the vertex in front toward the one behind (`d =
  behind - front`, `front + d (near.w - front.w) / d.w`); each part through
  `mc_ScissorShadowPolyXY` (0x6e2): its winding from its first three
  vertices divided, a kick of 0x203 (the fan's tag, 0x204 and the colour
  0x205 as the last polygon left it) before the colour and ALPHA are
  written to 0x205 and 0x206, then cut to the scissor in homogeneous space
  (x low, x high, y low, y high; `mc_ScissorShadowPoly`, the buffers taking
  turns at 0x208 and 0x1f9, a vertex out when `bound.lane v.w - v.lane
  bound.w` is negative, a crossing from the vertex in), divided, `FTOI4`,
  each vertex's w 0x8001 counting down (ADC on the first two), and kicked
  from 0x204 as a TRIFAN.

An open edge's second face is read from VU1 memory below the directions,
`0x211 - face`. The four in `DATA.BIN` are `e4w1`'s, faces 8 and 10: 0x207
holds the fan tag's w (0: unlit) and 0x209 what the last clipped polygon
left there (a fan's second vertex, w 0x8000, which matches no flag; or a
scissor pass's float w when the polygon was cut away whole).

## vf21 before the check

The only EE code that writes `vf21` is `ccCoord::_SetLWMatrix`
(0x00138380); the only VU0 programs are the two box checks. `ccObj::Draw`
(0x0013f220) runs, in order, before its shadow model:

- `_SetLWMatrix` when the object has a parent (+0x80; else
  `sceVu0CopyMatrix`, no VU0). It pushes each coord of the chain on VU0's
  stack as `qmtc2` of `$a0` (the pointer, its sign, `$a0`'s upper 64 bits)
  and leaves `vf21` the qword above the topmost changed coord (+0x8d): vf0
  when that is the root, else its parent's. `ccAnm::SetAnm` hangs every
  entry from its parent slot's object or the anm, and every stream node
  hangs from a node or its clump's coordinate, so an object drawn by
  either always has a parent.
- `ccModel::Draw` (0x0013eab0) when the model is drawn (+0x94, +0xa2 3, its
  transparency above 1/128): a rigid model with mmats (+0x22) is checked
  against its box (`mc0_CheckBoundingBox`), which leaves `vf21` the matrix
  (`world_screen` times the object's world) times the box's greatest
  corner, culled or not. A bone or skin model has no box; its
  `DrawBoneType` runs `_SetLWMatrix` per bone.

Every shadow owner in `DATA.BIN` has a model: 2,582 an empty placeholder
(0 mmats), 609 a rigid one. The stack's qword is a heap address over the
drawing thread's `$a0` upper half: in `ee_run` (Outbreak, the event-218
save, 7,200 frames) every one of 65,566 such checks read x a `ccCoord`'s
address (0x00a9xxxx to 0x0159xxxx), y 0 and z, w 0 in the event's thread,
where no instruction but `_SetLWMatrix`'s own `qmfc2 $a0, $vf20` touched
the upper half, or, in the field's, z the ground attribute `setNearest`
(OUT main 0x00152c70) last copied with `lq $a0` (hitResult +0x48,
0x20004f8f). Run through `mc0_CheckBoundingBoxShadow` in `tools/vu.py`,
each gives the same result with x and z 0. Of the 3,367 checks after a
rigid model's own, 3,243 give another with `vf21` 0.

## A frame's packet (`SetShadowPacket`, 0x00142b20)

`GetPacketList` (0x00142ec0) keeps one group of volumes per alpha,
descending; a sixteenth alpha joins the nearer of its neighbours
(ties: the higher). `SetShadowPacket` sends nothing without a group, and
for mode 1 builds, in this order (a layer draws what it was sent last
first, so the GS runs it backwards):

1. `TransShadowTex(n)`, `n` 1 unless a single group of alpha 128.
2. For each group: (with `n`) `FillShadowBuff(alpha)`, then
   `FillTexBuffer(0x80808080, subtract)`; the group's volumes;
   `SetShadowGSReg`; `FillTexBuffer` of grey (the last group: with alpha 0,
   else keeping the alpha).
3. `TransFrameBuffer`: the Z buffer to the buffer's Z.

The GS then:

1. Copies the frame's Z into the buffer's (context 2, a sprite reading Z
   as a texture at the buffer's points, unfiltered: buffer pixel (i, j)
   takes frame pixel `(c0 + i W / w, c1 + j H / h)`).
2. For each group, the last first: fills the buffer grey (0x80) (the
   first also sets alpha 0); draws the volumes (`SetShadowGSReg`: FRAME
   masking alpha, Z tested GREATER against the copy, not written):
   front-facing faces add 1 to red, back-facing subtract 1; subtracts 0x80
   (a pixel counted above 0 keeps its count, the rest 0); `FillShadowBuff`
   reads the buffer back as 24-bit with TEXA's alpha the group's where the
   colour is not black, alpha-tested GEQUAL: a pixel counted above 0 takes
   the group's alpha. Where groups overlap the earliest in the list (the
   highest alpha) is written last and wins.
3. `TransShadowTex` lays the buffer over the view's clip box: black,
   MODULATE by the darkness (`+0x15d`), MIX, bilinear; the texel's alpha
   its group's (one group of 128: TEXA's 0x30 where counted, the same). It
   is laid `+0x15e` times, each at `zureTbl[k(k - 1) / 2 + i] * +0x15f >>
   4` sixteenths of a pixel (`zureTbl`, 0x002fb130: (0, 0); (16, 16), (-16,
   -16); (0, 14), (-14, -8), (14, -8); (16, 16), (-16, 16), (16, -16),
   (-16, -16)): packet A once, B twice 0.875 pixels either way.

So a character's shadow darkens the ground by `alpha x 0x30 / 0x80` of
0x80 (37.5% at full alpha), in 2 x 1.75 pixel cells, blurred by the
filter.

## Streams

`Decode_Layer` numbers shadow layers (kind 1) with the draw layers ([the
stream page](stream.md)). A Shadow chunk (0x1800) belongs to a shadow
layer's object (0 the default one):

```
u32 obj        the LYR_ object, 0 the default shadow layer
u16 x, y       SetBuffer: the buffer's place
u16 w, h       and size
s16 px, py     ccGetPageAdrs(48, px, py): its Z
u8  copies     +0x15e
u8  spread     +0x15f
u16 pad
f32 length     the packet's +0x170
```

`InitScene` makes a packet (0x180 bytes: +0x160 the light, (0, 0, 0, 1);
+0x170 the length; +0x174 the alpha, 0) for the default shadow layer and
each shadow layer with a Shadow chunk, when the file has a Layer chunk. A
draw list entry takes its node's Obj2 shadow layer's packet, else the
default one. `F_Shadow` (0x1801, 20 bytes: `u32 obj, f32 rx, ry, rz, f32
alpha`) sets that layer's packet (else the default's): the light
`sceVu0RotMatrix` of the degrees turns (0, 0, -1) to, and the alpha.
Until one does, the alpha is 0 and nothing is cast.

17 of the 266 stream files have Shadow chunks, one each: `str7101`-`7104`
and `str7110`-`7112` (225 `F_Shadow` records each) and `str0090`, `0110`,
`0120`, `0150`, `0301`, `0305`, `0350`, `0570`, `0580`, `0610`.

## The port

```
piney_data::shadow         ShadowMesh::build: DecodeShadowModel
piney_data::vu1            VU1's arithmetic: ITOF12, FTOI4, the divide, the
                           MAC flag's sign
piney_desktop::shadow      draw (draw.rs): ccShadowModel::Draw to a Setup
                           (the matrices, the box check, Mode and the two
                           Blends); kicks (vu1.rs): VU1's programs to Kicks
                           (Triangles, Quads, FanTag, Fan of Xyz2s), with
                           Vu1Carry what a draw leaves the next; gif.rs:
                           Kick::words, the GIF packet word for word, and
                           gs_triangles, what the GS draws of it;
                           ShadowPacket (GetPacketList, SetShadowPacket's
                           pass: VU1 runs the volumes in the GS's order),
                           Shadows (the draw environment, GO's packets,
                           cast, the frame's passes, VU0's vf21 and the
                           Vu1Carry kept); Vf21 (Known, Chain) and
                           Prelude, what the owner's ccObj::Draw leaves
piney_desktop::layers      Layers::shadows; flatten puts each pass on its
                           layer
piney_draw                 Cmd::Shadow(ShadowPass), ShadowTri::pixels (the
                           GS's pixels and Z, exactly)
piney_gs::shadow           the pass on the GPU: the Z copy, the counts
                           (R16Float, additive), the alpha, the composite
piney_desktop::soft        the same on the CPU, exactly
piney_world::draw          go_shadows (GO's packets from the area's distant
                           light), char_shadow / shadow_env (ccChar::Draw's
                           alpha and length), cast_shadows (ccObj::Draw's
                           half, its Prelude: a parent, the model's box
                           checked above 1/128)
piney_stream               file (Shadow chunks, shadow layers), scene
                           (SceneShadow, F_Shadow, each node's packet),
                           draw (the stream's packets and casts)
```

The GS's pixels as the repo models them: a pixel whose top-left corner
lies inside a triangle (x, y in sixteenths), or on a top or a left edge of
it, so that two triangles sharing an edge never both take a pixel on it;
its Z the plane through the three vertices there, truncated, compared
unsigned (ZTST GREATER).

## Checks

`tools/test_shadow_rs.py`:

- `DecodeAgainstGame` runs `DecodeShadowModel` in eemu on every shadow
  mmat in `DATA.BIN` (1,957 distinct) and compares with `shadow_probe`:
  refusals, every direction (the stream's float bits, against the port's
  and against `vitof12` and the divide by 4096 of the port's integers),
  every triangle and edge in order with its positions and faces. All
  match, on every disc.
- `VolumeAgainstGame` runs `ccShadowModel::Draw` in eemu (a real
  `ccView` from `SetFrame` and a camera, random world matrices - turned,
  tilted, now and then mirrored, one in eight looking away - lights,
  lengths and buffer sizes, the decoded stream, the box from the game's
  `ccBbox_SetBox`), its box check through `mc0_CheckBoundingBoxShadow` in
  `tools/vu.py`'s VU0 and its packet through a VIF1 model into its VU1,
  both VUs kept from one draw to the next, the flags four pairs late
  (`FSSET` among them in order); each `XGKICK`'s GIF packet is read when
  kicked. `shadow_volume_probe` runs the port on the same draws, carrying
  its VU0 `vf21` and VU1 memory. Compared exactly: the box, whether the
  check lets the draw on and `vf21` after it, every word of every kick, and
  every pixel of the buffer each side's GS model counts (+1 add, -1
  subtract, against a flat Z of 0 and of the draw's median). 300 random
  draws and eight of the one model with open edges (`e4w1`): on Infection
  308 draws, 10 culled, 444 clipped polygons, 246,116 kicked words and
  29,753,344 pixels the same; on each later disc 8 culled, 429 clipped,
  about 238,000 words and 29,851,648 pixels.

- `OwnerAgainstGame` runs `ccObj::Draw` itself in eemu (a VU0 machine
  with VU0's integer registers and stack): an object under a parent, its
  `_SetLWMatrix`, its model's `ccModel::Draw` up to the check
  (`ccSetMatrixPacket` refused, so nothing is drawn), then its shadow,
  `vf21` one register between the EE's macro code and the VU0 program.
  Ninety draws, a third each with the model seen, empty and faded: seen,
  `vf21` the check's corner word for word; else the stack's qword (the
  parent's address, 0); every box, check and kicked word the same as the
  port's Prelude gives.

`piney-gs`'s `shadows_darken_where_a_volume_meets_the_floor` renders a
pass over a floor on the GPU; piney-desktop's `groups_by_alpha_descending`
and `a_box_casts_a_closed_volume`; the session test
`kite_casts_his_shadow` walks Kite in story area 14's field and in Mac
Anu and checks the pass and the CPU GS's darkened ground. `stream_shot
--no-shadows` and `--info` show a stream's passes.

## Unknown

- The stack's qword's bits (a `ccCoord`'s heap address and `$a0`'s upper
  half) the port does not have: it reads x, y, z as 0. By the EE's adder
  (exact, then truncated) a tiny x or z moves a corner by at most one step
  of its last bit, so the check's result differs only on a corner within
  one step of a bound; none of the 65,566 in `ee_run` does.
- VU1 0x209 before a pass, which `e4w1`'s open edge on face 8 reads (and
  0x20f, 0x210: the MAC flag and `vf18` the last program left, read by no
  model). The shadow pass runs no other program, so it is what the
  frame's model batches left: a rigid batch in buffer A (TOPS 0xec) of 21
  vertices or more writes vertex 20's XYZ2 there, w its ADC word
  (`mc_DrawTri*` output from TOPS + 0xc9, four qwords a vertex); a bone
  batch of 81 or more at 0x118 vertex 80's position, w the row's 1.0;
  the scissor and effect programs their buffers. The port draws models on
  the GPU without VU1's memory, so it carries what the last pass left.
- Modes 2 (front faces into red, back into green, both added) and 3 (the
  model flat, front faces opaque) are never made in Infection; their
  `SetShadowPacket` path (`FillTexBuffer`, the Z copy, `TransShadowTex(0)`)
  is not ported.
- Packet A's `+0x15e` is the constructor's 1 and B's is 2; streams give
  their own. Whether a stream's copies of 0 (no composite) occur.
- `EVENTAREA02` and `07` set the light again in `ChangeBlock`, after `GO`
  copied it: the port takes the area's distant light each frame.
- A stream's node whose Obj2 names shadow layer 0: its entry's table slot
  read as the default; the port uses the default packet.
