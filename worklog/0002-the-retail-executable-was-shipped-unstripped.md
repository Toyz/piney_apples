---
number: 2
title: The retail executable was shipped unstripped
date: 2026-09-22
area: decomp, engine, tooling
files: tools/elf.py, docs/engine/executable.md, docs/formats/prg.md
---

# 2. The retail executable was shipped unstripped

`INF SLUS_202.67` is 19,223,388 bytes, and only 2,590,848 of them are code and
data. The rest is everything a linker normally throws away: a full symbol
table, DWARF 1 debug information, line tables, and relocation sections for the
main program and all four overlays. Nobody stripped the release build. This
changes the shape of the whole project - we get original function names,
class names, parameter types and struct layouts instead of having to infer
them.

`tools/elf.py` was written to read it: header, program headers, section
headers, the symbol table, and reads by virtual address.

## What is in the file

The `.comment` section reads `MW MIPS C Compiler (2.4.1.01)` / `PlayStation2`:
Metrowerks CodeWarrior for PS2. The five `.mwcats` sections (type
`0xca2a82c2`) are CodeWarrior's own. The first DWARF compile unit is
`D:\usr\RpgUS\prog\source\crt0.s`, produced by
`Metrowerks MW GAS R5900 Assembler`, so the original source tree lived under
`D:\usr\RpgUS\prog\source\` - "RpgUS" being, presumably, the US localisation
branch.

| section | what | size |
| --- | --- | ---: |
| `main` | the resident program, VA 0x00100000 | 0x278880 |
| `gcmn.prg`, `demo.prg`, `desktop.prg`, `toppage.prg` | overlay sections, all VA 0x00400800, **size 0** - the bytes live in `DATA/*.PRG` | 0 |
| `heap` | VA 0x00730600, size 0 - marks where the heap starts | 0 |
| `.symtab` / `.strtab` | 35,383 symbols | 0x8a370 / 0x57ce4 |
| `.debug` | DWARF version 1 | 0xcb3be7 |
| `.line` | DWARF 1 line tables | 0x147ea0 |
| `.relmain` ... `.reltoppage.prg` | relocations, one section per code section | see below |

Symbols by section, total and functions:

| section | symbols | functions | relocations |
| --- | ---: | ---: | ---: |
| `main` | 10,424 | 2,241 | 35,752 |
| `gcmn.prg` | 18,424 | 2,764 | 73,990 |
| `desktop.prg` | 4,263 | 226 | 10,980 |
| `toppage.prg` | 1,843 | 49 | 2,731 |
| `demo.prg` | 396 | 131 | 2,380 |

Names are C++ mangled in the cfront style CodeWarrior uses: `ccMalloc__FUi`,
`Ctrl__5ccPadFv`, `__ct__13ccFrameBufferFv`, `OnThinkLaser__10ccBoss08_cFv`.
The engine's own classes are prefixed `cc` (CyberConnect): `ccHeap`, `ccPad`,
`ccStream`, `ccTexChunk`, `ccUngzip`, `ccMenuCtrl`, `ccAI`. Game-side classes
are not: `WORLD`, `FIELD`, `DUNGEON`, `EVENTAREA02`.

The relocations are the other windfall. Every `jal` carries an `R_MIPS_26`,
every address built with `lui`/`addiu` carries an `R_MIPS_HI16`/`R_MIPS_LO16`
pair, every `$gp`-relative access an `R_MIPS_GPREL16`, and every pointer in
data an `R_MIPS_32`, each naming its target symbol. That makes cross references
exact rather than heuristic, and tells pointers from integers in data. Ten
entries in `.relmain` use type 123, which is not a standard MIPS relocation;
not yet looked at.

## Memory map

From the program headers and the linker symbols `_gp`, `_fbss`, `_end`:

```
0x00100000  main .text/.data      filesz 0x278880   entry _start 0x00100008
0x00378880  main .bss (_fbss)     to 0x00400800
0x00400800  overlay window        one of gcmn / demo / desktop / toppage
0x00730600  heap (_end)           = 0x00400800 + gcmn's full footprint
```

`$gp` is 0x0037faf0 (`.reginfo`, and the symbol `_gp`). `_stack` and
`_heap_size` are 0xffffffff, which the PS2 runtime reads as "use the rest of
memory". The heap starts exactly where the largest overlay, gcmn, ends, so the
overlay window is sized for gcmn.

## The overlays

The ELF's overlay sections are empty; their bytes are in `DATA/GCMN.PRG`,
`DEMO.PRG`, `DESKTOP.PRG` and `TOPPAGE.PRG`, loaded by `mwLoadOverlay`
(`INF SLUS_202.67:0x001002c0`). Each starts with a 0x40-byte `MWo3` header,
and the header is loaded along with everything else, so file offset =
VA - 0x00400800 and code begins at 0x00400840. The three size fields were
identified against the symbol table rather than guessed: in all four overlays
the first data object sits exactly at `0x00400840 + text_size`, and the ELF
segment's `memsz` equals `0x40 + text + data + bss` exactly. The static
initialisers `__sinit_<file>.cpp` sit at the tail of `.data`, just before the
constructor table the header points at: gcmn's are `gamectrl.cpp`, `menu.cpp`,
`spc.cpp`; desktop's name the in-game operating system - `MediaTbl.cpp`,
`desktop.cpp`, `desktopMode.cpp`, `acces.cpp`, `audio.cpp`, `mailer.cpp` and
more. The layout is on [the overlay page](../docs/formats/prg.md).

## Other things that fell out

About 30 undefined symbols with value 0 are named `mc_*` - `mc_DrawTri`,
`mc_DrawShadow1`, `mc04b_DrawModel0`, `mc0_CheckBoundingBox`. The prefix and
the names suggest VU1 microcode entry labels; that is a guess until the VU
code is found.

**Still unknown:** relocation type 123; what the `mc_*` symbols resolve to;
how much of the DWARF survives intact (a reader is being written); what the
overlay id in the `MWo3` header is used for, if anything.
