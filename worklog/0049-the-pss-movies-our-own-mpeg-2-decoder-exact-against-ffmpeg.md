---
number: 49
title: The PSS movies: our own MPEG-2 decoder, exact against ffmpeg, played by the title
date: 2026-09-23
area: video, audio, test
files: crates/piney-mpeg, crates/piney-data/src/pss.rs, crates/piney-audio/src/lib.rs, crates/piney-game/src/session.rs, docs/formats/pss.md
---

# 49. The PSS movies: our own MPEG-2 decoder, exact against ffmpeg, played by the title

Until now the title's four movies counted as played ([[43]]), and the user
asked why PSS never worked. A helper agent wrote the format and a decoder.
I added the movie sound to `piney-audio` and the player to the runtime.
The reference is [the PSS page](../docs/formats/pss.md).

## What the movies are

- **Where they play.** Only the title plays PSS: `LogoMain` calls
  `ccDecodeMpeg` four times (`LOGO_B`, `LOGO_C`, `LOGO_H`, `OPENING`). The
  desktop Audio screen's "movies" are not PSS. `SimplePlayStream` starts
  `ccThExecuteStream`, an in-engine `STREAM/*.BIN` cutscene, which is the
  stream port's job.
- **The container.** 16 KB MPEG-2 packs of four PES packets. Video is
  stream 0xE0. Audio is private stream 1 with the sub-header
  `FF A0 00 ch`, which is also what the game's demultiplexer keys on
  (`_strmap`).
- **The video.** 640 x 448 at 30 pictures a second, Main profile, all
  progressive frame pictures. The logos have 146, 180 and 151 pictures and
  `OPENING` has 2,771.
- **The sound.** Only `OPENING` has sound: 48 kHz stereo 16-bit PCM in the
  SPU2's own block layout (256 left samples, then 256 right). The game
  sends it untouched to core 0's sound-data input at volume 0x7fff, not
  scaled by the BGM volume. It has no ADPCM.

## The decoder

`piney-mpeg` is ours, written from the MPEG-2 standard:
- the bitstream reader and VLC tables;
- a "simple IDCT" in ffmpeg's integer arithmetic;
- the IPU's colour conversion, whose alpha is 0x80.

`Movie::next_frame` gives each picture as the frame `setImageTag` draws: a
PSMCT32 upload and one sprite over the screen.

## Playing them

`Demo` asks with `Request::Movie` and waits for the movie before its next
step. The session plays it as `ccDecodeMpeg` does:
- the first picture and the sound start together;
- each picture shows for two frames (the game paces by vblank and ignores
  the time stamps);
- OK, cancel or START (the save's `assignPADok` and `assignPADcancel`,
  and 0x800) stop it once twelve pictures are decoded. The title then
  skips the remaining logos and the opening.

`Audio::movie_stream` and `movie_stream_stop` are `audioDecStart` and
`audioDecReset`.

The texture cache in `piney-gs` evicted nothing until 4,096 textures were
cached. For the opening that meant about 3 GB of pictures; the fix is
cee9693.

## Checked

- **The decoder.** `piney-mpeg`'s `ffmpeg` tests decode all four files both
  ways: 3,248 of 3,248 pictures are exact, the same count in the same
  order. The integer IDCT passes IEEE 1180.
- **Against another IDCT.** Against ffmpeg's `-idct int`, the worst PSNR is
  57.96 dB (`OPENING`). That is a guess at how far the IPU's undocumented
  IDCT could be.
- **A test race.** The two ffmpeg tests failed at random pictures of
  `OPENING` because both wrote the same temporary copy of the file while
  running in parallel. Each comparison now writes its own copy.
- **`movies_play` in `piney-game`.** From power-on, the three logos take
  exactly 2 x (146 + 180 + 151) frames of 640 x 448 pictures, all silent.
  `OPENING` opens with its sound; its PCM is silent for the first 2.57 s,
  and it is heard in more than 75 of frames 160-240. START then stops it,
  and the title goes on.
- **The other session tests.** They now skip the logos with START as a
  player would.
- **Shots.** A GPU shot at frame 100 shows the Bandai logo, and one at
  frame 2600 a scene of the opening.

**Still unknown:**
- **The IPU's IDCT.** Its exact arithmetic, and so how close our pictures
  are to the console's.
- **After the last audio block.** What the SPU2 plays until
  `audioDecReset`: the IOP buffer loops, and whether that tail is silence or
  a replay is not checked.
- **The first picture.** How many frames the console takes before it.
- **Other volumes.** Mutation, Outbreak and Quarantine's movies are not
  checked.
