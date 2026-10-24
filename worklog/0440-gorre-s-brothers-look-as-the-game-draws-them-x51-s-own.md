---
number: 440
title: Gorre's brothers look as the game draws them: x51's own faces, backs and orbs, never shaded, lit by the arena's green distant light, the close-ups being streams 71 and 72 and the flat teal the trail's own colour
date: 2026-10-24
area: render, battle, test
files: crates/piney-game/src/session/tests/boss_fx.rs, crates/piney-world/src/field_world.rs, docs/engine/boss-gorre.md
started: 2026-10-24T00:59:16-06:00
took: 13m
---

# 440. Gorre's brothers look as the game draws them: x51's own faces, backs and orbs, never shaded, lit by the arena's green distant light, the close-ups being streams 71 and 72 and the flat teal the trail's own colour

After worklog 437 the coordinator questioned the brothers' look in the
port's shots. The fight showed a green scaly pod and a flat teal shape.
The reporter's cinematic shot (issue #73's c.png) shows grey-pink carved
stone with faces, eyes, teeth, and a yellow and a blue orb. This entry
checks each step from the game's code and the disc's data. Nothing in
the port's draw is wrong. The look is the arena's own light, the faces
turned away, and the trail's colour.

## What a brother draws

- **Clump and clip.** `ccBoss05Brother::Init` (OUT gcmn 0x004ae990)
  takes `x51` (`GetCCSAdrs`). Its anm gets the clip of
  `boss05SlaveAnmTbl1`/`2` by act: `ANM_ex51nut` and `ANM_ex52nut` in
  the normal fight, `ANM_ex5xnut` in the Epitaph. They pose `CMP_trall`,
  `CMP_trall2` and `CMP_trallx`'s objects (worklog 437's `clip_body`).
- **Textures and palettes.**
  - Each body object draws its file's own materials: `TEX_ex51bod1`
    (the carved grey face), `TEX_ex51bod2` (the eyes, the teeth and a
    silver disk), and `TEX_ex51bod3` / `TEX_ex51bod4` (the back: scaly
    stones round a yellow or a blue oval).
  - The orb object draws `TEX_9504eye-y` or `-b`.
  - Each texture has its own `CLT_` (`tools/ccstex.py ls`).
  - Nothing in boss05.cpp (0x004a9130-0x004b0b50) changes a palette or
    a texture. No call of `ccClump::ChangeClut` (either form),
    `ccClump::ChangeTex`, `ccModel::ChangeClut`/`ChangeTex` or
    `ccAnm::ChangeClut` lies there. The bosses' only callers are
    effects: the thunderbolts, the transform and the ice missile.
- **Place and turn.** The brother's `PreDrawAnm` is `ccBoss`'s (vtable
  +8 = 0x0046e6c0). It steps the anm, then
  `SetMatrix_PosRotZYX(+0x40, +0x60)`: the character's place and its
  `dirc`. The rules harness already compares these each frame.
- **Light.** `ccBoss::Draw` (0x0046e730) draws with the field's light
  group. It sleeps the distant light only when `hitAttribute` (+0x80)
  has 0x40000. No `ccBoss` function writes +0x80, and
  `ccCharHit::CollisionDetection` keeps its attribute in the hit
  (+0x40), not in the character. So a brother is never shaded, as the
  port draws it (`a.ch.hit_attribute` 0).

The brothers are lit by the field's light group:

- ambient (0.243, 0.212, 0.071);
- one distant light (0.349, 0.435, 0.318) from (0.5, 0.5, -0.707);
- red omni lights (1.0, 0.184, 0.098), reaching 600 to 800, round the
  arena's edge.

## From the disc alone

`gorre_brothers_model_shots` (ignored) draws each clump at its neutral
clip from `x51` alone through the port's model draw and the GS renderer.
It draws them turned 0, 90, 180 and 270 degrees, once under a white
ambient and once under the arena's lights:

- Under white, they are what the textures say: the carved grey face
  with its orb in front, the scaly back with its yellow or blue oval
  behind, a thin edge from the side.
- Under the arena's lights the same models turn olive. The green
  distant light and the yellow ambient tint the grey and blue stones.

That is the fight's pod. In the normal fight the brothers turn toward
Gorre between them (`OnThinkNeutral`), so from behind Kite the camera
mostly sees their backs or edges. Frame 1400 shows both faces: one
olive, the other orange near a red omni light.

## The cinematic and the trail

- **c.png is a stream.** The Talk's stream 71 (and the Data Drain's 72)
  draws the brothers in close-up with its own models and lights, faces
  open on the eyes and teeth. The port's stream frames 3400 to 3700 match
  the reporter's shot.
- **The flat teal shape is a trail.** It is the brothers' after-image of
  the Wave's spin. `EntryAfterImageAnm`'s own params for a null param
  give colour 0x00505000 at `colorAlpha` 100. Through
  `ccSetFogBlendColor` that is fog coefficient 2.55 x (100 - 100) = 0:
  every vertex in the colour, red low, so teal. The harness checked these
  calls in worklog 437. The picture follows the blend rule in
  docs/engine/battle.md.

## Shots

- /mnt/data/claude/scratch/a440/side.png: c.png beside the port's stream
  71 (frames 3500, 3400) and the fight (frames 1500, 1400, 200).
- /mnt/data/claude/scratch/a440/models.png: the models alone, white and
  arena-lit.
- The textures: /mnt/data/claude/scratch/a440/texm2.png.

`gorre_brothers_shots` now runs 4,000 frames, so it reaches stream 71.
`Place::lights` is public for the model shots.

**Still unknown:** nothing.
