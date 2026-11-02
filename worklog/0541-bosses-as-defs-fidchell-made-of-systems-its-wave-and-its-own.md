---
number: 541
title: Bosses as defs: Fidchell made of systems, its wave and its own
date: 2026-11-01
area: battle
files: crates/piney-battle/src/boss/fidchell.rs, crates/piney-battle/src/boss/fidchell/tests.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/systems/, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/fidchell.rs, crates/piney-game/src/session/tests/fidchell.rs, docs/engine/boss-fidchell.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-01T22:35:33-07:00
took: 22m
---

# 541. Bosses as defs: Fidchell made of systems, its wave and its own


Section 2's "Bosses as defs and systems", the first boss after Skeith on
the runner (worklog 540): Outbreak's `ccBoss04`.

## The def

`FIDCHELL`'s systems are its wave (`systems::wave::Wave`, its `anmw`,
`xeffect`'s `ANM_xx11wave`) and its own (`fidchell::Fidchell`: the
prediction's text, place and spell, the spells' effects and camera,
`m_patMode`, camera 3 as it set it), started as `ccBoss04::ccBoss04`
does (no stage effect, the prediction's camera at the origin, opaque).
Its frame, `ccThBossEffect`'s pass then Outbreak's `ccBoss::Main` (OUT
gcmn 0x0046d490) around its `Think`, its `Action` and `Slave` the base's:

```text
order:  Fidchell.Pass, Live[Prologue, TickStop, Fidchell.Think, LockParty,
        Move, ForwardAndDraw, ClearHold]
affect: Fidchell.Affect
draw:   ccBoss::Draw's body, Wave (its pose)
```

Its acts reach the wave through `fidchell::wave` (the wave act and the
drained one). `FidchellData` is gone: its tables are views of the
volume's `tables::combat` (`fidchell_*`), a word past a table's end 0 as
before; `Class::Fidchell` and `BossData`'s field went with it.

## The pattern runner reads through the Cx

`Patterns::words` takes the `Cx` (the table comes back with the `Cx`'s
lifetime, so the pattern code still holds it while it calls back into
the class): a class whose tables are the volume's (Fidchell's are
Outbreak's alone) needs the volume, which `Cx::volume` gives from the
`AffectCtx`. The classes not yet systems read `cx.data` as before.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots: every PNG byte for byte as at worklog 537.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150 as before), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; Tarvos, Macha, Gorre, Magus, Kyvia, Innis and Corbenik remain to convert, then `Class`, `each_class!` and `BossData` out.
