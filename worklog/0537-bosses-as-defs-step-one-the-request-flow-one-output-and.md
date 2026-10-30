---
number: 537
title: Bosses as defs, step one: the request flow; one Output and generic Outs the field and the effects read the same for every boss, inputs through Cx::field, Corbenik's form change a hand-over
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, crates/piney-battle/examples/battle_probe/, crates/piney-world/src/combat/, crates/piney-world/src/field/world.rs, crates/piney-effect/src/, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T16:57:14-06:00
took: 41m
---

# 537. Bosses as defs, step one: the request flow; one Output and generic Outs the field and the effects read the same for every boss, inputs through Cx::field, Corbenik's form change a hand-over

Section 2's "Bosses as defs and systems" (`plans/rust-cleanup.md`), its
first step: the request flow. The world and the game read every boss the
same way now, through its def and what it asks; no boss is named in
piney-world's combat, nor in piney-game's fx and area.

## The def

`piney_battle::boss::defs`: `BossDef` with `BASE_BOSS` (and `BASE_PART`,
the same for now) and struct update, one def a class: the eleven codes'
(`BOSSES`, `bossFunc`), Corbenik's later forms (`CORBENIK_SECOND`,
`CORBENIK_THIRD`) and the parts (`LEAF`, `CORE`, `GOMORA`, `BROTHER`,
`SEED`, `ROOT`, `EYE`, `KILLER_EYE`), each with its `BossGameClass`. The
world's `BOSSES` table, `SKEITH_DEF`, the code constants and its
`BodyPick`, `PartBody`, `CamDef` and `Spawn` moved in. `Boss` carries its
def (`Boss::of`), and the camera is the def's: `CamDef::Transfer` and
what the constructor asked of it. Corbenik's third form's camera (the
pitch `atan2f(500, 1000)`, the range 1000, the lift `1500 tanf(pitch)`,
`SetRotXLimit(1.0)`) is asked by its constructor (`Out::CamLift` new),
not built by the world.

## What the boss asks: `Out` and `Output`

- `Shape` is gone. `Boss::show` writes an `Output` after the frame for the
  boss and each part: the body's draw (`BodyDraw`: place, turn, whether,
  a `PreDrawAnm` matrix, the clip drawn again), the parts, the images it
  keeps (`Figure`), the shard clumps, the objects its draw hides and
  whether its after-images do, the wave's pose, the sword trail (a row at
  two named dummies under a named node), the places its generators follow
  (`Follow`, moved out of Kyvia, with Magus's markers) and the bursts.
- The per-boss `Out` variants are gone: `Magus`, `Fidchell`, `Macha`,
  `Tarvos`, `Corbenik`, `Particles`, `KyviaParticles`, `KyviaGen` and
  `DrawAnm`. What the field drew of them is generic: `Out::Picture` (an
  object or a clip of the file or xeffect at a place, a matrix or a node
  of the body as last drawn, at the body's `setTransparency` unless its
  own; Corbenik's barrier, images, elements, root and lasers, built in
  `corbenik/pictures.rs`, Kyvia's arm, Macha's ball), `Count` (over a
  character or at a place), `AnmText` (Fidchell's prediction), `Voice`,
  `VoiceStop`, `Light` (an `OmniLight`), `Fog`, `StageClump`,
  `AfterImagePart` (a leaf's blink, its params `leaf::BLINK`),
  `PartRemoved`, `Remade`. What the effects make beside the ones a boss
  keeps is `Out::Fx(FxCall)`, one arm in piney-game's fx and one
  `Effects::boss_call` in piney-effect, which hands each to its boss's
  module. Magus's shocks and bursts carry their places and transparency.
- `piney_effect::boss::Source` is `fx`, `follow`, `explodes`; the boss
  effect files a volume needs are `piney_effect::boss::files`.

## What the boss reads: `Cx::field`

`PartHits` is `boss::Field`, with what a frame reads of the field: a node
of a body as last drawn, a clip's entries posed on it, a character's
heading. Magus reads its leaves' places and, in the laser, its shocks
(`read_body`, the start of its `main`); Macha its eyes. The world wrote
them into the class before; the probes still set them (the field they
pass answers nothing), which the systems step moves to the system list.

## Corbenik's form change

`combat/corbenik.rs` is gone. `ccThBoss08`'s step past the first form's
exit is `Boss::hand_over` (the class's `next`), after `Main` in the
field's frame and not in a probe's (the first form's harness runs the
form past its exit): the menu cleared, the flash, the seeds off the lists
and asked off the field, the second form made with the manager's effects
at Kite's place and heading (`Field::dirc`). `Out::Remade` has the field
make the hit, parts, after-images and camera anew from the new def.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots (`corbenik_under_god`, `kyvia_stage_shots`,
  `skeith_shots`, `tarvos_under_god`): every PNG byte for byte the same.
- piney-battle's, piney-world's, piney-effect's, piney-data's,
  piney-event's tests, piney-game's full suite, release.
- `piney-gen gen --check`, clippy with all features, fmt, `cairns check`,
  `tools/docs.py check`; `too_many_arguments`: 151 before, 150 after (the world's Corbenik form change went).

**Still unknown:** none this step opened; the boss item's next steps are
the runner and the shared systems, then each boss's own systems.
