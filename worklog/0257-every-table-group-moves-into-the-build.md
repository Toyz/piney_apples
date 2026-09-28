---
number: 257
title: Every table group moves into the build
date: 2026-09-27
area: build
files: crates/piney-gen/src/data.rs, crates/piney-gen/src/render.rs, crates/piney-data/src/tables, crates/piney-desktop/src, crates/piney-demo/src/dialog.rs, crates/piney-toppage/src/assets.rs, plans/build-data.md
---

# 257. Every table group moves into the build

After the first five (worklog 256), the other sixteen of the generator's
21 groups moved too: `battle`, `combat`, `desktop`, `dtmenu`, `effect`,
`fieldui`, `fonts`, `kanji`, `loaddisp`, `nameentry`, `party_chat`,
`stream`, `title`, `toppage`, `voice` and `world`. Their values are no
longer in the repository:
- `piney-data/src/tables` was 12 MB of Rust and is now 388 KB of types,
  loaders and accessors;
- the values are 6.3 MB for the four volumes in `work/data`;
- each build disc's `PINEY/` is 2.1-2.2 MiB, and after dedup and zstd the
  whole build grew by 1.8 MiB.

## The shared statics

These groups have 337 entries alike on every volume, which were
`pub static`s. They are now `LazyLock` statics of the same type, read on
first use from whichever volume's values can be had. Most uses compiled
as they were: indexing, fields, method calls and `&X` all go through the
deref. The ones that took the value itself did not:
- 67 uses passed a static where its type was wanted, and got a `*`. The
  compiler's JSON spans drove the edits (a scratch script over `cargo
  build --message-format=json`), in `piney-desktop`, `piney-demo`,
  `piney-toppage`, `piney-audio` and `piney-game`.
- 5 moved a static out (`[t::VIBRATION_INFO, ...]`, `ne::HIRA_BLOCK`), and
  one built an array of them to `concat` (the fonts' rows past the end).
  Those took a `*` by hand, and a test's iterator took `&*FOOD`.

## Every call cheap

`store::group` takes a lock and a map lookup. The tables are read every
frame, so each generated module now keeps its own reference per volume in
a `OnceLock` (`of`), and one for the shared values (`shared`). After the
first call, `of(volume)` is a load.

`pack::DATA_VERSION` is 3. The default build was rebuilt. From it:
- the launcher plays and starts a part's title;
- Mutation's desktop set-up runs its scripts and windows, with the fonts,
  the texts and the dialogue tables all read from `PINEY/`.

`piney-gen gen --check` is current, clippy is clean, and the whole
workspace's tests pass (694), reading the tables from `work/data`.

**Still unknown:** Nothing new. The generator still reads its inputs from
`work/` (plans/build-data.md, step 2's remainder).
