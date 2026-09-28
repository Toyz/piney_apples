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

First survey (`PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=30000`), each event from
its own start:

| event | state | last place |
| --- | --- | --- |
| 201, 202, 205, 208, 210, 212, 213, 216, 217 | done | |
| 219 | done: the ending's stream 65, the staff roll | desktop |
| 203 | blocks 0x1ff | area 2 field 71 (its boss room) |
| 204 | blocks 0xf | Mac Anu, target Gimmick 16 |
| 206 | blocks 0x7f | Fort Ouph, target Gimmick 16, block 9 |
| 207 | blocks 0x18f | area 2 field 73, menu 2 |
| 209 | blocks 0xc73 | area 1 field 74 |
| 211 | blocks 0x1c1f | area 1 field 10 |
| 214 | blocks 0xf | area 2 field 76 |
| 215 | blocks 0x3 | area 2 field 77 |
| 218 | blocks 0x8f | area 2 field 71 (its boss room) |

Seen on the way: `entry 3 29` (a walking PC, file `ctr1`) is not found
("not a walking PC's file"); the dungeon walker is put back in rooms it
cannot leave.
