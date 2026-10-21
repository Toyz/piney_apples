---
number: 413
title: Innis's HP on the target window is the game's: from 10000 maxHP the game spells the last four digits through cheatHpStr, so 29884/30000 reads ':%%4/0000' (#69)
date: 2026-10-21
area: ui, test
files: docs/engine/field-ui.md, tools/test_fieldui_rs.py
started: 2026-10-21T09:34:28-06:00
took: 1m
---

# 413. Innis's HP on the target window is the game's: from 10000 maxHP the game spells the last four digits through cheatHpStr, so 29884/30000 reads ':%%4/0000' (#69)

Issue #69's first screenshot shows Innis's HP on the target window as
`:%%4/0000`. That is what the game draws: a target whose maxHP is 10000
or more shows its HP and maxHP each as the last four digits spelled
through `cheatHpStr`, and the port draws the same characters.

## What the game draws

`ccMenuCtrl::Disp`'s target window (INF gcmn 0x0051fb8c, MUT 0x0053dd30,
the same code) sets `flyFont` to type 2, colour 22, and branches on maxHP
(`+0x74` of the target's info):

- below 10000: HP and maxHP as numbers, right aligned by maxHP's digits;
- from 10000: for each, `v % 10000`, its four digits each looked up in
  `cheatHpStr` (main 0x00377e88 on Infection, 0x0038ae1c on Mutation),
  written as `HHHH`, the slash, `MMMM` at (55, 132).

`cheatHpStr` points at `"01*#4>67%:"` on both volumes: 0 1 4 6 7 keep
their digit, 2 reads `*`, 3 `#`, 5 `>`, 8 `%`, 9 `:`. Type 2's cells are
`TEX_xasc00`'s 10 x 12 grid at (128, 96), ASCII 0x20-0x60, so those
symbols draw as themselves: there is no hidden digit glyph behind them.
Innis's maxHP is 30000 (`bossTbl` row 1) and its HP under `cheatHP`
falls by a tenth of each hit; at 29884 the window reads `:%%4/0000`, the
player's screenshot. Skeith (12000) at 9000 reads `:000/*000`. So a
boss with break-proof HP shows a garbled count on purpose, on every
volume.

## Checks

The port's `disp::target_window` does the same (`texts.cheat_hp`, from
the build). `tools/test_fieldui_rs.py`'s `test_target` runs the window
against the game's code and already covered Skeith's 9000/12000 and a
20000-HP target; Innis's 29884/30000 is a case now, and the test passes
on Infection and Mutation. Nothing in the port changed.

docs/engine/field-ui.md, "The target's HP". The reading was done while
[[411]]'s clock ran, so its time is counted there.

**Still unknown:** nothing.
