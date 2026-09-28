---
number: 16
title: The memory card save format
date: 2026-09-22
area: save, format, decomp
files: docs/formats/save.md
---

# 16. The memory card save format

A .hack save is the game's whole `ccSaveData` object written to the memory
card as it sits in memory, plus a small per-slot index. Because the DWARF
names every member of `ccSaveData` ([[6]]), the save format is known field by
field without ever having looked at a real save file. The layout is on
[the save page](../docs/formats/save.md).

## Where it goes

`ccMcard` (`mcard.cpp`) builds every path the same way: a directory from
`mcDirName[volumeNum - 1]` (`INF SLUS_202.67:0x00306be0`), a `/`, and a file
from `mcFname` (`0x00306d40`). The four directories are
`BASLUS-20267DOTHACK` - Infection's own product code, SLUS-20267 - and three
placeholders, `BASLUS-00001DOTHACK`, `-00002` and `-00003`, which read as
slots reserved for the other three volumes before their product codes
existed. `volumeNum` is 1 in the executable's data. The slot files are
`dhdata01` to `dhdata13` (`mcFname[1..13]`). The slot index is written by
`ccMcard::SaveSys` (`0x001664b0`) to the directory name repeated - it
`strcpy`s and then `strcat`s the same `mcDirName` entry - so Infection's is
`/BASLUS-20267DOTHACK/BASLUS-20267DOTHACK`, the usual PS2 practice of naming
the main file after its directory. `mcFname[0]`, `BISLPS-00000HUCKER`, is not
used by `SaveSys`; what uses it is not known.

`ccMcard::MakeDir` (`0x001659b0`) writes `/icon.sys` (it builds a `PS2D`
header) and the icon files, taking the icons from `\DATA\ICON.BIN;1` through
`iconBinTbl` (`0x00306c00`, twelve `{offset, size}` pairs) and `iconName`
(`icon10.ico`, `icon11.ico`, `icon12.ico`, `icon20.ico` ... `icon42.ico`).
`ICON.BIN` is therefore twelve PS2 icons at 90,112-byte strides, three per
volume.

## What is written

`ccSaveSys::MainProccess` (`0x00171c20`, a 9 KB state machine) saves in two
writes (at `0x00173e10` and `0x00173e4c`):

1. `ccMcard::SaveSys(port, 0, info, 336)` - `ccSaveSys.info[12]`, twelve
   28-byte `ccSaveDataInfo` slot records: `status`, `level`, `clearFlag`,
   `parodyFlag`, `char name[18]`, `u16 sum`, `int playtime`.
2. `ccMcard::DataWrite(port, 0, slot, saveData, 0x8530)` - the whole
   `ccSaveData` object, 34,096 bytes, verbatim.

Before the second write the slot record is refreshed from the save: status 1,
level and name from `spcParam[0]` (the player character, `+0x7488`), play time
from `+0x8400`, clear and parody flags from `+0x842a` and `+0x842b`, and `sum`.

`sum` is the 16-bit sum of all 0x8530 bytes of the save data (the loop at
`0x001724cc`); a load compares it against the slot record at `0x001724f8` and
rejects the file on a mismatch. `ccSaveSys::CheckRightInfo` (`0x001716e0`)
validates all twelve records: an empty slot (status 0) must be all zeros, a
used one (status 1) needs level 0-99, `clearFlag` 0-4, `parodyFlag` 0-1 and a
play time below 0x0CDFE5C4 - 215,999,940 sixtieths of a second, 999:59:59.

## Carrying over between volumes

`ccSaveSys` keeps a second array, `infoPrev[12]` (`+0x150`), with
`LoadInfoPrevReq` and `LoadDataPrevReq` beside the ordinary loads: the
machinery for reading the previous volume's saves. `clearFlag` runs 0 to 4,
one per volume. In Infection, the first volume, those paths have nothing to
read; how Mutation uses them is a question for that disc.
`ccSaveData::ConvGame` (`0x001752a0`, `sdmng.cpp`) rebuilds party parameters
from `charTbl` (in the demo overlay) and then calls `LoadGame` and
`InitTradeItem`; no relocation in main or any overlay calls it, so in this
build it may be dead code.

## What the save holds

98 members, among them: both player names (`plName`, `plRealName`), item
lists for 18 characters, trade lists, skills, growth, the mail list and its
order, web news and bulletin board read flags, 160 event entries, the Chaos
Gate lists and records, the keyword list (`wordList[15]`), 512 64-bit event
flags (`eventFlag`, `+0x54f8` - the flags `ccEvent::Execute` sets), area bans
and protected areas, Grunty and dog counters, Data Drain counts, per-enemy kill
counts and kill areas for 313 enemies, all 18 characters' parameters, play
time, the pad assignment, volumes, voice and parody flags, and 250 reserved
bytes.

**Still unknown:** what `mcFname[0]` (`BISLPS-00000HUCKER`) is for; the `icon.sys` fields the game fills in; which of the
twelve icons each volume uses for list, copy and delete; the meaning of most
event-flag bits; whether `ConvGame` is reachable; how the later volumes read
Infection's saves.
