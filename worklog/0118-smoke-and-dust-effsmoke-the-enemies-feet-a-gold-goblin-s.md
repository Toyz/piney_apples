---
number: 118
title: "Smoke and dust: effSmoke, the enemies' feet, a gold goblin's run and an idol opening"
date: 2026-09-25
area: battle, render, test
files: crates/piney-effect/src/dust.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/examples/particle_probe.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-world/src/combat/dust.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/examples/dust_probe.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, tools/test_effect_particle_rs.py, tools/test_dust_ctrl_rs.py, docs/engine/effects.md, docs/engine/particles.md, docs/engine/battle.md
---

# 118. Smoke and dust: effSmoke, the enemies' feet, a gold goblin's run and an idol opening

The battle's rules already raised three dust calls, and nothing drew
them:
- `Call::DustCtrl`, from the races' `exclusive()` and their footstep
  notes;
- `Call::Dust`, a gold goblin running;
- `Out::DustRing`, an idol opening ([[116]] left it).

All three end in `effSmoke`, which was not ported either.

## effSmoke and the dust (`piney_effect::dust`)

- **`effSmoke`** (main 0x001ce300) is one particle on effect.cpp's static
  `ccpgSmoke`. Its life is shortened by up to half at random, and it takes
  the velocity, size and fades it is given, plus a random turn.
- **`ccEnemyEffDust`** (gcmn 0x0043a3e0) makes n puffs at random headings
  (`ccRandF(pi)`), 10 s out, flying out and up.
- **`ccEnemyEffDustRing`** (gcmn 0x0043a590) makes n puffs evenly round
  from a random start, r out. Under 4 it is the plain dust instead.
- **The character overload** (0x0043a7a0) places the ring at the
  character's feet (FW2LW) plus its turned offset.

## The enemies' feet (`piney_world::combat::dust`)

`ccEnemyDustCtrl` keeps a node per `ccEnemyDustInfo` row with the last
frame bucket it fired in. `ctrl(obj, flag)` fires a row when:
- the enemy is shown, its `anmNum` is the row's (2 and 3 match either),
  and its `frameNum` is inside the row's window;
- and either:
  - on a note, for rows with `every` 0;
  - every frame, when the window is one frame;
  - once per bucket of `every` frames otherwise.

The rules now send what `ctrl` reads with the call (`DustAt`: the rows,
`dispSW`, `eneSmoke`, `anmNum`, `frameNum`, `pos`, `dirc`). The rules'
harnesses print only the flag, so they still match.

The controller draws nothing random. The runtime runs it in the effects'
pass over the frame's shows (`Combat::fx_call`), in order: a race
constructor's `Out::Dust` sets the nodes, and each `DustCtrl` is followed
by the `Show::DustRing`s it raised. fx.rs starts:
- those rings;
- the gold goblin's dust (FW2LW, then `ccEnemyEffDust(p, 1, size, 6,
  eneSmoke)`, arguments checked in its `exclusive()`);
- the idol's ring (`(0, 800, -530)`, 4, 100, 8, 30, 132).

## Checked

- **`test_effect_particle_rs.py`'s `test_smoke_and_dust`.** The game's
  `effSmoke`, `ccEnemyEffDust` and both rings run in eemu over the real
  particle system, beside `particle_probe`. The machine's `genrand` and
  the probe's are now the same MT19937 seeded alike. Ten cases in towns
  and fields: random sizes, lives, textures and ring counts, a character
  turned, the puffs run out. 904 frames of every particle's whole state
  match. The whole particle harness (15,263 frames) and the skill harness
  still pass.
- **`tools/test_dust_ctrl_rs.py` (new).** It lays a controller out in eemu
  over three races' real tables and a made-up one (rows of anm 2 and 3,
  a one-frame window), and runs the game's `ctrl` for 3000 steps each:
  animations changing, frames advancing, notes, `dispSW` off now and then.
  `dust_probe` runs the port's `ctrl` alongside. Every step's rings match,
  and every row fires somewhere.
- **`event_3_plays_its_fights`** now also asserts dust. The portal's
  goblin (row 130, type 1) has one row, its fall (anm 10, frame 35), and
  the fight raises exactly that one ring.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **`effSmoke`'s other callers** are not ported: the Grunties
  (`ccPGuso`), `WORLD::DrawSteam` and `DrawEffect`, `ccGimSymbol`,
  `ccEnemyBrPart::explode`, `ccDog` and the bosses' back dashes. The
  party's running dust (`ccEffPawSmoke`) is a separate function and is not
  ported.
- **No shot of the dust yet.** The one fight in the session tests raises a
  single ring as the goblin falls.
