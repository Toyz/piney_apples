---
number: 538
title: GameClass: the classes only later volumes have, read from their code: ROOTTOWN06 and Quarantine's ccFellow18-20
date: 2026-11-01
area: tooling
files: crates/piney-gen/src/classes.rs, crates/piney-gen/src/placement/statics.rs, crates/piney-data/src/classes.rs, crates/piney-data/tests/classes.rs
resolves: 535
started: 2026-11-01T21:39:17-07:00
took: 8m
---

# 538. GameClass: the classes only later volumes have, read from their code: ROOTTOWN06 and Quarantine's ccFellow18-20

Worklog 535 left open which classes only the later volumes have: the
carried symbols name none of them, so `GameClass` could not hold them.
They are read from the code that makes them now (`piney-gen`'s
`classes::found`), and the generated table has them.

## How a class is read from its code

- **Its constructor.** A Root Town no symbol names is the function its
  `GetCCSAdrs("townNN")` is in (`placement::statics::unnamed_towns`, which
  the statics already used for its model table), from its frame's
  `addiu sp`. A fellow is `ccSpcStart`'s row: row `k`'s start function
  (`ccFellow01Start` on Infection, INF data 0x00654074) calls
  `operator new` and hands the object to the next call, the constructor
  of `ccFellow<k>`.
- **Its vtable and base.** The constructor stores its base's vtable (the
  base's constructor inlined) and then its own: each a table whose third
  word is a function. The last is its own; the one before names its base
  through that vtable's symbol (`__vt__8ROOTTOWN`, `__vt__8ccFellow`),
  else the base is the class whose constructor it calls.
- **Checked against the symbols.** Where a symbol names the class too
  (Infection's seventeen fellows, both constructor and vtable) the code's
  reading must agree, or `gen` fails: it does for all seventeen.

## What only the later volumes have

- `ROOTTOWN06`: Quarantine's sixth Root Town, constructor QUA gcmn
  0x004ec350, vtable 0x002785e0, base `ROOTTOWN` (as
  `docs/engine/root-towns.md` had it by hand); Mutation's (0x005fbd90,
  0x0038a9a0) and Outbreak's (0x005f98f0, 0x00385ce0) gcmn carry it
  unreached.
- `ccFellow18`, `19`, `20`: Quarantine's `ccSpcStart` has 21 rows
  (Infection's 18): rows 18-20 start at 0x004f02e0, 0x004f0560,
  0x004f0060 and make classes of `ccFellow` (constructors 0x004f0420,
  0x004f06a0, 0x004f01a0; vtables 0x00278650, 0x00278680, 0x00278620).
- The later volumes' `ccFellow01`-`17` gain their constructors and
  vtables, which the carry had missed.

A survey of every table stored as a vtable in the later volumes' main and
gcmn that no symbol names found these and nothing else new: the rest are
classes Infection's DWARF holds (event areas, the Root Towns 1-5, the
part creators, a boss effect), stored by constructors and destructors the
carry left unnamed, and one string ("MENU").

## Checks

- `piney-gen gen --check`; piney-data's classes tests (one new:
  `later_volumes_classes_read_from_their_code`), piney-data's,
  piney-battle's, piney-world's and piney-gen's tests.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** the later volumes' vtables of classes Infection names, where the carry named a constructor but no vtable (Quarantine's EVENTAREA01, ccBoss among them), stay unknown in the table: the last table a constructor stores is its own for the fellows and the towns (Infection's seventeen fellows agree) but not in general (129 of 162 of Infection's named constructors agree; the others store static objects' tables last), so reading them needs the store's slot, the vptr's offset
