---
number: 79
title: piney-effect: the arrival, hits and numbers, the five spell systems, Data Drain, portals and particles, checked against the game
date: 2026-09-24
area: render, battle, test
files: crates/piney-effect, tools/test_effect_rs.py, tools/test_effect_draw_rs.py, tools/test_effect_hit_rs.py, tools/test_effect_particle_rs.py, tools/test_effect_misc_rs.py, tools/test_effect_spell_rs.py, docs/engine/effects.md, docs/engine/particles.md
---

# 79. piney-effect: the arrival, hits and numbers, the five spell systems, Data Drain, portals and particles, checked against the game

Combat needs its presentation: the marks where a blow lands, the rising
damage numbers, the spells, and the portals. [[67]] and [[78]] return
these as events without drawing them. An effects agent and four helpers
have ported the field's effect tasks as a new crate, `piney-effect`.
docs/engine/effects.md and particles.md hold the detail.

## What is ported

- **The core.**
  - `ccThEffect` / `ccEffectCtrl`: 500 effects, their dispatch by id, and
    `ccEffect2`.
  - The effect files and tables, read at run time, and the EFF chunk
    (0x0e00).
  - The draw path: `ccEff::Draw` and the VU1 program `mc_DrawEff` down to
    GS primitives, with fog, sort key and near/far culling.
  - `ccClump::Draw` and `ccAnm::Draw`'s node matrices and transparency.
    Porting these fixed two drawing bugs of the effects' own: animated
    transparency per object, and ExtObj copies.
- **Kite's arrival.** `effTransfer`, its ring, and `effWarpTransfer`.
  These also serve the walking PCs, merchants and party members.
- **Hits.**
  - `ccHitMarkDisp` with its spark, ring and photons.
  - The protect effect, the attribute guard and critical.
  - The words: CRITICAL, DYING and NO DAMAGE.
  - The rising damage numbers (`ccCtrlFlyFont`) and the stacked ones
    (`ccDamUprStr`), drawn as the game's own sprite packets on the menu
    layer.
- **Spells.**
  - Fall, Tornado, Convergence, Upheaval and Summons at all four levels,
    and skills 289-294 (289 is the drill missile).
  - `ccEffectElementManager`.
  - The spells' damage calls, as events on the exact frame the game makes
    them.
- **The rest of combat.**
  - Level up.
  - The dying blow.
  - Data Drain's effects: `effDrainCtrl`, the homing orbs, and the effect
    after the drain.
  - The magic portal (`ccMagicCircle`, `ccMcPart`).
- **Particles.** All of `ccThParticle`: the generators, the particles,
  the 17 kinds of force, and `ccConditionEffect`.

## Checked

- **Against the game.** Six harnesses, all with 0 mismatches:

  | harness | what it covers |
  | --- | --- |
  | `test_effect_rs` | the arrival in town, field, moving, faded and warp; the core's draw path over every object kind (2,800 frames) |
  | `test_effect_draw_rs` | sprites (random cases), 1,130 clump draws, 630 animation draws |
  | `test_effect_particle_rs` | 14,359 frames: 1.26 million particle states, 1.02 million draws |
  | `test_effect_hit_rs` | hits (2,169 frames, with the real particle system), protect and words (6,003), numbers (5,400 frames, 356,939 sprites word for word) |
  | `test_effect_misc_rs` | level up, dying, drains, orbs, after-drain, the portal (10,280 frames), and the math functions to the bit |
  | `test_effect_spell_rs` | every attack spell, 102 ids cast three times each (37,057 frames) |

- **The disc tests.** `cargo test -p piney-effect` runs 27 tests against
  the disc.
- **Re-run in a clean worktree.** All six harnesses pass, with the
  workspace's tests, clippy, fmt and the docs check.
- **Shots.** 87 frames from `effect_shot`, of each effect over its life.

**Still unknown:**
- **Not in the runtime yet.** Nothing calls the crate. The combat agent
  wires it into the field, and effects.md gives the order: start, step,
  spells, particles, draw, numbers, portals, events.
- **Not ported:**
  - `effSkillStart` and `effSkillStartEffect`, which run at every skill's
    start;
  - the exec rings;
  - the physical skills' shock wave;
  - heals, cures, Sanity, Resurrect, opening boxes, removing traps,
    stat changes, and the resistant shield;
  - the breakable objects' fragments;
  - the in-engine streams' effects.
- **Ported twice.** The magic portal is in both piney-battle's entry.rs
  and here, both checked. The runtime keeps one.
- **Undefined behaviour taken one way.** Odd sizes or drain types start
  nothing, a few stack leftovers the spells read are taken as 0, and the
  goblin summons' read past its table answers "busy".
- **Not modelled in drawing:**
  - a sprite's fog;
  - the spell pieces' fog and shadow switches;
  - the explosions' palette swaps.
