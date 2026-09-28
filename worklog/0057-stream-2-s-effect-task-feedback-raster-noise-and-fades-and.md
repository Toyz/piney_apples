---
number: 57
title: Stream 2's effect task: feedback, raster noise and fades, and frame-buffer textures
date: 2026-09-23
area: render, engine, test
files: crates/piney-stream/src/effect.rs, crates/piney-draw/src/lib.rs, crates/piney-gs/src/lib.rs, crates/piney-desktop/src/soft.rs, crates/piney-game/src/main.rs, tools/test_stream_rs.py, docs/engine/stream.md
---

# 57. Stream 2's effect task: feedback, raster noise and fades, and frame-buffer textures

After [[54]] the first cutscene's shapes were right, but its look was not.
Each stream may have its own effect task (`StreamDemoFuncTbl`). The one
for `str0001`, `Func_str0001` (0x001885d0), draws straight into and from the
frame buffer. A helper agent ported it and checked it against the game's
code; I put it in the runtime. The reference is "Stream 2's effect task" in
[the stream page](../docs/engine/stream.md).

## What the task does

**Timing.**
- It runs at priority 96, after the scene's task (24), every frame. Its
  first pass is one frame before the scene's first drawn frame.
- It takes the stream's 0x8010 cues last-queued first.
- After a natural end it runs in the first gap frame only.

**At start:**
- **The floor.** `OBJ_se1_6flo1` stops writing Z (NEVER with FB_ONLY).
- **Fog.** The scene's fog is set to black between depths 7,500 and
  20,000, with the sky, two cloud layers and the moon exempt.
- **Raster noise.** 8 bands are set up, which takes 216 `rand` calls.
- **Feedback.** It is switched on for layer 8 (`LYR_jm`).
- **A fade.** 30 frames in from black, on the font layer.

**The cues:**
- 5, 6 and 15 set the feedback's alpha to 0x58, 0x50 and 0x60;
- 11 starts the noise and 12 stops it;
- 999 fades to white;
- 800 and 16 do nothing.

**The feedback.** The previous frame's picture, drawn 1.015 times larger
and filtered, blended over layers 0-7. Its Z is 0x7fffffff, because the EE's
division by zero saturates, so it passes every model. This makes the
cutscene's dreamy trails.

**The raster noise** (layer 100). Each band copies 512 x 32 rows of the
current frame to spare VRAM, then draws one sprite per row, shifted by
`rand`. `rand` is newlib's generator; seed 1, since only the staff roll
calls `srand`.

## Frame-buffer textures

Two new `TexRef` variants in `piney-draw`:
- `FrameBuffer { x, y, width, height }`: that rectangle of the frame as drawn
  so far.
- `PreviousFrame`: the last finished picture.

On the GPU, a `FrameBuffer` draw ends the render pass, copies the rectangle
and goes on in a new pass that keeps colour and Z. `PreviousFrame` is a copy
taken before the clear. The CPU GS model has both.

Because the feedback samples the previous picture, the runtime now renders
every game frame. Before, the window loop drew only the newest frame when
it caught up, and `--shot` only the last one.

## Checked

- **Against the game's code.** `test_stream_rs.py effects` runs
  `Func_str0001` natively in eemu over all 2,043 steps of stream 2: the
  noise, feedback, fade and frame-copy code, the layer lists and `rand` are
  all the game's own. `tests/str0001.rs` compares every step bit for bit:
  - the step each cue is raised on;
  - all 4,164 primitives in GS order, with every register the draw sets and
    every vertex;
  - the `rand` state after each pass.

  I regenerated the fixture from the game's code in a clean worktree, and
  it is byte-identical to the one committed.
- **The new textures.** `frame_buffer_textures` (`piney-gs`, on the GPU) and
  a CPU GS model test check that the copy sees earlier draws and not later
  ones, and that rows past the frame read 0.
- **The rest.** The workspace's tests pass, and clippy and fmt are clean.
- **A shot.** Frame 822 shows the trails around the figure and the floor.

**Still unknown:**
- **Pixels.** The packets are checked, the pixels are not. The task's
  timing comes from the thread priorities and the code, not from a run.
- **Fog.** It is not drawn: `ModelDraw` has only constant fog, and
  per-vertex fog (VU1's `clamp(fogB + fogA * w)`) needs a new field.
- **`rand`.** Its state when stream 2 starts in the game depends on
  everything before it, so the noise matches only from a known state.
- **Past the draw buffer.** VRAM there reads 0 in the port.
- **The other tasks.** The table's 21 other effect tasks.
