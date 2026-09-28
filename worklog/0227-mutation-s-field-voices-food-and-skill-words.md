---
number: 227
title: Mutation's field voices, food and skill words
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/voice.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-data/src/sound/voice.rs, crates/piney-game/src/mode.rs, tools/sound_tables.py, crates/piney-audio/tests/voice.rs
---

# 227. Mutation's field voices, food and skill words

Reported from play: on Mutation, the party members' lines were silent in
game (Black Rose had text but no voice). Worklog 226 fixed the event
voices. The field voices, the food's and the skill words still came from
Infection's tables on Infection only, which `tools/sound_tables.py` wrote
by name. They are now piney-gen's `voice` group for every volume, in
Japanese and English, and the Python generator no longer writes any voice
table.

## What piney-gen reads

- **`field`: `ccVoiceRequest`'s cases.**
  - The dispatch is a run of `li r, group` / `beq $a0, r`.
  - In each case, the `lb` of `saveData.voice` (+0x842c) and the `bnez`
    after it mark where the English branch starts.
  - Each `lui`/`addiu` pair builds a table. Its file is the first
    `voiceFile` store after it.
  - A case has one Japanese table and one English table, which set the
    same file.
- **`food`: `ccVoicePgFood`'s table**, read the same way. It is the same
  on every disc, so it is written once (`voice::FOOD`).
- **`skill`: `skillVoicePlay`'s tables** by `charTbl` row, up to
  `spcVoiceData`'s NULL. The game gives up on a character with no file
  before it looks at the rows.
  - Mutation's `voiceData` has three more rows than files, which the game
    cannot reach.
  - A skill table ends with its (0, 0) row, which is kept.
- **`files`, `files_e`.** These run to a NULL or the next symbol.
  Mutation's `evVoiceFile` runs straight into `evVoiceFileE`, which gains
  `MIAE.BIN` (20).
  - The names are written as paths on the disc (`VOICE_E/PARTY_E.BIN`),
    not as the IOP's `cdrom0:\...`, so the port no longer strips them at
    run time.
- **Row bounds.** A table runs to the next table built in these
  functions, or to the next symbol if that comes first. A row is a line (a
  whole-sector offset and a positive size) or none (-1, -1).

On Infection, every field, food and skill table comes out the same as the
old tables, row for row (checked by a comparison test before the old
tables were removed).

## Mutation's differences

- **Mia's English lines.** The party, talk and present groups call a new
  getter of `talkNum` (MUT 0x0017abf0, 18-20 in the extension) with 1.
  While `talkNum[1]` is set, English messages 3-5 (party), 2-3 (talk) and
  5-9 (presents) play rows 0-2, 3-4 and 5-9 of one 10-row table from
  `MIAE.BIN`.
  - piney-gen reads this as `VoiceAlt`: the two `slti` bounds, the
    getter's index, the addend to the message (-3, +1, 0), the file and
    the rows.
  - `Event::VoiceOptions` now carries `talkNum` for rows 0-20 from the
    save (`mode::voice_options`). The field menus' requests send it fresh
    each time.
- **Table sizes.** The Japanese party, talk and present tables grew to
  63, 42 and 105 rows. The English ones kept 54, 36 and 90. An English
  message past its table reads the next table's rows in the game; the port
  plays nothing, as before.
- **Loading.** `ccVoiceRequest` and `ccWordsPlay` return at once while
  `ccSnd +0x139` is set, from `ccFileListLoad`'s first file to its last.
  This is not ported, because the port loads between frames.
- **Skill words.** `skillVoicePlay` and its row rule (the jump table's
  cases and bases) are Infection's.

`BOSSTALK.BIN` (Fidchell, -40) is on neither Infection's disc nor
Mutation's.

## Checks

- Every field, alternative, food and skill line on Infection's disc and on
  Mutation's lies inside its file, in both languages
  (`every_field_line_is_in_its_file`,
  `every_mutation_field_line_is_in_its_file`).
- Mia's English party, talk and present lines come from `MIAE.BIN` only
  while `talkNum[1]` is set, and never in Japanese
  (`mutation_mia_speaks_from_miae_once_talked_to`).
- The eemu voice fixture (every Infection row and group, both languages,
  the skill words) still matches, with its file names read as disc paths.

**Still unknown:** What `talkNum[1]` stands for on Mutation, and what
sets it; the port only reads it. Outbreak's and Quarantine's `spcVoiceData`
list 21 files: 18-20 have 3-row Japanese tables, and their English
entries point at full-size tables. These are to be checked on those
volumes. Where the English party tables are short, it is not known whether
the game really plays the next table's rows.
