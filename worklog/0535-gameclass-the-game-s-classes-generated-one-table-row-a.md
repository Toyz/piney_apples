---
number: 535
title: GameClass: the game's classes generated, one table row a class, the ccBoss family a narrow enum; the first defs pass set aside
date: 2026-10-30
area: battle
files: crates/piney-gen/src/classes.rs, crates/piney-gen/src/lib.rs, crates/piney-gen/src/main.rs, crates/piney-data/src/classes.rs, crates/piney-data/src/lib.rs, crates/piney-data/tests/classes.rs, crates/piney-world/src/chara/mod.rs, plans/rust-cleanup.md
started: 2026-10-30T15:16:29-06:00
took: 1h 11m
---

# 535. GameClass: the game's classes generated, one table row a class, the ccBoss family a narrow enum; the first defs pass set aside

The defs' `class` is a name the game gives, not a string: `piney-gen gen`
now writes `crates/piney-data/src/classes.rs` from Infection's DWARF and
every volume's symbols, and `gen --check` holds it current.

## What is generated

- **`GameClass`** (`#[repr(u16)]`, names only) and one table,
  `CLASSES: [ClassInfo; COUNT]`, a row a class: its `class`, `name`,
  `base` and its `constructor` and `vtable` on each volume. `info()` is
  `&CLASSES[self as usize]`; `name`, `base`, `constructor` and `vtable`
  read the row; `all()` walks the rows. No class is written twice.
- **Which classes**: every name with a `__vt__<n><name>` or a
  `__ct__<n><name>F..` symbol on some volume (Infection's own ELF table,
  the later volumes' carried `.syms`), and the bases of those, to the
  root: 310 classes, 1,061 lines (the table one row a line).
  - `base` is the DWARF's first `TAG_inheritance` (0x1c) child.
  - `constructor` is the lowest of a class's overloads.
  - A class only a later volume has and the carry did not name (Quarantine's
    `ROOTTOWN06`) is not in it.
- **`BossGameClass`**: the `ccBoss` family, every class whose ancestry
  reaches `ccBoss` (the bosses, Kyvia's, the parts and slaves: 24), each
  variant with its `GameClass` discriminant. `class()` is
  `CLASSES[self as usize].class` and `From<BossGameClass> for GameClass`
  uses it, so the family holds no data of its own. The towns', story
  maps' and characters' families follow with their defs.
- Checked against what the code already knew: ccBoss's constructor INF
  0x0045bcf0, ccBoss01's 0x0047b410, `__vt__13ccBossKyvia01` MUT
  0x0038a180; every boss class's base is `ccBoss` (worklog 534).

## The tests (`crates/piney-data/tests/classes.rs`)

- the table is in discriminant order (row `k` is the class `k`);
- the boss family is exactly `ccBoss`'s subtree in the game's tree, read
  from the generated enum's text, each member's ancestry reaching `ccBoss`;
- a `#[doc(alias)]` that names a class spells it as `GameClass::name`:
  `chara/mod.rs` had `ccnavi` (the file's name) for `ccNavi`, now fixed.

## Not committed: the first defs pass

`BossDef` with `BASE_BOSS` and `BASE_PART`, the per-boss tables owned by
the boss (`BossData` gone), the shared blocks in `boss/systems/` and one
runner over each def's `order` were built and checked (290 pins byte for
byte, every harness, the golden shots), but the systems were a flag enum
gating methods of a class trait, with a getter a system on `Boss` and
downcasts for the world. The maintainer's rule (plan section 2, "What a
system is") rules that shape out, so none of it is committed; the next
pass builds systems as objects emitting requests. Two findings carry over
(the plan's boss item):

- every boss def composes `BASE_BOSS` (parts `BASE_PART`): the generated
  `base()` of each boss class is `ccBoss`;
- Fidchell's, Macha's and Tarvos's own `ExecPatternIndex` fall back on
  `ccBoss::ExecPatternIndex` (OUT gcmn 0x004a4664; QUA 0x003a4228,
  0x003a8db0), whose unknown word lets the neutral go with forbid 1 (OUT
  0x0046e384, QUA 0x00360c94, Infection's too at 0x0045d10c), where the
  port holds it (3). The next commit fixes it.

## Checks

- `piney-gen gen --check`; the classes tests.
- The pins: 290 recorded `battle_probe` calls replayed, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots (`corbenik_under_god`, `kyvia_stage_shots`,
  `skeith_shots`, `tarvos_under_god`): every PNG byte for byte the same.
- piney-data's, piney-gen's, piney-battle's, piney-world's,
  piney-effect's and piney-event's tests, piney-game's full suite,
  release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** which classes Quarantine adds that Infection lacks
(`ROOTTOWN06` among them): the carry names none, so the generated list
cannot hold them until their names are found.
