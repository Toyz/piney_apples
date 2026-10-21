---
number: 410
title: A boss's protect shows its return: its own CalcReal named it by 'me', which the field could not place, on every boss (#69)
date: 2026-10-21
area: battle, world, test
files: crates/piney-battle/src/boss.rs, crates/piney-game/src/session/tests/boss_fx.rs
started: 2026-10-21T08:26:45-06:00
took: 28m
---

# 410. A boss's protect shows its return: its own CalcReal named it by 'me', which the field could not place, on every boss (#69)

Issue #69 (ThreePendant, build 3e87cd5, Mutation): fighting Innis, the
Protect Break came up, and once it had run out "the Protect Restore
graphic did not load". The return's picture was never made, on any boss
of any volume.

## The cause

A boss's protect returns in its own `ccChar::CalcReal(0)` (MUT gcmn
0x0056ba30), the first call of `ccBoss::Main`. When `PPcount` runs out it
calls `effProtect(this, 1, kind)` (0x005916e0; kind 2 when the row's id at
+0xc is not 0, else 1) and `SetProtect`. The port's
`chara::calc_real` raises those as `Event::Protect { on: Who::Me, .. }`
and `Event::SetProtect { on: Who::Me, .. }`. Kite's, the members' and the
enemies' frames place `Who::Me` on their own index (`map_who`,
`damage::place`), but no boss's did. The events went to the field
unplaced: `fx.rs`'s `event` turns `Who::Me` into a character only when
it knows whose frame raised them, and a rule's show carries none, so the
effect was never started.

The break is raised by `CalcBattleDamage` on `Who::Target`, which
`damage::place` names, so the break's picture always showed.

The same calls lost the boss's `DispConditionEffect` (`DispCondition(Me)`)
and the condition ticks `ConditionTimeCount` enters on the boss itself
(`EntryAffect(this, this, 9 | 3 | 10 | 4)`: HP and SP regeneration,
poison, the curse).

## The fix

`Boss::calc_real` runs `chara::calc_real` and names what it raised by the
boss's index. Every class's `Main` and every part's (Skeith's, Innis's and
its images', Magus's and its leaves', Kyvia's and its core's and gomoras',
Fidchell's, Gorre's) calls it in place of `chara::calc_real`. The ticks it
enters on itself are queued on the boss (`Boss::queued`), as Kite's and
the members' affects on it are, and run at the start of its next task:
the game runs them inside `CalcReal`, before `Think`. That frame's
difference is noted in docs/engine/boss.md ("In the port", 6).

## Tests

`session::tests::boss_fx`, on the discs:

- `innis_protect_breaks_and_returns_with_pictures` (Mutation's event 107
  in Innis's arena): Kite's blow, as `CalcBattleDamage` makes it with the
  gauge one short of full, breaks the protect twice, each left to run
  out. Before: each break's picture (effect 20, `ANM_xdhprom0`) showed 40
  frames and each return's none. After: each return's (effect 24,
  `ANM_xdhprol1`, kind 2 for Innis) shows 40 frames too.
- `skeith_protect_breaks_and_returns_with_pictures` (Infection's event
  30): the same on Skeith, 0 frames before, 40 after.
- `innis_shots` (ignored) writes the fight's frames; the return's picture
  is /mnt/data/claude/scratch/i69/after-restore-protect.png.

piney-game's suite (four threads), piney-battle's tests, clippy and fmt
pass.

The report's second break "only showed the text": the port makes the
break's picture each time (both tests break twice), at the boss's place.
In the run that broke it during Innis's spin (act 3), the boss was off
the screen and so was the picture.

**Still unknown:** nothing. The boss's own condition ticks run a frame
later than the game's (at the start of its next task); bringing them into
`CalcReal` needs each class's `Affect` to run while its `Main` holds the
class, which no boss's fight has needed yet.
