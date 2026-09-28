---
number: 162
title: "Dun Loireag's Grunties and the Grunty foods: ccPGuso, ccGimFood and menus 43, 45 and 46"
date: 2026-09-26
area: world
files: crates/piney-world/src/grunty.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/body.rs, crates/piney-world/examples/grunty_probe.rs, crates/piney-battle/src/gimmick.rs, crates/piney-battle/src/entry.rs, crates/piney-effect/src/pg.rs, crates/piney-fieldui/src/menus/inu.rs, crates/piney-fieldui/src/menus/objects.rs, crates/piney-audio/src/driver.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/dun_loireag.rs, crates/piney-game/src/session/tests/shrine.rs, tools/test_grunty_rs.py, tools/test_fieldui_talk_rs.py, tools/test_battle_spawn_rs.py, docs/engine/town02.md, docs/engine/field-ui.md
---

# 162. Dun Loireag's Grunties and the Grunty foods: ccPGuso, ccGimFood and menus 43, 45 and 46

A GAPS item. Dun Loireag keeps a Grunty record per town in the save. From
it, `ccSetChibiGuso` places the young Grunty that walks the town and the
grown ones at the pens. Kite feeds the young one foods found in the
dungeons, and it grows up through four stages into a grown kind. The
port's town had none of this: no Grunties, menus 45 and 46 unported, and
the dungeons' foods lying there invisible, with menu 43 unported.

## The Grunties

- **Placement.** `ccSetChibiGuso` places a grown one at `DMY_cdog0`-`2`
  for each kind the record has. A record at level 4 with a kind free
  starts over; then the young one (rows 154-157, by level) goes to its
  route's first dummy. The grown ones are made before the record is
  cleared and read it as it was, so the port constructs them from a copy
  of the save taken before the reset.
- **The class.** `ccPGuso` (pgbreed.cpp, 0x330 bytes) is ported whole in
  `piney_world::grunty`:
  - the walk;
  - sitting and sitting up for the menus;
  - eating, and `paramCalc`'s record, next food and level;
  - the three growing-up acts (the stretch, the cross-fade to the next
    body, `effEvolvePG`);
  - growing into a grown kind (`adultSetup`, `evoActAdult`: its line, the
    run-off of a kind the town has, `effGrowPG`);
  - the placed grown ones' `adultMain`;
  - the three affect functions;
  - `moveCam`'s fixed camera and the smoke, sounds and voices.
- **`pgPtr`.** `inuCheckNote` acts on `pgPtr`, the last Grunty made,
  whichever Grunty's anm passed the note. The port keeps that.
- **The world.** `World` lists the Grunties after the dogs, as
  `ccEntryEventMng` makes them, under the row each was made from (its base
  changes as it grows). Affects reach the class at once. The camera writes
  and Kite's placing are carried out after the list.
- **The effects.** piney-effect's new `pg` holds `effEvolvePG` (the -7
  controller: generators 126 and 127 at count 12, 128 at 30, following the
  Grunty half its height up, from `posT`) and `effGrowPG` (generator 129).
- **Drawing.** The main anm is drawn through `ccChar::Draw`'s shadow.
- **The Grunty files.** Their first `CMP_trall` is an animation's, whose
  nodes are ExtObj copies of the model's objects. `Body::of` now follows a
  node's ExtObj copy to its target (as `ccMatchIndex` does). Without that
  the Grunty drew nothing. Bodies whose nodes are their own objects are
  unchanged: the world, shadow, foe and town harnesses still match.

## Menus 45 and 46

- **The menus.** `OtonainuMenu` (45), `InuMenu` (46) and `InuMenuDisp`
  are ported in `menus/inu.rs`: the fade to the fixed camera, the STATUS
  window, Talk, and Give Food into `BreedingMenu`.
- **The Grunty's line.** The affect functions change the Grunty's
  `msgNum` at once, and the game's menus read it straight after. The
  port's menus see the world's Grunty as the frame began, so
  `after_affect` makes the same change to their copy.
- **The runtime.** piney-game hands the menus the Grunty (`World::grunty`)
  and carries out their requests: Feed (affect 19), `growthNum`,
  `foodMode` and `chatFlag`. It also turns the Grunties' events into
  sounds, effects, voices (a grown kind's through `ccCheckVoiceGrp`) and
  chat lines.

## The foods and menu 43

- **The food.** Gimmick rows 22-37 (`FOOD_00`-`0F`, key items 26-41; 22
  the Golden Egg) are `ccGimFood`. `SetItemBox` puts them in the dungeons'
  box slots. The port made them as inert gimmicks that drew nothing.
  `gimmick::food_new` and `food_main` port it:
  - it wobbles and rolls, turns to Kite and calls out within 1000
    (`ccVoicePgFood`);
  - FoodMenu's affect 11 takes it (sound 179 on note 70), then it bursts
    (`effOpenBox`, sound 77, the bounce and fade);
  - it is drawn at `SetMatrix_PosRotZYXScale`.
  Its act and count are `ccGimmick`'s own `actNum` and `actCnt`.
- **The menu.** `FoodMenu` (43) is VirusMenu's shape. The item is
  `0xf0000 | (base->id + 4)`, and `foodCount` (+0x7446) goes up.
- **The voice.** `ccVoicePgFood` sets its own request from
  `voiceFoodTbl(E)` and `FOOD.BIN`. The tables (generated) carry it as
  `EvVoice::food`. The runtime asks for it as event -100, which
  `Audio::voice` routes to `Driver::food_voice_request`. The driver's
  `voice_request` still plays nothing for -100, as `ccVoiceRequest` does.
- **The riding Grunty.** The flute that calls one (key item 49) is given
  by `BreedingMenu` when a new grown kind appears. So Infection reaches
  the ride. It is not ported here: it is the next task.

## Checks

- **`tools/test_grunty_rs.py`.** Runs the game's `ccSetChibiGuso`,
  `ccPGuso`'s constructor and `main` in eemu over town02, beside
  `grunty_probe`, in 15 scenarios: the walk, talk, eating, each growing
  up (0 to 1, 1 to 2, 2 to 3, 1 to 3), a new kind, a kind the town has,
  one far from a kind, the chat lines, a fresh one fed to a grown kind
  over five meals, and grown rows 145-147. It compares every member, the
  anms, notes and draws, and the events each frame. The events now carry
  the height the effects read. All match.
- **`tools/test_fieldui_talk_rs.py`.** `GruntyPages` covers 45 and 46.
  All 33 tests match.
- **`tools/test_fieldui_rs.py`.** `test_food` opens 43 on bases 22, 29 and
  37. All 51 tests match.
- **`tools/test_battle_spawn_rs.py`.** The harness now runs the
  constructors of `ccGimFood` and `ccGimSymbol` natively (sizes 0x240,
  0x710), and reads their members. `item_box` compares the foods and
  symbols `SetItemBox` makes, and `gim_frame` compares food frames
  (`main`, `actRolling`) near and far, taken and not.
  - Before this, the harness stubbed the symbol's constructor while the
    probe ran the port's (worklog 0149). So `dungeon_mc` (the lakes' row
    18) and `item_box` (kind 4) failed.
  - All six tests now pass, and the symbol's constructor is checked
    against the game for the first time.
- **`tools/test_effect_misc_rs.py`.** `GruntyAgainstGame` runs
  `effEvolvePG` and `effGrowPG` on moving characters: 30 cases, 141
  generators, every slot each frame. It first caught `posT`.
- **Sessions.**
  - `the_grunty_eats_and_grows`: the young Grunty walks. `InuMenu` opens
    on it under the fixed camera and it sits. Three foods make it Little
    Grunty (row 155, size 6, its voice). Talk gives its line. Leaving
    brings the field camera and its walk back.
  - `the_grown_grunty_talks`: row 145 from a record with kind 0, which
    starts over; `OtonainuMenu` opens, Talk and back.
  - `a_grunty_food_is_picked_up`: a Golden Egg in area 26's dungeon calls
    out, FoodMenu takes it, it bursts, and key item 26 and `foodCount`
    go up.
- **Shots.** `grunty_shots` saves the walk, the menu, the change and the
  grown Little Grunty. `grunty_food_shots` saves three foods lying there
  and one taken.
- **The rest.** The workspace's tests pass, as do the world, town02,
  shadow, foe, sound, effect and field-UI harnesses.

**Still unknown:** The ride (`ccPgAdultCheck`, `ccPuccigusoStart`,
`ccPucciguso`, `pgRideFlag`) is not ported. A field's
`WORLD_MAN::EntryGimmick` (`SetFood`, and its magic portals and special
objects) does not run in the port, so only the dungeons' foods appear. The
Grunties' camera writes land after the entry control's list, a frame's
NPCs later than the game's. `runAway`'s one frame of
`checkHitResultAttlibute` is not compared (the harness does not model the
collision's last result). The growing-up particles and the foods' look are
not compared with the console's pictures. `ccGimSymbol`'s `main` is still
not run in eemu.
