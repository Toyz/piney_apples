---
number: 175
title: "Skeith's stall and the drain's dither: stream files read ahead, vertex alpha as the GS's integer"
date: 2026-09-26
area: render
files: crates/piney-gs/src/lib.rs, crates/piney-gs/src/assets.rs, crates/piney-stream/tests/drain_render.rs, crates/piney-stream/examples/stream_time.rs, crates/piney-game/src/session.rs, docs/engine/render.md, docs/engine/stream.md, BUGS.md
---

# 175. Skeith's stall and the drain's dither: stream files read ahead, vertex alpha as the GS's integer

Two reports from playing event 4's streams in the window. The game lagged
badly as Skeith's animation began. The drain on Orca looked like its
textures z-fought.

**The stall.** `event_4_stream_times` (ignored) plays event 4's streams
4-6 through the session as the window does. Each step, its sound (the
events handed to a headless `Audio`, a frame of the driver, a 30th of a
second rendered) and its GPU draw are timed. Stream steps take about 1 ms
and draws about 5 ms. The exceptions:

- Each stream's load at its first frame: 78 ms for stream 4, 199 for 5,
  48 for 6. Stream 5's `str0120` is 31 MB, 83 MB inflated. Inflating it
  takes 107 ms and reading its scene 68 ms. It falls at the cut between
  two streams. flate2's zlib-rs backend saved only 8%, so it was not
  taken.
- One draw of 138-151 ms at stream 5's frame 1834. It made no new
  pipeline and only one texture. The draw commands of frames 1833 and
  1834 (`stream_time --dump`) differ by one new model, the book's bubble,
  and it is the first draw from `str0120` rather than `str0120e`. The
  renderer reads a file the first time a draw names it, which here means
  inflating and parsing all 83 MB mid-scene.

`piney_gs::Assets` now starts a thread when it is given a new overlay (a
stream's archive). The thread reads every member in turn and sends each
file as it is read. A lookup takes whatever has arrived, and reads the
file itself only if the thread has not got there yet. Files are `Arc` now,
not `Rc`. Stream 5's worst draw after the first frame is now 6.7 ms.

**The dither.** The pictures of stream 5's frames 2690-2697 showed the
figure in the drain's cylinder with its hat broken into a fine dither of
itself and what lay behind. It was not the texture: `TEX_ckitbodh2` reads
clean.

Leaving commands out of one frame's picture (`stream_time --drop`)
narrowed it down:

- Only draw 34 matters: a gold ring, `str0120e` object 0x20c, on layer 3.
- The figure is on layer 4 and draws after it.
- 8x the depth precision changed nothing, so it was not the depth.

The ring is fading in at transparency 0.02. Its vertex alpha,
`trunc(0x80 * t)`, is 2, and its AREF, `trunc(aref * t)`
(`ccSetMaterialPacket`, `docs/engine/desktop.md`), is 2 as well. So on
the GS every pixel passes and writes Z. The ring draws nearly nothing and
erases the figure behind it, as a band sweeping down before the figure
turns to particles.

The shader interpolated that constant 2 as a float. At some pixels it
landed a hair under 2, floored to 1, and failed. The shader now snaps the
interpolated vertex alpha to the GS's integer (`floor(a + 1/32)`) before
the modulate and the test. The erasure is clean now. The cylinder's dotted
floor, the same threshold, is smooth.

`tests/drain_render.rs` (piney-stream) renders frame 2694 and measures the
hat's place: 76 with the old shader, 24 with the new, and it asserts
under 40.

**`stream_time`** gained `--shots DIR FROM TO EVERY`, `--dump FRAME` (the
model draws with their edits), `--drop I,J` (commands left out of the
dumped frame), the textures and pipelines each draw made
(`Gs::counts`), and the load split into the disc, the stream and its
effects.

**Still unknown:** Whether the window showed the lag worse than this
headless run was not measured in the window itself; the audio was
simulated, not played. The drain was not compared with a picture of the
game, and it is inferred from the rules that the game erases the figure
too. The streams' own load was left as it is; the game streams its files
from the disc as they play.
