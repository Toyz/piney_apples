---
number: 505
title: Skill words are the game's on every disc: Mutation's Sora no longer holds the queue, Quarantine's Tsukasa, Subaru and Sora read their own rows
date: 2026-10-26
area: audio
files: crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/voice.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/tests/words.rs, crates/piney-audio/tests/voice.rs, tools/sound_ee.py, crates/piney-game/src/session/tests/skill_words.rs, docs/engine/sound.md, BUGS.md
started: 2026-10-26T01:10:14-06:00
took: 33m
---

# 505. Skill words are the game's on every disc: Mutation's Sora no longer holds the queue, Quarantine's Tsukasa, Subaru and Sora read their own rows

BUGS.md: "party members' skill voice lines not heard". Worklog 300 found
every path it tried speaking, so the bug stayed open. This time every
step was enumerated from the code, on all four discs.

**The steps.**
- **The request.** `_ccSkillRequest` (gcmn 0x00572b98) names the skill
  for any caster: stype 0 or 2, id 6 and up. The port shows `Show::Words`
  at every request site: the menus, the party AI, items through their
  user, the enemies.
- **`ccWordsPlay`** (0x0017e290). It refuses during an event, for a
  caster without `base->type & 5`, and for ids 304 and up. Every
  `charTbl` character is type 6, and Kite is 7.
- **`skillVoicePlay`** (0x0017e350). It takes the first queued word and
  empties all four slots: other words queued in the same frame are lost,
  and the menu task (34) queues Kite's before the party's (48). A NULL
  file, or a row of -1, returns before the queue empties. The word is
  tried every frame and holds the queue until power-on: `ccSound` is
  made once, by `ccSoundMain`.
- **SEWORDS's `wordPlay`.** It refuses a file while `gFd[0]` is open.
  When the open fails, it stores the negative fd and the channel stays
  free.

So on Infection a member's word goes unheard only when it lands with
another word in the same frame, or while a line plays. That is the
original's behaviour.

**The later volumes were not the game's.**
- **Mutation** reads `spcVoiceData` past its 18 entries:
  - Tsukasa and Subaru (18, 19) get NULLs, and the game holds the queue.
    The port did too.
  - Sora (20) gets `voiceDataE[0]` as a name in Japanese (an empty
    string) and address 2 in English. The game sends it, nothing opens,
    and the queue empties. The port had no table for her and held the
    queue, so after Sora's first skill no skill word was heard again.
- **Quarantine**'s jump table runs to 21 (QUA 0x00252710):
  - In Japanese, Tsukasa, Subaru and Sora read row 0, or row 1 when the
    type bit is clear.
  - In English, with the bit clear they subtract 150, 123 and 123. With
    it set, Tsukasa reads row 0, Subaru subtracts 96 and Sora 6.
  - The port took Infection's rule (less 6), which gave the wrong lines.
- **Outbreak**'s table still runs to 18, so less 6 is right there.

**The fix.**
- `piney-gen` reads both arrays for every `charTbl` row (18 on
  Infection, 21 on the others). A NULL becomes `file: None`. A name at an
  address no file maps (Mutation's English address 2, the kernel's
  memory) is an empty name, one the disc does not have.
- `skill_memory` becomes the merged reach of every table, stretch after
  stretch, so Sora's rows on Mutation are read where the game reads them.
- `skill_row` takes the volume and the language.
- The driver holds the queue on `None` and otherwise sends the row.

**Checked.**
- `tools/sound_ee.py words-fixture` (new) runs the game's `ccWordsPlay`
  and `skillVoicePlay` with GCMN.PRG in eemu, for every character, every
  skill 6-303 and both languages on all four discs, then one of Kite's
  words to show whether the queue held.
- `crates/piney-audio/tests/words.rs` compares the port with it: every
  line's file, offset and size, and every hold. All four pass: 10,728
  words on Infection and 12,516 on each later disc.
- `every_party_characters_skill_words` (piney-game) states the outcome per
  character from the tables.
- Also passing: piney-audio (62), piney-data, piney-gen, piney-game's
  suite (339), clippy, fmt and `piney-gen gen --check`.

The disc builds' `PINEY/TABLES/voice.bin` take the new layout at the next
piney-build.

Docs: docs/engine/sound.md (the skill words), BUGS.md.

**Still unknown:** nothing
