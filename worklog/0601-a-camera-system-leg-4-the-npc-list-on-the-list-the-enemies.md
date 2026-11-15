---
number: 601
title: A camera system, leg 4: the NPC list on the list, the enemies' fade at dispEnemy, the cinema per volume, the Kyvia wave, a story circle's stack w
date: 2026-11-15
area: engine, world, battle, test
files: crates/piney-world/src/systems/npcs.rs, crates/piney-world/src/grunty/mod.rs, crates/piney-world/src/chara/char.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/cinema.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/gimmick.rs, tools/test_grunty_rs.py, tools/test_cinema_rs.py, tools/test_battle_spawn_rs.py, tools/test_battle_kyvia_rs.py
started: 2026-11-15T02:34:08-07:00
took: 3h 6m
---

# 601. A camera system, leg 4: the NPC list on the list, the enemies' fade at dispEnemy, the cinema per volume, the Kyvia wave, a story circle's stack w

Leg 4 of the camera relay (plans/rust-cleanup.md section 2, "A camera
system"; branch `camera-system`, after worklog 600). It did step 6 (the NPC
list asks the list) and leg 3's two found items, the cinema harness on
every volume, the Kyvia core wave, the story circle's unwritten `w`, and
step 8's docs. Tarvos's phantom view is settled from the code but not
modelled. The full checks and the pins are the last leg's.

## The NPC list on the list (step 6)

`NpcCtx` carried a copy of the active camera (`chara::char::View`) and a
`&Cam`. A Grunty's camera writes went out as `GruntyEvent::{Camera, CamPos,
CamView}` and the world applied them after the list. Now `NpcCtx.cams` lends
camera.cpp's list:

- `Char::fade` reads the active camera at the call: its eye and pitch
  (`ccGetCameraTransparency`) and its type (`checkCameraType()` at gcmn
  0x0056b448). The merchants' and walking PCs' view cones do the same.
- A Grunty's `changeCamera` and `cameraSetPos/View(camID)` write the list at
  the call. The calls are the same on every volume: dogAction 0x0050f520,
  dogActionAdult 0x0050fd10, moveCam 0x00510634, main's eating 0x0050b778,
  evoActPon1 0x0050d21c and 0x0050d2bc, evoActAdult 0x0050e5dc, and from
  Mutation on the raced one (MUT 0x005283bc). The NPCs after it in the list
  see the write.
- The xrefs of the four camera.cpp setters on Infection show that only the
  Grunty writes a camera at priority 64.
- Each camera write in a Grunty's `main` pushes `GruntyEvent::Ear`, which
  goes out as `Show::Ear`, so the sounds after it pan by the camera it left.
  An affect's writes (the menus, at 34) push none: no 3D sound follows one,
  and the battle's first mark comes after `cameraMain` anyway.

tools/test_grunty_rs.py now runs `changeCamera`, `cameraSetPos` and
`cameraSetView` natively on the game's own cameraList. It compares camID
and tcam's and ecam's eye and target every frame, and with each 3D sound the
active camera at its call (eye, target, type, which is what calcPan reads).
With the Grunty's marks ignored it fails at eat frame 74 (a burp after
`moveCam`).

## Leg 3's two found items

- **The enemies' fade.** `dispEnemy` (gcmn 0x004348b0) runs from
  `ccEnemy::main` in ccThEntryCtrl's walk at 64. Its `ccChar::Draw` reads
  the active camera then. The port built `foe::Camera` after the boss's
  task, from the camera as `CamMain` left it and with camera 1's type. Now
  `Call::Draw` carries the enemy's `setTransparency`, and the stage draws
  it there (`Stage::foe_camera`).
- **The eye view.** Only Kite's `DispConditionEffect` reads it: a party
  character of base id 0, `checkCameraType()` at gcmn 0x0056f9f0. It comes
  from his `CalcReal` at 49, before his frame's camera writes. So it is now
  the active camera's type as the tasks start, where it was camera 1's.

## The cinema on every volume (task 4)

test_cinema_rs failed from Mutation on for two reasons. The probe named
cinema 2 as Infection's table has it. Behind that was a real difference:

- **The range.** `CinemaOn` reads rows 0-60 of `_g_cinemaSkillName` on
  Infection (gcmn 0x0046a9c8) and 0-78 from Mutation on (MUT 0x00480648,
  OUT 0x0047bfd8, QUA 0x0036e8e8). The table we read has 72 rows. Every
  `OnCinemaMode` call site was listed, and none passes 72-78 (70 is the
  largest), so `cinema::takes` is enough.
- **The outro.** From Mutation on, going out, the bars hold where they were
  while the name fades by 0.1 a frame. Only then do they slide (MUT
  0x00480ddc, OUT 0x0047c7b4, QUA 0x0036f0c4). The port slid them at once
  with Infection's 1/15. `Cinema::hold_out` fixes that.

cinema_probe now takes the volume. test_cinema_rs compares `SetTex`'s file
and texture with the volume's row, and passes on all four volumes.

## The Kyvia core wave (task 2)

`test_core_wave` failed on Outbreak, Quarantine and Quarantine's fourth
fight, and the same on main b1a9d90 (checked in a scratch worktree, since
removed). The cause is in the harness. From the second fight on, the
cases' random hits on the core kill it before frame 250 (its +8 is 1). The
harness forces no act on a dead core, on either side, so the wave never
came. The wave cases now keep only the gomoras' hits before it, and pass on
Mutation, Outbreak (fights 1 and 2) and Quarantine (3 and 4).

## A story circle's stack w (task 3)

`DUNGEON::SetMagicCircle` takes a story circle's `w` from a stack word it
never writes. The port used 1.0 and the harness stuffed 1.0. That word is
what the set-up before it leaves there:

- **Infection and Mutation.** The word sits 4 below EntryGimmick's sp
  (ccThEntryCtrl's frame 112, EntryGimmick's 32). That is
  `initEntryCCS`'s sp+268. SetItemBox's sp+476 is padding past its
  12-byte `BLOCKOBJECT_INFO`.
  - For each enemyTbl row with `entry.exist`, initEntryCCS does
    `sd @1781, 264(sp)` (INF gcmn 0x0042ff74, MUT 0x00445904). @1781 is
    `{13, the CCS name buffer}`, so `w` is the buffer's address (INF
    0x006a7b78, MUT 0x006dd8c0).
  - With no row existing, it is the upper word of a register `_ccMalloc`
    saved, which is 0. No code on any volume writes a register's upper half
    except by saving and restoring it: main and gcmn were scanned for `lq`
    from another base and for MMI writes. Infection's and Mutation's only
    hits are the exception unwinder and Infection's TLB handler.
- **Outbreak and Quarantine.** The word is SetItemBox's entry `pos[3]`
  (sp+396) as its last entry left it:
  - the last box slot's `w`;
  - then each edit row's (x, y, 0, sp+380), with sp+380 zeroed after the
    first row's (OUT 0x005e89f0, QUA 0x004db450);
  - a warning's door position over that.

  Every edit table with circles has 15 rows ending in a circle: event 14's
  on every volume, and Infection's 83. So the word is 0.

`entry::story_circle_w` and `gimmick::item_box_w` model it. The spawn
harness's `dungeon_mc` runs `initEntryCCS` and `SetItemBox` natively at
those depths on a fresh stack, then puts the word they leave under
`SetMagicCircle`. On Infection the word was the pointer in 129 of 150
cases and 0 in 21. With 1.0 stuffed there, the check fails (57 of 150).

**Tarvos's phantom view** is settled but not modelled:

- The constructor's sp+144 holds `$s1` as `_ccMalloc` saved it, under
  operator new in `ccBossEffPhantomCreate`. That is OnThinkPhantom's
  target: `plw.pw`, Kite's ccPlayer (QUA gcmn 0x003aaedc), or
  `SelectTarget`'s (0x003aaf2c).
- So the view is [the target's heap address as float bits, 0, `$s1`'s upper
  half]. The upper half is 0 by the scan above.
- The address is what `_ccMalloc` gave the ccPlayer in `ccPlayerStart`. The
  port keeps no heap addresses.
- It shows only in camera 2's view for the phantom's first frame.
- The harness's `install_phantom_view` stays. The relay's handoff assigns
  it.

## Also fixed where touched

test_foe_rs ran on no volume, for three reasons:

- `rnd_scene`'s `nn` went with c4f70b5;
- Outbreak and Quarantine name no `AwakeDistantLight`;
- foe_probe gave every volume Infection's sqrt.

It passes on all four now.

## Checks

On e6977e1, 2026-11-15 (scratch checks-l4.sh):

- **Harness:** leg 3's 24 and test_cinema_rs, test_foe_rs,
  test_battle_spawn_rs and test_battle_enemy_motion_rs, four volumes each:
  111 of 112. The one failure is test_battle_enemy_motion_rs on Mutation,
  in `mainedge`: at frame 2, call 74, the port makes a `weaponNote` where
  the game makes a `ccEntryFlyFontNew`. It fails the same way on main
  b1a9d90. It is not a camera matter, and the handoff assigns it.
- **Boss harnesses:** the eleven, 18 of 18, the Kyvia core waves among them.
- **Golden shots:** 4550 files. Six differ against the base, and the same
  six against main shot on 2026-11-15. All six are Macha's fight, and the
  only difference is the cinema's bars and name, held during the outro as
  Quarantine's `Draw` does (QUA 0x0036f0c4).
- **Battle pins:** "probe: .../battle_probe (2026-11-15 04:42:11), tree:
  /mnt/data/claude/wt-camera at e6977e1". 78 of 290 calls differ, all of
  them the boss-frame requests whose shape changed in leg 3. Every other
  call is byte-identical.
- **Tests:** crate tests 42 ok. The piney-game suite, uncapped, gives 348
  passed, 0 failed, 152 ignored.
- **Other checks:** gen --check, clippy, fmt, cairns and docs are clean.

**Still unknown:** the phantom's first view: the heap address `_ccMalloc` gave Tarvos's target (the port keeps none) and `$s1`'s upper half in the boss thread's first context (taken as 0)
