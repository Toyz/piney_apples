---
number: 513
title: field_probe reports arms and runs on its disc's volume: test_field_rt passes on all four volumes
date: 2026-10-29
area: test, world, volumes
files: crates/piney-world/examples/field_probe.rs
started: 2026-10-29T21:14:04-06:00
took: 10m
---

# 513. field_probe reports arms and runs on its disc's volume: test_field_rt passes on all four volumes

`test_field_rt.py` errored before comparing a single frame: KeyError
`arms`. Worklog 378 added `arms` (the `ArmsEffect` / `ClearArmsEffect`
calls from `ccPlayer::Main`) to the shared world harness's state and to
`world_probe`. Worklog 474 did the same for `evarea_probe`, but nobody
did it for `field_probe`. The probe now reports `arms` as the other two
do.

That made Infection and Mutation pass. Outbreak and Quarantine then
failed three tests each:

- `test_frames`: Kite's `transparency` 3 ulps off at walk frame 315;
- `test_frames_wrap`: `cam_pos`, `world_view` and `world_screen` a few
  ulps off at frame 153;
- `test_draw_object`: 451 mismatches.

This is the cause worklog 474 found in `evarea_probe`. The probe built
its field with `FieldArea::new` (Infection) and its player and camera
without a volume, so it took Infection's double square root where
Outbreak's and Quarantine's code has `sqrt.s`. It now reads the disc's
volume and builds with `FieldArea::with_world(volume, ...)`,
`Player { volume, .. }` and `Camera { volume, .. }`, as `world_probe`
does. The game itself always ran with the volume's square root: only
the probe was wrong.

Checks:
- `test_field_rt` passes on Infection, Mutation, Outbreak and Quarantine
  (5 tests each).
- piney-game's suite passes; clippy and fmt are clean.

**Still unknown:** nothing
