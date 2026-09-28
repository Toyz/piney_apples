---
number: 75
title: The tutorials run over a moving world: the event's menus open with mode 1, and R2 resets the camera in the lesson
date: 2026-09-23
area: script, ui, test
files: crates/piney-fieldui/src/lib.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, docs/engine/field-walk.md
---

# 75. The tutorials run over a moving world: the event's menus open with mode 1, and R2 resets the camera in the lesson

The user played events 2 and 3 and reported two things:
- the world froze under the tutorials, with the Chaos Gate stopping while
  its lesson ran;
- in event 3's camera lesson, R2 was accepted but the camera did not
  reset.

## The event's menus

- **What the game does.** The event's `menu num=` (`ccEvent::Execute`
  0x001b1e68) writes `openReqNum = num`, `mode = 1` and `firstTime = 1`,
  then waits for the menu to close. With `mode` set, `OpenMenu` neither
  sleeps the other tasks nor freezes their layers. So under the Party and
  Chaos Gate tutorials (75-79), the town, the gate and the characters
  keep moving.
- **The item menu differs.** `item_add_menu` (0x001afde0) writes `mode`
  0, so it does freeze the world.
- **What the port did.** The field host set only `openReqNum`, so the
  tutorials opened with `mode` 0 and froze everything.
- **The fix.** A new `FieldUi::event_menu` does what the instruction
  does, and both event hosts call it. `FieldUi::open_menu` still sets only
  `openReqNum`: the field UI's own tests and probe use it as a plain
  "open this menu". Changing it broke 24 of their checks, so it stays.
- **What still freezes, as in the game.** The pages inside a menu, such as
  the keyword screen, still sleep the world, because the menus ask for
  that themselves (checked against the game in [[68]]).

## The camera lesson's reset

This fix is the field agent's:
- **What the game does.** `teach_camera3` waits for the reset button (R2
  in scheme A, L1 in B). On the press, its handler (`ccEvent::Execute`
  0x001abfb0) sets `cpCtrl` 7 and starts the event camera's task. That
  turns the camera back behind Kite, a quarter of the way each frame. The
  task was already ported in evcam.rs and checked by `test_evcam_rs`.
- **What the port did.** The VM only asks the host whether the button
  came (`teach_input`), so the lesson moved on while the camera stayed.
- **The fix.** The area host records the press. `AreaMode` then calls
  `world.teach_camera(3)` after the event's frame and before the camera
  task, as the game orders its tasks (event 32, camera 33).

## Checked

- **The camera lessons** (runtime test, the port's own behaviour).
  `the_camera_lesson_moves_the_camera` plays a new game into field 14 and
  checks that each lesson moves the camera:
  - L1 turns it more than 0.5 rad;
  - the right stick changes its distance by more than 50;
  - R2 turns it back more than 0.3 rad.

  Without the fix it fails with the camera unchanged.
- **The event menus.** The event 2 and event 3 runtime tests pass with
  them.
- **Everything else still passes.** The field UI and PERSONAL suites, the
  event camera and field suites, the workspace's tests, clippy, fmt and
  the docs check.

**Still unknown:**
- **The town's event host** (field_host.rs) has the same gap for
  `teach_camera3`. Mac Anu has no camera lesson, so nothing reaches it
  yet.
- **Not shown.** No shot shows the world moving under a tutorial menu,
  because the menus dim the picture. The change follows the instruction's
  code and `OpenMenu`'s `mode` test.
- **Idle animations under events.** Characters under an event's manual
  control stand in their act's animation without the idle fidget. That is
  the game's rule (`ccFellow::Action` skips the fidget under manual
  control), but it has not been compared with the game frame by frame.
