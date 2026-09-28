---
number: 232
title: The pad's vibration
date: 2026-09-27
area: ui
files: crates/piney-input/src/actuator.rs, crates/piney-game/src/input.rs, crates/piney-game/src/main.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/desktop.rs, crates/piney-world/src/field_world.rs, docs/engine/field-ui.md
---

# 232. The pad's vibration

Reported from play: vibration never worked. The desktop's and the
field's Vibrate pages set the option, but nothing ever reached a pad.
`Request::Vibration` and piney-battle's `Out::Actuate` were both dropped.

## The game's motors

The game rumbles the pad in three places:
- `ccPlayer::DamageActuate` on a hit on Kite: the small motor and
  `DamActuTbl` (64, 128, 255 by damage under 10, under 100, above), for
  100 ms;
- the desktop's `VibrationMenu`;
- the field's `VibrationMenu`.

Both menus, switched on, write the save's `vibration` and
`ccPad::actuaterSw`, then call `SetActuater(pad 0, 1, 160, 200)`.

The rest is `ccPad`'s queue (`piney_input::actuator`):
- **`SetActuater(small, power, ms)`** (main 0x00102bf0). It works only
  with `actuaterSw` on. It converts the time to vblanks, (3 ms + 25) / 50.
  Each queued entry, from the top down, loses that time, or is dropped
  when it has none left. With room for another (at most three), the top
  is marked to send again, and the new entry goes on top, marked to send.
- **`ccSystem::Ctrl`** (0x0010a740), each frame. It drops the spent
  entries off the top, then takes the frame rate off the top entry's
  time.
- **`ccPad::Ctrl`** (0x00102a50), state 1. If the queue is empty and the
  idle entry is marked, it turns both motors off. Otherwise, if the top is
  marked and has 6 vblanks or more left, it sends the top's motors. It
  then waits a frame on the send before it looks again.

## The port

- **The model.** `piney_input::actuator::Actuator` models that queue.
  `set`, `tick` and `ctrl` are `SetActuater`, the countdown and the send.
  Its tests check a hit's rumble and its stop, the option off, and a
  shorter rumble over a longer one sending the longer one again.
- **`actuaterSw`.** `Mode::vibration` reports the save's Vibration option
  each frame, and the app keeps `actuaterSw` equal to it. The game keeps
  the two equal itself: the load, the new game and both menus write both.
- **`Event::Actuate`.** The modes send `SetActuater` calls as this event:
  - the desktop's and the title's system menus (`desktop::event`) and the
    field's Vibrate page, when switched on;
  - Kite's own rules' `Out::Actuate`;
  - an enemy's or a member's `EntryAffect` on Kite. These arrive as the
    `DamageActuate` rule event in their shows, and the area turns them
    into the rumble through `kite::damage_actuate`.
- **The app.** Before each frame it ticks the queue. When `ctrl` sends,
  it plays the motors on the first connected gamepad that takes force
  feedback (`input::Rumble`, gilrs):
  - the large motor as a strong rumble at `power / 255` gain;
  - the small motor as a weak rumble, on or off.
- **A world entry point.** `FieldWorld::entry_affect(on, by, kind, p)` is
  now public, with the parameters the menus' `affect_from_player` lacks.

The new test `a_hit_on_kite_rumbles` lands `EntryAffect(1, 20)` on Kite
in event 3's field. It checks that the mode asks for one rumble: the
small motor, power 128, 100 ms.

**Still unknown:** The rumble has not been felt on a real pad from here.
gilrs needs the pad's force feedback device (on Linux, write access to
its event node). The pad's connection states (`ccPad::Ctrl`'s other
states, `scePadInfoAct`) are not modelled: the port treats a connected
pad as a ready DualShock.
