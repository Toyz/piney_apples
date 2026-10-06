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
`PINEY_SURVEY_HITS` (each foe's and member's HP and last affect as they
change), `PINEY_DEBUG_PILOT`), as Infection's `side_event_survey`:

- the story start after the last main opener (`crate::start::build`);
- the side events it needs brought forward (`Start::bring_forward`,
  `ccEventFlagSet`); Mutation's only marked done; Mutation's key items
  given;
- its blocks before the first set in a field or dungeon (else a town)
  marked run, but for leading blocks with no settings, which play
  wherever the event is open (361's end), and for repeatable ones
  (`repeatable` keeps a block's bit clear: GOB3's town blocks, which bar
  the area while anyone goes along); their gate words, gate marks and
  town moves played, as a player passing them has them;
- the session put there, the members it refuses to go on without
  (`not_in_party`) in the party (`start::party_of`); in a dungeon, the
  first located block's event point reached as `room_point` goes
  (`RoomSelect`, the event task idled at once as `ChangeRequest` does);
- driven by `StoryPilot` following that event alone (`Follow::side`:
  `story_wants` reads it in place of the main story).

## Where it stands (2026-10-06)

Survey (`PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000`, the whole table
run again at worklog 385, after the spells' and the menus' draws moved to
the game's points, worklogs 382-384): 25 of the 26 finish; no panic, no host call left at its default, no fault. The golden
goblins of 250-253 fall with no HP aid (worklogs 373, 374).

| event | state | what stops it |
| --- | --- | --- |
| 250 | done (16,959) | |
| 251-253 | done (48,471; 24,735; 28,983) | a trip to Fort Ouph's magic shop for Speed Charms first (worklog 374); 251 a second one when they run out (worklog 385) |
| 254 | open | GOB3-5's golden goblins (row 156) evade blows, bar spells while their mDef holds, and shake off a spell's hold; their heals outpace what lands (worklog 374) |
| 255 | done (4,154) | |
| 256 | done (64,978) | room 0-2's Gaia Turtle (row 114, Exdefense 1) takes no blows (see below) |
| 257 | done (32,266) | the key-item trade over points 1-4 |
| 259 | done (1,908) | |
| 260, 261 | done (19,116; 41,050) | the Gott statue at point 7 and point 2 opened first: `no_active` waits on it (#18; worklog 375) |
| 262 | done (44,588) | room 2-6's Deadly Presents (row 243, Exdefense 1) take no blows |
| 263 | done (8,988) | Black Death (row 176, Exdefense 2) takes no spells |
| 264, 266 | done (376; 62) | |
| 265 | done (64,958) | the lone Kite at 90: at 75 the Data Bug (row 261, level 68) fells him with two blows of 703 in a frame (1,395 HP), at 47,391 once the pilot changed |
| 267-274 | done (3,375-3,447) | each NPC met at the six towns' gates in turn |
| 275 | done (1,335) | ends at the second meeting (see below) |
| 361 | done (12) | block 0 closes it once 264 is done |

SEARCH TUKASA (275) ends in Dun Loireag at the second meeting: blocks 3
and 11 are both set in town 1 and both hold `talked_to 159` in the same
pass (`operateSet` is cleared only at the pass's top, `eventSub` runs
every block that holds), and block 11 ends the event. That is the port
following the game's walk; it was not seen on the game itself.

## The foes that took no damage: Exdefense (worklog 307)

The stalls of 256, 262 and 263 are the game's rule, not a fault. A row's
`exdefense` (+0x64) bars a kind of hit while the defence behind it holds
(`CalcBattleDamage`, INF gcmn 0x0056d910; the later rule MUT
0x005933f8-0x005935bc, OUT 0x0058f5c0): bit 0x1 physical while `real` pDef
is not below the row's, 0x2 magic by mDef, 0x4-0x80 a skill of that
element. A barred hit does 0 and fills no protect gauge.
`tools/battle.py damage` on OUT's code: Kite at 75 does 0 to Gaia Turtle
(row 114: pDef 700, Exdefense 1) with ATTACK and 31-35 a blow to Black
Death (row 176: pDef 4,500, mDef 2,400, Exdefense 2; its 9,990 is the
physical PP defence, the gauge's). The port matched the game throughout
(`test_battle_rs.py` damage, protect, exdefense, affect: 0 mismatches on
INF and OUT).

The pilot picked the kind by the higher defence alone: spells at Black
Death, blows (and Skills! to the members) at the Gaia Turtle. In 262 it
cast at the Deadly Presents (row 243, physical barred) until the walk
gave them up (`HOPELESS`); `choose` then skipped them while the walk,
the room's doors shut, still went at them with the attack button.

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
- Survey aids (with god): a lone Kite in a field or dungeon at level 90
  (the story's aid for 206 is 75).
- Golden goblins of `goldVolume` 2 up (worklog 373): a blow makes one
  flinch and run, and it outruns Kite. The pilot uses a Speed Charm, the
  plain attack (an art lands one hit), the goblin casting its heal first,
  else one standing still, rushed with the stick at the rim (round a
  wall that stops the rush); while all run it waits for one to stop.
- Speed Charms bought (worklog 374): once Kite can act, the blows the
  goblins' HP takes at 15 a charm, and one more; short of them, Gate
  Out (after leaving the fight: it refuses in one), the town's magic
  shop (Buy, the count, OK), and back by the gate (`GateGoal::Shop`).
- A wanted event point's room with its Gott statue still shut after 300
  frames there, no foe about: the statue opened (worklog 375).
- Exdefense: a sure hit (`CalcBattleDamage` at 100 on a copy of the foe)
  tells whether a kind is barred. A foe barred from one kind gets the
  other (the members' order and Kite's page); Kite's and the members'
  skills that would do 0 are left out; a foe given up as hopeless is
  still fought with skills while the walk goes at it.

Both whole runs still finish, sooner: `outbreak_whole_story` 408,000
(from 463,200; 203 at 63,300 from 77,700), `mutation_whole_story` 506,100
(from 613,800). On 2026-10-04 (worklog 375): 423,600 and 497,100.

## Next

- 254: a way to out-damage GOB3-5's heals (worklog 374): summon scrolls
  (server 4's shop), or a player's record of the fight.
- Mutation's side events, the same way.
