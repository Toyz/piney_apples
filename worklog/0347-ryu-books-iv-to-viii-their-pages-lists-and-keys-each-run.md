---
number: 347
title: "Ryu Books IV to VIII: their pages, lists and keys, each run against the game"
date: 2026-10-02
area: ui
files: crates/piney-fieldui/src/book/pages.rs, crates/piney-fieldui/src/book/mod.rs, tools/test_fieldui_rs.py, crates/piney-game/src/session/tests/fairy_orb.rs, docs/engine/field-ui.md
resolves: 317
---

# 347. Ryu Books IV to VIII: their pages, lists and keys, each run against the game

[[317]] ported Books I-III and left IV-VIII's pages: their `DispNN`,
`CheckItemGetNN`, and `PadControl04`, `05` and `08`. The rewards
(`GetBookNNItem`) were already in. A player opened Book IV (issue #23,
and the user's own run): an empty window, and the game stuck.
`BOOK::PadControl` closes the book on cancel only once `wait` is 0, and
`wait` (15 at the start) is counted down only by the page's own keys,
which IV-VIII did not have.

All five are now ported (INF gcmn, the reference in
[field-ui.md](../docs/engine/field-ui.md#the-ryu-books-book-gcmn-bookcpp)).
`tools/test_fieldui_rs.py` gains `test_book_4` to `test_book_8`. Each runs
the game's own `ccThBook` in eemu beside the port, frame by frame, over
every sprite packet, kanji row, message state and sound. Every one
passes, along with the harness's other 60 tests. A one-pixel change to
Book VI's count column fails its test at once.

What the code showed, where it was not what one would guess:

- **Book IV** (`Disp04` 0x00415790): the list's search for the next enemy
  slain runs to 304, one past the 303 rows. Row 303's kill count is then
  `BOOK.checkValue[2]`, which `Draw` zeroes each frame, so it is never
  taken. The sub-window shows the enemy's `enemyTbl` row: `base.level`,
  `max_hp` (10000 or more spelled out of `cheatHpStr`), `max_sp`, the
  spells `mag0`/`mag1` (`skill` +0x70, +0x74), the three `item`s, and a
  weakness from the protect points (`max_pp` -1, else `p_def_pp` against
  `m_def_pp`). Its help line names where one was last slain:
  `enemyKillArea` (+0x69f0, server and three words) between `#B` and `#W`.
  The sub-window's column starts 40 higher than the list, which a first
  try got wrong and the harness caught.
- **Book V** (`Disp05`, `PadControl05`): off the list the cursor moves with
  the *held* buttons (`ccSys` +0x2cc), not the pushed ones, `keyWait`
  letting a hold through every third frame. The harness's first scenario
  pushed DOWN and the game did not move; the emulator's `BOOK` fields
  showed every gate open, which led to the read of +0x2cc. Book V's help
  sets `BookHelp50`'s second and third strings to NULL in place.
- **Book VI, VII** (`Disp06`, `Disp07`): counter rows like Book II's, the
  count at a fixed 250 or 182. VII's first two rows say "encountered"
  (`bookEncountMsg`), and its third row stands a line lower.
- **Book VIII** (`Disp08`, `PadControl08`): a menu with the Grunties
  (`ccGetNpcParam(145 + i)`) and their food (`gimmickTbl[22 + i]`); each
  page resizes the window by writing the global `BookOfs`. In the food
  list's help the count's `sprintf` is overwritten by the `strcpy` after
  it, so the line is the food's name and `bookPuchiFood`. On the food page
  the total moves down by repeat and up by the held buttons, and the
  list's "none above" sets `keyWait` where IV's and V's do not.

`the_later_ryu_books_open_and_close` opens each of IV-VIII in Mac Anu
through PERSONAL's Key Items and closes it with cancel.

**Still unknown:** the one-frame difference in the message window's
`mode` just after a book closes (frame 471 of `test_book_8` when run past
its closing frame; Books I-VII's scenarios also stop on that frame); the
cover's palette from the second book on ([[317]], [[318]]).
