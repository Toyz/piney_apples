---
number: 438
title: The endings hold the desktop's music in the frame their pass settles: a frame's events go out in the game's order, the event task's pass before the desktop's load, so Outbreak's sound 10 holds the theme under its ending and roll
date: 2026-10-23
area: audio, ui, test
files: crates/piney-game/src/desktop.rs, crates/piney-game/src/session/tests/ending_save.rs, docs/engine/sound.md, docs/engine/desktop.md
started: 2026-10-23T23:30:12-06:00
took: 12m
---

# 438. The endings hold the desktop's music in the frame their pass settles: a frame's events go out in the game's order, the event task's pass before the desktop's load, so Outbreak's sound 10 holds the theme under its ending and roll

Issue #73's second report (build a2c6920): at the end of Outbreak, the
music the player had picked for the ALTIMIT desktop played over the
ending credits, on top of the ending's own song. Infection had the same
report (#45), fixed in worklog 371.

## What the game does

Every ending's set-up pass ends the same way:

- `sound 8`, then `sound 10`. `sound 10` sets `ccSnd +0x138`, the hold.
- Once the pass settles, `ccSetupDesktop` loads `Wave[dtBgm]`
  (`ccSndChangeData(.., -1)`, load only) and calls `ccSndBgmCtrl`.
- The desktop's case of `ccSndBgmCtrl` sees the hold, clears it and
  starts nothing.
- `ccThStaffRoll`'s `ccBgmPlay(1)` (`BGM.BIN` track 1) plays alone.
- The theme starts only after the save menus.

The hold is set inside the pass, so it always comes first. The endings
differ only in what follows `sound 10`:

- Infection's event 31: `info_now` lines. The pass settles about 157
  frames later.
- Mutation's 116: the same, about 169 frames later.
- Outbreak's 219: two `mail_vol`. The pass settles in the frame of
  `sound 10` itself.
- Quarantine's 314: `sound 10` comes in its phase-0 pass, before a
  `wait 10`, a line and another `wait 10`. Its set-up's load comes after
  the phase-2 pass, later still.

## Cause

`DesktopMode::take_events` gave the desktop's own requests first and
the event task's events after them. On Outbreak the frame that settles
the pass carried both:

- the desktop's `DesktopBgm(15)` (the reporter's `dtBgm`);
- the pass's `HoldBgm`, after it.

So the driver loaded the bank and ran `ccSndBgmCtrl` before the hold was
there, and started sequence 0. The hold came a moment later and waited
for the next `ccSndBgmCtrl`. That was the one after the roll's save
menus, which it swallowed.

Infection and Mutation were only spared by their `info_now` lines. In a
frame the game runs the event task's pass before the desktop. In the
port's `step` the pass also runs first (a movie's end, the pass, then
`desktop.step` or the set-up's `build`). Only the order of the events was
wrong.

## Fix

`take_events` now gives the frame's events in the order they were made:
the event task's pass first (and a movie's end), then the desktop's own
requests. Only Outbreak heard the difference; on the other three volumes
nothing comes in that order within one frame.

## Checks

- **`ending_save::every_ending_plays_over_no_desktop_theme`.** It plays
  Infection's 31, Mutation's 116 and Outbreak's 219 from story starts,
  under `dtBgm` 15 (the reporter's), 47 and 27. Every frame's events go
  through `crate::handle` into a headless engine.
  - No sequence plays on any frame from the event's start to the roll's
    end.
  - After menu 8's cancel the theme's sequence plays: 1 for 27, and for
    47 on Outbreak; 0 otherwise.

  Before the fix, Outbreak failed at its frame 2279 under each row.
  Infection and Mutation passed either way.
- **The player's pad log.** `outbreak_end::issue_73_replay` plays it to
  the ending. Before the fix, `DesktopBgm(15)` then `HoldBgm` came at
  line 48288, and sequence 0 played under the whole roll. After it,
  `HoldBgm` comes first, nothing plays under the roll (every 120th frame
  sampled), and the save menus' `BgmCtrl` brings the theme back at line
  55963.
- piney-game's suite passes.
- docs/engine/sound.md and desktop.md say how the frame's events are
  ordered.

**Still unknown:** nothing.
