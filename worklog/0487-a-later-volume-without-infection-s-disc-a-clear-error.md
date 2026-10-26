---
number: 487
title: A later volume without Infection's disc: a clear error before generating, and every played disc named first so Infection's is found
date: 2026-10-25
area: build
files: crates/piney-build/src/lib.rs, crates/piney-build/src/main.rs, crates/piney-game/src/main.rs, plans/build-data.md
resolves: 282.1
started: 2026-10-25T18:38:31-06:00
took: 9m
---

# 487. A later volume without Infection's disc: a clear error before generating, and every played disc named first so Infection's is found

Play question [[259]] (282.1): a later volume's image played without
Infection's disc. The generator then falls back to `work/` for
Infection's files, and on a player's machine `crate::die` ends the
process.

**What stays.** A later volume's data is made from Infection's disc too:
- the tables' layouts are Infection's DWARF (`dtype::dwarf`);
- the carry finds the later volume's globals by Infection's own symbols
  and bytes (`carry::carry`);
- the talk records are Infection's DWARF globals (`talk.rs`).
plans/build-data.md records the owner's choice to need the disc rather
than commit the names, carry and layouts as facts. So a player with only
a later disc cannot make its data, and that is by design.

**What was wrong.**
- **The failure.** The generator's `die` ended the process with a
  generator message, partway into making the data.
- **A bug in the game's start-up, found on the way.**
  `piney_build::image_data` named only its own image to the generator
  (`use_disc`). Infection's image, given beside a later one, was named
  only if its own data was being made right then. When Infection's data
  already existed (`has_port_data`, the early return), or it came after
  the later disc, the later volume read Infection from `work/`. On a
  player's machine there is none, so even a player who gave both discs
  hit `die`.

**The fix.**
- `piney_build::infection_beside(v)` checks a later volume's need before
  anything is made. It reads Infection's executable through the
  generator's source, and on failure returns an error saying the data is
  made once with Infection's disc beside it. `image_data` calls it after
  naming its disc.
- piney-build checks it for every disc it makes fresh before writing
  anything.
- The game names every disc it plays (`use_disc`) before making any one's
  data, so Infection's is found whatever the order and whether or not its
  own data exists.

Check: `a_later_volume_asks_for_infections_disc` (piney-build). Infection
needs nothing more. With Infection named to a disc that cannot be read,
Mutation's check returns the error rather than ending the process.

The other half of [[259]]'s note (whether `root()` is reached in a build
run on another machine) is a separate question and stays open where it
is.

**Still unknown:** nothing
