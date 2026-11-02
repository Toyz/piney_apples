---
number: 554
title: Bosses as defs: Innis made of systems, its wave, its missiles' pass, its images and its own; the images' Main written out
date: 2026-11-02
area: battle
files: crates/piney-battle/examples/battle_probe/innis.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/class.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/innis.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-game/src/session/tests/boss_fx.rs, crates/piney-game/src/session/tests/skeith.rs, docs/engine/boss-innis.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-02T02:31:03-07:00
took: 29m
---

# 554. Bosses as defs: Innis made of systems, its wave, its missiles' pass, its images and its own; the images' Main written out

Section 2's "Bosses as defs and systems": Mutation's `ccBoss02` and its
three `ccBoss02Slave`s on the runner (worklog 540), after Kyvia (553).

## The def

`INNIS` names four systems:

- `systems::wave::Wave`: its `anmw` (`ANM_xx11wave`), the shared wave,
  set and run by the spin's fall (`AllAttack`) and drawn where posed.
- `innis::Missiles`: the effect manager's pass before the task, whose
  lives of their own are its missiles' (their `Draw` calls its
  `MagicDamage`).
- `innis::Images`: the three images (`slaveCtrl[0]`). Its `Action` is
  `Slave()`: each image's `ccBoss::Main` with its own `Think`, written
  out from `ccBoss::Main`'s blocks (`Slave::main`); the `base_main`
  closure the image ran through is gone from Innis. Its draw is the
  images as figures and their mirrors' shards.
- `innis::Innis`: the rest, as `ccBoss02::ccBoss02` starts it. `Think`,
  `Action` (the wave and the images reached through `Others`), its own
  `Move` (held at `Hipos`), `Lock` (`ccBoss::Main`'s lock with the menu's
  forbid as act 5 wrote it that frame), `PreDraw` (the clip forward and
  both transparencies its alpha), `Affect`; its draw is the body.

```text
order:  Missiles.Pass, Live[Prologue, TickStop, Innis.Think, Innis.Action,
        Innis.Move, Innis.Lock, Innis.PreDraw, Draw, Images.Action, ClearHold]
affect: Innis.Affect
```

`Class::Innis`, `InnisData` and `BossData`'s field are gone: the tables
are views of the volume's `tables::combat` (`innis_pattern`,
`innis_epitaph`, `innis_anims`, the skills, `innis_monster_anims`,
`innis_ring_models`). The probe and the tests read Innis, its images and
its wave by kind.

## The magic square

The plan named a magic-square system. `ccBoss02::MagicSquare` is one of
Innis's acts, run from `SwitchActionPattern` (word 19) like the others,
and all it keeps is what every act keeps: the step count (`Cou`), the
`EnyFlg`/`EnyInt`/`EnyFloat` words the missiles' `MagicDamage` and the
other patterns read too, and the acts' camera (`ZoomVec`, `CamMat`,
`CamPos`, `CamView`, which `SkillAttack` and the images' act use; only
`CamDist` is its alone). The square itself is a `ccBossEffBrightMagicSquare`
on the effect manager with no rules of its own. As a system it would
hold one float and reach into Innis for the rest, so it stays one of
Innis's acts.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden shots, all 21 tests of `s552` (worklog 553): 2,255 PNGs,
  every one byte for byte.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; Corbenik remains to convert, then `Class`, `each_class!` and `BossData` out.
