---
number: 540
title: Bosses as defs, step two: one runner over a def's steps, the systems as objects, Skeith made of them
date: 2026-11-01
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/systems/, crates/piney-battle/src/boss/skeith.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/class.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-game/src/session/tests/skeith.rs, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-01T22:07:03-07:00
took: 28m
---

# 540. Bosses as defs, step two: one runner over a def's steps, the systems as objects, Skeith made of them


Section 2's "Bosses as defs and systems" (`plans/rust-cleanup.md`), its
second step: the runner and the shared systems, with Skeith, the
simplest class, made of them so nothing lands unused.

## The runner

`piney_battle::boss::systems`. A def names its systems
(`BossDef::systems`, each a `Kind`) and four step lists in the game's
order: `order` (its `Main`), `affect` (its `Affect`), `pass` (the effect
manager's pass before the blur and the task) and `hand_over` (the task's
step after `Main`). A `Step` is one of `ccBoss::Main`'s blocks on the
shared `ccBoss` members (`TickEffects`, `Prologue`, `TickStop`,
`LockParty`, `Move`, `ForwardAndDraw`, `ClearHold`, `BaseAffect`),
`Live(steps)` for what `ccBoss::Main`'s early return guards (checked once,
as the game does), or `Run(kind, hook)`, a system's own step (`Think`,
`Action`, `PreDraw`, `Late`, `Affect`, `Pass`, `HandOver`). `Boss::main`,
`affect`, `effects_pass` and `hand_over` each walk one list; `Boss::show`
is the draw pass: `ccBoss::Draw`'s body (`BossBase::body_draw`), then
each system's draw into the `Output` in the def's order, then the parts'.

- A system is an object: `System` is one enum over the systems' types
  (boxed, one `match` a call, generated with the `Kind` enum by
  `systems!`), each type a `BossSystem` impl with its steps, its draw
  and its hand-over. `Systems::of(kinds)` builds them as their class's
  constructor starts them (each type's `Default`).
- Siblings: a system steps with `&mut BossBase`, `&mut Others` (the list
  without itself) and the `Cx`; a boss's module reaches a sibling by its
  kind through a private accessor (`skeith::wave`). Tests and probes read
  a system by kind (`Systems::get::<T>()`); the field never does, it
  reads the `Output` and the `Out`s.
- `ccBoss::Main`'s blocks moved to `systems::frame`, the pattern runner
  (`Patterns`, `BaseActs`, `base_exec_pattern_index`,
  `base_change_next_pattern`) to `systems::patterns`. `class.rs` keeps
  only what the classes not yet converted run: `BASE_BOSS`'s
  `Step::Class(hook)` hands their `Main`, `Affect`, pass and hand-over to
  `Class::with_rules` until each lands, and their `show` draws over the
  base's body.
- The wave (`m_anmw`, a member of each class that has one, not of
  `ccBoss`: Infection's DWARF puts it at `ccBoss01` +0x29358) is a shared
  system, `systems::wave::Wave`: its clip, drawn as its pose.

## Skeith

`SKEITH`'s systems are his wave and his own (`skeith::Skeith`: his
spell's force and ice effects, his death's effect and fades, the stage
effect, `m_patMode`, `m_actDirc`, started as `ccBoss01::ccBoss01` does).
His frame:

```text
order:  TickEffects, Live[Prologue, TickStop, Skeith.Think, Skeith.Action,
        LockParty, Move, Skeith.PreDraw, ForwardAndDraw], ClearHold, Skeith.Late
affect: Skeith.Affect
draw:   ccBoss::Draw's body, Wave (its pose), Skeith (the cross's trail)
```

`PreDraw` is `ccBoss01::PreDrawAnm`'s wave speed, `Late` `DrawCross`'s
first step and the PP count. His tables are views of `tables::combat`'s
(`skeith::words`, `anims`, alike on every volume), so `BossData` and
`Class` lose his field and variant.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots: every PNG byte for byte as at worklog 537.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release); `too_many_arguments`
  150 as before.
- `piney-gen gen --check`, clippy with all features, fmt, `cairns check`,
  `tools/docs.py check`.

**Still unknown:** nothing this step opened; the item goes on with each boss's own systems (Fidchell next), then `Class`, `each_class!`, `BossData` and the Innis slave's `base_main` out.
