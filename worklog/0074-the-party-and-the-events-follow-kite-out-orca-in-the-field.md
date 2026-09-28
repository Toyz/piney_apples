---
number: 74
title: The party and the events follow Kite out: Orca in the field, and event 3's lessons
date: 2026-09-23
area: script, world, test
files: crates/piney-game/src/area_host.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/main.rs, crates/piney-world/src/field_world.rs, crates/piney-world/examples/field_probe.rs, tools/test_field_rt.py, docs/engine/field-walk.md, docs/engine/event-vm.md
---

# 74. The party and the events follow Kite out: Orca in the field, and event 3's lessons

[[72]] brought Kite into field 14 alone. Orca, whom event 2 adds, stayed
behind, and event 3 (TEACH-F), which should begin there, never opened.
The field agent closed both gaps.

## Why event 3 did not open

Event 2's `scene` sleeps the event task with `ccSleepNoSleepThread`, as
[[69]] found. But the event task keeps its no-sleep flag
(`ccDeleteAllThread` skips it too), so it wakes on the next frame, inside
the field's set-up. There it runs `end_event`, which closes event 2. The
set-up's `ccStartThEvent` then turns closed into done, so event 3's
`event_done 2` holds at its first pass.

The port had left the VM parked inside `scene` for good, and
event-vm.md said it stayed asleep "until the next mode's set-up wakes
it". The doc now says what happens instead: the task wakes the next frame.

## What was built

- **The party goes along.** The session carries Mac Anu's party record
  (`ccSpcManager` and `ccPartyManager`) through every change of area: to
  the field, into the dungeon and back. `FieldWorld::reboot` runs where
  the game's `rebootSpcManager` does. It builds Kite and each member at
  the slot's start position, ported from `SetCharPosition`:
  - in a field, (+200, +100), (-200, +100) and (0, +300) from Kite;
  - in a dungeon, (300, -150), (-300, -150) and (0, -300), turned by his
    facing.

  The HUD panels and faces come from that record.
- **An event host for fields and dungeons** (area_host.rs). It:
  - resumes the parked pass in the area's first frame;
  - runs `start_thread` and the passes at phases 0 and 2 over the held
    black;
  - reboots the party, runs phase 4 at F0, then one pass per frame.

  It covers:
  - the message windows and menus;
  - `menu_ban` and `menu_clear`, `map_on` and `target_forbid`;
  - the fader and the event camera;
  - the camera lessons (`teach_camera1`-`3`, counting the pad as
    `ccEvent::Execute` 0x001ab9b4 does);
  - `pc_act`, `pc_mode`, `pc_turn` and `pc_face`;
  - `scene` and `area` from inside an area.

## How far it plays

A new game now runs event 2 in Mac Anu and arrives in field 14 with Orca
beside Kite. Event 3 opens with Orca's camera lesson: lines 2-6, all three
prompts, and lines 8-12. It goes on through:
- the field's explanation (13-19);
- the map (20-25);
- Fairy's Orb (26-29);
- the fight's introduction (30-33).

It stops at the skill tutorial (menu 80), which needs a battle.

## Old saves' buttons

The user still could not get past "press the triangle button" in event
2. The cause was a save from an earlier build of the port: its button
assignments from +0x8404 hold only ok and cancel, so no press ever
matches the personal menu's assignment of 0. The game never writes such
a save. The world mode now gives one `ccSaveData::Init`'s assignment
when it enters The World (`repair_buttons`), leaving any save with
buttons of its own alone.

## Checked

- **Against the game.** `tools/test_field_rt.py`'s `test_char_position`
  now also checks the three members' places, for every field type and 40
  dungeon positions and facings, with 0 mismatches.
- **The whole path, as a runtime test.** `event_3_opens_in_the_field`
  plays a new game through event 2 into the field. It asserts:
  - the scene (1, 14);
  - the party [0, 2, -1];
  - Orca at Kite + (200, 100);
  - event 2 done;
  - Orca's lines 2-6 opened.

  This checks the port's own behaviour, not against the game.
- **Re-run in a clean worktree.** The field, area, field host, world,
  event characters and camera, dungeon and field UI suites pass, with the
  workspace's tests, clippy, fmt and the docs check.
- **Shots.** Kite and Orca on the grass with the camera lesson's line;
  the field, the dungeon, and Mac Anu again after Gate Out.

**Still unknown:**
- **Needs the battle.** Menus 80 and 83, `player_skill`, `hold`, the
  magic portals (`entry_mc`) and the members' field AI (following,
  fighting).
- **Not done.**
  - `pc_command`, the walks and the puts, which are only partly traced.
  - The event positions in a field: the event camera finds no markers
    there.
- **Kite stays held.** Event 3 sets Kite's `bootParam` 4, which puts him
  under manual control. Nothing ported releases it yet, so after the
  tutorial he stays held.
- **Back in Mac Anu.** After Gate Out the party is not carried back into
  Mac Anu's own party record.
- **Registered characters outside the party** are not built. The game
  places them at the origin.
- **When the event task wakes** was read from the thread code, not run
  against the game.
