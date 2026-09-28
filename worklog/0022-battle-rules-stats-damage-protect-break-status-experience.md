---
number: 22
title: Battle rules: stats, damage, protect break, status, experience, checked against the game code
date: 2026-09-22
area: battle, decomp, test
files: tools/battle.py, tools/test_battle.py, tools/eemu.py, docs/engine/battle.md
---

# 22. Battle rules: stats, damage, protect break, status, experience, checked against the game code

The tables of [[12]] gave the numbers - `charTbl`, `enemyTbl`, `skillTbl`,
the equipment tables - but not what the game does with them. This entry
reads the `ccChar` and `ccSkill` code in `gcmn.prg` and reproduces it in
`tools/battle.py`, checked the same way as the generators of [[11]] and
[[19]]: the game's own function runs in `tools/eemu.py` on random inputs and
every output is compared. The rules are on
[the battle page](../docs/engine/battle.md).

## What was reproduced

- **Effective stats** (`ccChar::CalcReal`, `INF gcmn.prg:0x0056ba30`): a
  party member's five equipment pieces are summed onto the base stats with a
  clamp to ±999 after every addition, then the buffs. A foe's table row plus
  buffs clamps to 0..32767.
- **Hit and damage** (`ccChar::CalcBattleDamage`, 0x0056d910): the roll is
  `rand() % 101` and hits run from 50 to 100. Damage is `a²/(2d)` scaled by
  the hit, plus an element term per element bit, times `dmgRate`, then times
  a float `mag`, capped at 9999. Also: absorption at attribute 999,
  Exdefense immunity that a debuff lifts, and the equipment effects
  (critical, dying, drains, invincible).
- **The protect gauge**: the same formula against `pDefPP`/`mDefPP`, filling
  PP. At `maxPP` it breaks for 300 frames. Bosses scale by hit/80 and cap
  physical hits before `mag`.
- **Status** (`_ccSkillModifyCondition` 0x00575cf0,
  `ccCheckConditionSkillSuccess` 0x00571830, `ConditionTimeCount`
  0x0056bfd0): durations, resistance, and the per-frame poison, regeneration,
  curse and natural SP.
- **Healing and cures** (`ccSkillRecovery` 0x00574bc0, `RecoverySystem`
  0x00577070).
- **Experience**: 1000 per level. Exp per kill comes from `expCalcTbl`
  (`INF gcmn.prg:0x006518f0`), indexed by the level difference; the enemy
  table's own `exp` field is never read. Level-up adds `LevelUpParamTbl`
  rows. Infection (`AddLvErosion`, `INF SLUS_202.67:0x001787a0`) rises by
  `erosionTbl` times 1, 1.5 or 3 depending on the drain skill.

The RNG is newlib `rand()` (`INF SLUS_202.67:0x00133a38`): a 64-bit LCG with
multiplier 6364136223846793005 (0x5851f42d4c957f2d, built in four halves
before `__muldi3`), returning bits 32-62. It is a different generator from
the area RNG.

## eemu change

`Machine.call()` passes up to eight integer arguments in `a0-a3` and `t0-t3`,
as the EE ABI does, instead of four. `ccSkillDamage` takes five. Calls with
four or fewer behave as before, and every other test passed after the change.

## Checked

`tools/test_battle.py bulk N` runs 11 comparisons of the game functions
against `battle.py`. Each case compares:
- the return value;
- every field of the structs involved;
- the `rand()` state;
- the sequence of calls into stubbed effect, particle and `EntryAffect`
  functions.

The agent that wrote it ran 20,000 cases per comparison (40,000 for damage),
**240,000 in all, 0 mismatches**. The damage cases covered 17,659 hits,
18,609 misses, 4,669 protect breaks, 733 criticals, 185 dying, 1,449 drains,
246 invincible nullifications, 147 attribute guards and 735 absorptions.

Two transcription errors were caught this way:
- The equipment effects are gated on the skill's SP cost, not on
  `sk.condition`.
- A boss's magic protect damage is capped after `mag`, not before.

Deliberately planted bugs in `battle.py` also showed up as mismatches.

On integration I re-ran the bulk check at 3,000 cases per comparison: 0
mismatches. I also read `expCalcTbl` (21 u16s: 1, 2, 3, 4, 6, 8, 13, 28 ...
520) and the `rand` multiplier from the binaries myself.

## What it gives

`battle.py damage ELF Kite Goblin` works out Kite's hit against a Goblin:
- The Goblin's pEva of 4 makes `hit = roll + 4`, so rolls of 46 or more hit:
  54.5%.
- A hit does 8 to 16 damage, 6.47 per swing on average.

Other subcommands:
- `stats` - effective stats with any equipment;
- `levelup` - the level-up table;
- `exp` - exp per kill at a given level;
- `skills` - every skill's kind and effect;
- `drain` - Data Drain drop chances, read from the code.

**Still unknown:** the Data Drain drop roll (`DataDrainMenu`, 0x00533100) and
side-effect table are read, not run; `_ccSkillRequest`'s attribute-critical
roll and area splash damage are read only; the effects of sleep, paralysis,
charm and confusion on behaviour; boss skills (`_ccBossSkillDamage`); item
use; enemy AI beyond its structure; normal drops; what `noDeathFlag` and the
type bits 0x100/0x200 mean; whether `plcol` is the bracelet and the protect
break the Data Drain window (both inferred); whether volumes 2-4 changed any
of it.
