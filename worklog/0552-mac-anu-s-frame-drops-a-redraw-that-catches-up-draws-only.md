---
number: 552
title: Mac Anu's frame drops: a redraw that catches up draws only the frames it shows (#77)
date: 2026-11-02
area: render, engine, test
files: crates/piney-game/src/catch_up.rs, crates/piney-game/src/main.rs, crates/piney-draw/src/lib.rs, crates/piney-gs/src/lib.rs, crates/piney-game/src/session/tests/mac_anu_frames.rs
started: 2026-11-02T00:13:29-07:00
took: 26m
---

# 552. Mac Anu's frame drops: a redraw that catches up draws only the frames it shows (#77)

Issue #77 reports frame drops "from time to time" near Mac Anu's bridge and weapon shop. No pad log came with it. The measurement was taken first, on a release build, headless. `session::tests::mac_anu_frames::mac_anu_frame_times` is ignored and is kept as the trace. It starts a new game on INF, logs in to Mac Anu and walks Kite three times round every merchant and back. That is 5,023 frames past the first 200. Each frame's step, its sound (every event routed into a headless `Audio`) and its draw are timed, with the draw's CPU encode timed apart from the read-back.

## What a town frame costs

| part | median | p99 | max |
| --- | --- | --- | --- |
| `Session::step` | 1.84 ms | 2.4-3.5 ms | 3.8-5.6 ms |
| sound (handle, frame, render) | 0.28 ms | 0.40-0.54 ms | 1.0-1.9 ms |
| draw (`Gs::render` + `read_back`) | 4.7-5.0 ms | 6.3-8.5 ms | 9-13 ms |
| of it the encode (`Gs::render`) | 4.5 ms | | |

Binned by 1000-unit square, the encode follows the vertex count. Every view carries 145,000 to 265,000 vertices, put through on the CPU (`convert::mmat`). The most are near the town's centre: 259,080 at (0, 0) and 264,516 at (0, 1000), with an encode averaging 4.4-4.8 ms. Nowhere is there a spike tied to a place. The larger maxima moved between runs, sometimes far (a 50 ms draw once), so they are this shared machine's load, not the game. Run-time texture uploads are 1-2 a frame everywhere. Reusing the vertex list's allocation from frame to frame changed nothing measurable (4.66 against 4.72 ms median) and was dropped.

The field runs at `FRAME_RATE` 2 (`ccSetupNewGame` 0x001688d0), so a game frame has two vertical blanks. The step and the draw land in the same redraw, so on a machine about 2.5 times slower than this one a town frame passes one display refresh (16.7 ms) at the densest views. That is the reported "from time to time".

## The cause it makes worse

`App::tick` in `main.rs` steps every frame that is due, up to `MAX_CATCH_UP` 4, and drew every one of them, "caught up or not", so that a frame sampling the one before it would see it. Only the redraw's last frame is ever presented. A redraw that fell behind therefore paid the draw, about 70% of a town frame's cost, once per frame it caught up: two frames cost 2 x (1.9 + 4.7) = 13.2 ms here instead of 2 x 1.9 + 4.7 = 8.5 ms, and four cost 26.4 ms instead of 12.3 ms. On a slower machine that turns a single late frame into a run of them.

## The fix

`catch_up::CatchUp` holds the last frame stepped together with the overlay it was stepped under. When the next frame is stepped, the held one is drawn first only if the new frame samples the previous picture. That is the new `Frame::reads_previous`: a prim textured `TexRef::PreviousFrame`, the same test `Gs::render` now uses to decide when to copy the last picture. The held frame is drawn at the end of the redraw, and before the quit prompt draws its own frame. Every frame that is shown, or that another frame reads, is drawn exactly as before. Steps, events and sound are untouched, so the game's state does not change. `catch_up::tests::a_redraw_draws_the_frames_it_shows_or_that_are_read` holds the rule. The GS's `frame_buffer_textures` test still holds the previous-frame picture.

The town's own cost is not reduced. That needs the model vertices put through on the GPU, which is the render split, not a bug fix.

**Still unknown:** whether the reporter's machine passes a refresh at the bridge only through catch-up, or also on its first frame there. No timings came with the report, and the port does not record frame times in a pad log, so only the reporter running this build can say.
