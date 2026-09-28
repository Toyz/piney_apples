---
number: 60
title: Mac Anu's music: ccSndSQLoad's town case and ccSndBgmCtrl's
date: 2026-09-23
area: audio
files: crates/piney-audio/src/driver.rs, crates/piney-game/src/world.rs, docs/engine/sound.md
---

# 60. Mac Anu's music: ccSndSQLoad's town case and ccSndBgmCtrl's

After [[59]], the town was silent. `ccSetupGameCtrl` asks for
`ccSndSQLoad(2)` during its hold, and for `ccSndBgmCtrl` when the fade-in
ends, but neither function's town case had been traced.

## What the game does

`ccSndSQLoad`'s case 2 (0x00182380 for the bank, 0x001829d4 for the
volumes) picks the row by `WORLD_MAN::GetTownType()`, adding 5 while
`saveData.crisis` (+0x6772) is set:
- `sqDataTown` (0x00307d40, 24-byte rows) gives the bank;
- `sqVolTblTown` (0x00309f90, 36-byte rows) gives the sequences' volumes.

That makes ten rows: five towns, then their crisis versions. The case
leaves `area` and `playtype` at 0.

`ccSndBgmCtrl`'s town case (0x0017b064) branches by town type:
- **Mac Anu (type 0).** During the crisis it plays sequence 2, if the bank
  has three sequences and that one is not already playing. Otherwise it
  goes to the common tail, which plays sequence 0 as `ccSqPlay(0)` does.
- **Types 1-4.** Their branches are not traced.

## The port

- **`piney-audio`.** `SqContext` gained `Town { row }`, which loads that row
  of the town tables.
- **The runtime's world mode.** It turns the world's `SqLoad(2)` into the
  town row from the save's `lastTown` and crisis flag. It turns `BgmCtrl`
  into `SqPlay(0)`, or `SqPlay(2)` during the crisis.

## Checked

`mac_anu_music` (`piney-audio`) follows the game's order: all sound off,
Mac Anu's bank (row 0), then sequence 0. The bank loads silent, and the
sequence then plays at a sane level over six seconds. The crisis row loads
a bank of its own. `piney-audio` and `piney-game` pass, and clippy and fmt
are clean.

**Still unknown:**
- **The other towns.** `ccSndBgmCtrl`'s branches for town types 1-4.
- **The crisis flag in Mac Anu.** What `ccSnd` +0x105 (1 in Mac Anu, 3
  elsewhere) changes, and how the crisis music's three-sequence condition
  plays out.
- **Against a run.** No check compares the music with the game's; only
  the table rows and the call order come from the code.
