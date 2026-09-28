---
number: 94
title: The staff roll: the ending's credits as ccThStaffRoll runs them, checked frame by frame
date: 2026-09-25
area: ui, audio, test
files: crates/piney-desktop/src/staffroll.rs, crates/piney-desktop/src/lib.rs, crates/piney-desktop/src/view.rs, crates/piney-desktop/tests/staffroll.rs, crates/piney-desktop/tests/staffroll_fixture.txt, crates/piney-game/src/desktop.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/main.rs, tools/test_staffroll_rs.py, docs/engine/desktop.md
---

# 94. The staff roll: the ending's credits as ccThStaffRoll runs them, checked frame by frame

With story starts to the ending ([[93]]), `story:31` reached the
instruction `staff_roll`. The desktop host had nothing for it: its wait
ended at once, and the credits never showed. The desktop page listed
`ccThStaffRoll` as unknown.

## What the game does

The details are on [the desktop page](../docs/engine/desktop.md#the-staff-roll).
- **The task.** `ccThStaffRoll` (desktop.prg 0x004125c0) loads `stfroll1`
  and builds a `ccThStaffRollCtrl` of 19 lines. It plays `BGM.BIN` track 1
  and runs `Main` once a frame until the controller is done.
- **The pages.** There are 28 (`g_srDataGrp`): credits and a picture, or a
  picture alone.
- **The four phases.**
  1. **`_Random`.** The view zooms in from 5 to 1. Every line shows 44
     random characters (`ccRand`) and settles one more each frame into
     the credits (`StopChar`, the C library's `rand`, seeded by the frame
     count).
  2. **`_Fix`.** The random characters fade while the picture fades in
     behind the credits.
  3. **`_End`.** The page holds for 110 frames, then shrinks and fades.
  4. **`_BGOnly`.** A page with a picture alone fades in, holds and fades
     out.
- **Names.** A credit's `#0` and `#1` are the save's two names
  (`ccTransCode2Name`).

## The port

- **The controller.** piney-desktop's `staffroll.rs` keeps the fields
  under their offsets' names and records each frame's draws.
  `StaffRoll::render` draws them with the port's `ccKanji` (the
  `LargeFixed` font) and the page picture's mask.
- **The task.** `Desktop::start_staff_roll`: two breaths, then `Main` each
  frame, and `ccBgmPlay(1)` / `ccBgmStop` as new requests. They reach the
  audio as `Event::BgmStream` and `Event::BgmStreamStop`
  (`Audio::bgm_stream`, `bgm_stream_stop`).
- **The desktop host.** `begin(StaffRoll)` starts the task,
  `busy(StaffRoll)` waits for it, and `end` does nothing.
- **The layer centre.** The layer's view needed a centre
  (`LayerView::frame_centred`: `SetFrame` then `SetLayerCenter`).
- **Ending music.** `bgm_control` came in 4d7a889.

## Two mistakes the check caught

- **`Except`'s column.** `v * 0x2aaaaaab >> 32` is v / 6, and the `sra 1`
  after it makes it v / 12, the column. The first port used v / 6.
- **The picture's `MakePacket` flag.** `_Fix` and `_End` never set `$a2`
  before the call; it still holds the 1 stored in `wi` just before. The
  first port passed 0.

## Checked

- **The whole roll against the game.** `tools/test_staffroll_rs.py` runs
  the game's controller natively in eemu over all of it (7,231 frames,
  seed 12345, names Kite and Tester, `ccRand` as at boot). Only the
  drawing calls are hooked, and they are recorded.
- **What each frame keeps.** The controller's words, every line's bytes,
  and the draws (hashed). The fixture is 650 KB; `detail N` prints a frame
  in full.
- **The replay.** `crates/piney-desktop/tests/staffroll.rs` replays every
  frame with 0 mismatches.
- **The view.** The harness also runs the game's `ccView`: `SetFrame` at
  one scale, `SetLayerCenter`, then `SetFrame` at another (as `_Random`
  and `_End` do each frame). All 36 points through
  `ApplyLayerScreenMatrix` match `LayerView::frame_centred`. So a later
  `SetFrame` keeps the centre.
- **The rest.** The workspace's tests, clippy, fmt and the docs check
  pass.
- **Shots.** From `story:31`: "Story, Kazunori Ito" over Kite's picture,
  and the CAST page (the English voice actors in two columns).

**Still unknown:**
- **Not compared: the pixels and when music starts and stops.** The glyphs
  come from the port's `ccKanji` (checked for the desktop), and the
  picture from the port's sprite. The frame the ending song starts and
  stops is not compared.
- **The generators' state at the ending.** The runtime starts `ccRand` as
  at boot and seeds `rand` with the desktop's own frame count, so the
  random characters differ from a console's. They look alike.
- **Created and never used.** `ccBufferSampling` (+0x138c) and the second
  mask (+0x1358, left null).
- **The later volumes' staff rolls** (`stfroll2`-`4`) are not tried.
