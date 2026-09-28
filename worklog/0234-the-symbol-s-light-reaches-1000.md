---
number: 234
title: The symbol's light reaches 1000
date: 2026-09-27
area: render
files: crates/piney-world/src/field_world.rs, crates/piney-battle/src/gimmick.rs
---

# 234. The symbol's light reaches 1000

Reported from play: on Mutation's first field (Θ Chosen Hopeless
Nothingness, area 1 field 27), the lighting flickered and looked wrong.

## Finding it

- **The flicker.** Frames of the field with Kite standing still showed
  his body's brightness jumping every frame, between 37.7 and 42.4. The
  field's own light is fixed: frame 0 of the background's animation.
- **The source.** A debug print of the group the characters are lit by
  (`FieldWorld::cast_lights`) showed one extra light: the field's symbol
  (`ccGimSymbol`, row 17), about 2,500 from Kite. `symMain` sets its
  intensity each frame to 1 + `ccRandF(0.2)`. That is the game's own fire
  flicker.
- **The mistake.** The port gave the light a reach of "full to 1000,
  nothing past 1000000", white. So the whole field was lit by a white
  light that flickered.

## The game's light

`ccGimSymbol`'s constructor (INF gcmn 0x0045a0d0, the same values on
Mutation at 0x0046fc0c) writes the omni light's fields:

| offset | field          | value                  |
|--------|----------------|------------------------|
| +0xc4  | `farDownStart` | 0                      |
| +0xc8  | `farDownEnd`   | 1000 (0x447a0000)      |
| +0xcc  | `farDownEnd2`  | 1000000 (0x49742400)   |
| +0xb0  | `color`        | `ccSetColor(color, 0x7fff, 1.0)` |

`ccOmniLight::CheckRange` (0x00139830) compares the squared distance
with +0xcc. It then fades the light linearly from `farDownStart` to
`farDownEnd`. So the port had read the squared limit as the end, and the
end as the start. The colour 0x7fff is red 255, green 127, blue 0
(`ccSetColor` puts the low byte in red). That is an orange, the
symbol's flame.

## The fix

`symbol_light` in `field_world.rs` builds the light as the game does:
- orange (1, 127/255, 0);
- full at the flame;
- nothing 1000 away.

The springs' lights (`ccGimEtc`) and the effects' (`prim::OmniLight`)
already had the three fields right.

In the same run after the fix, Kite's brightness stands at 25.3 (±0.3).
Characters on the field are darker, lit by the field's own light alone
until they come near the symbol.

The new test `a_symbol_lights_only_near_it` puts the light 500 above a
model and checks the colour: half its orange, doubled as VU1 does. At
1000 and at 2500 it checks for none.

**Still unknown:** No picture of the real game on this field to compare
the overall darkness with. The field's light values themselves are
harness-checked (`test_field_rs.py`).
