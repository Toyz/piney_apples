---
number: 602
title: A camera system, leg 5: Tarvos's phantom's first view is its target's heap address, the full checks, the battle pins recorded anew
date: 2026-11-15
area: battle, engine, test, tooling
files: crates/piney-battle/src/boss/tarvos/eff.rs, crates/piney-battle/src/boss/tarvos.rs, crates/piney-battle/src/chara.rs, crates/piney-world/src/combat/boss.rs, crates/piney-eemu/examples/ee_run.rs, crates/piney-game/src/session/tests/tarvos.rs, tools/test_battle_tarvos_rs.py
resolves: 601
started: 2026-11-15T05:41:52-07:00
took: 2h 38m
---

# 602. A camera system, leg 5: Tarvos's phantom's first view is its target's heap address, the full checks, the battle pins recorded anew

Leg 5, the last of the camera relay (plans/rust-cleanup.md section 2, "A
camera system"; branch `camera-system`, after [[601]]). It settles what
[[601]] left about Tarvos's phantom view, runs the full checks on the tip,
records the battle pins anew and ticks the plan item.

## Tarvos's phantom view is the target's address, and it shows

[[601]] found that `ccBossEffPhantom`'s constructor (QUA gcmn 0x0037a4b0)
hands `SetFreeCamPosView` a view it never writes, its sp+144. `_ccMalloc`'s
frame under `operator new` in `ccBossEffPhantomCreate` (0x0037dca0; new
tail-jumps to `_ccMalloc`, main 0x001006e0, frame 144) saved `$s1` there
with `sq $s1, 16(sp)`, and `$s1` is OnThinkPhantom's target: `plw.pw`
(0x003aaedc) or `SelectTarget`'s (0x003aaf2c). The view is [address, 0,
`$s1`'s upper half]. The question for this leg was whether the address could
change anything, or whether 0.0 is exact.

**The range.** `ccHeap::InitFirst` (QUA main 0x00100390) takes the region
from the C library: `ee_run` reads it as 0x00670310 to 0x01ff3ff0 on
Quarantine. As float bits an address below 0x00800000 is a denormal, which
the EE's FPU and VU0 read as 0. From 0x00800000 it is a normal float,
2^-126 or more (exponent field 1 to 3).

**The readers.** `CamMain` (gcmn 0x00363810), locked, copies the view to
camera 2 with `cameraSetView(view, 2)`, a plain `lq`/`sq`. The next writer is
the next frame's `CamMain`, so camera 2 holds the address from the boss's
task at 66 to the next frame's 66: `cameraSet` at 49 builds the frame's
matrix from it (camID 2: the view copied, then `ccTransPosFW2LW`, then
`SetMatrix_PosTarget`), and everything that asks `activeCamPtr` or
`cameraGetRot(r, camID)` reads it (WORLD::Draw's haze turn among them).
`cameraGetRot(r, 2)` (main 0x00160580) was run in eemu with camera 2's eye
random within 2,000 and the view's x an address against 0 (scratch
`l5/exp2.py`):

| addresses | differ from 0 |
| --- | --- |
| 0x00670320-0x007fffff | 0 of 2,000 |
| 0x00800000-0x01ff3ff0 | 571 of 2,000 |

A tiny normal minus an eye's positive x truncates one step toward 0 (the
EE's add, `piney_data::field::ee::add`), so the angle moves by an ulp. So 0.0 is the game's view only if the target lies in the
region's first 1.6 MB.

**Where the party lands.** `ccPlayerStart` (gcmn 0x004b2900) and the
members' starts (`ccSpcStart`, gcmn 0x0057b5a0, `new(592)` each) make the
objects as the field loads. `ee_run` ran Quarantine with the save before
event 310's fight (new: `session::tests::tarvos::event_310_save_bytes`,
blocks 0-6 played), town 4 (`0,4,-1,-1,-1,-1`), the members invited, then
`area 7` and `scene 1 4 7`, the heap model placing every handout:

| history | Kite's `ccPlayer` | BlackRose (15) | Balmung (6) | handouts |
| --- | --- | --- | --- | --- |
| both, save at 900 frames | 0x00dd1070 | 0x013042f0 | 0x0130ae40 | 73,174 |
| Kite alone, 900 | 0x00dd0ea0 | - | - | 60,590 |
| both, 1,440 | 0x00dd1270 | 0x01303ce0 | 0x0130a730 | 77,238 |

Every one is a normal float. So the outputs are not identical, and the view
moves with the play as Gorre's manager does ([[436]]). The way into the
arena was not varied: the story walks area 102's dungeon first, and
[[436]] found a field's placement did not follow the way there.

To see the phantom itself in the whole program, `ee_run` got two steering
knobs: `EE_PRINT`'s `name@k` prints the four words argument k points at,
and `EE_POKE=vblank:addr:value:mask` sets a word's bits at a vblank. Act 6
(the phantom; a first try forced act 21, which is the Transfer) forced at
ten vblanks from the fight's start reached
`OnThinkPhantom` each time, but its target checks found no one
(`ccCheckTarget` of Kite failed, then `SelectTarget` returned 0), so it made
no phantom there. The harness below runs the game's own `_ccMalloc`
instead.

## The port

- `piney_battle::chara::Env::party_va`: `ccParty.member[k]`'s values, where
  `_ccMalloc` put each member's object.
- `piney_world::combat::boss::PARTY_VA` = [0x00dd1070, 0x013042f0,
  0x0130ae40], the first history's, which the world puts in `Env` each
  frame. The port has no game heap; like `HEAP_EXIT` it keeps one history.
- `on_phantom` finds its target's slot and `eff::Phantom::new` takes the
  address: `b.free_cam(cx, pos, [va, 0, 0, 0])`.

tools/test_battle_tarvos_rs.py:

- `install_phantom_view` (the hook that wrote zeros) is gone.
- Its operator new stand-in now leaves `_ccMalloc`'s register saves (s0-s7
  by `sq`, ra by `sd` at +128, 144 under the caller's sp; operator new[]'s
  own 16 bytes first), the words the constructor reads.
- The probe takes the members' addresses (SCN + 0x1000 k).
- `test_phantom_first_view` forces act 6 with operator new, new[], malloc
  and DIntr unhooked, so the game's own `_ccMalloc` runs on `InitFirst`'s
  region (malloc answering 0x00670308, the block the boot's lies at). Camera
  2 holds Kite's address, 0x01100000, for the frame, and the port matches
  every frame. With the probe's addresses 0 it fails at frame 20 (`cams
  [2][1][0]: game 17825792 port 0`).
- `bulk 300`: 300 cases, 913,177 frames, all agree, act 6 among those
  reached.

`boss::tarvos::tests::the_phantom_rings_and_bursts_its_fourteen_lights`
checks the free view is the address it was given.

## Checks

On the code tip f75fa65, 2026-11-15 (scratch `checks-l5.sh l5`):

- **Harness:** leg 4's 28 harnesses on four volumes (leg 1's 23, which
  hold the full 15-harness set, with test_listener_rs, test_cinema_rs,
  test_foe_rs, test_battle_spawn_rs and test_battle_enemy_motion_rs):
  111 of 112. The one failure is test_battle_enemy_motion_rs on Mutation in
  `mainedge`, the same as on main b1a9d90 ([[601]]), not a camera matter.
- **Boss harnesses:** the eleven, 18 of 18.
- **Golden shots:** 4550 files. Six differ against the base and the same six
  against main b1a9d90 shot this day (`main-1115`): Macha's fight, the
  cinema's outro held as Quarantine's `Draw` does (QUA 0x0036f0c4, [[601]]).
  Nothing from the phantom: no shot is in Tarvos's arena. The base is the
  tip's now; the old one is kept as `base-b1a9d90-1115`.
- **Tests:** crate tests 42 ok; the piney-game suite, uncapped, 348 passed,
  0 failed, 153 ignored (152 and `event_310_save_bytes`); gen --check,
  clippy, fmt, cairns and docs clean.

## The pins

The old set (scratch `pin/`, 290 calls recorded on main) replayed against
the tip: 78 differ, all the boss-frame requests whose shape changed in
[[600]]; every other call is byte-identical.

The set recorded anew from the tip into scratch `pin-new/` (the same eleven
harness runs as scratch `pinall.sh`, the commands written as main's):
291 calls, the one more `test_phantom_first_view`'s. Replayed
(`pins/pins-new.sh`): "probe: /mnt/data/claude/targets/wt-camera/release/
examples/battle_probe (2026-11-15 07:20:56), tree: /mnt/data/claude/
wt-camera at f75fa65", 291 calls, 0 differ.

**Still unknown:** `$s1`'s upper half in the boss thread when the phantom is made (the view's z and w): what the kernel's StartThread put in the thread's first context, the BIOS's code, not on the disc; taken as 0
