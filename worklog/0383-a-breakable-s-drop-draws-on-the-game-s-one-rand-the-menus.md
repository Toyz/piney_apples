---
number: 383
title: A breakable's drop draws on the game's one rand(): the menus take the area's generator, not a copy of their own from 1
date: 2026-10-06
area: ui, world, test
files: crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/menus/merchant.rs, crates/piney-fieldui/src/menus/inu.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/side_events.rs, crates/piney-game/src/session/tests/survey.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md, docs/engine/battle.md, docs/engine/dungeon.md, UNKNOWNS.md
---

# 383. A breakable's drop draws on the game's one rand(): the menus take the area's generator, not a copy of their own from 1

Issue #52 (ThreePendant, build 05b691e): the drops of the breakable
objects do not change on leaving and entering a room. However many objects
are broken, the n-th one always gives the same item. The reporter thought
this was not intended. The first question was what the game itself does.

## What the game draws

**The drop.** `ItemObjMenu` (gcmn 0x00546fd0, menu 34) makes two draws,
both newlib `rand()` (main 0x00133a38):

- at 0x00547144, `rand() % 3`. Not 0: the menu shuts with nothing;
- 17 frames later, `AreaItem` (0x00544df4) adds `rand() % 5` to the area
  list's index.

`EntryBreakObjectMain`'s `fieldrand(4)`, which picks the object's row, is
the dungeon's own generator. It plays no part in the drop.

**The seeding.** `rand()` starts at 1 at power-on. `srand` has one
caller in all four overlays and main, the staff roll
(`ccThStaffRollCtrl`, desktop 0x0041270c; `tools/disasm.py xrefs`, with
`--scan` finding no unrelocated call). Nothing reseeds it for an area, a
room or a load. `ccInitRand` seeds the other generator, `ccRand`.

**What else draws.** `rand` has 398 callers. Among those that run while
Kite stands in a dungeon room:

- `ccPlayer::AnimCtrl` (0x00599e14): his idle count, `rand() % 60` once
  it passes 450 frames;
- the party's AI: `ActInDungeon` (0x0057e550, 0x0057ea68),
  `FollowBeacon`, the `WalkingTalk` and `Neutral` lines, `ccFellow::Action`;
- the effects: `ccEffect::Main`, `ccParticleGenerator::Generate`, and the
  breakable's own debris (`effWoodFragment`, `effPotFragment`,
  `effBoneFragment`, `effEggshellFragment`);
- `Disp`'s noise;
- the menu task itself. Each `ccSetupGameCtrl` (every room of a dungeon,
  dungeon.md) starts a new `ccThMenu`, whose `new ccMenuCtrl`
  (0x0051c140, called at 0x00528114) makes a `ccNoiz` (main 0x001bb080).
  Its eight bands draw one `rand()` per row (`SetNoize`, main 0x00106320).

So on a console the drop depends on all that came before it: the time
Kite stood, the party's lines, the rooms entered. With Kite alone and
still, his idle count alone moves it about every 451 frames.

## The port

The draws were the game's, but not their source. `MenuCtrl` built its own
newlib generator from 1 (`newlib_rand`), and `FieldUi` is made once per
area visit. So the drops were the menu's own sequence, restarted at each
area: the n-th draw since entering the area always gave the same answer.
That is the report. PcMenu's lines had the same fault (`TalkState::rand`,
its own copy; `FieldUi::set_rand` was never wired). Entries 70, 71 and 73
had listed this as "random stream only".

## Fix

- **One source.** `ctrl::MenuRand` replaces the boxed closure.
  `Game(Rand)` is the game's generator. `Fixed` holds a harness's numbers.
  `FieldUi::run` and `answer_item` load it from `SaveState::rand` before
  the task's frame and store it back after.
- **The hosts.** `FieldWorld::with_live_state` and
  `World::with_live_state` lend the save with `rand` set to the mode's
  live generator (`Combat::rand`, the town's `rand`), then take back what
  was drawn. The area and the town step the menus through them.
- **The task's start.** On each scene's first task frame (`Phase::Play(1)`,
  every room), `FieldUi::menu_task_started` draws the noise's bands from
  the generator, as the new `ccMenuCtrl` does.
- **PcMenu.** Its two draws now come from the same source.
  `TalkState::rand` becomes `cc_rand`, NorainuMenu's stand-in for the
  town's `ccRand`, which it was all along (entry 142).
- **The probe.** `rand V..` feeds both the menu's `rand()` and `ccRand`,
  as the harness hooks the game's. A new `randlog` traces each draw in its
  frame.

## Checked

- **Against the game.** `tools/test_fieldui_rs.py` now traces every
  `rand()` in its frame (`randlog`) for ItemObjMenu (34), TrapObjMenu
  (35), ItemBoxMenu (32) and TrapBoxMenu (33), and compares them with the
  game's code. For 34 both sides draw at frames 27 and 44 (the item's
  path) or 27 alone. All the fieldui harnesses pass: rs (67), personal
  (17), shop (31), talk (33), trade (10), option (12).
- **`a_breakables_drop_draws_on_the_areas_rand`** (piney-game, shrine.rs).
  It sets area 26's generator to 110, 1, 113, 2 and 120 as ItemObjMenu
  opens. The item comes exactly when the first draw from that state is 0
  mod 3 (110, 113, 120), so nothing else draws between the open and the
  menu's draw. With the menu's own stream, every state gave the same
  answer, and the test fails at 110.
- **`a_breakables_drop_moves_on_with_the_time_waited`.** Kite stands 0,
  451 or 600 frames by the breakable before pushing X. The area's state at
  the open differs, and with it the drop: items 655364 and 655377, then
  nothing. Before the fix it fails on the first wait.
- **`the_menu_task_draws_its_noise_from_the_saves_rand`** (piney-fieldui):
  the save's state moves on by exactly the bands' draws, and a fixed
  source leaves it alone.
- **The autopilot.** The menus' draws (the noise's bands on every scene
  above all) move the battles' rolls. `gob3_1_golden_goblin_is_run_down`
  (Outbreak's GOB3-1) then stalled: Kite waited on goblins that fled and
  rushed the weakest standing one, which stood far off. The pilot now
  rushes the nearest that stands. The three fall by frame 16,959, so the
  run has 24,000. `gob3_4_golden_goblins_fall_after_a_shop` passes too.
- `infection_whole_story` runs to event 31's end (frame 417,900).
- The piney-game, piney-fieldui and piney-world suites, clippy, fmt and
  the docs check pass.

**Still unknown:** a room change makes a new `ccMenuCtrl` in the game.
The port draws only its noise's bands then. The rest of the
constructor's state (`panelAlpha` -48, `interNoiz` 0, `forbid` 0, the
protect marks, `mailCnt` 240) carries over a room in the port. What the
console shows anew after a room, and what the new scene's set-up puts
back, is not checked. An eemu run of `ccSetupGameCtrl` and the new
`ccThMenu` across a `ChangeScene` would settle it. Each `rand()` caller
is checked against the game's code on its own. The order of the draws
across a whole frame's tasks is not: that needs a console's `rand` state
(`*_impure_ptr + 168`) frame by frame, from a PS2 capture.
NorainuMenu's `ccRand` is still a stand-in (entry 142).
