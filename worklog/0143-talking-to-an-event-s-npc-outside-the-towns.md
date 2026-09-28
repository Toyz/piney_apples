---
number: 143
title: Talking to an event's NPC outside the towns
date: 2026-09-26
area: world
files: crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/field-game.md
---

# 143. Talking to an event's NPC outside the towns

A GAPS item: talking to the Administrator in event 17's dungeon (the
Data Drain lesson). Block 7 stands him at point 1 with `entry 4 29` and
makes him an event target with `add_target 4 29`. Block 9, `if
talked_to 4 29`, is his answer when Kite speaks to him: he turns to Kite
and repeats line 10 (sending Kite back to the field). The
block can be played again.

## What was wrong

In the port's dungeon, pressing the action button on him did nothing
the event could see.

- **In a town.** The world tests the target against `eventMng.target[]`,
  as `ccEvent::CheckOperate(9)` does. An event's target gets no menu. The
  session hands the event the command target (`operate_target`), and
  `talked_to` holds for that frame.
- **In a field or dungeon.** `FieldWorld::game_ctrl` sent every action
  to its menu, so the Administrator's type (0x10) opened `NpcMenu`. The
  area never told the event whom Kite spoke to.

## The fix

- **The test.** `FieldWorld::set_event_targets`, fed each frame from the
  interpreter's `mng.targets`, makes the same test as the town. A
  target's type is a bit of the character's base type flags, and its code
  is the base id (the field's target is a scene index).
- **The event.** A hit is `TalkRequest::Event`. `AreaMode::talk` puts the
  command target into `vm.mng.operate_target` (through
  `AreaHost::target_ref`, which `Host::command_target` now shares) and
  closes the menu state.

`event_17_talks_to_the_administrator` plays event 17 from block 7. It lets
block 8's lines run, walks Kite up to the Administrator's stand-in until
it is the command target, and presses the action button. Block 9 plays,
and plays again on a second talk, with no instruction falling to a host
default. The same path serves event 27's `add_target 3 85` in area
24's dungeon.

**Still unknown:** event 27's talk (a walking PC's row, 85) is not
played through by a test.
