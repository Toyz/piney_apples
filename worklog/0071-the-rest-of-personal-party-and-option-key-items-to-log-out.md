---
number: 71
title: The rest of PERSONAL, PARTY and OPTION: Key Items to Log Out, checked against the game, and Log Out back to the top page
date: 2026-09-23
area: ui, test
files: crates/piney-fieldui/src/menus/keyitem.rs, crates/piney-fieldui/src/menus/leave.rs, crates/piney-fieldui/src/menus/personal.rs, crates/piney-fieldui/src/menus/status.rs, crates/piney-fieldui/src/menus/equip.rs, crates/piney-fieldui/src/menus/party_menus.rs, crates/piney-fieldui/src/menus/option.rs, crates/piney-fieldui/src/menus/option/disp.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-world/src/lib.rs, tools/test_fieldui_personal_rs.py, tools/test_fieldui_option_rs.py, docs/engine/field-ui.md
---

# 71. The rest of PERSONAL, PARTY and OPTION: Key Items to Log Out, checked against the game, and Log Out back to the top page

The user reported that most menus did nothing. [[64]] ported PERSONAL's
Skills and Items and OPTION's list; an agent (with a helper for OPTION's
pages) has now ported the rest.

## The menus

Each is checked frame by frame against the game's `ccThMenu` in eemu.

- **6 Key Items.** Its pages; each item's use or refusal; the Grunty
  Flute; a Ryu Book's fade.
- **7 Discard Item.** The counts and the dialog.
- **Status.**
  - 8 Status, 31 a member's items, 64 a piece's status;
  - the whole grid;
  - L1/R1 through the members;
  - the shared boxes (ParameterDisp, SkillDisp, BeffDisp).
- **63 Equipment.**
  - A change writes DelItem, AddItem and ChangeEquipment: the skills, and
    the set bonuses 291-294 and 289. It then writes the save's `real` and
    `tune`, from piney-battle's `calc_real`.
  - The preview reads the equipment tables as raw bytes, as the game
    does. An empty slot reads the row before its table.
- **Leaving.** 10 Gate Out, 11 Log Out, 86 TransFieldMenu, 87 Area
  Information.
- **9 PARTY.** 68 Add, 69 Remove and 70 Disband, with the members'
  greetings and farewells in their voices (group -30).
- **OPTION's pages 13-20.** Controller, Vibrate, Adjust Screen, Sound,
  Title Screen, Data Drain, Voiceover and Movie Text.

## In the runtime

- **Log Out.** Log Out and OPTION's Title Screen ask for
  `ccGame::ChangeRequest` (4, 7) or (1, 7). The session now leaves The
  World for its top page on 4, handing the save and the event task back,
  and resets to the title on 1.
- **Sound.** The Sound page's volumes go to the sound driver.
- **The party.** Remove and Disband take the member out of the world's
  party (`ccParty::DelMember`).
- **Controller.** The page sets the field camera's scheme
  (`setCameraCtrlType`, through the new `World::set_camera_type`).
- **Not carried out yet:**
  - the Grunty Flute's use;
  - the way back to a field (86);
  - a member's `CalcReal`;
  - new equipment on the member's model;
  - Adjust Screen's offset;
  - vibration.

  The save already holds what each of them changed.

## Checked

- **Against the game.**
  - `tools/test_fieldui_personal_rs.py`: 17 tests, including 16 random
    pad runs through PERSONAL in towns and fields.
  - `tools/test_fieldui_option_rs.py`: 12 tests.
  - The field UI's and the shops' suites still pass.
- **Re-run in a clean worktree.** The same suites and the world's pass,
  with the workspace's tests, clippy (including piney-fieldui with
  `trace`), fmt and the docs check.
- **Shots from the runtime** (`--mode world --no-events`): Status,
  Equipment, PARTY, Key Items, Log Out's question, and the Controller
  page.

**Still unknown:**
- **The runtime's new-game save is wrong in two ways.** Status and
  Equipment show both. They are being fixed with `ccSaveData::Init`'s
  fills:
  - the skill list is zeros, not -1, so 16 "NOTHING" rows show;
  - the stats read 0.
- **Timing.** The game's equipment, party-add and Controller tasks load
  files and may take several frames. The port finishes each one frame
  later, the shortest the game can take.
- **Past the equipment tables.** The port reads zeros where the game
  reads whatever memory follows.
- **`bootParam`.** What it means beyond choosing Add's greeting.
- **Two copies of `rand()`.** The menus keep their own, and only the talk
  menus' copy can be fed from outside.
- **From the OPTION helper:**
  - the CD load's real frame count;
  - what `SetActuater(1, 160, 200)` means exactly;
  - where drainDemo, voice and strWinMode are read.
- **Still unported:** 62 (the gate hack), 66/67 (Data Drain), and 71-73
  (CHAT per member).
