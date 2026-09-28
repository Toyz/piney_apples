---
number: 263
title: Mutation's story under the autopilot, and area 43's map
date: 2026-09-28
area: world, test, volumes
files: crates/piney-world/src/story_map.rs, crates/piney-world/src/evarea03.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/area43.rs, crates/piney-game/src/start.rs, crates/piney-fieldui/src/lib.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/talk.rs
---

# 263. Mutation's story under the autopilot, and area 43's map

`mutation_story_survey` now finishes 13 of Mutation's 16 story events,
up from 6 in [[250]]. Event 101, the new game's, has a start (the desktop as
the boot leaves it) and plays from there through Mac Anu and field 27's
dungeon into event 104's field. Most of the gap was the survey and the
pilot, not the port. The rest is the first of the story maps Mutation
needs: area 43's `EVENTAREA03`, behind a new `StoryMap` trait.

## The survey read the flag in the wrong places

The survey took an event's flag only in a town, field or dungeon. An
event that ended on the desktop kept the flag it had there. And a
closed event (bit 63, its `end_event` played) turns done (bit 62) only at
the next mode's `ccStartThEvent` (`Vm::start_thread`). Event 106 had played
every block (flag `0x800000000000003b`) and was reported open. The flag
is now read on every stage, and bit 63 or 62 counts as the end.

## What the pilot learned

- **The arena door.** Event 107's boss is in field 2, `EVENTAREAB0`,
  reached from field 46's dungeon through the door into its special room
  (type 16 or more): `GotoNextRoom` answers -255 and `WORLD_MAN::Enter`
  calls `ChangeArea(1, special::exit_field(area))`. When the story wants
  that field, the pilot's dungeon goal is that room.
- **A lake's second dungeon.** Area 48 is a lake (field type 4): its
  dungeon 1 lies below dungeon 0, down the last floor's stairs (`Enter`'s
  -1). `Want::Dungeon` now carries the dungeon's index, and a goal one
  floor past the lake's last takes those stairs. Event 111 finishes.
- **A revive in progress.** `condition[0]` 5 is a member getting up (78
  frames, `ccFellow::Action`'s fade). `ChatMenu1` refuses him every order
  (rows 0, 2, 3 in a fight), and the open menu pauses the fight, so the
  count never ran out: the pilot ordered, was refused, and ordered again,
  for 35,000 frames in event 107. Members whose condition is not 0 are
  now not ordered or targeted, and menu 71's refusal (proccess 10, 11)
  takes OK.
- **The top page.** An `add_operate` bit (command + 6: 6 Log in, 7 the
  board, 8 Quit) makes the command an event's message instead. Event 108
  takes the board for its whole run. The pilot passes a taken command
  by, logs in instead, and does not log out for posts while the board is
  held. It also writes a post waiting to be written (state 7): OK types
  it out, OK posts it (`ExitWritingMsgPage`).
- **Repeatable blocks.** A block that plays again each time is a side
  line: event 115's point 3 clears block 10 on every entry, and 113's
  point 2 waits for an item. Their event points are not goals, though
  their other wants (a member, an area's words) still are.
- **Towns.** The pilot walked straight at its target. In Carmina
  Gadelica the equipment shop (event 103's `add_target` type 8, the
  `EQUIP_SHOP` flag 0x100, `npcTbl` row 11) is behind a wall. The field's
  breadth-first planner is now general (`path_to`), and towns use it.

## The story maps behind one trait

`Place` had a variant for each ported `EVENTAREA` (02, 07, B0), and each
of them went into some twenty matches. They are now one
`Place::Story(Box<dyn StoryMap>)`. The trait is `piney_world::story_map`:
hits, lights, start positions, the clear colour, `enter`, `frame` and
`draw`. `story_map::build` is `GO(1)`'s choice by field. `Place::story::<T>`
reaches a class's own state, such as the arena's `SwitchLayer`, as the
towns' `RootTown` does.

## Area 43: `EVENTAREA03`

Event 102 goes to field 43, which the port had given a generated field.
It is `EVENTAREA03` (MUT gcmn 0x00416590 constructor, 0x00416bf0 `Draw`;
INF 0x00403590, 0x00403bf0; Mutation's is 4 bytes shorter in each):

- **The scene.** `se2_1`, with `EA_MODELTABLE03`'s two pass-0 models
  (`MDL_se2_1fl1`, `MDL_se2_1ba1`) and `eventarea03Light`'s one distant
  light, `LGT_se2_1lig1`. Fog is (150, 4000, 0, 80, 0x1e1e1e), the start
  (0, 0, 0) with w 1.0.
- **`flag` (+0x1c8).** The constructor reads
  `gateListMark[game.server][1] & 0x800`: area 43 marked on the server
  (+0x1c is `ccGame.server`, from the DWARF).
- **The fly-over.** While `flag` is 0, `Draw` runs a scene of its own:
  1. Frame 0: `changeCamera(3)`, `MenuBan`.
  2. Camera 3 goes from view (1322, -1128, 1014) and position (1678,
     -1433, 1188) to (-210, 167, 36) and (162, -133, 178), with
     t = min(cnt / 120, 1), over frames 0-139.
  3. Frame 140: `ccMsg->Open(&kiteSelfTalk, kiteSelfTalk.name, -1, -1)`.
  4. Once `Check(0)` answers: `MenuClr`, `changeCamera(1)`,
     `ChangeArea(0, 1)`, back to Dun Loireag.
- **`DrawBG`** draws `bg[0]`, which the constructor never sets. `Draw`
  never calls it.

During the story the area is marked (event 102's `gate_mark`), so only
the event's own fade plays and the scene waits. The port runs the
scene in `StoryMap::frame`. The camera moves happen there. `MenuBan` and
the message go to the game as `Request::Story`, and `AreaMode` answers
`Check(0)` with the frame's pad before the field steps.
`kiteSelfTalk` is a record of the fieldui group now
(`kite_self_talk`, `kite_self_talk_va`), and a root of the talk records,
so `FieldUi::story_message` opens it as `talk::open_record` does.
`DATA_VERSION` is 9.

Checks: `evarea03::tests::area_43` (the models, the hits, the light, the
mark), and the session's `area_43_flies_over_and_sends_the_party_back`
(a new game's warp to area 43: camera 3, the ban, the line at `cnt` 141,
OK, town 1). Area 15's and 16's session tests pass on the trait.

## The survey now (60,000 frames a start; 150,000 for the long ones)

- **Done:** 101, 102, 103, 104, 105, 106, 109, 110, 111, 112, 113, 114,
  116.
- **107:** at the arena boss in field 2; the pilot never uses Data
  Drain.
- **108:** logs in to Dun Loireag, not Carmina Gadelica, and around again.
- **115:** in field 52's dungeon, slow; after it comes field 13,
  `EVENTAREA01`.

One more thing turned up. The game's own binary did not build: the
console's `invite_party` used `AreaMode::world_mut`, which was test-only.
Only the tests had been built since. It is public now.

**Still unknown:**
- Why event 108 logs in to Dun Loireag.
- How the pilot should find a Data Bug's protect break for Data Drain.
- `EVENTAREAB8` (field 9, event 108) and `EVENTAREA01` (field 13, event
  115), still to port.
- The towns read Infection's `statics::tables()` on every volume. Whether
  that differs from Mutation's own tables was not checked.
