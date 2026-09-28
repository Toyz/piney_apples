---
number: 134
title: near_marker outside the towns: event 17's Data Drain lesson plays through
date: 2026-09-26
area: script
files: crates/piney-world/src/field_world.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/event22.rs, docs/engine/field-game.md
---

# 134. near_marker outside the towns: event 17's Data Drain lesson plays through

Event 17 is where Mia teaches the Data Drain. In block 13, in area 18's
dungeon, she sends Kite toward the magic portal. The script walks
Kite part of the way (`pc_walk_pos`) and hands him back. Block 14 then
waits on `if near_marker 1 <= 250`.

The field and dungeon host (`AreaHost`) had no `player_distance`, so it
fell back to the host default. That default answers `f32::MAX`, so the
block could never open, and the lesson stopped at the portal however
close Kite walked. A scan of the host defaults against the Infection
scripts found it. The fields and dungeons still lack `marker`,
`load_overlay`, `set_frame_rate`, `affect`, `prev_room`,
`condition_effect` and `enemy_pp` as well. None of those is used in an
M1 event outside the towns: `overlay` and `frame_rate` are only used on
the desktop, in events 1, 4 and 31, and `near_marker` is the only one
that was missing ([field-game.md](../docs/engine/field-game.md#what-the-event-scripts-use)).

## The fix

`FieldWorld::player_distance` computes what `ccEvent::CheckOpen` (main
0x001a7d9c) does: `ccGetDist(ccTransPosW2P(pos), plw->posP)`, on the
ground. The player's frame wraps through the map's bounds
(`Combat::bounds`, `kite::w2p_pos`). The VM already looks the marker up
as the `evPos` of that number outside the towns. `AreaHost` answers
through it.

## The check

`event_17_walks_kite_to_the_portal` starts at event 17 at the dungeon's
entrance, with blocks 0-6 played. Kite walks to point 4 (floor 4,
room 1), where block 12 opens the portal. The walker goes up to the
portal, the fight follows, and the event plays blocks 7, 8, 10-17, then
goes back to Mac Anu.

The dungeon walker needed more on the way:

- **Magic portals.** A room with a live portal keeps its doors shut. The
  walker now walks up to the room's portal and presses X before taking
  the room's leg.
- **Waypoints.** The line-of-sight test now looks at knee height as well
  as waist height, and with Kite's width either side. A low rail around
  a caged statue had let the waist-height line pass over it.
- **Puts.** A put now lands in the leg's doorway, not 900 units inside
  or 1500 beyond. The walk then aims past the door: beyond it,
  `GotoNextRoom` never ran, and on the doorway Kite had stood still.
- **The goal room.** `leg_to` returns nothing once Kite is in it.

The event 25 and event 29 walks still pass.

## The town's item-get menu

The same scan covered each M1 block's instructions against the host that
runs it, with an untagged block taking the scene of the block before it.
It found one more gap. The town host had no `item_get_menu`, so
`item_add_menu` in a town played through without opening menu 29, and
menu 29 is what gives the item. Event 16 is the merchants' contest in
Mac Anu: its prize, the Book of Law (category 15, id 60), was never
given; the block still went on to mail 12, so the story did not stop.
Event 24's `item_add_menu 0 15 61 1`, in a town block too, lost its item
the same way. `FieldHost` now opens the menu as `AreaHost` does. The
scan's other hits are not gaps:

- `present 2` goes to `spc_present`;
- `mode 3` only asks a `ChangeRequest`;
- Kite's `item_add` of category 15 goes to the save;
- the desktop's `overlay` and `frame_rate` run in its own host.

`event_16_gives_the_contest_prize` starts event 16 logged in to Mac Anu
with blocks 0-5 played. Block 11 opens menu 29, the Book of Law goes
from 0 to 1, and nothing falls to a default. The tests' new
`story_in_mac_anu` (which `story_22_in_mac_anu` now calls) logs a desktop
start straight in.

**Still unknown:** whether the game's `ccGetDist` in `CheckOpen` takes
the wrapped frame for both points, as the town's port assumes. Also
unknown: whether a statue room's open doorway (no door dummy, floor 2,
room 1 of area 18's dungeon) should have Kite walk around the fenced
statue as he does here.
