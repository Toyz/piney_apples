---
number: 275
title: Field 13's talks: area n for the numbered maps, field markers, and the fellows' delete
date: 2026-09-28
area: script, world, volumes, test
files: crates/piney-world/src/party.rs, crates/piney-world/src/area.rs, crates/piney-world/src/field_world.rs, crates/piney-event/src/vm/exec.rs, crates/piney-game/src/session.rs
---

# 275. Field 13's talks: area n for the numbered maps, field markers, and the fellows' delete

Mutation's whole run ([[273]]) ended events 101 to 114 and then stood in
field 13 for event 115, talking to no one. Four things lay behind it, three
in the port and one in the pilot. A play-test report about Infection turned
up a fifth, in the same code: Mia and Elk stayed in Mac Anu after their
scene.

## `area n` for 1-13

Block 11 runs `area 13` and then `scene area=1 town=2 field=13`.
`ccEvAreaCodeAdd` (instruction 118) for `n` from 1 to 13 calls
`WORLD_MAN::Quit()` and sets `eventAreaNumber = n`. It generates nothing:
these story areas have no words (docs/engine/area-words.md). `Quit`
(INF main 0x0019c740) frees the map's layers and leaves the seeds and
words alone.

The port did nothing for these numbers, so `WORLD_MAN` stayed area 52's.
Its `EVENTAREA_INFO.model` is 0, and `story_map::build` made field 13 a
plain field of area 52's words instead of `EVENTAREA01`. Now
`WorldMan::with_event_number` gives the same `WORLD_MAN` the new number:
- `model`, `flag` (the hack flag), `protect` and `enemy` come from row n.
- The session uses it (`area::ev_area_number`) when `ev_area` answers
  `Number`.

## `marker_pos` in a field

Block 14 sets six event positions from markers: `marker_pos marker posnum`
(`ccEvMarker2Pos`, INF main 0x001b1aa4). On an event map (`game+0x24` set)
the marker is `markerEvTbl[marker]`, the town's names
(`DMY_marker_evNN`), as a dummy of the map's stream
(`WORLD_MAN` +0x444, stream +0x1a4). `SetEventPos` then stores the dummy's
position (+0x10) and its rotation z (+0x28) under the current floor and
block. A plain field reads `WORLD_MAN` +0x438 instead; that path is not
ported.

The field host answered no `marker`, so the VM faulted on `marker_pos`.
The pieces added:
- `StoryMap::file`, which only `EVENTAREA01` fills for now.
- `event::marker_in`, the town's lookup on any file.
- `FieldWorld::marker`, which the field host now answers `marker` with.

## Live event positions

`npc_put_marker` and `npc_walk_marker` outside the towns read eventMng's 16
positions (`evPos`, +0x1c0). The field read a copy taken at the area's
entry set-up, before block 14 set them, so every put was "not done" and
the six NPCs stood at the origin.

A new `Host::event_positions` is called by the VM after each `set_pos` and
`marker_pos`. The field host writes the positions into
`FieldWorld::set_event_positions`.

## The pilot's command target

The pilot waited for the command target to be `(Npc, code)`. An event
NPC's stand-in is on the entry control's object list, so it is targeted as
`(Gimmick, scene index)`. The pilot now maps that index to the NPC's code
through `Combat::npcs`.

`event_115_field_13_talks_to_six` holds all of this:
- field 13 is `EVENTAREA01`;
- the six NPCs are at their markers;
- the six talk statuses (`eventStatus[10..15]`) are 1;
- block 21 goes to the arena, field 3.

## The fellows' delete: Mia and Elk

Infection's event 13 (MG0340) enters Mia and Elk (`entry 2 1`,
`entry 2 10`) for a scene in Mac Anu. It ends with `remove -1 -1` (which
deletes entry objects only) and `scene -2`. The game drops them at the
change of scene: each fellow task's delete (`ccThFellow01Delete` ..
`17Delete`, INF gcmn 0x0041ec20 ..) calls `DelSpc(listNum)` when the
character's own `partyFlag` (+0xe0 bits 14-16) is not 1.

Mutation's (MUT gcmn 0x004327b0) also keeps a character whose `recallFlag`
(+0xe1 bit 4) is set. Outbreak's and Quarantine's were not matched by name
and are taken to be Mutation's.

The port kept them registered and built them again after the reload.
`Spcs::delete_fellows` now runs over each world's characters as a scene is
left (`World::delete_fellows`, `FieldWorld::delete_fellows`, from
`Session::change_scene`). `event_13_leaves_mia_and_elk_out` fails without
it. `event_11_church_ends_with_blackrose_out` expected BlackRose still
registered (`partyFlag` 0) back in Mac Anu. Her own flag is -1 by then,
so the game frees her slot, and the test now says so.

## The story areas' enemies

A community note says a story area takes its enemies from its server's
first list (`enemyList00` for Delta's event fields). The executables say
column 6 on all four volumes:
- `ccRegisterDifficultyEnemy` passes type 6 when `game+0x24 > 0`.
- `ccRegisterEnemyList(server, type, rank)` indexes
  `ccEnemyListInfo[server][type]`.
- `ccEnemyListInfo` points row s, column t at `enemyList<s><t>` in each
  volume.

The port follows the code. Whether the note counts the lists differently
is open.

**Still unknown:**
- `marker_pos` on a plain field (`WORLD_MAN` +0x438's stream) is not
  ported, and no event map but `EVENTAREA01` answers `StoryMap::file`
  yet.
- Outbreak's and Quarantine's fellow deletes were not compared (no name
  was carried to them).
- What the community note's `enemyList00` refers to: the executables give
  story areas column 6 (`enemyList06` on Delta).
