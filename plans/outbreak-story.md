# Outbreak's story end to end

Goal: a new game of Outbreak plays to its ending, every story event (M301-M319,
201-219) finishing under the story autopilot, with the story maps and bosses it
passes through ported as the game builds them. Started 2026-09-28, after
Mutation's (plans/mutation-story.md).

## The story

`tools/evscript.py list work/outbreak/disc/SLUS_205.63`: M301-M319 are events
201-219 (219 the ending), each opening on the one before (`event_done`). The
start points (`--mode story:N`, `crate::start`) are where each event's first
located block is:

| events | start |
| --- | --- |
| 201 | the new game's desktop, as the boot leaves it |
| 202-209, 212, 216, 218, 219 | the desktop (`game_status 2`) |
| 214, 215, 217 | the board (`game_status 3`) |
| 210, 211, 213 | a town from Log in (`game_status 5`, `in_town 3`) |

Events 203 and 218 end in a dungeon's boss room (`boss_smoke`, floor 3).

## How it is checked

- `outbreak_story_starts_open_their_event`: each start opens its event.
- `outbreak_story_survey` (`--ignored --nocapture`): each start driven by
  `StoryPilot`, with `PINEY_SURVEY_GOD` for the aids.
- `outbreak_whole_story`: one run from the new game to 219's end.

## Where it stands (2026-09-28)

Survey (`PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=60000`), each event from its
own start:

| event | state | last place |
| --- | --- | --- |
| 201-205, 208-210, 212-217 | done | |
| 219 | done: the ending's stream 65, the staff roll | desktop |
| 206 | blocks 0x7f: a lone Kite through the dungeon of field 72, slow (floor 1 by 150,000 frames) | dungeon of field 72 |
| 207 | done (120,000 frames): Fidchell ported (worklog 294), drained at 97,100, dead at 112,100 | |
| 211 | blocks 0x1c1f: field 10's boss, row 13 (Cubia, the second Kyvia), not ported | field 10 |
| 218 | done (150,000 frames): Gorre ported and checked (worklog 303); the brothers' protect broken at ~98,000, drained, the three down at ~107,000 | |

Seen on the way: `entry 3 29` (a walking PC, file `ctr1`) is not found
("not a walking PC's file"); the dungeon walker is put back in rooms it
cannot leave.

## The pilot's fixes so far

- Kite alone: a block that takes a wanted area off the gate's barred list
  (`del_area_code`) only with `not_in_party -1` makes the story want him
  alone (`Want::Alone`); the pilot disbands (PERSONAL, Party, Disband) and
  calls no one. Events 204 (area 15) and 206 (area 72). `in_party -1` in
  the VM is "someone else in the party" (`num >= 2`).
- A story map's other block (`scene` with a block in a field: area 15's
  church): the pilot walks to the map's door, the floor polygons with the
  Enter bit (`FieldWorld::door`).
- A Data Bug (enemy `type` 0x40: event 203's row 164, 20,624 HP, a
  protect of 7,250) is fought on in the goal room, never called
  hopeless, and drained once its protect breaks, as a boss is.
- A field's dungeon far off with a wall between: the entrance path is
  planned on a grid wide enough to hold Kite (81 cells, the step the
  distance asks) when he is found stopped (event 209's field 74).
- A room whose foe has held the walk three times its give-up time (event
  206's Napylons, healing each other faster than a lone Kite hurts them):
  any foe's broken protect is drained then. Raising the party's level
  instead (to 85) did not help: the fight turns on the healing, not the
  level.
- Gorre (event 218): Gorre itself is never listed, so Data Drain's
  candidates take in its two brothers too; the fight's focus is the
  brother whose shared protect gauge a physical hit fills (row 41, a
  physical PP defence of 2000 against row 40's 9990), with Skills!.
