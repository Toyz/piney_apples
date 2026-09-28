---
number: 88
title: The window's scaler: sharp bilinear, so thin strokes keep one weight at any size
date: 2026-09-24
area: render, test
files: crates/piney-gs/src/lib.rs
---

# 88. The window's scaler: sharp bilinear, so thin strokes keep one weight at any size

The user reported that the desktop's font looked buggy: the icon labels
(THE WORLD, MAILER, NEWS, ACCESSORY, AUDIO, DATA) had letters of uneven
weight.

## Not the game's drawing

- **The native frame is clean.** A 512 x 448 shot of the desktop, enlarged
  by whole pixels, shows every label crisp and even.
- **The game samples nearest too.** The labels are `ccSprite` cells.
  `ccSprite::SetTag` (0x0015c0c0) sends the TEX1 that `SetTex` (0x0015c2f0)
  copied from the texture's own data (+0x10 of the texture object) with MXL
  cleared. The desktop's textures set only MXL there, so the game samples
  them nearest, as [desktop](../docs/engine/desktop.md) records and the
  port does.

## The window

- **What the presenter did.** It stretched the frame to 4:3, letterboxed,
  with a plain bilinear sampler. The scale is not a whole number (1.5625
  wide for a 512-wide frame in an 800-wide window).
- **Why strokes went uneven.** A 1-pixel stroke came out crisp where it
  lined up with the window's pixels and smeared over two greyer pixels
  where it did not.
- **The fix.** The present shader now scales sharp bilinear. Each frame
  pixel is a solid block, and only the one-window-pixel seam between two
  blocks is blended.
  - At a whole-number scale it samples texel centres exactly.
  - At scale 1 it is plain bilinear.
  - The scale per axis comes from the derivatives of the texel
    coordinate, so no uniform is needed.

This is a presentation choice, not the game's: a television blurs the
analog picture instead.

## Checked

- **A new test on the GPU.** `present_keeps_thin_lines_whole` draws
  twelve 1-pixel lines, 7 frame pixels apart, and presents them at
  1600 x 1200 (3.125 wide). Every line keeps a pure-white peak.
- **The test catches the old behaviour.** With the plain sampler put back,
  the line at x 100 peaks at 219, and the test fails.
- **The earlier test still passes.** It is now written on a shared
  `presented` read-back helper and checks the letterbox and the colour
  values.

**Still unknown:**
- **Not seen in the user's window yet.** The fix is checked by the test,
  not by a shot of the desktop at the user's window size.
- **No option to switch.** There is no setting for the plain (softer)
  filter, or for a television-like one.
