---
number: 306
title: Outbreak's side events surveyed under the story autopilot
date: 2026-09-30
area: script, test, volumes
files: crates/piney-game/src/session/tests/side_events.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session.rs, crates/piney-game/src/start.rs, crates/piney-game/src/area.rs, crates/piney-event/src/vm/cond.rs, crates/piney-event/src/vm/mod.rs, plans/outbreak-side-events.md, GAPS.md
---

# 306. Outbreak's side events surveyed under the story autopilot

Outbreak's side content had never run. `tools/evscript.py list` on
OUT SLUS_205.63 gives it:

- the S3 table, events 250-275 (`eventTblS300` at 0x00338000 to
  `eventTblS325` at 0x0033c180; 258 has no script);
- the S4 table's 361 (`eventTblS411`, 0x00342d20). It opens on 201 and is
  Outbreak's.

The rest of S4 (350-360) and M4 open on Quarantine's events. No M3 script
follows 219. The ML table (401-455) is the members' friendship mails.

That makes 26 events. `outbreak_side_event_survey` now plays them all;
22 reach their end. plans/outbreak-side-events.md has the table of
openers, starts and states.

## The survey, by volume

`side_survey(disc, cases, frames)` serves Infection's `side_event_survey`
and Outbreak's. For each case:

- **The start.** Each event starts from the story start after its last
  main opener (`crate::start::build`).
- **Earlier side events.** The ones it needs are brought forward as the
  start brings the story's (`Start::bring_forward`: `ccEventFlagSet`, all
  blocks at level 1). Mutation's are only marked done, as a carried save
  would hold them, along with the key items its SIGN-02 to SIGN-05
  (165-168) give: 287-290, which SEARCH MIMIRU, BEA, KURIMU and A-20
  test for.
- **Its own earlier blocks.** The blocks before the first one set in a
  field or dungeon (else a town) are marked run. Leading blocks with no
  settings are left alone, because they play wherever the event is open:
  361's block 0 ends it once 264 is done.
- **The party.** The members the event will not go on without (its
  `not_in_party` refusals) are put in the party (`start::party_of`, the
  Orca start's code shared).
- **The event point.** The session is placed at the first located block.
  In a dungeon it then goes to the event point's room as `room_point`
  goes there (`FieldWorld::room_select`), and the event task is idled at
  once, as `ChangeRequest` idles it. The old teleport (`Session::go` from
  the harness) left the task at phase 4. A phase-4 block of the new room
  then played before that room's phase-0 set-up had run: NUKE-2 played
  its fight scene with no foe entered, and `no_active` ended it on the
  spot. Infection's survey had the same flaw.
- **The pilot.** `StoryPilot` follows that event alone (`Follow::side`:
  `story_wants` reads it in place of the main story).

## What stopped them, and the fixes

- **The golden goblins** (GOB3-1 to GOB3-4; Infection's 50 and 51 too).
  The pilot never went after a foe in the open. The new `Want::Clear(field)`
  comes from a block that waits on `no_active` in a field where the event
  put a foe (`entry 5`). `approach_foe` then walks to the live foe with
  the least HP, sharing `approach_bug`'s path.
  - It leaves live Data Bugs and foes within 300 alone.
  - Without the `entry 5` limit it also chased 206's drained Data Bug
    (field 13, `entry 6`), and 206's end in the whole run moved from
    156,600 to 165,900.
- **Near a marker** (RACHEL-2, TERASHIMA-1, SIGN-7; Infection's 58 and
  61). The new `Want::Marker(n)` comes from `near_marker n` wherever
  distance 0 satisfies it. The pilot walks to event position `n` in a
  field, or in the dungeon room that holds it, before the walker, which
  stands still once it is in its goal room.
- **Items.** The wants now test `has_item` with the VM's own rule
  (`piney_event::vm::has_item`, made public). RACHEL-2 had waited at
  point 1 for its last block, which wants item 73; with the test it goes
  on to trade items 71, 72 and 73 over points 1-4.
- **The SEARCH NPCs stand in the Chaos Gate.** Marker 0 in a town is
  `markerEvTbl[0]`, `DMY_gate`, and `ccSelectTarget` (INF gcmn 0x00518cc0)
  lists the gate (its second pass) before PCs (its third). So beside the
  NPC the pilot lets the stick go and pushes it again, which steps the
  target on (mode 2: the stick's power rising past 64). This happens
  only while the gate is the target.
- **Fort Ouph.** The SEARCH chain opens on 201, but its NPCs go on to
  Fort Ouph, which only 203 opens (`town_move 3`), so the chain plays
  from 204.
- **Aid for a lone Kite.** With god, a lone Kite in a field or dungeon is
  raised to `LONE_LEVEL` (75), as the story does for 206. At 50, SIGN-7's
  Kite was felled in one blow, a game over near frame 106,000; at 75 it
  ends at 131,678.

## What is left

- **GOB3-5 (254).** The goblins in field 82 run and heal back, and are
  not caught in 150,000 frames.
- **MARLOWE-2 (256), TERASHIMA-1 (262).** The dungeon walk stalls in one
  room (84's 0-2, 90's 2-6). Its foes stop taking damage while Kite runs
  about.
- **SERVER-3 (263).** Black Death (OUT `enemyTbl` row 176: 9,999 HP,
  level 70, physical PP defence 9,990, `exdefense` 2) takes no damage
  from Kite at 75.
- **SEARCH TUKASA (275)** ends at the second meeting. Blocks 3 and 11 are
  both set in Dun Loireag and both hold `talked_to 159` in one pass,
  because `operateSet` is cleared only at the top of `ccThEvent`'s loop
  (INF 0x001b5a60) and `eventSub` (INF 0x001b5ef0) runs every block that
  holds. Block 11 then ends the event. This follows the documented walk,
  but it has not been compared with the game running.

No run panicked, left a host call at its default, or faulted.

## Checks

- `a_followed_side_event_wants_its_foes_down_and_its_marker`: `Clear(78)`
  for GOB3-1, and `Marker(0)` and `Point(1)` for RACHEL-2, only while the
  pilot follows them.
- `search_bt_meets_its_npc_at_six_gates`: SEARCH BT (268) plays blocks
  2-12 across the six gates and ends.
- `gob3_1_golden_goblin_is_run_down`: GOB3-1 ends in field 78.
- The whole runs are unchanged: `outbreak_whole_story` 463,200 frames
  and `mutation_whole_story` 613,800. The baseline was measured again from
  HEAD in a worktree.
- Infection's survey, with god: 50, 51 and 55 end, and 58 and 61 play
  their near-marker scenes.
- The piney-game suite passes (168) and piney-event's passes. Clippy and
  fmt are clean.

**Still unknown:** why the foes of field 84's room 0-2 and field 90's room
2-6 stop taking damage, whether they are out of reach or not being hit;
whether Black Death can be hurt at any level, and what `exdefense` 2
does; whether the game really ends SEARCH TUKASA at the second meeting;
and whether a Kite carried over from Mutation would need the level aid.
