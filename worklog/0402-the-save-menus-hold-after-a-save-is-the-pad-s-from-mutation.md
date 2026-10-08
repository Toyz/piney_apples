---
number: 402
title: The save menus' hold after a save is the pad's: from Mutation on it is dropped with no controller in port 1, and the field harnesses now have one as the port does
date: 2026-10-08
area: ui, volumes, test
files: crates/piney-input/src/lib.rs, crates/piney-desktop/src/savesys.rs, crates/piney-desktop/src/dtmenu/save.rs, crates/piney-desktop/examples/desktop_probe.rs, crates/piney-fieldui/src/menus/record.rs, crates/piney-fieldui/examples/fieldui_probe.rs, tools/test_fieldui_rs.py, tools/test_fieldui_shop_rs.py, tools/test_desktop_savemenu_rs.py, docs/engine/field-ui.md, docs/engine/desktop.md, docs/engine/overview.md, UNKNOWNS.md
---

# 402. The save menus' hold after a save is the pad's: from Mutation on it is dropped with no controller in port 1, and the field harnesses now have one as the port does

[[392]] left three Recorder cases of tools/test_fieldui_shop_rs.py
failing on Mutation (`test_new_directory`, `test_save_new`,
`test_save_overwrite`). After "Data saved." the game's `waitCount` was 0
where the port's was 10.

## What the game does

From Mutation on, `RecordMenu` sets the hold to 10 after sound 74 as
Infection does. It then sets it back to 0 unless `game+0x7c` or
`ccSys+0x26c` is set (MUT gcmn 0x00577a8c, 0x00577af0, 0x00577b54).
Outbreak (0x0057376c) and Quarantine (0x004660ac) do the same. So does
the desktop's `SaveMenu` (MUT main 0x0016fb1c, OUT 0x0016f084, QUA
0x0016ef14). Infection has neither test.

- **`ccSys+0x26c`** is `pad[0].status`. `ccPad` is 0x74 bytes at +0x268,
  and +0x2d0, START's push in the same code, is `pad[0].push` there too.
  `ccPad::Read` stores `scePadGetState` in it each frame (INF main
  0x00102d80). That is 0 with no controller in port 1, and 2 or 6 once one
  is found.
- **`game+0x7c`** is `ccGame.pauseFlag` (Infection's DWARF). `ccThMother`
  pauses the game while `status` is 0 in a playing mode and `pauseFlag` is
  0 (INF main 0x00167d28, MUT 0x00167ea8). The screen fades, "P A U S E"
  is drawn, and with a controller back "START button: Retun to game"
  (Mutation: "Return") waits for START.
- **Who sets `pauseFlag`.** A game over (`ccThGameCtrl`, MUT gcmn
  0x0053598c) sets it and closes every menu. The event VM sets it around
  its sleep of every task (MUT main 0x001c7084). Mutation's
  `ccSaveSys::MainProccess` sets it around each card access and clears it
  in the same call (MUT main 0x00171f7c-0x0017528c). So it is 0 whenever
  either save menu runs, and the hold comes down to the controller.

## Cause

Not a port bug. The port's controller is always there (the keyboard is
one), so the hold is Infection's 10, as the game's is with a pad in. The
field harnesses' machine never set `pad[0].status`, so the game ran with
no controller. The desktop's save menu harness had set it to 1 since
[[226]], and its cases passed.

## Fix

- **`piney_input::Pad::unplugged`**: `status` 0. Only a probe sets it.
- **`piney_desktop::savesys::saved_hold`**: the rule, written once. Both
  `SaveMenu` and `RecordMenu` take their hold from it.
- **The harnesses.**
  - tools/test_fieldui_rs.py's machine has `pad[0].status` 6 (stable)
    unless a scenario is `unplugged`.
  - The probes (`fieldui_probe`, `desktop_probe savemenu`) take an
    `unplugged` line.
- **The pulled-controller pause** is written up in
  docs/engine/overview.md "The pad". It is not ported: the port's keyboard
  never goes away.

## Checked

- **Recorder cases (tools/test_fieldui_shop_rs.py `RecordPages`)** pass
  on Mutation and Infection. That is seven cases, with a new
  `test_save_unplugged` that answers OK every three frames after the
  save. With the hold forced to 10 it fails at frame 128 on Mutation,
  where the game's hold is 0.
- **Desktop save menus (tools/test_desktop_savemenu_rs.py)** pass on
  Mutation and Infection, with a new "create and save, unplugged". With
  the hold forced to 10 it fails on Mutation.
- **Whole harnesses.** The talk harness (48 cases) and the shop harness
  (34) pass on Mutation.
- piney-game's suite (four threads), the tests of piney-input,
  piney-desktop and piney-fieldui, clippy on those crates, fmt,
  `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing about the hold. The Quarantine shop cases
[[392]] listed beside it are still open (UNKNOWNS.md).
