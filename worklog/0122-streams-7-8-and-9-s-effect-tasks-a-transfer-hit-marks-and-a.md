---
number: 122
title: "Streams 7, 8 and 9's effect tasks: a transfer, hit marks and a turned feedback"
date: 2026-09-25
area: render, test
files: crates/piney-stream/src/effect.rs, crates/piney-stream/src/lib.rs, crates/piney-desktop/src/noiz.rs, crates/piney-data/src/anim.rs, crates/piney-game/src/stream.rs, crates/piney-stream/tests/effects.rs, tools/test_stream_rs.py, docs/engine/stream.md
---

# 122. Streams 7, 8 and 9's effect tasks: a transfer, hit marks and a turned feedback

This follows [[121]] with `Func_str0150`, `Func_str0240` and
`Func_str0250` (streams 7, 8 and 9).

## The tasks

- **`Func_str0150` (stream 7).**
  - Cue 899 fades to black over the frames left.
  - Cue 500 is `effTransferStr`: the arrival's rings (stream effect
    controller -2) at the note's object, for a character 160 high.
  - It also sets a fog and takes one object out of it. Fog is not drawn
    by the port.
  - The port raises the transfer as `Request::Transfer` and does not draw
    it, as it does with the hit marks.
- **`Func_str0240` (stream 8).**
  - Eight hit marks, one counter over cues 600-605, 610 and 611, each
    taking the next row of `hitRot0240`.
  - The table has seven rows. The eighth mark reads the string
    "OBJ_trall" that follows it as degrees. The port reads the same eight
    rows from the executable, so its eighth rotation matches the game's.
- **`Func_str0250` (stream 9).**
  - A hit mark.
  - Fades from white on the 900s, and to white at the end.
  - Feedbacks: plain; ready to fade over 15 frames; and turned.
  - The noise and the inversion.

## The turned feedback

`SetReflex(scale, roll, colour)` keeps a turn at +0x0c. Every earlier
task passed 0. `MakePacket` turns its matrix with `sceVu0RotMatrixZ`
before setting the translation, so the picture turns about the view's
centre.

- `piney_desktop::noiz::Sampling` now has `roll`.
- `piney_data::anim::rot_z_bits_of` is the EE-exact `RotMatrixZ`, which
  `rot_bits_of` now uses for its first step.
- Stream 9's cues 910-930 turn the picture by 0.00697 radians.

## The harness

- **The task's end.** `Func_str0150` never looks at the scene's state. It
  leaves only when the next scene's `ccSetStreamDemoThread` sets
  `param[1]`, and after `ccDeleteThread` it breathes forever. The harness
  now sets `param[1]` at the end as well, and treats `ccDeleteThread` as
  the end of the run: the hook raises and the call stops.
- **The packets.** Their work comes from an arena emptied after each
  pass's lists are decoded. Stream 7's 2,200 frames had run the heap out.
- **Regenerated fixtures.** With these changes every earlier fixture
  comes out the same except for the new header line.

## Checked

- `crates/piney-stream/tests/effects.rs` plays streams 7, 8 and 9 against
  the game's tasks. Every pass matches by hash, as do `rand` after it and
  each mark's and transfer's object and bits:
  - stream 7: 2,203 passes, 32 primitives;
  - stream 8: 1,583 passes, 82 primitives, 8 marks;
  - stream 9: 3,563 passes, 4,586 primitives.
- The workspace's tests, clippy and fmt pass. The field menu's noise
  (roll 0) is unchanged.

**Still unknown:**
- **What the marks and the transfer look like** in a stream: the
  `ccThEffectStr` task that runs them is not ported.
- **Stream 7's fog** is not drawn.
- **The remaining tasks** are not ported: streams 11-16's and the `str7xxx`,
  `str8xxx` and `str9xxx` scenes'.
