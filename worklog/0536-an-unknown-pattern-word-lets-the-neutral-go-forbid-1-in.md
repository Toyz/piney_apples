---
number: 536
title: An unknown pattern word lets the neutral go (forbid 1) in every class: Fidchell, Macha and Tarvos fall back on ccBoss::ExecPatternIndex, BaseActs.other_forbid gone
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss/class.rs, crates/piney-battle/src/boss/fidchell.rs, crates/piney-battle/src/boss/macha.rs, crates/piney-battle/src/boss/tarvos.rs, crates/piney-battle/src/boss/, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T16:27:25-06:00
took: 20m
---

# 536. An unknown pattern word lets the neutral go (forbid 1) in every class: Fidchell, Macha and Tarvos fall back on ccBoss::ExecPatternIndex, BaseActs.other_forbid gone

Section 2's boss item asked for Fidchell's `BaseActs.other_forbid` (3)
to be checked against Outbreak's `ccBoss::ExecPatternIndex`. The game's
code settles it, for Macha and Tarvos too: a pattern word no class knows
lets the neutral go (forbid 1) in every volume, so `other_forbid` was not
a difference between the classes at all, and is gone.

## The game's code

- **Fidchell** (`ccBoss04::ExecPatternIndex`, OUT gcmn 0x004a40b0): its
  own words (1, 9-11, 14-18) in its switch, every other word to
  `jal ExecPatternIndex__6ccBossFPii` (0x004a4664) with the index it
  started at.
- **Macha and Tarvos** (QUA `ccBoss06` 0x003a3bb0, `ccBoss07`
  0x003a8700) the same: `jal ccBoss::ExecPatternIndex` at 0x003a4228 and
  0x003a8db0.
- **`ccBoss::ExecPatternIndex`**: words 0-12 through a jump table (OUT
  `@1577`, 0x006ded00), where 0, 1 and 10 join the default; past 12 the
  default (`sltiu $at, $s3, 13`). The default calls `CheckEpitaph`
  (vtable +0x48), then `ChangeAction` (vtable +0x28) with `(0, 1, 4)` or,
  in the Epitaph, `(12, 3, 4)`: forbid 1 at OUT 0x0046e384, QUA
  0x00360c94 and Infection's 0x0045d10c alike.

The port's three copies held the neutral (3), the one way they differed
from the other classes' (worklog 524 found the difference by diffing the
ports, not the game).

## The change

- `BaseActs` keeps the two w lanes; `base_exec_pattern_index`'s default
  arm is the neutral with forbid 1 for every class.
- The three `Patterns` impls' docs name the fallback's address and the
  default's.
- docs/engine/boss.md says so.

## Behaviour

Only a word neither the class nor `ccBoss` knows reaches the change. The
three classes' tables, read from the discs and walked by their operand
counts (OUT gcmn `boss04NormalActTbl` 0x00617c30, `SuperActTbl`
0x00617ce0, `EpitaphActTbl` 0x00617da0; QUA `boss06NormalActTbl`
0x0050f650, `ExpertActTbl` 0x0050f6a0, its Epitaph's, `boss07*ActTbl`
0x0050f800, 0x0050f880, 0x0050f910), hold none: Fidchell's words are 1,
2, 3, 4, 6, 10, 11, 14-18; Macha's 1-4, 10, 11, 14, 15, 17, 18; Tarvos's
1-4, 10, 11, 14, 15, 17, 19-21. So no fight reaches it today; the port
now does what the code does if a word ever comes. The 290 pinned probe
answers are byte for byte the same, and every harness agrees with the
game.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots (`corbenik_under_god`, `kyvia_stage_shots`,
  `skeith_shots`, `tarvos_under_god`): every PNG byte for byte the same.
- piney-battle's, piney-world's, piney-effect's and piney-event's tests,
  piney-game's full suite, release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** nothing
