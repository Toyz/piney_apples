---
number: 298
title: "An event's scene -2 put the town's card up: the port skipped loadCheck"
date: 2026-09-28
area: engine, script, ui
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs
---

# 298. An event's scene -2 put the town's card up: the port skipped loadCheck

Reported in play (INF): in Mac Anu during event 13 (`MG0340`, Mia and
Elk), the "Aqua Capital / Mac Anu" loading card came up as though Kite had
logged in again. Block 2 of the event (`if near_marker 31 <= 80`, `sound`,
`fade 10 128`, `wait 10`, `remove -1 -1`, `scene -2`) sets the same town up
again so block 3 can play stream 13. The port put the card up on every set-up
of The World (`Session::world_stage`).

## What the game does

`ccFileListLoad` (INF SLUS_202.67:0x00164540) calls `ccLoadDispInit` at
0x00164a68 only when all three hold:

- `loadCheck()` (0x00165530) is nonzero: some entry of the new
  `sceneFileList` (40-byte rows, name at +4) is missing from the old list
  (`dbgList`, `sceneFileListNumOld` rows). It returns 1 at the first new
  file, 0 when every file was already there;
- the thread's +0x18 is 0. `ccLoadFLStart` (0x001654a0), the start for
  `ccLoadFLAdd`/`ccLoadFLAddOne`, stores 1 there, so extra loads never
  show a card;
- `ccLoadDispCheck()` (0x0019c430) is 0: no display is already up.

A `scene -2` reload of the same town reads nothing new, so the game shows
no card; the screen stays black from the script's fade until the town
fades back in.

## The fix

`world_stage` already recorded what each set-up inflates, for `--dvd`'s
timing (`resident`). When every file the new set-up read is in the last
set-up's set, the card is taken down before it draws. `Session::change`
(any `ChangeRequest`: desktop, board, title, log in) now clears `resident`,
because those modes load their own lists and the game's old list is then
theirs, not the last area's.

`event_13_s_town_again_has_no_card` puts Kite at marker 31 once block 1 is
done and fails on any card before stream 13 plays. Without the fix it fails
21 frames after the put ("loading 0"). The piney-game suite passes (162).

**Still unknown:**
- The port's recorded reads stand in for the game's `sceneFileList`; a set-up
  whose file list names a file the port never inflates (or the reverse)
  could still differ from the game on whether the card shows. No two lists
  were compared.
- Log in from the board goes through `WorldMode::enter` directly and puts up
  no card at all; whether the game shows the town's card there (it would:
  the board's list lacks the town's files) was not checked in play.
