---
number: 398
title: Every town's ranch plays the breeder's tune: bgmBreed reads each town's breeder after the load that clears its place, and the later volumes' crisis rows are six on
date: 2026-10-08
area: audio, volumes, test
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/tests/driver.rs, crates/piney-audio/tests/driver_fixture.txt, crates/piney-audio/tests/driver_fixture_mut.txt, crates/piney-audio/tests/driver_fixture_out.txt, crates/piney-audio/tests/driver_fixture_qua.txt, crates/piney-audio/tests/hsyn.rs, crates/piney-audio/tests/render.rs, crates/piney-audio/tests/stream.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/ranch_music.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/skeith.rs, crates/piney-game/src/session/tests/party_leave.rs, tools/sound_ee.py, docs/engine/sound.md
---

# 398. Every town's ranch plays the breeder's tune: bgmBreed reads each town's breeder after the load that clears its place, and the later volumes' crisis rows are six on

Issue #62 (build 4cce396): in Mutation, Carmina Gadelica's Grunty ranch
played the town's theme, not the ranch's.

## What the game does

- `ccSoundMain` runs `bgmBreed` each frame while `ccSnd +0x105` is 3: any
  town but Mac Anu. It is the same on all four volumes (MUT main 0x0017eda0).
  - Once (`+0x114`), it copies the town file's `DMY_merchant6` to `+0x120`.
    Every town stands its breeder at that dummy.
  - Within 1000 of it, sequence 0 fades out and sequence 1 (the ranch's
    tune) fades in. Beyond 1200, the reverse.
  - From Mutation on it does nothing while `+0x13a` (the Flag Race) is set.
    Outbreak's and Quarantine's fades go through a helper (OUT
    0x0017bde0); the steps are the same.
- `ccSndSQLoad` zeroes `free[4]` (`+0x108`-`+0x114`) in two places:
  - a town other than Mac Anu (INF 0x00182448);
  - area 15's event bank (0x00182528).

  No other load touches them. So `bgmBreed` reads each town's own breeder.
  Every town's bank but Mac Anu's has the ranch's tune as sequence 1.

## Cause

- The port latched the breeder's place once a session (`Driver::breeder`
  was never cleared). From the second town on, the tune listened at the
  first town's breeder. A player who had been to Dun Loireag heard the
  town's theme at Carmina Gadelica's ranch.
- The driver cleared `+0x108` on every load and `+0x10c` and `+0x114` on
  none.

## Found on the way

The driver fixture made again on the later executables turned up these:

- **The crisis rows.** From Mutation on `sqDataTown`'s crisis rows are 6
  on, not 5 (MUT 0x00185c08):
  - row 5 is empty;
  - rows 6-9 are Infection's 5-8;
  - Lia Fail's crisis row is an eleventh row past the symbol's ten.

  Outbreak's and Quarantine's new games start in the crisis. The port
  loaded row `town + 5` there:
  - Mac Anu got the empty row and was silent;
  - Dun Loireag got Mac Anu's crisis bank, with no ranch's tune.
- **Empty sound effects.** From Mutation on `ccSeOn` and `ccSeOnNote` send
  nothing for a row whose program is below 0 (MUT 0x0017c310, 0x0017c3f4).
  These are rows 233 and 234. The port sent program 127.
- **The desktop's music.** From Outbreak on, the desktop case of
  `ccSndBgmCtrl` starts sequence 1 for `dtBgm` 47 too (OUT 0x0017dca8).
- **The race's hold.** Every town's load clears `+0x13a` (MUT
  0x00185cb0). The port cleared it only at the race's end.

## Fix

- `SqContext::Town { town, crisis }`: the row is `town_row(volume, town,
  crisis)`, so `pick` takes the volume.
- `Pick::clears_scene` is set for a town other than Mac Anu and for area
  15. `sq_load` then clears `scene_bgm`, `church_block` and `breeder`, and
  clears nothing on other loads. Every town's load ends the race's hold.
- `Driver::se_on` and `se_note` are methods; they check the program from
  Mutation on.
- `bgm_plan` takes the volume, and `desktop_second` gives row 47.

## Checked

- **Against the game's code.** tools/sound_ee.py gained the scene sounds:
  - steps `breeder`, `kite`, `camera` and `block`;
  - `scene`, which is `ccSoundMain`'s dispatch to `bgmChurch` and
    `bgmBreed`;
  - test_anim's VU0 machine, for `ccGetDist`.

  Six new scenarios run the ranch near and far, two towns in a row, Mac
  Anu and a field between them, and the church with and without the
  ranch's tune before it.

  The fixture is now made on each executable: `driver_fixture.txt`,
  `driver_fixture_mut.txt`, `_out` and `_qua`. Each has 237 sound effects,
  30 notes, 51 desktops, 6 volumes, 10 jukebox changes and 376 sequences.
  The driver matches all four on their own discs. Before the fix the
  two-town scenarios failed on every volume.
- **Played.** In piney-game, `every_ranch_plays_the_breeders_tune` routes
  every frame's events from the session's start through `crate::handle`
  into a headless engine. On Mutation, and on Outbreak and Quarantine in
  the crisis, Kite logs in to Dun Loireag. The console's `town N` takes
  him to Carmina Gadelica, Fort Ouph, Lia Fail and back to Dun Loireag.
  - In each town he walks round the walls to the ranch (survey.rs's
    planner, now `path_wide` with a `Look`).
  - At each ranch sequence 1 must play and sequence 0 be off.
  - Before the fix it failed at Mutation's Carmina Gadelica (`[true,
    false, false]`). With the crisis rows 5 on, it failed at Outbreak's
    Dun Loireag.
- piney-audio's tests, piney-game's suite (four threads), clippy, fmt,
  `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing.
