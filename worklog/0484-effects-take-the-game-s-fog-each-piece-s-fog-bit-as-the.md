---
number: 484
title: Effects take the game's fog: each piece's fog bit as the game sets it, the area's SetFog in the effects' draw environment, a sprite's FOG drawn; no effect casts a shadow
date: 2026-10-25
area: render
files: crates/piney-effect/src/effect.rs, crates/piney-effect/src/draw.rs, crates/piney-effect/src/drawelm.rs, crates/piney-effect/src/sprite.rs, crates/piney-effect/src/host.rs, crates/piney-draw/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-desktop/src/soft.rs, crates/piney-world/src/town.rs, crates/piney-game/src/fx.rs, tools/test_effect_spell_rs.py
resolves: 284.1, 284.13
started: 2026-10-25T17:33:16-06:00
took: 40m
---

# 484. Effects take the game's fog: each piece's fog bit as the game sets it, the area's SetFog in the effects' draw environment, a sprite's FOG drawn; no effect casts a shadow

Play question [[79]]/[[231]] (284.1) and the sprites' fog from the same
entry: which effect objects the game fogs and shadows, and whether a
sprite's FOG reaches the picture. The port drew every effect clump and
anim without fog, and every `ccEff` sprite without its FOG. The effects'
camera held a `Reset` draw environment, not the area's fog.

**The fog bit.**
- `ccObj::Init` (main 0x0013b5c0) sets every object's fog bit (+0xa0 bit
  3), and its shadow bit (bit 2) when it has a shadow model.
  `ccObj::Draw` (0x0013f33c) makes PRIM.FGE from bit 3.
  `ccClump::SetFogSw` and `ccAnm::SetFogSw` (0x0013d6c0, 0x00152a60) set
  it on every object.
- Their callers are the same on all four volumes (checked with the
  relocations and `--scan`):
  - Off: `InitEffect`; the fire, fireball and plasma elements' anims and
    the fire's shade; the ring, thunder, thunderbolt and magic-circle
    elements; the particles; the portal.
  - Back on (`SetFogSw(1)`): `effMeteoFireBall2` (dark, and soil),
    `effSmokeRock`, `effIceRock`, `effSmokeIce`, `UpheavalSystem`'s
    pillars, `effUpheavalFragment` kinds 0 and 1, and four pieces by
    attribute: `effSkillTornadeObjPos`, `effRadiateSomething`/`2` and
    `effSmmonsFragment` with water or soil, `effSkillChargeObj` with dark,
    wind or water.
  - On: everything else - the rock, dark, ice, blantch and needle
    elements, the trees, the drills, every boss's own clumps and anims,
    the resistant shield. No boss class calls `SetFogSw`.
- The port: `Effect::fog_sw`, `AnmObj::fog` and a `fog` on
  `DrawRec::Clump`/`Anm`. A fogged one draws with VU1's fog by depth from
  the camera's draw environment (`DrawEnv::depth_fog`).
- The area-level calls (the towns' skies, clouds and water, the
  `EVENTAREA`s' backgrounds, `WORLD::Init`'s four background clumps and
  `SetLake`, the dungeons' lake backgrounds and `SetWater`) were already
  drawn without fog by their modules. "Only the gate was fixed" was out
  of date.

**The draw environment.** `ccDrawEnv::active` is `cc3d` from
`InitCCSys` (0x0010a5d0) on. Only `ccThDrainEnemy` and `ccThStrParty`
swap it for a moment. Every area's `SetFog` writes `cc3d`. So the effects
draw under the area's fog:
- `TownLights::fog` now holds `SetFog`'s arguments (`FogParams`), so the
  effects can build the environment with the EE's arithmetic.
- `FieldFx::set_fog` hands the place's fog over each frame, and the town
  passes `World::fog`.
- `Camera::fogged` writes it into `env`.

**The sprites.** `GsSprite` already held FOG's F (at the centre's w) and
FOGCOL, checked against the game; `prim` dropped them. Now
`piney_draw::DrawState::fog` carries them when PRIM.FGE is set, and
piney-gs (`Params::FOG` with the fog word) and the soft renderer put the
primitive through the fog stage, `(Cs F + FOGCOL (0xff - F)) >> 8`. The
effects' own sprites still clear FGE. The bosses' particles, a town's
sprites with depth and a story map's keep it, and now fog.

**Shadows.**
- Of the effect files only x703's objects 6 and 10 (the drill's) have
  shadow models, and `DecodeShadowModel` refuses both. Its refusals are
  checked against the game for every shadow model in `DATA.BIN`. So no
  effect casts a shadow.
- The drill's `ccAnm::SetShadowSw(1)` and its case's draw environment
  change nothing seen. At 0x001c7234 it sets alpha +0xa4 to 128 and
  length +0xa0 to 450.0, and at 0x001cbff4 it puts the alpha back to 128
  but leaves the length. Every caster that reads the length sets its own
  first (`ccChar::Draw`, `ccBoss::Draw`, `TOBJ::Draw`, `ccGimBox::main`,
  ...), as the port's casters do.
- `ccTreeUpheavalMngrElement`'s `SetShadowSw(0)` is on a tree with no
  shadow model. effects.md said `ccObj::Draw` skips the shadow model
  below 1/128; it draws it whatever the transparency, and the page is
  fixed.

Checks:
- `tools/test_effect_spell_rs.py` now hooks both `SetFogSw`s (`ccObj::Init`'s
  bit at `ccClump::Init` and `SetAnm`). Each frame it compares every clump
  and anim draw's bit with the probe's new `fog` list. All five spell
  systems pass (2 random casts of every spell, 400 frames). A sample of
  nine spells drew both 0s (CMP_x300) and 1s (CMP_x101-x202) the same as
  the game.
- `a_fogged_sprite_carries_its_f`: Mac Anu's fog gives the environment
  `DrawEnv::set_fog` does, F 154 at w 4000, and no fog without FGE.
- `a_fogged_primitive_takes_its_f` in piney-gs and the soft renderer.

**Still unknown:** nothing
