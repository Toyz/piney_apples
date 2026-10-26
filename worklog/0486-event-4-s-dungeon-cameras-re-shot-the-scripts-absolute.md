---
number: 486
title: Event 4's dungeon cameras re-shot: the scripts' absolute points on rooms the game's way; block 6's sweep past the walls is the script's own
date: 2026-10-25
area: world
files: crates/piney-world/src/evcam.rs, crates/piney-game/src/session.rs, UNKNOWNS.md
resolves: 287.1
started: 2026-10-25T18:35:02-06:00
took: 3m
---

# 486. Event 4's dungeon cameras re-shot: the scripts' absolute points on rooms the game's way; block 6's sweep past the walls is the script's own

Play question [[87]] (287.1): the event cameras in event 4's dungeon
rooms. The user once saw the camera in the walls. That was before
[[87]] put each block in its own room, and the scenes had not been shot
since.

**Shots.** `event_4_shots` was run with marks at every camera block:
- the arrival (block 4's `camera`), the treasure box lesson and the
  doors closing (block 6's `camz_set`) are framed on Kite and Orca;
- the portal lesson, `message_open 4 6` at 1, 15, 30, 45 and 80 frames,
  and message 7;
- the statue room (block 7).

All are framed as the scripts mean, but for a moment of block 6. While
message 6 opens, `camz_move` (vprate 70, cprate 80, `camz_speed` 2)
sweeps the eye from (22330, 13860, 50) to (24250, 13770, 350). The look
goes from (21860, 13690, 120) to (24200, 13270, 330). The eye passes
close to the room's walls and pillars for about half a second (frames
15-30 fill the screen with wall). From 45 frames it settles on the far
portal at (24000, 12000).

**Why this is the game's picture.**
- The camera instructions take absolute points: `ccEvent::Execute`'s
  `camz_set` and `camz_move` (INF main 0x001ac1c8, 0x001ac348) times ten,
  with no event or room origin. The port's camera (`evcam.rs`: the
  handlers, `ccThCameraExecute`, `CamCtrl`, `CamzMovMain`) is checked
  against the game by `tools/test_evcam_rs.py`.
- Story area 14's dungeon is built as the game builds it:
  `tools/test_dungeon_rt.py` runs `Generate`, `SetRoom` and the rooms'
  centres in eemu for `dungeon:14`.
- Event 4's points come from `SetEventData`, checked in the interpreter.
- Nothing in `DUNGEON::Draw` (0x005ce930) reads the camera: it draws the
  room under the player as built. The GS has no back-face culling
  (docs/engine/render.md).

So the eye's straight path past the walls is the script's own, the same
on the PS2. No code changed.

**Still unknown:** nothing
