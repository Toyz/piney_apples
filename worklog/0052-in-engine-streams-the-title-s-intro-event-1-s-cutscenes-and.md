---
number: 52
title: In-engine streams: the title's intro, event 1's cutscenes and the Audio screen's movies, bit for bit
date: 2026-09-23
area: engine, render, audio, test
files: crates/piney-stream, crates/piney-game/src/stream.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/session.rs, crates/piney-gs/src/assets.rs, tools/test_stream_rs.py, docs/engine/stream.md
---

# 52. In-engine streams: the title's intro, event 1's cutscenes and the Audio screen's movies, bit for bit

The last thing the title and a new game skipped was the in-engine streams:
cutscenes the engine itself plays from `STREAM/*.BIN` (`ccRequestLoadStream`),
not PSS movies ([[49]]). A helper agent ported them as `piney-stream`,
and I put them in the runtime. The reference is
[the stream page](../docs/engine/stream.md).

## What a stream is

- **Where they are called.** Three callers:
  - the title's `PlayOpeningStream` (stream 0, the intro);
  - the scripts' `stream` op (`ccEventStream`: event 1 plays streams 2 and
    106 before its first lines, at frame rates 2 and 1);
  - the desktop Audio screen's movies (`SimplePlayStream`, at rate 2, with
    every desktop task asleep).
- **Holding the caller.** The call plays the whole stream before it
  returns, so each caller is held meanwhile.
- **Inside.** A stream is a list of scenes. Each is a CCS-like file with
  per-frame records (objects, camera, lights, materials, notes and PCM).
- **Sound.** The stream owns SEWORDS channel 0 from `ccPcmSound::Open` to
  `Close`. Its PCM is 1,024-byte SPU2 blocks at 48 kHz.
- **Skip.** `WaitEnd` decides it from the file's flags: 0x80 never, 0x40 on
  START, anything else on cancel. On the title after the desktop has run
  (`DESKTOP_FLG`), cancel always skips.
- **This disc.** 35 streams play: 0-20, 47, 106-119 and 133. The rest live
  on the later volumes' discs.

## In the runtime

- **The player.** `stream::StreamPlayer` wraps a stream for all three
  callers. As it starts it stops the voice and channel 0's input. It then
  queues each block of samples (`Audio::stream_pcm`, appended after the
  last) and closes the channel at the end or on a skip.
- **The event VM.** It polls `busy(Wait::Stream)` once a frame while the op
  waits. The bridge steps the stream there, as the game's event task runs
  it inside the call. The stream's frame replaces the setup's black, or the
  desktop's frame.
- **Frame rate.** `frame_rate` now sets the setup's frame rate.
- **The overlay archive.** The frames name the stream's own files, which
  are not in `DATA.BIN`. `piney-gs` `Assets` gained an overlay archive,
  looked up first. The window loop sets it from `Mode::archive` each frame,
  and changing it drops the file textures read so far.
- **The title.** It waits on stream 0 as on a movie. `DESKTOP_FLG` is kept
  across resets.

## Checked

- **The port against the game.** The stream agent's `test_stream_rs.py`
  runs the game's code in eemu and writes a fixture, which
  `piney-stream`'s Rust test compares:
  - the table and file lists of all 134 streams, in both languages;
  - 180 cases of the skip rule;
  - the draw list of streams 0, 106 and 2;
  - every frame of streams 0 (410) and 106 (397), and the first 60 of 2:
    every object drawn and its matrix, bit for bit, the camera to 5e-7 and
    the light matrix to 6e-8.
- **The fixture.** I regenerated it from the game's code in a clean
  worktree (104 s). It is byte-identical to the one committed.
- **A bug the agent fixed.** Only a scene read whole into memory gets a
  `frameOffsetTbl`. The port had given every scene one, so stream 2's note
  frames came out one too high.
- **The runtime.**
  - `movies_play` now goes on past OPENING into stream 0, which draws on
    more than 150 of its first 200 frames.
  - `setup_lines_are_voiced` plays event 1's streams 2 and 106 for 120
    frames each, hears their sound in more than 100 of those frames, then
    skips them with cancel and goes on to the voiced lines.
  - The session and desktop tests skip the streams with cancel and START,
    as a player would. `piney-game` has 8 tests, all passing; the workspace
    is clippy and fmt clean.
- **Shots.** GPU shots show stream 0's title logo and stream 2 mid-scene.

**Still unknown:**
- **Not drawn.** Effect objects (particles, the `F_Morpher` blend,
  shadows) and the streams' own effect tasks (`StreamDemoFuncTbl`, such as
  str0001's noise and fades). Streams 108-111, which are effects only,
  draw nothing.
- **Subtitles.** `evStrMsgTbl`'s lines under a stream are not drawn; the
  requests carry the line index only.
- **Load time.** The frames the disc read and decoding take are not
  modelled.
- **The intro's end flash.** The title's white flash over the last 20
  frames of stream 0 is drawn after the stream, not over it.
- **Around a stream.** The sequenced music (`ccSndStreamCtrl`) is not
  driven. Streams 0, 2 and 106 ask for none.
- **The Audio screen's movies.** They play through the same player but
  have no test of their own.
