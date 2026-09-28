---
number: 259
title: The generator reads the discs
date: 2026-09-27
area: build, tooling
files: crates/piney-gen/src/source.rs, crates/piney-gen/src/program.rs, crates/piney-gen/src/volume.rs, crates/piney-gen/src/syms.rs, crates/piney-gen/src/carry.rs, crates/piney-gen/src/dtype.rs, crates/piney-build/src/lib.rs, crates/piney-build/src/main.rs, plans/build-data.md
---

# 259. The generator reads the discs

`piney-build` no longer needs the discs extracted in `work/`. The data it
writes into each disc's `PINEY/` is now made from the disc images it is
given, and it is byte for byte what the `work/` copies gave. This was the
remainder of step 2 of `plans/build-data.md` ([[256]], [[257]]).

## Where each input comes from

`piney_gen::source` is where the generator reads a volume's files. With a
disc named for the volume (`use_disc`, which `piney_build::use_disc`
wraps), a file comes off that disc through `Iso::read_path`. That disc can
be an image or a build's `.disc`. With none named, the file comes from
`work/<volume>/disc/`, as before, for the tools and checks.

| input | with a disc named | the tools |
| --- | --- | --- |
| the executable (`SLUS_...`) | the volume's disc | `work/<volume>/disc/` |
| the overlays (`DATA/*.PRG`) | the volume's disc | `work/<volume>/disc/DATA/` |
| `DATA.BIN` (the towns' statics) | the volume's disc | `work/<volume>/disc/DATA/` |
| the later volumes' carried names | made in memory (`syms::text`) | the `.syms` sidecar |
| the carry | made in memory (`carry::carry`) | `work/analysis/carry/*.json` |
| the tables' layouts (Infection's DWARF) | Infection's disc | `work/infection/disc/` |

The owner chose to make the names and the carry from the discs at build
time rather than commit them as facts. So a build of Mutation, Outbreak or
Quarantine needs Infection's disc in the same build. The alternative was
3.8 MB of committed names and carry plus a layouts file, so that any single
disc could build on its own; it was turned down.

## The pieces

- **Parsing split from opening.** `Elf::parse` and `Overlay::parse` take
  bytes. `Program::from_parts` takes the executable, a closure that gives
  the stripped volume's names, and a closure that reads the overlay.
  `Program::open` is those with files.
- **Opening a volume's program.** `volume::ctx` now opens it through the
  source.
- **Making a volume's names.** The transfer opens the volume's programs
  too. While it runs, `source::syms` answers None for that volume, so its
  programs open without names. After it, the volume's programs and xfer
  sides are dropped (`volume::forget`, `xfer::forget`), and the next ones
  opened carry the names.
- **The transfer does not depend on an old sidecar.** Mutation's names
  made with its sidecar moved away came out byte for byte the same as the
  sidecar.

`piney-build` names every disc of the build before it adds any. The game's
lone image (`image_data`) names its own disc. The old check that `work/`'s
executable matched the disc's is gone.

## Checked

A scratch program named the four `work/*.iso` images, ran `port_files` on
each, and compared every file with the default build's `PINEY/`, which
had been made from `work/`:

| volume | files | the same | time |
| --- | ---: | ---: | ---: |
| INF | 29 | 29 | 0.9 s |
| MUT | 29 | 29 | 17.0 s |
| OUT | 28 | 28 | 34.8 s |
| QUA | 28 | 28 | 34.7 s |

The run was then repeated with `work/*/disc` and `work/analysis/carry`
moved aside, with the same result, so nothing reached `work/`.
`piney-gen gen --check` (the tools' way) is current. The whole
workspace's tests pass (700), and clippy is clean.

**Still unknown:** Two things are untested:
- A lone image of a later volume played without Infection's disc
  (`image_data`): its generator falls back to `work/` for Infection's
  files. On a machine with neither, `crate::die` ends the process where an
  error to the game would be kinder.
- Whether `root()` (the repository's path, compiled in) is reached in a
  build run on another machine; in disc mode none of its callers were
  reached in this run.
