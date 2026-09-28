---
number: 213
title: The symbol transfer in Rust
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/syms.rs, crates/piney-gen/src/xfer.rs, crates/piney-gen/src/program.rs, tools/xfer.py, docs/disc/volumes.md, README.md
---

# 213. The symbol transfer in Rust

This is the third step after 0211 and 0212. The symbol transfer names
the stripped executables: it writes Mutation's, Outbreak's and
Quarantine's `<elf>.syms`, which the carry and every tool read. It was
`tools/xfer.py match` and is now `piney-gen syms`:

```text
piney-gen syms [--volume MUT|OUT|QUA] [--check]
```

With this step the whole chain that makes the generated tables is Rust:
the symbol transfer, the carry, then the tables.

## What moved

- `program.rs`: the relocations resolved as `tools/image.py` resolves
  them. A LO16 pairs with the nearest `lui` before it in its function
  that writes the register it uses, else with the latest HI16 in table
  order. GPREL16 is `$gp` plus the offset.
- `xfer.rs` (`transfer`): the first passes, a section at a time, for up to
  six rounds:
  - exact bodies;
  - callees and the globals the paired bodies build (`propagate`);
  - the functions between paired neighbours, by the shape of their code
    (`by_order`: a longest increasing run of anchors, then the best
    monotone pairing, scored by difflib's `ratio()`).
- `syms.rs`, the later passes in the same order as before:
  - cross;
  - content;
  - pointer;
  - layout, with `Agreement`;
  - content again;
  - ref;
  - pointer again.

The row order is the sidecar's order, so every Python dict whose order
reached the output kept that order. `Ordered` keeps a map's keys in the
order first inserted. A key removed and put back goes last, as in Python.
The float comparisons are the same `f64` operations: the order pass's
`2.0 * M / T` ratios and their sums, and the layout pass's word shares.

## Parity

All three sidecars matched byte for byte on the first run. Mutation's was
first checked to be what the Python writes today. A run takes 9 seconds
a volume; the Python took 32.

The sidecars were then rewritten with their header naming `piney-gen
syms`. Every row is unchanged, and `piney-gen carry --check` and `gen
--check` are still current.

## What stays in Python

`tools/xfer.py` is now only the code comparisons other tools import:
`normalise`, `addresses`, `shape` and `Side`. The users are `voldiff.py`,
`statics.py`, `font.py`, `battle.py`, `dungeon.py`, `field.py`,
`field_tables.py`, `evscript.py` and their tests. The transfer's passes
are gone from it. The tools' docs now name `piney-gen syms` as the
sidecars' source. `tools/test_font.py` passes on all four volumes.

**Still unknown:** The five older generators that write Rust from Python
(`statics.py`, `area_tables.py`, `dungeon_tables.py`, `field_tables.py`,
`sound_tables.py`) remain. They join `piney-gen` when the world's rebuild
needs their tables per volume, which is the next piece of the Infection
boot.
