---
number: 553
title: Bosses as defs: Kyvia made of systems, its core and gomoras lent, its disc and the fourth's places; the baselines taken anew at 9aaea16
date: 2026-11-02
area: battle
files: crates/piney-battle/examples/battle_probe/kyvia.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/class.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-battle/src/boss/kyvia/fourth.rs, crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-battle/src/boss/kyvia/magic_ball.rs, crates/piney-battle/src/boss/kyvia/tests.rs, crates/piney-battle/src/boss/kyvia/thunder.rs, crates/piney-battle/src/boss/part.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-game/src/session/tests/kyvia_fourth.rs, crates/piney-game/src/session/tests/kyvia_fx.rs, crates/piney-game/src/session/tests/kyvia_stage.rs, crates/piney-game/src/session/tests/kyvia_third.rs, crates/piney-game/src/session/tests/skeith.rs, crates/piney-game/src/session/tests/survey.rs, docs/engine/boss-kyvia-fourth.md, docs/engine/boss-kyvia-second.md, docs/engine/boss-kyvia-third.md, docs/engine/boss-kyvia.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-02T01:31:09-07:00
took: 59m
---

# 553. Bosses as defs: Kyvia made of systems, its core and gomoras lent, its disc and the fourth's places; the baselines taken anew at 9aaea16

Section 2's "Bosses as defs and systems": Kyvia's four fights
(`ccBossKyvia01`-`04`), their core (`kyviaCore`) and its five gomoras
(`kyviaGomora`) on the runner (worklog 540), after Magus (547). Before
it, the baselines were taken anew at 9aaea16 (below).

## The defs

`KYVIA` (and `KYVIA_SECOND`, `KYVIA_THIRD` from it) names four systems:

- `kyvia::SlaveCore`: the core's character (`SlaveCore`), the core lent
  out of it for the frame (`Lend`, `Return`) and drawn as a part. Its
  `Action` is the body's `Slave()`: the core's `Main`, which runs its
  gomoras' in turn.
- `kyvia::Gomoras`: the core's gomoras (its `slaveCtrl[0]`), lent with
  it and drawn after it.
- `kyvia::Disc`: the disc the body rides (`DiscPos`, `DiscLV`,
  `DiscMaxLV`, `StFlg`, the ride's count, `dummypos`). Its `Think` is
  `CheckDiscMove` (the ride's camera, the core's entry, the next stage).
- `kyvia::Kyvia`: the rest, as `ccBossKyvia01::ccBossKyvia01` starts it.
  `Think` is the after-image count and the pattern, `Action` the acts,
  `Lock` `ccBoss::Main`'s lock with the cinema off as the party is let
  go, `Pass` the effect manager's pass before the blur (the def's
  `pass`). Its draw is the body where it stands and what its generators
  follow.

```text
order:  SlaveCore.Lend, Gomoras.Lend, Live[Prologue, TickStop, Disc.Think,
        Kyvia.Think, Kyvia.Action, Move, Kyvia.Lock, ForwardAndDraw,
        SlaveCore.Action], Gomoras.Return, SlaveCore.Return
affect: BaseAffect
pass:   SlaveCore.Lend, Gomoras.Lend, Kyvia.Pass, Gomoras.Return, SlaveCore.Return
```

`KYVIA_FOURTH` adds `kyvia::fourth::Places` (where each core rises, the
walls and gates, `CoreLvCount`, the rise), whose `Think` makes the walls
on the first frame; its order has `Places.Think` before `Disc.Think`,
the body's own `Move` (`Kyvia.Move`) and the hold let go after
`Slave()`. The fourth fight's `CheckDiscMove` is the disc's `Think` with
the places.

The after-image count (`AfCou`) now runs after `CheckDiscMove` (it ran
before): `CheckDiscMove` neither reads nor writes it.

The body's code reaches its siblings through one view, `kyvia::Tree`
(the core and gomoras lent, the disc, the places), made from `Others`
for the step. `CORE` and `GOMORA` name their own system (`core::Core`,
`gomora::Gomora`, lent with `Part<T>` off their boss's list,
`part_system!`), their `Affect` their own; a gomora's draw is its body
and its aura's place. `Class::Kyvia`, `Class::KyviaCore`,
`Class::Gomora`, `KyviaData` and `BossData`'s field are gone: the
tables are views of the volume's `tables::combat` (`kyvia0N_anims`,
`kyvia_core_anims`, `kyvia_gomora_anims`, the skills, and the gomoras'
lists through `gomora::lists_of`).

The probe, the unit tests and piney-game's tests read the body, its
disc and places and the core's and gomoras' own by kind
(`Systems::get`).

## The baselines

Worklog 548 to 552 changed checked output on purpose, and 550 the data
version (35); the data was built again, the release probe built, and
the pins and golden shots taken anew at 9aaea16 unchanged:

- The pins: the old recordings replayed against 9aaea16's probe differ
  only in Kyvia's 13 of 14 calls, and there only in the after-image
  tokens (worklog 549's own after-images); every other set 0. Recorded
  again (the eleven harnesses passing against the game).
- The golden shots, a new set `s552`: the 13 tests of the old sets
  (`s537a`, `s543base`) and Kyvia's and Corbenik's not yet in them
  (`kyvia_push_and_turn_shots`, `kyvia_gomora_shape_shots`,
  `kyvia_shots`, `kyvia_attack_shots`, `kyvia_core_wave_shots`,
  `kyvia_third_shots`, `kyvia_fourth_shots`,
  `corbenik_third_under_god`). Against the old sets only the expected
  differ: `macha_shots` and `tarvos_under_god` from the drain on (548),
  `kyvia_stage_shots`' `fight-1000` (549).

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden shots, all 21 tests of `s552`: 2,255 PNGs, every one byte
  for byte.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

The main tree's generated tables (`work/data`, not tracked) were still
those before worklog 550, so `gen --check` named `world.bin` out of date
on three volumes and piney-world's `area_125_has_fifteen_floors_from_mutation_on`
failed on them; `piney-gen gen` made them again, nothing in the
repository changed.

**Still unknown:** nothing this step opened; Innis and Corbenik remain to convert, then `Class`, `each_class!` and `BossData` out.
