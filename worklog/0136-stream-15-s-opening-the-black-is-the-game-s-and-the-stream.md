---
number: 136
title: "Stream 15's opening: the black is the game's, and the stream lights' order"
date: 2026-09-26
area: render, test
files: crates/piney-stream/src/scene.rs, crates/piney-stream/tests/stream.rs, crates/piney-stream/tests/stream_fixture.txt, crates/piney-stream/examples/stream_probe.rs, tools/test_stream_rs.py, docs/engine/stream.md, BUGS.md
---

# 136. Stream 15's opening: the black is the game's, and the stream lights' order

BUGS.md had "first few scenes of story:31 the camera clips the ground
culling it out". The ending's first scene, `str0580`, looks like this in
the port:

- the world goes black but for Kite and the stone figure;
- the sky comes back tilted across the top of the screen;
- the lower half stays black for about 200 frames.

[[133]] showed that neither effect task draws before frame 1225, so the
black had to come from the scene draw itself. This entry checks that draw
against the game's, frame by frame. The black turns out to be the game's.
The check also found a real bug, in the order of the scene's lights.

## The game's draw

- **The scene check.** `tools/test_stream_rs.py fixture` runs the game's
  `DecodeSetup`, `InitScene`, `DecodeFrameSection`, `SetView` and layer
  draw in eemu. It covered streams 0, 106 and 2. It now also takes
  stream 15's `str0580` to frame 420, and stream 6's first 60 frames.
- **What it took.**
  - `str0580e` uploads textures: `sceGsSetDefLoadImage` uses an MMI `por`
    the machine lacks, and the uploads touch the DMA registers. Those calls
    are stubbed. The draw list does not depend on them.
  - Kite's bones have spaces in their names (`EXT_t0 r belt3`). The
    fixture's draw and light lines now escape them as `%20`.
  - The file area runs from `DATA` up to `SYS`. `str0580` to frame 300
    already ran past `SYS` and overwrote `ccSys`'s screen size: the first
    camera came out with its x and y rows equal. The guard now checks
    `SYS`, not `HEAP0`, and a scene file that does not fit goes in the
    heap.
- **What it shows.** The table is in `docs/engine/stream.md`, "Stream 15's
  opening".
  - At frames 61-70 the scene's own tracks fade the whole `se1_5` set to
    0: the sky, the clouds, the ground and the towers.
  - From frame 121 the sky and clouds fade back in. The ground and towers
    never come back.
  - At frames 316-404 the next set, `se1_6`, fades in with its own floor.
  - The scene's fifteen smoke effect objects keep pattern 0xffff and draw
    nothing, and neither file holds particles.
- **Why the lower half is dark.** The camera looks down at the figures
  from above, past the horizon.
  - The sky dome, `MDL_se1_5bac1`, is a closed shell down to a pole at z
    -4928.
  - Its texture runs from brown at the horizon to black at the bottom
    pole.
  - Every sky triangle on screen at frame 145 is wholly inside VU1's
    space, so none is dropped.
  - The port's frame 145 falls from an average (50, 90, 55) on its top
    row to (3, 2, 1) near the bottom, a gradient and not an edge.
- **The divZ.** `Func_str0580` sets it to 2000, and the port clips at
  1000. In frames 1-65 only one or two ground triangles on screen are not
  wholly inside, and their last vertex is nearer than 1000 either way. So
  divZ is not the cause either.

## The lights

- **The failure.** With stream 15 in the check, the draw list, every
  object's transparency and matrix, and the camera matched. The lights did
  not. At frame 1 the game's slot 2 held omni light `LGT_se1_5omn01`
  (709) and slot 1 the distant `LGT_se1_5lig1` (728). The port had them
  the other way round.
- **The cause.**
  - `ccCreateLight` (0x001389b0) gives a distant light priority -1 and
    every other kind 0.
  - `ccLightGrp::AddGrp` (0x00139060) keeps the group in descending
    priority, equal priorities in the order added.
  - So the distant lights come after the omni ones. The port had kept only
    the file's reversed order.
  - `docs/engine/stream.md` also named `ccCreateLight` wrongly
    (0x00138960 is `ccBbox_SetBox`) and gave every light priority -1.
- **The fix.** `Scene::new` sorts the lights stably by that priority.
- **What it changes.** The order only matters when more than three lights
  answer, so stream 15's pictures do not change.
  - A survey over the streams' lit models: streams 3, 5, 6, 7, 8, 10, 11
    and 16 pick different lights now. In 6, 7, 11 and 16 that happens in
    nearly every frame.
  - Stream 6 is in the check to prove it. With the old order its first
    step fails: the port lit the first object drawn with a white distant
    light where the game uses an omni at 0.22.

## Checked

- **`scenes_play_as_the_game_plays_them`** now covers five scenes, 1347
  frames. It compares the draw list, each object drawn (name,
  transparency, `lwMatrix` bit for bit), the camera (worst 2.8e-5) and the
  lights (worst 5e-7). Streams 0, 106 and 2's lines are unchanged.
- **Shots.** `piney-game --mode story:31 --shot --every 30` after the fix
  is pixel-identical to [[133]]'s 180 shots. `stream_shot` before and
  after the light fix differs in streams 6, 7 and 11: Kite and Blackrose
  (stream 7) and Balmung (stream 11) are lit by other lights.
- **`stream_probe --all`** lists the nodes a frame does not draw too, with
  their `disp` and transparency.

**Still unknown:**
- **The game's pixels.** The check is of the draw state, not the picture.
  That the dome's bottom rows are the black under the figures rests on
  the texture and the port's render, not on a game frame.
- **The near clip.** `piney-gs` still clips at a fixed 1000, not at the
  view's `divZ`. Nothing in stream 15's opening depends on it.
- **The other streams' lights.** Only streams 6 and 15 are in the check.
  The priority rule is the game's code, but the pictures of streams 3, 5,
  7, 8, 10, 11 and 16 are not compared to the game.
