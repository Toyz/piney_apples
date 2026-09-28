---
number: 171
title: "ccEnemyL: the snakoids, wyrms and dragons"
date: 2026-09-26
area: world
files: crates/piney-battle/src/races.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, tools/test_battle_spawn_rs.py, tools/test_battle_enemy_motion_rs.py
---

# 171. ccEnemyL: the snakoids, wyrms and dragons

`ccEnemyL` (race 14, rows 193-217) was the last race standing still after
worklogs 0168 and 0169. The Delta server's lists register two of its
rows: Snakoid (194, from rank 29) and Red Wyrm (199, from rank 56).
Dragon Puppy (193) is its drained form. `docs/engine/battle.md` has the
rules.

**What is ported:**

- The constructor for types 0, 1, 2 and 4. Types 2 and 4 make a breath
  from `elBrInfo`. Every type draws `ccRandS` four times into +0x390, for
  the type 3's colours. Those draws move the generator that the gold
  goblins' `checkGold` also draws, so they are kept (`Enemy::l_rand`).
- `moveEL` for the snakoids, `moveEL2` for the wyrms and dragons. The wyrm
  and dragon attack follows the target's heading with or without a
  target.
- `exclusive()`: the wyrms' breath, and the dragons' rise and their two
  breaths. `Call::BreathSet` now carries the breath's parameter row.

**What is not.** Type 3 (rows 203-206, named "AAAAAAA" to "DDDDDDD" in
the table) stays unported. Its constructor runs `initELG`, which
disables interrupts in eemu. It moves by `moveELG`, draws itself through
`dispELG` with its own lights, scaling, colour and texture animation,
and clears `dispSW` every frame. No list registers it. `construct`
returns `None` for those rows (the seam's bare `initEnemy`).

**The harnesses:**

- The spawn harness knows L's object size (0x3b0), hooks `ccDestEnemyL`
  (whose `delLight` crashed the interpreter on a dragon's removal) and
  records breaths by their info.
- The motion harness carries a breath flag per enemy, keeps L's random
  types to 0, 1, 2 and 4, and keeps rows 203-206 out of its extra
  enemies.

Results, all equal:

- `race`: 300 cases (299 rows).
- The motion harness: `action`, `excl` and `note` 800 cases each, `main`
  500.
- `frame_enemies`: 150 cases.

**Still unknown:** L's type 3 as a whole: what rows 203-206 are, and
whether any event places them. The breaths are still not drawn (GAPS).
The wyrms' and dragons' `setBreath` did not come up in a 300-case count
of the `excl` check: the random states seldom line up attack 4, a target
and a breath, so that branch is read, not measured.
