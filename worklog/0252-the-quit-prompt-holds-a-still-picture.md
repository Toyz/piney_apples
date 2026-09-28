---
number: 252
title: The quit prompt holds a still picture
date: 2026-09-27
area: ui
files: crates/piney-game/src/quit.rs, crates/piney-game/src/main.rs
---

# 252. The quit prompt holds a still picture

Reported from play: opening the quit prompt (Escape) over Infection's
desktop wrecked the picture. The screen went white, with mirrored copies
of the prompt's text and stray lines in the top left, and a movie playing
under it broke the same way.

## The cause

The prompt (worklog 237) kept the game's last frame (its commands and
uploads) and drew it again every frame with the window added on top. The
desktop's frames sample the previous finished picture
(`TexRef::PreviousFrame`), and so do the movie and the stream effects
under it. Once the window was drawn, the previous picture held the
window. The next redraw of the same frame took it in again, and again,
frame after frame. The feedback grew until the screen was white, with the
text copied wherever the effect scaled and flipped the sample.

## The fix

When the prompt opens, the game's frame is rendered once and the GS's
finished picture read back (`Gs::read_back`, as the PNG shots do). The
prompt holds that still picture as a frame of its own:
- one PSMCT32 upload (id 0, its alpha set to 0x80);
- one sprite over the screen, DECAL without TCC, bilinear, UV from 0.5 to
  width + 0.5, which is how the movie player draws a picture.

The window's uploads are numbered after it, and no command samples the
previous picture any more.

When the prompt closes on Cancel, the held picture is drawn once alone.
The GS's previous picture is then the game's again, not the window's, so
the game's first frame back samples no trace of it. The headless path
(`--quit N`) does the same: it holds the picture after rendering the
frame the prompt opens on, and draws the backdrop as it closes.

## Checked

Headless on Infection's desktop set-up stream, with the prompt opened at
frame 300:
- at frame 420 the window stands over the still picture of frame 300,
  unchanged;
- a circle at frame 400 closes it at 410, and by frame 440 the stream is
  playing on with nothing of the window left.

**Still unknown:** nothing.
