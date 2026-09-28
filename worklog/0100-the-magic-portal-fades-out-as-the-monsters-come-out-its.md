---
number: 100
title: The magic portal fades out as the monsters come out: its objects at their own transparency
date: 2026-09-25
area: render, test
files: crates/piney-effect/src/portal.rs, crates/piney-effect/examples/draw_probe.rs, tools/test_effect_draw_rs.py, crates/piney-game/src/session.rs, docs/engine/effects.md
---

# 100. The magic portal fades out as the monsters come out: its objects at their own transparency

The user saw that on a field an enemy portal did not vanish right after
its monsters came out. It stayed up, full, and went away only about when
the battle mode ended.

## What the portal does

`ccMagicCircle::main` was already ported and checked in eemu
(`test_battle_spawn_rs`'s `circle_main` and `frame`), and it runs right
in the port.

**Its acts:**
- **Act 0:** it wakes when Kite comes within 3000.
- **Act 1:** after 32 frames it switches to the opening, `ANM_xmagcir2`.
- **Act 2:** after 22 frames the monsters come out
  (`entryCircleObject`).
- **Act 3:** it waits for the opening's 111 frames to end.
- **Act 4:** 66 frames later the entry control deletes it.

Logged on event 3's east portal: the monsters out at frame 181; the
opening's end at 268; the portal deleted at 337; the battle mode on at
220 and still on after 364.

**The opening's picture.** Its Anime chunk fades the portal's objects
through their alpha tracks (`tools/anim.py pose`):
- the ball flares to 0.98 at frame 30 and is gone by 60;
- the rings grow to twice their size, then shrink to nothing by 100;
- the sphere fades from 0.9 to 0 between 60 and 110.

`ccAnm::Draw` hands each object to `ccObj::Draw` at its own transparency
times the anm's, and draws none at or below 1/128. So the portal fades
out over the frames after the monsters appear. Its last frame drawn is
the opening's 108th, about 86 frames after the monsters. Act 4 passes with
only its last sparks.

## The cause

`portal::MagicCircle::render` is piney-effect's draw of the portal, the
one the runtime uses with the sparks. It drew every object at the
portal's own transparency and never read the pose's alpha. So the sphere
and rings stayed full until the entry was deleted, about when the battle
mode ends.

piney-world's own draw of the circle (`foe::Circle`) did apply the
objects' alphas; `test_foe_rs` checks it. But the runtime does not use it
once the effects draw the portals.

The render now goes through `nodes::anm`. That gives each object its
matrix and its own transparency, in `ccAnmIndex` order; the palette swap
and morphs are kept.

## Checked

- **eemu: `ModelsAgainstGame`.** `tools/test_effect_draw_rs.py` now runs
  the game's `ccAnm::Draw` / `ccClump::Draw` on `XMAGCIR.CCS` as well. The
  file is decoded by the game's own `DecodeSetup`, and `draw_probe` adds
  the same file after the effect files, as the field does. The cases:
  - the clump;
  - `ANM_xmagcir1` and `ANM_xmagcir2`, stepped 1 to 179 frames at 256,
    into the opening's held end.

  In all: 1,140 clump draws (896 models) and 750 animation draws (2,872
  models). 70 of the draws are the portal's (107 models). 0 mismatches in
  models, order, transparency and lwMatrix (worst 2.3e-6 relative).
- **A unit test.** `the_opening_fades_the_portal_out`: the ball is drawn
  at the opening's frame 29 and gone by 70; the last frame with anything
  drawn is between 100 and 110 (it is 108).
- **Shots.** `portal_shots` is an ignored test. Kite walks up to event
  3's east portal, the camera reset behind him. A shot every 10 frames,
  from the wake to 40 frames after the deletion.
  - Before the fix (`scratch/combat/portal/sheet_before.png`): the full
    cage sphere through act 3 and act 4, gone at the deletion, with
    "BATTLE MODE OFF" 20 frames later.
  - After (`portal_after/sheet_after.png`): the goblin comes out and the
    sphere fades to a sliver and a glint. Nothing is left by act 4, the
    last sparks aside.
- **The other checks.** `test_effect_rs` and `test_foe_rs` pass. So do
  the workspace's tests, clippy, fmt and the docs check.

**Still unknown:**
- **Lights.** The portal is drawn unlit here (`lights: None`). The foe
  path lights it with the town lights. Whether `ccAnm::Draw` of a fog-off
  clump takes the field's lights was not checked.
- **The minimap's markers.** They are faded by the map's own rule
  (`dest_flag`, `+0xec`), which `test_map_rs` covers. The shots here were
  not checked against the game frame by frame.
