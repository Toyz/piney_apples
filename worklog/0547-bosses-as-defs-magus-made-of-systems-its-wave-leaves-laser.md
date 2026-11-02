---
number: 547
title: Bosses as defs: Magus made of systems, its wave, leaves, laser, needles and its own; the body read through the field
date: 2026-11-02
area: battle
files: crates/piney-battle/src/boss/magus.rs, crates/piney-battle/src/boss/magus/leaf.rs, crates/piney-battle/src/boss/magus/tests.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-battle/src/boss/systems/wave.rs, crates/piney-battle/src/boss/skeith.rs, crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss/part.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/magus.rs, crates/piney-game/src/session/tests/, docs/engine/boss-magus.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-02T00:37:04-07:00
took: 26m
---

# 547. Bosses as defs: Magus made of systems, its wave, leaves, laser, needles and its own; the body read through the field


Section 2's "Bosses as defs and systems": Mutation's `ccBoss03` and its
twelve `ccBoss03Leaf`s on the runner (worklog 540), after Gorre (546).

## The def

`MAGUS`'s systems are its wave, its leaves, its laser, its needles' pass
and its own:

- `magus::Leaves`: the leaves' characters (`slaveCtrl[0]`), each leaf a
  boss of its own lent for the body's frame (`Lend`, `Return`; one
  missing is skipped, as before), and where each hangs on the body
  (`Early`: `OBJ_ex31leafNN` through `Cx::field`, which a leaf's `Init`
  reads). `Action` is `Slave()`, `Late` lets the leaves' holds go. It
  draws the leaves as parts and hides a dropped leaf's objects and light
  outside the Epitaph and the grow.
- `magus::Laser`: the twelve shocks and `m_shockIdx`, their objects'
  places and where each beam meets the ground (`Early`: read off the
  laser clip one step on, as before), the beams drawn in `PreDraw` (the
  laser's third step of `ccBoss03::PreDrawAnm`).
- `magus::Needles`: the effect manager's pass, whose only own lives are
  the needles' (`ccBossEffNeedle`).
- `magus::Magus`: the rest, as `ccBoss03::ccBoss03` starts it. `Early`
  counts the attack's wait, `Move` is its own (`ccBoss03::Move`, the push
  count), `PreDraw` the clip forward (not in the laser's second step),
  `Late` `CalcCamera` and the PP count; its draw hides its images going
  to the centre.

```text
order:  Leaves.Early, Laser.Early, Leaves.Lend, Needles.Pass, Magus.Early,
        Live[Prologue, TickStop, Magus.Think, LockParty, Magus.Move,
        Magus.PreDraw, Laser.PreDraw, Draw, Leaves.Action, ClearHold],
        Magus.Late, ClearHold, Leaves.Late, Leaves.Return
affect: Magus.Affect
```

The PP count runs before the holds are let go now (it was after): it
reads and writes only the boss's gauge count, which no hold touches.
`LEAF` is its own system (`magus::leaf::Leaf`, its `Affect` and its draw:
its marker's place and its burst), lent with `Part<T>`.

## The runner

- `Hook::Early` (what a `Main` does before `ccBoss::Main`'s) and
  `Hook::Move` (a class's own `Move`) are new.
- `Others` holds the siblings as a list: `Others::take::<T>()` keeps one
  for the whole step, so a step holds several (Magus's `Think` its
  leaves, laser and wave); Gorre's two-at-once accessor went for it.
- A system's draw sees the boss's other systems (`all: &Systems`):
  `Leaves` reads Magus's `m_bEpitaph` through it.

## The field, not members

The probe and the tests no longer write the body's places into Magus's
members: the probe's field (`Body`) answers `node` with the case's leaf
places, `clip_entries` with its shock places, and `line` shock by shock
with the frame's hits; the unit tests' field (`Hangs`) hangs the leaves
in a row. `MagusData`, `Class::Magus`, `Class::MagusLeaf` and
`BossData`'s field are gone; the tables are views of the volume's
`tables::combat` (`magus_*`).

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots as at worklog 537, and `magus_shots` (126) and
  `magus_flash_shots` (77) as at worklog 546: every PNG byte for byte.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; Kyvia, Innis and Corbenik remain to convert, then `Class`, `each_class!` and `BossData` out.
