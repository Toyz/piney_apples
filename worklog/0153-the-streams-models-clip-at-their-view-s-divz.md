---
number: 153
title: The streams' models clip at their view's divZ
date: 2026-09-26
area: render
files: crates/piney-draw/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-gs/src/lib.rs, crates/piney-stream/src/scene.rs, crates/piney-stream/src/draw.rs, crates/piney-stream/src/lib.rs, docs/engine/stream.md
---

# 153. The streams' models clip at their view's divZ

The last GAPS drawing item. VU1's unlit rigid program (`mc_DrawTriSFast`)
handles a triangle that is not wholly inside the GS space and w range by
its last strip vertex. When that vertex is nearer than the view's `divZ`
(+0x25c) it cuts the triangle at the near plane; otherwise it drops it.
piney-gs used a fixed 1000. The streams change it: stream 5's cue 12 sets
500, and stream 15's tasks set 2000 (`str0580`) and 3000 (`str0581`).
`piney-stream` tracked these and sent them out as `Request::DivZ`, but its
draws did not use them.

## The change

- `piney_draw::ModelDraw::div_z`, with `piney_draw::DIV_Z` (1000) as the
  default that the towns, fields, dungeons, desktop and title pass.
- `piney_gs::convert::mmat` takes it in place of the constant.
- `piney_stream::Scene::div_z`: the stream sets it from its tracked
  `divZ` before each frame's draw, and the scene's models carry it.

## Checks

All 585 workspace tests pass. The streams' fixture comparisons are of
draw lists, which the change does not alter. The clip itself is unchanged
code with a parameter in place of a constant.

**Still unknown:** no close-up of stream 5 or 15 was pictured against the
game. Stream 15's worklog found that `divZ` 2000 changes nothing in
`str0580`'s first 420 frames. `str0581`'s 3000 and stream 5's 500 were
not looked at frame by frame.
