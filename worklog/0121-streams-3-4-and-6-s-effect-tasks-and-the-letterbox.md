---
number: 121
title: Streams 3, 4 and 6's effect tasks and the letterbox
date: 2026-09-25
area: render, test
files: crates/piney-stream/src/effect.rs, crates/piney-stream/src/scene.rs, crates/piney-stream/src/draw.rs, crates/piney-desktop/src/view.rs, crates/piney-desktop/src/camera.rs, crates/piney-desktop/src/noiz.rs, crates/piney-desktop/src/fade.rs, crates/piney-stream/tests/effects.rs, tools/test_stream_rs.py, crates/piney-game/src/session/tests/survey.rs, docs/engine/stream.md
---

# 121. Streams 3, 4 and 6's effect tasks and the letterbox

Of the stream effect tasks, only `str0001`, `str0120` and `str0300` were
ported. The scenes of streams 3, 4 and 6 played with no effects.

## The tasks

- **`Func_str0090` (stream 3).** This task uses the same effect objects
  as the others. It turns on a feedback of 1.001 at alpha 0x6c. Cue 6 or 16
  puts the feedback in its fading state with no fade time, so it keeps
  drawing at alpha 0.
- **`Func_str0110` (stream 4).** Plain feedbacks, timed feedbacks, the
  noise, the inversion and the fog off, by cue (`docs/engine/stream.md`).
- **`Func_str0130` (stream 6).** Before making its objects it letterboxes
  the scene's view: `ccView::SetFrame(0, 48, 512, 288, 256, 192, 0.75,
  0.75)`. Its only cue, 899, fades the letterbox to black over the frames
  left.

## The letterbox

The scene's layers and the task's own layer share one view, so
`SetFrame` changes everything drawn in them:

- **The 3D.** The centre moves to (2048, 2104) and the picture is drawn
  at 3/4 size. `piney_desktop::view::Frame` holds the arguments, and
  `camera::frame_projection` turns them into a projection. The scene
  draw also uses the frame's scissor (rows 56-391).
- **The effects.** The bands, the feedback and the inversion read the
  view's scissor, clip box and `layer_screen` scales. In the port they
  now read them from `noiz::EffView`. So the bands' `Init` makes 21-row
  bands and draws 168 numbers, not 216.
- **The fade.** `EntryFade` is given the view's rectangle.
  `ccScFade::SendPacket` draws on the font layer through that layer's
  default view: the corner by `ApplyLayerScreenMatrix`, the size
  truncated. That is `fade::draw_rect`. The whole-frame fade gives the
  old constant corners.

The game's own `SetFrame`, run in eemu, sets the default `scy` to
0x44ffffff, one ulp under 2048. This is the EE's truncating multiply. The
port's `default_projection` now goes through the same EE float
arithmetic, so it gives the same value.

## Checked

- **`tools/test_stream_rs.py effects str0090 | str0110 | str0130`.** Each
  runs the game's task in eemu and writes a fixture.
  `crates/piney-stream/tests/effects.rs` plays streams 3, 4 and 6 for
  real. Every pass's primitives match bit for bit (by hash), and so does
  `rand` after each pass: 1,203 passes and 637 primitives for stream 3,
  1,496 and 1,474 for stream 4, 1,043 and 67 for stream 6.
- **`view::tests::letterbox_frame`** checks the frame's centre, clip box
  and scissor against the values the game's `SetFrame` leaves in eemu.
- **`stream_shot --stream 6 --frame 400`** shows the black bars above
  and below the scene.
- **Also.** `session::tests::survey::story_survey` (ignored, a
  diagnostic) runs each M1 story start under the autopilot. It prints
  what each start reached, the host calls left at their defaults, and
  any panic.

**Still unknown:**
- **The fog.** `Func_str0090`'s and `Func_str0110`'s fog is not drawn.
- **Unported tasks.** Effect tasks of streams 7-9, 11-16 and of the
  `str7xxx`, `str8xxx` and `str9xxx` scenes are not ported.
- **Clipping.** The port's draw does not clip by the letterbox's clip
  box (`bboxClipMin` / `Max`), only by its scissor.
