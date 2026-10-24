---
number: 470
title: A kill's experience is the game's: ccExpDistributor, the level table and the level-ups agree natively on all four volumes, so Infection's slow levels are the pilot's few kills
date: 2026-10-24
area: battle, test
files: crates/piney-battle/src/exp.rs tools/test_battle_rs.py tools/test_battle_enemy_ai_rs.py
started: 2026-10-24T17:53:37-06:00
took: 3m
---

# 470. A kill's experience is the game's: ccExpDistributor, the level table and the level-ups agree natively on all four volumes, so Infection's slow levels are the pilot's few kills

## The check

[[469]] found Infection's no-aid party at level 2 by event 14. A
level-1 member gets 80-100 of a level's 1,000 experience for a kill of a
level-3 or level-4 foe. The question was whether that is the game's, or a
port bug in the rules every volume shares.

**What the code does** (`piney_battle::exp`, docs/engine/battle.md):
- **The share.** `ccExpDistributor(level)` (INF gcmn 0x00571290) runs once
  per kill. It gives each member `expCalcTbl[clamp(foe level - member
  level, -10, 10) + 10]`: 80 at +2 and 100 at +3. Nothing is split
  between members.
- **The bonus.** The foe's level gap is the only bonus, through the
  table. The foe row's own `exp` is never read.
- **Away members.** A member away from the party gets 60%, when the save
  flags him.
- **Levels.** `ccChar::CheckLevelUp` takes 1,000 a level.

**Checked natively, on all four volumes** (`PINEY_VOLUME`):
- `tools/test_battle_rs.py bulk 1000 exp level erosion`: 1,000 random
  cases each of `ccExpDistributor`, `CheckLevelUp`/`LevelDown` and
  `AddLvErosion`, with random levels 0-99, parties and away members. All
  agree.
- `tools/test_battle_enemy_ai_rs.py bulk 300` on Infection: the death
  routine's `ccExpDistributor` call and its level among the enemy AI's
  checks. All agree.

So the rules are the game's. Infection's slow levels come from the
pilot's few kills: one visit's portals bring about two levels.

**Still unknown:** nothing.
