# Outbreak's side events

Goal: every side event of Outbreak played where it happens, under the story
autopilot, and what stops the rest found and fixed where it is the port's.
Started 2026-09-29, after the story (plans/outbreak-story.md).

## The events

`tools/evscript.py list work/outbreak/disc/SLUS_205.63`: the S3 table is
events 250-275 (258 has no script), 25 events. The S4 table's 361
(`Vol3-SIGN06`) opens on 201 and is Outbreak's too; the rest of S4
(350-360) and M4 (301-317) open on Quarantine's events and never open here.
No M3 script follows 219 (the ending). The ML table (401-455) is the
members' friendship mails, on the desktop alone: not surveyed.

Each opens on a main event (`event_done`) and often on an earlier side
event; those below 200 are Mutation's, which only a carried save
(`ConvGame`) holds done. The survey starts each from the story start after
its last main opener.

| event | name | opens on | start | what it asks |
| --- | --- | --- | --- | --- |
| 250 | GOB3-1 | 204, MUT 154 | 205 | board 24; Kite alone; field 78: the golden goblins down |
| 251 | GOB3-2 | 204, 250 | 205 | the same, field 79 |
| 252 | GOB3-3 | 210, 251 | 211 | the same, field 80 |
| 253 | GOB3-4 | 210, 252 | 211 | the same, field 81 |
| 254 | GOB3-5 | 218, 253 | 219 | the same, field 82 |
| 255 | NUKE-2 | 204, MUT 155 | 205 | mail 176; Nuke (5); field 83's dungeon, point 1: a fight |
| 256 | MARLOWE-2 | 204, MUT 156 | 205 | mail 177, board 1; Marlowe (3); field 84, its dungeon's point 1: a fight, then two NPCs |
| 257 | RACHEL-2 | 210, MUT 157 | 211 | mail 178; Rachel (12); field 85's dungeon: key items 71-73 traded between points 1-4 |
| 259 | GALDE-4 | 204, MUT 159 | 205 | mail 179; Gardenia (13); field 87's dungeon, points 1-3 |
| 260 | SANJYURO-4 | 204, MUT 160 | 205 | mail 180; Sanjuro (4); field 88's dungeon: NPC 158 at points 1-6, point 7 |
| 261 | MOON-2 | 210, MUT 162 | 211 | mail 181; Moonstone (7); field 89's dungeon, points 1-2 |
| 262 | TERASHIMA-1 | 204 | 205 | board 26; field 90's dungeon: NPCs near markers at points 1-3; then Fort Ouph |
| 263 | SERVER-3 | 218 | 219 | mail 182; field 91's dungeon, point 1: a fight (enemy 176) |
| 264 | SIGN-6 | 201 | 202 | board 59; field 121's dungeon, point 1: a stream |
| 265 | SIGN-7 | always | 202 | field 99's dungeon: NPC 167 at point 1; point 2: a fight and streams |
| 266 | RYOKO-2 | 262 | 205 | mail 322; Ryoko (14); field 123's dungeon, point 1: a stream |
| 267-275 | SEARCH MIMIRU ... TUKASA | 201, each the one before | 204 | an NPC at a town's gate, met in six towns in turn |
| 361 | Vol3-SIGN06 | 201 | 202 | bars area 121 while Sora is in the party; ends once 264 is done |

The SEARCH chain opens on 201, but its NPCs go on to Fort Ouph, which 203
opens (`town_move 3`), so it plays from 204. SEARCH MIMIRU, BEA, KURIMU and
A-20 each want a key item of Mutation's SIGN-02 to SIGN-05 (165-168): 287,
288, 290, 289.

## How it is run

`outbreak_side_event_survey` (`--ignored --nocapture`, crates/piney-game/
src/session/tests/side_events.rs; `PINEY_SURVEY_ONLY=250,251`,
`PINEY_SURVEY_FRAMES`, `PINEY_SURVEY_GOD`, `PINEY_SURVEY_CALLS`,
`PINEY_DEBUG_PILOT`), as Infection's `side_event_survey`:

- the story start after the last main opener (`crate::start::build`);
- the side events it needs brought forward (`Start::bring_forward`,
  `ccEventFlagSet`); Mutation's only marked done; Mutation's key items
  given;
- its blocks before the first set in a field or dungeon (else a town)
  marked run, but for leading blocks with no settings, which play
  wherever the event is open (361's end);
- the session put there, the members it refuses to go on without
  (`not_in_party`) in the party (`start::party_of`); in a dungeon, the
  first located block's event point reached as `room_point` goes
  (`RoomSelect`, the event task idled at once as `ChangeRequest` does);
- driven by `StoryPilot` following that event alone (`Follow::side`:
  `story_wants` reads it in place of the main story).

## Where it stands (2026-09-30)

Survey (`PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000`, worklog 306): 22
of the 26 finish; no panic, no host call left at its default, no fault.

| event | state | what stops it |
| --- | --- | --- |
| 250-253 | done (3,687; 28,359; 35,487; 68,727) | |
| 254 | open | GOB3-5's golden goblins in field 82 are not run down in 150,000 frames: they run and heal back (the pilot's chase, a lone Kite at 75) |
| 255 | done (3,588) | |
| 256 | open | the dungeon walk stalls from ~14,000 in room 0-2 of field 84's dungeon: a foe (3,771 of 4,450 HP) takes no more damage while Kite runs about; point 1 not reached |
| 257 | done (33,644) | the key-item trade over points 1-4 |
| 259-261 | done (1,908; 22,294; 26,372) | |
| 262 | open | the same stall from ~30,000 in room 2-6 of field 90's dungeon (two foes at 678 and 1,107 HP); point 2 not reached |
| 263 | open | Black Death (OUT `enemyTbl` row 176: 9,999 HP, level 70, PP 8,888, physical PP defence 9,990, `exdefense` 2) at point 1 takes no damage from Kite at 75 |
| 264, 266 | done (386; 62) | |
| 265 | done (131,678) | the lone Kite at 75; at the story's 50, a game over near 106,000 |
| 267-274 | done (3,495-3,567) | each NPC met at the six towns' gates in turn |
| 275 | done (1,335) | ends at the second meeting (see below) |
| 361 | done (12) | block 0 closes it once 264 is done |

SEARCH TUKASA (275) ends in Dun Loireag at the second meeting: blocks 3
and 11 are both set in town 1 and both hold `talked_to 159` in the same
pass (`operateSet` is cleared only at the pass's top, `eventSub` runs
every block that holds), and block 11 ends the event. That is the port
following the game's walk; it was not seen on the game itself.

## The pilot's fixes

- `Want::Clear(field)`: a block in a field waits on `no_active` after the
  event put a foe there (`entry 5`). With no live Data Bug and no foe
  within 300, the pilot walks after the live foe with the least HP
  (`approach_foe`, sharing `approach_bug`'s walk). Unrestricted, it also
  chased 206's drained Data Bug (`entry 6`) and moved the whole run.
- `Want::Marker(n)`: a block waits on `near_marker n` (at distance 0 it
  holds). In a field, or in the dungeon room of that event position, the
  pilot walks to it.
- A block's `has_item` conditions are tested in the wants (the VM's own
  `has_item`): RACHEL-2 had waited at point 1 for a block wanting item 73.
- A town NPC beside the Chaos Gate with the gate the command target: the
  stick let go and pushed again steps the target on (`ccSelectTarget`
  mode 2). The SEARCH NPCs stand at marker 0, the gate's dummy.
- Survey aids (with god): a lone Kite in a field or dungeon at
  `LONE_LEVEL` (75), as the story's aid for 206.

Both whole runs are unchanged: `outbreak_whole_story` 463,200,
`mutation_whole_story` 613,800.

## Next

- 256 and 262: why the walk's foes stop taking damage (reached? hit?).
- 254: a chase that keeps up with GOB3-5's goblins.
- 263: Black Death at a level that hurts it, or what `exdefense` 2 asks.
- Mutation's side events, the same way.
