---
number: 133
title: "Stream 15's effect tasks: the ending's puffs, rocks and shades"
date: 2026-09-25
area: render
files: crates/piney-stream/src/ending.rs, crates/piney-stream/src/effect.rs, crates/piney-stream/src/lib.rs, crates/piney-stream/src/draw.rs, crates/piney-desktop/src/noiz.rs, crates/piney-desktop/src/soft.rs, crates/piney-draw/src/lib.rs, crates/piney-stream/tests/ending.rs, tools/test_stream_rs.py, docs/engine/stream.md
---

# 133. Stream 15's effect tasks: the ending's puffs, rocks and shades

Stream 15 is the ending, the last in-engine stream whose effect tasks
were not ported ([[121]]-[[126]]). It plays `str0580`, then `str0581`.
`Func_str0580` and `Func_str0581` keep the usual `ccStrEffectCtrl`.
Beside it, each has a table of cues that start parts at the scene's
objects, and a group of parts:

- puffs of smoke (`EFF_x001` of `str0580e`);
- in `Func_str0580`, rocks thrown up from the ground (`OBJ_se1_6ro*`).

`docs/engine/stream.md` ("Stream 15's effect tasks") has the details.

## What the tasks do

- **The walker.** `ccEventObj::SetObj(cue)` looks through the table from
  the entry it last found. Its loop tests are the wrong way round, so it
  finds a cue only when the cue is the very next entry (or the one
  before).
- **The walker meets the notes.** `str0580` gives 539, 540 and 541 in one
  frame, and the task takes a frame's cues last first. 541 and 540 miss,
  539 is found, and every later cue misses too. Only 40 of the table's
  entries ever play: 200 puffs and 138 rocks.
- **The rocks are never deleted.** A rock's `Ctrl` asks
  `ccObj::CheckBoundingBox`, which gives 1 for a model with no Bbox. The
  rock models have none. Every rock falls until the task ends, so the
  part draws grow to 89,116 over the scene.
- **The shades.** `Func_str0581`'s cue 55 sets two `ccBufferSampling`s
  with `SetShade`. That is REGION_REPEAT on the picture being drawn: 2-
  and 4-texel blocks, offset a little, at depth 1000. The Z test keeps
  them off what is nearer. Its feedback goes to that depth too.
- **The trails.** `Func_str0580`'s feedback sets its offset's y to 0.16.
  `MakePacket` adds the offset in sixteenths of the view, so the last
  frame's picture is drawn 4.5 pixels lower each frame and trails
  downwards.
- **`Func_str0580` outlives its scene.** After its loop it makes itself
  `eventExTscb2` and passes until the next scene's
  `ccSetStreamDemoThread` sets its `param[1]` to 2. Then it passes three
  more times. Each frame it passes before `Func_str0581`.
- **The view.** The tasks set the scene view's `divZ` to 2000 and 3000.
  `Func_str0581` sets a black fog (30000 to 50000) every pass, with three
  sky objects out of it.

## The port

- **The tasks.** `piney_stream::ending` holds `Str0580` and `Str0581`,
  the walker, and the parts. The parts give `PartDraw`s. The tasks read
  the scene's objects through `effect::SceneWorld`.
- **The parts' draws.** `Stream::draw_parts` draws them on `sysLayer`:
  - a puff through `piney_effect::eff::Eff`, from `str0580e` read into
    the stream's effect assets;
  - a rock with `draw::draw_rigid`, the scene's model draw split out of
    `draw_node`.
- **The tail.** `Stream`'s `tail` keeps `Func_str0580` after its scene and
  steps it before the next task.
- **The sampler.** `piney_desktop::noiz::Sampling` gained a centre, an
  offset, a depth, a page and a wrap. `Sampling::shade` is `SetShade`, and
  `Ctrl::shades` draws them.
- **The wrap.** `piney_draw::Wrap::Region` carries REGION_REPEAT. The
  software rasterizer reads it in blocks; `piney-gs` samples it as a plain
  repeat.

## Checked

- **The fixtures.** `tools/test_stream_rs.py effects str0580` and
  `str0581` run the game's tasks in eemu. They stand the tables' objects
  in by name and hook `ccEff::Draw` and `ccObj::Draw` to record the parts.
  The transfer records its note's object, and step 0 records the `divZ`.
- **`crates/piney-stream/tests/ending.rs`** compares the port with them:
  - every pass of `str0580` (1,950) and of `str0581` (3,218);
  - each pass's primitives, `rand`, part draws, transfers and the `divZ`;
  - all equal.
- **The divZ.** The first port set 500 for `Func_str0580`, a value
  misread from `Func_str0120`'s. The code has 2000 (`lui 0x44fa`). The
  fixtures did not record a `divZ` set before the first pass. They do now,
  and the test takes the value from them.
- **Stream 15 played for real** (`stream15_plays_through`):
  - `Func_str0580` over the scene's own objects sends the fixture's
    primitives and draws the fixture's `rand` to `str0580`'s last frame;
  - the cues come on the same steps;
  - the one transfer comes, and both tasks' `divZ`.
- **Shots.** `piney-game --mode story:31 --shot OUT --every 30` over the
  whole ending:
  - the rocks and the puffs fly from 1225 on;
  - the feedback's offset trails come from 1500;
  - `str0581`'s shades and fades show.
  With and without the parts drawn, frame 1250 differs where the rocks
  and puffs are.
- **The black sky.** The brief's black sky and missing ground (the port's
  frames 90-240) are `str0580`'s own. Its first frames draw only the sky,
  the clouds and the two figures. `OBJ_se1_5bac1`'s transparency is 0.9
  at frame 60, 0 at 90, 0.05 at 120 and 1 by 160. The tasks draw nothing
  of their own before frame 1225.

**Still unknown:**
- **Pixels.** The parts are checked as draws, not packets. The puffs go
  through `Eff` and the rocks through the scene's model draw. Their
  pixels against the game's are not checked.
- **The shades on the GPU.** `piney-gs` draws REGION_REPEAT as a plain
  repeat, so the shades are not blocky there.
- **The switch.** How many frames the game spends loading between
  `str0580` and `str0581` is not known. The port switches at once, so
  `Func_str0580`'s last passes may fall earlier than the game's.
- **`str0581`'s frame-0 note** (cue 900, a fade from white over 45). The
  check never gives the task a note on the scene's frame 0. The port's
  stream gives it to the task's second pass. Which the game does is not
  checked.
- **The near clip.** VU1's clip at the `divZ` is not modelled, as for
  stream 5.
