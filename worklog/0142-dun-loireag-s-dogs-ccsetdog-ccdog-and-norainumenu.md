---
number: 142
title: Dun Loireag's dogs: ccSetDog, ccDog and NorainuMenu
date: 2026-09-26
area: world
files: crates/piney-world/src/dog.rs, crates/piney-world/src/lib.rs, crates/piney-fieldui/src/menus/inu.rs, crates/piney-fieldui/src/talk.rs, crates/piney-game/src/world.rs, crates/piney-game/src/town_fx.rs, crates/piney-game/src/session/tests/dun_loireag.rs, tools/test_fieldui_talk_rs.py, docs/engine/town02.md, docs/engine/field-ui.md
---

# 142. Dun Loireag's dogs: ccSetDog, ccDog and NorainuMenu

A GAPS item. In the game, Dun Loireag has four stray dogs running about
the town. The port's town had none, and its action button closed menu 44
at once.

## The game

- **Placing them.** `ccEntryEventMng` calls `ccSetDog` (gcmn 0x00509f30)
  in town 1, after the merchants and the Chaos Gate. It makes four
  entries from `npcTbl` rows 141-144 (Johnny, Sal, Lottel, Suzie; base
  type 0x04000000, `CDOGBOD1.CCS`). Each starts at the first dummy of its
  route in `markPosTbl` (0x005ee220). The routes are lists of town02's
  `DMY_markerNN`.
- **The class.** `ccDog` (a `ccGimmick`, 0x240 bytes) runs to its next
  dummy. It turns a sixteenth of the way each frame and steps only once
  it faces within 4096 of the target. At a dummy, `rand() & 1` picks a
  run (12 a frame) or a walk (1.5). A change of gait goes through 101
  frames of idling (act 5). Touching another body turns its step aside
  for a while. When it has not moved 50 in 120 frames, it goes back to
  the last dummy.
- **Notes.** `inuCheckNote` plays `ccSeSetParamInu` for a note's param:
  1 is a step, by the ground; 3 and 5 are barks. On param 1, a run also
  kicks up `effSmoke` dust behind it.
- **Talking.** `dogAction` is its `affectFunc`: `EntryAffect` 14 makes it
  sit facing Kite (act 1), 15 sit up (act 2), and 0 walk on (act 4).
- **The menu.** The action button on type 0x04000000 opens `NorainuMenu`
  (0x0054a680). Its list is Talk alone. The first frame greets (the first
  record of `base->msg`, with the dog's voice group) and picks Talk's line
  from `ccRand()`: records 1, 3 or 5. Cancel sends the dog off. Its
  structure is BreederMenu's, including the keys still read in the frame
  after a lost target's close.

The same data puts the young Grunties (rows 154-157) on `InuMenu` (46:
Talk, Give Food) and the grown ones (145-153) on `OtonainuMenu` (45:
Talk, Trade). That is the next piece.

## The port

- **The dogs.** `piney_world::dog` holds `set_dogs` and `Dog` (with
  `EntryObj::routine`, the merchants'). `World` keeps them after the
  merchants, on the NPC list the targeting reads, so the action button
  finds them.
- **Sounds and dust.** Their notes come out as `DogEvent`s. `piney-game`
  plays them through `se3d::inu_note`, which was already there, and a new
  `town_fx::Start::Smoke`.
- **The menu.** `piney_fieldui::menus::inu::norainu_menu` is menu 44, and
  the session now tells the menu who a dog is (`talk_to`).
- **Checks.** `tools/test_fieldui_talk_rs.py`'s `DogPages` runs the
  game's `NorainuMenu` in eemu beside the port. It covers two dogs'
  greetings and cancel, Talk with each `talkNum`, the target lost under
  the list with each key, and no target. Every frame matches. The
  harness hooks the game's `ccRand` to the scenario's values, as it does
  `rand()`.
- **The session.** `the_dogs_walk_their_routes_and_sit_to_talk` runs the
  town for 20 seconds, and every dog reaches its next dummy. It then puts
  Kite before Lottel and presses the action button. The menu opens, the
  dog sits, Talk shows its line, and cancel sends it walking on.

**Still unknown:** the dogs' walks are not compared with the game frame
by frame. `ccDog`'s constructor leaves the stuck count (+0x21c) unset;
the port starts it at 0. The port's menu draws `talkNum` from `rand()`,
not the town's `ccRand`.
