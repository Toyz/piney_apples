---
number: 160
title: The shadow volumes: ccShadowModel, ccShadowPacket and the VU1 programs
date: 2026-09-26
area: render
files: crates/piney-data/src/shadow.rs, crates/piney-desktop/src/shadow.rs, crates/piney-gs/src/shadow.rs, crates/piney-desktop/src/soft.rs, crates/piney-world/src/draw.rs, crates/piney-stream/src/draw.rs, tools/test_shadow_rs.py
---

# 160. The shadow volumes: ccShadowModel, ccShadowPacket and the VU1 programs

Until now no shadow was drawn anywhere. GAPS listed TOBJ's and the streams'
shadows, but the gap was wider: every character body carries a shadow
model on each of its objects (a PC has 15 to 31 of them), and so do the
weapons and the dogs. Kite, the party, the NPCs and the enemies all stood
on the ground without one. `docs/engine/shadow.md` has the whole of it.

The game draws stencil shadow volumes. A shadow model is a closed mesh.
`DecodeShadowModel` works out each triangle's direction, merging those
within 0.996 of each other, and the edges between faces, matched by vertex
index. Each frame `ccShadowModel::Draw` sends VU1 the light in the model's
axes, the stretch (the light times the shadow's length), and a matrix to a
small buffer's pixels. VU1 then:

- lights each direction (`mc_DrawShadow1`);
- draws the faces, moving the unlit ones by the stretch (`mc_DrawShadow2`);
- draws a quad for each edge between a lit and an unlit face
  (`mc_DrawShadow3`).

The screen winding picks the add or subtract blend. At the frame's end,
each `ccShadowPacket` builds a short GS program, run backwards because a
layer draws what it was sent last first:

1. The frame's Z is copied, point sampled, into the buffer's Z.
2. The buffer is filled with grey.
3. The volumes are counted into red against that Z (GREATER, no write).
4. 0x80 is subtracted, and each group's alpha is written where the count
   is left above 0.
5. The buffer is laid over the frame, black, bilinear, at the darkness
   (0x30 of 0x80 for the characters). TOBJ's 128 x 128 packet is laid
   twice, 0.875 pixels either way, by `zureTbl`.

`WORLD_MAN::GO` makes the two packets in every area, towns included (a
field-game.md Unknown had it that towns made none). They sit at priority
2, so the Z the volumes meet is only the ground and the objects. The
light is the area's distant light, from `SetLightDirection`.
`ccChar::Draw` sets the alpha from the transparency and the length to
three times the height, then puts the alpha back to 128. In a stream, each
shadow layer with a Shadow chunk has its own packet, and `F_Shadow`
records turn its light and alpha frame by frame. 17 of the 266 stream
files have one, 10 of them story cutscenes.

The port follows those pieces:

- `piney_data::shadow::ShadowMesh` is the decode.
- `piney_desktop::shadow::volume` is the draw and the VU1 programs, done as
  polygons in buffer pixels with GS Z. It cuts at the near plane the way
  `mc_DrawShadowClip` does: the part behind the plane is put on it, the
  near cap.
- `ShadowPacket` and `Shadows` are the packets and the draw environment.
  They live in `Layers`, and `Layers::flatten` puts each packet's
  `Cmd::Shadow` pass on its layer.
- `piney-gs` runs the pass as four kinds of render pass: the Z copy, the
  count (R16Float, additive), the resolve, and the composite. The CPU GS
  does the same.
- In piney-world, `go_shadows` sets the packets up and `char_shadow` wraps
  each `ccChar::Draw`. `Body` draws and the enemies' draws cast their
  nodes' shadow models, and TOBJ casts into the second packet.
- piney-stream reads the Shadow chunks and shadow layers, decodes
  `F_Shadow`, and casts each draw-list node's shadow model.

Checks:

- `tools/test_shadow_rs.py` runs the game's `DecodeShadowModel` on all
  1,957 distinct shadow mmats: every direction, triangle and edge matches.
- The same test runs `ccShadowModel::Draw` in eemu and the packet through
  a VIF1 model and `tools/vu.py`'s VU1. Two things had to be added to the
  harness: FSSET/FSAND, and reading each XGKICK's GIF when it is sent,
  since the programs reuse their buffers. Both sides are counted into the
  buffer against two depth planes; 300 random draws match pixel for pixel.
- Two harness mistakes cost time. The test models first came out
  sub-pixel because they did not use their real vertex scale (512). Then
  pixel corners on fan edges were counted twice: the game's clipped
  vertices sit exactly on the corners.
- A piney-gs test renders a pass over a floor.
- The session test `kite_casts_his_shadow` checks the pass in area 14's
  field and in Mac Anu, and that the CPU GS darkens the ground.
- `stream_shot --no-shadows` shows stream 7's long shadow of Kite across
  the floor.

**Still unknown:** The console's choice between the direct and the
clipped path for a polygon. It turns on the status flag's pipelined sticky
sign. The two differ only where Z passes 0x7fffffff. The shadow bounding
box check reads a register (`vf21`) the program never sets. An open edge's
second face reads VU1 memory below the directions. Modes 2 and 3 are never
made in Infection and are not ported. `EVENTAREA02` and `07` reset the
light after GO.
