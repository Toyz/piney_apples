---
number: 293
title: Outbreak's story under the autopilot: its starts, going alone, a story map's door, the Data Bugs
date: 2026-09-28
area: script, test, volumes
files: crates/piney-game/src/start.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-world/src/field_world.rs
---

# 293. Outbreak's story under the autopilot: its starts, going alone, a story map's door, the Data Bugs

After Mutation's whole story ([[276]]), Outbreak's. Its main story is
M301-M319, events 201-219 (`tools/evscript.py list`); 219 is the ending.
Each opens on the one before (`event_done`).

## The starts

`start::OUT_STORY` lists them. The start places come from each event's
first located block, read as for Mutation (checked against Mutation's
known places 104, 105 and 115):
- the desktop: 201 (the new game's), 202-209, 212, 216, 218, 219;
- the board (`game_status 3`): 214, 215, 217;
- a town from Log in (`game_status 5`, `in_town 3`): 210, 211, 213.

`outbreak_story_starts_open_their_event` holds all 19. The start test now
counts the starts it checks and fails at none: Infection 24, Mutation 16,
Outbreak 19. `outbreak_story_survey` drives each start; `whole_story`
serves `mutation_whole_story` and `outbreak_whole_story` alike.

## What the pilot learnt

The first survey (30,000 frames each, god mode) ended 10 of the 19. Four
pilot changes and longer runs took it to 15 (plans/outbreak-story.md):

- **Going alone.** In the VM `in_party -1` is someone else in the party
  (`num >= 2`) and `not_in_party -1` is Kite alone. Event 204 bars area 15
  at the gate with company (`add_area_code 15`) and unbars it alone
  (`del_area_code 15`); with company, the lines tell Kite to go alone.
  The pilot read Balmung's line block as a want and called him in. A
  block that unbars a wanted area only with `not_in_party -1` now makes
  `Want::Alone`, and the pilot disbands (PERSONAL, Party, Disband: menu
  70) and calls no one. 206's area 72 is the same.
- **A story map's door.** 204 goes on in area 15's church, block 1 of
  `EVENTAREA02` (`scene ... field=15 block=1`). `Want::FieldBlock` comes
  from such a scene, and the pilot walks to the map's door: the middle of
  the floor polygons whose attribute has the Enter bit (0x8_0000),
  `FieldWorld::door`.
- **A Data Bug.** 203's goal room holds enemy row 164, a Data Bug: `type`
  0x40 (the common foes are 0x20), 20,624 HP, `maxPP` 7,250 (Infection's
  115, 201, 224 and 235 have 1,250-2,750). The walker gave it up as
  hopeless once its HP stopped falling; now no foe of the goal room is,
  and a Data Bug's broken protect is drained as a boss's is.
- **A far dungeon.** In 209's field 74 the pilot ran straight at the
  dungeon's entrance into a wall. Far off and found stopped, it plans the
  way on a grid wide enough to hold Kite (81 cells, the step the distance
  asks).
- **A fight that holds the walk.** 206's lone Kite meets Napylons that
  heal each other faster than he hurts them. Raising the party through
  the console (to level 85) did not change it, so that aid was dropped.
  After three times the walk's give-up time the pilot drains any foe whose
  protect breaks. The room then clears, slowly: floor 1 by 150,000 frames.

## Where it stops

207, 211 and 218 end in bosses the port does not have: field 4's
`bossTbl` row 3 (Fidchell), field 10's row 13 (Cubia, the second Kyvia
fight) and field 5's row 4 (Gorre). 206 is the slow walk above.

**Still unknown:**
- `entry 3 29` (Lios as a walking PC, file `ctr1`) is refused ("not a
  walking PC's file") in the towns of 209 and 216-218; whether the events
  need him there was not checked.
- The whole run from Outbreak's new game (`outbreak_whole_story`) was not
  made; the events were only run from their own starts.
- Whether a player's party, carried over from Mutation (`ConvGame`, not
  ported), would make 206's room easier than the starts' Kite at level 50
  with the new game's gear.
