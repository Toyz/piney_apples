---
number: 358
title: "Infection's whole story under the autopilot: the pilot's gaps up to event 21"
date: 2026-10-02
area: test, script
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs
---

# 358. Infection's whole story under the autopilot: the pilot's gaps up to event 21

[[276]] ran Mutation's story from its new game to its end under the story
autopilot (`mutation_whole_story`), and Outbreak's followed. Infection,
the volume being finished first, had only `story_survey`: one story start
at a time, 4,000 frames each. `infection_whole_story` now runs INF from its
first story start (event 3; 1 and 2 are the desktop's) to event 31's end.
Its first runs stopped at once. Each stop was the pilot's, not the port's.

## What the pilot lacked

- **Event 10: Kite's own post.** `bbs_post7` leaves a post in state 7,
  which the board writes out on the next visit. The top page player read
  only new posts (state 1), so it went between the desktop and the top
  page for ever. It now opens the board first when
  `piney_toppage::bbs::check_write_bbs` finds a post waiting.
- **Event 11: targets that are not town NPCs.** The town talk looked
  every `add_target` up as an NPC. Event 11's are BlackRose (type 2, a
  party character) and the Chaos Gate (13). `GateGoal::Talk` now carries
  the kind: 2 a party character, 3, 4 and 8-12 an NPC (8-12 are event
  16's five merchants), 13 the gate.
- **Events 12 and 14: a closed event.** An event's `end_event` closes it
  (bit 63), and it turns done (bit 62) only at the next mode's
  `ccStartThEvent`. The next story event opens on done. With the story
  wanting nothing in town, the pilot took the last marked area (field
  28, an old mark) instead. It now logs out whenever a story event is
  closed and not yet done.
- **Event 13: a town marker.** Block 2 waits on `near_marker 31` in Mac
  Anu. The pilot walked to markers only in fields. `GateGoal::Marker`
  walks to `World::marker`'s place first.
- **Event 21: Elk alone.** Block 9 refuses the gate while anyone but Elk
  is along (`in_party 10`, `party_other 10`). The pilot had filled the
  free slot with Mistral. `story_wants` now gives `Want::Only(pc)` for a
  `party_other` block; the pilot then sends the others away, re-invites
  the member, and fills no slots.
- **Event 22: two towns.** Event 22 wants both Mac Anu and Dun Loireag.
  Each town's goal was the other, so Kite went back and forth. A town
  that is itself wanted now keeps him.

`whole_story` now starts at the volume's first story start
(`crate::start::POINTS`). It gives up after `PINEY_SURVEY_STALL` frames
(200,000) with no event ending, and says where. The stall report
(`whole_state`) adds, in a field or dungeon, the event targets, the foes
and the nearest one's distance, `in_battle`, the menu, the phase and the
playing block.

## Where it stands

Events 3, 4 and 10-21 end, the last at frame 222,600 (with god, the
infection held at 0, cores given for the hacks, as the survey's aids do).
The run then stops in area 31's dungeon with Piros in the party. Event
22's blocks 21-24 wait on Kite holding the cures (items 15/54-57). In a
story dungeon the idol of a story room holds that room's `itemID`
(`SetIDOL`, `ep.param[1]`), and the pilot never opens idols.

The stall in field 28 showed one more thing to look at: `in_battle`
stayed 1 for 60,000 frames with Kite idle at the field's start. The
pilot's `gate_out` takes `in_battle` as busy, so it never left. That run
predates the closed-event rule, which keeps the pilot out of field 28;
the `in_battle` reading itself was not explained.

**Still unknown:**
- The pilot opens no idols, so event 22's cures (and any story item an
  idol holds) are not taken. Event 22 onward is unrun.
- Why `in_battle` held at 1 in field 28 with no fight; a pilot that
  waits on it can stall.
- `story_survey` (one start at a time) was not rerun across 1-31 after
  these changes; the game suite's pilot tests pass.
