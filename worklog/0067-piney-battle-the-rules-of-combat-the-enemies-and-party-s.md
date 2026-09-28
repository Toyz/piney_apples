---
number: 67
title: piney-battle: the rules of combat, the enemies' and party's decisions and items, checked against the game
date: 2026-09-23
area: battle, test
files: crates/piney-battle, tools/test_battle_rs.py, tools/test_battle_flow_rs.py, tools/test_battle_drain_rs.py, tools/test_battle_items_rs.py, tools/test_battle_enemy_ai_rs.py, tools/test_battle_party_ai_rs.py, docs/engine/battle.md
---

# 67. piney-battle: the rules of combat, the enemies' and party's decisions and items, checked against the game

The goal needs the opening's fights. [[22]] and [[25]] had worked out
Infection's battle formulas in Python (`tools/battle.py`). A battle agent
and three helpers have now ported the rules half of combat to Rust as a
pure-logic crate, `piney-battle` (about 14,000 lines). It has no drawing,
animation or movement: those are the runtime's.

## What it holds

- **The formulas.**
  - `CalcReal`;
  - `CalcBattleDamage` with the protect gauge and Exdefense;
  - `ccSkillDamage`'s four forms and `ccSkillDamageValue`;
  - recovery, cures, hold and condition changes;
  - experience, levels and erosion;
  - `EntryAffect` with each kind of character's `affectFunc`.
- **A skill's life.** `_ccSkillRequest` through `ccSkill::Main`
  (`ccThSkill`) with its systems, `NoteEventAffect`, and the player's and
  fellows' attack notes.
- **Data Drain.** `DataDrainMenu`'s steps: the drain, the side effect
  and level down, and evolution.
- **Items.** All of `useitem.cpp` and the save's item lists.
- **The enemies' decisions.** `enemy.cpp`'s think, attack, skill and
  target choices, and `genrand` (the enemies' own MT19937).
- **The party's decisions.** `ccAI`: `Brains`, `Reconnoiter`,
  `ActInField`, `ActInDungeon`, healing, attack choice, chat commands, and
  its message bus.
- **The shape.** Each rule returns events in the game's call order. A
  `Scene` stands for the three command lists; `Rand` is newlib's `rand()`
  and `Genrand` the enemies' generator.

## Bugs found while merging

- `_ccSkillCheckType` had 1 and -2 swapped: 1 is an attack spell, -2 a
  debuff.
- `ccSkillDamageValue` takes both characters' list checks.
- A cure cast on oneself lost its SP change.
- `ConditionAdjustment` does not run every frame.

## Checked

- **Against the game.** Six harnesses run the game's functions natively in
  eemu beside the port. Each compares the return value, every character
  and AI field, both generators' states, and the calls in order. There
  are 125 checks, each over at least 1,000 random cases, with 0
  mismatches. The core, flow and drain checks also passed 10,000 cases
  each.

  | harness | covers |
  | --- | --- |
  | `test_battle_rs.py` | damage, skills' effects, stats, levels, conditions, experience, erosion, `EntryAffect` |
  | `test_battle_flow_rs.py` | `ccSkill::Main`, notes, `ccSkillCheck`, `ConditionAdjustment`, level ups, `AreaItem` |
  | `test_battle_drain_rs.py` | `DataDrainMenu` steps 0-1, 10, 20 |
  | `test_battle_items_rs.py` | `useitem.cpp` and the item lists |
  | `test_battle_enemy_ai_rs.py` | `enemy.cpp`'s decisions, `genrand` |
  | `test_battle_party_ai_rs.py` | `ccAI` and its message bus |

- **Re-run in a clean worktree.** I re-ran all six there, with the older
  `test_battle.py`: all pass. The workspace's tests pass, and clippy, fmt
  and the docs check are clean.

**Still unknown:**
- **Not in the runtime yet.** The crate supplies the rules; nothing in the
  field calls it. The runtime must still supply:
  - the characters' animation and notes;
  - enemy spawning and portals (`ccThEntryCtrl`, `entryEnemy`,
    `entryMagicCircle`);
  - enemy movement and each race's `action()`;
  - party movement (`FollowPlayer`, `FollowTarget`, `FollowBeacon`,
    `ActInTown`) and `ccFellow::Action` / `Main`;
  - the spells' element systems;
  - the Data Drain movie and transformation;
  - damage numbers, hit marks and sounds.

  `docs/engine/battle.md` "What the runtime does" lists each with its
  address.
- **Not ported:**
  - bosses: `_ccBossSkillDamage`, their affects and AI;
  - `ccEnemyG::thinkGold`;
  - the chat text;
  - `ChangeEquipReport`.
- **Not checked.** `party_ai::Ai::new` has no check.
- **Other volumes.** The port is checked on Infection only.
