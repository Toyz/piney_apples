---
number: 510
title: Clean-up section 0, item 6: piney-gen's library returns errors instead of ending the process; the manifest's own mistakes panic
date: 2026-10-29
area: tooling, build
files: crates/piney-gen/src/lib.rs, crates/piney-gen/src/main.rs, crates/piney-gen/src/volume.rs, crates/piney-gen/src/dtype.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/locate.rs, crates/piney-gen/src/carry.rs, crates/piney-gen/src/syms.rs, crates/piney-gen/src/sinit.rs, crates/piney-gen/src/talk.rs, crates/piney-gen/src/layout.rs, crates/piney-gen/src/placement/mod.rs, crates/piney-build/src/lib.rs, plans/rust-cleanup.md
started: 2026-10-29T20:32:26-06:00
took: 11m
---

# 510. Clean-up section 0, item 6: piney-gen's library returns errors instead of ending the process; the manifest's own mistakes panic

Section 0 of `plans/rust-cleanup.md`, item 6: piney-gen's library no
longer ends the process. `piney_gen::die` called `process::exit(2)` at
43 sites. The game reaches them through `piney_build::image_data` when
the launcher builds a disc's data, so a missing Infection disc, an
unreadable carry cache or a later volume's table the generator could
not place closed the game without a word. The function is gone from
the library; only the `piney-gen` binary keeps a `die` for its own
command line.

## What each failure became

**Read faults and data the generator searches for: `Result<_, String>`.**

- `dtype::dwarf` (Infection's disc is needed for the layouts) and
  `manifest::groups`, which reads it first.
- `locate`: `carried`, `locate`, `find`, `extract` and
  `neighbour_candidates`, and the `rows` cache behind them.
- `carry`: `carry`, `cached` and `write`; the fonts found by content.
- `syms`: `content`, `refs`, `carry_symbols`, `text` and `write`.
- `sinit`: `save_sys_msg` and `mail_links`, whose eemu runs can fault.
- `talk`'s closure of the message objects.
- The file writes.

The manifest's finders already returned `Read`, so their `find` calls
take `?`. A volume's counts (`Count::By`, now a `CountFn`) return
`Result<usize, String>`.

**Programs: read up front.** `volume::open` returns `Result`, and
`volume::load(v)` reads all five of a volume's programs. These entry
points load Infection and the volume before anything else:

- `locate::extract`;
- every placement maker (`placement::read`);
- `carry::carry`;
- `syms::carry_symbols`.

`ctx` then has nothing left to fail on. If it is ever reached unloaded
and the read fails, it panics and names the cause.

**The manifest's own mistakes: `panic!`.** These are bugs in the
generator, not faults of a disc:

- a type name the DWARF lacks, or a member with no layout;
- two layouts under one name;
- an overlay or section name outside the five;
- a read of a program's own span or of Infection's own symbols.

`piney-gen gen --check` meets every one on each run.

`piney-build`'s `image_data` passes the groups' error on as it already
did the others.

## Checks

- `piney-gen gen --check`: every table module and every built group's
  bytes, all four volumes, unchanged.
- `piney-gen syms --check`: all three sidecars current.
- `piney-gen carry --check`: Mutation's cache is current. Outbreak's and
  Quarantine's caches in `work/` "differ" both before this change (HEAD
  21d97e2) and after. The tables built from them do not change.
- The tests of piney-gen and piney-build, and piney-game's suite.

**Still unknown:** Why work/analysis/carry/out.json and qua.json no longer match what piney-gen carry makes (they differ at 21d97e2 too); the tables built from the carry are unchanged either way.
