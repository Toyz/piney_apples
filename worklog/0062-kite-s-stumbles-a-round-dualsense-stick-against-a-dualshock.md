---
number: 62
title: Kite's stumbles: a round DualSense stick against a DualShock 2's threshold
date: 2026-09-23
area: ui, test
files: crates/piney-game/src/input.rs, crates/piney-game/src/main.rs
---

# 62. Kite's stumbles: a round DualSense stick against a DualShock 2's threshold

The user played Mac Anu with a DualSense and reported:
- Kite's running breaks while the camera turns;
- he stalls and then keeps moving;
- it happens when the stick is pushed fully north-west.

## Ruling out the game logic

A probe fed stick bytes through `piney-input`'s `ccPad::Read` into the
world:
- **Clean input.** A steady full north-west (0, 0), with or without the
  camera turning at full speed, runs steadily for every frame.
- **Noisy input.** The same with a few units of noise, but at (30, 30)
  instead of the corner: Kite switched between the run act (5) and the walk
  act (6) nearly every frame. `pow_l` wobbled between 211 and 238, across
  the run threshold, and the speed stayed 27.5 throughout.

So the logic, which the eemu checks already match to the game, does what the
game does. The input it was given was the problem.

## The cause

- **The game's scale.** `SetAnalogStick` measures the stick's strength as
  255 over 80 steps past its dead zone, and the run/walk choice sits high on
  that scale.
- **How a DualShock 2 reports.** Its pots saturate before its round gate
  stops the stick, so a full push, diagonals included, reads at or near
  both axes' ends.
- **How a DualSense reports.** A modern round stick pushed fully diagonal
  reaches only about 0.7 per axis. That is (38, 38) as bytes, a strength on
  or below the threshold, so small movements, like the thumb shifting as
  the other thumb turns the camera, tip it between walking and running.
- **gilrs made it worse.** Its default filters apply a radial dead zone
  that rescales both axes, drop changes under 0.01, and can insert a zero
  for one axis on their own. The game applies its own dead zone after
  them.

## The fix

- **The stick's radius.** `input::square_stick` carries the stick's
  radius to the square: the vector is scaled so that its larger axis equals
  its length, at most 1. A full push in any direction then reaches the
  corner, as on a DualShock 2. Straight pushes, half pushes along an axis
  and the centre are unchanged.
- **No gilrs filters.** gilrs is built with its default filters off. The
  stick values are raw, and only the game's dead zone applies.
- **A diagnostic.** `--pad-log FILE` writes every gamepad gilrs sees, then
  one line per game frame: the chosen pad's axes as gilrs gives them, the
  bytes passed on, the decoded direction and strength, and the mode's
  status. In The World the status now includes Kite's act and speed.

## Checked

- **Unit test.** `a_full_diagonal_reaches_the_corner`: a round stick at
  full north-west becomes bytes (0, 0) and strength 255, while straight
  pushes, half pushes and the centre keep their values.
- **The rest.** `piney-game` has 11 tests, all passing; clippy and fmt are
  clean.
- **Not yet on the controller.** It is not confirmed on the user's
  DualSense; the pad log is there for that.

**Still unknown:**
- **The DualShock 2's real values.** The exact values a DualShock 2 gives
  at full diagonals are recalled, not measured. The square mapping assumes
  full saturation.
- **Partial diagonals.** Half-way diagonals now read slightly stronger
  than a round stick's length would suggest. The game's analogue walk speed
  may feel different there.
