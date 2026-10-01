---
number: 315
title: Three player reports: Elk left in the party, Natsume's talk, the Gott statue's hold
date: 2026-10-01
area: script, world
files: crates/piney-world/src/field_world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/party_leave.rs, crates/piney-game/src/session/tests/side_events.rs, crates/piney-game/src/session/tests/town_return.rs, BUGS.md
---

# 315. Three player reports: Elk left in the party, Natsume's talk, the Gott statue's hold

GitHub issues 16, 17 and 18 (Infection). Each case was played in the
session as the report describes it before any change, and each test fails
without its fix. Addresses are INF unless marked; messages by number only.

## Issue 16: Elk kept a nameless party slot after event 18

Event 18 (`MG0510`) block 20, after the Data Bug in area 19's dungeon
(point 1's room), ends with `pc_act 1 5`, `pc_act 10 5` and `scene 0 0`.
It has no `party_remove`. Act 5 is remote command 5: `ccAI::ManualControl`
(gcmn 0x00580ef0) transfers the member out and calls `resignParty(id)`
(gcmn 0x0059d150), so `DelMember` empties his slot at once. The fellow
task's exit later frees his registry slot (`expulsionSpc`, `DelSpc`).

The port's battle stage only carried out `ResignParty`/`DisbandSpc` where
it held the registry, which was the town. In fields and dungeons the call
was dropped, so a member left only through his exit. Played from block 18
with Elk and Mia in the party: Mia's exit came before `scene 0 0`, Elk's
did not. The change of scene's fellow deletes freed his registry slot and
kept his party slot. Mac Anu then showed party `[0, 10, -1]` with the
registry Kite alone: a face with no name (the town's panel draws every
`memberID`; the name comes from the built character), and nobody to see.
A field built only registered members, so Elk was gone there; Mac Anu
showed him again. This is the report's three screenshots.

Fix: `combat::Tasks` carries the field world's `Spcs`, and its frame
resigns and disbands on it as the town's does; the battle's party is
re-synced when that changes. `event_18_leaves_kite_alone` plays blocks
18-20 (the pilot drains the Data Bug) and checks the party and registry
in Mac Anu, a field and Mac Anu again. Before: `[0, 10, -1]`. After:
Kite alone each time.

## Issue 17: Natsume's PC menu instead of her line

Event 61 block 4 (`has_item 0 0 60 <= 0`, `talked_to 2 11`) answers a talk
to Natsume with message 3. `ccEvent::CheckOperate(9)` (main 0x001b32f0)
refuses the talk when `cmndTarget->base->type` has a target's type bit
and `base->id` (+0xc) is its code. The field's copy of that test read the
character at scene index `code`, but a party character's command code is
its `charTbl` row (11). Natsume is scene character 1, so the id was wrong
and the field opened menu 21 (Talk, Trade, Gift), as in the screenshot.
Now it takes the scene index from `target_index`.
`event_61_natsume_wants_the_spiral_edge`: before, menu 21 and no message
3; after, message 3.

## Issue 18: block 11 ran on entering the statue's room

Block 11 (in_point 2, the statue's room at floor 2 block 5) waits on
`no_active`. CheckOpen's case (0x001a8614-0x001a86d4) needs
`ccCheckActiveObject()` and also walks `g_entCtrl.gimHead` (+0x2c,
`gimNum` +0x28). Any gimmick with `objFlag` set (+0xe0 bit 0),
`base->type` bit 20 (0x100000, the idols' `ItemIdolMenu` type) and
`cmndFlag` clear (bit 6) fails it. The unopened statue is one until
`ccGimIdol::main`'s opening `deleteCmnd`. The port's `no_active_object`
left the gimmicks out. Mutation, Outbreak and Quarantine run the same
walk (`lui 0x10`, `lbu 0xe0`, next at +0x1c4).

`event_61_the_statue_holds_block_11`: before, message 12 and `room 0 0`
within 300 frames of entering; after, block 11 waits, the statue opened
lets it run, and at the entrance block 5 asks for the Spiral Edge. Kept
(message 5's second answer), block 6 runs, and a talk then runs block 8.

## Harness note

`natsume_dungeon` now goes to the point's room with `room_select`, as
`room_point` does. The old test's `ChangeScene` to the same room, asked in
the first frames of play, left Natsume off the command list. The same
request a little later kept her listed. No player can ask for a scene
change in those frames.

**Still unknown:** nothing about these three rules. The reporter's own
order of play is not known (in the screenshots Elk is in slot 1); it
does not change the cause.
