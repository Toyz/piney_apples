---
number: 240
title: STRT.BIN played: the demo discs' screens
date: 2026-09-27
area: video
files: crates/piney-stream/src/load.rs, crates/piney-stream/src/lib.rs, crates/piney-game/src/loose.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/main.rs, docs/disc/outbreak.md
---

# 240. STRT.BIN played: the demo discs' screens

Asked for: a way to see Outbreak's `STREAM/STRT.BIN`, the six scenes no
code opens (worklog 239).

## The viewer

A stream normally comes from the stream tables: a `Def` names the archive
by its type, and each `Entry` gives the offset, size and kind. STRT.BIN is
in no table. Nothing new was needed to play it, only the records:
- **`piney_stream::load::scan`** reads an archive's records off the
  archive itself. A member starts on a sector with a gzip header, and its
  record takes:
  - the CCSF file's own name (after `CCSF` at +0x0c);
  - its bytes to the next member;
  - its inflated length.
- **`load::loose_streams`** groups the records into streams. A setup file
  (`...e`) goes with the scene of its stem; it gets `type` 0 and `flag` 16,
  as the tables' `strNNNNe` records have. Every other record is a last
  scene (`type` -1).

  The first try left `flag` at 0. The setup file then went into the scene
  list rather than the preloads, and the stream sat at frame 0 drawing
  nothing.
- **`load::read_in`** reads a member from any archive path.
- **`Stream::loose`** plays records from an archive path under stream 47's
  number (a `strdummy`: no music, no effect task). `from_files` gained the
  path.
- **`piney-game --mode loose[:PATH][:N]`** (`LooseMode`) plays stream N of
  `STREAM/STRT.BIN` or another archive, at the event streams' rate 2. It
  starts over when the stream ends, and START goes to the next. It prints
  the streams it found; `--shot` and `--every` work with it.

The test `outbreak_strt_scans_into_five_streams` checks the six names,
`str9999`'s inflated length, and the grouping into five streams.

## What they are

| stream | scenes | frames | shows |
| ---: | --- | ---: | --- |
| 0 | `title1_st1t` | 413 | `.hack//INFECTION DEMO` title (感染拡大), orange backdrop |
| 1 | `title1_st1t2` | 413 | the same, with NEWGAME / DATALOAD / OPTION |
| 2 | `title2_st1` | 453 | `.hack//MUTATION DEMO` title (悪性変異), blue, NEWGAME / DATALOAD / OPTION / CONVERT |
| 3 | `str9999e` + `str9999` | 553 | a teaser: rings, the Infection logo, "February 2003", `www.dothack.com`, Bandai, over Aura |
| 4 | `trial_v2st` | 453 | a selector over the party's art: `.hack//INFECTION Part 1` to `QUARANTINE Part 4` |

These are demo discs' title and menu screens and a release teaser, left
on the retail Outbreak disc. The pictures were checked in contact sheets
of `--every` shots.

**Still unknown:** Which demo disc each belonged to; the two titles' menus
are models in the scene, so how their selection worked is not in these
files.
