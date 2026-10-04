---
number: 371
title: The staff roll silences the desktop's theme and scrolls at the game's pace
date: 2026-10-04
area: audio, ui
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/tests/driver.rs, crates/piney-audio/tests/driver_fixture.txt, tools/sound_ee.py, crates/piney-game/src/desktop.rs, crates/piney-desktop/src/lib.rs, crates/piney-game/src/session/tests/ending_save.rs, docs/engine/sound.md, docs/engine/desktop.md
---

# 371. The staff roll silences the desktop's theme and scrolls at the game's pace

Two player reports from Infection's credits (build f6e01cc).

## The desktop's theme under the roll (#45)

The desktop's music setting (Mac Anu's theme, for the reporter) played
under the staff roll's own music. Event 31's set-up pass (block 0, phase
2) runs `sound 8` and then `sound 10`, which sets `ccSnd +0x138`, the
hold. `ccSetupDesktop` then calls `ccSndChangeData(&Wave[dtBgm], -1)`,
which only loads, and `ccSndBgmCtrl`. Its desktop case (0x0017b7d4)
clears the hold and starts nothing. So on the console the desktop after
the ending is silent, and `ccThStaffRoll`'s `ccBgmPlay(1)` (`BGM.BIN`
track 1) plays alone. The `staff_roll` instruction's own
`ccSndBgmCtrl`, after the save menus, starts the theme.

The port's `Driver::desktop` loaded and then played the sequence
outright, so it never looked at the hold. A second fault sat under it.
`ccSndChangeData` ends by setting `+0xf0` to 1 (0x00183c68), so
`ccSndBgmCtrl` takes the desktop's case whatever `ccSndSQLoad` loaded
before. The port left `context` at the last area's bank, so the roll's
closing `BgmCtrl` ran a town's or a field's case.

`Driver::desktop` is now `change_data_fresh` (the `old == -1` path:
fades off, `bgmStopFlag` set, all sound off, TOBJ's hum and `+0x137`
cleared, the load), `gameStart`, then `bgm_ctrl` with `dtBgm`. `load`
ends as the game's does: the loop slots free, the battle music and
`bgmStopFlag` off, `context` 1. That also covers the jukebox's change.
`tools/sound_ee.py` gains a `desktop NO` step in its `seq` scenarios.
Three new fixture lines run the game's code in eemu: a held desktop after
a town (rows 50 and 27, the second's sequence 1), and an unheld one after
a field. The driver matches all 370 scenarios.

`the_staff_roll_plays_over_no_desktop_theme` plays story:31 with every
frame's events routed through `crate::handle` into a headless engine,
from the session's start. No sequencer may play at the roll's start or
on any of its frames, track 1 must stream, and after menu 8's cancel the
theme's sequence plays. Before the fix, sequence 0 was already playing
when the roll began.

## The roll's pace (#46)

The roll went by twice as fast. Every ending script (31, 116, 219, 314)
sets `frame_rate 2` just before `staff_roll` and `frame_rate 1` after
it. `ccSys` has one rate, so the roll runs at 30 frames a second. That
is 7,230 `Main`s, about 241 s, against track 1's 239.96 s. The port's
desktop took a script's rate only while asleep (`slept()`), and the
`frame_rate 2` comes one instruction before the sleep. So the roll ran
at 60 and took 120.5 s. The desktop now takes the rate whenever the
scripts set it; on the desktop only the endings do so during play.

The start-up frames were also one short. `GoThread` starts the task in
the instruction's frame (priority 33, under `ccThEvent`'s 32), where it
calls `Breath(2)`. `Breath(n)` sleeps through `n` wake-ups (0x00159e10),
and the loop's `Breath(1)` comes before `Main`. So the first `Main` runs
on the third frame after, and the port's wait is now 3.

`the_staff_roll_runs_at_the_music_s_pace` counts the frames from the one
after the roll starts until it ends: 2 + 7,230, every one at rate 2,
14,464 vertical blanks (241.07 s), at least the music's length. Before
the fix: 7,231 frames at rate 1, 120.52 s.

## Checks

piney-game's suite passes (231, 82 ignored) with four test threads.
Under the 8 GB address-space cap at the default thread count it ran out
of memory twice in `the_tutorial_statue_s_glow_ends` (once as "Parent
device is lost" from the headless GPU). That test passes alone and
touches nothing changed here. piney-audio's and
piney-desktop's tests pass, including the driver fixture and the staff
roll's frame-by-frame fixture.

**Still unknown:** nothing.
