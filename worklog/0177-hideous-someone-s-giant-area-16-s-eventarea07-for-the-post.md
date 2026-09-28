---
number: 177
title: "Hideous Someone's Giant: area 16's EVENTAREA07 for the post-ending side event"
date: 2026-09-26
area: world
files: crates/piney-world/src/evarea07.rs, crates/piney-world/src/evarea.rs, crates/piney-world/src/evarea_b0.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/field_world/ride.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, docs/engine/evarea.md, docs/engine/field-walk.md, GAPS.md
---

# 177. Hideous Someone's Giant: area 16's EVENTAREA07 for the post-ending side event

GAPS listed the other `EVENTAREA` classes under "After Infection", but
one of them is Infection's. Side event 62, "SERVER1", opens once event 30
(the ending) is done. It gives area 16's words with a mail and Virus Core
T, then waits in the area's dungeon (event point 1, a magic portal of
code 214). Area 16's `EVENTAREA_INFO.model` is 1, so `GO(1)` builds
`EVENTAREA07`, not a generated field. The port gave it a generated field.

**Which classes Infection needs.** `GO(1)`'s table: 13 → 01, 15 → 02,
16 → 07, 43 → 03, 66 → 04, 67 → 05, 91 → 06, 9-12 → B8, 1-8 → B0. Of the
events Infection walks, only event 62 names one of the unported areas
(16). The others' areas appear only in later volumes' scripts.

**The class** (gcmn 0x00405a00-0x00406cdc, `docs/engine/evarea.md`,
"Area 16") follows `EVENTAREA02`'s shape with its own pieces:

- Block 0 has 8 static models (the arm), 2 animated objects (a floor
  and wind) and 4 background clumps: the sky and three cloud layers.
- Its light animation has one distant light.
- 25 `CLOUD`s of type 1 (already ported for Dun Loireag) and a
  `LENSFLARE` draw on `effLayer`.
- The background bobs: `DrawBG` steps a z by `fieldrand(25)` between 0 and
  350, turning at the ends, and draws the four clumps there.
- The start is `DMY_marker02` when `game.areaPrev` is 2 (back from the
  dungeon), else `DMY_marker01`.
- `Enter` from block 0 is `ChangeArea(2, 0)`.

`STATICMODEL::SetPos` and `STATICOBJECT::SetPos` were read: they copy the
position once, and the model's hit takes the translation.

**Block 1** names `se1_7_2`'s models through `se1_7_1`'s handle, and its
start, `DMY_marker03`, is in neither file. No Infection script reaches
it. The port reads block 1 from `se1_7_2` and keeps the start when the
marker is missing.

**The port.**

- `piney_world::evarea07::Giant`, held as `Place::Giant`, with arms beside
  `Arena`'s wherever the place is matched.
- `StorySprite` gained `rotate`, a `CLOUD`'s turn, which the game's
  effects now pass to `ccEff`.
- Checks: `the_giants_blocks` (both blocks' pieces, the hits
  `HIT_se1_7ob1hit` on `MDL_se1_7ob1_1` and `HIT_se1_7ob1_5hit`, the
  lights, the clouds, the door), `the_background_bobs`, and the session's
  `the_giants_door_to_its_dungeon_and_back`. That test arrives at
  `DMY_marker01` with area 16's event bank, walks up the arm into the
  dungeon (area 2, field 16), comes back with `GoField` to a new map, and
  stands at `DMY_marker02`.
- A shot (`--mode field:16`) shows the arm, the doorway, the sky and the
  clouds. `--press 200-1300:lup` walks into the dungeon, whose look is
  the data-noise one.

**Still unknown:** Nothing here was compared with a picture of the game.
Event 62 was not played through: the mail, the words, the portal at
event point 1 and the scene back to the town were not run end to end.
Whether the game's `GetChunkAdrsF` falls back to other loaded files for
block 1 was not read.
