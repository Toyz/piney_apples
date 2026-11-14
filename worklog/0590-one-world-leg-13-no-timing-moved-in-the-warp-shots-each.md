---
number: 590
title: One world, leg 13: no timing moved in the warp shots, each draw lit as the tasks before it left the lights, full checks and goldens
date: 2026-11-14
area: world, render, decomp, test
files: crates/piney-world/src/world/lights.rs, crates/piney-world/src/combat/lights.rs, crates/piney-world/src/world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/event/gate.rs, crates/piney-world/src/town/cast.rs
started: 2026-11-14T00:00:43-07:00
took: 3h 19m
---

# 590. One world, leg 13: no timing moved in the warp shots, each draw lit as the tasks before it left the lights, full checks and goldens

The last leg of the relay on the plan's "One world, one area system" item
(branch `one-world`, after [[589]]). Two open points settled from the
game's code, one of them a fix (c8f5f87); the full checks on the final
tip; every shot that moved since main attributed; the goldens made again;
the docs and the plan (11c9a56).

## event_11_warp_shots 0255 and 0260: no timing moved

[[589]] put the two arrival frames down to the shots' clock (Kite's act
12) moving with One Kite ([[581]]). Measured, nothing moved. Every newlib
`rand()` draw was logged with its frame and callers at main (b2d7ca6) and
at the tip (scratch/world3/l13: dbg_nl.py, nl-base.log, nl-tip.log): area
15's warp-in rings are made at the clock + 250, + 253 and + 256 in both,
the field's Reboot at the clock + 117 in both; the frame offset is 0. The
rings differ only in their turn: `effTransferRing` (INF main 0x001cdef0)
takes `rand() & 0x3f00`. The two runs draw the same calls in the same
order except 48 more in Mac Anu at the tip: 16 `ccEffPawSmoke` calls (two
`rand()`s at main 0x001ce8e0 and 0x001ce960, after `ccLandHitCheck` and
`checkHitResultAttlibute` find stone, and one in the particle's set-up)
from the party running during event 11 block 4, 15 by the member on
scene 1 coming up to Kite from the town's start and one by Kite. Main's
town dropped the party's paw smokes. The game raises them in every area:
`ccFellow::CheckNote` (INF gcmn 0x0041e1e0) calls `ccEffPawSmoke(this,
speed)` at 0x0041e3f0 on notes 1 and 2 when actType (+0xee) is 5, and
`ccPlayer::CheckNote` (0x0059c300) the same at 0x0059c528, neither with an
area test, and `ccEffPawSmoke` reads no area. The field's trade restock
then draws 92 rather than 102 (`set_trade_item_town` draws by the values:
`rand() & 1`). So the tip's rings are the game's; no code changed.

The clock itself starts 48 frames later at the tip (system frame 3094
against 3046). Logged frame by frame (dbg_fr.py, fr-base.log, fr-tip.log)
the two runs are in the same state up to frame 2820, where the test's
Triangle falls in `MenuClr`'s two-frame wait (`menuClrWait = 2`, INF main
0x001b2788, [[589]]): the test presses every 30 frames and its menu
presses every 8, which makes the two frames 48. That is [[589]]'s cause
for frames 0000-0110.

## What lights a draw sees

`ccModel::Draw` (INF main 0x0013eab0) lights a model through
`SetLightMatrix` (0x0013ec14) at the call, so each character is lit by the
group as the tasks before its draw left it. The port lit everything but
the party by the group as the whole frame left it. From the game's code:

- `ccThEntryCtrl` (64, gcmn 0x00431970) walks the enemies, then the
  circles, the gimmicks and the NPCs. `ccEnemy::main` (0x00432cd0) calls
  `dispEnemy` (0x00432f34; its `ccAnm::Draw` 0x00434a7c and `ccChar::Draw`
  0x00434a98) before the race's `exclusive()` (0x00432f48), where the
  weapon controller moves the flashes' lights. So an enemy sees the
  flashes of the enemies walked before it this frame, its own and the
  rest as the last frame left them, and the gimmicks', the cast's, the
  boss's and the boss effects' as the last frame left them.
- A circle or gimmick sees every flash this frame and the gimmicks' lights
  as the walk had moved them at its draw (the outs come in the walk's
  order, so the port snapshots them at each draw out).
- The NPC list (a field's event NPCs, a town's merchants, dogs, Grunties
  and walking PCs) sees the walk's and the Chaos Gate's lights this frame
  (the gate is on the object list). The gate's own `ccChar::Draw`
  (`ccChgate::main`, 0x00459348) comes before its circle's
  `_AnimateForward` (0x0045937c), so the gate's body sees the circle's
  light as the last frame left it.
- `ccThBossEffect` (66) moves the boss effects' lights before the boss's
  own task (docs/engine/boss.md), so only the boss sees them this frame.
- A frame whose tasks sleep draws nothing new: `ccSleepNoSleepThread`
  sleeps the tasks and the layers are frozen (event-vm.md, field-ui.md),
  so the picture is the last awake frame's, lit as it was then. The port
  redrew the party under the lights as the last frame left them.

`world::lights` (`LightSources`, `FrameLights`, `CastLights`) and
`combat::lights` (`GimmickLights`, `WalkLights`) hold this; Combat's three
light maps became one `GimmickLights`; `Gate::body_lights`. Unit test
`an_enemy_sees_only_the_flashes_moved_before_it`.

## Checks on the final tip

At c8f5f87 (the later commits change docs and the log only), all on
2026-11-14:

- The fifteen harnesses on all four volumes: 60/60 rc=0, every line as
  [[588]]'s; test_evchar_rs (39 scenarios, 12330 frames), test_evcam_rs,
  test_bosscam_rs, test_battle_kite_rs on all four volumes and
  test_battle_event_rs: all OK.
- The battle pins (world3/pinwt.sh): 290 calls over the 11 fights, 0
  differ.
- piney-game suite 345 passed, 0 failed, 152 ignored (one fewer than
  [[588]]'s 346: [[589]] removed a test of the town's old view); piney-world
  and the other crates' tests all pass (39 binaries); `gen --check`,
  clippy (workspace, all targets and features), fmt, `docs.py check`
  clean.
- Every golden shot at main, at leg 12's tip (b6680a6) and at this tip,
  the same day (world2/shots l13base, l13pre, l13tip; world3/l13/
  attrib.py, attrib.lists). 797 images differ from main by pixels, and 27
  are named differently (the macha shots' acts, the six
  mac_anu_chat_shots, two shrine rooms: as at [[588]]'s 477cf7f).

## Every moved shot attributed

An image can have moved in more than one interval.

- 427 are in [[588]]'s list (main to 477cf7f) and keep its causes:
  `ccSys.count` from power-on, the party lit by the last frame's lights,
  no trails while asleep, the particle task after the area's draw, the
  ground for `ccEffPawSmoke`, One Kite, the town party's command lists,
  the menu's two-frame wait. Its list left out shrine_map_shots: bisected
  (world3/l13/bis.sh), one image moved with 58ba3e6 (the particle task
  after the draw) and the rest with 9855a4d (the count).
- 384 moved between 477cf7f and leg 12's tip, all from 3f4bc47. 364
  field_shots frames 160 differ only in the minimap's arrow: the test puts
  the tasks to sleep for frames 150-169, and `ccThFieldDisp` sleeps with
  them, so `WORLD::DrawMiniMap` does not step its pulse; 20
  shrine_map_shots, whose fight went another way from one frame (1984):
  the menu task, asleep from the door's `ChangeRequest` (main 0x001671e0
  calls `ccSleepNoSleepThread`, 0x0015a200), no longer draws its noise's
  `rand()`s.
- 33 moved with this leg's lights: 11 at frames taken while asleep (ten
  field_shots at frame 160, event_11's "joined" during the party call), 3
  event_11_warp_shots (the gate's body before its circle steps), 19 with
  enemies or gimmicks in view (field_shots frames 120 and 250, field_type,
  shrine, dressed_room).

The goldens (world2/shots/base) are now this tip's; main's old ones are
base-b2d7ca6-1103.

## Found, not changed

The minimap pulses are function statics set once (`DrawMiniMap`'s init
guard at INF gcmn 0x005a3960, `r$3072`; `ROOTTOWN01::DrawMap`'s `r$1560`;
the other maps' alike) and never reset; the port starts each map's at 0.
Main does the same; it shows only across a change of scene.

**Still unknown:**
- the minimap pulses' phase across a change of scene: the game keeps each
  map's static from power-on, the port restarts it with each map
