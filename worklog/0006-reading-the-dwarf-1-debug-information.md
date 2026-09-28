---
number: 6
title: Reading the DWARF 1 debug information
date: 2026-09-22
area: tooling, decomp
files: tools/dwarf1.py, tools/demangle.py, tools/test_dwarf1.py, tools/test_demangle.py, docs/engine/source-tree.md
---

# 6. Reading the DWARF 1 debug information

`INF SLUS_202.67`'s `.debug` section ([[2]]) is DWARF version 1 as
Metrowerks wrote it, and it survives complete. `tools/dwarf1.py` reads it and
`tools/demangle.py` reads the cfront-style mangled names; between them we have
the original class layouts, function signatures, parameter and local names,
and the source file and line of every function.

## What was checked

- The whole section - 0xcb3be7 bytes - parses in about 1.2 s: 336,743 DIEs
  plus 39,462 null entries in 4,612 compile units. Every sibling chain stays
  inside its parent and ends exactly at the parent's end or a null entry; 0
  violations.
- All 4,396 functions with code have `low_pc`/`high_pc` equal to their ELF
  symbol's value and size, and the same name: 4,396 of 4,396. The 1,015 ELF
  functions with no DWARF are Sony's C libraries and assembler (896), the MSL
  runtime and compiler-generated constructors and inlines in main (66), and
  implicit constructors of nested structs in gcmn (30).
- Demangling: of 14,219 distinct symbol names, 4,740 are mangled and all 4,740
  demangle; the other 9,479 are genuinely unmangled (C and assembler names,
  3,055 `@NNN` literals, `.xxx` section names, `__sinit_<file>`, `$L` labels).
  For the 4,388 functions that have both a mangled signature and DWARF
  parameters, 4,226 agree exactly and the rest differ only in ways DWARF is
  known to lose - nested-class qualification (9), `const` on a pointee or
  `bool` recorded as `unsigned char` (27), unused unnamed parameters left out
  (126). No real disagreement.
- `tools/test_dwarf1.py` (7 tests) and `tools/test_demangle.py` (5 tests) hold
  this.

## How CodeWarrior laid it out

Each function has **its own compile unit** with its own `.line` table. Before a
translation unit's functions comes one compile unit with no address range that
holds that unit's types and globals, and none of 119,216 references crosses
from one translation unit to another - every unit carries its own copy of
every type it uses (`ccTexChunk` is described 163 times, identically).

The builds are **unity builds**: a function's compile unit is named after the
file the function is actually written in, so `system.cpp`'s translation unit
spans 13 files (`sysmem.cpp`, `syspad.cpp`, `syslayer.cpp`, ...), and inline
functions show up under their headers (`eventarea.h`, MSL's `vector`). 216
translation units, 244 distinct source paths.

**Overlays are described exactly.** A vendor tag, 0x4080, appears four times,
once per overlay, with attributes `0x2296` (the overlay id), `0x22a8` (the
name), a `low_pc`/`high_pc`, and an `AT_member` list naming its compile units:
gcmn 2,871 units in 140 translation units, demo 132 / 5, desktop 227 / 17,
toppage 53 / 4. That covers exactly the 3,117 units whose code is at
0x00400800 or above.

Other vendor attributes, all decoded: `0x2008` the mangled name (4,849);
`0x2013` the frame (register and size, 3,640); `0x2043` to `0x20c3` the stack
slots where `s0`-`s7` and `fp` are saved, and `0x20d3` to `0x2173` for `f20` to
`f30`; `0x2303` the list of globals a function uses (2,420). Two location
opcodes are outside DWARF 1: `0x80`, an FPU register (1,004), and `0x82`, a
value in a saved-register slot (13,446).

Oddities a reader must know:

- Every struct, class and union is `TAG_class_type` (0x02); there are no
  typedef, pointer_type or union_type DIEs, and only 10 enums. `structure_type`
  (0x13) appears only for member-less `@anonN` vtable types. Unions are
  recognised by every member being at offset 0 (214 found).
- Bit-field offsets count from the least significant bit - the GS register
  field `tGS_PMODE.EN1` is bit 0 - the opposite of the DWARF 1 specification.
- `long` is 8 bytes. Fundamental type `0x8208` is `unsigned long long`, and
  `0xa510` is the EE's 128-bit `u_long128`, which mangles as `1` followed by
  nothing (`SetDataAdrs__9ccBltDataFP1`).
- Register 2 in a location means "a scratch register"; parameters that simply
  stay in `a0`-`a3` are recorded that way.
- Unused parameters are dropped from the DWARF, `this` included, so a method
  that never touches `this` looks static.
- Each function has exactly one lexical block; inner scopes are flattened.
- Many file-static variables have address 0 in the DWARF although the ELF has
  a local symbol of the same name; `globals` takes the ELF address when the
  name is unique in its section (128 cases, labelled).
- 19 of 1,026 classes have gaps no alignment explains - `ccChunkIndex+0x4` is
  one - so a few members were never described.

## The source tree

Two roots: `D:\usr\RpgUS\prog\source` (211 files, 200 translation units) is
the game, `D:\usr\RpgUS\prog\system` (28 files, 14 units) is the engine - the
`cc` classes, the CCS loader `libccs2.cpp`, the inflater `ungzip.cpp`, the
CD code `cdvd.cpp`. The rest is CodeWarrior's (`gcc_wrapper.c`, MSL). The
translation units by overlay are on
[the source tree page](../docs/engine/source-tree.md), and the full list is
what `tools/dwarf1.py cus --tus` prints.

Some of what the names say about the game before any code is read: gcmn holds
the whole field game - `enemy1.cpp` to `enemyz.cpp`, `boss01.cpp` to
`boss08_c.cpp`, `kyvia01.cpp` to `kyviaCore.cpp`, `fellow01.cpp` to
`fellow17.cpp`, `town01.cpp` to `town05.cpp`, `field.cpp`, `fieldmesh.cpp`,
`fractal.cpp`, `dungeon.cpp`, `lattice.cpp`, `gmfood.cpp`, `dog.cpp`,
`pgbreed.cpp`, `pgrider.cpp`. desktop holds the in-game operating system -
`mailer.cpp`, `webnews.cpp`, `audio.cpp`, `savedata.cpp`, `NameEntry.cpp`,
`stfroll.cpp`. toppage is the bulletin board - `bbs.cpp`, `bbsmsg.cpp`,
`bbsmsgP.cpp`. demo is `opening.cpp` and `mpeg.cpp`.

## Examples

```
$ tools/dwarf1.py fn SLUS_202.67 mwLoadOverlay
// D:\usr\RpgUS\prog\source\mwUtils_PS2.c:339-355
// main 0x001002c0-0x00100368 (0xa8 bytes)
// frame 0x50, saves s0@sp+0x0 s1@sp+0x10 s2@sp+0x20 s3@sp+0x30 ra@sp+0x40
int mwLoadOverlay(char *pFilePath /* scratch */, void *pAddress /* $s3 */)
{
    int result;  /* $s1 */
    int size;  /* $s0 */
}
```

`type ccChunkIndex` gives the CCS object index entry, 0x40 bytes: `char *path`
at 0, an undescribed word at 4, `char name[30]` at 8, `u16 ccstag` at 0x26,
`void *subst`, `ccChunkBasis *chunk`, `ccAnmIndex *anmIndex` at 0x28-0x30.

**Still unknown:** the four-byte member CodeWarrior did not describe in
`ccChunkIndex` and the other 18 classes with unexplained gaps; the contents of
the `.mwcats` sections, which the DWARF does not reference.
