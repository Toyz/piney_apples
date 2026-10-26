---
number: 495
title: Which events place characters in Fort Ouph and Lia Fail, each placed by the port; a party entry at marker 99 stays put, and a full registry still places a registered one
date: 2026-10-25
area: script
files: crates/piney-world/src/party.rs, crates/piney-world/src/event.rs, crates/piney-world/tests/later_towns.rs, docs/engine/field-walk.md
resolves: 291.13
started: 2026-10-25T20:40:48-06:00
took: 16m
---

# 495. Which events place characters in Fort Ouph and Lia Fail, each placed by the port; a party entry at marker 99 stays put, and a full registry still places a registered one

Play question [[260]] (291.13): which events place characters in Fort
Ouph and Lia Fail, and does the port place them?

**Which events.** Read from each later disc's scripts (the same on
Mutation, Outbreak and Quarantine). A block counts when its own
`in_town` condition, or the scene tag in force, names the town.
- Fort Ouph (town 3), 26 `entry`s: Outbreak's 205, 207, 209, 210 and
  211; its side events 262 and 267-275; Quarantine's 301 and 304.
- Lia Fail (town 4), 41: Quarantine's 301, 303, 306, 309, 310 and 313,
  and side event 360.
- The types are party members (2) and town NPCs (3, and the
  Administrator's type 4). Side events 267-275 put their NPC with
  `param` -1, which the town's search places.
- All of these run under the crisis: event 116 sets it and 313-315
  clear it. So Fort Ouph's file is `town04d`; it has `DMY_marker_ev07`-
  `09`, which event 209 needs and `town04` lacks. Lia Fail has one file.

**Two rules the port lacked**, found by putting every entry through
the town:
- **Marker 99.** From Mutation on, `ccEntryEventMng` (MUT main
  0x001cba88, OUT 0x001c2158) puts a party character with a marker of 99
  or more back at its own `pos` and `dirc`, instead of a dummy.
  Infection has no such test. Lia Fail's event 301 block 9 enters Kite
  this way, and the port used to put him at the origin.
  `World::spc_marker` now answers None there, and `place_spc` and Kite's
  at-once path keep the character where it stands.
- **A full registry.** `EntrySpc` gives -1 with five in the registry.
  `ccRegisterEventMng` (OUT 0x001c2c78) then only skips the
  `bootParam`; the entry stays on the list, and `ccEntryEventMng` still
  places a character that is already registered. 301 block 9 enters
  Kite with four members already in. The port's `entry_spc` refused it;
  now it is false only when the character is not registered at all.

**Checked.** crates/piney-world/tests/later_towns.rs
(`every_entry_in_fort_ouph_and_lia_fail_is_placed`) runs on the
Mutation, Outbreak and Quarantine discs. For each block it enters the
town under the crisis and puts the block's entries through `World::entry`:
- every marker is in the town's file;
- every entry is taken;
- after the set-up's frames, each party member and NPC stands at its
  marker;
- Kite's entry at marker 99 leaves him where he was.
piney-world's tests and piney-game's suite (335) pass.

Docs: field-walk.md (the town's marker rules and the list of events),
UNKNOWNS.md.

**Still unknown:** nothing
