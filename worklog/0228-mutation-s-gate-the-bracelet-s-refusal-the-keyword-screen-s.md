---
number: 228
title: Mutation's gate: the bracelet's refusal, the keyword screen's marks and levels
date: 2026-09-27
area: volumes
files: crates/piney-fieldui/src/menus/gate.rs, crates/piney-fieldui/src/words.rs, crates/piney-fieldui/src/tables.rs, crates/piney-gen/src/manifest.rs, tools/test_fieldui_rs.py
---

# 228. Mutation's gate: the bracelet's refusal, the keyword screen's marks and levels

The field menus' harness had 14 failures on Mutation. Five of them were
the Chaos Gate, which the story goes through. Each difference below was
found by running Mutation's own code against Infection's.

## The keyword screen's level (`ccAnalyzeEnemyList`)

Mutation's `ccAnalyzeEnemyList` (gcmn 0x00444c20) reads
`ccRegisterEnemyRange`. `ccInitRegisterEnemy` sets it to 3, and nothing
else writes it. If the list is too short for three rows from the rank,
the window moves back to the last three rows, where Infection repeats the
last row.

- piney-battle's copy already had this rule. The field menus' copy
  (`Words::analyze_enemies`) now has it too.
- The harness never ran the init, so the game read an unset range and
  wrote stack garbage into `areaLevel`. It now sets the range to 3.

## The keyword screen's marks (`GtNewMenuDisp`)

The attribute marks test each of a word's nine fields against 255.
Mutation tests the dungeon size (field 1) against 0 instead, and its
colour treats a 0 as unset. The empty slots' placeholder word has a
dungeon size of 0, where Infection's has 255.

## `GtNewMenu`

- **Kanji alpha.** Proccess 7 (the address shown), and OK in proccess 8,
  set `kanjiAlpha` to -24, so the words fade in again.
- **The refusal.** The new-keyword, random, Word List and history menus
  each gained the same check in their Warp. If `eventStatus[40]` is set
  (the bracelet off, which already hides Data Drain), and the area's
  `GetEventAreaNumber` is on a list of 16 story areas (101, 92-99, 118,
  119, 123, 108, 16, 66, 91), the menu goes to proccess 30 instead:
  1. **Proccess 30.** The target is put back on the gate, and the dim and
     the window go out.
  2. **Proccess 31.** After seven frames, Kite's line opens (a bad
     feeling, so not now), with no voice.
  3. **Proccess 32.** When the line closes, the menu shuts.
- **The data.** piney-gen reads the check from `GtNewMenu`'s code:
  - the `lb` of `eventStatus[n]`;
  - the table and the `$gp` count its loop runs over;
  - the record built into `$a1` for `ccMessage::Open`.

  The result is `fieldui::gate_refusal`. Infection has none, and neither
  do Outbreak's and Quarantine's `GtNewMenu`.
- **The harness.** A new scenario, `test_gate_refusal`, covers this: the
  Word List's area 16 with the bracelet off. The game goes through
  proccesses 30, 31 and 32, and the port matches it frame for frame.

The harness now has 9 failures on Mutation and none on Infection.

**Still unknown:** The remaining failures:
- Eight are the tutorial and item windows. On Mutation they open four
  lines where the port opens one, because the port reads Infection's
  events 2-4 for those records.
- One is `interNoiz`, which Mutation does not burst in the scenario the
  harness sets up.

Neither blocks the story. What sets and clears `eventStatus[40]` in
Mutation's story has not been followed.
