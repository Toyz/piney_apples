---
number: 65
title: Mac Anu's people: the five merchants, the walking PCs, the character layer and talking
date: 2026-09-23
area: world, test
files: crates/piney-world/src/body.rs, crates/piney-world/src/char.rs, crates/piney-world/src/entry.rs, crates/piney-world/src/event.rs, crates/piney-world/src/fellow.rs, crates/piney-world/src/merchant.rs, crates/piney-world/src/mt.rs, crates/piney-world/src/navi.rs, crates/piney-world/src/npc.rs, crates/piney-world/src/rtownpc.rs, crates/piney-world/src/talk.rs, crates/piney-world/src/hit.rs, crates/piney-world/src/lib.rs, tools/test_world_rs.py, docs/engine/field-game.md
---

# 65. Mac Anu's people: the five merchants, the walking PCs, the character layer and talking

After [[61]] Mac Anu was empty. Kite walked alone past empty booths, and
the user reported the missing NPCs. The world agent and two helpers ported
the town's people from `ccEntryEventMng` down, each piece checked against
the game's own code in eemu.

## Placement

When the town's tasks start, `ccEntryEventMng` places, in this order:
1. an event's town NPCs;
2. `ccSetMerchant(0)`;
3. the gate;
4. `ccEntryRandomNpc`.

`World::place_entries` follows that order at Play(0).

## The merchants

- **Where.** The five sit in their booths. The Recorder is at (-850, 3600,
  300), facing +x.
- **Drawing.** Each is drawn within 4,600 units of Kite and inside the
  camera's 67.5-degree cone (`ccCheckCameraDeg`), through `ccChar::Draw`'s
  fade.
- **Targets.** Each joins the command list within 10,000.
- **Turning.** Mac Anu's merchants never turn.

## The walking PCs

- **How many.** 16 minus the characters already placed: Kite, the party
  and the event's NPCs. That is 15 with Kite alone and 14 with Orca.
- **Which.** `ccInitRand` and `ccRegisterRandomNpc` choose them.
- **How they walk.** They follow routes over the town's 72 landmarks, hide
  beyond 4,400 units, gather in chat groups and stop at shops.
- **Collision.** There is one character list (`Hits::chars`), in
  `HitEnable` order: merchants, PCs, then Kite once he has arrived. Kite
  bumps into them, and the PCs push each other.

## The character layer

- **The types.**
  - `body::Body` is a clump with models hung on its nodes.
  - `char::Char` is `ccChar`: position, direction, animation, fade and
    draw.
  - `entry::Npc` is the trait every placed character implements.
  - `fellow::Fellow` is a party member an event places.
- **What the event scripts call.** This is what the field host will need:
  - `World::entry` places party members at markers and queues town PCs.
  - `World::remove` takes a party member out.
  - `pc_command` and `npc_command` take the put, turn and face opcodes.
  - `marker`, `char_pos`, `set_event_targets` and `npc` look things up.

## Talking

- **The targeting half of `ccThGameCtrl`**, as the game does it:
  - `ccEntryCmnd`;
  - `ccSortCmnd`: `cmndSortRoot`'s order, distance and direction;
  - `ccCheckTargetRange`;
  - `ccSelectTarget`'s three modes;
  - `ccChangeCmndTarget`.
- **The action button** (`assignPADaction`, default cross) on the target:
  - reaches 60 plus both widths, or 300 plus both widths within 1.2
    radians ahead;
  - asks for the menu the target's type bits choose: a walking PC's talk
    (22), a shop (24-26, with the Recorder's greeting table at
    0x00631a50), the Chaos Gate (28), or a party member (21);
  - holds Kite (`pauseSW`), and the PC stops and faces him, until
    `close_menu`.
- **How the runtime sees it.** `World::take_talk` reports these requests.
- **For the field UI.** The world now offers:
  - `command_sorted`, `targeting`, `tag_pos` (`ccCalcTagPosChar`) and
    `char_info`;
  - `step_into` for drawing into a shared frame;
  - `state_mut`;
  - `set_asleep`, for the other tasks' sleep while a menu is open.

## Checked

- **Against the game.** `tools/test_world_rs.py` now has 18 tests, all
  with 0 mismatches:
  - **targeting:** 600 random scenes through the game's
    `ccEntryCmnd`, `ccSortCmnd`, `ccCheckTargetRange` and
    `ccSelectTarget`;
  - **facing:** 1,505 `ccGetDirc` cases;
  - **tag points:** 800 `ccCalcTagPosChar` cases;
  - **merchants:** the game's `ccSetMerchant(0)`, then 1,500 frames of
    all five, compared in placement, lists, view cone, animation, fades,
    turns, hits and root matrix;
  - **walking PCs:** 168 selection seeds, 400 route searches, 700
    `HitCheck` calls, and 3,720 frames of 14-16 PCs beside the game's own
    constructor.
- **Disc tests.**
  - Orca's entry at marker 3, with `pc_put`, `pc_face` and remove.
  - Talking to the Recorder asks for its shop.
  - Placement order.
- **Shots.** The Recorder in his booth, armed PCs crossing the plaza, and
  Orca at marker 3.
- **Re-run in a clean worktree.** I re-ran the world and field UI suites
  there. The workspace's tests pass, and clippy, including `piney-fieldui`
  with `trace`, fmt and the docs check are clean.

**Still unknown:**
- **Not ported:**
  - the party's own AI (`ccThFellow`, `ccAI`): a party member's walking,
    gradual turns and `pc_act`;
  - the administrators;
  - event mode -3.
- **Not yet in the runtime.** The other `ccThGameCtrl` buttons (map, chat,
  option, personal, the menu ban) are not ported, and the menus the talk
  requests ask for are not wired.
- **Drawing and sound.** The walking PCs' texture variants draw with the
  default texture. Their footsteps and dust are not read.
- **Unknown:**
  - whether a party member or the gate has a body on the character list;
  - `ccSetRtownPC`'s second argument on the event path;
  - `ccSys` +0x358, the frame count that picks the PCs, is an input.
- **Order.** An event NPC placed after set-up goes to the list's end.
