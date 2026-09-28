---
number: 104
title: The noise: ccNoiz drawn on its own layer, and the events' interNoiz
date: 2026-09-25
area: ui, render, test
files: crates/piney-desktop/src/noiz.rs, crates/piney-desktop/src/lib.rs, crates/piney-stream/src/effect.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/render.rs, crates/piney-fieldui/src/menus/drain.rs, crates/piney-fieldui/src/menus/hack.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/gate_hack.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md, docs/engine/stream.md
---

# 104. The noise: ccNoiz drawn on its own layer, and the events' interNoiz

The gate hack ([[96]]) and Data Drain ([[102]]) break the screen up
through the menu's `ccNoiz`. The port only passed their calls on as
`Request::MenuNoise`, which nothing drew. The events' `noise1` and
`noise2` set `interNoiz`, which nothing read.

## What the game does

The details are on [the field UI page](../docs/engine/field-ui.md#the-noise-ccnoiz-0xe8).
- **`ccNoiz` (main 0x001bb080).** It keeps four frame counters: the
  raster bands, the sampling, the sampling's fade, and the inversion.
  `Draw` counts them down and draws, on layer 241, what the streams'
  effect tasks draw ([[57]]):
  - eight `ccRasterNoize` bands;
  - `ccBufferSampling` at 1.0284 and alpha 0x50, then six frames fading;
  - `ccBufferReverce`.
- **What differs from the streams.**
  - The bands' row offsets are drawn once, by the constructor.
  - `SetNoizRn` draws three numbers per band, not four.
  - `SetNoiz` plays sound 94.
- **`interNoiz`.** It is read at the end of `Disp`:
  - 2 is one burst.
  - 3 in town 4 gives a long sampling every 16th frame.
  - Otherwise there is a one-in-four chance every 64th frame out of the
    menus, and the noise is off inside one (but 66 and 62).

## The port

- **`piney_desktop::noiz`.** `Band`, `Sampling` and `reverse_prim` moved
  out of `piney_stream::effect`, which now uses them, and `Noiz` is
  added. The desktop crate is where the stream and the field UI both
  reach.
- **`MenuCtrl::noiz`** holds the state. `disp::noiz` makes the menus'
  calls (with the sound), `disp::inter_noiz` makes Disp's, and
  `Draw::Noiz` goes to layer 241 in `render.rs`.
- **`Request::MenuNoise` is gone.**

## Checked

- **The stream's fixtures.** `str0001`, `str0300` and `str0120` still
  match primitive for primitive after the move.
- **`noiz::tests`.** They cover the band height, the sampling's 8 + 6
  frames of alpha, and the draw order.
- **`tools/test_fieldui_rs.py test_inter_noiz`.**
  - It sets `interNoiz` 1, 2 and 3 (in town 4 and in town 1) with two
    `rand` values, and opens a menu and a ban over the 64th frames.
  - Every `SetNoiz` call Disp makes, and its sound, matches the game's.
  - It first failed only on the harness's own range, which ended one
    instruction short of the last call's return.
- **The sound on the other calls.** The drain's and the hack's `SetNoiz`
  calls now log sound 94 on both sides.
- **The other suites.** All six field UI suites pass.
- **`gate_hack_shots`.** It adds `0-noise`, the first frame of the
  hack's noise.

**Still unknown:**
- **The pixels are not compared** with the game's: the pieces are the
  stream's, checked per primitive there, but no capture of the field's
  noise was taken.
- **`ccGameOverNoise`** (the game-over screen's own `ccNoiz`, gcmn
  0x00517078) is not ported.
- **Town 4.** Which town is `game.town` 4 is not looked at here.
- **The band's copy past the frame.** The VRAM a band copies past the
  draw buffer (a band that starts low) reads as 0 in the port, as in the
  stream.
