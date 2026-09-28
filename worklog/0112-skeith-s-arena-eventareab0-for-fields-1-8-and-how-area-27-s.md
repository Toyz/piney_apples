---
number: 112
title: Skeith's arena: EVENTAREAB0 for fields 1-8, and how area 27's last door gets there
date: 2026-09-25
area: world, render, test
files: crates/piney-world/src/evarea_b0.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/lib.rs, crates/piney-world/examples/evarea_probe.rs, crates/piney-game/src/main.rs, tools/test_evarea_rs.py, docs/engine/evarea.md
---

# 112. Skeith's arena: EVENTAREAB0 for fields 1-8, and how area 27's last door gets there

Part two of Skeith ([[109]]) begins with where the fight happens.

## Where event 30 goes

Event 30 (`ML0180`) is Infection's last main event. Its blocks 17-19 are
Chosen Hopeless Nothingness (area 27), the field and the dungeon. Block 20's
settings are `in_field town=1 field=1`, `in_point 1` and `phase == 0`:
- `entry type=7 code=0`;
- `battle_ready`;
- `save_party`.

Settings accumulate from block to block into `currentOpen`, and
`CheckCurrentOpen` compares the whole scene. Field 1 has no words and no
event leads there with `scene` or `area`.

**The door.** Area 27's dungeon row floor 4 room 3 is of type 16. Entering
it, `GotoNextRoom`'s branch for areas 108, 73, 47, 66, 46 and 27 finds an
edit row of type 16 or more with this floor and the room being entered.
Unless `CheckAreaBan` says the room is banned, it answers -255.
`WORLD_MAN::Enter` then changes area by `eventAreaNumber`:
- 27 goes to field 1;
- 46 goes to field 2;
- 73 goes to field 4;
- 66 goes to field 67;
- 108 goes to field 9;
- 47 goes to field 9 on volume 2, else 10.

Block 22's `area_ban field=27 dungeon=0 floor=4 block=3` closes the door
after the fight.

**The missing break.** Case 27 calls `ChangeArea(1, 1)` and falls into
case 46's `ChangeArea(1, 2)`. Mutation has the same code.
- In eemu, with `ChangeRequest` stubbed, both run and `game.field` ends
  as 2, which block 20 would never match.
- In the game, the first `ChangeArea`'s `ChangeScene` asks for
  `ChangeRequest(6, 7)`. That calls `ccSleepNoSleepThread(1, 1)`, which
  sleeps the calling task: the player's, inside `Enter`.
- The mode change deletes the task before it wakes, so the second call
  never runs, and the party lands on field 1.

The port must stop at the first call. The door itself, and the story
rooms of types 16-35 in general, are the other thread's work.

## EVENTAREAB0

`GO(1)` makes it for fields 1-8, whose `EVENTAREA_INFO` rows all have
model 1 (fields 9-12 too, for `EVENTAREAB8`). By `game.field` it picks a
stage: `se1_5` for 1, then `se2_3`, `se2_4`, `se3_2`, `se3_4`, `se4_3`,
`se4_5`, `se4_7`. Field 8 also loads `se4_8` for `NextStage`. The
constructor writes the field's digits into `EA_MODELTABLEB0`'s and
`eventareaB0Light`'s `se1_5` names. The rest is on
[the story maps page](../docs/engine/evarea.md#the-boss-arenas-eventareab0):
- nine static models;
- three background clumps;
- 20 lights from `ANM_*bac1a`;
- `DMY_center01` as the start;
- 54 fireflies.

`Draw` has three things of its own:
- a scrolling cloud material;
- `ob0` on refLayer, which `SwitchLayer` (Skeith's magic) moves to
  objLayer;
- `ob1`-`ob3` bobbing by `fieldrand(5)` between +-50.

`piney_world::evarea_b0::Arena` is the port. `FieldWorld` holds it as
`Place::Arena` for fields 1-8. The condition cannot be
`WORLD_MAN.field_model`, which is still area 27's (0), so it tests the
field number. `piney-game --mode field:1` starts there. Area 27's
`WORLD_MAN` is on town 1, since the field has no words.

The first shot shows the stage: floating rocks, the floor's magic circle,
Kite under its green lights.

## Checked

**`test_evarea_rs.py`'s `ArenaAgainstGame`** builds the game's
`EVENTAREAB0` for field 1 in eemu, with the fireflies as stand-ins and
`fieldrand` running natively from a seed. It checks:
- the models' rows, names and positions, the clumps, the lights in order
  and the distant light, the hit model, `SetFog`, `eventStartPos` and
  `bgColor` against `evarea_probe arena`;
- 600 `Draw`s with `SwitchLayer` now and then: the pieces and layers, the
  bobbing positions, the cloud's v offset and `fieldrand`'s seed each frame.

Everything matches. The other six evarea checks, and piney-world's tests,
clippy and fmt, pass.

**Still unknown:**
- **`FIREFLY2`** (the arena's 54 fireflies) is not ported, nor is field 8's
  `NextStage`.
- **`fieldrand`'s seed** on arrival is whatever the dungeon left, which the
  port does not carry across scenes. The arena starts its own, so the
  bobbing differs from the game's in play.
- **Sound bank 5.** `ccSetupGameCtrl` loads it for a story map by
  `EVENTAREA_INFO.model`. The session reads that from its `WORLD_MAN`,
  which for the arena is area 27's.
