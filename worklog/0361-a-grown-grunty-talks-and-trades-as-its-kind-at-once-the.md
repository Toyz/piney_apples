---
number: 361
title: "A grown Grunty talks and trades as its kind at once: the menus read its row as it is now"
date: 2026-10-03
area: ui, world
files: crates/piney-fieldui/src/talk.rs, crates/piney-game/src/session/tests/dun_loireag.rs, docs/engine/field-ui.md
---

# 361. A grown Grunty talks and trades as its kind at once: the menus read its row as it is now

Issue #33 (build 76f4cb6): in Dun Loireag, a Grunty that has just grown
up opens the grown one's menu (`OtonainuMenu`, 45: Talk, Trade). The
target tag reads "Noble Grunty", but the window speaks as "Little Grunty"
with a young one's line, and Trade has nothing until the town is entered
again.

## Cause

A Grunty's base row moves as it grows. Row 154 is Little Grunty; level 1
is 155 (`evoActChibi2`); the Kid is 156-157. `adultSetup` then sets the
grown kind's row, 145 + `local_id` (`Grunty::set_base`). The game's menus
read `cmndTarget->base` (or `cmndTargetPrev->base`) at the moment they
need it, so they see the new row straight away.

The port's town gives the menus a `TalkTarget` whose `Speaker::Npc` is the
NPC's code, and `talk::base_of` reads the `npcTbl` row from it. For a
Grunty the code is the row it was made with, kept as its identity for
targeting. So after it grew up the menus still read the young row:
- its name and its message table (`chibiMsg` in place of
  `kusoKizokuMsg`);
- for Trade, the young rows' type 0x01000000, which has none of
  `trader`'s bits. The menu then has nothing to offer. The grown rows'
  0x02000000 leads to `npcTradeList[id - 109]`.

Entering the town again places the grown one through `setDog` with its
own row, which is why trading worked "later".

## Fix

`talk::base_of` now takes a Grunty's row from the world's per-frame copy
(`breeder::Grunty::row`, `g.row.id`) whenever its handle matches. Every
other NPC still reads its code. The code itself is unchanged, since the
targeting keys on it. The same holds while the young one grows (the Kid's
name at level 2-3).

## Test

`a_grown_grunty_talks_and_trades_as_its_kind` (piney-game) sets Dun
Loireag's growth record to level 3, size 28 of the 30 a grown one needs,
with no grown kind yet. It feeds one of the first food, and the Grunty
grows up into row 145 (Noble Grunty) and stays, a new kind. Spoken to
again, `OtonainuMenu`'s greeting is in Noble Grunty's name, and Trade
opens the trade menu with offers (not process 30, "nothing to trade").
Without the fix the greeting comes in "Grunty the Kid"'s name.

**Still unknown:** nothing.
