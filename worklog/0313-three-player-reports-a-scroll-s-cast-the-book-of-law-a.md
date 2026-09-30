---
number: 313
title: Three player reports: a scroll's cast, the Book of Law, a story box's body
date: 2026-09-30
area: ui, battle, world
files: crates/piney-battle/src/world.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/gimmick.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/lib.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/menus/useitem.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/fairy_orb.rs, crates/piney-game/src/session/tests/boxes.rs, BUGS.md
---

# 313. Three player reports: a scroll's cast, the Book of Law, a story box's body

GitHub issues 10, 11 and 12 (Infection). Each case was played through the
session first, as the report describes it, and each test fails without its
fix. Addresses are INF gcmn unless marked.

## Issue 10: a scroll cast with no hold and no animation

A spell scroll (category 11) goes `ccUseItemRequest` (0x0057aa80) →
`ccItemSkillRequest(pw, tp, sid, 1)`, stype 2. `_ccSkillRequest` stores
`skillID`, `skillStatus` 9 and `targetChar = tp` (0x00572b80). Kite's
`AnimCtrl` (0x005995c0) then plays act 17-21 by the skill's type bits
(0x100 → 17) and holds him (`restraintSW`, `stopFlag`).

This is issue 8's cause: before 015c996 the port left `targetChar` as it
was. Kite with no target of his own (his last one fell, or nothing struck
him yet) dropped the skill before its act. The spell itself still landed,
through the item's own skill run, which is what the report describes. The
reporter's issue (15:46Z) predates that commit (16:31Z).

`a_scroll_on_a_foe_is_cast_by_kite`: `itemTblD` row 0 (skill 193) on the
goblin from PERSONAL, Kite's `targetChar` cleared until the use. With
015c996's line taken out the goblin is hurt and act 17 never comes; with
it he plays act 17, held, and the act ends.

## Issue 11: the Book of Law did nothing

Key item 60 (61 and 69 too) at 0x0057c02c: `OpenInfo` of `installWarnStr`'s
three lines (main 0x00377e80), 8 frames, `ccSeOnNote(196, 52)` (0x0057c0c4),
8 frames, until OK, `Close`. The port had these steps, but:
- in a town (where the book is got) `WorldMode` answered every use with
  no steps: no window, no sound. Books (category 12) and epitaphs used in
  a town did nothing either.
- in a field the menu handed `SeNote` to the world, which dropped it.

Fixes: `World::use_item` runs the use on the town party's scene (Kite's
stand-in the user; a book's stat lands in his record, stored at the frame's
end); `World::item_step` sets his `pauseSW`; the menu plays the note
(`Request::SeNote`). Tests: `the_book_of_law_warns_in_town` (before:
neither), `the_book_of_law_warns_in_a_field` (before: the window only),
`a_book_read_in_town_raises_the_stat` (before: the book spent, the stat
unchanged). The text comes from the build's `fieldui` group.

## Issue 12: boxes without collision

The screenshot: B2 of Δ area 18, Kite standing inside a treasure box.
`DUNGEON::SetItemBox` (0x005beb30) makes each `GIMMICKDATA` type-0 row a
box at (x, y, 250), `entRoot` 0, `land` -1. A box outside Kite's room can
not be landed then; `initObject` (0x00430ec0) switches its body on when he
enters (`SetHitSW(1)`, body still at z 250), then sets the box's `pos` on
the ground, not the body's. `boxMain` ends with `bodyHit.pos = pos`
(0x0045410c; `objectMain` 0x004543e0, `virusMain` 0x004545a4). The
game's list links the `ccCharHit` itself, so from that frame the body is
on the ground.

The port's list keeps copies, and nothing refreshed a box's. So every
story box outside the first room kept a body 250 up, which Kite's body
never meets. Random boxes and a first room's are made on the ground and
were fine. Fix: `World::hit_sync`, called by the boxes' frames
(`gimmick::body_follows`); Stage refreshes the list's copy.

- `area_18_s_boxes_stand_in_the_way` walks every room of area 18. Before:
  three boxes loose, on floors 1, 2 and 4.
- `kite_stops_at_a_story_box_on_b2` runs Kite at room 4's box on B2.
  Before: he came within 9.9 of its centre (radius 40).

**Still unknown:**
- The Ryu Books (273-280, `ccThBook` 0x0041a990, the `BOOK` class) are
  still not ported; the Book of Law does not need them.
- Whether the report's box was room 4's; the screenshot's room is not
  named. Other gimmicks whose owner moves the body were not surveyed.
