---
number: 359
title: "Infection's whole story ends under the autopilot: event 22's cure boxes, a ghost Kite under god, board posts first"
date: 2026-10-02
area: test, script, world
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/boxes.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs
---

# 359. Infection's whole story ends under the autopilot: event 22's cure boxes, a ghost Kite under god, board posts first

[[358]] took `infection_whole_story` to event 21. It now runs INF's story
from its first start to event 31's end: 401,400 frames, 379 seconds
headless. Every event from 3 to 31 ends in order, the last on the staff
roll. Three more gaps were the pilot's; one was the harness's god.

## Event 22's cures are story boxes

Event 22's blocks 21-24 wait on Kite holding items 15/54-57 (Remedy,
Custom Remedy, True Remedy, First Remedy). No script gives them; no story
room's statue holds them (`ROOMDATA.itemID` names no `E05x`). They are in
area 31's dungeon (`D0171`), in four story boxes, one a floor.

```
D0171_gim  floor room  x      y      type  flag
row 3      0     9     6000   21562  0     0x450036  E054 Remedy
row 4      1     11    18750  12843  0     0x450037  E055 Custom Remedy
row 9      2     7     21046  18093  0     0x450038  E056 True Remedy
row 2      3     11    21000  24890  0     0x450039  E057 First Remedy
```

`SetItemBox` gives a type-0 row with a `flag` that item
(`box_item_code(flag)`). The flag is the item's code as `0x45_0000 | id`
(`E` and the id; `itemTblE`'s `code` for 54 is the text `E054`).

The pilot gets them:
- **`Want::Item(id)`.** `story_wants` gives it for a block short only of
  an important item (category 15, Kite).
- **`item_spot`.** In a story dungeon it finds the box rows holding a
  wanted item, nearest floor at or below Kite's first (the walker does
  not climb). Else a room whose row gives its statue an item, else the
  statue room (`DungeonArea::statue_room`, the room built from the type's
  `symroom` model).
- **`open_item`.** In the room it walks to the unopened gimmick nearest the
  row's x, y (`FieldWorld::unopened_gimmick_near`: switched on, still on
  the command list) and gives OK. The box's menus are `story_player`'s. A
  room counts as done only after 300 frames there with nothing to open,
  since the boxes switch on with the room.

`FieldWorld::unopened_statue` (a statue not yet opened) is now its own
function; `no_active_object` uses it.

## A ghost walks through boxes; god left Kite one

The pilot reached the First Remedy's box and stood in it. Kite came
within 0.63 of the box's centre (radius 40). The box's body was in the
list where it stood, and Kite's was not. Traced room by room, Kite's body
went at room (0, 10) and in every room after: there `hp` reached 0 and his
`cond[DEAD]` went 2, 3, 4. With dead 4, `ccSpcChar::HitCheck` takes the
body off the list (`kite_hit_check`), as in the game: a ghost goes
through.

The god console heals the party at the end of each frame, so a hit worth
more than Kite's HP still fells him. `FieldWorld::revive_party` got the
members up but not Kite. It now revives Kite too (affect 20, by a member
still standing). `kite_stops_at_area_31_s_cure_box` (with
`kite_stops_at_a_story_box_on_b2`, now sharing `kite_stops_at_the_story_box`
and `in_story_dungeon`) walks to floor 3, room 11 under god and runs at the
box: he stops against it. The game's rule is untouched; only the harness's
god changed.

## The other two

- **The cure line.** `info_lines` (the window [[355]] gave `piros_colour`'s
  line) is a window every pilot now answers, as it does `message_open`
  and `announce`. Without it the pilot waited on "Piros used the Remedy"
  for ever.
- **Event 23's posts.** Block 1 waits on four new posts read on the board,
  and block 2 on the mail they bring, on the desktop. The top page pilot
  left for the desktop first and never opened the board. New posts (and
  a post to write) now come before leaving.

## Checked

`infection_whole_story` (ignored, a diagnostic: `--ignored --nocapture`)
passes, events 3, 4 and 10-31 each ending once. The game suite (220) and
the world suite (92) pass.

**Still unknown:**
- [[358]]'s field-28 hold (`in_battle` 1 with no fight) did not come up
  again and was not explained.
- The run plays with god, the infection held at 0 and cores given for the
  hacks; how the story goes without them is not measured.
