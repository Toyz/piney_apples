---
number: 168
title: The other enemy races: every constructor, and the motion of 2, C, F, H, I and U
date: 2026-09-26
area: world
files: crates/piney-battle/src/races.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, crates/piney-game/src/fx.rs, tools/test_battle_spawn_rs.py, tools/test_battle_enemy_motion_rs.py
---

# 168. The other enemy races: every constructor, and the motion of 2, C, F, H, I and U

The port made only the enemy races of the opening's field and dungeon (B,
G, K, P, V). Any other row got `initEnemy` alone and did nothing in its
`action()`, so it stood still. That was fine for the tutorial, but not for
the Delta server's other areas. Their lists (`ccEnemyListInfo`, gcmn
0x005da200) register most of the 22 races. Ranked by where each first
appears: F at rank 6, `ccEnemy2` at 7, C at 9, U at 20, then I, H, L, W,
D, E, T, A, 4 and S. Swordmanoid (row 1, `ccEnemy1`) is registered from
rank 4. Its motion was ported, but its constructor was not. The docs are
in `docs/engine/battle.md`: "The race constructors" and "Enemy movement
and animation".

**The constructors.** Each race's constructor is the same code with other
tables:

- `eneType` by row;
- the weapon and dust controllers by type (by row in a few races);
- `anmFlag` and `frameNum`;
- the clean-up hook.

So the tables were read off by running each constructor in eemu for every
row. `races.rs` now holds one `(type, weapon, dust)` row table per race,
generated from that run, for all races but L. The extras:

- the gold goblins, as before;
- the spellcasters' reweighted attacks, which `ccEnemy3` and `ccEnemy4`
  types 1 and 2 share with G type 4 (the first comparison caught them);
- H types 3 and 4 make two fire breaths (`Out::Breath`).

L's constructor sets up lights and scaling and waits for its own worklog.

**The motion** of six races, read from their code:

- `ccEnemy2`: B's moves, with its own escape and the walk's turn of
  `ccEnemy1`.
- C: the crabs move as B. The scorpions (type 3, `moveECS`) slide, and a
  stop spins them to a heading kept at +0x340. The spin rattles, and dust
  flies from the four wheels through the model's world matrix. That
  matrix is new in the port, `Enemy::anm_lw`: what the last draw left in
  the anm's `ccCoord` +0. The harnesses' draw stubs now copy the drawn
  matrix there, as `_SetLWMatrix` does for a root.
- F: moves as P. Minnow lunges, and the rays dive by `yoffs`.
- H: B exactly (`moveEH` is `moveEB`), plus the breaths' calls.
- I: moves as P. Wiggle Snake lunges for 24 frames beyond 250.
- U: every frame the target's heading is cut to its top bits
  (`ccRand() & 3`). The wander rates go by type. The tails escape tries
  the escape once to see which way it turns, undoes it, and turns aside
  by up to 3pi/4 first. Death Head lunges beyond its `atkRangeA` and bobs
  by `80 + 30 sinf(radCnt)`.

The notes' dust and `exclusive()` also differ by race: 2, C and H drive
only the weapon trail on notes, and `ccEnemy2` never runs a dust
controller. New calls: `Call::EffDust` (the wheels' dust, now shown by
piney-game), `BreathCtrl` and `BreathSet`.

**Checks.**

- `tools/test_battle_spawn_rs.py race`: 300 cases cover all 278 rows of
  the 21 races, field by field, controllers and breaths included. All
  equal.
- `tools/test_battle_enemy_motion_rs.py` now picks rows of every ported
  race: `action` 1,200 cases, `excl` 600 (with cases aimed at H's breath
  attack, the scorpions' spin and the rays' dive), `note` 300 and `main`
  400. All equal.
- `frame_enemies` spawns the new races' rows (but the drained forms and
  the middle bosses) in 60% of the cells: 120 cases, all equal.
- 609 workspace tests pass.

**Still unknown:** Two things are not ported. The breath itself
(`ccEnemyBreath`: the flames move, burst with `ccRandF` and `effSmoke`,
and are drawn) is handed to the world and not drawn, so the runtime's
`ccRand` stream lacks its bursts' draws while a Cerberus breathes. The
motion of L, W, D, E, T, A, 4 and S is not ported; those enemies still
stand. How often the breath's `setBreath` and the scorpions' spin come up
in the multi-frame runs was only counted roughly: the breath's calls did
come up.
