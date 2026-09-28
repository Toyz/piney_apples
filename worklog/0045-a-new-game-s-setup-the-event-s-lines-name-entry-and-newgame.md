---
number: 45
title: A new game's setup: the event's lines, name entry, and NewGame's save
date: 2026-09-23
area: ui, save, decomp, test
files: crates/piney-demo/src/newgame.rs, crates/piney-desktop/src/name_entry.rs, crates/piney-desktop/src/setup.rs, crates/piney-game/src/desktop.rs, tools/test_demo_rs.py, tools/test_desktop_name_rs.py
---

# 45. A new game's setup: the event's lines, name entry, and NewGame's save

After [[44]] a new game went from the title to the desktop through about a
second of black. The desktop agent ported what the game shows there, the
title agent ported the save New Game makes, and I put them in the runtime's
setup stage.

## What a new game shows

Event 1's pass at phase 0 runs while `ccSetupDesktop` waits:
- **The screen is black.** There is no loading display.
- **Two lines.** Messages 1 and 2 each open in a window the event makes
  itself: a fresh layer 242 and `ccMessage`, with no dim. They wait for the
  confirm button.
- **Name entry.** `NameEntry_Control` opens with its own messages
  ("Registration process for "The World" is finished.", "Reconfirming your
  name and character name.") and then its keyboard, the face panel and the
  two names.
- **The desktop.** It is built on the save name entry leaves.

In the runtime the setup stage draws `SetupScreen` (the black and the
event's window) every frame. Name entry's frame replaces it while name entry
runs. The bridge routes each call:
- Place::Setup `message_open` / `message_check` / `message_close` go to the
  screen;
- `name_entry_start` / `name_entry_step` / `name_entry_end` go to
  `NameEntry`, which writes `plName` and `plRealName`;
- `SetupScreen::update` runs afterwards, so later lines use the new names.

`--skip-name` keeps the default names. The tests use it: blind presses on
the keyboard only type letters, and name entry has its own check.

## NewGame's save

`ccSaveData::NewGame(sw)` runs twice: `NewGame(1)` in `ccSetupDemo`, then
`NewGame(0)` on New Game. It does the following:
- copies the 18 `charTbl` rows from `DEMO.PRG` into `spcParam`;
- puts the names in `plName` and in lists outside the save;
- sets `playTime` to 0 and ORs the volume's default words in;
- fills the trade lists from the executable's tables.

With the parody flag set, Kite also starts at level 20.

`piney-demo` applies both calls itself. The tables are read from the disc at
run time. Until then the face panel's HP and SP bars drew red, because
`spcParam` was zero and the port follows the EE's divide-by-zero result.
Now Kite has maxHP 63 and maxSP 13, and the bars are green and blue.

## Checked

- **`NewGame`.** `test_demo_rs.py`'s `test_new_game` runs the game's own
  `NewGame` from its own `Init(1)`. The whole 0x8530-byte save and the copied
  names match after each call, parody off and on. A random save stays
  untouched where it should.
- **Name entry.** `test_desktop_name_rs.py` runs `NameEntry_Control` beside
  the port. The logic and the written names match every frame over random
  pad runs, and every draw call matches over a scripted pass through all its
  screens: 3 tests OK.
- **The menu task.** `test_desktop_menu_rs.py` runs the game's own
  `ccThDtMenu` beside the port.
- **The runtime.** The session and desktop tests press through the setup
  lines, which now take more than the 61 frames of [[44]]. The desktop crate
  has 20 unit and 16 integration tests.

**Still unknown:**
- **`save_va`.** `NewGame` stores the save's own EE address as character 0's
  name pointer. The port uses the eemu scratch address; the real heap
  address was not measured.
- **Not reached in the US build.** The kana, symbol and kanji grids of name
  entry.
- **Timing.** Real disc and card load times; the port loads at once.
