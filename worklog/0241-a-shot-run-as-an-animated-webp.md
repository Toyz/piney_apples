---
number: 241
title: A shot run as an animated WebP
date: 2026-09-27
area: tooling
files: crates/piney-game/src/webp.rs, crates/piney-game/src/main.rs, crates/piney-game/Cargo.toml
---

# 241. A shot run as an animated WebP

Asked for: a way to share what the port draws, such as STRT.BIN's scenes
(worklog 240), as animated WebP.

`--shot` takes new options:
- `--webp OUT.webp` writes the run's frames as one animated WebP, looping.
- `--webp-from F` skips the frames before F.
- `--webp-every N` keeps every Nth frame.
- `--webp-quality Q` sets the lossy quality (default 80).

Each kept frame lasts the vertical blanks the game spent up to the next
one (the mode's frame rate times N) at 59.94 a second. So the file runs at
the game's own speed. `webp::Recorder` encodes with `webp-animation`
(libwebp's animation encoder), reading each frame back from the GS after
it is drawn.

The first encode had blocky checkers over the Mutation demo title's logo.
ImageMagick's decoder, which goes through libwebp, showed them too, so
they were in the file. With no key frames after the first, libwebp encodes
each frame as a blended difference. It marks the pixels it takes for
unchanged transparent, and over lossy frames the error builds up into
blocks. With every frame a key frame (`kmax` 1) the frames match the PNG
shots. One loop of STRT stream 2 (453 frames, 15.1 s) is 8.2 MB, where the
diffed file was 5.8 MB.

The file's frame times were read back from its ANMF chunks: 15,115 ms in
all, 453 game frames at rate 2. libwebp merges identical consecutive
frames into longer ones. The test `frames_timed_on_the_game_clock` checks
three frames' ANMF durations against the game clock.

Tried from the command line as `--webp out.webp --shot`, it wrote
nothing. A `--shot` with no file after it started no headless run, so the
window came up and ignored `--webp`. Now:
- `--webp` alone, or a bare `--shot`, runs headless;
- `--shot OUT.webp` records the WebP instead of writing PNG bytes under
  that name;
- with no `--frames`, a mode that ends by itself (`Mode::ends`, the loose
  streams) runs until it has played through once (`Mode::finished`), rather
  than 60 frames. The frame that would start it over is not recorded.

The written files are printed with their full paths.

**Still unknown:** Nothing records from the window yet; a key to start and
stop a recording while playing would share live play.
