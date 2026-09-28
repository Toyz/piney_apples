---
number: 81
title: The fight's sounds and the spells: ccSeOn3D and the sound tables, and attack spells cast through the effects
date: 2026-09-24
area: audio, battle, test
files: crates/piney-audio/src/se3d.rs, crates/piney-audio/src/setbl.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/src/driver.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, tools/test_sound3d_rs.py, tools/sound_tables.py, tools/iopemu.py, docs/engine/sound.md, docs/engine/battle.md
---

# 81. The fight's sounds and the spells: ccSeOn3D and the sound tables, and attack spells cast through the effects

[[80]]'s fights were silent, and spells did nothing on screen. The combat
agent's second increment adds both.

## The sounds

- **Positioned sound.** The game places a fight's sounds in space:
  - `ccSeOn3D`, `ccSeOn3DNote`, `ccSeOn3DLoop` and `ccSeOffLoop` work out
    each sound's volume and pan from where it is (`calcVel`, `calcPan`);
  - the parameters come from setbl.cpp's tables through
    `ccSeSetParamEnemy`, `SPC`, `PC` and `Inu`.
- **The port.** piney-audio has them as se3d.rs. setbl.rs holds the
  tables, generated from the executable by `tools/sound_tables.py setbl`.
- **In the runtime.** piney-game plays the battle's sounds, the effects'
  sounds and ccThSkill's heal and note sounds, heard from the active
  camera.

## The spells

ccThSkill walks the running skills itself. So an attack spell's element
system (Fall, Tornado and the rest, from [[79]]) runs in `ccSkill::Main`'s
place, and its damage calls go back to piney-battle's `skill_damage` with
the run's flags, on the frame the game makes them.

## Checked

- **Against the game.** `tools/test_sound3d_rs.py`: 9 checks at 50,000
  cases each, 0 mismatches. The sound chip's register writes (MODHSYN)
  also match over 350 fixture cases.
- **Runtime test.** `a_spell_lands_through_the_effects`: Kite's Tornado
  hits a portal goblin. This tests the port against itself, not the game.
- **Re-run in a clean worktree.** The positioned-sound, sound, world,
  field, skill-flow and game-control suites pass, with the workspace's
  tests, clippy, fmt and the docs check.

**Still unknown:**
- **Not reached yet.** The dungeon entry and event 4 are the combat
  agent's next increment.
- **Other agents in the same files.** The stream presentation agent also
  works in piney-audio. This increment's additions to lib.rs and
  driver.rs are small and additive.
- **Not done yet:**
  - the party AI's chat lines;
  - Data Drain's presentation;
  - game over;
  - `boss()` for the events.
