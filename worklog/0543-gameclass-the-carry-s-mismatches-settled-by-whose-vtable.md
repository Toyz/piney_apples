---
number: 543
title: GameClass: the carry's mismatches settled by whose vtable the constructor stores, not by likeness
date: 2026-11-01
area: tooling
files: crates/piney-gen/src/classes.rs
supersedes: 539
started: 2026-11-01T23:26:04-07:00
took: 2m
---

# 543. GameClass: the carry's mismatches settled by whose vtable the constructor stores, not by likeness

Worklog 539 settled the three places where a carried volume's constructor
stores another table at its vptr than the vtable the carry named by how
alike the constructor's code is to Infection's (`xfer::likeness`,
`SAME_CODE` 0.5). That threshold rested on two cases (0.6 and 0.42). The
table the constructor stores decides it exactly (`classes::reassign`):

- **Another class's vtable, by the symbols:** the constructor is that
  class's. Outbreak's and Quarantine's `ccDarkUpheavalMngrElement`
  (OUT 0x00508370, QUA 0x003fac80) store `ccSoilUpheavalMngrElement`'s
  (OUT 0x00385bc0, QUA 0x002784c0): the Soil manager's constructor, the
  Dark one having none there. A class that already has another
  constructor fails `gen`.
- **A table no symbol names:** the class's own, the carried vtable name
  off. Mutation's `ccBossEffEnergyGrow2` stores 0x00389490, unnamed; the
  carry's 0x003894a0 is stored by no constructor. A named vtable some
  constructor does store fails `gen`.
- **On Infection** any mismatch fails `gen`: the symbols are the ELF's
  own (the old code compared Infection's constructor with itself there,
  likeness 1, and would have let one through).

The generated `piney_data::classes` is unchanged (`gen --check`), and
piney-data's `later_volumes_vtables_read_at_the_vptr` still pins the
three rows.

## Checks

- `piney-gen gen --check`; piney-gen's and piney-data's classes tests;
  clippy, fmt, `cairns check`, `tools/docs.py check`. Nothing at runtime
  reads `GameClass`'s addresses, and the table is byte-identical.

**Still unknown:** nothing; the threshold 539 rested on is gone.
