---
number: 299
title: A Data Bug drawn and held at half its HP: the area's entry copy, virusFlag on the character
date: 2026-09-28
area: battle, render
files: crates/piney-battle/src/tables.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/src/races.rs, crates/piney-battle/src/world.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session/tests/data_bug.rs, tools/test_battle_spawn_rs.py, docs/engine/battle.md
---

# 299. A Data Bug drawn and held at half its HP: the area's entry copy, virusFlag on the character

Reported in play (INF, Mistral's side quest, a dungeon's B3): a Data Bug
(an `enemyTbl` row of type 0x40) fought with no model on screen, the hits
and "DATA DRAIN OK" shown, and one hit of 1813 took its HP down; normal
hits could finish it.

## What the game does

The 45 middle-boss rows (type 0x40: 54, 55, ... 115, ... 302 on INF) have
no animation table of their own and a file named with a fourth letter `X`
(row 115 `EETX`, base form 113 `EET1`).

- **Loading.** `ccAddRequestFileListEntry` (INF GCMN.PRG:0x0042f5a0) copies
  the base form's (`base.gold`) `entry.anm` (+0x80 of the row), `clut`
  (+0x88, 30 bytes) and `fileList` (+0xa8, +0xac) into a registered middle
  boss's entry, loads that file and the name with its fourth letter `X`.
  `initEntryCCS` (0x0042feb0) sets `ccsc` and `ccsc2` from them. The same
  function is at MUT 0x00444f88, OUT 0x0043fdf8, QUA 0x00332708 (its 0x40
  test and the +0x80 copy).
- **The models.** `initEnemyCCS` (0x00432fc0) makes `CMP_trall` of `ccsc`
  playing `anm[6]` (`ANM_eet1nut0`), and for a middle boss (the only caller
  of `ccCheckMiddleBoss`, 0x004330c4) `CMP_trall` of `ccsc2` playing the
  same name with its eighth letter `x` (`ANM_eetxnut0`, in `eetx`), shadow
  off. `dispEnemy` (0x004348b0) draws that one on layer 5, then the body.
- **The HP rule.** `initEnemyCCS` sets `virusFlag` (`ccEnemy` +0x250 bit 6,
  0x004331f8-0x0043320c) when the row's `maxPP` is not -1. `affectEnemy`
  (0x00433c0c-0x00433c8c) is the only reader: damage and poison take
  `d / 10` (toward 0), and HP below `maxHP / 2` is set to `maxHP / 2`. The
  fly font (0x00433bb8) shows the whole `d`. `CalcBattleDamage`
  (0x0056d910) and the skill paths have no case for type 0x40 (no other
  function tests the bit or calls `ccCheckMiddleBoss`), so the numbers are
  ordinary and the HP stops at half: row 115 (20169 HP) at 10084. Only
  Data Drain after the protect breaks ends it. Nothing restores HP, and the
  name's garbling is the row's own name text.

## Why the port differed

- The battle tables kept the disc's empty `anm` for the middle bosses (the
  spawn check's interpreter read EE address 0xb4 as empty too, since it
  never ran the loading's copy). `races::init_enemy` set the clip "",
  `Cast::set` found no look with that clip, no actor was made, and nothing
  was drawn; the skill list was empty, so it never attacked.
- An actor's look was the first look holding its clip: row 115 would have
  drawn as 113 (no second model; rows sharing a file with an earlier row
  took its palette). The second model's clip was played on the body's own
  file, which lacks the `x` clips.
- `virusFlag` was set on the port's `Enemy`, but `affect_enemy` reads the
  character's `enemy_flags` (the same `ccEnemy` word), which nothing set.

## The fix

- `Tables::of` applies the loading's copy (`with_base_forms`).
- `init_enemy` sets `enemy_flag::VIRUS` on the character with `virus_flag`.
- A new `World::enemy_ccs(who, row)` (default nothing), called first in
  `races::init_enemy`: the field's `Cast::enemy_ccs` makes the actor from
  the row's own look and keeps the second clump's file (`second_file`),
  which the second slot's `SetAnm` and `_AnimateForward` now use.
- `FieldWorld::put_enemy` (tests and tools) puts an enemy as an event's
  `entry 5` does, loading the row's look (`Looks::add_enemies`, split out
  of `load_looks`).
- `tools/test_battle_spawn_rs.py` does the loading's copy in its scene, so
  its `race` check covers the middle bosses with their clips.

Tests: `middle_bosses_take_their_base_forms_entry` and
`a_data_bug_is_not_beaten_by_damage` (piney-battle) fail without the fix
(no clips; no virus bit). `a_data_bug_is_drawn_and_not_beaten_by_damage`
(session) puts row 115 in a field, sees it drawn with both models on 1154
frames and the console's `kill` every 20 frames leave it at 10084; without
the fix it fails with no actor. `test_battle_spawn_rs.py bulk 300` of
`race`, `entry` and `frame_enemies`: 0 mismatches (all 299 rows); the piney-game suite passes (163).

**Still unknown:**
- Which Data Bug the report met (Mistral's quest, B3) was not identified;
  the fix covers all 45 rows but none was checked in that dungeon.
- The second model's look on screen (layer 5, alpha from the camera) was
  not compared with the game's pictures; only that it is drawn.
- `tools/test_battle_enemy_motion_rs.py` still leaves the middle bosses
  out of its rows; their motion with the copied clips is unchecked there.
