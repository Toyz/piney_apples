---
number: 411
title: Innis's magic shows its missiles: the crystals, bolts and fireballs fly with their trails and burst, its images' rings and shards and its particles are drawn (#69)
date: 2026-10-21
area: render, battle, test, tooling
files: crates/piney-effect/src/innis.rs, crates/piney-effect/src/boss.rs, crates/piney-battle/src/boss/innis.rs, tools/test_boss_effect_rs.py, crates/piney-game/src/session/tests/boss_fx.rs
started: 2026-10-21T08:55:01-06:00
took: 29m
---

# 411. Innis's magic shows its missiles: the crystals, bolts and fireballs fly with their trails and burst, its images' rings and shards and its particles are drawn (#69)

Issue #69 (ThreePendant, build 3e87cd5, Mutation): fighting Innis, "the
particle effects from magic do not appear to load at first". Absolute Zero
showed nothing, Demon Fire only its fire tornado. None of Innis's own
pictures had been ported ([[266]] left them open): its three missiles,
the rings its images and its fall make, its particle generators, the
shield on a spell's hit and its images' mirror shards. What the player
saw was the skill `MagicDamage` lands at the end (the fire tornado is
skill row 7's spell); before it, the square's glow and nothing else.

## What the game draws

From Mutation's code (gcmn; the addresses in docs/engine/boss-innis.md,
"The pictures"):

- **The missiles.** Each constructor loads its picture and starts its
  trails, `MissileSmokeGenerator` rows following the missile's place and
  turn (`syncPos` +0x10, `syncRot` +0x20, distSW off): the ice's
  `CMP_x202a` (row 0), the lightning's `CMP_x500` (rows 1, 2), the fire's
  `ANM_x300` (rows 3, 4). In flight each `Draw` draws it at
  `SetMatrix_PosRotXYZScale(pos, rot, 3)`, the lightning and the fire at
  transparency 0.5.
- **The landing.** Every missile's trails are killed. The first (NO 1)
  then bursts: `BurstSmokeGenerator` rows by element; the ice two
  `effIceRock`s turned by -pi/2 and a `ccEnemyEffDust(pos, 10, 5.0)`; the
  fire the dust too, and its `ccAnm` set to `ANM_x305`. After it the ice
  draws xeffect's `ANM_ex31lhit` (scale 5) and `ANM_x305` (2.5) with
  `x305`'s substitutes `OBJ_x048`-`051` on their `c1` palettes; the fire
  its `ANM_x305` at scale 4, set again at each end.
- **The rings.** `ccEffSamonRing::SetModel` is one switch on every volume:
  five sets of a short number and seven long ones, plus 34. Innis's images
  burst into 191 and 193, its fall raises 192 (`CMP_x037` of xeffect on
  `c1`, `c3`, `c2`); Magus's dead leaves' 195 is the same set's `c5`.
- **The generators.** `InisFieldGenerator` rows 2, 3, 0, 1 at the first
  pattern, row 4 over a skill's target; `TornadoGenerator` at the escape;
  `BurstGenerator` rows 2 (the fall), 3 and 4 (an image's burst).
- **The shards.** `BreakMirror::Mbreak` draws each shard of x21's
  `CMP_ex21ef{ID}1`-`5` with `DrawPartsClump` on objLayer at its place and
  turn (`SetMatrix_PosRotZYX`).
- **The shield.** `ccBoss02::Affect` on a spell's hit: `effResistantShield(this,
  1, -1)`.

## In the port

- piney-battle: `innis::Missile::step` is the `Draw` as a pure step with
  what it asks in order (`innis::Call`: sounds, the lightning's WaveShock,
  the quake, `MagicDamage` due) and what it drew (`Missile::shown`:
  flight, landed, burst). The boss's manager pass makes the calls as
  before. The rules harness (`tools/test_battle_innis_rs.py`) and the other
  bosses' still pass.
- piney-effect `innis`: a missile made at its `Out::Effect` and drawn
  after the boss's task from the flight its rules left
  (`BossEffects::sync_missiles`), its trails on anchors, its landing's
  calls there. The rocks' and the dust's `rand()` thus come after Innis's
  task, not in the manager's pass before it; while missiles fly the task
  draws none. A manager can also run one alone (`Make::MissileAlone`, the
  probe's). `innis::start` starts the generators.
- piney-effect `boss`: `ring_model` is `SetModel` whole; `Make::SamonRing`
  makes Innis's rings; `WaveShock::begin` lets a missile's landing make its
  wave in the game's order.
- `ccAnm` draws carry their objects' palette swaps (`AnmObj::change_clut`).
  The spells' explosion (`ccExplodeElement`, [[79]]) gets its elements'
  palettes with it; `the_explosions_substitutes_own_their_palettes`
  checks the substitutes' own palettes on the disc.
- piney-gen: the five tables and their force fields (`boss_inis_field`,
  `boss_tornado`, `boss_burst`, `boss_missile_smoke`, `boss_burst_smoke`);
  data version 32.
- piney-world: `BossRun::shards` from the images' mirrors, drawn on
  objLayer; the census counts the generators following an anchor.
- piney-game: `Out::Particles` and `Out::Shield` started; the missiles
  synced after `boss_sync`.

## Checks

- `tools/test_boss_effect_rs.py`'s `test_innis_effects`
  (`PINEY_VOLUME=mutation`): each missile's real `Create` (each element as
  the first and as a later one, no callback, no camera) and the rings'
  (191-193, their burst set) run with the manager's pass in eemu against
  boss_probe's `missile` and `sring`, every frame until the slots are
  free: the draws (clump and its palette swap, matrix, transparency;
  `ccAnm` and its frame), the sounds, the generators started (row, force
  fields, flags, place, what they follow) and killed, the rocks' effect
  slots, `rand()` and `genrand()`. Two passes of nine cases, 1,328 frames,
  agree. The clump records now carry the palette swap, and Skeith's and
  Fidchell's cases still agree on Infection and Outbreak. Stand-ins: the
  explosion's substitutes have no model there (their palettes are the lib
  test's), and the missile's +0x40, which its constructor leaves unset, is
  taken as (0, 0, 0, 1).
- `session::tests::boss_fx` on Mutation's event 107, the gauge at half
  for the magic's run and each magic's element forced in turn:
  `innis_magic_shows_its_missiles` (before: none of `CMP_x202a`,
  `CMP_x500`, `ANM_x300` or the bursts drawn; after: each in flight 260,
  400, 230 frames, the bursts, trails following them) and
  `innis_images_burst_into_rings_and_shards` (the images' rings 75 frames,
  the fall's, shards 162 frames).
- `ring_model`'s unit test; piney-effect, piney-battle and piney-world
  tests, piney-game's suite (four threads), clippy, fmt, `piney-gen gen
  --check`.

Shots (/mnt/data/claude/scratch/i69): `after-innis-00520-act4-w19.png`
(the ice's crystals, forced while the square named Demon Fire),
`after-innis-00570-act4-w19.png` (its burst), `after-innis-02280-act4-w19.png`
(the lightning), `after-innis-03400-act4-w19.png` (the fireballs),
`after-innis-01400-act5-w17.png` (the images' shards and rings).

**Still unknown:**
- the blur (`ccBossBlur`) is drawn for no boss ([[117]]: it depends on what the heap left)
- Kyvia's (Mutation, Outbreak), Magus's and Gorre's own pictures are still not drawn (docs/engine/boss-kyvia.md, boss-magus.md, boss-gorre.md): the same fault as Innis's, each its own port
