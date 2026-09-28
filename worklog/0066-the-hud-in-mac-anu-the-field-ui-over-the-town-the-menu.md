---
number: 66
title: The HUD in Mac Anu: the field UI over the town, the menu buttons, and talking
date: 2026-09-23
area: ui, world
files: crates/piney-game/src/world.rs, crates/piney-game/src/main.rs, crates/piney-world/src/talk.rs, crates/piney-world/src/lib.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/render.rs, docs/engine/field-game.md, docs/engine/field-ui.md
---

# 66. The HUD in Mac Anu: the field UI over the town, the menu buttons, and talking

The user reported no HUD in Mac Anu. [[64]] ported the field UI and
[[65]] the town's people and the command target, but nothing put the two
together. The runtime now does that, and the rest of `ccThGameCtrl`'s
buttons are ported, so the menus open as in the game.

## The field UI in the runtime

- **Order.** `piney-game`'s world mode steps the world, then the field UI,
  from the town's first task frame on.
- **One frame.** Both draw into one frame, the HUD under the world's fade:
  `World::step_into` and the new `FieldUi::step_into` / `render::draw`.
- **What the UI reads** (its `World`):
  - Kite in party slot 0 from `spcParam[0]`, named by `plName`. The save
    keeps no current HP or SP, so he shows full: 63/63 and 13/13 on a new
    game.
  - The command target and the one before it, with the world's
    `ccCalcTagPosChar` points.
  - `cmndSortRoot`'s chain.
- **The face.** Found in the constructor: `ccCheckMenuFaceNameParty`
  (gcmn 0x0056a930) gives member 0 face 18 until the bracelet's colours
  (saveData+0x6771). The event opcode's `SetMenuFace` is the only other
  path. The runtime sets it at entry.
- **What comes back.**
  - Sounds and voices go to the sound driver.
  - `SleepAll` and `WakeAll` put the world to sleep and wake it.
  - An unported menu is named in the window title.

## The rest of `ccThGameCtrl`'s buttons

Read from gcmn 0x00517ff0-0x00518a98 and added to
`talk::Targeting::step`, in the game's order after the target:
1. the menu ban (chat only, with `forbidChatExcept`);
2. chat (3);
3. option (12);
4. the held conditions (sleep, paralysis, confusion, charm);
5. personal (0 in a town; fields and dungeons by number and floor type);
6. dead;
7. the action button.

Each goes through its `ccEvent::CheckOperate` number. Three guards come
first:
- `ccPlayerMenuCheck` (gcmn 0x0059cd70);
- no menu open;
- `openReqNum` not 74.

**How the menus open.**
- **By a button.** The chat, option and personal buttons open with `mode`
  0, so the town sleeps.
- **By the action button.** Its menus open with `mode` 1: the town keeps
  moving while Kite is held. The exceptions are a party member outside a
  town and a fountain.
- **Closing.** Every state waits for the menu type to return to -1, then
  releases Kite.

The map button (SELECT) is left out: it needs the minimap.

## A world start mode

`piney-game --mode world` starts in Mac Anu on a new game's save, with the
player named Kite. `--press` now takes held ranges and the left stick
(`200-310:lup`), so a shot can walk.

## Checked

- **Unit tests.**
  - `menu_buttons_in_the_game_order`: priorities, the ban, operation locks,
    each area's personal menu, the held conditions, and the guards.
  - The earlier targeting test, now through the new input.
  - The runtime's handles and Kite's panel.
- **Shots from the runtime.**
  - The HUD over the town at the Chaos Gate, with Kite's face, name and
    bars.
  - Triangle opens PERSONAL.
  - Square opens CHAT ("You cannot use this command unless in a party.").
  - Circle closes PERSONAL.
  - Walking up the bridge, the target window names Bell ("X Talk") with
    the cursor frame on her. X holds Kite, the talk menu (22) closes at
    once, unported, and he runs on.
- **In a clean worktree.** The workspace's tests, clippy (with `trace` for
  `piney-fieldui`), fmt, the docs check, `test_world_rs` and
  `test_fieldui_rs`.

**Still unknown:**
- **Not checked against the game.** The menu buttons were read from the
  code, not run in eemu. `TalkAgainstGame` checks only the targeting
  functions.
- **One frame late.** A menu a button opens stops the camera and player
  one frame later than in the game: the world steps all its tasks before
  the menu task (priority 34).
- **The target requests are dropped.** The menus' own target changes
  (`TargetFix`, `Target`, `TargetClear`) are not passed to the world;
  they matter in battle.
- **Not ported:**
  - the talk, shop and gate menus that the action button opens;
  - the map button.
- **A drawing bug.** PERSONAL's last row draws as " og Out": the "L" glyph
  is lost in the list's text texture. It is reported to the field UI
  agent.
