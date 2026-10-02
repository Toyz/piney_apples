---
number: 318
title: "A Ryu Book was never opened: its use was a bare request, and the Key Items refuse books outside towns"
date: 2026-10-01
area: ui
files: crates/piney-fieldui/src/menus/keyitem.rs, crates/piney-fieldui/src/menus/useitem.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/fairy_orb.rs
supersedes: 317
---

# 318. A Ryu Book was never opened: its use was a bare request, and the Key Items refuse books outside towns

[[317]] put `ccThBook` in, but no book ever opened. A test reading Ryu Book I
(key item 273) from PERSONAL's Key Items showed why: the menu went to its
proccess 2 and straight back, with no book task.

## The cause

The Key Items menu's proccess 2 pushed `Request::UseItem` for a book by
itself, then ran the menu's fade-back tail. A use is answered only while
the menu task waits in one (`useitem::call`, [[292]]). So this request fell
through unanswered: `BookStart`, `CloseChat` and `WaitBook` never ran.

It is the same trap the epitaphs fell into in [[292]], which left the
Grunty Flute and the Ryu Books on "their own paths".

## The game's order

`ImportantItemMenu` (INF gcmn 0x0052f980) calls `ccUseItemRequest` at
0x0052ffc8 for the item. For a book that call runs the whole book: the
cover stream, the pages, and `WaitBook` until it is shut. Only afterwards
does the code test for 273-280 again (0x00530088) and run
`EntryFade(1, black, black, ...)`, `CloseInstant`, `ccWakeAllThread`,
`Disp` and the breaths (0x005300cc on).

The port now calls the use through `useitem::call` with a new
`Resume::Book`. Its resume, `keyitem::book_closed`, is that tail.

## Outside towns

`ImportantItemMenu` refuses a Ryu Book with help line 8 when `game.area`
is not 0 (the port's `refuse(m, 8)`). So a book is read only in a Root
Town, and its cover stream only ever plays there. [[317]] also answered
`Request::BookStream` in fields. That path cannot be reached, and it is
removed: the field answers nothing for it.

## Tests

- `a_ryu_book_opens_in_town`: Ryu Book I in Mac Anu. The cover stream plays
  over the town and the pages open. It fails without the fix, with neither.
- `a_ryu_book_is_refused_in_a_field`: no cover and no pages.

The books' page (3) of the Key Items exists only with the bracelet
(`PLCOL`). The tests set it.

**Still unknown:**
- Books 4-8's pages and rewards (`type` 3-7) are not ported ([[317]]).
- The cover's palette from the second book on is not swapped.
- No test turns a book's pages or takes a reward yet.
