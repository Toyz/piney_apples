---
number: 324
title: "File-list entries: deleteCnt is a reference count, addFlag is always -1"
date: 2026-10-01
area: format, engine, volumes
files: docs/formats/data-bin.md
resolves: 5
---

# 324. File-list entries: deleteCnt is a reference count, addFlag is always -1

[[5]] left three questions: whether a requested name can miss its category
table, what fills `directCCSTbl` (answered by [[242]]), and the two `s16`
fields of a file-list entry. The DWARF names the entry `FILELIST`
(`filelib.cpp`, 0x28 bytes): `s32 category`, `char name[32]`, `short
deleteCnt` (+0x24), `short addFlag` (+0x26). All addresses below are INF
SLUS_202.67.

## deleteCnt

It counts how many requests hold the file:
- `ccInitFileList` sets it to 0;
- `ccAddFileList`, `ccAddFileListOne` and `ccAddFileListName` increment it
  (`0x00163a38` and the same place in the other two);
- `fileConflictCheck` increments the existing entry when the same file is
  asked for again (`0x0016408c`);
- `ccFileListDeleteOne` decrements it (`0x00163664`). At 0 it destroys the
  file's `ccStream` (`ccsLoad[i]`), decrements `directNum` when the category
  is 19, and renames the entry `"NULL"`.

## addFlag

Written -1 by `ccInitFileList` and by each of the three adds (`0x00163a50`,
`0x00163c58`, `0x00163e4c`). `ccFileListLoad` and `ccFileListDeleteOne` only
copy it. Its one reader, `ccFileExistCheck`, tests it `== 1` at
`0x001641a4`, so that branch is dead.

## Names that miss their table

The three adds copy the name, append `".ccs"` (`0x0034b620`) when
`strchr(name, '.')` finds no dot, and `strupr` it (`0x00163a80`,
`0x00163aa8`). `searchFname` then compares with `strcmp`, so case in a
static list does not matter.

A scan of every static file list was run on the four volumes: a run of
`{s32 category 0..19, char *name}` pairs whose name starts a string and ends
in `.CCS`, closed by a negative category or a null pointer. It covered the
executable and the gcmn, desktop, demo and toppage overlays. After upper-casing,
every row's name is in its own category's table:

| | main | gcmn | desktop | demo | toppage | total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| INF | 9 | 5 | 122 | 5 | 1 | 142 |
| MUT | 9 | 8 | 129 | 5 | 1 | 152 |
| OUT | 9 | 8 | 169 | 5 | 1 | 192 |
| QUA | 10 | 8 | 169 | 5 | 1 | 193 |

The scan's one other hit, INF `GCMN.PRG:0x005e275c` (category 0 naming
`CW2HSW01_4.CCS` of category 7), is a false match. The word before it is
zero padding and the pointer is `ehkBrParam`'s first word, `0x00300018`.
Without the terminator rule, the extra hits are other structures: the sound
bank table (`SE*.CCS`), the field table (`FIELD_*.CCS`), `enemyTbl`, and the
category tables themselves. Those load under their own constant categories.

The port finds files by member name, not through the category tables, so a
miss would not show there.

**Still unknown:** whether a name built at run time (`sprintf` area and
model names, names read from `enemyTbl`, `gimmickTbl`, `npcTbl`) can miss its
category's table, which would walk past the table's empty terminator. That
needs each builder's name pattern checked against the tables.
