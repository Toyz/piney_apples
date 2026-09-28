---
number: 44
title: The game from power-on: title, New Game, the desktop, and back
date: 2026-09-23
area: ui, script, test
files: crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/main.rs
---

# 44. The game from power-on: title, New Game, the desktop, and back

With the title of [[43]], the desktop of [[37]] and the scripts of [[41]],
`piney-game` now runs as the game does from power-on. It is the default
mode (`cargo run --release -p piney-game`).

## The session

`ccThMother` keeps the save and the event task across its modes and moves
between them on `ccGame::ChangeRequest`. `session::Session` does the same:
- **Boot.** A fresh save, and `ccStartEvent(1, 0)` on the scripts from the
  disc.
- **2, the title.** The card check, the logos and intro (requested and
  counted as played), and the menu.
- **3, the desktop.** New Game hands over the title's copy of the save and
  the event task. The desktop's setup passes then run one event frame per
  game frame: 59 frames for event 1's phase-0 pass, then 2. The desktop is
  built on the save they leave.
- **1, the reset.** START's Title Screen, with YES to both questions,
  asks for it. The save and the scripts start again, and the title skips
  its logos, as after `ResetOpFlg`.
- **4, The World.** `TOPPAGE.PRG` is not ported, so the desktop is entered
  again on the same save.

## Checked

The session's tests run it headless:
- **`power_on_to_the_desktop`.** The title, Up to NEWGAME and OK, the
  screen-out, the setup and the desktop. START then does nothing, because
  event 1 holds back every operation but the mailer, START included, until
  the first mails are read.
- **`title_screen_resets`.** The same, without the scripts. START, Title
  Screen, and YES twice: back on the title.

Two things the tests had to learn from the game:
- **The questions default to NO.** `ResetMenu` selects row 1, so each
  question needs Up before OK.
- **START comes late.** The desktop's opening runs about 400 frames before
  START is taken.

**Still unknown:**
- **`NewGame(0)`.** Its copy of `charTbl` into `spcParam` is not ported. No
  desktop member and nothing event 1 reads depends on it.
- **Before the desktop.** The setup screen and name entry during event 1's
  pass are black.
- **Not ported.** The title's music context, the PSS movies, the intro
  stream, and The World.
