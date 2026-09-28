---
title: The EE executable
status: solid
volumes: INF
covers: INF SLUS_202.67
worklog: 2, 4, 6, 243
---

# The EE executable

`SLUS_202.67` is the Emotion Engine program the BIOS boots. It is a 32-bit
little-endian MIPS ELF built with Metrowerks CodeWarrior for PS2
(`MW MIPS C Compiler (2.4.1.01)`), and it was shipped with its symbol table,
DWARF 1 debug information and relocations intact.

## Header

```
e_type     2 (EXEC)          e_machine  8 (MIPS)
e_entry    0x00100008        e_flags    0x20924000
6 program headers, 24 sections
```

## Program headers

```
LOAD  off 0x00000100  va 0x00100000  filesz 0x278880  memsz 0x300800   main
LOAD  off 0x00278980  va 0x00400800  filesz 0         memsz 0x32fe00   gcmn.prg
LOAD  off 0x00278980  va 0x00400800  filesz 0         memsz 0x00fc00   demo.prg
LOAD  off 0x00278980  va 0x00400800  filesz 0         memsz 0x069b80   desktop.prg
LOAD  off 0x00278980  va 0x00400800  filesz 0         memsz 0x02a300   toppage.prg
LOAD  off 0x00278980  va 0x00730600  filesz 0         memsz 0          heap
```

Only `main` has bytes in the file. The four overlays come from `DATA/*.PRG`
([the overlay format](../formats/prg.md)); their `memsz` is the overlay's full
footprint including its 0x40-byte header.

## Sections

| # | name | type | addr | size |
| ---: | --- | --- | --- | ---: |
| 1 | `.shstrtab` | STRTAB | | 0xd9 |
| 2 | `.strtab` | STRTAB | | 0x57ce4 |
| 3 | `.symtab` | SYMTAB | | 0x8a370 |
| 4 | `main` | PROGBITS | 0x00100000 | 0x278880 |
| 5 | `.relmain` | REL | | 0x45d40 |
| 6 | `gcmn.prg` | PROGBITS | 0x00400800 | 0 |
| 7 | `.relgcmn.prg` | REL | | 0x90830 |
| 8 | `demo.prg` | PROGBITS | 0x00400800 | 0 |
| 9 | `.reldemo.prg` | REL | | 0x4a60 |
| 10 | `desktop.prg` | PROGBITS | 0x00400800 | 0 |
| 11 | `.reldesktop.prg` | REL | | 0x15720 |
| 12 | `toppage.prg` | PROGBITS | 0x00400800 | 0 |
| 13 | `.reltoppage.prg` | REL | | 0x5558 |
| 14 | `heap` | PROGBITS | 0x00730600 | 0 |
| 15-19 | `.mwcats` | 0xca2a82c2 | | 0x2aa4, 0x5660, 0x410, 0x708, 0x188 |
| 20 | `.debug` | MIPS_DEBUG | | 0xcb3be7 |
| 21 | `.line` | MIPS_DEBUG | | 0x147ea0 |
| 22 | `.comment` | PROGBITS | | 0x2b |
| 23 | `.reginfo` | MIPS_REGINFO | | 0x18 |

## Memory map

```
0x00100000 - 0x00378880   main .text and .data
0x00378880 - 0x00400800   main .bss             _fbss = 0x00378880
0x00400800 - 0x00730600   overlay window        sized for gcmn, the largest
0x00730600 -              heap                  _end = 0x00730600
```

`$gp` = 0x0037faf0 (`.reginfo` word 5, and the symbol `_gp`). `_stack` and
`_heap_size` are 0xffffffff; `_stack_size` is 0x8000.

## Symbols

35,383 entries in `.symtab`. Section index says where a symbol lives, and for
the overlay window it is the only thing that says which overlay:

| shndx | section | symbols | functions |
| ---: | --- | ---: | ---: |
| 4 | `main` | 10,424 | 2,241 |
| 6 | `gcmn.prg` | 18,424 | 2,764 |
| 8 | `demo.prg` | 396 | 131 |
| 10 | `desktop.prg` | 4,263 | 226 |
| 12 | `toppage.prg` | 1,843 | 49 |
| 14 | `heap` | 2 | 0 |
| 0 | undefined / absolute | 31 | 0 |

Function names are mangled in the cfront style: `name__F<args>` for free
functions, `name__<len><class>F<args>` for methods, `__ct`/`__dt` for
constructors and destructors. Local string literals and constants appear as
`@<n>` objects. The undefined symbols include linker values (`_stack_size`,
`_align_segment`) and a family of `mc_*` names with value 0.

The 26 undefined `mc*` names are the entry points of the VU microcode
embedded in `main` ([the draw path](render.md#microcode)). Each has a local
`_$<name>` symbol at its EE address, and the relocated code holds its
address in micro memory, in bytes:
- the EE code loads it with `lui`/`addiu` against the undefined name, and
  shifts it right by 3 for an `MSCAL`;
- for all 21 names the code references, that value is the label's offset
  from the `MPG` code before it, plus the `MPG`'s load address times 8
  (VU0's two from `mcVu0_Top`'s `MPG`);
- the other five (`mc_DrawTri`, `mc_DrawTriC`, `mc_DrawTriS`,
  `mc04b_DrawModel0`, `mc04b_DrawModel1`) have labels in the microcode, but
  no EE code calls them (`mc04b` is reached by a branch from `mc03b`).

## Relocations

One `SHT_REL` section per code section, 8-byte entries (`r_offset`, `r_info`),
symbol index `r_info >> 8` into `.symtab`, type `r_info & 0xff`:

| section | R_MIPS_32 | R_MIPS_26 | HI16 | LO16 | GPREL16 | other |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `.relmain` | 12,308 | 12,374 | 3,438 | 3,727 | 3,895 | 10 (type 123, see below) |
| `.relgcmn.prg` | 22,463 | 31,441 | 6,697 | 6,697 | 6,692 | 0 |
| `.reldemo.prg` | 107 | 849 | 559 | 559 | 306 | 0 |
| `.reldesktop.prg` | 5,063 | 1,678 | 1,854 | 1,854 | 531 | 0 |
| `.reltoppage.prg` | 2,303 | 276 | 45 | 45 | 62 | 0 |

Type 123 is not in the MIPS ABI. All ten sit in VU1 microcode embedded in
main between 0x001dbc00 and 0x001de558, on the 15-bit immediate of
`iaddiu vi14, vi0, imm15` lower instructions, and point at value-0 micro-memory
labels (`mcstp_clip*`, `mcssp_clip*`). In all ten the immediate is the
target label's micro address (its byte address `>> 3`). That matches old
binutils' DVP numbering, `R_MIPS_DVP_U15_S3`; the name is inferred.

## .mwcats

Five sections of type 0xca2a82c2, one per code section: each one's
`sh_link` is its section (4 `main`, 6 `gcmn.prg`, 8 `demo.prg`, 10
`desktop.prg`, 12 `toppage.prg`). They have no address and are not loaded;
they are for the tools, not the game. Each is a list of records, one per
function, in address order:

```
u8   kind       2: size in 15 bits; 3: size in 32 bits
u8   flags      bit 0: a tail-jump word follows
kind 2:  u16 size, u32 address                     8 bytes
kind 3:  u16 0, u32 size, u32 address             12 bytes
flags & 1:  u32 offset of the function's last j   +4 bytes
```

Every record matches a function symbol's address and size. Checked over
`main`'s 1,351 records:
- **flags bit 0** is set on exactly the 25 functions that end in a `j`
  (a tail call); the offset is always the function's size minus 8, the
  `j` and its delay slot. 1,292 others end in `jr $ra`, and 34 in a `b`
  back into themselves (the threads' loops), a `jal` or a `jr` through
  another register.
- **kind 3** is used twice, for the two functions of 32 KiB and more
  (`ccEvent::Execute`, 0x9734 bytes, and `ccEffect::Main`, 0x82ac); the
  largest kind 2 in any section is 0x5480.

The records cover the functions the Metrowerks compiler built:
- every function of `gcmn.prg` and `toppage.prg`;
- all but one of `demo.prg`'s and `desktop.prg`'s, each missing its copy of
  `ccKanji::ccKanji`;
- 1,351 of `main`'s 2,241.

`main`'s other 890 are the SCE libraries (`sce*`), newlib's C and maths
library and libgcc (`__ieee754_*`, `__divdi3`, `__sfvwrite`, ...), which
were built by other compilers, and twelve out-of-line copies of header
inlines (symbol binding 13, such as `ccMmat::ccMmat` and
`ccModifier::~ccModifier`); 19 other binding-13 functions have records.
The later volumes' executables have no `.mwcats`.

## Debug information

`.debug` is DWARF version 1; `.line` holds its line tables. The first compile
unit is `D:\usr\RpgUS\prog\source\crt0.s` (producer
`Metrowerks MW GAS R5900 Assembler`); C and C++ units carry producer
`MW MIPS C Compiler`.

Each function has its own compile unit, preceded by one range-less unit per
translation unit holding its types and globals; the four overlays are
described by vendor tag 0x4080 with their member compile units. The reader is
`tools/dwarf1.py`, and the translation units are listed on
[the source tree page](source-tree.md).

## Unknown

None.
