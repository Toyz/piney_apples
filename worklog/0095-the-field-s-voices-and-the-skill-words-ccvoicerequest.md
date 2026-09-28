---
number: 95
title: The field's voices and the skill words: ccVoiceRequest, ccWordsPlay and skillVoicePlay
date: 2026-09-25
area: audio, test
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/tests/voice.rs, crates/piney-audio/tests/voice_ee_fixture.txt, crates/piney-data/src/sound/voice.rs, crates/piney-data/src/sound/mod.rs, crates/piney-data/src/sound/inf.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/main.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, tools/sound_tables.py, tools/sound_ee.py, docs/engine/sound.md
---

# 95. The field's voices and the skill words: ccVoiceRequest, ccWordsPlay and skillVoicePlay

The user reported that the skills had no voice lines. The audio driver
played the event dialogue's voices, but it marked everything below event
-1 as not ported. That meant these were silent:
- the field's own voices, including the party members' greetings on
  joining and leaving;
- the skill words, which piney-battle's `skill::Request.words` already
  asks for.

This port lives in piney-audio, so the combat agent only has to wire the
calls.

## The field's voices

`ccEvVoiceRequest` hands every event below -1 to `ccVoiceRequest`
(0x0017eeb0). That is a switch over 20 groups; each has a Japanese and an
English table in gcmn.prg, and a `voiceFile`:
- the Grunties (-2 to -12) use file 0;
- the dogs (-13 to -16) use 5;
- the fountain (-20) uses 1;
- party members in and out (-30) use 2;
- member talk (-31) uses 3;
- presents (-32) use 4;
- Fidchell (-40) uses 19.

`tools/sound_tables.py` now writes the tables into
`piney_data::sound::inf` (`EvVoice::field`), and
`Driver::field_voice_request` plays them. Because the field UI's Party
menu already sends `ccEvVoiceRequest(-30, ...)`, the greetings as members
join and leave now speak.

## The skill words

- **Queueing.** `ccWordsPlay` (0x0017e290) queues (character, skill)
  pairs, up to four.
- **Playing.** `skillVoicePlay` (0x0017e350) plays the first from the
  sound task, in a field or dungeon. It takes the character's file
  (`spcVoiceData`) and its rows (`voiceData`: `voiceKiteTbl` to
  `voiceHerubaTbl`), and finds the row by a per-character rule on the
  skill id and on bit 0 of the skill's type.
- **In the port.** These are `Driver::words_play`, `skill_voice_play` and
  `skill_row`. `Audio::words_play` and `set_game_area` give the runtime
  its handle, through `Event::SkillWords` and `Event::GameArea`. The town
  and the fields now report their `game.area`.
- **What is left to wire.** The fights raising `SkillWords` is the combat
  agent's.

## Checked

`tools/sound_ee.py voice-fixture` runs the game's `ccEvVoiceRequest`,
`ccWordsPlay` and `skillVoicePlay` in eemu, with GCMN.PRG loaded for the
new parts, and records what reaches SEWORDS.
- **The field's voices: 2,226 cases, 0 mismatches.** Every row of the 20
  groups in both languages, with Parody Mode on and off; seven groups
  with no case; three frames mixing a group with an event voice or a
  stop.
- **The skill words: 6,274 cases.** Each of the 18 characters' 304 ids in
  English, every seventh in Japanese, the gates (an event running, a
  caster of neither type, id 304), characters 18 and 19, the queue full,
  and a word left queued.
  - **5,236 of them match line for line.**
  - **The other 1,038 are outside their tables.** The row rule puts them
    outside a character's table: ids below the base, or past the end. The
    game reads the memory next to the table there, and the port plays
    nothing. A character casts only its own skills, so the rows it really
    reaches are inside.
- **The generated tables.** `tools/sound_tables.py --check` finds
  `inf.rs` current.
- **The rest.** The workspace's tests, clippy, fmt and the docs check
  pass.

**Still unknown:**
- **Not wired yet: the fights' calls.** The combat agent will raise
  `Event::SkillWords` from a skill request, and the battle's other voice
  cues through the same groups.
- **Kite calling the party's strategy** (`Show::Shout`,
  `ccSpcShoutOperationName`) goes through a path not looked at here.
- **The type bit.** `ccGetSkillParam(sid)+0x2c`'s bit 0 is taken from the
  caller (piney-battle's skill table). Its meaning (physical against
  magic?) is not named.
- **`vBank+0x08`.** `skillVoicePlay` leaves it as `evVoicePlay` last wrote
  it (0), and the port sends none.
