---
number: 169
title: "The enemy races' motion: A, D, E, S, T, W, 3, 4 and Z"
date: 2026-09-26
area: world
files: crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, tools/test_battle_spawn_rs.py, tools/test_battle_enemy_motion_rs.py, docs/engine/battle.md
---

# 169. The enemy races' motion: A, D, E, S, T, W, 3, 4 and Z

Worklog 0168 made every race but L and moved six more. This moves the
rest of the Delta server's races but L: the golems (A), the bats and eyes
(D), the rocks and turtles (E), the snakes (S), the boxes (T), the ghosts
(W) and the witches (4). `ccEnemy3` and `ccEnemyZ` are not on the Delta
server but were a few lines each. The rules are in
`docs/engine/battle.md`, "Enemy movement and animation". They were read
with a small symbolic tracer over the listings, which prints each call to
the act helpers with its arguments.

What is new beyond the rates:

- **The mimics** (T type 1) open their lid to move (clip 11) and close it
  to wait (12). The code keys this on `anmNumOld`, which is the clip
  playing. After 48 frames of closing in, a mimic gives up and stops, and
  returns before `moveFlag` is set.
- **The ghosts** (W types 0-1) bob as the Death Head does, by `100 + 60
  sinf`. Their chase and their flinch bump the act's count on its first
  frame, and a flinch plays clip 8 only.
- **Ark Prince and its kin** (D type 3) float by `zoffs` in `exclusive()`.
  The Bat lunges at 30 for 48 frames. S's `moveES` carries the same
  row-89 test, copied from D, which never holds for a snake.
- **The turtles** (E type 3) raise dust rings at two feet, looked up by
  node name in the clump: `Call::FootDust`, which the harnesses record at
  `GetObjAdrsF`.
- **Two traps in the switches.** D's type switch has no default, so any
  type but 2 and 4 falls into `moveED`. The first comparison found this
  with a random type 6. A's escape reads uninitialised registers for types
  above 4, which no row has, so the harness keeps A's random types to 0-4
  (as it keeps `ccEnemy1`'s to 0-5).

`Kind` now names every race but L, and `note()`'s and `exclusive()`'s dust
go by race (`Kind::note_dust`, `excl_dust`).

**Checks.** The motion harness picks every race now:

- `action`: 1,500 cases.
- `excl`: 600 cases, aimed as well at D's float and E's turtles.
- `note`: 600 cases, with turtles walking on notes 1 and 2; the foot dust
  came up.
- `main`: 600 cases.

In `tools/test_battle_spawn_rs.py`:

- `race`: 150 cases.
- `frame_enemies`: 150 cases, with rows of every race but L's, the drained
  forms and the middle bosses.

All cases were equal.

**Still unknown:** `ccEnemyL` (the dragons and snakoids, rows 193-217,
from rank 29) still stands still: its constructor sets up lights, scaling
and extra skills, and waits for its own worklog. The runtime does not draw
the turtles' foot dust yet: it would need the model's node places by name.
Nor does it draw the Cerberus's breath.
