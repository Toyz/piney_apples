---
number: 245
title: The later openings, libmpeg's frame base, and a line of size 0
date: 2026-09-27
area: format
files: docs/formats/pss.md, docs/formats/voice.md, crates/piney-mpeg/tests/ffmpeg.rs, crates/piney-audio/src/seword.rs
---

# 245. The later openings, libmpeg's frame base, and a line of size 0

Asked for: more of the docs' open questions. This entry covers the movie
and voice pages. The questions left there need a console: the IPU's
IDCT, the SPU2's timing, the disc's latency.

## The movies

- **The other volumes.** The logos are the same bytes on all four discs.
  - `OPENING2`-`4` are made as Infection's `OPENING` is. Only Mutation's
    has a higher bit rate (8.19 against 5.12 Mbit/s).
  - Every audio stream has the same `SShd`.
  - The players:
    - Mutation's is Infection's code;
    - Outbreak's and Quarantine's are that code built by another compiler.

    Compared function by function, they make the same calls with the same
    constants and field offsets.

  The new test `later_openings_match_ffmpeg_exactly` finds all 2,733,
  2,630 and 2,715 pictures equal to ffmpeg's, byte for byte.
- **`frameCount`'s base.** It is libmpeg's private +0xac. `sceMpegCreate`
  and `Reset` zero it. `_markOutput` sets it to the picture counter when the
  first reference picture is shown, which is during the second decode. So
  it is 1, and `frameCount` is the display index of the last picture out,
  as the page and the port (`Movie::skippable`) already had it.
- **`SShd.type`** is 1 in all four openings and read by nothing: moved to
  Notes.

## The voice

- **`CCSTRM_PCM`'s `type`, `bitNum`, `trackType`.** `Decode_Pcm` reads the
  structure onto its stack and uses only `dataNum` and `dataSize`.
- **A read of a negative size.** A line under 16 KiB asks for one at its
  first interrupt. The IOP's reply, read from `IOPRP243.IMG`'s 2.4.3
  modules:
  - SEWORDS's files are opened for stream reads, so the size reaches
    `CDVDMAN`'s blocking stream read as `size >> 11` sectors.
  - Its copy compares sizes unsigned, so it copies every buffered sector
    past the 8 KiB raw half and waits for more; the count never matches.
  - A normal read would have returned -22.
- **Who asks.** Every skill-word table, in both languages and on all four
  discs, ends with a `{0, 0}` row. `skillVoicePlay` only skips `ofs == -1`.
  - Kite reaches his with skill 295, `Para Repth` (base 123).
  - `Para Repth` is the Recovery Drink's skill (`itemTblR` 23).
  - Quarantine gives Kite five at a new game, and trades them.

  Quarantine's code along the way is the same. The port's
  `seword::Word` returns nothing for a read of 0 or fewer bytes, so there
  the line plays its 16 KiB preload and ends; its comment now says so.

**Still unknown:** Whether that Recovery Drink really stops channel 0 on
a console; it follows from the code but was not run. Other characters'
bases may reach their tables' last rows through other skills; only Kite's
was traced.
