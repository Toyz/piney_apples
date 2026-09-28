---
number: 83
title: The skills' other effects: skill starts, rings, the shock wave, heals, cures, stat changes and the resistant shield
date: 2026-09-24
area: render, battle, test
files: crates/piney-effect/src/skillstart.rs, crates/piney-effect/src/shockwave.rs, crates/piney-effect/src/heal.rs, crates/piney-effect/src/ability.rs, crates/piney-effect/src/shield.rs, crates/piney-effect/src/effect.rs, crates/piney-effect/src/element.rs, crates/piney-effect/src/spell.rs, tools/test_effect_skill_rs.py, docs/engine/effects.md
---

# 83. The skills' other effects: skill starts, rings, the shock wave, heals, cures, stat changes and the resistant shield

[[79]] ported the attack spells, the hits and the portals, but not what
the game shows around every other skill. A second effects agent has
added those to piney-effect.

## What is ported

- **A skill's start.**
  - `effSkillStart`: both forms, 0x001d35f0 by skill id and 0x001d3650 by
    `ccSkillParam`.
  - `effSkillStartEffect` (0x001d39d0).
  - The start's controller, which at count 7 calls the summons circle for a
    spell or the circle for a physical skill.
  - The exec and force rings (82-84), coloured by element.
  - The circles (85, 86).
- **The physical skills' shock wave.** `effPhysicalSkillHitShockWave`
  (0x001d2790): the element's smoke and debris, the waves 79-81, the
  camera shake and sound 35.
- **Heals and cures.**
  - `effHeal` and `effHealSkill`;
  - `effCure`, `effSanity` and `effResurrect`, with their generator tables;
  - `effOpenBox` and `effRemoveTrap`.
- **Stat changes.** `effAbilityUp` and `effAbilityDown` with
  `checkEffAbilityColor` and their seven textures; `setConditionEffect`
  uses them.
- **The resistant shield.** `effResistantShield` (gcmn 0x00501ab0,
  0x00501cb0) and its element on the ported element manager.

The API only grows: new starters, a new element, and two host methods
with defaults. effects.md lists which piney-battle outputs call which
starter, and when in the frame.

## Checked

- **Against the game.** `tools/test_effect_skill_rs.py` compares, every
  frame:
  - each start's result;
  - the slots, elements and draws;
  - the events;
  - both generators' states.

  Each group first sweeps every relevant value once:
  - all 304 skills;
  - every element;
  - heal levels -8 to 16;
  - conditions 0-40;
  - every shield case.

  Then come random cases. The final run covered 36,605 frames with 0
  mismatches.
- **Mutation check.** Eighteen single-constant changes to the port were
  made deliberately; the harness caught all 18.
- **The shield's animations.** They were added to the draw harness: 690
  animation draws, 0 mismatches.
- **Re-run in a clean worktree.** All seven effect suites pass, with the
  workspace's tests, clippy, fmt and the docs check.
- **Shots.** The rings and circle at Kite's feet, the fire blow's burst,
  the cure's glow, the blue shield.

**Still unknown:**
- **Not in the runtime yet.** The combat agent is wiring the new starters
  into piney-game's fx.rs.
- **Not ported:** the breakable objects' crush and fragment effects, and
  the bosses' own deaths.
- **A null pointer.** With no `affectPerson`, `effResistantShield` reads
  through a null pointer at 0x50. eemu has zero there; a real PS2 has
  kernel memory.
- **`effHeal`'s return.** The game always returns 0; the port returns its
  controller's slot, which the harness does not compare.
- **Not yet seen in a fight.** The new particle generator rows have been
  seen only in the shots.
