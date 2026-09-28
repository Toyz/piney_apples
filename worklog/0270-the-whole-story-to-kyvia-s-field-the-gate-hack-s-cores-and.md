---
number: 270
title: The whole story to Kyvia's field: the gate hack's cores and the desktop's late mail
date: 2026-09-28
area: test, script
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session.rs
---

# 270. The whole story to Kyvia's field: the gate hack's cores and the desktop's late mail

`mutation_whole_story` (one run from Mutation's new game) now ends events
101 to 107 and walks into field 9 for 108, where it waits on Kyvia (not
yet ported). Before this it ended 101 to 104 and then stood at the Chaos
Gate. Five things were in the way. All of them were in the pilot or the
harness; none were in the port.

## Gate Out from a field

After [[268]], the run stood in field 58 with nothing the story wanted
there. `gate_out` opened PERSONAL and waited for menu 0, but menu 0 is
PERSONAL only in a town. In a field it is 1, and in a dungeon it is 2. The
pilot now takes Gate Out from menu 1 or 2, and 104 ends at frame 117,600
back in Carmina Gadelica.

## A dungeon room's shut doors

In 115's dungeon the walker stood at a door that would not open. The
room's doors stay shut while its foes stand (`MoveDoor`'s `doorAnm`). The
walker had marked those foes too far or hopeless and left them alone. Now,
while `DungeonArea.door.door_anm` is false, every foe in the room is fought.
The 115 survey then reached field 13.

## The gate hack's Virus Cores

Area 45 is protected, so the gate's Warp opens the hack (menu 62). The
hack wants the area's `protect` row: four pairs of core kind and count
(key items, category 15). A player drains common foes for these cores, but
the pilot drains only bosses, so it never had them, and the run stood
before the hack from frame 129,000 to the end.

Under god mode, `cores_for_hack` (`mutation_whole_story`, and the survey's
`PINEY_SURVEY_GOD`) now gives any cores that are short through the
console's `core LETTER N` while menu 62 is open. This is a harness aid,
like god and the infection hold. The pilot's own (62) arm then sets each
slot and confirms, and 105 and 106 end at frames 146,100 and 147,900.

## The desktop's late mail

After 107 the run stood on the desktop for 200,000 frames. The trace
showed `operate` 0, with the menu idle and mail 88 unread. Reading mail 87
had delivered 88, but the desktop lists its inbox only once, in
`Desktop_control::AddAllList` (0x00402030 in the desktop overlay, as
the port cites it; run after the opening). So mail
88 was not in the list until the desktop opened again.

The pilot aimed at Mail whenever any mail in the save was unread. Each
time it found nothing unread in the listed inbox, backed out, and went in
again. It now aims at Mail only for the listed inbox, so it logs in
instead. 108 then takes it to field 9 (frame 300,000 onward).

The desktop trace (`PINEY_SURVEY_TRACE`) now also prints the `operate`
bits and the menu task's menu, status, open request and step.

## Where the run stands

At 500,000 frames the run is in field 9 with the wants `Leave`,
`Town(2)`, `Party(16)`, `Area(47)`, `Dungeon(47, 0)`, `Point(2)` and
`Area(9)`. 108's `entry type=7 code=12` needs Kyvia, whose port is under
way.

**Still unknown:**
- Whether the game delivers mail 88 in the same desktop session, as the
  port does: a script's mail on reading 87, or on the next start. It was
  not checked against the executable.
- The whole run after 108 (109 on) has not been seen yet; the single
  surveys for 109-114 and 116 end, each from its own start.
