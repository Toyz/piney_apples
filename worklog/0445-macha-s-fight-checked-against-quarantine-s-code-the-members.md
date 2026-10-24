---
number: 445
title: Macha's fight checked against Quarantine's code: the members' AI, the battle rules and the new game's party agree, and the members' lines were Outbreak's on the new voices, misplaced on two discs
date: 2026-10-24
area: battle, volumes, tooling
files: crates/piney-battle/src/ai_move.rs, crates/piney-battle/src/party_chat.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/party_chat.rs, crates/piney-demo/examples/demo_probe.rs, tools/test_battle_party_ai_rs.py, tools/test_demo_rs.py, docs/engine/battle.md
started: 2026-10-24T03:11:32-06:00
took: 25m
---

# 445. Macha's fight checked against Quarantine's code: the members' AI, the battle rules and the new game's party agree, and the members' lines were Outbreak's on the new voices, misplaced on two discs

[[444]] left the no-aid fight with Macha lost. Before tuning the pilot
further, this checks whether the port makes the fight harder than the
game does.

## What was checked, on Quarantine's code

- **The members' battle AI.** `tools/test_battle_party_ai_rs.py` runs
  every `ccAI` function natively against the port. That covers heals,
  cures, Resurrect, item use, skill choice, `Brains`, `ReadSysMsg`, the
  chat commands, and the cures for charm and confusion. With
  `PINEY_VOLUME=quarantine`: all 75 checks, 200 cases each, no
  mismatch.
  - `FellowUseSkill` had stopped in the interpreter on Outbreak and
    Quarantine. The carry names `ccFellow::UseSkill` only up to
    Mutation, so the harness called address 0. It now takes the function
    from `ccFellow`'s vtable (slot +0x28). 100 cases agree on both discs.
- **The battle rules.** `tools/test_battle_rs.py bulk 300` covers damage,
  protect, exdefense, skill and area damage, recovery, cures, holds,
  condition modifiers, condition success, levels, experience and
  `EntryAffect`: no mismatch. The fellow, Kite, item, flow and boss
  harnesses all pass. The fellow and Kite harnesses randomise charm and
  confusion.
- **The party at 307's start.** `ccSaveData::NewGame` gives the levels,
  stats and equipment (`charTbl`). `test_new_game` now agrees with the
  game, byte for byte, on all four discs.
  - It had failed from Mutation on, but only in the probe: `demo_probe`
    built Infection's tables whatever the disc, and the harness cleared
    only 17 and 18 name slots. Both now follow the disc (21 characters
    from Mutation on).
  - The events give no levels or equipment: the only stat op is
    `exp_on`/`exp_off`.

  So Kite at 70, BlackRose at 70 and Balmung at 75, in their new-game
  gear, are the game's own new-game party at 307.

## What was wrong: the members' lines

`tools/test_battle_chat_rs.py` failed on Quarantine alone, in 30 of its
functions. Four port bugs, all in what members say, none in what they
do:

- **Quarantine has three more voices.** Its tables hold 22 rows a
  personality, not 19 (66 for the three-variant ones). Rows 19 and 20
  are its new members' spellings ("U 907 hi7 700 MuCh!", "Ai lure cover
  Ure HP!"). Mutation and Outbreak keep 19 with a null pad. piney-gen's
  `by_char` reads 22 on Quarantine.
- **`MessageIndex` gives 21, not 18,** for character 1 while `saveData`
  +0x220d is set (QUA gcmn 0x004a5ec8). The port gave 18 on every disc.
- **The carry misplaces some tables.**
  - Outbreak's `attackSmallHitMessages` landed on the next table.
  - Quarantine's `bullMessages` landed on `dashForEnemyMessages`, and
    `use_physical_skill_accept`, `use_magic_skill_accept` and
    `buff_accept` were placed wrongly too.

  On every disc the tables lie end to end in Infection's order, each
  padded to 16 bytes. Quarantine's `ChatMessageAttackTarget` reads
  0x005794e0, 0x005795f0 and 0x00579700, the addresses that order
  gives. piney-gen now places each table from the first by the sizes
  before it (`CHAT_CHAIN`, `chained`).
- **Quarantine's members 18-20 also speak of a crowd and of a foe's
  tricks** in `ChatMessageAttackTarget` (QUA gcmn 0x004a5acc,
  0x004a5b80).

After the fixes, the chat harness passes 500 cases on Outbreak and
Quarantine and the unit tests on all four discs. GOTO_RECORD_SHOP is no
longer the same on every disc, so its shared static is gone.

## So

Nothing found makes Macha harder than the game: the party, its AI and
the rules are the game's. The fight is won by playing better or by
being stronger, as a player wins it.

**Still unknown:** the members' in-session frame order around the boss
(the AI runs before or after the boss's `Main`) is checked only by
`test_battle_frame_rs`'s one case. That is not Quarantine's own frame.
