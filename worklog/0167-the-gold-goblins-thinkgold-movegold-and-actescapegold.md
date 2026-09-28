---
number: 167
title: The gold goblins: thinkGold, moveGold and actEscapeGold
date: 2026-09-26
area: world
files: crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/examples/battle_probe/spawn.rs, tools/test_battle_spawn_rs.py
---

# 167. The gold goblins: thinkGold, moveGold and actEscapeGold

The goblin rows 131-138, 140-143, 147-150 and 154-157 are gold goblins
(`goldFlag`). Their constructor (`checkGold`) was ported, but they thought
with `defaultThink` and moved like any goblin. That was wrong in every
field once the portals came (worklog 0164). Now `ccEnemyG::think` sends
them to `thinkGold`, and `ccEnemyG::action` to `moveGold`.
`docs/engine/battle.md`, "The gold goblins", has the rules.

**What they do.** They run from the party and wear sleep, confusion,
charm and paralysis off by their hoard's size. A volume 2 or larger
goblin that is hit while held, or dragged more than 500 from where the
hold caught it, shakes the hold off. For one frame it sets
`game.inBattleDist` to -2 and goes off the command lists, then puts the
distance back to 2200 and comes back on them. That is a new
`enemy_ai::Out::InBattleDist`, which the runtime writes into its
`ccGame` battle state after the entry frame. `actEscapeGold` picks a heading away from the
target and a turn rate from `goldParam`, both with a little
randomness, and sets its speed from the zoom and `crisisRate`. The hold
escape overwrites `crisisRate`.

Four things came up on the way:

- **The dropped draw.** One of `actEscapeGold`'s branches draws
  `ccRandF(pi/10)` and drops the result; the generator still moves.
- **The widened ranges.** `checkGold` writes 50000 into a volume 3 or 4
  row's `area`, `territory` and `viewRange` in the table itself.
  `Out::GoldRanges` said so, but nothing applied it. The first
  comparison found a goblin 8,000 from Kite that the game's goblin saw
  and the port's did not. The port's `think()` now reads the widened
  ranges for those rows; every goblin of a row has the row's volume.
- **The dust.** A walking gold goblin's dust goes through
  `ccTransPosFW2LW` first. The runtime did this already, but the probe
  printed the raw position, one unit in the last place off.
- **Delay slots in my listings.** My no-nop listings hide the delay
  slots. An instruction that follows a branch in them is not always in
  its slot. That was checked against the full listing where it
  mattered. The same check showed that worklog 0164's note about
  `effDungeonEntrance` was wrong: its `pTexMod` store runs only when the
  colour is not -1. The result is the same (the generator starts at -1),
  and the code comment and effects.md are corrected.

**Checks.** `tools/test_battle_spawn_rs.py`'s `frame_enemies` now spawns
gold goblins as well: rows of each volume and type, in 40% of the
cases. Between frames it holds them, lets them go, puts them to sleep,
paralyses, confuses or charms them (event kind 2, which the probe
already had). `game.inBattleDist` is part of each frame's scene (left at
0x7f7f7f7f before, so a write shows). All 200 cases were equal. A
60-case sample saw every act but wander and home, and the hold escapes
set the distance to -2 and 2200. The whole spawn suite, the enemy AI and
motion harnesses and 605 workspace tests pass.

**Still unknown:** `goldVolume` outside 1-4 would leave the conditions'
wear to a register's value; `checkGold` never makes one. Acts 2 (wander)
and 4 (home) of a gold goblin did not come up in the sample. A shaken-off hold was
not played through in a session.
