---
number: 247
title: One game from the four discs with piney-build, and a launcher
date: 2026-09-27
area: tooling
files: crates/piney-build/src/main.rs, crates/piney-build/examples/estimate.rs, crates/piney-data/src/pack.rs, crates/piney-data/src/iso.rs, crates/piney-game/src/launcher.rs, crates/piney-game/src/main.rs, crates/piney-stream/src/scene.rs, crates/piney-stream/src/lib.rs, crates/piney-gs/src/lib.rs, crates/piney-gs/src/assets.rs, crates/piney-audio/src/lib.rs, docs/disc/outbreak.md, README.md
---

# 247. One game from the four discs with piney-build, and a launcher

Asked for: a `piney-build` that takes the four images, deduplicates them,
and builds one clean game that starts on the multi-game selector from
Outbreak's `STRT.BIN`. Players could use a single image, or build once and
not pass options after. Later in the session: deduplicate inside
`DATA.BIN`, the sound banks and the rest too, not whole files.

## The build

A build is a folder: `chunks.pak` and one `.disc` per volume
(`piney_data::pack` lays out both).
- **The chunks.** Every file of every disc is cut into content-defined
  chunks (FastCDC: 16, 64, 256 KiB). A chunk is kept once, by its BLAKE3
  hash, and zstd-compressed at level 15 where that makes it smaller.
- **A `.disc`.** The disc's files, with their paths, original LBAs, sizes
  and BLAKE3 hashes, each as a run of chunk references.
- **Reading.** `Iso::open` takes a `.disc` as it takes an image, so every
  reader in the port works unchanged. The one reader that went round
  `Iso`, `piney-audio`'s `BGM.BIN` seek, now reads through it.
- **The builder.** `piney-build` tells the discs apart by `GCMN.PRG`
  (`Volume::detect`) and writes to the port's folder
  (`~/.local/share/piney/game`) unless told otherwise.
  - It compresses on 8 threads in 32 MiB batches.
  - It reads every file back through the build and checks its hash.
  - A disc of an earlier build that is not given again is read from that
    build.
  - The images' volume identifiers are blank, so each disc is labelled by
    SYSTEM.CNF's boot name.

Measured on the four discs, 13,933.9 MiB of files:

| how | stored |
| --- | ---: |
| chunks as they are | 7,756.1 MiB (55.7%) |
| chunks zstd 3 / 9 / 15 | 7,254.3 / 7,233.1 / 7,136.9 MiB |
| archives' members inflated, then chunked, zstd 3 | 7,136.5 MiB |

The whole build, with the check, took 53 s from the page cache; 101,251
chunks.

On deduplicating inside the archives: the chunks already do.
- Mutation's, Outbreak's and Quarantine's `DATA.BIN` add 26.9, 11.5 and
  0.8 MiB to Infection's 130.3.
- `SNDDATA.BIN` adds 0.3, 0.5 and 0.2 MiB.
- Stream archives shared byte for byte (Quarantine's `STR1E`, `STR2E`,
  `STRCMNE`) add nothing.

What stays is different content.
- Outbreak's `STR1E` holds Infection's 40 scenes. 21 are the same
  inflated but compressed again (25.7 MiB); the other 19 differ inside
  (312.5 MiB), a few KiB changed early and the rest shifted. An aligned
  4 KiB block search over the 19 found 24.2 of their 687.4 MiB inflated in
  Infection's.
- Chunking the members' inflated contents (`examples/estimate.rs`) saves
  no more than zstd 15 on the files as they are.

So the port's archive readers, which read gzip members at the tables'
offsets, stay as they are.

The first run looped: `Iso::list` walked the root's `.` record as a
directory without end, and the process reached 39 GB before the OOM
killer took it. `children` now leaves that record out; the new test
`list_walks_infection_once` walks Infection's disc.

## The launcher

`piney-game` with no disc given finds the build. With more than one disc
in it, `LauncherMode` plays `trial_v2st`.
- **The scene.** It has two copies of most objects: the opening's, posed
  by the frames, and a still set at the origin that the scene never shows.
  The still set holds each row's highlight (`men_X0` with glows `lig_X0`,
  `lig_X1`), the demo labels (`mes`), extra icons and a background
  (docs/disc/outbreak.md).
- **What the launcher does.** The scene holds its last frame
  (`Stream::hold_end`), and a push during the opening skips to it
  (`fast_forward`). The chosen row's highlight is placed on it (a node's
  world matrix set from outside, `Scene::place`) 7,504.7 units left of the
  row's origin, where the two models' texts line up. Its glows pulse at a
  third. A part the build lacks is dimmed.
- **Controls.** Up and down move (the title's sounds 6, 4, 20). Cross or
  START starts that volume after 24 frames of flashing.
- **The switch.** `Event::Boot` makes the window open that disc: its
  `DATA.BIN` in the renderer (`Gs::set_archive`), its sound, its fonts,
  and the game mode.
- **Other cases.**
  - Without Outbreak in the build, a list in the game's font.
  - The last choice is kept (`launcher.txt`).
  - `--volume 1-4` skips the launcher; `--game DIR` names a build.
  - A build's card is `memcard/slot1` in the port's folder.

Checked headless: the opening skipped, down, cross, Mutation's title by
frame 136. With a two-disc build (Infection and Mutation) it shows the
list. The piney-data, piney-stream and piney-audio suites pass, and so
does `selector_rows_at_the_end` (the four rows at y 9000 to -4500, each
with its still highlight).

A piney-gs test had not compiled since the console's overlay argument
(03fbb29); it passes `None` now.

**Still unknown:** The window's switch from the launcher to a part was
checked headless only. Nothing tells the player that a finished part's
save loads in the next; the port's New Game from a previous volume's save
was not checked from a build.
