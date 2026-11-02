---
number: 555
title: Bosses as defs: Corbenik made of systems, each form its own with its parts lent, the first's hand-over its system's; seeds, roots, eyes and killer eyes each a system
date: 2026-11-02
area: battle
files: crates/piney-battle/examples/battle_probe/corbenik.rs, crates/piney-battle/examples/battle_probe/corbenik_b.rs, crates/piney-battle/examples/battle_probe/corbenik_c.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/class.rs, crates/piney-battle/src/boss/corbenik.rs, crates/piney-battle/src/boss/corbenik/eye.rs, crates/piney-battle/src/boss/corbenik/first.rs, crates/piney-battle/src/boss/corbenik/second.rs, crates/piney-battle/src/boss/corbenik/seed.rs, crates/piney-battle/src/boss/corbenik/tests.rs, crates/piney-battle/src/boss/corbenik/third.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/part.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-game/src/session/tests/corbenik.rs, crates/piney-game/src/session/tests/survey.rs, docs/engine/boss-corbenik.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-02T03:00:18-07:00
took: 32m
---

# 555. Bosses as defs: Corbenik made of systems, each form its own with its parts lent, the first's hand-over its system's; seeds, roots, eyes and killer eyes each a system

Section 2's "Bosses as defs and systems": Quarantine's Corbenik
(`ccBoss08`, `ccBoss08_b`, `ccBoss08_c`) and its parts on the runner
(worklog 540), after Innis (554). The last boss.

## The forms

Each form is its def with one system, its own:

- `CORBENIK`: `corbenik::first::First` (`ccBoss08`). `Lend` and `Return`
  its eight seeds out of their characters, `Pass` the effect manager's
  pass, `Think`, `PreDraw` (`ccBoss08::PreDrawAnm`, the clip forward),
  `Action` its `Slave` (each seed not exited, its `Main`), `Affect`. Its
  `Affect` runs with the seeds lent too (its def's `affect`). Its
  hand-over is the system's: once it exits, the second form on the same
  character (`ccThBoss08` past the first form, QUA gcmn 0x003acaf8).
- `CORBENIK_SECOND`: `second::Second`, its five roots lent; its `Think`
  the barrier message's wait, then `Think`; `PreDraw` its own (the press,
  the barrier's flash); `Action` its `Slave` (each root's `Main`, its
  quake).
- `CORBENIK_THIRD`: `third::Third`, its sixteen eyes and three killer
  eyes lent (for its `Affect` too); its own `PreDrawAnm` and its own
  `Draw` (`Hook::Draw`, new: `ccBoss::Draw` with the body's world matrix
  read from it); `Action` its `Slave` (the eyes', then the killer
  eyes').

```text
CORBENIK         order:  First.Lend, First.Pass, Live[Prologue, TickStop,
                         First.Think, LockParty, Move, First.PreDraw, Draw,
                         First.Action], First.Return, ClearHold
                 affect: First.Lend, First.Affect, First.Return
                 hand_over: First.HandOver
CORBENIK_SECOND  order:  Second.Lend, Second.Pass, Live[Prologue, TickStop,
                         Second.Think, LockParty, Move, Second.PreDraw, Draw,
                         Second.Action, ClearHold], Second.Return
CORBENIK_THIRD   order:  Third.Lend, Third.Pass, Live[Prologue, TickStop,
                         Third.Think, LockParty, Move, Third.PreDraw,
                         Third.Draw, Third.Action, ClearHold], Third.Return
                 affect: Third.Lend, Third.Affect, Third.Return
```

## The parts

`SEED`, `ROOT`, `EYE` and `KILLER_EYE` are each a def of one system, its
own (`seed::Seed`, `second::Root`, `eye::Eye`, `eye::Killer`), lent with
`Part<T>` off their boss's list; the seed's and the killer eye's
`Affect` their own, the root's and the eye's `ccBoss`'s. Their draws
(the seed at 3x 50 up and again, the root at 1.5x, the eye turned XYZ)
are their systems'. A root's `Main` is written out from `ccBoss::Main`'s
blocks around its `Think` (`root_think`): the `base_main` closure it ran
through is gone from Corbenik, as from Innis's images (554).

`Class::Corbenik`, `CorbenikB`, `CorbenikC`, `CorbenikSeed`,
`CorbenikRoot`, `CorbenikEye`, `CorbenikKiller` and `CorbenikData` are
gone: the tables are views of the volume's `tables::combat` (the
patterns by form, the killer eyes' by slot, the clips, the rings'
radii and heights read as bits). That leaves `BossData` with nothing
and the class enum with only its bare `ccBoss`; worklog 556 takes them
out. The probes and the tests read the forms and the parts by kind,
the form a fight is in by its def.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ (Corbenik's 22 among them).
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden shots, all 21 tests of `s552` (worklog 553, with
  `corbenik_under_god` and `corbenik_third_under_god`): 2,255 PNGs,
  every one byte for byte.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; the class machinery and `BossData` are left to take out (worklog 556).
