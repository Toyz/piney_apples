---
number: 69
title: Orca's opening plays: event 2 hosted in Mac Anu, the event camera and the characters' commands
date: 2026-09-23
area: script, world, test
files: crates/piney-game/src/field_host.rs, crates/piney-game/src/story.rs, crates/piney-game/src/world.rs, crates/piney-game/src/main.rs, crates/piney-event/src/host.rs, crates/piney-event/src/vm/exec.rs, crates/piney-world/src/evcam.rs, crates/piney-world/src/ai.rs, crates/piney-world/src/party.rs, crates/piney-world/src/fellow.rs, crates/piney-world/src/player.rs, tools/test_evcam_rs.py, tools/test_evchar_rs.py, tools/test_field_host.py, docs/engine/event-vm.md, docs/engine/field-game.md
---

# 69. Orca's opening plays: event 2 hosted in Mac Anu, the event camera and the characters' commands

The user reported that the first login flow was missing: Orca's
walkthrough and the trip to the first field. In the game this is event 2,
"MG0020 TEACH_T". It is 57 messages, two tutorials and a scene change,
played in Mac Anu the first time Kite logs in. The event VM was ported and
checked in [[40]], but only the desktop ([[41]]) and the top page hosted
it. An agent and two helpers built the field's host and the
pieces event 2 needs.

## How it plays

`piney-game --mode world` now builds the event task as a new game reaches
Log in:
- the boot's `ccStartEvent(1, 0)`;
- the desktop's passes, with event 1 played until mails 4 and 5 are read;
- the board's;
- Log in's disable.

The session's own Log in hands over its real VM. From there, by frame:

| frames | what happens |
| --- | --- |
| 1-12 | The set-up's fade and hold. |
| 13 | The phase 0 pass: Orca's entry at marker 3, `add_target`, `gate_add` and `gate_mark` 14, the party mode. |
| 15, 16 | The phase 2 pass, then ccEnableThEvent(4) at F0. |
| 100 | `pc_act 0 3`: Kite's real arrival (TransferIn). Until then he is invisible, under manual AI. |
| to 221 | The event camera's moves, then `camera` points at Orca. |
| to 685 | Messages 0-11 in the field UI's window, each with its speaker and voice. Kite turns toward Orca gradually. |
| 685 | `member_add_msg`: sound 74 and "You now have Orca's member address!" |
| from 949 | Menus 75-77. Party > Add puts Orca in slot 1, and the HUD shows two panels. |
| 1741, 1752 | `menu_ban`, messages 21-29, then the fade and `fade_more`. |
| from 1783 | Menus 78 and 79, the gate lessons (messages 30-56). Kite and Orca warp out. |
| 2295 | `area 14` and `scene 1 0 14`: Bursting Passed Over Aqua Field. |

The runtime stops at the scene change. The window title names the area
and its words (server 0; words 0, 13, 26); the field is the next piece.

## What was built

- **The field host** (field_host.rs). piney-event's `Host` for The World
  covers:
  - messages, announcements and menus through piney-fieldui;
  - the camera and characters through piney-world;
  - sounds and voices, the fader, the party, and the gate lists in the
    save.
- **The phases.** The VM runs at the set-up's phases 0, 2 and 4, then
  once a frame before any other task.
- **The event camera** (evcam.rs):
  - `camz_set`, `camz_speed` and `camz_move`;
  - `camera`, the character camera;
  - the hand back to the player's camera.
- **The characters' event commands.**
  - `pc_act`, including Kite's TransferIn;
  - `pc_face` with a gradual turn;
  - `pc_turn`;
  - `pc_mode`;
  - the AI's manual and remote switches.

  These work for Kite and for Orca as a fellow (ai.rs, party.rs, and
  changes to fellow.rs and player.rs).
- **One party.** piney-world's `ccPartyManager` is the only party. Both
  the event's `party_add` and the field UI's `AddMember` go to it.
- **The fader.** `ccScFade`'s `EntryFade`, `ContinueFade` and the
  `SendPacket` counting.
- **ChangeScene.** The story areas are read from the executable: 126
  `eventAreaInfo` records with their word ids (story.rs).
- **A VM change.** `scene` now waits (`Wait::ChangeRequest`) after
  `change_scene`. In the game, `ChangeRequest(6, 7)` puts the event task
  to sleep inside the instruction. A host that does not model this
  answers at once, so the VM's existing checks are unchanged.

## Checked

- **The event camera.** `tools/test_evcam_rs.py`: 6 tests, 2,643
  instructions and 55,433 frames, 0 mismatches. It compares the whole
  `ccEvCamCtrl`, the camera structures and matrices, and Kite.
- **The characters' commands.** `tools/test_evchar_rs.py`: 6 tests. It
  runs 39 scenarios (311 instructions through `ccEvent::Execute`, 12,330
  frames), 0 mismatches. It compares Kite and Orca frame by frame:
  - position, heading and acts;
  - cloak and transparency;
  - list membership;
  - the AI's switches and turn.
- **The fader and ChangeScene.** `tools/test_field_host.py`: 592 fader
  calls and 122 ChangeScene states against the game, replayed by the Rust
  tests from a fixture.
- **The story areas.** All 126 match the values the game's own functions
  gave in the VM fixture.
- **The whole event.** `event_2_plays_to_the_scene_change` plays event 2
  headless from `--mode world`'s state with a scripted pad. It asserts:
  - all 69 host calls and their frames;
  - the end frame, 2295;
  - the voices;
  - the gate, word and member bits in the save;
  - the party [0, 2, -1];
  - that the VM ends asleep inside `scene`.

  `a_new_game_reaches_the_world_with_event_1_done` checks the starting
  state.
- **Re-run in a clean worktree.** The event camera, characters, field
  host, event VM, world, field UI, top page and desktop suites pass. So
  do the workspace's tests, clippy, fmt and the docs check.
- **Shots.** Orca waiting at the plaza as Kite arrives, his first line,
  the member address notice, the HUD with Kite's and Orca's panels, the
  fade, and the gate tutorial's keyword screen.

**Still unknown:**
- **What follows the scene.** The field runtime is next, so `end_event`
  is not reached here.
- **Timing not checked.**
  - The load before F0 (`ccLoadResourceFL`) takes no frames here; the
    game's takes a disc-dependent number.
  - When the field set-up enables the event task follows the desktop's
    check; it is not checked separately.
- **Not ported.**
  - `WORLD_MAN::SetEventData`.
  - `teach_input` (event 3's camera lesson).
- **The fader's drawing.** The fader is drawn with the desktop port's
  corner constants, not checked for the field's rectangle.
- **The party's own AI** is not ported. Once out of manual control, Orca
  stands. Also missing: `pc_walk_*` remote commands 1 and 2, and the
  message bus.
- **The event camera's unknowns** are listed in field-game.md.
- **Menu 79.** The ported menu 79 bumps its wait twice a frame, as
  checked against the game. So Kite is still mid warp-out (act 12) when
  the scene changes.
