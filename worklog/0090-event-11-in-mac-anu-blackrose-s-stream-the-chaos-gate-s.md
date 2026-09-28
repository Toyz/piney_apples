---
number: 90
title: Event 11 in Mac Anu: BlackRose's stream, the Chaos Gate's talk, and her call into the party
date: 2026-09-25
area: script, world, video, test
files: crates/piney-game/src/field_host.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/event11.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/event.rs, docs/engine/field-game.md
---

# 90. Event 11 in Mac Anu: BlackRose's stream, the Chaos Gate's talk, and her call into the party

The user reported that adding BlackRose to the party does not work. In
`story:11` Kite ended alone at the Chaos Gate ([[86]]). Three things were
missing on the way, none of them in the interpreter.

## What was missing

1. **Stream 7 never played.** The town host had no `stream`. The stream
   plays in the town's set-up pass at phase 2, before the town's tasks
   start.
2. **Speaking to the Chaos Gate opened its menu.** Block 6 of event 11
   (`talked_to type=13 code=16`) never ran, so BlackRose never came over
   with her member address.
   - **The game's rule.** `ccEvent::CheckOperate(9)` gives the talk to the
     event when a target's type is a bit of the character's base type
     flags and the codes match.
   - **The port's rule.** It had mapped event types 0-6 to world kinds (PC,
     NPC, enemy), so the gate, a type-13 gimmick, was never a target.
3. **The Member Address list was empty.** The town never gave the menus
   the registry (`ccSpcManager.registry[registryNum]`) that `PartyInMenu`
   reads for its greeting and its room check.

## The fix

- **The town host.**
  - `stream`: `ccEventStream(num, 1)` through the stream player, with
    subtitles and music, `ccGame` status 5. It steps a frame at a time
    while the call waits (`Wait::Stream`), and its frames show instead of
    the town's, whose tasks do not run meanwhile. The session gives the
    renderer the stream's archive.
  - `target_alive` (`ccCheckTarget`).
  - `game_over`: `compulsionGameOver`, which only the Data Drain menu sets,
    so always false in a town.
  - `clear_gate_hack`: `ccClearGtHack`, logged; nothing in the port sets
    `gtHackFlag` yet.
- **The talk test.** The town world keeps the event's targets as (type,
  code) and tests them as the interpreter does, against the target's base
  type flags.
- **The Member Address list.** `ui_world` fills fieldui's `spc` from the
  registry.

## Checked

- **A playthrough test.** `event_11_blackrose_joins_in_mac_anu` plays
  `story:11` with a scripted pad, as a player would:
  1. windows closed;
  2. Kite walks to the Chaos Gate and speaks to it;
  3. BlackRose's block runs and gives her member address;
  4. then triangle, Party, Add, BlackRose, OK.

  She joins the party, and no event instruction falls to a host default
  (`take_unported()` is empty).
- **An existing test updated.** `event_2_plays_to_the_scene_change` now
  expects `clear_gate_hack` at frame 12, which the game calls from
  `ccStartThEvent`.
- **The rest.** The workspace's tests, clippy, fmt, the docs check,
  test_world_rs and test_field_rt pass.
- **A shot.** Stream 7 in Mac Anu: BlackRose facing Kite.

**Still unknown:**
- **Party members stand still in town.** `ccAI::ActInTown` is ported but
  runs only in the fields' combat, so BlackRose does not follow Kite
  around Mac Anu.
- **The warp is now played through.** `event_11_gates_to_area_15_with_blackrose`
  goes back to the gate, picks area 15 in the Word List, and chooses
  Warp. Kite arrives on the holy ground with BlackRose in the party and
  placed, and her first line there opens. `event_11_shots` (ignored)
  saves the pictures. Not yet checked: event 11's block 17 as the gate
  sees the words, and the rest of event 11 in area 15.
- **Not ported.** The invite of an unregistered member at the Chaos Gate
  (`inviteSpc`'s warp-in with `StartPos` and `inviteOffsetTbl`).
- **The talk rule is not checked in eemu.** `ccEvent::CheckOperate(9)`'s
  target test is the interpreter's (checked), and the town now reuses it.
- **The gate-address window in town is fixed.** It was empty ([[82]]).
  The town host now takes a gate address's and a desktop item's lines
  from the desktop's composer (`story::Announcements`, checked in [[82]])
  and opens `DispInfo` with them (`FieldUi::announce_lines`). Event 11
  shows "Δ Hidden Forbidden Holy Ground / is added to the Word List."
  (`event_11_shots`' 2b).
