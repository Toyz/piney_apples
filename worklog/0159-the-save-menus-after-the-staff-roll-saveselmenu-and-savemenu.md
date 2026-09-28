---
number: 159
title: "The save menus after the staff roll: SaveSelMenu and SaveMenu"
date: 2026-09-26
area: ui
files: crates/piney-desktop/src/dtmenu/save.rs, crates/piney-desktop/src/dtmenu.rs, crates/piney-desktop/src/lib.rs, crates/piney-desktop/src/data.rs, crates/piney-desktop/examples/desktop_probe.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/session/tests/ending_save.rs, tools/test_desktop_savemenu_rs.py, crates/piney-demo/examples/demo_probe.rs, docs/engine/desktop.md
---

# 159. The save menus after the staff roll: SaveSelMenu and SaveMenu

The ending's `staff_roll` instruction opens `ccDtMenu` menu 8 after the
roll and waits for the menus to close. The port had neither menu 8 nor
menu 9: both closed at once, so the ending never saved the clear data.

## What the game does

After the roll the instruction waits 30 frames, calls `SetFrameRate(2)`
and, unless parody mode is on, sets `openReqNum` 8.

- **Menu 8, `SaveSelMenu`** (0x0016ef90) asks "You now have the Data Flag
  for .hack//INFECTION. Save?" in `ccMsg`'s information window over the
  OK / Cancel dialog. OK opens menu 9; Cancel closes.
- **Menu 9, `SaveMenu`** (0x0016f2c0) drives `ccSaveSys` as the Data
  screen does (`StartReq`, `SlotSelectReq`, `LoadInfoReq`,
  `SaveSelectReq`, `SaveDataReq`, `NextProccess`, `EndReq`). It draws the
  card slots and the twelve files in the menu window (`SaveMenuDisp`
  0x0016fe70). The task's messages and questions go through `ccMsg`'s
  `DispInfo`, with the dialog for a question. The plain lines ("Select
  MEMORY CARD slot.", "Select a place to save.") go through `DispMsg`.
- **The file list** shows each file's number, the name, "LV" and the
  level, and the play time, in yellow for clear data and red for parody.

## Three surprises

- **Registers left behind.** In `SaveSelMenu` the `InitCursol` after OK
  has a nop in its delay slot, so it gets the 1 left in `$a1` from the
  select test, not a 0. `SaveMenu`'s `StartReq` gets 12, left by
  `ccThDtMenu`'s `InitCursol(1)` call when the menu changed. `ccSaveSys`
  tests `operate` only against 1 and 2, so 12 saves as the Data screen's
  3 does.
- **The frame rate.** The menus run at frame rate 2. `DispSelectCursol`
  reads the rate five times: the bars step by `/(3 - rate)`, the trailing
  wings ease by `(10 - k)(3 - rate)`, and the lead wing by `4.5 / rate`.
  The port's cursor had rate 1 built in.
- **When the card task runs.** `ccThSaveSys` (priority 20) runs before
  the menu task (33). Its first run only breathes, so the first
  `MainProccess` comes two frames after `StartReq`.

## The port

- `crates/piney-desktop/src/dtmenu/save.rs` holds the two menus,
  `SaveMenuDisp`, the texts (`clearFlagStr`, `recordMenuStr`,
  `saveSysMsg`'s lines) and `dec2str`.
- `dtmenu.rs` draws the four name kanji and `menuFont` strings in
  colour. Its select cursor now takes the frame rate.
- `MenuCtx` carries the desktop's `SaveSys`, and `Desktop::step` runs its
  task before the menu, from the second frame on.
- `Desktop` keeps `ccSystem`'s frame rate. piney-game's host passes on a
  `SetFrameRate` while the desktop sleeps (the staff roll's), so the
  menus and the frame pacing follow it.
- desktop_probe's `savemenu` runs the menus over the same simulated card
  as `datascen`.

`demo_probe` had stopped building after `convert::mmat` gained `div_z`
and `depth_fog`. It now passes `DIV_Z` and no fog, and
`tools/test_demo_rs.py` passes again.

## Checked

`tools/test_desktop_savemenu_rs.py` runs the game's `ccThDtMenu` natively
in eemu (as `tools/test_desktop_menu_rs.py` does), with `ccSaveSys`
constructed and its `MainProccess` called between the menu's frames. The
card is `tools/test_desktop_data_rs.py`'s. There are 16 scenarios:
- saving to new and used files, and the clear data warning;
- NO answers, both card slots, no card, not a PS2 card;
- unformatted (NO, then formatted), a failing format, no directory;
- a full card, failing slot and index writes;
- frame rate 2 twice, and 160 random pads over two prepared cards.

Every frame agrees: the menu's and the task's state, the sounds, the
`DispInfo` and `DispMsg` lines, every window and kanji packet, and the
font strings with their colours. So do the card files at the end. The runs
reach all 25 results the test asks for. The first mismatch it found was the
`InitCursol` 1.

piney-game's `ending_saves_the_clear_data` plays story:31 through the
ending and its staff roll (menu 8 at frame 12,587, at rate 2). It saves
to an empty card: the directory is made, then the first file written.
Read back, the index's first record is used, with clear flag 1 and the
save's level, and the file holds clear flag 1. The desktop wakes after.
`save_menu_shots` (ignored) writes the menus to
`/mnt/data/claude/scratch/savemenus/shots`.

**Still unknown:** the busy operations ("Saving....", `ccMsg +0x24` 3
with `DispPageCursol`) are never on screen in the port, since its card
finishes within the menu's 10-frame wait. How long a console card takes
is not known. `ccMessage`'s drawing at rate 2 (its button's pulse) is not
compared with the game's here. The harness hooks `DispInfo` and `DispMsg`.
