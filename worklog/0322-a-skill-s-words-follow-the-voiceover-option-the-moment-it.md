---
number: 322
title: A skill's words follow the Voiceover option the moment it changes
date: 2026-10-01
area: audio
files: crates/piney-game/src/area.rs, crates/piney-game/src/session.rs
resolves: 89
---

# 322. A skill's words follow the Voiceover option the moment it changes

[[89]] left open whether the driver follows a change of The World's Voice
option at once. OPTION's Sound page goes through `SetSoundEnv`, which the
port turns into `Event::Volumes` with the volumes only.

The game's sound code reads the save each time it plays a line:
- `ccEvVoiceRequest` (INF SLUS_202.67:0x0017e810) reads `saveData.voice`
  (+0x842c) for a message's voice;
- `skillVoicePlay` (0x0017e350) reads it for a skill's words.

So nothing has to carry the option over.

The port's driver holds its own copy, set by `Event::VoiceOptions`. Every
message voice was already preceded by one, built from the save as it
stands: in the town (world.rs) and in the field (area.rs). A skill's words
were not. `Event::SkillWords` came alone, so after the option changed in a
field, the next skill words were spoken in the old language until a message
voice or a new mode sent the options.

A skill's words in the field now go out after `voice_options` of the save.
The town has no skill words.

The test is `a_skill_in_a_fight_names_itself`: the save's `VOICE` is set to
Japanese before the fight, and the frame that sends Kite's `SkillWords` must
also send `VoiceOptions { english: false }`. It failed without the change
(`Some(false)`) and passes with it. The piney-game suite passes (202).

**Still unknown:** nothing about the option. The volumes' change from the same
page reaching the driver a frame after the game's is [[101]]'s task order,
which is still open.
