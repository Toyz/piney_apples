---
number: 308
title: Outbreak's sqrt.s in the party's distances: Kite's harness at 0
date: 2026-09-30
area: battle, volumes
files: crates/piney-battle/src/geom.rs, crates/piney-battle/src/party_ai.rs, crates/piney-battle/src/flow.rs, crates/piney-battle/src/kite.rs, crates/piney-battle/src/follow.rs, crates/piney-battle/src/ai_move.rs, crates/piney-battle/src/fellow.rs, crates/piney-battle/src/damage.rs, crates/piney-battle/src/skill.rs, crates/piney-battle/src/drain.rs, crates/piney-battle/src/navi.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/ride.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/examples/battle_probe, crates/piney-battle/examples/ride_probe.rs, docs/engine/battle.md
---

# 308. Outbreak's sqrt.s in the party's distances: Kite's harness at 0

Worklog 307 left `test_battle_kite_rs.py` on Outbreak at 10 of 200
mismatches in `main_run` and 31 in `main_run_ai`. Infection and Mutation
were at 0 on all ten checks, and still are.

## One cause

A deep diff of every mismatching case (scratch script, first differing
leaf) put all 41 on one field: word 34 of Kite's AI record, `distTg`, the
port above the game by one unit in the last place (up to 32 where the
widths come off a larger distance). The writer is
`ccAI::DistanceToTarget`: INF gcmn 0x00582f50 calls `sqrtf` at 0x00582ff0;
OUT 0x005a5eb0 has `sqrt.s` inline at 0x005a5f54. Worklog 305 found the same
in `ccGetDist` (OUT main 0x001e6e70).

It is the whole executable, not one function. A scan of every gcmn function
for `jal sqrtf` and for `sqrt.s`: Infection has 70 functions calling
`sqrtf` (112 calls), Mutation 71 (115). Outbreak and Quarantine have 85
functions each with `sqrt.s` and no `sqrtf` symbol at all; on OUT the only
callers of `__ieee754_sqrtf` are libm's `acosf` and `asinf`.

## The fix

`geom::sqrt_on(volume, v)` is the one place: newlib `sqrtf` on INF and MUT,
the FPU's truncating `sqrt` on OUT and QUA. `geom::plane_dist` and
`geom::length_on` take the volume and go through it. Every `sqrtf` of
piney-battle now does: `distance_to_target` (both `DistanceToTarget`s),
`SetTargetDist`, `CheckNote`'s reach, `FollowPlayer`, `FollowTarget`,
`FollowTargetDirc`, `FollowTargetTown`, `MoveP2P`, `ccSpcChar::HitCheck`,
the target searches, the area rules (`ground_distance`, `in_area`,
`ccSkillHold`), Data Drain's reach, the landmark searches,
`ccEntryObj::routine`'s `plDist` and the Grunty's `ControlMove`
(`RideTables` keeps the volume). `get_dist_on` and the bosses' `sqrt_of`
use it too. INF and MUT compute exactly what they did.

## Checks

- `test_battle_kite_rs.py`, 200 cases of all ten checks: 0 on INF, MUT and
  OUT (OUT was 10 and 31).
- Other harnesses on OUT, 200 cases, before and after: party AI `UseSkill`
  15 to 0; fellow `FollowTarget` 1, `FollowTargetDirc` 6, `Main` 74 and the
  six `Run*` 200 each, all to 0; navi `ActInTownRun` 88, `ActInTownFull`
  81, `BrainsDungeon` 85, all to 0. Unchanged: flow `skill_main` 4 and
  `area_item` 35, drain `drain` 176 and `side_effect` 1. Enemy AI 0 and
  enemy motion `main` 2 of 300, as in worklog 305.
- `test_ride_rs.py` does not start on OUT (`AwakeDistantLight__9WORLD_MANFv`
  is not carried).
- Suites: piney-battle 117 + 5, piney-world 88 + 3 + 12, piney-game 171.
  Clippy and fmt clean. `outbreak_whole_story` ends at 408,000 and
  `mutation_whole_story` at 506,100, both as before.

**Still unknown:** what the leftover OUT mismatches in `test_battle_flow_rs.py`
(`area_item` returns, `skill_main` affects) and `test_battle_drain_rs.py`
(the erosion, 176 of 200) are; they predate this and are not `sqrt.s`.
Whether the Grunty's stick matches on OUT (its harness does not run there).
The other crates (piney-world's field objects, piney-effect, piney-audio)
still call newlib `sqrtf` on every volume.
