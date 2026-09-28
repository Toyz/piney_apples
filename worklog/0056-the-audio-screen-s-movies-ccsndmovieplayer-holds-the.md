---
number: 56
title: The Audio screen's movies: ccSndMoviePlayer holds the desktop music around the stream
date: 2026-09-23
area: audio, test
files: crates/piney-game/src/desktop.rs
---

# 56. The Audio screen's movies: ccSndMoviePlayer holds the desktop music around the stream

[[52]] played the desktop Audio screen's movies through the stream player,
but no test covered them. The runtime also ignored the `bgm` the request
carries for `ccSndMoviePlayer`, so the desktop music went on under the
movie.

## `ccSndMoviePlayer(mode, bgm)` (0x00180b30)

**The slot.** It works on one sequence slot: slot 1 when the desktop music
is 27 or 7, slot 0 otherwise. The slot must exist (`ccSnd` +100 counts the
loaded sequences).

**Mode 0**, before the movie, runs only if that slot is playing
(`sqStatus` 1 or 3). It is `ccSqStop`:
- the sequencer off (command `0x20` + its MIDI port);
- its fade off;
- `sqStatus` 0.

Either way it sets `ccSnd` +98 to 1.

**Mode 1**, after the movie, runs unless that slot is playing, and is
`ccSqPlay`:
- the synthesizer port's volume, the sequence's volume times the BGM volume
  over 256 (port 0 takes the SE volume instead);
- the sequencer located to tick 0 and played (command `0x110` + port);
- `sqStatus` 1.

`piney-audio`'s `sq_stop` and `sq_play` already do both. So the runtime now
sends `SqStop(slot)` when the movie starts and `SqPlay(slot)` when it ends.

## Checked

`audio_movie_plays_as_a_stream` (`piney-game`) runs on a save with movies 1
and 2 unlocked (`dtStrList`) and `clearFlag` 1. It goes AUDIO, Movie,
Movie 01, then checks:
- the movie starts as a stream, with `SqStop(0)` and frame rate 2;
- it draws in more than 30 of its first 60 frames;
- cancel and START end it, and `SqPlay(0)` follows;
- the frame rate is back to 1 and the desktop runs again.

`piney-game` has 9 tests, all passing; clippy and fmt are clean.

**Still unknown:**
- **`ccSnd` +98.** What the flag mode 0 sets does, and whether mode 1
  clears it.
- **The volume detail.** The port-0 branch of the volume (the SE volume,
  sign-extended from 24 bits) is not modelled separately; `sq_play` covers
  ports 1-3.
