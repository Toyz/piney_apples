---
number: 89
title: English voices on a story start, and --voice en|jp
date: 2026-09-24
area: audio, test
files: crates/piney-game/src/area.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/main.rs, crates/piney-game/src/world.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/session.rs, docs/engine/sound.md
---

# 89. English voices on a story start, and --voice en|jp

The user played `story:4` and got English text with Japanese voices.

## Why

- **The save said English.** A new game's save comes from the boot's
  `Init(0)`, which sets `saveData.voice` to 1 (English, `VOICE_E/`).
- **The driver never heard it.** The game's sound driver reads that byte
  each time it plays a line. The port's driver keeps its own copy,
  `voice_english`, which is false until a mode sends `VoiceOptions`.
- **Who sent it.** Only the desktop and the town sent it as they start.
  `story:4` starts straight in the dungeon, a field-and-dungeon mode that
  never sent it, so the copy stayed Japanese. Anyone who reached a field
  through the desktop or Mac Anu heard English.

## The fix

- **Fields and dungeons send it too.** They hand the driver the save's
  voice language and Parody Mode as they start, as the town does.
- **One reader.** Every read of the language goes through
  `mode::voice_english(save)`:
  - the three modes' `VoiceOptions`;
  - the town's event voice requests;
  - the stream player, which picks a stream's voice track from it.
- **The command-line option.** `--voice en|jp` sets the language for the
  whole run, over the save's Options setting. The user asked for an easy
  switch.

## Checked

- **A new test.** `a_dungeon_start_sets_the_voice_language` runs
  `story:4` for 30 frames and requires `VoiceOptions`, every one English.
- **The test catches the bug.** With the dungeon's events emptied again, it
  fails ("no voice options in the first 30 frames").
- **The rest.** `story_starts_open_their_event`, the workspace's tests,
  clippy, fmt and the docs check pass.

**Still unknown:**
- **Not seen with the new flag.** `--voice` is not yet in any playthrough
  test; the unit path is the same helper.
- **Options changes during play.** Changing Voice in the game's Options
  inside The World goes through the field UI's `SoundEnv` request, which
  carries the volumes only. The driver's copy may not follow until the
  next mode starts. Whether the game applies it at once is not checked.
