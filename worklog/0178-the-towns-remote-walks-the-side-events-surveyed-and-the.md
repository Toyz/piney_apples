---
number: 178
title: "The towns' remote walks, the side events surveyed, and the operations numbered"
date: 2026-09-26
area: world
files: crates/piney-world/src/ai.rs, crates/piney-world/src/party.rs, crates/piney-world/src/event.rs, crates/piney-world/src/player.rs, crates/piney-world/src/combat/spc.rs, crates/piney-world/tests/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/side_events.rs, docs/engine/field-game.md, docs/engine/events.md
---

# 178. The towns' remote walks, the side events surveyed, and the operations numbered

**The side events.** `side_event_survey` (ignored, a diagnostic) plays
Infection's side events 50-53, 55 and 58-61 where they happen. For each
event it:

- takes the story start the event opens after, and marks as done the side
  events it needs;
- marks as run the blocks before the first one that names a place (the
  board's post, the read, the words);
- puts the session in that field, or in the dungeon room of the event
  point, found from the points the set-up registers;
- drives it with the story autopilot.

None panicked and none reached a host default.

- The goblin chases (50-53) place their goblin (`entry 5 131` and on)
  and talk.
- 55 closes itself.
- 59 and 60 run most of their blocks.
- 58 and 61 stop where the player has to walk to a marker or fight.

**A gap the host check could not see.** Worklog 0176's check found every
host method implemented. But the town's `pc_command` answered the remote
walks (`pc_walk_pos`, `_dir`, `_marker`, `_char`, `pc_run_pos`) and
`pc_command` itself with "not done". Two Infection towns walk characters
this way:

- event 21 walks Elk up to Kite in Mac Anu (`pc_walk_dir 10 6144 30`);
- event 30 walks Kite and BlackRose in Dun Loireag (`pc_walk_pos`).

The fields had these through piney-battle's `EvParty`.

**What was missing** was only the town side:

- A member in a town is a battle character (`TownParty`), so the battle's
  `ManualControl` (`ai_move`, already checked against the game) walks it
  once its AI has the remote command and the goal. piney-world's `Ai`
  gained `gPoint` and `gPos`, and the bridge to the battle's AI copies
  them both ways.
- Kite in a town is the town's own player, whose `ManualControl`
  (`piney_world::ai`) did remote commands 0 and 3-7 only. It now does 1
  and 2 on a straight goal, transcribing piney-battle's `MoveP2P`
  (0x00582d50): a step toward the goal, turning to face it when it may
  (`CheckAction` 2) and starting to move when it may (`CheckAction` 1).
  Once the ground distance less the body's radius is under 50, the body
  stands, the command is done with `remoteFlag` 1, and `gDeg` takes the
  heading the body had. `SpcRef` carries the stride (`velocity`, Kite's
  `speed`).
- `party::pc_walk` starts a walk: `ManualModeAI(1)`, `SetRemoteCmd`, and
  `SetGoalPos(v, 0)`. The goals are as `EvParty`'s: tenths for `_pos`,
  the offset for `_dir` and `_char`, the marker's dummy with w 1 for
  `_marker`. `party::pc_command` lists or unlists the characters named.

A detail the first test assertion missed: running widens the body's hit
radius (Kite's 45 is 72.5 on the run), so he stops about 107 short of a
goal 300 away.

Checks:

- `kite_walks_to_an_event_goal` (piney-world): Kite walks 300 in Mac Anu
  and stops by that rule, with the command done.
- `event_21_elk_walks_up_in_mac_anu` (piney-game): story 21 played by the
  autopilot to block 4. The walk is done and Elk moves more than 100
  before the gate address.

**The operations numbered.** `events.md` listed "operation numbering" as
unknown. Every `CheckOperate` call site in main and the overlays (`a1`,
the method's number) with the port's reading of each gives the table now
in `events.md`:

- 0-5 the desktop's icons; 6-8 the top page's commands;
- 9 action, 10 chat, 11 START on the desktop and PERSONAL in The World,
  12 options, 13 the map, 14 leaving (Gate Out, the Sprite Ocarina), 15
  a gate hack, 16 the Chaos Gate's Warp, 17 a skill;
- the second locks 19-24, 25-27 and 28 (`_sf`).

The rest of that unknown line was already answered elsewhere: the
streams (`streamTbl`), the markers (`markerEvTbl`), the menus (the field
UI's table), the sound commands (0176), and `pc_act`'s and `npc_act`'s
codes. The line now points to each. `npc_act` -3 and -5 are the only
codes not ported, and only volume 2's event 114 uses them.

**Still unknown:** The walks along the town navigator's route (`gPoint`
not -1) are not ported for Kite; no event sets one. Event 30's walks in
Dun Loireag were not run in a test, only the same code in Mac Anu. The
side events were not played to their ends: the fights, the walks to
markers and the rewards need a player the autopilot is not.
