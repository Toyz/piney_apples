---
number: 212
title: The carry in Rust
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/carry.rs, crates/piney-gen/src/xfer.rs, crates/piney-gen/src/sinit.rs, crates/piney-gen/src/locate.rs, plans/volumes.md
---

# 212. The carry in Rust

The second step after 0211. The carry finds where Infection's globals sit
in Mutation, Outbreak and Quarantine, and `piney-gen` places every later
volume's table through it. It was `tools/carry.py` and used pieces of
`tools/xfer.py`. It is now `piney-gen carry`:

```text
piney-gen carry [--volume MUT|OUT|QUA] [--check]
```

The command writes `work/analysis/carry/<volume>.json`, or with
`--check` compares against it. The generator makes a missing cache
itself, as the Python did.

## What moved

- `carry.rs`: every pass, in the same order:
  - names;
  - the paired functions' votes;
  - unique content;
  - tables of pointers (`by_pointers`, with the edited-table vote);
  - by order in a changed function (`by_sequence`);
  - the fonts.
- `xfer.rs`: `normalise`, `addresses`, `shape` and `aligned`, the parts
  of `tools/xfer.py` the carry uses.

`aligned` rests on Python's `difflib.SequenceMatcher`, without its
junk heuristic (`autojunk=False`). `xfer::matching_blocks` reproduces
it: the longest match first (the earliest of equals), then either side
of it, the blocks sorted and joined. A unit test holds two cases against
difflib's own answers.

## Parity

`piney-gen carry --check` matched all three caches byte for byte. It
takes 20 seconds. `piney-gen gen --check` is still current.

Two places where Python's order is its hash order were made
deterministic:

- the votes' globals, now by (name, address, size);
- `similar`'s tally, now by first sight, as Python's `Counter` keeps it.

Neither changed a row.

`sinit.rs` had its own copy of `addresses` with a shorter list of
low-half opcodes than `xfer.py`'s. It gave the same mail links, but it
now uses `xfer::addresses`, as `sinit_tables.py` did.

`tools/xfer.py` stays: `voldiff.py`, `battle.py`, `statics.py` and other
tools use its `addresses`, `normalise` and `shape`, and it still writes
the `.syms` sidecars.

**Still unknown:** Whether the symbol transfer (`xfer.py transfer`, the
`.syms` sidecars) ports as exactly. It uses more of `difflib`:
`likeness` is `ratio()`, and ratios break ties. The sidecars feed the
carry's name pass, so any change there shows up in `piney-gen carry
--check`.
