---
number: 165
title: The screen shake: cameraShake, cameraShockAbsorber and cameraSet's offset
date: 2026-09-26
area: render
files: crates/piney-world/src/camera.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-world/examples/shake_probe.rs, tools/test_camera_shake_rs.py
---

# 165. The screen shake: cameraShake, cameraShockAbsorber and cameraSet's offset

Hits and spells did not shake the screen. The effects already raised
`Event::CameraShake` for the criticals, the spells' blasts and the
shockwaves, but nothing took the events. The enemies' note 0x8002 asked
`checkCameraShakeRange`, which answered no. `docs/engine/field-game.md`,
"The screen shake", has the rules.

`camera::Shake` holds camera.cpp's eight `camShockForce` slots and the
vibrate statics:

- `cameraShake` (main 0x00162cd0) keeps or drops a new shock against the
  held ones, and a turning shake (dirc 2) takes its heading from
  `rand()`.
- `cameraShockAbsorber` (0x001629b0) runs at the end of every
  `cameraMain`. The strongest shock sets the force (10, 20 or 30), the
  cycle and a `Ry(dirc)` matrix, which a turning one turns 0x1000 a
  frame. The oscillator gives the offset's z, each crossing of 0 keeps a
  fifth of the force, and the slots count down.
- `cameraSet`, for camera 1, moves the eye by `o = 0.5 (Rz(a)
  vibrateMatrix) vibrateOffset`, with `a` the heading (a double
  `atan2`). The target moves by `o` and then again by `o` times
  `min(dist / 280, 1)`.

The first comparison caught that last one: the port had moved the target
only by the scaled part.

Two statics matter at the start. `vibrateRotate` starts at 0, not -1, so
slot 0's heading turns every frame until a shake without `rot` comes.
`vibrateCycle` starts at 0, so the game's first absorber divides by
zero. The force is 0 then, so the oscillator is reset right after, and
the port skips the division.

The shakes now reach the camera three ways:

- the enemies' note, through `Stage::shake_range` (`ccCheckCameraDeg`
  67.5 degrees, and within 2000 of the eye), then `Call::CameraShake` at
  the note;
- the effects' events, which `FieldFx::take_shakes` hands over after the
  frame;
- `FieldWorld::camera_shake`, which draws `rand()` from the battle's
  generator.

`tools/test_camera_shake_rs.py` (new) runs the game's `cameraShake`,
`cameraShockAbsorber` and `cameraSet` in eemu. It does 12 runs of 400
frames with random shakes, and `rand()` hooked to the port's generator
from one seed. `SetMatrix_PosTarget` is hooked to take the eye and
target. The `shake_probe` example answers with the port's state. All 801
shakes, 4,406 shaking frames and 4,800 eyes and targets were equal.
`test_world_rs` (18) and `test_field_rt` (5) still pass, as do 605
workspace tests.

**Still unknown:** The double `atan2` is Rust's (the system libm), not
newlib's. It agreed in every case here, since the result is rounded to
float. The effects' shakes are applied after the frame, where the
game's effect task (priority 80) calls `cameraShake` inside it. Since
the absorber runs in the next `cameraMain` either way, only the order of
`rand()` draws within the frame can differ.
