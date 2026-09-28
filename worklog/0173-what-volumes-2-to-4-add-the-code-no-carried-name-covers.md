---
number: 173
title: "What volumes 2 to 4 add: the code no carried name covers"
date: 2026-09-26
area: disc
files: tools/voldiff.py, docs/disc/volumes.md, GAPS.md
---

# 173. What volumes 2 to 4 add: the code no carried name covers

With Infection's combat, dungeons and story playable, this is the first
look at what the other three volumes need beyond it. The later
executables are stripped, but `tools/xfer.py` carried Infection's names
onto them (worklogs 20-25). Code that no carried name covers is either
new or compiled too differently to be matched.
`docs/disc/volumes.md`, "What the later volumes' code adds", has the
tables.

**The tool.** `tools/voldiff.py gaps` lists, per section, how much text
the names cover and the stretches between named functions. `look` prints
the strings a stretch materialises (the lui/addiu pairs the disassembler
tracks; the stripped files have no relocations) and the named functions
it calls. On Infection only main's fonts show, which is the check that the
measure means something.

**What it found:**

- **Mutation** is Infection plus little: 3.5% of `GCMN.PRG` and about
  100 KB of main have no name.
  - The new streams' effects (`str0710e` and on: raster noise, buffer
    sampling).
  - The event engine's new instructions, about 13 KB.
  - The Grunty race (`PG_RACE`, `town06`).
  - A new menu.
- **Outbreak and Quarantine** show a fifth of `GCMN.PRG` unnamed. Part of
  that is Infection's code under the new compiler settings: the enemy
  races show up there, `ccEnemyL`'s type 3 among them. The new parts:
  - Kyvia's fights, 155 KB.
  - The Root Towns 02-05 and the party of 21, 69 KB.
  - The other event areas, 45 KB.
  - Quarantine's staff roll and hacking logos.

`GAPS.md`'s "After Infection" is now that list, with the gaps it already
named (type-32 rooms, warps, the save conversion).

**Still unknown:** Only stretches of 8 KB or more were looked at, 68 in
Outbreak's `GCMN.PRG` alone. How much of Outbreak's and Quarantine's
unnamed code is Infection's, recompiled, was not measured. That would take
matching by shape instead of by bytes. Nothing was ported here.
