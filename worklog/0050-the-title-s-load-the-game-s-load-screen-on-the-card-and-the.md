---
number: 50
title: The title's Load: the game's load screen on the card, and the loaded save handed to the desktop
date: 2026-09-23
area: ui, save, test
files: crates/piney-demo/src/dataload.rs, crates/piney-desktop/src/savesys.rs, crates/piney-desktop/src/card.rs, crates/piney-game/src/session.rs, tools/test_demo_rs.py
---

# 50. The title's Load: the game's load screen on the card, and the loaded save handed to the desktop

The user found DATALOAD on the title missing; in [[43]] it was a
stand-in. The title agent ported the game's load screen; I put it in the
runtime. The reference is "The load screen" in
[the title page](../docs/engine/title.md).

## What the game does

- **The screen.** `DataLoad_Control::Main_Control` runs `Data_Control`'s
  steps (the slot list, the save list with its panel, the question, the
  messages and YES/NO). It works over the same `ccSaveSys` as the boot's
  card check and the desktop's Data screen: the game keeps one from boot.
- **The title-only steps.** `ccSaveSys` has steps only the title uses:
  - `LoadSelectReq` and `LoadDataReq`;
  - `MainProccess` 6, which asks "Load this data?";
  - `MainProccess` 8, which reads the slot (`DataRead`), checks its 16-bit
    byte sum against the index record, and copies it into `saveData`,
    except three padding runs.
- **The hand-off.** `Main` returns 3, and `ccThDemo` then runs:
  - `saveData->LoadGame()`;
  - the same music fade and `ChangeRequest(3, 7)` as New Game.

  No event is started or converted. The event task started at boot keeps
  running, and every event flag it reads comes from the loaded save.
  `LoadGame` sets:
  - each party member's name pointers and the float from `charTbl`;
  - the vol1 words;
  - outside the save, the camera scheme, vibration, the display offset and
    `SetSoundEnv`.

## In the runtime

- **The card.** The title now gets the same card as the desktop:
  - `FilesCard` on the `--card` directory (`work/memcard/slot1` by
    default);
  - with `--no-card`, `NoCard`, and the boot asks whether to start without
    one, as the game does.
- **Load.** Loading hands the title's save to the desktop just as New Game
  does.
  - `SetSoundEnv`'s volumes go to the sound driver.
  - The Data screen starts at the card slot and save the load used
    (`set_card_position`).
  - The camera scheme, vibration and display offset have nothing to act on
    in the runtime yet.

## Checked

- **Against the game's code.** `test_data_load` in `tools/test_demo_rs.py`
  runs the game's `DataLoad_Control` and `ccSaveSys` in eemu beside the
  port. It covers 15 pad runs over cards kept as files:
  - saves, and empty, corrupt, missing and short slots;
  - a card without the save directory, and no card.

  Every frame matches: state, text draws, cursors and sounds. So does the
  whole save at the end. The runs reach "Data loaded." and each error.
- **`LoadGame`.** `test_load_game` compares `LoadGame` on random saves.
  All 18 tests in the file pass.
- **The runtime.** `load_from_the_card` in `piney-game` writes a save and
  its index record to a card directory. It loads the save through the
  title's screen (slot 1, the third save, YES, OK), and the desktop starts
  on it, play time and name included.
- **The other session tests.** They now boot with an empty card directory,
  because with no card the boot asks its question. `piney-game` has 7
  tests, `piney-demo` and `piney-desktop` pass, and clippy (also with
  `trace`) and fmt are clean.

**Still unknown:**
- **Outside the save.** `LoadGame`'s camera scheme, vibration and display
  offset are not applied; nothing in the runtime uses them yet.
- **The jingle.** `m_tempPN` is never initialised, so the "Data loaded."
  jingle (se 74) plays only if an earlier message in the same visit was
  acknowledged. The port starts it at 0.
- **The card position after a reset.** It starts at (0, 0) again. Whether
  the game's `ccSaveSys` keeps its position through a reset is not
  checked.
