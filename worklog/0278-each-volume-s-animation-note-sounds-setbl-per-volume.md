---
number: 278
title: Each volume's animation-note sounds: setbl per volume, Outbreak's 21 party tables
date: 2026-09-28
area: audio, volumes
files: crates/piney-gen/src/placement/sound.rs, crates/piney-audio/src/setbl.rs, crates/piney-audio/src/se3d.rs
---

# 278. Each volume's animation-note sounds: setbl per volume, Outbreak's 21 party tables

The animation notes' sounds (`ccSeSetParamSPC`, `ccSeSetParamEnemy`,
`ccSeSetParamInu`) read setbl.cpp's tables. The port read Infection's for
every volume. Two things kept it that way:
- The generator gave up where `spc0SeData` has no carried name (OUT and
  QUA).
- piney-audio's `setbl` read `Volume::Inf` into global statics.

Now each volume's own tables are used.

## Where the data starts

`spc0SeData` is `spcSeTbl[0]`, and it is the lowest row pointer on all
four volumes:

| volume | `spc0SeData` | `spcSeTbl` | `enemySeTbl` | `inuSeData` |
| --- | --- | --- | --- | --- |
| INF | gcmn 0x00639560 | 0x00639d40 | 0x0063a850 | 0x0063a8a0 |
| MUT | 0x00669dc0 | 0x0066a5a0 | 0x0066b0b0 | 0x0066b100 |
| OUT | 0x006687b0 (no name) | 0x006690e0 | 0x00669c50 | 0x00669ca0 |
| QUA | 0x0055fb30 (no name) | 0x00560460 | 0x00560fd0 | 0x00561020 |

`placement::sound::setbl` takes `spcSeTbl[0]` where the name is missing.

## Outbreak's 21 party tables

The carried size of `spcSeTbl` is Infection's, 76 bytes (19 pointers).
On OUT and QUA the words at the table are 21 pointers (ids 0-20, each a
14-row table) and then 3 NULL words up to the first enemy table at
0x00669140. On INF and MUT, id 18 is NULL, and one padding word follows.

The generator now runs each pointer table to the next object in setbl's
data (a row a pointer names, the other table, or `inuSeData`). Padding is
read as NULL, as the game would read it for an id past the table:

| volume | `spcSeTbl` | `enemySeTbl` |
| --- | --- | --- |
| INF, MUT | 20 (18 tables, 2 NULL) | 20 (19 tables, 1 NULL) |
| OUT, QUA | 24 (21 tables, 3 NULL) | 20 (19 tables, 1 NULL) |

## What differs

MUT's rows are INF's, shifted to its own base. OUT's rows differ from
INF's:
- row 11 of each of the 18 shared party tables (Kite's is 106 on INF,
  105 on OUT);
- the enemy tables, which start 44 rows later, behind the three new
  party tables, and change size (`cateUndead` starts 52 rows later);
- `inuSeData`, which is row 670 instead of 616.

QUA's rows are OUT's. Of 677 rows, only 21 differ: the pointer tables'
words (rows 294-304 and 660-669), which hold addresses from their own
base.

## The readers

`plans/volumes.md` noted that the readers changed size. The instructions
of `ccSeOnPCStep`, `seHitAttr`, `ccSeSetParamSPC`, `ccSeSetParamPC` and
`ccSeSetParamInu` on INF, MUT and OUT were compared with addresses and
labels stripped. They differ only in branch merging, delay-slot `nop`s and
stack slots:
- the footstep notes by id are the same (15: 63, 10: 61, 6: 64,
  2/3/8: 58, 1: 62, 0: 60, else 61);
- `seHitAttr`'s constants are the same (INF's 50 against OUT's).

The port's readers stay as they were and take the volume:
- `se3d::spc_note`, `enemy_note` and `inu_note` take a `Volume`;
- `setbl::rows(v)`, `spc(v)`, `enemy(v)`, `inu(v)` and `base(v)` read
  each volume's `setbl.bin` once.

The callers pass the world's volume. `DATA_VERSION` is 14, since OUT's
and QUA's builds had no `setbl.bin`. `notes_are_the_volumes_own` holds
row 11 and ids 18 and 20 on INF and OUT.

**Still unknown:**
- Who ids 18-20 are on Outbreak and Quarantine (`ccCharBaseParam.id`
  rows added after Mutation), and what their tables' sounds are.
- How the enemy tables changed row by row on OUT: only the offsets
  were compared.
