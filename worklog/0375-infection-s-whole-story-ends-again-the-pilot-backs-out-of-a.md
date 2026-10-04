---
number: 375
title: Infection's whole story ends again: the pilot backs out of a passer-by's talk
date: 2026-10-04
area: test, world
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs, plans/outbreak-side-events.md, plans/outbreak-story.md, plans/mutation-story.md
---

# 375. Infection's whole story ends again: the pilot backs out of a passer-by's talk

`infection_whole_story` stalled in Mac Anu after events 1-4 and 10, at
frame 215,400 (worklog 372). In this entry it runs to event 31's end
again. Looking at the other whole runs and worklog 374's two open side
events turned up two more stalls of the same kind: the port changed
rightly, and the pilot had no answer.

## Where it broke

A worktree (`/mnt/data/claude/wt-bisect`, its own target dir) and `git
bisect`. Each step ran `infection_whole_story` for 40,000 frames
(`PINEY_SURVEY_FRAMES`) and looked for event 11's end:
- 1c36760 (worklog 359's): good, event 11 at 19,800;
- 97ec1f6: bad;
- 5a707f6, 7c01aca, e129a31: good.

The first bad commit is 97ec1f6, "The game's one rand()" (worklog 364).
Since then, a town takes the stream the last mode left instead of
starting at 1, so Mac Anu's walking PCs walk differently.

## The cause

`PINEY_DEBUG_PILOT` at the stall: from 18,000 on, Kite is still at
(-12, 1912), the command target is Npc 54 (a walking PC), and menu 47
(`TalkMenu`) is open.
- At 15,800 the pilot's goal was event 11's Chaos Gate target
  (`add_target 13 16`). A passer-by was the command target when an OK
  went, and his list (`PcMenu`, 22) opened. A trace in the new arm below
  shows it: target Npc 54, goal the gate.
- `gate_player` answered 22 for any `GateGoal::Talk` with OK: "a talk
  the events do not take: on". That took Talk.
- It had no case for 47. `TalkMenu` waits for the line's OK
  (`ccMsg->Check(1)`, docs/engine/field-ui.md), so it stood there for
  200,000 frames.

The port behaved as the game: one `rand()`, and a walking PC can step in
as the command target. The pilot was at fault. Now:
- A PC's list (22) whose command target is not the talk the pilot went
  for is backed out of (CIRCLE).
- `TalkMenu` (47) gets OK. The line closes and its list comes back.

`infection_whole_story` (full run) ends at 408,900, event 31's staff roll.
Events 11-13 at 18,900, 20,400 and 21,900.

## Outbreak's whole run: a rock on the way to area 47

`outbreak_whole_story` stalled after 210 at 468,900. Kite was in area
47's field, still at (33,299, 19,929) on the walk to the dungeon. At
268db68 it stalls the same, so this came before this work.
`outbreak_story_survey` from start 211 shows it in 5,000 frames:
- at baa897d Kite is in the dungeon by 6,000;
- at f87afc7 (worklog 366: every field object has its own hit) he
  stands at (33,299, 19,929).

Before that commit only the last-drawn rock of a model stood in the way.
Now a rock is on his straight line. `walk_into_dungeon`'s far search
(`path_to`, 81 cells) found nothing. Pressed to the rock, only one start
cell is clear of it, and from that cell no neighbour is. The pilot now
steps aside at a right angle for 90 frames, the other side each time,
when the search finds no way. The run ends at 423,600.

## SANJYURO-4 and MOON-2: the statue that holds `no_active`

Worklog 374 left 260 and 261 open. Both stop in the room of their last
point (7 and 2), no foe left. The blocks there (`tools/evscript.py dis`)
wait on `no_active`, then `item_del` the key item that room's Gott
statue gives: 283 (260), the Moon Knife 284 (261). Since 86c7903 (#18),
`no_active` also waits on a switched-on statue still on the command list
(`no_active_object`, INF main 0x001a8614-0x001a86d4). A player opens it.

The pilot opened statues only for a block's `has_item`. With the statue
rule switched off for a run (and no opener), both end. So the rule is
what stops them. 86c7903 predates data version 22, so it could not be
run in the worktree against today's port data.

The pilot now opens it: in a wanted point's room, no foe about and not in
a fight, after 300 frames there, the shut statue
(`open_point_statue`). 260 ends at 16,908, 261 at 31,570.

## Checked

- Whole runs (release, the survey's aids):
  - `infection_whole_story` ends at 408,900;
  - `mutation_whole_story` at 497,100;
  - `outbreak_whole_story` at 423,600.
- `outbreak_side_event_survey`, all 26: 25 finish. 254 stays open
  (worklog 374).
- New tests:
  - `story_211_walks_round_the_rock_into_area_47_s_dungeon`, in the
    dungeon within 10,000 frames;
  - `sanjyuro_4_opens_point_7_s_statue`, ended within 22,000.
  - With the old pilot, the first stands at the rock and 260 stays
    open.
- piney-game suite: 236 passed, 82 ignored, four threads. Clippy on
  piney-game clean; `cargo fmt --check` clean.
- The worktree and its target dir were removed.

**Still unknown:** nothing.
