---
number: 85
title: The opening arc's cutscenes as the game presents them: subtitles, the music around the streams, and streams 5 and 10's effect tasks
date: 2026-09-24
area: video, audio, ui, test
files: crates/piney-stream/src/subtitle.rs, crates/piney-stream/src/event.rs, crates/piney-stream/src/effect.rs, crates/piney-stream/src/table.rs, crates/piney-audio/src/stream.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/desktop.rs, tools/test_stream_rs.py, docs/engine/stream.md, docs/engine/sound.md
---

# 85. The opening arc's cutscenes as the game presents them: subtitles, the music around the streams, and streams 5 and 10's effect tasks

[[52]] ported the in-engine streams as picture and sound. The arc's
cutscenes are:
- stream 3, Aura chased by Skeith;
- streams 4-6, Orca's fall;
- stream 7, BlackRose;
- streams 8-12, the church and the first Data Drain.

The game puts three more things around them, which an agent has now ported
and checked:
- the subtitle window;
- the music before, during and after;
- the effect tasks of streams 5 and 10.

## Subtitles

- **Where they live.** The event's `stream` is `ccEventStream(num, 1)`
  (0x001b5670). When a stream has a table in `evStrMsgTbl` (0x003112f0,
  or 0x00311510 in Parody Mode), it makes its own layer at 242 and a
  fresh `ccMessage`. It replaces the global `ccMsg` and leaves it
  pointing at the freed object afterwards.
- **What runs each frame.** `ccGetStreamDemoMsg` gives the note's line
  (or -1 to close). `Change` shows it when Movie Text (`strWinMode`) is on
  or the line has flag 0x800. `Disp` draws the window.
- **What it does not do.** It never reads the pad. At the end it leaves
  without a last `Disp`, so the window vanishes instead of fading.
- **When a line shows.** The scene's task (24) handles the notes before
  the event task (32), so a line shows on its note's frame. A skip's
  close arrives one frame later (the stream's task is 33).
- **The drawing.** The window is the desktop's `ccMessage` port.

## The music around the streams

- **The functions.** `ccSndStreamCtrl` (0x0017d190) sets the music before
  and after a stream, on the area's bank. `ccSndStreamBGM` (0x0017cb20)
  plays at event 4's notes, walking `strSndTbl` (0x0030b950) for streams
  3, 8 and 58.
- **In the arc.**
  - Stream 3 stops the dungeon's music at frame 385 and restarts it fading
    in after.
  - In the church (play type 3), stream 8's note at frame 366 stops
    sequence 1.
  - Stream 12's end starts sequence 1.
- **A mistake the sweep caught.** Streams 112-119 fade to half and back,
  not out and then half.

## Streams 5 and 10's effect tasks

- **The shared part.** Both keep `Func_str0001`'s `ccStrEffectCtrl`, which
  the port now shares as `effect::Ctrl`, with a cue map per task.
- **Stream 10, the first drain** (`Func_str0300`): noise, one-frame white
  flashes, feedbacks at two scales, and `ccBufferReverce`, which inverts
  the frame.
- **Stream 5, Skeith and Orca** (`Func_str0120`):
  - feedbacks that hold then fade;
  - fades to and from white and black;
  - the view's `divZ` set to 500;
  - eight hit marks at the cue objects. Cues 610-614 read two entries past
    the three-entry `hitRot0120a`, into jump-table words the EE's float
    unit takes as 0.

## In the runtime

- **On the desktop.** piney-game plays the scripts' streams with subtitles
  and music (`StreamPlayer::event`). Through `story:4`, streams 4 and 5
  show their subtitles on the desktop's setup screen at 30 frames a
  second.
- **In The World.** The API for hosting streams there is at the top of
  piney-game's stream.rs.

## Checked

Against the game in eemu:
- **Subtitles.** `test_stream_rs.py subtitles` runs `ccEventStream` over
  the scenes' notes. The Rust side plays each stream for real:
  - 19 runs: every stream with a table, Movie Text off, Parody Mode, and
    two skips mid-line;
  - 5,729 changing steps, all matching.
- **Music.** `test_stream_rs.py music`: 1,399 scenarios, command for
  command.
- **The effect tasks.** `test_stream_rs.py effects`, with every
  primitive and `rand` after every pass:
  - stream 10: 1,257 passes;
  - stream 5: 3,103 passes, including the marks and the `divZ` change.

  `str0001`'s fixture regenerates byte for byte.
- **Re-run in a clean worktree.** The stream, desktop, demo and sound
  suites pass, with the workspace's tests, clippy, fmt and the docs check.
- **Shots.** Stream 3's "What the...!", stream 5's noise and feedback,
  stream 7's BlackRose, stream 10's inversion, and streams 4 and 5 with
  subtitles on the desktop.

**Still unknown:**
- **Streams in The World** (the town and field hosts) are not hosted yet.
  Streams 3 and 7-12 need that.
- **Not drawn.**
  - Stream 5's eight hit marks: `effHitMarkStr` is a `ccEffect` that
    piney-effect lacks.
  - The view's clip at `divZ`, which is not modelled.
- **Not ported.** The arc's other nine effect tasks (`Func_str0090`, 0110,
  0130, 0150, 0240, 0250, 0301, 0305, 0350).
- **When the call sees the end.** The exact frame `ccEventStream`'s loop
  sees the stream end is not pinned down. Every window has closed by then.
