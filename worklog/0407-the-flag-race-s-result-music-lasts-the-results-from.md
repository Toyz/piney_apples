---
number: 407
title: The Flag Race's result music lasts the results: from Mutation on ccEvVoiceStop does nothing while the race holds the music, so a box's OK no longer cuts it
date: 2026-10-08
area: audio, world, test
files: crates/piney-audio/src/driver.rs, crates/piney-audio/tests/voice.rs, crates/piney-audio/tests/voice_hold_fixture.txt, crates/piney-game/src/session/tests/flag_race.rs, tools/sound_ee.py, docs/engine/flag-race.md, docs/engine/sound.md
---

# 407. The Flag Race's result music lasts the results: from Mutation on ccEvVoiceStop does nothing while the race holds the music, so a box's OK no longer cuts it

Issue #64 (build 7b228d3): in Mutation, after a 1st, 2nd or 3rd place
in the Flag Race ([[395]]), the celebration theme stopped at the first
box's OK. It should play until the player leaves the results.

## What the game does

- **The race's music is `VOICE/BGM.BIN`** on SEWORDS's channel 0
  (`ccBgmPlay`, MUT main 0x001816f0; `bgmWavTbl` 0x0031e840, `bgmParam`
  0x0031cce0, mode 2 loops). Ranks 1-3 play track 4 with the cups for 191
  frames, stop it, and 12 frames later start track 5. That is the
  celebration theme: 36.6 s, looped. Rank 4 and no rank play track 6
  once (6.3 s).
- **Start.** The race's start, `ccPgBgmInit` in a town (0x001835b8), sets
  `ccSnd +0x13a`.
- **The boxes.** Each box's OK is `ccMessage::Check` calling
  `ccEvVoiceStop` (0x001bad90). From Mutation on, `ccEvVoiceStop`
  (0x00181f80) returns at once while `+0x13a` is set. Outbreak's
  (0x00181d50) and Quarantine's (0x00181e10) are the same; Infection's
  (0x0017ee40) has no test. `ccPgBgmInit`'s inline stop (0x00183424)
  tests the byte too.
- **Stop.** Menu 88's step 44 hands the prize to Get Item and sets the
  race's `+0xa5` (gcmn 0x0058bb40). The race loop's tail (0x005fe058)
  fades out over 30 frames, calls `ccBgmStop` (0x005fe0d0), and clears
  `+0x13a` (0x005fe104).
- So track 5 lasts through the time, the Rankings page, the breeder's
  word, the wallpaper and into the prize's window. Track 6 ends by
  itself while the time's box opens.

## Cause

The port's `Driver::voice_stop` always claimed a stop slot. The time
box's OK therefore stopped channel 0, and the `BGM.BIN` track with it.
The port already kept the hold as `race_music`, but only for
`bgmBreed`.

## Fix

`Driver::voice_stop` claims no slot while the race holds the channel
(`race_music`, from Mutation on). `pg_bgm_init` and `all_sound_off` go
through it, as `ccPgBgmInit` and `ccAllSoundOff` do in the game.

## Checked

- **`tools/sound_ee.py hold-fixture`** runs `ccEvVoiceStop`,
  `ccPgBgmInit` (field and town paths) and `ccAllSoundOff` with `+0x13a`
  set and clear, then `evVoicePlay`, on the four executables. It records
  what reaches `sewordCmd` (`voice_hold_fixture.txt`, 49 cases).
  `the_race_holds_channel_0_as_the_game_does` (piney-audio) matches the
  driver to it.
- **`mutations_race_music_lasts_the_results`** (piney-game, Mutation, Dun
  Loireag) hears every frame through `crate::handle` into a headless
  engine. Kite rides the race, and the records are set once the flags are
  taken, for ranks 1, 3, 4 and none. OK is pressed every 5 frames.
  - Ranks 1 and 3: track 5 plays through boxes 31, 32 and 41 (and the
    wallpaper's 51 for rank 1), through 10 and 7 voice stops. It stops on
    the race's own `BgmStop`, after `+0xa5`.
  - Ranks 4 and none: track 6 plays its 301,824 samples out.
- Without the fix, rank 1 fails: track 5 lasted 2 frames and stopped at
  the time box's OK. Ranks 4 and none pass either way, since their
  jingle ends before an OK can register.
- The flag race test helpers now step through a `Frames` trait, so the
  same helpers drive a plain session or a heard one.
- piney-game's suite (four threads), piney-audio's tests, clippy, fmt,
  `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing. The session test runs on Mutation only;
Outbreak's and Quarantine's races share the same driver path, and the
fixture checks their `ccEvVoiceStop` and `ccPgBgmInit`.
