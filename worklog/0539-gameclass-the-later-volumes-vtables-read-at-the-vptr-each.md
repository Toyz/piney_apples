---
number: 539
title: GameClass: the later volumes' vtables read at the vptr each constructor stores into
date: 2026-11-01
area: tooling
files: crates/piney-gen/src/classes.rs, crates/piney-data/src/classes.rs, crates/piney-data/tests/classes.rs
resolves: 538
started: 2026-11-01T21:49:46-07:00
took: 17m
---

# 539. GameClass: the later volumes' vtables read at the vptr each constructor stores into

Worklog 538 left the later volumes' vtables of classes Infection names
unknown where the carry named a constructor and no vtable: "the last
table a constructor stores" is not its own in general. `piney-gen`'s
`classes::vtables` reads them now, at the slot the vtable goes in.

## The slot, not the order

- **What a constructor stores into its object** (`classes::stores`):
  `this` comes in `a0`, and the code is unoptimised, so it is spilled to
  the stack and reloaded for every store; offsets past 32 KiB are built
  with `lui` and `addu` (`ccBoss`'s vptr is at 0x29324 on Infection). A
  store of a table (its third word a function) into `this` at an offset
  is kept with the offset. A member object's vtable lands at its own
  offset (`ccBoss`'s `ccBossAfterImageManager`), a static object's
  through `gp` or `lui`, so neither is taken for the class's.
- **The vptr's slot**, per volume (the layouts grow: `ROOTTOWN`'s is
  0x1b0 on Mutation, not Infection's): where a constructor stores a
  vtable the symbols name, the class's own or an ancestor's (its
  constructor inlined, as `ROOTTOWN`'s in every town's). A class with
  neither takes its nearest ancestor's, else that of any class under the
  root-most class with a vtable (one vptr down a single-inheritance
  chain). The DWARF has no `_vptr` member, so the code is the source.
- **The vtable** is the last table the constructor stores at that slot.

## Checked against the symbols

On Infection every class whose constructor and vtable the ELF names reads
the same from the code, or `gen` fails. On the carried volumes three
disagree, each a carry mistake, settled by how alike the carried
constructor is to Infection's (`xfer::likeness`, `SAME_CODE` 0.5):

- Mutation's `ccBossEffEnergyGrow2` (0.6): its constructor builds the
  energy-charge generator from `g_energyChargeForceField` as Infection's
  does, and stores 0x00389490; the carry named the next table, 0x003894a0
  (stored only by the destructor it named, a class Mutation adds), after
  it. The vtable is 0x00389490.
- Outbreak's and Quarantine's `ccDarkUpheavalMngrElement` (0.42): the
  carried constructor (OUT 0x00508370, QUA 0x003fac80) is a quarter of
  Infection's size and stores `ccSoilUpheavalMngrElement`'s vtable,
  which that class's destructor stores too: it is the Soil manager's
  constructor, and the Dark one has none on those volumes.

## What the table gains

The nine event areas on Outbreak and Quarantine, the five Root Towns,
Kyvia's four classes, eleven `ccEnemy` kinds and the two corrected rows
above. Every class with a vtable on some volume now has one wherever it
has a constructor (`later_volumes_vtables_read_at_the_vptr`).

## Checks

- `piney-gen gen --check`; piney-data's classes tests (one new),
  piney-gen's tests; clippy, fmt, `cairns check`, `tools/docs.py check`.
  Nothing at runtime reads `GameClass`'s addresses yet.

**Still unknown:** nothing this step opened.
