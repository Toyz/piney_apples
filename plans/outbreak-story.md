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

## Where it stands (2026-09-29)

Survey (`PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000`), each event from
its own start: all 19 done (worklog 305).

| event | state | last place |
| --- | --- | --- |
| 201-205, 208-210, 212-217 | done | |
| 219 | done: the ending's stream 65, the staff roll | desktop |
| 206 | done (82,000 frames): field 13's Data Bug drained, then the lone Kite (raised to 75) through the dungeon of field 72, its goal room's Data Bug and Comad Goo; the dungeon from 9,750 to 82,000 | |
| 207 | done (120,000 frames): Fidchell ported (worklog 294), drained at 97,100, dead at 112,100 | |
| 211 | done (150,000 frames): Kyvia's second fight (`ccBossKyvia02`, row 13; `bossTbl`'s Cubia is Kyvia's English name, rows 12 and 13 alike) ported and checked (worklog 304); the first stage's core down near 16,500, the second's (5000 HP) near 22,000 | |
| 218 | done (150,000 frames): Gorre ported and checked (worklog 303); the brothers' protect broken at ~98,000, drained, the three down at ~107,000 | |

The whole run (`outbreak_whole_story`, the same aids): 201 to 219's staff
roll in 463,200 frames, from Outbreak's own new game (the title's New
Game, `ccSetupNewGame` at the first Log in: Kite at 50 with its gear, the
members' levels and gear from `InitSpcParam`; Mutation's carry-over,
`ConvGame`, is not ported). 201's `call_lock` keeps Kite alone to 204.
Events ended at: 201 9,900; 202 14,100; 203 77,700; 204 81,600; 205
84,300; 206 156,600; 207 232,200; 208 233,700; 209 275,700; 210 298,800;
211 320,400; 212 323,400; 213 338,400; 214 365,100; 215 387,900; 216
390,000; 217 391,500; 218 461,100; 219 463,200.

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
  any foe's broken protect is drained then. Raising the level alone (to
  85) did not help while the pilot wasted its skills; with them fixed a
  Kite at 75 reaches the goal room in two thirds of the frames at 50
  (64,000 against 97,000), and lives through it.
- Gorre (event 218): Gorre itself is never listed, so Data Drain's
  candidates take in its two brothers too; the fight's focus is the
  brother whose shared protect gauge a physical hit fills (row 41, a
  physical PP defence of 2000 against row 40's 9990), with Skills!.
- A field's Data Bug (206's in field 13's story map, its HP held at half
  since worklog 299): walked to, fight or not, on `path_to` over the map's
  hits when found stopped, and drained once its protect breaks.
- Kite's skill: the strongest (`atk` by `dmgRate`, over the foe's
  resistance to its element) whose `triggerRange` holds the nearest foe,
  of the page the foe is weaker to, else the other; the target menu's "no
  target" gives the action up.
- The dungeon walk: at the nearest foe; round a wall by a room waypoint
  when a chase is found stopped; the goal room's foes at any distance
  (203's and 206's Data Bugs and drained forms stood off past 600).
- A lone Kite where the story wants him alone (`Want::Alone`, 204 and
  206) raised to 75 (`LONE_LEVEL`, a harness aid as the boss levels are):
  at 50 the drained Data Bug of 206's goal room fells his 963 HP in one
  blow, which god's heal does not catch.
- The whole run: the top page quits to the desktop when the story wants
  only it (208's news); a member the story wants, the party full (210's
  Piros), the party disbanded first.
- 206's rooms are the game's: shut (`SetDoor`) until no foe is active,
  each with two Napylons whose Ola Repth (400 to each foe within 600 of
  one under half, 26 casts) the enemy harnesses check exactly on Outbreak.
