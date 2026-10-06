---
number: 384
title: A room's menu is made anew after its passes, and keeps gcmn's lists: the noise draws before the party's boot, and PERSONAL's cursor stays from room to room
date: 2026-10-06
area: ui, world, test
files: crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/revive.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md, docs/engine/dungeon.md, docs/README.md
resolves: 383
---

# 384. A room's menu is made anew after its passes, and keeps gcmn's lists: the noise draws before the party's boot, and PERSONAL's cursor stays from room to room

Entry 383 left one question open. The game builds a whole new
`ccMenuCtrl` on every room change. Did the port's `FieldUi` carry the
rest over (`panelAlpha` -48, `interNoiz` 0, `forbid` 0, the protect
marks, `mailCnt` 240)?

## The premise

It did not. The session makes a new mode for every scene, a dungeon's
rooms too (`Session::change_scene`, then `AreaMode::enter` or
`WorldMode::enter`). Each one builds its own `FieldUi`, and with it
`MenuCtrl::new`, the constructor's port. Every member the question named
was the constructor's in each room already. Entry 383's "made once per
area visit" meant per scene. Reading the game's set-up did turn up three
real differences.

## What the game does

`ccSetupGameCtrl` (main 0x00168960), for every scene:

1. After the fade out, `ccDeleteAllThread`. The menu task's delete calls
   follow: `ccThMenuDeleteInstant` (gcmn 0x0051c110, `StillOff`) and
   `ccThMenuDelete` (0x0051c0e0: `~ccMenuCtrl` 0x0051cc00, which also
   deletes `ccMsg` and `ccChat`, then `ccMenu = NULL`).
2. The event passes at phases 0 and 2, with no menu. `MenuBan` and the
   other writers in `Execute` do not test `ccMenu`. A pass that wrote to
   it would write through NULL.
3. `ccStartThread(ccThGameCtrl)`, `ccStartThread(ccThMenu)`, then a
   breath (0x001693e8). The mother runs at priority 2, and
   `ccTscb::GoThread` starts a thread at once. So `new ccMenuCtrl`
   (0x0051c140) runs in that breath, with its `ccNoiz`'s 216 draws.
4. The next frame: `WORLD_MAN::GO`, `ccGetStartPositions`,
   `rebootSpcManager` (`ccSPC::Reboot`, whose `SetParty` fills
   `ccPartyManager.memberChar`), `ccEnableThEvent(4)`, the fade in. Then
   the tasks run in order. The menu's party check passes and it breathes.
   Its first `Disp` comes the frame after.

The constructor sets every member from +0x00 to +0x104, `mailCnt`, the
protect marks, `subTarget` and `dummyTarget`. In a town it also sets
`ccGame.areaLevel` to 0. It leaves `mode`, `streamFlag`, +0x106-0x129,
`temp`, `drainItem`, `itemNum` and `trapNum` as the heap gives them.
`menuList[89]` (0x0072efe0) is gcmn's data, not a member.
`InitMenuList` (0x0051ced0) writes only each list's element pointers,
`disp`, `x` and `y`. So `select`, `page`, `index` and the scroll stay
as the last menu left them. gcmn is loaded at the log-in
(`ccSetupNewGame`, mode 5) and stays resident through mode 6. `BookOfs`
(0x005d3530) is gcmn's too.

## What differed

- **The noise's place in `rand()`.** The port drew the bands at the
  scene's `Play(1)`. That was two frames after `FieldWorld::reboot`
  (GO, start positions, `rebootSpcManager`), which drew 7 numbers in area
  26. The game draws the bands first. Who got which numbers was swapped.
- **The passes.** The port's menu existed through the passes, so a pass
  writing to it would have left its mark. The game's had none.
- **The lists.** Every scene began with `texts.lists`, so PERSONAL's
  cursor went back to the top in each room. The console keeps it.
- **A frame behind.** The port first stepped the menu at `Play(1)`, the
  world's first full frame. That step was only its party check, and its
  first `Disp` came at `Play(2)`, one frame after the world's and the
  fade in's.

## The fix

- `FieldWorld::tasks_start` and `World::tasks_start` name the frame whose
  step ends the hold. The hosts call `FieldUi::menu_task_started` then,
  before the world's step boots the party. It builds a whole new
  `MenuCtrl` (`MenuCtrl::with_rand`) on the game's generator, faces from
  the party's ids (`ccCheckMenuFaceNameParty`). `MenuCtrl::keep` brings
  over what is not a member: `MenuGlobals` (the lists, with
  `InitMenuList`'s three fields reset, and `BookOfs`), `WORLD_MAN`'s
  area, `ccSaveSys` with the cards, and `ccRand`.
- `Session::change_scene` hands the old scene's `MenuGlobals` to the new
  scene's `FieldUi`. A log-in or the desktop starts from the tables.
- The menu task now runs from `Play(0)`: the party check there, the
  first `Disp` with the world's first frame.

## Checked

- **Against the game.** `tools/test_fieldui_rs.py`'s
  `test_a_new_scenes_menu`, in a town and a dungeon. The harness opens
  PERSONAL, moves its cursor, bans the menu and sets `interNoiz`. At
  frame 70 it calls the game's two delete functions and starts a new
  `ccThMenu`; the probe calls `menu_task_started` (`restart`). Every
  frame compares every constructor member (`ctorfields`, 79 values),
  the old state and the packets. The panels fade in from -48 on both
  sides, and PERSONAL opens again with its cursor on the second row. The
  probe doing nothing at the restart fails at frame 70. The harness
  needed `ccDamUprStr::ResetAll` hooked, and kept the old menu's sprite
  names and message rows. All six fieldui harnesses pass (170
  scenarios).
- **The members the constructor leaves.** The six harnesses were run
  once with the menu's 0x240 bytes filled with 0xa5. The harness's own
  `openReqNum` writes then had to set `mode` as `ccThGameCtrl`'s twenty
  openers do (`sh $zero, 22`). Every scenario matched but one value:
  `trapNum` after a side effect other than 29. The harness hands that to
  the port from the game's memory. That patch was not kept.
- **`a_rooms_menu_is_made_anew_after_its_passes`** (piney-game). It walks
  area 26 through rooms (0,0), (0,1) and (0,6), and writes `forbid`,
  `interNoiz`, `panelAlpha` and `mailCnt` in every pass frame. At each
  room's `Play(0)` the bands must equal `Noiz::new` from the last frame's
  `rand()`, those members the constructor's, and PERSONAL's cursor what
  the room before set. After `Play(1)` the panels are at -24. Before the
  fix it failed: the passes' values stayed, and the bands came from
  elsewhere.
- `a_new_scene_makes_the_whole_menu_anew` (piney-fieldui): the members,
  the kept data and the faces.
- `the_fallen_keep_no_condition_marker`'s second case relied on Kite's
  blow felling Mia under the old order of draws. It now tries a dozen
  states of `rand()` and keeps the first where his blow does.
- The piney-game (252), piney-fieldui and piney-world suites, clippy,
  fmt and the docs check pass.

**Still unknown:** whether the order of the draws across a whole frame's
tasks matches a console's. Each caller is checked against the game's
code on its own, and the set-up's order now follows its code. Checking
the whole needs a PS2 capture of `rand`'s state (`*_impure_ptr + 168`)
frame by frame. NorainuMenu's `ccRand` is still a stand-in (entry 142).
