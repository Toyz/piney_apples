---
number: 506
title: Drained forms are the game's: an ordinary foe's is its race's 50-HP head, a Data Bug's its whole base form, three barring blows
date: 2026-10-26
area: battle
files: crates/piney-battle/src/enemy_ai.rs, docs/engine/battle.md, BUGS.md
started: 2026-10-26T01:43:58-06:00
took: 14m
---

# 506. Drained forms are the game's: an ordinary foe's is its race's 50-HP head, a Data Bug's its whole base form, three barring blows

BUGS.md: "drained forms feel too tanky". The entry was left "not
reproduced, needs a --pad-log". The code settles it: the drained forms
are the tables' rows, and the port's drain path is the game's.

**The path, checked against the game.**
- `ccGetDrainId` (gcmn 0x0042e3c0): `test_battle_enemy_ai_rs.py`, on
  random rows, with the rows patched too.
- `entryDrainEnemy` (0x004359e0): `entryObject(ep, 1)` with the row
  changed, transparency 1, `condition.dead` 1, `drainCnt` 60 or 30, and
  `effAfterDrain`. No HP or level is touched after `initEnemy`. It is
  compared in `test_battle_spawn_rs.py`'s `frame_enemies`: the game's own
  `ccEnemy::main` drains foes there, and every character is compared each
  frame.
- `CalcBattleDamage` with Exdefense: `test_battle_rs.py` on all four
  volumes (worklog 307).

**What a drain leaves.** `drained_forms_by_volume` (new, piney-battle)
lists them on all four discs:
- **Ordinary foes.** Each of the 23 race heads is 50 HP at level 0, with
  no Exdefense, except:
  - Astro Prince (288), 1,000 HP from Mutation on;
  - the head of rows 203-206: unused `AAAAAAA` rows (20,806 HP, level 99)
    on Infection and Mutation, Quarantine's event foe The Bracelet (3,000)
    there.
- **Ordinary drains in play.** In `a_drained_form_falls_to_kites_blows`,
  a level-1 Kite took 19 of a drained Gremlin's 50 HP with a blow.
- **Data Bugs.** A Data Bug (a middle boss) leaves its base form at full
  HP. There are 31 of these, at 250 to 7,890 HP and levels 5 to 98.
  Most bar an element. House Golem (52), Earthman Drill (53) and Sand
  Mountain (109) bar blows (Exdefense 0x1): ATTACK and the arts do 0
  until their pDef falls below the row's. These are the drained forms
  that feel tanky, and they are the original's.

Checked: piney-battle (156) and piney-game's suite (339); clippy; fmt.

Docs: docs/engine/battle.md (the spawn's census), BUGS.md.

**Still unknown:** nothing
