---
number: 239
title: Each disc file by file, and the four discs' unknowns
date: 2026-09-27
area: disc
files: docs/disc/layout.md, docs/disc/mutation.md, docs/disc/outbreak.md, docs/disc/quarantine.md, docs/disc/volumes.md, docs/disc/unnamed-code.md, tools/voldiff.py, tools/iso.py
---

# 239. Each disc file by file, and the four discs' unknowns

Asked for: settle the four unknowns `docs/disc/volumes.md` listed, and
give each game its own disc page, as Infection has.

## The tools

- **`tools/iso.py sums IMAGE`**: every file's SHA-1, in disc order. Each
  disc page's listing names, file by file, the other discs that hold the
  same bytes.
- **`tools/voldiff.py report ELF`**: every stretch of code that no carried
  name covers, as markdown. Each row gives its place, the names either
  side, its function prologues, and its strings and named calls. `gaps`
  and `look` now share its code (`stretches`, `look_data`).

## The pages

- `docs/disc/mutation.md`, `outbreak.md` and `quarantine.md` follow
  `layout.md`:
  - the volume;
  - `SYSTEM.CNF`;
  - every file in disc order, with the discs that share its bytes;
  - what each file is;
  - what changed from the volume before.
- All four are packed without gaps: 377-1830439, 386-1859588 and
  390-2176898. All fit a DVD-5.
- `docs/disc/unnamed-code.md` holds the three reports: 423 stretches, 351
  of them starting a function, 17 all zero.

## The unknowns

- **`KFED.BIN` / `KFAED.BIN`.** Quarantine's opcode 169 (`0x001c52e8`):
  1. sets `game+0x7c`;
  2. reads both files into `$gp` -29188 and -29184;
  3. loads `ending.ccs`;
  4. runs gcmn `0x004f0a30`, which starts `STFROLL_VOL4` (the staff roll,
     thread `0x004f0950`) and waits for it;
  5. frees both.

  A scan of every word of the executable and the four overlays found:
  - four instructions using those `$gp` offsets, all the handler's (two
    stores, two loads for `ccFree`);
  - no absolute references and no stored pointer.

  The staff roll draws with `ccKanji`, on the `ef` fonts. So the kanji are
  loaded and never read.
- **`STRT.BIN` (Outbreak).** Six gzip `tmp.ccs` members:
  - `title1_st1t`, `title1_st1t2`, `title2_st1`: the title's `xdt_` models;
  - `str9999e`, `str9999`: `hack` / `coming` planes and rings;
  - `trial_v2st`: `trilog00`-`10`.

  They date from 2002-10-01 to 2003-03-13 (gzip times). No code opens the
  file: `ccCdInit`'s list has no `STRT`, no string contains it, and no
  stream table names those scenes. The live `title2_st1` is `STRCMNE`'s,
  at another size.
- **`VOICE2`.** The skill words (`spcVoiceData`). `SPC00`-`17` and `_E`
  are Infection's `VOICE/SPC*` byte for byte. `SPC18`-`20` are `charTbl`
  rows 18-20, Tsukasa, Subaru and Sora: in Japanese, two clips each.
  Outbreak's disc lacks their English files, though its table (shared with
  Quarantine) names them; Quarantine has them.
- **The CD modules.** They did not move into `IOPRP243.IMG`.
  - The image is the same on all four discs, built 2001-12-05. Its ROMDIR
    lists 15 modules, `CDVDMAN` and `CDVDFSV` among them (0x021a,
    `PsII... 2430`).
  - `ccCdInit` reboots the IOP with it, then loads ten modules by name
    from `MODULES`. No executable names `cdvdman` or `cdvdfsv`.
  - Infection's and Mutation's `MODULES/CDVD*.IRX` are 1.04
    (`PsIIcdvdman 134`), never loaded. Outbreak and Quarantine dropped
    them.
- **The stretches.** All listed (above).

Infection's page had two open questions and one stale entry:
- **`OUTSIDE.BIN`**: no string `OUTSIDE` in any volume's code.
- **Reads by sector**: every sector read comes from a name.
  - `sceCdSearchFile` has seven callers: `ccCdInit`'s six files and
    `ccMcard::MakeDir`'s `ICON.BIN`.
  - demo.prg's `strFileOpen` looks up the movies.
  - The rest opens by path: the overlays, the modules, and the IOP's voice
    files.
- **`ICON.BIN`** was already decoded in the save page (`iconBinTbl`).

Measured along the way:
- **Mutation's `STR1` and `STR1E`.** Infection's sizes. 19 members differ
  only in the gzip time stamp (packed again on 2002-10-23; the inflated
  bytes are equal). One scene changed: `str9102`.
- **Quarantine's `STR3E`.** 230 MB smaller than Outbreak's, the same 48
  scenes. 23 of them drop one of Outbreak's two Pcm tracks; the two setup
  chunks differ only in a padding byte (+0x07, per `CCSTRM_PCM`'s DWARF).
  Frames are unchanged.
- **Fidchell's voice group (19, `BOSSTALK`).** Only Outbreak's disc
  carries it, though Quarantine's tables name it (and `MIAE.BIN`).

`volumes.md` is now solid. Its code table has the current carry's
figures: Outbreak's gcmn is 15.4% unnamed, not 20.4%.

**Still unknown:** What `STRT.BIN`'s scenes look like; the port's stream
player plays only table streams, so it cannot show them yet. The names of
the functions in the unnamed stretches, which survive nowhere.
