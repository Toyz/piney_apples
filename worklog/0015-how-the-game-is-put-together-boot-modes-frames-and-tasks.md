---
number: 15
title: How the game is put together: boot, modes, frames and tasks
date: 2026-09-22
area: engine, decomp
files: docs/engine/overview.md
---

# 15. How the game is put together: boot, modes, frames and tasks

The top of the program, read from `main` down. A port needs this shape
before any subsystem; the summary is on
[the overview page](../docs/engine/overview.md).

## Boot

`main` (`INF SLUS_202.67:0x0015a780`, `main.cpp`) calls, in order,
`ccHeap::InitFirst`, `mwInit`, `ccCdInit`, `InitCCSys`, `ccLoad_SndModules`,
`ccSndInit`, `ccMcardInit` and `ccInitThread`, starts two tasks -
`ccPadThread` and `ccThMother` - and then loops forever: `WakeupThread` on the
control task `tscbCtrl`, `SleepThread`, `ccSystem::Ctrl`, `ccAddPlayTime`.

`ccThMother` (`0x00167940`, `mother.cpp`) builds the long-lived systems: the
common file list and resources (`ccSetFileListCmn`, `ccLoadResourceFL`), the
fonts, the sound task `ccSoundMain`, the gcmn file list, the draw packet
controller and layer, the screen fader, `ccGame` (global `game`),
`ccSaveSys`, `ccSaveData` (global `saveData`), `ccEvent` (global `eventMng`)
and the event task `ccThEvent`. It starts event 1 and asks for mode 2 with
`ccGame::ChangeRequest(2, 7)`.

## Modes and overlays

`ccGame.request[4]` is a queue. Each time round, `ccThMother` takes
`request[0]`, shifts the rest down, and dispatches:

| request | setup | overlay | frame rate |
| ---: | --- | --- | --- |
| 2 | `ccSetupDemo` | `DEMO.PRG` | 1 |
| 3 | `ccSetupDesktop` | `DESKTOP.PRG` | 1 |
| 4 | `ccSetupToppage` | `TOPPAGE.PRG` | 1 |
| 5 | `ccSetupNewGame` | `GCMN.PRG` | 2 |
| 6 | `ccSetupGameCtrl` | none - gcmn stays | - |
| 1, 0x1000 | handled inside `ccThMother` | | |

Each setup tears down the previous mode (`ccDeleteAllThread`,
`ccDrawEnv::Reset`, font and layer flips), loads its overlay, sets its file
list and loads its resources. The overlay is loaded on a task,
`ccThLoadOverlay` (`0x001680e0`), which calls `mwLoadOverlay` ([[4]]) with a
path from `overlay_tbl` and retries until it returns non-zero. Paths are
`cdrom0:\DATA\DEMO.PRG`, `DESKTOP.PRG`, `TOPPAGE.PRG`, `GCMN.PRG`.
`ccSetupGameCtrl` picks the town, field or dungeon file list
(`ccSetFileListTown`/`Field`/`Dungeon`) and starts the field game.
`ccLoadOverlay` (`0x00168030`) can also be called from four places in the
event script interpreter `ccEvent::Execute` - a function large enough that
those calls are at offsets 0x7650 to 0x76a4.

Main calls straight into gcmn at fixed addresses - `ccSetupNewGame` calls
`ccSPC::Initialise` and `ccParty::InitParty`, `ccSetupGameCtrl` calls
`ccStoreSpcCondition` and `initHitCheck`, all in gcmn - so those paths are
only valid while gcmn is the resident overlay. The relocations record these
cross-overlay calls like any other.

## Frames

`ccSystem::Ctrl` (`0x0010a5f0`) is one frame: shadow packets, wait for the GS
path (`sceGsSyncPath`), then wait for vertical sync until
`frameRateCnt >= frameRate`; reset the count, zero EE Timer 0 (a store to
0x10000000), bump `ccSystem.count`, set the screen mode, flush the cache,
swap the double buffer, queue the font and layer display lists, `SendDMA`,
read both pads, refresh the display list. `VSyncCallBack` (`0x001099e0`)
increments `frameRateCnt` and a global `tick` every vertical blank.

So `frameRate` is vertical blanks per frame: 1 is 60 frames per second on
NTSC, 2 is 30. `SetFrameRate` is called with 1 by `ccSystem::Init`,
`ccThMother`, and the demo, desktop and bulletin-board setups; with 2 by
`ccSetupNewGame`; and by the event interpreter (once with 2, once with a
computed value). The field game runs at 30 frames per second, the desktop at
60. That matches the 30 fps inferred for the cutscenes in [[14]].

## Tasks

Every task is a PS2 kernel thread wrapped in a `ccTscb` (0x54 bytes:
entry, `tid`, `priority`, `sleep`, `sr` start request, `del` delete stage,
`mask` flags, eight `param`s, `link`, `name`, two cleanup callbacks, stack).
`ccStartThread` allocates one and `ccTscb::GoThread` (`0x00159d40`) does
`CreateThread` + `StartThread` with the game's `$gp`.

Scheduling is cooperative and frame-locked. Once per frame `main` wakes the
control task `ccThControl` (`0x0015a5c0`), which walks the `ccTscb` list: a
pending start request gets `StartThread`; a task being deleted runs its
`funcI` callback at stage 2 and is terminated, deleted and freed at stage 3;
a suspended task (mask bit 0x08) is resumed; and a task that is waiting in
`Breath` (mask bit 0x20) and not held by its `sleep` flag gets
`WakeupThread`. `ccTscb::Breath(n)` (`0x00159e10`) sleeps through `n` of those
wake-ups and then keeps sleeping while `sleep` or `del` is set. A task's
per-frame work is the code between two `Breath` calls; the kernel's thread
priorities decide the order tasks run in within a frame, then `main` resumes
and draws.

**Still unknown:** what modes 1 and 0x1000 do; the full list of tasks each
mode starts and their priorities; `ccEvent::Execute`, the event script
interpreter, which is its own large subject; what `ccSystem::Ctrl`'s other
branches (screen-mode changes, the flags at `+0xbd4`/`+0xbd5`) handle.
