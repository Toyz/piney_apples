---
number: 5
title: The DATA.BIN index lives in the executable
date: 2026-09-22
area: format, decomp, tooling
files: tools/fdtbl.py, docs/formats/data-bin.md
resolves: 3
---

# 5. The DATA.BIN index lives in the executable

[[3]] found no directory in `DATA.BIN` and no table of sector numbers in the
executable. There is a table; it stores sizes and category-relative offsets,
and the category bases are 64-bit - which is why a search for 32-bit sector
numbers missed it.

## How a name becomes a sector

`searchFname__FP8FILELIST` (`INF SLUS_202.67:0x001651d0`) takes a file-list
entry whose first word is a category and whose name starts at `+4`. It indexes
`categoryFDTbl` (`0x002fb750`, 20 pointers) by the category, walks that
category's table in 44-byte steps, and `strcmp`s each record's name against
the wanted one. It stops on a match or on a record named `"NULL"`; on `"NULL"`
it draws `FILE NONE` on screen and loops on `ccBreathThread` forever.
Category 19 compares only the part before the `.`, upper-cased. The record:

```
char[32]  name       "XASC00.CCS"
u32       offset     bytes from the start of the category, sector aligned
u32       csize      the gzip member's exact length, padding excluded
u32       usize      inflated length; 0 sends ccFileListLoad down a
                     different path (0x00164ea8) - presumably stored
```

`ccFileListLoad__FPv` (`0x00164540`) then computes the sector, at
`0x00164e00`-`0x00164e58`:

```
sector = ccCd->data_bin.lsn                  lwu 12(ccCd)
       + cateCDOfsTbl[category] >> 11        u64, byte offset of the category
       + (record.offset + 2047) >> 11
```

and hands it to `StStart__6ccCdvdFUi`. `ccCd+0x0c` is a `sceCdlFILE` filled by
`ccCdInit__Fv` (`0x00159720`), which calls `sceCdSearchFile` on
`\DATA\DATA.BIN;1` and retries until it succeeds. The same function looks up
`\DATA\SNDDATA.BIN;1` at `ccCd+0x30`, `\STREAM\STRCMN.BIN;1` at `+0x54`,
`\STREAM\STR1.BIN;1` at `+0x78`, `\STREAM\STRCMNE.BIN;1` at `+0x108` and
`\STREAM\STR1E.BIN;1` at `+0x12c`.

`cateCDOfsTbl` (`0x002fb7a0`) is 20 u64s: 0, 63488, 706560, 995328, ... - in
sectors 0, 31, 345, 486, 9175, 9431, 9540, 9889, 12356, 12371, 14442, 16458,
17028, 17498, 24683, 27160, 33033, 48430, 58576, and 0 for category 19.

## The categories

The table symbols name them. Record counts are without the terminator:

| # | table | records | | # | table | records |
| ---: | --- | ---: | --- | ---: | --- | ---: |
| 0 | `cmnCCSTbl` | 6 | | 10 | `pcCCSTbl` | 37 |
| 1 | `gcmnCCSTbl` | 8 | | 11 | `npcCCSTbl` | 19 |
| 2 | `demoCCSTbl` | 5 | | 12 | `gimmickCCSTbl` | 37 |
| 3 | `desktopCCSTbl` | 136 | | 13 | `enemyCCSTbl` | 133 |
| 4 | `toppageCCSTbl` | 1 | | 14 | `bossCCSTbl` | 13 |
| 5 | `menuCCSTbl` | 4 | | 15 | `townCCSTbl` | 14 |
| 6 | `effectCCSTbl` | 12 | | 16 | `fieldCCSTbl` | 147 |
| 7 | `equipCCSTbl` | 350 | | 17 | `dungeonCCSTbl` | 18 |
| 8 | `skillCCSTbl` | 15 | | 18 | `eventCCSTbl` | 50 |
| 9 | `spcCCSTbl` | 18 | | 19 | `directCCSTbl` | 0 (in .bss) |

Categories 0 to 18 hold 1,023 records - every member of the archive, in
archive order.

## The terminator is not "NULL"

In the executable every table ends with one all-zero record, not one named
`"NULL"`. The string `"NULL"` (`@785`, `0x0034b618`) is written only by
`ccInitFileList__Fv` (`0x00163440`), and only into two run-time tables: all 16
slots of `directCCSTbl`, where it marks a free slot, and the `name` of all 128
entries of `sceneFileList` (`0x00384180`, 40 bytes each: `s32 category` set to
-1, `char[32] name`, `s16` at `+0x24` set to 0, `s16` at `+0x26` set to -1).
So the `"NULL"` stop in `searchFname` only ever fires for category 19. For
categories 0 to 18, a name that is not in its table would walk past the
nameless terminator into the next category's records and beyond; the game
relies on never asking for one.

## Category 19 is the loose-file path

For category 19, `ccFileListLoad` does not use the archive at all. It builds a
path from `categoryPathTbl` (`0x002fb700`) - which for 19 is `cdrom0:\DATA\` -
plus the name, opens it with `sceOpen`, and if the record's `csize` is -1 asks
`sceLseek(fd, 0, SEEK_END)` for the size. `directCCSTbl` (`0x00383ec0`, 16
records) is in `.bss` and is filled by `ccInitFileList` / `ccAddFileList`.
The other nineteen entries of `categoryPathTbl` are `cdrom0:\DATA\cmn.bin`,
`gcmn.bin`, `demo.bin` ... `event.bin` - one archive per category, none of
which is on the disc. They are left over from before the categories were
concatenated into `DATA.BIN`, and the retail code only ever reads entry 19.

## Checked

`tools/fdtbl.py` reads the three tables through the symbol table and computes
every record's sector. `fdtbl.py check` then, for all 1,023 records: finds a
gzip member at the computed sector; inflates exactly `csize` bytes and reaches
the end of the gzip stream; gets `usize` bytes; finds the member's FNAME equal
to the record name with `.cmp` for `.CCS`. It also checks that each category
begins where the last ended and that the last record ends at the last byte of
the archive. Result: 1,023 records, 0 problems.

**Still unknown:** whether any requested name can miss its table; what fills
`directCCSTbl` and which files use category 19; the two `s16` fields of a
`sceneFileList` entry.
