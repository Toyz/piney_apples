---
number: 131
title: The streams' own effects: effcStr's hit marks and transfers, checked against the game
date: 2026-09-25
area: render, test
files: crates/piney-effect/src/strfx.rs, crates/piney-effect/src/effect.rs, crates/piney-effect/src/particle.rs, crates/piney-effect/src/particle/one.rs, crates/piney-effect/src/files.rs, crates/piney-effect/src/hit.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/examples/hit_probe.rs, crates/piney-stream/src/lib.rs, crates/piney-game/src/stream.rs, tools/test_effect_str_rs.py, docs/engine/effects.md
---

# 131. The streams' own effects: effcStr's hit marks and transfers, checked against the game

The in-engine streams' effect tasks ([[125]], [[126]]) ask for hit marks
and transfers. The stream player took those requests and dropped them.
The game runs them in a second effect system of its own, and this entry
ports that system, gives it to the game's streams and checks it against
the game.

## What the game has

- **`ccThEffectStr` (priority 80).** It makes and steps a `ccEffectCtrl(1)`
  (`effcStr`): 50 slots in `effStrWork`, and `effectStrTbl`'s three
  objects of `particle` (the hit mark's sprite, the hit ring and the
  transfer ring).
- **A second `ccParticleCtrl(1)`.** It holds `particlesStr`'s 150
  particles, each run by `ccParticle::MainStr`. While
  `particleThreadStrFlag` is set, every generator started and every
  particle made goes to it.
- **`effHitMarkStr`, `effTransferStr` and `ccEffect::MainStr`.** These
  work like the field's hit mark and arrival, with their own cases. The
  draw uses no field-to-local transform and no camera test.

`docs/engine/effects.md` ("The streams' effects") has the details.

## Three things only the check found

- **Force fields.** `ccEffectCtrl(1)`'s constructor gives
  `hitPhotonDummyStrG` the photons' two force fields, which make the
  stream's photons four times the size and slow them. Its destructor takes
  them away again. The port now wires them for the stream system only.
- **The active layer.** `ccEffectCtrl::MainStr` sets `active__7ccLayer` for
  each effect that has a layer and never resets it per slot. So an effect
  with no layer draws on the last layer set in that pass, not on the
  streams' layer.
- **The slot readers.** The effect harnesses read the slots at a base
  address now (`effWork` or `effStrWork`), and their `slot` overrides take
  it as well.

## In the game

- `StreamEffects` goes to the event and Data Drain streams
  (`piney-game`'s `stream::stream_effects`), built from DATA.BIN and main
  with GCMN.PRG.
- Stream 7 shows Kite's arrival rings at Mac Anu's gate.

## Checked

- **`tools/test_effect_str_rs.py` (new).** It builds the game's two stream
  systems by their constructors, with `particleThreadStrFlag` set, and
  runs `hit_probe`'s `strreset`, `strhit` and `strtransfer` alongside.
  There are 30 random runs of eight starts (hit marks at random turns and
  layers, transfers at random heights), each run until everything ends.
  Every frame, every live slot, draw, generator, particle and `rand()`
  state matches: 5313 frames, 15093 effect draws, 234972 particle draws.
- **The field's harnesses** (`test_effect_rs`, `_particle_rs`, `_hit_rs`,
  `_misc_rs`, `_spell_rs`) still pass.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **`strEffectStopFlag`**, which holds a particle with a `strFlag` still,
  is never set by anything the port runs.
- **Stream 15's `Func_str0580` and `Func_str0581`**, with their own
  `ccEffPart0580` parts, are still not ported.
