---
number: 496
title: A story dungeon's fieldrand after Generate is the game's: each floor's draw and SetAllGim's story rolls at construction, and the gimPos headings ported
date: 2026-10-25
area: world
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/examples/dungeon_probe.rs, tools/test_dungeon_rt.py, docs/engine/dungeon.md
resolves: 285.31
started: 2026-10-25T20:58:11-06:00
took: 20m
---

# 496. A story dungeon's fieldrand after Generate is the game's: each floor's draw and SetAllGim's story rolls at construction, and the gimPos headings ported

Open question [[157]]: the port's `fieldrand` after a story dungeon's
`Generate` was not the game's. tools/test_dungeon_rt.py worked around it
by copying the game's state into the probe after `make`.

**What the game draws** (INF gcmn `DUNGEON::Generate` 0x005c12a0):
- A story dungeon (an edit table) runs `MakeRealMap` ten times, with no
  draws.
- Then, as a random dungeon does, it draws `fieldrand(5)` before each
  floor's `MakeFloor`: ten floors, or one for a lake.
- The story `MakeFloor` (0x005c0148) passes every row to
  `MakeRoom(ROOMDATA *)` (0x005bcbb0), which runs `SetAllGim` (0x005bb340)
  on the room's model.
- There, an item box asks `CheckEntryItemBox` (0x005cf800). It keeps no
  box, and draws nothing, where a `GIMMICKDATA` row of type 0-4 or a
  `ROOMDATA` row with an event (+0x20) names the room; otherwise it keeps
  one on `fieldrand(100) >= 20`.
- A magic circle asks `CheckEntryCircle` (0x005cf690): the same test for
  types 1-4, with no draw.
- The special objects draw as in a random room.
The port made these rolls only when `gim_slots` ran, at `EntryGimmick`,
and never drew the ten `fieldrand(5)`s.

**The fix.**
- `DungeonArea::story_rolls` makes the draws at construction, in
  `Generate`'s order: each floor's `fieldrand(5)`, then each built room's
  dummies in `MakeRoom`'s order, rolled by `story_keep`. It stores them as
  the room's `rolls`, as a random room's are, so `gim_slots` draws
  nothing.
- With `gimPos` now compared, its headings differed. They are ported from
  `SetAllGim` (`gim_heading`); `p` is the point 150 down the dummy's y axis:
  - a box: `atan2` in doubles of (x, -y) of `p` less the dummy's place
    (0x005bb5c0);
  - a magic circle: `atan2f(p.x, p.y)` of `p` itself, which overwrites
    the first heading it takes (0x005bb8a4);
  - the statue, fountain and special objects: `atan2f` of the x axis
    (0x005bbb74).

**Checked.** tools/test_dungeon_rt.py:
- `test_dungeons` compares every dungeon's dressing and `fieldrand` after
  `SetRoom(0, 0)` with no copying for story dungeons, and every
  `gimPos` slot (773: place, heading, floor, room, type) through the
  probe's new `gims`;
- `test_special_rooms` checks the 15 story dungeons' `fieldrand` and
  `gimPos` after `Generate`;
- `test_leaves` no longer syncs `fieldrand`.
All 12 tests pass. So do piney-world's tests and piney-game's suite (335).

Docs: dungeon.md (`gimPos`, its headings, the story rolls, the checks; the
open line removed), UNKNOWNS.md.

**Still unknown:** nothing
