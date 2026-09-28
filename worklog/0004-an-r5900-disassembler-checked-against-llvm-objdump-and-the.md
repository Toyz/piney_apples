---
number: 4
title: An R5900 disassembler checked against llvm-objdump and the relocations
date: 2026-09-22
area: tooling, decomp
files: tools/mips.py, tools/disasm.py, tools/image.py, tools/test_mips.py, tools/elf.py
---

# 4. An R5900 disassembler checked against llvm-objdump and the relocations

There is no PS2-aware disassembler on this machine. llvm-objdump knows
`mipsel` but not the R5900, so it cannot be the tool - but it can be the
check. We wrote our own:

- `tools/mips.py` decodes one word: the MIPS III/IV integer set with the EE's
  64-bit operations, the R5900 additions (`lq`/`sq`, three-operand `mult`,
  the pipeline-1 `mult1`/`div1`/`madd1` family, `mfsa`/`mtsa`), all four MMI
  groups, COP0, the EE's single-precision FPU including the accumulator ops
  (`adda.s`, `madd.s`, `mula.s`, ...), and COP2 in VU0 macro mode (`lqc2`,
  `qmfc2`, `vadd.xyz`, `vmaddaz`, `vdiv $Q`, `vcallms`, ...). Encodings with
  fields that must be zero are checked, so data decodes as `.word` rather than
  as a plausible instruction.
- `tools/image.py` is the program image: main plus, optionally, one overlay
  mapped at 0x00400800, with symbol lookup that only sees the loaded overlay's
  symbols, and the relocations.
- `tools/disasm.py` is the command line: `fn`, `range`, `xrefs`, `find`.
- `tools/elf.py` gained `Elf.relocs(section)`.

The CLI was first called `dis.py`. That name hides the standard library's
`dis` module for every script in `tools/`, because Python puts the script's
directory first on `sys.path`, and on Python 3.14 `argparse` reaches `dis`
through `inspect` - every tool crashed. It is `disasm.py` now.

## How it was checked

Against llvm-objdump (`--triple=mipsel --mcpu=mips4`) over the 2,241 function
bodies in main, word by word: llvm decodes 215,382 words; 200,606 have the
same mnemonic, and 13,224 more agree once objdump's pseudo-op spellings are
normalised (`li`, `move`, `c.lt.s` against `c.olt.s`, `jr` against
`jalr $zero`). That is 99.28%. Every one of the remaining 1,552 is an R5900
encoding that llvm reads as something else - `sq` as `ext`/`dext`, `lqc2`/`sqc2`
as Octeon `bbit0`/`bbit1`, MMI opcode 0x1c as SPECIAL2. llvm gives up on 6,677
further words, all EE-only (`lq`/`sq` alone are 5,524). Where the mnemonic
agrees the operands agree in 200,602 of 200,606; the 4 are a quirk in the
comparison script. gcmn, compared through llvm-mc because the ELF holds no
overlay bytes, agrees on 99.67%, the rest again EE-only.

Only 4 words in any function body of main or the four overlays fall back to
`.word`, and they are a genuine 128-bit constant (`0x00ff00ff` x4) embedded
in hand-written MMI code in `_copyRefImage` (`INF SLUS_202.67:0x001130e0`) and
loaded with `lq`.

The stronger check is the relocations ([[2]]). Every `jal` in every function
body carries an `R_MIPS_26` whose symbol is the target the decoder computed:
12,244 in main, 31,441 in gcmn, 849 in demo, 1,678 in desktop, 276 in toppage.
Every `HI16` sits on a `lui`, every `LO16` on an `addiu`/`ori`/load/store,
every `GPREL16` on a `$gp` access. And the address the decoder reconstructs by
following `lui` through the function agrees with the relocation's target at
all 7,521 sites in main and all sites in the overlays. Getting there needed
one fix: Sony's library code, built with GCC, reuses one `lui` for several
later uses, so pairing each `LO16` with "the latest `HI16`" gave `sceIoctl`
the wrong address; the tracker now pairs each low half with the nearest
preceding `lui` that wrote the same register.

`tools/test_mips.py` holds this: nine tests, the ones needing the game files
skipped when they are absent.

## What it showed on the way

`mwLoadOverlay` (`INF SLUS_202.67:0x001002c0`) reads the file with `mwBload`,
calls `FlushCache(2)`, `memset`s `bss_size` (`+0x14`) bytes after what it
loaded, and calls `__initialize_cpp_rts` with `ctor_start` and `ctor_end`
(`+0x18`, `+0x1c`) - confirming the overlay header fields read in [[2]].

The ten type-123 relocations in `.relmain` all sit in VU1 microcode embedded in
main between 0x001dbc00 and 0x001de558, on `iaddiu vi14, vi0, imm15` lower
instructions, targeting value-0 micro-address labels (`mcstp_clip*`,
`mcssp_clip*`). That matches old binutils' DVP relocation numbering,
`R_MIPS_DVP_U15_S3` (unsigned 15 bits scaled by 8); the name is inferred from
binutils history, not read from anywhere. It also makes the `mc_*` symbols of [[2]]
all but certain to be VU microcode labels - addresses in micro memory, which
would explain their value of 0 in the EE's symbol table.

`sceVu0Normalize` matches Sony's libvu0 source instruction for instruction,
and `_SetLWMatrix__7ccCoordFv` (`0x00138380`) is the heaviest VU0 user in main
at 29 macro ops. 43 functions in main and 13 in gcmn use VU0 macro mode.

**Still unknown:** the encodings the game never uses - the EE debug and
performance-counter moves, `rsqrt.s`, `max.s`/`min.s`, `vrnext`/`vrget`, the
`pmfhl` variants - are decoded from the manuals and not confirmed by any
binary; the VU1 microcode itself is not disassembled yet.
