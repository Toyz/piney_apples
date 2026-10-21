---
number: 412
title: Gorre's brothers show its protect's break and return: the other brother breaks with its picture, and CalcRealEx shows the return on both brothers, not on Gorre (#69)
date: 2026-10-21
area: battle, volumes, test
files: crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/message.rs, crates/piney-game/src/session/tests/boss_fx.rs
started: 2026-10-21T09:23:56-06:00
took: 10m
---

# 412. Gorre's brothers show its protect's break and return: the other brother breaks with its picture, and CalcRealEx shows the return on both brothers, not on Gorre (#69)

Issue #69's protect pictures ([[410]]) checked on the other volumes'
bosses. Outbreak's Gorre (`ccBoss05`) keeps its protect on its two
brothers, and two of its pictures were missing.

## What the game does

- **The break.** Kite's blow breaks the struck brother's protect in
  `CalcBattleDamage` (its picture, as on any boss). The brother's
  `Affect` then sends Gorre msg 0, and `SendMessage` (OUT gcmn 0x004adbb0)
  hands the count to Gorre. At a fresh break (the count 300) it breaks the
  other brother with it: `effProtect(other, 0, 2)` (0x004add68), its PP 0
  and count 300. The port set the gauge and count but showed no picture.
- **The return.** Gorre's `Main` begins with `CalcRealEx(0)`
  (0x004a9650), `ccChar::CalcReal` on Gorre's own row
  (`ccGetBossParam`). When the count runs out it shows the return on both
  brothers, `effProtect(brother, 1, kind)` and `SetProtect(1, brother)`
  for each (0x004a980c, 0x004a9844), not on Gorre. The port ran the base
  `CalcReal`, so since [[410]] the return showed on Gorre, whose body is
  not drawn; before it, nowhere.
- Each brother's own `CalcReal` (in its `Main`, after Gorre's hands it
  the count) runs out a frame before Gorre's, so each brother shows its
  own return and then Gorre's.

## The fix

- `gorre::calc_real_ex` runs `Boss::calc_real` and moves the return and
  its `SetProtect` from Gorre onto both brothers, in the game's order.
- `share_protect` raises the other brother's break picture.

## Checks

- `session::tests::boss_fx::gorres_brothers_show_the_protect_break_and_return`
  (Outbreak's event 218 in Gorre's arena, a spell's blow on one brother
  with its gauge one short, then its hit as an affect): before, the break
  showed on the struck brother only (40 frames) and the returns on both
  brothers and on Gorre; after, both brothers show the break (40 frames
  each) and two returns each (80 frames), Gorre none.
- `tools/test_battle_gorre_rs.py` (Outbreak) still agrees; piney-battle's
  tests, piney-game's suite (four threads), clippy and fmt pass.

The other bosses: Skeith, Innis, Magus and its leaves, Kyvia's first and
second fights with the core and gomoras, and Fidchell show their returns
through `Boss::calc_real` ([[410]]); none but Gorre has a `CalcReal` of
its own. Quarantine has no boss ported yet.

**Still unknown:** nothing. Quarantine's code has the same `effProtect` callers (only Gorre's own besides `ccChar`'s); its bosses are not ported yet.
