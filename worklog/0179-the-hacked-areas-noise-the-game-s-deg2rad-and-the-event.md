---
number: 179
title: "The hacked areas' noise, the game's DEG2RAD, and the event pages' unknowns"
date: 2026-09-26
area: script
files: crates/piney-game/src/area_host.rs, crates/piney-game/src/field_host.rs, crates/piney-event/src/vm/exec.rs, tools/evscript.py, docs/engine/events.md, docs/engine/event-vm.md
---

# 179. The hacked areas' noise, the game's DEG2RAD, and the event pages' unknowns

Going down the "Unknown" lists of `events.md` and `event-vm.md`.

**The first play pass's noise.** `event-vm.md` said `play_pass_done`'s
`worldman +0xf0 == 3` case (`ccMenu +0x104 = 1`) was not modelled, and
inferred that a town never has it.

- `ccThEvent` (0x001b5eb0) was read: after the first play pass,
  `WORLD_MAN.hackFlag == 3` sets `ccMenu.interNoiz` (+0x104) to 1, the
  menu's noise (`field-ui.md`).
- The writes of `hackFlag` in `WORLD_MAN::GO` were found: before GO
  branches on the area, it sets 2 for every area, or 3 when the save's
  crisis byte (+0x6772) is set; a field or a dungeon then takes its
  area's own.
- So the inference was wrong: in the crisis a town has the noise too.

The area host now turns it on from `WorldMan.hack`, which already holds
3 for a hacked area and for any in the crisis. The town host turns it on
from the crisis byte.

**DEG2RAD.** Also an unknown: whether the event VM's `deg2rad` (event
positions' headings) equals the game's. It computed `pi * v / 32768` in
the host's IEEE floats. The game's `DEG2RAD` (0x001dabb0) was run in
`tools/eemu.py` for all 65,536 shorts:

- The IEEE form is one bit high for 32,680 of them.
- The EE-arithmetic form the world and battle crates already use
  (`piney_data::field::ee` `mul` and `div`) equals the game's for all of
  them.

The VM now uses the EE form, and `deg2rad_is_the_games` pins four of the
values that differed.

**Numbering and meanings that were already known.** Several items only
needed a pointer to where the answer lives:

- The board states: 0, 1 new, 3 read, 7 the player's own post waiting to
  be written out. The top page's port has them.
- `emode`: the low byte picks how the window ends (0 closes, 1 chains, 2
  stays up, 3 asks); 0x100 shows the text untyped; 0x200 is a typing
  test with no effect. The desktop and field UI pages have these.
- The camera: its angles are `DEG2RAD` shorts, its distances tenths, and
  `cam_mode4` (`cpCtrl` 4) holds the camera point while the look-at goes
  on. `piney_world::evcam`, checked against the game, has this. The
  opcode table's "mode not traced" is gone, in `tools/evscript.py` too.
- `WORLD_MAN::SetEventData` "not read": `dungeon.md` documents it and the
  area mode runs it.

`events.md`'s unknowns are now only the later volumes' (their BSS
message groups, OUT's rewritten cases, the KFED thread).

**Still unknown:** The crisis town's noise was not seen in a picture; it
follows from the code. Whether any Infection field is hacked outside the
crisis (`EVENTAREA_INFO.flag` 3) was not surveyed.
