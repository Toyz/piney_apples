---
number: 255
title: The event scripts move into the build
date: 2026-09-27
area: build
files: crates/piney-data/src/pack.rs, crates/piney-data/src/iso.rs, crates/piney-event/src/official/mod.rs, crates/piney-build/src/lib.rs, crates/piney-build/src/main.rs, crates/piney-game/src/main.rs, plans/build-data.md
---

# 255. The event scripts move into the build

The owner's call: everything the port takes from the discs is to leave
the repository and live in what `piney-build` makes from the player's own
discs. That covers the scripts, the text, the tables and the placement
data. The repository keeps code, generators, type definitions, docs and
facts about the binaries. `plans/build-data.md` has the plan in four
steps. This entry is step 1: the event scripts and messages. They were
2.6 MB of `.evs` text generated into `piney-data/src/events` and compiled
into the port.

## Where the data lives now

- **The port's files, `PINEY/`.** Each volume's disc in a build gets the
  port's own files under `PINEY/`, beside the disc's. There are two so
  far:
  - `PINEY/VERSION` holds `pack::DATA_VERSION` (1);
  - `PINEY/EVENTS.EVS` holds the scripts and messages in their text form
    (`official::events_text`).

  They are ordinary manifest entries at LBA 0xffffffff (no place on the
  disc), chunked into `chunks.pak` with the rest, so what the volumes
  share is kept once. `piney-build` (now a library too, `port_files`)
  makes them from the disc's executable with `official::load_iso`, which
  finds the tables through the game's code on every volume. An earlier
  build's `PINEY/` is dropped and made again.
- **A disc image on its own.** It has no `PINEY/`: `Iso` serves those
  paths from the image's data folder in the port's home
  (`pack::image_data_dir`: `data/<image name>-<size in hex>/`).
  `piney_build::image_data` fills the folder. The game calls it for every
  disc it will play, before `deny_executable`, and it makes the files
  when they are missing or of another version, writing the version last.
- **Reading them.** `official::events(iso)` reads `PINEY/EVENTS.EVS`. A
  disc without it (a tool's or a check's raw image, where the executable
  may still be read) falls back to the executable. `events_of(volume)` now
  only gives what `events` read earlier in the run.
- **Refusing stale data.** The game refuses a disc whose port data is
  missing or of another version, saying to run `piney-build` again. Builds
  made before this change have no `PINEY/`.

Removed: `piney-data/src/events` (the four `.evs` and `mod.rs`) and
`piney-event`'s `gen_events` example. `export` stays, for per-event copies
in `work/`.

## Checked

- `the_builds_scripts_are_the_executables`: on every volume the text form
  reads back as the executable's events, event for event, and
  `official::events` on the raw image gives the same.
- A raw image (`--iso infection.iso --mode desktop`, a scratch
  `PINEY_HOME`): the first start makes `EVENTS.EVS` (692,480 bytes) and
  `VERSION`, the second reuses them, and the desktop runs its scripts.
- A one-disc build of Infection: the port's data is 0.7 MiB, every file
  reads back, and the game plays the title and the desktop from it.
- The whole workspace's tests pass: they open raw images and read the
  scripts from the executable.

**Still unknown:** How long an image's first start takes on a slow drive
(reading the executable is 0.1 s here).
