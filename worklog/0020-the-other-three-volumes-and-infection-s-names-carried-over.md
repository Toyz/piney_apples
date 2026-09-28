---
number: 20
title: The other three volumes, and Infection's names carried over to them
date: 2026-09-22
area: volumes, tooling, disc
files: tools/xfer.py, tools/image.py, tools/fdtbl.py, docs/disc/volumes.md, docs/formats/data-bin.md
---

# 20. The other three volumes, and Infection's names carried over to them

Up to now every claim in this log was about Infection. This entry opens the
other three discs, finds that they are the same engine in the same container
formats, and carries Infection's 35,383 symbols over to their stripped
executables so that the tools work on them by name. The reference is
[the four discs](../docs/disc/volumes.md).

## The discs

The boot ELFs are `MUT SLUS_205.62`, `OUT SLUS_205.63` and `QUA SLUS_205.64`.
All three are stripped: no symbol table, no DWARF, no relocations, and
section names blanked. The section headers remain, in Infection's order -
main, the four overlays, heap - so `tools/image.py` now maps a stripped
executable's sections by position. Each carries
`MW MIPS C Compiler (2.4.1.01)`. The overlay window moves with the size of
main: `0x00413800` in MUT, `0x0040f200` in OUT, `0x00301b00` in QUA
(Infection `0x00400800`).

The CCS rules of [[7]] and [[10]] walk every member of every volume's
`DATA.BIN` with no desync. The file lists differ in the expected ways: more
`STREAM` archives each volume, a `VOICE2` directory from OUT on, OUT and QUA
dropping the Japanese stream archives and two CDVD IRX modules. QUA adds
`DATA/KFED.BIN` and `DATA/KFAED.BIN`.

QUA's main section is 0x179480 bytes against OUT's 0x286b80. Infection keeps
its bitmap fonts between the end of its code and its data - `kf14x16`,
`kf20x20`, `kfa20x20`, `kfa14x16`, `ef12x20`, `ef8x16`, 1,148,800 bytes -
and MUT and OUT have a stretch of about that size in the same place, but QUA
has 36,868 bytes there. `KFED.BIN` (688,000) and `KFAED.BIN` (35,200) are
exactly the sizes of `kf20x20` and `kfa20x20`, yet share almost no bytes
with them - 0.2% equal. (I concluded here that the encoding or order
changed; [[23]] shows the format and glyph order are the same and the
glyphs re-shaded - my 200-byte "glyph block" test was wrong, since a glyph
is a run of sheet rows, not a contiguous block.)

## Carrying the names over

`tools/xfer.py` pairs functions by their code with every address-bearing
field masked (jal targets, lui halves and the low halves paired with them,
`$gp` offsets), then names callees and globals from the paired bodies. On
MUT, 3,518 of Infection's 4,998 functions of 6 or more instructions match
exactly.

On OUT only 1,029 do. The same compiler generated different code:
- In `GetAmbient__9ccDrawEnvFv` (`INF SLUS_202.67:0x00105750`,
  `OUT SLUS_205.63:0x001059a0`), Infection calls `fptoui` three times. OUT
  inlines each conversion as a compare, `cvt.w.s` and fix-up.
- Functions with no floating point differ as well. `mwLoadOverlay` (168
  bytes in both) has its saved registers permuted (`$s0`/`$s1`/`$s2`).
  `SearchFree__10ccHeapFreeFUi` has branch delay slots left as nops that
  Infection filled, with the instructions moved to other places.
Of the 1,208 main functions the order pass names in OUT, 911 contain no
floating-point instruction and no `fptoui`/`fptosi` call. So the scheduling
and register allocation changed between MUT and OUT, not only the float
handling. Which option changed is not known.

The link order did not change: in MUT's first pass the exact matches run in
Infection's order 1,659 of 1,659 in main, 1,533 of 1,534 in `gcmn`, 160 of
160 in `desktop`. So a fourth pass, `order`:
- It takes the paired functions whose order agrees in both volumes as
  anchors.
- In each gap between anchors, it aligns the remaining source functions to
  the destination's candidate entry points by a shape score: the difflib
  ratio of the two opcode sequences, with nops dropped.
- Unrelated functions of similar size score 0.22 at the median and 0.58 at
  the 99th percentile. True pairs from single-function gaps score 1.0 at the
  median, and `GetAmbient` scores 0.28.
- A gap holding as many functions on each side is paired one for one if
  every pair scores at least 0.3. Any other gap keeps only pairs scoring
  0.65 or more, in a monotone alignment.

The same rescheduling broke the data pass: `searchFname` in OUT
(`0x001644d0`) builds `categoryFDTbl`, but not inside a run of 8 identical
words. For pairs that are not identical, the data and call passes now also
align on the shape tokens. They take runs of 8 or more, and runs of 4 or
more inside a gap already bounded by such runs. A name is still taken only
when every vote agrees and both sides use the same opcode. That found
`cateCDOfsTbl` at `OUT SLUS_205.63:0x00308750` from a 7-instruction run in
`ccFileListLoad`, next to an `lwu` that had moved.

Two smaller additions:
- A function's address taken in code (a thread entry, a callback) names it
  the way a jal does.
- An entry point is also assumed after a `j` whose delay slot is followed by
  padding, which is how a tail-calling function such as `ccMallocB` ends.

The totals:

| | exact | call | order | data | total |
| --- | ---: | ---: | ---: | ---: | ---: |
| MUT | 3,518 | 604 | 1,214 | 4,292 | 9,628 |
| OUT | 1,029 | 500 | 3,331 | 3,147 | 8,007 |
| QUA | 1,026 | 499 | 3,311 | 3,131 | 7,967 |

Each goes to a sidecar file, e.g. `work/outbreak/disc/SLUS_205.63.syms`
(under `work/`, so never committed), which `tools/image.py` loads whenever an
executable has no symbol table of its own.

## How far to trust them

Two checks.

First, `tools/fdtbl.py` now reads the index through the names alone. It finds
`categoryFDTbl` and `cateCDOfsTbl`, and walks each table to its all-zero
terminator rather than by Infection's sizes. `fdtbl.py check` then passes
every record of every volume against that volume's own archive: MUT 1,031,
OUT 1,072, QUA 1,072 records, 0 problems. The tables sit at `+0x50` from each
other on every disc, as in Infection.

Second, a round trip: carry a volume's names back onto Infection, whose
symbol table is the answer.
- From MUT: 9,512 names, 7 wrong (0.07%).
- From OUT: 7,927 names, 18 wrong (0.23%).
  - One is the `call` name `_exit`, at an address with no symbol.
  - Five are `order` pairs landing on a neighbouring function:
    `ccFileListLoad` on `FileReadTh`, `ccThEntryCtrlDelete` on
    `ccThEntryCtrl`, `PersonalMenuTS` on `ItemBoxMenuT`, `ccPlayerStart` on
    `ccThPlayer`, and one with no symbol at its address.
  - Eleven are `data` names: on an address with no symbol of its own (inside
    a larger object), or on a different object (`bookMagicCircleMsg` on
    `bookMagicCircleAllOpenMsg`, `dialogVoice` on `cmndTarget`).
- `_kill_r` and `__sigtramp` have identical bodies, and one is always
  mis-named.

A round trip can hide an error that happens the same way in both directions,
so these rates are a floor on how often a name is wrong, not a proof.

## The archives across volumes

- MUT drops `pc/CBUZ.CCS` and adds 9 records. OUT adds 41 more, all
  desktop.
- OUT and QUA list the same 1,072 records. Only five members differ: four
  desktop images and `boss/X11.CCS`.
- Of the 1,022 records INF and MUT share, 992 inflate to the same size.

**Still unknown:** what QUA uses `KFED.BIN` and `KFAED.BIN` for (see [[23]]
for their format); what `OUT STREAM/STRT.BIN` and the `VOICE2`
banks hold; whether OUT and QUA's missing `CDVDFSV.IRX`/`CDVDMAN.IRX` moved
into `IOPRP243.IMG`; the functions new in volumes 2 to 4, which have no name
to carry; which exact compiler options changed between MUT and OUT; whether
anything but the index (the CCS chunk rules, event scripts, save layout, area
generator) changed in the later volumes - only the containers have been
checked.
