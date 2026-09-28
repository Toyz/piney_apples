---
number: 221
title: Mutation's grown tables and its battle rules
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/manifest.rs, crates/piney-gen/src/locate.rs, crates/piney-gen/src/layout.rs, crates/piney-battle/src/chara.rs, crates/piney-battle/src/damage.rs, crates/piney-battle/src/exp.rs, crates/piney-battle/src/param.rs, crates/piney-battle/src/party_ai.rs, tools/volume.py
---

# 221. Mutation's grown tables and its battle rules

The Mutation battle harness (`tools/test_battle_rs.py` with
`PINEY_VOLUME=mutation`) failed six of its nine checks. All nine pass
now, and they still pass on Infection. Some failures were tables the
generator read with Infection's row counts. The rest were rules Mutation
changed.

## Tables that grew

Several manifest entries had fixed row counts. On a later volume that
cuts the table short. The carry's gap to the next global showed which
tables grew, and the rows past Infection's count were checked by hand.
Outbreak and Quarantine have the same counts.

| table | Infection | from Mutation on |
|---|---|---|
| `npcTbl` | 175 | 184 (five event NPCs, Mimiru, Bear, Crim, A-20) |
| `gimmickTbl` | 45 | 46 (`PG_FLAG`) |
| `BossSkillTbl` | 61 | 79 |
| `LevelUpParamTbl`, `spcDefaultItemList` | 18 | 21 |
| `npcTradeRateTbl` | 48 | 54 |
| `spcTradeRateTbl` | 17 | 20 |
| `spcMsgTbl`, `spcMsg0`-`5`, `spcMsgPresent10`/`11` | 18 | 21 |
| `debuffTableIndex`, `buffTableIndex` | 18 | 21 |
| `equipmentWeapon1`/`4`/`6Tbl` | 82/75/76 | 83/76/77 |

The weapon tables were the cause of `calc_real`'s failures.
`spcAIParam` stays at 18 on Mutation and Outbreak. Quarantine's gap
fits 21; that waits for its turn.

Two generator changes came with the counts:

- **Locating by Mutation's layout.** The carry places Outbreak's and
  Quarantine's `spcMsgPresent10` in text, and has no row for
  `spcMsgPresent11`. The Present menu changed there, so the code finder
  misses it too. `locate::neighbour_candidates` now adds one more
  candidate on those two volumes: Mutation's distance from a neighbour,
  applied to the volume's own place for that neighbour. A block that grew
  on Mutation keeps Mutation's spacing. The likeness vote picks among the
  candidates as before.
- **Entry functions Infection lacks.** `gimmickTbl` row 45's entry
  function has no carried symbol. `layout::LATER_FUNCS` names it
  `ccEntryGimPgFlag`, with its address on each later volume. That gives
  `EntryFunc::GimPgFlag`. The port does not build this gimmick yet (its
  `construct` returns None).

## Rules Mutation changed

- **`ccChar::DispConditionEffect`.** A dead character, or a hidden party
  member, now gets `conditionNum` -1 even when it has no effect.
- **Exdefense in `CalcBattleDamage`.** On Infection an enemy is immune to
  every Exdefense kind while any defence the skill works against is
  unbroken. From Mutation on it is immune only to the kinds still held,
  and to none when the row has bit 0x100.
- **Experience.** `ccExpDistributor` and `ccSpcCheckLevelUp` loop over 21
  characters, not 18. The port loops over the volume's `chars`.
  `piney_battle::tables::CHARS` is gone.
- **`ccSpcChar::CheckAction(0)`.** From Mutation on it also holds during a
  normal attack (skill 0 or 1), whatever the skill's status.

`SpcParam::from_save` and `store` read only `ccSaveData`'s own bytes, so
characters 18-20, whose records are in the extension, panicked. They
read the whole record now.

## The harnesses' layouts

Two structs changed shape on Mutation. The harnesses write them with
Infection's offsets, so `tools/volume.py` now maps those offsets:

- **`ccSkill`** (`SKILL_SIZE`, `skill_at`) grew from 0xb0 to 0xc0.
  `_ccSkillRequest` stores the target's position in a new vector at
  +0x70. The creator and everything after it move 0x10 on.
- **`ccAI`** (`ai_at`) keeps its size, but three fields are new:
  - an int at +0x24, which moves `chatText` to `gDeg` 4 bytes on;
  - two shorts at +0xa0, which move `gRotSp` to `noTurnRange` 8 bytes on.

  Nothing from `posOld` on moves.

The flow, items, party AI, chat, event, fellow and navi harnesses use
these helpers. `test_battle_flow_rs.py` now passes on Mutation.

**Still unknown:** Mutation's `ccAI::UseItem` changed further:

- it checks `CheckAction(7)`;
- a held member may use items unless a boss is in (then it returns -1);
- the member remarks on the use from two new chat tables.

There are four more new tables: physical and magic tolerance, and only
one buff or debuff. The code that reads the new `ccSkill` vector and
`ccAI` fields is unread. The items, fellow, kite, party AI, enemy AI,
spawn, event, chat and drain harnesses still fail on Mutation.
