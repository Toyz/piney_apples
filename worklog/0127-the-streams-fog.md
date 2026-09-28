---
number: 127
title: "The streams' fog"
date: 2026-09-25
area: render
files: crates/piney-stream/src/scene.rs, crates/piney-stream/src/draw.rs, crates/piney-stream/src/effect.rs, crates/piney-stream/src/lib.rs, docs/engine/stream.md
---

# 127. The streams' fog

Seven of the effect tasks [[121]]-[[125]] ported set a fog on the scene's
draw environment: streams 2, 3, 4, 7, 13, 16 and 20. None of them was
drawn, so those cutscenes had no depth haze.

## SetFog

`ccDrawEnv::SetFog(near, far, nearRate, farRate, colour)` (0x00105820)
keeps these values:

- `fMax = 2.55 (100 - nearRate)` and `fMin = 2.55 (100 - farRate)`;
- `fogA` and `fogB`, the line from `fMax` at `near` to `fMin` at `far`.

The three-argument form (0x00105890) is that with rates 0 and 100: clear
to full fog. VU1 fogs every vertex of an object whose `fogSW` bit 3 is
set, at `fogB + fogA w`, held between the two. The tasks take a few
objects out of it: `Func_str0001`'s four sky objects and the others'
`OBJ_sfp7bac2`.

## The port

- **The fog.** `Scene::fog` is a `SceneFog`, and `Scene::fog_off` holds
  the objects taken out of it. The tasks set both at their start.
- **The draw.** Each model gets one `Fog` at its centre's view depth,
  the approximation the field and dungeon draws already use.
- **Stream 4.** `Func_str0110`'s cue x3 (`SetFog(0, 0, 0, 0, 0)`) takes
  the fog away from the next frame on.

## Checked

- **`scene::tests::fog_lines`** checks both forms: clear at `near`, full
  or held at `fMin` from `far`, and the colour bytes.
- **Shots.**
  - Stream 3's dungeon walls darken to its green-blue with distance.
  - Stream 7's floor takes its tint.
  - Stream 2 still looks as it did.
- **The packet fixtures** are unchanged: they do not include the models.

**Still unknown:**
- **Per-vertex fog.** The port's fog is per model, not per vertex, so a
  large model (a whole room) is fogged at its centre's depth throughout.
- **Stream 4's fog off.** What `SetFog(0, 0, ...)`'s divisions by zero
  leave in `fogA` and `fogB` on the EE is not worked out. The port takes
  it as no fog.
