---
number: 119
title: "The runners' dust: ccEffPawSmoke at Kite's and the party's feet"
date: 2026-09-25
area: battle, render, test
files: crates/piney-effect/src/dust.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/examples/particle_probe.rs, crates/piney-game/src/fx.rs, tools/test_effect_particle_rs.py, docs/engine/effects.md, docs/engine/particles.md
resolves: 118
---

# 119. The runners' dust: ccEffPawSmoke at Kite's and the party's feet

Kite's and the members' rules already raise `Out::PawSmoke { speed }` as
they run ([[118]] listed it among `effSmoke`'s relatives still missing).
It is now drawn.

## ccEffPawSmoke (main 0x001ce640)

- **Which foot.** The character's clump's `OBJ_t0 l foot` and `OBJ_t0 r
  foot` nodes are found (`GetObjAdrsF`) and their LW matrices made. The
  puff rises at the lower of the two.
- **The ground.** Under that foot, `ccLandHitCheck` then
  `checkHitResultAttlibute`:
  - water and grass (0xb0c000, 0xc0d000, 0x60b0d0, 0x70c0e0, 0x8080f0)
    raise nothing;
  - the grey and dark grounds give texture 4;
  - the rest give 133.
- **The puff.** One particle on effect.cpp's second static generator,
  `ccpgPawSmoke`:
  - thrown back along the heading at 0.1 x 0.0375 x speed x (10 - rand()
    % 5);
  - it lives 10 less up to 5 frames;
  - size 1.5, fades 1024 in and 64 out, a random turn.

The port's `eff_paw_smoke` takes the two feet's places. fx.rs poses them
from the runner's actor (`Body::worlds` of its play under its root) and
asks the ground through a new `Host::ground_attribute`, which the battle
host answers from `Hits::land` and `Hits::attribute`.

## Checked

- **`test_effect_particle_rs.py`'s `test_paw_smoke`.** It runs the game's
  `ccEffPawSmoke` in eemu on a character whose feet nodes are stubbed:
  - `GetObjAdrsF` answers two unparented nodes at the chosen places, so
    their LW matrices are copied from the local ones;
  - `ccLandHitCheck` is answered, and `checkHitResultAttlibute` gives the
    chosen ground.

  Eight runs of 20-60 steps, a step a frame, with either foot lower,
  random speeds and every kind of ground. 635 frames of the whole particle
  state match, including when no puff is made.
- **`story_4_shots`.** A run shot shows a grey puff at Kite's heel as he
  runs down the dungeon's corridor. Field 14's grass raises none, as in
  the game.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **The feet's pose.** The port poses them as the actor stands when the
  effects start. The game makes the LW matrices inside the runner's own
  frame, from the pose its last draw left. Whether the two ever differ by
  a frame was not measured.
- **`effSmoke`'s other callers** are still not ported: the Grunties,
  `WORLD::DrawSteam` and `DrawEffect`, `ccGimSymbol`,
  `ccEnemyBrPart::explode`, `ccDog` and the bosses.
