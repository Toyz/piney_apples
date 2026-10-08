---
number: 401
title: The Flag Race test rides as a player would: the test-only ride_put is gone, and Kite walks to the breeder and rides round Dun Loireag's walls to each flag by a planned way
date: 2026-10-08
area: test, world
files: crates/piney-world/src/town_ride.rs, crates/piney-game/src/session/tests/flag_race.rs, crates/piney-game/src/session/tests/town_walk.rs, docs/engine/flag-race.md
---

# 401. The Flag Race test rides as a player would: the test-only ride_put is gone, and Kite walks to the breeder and rides round Dun Loireag's walls to each flag by a planned way

[[395]]'s `mutations_flag_race_won` steered the Grunty straight at each
flag. After 150 frames stuck on a wall it set the ride down at the flag
with `World::ride_put`, a hook made for the test. The speaking helper set
Kite down before the breeder with `pc_put`. The maintainer wants no
position or stat cheats in the tests (as with the golden goblin's HP aid,
[[373]]).

## What changed

- **`ride_put` is gone** from piney-world (`town_ride.rs`).
- **The ride's driver** (`ride_to_flags`) takes the nearest flag still out
  and rides there by the way `town_walk::TownWalker` plans:
  - survey.rs's planner over the town's walls, 100-unit cells;
  - 85 clear either side, the town body's 0.7 of the Grunty's 120 (MUT
    gcmn 0x0052e3a0);
  - planned again each second, the stick toward the next turn.

  A rider does not plan round the walking PCs. Those still walking during
  the race moved through the lane north of the ranch and closed it to the
  planner. A place he has stopped short of is gone round for 600 frames.
- **`speak_to_breeder`** walks Kite round the walls to within 300 of the
  breeder and speaks when the breeder is the target. It no longer sets
  Kite down.

## Is the ride stuck where the game's is not?

No. Riding the planned way, neither the ride nor Kite on foot ever
stopped short of a turn (checked with a print on the walker's stop).
- The run: flag 1 east of the ranch at frame 300, flag 0 by the gate,
  then flag 2 in the west. All three were taken in 910 frames (0:30:33),
  every 20-unit kerb on the way climbed.
- Steered straight at the flags, the ride drove into a wall north-east of
  the ranch, at (1377, -5906), and stayed. That is a straight line into a
  wall, not the port.
- The ride's walls and body are the town's `Hits` with
  `collision_detection`, the code tools/test_world_rs.py checks for Kite.

## The ranks

At 910 frames the ride is slower than Dun Loireag's racers (Balmung 600,
Gardenia 626, Cima 646).
- `mutations_flag_race_won` keeps its save's ranks at the player's own
  earlier 5:00s, so the race is won. Its comment now says so.
- `mutations_flag_race_against_the_towns_racers` (new) races from a new
  save. It checks the rank the game's ranking gives (main 0x0017a860):
  here 0, "Out of Rank", with nothing written to the save.
- `flag_race_shots` now shoots the result's clip, whatever it is, and
  then Rankings from the breeder's list after the race.

## Checked

- piney-game's `flag_race` tests pass: runs and quits, won, against the
  town's racers, the Rankings and the breeder's list.
- Shots in `/mnt/data/claude/scratch/i56/d/`:
  - `mut-race-riding.png`;
  - `mut-race-result.png`: "Out of Rank" at 00:31:66;
  - `mut-race-rankings.png`: Balmung, Gardenia and Cima.
- piney-game's suite (four threads), clippy on piney-world and piney-game,
  fmt, `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing.
