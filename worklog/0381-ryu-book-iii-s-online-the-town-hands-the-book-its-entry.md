---
number: 381
title: Ryu Book III's Online: the town hands the book its entry control's NPC list, so a person or walking PC in town today is online
date: 2026-10-05
area: ui, world, test
files: crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/ryu_book_online.rs, crates/piney-game/src/session/tests/fairy_orb.rs, crates/piney-game/src/session.rs, crates/piney-fieldui/examples/fieldui_probe.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md
---

# 381. Ryu Book III's Online: the town hands the book its entry control's NPC list, so a person or walking PC in town today is online

Issue #50 (ThreePendant, build 86c2571): "Town PCs don't show up as
Online in the Ryu Book, even when they are online". The screenshot is
Book III's sub-window for Stare: the name, "Trade Items" and three items,
and no "Online".

## What the game reads

`BOOK::SetCharInfo` (gcmn 0x0040bcd0) builds the book's 67 characters when
it opens:

- **Members 1-17.** `isOnline` is `ccSPC::CheckSpc(id)`.
- **People 30-65 and other players 66-79.** `isOnline` is 1 when an
  entry on `g_entCtrl`'s NPC list has the row as its `entParam.id`.
  The loops at 0x0040bf04 and 0x0040c0a4 walk the list: the count at
  +0x34, the head at +0x38, the next entry at +0x1c4, and each entry's
  `entParam.id` at +0x124. `ccEntryObj`'s DWARF puts `entParam` at
  +0x100, and `id` is +0x24 in it.
- **The walking PCs.** `ccSetRtownPC(row, k)` (0x00506640) stores the
  `npcTbl` row as that id (`sw $s1, 84($sp)`, the entry parameter at
  +0x30). Stare is row 67 (`TPC1`).
- **Hidden PCs.** A walking PC that hides beyond 4400 stays on the list.

So "online" means placed in this town today. `Disp03`'s sub-window draws
`bookOnLine` in colour 2 when `isOnline` is above 0 (`blez` at 0x00415360).

## The cause

The port's book read `World::npcs`, but no host ever filled it. The
town's `ui_world` left it empty, so no person or player was online. This
dates from the Ryu Books' first commit (27f3df6), and `git log -S` finds
no later change. It is not a regression. The walking PCs drawn per
arrival (d04b366, [[356]]) were placed, but never handed to the book.

## The fix

`ui_world` fills `npcs` with the code of every NPC on the town's entry
list (`World::all_npcs`: the merchants, dogs, Grunties, walking PCs and
the rest). Each code is the `npcTbl` row, the same as the game's
`entParam.id`.

## Checked

- **Against the game.** `tools/test_fieldui_rs.py` now gives the game a
  `g_entCtrl` with an NPC list, built from the scenario's `online` rows
  (the probe takes `online`). The new `test_book_3_online` meets three
  characters: person 35, Alicia 66 and Stare 67. The list holds 35, 67 and
  70 (70 never met). It opens each sub-window in turn. The game's
  `SetCharInfo` and `Disp03` draw "Online" for 35 and 67, the port the
  same, frame by frame. The whole harness passes (67 tests, one skipped),
  now that every scenario has a `g_entCtrl`.
- **The session.** `session/tests/ryu_book_online.rs`,
  `a_walking_pc_in_town_is_online_in_ryu_book_iii`, in Mac Anu:
  - one of the town's walking PCs (Tim, 79, this run) and one person not
    in town are the only people met;
  - Book III is read and the PC's sub-window opened;
  - the PC is online and "Online" is drawn, and the absent person is not
    online.
  Before the fix the PC's `isOnline` was 0.
- **Shots.** `ryu_book_online_shot` (ignored):
  - Before: /mnt/data/claude/scratch/issue50/before/book3-pc79.png, Tim
    with no mark, as in the issue.
  - After: /mnt/data/claude/scratch/issue50/after/book3-pc79.png, "Online"
    in red right of the name.
- The suite passes.

**Still unknown:** nothing. The members' mark is unchanged: the port
gives `CheckSpc`'s place in the registry, as the game does. The `blez`
skips slot 0, but that slot is always Kite (`ccSPC::Initialise`,
field-game.md), and Kite is not on the book's list. So every member who is
loaded shows "Online".
