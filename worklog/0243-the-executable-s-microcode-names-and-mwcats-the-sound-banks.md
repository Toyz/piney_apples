---
number: 243
title: The executable's microcode names and .mwcats, the sound banks' unused parts
date: 2026-09-27
area: format
files: docs/engine/executable.md, docs/formats/snddata.md
---

# 243. The executable's microcode names and .mwcats, the sound banks' unused parts

Asked for: more of the docs' open questions. `docs/engine/executable.md`
is now solid; `docs/formats/snddata.md` is down to one question about the
hardware.

## The executable

- **The `mc_*` names.** The 26 undefined `mc*` symbols are the VU entry
  points; [the draw path](../docs/engine/render.md) already had their
  table. Checked here against the bytes:
  - the code references 21 of them;
  - each one's relocated `lui`/`addiu` value is its `_$` label's byte
    address in micro memory: the offset from the `MPG` code before it,
    plus that `MPG`'s load address times 8.

  The other five have labels but no EE caller. The ten type-123
  relocations hold their target label's micro address (`>> 3`) in the
  15-bit immediate, all ten.
- **`.mwcats`.** One section per code section (`sh_link`), not loaded: a
  record per function built by the Metrowerks compiler.
  - Kind 2 holds a 16-bit size and the address. Kind 3 has a 32-bit size,
    used for the only two functions of 32 KiB or more (`ccEvent::Execute`,
    `ccEffect::Main`).
  - Flag bit 0 adds the offset of the function's closing `j`. It is set on
    exactly the 25 functions that end in a tail jump, always at size minus
    8.

  Every record matches a function symbol.
  - Records: all of `gcmn.prg`'s and `toppage.prg`'s functions, all but
    `ccKanji::ccKanji` in `demo.prg` and `desktop.prg`, and 1,351 of
    `main`'s 2,241.
  - `main`'s missing ones: the SCE libraries, newlib and libgcc (other
    compilers), and twelve out-of-line header inlines (binding 13).

## The sound banks

- **The SE tables.** `spcSeTbl`, `enemySeTbl` and `strSndTbl` were already
  covered in the sound engine page; the entry was stale.
- **`seTimbre`, Song, SE sequences.** All absent: -1 in all 79 `Head`s,
  and in all 150 `Sequ`s' Song, SE sequence and SE song. The code asks for
  none of them either:
  - `sceHSyn_Load` reads `seTimbre` (the banks are `Vers` 2) and skips a
    -1;
  - `SNDBASE.IRX`, the only importer of `MODMIDI.IRX`, never selects a
    Song;
  - no loaded module plays SE sequences.
- **ADPCM rounding.** Still open.
  - The port rounds the sum as PCSX2 does: `(s1*F0 + s2*F1 + 32) >> 6`.
  - DuckStation shifts each product on its own, without rounding.

  What the SPU2 does needs a measurement on the console.

**Still unknown:** The SPU2's ADPCM rounding.
