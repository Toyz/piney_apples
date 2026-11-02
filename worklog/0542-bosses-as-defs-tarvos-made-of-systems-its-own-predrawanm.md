---
number: 542
title: Bosses as defs: Tarvos made of systems, its own PreDrawAnm before the body's Draw
date: 2026-11-01
area: battle
files: crates/piney-battle/src/boss/tarvos.rs, crates/piney-battle/src/boss/tarvos/tests.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/tarvos.rs, crates/piney-game/src/session/tests/, docs/engine/boss-tarvos.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-01T22:57:52-07:00
took: 24m
---

# 542. Bosses as defs: Tarvos made of systems, its own PreDrawAnm before the body's Draw


Section 2's "Bosses as defs and systems": Quarantine's `ccBoss07` on the
runner (worklog 540), after Fidchell (541).

## The def

`TARVOS`'s systems are its wave (`systems::wave::Wave`, its `anmw`) and
its own (`tarvos::Tarvos`: its aura, spike, phantom, death and start
effects, the skill, the stage effect, `m_patternMode`, `m_defMode`, the
transfer's `m_blendRate`), started as `ccBoss07::ccBoss07` does (no
stage effect). Its frame, `ccThBossEffect`'s pass then `ccBoss07::Main`
(QUA gcmn 0x003a91e0):

```text
order:  Tarvos.Pass, Live[Prologue, TickStop, Tarvos.Think, LockParty, Move,
        Tarvos.PreDraw, Draw, ClearHold], ClearHold, Tarvos.Late
affect: Tarvos.Affect
draw:   ccBoss::Draw's body, Wave (its pose)
```

- Its `PreDrawAnm` (0x003a96b0) takes the place of `ccBoss`'s: the body
  not posed while the transfer's stream plays (proccess 2), the
  transfer's fog asked. A new base step, `Step::Draw`, is `ccBoss::Draw`
  alone while `drawSW` holds; `ForwardAndDraw` stays `ccBoss`'s own pair.
- The hold is let go twice, as the game does (in `ccBoss::Main` and
  again in `ccBoss07::Main`); `Late` is the PP count while the party is
  held.
- The wave act and the Epitaph's reach the wave through `tarvos::wave`.
- `TarvosData` is gone: the tables are views of the volume's
  `tables::combat` (`tarvos_*`); its `rand_skills` (`@1178`, pattern
  10's draw, which the port drops) went unread and are not kept.
  `Class::Tarvos` and `BossData`'s field went with it.
- piney-game's arena tests name their boss by a `fn(&Boss) -> bool`
  (Tarvos's by its def's class, `CcBoss07`), not by a `Class` variant;
  not by the def's address: `BossDef`'s `==` compares addresses, and a
  `const` def referred to from two crates need not share one.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots (`tarvos_under_god` among them): every PNG byte
  for byte as at worklog 537.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150 as before), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; Macha, Gorre, Magus, Kyvia, Innis and Corbenik remain to convert, then `Class`, `each_class!` and `BossData` out.
