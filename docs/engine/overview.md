---
title: How the game is put together
status: partial
volumes: INF
covers: INF SLUS_202.67:0x0015a780 main, 0x00167940 ccThMother, 0x001671e0 ccGame::ChangeRequest, 0x001680e0 ccThLoadOverlay, 0x0010a5f0 ccSystem::Ctrl, 0x001099e0 VSyncCallBack, 0x0015a5c0 ccThControl, 0x00159e10 ccTscb::Breath, 0x00102d40 ccPad::Read, 0x00102a50 ccPad::Ctrl, 0x00102bf0 ccPad::SetActuater, 0x0010a740 ccSystem::Ctrl (the motors), 0x0010a900 ccSystem::Init, 0x0010ad20 ccSystem::SetScreenMode, 0x0010ad40 ccSystem::SetScreenModeMain, 0x0010ab30 ccSystem::SetDisplayOffset, 0x0010ac40 ccSystem::SwapDoubleBuffer, 0x0010de50 sceGsSetDefDispEnv
worklog: 15, 18, 40, 331, 339, 340
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
| 1 | inside `ccThMother`: the soft reset | | 1 |
| 0x1000 | inside `ccThMother`: every task deleted, no set-up | | |

Mode 1 (INF `ccThMother` 0x00167ca0) goes back to the start-up path at
0x00167b18:
1. `ccAllSoundOff`, the layers flipped back and every task deleted
   (`ccDeleteAllThread`);
2. the system and draw environment reset, `SetFrameRate(1)`;
3. `ccGame::Init`, `ccSaveData::Init(1)`, `ccEvent::Init`,
   `ccStartEvent(1, 0)`;
4. `ChangeRequest(2, 7)`: the title.

Three things ask for it with `ChangeRequest(1, 7)`: the desktop's Reset
(`ccDtMenu::ResetMenu`, 0x0016ed90), The World's Reset
(`ccMenuCtrl::ResetMenu`, gcmn 0x00540688) and the game over
(`ccOpenGameOverMenu`, gcmn 0x0056a7d4). Mode 0x1000 (0x00167d00) only
flips the layers and deletes every task. None of the 12 `ChangeRequest`
calls in main and the four overlays asks for it; the scripts' `mode`
uses 3 and 5.

Overlays are loaded on the task `ccThLoadOverlay`, which retries
`mwLoadOverlay` until it succeeds; see [the overlay format](../formats/prg.md).
The event interpreter `ccEvent::Execute` ([event VM](event-vm.md)) can also
load one through `ccLoadOverlay`. Main code calls into gcmn at fixed addresses, valid only
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

The woken tasks run in kernel priority order, the lowest number first,
each until its next `Breath`. The tasks each set-up starts (INF, the
priority as `ccStartThread`'s second argument):

| started by | tasks, by priority |
| --- | --- |
| `ccThMother` (start-up) | `ccThEvent` 32, `ccSoundMain` 112 |
| `ccSetupDemo` (title) | `ccThLoadOverlay` 17, `ccThDemo` 33, `ccThDtMenu` 33 |
| `ccSetupDesktop` | `ccThLoadOverlay` 17, `ccThDtMenu` 33, `ccThDesktop` 33 |
| `ccSetupToppage` | `ccThLoadOverlay` 17, `ccThDtMenu` 33, `ccThToppage` 33 |
| `ccSetupNewGame` | `ccThLoadOverlay` 17 |
| `ccSetupGameCtrl` (towns, fields, dungeons) | `ccThGameCtrl` 33, `ccThMenu` 34, `ccThCamera` 40, `ccThSpc` 48, `ccThEntryCtrl` 64, `ccThExecuteStream` 65, `ccThEffect` 80, `ccThSkill` 82, `ccThFieldDisp` 96, `ccThParticle` 98 |

Others start on demand: `ccLoadDispTh` 17, `ccThSaveSys` 20, `ccThGameOver`
33 ([field walk](field-walk.md)). So in The World a frame runs the event
task, the game control, the menus, the camera, the party, the entries
(enemies, gimmicks, NPCs), the effects, the skills, the display and the
particles, and the sound task last. A menu's request is acted on by the
characters and heard in the same frame.

## The pad

`ccPad::Read` (0x00102d40) runs once a frame for a pad already connected
(`ccPad::Ctrl`'s state 0x40). The pad reports its buttons active low in two
bytes; `Read` inverts them (the first byte in bits 8-15, the second in bits
0-7). In analog mode the left stick also presses the D-pad by its angle when
the D-pad itself is not held. Then:

```
push   = now & ~direct          pressed this frame
unpush = direct & ~now          released this frame
repeat = now on the first frame of a new combination, nothing for the
         next 15 frames it is held, then now every frame
direct = now
```

Two quirks: a stick with its raw y exactly 128 has angle 0, down
(`SetAnalogStick` tests the y offset before `atan2f`); and the up-right
eighth of the circle presses right alone, where the other diagonals press
both directions.

The motors (`ccPad` +0x28..+0x3f):

```
SetActuater(small, power, ms)   0x00102bf0: with ccPad::actuaterSw on and a
  DualShock ready, the time (3 ms + 25) / 50 in vblanks; each queued entry
  from the top down loses that time, or is dropped when it has no more
  (the ones above it move down); with room (at most three), the top (or
  the idle entry 0) is marked to send again, and the new one goes on top,
  marked: its time, the small motor on or off, the large motor's power
ccSystem::Ctrl                  0x0010a740, each frame for each pad: the
  finished entries off the top, then the top's time less the frame rate
  (vblanks a frame), not below 0
ccPad::Ctrl                     0x00102a50, state 1: with the queue empty
  and the idle entry marked, both motors off; else a marked top with 6
  vblanks or more left: its motors; either sent (scePadSetActDirect, state
  2), then waited on (scePadGetReqState, state 4) before state 1 looks again
```

The callers are `ccPlayer::DamageActuate` (Kite hit: the small motor and
`DamActuTbl` by the damage, 100 ms) and the Vibration menus switching it on
(the small motor and 160, 200 ms). The port is `crates/piney-input`.

## The screen

`ccSystem::Init` (0x0010a900) asks `SetScreenMode(512, 448, 0)`, applied by
`SetScreenModeMain` (0x0010ad40) on the next `Ctrl`. The mode is width
(+0xbd0), height (+0xbd2) and a field flag (+0xbd4, forced 0 at 256 lines
or fewer):

```
height <= 256       sceGsResetGraph(0, 0, 2, 1)   NTSC, not interlaced
field flag 0        sceGsResetGraph(0, 1, 2, 0)   interlaced, one 448 buffer
field flag 1        sceGsResetGraph(0, 1, 2, 1)   interlaced, 224 a field
```

then `sceGsSetDefDBuff` (the buffers at +0xbe0, their DISPLAY registers at
+0xbf8 and +0xc20), and keeps DISPLAY's DX and DY as the base (+0x1c,
+0x1e) before re-applying the offset. `sceGsSetDefDispEnv` (0x0010de50),
NTSC interlaced: MAGH = (2559 + w) / w - 1 (4 at 512, 3 at 640), DX = 636,
DY = 50; DH is 2h - 1 in field mode, h - 1 otherwise. So DX counts video
clocks (2,560 across the picture at either width) and DY lines of the
interlaced frame (one row of the 448).

`SetDisplayOffset(x, y)` (0x0010ab30) keeps (x, y) (+0x20, +0x22) and
writes base + x (0..4095) and base + y (0..2046) into both buffers' DISPLAY;
with the field flag 0 also into read circuit 1's pair (below), there with
y + 1.
Adjust Screen moves x by 3 (-48..48) and y by 1 (-16..16), so the picture
moves up to 9.6 pixels across and 16 rows down at 512 x 448. The port moves
the presented picture by x / 5 pixels and y rows and leaves black where it
left (`piney_gs::Presenter::present`).

In frame mode (field flag 0) the picture is merged with itself a line
lower, a deflicker for the interlaced TV. `SetScreenModeMain` sets both
buffers' PMODE EN1 (libgraph's 0x66 already has EN2, MMOD 1, AMOD 1) and
ALP 127 (0x0010b4e8-0x0010b520), and copies their DISPLAY with DY + 1 to
+0xe10 / +0xe20. Each frame `SwapDoubleBuffer` (0x0010ac40) writes the
shown buffer's DISPFB to DISPFB1 (0x12000070) and its +0xe10 copy to
DISPLAY1 (0x12000080) before `sceGsSwapDBuff` puts circuit 2. So a screen
line is 127/255 of the buffer's line above and the rest of its own. Field
mode (the movies) skips both. The port merges only when asked
(`--deflicker`, `piney_gs::Pcrtc::deflicker`), since its picture is
progressive.

## Unknown

- How the kernel orders tasks of equal priority (the title's, the
  desktop's and the top page's two at 33): taken as their start order,
  not checked.
- The merge circuit's arithmetic: the port takes ALP / 255 as PCSX2
  does; the console's rounding is not measured.
