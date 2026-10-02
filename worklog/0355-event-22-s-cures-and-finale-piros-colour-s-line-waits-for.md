---
number: 355
title: "Event 22's cures and finale: piros_colour's line waits for its window, and a room change disables the event task at once"
date: 2026-10-02
area: script, ui, world
files: crates/piney-event/src/vm/exec.rs, crates/piney-event/src/host.rs, crates/piney-event/tests/vm_fixture.txt, tools/test_event_vm.py, crates/piney-game/src/piros.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/session/tests/event22.rs, docs/engine/event-vm.md
---

# 355. Event 22's cures and finale: piros_colour's line waits for its window, and a room change disables the event task at once

Two reports on INF event 22 (PIRO02, the Cursed Piros quest), both fixed.

## Issue #30: the cure's line went by unseen

When Kite takes a cure (items 15/54-57), blocks 21-24 delete it, add one
to `eventStatus[1]` and set `eventStatus[24]` (the save's +0x6510) to the
message of the line: 16, 20, 24 or 35. Blocks 25-27 then run
`piros_colour 0`. On statuses 3, 5, 7 (and 9 with operand 0) the case
plays sound 74 and shows that message's first line through
`ccEvent::DispInfo` (INF SLUS_202.67:0x001b27b0). In the field,
`DispInfo` blocks:

- it waits for `ccMenuCtrl::CheckMenuType` to be -1;
- it calls `ChangeInfo`, then `ccBreathThread(5)`;
- it polls `Check(0)` a frame at a time until the window is answered;
- it calls `Close`, then `ccBreathThread(10)`.

The case's flash and tint come after. [[99]] ported the case from an eemu
run that stubbed `DispInfo` as a plain call, so the port opened the window
and went on. Five frames later message 17 replaced it.

Now `Host::piros_colour(code)` sets the case up and answers whether it
tells the line (`piros::Sequence::tells`). If so, the interpreter plays
sound 74 and runs `DispInfo`'s wait itself. It reuses the announcements'
code (`announce_from`, which now takes the window's opener), and
`Host::info_lines` opens the window. Then comes the host's
`Wait::PirosColour`. `Sequence` no longer carries the sound or the line.
Its first `tick` is the instruction's own frame. The fixture test
(`piros_colour_is_the_games`, 66 records) adds the sound and the line back
when the case tells one, and still matches.

`event_22_tells_piros_used_the_cure` (area 31's dungeon, the first cure
given) checks the order:
- the line (message 16) opens at frame 134, after sound 74;
- it is answered at 171;
- message 17 opens at 186: 10 frames of `DispInfo`'s tail, then the
  case's breath of 5.

## Issue #31: the finale played on the loading screen, and gave no diary

The fourth cure takes the status to 9. Block 28 then sets
`eventStatus[0]` to 2 and runs `room_point 5`. Block 29 (`eventStatus[0]`
2, in point 5, phase 4 or more) is the finale. It ends with `item_add_menu
0 12 24` (Piros' Diary) and `scene` to Mac Anu.

`room_point` calls `WORLD_MAN::RoomSelect` (main 0x0019dca0). It always
ends in `ccGame::ChangeScene`, whose `ChangeRequest(6, 7)` calls
`ccDisableThEvent` at once (phase -1). So in the game, the rest of that
pass cannot start block 29. Block 29 waits for the new room's set-up and
runs once play begins, in view.

The port's host made the scene change, but the task was disabled only
when the area took the request, after the interpreter's frame. In between,
block 29 started in the old room, where the scene had already moved to
point 5's room. The new room's set-up then held its loading frame
(`Phase::Hold`) through the whole cutscene. Its windows showed over the
black screen with the logo, as in the report.

The diary was lost the same way. `item_add_menu` asks for menu 29, but the
held set-up never runs the menus, so `field_menu` read -1 the next frame.
The interpreter took that as the menu closed, and nothing gave the item.

The interpreter now disables itself after `room` and `room_point`, as it
does at `scene`. Both run only while playing (level 2 or more; the
cases check `$s5 < 2`), and `room_point` with no such point does nothing,
as the game's loop does. `prev_room` (127, `WORLD_MAN::GoPrevRoom`
0x001a40b0) also ends in `ChangeScene` and is treated the same, though no
script on any of the four discs uses it.

`tools/test_event_vm.py` had stubbed `RoomSelect` and `GoPrevRoom` whole,
so its run of the game never reached their `ChangeScene`, and the fixture
kept the phase at 4 after a room change. Both are now hooked to disable
the task, as its `ChangeScene` hook already does. The fixture was written
again: 5 of its 252 `play` records changed, all to the port's new counts.
- Event 22 block 29 takes 365 frames (was 299) once block 28's `room_point`
  has disabled the task: its windows take the set-up screen's shape.
- Event 58 blocks 6-8 and event 162 block 8 change the same way, after an
  earlier block's `room`.
`every_block_played_matches_the_game` passes on it.

`event_22_finale_plays_in_the_room_and_gives_the_diary` gives the fourth
cure at status 8 and checks four things:
- block 29's first line opens in the new room's area, in play;
- menu 29 opens;
- Piros' Diary (12/24) is in Kite's items back in Mac Anu;
- `eventStatus[1]` ends at 10.

Without the fix, the line opens in the old room and menu 29 never opens.
The test helper `board_done` must stop rewriting `eventStatus[0]` once
the cure is given, or it undoes block 28's 2 in the new room's set-up.

Shots of the fixed finale: the room lit with Kite and Piros under message
32, then "You now have Piros' Diary!" over the same scene.

**Still unknown:** nothing.
