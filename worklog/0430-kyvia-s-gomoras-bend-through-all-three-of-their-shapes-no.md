---
number: 430
title: Kyvia's gomoras bend through all three of their shapes: no gomora picks one of CMP_ex01gom2-4, MPH_ex01gom1 bends MDL_ex01gom1 through them by each gomora's own clip, and the port's bodies now draw their clips' morphs, checked against the game's ccAnm and GomoraInit on both discs
date: 2026-10-22
area: battle, render, test, tooling
files: crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, crates/piney-world/src/body.rs, crates/piney-world/src/foe.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session/tests/kyvia_fx.rs, tools/test_battle_kyvia_rs.py, docs/engine/boss-kyvia.md, docs/engine/boss-kyvia-second.md, UNKNOWNS.md
started: 2026-10-22T18:45:28-06:00
took: 25m
---

# 430. Kyvia's gomoras bend through all three of their shapes: no gomora picks one of CMP_ex01gom2-4, MPH_ex01gom1 bends MDL_ex01gom1 through them by each gomora's own clip, and the port's bodies now draw their clips' morphs, checked against the game's ccAnm and GomoraInit on both discs

Worklog 272 left open which of x01's `CMP_ex01gom2`, `3` and `4` each
gomora draws (boss-kyvia.md Unknown, UNKNOWNS.md). The port drew every
gomora from `CMP_ex01gom2`.

## What the game does

No gomora picks one of the three. They are morph targets, and every
gomora shows all three in turn.

- **The stream.** `kyviaCore::CoreInit(lv)` (MUT gcmn 0x004f82c0) makes
  the five gomoras and calls `GomoraInit(lv)` (0x004fe460) on each with
  the core's own level. `GomoraInit` switches on the level, not on the
  attribute:
  - levels 1-4 take `GetCCSAdrs("x01")` to `"x04"` into the stream
    (+0xcc), `kyviaGomoraAnmTbl` into +0x1a8, and `AllGomoraList_lv`;
  - then `ANM_ex0gnut0` of that stream into a new `ccAnm` (+0xd4).
  `CoreInit` takes the same streams for the core. So all gomoras of a
  fight share one stream: x01 in Mutation's fight, x02 in Outbreak's
  second.
- **The draw.** `ccBoss::Draw` (0x00472e50) sets the fog blend and the
  transparency, then calls `ccAnm::Draw` on +0xd4. No gomora code calls
  `ChangeClut`, `SetUV` or a clump. The anm draws its clip's objects:
  `ANM_ex0gnut0`, `atc0`, `dmg0` and `mag0` move `OBJ_trall3`,
  `OBJ_ex01gom1` and `OBJ_ex31gomc`, the nodes of `CMP_trall3`. Only
  `MDL_ex01gom1` (lit) and `MDL_ex31gomc` have drawn models.
- **The morph.** `ccAnm::SetAnm` (main 0x00150fd0) makes each anm its own
  `ccMorpher` (`new ccMorpher` at 0x001518a8). `DecodeF_Morpher`
  (0x0014e760) sets its targets and weights from the clip's `F_Morpher`
  records through the anm's own index. So each gomora bends by its own
  clip and frame.
  - `MPH_ex01gom1` bends `MDL_ex01gom1`.
  - `ANM_ex0gnut0` (41 frames, a loop): toward `MDL_ex01gom2` over frames
    1-10, from it to `gom3` by 20 and `gom4` by 30, back to the base by
    40.
  - The attack, damage and magic clips: toward `MDL_ex01atc1`, `atc02`
    and `atc03`. In one frame the weights can add up past 1 (0.99 and
    0.027).
- The targets are full models (mtype 0, not position-only), each with
  its own one-node clump. That is why they looked like models to pick.

The same holds for the core (`MPH_ex0c_out`) and for Kyvia's body
(`MPH_ex0bbod1`, `2`). x01 and x02 have the same names on Mutation and
Outbreak.

## What the port did

`piney_world::body::Body::draw_parts` drew every model unbent
(`morph: Vec::new()`). Only `foe::Model` (enemies) and the arm's `ccAnm`
draw applied the clip's weights. So:

- each gomora drew the bare `MDL_ex01gom1`, its clump taken as
  `CMP_ex01gom2`;
- the core and the body never bent either.

## The port

- piney-battle:
  - `kyvia::stream(lv)` is `CoreInit`'s and `GomoraInit`'s stream.
  - `Fight::file` returns the stream at its level.
- piney-world:
  - `Body` keeps its file's morphers (`MPH_` to `MDL_`).
    `Body::morph_of` gives a model's targets from `Play::morph`, and
    `draw_parts` draws each model of the body's own file with them.
  - `foe::Model` drops its own copy of the map and uses `Body`'s.
  - `BossLook`'s `body_morphers` is gone; the arm's draw reads the
    body's.
  - The parts' clumps are `CMP_trall2` and `CMP_trall3`, of
    `Fight::file`.

## Checks

- `tools/test_battle_kyvia_rs.py`:
  - Each part's stream (+0xcc, the name `CoreInit` or `GomoraInit` asked
    `GetCCSAdrs` for) is compared every frame with battle_probe's
    `kyvia::stream`.
  - `bulk` agrees: Mutation 120 from 9300 (161,184 frames, stream x01),
    Outbreak's second fight 60 (180,064, x02), its first 30 (41,061,
    x01).
  - The new `test_parts_shapes` steps the core's and the gomoras' clips
    of the fight's file in the game's `ccAnm` (`_AnimateForward`, the
    morpher's targets and weights each step) against `tools/anim.py`, at
    two speeds, on all three fights. `anim.py`'s weights are the ones
    the port draws with (the morph fixtures of `piney-data`'s
    `tests/anim.rs`).
- `session::tests::kyvia_fx::kyvias_gomoras_bend_through_their_shapes`
  plays 6,000 frames of each fight and reads the gomoras'
  `MDL_ex01gom1` draws:
  - Mutation: drawn in 5,319 frames, bent apart (two gomoras unlike) in
    5,237; gom2 10,544 draws, gom3 10,214, gom4 9,689, atc1 2,162, atc02
    1,789, atc03 1,051.
  - Outbreak: 5,324 and 5,242; 5,757, 5,664, 5,091, 978, 1,747 and 798.
  - Before, every draw had no targets.
- piney-world and piney-battle tests, piney-game's suite (four threads),
  clippy, fmt.

Shots (`kyvia_gomora_shape_shots`, /mnt/data/claude/scratch/a430): every
50th frame the gomoras are drawn. Each file is named by the first
gomora's targets and weights in percent.

- `gomora-x01-00750-act0-gom390-gom410.png`: four gomoras, each a
  different shape.
- `gomora-x01-01250-act0-atc196.png`: the attack's shape.
- `gomora-x02-01200-act0-gom360-gom440.png`: Outbreak's second fight.

**Still unknown:** nothing.
