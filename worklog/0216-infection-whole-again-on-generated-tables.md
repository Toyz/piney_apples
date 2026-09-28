---
number: 216
title: Infection whole again on generated tables
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/world.rs, crates/piney-world/src/navi.rs, crates/piney-world/src/town_party.rs, crates/piney-world/src/event.rs, crates/piney-world/src/map/field.rs, crates/piney-game/src/story.rs
---

# 216. Infection whole again on generated tables

The rebuild that began in 0207 left piney-game with 97 failing tests:

- The World, the fields and the streams could not load.
- Every read of main's data found nothing, because the run-time image has
  no executable.

That rebuild has now reached every system the Infection boot touches.
The suite passes again.

## The last pieces

The new `world` group holds these:

- **`markers`.** `markerEvTbl` is the town's event markers by script
  number. The event instruction `marker` read it from main and silently
  found nothing.
- **`map_labels`.** `mapmsg` gives the field map's "Overall Map" and
  "Default Map".
- **`book`.** `bookItemAddMsg` and the others give the story's
  announcement of a desktop item. Outbreak and Quarantine lead it with
  `#W`.
- **The navigation.** `navi_marks`, `navi_landmarks`, `navi_lines` and
  `navi_points` hold each town's landmarks and main lines, and the points
  the party walks to.
  - Each landmark list goes one past its town's last, as
    `ccNaviGetLandmarkPos` reads it.
  - The main lines are `unsigned char` runs, and they are main's
    (`mainLine26` and the others). Dun Loireag's and the shrine's tests
    stopped there.

The story's announcements take the server letters and item lines from
`fieldui` (`server_str`, `get_item_str`). `Execute`'s three literals
(`#Y`, `#B`, a space) are the reader's own constants. `World` keeps its
volume, and `NaviMap`, `TownPcs` and the battle's `TownMap` read by it.

## Where Infection stands

- piney-game: 123 passed, 0 failed, 59 ignored. The ignored ones are the
  long logs, shots and surveys.
- `story_survey` runs every main event with a start (3, 4, 10-31) under
  the autopilot for 4000 frames:
  - no panic;
  - no host call left at its default;
  - event 31 plays into the ending stream.
- The field UI's, the battle's, the effects' and the streams' harnesses
  pass against the game.

What Infection still reads from GCMN.PRG uses Infection's addresses, and
reading a PRG at run time is allowed. These are:

- `npcTbl` rows, with their constructors, through `NpcRow`;
- the walking PCs' `rtpc*` tables, the Grunty's and the breeder's
  cameras;
- the field map's icons, the town maps' icon positions, the dungeon
  map's floor names;
- the gimmick rows' files through `SpawnTables.img`;
- the enemies' weapon and dust blocks (`races.rs`'s 156 addresses);
- `charTbl` through DEMO.PRG.

Infection is right on them, but no later disc is.

**Still unknown:** Mutation's boot past the desktop. The GCMN readers
above are the known part of it. The rest is Mutation's own story (its
events and areas) and the functions the audit (`voldiff`) lists as
changed. The five older Python generators (`statics.py`,
`area_tables.py`, `dungeon_tables.py`, `field_tables.py`,
`sound_tables.py`) still write their per-volume Rust.
