---
number: 256
title: Five table groups move into the build
date: 2026-09-27
area: build
files: crates/piney-data/src/store.rs, crates/piney-gen/src/data.rs, crates/piney-gen/src/lib.rs, crates/piney-gen/src/render.rs, crates/piney-build/src/lib.rs, crates/piney-game/src/main.rs
---

# 256. Five table groups move into the build

Step 2 of `plans/build-data.md` began: the generator's tables, whose
values `piney-gen` wrote into `piney-data/src/tables` as Rust (12 MB). The
machinery went in first, and then five groups moved:
- `game`, `mail`, `newgame`, `staffroll` and `talk`, which were 2.8 MB of
  Rust;
- they are now 11 KB of types and accessors, with 1.8 MB of values for
  the four volumes in the build.

These five went first because their users only reach them through
`of(volume)`, so no engine code changed.

## The format

Each group's values for a volume are one file,
`PINEY/TABLES/<group>.bin`. It holds every entry in the manifest's order,
in the generated types' own shape (`piney_data::store`, documented
there):
- numbers little-endian, and `f32` as its bits;
- text and slices as a u32 count followed by the items;
- a fixed array as its items;
- `Option` as a tag byte and then the value;
- a struct as its fields in turn;
- a function-name enum as its variant's name.

`piney-gen`'s writer (`data::write`) follows `Layout::emit` case for case,
so the file holds what the Rust literal held. It also records the
functions and characters it meets, for `types.rs` and `sjis.rs`.

## The generated side

A group in `data::IN_BUILD` renders as:
- its types;
- a struct of every entry, with its `Load`, and each entry as a method;
- `of(volume)` from `store::group`.

An entry alike on every volume keeps its `pub static`, now a
`LazyLock` read on first use from whichever volume's values can be had.
A keyed entry's enum method reads the same way. Every generated struct
and function enum now has a `Load` impl, including in the groups not
moved yet.

## Where the values are read from

`store::group(volume, name)` reads a group once a run and hands out
`&'static` references into it. It looks in two places:
1. **The game's discs.** The game registers each disc it plays
   (`store::use_disc`, before `deny_executable`), and the file is read
   through `Iso`: the build's `PINEY/`, or an image's data folder.
2. **The tools' and checks' copy.** `piney-gen gen` now also writes each
   built group's values to `work/data/<volume>/TABLES/` (gitignored with
   the rest of `work/`), and `--check` compares them. A missing file
   fails with the command that makes it.

`piney-gen` is a library as well now, and `piney-build`'s `port_files`
adds each built group's file. For now the generator still reads the
executable, the overlays and the carried symbols from the extracted discs
in `work/`. The build checks that the executable there is byte for byte
the disc's, and refuses otherwise. A single disc on a machine without
`work/` comes later: the symbols and layouts committed as facts.

`pack::DATA_VERSION` is 2. A raw image's data folder was remade on the
next start, and its `TABLES/staffroll.bin` matched `work/data`'s byte for
byte. The default build was rebuilt.

## Checked

The whole workspace's tests pass (694), reading the five groups through
the store's `work/data` copy. The staff roll's checks
(`the_staff_roll_is_the_games`, `the_staff_roll_view_is_the_games`) read
`staffroll` that way.

**Still unknown:** How many of the 337 shared statics' users will need
changing when their groups move, where a `LazyLock` does not stand in for
the value's own type.
