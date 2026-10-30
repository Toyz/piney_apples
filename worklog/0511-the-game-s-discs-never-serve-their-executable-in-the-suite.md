---
number: 511
title: The game's discs never serve their executable, in the suite too: the suite's check images get their port data as the launcher makes it
date: 2026-10-29
area: test, build
files: crates/piney-game/src/main.rs, crates/piney-game/src/session.rs
started: 2026-10-29T20:44:27-06:00
took: 16m
---

# 511. The game's discs never serve their executable, in the suite too: the suite's check images get their port data as the launcher makes it

Worklog 509 left `open_disc` strict outside tests only: under
`cfg!(test)` it opened the disc as it was. Without that, seven
Quarantine tests in the suite failed. Quarantine's check image had no
port data, so `official::events` fell back to `SLUS_205.64`. Library
code should not branch on test builds, so the carve-out is gone.
`open_disc` always opens with `Iso::without_executable`.

The honest fix is the one the game itself makes. Before playing an
image, `main` calls `piney_build::image_data`, which makes the image's
port data in its data folder when it is missing. The suite now does the
same through one test helper, `check_image(disc)`:

- It returns `work/DISC/DISC.iso`, there or not, so the tests' own
  skips still work.
- When the image is there, it calls `image_data`, holding a lock so only
  one test makes it at a time.

All 114 places where the suite built a check image's path now call it.
`launcher.rs`'s tests make data themselves and keep their paths. Tests
that read the executable on purpose (comparisons with the game's code)
open the disc the tool way, with `Iso::open`, as before.

The first run made Mutation's and Quarantine's port data
(`quarantine-109bc8000` and `mutation-df718000` beside the existing
two). For the Quarantine ending test that took 218 s, once.

Checks: piney-game's full suite passes (339, 143 ignored), and clippy
and fmt are clean.

**Still unknown:** nothing
