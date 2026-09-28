---
number: 287
title: Open questions after the triage: the event scripts and their hosts
date: 2026-09-28
area: script
files: UNKNOWNS.md
resolves: 18, 40, 69, 74, 75, 86, 87, 91, 92, 99, 134, 143, 178, 180, 189, 270, 275
---

# 287. Open questions after the triage: the event scripts and their hosts

This entry closes the open questions of 17 entries about the event scripts:
the instruction set, the interpreter and the hosts that answer it in the
towns, the fields and the dungeons. The triage of 2026-09-28 (UNKNOWNS.md)
split every entry's open paragraph into single questions and checked each
one against the later log, the docs and the code. The answered ones are
listed below with their evidence. What is still open is restated at the end,
with the entry that asked it, the play questions first. Open questions of
these entries that belong to another subsystem are restated in that
subsystem's entry: [[282]], [[283]], [[285]], [[286]], [[288]] and [[291]].

## Answered

- [[18]]: the numbering of the streams, markers, menus, sound commands,
  player operations and NPC and PC action codes — answered by [[176]],
  [[178]], [[179]] / docs/engine/events.md; only `npc_act` -3 and -5 are
  unported, in the volumes' entry
- [[18]]: the mail and BBS state values — answered by docs/engine/events.md
  (`bbsList` 0, 1, 3, 7; mail 4-6), [[58]]
- [[18]]: the camera's angle units — answered by [[179]] /
  docs/engine/events.md (`DEG2RAD` shorts, 65536 to a turn; distances in
  tenths)
- [[18]]: whether the ML events (400-455) can open in Infection — answered
  by [[189]] (twelve of them open in Infection)
- [[40]]: the story areas' gate words and servers not loaded from the
  executable — answered by [[181]], [[217]] (generated per volume) /
  `crates/piney-data/src/area/mod.rs` `AreaTables`
- [[40]], [[92]]: `piros_colour` and its frame shape — answered by [[99]],
  [[107]] (ported; `tools/test_piros_rs.py`)
- [[40]]: the field side of the host (markers, distances, entries) —
  answered by [[74]], [[91]], [[134]], [[180]]
- [[69]]: `end_event` not reached after the scene — answered by [[72]],
  [[74]]
- [[69]]: `WORLD_MAN::SetEventData` — answered by [[87]]
- [[69]]: `teach_input` (event 3's camera lesson) — answered by [[75]]
- [[69]]: the party's AI, `pc_walk` remote commands 1 and 2, the message bus
  — answered by [[78]], [[178]] (the towns' remote walks), [[223]]
- [[74]]: menus 80 and 83, `player_skill`, `hold`, the magic portals
  (`entry_mc`), the members' field AI — answered by [[78]], [[80]] /
  docs/engine/battle.md "`player_skill`", "`hold` and `ccThEvHold`"
- [[74]]: `pc_command`, the walks and the puts — answered by [[178]] (remote
  walks), [[134]]
- [[74]]: the event positions in a field or dungeon — answered by [[87]],
  [[134]] (`evPos` outside the towns); a plain field's `marker_pos` is in
  the volumes' entry
- [[74]]: Kite stays held after event 3 (`bootParam` 4) — answered by
  docs/engine/battle.md "The party's `bootParam` from area to area"
- [[74]]: the party not carried back into Mac Anu after Gate Out — answered
  by [[108]]
- [[74]]: registered characters outside the party are not built — answered
  by [[180]]
- [[75]]: the town's event host gap for `teach_camera3` — answered by
  `crates/piney-world/src/lib.rs` `World::teach_camera`
- [[86]]: the desktop's `play_pass_done` and `load_overlay` — answered by
  [[91]]
- [[86]]: branches the talk-through did not take may call other defaults —
  answered by [[176]] (every host call the scripts reach)
- [[87]]: event 4's host methods `add_spc_item`, `item_get_menu(_end)`,
  `remove_trap_done`, `room`, `gimmick`, `boss` — answered by [[98]],
  [[111]], [[113]] / `crates/piney-game/src/area_host.rs`
- [[87]]: the dungeon's own objects (boxes, Fortune Wire, idols) — answered
  by [[98]]
- [[87]]: `SetDoor`'s ban room — answered by [[114]] (the ban block)
- [[87]]: the door palettes for `clutType` 3 and 4 — answered by [[147]]
- [[91]]: the church's streams lack subtitles and music — answered by
  `crates/piney-game/src/area_host.rs`, `field_host.rs`
  (`StreamPlayer::event`)
- [[91]]: `party_remove`, `game_over` and `clear_gate_hack` still defaults
  in the fields — answered by `crates/piney-game/src/area_host.rs` (all
  three implemented)
- [[92]]: `trans` not run in a playthrough (events 14, 27, 29) — answered by
  `crates/piney-game/src/area_host.rs`, `field_host.rs` (`trans`); [[93]];
  the story survey (`crates/piney-game/src/session/tests/survey.rs`
  `story_survey`) plays Infection's events 1-31
- [[92]]: no story starts past event 14 — answered by [[93]]
- [[99]]: `piros_colour` in the field and dungeon — answered by [[107]]
- [[180]]: `inviteSpc`'s build at the Chaos Gate — answered for the towns by
  docs/engine/field-walk.md (`party_add`), [[250]]; the fields and dungeons
  are in the world's entry
- [[270]]: the whole run after 108 not seen — answered by [[276]] (101 to
  116 under the autopilot)

**Still unknown:**
- (play) [[87]]: the event cameras in event 4's rooms (the user saw the
  camera in the walls) were not re-shot since each block plays in its own
  room — event 4's dungeon scenes.
- [[11]]: which story areas Infection itself can reach; derivable from the
  scripts' `gate_add` (docs/engine/events.md, ops 89 and 90), but no list is
  made.
- [[18]]: how far the parody script diverges beyond the lines compared; only
  sampled.
- [[69]]: when the field set-up enables the event task; taken from the
  desktop's check (docs/engine/event-vm.md Unknown).
- [[74]]: when the event task wakes, read from the thread code
  (docs/engine/field-walk.md Unknown).
- [[69]]: the event camera's remaining unknowns: its work-area reads, stack
  leftovers, `currentOpen`, the base-id lookups, the FIFO order within a
  priority (docs/engine/field-game.md Unknown).
- [[87]], [[111]]: `prev_room` (`WORLD_MAN::GoPrevRoom`) is still the host
  default; no volume-1 script needs it, and the later volumes are not
  surveyed.
- [[109]], [[117]], [[128]]: `hold 7` is not ported; no Infection script
  uses it (docs/engine/boss.md "Not described yet").
- [[99]]: `piros_colour` status 9 with an operand past 5; no script uses it.
- [[178]]: walks along the town navigator's route (`gPoint` not -1) are not
  ported for Kite; no event sets one.
- [[134]]: whether the game's `ccGetDist` in `CheckOpen` takes the wrapped
  frame for both points (assumed).
- [[107]]: whether block 18's `in_point 5` gates block 19 in the dungeon's
  first room.
- [[228]]: what sets and clears `eventStatus[40]` in Mutation's story.
- [[270]]: whether the game delivers mail 88 in the same desktop session as
  the port; not checked against the executable.
- [[189]]: whether a player of Infection alone reaches friendship 50 with
  Black Rose by event 14.
- [[90]]: `ccEvent::CheckOperate(9)`'s whole talk rule is not run in eemu;
  its pieces are checked.
- [[91]]: `player_distance` in town is not run whole against the game; it
  reuses checked pieces.
- [[91]]: the church's messages, cameras and timing are only seen to run.
- [[277]]: Black Rose's own lines at the holy ground once her talk target
  was right.
- [[143]]: event 27's talk (walking PC row 85) is not played through by a
  test.
- [[177]]: event 62 is not played through end to end (mail, words, portal,
  the scene back).
- [[178]]: event 30's walks in Dun Loireag are not run in a test (the same
  code as Mac Anu's).
- [[178]]: the side events are not played to their ends.
- [[180]]: event 61 is driven only to its opening lines; events 58 and 60
  have no tests.
