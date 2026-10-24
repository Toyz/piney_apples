---
number: 441
title: Quarantine's story surveyed: fourteen starts, nine events done, the rest stopped by town 5, two missing bosses, a short run and an ending's area
date: 2026-10-24
area: script, test, volumes
files: crates/piney-game/src/start.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, plans/quarantine-story.md, docs/engine/event-vm.md, GAPS.md
started: 2026-10-24T01:13:51-06:00
took: 17m
---

# 441. Quarantine's story surveyed: fourteen starts, nine events done, the rest stopped by town 5, two missing bosses, a short run and an ending's area

Quarantine's story is now surveyed as Mutation's and Outbreak's were:
each of its fourteen events started on its own and driven by the story
autopilot. Nine of the fourteen finish; the five that stop each stop on a
piece of the game the port lacks or on a fight longer than the run.

## The starts

`tools/evscript.py list` on Quarantine's executable: M401-M414 are events
301-314, each opening on the one before (`event_done`), 314 the
"Ending". M415 (315) is the after-game and M416, M417 the extras.

Each start is where its event's first located block is, as for
Outbreak (`start::place`, `log_in_town`):

| events | start |
| --- | --- |
| 301 | the new game's desktop, nothing brought forward |
| 302, 303, 305-307, 311, 314 | the desktop |
| 304 | Fort Ouph (`in_town 3`) |
| 308-310, 313 | Lia Fail (`in_town 4`) |
| 312 | Mac Anu (`in_town 0`) |

313's first blocks, in towns 0-3, only mark its gate (`gate_mark 106`,
repeatable); the first that plays a scene is town 4's, so it starts
there. All fourteen are in `POINTS` now (308 and 312 were the only two,
for Kyvia's tests).

`quarantine_story_starts_open_their_event` checks that each opens its
event and that nothing before it plays again (14 starts).

## The survey

`quarantine_story_survey` with `PINEY_SURVEY_GOD=1` and 40,000 frames
an event took 447 s:

| event | end | where it stops, and why |
| --- | --- | --- |
| 301 | blocks 0x7b | Fort Ouph. Block 7 is in town 5 |
| 302-306, 309 | done | |
| 307 | blocks 0xf | field 6's arena: `entry 7 5`, no `ccBoss06` |
| 308 | blocks 0xf7 | field 11: Kyvia's third fight still on |
| 310 | blocks 0x3d1 | field 7's arena: `entry 7 6`, no `ccBoss07` |
| 311 | blocks 0x6ff | field 103's dungeon, the first of four goal rooms done |
| 312 | blocks 0x209 | field 108's dungeon |
| 313 | blocks 0x1 | Lia Fail: BlackRose's question unanswered |
| 314 | blocks 0x7 | field 122 through the gate, not field 126 |

Notes on the stops:

- **301.** Town 5 is `town06`: the sixth case of `WORLD_MAN::GO(0)`'s
  town switch (QUA main 0x001b2ac0) constructs it at gcmn 0x004ec350,
  a class no symbol names. Nothing moves the player there by name. The
  gate's town warp (`GtTownMenu`, QUA gcmn 0x0047305c) turns town 4
  into town 5 while `eventStatus[51]` is set. Event 301's block 3 sets
  that byte, and block 8 clears it before its `scene` to town 4. The
  same check is in Mutation's and Outbreak's `GtTownMenu` (MUT
  0x0058499c, OUT 0x0058071c), not in Infection's. Quarantine's carry
  names the function `GtRandomMenu+0x964`: its `GtTownMenu` symbol was
  not carried.
- **307, 310.** `start_boss` takes no code 5 or 6, so `absent 7 5`
  waits for a fight that never comes.
- **314.** `area 126` names a story area with no words, and the VM
  reports `area` and `change_scene` unported there. The pilot then
  takes the gate to the nearest area it knows, field 122.

plans/quarantine-story.md is the plan and ledger, as the two before.

**Still unknown:** nothing: each stop above is a piece of the game the port lacks, taken up in story order in plans/quarantine-story.md
