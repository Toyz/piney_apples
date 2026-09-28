---
number: 110
title: The party follows Kite in town: the members as the battle's characters under ActInTown
date: 2026-09-25
area: world, battle, test
files: crates/piney-world/src/town_party.rs, crates/piney-world/src/combat/town.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/party.rs, crates/piney-world/src/event.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-world/examples/evchar_probe.rs, crates/piney-world/tests/world.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/event22.rs, tools/test_evchar_rs.py, docs/engine/field-game.md, docs/engine/field-walk.md
---

# 110. The party follows Kite in town: the members as the battle's characters under ActInTown

Before this, a party member in a Root Town was piney-world's own
`Fellow`. It ran only the manual paths the event scripts drive, so out of
manual control it just stood. The fields already ran their members
through piney-battle (`ccFellow::Main`, `ccAI::Brains` and the movement
runtime), which is checked in eemu. `ccAI::ActInTown` was ported and
checked there too, but nothing in the town called it. Now the town's
members run through that same machinery.

## The town's party

- **`TownParty`** (town_party.rs) holds a `Combat` of area 0.
  - Each member is built by `Combat::add_member`
    (`ccFellow::Initialize` and Reboot's AI, with `arrivalChatCnt` 150).
  - Each frame, `TownParty::frame` runs after Kite's frame. It calls
    `Combat::town_frame` (combat/town.rs): `ccThSpc`, `ccThAISystem`, then
    each member's `ccFellow::Main` with the `Movement` runtime, as the
    field does.
- **Kite stays the town's `Player`**, which the game checks frame by frame.
  - The scene holds a stand-in for him, made by `Combat::add_leader`.
  - `Combat::mirror_leader` updates it before the members run. It copies
    his place, heading, act, the flags of his step, `dead`, `partyFlag`,
    `dispSW`, whether he is on the command list, and his AI's manual
    switch.
  - The stand-in has no body in the collision and no animation.
- **The town's navigation** is handed to the members' `Stage`
  (`TownNav`):
  - `ccSetNaviMap`'s landmarks (piney-world's `NaviMap`, which also routes
    through `RouteSearchByMap`), converted to the battle's `TownMap`.
  - `naviPointNameTable` (gcmn 0x00653c80): eight names, 1 `DMY_gate`
    and 2-7 `DMY_merchant1`-`6`, placed at the town file's dummies.
- **The events** reach the members through `TownChars`, which lends
  them the way the field does.
  - Id 0 is Kite's `Player`. The others are the battle's records
    (`SpcRec`), written back afterwards.
  - `World::with_party` and the probe both use
    `TownParty::with_chars`.
  - The Stage's remote command 5 reaches the registry (`resignParty` /
    `disbandSpc`).
- **piney-game's request arms** reach the members' AI: chat orders,
  manual off, `ManualModeAI` and remote commands.
- **Removed:** the old `Fellow` (fellow.rs) and `party::Chars`.

## What the check against the game found

`tools/test_evchar_rs.py` runs Orca through the game's own `ccFellow::Main`
in eemu. It used to compare against the old `Fellow`. I rewrote
`evchar_probe` to build, lend and step the members through `TownParty`,
the same path the World takes, and the comparison found three
differences:

- **`ccAI::ChatMessageSender`** (gcmn 0x00586420) was not handled.
  - piney-battle raises it as a runtime call, but the `Stage` let it fall
    through, so `arrivalChatCnt` never counted down in towns or fields.
  - The game's behaviour:
    - It does nothing once the party is wiped out.
    - A line that was asked for is opened, and `chatRequest` is
      cleared.
    - The count runs down past 0. At 0, out of a fight, it says the
      arrival's line: `ChatMessageEnteredTown` (0x0058f810, from
      `arriveDeltaMessages` and the other servers' tables, silent under
      manual control), or `ChatMessageEnteredField` in a field reached
      from a town. Neither draws `rand()`.
  - The Stage now does the count and the flag. The lines are not shown.
- **A leaver's end came a frame early.**
  - In the game, `ccThFellow02` (0x0041ed60) loops: `Breath`, then
    `Main` unless `exitFlag` is set. So the frame after a Main sets
    `exitFlag`, the task wakes to it and ends.
  - At the end it calls `expulsionSpc(listNum)`, then `ccDeleteThread`.
    The delete runs `ccThFellow02Delete` (0x0041ee90): `~ccFellow`
    (0x0041ad60: the registry's and the party's pointers cleared, the AI
    deleted, the body off the list, `ccDeleteCmnd`), then
    `DelSpc(listNum)` unless `partyFlag` is 1.
  - The port did all of this in the same frame as the Main. Both the town
    (`TownParty::frame`) and the field (`FieldWorld::fellows_gone`, now
    before the combat frame) now do it the frame after.
  - The eemu side now plays the task's end too: `expulsionSpc`, the real
    destructor, `DelSpc`.
- **The constructors' `rand()` draws** (found in the disassembly while
  scripting the probe).
  - The order is:
    - `ccSpcChar::ccSpcChar` (0x0059d230): `cycle = rand() >> 3`.
    - `ccFellow::Initialize`: `atkDellay = (rand() >> 3) & 31` (always),
      then `transferLag` (not hacked).
    - Reboot's `ccAI`: two draws.
  - The port drew only `transferLag` and the AI's two draws.
  - `add_member` and `add_kite` now draw these in the fields as well.
  - In the town, Kite's cycle and his AI's count are taken from his
    stand-in's draws.

## Checked

- **`test_evchar_rs`:** 6 tests, 39 scenarios, 311 instructions, 12,330
  frames, 0 mismatches. This covers Kite and Orca under event 2,
  `pc_act`/`pc_turn`/`pc_face`/`pc_mode`, `menu_ban`/`menu_clear`,
  `party_add`/`party_remove`, and the leaver's exit.
- **`test_world_rs`**, **`test_gate_out_rs`** and **`test_party_rs`** pass.
- **piney-game**, all 56 tests:
  - `piros_walks_mac_anu_on_his_own`: back in Mac Anu with no order,
    Piros goes 96, 99, then a shop or landmark (5-10), then rests (11).
    With the new rand draws he picks a shop now.
  - `piros_follows_kite_in_mac_anu`: after chat 9 (acts 3 then 94), he
    stays within 400 of Kite as Kite runs down the plaza.
  - The event 11 and 22 tests and `the_town_takes_the_party_back` pass.
- **piney-world's tests**, with the event-command tests now reading
  `TownParty::rec`.
- **Shots:** follow-350 and follow-550 show Piros drawn behind Kite in Mac
  Anu.

**Still unknown:**
- The members' chat lines and the chat window are not shown. The text of
  `ChatMessageEnteredTown` and `EnteredField` is not ported.
- `ccThSpc` and `ccThAISystem` run before a leaver's task ends in the
  port. In the game the task ends at its own slot (50), after them.
- The `menu_type` handed to a town frame is -1 or 0 (the town knows
  whether a menu is open, not which one).
- A second `rebootSpcManager` in the same town visit makes no new AI for
  Kite's stand-in, and draws nothing for it.
