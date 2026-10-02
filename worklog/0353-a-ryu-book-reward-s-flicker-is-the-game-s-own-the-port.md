---
number: 353
title: "A Ryu Book reward's flicker is the game's own; the port draws the book once a frame"
date: 2026-10-02
area: ui
files: crates/piney-fieldui/src/book/mod.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-fieldui/tests/fieldui.rs, docs/engine/field-ui.md
---

# 353. A Ryu Book reward's flicker is the game's own; the port draws the book once a frame

The user saw a Ryu Book "glitch out and in" for an instant when it gives
a reward. The cause is the game's own loop. `ccThBook` (INF gcmn
0x0041a990) runs `Breath; BOOK::Draw; BOOK::CheckItemGet;` and then
`PadControl` unless a reward ran. A reward's windows breathe inside
`CheckItemGet`, each `Draw; Breath` until `Check(0)` answers. So:

- the frame a reward starts draws the book twice: the loop's `Draw`, then
  the first window's;
- the frame one ends draws no book: the last `Check` answers, the window
  closes, the reward is added, and the loop goes on to its next `Breath`.

Measured with `tools/test_fieldui_rs.py`'s game run (Book I, 15 areas
visited, the scenario of `test_book_1`). Frame 36 sends 36 `bookWin` and 2
`bookBg` packets, against 18 and 1 on the frames around it. Frame 102
sends none of either. The port matched that exactly; that is why the
harness passed. On screen the book's translucent background flashes
darker on the first frame and vanishes on the last.

Smoothed, by the user's choice: `Book::frame` draws the book exactly once
a frame. It keeps the first frame's first draw (skipping a window step's
second) and adds one on the frame a reward ends. Over a whole reward the
draw calls are the game's count, one taken away and one added, so the
book's state keeps the game's timing.

`Book::as_the_game` (on the book task, `Task::as_the_game`) restores the
game's draws. `fieldui_probe` sets it, and the 65 harness tests still
match the game frame for frame.

`a_ryu_book_reward_draws_the_book_once_a_frame` (piney-fieldui) opens
Book I with its reward due, dismisses the windows, and requires one
background on every frame from the book's first draw. With the game's
draws it fails at exactly frames 35 (2) and 102 (0). The book's first
frame (`BOOK::BOOK`, drawn after the loop's first breath) is the game's
appearance, not a flicker, and is left out.

**Still unknown:** nothing.
