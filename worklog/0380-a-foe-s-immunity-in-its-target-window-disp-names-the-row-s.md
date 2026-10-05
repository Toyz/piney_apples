---
number: 380
title: A foe's immunity in its target window: Disp names the row's lowest Exdefense bit beside the HP, struck through once the defence is lowered
date: 2026-10-05
area: ui, battle, test
files: crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/world.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-battle/src/damage.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/tolerance.rs, crates/piney-game/src/session.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md
---

# 380. A foe's immunity in its target window: Disp names the row's lowest Exdefense bit beside the HP, struck through once the defence is lowered

Issue #49 (ThreePendant, build 86c2571): "Enemies don't have their
Elemental Immunity icon showing on their stats". The screenshots show a
Hell Hound's target window: its name, HP 1210/1210 and the fire element
icon, and nothing else.

## What the game draws

`ccMenuCtrl::Disp` (gcmn 0x0051cfb0), read:

- **The text** (0x0051e960 - 0x0051ec3c). Outside a town, `nameKanji`'s
  sixth piece comes from the target's table row (`ccGetEnemyParam` for
  type 0x60, `ccGetBossParam` for 0x80). It is the lowest bit of
  `Exdefense` (+0x64), as a piece of `kyviaStatusStr` (main 0x00377e90):
  "Physical Tol.", "Magic Tol.", "Earth Tol.", "Water Tol.", "Fire Tol.",
  "Wood Tol.", "Thunder Tol.", "Darkness Tol.". If that bit's defence in
  `personality.real` is below the row's, the bit is kept in `$fp`.
- **The drawing** (0x005202e4 - 0x0052063c), after `ConditionIconDisp`.
  If the row's `Exdefense` is not 0, the piece is drawn (`nameKanji` cell 5)
  at (145, 122) in colour 23. Its alpha follows the cursor's pulse, capped
  at `targetAlpha`. With `$fp` set, `itemIcon`'s cell (0, 0x1800) is drawn
  over it, as wide as the piece (60 for "Fire Tol."): a strike-through.

So the panel shows the immunity as text, not as an icon. Only the lowest
bit is named. Hell Hound's row 172 has `Exdefense` 0x10, so it shows
"Fire Tol.". The `Disp` of Mutation (0x0053b100), Outbreak (0x00536480)
and Quarantine (0x00428d90) has the same chain, pulse, position and bar
widths.

## The cause

The port's `name_texts` put an empty piece there, with the comment
"The weakness line of Kyvia's forms: none in the port's world". The
window had nothing after the condition icons. `CharInfo` did not carry
the row's `Exdefense`. That dates from the first public commit (3fa1e9e).
`git log -S` finds no other version of either line, so this is not a
regression to bisect. The element icon the reporter remembers is still
drawn. 73fddcd (the affect flag) does not touch the HUD.

## The fix

- `CharInfo` has `exdefense` and `exdefense_lowered`. The field fills them
  from the foe's row and `real`, through the new
  `piney_battle::damage::exdefense_lowered`. That uses the same
  bit-to-defence table as the damage rules.
- `CharInfo::tolerance` picks the piece and whether it is broken.
- `name_texts` puts the piece in line 5, and `disp::tolerance` draws the
  line and the bar as above.

## Checked

- **Against the game.** `tools/test_fieldui_rs.py`'s new
  `test_target_tolerance` runs the game's `Disp` in eemu. The harness's
  `ccGetEnemyParam`/`ccGetBossParam` hooks now return a row with the
  case's `Exdefense`, and `personality.real` is lowered where asked. The
  probe takes `exdef`. Six cases:
  - fire held, and fire broken;
  - 0x12 with fire lowered: "Magic Tol." is named, not struck;
  - a boss's physical immunity broken;
  - dark broken;
  - a town, where nothing is drawn.
  Without the `disp.rs` change it fails on frame 2: the game sends
  `nameKanji` cell 5 at (145, 122), the port does not. With it all six
  pass, and so does `test_target`.
- **The session.** `session/tests/tolerance.rs`,
  `a_foes_immunity_shows_in_its_target_window`: Hell Hounds put in story
  area 14's field, one fixed as the command target. Line 5 of `nameKanji`
  reads "Fire Tol." and is drawn at (145, 122). Once a debuff (`temp` -10)
  takes its fire below the row's, the 60-wide bar is drawn over it. The
  test fails before the fix.
- **Shots.** `tolerance_shots` (ignored):
  - Before: /mnt/data/claude/scratch/issue49/before/hell-hound-held.png,
    as in the issue: name, HP and fire icon only.
  - After: /mnt/data/claude/scratch/issue49/after/hell-hound-held.png
    shows "Fire Tol." in orange right of the HP. hell-hound-lowered.png
    shows it struck through.
- The suite passes.

**Still unknown:** nothing. The game's code settles every part of it,
in all four volumes, so no PS2 capture is needed.
