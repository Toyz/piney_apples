---
title: How the game is put together
status: partial
volumes: INF
covers: INF SLUS_202.67:0x0015a780 main, 0x00167940 ccThMother, 0x001671e0 ccGame::ChangeRequest, 0x001680e0 ccThLoadOverlay, 0x0010a5f0 ccSystem::Ctrl, 0x001099e0 VSyncCallBack, 0x0015a5c0 ccThControl, 0x00159e10 ccTscb::Breath
worklog: 15
---

# How the game is put together

One resident program, one overlay at a time, and a set of frame-locked tasks.

## Boot

```
main                                   main.cpp
  ccHeap::InitFirst, mwInit, ccCdInit, InitCCSys, ccLoad_SndModules,
  ccSndInit, ccMcardInit, ccInitThread
  start task ccPadThread
  start task ccThMother
  loop:  WakeupThread(tscbCtrl); SleepThread(); ccSystem::Ctrl(); ccAddPlayTime()

ccThMother                             mother.cpp
  common file list + resources, fonts, task ccSoundMain, gcmn file list,
  draw packet controller, layer, screen fader,
  game = new ccGame, saveSys, saveData = new ccSaveData,
  eventMng = new ccEvent, task ccThEvent
  ccStartEvent(1); game->ChangeRequest(2, 7)
  loop: take game->request[0], shift the queue, dispatch (below)
```

## Modes

`ccGame::ChangeRequest(num, sf)` puts `num` in the first free slot of
`ccGame.request[4]` and, unless `num` is 6, calls `ccGame::InitScene`.

| request | setup | overlay loaded | frameRate |
| ---: | --- | --- | ---: |
| 2 | `ccSetupDemo` | `cdrom0:\DATA\DEMO.PRG` | 1 |
| 3 | `ccSetupDesktop` | `cdrom0:\DATA\DESKTOP.PRG` | 1 |
| 4 | `ccSetupToppage` | `cdrom0:\DATA\TOPPAGE.PRG` | 1 |
| 5 | `ccSetupNewGame` | `cdrom0:\DATA\GCMN.PRG` | 2 |
| 6 | `ccSetupGameCtrl` | none; gcmn stays resident | - |
| 1, 0x1000 | inside `ccThMother` | | |

Overlays are loaded on the task `ccThLoadOverlay`, which retries
`mwLoadOverlay` until it succeeds; see [the overlay format](../formats/prg.md).
The event interpreter `ccEvent::Execute` can also load one through
`ccLoadOverlay`. Main code calls into gcmn at fixed addresses, valid only
while gcmn is resident.

## Frames

```
ccSystem::Ctrl                          one frame
  shadow packets; wait for the GS path
  while (frameRateCnt < frameRate) sceGsSyncV
  frameRateCnt = 0; EE Timer 0 = 0; count += 1
  screen mode; FlushCache; swap double buffer
  font and layer display lists; SendDMA
  read pad 0 and pad 1; refresh the display list

VSyncCallBack                           every vertical blank
  frameRateCnt += 1; tick += 1
```

`frameRate` is vertical blanks per frame: 1 = 60 fps, 2 = 30 fps (NTSC). The
field game runs at 30, the desktop, bulletin board and demo at 60; event
scripts can change it.

## Tasks

```
ccTscb              0x54 bytes, one per task
  +0x00  entry      void (*)(void *)
  +0x04  tid        s16, the kernel thread id
  +0x06  priority   s16
  +0x08  sleep      s16, holds the task asleep while non-zero
  +0x0a  sr         s8, start request
  +0x0b  del        s8, delete stage
  +0x0c  size       stack size
  +0x10  mask       0x08 suspended, 0x20 waiting in Breath
  +0x14  param[8]
  +0x34  link       next ccTscb
  +0x38  name
  +0x3c  tmng, +0x40 func      cleanup callback and argument
  +0x44  tmngI, +0x48 funcI    delete-stage-2 callback and argument
  +0x4c  stack
  +0x50  t0count, +0x52 stackOverFlag
```

Each task is a kernel thread (`CreateThread` + `StartThread` in
`ccTscb::GoThread`). Once per frame `main` wakes `ccThControl`, which walks the
list and, per task: starts it if `sr`; runs `funcI` at `del` stage 2 and
terminates, deletes and frees it at stage 3; resumes it if suspended; wakes it
if it is in `Breath` and `sleep` is 0. `ccTscb::Breath(n)` sleeps through `n`
wake-ups, then keeps sleeping while `sleep` or `del` is set. A task's work for
one frame is the code between two `Breath` calls.

## Unknown

- Modes 1 and 0x1000.
- Which tasks each mode starts, and their priorities.
- `ccEvent::Execute`, the event script interpreter.
