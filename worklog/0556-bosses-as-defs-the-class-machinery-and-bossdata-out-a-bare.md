---
number: 556
title: Bosses as defs: the class machinery and BossData out; a bare ccBoss its own def's steps, Patterns::words by the volume, actAnmTbl a static slice
date: 2026-11-02
area: battle
files: crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-battle/examples/battle_probe/corbenik.rs, crates/piney-battle/examples/battle_probe/corbenik_b.rs, crates/piney-battle/examples/battle_probe/corbenik_c.rs, crates/piney-battle/examples/battle_probe/fidchell.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-battle/examples/battle_probe/innis.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, crates/piney-battle/examples/battle_probe/macha.rs, crates/piney-battle/examples/battle_probe/magus.rs, crates/piney-battle/examples/battle_probe/tarvos.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/class.rs, crates/piney-battle/src/boss/corbenik.rs, crates/piney-battle/src/boss/corbenik/eye.rs, crates/piney-battle/src/boss/corbenik/first.rs, crates/piney-battle/src/boss/corbenik/second.rs, crates/piney-battle/src/boss/corbenik/seed.rs, crates/piney-battle/src/boss/corbenik/tests.rs, crates/piney-battle/src/boss/corbenik/third.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/fidchell.rs, crates/piney-battle/src/boss/fidchell/tests.rs, crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss/gorre/message.rs, crates/piney-battle/src/boss/gorre/tests.rs, crates/piney-battle/src/boss/innis.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-battle/src/boss/kyvia/fourth.rs, crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-battle/src/boss/kyvia/tests.rs, crates/piney-battle/src/boss/macha.rs, crates/piney-battle/src/boss/macha/tests.rs, crates/piney-battle/src/boss/magus.rs, crates/piney-battle/src/boss/magus/leaf.rs, crates/piney-battle/src/boss/magus/tests.rs, crates/piney-battle/src/boss/part.rs, crates/piney-battle/src/boss/skeith.rs, crates/piney-battle/src/boss/systems/frame.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-battle/src/boss/systems/patterns.rs, crates/piney-battle/src/boss/tarvos.rs, crates/piney-battle/src/boss/tarvos/tests.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/mod.rs, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-02T03:32:28-07:00
took: 28m
---

# 556. Bosses as defs: the class machinery and BossData out; a bare ccBoss its own def's steps, Patterns::words by the volume, actAnmTbl a static slice

Section 2's "Bosses as defs and systems", its last step: with Corbenik
(555) every boss class is its def and its systems, so what dispatched to
the classes not yet converted goes.

## Gone

- `Class` (the enum a boss carried beside its systems) and `Boss.class`;
  `boss/class.rs` with `BossClass` (the class's `Main`, `Affect`, pass,
  next and show), `Bare` and `each_class!`. `Boss::build(def, base)` is
  the one way to make a boss.
- `Step::Class` and `Hook::Main` from the runner, and the class's show in
  `Boss::show` (each system's draw over `ccBoss::Draw`'s body is all of
  it now).
- `BossBase::base_main`, the closure-driven `ccBoss::Main` the classes and
  the bare base ran through: its last users, Innis's images (554) and
  Corbenik's roots (555), write the blocks out.
- `PartClass` and `part_system!`: a part's own is any system kind,
  `Part<T: SystemKind>` lends it off its boss's list and puts it back at
  its place in the def's.
- `BossData`, `Cx::data`, `BossEnv::data` and piney-world's
  `BattleData::bosses` (always set; the boss start and the affects
  checked it): every boss's tables are views of the volume's
  `tables::combat`.

## Changed

- `BASE_BOSS` is what a bare `ccBoss` is: its `Main` the blocks alone
  (`Live[Prologue, TickStop, LockParty, Move, ForwardAndDraw]`, its
  `Think` and `Action` empty), its `Affect` `ccBoss::Affect`, no pass and
  no hand-over. `BASE_PART` has no frame of its own (its master runs it).
  The defs no longer write out the empty pass and hand-over or `ccBoss`'s
  `Affect` they get from it; Corbenik's later forms keep an empty
  hand-over over the first's.
- `Patterns::words` takes the volume (`&'static` words), and each class's
  table helper with it.
- `BossBase.anm_tbl` (`actAnmTbl`) is a static slice of the volume's
  tables, no longer a copy of owned strings.
- piney-game's tests name the boss they look for by its def (`r.look.def
  == &defs::GORRE`), not by its game class.

`rg -n 'class::<|split_mut|has\(System::|Class::' crates/piney-battle
crates/piney-world/src/combat crates/piney-game/src` now finds, of the
bosses, only the defs' own `class: BossGameClass::...` rows (the game
class a def is); the rest are the towns' `town.class::<T>()` (the town
item next), the sky maps' and the gimmicks' own `Class` enums.

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

**Still unknown:** nothing this step opened.
