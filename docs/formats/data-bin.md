---
title: The DATA.BIN archive
status: solid
volumes: all
covers: INF DATA/DATA.BIN, INF SLUS_202.67:0x002fb750 categoryFDTbl, 0x002fb7a0 cateCDOfsTbl, 0x001651d0 searchFname, 0x00164540 ccFileListLoad, 0x001639c0 ccAddFileList, 0x00383ec0 directCCSTbl
worklog: 3, 5, 20, 242
---

# The DATA.BIN archive

The game's main asset archive: 1,023 gzip members laid end to end on sector
boundaries, each holding one [CCS scene file](ccs.md). The archive has no
directory of its own; the index is compiled into the executable.

## Layout

All little-endian.

```
DATA.BIN
  member    starts on a 2048-byte boundary
    gzip    RFC 1952, one member: 1f 8b 08, FLG 0x08 (FNAME only), MTIME,
            XFL, OS, NUL-terminated name, deflate data, CRC32, ISIZE
    padding zero bytes to the next 2048-byte boundary

index, in SLUS_202.67 .data
  categoryFDTbl   0x002fb750   u32[20]   pointer to each category's table
  cateCDOfsTbl    0x002fb7a0   u64[20]   byte offset of each category in DATA.BIN

category table    44-byte records, ended by one all-zero record
  char[32]  name      "XASC00.CCS", upper case, NUL-padded
  u32       offset    bytes from the category's start; a multiple of 2048
  u32       csize     exact length of the gzip member, padding excluded
  u32       usize     inflated length
```

A record's sector on the disc:

```
lba = lsn(\DATA\DATA.BIN;1) + cateCDOfsTbl[category] / 2048 + offset / 2048
```

where `lsn` is what `sceCdSearchFile` returns (12463 on the Infection disc).
The code rounds `offset` up to a sector - `(offset + 2047) >> 11` - which makes
no difference, since every offset is aligned.

## Categories

| # | table | records | base sector |
| ---: | --- | ---: | ---: |
| 0 | `cmnCCSTbl` | 6 | 0 |
| 1 | `gcmnCCSTbl` | 8 | 31 |
| 2 | `demoCCSTbl` | 5 | 345 |
| 3 | `desktopCCSTbl` | 136 | 486 |
| 4 | `toppageCCSTbl` | 1 | 9175 |
| 5 | `menuCCSTbl` | 4 | 9431 |
| 6 | `effectCCSTbl` | 12 | 9540 |
| 7 | `equipCCSTbl` | 350 | 9889 |
| 8 | `skillCCSTbl` | 15 | 12356 |
| 9 | `spcCCSTbl` | 18 | 12371 |
| 10 | `pcCCSTbl` | 37 | 14442 |
| 11 | `npcCCSTbl` | 19 | 16458 |
| 12 | `gimmickCCSTbl` | 37 | 17028 |
| 13 | `enemyCCSTbl` | 133 | 17498 |
| 14 | `bossCCSTbl` | 13 | 24683 |
| 15 | `townCCSTbl` | 14 | 27160 |
| 16 | `fieldCCSTbl` | 147 | 33033 |
| 17 | `dungeonCCSTbl` | 18 | 48430 |
| 18 | `eventCCSTbl` | 50 | 58576 |
| 19 | `directCCSTbl` | run time | - |

"Base sector" is `cateCDOfsTbl[k] / 2048`, relative to the archive. Categories
0 to 18 hold all 1,023 members, in archive order, with no gaps: each category
starts where the previous one's last member ends, and the last record ends at
the last byte of the file.

## Fields

`name` - the gzip FNAME is the same name in lower case with `.cmp` in place
of `.CCS` (`XASC00.CCS` / `xasc00.cmp`). The game only ever uses the record
name.

`csize` - reading exactly `csize` bytes from the member's start reaches the
end of the gzip stream in all 1,023 records.

`usize` - equals the gzip `ISIZE` in all 1,023. `ccFileListLoad` (at
`0x00164ea8`) chooses how to read a file by it:
- **not 0**: `FileReadTh` reads the member into one ring buffer, and
  `UngzipTh` inflates it from there into a second ring, which the scene
  loader (`ccStream`) reads.
- **0**: `FileReadTh` reads straight into the loader's ring, and `UngzipTh`
  gets no `ccUngzip`, so it never calls `Decode`. The file's bytes reach the
  loader as they are: a file stored without compression.

Only category 19's entries have a `usize` of 0 (below).

## Lookup

`searchFname__FP8FILELIST` (`0x001651d0`) gets a file-list entry
(`s32 category`, then the name at `+4`), takes `categoryFDTbl[category]`, and
compares names with `strcmp`, 44 bytes at a time, until a match or a record
named `"NULL"`. On `"NULL"` it draws `FILE NONE` and hangs. Category 19
compares only the part of the name before the `.`, upper-cased.

## Category 19

Category 19 bypasses the archive. `directCCSTbl` (`0x00383ec0`, 16 records) is
in `.bss`; `ccInitFileList__Fv` fills every slot's name with `"NULL"`, and
`ccAddFileList` / `ccAddFileListOne` add entries. Each new entry gets the
name as the list gives it, `offset` 0, `csize` -1 and `usize` 0.
`ccFileListLoad` opens `cdrom0:\DATA\` + name with `sceOpen`, takes the size
from `sceLseek(fd, 0, SEEK_END)` (the `csize` of -1), and reads the file
without inflating it (the `usize` of 0). So category 19 loads a loose,
uncompressed `.CCS` from `DATA`.

No volume uses it. A file list is an array of `{s32 category, char *name}`
pairs, ended by a negative category or a null name, and all of them were
checked on the four volumes:
- **Lists in the executables and overlays.** A scan of every word pair in
  the executable and the four overlays finds no category 19 followed by a
  pointer to a name.
- **Lists made at run time.** Their categories are constants
  (`spcTempFileList` 7 and 9, `GateHackOutFileList` 18, `ccPuccigusoStart`'s
  11, `RequestCCS`'s calls 0 and 15 to 18,
  `ccAddRequestFileListInu`'s 11), or come from rows of `enemyTbl`,
  `gimmickTbl` and `npcTbl`, which are in the scan above.
- **The constant 19.** None of the eight or nine places per volume that
  store it builds a file list: they are in `sceMcRename`, `ccVoiceRequest`,
  two event opcodes (a gimmick's entry type), `ccAddRequestFileListSpc` (a
  `gateHackingOutID`), `MailList_control::PreviewRes` and
  `BOOK::PadControl03`.

No disc has a loose `.CCS` in `DATA` either. Category 19 loads a scene file
that is not in the archive, and the retail game never asks it to.

## Notes

The static tables end with a record whose name is empty, not `"NULL"`, so for
categories 0 to 18 the `"NULL"` stop never fires: a name missing from its table
would be searched for past the end, through the following categories' records.

`categoryPathTbl` (`0x002fb700`) names one archive per category -
`cdrom0:\DATA\cmn.bin` through `cdrom0:\DATA\event.bin` - none of which is on
the disc. Its only reader uses it for category 19, whose entry is
`cdrom0:\DATA\`.

The game inflates with its own `ccUngzip` (`Decode__8ccUngzipFUi`,
`0x00156ba0`), whose methods carry the function names of gzip 1.2's
`inflate.c`. A reader thread (`FileReadTh`) feeds it through `ccRingBufferTh`
ring buffers, and it runs on its own thread (`UngzipTh`).

## Counts

Measured over INF's whole file:

| | |
| --- | ---: |
| file size | 136,665,088 |
| members | 1,023 |
| `1f 8b 08` elsewhere than a member start | 6, inside compressed data |
| padding after a member | 1 to 2,047 bytes |
| compressed bytes | 135,599,928 |
| inflated bytes | 338,646,324 |
| MTIME range | 973651006 (2000-11-08) to 1030068730 (2002-08-23) |

`tools/fdtbl.py check` verifies every record against the archive. It passes
on all four volumes - 1,023, 1,031, 1,072 and 1,072 records - through the
table names `piney-gen syms` carries to the stripped executables; the other
volumes' category counts are on [the four discs](../disc/volumes.md).

## Unknown

None.
