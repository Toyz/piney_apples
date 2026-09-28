---
number: 43
title: The title screen from DEMO.PRG: boot, logos, menu, and the hand-off to the desktop
date: 2026-09-23
area: ui, decomp, test
files: crates/piney-demo, tools/test_demo_rs.py, docs/engine/title.md
---

# 43. The title screen from DEMO.PRG: boot, logos, menu, and the hand-off to the desktop

The game does not boot into the desktop. `ccThMother` asks for mode 2,
`ccSetupDemo` with `DEMO.PRG`: first the memory-card check, then the logo
movies, the intro stream, and the title menu. New Game from there starts the
desktop of [[37]]. A helper agent ported it as `piney-demo`, reusing the
desktop's 2D and 3D machinery, and I re-ran its checks. The reference is
[the title page](../docs/engine/title.md).

## What it is

`ccThDemo` runs `ccOpening_Control`:
- **Boot check.** `PlayBootMemCard` / `BootMem_Control`. With no card it
  asks whether to start anyway.
- **Logos.** `LogoMain` plays three PSS movies (`LOGO_B`, `LOGO_C`,
  `LOGO_H`) with its skip rule, then the title music.
- **Intro.** `PlayOpeningStream` plays stream 0 for 410 frames and fades the
  menu in from white.
- **Menu.** NEWGAME, DATALOAD and OPTION, drawn as 3D icons (lit by one omni
  light) over the animated hexagon backdrop, with the logo and copyright.
- **Attract loop.** An idle timer fades to black at 2,380 frames and loops
  back at 2,400.

The crate asks the runtime for movies, the stream, music and mode changes
through requests, as the desktop does. The memory card and the system menu
are seams.

## Findings

- **Parody Mode cannot be chosen in Infection.** `m_ParoFLG` is cleared in
  `Init` and written nowhere else, in Mutation's `DEMO.PRG` too, and
  `m_Max_CurNO = 3` stops the cursor before the fourth item anyway. The
  branch is ported behind a test switch.
- **The hand-off.** New Game ends the 61-frame screen-out animation, then
  emits `NewGame{parody}` and a music fade. `Breath(2)` follows, then
  `ChangeRequest(3, 7)`: the desktop.
- **The save.** `ccSaveData::Init(1)` runs at boot, `NewGame(1)` in
  `ccSetupDemo`, and `NewGame(0)` on New Game. `NewGame(0)` copies
  `charTbl` into `spcParam` and resets play time, words and trade items.
  None of it touches the desktop's members, so the desktop's fresh state is
  what New Game hands over.
- **Depth.** The title's camera puts every model at GS Z of a few thousand.
  The GS renderer mapped Z as `1 - z/2^32` and lost the order there. The
  agent found it and I fixed it in `piney-gs`: depth is now integer GS Z,
  larger nearer. Against the CPU GS model, the desktop's differing bytes went
  from 6,177 to 3,459. The title now matches it, except the lit icons, which
  the CPU model draws unlit.

## Checked

`tools/test_demo_rs.py` runs the game's code in eemu against the port, state
by state, 14 tests:
- **Cursor and actions.** `MoveCurNut` (40 random runs), `SwitchCur`,
  `PlayNeutral`, `ChangeMainAct`, and every `Set*` / `Play*`.
- **Per-frame drawing.** `AllTransparency` bit for bit, `AllAnimate` (the
  icon spin bit for bit), `AllDraw`.
- **Timing.** The fader after each kind of flash and fade, the idle timer,
  and `LogoMain`'s movie order, skip rule and waits.
- **The rest.** The boot check, and `ccThDemo` itself, breath by breath.

On integration:
- the crate's tests pass (4 unit, 7 on the disc);
- `test_demo_rs.py` is OK;
- clippy and fmt are clean.

A 150-frame shot shows the menu as above.

**Still unknown:**
- **Stand-ins.** The load screen (`Data_Control`'s lists and `LoadData`) and
  the Option menu are stand-ins.
- **Playback.** PSS movie playback and the stream are not modelled; they
  count as played.
- **Title music.** It needs `piney-audio`'s title context
  (`ccSndSQLoad(7)`).
- **Later volumes.** `SetNextData` / `PlayNextData` and `ConvGame` are not
  ported.
- **Inferred timing.** The boot check's timing and the stream's end-flash
  alignment are inferred, not read.
