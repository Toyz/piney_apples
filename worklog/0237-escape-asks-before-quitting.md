---
number: 237
title: Escape asks before quitting
date: 2026-09-27
area: ui
files: crates/piney-game/src/quit.rs, crates/piney-game/src/main.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/session.rs, crates/piney-desktop/src/dtmenu.rs
---

# 237. Escape asks before quitting

Asked for from play: Escape closed the window at once. It should ask
first, in the game's own confirmation window.

## The window

The window is the desktop menu's own (`ccDtMenu`), which the port already
has as `piney_desktop::dtmenu::DtMenu`. Its Reset (menu 6) is the game's
"are you sure" dialog:
- the information window (`xwindow`'s art, the `ef12x20` font);
- an OK / Cancel list with the hand cursor, Cancel first;
- the menu's sounds.

`DtMenu::open_quit` opens that menu on its own with the port's lines
instead of `resetMenuInfo`:
- "#YQuit#W and close the game."
- "#YData not saved will be lost.#W"

These are `QUIT_INFO`, marked as the port's text. OK sets `quit_ok`
instead of going on to the game's second question, and `closed()` tells
when the menu has shut. Nothing else in `DtMenu` changes; the desktop's
own instance never opens it.

## The app

`piney_game::quit::QuitPrompt` owns its own `DtMenu`. It is drawn from the
archive's window texture and the console's fonts, so it works in every
mode: title, desktop, top page, town, field, dungeon, streams.
- **Escape.** Escape opens the prompt; Escape again closes it. If there
  are no fonts to draw with, Escape quits as before.
- **The game waits.** The first frame steps the game once on a neutral
  pad, and that picture is kept, without its uploads. After that the game
  is not stepped. Each frame draws the kept picture with the window over
  it, and only the sound driver runs on.
- **The pad.** The window reads the live pad through the save's OK and
  Cancel buttons (`Mode::save_copy`: the mode's save, or on the title its
  demo's). Without a save it uses `ccSaveData::Init`'s Cross and Circle.
  Enter (START on the keyboard) also counts as OK.
- **The answer.** OK ends the event loop after that frame. Cancel waits
  for the window to close, then the game goes on.

`--quit N` opens the prompt at frame N of a `--shot`, and prints its
answer each frame. In Mac Anu:
- Up then Cross quits.
- Cross on Cancel goes back.
- START on Cancel goes back.

The window sits in the middle of the town, over the paused game.

**Still unknown:** The window is the desktop's; in a field the game's own
confirmations (Log out and the rest) use the field menu's windows, which
look the same but are not this code.
